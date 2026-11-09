//! 代客下单优惠券域（trade::proxy_shopping_user_coupon）仓储：`coupon` / `user_coupon` / `promotion` /
//! `user` / `order`。
//!
//! 对应原 Java `UserCouponServiceImpl#claim`（由
//! `ProxyShoppingUserCouponController` `@RequestMapping("/adminapi/proxyShopping/coupon")`
//! 暴露）一条链路上用到的全部 SQL：
//!
//! | 本仓储方法 | 原 Java |
//! |-----------|---------|
//! | [`ProxyShoppingUserCouponRepository::find_coupon`] | `couponMapper.selectById(couponId)` |
//! | [`ProxyShoppingUserCouponRepository::count_user_claimed`] | `lambdaQuery()...count()`（每人已领） |
//! | [`ProxyShoppingUserCouponRepository::count_issued`] | `lambdaQuery()...count()`（全站已发） |
//! | [`ProxyShoppingUserCouponRepository::count_user_paid_orders`] | `orderMapper.selectCount(... gt(payStatus, 0))` |
//! | [`ProxyShoppingUserCouponRepository::find_user_flags`] | `userMapper.selectById(userId)`（只取 3 列） |
//! | [`ProxyShoppingUserCouponRepository::find_coupon_promotion`] | `promotionMapper.selectOne(type = 2)` |
//! | [`ProxyShoppingUserCouponRepository::insert_user_coupon`] | `this.save(userCoupon)` |
//!
//! 列名严格对齐权威 schema（`java/database/pure.sql`，
//! `cd rust && python tools/schema_parser.py --table <表名>` 可复核）：
//! - `coupon`（**29 列**）：`coupon_id` / `coupon_name` / `coupon_money` /
//!   `coupon_discount` / `coupon_desc` / `coupon_type` / `send_range` /
//!   `send_range_data` / `min_order_amount` / `send_start_date` / `send_end_date` /
//!   `send_type` / `use_day` / `use_start_date` / `use_end_date` / `is_show` /
//!   `is_global` / `is_new_user` / `enabled_click_get` / `limit_user_rank` /
//!   `shop_id` / `is_delete` / `limit_num` / `delay_day` / `send_num` /
//!   `max_order_amount` / `coupon_unit` / `reduce_type` / `add_time`
//! - `user_coupon`（**9 列**，主键列名是 `id`）：`id` / `coupon_id` /
//!   `coupon_sn` / `user_id` / `used_time` / `order_id` / `start_date` /
//!   `end_date` / `source_type`
//! - `promotion`（**15 列**）：只读 `member_scope` / `member_range` /
//!   `member_marks` 三列（`relation_id` / `type` 仅用于 WHERE）
//! - `user`（**32 列**）：只读 `rank_id` / `mark` 两列
//!   （⚠️ Java 原链路用的是 `userMapper.selectById` 整行，本域只取判定所需的两列；
//!   这条链路**不判** `user_type` / `status`）
//! - `order`（**反引号包裹**，`order` 是 MySQL 保留字）：只读计数
//!
//! # 约定
//!
//! - 一律使用**运行时** `sqlx::query_as` / `sqlx::query`，**不使用** `query!` 宏
//!   （本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 静态 SQL 一律**单行字面量**，便于 `tools/check_sql_columns.py` 静态校验列名。
//! - `.bind()` 结果先绑到局部变量再 `execute` / `fetch_*`，避免临时值被 drop
//!   （本项目高发错误 E0716）。
//! - 仓储层只忠实执行 SQL，**不做业务校验**；校验链在
//!   `crates/services/src/proxy_shopping_user_coupon.rs` 中复刻。
//!
//! # 为什么重新建一个仓储而不是复用 `repo::coupon`
//!
//! `repo::coupon::CouponRepository` 已提供 `find_by_id` / `count_user_claimed` /
//! `count_issued` / `insert_user_coupon`，但**缺少**本域必需的
//! `promotion` 人群配置查询（`member_scope` / `member_range` / `member_marks`）
//! 与 `user` 的 `rank_id` / `mark` 读取。
//! `repo::coupon` 属其他实现者的独占文件，本切片**不得修改**，
//! 故在 `repo::nextrade_core::models::trade::proxy_shopping_user_coupon::ProxyShoppingUserCouponRepository` 中独立实现本域所需的全部 SQL
//! （含前四者，保持本域仓储自洽、可独立审阅）。
//!
//! # 并发（原系统的已知缺陷，本切片不擅自修复）
//!
//! 原 Java `claim` 从「计数校验」到 `this.save(userCoupon)` 的整条链路
//! **既无 `@Transactional` 也无行锁/唯一索引**，并发下可超出 `limit_num` /
//! `send_num` 超发（先查后写，非原子）。
//! `user_coupon` 表上也没有 `(coupon_id, user_id)` 唯一键，
//! 因此**无法**用「插入 + 唯一键冲突」做原子防超卖式兜底。
//! 本实现与 Java 保持**同一隔离级别与同一执行序**（普通计数 + 普通 INSERT），
//! 不做「插后复核 + 补偿删除」这类会改变可观察行为的加固 ——
//! 超发风险与原系统一致，已在服务层模块头第 7 条与 `uncertain_points` 中标注。

