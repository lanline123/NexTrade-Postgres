//! 充值赠品领取记录域（marketing::recharge_gift）DTO / VO / 纯函数。
//!
//! 【我方新增 2026-11-21，论坛帖 717721「余额充值送实物礼品」；原版无此功能】
//! 表 `recharge_gift_record`：充值单置「已支付」时由充值单服务同事务生成
//! （`recharge_order_id` 唯一防重）；会员领取时扣减活动赠品池
//! `product_gift.gift_stock` 并生成 ¥0 履约订单（order_type = 12）走商家
//! 正常发货链。状态机：0 待领取 → 1 已领取 / 2 已作废 / 3 异常（库存不足等），
//! 异常/作废可由后台在关联充值单仍为已支付时恢复为待领取。

use serde::{Deserialize, Serialize};

use crate::entities::RechargeGiftRecord;

pub const COLUMNS: [&str; 20] = [
    "record_id",
    "recharge_order_id",
    "recharge_id",
    "user_id",
    "gift_id",
    "gift_name",
    "product_id",
    "sku_id",
    "shop_id",
    "quantity",
    "status",
    "order_id",
    "address_id",
    "consignee",
    "mobile",
    "address_text",
    "admin_note",
    "add_time",
    "claim_time",
    "update_time",
];
pub const KEY_COLUMN: &str = "record_id";

/// 状态：待领取。
pub const STATUS_PENDING: i32 = 0;
/// 状态：已领取（已生成履约订单）。
pub const STATUS_CLAIMED: i32 = 1;
/// 状态：已作废。
pub const STATUS_VOID: i32 = 2;
/// 状态：异常（库存不足等，待后台处置）。
pub const STATUS_ABNORMAL: i32 = 3;

/// 充值赠品履约订单的 order_type（我方新增编号；既有编号见订单实体注释，
/// 12 此前未占用）。
pub const GIFT_ORDER_TYPE: i32 = 12;
/// 履约订单来源标识。
pub const GIFT_ORDER_SOURCE: &str = "rechargeGift";

pub const MSG_RECORD_NOT_EXISTS: &str = "赠品记录不存在";
pub const MSG_NOT_OWNER: &str = "赠品记录不存在";
pub const MSG_NOT_PENDING: &str = "该赠品已领取或已失效";
pub const MSG_STOCK_NOT_ENOUGH: &str = "赠品库存不足，已为您登记异常，请联系客服";
pub const MSG_ADDRESS_REQUIRED: &str = "请选择收货地址";
pub const MSG_ADDRESS_NOT_EXISTS: &str = "收货地址不存在";
pub const MSG_RECHARGE_NOT_PAID: &str = "关联充值单非已支付状态，不可恢复";
pub const MSG_STATUS_INVALID: &str = "当前状态不可执行该操作";

/// 状态文案（与后台/会员端展示一致）。
pub fn status_text(status: i32) -> &'static str {
    match status {
        STATUS_PENDING => "待领取",
        STATUS_CLAIMED => "已领取",
        STATUS_VOID => "已作废",
        STATUS_ABNORMAL => "异常",
        _ => "未知",
    }
}

/// 领取入参（会员端）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RechargeGiftClaimDTO {
    pub record_id: Option<i64>,
    pub address_id: Option<i32>,
}

/// 后台处置入参（作废 / 恢复）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RechargeGiftAdminActionDTO {
    pub record_id: Option<i64>,
    pub note: Option<String>,
}

/// 赠品记录 VO（后台与会员端共用字段集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeGiftRecordVO {
    pub record_id: i64,
    pub recharge_order_id: i32,
    pub recharge_id: i32,
    pub user_id: i32,
    pub gift_id: i32,
    pub gift_name: String,
    pub product_id: i32,
    pub sku_id: i32,
    pub shop_id: i32,
    pub quantity: i32,
    pub status: i32,
    pub status_text: String,
    pub order_id: i32,
    pub address_id: i32,
    pub consignee: String,
    pub mobile: String,
    pub address_text: String,
    pub admin_note: String,
    pub add_time: i32,
    pub claim_time: i32,
    /// 赠品图片（服务层按 product_id 装配 product.pic_thumb）
    pub gift_pic: Option<String>,
}

