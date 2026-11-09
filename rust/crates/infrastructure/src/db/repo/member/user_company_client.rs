//! 用户实名认证仓储（member::user_company_client）。表 `user_company`。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct UserCompanyClientRepository {
    pool: PgPool,
}

impl UserCompanyClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 当前用户最新一条申请：`(id, user_id, type, contact_name, contact_mobile,
    /// company_name, company_data, status, audit_remark, audit_time, add_time)`。
    #[allow(clippy::type_complexity)]
    pub async fn latest(
        &self,
        user_id: i32,
    ) -> Result<Option<(i32, i32, i32, String, String, String, Option<String>, i32, String, Option<i32>, Option<i32>)>> {
        sqlx::query_as(
            "SELECT id, user_id, type, contact_name, contact_mobile, company_name, company_data, status, audit_remark, audit_time, add_time FROM user_company WHERE user_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 我的全部申请（Java `myApply`）。
    #[allow(clippy::type_complexity)]
    pub async fn list_by_user(
        &self,
        user_id: i32,
    ) -> Result<Vec<(i32, i32, i32, String, String, String, Option<String>, i32, String, Option<i32>, Option<i32>)>> {
        sqlx::query_as(
            "SELECT id, user_id, type, contact_name, contact_mobile, company_name, company_data, status, audit_remark, audit_time, add_time FROM user_company WHERE user_id = $1 ORDER BY id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 是否存在待审核申请。
    pub async fn has_pending(&self, user_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM user_company WHERE user_id = $1 AND status = 0 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 插入申请，返回 id。
    pub async fn insert(
        &self,
        user_id: i32,
        r#type: i32,
        contact_name: &str,
        contact_mobile: &str,
        company_name: &str,
        company_data: Option<&str>,
        now: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_company (user_id, type, contact_name, contact_mobile, company_name, company_data, status, audit_remark, add_time) VALUES ($1, $2, $3, $4, $5, $6, 0, '', $7) RETURNING id",
        )
        .bind(user_id)
        .bind(r#type)
        .bind(contact_name)
        .bind(contact_mobile)
        .bind(company_name)
        .bind(company_data)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
