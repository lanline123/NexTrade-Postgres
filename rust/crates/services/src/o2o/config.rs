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

use nextrade_core::models::o2o::config::{
    parse_json_col, ShopConfigQuery, ShopHomeServiceConfigVO, ShopLocalDeliveryConfigVO,
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
    /// 路由层 `Option` 判空实现）。
    pub async fn home_service_config(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopHomeServiceConfigVO>> {
        Ok(self
            .repo()
            .find_home_service(shop_id)
            .await?
            .map(|c| ShopHomeServiceConfigVO {
                shop_home_service_config_id: c.shop_home_service_config_id,
                shop_id: c.shop_id,
                enabled: c.enabled,
                service_radius: c.service_radius,
                home_fee: c.home_fee,
                distance_pricing_enabled: c.distance_pricing_enabled,
                order_dispatch_mode: c.order_dispatch_mode,
                before_service_photo_enabled: c.before_service_photo_enabled,
                after_service_photo_enabled: c.after_service_photo_enabled,
                add_time: c.add_time,
            }))
    }

    /// `GET /o2o/localDelivery/getConfig`：同城配送配置。
    pub async fn local_delivery_config(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopLocalDeliveryConfigVO>> {
        Ok(self
            .repo()
            .find_local_delivery(shop_id)
            .await?
            .map(|c| ShopLocalDeliveryConfigVO {
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
                add_time: c.add_time,
            }))
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
