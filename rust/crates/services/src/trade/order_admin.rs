//! 订单管理服务（trade::order_admin）：订单列表、详情、状态流转、发货、
//! 打印、导出、批量发货导入等 **34 条**管理端接口
//! （wave1 已实现 `list` / `detail` / `deliver` / `cancelOrderByAdmin` 4 条，
//! wave3 扩展其余 30 条）。
//!
//! 对应原 Java `OrderServiceImpl`（6585 行）与 `OrderController` 中的对应链路。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：`buildQueryWrapper` 的全部筛选条件在仓储层按同序绑定；
//!   列表所需关联数据（明细 / 用户 / 店铺 / 在售商品 / 整单售后 / 拼团记录）
//!   全部**批量预取**，与原 Java 的 N+1 消除改造一致。
//! - `detail`：失效父单（`is_valid = 0` 且无子单）返回 `5004 订单已被拆分为多个订单`；
//!   后台状态头文案覆盖仅在「状态名为标准文案」时生效（对齐 Java 的字符串比较）。
//! - `deliver`：状态门（仅「已确认，待发货」可发货）+ 整单售后拦截 +
//!   发货数量校验 + **原子条件更新**（`WHERE order_status = 1`，防并发重复发货）。
//! - `cancelOrderByAdmin`：仅「已确认，待发货」可取消，按整单生成
//!   `aftersales`（类型=仅退款，状态=审核处理中）+ 明细 + 日志，单事务写入。
//! - wave3 新增（30 条）：`statusCount` / `setAdminNote` / `parentDetail` /
//!   `setConfirm` / `splitStoreOrder` / `setPaid` / `cancelOrder` / `delOrder` /
//!   `modifyMoney` / `modifyConsignee` / `confirmReceipt` / `modifyShipping` /
//!   `modifyProduct` / `getAddProductInfo` / `orderPrint` / `orderPrintBill` /
//!   `severalDetail` / `batch` / `printSeveral` / 导出标签 3 条 / `orderExport` /
//!   `shippingInfo` / `getOrderPageConfig` / `changeOrderStatus` / `remindDeliver` /
//!   批量发货导入 2 条 / `downloadDeliverTemplate`。
//!
//! **仍未迁移（偏差已在各方法注释列明）**：
//! - 多店铺/多供应商支付后拆单（`createSplitOrder` 的尾差/税费补齐 + 分销拆单）
//! - 定金预售支付状态机（`presaleType = 2`）
//! - 拼团限购恢复、拼团退款解散整团
//! - 售后自动退款/退款申请创建（财务域）
//! - 微信小程序发货信息录入/RabbitMQ 延时任务
//! - xlsx 读写（部分导出/批量导入降级为结构化 JSON/CSV）
//!
//! 已实现：部分发货拆单、电子面单（KDNiao 1007）、快递鸟 1002 轨迹、
//! 取消订单积分/余额/优惠券/秒杀/礼品卡返还、催发货/确认收货后台消息、
//! 商品改价 `order_amount_detail` 店铺级分摊。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_common::utils;
use nextrade_core::entities::{Aftersales, AftersalesItem, AftersalesLog, Order, OrderItem, Product, Shop, User};
use nextrade_core::models::trade::order_admin::{
    aftersales_status, aftersales_type, join_status, order_type, pay_type, shipping_method,
    DeliverOrderParam, ExportItemSaveParam, LogisticsApiLogVO, OrderAdminNoteParam, OrderActionVO,
    OrderBatchParam, OrderChangeStatusParam, OrderDeliverImportErrorVO, OrderDeliverImportFailVO,
    OrderDeliverImportResultVO, OrderDeliverImportRow, OrderDeliverTemplateRow,
    OrderDeliverTemplateVO, OrderExportItemVO, OrderExportRowVO, OrderExportVO, OrderItemParam,
    OrderItemVO, OrderListQuery, OrderModifyConsigneeParam, OrderModifyMoneyParam,
    OrderModifyProductParam, OrderModifyShippingParam, OrderPageConfigVO, OrderShopVO,
    OrderStatus, OrderStatusCountVO, OrderUserVO, OrderVO, ParentOrderDetailVO, PayStatus,
    ProductAddInfoVO, ShippingInfoVO, ShippingStatus, ShippingTraceVO, AftersalesApplyAdminParam,
    ORDER_DELIVER_TEMPLATE_DESCRIPTION, ORDER_DELIVER_TEMPLATE_HEADERS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::order_admin::OrderPatch;
use nextrade_infrastructure::db::Repositories;

/// 待支付订单支付倒计时兜底分钟数（Java `AUTO_CANCEL_ORDER_MINUTE` 未配置时同为 30）。
const DEFAULT_CANCEL_MINUTE: i32 = 30;

/// 订单类型「不支持售后」的取值（Java `OrderType.getNoAfterSalesType`：7 付费商品、8 卡密）。
fn is_no_aftersales_type(order_type: Option<i32>) -> bool {
    matches!(order_type, Some(7 | 8))
}

/// 预约订单（Java `OrderType.isReservationOrder`：9 预约到店、10 预约上门）。
fn is_reservation_order(order_type: Option<i32>) -> bool {
    matches!(order_type, Some(order_type::RESERVATION_STORE | order_type::RESERVATION_HOME))
}

/// 同城配送（Java `OrderDeliveryTypeEnum.SAME_CITY`，`order.shipping_type_id == 3`）。
fn is_same_city(shipping_type_id: i32) -> bool {
    shipping_type_id == 3
}

/// 门店状态文案（Java `ShopType.getTypeName`：1 开业 4 停业 10 打烊）。
fn shop_status_text(status: Option<i32>) -> String {
    match status {
        Some(1) => "开业".into(),
        Some(4) => "停业".into(),
        Some(10) => "打烊".into(),
        _ => String::new(),
    }
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

/// JSON 数组字符串 → `Vec<i32>`（对应 Java `JsonUtil.jsonToList`，非法时返回空表）。
fn parse_i32_list(raw: &str) -> Vec<i32> {
    serde_json::from_str::<Vec<i32>>(raw).unwrap_or_default()
}

/// JSON 数组字符串 → `Vec<String>`。
fn parse_string_list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

/// 解析 JSON 对象字符串（非法返回 None）。
fn parse_json(raw: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(raw).ok()
}

/// 解析收货人信息 JSON；`regionName` 为空且 `regionNames` 非空时按 Java 逻辑补全。
fn parse_address_data(raw: Option<&str>) -> Option<serde_json::Value> {
    let mut v = parse_json(raw?)?;
    if !v.is_object() {
        return Some(v);
    }
    let region_name_blank = v
        .get("regionName")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|s| s.trim().is_empty());
    if region_name_blank {
        let joined = v
            .get("regionNames")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        if !joined.is_empty() {
            v["regionName"] = serde_json::Value::String(joined);
        }
    }
    Some(v)
}

/// 订单状态名（按订单类型/配送类型选择枚举，对齐 Java `list`/`OrderVO` 构造）。
fn order_status_name(order: &Order) -> String {
    let code = order.order_status;
    if is_reservation_order(order.order_type) {
        OrderStatus::reservation_name(code).to_string()
    } else if is_same_city(order.shipping_type_id) {
        OrderStatus::same_city_name(code).to_string()
    } else {
        OrderStatus::name_of(code)
    }
}

/// 发货计划：本次应发（含售后占用数量）与剩余数量。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeliverPlan {
    /// `(item_id, 本次发货数量)`——已并入售后占用数量
    pub deliver: Vec<(i32, i32)>,
    /// `(item_id, 剩余数量)`
    pub remain: Vec<(i32, i32)>,
}

/// 计算发货/剩余明细。
///
/// 逐条对照 Java `DeliverOrderParam#getDeliverItems` / `#getRemainItems`：
/// - 未指定发货数量的明细：若存在有效售后占用则按占用数量发货，否则整条留作剩余；
/// - 指定发货数量的明细：必须 `0 < 数量 <= 购买数量`，否则「发货数量错误」；
///   未全额发货时，售后占用数量并入发货单，超出可发数量报
///   「发货数量超出可发数量，该商品有部分数量正在售后中」。
pub fn compute_deliver_plan(
    items: &[OrderItem],
    deliver_map: &HashMap<i32, i32>,
    aftersales: &HashMap<i32, i32>,
) -> Result<DeliverPlan> {
    let mut plan = DeliverPlan::default();
    for item in items {
        let qty = item.quantity;
        match deliver_map.get(&item.item_id).copied() {
            None => {
                match aftersales.get(&item.item_id).copied().filter(|n| *n > 0) {
                    None => plan.remain.push((item.item_id, qty)),
                    Some(occupied) => {
                        let remain = qty - occupied;
                        if remain > 0 {
                            plan.remain.push((item.item_id, remain));
                        }
                        plan.deliver.push((item.item_id, occupied));
                    }
                }
            }
            Some(deliver_qty) => {
                if deliver_qty <= 0 || deliver_qty > qty {
                    return Err(Error::biz_msg("发货数量错误"));
                }
                let occupied = aftersales.get(&item.item_id).copied().unwrap_or(0);
                if deliver_qty == qty {
                    plan.deliver.push((item.item_id, qty));
                } else {
                    let total = deliver_qty.saturating_add(occupied);
                    if total > qty {
                        return Err(Error::biz_msg(
                            "发货数量超出可发数量，该商品有部分数量正在售后中",
                        ));
                    }
                    plan.deliver.push((item.item_id, total));
                    let remain = qty - deliver_qty - occupied;
                    if remain > 0 {
                        plan.remain.push((item.item_id, remain));
                    }
                }
            }
        }
    }
    Ok(plan)
}

/// `build_vo` 的外部数据集合。
struct VoInput<'a> {
    order: &'a Order,
    items: &'a [OrderItem],
    user: Option<&'a User>,
    shop: Option<&'a Shop>,
    /// 是否整单售后进行中（`confirmReceipt` 拦截条件）
    full_aftersales: bool,
    /// 订单内仍有在售商品（`rebuy` 条件）
    rebuy_available: bool,
    /// 拼团记录 `(groupon_promotion_record_id, join_status)`
    groupon: Option<(i64, i32)>,
    /// Java `OrderVO.wayBill`：`list` 走默认构造（false），`detail` 走 `OrderVO(Order)`（true）
    way_bill: bool,
    now: i32,
}

/// 组装订单 VO（对齐 Java `OrderVO` 构造 + `list` 循环内的派生字段）。
fn build_vo(input: VoInput<'_>) -> OrderVO {
    let VoInput {
        order: o,
        items,
        user,
        shop,
        full_aftersales,
        rebuy_available,
        groupon,
        way_bill,
        now,
    } = input;

    let region_names = parse_string_list(&o.region_names);
    let user_address = if region_names.is_empty() {
        None
    } else {
        Some(format!("{} {}", region_names.join(" "), o.address))
    };

    let mut status_name = order_status_name(o);
    let mut join_status_code = None;
    let mut join_status_desc = None;
    let mut groupon_record_id = None;
    let mut deliver_blocked_by_groupon = false;

    if o.order_type == Some(order_type::PIN) {
        let cancelled = o.order_status == OrderStatus::Cancelled as i32;
        if cancelled {
            join_status_desc = Some("已取消".to_string());
        }
        if let Some((rec_id, js)) = groupon {
            groupon_record_id = Some(rec_id);
            join_status_code = Some(js);
            if !cancelled {
                join_status_desc = Some(join_status::description(js));
            }
            if o.order_status == OrderStatus::Confirmed as i32 && js == join_status::WAIT_GROUP {
                status_name = "已付款，待成团".to_string();
                deliver_blocked_by_groupon = true;
            }
            if js == join_status::FAIL {
                status_name = if cancelled {
                    "拼团失败".to_string()
                } else {
                    "拼团失败，待退款".to_string()
                };
            }
        }
    }

    // 待支付订单支付截止时间（Java：下单时间 + 自动取消分钟数，配置缺失时兜底 30）
    let pay_end_time = if o.order_status == OrderStatus::Pending as i32 && o.add_time > 0 {
        Some(fmt_time(
            o.add_time.saturating_add(DEFAULT_CANCEL_MINUTE * 60),
        ))
    } else {
        None
    };

    let mut actions = available_actions(o, full_aftersales, rebuy_available);
    if deliver_blocked_by_groupon {
        actions.deliver = false;
    }

    OrderVO {
        order_id: o.order_id,
        order_type: o.order_type,
        order_sn: o.order_sn.clone(),
        user_id: o.user_id,
        parent_order_id: o.parent_order_id,
        parent_order_sn: o.parent_order_sn.clone(),
        order_status: o.order_status,
        shipping_status: o.shipping_status,
        pay_status: o.pay_status,
        consignee: o.consignee.clone(),
        address: o.address.clone(),
        mobile: o.mobile.clone(),
        email: o.email.clone(),
        buyer_note: o.buyer_note.clone(),
        admin_note: o.admin_note.clone(),
        shipping_method: o.shipping_method,
        logistics_id: o.logistics_id,
        logistics_provider: o.logistics_provider,
        logistics_name: o.logistics_name.clone(),
        shipping_type_id: o.shipping_type_id,
        shipping_type_name: o.shipping_type_name.clone(),
        tracking_no: o.tracking_no.clone(),
        pay_type_id: o.pay_type_id,
        use_points: o.use_points,
        is_need_commisson: o.is_need_commisson,
        distribution_status: o.distribution_status,
        referrer_user_id: o.referrer_user_id,
        is_del: o.is_del,
        shop_id: o.shop_id,
        is_store_splited: o.is_store_splited,
        is_valid: o.is_valid,
        comment_status: o.comment_status,
        total_amount: o.total_amount,
        paid_amount: o.paid_amount,
        unpaid_amount: o.unpaid_amount,
        unrefund_amount: o.unrefund_amount,
        product_amount: o.product_amount,
        coupon_amount: o.coupon_amount,
        points_amount: o.points_amount,
        discount_amount: o.discount_amount,
        balance: o.balance,
        credit_amount: o.credit_amount,
        is_credit_order: o.is_credit_order,
        online_paid_amount: o.online_paid_amount,
        offline_paid_amount: o.offline_paid_amount,
        service_fee: o.service_fee,
        shipping_fee: o.shipping_fee,
        invoice_fee: o.invoice_fee,
        order_extension: o.order_extension.clone(),
        order_source: o.order_source.clone(),
        out_trade_no: o.out_trade_no.clone(),
        is_settlement: o.is_settlement,
        is_exchange_order: o.is_exchange_order,
        mark: o.mark,
        is_presale: o.is_presale,
        presale_type: o.presale_type,
        vendor_id: o.vendor_id,
        cancel_remark: o.cancel_remark.clone(),
        round_off_cents_amount: o.round_off_cents_amount,
        round_off_dime_amount: o.round_off_dime_amount,
        tax_rate: o.tax_rate.unwrap_or(Decimal::ZERO),
        tax_rate_amount: o.tax_rate_amount.unwrap_or(Decimal::ZERO),
        discount_change_price: o.discount_change_price,
        order_status_name: status_name,
        order_status_tip: None,
        pay_status_name: PayStatus::name_of(o.pay_status),
        shipping_status_name: ShippingStatus::name_of(o.shipping_status),
        add_time: fmt_time(o.add_time),
        pay_time: (o.pay_time > 0).then(|| fmt_time(o.pay_time)),
        shipping_time: (o.shipping_time > 0).then(|| fmt_time(o.shipping_time)),
        received_time: (o.received_time > 0).then(|| fmt_time(o.received_time)),
        region_ids: parse_i32_list(&o.region_ids),
        region_names,
        address_data: parse_address_data(o.address_data.as_deref()),
        invoice_data: o.invoice_data.as_deref().and_then(parse_json),
        user_address,
        server_time: Some(fmt_time(now)),
        pay_end_time,
        way_bill,
        pre_order_status: None,
        pre_order_status_desc: None,
        is_change_order_status: 0,
        to_aftersalses: actions.to_aftersales,
        join_status: join_status_code,
        join_status_desc,
        groupon_promotion_record_id: groupon_record_id,
        items: items.iter().map(to_item_vo).collect(),
        user: user.map(to_user_vo),
        shop: shop.map(to_shop_vo),
        shop_name: shop.and_then(|s| s.shop_title.clone()),
        vendor_name: String::new(),
        available_actions: actions,
        host: None,
        print_time: None,
        sub_order_ids: None,
        pay_code: None,
    }
}

/// 订单明细实体 → VO。
fn to_item_vo(it: &OrderItem) -> OrderItemVO {
    OrderItemVO {
        item_id: it.item_id,
        order_id: it.order_id,
        order_sn: it.order_sn.clone(),
        user_id: it.user_id,
        product_id: it.product_id,
        product_name: it.product_name.clone(),
        product_sn: it.product_sn.clone(),
        pic_thumb: it.pic_thumb.clone(),
        sku_id: it.sku_id,
        sku_data: parse_sku_data_array(&it.sku_data),
        price: it.price,
        origin_price: it.origin_price,
        quantity: it.quantity,
        delivery_quantity: it.delivery_quantity,
        product_type: it.product_type,
        is_gift: it.is_gift,
        shop_id: it.shop_id,
        suppliers_id: it.suppliers_id,
        vendor_id: it.vendor_id,
        card_group_name: it.card_group_name.clone(),
        promotion_data: it.promotion_data.clone(),
        sku_value: None,
        product_weight: None,
        commission: it.commission.clone(),
    }
}

/// 用户实体 → VO。
fn to_user_vo(u: &User) -> OrderUserVO {
    OrderUserVO {
        user_id: u.user_id,
        username: u.username.clone(),
        nickname: u.nickname.clone(),
        avatar: u.avatar.clone(),
        mobile: u.mobile.clone(),
        email: u.email.clone(),
    }
}

/// 店铺实体 → VO。
fn to_shop_vo(s: &Shop) -> OrderShopVO {
    OrderShopVO {
        shop_id: s.shop_id,
        shop_title: s.shop_title.clone().unwrap_or_default(),
        status_text: shop_status_text(s.status),
        kefu_inlet: s.kefu_inlet.clone(),
        shop_logo: s.shop_logo.clone(),
    }
}

/// 可操作项（逐条对照 Java `OrderServiceImpl#availableActions`）。
///
/// 依赖未迁移服务/配置的项取安全默认值（`to_aftersales` 恒 false：
/// Java 由 `orderAfterSalesService.getOrderAfterSalesToAction` 判定）；
/// `to_comment` 依赖配置项 `product_comment_enabled`，同样恒 false。
fn available_actions(o: &Order, full_aftersales: bool, rebuy_available: bool) -> OrderActionVO {
    let mut a = OrderActionVO::default();
    match OrderStatus::of(o.order_status) {
        Some(OrderStatus::Pending) => {
            a.set_confirm = true;
            a.cancel_order = true;
            a.modify_order = true;
            a.modify_shipping_info = true;
            a.modify_order_money = true;
            a.modify_order_product = true;
            a.set_paid = true;
            a.modify_order_consignee = true;
            a.to_pay = true;
            // Java：后台用户（`SecurityUtils.isAdminUser()` 即 adminId > 0，本切片
            // 四条端点全在管理端）不允许对「非线下支付」订单执行「设置已支付」
            if o.pay_type_id != pay_type::OFFLINE {
                a.set_paid = false;
            }
        }
        Some(OrderStatus::Confirmed) => {
            a.split_order = true;
            a.deliver = true;
            a.modify_order_consignee = true;
            if o.pay_status != PayStatus::Paid as i32 {
                a.set_paid = true;
                a.cancel_order = true;
            }
        }
        Some(OrderStatus::Processing) => {
            // Java 另有「自提订单不展示确认收货」判断（`OrderVO.isPickup` 由 O2O 域写入，未迁移）
            if !full_aftersales {
                a.confirm_receipt = true;
            }
            a.modify_shipping_info = true;
        }
        Some(OrderStatus::Completed) => {
            a.rebuy = true;
        }
        Some(OrderStatus::Cancelled) => {
            a.cancel_order = false;
            a.rebuy = true;
            a.del_order = true;
        }
        Some(OrderStatus::Invalid) => {
            a.del_order = true;
        }
        None => {}
    }

    if o.pay_status == PayStatus::Paid as i32 {
        a.set_unpaid = false;
    }

    // 未支付拆单子单：只读，禁止所有操作
    let is_unpaid_split_child =
        o.parent_order_id.is_some_and(|p| p > 0) && o.pay_status == PayStatus::Unpaid as i32;
    if is_unpaid_split_child {
        return OrderActionVO::default();
    }

    // 互斥的 kill-switch（Java `OrderServiceImpl:1082-1086` 逐字对照）：
    //   if (isStoreSplited == 1) setSplitOrder(false); else setDeliver(false);
    // 即已是店铺拆单订单时不再提供「拆单」入口；尚未拆单时反而**关闭发货**入口
    // （必须先拆单才能发货）。两者永远不可能同时为 true。
    if o.is_store_splited == 1 {
        a.split_order = false;
    } else {
        a.deliver = false;
    }
    if o.is_del == 1 {
        a.del_order = false;
    }
    if o.order_type.is_some_and(|t| t > 1) {
        a.rebuy = false;
        if is_no_aftersales_type(o.order_type) {
            a.to_aftersales = false;
        }
    }
    if a.rebuy && !rebuy_available {
        a.rebuy = false;
    }
    a
}

pub struct OrderAdminService {
    repos: Arc<Repositories>,
}

