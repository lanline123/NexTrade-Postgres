//! 营销活动下发域（marketing::distribution）DTO / VO / 纯函数。
//!
//!
//! 平台级（shop_id = 0）优惠券 / 满减活动一键下发：按目标店铺逐店复制
//! 一份店铺级活动副本（coupon / product_promotion 本体 + 对应 promotion
//! 同步行），下发关系落 `marketing_distribution`（批次）与
//! `marketing_distribution_item`（逐店）两表（迁移 020）。
//!
//! # 口径（与报告一致）
//!
//! - **副本独立**：副本是目标店铺自己的活动行，下发后店铺可在本店的
//! 优惠券 / 满减管理页改任意本地参数（名称、金额、时间、范围、启停），
//! 改动只影响副本；平台此后改来源活动**不回同步**已有副本。
//! - **防重复**：同一来源活动对同一店铺已有「有效副本」（下发记录为
//! 已生成且副本本体未删除）时再次下发自动跳过并在记录中注明。
//! - **撤销只停未开始的副本**：已开始（含已结束、优惠券已有人领取）的
//! 副本不撤销，逐条在记录中注明原因；撤销动作 = 停用并软删副本
//! （优惠券走既有软删口径、满减置停用），已生成/已发放到会员手里的
//! 用户券不受影响（未开始判定含「无人领取」条件）。

use serde::{Deserialize, Serialize};

use crate::entities::{MarketingDistribution, MarketingDistributionItem};

/// 来源类型：优惠券。
pub const SOURCE_KIND_COUPON: i32 = 1;
/// 来源类型：满减（product_promotion.promotion_type = 1）。
pub const SOURCE_KIND_FULL_REDUCTION: i32 = 2;

/// 满减在 product_promotion.promotion_type 的取值。
pub const FULL_REDUCTION_PROMOTION_TYPE: i32 = 1;
/// 满减在 promotion.type 的取值（1 秒杀 2 优惠券 3 满减 4 折扣 5 满赠）。
pub const FULL_REDUCTION_SYNC_TYPE: i32 = 3;
/// 优惠券在 promotion.type 的取值。
pub const COUPON_SYNC_TYPE: i32 = 2;

/// 批次状态：已下发。
pub const BATCH_STATUS_DISTRIBUTED: i32 = 1;
/// 批次状态：已撤销（可撤销副本全部撤销完）。
pub const BATCH_STATUS_REVOKED: i32 = 2;
/// 批次状态：部分撤销。
pub const BATCH_STATUS_PARTIAL_REVOKED: i32 = 3;

/// 逐店状态：待处理。
pub const ITEM_STATUS_PENDING: i32 = 0;
/// 逐店状态：已生成副本。
pub const ITEM_STATUS_CREATED: i32 = 1;
/// 逐店状态：已跳过（已有有效副本）。
pub const ITEM_STATUS_SKIPPED: i32 = 2;
/// 逐店状态：已撤销。
pub const ITEM_STATUS_REVOKED: i32 = 3;
/// 逐店状态：失败。
pub const ITEM_STATUS_FAILED: i32 = 4;

pub const MSG_PLATFORM_ONLY: &str = "仅平台管理员可执行活动下发";
pub const MSG_KIND_INVALID: &str = "活动类型有误";
pub const MSG_SOURCE_REQUIRED: &str = "请选择要下发的活动";
pub const MSG_SOURCE_NOT_EXISTS: &str = "来源活动不存在或已删除";
pub const MSG_SOURCE_NOT_PLATFORM: &str = "只能下发平台级活动（店铺活动请在店铺内自行管理）";
pub const MSG_SOURCE_NOT_FULL_REDUCTION: &str = "该活动不是满减活动，暂不支持下发";
pub const MSG_TARGETS_REQUIRED: &str = "请选择至少一个目标店铺";
pub const MSG_SHOP_NOT_EXISTS: &str = "目标店铺不存在";
pub const MSG_BATCH_NOT_EXISTS: &str = "下发记录不存在";
pub const MSG_ITEM_NOT_EXISTS: &str = "下发记录明细不存在";
pub const MSG_ITEM_NOT_CREATED: &str = "该记录没有可撤销的副本";
pub const MSG_COPY_NOT_EXISTS: &str = "副本已被删除，无需撤销";
pub const MSG_COPY_STARTED: &str = "副本已开始，不可撤销";
pub const MSG_COPY_ENDED: &str = "副本已结束，无需撤销";
pub const MSG_COPY_CLAIMED: &str = "副本优惠券已被会员领取，不可撤销";
pub const MSG_DUPLICATED: &str = "该店铺已有本活动的有效副本，已跳过";

