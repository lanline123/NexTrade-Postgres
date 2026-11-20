//! 前台用户订单发票域（trade::order_invoice_client）DTO / VO / 纯函数。
//!
//! 对应原 Java `com.nextrade.api.controller.user.OrderInvoiceClientController`
//! （`/api/user/orderInvoice`，3 个端点：detail / create / update）。
//!
//! - `detail`：按发票 id + 当前会员查询 `order_invoice`，并回填订单快照（订单号/地址/金额）。
//! - `create`：校验「单位/个人名称不能为空」+ 订单归属 + 重复申请，落库 `status = 0`。
//! - `update`：仅 `status = 0`（待审核）可修改申请数据。
//! - 发票开具/状态流转（微信/百望等电子发票平台）未迁移 → 本域不实现开票动作。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// `请先登录`
pub const MSG_LOGIN_REQUIRED: &str = "请先登录";
/// `id不能为空`
pub const MSG_ID_REQUIRED: &str = "id不能为空";
/// `单位/个人名称不能为空`（Java `OrderInvoiceSaveParam.companyName @NotNull`）
pub const MSG_COMPANY_NAME_REQUIRED: &str = "单位/个人名称不能为空";
/// `订单不存在`
pub const MSG_ORDER_NOT_EXIST: &str = "订单不存在";
/// `发票不存在`
pub const MSG_INVOICE_NOT_EXIST: &str = "发票不存在";
/// `发票状态不允许修改`
pub const MSG_INVOICE_STATUS_INVALID: &str = "发票状态不允许修改";
/// `该订单已申请发票`
pub const MSG_INVOICE_ALREADY_APPLIED: &str = "该订单已申请发票";
/// 发票状态：待审核（Java `status = 0`）
pub const INVOICE_STATUS_PENDING: i32 = 0;

// ---------------------------------------------------------------------------
// 开票设置增强（Tigshop v5.8.65「线上开票」跟进移植，原版有·我方自研实现）
//
// 配置口径（biz_code 见 `models::system::config_client`）：
// - `canInvoice`：既有开关，语义明确化为「普通发票（invoice_type=1）开放开关」。
// - `invoiceAdded`：既有开关，语义明确化为「增值税专用发票（invoice_type=2）开放开关」。
//   不新增并行类型开关，避免新旧两套开关打架；存量行为不变：配置行缺失时
//   两类均视为开放（本域此前无类型校验），只有显式存为 0 才关闭对应类型。
// - `invoiceApplyNode`（新）：可开票节点。0=不限制（关闭节点校验），
//   1=支付后，2=确认收货后，3=订单完成后（默认，缺失/非法值回落 3）。
// - `invoiceApplyDeadlineDays`（新）：节点达成后 N 天内可申请，0=不限（默认）。
//
// 节点判定（按订单现有状态机真实字段，见 `entities::order::Order`）：
// - 支付后：`pay_status = 2`（已支付）；期限起算 = `pay_time`。
// - 确认收货后：`received_time > 0`；期限起算 = `received_time`。
//   本仓确认收货动作同时写 `order_status = 5` 与 `received_time = now`
//   （services/trade/order_admin.rs、settlement.rs），故 2 与 3 在数据上同批达成，
//   区别只在判定字段：2 看收货时间、3 看订单状态。
// - 订单完成后：`order_status = 5`（已完成）；期限起算 = `received_time`
//   （订单表无独立完成时间列，完成动作以 received_time 落点；为 0 的历史
//   完成单不起算期限、只判节点，避免误伤存量订单）。
// ---------------------------------------------------------------------------