impl OrderAdminService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 —————————————————————

    /// 订单分页（对应 Java `OrderController.list` / `OrderServiceImpl#list`）。
    ///
    /// `platform_view`：Java 用请求头 `x-shop-id` / `x-vendor-id` 判定
    /// 「平台 / 店铺（门店、供应商）视角」；Rust 侧 JWT 尚未签发 shop_id，
    /// 改由路由层根据 JWT 的 `admin_type` 传入（`admin` 或缺失 → 平台视角）。
    pub async fn list(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
    ) -> Result<PageResult<OrderVO>> {
        let mut query = q.clone();
        if let Some(ref tag_ids) = query.member_tag_ids {
            if !tag_ids.is_empty() {
                let match_all = query.member_tag_match_mode == Some(1);
                let uids = self
                    .repos
                    .marketing
                    .coupon_manage
                    .resolve_user_ids_by_tags(tag_ids, match_all)
                    .await?;
                query.tag_user_ids = Some(uids);
            }
        }

        let page = query.page_query();
        let (total, orders) = self.repos.trade.order_admin.paginate_orders(&query, platform_view).await?;
        if orders.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let order_ids: Vec<i32> = orders.iter().map(|o| o.order_id).collect();
        let items = self
            .repos.trade.order_admin
            .find_items_by_order_ids(&order_ids)
            .await?;
        let mut items_by_order: HashMap<i32, Vec<OrderItem>> = HashMap::new();
        for it in items {
            items_by_order.entry(it.order_id).or_default().push(it);
        }

        let user_ids = distinct_i32(orders.iter().map(|o| o.user_id));
        let users = self.repos.trade.order_admin.find_users(&user_ids).await?;
        let user_map: HashMap<i32, User> = users.into_iter().map(|u| (u.user_id, u)).collect();

        let shop_ids = distinct_i32(orders.iter().map(|o| o.shop_id).filter(|v| *v > 0));
        let shops = self.repos.trade.order_admin.find_shops(&shop_ids).await?;
        let shop_map: HashMap<i32, Shop> = shops.into_iter().map(|s| (s.shop_id, s)).collect();

        let covered: HashSet<i32> = self
            .repos.trade.order_admin
            .fully_covered_order_ids(&order_ids, &aftersales_status::valid_statuses())
            .await?
            .into_iter()
            .collect();

        let product_ids = distinct_i32(
            orders
                .iter()
                .flat_map(|o| items_by_order.get(&o.order_id).map(Vec::as_slice).unwrap_or(&[]))
                .map(|it| it.product_id),
        );
        let on_sale: HashSet<i32> = self
            .repos.trade.order_admin
            .on_sale_product_ids(&product_ids)
            .await?
            .into_iter()
            .collect();

        let groupon: HashMap<i32, (i64, i32)> = self
            .repos.trade.order_admin
            .find_groupon_records(&order_ids)
            .await?
            .into_iter()
            .map(|(order_id, rec_id, js)| (order_id, (rec_id, js)))
            .collect();

        let now = utils::now_seconds() as i32;
        let mut list = Vec::with_capacity(orders.len());
        for o in &orders {
            let empty: Vec<OrderItem> = Vec::new();
            let order_items = items_by_order.get(&o.order_id).unwrap_or(&empty);
            let rebuy_available = order_items.is_empty()
                || order_items.iter().any(|it| on_sale.contains(&it.product_id));
            list.push(build_vo(VoInput {
                order: o,
                items: order_items,
                user: user_map.get(&o.user_id),
                shop: (o.shop_id > 0)
                    .then(|| shop_map.get(&o.shop_id))
                    .flatten(),
                full_aftersales: covered.contains(&o.order_id),
                rebuy_available,
                groupon: groupon.get(&o.order_id).copied(),
                way_bill: false,
                now,
            }));
        }
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 详情 —————————————————————

    /// 订单详情（对应 Java `OrderController.detail` / `OrderServiceImpl#detail(id)`）。
    pub async fn detail(&self, order_id: i32) -> Result<OrderVO> {
        let order = self
            .repos.trade.order_admin
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;

        // 失效父单（已拆单）且找不到子单 → Java 抛 5004「订单已被拆分为多个订单」
        if order.is_valid == 0 && self.repos.trade.order_admin.count_children(order_id).await? == 0 {
            return Err(Error::biz(5004, "订单已被拆分为多个订单"));
        }

        let items = self.repos.trade.order_admin.find_items(order_id).await?;
        let users = self.repos.trade.order_admin.find_users(&[order.user_id]).await?;
        let shops = if order.shop_id > 0 {
            self.repos.trade.order_admin.find_shops(&[order.shop_id]).await?
        } else {
            Vec::new()
        };
        let total_qty = self.repos.trade.order_admin.order_items_total_quantity(order_id).await?;
        let covered_qty = self
            .repos.trade.order_admin
            .aftersales_covered_quantity(order_id, &aftersales_status::valid_statuses())
            .await?;
        let product_ids = distinct_i32(items.iter().map(|it| it.product_id));
        let on_sale: HashSet<i32> = self
            .repos.trade.order_admin
            .on_sale_product_ids(&product_ids)
            .await?
            .into_iter()
            .collect();
        let groupon = self
            .repos.trade.order_admin
            .find_groupon_records(&[order_id])
            .await?
            .into_iter()
            .next()
            .map(|(_, rec_id, js)| (rec_id, js));

        let rebuy_available =
            items.is_empty() || items.iter().any(|it| on_sale.contains(&it.product_id));
        let mut vo = build_vo(VoInput {
            order: &order,
            items: &items,
            user: users.first(),
            shop: shops.first(),
            full_aftersales: total_qty > 0 && covered_qty >= total_qty,
            rebuy_available,
            groupon,
            way_bill: true,
            now: utils::now_seconds() as i32,
        });

        // 后台状态头文案覆盖：仅当当前状态名仍为标准文案时覆盖（Java 用字符串相等判断）
        if let Some(admin) = OrderStatus::of(order.order_status)
            .and_then(OrderStatus::admin_name)
            .filter(|_| vo.order_status_name == OrderStatus::name_of(order.order_status))
        {
            vo.order_status_name = admin.to_string();
        }
        vo.order_status_tip = OrderStatus::of(order.order_status).map(|s| s.tip().to_string());
        Ok(vo)
    }

    // ————————————————————— 发货 —————————————————————

    /// 发货（对应 Java `OrderController.deliver` / `OrderServiceImpl#deliverOrder`）。
    ///
    /// 支持整单与部分发货：部分发货时把未发商品拆成待发货子订单
    /// （`createSplitOrder` 简化版），并写拆单日志。
    pub async fn deliver(&self, param: &DeliverOrderParam) -> Result<()> {
        let order_id = param
            .id
            .ok_or_else(|| Error::Validation("订单 ID不能为空".into()))?;
        let order = self
            .repos.trade.order_admin
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::biz_msg("未知的订单"))?;

        // 状态门：等价 Java `Assert.isTrue(orderVO.getAvailableActions().getDeliver(), "状态错误")`
        // —— 已确认待发货 + 已按店铺拆分（支付成功时自动拆分）+ 非未支付拆单子单。
        // 整单售后标记只影响 confirmReceipt，不影响 deliver，故此处传 false。
        if !available_actions(&order, false, true).deliver {
            return Err(Error::biz_msg("状态错误"));
        }
        // 拼团待成团不允许发货（Java `detail` 中把 availableActions.deliver 置 false）
        let groupon = if order.order_type == Some(order_type::PIN) {
            self.repos.trade.order_admin
                .find_groupon_records(&[order_id])
                .await?
                .into_iter()
                .next()
        } else {
            None
        };
        if groupon.is_some_and(|(_, _, js)| js == join_status::WAIT_GROUP) {
            return Err(Error::biz_msg("状态错误"));
        }

        // 整单售后进行中禁止发货（Java `hasInProgressFullOrderAftersales`）
        let total_qty = self.repos.trade.order_admin.order_items_total_quantity(order_id).await?;
        if total_qty > 0 {
            let covered_qty = self
                .repos.trade.order_admin
                .aftersales_covered_quantity(order_id, &aftersales_status::valid_statuses())
                .await?;
            if covered_qty >= total_qty {
                return Err(Error::biz_msg("该订单存在售后，无法发货"));
            }
        }

        if param.deliver_data.as_ref().is_none_or(Vec::is_empty) {
            return Err(Error::Validation("请填写发货数量".into()));
        }

        let items = self.repos.trade.order_admin.find_items(order_id).await?;
        let occupied = self
            .repos.trade.order_admin
            .aftersales_quantity_by_item(order_id, &aftersales_status::valid_statuses())
            .await?;
        let plan = compute_deliver_plan(&items, &param.deliver_quantity_map(), &occupied)?;
        if plan.deliver.is_empty() {
            return Err(Error::Validation("请填写发货数量".into()));
        }
        let now = utils::now_seconds() as i32;
        if !plan.remain.is_empty() {
            // 部分发货：拆分未发货的商品为待发货子订单
            let item_map: HashMap<i32, &OrderItem> = items.iter().map(|it| (it.item_id, it)).collect();
            let mut remain_order = order.clone();
            remain_order.order_id = 0;
            let remain_sn = utils::generate_order_sn();
            remain_order.order_sn = remain_sn.clone();
            remain_order.parent_order_id = Some(order_id);
            remain_order.parent_order_sn = Some(order.order_sn.clone());
            remain_order.order_status = OrderStatus::Confirmed as i32;
            remain_order.shipping_status = ShippingStatus::Pending as i32;
            remain_order.is_store_splited = 1;
            remain_order.is_valid = 1;

            let mut remain_items = Vec::new();
            let mut remain_product_amount = Decimal::ZERO;
            for (item_id, remain_qty) in &plan.remain {
                if let Some(orig_item) = item_map.get(item_id) {
                    let mut r_item = (*orig_item).clone();
                    r_item.item_id = 0;
                    r_item.order_sn = remain_sn.clone();
                    r_item.quantity = *remain_qty;
                    remain_product_amount += r_item.price * Decimal::from(*remain_qty);
                    remain_items.push(r_item);
                }
            }
            remain_order.product_amount = remain_product_amount;
            if order.product_amount > Decimal::ZERO {
                let ratio = (remain_product_amount / order.product_amount).min(Decimal::ONE);
                remain_order.total_amount = (order.total_amount * ratio).round_dp(2);
                remain_order.paid_amount = (order.paid_amount * ratio).round_dp(2);
                remain_order.unpaid_amount = (remain_order.total_amount - remain_order.paid_amount).max(Decimal::ZERO);
            } else {
                remain_order.total_amount = remain_product_amount;
            }

            let remain_order_id = self.repos.trade.order.insert_order(&remain_order, &remain_items).await?;
            for (item_id, deliver_qty) in &plan.deliver {
                let _ = self.repos.trade.order_admin.update_item_quantity(*item_id, *deliver_qty).await;
            }
            let _ = self.repos.trade.order_admin.insert_order_split_log(
                order_id,
                remain_order_id,
                now,
                &format!("部分发货拆单，剩余商品拆入子订单 {remain_order_id}"),
            ).await;
            let _ = self.repos.trade.order_admin.quick_order_log(
                order_id,
                &order.order_sn,
                &format!("部分发货拆单，生成剩余待发货子订单 {remain_sn}"),
            ).await;
        }

        let method = param.shipping_method.unwrap_or(shipping_method::LOGISTICS);
        let (logistics_id, logistics_provider, logistics_name, tracking_no) = match method {
            shipping_method::LOGISTICS => {
                let lid = param
                    .logistics_id
                    .filter(|v| *v > 0)
                    .ok_or_else(|| Error::biz_msg("物流公司不能为空"))?;
                let name = self
                    .repos.trade.order_admin
                    .find_company_name(lid)
                    .await?
                    .unwrap_or_default();
                let tn = param
                    .tracking_no
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| Error::biz_msg("物流快递单号不能为空"))?
                    .to_string();
                // 物流服务商由 Java `logisticsProviderRouter.getActive()`（配置驱动）决定，
                // 该配置未迁移：保留订单既有 provider，避免写入臆测值
                (lid, order.logistics_provider, name, tn)
            }
            shipping_method::MERCHANT => (0, 0, "商家配送".to_string(), String::new()),
            shipping_method::NONE => (0, 0, "无需配送".to_string(), String::new()),
            shipping_method::WAYBILL => {
                let lid = param.logistics_id.unwrap_or(0);
                let name = if lid > 0 {
                    self.repos
                        .trade
                        .order_admin
                        .find_company_name(lid)
                        .await?
                        .unwrap_or_else(|| "电子面单".to_string())
                } else {
                    "电子面单".to_string()
                };
                let mut tn = param
                    .tracking_no
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("EB{}", utils::now_millis()));
                let mut provider = order.logistics_provider;
                // 当前激活快递鸟且配置完整时，真正下 1007 电子面单；
                // 面单失败直接报错，不写入假单号。
                if lid > 0
                    && crate::logistics::logistics_company::LogisticsCompanyService::new(
                        self.repos.clone(),
                    )
                    .resolve_active_provider_type()
                    .await?
                        == nextrade_core::models::logistics::logistics_company::PROVIDER_KDNIAO
                {
                    let waybill = self
                        .create_kdn_waybill(&order, lid, param.bill_remark.as_deref(), &items)
                        .await?;
                    tn = waybill.1.clone();
                    provider = 1;
                    self.repos
                        .trade
                        .order_admin
                        .insert_logistics_api_log(
                            order_id,
                            1,
                            &waybill.0,
                            &waybill.1,
                            waybill.2.as_deref().unwrap_or(""),
                            now,
                        )
                        .await?;
                } else if lid > 0
                    && crate::logistics::logistics_company::LogisticsCompanyService::new(
                        self.repos.clone(),
                    )
                    .resolve_active_provider_type()
                    .await?
                        == nextrade_core::models::logistics::logistics_company::PROVIDER_KUAIDI100
                {
                    // 快递100 电子面单（Java Kuaidi100LogisticsProvider#createElectronicWaybill）
                    let waybill = self
                        .create_kuaidi100_waybill(&order, lid, param.bill_remark.as_deref(), &items)
                        .await?;
                    tn = waybill.1.clone();
                    provider = 2;
                    self.repos
                        .trade
                        .order_admin
                        .insert_logistics_api_log(
                            order_id,
                            2,
                            &waybill.0,
                            &waybill.1,
                            waybill.2.as_deref().unwrap_or(""),
                            now,
                        )
                        .await?;
                }
                (lid, provider, name, tn)
            }
            other => {
                return Err(Error::Validation(format!("无效的配送方式: {other}")));
            }
        };

        // 同城配送不改动发货状态（Java 对 SAME_CITY 不 setShippingStatus）
        let shipping_status = if is_same_city(order.shipping_type_id) {
            order.shipping_status
        } else {
            ShippingStatus::Sent as i32
        };
        let now = utils::now_seconds() as i32;
        let updated = self
            .repos.trade.order_admin
            .update_deliver(
                order_id,
                method,
                shipping_status,
                now,
                logistics_id,
                logistics_provider,
                &logistics_name,
                &tracking_no,
            )
            .await?;
        if !updated {
            return Err(Error::Conflict("订单状态已变更，请刷新后重试".into()));
        }

        // 发货后置动作：赠送积分（发货节点 Node 2）及向用户发送发货站内信通知
        let order_svc = crate::trade::order::OrderService::from_repos(self.repos.clone());
        if let Ok(Some(full_order)) = self.repos.trade.order.find_by_id(order_id).await {
            let _ = order_svc.order_success_point(&full_order, 2).await;
            if full_order.user_id > 0 {
                let _ = self.repos.member.user_message.insert_user_message(
                    full_order.user_id,
                    "订单发货提醒",
                    &format!("您的订单【{}】已发货，快递单号：{}，请注意查收。", full_order.order_sn, tracking_no),
                    1,
                    Some(&format!("/order/detail?id={}", full_order.order_id)),
                ).await;
            }
        }

        tracing::info!(
            "订单发货成功: order_id={order_id}, method={method}, tracking_no={tracking_no}, items={}",
            plan.deliver.len()
        );
        Ok(())
    }

    // ————————————————————— 后台取消订单 —————————————————————

    /// 后台取消订单（对应 Java `OrderController.cancelOrderByAdmin` /
    /// `OrderServiceImpl#cancelOrderByAdmin`）。
    ///
    /// 语义与原 Java 一致：**不直接改订单状态**，而是按整单创建一张
    /// 「仅退款」售后申请（状态=审核处理中），由售后审核流程完成退款/关单。
    ///
    /// 对齐 Java：随后按配置项 `afterSalesReview` 判定，若关闭人工审核（!= "1"），
    /// 则自动审核通过并触发财务退款、回滚库存以及取消订单。
    ///
    /// 返回新建的 `aftersale_id`。
    pub async fn cancel_order_by_admin(
        &self,
        param: &AftersalesApplyAdminParam,
        operator: Option<&str>,
    ) -> Result<i32> {
        let order_id = param
            .order_id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;
        let order = self
            .repos.trade.order_admin
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;

        // Java：订单必须处于「已确认，待发货」，否则「已发货订单不能取消」
        if order.order_status != OrderStatus::Confirmed as i32 {
            return Err(Error::biz_msg("已发货订单不能取消"));
        }
        // Java `afterSalesApply` 的前置校验
        if order.pay_status != PayStatus::Paid as i32 {
            return Err(Error::biz_msg("未支付订单不允许售后"));
        }
        if order.order_source == "pos" {
            return Err(Error::biz_msg("收银台订单无法售后"));
        }

        let items = self.repos.trade.order_admin.find_items(order_id).await?;
        if items.is_empty() {
            return Err(Error::biz_msg("请填写申请售后数量"));
        }
        if items.iter().any(|it| it.gift_card_id.is_some_and(|v| v > 0)) {
            return Err(Error::biz_msg("电子礼品卡不支持申请售后"));
        }

        // 数量校验（Java `validateOrderQuantity`，isAdminCancel=true 文案）
        let occupied = self
            .repos.trade.order_admin
            .aftersales_quantity_by_item(order_id, &aftersales_status::valid_statuses())
            .await?;
        for it in &items {
            if self
                .repos.trade.order_admin
                .has_applied_aftersales(order_id, it.item_id, &aftersales_status::applied_statuses())
                .await?
            {
                return Err(Error::biz_msg("订单已申请售后，无法取消，请先处理售后"));
            }
            let used = occupied.get(&it.item_id).copied().unwrap_or(0);
            // Java `validateOrderQuantity`：申请数量 + 已占用数量 <= 购买数量；
            // 后台取消订单按整单申请（申请数量 = 购买数量），故存在任何有效占用即不足
            if used > 0 {
                return Err(Error::biz_msg("该商品可申请数量不足"));
            }
        }

        let now = utils::now_seconds() as i32;
        let description = param
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("商家取消订单")
            .to_string();
        let pics = param
            .pics
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let aftersales = Aftersales {
            aftersale_id: 0,
            aftersale_type: aftersales_type::PAY_RETURN,
            status: aftersales_status::IN_REVIEW,
            pics: pics.clone(),
            description: description.clone(),
            reply: String::new(),
            add_time: now,
            tracking_no: String::new(),
            logistics_name: String::new(),
            return_address: None,
            aftersale_reason: Some("其他".into()),
            aftersales_sn: Some(aftersales_sn(order_id, now)),
            order_id: Some(order.order_id),
            user_id: Some(order.user_id),
            refund_amount: param.refund_amount.unwrap_or(Decimal::ZERO),
            shop_id: order.shop_id,
            audit_time: 0,
            deal_time: 0,
            final_time: 0,
            return_goods_tip: String::new(),
            // Java：订单绑定供应商时带上供应商ID
            vendor_id: order.vendor_id.filter(|v| *v > 0),
            close_type: 0,
        };
        let af_items: Vec<AftersalesItem> = items
            .iter()
            .map(|it| AftersalesItem {
                aftersales_item_id: 0,
                order_item_id: Some(it.item_id),
                number: Some(it.quantity),
                aftersale_id: None,
            })
            .collect();
        let log = AftersalesLog {
            log_id: 0,
            aftersale_id: 0,
            log_info: format!("会员发起了仅退款: {description}"),
            add_time: now,
            admin_name: String::new(),
            refund_money: Decimal::ZERO,
            refund_type: 0,
            refund_desc: String::new(),
            // Java：`SecurityUtils.getCurrentUsername()`（当前登录管理员）
            user_name: operator.map(ToString::to_string),
            return_pic: (!pics.is_empty()).then_some(pics),
            shop_id: Some(order.shop_id),
            vendor_id: order.vendor_id.filter(|v| *v > 0),
        };

        let aftersale_id = self
            .repos.trade.order_admin
            .insert_aftersales_bundle(&aftersales, &af_items, &log)
            .await?;

        let aftersale_review = self
            .repos
            .system
            .config_basic
            .find_by_code("afterSalesReview")
            .await?
            .unwrap_or_else(|| "1".into());

        // 当后台售后审核关闭（afterSalesReview != "1"），自动审核、退款、回滚库存并取消订单
        if aftersale_review != "1" {
            for it in &items {
                let _ = self
                    .repos
                    .trade
                    .order_admin
                    .restore_stock(it.product_id, it.sku_id, it.quantity)
                    .await;
            }

            let _ = self
                .repos
                .trade
                .aftersales
                .update_aftersales(
                    aftersale_id,
                    Some(aftersales_status::COMPLETE),
                    Some("后台取消订单，系统自动审核通过"),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(now),
                    None,
                    None,
                )
                .await;

            let _ = self
                .repos
                .trade
                .order_admin
                .update_order(
                    order_id,
                    &OrderPatch::new().i32_val("order_status", OrderStatus::Cancelled as i32),
                )
                .await;

            let af_service = crate::trade::aftersales::AftersalesService::new(self.repos.clone());
            let mut created_aftersales = aftersales.clone();
            created_aftersales.aftersale_id = aftersale_id;
            let _ = af_service
                .create_refund_apply_and_handle_finance(
                    &created_aftersales,
                    aftersales.refund_amount,
                    "后台取消订单自动退款",
                )
                .await;
        }

        tracing::info!(
            "后台取消订单处理完成: order_id={order_id}, aftersale_id={aftersale_id}, items={}",
            af_items.len()
        );
        Ok(aftersale_id)
    }

    // ————————————————————— 管理端扩展（wave3：30 个端点） —————————————————————

    /// 各状态订单数量统计（`GET /order/order/statusCount`）。
    ///
    /// 对应 Java `getStatusCount` + `buildQueryWrapper`：复用列表筛选条件
    /// （`orderStatus = -2` 表示已删除订单的虚拟状态），已删除单独统计。
    pub async fn status_count(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
    ) -> Result<OrderStatusCountVO> {
        let mut vo = OrderStatusCountVO::default();
        vo.deleted_count = self
            .repos
            .trade
            .order_admin
            .count_with_status_filter(q, platform_view, Some("-2"))
            .await?;
        for (status, count) in self
            .repos
            .trade
            .order_admin
            .status_counts(q, platform_view)
            .await?
        {
            vo.all_count += count;
            match OrderStatus::of(status) {
                Some(OrderStatus::Pending) => vo.pending_count = count,
                Some(OrderStatus::Confirmed) => vo.confirmed_count = count,
                Some(OrderStatus::Processing) => vo.processing_count = count,
                Some(OrderStatus::Cancelled) => vo.cancelled_count = count,
                Some(OrderStatus::Completed) => vo.completed_count = count,
                _ => {}
            }
        }
        Ok(vo)
    }

    /// 设置商家备注 / 标记（`POST /order/order/setAdminNote`）。
    ///
    /// 对应 Java `setAdminNote`：`ids` 非空时批量更新，否则按 `id`；
    /// 每单写一条「订单商家备注已更新」日志。未传的字段不写库
    /// （MyBatis `NOT_NULL` 语义）。
    pub async fn set_admin_note(
        &self,
        param: &OrderAdminNoteParam,
        admin_id: i32,
    ) -> Result<()> {
        let ids = param.target_ids();
        if ids.is_empty() {
            return Err(Error::Validation("订单ID不能为空".into()));
        }
        let note = param.admin_note.clone();
        if note.is_none() && param.mark.is_none() {
            return Ok(());
        }
        for id in ids {
            let Some(order) = self.repos.trade.order_admin.find_order(id).await? else {
                continue;
            };
            let mut patch = OrderPatch::new();
            if let Some(v) = note.as_deref() {
                patch = patch.str_val("admin_note", v);
            }
            if let Some(mark) = param.mark {
                patch = patch.i32_val("mark", mark);
            }
            self.repos.trade.order_admin.update_order(id, &patch).await?;
            self.repos
                .trade
                .order_admin
                .insert_order_log(id, &order.order_sn, "订单商家备注已更新", admin_id, order.shop_id)
                .await?;
        }
        Ok(())
    }

    /// 父订单详情（`GET /order/order/parentDetail`）。
    ///
    /// 对应 Java `OrderSpitLogServiceImpl#getParentDataByOrderId`：
    /// 优先按拆单日志（`order_split_log`）取父单；父单可读时返回详情并附
    /// `subOrderIds`，否则回退日志里的 `parent_order_data` JSON 快照；
    /// 无拆单日志时按 `order.parent_order_id` 兜底。无父单返回 `null`。
    pub async fn parent_detail(
        &self,
        order_id: i32,
    ) -> Result<Option<ParentOrderDetailVO>> {
        if let Some((parent_id, data)) = self
            .repos
            .trade
            .order_admin
            .find_order_split_log(order_id)
            .await?
        {
            if parent_id > 0 {
                if let Ok(mut vo) = self.detail(parent_id).await {
                    let subs = self
                        .repos
                        .trade
                        .order_admin
                        .find_sub_order_ids(parent_id)
                        .await?;
                    if !subs.is_empty() {
                        vo.sub_order_ids = Some(subs);
                    }
                    return Ok(Some(ParentOrderDetailVO::Order(vo)));
                }
            }
            if let Some(raw) = data.filter(|s| !s.trim().is_empty()) {
                if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw) {
                    let subs = self
                        .repos
                        .trade
                        .order_admin
                        .find_sub_order_ids(parent_id)
                        .await?;
                    if !subs.is_empty() {
                        value["subOrderIds"] = serde_json::json!(subs);
                    }
                    return Ok(Some(ParentOrderDetailVO::Snapshot(value)));
                }
            }
            return Ok(None);
        }

        // 兜底：无拆单日志时通过 parent_order_id 直查主订单（次卡兑换子订单等）
        if let Some(order) = self.repos.trade.order_admin.find_order(order_id).await? {
            if let Some(parent_id) = order.parent_order_id.filter(|v| *v > 0) {
                if let Ok(mut vo) = self.detail(parent_id).await {
                    let subs = self
                        .repos
                        .trade
                        .order_admin
                        .find_sub_order_ids(parent_id)
                        .await?;
                    if !subs.is_empty() {
                        vo.sub_order_ids = Some(subs);
                    }
                    return Ok(Some(ParentOrderDetailVO::Order(vo)));
                }
            }
        }
        Ok(None)
    }

    /// 设置订单确认（`POST /order/order/setConfirm`）。
    ///
    /// ⚠️ 逐字对照 Java `setOrderConfirm`：仅当 `availableActions.confirmReceipt`
    /// 为 true（即「处理中，已发货」）时才把订单置为「已确认，待发货」；
    /// 其余状态返回 false（Java 原实现同样返回 false，接口仍 200）。
    pub async fn set_confirm(&self, id: i32) -> Result<bool> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.confirm_receipt {
            return Ok(false);
        }
        self.repos
            .trade
            .order_admin
            .update_order(
                id,
                &OrderPatch::new().i32_val("order_status", OrderStatus::Confirmed as i32),
            )
            .await?;
        self.repos
            .trade
            .order_admin
            .quick_order_log(id, &order.order_sn, "订单状态已更新")
            .await?;
        Ok(true)
    }

    /// 订单拆分（`POST /order/order/splitStoreOrder`）。
    ///
    /// 对应 Java `splitStoreOrder`：
    /// - 已拆单（`is_store_splited = 1`）→ 直接返回 false；
    /// - 全部明细同店铺同供应商（单组）→ 只置 `is_store_splited = 1` 并返回 true。
    /// - 多店铺/多供应商 → 按 (shop_id, vendor_id) 拆分子订单并置父单 `is_store_splited = 1, is_valid = 0`。
    pub async fn split_store_order(&self, id: i32) -> Result<bool> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        if order.is_store_splited == 1 {
            return Ok(false);
        }
        let items = self.repos.trade.order_admin.find_items(id).await?;
        let mut groups: HashMap<(i32, i32), Vec<OrderItem>> = HashMap::new();
        for it in items {
            groups.entry((it.shop_id, it.vendor_id.unwrap_or(0))).or_default().push(it);
        }
        if groups.len() <= 1 {
            self.repos
                .trade
                .order_admin
                .update_order(id, &OrderPatch::new().i32_val("is_store_splited", 1))
                .await?;
            return Ok(true);
        }

        // 多店铺/多供应商拆单
        let now = utils::now_seconds() as i32;
        for ((shop_id, vendor_id), group_items) in &groups {
            let mut sub_order = order.clone();
            sub_order.order_id = 0;
            let sub_sn = utils::generate_order_sn();
            sub_order.order_sn = sub_sn.clone();
            sub_order.parent_order_id = Some(id);
            sub_order.parent_order_sn = Some(order.order_sn.clone());
            sub_order.shop_id = *shop_id;
            sub_order.vendor_id = if *vendor_id > 0 { Some(*vendor_id) } else { None };
            sub_order.is_store_splited = 1;
            sub_order.is_valid = 1;

            let mut sub_product_amount = Decimal::ZERO;
            for it in group_items {
                sub_product_amount += it.price * Decimal::from(it.quantity);
            }
            sub_order.product_amount = sub_product_amount;
            if order.product_amount > Decimal::ZERO {
                let ratio = (sub_product_amount / order.product_amount).min(Decimal::ONE);
                sub_order.total_amount = (order.total_amount * ratio).round_dp(2);
                sub_order.paid_amount = (order.paid_amount * ratio).round_dp(2);
                sub_order.unpaid_amount = (sub_order.total_amount - sub_order.paid_amount).max(Decimal::ZERO);
            } else {
                sub_order.total_amount = sub_product_amount;
            }

            let sub_id = self.repos.trade.order.insert_order(&sub_order, group_items).await?;
            for it in group_items {
                let _ = self.repos.trade.order_admin.update_item_order_id(it.item_id, sub_id).await;
            }
            let _ = self.repos.trade.order_admin.insert_order_split_log(
                id,
                sub_id,
                now,
                &format!("店铺拆单：shop_id={}, vendor_id={}", shop_id, vendor_id),
            ).await;
        }

        self.repos
            .trade
            .order_admin
            .update_order(
                id,
                &OrderPatch::new()
                    .i32_val("is_store_splited", 1)
                    .i32_val("is_valid", 0),
            )
            .await?;
        self.repos
            .trade
            .order_admin
            .quick_order_log(id, &order.order_sn, "商品属于不同商铺/供应商，已拆分订单")
            .await?;

        Ok(true)
    }

    /// 设置已付款（`POST /order/order/setPaid`）。
    ///
    /// 对应 Java `setOrderPaid` 的非预售分支：
    /// `pay_status = 已支付` + `pay_time = now`；线下支付补 `offline_paid_amount`
    /// 与 `paid_amount`；待支付订单同步转「已确认，待发货」并重算未付金额；
    /// 最后尝试自动拆单（单店铺分支）。返回 false 表示订单仍存在（未拆单）。
    ///
    /// ⚠️ **未迁移**：定金预售（`presale_type = 2`）的定金/尾款状态机、
    /// 积分赠送、电子卡券发放、拼团成团、分销佣金、店铺会员记账。
    pub async fn set_paid(&self, id: i32, description: Option<&str>) -> Result<bool> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        // 定金预售：按「待付定金 → 待付尾款 → 已完成」推进
        if order.presale_type == Some(2) {
            let presale = self
                .repos
                .payment
                .pay
                .find_presale_by_order_id(id)
                .await?
                .ok_or_else(|| Error::biz_msg("预售订单信息不存在"))?;
            let now = utils::now_seconds() as i32;
            match presale.presale_status {
                0 => {
                    // 确认定金收款：订单已付 + 定金、未付改为尾款；预售状态置「待付尾款」
                    self.repos
                        .payment
                        .pay
                        .settle_presale_deposit(id, presale.deposit_amount)
                        .await?;
                    self.repos
                        .payment
                        .pay
                        .update_presale_deposit_paid(id, now, Some(now))
                        .await?;
                    self.repos
                        .trade
                        .order_admin
                        .quick_order_log(id, &order.order_sn, "定金收款")
                        .await?;
                    if let Err(e) = self
                        .repos
                        .payment
                        .paylog
                        .mark_order_paylogs_paid(id)
                        .await
                    {
                        tracing::warn!("定金确认同步 paylog 失败: order={id}, err={e}");
                    }
                    return Ok(false);
                }
                1 => {
                    // 确认尾款收款：订单全额付清 + 预售完成
                    let patch = OrderPatch::new()
                        .i32_val("pay_status", PayStatus::Paid as i32)
                        .i32_val("pay_time", now)
                        .decimal("paid_amount", order.total_amount)
                        .decimal("unpaid_amount", Decimal::ZERO);
                    self.repos
                        .trade
                        .order_admin
                        .update_order(id, &patch)
                        .await?;
                    self.repos
                        .payment
                        .pay
                        .update_presale_final_paid(id, now)
                        .await?;
                    self.repos
                        .trade
                        .order_admin
                        .quick_order_log(id, &order.order_sn, "尾款收款")
                        .await?;
                    if let Err(e) = self
                        .repos
                        .payment
                        .paylog
                        .mark_order_paylogs_paid(id)
                        .await
                    {
                        tracing::warn!("尾款确认同步 paylog 失败: order={id}, err={e}");
                    }
                    return Ok(false);
                }
                _ => return Err(Error::biz_msg("预售订单已完成支付")),
            }
        }
        let now = utils::now_seconds() as i32;
        let offline = order.pay_type_id == pay_type::OFFLINE;
        let paid = if offline {
            order.balance + order.unpaid_amount
        } else {
            order.total_amount
        };

        // 触发订单支付后置流程（积分赠送 Node 1、电子卡券分配、对账单入账流水）
        let order_svc = crate::trade::order::OrderService::from_repos(self.repos.clone());
        if let Err(e) = order_svc.on_payment_success(id, paid, None).await {
            tracing::warn!("后台设置已支付触发后置动作告警: order={id}, err={e}");
        }

        let mut patch = OrderPatch::new()
            .i32_val("pay_status", PayStatus::Paid as i32)
            .i32_val("pay_time", now);
        if order.order_status == OrderStatus::Pending as i32 {
            patch = patch.i32_val("order_status", OrderStatus::Confirmed as i32);
            let paid = if offline {
                order.balance + order.unpaid_amount
            } else {
                order.paid_amount
            };
            patch = patch.decimal("unpaid_amount", (order.total_amount - paid).max(Decimal::ZERO));
        }
        self.repos
            .trade
            .order_admin
            .update_order(id, &patch)
            .await?;
        let desc = description
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("订单支付");
        self.repos
            .trade
            .order_admin
            .quick_order_log(id, &order.order_sn, desc)
            .await?;
        // 线下汇款确认收款：同步把待支付 paylog 置为已支付，保证资金流水一致
        if offline {
            match self
                .repos
                .payment
                .paylog
                .mark_order_paylogs_paid(id)
                .await
            {
                Ok(n) => tracing::info!("后台确认收款：订单 {id} 同步 {n} 条 paylog 为已支付"),
                Err(e) => tracing::warn!("后台确认收款：订单 {id} 同步 paylog 失败: {e}"),
            }
        }
        // Java 末尾无条件调用 splitStoreOrder；多店铺返回明确错误，这里降级为告警
        if let Err(e) = self.split_store_order(id).await {
            tracing::warn!("设置已支付后自动拆单未执行: order_id={id}, err={e}");
        }
        Ok(self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .is_none())
    }

    /// 取消订单（`POST /order/order/cancelOrder`）。
    ///
    /// 对应 Java `cancelOrder`：状态门（`availableActions.cancelOrder`）→
    /// 返还库存 + 扣回虚拟销量 → 置「已取消」+ 取消原因 → 级联取消子单 → 日志。
    ///
    /// 已补：未付款/兑换订单返还抵扣积分、余额原路返还并写余额日志、
    /// 释放订单/父单优惠券占用、写店铺与供应商后台取消消息、
    /// 回补秒杀库存/扣回秒杀销量、返还礼品卡组可用库存、
    /// 拼团记录置已取消并恢复明细可售数量。
    /// 仍未迁移：预售尾款校验。
    pub async fn cancel_order(
        &self,
        id: i32,
        description: Option<&str>,
        admin_id: i32,
    ) -> Result<()> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        // 定金预售：已付定金、待付尾款阶段不允许取消（Java 返回 false）。
        if order.presale_type == Some(2) {
            if let Some(presale) = self
                .repos
                .payment
                .pay
                .find_presale_by_order_id(id)
                .await?
            {
                if presale.presale_status == 1 {
                    return Err(Error::biz_msg("预售订单已支付定金，待付尾款阶段无法取消"));
                }
            }
        }
        let actions = self.order_actions(&order).await?;
        if !actions.cancel_order {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let items = self.repos.trade.order_admin.find_items(id).await?;
        for it in &items {
            self.repos
                .trade
                .order_admin
                .restore_stock(it.product_id, it.sku_id, it.quantity)
                .await?;
            // 秒杀订单取消：回补秒杀库存并扣回秒杀销量。
            if it.is_seckill == Some(1) {
                let seckill = crate::marketing::seckill::SeckillService::new(self.repos.clone());
                let sku_id = (it.sku_id > 0).then_some(it.sku_id);
                let _ = seckill.inc_stock(it.product_id, sku_id, it.quantity).await;
                let _ = seckill.dec_sales(it.product_id, sku_id, it.quantity).await;
            }
            // 礼品卡商品取消：返还卡组可用库存。
            if let Some(product) = self
                .repos
                .catalog
                .product_manage
                .find_product(it.product_id)
                .await?
            {
                if let Some(group_id) = product.gift_card_groups_id.filter(|v| *v > 0) {
                    if let Some(group) = self
                        .repos
                        .marketing
                        .gift_card
                        .find_group(group_id)
                        .await?
                    {
                        let _ = self
                            .repos
                            .marketing
                            .gift_card
                            .update_group_stock(
                                group_id,
                                group.availability_stock.unwrap_or(0) + it.quantity,
                            )
                            .await;
                    }
                }
            }
        }
        // 拼团订单取消：待成团记录置已取消，并恢复明细可售数量。
        if order.order_type == Some(order_type::PIN) {
            let _ = self
                .repos
                .marketing
                .groupon_promotion_record
                .cancel_waiting_by_order(i64::from(order.order_id))
                .await;
            for it in &items {
                if it.is_pin == 1 {
                    if let Some(detail_id) = it
                        .promotion_data
                        .as_deref()
                        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                        .and_then(|v| v.get("grouponDetailId").and_then(|x| x.as_i64()))
                    {
                        let _ = self
                            .repos
                            .marketing
                            .groupon_promotion
                            .inc_detail_limit(detail_id, it.quantity)
                            .await;
                    }
                }
            }
        }
        let remark = description
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("");
        let mut patch = OrderPatch::new().i32_val("order_status", OrderStatus::Cancelled as i32);
        if !remark.is_empty() {
            patch = patch.str_val("cancel_remark", remark);
        }
        self.repos.trade.order_admin.update_order(id, &patch).await?;

        // 取消返还：未付款/兑换订单返还抵扣积分；余额原路返还；优惠券释放占用。
        if order.use_points > 0
            && (order.is_exchange_order == Some(1) || order.pay_status == 0)
        {
            self.repos
                .member
                .user_points_log
                .inc_points(
                    order.use_points,
                    order.user_id,
                    "订单取消返还积分",
                    Some(order.order_id),
                )
                .await?;
        }
        if order.balance > Decimal::ZERO {
            if let Some(user) = self.repos.member.user_manage.find_user(order.user_id).await? {
                let before = user.balance;
                if self
                    .repos
                    .member
                    .user_manage
                    .apply_balance_delta(
                        order.user_id,
                        nextrade_core::models::member::user_manage::CHANGE_TYPE_INC,
                        order.balance,
                    )
                    .await?
                    > 0
                {
                    let _ = self
                        .repos
                        .member
                        .user_manage
                        .insert_balance_log(&nextrade_core::entities::UserBalanceLog {
                            log_id: 0,
                            user_id: order.user_id,
                            before_balance: before,
                            balance: order.balance,
                            after_balance: before + order.balance,
                            frozen_balance: Decimal::ZERO,
                            before_frozen_balance: Decimal::ZERO,
                            after_frozen_balance: Decimal::ZERO,
                            change_time: utils::now_seconds() as i32,
                            change_desc: "订单取消返还余额".to_string(),
                            change_type:
                                nextrade_core::models::member::user_manage::CHANGE_TYPE_INC,
                            wallet_type: 0,
                            new_balance: before + order.balance,
                            new_frozen_balance: Decimal::ZERO,
                        })
                        .await;
                }
            }
        }
        if order.coupon_amount > Decimal::ZERO {
            let released = self.repos.marketing.coupon.release_by_order(id).await?;
            if released == 0 {
                if let Some(parent_id) = order.parent_order_id.filter(|v| *v > 0) {
                    let _ = self
                        .repos
                        .marketing
                        .coupon
                        .release_by_order(parent_id)
                        .await;
                }
            }
        }

        // 后台订单取消消息（Java AdminMsgTypeEnum.ORDER_CANCELED = 31）。
        let title = format!("订单已被取消：{},金额：{}", order.order_sn, order.total_amount);
        let content = format!(
            "订单编号：{}，订单金额：{}，订单取消。",
            order.order_sn, order.total_amount
        );
        let related = serde_json::json!({ "orderId": order.order_id }).to_string();
        let _ = self
            .repos
            .system
            .admin_msg
            .create_message(31, order.shop_id, order.order_id, &title, &content, &related)
            .await;
        if let Some(vendor_id) = order.vendor_id.filter(|v| *v > 0) {
            let _ = self
                .repos
                .system
                .admin_msg
                .create_vendor_message(
                    31,
                    vendor_id,
                    order.shop_id,
                    order.order_id,
                    &title,
                    &content,
                    &related,
                )
                .await;
        }
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "订单已取消", admin_id, order.shop_id)
            .await?;
        // 级联取消子单（Java：is_store_splited=1 且父单取消）
        for child in self.repos.trade.order_admin.find_children(id).await? {
            if child.order_status == OrderStatus::Cancelled as i32 {
                continue;
            }
            self.repos
                .trade
                .order_admin
                .update_order(
                    child.order_id,
                    &OrderPatch::new()
                        .i32_val("order_status", OrderStatus::Cancelled as i32)
                        .str_val("cancel_remark", "父订单取消，级联取消"),
                )
                .await?;
        }
        Ok(())
    }

    /// 删除订单（`POST /order/order/delOrder`，软删除：`is_del = 1` + 已取消）。
    pub async fn del_order(&self, id: i32, admin_id: i32) -> Result<()> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.del_order {
            return Err(Error::biz_msg("订单状态错误"));
        }
        self.repos
            .trade
            .order_admin
            .update_order(
                id,
                &OrderPatch::new()
                    .i32_val("order_status", OrderStatus::Cancelled as i32)
                    .i32_val("is_del", 1),
            )
            .await?;
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "订单删除", admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 修改订单金额（`POST /order/order/modifyMoney`）。
    ///
    /// 对应 Java `modifyOrderMoney`：可换订单号（仅待支付未付款）、改运费/
    /// 发票费/服务费/折扣；折扣按明细小计比例分摊到 `order_item.price`
    /// （`admin_change_pre_price` 记录改价前单价），随后 `updateOrderMoney`
    /// 重算订单总额/未付金额。
    pub async fn modify_money(
        &self,
        param: &OrderModifyMoneyParam,
        admin_id: i32,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.modify_order_money {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let mut patch = OrderPatch::new();
        if param.update_order_sn == Some(1) {
            if order.pay_status != PayStatus::Unpaid as i32
                || order.order_status != OrderStatus::Pending as i32
            {
                return Err(Error::biz_msg("支付状态异常，仅未支付订单可更换订单号"));
            }
            let mut sn = utils::generate_order_sn();
            let mut guard = 0;
            while self
                .repos
                .trade
                .order_admin
                .find_order_by_sn(&sn)
                .await?
                .is_some()
                && guard < 5
            {
                sn = utils::generate_order_sn();
                guard += 1;
            }
            patch = patch.str_val("order_sn", &sn);
        }
        if let Some(v) = param.shipping_fee {
            patch = patch.decimal("shipping_fee", v);
        }
        if let Some(v) = param.invoice_fee {
            patch = patch.decimal("invoice_fee", v);
        }
        if let Some(v) = param.service_fee {
            patch = patch.decimal("service_fee", v);
        }
        if let Some(discount) = param.discount_amount {
            let items = self.repos.trade.order_admin.find_items(id).await?;
            let updates = allocate_discount(&items, discount)?;
            self.repos
                .trade
                .order_admin
                .update_item_prices(&updates)
                .await?;
            self.reallocate_amount_detail_shop_discount(id, &items, discount)
                .await?;
            patch = patch.decimal("discount_amount", discount);
        }
        self.repos.trade.order_admin.update_order(id, &patch).await?;
        self.recompute_order_money(id).await?;
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "订单金额修改", admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 按店铺重算 `order_amount_detail.discount_amount`（Java `modifyOrderMoney`）。
    async fn reallocate_amount_detail_shop_discount(
        &self,
        order_id: i32,
        items: &[OrderItem],
        discount: Decimal,
    ) -> Result<()> {
        let details = self
            .repos
            .trade
            .order_admin
            .find_amount_details(order_id)
            .await?;
        if details.is_empty() {
            return Ok(());
        }
        let mut shop_subtotals: HashMap<i32, Decimal> = HashMap::new();
        let mut total = Decimal::ZERO;
        for it in items {
            let unit = it.admin_change_pre_price.unwrap_or(it.price);
            let subtotal = unit * Decimal::from(it.quantity);
            *shop_subtotals.entry(it.shop_id).or_insert(Decimal::ZERO) += subtotal;
            total += subtotal;
        }
        if total <= Decimal::ZERO {
            return Ok(());
        }
        let mut allocated = Decimal::ZERO;
        for (index, detail) in details.iter().enumerate() {
            let part = if index + 1 < details.len() {
                let shop_subtotal = shop_subtotals
                    .get(&detail.shop_id.unwrap_or(0))
                    .copied()
                    .unwrap_or(Decimal::ZERO);
                let v = (discount * shop_subtotal / total)
                    .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
                allocated += v;
                v
            } else {
                discount - allocated
            };
            self.repos
                .trade
                .order_admin
                .update_amount_detail_discount(detail.order_discount_detail_id, part)
                .await?;
        }
        Ok(())
    }

    /// 修改收货人信息（`POST /order/order/modifyConsignee`）。
    pub async fn modify_consignee(
        &self,
        param: &OrderModifyConsigneeParam,
        admin_id: i32,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.modify_order_consignee {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let mut patch = OrderPatch::new();
        if let Some(v) = param.consignee.as_deref() {
            patch = patch.str_val("consignee", v);
        }
        if let Some(v) = param.mobile.as_deref() {
            patch = patch.str_val("mobile", v);
        }
        if let Some(v) = param.email.as_deref() {
            patch = patch.str_val("email", v);
        }
        if let Some(v) = param.address.as_deref() {
            patch = patch.str_val("address", v);
        }
        let region_ids = param.region_ids.clone().unwrap_or_default();
        let region_names = if region_ids.is_empty() {
            Vec::new()
        } else {
            self.repos
                .trade
                .order_admin
                .find_region_names(&region_ids)
                .await?
        };
        if !region_ids.is_empty() {
            let ids_json =
                serde_json::to_string(&region_ids).unwrap_or_else(|_| "[]".to_string());
            let names_json =
                serde_json::to_string(&region_names).unwrap_or_else(|_| "[]".to_string());
            patch = patch
                .str_val("region_ids", &ids_json)
                .str_val("region_names", &names_json);
        }
        // address_data：保留原快照中的地址标签/用户/默认标记，覆盖展示字段
        let address_data = merge_address_data(
            order.address_data.as_deref(),
            param,
            &region_ids,
            &region_names,
        );
        if let Some(raw) = address_data {
            patch = patch.str_val("address_data", &raw);
        }
        self.repos.trade.order_admin.update_order(id, &patch).await?;
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "修改收货人信息", admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 确认收货（`POST /order/order/confirmReceipt`）。
    ///
    /// 对应 Java `confirmReceipt`：状态门（`availableActions.confirmReceipt`）→
    /// `order_status = 已完成`、`shipping_status = 已发货`、`received_time = now`；
    /// 预约上门订单写「完成上门服务」日志，其余写「确认收货」。
    ///
    /// 收尾统一走 [`crate::trade::settlement::SettlementService::after_order_completed`]
    /// （2026-10-25 起）：积分/成长值/次卡/出账单/分销结算/店铺结算/分销员升级。
    pub async fn confirm_receipt(&self, id: i32, admin_id: i32) -> Result<()> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.confirm_receipt {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let now = utils::now_seconds() as i32;
        self.repos
            .trade
            .order_admin
            .update_order(
                id,
                &OrderPatch::new()
                    .i32_val("order_status", OrderStatus::Completed as i32)
                    .i32_val("shipping_status", ShippingStatus::Sent as i32)
                    .i32_val("received_time", now),
            )
            .await?;
        let desc = if order.order_type == Some(order_type::RESERVATION_HOME) {
            "完成上门服务"
        } else {
            "确认收货"
        };
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, desc, admin_id, order.shop_id)
            .await?;

        // 订单完成后台消息（Java AdminMsgTypeEnum.ORDER_COMPLETED = 13）。
        let title = format!(
            "订单已完成通知：{},金额：{}",
            order.order_sn, order.total_amount
        );
        let content = format!("您有订单【{}】已完成，请注意查看。", order.order_sn);
        let related = serde_json::json!({ "orderId": order.order_id }).to_string();
        let _ = self
            .repos
            .system
            .admin_msg
            .create_message(13, order.shop_id, order.order_id, &title, &content, &related)
            .await;
        if let Some(vendor_id) = order.vendor_id.filter(|v| *v > 0) {
            let _ = self
                .repos
                .system
                .admin_msg
                .create_vendor_message(
                    13,
                    vendor_id,
                    order.shop_id,
                    order.order_id,
                    &title,
                    &content,
                    &related,
                )
                .await;
        }

        // 统一收尾：积分（节点 3）/成长值/次卡/出账单/结算级联（与前台同款）
        if let Ok(Some(full_order)) = self.repos.trade.order.find_by_id(id).await {
            crate::trade::settlement::SettlementService::new(self.repos.clone())
                .after_order_completed(&full_order)
                .await;
        }
        Ok(())
    }

    /// 门店核销（`POST /order/order/writeOff?pickupSn=`）。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/order/order.ts#writeOffOrder`（扫码核销，门店自提单）。
    /// 旧版 Java `OrderController` 未含该端点（o2o 门店核销链路未迁移），
    /// 按**最小可用**实现：
    /// 1. `pickupSn` 按**订单号**匹配订单（前端扫码/手输核销码即订单号）；
    /// 2. 仅「已确认，待发货」/「处理中，已发货」且未删除的订单可核销，
    ///    核销 = 置为已完成（`order_status = Completed`）并记录收货时间；
    /// 3. 写订单日志「核销」（不涉及核销员/门店归属校验）；
    /// 4. 返回核销后的订单详情 VO（前端把返回值 unshift 进核销列表）。
    ///
    /// # Errors
    /// `pickupSn` 为空 → `Validation`；订单不存在 → 业务错误
    /// `订单不存在`；状态不允许 → 业务错误 `订单状态错误`。
    pub async fn write_off(&self, pickup_sn: &str, admin_id: i32) -> Result<OrderVO> {
        let pickup_sn = pickup_sn.trim();
        if pickup_sn.is_empty() {
            return Err(Error::Validation("核销码不能为空".into()));
        }
        let order = self
            .repos
            .trade
            .order_admin
            .find_order_by_sn(pickup_sn)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        if order.is_del == 1 {
            return Err(Error::biz_msg("订单不存在"));
        }
        let can_write_off = matches!(
            OrderStatus::of(order.order_status),
            Some(OrderStatus::Confirmed) | Some(OrderStatus::Processing)
        );
        if !can_write_off {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let now = utils::now_seconds() as i32;
        self.repos
            .trade
            .order_admin
            .update_order(
                order.order_id,
                &OrderPatch::new()
                    .i32_val("order_status", OrderStatus::Completed as i32)
                    .i32_val("shipping_status", ShippingStatus::Sent as i32)
                    .i32_val("received_time", now),
            )
            .await?;
        self.repos
            .trade
            .order_admin
            .insert_order_log(order.order_id, &order.order_sn, "核销", admin_id, order.shop_id)
            .await?;

        // 核销即完成：走统一收尾（积分/成长值/次卡/出账单/结算级联），
        // 否则自提核销单永远不触发店铺结算与分销佣金。
        if let Ok(Some(full_order)) = self.repos.trade.order.find_by_id(order.order_id).await {
            crate::trade::settlement::SettlementService::new(self.repos.clone())
                .after_order_completed(&full_order)
                .await;
        }
        self.detail(order.order_id).await
    }

    /// 修改配送信息（`POST /order/order/modifyShipping`）。
    ///
    /// 对应 Java `modifyOrderShipping`：`shipping_method` 1 物流（物流公司 + 单号）、
    /// 2 商家配送、3 无需配送、4 电子面单（物流公司，单号留空）。
    /// Java 末尾固定 `return false`（接口 void），本实现同样只落库 + 日志。
    pub async fn modify_shipping(
        &self,
        param: &OrderModifyShippingParam,
        admin_id: i32,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.modify_shipping_info {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let mut patch = OrderPatch::new();
        if let Some(method) = param.shipping_method {
            patch = patch.i32_val("shipping_method", method);
            match method {
                shipping_method::LOGISTICS | shipping_method::WAYBILL => {
                    let lid = param.logistics_id.unwrap_or(0);
                    let name = self
                        .repos
                        .trade
                        .order_admin
                        .find_company_name(lid)
                        .await?
                        .unwrap_or_default();
                    patch = patch
                        .i32_val("logistics_id", lid)
                        .str_val("logistics_name", &name);
                    let tracking = if method == shipping_method::LOGISTICS {
                        param.tracking_no.clone().unwrap_or_default()
                    } else {
                        String::new()
                    };
                    patch = patch.str_val("tracking_no", &tracking);
                }
                shipping_method::MERCHANT => {
                    patch = patch
                        .i32_val("logistics_provider", 0)
                        .i32_val("logistics_id", 0)
                        .str_val("tracking_no", "")
                        .str_val("logistics_name", "商家配送");
                }
                shipping_method::NONE => {
                    patch = patch
                        .i32_val("logistics_provider", 0)
                        .i32_val("logistics_id", 0)
                        .str_val("tracking_no", "")
                        .str_val("logistics_name", "无需配送");
                }
                _ => {}
            }
        }
        self.repos.trade.order_admin.update_order(id, &patch).await?;
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "修改配送信息", admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 修改商品信息（`POST /order/order/modifyProduct`）。
    ///
    /// 对应 Java `modifyOrderProduct`：
    /// 1. 状态门 `availableActions.modifyOrderProduct`；
    /// 2. 商品单价合计 - 折扣 - 优惠券必须 > 0；
    /// 3. 以请求明细整批替换订单明细（Java `orderItemService.modifyOrderItem`，
    ///    这里简化为「删除原明细 + 整批插入」，**item_id 会变化**，见偏差说明）；
    /// 4. 重算 `product_amount` 与订单金额，写日志。
    pub async fn modify_product(
        &self,
        param: &OrderModifyProductParam,
        admin_id: i32,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let actions = self.order_actions(&order).await?;
        if !actions.modify_order_product {
            return Err(Error::biz_msg("当前订单状态不可执行此操作"));
        }
        let items = param
            .items
            .as_ref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::Validation("请填写商品信息".into()))?;
        let unit_sum: Decimal = items.iter().map(|it| it.price).sum();
        if unit_sum - order.discount_amount - order.coupon_amount <= Decimal::ZERO {
            return Err(Error::biz_msg(
                "商品价格总和减去优惠价格（优惠券抵扣，优惠折扣）必须大于0",
            ));
        }
        self.repos.trade.order_admin.delete_items(id).await?;
        for it in items {
            let entity = to_order_item(id, &order, it);
            self.repos.trade.order_admin.insert_order_item(&entity).await?;
        }
        self.recompute_order_money(id).await?;
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "订单商品信息修改", admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 添加商品时获取商品信息（`POST /order/order/getAddProductInfo`）。
    pub async fn add_product_info(&self, ids: &[i32]) -> Result<Vec<ProductAddInfoVO>> {
        if ids.is_empty() {
            return Err(Error::Validation("请选择商品".into()));
        }
        let products = self.repos.trade.order_admin.find_products(ids).await?;
        Ok(products.iter().map(to_product_add_info).collect())
    }

    /// 打印订单信息（`GET /order/order/orderPrint`）。
    ///
    /// 返回订单详情并补全 `host`（站点根地址）与 `printTime`
    /// （Java 从 `HttpServletRequest` 取 scheme/host/port 与 `DateUtil.date()`）。
    pub async fn order_print(&self, id: i32, host: &str) -> Result<OrderVO> {
        let mut vo = self.detail(id).await?;
        vo.host = Some(host.to_string());
        vo.print_time = Some(fmt_time(utils::now_seconds() as i32));
        Ok(vo)
    }

    /// 打印电子面单（`GET /order/order/orderPrintBill`）。
    ///
    /// 对应 Java `getOrderPrintWaybillInfo` → `logisticsApiLogService`；
    /// 无面单日志返回 `null`。
    pub async fn order_print_bill(&self, id: i32) -> Result<Option<LogisticsApiLogVO>> {
        Ok(self
            .repos
            .trade
            .order_admin
            .find_logistics_api_log(id)
            .await?
            .map(|log| LogisticsApiLogVO {
                id: log.id,
                order_id: log.order_id,
                provider_type: log.provider_type,
                order_code: log.order_code,
                logistic_code: log.logistic_code,
                add_time: log.add_time,
                print_template: log.print_template,
            }))
    }

    /// 批量详情（`GET /order/order/severalDetail?ids=1,2`）。
    pub async fn several_detail(&self, ids: &[i32]) -> Result<Vec<OrderVO>> {
        if ids.is_empty() {
            return Err(Error::biz_msg("#ids 错误"));
        }
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push(self.detail(*id).await?);
        }
        if out.is_empty() {
            return Err(Error::biz_msg("请选择有效订单"));
        }
        Ok(out)
    }

    /// 创建快递鸟 1007 电子面单。
    ///
    /// 返回 `(OrderCode, LogisticCode, PrintTemplate)`；失败直接返回业务错误。
    async fn create_kdn_waybill(
        &self,
        order: &nextrade_core::entities::Order,
        logistics_id: i32,
        remark: Option<&str>,
        items: &[nextrade_core::entities::OrderItem],
    ) -> Result<(String, String, Option<String>)> {
        let company = self
            .repos
            .logistics
            .logistics_company
            .find_by_id(logistics_id)
            .await?
            .ok_or_else(|| Error::biz_msg("物流公司不存在"))?;
        let exp_type = company
            .exp_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                Error::biz_msg(format!(
                    "物流公司[{}]未配置快递类型(ExpType)，请在后台物流公司管理中完善后再发货",
                    company.logistics_name
                ))
            })?
            .to_string();
        let client = crate::logistics::kdniao::KdniaoClient::from_repos(&self.repos).await?;
        let cfg = self
            .repos
            .system
            .config_basic
            .find_by_codes(&[
                "sender",
                "mobile",
                "provinceName",
                "cityName",
                "areaName",
                "address",
            ])
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let get = |key: &str| cfg.get(key).cloned().unwrap_or_default();
        let region_names: Vec<String> =
            serde_json::from_str(&order.region_names).unwrap_or_default();
        let receiver = crate::logistics::kdniao::KdniaoAddress {
            name: order.consignee.clone(),
            mobile: order.mobile.clone(),
            province_name: region_names.first().cloned().unwrap_or_default(),
            city_name: region_names.get(1).cloned().unwrap_or_default(),
            exp_area_name: region_names.get(2).cloned().unwrap_or_default(),
            address: order.address.clone(),
            post_code: None,
        };
        let sender = crate::logistics::kdniao::KdniaoAddress {
            name: get("sender"),
            mobile: get("mobile"),
            province_name: get("provinceName"),
            city_name: get("cityName"),
            exp_area_name: get("areaName"),
            address: get("address"),
            post_code: Some("000000".to_string()),
        };
        let now = chrono::Local::now();
        let request = crate::logistics::kdniao::KdniaoWaybillRequest {
            order_code: format!(
                "{}{}{}",
                order.order_sn,
                order.order_id,
                100 + (utils::now_millis() % 900)
            ),
            shipper_code: company.logistics_code.clone(),
            pay_type: 3,
            exp_type,
            receiver,
            sender,
            quantity: 1,
            weight: 1.0,
            volume: 1.0,
            remark: remark.map(ToString::to_string),
            commodity: vec![serde_json::json!({ "GoodsName": "电商产品" })],
            customer_name: company.customer_name.clone(),
            customer_pwd: company.customer_pwd.clone(),
            send_site: company.send_site.clone(),
            send_staff: company.send_staff.clone(),
            month_code: company.month_code.clone(),
            is_notice: 0,
            start_date: (now + chrono::Duration::hours(1))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            end_date: (now + chrono::Duration::hours(3))
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            currency_code: "CNY".to_string(),
            is_return_print_template: 1,
        };
        let result = client.create_waybill(&request).await?;
        let order_code = result.order_code.unwrap_or(request.order_code);
        let logistic_code = result
            .logistic_code
            .ok_or_else(|| Error::biz_msg("快递鸟电子面单未返回运单号"))?;
        tracing::info!(
            "快递鸟电子面单创建成功: order_id={}, logistics={}, tracking_no={}, items={}",
            order.order_id,
            company.logistics_name,
            logistic_code,
            items.len()
        );
        Ok((order_code, logistic_code, result.print_template))
    }

    /// 快递100 电子面单（Java `Kuaidi100LogisticsProvider#createElectronicWaybill`）。
    ///
    /// 校验逐条对齐：凭证三件套 / CLOUD 需 siid / IMAGE|HTML|CLOUD 需 tempId /
    /// 编码解析（`kuaidi100_code` 优先，内置映射兜底）。
    /// 返回 `(order_code, 运单号, 面单模板)`。
    async fn create_kuaidi100_waybill(
        &self,
        order: &nextrade_core::entities::Order,
        logistics_id: i32,
        remark: Option<&str>,
        items: &[nextrade_core::entities::OrderItem],
    ) -> Result<(String, String, Option<String>)> {
        let company = self
            .repos
            .logistics
            .logistics_company
            .find_by_id(logistics_id)
            .await?
            .ok_or_else(|| Error::biz_msg("未存在物流公司信息"))?;
        let client = crate::logistics::kuaidi100::Kuaidi100Client::from_repos(&self.repos).await?;
        if !client.is_label_configured() {
            return Err(Error::biz_msg(
                "快递100凭证未配置完整（customer/key/secret），请先在配送设置中填写",
            ));
        }
        let print_type = client.print_type();
        if print_type.eq_ignore_ascii_case("CLOUD") && client.siid().trim().is_empty() {
            return Err(Error::biz_msg("快递100云打印需要配置云打印机设备码(siid)"));
        }
        let need_template = print_type.eq_ignore_ascii_case("IMAGE")
            || print_type.eq_ignore_ascii_case("HTML")
            || print_type.eq_ignore_ascii_case("CLOUD");
        if need_template && company.kuaidi100_temp_id.trim().is_empty() {
            return Err(Error::biz_msg(format!(
                "物流公司[{}]未配置快递100面单模板ID(tempId)，打印类型[{print_type}]无法生成面单，请在物流公司管理中补充",
                company.logistics_name
            )));
        }
        let com = nextrade_core::models::logistics::logistics_company::resolve_kuaidi100_code(
            Some(&company.logistics_code),
        )
        .or_else(|| {
            let s = company.kuaidi100_code.trim();
            (!s.is_empty()).then(|| s.to_string())
        })
        .ok_or_else(|| {
            Error::biz_msg(format!(
                "物流公司[{}]未配置快递100编码(kuaidi100Code)，且内置映射无匹配物流编码[{}]，请在后台物流公司管理中补充",
                company.logistics_name, company.logistics_code
            ))
        })?;

        let cfg = self
            .repos
            .system
            .config_basic
            .find_by_codes(&[
                "sender",
                "mobile",
                "provinceName",
                "cityName",
                "areaName",
                "address",
            ])
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let get = |key: &str| cfg.get(key).cloned().unwrap_or_default();
        let region_names: Vec<String> =
            serde_json::from_str(&order.region_names).unwrap_or_default();
        let recv_addr = format!(
            "{}{}{}{}",
            region_names.first().cloned().unwrap_or_default(),
            region_names.get(1).cloned().unwrap_or_default(),
            region_names.get(2).cloned().unwrap_or_default(),
            order.address
        );
        let send_addr = format!("{}{}{}{}", get("provinceName"), get("cityName"), get("areaName"), get("address"));
        // Java 汇总 OrderItemVO.productWeight（商品联表值）；Rust OrderItem
        // 不携带该字段，取 1.0 兜底（与快递鸟路径同口径，已记 uncertain_points）。
        let weight_sum = 0.0_f64;
        let order_code = format!(
            "{}{}{}",
            order.order_sn,
            order.order_id,
            100 + (utils::now_millis() % 900)
        );
        let request = crate::logistics::kuaidi100::Kuaidi100WaybillRequest {
            kuaidicom: com,
            print_type: print_type.to_string(),
            order_code: order_code.clone(),
            rec_man: crate::logistics::kuaidi100::Kuaidi100Man {
                name: order.consignee.clone(),
                mobile: order.mobile.clone(),
                print_addr: recv_addr,
            },
            send_man: crate::logistics::kuaidi100::Kuaidi100Man {
                name: get("sender"),
                mobile: get("mobile"),
                print_addr: send_addr,
            },
            weight: if weight_sum > 0.0 { weight_sum } else { 1.0 },
            remark: remark.unwrap_or_default().to_string(),
            partner_id: Some(company.kuaidi100_partner_id.clone()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            partner_key: Some(company.kuaidi100_partner_key.clone()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            partner_secret: Some(company.kuaidi100_partner_secret.clone()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            net: Some(company.kuaidi100_net.clone()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            check_man: (!company.kuaidi100_check_man.trim().is_empty())
                .then(|| company.kuaidi100_check_man.trim().to_string()),
            exp_type: (!company.kuaidi100_exp_type.trim().is_empty())
                .then(|| company.kuaidi100_exp_type.trim().to_string()),
            temp_id: Some(company.kuaidi100_temp_id.clone()).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            child_temp_id: (!company.kuaidi100_child_temp_id.trim().is_empty())
                .then(|| company.kuaidi100_child_temp_id.trim().to_string()),
            back_temp_id: (!company.kuaidi100_back_temp_id.trim().is_empty())
                .then(|| company.kuaidi100_back_temp_id.trim().to_string()),
        };
        let result = client.create_waybill(&request).await?;
        tracing::info!(
            "快递100电子面单创建成功: order_id={}, logistics={}, tracking_no={}, items={}",
            order.order_id,
            company.logistics_name,
            result.logistic_code,
            items.len()
        );
        Ok((
            result.order_code,
            result.logistic_code,
            Some(result.print_template).filter(|l| !l.is_empty()),
        ))
    }

    /// 批量操作（`POST /order/order/batch`）：仅支持 `type = "deliver"`。
    pub async fn batch_operation(
        &self,
        param: &OrderBatchParam,
        admin_id: i32,
    ) -> Result<()> {
        if param.op_type.as_deref() != Some("deliver") {
            return Err(Error::biz_msg("#type 错误"));
        }
        let data = param
            .data
            .as_ref()
            .ok_or_else(|| Error::biz_msg("请选择发货信息"))?;
        let ids = param.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Err(Error::Validation("未选择订单".into()));
        }
        for id in ids {
            let entry = data
                .deliver_data
                .as_ref()
                .and_then(|list| list.iter().find(|d| d.id == Some(*id)));
            let deliver_param = DeliverOrderParam {
                id: Some(*id),
                deliver_data: entry.and_then(|e| e.deliver_info.clone()),
                shipping_method: data.shipping_method,
                logistics_id: data.logistics_id,
                tracking_no: entry.and_then(|e| e.tracking_no.clone()),
                ..Default::default()
            };
            if let Err(e) = self.deliver(&deliver_param).await {
                return Err(Error::biz_msg(e.to_string()));
            }
        }
        let _ = admin_id;
        Ok(())
    }

    /// 批量打印（`GET /order/order/printSeveral?ids=1,2`）。
    ///
    /// 与 Java `printSeveral` 一致：逐单 `detail` 并补全 `host` / `printTime`。
    pub async fn print_several(&self, ids: &[i32], host: &str) -> Result<Vec<OrderVO>> {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push(self.order_print(*id, host).await?);
        }
        Ok(out)
    }

    // ————————————————————— 导出列 —————————————————————

    /// 订单导出可选列（`GET /order/order/getExportItemList`）。
    ///
    /// 与 Java `getExportItemList()` 的 Map 顺序/文案一致（O2O 场景店铺名称
    /// 文案差异未迁移，统一「店铺名称」）。
    #[must_use]
    pub fn export_item_list(&self) -> Vec<OrderExportItemVO> {
        export_columns()
    }

    /// 当前管理员已选导出列（`GET /order/order/exportItemInfo`）。
    pub async fn export_item_info(&self, admin_id: i32) -> Result<Vec<OrderExportItemVO>> {
        let raw = self
            .repos
            .trade
            .order_admin
            .find_admin_order_export(admin_id)
            .await?
            .unwrap_or_default();
        let selected: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        if selected.is_empty() {
            return Ok(Vec::new());
        }
        Ok(export_columns()
            .into_iter()
            .filter(|c| selected.iter().any(|k| k == &c.key))
            .collect())
    }

    /// 保存导出列（`POST /order/order/saveExportItem`）。
    pub async fn save_export_item(
        &self,
        param: &ExportItemSaveParam,
        admin_id: i32,
    ) -> Result<()> {
        let raw = param
            .export_item
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请选择导出项".into()))?;
        let items: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let json = serde_json::to_string(&items)
            .map_err(|e| Error::Internal(format!("导出列序列化失败: {e}")))?;
        self.repos
            .trade
            .order_admin
            .update_admin_order_export(admin_id, &json)
            .await?;
        Ok(())
    }

    /// 订单导出为 xlsx（Java `orderExport` 直接回写 xlsx 二进制）。
    ///
    /// 复用 [`Self::order_export`] 的列表读取/富化逻辑，按 `columns` 顺序
    /// 生成与 Java EasyExcel 等价的工作表。
    pub async fn order_export_xlsx(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
    ) -> Result<(String, Vec<u8>)> {
        let vo = self.order_export(q, platform_view).await?;
        let headers: Vec<&str> = vo.columns.iter().map(|c| c.label.as_str()).collect();
        let rows: Vec<Vec<String>> = vo
            .rows
            .iter()
            .map(|row| {
                vo.columns
                    .iter()
                    .map(|col| export_cell(&col.key, row))
                    .collect()
            })
            .collect();
        let bytes = crate::system::excel::build_xlsx(&headers, &rows, &vo.sheet_name)
            .map_err(|e| Error::Internal(format!("订单导出 Excel 生成失败: {e}")))?;
        Ok((vo.file_name, bytes))
    }

    /// 订单导出（`GET /order/order/orderExport`）。
    ///
    /// 对应 Java `orderExport`：按 2000 条/批流式读取列表并富化，
    /// 一行 = 一个订单的一个商品；选中 `productInfo` 时展开为商品字段。
    ///
    /// ⚠️ **降级**：Java 直接回写 xlsx 二进制（EasyExcel）；Rust 侧未启用
    /// xlsx 写出依赖，返回「列定义 + 行数据」JSON，前端/运维可自行落地。
    pub async fn order_export(
        &self,
        q: &OrderListQuery,
        platform_view: bool,
    ) -> Result<OrderExportVO> {
        let mut query = q.clone();
        if let Some(ref tag_ids) = query.member_tag_ids {
            if !tag_ids.is_empty() {
                let match_all = query.member_tag_match_mode == Some(1);
                let uids = self
                    .repos
                    .marketing
                    .coupon_manage
                    .resolve_user_ids_by_tags(tag_ids, match_all)
                    .await?;
                query.tag_user_ids = Some(uids);
            }
        }
        let selected = query.export_items();
        if selected.is_empty() {
            return Err(Error::Validation("请选择导出项".into()));
        }
        let columns = expand_export_columns(&selected)?;
        let mut rows: Vec<OrderExportRowVO> = Vec::new();
        let batch = 2000u64;
        let mut page = 1u64;
        loop {
            let (_, orders) = self
                .repos
                .trade
                .order_admin
                .paginate_orders_with(&query, platform_view, page, batch)
                .await?;
            if orders.is_empty() {
                break;
            }
            let fetched = orders.len() as u64;
            let order_ids: Vec<i32> = orders.iter().map(|o| o.order_id).collect();
            let items = self
                .repos
                .trade
                .order_admin
                .find_items_by_order_ids(&order_ids)
                .await?;
            let mut items_by_order: HashMap<i32, Vec<OrderItem>> = HashMap::new();
            for it in items {
                items_by_order.entry(it.order_id).or_default().push(it);
            }
            let user_ids = distinct_i32(orders.iter().map(|o| o.user_id));
            let users = self.repos.trade.order_admin.find_users(&user_ids).await?;
            let user_map: HashMap<i32, User> = users.into_iter().map(|u| (u.user_id, u)).collect();
            let shop_ids = distinct_i32(orders.iter().map(|o| o.shop_id).filter(|v| *v > 0));
            let shops = self.repos.trade.order_admin.find_shops(&shop_ids).await?;
            let shop_map: HashMap<i32, Shop> = shops.into_iter().map(|s| (s.shop_id, s)).collect();
            let pay_codes: HashMap<i32, String> = self
                .repos
                .trade
                .order_admin
                .find_pay_codes(&order_ids)
                .await?
                .into_iter()
                .collect();
            let product_ids = distinct_i32(
                items_by_order
                    .values()
                    .flat_map(|list| list.iter().map(|it| it.product_id)),
            );
            let products = self.repos.trade.order_admin.find_products(&product_ids).await?;
            let weight_map: HashMap<i32, Decimal> =
                products.iter().map(|p| (p.product_id, p.product_weight)).collect();

            let now = utils::now_seconds() as i32;
            for order in &orders {
                let empty: Vec<OrderItem> = Vec::new();
                let order_items = items_by_order.get(&order.order_id).unwrap_or(&empty);
                let vo = build_vo(VoInput {
                    order,
                    items: order_items,
                    user: user_map.get(&order.user_id),
                    shop: shop_map.get(&order.shop_id),
                    full_aftersales: false,
                    rebuy_available: true,
                    groupon: None,
                    way_bill: false,
                    now,
                });
                for item in order_items {
                    rows.push(OrderExportRowVO {
                        order_sn: vo.order_sn.clone(),
                        add_time: vo.add_time.clone(),
                        order_status_name: vo.order_status_name.clone(),
                        pay_status_name: vo.pay_status_name.clone(),
                        shop_name: vo.shop_name.clone().unwrap_or_default(),
                        username: user_map
                            .get(&order.user_id)
                            .map(|u| u.username.clone())
                            .unwrap_or_default(),
                        consignee: vo.consignee.clone(),
                        mobile: vo.mobile.clone(),
                        address: vo.user_address.clone().unwrap_or_default(),
                        postcode: vo
                            .address_data
                            .as_ref()
                            .and_then(|v| v.get("postcode"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        total_amount: vo.total_amount,
                        balance: vo.balance,
                        discount_amount: vo.discount_amount,
                        points_amount: vo.points_amount,
                        coupon_amount: vo.coupon_amount,
                        pay_time: vo.pay_time.clone(),
                        pay_type_id: vo.pay_type_id,
                        pay_type_name: pay_type_name(vo.pay_type_id),
                        pay_code: pay_codes.get(&order.order_id).cloned().unwrap_or_else(|| "余额支付".to_string()),
                        out_trade_no: vo.out_trade_no.clone(),
                        paid_amount: vo.paid_amount,
                        shipping_time: vo.shipping_time.clone(),
                        shipping_fee: vo.shipping_fee,
                        logistics_name: vo.logistics_name.clone(),
                        tracking_no: vo.tracking_no.clone(),
                        buyer_note: vo.buyer_note.clone(),
                        admin_note: vo.admin_note.clone(),
                        shipping_status_name: vo.shipping_status_name.clone(),
                        product_weight: weight_map
                            .get(&item.product_id)
                            .copied()
                            .unwrap_or(Decimal::ZERO),
                        use_points: vo.use_points,
                        product_name: item.product_name.clone(),
                        sku_data: item.sku_data.clone(),
                        product_sn: item.product_sn.clone(),
                        quantity: item.quantity,
                        total_price: item.price * Decimal::from(item.quantity),
                        price: item.price,
                    });
                }
            }
            if fetched < batch {
                break;
            }
            page += 1;
        }
        Ok(OrderExportVO {
            file_name: format!("订单列表信息{}.xlsx", fmt_time(utils::now_seconds() as i32)),
            sheet_name: "订单列表信息".to_string(),
            columns,
            rows,
        })
    }

    /// 物流信息（`GET /order/order/shippingInfo`）。
    ///
    /// 对应 Java `getShippingInfo`：无运单号 / 物流公司不存在返回 `null`。
    ///
    /// 当前激活服务商为快递鸟且配置完整时，实时调用 1002 查询轨迹；
    /// 快递鸟失败/未配置或激活快递100 时回退为订单运单快照
    /// （`success = "0"` 表示未接入对应第三方查询）。
    pub async fn shipping_info(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<Option<ShippingInfoVO>> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        if user_id != 0 && order.user_id != user_id {
            return Ok(None);
        }
        if order.tracking_no.trim().is_empty() {
            return Ok(None);
        }
        if self
            .repos
            .trade
            .order_admin
            .find_company_name(order.logistics_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }

        // 当前激活服务商为快递鸟时，实时查询 1002 轨迹。
        let provider = crate::logistics::logistics_company::LogisticsCompanyService::new(
            self.repos.clone(),
        )
        .resolve_active_provider_type()
        .await?;
        if provider == nextrade_core::models::logistics::logistics_company::PROVIDER_KDNIAO {
            if let Some((company_code, _)) = self
                .repos
                .trade
                .order_admin
                .find_company_code_name(order.logistics_id)
                .await?
            {
                if let Ok(client) =
                    crate::logistics::kdniao::KdniaoClient::from_repos(&self.repos).await
                {
                    let customer_name = Self::kdn_customer_name(&company_code, &order.mobile);
                    match client
                        .query_track(&company_code, &order.tracking_no, customer_name.as_deref())
                        .await
                    {
                        Ok(r) if r.success => {
                            return Ok(Some(ShippingInfoVO {
                                e_business_id: None,
                                shipper_code: r.shipper_code.or(Some(company_code)),
                                logistic_code: r.logistic_code.or(Some(order.tracking_no.clone())),
                                state: r.state,
                                state_ex: r.state_ex,
                                location: r.location,
                                traces: r
                                    .traces
                                    .into_iter()
                                    .map(|t| ShippingTraceVO {
                                        action: None,
                                        accept_time: t.accept_time,
                                        accept_station: t.accept_station,
                                        location: t.location,
                                    })
                                    .collect(),
                                success: Some("true".to_string()),
                            }));
                        }
                        Ok(r) => {
                            tracing::warn!(
                                order_id = id,
                                reason = r.reason.as_deref().unwrap_or(""),
                                "快递鸟轨迹查询失败，按 Java 语义返回 null"
                            );
                            return Ok(None);
                        }
                        Err(e) => {
                            tracing::warn!(order_id = id, "快递鸟轨迹查询异常: {e}");
                            return Ok(None);
                        }
                    }
                }
            }
        } else if provider == nextrade_core::models::logistics::logistics_company::PROVIDER_KUAIDI100
        {
            // 快递100 实时轨迹（Java Kuaidi100LogisticsProvider#queryTrack）
            if let Some(company) = self
                .repos
                .logistics
                .logistics_company
                .find_by_id(order.logistics_id)
                .await?
            {
                let com = nextrade_core::models::logistics::logistics_company::resolve_kuaidi100_code(
                    Some(&company.logistics_code),
                )
                .or_else(|| {
                    let s = company.kuaidi100_code.trim();
                    (!s.is_empty()).then(|| s.to_string())
                });
                let Some(com) = com else {
                    tracing::warn!(order_id = id, "物流公司未配置快递100编码，跳过轨迹查询");
                    return Ok(None);
                };
                let Ok(client) =
                    crate::logistics::kuaidi100::Kuaidi100Client::from_repos(&self.repos).await
                else {
                    return Ok(None);
                };
                if !client.is_configured() {
                    tracing::warn!("快递100凭证未配置，跳过轨迹查询");
                    return Ok(None);
                }
                match client.query_track(&com, &order.tracking_no, None).await {
                    Ok(r) if r.success => {
                        return Ok(Some(ShippingInfoVO {
                            e_business_id: None,
                            shipper_code: r.shipper_code.or(Some(com)),
                            logistic_code: r
                                .logistic_code
                                .or(Some(order.tracking_no.clone())),
                            state: r.state,
                            state_ex: r.state_ex,
                            location: r.location,
                            traces: r
                                .traces
                                .into_iter()
                                .map(|t| ShippingTraceVO {
                                    action: None,
                                    accept_time: t.accept_time,
                                    accept_station: t.accept_station,
                                    location: t.location,
                                })
                                .collect(),
                            success: Some("true".to_string()),
                        }));
                    }
                    Ok(r) => {
                        tracing::warn!(
                            order_id = id,
                            reason = r.reason.as_deref().unwrap_or(""),
                            "快递100轨迹查询未成功，按 Java 语义返回 null"
                        );
                        return Ok(None);
                    }
                    Err(e) => {
                        tracing::warn!(order_id = id, "快递100轨迹查询异常: {e}");
                        return Ok(None);
                    }
                }
            }
        }

        Ok(Some(ShippingInfoVO {
            e_business_id: None,
            shipper_code: Some(order.logistics_name.clone()),
            logistic_code: Some(order.tracking_no.clone()),
            state: None,
            state_ex: None,
            location: None,
            traces: Vec::new(),
            success: Some("0".to_string()),
        }))
    }

    /// 快递鸟 SF/KYSY/ZTO 查询所需的手机号后四位。
    fn kdn_customer_name(company_code: &str, mobile: &str) -> Option<String> {
        let code = company_code.to_ascii_uppercase();
        if !matches!(code.as_str(), "SF" | "KYSY" | "ZTO") || mobile.len() < 4 {
            return None;
        }
        Some(mobile[mobile.len() - 4..].to_string())
    }

    /// 订单列表页配置（`GET /order/order/getOrderPageConfig`）。
    ///
    /// 对应 Java `configService.getSettings(OrderPageConfigVO.class)`：
    /// `config.biz_code = isChangeOrderStatus`（`"1"` → 1）。
    pub async fn order_page_config(&self) -> Result<OrderPageConfigVO> {
        let raw = self
            .repos
            .trade
            .order_admin
            .find_config_val("isChangeOrderStatus")
            .await?;
        Ok(OrderPageConfigVO {
            is_change_order_status: if raw.as_deref() == Some("1") { 1 } else { 0 },
        })
    }

    /// 修改订单状态（`POST /order/order/changeOrderStatus`）。
    ///
    /// 对应 Java `changeOrderStatus`：
    /// - `val = 3`（已取消）→ 走「后台取消订单」售后流程；
    /// - 平台取消已付款订单（配置 `isPlatformCancelPaidOrder=1` 且状态=1）→ 回退到已取消；
    /// - 平台取消已发货订单（配置 `isPlatformCancelDeliverOrder=1` 且状态=2）→ 回退到待发货；
    /// - 未开启对应配置 → 静默返回（Java 也直接 return）。
    pub async fn change_order_status(
        &self,
        param: &OrderChangeStatusParam,
        admin_id: i32,
        operator: Option<&str>,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::Validation("订单ID不能为空".into()))?;
        let field = param
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        let val = param
            .val
            .ok_or_else(|| Error::Validation("目标状态不能为空".into()))?;
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        if field != "orderStatus" {
            return Err(Error::Validation("仅支持修改订单状态".into()));
        }
        if val == OrderStatus::Cancelled as i32 {
            let mut admin_param = AftersalesApplyAdminParam {
                order_id: Some(id),
                description: param.admin_note.clone(),
                ..Default::default()
            };
            if admin_param.description.is_none() {
                admin_param.description = Some("商家取消订单".into());
            }
            let _ = self
                .cancel_order_by_admin(&admin_param, operator)
                .await?;
            return Ok(());
        }
        let old_status = order.order_status;
        let cfg_paid = self
            .repos
            .trade
            .order_admin
            .find_config_val("isPlatformCancelPaidOrder")
            .await?
            .as_deref()
            == Some("1");
        let cfg_deliver = self
            .repos
            .trade
            .order_admin
            .find_config_val("isPlatformCancelDeliverOrder")
            .await?
            .as_deref()
            == Some("1");
        let target = if cfg_paid && old_status == OrderStatus::Confirmed as i32 {
            Some(OrderStatus::Cancelled)
        } else if cfg_deliver && old_status == OrderStatus::Processing as i32 {
            Some(OrderStatus::Confirmed)
        } else {
            None
        };
        let Some(target) = target else {
            return Ok(());
        };
        let mut patch = OrderPatch::new().i32_val("order_status", target as i32);
        if let Some(note) = param.admin_note.as_deref() {
            patch = patch.str_val("admin_note", note);
        }
        if target == OrderStatus::Cancelled {
            let items = self.repos.trade.order_admin.find_items(id).await?;
            for it in &items {
                self.repos
                    .trade
                    .order_admin
                    .restore_stock(it.product_id, it.sku_id, it.quantity)
                    .await?;
            }
        } else if target == OrderStatus::Confirmed {
            patch = patch
                .i32_val("shipping_status", ShippingStatus::Pending as i32)
                .i32_val("shipping_time", 0)
                .i32_val("logistics_id", 0)
                .str_val("tracking_no", "")
                .str_val("logistics_name", "");
        }
        self.repos.trade.order_admin.update_order(id, &patch).await?;
        let desc = format!(
            "订单状态由{}改为{}",
            OrderStatus::name_of(old_status),
            OrderStatus::name_of(target as i32)
        );
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, &desc, admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 催发货（`POST /order/order/remindDeliver`）。
    ///
    /// 对应 Java `remindDeliver`：仅供应商订单（`vendor_id > 0`）且状态为
    /// 「已确认，待发货」可催；写入 `admin_msg`（类型 54 发货提醒，vendor 维度），
    /// 同时落订单日志便于审计。
    pub async fn remind_deliver(&self, id: i32, admin_id: i32) -> Result<()> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        if order.vendor_id.is_none_or(|v| v <= 0) {
            return Err(Error::biz_msg("非供应商订单不允许操作"));
        }
        if order.order_status != OrderStatus::Confirmed as i32 {
            return Err(Error::biz_msg("订单状态错误"));
        }
        let vendor_id = order.vendor_id.unwrap_or(0);
        let title = format!(
            "发货提醒：{},金额：{}",
            order.order_sn, order.total_amount
        );
        let content = format!(
            "收到订单号：{}，金额：{}，的催发货提醒，请尽快进行发货",
            order.order_sn, order.total_amount
        );
        let related_data = serde_json::json!({ "orderId": order.order_id }).to_string();
        let _ = self
            .repos
            .system
            .admin_msg
            .create_vendor_message(
                54, // AdminMsgTypeEnum.QUICK_DELIVERY
                vendor_id,
                order.shop_id,
                order.order_id,
                &title,
                &content,
                &related_data,
            )
            .await;
        self.repos
            .trade
            .order_admin
            .insert_order_log(id, &order.order_sn, "催发货", admin_id, order.shop_id)
            .await?;
        Ok(())
    }

    /// 批量发货模板（`GET /order/order/downloadDeliverTemplate?ids=`）。
    ///
    /// 对应 Java `downloadDeliverTemplate`：每个订单的每个商品一行，
    /// 发货数量默认 = 购买数量、配送方式/物流公司/单号留空待填写。
    ///
    /// ⚠️ **降级**：Java 回写 xlsx；Rust 返回「说明 + 表头 + 行数据」JSON。
    pub async fn download_deliver_template(
        &self,
        ids: &[i32],
    ) -> Result<OrderDeliverTemplateVO> {
        if ids.is_empty() {
            return Err(Error::biz_msg("请选择要发货的订单"));
        }
        let mut rows = Vec::new();
        for id in ids {
            let vo = self.detail(*id).await?;
            let users = self
                .repos
                .trade
                .order_admin
                .find_users(&[vo.user_id])
                .await?;
            let username = users
                .first()
                .map(|u| u.username.clone())
                .unwrap_or_default();
            let full_address = if vo.region_names.is_empty() {
                vo.address.clone()
            } else {
                format!("{} {}", vo.region_names.join(" "), vo.address)
            };
            if vo.items.is_empty() {
                rows.push(OrderDeliverTemplateRow {
                    order_sn: vo.order_sn.clone(),
                    username: username.clone(),
                    consignee: vo.consignee.clone(),
                    mobile: vo.mobile.clone(),
                    address: full_address.trim().to_string(),
                    total_amount: vo.total_amount.to_string(),
                    product_name: String::new(),
                    sku_name: String::new(),
                    quantity: String::new(),
                    delivery_quantity: String::new(),
                    shipping_method: String::new(),
                    logistics_name: String::new(),
                    logistics_code: String::new(),
                    tracking_no: String::new(),
                    add_time: vo.add_time.clone(),
                });
                continue;
            }
            for item in &vo.items {
                let sku_name = item
                    .sku_value
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "-".to_string());
                rows.push(OrderDeliverTemplateRow {
                    order_sn: vo.order_sn.clone(),
                    username: username.clone(),
                    consignee: vo.consignee.clone(),
                    mobile: vo.mobile.clone(),
                    address: full_address.trim().to_string(),
                    total_amount: vo.total_amount.to_string(),
                    product_name: item.product_name.clone(),
                    sku_name,
                    quantity: item.quantity.to_string(),
                    delivery_quantity: item.quantity.to_string(),
                    shipping_method: String::new(),
                    logistics_name: String::new(),
                    logistics_code: String::new(),
                    tracking_no: String::new(),
                    add_time: vo.add_time.clone(),
                });
            }
        }
        Ok(OrderDeliverTemplateVO {
            file_name: "批量发货模板.xlsx".to_string(),
            description: ORDER_DELIVER_TEMPLATE_DESCRIPTION.to_string(),
            headers: ORDER_DELIVER_TEMPLATE_HEADERS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            rows,
        })
    }

    /// 批量发货模板导出为 xlsx（Java `downloadDeliverTemplate` 直接回写 xlsx）。
    pub async fn download_deliver_template_xlsx(
        &self,
        ids: &[i32],
    ) -> Result<(String, Vec<u8>)> {
        let vo = self.download_deliver_template(ids).await?;
        let headers: Vec<&str> = vo.headers.iter().map(String::as_str).collect();
        let rows: Vec<Vec<String>> = vo
            .rows
            .iter()
            .map(|row| {
                vec![
                    row.order_sn.clone(),
                    row.username.clone(),
                    row.consignee.clone(),
                    row.mobile.clone(),
                    row.address.clone(),
                    row.total_amount.clone(),
                    row.product_name.clone(),
                    row.sku_name.clone(),
                    row.quantity.clone(),
                    row.delivery_quantity.clone(),
                    row.shipping_method.clone(),
                    row.logistics_name.clone(),
                    row.logistics_code.clone(),
                    row.tracking_no.clone(),
                    row.add_time.clone(),
                ]
            })
            .collect();
        let leading = vec![vo.description.clone()];
        let bytes = crate::system::excel::build_xlsx_with_leading(
            &headers,
            Some(&leading),
            &rows,
            "批量发货模板",
        )
        .map_err(|e| Error::Internal(format!("批量发货模板生成失败: {e}")))?;
        Ok((vo.file_name, bytes))
    }

    /// 批量发货导入失败数据导出为 xlsx。
    pub async fn batch_deliver_import_error_xlsx(
        &self,
        bytes: &[u8],
    ) -> Result<(String, Vec<u8>)> {
        let rows = self.batch_deliver_import_error_data(bytes).await?;
        let headers = [
            "行号",
            "订单编号",
            "商品名称",
            "商品规格",
            "发货数量",
            "配送方式",
            "物流公司",
            "物流单号",
            "错误信息",
        ];
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.row_num.to_string(),
                    row.order_sn.clone(),
                    row.product_name.clone().unwrap_or_default(),
                    row.sku_name.clone().unwrap_or_default(),
                    row.delivery_quantity.clone().unwrap_or_default(),
                    row.shipping_method.clone().unwrap_or_default(),
                    row.logistics_name.clone().unwrap_or_default(),
                    row.tracking_no.clone().unwrap_or_default(),
                    row.error_msg.clone(),
                ]
            })
            .collect();
        let bytes = crate::system::excel::build_xlsx(&headers, &data, "错误数据")
            .map_err(|e| Error::Internal(format!("失败数据 Excel 生成失败: {e}")))?;
        Ok(("批量发货失败数据.xlsx".to_string(), bytes))
    }

    /// 批量发货 Excel/CSV/JSON 导入（`POST /order/order/batchDeliverImport`）。
    ///
    /// 对应 Java `batchDeliverImport`：按订单号分组，每个订单只发一次；
    /// 订单级校验 → 配送方式/物流公司校验 → 逐行匹配商品并汇总发货数量 →
    /// 调用 `deliverOrder`。失败行进 `failDetails`，不阻断其它订单。
    ///
    /// 支持 `.xlsx`（calamine）以及 JSON 数组 / CSV 文本（跳过说明行+表头）。
    pub async fn batch_deliver_import(
        &self,
        bytes: &[u8],
        _file_name: Option<&str>,
    ) -> Result<OrderDeliverImportResultVO> {
        let rows = parse_deliver_import_rows(bytes)?;
        if rows.is_empty() {
            return Err(Error::biz_msg("表格数据为空"));
        }
        let groups = group_import_rows(&rows);
        let mut fail_details: Vec<OrderDeliverImportFailVO> = Vec::new();
        let mut success_count = 0;
        for (order_sn, row_list) in groups {
            let first = &row_list[0].1;
            let mut order_errors: Vec<(i32, String)> = Vec::new();
            if let Some(err) = validate_deliver_excel_data(first) {
                for (row_num, _) in &row_list {
                    order_errors.push((*row_num, err.clone()));
                }
                push_fail(&mut fail_details, &order_errors, &order_sn);
                continue;
            }
            let Some(order) = self
                .repos
                .trade
                .order_admin
                .find_order_by_sn(&order_sn)
                .await?
            else {
                push_fail(
                    &mut fail_details,
                    &row_list.iter().map(|(n, _)| (*n, "订单不存在".to_string())).collect::<Vec<_>>(),
                    &order_sn,
                );
                continue;
            };
            let actions = self.order_actions(&order).await?;
            if !actions.deliver {
                push_fail(
                    &mut fail_details,
                    &row_list.iter().map(|(n, _)| (*n, "订单状态不允许发货".to_string())).collect::<Vec<_>>(),
                    &order_sn,
                );
                continue;
            }
            let Some(method) = first.shipping_method.as_deref().and_then(parse_shipping_method_name) else {
                push_fail(
                    &mut fail_details,
                    &row_list.iter().map(|(n, _)| (*n, "无效的配送方式，有效值：物流配送、商家配送、无需配送、电子面单".to_string())).collect::<Vec<_>>(),
                    &order_sn,
                );
                continue;
            };
            if let Some(err) = validate_shipping_method_fields(first, method) {
                push_fail(
                    &mut fail_details,
                    &row_list.iter().map(|(n, _)| (*n, err.clone())).collect::<Vec<_>>(),
                    &order_sn,
                );
                continue;
            }
            let mut logistics_id = 0i32;
            if method == shipping_method::LOGISTICS || method == shipping_method::WAYBILL {
                let name = first.logistics_name.as_deref().unwrap_or("").trim();
                match self.find_logistics_company_by_name(name).await? {
                    Some(id) => logistics_id = id,
                    None => {
                        push_fail(
                            &mut fail_details,
                            &row_list
                                .iter()
                                .map(|(n, _)| (*n, format!("物流公司不存在：{name}")))
                                .collect::<Vec<_>>(),
                            &order_sn,
                        );
                        continue;
                    }
                }
            }
            let items = self.repos.trade.order_admin.find_items(order.order_id).await?;
            let mut deliver_info = Vec::new();
            let mut has_error = false;
            for (row_num, row) in &row_list {
                let qty = match parse_delivery_quantity(row) {
                    Ok(q) => q,
                    Err(msg) => {
                        fail_details.push(OrderDeliverImportFailVO {
                            row_num: *row_num,
                            order_sn: order_sn.clone(),
                            reason: msg,
                        });
                        has_error = true;
                        continue;
                    }
                };
                let matched = match_order_item(
                    &items,
                    row.product_name.as_deref(),
                    row.sku_name.as_deref(),
                );
                let Some(matched) = matched else {
                    fail_details.push(OrderDeliverImportFailVO {
                        row_num: *row_num,
                        order_sn: order_sn.clone(),
                        reason: format!(
                            "未找到匹配的商品：{}{}",
                            row.product_name.as_deref().unwrap_or(""),
                            row.sku_name
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                                .map(|s| format!("({s})"))
                                .unwrap_or_default()
                        ),
                    });
                    has_error = true;
                    continue;
                };
                deliver_info.push(nextrade_core::models::trade::order_admin::DeliverInfo {
                    item_id: Some(matched.item_id),
                    to_delivery_quantity: Some(qty.unwrap_or(matched.quantity)),
                });
            }
            if has_error || deliver_info.is_empty() {
                continue;
            }
            let param = DeliverOrderParam {
                id: Some(order.order_id),
                deliver_data: Some(deliver_info),
                shipping_method: Some(method),
                logistics_id: Some(logistics_id),
                tracking_no: first
                    .tracking_no
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                ..Default::default()
            };
            match self.deliver(&param).await {
                Ok(()) => success_count += 1,
                Err(e) => push_fail(
                    &mut fail_details,
                    &row_list
                        .iter()
                        .map(|(n, _)| (*n, format!("发货失败：{e}")))
                        .collect::<Vec<_>>(),
                    &order_sn,
                ),
            }
        }
        Ok(OrderDeliverImportResultVO {
            total_count: rows.len() as i32,
            success_count,
            fail_count: fail_details.len() as i32,
            fail_details,
        })
    }

    /// 批量发货导入失败数据（`POST /order/order/getBatchDeliverImportErrorData`）。
    ///
    /// 校验逻辑与 [`Self::batch_deliver_import`] 完全一致；本方法返回失败行数据，
    /// 路由层再经 [`Self::batch_deliver_import_error_xlsx`] 输出 xlsx 下载流。
    pub async fn batch_deliver_import_error_data(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<OrderDeliverImportErrorVO>> {
        let rows = parse_deliver_import_rows(bytes)?;
        if rows.is_empty() {
            return Err(Error::biz_msg("表格数据为空"));
        }
        let mut errors = Vec::new();
        for (order_sn, row_list) in group_import_rows(&rows) {
            let first = &row_list[0].1;
            let shipping_method = first
                .shipping_method
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .to_string();
            let logistics_name = first.logistics_name.clone().unwrap_or_default();
            let tracking_no = first.tracking_no.clone().unwrap_or_default();
            if let Some(err) = validate_deliver_excel_data(first) {
                push_all_import_errors(
                    &mut errors,
                    &order_sn,
                    &row_list,
                    &shipping_method,
                    &logistics_name,
                    &tracking_no,
                    &err,
                );
                continue;
            }
            let Some(order) = self
                .repos
                .trade
                .order_admin
                .find_order_by_sn(&order_sn)
                .await?
            else {
                push_all_import_errors(
                    &mut errors,
                    &order_sn,
                    &row_list,
                    &shipping_method,
                    &logistics_name,
                    &tracking_no,
                    "订单不存在",
                );
                continue;
            };
            let actions = self.order_actions(&order).await?;
            if !actions.deliver {
                push_all_import_errors(
                    &mut errors,
                    &order_sn,
                    &row_list,
                    &shipping_method,
                    &logistics_name,
                    &tracking_no,
                    "订单状态不允许发货",
                );
                continue;
            }
            let method = if shipping_method.is_empty() {
                None
            } else {
                parse_shipping_method_name(&shipping_method)
            };
            let Some(method) = method else {
                push_all_import_errors(
                    &mut errors,
                    &order_sn,
                    &row_list,
                    &shipping_method,
                    &logistics_name,
                    &tracking_no,
                    "无效的配送方式，有效值：物流配送、商家配送、无需配送、电子面单",
                );
                continue;
            };
            if let Some(err) = validate_shipping_method_fields(first, method) {
                push_all_import_errors(
                    &mut errors,
                    &order_sn,
                    &row_list,
                    &shipping_method,
                    &logistics_name,
                    &tracking_no,
                    &err,
                );
                continue;
            }
            if (method == shipping_method::LOGISTICS || method == shipping_method::WAYBILL)
                && self
                    .find_logistics_company_by_name(logistics_name.trim())
                    .await?
                    .is_none()
            {
                push_all_import_errors(
                    &mut errors,
                    &order_sn,
                    &row_list,
                    &shipping_method,
                    &logistics_name,
                    &tracking_no,
                    &format!("物流公司不存在：{logistics_name}"),
                );
                continue;
            }
            let items = self.repos.trade.order_admin.find_items(order.order_id).await?;
            for (row_num, row) in &row_list {
                if let Err(msg) = parse_delivery_quantity(row) {
                    errors.push(build_import_error_row(
                        *row_num,
                        &order_sn,
                        row,
                        &shipping_method,
                        &logistics_name,
                        &tracking_no,
                        msg.as_str(),
                    ));
                    continue;
                }
                if match_order_item(&items, row.product_name.as_deref(), row.sku_name.as_deref())
                    .is_none()
                {
                    errors.push(build_import_error_row(
                        *row_num,
                        &order_sn,
                        row,
                        &shipping_method,
                        &logistics_name,
                        &tracking_no,
                        &format!(
                            "未找到匹配的商品：{}{}",
                            row.product_name.as_deref().unwrap_or(""),
                            row.sku_name
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                                .map(|s| format!("({s})"))
                                .unwrap_or_default()
                        ),
                    ));
                }
            }
        }
        Ok(errors)
    }

    // ————————————————————— 内部辅助 —————————————————————

    /// 订单可操作项（`availableActions`；整单售后/在售商品批量预取）。
    async fn order_actions(&self, order: &Order) -> Result<OrderActionVO> {
        let total_qty = self
            .repos
            .trade
            .order_admin
            .order_items_total_quantity(order.order_id)
            .await?;
        let full_aftersales = if total_qty > 0 {
            let covered = self
                .repos
                .trade
                .order_admin
                .aftersales_covered_quantity(order.order_id, &aftersales_status::valid_statuses())
                .await?;
            covered >= total_qty
        } else {
            false
        };
        let items = self.repos.trade.order_admin.find_items(order.order_id).await?;
        let product_ids = distinct_i32(items.iter().map(|it| it.product_id));
        let rebuy_available = items.is_empty()
            || self
                .repos
                .trade
                .order_admin
                .any_product_on_sale(&product_ids)
                .await?;
        Ok(available_actions(order, full_aftersales, rebuy_available))
    }

    /// 重算订单金额（对应 Java `updateOrderMoney(order, true)` 的可迁移部分）：
    /// `product_amount = Σ(price × quantity)`；
    /// `total_amount = product_amount - 折扣 - 优惠券 - 积分抵现 + 运费 + 发票费 + 服务费`；
    /// `unpaid_amount = max(total - paid, 0)`。
    ///
    /// ⚠️ Java 还会按 `order_extension` 重算各店铺金额明细（`order_amount_detail`），
    /// 该表未迁移，店铺级分摊保持原值。
    async fn recompute_order_money(&self, order_id: i32) -> Result<()> {
        let order = self
            .repos
            .trade
            .order_admin
            .find_order(order_id)
            .await?
            .ok_or_else(|| Error::biz_msg("订单不存在"))?;
        let items = self.repos.trade.order_admin.find_items(order_id).await?;
        let product_amount: Decimal = items
            .iter()
            .map(|it| it.price * Decimal::from(it.quantity))
            .sum();
        let total = product_amount
            - order.discount_amount
            - order.coupon_amount
            - order.points_amount
            + order.shipping_fee
            + order.invoice_fee
            + order.service_fee;
        let unpaid = (total - order.paid_amount).max(Decimal::ZERO);
        self.repos
            .trade
            .order_admin
            .update_order(
                order_id,
                &OrderPatch::new()
                    .decimal("product_amount", product_amount)
                    .decimal("total_amount", total)
                    .decimal("unpaid_amount", unpaid),
            )
            .await?;
        Ok(())
    }

    /// 按物流公司名查询 ID（批量发货导入用）。
    async fn find_logistics_company_by_name(&self, name: &str) -> Result<Option<i32>> {
        if name.is_empty() {
            return Ok(None);
        }
        self.repos
            .trade
            .order_admin
            .find_logistics_company_id_by_name(name)
            .await
    }
}

// ————————————————————— wave3 纯函数辅助（可单测） —————————————————————

/// 订单导出可选列（顺序与 Java `getExportItemList()` 一致）。
/// 单个导出单元格取值（列 key → 展示文本），与 Java EasyExcel 字段一致。
fn export_cell(key: &str, row: &OrderExportRowVO) -> String {
    match key {
        "orderSn" => row.order_sn.clone(),
        "addTime" => row.add_time.clone(),
        "orderStatusName" => row.order_status_name.clone(),
        "payStatusName" => row.pay_status_name.clone(),
        "shopName" => row.shop_name.clone(),
        "username" => row.username.clone(),
        "consignee" => row.consignee.clone(),
        "mobile" => row.mobile.clone(),
        "address" => row.address.clone(),
        "postcode" => row.postcode.clone(),
        "totalAmount" => row.total_amount.to_string(),
        "balance" => row.balance.to_string(),
        "discountAmount" => row.discount_amount.to_string(),
        "pointsAmount" => row.points_amount.to_string(),
        "couponAmount" => row.coupon_amount.to_string(),
        "payTime" => row.pay_time.clone().unwrap_or_default(),
        "payTypeId" => row.pay_type_id.to_string(),
        "payTypeName" => row.pay_type_name.clone(),
        "payCode" => row.pay_code.clone(),
        "outTradeNo" => row.out_trade_no.clone().unwrap_or_default(),
        "paidAmount" => row.paid_amount.to_string(),
        "shippingTime" => row.shipping_time.clone().unwrap_or_default(),
        "shippingFee" => row.shipping_fee.to_string(),
        "logisticsName" => row.logistics_name.clone(),
        "trackingNo" => row.tracking_no.clone(),
        "buyerNote" => row.buyer_note.clone(),
        "adminNote" => row.admin_note.clone(),
        "shippingStatusName" => row.shipping_status_name.clone(),
        "productWeight" => row.product_weight.to_string(),
        "usePoints" => row.use_points.to_string(),
        "productName" => row.product_name.clone(),
        "skuData" => row.sku_data.clone(),
        "productSn" => row.product_sn.clone(),
        "quantity" => row.quantity.to_string(),
        "price" => row.price.to_string(),
        "totalPrice" => row.total_price.to_string(),
        other => String::new(),
    }
}

fn export_columns() -> Vec<OrderExportItemVO> {
    const COLS: [(&str, &str); 30] = [
        ("orderSn", "订单编号"),
        ("addTime", "下单时间"),
        ("orderStatusName", "订单状态"),
        ("payStatusName", "支付状态"),
        ("shopName", "店铺名称"),
        ("username", "会员名称"),
        ("consignee", "收件人姓名"),
        ("mobile", "收件人电话"),
        ("address", "收件人地址"),
        ("postcode", "收件人邮编"),
        ("totalAmount", "订单总价"),
        ("balance", "使用余额"),
        ("discountAmount", "折扣"),
        ("pointsAmount", "积分抵金额"),
        ("couponAmount", "优惠券金额"),
        ("payTime", "支付时间"),
        ("payTypeId", "支付类型"),
        ("payCode", "支付方式"),
        ("outTradeNo", "交易单号"),
        ("paidAmount", "支付金额"),
        ("shippingTime", "发货时间"),
        ("shippingFee", "运费"),
        ("logisticsName", "物流名称"),
        ("trackingNo", "发货单号"),
        ("buyerNote", "订单备注"),
        ("adminNote", "商家备注"),
        ("productInfo", "商品信息"),
        ("shippingStatusName", "发货状态"),
        ("productWeight", "总重量(KG)"),
        ("usePoints", "使用积分"),
    ];
    COLS.iter()
        .map(|(key, label)| OrderExportItemVO {
            key: (*key).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

/// 展开选中导出列（Java `orderExport` 的 `productInfo` / `payTypeId` 展开规则）。
fn expand_export_columns(selected: &[String]) -> Result<Vec<OrderExportItemVO>> {
    let all = export_columns();
    let mut keys: Vec<String> = Vec::new();
    for key in selected {
        if key == "productInfo" {
            for expanded in [
                "productName",
                "skuData",
                "productSn",
                "quantity",
                "price",
                "totalPrice",
            ] {
                keys.push(expanded.to_string());
            }
        } else {
            keys.push(key.clone());
            if key == "payTypeId" {
                keys.push("payTypeName".to_string());
            }
        }
    }
    let mut columns = Vec::new();
    for key in keys {
        // 原版前端未选择列时回传导出列的数组下标（如 `0,1`），
        // 兼容“键名”和“下标”两种提交形式。
        if let Some(col) = key
            .parse::<usize>()
            .ok()
            .and_then(|idx| all.get(idx))
            .cloned()
        {
            columns.push(col);
            continue;
        }
        if let Some(col) = all.iter().find(|c| c.key == key) {
            columns.push(col.clone());
            continue;
        }
        // 展开字段没有单独的列定义，按 Java `@ExcelProperty` 文案补齐
        let label = match key.as_str() {
            "productName" => "商品名称",
            "skuData" => "商品规格",
            "productSn" => "商品编号",
            "quantity" => "购买数量",
            "price" => "购买价格",
            "totalPrice" => "小计",
            "payTypeName" => "支付类型",
            _ => {
                return Err(Error::Validation(format!("无效的导出项: {key}")));
            }
        };
        columns.push(OrderExportItemVO {
            key,
            label: label.to_string(),
        });
    }
    // 去重（保持首次出现顺序）
    let mut seen = HashSet::new();
    columns.retain(|c| seen.insert(c.key.clone()));
    Ok(columns)
}

/// 支付类型文案（Java `payTypeName` 翻译前的兜底文案）。
fn pay_type_name(id: i32) -> String {
    match id {
        1 => "在线支付",
        2 => "货到付款",
        _ => "线下支付",
    }
    .to_string()
}

/// 折扣按明细小计比例分摊（Java `modifyOrderMoney` 的分配算法）：
/// 除最后一行外按 `折扣 × 行小计 / 总小计` 四舍五入（2 位），最后一行取差额；
/// 单价 = 原单价 - 每件分摊（不小于 0）；返回 `(item_id, 新单价, 改价前单价)`。
fn allocate_discount(
    items: &[OrderItem],
    discount: Decimal,
) -> Result<Vec<(i32, Decimal, Option<Decimal>)>> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let originals: Vec<Decimal> = items
        .iter()
        .map(|it| it.admin_change_pre_price.unwrap_or(it.price))
        .collect();
    let subtotals: Vec<Decimal> = items
        .iter()
        .zip(originals.iter())
        .map(|(it, unit)| *unit * Decimal::from(it.quantity))
        .collect();
    let total: Decimal = subtotals.iter().copied().sum();
    if total <= Decimal::ZERO {
        return Err(Error::biz_msg(
            "商品价格总和减去优惠价格（优惠券抵扣，优惠折扣）必须大于0",
        ));
    }
    let mut updates = Vec::with_capacity(items.len());
    let mut allocated = Decimal::ZERO;
    let mut new_price_sum = Decimal::ZERO;
    for (i, it) in items.iter().enumerate() {
        let line_discount = if i + 1 < items.len() {
            let v = (discount * subtotals[i] / total)
                .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
            allocated += v;
            v
        } else {
            discount - allocated
        };
        let qty = Decimal::from(it.quantity.max(1));
        let per_unit =
            (line_discount / qty).round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        let mut new_price = originals[i] - per_unit;
        if new_price < Decimal::ZERO {
            new_price = Decimal::ZERO;
        }
        new_price_sum += new_price;
        updates.push((it.item_id, new_price, Some(originals[i])));
    }
    if new_price_sum <= Decimal::ZERO {
        return Err(Error::biz_msg(
            "商品价格总和减去优惠价格（优惠券抵扣，优惠折扣）必须大于0",
        ));
    }
    Ok(updates)
}

/// 合并收货人地址快照（Java `modifyOrderConsignee` 构造 `UserAddressVO` 的逻辑）：
/// 保留原快照中的地址标签 / 用户 / 默认标记，覆盖展示字段。
fn merge_address_data(
    old: Option<&str>,
    p: &OrderModifyConsigneeParam,
    region_ids: &[i32],
    region_names: &[String],
) -> Option<String> {
    let mut obj = old
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(v) = p.address.as_deref() {
        obj["address"] = serde_json::json!(v);
    }
    if !region_ids.is_empty() {
        obj["regionIds"] = serde_json::json!(region_ids);
    }
    if !region_names.is_empty() {
        obj["regionNames"] = serde_json::json!(region_names);
    }
    if let Some(v) = p.postcode.as_deref() {
        obj["postcode"] = serde_json::json!(v);
    }
    if let Some(v) = p.telephone.as_deref() {
        obj["telephone"] = serde_json::json!(v);
    }
    if let Some(v) = p.mobile.as_deref() {
        obj["mobile"] = serde_json::json!(v);
    }
    if let Some(v) = p.email.as_deref() {
        obj["email"] = serde_json::json!(v);
    }
    Some(obj.to_string())
}

/// 请求明细 → 订单明细实体（`modifyProduct` 整批写入）。
fn to_order_item(order_id: i32, order: &Order, p: &OrderItemParam) -> OrderItem {
    OrderItem {
        item_id: 0,
        order_id,
        order_sn: order.order_sn.clone(),
        user_id: order.user_id,
        price: p.price,
        quantity: p.quantity,
        product_id: p.product_id.unwrap_or(0),
        product_name: p.product_name.clone().unwrap_or_default(),
        product_sn: p.product_sn.clone().unwrap_or_default(),
        pic_thumb: p.pic_thumb.clone().unwrap_or_default(),
        sku_id: p.sku_id.unwrap_or(0),
        sku_data: p
            .sku_data
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()))
            .unwrap_or_else(|| "[]".to_string()),
        delivery_quantity: p.delivery_quantity.unwrap_or(0),
        product_type: p.product_type.unwrap_or(1),
        is_gift: p.is_gift.unwrap_or(0),
        shop_id: p.shop_id.unwrap_or(order.shop_id),
        is_pin: p.is_pin.unwrap_or(0),
        prepay_price: p.prepay_price.unwrap_or(Decimal::ZERO),
        commission: p.commission.clone(),
        origin_price: p.origin_price,
        promotion_data: p.promotion_data.clone(),
        is_seckill: p.is_seckill,
        extra_sku_data: p
            .extra_sku_data
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default()),
        suppliers_id: p.suppliers_id.unwrap_or(0),
        card_group_name: p.card_group_name.clone().unwrap_or_default(),
        vendor_product_id: p.vendor_product_id,
        vendor_product_sku_id: p.vendor_product_sku_id,
        vendor_id: p.vendor_id,
        vendor_product_supply_price: p.vendor_product_supply_price,
        ..Default::default()
    }
}

/// 商品实体 → `ProductAddInfoVO`（`getAddProductInfo` 返回）。
fn to_product_add_info(p: &Product) -> ProductAddInfoVO {
    ProductAddInfoVO {
        product_id: p.product_id,
        product_name: p.product_name.clone(),
        product_sn: p.product_sn.clone(),
        product_stock: p.product_stock,
        product_price: p.product_price,
        market_price: p.market_price,
        product_status: p.product_status,
        product_type: p.product_type,
        shop_id: p.shop_id,
        pic_url: p.pic_url.clone(),
        pic_thumb: p.pic_thumb.clone(),
        product_weight: p.product_weight,
        virtual_sales: p.virtual_sales,
        suppliers_id: p.suppliers_id,
        product_brief: p.product_brief.clone(),
        is_delete: Some(p.is_delete),
    }
}

/// 订单明细规格数组：把下单快照 `order_item.sku_data` 的 JSON 字符串还原成数组。
fn parse_sku_data_array(raw: &str) -> Vec<serde_json::Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(items)) => items,
        _ => Vec::new(),
    }
}

