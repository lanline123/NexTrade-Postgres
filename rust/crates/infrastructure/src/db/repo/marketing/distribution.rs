//! 营销活动下发仓储（marketing::distribution）。
//! 表 `marketing_distribution` / `marketing_distribution_item`（迁移 020）。
//!
//! 副本生成走 INSERT... SELECT 整行复制（字段保真，不经 DTO 派生）：
//! - 优惠券：复制 `coupon` 行（shop_id 改目标店铺、is_global 归 0、
//! is_delete 归 0、add_time 取下发时刻）+ 复制其 `promotion` 同步行
//! （type = 2，relation_id 改新券 id）；来源无同步行时由服务层兜底构造。
//! - 满减：复制 `product_promotion` 行（shop_id 改目标店铺）+ 复制其
//! `promotion` 同步行（type = 3，relation_id 改新活动 id）。
//! 撤销的停用/软删也在本仓储（与 coupon_manage 软删口径一致）。

use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use nextrade_core::entities::{
Coupon, MarketingDistribution, MarketingDistributionItem, ProductPromotion, Promotion,
};
use nextrade_core::models::marketing::distribution::{
limit_offset, DistributionListQuery, COUPON_SYNC_TYPE, FULL_REDUCTION_PROMOTION_TYPE,
FULL_REDUCTION_SYNC_TYPE, ITEM_STATUS_CREATED,
};
use nextrade_core::{Error, Result};

const BATCH_SELECT: &str = "SELECT distribution_id, source_kind, source_id, source_name, target_count, success_count, skipped_count, failed_count, revoked_count, status, remark, admin_id, add_time, update_time FROM marketing_distribution";
const ITEM_SELECT: &str = "SELECT item_id, distribution_id, source_kind, source_id, target_shop_id, target_shop_name, copy_id, status, message, add_time, update_time FROM marketing_distribution_item";

/// 营销活动下发仓储。
#[derive(Clone)]
pub struct DistributionRepository {
pool: PgPool,
}

impl DistributionRepository {
#[must_use]
pub fn new(pool: PgPool) -> Self {
Self { pool}
}

/// 开启事务（一次下发的原子边界由服务层持有）。
pub async fn begin(&self) -> Result<Transaction<'static, Postgres>> {
self.pool.begin().await.map_err(map_err)
}

// ————————————————— 店铺与来源 —————————————————

/// 全部店铺（目标选择器用；shop_title 为空时回退空串由前端兜底）。
pub async fn list_shops(&self) -> Result<Vec<(i32, String)>> {
let rows: Vec<(i32, Option<String>)> = sqlx::query_as(
"SELECT shop_id, shop_title FROM shop WHERE shop_id > 0 ORDER BY shop_id",
)
.fetch_all(&self.pool)
.await
.map_err(map_err)?;
Ok(rows
.into_iter()
.map(|(id, name)| (id, name.unwrap_or_default()))
.collect())
}

/// 单个店铺名称（不存在 → None）。
pub async fn find_shop_name(&self, shop_id: i32) -> Result<Option<String>> {
let row: Option<(Option<String>,)> =
sqlx::query_as("SELECT shop_title FROM shop WHERE shop_id = $1")
.bind(shop_id)
.fetch_optional(&self.pool)
.await
.map_err(map_err)?;
Ok(row.map(|(name,)| name.unwrap_or_default()))
}

/// 平台级优惠券（shop_id 归一为 0、未删除），按下发选择器口径排序。
pub async fn list_source_coupons(&self) -> Result<Vec<Coupon>> {
sqlx::query_as::<_, Coupon>(
"SELECT * FROM coupon WHERE COALESCE(shop_id, 0) = 0 \
AND COALESCE(is_delete, 0) = 0 ORDER BY coupon_id DESC",
)
.fetch_all(&self.pool)
.await
.map_err(map_err)
}

/// 平台级满减活动（product_promotion，promotion_type = 1）。
pub async fn list_source_full_reductions(&self) -> Result<Vec<ProductPromotion>> {
sqlx::query_as::<_, ProductPromotion>(
"SELECT * FROM product_promotion WHERE shop_id = 0 AND promotion_type = $1 \
ORDER BY promotion_id DESC",
)
.bind(FULL_REDUCTION_PROMOTION_TYPE)
.fetch_all(&self.pool)
.await
.map_err(map_err)
}

