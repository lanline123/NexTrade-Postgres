//! 前台用户售后服务（trade::aftersales_client）：售后列表、售后配置、售后申请数据、
//! 售后申请、申请修改、申请记录、售后详情、售后日志、回寄反馈、取消申请。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.user.AfterSalesController`（10 个端点）
//! - `com.nextrade.service.order.impl.AftersalesServiceImpl`
//!   （`list` / `afterSalesApply` / `afterSalesApplyUpdate` / `clientDetail` /
//!   `submitFeedbackRecord` / `cancel` / `validateOrderQuantity` /
//!   `calculateSuggestRefundAmount` / `calculateFinalRefundAmount` /
//!   `calculateBaseRefundAmount` / `calcRefundProductAmount` / `getStep` /
//!   `resolveAutoAgreeEndTime`）
//! - `com.nextrade.service.order.impl.OrderServiceImpl`
//!   （`afterSalesList` / `applyData`）
//! - `com.nextrade.service.order.impl.AftersalesLogServiceImpl#getDetailLog`
//! - `com.nextrade.service.order.impl.AftersalesItemServiceImpl#getItemByIds`
//! - `com.nextrade.service.setting.AfterSalesReasonConfigService`
//! - `com.nextrade.helper.SettingsHelper`（`config` 表读取）
//!
//! **未迁移（本切片明确不做，已在 `uncertain_points` 中列明）**：
//! - 售后审核开关 `afterSalesReview` **开启**时的完整链路：后台消息中心、
//!   微信小程序订单同步、账期订单置「退款中」、RabbitMQ 自动同意退货延时消息
//!   与自动确认收货延时消息。**但「关闭」时的自动审核结果
//!   （仅退款 → 审核通过 / 退货退款 → 待回寄）按 Java 逻辑复刻**，
//!   见 [`AftersalesClientService::create`]。
//! - 退款执行 / 退款申请（`RefundApply`）：本域只写售后单，不发起实际退款；
//!   `refund_amount` 的「运费是否已退过」（Java `isShippingAlreadyRefunded`）
//!   需要 `refund_apply` 表数据，见该函数注释。
//! - 多语言翻译（`translatePackage` / `MessageHelper`）：文案取
//!   `java/nextrade-common/src/main/resources/i18n/messages.properties` 的
//!   `aftersales.*` / `error.*` 静态中文，逐字符对齐。
//! - 拼团团长售后规则（Java `validateGrouponCaptainRefundRule`）与
//!   「已超售后期限」（`afterSalesLimitDays`）为**配置/依赖未齐**，
//!   见 `uncertain_points`，未实现。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use rust_decimal::prelude::Zero;
use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_common::utils;
use nextrade_core::entities::{Aftersales, AftersalesItem, AftersalesLog, Order, OrderItem, Product};
use nextrade_core::models::trade::aftersales_client::{
    aftersales_status, aftersales_type, client_status_desc, client_status_name, type_name,
    AfterSalesApplyDataVO, AfterSalesClientDetailVO, AfterSalesClientItemVO, AfterSalesClientVO,
    AfterSalesDetailVO, AftersalesApplyItem, AftersalesConfigVO, AftersalesFeedbackParam,
    AftersalesItemVO, AftersalesListQuery, AftersalesLogVO, AftersalesVO,
    AftersalesCancelParam, ClientAfterSalesUpdateParam, CreateAftersalesParam, SkuDataVO, StepStatusVO,
    StepVO, ERR_AFTERSALES_NOT_EXIST, ERR_AFTERSALES_TIME_EXPIRED, ERR_APPLY_NOT_EXIST,
    ERR_FULL_REFUND_ONLY, ERR_GIFTCARD_NOT_ALLOWED, ERR_ORDER_ITEM_NOT_EXIST, ERR_ORDER_NOT_EXIST,
    ERR_POS_NOT_ALLOWED, ERR_PRODUCT_HAS_APPLIED, ERR_QUANTITY_INSUFFICIENT,
    ERR_QUANTITY_REQUIRED, ERR_QUANTITY_ZERO, ERR_RECORD_NOT_EXIST, ERR_STATUS_CANNOT_CANCEL,
    ERR_STATUS_CANNOT_MODIFY, ERR_UNPAID_NOT_ALLOWED, STEP_DESC_APPROVED, STEP_DESC_FINANCE_REVIEWING,
    STEP_DESC_REJECTED, STEP_DESC_REVIEWING, STEP_DESC_WAIT_MERCHANT_RECEIPT,
    STEP_DESC_WAIT_RETURN, STEP_DESC_WAIT_SUPPLIER_RECEIPT, STEP_DESC_WAIT_SUPPLIER_REVIEW,
    STEP_STATUS_FINISH, STEP_STATUS_PROCESS, STEP_TITLE_CANCELLED, STEP_TITLE_COMPLETED,
    STEP_TITLE_PROCESSING, STEP_TITLE_REVIEWING, STEP_TITLE_SUBMIT,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::aftersales_client::AftersalesLogRow;
use nextrade_infrastructure::db::Repositories;

// ————————————————————— 配置项（Java `SettingsEnum`） —————————————————————

/// 全局业务错误码（Java `HttpStatus.SERVICE_DATA_ERROR`）。
///
/// Java 侧 `throw new GlobalException(msg)` 的默认 `code` 即该值，由
/// `GlobalExceptionHandler` 原样放进 `ResultWrapper`（HTTP 200 + 业务码）。
const SERVICE_DATA_ERROR: i32 = 1001;
/// `afterSalesReview`（Java 默认 `"1"`）。
const CFG_AFTER_SALES_REVIEW: &str = "afterSalesReview";
/// `afterSalesReasonEnabled`（Java 默认 `"1"`）。
const CFG_AFTER_SALES_REASON_ENABLED: &str = "afterSalesReasonEnabled";
/// `afterSalesReasonList`（Java 默认见 [`DEFAULT_REASON_LIST`]）。
const CFG_AFTER_SALES_REASON_LIST: &str = "afterSalesReasonList";
/// `autoReturnGoods`（Java 默认 `null` → `"0"`）。
const CFG_AUTO_RETURN_GOODS: &str = "autoReturnGoods";
/// `autoReturnGoodsDays`（Java 默认 `null`）。
const CFG_AUTO_RETURN_GOODS_DAYS: &str = "autoReturnGoodsDays";

/// 售后原因默认列表（Java `SettingsEnum.AFTER_SALES_REASON_LIST` 的默认值）。
pub const DEFAULT_REASON_LIST: [&str; 5] = [
    "多拍/拍错/不喜欢",
    "未按约定时间发货",
    "协商一致退款",
    "地址/电话填错了",
    "其他",
];

/// 原因条数上限（Java `AfterSalesReasonConfigService.MAX_REASON_COUNT`）。
const MAX_REASON_COUNT: usize = 10;
/// 原因长度上限（Java `MAX_REASON_LENGTH`）。
const MAX_REASON_LENGTH: usize = 50;

/// 订单状态：已确认，待发货（Java `OrderStatusEnum.CONFIRMED`）。
const ORDER_STATUS_CONFIRMED: i32 = 1;
/// 订单状态：处理中（Java `OrderStatusEnum.PROCESSING`）。
const ORDER_STATUS_PROCESSING: i32 = 2;
/// 订单状态：已完成（Java `OrderStatusEnum.COMPLETED`）。
const ORDER_STATUS_COMPLETED: i32 = 5;

/// 支付状态：已支付（Java `PaymentStatus.PAID`）。
const PAY_STATUS_PAID: i32 = 2;
/// 发货状态：未发货（Java `ShippingStatusEnum.PENDING`）。
const SHIPPING_STATUS_PENDING: i32 = 0;
/// 发货状态：已发货（Java `ShippingStatusEnum.SHIPPED`）。
const SHIPPING_STATUS_SHIPPED: i32 = 1;

/// 商品类型：虚拟电子礼品卡（Java `ProductType.PRODUCT_TYPE_GIFT_CARD_VIRTUAL`）。
const PRODUCT_TYPE_GIFT_CARD_VIRTUAL: i32 = 9;

/// 回寄时可提交物流的售后状态（Java `submitFeedbackRecord` 的 `canSubmitReturnLogistics`）。
fn can_submit_return_logistics(status: i32) -> bool {
    matches!(
        status,
        aftersales_status::SEND_BACK
            | aftersales_status::RETURNED
            | aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
    )
}

/// 售后审核开关：是否开启（Java `"1".equals(configService.getConfigVal(AFTER_SALES_REVIEW))`）。
fn review_enabled(raw: Option<&str>) -> bool {
    raw.map(str::trim) == Some("1")
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`：
/// 0 或负数返回空串）。
#[must_use]
pub fn fmt_time(secs: i32) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 可选时间戳 → `Option<yyyy-MM-dd HH:mm:ss>`（0/负数 → None）。
#[must_use]
pub fn fmt_time_opt(secs: i32) -> Option<String> {
    (secs > 0).then(|| fmt_time(secs))
}

/// Java `TigUtils.handelTime(Long)`：毫秒时间戳版本（日志 `add_time` 为秒）。
fn fmt_time_secs_i64(secs: i64) -> String {
    if secs <= 0 {
        return String::new();
    }
    i32::try_from(secs).map_or_else(|_| String::new(), fmt_time)
}

/// 判定「售后进行中」的订单项是否已有申请占用。
///
/// 对应 Java `OrderServiceImpl#handleSumValidNum`：`quantity - 已申请数量`。
#[must_use]
pub fn can_apply_quantity(quantity: i32, applied: i32) -> i32 {
    quantity.saturating_sub(applied).max(0)
}

/// 判定订单项是否可申请售后（Java `OrderServiceImpl#getAfterSalesItem`）。
///
/// `aftersale_status` = 该订单项最新一条「进行中」售后明细所属售后单的状态。
/// - 无进行中售后 → 可申请（1）
/// - 售后已完成（6）：额度用尽（`can_apply == 0`）则不可申请（0），否则可申请（1）
/// - 售后已取消（7）/ 审核未通过（3）：额度已释放 → 可申请（1）
/// - 其他进行中状态 → 不可申请（0）
#[must_use]
pub fn aftersale_flag(aftersale_status: Option<i32>, can_apply: i32) -> i32 {
    match aftersale_status {
        None => 1,
        Some(aftersales_status::COMPLETE) => i32::from(can_apply != 0),
        Some(aftersales_status::CANCEL | aftersales_status::REFUSE) => 1,
        Some(_) => 0,
    }
}

/// 乘并向上取整到 2 位小数（Java `setScale(2, RoundingMode.CEILING)`）。
#[must_use]
pub fn mul_ceil2(a: Decimal, b: i32) -> Decimal {
    (a * Decimal::from(b)).round_dp_with_strategy(2, RoundingStrategy::ToPositiveInfinity)
}

/// 单件税费（Java：`price > 0 且 taxRate > 0` 时
/// `price * taxRate / 100`，2 位向上取整，否则 0）。
#[must_use]
pub fn unit_tax_amount(price: Decimal, tax_rate: Decimal) -> Decimal {
    if price > Decimal::ZERO && tax_rate > Decimal::ZERO {
        (price * tax_rate / Decimal::from(100)).round_dp_with_strategy(2, RoundingStrategy::ToPositiveInfinity)
    } else {
        Decimal::ZERO
    }
}

/// 解析 `order_item.extra_sku_data` 并累加 `attrPrice`
/// （Java `calculateSuggestRefundAmount` / `calculateBaseRefundAmount`）。
#[must_use]
pub fn extra_sku_attr_price_sum(raw: Option<&str>) -> Decimal {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Decimal::ZERO;
    };
    let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw) else {
        return Decimal::ZERO;
    };
    arr.iter()
        .filter_map(|v| v.get("attrPrice"))
        .filter_map(json_decimal)
        .fold(Decimal::ZERO, |acc, v| acc + v)
}

/// JSON 值 → `Decimal`（兼容数字与数字字符串）。
fn json_decimal(v: &serde_json::Value) -> Option<Decimal> {
    match v {
        serde_json::Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        serde_json::Value::String(s) => Decimal::from_str(s.trim()).ok(),
        _ => None,
    }
}

