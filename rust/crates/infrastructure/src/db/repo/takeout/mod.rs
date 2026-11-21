//! 外卖聚合仓储组（takeout 上下文，我方新增）。
//!
//! 表见 `rust/migrations/028_takeout_aggregation.sql`：渠道门店绑定
//! `takeout_channel_binding`、SKU 映射 `takeout_sku_mapping`、外部订单
//! `takeout_external_order`、回调日志 `takeout_callback_log`，以及外部单
//! 在 `order` 表的落库（`order` 子模块）。

pub mod order;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// `takeout_channel_binding` 行（与迁移 030 逐字对齐）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ChannelBindingRow {
    /// 主键。
    pub id: i32,
    /// 渠道 id。
    pub channel: String,
    /// 我方门店 id。
    pub shop_id: i32,
    /// 我方侧门店编码（渠道绑定键）。
    pub app_poi_code: String,
    /// 渠道侧门店 id。
    pub external_poi_id: String,
    /// 渠道侧门店名。
    pub external_poi_name: String,
    /// 授权状态：0 未授权 / 1 已授权 / 2 已失效。
    pub auth_status: i32,
    /// 商家授权令牌（后台列表不回明文，服务层脱敏）。
    pub access_token: String,
    /// 新单自动确认：0/1。
    pub auto_accept: i32,
    /// 启用：0/1。
    pub enabled: i32,
    /// 创建时间（秒）。
    pub add_time: i32,
    /// 更新时间（秒）。
    pub update_time: i32,
}

/// `takeout_sku_mapping` 行（与迁移 030 逐字对齐）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SkuMappingRow {
    /// 主键。
    pub id: i32,
    /// 渠道 id。
    pub channel: String,
    /// 我方门店 id。
    pub shop_id: i32,
    /// 渠道商品 id。
    pub external_spu_id: String,
    /// 渠道 SKU id。
    pub external_sku_id: String,
    /// 渠道商品名快照。
    pub external_name: String,
    /// 我方商品 id。
    pub product_id: i32,
    /// 我方 SKU id。
    pub sku_id: i32,
    /// 创建时间（秒）。
    pub add_time: i32,
    /// 更新时间（秒）。
    pub update_time: i32,
}

/// 渠道门店绑定仓储。
#[derive(Clone)]
pub struct BindingRepository {
    pool: PgPool,
}