impl From<&RechargeGiftRecord> for RechargeGiftRecordVO {
    fn from(record: &RechargeGiftRecord) -> Self {
        Self {
            record_id: record.record_id,
            recharge_order_id: record.recharge_order_id,
            recharge_id: record.recharge_id,
            user_id: record.user_id,
            gift_id: record.gift_id,
            gift_name: record.gift_name.clone(),
            product_id: record.product_id,
            sku_id: record.sku_id,
            shop_id: record.shop_id,
            quantity: record.quantity,
            status: record.status,
            status_text: status_text(record.status).to_string(),
            order_id: record.order_id,
            address_id: record.address_id,
            consignee: record.consignee.clone(),
            mobile: record.mobile.clone(),
            address_text: record.address_text.clone(),
            admin_note: record.admin_note.clone(),
            add_time: record.add_time,
            claim_time: record.claim_time,
            gift_pic: None,
        }
    }
}

/// 列表筛选（后台 / 会员端共用；会员端由服务层强制 user_id）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RechargeGiftListQuery {
    pub user_id: Option<i32>,
    pub status: Option<i32>,
    pub page: Option<i64>,
    pub size: Option<i64>,
}

/// 分页（与 recharge_setting 同口径）。
pub fn limit_offset(page: Option<i64>, size: Option<i64>) -> (i64, i64) {
    let size = size.unwrap_or(15).clamp(1, 100);
    let page = page.unwrap_or(1).max(1);
    (size, (page - 1) * size)
}

/// 领取前置校验（纯函数）：记录必须属于领取人且处于待领取。
pub fn validate_claim(record: &RechargeGiftRecord, user_id: i32) -> Result<(), String> {
    if record.user_id != user_id {
        return Err(MSG_NOT_OWNER.to_string());
    }
    if record.status != STATUS_PENDING {
        return Err(MSG_NOT_PENDING.to_string());
    }
    Ok(())
}

/// 后台恢复前置校验（纯函数）：仅异常/作废可恢复。
pub fn validate_restore(record: &RechargeGiftRecord) -> Result<(), String> {
    if record.status != STATUS_ABNORMAL && record.status != STATUS_VOID {
        return Err(MSG_STATUS_INVALID.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(status: i32, user_id: i32) -> RechargeGiftRecord {
        RechargeGiftRecord {
            record_id: 1,
            user_id,
            status,
            ..Default::default()
        }
    }

    #[test]
    fn status_texts() {
        assert_eq!(status_text(STATUS_PENDING), "待领取");
        assert_eq!(status_text(STATUS_CLAIMED), "已领取");
        assert_eq!(status_text(STATUS_VOID), "已作废");
        assert_eq!(status_text(STATUS_ABNORMAL), "异常");
        assert_eq!(status_text(99), "未知");
    }

    #[test]
    fn claim_validation() {
        assert!(validate_claim(&record(STATUS_PENDING, 7), 7).is_ok());
        // 他人记录与不存在同口径，不泄露归属
        assert_eq!(
            validate_claim(&record(STATUS_PENDING, 8), 7),
            Err(MSG_NOT_OWNER.to_string())
        );
        assert_eq!(
            validate_claim(&record(STATUS_CLAIMED, 7), 7),
            Err(MSG_NOT_PENDING.to_string())
        );
        assert_eq!(
            validate_claim(&record(STATUS_ABNORMAL, 7), 7),
            Err(MSG_NOT_PENDING.to_string())
        );
    }

    #[test]
    fn restore_validation() {
        assert!(validate_restore(&record(STATUS_ABNORMAL, 7)).is_ok());
        assert!(validate_restore(&record(STATUS_VOID, 7)).is_ok());
        assert!(validate_restore(&record(STATUS_PENDING, 7)).is_err());
        assert!(validate_restore(&record(STATUS_CLAIMED, 7)).is_err());
    }

    #[test]
    fn columns_match_entity() {
        // 列数与建表 SQL / 实体字段数对齐（20 列）
        assert_eq!(COLUMNS.len(), 20);
        assert_eq!(COLUMNS[0], KEY_COLUMN);
    }
}
