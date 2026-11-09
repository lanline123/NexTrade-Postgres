//! 售后管理服务（trade::aftersales）：申请类型/退换货状态下拉、售后列表、售后详情、
//! 售后创建、售后审核（同意/驳回）、售后确认收货、售后拒收、售后反馈记录、售后完结。
//!
//! 对应原 Java `com.nextrade.service.order.impl.AftersalesServiceImpl`（2753 行）
//! 中 `list` / `detail` / `create` / `update` / `receive`（`completeReturnAftersales`）
//! / `rejectReceive` / `submitFeedbackRecord` / `complete` 八条链路，
//! 以及 `AftersalesController` 的 10 个端点。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：筛选条件在仓储层按 Java 的 `.eq(...)` 同序绑定；关联数据（售后明细 /
//!   订单 / 订单明细 / 退款申请）全部**批量预取**，与 Java 的 N+1 消除改造一致。
//!   供应商后台的状态白名单（21/4/22/6）与「非法状态必然查空」的口径逐字保留。
//! - `detail`：状态头/副文案按 类型/状态/供应商/关闭子类型 分支
//!   （`AftersalesStatusEnum.getAdminStatusName` / `getAdminStatusDescription`）；
//!   建议退款金额逐条复刻 `calculateBaseRefundAmount` → `calculateFinalRefundAmount`
//!   → 整单/部分运费分摊 → `refundTaxAmount`。
//! - `update`：状态门（仅 1/21 可审）+ `validParam` 校验 + 状态流转
//!   （退货退款 1→4、仅退款 1→6）+ 退款金额按申请明细重算 + 协商记录。
//! - `receive`：状态门（非供应商端仅 5 可收）+ 退款金额重算 + 6 状态 + 完结时间 + 协商记录。
//! - `rejectReceive`：状态门与确认收货同节点 + 3 状态 + `close_type = 2` + 协商记录。
//! - `complete`：仅 3（审核未通过）可关单 → 7 状态 + 协商记录。
//! - `record`：写协商记录；状态为 4（待回寄）且登录管理员有效时回写物流信息与处理时间。
//! - `create`：**裸插入**（忠实复刻 Java 该端点的语义，详见方法注释）。
//!
//! **仍未迁移（本切片明确不做，已在 `uncertain_points` 中列明）**：
//! - **财务退款执行**：退款申请与 `refund_log` 已创建；`financialReview != 1`
//!   时自动把申请置为处理中。实际渠道退款执行（微信/支付宝/线下）仍由支付域
//!   负责；退款结算单/RabbitMQ 退款结算路由未迁移。
//! - **整单售后退回优惠券**：仅退款审核通过且整单被售后覆盖时，
//!   释放订单/父单的 `user_coupon` 占用（与 Java 一致）。
//! - **供应商审核环节**：已读取 `vendor.after_sales_review`（仅
//!   `nextrade.is-vendor=1` 的供应商退货售后生效）；未开启时按 Java 跳过供应商审核。
//! - **站内消息 / 语音播报**（`AdminMsgService`）、**微信直播同步**
//!   （`wechatLiveOrderSyncService`）、**订单日志**（`orderLogService`）、
//!   **后台操作日志**（`adminLogService`）、**自动同意退货/自动确认收货延时队列**、
//!   **拼团团长退款解散整团**、**账期订单状态回滚**。
//! - **退货地址自动回填**：已实现 `vendor.receive_info` → `shop.receive_info` →
//!   平台 `receiveSender/receiveMobile/receive*` 三级兜底。
//! - **多语言翻译**（`TranslatePackage` / `MessageHelper`）：文案按中文原文返回。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_common::utils;
use nextrade_core::entities::{
    Aftersales, AftersalesItem, AftersalesLog, Order, OrderItem, RefundApply, RefundLog,
};
use nextrade_core::models::payment::refund_apply::RefundApplyStatus;
use nextrade_core::models::trade::aftersales::{
    admin_type, aftersales_status, aftersales_type, is_vendor_deployment,
    resolve_auto_agree_end_time, AfterSalesCompleteParam, AfterSalesRejectReceiveParam,
    AftersalesCreateParam, AftersalesEditParam, AftersalesItemVO, AftersalesListQuery,
    AftersalesLogVO, AftersalesReceiveParam, AftersalesRecordCreateParam, AftersalesVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::aftersales::{AftersalesItemRow, AftersalesRepository};
use nextrade_infrastructure::db::Repositories;

/// 金额/税费的小数位（Java `RoundingMode.CEILING` + `scale = 2`）。
const AMOUNT_SCALE: u32 = 2;

/// 向上取整到 2 位小数（对应 Java `setScale(2, RoundingMode.CEILING)`）。
fn ceil2(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(AMOUNT_SCALE, RoundingStrategy::ToPositiveInfinity)
}

/// 「单价 × 数量」再向上取整（对应 Java `unitPrice.multiply(qty).setScale(2, CEILING)`）。
fn ceil2_mul(unit: Decimal, qty: i32) -> Decimal {
    ceil2(unit * Decimal::from(qty))
}

/// 单价税费：`单价 × 税率 ÷ 100`，2 位小数向上取整（对应 Java
/// `AftersalesItemServiceImpl#getItemByIds` 中的 `unitTaxAmount`）。
#[must_use]
pub fn unit_tax_amount(price: Decimal, tax_rate: Decimal) -> Decimal {
    if price <= Decimal::ZERO || tax_rate <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    ceil2(price * tax_rate / Decimal::from(100))
}

/// 解析 `order_item.extra_sku_data`（JSON 数组）中 `attrPrice` 之和。
///
/// 对应 Java `JSONUtil.toList(extraSkuData, OrderExtraSkuDataDTO.class)` 后对
/// `attrPrice` 求和；非法 JSON / 缺字段时按 0 累加（与 Hutool 的容错行为一致）。
fn extra_sku_amount(raw: Option<&str>) -> Decimal {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Decimal::ZERO;
    };
    let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(raw) else {
        return Decimal::ZERO;
    };
    arr.iter()
        .filter_map(|v| v.get("attrPrice"))
        .filter_map(|v| match v {
            serde_json::Value::Number(n) => n.to_string().parse::<Decimal>().ok(),
            serde_json::Value::String(s) => s.trim().parse::<Decimal>().ok(),
            _ => None,
        })
        .fold(Decimal::ZERO, |acc, v| acc + v)
}

/// 售后明细的展示单价（Java：`order_item.price + unitTaxAmount`）。
fn display_unit_price(order_item: &OrderItem, tax_rate: Decimal) -> Decimal {
    order_item.price + unit_tax_amount(order_item.price, tax_rate)
}

/// 退款计算所用的「售后明细 + 订单明细」对。
pub type ItemPairs<'a> = [(&'a AftersalesItem, &'a OrderItem)];

/// 按「单价 + 附加规格」× 数量求和（对应 Java `calculateBaseRefundAmount` /
/// `calcRefundProductAmount` 的循环体；已确认订单把 `extraSkuData` 的附加价计入单价）。
fn sum_item_amounts(order_status: i32, pairs: &ItemPairs<'_>) -> Decimal {
    pairs
        .iter()
        .map(|(item, order_item)| {
            let mut unit_price = order_item.price;
            if order_status == nextrade_core::models::trade::order_admin::OrderStatus::Confirmed as i32 {
                unit_price += extra_sku_amount(order_item.extra_sku_data.as_deref());
            }
            ceil2_mul(unit_price, item.number.unwrap_or(0))
        })
        .fold(Decimal::ZERO, |acc, v| acc + v)
}

/// 是否「整单售后」（对应 Java 的 `isFullOrderRefund` 判定）。
///
/// Java 的判定是 `orderItems.size() == aftersalesItems.size()` 且每个 `item_id`
/// 的购买数量与售后数量相等。前者是**条数**比较，当订单行被多次部分售后时该比较会
/// 失真；本实现改用「每个订单行的售后数量合计 >= 购买数量」（见 `uncertain_points`）。
#[must_use]
pub fn is_full_order_refund(order_items: &[OrderItem], pairs: &ItemPairs<'_>) -> bool {
    if order_items.is_empty() || pairs.is_empty() {
        return false;
    }
    let mut per_item: HashMap<i32, i32> = HashMap::new();
    for (item, order_item) in pairs {
        if item.order_item_id == Some(order_item.item_id) {
            *per_item.entry(order_item.item_id).or_insert(0) += item.number.unwrap_or(0);
        }
    }
    order_items
        .iter()
        .all(|oi| per_item.get(&oi.item_id).copied().unwrap_or(0) >= oi.quantity)
}

/// 基础退款额（对应 Java `calculateBaseRefundAmount`）。
///
/// 无售后明细时回退到售后单已存的 `refund_amount`；整单售后取
/// `订单总额 - 运费`；否则按明细逐行累加。
#[must_use]
pub fn calculate_base_refund_amount(
    order: &Order,
    aftersales: &Aftersales,
    order_items: &[OrderItem],
    pairs: &ItemPairs<'_>,
) -> Decimal {
    if pairs.is_empty() {
        return aftersales.refund_amount;
    }
    if is_full_order_refund(order_items, pairs) {
        return order.total_amount - order.shipping_fee;
    }
    sum_item_amounts(order.order_status, pairs)
}

/// 是否「覆盖了整单售后商品」的通用判定（对应 Java `coversAllOrderItems`）。
fn covers_all_order_items(order_items: &[OrderItem], pairs: &ItemPairs<'_>) -> bool {
    if pairs.is_empty() {
        return false;
    }
    let mut per_item: HashMap<i32, i32> = HashMap::new();
    for (item, order_item) in pairs {
        if item.order_item_id == Some(order_item.item_id) {
            *per_item.entry(order_item.item_id).or_insert(0) += item.number.unwrap_or(0);
        }
    }
    order_items
        .iter()
        .all(|oi| per_item.get(&oi.item_id).copied().unwrap_or(0) >= oi.quantity)
}

/// `calculateFinalRefundAmount` / `isFullRefundCoverage` 所需的上下文。
///
/// 全部借用统一到同一生命周期 `'a`（调用点都在同一次请求处理作用域内），
/// 因此用切片而非 `&Vec<_>`，避免「外层引用」与「内层元素」生命周期被强行绑死。
pub struct RefundContext<'a> {
    /// 订单
    pub order: &'a Order,
    /// 订单全部明细
    pub order_items: &'a [OrderItem],
    /// 当前售后单
    pub current_aftersales: &'a Aftersales,
    /// 当前售后的明细（与订单明细配对后）
    pub current_pairs: &'a [(&'a AftersalesItem, &'a OrderItem)],
    /// 该订单下**已完成**的其他售后单的明细（已与订单明细配对）
    pub completed_other_pairs: &'a [(&'a AftersalesItem, &'a OrderItem)],
    /// 该订单下**更早创建且未取消/驳回**的其他售后明细（已与订单明细配对）
    ///
    /// 该字段对应 Java `isFullRefundCoverage` 里「其他已完成售后」的取数口径；
    /// Java 源码把 `validStatus()` 与「已完成」两个口径混用，此处保留字段以对齐语义。
    pub earlier_other_pairs: &'a [(&'a AftersalesItem, &'a OrderItem)],
    /// 该订单其他已处理/处理中的退款申请金额合计
    /// （Java `isShippingAlreadyRefunded` 里的 `completedRefundTotal`）
    pub other_processed_refund_total: Decimal,
}

/// 退款覆盖判定（对应 Java `isFullRefundCoverage`）。
///
/// 覆盖集合 = 其他已完成售后的明细（`include_current` 为真时再并上当前售后明细）。
#[must_use]
pub fn is_full_refund_coverage(ctx: &RefundContext<'_>, include_current: bool) -> bool {
    if ctx.order_items.is_empty() {
        return false;
    }
    let mut merged: Vec<(&AftersalesItem, &OrderItem)> = ctx.completed_other_pairs.to_vec();
    if include_current {
        merged.extend(ctx.current_pairs.iter().copied());
    }
    covers_all_order_items(ctx.order_items, &merged)
}

/// 运费是否已在其他售后的退款里退过（对应 Java `isShippingAlreadyRefunded`）。
#[must_use]
pub fn is_shipping_already_refunded(
    order: &Order,
    current_refund_amount: Decimal,
    other_processed_refund_total: Decimal,
) -> bool {
    if order.shipping_fee <= Decimal::ZERO {
        return false;
    }
    (current_refund_amount + order.shipping_fee + other_processed_refund_total) > order.total_amount
}

