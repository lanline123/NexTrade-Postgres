//! 订单 DTO，对应原 Java `dto/order`、`query/order`、`vo/order`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 随单发票提交数据。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InvoiceDataDTO {
    /// 发票类型：1-普通发票 2-增值税专用发票
    pub invoice_type: Option<i32>,
    /// 抬头类型：1-个人 2-单位
    pub title_type: Option<i32>,
    /// 发票抬头
    pub title: Option<String>,
    /// 纳税人识别号
    pub tax_no: Option<String>,
    /// 发票内容
    pub content: Option<String>,
    /// 单位电话
    pub tax_phone: Option<String>,
    /// 个人或单位名称
    pub company_name: Option<String>,
    /// 收票人邮箱
    pub email: Option<String>,
    /// 收票人手机号
    pub mobile: Option<String>,
}

/// 创建订单请求（提交结算页数据）。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderCreateDTO {
    /// 购物车行ID列表
    pub cart_ids: Vec<i32>,
    /// 收货地址ID；自提时可空
    pub address_id: Option<i32>,
    /// 配送方式：1-快递 2-自提 3-同城配送
    pub shipping_method: Option<i32>,
    /// 兼容前端配送类型字段名
    #[serde(default, alias = "deliveryType")]
    pub delivery_type: Option<i32>,
    /// 自提点ID（自提时有效）
    #[serde(default)]
    pub pickup_id: Option<i32>,
    /// 自提联系人姓名
    #[serde(default)]
    pub pickup_contact_name: Option<String>,
    /// 自提联系人手机号
    #[serde(default)]
    pub pickup_contact_mobile: Option<String>,
    /// 使用优惠券ID
    pub coupon_id: Option<i32>,
    /// 使用积分（前端结算表单字段名为 `usePoint`）
    #[serde(default, alias = "usePoint")]
    pub use_points: Option<i32>,
    /// 买家备注
    pub buyer_note: Option<String>,
    /// 支付方式：对应 pay_type_id
    pub pay_type_id: Option<i32>,
    /// 使用余额金额（0/缺省 = 不使用；`true` 兼容旧前端 = 尽可能全额抵扣）
    #[serde(default, deserialize_with = "de_use_balance")]
    pub use_balance: Option<Decimal>,
    /// 礼品卡抵扣（单选一张，与 useGiftCardSn 二选一互斥，Sn 优先）
    #[serde(default)]
    pub use_gift_card_id: Option<i32>,
    #[serde(default)]
    pub use_gift_card_sn: Option<String>,
    /// 结算流类型（12 = 礼品卡兑换流；兑换覆盖由购物车行标记决定，此字段仅回显）
    #[serde(default)]
    pub flow_type: Option<i32>,
    /// 前端结算表单里的用户优惠券 ID 列表（`useCouponIds`）
    #[serde(default)]
    pub use_coupon_ids: Option<Vec<i32>>,
    /// 随单发票申请信息
    #[serde(default)]
    pub invoice_data: Option<InvoiceDataDTO>,
    /// 订单来源（如 "web", "代客下单" 等）
    #[serde(default)]
    pub order_source: Option<String>,
    /// POS/代客下单后台管理员ID
    #[serde(default)]
    pub pos_user_id: Option<i32>,
    /// 代客下单跨店履约的行价覆盖（我方新增）：cart_id → 履约门店单价。
    /// 仅代客跨店提交时由服务层按履约店分配价解析后注入；命中行以覆盖价
    /// 为最终计价单价、不叠加活动/满减。None/空 = 现行口径逐字不变。
    #[serde(skip)]
    pub line_price_overrides: Option<std::collections::HashMap<i32, Decimal>>,
    /// 送礼物模式（我方新增，论坛帖 197742）：true 时不填收货地址、
    /// 付款后订单进入待领取，好友领取时再落地址。缺省 = 普通订单。
    #[serde(default)]
    pub is_gift_order: Option<bool>,
    /// 赠言（送礼物模式，可选，上限 200 字）
    #[serde(default)]
    pub gift_message: Option<String>,
    /// 同城配送送达时段选择（我方新增，论坛帖 467744）：仅同城配送
    /// （shipping_method = 3）时有效；涉单店铺开启时段选择时必传，
    /// 服务端逐店校验（日期范围/时段命中/截单/容量）后落
    /// order_delivery_slot。缺省 = 未选择。
    #[serde(default)]
    pub delivery_time_slot: Option<super::delivery_slot::DeliverySlotSelection>,
    /// 预约时段 ID（预约商品结算提交，`product_reservation_time` 主键）。
    /// 与 `reservation_date` **成对出现**才触发预约接线（占用时段名额 +
    /// 写 `order_reservations`）；只有时段无日期是周期购/次卡预约的既有
    /// 传参形态，不在本接线范围。口径见 services `trade::reservation`。
    #[serde(default)]
    pub product_reservation_time_id: Option<i32>,
    /// 预约日期 `yyyy-MM-dd`（预约商品结算提交，与时段 ID 成对）。
    #[serde(default)]
    pub reservation_date: Option<String>,
    /// 下单幂等键（我方新增）：客户端为一次结算会话生成一个 UUID，
    /// 本次结算的全部提交（含弱网重试/重复点击）共用。服务端同
    /// (user_id, 键) 在窗口内只建一单，重复提交返回原单结果而非
    /// 报错。缺省/非法 = 不启用幂等（存量客户端行为逐字不变）。
    /// 口径见 `normalize_idempotency_key` 与 services `trade::order`。
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

