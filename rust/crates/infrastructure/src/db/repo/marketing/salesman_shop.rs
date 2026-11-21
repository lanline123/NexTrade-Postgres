//! 分销员外壳店「店中店」仓储（marketing::salesman_shop，我方新增）。
//!
//! 表：`salesman_shop`（迁移 `rust/deploy/pg/2026-11-21_salesman_shop.sql`）
//! —— 一分销员一行的**呈现配置**（店名 / 店招 / 简介 / 平台展示开关）。
//! 本仓储只读写该表 + 平台列表所需的 salesman/user 只读联查；分销员
//! 身份、客户绑定、佣金均不在此动（分别归 salesman_admin 仓储与
//! salesman_generation 服务）。

use sqlx::{FromRow, PgPool};

use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// `salesman_shop` 行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct SalesmanShopRow {
    pub salesman_id: i32,
    pub shop_name: String,
    pub banner: String,
    pub intro: String,
    pub is_enabled: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 平台列表联查行（salesman LEFT JOIN salesman_shop LEFT JOIN "user"）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct SalesmanShopAdminRow {
    pub salesman_id: i32,
    pub user_id: i32,
    pub nickname: String,
    pub mobile: String,
    pub salesman_shop_id: i32,
    pub shop_name: String,
    pub banner: String,
    pub intro: String,
    /// 无配置行时为 NULL（服务层按默认启用口径解析）。
    pub is_enabled: Option<i32>,
    pub update_time: i32,
}

const SHOP_COLUMNS: &str =
    "salesman_id, shop_name, banner, intro, is_enabled, add_time, update_time";

#[derive(Debug, Clone)]
pub struct SalesmanShopRepository {
    pool: PgPool,
}

impl SalesmanShopRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按分销员取外壳配置行（无行 = 未自定义）。
    pub async fn find_by_salesman_id(&self, salesman_id: i32) -> Result<Option<SalesmanShopRow>> {
        let sql = format!("SELECT {SHOP_COLUMNS} FROM salesman_shop WHERE salesman_id = $1");
        sqlx::query_as::<_, SalesmanShopRow>(&sql)
            .bind(salesman_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 保存三项自定义（upsert）：不动 `is_enabled` 与 `add_time`。
    pub async fn upsert_profile(
        &self,
        salesman_id: i32,
        shop_name: &str,
        banner: &str,
        intro: &str,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO salesman_shop (salesman_id, shop_name, banner, intro, is_enabled, add_time, update_time) \
             VALUES ($1, $2, $3, $4, 1, $5, $5) \
             ON CONFLICT (salesman_id) DO UPDATE SET shop_name = EXCLUDED.shop_name, \
             banner = EXCLUDED.banner, intro = EXCLUDED.intro, update_time = EXCLUDED.update_time",
        )
        .bind(salesman_id)
        .bind(shop_name)
        .bind(banner)
        .bind(intro)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 平台启停（upsert）：无行时以空三项建行只落开关，不动既有三项。
    pub async fn set_enabled(&self, salesman_id: i32, enabled: bool, now: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO salesman_shop (salesman_id, shop_name, banner, intro, is_enabled, add_time, update_time) \
             VALUES ($1, '', '', '', $2, $3, $3) \
             ON CONFLICT (salesman_id) DO UPDATE SET is_enabled = EXCLUDED.is_enabled, \
             update_time = EXCLUDED.update_time",
        )
        .bind(salesman_id)
        .bind(i32::from(enabled))
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 平台列表：以 salesman 为主表 LEFT JOIN 外壳配置，无配置行的
    /// 分销员也在列（平台可预先禁用）。`scope_shop_id` 为租户作用域
    /// （None = 平台不限；Some 时按 salesman.shop_id 过滤，哨兵 -1
    /// 天然命中空集）。`enabled`：Some(1) 只看启用（含无行默认启用）、
    /// Some(0) 只看禁用、None 全部。
    pub async fn admin_list(
        &self,
        scope_shop_id: Option<i32>,
        keyword: Option<&str>,
        enabled: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<SalesmanShopAdminRow>)> {
        let keyword_like =
            keyword.map(|k| format!("%{}%", k.replace('%', "\\%").replace('_', "\\_")));
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM salesman s \
             LEFT JOIN salesman_shop ss ON ss.salesman_id = s.salesman_id \
             LEFT JOIN \"user\" u ON u.user_id = s.user_id \
             WHERE ($1::int IS NULL OR s.shop_id = $1) \
             AND ($2::text IS NULL OR u.nickname ILIKE $2 OR u.mobile ILIKE $2 OR ss.shop_name ILIKE $2) \
             AND ($3::int IS NULL OR COALESCE(ss.is_enabled, 1) = $3)",
        )
        .bind(scope_shop_id)
        .bind(&keyword_like)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, SalesmanShopAdminRow>(
            "SELECT s.salesman_id AS salesman_id, COALESCE(s.user_id, 0) AS user_id, \
             COALESCE(u.nickname, '') AS nickname, COALESCE(u.mobile, '') AS mobile, \
             COALESCE(s.shop_id, 0) AS salesman_shop_id, \
             COALESCE(ss.shop_name, '') AS shop_name, COALESCE(ss.banner, '') AS banner, \
             COALESCE(ss.intro, '') AS intro, ss.is_enabled AS is_enabled, \
             COALESCE(ss.update_time, 0) AS update_time \
             FROM salesman s \
             LEFT JOIN salesman_shop ss ON ss.salesman_id = s.salesman_id \
             LEFT JOIN \"user\" u ON u.user_id = s.user_id \
             WHERE ($1::int IS NULL OR s.shop_id = $1) \
             AND ($2::text IS NULL OR u.nickname ILIKE $2 OR u.mobile ILIKE $2 OR ss.shop_name ILIKE $2) \
             AND ($3::int IS NULL OR COALESCE(ss.is_enabled, 1) = $3) \
             ORDER BY s.salesman_id DESC LIMIT $4 OFFSET $5",
        )
        .bind(scope_shop_id)
        .bind(&keyword_like)
        .bind(enabled)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0, rows))
    }
}