/// 计算最终退款金额（对应 Java `calculateFinalRefundAmount`）。
///
/// - 无运费：直接返回入参金额（`None` 时回退基础退款额）；
/// - 入参金额 > 基础退款额：封顶为 `基础退款额 + 运费`；
/// - 待发货、或「已完成 + 已收货状态 + 无发货时间」且本次覆盖整单：追加运费，
///   但若运费已在其他退款里退过则不再追加。
#[must_use]
pub fn calculate_final_refund_amount(
    ctx: &RefundContext<'_>,
    input_refund_amount: Option<Decimal>,
    include_current_aftersales: bool,
) -> Decimal {
    let base = calculate_base_refund_amount(
        ctx.order,
        ctx.current_aftersales,
        ctx.order_items,
        ctx.current_pairs,
    );
    let refund_amount = input_refund_amount.unwrap_or(base);
    let shipping_fee = ctx.order.shipping_fee;
    if shipping_fee <= Decimal::ZERO {
        return refund_amount;
    }
    if refund_amount > base {
        return refund_amount.min(base + shipping_fee);
    }

    let pending = nextrade_core::models::trade::order_admin::ShippingStatus::Pending as i32;
    let shipped = nextrade_core::models::trade::order_admin::ShippingStatus::Shipped as i32;
    let completed = nextrade_core::models::trade::order_admin::OrderStatus::Completed as i32;

    let mut should_add_shipping = false;
    if ctx.order.shipping_status == pending {
        should_add_shipping = is_full_refund_coverage(ctx, include_current_aftersales);
    } else if ctx.order.order_status == completed
        && ctx.order.shipping_status == shipped
        && ctx.order.shipping_time <= 0
    {
        should_add_shipping = is_full_refund_coverage(ctx, include_current_aftersales);
    }
    if should_add_shipping {
        should_add_shipping = !is_shipping_already_refunded(
            ctx.order,
            refund_amount,
            ctx.other_processed_refund_total,
        );
    }
    if should_add_shipping {
        refund_amount + shipping_fee
    } else {
        refund_amount
    }
}

/// 退款商品金额（含税），逐条对照 Java `calcRefundProductAmount`：
/// 按 `(商品单价 + 附加规格) × 退款数量` 累加后向上取整到 2 位小数；
/// 整单售后时改用 `订单总额 - 运费`。
#[must_use]
pub fn calc_refund_product_amount(
    order: &Order,
    order_items: &[OrderItem],
    pairs: &ItemPairs<'_>,
) -> Decimal {
    if pairs.is_empty() {
        return Decimal::ZERO;
    }
    if is_full_order_refund(order_items, pairs) {
        return order.total_amount - order.shipping_fee;
    }
    ceil2(sum_item_amounts(order.order_status, pairs))
}

/// 构造售后日志（协商记录）实体。
#[allow(clippy::too_many_arguments)]
fn build_log(
    aftersale_id: i32,
    log_info: String,
    now: i32,
    user_name: Option<String>,
    admin_name: String,
    shop_id: Option<i32>,
    vendor_id: Option<i32>,
    refund_money: Decimal,
    return_pic: Option<String>,
) -> AftersalesLog {
    AftersalesLog {
        log_id: 0,
        aftersale_id,
        log_info,
        add_time: now,
        admin_name,
        refund_money,
        refund_type: 0,
        refund_desc: String::new(),
        user_name,
        return_pic,
        shop_id,
        vendor_id,
    }
}