/// 订单明细规格文本（`sku_value` 未写时由 `sku_data` JSON 还原，`名称:值|...`）。
fn item_sku_value(it: &OrderItem) -> Option<String> {
    let raw = it.sku_data.trim();
    if raw.is_empty() {
        return None;
    }
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(raw) else {
        return None;
    };
    let parts: Vec<String> = items
        .iter()
        .filter_map(|v| {
            let o = v.as_object()?;
            let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let value = o.get("value").and_then(|v| v.as_str()).unwrap_or("");
            Some(format!("{name}:{value}"))
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("|"))
    }
}

/// 解析批量发货导入文件：Excel (.xlsx)、JSON 数组或 CSV 文本（跳过说明行 + 表头）。
fn parse_deliver_import_rows(bytes: &[u8]) -> Result<Vec<OrderDeliverImportRow>> {
    // 检查是否为 xlsx 二进制（PK zip magic: 0x50, 0x4B, 0x03, 0x04）
    if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
        let sheet = crate::system::excel::read_xlsx(bytes)
            .map_err(|e| Error::Validation(format!("Excel 解析失败: {e}")))?;
        let mut rows = Vec::new();
        // 物理行 0 = 说明，物理行 1 = 表头，物理行 2 起是数据（从 Excel 第 3 行开始）
        for (idx, line) in sheet.into_iter().enumerate() {
            if idx < 2 {
                continue;
            }
            let get = |i: usize| {
                line.get(i)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            let row = OrderDeliverImportRow {
                order_sn: get(0),
                product_name: get(6),
                sku_name: get(7),
                delivery_quantity: get(9),
                shipping_method: get(10),
                logistics_name: get(11),
                tracking_no: get(13),
                row_num: Some(idx as i32 + 1),
            };
            if row.order_sn.is_some() || row.product_name.is_some() {
                rows.push(row);
            }
        }
        return Ok(rows);
    }

    let text = std::str::from_utf8(bytes)
        .map_err(|e| Error::Validation(format!("文件编码错误: {e}")))?;
    let text = text.trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        return Err(Error::biz_msg("表格数据为空"));
    }
    if text.starts_with('[') {
        let mut rows: Vec<OrderDeliverImportRow> = serde_json::from_str(text)
            .map_err(|e| Error::Validation(format!("导入数据 JSON 解析失败: {e}")))?;
        for (i, row) in rows.iter_mut().enumerate() {
            if row.row_num.is_none() {
                row.row_num = Some(i as i32 + 3);
            }
        }
        return Ok(rows);
    }
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || idx < 2 {
            // CSV 是「整张表」的文本镜像：物理行 0 = Excel 第 1 行（说明），
            // 物理行 1 = Excel 第 2 行（表头），物理行 2 起是数据。
            continue;
        }
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        let get = |i: usize| {
            cols.get(i)
                .map(|s| s.trim_matches('"').trim())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        rows.push(OrderDeliverImportRow {
            order_sn: get(0),
            product_name: get(6),
            sku_name: get(7),
            delivery_quantity: get(9),
            shipping_method: get(10),
            logistics_name: get(11),
            tracking_no: get(13),
            // 物理行号 0-based → Excel 行号 1-based：
            // 首条数据 idx=2 → Excel 第 3 行，与 Java `rowNum = dataListIndex + 3`
            // 在「数据从 Excel 第 3 行开始」时的口径一致。
            row_num: Some(idx as i32 + 1),
        });
    }
    Ok(rows)
}

