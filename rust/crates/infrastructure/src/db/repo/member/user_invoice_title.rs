//! 用户常用发票抬头仓储（member::user_invoice_title）。表 `user_invoice_title`。
//!
//! 默认唯一性：写路径全部走事务——先 `UPDATE ... SET is_default = 0`
//! 清本用户默认，再置目标行；表上另有部分唯一索引兜底。
//! 删除默认后递补：同事务内把「最近更新的一条」未删除抬头置默认。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 抬头行（与 VO 字段同序）。
pub type TitleRow = (i32, i32, i32, String, String, String, String, String, String, i32, i32, i32);

#[derive(Clone)]
pub struct UserInvoiceTitleRepository {
    pool: PgPool,
}

impl UserInvoiceTitleRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const COLUMNS: &'static str = "id, user_id, title_type, title_name, tax_no, company_address, company_phone, bank_name, bank_account, is_default, add_time, update_time";

    /// 本人全部未删除抬头（默认在前，其余按更新时间倒序）。
    pub async fn list(&self, user_id: i32) -> Result<Vec<TitleRow>> {
        let sql = format!(
            "SELECT {} FROM user_invoice_title WHERE user_id = $1 AND is_del = 0 ORDER BY is_default DESC, update_time DESC, id DESC",
            Self::COLUMNS
        );
        sqlx::query_as::<_, TitleRow>(&sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 本人未删除抬头数（上限校验）。
    pub async fn count_active(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_invoice_title WHERE user_id = $1 AND is_del = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 按主键 + 用户查未删除抬头（归属校验）。
    pub async fn find_own(&self, id: i32, user_id: i32) -> Result<Option<TitleRow>> {
        let sql = format!(
            "SELECT {} FROM user_invoice_title WHERE id = $1 AND user_id = $2 AND is_del = 0 LIMIT 1",
            Self::COLUMNS
        );
        sqlx::query_as::<_, TitleRow>(&sql)
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新建抬头。`make_default=true` 时同事务清旧默认；首条由服务层传 true。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        user_id: i32,
        title_type: i32,
        title_name: &str,
        tax_no: &str,
        company_address: &str,
        company_phone: &str,
        bank_name: &str,
        bank_account: &str,
        make_default: bool,
        now: i32,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if make_default {
            sqlx::query(
                "UPDATE user_invoice_title SET is_default = 0 WHERE user_id = $1 AND is_del = 0",
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        let id: i32 = sqlx::query_scalar(
            "INSERT INTO user_invoice_title (user_id, title_type, title_name, tax_no, company_address, company_phone, bank_name, bank_account, is_default, is_del, add_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, $10, $10) RETURNING id",
        )
        .bind(user_id)
        .bind(title_type)
        .bind(title_name)
        .bind(tax_no)
        .bind(company_address)
        .bind(company_phone)
        .bind(bank_name)
        .bind(bank_account)
        .bind(i32::from(make_default))
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(id)
    }

    /// 更新抬头（仅本人未删除行）。`make_default=true` 时同事务切换默认。
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        id: i32,
        user_id: i32,
        title_type: i32,
        title_name: &str,
        tax_no: &str,
        company_address: &str,
        company_phone: &str,
        bank_name: &str,
        bank_account: &str,
        make_default: bool,
        now: i32,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if make_default {
            sqlx::query(
                "UPDATE user_invoice_title SET is_default = 0 WHERE user_id = $1 AND is_del = 0",
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        let r = sqlx::query(
            "UPDATE user_invoice_title SET title_type = $1, title_name = $2, tax_no = $3, company_address = $4, company_phone = $5, bank_name = $6, bank_account = $7, is_default = CASE WHEN $8 THEN 1 ELSE is_default END, update_time = $9 WHERE id = $10 AND user_id = $11 AND is_del = 0",
        )
        .bind(title_type)
        .bind(title_name)
        .bind(tax_no)
        .bind(company_address)
        .bind(company_phone)
        .bind(bank_name)
        .bind(bank_account)
        .bind(make_default)
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 设为默认（同事务清旧默认）。
    pub async fn set_default(&self, id: i32, user_id: i32, now: i32) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query(
            "UPDATE user_invoice_title SET is_default = 0 WHERE user_id = $1 AND is_del = 0",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        let r = sqlx::query(
            "UPDATE user_invoice_title SET is_default = 1, update_time = $1 WHERE id = $2 AND user_id = $3 AND is_del = 0",
        )
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 软删抬头；若删的是默认，同事务递补最近更新的一条未删除抬头为默认。
    pub async fn soft_delete(&self, id: i32, user_id: i32, now: i32) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let was_default: Option<(i32,)> = sqlx::query_as(
            "SELECT is_default FROM user_invoice_title WHERE id = $1 AND user_id = $2 AND is_del = 0 LIMIT 1",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let Some((was_default,)) = was_default else {
            tx.rollback().await.map_err(map_err)?;
            return Ok(false);
        };
        sqlx::query(
            "UPDATE user_invoice_title SET is_del = 1, is_default = 0, update_time = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        if was_default == 1 {
            // 递补：最近更新的一条未删除抬头（无剩余则不递补）
            sqlx::query(
                "UPDATE user_invoice_title SET is_default = 1 WHERE id = (SELECT id FROM user_invoice_title WHERE user_id = $1 AND is_del = 0 ORDER BY update_time DESC, id DESC LIMIT 1)",
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(true)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_type_exists() {
        assert!(std::any::type_name::<UserInvoiceTitleRepository>().contains("UserInvoiceTitle"));
    }
}
