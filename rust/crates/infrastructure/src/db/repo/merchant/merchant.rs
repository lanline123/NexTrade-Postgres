//! 商户域（merchant::merchant）仓储。
//!
//! 对应原 Java `MerchantMapper` / `MerchantServiceImpl` / `MerchantUserMapper`
//! / `AdminUserService` / `RegionService` / `ShopFundsService#getCountByMerchantId`。
//!
//! 表 / 列（`python tools/schema_parser.py --table merchant`）：
//! `merchant_id` / `merchant_apply_id` / `user_id` / `add_time` / `base_data` /
//! `shop_data` / `merchant_data` / `status` / `is_delete` / `type` /
//! `company_name` / `corporate_name` / `settlement_cycle`。
//!
//! 只读回填表：`merchant_user`（is_admin=1）、`admin_user`（admin_id/username）、
//! `user`（会员精简列）、`region`（region_id/region_name）、`shop`（关联店铺数）。
//!
//! # 软删口径（重要）
//!
//! `merchant` 无 `@TableLogic`，Java 列表显式过滤 `is_delete is null or 0`；
//! `del` 为软删 `is_delete = 1`。本仓储逐条复刻。
//!
//! 查询统一使用运行时 `sqlx::query_as`。

use sqlx::{FromRow,PgPool};

use nextrade_core::entities::Merchant;
use nextrade_core::models::merchant::merchant::{MSG_INVALID_FIELD_VALUE, MSG_MERCHANT_NOT_FOUND};
use nextrade_core::{Error, Result};

/// `merchant` 的固定 SELECT 列清单（13 列，顺序与实体一致）。
pub const MERCHANT_COLUMNS: &str = "merchant_id, merchant_apply_id, user_id, add_time, base_data, shop_data, merchant_data, status, is_delete, type, company_name, corporate_name, settlement_cycle";

/// 会员精简行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct MerchantUserRow {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub mobile: String,
    pub avatar: String,
}

/// 商户管理员关联行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct MerchantAdminRow {
    pub merchant_id: i32,
    pub admin_user_id: Option<i32>,
    pub user_id: Option<i32>,
    pub is_admin: Option<i32>,
}

/// 商户仓储（持有连接池）。
#[derive(Clone)]
pub struct MerchantRepository {
    pool: PgPool,
}