/// 解析 `order_item.sku_data`（Java `JSON.parseArray(...)`；
/// `"[]"` 或非法时返回空表）。
#[must_use]
pub fn parse_sku_data(raw: &str) -> Vec<SkuDataVO> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Vec::new();
    }
    serde_json::from_str::<Vec<serde_json::Value>>(trimmed)
        .map(|arr| {
            arr.iter()
                .map(|v| SkuDataVO {
                    name: v
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    value: v
                        .get("value")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    attr_pic_thumb: v
                        .get("attrPicThumb")
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 规格值拼接（Java `Collectors.joining("|")`，`name:value`）。
#[must_use]
pub fn sku_value_of(sku_data: &[SkuDataVO]) -> String {
    sku_data
        .iter()
        .map(|s| format!("{}:{}", s.name, s.value))
        .collect::<Vec<_>>()
        .join("|")
}

/// 解析 `aftersales.pics`（Java `JSON.parseArray(pics, ReturnPicDTO.class)`）；
/// 空串 / 非法时下发 `[]`。
#[must_use]
pub fn parse_pics(raw: &str) -> Vec<serde_json::Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<serde_json::Value>>(trimmed).unwrap_or_default()
}

/// 解析日志 `return_pic`（Java：`"[]"` 或空 → 空表）。
#[must_use]
pub fn parse_return_pic(raw: Option<&str>) -> Vec<serde_json::Value> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty() && *s != "[]") else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<serde_json::Value>>(raw).unwrap_or_default()
}

/// 建议退款金额（Java `AftersalesServiceImpl#calculateSuggestRefundAmount`）。
///
/// - 逐条：`单价 (+ 附加规格)` × 申请数量；「已确认待发货」订单才叠加附加规格；
/// - 整单申请（明细条数与数量都完整覆盖）时改为 `订单总额 − 运费`。
#[must_use]
pub fn suggest_refund_amount(
    order: &Order,
    order_items: &[OrderItem],
    apply_items: &[AftersalesApplyItem],
) -> Decimal {
    let item_map: HashMap<i32, &OrderItem> =
        order_items.iter().map(|it| (it.item_id, it)).collect();

    let refund_amount = apply_items
        .iter()
        .map(|a| {
            let Some(oi) = a.order_item_id.and_then(|id| item_map.get(&id).copied()) else {
                return Decimal::ZERO;
            };
            let mut unit_price = oi.price;
            if order.order_status == ORDER_STATUS_CONFIRMED {
                unit_price += extra_sku_attr_price_sum(oi.extra_sku_data.as_deref());
            }
            unit_price * Decimal::from(a.number.unwrap_or(0))
        })
        .fold(Decimal::ZERO, |acc, v| acc + v);

    // 整单判定：明细条数一致，且每条数量完全等于购买数量
    let mut apply_qty: HashMap<i32, i32> = HashMap::new();
    for a in apply_items {
        if let Some(id) = a.order_item_id {
            *apply_qty.entry(id).or_insert(0) += a.number.unwrap_or(0);
        }
    }
    let is_full_order_refund = order_items.len() == apply_items.len()
        && order_items
            .iter()
            .all(|oi| apply_qty.get(&oi.item_id).copied() == Some(oi.quantity));

    if is_full_order_refund {
        order.total_amount - order.shipping_fee
    } else {
        refund_amount
    }
}

/// 基础退款额（Java `calculateBaseRefundAmount`）：按售后明细重算商品额，
/// 整单覆盖时改为 `订单总额 − 运费`。
#[must_use]
pub fn base_refund_amount(
    order: &Order,
    order_items: &[OrderItem],
    aftersales_items: &[AftersalesItem],
    fallback_refund_amount: Decimal,
) -> Decimal {
    if aftersales_items.is_empty() {
        return fallback_refund_amount;
    }
    let item_map: HashMap<i32, &OrderItem> =
        order_items.iter().map(|it| (it.item_id, it)).collect();

    let refund_amount = aftersales_items
        .iter()
        .map(|ai| {
            let Some(oi) = ai.order_item_id.and_then(|id| item_map.get(&id).copied()) else {
                return Decimal::ZERO;
            };
            let mut unit_price = oi.price;
            if order.order_status == ORDER_STATUS_CONFIRMED {
                unit_price += extra_sku_attr_price_sum(oi.extra_sku_data.as_deref());
            }
            unit_price * Decimal::from(ai.number.unwrap_or(0))
        })
        .fold(Decimal::ZERO, |acc, v| acc + v);

    let mut as_qty: HashMap<i32, i32> = HashMap::new();
    for ai in aftersales_items {
        if let Some(id) = ai.order_item_id {
            *as_qty.entry(id).or_insert(0) += ai.number.unwrap_or(0);
        }
    }
    let is_full_order_refund = order_items.len() == aftersales_items.len()
        && order_items
            .iter()
            .all(|oi| as_qty.get(&oi.item_id).copied() == Some(oi.quantity));

    if is_full_order_refund {
        order.total_amount - order.shipping_fee
    } else {
        refund_amount
    }
}

/// 退款商品金额（Java `calcRefundProductAmount`）：按
/// `(单价 + 附加规格) × 数量` 累加后 2 位向上取整。
#[must_use]
pub fn calc_refund_product_amount(order: &Order, items: &[AftersalesItemVO]) -> Decimal {
    let total = items
        .iter()
        .filter(|i| i.number.is_some())
        .map(|i| {
            let mut unit_price = i.price.unwrap_or(Decimal::ZERO);
            if order.order_status == ORDER_STATUS_CONFIRMED
                && i.extra_sku_data.as_deref().is_some_and(|s| !s.trim().is_empty())
            {
                unit_price += extra_sku_attr_price_sum(i.extra_sku_data.as_deref());
            }
            unit_price * Decimal::from(i.number.unwrap_or(0))
        })
        .fold(Decimal::ZERO, |acc, v| acc + v);
    total.round_dp_with_strategy(2, RoundingStrategy::ToPositiveInfinity)
}

/// 运费是否应并入本次退款（Java `calculateFinalRefundAmount` 的判定部分）。
///
/// 入参 `is_full_coverage_with_current`：把**当前**售后明细也算进去后
/// 是否已完整覆盖订单全部商品（Java `isFullRefundCoverage(order, current, true)`）。
/// `is_full_coverage_without_current`：不含当前单是否已完整覆盖
/// （Java `coversAllOrderItems(orderItems, earlierItems)`，用于补最后一单的判定）。
#[must_use]
pub fn should_add_shipping(
    order: &Order,
    refund_amount: Decimal,
    base: Decimal,
    is_full_coverage_with_current: bool,
    is_full_coverage_without_current: bool,
) -> Decimal {
    let shipping_fee = order.shipping_fee;
    if shipping_fee <= Decimal::ZERO {
        return refund_amount;
    }
    // 金额已高于基础商品退款额（优惠券/折扣未按比例扣除导致虚高）→ 封顶
    if refund_amount > base {
        return refund_amount.min(base + shipping_fee);
    }
    let in_window = order.shipping_status == SHIPPING_STATUS_PENDING
        || (order.order_status == ORDER_STATUS_COMPLETED
            && order.shipping_status == SHIPPING_STATUS_SHIPPED
            && order.shipping_time <= 0);
    if in_window && is_full_coverage_with_current && !is_full_coverage_without_current {
        refund_amount + shipping_fee
    } else {
        refund_amount
    }
}

/// 完成退款金额（Java `calculateFinalRefundAmount`）。
#[must_use]
pub fn final_refund_amount(
    order: &Order,
    base: Decimal,
    input_refund_amount: Option<Decimal>,
    is_full_coverage_with_current: bool,
    is_full_coverage_without_current: bool,
) -> Decimal {
    let refund_amount = input_refund_amount.unwrap_or(base);
    let shipping_fee = order.shipping_fee;
    if shipping_fee <= Decimal::ZERO {
        return refund_amount;
    }
    if refund_amount > base {
        return refund_amount.min(base + shipping_fee);
    }
    let in_window = order.shipping_status == SHIPPING_STATUS_PENDING
        || (order.order_status == ORDER_STATUS_COMPLETED
            && order.shipping_status == SHIPPING_STATUS_SHIPPED
            && order.shipping_time <= 0);
    if in_window && is_full_coverage_with_current && !is_full_coverage_without_current {
        refund_amount + shipping_fee
    } else {
        refund_amount
    }
}

/// 判断售后明细是否完整覆盖订单全部商品（Java `coversAllOrderItems`）。
#[must_use]
pub fn covers_all_order_items(order_items: &[OrderItem], items: &[AftersalesItem]) -> bool {
    if items.is_empty() {
        return false;
    }
    let mut qty: HashMap<i32, i32> = HashMap::new();
    for it in items {
        if let Some(id) = it.order_item_id {
            *qty.entry(id).or_insert(0) += it.number.unwrap_or(0);
        }
    }
    order_items
        .iter()
        .all(|oi| qty.get(&oi.item_id).copied().is_some_and(|v| v >= oi.quantity))
}

/// 售后原因配置校验（Java `AfterSalesReasonConfigService#validateNewBuyerReason`）。
///
/// 返回归一化后的原因；`enabled = 0` 时原样返回。
pub fn validate_buyer_reason(incoming: Option<&str>, enabled: i32, reasons: &[String]) -> Result<Option<String>> {
    let normalized = incoming
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    if enabled == 0 {
        return Ok(normalized);
    }
    let Some(normalized) = normalized else {
        return Err(Error::biz_msg("请选择原因"));
    };
    if reasons.iter().any(|r| r == &normalized) {
        return Ok(Some(normalized));
    }
    // `matchTranslatedReason` 依赖 `translatePackage`（未迁移），
    // 无多语言上下文时必然匹配失败 → 与 Java 单语言部署行为一致
    Err(Error::biz_msg("所选原因已失效，请重新选择"))
}

/// 修改申请时的原因解析（Java `resolveUpdateReason`）。
pub fn resolve_update_reason(
    stored: Option<&str>,
    incoming: Option<&str>,
    enabled: i32,
    reasons: &[String],
) -> Result<Option<String>> {
    let stored_norm = stored.map(str::trim).filter(|s| !s.is_empty());
    let incoming_norm = incoming.map(str::trim).filter(|s| !s.is_empty());
    if let Some(stored_norm) = stored_norm {
        if incoming_norm.is_none() || incoming_norm == Some(stored_norm) {
            return Ok(Some(stored_norm.to_string()));
        }
    }
    validate_buyer_reason(incoming, enabled, reasons)
}

/// 解析原因列表配置（Java `parseStoredReasons` + `normalizeReasons`）。
///
/// 与 Java 的差别：Java 配置异常时**抛业务异常**（`GlobalException`）；
/// 这里按 `SettingsHelper` 的默认值兜底，避免配置异常把只读接口打成 500。
#[must_use]
pub fn parse_reason_list(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
    };
    let Ok(arr) = serde_json::from_str::<serde_json::Value>(raw) else {
        return DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
    };
    let Some(arr) = arr.as_array() else {
        return DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
    };
    let mut out: Vec<String> = Vec::with_capacity(arr.len());
    for v in arr {
        let Some(s) = v.as_str() else {
            return DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
        };
        let s = s.trim();
        if s.is_empty() || s.chars().count() > MAX_REASON_LENGTH || out.iter().any(|x| x == s) {
            return DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
        }
        out.push(s.to_string());
    }
    if out.len() > MAX_REASON_COUNT {
        return DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
    }
    out
}

/// 解析 `afterSalesReasonEnabled`（Java `parseEnabled`：非 `"0"`/`"1"` 抛异常，
/// 此处按默认值 `1` 兜底）。
#[must_use]
pub fn parse_reason_enabled(raw: Option<&str>) -> i32 {
    match raw.map(str::trim) {
        Some("0") => 0,
        Some("1") => 1,
        None => 1,
        Some(_) => 1,
    }
}

/// `config` 表快照（Java `SettingsHelper#getRawSnapshot` 的等价物）。
#[derive(Debug, Clone)]
struct Configs {
    review_enabled: bool,
    reason_enabled: i32,
    reasons: Vec<String>,
    auto_return_goods_days: Option<String>,
}

