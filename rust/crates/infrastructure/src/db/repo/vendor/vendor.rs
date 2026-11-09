//! 供应商（`vendor` / `admin_user_vendor`）仓储。
//!
//! 对应原 Java（overseas）：
//! - `com.nextrade.mapper.vendor.VendorMapper`（MyBatis-Plus BaseMapper 通用查询）
//! - `com.nextrade.mapper.vendor.AdminUserVendorMapper`
//! - `VendorServiceImpl#list` / `#create` / `#currentDetail` / `#setting` 所需查询
//!
//! 列名对齐 `vendor` / `admin_user_vendor` 真实 schema
//! （`crates/core/src/entities/vendor.rs`，由 `tools/schema_parser.py` 生成）。
//! 查询统一使用运行时 `sqlx::query_as` / `QueryBuilder`（本机无 `DATABASE_URL`，
//! 不使用编译期宏）。`vendor.active_source_product_key` 等
//! **生成列（GENERATED ALWAYS）不出现在 INSERT/UPDATE 里**。

use sqlx::{PgPool, QueryBuilder};

use nextrade_core::entities::vendor::{AdminUserVendor, Vendor};
use nextrade_core::models::vendor::vendor::VendorListQuery;
use nextrade_core::{Error, Result};

/// `vendor` 表固定 SELECT 列清单（与 `entities::vendor::Vendor` 字段一一对应，
/// 含两个 GENERATED ALWAYS 生成列——可 SELECT 但**不得**出现在 INSERT/UPDATE）。
const VENDOR_SELECT: &str = "SELECT vendor_id, source_type, shop_id, vendor_code, purchase_category_id, vendor_logo, add_time, contact_mobile, vendor_data, person_data, status, after_sales_review, type, vendor_name, kefu_phone, ship_info, receive_info, vendor_money, frozen_money, description, last_login_time, service_fee_rate, fee_rate, self_vendor_name_key, self_vendor_code_key FROM vendor";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct VendorRepository {
    pool: PgPool,
}

