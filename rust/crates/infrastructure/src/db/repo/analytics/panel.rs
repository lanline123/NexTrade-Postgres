//! 首页面板域仓储（analytics::panel）。
//!
//! 真实 SQL 聚合：订单（`order`）/订单项（`order_item`）/会员（`user`）/商品（`product`）/
//! 面板菜单（`panel_shortcut_menu`）/权限（`authority`）。
//! 运行时 `sqlx`（不用 `query!` 宏）；`order` 是 MySQL 保留字，统一反引号。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 首页面板域仓储。
#[derive(Clone)]
pub struct PanelRepository {
    pool: PgPool,
}

impl PanelRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 订单窗口统计：`(订单数, 支付金额)`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn order_window(
        &self,
        from: i64,
        to: i64,
        shop_scope: Option<i32>,
        vendor_scope: Option<i32>,
    ) -> Result<(i64, String)> {
        let shop_sql = if shop_scope.is_some() { " AND shop_id = ?" } else { "" };
        let vendor_sql = if vendor_scope.is_some() { " AND vendor_id = ?" } else { "" };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*), COALESCE(SUM(paid_amount), 0) FROM \"order\" WHERE add_time >= ? AND add_time < ?{shop_sql}{vendor_sql}"
        ));
        let mut query = sqlx::query_as(&sql).bind(from).bind(to);
        if let Some(shop) = shop_scope {
            query = query.bind(shop);
        }
        if let Some(vendor) = vendor_scope {
            query = query.bind(vendor);
        }
        let row: Option<(i64, Option<rust_decimal::Decimal>)> = query
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map_or((0, "0".into()), |(n, amt)| {
            (n, amt.unwrap_or_default().to_string())
        }))
    }

    /// 会员注册数窗口。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn user_window(
        &self,
        from: i64,
        to: i64,
        shop_scope: Option<i32>,
    ) -> Result<i64> {
        let (n,): (i64,) = match shop_scope {
            Some(shop) => {
                sqlx::query_as(
                    "SELECT COUNT(DISTINCT u.user_id) FROM \"user\" u JOIN shop_member m ON m.user_id = u.user_id AND m.is_del = 0 WHERE m.shop_id = $1 AND u.reg_time >= $2 AND u.reg_time < $3",
                )
                .bind(shop)
                .bind(from)
                .bind(to)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
            }
            None => sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE reg_time >= $1 AND reg_time < $2")
                .bind(from)
                .bind(to)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?,
        };
        Ok(n)
    }

    /// 按状态计数（`order_status` 过滤）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn order_count_by_status(
        &self,
        status: i32,
        shop_scope: Option<i32>,
        vendor_scope: Option<i32>,
    ) -> Result<i64> {
        let shop_sql = if shop_scope.is_some() { " AND shop_id = ?" } else { "" };
        let vendor_sql = if vendor_scope.is_some() { " AND vendor_id = ?" } else { "" };
        let sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"order\" WHERE order_status = ?{shop_sql}{vendor_sql}"));
        let mut query = sqlx::query_as(&sql).bind(status);
        if let Some(shop) = shop_scope {
            query = query.bind(shop);
        }
        if let Some(vendor) = vendor_scope {
            query = query.bind(vendor);
        }
        let (n,): (i64,) = query.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(n)
    }

    /// 商品总数 / 上架数（`product`：`is_delete = 0`，`product_status = 1` 为上架）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn product_counts(
        &self,
        shop_scope: Option<i32>,
        vendor_scope: Option<i32>,
    ) -> Result<(i64, i64)> {
        let shop_sql = if shop_scope.is_some() { " AND shop_id = ?" } else { "" };
        let vendor_sql = if vendor_scope.is_some() { " AND vendor_id = ?" } else { "" };
        let total_sql =
            crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM product WHERE is_delete = 0{shop_sql}{vendor_sql}"));
        let on_sale_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM product WHERE is_delete = 0 AND product_status = 1{shop_sql}{vendor_sql}"
        ));
        let mut total_query = sqlx::query_as(&total_sql);
        let mut on_sale_query = sqlx::query_as(&on_sale_sql);
        if let Some(shop) = shop_scope {
            total_query = total_query.bind(shop);
            on_sale_query = on_sale_query.bind(shop);
        }
        if let Some(vendor) = vendor_scope {
            total_query = total_query.bind(vendor);
            on_sale_query = on_sale_query.bind(vendor);
        }
        let total: (i64,) = total_query.fetch_one(&self.pool).await.map_err(map_err)?;
        let on_sale: (i64,) = on_sale_query.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok((total.0, on_sale.0))
    }

    /// 按天趋势：`(日期, 订单数, 支付金额)`，按时间升序。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn order_trend(
        &self,
        from: i64,
        to: i64,
        shop_scope: Option<i32>,
        vendor_scope: Option<i32>,
    ) -> Result<Vec<(i64, i64, String)>> {
        let shop_sql = if shop_scope.is_some() { " AND shop_id = ?" } else { "" };
        let vendor_sql = if vendor_scope.is_some() { " AND vendor_id = ?" } else { "" };
        let sql = crate::db::to_pg_placeholders(&format!(
            // PG 的整数除法返回 INT4（MySQL `/` 恒返回 DECIMAL），而 Rust 侧按 i64 解码，
            // 不 CAST 会报 `Rust i64 (INT8) is not compatible with SQL type INT4`。
            "SELECT CAST(add_time / 86400 AS BIGINT) AS day, COUNT(*), COALESCE(SUM(paid_amount), 0) FROM \"order\" WHERE add_time >= ? AND add_time < ?{shop_sql}{vendor_sql} GROUP BY day ORDER BY day ASC"
        ));
        let mut query = sqlx::query_as(&sql).bind(from).bind(to);
        if let Some(shop) = shop_scope {
            query = query.bind(shop);
        }
        if let Some(vendor) = vendor_scope {
            query = query.bind(vendor);
        }
        let rows: Vec<(i64, i64, Option<rust_decimal::Decimal>)> = query
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(day, n, amt)| (day, n, amt.unwrap_or_default().to_string()))
            .collect())
    }

    /// 一键直达：权限名称模糊匹配（Java `authorityService.getAuthorityByName`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn search_authority(
        &self,
        keyword: Option<&str>,
    ) -> Result<Vec<(i32, String, String, Option<String>)>> {
        let like = keyword
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        if let Some(v) = like.as_deref() {
            sqlx::query_as::<_, (i32, String, String, Option<String>)>(
                "SELECT authority_id, authority_sn, authority_name, route_link FROM authority WHERE authority_name ILIKE $1 AND is_show = 1 LIMIT 50",
            )
            .bind(v)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
        } else {
            sqlx::query_as::<_, (i32, String, String, Option<String>)>(
                "SELECT authority_id, authority_sn, authority_name, route_link FROM authority WHERE is_show = 1 LIMIT 50",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
        }
    }

    /// 快捷菜单配置（Java `getByIdentity`：`admin_id + shop_id` 唯一维度）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn shortcut_menu_keys(&self, admin_id: i32, shop_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT menu_keys FROM panel_shortcut_menu WHERE admin_id = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 保存快捷菜单（Java：存在则更新，否则插入）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn save_shortcut_menu_keys(
        &self,
        admin_id: i32,
        shop_id: i32,
        menu_keys: &str,
        now: i64,
    ) -> Result<()> {
        // Java 侧 getByIdentity 后判空再 insert/update；该表没有 (admin_id, shop_id) 唯一键，
        // 因此不能用 ON DUPLICATE KEY。
        let existing: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM panel_shortcut_menu WHERE admin_id = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(admin_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        match existing {
            Some((id,)) => {
                sqlx::query(
                    "UPDATE panel_shortcut_menu SET menu_keys = $1, update_time = $2 WHERE id = $3",
                )
                .bind(menu_keys)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO panel_shortcut_menu (admin_id, shop_id, menu_keys, update_time) VALUES ($1, $2, $3, $4)",
                )
                .bind(admin_id)
                .bind(shop_id)
                .bind(menu_keys)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            }
        }
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