/// 按订单号分组（保持首次出现顺序），行号缺省按 `index + 3` 补全。
fn group_import_rows(
    rows: &[OrderDeliverImportRow],
) -> Vec<(String, Vec<(i32, OrderDeliverImportRow)>)> {
    let mut groups: Vec<(String, Vec<(i32, OrderDeliverImportRow)>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        let sn = row.order_sn.as_deref().unwrap_or("").trim().to_string();
        let row_num = row.row_num.unwrap_or(i as i32 + 3);
        let pos = *index.entry(sn.clone()).or_insert_with(|| {
            groups.push((sn.clone(), Vec::new()));
            groups.len() - 1
        });
        groups[pos].1.push((row_num, row.clone()));
    }
    groups
}

/// 配送方式名 → 数值（Java \"parseShippingMethod\"）。
fn parse_shipping_method_name(name: &str) -> Option<i32> {
    match name.trim() {
        "物流配送" => Some(shipping_method::LOGISTICS),
        "商家配送" => Some(shipping_method::MERCHANT),
        "无需配送" => Some(shipping_method::NONE),
        "电子面单" => Some(shipping_method::WAYBILL),
        _ => None,
    }
}

/// 基础字段校验（Java `validateDeliverExcelData`）。
fn validate_deliver_excel_data(row: &OrderDeliverImportRow) -> Option<String> {
    if row.order_sn.as_deref().is_none_or(|s| s.trim().is_empty()) {
        return Some("订单编号不能为空".to_string());
    }
    if row
        .shipping_method
        .as_deref()
        .is_none_or(|s| s.trim().is_empty())
    {
        return Some("配送方式不能为空".to_string());
    }
    None
}

