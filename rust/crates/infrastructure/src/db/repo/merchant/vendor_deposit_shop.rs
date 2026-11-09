//! 供应商端保证金域（merchant::vendor_deposit_shop）仓储：`shop_deposit` / `shop_deposit_log` /
//! `shop_deposit_refund` / `shop_deposit_recharge` / `shop_deposit_rule` /
//! `shop_deposit_rule_category` / `vendor_product` / `vendor` / `category` / `config`。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.deposit.ShopDepositMapper`（`selectForUpdateByOwner`
//!   之外的只读查询 + `selectVendorOnSaleCategoryIds`，见
//!   `nextrade-service/src/main/resources/mapper/deposit/ShopDepositMapper.xml`）
//! - `com.nextrade.mapper.deposit.ShopDepositRuleMapper#matchMaxByCategories`
//!   （`ShopDepositRuleMapper.xml`）
//! - `com.nextrade.mapper.deposit.ShopDepositLogMapper`（`BaseMapper` 通用查询）
//! - `com.nextrade.mapper.deposit.ShopDepositRefundMapper` /
//!   `ShopDepositRechargeMapper`（`selectBatchIds`）
//! - `com.nextrade.service.setting.ConfigService#getConfigByCode`
//! - `com.nextrade.mapper.vendor.VendorMapper#selectOnboardedById`
//! - `com.nextrade.mapper.product.CategoryMapper#selectById`
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table <表名>`）。
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query_scalar` /
//! `QueryBuilder`（**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! 金额：`paid` / `change_amount` / `amount` / `deposit_amount` 都是 MySQL
//! `decimal`，绑定类型为 `rust_decimal::Decimal`（sqlx 的 `rust_decimal`
//! feature 已由 workspace 启用），**全程不经过 f64**。

use rust_decimal::Decimal;
use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::deposit::ShopDeposit;
use nextrade_core::models::merchant::vendor_deposit_shop::{DepositShopLogQuery, LOG_TYPE_DEDUCT, OWNER_TYPE_SUPPLIER};
use nextrade_core::{Error, Result};

/// `shop_deposit` 的固定 SELECT 列清单（避免 `SELECT *` 与实体字段顺序耦合；
/// 与 `entities::deposit::ShopDeposit` 的 13 个字段一一对应，顺序也一致）。
const SHOP_DEPOSIT_SELECT: &str = "SELECT shop_deposit_id, owner_type, owner_id, paid, required_manual, required_snapshot, collect_due_time, supplement_due_time, collect_activated, penalty_status, status, add_time, update_time FROM shop_deposit";

/// `shop_deposit_log` 的固定 SELECT 列清单
/// （与 `entities::deposit::ShopDepositLog` 的 12 个字段一一对应）。
const SHOP_DEPOSIT_LOG_SELECT: &str = "SELECT shop_deposit_log_id, owner_type, owner_id, type, change_amount, before_paid, after_paid, related_id, order_id, admin_id, remark, add_time FROM shop_deposit_log";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct VendorDepositShopRepository {
    pool: PgPool,
}

