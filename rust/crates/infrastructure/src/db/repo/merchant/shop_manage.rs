//! 店铺信息域（merchant::shop_manage）仓储：`shop` / `admin_user` / `admin_user_shop` /
//! `admin_role` / `admin_log` / `merchant` / `store_tip` / `tip` /
//! `store_product` / `area_store_manager_shop` / `config` / `product`。
//!
//! 列名严格对齐 `java/database/pure.sql`（用
//! `python tools/schema_parser.py --table <表名>` 核对）；
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//!
//! 说明：
//! - `shop.add_time` 为**秒级** `int`；日期区间对齐 Java
//!   `StringUtils.dateToTimestampExample(date + " 00:00:00" / " 23:59:59")`
//!   → `[当日 00:00:00, 当日 23:59:59]`。
//! - 列表的 `shopType` 过滤对齐 Java `ShopServiceImpl#list`：
//!   `isMyShop != 1` 时按单个 `shop_type` 等值，否则按 `shopTypes` 集合。
//!   为免动态拼 SQL，用「等值片段（集合为空时绑定单值）」+
//!   「集合片段（集合为空时不生成）」组合表达。
//! - `corporateName` 需 JOIN `merchant`、`areaStoreManagerId` 需 EXISTS
//!   `area_store_manager_shop`；都用子查询而非 LEFT JOIN，使列名可被
//!   `tools/check_sql_columns.py` 的单表归因校验覆盖。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{AdminLog, AdminUser, AdminUserShop, Merchant, Shop, StoreTip, Tip};
use nextrade_core::models::merchant::shop_manage::ShopListQuery;
use nextrade_core::{Error, Result};

/// 售后终态：已完成（Java `AftersalesStatusEnum.COMPLETE`）。
const AFTERSALES_STATUS_COMPLETE: i32 = 6;
/// 售后终态：已取消（Java `AftersalesStatusEnum.CANCEL`）。
const AFTERSALES_STATUS_CANCEL: i32 = 7;

/// 待绑定参数（按 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

#[derive(Clone)]
pub struct ShopManageRepository {
    pool: PgPool,
}