use sqlx::PgPool;
use nextrade_core::entities::Coupon;
use nextrade_core::{Error, Result};

/// `coupon` 表全部 29 列（顺序与 `Coupon` 实体字段一致）。
const COUPON_COLUMNS: &str = "coupon_id, coupon_name, coupon_money, coupon_discount, coupon_desc, coupon_type, send_range, send_range_data, min_order_amount, send_start_date, send_end_date, send_type, use_day, use_start_date, use_end_date, is_show, is_global, is_new_user, enabled_click_get, limit_user_rank, shop_id, is_delete, limit_num, delay_day, send_num, max_order_amount, coupon_unit, reduce_type, add_time";

/// `promotion` 表中本域只读的 3 列（人群限制配置）。
///
/// ⚠️ 表归属区分：这 3 列**只在 `promotion` 表存在**；
/// 同义的「活动名称」列在 `promotion` 是 `promotion_name`，
/// 不要照抄其它活动表的列名。
const PROMOTION_MEMBER_COLUMNS: &str = "member_scope, member_range, member_marks";

/// 一张券的人群限制配置（`promotion` 表，`type = 2` 优惠券活动）。
///
/// 三个字段名与 SQL 选出的真实列名逐字一致，无需 `#[sqlx(rename_all)]`。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct PromotionMemberRule {
    /// 二期人群圈选 JSON（`member_scope`），非空时优先于下面两项
    pub member_scope: Option<String>,
    /// 适用会员范围：0 全部会员，1 指定标记会员
    pub member_range: i32,
    /// 会员标记列表 JSON，如 `[1,2]`
    pub member_marks: Option<String>,
}

/// 用户侧判定所需的 2 个字段（`user` 表）。
///
/// Java `claim` 只用到 `user.getRankId()`（L346）与 `user.getMark()`（L368），
/// 故本域**不整行读取** `user`（32 列），只取这 2 列。
/// `user_type` 在这条链路里**没有**被 Java 判定（前台 `matches` 才判会员身份），
/// 因此不取 —— 不臆造 Java 不存在的「必须是在册会员」门。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct UserCouponFlags {
    /// 会员等级 ID
    pub rank_id: i32,
    /// 标记 12345
    pub mark: i32,
    /// 用户类型（0=会员；`matchesScope` 只对会员生效）
    pub user_type: i32,
}

/// 代客下单优惠券域仓储（本切片独占）。
#[derive(Clone)]
pub struct ProxyShoppingUserCouponRepository {
    pool: PgPool,
}

impl ProxyShoppingUserCouponRepository {
    /// 构造：持有连接池（与 buyer_show/025/026 同一约定）。
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按主键查询券定义（Java `couponMapper.selectById`）。
    ///
    /// ⚠️ 与 Java 一致：**不过滤 `is_delete`**（Java `claim` 里没有删除判定，
    /// 只有 `getManualIssueCoupon` 那条链路才判）。软删除的券仍会被领到，
    /// 这是原系统既有行为，本切片不擅自加门（见服务层模块头）。
    pub async fn find_coupon(&self, coupon_id: i32) -> Result<Option<Coupon>> {
        let sql = format!("SELECT {COUPON_COLUMNS} FROM coupon WHERE coupon_id = $1 LIMIT 1");
        let q = sqlx::query_as::<_, Coupon>(&sql).bind(coupon_id);
        q.fetch_optional(&self.pool).await.map_err(map_err)
    }