impl MerchantRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 商户分页（Java `MerchantServiceImpl#list`：
    /// `(is_delete is null or 0)` + merchantId + keyword(companyName|corporateName)）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn paginate(
        &self,
        merchant_id: Option<i32>,
        status: Option<i32>,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<Merchant>)> {
        let like = keyword
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));
        let mut where_parts = vec!["(is_delete IS NULL OR is_delete = 0)".to_string()];
        if merchant_id.is_some() {
            where_parts.push("merchant_id = ?".into());
        }
        if status.is_some() {
            where_parts.push("status = ?".into());
        }
        if like.is_some() {
            where_parts.push("(company_name ILIKE ? OR corporate_name ILIKE ?)".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let column = resolve_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM merchant {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_filters(cq, merchant_id, status, like.as_deref());
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {MERCHANT_COLUMNS} FROM merchant {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, Merchant>(&list_sql);
        lq = bind_filters(lq, merchant_id, status, like.as_deref());
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查未删除商户（Java `getById` + 列表口径）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn find_by_id(&self, merchant_id: i32) -> Result<Option<Merchant>> {
        let sql = format!(
            "SELECT {MERCHANT_COLUMNS} FROM merchant WHERE merchant_id = $1 AND (is_delete IS NULL OR is_delete = 0) LIMIT 1"
        );
        sqlx::query_as::<_, Merchant>(&sql)
            .bind(merchant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增商户，返回自增主键。
    pub async fn insert(&self, m: &Merchant) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO merchant (merchant_apply_id, user_id, add_time, base_data, shop_data, merchant_data, status, is_delete, type, company_name, corporate_name, settlement_cycle) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING merchant_id",
        )
        .bind(m.merchant_apply_id)
        .bind(m.user_id)
        .bind(m.add_time)
        .bind(&m.base_data)
        .bind(&m.shop_data)
        .bind(&m.merchant_data)
        .bind(m.status)
        .bind(m.is_delete)
        .bind(m.r#type)
        .bind(&m.company_name)
        .bind(&m.corporate_name)
        .bind(m.settlement_cycle)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新商户（`merchant_id` 之外 12 列）。
    pub async fn update(&self, m: &Merchant) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE merchant SET merchant_apply_id = $1, user_id = $2, add_time = $3, base_data = $4, shop_data = $5, merchant_data = $6, status = $7, is_delete = $8, type = $9, company_name = $10, corporate_name = $11, settlement_cycle = $12 WHERE merchant_id = $13",
        )
        .bind(m.merchant_apply_id)
        .bind(m.user_id)
        .bind(m.add_time)
        .bind(&m.base_data)
        .bind(&m.shop_data)
        .bind(&m.merchant_data)
        .bind(m.status)
        .bind(m.is_delete)
        .bind(m.r#type)
        .bind(&m.company_name)
        .bind(&m.corporate_name)
        .bind(m.settlement_cycle)
        .bind(m.merchant_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 软删（`is_delete = 1`，Java `delMerchant`）。
    pub async fn soft_delete(&self, ids: &[i32], now: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE merchant SET is_delete = 1, add_time = add_time WHERE merchant_id IN ({placeholders})"
        ));
        let _ = now;
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新单个字段（白名单由服务层归一；防注入）。
    pub async fn update_field(&self, merchant_id: i32, field: &str, val: &str) -> Result<bool> {
        if !matches!(field, "status") {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        let sql = format!("UPDATE merchant SET \"{field}\" = $1 WHERE merchant_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(merchant_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 商户关联店铺数（Java `ShopFundsService#getCountByMerchantId`：
    /// 排除自提点 `shop_type = 3`、未归档 `is_delete is null or 0`）。
    pub async fn shop_count_by_merchant(&self, merchant_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop WHERE merchant_id = $1 AND shop_type <> 3 AND (is_delete IS NULL OR is_delete = 0)",
        )
        .bind(merchant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批量取商户管理员关联（`merchant_user.is_admin = 1`，每个 merchant 取首条）。
    pub async fn find_admin_links(&self, merchant_ids: &[i32]) -> Result<Vec<MerchantAdminRow>> {
        if merchant_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; merchant_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT merchant_id, admin_user_id, user_id, is_admin FROM merchant_user WHERE is_admin = 1 AND merchant_id IN ({placeholders}) ORDER BY merchant_user_id ASC"
        ));
        let mut q = sqlx::query_as::<_, MerchantAdminRow>(&sql);
        for id in merchant_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量管理员用户名（`admin_user.admin_id → username`）。
    pub async fn find_admin_usernames(&self, admin_user_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let ids: Vec<i32> = admin_user_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT admin_id, username FROM admin_user WHERE admin_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量会员精简信息（`user` 表）。
    pub async fn find_users(&self, user_ids: &[i32]) -> Result<Vec<MerchantUserRow>> {
        let ids: Vec<i32> = user_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_id, username, nickname, mobile, avatar FROM \"user\" WHERE user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, MerchantUserRow>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量地区名（`region.region_id → region_name`，详情拼接完整地址用）。
    pub async fn find_region_names(&self, region_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if region_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; region_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT region_id, region_name FROM region WHERE region_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in region_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// 排序字段白名单（`merchant` 实存列；未知回退主键）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some("merchantId") | Some("merchant_id") => "merchant_id",
        Some("addTime") | Some("add_time") => "add_time",
        Some("status") => "status",
        Some("type") => "type",
        Some("settlementCycle") | Some("settlement_cycle") => "settlement_cycle",
        _ => "merchant_id",
    }
}

fn bind_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    merchant_id: Option<i32>,
    status: Option<i32>,
    like: Option<&'q str>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    if let Some(v) = merchant_id {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    if let Some(v) = like {
        q = q.bind(v).bind(v);
    }
    q
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_MERCHANT_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merchant_columns_cover_entity() {
        let listed: Vec<&str> = MERCHANT_COLUMNS.split(", ").collect();
        assert_eq!(
            listed,
            vec![
                "merchant_id",
                "merchant_apply_id",
                "user_id",
                "add_time",
                "base_data",
                "shop_data",
                "merchant_data",
                "status",
                "is_delete",
                "type",
                "company_name",
                "corporate_name",
                "settlement_cycle",
            ]
        );
        assert!(!MERCHANT_COLUMNS.contains('*'));
    }

    #[test]
    fn sort_whitelist_falls_back_to_primary_key() {
        assert_eq!(resolve_sort_column(None), "merchant_id");
        assert_eq!(resolve_sort_column(Some("merchantId")), "merchant_id");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("settlementCycle")), "settlement_cycle");
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE merchant")), "merchant_id");
    }

    #[test]
    fn not_found_message_is_verbatim() {
        assert_eq!(MSG_MERCHANT_NOT_FOUND, "商户不存在");
    }
}
