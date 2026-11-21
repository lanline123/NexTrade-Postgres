//! 预约商品下单接线（我方新增，论坛 B 类帖 497863 + 377834）。
//!
//! # 背景
//!
//! 预约模板与库存日历（管理端）早已移植，但买家下单链此前未接线：
//! 结算页提交的 `productReservationTimeId` / `reservationDate` 被 DTO
//! 丢弃、`order_reservations` 全仓无写入方、`product_reservation_stock.
//! used_stock` 无人占用——预约商品只能按普通商品走商品库存下单，
//! 更无法以拼团/秒杀活动价成交。本模块补齐这条链，并明确
//! **预约库存/时段与活动库存的双口径**（见下）。
//!
//! # 双口径规则（归属与扣减）
//!
//! 1. **价格口径**：活动价（拼团阶梯价 / 秒杀价）作用于预约单，与普通
//!    商品完全一致——价格解析（`trade::check` 的拼团/秒杀价加载）不分
//!    商品类型，本模块不改价格链。
//! 2. **活动库存口径（归活动）**：秒杀库存池 `seckill_item` 在下单事务
//!    内守卫扣减（既有逻辑）；拼团无下单期活动库存池（明细 `limit_num`
//!    现行仅在后台取消时回补，本批不改动这一存量口径）。活动库存是
//!    **活动窗口内的全局上限**，跨日期/时段共享。
//! 3. **时段容量口径（归门店×商品×SKU×日期×时段）**：预约行下单必须
//!    在同一事务内守卫占用 `product_reservation_stock.used_stock`
//!    （余量 = `total_stock - used_stock`），无库存行 = 该时段未开放
//!    预约、不可下单。时段容量是**单日单时段的硬上限**。
//! 4. **不超卖铁律**：活动预约单的可售量 = min（活动库存， 时段容量）
//!    ——两类扣减与订单落库同事务，任一不足整单回滚，活动不得突破
//!    时段容量、时段容量也不得被活动放大。
//! 5. **商品库存口径**：预约接线命中的预约行**不扣**商品/SKU 库存
//!    （服务类商品的可售量由日历时段容量承载，与购物车对预约商品
//!    跳过商品库存校验的既有口径一致）；未提交预约选择的预约商品
//!    订单保持存量行为（按商品库存下单），逐字不变。
//! 6. **订单类型**：礼品卡兑换 12 > 拼团 2 > 预约（5 到店 = 9 /
//!    6 上门 = 10）> 普通 0。拼团预约单保拼团型（拼团状态机/取消链
//!    以之为键），预约信息一律落 `order_reservations` 行，与订单
//!    类型解耦（上门派送单生产者亦以预约行存在为触发条件）。
//! 7. **取消释放**：一切「取消订单 + 回滚库存」路径同口径——买家
//!    取消（`OrderService::cancel`，含超时自动取消）、后台取消
//!    （`OrderAdminService::cancel_order`）、商家取消售后自动审核
//!    （`cancel_order_by_admin`）、改状态取消（`change_order_status`）
//!    ——预约行不回补商品库存（建单未扣），改释放时段名额
//!    （`used_stock` 回减、不减为负），尽力而为失败仅告警。买家售后
//!    仅退款不取消订单的，名额不自动释放（随服务日期过期自然失效，
//!    商家可在库存日历调整），与秒杀库存售后口径一致。
//! 8. **触发条件**：时段 ID 与预约日期**成对**提交才生成预约计划；
//!    周期购/次卡预约只传时段 ID（无日期）的既有形态不触发本接线。
//!    一个订单至多一行预约商品（结算页预约选择为订单级单选），
//!    多行预约商品整单拒绝并提示单独下单。

use sqlx::{Postgres, Transaction};