pub async fn find_coupon(&self, coupon_id: i32) -> Result<Option<Coupon>> {
sqlx::query_as::<_, Coupon>("SELECT * FROM coupon WHERE coupon_id = $1")
.bind(coupon_id)
.fetch_optional(&self.pool)
.await
.map_err(map_err)
}

pub async fn find_full_reduction(
&self,
promotion_id: i32,
) -> Result<Option<ProductPromotion>> {
sqlx::query_as::<_, ProductPromotion>(
"SELECT * FROM product_promotion WHERE promotion_id = $1",
)
.bind(promotion_id)
.fetch_optional(&self.pool)
.await
.map_err(map_err)
}

// ————————————————— 批次与逐店记录 —————————————————

pub async fn insert_batch_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
batch: &MarketingDistribution,
) -> Result<i64> {
let id: i64 = sqlx::query_scalar(
"INSERT INTO marketing_distribution (source_kind, source_id, source_name, target_count, success_count, skipped_count, failed_count, revoked_count, status, remark, admin_id, add_time, update_time) \
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING distribution_id",
)
.bind(batch.source_kind)
.bind(batch.source_id)
.bind(&batch.source_name)
.bind(batch.target_count)
.bind(batch.success_count)
.bind(batch.skipped_count)
.bind(batch.failed_count)
.bind(batch.revoked_count)
.bind(batch.status)
.bind(&batch.remark)
.bind(batch.admin_id)
.bind(batch.add_time)
.bind(batch.update_time)
.fetch_one(&mut **tx)
.await
.map_err(map_err)?;
Ok(id)
}

pub async fn insert_item_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
item: &MarketingDistributionItem,
) -> Result<i64> {
let id: i64 = sqlx::query_scalar(
"INSERT INTO marketing_distribution_item (distribution_id, source_kind, source_id, target_shop_id, target_shop_name, copy_id, status, message, add_time, update_time) \
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING item_id",
)
.bind(item.distribution_id)
.bind(item.source_kind)
.bind(item.source_id)
.bind(item.target_shop_id)
.bind(&item.target_shop_name)
.bind(item.copy_id)
.bind(item.status)
.bind(&item.message)
.bind(item.add_time)
.bind(item.update_time)
.fetch_one(&mut **tx)
.await
.map_err(map_err)?;
Ok(id)
}

/// 批次计数与状态回写（下发完成 / 撤销后共用）。
pub async fn update_batch_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
batch: &MarketingDistribution,
) -> Result<u64> {
let res = sqlx::query(
"UPDATE marketing_distribution SET success_count = $1, skipped_count = $2, failed_count = $3, revoked_count = $4, status = $5, update_time = $6 WHERE distribution_id = $7",
)
.bind(batch.success_count)
.bind(batch.skipped_count)
.bind(batch.failed_count)
.bind(batch.revoked_count)
.bind(batch.status)
.bind(batch.update_time)
.bind(batch.distribution_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
Ok(res.rows_affected())
}

/// 逐店记录回写（撤销置状态/原因共用）。
pub async fn update_item_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
item: &MarketingDistributionItem,
) -> Result<u64> {
let res = sqlx::query(
"UPDATE marketing_distribution_item SET copy_id = $1, status = $2, message = $3, update_time = $4 WHERE item_id = $5",
)
.bind(item.copy_id)
.bind(item.status)
.bind(&item.message)
.bind(item.update_time)
.bind(item.item_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
Ok(res.rows_affected())
}

pub async fn find_batch(
&self,
distribution_id: i64,
) -> Result<Option<MarketingDistribution>> {
sqlx::query_as::<_, MarketingDistribution>(&format!(
"{BATCH_SELECT} WHERE distribution_id = $1"
))
.bind(distribution_id)
.fetch_optional(&self.pool)
.await
.map_err(map_err)
}

/// 事务内锁定读取批次（撤销防并发重复操作）。
pub async fn find_batch_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
distribution_id: i64,
) -> Result<Option<MarketingDistribution>> {
sqlx::query_as::<_, MarketingDistribution>(&format!(
"{BATCH_SELECT} WHERE distribution_id = $1 FOR UPDATE"
))
.bind(distribution_id)
.fetch_optional(&mut **tx)
.await
.map_err(map_err)
}