    /// 代客下单可领取（可见）优惠券列表。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/proxyShopping/cart.ts#getProxyShoppingCouponList`
    /// （`POST /proxyShopping/coupon/visible-list`，注释「对齐收银台
    /// pos/user/coupon/visible-list」；对应 Java 控制器未随本仓提供）。
    /// 过滤口径对齐 `repo::marketing::coupon::paginate_claimable`：
    /// 未删除 + 展示 + 可点击领取 + 有效期末过（`send_type = 0` 或
    /// `use_end_date >= now`）。
    pub async fn find_visible_coupons(&self, now: i64) -> Result<Vec<Coupon>> {
        let sql = format!(
            "SELECT {COUPON_COLUMNS} FROM coupon
             WHERE is_delete = 0 AND is_show = 1 AND enabled_click_get = 1
               AND (send_type = 0 OR use_end_date >= $1)
             ORDER BY add_time DESC"
        );
        sqlx::query_as::<_, Coupon>(&sql)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 统计某用户对某券的已领数量（Java `count(user_id, coupon_id)`）。
    pub async fn count_user_claimed(&self, user_id: i32, coupon_id: i32) -> Result<i64> {
        let q = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM user_coupon WHERE coupon_id = $1 AND user_id = $2",
        )
        .bind(coupon_id)
        .bind(user_id);
        let (n,) = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(n)
    }

    /// 统计某券的全站已发数量（Java `count(coupon_id)`）。
    pub async fn count_issued(&self, coupon_id: i32) -> Result<i64> {
        let q = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM user_coupon WHERE coupon_id = $1",
        )
        .bind(coupon_id);
        let (n,) = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(n)
    }

    /// 统计用户**已支付**订单数（Java `gt(Order::getPayStatus, 0)`）。
    ///
    /// ⚠️ `order` 是 MySQL 保留字，必须反引号包裹。
    /// `pay_status > 0` 含「支付中 / 已支付 / 退款中 / 已退款」（与原 Java 一致）。
    pub async fn count_user_paid_orders(&self, user_id: i32) -> Result<i64> {
        let q = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM \"order\" WHERE user_id = $1 AND pay_status > 0",
        )
        .bind(user_id);
        let (n,) = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(n)
    }

    /// 读取用户侧判定所需的 `rank_id` / `mark`
    /// （Java `userMapper.selectById(userId)`，本域只用到这 2 列）。
    ///
    /// `None` 表示用户不存在 —— ⚠️ Java 此时会在
    /// `user.getRankId()` / `user.getMark()` 上抛 NPE（详见服务层模块头）。
    pub async fn find_user_flags(&self, user_id: i32) -> Result<Option<UserCouponFlags>> {
        let sql = "SELECT rank_id, mark, user_type FROM \"user\" WHERE user_id = $1 LIMIT 1";
        let q = sqlx::query_as::<_, UserCouponFlags>(sql).bind(user_id);
        q.fetch_optional(&self.pool).await.map_err(map_err)
    }

    /// 查询券关联的活动人群配置。
    ///
    /// 对应 Java：
    /// ```text
    /// promotionMapper.selectOne(new LambdaQueryWrapper<Promotion>()
    ///     .eq(Promotion::getRelationId, couponId)
    ///     .eq(Promotion::getType, 2)   // 优惠券类型
    ///     .last("limit 1"))
    /// ```
    ///
    /// ⚠️ 与 Java 一致：**不过滤 `is_delete` / `is_available`**，
    /// 且 `LIMIT 1` 在按主键（`promotion_id`）之外无 `ORDER BY` ——
    /// 若同一 `relation_id` 有多条 `type = 2` 活动，Java 取的哪一条是
    /// **不确定**的（取决于存储引擎返回顺序）。本实现照抄该不确定性。
    pub async fn find_coupon_promotion(
        &self,
        coupon_id: i32,
    ) -> Result<Option<PromotionMemberRule>> {
        let sql = format!(
            "SELECT {PROMOTION_MEMBER_COLUMNS} FROM promotion WHERE relation_id = $1 AND type = $2 LIMIT 1"
        );
        let q = sqlx::query_as::<_, PromotionMemberRule>(&sql)
            .bind(coupon_id)
            .bind(PROMOTION_TYPE_COUPON);
        q.fetch_optional(&self.pool).await.map_err(map_err)
    }

    /// 发放一张用户券，返回自增主键（Java `this.save(userCoupon)`）。
    ///
    /// ⚠️ 与原 Java 的**已知差异**（详见服务层模块头）：
    /// Java 的 `UserCoupon.builder()` **没有** `usedTime` / `orderId` / `couponSn`，
    /// 三者保持 `null`，MyBatis-Plus 默认 `FieldStrategy.NOT_NULL` 会把它们
    /// **从 INSERT 语句里剔除**，落库取列默认值（`used_time` = 0 / `order_id` = 0 /
    /// `coupon_sn` = `''`）。
    /// 本实现显式写 `0 / 0 / ''`，与 Java 的**最终落库值**逐字节一致，
    /// 且不依赖 MySQL 对 NOT NULL 列的隐式默认（`coupon_sn` 是
    /// `NOT NULL DEFAULT ''`，显式传 NULL 会失败）。
    pub async fn insert_user_coupon(
        &self,
        coupon_id: i32,
        user_id: i32,
        start_date: i32,
        end_date: i32,
        source_type: i32,
    ) -> Result<i32> {
        let q = sqlx::query_scalar(
            "INSERT INTO user_coupon (coupon_id, coupon_sn, user_id, used_time, order_id, start_date, end_date, source_type) VALUES ($1, '', $2, 0, 0, $3, $4, $5) RETURNING id",
        )
        .bind(coupon_id)
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .bind(source_type);
        let r: i32 = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(r)
    }
}

