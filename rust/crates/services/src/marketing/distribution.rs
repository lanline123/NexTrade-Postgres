//! 营销活动下发服务（marketing::distribution）。
//!
//! 平台级优惠券 / 满减活动一键下发到目标店铺，逐店生成店铺级活动副本。
//! 全部端点仅平台管理员可用（TenantScope 非平台 → 403）。
//!
//! # 口径
//!
//! - **下发为单事务**：批次行 + 逐店复制 + 逐店记录同事务落库，任一
//! 数据库错误整批回滚；逻辑性结果（店铺不存在、已有有效副本跳过）
//! 按逐店记录落 item 状态，不中断整批。
//! - **副本独立**：副本落 coupon / product_promotion 本体（shop_id =
//! 目标店铺）+ promotion 同步行，下发后与来源再无联动——店铺在本店
//! 优惠券 / 满减管理页可改任意本地参数，平台改来源不回同步副本。
//! - **撤销只停未开始的副本**：判定见 core 模型纯函数（优惠券看领取
//! 数与使用窗、满减看活动起止）；已开始/已结束/已被领取的副本保持
//! 不动并把原因回写到逐店记录 message；副本已被店铺自行删除的，
//! 记录直接置已撤销（视为已停）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{
Coupon, MarketingDistribution, MarketingDistributionItem, ProductPromotion, Promotion,
};
use nextrade_core::models::marketing::distribution::{
coupon_copy_revoke_check, derive_batch_status, full_reduction_copy_revoke_check,
source_kind_text, validate_distribute, DistributeDTO, DistributeResultVO,
DistributionDetailVO, DistributionItemVO, DistributionListQuery, DistributionVO, RevokeDTO,
SourceActivityVO, TargetShopVO, BATCH_STATUS_DISTRIBUTED, COUPON_SYNC_TYPE,
FULL_REDUCTION_SYNC_TYPE, ITEM_STATUS_CREATED, ITEM_STATUS_FAILED, ITEM_STATUS_REVOKED,
ITEM_STATUS_SKIPPED, MSG_BATCH_NOT_EXISTS, MSG_COPY_NOT_EXISTS, MSG_DUPLICATED,
MSG_ITEM_NOT_CREATED, MSG_ITEM_NOT_EXISTS, MSG_PLATFORM_ONLY, MSG_SHOP_NOT_EXISTS,
MSG_SOURCE_NOT_EXISTS, MSG_SOURCE_NOT_FULL_REDUCTION, MSG_SOURCE_NOT_PLATFORM,
SOURCE_KIND_COUPON, SOURCE_KIND_FULL_REDUCTION,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 营销活动下发服务。
pub struct DistributionService {
repos: Arc<Repositories>,
}

impl DistributionService {
#[must_use]
pub fn new(repos: Arc<Repositories>) -> Self {
Self { repos}
}

fn repo(
&self,
) -> &nextrade_infrastructure::db::repo::marketing::distribution::DistributionRepository {
&self.repos.marketing.distribution
}

/// 下发控制台仅平台管理员可用。
fn ensure_platform() -> Result<()> {
if TenantScope::current().is_platform() {
Ok(())
} else {
Err(Error::Forbidden(MSG_PLATFORM_ONLY.to_string()))
}
}

// ————————————————— 选择器 —————————————————

/// 可下发的平台级活动（kind 为空时两类都返回）。
pub async fn sources(&self, kind: Option<i32>) -> Result<Vec<SourceActivityVO>> {
Self::ensure_platform()?;
let repo = self.repo();
let mut out = Vec::new();
if kind.is_none() || kind == Some(SOURCE_KIND_COUPON) {
for c in repo.list_source_coupons().await? {
out.push(SourceActivityVO {
source_kind: SOURCE_KIND_COUPON,
source_kind_text: source_kind_text(SOURCE_KIND_COUPON).to_string(),
source_id: c.coupon_id,
source_name: c.coupon_name.clone(),
start_time: if c.send_type == 1 { c.use_start_date} else { 0},
end_time: if c.send_type == 1 { c.use_end_date} else { 0},
});
}
}
if kind.is_none() || kind == Some(SOURCE_KIND_FULL_REDUCTION) {
for p in repo.list_source_full_reductions().await? {
out.push(SourceActivityVO {
source_kind: SOURCE_KIND_FULL_REDUCTION,
source_kind_text: source_kind_text(SOURCE_KIND_FULL_REDUCTION).to_string(),
source_id: p.promotion_id,
source_name: p.promotion_name.clone(),
start_time: p.start_time,
end_time: p.end_time,
});
}
}
Ok(out)
}

/// 目标店铺列表。
pub async fn shops(&self) -> Result<Vec<TargetShopVO>> {
Self::ensure_platform()?;
Ok(self
.repo()
.list_shops()
.await?
.into_iter()
.map(|(shop_id, shop_name)| TargetShopVO { shop_id, shop_name})
.collect())
}

// ————————————————— 下发 —————————————————

/// 一键下发：逐店生成副本，返回逐店结果。
pub async fn distribute(&self, dto: &DistributeDTO) -> Result<DistributeResultVO> {
Self::ensure_platform()?;
let (kind, source_id, targets) =
validate_distribute(dto).map_err(Error::biz_msg)?;
let repo = self.repo();

// 来源校验：必须存在、未删除、平台级（shop_id 归一为 0）。
let (source_name, source_coupon, source_reduction) = match kind {
SOURCE_KIND_COUPON => {
let coupon = repo
.find_coupon(source_id)
.await?
.filter(|c| c.is_delete.unwrap_or(0) == 0)
.ok_or_else(|| Error::biz_msg(MSG_SOURCE_NOT_EXISTS))?;
if coupon.shop_id.unwrap_or(0)!= 0 {
return Err(Error::biz_msg(MSG_SOURCE_NOT_PLATFORM));
}
(coupon.coupon_name.clone(), Some(coupon), None)
}
_ => {
let row = repo
.find_full_reduction(source_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_SOURCE_NOT_EXISTS))?;
if row.shop_id!= 0 {
return Err(Error::biz_msg(MSG_SOURCE_NOT_PLATFORM));
}
if row.promotion_type
!= nextrade_core::models::marketing::distribution::FULL_REDUCTION_PROMOTION_TYPE
{
return Err(Error::biz_msg(MSG_SOURCE_NOT_FULL_REDUCTION));
}
(row.promotion_name.clone(), None, Some(row))
}
};

let now = now_seconds() as i32;
let mut tx = repo.begin().await?;
let batch = MarketingDistribution {
source_kind: kind,
source_id,
source_name: source_name.clone(),
target_count: i32::try_from(targets.len()).unwrap_or(i32::MAX),
status: BATCH_STATUS_DISTRIBUTED,
remark: dto.remark.clone().unwrap_or_default(),
add_time: now,
update_time: now,
..Default::default()
};
let batch_id = repo.insert_batch_in_tx(&mut tx, &batch).await?;

let mut items: Vec<MarketingDistributionItem> = Vec::with_capacity(targets.len());
let (mut success, mut skipped, mut failed) = (0_i32, 0_i32, 0_i32);
for shop_id in &targets {
let mut item = MarketingDistributionItem {
distribution_id: batch_id,
source_kind: kind,
source_id,
target_shop_id: *shop_id,
add_time: now,
update_time: now,
..Default::default()
};
let Some(shop_name) = repo.find_shop_name(*shop_id).await? else {
item.status = ITEM_STATUS_FAILED;
item.message = MSG_SHOP_NOT_EXISTS.to_string();
failed += 1;
item.item_id = repo.insert_item_in_tx(&mut tx, &item).await?;
items.push(item);
continue;
};
item.target_shop_name = shop_name;

// 防重复：已有有效副本（记录为已生成且本体未删除）→ 跳过。
if let Some(prev) = repo.find_created_item(kind, source_id, *shop_id).await? {
let alive = match kind {
SOURCE_KIND_COUPON => repo.coupon_copy_alive(prev.copy_id).await?,
_ => repo.full_reduction_copy_alive(prev.copy_id).await?,
};
if alive {
item.status = ITEM_STATUS_SKIPPED;
item.copy_id = prev.copy_id;
item.message = MSG_DUPLICATED.to_string();
skipped += 1;
item.item_id = repo.insert_item_in_tx(&mut tx, &item).await?;
items.push(item);
continue;
}
}

// 生成副本本体 + promotion 同步行。
let copy_id = match kind {
SOURCE_KIND_COUPON => {
let coupon = source_coupon.as_ref().expect("coupon source loaded");
let new_id = repo
.copy_coupon_in_tx(&mut tx, source_id, *shop_id, now)
.await?
.ok_or_else(|| Error::biz_msg(MSG_SOURCE_NOT_EXISTS))?;
if!repo
.copy_promotion_row_in_tx(
&mut tx,
COUPON_SYNC_TYPE,
source_id,
new_id,
*shop_id,
)
.await?
{
// 来源缺同步行（存量异常数据）：按券本体字段兜底构造。
let fallback = coupon_promotion_fallback(coupon, new_id, *shop_id);
repo.insert_promotion_row_in_tx(&mut tx, &fallback).await?;
}
new_id
}
_ => {
let row = source_reduction.as_ref().expect("reduction source loaded");
let new_id = repo
.copy_full_reduction_in_tx(&mut tx, source_id, *shop_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_SOURCE_NOT_EXISTS))?;
if!repo
.copy_promotion_row_in_tx(
&mut tx,
FULL_REDUCTION_SYNC_TYPE,
source_id,
new_id,
*shop_id,
)
.await?
{
let fallback = reduction_promotion_fallback(row, new_id, *shop_id);
repo.insert_promotion_row_in_tx(&mut tx, &fallback).await?;
}
new_id
}
};
item.status = ITEM_STATUS_CREATED;
item.copy_id = copy_id;
success += 1;
item.item_id = repo.insert_item_in_tx(&mut tx, &item).await?;
items.push(item);
}

let mut batch_row = batch;
batch_row.distribution_id = batch_id;
batch_row.success_count = success;
batch_row.skipped_count = skipped;
batch_row.failed_count = failed;
batch_row.update_time = now;
repo.update_batch_in_tx(&mut tx, &batch_row).await?;
tx.commit()
.await
.map_err(|e| Error::Database(e.to_string()))?;

let log_info = format!(
"营销活动下发:{}({}) → {} 店（成功 {} / 跳过 {} / 失败 {}）",
source_name,
source_kind_text(kind),
targets.len(),
success,
skipped,
failed
);
let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
tracing::warn!("写管理操作日志失败: {e}");
}
tracing::info!("{log_info}");

