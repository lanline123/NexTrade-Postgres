//! 保证金账户域仓储（merchant::deposit_account）。
//!
//! 表：`shop_deposit`（账户）/ `shop_deposit_log`（流水）/ `shop_deposit_adjust_log`（调整记录）。
//! 对应 Java `DepositAccountAdminServiceImpl`（47KB，完整生命周期逻辑见服务层文档）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::models::merchant::deposit_account::{
    DepositAccountListQuery, DepositDeductDTO, DepositAdjustDTO,
};
use nextrade_core::{Error, Result};

/// 账户分页行。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AccountRow {
    pub shop_deposit_id: i32,
    pub owner_type: i32,
    pub owner_id: i32,
    pub paid: Decimal,
    pub required_manual: Option<Decimal>,
    pub required_snapshot: Decimal,
    pub penalty_status: i32,
    pub add_time: i32,
}

/// 流水行。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LogRow {
    pub shop_deposit_log_id: i32,
    pub owner_type: i32,
    pub owner_id: i32,
    pub r#type: i32,
    pub change_amount: Decimal,
    pub before_paid: Decimal,
    pub after_paid: Decimal,
    pub related_id: i32,
    pub order_id: i32,
    pub admin_id: i32,
    pub remark: Option<String>,
    pub add_time: i32,
}

/// 调整记录行。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdjustLogRow {
    pub shop_deposit_adjust_id: i32,
    pub owner_type: i32,
    pub owner_id: i32,
    pub before_required: Decimal,
    pub after_required: Decimal,
    pub source: i32,
    pub admin_id: i32,
    pub reason: Option<String>,
    pub add_time: i32,
}

/// 保证金账户域仓储。
#[derive(Clone)]
pub struct DepositAccountRepository {
    pool: PgPool,
}

