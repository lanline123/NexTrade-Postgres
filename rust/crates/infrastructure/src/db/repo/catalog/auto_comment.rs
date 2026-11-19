//! 收货后自动评价仓储（我方新增表，迁移 `005_auto_comment.sql`）。
//!
//! 两张表：
//! - `shop_auto_comment_setting`：店铺级设置（shop_id 主键，无行 = 未配置 = 关）；
//! - `order_auto_comment`：确认收货时落的快照 + 待生成任务（一单一行，
//!   `order_id` 唯一约束作幂等锚）。
//!
//! 全部 SQL 为运行时 `sqlx::query` / `query_as`（**禁止** `query!` 宏，
//! 工作区无 `DATABASE_URL`，与本域既有仓储一致）。

use sqlx::PgPool;

use nextrade_core::models::catalog::auto_comment::{
    AutoCommentSetting, AutoCommentTask, CLAIM_STALE_SECONDS, TASK_STATUS_CLAIMED,
    TASK_STATUS_PENDING,
};
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct AutoCommentRepository {
    pool: PgPool,
}

impl AutoCommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 供服务层在同一池上做少量旁路查询（如生成时查订单/会员）。
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ————————————————————— 店铺设置 —————————————————————

    /// 读店铺设置；无行返回 `None`（未配置 = 关闭）。
    pub async fn find_setting(&self, shop_id: i32) -> Result<Option<AutoCommentSetting>> {
        let row: Option<(i32, i32, i32, String, i32)> = sqlx::query_as(
            "SELECT shop_id, enabled, days, contents, update_time \
             FROM shop_auto_comment_setting WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| AutoCommentSetting {
            shop_id: r.0,
            enabled: r.1,
            days: r.2,
            contents: nextrade_core::models::catalog::auto_comment::parse_contents(&r.3),
            update_time: r.4,
        }))
    }

    /// 保存店铺设置（整行 upsert；contents 由调用方序列化前已清洗校验）。
    pub async fn save_setting(&self, setting: &AutoCommentSetting, now: i32) -> Result<()> {
        let contents =
            nextrade_core::models::catalog::auto_comment::serialize_contents(&setting.contents);
        sqlx::query(
            "INSERT INTO shop_auto_comment_setting (shop_id, enabled, days, contents, update_time) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (shop_id) DO UPDATE SET enabled = EXCLUDED.enabled, \
               days = EXCLUDED.days, contents = EXCLUDED.contents, \
               update_time = EXCLUDED.update_time",
        )
        .bind(setting.shop_id)
        .bind(setting.enabled)
        .bind(setting.days)
        .bind(contents)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 快照 / 任务 —————————————————————

    /// 落收货快照。`order_id` 唯一冲突 → 忽略（重复收货事件/重复扫描不重复落行），
    /// 返回是否真正插入。
    pub async fn insert_snapshot(&self, task: &AutoCommentTask) -> Result<bool> {
        let r = sqlx::query(
            "INSERT INTO order_auto_comment \
             (order_id, shop_id, user_id, received_time, days, due_time, content, status, generated_time, add_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $9) \
             ON CONFLICT (order_id) DO NOTHING",
        )
        .bind(task.order_id)
        .bind(task.shop_id)
        .bind(task.user_id)
        .bind(task.received_time)
        .bind(task.days)
        .bind(task.due_time)
        .bind(&task.content)
        .bind(TASK_STATUS_PENDING)
        .bind(task.add_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 到期可生成任务：待生成且已到期，或生成中但抢占已僵尸（进程中断遗留）。
    pub async fn find_due_tasks(&self, now: i32, limit: u64) -> Result<Vec<AutoCommentTask>> {
        let stale_before = i64::from(now) - CLAIM_STALE_SECONDS;
        let rows: Vec<(i32, i32, i32, i32, i32, i32, i32, String, i32, i32, i32, i32)> =
            sqlx::query_as(
                "SELECT id, order_id, shop_id, user_id, received_time, days, due_time, \
                        content, status, generated_time, add_time, update_time \
                 FROM order_auto_comment \
                 WHERE (status = $1 AND due_time <= $2) \
                    OR (status = $3 AND update_time <= $4) \
                 ORDER BY due_time ASC LIMIT $5",
            )
            .bind(TASK_STATUS_PENDING)
            .bind(now)
            .bind(TASK_STATUS_CLAIMED)
            .bind(stale_before as i32)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| AutoCommentTask {
                id: r.0,
                order_id: r.1,
                shop_id: r.2,
                user_id: r.3,
                received_time: r.4,
                days: r.5,
                due_time: r.6,
                content: r.7,
                status: r.8,
                generated_time: r.9,
                add_time: r.10,
                update_time: r.11,
            })
            .collect())
    }

    /// 抢占任务（条件更新，与纯函数 `can_claim` 同条件）：
    /// 只有当前仍可抢占时才置生成中，返回是否抢占成功——重复扫描只有一轮能成功。
    pub async fn claim_task(&self, task: &AutoCommentTask, now: i32) -> Result<bool> {
        let stale_before = i64::from(now) - CLAIM_STALE_SECONDS;
        let r = sqlx::query(
            "UPDATE order_auto_comment SET status = $1, update_time = $2 \
             WHERE id = $3 AND ((status = $4) OR (status = $1 AND update_time <= $5))",
        )
        .bind(TASK_STATUS_CLAIMED)
        .bind(now)
        .bind(task.id)
        .bind(TASK_STATUS_PENDING)
        .bind(stale_before as i32)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 任务收尾：置终态（已生成/已跳过）+ 生成时间。
    pub async fn finish_task(&self, id: i32, final_status: i32, now: i32) -> Result<()> {
        sqlx::query(
            "UPDATE order_auto_comment SET status = $1, generated_time = $2, update_time = $2 \
             WHERE id = $3",
        )
        .bind(final_status)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("自动评价记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