/// 下单幂等键有效窗口（秒）：窗口内同 (user_id, key) 只建一单；
/// 窗口过后键可被新结算重新占用（登记行过期删除）。
pub const IDEMPOTENCY_WINDOW_SECONDS: i64 = 24 * 3600;

/// 归一幂等键：去首尾空白后长度 16..=64 且仅含 `[A-Za-z0-9_-]`
/// 才有效，其余（缺省/过短/过长/含非法字符）一律视为未传键——
/// 宁可不启用幂等，也绝不因怪键阻断正常下单。
#[must_use]
pub fn normalize_idempotency_key(raw: Option<&str>) -> Option<String> {
    let key = raw?.trim();
    if key.len() < 16 || key.len() > 64 {
        return None;
    }
    if !key
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    Some(key.to_string())
}

/// 兼容前端三种 `useBalance` 入参：数字（余额抵扣金额）、布尔、字符串数字。
fn de_use_balance<'de, D>(deserializer: D) -> std::result::Result<Option<Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Bool(true) => Ok(Some(Decimal::MAX)),
        serde_json::Value::Bool(false) => Ok(None),
        serde_json::Value::Number(n) => Ok(n.to_string().parse::<Decimal>().ok()),
        serde_json::Value::String(s) => Ok(s.trim().parse::<Decimal>().ok()),
        _ => Ok(None),
    }
}

impl Default for OrderCreateDTO {
    fn default() -> Self {
        Self {
            cart_ids: Vec::new(),
            address_id: None,
            shipping_method: Some(1),
            delivery_type: None,
            pickup_id: None,
            pickup_contact_name: None,
            pickup_contact_mobile: None,
            coupon_id: None,
            use_points: None,
            buyer_note: None,
            pay_type_id: None,
            use_balance: None,
            use_gift_card_id: None,
            use_gift_card_sn: None,
            flow_type: None,
            use_coupon_ids: None,
            invoice_data: None,
            order_source: None,
            pos_user_id: None,
            line_price_overrides: None,
            is_gift_order: None,
            gift_message: None,
            delivery_time_slot: None,
            product_reservation_time_id: None,
            reservation_date: None,
            idempotency_key: None,
        }
    }
}

/// 创建订单结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderCreateVO {
    pub order_id: i32,
    pub order_sn: String,
    /// 应付总额
    pub total_amount: Decimal,
    /// 需在线支付金额
    pub online_amount: Decimal,
    /// 支付流水号（预支付用）
    pub out_trade_no: String,
    /// 1=需继续支付（跳支付页）；2=无需支付（余额已全额抵扣，前端直接跳订单列表）
    #[serde(default = "default_return_type")]
    pub return_type: i32,
    /// 拼团记录 ID（下单成功时回填）
    #[serde(default)]
    pub groupon_promotion_record_id: Option<i32>,
    /// 余额支付的金额
    #[serde(default)]
    pub balance: Option<Decimal>,
    /// 是否为次卡兑换的 0 元子订单
    #[serde(default)]
    pub is_card_redeem: Option<bool>,
    /// 送礼物订单的领取 token 明文（仅建单时一次性返回；库内只存摘要，
    /// 后续分享走「重新生成」接口）
    #[serde(default)]
    pub gift_token: Option<String>,
    /// 本次响应是否为幂等重放（我方新增）：同键重复提交命中原单时
    /// 为 true，首次建单恒 false。供服务端内部链路（POS/代客下单
    /// 据此跳过重复后置动作）与排障使用，前端可忽略。
    #[serde(default)]
    pub idempotent_replay: bool,
}