/// 发票类型：普通发票。
pub const INVOICE_TYPE_NORMAL: i32 = 1;
/// 发票类型：增值税专用发票。
pub const INVOICE_TYPE_SPECIAL: i32 = 2;
/// 可开票节点：不限制。
pub const APPLY_NODE_UNLIMITED: i32 = 0;
/// 可开票节点：支付后。
pub const APPLY_NODE_AFTER_PAY: i32 = 1;
/// 可开票节点：确认收货后。
pub const APPLY_NODE_AFTER_RECEIVE: i32 = 2;
/// 可开票节点：订单完成后（默认）。
pub const APPLY_NODE_AFTER_COMPLETE: i32 = 3;
/// 支付状态：已支付（`entities::order::Order.pay_status`）。
pub const PAY_STATUS_PAID: i32 = 2;
/// 订单状态：已完成（`entities::order::Order.order_status`）。
pub const ORDER_STATUS_COMPLETED: i32 = 5;
/// 一天的秒数（期限换算）。
pub const SECONDS_PER_DAY: i64 = 86_400;

/// `普通发票暂未开放`
pub const MSG_NORMAL_INVOICE_CLOSED: &str = "普通发票暂未开放";
/// `增值税专用发票暂未开放`
pub const MSG_SPECIAL_INVOICE_CLOSED: &str = "增值税专用发票暂未开放";
/// `订单支付后才可申请发票`
pub const MSG_NODE_AFTER_PAY: &str = "订单支付后才可申请发票";
/// `确认收货后才可申请发票`
pub const MSG_NODE_AFTER_RECEIVE: &str = "确认收货后才可申请发票";
/// `订单完成后才可申请发票`
pub const MSG_NODE_AFTER_COMPLETE: &str = "订单完成后才可申请发票";
/// `已超过发票申请期限`
pub const MSG_APPLY_DEADLINE_EXCEEDED: &str = "已超过发票申请期限";

/// 开票申请规则配置（由 `config` 表原始字符串值解析）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvoiceApplyConfig {
    /// 普通发票开放（`canInvoice`；缺失=None，视为开放）。
    pub can_invoice: Option<i32>,
    /// 专票开放（`invoiceAdded`；缺失=None，视为开放）。
    pub invoice_added: Option<i32>,
    /// 可开票节点（见 `APPLY_NODE_*`）。
    pub apply_node: i32,
    /// 申请期限天数（0=不限）。
    pub deadline_days: i32,
}

impl Default for InvoiceApplyConfig {
    fn default() -> Self {
        Self {
            can_invoice: None,
            invoice_added: None,
            apply_node: APPLY_NODE_AFTER_COMPLETE,
            deadline_days: 0,
        }
    }
}

impl InvoiceApplyConfig {
    /// 由四个 biz_code 的原始值解析（任一缺失/非法按默认口径回落）。
    #[must_use]
    pub fn from_raw(
        can_invoice: Option<&str>,
        invoice_added: Option<&str>,
        apply_node: Option<&str>,
        deadline_days: Option<&str>,
    ) -> Self {
        let parse = |raw: Option<&str>| raw.and_then(|v| v.trim().parse::<i32>().ok());
        let node = parse(apply_node).unwrap_or(APPLY_NODE_AFTER_COMPLETE);
        Self {
            can_invoice: parse(can_invoice),
            invoice_added: parse(invoice_added),
            apply_node: if (APPLY_NODE_UNLIMITED..=APPLY_NODE_AFTER_COMPLETE).contains(&node) {
                node
            } else {
                APPLY_NODE_AFTER_COMPLETE
            },
            deadline_days: parse(deadline_days).unwrap_or(0).max(0),
        }
    }
}

/// 订单开票相关快照（规则校验入参，与 `entities::order::Order` 对应字段同名）。
#[derive(Debug, Clone, Copy, Default)]
pub struct InvoiceApplyOrderSnapshot {
    pub order_status: i32,
    pub pay_status: i32,
    pub pay_time: i32,
    pub received_time: i32,
}

/// 校验发票类型是否开放（显式配置为 0 才关闭；缺失视为开放，兼容存量）。
#[must_use]
pub fn validate_invoice_type_open(
    cfg: &InvoiceApplyConfig,
    invoice_type: i32,
) -> Result<(), String> {
    match invoice_type {
        INVOICE_TYPE_SPECIAL => {
            if cfg.invoice_added == Some(0) {
                return Err(MSG_SPECIAL_INVOICE_CLOSED.to_string());
            }
        }
        _ => {
            if cfg.can_invoice == Some(0) {
                return Err(MSG_NORMAL_INVOICE_CLOSED.to_string());
            }
        }
    }
    Ok(())
}