/// 来源类型文案。
pub fn source_kind_text(kind: i32) -> &'static str {
match kind {
SOURCE_KIND_COUPON => "优惠券",
SOURCE_KIND_FULL_REDUCTION => "满减",
_ => "未知",
}
}

/// 批次状态文案。
pub fn batch_status_text(status: i32) -> &'static str {
match status {
BATCH_STATUS_DISTRIBUTED => "已下发",
BATCH_STATUS_REVOKED => "已撤销",
BATCH_STATUS_PARTIAL_REVOKED => "部分撤销",
_ => "未知",
}
}

/// 逐店状态文案。
pub fn item_status_text(status: i32) -> &'static str {
match status {
ITEM_STATUS_PENDING => "待处理",
ITEM_STATUS_CREATED => "已生成",
ITEM_STATUS_SKIPPED => "已跳过",
ITEM_STATUS_REVOKED => "已撤销",
ITEM_STATUS_FAILED => "失败",
_ => "未知",
}
}

/// 活动时间状态（撤销判定用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyPhase {
/// 未开始（可撤销）。
NotStarted,
/// 进行中（不可撤销）。
Running,
/// 已结束（不可撤销）。
Ended,
}

/// 按起止时间判定活动阶段（纯函数）：start/end 为秒级时间戳，
/// 0 表示未设置（start = 0 视为已开始，end = 0 视为不限期）。
pub fn time_phase(start_time: i32, end_time: i32, now: i32) -> CopyPhase {
if end_time > 0 && now > end_time {
return CopyPhase::Ended;
}
if start_time > 0 && now < start_time {
return CopyPhase::NotStarted;
}
CopyPhase::Running
}

/// 优惠券副本可否撤销（纯函数）：
/// - 已有人领取（user_coupon 存在）→ 不可撤销（券已进入流通）；
/// - 固定时间型（send_type = 1）：使用窗未开始才可撤销；
/// - 领取后天数型（send_type = 0）：无固定使用窗，无人领取即可撤销。
/// 返回 Err 为不可撤销原因（与后台提示文案一致）。
pub fn coupon_copy_revoke_check(
send_type: i32,
use_start_date: i32,
use_end_date: i32,
claim_count: i64,
now: i32,
) -> Result<(), &'static str> {
if claim_count > 0 {
return Err(MSG_COPY_CLAIMED);
}
if send_type == 1 {
match time_phase(use_start_date, use_end_date, now) {
CopyPhase::NotStarted => Ok(()),
CopyPhase::Running => Err(MSG_COPY_STARTED),
CopyPhase::Ended => Err(MSG_COPY_ENDED),
}
} else {
Ok(())
}
}

/// 满减副本可否撤销（纯函数）：仅未开始可撤销。
pub fn full_reduction_copy_revoke_check(
start_time: i32,
end_time: i32,
now: i32,
) -> Result<(), &'static str> {
match time_phase(start_time, end_time, now) {
CopyPhase::NotStarted => Ok(()),
CopyPhase::Running => Err(MSG_COPY_STARTED),
CopyPhase::Ended => Err(MSG_COPY_ENDED),
}
}

/// 批次状态推导（纯函数）：按逐店最终状态汇总。
/// - 无任何已生成/已撤销条目（全跳过或失败）→ 保持已下发；
/// - 已生成条目全部撤销 → 已撤销；
/// - 部分撤销 → 部分撤销；
/// - 无撤销发生 → 已下发。
pub fn derive_batch_status(items: &[MarketingDistributionItem]) -> i32 {
let created = items
.iter()
.filter(|i| i.status == ITEM_STATUS_CREATED)
.count();
let revoked = items
.iter()
.filter(|i| i.status == ITEM_STATUS_REVOKED)
.count();
if revoked == 0 {
return BATCH_STATUS_DISTRIBUTED;
}
if created == 0 {
return BATCH_STATUS_REVOKED;
}
BATCH_STATUS_PARTIAL_REVOKED
}

/// 下发入参。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DistributeDTO {
/// 来源类型：1 优惠券 / 2 满减。
pub source_kind: Option<i32>,
/// 来源活动 ID。
pub source_id: Option<i32>,
/// 目标店铺 ID 列表。
pub target_shop_ids: Vec<i32>,
/// 备注（可选）。
pub remark: Option<String>,
}

