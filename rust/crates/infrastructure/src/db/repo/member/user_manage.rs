//! 会员管理域（member::user_manage）仓储：`user` / `user_rank` / `user_address` /
//! `order` / `user_member_tag` / `member_tag` / `member_tag_group` /
//! `user_balance_log` / `user_points_log` / `user_growth_points_log`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table <表名>`）：
//! - `user`：`user_id` / `username` / `avatar` / `mobile` / `mobile_validated` /
//!   `email` / `email_validated` / `nickname` / `password` / `birthday` / `balance` /
//!   `frozen_balance` / `points` / `growth_points` / `address_id` / `reg_time` /
//!   `last_login` / `last_ip` / `rank_id` / `referrer_user_id` / `from_tag` /
//!   `is_svip` / `svip_expire_time` / `order_count` / `order_amount` /
//!   `history_product_ids` / `is_distribution` / `distribution_register_time` /
//!   `wechat_img` / `is_company_auth` / `status` / `user_type` / `admin_note` / `mark`
//! - `user_rank`：`rank_id` / `rank_name` / `rank_ico` / `rank_logo` / `discount`
//! - `user_address`：`address_id` / `user_id` / `is_default` / `is_selected` /
//!   `region_ids` / `region_names` / `position_address` / `mobile_area_code` /
//!   `is_system` / `latitude` / `longitude`
//! - `user_member_tag`：`id` / `shop_id` / `user_id` / `tag_id` / `create_time`
//! - `member_tag`：`id` / `shop_id` / `group_id` / `tag_name` / `is_del`
//! - `member_tag_group`：`id` / `shop_id` / `group_name` / `is_del`
//! - `user_balance_log`：`before_balance` / `balance` / `after_balance` /
//!   `frozen_balance` / `before_frozen_balance` / `after_frozen_balance` /
//!   `wallet_type` / `new_balance` / `new_frozen_balance`
//! - `user_points_log`：`points` / `change_time` / `change_desc` / `change_type`
//! - `user_growth_points_log`：`points` / `change_time` / `change_desc` / `change_type`
//!
//! 注意：`order` 是 MySQL 保留字，SQL 中必须使用反引号。
//! 查询统一使用运行时 `sqlx::query_as`（不使用 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{
    MemberTag, MemberTagGroup, User, UserAddress, UserBalanceLog, UserGrowthPointsLog,
    UserMemberTag, UserPointsLog, UserRank,
};
use nextrade_core::models::member::user_manage::{
    change_type_name, fmt_time, from_tag_name, CreateUserParam, FundDetailItem, SetAdminNoteParam,
    UpdateUserParam, UserAddressVO, UserBalanceLogVO, UserFundDetailQuery, UserGrowthPointsLogVO,
    UserListQuery, UserMemberTagVO, UserPointsLogVO, UserVO, CHANGE_TYPE_INC, FROM_TAG_ADMIN_ADD,
    FUND_TAG_GROWTH, FUND_TAG_POINTS,
};
use nextrade_core::{Error, Result};

/// 待绑定参数（按 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
pub enum Bind {
    /// 32 位整数
    I32(i32),
    /// 64 位整数
    I64(i64),
    /// 字符串
    Str(String),
    /// 金额
    Dec(Decimal),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
                Bind::Dec(v) => q.bind(*v),
            };
        }
        q
    }};
}

/// 会员列表分页筛选条件（由 [`UserListQuery`] 归一化而来）。
///
/// 逐条对应 Java `UserServiceImpl#list` 中的 wrapper 链；`tag_user_ids`
/// 为 `Some` 时对应 Java 的 `.in(User::getUserId, tagFilterUserIds)`。
#[derive(Debug, Clone, Default)]
pub struct UserPageFilter {
    /// 关键词（Java `dto.getKeyword()`，非空白才生效）
    pub keyword: Option<String>,
    /// 精确匹配（Java `Boolean.TRUE.equals(dto.getIsExact())`）
    pub exact: bool,
    /// 标签筛选命中的用户ID（Java `tagFilterUserIds`）
    pub tag_user_ids: Option<Vec<i32>>,
    /// 会员类型（Java 缺省 `UserTypeEnum.MEMBER`）
    pub user_type: i32,
    /// 来源标签（`> 0` 才生效，对应 Java `fromTag != null && fromTag > 0`）
    pub from_tag: Option<i32>,
    /// 会员等级（`> 0` 才生效）
    pub rank_id: Option<i32>,
    /// 积分大于（`> 0` 才生效）
    pub points_gt: Option<Decimal>,
    /// 积分小于（`> 0` 才生效）
    pub points_lt: Option<Decimal>,
    /// 余额严格大于
    pub balance_gt: Option<Decimal>,
    /// 标记等值
    pub mark: Option<i32>,
    /// 店铺隔离作用域：`Some(shop_id)` 时仅该店铺会员（`shop_member` 关联）
    pub shop_scope: Option<i32>,
    /// 分页大小
    pub size: i64,
    /// 偏移量
    pub offset: i64,
}

impl UserPageFilter {
    /// 由列表查询条件构造筛选条件（对应 Java `UserServiceImpl#list` 的 wrapper 链）。
    ///
    /// `tag_user_ids` 由服务层算出（`None` = 不按标签筛选；`Some([])` = 无命中，
    /// 服务层会直接返回空页，与 Java `tagFilterUserIds.isEmpty()` 分支一致）。
    #[must_use]
    pub fn from_query(q: &UserListQuery, tag_user_ids: Option<Vec<i32>>) -> Self {
        let (_, size) = q.page_params();
        let (_, offset) = q.limit_offset();
        Self {
            keyword: q
                .keyword
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string),
            exact: q.is_exact == Some(true),
            tag_user_ids,
            user_type: q.effective_user_type(),
            from_tag: q.from_tag.filter(|v| *v > 0),
            rank_id: q.rank_id.filter(|v| *v > 0),
            points_gt: q.points_gt.filter(|v| *v > Decimal::ZERO),
            points_lt: q.points_lt.filter(|v| *v > Decimal::ZERO),
            balance_gt: q.balance,
            mark: q.mark,
            shop_scope: None,
            size: size as i64,
            offset,
        }
    }
}

