//! 送礼物订单流 DTO / VO（我方新增，论坛帖 197742，用户 2026-11-21 拍板）。
//!
//! 形态：买家结算选「送礼物」→ 不填收货地址、正常付款 → 订单进入
//! 「待领取」→ 买家分享领取链接 → 好友登录填自己的地址领取 →
//! 订单落入正常履约链；超时未领取自动原路退款给买家。
//! 礼物关系独立落 `order_gift` 表（迁移 014），不侵入订单状态枚举。

use serde::{Deserialize, Serialize};

/// 礼物子状态（`order_gift.gift_status`）。
pub mod gift_status {
    /// 已建单未付款（付款成功后转待领取）
    pub const PENDING_PAYMENT: i32 = 0;
    /// 已付款、待好友领取
    pub const PENDING_CLAIM: i32 = 1;
    /// 已领取（地址已落入订单，进入正常履约）
    pub const CLAIMED: i32 = 2;
    /// 已关闭（领取前取消 / 超时未领取，均已走退款链）
    pub const CLOSED_REFUNDED: i32 = 3;
    /// 未付款时买家取消
    pub const CANCELED_UNPAID: i32 = 4;
}

/// 礼物状态文案。
#[must_use]
pub fn gift_status_name(status: i32) -> &'static str {
    match status {
        gift_status::PENDING_PAYMENT => "待付款",
        gift_status::PENDING_CLAIM => "待领取",
        gift_status::CLAIMED => "已领取",
        gift_status::CLOSED_REFUNDED => "已关闭",
        gift_status::CANCELED_UNPAID => "已取消",
        _ => "",
    }
}

/// 领取页预览请求（按 token；token 为不可猜随机串，库内只存摘要）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftClaimPreviewDTO {
    pub token: String,
}

/// 领取提交请求（必须登录；地址取领取人自己的地址簿）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftClaimDTO {
    pub token: String,
    pub address_id: i32,
}

/// 领取页 / 领取人视图的商品行（**刻意不含任何价格字段**：送礼语义，
/// 领取人不得见付款金额；单价/总价/运费一律不下发）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftItemVO {
    pub item_id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub pic_thumb: String,
    pub sku_data: String,
    pub quantity: i32,
}

/// 领取页预览 VO（匿名可看概要；领取动作必须登录）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftClaimPreviewVO {
    /// 礼物子状态
    pub gift_status: i32,
    pub gift_status_name: String,
    /// 赠言（买家下单时填写）
    pub gift_message: String,
    /// 送礼人昵称（脱敏：首字 + ***）
    pub buyer_name_masked: String,
    /// 领取截止时间（秒级时间戳字符串，空 = 未付款尚未起算）
    pub expire_time: Option<String>,
    /// 是否已过领取期
    pub expired: bool,
    /// 当前访问者是否就是领取人本人视角可领取（已登录且非买家本人）
    pub can_claim: bool,
    /// 商品行（无价格）
    pub items: Vec<GiftItemVO>,
    /// 已领取时下发：领取落入的履约订单（子单）概要，供领取人跟踪/售后
    pub units: Vec<GiftReceivedUnitVO>,
}

/// 买家侧礼物信息 VO（订单详情礼物面板）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftInfoVO {
    pub order_id: i32,
    pub gift_status: i32,
    pub gift_status_name: String,
    pub gift_message: String,
    /// 领取截止时间（秒级时间戳字符串）
    pub expire_time: Option<String>,
    /// 领取人昵称（脱敏；未领取为 null）
    pub claim_user_name_masked: Option<String>,
    /// 领取时间（秒级时间戳字符串）
    pub claim_time: Option<String>,
    /// 关闭原因（超时/取消时落字）
    pub close_reason: String,
}

/// 分享 token 生成结果（明文仅此一次返回；库内只存摘要，
/// 重新生成即作废旧链接）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftShareTokenVO {
    pub token: String,
    /// 前台领取页相对路径（H5 与 PC 同路径约定 `/gift/claim?token=`）
    pub claim_path: String,
}

/// 「我收到的礼物」单条（领取人视角，**无价格字段**）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftReceivedVO {
    pub order_id: i32,
    pub order_sn: String,
    pub gift_status: i32,
    pub gift_status_name: String,
    pub gift_message: String,
    pub buyer_name_masked: String,
    pub claim_time: Option<String>,
    /// 履约单元（跨店拆单时为各子单；未拆单时为根订单自身）
    pub units: Vec<GiftReceivedUnitVO>,
}

/// 领取人的履约单元概要（无价格）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftReceivedUnitVO {
    pub order_id: i32,
    pub order_sn: String,
    pub shop_id: i32,
    pub order_status: i32,
    pub order_status_name: String,
    pub tracking_no: String,
    pub logistics_name: String,
    pub items: Vec<GiftItemVO>,
}

/// 平台礼物设置（存 config 表 `giftOrderSettings` 一行 JSON）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GiftOrderSettings {
    /// 领取有效期（天），默认 7，合法区间 1..=90
    pub claim_expire_days: i32,
}

impl Default for GiftOrderSettings {
    fn default() -> Self {
        Self {
            claim_expire_days: 7,
        }
    }
}

impl GiftOrderSettings {
    /// 清洗外部输入：天数钳制到 1..=90，非法/缺失回默认 7。
    #[must_use]
    pub fn sanitized(value: &serde_json::Value) -> Self {
        let days = value
            .get("claimExpireDays")
            .and_then(serde_json::Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| (1..=90).contains(v))
            .unwrap_or(7);
        Self {
            claim_expire_days: days,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_sanitized_clamps_days() {
        assert_eq!(
            GiftOrderSettings::sanitized(&serde_json::json!({"claimExpireDays": 3}))
                .claim_expire_days,
            3
        );
        assert_eq!(
            GiftOrderSettings::sanitized(&serde_json::json!({"claimExpireDays": 0}))
                .claim_expire_days,
            7
        );
        assert_eq!(
            GiftOrderSettings::sanitized(&serde_json::json!({"claimExpireDays": 365}))
                .claim_expire_days,
            7
        );
        assert_eq!(
            GiftOrderSettings::sanitized(&serde_json::json!({})).claim_expire_days,
            7
        );
    }

    #[test]
    fn gift_item_vo_has_no_price_fields() {
        let v = serde_json::to_value(GiftItemVO::default()).unwrap();
        let obj = v.as_object().unwrap();
        for key in obj.keys() {
            assert!(
                !key.to_lowercase().contains("price")
                    && !key.to_lowercase().contains("amount"),
                "领取人视图不得下发价格字段: {key}"
            );
        }
    }
}