/// 按配送方式校验必填项（Java `validateShippingMethodFields`）。
fn validate_shipping_method_fields(row: &OrderDeliverImportRow, method: i32) -> Option<String> {
    let blank = |v: Option<&str>| v.is_none_or(|s| s.trim().is_empty());
    match method {
        shipping_method::LOGISTICS => {
            if blank(row.logistics_name.as_deref()) {
                return Some("配送方式为物流配送时，物流公司必填".to_string());
            }
            if blank(row.tracking_no.as_deref()) {
                return Some("配送方式为物流配送时，物流单号必填".to_string());
            }
        }
        shipping_method::WAYBILL => {
            if blank(row.logistics_name.as_deref()) {
                return Some("配送方式为电子面单时，物流公司必填".to_string());
            }
        }
        shipping_method::MERCHANT | shipping_method::NONE => {}
        _ => return Some("无效的配送方式".to_string()),
    }
    None
}

/// 解析发货数量（`std::result::Result<Option<i32>, 错误文案>`）：空白 → `None`（整行数量）。
///
/// 注意：本文件顶部 `use nextrade_core::Result` 引用了单参数别名
/// `Result<T> = std::result::Result<T, nextrade_core::Error>`，因此这里必须显式写
/// `std::result::Result<_, String>`，不能用 2 参数形式。
fn parse_delivery_quantity(row: &OrderDeliverImportRow) -> std::result::Result<Option<i32>, String> {
    let Some(raw) = row.delivery_quantity.as_deref().map(str::trim).filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };
    match raw.parse::<i32>() {
        Ok(qty) if qty > 0 => Ok(Some(qty)),
        Ok(_) => Err("发货数量必须大于0".to_string()),
        Err(_) => Err("发货数量格式错误，必须为正整数".to_string()),
    }
}

