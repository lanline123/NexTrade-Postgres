//! o2o 门店级配置域服务（o2o::config）。
//!
//! 对应原 Java（overseas）：
//! - `ShopHomeServiceConfigMapper`（`nextrade-service/src/main/resources/mapper/o2o/`）
//!   + `ShopLocalDeliveryConfigMapper` / `ShopPickupConfigMapper` / `ShopPickupTplMapper`
//! - overseas 工程未提供 `/adminapi/o2o/**` 控制器；端点路径以线上前端契约为准
//!   （`java/view/admin/src/api/setting/homeService.ts`、`intraCityDelivery.ts`、
//!   `pickupSetting.ts`）。
//!
//! ## 任务口径
//! - `getConfig` 读不到配置行时返回**空对象**（前端按默认值渲染），不报错；
//! - `shopId`：店铺 / 门店 / POS 账号强制自己的 `shop_id`（忽略请求值），
//!   平台账号用请求值，缺省 0（平台级配置行）。

use std::sync::Arc;

use nextrade_core::entities::o2o::{ShopHomeServiceConfig, ShopLocalDeliveryConfig};
use nextrade_core::models::o2o::config::{
    parse_json_col, DistancePricingVO, ShopConfigQuery, ShopHomeServiceConfigVO,
    ShopHomeServiceSaveParam, ShopLocalDeliveryConfigVO, ShopLocalDeliverySaveParam,
    ShopPickupTplVO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// o2o 配置域服务（挂载于 `AdminState.o2o.config`）。
pub struct O2oConfigService {
    repos: Arc<Repositories>,
}

impl O2oConfigService {
    /// 构造。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::o2o::config::O2oConfigRepository {
        &self.repos.o2o.config
    }

    /// 解析 shopId：店铺账号固定自己；平台用请求值（缺省 0 = 平台级）。
    #[must_use]
    pub fn resolve_shop_id(q: &ShopConfigQuery, shop_scope: Option<i32>) -> i32 {
        match shop_scope {
            Some(scope) => scope,
            None => q.shop_id.unwrap_or(0),
        }
    }

    /// `GET /o2o/homeService/getConfig`：预约上门服务配置（无行返回空对象语义由
    /// 路由层 `Option` 判空实现）。含距离溢价行（往返保真）。
    pub async fn home_service_config(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopHomeServiceConfigVO>> {
        let Some(c) = self.repo().find_home_service(shop_id).await? else {
            return Ok(None);
        };
        let pricing = self.repo().find_home_pricing(shop_id).await?;
        Ok(Some(ShopHomeServiceConfigVO {
            shop_home_service_config_id: c.shop_home_service_config_id,
            shop_id: c.shop_id,
            enabled: c.enabled,
            service_radius: c.service_radius,
            home_fee: c.home_fee,
            distance_pricing_enabled: c.distance_pricing_enabled,
            order_dispatch_mode: c.order_dispatch_mode,
            before_service_photo_enabled: c.before_service_photo_enabled,
            after_service_photo_enabled: c.after_service_photo_enabled,
            default_service_fee_type: c.default_service_fee_type,
            default_service_fee_value: c.default_service_fee_value,
            distance_pricing_list: pricing_vo(pricing),
            add_time: c.add_time,
        }))
    }

    /// `GET /o2o/localDelivery/getConfig`：同城配送配置（含距离溢价行）。
    pub async fn local_delivery_config(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopLocalDeliveryConfigVO>> {
        let Some(c) = self.repo().find_local_delivery(shop_id).await? else {
            return Ok(None);
        };
        let pricing = self.repo().find_local_pricing(shop_id).await?;
        Ok(Some(ShopLocalDeliveryConfigVO {
            shop_local_delivery_config_id: c.shop_local_delivery_config_id,
            shop_id: c.shop_id,
            enabled: c.enabled,
            delivery_radius: c.delivery_radius,
            min_order_amount: c.min_order_amount,
            base_delivery_fee: c.base_delivery_fee,
            distance_pricing_enabled: c.distance_pricing_enabled,
            order_dispatch_mode: c.order_dispatch_mode,
            pickup_photo_enabled: c.pickup_photo_enabled,
            delivery_complete_photo_enabled: c.delivery_complete_photo_enabled,
            default_service_fee_type: c.default_service_fee_type,
            default_service_fee_value: c.default_service_fee_value,
            distance_pricing_list: pricing_vo(pricing),
            time_slot_enabled: c.time_slot_enabled,
            time_slots: nextrade_core::models::trade::delivery_slot::parse_slot_defs(
                &c.time_slots,
            ),
            booking_days: c.booking_days,
            cutoff_minutes: c.cutoff_minutes,
            add_time: c.add_time,
        }))
    }

    /// `POST /o2o/localDelivery/saveConfig`：保存同城配送配置（配置双轨统一后
    /// 的唯一写路径；shop_id 已由路由层按租户作用域解析）。
    pub async fn save_local_delivery(
        &self,
        shop_id: i32,
        p: &ShopLocalDeliverySaveParam,
        now: i32,
    ) -> Result<()> {
        let existing = self.repo().find_local_delivery(shop_id).await?;
        let base = existing.unwrap_or_default();
        let cfg = ShopLocalDeliveryConfig {
            shop_local_delivery_config_id: base.shop_local_delivery_config_id,
            shop_id,
            enabled: p.enabled.unwrap_or(base.enabled),
            delivery_radius: p.delivery_radius.unwrap_or(base.delivery_radius),
            min_order_amount: p.min_order_amount.unwrap_or(base.min_order_amount),
            base_delivery_fee: p.base_delivery_fee.unwrap_or(base.base_delivery_fee),
            distance_pricing_enabled: p
                .distance_pricing_enabled
                .unwrap_or(base.distance_pricing_enabled),
            order_dispatch_mode: p.order_dispatch_mode.unwrap_or(base.order_dispatch_mode),
            pickup_photo_enabled: p
                .pickup_photo_enabled
                .unwrap_or(base.pickup_photo_enabled),
            delivery_complete_photo_enabled: p
                .delivery_complete_photo_enabled
                .unwrap_or(base.delivery_complete_photo_enabled),
            default_service_fee_type: p
                .default_service_fee_type
                .unwrap_or(base.default_service_fee_type),
            default_service_fee_value: p
                .default_service_fee_value
                .unwrap_or(base.default_service_fee_value),
            // 送达时段（我方新增，迁移 025）：时段定义提交时全量替换
            // 并经纯逻辑清洗（非法行丢弃、按开始时刻排序去重）；数值
            // 字段做范围钳制（天数 1..=7、截单分钟 >= 0）。
            time_slot_enabled: p.time_slot_enabled.unwrap_or(base.time_slot_enabled),
            time_slots: p
                .time_slots
                .as_deref()
                .map(nextrade_core::models::trade::delivery_slot::slot_defs_json)
                .unwrap_or(base.time_slots),
            // 无存量行时 base 为全零默认，须回落到口径默认（天数 2 / 截单 30 分钟，与迁移 025 列默认一致）
            booking_days: {
                let v = p.booking_days.unwrap_or(base.booking_days);
                if v <= 0 { 2 } else { v.clamp(1, 7) }
            },
            cutoff_minutes: {
                let v = p.cutoff_minutes.unwrap_or(base.cutoff_minutes);
                if existing.is_none() && p.cutoff_minutes.is_none() {
                    30
                } else {
                    v.max(0)
                }
            },
            add_time: if base.add_time > 0 { base.add_time } else { now },
        };
        let pricing = pricing_rows(p.distance_pricing_list.as_deref());
        self.repo().save_local_delivery(&cfg, &pricing).await
    }

    /// `POST /o2o/homeService/saveConfig`：保存上门服务配置（唯一写路径）。
    pub async fn save_home_service(
        &self,
        shop_id: i32,
        p: &ShopHomeServiceSaveParam,
        now: i32,
    ) -> Result<()> {
        let existing = self.repo().find_home_service(shop_id).await?;
        let base = existing.unwrap_or_default();
        let cfg = ShopHomeServiceConfig {
            shop_home_service_config_id: base.shop_home_service_config_id,
            shop_id,
            enabled: p.enabled.unwrap_or(base.enabled),
            service_radius: p.service_radius.or(base.service_radius),
            home_fee: p.home_fee.unwrap_or(base.home_fee),
            distance_pricing_enabled: p
                .distance_pricing_enabled
                .unwrap_or(base.distance_pricing_enabled),
            order_dispatch_mode: p.order_dispatch_mode.unwrap_or(base.order_dispatch_mode),
            before_service_photo_enabled: p
                .before_service_photo_enabled
                .unwrap_or(base.before_service_photo_enabled),
            after_service_photo_enabled: p
                .after_service_photo_enabled
                .unwrap_or(base.after_service_photo_enabled),
            default_service_fee_type: p
                .default_service_fee_type
                .unwrap_or(base.default_service_fee_type),
            default_service_fee_value: p
                .default_service_fee_value
                .unwrap_or(base.default_service_fee_value),
            add_time: if base.add_time > 0 { base.add_time } else { now },
        };
        let pricing = pricing_rows(p.distance_pricing_list.as_deref());
        self.repo().save_home_service(&cfg, &pricing).await
    }

    /// `GET /o2o/shopPickupConfig/getShopPickupTpl`：自提模板
    /// （门店级模板 → 平台默认模板回退；`status` 取 `shop_pickup_config.status`）。
    pub async fn shop_pickup_tpl(&self, shop_id: i32) -> Result<Option<ShopPickupTplVO>> {
        let Some(tpl) = self.repo().find_pickup_tpl(i64::from(shop_id)).await? else {
            return Ok(None);
        };
        let status = self
            .repo()
            .find_pickup_config(i64::from(shop_id))
            .await?
            .map(|c| c.status)
            .unwrap_or(1);
        Ok(Some(ShopPickupTplVO {
            shop_pickup_tpl_id: tpl.shop_pickup_tpl_id,
            shop_id: tpl.shop_id,
            tpl_name: Some(tpl.tpl_name),
            stocking_status: tpl.stocking_status,
            stocking_json: parse_json_col(tpl.stocking_json.as_deref()),
            pickup_time_status: tpl.pickup_time_status,
            pickup_time_json: parse_json_col(tpl.pickup_time_json.as_deref()),
            pickup_end_status: tpl.pickup_end_status,
            pickup_end_json: parse_json_col(tpl.pickup_end_json.as_deref()),
            is_default: tpl.is_default,
            status,
        }))
    }
}

/// 仓储定价行元组 → VO 行。
fn pricing_vo(
    rows: Vec<(rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)>,
) -> Vec<DistancePricingVO> {
    rows.into_iter()
        .map(|(threshold, unit, fee, sort)| DistancePricingVO {
            distance_threshold: threshold,
            distance_unit: unit,
            additional_fee: fee,
            sort_order: sort,
        })
        .collect()
}

/// 保存入参定价行 → 仓储元组（缺省值兜底：unit 缺省 1，sort 缺省按序号）。
fn pricing_rows(
    rows: Option<&[nextrade_core::models::o2o::config::DistancePricingSaveRow]>,
) -> Vec<(rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)> {
    use rust_decimal::Decimal;
    rows.unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                r.distance_threshold.unwrap_or_default(),
                r.distance_unit.unwrap_or(Decimal::ONE),
                r.additional_fee.unwrap_or_default(),
                r.sort_order.unwrap_or(i as i32),
            )
        })
        .collect()
}