impl VendorRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 列表 —————————————————————

    /// 供应商分页（对应 Java `VendorServiceImpl#list`）。
    ///
    /// 过滤条件：`status` 等值、`keyword`（vendor_name / contact_mobile LIKE）、
    /// `vendorIds IN (...)`；`mainAccount` / `account` 由服务层先解析成
    /// vendor id 集合后传入 `extra_vendor_ids`（Java 经 adminUserService 两段查询）。
    ///
    /// `vendor_scope`：供应商账号登录时固定只看自己（Java `getVendorId()`）。
    /// 排序：有 `vendorIds` 时按 `last_login_time DESC`，其余按 `vendor_id DESC`
    /// （与 Java 一致）。
    pub async fn paginate(
        &self,
        q: &VendorListQuery,
        extra_vendor_ids: &[i32],
        vendor_scope: Option<i32>,
    ) -> Result<(u64, Vec<Vendor>)> {
        let (limit, offset) = q.limit_offset();

        let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM vendor WHERE 1=1");
        Self::push_filters(
            &mut count_qb,
            q,
            extra_vendor_ids,
            vendor_scope,
        );
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new(format!("{VENDOR_SELECT} WHERE 1=1"));
        let by_last_login = Self::push_filters(&mut row_qb, q, extra_vendor_ids, vendor_scope);
        row_qb.push(if by_last_login {
            " ORDER BY last_login_time DESC, vendor_id DESC LIMIT "
        } else {
            " ORDER BY vendor_id DESC LIMIT "
        });
        row_qb.push(format!("{limit} OFFSET {offset}"));
        let rows: Vec<Vendor> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 追加 WHERE 片段；返回是否命中 `vendorIds` 分支（决定排序字段）。
    fn push_filters(
        qb: &mut QueryBuilder<'_, sqlx::Postgres>,
        q: &VendorListQuery,
        extra_vendor_ids: &[i32],
        vendor_scope: Option<i32>,
    ) -> bool {
        let _ = q;
        let mut by_last_login = false;
        if let Some(status) = q.status.filter(|s| *s > 0) {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(keyword) = q.keyword.as_deref().filter(|s| !s.trim().is_empty()) {
            let like = format!("%{}%", keyword.trim());
            qb.push(" AND (vendor_name LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR contact_mobile LIKE ");
            qb.push_bind(like);
            qb.push(")");
        }
        if !extra_vendor_ids.is_empty() {
            qb.push(" AND vendor_id IN (");
            let mut sep = qb.separated(", ");
            for id in extra_vendor_ids {
                sep.push_bind(*id);
            }
            drop(sep);
            qb.push(")");
            by_last_login = true;
        } else if let Some(scope) = vendor_scope {
            qb.push(" AND vendor_id = ");
            qb.push_bind(scope);
        }
        by_last_login
    }

    // ————————————————————— 单行 —————————————————————

    /// 按主键取供应商。
    pub async fn find_by_id(&self, vendor_id: i32) -> Result<Option<Vendor>> {
        sqlx::query_as::<_, Vendor>(&format!("{VENDOR_SELECT} WHERE vendor_id = $1 LIMIT 1"))
            .bind(vendor_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 写入 —————————————————————

    /// 创建供应商（对应 Java `VendorServiceImpl#create` 的 vendor 落库段）。
    ///
    /// 返回自增 `vendor_id`。
    pub async fn insert(
        &self,
        vendor_logo: Option<&str>,
        contact_mobile: Option<&str>,
        vendor_data: Option<&str>,
        person_data: Option<&str>,
        status: i32,
        r#type: Option<i32>,
        vendor_name: Option<&str>,
        kefu_phone: Option<&str>,
        now: i32,
    ) -> Result<i32> {
        let out: i32 = sqlx::query_scalar(
            "INSERT INTO vendor (source_type, shop_id, vendor_logo, add_time, contact_mobile, vendor_data, person_data, status, type, vendor_name, kefu_phone) VALUES (2, 0, $1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING vendor_id",
        )
        .bind(vendor_logo)
        .bind(now)
        .bind(contact_mobile)
        .bind(vendor_data)
        .bind(person_data)
        .bind(status)
        .bind(r#type)
        .bind(vendor_name)
        .bind(kefu_phone)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(out)
    }

    /// 更新供应商设置（对应 Java `VendorServiceImpl#setting`）：
    /// status / kefu_phone / ship_info / receive_info / vendor_data。
    pub async fn update_setting(
        &self,
        vendor_id: i32,
        status: i32,
        kefu_phone: Option<&str>,
        ship_info: Option<&str>,
        receive_info: Option<&str>,
        vendor_data: Option<&str>,
    ) -> Result<u64> {
        let out = sqlx::query(
            "UPDATE vendor SET status = $1, kefu_phone = $2, ship_info = $3, receive_info = $4, vendor_data = COALESCE($5, vendor_data) WHERE vendor_id = $6",
        )
        .bind(status)
        .bind(kefu_phone)
        .bind(ship_info)
        .bind(receive_info)
        .bind(vendor_data)
        .bind(vendor_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(out.rows_affected())
    }

    // ————————————————————— admin_user_vendor —————————————————————

    /// 按 vendor_id 取绑定关系（服务层筛 is_admin）。
    pub async fn find_admin_user_vendors(&self, vendor_id: i32) -> Result<Vec<AdminUserVendor>> {
        sqlx::query_as::<_, AdminUserVendor>(
            "SELECT id, admin_id, user_id, vendor_id, username, email, avatar, auth_list, is_using, is_admin, add_time, role_id FROM admin_user_vendor WHERE vendor_id = $1",
        )
        .bind(vendor_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 admin_id 查供应商绑定（`GET vendor/vendor/currentDetail` 用：
    /// 当前登录管理员绑定的供应商）。
    pub async fn find_vendor_id_by_admin(&self, admin_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT vendor_id FROM admin_user_vendor WHERE admin_id = $1 LIMIT 1")
                .bind(admin_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 插入绑定行（对应 Java `VendorServiceImpl#create`）。
    pub async fn insert_admin_user_vendor(&self, row: &AdminUserVendor) -> Result<i32> {
        let out: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user_vendor (admin_id, user_id, vendor_id, username, email, avatar, auth_list, is_using, is_admin, add_time, role_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
        )
        .bind(row.admin_id)
        .bind(row.user_id)
        .bind(row.vendor_id)
        .bind(&row.username)
        .bind(&row.email)
        .bind(&row.avatar)
        .bind(&row.auth_list)
        .bind(row.is_using)
        .bind(row.is_admin)
        .bind(row.add_time)
        .bind(row.role_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(out)
    }

    // ————————————————————— 账号解析 —————————————————————

    /// 按登录名精确查管理员 id 集合（Java `adminUserService.getBindMainVendorIds` 的第一段）。
    pub async fn find_admin_ids_by_username(&self, username: &str) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT admin_id FROM admin_user WHERE username = $1 LIMIT 20")
                .bind(username)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 按管理员 id 集合查其绑定的 vendor id 集合。
    pub async fn find_vendor_ids_by_admins(&self, admin_ids: &[i32]) -> Result<Vec<i32>> {
        if admin_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT DISTINCT vendor_id FROM admin_user_vendor WHERE admin_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in admin_ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 批量取供应商名称（商品列表 / 导出附表用）。
    pub async fn find_vendor_names(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT vendor_id, vendor_name FROM vendor WHERE vendor_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, Option<String>)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, name)| (id, name.unwrap_or_default()))
            .collect())
    }

    /// 批量取类目名称（导出用；Java 导出为多级 `|` 拼接，最小实现取本级名称）。
    pub async fn find_category_names(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
            "SELECT category_id, category_name FROM category WHERE category_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 批量取品牌名称（导出用）。
    pub async fn find_brand_names(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, sqlx::Postgres> =
            QueryBuilder::new("SELECT brand_id, brand_name FROM brand WHERE brand_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        drop(sep);
        qb.push(")");
        let rows: Vec<(i32, String)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }
}

/// SQLx 错误 → 统一错误类型。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("供应商不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