use nextrade_core::entities::{OrderItem, OrderReservations};
use nextrade_core::models::marketing::groupon_promotion_item::is_reservation_product;
use nextrade_core::models::payment::pay::{
    ORDER_TYPE_RESERVATION_HOME, ORDER_TYPE_RESERVATION_STORE,
};
use nextrade_core::models::trade::order::OrderCreateDTO;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 一笔订单的预约计划（由结算提交的预约选择 + 唯一预约行解析而成）。
#[derive(Debug, Clone)]
pub struct ReservationPlan {
    /// 预约商品 ID。
    pub product_id: i32,
    /// 预约行 SKU（无规格为 0）。
    pub sku_id: i32,
    /// 预约行门店（时段库存归属门店）。
    pub shop_id: i32,
    /// 预约行数量（占用名额数）。
    pub quantity: i32,
    /// 预约时段 ID（`product_reservation_time` 主键）。
    pub time_id: i32,
    /// 预约日期 `yyyy-MM-dd`。
    pub date: String,
    /// 时段开始 `HH:mm`（落 `order_reservations` 快照）。
    pub start_time: String,
    /// 时段结束 `HH:mm`。
    pub end_time: String,
    /// 订单类型：5 到店 = 9 / 6 上门 = 10。
    pub order_type: i32,
}

impl ReservationPlan {
    /// 是否为给定订单行（建单扣减跳过与取消释放共用同一判定）。
    #[must_use]
    pub fn matches(&self, item: &OrderItem) -> bool {
        item.is_gift != 1 && item.product_id == self.product_id && item.sku_id == self.sku_id
    }
}

/// 商品类型 → 预约订单类型（5 到店 = 9、6 上门 = 10，其余 0）。
#[must_use]
pub fn reservation_order_type(product_type: i32) -> i32 {
    match product_type {
        5 => ORDER_TYPE_RESERVATION_STORE,
        6 => ORDER_TYPE_RESERVATION_HOME,
        _ => 0,
    }
}

/// 时段描述（落 `order_reservations.time_slot_description`，与小票
/// 打印的回退拼接 `startTime-endTime` 同形）。
#[must_use]
pub fn slot_description(start_time: &str, end_time: &str) -> String {
    format!("{start_time}-{end_time}")
}

/// 预约日期合法性：`yyyy-MM-dd` 且不早于今天（纯函数，供单测）。
#[must_use]
pub fn is_bookable_date(raw: &str, today: chrono::NaiveDate) -> bool {
    chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
        .is_ok_and(|d| d >= today)
}

/// 解析预约计划：未成对提交预约选择、或单内无预约商品 → `None`
/// （存量行为逐字不变）；提交了选择但校验不过 → 报错整单拒绝。
///
/// # Errors
/// 预约行不唯一、日期非法/已过、时段不存在或与商品不匹配、数据库错误。
pub async fn build_plan(
    repos: &Repositories,
    dto: &OrderCreateDTO,
    items: &[OrderItem],
) -> Result<Option<ReservationPlan>> {
    let (Some(time_id), Some(date_raw)) = (
        dto.product_reservation_time_id.filter(|id| *id > 0),
        dto.reservation_date
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    ) else {
        return Ok(None);
    };
    let reservation_lines: Vec<&OrderItem> = items
        .iter()
        .filter(|it| it.is_gift != 1 && is_reservation_product(Some(it.product_type)))
        .collect();
    if reservation_lines.is_empty() {
        return Ok(None);
    }
    if reservation_lines.len() > 1 {
        return Err(Error::biz(
            1001,
            "预约商品请单独下单（一个订单只能预约一件服务商品）".to_string(),
        ));
    }
    let line = reservation_lines[0];
    let today = chrono::Local::now().date_naive();
    if !is_bookable_date(date_raw, today) {
        return Err(Error::biz(
            1001,
            "预约日期不正确或已过期，请重新选择".to_string(),
        ));
    }
    let date = chrono::NaiveDate::parse_from_str(date_raw, "%Y-%m-%d")
        .map(|d| d.format("%Y-%m-%d").to_string())
        .map_err(|_| Error::Validation("预约日期格式不正确".into()))?;
    let time = repos
        .o2o
        .stock_calendar
        .find_time_by_id(time_id)
        .await?
        .ok_or_else(|| Error::biz(1001, "预约时段不存在，请重新选择".to_string()))?;
    if time.product_id != line.product_id {
        return Err(Error::biz(
            1001,
            "预约时段与商品不匹配，请重新选择".to_string(),
        ));
    }
    Ok(Some(ReservationPlan {
        product_id: line.product_id,
        sku_id: line.sku_id,
        shop_id: line.shop_id,
        quantity: line.quantity.max(1),
        time_id,
        date,
        start_time: time.start_time,
        end_time: time.end_time,
        order_type: reservation_order_type(line.product_type),
    }))
}