/// 售后流程步骤（Java `resolveAutoAgreeEndTime`：仅
/// 「待审核 / 待供应商审核」且开了自动同意退货时返回截止时间）。
#[must_use]
pub fn auto_agree_end_time(
    status: i32,
    add_time: i32,
    auto_return_goods: Option<&str>,
    days_raw: Option<&str>,
) -> Option<String> {
    if !matches!(
        status,
        aftersales_status::IN_REVIEW | aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
    ) || auto_return_goods.map(str::trim) != Some("1")
        || add_time <= 0
    {
        return None;
    }
    let days: f64 = days_raw?.trim().parse().ok()?;
    if days <= 0.0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let end = i64::from(add_time) + (days * 86_400.0) as i64;
    i32::try_from(end).ok().map(fmt_time).filter(|s| !s.is_empty())
}

/// 售后进度条（Java `AftersalesServiceImpl#getStep`）。
#[must_use]
pub fn build_step_status(a: &Aftersales) -> StepStatusVO {
    let pay_return = aftersales_type::is_pay_return(Some(a.aftersale_type));
    let mut current = 0;
    let mut status = STEP_STATUS_PROCESS.to_string();

    let mut steps: Vec<StepVO> = Vec::with_capacity(4);
    steps.push(StepVO {
        title: STEP_TITLE_SUBMIT.into(),
        active: true,
        description: fmt_time(a.add_time),
    });
    steps.push(StepVO {
        title: STEP_TITLE_REVIEWING.into(),
        active: false,
        description: fmt_time_opt(a.audit_time).unwrap_or_default(),
    });
    if pay_return {
        steps.push(StepVO {
            title: STEP_TITLE_COMPLETED.into(),
            active: false,
            description: fmt_time_opt(a.final_time).unwrap_or_default(),
        });
    } else {
        steps.push(StepVO {
            title: STEP_TITLE_PROCESSING.into(),
            active: false,
            description: fmt_time_opt(a.deal_time).unwrap_or_default(),
        });
        steps.push(StepVO {
            title: STEP_TITLE_COMPLETED.into(),
            active: false,
            description: fmt_time_opt(a.final_time).unwrap_or_default(),
        });
    }

    match a.status {
        aftersales_status::IN_REVIEW => {
            current = 1;
            steps[1].active = true;
            steps[1].description = STEP_DESC_REVIEWING.into();
        }
        aftersales_status::APPROVED_FOR_PROCESSING => {
            current = 1;
            steps[1].active = true;
            steps[1].description = STEP_DESC_APPROVED.into();
        }
        aftersales_status::REFUSE => {
            current = 1;
            steps[1].active = true;
            steps[1].description = STEP_DESC_REJECTED.into();
        }
        aftersales_status::SEND_BACK => {
            current = 2;
            steps[1].active = true;
            steps[1].description = STEP_DESC_WAIT_RETURN.into();
            steps[2].active = true;
        }
        aftersales_status::RETURNED => {
            current = 2;
            steps[1].active = true;
            steps[1].description = STEP_DESC_WAIT_MERCHANT_RECEIPT.into();
            steps[2].active = true;
        }
        aftersales_status::WAIT_FOR_SUPPLIER_AUDIT => {
            current = 2;
            steps[1].active = true;
            steps[1].description = STEP_DESC_WAIT_SUPPLIER_REVIEW.into();
            steps[2].active = true;
        }
        aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT => {
            current = 2;
            steps[1].active = true;
            steps[1].description = STEP_DESC_WAIT_SUPPLIER_RECEIPT.into();
            steps[2].active = true;
        }
        aftersales_status::COMPLETE => {
            current = if pay_return { 2 } else { 3 };
            status = STEP_STATUS_FINISH.to_string();
            steps[1].active = true;
            steps[2].active = true;
            steps[2].description = fmt_time(a.final_time);
            if !pay_return {
                if let Some(s) = steps.get_mut(3) {
                    s.active = true;
                }
            }
            // Java 会再查 RefundApply：仅退款且退款未处理完成时降级为「财务审核中」。
            // `refund_apply` 关联在本切片未迁移，见 `uncertain_points`。
            if pay_return {
                status = STEP_STATUS_PROCESS.to_string();
                current = 1;
                steps[1].active = true;
                steps[2].active = false;
                if let Some(s) = steps.get_mut(3) {
                    s.active = false;
                }
                steps[1].description = STEP_DESC_FINANCE_REVIEWING.into();
            }
        }
        aftersales_status::CANCEL => {
            current = 1;
            status = STEP_STATUS_FINISH.to_string();
            steps[1].title = STEP_TITLE_CANCELLED.into();
            steps[1].active = true;
            steps[1].description = String::new();
            // Java：remove(2)，非仅退款时再 remove(2)（只影响后两步）
            if pay_return {
                steps.truncate(2);
            } else {
                steps.truncate(1);
            }
        }
        _ => {}
    }

    StepStatusVO {
        current,
        status,
        steps,
    }
}

/// 前台用户售后服务。
pub struct AftersalesClientService {
    repos: Arc<Repositories>,
}