/// 按商品名/规格匹配订单明细（Java `matchOrderItem`）。
fn match_order_item<'a>(
    items: &'a [OrderItem],
    product_name: Option<&str>,
    sku_name: Option<&str>,
) -> Option<&'a OrderItem> {
    let name = product_name?.trim();
    if name.is_empty() {
        return None;
    }
    let sku = sku_name
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "-");
    for it in items {
        if it.product_name.trim() != name {
            continue;
        }
        let item_sku = item_sku_value(it);
        match sku {
            Some(want) => {
                if item_sku.as_deref() == Some(want) {
                    return Some(it);
                }
            }
            None => {
                if item_sku
                    .as_deref()
                    .is_none_or(|v| v.trim().is_empty() || v == "-")
                {
                    return Some(it);
                }
            }
        }
    }
    items.iter().find(|it| it.product_name.trim() == name)
}

/// 构造批量发货导入失败行（Java `OrderDeliverImportErrorExcelVO`）。
#[allow(clippy::too_many_arguments)]
fn build_import_error_row(
    row_num: i32,
    order_sn: &str,
    row: &OrderDeliverImportRow,
    shipping_method: &str,
    logistics_name: &str,
    tracking_no: &str,
    error_msg: &str,
) -> OrderDeliverImportErrorVO {
    OrderDeliverImportErrorVO {
        row_num,
        order_sn: order_sn.to_string(),
        product_name: row.product_name.clone(),
        sku_name: row.sku_name.clone(),
        delivery_quantity: row.delivery_quantity.clone(),
        shipping_method: (!shipping_method.trim().is_empty()).then(|| shipping_method.to_string()),
        logistics_name: (!logistics_name.trim().is_empty()).then(|| logistics_name.to_string()),
        tracking_no: (!tracking_no.trim().is_empty()).then(|| tracking_no.to_string()),
        error_msg: error_msg.to_string(),
    }
}