/// 下单事务内执行预约计划：守卫占用时段名额 + 写 `order_reservations`
/// 行。任一步失败由调用方回滚整单（含已扣的活动/商品库存）。
///
/// # Errors
/// 时段余量不足（或该时段未开放预约）、数据库错误。
pub async fn reserve_in_tx(
    repos: &Repositories,
    tx: &mut Transaction<'_, Postgres>,
    plan: &ReservationPlan,
    order_id: i32,
) -> Result<()> {
    let occupied = repos
        .o2o
        .stock_calendar
        .inc_used_stock_in_tx(
            tx,
            plan.shop_id,
            plan.product_id,
            plan.sku_id,
            &plan.date,
            plan.time_id,
            plan.quantity,
        )
        .await?;
    if !occupied {
        return Err(Error::biz(
            1001,
            "预约时段名额不足或该时段未开放预约，请更换日期或时段".to_string(),
        ));
    }
    let row = OrderReservations {
        order_reservations_id: 0,
        order_id,
        product_reservation_time_id: Some(plan.time_id),
        sku_id: plan.sku_id,
        reservation_date: plan.date.clone(),
        start_time: plan.start_time.clone(),
        end_time: plan.end_time.clone(),
        time_slot_description: slot_description(&plan.start_time, &plan.end_time),
        add_time: nextrade_common::utils::now_seconds() as i32,
    };
    repos.payment.pay.insert_reservation_in_tx(tx, &row).await?;
    Ok(())
}

/// 取消订单时释放预约名额：按订单的 `order_reservations` 行定位
/// 预约行（商品类型 5/6 且 SKU 与预约行一致），回减时段 `used_stock`。
/// 无预约行 → 返回 `false` 空转（普通订单零影响）。
///
/// # Errors
/// 数据库错误（调用方按尽力而为处理、仅告警）。
pub async fn release_for_order(
    repos: &Repositories,
    order_id: i32,
    items: &[OrderItem],
) -> Result<bool> {
    let Some(row) = repos
        .payment
        .pay
        .find_reservation_by_order_id(order_id)
        .await?
    else {
        return Ok(false);
    };
    let Some(item) = items.iter().find(|it| {
        it.is_gift != 1
            && is_reservation_product(Some(it.product_type))
            && it.sku_id == row.sku_id
    }) else {
        return Ok(false);
    };
    let Some(time_id) = row.product_reservation_time_id.filter(|id| *id > 0) else {
        return Ok(false);
    };
    repos
        .o2o
        .stock_calendar
        .dec_used_stock(
            item.shop_id,
            item.product_id,
            row.sku_id,
            &row.reservation_date,
            time_id,
            item.quantity.max(1),
        )
        .await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_type_maps_reservation_product_types() {
        assert_eq!(reservation_order_type(5), 9);
        assert_eq!(reservation_order_type(6), 10);
        assert_eq!(reservation_order_type(1), 0);
    }

    #[test]
    fn bookable_date_rejects_past_and_garbage() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 21).unwrap();
        assert!(is_bookable_date("2026-11-21", today));
        assert!(is_bookable_date("2026-12-01", today));
        assert!(!is_bookable_date("2026-11-20", today));
        assert!(!is_bookable_date("21/11/2026", today));
        assert!(!is_bookable_date("", today));
    }

    #[test]
    fn slot_description_joins_with_dash() {
        assert_eq!(slot_description("09:00", "10:00"), "09:00-10:00");
    }
}
