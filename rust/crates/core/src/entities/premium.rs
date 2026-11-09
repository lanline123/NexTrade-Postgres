//! 付费会员卡域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `premium_member_card`, `premium_member_card_right`, `premium_member_card_tier`, `premium_member_decorate`, `premium_member_order`, `premium_member_right`, `premium_member_right_rule`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 付费会员卡
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberCard {
    /// 付费会员卡ID [NOT NULL]
    pub card_id: i32,
    /// 卡名称 [NOT NULL]
    pub card_name: String,
    /// 会员须知 [NOT NULL]
    pub member_notice: String,
    /// 领卡条件[JSON]
    pub member_scope: Option<String>,
    /// 卡主题色 [NOT NULL]
    pub card_face_color: String,
    /// 卡面图片(完整URL) [NOT NULL]
    pub card_face_image: String,
    /// [NOT NULL]
    pub trial_enabled: i32,
    /// [NOT NULL]
    pub trial_price: Decimal,
    /// [NOT NULL]
    pub trial_days: i32,
    /// [NOT NULL]
    pub trial_limit: i32,
    /// [NOT NULL]
    pub renew_remind_enabled: i32,
    /// [NOT NULL]
    pub renew_remind_days: i32,
    /// 支付方式：online在线支付、balance余额支付，逗号分隔 [NOT NULL]
    pub pay_types: String,
    /// [NOT NULL]
    pub reward_points: i32,
    /// [NOT NULL]
    pub reward_balance: Decimal,
    /// [NOT NULL]
    pub reward_coupon_ids: String,
    /// [NOT NULL]
    pub reward_repeat: i32,
    /// [NOT NULL]
    pub share_mini_title: String,
    /// [NOT NULL]
    pub share_mini_desc: String,
    /// [NOT NULL]
    pub share_mini_image: String,
    /// [NOT NULL]
    pub share_h5_title: String,
    /// [NOT NULL]
    pub share_h5_desc: String,
    /// [NOT NULL]
    pub share_h5_image: String,
    /// [NOT NULL]
    pub status: i32,
    /// 用户端是否展示：0否1是 [NOT NULL]
    pub is_show: i32,
    /// [NOT NULL]
    pub is_delete: i32,
    /// [NOT NULL]
    pub sort_order: i32,
    /// [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub update_time: i32,
}

impl PremiumMemberCard {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_card";
}


/// 付费会员卡权益关联
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberCardRight {
    /// [NOT NULL]
    pub id: i32,
    /// [NOT NULL]
    pub card_id: i32,
    /// [NOT NULL]
    pub right_id: i32,
    /// 所选权益规则ID
    pub rule_id: Option<i32>,
    /// [NOT NULL]
    pub right_value: String,
    /// [NOT NULL]
    pub sort_order: i32,
}

impl PremiumMemberCardRight {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_card_right";
}


/// 付费会员卡档位
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberCardTier {
    /// [NOT NULL]
    pub tier_id: i32,
    /// [NOT NULL]
    pub card_id: i32,
    /// [NOT NULL]
    pub duration: i32,
    /// [NOT NULL]
    pub duration_unit: i32,
    /// 支付金额 [NOT NULL]
    pub price: Decimal,
    /// 续费优惠值（基于支付金额的优惠，非成交价）；0=不优惠 [NOT NULL]
    pub renew_price: Decimal,
    /// 续费优惠类型：1=元减免，2=折（折数/10） [NOT NULL]
    pub renew_price_type: i32,
    /// [NOT NULL]
    pub is_recommend: i32,
    /// [NOT NULL]
    pub sort_order: i32,
}

impl PremiumMemberCardTier {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_card_tier";
}


/// 付费会员装修配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberDecorate {
    /// [NOT NULL]
    pub id: i32,
    /// [NOT NULL]
    pub card_id: i32,
    /// [NOT NULL]
    pub hide_card_no: i32,
    /// [NOT NULL]
    pub hide_qrcode: i32,
    /// [NOT NULL]
    pub hide_validity: i32,
    /// [NOT NULL]
    pub rights_display_type: i32,
    pub rights_display_config: Option<String>,
    pub promo_config: Option<String>,
    /// [NOT NULL]
    pub update_time: i32,
}

impl PremiumMemberDecorate {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_decorate";
}


/// 付费会员订单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberOrder {
    /// [NOT NULL]
    pub order_id: i32,
    /// [NOT NULL]
    pub order_sn: String,
    /// [NOT NULL]
    pub user_id: i32,
    /// [NOT NULL]
    pub card_id: i32,
    /// [NOT NULL]
    pub tier_id: i32,
    /// [NOT NULL]
    pub card_name_snapshot: String,
    /// [NOT NULL]
    pub duration: i32,
    /// [NOT NULL]
    pub duration_unit: i32,
    /// [NOT NULL]
    pub amount: Decimal,
    /// [NOT NULL]
    pub buy_type: i32,
    /// [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub pay_sn: String,
    /// [NOT NULL]
    pub pay_code: String,
    /// 本订单已使用的余额金额 [NOT NULL]
    pub balance_amount: Decimal,
    /// 本订单剩余待支付金额
    pub unpaid_amount: Option<Decimal>,
    /// [NOT NULL]
    pub reward_granted: i32,
    /// 领卡资料快照JSON
    pub claim_profile: Option<String>,
    /// 是否同意协议 [NOT NULL]
    pub agreement_accepted: i32,
    /// 协议版本快照 [NOT NULL]
    pub agreement_version: String,
    /// [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub paid_time: i32,
}

impl PremiumMemberOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_order";
}


/// 付费会员权益中心
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberRight {
    /// [NOT NULL]
    pub right_id: i32,
    /// [NOT NULL]
    pub right_type: i32,
    /// [NOT NULL]
    pub right_key: String,
    /// [NOT NULL]
    pub show_name: String,
    /// [NOT NULL]
    pub icon: String,
    /// [NOT NULL]
    pub description: String,
    /// [NOT NULL]
    pub is_builtin: i32,
    /// [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub sort_order: i32,
    /// [NOT NULL]
    pub add_time: i32,
}

impl PremiumMemberRight {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_right";
}


/// 付费会员权益规则
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PremiumMemberRightRule {
    /// [NOT NULL]
    pub rule_id: i32,
    /// [NOT NULL]
    pub right_id: i32,
    /// [NOT NULL]
    pub rule_name: String,
    /// [NOT NULL]
    pub rule_value: String,
    /// [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub sort_order: i32,
    /// [NOT NULL]
    pub add_time: i32,
}

impl PremiumMemberRightRule {
    /// 数据库表名。
    pub const TABLE: &'static str = "premium_member_right_rule";
}