impl AftersalesClientService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 读取本域需要的全部配置快照。
    async fn load_configs(&self) -> Result<Configs> {
        let review = self
            .repos.trade.aftersales_client
            .find_config_raw(CFG_AFTER_SALES_REVIEW)
            .await?;
        let reason_enabled = self
            .repos.trade.aftersales_client
            .find_config_raw(CFG_AFTER_SALES_REASON_ENABLED)
            .await?;
        let reason_list = self
            .repos.trade.aftersales_client
            .find_config_raw(CFG_AFTER_SALES_REASON_LIST)
            .await?;
        let auto_days = self
            .repos.trade.aftersales_client
            .find_config_raw(CFG_AUTO_RETURN_GOODS_DAYS)
            .await?;
        Ok(Configs {
            // Java `AFTER_SALES_REVIEW` 默认值 "1"
            review_enabled: review_enabled(review.as_deref().or(Some("1"))),
            reason_enabled: parse_reason_enabled(reason_enabled.as_deref()),
            reasons: parse_reason_list(reason_list.as_deref()),
            auto_return_goods_days: auto_days,
        })
    }

    // ————————————————————— ① 售后配置 —————————————————————

    /// `GET /api/user/aftersales/config`（Java `AfterSalesController.config`）。
    pub async fn config(&self, _order_id: Option<i32>) -> Result<AftersalesConfigVO> {
        let cfg = self.load_configs().await?;
        let mut types = BTreeMap::new();
        types.insert(
            aftersales_type::RETURN.to_string(),
            nextrade_core::models::trade::aftersales_client::TYPE_NAME_RETURN.to_string(),
        );
        types.insert(
            aftersales_type::PAY_RETURN.to_string(),
            nextrade_core::models::trade::aftersales_client::TYPE_NAME_PAY_RETURN.to_string(),
        );
        Ok(AftersalesConfigVO {
            aftersale_type: types,
            aftersale_reason: cfg.reasons,
            after_sales_reason_enabled: cfg.reason_enabled,
        })
    }

    // ————————————————————— ② 售后列表（可按订单申请） —————————————————————

    /// `GET /api/user/aftersales/list`（Java
    /// `AfterSalesController.list` → `OrderServiceImpl#afterSalesList`）。
    pub async fn list(&self, q: &AftersalesListQuery, user_id: i32) -> Result<PageResult<AfterSalesClientVO>> {
        let page = q.page_query();
        let (total, orders) = self
            .repos.trade.aftersales_client
            .paginate_aftersales_orders(q, user_id)
            .await?;
        if orders.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let order_ids: Vec<i32> = orders.iter().map(|o| o.order_id).collect();
        let items = self
            .repos.trade.aftersales_client
            .find_order_items_by_order_ids(&order_ids)
            .await?;
        let mut items_by_order: HashMap<i32, Vec<OrderItem>> = HashMap::new();
        for it in items {
            items_by_order.entry(it.order_id).or_default().push(it);
        }

        let product_ids = distinct(
            items_by_order
                .values()
                .flatten()
                .map(|it| it.product_id)
                .filter(|id| *id > 0),
        );
        let products: HashMap<i32, Product> = self
            .repos.trade.aftersales_client
            .find_products(&product_ids)
            .await?
            .into_iter()
            .map(|p| (p.product_id, p))
            .collect();

        // 各订单「进行中」售后明细：order_item_id -> (aftersale_id, number, status)
        let mut applied: HashMap<i32, Vec<AppliedItem>> = HashMap::new();
        for order in &orders {
            let rows = self
                .repos.trade.aftersales_client
                .find_in_progress_aftersales(order.order_id, &aftersales_status::applied_statuses())
                .await?;
            let ids: Vec<i32> = rows.iter().map(|a| a.aftersale_id).collect();
            let status_of: HashMap<i32, i32> =
                rows.iter().map(|a| (a.aftersale_id, a.status)).collect();
            let detail = self
                .repos.trade.aftersales_client
                .find_items_by_aftersale_ids(&ids)
                .await?;
            let mut list: Vec<AppliedItem> = Vec::new();
            for (aid, its) in detail {
                let status = status_of.get(&aid).copied().unwrap_or_default();
                for it in its {
                    if let Some(oid) = it.order_item_id {
                        list.push(AppliedItem {
                            order_item_id: oid,
                            number: it.number.unwrap_or(0),
                            status,
                            aftersale_id: aid,
                        });
                    }
                }
            }
            applied.insert(order.order_id, list);
        }

        // 店铺配置：`order_config.dateType == 0` 表示「确认收货后立即不可售后」
        let mut shop_instant_close: HashMap<i32, bool> = HashMap::new();
        for it in items_by_order.values().flatten() {
            if it.shop_id > 0 && !shop_instant_close.contains_key(&it.shop_id) {
                let raw = self
                    .repos.trade.aftersales_client
                    .find_order_config(it.shop_id, "order_config")
                    .await?;
                let date_type = raw
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    .and_then(|v| v.get("dateType").and_then(serde_json::Value::as_i64));
                shop_instant_close.insert(it.shop_id, date_type == Some(0));
            }
        }

        let mut list = Vec::with_capacity(orders.len());
        for order in &orders {
            let empty: Vec<OrderItem> = Vec::new();
            let order_items = items_by_order.get(&order.order_id).unwrap_or(&empty);
            let applied_items = applied.get(&order.order_id);
            let mut vo_items = Vec::with_capacity(order_items.len());
            for oi in order_items {
                let used: i32 = applied_items
                    .map(|v| {
                        v.iter()
                            .filter(|a| a.order_item_id == oi.item_id)
                            .map(|a| a.number)
                            .sum()
                    })
                    .unwrap_or(0);
                let can_apply = can_apply_quantity(oi.quantity, used);
                // Java `toMap` 冲突时保留首条；列表按 `aftersale_id` 倒序，
                // 取最大 `aftersale_id` 即等价于「首条」
                let latest = applied_items.and_then(|v| {
                    v.iter()
                        .filter(|a| a.order_item_id == oi.item_id)
                        .max_by_key(|a| a.aftersale_id)
                });
                let flag = aftersale_flag(latest.map(|a| a.status), can_apply);
                let mut flag_final = flag;
                // 店铺配置：已完成订单 + dateType == 0 → 立即不可申请
                if order.order_status == ORDER_STATUS_COMPLETED
                    && shop_instant_close.get(&oi.shop_id).copied().unwrap_or(false)
                {
                    flag_final = 0;
                }

                let product = products.get(&oi.product_id);
                vo_items.push(AfterSalesClientItemVO {
                    item_id: oi.item_id,
                    order_id: oi.order_id,
                    product_id: oi.product_id,
                    price: oi.price,
                    quantity: oi.quantity,
                    product_name: oi.product_name.clone(),
                    product_sn: oi.product_sn.clone(),
                    pic_thumb: oi.pic_thumb.clone(),
                    product_pic_thumb: product.map(|p| p.pic_thumb.clone()),
                    product_stock: product.map(|p| p.product_stock),
                    product_weight: product.map(|p| p.product_weight),
                    virtual_sample: product.and_then(|p| p.virtual_sample.clone()),
                    paid_content: product
                        .and_then(|p| p.paid_content.as_deref())
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
                    card_group_id: product.map(|p| p.card_group_id),
                    // Java `AfterSalesClientVO.Item.aftersalesItem` 在该链路
                    // 从未被赋值，故恒为 null；此处保持一致
                    aftersales_item: None,
                    aftersale_flag: flag_final,
                    // Java：默认 true，再由店铺配置 / 全局售后期限覆盖；
                    // `afterSalesLimitDays` 未迁移，故已完成订单不下发覆盖
                    to_aftersalses: flag_final != 0,
                });
            }
            list.push(AfterSalesClientVO {
                order_id: order.order_id,
                order_sn: order.order_sn.clone(),
                shipping_time: fmt_time(order.shipping_time),
                items: vo_items,
            });
        }

        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— ③ 售后申请数据（applyData） —————————————————————

    /// `GET /api/user/aftersales/applyData`（Java `AfterSalesController.applyData`
    /// → `OrderServiceImpl#detail` + `OrderServiceImpl#applyData`）。
    pub async fn apply_data(
        &self,
        item_id: Option<i32>,
        order_id: Option<i32>,
    ) -> Result<AfterSalesApplyDataVO> {
        let order_id = order_id.ok_or_else(|| Error::Validation("orderId 不能为空".into()))?;
        let order = self
            .repos.trade.aftersales_client
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_ORDER_NOT_EXIST))?;

        // 店铺「确认收货后 N 天不可申请」限制
        let config_raw = self
            .repos.trade.aftersales_client
            .find_order_config(order.shop_id, "order_config")
            .await?;
        if order.order_status == ORDER_STATUS_COMPLETED {
            if let Some(cfg) = config_raw
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            {
                let data_type = cfg.get("dataType").and_then(serde_json::Value::as_i64);
                let use_day = cfg.get("useDay").and_then(serde_json::Value::as_i64);
                if data_type == Some(1) {
                    let elapsed = i64::from(utils::now_seconds() as i32) - i64::from(order.received_time);
                    if let Some(use_day) = use_day {
                        if elapsed > use_day * 24 * 60 * 60 {
                            return Err(Error::biz_msg(ERR_AFTERSALES_TIME_EXPIRED));
                        }
                    }
                }
            }
        }

        let order_items = self
            .repos.trade.aftersales_client
            .find_order_items(order_id, item_id)
            .await?;
        if order_items.is_empty() {
            return Err(Error::biz_msg(ERR_ORDER_ITEM_NOT_EXIST));
        }

        // 云支付渠道（微信/支付宝/云闪付）下单件退款不受支持
        if item_id.is_some()
            && self
                .repos.trade.aftersales_client
                .has_success_paylog_with_codes(
                    order_id,
                    &["yunpay_wechat", "yunpay_alipay", "yunpay_yunshanfu"],
                )
                .await?
        {
            return Err(Error::biz_msg(ERR_FULL_REFUND_ONLY));
        }

        let applied = self
            .repos.trade.aftersales_client
            .find_in_progress_aftersales(order_id, &aftersales_status::applied_statuses())
            .await?;
        let applied_ids: Vec<i32> = applied.iter().map(|a| a.aftersale_id).collect();
        let applied_items = self
            .repos.trade.aftersales_client
            .find_items_by_aftersale_ids(&applied_ids)
            .await?;
        let mut applied_qty: HashMap<i32, i32> = HashMap::new();
        for its in applied_items.values() {
            for it in its {
                if let Some(oid) = it.order_item_id {
                    *applied_qty.entry(oid).or_insert(0) += it.number.unwrap_or(0);
                }
            }
        }

        let tax_rate = order.tax_rate.unwrap_or(Decimal::ZERO);
        let mut list = Vec::with_capacity(order_items.len());
        for oi in &order_items {
            let unit_tax = unit_tax_amount(oi.price, tax_rate);
            let price_with_tax = oi.price + unit_tax;
            let used = applied_qty.get(&oi.item_id).copied().unwrap_or(0);
            let sku_data = parse_sku_data(&oi.sku_data);
            list.push(AfterSalesDetailVO {
                item_id: oi.item_id,
                pic_thumb: oi.pic_thumb.clone(),
                product_sn: oi.product_sn.clone(),
                product_id: oi.product_id,
                product_name: oi.product_name.clone(),
                price: price_with_tax,
                tax_rate_amount: unit_tax,
                quantity: oi.quantity,
                subtotal: mul_ceil2(price_with_tax, oi.quantity),
                sku_data,
                can_apply_quantity: can_apply_quantity(oi.quantity, used),
                is_gift: oi.is_gift,
                attr_price: None,
            });
        }

        let detail = self.order_detail(order_id).await?;

        // Controller 层回填：`order_item.extra_sku_data[0].attrPrice`
        for d in &mut list {
            if let Some(oi) = order_items.iter().find(|o| o.item_id == d.item_id) {
                let extra: Vec<serde_json::Value> = oi
                    .extra_sku_data
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                if let Some(first) = extra.first() {
                    d.attr_price = first.get("attrPrice").and_then(json_decimal);
                }
            }
        }

        Ok(AfterSalesApplyDataVO { list, order: detail })
    }

    /// 订单详情 VO。
    ///
    /// **复用说明**：Java `AfterSalesController` 的 `applyData` / `detail` 调的
    /// 就是同一个 `orderService.detail(orderId, userId)`，而本仓库对该位置唯一的
    /// 逐字段还原实现是 trade::order_admin 的 `OrderAdminService::detail`（156 个字段 + 批量
    /// 预取 + 订单状态文案矩阵）。为了不产生第二份会漂移的 `OrderVO` 构造逻辑，
    /// 这里直接复用（切片间复用仓储/服务在仓库中已有先例，见
    /// `crates/infrastructure/src/db/repo/aftersales.rs`）。
    ///
    /// 已知差异：trade::order_admin 会在「状态名仍为标准文案」时把
    /// `orderStatusName` 覆盖为**后台**文案（Java
    /// `OrderStatusEnum.getAdminStatusName`），前台订单详情本不应叠加该覆盖。
    /// 影响面见 `uncertain_points`。
    async fn order_detail(&self, order_id: i32) -> Result<nextrade_core::models::trade::aftersales_client::OrderVO> {
        crate::trade::order_admin::OrderAdminService::new(Arc::clone(&self.repos))
            .detail(order_id)
            .await
    }

    // ————————————————————— ④ 售后申请 —————————————————————

    /// `POST /api/user/aftersales/create`（Java `AfterSalesController.create`
    /// → `AftersalesServiceImpl#afterSalesApply`）。
    pub async fn create(
        &self,
        param: &CreateAftersalesParam,
        user_id: i32,
    ) -> Result<CreateAftersalesResult> {
        if param.items.as_deref().is_none_or(<[AftersalesApplyItem]>::is_empty)
            || param.total_number() == 0
        {
            return Err(Error::Validation(ERR_QUANTITY_REQUIRED.into()));
        }
        let items = param.non_zero_items();

        let order_id = param
            .order_id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;

        // Java：先逐条校验礼品卡兑换商品
        let item_ids: Vec<i32> = items.iter().filter_map(|i| i.order_item_id).collect();
        let requested_items = self
            .repos.trade.aftersales_client
            .find_order_items_by_ids(&item_ids)
            .await?;
        if requested_items
            .iter()
            .any(|it| it.gift_card_id.is_some_and(|v| v > 0))
        {
            return Err(Error::biz_msg("该商品时礼品卡兑换商品不可进行售后"));
        }

        let order = self
            .repos.trade.aftersales_client
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_ORDER_NOT_EXIST))?;

        // 电子礼品卡（虚拟）不支持售后
        if !item_ids.is_empty() {
            let apply_items = self
                .repos.trade.aftersales_client
                .find_order_items(order.order_id, None)
                .await?;
            let contains_virtual_gift_card = apply_items.iter().any(|it| {
                item_ids.contains(&it.item_id) && it.product_type == PRODUCT_TYPE_GIFT_CARD_VIRTUAL
            });
            if contains_virtual_gift_card {
                return Err(Error::biz_msg(ERR_GIFTCARD_NOT_ALLOWED));
            }
        }

        if order.pay_status != PAY_STATUS_PAID {
            return Err(Error::biz_msg(ERR_UNPAID_NOT_ALLOWED));
        }
        if order.order_source == "pos" {
            return Err(Error::biz_msg(ERR_POS_NOT_ALLOWED));
        }

        let cfg = self.load_configs().await?;

        // 原因校验（Java：非后台取消时校验买家原因）
        let reason = if param.is_admin_cancel == Some(true) {
            param
                .aftersale_reason
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
        } else {
            validate_buyer_reason(param.aftersale_reason.as_deref(), cfg.reason_enabled, &cfg.reasons)?
        };

        // 数量校验（Java `validateOrderQuantity(items, orderId, isAdd = true, ..)`）
        self.validate_order_quantity(order_id, &items, None).await?;

        let now = utils::now_seconds() as i32;
        let aftersale_type_value = param.aftersale_type.unwrap_or(aftersales_type::PAY_RETURN);
        let pics = param.pics_json();
        let description = param.description.clone().unwrap_or_default();
        // Java `param.getRefundAmount() == null` 的等价物：前端是否传了金额
        let refund_amount_provided = param.refund_amount;
        let pics_clone = pics.clone();

        let mut aftersales = Aftersales {
            aftersale_id: 0,
            aftersale_type: aftersale_type_value,
            status: aftersales_status::IN_REVIEW,
            pics,
            description: description.clone(),
            reply: String::new(),
            add_time: now,
            tracking_no: String::new(),
            logistics_name: String::new(),
            return_address: None,
            aftersale_reason: reason,
            aftersales_sn: Some(aftersales_sn(now)),
            order_id: Some(order.order_id),
            user_id: Some(user_id),
            refund_amount: param.refund_amount.unwrap_or(Decimal::ZERO),
            shop_id: order.shop_id,
            audit_time: 0,
            deal_time: 0,
            final_time: 0,
            return_goods_tip: String::new(),
            vendor_id: order.vendor_id.filter(|v| *v > 0),
            close_type: 0,
        };

        let af_items: Vec<AftersalesItem> = items
            .iter()
            .map(|i| AftersalesItem {
                aftersales_item_id: 0,
                order_item_id: i.order_item_id,
                number: i.number,
                aftersale_id: None,
            })
            .collect();

        let username = self.repos.trade.aftersales_client.find_username(user_id).await?;
        let log = AftersalesLog {
            log_id: 0,
            aftersale_id: 0,
            log_info: format!(
                "会员发起了{}: {}",
                type_name(Some(aftersale_type_value)),
                description
            ),
            add_time: now,
            admin_name: String::new(),
            refund_money: Decimal::ZERO,
            refund_type: 0,
            refund_desc: String::new(),
            user_name: username,
            return_pic: Some(pics_clone).filter(|s| !s.is_empty()),
            shop_id: Some(order.shop_id),
            vendor_id: order.vendor_id.filter(|v| *v > 0),
        };

        let aftersale_id = self
            .repos.trade.aftersales_client
            .insert_aftersales_bundle(&aftersales, &af_items, &log)
            .await?;
        aftersales.aftersale_id = aftersale_id;

        if cfg.review_enabled {
            // 审核开启：等待商家审核；自动同意退货延时消息（RabbitMQ）与
            // 后台消息中心未迁移，如实返回「待审核」。
            return Ok(CreateAftersalesResult {
                aftersale_id,
                aftersale_type: aftersale_type_value,
                status: aftersales_status::IN_REVIEW,
                status_name: type_name(Some(aftersale_type_value)),
                refund_amount: aftersales.refund_amount,
                auto_approved: false,
            });
        }

        // ——— 审核关闭：按 Java 逻辑自动审核 ———
        let all_order_items = self
            .repos.trade.aftersales_client
            .find_order_items(order.order_id, None)
            .await?;

        let mut final_status = aftersales_status::IN_REVIEW;

        if aftersale_type_value == aftersales_type::PAY_RETURN {
            // 仅退款：金额缺失时按建议金额补齐，再按运费规则封顶/并入
            let base = if refund_amount_provided.is_none() {
                suggest_refund_amount(&order, &all_order_items, &items)
            } else {
                let as_items = self.repos.trade.aftersales_client.find_items(aftersale_id).await?;
                base_refund_amount(&order, &all_order_items, &as_items, aftersales.refund_amount)
            };
            let (full_with, full_without) = self.coverage(&order, aftersale_id).await?;
            let amount = final_refund_amount(
                &order,
                base,
                refund_amount_provided.or(Some(base)),
                full_with,
                full_without,
            );
            aftersales.refund_amount = amount;
            final_status = aftersales_status::APPROVED_FOR_PROCESSING;
            aftersales.reply = "系统自动审核通过".into();
            aftersales.return_goods_tip = "您的售后申请已通过".into();
        } else if aftersale_type_value == aftersales_type::RETURN {
            // 退货退款：重算商品金额（> 0 才覆盖），流转到「待回寄」
            let as_items = self.repos.trade.aftersales_client.find_items(aftersale_id).await?;
            let vos = self.build_item_vos(&order, &as_items).await?;
            let recalc = calc_refund_product_amount(&order, &vos);
            if recalc > Decimal::ZERO {
                aftersales.refund_amount = recalc;
            }
            final_status = aftersales_status::SEND_BACK;
            aftersales.reply = "系统自动审核通过".into();
            aftersales.return_goods_tip = "您的售后申请已通过".into();
        }

        if final_status != aftersales_status::IN_REVIEW {
            aftersales.status = final_status;
            aftersales.audit_time = utils::now_seconds() as i32;
            self.repos.trade.aftersales_client.update_aftersales(&aftersales).await?;

            // 自动审核留痕（Java `orderLogService.quickCreateBySystemAdmin` 属订单域，
            // 未迁移；此处只追加售后日志，保持前台可见的进度可解释）
            let auto_log = AftersalesLog {
                log_id: 0,
                aftersale_id,
                log_info: "系统自动审核通过".into(),
                add_time: utils::now_seconds() as i32,
                admin_name: String::new(),
                refund_money: Decimal::ZERO,
                refund_type: 0,
                refund_desc: String::new(),
                user_name: None,
                return_pic: None,
                shop_id: None,
                vendor_id: None,
            };
            self.repos.trade.aftersales_client
                .insert_aftersales_log(&auto_log)
                .await?;
        }
        Ok(CreateAftersalesResult {
            aftersale_id,
            aftersale_type: aftersale_type_value,
            status: final_status,
            status_name: type_name(Some(aftersale_type_value)),
            refund_amount: aftersales.refund_amount,
            auto_approved: final_status != aftersales_status::IN_REVIEW,
        })
    }

    /// 数量校验（Java `AftersalesServiceImpl#validateOrderQuantity`）。
    ///
    /// `exclude_aftersale_id`：修改申请时排除自身（Java `isAdd = false`）。
    async fn validate_order_quantity(
        &self,
        order_id: i32,
        items: &[AftersalesApplyItem],
        exclude_aftersale_id: Option<i32>,
    ) -> Result<()> {
        for item in items {
            let number = item.number.unwrap_or(0);
            if number == 0 {
                continue;
            }
            let Some(order_item_id) = item.order_item_id else {
                continue;
            };

            // 新增时才判「是否已有进行中申请」
            if exclude_aftersale_id.is_none()
                && self
                    .repos.trade.aftersales_client
                    .has_applied_aftersales(
                        order_id,
                        order_item_id,
                        &aftersales_status::applied_statuses(),
                    )
                    .await?
            {
                return Err(Error::biz_msg(ERR_PRODUCT_HAS_APPLIED));
            }

            let quantity = self
                .repos.trade.aftersales_client
                .find_order_items(order_id, Some(order_item_id))
                .await?
                .first()
                .map_or(0, |it| it.quantity);
            let used = self
                .repos.trade.aftersales_client
                .valid_quantity_by_item(
                    order_id,
                    &aftersales_status::valid_statuses(),
                    exclude_aftersale_id,
                )
                .await?
                .get(&order_item_id)
                .copied()
                .unwrap_or(0);

            if quantity < number + used {
                return Err(Error::biz_msg(ERR_QUANTITY_INSUFFICIENT));
            }
        }
        Ok(())
    }

    /// 「当前售后单 + 其他有效售后」是否完整覆盖订单
    /// （返回 `(含当前单, 不含当前单)`）。
    ///
    /// 对应 Java `isFullRefundCoverage(order, current, true)` 与
    /// `coversAllOrderItems(orderItems, earlierItems)`：
    /// - 「含当前单」= 订单内**全部有效状态**售后（含已完成）的明细；
    /// - 「不含当前单」= 其余有效状态售后的明细。
    async fn coverage(&self, order: &Order, aftersale_id: i32) -> Result<(bool, bool)> {
        let order_items = self
            .repos.trade.aftersales_client
            .find_order_items(order.order_id, None)
            .await?;
        if order_items.is_empty() {
            return Ok((false, false));
        }

        let current = self.repos.trade.aftersales_client.find_items(aftersale_id).await?;
        let valid = self
            .repos.trade.aftersales_client
            .find_valid_aftersales(order.order_id, &aftersales_status::valid_statuses())
            .await?;
        let other_ids: Vec<i32> = valid
            .iter()
            .map(|a| a.aftersale_id)
            .filter(|id| *id != aftersale_id)
            .collect();
        let other_items = self
            .repos.trade.aftersales_client
            .find_items_by_aftersale_ids(&other_ids)
            .await?;
        let others_flat: Vec<AftersalesItem> = other_items.into_values().flatten().collect();

        let mut all: Vec<AftersalesItem> = others_flat.clone();
        all.extend(current);

        Ok((
            covers_all_order_items(&order_items, &all),
            covers_all_order_items(&order_items, &others_flat),
        ))
    }

    // ————————————————————— ⑤ 售后申请修改 —————————————————————

    /// `POST /api/user/aftersales/update`（Java `AfterSalesController.update`
    /// → `AftersalesServiceImpl#afterSalesApplyUpdate`）。
    pub async fn update(
        &self,
        param: &ClientAfterSalesUpdateParam,
        user_id: i32,
    ) -> Result<()> {
        let aftersale_id = param
            .aftersale_id
            .ok_or_else(|| Error::Validation("售后id不能为空".into()))?;
        let mut aftersales = self
            .repos.trade.aftersales_client
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_AFTERSALES_NOT_EXIST))?;

        if aftersales.status != aftersales_status::IN_REVIEW {
            return Err(Error::biz_msg(ERR_STATUS_CANNOT_MODIFY));
        }
        // Java：`Assert.isFalse(updateDTO.getItems().isEmpty(), "请填写申请售后数量")`
        // —— `items` 为 null 时 Java 抛 NPE，此处同样按「请填写申请售后数量」拒绝。
        if param.items.as_deref().is_none_or(<[AftersalesApplyItem]>::is_empty) {
            return Err(Error::biz_msg(ERR_QUANTITY_REQUIRED));
        }
        if param.total_number() <= 0 {
            return Err(Error::biz_msg(ERR_QUANTITY_ZERO));
        }

        let cfg = self.load_configs().await?;
        let order_id = param.order_id.or(aftersales.order_id).unwrap_or_default();
        let reason = resolve_update_reason(
            aftersales.aftersale_reason.as_deref(),
            param.aftersale_reason.as_deref(),
            cfg.reason_enabled,
            &cfg.reasons,
        )?;

        let items = param.items.clone().unwrap_or_default();
        self.validate_order_quantity(order_id, &items, Some(aftersale_id))
            .await?;

        // Java `BeanUtils.copyProperties` 忽略 null 字段
        if let Some(v) = param.aftersale_type {
            aftersales.aftersale_type = v;
        }
        if let Some(v) = param.description.as_ref() {
            aftersales.description = v.clone();
        }
        if let Some(v) = param.refund_amount {
            aftersales.refund_amount = v;
        }
        aftersales.aftersale_reason = reason;
        if let Some(pics) = param.pics_json() {
            aftersales.pics = pics;
        }

        let af_items: Vec<AftersalesItem> = items
            .iter()
            .map(|i| AftersalesItem {
                aftersales_item_id: 0,
                order_item_id: i.order_item_id,
                number: i.number,
                aftersale_id: Some(aftersale_id),
            })
            .collect();

        // Java 日志使用**入参**的 aftersaleType
        let log_type = param.aftersale_type.unwrap_or(aftersales.aftersale_type);
        let username = self.repos.trade.aftersales_client.find_username(user_id).await?;
        let log = AftersalesLog {
            log_id: 0,
            aftersale_id,
            log_info: format!(
                "会员发起了{}: {}",
                type_name(Some(log_type)),
                param.description.clone().unwrap_or_default()
            ),
            add_time: utils::now_seconds() as i32,
            admin_name: String::new(),
            refund_money: Decimal::ZERO,
            refund_type: 0,
            refund_desc: String::new(),
            user_name: username,
            return_pic: Some(param.pics_json().unwrap_or_default()),
            shop_id: None,
            vendor_id: None,
        };

        self.repos.trade.aftersales_client
            .update_aftersales_with_items_and_log(&aftersales, &af_items, &log)
            .await
    }

    // ————————————————————— ⑥ 售后申请记录（getRecord） —————————————————————

    /// `GET /api/user/aftersales/getRecord`（Java
    /// `AfterSalesController.getRecord` → `AftersalesServiceImpl#list`）。
    pub async fn get_record(
        &self,
        q: &AftersalesListQuery,
        user_id: i32,
    ) -> Result<PageResult<AftersalesVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos.trade.aftersales_client
            .paginate_aftersales(q, user_id)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let ids: Vec<i32> = rows.iter().map(|a| a.aftersale_id).collect();
        let items = self
            .repos.trade.aftersales_client
            .find_items_by_aftersale_ids(&ids)
            .await?;

        let order_ids = distinct(rows.iter().filter_map(|a| a.order_id));
        let orders: HashMap<i32, Order> = self
            .repos.trade.aftersales_client
            .find_orders_by_ids(&order_ids)
            .await?
            .into_iter()
            .map(|o| (o.order_id, o))
            .collect();

        // 逐售后单组装明细 VO（保持明细与 VO 一一对应；不可先展平再 zip，
        // 否则 `HashMap::values()` 的迭代顺序会与输入顺序不一致）
        let mut by_aftersale: HashMap<i32, Vec<AftersalesItemVO>> = HashMap::new();
        for (aid, its) in &items {
            let vos = self.build_item_vos_multi(its).await?;
            by_aftersale.insert(*aid, vos);
        }

        let server_time = fmt_time(utils::now_seconds() as i32);
        let list = rows
            .iter()
            .map(|a| {
                let order = a.order_id.and_then(|id| orders.get(&id));
                AftersalesVO {
                    aftersale_id: a.aftersale_id,
                    status: a.status,
                    add_time: fmt_time(a.add_time),
                    aftersale_reason: a.aftersale_reason.clone(),
                    aftersale_type: a.aftersale_type,
                    aftersales_items: by_aftersale
                        .get(&a.aftersale_id)
                        .cloned()
                        .unwrap_or_default(),
                    aftersales_sn: a.aftersales_sn.clone(),
                    aftersales_type_name: type_name(Some(a.aftersale_type)),
                    audit_time: fmt_time_opt(a.audit_time),
                    deal_time: fmt_time_opt(a.deal_time),
                    description: a.description.clone(),
                    final_time: fmt_time_opt(a.final_time),
                    logistics_name: a.logistics_name.clone(),
                    order_id: a.order_id,
                    order_sn: order.map_or_else(String::new, |o| o.order_sn.clone()),
                    refund_amount: a.refund_amount.to_string(),
                    reply: a.reply.clone(),
                    return_address: a.return_address.clone(),
                    shop_id: a.shop_id,
                    status_name: client_status_name(
                        Some(a.aftersale_type),
                        Some(a.status),
                        Some(a.close_type),
                        false,
                    )
                    .map(ToString::to_string),
                    tracking_no: a.tracking_no.clone(),
                    user_id: a.user_id,
                    server_time: Some(server_time.clone()),
                }
            })
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— ⑦ 售后详情 —————————————————————

    /// `GET /api/user/aftersales/detail`（Java `AfterSalesController.detail`
    /// → `AftersalesServiceImpl#clientDetail` + `OrderService#detail`）。
    pub async fn detail(&self, id: i32, user_id: i32) -> Result<AfterSalesClientDetailVO> {
        let mut vo = self.client_detail(id, user_id).await?;
        if let Some(order_id) = vo.order_id {
            vo.orders = Some(self.order_detail(order_id).await?);
        }
        Ok(vo)
    }

    /// `clientDetail`：权限校验 + 详情组装（Java
    /// `AftersalesServiceImpl#clientDetail`）。
    async fn client_detail(&self, id: i32, user_id: i32) -> Result<AfterSalesClientDetailVO> {
        let a = self
            .repos.trade.aftersales_client
            .find_aftersales(id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_AFTERSALES_NOT_EXIST))?;
        // Java `AftersalesServiceImpl#clientDetail`：
        // `if (userId != null && !userId.equals(aftersales.getUserId()))`
        // `throw new GlobalException("无权限查看此售后记录")`。
        // Java `GlobalException(String)` 的默认 code 为 `SERVICE_DATA_ERROR`(1001)，
        // 走 `GlobalExceptionHandler` 返回 HTTP 200 + 业务码，故这里用 `Error::biz`。
        if a.user_id.is_some_and(|uid| uid != user_id) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "无权限查看此售后记录"));
        }

        let items = self.repos.trade.aftersales_client.find_items(id).await?;
        let item_vos = self.build_item_vos_by_order(&a, &items).await?;
        let logs = self.build_log_vos(id).await?;

        // 「待平台财务退款 / 财务已拒绝」依赖 `refund_apply` 关联（未迁移）→ 恒 false
        let finance_rejected = false;
        let display_status = a.status;

        let mut refund_amount = a.refund_amount;
        if matches!(
            a.status,
            aftersales_status::IN_REVIEW | aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
        ) {
            if let Some(order_id) = a.order_id {
                if let Some(order) = self.repos.trade.aftersales_client.find_order(order_id).await? {
                    let order_items = self
                        .repos.trade.aftersales_client
                        .find_order_items(order.order_id, None)
                        .await?;
                    let mut qty: HashMap<i32, i32> = HashMap::new();
                    for oi in &order_items {
                        qty.insert(oi.item_id, oi.quantity);
                    }
                    let mut as_qty: HashMap<i32, i32> = HashMap::new();
                    for it in &item_vos {
                        if let Some(oid) = it.order_item_id {
                            *as_qty.entry(oid).or_insert(0) += it.number.unwrap_or(0);
                        }
                    }
                    let is_full = order_items.len() == item_vos.len()
                        && qty
                            .iter()
                            .all(|(k, v)| as_qty.get(k).copied() == Some(*v));
                    refund_amount = if is_full {
                        if order.order_status >= ORDER_STATUS_PROCESSING {
                            order.total_amount - order.shipping_fee
                        } else {
                            order.total_amount
                        }
                    } else {
                        calc_refund_product_amount(&order, &item_vos)
                    };
                }
            }
        }

        let cfg = self.load_configs().await?;
        let auto_return_goods = self
            .repos.trade.aftersales_client
            .find_config_raw(CFG_AUTO_RETURN_GOODS)
            .await?;

        let reply = Some(a.reply.as_str()).filter(|s| !s.is_empty());
        Ok(AfterSalesClientDetailVO {
            aftersale_id: a.aftersale_id,
            status: a.status,
            add_time: fmt_time(a.add_time),
            aftersale_reason: a.aftersale_reason.clone(),
            aftersale_type: a.aftersale_type,
            aftersales_items: item_vos,
            aftersales_log: logs,
            aftersales_sn: a.aftersales_sn.clone(),
            aftersales_type_name: type_name(Some(a.aftersale_type)),
            audit_time: fmt_time_opt(a.audit_time),
            deal_time: fmt_time_opt(a.deal_time),
            description: a.description.clone(),
            final_time: fmt_time_opt(a.final_time),
            logistics_name: a.logistics_name.clone(),
            order_id: a.order_id,
            pics: parse_pics(&a.pics),
            refund_amount: refund_amount.to_string(),
            reply: a.reply.clone(),
            return_address: a.return_address.clone(),
            shop_id: a.shop_id,
            status_name: client_status_name(
                Some(a.aftersale_type),
                Some(display_status),
                Some(a.close_type),
                finance_rejected,
            )
            .map(ToString::to_string),
            status_desc: client_status_desc(
                Some(a.aftersale_type),
                Some(display_status),
                Some(a.close_type),
                reply,
                finance_rejected,
            ),
            tracking_no: a.tracking_no.clone(),
            user_id: a.user_id,
            orders: None,
            step_status: build_step_status(&a),
            can_cancel: a.status == aftersales_status::IN_REVIEW,
            // Java：仅「待回寄」下发退货提示
            return_goods_tip: (a.status == aftersales_status::SEND_BACK)
                .then(|| a.return_goods_tip.clone()),
            auto_agree_end_time: auto_agree_end_time(
                a.status,
                a.add_time,
                auto_return_goods.as_deref(),
                cfg.auto_return_goods_days.as_deref(),
            ),
            server_time: Some(fmt_time(utils::now_seconds() as i32)),
        })
    }

    // ————————————————————— ⑧ 售后日志 —————————————————————

    /// `GET /api/user/aftersales/detailLog`（Java
    /// `AfterSalesController.detailLog` → `clientDetail` 权限校验 +
    /// `AftersalesLogServiceImpl#getDetailLog`）。
    pub async fn detail_log(&self, id: i32, user_id: i32) -> Result<Vec<AftersalesLogVO>> {
        // Java：先调 `clientDetail` 校验归属
        let _ = self.client_detail(id, user_id).await?;
        self.build_log_vos(id).await
    }

    // ————————————————————— ⑨ 回寄反馈 —————————————————————

    /// `POST /api/user/aftersales/feedback`（Java
    /// `AfterSalesController.feedback` → `AftersalesServiceImpl#submitFeedbackRecord`）。
    pub async fn feedback(
        &self,
        param: &AftersalesFeedbackParam,
        user_id: i32,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::Validation("售后id不能为空".into()))?;
        let mut a = self
            .repos.trade.aftersales_client
            .find_aftersales(id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_APPLY_NOT_EXIST))?;

        let before_status = a.status;
        if can_submit_return_logistics(a.status) {
            a.tracking_no = param.tracking_no.clone().unwrap_or_default();
            a.logistics_name = param.logistics_name.clone().unwrap_or_default();
            a.deal_time = utils::now_seconds() as i32;
            if a.status == aftersales_status::SEND_BACK {
                a.status = if a.vendor_id.is_some_and(|v| v > 0) {
                    aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
                } else {
                    aftersales_status::RETURNED
                };
            }
            self.repos.trade.aftersales_client.update_aftersales(&a).await?;
            let _ = before_status;
            // Java：状态由待回寄流转时投递「自动确认收货」延时队列
            // （RabbitMQ 未迁移），见 `uncertain_points`。
        }

        let username = self.repos.trade.aftersales_client.find_username(user_id).await?;
        let log = AftersalesLog {
            log_id: 0,
            aftersale_id: id,
            log_info: format!(
                "会员已回寄，物流名称：{}，物流单号：{}",
                param.logistics_name.clone().unwrap_or_default(),
                param.tracking_no.clone().unwrap_or_default()
            ),
            add_time: utils::now_seconds() as i32,
            admin_name: String::new(),
            refund_money: Decimal::ZERO,
            refund_type: 0,
            refund_desc: String::new(),
            user_name: username,
            return_pic: Some(param.return_pic_json()),
            shop_id: None,
            vendor_id: None,
        };
        self.repos.trade.aftersales_client.insert_aftersales_log(&log).await?;
        Ok(())
    }

    // ————————————————————— ⑩ 取消申请 —————————————————————

    /// `POST /api/user/aftersales/cancel`（Java `AfterSalesController.cancel`
    /// → `AftersalesServiceImpl#cancel`）。
    pub async fn cancel(&self, param: &AftersalesCancelParam, user_id: i32) -> Result<()> {
        let aftersale_id = param
            .aftersale_id
            .ok_or_else(|| Error::Validation("售后id不能为空".into()))?;
        let mut a = self
            .repos.trade.aftersales_client
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_RECORD_NOT_EXIST))?;
        if a.status != aftersales_status::IN_REVIEW {
            return Err(Error::biz_msg(ERR_STATUS_CANNOT_CANCEL));
        }
        a.status = aftersales_status::CANCEL;
        self.repos.trade.aftersales_client.update_aftersales(&a).await?;

        let username = self.repos.trade.aftersales_client.find_username(user_id).await?;
        let log = AftersalesLog {
            log_id: 0,
            aftersale_id,
            log_info: "已取消".into(),
            add_time: utils::now_seconds() as i32,
            admin_name: String::new(),
            refund_money: Decimal::ZERO,
            refund_type: 0,
            refund_desc: String::new(),
            user_name: username,
            return_pic: Some(String::new()),
            shop_id: None,
            vendor_id: None,
        };
        self.repos.trade.aftersales_client.insert_aftersales_log(&log).await?;
        // Java 另有：账期订单恢复待结算、微信小程序订单同步（依赖未迁移服务）
        Ok(())
    }

    // ————————————————————— 内部：VO 组装 —————————————————————

    /// 按售后单批量组装明细 VO（对应 Java `AftersalesItemServiceImpl#getItemByIds`）。
    async fn build_item_vos(
        &self,
        order: &Order,
        items: &[AftersalesItem],
    ) -> Result<Vec<AftersalesItemVO>> {
        let order_items = self
            .repos.trade.aftersales_client
            .find_order_items_by_ids(
                &items.iter().filter_map(|i| i.order_item_id).collect::<Vec<_>>(),
            )
            .await?;
        let map: HashMap<i32, &OrderItem> =
            order_items.iter().map(|it| (it.item_id, it)).collect();
        Ok(items
            .iter()
            .map(|it| self.item_vo(it, it.order_item_id.and_then(|id| map.get(&id).copied()), Some(order)))
            .collect())
    }

    /// 按售后单组装明细 VO（取该售后单所属订单的税率）。
    async fn build_item_vos_by_order(
        &self,
        a: &Aftersales,
        items: &[AftersalesItem],
    ) -> Result<Vec<AftersalesItemVO>> {
        let order = match a.order_id {
            Some(id) => self.repos.trade.aftersales_client.find_order(id).await?,
            None => None,
        };
        let order_items = self
            .repos.trade.aftersales_client
            .find_order_items_by_ids(
                &items.iter().filter_map(|i| i.order_item_id).collect::<Vec<_>>(),
            )
            .await?;
        let map: HashMap<i32, &OrderItem> =
            order_items.iter().map(|it| (it.item_id, it)).collect();
        Ok(items
            .iter()
            .map(|it| self.item_vo(it, it.order_item_id.and_then(|id| map.get(&id).copied()), order.as_ref()))
            .collect())
    }

    /// 多订单混合明细（`getRecord`）：逐条取所属订单的税率。
    async fn build_item_vos_multi(&self, items: &[AftersalesItem]) -> Result<Vec<AftersalesItemVO>> {
        let order_items = self
            .repos.trade.aftersales_client
            .find_order_items_by_ids(
                &items.iter().filter_map(|i| i.order_item_id).collect::<Vec<_>>(),
            )
            .await?;
        let map: HashMap<i32, &OrderItem> =
            order_items.iter().map(|it| (it.item_id, it)).collect();
        let order_ids = distinct(order_items.iter().map(|it| it.order_id));
        let orders: HashMap<i32, Order> = self
            .repos.trade.aftersales_client
            .find_orders_by_ids(&order_ids)
            .await?
            .into_iter()
            .map(|o| (o.order_id, o))
            .collect();
        Ok(items
            .iter()
            .map(|it| {
                let oi = it.order_item_id.and_then(|id| map.get(&id).copied());
                let order = oi.and_then(|o| orders.get(&o.order_id));
                self.item_vo(it, oi, order)
            })
            .collect())
    }

    /// 单条售后明细 → VO。
    fn item_vo(
        &self,
        ai: &AftersalesItem,
        oi: Option<&OrderItem>,
        order: Option<&Order>,
    ) -> AftersalesItemVO {
        let mut vo = AftersalesItemVO {
            aftersales_item_id: ai.aftersales_item_id,
            order_item_id: ai.order_item_id,
            number: ai.number,
            aftersale_id: ai.aftersale_id,
            ..Default::default()
        };
        let Some(oi) = oi else {
            vo.tax_rate = Some(Decimal::ZERO);
            vo.tax_rate_amount = Some(Decimal::ZERO);
            vo.tax_total_amount = Some(Decimal::ZERO);
            return vo;
        };
        let tax_rate = order.map_or(Decimal::ZERO, |o| o.tax_rate.unwrap_or(Decimal::ZERO));
        let unit_tax = unit_tax_amount(oi.price, tax_rate);
        vo.quantity = Some(oi.quantity);
        vo.product_name = Some(oi.product_name.clone());
        vo.product_sn = Some(oi.product_sn.clone());
        vo.product_id = Some(oi.product_id);
        vo.pic_thumb = Some(oi.pic_thumb.clone());
        vo.order_sn = Some(oi.order_sn.clone());
        vo.order_id = Some(oi.order_id);
        vo.extra_sku_data = oi.extra_sku_data.clone();
        let sku_data = parse_sku_data(&oi.sku_data);
        vo.sku_value = Some(sku_value_of(&sku_data));
        vo.sku_data = sku_data;
        vo.tax_rate = Some(tax_rate);
        vo.tax_rate_amount = Some(unit_tax);
        vo.price = Some(oi.price + unit_tax);
        vo.tax_total_amount = Some(unit_tax * Decimal::from(ai.number.unwrap_or(0)));
        vo
    }

    /// 售后日志 VO（对应 Java `AftersalesLogServiceImpl#getList` +
    /// `translateForLogInfo`）。
    async fn build_log_vos(&self, aftersale_id: i32) -> Result<Vec<AftersalesLogVO>> {
        let rows: Vec<AftersalesLogRow> = self.repos.trade.aftersales_client.find_logs(aftersale_id).await?;
        Ok(rows
            .into_iter()
            .map(|r| AftersalesLogVO {
                log_id: r.log_id,
                admin_name: r.admin_name,
                aftersales_id: Some(r.aftersale_id.to_string()),
                return_pic: parse_return_pic(r.return_pic.as_deref()),
                // Java `translateForLogInfo` 依赖 `translatePackage`（未迁移）：
                // 单语言部署下 translate 为恒等，故原样下发
                log_info: r.log_info,
                refund_desc: r.refund_desc,
                refund_money: r.refund_money,
                refund_type: r.refund_type,
                user_name: r.user_name,
                add_time: fmt_time_secs_i64(i64::from(r.add_time)),
                shop_id: r.shop_id,
                vendor_id: r.vendor_id,
            })
            .collect())
    }
}

