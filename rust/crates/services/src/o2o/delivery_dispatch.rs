//! 派送单生产者（骑手 MVP 阶段一激活，我方新增接线层；地基表为原版有）。
//!
//! 背景：原版 5.8.57+ 的 `delivery_order` / `service_staff` 模型已全量移植，
//! 但全仓此前**没有任何一处** `INSERT INTO delivery_order`——同城订单付款后
//! 止步订单域、线下履约、买家不可见（与分销佣金生成链缺失同形）。本服务在
//! 订单付款成功点（与 `SalesmanGenerationService::on_order_paid` 同三处接线、
//! 同 best-effort 语义）补上生产者：
//!
//! - type=1 同城配送：`order.shipping_type_id == 3`（SAME_CITY）+ 店铺
//!   `shop_local_delivery_config.enabled = 1` + 平台总控
//!   `o2oIntraCityDeliveryConfig.intraCitySelfDeliveryEnabled = 1`
//!   （仅自建车队 carrier_type=1；美团等三方仅有配置壳、无 API 对接，不生成）；
//! - type=2 上门服务：订单存在 `order_reservations` 预约行 + 店铺
//!   `shop_home_service_config.enabled = 1`；service_time 取预约日期+开始时间。
//!   ⚠️ 当前预约下单链本身尚未移植（order_reservations 全仓无写入方），
//!   本分支在该链落地前无自然触发源，属前向接线（验收以构造数据验证）。
//!
//! 幂等：同订单同类型已有未删除派送单 → 跳过（支付回调可重复投递）。
//! 发单模式取店铺配置 `order_dispatch_mode`（1=抢单 2=手动派单），生成即
//! status=10 待派发；手动模式同样落 10，由商家在派送单列表指派（指派后 20）。
//! 主订单状态联动：**不**推进主订单 shipping 状态——原版口径即如此
//! （`order_admin.rs` 对 SAME_CITY 故意不推进，见该处注释），派送进度由
//! delivery_order 独立承载、买家侧经派送进度接口呈现。

use std::sync::Arc;

use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 平台同城总控配置（`config` 表 JSON 码 `o2oIntraCityDeliveryConfig`）。
#[derive(Debug, Clone, Copy)]
struct IntraCitySwitches {
    enabled: bool,
    self_delivery: bool,
}

impl IntraCitySwitches {
    /// 与 lowfreq h_12 的缺省口径一致：总开关/自配送缺省开，三方缺省关。
    fn parse(raw: Option<&str>) -> Self {
        let v: serde_json::Value = raw
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Null);
        let flag = |key: &str, default: bool| -> bool {
            match v.get(key) {
                Some(serde_json::Value::Bool(b)) => *b,
                Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
                _ => default,
            }
        };
        Self {
            enabled: flag("intraCityDeliveryEnabled", true),
            self_delivery: flag("intraCitySelfDeliveryEnabled", true),
        }
    }
}

/// 派送单生产者服务（挂载点：订单付款成功三处，与分销生成链同位）。
pub struct DeliveryDispatchService {
    repos: Arc<Repositories>,
}