/// 会员更新字段集（对应 Java `UserParam` 经 `BeanUtils.copyProperties` 后的
/// 非空字段；MyBatis-Plus `updateById` 默认 `NOT_NULL` 策略）。
#[derive(Debug, Clone, Default)]
pub struct UserEdit {
    /// 用户ID（WHERE 条件，必填）
    pub user_id: i32,
    /// 用户名
    pub username: Option<String>,
    /// 头像URL
    pub avatar: Option<String>,
    /// 手机号
    pub mobile: Option<String>,
    /// 手机号是否验证
    pub mobile_validated: Option<i32>,
    /// 邮箱
    pub email: Option<String>,
    /// 邮箱是否验证
    pub email_validated: Option<i32>,
    /// 昵称
    pub nickname: Option<String>,
    /// 密码（已加密；Java 清空密码时传 `null` 表示不更新）
    pub password: Option<String>,
    /// 生日
    pub birthday: Option<String>,
    /// 余额
    pub balance: Option<Decimal>,
    /// 冻结余额
    pub frozen_balance: Option<Decimal>,
    /// 积分
    pub points: Option<i32>,
    /// 成长积分
    pub growth_points: Option<i32>,
    /// 地址ID
    pub address_id: Option<i32>,
    /// 等级ID
    pub rank_id: Option<i32>,
    /// 推荐人用户ID
    pub referrer_user_id: Option<i32>,
    /// 是否SVIP
    pub is_svip: Option<i32>,
    /// SVIP过期时间
    pub svip_expire_time: Option<i32>,
    /// 是否分销
    pub is_distribution: Option<i32>,
    /// 微信头像
    pub wechat_img: Option<String>,
    /// 是否企业认证
    pub is_company_auth: Option<i32>,
    /// 会员状态
    pub status: Option<i32>,
    /// 管理员备注
    pub admin_note: Option<String>,
    /// 标记
    pub mark: Option<i32>,
}

#[derive(Clone)]
pub struct UserManageRepository {
    pool: PgPool,
}