fn default_return_type() -> i32 {
    1
}

/// 订单列表项 VO。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderVO {
    pub order_id: i32,
    pub order_sn: String,
    pub shop_id: i32,
    pub order_status: i32,
    /// 原版前端订单卡读取 `orderStatusName`（不能只给 statusLabel）
    pub order_status_name: String,
    pub status_label: String,
    pub pay_status: i32,
    pub shipping_status: i32,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub consignee: String,
    pub mobile: String,
    pub add_time: String,
    pub item_count: i32,
    /// 订单商品明细（前端订单卡需要渲染 items）
    pub items: Vec<serde_json::Value>,
    /// 订单可操作项（前端直接访问 availableActions.toPay 等）
    pub available_actions: serde_json::Value,
    pub cancel_remark: Option<String>,
    pub order_type: Option<i32>,
    pub parent_order_sn: Option<String>,
    pub is_card_order: bool,
    /// 周期购标记（旧前端读 `Number(isCycleOrder) !== 1`，必须下发数字 0/1 而非布尔）
    pub is_cycle_order: i32,
    pub is_pickup: bool,
    pub presale_info: Option<serde_json::Value>,
    pub order_source: Option<String>,
    pub cycle_card_info: Option<serde_json::Value>,
    /// 发票申请数据（发票列表页读取 `invoiceData`）
    pub invoice_data: Option<serde_json::Value>,
    /// 订单状态提示文案（Java `orderStatusTip`，PC 订单详情无倒计时时展示）
    pub order_status_tip: String,
    /// 订单支付时间（未支付为 null）
    pub pay_time: Option<String>,
    /// 订单发货时间（未发货为 null）
    pub shipping_time: Option<String>,
    /// 订单收货时间（未收货为 null）
    pub received_time: Option<String>,
    /// 买家备注（PC 订单详情展示）
    pub buyer_note: String,
    /// 线上支付金额（PC 订单金额明细）
    pub online_paid_amount: Decimal,
    /// 线下支付金额（PC 订单金额明细）
    pub offline_paid_amount: Decimal,
    /// 账期支付金额（PC 订单金额明细）
    pub credit_amount: Decimal,
    /// 发票费用
    pub invoice_fee: Decimal,
    pub balance: Decimal,
    pub shipping_fee: Decimal,
    // ————— 详情页需要的扩展字段 —————
    pub pay_status_name: String,
    pub shipping_status_name: String,
    pub pay_end_time: Option<String>,
    pub shipping_type_name: String,
    pub tracking_no: String,
    pub logistics_name: String,
    pub user_address: String,
    pub pay_type_id: i32,
    pub product_amount: Decimal,
    pub coupon_amount: Decimal,
    pub points_amount: Decimal,
    pub discount_amount: Decimal,
    pub service_fee: Decimal,
    pub unpaid_amount: Decimal,
    pub is_presale: bool,
    pub auto_delivery_days: Option<i32>,
    pub join_status: Option<i32>,
    pub join_status_desc: Option<String>,
    pub after_sales_ing: bool,
    pub pickup_detail: Option<serde_json::Value>,
    pub delivery_order_info: Option<serde_json::Value>,
    pub order_reservation: Option<serde_json::Value>,
    pub is_reservation_home: bool,
    pub shop: Option<serde_json::Value>,
    // ————— 线下支付认款（v5.8.61 补下发） —————
    /// 最新一笔线下认款状态：0 待审核 / 1 审核通过 / 2 已驳回；无认款为 null
    pub offline_payment_claim_status: Option<i32>,
    /// 认款状态文案（待审核 / 审核通过 / 审核驳回）
    pub offline_payment_claim_status_name: Option<String>,
    /// 最新一笔认款单 ID；无认款为 null
    pub offline_payment_claim_id: Option<i32>,
    /// 最新一笔认款的驳回原因（未驳回为 null）
    pub offline_payment_claim_reject_reason: Option<String>,
    /// 最新一笔线下认款单全量；无认款为 null
    pub offline_payment_claim: Option<super::offline_claim::OfflinePaymentClaimVO>,
    /// 买家取消直退进度（我方新增）：该订单最新一笔买家取消退款
    /// 申请的状态（0 待处理 / 1 处理中 / 2 已处理 / 3 已拒绝 /
    /// 4 已退回）；无申请时为 None。由服务层在买家列表/详情
    /// 后置附加（`to_vo` 本体恒 None）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_cancel_refund_status: Option<i32>,
    /// 买家取消直退进度展示名（待审核 / 退款中 / 已退款 /
    /// 已拒绝 / 已退回）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_cancel_refund_status_name: Option<String>,
    // ————— 同城送达时段（我方新增，论坛帖 467744） —————
    /// 送达时段开始 Unix 秒（服务层在列表/详情后置附加，`to_vo`
    /// 本体恒 None；未选时段为 null）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_time_start: Option<i32>,
    /// 送达时段结束 Unix 秒
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_time_end: Option<i32>,
    /// 送达时段展示文案（`yyyy-MM-dd HH:mm-HH:mm`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_time_text: Option<String>,
    // ————— 送礼物订单（我方新增） —————
    /// 是否送礼物订单（根订单或其拆单子单命中 order_gift 即为 true）
    pub is_gift_order: bool,
    /// 礼物子状态（见 models::trade::gift_order::gift_status）；非礼物订单为 null
    pub gift_status: Option<i32>,
    /// 礼物详情（仅订单详情接口填充：有效期/领取人脱敏/关闭原因等）
    pub gift_info: Option<super::gift_order::GiftInfoVO>,
}