/// 节点是否达成 + 期限起算时间（秒）；节点不限时返回 `(true, 0)`。
#[must_use]
pub fn apply_node_reached(
    cfg: &InvoiceApplyConfig,
    order: &InvoiceApplyOrderSnapshot,
) -> (bool, i64) {
    match cfg.apply_node {
        APPLY_NODE_UNLIMITED => (true, 0),
        APPLY_NODE_AFTER_PAY => (
            order.pay_status == PAY_STATUS_PAID,
            i64::from(order.pay_time),
        ),
        APPLY_NODE_AFTER_RECEIVE => (
            order.received_time > 0,
            i64::from(order.received_time),
        ),
        _ => (
            order.order_status == ORDER_STATUS_COMPLETED,
            i64::from(order.received_time),
        ),
    }
}

/// 校验可开票节点 + 申请期限（`now` 为当前秒级时间戳）。
///
/// 期限起算点见模块头注释；起算时间为 0（无法确定，如历史完成单无
/// `received_time`）时只判节点、不判期限，避免误伤存量订单。
#[must_use]
pub fn validate_apply_node_and_deadline(
    cfg: &InvoiceApplyConfig,
    order: &InvoiceApplyOrderSnapshot,
    now: i64,
) -> Result<(), String> {
    let (reached, start) = apply_node_reached(cfg, order);
    if !reached {
        return Err(match cfg.apply_node {
            APPLY_NODE_AFTER_PAY => MSG_NODE_AFTER_PAY.to_string(),
            APPLY_NODE_AFTER_RECEIVE => MSG_NODE_AFTER_RECEIVE.to_string(),
            _ => MSG_NODE_AFTER_COMPLETE.to_string(),
        });
    }
    if cfg.deadline_days > 0 && start > 0 {
        let deadline = start.saturating_add(i64::from(cfg.deadline_days) * SECONDS_PER_DAY);
        if now > deadline {
            return Err(MSG_APPLY_DEADLINE_EXCEEDED.to_string());
        }
    }
    Ok(())
}

/// 申请全规则校验（类型开放 → 节点 → 期限），create/update 共用。
#[must_use]
pub fn validate_invoice_apply_rules(
    cfg: &InvoiceApplyConfig,
    invoice_type: i32,
    order: &InvoiceApplyOrderSnapshot,
    now: i64,
) -> Result<(), String> {
    validate_invoice_type_open(cfg, invoice_type)?;
    validate_apply_node_and_deadline(cfg, order, now)
}

/// 发票申请入参（Java `OrderInvoiceSaveParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderInvoiceSaveParam {
    pub id: Option<i32>,
    pub user_id: Option<i32>,
    pub order_id: Option<i32>,
    pub invoice_type: Option<i32>,
    pub status: Option<i32>,
    pub apply_reply: Option<String>,
    pub title_type: Option<i32>,
    pub company_code: Option<String>,
    /// @NotNull：单位/个人名称不能为空
    pub company_name: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_bank: Option<String>,
    pub company_account: Option<String>,
    pub invoice_content: Option<String>,
    pub invoice_no: Option<String>,
    pub amount: Option<Decimal>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub add_time: Option<i64>,
}

/// 发票修改入参（Java `OrderInvoiceEditParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderInvoiceEditParam {
    pub id: Option<i32>,
    pub user_id: Option<i32>,
    pub order_id: Option<i32>,
    pub invoice_type: Option<i32>,
    pub status: Option<i32>,
    pub apply_reply: Option<String>,
    pub title_type: Option<i32>,
    pub company_code: Option<String>,
    pub company_name: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_bank: Option<String>,
    pub company_account: Option<String>,
    pub invoice_content: Option<String>,
    pub invoice_no: Option<String>,
    pub amount: Option<Decimal>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub add_time: Option<i32>,
    pub invoice_attachment: Option<Vec<serde_json::Value>>,
}