impl ShopManageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 店铺（`shop`） —————————————————————

    /// 店铺分页（对应 Java `ShopServiceImpl#list` 的查询构造器部分）。
    ///
    /// `shop_id_scope`：登录态店铺视角（Java `getShopId()`）；
    /// `None` 或 `<= 0` 表示平台视角（不限定）。
    /// `manager_shop_ids`：`mainAccount` / `account` 命中的店铺集合
    /// （Java `adminUserService.getBindMainShopIds`），空切片表示「无命中」。
    pub async fn paginate_shops(
        &self,
        q: &ShopListQuery,
        shop_id_scope: Option<i32>,
        manager_shop_ids: Option<&[i32]>,
    ) -> Result<(u64, Vec<Shop>)> {
        let (where_sql, binds) = build_filter(q, shop_id_scope, manager_shop_ids)?;
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"shop\" s{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM \"shop\" s{where_sql}{} LIMIT ? OFFSET ?",
            order_by_clause(q)
        ));
        let rows = bind_all!(sqlx::query_as::<_, Shop>(&rows_sql), binds)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 店铺下拉列表（对应 Java `ShopServiceImpl#getShopList`，
    /// `ShopController.shopList`）。
    ///
    /// Java：`shop_type = ?` + 未删除 +（`shopTitle` 或 `keyword` 模糊）。
    pub async fn find_shops_for_picker(
        &self,
        shop_type: i32,
        shop_title: Option<&str>,
        keyword: Option<&str>,
    ) -> Result<Vec<Shop>> {
        sqlx::query_as::<_, Shop>(
            "SELECT * FROM \"shop\"
              WHERE shop_type = $1
                AND (is_delete IS NULL OR is_delete = 0)
                AND (
                      ($2 IS NULL OR shop_title LIKE CONCAT('%', $3, '%'))
                   OR ($4 IS NULL OR shop_title LIKE CONCAT('%', $5, '%'))
                )
              ORDER BY shop_id DESC",
        )
        .bind(shop_type)
        .bind(shop_title)
        .bind(shop_title)
        .bind(keyword)
        .bind(keyword)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 查店铺（Java `BaseServiceImpl#getById`）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM \"shop\" WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 同商户同类型未删店铺数（Java `updateShop` 的 `maxShopCount` 校验）。
    pub async fn count_shops_by_merchant(&self, merchant_id: i32, shop_type: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"shop\"
              WHERE merchant_id = $1 AND shop_type = $2 AND (is_delete IS NULL OR is_delete = 0)",
        )
        .bind(merchant_id)
        .bind(shop_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 该店铺下的自提点数（Java `delShop` 的「门店下有自提点」校验）。
    pub async fn count_pickups_under(&self, store_parent_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"shop\"
              WHERE store_parent_id = $1 AND (is_delete IS NULL OR is_delete = 0)",
        )
        .bind(store_parent_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 店铺下的商品数（Java `productService.count(...)`，`delShop` 校验）。
    pub async fn count_products_of_shop(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM product WHERE shop_id = $1")
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 插入店铺（Java `save(shopData)` + `assignRandomShopId`）。
    pub async fn insert_shop(&self, shop: &Shop) -> Result<i32> {
        let _ = sqlx::query(
            "INSERT INTO \"shop\"
               (shop_id, shop_title, add_time, shop_logo, status, merchant_id,
                shop_money, frozen_money, contact_mobile, \"description\",
                kefu_phone, kefu_weixin, kefu_link, is_contact_kefu, kefu_inlet,
                ship_info, receive_info, last_login_time,
                vendor_set_price_type, vendor_set_price_auto_value,
                shop_type, store_parent_id, shop_cover_picture, shop_show_picture,
                shop_contact_json, shop_open_close_json, shop_region_id,
                shop_detailed_address, shop_longitude, shop_latitude,
                shop_star, shop_star_num, shop_sales, use_shop_category,
                shop_show_category, is_delete)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                     $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36)",
        )
        .bind(shop.shop_id)
        .bind(shop.shop_title.as_deref())
        .bind(shop.add_time)
        .bind(shop.shop_logo.as_deref())
        .bind(shop.status)
        .bind(shop.merchant_id)
        .bind(shop.shop_money)
        .bind(shop.frozen_money)
        .bind(shop.contact_mobile.as_deref())
        .bind(shop.description.as_deref())
        .bind(shop.kefu_phone.as_deref())
        .bind(shop.kefu_weixin.as_deref())
        .bind(shop.kefu_link.as_deref())
        .bind(shop.is_contact_kefu)
        .bind(shop.kefu_inlet.as_deref())
        .bind(shop.ship_info.as_deref())
        .bind(shop.receive_info.as_deref())
        .bind(shop.last_login_time)
        .bind(shop.vendor_set_price_type)
        .bind(shop.vendor_set_price_auto_value)
        .bind(shop.shop_type)
        .bind(shop.store_parent_id)
        .bind(shop.shop_cover_picture.as_deref())
        .bind(shop.shop_show_picture.as_deref())
        .bind(shop.shop_contact_json.as_deref())
        .bind(shop.shop_open_close_json.as_deref())
        .bind(shop.shop_region_id.as_deref())
        .bind(shop.shop_detailed_address.as_deref())
        .bind(shop.shop_longitude)
        .bind(shop.shop_latitude)
        .bind(shop.shop_star)
        .bind(shop.shop_star_num)
        .bind(shop.shop_sales)
        .bind(shop.use_shop_category)
        .bind(shop.shop_show_category.as_deref())
        .bind(shop.is_delete)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        // `shop_id` 由服务层显式生成（Java `assignRandomShopId`），
        // 直接用实体上的值，该列非自增，不走 RETURNING。
        Ok(shop.shop_id)
    }

    /// 更新店铺可写列（Java `ShopServiceImpl#updateShop` → `updateById`）。
    ///
    /// `COALESCE(?, col)` 对齐 MyBatis-Plus 的 `FieldStrategy.NOT_NULL`
    /// （`null` 字段不参与更新）。返回是否命中行。
    pub async fn update_shop(&self, shop: &Shop) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE \"shop\" SET
                shop_title = COALESCE($1, shop_title),
                shop_logo = COALESCE($2, shop_logo),
                contact_mobile = COALESCE($3, contact_mobile),
                shop_type = COALESCE($4, shop_type),
                store_parent_id = COALESCE($5, store_parent_id),
                shop_cover_picture = COALESCE($6, shop_cover_picture),
                shop_show_picture = COALESCE($7, shop_show_picture),
                \"description\" = COALESCE($8, \"description\"),
                shop_contact_json = COALESCE($9, shop_contact_json),
                shop_open_close_json = COALESCE($10, shop_open_close_json),
                shop_region_id = COALESCE($11, shop_region_id),
                shop_detailed_address = COALESCE($12, shop_detailed_address),
                shop_longitude = COALESCE($13, shop_longitude),
                shop_latitude = COALESCE($14, shop_latitude),
                status = COALESCE($15, status),
                shop_show_category = COALESCE($16, shop_show_category),
                use_shop_category = COALESCE($17, use_shop_category),
                kefu_inlet = COALESCE($18, kefu_inlet),
                kefu_phone = COALESCE($19, kefu_phone),
                kefu_weixin = COALESCE($20, kefu_weixin),
                kefu_link = COALESCE($21, kefu_link),
                is_contact_kefu = COALESCE($22, is_contact_kefu),
                ship_info = COALESCE($23, ship_info),
                receive_info = COALESCE($24, receive_info)
             WHERE shop_id = $25",
        )
        .bind(shop.shop_title.as_deref())
        .bind(shop.shop_logo.as_deref())
        .bind(shop.contact_mobile.as_deref())
        .bind(shop.shop_type)
        .bind(shop.store_parent_id)
        .bind(shop.shop_cover_picture.as_deref())
        .bind(shop.shop_show_picture.as_deref())
        .bind(shop.description.as_deref())
        .bind(shop.shop_contact_json.as_deref())
        .bind(shop.shop_open_close_json.as_deref())
        .bind(shop.shop_region_id.as_deref())
        .bind(shop.shop_detailed_address.as_deref())
        .bind(shop.shop_longitude)
        .bind(shop.shop_latitude)
        .bind(shop.status)
        .bind(shop.shop_show_category.as_deref())
        .bind(shop.use_shop_category)
        .bind(shop.kefu_inlet.as_deref())
        .bind(shop.kefu_phone.as_deref())
        .bind(shop.kefu_weixin.as_deref())
        .bind(shop.kefu_link.as_deref())
        .bind(shop.is_contact_kefu)
        .bind(shop.ship_info.as_deref())
        .bind(shop.receive_info.as_deref())
        .bind(shop.shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 更新最后登录时间（Java `currentShop` 的 `lambdaUpdate().set(lastLoginTime)`）。
    pub async fn touch_last_login(&self, shop_id: i32, ts: i32) -> Result<bool> {
        let res = sqlx::query("UPDATE \"shop\" SET last_login_time = $1 WHERE shop_id = $2")
            .bind(ts)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 更新店铺状态（`updateField` 白名单命中 `status`）。
    pub async fn update_shop_status(&self, shop_id: i32, status: i32) -> Result<bool> {
        let res = sqlx::query("UPDATE \"shop\" SET status = $1 WHERE shop_id = $2")
            .bind(status)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 硬删除店铺（Java `BaseServiceImpl#removeById` / `del` / `batch("del")`）。
    pub async fn delete_shops(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "DELETE FROM \"shop\" WHERE shop_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 软删除店铺（Java `delShop`：`is_delete = 1`）。
    pub async fn soft_delete_shop(&self, shop_id: i32) -> Result<bool> {
        let res = sqlx::query("UPDATE \"shop\" SET is_delete = 1 WHERE shop_id = $1")
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 供应商配置写入（Java `updateVendorSetting`）。
    ///
    /// `vendor_set_price_type` 恒写（Java 未判空）；
    /// `vendor_set_price_auto_value` 仅在 Java `ObjectUtil.isNotEmpty` 时写入。
    pub async fn update_vendor_setting(
        &self,
        shop_id: i32,
        price_type: Option<i32>,
        auto_value: Option<Decimal>,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE \"shop\" SET
                vendor_set_price_type = $1,
                vendor_set_price_auto_value = COALESCE($2, vendor_set_price_auto_value)
             WHERE shop_id = $3",
        )
        .bind(price_type)
        .bind(auto_value)
        .bind(shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量把店铺内商品下架（Java `setting` 停业后 `product.setStatus(0)`）。
    pub async fn off_shelf_shop_products(&self, shop_id: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE product SET product_status = 0 WHERE shop_id = $1")
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 店铺商品评分 `AVG(comment_rank)`（`detailWithScore`）。
    pub async fn avg_shop_comment_rank(&self, shop_id: i32) -> Result<Option<Decimal>> {
        let row: Option<(Option<Decimal>,)> = sqlx::query_as(
            "SELECT AVG(comment_rank) FROM comment WHERE shop_id = $1 GROUP BY shop_id",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— 商户（`merchant`） —————————————————————

    /// 批量查商户（列表/详情回填 `merchant`），替代逐行 N+1。
    pub async fn find_merchants(&self, ids: &[i32]) -> Result<Vec<Merchant>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM merchant WHERE merchant_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Merchant>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按账号名（模糊）取绑定店铺 ID（Java `AdminUserMapper#getBindMainShopIds`）。
    ///
    /// `only_admin = true` 时限定 `admin_user_shop.is_admin = 1`。
    pub async fn find_bind_shop_ids_by_account(
        &self,
        account: &str,
        only_admin: bool,
    ) -> Result<Vec<i32>> {
        let sql = if only_admin {
            "SELECT s.shop_id FROM admin_user u
               JOIN admin_user_shop s ON s.admin_id = u.admin_id
              WHERE u.username LIKE CONCAT('%', $1, '%') AND s.is_admin = 1"
        } else {
            "SELECT s.shop_id FROM admin_user u
               JOIN admin_user_shop s ON s.admin_id = u.admin_id
              WHERE u.username LIKE CONCAT('%', $1, '%')"
        };
        let rows: Vec<(i32,)> = sqlx::query_as(sql)
            .bind(account)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ————————————————————— 管理员（`admin_user`） —————————————————————

    /// 按 ID 查管理员。
    pub async fn find_admin_user(&self, admin_id: i32) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_user WHERE admin_id = $1 LIMIT 1")
            .bind(admin_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按 ID 查管理员（列表回填店铺管理员账号）。
    pub async fn find_admin_users(&self, ids: &[i32]) -> Result<Vec<AdminUser>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM admin_user WHERE admin_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, AdminUser>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 role_id 查角色（`choose` 的 RBAC 分支）。
    pub async fn find_admin_role(
        &self,
        role_id: i32,
    ) -> Result<Option<nextrade_core::entities::AdminRole>> {
        sqlx::query_as::<_, nextrade_core::entities::AdminRole>(
            "SELECT * FROM admin_role WHERE role_id = $1 LIMIT 1",
        )
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某商户的主账号（Java `MerchantUserService#getDetailByMerchantId(merchantId, true)`）。
    ///
    /// Java `needAdmin = true` 时严格限定 `merchant_user.is_admin = 1`
    /// 并 `limit 1`；查不到返回 `None`（调用方回退当前登录管理员）。
    pub async fn find_merchant_main_admin_user_id(
        &self,
        merchant_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT admin_user_id FROM merchant_user
              WHERE merchant_id = $1 AND is_admin = 1
              ORDER BY merchant_user_id ASC LIMIT 1",
        )
        .bind(merchant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— 店铺员工绑定（`admin_user_shop`） —————————————————————

    /// 本页店铺的「主管理员」绑定（`is_admin = 1`，每店取一条）。
    pub async fn find_shop_admin_bindings(&self, shop_ids: &[i32]) -> Result<Vec<AdminUserShop>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM admin_user_shop
              WHERE is_admin = 1 AND shop_id IN ({})
              ORDER BY id ASC",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, AdminUserShop>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某管理员在某店铺的绑定（`detail(onlyCurrentAdmin)` / `setPosLoginDefault`）。
    pub async fn find_admin_shop_binding(
        &self,
        shop_id: i32,
        admin_id: i32,
    ) -> Result<Option<AdminUserShop>> {
        sqlx::query_as::<_, AdminUserShop>(
            "SELECT * FROM admin_user_shop
              WHERE shop_id = $1 AND admin_id = $2 LIMIT 1",
        )
        .bind(shop_id)
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 管理员启用的店铺 ID 集合（Java `AdminUserShopService#getShopIds`：
    /// 仅 `is_using = 0`）。
    pub async fn find_enabled_shop_ids(&self, admin_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM admin_user_shop WHERE admin_id = $1 AND is_using = 0")
                .bind(admin_id)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 店铺下所有绑定记录（`staffShow` 中「员工日志涉及的管理员」集合）。
    pub async fn find_shop_bindings(
        &self,
        shop_id: i32,
        exclude_role_ids: &[i32],
    ) -> Result<Vec<AdminUserShop>> {
        let mut sql = String::from("SELECT * FROM admin_user_shop WHERE shop_id = $1");
        if !exclude_role_ids.is_empty() {
            sql.push_str(&format!(
                " AND role_id NOT IN ({})",
                placeholders(exclude_role_ids.len())
            ));
        }
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, AdminUserShop>(&sql);
        q = q.bind(shop_id);
        for rid in exclude_role_ids {
            q = q.bind(*rid);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 店铺员工统计 `(using_user, sub_using_user, stop_using_user)`。
    ///
    /// - `using_user`：`is_using = 0` 且非收银角色
    /// - `sub_using_user`：`is_using = 0` 且 `is_admin = 0` 且非收银角色
    /// - `stop_using_user`：`is_using = 1` 且非收银角色
    pub async fn count_staff(
        &self,
        shop_id: i32,
        exclude_role_ids: &[i32],
    ) -> Result<(i64, i64, i64)> {
        let mut sql = String::from(
            "SELECT
               SUM(CASE WHEN is_using = 0 THEN 1 ELSE 0 END),
               SUM(CASE WHEN is_using = 0 AND is_admin = 0 THEN 1 ELSE 0 END),
               SUM(CASE WHEN is_using = 1 THEN 1 ELSE 0 END)
             FROM admin_user_shop WHERE shop_id = $1",
        );
        if !exclude_role_ids.is_empty() {
            sql.push_str(&format!(
                " AND role_id NOT IN ({})",
                placeholders(exclude_role_ids.len())
            ));
        }
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>)>(&sql);
        q = q.bind(shop_id);
        for rid in exclude_role_ids {
            q = q.bind(*rid);
        }
        let row = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0), row.2.unwrap_or(0)))
    }

    /// 收银员角色 ID 集合（Java `ShopServiceImpl#posCashierRoleIds`：
    /// `admin_role.role_code = 'pos_cashier'`）。
    pub async fn find_pos_cashier_role_ids(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT role_id FROM admin_role WHERE role_code = 'pos_cashier'")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 更新「登录后默认进收银台」开关。
    pub async fn update_default_pos_login(
        &self,
        binding_id: i32,
        default_pos_login: i32,
    ) -> Result<bool> {
        let res = sqlx::query("UPDATE admin_user_shop SET default_pos_login = $1 WHERE id = $2")
            .bind(default_pos_login)
            .bind(binding_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 新增店铺管理员绑定（Java `AdminUserShopService#createAdminUserShop`）。
    ///
    /// `role_id` 由 Java 的角色初始化返回；角色域未迁移，这里落 0
    /// （见服务层模块头第 3 条）。
    pub async fn insert_admin_user_shop(
        &self,
        shop_id: i32,
        admin_id: i32,
        username: &str,
        email: &str,
        add_time: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO admin_user_shop
               (admin_id, user_id, shop_id, username, email, auth_list,
                is_using, is_admin, add_time, role_id, default_pos_login, last_login_system)
             VALUES ($1, 0, $2, $3, $4, '[\"all\"]', 0, 1, $5, 0, 0, '')",
        )
        .bind(admin_id)
        .bind(shop_id)
        .bind(username)
        .bind(email)
        .bind(add_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 记录上次登录端。
    pub async fn update_last_login_system(
        &self,
        binding_id: i32,
        last_login_system: &str,
    ) -> Result<bool> {
        let res = sqlx::query("UPDATE admin_user_shop SET last_login_system = $1 WHERE id = $2")
            .bind(last_login_system)
            .bind(binding_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 「登录后默认进收银台」的门店 ID（Java `getDefaultPosShopId`）。
    pub async fn find_default_pos_shop_id(&self, admin_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT shop_id FROM admin_user_shop
              WHERE admin_id = $1 AND default_pos_login = 1 AND is_using = 0
              LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 最近一次登录端（Java `getLastLoginSystem`：非空且按 `add_time` 倒序取一）。
    pub async fn find_last_login_system(&self, admin_id: i32) -> Result<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT last_login_system FROM admin_user_shop
              WHERE admin_id = $1 AND last_login_system <> ''
              ORDER BY add_time DESC LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0).unwrap_or_default())
    }

    // ————————————————————— 操作日志（`admin_log`） —————————————————————

    /// 最近 N 条员工操作日志（Java `staffShow`：`order by log_id desc limit 5`）。
    pub async fn find_recent_admin_logs(
        &self,
        admin_ids: &[i32],
        limit: u64,
    ) -> Result<Vec<AdminLog>> {
        if admin_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM admin_log WHERE user_id IN ({}) ORDER BY log_id DESC LIMIT ?",
            placeholders(admin_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, AdminLog>(&sql);
        for id in admin_ids {
            q = q.bind(*id);
        }
        q.bind(limit as i64).fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 门店标签（`store_tip` / `tip`） —————————————————————

    /// 店铺关联标签（按 `sort` 升序，与 Java `store_tip` 查询一致）。
    pub async fn find_store_tips(&self, shop_id: i32) -> Result<Vec<StoreTip>> {
        sqlx::query_as::<_, StoreTip>(
            "SELECT * FROM store_tip WHERE shop_id = $1 ORDER BY sort ASC, store_tip_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量（按 `tip_id`）查标签名。
    pub async fn find_tips(&self, tip_ids: &[i32]) -> Result<Vec<Tip>> {
        if tip_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM tip WHERE tip_id IN ({})",
            placeholders(tip_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Tip>(&sql);
        for id in tip_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 删除店铺全部标签（Java `update` 中先删后插）。
    pub async fn delete_store_tips(&self, shop_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM store_tip WHERE shop_id = $1")
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量插入店铺标签（`sort` 从 0 递增，`add_time` 为毫秒时间戳）。
    pub async fn insert_store_tips(
        &self,
        shop_id: i32,
        tip_ids: &[i32],
        add_time_millis: i64,
    ) -> Result<()> {
        if tip_ids.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for (sort, tip_id) in tip_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO store_tip (shop_id, tip_id, sort, add_time) VALUES ($1, $2, $3, $4)",
            )
            .bind(shop_id)
            .bind(*tip_id)
            .bind(sort as i32)
            .bind(add_time_millis)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 门店商品（`store_product`） —————————————————————

    /// 已分配某商品的门店 ID（Java `getStoreListByProductId` 的 `count > 0` 判定）。
    pub async fn find_shop_ids_by_product(&self, product_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT shop_id FROM store_product WHERE product_id = $1")
                .bind(product_id)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0 as i32).collect())
    }

    // ————————————————————— 系统配置（`config`） —————————————————————

    /// 按 `biz_code` 取配置值（Java `ConfigService#getConfigVal`；
    /// 缺失或空值返回空串）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<String> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0).unwrap_or_default())
    }

    // ————————————————————— 订单 / 售后（`setting` 停业前置校验） —————————————————————

    /// 进行中订单数（Java `ShopServiceImpl#setting`：
    /// `order_status IN (0, 1, 2)` 且 `order.is_del = 0`）。
    pub async fn count_ongoing_orders(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"order\"
              WHERE shop_id = $1 AND is_del = 0 AND order_status IN (0, 1, 2)",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 未完结售后数（Java `ShopServiceImpl#setting`：
    /// `aftersales.status NOT IN (完成, 取消)`）。
    pub async fn count_ongoing_aftersales(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM aftersales WHERE shop_id = $1 AND status NOT IN ($2, $3)",
        )
        .bind(shop_id)
        .bind(AFTERSALES_STATUS_COMPLETE)
        .bind(AFTERSALES_STATUS_CANCEL)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }
}

/// 店铺列表过滤条件与绑定参数（顺序严格一致）。
fn build_filter(
    q: &ShopListQuery,
    shop_id_scope: Option<i32>,
    manager_shop_ids: Option<&[i32]>,
) -> Result<(String, Vec<Bind>)> {
    let mut sql = String::from(" WHERE (is_delete IS NULL OR is_delete = 0)");
    let mut binds: Vec<Bind> = Vec::new();

    // shop_type：Java 在 isMyShop != 1 时按单值等值，否则按 shopTypes 集合
    let (single, listed): (Option<i32>, Vec<i32>) = if q.is_my_shop_flag() == 1 {
        match q.shop_types.as_ref().filter(|v| !v.is_empty()) {
            Some(types) => (None, types.clone()),
            None => (Some(q.shop_type_or_default()), Vec::new()),
        }
    } else {
        (Some(q.shop_type_or_default()), Vec::new())
    };

    if listed.is_empty() {
        // 集合片段不生成；等值片段绑定单值（None → 0 与任何 shop_type 都不等，
        // 但 None 只可能出现在「isMyShop=1 且 shopTypes 非空」分支，
        // 该分支不会走到这里，故不会误过滤）。
        sql.push_str(" AND (? IS NULL OR shop_type = ?)");
        binds.push(Bind::I32(single.unwrap_or(0)));
        binds.push(Bind::I32(single.unwrap_or(0)));
    } else {
        sql.push_str(&format!(" AND shop_type IN ({})", placeholders(listed.len())));
        for t in &listed {
            binds.push(Bind::I32(*t));
        }
    }

    // 状态：原 Java 仅 status > 0 才过滤
    if let Some(status) = q.status.filter(|s| *s > 0) {
        sql.push_str(" AND status = ?");
        binds.push(Bind::I32(status));
    }

    // 关键字：Java `list` 只用父类 `keyword`；`shopList` 接口用 `shopTitle`。
    // 两者取并集（任一命中即匹配）。
    if let Some(kw) = first_non_empty(&[q.keyword.as_deref(), q.shop_title.as_deref()]) {
        sql.push_str(" AND shop_title LIKE CONCAT('%', ?, '%')");
        binds.push(Bind::Str(kw.to_string()));
    }

    // 创建日期区间（含当天 00:00:00 / 23:59:59）
    if let Some(v) = q.add_start_time.as_deref().filter(|s| !s.trim().is_empty()) {
        sql.push_str(" AND add_time >= ?");
        binds.push(Bind::I64(i64::from(day_start_seconds(v)?)));
    }
    if let Some(v) = q.add_end_time.as_deref().filter(|s| !s.trim().is_empty()) {
        sql.push_str(" AND add_time <= ?");
        binds.push(Bind::I64(i64::from(day_end_seconds(v)?)));
    }

    if let Some(parent) = q.store_parent_id {
        sql.push_str(" AND store_parent_id = ?");
        binds.push(Bind::I32(parent));
    }

    if let Some(region) = q.shop_region_id.as_deref().filter(|s| !s.trim().is_empty()) {
        sql.push_str(" AND shop_region_id LIKE CONCAT('%', ?, '%')");
        binds.push(Bind::Str(region.to_string()));
    }

    // 主账号 / 账号筛选：Java 先查绑定店铺集合，空集直接返回空页
    if q.main_account
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
        || q.account.as_deref().is_some_and(|s| !s.trim().is_empty())
    {
        let ids = manager_shop_ids.unwrap_or(&[]);
        if ids.is_empty() {
            // 空集合语义：恒假（Java 直接返回空 Page）
            sql.push_str(" AND 1 = 0");
        } else {
            sql.push_str(&format!(" AND shop_id IN ({})", placeholders(ids.len())));
            for id in ids {
                binds.push(Bind::I32(*id));
            }
        }
    }

    // 显式 shopIds 优先；否则用登录态店铺
    match q.shop_ids.as_ref().filter(|v| !v.is_empty()) {
        Some(ids) => {
            sql.push_str(&format!(" AND shop_id IN ({})", placeholders(ids.len())));
            for id in ids {
                binds.push(Bind::I32(*id));
            }
        }
        None => {
            if let Some(scope) = shop_id_scope.filter(|s| *s > 0) {
                sql.push_str(" AND shop_id = ?");
                binds.push(Bind::I32(scope));
            }
        }
    }

    // 区域经理筛选（Java LEFT JOIN area_store_manager_shop）
    if let Some(mid) = q.area_store_manager_id {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM area_store_manager_shop asm
                           WHERE asm.shop_id = s.shop_id AND asm.area_store_manager_id = $1)",
        );
        binds.push(Bind::I32(mid));
    }

    // 商户公司名筛选（Java LEFT JOIN merchant）
    if let Some(name) = q.corporate_name.as_deref().filter(|s| !s.trim().is_empty()) {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM merchant m
                           WHERE m.merchant_id = s.merchant_id
                             AND (m.is_delete IS NULL OR m.is_delete = 0)
                             AND m.corporate_name LIKE CONCAT('%', $1, '%'))",
        );
        binds.push(Bind::Str(name.to_string()));
    }

    Ok((sql, binds))
}

/// 排序子句：Java `BaseServiceImpl#buildSortOrder` + `getColumnByPropertyName`。
///
/// `sortField` 为空或不在白名单时回落主键；方向非 `asc` 一律 `DESC`。
fn order_by_clause(q: &ShopListQuery) -> String {
    const PAIRS: [(&str, &str); 6] = [
        ("shopId", "shop_id"),
        ("lastLoginTime", "last_login_time"),
        ("addTime", "add_time"),
        ("status", "status"),
        ("shopSales", "shop_sales"),
        ("shopStar", "shop_star"),
    ];
    let field = q.sort_field.as_deref().unwrap_or("").trim();
    let col = PAIRS
        .iter()
        .find(|(prop, col)| prop.eq_ignore_ascii_case(field) || col.eq_ignore_ascii_case(field))
        .map(|(_, col)| *col)
        .unwrap_or("shop_id");
    let dir = if q
        .sort_order
        .as_deref()
        .is_some_and(|o| o.eq_ignore_ascii_case("asc"))
    {
        "ASC"
    } else {
        "DESC"
    };
    format!(" ORDER BY \"{col}\" {dir}")
}

/// `IN (?, ?, ?)` 占位符（长度由调用方控制，恒为合法 SQL 片段）。
fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(", ")
}

/// 取第一个非空白字符串。
fn first_non_empty<'a>(candidates: &[Option<&'a str>]) -> Option<&'a str> {
    candidates
        .iter()
        .flatten()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
}

/// `yyyy-MM-dd` → 当日 00:00:00 的秒级时间戳（本地时区）。
fn day_start_seconds(date: &str) -> Result<i32> {
    let (y, m, d) = parse_ymd(date)?;
    Ok(local_seconds(y, m, d, 0, 0, 0))
}

/// `yyyy-MM-dd` → 当日 23:59:59 的秒级时间戳（本地时区）。
fn day_end_seconds(date: &str) -> Result<i32> {
    let (y, m, d) = parse_ymd(date)?;
    Ok(local_seconds(y, m, d, 23, 59, 59))
}

/// 解析 `yyyy-MM-dd`（允许带 ` HH:mm:ss` 后缀，按日期部分处理）。
fn parse_ymd(date: &str) -> Result<(i32, u32, u32)> {
    let head = date.trim().split(' ').next().unwrap_or("").trim();
    let mut it = head.split('-');
    let y = it.next().and_then(|v| v.trim().parse::<i32>().ok());
    let m = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    let d = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d) => {
            Ok((y, m, d))
        }
        _ => Err(Error::Validation(format!("日期格式应为 yyyy-MM-dd: {date}"))),
    }
}

/// 本地时区日期时间 → 秒级时间戳。
fn local_seconds(y: i32, m: u32, d: u32, hh: u32, mm: u32, ss: u32) -> i32 {
    use chrono::TimeZone;
    chrono::Local
        .with_ymd_and_hms(y, m, d, hh, mm, ss)
        .single()
        .map(|dt| dt.timestamp() as i32)
        .unwrap_or(0)
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("店铺不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_defaults_to_not_deleted_and_shop_type_one() {
        let q = ShopListQuery::default();
        let (sql, binds) = build_filter(&q, None, None).unwrap();
        assert!(sql.contains("(is_delete IS NULL OR is_delete = 0)"));
        // 默认 shopType = 1 → 等值片段
        assert!(sql.contains("(? IS NULL OR shop_type = ?)"));
        assert_eq!(binds.len(), 2);
        assert_eq!(order_by_clause(&q), " ORDER BY \"shop_id\" DESC");
    }

    #[test]
    fn filter_my_shop_uses_shop_types_list() {
        let q: ShopListQuery = serde_json::from_str(r#"{"isMyShop":1,"shopTypes":[2,3]}"#).unwrap();
        let (sql, binds) = build_filter(&q, None, None).unwrap();
        assert!(sql.contains("shop_type IN (?, ?)"));
        assert_eq!(binds.len(), 2);

        // isMyShop = 1 且 shopTypes 为空 → 退化为单值 shopType（默认 1）
        let q: ShopListQuery = serde_json::from_str(r#"{"isMyShop":1}"#).unwrap();
        let (sql, _) = build_filter(&q, None, None).unwrap();
        assert!(sql.contains("(? IS NULL OR shop_type = ?)"));
    }

    #[test]
    fn filter_binds_match_placeholder_count() {
        let q: ShopListQuery = serde_json::from_str(
            r#"{"shopType":2,"status":1,"keyword":"店","addStartTime":"2026-01-01",
                "addEndTime":"2026-01-31","storeParentId":9,"shopRegionId":"11",
                "shopIds":[1,2,3],"areaStoreManagerId":4,"corporateName":"公司"}"#,
        )
        .unwrap();
        let (sql, binds) = build_filter(&q, Some(7), None).unwrap();
        assert_eq!(sql.matches('?').count(), binds.len(), "SQL: {sql}");
        assert!(sql.contains("shop_id IN (?, ?, ?)"));
        assert!(sql.contains("area_store_manager_shop"));
        assert!(sql.contains("merchant m"));
        // 显式 shopIds 存在时不叠加登录态 shop_id 等值
        assert!(!sql.contains("AND shop_id = ?"));
    }

    #[test]
    fn filter_scope_applies_when_no_shop_ids() {
        let q = ShopListQuery::default();
        let (sql, binds) = build_filter(&q, Some(12), None).unwrap();
        assert!(sql.contains("AND shop_id = ?"));
        assert_eq!(sql.matches('?').count(), binds.len());

        // 登录态为平台（0 / None）不加店铺条件
        let (sql, _) = build_filter(&q, Some(0), None).unwrap();
        assert!(!sql.contains("AND shop_id = ?"));
    }

    #[test]
    fn filter_account_with_empty_bindings_is_false() {
        let q: ShopListQuery = serde_json::from_str(r#"{"mainAccount":"nobody"}"#).unwrap();
        let (sql, _) = build_filter(&q, None, Some(&[])).unwrap();
        assert!(sql.contains("AND 1 = 0"));

        let (sql, binds) = build_filter(&q, None, Some(&[1, 2])).unwrap();
        assert!(sql.contains("shop_id IN (?, ?)"));
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn order_by_whitelist_and_fallback() {
        let q: ShopListQuery =
            serde_json::from_str(r#"{"sortField":"lastLoginTime","sortOrder":"desc"}"#).unwrap();
        assert_eq!(order_by_clause(&q), " ORDER BY \"last_login_time\" DESC");

        let q: ShopListQuery =
            serde_json::from_str(r#"{"sortField":"addTime","sortOrder":"asc"}"#).unwrap();
        assert_eq!(order_by_clause(&q), " ORDER BY \"add_time\" ASC");

        // 未知字段（注入尝试）回落主键
        let q: ShopListQuery =
            serde_json::from_str(r#"{"sortField":"1;DROP TABLE shop","sortOrder":"asc"}"#).unwrap();
        assert_eq!(order_by_clause(&q), " ORDER BY \"shop_id\" ASC");
    }

    #[test]
    fn date_bounds_are_day_start_and_end() {
        let s = day_start_seconds("2026-01-01").unwrap();
        let e = day_end_seconds("2026-01-31").unwrap();
        assert!(e > s);
        let span = i64::from(e) - i64::from(s);
        let expected = 30 * 86_400 + 86_399;
        assert!(
            (expected - 7_200..=expected + 7_200).contains(&span),
            "span = {span}"
        );
        assert!(day_start_seconds("2026-13-01").is_err());
        assert!(day_start_seconds("bad").is_err());
    }

    #[test]
    fn placeholders_and_first_non_empty() {
        assert_eq!(placeholders(3), "?, ?, ?");
        assert_eq!(first_non_empty(&[None, Some("  "), Some(" x ")]), Some("x"));
        assert_eq!(first_non_empty(&[None, Some("  ")]), None);
    }
}
