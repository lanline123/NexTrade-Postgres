//! 物流仓储：`shipping_tpl` / `shipping_tpl_info` / `shipping_type` /
//! `logistics_company` / `logistics_company_config`。
//! 替代 `ShippingTplMapper` 等 5 个 Mapper。
//!
//! TODO(域实现者)：对照 `java/nextrade-service` 中 `setting/impl/*ServiceImpl`
//! 补全查询方法。

use sqlx::PgPool;
use nextrade_core::entities::{LogisticsCompany, ShippingTpl, ShippingType};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct LogisticsRepository {
    pool: PgPool,
}

impl LogisticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按 ID 查询运费模板。
    ///
    /// 注意真实列名为 `shipping_tpl_id`（非 `tpl_id`）。
    pub async fn find_tpl(&self, tpl_id: i32) -> Result<Option<ShippingTpl>> {
        sqlx::query_as::<_, ShippingTpl>(
            "SELECT shipping_tpl_id, shipping_time, shipping_tpl_name, is_free, pricing_type, is_default, shop_id FROM shipping_tpl WHERE shipping_tpl_id = $1 LIMIT 1",
        )
        .bind(tpl_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 运费模板分页。
    ///
    /// 注意真实列名为 `shipping_tpl_name`（非 `tpl_name`）。
    pub async fn paginate_tpls(
        &self,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ShippingTpl>)> {
        let like = keyword.map(|k| format!("%{k}%"));
        let offset = (page.page.max(1) - 1) * page.page_size;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shipping_tpl WHERE ($1 IS NULL OR shipping_tpl_name LIKE $2)",
        )
        .bind(&like)
        .bind(&like)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, ShippingTpl>(
            "SELECT shipping_tpl_id, shipping_time, shipping_tpl_name, is_free, pricing_type, is_default, shop_id FROM shipping_tpl WHERE ($1 IS NULL OR shipping_tpl_name LIKE $2)
             ORDER BY shipping_tpl_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(&like)
        .bind(&like)
        .bind(page.page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0 as u64, rows))
    }

    /// 全部启用的物流公司。
    ///
    /// 注意真实列名为 `logistics_id`（非 `company_id`）。
    /// 库中 `is_show`（是否展示）与 `is_enabled`（是否启用）是两个独立列，
    /// 本方法语义为「启用」，故过滤 `is_enabled`。
    pub async fn find_enabled_companies(&self) -> Result<Vec<LogisticsCompany>> {
        sqlx::query_as::<_, LogisticsCompany>(
            "SELECT * FROM logistics_company WHERE is_enabled = 1 ORDER BY logistics_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 全部展示中的物流公司（前台列表用，过滤 `is_show`）。
    pub async fn find_visible_companies(&self) -> Result<Vec<LogisticsCompany>> {
        sqlx::query_as::<_, LogisticsCompany>(
            "SELECT * FROM logistics_company WHERE is_show = 1 ORDER BY logistics_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 全部配送方式。
    ///
    /// 注意真实列名为 `shipping_type_id`（非 `type_id`）。
    pub async fn find_all_types(&self) -> Result<Vec<ShippingType>> {
        sqlx::query_as::<_, ShippingType>(
            "SELECT * FROM shipping_type ORDER BY shipping_type_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("物流配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
