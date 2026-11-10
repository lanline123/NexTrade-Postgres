//! 店铺资金域（merchant::shop_funds）仓储。
//!
//! 对应原 Java `ShopFundsMapper`（`@TableName("shop")`）/
//! `ShopFundsServiceImpl` / `ConfigService#getConfigVal`。
//!
//! # 表 / 列（权威来源 `python tools/schema_parser.py --table shop`）
//!
//! 本域只读写 `shop` 表的资金 / 展示相关列：
//! `shop_id` / `shop_title` / `shop_logo` / `add_time` / `click_count` /
//! `status` / `merchant_id` / `shop_money` / `frozen_money` / `contact_mobile` /
//! `description` / `kefu_phone` / `kefu_weixin` / `kefu_link` /
//! `is_contact_kefu` / `kefu_inlet` / `shop_type` / `is_delete`。
//!
//! # 过滤口径（逐条对照 Java `buildListQueryWrapper`）
//! - `shop_type <> 3`（排除自提点，Java `ShopTypeEnum.PICKUP`）
//! - `(is_delete IS NULL OR is_delete = 0)`（排除已归档门店）
//! - `status > 0` 时按状态过滤；`shop_type` 精确；`keyword` ILIKE `shop_title`
//!
//! # 软删
//! Java `del(id)` 走 `BaseServiceImpl#del` → `removeById`；`shop.is_delete`
//! 为逻辑删除列（`ShopFunds` 模型 `@TableLogic`），本仓储显式
//! `UPDATE shop SET is_delete = 1`，并用 `is_delete is null or 0` 口径查询。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};

use nextrade_core::models::merchant::shop_funds::{MSG_INVALID_FIELD_VALUE, MSG_SHOP_NOT_FOUND};
use nextrade_core::{Error, Result};

/// `shop` 表本域读取的列清单（18 列）。
pub const SHOP_FUNDS_COLUMNS: &str = "shop_id, shop_title, shop_logo, add_time, click_count, status, merchant_id, shop_money, frozen_money, contact_mobile, description, kefu_phone, kefu_weixin, kefu_link, is_contact_kefu, kefu_inlet, shop_type, is_delete";

/// 店铺资金行（`shop` 表子集）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ShopFundsRow {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
    pub add_time: Option<i32>,
    pub click_count: Option<i32>,
    pub status: Option<i32>,
    pub merchant_id: Option<i32>,
    pub shop_money: Option<Decimal>,
    pub frozen_money: Option<Decimal>,
    pub contact_mobile: Option<String>,
    pub description: Option<String>,
    pub kefu_phone: Option<String>,
    pub kefu_weixin: Option<String>,
    pub kefu_link: Option<String>,
    pub is_contact_kefu: Option<i32>,
    pub kefu_inlet: Option<i32>,
    pub shop_type: Option<i32>,
    pub is_delete: Option<i32>,
}

/// 局部更新入参（`None` 保留原值）。
#[derive(Debug, Clone, Default)]
pub struct ShopFundsUpdateFields {
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
    pub shop_money: Option<Decimal>,
    pub frozen_money: Option<Decimal>,
    pub status: Option<i32>,
    pub merchant_id: Option<i32>,
    pub description: Option<String>,
    pub contact_mobile: Option<String>,
}

/// 店铺资金仓储。
#[derive(Clone)]
pub struct ShopFundsRepository {
    pool: PgPool,
}