impl DepositAccountRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 账户分页（Java `list`；`deposit_status` 过滤在服务层计算后内存过滤）。
    pub async fn paginate(&self, q: &DepositAccountListQuery) -> Result<(u64, Vec<AccountRow>)> {
        let (page, size) = q.page_params();
        let mut conds: Vec<String> = Vec::new();
        if let Some(t) = q.owner_type {
            conds.push(format!("owner_type = {t}"));
        }
        if let Some(id) = q.owner_name.as_deref().filter(|s| !s.is_empty()).map(|_| None::<i32>) {
            let _ = id;
        }
        let where_sql = if conds.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conds.join(" AND "))
        };
        let total: (i64,) =
            sqlx::query_as(&format!("SELECT COUNT(*) FROM shop_deposit{where_sql}"))
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AccountRow>(&crate::db::to_pg_placeholders(&format!(
            "SELECT shop_deposit_id, owner_type, owner_id, paid, required_manual, required_snapshot, penalty_status, add_time FROM shop_deposit{where_sql} ORDER BY shop_deposit_id DESC LIMIT ? OFFSET ?"
        )))
        .bind(i64::try_from(size).unwrap_or(15))
        .bind(i64::try_from((page - 1) * size).unwrap_or(0))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 按持有者查账户（Java `getByOwner`）。
    /// 全部保证金账户（Java `shopDepositMapper.selectList(null)`）。
    pub async fn list_all_accounts(
        &self,
    ) -> Result<Vec<nextrade_core::entities::deposit::ShopDeposit>> {
        sqlx::query_as::<_, nextrade_core::entities::deposit::ShopDeposit>(
            "SELECT * FROM shop_deposit",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 更新惩罚状态（0 正常 / 1 限制上架 / 2 全店下架）。
    pub async fn update_penalty_status(
        &self,
        shop_deposit_id: i32,
        penalty_status: i32,
        now: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE shop_deposit SET penalty_status = $1, update_time = $2 WHERE shop_deposit_id = $3",
        )
        .bind(penalty_status)
        .bind(now)
        .bind(shop_deposit_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 恢复保证金惩罚状态并将下架商品重新上架（Java DepositLifecycleServiceImpl#restore）。
    pub async fn restore_account_penalty(
        &self,
        owner_type: i32,
        owner_id: i32,
        now: i32,
    ) -> Result<u64> {
        sqlx::query(
            "UPDATE shop_deposit SET penalty_status = 0, collect_due_time = 0, supplement_due_time = 0, update_time = $1 WHERE owner_type = $2 AND owner_id = $3",
        )
        .bind(now)
        .bind(owner_type)
        .bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM shop_deposit_offshelf_log WHERE owner_type = $1 AND owner_id = $2 AND status = 1",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let mut restored_count = 0u64;
        if !rows.is_empty() {
            let product_ids: Vec<i32> = rows.into_iter().map(|(id,)| id).collect();
            if owner_type == 1 {
                for chunk in product_ids.chunks(100) {
                    let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    let sql = format!(
                        "UPDATE product SET product_status = 1 WHERE shop_id = $1 AND is_delete = 0 AND product_id IN ({})",
                        placeholders
                    );
                    let sql = crate::db::to_pg_placeholders(&sql);
                    let mut q = sqlx::query(&sql).bind(owner_id);
                    for id in chunk {
                        q = q.bind(*id);
                    }
                    if let Ok(res) = q.execute(&self.pool).await {
                        restored_count += res.rows_affected();
                    }
                }
            } else {
                for chunk in product_ids.chunks(100) {
                    let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    let sql = format!(
                        "UPDATE vendor_product SET product_state = 1 WHERE is_del = 0 AND id IN ({})",
                        placeholders
                    );
                    let sql = crate::db::to_pg_placeholders(&sql);
                    let mut q = sqlx::query(&sql);
                    for id in chunk {
                        q = q.bind(*id);
                    }
                    if let Ok(res) = q.execute(&self.pool).await {
                        restored_count += res.rows_affected();
                    }
                }
            }

            let _ = sqlx::query(
                "UPDATE shop_deposit_offshelf_log SET status = 2, restore_time = $1 WHERE owner_type = $2 AND owner_id = $3 AND status = 1",
            )
            .bind(now)
            .bind(owner_type)
            .bind(owner_id)
            .execute(&self.pool)
            .await;
        }

        Ok(restored_count)
    }

    /// 记录惩罚下架日志并物理下架在售商品（Java DepositLifecycleServiceImpl#offShelfAllProducts）。
    pub async fn record_and_off_shelf_products(
        &self,
        owner_type: i32,
        owner_id: i32,
        now: i32,
    ) -> Result<u64> {
        if owner_type == 1 {
            let on_sale_products: Vec<(i32,)> = sqlx::query_as(
                "SELECT product_id FROM product WHERE shop_id = $1 AND product_status = 1 AND is_delete = 0",
            )
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

            if on_sale_products.is_empty() {
                return Ok(0);
            }

            let logged: Vec<(i32,)> = sqlx::query_as(
                "SELECT product_id FROM shop_deposit_offshelf_log WHERE owner_type = $1 AND owner_id = $2 AND status = 1",
            )
            .bind(owner_type)
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
            let logged_set: std::collections::HashSet<i32> = logged.into_iter().map(|(id,)| id).collect();

            for (pid,) in &on_sale_products {
                if !logged_set.contains(pid) {
                    let _ = sqlx::query(
                        "INSERT INTO shop_deposit_offshelf_log (owner_type, owner_id, product_id, status, off_time, add_time) VALUES ($1, $2, $3, 1, $4, $5)",
                    )
                    .bind(owner_type)
                    .bind(owner_id)
                    .bind(*pid)
                    .bind(now)
                    .bind(now)
                    .execute(&self.pool)
                    .await;
                }
            }

            let res = sqlx::query("UPDATE product SET product_status = 0 WHERE shop_id = $1 AND is_delete = 0")
                .bind(owner_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            Ok(res.rows_affected())
        } else {
            let on_sale_products: Vec<(i64,)> = sqlx::query_as(
                "SELECT id FROM vendor_product WHERE vendor_id = $1 AND product_state = 1 AND is_del = 0",
            )
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

            if on_sale_products.is_empty() {
                return Ok(0);
            }

            for (vid,) in &on_sale_products {
                let _ = sqlx::query(
                    "INSERT INTO shop_deposit_offshelf_log (owner_type, owner_id, product_id, status, off_time, add_time) VALUES ($1, $2, $3, 1, $4, $5)",
                )
                .bind(owner_type)
                .bind(owner_id)
                .bind(*vid as i32)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await;
            }

            let res = sqlx::query("UPDATE vendor_product SET product_state = 0 WHERE vendor_id = $1 AND is_del = 0")
                .bind(owner_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            Ok(res.rows_affected())
        }
    }

    pub async fn find_by_owner(&self, owner_type: i32, owner_id: i32) -> Result<Option<AccountRow>> {
        sqlx::query_as::<_, AccountRow>(
            "SELECT shop_deposit_id, owner_type, owner_id, paid, required_manual, required_snapshot, penalty_status, add_time FROM shop_deposit WHERE owner_type = $1 AND owner_id = $2 LIMIT 1",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 持有者名称（店铺名/供应商名）。
    pub async fn owner_name(&self, owner_type: i32, owner_id: i32) -> Result<Option<String>> {
        let sql = if owner_type == 1 {
            "SELECT shop_title FROM shop WHERE shop_id = $1 LIMIT 1"
        } else {
            "SELECT vendor_name FROM vendor WHERE vendor_id = $1 LIMIT 1"
        };
        let row: Option<(Option<String>,)> = sqlx::query_as(sql)
            .bind(owner_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 持有者是否存在。
    pub async fn owner_exists(&self, owner_type: i32, owner_id: i32) -> Result<bool> {
        let sql = if owner_type == 1 {
            "SELECT COUNT(*) FROM shop WHERE shop_id = $1"
        } else {
            "SELECT COUNT(*) FROM vendor WHERE vendor_id = $1"
        };
        let (n,): (i64,) = sqlx::query_as(sql)
            .bind(owner_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(n > 0)
    }

    /// 流水分页（Java `logList`）。
    pub async fn paginate_logs(
        &self,
        owner_type: i32,
        owner_id: i32,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<LogRow>)> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop_deposit_log WHERE owner_type = $1 AND owner_id = $2",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, LogRow>(
            "SELECT shop_deposit_log_id, owner_type, owner_id, type, change_amount, before_paid, after_paid, related_id, order_id, admin_id, remark, add_time FROM shop_deposit_log WHERE owner_type = $1 AND owner_id = $2 ORDER BY shop_deposit_log_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(i64::try_from(size).unwrap_or(15))
        .bind(i64::try_from(page.saturating_sub(1) * size).unwrap_or(0))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 调整记录分页（Java `adjustLogList`）。
    pub async fn paginate_adjust_logs(
        &self,
        owner_type: i32,
        owner_id: i32,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<AdjustLogRow>)> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop_deposit_adjust_log WHERE owner_type = $1 AND owner_id = $2",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AdjustLogRow>(
            "SELECT shop_deposit_adjust_id, owner_type, owner_id, before_required, after_required, source, admin_id, reason, add_time FROM shop_deposit_adjust_log WHERE owner_type = $1 AND owner_id = $2 ORDER BY shop_deposit_adjust_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(i64::try_from(size).unwrap_or(15))
        .bind(i64::try_from(page.saturating_sub(1) * size).unwrap_or(0))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 写调整记录并更新账户应缴（Java `adjust` 核心写库）。
    pub async fn apply_adjust(
        &self,
        dto: &DepositAdjustDTO,
        owner_type: i32,
        owner_id: i32,
        admin_id: i32,
        now: i32,
    ) -> Result<()> {
        let account = self.find_by_owner(owner_type, owner_id).await?;
        let before = account
            .as_ref()
            .map(|a| a.required_manual.unwrap_or(a.required_snapshot))
            .unwrap_or(Decimal::ZERO);
        let after = match dto.adjust_type.unwrap_or(2) {
            1 => account
                .as_ref()
                .map(|a| a.required_snapshot)
                .unwrap_or(Decimal::ZERO),
            _ => dto.required_manual.unwrap_or(Decimal::ZERO),
        };
        sqlx::query(
            "INSERT INTO shop_deposit_adjust_log (owner_type, owner_id, before_required, after_required, source, admin_id, reason, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(before)
        .bind(after)
        .bind(dto.adjust_type.unwrap_or(2))
        .bind(admin_id)
        .bind(dto.reason.as_deref())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        sqlx::query(
            "UPDATE shop_deposit SET required_manual = $1, required_snapshot = $2, update_time = $3 WHERE owner_type = $4 AND owner_id = $5",
        )
        .bind(if dto.adjust_type == Some(2) { dto.required_manual } else { None })
        .bind(after)
        .bind(now)
        .bind(owner_type)
        .bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 规则变更对账：写入调整日志并更新账户应缴快照与补缴缓冲时间（Java DepositLifecycleServiceImpl#reconcileRequired）。
    pub async fn apply_rule_change_adjust(
        &self,
        shop_deposit_id: i32,
        owner_type: i32,
        owner_id: i32,
        before: Decimal,
        after: Decimal,
        supplement_due_time: Option<i32>,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO shop_deposit_adjust_log (owner_type, owner_id, before_required, after_required, source, admin_id, reason, add_time) VALUES ($1, $2, $3, $4, 1, 0, '规则变更自动对账', $5)",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(before)
        .bind(after)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        if let Some(due) = supplement_due_time {
            sqlx::query(
                "UPDATE shop_deposit SET required_snapshot = $1, supplement_due_time = $2, update_time = $3 WHERE shop_deposit_id = $4",
            )
            .bind(after)
            .bind(due)
            .bind(now)
            .bind(shop_deposit_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        } else {
            sqlx::query(
                "UPDATE shop_deposit SET required_snapshot = $1, update_time = $2 WHERE shop_deposit_id = $3",
            )
            .bind(after)
            .bind(now)
            .bind(shop_deposit_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }

        Ok(())
    }

    /// 赔付扣除：写流水 + 扣减实缴（Java `deduct` 核心写库）。
    pub async fn apply_deduct(
        &self,
        dto: &DepositDeductDTO,
        owner_type: i32,
        owner_id: i32,
        admin_id: i32,
        now: i32,
    ) -> Result<bool> {
        let Some(account) = self.find_by_owner(owner_type, owner_id).await? else {
            return Ok(false);
        };
        let amount = dto.amount.unwrap_or(Decimal::ZERO);
        let before = account.paid;
        let after = before - amount;
        sqlx::query(
            "INSERT INTO shop_deposit_log (owner_type, owner_id, type, change_amount, before_paid, after_paid, related_id, order_id, admin_id, remark, add_time) VALUES ($1, $2, 4, $3, $4, $5, 0, $6, $7, $8, $9)",
        )
        .bind(owner_type)
        .bind(owner_id)
        .bind(-amount)
        .bind(before)
        .bind(after)
        .bind(dto.order_id.unwrap_or(0))
        .bind(admin_id)
        .bind(dto.reason.as_deref())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        sqlx::query("UPDATE shop_deposit SET paid = $1, update_time = $2 WHERE owner_type = $3 AND owner_id = $4")
            .bind(after)
            .bind(now)
            .bind(owner_type)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(true)
    }

    /// 管理员用户名（调整记录展示用）。
    pub async fn admin_name(&self, admin_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT username FROM admin_user WHERE admin_id = $1 LIMIT 1")
                .bind(admin_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(u,)| u))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
