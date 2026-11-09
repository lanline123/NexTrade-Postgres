//! 前台充值仓储（payment::recharge_order）。
//!
//! 表：`recharge_setting` / `user_recharge_order` / `user_withdraw_apply` /
//! `paylog` / `user`。支付方式配置复用 `payment::pay` 仓储的 `config_values`。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query`（禁止 `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;

use nextrade_core::entities::finance::{UserRechargeOrder, UserWithdrawApply};
use nextrade_core::entities::promotion::RechargeSetting;
use nextrade_core::entities::user::User;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct RechargeOrderRepository {
    pool: PgPool,
}

impl RechargeOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 充值配置分页（Java `RechargeSettingServiceImpl#settingPage`：`is_show = 1`）。
    pub async fn paginate_settings(&self, page: i64, size: i64) -> Result<(i64, Vec<RechargeSetting>)> {
        let total: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM recharge_setting WHERE is_show = 1")
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        let rows = sqlx::query_as::<_, RechargeSetting>(
            "SELECT * FROM recharge_setting WHERE is_show = 1 ORDER BY sort_order ASC, recharge_id ASC LIMIT $1 OFFSET $2",
        )
        .bind(size.max(1))
        .bind((page.max(1) - 1) * size.max(1))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 精确匹配充值金额的配置行（Java `rechargeOperation`：`money = amount ORDER BY money DESC LIMIT 1`）。
    pub async fn find_setting_by_money(&self, amount: Decimal) -> Result<Option<RechargeSetting>> {
        sqlx::query_as::<_, RechargeSetting>(
            "SELECT * FROM recharge_setting WHERE money = $1 ORDER BY money DESC LIMIT 1",
        )
        .bind(amount)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 id + is_show=1 取充值配置（Java 无 amount 时的分支）。
    pub async fn find_setting_by_id_show(&self, id: i32) -> Result<Option<RechargeSetting>> {
        sqlx::query_as::<_, RechargeSetting>(
            "SELECT * FROM recharge_setting WHERE recharge_id = $1 AND is_show = 1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 用户提现申请（`wallet_type = 0` 余额钱包，status 可选过滤）。
    pub async fn list_withdraw(
        &self,
        user_id: i32,
        status: Option<i32>,
    ) -> Result<Vec<UserWithdrawApply>> {
        let mut sql = String::from(
            "SELECT * FROM user_withdraw_apply WHERE user_id = $1 AND wallet_type = 0 AND status <> -1",
        );
        if status.is_some() {
            sql.push_str(" AND status = ?");
        }
        sql.push_str(" ORDER BY add_time DESC");
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, UserWithdrawApply>(&sql).bind(user_id);
        if let Some(status) = status {
            q = q.bind(status);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 用户充值记录（status 可选过滤）。
    pub async fn list_recharge(
        &self,
        user_id: i32,
        status: Option<i32>,
    ) -> Result<Vec<UserRechargeOrder>> {
        let mut sql = String::from(
            "SELECT * FROM user_recharge_order WHERE user_id = $1 AND status <> -1",
        );
        if status.is_some() {
            sql.push_str(" AND status = ?");
        }
        sql.push_str(" ORDER BY add_time DESC");
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, UserRechargeOrder>(&sql).bind(user_id);
        if let Some(status) = status {
            q = q.bind(status);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 新建充值单（Java `rechargeOperation` 保存；状态默认 0 待支付）。
    pub async fn insert_recharge(
        &self,
        user_id: i32,
        amount: Decimal,
        discount_money: Decimal,
        add_time: i32,
        postscript: &str,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_recharge_order (user_id, amount, discount_money, add_time, paid_time, postscript, status)
             VALUES ($1,$2,$3,$4,0,$5,0) RETURNING order_id",
        )
        .bind(user_id)
        .bind(amount)
        .bind(discount_money)
        .bind(add_time)
        .bind(postscript)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn find_recharge(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<Option<UserRechargeOrder>> {
        sqlx::query_as::<_, UserRechargeOrder>(
            "SELECT * FROM user_recharge_order WHERE order_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_user(&self, user_id: i32) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("充值申请不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn insert_placeholder_count() {
        let sql = "INSERT INTO user_recharge_order (user_id, amount, discount_money, add_time, paid_time, postscript, status)
             VALUES ($1,$2,$3,$4,0,$5,0)";
        assert_eq!(sql.matches('?').count(), 5);
    }
}