/// `promotion.type = 2`：优惠券活动（Java 源码里的字面量 `2`）。
pub const PROMOTION_TYPE_COUPON: i32 = 2;

/// `user_coupon.source_type = 0`：普通领取（含代客领取）。
///
/// ⚠️ Java `claim` 构建 `UserCoupon` 时**未显式设置** `sourceType`，
/// MyBatis-Plus 剔除 null 后落库取列默认值 `'0'`（普通领取）。
/// 故代客领取与前台自助领取在库里**不可区分**（见服务层模块头）。
pub const SOURCE_CLAIM: i32 = 0;

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("优惠券不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupon_columns_cover_all_twenty_nine_from_schema() {
        // 列数必须与 `schema_parser.py --table coupon` 的 29 列一致
        // （列名本身由 `check_sql_columns.py` 对 pure.sql 校验）。
        assert_eq!(COUPON_COLUMNS.split(',').count(), 29);
        for col in [
            "coupon_id",
            "is_new_user",
            "limit_user_rank",
            "limit_num",
            "send_type",
            "delay_day",
            "use_day",
            "use_start_date",
            "use_end_date",
            "send_num",
        ] {
            assert!(COUPON_COLUMNS.contains(col), "COUPON_COLUMNS 缺少列 {col}");
        }
    }

    #[test]
    fn promotion_member_columns_exist_on_promotion_table_only() {
        // `member_scope` / `member_range` / `member_marks` 只属于 `promotion` 表；
        // 它们在 `coupon` 表并不存在，故不得出现在 COUPON_COLUMNS 中。
        assert_eq!(PROMOTION_MEMBER_COLUMNS.split(',').count(), 3);
        for col in ["member_scope", "member_range", "member_marks"] {
            assert!(PROMOTION_MEMBER_COLUMNS.contains(col));
            assert!(
                !COUPON_COLUMNS.contains(col),
                "coupon 表没有列 {col}，不得混用"
            );
        }
    }

    #[test]
    fn promotion_type_and_source_type_match_java_literals() {
        // Java：`.eq(Promotion::getType, 2)` 与 `user_coupon.source_type` 默认 0。
        assert_eq!(PROMOTION_TYPE_COUPON, 2);
        assert_eq!(SOURCE_CLAIM, 0);
    }
}