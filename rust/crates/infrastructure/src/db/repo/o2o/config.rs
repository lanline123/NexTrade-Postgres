//! o2o 门店级配置仓储：`shop_home_service_config` / `shop_local_delivery_config` /
//! `shop_pickup_config` / `shop_pickup_tpl`。
//!
//! 对应原 Java（overseas）：
//! - `com.nextrade.mapper.o2o.ShopHomeServiceConfigMapper`（XML：按 shop_id 查询）
//! - `ShopLocalDeliveryConfigMapper` / `ShopPickupConfigMapper` / `ShopPickupTplMapper`
//!   （BaseMapper 通用 `selectOne(shop_id)` 查询）
//!
//! 列名对齐 `crates/core/src/entities/o2o.rs` 对应实体。

use sqlx::PgPool;

use nextrade_core::entities::o2o::{
    ShopHomeServiceConfig, ShopLocalDeliveryConfig, ShopPickupConfig, ShopPickupTpl,
};
use nextrade_core::{Error, Result};

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct O2oConfigRepository {
    pool: PgPool,
}

impl O2oConfigRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 预约上门服务配置（按 shop_id 唯一键；无配置行返回 `None`）。
    pub async fn find_home_service(&self, shop_id: i32) -> Result<Option<ShopHomeServiceConfig>> {
        sqlx::query_as::<_, ShopHomeServiceConfig>(
            "SELECT shop_home_service_config_id, shop_id, enabled, service_radius, home_fee, distance_pricing_enabled, order_dispatch_mode, before_service_photo_enabled, after_service_photo_enabled, add_time FROM shop_home_service_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 同城配送配置（按 shop_id 唯一键）。
    pub async fn find_local_delivery(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopLocalDeliveryConfig>> {
        sqlx::query_as::<_, ShopLocalDeliveryConfig>(
            "SELECT shop_local_delivery_config_id, shop_id, enabled, delivery_radius, min_order_amount, base_delivery_fee, distance_pricing_enabled, order_dispatch_mode, pickup_photo_enabled, delivery_complete_photo_enabled, add_time FROM shop_local_delivery_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 自提配置（按 shop_id 唯一键）。
    pub async fn find_pickup_config(&self, shop_id: i64) -> Result<Option<ShopPickupConfig>> {
        sqlx::query_as::<_, ShopPickupConfig>(
            "SELECT shop_pickup_config_id, shop_id, pickup_name, status, pickup_filed, create_time, update_time FROM shop_pickup_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 自提模板：优先取店铺自己的模板，回退平台默认（`shop_id = 0 AND is_default = 1`）。
    ///
    /// 对齐 Java `getShopPickupTpl` 语义（门店级模板 → 平台默认模板）。
    pub async fn find_pickup_tpl(&self, shop_id: i64) -> Result<Option<ShopPickupTpl>> {
        let row = sqlx::query_as::<_, ShopPickupTpl>(
            "SELECT shop_pickup_tpl_id, shop_id, tpl_name, stocking_status, stocking_json, pickup_time_status, pickup_time_json, pickup_end_status, pickup_end_json, is_default, create_time, update_time FROM shop_pickup_tpl WHERE shop_id = $1 ORDER BY is_default DESC, shop_pickup_tpl_id ASC LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        if row.is_some() {
            return Ok(row);
        }
        sqlx::query_as::<_, ShopPickupTpl>(
            "SELECT shop_pickup_tpl_id, shop_id, tpl_name, stocking_status, stocking_json, pickup_time_status, pickup_time_json, pickup_end_status, pickup_end_json, is_default, create_time, update_time FROM shop_pickup_tpl WHERE shop_id = 0 AND is_default = 1 ORDER BY shop_pickup_tpl_id ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("o2o 配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
