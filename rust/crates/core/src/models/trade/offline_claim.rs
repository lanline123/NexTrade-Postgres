//! 线下支付认款 DTO / VO（v5.8.61）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OfflinePaymentClaimSubmitDTO {
    pub order_id: Option<i32>,
    pub transfer_time: Option<i64>,
    pub payer_name: Option<String>,
    pub payer_account: Option<String>,
    pub bank_serial_no: Option<String>,
    pub voucher_urls: Option<Vec<String>>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OfflinePaymentClaimAuditDTO {
    pub claim_id: Option<i32>,
    pub pass: Option<bool>,
    pub audit_remark: Option<String>,
    pub reject_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflinePaymentClaimVO {
    pub claim_id: i32,
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub shop_id: i32,
    pub claimed_amount: Decimal,
    pub pay_stage: i32,
    pub pay_stage_name: String,
    pub transfer_time: i64,
    pub payer_name: String,
    pub payer_account: String,
    pub bank_serial_no: String,
    pub voucher_urls: Vec<String>,
    pub remark: String,
    pub status: i32,
    pub status_name: String,
    pub review_admin_id: i32,
    pub review_time: i64,
    pub paid_time: i64,
    pub audit_remark: String,
    pub reject_reason: String,
    pub add_time: i64,
    pub update_time: i64,
}

pub fn stage_name(stage: i32) -> String {
    match stage {
        1 => "定金".into(),
        2 => "尾款".into(),
        _ => "全款".into(),
    }
}

pub fn status_name(status: i32) -> String {
    match status {
        1 => "审核通过".into(),
        2 => "已驳回".into(),
        _ => "待审核".into(),
    }
}