impl ShopFundsRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 资金列表分页（Java `ShopFundsServiceImpl#list` 常规排序分支）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn paginate(
        &self,
        status: Option<i32>,
        shop_type: Option<i32>,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<ShopFundsRow>)> {
        let like = keyword
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));
        let mut where_parts = vec![
            "shop_type <> 3".to_string(),
            "(is_delete IS NULL OR is_delete = 0)".to_string(),
        ];
        if status.is_some() {
            where_parts.push("status = ?".into());
        }
        if shop_type.is_some() {
            where_parts.push("shop_type = ?".into());
        }
        if like.is_some() {
            where_parts.push("shop_title ILIKE ?".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let column = resolve_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_filters(cq, status, shop_type, like.as_deref());
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {SHOP_FUNDS_COLUMNS} FROM shop {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, ShopFundsRow>(&list_sql);
        lq = bind_filters(lq, status, shop_type, like.as_deref());
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询未删除店铺（Java `getById`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn find_by_id(&self, shop_id: i32) -> Result<Option<ShopFundsRow>> {
        let sql = format!(
            "SELECT {SHOP_FUNDS_COLUMNS} FROM shop WHERE shop_id = $1 AND (is_delete IS NULL OR is_delete = 0) LIMIT 1"
        );
        sqlx::query_as::<_, ShopFundsRow>(&sql)
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 创建店铺资金记录（Java `ShopFundsServiceImpl#create` → `save`）。
    pub async fn insert_shop_funds(
        &self,
        dto: &nextrade_core::models::merchant::shop_funds::ShopFundsCreateDTO,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let sql = "INSERT INTO shop (shop_id, shop_title, shop_logo, status, merchant_id, shop_money, \
                   frozen_money, contact_mobile, description, add_time) \
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
        let res: i32 = sqlx::query_scalar(&format!("{} RETURNING shop_id", sql))
            .bind(dto.shop_id.unwrap_or(0))
            .bind(dto.shop_title.as_deref().unwrap_or(""))
            .bind(dto.shop_logo.as_deref().unwrap_or(""))
            .bind(dto.status.unwrap_or(0))
            .bind(dto.merchant_id.unwrap_or(0))
            .bind(dto.shop_money.unwrap_or(Decimal::ZERO))
            .bind(dto.frozen_money.unwrap_or(Decimal::ZERO))
            .bind(dto.contact_mobile.as_deref().unwrap_or(""))
            .bind(dto.description.as_deref().unwrap_or(""))
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    /// 局部更新资金字段（见 models 模块头差异第 2 条）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn update_fields(&self, shop_id: i32, f: &ShopFundsUpdateFields) -> Result<bool> {
        let mut sets: Vec<&str> = Vec::new();
        if f.shop_title.is_some() {
            sets.push("shop_title = ?");
        }
        if f.shop_logo.is_some() {
            sets.push("shop_logo = ?");
        }
        if f.shop_money.is_some() {
            sets.push("shop_money = ?");
        }
        if f.frozen_money.is_some() {
            sets.push("frozen_money = ?");
        }
        if f.status.is_some() {
            sets.push("status = ?");
        }
        if f.merchant_id.is_some() {
            sets.push("merchant_id = ?");
        }
        if f.description.is_some() {
            sets.push("description = ?");
        }
        if f.contact_mobile.is_some() {
            sets.push("contact_mobile = ?");
        }
        if sets.is_empty() {
            return Ok(false);
        }
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE shop SET {} WHERE shop_id = ? AND (is_delete IS NULL OR is_delete = 0)",
            sets.join(", ")
        ));
        let mut q = sqlx::query(&sql);
        if let Some(v) = &f.shop_title {
            q = q.bind(v);
        }
        if let Some(v) = &f.shop_logo {
            q = q.bind(v);
        }
        if let Some(v) = f.shop_money {
            q = q.bind(v);
        }
        if let Some(v) = f.frozen_money {
            q = q.bind(v);
        }
        if let Some(v) = f.status {
            q = q.bind(v);
        }
        if let Some(v) = f.merchant_id {
            q = q.bind(v);
        }
        if let Some(v) = &f.description {
            q = q.bind(v);
        }
        if let Some(v) = &f.contact_mobile {
            q = q.bind(v);
        }
        let res = q.bind(shop_id).execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 软删（Java \"removeById\" + `@TableLogic`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn soft_delete(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE shop SET is_delete = 1 WHERE (is_delete IS NULL OR is_delete = 0) AND shop_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新单个字段（白名单在服务层归一；防注入）。
    ///
    /// # Errors
    /// 白名单外字段 → \"Validation\"；数据库错误 → \"Database\"。
    pub async fn update_field(&self, shop_id: i32, field: &str, val: &str) -> Result<bool> {
        if !matches!(field, "status") {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE shop SET \"{field}\" = ? WHERE shop_id = ?"));
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 读取配置项（Java `ConfigService#getConfigVal`：
    /// `biz_code = ? AND is_del = 0 LIMIT 1`，缺失返回空串）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn config_val(&self, biz_code: &str) -> Result<String> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v).unwrap_or_default())
    }
}

/// 排序字段白名单（`shop` 实存列；`unSettlementOrder` 为 Java 派生值，
/// 本实现回退主键，见 models 模块头差异第 3 条）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some("shopId") | Some("shop_id") => "shop_id",
        Some("shopTitle") | Some("shop_title") => "shop_title",
        Some("addTime") | Some("add_time") => "add_time",
        Some("shopMoney") | Some("shop_money") => "shop_money",
        Some("frozenMoney") | Some("frozen_money") => "frozen_money",
        Some("status") => "status",
        Some("clickCount") | Some("click_count") => "click_count",
        _ => "shop_id",
    }
}

fn bind_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    status: Option<i32>,
    shop_type: Option<i32>,
    like: Option<&'q str>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    if let Some(v) = status {
        q = q.bind(v);
    }
    if let Some(v) = shop_type {
        q = q.bind(v);
    }
    if let Some(v) = like {
        q = q.bind(v);
    }
    q
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_SHOP_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_cover_shop_funds_vo() {
        let listed: Vec<&str> = SHOP_FUNDS_COLUMNS.split(", ").collect();
        assert_eq!(listed.len(), 18);
        for column in [
            "shop_id",
            "shop_title",
            "shop_money",
            "frozen_money",
            "merchant_id",
            "shop_type",
            "is_delete",
        ] {
            assert!(listed.contains(&column), "缺少列 {column}");
        }
        assert!(!SHOP_FUNDS_COLUMNS.contains('*'));
    }

    #[test]
    fn sort_whitelist_falls_back_to_primary_key() {
        assert_eq!(resolve_sort_column(None), "shop_id");
        assert_eq!(resolve_sort_column(Some("shopMoney")), "shop_money");
        // Java 派生排序字段 → 回退主键（差异 3）
        assert_eq!(resolve_sort_column(Some("unSettlementOrder")), "shop_id");
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE shop")), "shop_id");
    }

    #[test]
    fn not_found_message_is_verbatim() {
        assert_eq!(MSG_SHOP_NOT_FOUND, "店铺不存在");
    }
}
