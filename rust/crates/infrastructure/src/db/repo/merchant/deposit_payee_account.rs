//! 保证金收款账户 域仓储（merchant::deposit_payee_account）—— wave9。
//!
//! 表 `shop_deposit_payee_account`。运行时 sqlx。

use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::merchant::deposit_payee_account::{
    DepositPayeeAccountDTO, DepositPayeeAccountListQuery,
};
use nextrade_core::{Error, Result};

/// 账户行。
#[derive(Debug, Clone, FromRow)]
pub struct DepositPayeeAccountRow {
    /// 账户 ID
    pub shop_deposit_payee_account_id: i32,
    /// 账户类型
    pub account_type: i32,
    /// 账户名称
    pub account_name: String,
    /// 账户号
    pub account_no: String,
    /// 银行名称
    pub bank_name: String,
    /// 银行支行
    pub bank_branch: String,
    /// 状态
    pub status: i32,
    /// 排序
    pub sort_order: i32,
    /// 创建时间（秒）。列为 int4，必须用 i32 解码：此前声明 i64，
    /// 有行时 sqlx 解码 int4→i64 直接报错，列表/详情恒 500（空表掩盖）。
    pub add_time: i32,
    /// 更新时间（秒）。同上，int4 列。
    pub update_time: i32,
}

/// 账户仓储。
#[derive(Clone)]
pub struct DepositPayeeAccountRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl DepositPayeeAccountRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页。
    pub async fn paginate(
        &self,
        q: &DepositPayeeAccountListQuery,
    ) -> Result<(u64, Vec<DepositPayeeAccountRow>)> {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();
        if let Some(t) = q.account_type {
            conds.push("account_type = ?".to_string());
            binds.push(Bind::I32(t));
        }
        if let Some(s) = q.status {
            conds.push("status = ?".to_string());
            binds.push(Bind::I32(s));
        }
        if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            conds.push("(account_name ILIKE ? OR account_no ILIKE ?)".to_string());
            binds.push(Bind::Str(format!("%{kw}%")));
            binds.push(Bind::Str(format!("%{kw}%")));
        }
        let where_sql = if conds.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conds.join(" AND "))
        };
        let page = q.page_query();
        let size = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * size;
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop_deposit_payee_account{where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            cq = match b {
                Bind::I32(v) => cq.bind(*v),
                Bind::Str(v) => cq.bind(v.clone()),
            };
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?.0;
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT shop_deposit_payee_account_id, account_type, account_name, account_no, \
             bank_name, bank_branch, status, sort_order, add_time, update_time \
             FROM shop_deposit_payee_account{where_sql} ORDER BY sort_order ASC, \
             shop_deposit_payee_account_id DESC LIMIT ? OFFSET ?"
        ));
        let mut rq = sqlx::query_as::<_, DepositPayeeAccountRow>(&rows_sql);
        for b in &binds {
            rq = match b {
                Bind::I32(v) => rq.bind(*v),
                Bind::Str(v) => rq.bind(v.clone()),
            };
        }
        let rows = rq
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total).unwrap_or(0), rows))
    }

    /// 详情。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<DepositPayeeAccountRow>> {
        sqlx::query_as::<_, DepositPayeeAccountRow>(
            "SELECT shop_deposit_payee_account_id, account_type, account_name, account_no, \
             bank_name, bank_branch, status, sort_order, add_time, update_time \
             FROM shop_deposit_payee_account WHERE shop_deposit_payee_account_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入。
    pub async fn insert(&self, dto: &DepositPayeeAccountDTO, now: i64) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO shop_deposit_payee_account \
             (account_type, account_name, account_no, bank_name, bank_branch, status, sort_order, add_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING shop_deposit_payee_account_id",
        )
        .bind(dto.account_type.unwrap_or(1))
        .bind(dto.account_name.clone().unwrap_or_default())
        .bind(dto.account_no.clone().unwrap_or_default())
        .bind(dto.bank_name.clone().unwrap_or_default())
        .bind(dto.bank_branch.clone().unwrap_or_default())
        .bind(dto.status.unwrap_or(1))
        .bind(dto.sort_order.unwrap_or(0))
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新。
    pub async fn update(&self, dto: &DepositPayeeAccountDTO, now: i64) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE shop_deposit_payee_account SET account_type = $1, account_name = $2, \
             account_no = $3, bank_name = $4, bank_branch = $5, status = $6, sort_order = $7, \
             update_time = $8 WHERE shop_deposit_payee_account_id = $9",
        )
        .bind(dto.account_type.unwrap_or(1))
        .bind(dto.account_name.clone().unwrap_or_default())
        .bind(dto.account_no.clone().unwrap_or_default())
        .bind(dto.bank_name.clone().unwrap_or_default())
        .bind(dto.bank_branch.clone().unwrap_or_default())
        .bind(dto.status.unwrap_or(1))
        .bind(dto.sort_order.unwrap_or(0))
        .bind(now)
        .bind(dto.id.unwrap_or(0))
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除。
    pub async fn delete(&self, id: i32) -> Result<u64> {
        let r = sqlx::query(
            "DELETE FROM shop_deposit_payee_account WHERE shop_deposit_payee_account_id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

enum Bind {
    I32(i32),
    Str(String),
}

#[cfg(test)]
mod tests {
    /// 回归不变式：`shop_deposit_payee_account.add_time` / `update_time` 是
    /// int4 列，行结构必须以 i32 解码。曾声明 i64，有行时 sqlx 解码
    /// int4→i64 报类型不兼容，`payeeAccount/list` 有行即 500（空表长期掩盖）。
    ///
    /// 扫描范围限定在行结构定义体内（避免整文件扫描被断言自身的字面量污染），
    /// 且只匹配字段声明形态。
    #[test]
    fn row_time_fields_decode_as_i32_for_int4_columns() {
        let src = include_str!("deposit_payee_account.rs");
        let start = src
            .find("pub struct DepositPayeeAccountRow")
            .expect("row struct must exist");
        let body = &src[start..src[start..].find("\n}\n").map(|e| start + e).unwrap_or(src.len())];
        assert!(
            body.contains("pub add_time: i32"),
            "add_time 必须以 i32 解码 int4 列"
        );
        assert!(
            body.contains("pub update_time: i32"),
            "update_time 必须以 i32 解码 int4 列"
        );
        assert!(
            !body.contains("pub add_time: i64") && !body.contains("pub update_time: i64"),
            "时间字段不得再以 i64 声明（int4 列解码必炸）"
        );
    }
}
