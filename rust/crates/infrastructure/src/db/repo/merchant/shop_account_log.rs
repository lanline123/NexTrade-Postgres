//! 店铺账户 域仓储（merchant::shop_account_log）—— wave9。
//!
//! 表：`shop`（店铺余额汇总）、`user_balance_log`（用户余额/佣金流水）。
//! 运行时 sqlx。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::merchant::shop_account_log::UserAccountLogQuery;
use nextrade_core::{Error, Result};

/// 店铺余额行。
#[derive(Debug, Clone, FromRow)]
pub struct ShopAccountRow {
    /// 店铺 ID
    pub shop_id: i32,
    /// 店铺名称
    pub shop_title: String,
    /// 店铺余额
    pub shop_money: Decimal,
    /// 冻结金额
    pub frozen_money: Decimal,
    /// 状态
    pub status: i32,
}

/// 用户流水行。
#[derive(Debug, Clone, FromRow)]
pub struct UserAccountLogRow {
    /// 流水 ID
    pub log_id: i32,
    /// 用户 ID
    pub user_id: i32,
    /// 变动前余额
    pub before_balance: Decimal,
    /// 本次变动金额
    pub balance: Decimal,
    /// 变动后余额
    pub after_balance: Decimal,
    /// 冻结余额
    pub frozen_balance: Decimal,
    /// 钱包类型
    pub wallet_type: i32,
    /// 变动说明
    pub change_desc: String,
    /// 变动类型
    pub change_type: i32,
    /// 变动时间（秒）
    pub change_time: i32,
}

/// 店铺账户仓储。
#[derive(Clone)]
pub struct ShopAccountLogRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl ShopAccountLogRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 店铺余额汇总（Java `AccountController.index`）。
    pub async fn find_shop(&self, shop_id: i32) -> Result<Option<ShopAccountRow>> {
        sqlx::query_as::<_, ShopAccountRow>(
            "SELECT shop_id, shop_title, shop_money, frozen_money, status FROM shop \
             WHERE shop_id = ? LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 用户钱包流水分页（Java `/api/user/account/list` / `commissionList`）。
    pub async fn paginate_user_logs(
        &self,
        user_id: i32,
        wallet_type: i32,
        q: &UserAccountLogQuery,
    ) -> Result<(u64, Vec<UserAccountLogRow>)> {
        let mut conds = vec!["user_id = ?".to_string(), "wallet_type = ?".to_string()];
        let mut binds: Vec<Bind> = vec![Bind::I32(user_id), Bind::I32(wallet_type)];
        if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            conds.push("change_desc LIKE ?".to_string());
            binds.push(Bind::Str(format!("%{kw}%")));
        }
        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        let page = q.page_query();
        let size = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * size;
        let count_sql = format!("SELECT COUNT(*) FROM user_balance_log{where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            cq = match b {
                Bind::I32(v) => cq.bind(*v),
                Bind::Str(v) => cq.bind(v.clone()),
            };
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?.0;
        let rows_sql = format!(
            "SELECT log_id, user_id, before_balance, balance, after_balance, frozen_balance, \
             wallet_type, change_desc, change_type, change_time FROM user_balance_log{where_sql} \
             ORDER BY log_id DESC LIMIT ? OFFSET ?"
        );
        let mut rq = sqlx::query_as::<_, UserAccountLogRow>(&rows_sql);
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
}

enum Bind {
    I32(i32),
    Str(String),
}