Ok(DistributeResultVO {
distribution_id: batch_id,
items: items.iter().map(DistributionItemVO::from).collect(),
})
}

// ————————————————— 记录查询 —————————————————

/// 下发记录分页。
pub async fn list(
&self,
q: &DistributionListQuery,
) -> Result<PageResult<DistributionVO>> {
Self::ensure_platform()?;
let (total, rows) = self.repo().paginate_batches(q).await?;
let list: Vec<DistributionVO> = rows.iter().map(DistributionVO::from).collect();
let (size, offset) =
nextrade_core::models::marketing::distribution::limit_offset(q.page, q.size);
let page = nextrade_core::types::PageQuery {
page: u64::try_from(if size > 0 { offset / size + 1} else { 1}).unwrap_or(1),
page_size: u64::try_from(size.max(1)).unwrap_or(15),
};
Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
}

/// 批次详情（批次 + 逐店记录）。
pub async fn detail(&self, distribution_id: i64) -> Result<DistributionDetailVO> {
Self::ensure_platform()?;
let repo = self.repo();
let batch = repo
.find_batch(distribution_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_BATCH_NOT_EXISTS))?;
let items = repo.list_items(distribution_id).await?;
Ok(DistributionDetailVO {
batch: DistributionVO::from(&batch),
items: items.iter().map(DistributionItemVO::from).collect(),
})
}