/// 新建售后申请的结果（Java `create` 返回 `void`）。
///
/// Java 侧前端在调用后自行刷新列表，因此这里额外回传状态便于联调排查；
/// 字段全部为新增的**只读回执**，不影响原契约的 `data = null`。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateAftersalesResult {
    pub aftersale_id: i32,
    pub aftersale_type: i32,
    pub status: i32,
    pub status_name: String,
    pub refund_amount: Decimal,
    /// 是否因「售后审核关闭」被系统自动审核
    pub auto_approved: bool,
}

/// 订单项上的「进行中」售后占用。
#[derive(Debug, Clone, Copy)]
struct AppliedItem {
    order_item_id: i32,
    number: i32,
    status: i32,
    aftersale_id: i32,
}

/// 售后单号：`yyyyMMddHHmmss` + 5 位数字（Java
/// `LocalDateTime.now().format("yyyyMMddHHmmss") + RandomUtil.randomNumbers(5)`）。
///
/// Rust 侧未引入 `rand`，后缀由毫秒时间戳派生以保证同一秒内多次申请不重号。
#[must_use]
pub fn aftersales_sn(now_secs: i32) -> String {
    use chrono::TimeZone;
    let ts = chrono::Local
        .timestamp_opt(i64::from(now_secs), 0)
        .single()
        .map(|dt| dt.format("%Y%m%d%H%M%S").to_string())
        .unwrap_or_default();
    let suffix = utils::now_millis().rem_euclid(100_000);
    format!("{ts}{suffix:05}")
}