/// 下发校验（纯函数）：返回 (source_kind, source_id, 去重保序后的目标店铺)。
pub fn validate_distribute(dto: &DistributeDTO) -> Result<(i32, i32, Vec<i32>), String> {
let kind = dto.source_kind.unwrap_or(0);
if kind!= SOURCE_KIND_COUPON && kind!= SOURCE_KIND_FULL_REDUCTION {
return Err(MSG_KIND_INVALID.to_string());
}
let source_id = dto.source_id.unwrap_or(0);
if source_id <= 0 {
return Err(MSG_SOURCE_REQUIRED.to_string());
}
let mut targets: Vec<i32> = Vec::new();
for id in &dto.target_shop_ids {
if *id > 0 &&!targets.contains(id) {
targets.push(*id);
}
}
if targets.is_empty() {
return Err(MSG_TARGETS_REQUIRED.to_string());
}
Ok((kind, source_id, targets))
}

/// 撤销入参（整批 / 单条共用结构，按字段二选一）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RevokeDTO {
pub distribution_id: Option<i64>,
pub item_id: Option<i64>,
}

/// 批次列表筛选。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DistributionListQuery {
pub source_kind: Option<i32>,
pub status: Option<i32>,
pub page: Option<i64>,
pub size: Option<i64>,
}

/// 可下发来源活动 VO（平台级活动选择器）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceActivityVO {
pub source_kind: i32,
pub source_kind_text: String,
pub source_id: i32,
pub source_name: String,
/// 活动开始时间（秒；优惠券固定时间型取 use_start_date，其余 0）。
pub start_time: i32,
/// 活动结束时间（秒；口径同上）。
pub end_time: i32,
}

/// 目标店铺 VO（店铺选择器）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetShopVO {
pub shop_id: i32,
pub shop_name: String,
}

/// 批次 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionVO {
pub distribution_id: i64,
pub source_kind: i32,
pub source_kind_text: String,
pub source_id: i32,
pub source_name: String,
pub target_count: i32,
pub success_count: i32,
pub skipped_count: i32,
pub failed_count: i32,
pub revoked_count: i32,
pub status: i32,
pub status_text: String,
pub remark: String,
pub add_time: i32,
pub update_time: i32,
}

impl From<&MarketingDistribution> for DistributionVO {
fn from(row: &MarketingDistribution) -> Self {
Self {
distribution_id: row.distribution_id,
source_kind: row.source_kind,
source_kind_text: source_kind_text(row.source_kind).to_string(),
source_id: row.source_id,
source_name: row.source_name.clone(),
target_count: row.target_count,
success_count: row.success_count,
skipped_count: row.skipped_count,
failed_count: row.failed_count,
revoked_count: row.revoked_count,
status: row.status,
status_text: batch_status_text(row.status).to_string(),
remark: row.remark.clone(),
add_time: row.add_time,
update_time: row.update_time,
}
}
}

/// 逐店记录 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionItemVO {
pub item_id: i64,
pub distribution_id: i64,
pub target_shop_id: i32,
pub target_shop_name: String,
pub copy_id: i32,
pub status: i32,
pub status_text: String,
pub message: String,
pub add_time: i32,
pub update_time: i32,
}

impl From<&MarketingDistributionItem> for DistributionItemVO {
fn from(row: &MarketingDistributionItem) -> Self {
Self {
item_id: row.item_id,
distribution_id: row.distribution_id,
target_shop_id: row.target_shop_id,
target_shop_name: row.target_shop_name.clone(),
copy_id: row.copy_id,
status: row.status,
status_text: item_status_text(row.status).to_string(),
message: row.message.clone(),
add_time: row.add_time,
update_time: row.update_time,
}
}
}

/// 批次详情 VO（批次 + 逐店记录）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDetailVO {
pub batch: DistributionVO,
pub items: Vec<DistributionItemVO>,
}

/// 下发结果 VO（逐店结果即时回显）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributeResultVO {
pub distribution_id: i64,
pub items: Vec<DistributionItemVO>,
}

/// 分页（与 recharge_gift 同口径）。
pub fn limit_offset(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
let size = size.unwrap_or(15).clamp(1, 100);
let page = page.unwrap_or(1).max(1);
(size, (page - 1) * size)
}