impl DeliveryDispatchService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 订单付款成功钩子：按订单形态生成派送单（无适用形态时静默跳过）。
    ///
    /// 调用方按 best-effort 语义处理错误（记 warn 不回滚支付），与分销生成链一致。
    pub async fn on_order_paid(&self, order_id: i32) -> Result<()> {
        let Some(order) = self.repos.trade.order.find_by_id(order_id).await? else {
            return Ok(());
        };
        if order.shop_id <= 0 {
            return Ok(());
        }
        // type=2 上门服务优先判定（预约单与同城互斥：预约单 shipping 非 SAME_CITY）。
        if order.shipping_type_id != 3 {
            if let Some(res) = self
                .repos
                .payment
                .pay
                .find_reservation_by_order_id(order_id)
                .await?
            {
                return self.create_service_order(&order, &res).await;
            }
            return Ok(());
        }
        self.create_delivery_order(&order).await
    }

    /// type=1：同城配送单生成。
    async fn create_delivery_order(
        &self,
        order: &nextrade_core::entities::order::Order,
    ) -> Result<()> {
        if self
            .repos
            .o2o
            .pickup_client
            .find_delivery_by_order(order.order_id, 1)
            .await?
            .is_some()
        {
            return Ok(()); // 幂等：已生成
        }
        let Some(cfg) = self
            .repos
            .o2o
            .config
            .find_local_delivery(order.shop_id)
            .await?
        else {
            return Ok(());
        };
        if cfg.enabled != 1 {
            return Ok(());
        }
        let switches = IntraCitySwitches::parse(
            self.repos
                .system
                .config_basic
                .find_by_code("o2oIntraCityDeliveryConfig")
                .await?
                .as_deref(),
        );
        if !switches.enabled || !switches.self_delivery {
            return Ok(());
        }
        let shop = self.repos.merchant.shop_client.find_shop(order.shop_id).await?;
        let (pickup_address, pickup_phone, pickup_lat, pickup_lng) = match &shop {
            Some(s) => (
                s.shop_detailed_address.clone().unwrap_or_default(),
                s.contact_mobile
                    .clone()
                    .filter(|p| !p.is_empty())
                    .or_else(|| s.kefu_phone.clone())
                    .unwrap_or_default(),
                s.shop_latitude,
                s.shop_longitude,
            ),
            None => (String::new(), String::new(), None, None),
        };
        let (lat, lng) = buyer_coords(order.address_data.as_deref());
        let sn = delivery_sn(order.order_id, 1);
        self.repos
            .o2o
            .pickup_client
            .create_delivery_order(
                &sn,
                order.order_id,
                &order.order_sn,
                order.shop_id,
                1,
                cfg.order_dispatch_mode,
                &order.consignee,
                &order.mobile,
                &order.address,
                &order.region_names,
                lat,
                lng,
                &pickup_address,
                "",
                &pickup_phone,
                pickup_lat,
                pickup_lng,
                None,
                order.total_amount,
                &order.buyer_note,
            )
            .await?;
        Ok(())
    }

    /// type=2：上门服务单生成（预约行驱动；见文件头注的触发源说明）。
    async fn create_service_order(
        &self,
        order: &nextrade_core::entities::order::Order,
        res: &nextrade_core::entities::order::OrderReservations,
    ) -> Result<()> {
        if self
            .repos
            .o2o
            .pickup_client
            .find_delivery_by_order(order.order_id, 2)
            .await?
            .is_some()
        {
            return Ok(()); // 幂等：已生成
        }
        let Some(cfg) = self
            .repos
            .o2o
            .config
            .find_home_service(order.shop_id)
            .await?
        else {
            return Ok(());
        };
        if cfg.enabled != 1 {
            return Ok(());
        }
        let shop = self.repos.merchant.shop_client.find_shop(order.shop_id).await?;
        let (pickup_address, pickup_phone, pickup_lat, pickup_lng) = match &shop {
            Some(s) => (
                s.shop_detailed_address.clone().unwrap_or_default(),
                s.contact_mobile
                    .clone()
                    .filter(|p| !p.is_empty())
                    .or_else(|| s.kefu_phone.clone())
                    .unwrap_or_default(),
                s.shop_latitude,
                s.shop_longitude,
            ),
            None => (String::new(), String::new(), None, None),
        };
        let (lat, lng) = buyer_coords(order.address_data.as_deref());
        let sn = delivery_sn(order.order_id, 2);
        self.repos
            .o2o
            .pickup_client
            .create_delivery_order(
                &sn,
                order.order_id,
                &order.order_sn,
                order.shop_id,
                2,
                cfg.order_dispatch_mode,
                &order.consignee,
                &order.mobile,
                &order.address,
                &order.region_names,
                lat,
                lng,
                &pickup_address,
                "",
                &pickup_phone,
                pickup_lat,
                pickup_lng,
                service_time_of(res),
                order.total_amount,
                &order.buyer_note,
            )
            .await?;
        Ok(())
    }
}