/// 解析 `pics` / `return_pic` 的 JSON 文本（非法/空时返回空表）。
fn parse_json_list(raw: Option<&str>) -> Vec<serde_json::Value> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<serde_json::Value>>(raw).unwrap_or_default()
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`：0 或负数返回空串）。
fn fmt_time(secs: i32) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

// ————————————————————— 枚举下拉（纯函数，便于单测） —————————————————————

/// 申请类型（对应 Java `AftersalesController.applyType`）。
///
/// 返回 `{1: "退货/退款", 2: "仅退款"}`（Java 用 `HashMap`，此处用 `BTreeMap`
/// 让 JSON 键有序，值与 Java 完全一致）。
#[must_use]
pub fn apply_type() -> BTreeMap<i32, String> {
    let mut map = BTreeMap::new();
    map.insert(aftersales_type::RETURN, "退货/退款".to_string());
    map.insert(aftersales_type::PAY_RETURN, "仅退款".to_string());
    map
}

/// 退换货状态（对应 Java `AftersalesController.returnGoodsStatus`）。
///
/// Java 逻辑：
/// 1. 基础表 1..7（注意 Java 源码里 4 被赋值了两次，最终文案是「待回寄」）；
/// 2. `nextradeProperties.isVendor == 1` 时追加 `21/22`；
/// 3. `getAdminType() == "vendor"` 时**直接返回** `{21, 22, 6}` 三元素表。
///
/// 当前部署 `nextrade.is-vendor: 0`（见 `models::nextrade_core::models::trade::aftersales::IS_VENDOR`），故第 2 步不生效。
/// `x-admin-type` 缺失时 Java 会在 `AdminTypeEnum.fromCode(null)` 抛
/// `IllegalArgumentException` 并被全局异常处理包成错误响应；此处在 `None` 时按
/// 「非供应商」继续，避免整个下拉不可用（见 `uncertain_points`）。
#[must_use]
pub fn return_goods_status(admin_type_code: Option<&str>) -> BTreeMap<i32, String> {
    let mut reason = BTreeMap::new();
    reason.insert(1, "审核处理中".to_string());
    reason.insert(2, "审核通过".to_string());
    reason.insert(3, "审核未通过".to_string());
    reason.insert(4, "待回寄".to_string());
    reason.insert(5, "待商家收货".to_string());
    reason.insert(6, "已完成".to_string());
    reason.insert(7, "已取消".to_string());

    if is_vendor_deployment() {
        reason.insert(21, "供应商待审核".to_string());
        reason.insert(22, "供应商待收货".to_string());
    }

    if admin_type_code == Some(admin_type::VENDOR) {
        let mut vendor_only = BTreeMap::new();
        vendor_only.insert(21, "供应商待审核".to_string());
        vendor_only.insert(22, "供应商待收货".to_string());
        vendor_only.insert(6, "已完成".to_string());
        return vendor_only;
    }
    reason
}

/// 售后管理域服务（挂载于 `AdminState.aftersales`）。
pub struct AftersalesService {
    repos: Arc<Repositories>,
}

impl AftersalesService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 取本域仓储（`Repositories` 的 `aftersales` 字段由主会话注册后可用）。
    fn repo(&self) -> &AftersalesRepository {
        &self.repos.trade.aftersales
    }

    /// 申请类型（对应 Java `AftersalesController.applyType`）。
    #[must_use]
    pub fn apply_type(&self) -> BTreeMap<i32, String> {
        apply_type()
    }

    /// 退换货状态（对应 Java `AftersalesController.returnGoodsStatus`）。
    #[must_use]
    pub fn return_goods_status(&self, admin_type_code: Option<&str>) -> BTreeMap<i32, String> {
        return_goods_status(admin_type_code)
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建售后单（对应 Java `AftersalesController.create` /
    /// `AftersalesServiceImpl#create`）。
    ///
    /// **忠实复刻 Java 的「裸插入」语义**：只把 DTO 中与 `Aftersales` 同名的字段拷过去
    /// 后 `save()`，**不校验订单、不写售后明细、不写协商记录、不计算退款金额、
    /// 不生成售后单号、不设置状态与添加时间**（Java 该端点确实如此；完整申请流程在
    /// `afterSalesApply`，由前台 `/api/user/aftersales/create` 触发）。
    ///
    /// 因此落库行的 `status` / `add_time` / `shop_id` / `vendor_id` 均为库默认值，
    /// 与 Java（MyBatis-Plus 只写非 null 字段）一致。该语义很可能并非业务预期，
    /// 已在 `uncertain_points` 中列明。
    ///
    /// **未迁移依赖**：微信直播订单同步（`wechatLiveOrderSyncService.onAftersaleChanged`）。
    pub async fn create(&self, param: &AftersalesCreateParam) -> Result<i32> {
        let aftersales = Aftersales {
            aftersale_id: 0,
            aftersale_type: param.aftersale_type.unwrap_or_default(),
            status: 0,
            pics: param
                .pics
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .unwrap_or_default(),
            description: param.description.clone().unwrap_or_default(),
            reply: String::new(),
            add_time: 0,
            tracking_no: String::new(),
            logistics_name: String::new(),
            return_address: None,
            aftersale_reason: param.aftersale_reason.clone(),
            aftersales_sn: None,
            order_id: param.order_id,
            user_id: param.user_id,
            refund_amount: param.refund_amount.unwrap_or(Decimal::ZERO),
            shop_id: 0,
            audit_time: 0,
            deal_time: 0,
            final_time: 0,
            return_goods_tip: String::new(),
            vendor_id: None,
            close_type: 0,
        };
        self.repo().insert_aftersales(&aftersales).await
    }

    // ————————————————————— 列表 —————————————————————

    /// 售后分页（对应 Java `AftersalesController.list` /
    /// `AftersalesServiceImpl#list`）。
    ///
    /// `shop_id` / `vendor_id` / `is_vendor_admin` 由路由层按请求头
    /// （`x-shop-id` / `x-vendor-id` / `x-admin-type`）解析后传入。
    pub async fn list(
        &self,
        q: &AftersalesListQuery,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        is_vendor_admin: bool,
    ) -> Result<PageResult<AftersalesVO>> {
        // 微信直播筛选依赖 `wechat_live_order_rel` 直播域（未迁移）：
        // 显式报错优于静默忽略（否则会返回全量数据，比 Java 的「查不到」更危险）
        if q.filter_by_live() {
            return Err(Error::Validation(
                "暂不支持按微信直播维度筛选售后（微信直播订单关系域未迁移）".into(),
            ));
        }

        let page = q.page_query();

        // 关键词 / 交易单号 → 订单ID集合（Java 用 `like` 查 order 表）
        let mut search_order_ids: Option<Vec<i32>> = None;
        if let Some(keyword) = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let ids = self.repo().find_order_ids_by_sn(keyword).await?;
            if ids.is_empty() {
                return Ok(PageResult::empty(&page));
            }
            search_order_ids = Some(ids);
        }
        if let Some(out_trade_no) = q
            .out_trade_no
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let ids = self
                .repo()
                .find_order_ids_by_out_trade_no(out_trade_no)
                .await?;
            if ids.is_empty() {
                return Ok(PageResult::empty(&page));
            }
            search_order_ids = Some(match search_order_ids {
                None => ids,
                Some(prev) => {
                    let set: HashSet<i32> = ids.into_iter().collect();
                    let filtered: Vec<i32> =
                        prev.into_iter().filter(|id| set.contains(id)).collect();
                    if filtered.is_empty() {
                        return Ok(PageResult::empty(&page));
                    }
                    filtered
                }
            });
        }

        let (total, rows) = self
            .repo()
            .paginate_aftersales(
                q,
                shop_id,
                vendor_id,
                is_vendor_admin,
                search_order_ids.as_deref(),
            )
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let aftersale_ids: Vec<i32> = rows.iter().map(|a| a.aftersale_id).collect();

        // 售后明细（批量）
        let item_rows = self
            .repo()
            .find_items_by_aftersale_ids(&aftersale_ids)
            .await?;
        let mut items_by_aftersale: HashMap<i32, Vec<AftersalesItemRow>> = HashMap::new();
        for row in item_rows {
            let key = row.item.aftersale_id.unwrap_or_default();
            items_by_aftersale.entry(key).or_default().push(row);
        }

        // 订单（批量）
        let order_ids: Vec<i32> = {
            let mut seen = HashSet::new();
            rows.iter()
                .filter_map(|a| a.order_id)
                .filter(|id| seen.insert(*id))
                .collect()
        };
        let orders = self.repo().find_orders(&order_ids).await?;
        let order_map: HashMap<i32, Order> = orders.into_iter().map(|o| (o.order_id, o)).collect();

        // 每个订单的全部明细（用于整单售后判定与建议退款金额）
        let mut order_items_map: HashMap<i32, Vec<OrderItem>> = HashMap::new();
        for oid in &order_ids {
            order_items_map.insert(*oid, self.repo().find_order_items_by_order(*oid).await?);
        }

        // 退款申请（批量取最新一笔）：驱动「待财务退款」「财务已拒绝」列表文案
        let refund_map = self
            .repo()
            .find_latest_refund_applies(&aftersale_ids)
            .await?;

        let now = utils::now_seconds() as i32;
        let mut list = Vec::with_capacity(rows.len());
        for a in &rows {
            let empty: Vec<AftersalesItemRow> = Vec::new();
            let row_items = items_by_aftersale.get(&a.aftersale_id).unwrap_or(&empty);
            let order = a.order_id.and_then(|id| order_map.get(&id));
            let empty_order_items: Vec<OrderItem> = Vec::new();
            let order_items = a
                .order_id
                .and_then(|id| order_items_map.get(&id))
                .unwrap_or(&empty_order_items);

            let latest_refund = refund_map.get(&a.aftersale_id);
            let pending_finance_refund = is_pending_finance_refund(a, latest_refund);
            let finance_rejected = is_finance_rejected(a, latest_refund);

            let mut vo = build_row_vo(
                a,
                row_items,
                order,
                pending_finance_refund,
                finance_rejected,
                now,
            );

            // 建议退款金额：仅「待处理/待供应商审核」行需要（内联「同意退款」金额上限校验）
            if let Some(order) = order {
                if a.status == aftersales_status::IN_REVIEW
                    || a.status == aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
                {
                    let pairs = pair_items(row_items);
                    let suggest = calc_refund_product_amount(order, order_items, &pairs);
                    let ctx = RefundContext {
                        order,
                        order_items,
                        current_aftersales: a,
                        current_pairs: pairs.as_slice(),
                        completed_other_pairs: &[],
                        earlier_other_pairs: &[],
                        other_processed_refund_total: Decimal::ZERO,
                    };
                    vo.suggest_refund_amount =
                        Some(calculate_final_refund_amount(&ctx, Some(suggest), true));
                }
            }
            list.push(vo);
        }
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 详情 —————————————————————

    /// 售后详情（对应 Java `AftersalesController.detail` /
    /// `AftersalesServiceImpl#detail`）。
    pub async fn detail(&self, aftersale_id: i32) -> Result<AftersalesVO> {
        let aftersales = self
            .repo()
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg("售后信息不存在"))?;

        let item_rows = self.repo().find_items_by_aftersale_id(aftersale_id).await?;
        let order = match aftersales.order_id {
            Some(oid) => self.repo().find_orders(&[oid]).await?.into_iter().next(),
            None => None,
        };
        let order_items = match order.as_ref() {
            Some(o) => self.repo().find_order_items_by_order(o.order_id).await?,
            None => Vec::new(),
        };
        let logs = self.repo().find_logs(aftersale_id).await?;
        let latest_refund = self.repo().find_latest_refund_apply(aftersale_id).await?;

        let pairs = pair_items(&item_rows);
        let now = utils::now_seconds() as i32;
        let mut vo = build_detail_vo(
            &aftersales,
            &item_rows,
            order.as_ref(),
            &logs,
            latest_refund.as_ref(),
            now,
        );

        // 建议退款金额 + 退款税费（Java `detail` 的整单/部分售后分支）
        if let Some(order) = order.as_ref() {
            let suggest = calc_refund_product_amount(order, &order_items, &pairs);
            let other_total = self
                .repo()
                .sum_other_processed_refunds(order.order_id, aftersale_id)
                .await?;
            let (completed_other, earlier_other) = self
                .load_other_aftersales_items(&aftersales, order.order_id)
                .await?;

            let is_full = is_full_order_refund(&order_items, &pairs);
            let refund_tax_amount = if is_full {
                // 整单售后：订单总价已含税，无需再摊税费
                ceil2(order.tax_rate_amount.unwrap_or(Decimal::ZERO))
            } else {
                ceil2(
                    item_rows
                        .iter()
                        .map(|r| row_tax_total_amount(r, order))
                        .fold(Decimal::ZERO, |acc, v| acc + v),
                )
            };

            let suggest_refund = if is_full {
                if is_at_least_processing(order.order_status) {
                    // 已发货不退运费
                    order.total_amount - order.shipping_fee
                } else {
                    // 未发货退运费
                    order.total_amount
                }
            } else {
                // 部分售后：基础金额为商品金额；若本次补齐最后一个订单行且运费未退过，则追加运费
                let shipping_fee = order.shipping_fee;
                let mut partial = suggest;
                if shipping_fee > Decimal::ZERO
                    && is_completing_last_items_current(
                        &order_items,
                        &pairs,
                        &completed_other,
                        &earlier_other,
                    )
                    && !is_shipping_already_refunded(order, partial, other_total)
                {
                    let pending = nextrade_core::models::trade::order_admin::ShippingStatus::Pending as i32;
                    let shipped = nextrade_core::models::trade::order_admin::ShippingStatus::Shipped as i32;
                    let completed = nextrade_core::models::trade::order_admin::OrderStatus::Completed as i32;
                    let should_add = order.shipping_status == pending
                        || (order.order_status == completed
                            && order.shipping_status == shipped
                            && order.shipping_time <= 0);
                    if should_add {
                        partial += shipping_fee;
                    }
                }
                partial
            };
            vo.suggest_refund_amount = Some(suggest_refund);
            vo.refund_tax_amount = Some(refund_tax_amount);
        }

        Ok(vo)
    }

    /// 载入「其他已完成售后」与「更早创建且未取消/驳回的售后」的明细
    /// （用于运费分摊与整单覆盖判定），返回两组已与订单明细配对的集合。
    async fn load_other_aftersales_items(
        &self,
        current: &Aftersales,
        order_id: i32,
    ) -> Result<(
        Vec<(AftersalesItem, OrderItem)>,
        Vec<(AftersalesItem, OrderItem)>,
    )> {
        let others = self
            .repo()
            .find_aftersales_by_order_and_status(order_id, &aftersales_status::valid_statuses())
            .await?;
        let other_ids: Vec<i32> = others
            .iter()
            .filter(|a| a.aftersale_id != current.aftersale_id)
            .map(|a| a.aftersale_id)
            .collect();

        let completed = self
            .repo()
            .find_aftersales_by_order_and_status(order_id, &[aftersales_status::COMPLETE])
            .await?;
        let completed_ids: Vec<i32> = completed
            .iter()
            .filter(|a| a.aftersale_id != current.aftersale_id)
            .map(|a| a.aftersale_id)
            .collect();

        let earlier = self
            .repo()
            .find_earlier_active_aftersales(order_id, current.aftersale_id)
            .await?;
        let earlier_ids: Vec<i32> = earlier.iter().map(|a| a.aftersale_id).collect();

        // 一次取回全部相关明细，再按集合切分（避免多次往返）
        let mut all_ids: Vec<i32> = other_ids;
        all_ids.extend(completed_ids.iter().copied());
        all_ids.extend(earlier_ids.iter().copied());
        all_ids.sort_unstable();
        all_ids.dedup();
        let rows = self.repo().find_items_by_aftersale_ids(&all_ids).await?;

        let completed_set: HashSet<i32> = completed_ids.into_iter().collect();
        let earlier_set: HashSet<i32> = earlier_ids.into_iter().collect();
        let mut completed_pairs: Vec<(AftersalesItem, OrderItem)> = Vec::new();
        let mut earlier_pairs: Vec<(AftersalesItem, OrderItem)> = Vec::new();
        for row in rows {
            let Some(order_item) = row.order_item.clone() else {
                continue;
            };
            let aid = row.item.aftersale_id.unwrap_or_default();
            if completed_set.contains(&aid) {
                completed_pairs.push((row.item.clone(), order_item.clone()));
            }
            if earlier_set.contains(&aid) {
                earlier_pairs.push((row.item, order_item));
            }
        }
        Ok((completed_pairs, earlier_pairs))
    }

    // ————————————————————— 审核（update） —————————————————————

    /// 退货地址三级兜底（Java `resolveReturnAddress`）：
    /// 供应商 `receive_info` → 店铺 `receive_info` → 平台 `shippingSettings`。
    async fn resolve_return_address(&self, order_id: i32) -> Result<Option<String>> {
        let Some(order) = self.repos.trade.order_admin.find_order(order_id).await? else {
            return Ok(None);
        };
        if let Some(vendor_id) = order.vendor_id.filter(|v| *v > 0) {
            if let Some(vendor) = self
                .repos
                .merchant
                .deposit_recharge
                .find_vendor(vendor_id)
                .await?
            {
                if let Some(addr) = delivery_address_from_json(vendor.receive_info.as_deref()) {
                    return Ok(Some(addr));
                }
            }
        }
        if order.shop_id > 0 {
            if let Some(shop) = self
                .repos
                .identity
                .admin_user
                .find_shop(order.shop_id)
                .await?
            {
                if let Some(addr) = delivery_address_from_json(shop.receive_info.as_deref()) {
                    return Ok(Some(addr));
                }
            }
        }
        let pairs = self
            .repos
            .system
            .config_basic
            .find_by_codes(&[
                "receiveSender",
                "receiveMobile",
                "receiveProvinceName",
                "receiveCityName",
                "receiveAreaName",
                "receiveAddress",
            ])
            .await?;
        let map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
        Ok(build_delivery_address(
            map.get("receiveSender").map(String::as_str),
            map.get("receiveMobile").map(String::as_str),
            map.get("receiveProvinceName").map(String::as_str),
            map.get("receiveCityName").map(String::as_str),
            map.get("receiveAreaName").map(String::as_str),
            map.get("receiveAddress").map(String::as_str),
        ))
    }

    /// 售后审核（对应 Java `AftersalesController.update` /
    /// `AftersalesServiceImpl#update`）。
    ///
    /// 状态门：仅 `1 审核处理中` / `21 待供应商审核` 可审，否则「该售后状态不能操作」。
    ///
    /// 收尾（2026-10-25 迁移）：仅退款审核通过后创建退款申请并按配置处理财务退款、
    /// 生成退款结算单、店铺退款结算（[`crate::trade::settlement`]）、拼团团长退款
    /// 整团解散；审核驳回时回滚账期订单（退款中 → 未结算）。
    pub async fn update(&self, param: &AftersalesEditParam) -> Result<()> {
        let aftersale_id = param
            .id
            .ok_or_else(|| Error::Validation("售后管理ID不能为空".into()))?;
        let aftersales = self
            .repo()
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg("售后信息不存在"))?;

        // Java `Assert.isTrue(IN_REVIEW || WAIT_FOR_SUPPLIER_AUDIT, "该售后状态不能操作")`
        if aftersales.status != aftersales_status::IN_REVIEW
            && aftersales.status != aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
        {
            return Err(Error::biz_msg("该售后状态不能操作"));
        }

        let status = param
            .status
            .ok_or_else(|| Error::Validation("状态不能为空".into()))?;
        let reply = param.reply.as_deref().unwrap_or_default();
        let now = utils::now_seconds() as i32;
        let shop_id = Some(aftersales.shop_id);
        let vendor_id = aftersales.vendor_id.filter(|v| *v > 0);

        // ——— Java `AftersalesEditParam#validParam` ———
        if status == aftersales_status::REFUSE && reply.trim().is_empty() {
            return Err(Error::Validation("拒绝请填原因".into()));
        }
        if status == aftersales_status::APPROVED_FOR_PROCESSING
            && aftersales.aftersale_type == aftersales_type::RETURN
            && !aftersales_status::is_supplier(aftersales.vendor_id)
            && param
                .return_address
                .as_deref()
                .is_none_or(|s| s.trim().is_empty())
        {
            return Err(Error::Validation(
                "同意退货退款请填写详细退货地址：姓名、联系方式、具体地址".into(),
            ));
        }

        // ——— 审核通过 ———
        if status == aftersales_status::APPROVED_FOR_PROCESSING {
            let order = match aftersales.order_id {
                Some(oid) => self.repo().find_orders(&[oid]).await?.into_iter().next(),
                None => None,
            };
            let order_items = match order.as_ref() {
                Some(o) => self.repo().find_order_items_by_order(o.order_id).await?,
                None => Vec::new(),
            };

            // 供应商是否还需人工审核（Java `isVendorAftersalesReviewRequired`：
            // 仅供应商退货售后 + 供应商 `after_sales_review = 1`）。
            let vendor_review_required = if is_vendor_return_aftersales(&aftersales)
                && is_vendor_deployment()
            {
                match aftersales
                    .vendor_id
                    .filter(|v| *v > 0)
                {
                    Some(vendor_id) => self
                        .repos
                        .merchant
                        .deposit_recharge
                        .find_vendor(vendor_id)
                        .await?
                        .is_some_and(|v| v.after_sales_review == Some(1)),
                    None => false,
                }
            } else {
                false
            };
            let vendor_audit_skipped = !vendor_review_required
                && is_vendor_return_aftersales(&aftersales)
                && aftersales.status == aftersales_status::IN_REVIEW;

            // 目标状态（Java `AftersalesEditParam#createUpdateAftersales`）
            let target_status = if aftersales.aftersale_type == aftersales_type::RETURN {
                if vendor_review_required
                    && aftersales.status != aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
                {
                    aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
                } else {
                    aftersales_status::SEND_BACK
                }
            } else {
                aftersales_status::COMPLETE
            };
            let final_time = if aftersales.aftersale_type == aftersales_type::PAY_RETURN {
                Some(now)
            } else {
                None
            };

            // 退货地址兜底：Java 在「供应商审核被跳过」且地址为空时自动回填
            // （依赖 vendor/shop/平台收货信息三级兜底，配置服务未迁移 → 记 uncertain）
            let mut return_address = param
                .return_address
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .or_else(|| aftersales.return_address.clone());
            let mut return_goods_tip = param.return_goods_tip.clone();
            if vendor_audit_skipped && return_address.as_deref().is_none_or(|s| s.trim().is_empty())
            {
                return_address = match aftersales.order_id {
                    Some(order_id) => self.resolve_return_address(order_id).await?,
                    None => None,
                };
                return_goods_tip = Some(match return_address.as_deref() {
                    Some(addr) => format!(
                        "您的售后申请已通过，请按以下地址回寄商品，并在回寄后填写物流信息：{addr}"
                    ),
                    None => "您的售后申请已通过，请按商家提供的地址回寄商品，并在回寄后填写物流信息"
                        .to_string(),
                });
            }

            // 退款金额：管理员未填写（null）时按退款商品金额兜底；
            // 显式填 0 是合法输入（如积分全额兑换订单实收为 0），不能被覆盖
            let mut refund_amount = param.refund_amount;
            if refund_amount.is_none() {
                if let Some(order) = order.as_ref() {
                    let rows = self.repo().find_items_by_aftersale_id(aftersale_id).await?;
                    let pairs = pair_items(&rows);
                    let product_amount = calc_refund_product_amount(order, &order_items, &pairs);
                    if product_amount > Decimal::ZERO {
                        refund_amount = Some(product_amount);
                    }
                }
            }

            self.repo()
                .update_aftersales(
                    aftersale_id,
                    Some(target_status),
                    Some(reply),
                    refund_amount,
                    return_address.as_deref(),
                    return_goods_tip.as_deref(),
                    None,
                    Some(now),
                    None,
                    final_time,
                    None,
                    None,
                )
                .await?;

            // 协商记录（Java `addAuditResultLog`：reply 为「系统自动审核通过」时归系统日志）
            let log_info = build_audit_log_info(status, reply);
            let is_system = reply == "系统自动审核通过";
            self.repo()
                .insert_log(&build_log(
                    aftersale_id,
                    log_info,
                    now,
                    if is_system {
                        None
                    } else {
                        Some(String::new())
                    },
                    String::new(),
                    if is_system { Some(0) } else { shop_id },
                    if is_system { None } else { vendor_id },
                    Decimal::ZERO,
                    None,
                ))
                .await?;

            // 供应商审核环节被系统代过的留痕
            if vendor_audit_skipped {
                self.repo()
                    .insert_log(&build_log(
                        aftersale_id,
                        "系统自动审核通过（供应商售后审核已关闭，跳过供应商审核环节）".to_string(),
                        now,
                        None,
                        String::new(),
                        Some(0),
                        vendor_id,
                        Decimal::ZERO,
                        None,
                    ))
                    .await?;
            }

            // 仅退款审核通过后创建财务退款申请并根据配置处理
            if aftersales.aftersale_type == aftersales_type::PAY_RETURN {
                if let Err(e) = self
                    .create_refund_apply_and_handle_finance(
                        &aftersales,
                        aftersales.refund_amount,
                        reply,
                    )
                    .await
                {
                    tracing::warn!("售后仅退款创建财务退款申请失败: {e}");
                }
                // 整单售后：释放订单/父单优惠券占用（Java
                // `restoreOrderCouponUsageIfFullAftersales`）。
                if let Some(order_id) = aftersales.order_id.filter(|id| *id > 0) {
                    let total = self
                        .repos
                        .trade
                        .order_admin
                        .order_items_total_quantity(order_id)
                        .await?;
                    let covered = self
                        .repos
                        .trade
                        .order_admin
                        .aftersales_covered_quantity(
                            order_id,
                            &aftersales_status::valid_statuses(),
                        )
                        .await?;
                    if total > 0 && covered >= total {
                        let released = self
                            .repos
                            .marketing
                            .coupon
                            .release_by_order(order_id)
                            .await?;
                        if released == 0 {
                            if let Some(order) = order.as_ref() {
                                if let Some(parent_id) =
                                    order.parent_order_id.filter(|id| *id > 0)
                                {
                                    let _ = self
                                        .repos
                                        .marketing
                                        .coupon
                                        .release_by_order(parent_id)
                                        .await;
                                }
                            }
                        }
                    }
                }
            }
            let log_info = format!("更新退换货:{aftersale_id}");
            // 供应商售后消息（Java `notifyVendorAftersale`，type=32）。
            if let Some(vendor_id) = vendor_id.filter(|v| *v > 0) {
                if matches!(
                    target_status,
                    aftersales_status::WAIT_FOR_SUPPLIER_AUDIT
                        | aftersales_status::SEND_BACK
                        | aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
                ) {
                    let order_sn = order.as_ref().map(|o| o.order_sn.as_str()).unwrap_or("");
                    let related = serde_json::json!({
                        "aftersaleId": aftersales.aftersale_id,
                        "aftersalesSn": aftersales.aftersales_sn,
                    })
                    .to_string();
                    let _ = self
                        .repos
                        .system
                        .admin_msg
                        .create_vendor_message(
                            32,
                            vendor_id,
                            aftersales.shop_id,
                            aftersales.order_id.unwrap_or(0),
                            "售后申请通知",
                            &format!(
                                "您的订单【{order_sn}】发起售后申请，请及时处理。"
                            ),
                            &related,
                        )
                        .await;
                }
            }
            // 退款收尾（Java 审核通过段）：退款出账单 + 店铺退款结算 +
            // 拼团团长退款整团解散（2026-10-25 迁移）
            crate::trade::settlement::SettlementService::new(self.repos.clone())
                .on_refund_completed(aftersale_id)
                .await;

            let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
            if let Err(e) = admin_log.record_current_log(if aftersales.shop_id > 0 { "shop" } else { "admin" }, aftersales.shop_id, &log_info).await {
                tracing::warn!("写管理操作日志失败（售后）: {e}");
            }
            return Ok(());
        }

        // ——— 审核未通过 ———
        if status == aftersales_status::REFUSE {
            self.repo()
                .update_aftersales(
                    aftersale_id,
                    Some(aftersales_status::REFUSE),
                    Some(reply),
                    None,
                    None,
                    None,
                    Some(aftersales_status::CLOSE_TYPE_AUDIT_REJECT),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
            let log_info = build_audit_log_info(status, reply);
            self.repo()
                .insert_log(&build_log(
                    aftersale_id,
                    log_info,
                    now,
                    Some(String::new()),
                    String::new(),
                    shop_id,
                    vendor_id,
                    Decimal::ZERO,
                    None,
                ))
                .await?;
            // 售后驳回：若该订单没有其他「处理中」售后，恢复账期订单为待结算
            // （Java 审核未通过段，settlement_status 3 退款中 → 0 未结算）
            if let Some(order_id) = aftersales.order_id.filter(|id| *id > 0) {
                let processing: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM aftersales WHERE order_id = $1 AND status IN (1, 2, 3, 4, 21, 22)",
                )
                .bind(order_id)
                .fetch_one(&self.repo().pool_clone())
                .await
                .unwrap_or(0);
                if processing <= 0 {
                    let _ = sqlx::query(
                        "UPDATE credit_order SET settlement_status = 0, update_time = $1                          WHERE order_id = $2 AND settlement_status = 3",
                    )
                    .bind(now)
                    .bind(order_id)
                    .execute(&self.repo().pool_clone())
                    .await;
                }
            }

            let admin_log_text = format!("更新退换货:{aftersale_id}");
            let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
            if let Err(e) = admin_log.record_current_log(if aftersales.shop_id > 0 { "shop" } else { "admin" }, aftersales.shop_id, &admin_log_text).await {
                tracing::warn!("写管理操作日志失败（售后）: {e}");
            }
            return Ok(());
        }

        Err(Error::Validation(format!(
            "不支持的售后审核目标状态: {status}"
        )))
    }

    // ————————————————————— 确认收货（receive） —————————————————————

    /// 售后确认收货（对应 Java `AftersalesController.receive` /
    /// `AftersalesServiceImpl#receive` → `completeReturnAftersales(id, false)`）。
    ///
    /// 收尾（2026-10-25 迁移）：置完成后创建退款申请（`createRefundApplyAfterReceive`）、
    /// 退款结算单与店铺退款结算走 [`crate::trade::settlement::SettlementService`]。
    pub async fn receive(&self, param: &AftersalesReceiveParam) -> Result<()> {
        let aftersale_id = param
            .id
            .ok_or_else(|| Error::Validation("售后ID不能为空".into()))?;
        self.complete_return_aftersales(aftersale_id, false).await?;
        Ok(())
    }

    /// 系统自动确认收货（Java `AftersalesServiceImpl#autoReceive`）。
    pub async fn auto_receive(&self, aftersale_id: i32) -> Result<bool> {
        self.complete_return_aftersales(aftersale_id, true).await
    }

    /// 拼团失败自动发起退款售后（Java `GrouponPromotionRecordServiceImpl#buildAftersale`）。
    ///
    /// 仅创建售后单/明细/日志，实际退款仍由财务退款流程处理。
    pub async fn create_groupon_refund(
        &self,
        order_id: i32,
        user_id: i32,
        refund_amount: Decimal,
    ) -> Result<bool> {
        let existing = self.repo().find_by_order(order_id).await?;
        if !existing.is_empty() {
            return Ok(false);
        }
        let Some(order) = self.repo().find_orders(&[order_id]).await?.into_iter().next() else {
            return Ok(false);
        };
        let items = self.repo().find_order_items_by_order(order_id).await?;
        let now = nextrade_common::utils::now_seconds() as i32;
        let description = "拼团失败，发起退款";
        let aftersales = Aftersales {
            aftersale_id: 0,
            aftersale_type: aftersales_type::PAY_RETURN,
            status: aftersales_status::IN_REVIEW,
            pics: "[]".to_string(),
            description: description.to_string(),
            reply: String::new(),
            add_time: now,
            tracking_no: String::new(),
            logistics_name: String::new(),
            return_address: None,
            aftersale_reason: Some(description.to_string()),
            aftersales_sn: None,
            order_id: Some(order_id),
            user_id: Some(user_id),
            refund_amount,
            shop_id: order.shop_id,
            audit_time: 0,
            deal_time: 0,
            final_time: 0,
            return_goods_tip: String::new(),
            vendor_id: order.vendor_id,
            close_type: 0,
        };
        let aftersale_id = self.repo().insert_aftersales(&aftersales).await?;
        let rows: Vec<nextrade_core::entities::AftersalesItem> = items
            .into_iter()
            .map(|item| nextrade_core::entities::AftersalesItem {
                aftersales_item_id: 0,
                order_item_id: Some(item.item_id),
                number: Some(item.quantity),
                aftersale_id: Some(aftersale_id),
            })
            .collect();
        self.repo().insert_items(&rows).await?;
        self.repo()
            .insert_log(&nextrade_core::entities::AftersalesLog {
                log_id: 0,
                aftersale_id,
                log_info: description.to_string(),
                add_time: now,
                admin_name: String::new(),
                refund_money: refund_amount,
                refund_type: aftersales_type::PAY_RETURN,
                refund_desc: description.to_string(),
                user_name: None,
                return_pic: Some("[]".to_string()),
                shop_id: Some(order.shop_id),
                vendor_id: order.vendor_id,
            })
            .await?;
        tracing::info!(
            "拼团失败创建退款售后: order_id={order_id}, aftersale_id={aftersale_id}"
        );
        Ok(true)
    }

    /// 完结退货售后（对应 Java `AftersalesServiceImpl#completeReturnAftersales`）。
    async fn complete_return_aftersales(
        &self,
        aftersale_id: i32,
        system_auto: bool,
    ) -> Result<bool> {
        let aftersales = self
            .repo()
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg("售后信息不存在"))?;

        let status = aftersales.status;
        if system_auto {
            if status != aftersales_status::RETURNED
                && status != aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
            {
                return Ok(false);
            }
        } else {
            let can_receive =
                if is_vendor_deployment() && aftersales.vendor_id.is_some_and(|v| v > 0) {
                    status == aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
                } else {
                    status == aftersales_status::RETURNED
                };
            if !can_receive {
                return Err(Error::biz_msg("该售后状态不能操作"));
            }
        }

        let order = match aftersales.order_id {
            Some(oid) => self.repo().find_orders(&[oid]).await?.into_iter().next(),
            None => None,
        };
        let order_items = match order.as_ref() {
            Some(o) => self.repo().find_order_items_by_order(o.order_id).await?,
            None => Vec::new(),
        };
        let rows = self.repo().find_items_by_aftersale_id(aftersale_id).await?;
        let pairs = pair_items(&rows);

        // 退款金额重算（Java `calculateFinalRefundAmount(order, aftersales, refundAmount, true)`）
        let final_refund_amount = match order.as_ref() {
            Some(order) => {
                let other_total = self
                    .repo()
                    .sum_other_processed_refunds(order.order_id, aftersale_id)
                    .await?;
                let (completed_other, earlier_other) = self
                    .load_other_aftersales_items(&aftersales, order.order_id)
                    .await?;
                // `RefundContext` 的这两个字段是**引用元组**切片，而
                // `load_other_aftersales_items` 返回拥有值的元组；此处按
                // 「改调用点、不改被调函数签名」的原则就地取引用。
                let completed_other_pairs: Vec<(&AftersalesItem, &OrderItem)> =
                    completed_other.iter().map(|(a, o)| (a, o)).collect();
                let earlier_other_pairs: Vec<(&AftersalesItem, &OrderItem)> =
                    earlier_other.iter().map(|(a, o)| (a, o)).collect();
                let ctx = RefundContext {
                    order,
                    order_items: &order_items,
                    current_aftersales: &aftersales,
                    current_pairs: pairs.as_slice(),
                    completed_other_pairs: completed_other_pairs.as_slice(),
                    earlier_other_pairs: earlier_other_pairs.as_slice(),
                    other_processed_refund_total: other_total,
                };
                calculate_final_refund_amount(&ctx, Some(aftersales.refund_amount), true)
            }
            None => aftersales.refund_amount,
        };

        let now = utils::now_seconds() as i32;
        let updated = self
            .repo()
            .update_aftersales(
                aftersale_id,
                Some(aftersales_status::COMPLETE),
                None,
                Some(final_refund_amount),
                None,
                None,
                None,
                None,
                None,
                Some(now),
                None,
                None,
            )
            .await?;
        if !updated {
            return Ok(false);
        }

        // 协商记录（Java：系统自动走 `addSystemAftersaleLog`，人工走 `addAftersaleLog`）
        let (log_info, log_user, log_shop, log_vendor) = if system_auto {
            let vendor_receipt = is_vendor_return_aftersales(&aftersales);
            (
                "系统自动确认收货，售后申请已完成".to_string(),
                None,
                if vendor_receipt {
                    Some(0)
                } else {
                    Some(aftersales.shop_id)
                },
                if vendor_receipt {
                    aftersales.vendor_id
                } else {
                    None
                },
            )
        } else {
            (
                format!(
                    "{}: 售后申请通过",
                    aftersales_status::name(aftersales_status::COMPLETE)
                ),
                Some(String::new()),
                Some(aftersales.shop_id),
                aftersales.vendor_id.filter(|v| *v > 0),
            )
        };
        self.repo()
            .insert_log(&build_log(
                aftersale_id,
                log_info,
                now,
                log_user,
                String::new(),
                log_shop,
                log_vendor,
                final_refund_amount,
                None,
            ))
            .await?;

        // Java：`if (!systemAuto) adminLogService.createByLogInfo("完成退换货:" + id)`
        if !system_auto {
            let log_info = format!("完成退换货:{aftersale_id}");
            let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
            if let Err(e) = admin_log
                .record_current_log(
                    if aftersales.shop_id > 0 { "shop" } else { "admin" },
                    aftersales.shop_id,
                    &log_info,
                )
                .await
            {
                tracing::warn!("写管理操作日志失败（售后）: {e}");
            }
        }

        if let Err(e) = self
            .create_refund_apply_and_handle_finance(
                &aftersales,
                final_refund_amount,
                "售后确认收货完成退款",
            )
            .await
        {
            tracing::warn!("售后确认收货完成创建财务退款申请失败: {e}");
        }

        // 退款收尾（Java completeReturnAftersales）：退款出账单 + 店铺退款结算 +
        // 拼团团长退款整团解散（2026-10-25 迁移）
        crate::trade::settlement::SettlementService::new(self.repos.clone())
            .on_refund_completed(aftersale_id)
            .await;

        tracing::info!(
            "售后确认收货完成: aftersale_id={aftersale_id}, refund_amount={final_refund_amount}, \
             system_auto={system_auto}"
        );
        Ok(true)
    }

    // ————————————————————— 拒收（rejectReceive） —————————————————————

    /// 售后拒收（对应 Java `AftersalesController.rejectReceive` /
    /// `AftersalesServiceImpl#rejectReceive`）。
    ///
    /// 拒收后售后置为「审核未通过」（待关闭），不发起任何退款/结算；
    /// 货品由买卖双方线下协商，商家后续可走 `complete` 关闭为已取消。
    pub async fn reject_receive(&self, param: &AfterSalesRejectReceiveParam) -> Result<()> {
        let aftersale_id = param
            .id
            .ok_or_else(|| Error::Validation("售后ID不能为空".into()))?;
        let reply = param
            .reply
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("拒收理由不能为空".into()))?;
        // Java `@Size(max = 200)`
        if reply.chars().count() > 200 {
            return Err(Error::Validation("拒收理由最多200字".into()));
        }

        let aftersales = self
            .repo()
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg("售后信息不存在"))?;

        // 状态门：与确认收货同节点、同角色（商家取 5，供应商取 22）
        let can_reject = if is_vendor_deployment() && aftersales.vendor_id.is_some_and(|v| v > 0) {
            aftersales.status == aftersales_status::WAIT_FOR_SUPPLIER_RECEIPT
        } else {
            aftersales.status == aftersales_status::RETURNED
        };
        if !can_reject {
            return Err(Error::biz_msg("该售后状态不能操作"));
        }

        let now = utils::now_seconds() as i32;
        self.repo()
            .update_aftersales(
                aftersale_id,
                Some(aftersales_status::REFUSE),
                Some(reply),
                None,
                None,
                None,
                Some(aftersales_status::CLOSE_TYPE_REJECT_RECEIVE),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;

        self.repo()
            .insert_log(&build_log(
                aftersale_id,
                format!("拒收退货：{reply}"),
                now,
                Some(String::new()),
                String::new(),
                Some(aftersales.shop_id),
                aftersales.vendor_id.filter(|v| *v > 0),
                Decimal::ZERO,
                None,
            ))
            .await?;
        let log_info = format!("拒收退货:{aftersale_id}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if aftersales.shop_id > 0 { "shop" } else { "admin" }, aftersales.shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败（售后）: {e}");
        }
        Ok(())
    }

    // ————————————————————— 反馈记录（record） —————————————————————

    /// 提交售后反馈记录（对应 Java `AftersalesController.record` /
    /// `AftersalesServiceImpl#submitFeedbackRecord(Integer, DTO, Integer)`）。
    ///
    /// 状态为 `4 待回寄` 且当前登录管理员有效（id != 0）时，把物流单号/物流公司
    /// 回写到售后主单并记 \"deal_time\"，随后写入协商记录。
    ///
    /// **与 Java 的差异（Java 自身的缺陷）**：Java 此处用 `this.save(updateAftersales)`
    /// 而非 \"updateById\"，在 MyBatis-Plus 下会**再插入一行**（主键已存在则报错），
    /// 属明显的实现事故。Rust 按注释所述业务意图（回写物流信息）实现为 UPDATE，
    /// 详见 \"uncertain_points\"。
    pub async fn submit_record(
        &self,
        param: &AftersalesRecordCreateParam,
        current_admin_id: Option<i32>,
        current_username: Option<&str>,
    ) -> Result<()> {
        let aftersale_id = param
            .aftersale_id
            .ok_or_else(|| Error::Validation("售后ID不能为空".into()))?;
        let aftersales = self
            .repo()
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg("售后信息不存在"))?;

        let now = utils::now_seconds() as i32;
        let tracking_no = param
            .tracking_no
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let logistics_name = param
            .logistics_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        // Java：`status == SEND_BACK && currentAdminId != null && currentAdminId != 0`
        if aftersales.status == aftersales_status::SEND_BACK
            && current_admin_id.is_some_and(|id| id != 0)
        {
            self.repo()
                .update_aftersales(
                    aftersale_id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(now),
                    None,
                    tracking_no,
                    logistics_name,
                )
                .await?;
        }

        let return_pic = param
            .return_pic
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        // Java：`setAdminName(currentAdminId != null $3 SecurityUtils.getCurrentUsername() : "")`
        let admin_name = if current_admin_id.is_some() {
            current_username.unwrap_or_default().to_string()
        } else {
            String::new()
        };

        self.repo()
            .insert_log(&build_log(
                aftersale_id,
                param.log_info.clone().unwrap_or_default(),
                now,
                current_username.map(ToString::to_string),
                admin_name,
                Some(aftersales.shop_id),
                aftersales.vendor_id.filter(|v| *v > 0),
                Decimal::ZERO,
                return_pic,
            ))
            .await?;
        Ok(())
    }

    // ————————————————————— 完结（complete） —————————————————————

    /// 售后完结（对应 Java `AftersalesController.complete` /
    /// `AftersalesServiceImpl#complete`）：仅「审核未通过」可关单为「已取消」。
    pub async fn complete(&self, param: &AfterSalesCompleteParam) -> Result<()> {
        let aftersale_id = param
            .resolved_id()
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        let aftersales = self
            .repo()
            .find_aftersales(aftersale_id)
            .await?
            .ok_or_else(|| Error::biz_msg("售后信息不存在"))?;

        if aftersales.status != aftersales_status::REFUSE {
            return Err(Error::biz_msg("该售后状态不能操作"));
        }

        let now = utils::now_seconds() as i32;
        self.repo()
            .update_aftersales(
                aftersale_id,
                Some(aftersales_status::CANCEL),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await?;

        self.repo()
            .insert_log(&build_log(
                aftersale_id,
                "售后驳回，关闭售后单".to_string(),
                now,
                Some(String::new()),
                String::new(),
                Some(aftersales.shop_id),
                aftersales.vendor_id.filter(|v| *v > 0),
                Decimal::ZERO,
                None,
            ))
            .await?;
        Ok(())
    }

    /// 创建财务退款申请并根据财务审核配置处理（对齐 Java createRefundApplyAndHandleFinance）。
    pub async fn create_refund_apply_and_handle_finance(
        &self,
        aftersales: &Aftersales,
        refund_amount: Decimal,
        refund_note: &str,
    ) -> Result<i32> {
        let now = utils::now_seconds() as i32;
        if let Some(existing) = self
            .repos
            .payment
            .refund_apply
            .find_by_aftersale(aftersales.aftersale_id)
            .await?
        {
            return Ok(existing.refund_id);
        }

        let apply = RefundApply {
            refund_id: 0,
            refund_type: if aftersales.order_id.unwrap_or(0) > 0 { 1 } else { 2 },
            order_id: aftersales.order_id.unwrap_or(0),
            user_id: aftersales.user_id.unwrap_or(0),
            aftersale_id: aftersales.aftersale_id,
            refund_status: 1, // 待审核
            add_time: now,
            refund_note: refund_note.to_string(),
            online_balance: refund_amount,
            offline_balance: Decimal::ZERO,
            refund_balance: Decimal::ZERO,
            is_online: 1,
            is_offline: 0,
            is_receive: 0,
            paylog_refund_id: 0,
            shop_id: aftersales.shop_id,
            payment_voucher: None,
        };
        let refund_id = self.repos.payment.refund_apply.insert(&apply).await?;

        let log = RefundLog {
            log_id: 0,
            refund_apply_id: refund_id,
            refund_type: 1,
            refund_pay_code: "online".into(),
            transaction_id: String::new(),
            refund_amount,
            add_time: now,
            description: Some(format!("售后通过，创建财务退款申请: {refund_note}")),
            user_id: aftersales.user_id.unwrap_or(0),
            order_id: aftersales.order_id.unwrap_or(0),
        };
        let _ = self.repos.payment.refund_apply.insert_log(&log).await;

        let financial_review = self
            .repos
            .system
            .config_basic
            .find_by_code("financialReview")
            .await?
            .unwrap_or_default();
        if financial_review != "1" {
            let _ = self
                .repos
                .payment
                .refund_apply
                .transition_status(refund_id, &[1], 2, Some("财务审核关闭自动执行退款"))
                .await;
            self.repo()
                .insert_log(&build_log(
                    aftersales.aftersale_id,
                    "财务审核已关闭，系统已自动执行退款处理".to_string(),
                    now,
                    None,
                    String::new(),
                    Some(aftersales.shop_id),
                    aftersales.vendor_id,
                    refund_amount,
                    None,
                ))
                .await?;
        }

        Ok(refund_id)
    }
}