/// 把订单级错误复制到该订单的每一行（`getBatchDeliverImportErrorData`）。
#[allow(clippy::too_many_arguments)]
fn push_all_import_errors(
    errors: &mut Vec<OrderDeliverImportErrorVO>,
    order_sn: &str,
    row_list: &[(i32, OrderDeliverImportRow)],
    shipping_method: &str,
    logistics_name: &str,
    tracking_no: &str,
    msg: &str,
) {
    for (row_num, row) in row_list {
        errors.push(build_import_error_row(
            *row_num,
            order_sn,
            row,
            shipping_method,
            logistics_name,
            tracking_no,
            msg,
        ));
    }
}

/// 把「(行号, 原因)」列表追加到失败明细。
fn push_fail(
    out: &mut Vec<OrderDeliverImportFailVO>,
    rows: &[(i32, String)],
    order_sn: &str,
) {
    for (row_num, reason) in rows {
        out.push(OrderDeliverImportFailVO {
            row_num: *row_num,
            order_sn: order_sn.to_string(),
            reason: reason.clone(),
        });
    }
}

/// 售后单号：`yyyyMMddHHmmss` + 5 位数字（Java
/// `LocalDateTime.now().format("yyyyMMddHHmmss") + RandomUtil.randomNumbers(5)`）。
///
/// Rust 侧未引入 \"rand\" 依赖，后缀由「毫秒时间戳 + 订单号」派生，保证同一订单
/// 重复申请时不会生成相同单号。
fn aftersales_sn(order_id: i32, now_secs: i32) -> String {
    use chrono::TimeZone;
    let ts = chrono::Local
        .timestamp_opt(i64::from(now_secs), 0)
        .single()
        .map(|dt| dt.format("%Y%m%d%H%M%S").to_string())
        .unwrap_or_default();
    let suffix = (utils::now_millis() + i64::from(order_id)).rem_euclid(100_000);
    format!("{ts}{suffix:05}")
}