/// 发票详情（Java `OrderInvoiceDetailVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderInvoiceDetailVO {
    pub id: i32,
    pub user_id: i32,
    pub order_id: i32,
    pub address: Option<String>,
    pub amount: Option<Decimal>,
    pub apply_reply: Option<String>,
    pub company_account: Option<String>,
    pub company_address: Option<String>,
    pub company_bank: Option<String>,
    pub company_code: Option<String>,
    pub company_name: Option<String>,
    pub company_phone: Option<String>,
    pub email: Option<String>,
    pub invoice_content: Option<String>,
    pub invoice_no: Option<String>,
    pub invoice_type: i32,
    pub invoice_type_name: Option<String>,
    pub mobile: Option<String>,
    pub order_sn: Option<String>,
    pub region_names: Vec<String>,
    pub status: i32,
    pub status_name: Option<String>,
    pub title_type: i32,
    pub title_type_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub user_address: Option<String>,
    pub invoice_attachment: Vec<serde_json::Value>,
}

/// 校验创建入参（Java `@Valid`：companyName @NotNull）。
#[must_use]
pub fn validate_create(param: &OrderInvoiceSaveParam) -> Result<(), String> {
    if param
        .company_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .is_none()
    {
        return Err(MSG_COMPANY_NAME_REQUIRED.to_string());
    }
    Ok(())
}