#[cfg(test)]
mod tests {
use super::*;

#[test]
fn time_phase_boundaries() {
// 未设置开始时间 = 已开始；未设置结束 = 不限期
assert_eq!(time_phase(0, 0, 100), CopyPhase::Running);
assert_eq!(time_phase(200, 300, 100), CopyPhase::NotStarted);
assert_eq!(time_phase(200, 300, 200), CopyPhase::Running);
assert_eq!(time_phase(200, 300, 300), CopyPhase::Running);
assert_eq!(time_phase(200, 300, 301), CopyPhase::Ended);
assert_eq!(time_phase(0, 300, 301), CopyPhase::Ended);
}

#[test]
fn coupon_revoke_rules() {
// 领取后天数型：无人领取可撤销，有人领取不可
assert!(coupon_copy_revoke_check(0, 0, 0, 0, 100).is_ok());
assert_eq!(
coupon_copy_revoke_check(0, 0, 0, 1, 100),
Err(MSG_COPY_CLAIMED)
);
// 固定时间型：窗前可撤销，窗内已开始，窗后已结束
assert!(coupon_copy_revoke_check(1, 200, 300, 0, 100).is_ok());
assert_eq!(
coupon_copy_revoke_check(1, 200, 300, 0, 250),
Err(MSG_COPY_STARTED)
);
assert_eq!(
coupon_copy_revoke_check(1, 200, 300, 0, 400),
Err(MSG_COPY_ENDED)
);
// 固定时间型但已有人领取：领取优先判定
assert_eq!(
coupon_copy_revoke_check(1, 200, 300, 3, 100),
Err(MSG_COPY_CLAIMED)
);
}

#[test]
fn full_reduction_revoke_rules() {
assert!(full_reduction_copy_revoke_check(200, 300, 100).is_ok());
assert_eq!(
full_reduction_copy_revoke_check(200, 300, 250),
Err(MSG_COPY_STARTED)
);
assert_eq!(
full_reduction_copy_revoke_check(200, 300, 400),
Err(MSG_COPY_ENDED)
);
}

#[test]
fn batch_status_derivation() {
let item = |status: i32| MarketingDistributionItem {
status,
..Default::default()
};
assert_eq!(derive_batch_status(&[]), BATCH_STATUS_DISTRIBUTED);
assert_eq!(
derive_batch_status(&[item(ITEM_STATUS_CREATED), item(ITEM_STATUS_SKIPPED)]),
BATCH_STATUS_DISTRIBUTED
);
assert_eq!(
derive_batch_status(&[item(ITEM_STATUS_REVOKED), item(ITEM_STATUS_SKIPPED)]),
BATCH_STATUS_REVOKED
);
assert_eq!(
derive_batch_status(&[item(ITEM_STATUS_CREATED), item(ITEM_STATUS_REVOKED)]),
BATCH_STATUS_PARTIAL_REVOKED
);
}

#[test]
fn distribute_validation() {
let dto = DistributeDTO {
source_kind: Some(SOURCE_KIND_COUPON),
source_id: Some(9),
target_shop_ids: vec![3, 0, 3, -1, 5],
remark: None,
};
assert_eq!(validate_distribute(&dto), Ok((SOURCE_KIND_COUPON, 9, vec![3, 5])));
let bad_kind = DistributeDTO {
source_kind: Some(7),
source_id: Some(9),
target_shop_ids: vec![3],
remark: None,
};
assert_eq!(validate_distribute(&bad_kind), Err(MSG_KIND_INVALID.to_string()));
let no_targets = DistributeDTO {
source_kind: Some(SOURCE_KIND_FULL_REDUCTION),
source_id: Some(9),
target_shop_ids: vec![],
remark: None,
};
assert_eq!(
validate_distribute(&no_targets),
Err(MSG_TARGETS_REQUIRED.to_string())
);
let no_source = DistributeDTO {
source_kind: Some(SOURCE_KIND_COUPON),
source_id: None,
target_shop_ids: vec![3],
remark: None,
};
assert_eq!(
validate_distribute(&no_source),
Err(MSG_SOURCE_REQUIRED.to_string())
);
}

#[test]
fn status_texts() {
assert_eq!(source_kind_text(SOURCE_KIND_COUPON), "优惠券");
assert_eq!(source_kind_text(SOURCE_KIND_FULL_REDUCTION), "满减");
assert_eq!(batch_status_text(BATCH_STATUS_PARTIAL_REVOKED), "部分撤销");
assert_eq!(item_status_text(ITEM_STATUS_SKIPPED), "已跳过");
}
}