// ————————————————————— 内部辅助 —————————————————————

/// 是否为供应商订单的退货退款售后（Java `isVendorReturnAftersales`）。
fn is_vendor_return_aftersales(aftersales: &Aftersales) -> bool {
    if aftersales.aftersale_type != aftersales_type::RETURN {
        return false;
    }
    if !is_vendor_deployment() {
        return false;
    }
    aftersales.vendor_id.is_some_and(|v| v > 0)
}

/// 审核结果日志文案（Java `buildAuditLogInfo`：reply 为空时仅返回状态名）。
fn build_audit_log_info(status: i32, reply: &str) -> String {
    let status_name = aftersales_status::name(status);
    if reply.trim().is_empty() {
        status_name.to_string()
    } else {
        format!("{status_name}:{reply}")
    }
}

/// 订单状态是否已达到「处理中，已发货」及以上（Java `>= OrderStatusEnum.PROCESSING`）。
fn is_at_least_processing(order_status: i32) -> bool {
    order_status >= nextrade_core::models::trade::order_admin::OrderStatus::Processing as i32
}

/// 售后明细行 → 税率/单件税费/数量税费三元组。
fn tax_parts(row: &AftersalesItemRow, order: &Order) -> (Decimal, Decimal, Decimal) {
    let Some(order_item) = row.order_item.as_ref() else {
        return (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    };
    let tax_rate = order.tax_rate.unwrap_or(Decimal::ZERO);
    let unit = unit_tax_amount(order_item.price, tax_rate);
    let total = unit * Decimal::from(row.item.number.unwrap_or(0));
    (tax_rate, unit, total)
}

/// 售后明细行的「数量 × 单件税费」。
fn row_tax_total_amount(row: &AftersalesItemRow, order: &Order) -> Decimal {
    tax_parts(row, order).2
}

/// 售后明细行 → 展示 VO（对应 Java `AftersalesItemServiceImpl#getItemByIds`）。
fn to_item_vo(row: &AftersalesItemRow, order: Option<&Order>) -> AftersalesItemVO {
    let item = &row.item;
    let Some(order_item) = row.order_item.as_ref() else {
        return AftersalesItemVO {
            aftersales_item_id: item.aftersales_item_id,
            order_item_id: item.order_item_id,
            number: item.number,
            aftersale_id: item.aftersale_id,
            ..Default::default()
        };
    };

    let tax_rate = order.and_then(|o| o.tax_rate).unwrap_or(Decimal::ZERO);
    let unit_tax = unit_tax_amount(order_item.price, tax_rate);
    let number = item.number.unwrap_or(0);

    // 规格数据（Java：`skuData == "[]"` 时为空表）
    let sku_data: Vec<serde_json::Value> = {
        let raw = order_item.sku_data.trim();
        if raw.is_empty() || raw == "[]" {
            Vec::new()
        } else {
            serde_json::from_str(raw).unwrap_or_default()
        }
    };
    let sku_value = sku_data
        .iter()
        .filter_map(|v| {
            let name = v
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let value = v
                .get("value")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if name.is_empty() && value.is_empty() {
                None
            } else {
                Some(format!("{name}:{value}"))
            }
        })
        .collect::<Vec<_>>()
        .join("|");

    AftersalesItemVO {
        aftersales_item_id: item.aftersales_item_id,
        order_item_id: item.order_item_id,
        number: item.number,
        aftersale_id: item.aftersale_id,
        order_sn: order_item.order_sn.clone(),
        product_name: order_item.product_name.clone(),
        order_id: order_item.order_id,
        pic_thumb: order_item.pic_thumb.clone(),
        product_sn: order_item.product_sn.clone(),
        product_id: order_item.product_id,
        quantity: order_item.quantity,
        // Java：\"price\" 回显为「商品单价 + 单件税费」
        price: display_unit_price(order_item, tax_rate),
        supply_price: None,
        tax_rate,
        tax_rate_amount: unit_tax,
        tax_total_amount: unit_tax * Decimal::from(number),
        extra_sku_data: order_item.extra_sku_data.clone(),
        sku_data,
        sku_value,
    }
}

/// 售后明细行列表 → VO 列表。
fn to_item_vos(rows: &[AftersalesItemRow], order: Option<&Order>) -> Vec<AftersalesItemVO> {
    rows.iter().map(|r| to_item_vo(r, order)).collect()
}

/// 把「售后明细行」与「订单明细」配对（仅保留能配上订单明细的行），
/// 供退款金额计算使用。
fn pair_items(rows: &[AftersalesItemRow]) -> Vec<(&AftersalesItem, &OrderItem)> {
    rows.iter()
        .filter_map(|r| r.order_item.as_ref().map(|oi| (&r.item, oi)))
        .collect()
}

/// 协商记录实体 → VO。
fn to_log_vo(log: &AftersalesLog) -> AftersalesLogVO {
    AftersalesLogVO {
        log_id: log.log_id,
        admin_name: log.admin_name.clone(),
        aftersales_id: log.aftersale_id.to_string(),
        return_pic: parse_json_list(log.return_pic.as_deref()),
        log_info: log.log_info.clone(),
        refund_desc: log.refund_desc.clone(),
        refund_money: log.refund_money,
        refund_type: log.refund_type,
        user_name: log.user_name.clone(),
        add_time: fmt_time(log.add_time),
        shop_id: log.shop_id,
        vendor_id: log.vendor_id,
    }
}

/// 「待平台财务退款」判定（对应 Java \"isPendingFinanceRefund\"）。
fn is_pending_finance_refund(aftersales: &Aftersales, latest: Option<&RefundApply>) -> bool {
    if aftersales.status != aftersales_status::COMPLETE {
        return false;
    }
    let Some(r) = latest else {
        // 查不到退款申请的历史数据不覆盖，仍按「已完成」展示
        return false;
    };
    matches!(
        RefundApplyStatus::try_from(r.refund_status).ok(),
        Some(RefundApplyStatus::Wait | RefundApplyStatus::Processing)
    )
}

/// 「平台财务已拒绝」判定（对应 Java `getFinanceRejectReason != null`）。
///
/// 只看类型是否为仅退款、最新退款申请是否已拒绝，不限制售后主单状态
/// （否则同一单会在列表和详情出现两种状态）。
fn is_finance_rejected(aftersales: &Aftersales, latest: Option<&RefundApply>) -> bool {
    if aftersales.aftersale_type != aftersales_type::PAY_RETURN {
        return false;
    }
    latest.is_some_and(|r| {
        matches!(
            RefundApplyStatus::try_from(r.refund_status).ok(),
            Some(RefundApplyStatus::Rejected)
        )
    })
}

/// 财务拒绝原因（对应 Java \"getFinanceRejectReason\"）。
fn finance_reject_reason(aftersales: &Aftersales, latest: Option<&RefundApply>) -> Option<String> {
    if !is_finance_rejected(aftersales, latest) {
        return None;
    }
    let r = latest?;
    let note = r.refund_note.trim();
    Some(if note.is_empty() {
        "未提供拒绝原因".to_string()
    } else {
        note.to_string()
    })
}

/// 展示用状态（对应 Java \"resolveDisplayStatus\"）：待平台财务退款或财务已拒绝时
/// 按「审核通过待处理(2)」出文案，其余保持真实状态。
fn resolve_display_status(aftersales: &Aftersales, latest: Option<&RefundApply>) -> i32 {
    if is_finance_rejected(aftersales, latest) || is_pending_finance_refund(aftersales, latest) {
        return aftersales_status::APPROVED_FOR_PROCESSING;
    }
    aftersales.status
}

/// 实际退款金额（Java：\"IN_REVIEW\" / \"REFUSE\" / \"CANCEL\" 时为 0，之后取主单金额）。
fn actual_refund_amount(aftersales: &Aftersales) -> Decimal {
    match aftersales.status {
        aftersales_status::IN_REVIEW
        | aftersales_status::REFUSE
        | aftersales_status::CANCEL => Decimal::ZERO,
        _ => aftersales.refund_amount,
    }
}

/// 构造列表行 VO（对应 Java \"list\" 循环内的 \"AftersalesVO\" 组装）。
fn build_row_vo(
    a: &Aftersales,
    rows: &[AftersalesItemRow],
    order: Option<&Order>,
    pending_finance_refund: bool,
    finance_rejected: bool,
    now: i32,
) -> AftersalesVO {
    let status_name =
        aftersales_status::resolve_list_status_name(a, pending_finance_refund, finance_rejected);
    AftersalesVO {
        aftersale_id: a.aftersale_id,
        status: a.status,
        add_time: fmt_time(a.add_time),
        aftersale_reason: a.aftersale_reason.clone(),
        aftersale_type: a.aftersale_type,
        aftersales_sn: a.aftersales_sn.clone(),
        audit_time: fmt_time(a.audit_time),
        deal_time: fmt_time(a.deal_time),
        final_time: fmt_time(a.final_time),
        description: a.description.clone(),
        logistics_name: a.logistics_name.clone(),
        tracking_no: a.tracking_no.clone(),
        order_id: a.order_id.unwrap_or_default(),
        order_sn: order.map(|o| o.order_sn.clone()).unwrap_or_default(),
        pics: parse_json_list(Some(a.pics.as_str())),
        refund_amount: a.refund_amount.to_string(),
        actual_refund_amount: actual_refund_amount(a),
        reply: a.reply.clone(),
        return_address: a.return_address.clone(),
        return_goods_tip: a.return_goods_tip.clone(),
        shop_id: a.shop_id,
        user_id: a.user_id.unwrap_or_default(),
        vendor_id: a.vendor_id,
        close_type: a.close_type,
        status_name,
        status_description: None,
        aftersales_type_name: aftersales_type::name(Some(a.aftersale_type)).to_string(),
        suggest_refund_amount: None,
        refund_tax_amount: None,
        // 表头「订单详情」「标记」入口回显（订单查不到时按 Java 不下发）
        mark: order.and_then(|o| o.mark),
        admin_note: order.map(|o| o.admin_note.clone()),
        auto_agree_end_time: None,
        server_time: fmt_time(now),
        aftersales_items: to_item_vos(rows, order),
        aftersales_log: Vec::new(),
    }
}

/// 构造详情 VO（对应 Java \"detail\" 的 VO 组装；建议退款金额/税费由调用方补齐）。
fn build_detail_vo(
    a: &Aftersales,
    rows: &[AftersalesItemRow],
    order: Option<&Order>,
    logs: &[AftersalesLog],
    latest_refund: Option<&RefundApply>,
    now: i32,
) -> AftersalesVO {
    let finance_rejected = is_finance_rejected(a, latest_refund);
    let status_for_display = resolve_display_status(a, latest_refund);
    let reject_reason = finance_reject_reason(a, latest_refund);
    let reply_for_desc = if finance_rejected {
        reject_reason.as_deref()
    } else {
        Some(a.reply.as_str())
    };

    // 退货退款且未存回寄地址时，Java 会尝试按 供应商→店铺→平台 三级回填；
    // 配置服务未迁移，此处保留原值（不做臆测写入）
    let return_address = a.return_address.clone();

    AftersalesVO {
        aftersale_id: a.aftersale_id,
        status: a.status,
        add_time: fmt_time(a.add_time),
        aftersale_reason: a.aftersale_reason.clone(),
        aftersale_type: a.aftersale_type,
        aftersales_sn: a.aftersales_sn.clone(),
        audit_time: fmt_time(a.audit_time),
        deal_time: fmt_time(a.deal_time),
        final_time: fmt_time(a.final_time),
        description: a.description.clone(),
        logistics_name: a.logistics_name.clone(),
        tracking_no: a.tracking_no.clone(),
        order_id: a.order_id.unwrap_or_default(),
        order_sn: order.map(|o| o.order_sn.clone()).unwrap_or_default(),
        pics: parse_json_list(Some(a.pics.as_str())),
        refund_amount: a.refund_amount.to_string(),
        actual_refund_amount: actual_refund_amount(a),
        reply: a.reply.clone(),
        return_address,
        return_goods_tip: a.return_goods_tip.clone(),
        shop_id: a.shop_id,
        user_id: a.user_id.unwrap_or_default(),
        vendor_id: a.vendor_id,
        close_type: a.close_type,
        status_name: aftersales_status::admin_name(
            Some(a.aftersale_type),
            status_for_display,
            a.vendor_id,
            finance_rejected,
        )
        .unwrap_or_default()
        .to_string(),
        status_description: aftersales_status::admin_description(
            Some(a.aftersale_type),
            status_for_display,
            a.vendor_id,
            a.close_type,
            reply_for_desc,
            finance_rejected,
        ),
        aftersales_type_name: aftersales_type::name(Some(a.aftersale_type)).to_string(),
        suggest_refund_amount: None,
        refund_tax_amount: None,
        mark: order.and_then(|o| o.mark),
        admin_note: order.map(|o| o.admin_note.clone()),
        auto_agree_end_time: resolve_auto_agree_end_time(a.status, a.add_time),
        server_time: fmt_time(now),
        aftersales_items: to_item_vos(rows, order),
        aftersales_log: logs.iter().map(to_log_vo).collect(),
    }
}

/// 当前售后是否「补齐了最后一个订单行」（对应 Java \"isCompletingLastItems\"）。
///
/// 逐条对照 Java：
/// - 「加上当前售后」的集合 = **已完成的其他售后明细** + 当前售后明细；
/// - 「不加当前售后」的集合 = 比当前售后更早创建、且未取消/驳回的其他售后明细；
/// - 前者覆盖全部订单商品、后者不能覆盖 → 当前售后补齐了最后的缺口，运费归入该单。
fn is_completing_last_items_current(
    order_items: &[OrderItem],
    current_pairs: &ItemPairs<'_>,
    completed_other: &[(AftersalesItem, OrderItem)],
    earlier_other: &[(AftersalesItem, OrderItem)],
) -> bool {
    if order_items.is_empty() || current_pairs.is_empty() {
        return false;
    }

    let mut with_current: HashMap<i32, i32> = HashMap::new();
    for (item, order_item) in completed_other {
        if item.order_item_id == Some(order_item.item_id) {
            *with_current.entry(order_item.item_id).or_insert(0) += item.number.unwrap_or(0);
        }
    }
    for (item, order_item) in current_pairs {
        if item.order_item_id == Some(order_item.item_id) {
            *with_current.entry(order_item.item_id).or_insert(0) += item.number.unwrap_or(0);
        }
    }

    let mut earlier_only: HashMap<i32, i32> = HashMap::new();
    for (item, order_item) in earlier_other {
        if item.order_item_id == Some(order_item.item_id) {
            *earlier_only.entry(order_item.item_id).or_insert(0) += item.number.unwrap_or(0);
        }
    }

    let covers = |map: &HashMap<i32, i32>| -> bool {
        !map.is_empty()
            && order_items
                .iter()
                .all(|oi| map.get(&oi.item_id).copied().unwrap_or(0) >= oi.quantity)
    };
    covers(&with_current) && !covers(&earlier_only)
}

fn delivery_address_from_json(raw: Option<&str>) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw?.trim()).ok()?;
    build_delivery_address(
        value.get("sender").and_then(|v| v.as_str()),
        value.get("mobile").and_then(|v| v.as_str()),
        value.get("provinceName").and_then(|v| v.as_str()),
        value.get("cityName").and_then(|v| v.as_str()),
        value.get("areaName").and_then(|v| v.as_str()),
        value.get("address").and_then(|v| v.as_str()),
    )
}