impl UserManageRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 会员列表 —————————————————————

    /// 会员是否属于指定店铺（`shop_member` 有效关联）。
    pub async fn user_in_shop(&self, user_id: i32, shop_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM shop_member WHERE user_id = $1 AND shop_id = $2 AND is_del = 0 LIMIT 1",
        )
        .bind(user_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 会员分页（对应 Java `UserServiceImpl#list`）。
    ///
    /// 返回 `(总数, 当前页会员实体)`。
    pub async fn paginate_users(&self, f: &UserPageFilter) -> Result<(u64, Vec<User>)> {
        let (where_sql, binds) = build_filter(f);

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"user\"{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        // Java `BaseServiceImpl#buildSortOrder` 未指定字段时按主键降序
        let rows_sql =
            crate::db::to_pg_placeholders(&format!("SELECT * FROM \"user\"{where_sql} ORDER BY user_id DESC LIMIT ? OFFSET ?"));
        let rows = bind_all!(sqlx::query_as::<_, User>(&rows_sql), binds)
            .bind(f.size as i64)
            .bind(f.offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 标签筛选命中的用户ID（对应 Java `list` 中按 `user_member_tag` 的筛选）。
    ///
    /// `match_all = false`（Java 默认的 OR 模式）→ 命中任意一个标签即可；
    /// `match_all = true`（Java `tagMatchMode == 1`）→ 用户的标签集合必须
    /// **包含**全部传入标签（可以有多余标签），与 Java 的
    /// `containsAll(requiredTagIds)` 语义一致。
    pub async fn tag_filter_user_ids(&self, tag_ids: &[i64], match_all: bool) -> Result<Vec<i32>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; tag_ids.len()].join(", ");
        let rows: Vec<(i64,)> = if match_all {
            let sql = crate::db::to_pg_placeholders(&format!(
                "SELECT user_id FROM user_member_tag
                  WHERE tag_id IN ({placeholders})
                  GROUP BY user_id
                 HAVING COUNT(DISTINCT tag_id) = ?"
            ));
            let mut q = sqlx::query_as::<_, (i64,)>(&sql);
            for id in tag_ids {
                q = q.bind(*id);
            }
            q.bind(i64::try_from(tag_ids.len()).unwrap_or(i64::MAX))
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            let sql = crate::db::to_pg_placeholders(&format!(
                "SELECT DISTINCT user_id FROM user_member_tag WHERE tag_id IN ({placeholders})"
            ));
            let mut q = sqlx::query_as::<_, (i64,)>(&sql);
            for id in tag_ids {
                q = q.bind(*id);
            }
            q.fetch_all(&self.pool).await.map_err(map_err)?
        };
        Ok(rows
            .into_iter()
            .filter_map(|(id,)| i32::try_from(id).ok())
            .collect())
    }

    // ————————————————————— 会员详情 —————————————————————

    /// 按会员ID查询（对应 Java `this.getOne(eq(User::getUserId, id))`）。
    pub async fn find_user(&self, user_id: i32) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按用户名 / 手机号精确命中一个会员（对应 Java `SalesmanServiceImpl
    /// #getUserByMobileOrUsername` 的 `eq(mobile).or().eq(username)`）。
    pub async fn find_user_by_mobile_or_username(&self, search: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE mobile = $1 OR LOWER(username) = LOWER($2) LIMIT 1")
            .bind(search)
            .bind(search)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 统计用户名重复数（Java `checkUserInfo("username", value)`）。
    pub async fn count_by_username(&self, username: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE LOWER(username) = LOWER($1)")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 统计手机号重复数（Java `checkUserInfo("mobile", value)`）。
    pub async fn count_by_mobile(&self, mobile: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE mobile = $1")
            .bind(mobile)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 统计邮箱重复数（`exclude_user_id` 非空时排除自身，
    /// 对应 Java `update` 的 `.ne(User::getUserId, userId)`）。
    pub async fn count_by_email(&self, email: &str, exclude_user_id: Option<i32>) -> Result<i64> {
        let row: (i64,) = match exclude_user_id {
            Some(id) => {
                sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE LOWER(email) = LOWER($1) AND user_id <> $2")
                    .bind(email)
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
            }
            None => {
                sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE LOWER(email) = LOWER($1)")
                    .bind(email)
                    .fetch_one(&self.pool)
                    .await
            }
        }
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 统计手机号重复数（可排除自身，对应 Java `update` 的手机号唯一校验）。
    pub async fn count_by_mobile_excluding(
        &self,
        mobile: &str,
        exclude_user_id: Option<i32>,
    ) -> Result<i64> {
        let row: (i64,) = match exclude_user_id {
            Some(id) => {
                sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE mobile = $1 AND user_id <> $2")
                    .bind(mobile)
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await
            }
            None => {
                sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE mobile = $1")
                    .bind(mobile)
                    .fetch_one(&self.pool)
                    .await
            }
        }
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批量查询会员等级（列表页按当前页 `rank_id` 一次取回，消除 N+1；
    /// 对应 Java `userRankService.getById`）。
    pub async fn find_ranks(&self, rank_ids: &[i32]) -> Result<Vec<UserRank>> {
        if rank_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; rank_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM user_rank WHERE rank_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, UserRank>(&sql);
        for id in rank_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量查询会员地址（Java 逐个 `orderByDesc(is_default).last("limit 1")`；
    /// 这里一次取回后由服务层按 `user_id` 取首条，消除 N+1）。
    pub async fn find_addresses(&self, user_ids: &[i32]) -> Result<Vec<UserAddress>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM user_address WHERE user_id IN ({placeholders})
              ORDER BY user_id ASC, is_default DESC, address_id ASC"
        ));
        let mut q = sqlx::query_as::<_, UserAddress>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量统计「已支付」订单数与金额（对应 Java `convertToDTO` 中
    /// `eq(Order::getUserId, uid).eq(Order::getPayStatus, 2)` 的 list + reduce）。
    ///
    /// 返回 `(user_id, 订单数, 金额合计)`。
    pub async fn paid_order_stats(&self, user_ids: &[i32]) -> Result<Vec<(i32, i64, Decimal)>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT o.user_id, COUNT(*) AS cnt,
                    CAST(COALESCE(SUM(o.total_amount), 0) AS DECIMAL(20,2)) AS amt
               FROM \"order\" o
              WHERE o.pay_status = 2 AND o.user_id IN ({placeholders})
              GROUP BY o.user_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64, Decimal)>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 会员标签 —————————————————————

    /// 某个会员的标签列表（对应 Java `UserMemberTagServiceImpl#listByUser`）。
    ///
    /// `shop_id` 为店铺上下文（Java `HeaderUtils.getShopId()`），
    /// Rust 侧由服务层决定取值（当前固定平台 `0`）。
    /// `tag_name` / `group_id` / `group_name` 在标签不存在时保持 `None`，
    /// 与 Java「仅在 `tag != null` 分支内 set」一致。
    pub async fn find_user_member_tags(
        &self,
        user_id: i64,
        shop_id: i32,
    ) -> Result<Vec<UserMemberTagVO>> {
        let rows: Vec<(i64, Option<i64>, Option<String>, Option<i64>, Option<String>)> =
            sqlx::query_as(
                "SELECT ut.tag_id,
                        ut.create_time,
                        t.tag_name,
                        t.group_id,
                        g.group_name
                   FROM user_member_tag ut
                   LEFT JOIN member_tag t
                          ON t.id = ut.tag_id AND t.shop_id = ut.shop_id AND t.is_del = 0
                   LEFT JOIN member_tag_group g
                          ON g.id = t.group_id AND g.shop_id = t.shop_id AND g.is_del = 0
                  WHERE ut.user_id = $1 AND ut.shop_id = $2
                  ORDER BY ut.id ASC",
            )
            .bind(user_id)
            .bind(shop_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(tag_id, create_time, tag_name, group_id, group_name)| UserMemberTagVO {
                    tag_id,
                    tag_name: tag_name.clone(),
                    group_id,
                    // Java：tag 存在但组不存在时 groupName 为 ""（getOrDefault）
                    group_name: match (tag_name, group_name) {
                        (Some(_), Some(name)) => Some(name),
                        (Some(_), None) => Some(String::new()),
                        (None, _) => None,
                    },
                    // Java `createTime != null ? createTime.toString() : null`
                    create_time: create_time.map(|v| v.to_string()),
                },
            )
            .collect())
    }

    /// 批量查询会员标签关联（预取用，避免逐会员查库）。
    pub async fn find_member_tags(&self, user_ids: &[i64]) -> Result<Vec<UserMemberTag>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM user_member_tag WHERE user_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, UserMemberTag>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量查询标签（列表页 / 详情页共用）。
    pub async fn find_tags(&self, tag_ids: &[i64]) -> Result<Vec<MemberTag>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; tag_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM member_tag WHERE id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, MemberTag>(&sql);
        for id in tag_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量查询标签组。
    pub async fn find_tag_groups(&self, group_ids: &[i64]) -> Result<Vec<MemberTagGroup>> {
        if group_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; group_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM member_tag_group WHERE id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, MemberTagGroup>(&sql);
        for id in group_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 新增 / 更新 / 删除 —————————————————————

    /// 新增会员，返回自增 `user_id`（对应 Java `this.save(user)`）。
    ///
    /// 未在 [`CreateUserParam`] 中出现的列一律取 SQL 默认值（Java
    /// `BeanUtils.copyProperties` 亦不写入这些列，落库后为表默认值）。
    pub async fn insert_user(
        &self,
        p: &CreateUserParam,
        password: &str,
        reg_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO \"user\"
               (username, avatar, mobile, mobile_validated, email, email_validated, nickname,
                password, birthday, balance, frozen_balance, points, growth_points, address_id,
                reg_time, rank_id, referrer_user_id, from_tag, is_svip, svip_expire_time,
                is_distribution, wechat_img, is_company_auth, status, user_type, admin_note, mark)
             VALUES ($1,  $2,  $3,  $4,  $5,  $6,  $7,  $8, NULLIF($9, ''),  $10,  $11,  $12,  $13,  $14,  $15,  $16,  $17,  $18,  $19,  $20,  $21,  $22,  $23,  $24,  $25,  $26,  $27) RETURNING user_id",
        )
        .bind(p.username.as_deref().unwrap_or_default())
        .bind(p.avatar.as_deref().unwrap_or_default())
        .bind(p.mobile.as_deref().unwrap_or_default())
        .bind(p.mobile_validated.unwrap_or(0))
        .bind(p.email.as_deref().unwrap_or_default())
        .bind(p.email_validated.unwrap_or(0))
        .bind(&p.nickname)
        .bind(password)
        .bind(&p.birthday)
        .bind(p.balance.unwrap_or(Decimal::ZERO))
        .bind(p.frozen_balance.unwrap_or(Decimal::ZERO))
        .bind(p.points.unwrap_or(0))
        .bind(p.growth_points.unwrap_or(0))
        .bind(p.address_id.unwrap_or(0))
        .bind(reg_time)
        .bind(p.rank_id.unwrap_or(0))
        .bind(p.referrer_user_id.unwrap_or(0))
        // Java `UserController#create` / `batchImportUser` 都强制
        // `user.setFromTag(FromTag.ADMIN_ADD.getCode())`（=7），落库恒为「后台添加」；
        // `None` 时回退该常量，保证与 Java 逐字节一致。
        .bind(p.from_tag.unwrap_or(FROM_TAG_ADMIN_ADD))
        .bind(p.is_svip)
        .bind(p.svip_expire_time)
        .bind(p.is_distribution)
        .bind(p.wechat_img.as_deref().unwrap_or_default())
        .bind(p.is_company_auth.unwrap_or(0))
        .bind(p.status)
        .bind(p.user_type)
        .bind(p.admin_note.as_deref().unwrap_or_default())
        .bind(p.mark.unwrap_or(0))
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按非空字段更新会员（对应 Java `this.updateById(user)` 的
    /// MyBatis-Plus `NOT_NULL` 策略）。返回受影响行数。
    pub async fn update_user(&self, e: &UserEdit) -> Result<u64> {
        let mut sets: Vec<&str> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();

        macro_rules! push_opt {
            ($field:expr, $col:literal, $variant:ident) => {
                if let Some(v) = $field {
                    sets.push(concat!($col, " = ?"));
                    // `Clone::clone` 对 `&String` / `&i32` / `&i64` / `&Decimal`
                    // 都能得到对应 `Bind` 变体所需的拥有值（`ToString` 只适配 `Str`）。
                    binds.push(Bind::$variant(Clone::clone(v)));
                }
            };
        }

        push_opt!(e.username.as_ref(), "username", Str);
        push_opt!(e.avatar.as_ref(), "avatar", Str);
        push_opt!(e.mobile.as_ref(), "mobile", Str);
        push_opt!(e.mobile_validated.as_ref(), "mobile_validated", I32);
        push_opt!(e.email.as_ref(), "email", Str);
        push_opt!(e.email_validated.as_ref(), "email_validated", I32);
        push_opt!(e.nickname.as_ref(), "nickname", Str);
        push_opt!(e.password.as_ref(), "password", Str);
        push_opt!(e.birthday.as_ref(), "birthday", Str);
        push_opt!(e.balance.as_ref(), "balance", Dec);
        push_opt!(e.frozen_balance.as_ref(), "frozen_balance", Dec);
        push_opt!(e.points.as_ref(), "points", I32);
        push_opt!(e.growth_points.as_ref(), "growth_points", I32);
        push_opt!(e.address_id.as_ref(), "address_id", I32);
        push_opt!(e.rank_id.as_ref(), "rank_id", I32);
        push_opt!(e.referrer_user_id.as_ref(), "referrer_user_id", I32);
        push_opt!(e.is_svip.as_ref(), "is_svip", I32);
        push_opt!(e.svip_expire_time.as_ref(), "svip_expire_time", I32);
        push_opt!(e.is_distribution.as_ref(), "is_distribution", I32);
        push_opt!(e.wechat_img.as_ref(), "wechat_img", Str);
        push_opt!(e.is_company_auth.as_ref(), "is_company_auth", I32);
        push_opt!(e.status.as_ref(), "status", I32);
        push_opt!(e.admin_note.as_ref(), "admin_note", Str);
        push_opt!(e.mark.as_ref(), "mark", I32);

        if sets.is_empty() {
            // Java `updateById` 在无非空字段时不产生 SQL（返回 false）
            return Ok(0);
        }
        let sql = format!("UPDATE \"user\" SET {} WHERE user_id = $1", sets.join(", "));
        let affected = bind_all!(sqlx::query(&sql), binds)
            .bind(e.user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 物理删除会员（对应 Java `BaseServiceImpl#del` → `removeById`；
    /// MyBatis-Plus 在无 `@TableLogic` 标注时执行物理删除）。
    pub async fn delete_users(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM \"user\" WHERE user_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        Ok(q.execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    /// 按白名单字段名更新单个会员（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// 仅接受 `username` / `nickname` / `status` 三列，其他列名一律拒绝，
    /// 避免列名拼接导致 SQL 注入。
    pub async fn update_user_field(&self, user_id: i32, column: &str, value: &str) -> Result<u64> {
        let affected = match column {
            "username" => sqlx::query("UPDATE \"user\" SET username = $1 WHERE user_id = $2")
                .bind(value)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected(),
            "nickname" => sqlx::query("UPDATE \"user\" SET nickname = $1 WHERE user_id = $2")
                .bind(value)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected(),
            // Java 白名单里的 `status` 是 tinyint，`UpdateFieldDTO.val` 为 String；
            // MyBatis 会做隐式转换，这里显式解析为 i32
            "status" => {
                let parsed: i32 = value
                    .trim()
                    .parse()
                    .map_err(|_| Error::Validation("字段值不合法".into()))?;
                sqlx::query("UPDATE \"user\" SET status = $1 WHERE user_id = $2")
                    .bind(parsed)
                    .bind(user_id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?
                    .rows_affected()
            }
            other => {
                return Err(Error::Validation(format!("字段值不合法: {other}")));
            }
        };
        Ok(affected)
    }

    /// 批量设置会员等级（对应 Java `batch` 的 `setRank` 分支）。
    pub async fn update_rank_batch(&self, ids: &[i32], rank_id: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE \"user\" SET rank_id = $1 WHERE user_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql).bind(rank_id);
        for id in ids {
            q = q.bind(*id);
        }
        Ok(q.execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    /// 设置管理员备注与标记（对应 Java `setAdminNote`，`adminNote` / `mark`
    /// 取值为 `None` 时会写入 `NULL`，与 Java `UpdateWrapper.set(col, null)` 一致）。
    pub async fn update_admin_note(&self, p: &SetAdminNoteParam, user_ids: &[i32]) -> Result<u64> {
        if user_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql =
            crate::db::to_pg_placeholders(&format!("UPDATE \"user\" SET admin_note = $1, mark = $2 WHERE user_id IN ({placeholders})"));
        let mut q = sqlx::query(&sql).bind(&p.admin_note).bind(p.mark);
        for id in user_ids {
            q = q.bind(*id);
        }
        Ok(q.execute(&self.pool).await.map_err(map_err)?.rows_affected())
    }

    // ————————————————————— 资金：原子增减 —————————————————————

    /// 余额原子增减（增加：`balance = balance + ?`；减少：
    /// `balance = balance - ? WHERE balance >= ?`，防并发透支）。
    ///
    /// 返回受影响行数：`0` 表示条件不满足（余额不足）。
    pub async fn apply_balance_delta(
        &self,
        user_id: i32,
        change_type: i32,
        amount: Decimal,
    ) -> Result<u64> {
        let affected = if change_type == CHANGE_TYPE_INC {
            sqlx::query("UPDATE \"user\" SET balance = balance + $1 WHERE user_id = $2")
                .bind(amount)
                .bind(user_id)
                .execute(&self.pool)
                .await
        } else {
            sqlx::query("UPDATE \"user\" SET balance = balance - $1 WHERE user_id = $2 AND balance >= $3")
                .bind(amount)
                .bind(user_id)
                .bind(amount)
                .execute(&self.pool)
                .await
        }
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 冻结余额「可用 ↔ 冻结」互转（原子条件更新，防并发透支）。
    ///
    /// `type = 1` 冻结：可用余额 `>= ?` 时 `balance -= ?`、`frozen_balance += ?`；
    /// `type = 2` 解冻：冻结余额 `>= ?` 时反向。返回受影响行数。
    pub async fn apply_frozen_transfer(
        &self,
        user_id: i32,
        change_type: i32,
        amount: Decimal,
    ) -> Result<u64> {
        let affected = if change_type == CHANGE_TYPE_INC {
            sqlx::query(
                "UPDATE \"user\"
                    SET balance = balance - $1,
                        frozen_balance = frozen_balance + $2
                  WHERE user_id = $3 AND balance >= $4",
            )
            .bind(amount)
            .bind(amount)
            .bind(user_id)
            .bind(amount)
            .execute(&self.pool)
            .await
        } else {
            sqlx::query(
                "UPDATE \"user\"
                    SET balance = balance + $1,
                        frozen_balance = frozen_balance - $2
                  WHERE user_id = $3 AND frozen_balance >= $4",
            )
            .bind(amount)
            .bind(amount)
            .bind(user_id)
            .bind(amount)
            .execute(&self.pool)
            .await
        }
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 积分原子增减（增加：`points = points + ?`；减少：
    /// `points = points - ? WHERE points >= ?`）。
    ///
    /// 返回受影响行数：`0` 表示条件不满足（积分不足）。
    pub async fn apply_points_delta(
        &self,
        user_id: i32,
        change_type: i32,
        points: i32,
    ) -> Result<u64> {
        let affected = if change_type == CHANGE_TYPE_INC {
            sqlx::query("UPDATE \"user\" SET points = points + $1 WHERE user_id = $2")
                .bind(points)
                .bind(user_id)
                .execute(&self.pool)
                .await
        } else {
            sqlx::query("UPDATE \"user\" SET points = points - $1 WHERE user_id = $2 AND points >= $3")
                .bind(points)
                .bind(user_id)
                .bind(points)
                .execute(&self.pool)
                .await
        }
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 成长积分原子增减（条件同 [`Self::apply_points_delta`]）。
    pub async fn apply_growth_points_delta(
        &self,
        user_id: i32,
        change_type: i32,
        points: i32,
    ) -> Result<u64> {
        let affected = if change_type == CHANGE_TYPE_INC {
            sqlx::query("UPDATE \"user\" SET growth_points = growth_points + $1 WHERE user_id = $2")
                .bind(points)
                .bind(user_id)
                .execute(&self.pool)
                .await
        } else {
            sqlx::query(
                "UPDATE \"user\"
                    SET growth_points = growth_points - $1
                  WHERE user_id = $2 AND growth_points >= $3",
            )
            .bind(points)
            .bind(user_id)
            .bind(points)
            .execute(&self.pool)
            .await
        }
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    // ————————————————————— 资金流水 —————————————————————

    /// 写入余额日志（对应 Java `UserServiceImpl#saveUserBalanceLog`）。
    ///
    /// `new_balance` / `new_frozen_balance` 在 Java 的 builder 中未赋值
    /// （落库为表默认 `0.00`），此处保持一致。
    pub async fn insert_balance_log(&self, log: &UserBalanceLog) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_balance_log
               (user_id, before_balance, balance, after_balance, frozen_balance,
                before_frozen_balance, after_frozen_balance, change_time, change_desc,
                change_type, wallet_type, new_balance, new_frozen_balance)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(log.user_id)
        .bind(log.before_balance)
        .bind(log.balance)
        .bind(log.after_balance)
        .bind(log.frozen_balance)
        .bind(log.before_frozen_balance)
        .bind(log.after_frozen_balance)
        .bind(log.change_time)
        .bind(&log.change_desc)
        .bind(log.change_type)
        .bind(log.wallet_type)
        .bind(log.new_balance)
        .bind(log.new_frozen_balance)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 写入消费积分日志（对应 Java `saveUserPointsLog`）。
    pub async fn insert_points_log(
        &self,
        user_id: i32,
        points: i32,
        change_type: i32,
        change_desc: &str,
        change_time: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_points_log
               (user_id, points, change_time, change_desc, change_type)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(points)
        .bind(change_time)
        .bind(change_desc)
        .bind(change_type)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 写入成长积分日志（对应 Java `saveUserGrowthPointsLog`）。
    pub async fn insert_growth_points_log(
        &self,
        user_id: i32,
        points: i32,
        change_type: i32,
        change_desc: &str,
        change_time: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_growth_points_log
               (user_id, points, change_time, change_desc, change_type)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(points)
        .bind(change_time)
        .bind(change_desc)
        .bind(change_type)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 资金明细 —————————————————————

    /// 资金明细分页（对应 Java `UserServiceImpl#userFundDetail` 的三条分支）。
    ///
    /// - `fromTag = 3` → `user_growth_points_log`
    /// - `fromTag = 4` → `user_points_log`
    /// - 其他 → `user_balance_log`（附加 `balance > 0 OR frozen_balance > 0`，
    ///   逐字对齐 Java）
    pub async fn paginate_fund_detail(
        &self,
        q: &UserFundDetailQuery,
        user: Option<&User>,
    ) -> Result<(u64, Vec<FundDetailItem>)> {
        let user_id = q.user_id.unwrap_or(0);
        let (size, offset) = q.limit_offset();
        match q.from_tag {
            Some(FUND_TAG_GROWTH) => {
                let total: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM user_growth_points_log WHERE user_id = $1",
                )
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
                let rows = sqlx::query_as::<_, UserGrowthPointsLog>(
                    "SELECT * FROM user_growth_points_log
                      WHERE user_id = $1
                      ORDER BY change_time DESC
                      LIMIT $2 OFFSET $3",
                )
                .bind(user_id)
                .bind(size)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
                Ok((
                    u64::try_from(total.0).unwrap_or(0),
                    rows.into_iter()
                        .map(growth_to_vo)
                        .map(FundDetailItem::Growth)
                        .collect(),
                ))
            }
            Some(FUND_TAG_POINTS) => {
                let total: (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM user_points_log WHERE user_id = $1")
                        .bind(user_id)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(map_err)?;
                let rows = sqlx::query_as::<_, UserPointsLog>(
                    "SELECT * FROM user_points_log
                      WHERE user_id = $1
                      ORDER BY change_time DESC
                      LIMIT $2 OFFSET $3",
                )
                .bind(user_id)
                .bind(size)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
                Ok((
                    u64::try_from(total.0).unwrap_or(0),
                    rows.into_iter()
                        .map(points_to_vo)
                        .map(FundDetailItem::Points)
                        .collect(),
                ))
            }
            _ => {
                let total: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM user_balance_log
                      WHERE user_id = $1 AND (balance > 0 OR frozen_balance > 0)",
                )
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
                let rows = sqlx::query_as::<_, UserBalanceLog>(
                    "SELECT * FROM user_balance_log
                      WHERE user_id = $1 AND (balance > 0 OR frozen_balance > 0)
                      ORDER BY change_time DESC
                      LIMIT $2 OFFSET $3",
                )
                .bind(user_id)
                .bind(size)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
                Ok((
                    u64::try_from(total.0).unwrap_or(0),
                    rows.iter()
                        .map(|r| FundDetailItem::Balance(balance_to_vo(r, user)))
                        .collect(),
                ))
            }
        }
    }
}

/// `user_growth_points_log` → VO（Java `UserGrowthPointsLogVO` 构造器）。
fn growth_to_vo(l: UserGrowthPointsLog) -> UserGrowthPointsLogVO {
    UserGrowthPointsLogVO {
        log_id: l.log_id,
        user_id: l.user_id,
        points: l.points,
        change_time: fmt_time(l.change_time),
        change_desc: l.change_desc,
        change_type: l.change_type,
        change_type_name: change_type_name(l.change_type).to_string(),
    }
}

/// `user_points_log` → VO（Java `UserPointsLogVO` 构造器：`changeTime` 为
/// 秒级时间戳原文 `String.valueOf(...)`，不做格式化）。
fn points_to_vo(l: UserPointsLog) -> UserPointsLogVO {
    UserPointsLogVO {
        log_id: l.log_id,
        user_id: l.user_id,
        points: l.points,
        change_time: l.change_time.to_string(),
        change_desc: l.change_desc,
        change_type: l.change_type,
        change_type_name: change_type_name(l.change_type).to_string(),
    }
}

/// `user_balance_log` → VO（Java `UserBalanceLogVO(log, user)` 构造器）。
///
/// `beforeBalance` / `frozenBalance` 取日志值；`beforeFrozenBalance` 由
/// 「当前冻结余额」反推（Java `getBeforeFrozenBalance`）；
/// `afterBalance` / `afterFrozenBalance` / `username` 保持 `None`（Java 未赋值）。
fn balance_to_vo(l: &UserBalanceLog, user: Option<&User>) -> UserBalanceLogVO {
    let before_frozen = match user {
        None => Decimal::ZERO,
        Some(u) => {
            if l.change_type == CHANGE_TYPE_INC {
                u.frozen_balance - l.frozen_balance
            } else {
                u.frozen_balance + l.frozen_balance
            }
        }
    };
    UserBalanceLogVO {
        log_id: l.log_id,
        user_id: l.user_id,
        username: None,
        balance: l.balance,
        before_balance: l.before_balance,
        after_balance: None,
        frozen_balance: l.frozen_balance,
        before_frozen_balance: before_frozen,
        after_frozen_balance: None,
        change_time: fmt_time(l.change_time),
        change_desc: l.change_desc.clone(),
        change_type: l.change_type,
        change_type_name: change_type_name(l.change_type).to_string(),
    }
}

/// 构造 WHERE 子句（含前导空格）与绑定参数列表。
///
/// 逐条对照 Java `UserServiceImpl#list` 的 wrapper 链，条件与绑定严格同序：
/// 关键词（精确/模糊，`username OR email OR mobile`）→ 标签用户ID →
/// `user_type` → `from_tag` → `rank_id` → `points >` → `points <` →
/// `balance >` → `mark =`。
fn build_filter(f: &UserPageFilter) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    if let Some(kw) = f.keyword.as_deref() {
        let (op, value) = if f.exact {
            ("=", kw.to_string())
        } else {
            ("ILIKE", format!("%{kw}%"))
        };
        conds.push(format!("(username {op} ? OR email {op} ? OR mobile {op} ?)"));
        binds.push(Bind::Str(value.clone()));
        binds.push(Bind::Str(value.clone()));
        binds.push(Bind::Str(value));
    }

    if let Some(ids) = f.tag_user_ids.as_ref() {
        if ids.is_empty() {
            // Java `tagFilterUserIds.isEmpty()` 分支直接返回空页；
            // 到这里说明服务层未短路，用恒假条件兜底。
            conds.push("1 = 0".into());
        } else {
            conds.push(format!("user_id IN ({})", vec!["?"; ids.len()].join(", ")));
            for id in ids {
                binds.push(Bind::I32(*id));
            }
        }
    }

    conds.push("user_type = ?".into());
    binds.push(Bind::I32(f.user_type));

    if let Some(v) = f.from_tag {
        conds.push("from_tag = ?".into());
        binds.push(Bind::I32(v));
    }
    if let Some(v) = f.rank_id {
        conds.push("rank_id = ?".into());
        binds.push(Bind::I32(v));
    }
    if let Some(v) = f.points_gt {
        conds.push("points > ?".into());
        binds.push(Bind::Dec(v));
    }
    if let Some(v) = f.points_lt {
        conds.push("points < ?".into());
        binds.push(Bind::Dec(v));
    }
    if let Some(v) = f.balance_gt {
        conds.push("balance > ?".into());
        binds.push(Bind::Dec(v));
    }
    if let Some(shop) = f.shop_scope {
        conds.push(
            "EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = \"user\".user_id AND m.shop_id = $1 AND m.is_del = 0)"
                .into(),
        );
        binds.push(Bind::I32(shop));
    }
    if let Some(v) = f.mark {
        conds.push("mark = ?".into());
        binds.push(Bind::I32(v));
    }

    if conds.is_empty() {
        return (String::new(), binds);
    }
    (format!(" WHERE {}", conds.join(" AND ")), binds)
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("会员不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// 把 [`User`] 实体映射为列表 / 详情基础 VO（不含关联数据，
/// 关联字段由服务层回填）。
#[must_use]
pub fn user_to_vo(u: &User) -> UserVO {
    UserVO {
        user_id: u.user_id,
        username: u.username.clone(),
        // Java `convertToDTO`：头像为空时取配置项「默认头像」；
        // 配置服务未迁移，此处保留原值（见 uncertain_points）。
        avatar: u.avatar.clone(),
        mobile: u.mobile.clone(),
        mobile_validated: u.mobile_validated,
        email: u.email.clone(),
        email_validated: u.email_validated,
        nickname: u.nickname.clone(),
        // Java `convertToDTO` 显式清空密码
        password: String::new(),
        pwd_confirm: None,
        birthday: u.birthday.clone(),
        balance: u.balance,
        frozen_balance: u.frozen_balance,
        points: u.points,
        growth_points: u.growth_points,
        address_id: u.address_id,
        reg_time: fmt_time(u.reg_time),
        last_login: u.last_login,
        last_ip: u.last_ip.clone(),
        rank_id: u.rank_id,
        referrer_user_id: u.referrer_user_id,
        from_tag: u.from_tag,
        from_tag_name: from_tag_name(u.from_tag).to_string(),
        is_svip: u.is_svip,
        svip_expire_time: u.svip_expire_time,
        // Java：无已支付订单时保留 `user.order_count`（`UserDTO` 由 BeanUtils 拷贝）
        order_count: u.order_count,
        order_amount: u.order_amount.to_string(),
        history_product_ids: u.history_product_ids.clone(),
        is_distribution: u.is_distribution,
        distribution_register_time: u.distribution_register_time,
        wechat_img: u.wechat_img.clone(),
        is_company_auth: u.is_company_auth,
        rank_name: None,
        rank_ico: None,
        rank_logo: None,
        discount: None,
        user_address: None,
        user_type: u.user_type,
        admin_note: u.admin_note.clone(),
        mark: u.mark,
        status: u.status,
    }
}

/// 把会员等级写入 VO 的 `rankName` / `rankIco` / `rankLogo` / `discount`
/// （对应 Java `convertToDTO` 里 `userRank != null` 的分支）。
pub fn apply_rank(vo: &mut UserVO, rank: &UserRank) {
    vo.rank_name = Some(rank.rank_name.clone());
    vo.rank_ico = Some(rank.rank_ico.clone());
    vo.rank_logo = Some(rank.rank_logo.clone());
    vo.discount = Some(rank.discount.normalize().to_string());
}

/// [`UserAddress`] → VO（对应 Java `userAddressService.detail(addressId)`）。
#[must_use]
pub fn address_to_vo(a: &UserAddress) -> UserAddressVO {
    let region_names: Vec<String> = serde_json::from_str(&a.region_names).unwrap_or_default();
    let region_ids: Vec<i32> = serde_json::from_str(&a.region_ids).unwrap_or_default();
    UserAddressVO {
        address: a.address.clone(),
        position_address: a.position_address.clone(),
        address_id: a.address_id,
        consignee: a.consignee.clone(),
        email: a.email.clone(),
        is_default: a.is_default,
        is_selected: a.is_selected,
        mobile: a.mobile.clone(),
        mobile_area_code: a.mobile_area_code.clone(),
        region_name: region_names.join(" "),
        region_names,
        telephone: a.telephone.clone(),
        user_id: a.user_id,
        region_ids,
        postcode: a.postcode.clone(),
        address_tag: a.address_tag.clone(),
        latitude: a.latitude,
        longitude: a.longitude,
        is_system: a.is_system,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_binds(f: UserPageFilter) -> (String, Vec<Bind>) {
        build_filter(&f)
    }

    #[test]
    fn default_filter_only_types_member() {
        let q = UserListQuery::default();
        let f = UserPageFilter::from_query(&q, None);
        let (sql, binds) = with_binds(f);
        assert_eq!(sql, " WHERE user_type = ?");
        assert_eq!(binds.len(), 1);
        match &binds[0] {
            Bind::I32(v) => assert_eq!(*v, 0),
            other => panic!("期望 I32(0)，实际 {other:?}"),
        }
    }

    #[test]
    fn fuzzy_keyword_binds_three_like_params() {
        let q: UserListQuery = serde_json::from_str(r#"{"keyword":"tig"}"#).unwrap();
        let f = UserPageFilter::from_query(&q, None);
        let (sql, binds) = with_binds(f);
        assert!(sql.contains("username ILIKE ? OR email ILIKE ? OR mobile ILIKE ?"));
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn exact_keyword_uses_equality() {
        let q: UserListQuery =
            serde_json::from_str(r#"{"keyword":"13800000000","isExact":true}"#).unwrap();
        let f = UserPageFilter::from_query(&q, None);
        let (sql, _) = with_binds(f);
        assert!(sql.contains("username = ? OR email = ? OR mobile = ?"));
        assert!(!sql.contains("ILIKE"));
    }

    #[test]
    fn zero_and_negative_bounds_are_ignored_like_java() {
        let q: UserListQuery =
            serde_json::from_str(r#"{"fromTag":0,"rankId":0,"pointsGt":"0","pointsLt":"0"}"#)
                .unwrap();
        let f = UserPageFilter::from_query(&q, None);
        let (sql, binds) = with_binds(f);
        assert!(!sql.contains("from_tag"));
        assert!(!sql.contains("rank_id"));
        assert!(!sql.contains("points"));
        assert_eq!(binds.len(), 1); // 仅 user_type
    }

    #[test]
    fn positive_filters_are_bound_in_order() {
        let q: UserListQuery = serde_json::from_str(
            r#"{"fromTag":7,"rankId":2,"pointsGt":"10","pointsLt":"100","balance":"5","mark":3,"userType":1}"#,
        )
        .unwrap();
        let f = UserPageFilter::from_query(&q, None);
        let (sql, binds) = with_binds(f);
        assert!(sql.contains("user_type = ?"));
        assert!(sql.contains("from_tag = ?"));
        assert!(sql.contains("rank_id = ?"));
        assert!(sql.contains("points > ?"));
        assert!(sql.contains("points < ?"));
        assert!(sql.contains("balance > ?"));
        assert!(sql.contains("mark = ?"));
        assert_eq!(sql.matches('?').count(), binds.len());
        // 7 个正向过滤：userType + fromTag + rankId + pointsGt + pointsLt
        // + balance + mark（与上面 7 条 `sql.contains` 断言一一对应）
        assert_eq!(binds.len(), 7);
    }

    #[test]
    fn tag_user_ids_expand_to_in_clause() {
        let q = UserListQuery::default();
        let f = UserPageFilter::from_query(&q, Some(vec![11, 22, 33]));
        let (sql, binds) = with_binds(f);
        assert!(sql.contains("user_id IN (?, ?, ?)"));
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn empty_tag_user_ids_short_circuits() {
        let q = UserListQuery::default();
        let f = UserPageFilter::from_query(&q, Some(Vec::new()));
        let (sql, _) = with_binds(f);
        assert!(sql.contains("1 = 0"));
    }

    #[test]
    fn tag_match_mode_and_dedup() {
        let q: UserListQuery =
            serde_json::from_str(r#"{"tagIds":[5,5,7],"tagMatchMode":1}"#).unwrap();
        assert!(q.tag_match_all());
        assert_eq!(q.tag_ids(), vec![5, 7]);

        let q: UserListQuery = serde_json::from_str(r#"{"tagIds":[5]}"#).unwrap();
        assert!(!q.tag_match_all());
    }

    #[test]
    fn page_params_follow_base_page() {
        let q = UserListQuery::default();
        assert_eq!(q.page_params(), (1, 15));
        assert_eq!(q.limit_offset(), (15, 0));

        let q: UserListQuery = serde_json::from_str(r#"{"page":3,"size":500}"#).unwrap();
        assert_eq!(q.page_params(), (3, 100));
        assert_eq!(q.limit_offset(), (100, 200));

        let q: UserListQuery = serde_json::from_str(r#"{"page":0,"size":0}"#).unwrap();
        assert_eq!(q.page_params(), (1, 1));
    }

    // ————————————————— 资金明细 VO 映射 —————————————————

    #[test]
    fn points_log_keeps_raw_timestamp() {
        let l = UserPointsLog {
            log_id: 1,
            user_id: 9,
            points: 30,
            change_time: 1_700_000_000,
            change_desc: "后台增加积分".into(),
            change_type: 1,
            relation_type: None,
            relation_id: None,
        };
        let vo = points_to_vo(l);
        assert_eq!(vo.change_time, "1700000000");
        assert_eq!(vo.change_type_name, "增加");
    }

    #[test]
    fn growth_log_formats_timestamp() {
        let l = UserGrowthPointsLog {
            log_id: 1,
            user_id: 9,
            points: 5,
            change_time: 1_700_000_000,
            change_desc: "后台增加成长值".into(),
            change_type: 2,
        };
        let vo = growth_to_vo(l);
        assert_eq!(vo.change_time.len(), 19);
        assert_eq!(vo.change_type_name, "减少");
    }

    #[test]
    fn balance_log_derives_before_frozen_balance() {
        let log = UserBalanceLog {
            log_id: 1,
            user_id: 9,
            before_balance: Decimal::new(1000, 2),
            balance: Decimal::new(500, 2),
            change_time: 1_700_000_000,
            change_desc: "后台增加余额".into(),
            change_type: 1,
            frozen_balance: Decimal::new(500, 2),
            ..Default::default()
        };
        let user = User {
            user_id: 9,
            frozen_balance: Decimal::new(2000, 2),
            ..Default::default()
        };
        let vo = balance_to_vo(&log, Some(&user));
        // 1（增加）→ 当前冻结 - 本次冻结 = 20.00 - 5.00 = 15.00
        assert_eq!(vo.before_frozen_balance, Decimal::new(1500, 2));
        // Java 未赋值的字段保持 null
        assert!(vo.after_balance.is_none());
        assert!(vo.username.is_none());

        let dec_log = UserBalanceLog {
            change_type: 2,
            ..log
        };
        let vo = balance_to_vo(&dec_log, Some(&user));
        // 2（减少）→ 当前冻结 + 本次冻结 = 25.00
        assert_eq!(vo.before_frozen_balance, Decimal::new(2500, 2));

        // user 为 null 时 Java 返回 BigDecimal.ZERO
        let vo = balance_to_vo(&dec_log, None);
        assert_eq!(vo.before_frozen_balance, Decimal::ZERO);
    }

    #[test]
    fn change_type_names_match_java_switch() {
        assert_eq!(change_type_name(1), "增加");
        assert_eq!(change_type_name(2), "减少");
        assert_eq!(change_type_name(99), "其他");
        assert_eq!(change_type_name(7), "未知");
    }

    // ————————————————— 实体 → VO —————————————————

    #[test]
    fn user_to_vo_clears_password_and_formats_time() {
        let u = User {
            user_id: 3,
            username: "alice".into(),
            password: "$2a$hash".into(),
            from_tag: 7,
            reg_time: 1_700_000_000,
            order_count: 4,
            order_amount: Decimal::new(12345, 2),
            ..Default::default()
        };
        let vo = user_to_vo(&u);
        assert!(vo.password.is_empty());
        assert_eq!(vo.from_tag_name, "Admin");
        assert_eq!(vo.reg_time.len(), 19);
        assert_eq!(vo.order_amount, "123.45");
        assert_eq!(vo.order_count, 4);
    }

    #[test]
    fn apply_rank_and_address_mapping() {
        let mut vo = UserVO::default();
        let rank = UserRank {
            rank_id: 2,
            rank_name: "黄金".into(),
            rank_ico: "i.png".into(),
            rank_logo: "l.png".into(),
            discount: Decimal::new(95, 1),
            ..Default::default()
        };
        apply_rank(&mut vo, &rank);
        assert_eq!(vo.rank_name.as_deref(), Some("黄金"));
        assert_eq!(vo.discount.as_deref(), Some("9.5"));

        let a = UserAddress {
            address_id: 8,
            user_id: 3,
            consignee: "张三".into(),
            region_ids: "[1,2]".into(),
            region_names: r#"["浙江省","杭州市"]"#.into(),
            address: "文一西路 1 号".into(),
            mobile: "13800000000".into(),
            ..Default::default()
        };
        let addr = address_to_vo(&a);
        assert_eq!(addr.region_names, vec!["浙江省", "杭州市"]);
        assert_eq!(addr.region_name, "浙江省 杭州市");
        assert_eq!(addr.region_ids, vec![1, 2]);
    }
}
