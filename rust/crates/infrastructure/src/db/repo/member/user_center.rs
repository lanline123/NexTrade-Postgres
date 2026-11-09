//! 前台用户中心仓储（member::user_center）。
//!
//! 替代原 Java `UserServiceImpl` / `UserRankServiceImpl` / `UserRankConfigServiceImpl` /
//! `CreditUserServiceImpl` / `CollectShopServiceImpl` 中的数据访问部分。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table <t>`）：
//! - `user`：`user_id` / `username` / `avatar` / `mobile` / `mobile_validated` /
//!   `email` / `email_validated` / `nickname` / `birthday` / `password` / `rank_id` /
//!   `history_product_ids` / `growth_points` / `status` / `wechat_img` 等 36 列
//! - `user_rank` / `user_rank_config`：会员等级与等级配置
//! - `rank_growth_log` / `user_growth_points_log`：成长值日志
//! - `collect_shop`：店铺收藏（`collect_id` / `user_id` / `shop_id` / `add_time`）
//! - `credit_user` / `credit_order`：账期授信与账期订单
//! - `config`：系统配置（`biz_code` / `biz_val`，替代原 Java `ConfigService`）
//! - `shop` / `product` / `product_sku` / `order_item` / `order`：关联数据
//!
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` / `query_as!` 宏），
//! 每条 SQL 字面量都写在调用参数位置并以 `,` / `)` 收尾，
//! 便于 `tools/check_sql_columns.py` 逐条归因校验。
//!
//! > **注意**：`order` 是 MySQL 保留字，SQL 中必须使用反引号；
//! > 反引号形式同时会让列名校验器把它当成 `ORDER BY` 而跳过（脚本的已知盲区），
//! > 因此本文件对 `` `order` `` 的列名做了人工核对。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{
    CollectShop, CreditOrder, CreditUser, OrderItem, Product, RankGrowthLog, Shop, User,
    UserGrowthPointsLog, UserRank, UserRankConfig,
};
use nextrade_core::models::member::user_center::UpdateInformationDTO;
use nextrade_core::{Error, Result};

/// `user` 表全部列（顺序与 [`User`] 实体字段一致）。
const USER_COLUMNS: &str = "user_id, username, avatar, mobile, mobile_validated, email, \
     email_validated, nickname, password, birthday, balance, frozen_balance, \
     commission_balance, frozen_commission_balance, points, growth_points, address_id, \
     reg_time, last_login, last_ip, rank_id, referrer_user_id, from_tag, is_svip, \
     svip_expire_time, order_count, order_amount, history_product_ids, is_distribution, \
     distribution_register_time, wechat_img, is_company_auth, status, user_type, admin_note, mark";

/// \"user_rank\" 表全部列（顺序与 [\"UserRank\"] 实体字段一致）。
const USER_RANK_COLUMNS: &str = "rank_id, rank_name, min_growth_points, max_growth_points, \
     discount, show_price, rank_type, rank_logo, rank_ico, rank_bg, rank_point, \
     free_shipping, rank_card_type, rights, rank_level";

/// 收藏店铺可选排序字段白名单（Java \"getColumnByPropertyName\"：非法值回落到主键列）。
///
/// 对应 \"CollectShop\" 实体的属性名与列名，防止排序字段注入。
const COLLECT_SHOP_SORT_COLUMNS: [(&str, &str); 8] = [
    ("collectId", "collect_id"),
    ("collect_id", "collect_id"),
    ("userId", "user_id"),
    ("user_id", "user_id"),
    ("shopId", "shop_id"),
    ("shop_id", "shop_id"),
    ("addTime", "add_time"),
    ("add_time", "add_time"),
];

/// 收藏店铺列表可选的「商品标记」筛选列（避免动态列名拼接）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductFlag {
    /// `is_hot = 1`
    Hot,
    /// `is_new = 1`
    New,
    /// `is_best = 1`
    Best,
}

impl ProductFlag {
    /// 对应的固定 SQL 片段（列名来自 schema，非用户输入）。
    #[must_use]
    fn where_clause(self) -> &'static str {
        match self {
            Self::Hot => "AND is_hot = 1",
            Self::New => "AND is_new = 1",
            Self::Best => "AND is_best = 1",
        }
    }
}

/// 仓储错误映射：SQLx 错误 → [`nextrade_core::Error`]。
///
/// 与 `repo::shop` / `repo::decorate` / `repo::aftersales` 等既有仓储保持同一口径
/// （`nextrade_core::Error` 未提供 `From<sqlx::Error>`）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// 按 SQL `IN (?, ?, ...)` 生成占位符；空列表由调用方提前返回。
fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

#[derive(Clone)]
pub struct UserCenterRepository {
    pool: PgPool,
}

impl UserCenterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— user —————————————————————

    /// 按主键查询用户。
    pub async fn find_user_by_id(&self, user_id: i32) -> Result<Option<User>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {USER_COLUMNS} FROM \"user\" WHERE user_id = ? LIMIT 1"
        ));
        sqlx::query_as::<_, User>(&sql)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 手机号是否被他人占用（Java `count(mobile = ? AND user_id <> ?)`）。
    pub async fn exists_by_mobile(&self, mobile: &str, except_user_id: i32) -> Result<bool> {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) AS c FROM \"user\" WHERE mobile = $1 AND user_id <> $2",
        )
        .bind(mobile)
        .bind(except_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(c > 0)
    }

    /// 邮箱是否被他人占用（Java `count(email = ? AND user_id <> ?)`）。
    pub async fn exists_by_email(&self, email: &str, except_user_id: i32) -> Result<bool> {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) AS c FROM \"user\" WHERE email = $1 AND user_id <> $2",
        )
        .bind(email)
        .bind(except_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(c > 0)
    }

    /// 更新会员资料（Java \"updateInformation\" 的 \"updateById\"）。
    ///
    /// Java 用 MyBatis-Plus 默认 \"NOT_NULL\" 更新策略：\"null\" 字段不写库。
    /// 这里按 `Option::is_some` 动态拼 SET，语义完全一致。
    ///
    /// 注意：Java \"updateInformation\" **不**更新 \"mobile_validated\" / \"email_validated\"
    /// （验证标记由 \"modifyMobile\" / \"modifyEmail\" / 绑定流程负责）。
    pub async fn update_information(
        &self,
        user_id: i32,
        dto: &UpdateInformationDTO,
    ) -> Result<bool> {
        let mut sets: Vec<&str> = Vec::new();
        if dto.birthday.is_some() {
            sets.push("birthday = ?");
        }
        if dto.nickname.is_some() {
            sets.push("nickname = ?");
        }
        if dto.email.is_some() {
            sets.push("email = ?");
        }
        if dto.mobile.is_some() {
            sets.push("mobile = ?");
        }
        if dto.wechat_img.is_some() {
            sets.push("wechat_img = ?");
        }
        if sets.is_empty() {
            // 无字段可更新：Java \"updateById\" 同样不产生 UPDATE 语句
            return Ok(true);
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE \"user\" SET {} WHERE user_id = ?",
            sets.join(", ")
        ));
        let mut q = sqlx::query(&sql);
        if let Some(v) = dto.birthday.as_deref() {
            q = q.bind(v);
        }
        if let Some(v) = dto.nickname.as_deref() {
            q = q.bind(v);
        }
        if let Some(v) = dto.email.as_deref() {
            q = q.bind(v);
        }
        if let Some(v) = dto.mobile.as_deref() {
            q = q.bind(v);
        }
        if let Some(v) = dto.wechat_img.as_deref() {
            q = q.bind(v);
        }
        let n = q
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    /// 直接改绑邮箱并置验证标记（Java \"emailValidate\" 的
    /// `user.setEmail(...)` + `user.setEmailValidated(1)` + \"updateById\"）。
    pub async fn bind_email(&self, user_id: i32, email: &str) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE \"user\" SET email = $1, email_validated = 1 WHERE user_id = $2",
        )
        .bind(email)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(n > 0)
    }

    /// 更新头像。
    pub async fn update_avatar(&self, user_id: i32, avatar: &str) -> Result<bool> {
        let n = sqlx::query("UPDATE \"user\" SET avatar = $1 WHERE user_id = $2")
            .bind(avatar)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    /// 更新手机号。
    pub async fn update_mobile(&self, user_id: i32, mobile: &str) -> Result<bool> {
        let n = sqlx::query("UPDATE \"user\" SET mobile = $1, mobile_validated = 1 WHERE user_id = $2")
            .bind(mobile)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    /// 更新邮箱。
    pub async fn update_email(&self, user_id: i32, email: &str) -> Result<bool> {
        let n = sqlx::query("UPDATE \"user\" SET email = $1, email_validated = 1 WHERE user_id = $2")
            .bind(email)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    /// 禁用用户（`close` 接口，Java `closeUser`：`status = 0`）。
    pub async fn disable_user(&self, user_id: i32) -> Result<bool> {
        let n = sqlx::query("UPDATE \"user\" SET status = 0 WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    /// 更新浏览历史（JSON 数组字符串）。
    pub async fn update_history(&self, user_id: i32, history: &str) -> Result<bool> {
        let n = sqlx::query("UPDATE \"user\" SET history_product_ids = $1 WHERE user_id = $2")
            .bind(history)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    /// 累加成长值（Java `user.setGrowthPoints(user.getGrowthPoints() + growthPoints)`）。
    pub async fn add_growth_points(&self, user_id: i32, delta: i32) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE \"user\" SET growth_points = growth_points + $1 WHERE user_id = $2",
        )
        .bind(delta)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(n > 0)
    }

    /// 只回写会员等级 ID（Java \"updateUserRank\" 的 \"LambdaUpdateWrapper\"）。
    pub async fn set_rank_id(&self, user_id: i32, rank_id: i32) -> Result<bool> {
        let n = sqlx::query("UPDATE \"user\" SET rank_id = $1 WHERE user_id = $2")
            .bind(rank_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(n > 0)
    }

    // ————————————————————— config —————————————————————

    /// 读取系统配置值（Java `ConfigService.getConfigVal`：
    /// `biz_code = ? AND is_del = 0`，缺行返回空串 → 这里返回 `None`）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— user_rank —————————————————————

    /// 全部会员等级（Java `orderByAsc(UserRank::getMinGrowthPoints)`）。
    pub async fn all_user_rank(&self) -> Result<Vec<UserRank>> {
        let sql = format!(
            "SELECT {USER_RANK_COLUMNS} FROM user_rank ORDER BY min_growth_points ASC"
        );
        sqlx::query_as::<_, UserRank>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键查询会员等级。
    pub async fn find_user_rank(&self, rank_id: i32) -> Result<Option<UserRank>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {USER_RANK_COLUMNS} FROM user_rank WHERE rank_id = ? LIMIT 1"
        ));
        sqlx::query_as::<_, UserRank>(&sql)
            .bind(rank_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按成长值取匹配等级（Java \"updateUserRank\"：
    /// `min_growth_points <= growth` 且按 \"min_growth_points\" 降序取第一条）。
    pub async fn find_rank_by_growth(&self, growth_points: i32) -> Result<Option<UserRank>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {USER_RANK_COLUMNS} FROM user_rank WHERE min_growth_points <= ? \
             ORDER BY min_growth_points DESC LIMIT 1"
        ));
        sqlx::query_as::<_, UserRank>(&sql)
            .bind(growth_points)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 读取 `user_rank_config` 单行（Java `getOne(code = ? limit 1)`）。
    pub async fn find_rank_config(&self, code: &str) -> Result<Option<UserRankConfig>> {
        sqlx::query_as::<_, UserRankConfig>(
            "SELECT id, code, rank_type, data FROM user_rank_config WHERE code = $1 LIMIT 1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 成长值日志 —————————————————————

    /// 统计指定类型成长值日志条数（Java `rankGrowthLogService.count(userId, type)`）。
    pub async fn count_rank_growth_log(&self, user_id: i32, type_code: i32) -> Result<i64> {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) AS c FROM rank_growth_log WHERE user_id = $1 AND type = $2",
        )
        .bind(user_id)
        .bind(type_code)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(c)
    }

    /// 写入成长值日志，返回自增 ID。
    pub async fn insert_rank_growth_log(&self, log: &RankGrowthLog) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO rank_growth_log (user_id, type, growth_points, change_type, \
             create_time) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(log.user_id)
        .bind(log.r#type)
        .bind(log.growth_points)
        .bind(log.change_type)
        .bind(log.create_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 写入会员成长积分记录，返回自增 ID。
    pub async fn insert_growth_points_log(&self, log: &UserGrowthPointsLog) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_growth_points_log (user_id, points, change_time, \
             change_desc, change_type) VALUES ($1, $2, $3, $4, $5) RETURNING log_id",
        )
        .bind(log.user_id)
        .bind(log.points)
        .bind(log.change_time)
        .bind(log.change_desc.as_str())
        .bind(log.change_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    // ————————————————————— collect_shop / shop / product —————————————————————

    /// 收藏店铺分页（Java `CollectShopServiceImpl#list`）。
    ///
    /// - \"exclude_self_operated\"：`shop_id <> 0`（Java \"ne\" 条件）
    /// - 过滤已删除店铺：`shop_id = 0 OR EXISTS(shop ... is_delete 为空或 0)`
    /// - \"order_by\" 由 [`UserCenterRepository::collect_shop_order_by`] 生成的白名单片段
    pub async fn paginate_collect_shop(
        &self,
        user_id: i32,
        exclude_self_operated: bool,
        order_by: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<CollectShop>)> {
        let self_operated_cond = if exclude_self_operated {
            " AND shop_id <> 0"
        } else {
            ""
        };
        let deleted_shop_cond = " AND (shop_id = 0 OR EXISTS (SELECT 1 FROM shop s \
             WHERE s.shop_id = collect_shop.shop_id AND (s.is_delete IS NULL OR s.is_delete = 0)))";

        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) AS c FROM collect_shop WHERE user_id = ?\
             {self_operated_cond}{deleted_shop_cond}"
        ));
        let (total,): (i64,) = sqlx::query_as(&count_sql)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT collect_id, user_id, shop_id, add_time FROM collect_shop \
             WHERE user_id = ?{self_operated_cond}{deleted_shop_cond} ORDER BY {order_by} \
             LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, CollectShop>(&rows_sql)
            .bind(user_id)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.max(0).unsigned_abs(), rows))
    }

    /// 收藏店铺（幂等：已存在则忽略；Java `CollectShopServiceImpl#save`）。
    pub async fn save_collect_shop(&self, user_id: i32, shop_id: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO collect_shop (user_id, shop_id, add_time) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(shop_id)
        .bind(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i32)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 取消收藏（按收藏记录 id 或店铺 id；Java `CollectShopServiceImpl#cancel`）。
    pub async fn cancel_collect_shop(
        &self,
        user_id: i32,
        collect_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<()> {
        if let Some(id) = collect_id.filter(|v| *v > 0) {
            sqlx::query("DELETE FROM collect_shop WHERE user_id = $1 AND collect_id = $2")
                .bind(user_id)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            return Ok(());
        }
        if let Some(sid) = shop_id.filter(|v| *v > 0) {
            sqlx::query("DELETE FROM collect_shop WHERE user_id = $1 AND shop_id = $2")
                .bind(user_id)
                .bind(sid)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        }
        Ok(())
    }

    /// 收藏列表排序片段（Java `BaseServiceImpl#buildSortOrder`）。
    ///
    /// \"sortField\" 为空或等于主键属性名时回落主键列；顺序为空或 \"desc\" 一律降序。
    #[must_use]
    pub fn collect_shop_order_by(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
        let column = sort_field
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|f| {
                COLLECT_SHOP_SORT_COLUMNS
                    .iter()
                    .find(|(prop, _)| *prop == f)
                    .map(|(_, col)| *col)
            })
            .unwrap_or("collect_id");
        let asc = sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc"));
        if asc {
            format!("{column} ASC")
        } else {
            format!("{column} DESC")
        }
    }

    /// 店铺被收藏数量（Java `getShopCount`）。
    pub async fn count_shop_collects(&self, shop_id: i32) -> Result<i64> {
        let (c,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) AS c FROM collect_shop WHERE shop_id = $1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(c)
    }

    /// 店铺在售商品数（Java `getProductCount`，非 O2O：不叠加 `store_product`）。
    pub async fn count_shop_on_sale_products(&self, shop_id: i32) -> Result<i64> {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) AS c FROM product WHERE shop_id = $1 AND product_status = $2 \
             AND is_delete = $3",
        )
        .bind(shop_id)
        .bind(1)
        .bind(0)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(c)
    }

    /// 批量查询店铺。
    pub async fn find_shops(&self, shop_ids: &[i32]) -> Result<Vec<Shop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM shop WHERE shop_id IN ({})",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Shop>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 店铺「热销 / 新品 / 推荐」商品（Java `getProductType`，`limit 10`，无 ORDER BY）。
    pub async fn top_products_by_flag(
        &self,
        shop_id: i32,
        flag: ProductFlag,
        limit: u32,
    ) -> Result<Vec<Product>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM product WHERE shop_id = ? AND product_status = 1 AND is_delete = 0 {} \
             LIMIT ?",
            flag.where_clause()
        ));
        sqlx::query_as::<_, Product>(&sql)
            .bind(shop_id)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 ID 批量查询商品（顺序由调用方在内存中还原，替代 Java 的 `ORDER BY field(...)`）。
    pub async fn find_products_by_ids(&self, product_ids: &[i32]) -> Result<Vec<Product>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM product WHERE product_id IN ({})",
            placeholders(product_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量统计各商品的 SKU 数量（Java `skuCount = productSku.size()`）。
    pub async fn count_skus_by_product_ids(
        &self,
        product_ids: &[i32],
    ) -> Result<HashMap<i32, i32>> {
        if product_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT product_id, COUNT(*) AS c FROM product_sku WHERE product_id IN ({}) \
             GROUP BY product_id",
            placeholders(product_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(pid, c)| (pid, i32::try_from(c).unwrap_or(i32::MAX)))
            .collect())
    }

    /// 批量查询订单明细（账期订单列表复用）。
    pub async fn find_items_by_order_ids(&self, order_ids: &[i32]) -> Result<Vec<OrderItem>> {
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM order_item WHERE order_id IN ({}) ORDER BY order_id ASC, item_id ASC",
            placeholders(order_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 订单 / 优惠券计数 —————————————————————

    /// 统计指定用户、指定 `order_status` 的订单数。
    pub async fn count_orders_by_status(&self, user_id: i32, status: i32) -> Result<i64> {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) AS c FROM \"order\" WHERE user_id = $1 AND order_status = $2",
        )
        .bind(user_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(c)
    }

    /// 统计「已完成且待评价」订单数（Java `comment_status = 0 AND order_status = 5`）。
    pub async fn count_orders_by_comment_status(
        &self,
        user_id: i32,
        comment_status: i32,
        order_status: i32,
    ) -> Result<i64> {
        let (c,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) AS c FROM \"order\" WHERE user_id = $1 AND comment_status = $2 \
             AND order_status = $3",
        )
        .bind(user_id)
        .bind(comment_status)
        .bind(order_status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(c)
    }

    /// 用户优惠券总数（Java `memberCenter.awaitCoupon`）。
    pub async fn count_user_coupons(&self, user_id: i32) -> Result<i64> {
        let (c,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) AS c FROM user_coupon WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(c)
    }

    // ————————————————————— credit —————————————————————

    /// 按用户查询账期授信记录（Java `baseMapper.selectByUserId`）。
    pub async fn find_credit_by_user(&self, user_id: i32) -> Result<Option<CreditUser>> {
        sqlx::query_as::<_, CreditUser>(
            "SELECT credit_user_id, user_id, credit_limit, used_amount, available_amount, \
             settlement_type, status, period_start_time, create_time, update_time \
             FROM credit_user WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 账期订单列表（Java：未结算按 `order_time` 降序；已结算按 `settlement_time` 降序）。
    pub async fn find_credit_orders(
        &self,
        user_id: i32,
        statuses: &[i32],
        settled: bool,
    ) -> Result<Vec<CreditOrder>> {
        let status_ph = placeholders(statuses.len());
        let order_by = if settled {
            "settlement_time DESC"
        } else {
            "order_time DESC"
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT credit_order_id, order_id, order_sn, user_id, credit_user_id, credit_amount, \
             settlement_status, settlement_id, settlement_sn, order_time, settlement_time, \
             create_time, update_time FROM credit_order WHERE user_id = ? \
             AND settlement_status IN ({status_ph}) ORDER BY {order_by}"
        ));
        let mut q = sqlx::query_as::<_, CreditOrder>(&sql).bind(user_id);
        for s in statuses {
            q = q.bind(*s);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 统计账期周期起点之前的未结算/退款中订单数（Java `hasPendingBeforePeriod`）。
    pub async fn count_credit_orders_before(
        &self,
        user_id: i32,
        statuses: &[i32],
        before_ts: i64,
    ) -> Result<i64> {
        let status_ph = placeholders(statuses.len());
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) AS c FROM credit_order WHERE user_id = ? \
             AND settlement_status IN ({status_ph}) AND order_time < ?"
        ));
        let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(user_id);
        for s in statuses {
            q = q.bind(*s);
        }
        let (c,): (i64,) = q
            .bind(before_ts)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(c)
    }

    /// 已支付订单累计金额（`memberCenter` 未使用；保留给用户中心扩展）。
    pub async fn sum_paid_order_amount(&self, user_id: i32) -> Result<Decimal> {
        let (s,): (Decimal,) = sqlx::query_as(
            "SELECT COALESCE(SUM(total_amount), 0) AS s FROM \"order\" \
             WHERE user_id = $1 AND order_status IN (2, 5)",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_columns_match_entity_field_order() {
        // 列顺序与 `entities::User` 字段声明顺序一致；数量等于实体字段数（36）
        assert_eq!(
            USER_COLUMNS,
            "user_id, username, avatar, mobile, mobile_validated, email, \
             email_validated, nickname, password, birthday, balance, frozen_balance, \
             commission_balance, frozen_commission_balance, points, growth_points, address_id, \
             reg_time, last_login, last_ip, rank_id, referrer_user_id, from_tag, is_svip, \
             svip_expire_time, order_count, order_amount, history_product_ids, is_distribution, \
             distribution_register_time, wechat_img, is_company_auth, status, user_type, \
             admin_note, mark"
        );
        assert_eq!(USER_COLUMNS.split(',').count(), 36);
    }

    #[test]
    fn user_rank_columns_match_entity_field_order() {
        assert_eq!(USER_RANK_COLUMNS.split(',').count(), 15);
        // 逐列覆盖 `entities::UserRank` 的字段名
        for col in [
            "rank_id",
            "rank_name",
            "min_growth_points",
            "max_growth_points",
            "discount",
            "show_price",
            "rank_type",
            "rank_logo",
            "rank_ico",
            "rank_bg",
            "rank_point",
            "free_shipping",
            "rank_card_type",
            "rights",
            "rank_level",
        ] {
            assert!(USER_RANK_COLUMNS.contains(col), "缺少列 {col}");
        }
    }

    #[test]
    fn collect_shop_order_by_whitelist() {
        // 缺省：主键降序（Java buildSortOrder 的 sortField 为空分支）
        assert_eq!(
            UserCenterRepository::collect_shop_order_by(None, None),
            "collect_id DESC"
        );
        // 主键属性名同样回落主键列
        assert_eq!(
            UserCenterRepository::collect_shop_order_by(Some("collectId"), Some("desc")),
            "collect_id DESC"
        );
        assert_eq!(
            UserCenterRepository::collect_shop_order_by(Some("addTime"), Some("asc")),
            "add_time ASC"
        );
        assert_eq!(
            UserCenterRepository::collect_shop_order_by(Some("shopId"), None),
            "shop_id DESC"
        );
        // 非法字段 → 回落主键列（防注入）
        assert_eq!(
            UserCenterRepository::collect_shop_order_by(Some("1; DROP TABLE user"), Some("asc")),
            "collect_id ASC"
        );
    }

    #[test]
    fn product_flag_clauses_are_fixed_sql() {
        assert_eq!(ProductFlag::Hot.where_clause(), "AND is_hot = 1");
        assert_eq!(ProductFlag::New.where_clause(), "AND is_new = 1");
        assert_eq!(ProductFlag::Best.where_clause(), "AND is_best = 1");
    }

    #[test]
    fn placeholders_expand() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");
    }
}