/// 去重后的 i32 列表（保持首次出现顺序）。
fn distinct_i32<I: Iterator<Item = i32>>(it: I) -> Vec<i32> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for v in it {
        if seen.insert(v) {
            out.push(v);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn item(item_id: i32, quantity: i32) -> OrderItem {
        OrderItem {
            item_id,
            order_id: 1,
            quantity,
            ..Default::default()
        }
    }

    fn order(order_status: i32, pay_status: i32) -> Order {
        Order {
            order_id: 1,
            order_sn: "SN1".into(),
            user_id: 2,
            order_status,
            pay_status,
            shipping_status: 0,
            add_time: 1_700_000_000,
            is_valid: 1,
            shop_id: 0,
            order_type: Some(order_type::NORMAL),
            total_amount: Decimal::from_str("100.00").unwrap(),
            ..Default::default()
        }
    }

    // ————————————————— 发货数量计算 —————————————————

    #[test]
    fn deliver_plan_full_delivery_has_no_remain() {
        let items = vec![item(1, 2), item(2, 3)];
        let mut map = HashMap::new();
        map.insert(1, 2);
        map.insert(2, 3);
        let plan = compute_deliver_plan(&items, &map, &HashMap::new()).unwrap();
        assert_eq!(plan.deliver, vec![(1, 2), (2, 3)]);
        assert!(plan.remain.is_empty());
    }

    #[test]
    fn deliver_plan_partial_reports_remain() {
        let items = vec![item(1, 5)];
        let mut map = HashMap::new();
        map.insert(1, 2);
        let plan = compute_deliver_plan(&items, &map, &HashMap::new()).unwrap();
        assert_eq!(plan.deliver, vec![(1, 2)]);
        assert_eq!(plan.remain, vec![(1, 3)]);
    }

    #[test]
    fn deliver_plan_rejects_bad_quantity() {
        let items = vec![item(1, 2)];
        let mut map = HashMap::new();
        map.insert(1, 3); // 超过购买数量
        assert!(compute_deliver_plan(&items, &map, &HashMap::new()).is_err());
        map.insert(1, 0); // 非正数
        assert!(compute_deliver_plan(&items, &map, &HashMap::new()).is_err());
    }

    #[test]
    fn deliver_plan_merges_aftersales_quantity() {
        // 购买 5 件，售后占用 2 件，本次发 3 件 → 发货单并入售后 2 件 = 5，无剩余
        let items = vec![item(1, 5)];
        let mut map = HashMap::new();
        map.insert(1, 3);
        let mut af = HashMap::new();
        af.insert(1, 2);
        let plan = compute_deliver_plan(&items, &map, &af).unwrap();
        assert_eq!(plan.deliver, vec![(1, 5)]);
        assert!(plan.remain.is_empty());

        // 发 1 件 + 售后 2 件 = 3 < 5 → 剩余 2
        map.insert(1, 1);
        let plan = compute_deliver_plan(&items, &map, &af).unwrap();
        assert_eq!(plan.deliver, vec![(1, 3)]);
        assert_eq!(plan.remain, vec![(1, 2)]);
    }

    #[test]
    fn deliver_plan_rejects_quantity_over_available_with_aftersales() {
        let items = vec![item(1, 4)];
        let mut map = HashMap::new();
        map.insert(1, 3); // 3 + 售后2 = 5 > 4
        let mut af = HashMap::new();
        af.insert(1, 2);
        let err = compute_deliver_plan(&items, &map, &af).unwrap_err();
        assert!(err.to_string().contains("发货数量超出可发数量"));
    }

    #[test]
    fn deliver_plan_treats_aftersales_only_items_as_deliverable() {
        // 未指定发货数量但存在售后占用 → 按占用数量发货，剩余保留
        let items = vec![item(1, 4)];
        let mut af = HashMap::new();
        af.insert(1, 1);
        let plan = compute_deliver_plan(&items, &HashMap::new(), &af).unwrap();
        assert_eq!(plan.deliver, vec![(1, 1)]);
        assert_eq!(plan.remain, vec![(1, 3)]);
    }

    // ————————————————— 可操作项 —————————————————

    #[test]
    fn available_actions_follow_java_status_matrix() {
        // 待支付：可取消/改价/设置已支付
        let a = available_actions(&order(0, 0), false, true);
        assert!(a.cancel_order && a.set_confirm && a.to_pay && a.modify_order_money);
        assert!(!a.deliver && !a.confirm_receipt);
        // 后台用户对非线下支付订单不下发「设置已支付」入口（pay_type_id 默认 0）
        assert!(!a.set_paid);
        let mut offline = order(0, 0);
        offline.pay_type_id = pay_type::OFFLINE;
        assert!(available_actions(&offline, false, true).set_paid);

        // 已确认待发货 + 已支付：\"deliver\" 与 \"split_order\" **互斥**。
        // Java `OrderServiceImpl:1082-1086`：
        //   if (isStoreSplited == 1) setSplitOrder(false); else setDeliver(false);
        // 默认 `is_store_splited == 0` → 关闭发货入口、保留拆单入口。
        let confirmed = order(1, 2);
        let a = available_actions(&confirmed, false, true);
        assert!(a.split_order && !a.deliver);
        assert!(!a.cancel_order);
        assert!(!a.set_unpaid);

        // `is_store_splited == 1`（已拆单）：关闭拆单入口、保留发货入口
        let mut splited = order(1, 2);
        splited.is_store_splited = 1;
        let a = available_actions(&splited, false, true);
        assert!(a.deliver && !a.split_order);

        // 已确认待发货 + 未支付：可取消 + 可设置已支付（该分支不受线下支付限制），
        // 同样受 \"is_store_splited\" 互斥规则约束（默认 0 → 保留拆单、关闭发货）
        let unpaid = order(1, 0);
        let a = available_actions(&unpaid, false, true);
        assert!(a.split_order && !a.deliver && a.cancel_order && a.set_paid);

        // 处理中：可确认收货；整单售后进行中则不可
        let a = available_actions(&order(2, 2), false, true);
        assert!(a.confirm_receipt);
        let a = available_actions(&order(2, 2), true, true);
        assert!(!a.confirm_receipt);

        // 已完成：可再次购买（无在售商品时关闭）
        let a = available_actions(&order(5, 2), false, true);
        assert!(a.rebuy);
        let a = available_actions(&order(5, 2), false, false);
        assert!(!a.rebuy);

        // 已取消 / 无效
        let a = available_actions(&order(3, 0), false, true);
        assert!(!a.cancel_order && a.rebuy && a.del_order);
        let a = available_actions(&order(4, 0), false, true);
        assert!(a.del_order && !a.rebuy);
    }

    #[test]
    fn available_actions_respects_split_flags() {
        // Java `OrderServiceImpl:1082-1086` 是**互斥**的 kill-switch：
        //   if (isStoreSplited == 1) setSplitOrder(false); else setDeliver(false);
        // 因此 `is_store_splited == 1` → 关闭拆单入口，\"deliver\" 保留；
        // `is_store_splited == 0`  → 关闭发货入口，\"split_order\" 保留。
        // ⚠️ 早先本测试把两个分支写反了（与服务实现一致地反），此处按 Java 修正。
        let mut o = order(1, 2);
        o.is_store_splited = 1;
        let a = available_actions(&o, false, true);
        assert!(!a.split_order, "is_store_splited=1 必须关闭拆单入口");
        assert!(a.deliver, "is_store_splited=1 时发货入口应保留");

        // is_store_splited = 0 → 关闭发货，保留拆分
        let o = order(1, 2);
        let a = available_actions(&o, false, true);
        assert!(a.split_order, "is_store_splited=0 时拆单入口应保留");
        assert!(!a.deliver, "is_store_splited=0 必须关闭发货入口");

        // 未支付拆单子单：全只读
        let mut o = order(1, 0);
        o.parent_order_id = Some(9);
        let a = available_actions(&o, false, true);
        let json = serde_json::to_value(&a).unwrap();
        assert!(json.as_object().unwrap().values().all(|v| v == &serde_json::json!(false)));

        // 卡密订单（type 8）不再下发售后入口
        let mut o = order(5, 2);
        o.order_type = Some(8);
        let a = available_actions(&o, false, true);
        assert!(!a.to_aftersales && !a.rebuy);
    }

    // ————————————————— 派生字段 —————————————————

    #[test]
    fn status_name_switches_by_order_kind() {
        let mut o = order(2, 2);
        assert_eq!(order_status_name(&o), "处理中，已发货");

        o.order_type = Some(order_type::RESERVATION_STORE);
        assert_eq!(order_status_name(&o), "预约成功");

        o.order_type = Some(order_type::NORMAL);
        o.shipping_type_id = 3; // 同城配送
        assert_eq!(order_status_name(&o), "配送中");
    }

    #[test]
    fn fmt_time_handles_zero_and_positive() {
        assert_eq!(fmt_time(0), "");
        assert_eq!(fmt_time(-1), "");
        assert_eq!(fmt_time(1_700_000_000).len(), 19);
    }

    #[test]
    fn parse_lists_tolerate_bad_json() {
        assert_eq!(parse_i32_list("[1,2,3]"), vec![1, 2, 3]);
        assert!(parse_i32_list("").is_empty());
        assert!(parse_i32_list("not json").is_empty());
        assert_eq!(parse_string_list(r#"["A","B"]"#), vec!["A", "B"]);
        assert!(parse_json("").is_none());
    }

    #[test]
    fn address_data_fills_region_name() {
        let v = parse_address_data(Some(r#"{"regionNames":["浙江省","杭州市"],"regionName":""}"#))
            .unwrap();
        assert_eq!(v["regionName"], "浙江省 杭州市");

        // 已有 regionName 时不覆盖
        let v = parse_address_data(Some(r#"{"regionNames":["浙江省"],"regionName":"X"}"#)).unwrap();
        assert_eq!(v["regionName"], "X");

        assert!(parse_address_data(None).is_none());
    }

    #[test]
    fn shop_status_text_matches_shop_type() {
        assert_eq!(shop_status_text(Some(1)), "开业");
        assert_eq!(shop_status_text(Some(10)), "打烊");
        assert_eq!(shop_status_text(None), "");
    }

    #[test]
    fn aftersales_sn_is_unique_per_call() {
        let a = aftersales_sn(1, 1_700_000_000);
        let b = aftersales_sn(2, 1_700_000_000);
        assert_eq!(a.len(), 19); // 14 时间 + 5 数字
        assert_ne!(a, b);
    }

    #[test]
    fn build_vo_derives_user_address_and_pay_end_time() {
        let mut o = order(0, 0);
        o.region_names = r#"["浙江省","杭州市"]"#.into();
        o.address = "文一西路 1 号".into();
        o.mobile = "13812348000".into();
        let vo = build_vo(VoInput {
            order: &o,
            items: &[],
            user: None,
            shop: None,
            full_aftersales: false,
            rebuy_available: true,
            groupon: None,
            way_bill: true,
            now: 1_700_000_000,
        });
        assert_eq!(vo.user_address.as_deref(), Some("浙江省 杭州市 文一西路 1 号"));
        // 待支付：支付截止时间 = add_time + 30 分钟
        assert!(vo.pay_end_time.is_some());
        assert_eq!(vo.pay_status_name, "未支付");
        assert_eq!(vo.order_status_tip, None); // tip 仅详情接口下发
        assert!(vo.way_bill);
        assert!(vo.available_actions.to_pay);
    }

    #[test]
    fn build_vo_applies_groupon_overrides() {
        let mut o = order(1, 2);
        o.order_type = Some(order_type::PIN);
        // is_store_splited = 1 时发货入口默认保留，才能验证拼团把 deliver 关掉
        o.is_store_splited = 1;
        let vo = build_vo(VoInput {
            order: &o,
            items: &[],
            user: None,
            shop: None,
            full_aftersales: false,
            rebuy_available: true,
            groupon: Some((77, join_status::WAIT_GROUP)),
            way_bill: false,
            now: 1_700_000_000,
        });
        assert_eq!(vo.order_status_name, "已付款，待成团");
        assert_eq!(vo.groupon_promotion_record_id, Some(77));
        assert_eq!(vo.join_status, Some(join_status::WAIT_GROUP));
        assert_eq!(vo.join_status_desc.as_deref(), Some("待成团"));
        // 待成团不允许发货
        assert!(!vo.available_actions.deliver);
        assert!(!vo.way_bill);
    }

    #[test]
    fn user_and_shop_vo_mapping() {
        let u = User {
            user_id: 5,
            username: "u5".into(),
            nickname: Some("昵称".into()),
            avatar: "a.png".into(),
            mobile: "13800000000".into(),
            email: "u@example.com".into(),
            ..Default::default()
        };
        let vo = to_user_vo(&u);
        assert_eq!(vo.user_id, 5);
        assert_eq!(vo.nickname.as_deref(), Some("昵称"));

        let s = Shop {
            shop_id: 3,
            shop_title: Some("旗舰店".into()),
            status: Some(1),
            shop_logo: Some("l.png".into()),
            kefu_inlet: Some("kf".into()),
            ..Default::default()
        };
        let vo = to_shop_vo(&s);
        assert_eq!(vo.shop_title, "旗舰店");
        assert_eq!(vo.status_text, "开业");
    }

    // ————————————————— wave3 新增纯函数 —————————————————

    #[test]
    fn export_columns_match_java_map_order() {
        let cols = export_columns();
        assert_eq!(cols.len(), 30);
        assert_eq!(cols[0].key, "orderSn");
        assert_eq!(cols[0].label, "订单编号");
        assert_eq!(cols[26].key, "productInfo");
        assert_eq!(cols[29].key, "usePoints");
        assert_eq!(cols[29].label, "使用积分");
    }

    #[test]
    fn expand_export_columns_handles_product_info_and_pay_type() {
        let cols = expand_export_columns(&[
            "orderSn".to_string(),
            "productInfo".to_string(),
            "payTypeId".to_string(),
        ])
        .unwrap();
        let keys: Vec<&str> = cols.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "orderSn",
                "productName",
                "skuData",
                "productSn",
                "quantity",
                "price",
                "totalPrice",
                "payTypeId",
                "payTypeName"
            ]
        );
        // 未知导出项报错
        assert!(expand_export_columns(&["notAKey".to_string()]).is_err());
        // 重复 key 去重
        let cols = expand_export_columns(&["orderSn".to_string(), "orderSn".to_string()]).unwrap();
        assert_eq!(cols.len(), 1);
    }

    #[test]
    fn allocate_discount_distributes_and_keeps_positive_prices() {
        let a = OrderItem {
            item_id: 1,
            price: Decimal::new(1000, 2), // 10.00
            quantity: 2,
            ..Default::default()
        };
        let b = OrderItem {
            item_id: 2,
            price: Decimal::new(500, 2), // 5.00
            quantity: 1,
            ..Default::default()
        };
        let updates = allocate_discount(&[a, b], Decimal::new(300, 2)).unwrap();
        assert_eq!(updates.len(), 2);
        // 小计 20 : 5，折扣 3 元 → 行折扣 2.40 / 0.60，单价降 1.20 / 0.60
        assert_eq!(updates[0].0, 1);
        assert_eq!(updates[0].1, Decimal::new(880, 2)); // 10.00 - 1.20
        assert_eq!(updates[0].2, Some(Decimal::new(1000, 2)));
        assert_eq!(updates[1].1, Decimal::new(440, 2)); // 5.00 - 0.60
        // 空明细不报错
        assert!(allocate_discount(&[], Decimal::ONE).unwrap().is_empty());
        // 无小计（全部 0 元）→ 报错
        let zero = OrderItem {
            item_id: 3,
            price: Decimal::ZERO,
            quantity: 1,
            ..Default::default()
        };
        assert!(allocate_discount(&[zero], Decimal::ONE).is_err());
    }

    #[test]
    fn merge_address_data_keeps_snapshot_extras() {
        let old = Some(r#"{"addressTag":"家","userId":5,"addressId":9,"isDefault":1}"#);
        let param = OrderModifyConsigneeParam {
            address: Some("文一西路 1 号".into()),
            mobile: Some("13800000000".into()),
            postcode: Some("310000".into()),
            ..Default::default()
        };
        let raw = merge_address_data(old, &param, &[110000, 110100], &["北京市".to_string(), "东城区".to_string()])
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["address"], "文一西路 1 号");
        assert_eq!(v["mobile"], "13800000000");
        assert_eq!(v["postcode"], "310000");
        assert_eq!(v["regionIds"], serde_json::json!([110000, 110100]));
        assert_eq!(v["regionNames"], serde_json::json!(["北京市", "东城区"]));
        // 原快照附加字段保留
        assert_eq!(v["addressTag"], "家");
        assert_eq!(v["addressId"], 9);
        assert_eq!(v["isDefault"], 1);
        // 旧值非法 JSON 时也能生成新快照
        let raw = merge_address_data(Some("oops"), &param, &[], &[]).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&raw).is_ok());
    }

    #[test]
    fn import_rows_parse_from_csv_and_json() {
        let csv = "说明行\n订单编号,会员名称,收货人,手机号,收货地址,订单金额,商品名称,商品规格,商品数量,*发货数量,配送方式,物流公司,物流代号,物流单号,下单时间\n\
                   SN1,张三,张三,13800000000,北京市,100.00,衬衫,颜色:红,2,2,物流配送,顺丰,SF,SF123,2026-01-01 10:00:00\n";
        let rows = parse_deliver_import_rows(csv.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_sn.as_deref(), Some("SN1"));
        assert_eq!(rows[0].product_name.as_deref(), Some("衬衫"));
        assert_eq!(rows[0].sku_name.as_deref(), Some("颜色:红"));
        assert_eq!(rows[0].delivery_quantity.as_deref(), Some("2"));
        assert_eq!(rows[0].shipping_method.as_deref(), Some("物流配送"));
        assert_eq!(rows[0].logistics_name.as_deref(), Some("顺丰"));
        assert_eq!(rows[0].tracking_no.as_deref(), Some("SF123"));
        assert_eq!(rows[0].row_num, Some(3));

        let json = r#"[{"orderSn":"SN2","productName":"裤子","deliveryQuantity":"1"}]"#;
        let rows = parse_deliver_import_rows(json.as_bytes()).unwrap();
        assert_eq!(rows[0].row_num, Some(3), "JSON 载体缺省行号按 index+3 补全");
        assert!(parse_deliver_import_rows(b"").is_err());
    }

    #[test]
    fn import_validation_matches_java_messages() {
        let row = OrderDeliverImportRow {
            order_sn: Some("SN1".into()),
            shipping_method: Some("物流配送".into()),
            ..Default::default()
        };
        assert_eq!(parse_shipping_method_name("物流配送"), Some(shipping_method::LOGISTICS));
        assert_eq!(parse_shipping_method_name("商家配送"), Some(shipping_method::MERCHANT));
        assert_eq!(parse_shipping_method_name("无需配送"), Some(shipping_method::NONE));
        assert_eq!(parse_shipping_method_name("电子面单"), Some(shipping_method::WAYBILL));
        assert_eq!(parse_shipping_method_name("x"), None);
        // 物流配送缺物流公司/单号
        assert_eq!(
            validate_shipping_method_fields(&row, shipping_method::LOGISTICS).as_deref(),
            Some("配送方式为物流配送时，物流公司必填")
        );
        let row2 = OrderDeliverImportRow {
            logistics_name: Some("顺丰".into()),
            ..row.clone()
        };
        assert_eq!(
            validate_shipping_method_fields(&row2, shipping_method::LOGISTICS).as_deref(),
            Some("配送方式为物流配送时，物流单号必填")
        );
        let row3 = OrderDeliverImportRow {
            logistics_name: Some("顺丰".into()),
            tracking_no: Some("SF1".into()),
            ..row.clone()
        };
        assert!(validate_shipping_method_fields(&row3, shipping_method::LOGISTICS).is_none());
        // 基础校验
        let blank = OrderDeliverImportRow::default();
        assert_eq!(validate_deliver_excel_data(&blank).as_deref(), Some("订单编号不能为空"));
        assert_eq!(
            validate_deliver_excel_data(&row).as_deref(),
            None
        );
        // 发货数量
        let q = OrderDeliverImportRow { delivery_quantity: Some("3".into()), ..Default::default() };
        assert_eq!(parse_delivery_quantity(&q), Ok(Some(3)));
        let q = OrderDeliverImportRow { delivery_quantity: Some("0".into()), ..Default::default() };
        assert_eq!(parse_delivery_quantity(&q), Err("发货数量必须大于0".to_string()));
        let q = OrderDeliverImportRow { delivery_quantity: Some("x".into()), ..Default::default() };
        assert_eq!(
            parse_delivery_quantity(&q),
            Err("发货数量格式错误，必须为正整数".to_string())
        );
        // 分组：同订单号聚合、保留行号
        let rows = vec![
            OrderDeliverImportRow { order_sn: Some("A".into()), row_num: Some(3), ..Default::default() },
            OrderDeliverImportRow { order_sn: Some("B".into()), row_num: Some(4), ..Default::default() },
            OrderDeliverImportRow { order_sn: Some("A".into()), row_num: Some(5), ..Default::default() },
        ];
        let groups = group_import_rows(&rows);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "A");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[0].1[1].0, 5);
        assert_eq!(groups[1].0, "B");
    }

    #[test]
    fn match_order_item_prefers_exact_sku_then_name() {
        let items = vec![
            OrderItem {
                item_id: 1,
                product_id: 7,
                product_name: "衬衫".into(),
                sku_data: r#"[{"name":"颜色","value":"红"}]"#.into(),
                ..Default::default()
            },
            OrderItem {
                item_id: 2,
                product_id: 8,
                product_name: "裤子".into(),
                sku_data: "[]".into(),
                ..Default::default()
            },
        ];
        // 精确规格命中
        let hit = match_order_item(&items, Some("衬衫"), Some("颜色:红")).unwrap();
        assert_eq!(hit.item_id, 1);
        // 规格 "-" 视为无规格 → 命中无规格商品
        let hit = match_order_item(&items, Some("裤子"), Some("-")).unwrap();
        assert_eq!(hit.item_id, 2);
        // 规格不匹配 → 回退按商品名命中
        let hit = match_order_item(&items, Some("衬衫"), Some("颜色:蓝")).unwrap();
        assert_eq!(hit.item_id, 1);
        assert!(match_order_item(&items, Some("不存在"), None).is_none());
        assert!(match_order_item(&items, None, None).is_none());
    }

    #[test]
    fn export_cell_maps_core_columns() {
        let row = OrderExportRowVO {
            order_sn: "SN2026".into(),
            order_status_name: "待发货".into(),
            total_amount: Decimal::new(12345, 2),
            quantity: 2,
            product_name: "测试商品".into(),
            ..Default::default()
        };
        assert_eq!(export_cell("orderSn", &row), "SN2026");
        assert_eq!(export_cell("orderStatusName", &row), "待发货");
        assert_eq!(export_cell("totalAmount", &row), "123.45");
        assert_eq!(export_cell("quantity", &row), "2");
        assert_eq!(export_cell("productName", &row), "测试商品");
        assert_eq!(export_cell("unknown", &row), "");
    }
}"