// ————————————————— 撤销 —————————————————

/// 整批撤销：逐店尝试撤销未开始的副本，返回最新详情。
pub async fn revoke_batch(&self, dto: &RevokeDTO) -> Result<DistributionDetailVO> {
Self::ensure_platform()?;
let distribution_id = dto.distribution_id.unwrap_or(0);
if distribution_id <= 0 {
return Err(Error::biz_msg(MSG_BATCH_NOT_EXISTS));
}
let repo = self.repo();
let mut tx = repo.begin().await?;
let mut batch = repo
.find_batch_in_tx(&mut tx, distribution_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_BATCH_NOT_EXISTS))?;
let items = repo.list_items(distribution_id).await?;
for item in &items {
if item.status == ITEM_STATUS_CREATED {
self.revoke_one_in_tx(&mut tx, item.item_id).await?;
}
}
self.refresh_batch_in_tx(&mut tx, &mut batch).await?;
tx.commit()
.await
.map_err(|e| Error::Database(e.to_string()))?;
tracing::info!("营销活动下发整批撤销: batch={distribution_id}");
self.detail(distribution_id).await
}

/// 单店撤销：返回该批次最新详情。
pub async fn revoke_item(&self, dto: &RevokeDTO) -> Result<DistributionDetailVO> {
Self::ensure_platform()?;
let item_id = dto.item_id.unwrap_or(0);
if item_id <= 0 {
return Err(Error::biz_msg(MSG_ITEM_NOT_EXISTS));
}
let repo = self.repo();
let item = repo
.find_item(item_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_ITEM_NOT_EXISTS))?;
if item.status!= ITEM_STATUS_CREATED {
return Err(Error::biz_msg(MSG_ITEM_NOT_CREATED));
}
let mut tx = repo.begin().await?;
let mut batch = repo
.find_batch_in_tx(&mut tx, item.distribution_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_BATCH_NOT_EXISTS))?;
self.revoke_one_in_tx(&mut tx, item_id).await?;
self.refresh_batch_in_tx(&mut tx, &mut batch).await?;
tx.commit()
.await
.map_err(|e| Error::Database(e.to_string()))?;
tracing::info!("营销活动下发单店撤销: item={item_id}");
self.detail(item.distribution_id).await
}