impl VendorDepositShopRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 账户 —————————————————————

    /// 只读读取 owner 维度保证金账户（不存在返回 `None`，**不创建行**）
    /// （对应 Java `VendorDepositShopServiceImpl#findAccount` →
    /// `shopDepositMapper.selectOne(ownerType + ownerId)`）。
    ///
    /// ⚠️ Java 用的是 `selectOne`：若同一 owner 存在多行会抛
    /// `TooManyResultsException`。表上 `(owner_type, owner_id)` 应有唯一约束，
    /// 本实现加 `LIMIT 1` 以保证健壮（见 `uncertain_points`）。
    pub async fn find_account(
        &self,
        owner_type: i32,
        owner_id: i32,
    ) -> Result<Option<ShopDeposit>> {
        sqlx::query_as::<_, ShopDeposit>(&format!(
            "{SHOP_DEPOSIT_SELECT} WHERE owner_type = $1 AND owner_id = $2 LIMIT 1"
        ))
        .bind(owner_type)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 配置 —————————————————————

    /// 按 `biz_code` 读单个配置值
    /// （对应 Java `ConfigService#getConfigByCode` + `is_del = 0`）。
    ///
    /// Java 在配置不存在时抛 `GlobalException(CONFIG_NOT_EXIST)`，
    /// 而调用方 `VendorDepositShopServiceImpl#readConfig` 用 `try/catch`
    /// 吞掉异常并返回 `null` ⇒ **净语义就是 `Option<String>`**，
    /// 因此本方法直接返回 `Ok(None)`，不抛错。
    pub async fn find_config(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— 应缴规则匹配 —————————————————————

    /// 供应商**在售**商品类目 id 去重清单
    /// （对应 Java `ShopDepositMapper#selectVendorOnSaleCategoryIds`）。
    ///
    /// SQL 逐条照抄 XML：
    /// `vendor_id` + `product_state = 1`（在售）+ `audit_state = 1`（审核通过，
    /// `VendorProductAuditStateEnum.PASS_AUDIT`）+ `is_del = 0` +
    /// `product_category_id > 0`，并要求该供应商 `vendor.source_type = 1`
    /// （`VendorSourceTypeEnum.ONBOARDED` 入驻供应商）。
    ///
    /// ⚠️ `vendor_product.id` 是 `bigint`，但 `product_category_id` 是 `int`，
    /// 因此返回值是 `Vec<i32>`。
    pub async fn vendor_on_sale_category_ids(&self, vendor_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT product_category_id FROM vendor_product WHERE vendor_id = $1 AND product_state = 1 AND audit_state = 1 AND is_del = 0 AND product_category_id > 0 AND EXISTS (SELECT 1 FROM vendor v WHERE v.vendor_id = $2 AND v.source_type = 1)",
        )
        .bind(vendor_id)
        .bind(vendor_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 供应商主体类型 `vendor.type`（1 个人 2 企业）
    /// （对应 Java `VendorMapper#selectOnboardedById(vendorId).getType()`）。
    ///
    /// 只取入驻供应商（`source_type = 1`）且只 SELECT `type` 一列；
    /// 查不到行返回 `Ok(None)`（Java `vendor == null` → `null` → 应缴 0）。
    pub async fn find_vendor_type(&self, vendor_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT type FROM vendor WHERE vendor_id = $1 AND source_type = 1 LIMIT 1",
        )
        .bind(vendor_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 类目父 id（对应 Java `CategoryMapper#selectById(categoryId).getParentId()`）。
    ///
    /// `category.parent_id` 是 `int unsigned NOT NULL`（一级类目为 0），
    /// 因此元组里用裸 `i32`；查不到行才是 `None`。
    pub async fn find_category_parent(&self, category_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT parent_id FROM category WHERE category_id = $1 LIMIT 1")
                .bind(category_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 按类目集合取每个类目的**最高**应缴金额
    /// （对应 Java `ShopDepositRuleMapper#matchMaxByCategories`）。
    ///
    /// SQL 逐条照抄 XML：
    /// `r.status = 1`（启用）+ `r.is_delete = 0` + `r.start_time <= now`
    /// + `(r.end_time = 0 OR r.end_time >= now)`（当前生效期）
    /// + `FIND_IN_SET(merchantType, r.merchant_types)`
    /// + `FIND_IN_SET(shopType, r.shop_types)` + 类目命中 `IN (...)`，
    /// 最后 `GROUP BY rc.category_id` 取 `MAX(r.deposit_amount)`。
    ///
    /// `merchant_types` / `shop_types` 是逗号分隔的**字符串**列（`varchar(50)`），
    /// Java 绑定的是 `Integer`（MyBatis 会 `setInt`），MySQL 隐式转字符串比较，
    /// 因此这里同样绑定 `i32` 以保持完全一致。
    ///
    /// ⚠️ 类目集合为空时返回空 `Vec`（调用方已在服务层短路，此处再兜一层，
    /// 避免生成 `IN ()` 这种 MySQL 语法错误）。
    pub async fn match_max_by_categories(
        &self,
        merchant_type: i32,
        shop_type: i32,
        category_ids: &[i32],
        now: i64,
    ) -> Result<Vec<(i32, Decimal)>> {
        if category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT rc.category_id, MAX(r.deposit_amount) FROM shop_deposit_rule r INNER JOIN shop_deposit_rule_category rc ON rc.rule_id = r.shop_deposit_rule_id WHERE r.status = 1 AND r.is_delete = 0 AND r.start_time <= ",
        );
        qb.push_bind(now);
        qb.push(" AND (r.end_time = 0 OR r.end_time >= ");
        qb.push_bind(now);
        qb.push(") AND ");
        qb.push_bind(merchant_type);
        qb.push("::text = ANY(string_to_array(r.merchant_types, ',')) AND ");
        qb.push_bind(shop_type);
        qb.push("::text = ANY(string_to_array(r.shop_types, ',')) AND rc.category_id IN (");
        let mut separated = qb.separated(", ");
        for id in category_ids {
            separated.push_bind(*id);
        }
        drop(separated);
        qb.push(") GROUP BY rc.category_id");
        let rows: Vec<(i32, Decimal)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    // ————————————————————— 待审核提现合计 —————————————————————

    /// owner 维度**待审核**退还/提现申请金额合计
    /// （对应 Java `DepositRefundService#pendingRefundSum`：
    /// `owner_type` + `owner_id` + `status = 0`，`amount` 求和）。
    ///
    /// Java 是 `list(...).stream().map(getAmount).filter(非空).reduce(ZERO, add)`，
    /// 即**在内存里求和**；本实现在 SQL 侧 `SUM()` 并 `COALESCE(..., 0)`，
    /// 结果等价且不把整表行拉进内存（金额仍在 `Decimal` 上，无浮点）。
    ///
    /// ⚠️ Java 在 `ownerId == null || ownerId <= 0` 时直接返回 `BigDecimal.ZERO`，
    /// 本实现由服务层保证 `owner_id > 0`（控制器已校验），此处不再重复判断。
    pub async fn pending_refund_sum(&self, owner_type: i32, owner_id: i32) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM shop_deposit_refund WHERE owner_type = $1 AND owner_id = $2 AND status = 0",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(Decimal::ZERO))
    }

    // ————————————————————— 流水 / 冻结明细分页 —————————————————————

    /// 保证金流水分页（对应 Java `VendorDepositShopServiceImpl#logList`）。
    ///
    /// 过滤条件与 Java 逐条对齐：
    /// - `owner_type = 2`（供应商）、`owner_id = vendorId`
    /// - `type`：`null` 不加；提现类（2/3）→ `IN (2, 3)`；其它 → 等值
    /// - `startTime` / `endTime` 非 `null` → `add_time >= / <=`
    /// - `tradeNo` 为纯数字 → `(order_id = ? OR related_id = ?)`
    /// - 排序固定 `ORDER BY shop_deposit_log_id DESC`（Java 未用 `BasePage`
    ///   的 `sortField`）
    ///
    /// `deduct_only = true` 时额外加 `type = 4`（冻结明细，
    /// 对应 Java `frozenDetailList`）；此时**忽略** `q.type`（Java 亦然）。
    pub async fn paginate_logs(
        &self,
        vendor_id: i32,
        q: &DepositShopLogQuery,
        deduct_only: bool,
    ) -> Result<(u64, Vec<nextrade_core::entities::deposit::ShopDepositLog>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();

        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM shop_deposit_log WHERE owner_type = ",
        );
        count_qb.push_bind(OWNER_TYPE_SUPPLIER);
        Self::push_log_filters(&mut count_qb, vendor_id, q, deduct_only);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(format!(
            "{SHOP_DEPOSIT_LOG_SELECT} WHERE owner_type = "
        ));
        row_qb.push_bind(OWNER_TYPE_SUPPLIER);
        Self::push_log_filters(&mut row_qb, vendor_id, q, deduct_only);
        // ORDER BY 列名是硬编码字面量，LIMIT/OFFSET 是 u64 字面量，均无注入面。
        row_qb.push(format!(
            " ORDER BY shop_deposit_log_id DESC LIMIT {limit} OFFSET {offset}"
        ));

        let rows: Vec<nextrade_core::entities::deposit::ShopDepositLog> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 追加流水查询的 WHERE 片段（`count` 与取行两条 SQL 共用，保证条件一致）。
    ///
    /// ⚠️ `tradeNo` 分支必须写成 `AND (order_id = ? OR related_id = ?)`
    /// —— 外层括号不能省，否则会与后面的时间条件发生优先级错误。
    fn push_log_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        vendor_id: i32,
        q: &DepositShopLogQuery,
        deduct_only: bool,
    ) {
        qb.push(" AND owner_id = ");
        qb.push_bind(vendor_id);
        if deduct_only {
            qb.push(" AND type = ");
            qb.push_bind(LOG_TYPE_DEDUCT);
        } else {
            let types = q.type_filter();
            match types.as_slice() {
                [] => {}
                [only] => {
                    qb.push(" AND type = ");
                    qb.push_bind(*only);
                }
                many => {
                    qb.push(" AND type IN (");
                    let mut separated = qb.separated(", ");
                    for t in many {
                        separated.push_bind(*t);
                    }
                    drop(separated);
                    qb.push(")");
                }
            }
        }
        if let Some(ts) = q.start_time {
            qb.push(" AND add_time >= ");
            qb.push_bind(ts);
        }
        if let Some(ts) = q.end_time {
            qb.push(" AND add_time <= ");
            qb.push_bind(ts);
        }
        if let Some(trade_no) = q.trade_no_filter() {
            qb.push(" AND (order_id = ");
            qb.push_bind(trade_no);
            qb.push(" OR related_id = ");
            qb.push_bind(trade_no);
            qb.push(")");
        }
    }

    // ————————————————————— 流水来源/去向账户富化 —————————————————————
    //
    // 对应 Java `DepositShopLogEnricher#fillSourceDestAccounts` 的
    // `selectBatchIds` 批量回查。返回 `HashMap` 便于按 `relatedId` 取用。

    /// 批量回查充值单（`shop_deposit_recharge`）的
    /// `payee_account_type` + `payee_snapshot`
    /// （Java `ShopDepositRechargeMapper#selectBatchIds` 的子集）。
    pub async fn find_recharges_by_ids(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(i32, i32, Option<String>)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT shop_deposit_recharge_id, payee_account_type, payee_snapshot FROM shop_deposit_recharge WHERE shop_deposit_recharge_id IN (",
        );
        let mut separated = qb.separated(", ");
        for id in ids {
            separated.push_bind(*id);
        }
        drop(separated);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量回查退还单（`shop_deposit_refund`）的 `payee_info`
    /// （Java `ShopDepositRefundMapper#selectBatchIds` 的子集）。
    pub async fn find_refunds_by_ids(&self, ids: &[i32]) -> Result<Vec<(i32, Option<String>)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT shop_deposit_refund_id, payee_info FROM shop_deposit_refund WHERE shop_deposit_refund_id IN (",
        );
        let mut separated = qb.separated(", ");
        for id in ids {
            separated.push_bind(*id);
        }
        drop(separated);
        qb.push(")");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("保证金数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SELECT 列清单必须与 schema 一致（防止漏列导致 `FromRow` 运行期报错）。
    #[test]
    fn select_clauses_list_all_schema_columns() {
        for col in [
            "shop_deposit_id",
            "owner_type",
            "owner_id",
            "paid",
            "required_manual",
            "required_snapshot",
            "collect_due_time",
            "supplement_due_time",
            "collect_activated",
            "penalty_status",
            "status",
            "add_time",
            "update_time",
        ] {
            assert!(
                SHOP_DEPOSIT_SELECT.contains(col),
                "shop_deposit SELECT 缺少列 {col}"
            );
        }
        for col in [
            "shop_deposit_log_id",
            "owner_type",
            "owner_id",
            "type",
            "change_amount",
            "before_paid",
            "after_paid",
            "related_id",
            "order_id",
            "admin_id",
            "remark",
            "add_time",
        ] {
            assert!(
                SHOP_DEPOSIT_LOG_SELECT.contains(col),
                "shop_deposit_log SELECT 缺少列 {col}"
            );
        }
        // 不得使用 SELECT *
        assert!(!SHOP_DEPOSIT_SELECT.contains('*'));
        assert!(!SHOP_DEPOSIT_LOG_SELECT.contains('*'));
    }

    /// 常量必须与 Java 枚举同码。
    #[test]
    fn constants_match_java_enums() {
        assert_eq!(OWNER_TYPE_SUPPLIER, 2);
        assert_eq!(LOG_TYPE_DEDUCT, 4);
    }
}