/// 校验修改入参（id 必填）。
#[must_use]
pub fn validate_edit(param: &OrderInvoiceEditParam) -> Result<i32, String> {
    param
        .id
        .filter(|id| *id > 0)
        .ok_or_else(|| MSG_ID_REQUIRED.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_requires_company_name() {
        let mut param = OrderInvoiceSaveParam::default();
        assert_eq!(
            validate_create(&param).unwrap_err(),
            MSG_COMPANY_NAME_REQUIRED
        );
        param.company_name = Some("  ".into());
        assert_eq!(
            validate_create(&param).unwrap_err(),
            MSG_COMPANY_NAME_REQUIRED
        );
        param.company_name = Some("某某公司".into());
        assert!(validate_create(&param).is_ok());
    }

    #[test]
    fn edit_requires_id() {
        let mut param = OrderInvoiceEditParam::default();
        assert_eq!(validate_edit(&param).unwrap_err(), MSG_ID_REQUIRED);
        param.id = Some(3);
        assert_eq!(validate_edit(&param).unwrap(), 3);
    }

    fn completed_order() -> InvoiceApplyOrderSnapshot {
        InvoiceApplyOrderSnapshot {
            order_status: ORDER_STATUS_COMPLETED,
            pay_status: PAY_STATUS_PAID,
            pay_time: 1_000_000,
            received_time: 1_100_000,
        }
    }

    #[test]
    fn apply_config_defaults_and_fallbacks() {
        // 全缺失 → 默认节点=订单完成后、期限不限、类型开放
        let cfg = InvoiceApplyConfig::from_raw(None, None, None, None);
        assert_eq!(cfg, InvoiceApplyConfig::default());
        // 非法节点回落默认、负期限回落 0
        let cfg = InvoiceApplyConfig::from_raw(Some("1"), Some("0"), Some("9"), Some("-3"));
        assert_eq!(cfg.can_invoice, Some(1));
        assert_eq!(cfg.invoice_added, Some(0));
        assert_eq!(cfg.apply_node, APPLY_NODE_AFTER_COMPLETE);
        assert_eq!(cfg.deadline_days, 0);
    }

    #[test]
    fn apply_rules_reject_when_type_closed() {
        let order = completed_order();
        // 普票被显式关闭
        let cfg = InvoiceApplyConfig::from_raw(Some("0"), None, Some("0"), None);
        assert_eq!(
            validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &order, 1_200_000)
                .unwrap_err(),
            MSG_NORMAL_INVOICE_CLOSED
        );
        // 专票被显式关闭，普票不受影响
        let cfg = InvoiceApplyConfig::from_raw(None, Some("0"), Some("0"), None);
        assert_eq!(
            validate_invoice_apply_rules(&cfg, INVOICE_TYPE_SPECIAL, &order, 1_200_000)
                .unwrap_err(),
            MSG_SPECIAL_INVOICE_CLOSED
        );
        assert!(validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &order, 1_200_000).is_ok());
        // 配置缺失时两类都开放（存量兼容）
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("0"), None);
        assert!(validate_invoice_apply_rules(&cfg, INVOICE_TYPE_SPECIAL, &order, 1_200_000).is_ok());
    }

    #[test]
    fn apply_rules_reject_before_node_reached() {
        // 默认节点=订单完成后：已支付未完成 → 未到节点
        let cfg = InvoiceApplyConfig::default();
        let mut order = completed_order();
        order.order_status = 2;
        assert_eq!(
            validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &order, 1_200_000)
                .unwrap_err(),
            MSG_NODE_AFTER_COMPLETE
        );
        // 支付后节点：未支付 → 未到节点；已支付即达
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("1"), None);
        let mut unpaid = completed_order();
        unpaid.pay_status = 0;
        assert_eq!(
            validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &unpaid, 1_200_000)
                .unwrap_err(),
            MSG_NODE_AFTER_PAY
        );
        assert!(validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &order, 1_200_000).is_ok());
        // 确认收货节点：received_time=0 → 未到节点
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("2"), None);
        let mut unreceived = completed_order();
        unreceived.received_time = 0;
        assert_eq!(
            validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &unreceived, 1_200_000)
                .unwrap_err(),
            MSG_NODE_AFTER_RECEIVE
        );
        // 节点=不限制：任何状态可申请
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("0"), None);
        assert!(validate_invoice_apply_rules(
            &cfg,
            INVOICE_TYPE_NORMAL,
            &InvoiceApplyOrderSnapshot::default(),
            1_200_000
        )
        .is_ok());
    }

    #[test]
    fn apply_rules_deadline_enforced_and_zero_unlimited() {
        let order = completed_order(); // received_time = 1_100_000
        // 期限 1 天：起算后 1 天内可申请，超过即拒
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("3"), Some("1"));
        assert!(validate_invoice_apply_rules(
            &cfg,
            INVOICE_TYPE_NORMAL,
            &order,
            1_100_000 + SECONDS_PER_DAY
        )
        .is_ok());
        assert_eq!(
            validate_invoice_apply_rules(
                &cfg,
                INVOICE_TYPE_NORMAL,
                &order,
                1_100_000 + SECONDS_PER_DAY + 1
            )
            .unwrap_err(),
            MSG_APPLY_DEADLINE_EXCEEDED
        );
        // 期限=0 不限：多久都可申请
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("3"), Some("0"));
        assert!(validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &order, i64::MAX - 1).is_ok());
        // 起算时间缺失（历史完成单 received_time=0）→ 只判节点不判期限
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("3"), Some("1"));
        let mut legacy = completed_order();
        legacy.received_time = 0;
        assert!(validate_invoice_apply_rules(&cfg, INVOICE_TYPE_NORMAL, &legacy, i64::MAX - 1).is_ok());
        // 支付后节点以 pay_time 起算期限
        let cfg = InvoiceApplyConfig::from_raw(None, None, Some("1"), Some("1"));
        assert_eq!(
            validate_invoice_apply_rules(
                &cfg,
                INVOICE_TYPE_NORMAL,
                &order,
                1_000_000 + SECONDS_PER_DAY + 1
            )
            .unwrap_err(),
            MSG_APPLY_DEADLINE_EXCEEDED
        );
    }

    #[test]
    fn params_accept_java_names() {
        let param: OrderInvoiceSaveParam = serde_json::from_str(
            r#"{"orderId":5,"invoiceType":1,"titleType":2,"companyName":"公司","amount":10.5}"#,
        )
        .unwrap();
        assert_eq!(param.order_id, Some(5));
        assert_eq!(param.company_name.as_deref(), Some("公司"));
        let vo = OrderInvoiceDetailVO {
            id: 1,
            status: INVOICE_STATUS_PENDING,
            ..Default::default()
        };
        let value = serde_json::to_value(&vo).unwrap();
        assert_eq!(value["invoiceType"], 0);
        assert_eq!(value["status"], 0);
    }
}