/// 事务内撤销一条逐店记录：未开始才停副本；不可撤销时把原因回写
/// message（状态保持已生成）；副本已被店铺删除的直接置已撤销。
async fn revoke_one_in_tx(
&self,
tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
item_id: i64,
) -> Result<()> {
let repo = self.repo();
let Some(mut item) = repo.find_item_in_tx(tx, item_id).await? else {
return Ok(());
};
if item.status!= ITEM_STATUS_CREATED {
return Ok(());
}
let now = now_seconds() as i32;
let alive = match item.source_kind {
SOURCE_KIND_COUPON => repo.coupon_copy_alive(item.copy_id).await?,
_ => repo.full_reduction_copy_alive(item.copy_id).await?,
};
if!alive {
item.status = ITEM_STATUS_REVOKED;
item.message = MSG_COPY_NOT_EXISTS.to_string();
item.update_time = now;
repo.update_item_in_tx(tx, &item).await?;
return Ok(());
}
let check = match item.source_kind {
SOURCE_KIND_COUPON => {
let copy = repo
.find_coupon(item.copy_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_COPY_NOT_EXISTS))?;
let claims = repo.coupon_claim_count(item.copy_id).await?;
coupon_copy_revoke_check(
copy.send_type,
copy.use_start_date,
copy.use_end_date,
claims,
now,
)
}
_ => {
let copy = repo
.find_full_reduction(item.copy_id)
.await?
.ok_or_else(|| Error::biz_msg(MSG_COPY_NOT_EXISTS))?;
full_reduction_copy_revoke_check(copy.start_time, copy.end_time, now)
}
};
match check {
Ok(()) => {
match item.source_kind {
SOURCE_KIND_COUPON => repo.stop_coupon_copy_in_tx(tx, item.copy_id).await?,
_ => repo.stop_full_reduction_copy_in_tx(tx, item.copy_id).await?,
}
item.status = ITEM_STATUS_REVOKED;
item.message = String::new();
}
Err(reason) => {
// 已开始/已结束/已被领取：副本不动，原因回写供后台查看。
item.message = reason.to_string();
}
}
item.update_time = now;
repo.update_item_in_tx(tx, &item).await?;
Ok(())
}

/// 撤销后按逐店最终状态刷新批次计数与状态。
async fn refresh_batch_in_tx(
&self,
tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
batch: &mut MarketingDistribution,
) -> Result<()> {
let items = self.repo().list_items(batch.distribution_id).await?;
batch.revoked_count = i32::try_from(
items
.iter()
.filter(|i| i.status == ITEM_STATUS_REVOKED)
.count(),
)
.unwrap_or(0);
batch.status = derive_batch_status(&items);
batch.update_time = now_seconds() as i32;
self.repo().update_batch_in_tx(tx, batch).await?;
Ok(())
}
}

/// 来源缺 promotion 同步行时，按券本体字段兜底构造（与 coupon_manage
/// 的 build_coupon_promotion 同口径：固定时间型取使用窗，其余 0；
/// 会员范围字段来源行已佚失，落默认全员）。
fn coupon_promotion_fallback(coupon: &Coupon, new_coupon_id: i32, shop_id: i32) -> Promotion {
let (start_time, end_time) = if coupon.send_type > 0 {
(coupon.use_start_date, coupon.use_end_date)
} else {
(0, 0)
};
Promotion {
promotion_name: coupon.coupon_name.clone(),
start_time,
end_time,
r#type: COUPON_SYNC_TYPE,
shop_id,
relation_id: Some(new_coupon_id),
range: Some(coupon.send_range),
range_data: Some(coupon.send_range_data.clone()),
is_available: Some(1),
..Default::default()
}
}

/// 来源缺 promotion 同步行时，按满减本体字段兜底构造。
fn reduction_promotion_fallback(
row: &ProductPromotion,
new_promotion_id: i32,
shop_id: i32,
) -> Promotion {
Promotion {
promotion_name: row.promotion_name.clone(),
start_time: row.start_time,
end_time: row.end_time,
r#type: FULL_REDUCTION_SYNC_TYPE,
shop_id,
relation_id: Some(new_promotion_id),
range: Some(row.range),
range_data: Some(row.range_data.clone()),
is_available: Some(row.is_available),
..Default::default()
}
}
