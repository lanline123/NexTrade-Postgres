//! 入驻申请域仓储（merchant::merchant_apply）。
//!
//! 表/列来自权威 schema `java/database/pure.sql`
//! （核对：`cd rust && python tools/schema_parser.py --table merchant_apply`）：
//! `merchant_apply`：`merchant_apply_id` / `user_id` / `add_time` / `shop_title` /
//! `status` / `type` / `contact_name` / `contact_mobile` / `audit_time` /
//! `audit_admin_id` / `shop_data` / `base_data` / `merchant_data` / `company_name` /
//! `corporate_name`。
//!
//! 对应 Java：
//! - `MerchantApplyServiceImpl#list`（`page` + `eq(status)` + `like(companyName|contactName)`）
//! - `#detail`（`getById` + `userMapper.selectById` + 审核管理员名称）
//! - `#audit`（更新状态/备注/时间/审核人；通过时创建 `merchant` 行）
//! - `BaseServiceImpl#del/#updateField/#batch`
//!
//! 运行时 `sqlx`（**不用** `query!` 宏）；每条 SQL 单行字面量。

use sqlx::PgPool;
use nextrade_core::entities::MerchantApply;
use nextrade_core::models::merchant::merchant_apply::{apply_field_to_column, MerchantApplyListQuery};
use nextrade_core::{Error, Result};

/// Java `MerchantApply` 的 15 列（顺序与实体字段一致）。
const APPLY_COLUMNS: &str = "merchant_apply_id, user_id, add_time, shop_title, status, type, contact_name, contact_mobile, audit_time, audit_admin_id, shop_data, base_data, merchant_data, company_name, corporate_name, audit_remark";

/// 入驻申请域仓储（持有连接池）。
#[derive(Clone)]
pub struct MerchantApplyRepository {
    pool: PgPool,
}

impl MerchantApplyRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页查询（Java `this.page(page, queryWrapper)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn paginate(
        &self,
        q: &MerchantApplyListQuery,
    ) -> Result<(u64, Vec<MerchantApply>)> {
        let (where_sql, like) = build_where(q);
        let (page, size) = q.page_params();
        let offset = (page - 1) * size;
        let column = apply_field_to_column(q.sort_field.as_deref());
        let dir = q.sort_dir();

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM merchant_apply{where_sql}"));
        let total: (i64,) = if let Some(kw) = like.as_deref() {
            sqlx::query_as(&count_sql)
                .bind(kw)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as(&count_sql)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?
        };

        let list_sql =
            crate::db::to_pg_placeholders(&format!("SELECT {APPLY_COLUMNS} FROM merchant_apply{where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"));
        let mut q_ = sqlx::query_as::<_, MerchantApply>(&list_sql);
        if let Some(kw) = like.as_deref() {
            q_ = q_.bind(kw);
        }
        let rows = q_
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 按主键查询（Java `getById`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<MerchantApply>> {
        sqlx::query_as::<_, MerchantApply>(&format!(
            "SELECT {APPLY_COLUMNS} FROM merchant_apply WHERE merchant_apply_id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取会员展示信息（Java `userMapper.selectBatchIds`）。
    ///
    /// 返回 `(user_id, username, nickname, mobile, avatar)`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_users(
        &self,
        user_ids: &[i32],
    ) -> Result<Vec<(i32, String, Option<String>, String, String)>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_id, username, nickname, mobile, avatar FROM \"user\" WHERE user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String, Option<String>, String, String)>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取管理员用户名（Java `fetchAdminNameMap`：`audit_admin_id → username`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn admin_usernames(&self, admin_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let ids: Vec<i32> = admin_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql =
            crate::db::to_pg_placeholders(&format!("SELECT admin_id, username FROM admin_user WHERE admin_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 更新审核结果（Java `updateById(merchantApply)` 的审核字段子集）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_audit(
        &self,
        id: i32,
        status: i32,
        audit_remark: Option<&str>,
        audit_time: i32,
        audit_admin_id: i32,
    ) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE merchant_apply SET status = $1, audit_remark = $2, audit_time = $3, audit_admin_id = $4 WHERE merchant_apply_id = $5",
        )
        .bind(status)
        .bind(audit_remark)
        .bind(audit_time)
        .bind(audit_admin_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 审核通过：创建 `merchant` 行（Java `merchantService.save` 的字段子集）。
    ///
    /// 返回新 `merchant_id`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_merchant_from_apply(
        &self,
        merchant_apply_id: i32,
        user_id: i32,
        add_time: i32,
        base_data: Option<&str>,
        shop_data: Option<&str>,
        merchant_data: Option<&str>,
        r#type: i32,
        company_name: Option<&str>,
        corporate_name: Option<&str>,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO merchant (merchant_apply_id, user_id, add_time, base_data, shop_data, merchant_data, status, type, company_name, corporate_name) VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, $9) RETURNING merchant_id",
        )
        .bind(merchant_apply_id)
        .bind(user_id)
        .bind(add_time)
        .bind(base_data)
        .bind(shop_data)
        .bind(merchant_data)
        .bind(r#type)
        .bind(company_name)
        .bind(corporate_name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 删除（Java `removeById` / `BaseServiceImpl#del`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM merchant_apply WHERE merchant_apply_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除（Java `BaseServiceImpl#batch` 的 `del` 分支）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "DELETE FROM merchant_apply WHERE merchant_apply_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// `BaseServiceImpl#updateField`：`SET <column> = ? WHERE merchant_apply_id = ?`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_field(
        &self,
        id: i32,
        column: &'static str,
        val: Option<&str>,
    ) -> Result<bool> {
        let sql = format!("UPDATE merchant_apply SET {column} = $1 WHERE merchant_apply_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

/// 拼装 WHERE（含前导空格）+ 关键字绑定值。
///
/// Java：
/// `eq(status != null && status > -1, status)`
/// 且 `and(isNotEmpty(keyword), w -> w.like(companyName, kw).or().like(contactName, kw))`。
#[must_use]
fn build_where(q: &MerchantApplyListQuery) -> (String, Option<String>) {
    let mut conds: Vec<String> = Vec::new();
    if let Some(status) = q.status.filter(|s| *s > -1) {
        conds.push(format!("status = {status}"));
    }
    let like = q
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|kw| format!("%{kw}%"));
    if like.is_some() {
        conds.push("(company_name LIKE ? OR contact_name LIKE ?)".to_string());
    }
    if conds.is_empty() {
        (String::new(), None)
    } else {
        (format!(" WHERE {}", conds.join(" AND ")), like)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn where_clause_matches_java_wrapper() {
        let q = MerchantApplyListQuery::default();
        assert_eq!(build_where(&q), (String::new(), None));

        // status = -1 表示全部
        let q = MerchantApplyListQuery {
            status: Some(-1),
            ..Default::default()
        };
        assert_eq!(build_where(&q), (String::new(), None));

        let q = MerchantApplyListQuery {
            status: Some(10),
            ..Default::default()
        };
        assert_eq!(build_where(&q).0, " WHERE status = 10");

        // 关键字匹配 company_name 或 contact_name
        let q = MerchantApplyListQuery {
            username: Some("abc".into()),
            status: Some(1),
            ..Default::default()
        };
        let (sql, like) = build_where(&q);
        assert_eq!(
            sql,
            " WHERE status = 1 AND (company_name LIKE ? OR contact_name LIKE ?)"
        );
        assert_eq!(like.as_deref(), Some("%abc%"));
    }
}