/// 派送单号：`DO` + 主订单号派生（同订单同类型恒定，便于对账与幂等排查）。
fn delivery_sn(order_id: i32, order_type: i32) -> String {
    format!("DO{order_id}{order_type}")
}

/// 从订单地址快照 JSON 解析买家坐标（下单时写入 user_address 经纬度；
/// 缺失/非法 → None，不阻塞生成）。
fn buyer_coords(address_data: Option<&str>) -> (Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>) {
    let Some(raw) = address_data else {
        return (None, None);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (None, None);
    };
    let pick = |keys: &[&str]| -> Option<rust_decimal::Decimal> {
        for k in keys {
            match v.get(k) {
                Some(serde_json::Value::Number(n)) => {
                    // try_from 与地址写入侧 decimal_from_f64 同口径：只保留 f64
                    // 保证精度内的最短表示（28.68 → 28.68）；from_f64_retain 会
                    // 把二进制近似原样展开（28.679999999999999715782905687）落库。
                    if let Some(d) = rust_decimal::Decimal::try_from(n.as_f64()?).ok() {
                        return Some(d);
                    }
                }
                Some(serde_json::Value::String(s)) => {
                    if let Ok(d) = s.trim().parse::<rust_decimal::Decimal>() {
                        return Some(d);
                    }
                }
                _ => {}
            }
        }
        None
    };
    (
        pick(&["latitude", "lat"]),
        pick(&["longitude", "lng", "lon"]),
    )
}

/// 预约服务时间：reservation_date（YYYY-MM-DD）+ start_time（HH:MM[:SS]），
/// 按平台业务时区 UTC+8 换算 epoch 秒；解析失败 → None（不阻塞生成）。
fn service_time_of(res: &nextrade_core::entities::order::OrderReservations) -> Option<i32> {
    let date = chrono::NaiveDate::parse_from_str(res.reservation_date.trim(), "%Y-%m-%d").ok()?;
    let time_str = res.start_time.trim();
    let time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S")
        .or_else(|_| chrono::NaiveTime::parse_from_str(time_str, "%H:%M"))
        .ok()?;
    let dt = date.and_time(time);
    // UTC+8 固定偏移（平台业务时区）。
    Some((dt.and_utc().timestamp() - 8 * 3600) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_defaults_open_self_delivery() {
        let s = IntraCitySwitches::parse(None);
        assert!(s.enabled && s.self_delivery);
        let s = IntraCitySwitches::parse(Some(r#"{"intraCitySelfDeliveryEnabled":0}"#));
        assert!(s.enabled && !s.self_delivery);
        let s = IntraCitySwitches::parse(Some(r#"{"intraCityDeliveryEnabled":false}"#));
        assert!(!s.enabled);
    }

    #[test]
    fn buyer_coords_parses_number_and_string() {
        let (lat, lng) = buyer_coords(Some(r#"{"latitude":28.68,"longitude":"115.85"}"#));
        assert_eq!(lat.map(|d| d.to_string()), Some("28.68".to_string()));
        assert_eq!(lng.map(|d| d.to_string()), Some("115.85".to_string()));
        // 整数坐标：数字与字符串两种输入都不带小数尾巴。
        let (lat, lng) = buyer_coords(Some(r#"{"latitude":30,"longitude":"120"}"#));
        assert_eq!(lat.map(|d| d.to_string()), Some("30".to_string()));
        assert_eq!(lng.map(|d| d.to_string()), Some("120".to_string()));
        // 负坐标（南纬/西经）：符号保留、仍是最短表示。
        let (lat, lng) = buyer_coords(Some(r#"{"latitude":-33.86,"longitude":-151.21}"#));
        assert_eq!(lat.map(|d| d.to_string()), Some("-33.86".to_string()));
        assert_eq!(lng.map(|d| d.to_string()), Some("-151.21".to_string()));
        assert_eq!(buyer_coords(None), (None, None));
        assert_eq!(buyer_coords(Some("not-json")), (None, None));
    }

    #[test]
    fn delivery_sn_is_stable() {
        assert_eq!(delivery_sn(123, 1), "DO1231");
        assert_eq!(delivery_sn(123, 2), "DO1232");
    }
}
