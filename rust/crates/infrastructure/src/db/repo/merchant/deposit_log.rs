//! 店铺保证金交易明细 域仓储（merchant::deposit_log）—— wave9。
//!
//! 表 `shop_deposit_log` + 关联 `shop.shop_title`。运行时 sqlx。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::merchant::deposit_log::DepositLogListQuery;
use nextrade_core::{Error, Result};

/// 明细行。
#[derive(Debug, Clone, FromRow)]
pub struct DepositLogRow {
    /// 明细 ID
    pub shop_deposit_log_id: i32,
    /// 归属主体类型
    pub owner_type: i32,
    /// 归属主体 ID
    pub owner_id: i32,
    /// 变动类型
    pub r#type: i32,
    /// 变动金额
    pub change_amount: Decimal,
    /// 变动前保证金
    pub before_paid: Decimal,
    /// 变动后保证金
    pub after_paid: Decimal,
    /// 关联 ID
    pub related_id: i32,
    /// 订单 ID
    pub order_id: i32,
    /// 管理员 ID
    pub admin_id: i32,
    /// 备注
    pub remark: String,
    /// 变动时间（秒）
    pub add_time: i64,
    /// 归属主体名称（shop 标题；非店铺主体为空）
    pub shop_title: Option<String>,
    /// 店铺 ID（= owner_id when owner_type=1）
    pub shop_id: Option<i32>,
}

/// 明细仓储。
#[derive(Clone)]
pub struct DepositLogRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl DepositLogRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 分页（Java `DepositLogController.list`）。
    pub async fn paginate(
        &self,
        q: &DepositLogListQuery,
    ) -> Result<(u64, Vec<DepositLogRow>)> {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();
        if let Some(t) = q.owner_type.or(q.subject_type) {
            conds.push("l.owner_type = ?".to_string());
            binds.push(Bind::I32(t));
        }
        if let Some(t) = q.r#type {
            conds.push("l.type = ?".to_string());
            binds.push(Bind::I32(t));
        }
        if let Some(s) = q.start_time {
            conds.push("l.add_time >= ?".to_string());
            binds.push(Bind::I64(s));
        }
        if let Some(e) = q.end_time {
            conds.push("l.add_time <= ?".to_string());
            binds.push(Bind::I64(e));
        }
        if let Some(kw) = q.shop_title.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            conds.push("s.shop_title ILIKE ?".to_string());
            binds.push(Bind::Str(format!("%{kw}%")));
        }
        if let Some(kw) = q.trade_no.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            conds.push("(l.related_id = ? OR l.order_id = ?)".to_string());
            if let Ok(v) = kw.parse::<i32>() {
                binds.push(Bind::I32(v));
                binds.push(Bind::I32(v));
            } else {
                binds.push(Bind::I32(-1));
                binds.push(Bind::I32(-1));
            }
        }
        let where_sql = if conds.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conds.join(" AND "))
        };
        let page = q.page_query();
        let size = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * size;
        let base = crate::db::to_pg_placeholders(&format!(
            " FROM shop_deposit_log l LEFT JOIN shop s ON s.shop_id = l.owner_id AND l.owner_type = 1{where_sql}"
        ));
        let count_sql = format!("SELECT COUNT(*){base}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            cq = match b {
                Bind::I32(v) => cq.bind(*v),
                Bind::I64(v) => cq.bind(*v),
                Bind::Str(v) => cq.bind(v.clone()),
            };
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?.0;
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT l.shop_deposit_log_id, l.owner_type, l.owner_id, l.type, l.change_amount, \
             l.before_paid, l.after_paid, l.related_id, l.order_id, l.admin_id, l.remark, l.add_time, \
             s.shop_title, s.shop_id{base} ORDER BY l.shop_deposit_log_id DESC LIMIT ? OFFSET ?"
        ));
        let mut rq = sqlx::query_as::<_, DepositLogRow>(&rows_sql);
        for b in &binds {
            rq = match b {
                Bind::I32(v) => rq.bind(*v),
                Bind::I64(v) => rq.bind(*v),
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

    /// 明细详情（Java `DepositLogController.detail`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<DepositLogRow>> {
        sqlx::query_as::<_, DepositLogRow>(
            "SELECT l.shop_deposit_log_id, l.owner_type, l.owner_id, l.type, l.change_amount, \
             l.before_paid, l.after_paid, l.related_id, l.order_id, l.admin_id, l.remark, l.add_time, \
             s.shop_title, s.shop_id FROM shop_deposit_log l \
             LEFT JOIN shop s ON s.shop_id = l.owner_id AND l.owner_type = 1 \
             WHERE l.shop_deposit_log_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }
}

enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
}