/// 去重（保持首次出现顺序）。
fn distinct<I: Iterator<Item = i32>>(it: I) -> Vec<i32> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for v in it {
        if seen.insert(v) {
            out.push(v);
        }
    }
    out
}

/// 把 `Order` 的 `total_amount` 等字段暴露给测试（避免测试里重复解析）。
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn order(order_status: i32, shipping_status: i32, shipping_time: i32) -> Order {
        Order {
            order_id: 1,
            order_sn: "SN1".into(),
            user_id: 2,
            order_status,
            pay_status: PAY_STATUS_PAID,
            shipping_status,
            shipping_time,
            shipping_fee: dec("10.00"),
            total_amount: dec("110.00"),
            tax_rate: Some(dec("6.00")),
            add_time: 1_700_000_000,
            ..Default::default()
        }
    }

    fn order_item(item_id: i32, quantity: i32, price: &str) -> OrderItem {
        OrderItem {
            item_id,
            order_id: 1,
            quantity,
            price: dec(price),
            product_name: format!("P{item_id}"),
            ..Default::default()
        }
    }

    fn apply_item(item_id: i32, number: i32) -> AftersalesApplyItem {
        AftersalesApplyItem {
            order_item_id: Some(item_id),
            number: Some(number),
        }
    }

    // ————————————————— 金额计算 —————————————————

    #[test]
    fn suggest_refund_partial_sums_price_times_number() {
        let o = order(ORDER_STATUS_PROCESSING, SHIPPING_STATUS_SHIPPED, 1);
        let items = vec![order_item(1, 5, "20.00"), order_item(2, 2, "10.00")];
        let apply = vec![apply_item(1, 2), apply_item(2, 1)];
        // 20*2 + 10*1 = 50（非整单，且订单非「已确认」故不叠加附加规格）
        assert_eq!(suggest_refund_amount(&o, &items, &apply), dec("50.00"));
    }

    #[test]
    fn suggest_refund_full_order_subtracts_shipping() {
        let o = order(ORDER_STATUS_PROCESSING, SHIPPING_STATUS_SHIPPED, 1);
        let items = vec![order_item(1, 5, "20.00")];
        let apply = vec![apply_item(1, 5)];
        // 整单：total_amount - shipping_fee = 100
        assert_eq!(suggest_refund_amount(&o, &items, &apply), dec("100.00"));
    }

    #[test]
    fn suggest_refund_adds_extra_sku_price_only_when_confirmed() {
        // 购买 5 件、只申请 2 件（**部分**售后，避开「整单 → 订单总额−运费」分支，
        // 否则断言将落在 Java `isFullOrderRefund` 分支上）
        let items = vec![OrderItem {
            extra_sku_data: Some(r#"[{"attrPrice":3.50},{"attrPrice":1.50}]"#.into()),
            ..order_item(1, 5, "10.00")
        }];
        let apply = vec![apply_item(1, 2)];
        // 已确认待发货：叠加附加规格 (10 + 5) * 2 = 30
        let confirmed = order(ORDER_STATUS_CONFIRMED, SHIPPING_STATUS_PENDING, 0);
        assert_eq!(suggest_refund_amount(&confirmed, &items, &apply), dec("30.00"));
        // 其他状态：不叠加 → 20
        let processing = order(ORDER_STATUS_PROCESSING, SHIPPING_STATUS_SHIPPED, 1);
        assert_eq!(suggest_refund_amount(&processing, &items, &apply), dec("20.00"));
    }

    #[test]
    fn extra_sku_attr_price_sum_tolerates_bad_json() {
        assert_eq!(extra_sku_attr_price_sum(None), Decimal::ZERO);
        assert_eq!(extra_sku_attr_price_sum(Some("")), Decimal::ZERO);
        assert_eq!(extra_sku_attr_price_sum(Some("not json")), Decimal::ZERO);
        assert_eq!(extra_sku_attr_price_sum(Some("[]")), Decimal::ZERO);
        // 数字字符串同样可解析
        assert_eq!(
            extra_sku_attr_price_sum(Some(r#"[{"attrPrice":"2.25"}]"#)),
            dec("2.25")
        );
    }

    #[test]
    fn final_refund_amount_caps_at_base_plus_shipping() {
        let o = order(ORDER_STATUS_PROCESSING, SHIPPING_STATUS_SHIPPED, 1);
        // 传入金额高于基础额 → 封顶 base + 运费
        assert_eq!(
            final_refund_amount(&o, dec("50.00"), Some(dec("80.00")), true, false),
            dec("60.00")
        );
        // 运费为 0 → 原样返回
        let mut no_fee = o.clone();
        no_fee.shipping_fee = Decimal::ZERO;
        assert_eq!(
            final_refund_amount(&no_fee, dec("50.00"), Some(dec("80.00")), true, false),
            dec("80.00")
        );
    }

    #[test]
    fn final_refund_amount_adds_shipping_when_not_yet_shipped() {
        // 未发货 + 本单补齐整单 → 并入运费
        let o = order(ORDER_STATUS_CONFIRMED, SHIPPING_STATUS_PENDING, 0);
        assert_eq!(
            final_refund_amount(&o, dec("100.00"), None, true, false),
            dec("110.00")
        );
        // 不含本单已完整覆盖 → 不再并入运费
        assert_eq!(
            final_refund_amount(&o, dec("100.00"), None, true, true),
            dec("100.00")
        );
        // 未完整覆盖 → 不并入
        assert_eq!(
            final_refund_amount(&o, dec("50.00"), None, false, false),
            dec("50.00")
        );
    }

    #[test]
    fn final_refund_amount_adds_shipping_for_completed_unshipped() {
        // 已完成 + 已发货 + shipping_time = 0 也视为可并入运费（Java 特殊分支）
        let o = order(ORDER_STATUS_COMPLETED, SHIPPING_STATUS_SHIPPED, 0);
        assert_eq!(
            final_refund_amount(&o, dec("100.00"), None, true, false),
            dec("110.00")
        );
        // shipping_time > 0 时不在窗口内
        let o2 = order(ORDER_STATUS_COMPLETED, SHIPPING_STATUS_SHIPPED, 1_700_000_000);
        assert_eq!(
            final_refund_amount(&o2, dec("100.00"), None, true, false),
            dec("100.00")
        );
    }

    #[test]
    fn base_refund_amount_uses_fallback_when_no_items() {
        let o = order(ORDER_STATUS_PROCESSING, SHIPPING_STATUS_SHIPPED, 1);
        // 购买 5 件；第二段只申请 2 件 → **部分**售后，走「单价×数量」分支
        let items = vec![order_item(1, 5, "10.00")];
        assert_eq!(
            base_refund_amount(&o, &items, &[], dec("7.77")),
            dec("7.77")
        );
        let as_items = vec![AftersalesItem {
            order_item_id: Some(1),
            number: Some(2),
            ..Default::default()
        }];
        assert_eq!(
            base_refund_amount(&o, &items, &as_items, dec("7.77")),
            dec("20.00")
        );
    }

    #[test]
    fn calc_refund_product_amount_ceils_to_two_decimals() {
        let o = order(ORDER_STATUS_PROCESSING, SHIPPING_STATUS_SHIPPED, 1);
        let vos = vec![AftersalesItemVO {
            number: Some(3),
            price: Some(dec("1.005")),
            ..Default::default()
        }];
        // 3.015 → 向上取整 3.02
        assert_eq!(calc_refund_product_amount(&o, &vos), dec("3.02"));
        assert_eq!(calc_refund_product_amount(&o, &[]), Decimal::ZERO);
    }

    #[test]
    fn unit_tax_amount_matches_java_ceiling() {
        // 100 * 6% = 6.00
        assert_eq!(unit_tax_amount(dec("100.00"), dec("6.00")), dec("6.00"));
        // 1.00 * 6.5% = 0.065 → 0.07
        assert_eq!(unit_tax_amount(dec("1.00"), dec("6.50")), dec("0.07"));
        // 税率为 0 或单价为 0 → 0
        assert_eq!(unit_tax_amount(dec("100.00"), Decimal::ZERO), Decimal::ZERO);
        assert_eq!(unit_tax_amount(Decimal::ZERO, dec("6.00")), Decimal::ZERO);
    }

    #[test]
    fn mul_ceil2_matches_java_subtotal() {
        assert_eq!(mul_ceil2(dec("10.005"), 2), dec("20.01"));
        assert_eq!(mul_ceil2(dec("10.00"), 3), dec("30.00"));
    }

    // ————————————————— 数量 / 可申请判定 —————————————————

    #[test]
    fn can_apply_quantity_never_negative() {
        assert_eq!(can_apply_quantity(5, 2), 3);
        assert_eq!(can_apply_quantity(5, 5), 0);
        assert_eq!(can_apply_quantity(2, 5), 0);
    }

    #[test]
    fn aftersale_flag_follows_java_matrix() {
        // 无进行中售后 → 可申请
        assert_eq!(aftersale_flag(None, 5), 1);
        // 已完成：额度用尽 → 不可申请；还有余额 → 可申请
        assert_eq!(aftersale_flag(Some(aftersales_status::COMPLETE), 0), 0);
        assert_eq!(aftersale_flag(Some(aftersales_status::COMPLETE), 2), 1);
        // 已取消 / 审核未通过（额度已释放）→ 可申请
        assert_eq!(aftersale_flag(Some(aftersales_status::CANCEL), 0), 1);
        assert_eq!(aftersale_flag(Some(aftersales_status::REFUSE), 0), 1);
        // 其他进行中状态 → 不可申请
        assert_eq!(aftersale_flag(Some(aftersales_status::IN_REVIEW), 5), 0);
        assert_eq!(aftersale_flag(Some(aftersales_status::SEND_BACK), 5), 0);
    }

    // ————————————————— 原因配置 —————————————————

    #[test]
    fn reason_validation_matches_java() {
        let reasons: Vec<String> = DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
        // 开启 + 未传 → 「请选择原因」
        let err = validate_buyer_reason(None, 1, &reasons).unwrap_err();
        assert!(err.to_string().contains("请选择原因"));
        // 开启 + 不在列表 → 「所选原因已失效，请重新选择」
        let err = validate_buyer_reason(Some("乱填"), 1, &reasons).unwrap_err();
        assert!(err.to_string().contains("所选原因已失效"));
        // 开启 + 合法（含首尾空白会被 trim）
        assert_eq!(
            validate_buyer_reason(Some(" 其他 "), 1, &reasons).unwrap(),
            Some("其他".to_string())
        );
        // 关闭 → 原样（空 → None）
        assert_eq!(validate_buyer_reason(None, 0, &reasons).unwrap(), None);
        assert_eq!(
            validate_buyer_reason(Some("任意"), 0, &reasons).unwrap(),
            Some("任意".to_string())
        );
    }

    #[test]
    fn resolve_update_reason_keeps_stored_when_unchanged() {
        let reasons: Vec<String> = DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
        // 未传 → 保留原值
        assert_eq!(
            resolve_update_reason(Some("其他"), None, 1, &reasons).unwrap(),
            Some("其他".to_string())
        );
        // 传入与库中一致 → 保留原值
        assert_eq!(
            resolve_update_reason(Some("其他"), Some("其他"), 1, &reasons).unwrap(),
            Some("其他".to_string())
        );
        // 传入新值 → 走校验
        assert_eq!(
            resolve_update_reason(Some("其他"), Some("多拍/拍错/不喜欢"), 1, &reasons).unwrap(),
            Some("多拍/拍错/不喜欢".to_string())
        );
        assert!(resolve_update_reason(Some("其他"), Some("乱填"), 1, &reasons).is_err());
    }

    #[test]
    fn reason_config_parsing_defaults_on_bad_input() {
        let default: Vec<String> = DEFAULT_REASON_LIST.iter().map(ToString::to_string).collect();
        assert_eq!(parse_reason_list(None), default);
        assert_eq!(parse_reason_list(Some("not json")), default);
        assert_eq!(parse_reason_list(Some("{}")), default);
        // 元素非字符串 → 回退默认
        assert_eq!(parse_reason_list(Some("[1,2]")), default);
        // 重复 → 回退默认
        assert_eq!(parse_reason_list(Some(r#"["a","a"]"#)), default);
        // 合法自定义列表原样返回
        assert_eq!(
            parse_reason_list(Some(r#"["甲","乙"]"#)),
            vec!["甲".to_string(), "乙".to_string()]
        );
        // 开关解析
        assert_eq!(parse_reason_enabled(Some("0")), 0);
        assert_eq!(parse_reason_enabled(Some("1")), 1);
        assert_eq!(parse_reason_enabled(None), 1);
        assert_eq!(parse_reason_enabled(Some("乱")), 1);
    }

    // ————————————————— 文案 / 步骤 / 时间 —————————————————

    #[test]
    fn review_enabled_follows_config_default() {
        assert!(review_enabled(Some("1")));
        assert!(!review_enabled(Some("0")));
        assert!(!review_enabled(None)); // 调用方需显式传默认 "1"
        assert!(review_enabled(Some(" 1 ")));
    }

    #[test]
    fn step_status_for_review_and_cancel() {
        // 审核处理中：2 步（退货退款）/ 3 步（仅退款）
        let mut a = Aftersales {
            aftersale_type: aftersales_type::RETURN,
            status: aftersales_status::IN_REVIEW,
            add_time: 1_700_000_000,
            ..Default::default()
        };
        let s = build_step_status(&a);
        assert_eq!(s.current, 1);
        assert_eq!(s.status, STEP_STATUS_PROCESS);
        assert_eq!(s.steps.len(), 4);
        assert_eq!(s.steps[0].title, STEP_TITLE_SUBMIT);
        assert!(s.steps[1].active);
        assert_eq!(s.steps[1].description, STEP_DESC_REVIEWING);

        // 仅退款：第 3 步标题为「已完成」
        a.aftersale_type = aftersales_type::PAY_RETURN;
        let s = build_step_status(&a);
        assert_eq!(s.steps.len(), 3);
        assert_eq!(s.steps[2].title, STEP_TITLE_COMPLETED);

        // 待回寄：current = 2，第 3 步激活
        a.aftersale_type = aftersales_type::RETURN;
        a.status = aftersales_status::SEND_BACK;
        let s = build_step_status(&a);
        assert_eq!(s.current, 2);
        assert!(s.steps[2].active);
        assert_eq!(s.steps[1].description, STEP_DESC_WAIT_RETURN);

        // 已取消：仅退款保留 2 步，退货退款保留 1 步
        a.status = aftersales_status::CANCEL;
        let s = build_step_status(&a);
        assert_eq!(s.status, STEP_STATUS_FINISH);
        assert_eq!(s.steps.len(), 1);
        a.aftersale_type = aftersales_type::PAY_RETURN;
        let s = build_step_status(&a);
        assert_eq!(s.steps.len(), 2);
        assert_eq!(s.steps[1].title, STEP_TITLE_CANCELLED);
    }

    #[test]
    fn step_status_complete_downgrades_payreturn_to_finance_review() {
        let a = Aftersales {
            aftersale_type: aftersales_type::RETURN,
            status: aftersales_status::COMPLETE,
            add_time: 1_700_000_000,
            final_time: 1_700_100_000,
            ..Default::default()
        };
        let s = build_step_status(&a);
        assert_eq!(s.status, STEP_STATUS_FINISH);
        assert_eq!(s.current, 3);

        // 仅退款 + 已完成：Java 会查 RefundApply 降级为「财务审核中」
        let a = Aftersales {
            aftersale_type: aftersales_type::PAY_RETURN,
            ..a
        };
        let s = build_step_status(&a);
        assert_eq!(s.status, STEP_STATUS_PROCESS);
        assert_eq!(s.current, 1);
        assert_eq!(s.steps[1].description, STEP_DESC_FINANCE_REVIEWING);
    }

    #[test]
    fn auto_agree_end_time_gate() {
        // 状态不是待审核 → None
        assert_eq!(
            auto_agree_end_time(aftersales_status::SEND_BACK, 1_700_000_000, Some("1"), Some("3")),
            None
        );
        // 未开启自动同意退货 → None
        assert_eq!(
            auto_agree_end_time(aftersales_status::IN_REVIEW, 1_700_000_000, Some("0"), Some("3")),
            None
        );
        // 天数为空 / 非正 → None
        assert_eq!(
            auto_agree_end_time(aftersales_status::IN_REVIEW, 1_700_000_000, Some("1"), None),
            None
        );
        assert_eq!(
            auto_agree_end_time(aftersales_status::IN_REVIEW, 1_700_000_000, Some("1"), Some("0")),
            None
        );
        // 正常：add_time + 3 天
        let got =
            auto_agree_end_time(aftersales_status::IN_REVIEW, 1_700_000_000, Some("1"), Some("3"))
                .unwrap();
        assert_eq!(got.len(), 19);
        assert_eq!(got, fmt_time(1_700_000_000 + 3 * 86_400));
    }

    #[test]
    fn fmt_time_handles_zero_and_positive() {
        assert_eq!(fmt_time(0), "");
        assert_eq!(fmt_time(-1), "");
        assert_eq!(fmt_time(1_700_000_000).len(), 19);
        assert_eq!(fmt_time_opt(0), None);
        assert!(fmt_time_opt(1_700_000_000).is_some());
    }

    #[test]
    fn parse_pics_and_return_pic_tolerate_junk() {
        assert!(parse_pics("").is_empty());
        assert!(parse_pics("not json").is_empty());
        assert_eq!(parse_pics(r#"[{"picUrl":"a"}]"#).len(), 1);
        assert!(parse_return_pic(None).is_empty());
        assert!(parse_return_pic(Some("[]")).is_empty());
        assert_eq!(parse_return_pic(Some(r#"[{"picUrl":"a"}]"#)).len(), 1);
    }

    #[test]
    fn parse_sku_data_and_value() {
        assert!(parse_sku_data("[]").is_empty());
        assert!(parse_sku_data("").is_empty());
        let sku = parse_sku_data(r#"[{"name":"颜色","value":"红","attrPicThumb":"r.png"}]"#);
        assert_eq!(sku.len(), 1);
        assert_eq!(sku[0].name, "颜色");
        assert_eq!(sku_value_of(&sku), "颜色:红");
        // 多项以 | 连接
        let sku = parse_sku_data(r#"[{"name":"a","value":"1"},{"name":"b","value":"2"}]"#);
        assert_eq!(sku_value_of(&sku), "a:1|b:2");
    }

    #[test]
    fn aftersales_sn_shape() {
        let sn = aftersales_sn(1_700_000_000);
        assert_eq!(sn.len(), 19); // 14 位时间 + 5 位数字
    }

    #[test]
    fn covers_all_order_items_requires_full_quantity() {
        let order_items = vec![order_item(1, 3, "1.00"), order_item(2, 2, "1.00")];
        let full = vec![
            AftersalesItem {
                order_item_id: Some(1),
                number: Some(3),
                ..Default::default()
            },
            AftersalesItem {
                order_item_id: Some(2),
                number: Some(2),
                ..Default::default()
            },
        ];
        assert!(covers_all_order_items(&order_items, &full));
        // 少一件 → false
        assert!(!covers_all_order_items(&order_items, &full[..1]));
        // 空 → false
        assert!(!covers_all_order_items(&order_items, &[]));
        // 数量拆成多条也可覆盖
        let split = vec![
            AftersalesItem {
                order_item_id: Some(1),
                number: Some(1),
                ..Default::default()
            },
            AftersalesItem {
                order_item_id: Some(1),
                number: Some(2),
                ..Default::default()
            },
            AftersalesItem {
                order_item_id: Some(2),
                number: Some(2),
                ..Default::default()
            },
        ];
        assert!(covers_all_order_items(&order_items, &split));
    }

    #[test]
    fn can_submit_return_logistics_matches_java() {
        assert!(can_submit_return_logistics(aftersales_status::SEND_BACK));
        assert!(can_submit_return_logistics(aftersales_status::RETURNED));
        assert!(can_submit_return_logistics(
            aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
        ));
        assert!(!can_submit_return_logistics(aftersales_status::IN_REVIEW));
        assert!(!can_submit_return_logistics(aftersales_status::COMPLETE));
    }
}