pub async fn find_item(&self, item_id: i64) -> Result<Option<MarketingDistributionItem>> {
sqlx::query_as::<_, MarketingDistributionItem>(&format!(
"{ITEM_SELECT} WHERE item_id = $1"
))
.bind(item_id)
.fetch_optional(&self.pool)
.await
.map_err(map_err)
}

/// 事务内锁定读取逐店记录。
pub async fn find_item_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
item_id: i64,
) -> Result<Option<MarketingDistributionItem>> {
sqlx::query_as::<_, MarketingDistributionItem>(&format!(
"{ITEM_SELECT} WHERE item_id = $1 FOR UPDATE"
))
.bind(item_id)
.fetch_optional(&mut **tx)
.await
.map_err(map_err)
}

/// 某批次的全部逐店记录（详情与批次状态推导用）。
pub async fn list_items(
&self,
distribution_id: i64,
) -> Result<Vec<MarketingDistributionItem>> {
sqlx::query_as::<_, MarketingDistributionItem>(&format!(
"{ITEM_SELECT} WHERE distribution_id = $1 ORDER BY item_id"
))
.bind(distribution_id)
.fetch_all(&self.pool)
.await
.map_err(map_err)
}

/// 批次分页（后台下发记录）。
pub async fn paginate_batches(
&self,
q: &DistributionListQuery,
) -> Result<(i64, Vec<MarketingDistribution>)> {
let (limit, offset) = limit_offset(q.page, q.size);
let mut cq: QueryBuilder<'_, Postgres> =
QueryBuilder::new("SELECT COUNT(*) FROM marketing_distribution WHERE 1 = 1");
Self::push_filters(&mut cq, q);
let total: (i64,) = cq
.build_query_as()
.fetch_one(&self.pool)
.await
.map_err(map_err)?;
let mut rq: QueryBuilder<'_, Postgres> =
QueryBuilder::new(format!("{BATCH_SELECT} WHERE 1 = 1"));
Self::push_filters(&mut rq, q);
rq.push(format!(
" ORDER BY distribution_id DESC LIMIT {limit} OFFSET {offset}"
));
let rows: Vec<MarketingDistribution> = rq
.build_query_as()
.fetch_all(&self.pool)
.await
.map_err(map_err)?;
Ok((total.0, rows))
}

fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &DistributionListQuery) {
if let Some(kind) = q.source_kind {
qb.push(" AND source_kind = ");
qb.push_bind(kind);
}
if let Some(status) = q.status {
qb.push(" AND status = ");
qb.push_bind(status);
}
}

/// 同一来源对同一店铺最近一条「已生成」记录（防重复下发判定；
/// 副本本体是否仍有效由服务层结合本体表二次确认）。
pub async fn find_created_item(
&self,
source_kind: i32,
source_id: i32,
target_shop_id: i32,
) -> Result<Option<MarketingDistributionItem>> {
sqlx::query_as::<_, MarketingDistributionItem>(&format!(
"{ITEM_SELECT} WHERE source_kind = $1 AND source_id = $2 AND target_shop_id = $3 AND status = $4 ORDER BY item_id DESC LIMIT 1"
))
.bind(source_kind)
.bind(source_id)
.bind(target_shop_id)
.bind(ITEM_STATUS_CREATED)
.fetch_optional(&self.pool)
.await
.map_err(map_err)
}

// ————————————————— 副本生成（INSERT... SELECT 整行复制） —————————————————