#[cfg(test)]
mod idempotency_key_tests {
    use super::normalize_idempotency_key;

    #[test]
    fn uuid_shaped_key_is_accepted() {
        let key = "3f8a2c1e-9b4d-4e5f-a6b7-c8d9e0f1a2b3";
        assert_eq!(
            normalize_idempotency_key(Some(key)).as_deref(),
            Some(key)
        );
        // 首尾空白被裁掉
        assert_eq!(
            normalize_idempotency_key(Some("  3f8a2c1e-9b4d-4e5f-a6b7-c8d9e0f1a2b3  ")).as_deref(),
            Some(key)
        );
        // 下划线合法
        assert_eq!(
            normalize_idempotency_key(Some("checkout_20261121_abcd")).as_deref(),
            Some("checkout_20261121_abcd")
        );
    }

    #[test]
    fn missing_or_malformed_key_means_no_idempotency() {
        assert_eq!(normalize_idempotency_key(None), None);
        assert_eq!(normalize_idempotency_key(Some("")), None);
        assert_eq!(normalize_idempotency_key(Some("   ")), None);
        // 过短（< 16）
        assert_eq!(normalize_idempotency_key(Some("abc123")), None);
        // 过长（> 64）
        assert_eq!(normalize_idempotency_key(Some(&"a".repeat(65))), None);
        // 非法字符（空格/中文/斜杠）
        assert_eq!(normalize_idempotency_key(Some("abcd efgh ijkl mnop")), None);
        assert_eq!(normalize_idempotency_key(Some("下单幂等键下单幂等键下单幂等")), None);
        assert_eq!(normalize_idempotency_key(Some("abcd/efgh/ijkl/mnop")), None);
    }

    #[test]
    fn dto_without_key_still_deserializes() {
        let dto: super::OrderCreateDTO =
            serde_json::from_str(r#"{"cartIds":[1]}"#).expect("deserialize");
        assert_eq!(dto.idempotency_key, None);
        let dto: super::OrderCreateDTO = serde_json::from_str(
            r#"{"cartIds":[1],"idempotencyKey":"3f8a2c1e-9b4d-4e5f-a6b7-c8d9e0f1a2b3"}"#,
        )
        .expect("deserialize");
        assert_eq!(
            dto.idempotency_key.as_deref(),
            Some("3f8a2c1e-9b4d-4e5f-a6b7-c8d9e0f1a2b3")
        );
    }
}