impl BindingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 列表（`shop_filter = Some(id)` 时只看该店；平台传 None 看全部）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn list(
        &self,
        shop_filter: Option<i32>,
        channel: Option<&str>,
    ) -> Result<Vec<ChannelBindingRow>> {
        let mut sql = String::from(
            "SELECT id, channel, shop_id, app_poi_code, external_poi_id, external_poi_name, \
             auth_status, access_token, auto_accept, enabled, add_time, update_time \
             FROM takeout_channel_binding WHERE 1 = 1",
        );
        if shop_filter.is_some() {
            sql.push_str(" AND shop_id = $1");
        }
        sql.push_str(" ORDER BY id DESC");
        let mut q = sqlx::query_as::<_, ChannelBindingRow>(&sql);
        if let Some(shop_id) = shop_filter {
            q = q.bind(shop_id);
        }
        let mut rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        if let Some(ch) = channel {
            rows.retain(|r| r.channel == ch);
        }
        Ok(rows)
    }

    /// 按主键查绑定。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<ChannelBindingRow>> {
        sqlx::query_as::<_, ChannelBindingRow>(
            "SELECT id, channel, shop_id, app_poi_code, external_poi_id, external_poi_name, \
             auth_status, access_token, auto_accept, enabled, add_time, update_time \
             FROM takeout_channel_binding WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按绑定键查绑定（回调接入时用；只认启用中的绑定）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn find_by_poi(
        &self,
        channel: &str,
        app_poi_code: &str,
    ) -> Result<Option<ChannelBindingRow>> {
        sqlx::query_as::<_, ChannelBindingRow>(
            "SELECT id, channel, shop_id, app_poi_code, external_poi_id, external_poi_name, \
             auth_status, access_token, auto_accept, enabled, add_time, update_time \
             FROM takeout_channel_binding \
             WHERE channel = $1 AND app_poi_code = $2 AND enabled = 1 LIMIT 1",
        )
        .bind(channel)
        .bind(app_poi_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增（id = 0）或按 id 更新。唯一键（channel, app_poi_code）冲突
    /// 转业务错误，不把 DB 原文抛给前端。
    ///
    /// # Errors
    /// 绑定键重复 → 「该渠道下门店编码已绑定」；其余 DB 错误直传。
    pub async fn upsert(&self, row: &ChannelBindingRow) -> Result<i32> {
        let res = if row.id > 0 {
            sqlx::query_scalar::<_, i32>(
                "UPDATE takeout_channel_binding SET channel = $2, shop_id = $3, \
                 app_poi_code = $4, external_poi_id = $5, external_poi_name = $6, \
                 auth_status = $7, access_token = $8, auto_accept = $9, enabled = $10, \
                 update_time = $11 WHERE id = $1 RETURNING id",
            )
            .bind(row.id)
            .bind(&row.channel)
            .bind(row.shop_id)
            .bind(&row.app_poi_code)
            .bind(&row.external_poi_id)
            .bind(&row.external_poi_name)
            .bind(row.auth_status)
            .bind(&row.access_token)
            .bind(row.auto_accept)
            .bind(row.enabled)
            .bind(row.update_time)
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO takeout_channel_binding \
                   (channel, shop_id, app_poi_code, external_poi_id, external_poi_name, \
                    auth_status, access_token, auto_accept, enabled, add_time, update_time) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
            )
            .bind(&row.channel)
            .bind(row.shop_id)
            .bind(&row.app_poi_code)
            .bind(&row.external_poi_id)
            .bind(&row.external_poi_name)
            .bind(row.auth_status)
            .bind(&row.access_token)
            .bind(row.auto_accept)
            .bind(row.enabled)
            .bind(row.add_time)
            .bind(row.update_time)
            .fetch_one(&self.pool)
            .await
        };
        match res {
            Ok(id) => Ok(id),
            Err(e) => {
                if let sqlx::Error::Database(db) = &e {
                    if db.kind() == sqlx::error::ErrorKind::UniqueViolation {
                        return Err(Error::biz_msg("该渠道下门店编码已绑定"));
                    }
                }
                Err(map_err(e))
            }
        }
    }

    /// 删除绑定。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn delete(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM takeout_channel_binding WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

/// SKU 映射仓储。
#[derive(Clone)]
pub struct MappingRepository {
    pool: PgPool,
}

impl MappingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 列表（按渠道 + 门店过滤，任一为空串/0 表示不过滤该维）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn list(
        &self,
        shop_filter: Option<i32>,
        channel: Option<&str>,
    ) -> Result<Vec<SkuMappingRow>> {
        let mut rows = sqlx::query_as::<_, SkuMappingRow>(
            "SELECT id, channel, shop_id, external_spu_id, external_sku_id, external_name, \
             product_id, sku_id, add_time, update_time \
             FROM takeout_sku_mapping ORDER BY id DESC LIMIT 500",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        if let Some(shop_id) = shop_filter {
            rows.retain(|r| r.shop_id == shop_id);
        }
        if let Some(ch) = channel {
            rows.retain(|r| r.channel == ch);
        }
        Ok(rows)
    }

    /// 按（渠道， 门店， 渠道 SKU）查映射（订单接入时逐行解析用）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn find(
        &self,
        channel: &str,
        shop_id: i32,
        external_sku_id: &str,
    ) -> Result<Option<SkuMappingRow>> {
        sqlx::query_as::<_, SkuMappingRow>(
            "SELECT id, channel, shop_id, external_spu_id, external_sku_id, external_name, \
             product_id, sku_id, add_time, update_time \
             FROM takeout_sku_mapping \
             WHERE channel = $1 AND shop_id = $2 AND external_sku_id = $3 LIMIT 1",
        )
        .bind(channel)
        .bind(shop_id)
        .bind(external_sku_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查映射。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<SkuMappingRow>> {
        sqlx::query_as::<_, SkuMappingRow>(
            "SELECT id, channel, shop_id, external_spu_id, external_sku_id, external_name, \
             product_id, sku_id, add_time, update_time \
             FROM takeout_sku_mapping WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增（id = 0）或按 id 更新。唯一键冲突转业务错误。
    ///
    /// # Errors
    /// 映射重复 → 「该渠道 SKU 已有映射」；其余 DB 错误直传。
    pub async fn upsert(&self, row: &SkuMappingRow) -> Result<i32> {
        let res = if row.id > 0 {
            sqlx::query_scalar::<_, i32>(
                "UPDATE takeout_sku_mapping SET channel = $2, shop_id = $3, \
                 external_spu_id = $4, external_sku_id = $5, external_name = $6, \
                 product_id = $7, sku_id = $8, update_time = $9 WHERE id = $1 RETURNING id",
            )
            .bind(row.id)
            .bind(&row.channel)
            .bind(row.shop_id)
            .bind(&row.external_spu_id)
            .bind(&row.external_sku_id)
            .bind(&row.external_name)
            .bind(row.product_id)
            .bind(row.sku_id)
            .bind(row.update_time)
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO takeout_sku_mapping \
                   (channel, shop_id, external_spu_id, external_sku_id, external_name, \
                    product_id, sku_id, add_time, update_time) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
            )
            .bind(&row.channel)
            .bind(row.shop_id)
            .bind(&row.external_spu_id)
            .bind(&row.external_sku_id)
            .bind(&row.external_name)
            .bind(row.product_id)
            .bind(row.sku_id)
            .bind(row.add_time)
            .bind(row.update_time)
            .fetch_one(&self.pool)
            .await
        };
        match res {
            Ok(id) => Ok(id),
            Err(e) => {
                if let sqlx::Error::Database(db) = &e {
                    if db.kind() == sqlx::error::ErrorKind::UniqueViolation {
                        return Err(Error::biz_msg("该渠道 SKU 已有映射"));
                    }
                }
                Err(map_err(e))
            }
        }
    }

    /// 删除映射。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn delete(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM takeout_sku_mapping WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

/// 外卖聚合仓储聚合（挂载于 `Repositories.takeout`）。
#[derive(Clone)]
pub struct Repositories {
    /// 门店绑定。
    pub binding: BindingRepository,
    /// SKU 映射。
    pub mapping: MappingRepository,
    /// 外部订单 + 回调日志 + 内部单落库。
    pub order: order::ExternalOrderRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            binding: BindingRepository::new(pool.clone()),
            mapping: MappingRepository::new(pool.clone()),
            order: order::ExternalOrderRepository::new(pool),
        }
    }
}