/// 复制优惠券本体：shop_id 改目标店铺、全场通用标记归 0（副本为店铺券）、
/// is_delete 归 0、add_time 取下发时刻，其余字段逐字复制。
pub async fn copy_coupon_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
source_id: i32,
target_shop_id: i32,
now: i32,
) -> Result<Option<i32>> {
let id: Option<i32> = sqlx::query_scalar(
"INSERT INTO coupon (coupon_name, coupon_money, coupon_discount, coupon_desc, coupon_type, \
send_range, send_range_data, min_order_amount, send_start_date, send_end_date, send_type, \
use_day, use_start_date, use_end_date, is_show, is_global, is_new_user, enabled_click_get, \
limit_user_rank, shop_id, is_delete, limit_num, delay_day, send_num, max_order_amount, \
coupon_unit, reduce_type, add_time) \
SELECT coupon_name, coupon_money, coupon_discount, coupon_desc, coupon_type, \
send_range, send_range_data, min_order_amount, send_start_date, send_end_date, send_type, \
use_day, use_start_date, use_end_date, is_show, 0, is_new_user, enabled_click_get, \
limit_user_rank, $1, 0, limit_num, delay_day, send_num, max_order_amount, \
coupon_unit, reduce_type, $2 \
FROM coupon WHERE coupon_id = $3 \
RETURNING coupon_id",
)
.bind(target_shop_id)
.bind(now)
.bind(source_id)
.fetch_optional(&mut **tx)
.await
.map_err(map_err)?;
Ok(id)
}

/// 复制满减本体：shop_id 改目标店铺，其余字段逐字复制。
pub async fn copy_full_reduction_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
source_id: i32,
target_shop_id: i32,
) -> Result<Option<i32>> {
let id: Option<i32> = sqlx::query_scalar(
"INSERT INTO product_promotion (promotion_name, start_time, end_time, limit_user_rank, \
\"range\", range_data, min_order_amount, max_order_amount, promotion_type, \
promotion_type_data, is_available, sort_order, shop_id, rules_type, unit) \
SELECT promotion_name, start_time, end_time, limit_user_rank, \
\"range\", range_data, min_order_amount, max_order_amount, promotion_type, \
promotion_type_data, is_available, sort_order, $1, rules_type, unit \
FROM product_promotion WHERE promotion_id = $2 \
RETURNING promotion_id",
)
.bind(target_shop_id)
.bind(source_id)
.fetch_optional(&mut **tx)
.await
.map_err(map_err)?;
Ok(id)
}

/// 复制 promotion 同步行：type 不变、shop_id 改目标店铺、
/// relation_id 改新副本 id、is_delete 归 0。来源无同步行时返回 false。
pub async fn copy_promotion_row_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
sync_type: i32,
source_relation_id: i32,
new_relation_id: i32,
target_shop_id: i32,
) -> Result<bool> {
let id: Option<i32> = sqlx::query_scalar(
"INSERT INTO promotion (promotion_name, start_time, end_time, type, shop_id, relation_id, \
\"range\", range_data, sku_ids, is_available, is_delete, member_range, member_marks, member_scope) \
SELECT promotion_name, start_time, end_time, type, $1, $2, \
\"range\", range_data, sku_ids, is_available, 0, member_range, member_marks, member_scope \
FROM promotion WHERE type = $3 AND relation_id = $4 AND COALESCE(is_delete, 0) = 0 \
ORDER BY promotion_id LIMIT 1 \
RETURNING promotion_id",
)
.bind(target_shop_id)
.bind(new_relation_id)
.bind(sync_type)
.bind(source_relation_id)
.fetch_optional(&mut **tx)
.await
.map_err(map_err)?;
Ok(id.is_some())
}

