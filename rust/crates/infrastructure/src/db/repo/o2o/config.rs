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
            "SELECT shop_home_service_config_id, shop_id, enabled, service_radius, home_fee, distance_pricing_enabled, order_dispatch_mode, before_service_photo_enabled, after_service_photo_enabled, default_service_fee_type, default_service_fee_value, add_time FROM shop_home_service_config WHERE shop_id = $1 LIMIT 1",
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
            "SELECT shop_local_delivery_config_id, shop_id, enabled, delivery_radius, min_order_amount, base_delivery_fee, distance_pricing_enabled, order_dispatch_mode, pickup_photo_enabled, delivery_complete_photo_enabled, default_service_fee_type, default_service_fee_value, time_slot_enabled, time_slots, booking_days, cutoff_minutes, add_time FROM shop_local_delivery_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量同城配送配置（后台店铺列表逐页填充起送价/配送费；无配置的店铺不在结果内）。
    pub async fn find_local_delivery_for_shops(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<ShopLocalDeliveryConfig>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, ShopLocalDeliveryConfig>(
            "SELECT shop_local_delivery_config_id, shop_id, enabled, delivery_radius, min_order_amount, base_delivery_fee, distance_pricing_enabled, order_dispatch_mode, pickup_photo_enabled, delivery_complete_photo_enabled, default_service_fee_type, default_service_fee_value, time_slot_enabled, time_slots, booking_days, cutoff_minutes, add_time FROM shop_local_delivery_config WHERE shop_id = ANY($1)",
        )
        .bind(shop_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 距离定价行（同城配送）：返回 (distance_threshold, distance_unit, additional_fee, sort_order)。
    pub async fn find_local_pricing(
        &self,
        shop_id: i32,
    ) -> Result<Vec<(rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)>> {
        sqlx::query_as::<_, (rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)>(
            "SELECT p.distance_threshold, p.distance_unit, p.additional_fee, p.sort_order \
             FROM shop_local_delivery_distance_pricing p \
             JOIN shop_local_delivery_config c \
               ON c.shop_local_delivery_config_id = p.shop_local_delivery_config_id \
             WHERE c.shop_id = $1 ORDER BY p.sort_order, p.shop_local_delivery_distance_pricing_id",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 距离定价行（上门服务）：返回 (distance_threshold, distance_unit, additional_fee, sort_order)。
    pub async fn find_home_pricing(
        &self,
        shop_id: i32,
    ) -> Result<Vec<(rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)>> {
        sqlx::query_as::<_, (rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)>(
            "SELECT p.distance_threshold, p.distance_unit, p.additional_fee, p.sort_order \
             FROM shop_home_service_distance_pricing p \
             JOIN shop_home_service_config c \
               ON c.shop_home_service_config_id = p.shop_home_service_config_id \
             WHERE c.shop_id = $1 ORDER BY p.sort_order, p.shop_home_service_distance_pricing_id",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 保存同城配送配置（upsert）+ 全量替换距离定价行。
    ///
    /// 骑手 MVP 配置双轨统一：此前 admin「保存」走通用 config JSON（h_11 cfg_set），
    /// GET 却读本表，两轨不一致导致保存不生效；现统一落表，h_10/h_11 已下线。
    pub async fn save_local_delivery(
        &self,
        cfg: &ShopLocalDeliveryConfig,
        pricing: &[(rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let (config_id,): (i64,) = sqlx::query_as(
            "INSERT INTO shop_local_delivery_config (shop_id, enabled, delivery_radius, \
             min_order_amount, base_delivery_fee, distance_pricing_enabled, order_dispatch_mode, \
             pickup_photo_enabled, delivery_complete_photo_enabled, \
             default_service_fee_type, default_service_fee_value, \
             time_slot_enabled, time_slots, booking_days, cutoff_minutes, add_time) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) \
             ON CONFLICT (shop_id) DO UPDATE SET enabled = EXCLUDED.enabled, \
             delivery_radius = EXCLUDED.delivery_radius, \
             min_order_amount = EXCLUDED.min_order_amount, \
             base_delivery_fee = EXCLUDED.base_delivery_fee, \
             distance_pricing_enabled = EXCLUDED.distance_pricing_enabled, \
             order_dispatch_mode = EXCLUDED.order_dispatch_mode, \
             pickup_photo_enabled = EXCLUDED.pickup_photo_enabled, \
             delivery_complete_photo_enabled = EXCLUDED.delivery_complete_photo_enabled, \
             default_service_fee_type = EXCLUDED.default_service_fee_type, \
             default_service_fee_value = EXCLUDED.default_service_fee_value, \
             time_slot_enabled = EXCLUDED.time_slot_enabled, \
             time_slots = EXCLUDED.time_slots, \
             booking_days = EXCLUDED.booking_days, \
             cutoff_minutes = EXCLUDED.cutoff_minutes \
             RETURNING shop_local_delivery_config_id",
        )
        .bind(cfg.shop_id)
        .bind(cfg.enabled)
        .bind(cfg.delivery_radius)
        .bind(cfg.min_order_amount)
        .bind(cfg.base_delivery_fee)
        .bind(cfg.distance_pricing_enabled)
        .bind(cfg.order_dispatch_mode)
        .bind(cfg.pickup_photo_enabled)
        .bind(cfg.delivery_complete_photo_enabled)
        .bind(cfg.default_service_fee_type)
        .bind(cfg.default_service_fee_value)
        .bind(cfg.time_slot_enabled)
        .bind(&cfg.time_slots)
        .bind(cfg.booking_days)
        .bind(cfg.cutoff_minutes)
        .bind(cfg.add_time)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        sqlx::query(
            "DELETE FROM shop_local_delivery_distance_pricing \
             WHERE shop_local_delivery_config_id = $1",
        )
        .bind(config_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        for (threshold, unit, fee, sort) in pricing {
            sqlx::query(
                "INSERT INTO shop_local_delivery_distance_pricing \
                 (shop_local_delivery_config_id, distance_threshold, distance_unit, \
                  additional_fee, sort_order, add_time) VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(config_id)
            .bind(threshold)
            .bind(unit)
            .bind(fee)
            .bind(sort)
            .bind(cfg.add_time)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)
    }

    /// 保存上门服务配置（upsert）+ 全量替换距离定价行（同上，配置双轨统一）。
    pub async fn save_home_service(
        &self,
        cfg: &ShopHomeServiceConfig,
        pricing: &[(rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, i32)],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let (config_id,): (i32,) = sqlx::query_as(
            "INSERT INTO shop_home_service_config (shop_id, enabled, service_radius, home_fee, \
             distance_pricing_enabled, order_dispatch_mode, before_service_photo_enabled, \
             after_service_photo_enabled, default_service_fee_type, default_service_fee_value, \
             add_time) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) \
             ON CONFLICT (shop_id) DO UPDATE SET enabled = EXCLUDED.enabled, \
             service_radius = EXCLUDED.service_radius, home_fee = EXCLUDED.home_fee, \
             distance_pricing_enabled = EXCLUDED.distance_pricing_enabled, \
             order_dispatch_mode = EXCLUDED.order_dispatch_mode, \
             before_service_photo_enabled = EXCLUDED.before_service_photo_enabled, \
             after_service_photo_enabled = EXCLUDED.after_service_photo_enabled, \
             default_service_fee_type = EXCLUDED.default_service_fee_type, \
             default_service_fee_value = EXCLUDED.default_service_fee_value \
             RETURNING shop_home_service_config_id",
        )
        .bind(cfg.shop_id)
        .bind(cfg.enabled)
        .bind(cfg.service_radius)
        .bind(cfg.home_fee)
        .bind(cfg.distance_pricing_enabled)
        .bind(cfg.order_dispatch_mode)
        .bind(cfg.before_service_photo_enabled)
        .bind(cfg.after_service_photo_enabled)
        .bind(cfg.default_service_fee_type)
        .bind(cfg.default_service_fee_value)
        .bind(cfg.add_time)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        sqlx::query(
            "DELETE FROM shop_home_service_distance_pricing \
             WHERE shop_home_service_config_id = $1",
        )
        .bind(config_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        for (threshold, unit, fee, sort) in pricing {
            sqlx::query(
                "INSERT INTO shop_home_service_distance_pricing \
                 (shop_home_service_config_id, distance_threshold, distance_unit, \
                  additional_fee, sort_order, add_time) VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(config_id)
            .bind(threshold)
            .bind(unit)
            .bind(fee)
            .bind(sort)
            .bind(cfg.add_time)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)
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