fn build_delivery_address(
    sender: Option<&str>,
    mobile: Option<&str>,
    province: Option<&str>,
    city: Option<&str>,
    area: Option<&str>,
    address: Option<&str>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(v) = sender.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(v.to_string());
    }
    if let Some(v) = mobile.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(v.to_string());
    }
    let detail = format!(
        "{}{}{}{}",
        province.unwrap_or("").trim(),
        city.unwrap_or("").trim(),
        area.unwrap_or("").trim(),
        address.unwrap_or("").trim()
    );
    if !detail.trim().is_empty() {
        parts.push(detail);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn order(status: i32, shipping_status: i32, total: &str, shipping_fee: &str) -> Order {
        Order {
            order_id: 1,
            order_status: status,
            shipping_status,
            total_amount: Decimal::from_str(total).unwrap(),
            shipping_fee: Decimal::from_str(shipping_fee).unwrap(),
            shipping_time: 0,
            tax_rate: Some(Decimal::from_str("6").unwrap()),
            tax_rate_amount: Some(Decimal::from_str("3.40").unwrap()),
            ..Default::default()
        }
    }

    fn order_item(item_id: i32, price: &str, quantity: i32) -> OrderItem {
        OrderItem {
            item_id,
            order_id: 1,
            price: Decimal::from_str(price).unwrap(),
            quantity,
            ..Default::default()
        }
    }

    fn afs_item(order_item_id: i32, number: i32) -> AftersalesItem {
        AftersalesItem {
            aftersales_item_id: 0,
            order_item_id: Some(order_item_id),
            number: Some(number),
            aftersale_id: Some(1),
        }
    }

    fn aftersales(aftersale_type: i32, status: i32, refund_amount: &str) -> Aftersales {
        Aftersales {
            aftersale_id: 1,
            aftersale_type,
            status,
            refund_amount: Decimal::from_str(refund_amount).unwrap(),
            shop_id: 0,
            ..Default::default()
        }
    }

    // ————————————————— 枚举下拉 —————————————————

    #[test]
    fn apply_type_matches_java_controller() {
        let map = apply_type();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&1).map(String::as_str), Some("退货/退款"));
        assert_eq!(map.get(&2).map(String::as_str), Some("仅退款"));
    }

    #[test]
    fn return_goods_status_matches_java_controller() {
        let map = return_goods_status(Some("admin"));
        assert_eq!(map.len(), 7);
        assert_eq!(map.get(&1).map(String::as_str), Some("审核处理中"));
        assert_eq!(map.get(&3).map(String::as_str), Some("审核未通过"));
        // Java 源码里 4 被赋值了两次，最终文案是「待回寄」
        assert_eq!(map.get(&4).map(String::as_str), Some("待回寄"));
        assert_eq!(map.get(&5).map(String::as_str), Some("待商家收货"));
        assert_eq!(map.get(&7).map(String::as_str), Some("已取消"));
        // 当前部署 is-vendor = 0，故不含供应商状态
        assert!(!map.contains_key(&21));
        assert!(!map.contains_key(&22));
    }

    #[test]
    fn return_goods_status_for_vendor_admin_returns_three_entries() {
        let map = return_goods_status(Some("vendor"));
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&21).map(String::as_str), Some("供应商待审核"));
        assert_eq!(map.get(&22).map(String::as_str), Some("供应商待收货"));
        assert_eq!(map.get(&6).map(String::as_str), Some("已完成"));
        // 供应商账号不再看到买家端状态
        assert!(!map.contains_key(&1));
    }

    #[test]
    fn return_goods_status_tolerates_missing_admin_type() {
        // Java 在 admin_type 为 null 时会抛异常；此处按「非供应商」返回基础表
        let map = return_goods_status(None);
        assert_eq!(map.len(), 7);
        assert!(map.contains_key(&1));
    }

    // ————————————————— 税费与金额 —————————————————

    #[test]
    fn unit_tax_amount_ceils_to_two_decimals() {
        // 10.00 × 6% = 0.60
        assert_eq!(
            unit_tax_amount(Decimal::from_str("10.00").unwrap(), Decimal::from(6)),
            Decimal::from_str("0.60").unwrap()
        );
        // 9.99 × 6% = 0.5994 → 向上取整 0.60
        assert_eq!(
            unit_tax_amount(Decimal::from_str("9.99").unwrap(), Decimal::from(6)),
            Decimal::from_str("0.60").unwrap()
        );
        // 税率为 0 或单价为 0 → 0
        assert_eq!(
            unit_tax_amount(Decimal::ZERO, Decimal::from(6)),
            Decimal::ZERO
        );
        assert_eq!(
            unit_tax_amount(Decimal::from(10), Decimal::ZERO),
            Decimal::ZERO
        );
    }

    #[test]
    fn ceil2_mul_rounds_up() {
        // 0.005 × 3 = 0.015 → 0.02
        assert_eq!(
            ceil2_mul(Decimal::from_str("0.005").unwrap(), 3),
            Decimal::from_str("0.02").unwrap()
        );
        // 1.00 × 2 = 2.00（不引入额外进位）
        assert_eq!(
            ceil2_mul(Decimal::from(1), 2),
            Decimal::from_str("2.00").unwrap()
        );
    }

    #[test]
    fn extra_sku_amount_parses_attr_price() {
        assert_eq!(
            extra_sku_amount(Some(r#"[{"attrPrice":1.5},{"attrPrice":"2.25"}]"#)),
            Decimal::from_str("3.75").unwrap()
        );
        assert_eq!(extra_sku_amount(Some("[]")), Decimal::ZERO);
        assert_eq!(extra_sku_amount(Some("not json")), Decimal::ZERO);
        assert_eq!(extra_sku_amount(None), Decimal::ZERO);
    }

    #[test]
    fn base_refund_amount_partial_uses_item_price() {
        let o = order(1, 0, "100.00", "10.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let order_items = vec![order_item(1, "20.00", 5), order_item(2, "10.00", 2)];
        let it1 = afs_item(1, 2);
        let oi1 = order_items[0].clone();
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&it1, &oi1)];
        // 部分售后：20.00 × 2 = 40.00
        assert_eq!(
            calculate_base_refund_amount(&o, &a, &order_items, &pairs),
            Decimal::from_str("40.00").unwrap()
        );
    }

    #[test]
    fn base_refund_amount_full_order_excludes_shipping() {
        let o = order(1, 0, "100.00", "10.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let order_items = vec![order_item(1, "20.00", 5)];
        let it1 = afs_item(1, 5);
        let oi1 = order_items[0].clone();
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&it1, &oi1)];
        // 整单售后：100.00 - 10.00 = 90.00
        assert_eq!(
            calculate_base_refund_amount(&o, &a, &order_items, &pairs),
            Decimal::from_str("90.00").unwrap()
        );
    }

    #[test]
    fn base_refund_amount_falls_back_to_stored_amount_without_items() {
        let o = order(1, 0, "100.00", "10.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "12.34");
        assert_eq!(
            calculate_base_refund_amount(&o, &a, &[], &[]),
            Decimal::from_str("12.34").unwrap()
        );
    }

    #[test]
    fn full_order_refund_requires_every_order_line_covered() {
        let order_items = vec![order_item(1, "10.00", 2), order_item(2, "5.00", 1)];
        let i1 = afs_item(1, 2);
        let i2 = afs_item(2, 1);
        let oi1 = order_items[0].clone();
        let oi2 = order_items[1].clone();
        assert!(is_full_order_refund(
            &order_items,
            &[(&i1, &oi1), (&i2, &oi2)]
        ));
        // 只覆盖一行 → 非整单
        assert!(!is_full_order_refund(&order_items, &[(&i1, &oi1)]));
        // 数量不足 → 非整单
        let i3 = afs_item(1, 1);
        assert!(!is_full_order_refund(
            &order_items,
            &[(&i3, &oi1), (&i2, &oi2)]
        ));
        // 空输入
        assert!(!is_full_order_refund(&[], &[(&i1, &oi1)]));
    }

    #[test]
    fn full_order_refund_handles_split_partial_aftersales() {
        // 回归：Java 用「条数」比较，两次部分售后（各 1 件 / 共 2 件）会被误判为非整单；
        // 本实现按数量合计判定，能正确识别为整单。
        let order_items = vec![order_item(1, "10.00", 2)];
        let i1 = afs_item(1, 1);
        let i2 = afs_item(1, 1);
        let oi1 = order_items[0].clone();
        assert!(is_full_order_refund(
            &order_items,
            &[(&i1, &oi1), (&i2, &oi1)]
        ));
    }

    #[test]
    fn final_refund_adds_shipping_when_pending_and_full_coverage() {
        // 待发货 + 整单覆盖 → 追加运费
        let o = order(1, 0, "100.00", "10.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let order_items = vec![order_item(1, "20.00", 5)];
        let i1 = afs_item(1, 5);
        let oi1 = order_items[0].clone();
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&i1, &oi1)];
        let ctx = RefundContext {
            order: &o,
            order_items: &order_items,
            current_aftersales: &a,
            current_pairs: pairs.as_slice(),
            completed_other_pairs: &[],
            earlier_other_pairs: &[],
            other_processed_refund_total: Decimal::ZERO,
        };
        // 基础额 = 100 - 10 = 90，入参 None → 90 + 10（运费）= 100
        assert_eq!(
            calculate_final_refund_amount(&ctx, None, true),
            Decimal::from_str("100.00").unwrap()
        );
    }

    #[test]
    fn final_refund_does_not_add_shipping_when_not_full_coverage() {
        let o = order(1, 0, "100.00", "10.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let order_items = vec![order_item(1, "20.00", 5)];
        let i1 = afs_item(1, 2); // 只申请 2 件
        let oi1 = order_items[0].clone();
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&i1, &oi1)];
        let ctx = RefundContext {
            order: &o,
            order_items: &order_items,
            current_aftersales: &a,
            current_pairs: pairs.as_slice(),
            completed_other_pairs: &[],
            earlier_other_pairs: &[],
            other_processed_refund_total: Decimal::ZERO,
        };
        // 基础额 = 40（部分售后），未覆盖整单 → 不加运费
        assert_eq!(
            calculate_final_refund_amount(&ctx, None, true),
            Decimal::from_str("40.00").unwrap()
        );
    }

    #[test]
    fn final_refund_caps_amount_over_base() {
        // 入参金额 > 基础额 → 封顶为 基础额 + 运费
        let o = order(1, 0, "100.00", "10.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let order_items = vec![order_item(1, "20.00", 5)];
        let i1 = afs_item(1, 2);
        let oi1 = order_items[0].clone();
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&i1, &oi1)];
        let ctx = RefundContext {
            order: &o,
            order_items: &order_items,
            current_aftersales: &a,
            current_pairs: pairs.as_slice(),
            completed_other_pairs: &[],
            earlier_other_pairs: &[],
            other_processed_refund_total: Decimal::ZERO,
        };
        // 基础额 40，入参 999 → min(999, 40+10) = 50
        assert_eq!(
            calculate_final_refund_amount(&ctx, Some(Decimal::from(999)), true),
            Decimal::from_str("50.00").unwrap()
        );
        // 入参 0（合法的「不退款」输入）不触发封顶分支
        assert_eq!(
            calculate_final_refund_amount(&ctx, Some(Decimal::ZERO), true),
            Decimal::ZERO
        );
    }

    #[test]
    fn final_refund_without_shipping_returns_input() {
        let o = order(1, 0, "100.00", "0.00");
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let order_items = vec![order_item(1, "20.00", 5)];
        let i1 = afs_item(1, 5);
        let oi1 = order_items[0].clone();
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&i1, &oi1)];
        let ctx = RefundContext {
            order: &o,
            order_items: &order_items,
            current_aftersales: &a,
            current_pairs: pairs.as_slice(),
            completed_other_pairs: &[],
            earlier_other_pairs: &[],
            other_processed_refund_total: Decimal::ZERO,
        };
        assert_eq!(
            calculate_final_refund_amount(&ctx, Some(Decimal::from(42)), true),
            Decimal::from(42)
        );
    }

    #[test]
    fn shipping_already_refunded_when_exceeding_order_total() {
        let o = order(1, 0, "100.00", "10.00");
        // 40 + 10 + 0 = 50 ≤ 100 → 未退过
        assert!(!is_shipping_already_refunded(
            &o,
            Decimal::from(40),
            Decimal::ZERO
        ));
        // 90 + 10 + 0 = 100 ≤ 100 → 未退过（Java 用严格大于）
        assert!(!is_shipping_already_refunded(
            &o,
            Decimal::from(90),
            Decimal::ZERO
        ));
        // 91 + 10 = 101 > 100 → 已退过
        assert!(is_shipping_already_refunded(
            &o,
            Decimal::from(91),
            Decimal::ZERO
        ));
        // 已有其他退款 5 → 90 + 10 + 5 = 105 > 100
        assert!(is_shipping_already_refunded(
            &o,
            Decimal::from(90),
            Decimal::from(5)
        ));
        // 无运费 → 恒 false
        let free = order(1, 0, "100.00", "0.00");
        assert!(!is_shipping_already_refunded(
            &free,
            Decimal::from(200),
            Decimal::ZERO
        ));
    }

    #[test]
    fn completing_last_items_detects_final_gap() {
        let order_items = vec![order_item(1, "10.00", 2), order_item(2, "10.00", 1)];
        let earlier = afs_item(1, 2);
        let current = afs_item(2, 1);
        let oi1 = order_items[0].clone();
        let oi2 = order_items[1].clone();

        // 「加上当前售后」的集合 = 已完成其他（行1，2件）+ 当前（行2，1件）= 全覆盖；
        // 「不加当前售后」的集合 = 更早集合（仅行1）→ 不能全覆盖
        // （对应 Java `allItems = earlierItems + currentItems` 与 `!covers(earlierItems)`）
        // → 当前售后补齐最后一个缺口
        let current_pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&current, &oi2)];
        assert!(is_completing_last_items_current(
            &order_items,
            &current_pairs,
            &[(earlier.clone(), oi1.clone())],
            &[(earlier.clone(), oi1.clone())],
        ));

        // 更早的售后已覆盖全部订单行 → 本次不是「补齐最后缺口」
        let earlier_full = afs_item(2, 1);
        assert!(!is_completing_last_items_current(
            &order_items,
            &current_pairs,
            &[(current.clone(), oi2.clone())],
            &[(earlier.clone(), oi1.clone()), (earlier_full, oi2.clone())],
        ));

        // 「加上当前售后」也不覆盖 → false（当前只申请 0 件）
        let small = afs_item(2, 0);
        let small_pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&small, &oi2)];
        assert!(!is_completing_last_items_current(
            &order_items,
            &small_pairs,
            &[(small.clone(), oi2.clone())],
            &[],
        ));

        // 空输入
        assert!(!is_completing_last_items_current(
            &[],
            &current_pairs,
            &[],
            &[]
        ));
        assert!(!is_completing_last_items_current(
            &order_items,
            &[],
            &[],
            &[]
        ));
    }

    #[test]
    fn calc_refund_product_amount_ceils_and_handles_full_order() {
        let o = order(1, 0, "100.00", "10.00");
        let order_items = vec![order_item(1, "9.995", 3)];
        let i1 = afs_item(1, 1);
        let oi1 = order_items[0].clone();
        // 部分售后：9.995 × 1 = 9.995 → 向上取整 10.00
        assert_eq!(
            calc_refund_product_amount(&o, &order_items, &[(&i1, &oi1)]),
            Decimal::from_str("10.00").unwrap()
        );
        // 整单售后：100 - 10 = 90
        let full = afs_item(1, 3);
        assert_eq!(
            calc_refund_product_amount(&o, &order_items, &[(&full, &oi1)]),
            Decimal::from_str("90.00").unwrap()
        );
        // 无明细 → 0
        assert_eq!(
            calc_refund_product_amount(&o, &order_items, &[]),
            Decimal::ZERO
        );
    }

    #[test]
    fn confirmed_order_includes_extra_sku_price() {
        let mut o = order(1, 0, "100.00", "0.00");
        o.order_status = 1; // 已确认，待发货
        let mut oi = order_item(1, "20.00", 2);
        oi.extra_sku_data = Some(r#"[{"attrPrice":5.00}]"#.into());
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        let i1 = afs_item(1, 1);
        let pairs: Vec<(&AftersalesItem, &OrderItem)> = vec![(&i1, &oi)];
        // (20 + 5) × 1 = 25
        assert_eq!(
            calculate_base_refund_amount(&o, &a, std::slice::from_ref(&oi), &pairs),
            Decimal::from_str("25.00").unwrap()
        );
        // 非「已确认」状态不吃附加规格
        let mut o2 = o.clone();
        o2.order_status = 2;
        assert_eq!(
            calculate_base_refund_amount(&o2, &a, std::slice::from_ref(&oi), &pairs),
            Decimal::from_str("20.00").unwrap()
        );
    }

    // ————————————————— 展示派生 —————————————————

    #[test]
    fn finance_flags_follow_java_semantics() {
        // 已完成 + 退款申请待处理 → 列表按「待财务退款」展示
        let done = aftersales(
            aftersales_type::PAY_RETURN,
            aftersales_status::COMPLETE,
            "10.00",
        );
        let wait = RefundApply {
            refund_status: RefundApplyStatus::Wait as i32,
            ..Default::default()
        };
        assert!(is_pending_finance_refund(&done, Some(&wait)));
        assert_eq!(
            resolve_display_status(&done, Some(&wait)),
            aftersales_status::APPROVED_FOR_PROCESSING
        );
        // 已拒绝 → 财务已拒绝优先于完成态
        let rejected = RefundApply {
            refund_status: RefundApplyStatus::Rejected as i32,
            refund_note: "资料不全".into(),
            ..Default::default()
        };
        assert!(is_finance_rejected(&done, Some(&rejected)));
        assert_eq!(
            finance_reject_reason(&done, Some(&rejected)).as_deref(),
            Some("资料不全")
        );
        // 拒绝原因为空 → 「未提供拒绝原因」
        let rejected_blank = RefundApply {
            refund_status: RefundApplyStatus::Rejected as i32,
            refund_note: "  ".into(),
            ..Default::default()
        };
        assert_eq!(
            finance_reject_reason(&done, Some(&rejected_blank)).as_deref(),
            Some("未提供拒绝原因")
        );
        // 非仅退款不参与财务拒绝判定
        let ret = aftersales(aftersales_type::RETURN, aftersales_status::COMPLETE, "10.00");
        assert!(!is_finance_rejected(&ret, Some(&rejected)));
        // 查不到退款申请 → 不覆盖「已完成」
        assert!(!is_pending_finance_refund(&done, None));
        assert_eq!(
            resolve_display_status(&done, None),
            aftersales_status::COMPLETE
        );
    }

    #[test]
    fn actual_refund_amount_zero_before_approval() {
        for st in [
            aftersales_status::IN_REVIEW,
            aftersales_status::REFUSE,
            aftersales_status::CANCEL,
        ] {
            let a = aftersales(aftersales_type::PAY_RETURN, st, "10.00");
            assert_eq!(actual_refund_amount(&a), Decimal::ZERO, "status={st}");
        }
        for st in [
            aftersales_status::APPROVED_FOR_PROCESSING,
            aftersales_status::SEND_BACK,
            aftersales_status::RETURNED,
            aftersales_status::COMPLETE,
        ] {
            let a = aftersales(aftersales_type::PAY_RETURN, st, "10.00");
            assert_eq!(
                actual_refund_amount(&a),
                Decimal::from_str("10.00").unwrap(),
                "status={st}"
            );
        }
    }

    #[test]
    fn audit_log_info_omits_empty_reply() {
        assert_eq!(
            build_audit_log_info(aftersales_status::APPROVED_FOR_PROCESSING, "同意"),
            "审核通过:同意"
        );
        assert_eq!(
            build_audit_log_info(aftersales_status::REFUSE, "  "),
            "审核未通过"
        );
    }

    #[test]
    fn item_vo_maps_price_with_tax_and_sku_value() {
        let mut o = order(1, 0, "100.00", "0.00");
        o.tax_rate = Some(Decimal::from(6));
        let mut oi = order_item(1, "10.00", 3);
        oi.order_sn = "SN1".into();
        oi.product_name = "商品A".into();
        oi.sku_data = r#"[{"name":"颜色","value":"红"}]"#.into();
        let row = AftersalesItemRow {
            item: afs_item(1, 2),
            order_item: Some(oi),
        };
        let vo = to_item_vo(&row, Some(&o));
        assert_eq!(vo.order_item_id, Some(1));
        assert_eq!(vo.number, Some(2));
        assert_eq!(vo.quantity, 3);
        // 单价 10.00 + 税费 0.60 = 10.60
        assert_eq!(vo.price, Decimal::from_str("10.60").unwrap());
        assert_eq!(vo.tax_rate_amount, Decimal::from_str("0.60").unwrap());
        assert_eq!(vo.tax_total_amount, Decimal::from_str("1.20").unwrap());
        assert_eq!(vo.sku_value, "颜色:红");
        assert_eq!(vo.product_name, "商品A");
        assert_eq!(vo.supply_price, None);
    }

    #[test]
    fn item_vo_without_order_item_keeps_zero_defaults() {
        let row = AftersalesItemRow {
            item: afs_item(9, 1),
            order_item: None,
        };
        let vo = to_item_vo(&row, None);
        assert_eq!(vo.price, Decimal::ZERO);
        assert_eq!(vo.tax_rate, Decimal::ZERO);
        assert_eq!(vo.tax_total_amount, Decimal::ZERO);
        assert_eq!(vo.product_name, "");
        assert!(vo.sku_data.is_empty());
    }

    #[test]
    fn parse_json_list_tolerates_bad_input() {
        assert_eq!(parse_json_list(Some(r#"[{"a":1}]"#)).len(), 1);
        assert!(parse_json_list(Some("")).is_empty());
        assert!(parse_json_list(Some("not json")).is_empty());
        assert!(parse_json_list(None).is_empty());
    }

    #[test]
    fn fmt_time_handles_zero_and_positive() {
        assert_eq!(fmt_time(0), "");
        assert_eq!(fmt_time(-1), "");
        assert_eq!(fmt_time(1_700_000_000).len(), 19);
    }

    #[test]
    fn row_vo_exposes_mark_and_admin_note() {
        let mut o = order(1, 0, "100.00", "0.00");
        o.mark = Some(2);
        o.admin_note = "备注".into();
        o.order_sn = "SN9".into();
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "12.30");
        let vo = build_row_vo(&a, &[], Some(&o), false, false, 1_700_000_000);
        assert_eq!(vo.order_sn, "SN9");
        assert_eq!(vo.mark, Some(2));
        assert_eq!(vo.admin_note.as_deref(), Some("备注"));
        assert_eq!(vo.refund_amount, "12.30");
        assert_eq!(vo.status_name, "审核处理中");
        assert_eq!(vo.aftersales_type_name, "退货/退款");
        assert_eq!(vo.actual_refund_amount, Decimal::ZERO);
        assert_eq!(vo.server_time.len(), 19);
    }

    #[test]
    fn detail_vo_uses_admin_status_copy() {
        let a = aftersales(
            aftersales_type::PAY_RETURN,
            aftersales_status::APPROVED_FOR_PROCESSING,
            "10.00",
        );
        let vo = build_detail_vo(&a, &[], None, &[], None, 1_700_000_000);
        assert_eq!(vo.status_name, "平台处理中");
        assert_eq!(
            vo.status_description.as_deref(),
            Some("售后已通过，平台退款处理中")
        );
        // 审核中无副文案（由倒计时驱动）
        let a2 = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "10.00");
        let vo2 = build_detail_vo(&a2, &[], None, &[], None, 1_700_000_000);
        assert_eq!(vo2.status_name, "买家申请售后");
        assert_eq!(vo2.status_description, None);
    }

    #[test]
    fn log_vo_maps_fields() {
        let log = AftersalesLog {
            log_id: 5,
            aftersale_id: 7,
            log_info: "拒收退货：破损".into(),
            add_time: 1_700_000_000,
            admin_name: "admin".into(),
            refund_money: Decimal::ZERO,
            refund_type: 0,
            refund_desc: String::new(),
            user_name: Some("u1".into()),
            return_pic: Some(r#"["a.png"]"#.into()),
            shop_id: Some(0),
            vendor_id: None,
        };
        let vo = to_log_vo(&log);
        assert_eq!(vo.log_id, 5);
        assert_eq!(vo.aftersales_id, "7");
        assert_eq!(vo.return_pic.len(), 1);
        assert_eq!(vo.add_time.len(), 19);
        assert_eq!(vo.admin_name, "admin");
    }

    #[test]
    fn is_at_least_processing_matches_java_compare() {
        assert!(!is_at_least_processing(0));
        assert!(!is_at_least_processing(1));
        assert!(is_at_least_processing(2));
        assert!(is_at_least_processing(5));
    }

    #[test]
    fn vendor_return_aftersales_requires_vendor_deployment() {
        // is-vendor = 0 的部署下恒为 false（与 application.yaml 一致）
        let a = aftersales(aftersales_type::RETURN, aftersales_status::IN_REVIEW, "0.00");
        assert!(!is_vendor_return_aftersales(&a));
        let mut with_vendor = a.clone();
        with_vendor.vendor_id = Some(3);
        assert!(!is_vendor_return_aftersales(&with_vendor));
        // 仅退款也不计
        let pr = aftersales(aftersales_type::PAY_RETURN, aftersales_status::IN_REVIEW, "0.00");
        assert!(!is_vendor_return_aftersales(&pr));
    }

    #[test]
    fn return_address_fallback_builds_java_style_string() {
        assert_eq!(
            delivery_address_from_json(Some(
                r#"{"sender":"张三","mobile":"13800000000","provinceName":"山东省","cityName":"青岛市","areaName":"市南区","address":"香港中路1号"}"#
            ))
            .as_deref(),
            Some("张三 13800000000 山东省青岛市市南区香港中路1号")
        );
        assert!(delivery_address_from_json(Some("{}")).is_none());
        assert!(delivery_address_from_json(Some("not-json")).is_none());
        assert_eq!(
            build_delivery_address(
                Some("平台"),
                Some("400-000"),
                Some("山东省"),
                Some("青岛市"),
                Some("市南区"),
                Some("某路")
            )
            .as_deref(),
            Some("平台 400-000 山东省青岛市市南区某路")
        );
    }
}