/// 兜底插入 promotion 同步行（来源缺同步行时由服务层按本体字段构造）。
pub async fn insert_promotion_row_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
row: &Promotion,
) -> Result<i32> {
let id: i32 = sqlx::query_scalar(
"INSERT INTO promotion (promotion_name, start_time, end_time, type, shop_id, relation_id, \
\"range\", range_data, sku_ids, is_available, is_delete, member_range, member_marks, member_scope) \
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING promotion_id",
)
.bind(&row.promotion_name)
.bind(row.start_time)
.bind(row.end_time)
.bind(row.r#type)
.bind(row.shop_id)
.bind(row.relation_id)
.bind(row.range)
.bind(&row.range_data)
.bind(&row.sku_ids)
.bind(row.is_available)
.bind(row.is_delete)
.bind(row.member_range)
.bind(&row.member_marks)
.bind(&row.member_scope)
.fetch_one(&mut **tx)
.await
.map_err(map_err)?;
Ok(id)
}

// ————————————————— 副本状态与撤销 —————————————————

/// 优惠券副本是否仍有效（存在且未软删）。
pub async fn coupon_copy_alive(&self, copy_id: i32) -> Result<bool> {
let row: (i64,) =
sqlx::query_as("SELECT COUNT(*) FROM coupon WHERE coupon_id = $1 AND COALESCE(is_delete, 0) = 0")
.bind(copy_id)
.fetch_one(&self.pool)
.await
.map_err(map_err)?;
Ok(row.0 > 0)
}

/// 满减副本是否仍存在（product_promotion 无软删列，删除即行消失）。
pub async fn full_reduction_copy_alive(&self, copy_id: i32) -> Result<bool> {
let row: (i64,) =
sqlx::query_as("SELECT COUNT(*) FROM product_promotion WHERE promotion_id = $1")
.bind(copy_id)
.fetch_one(&self.pool)
.await
.map_err(map_err)?;
Ok(row.0 > 0)
}

/// 优惠券副本的已领取数（user_coupon 行数，撤销「未开始」判定用）。
pub async fn coupon_claim_count(&self, copy_id: i32) -> Result<i64> {
let row: (i64,) =
sqlx::query_as("SELECT COUNT(*) FROM user_coupon WHERE coupon_id = $1")
.bind(copy_id)
.fetch_one(&self.pool)
.await
.map_err(map_err)?;
Ok(row.0)
}

/// 撤销优惠券副本：本体软删 + 隐藏，同步行软删 + 停用
/// （与 coupon_manage 的 del 口径一致；已发出的 user_coupon 不动）。
pub async fn stop_coupon_copy_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
copy_id: i32,
) -> Result<()> {
sqlx::query("UPDATE coupon SET is_delete = 1, is_show = 0 WHERE coupon_id = $1")
.bind(copy_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
sqlx::query(
"UPDATE promotion SET is_delete = 1, is_available = 0 WHERE type = $1 AND relation_id = $2",
)
.bind(COUPON_SYNC_TYPE)
.bind(copy_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
Ok(())
}

/// 撤销满减副本：本体停用，同步行停用 + 软删。
pub async fn stop_full_reduction_copy_in_tx(
&self,
tx: &mut Transaction<'_, Postgres>,
copy_id: i32,
) -> Result<()> {
sqlx::query("UPDATE product_promotion SET is_available = 0 WHERE promotion_id = $1")
.bind(copy_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
sqlx::query(
"UPDATE promotion SET is_delete = 1, is_available = 0 WHERE type = $1 AND relation_id = $2",
)
.bind(FULL_REDUCTION_SYNC_TYPE)
.bind(copy_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
Ok(())
}
}

fn map_err(e: sqlx::Error) -> Error {
Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
use super::*;

#[test]
fn select_columns_match_entities() {
// 与迁移 020 的列序 / 实体字段序对齐（批次 14 列、逐店 11 列）
let batch_cols = BATCH_SELECT
.trim_start_matches("SELECT ")
.split(" FROM ")
.next()
.unwrap();
assert_eq!(batch_cols.split(", ").count(), 14);
assert!(batch_cols.starts_with("distribution_id"));
let item_cols = ITEM_SELECT
.trim_start_matches("SELECT ")
.split(" FROM ")
.next()
.unwrap();
assert_eq!(item_cols.split(", ").count(), 11);
assert!(item_cols.starts_with("item_id"));
}

#[test]
fn static_guards_for_sql() {
let src = include_str!("distribution.rs");
// 副本复制必须整行 SELECT 且改写 shop_id / is_global / is_delete
assert!(src.contains("FROM coupon WHERE coupon_id = $3"));
assert!(src.contains("FROM product_promotion WHERE promotion_id = $2"));
// 撤销口径：优惠券软删、满减停用
assert!(src.contains("UPDATE coupon SET is_delete = 1, is_show = 0"));
assert!(src.contains("UPDATE product_promotion SET is_available = 0"));
// 事务内锁定读防并发重复撤销
assert!(src.contains("FOR UPDATE"));
}
}
