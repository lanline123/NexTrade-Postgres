//! 店铺标签二期：自动榜单授标服务（merchant::shop_label_rule）—— **本平台新增**
//! （论坛帖 207878955346177229 二期，一期手动打标见 `shop_label` 模块）。
//!
//! 模型：一行规则 = 一个榜单口径（指标 + 统计窗口 + 前 N + 门槛）→ 一个标签。
//! 定时任务（`system::scheduler` 的 shop_label 循环，默认每小时）与管理端
//! 「立即重算」共用同一条重算路径：按指标聚合各店数值 → 排名取前 N 且过
//! 门槛者为上榜名单 → 与该规则已授出的自动标签做差集，授新摘旧。
//!
//! 与手动打标的冲突策略（迁移 028 给 `shop_label_rel` 加了 source / rule_id）：
//! - 手动分配（source=0）只归平台在店铺列表里维护，一期整店替换只动这类行；
//! - 自动授标（source=1）只归所属规则的重算维护，手动保存永不摘自动标签；
//! - 同店同标签只有一行：授标 INSERT 走 ON CONFLICT DO NOTHING——已有手动行
//!   时不另建行（该店掉出榜单时手动标签照留）；两条规则授同一标签时先到
//!   的规则持有该行，另一规则摘标不动它（其重算下轮会重新授上，空窗 ≤1 轮）。
//!
//! 指标口径（聚合源均为现行生产表，与既有统计口径对齐）：
//! - `sales_amount` 成交额：`order_item.price * quantity` 按 item 的 shop_id
//!   聚合，订单须已支付（`"order".pay_status = 2`、未删除），窗口按 pay_time；
//!   样本 = 订单数（COUNT DISTINCT order_id）。退款净额不扣减，与平台
//!   统计仓储（analytics pay_status=2 口径）一致。
//! - `sales_orders` 成交单量：同上，指标 = COUNT(DISTINCT order_id)。
//! - `rating` 评分：`comment.comment_rank` 均值，仅审核通过的主评价
//!   （status=1、parent_id=0），窗口按 add_time；样本 = 评价数。
//! - `collectors` 收藏人数：`collect_shop` 当前总收藏人数（COUNT DISTINCT
//!   user_id）。**不吃窗口**——collect_shop 的 add_time 在现行写入路径恒为
//!   0（仓储 insert 实证），窗口统计无意义，收藏榜按存量排名。
//!
//! 维度：本期仅全平台榜（类目/地区维度未做，理由见二期报告第三节）。
//! 迁移：`rust/migrations/028_shop_label_auto_rule.sql`（部署前必须先执行）。

use std::collections::HashSet;
use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 全局业务错误码（与 shop_label 的 SERVICE_DATA_ERROR 口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;

pub const METRIC_SALES_AMOUNT: &str = "sales_amount";
pub const METRIC_SALES_ORDERS: &str = "sales_orders";
pub const METRIC_RATING: &str = "rating";
pub const METRIC_COLLECTORS: &str = "collectors";

const ALL_METRICS: [&str; 4] = [
    METRIC_SALES_AMOUNT,
    METRIC_SALES_ORDERS,
    METRIC_RATING,
    METRIC_COLLECTORS,
];

/// 规则一行（管理端全字段；label_name 为标签库 LEFT JOIN 回填，标签已删时为空）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopLabelRuleItem {
    pub rule_id: i32,
    pub rule_name: String,
    pub label_id: i32,
    pub label_name: String,
    pub metric: String,
    pub window_days: i32,
    pub rank_top: i32,
    pub min_value: Decimal,
    pub min_samples: i32,
    /// 1 启用 0 停用（停用只停止重算，已授标签保留，与一期标签停用口径一致）
    pub status: i32,
    pub last_run_time: i32,
    pub last_award_count: i32,
    pub last_remove_count: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 规则写参数（create / update 共用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopLabelRuleWrite {
    pub rule_id: Option<i32>,
    pub rule_name: Option<String>,
    pub label_id: Option<i32>,
    pub metric: Option<String>,
    pub window_days: Option<i32>,
    pub rank_top: Option<i32>,
    pub min_value: Option<Decimal>,
    pub min_samples: Option<i32>,
    pub status: Option<i32>,
}

/// 一条重算流水（管理端规则行展开查看）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopLabelRuleLogItem {
    pub log_id: i32,
    pub rule_id: i32,
    pub run_time: i32,
    pub trigger_type: String,
    pub awarded: i32,
    pub removed: i32,
    pub winners: String,
}

/// 上榜名单一条（重算结果与流水快照共用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleWinner {
    pub shop_id: i32,
    /// 指标值（字符串保精度：金额/评分均为 Decimal 原文）
    pub value: String,
    pub samples: i64,
}

/// 一次重算的结果摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRunSummary {
    pub rule_id: i32,
    /// 本轮新授标店铺数
    pub added: i32,
    /// 本轮摘标店铺数
    pub removed: i32,
    /// 重算后该规则实际持有标签的店铺数
    pub awarded: i32,
    pub winners: Vec<RuleWinner>,
}

#[derive(Debug, FromRow)]
struct WinnerRow {
    shop_id: i32,
    value: Decimal,
    samples: i64,
}

fn now_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

/// 样本下限的实际生效值：单量/收藏的样本即指标本身，min_samples 不参与。
fn effective_min_samples(metric: &str, min_samples: i32) -> i32 {
    match metric {
        METRIC_SALES_ORDERS | METRIC_COLLECTORS => 0,
        _ => min_samples.max(0),
    }
}

/// 差集计算（纯函数，可单测）：当前持有者 vs 新上榜名单 →（待授，待摘）。
fn diff_winners(current: &[i32], winners: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let cur: HashSet<i32> = current.iter().copied().collect();
    let win: HashSet<i32> = winners.iter().copied().collect();
    let to_add: Vec<i32> = winners.iter().filter(|id| !cur.contains(id)).copied().collect();
    let to_remove: Vec<i32> = current.iter().filter(|id| !win.contains(id)).copied().collect();
    (to_add, to_remove)
}

fn validate_metric(metric: &str) -> Result<()> {
    if ALL_METRICS.contains(&metric) {
        Ok(())
    } else {
        Err(Error::biz(SERVICE_DATA_ERROR, "指标类型不合法"))
    }
}

/// 规则字段校验（纯函数，可单测；标签存在性由服务层查库校验）。
fn validate_rule_fields(
    rule_name: &str,
    metric: &str,
    window_days: i32,
    rank_top: i32,
    min_value: Decimal,
    min_samples: i32,
) -> Result<String> {
    let name = rule_name.trim();
    if name.is_empty() {
        return Err(Error::biz(SERVICE_DATA_ERROR, "规则名称不能为空"));
    }
    if name.chars().count() > 20 {
        return Err(Error::biz(SERVICE_DATA_ERROR, "规则名称最多 20 个字"));
    }
    validate_metric(metric)?;
    if !(1..=365).contains(&window_days) {
        return Err(Error::biz(SERVICE_DATA_ERROR, "统计窗口须在 1..365 天"));
    }
    if !(1..=100).contains(&rank_top) {
        return Err(Error::biz(SERVICE_DATA_ERROR, "上榜名额须在 1..100"));
    }
    if min_value < Decimal::ZERO {
        return Err(Error::biz(SERVICE_DATA_ERROR, "入榜门槛不能为负"));
    }
    if !(0..=1_000_000).contains(&min_samples) {
        return Err(Error::biz(SERVICE_DATA_ERROR, "样本下限须在 0..1000000"));
    }
    Ok(name.to_string())
}

pub struct ShopLabelRuleService {
    repos: Arc<Repositories>,
}

impl ShopLabelRuleService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.repos.pool
    }

    /// 规则全量（含停用），按 id 升序。
    pub async fn list_rules(&self) -> Result<Vec<ShopLabelRuleItem>> {
        let rows: Vec<ShopLabelRuleItem> = sqlx::query_as(
            "SELECT r.rule_id, r.rule_name, r.label_id, \
                    COALESCE(l.label_name, '') AS label_name, \
                    r.metric, r.window_days, r.rank_top, r.min_value, r.min_samples, \
                    r.status, r.last_run_time, r.last_award_count, r.last_remove_count, \
                    r.add_time, r.update_time \
             FROM shop_label_rule r \
             LEFT JOIN shop_label l ON l.label_id = r.label_id \
             ORDER BY r.rule_id ASC",
        )
        .fetch_all(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    async fn get_rule(&self, rule_id: i32) -> Result<Option<ShopLabelRuleItem>> {
        let row: Option<ShopLabelRuleItem> = sqlx::query_as(
            "SELECT r.rule_id, r.rule_name, r.label_id, \
                    COALESCE(l.label_name, '') AS label_name, \
                    r.metric, r.window_days, r.rank_top, r.min_value, r.min_samples, \
                    r.status, r.last_run_time, r.last_award_count, r.last_remove_count, \
                    r.add_time, r.update_time \
             FROM shop_label_rule r \
             LEFT JOIN shop_label l ON l.label_id = r.label_id \
             WHERE r.rule_id = $1",
        )
        .bind(rule_id)
        .fetch_optional(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(row)
    }

    async fn ensure_label_exists(&self, label_id: i32) -> Result<()> {
        if label_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "请选择授标标签"));
        }
        let found: Option<(i32,)> =
            sqlx::query_as("SELECT label_id FROM shop_label WHERE label_id = $1")
                .bind(label_id)
                .fetch_optional(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?;
        if found.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "授标标签不存在"));
        }
        Ok(())
    }

    pub async fn create_rule(&self, p: &ShopLabelRuleWrite) -> Result<i32> {
        let metric = p.metric.as_deref().unwrap_or(METRIC_SALES_AMOUNT);
        let name = validate_rule_fields(
            p.rule_name.as_deref().unwrap_or(""),
            metric,
            p.window_days.unwrap_or(30),
            p.rank_top.unwrap_or(10),
            p.min_value.unwrap_or(Decimal::ZERO),
            p.min_samples.unwrap_or(0),
        )?;
        let label_id = p.label_id.unwrap_or(0);
        self.ensure_label_exists(label_id).await?;
        let now = now_i32();
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shop_label_rule (rule_name, label_id, metric, window_days, \
                rank_top, min_value, min_samples, status, add_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9) RETURNING rule_id",
        )
        .bind(name)
        .bind(label_id)
        .bind(metric)
        .bind(p.window_days.unwrap_or(30))
        .bind(p.rank_top.unwrap_or(10))
        .bind(p.min_value.unwrap_or(Decimal::ZERO))
        .bind(p.min_samples.unwrap_or(0))
        .bind(if p.status == Some(0) { 0 } else { 1 })
        .bind(now)
        .fetch_one(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.0)
    }

    pub async fn update_rule(&self, p: &ShopLabelRuleWrite) -> Result<()> {
        let rule_id = p
            .rule_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "规则ID不能为空"))?;
        let metric = p.metric.as_deref().unwrap_or(METRIC_SALES_AMOUNT);
        let name = validate_rule_fields(
            p.rule_name.as_deref().unwrap_or(""),
            metric,
            p.window_days.unwrap_or(30),
            p.rank_top.unwrap_or(10),
            p.min_value.unwrap_or(Decimal::ZERO),
            p.min_samples.unwrap_or(0),
        )?;
        let label_id = p.label_id.unwrap_or(0);
        self.ensure_label_exists(label_id).await?;
        let affected = sqlx::query(
            "UPDATE shop_label_rule SET rule_name = $2, label_id = $3, metric = $4, \
                window_days = $5, rank_top = $6, min_value = $7, min_samples = $8, \
                status = $9, update_time = $10 \
             WHERE rule_id = $1",
        )
        .bind(rule_id)
        .bind(name)
        .bind(label_id)
        .bind(metric)
        .bind(p.window_days.unwrap_or(30))
        .bind(p.rank_top.unwrap_or(10))
        .bind(p.min_value.unwrap_or(Decimal::ZERO))
        .bind(p.min_samples.unwrap_or(0))
        .bind(if p.status == Some(0) { 0 } else { 1 })
        .bind(now_i32())
        .execute(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?
        .rows_affected();
        if affected == 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "规则不存在"));
        }
        Ok(())
    }

    /// 删除规则：同事务摘除该规则授出的全部自动标签（手动标签不受影响），
    /// 并删除其重算流水。
    pub async fn delete_rule(&self, rule_id: i32) -> Result<()> {
        if rule_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "规则ID不能为空"));
        }
        let mut tx = self.pool().begin().await.map_err(|e| Error::Database(e.to_string()))?;
        sqlx::query("DELETE FROM shop_label_rel WHERE source = 1 AND rule_id = $1")
            .bind(rule_id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        sqlx::query("DELETE FROM shop_label_rule_log WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        let affected = sqlx::query("DELETE FROM shop_label_rule WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?
            .rows_affected();
        if affected == 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "规则不存在"));
        }
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 某规则最近的重算流水（默认 20 条，新→旧）。
    pub async fn list_rule_logs(&self, rule_id: i32, limit: i64) -> Result<Vec<ShopLabelRuleLogItem>> {
        let rows: Vec<ShopLabelRuleLogItem> = sqlx::query_as(
            "SELECT log_id, rule_id, run_time, trigger_type, awarded, removed, winners \
             FROM shop_label_rule_log WHERE rule_id = $1 \
             ORDER BY log_id DESC LIMIT $2",
        )
        .bind(rule_id)
        .bind(limit.clamp(1, 100))
        .fetch_all(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    /// 按指标聚合上榜名单（已按 名次序 排好：指标降序、样本降序、shop_id 升序）。
    async fn compute_winners(&self, rule: &ShopLabelRuleItem) -> Result<Vec<WinnerRow>> {
        let since = now_i32() - rule.window_days.max(1) * 86_400;
        let min_samples = effective_min_samples(&rule.metric, rule.min_samples);
        let rows: Vec<WinnerRow> = match rule.metric.as_str() {
            METRIC_SALES_AMOUNT => {
                sqlx::query_as(
                    "SELECT oi.shop_id AS shop_id, \
                            SUM(oi.price * oi.quantity)::numeric AS value, \
                            COUNT(DISTINCT oi.order_id) AS samples \
                     FROM order_item oi \
                     JOIN \"order\" o ON o.order_id = oi.order_id \
                     JOIN shop s ON s.shop_id = oi.shop_id \
                     WHERE o.pay_status = 2 AND o.is_del = 0 AND o.pay_time >= $1 \
                       AND oi.shop_id > 0 AND (s.is_delete IS NULL OR s.is_delete = 0) \
                     GROUP BY oi.shop_id \
                     HAVING SUM(oi.price * oi.quantity) >= $2 \
                        AND COUNT(DISTINCT oi.order_id) >= $3 \
                     ORDER BY value DESC, samples DESC, oi.shop_id ASC \
                     LIMIT $4",
                )
                .bind(since)
                .bind(rule.min_value)
                .bind(min_samples)
                .bind(i64::from(rule.rank_top))
                .fetch_all(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?
            }
            METRIC_SALES_ORDERS => {
                sqlx::query_as(
                    "SELECT oi.shop_id AS shop_id, \
                            COUNT(DISTINCT oi.order_id)::numeric AS value, \
                            COUNT(DISTINCT oi.order_id) AS samples \
                     FROM order_item oi \
                     JOIN \"order\" o ON o.order_id = oi.order_id \
                     JOIN shop s ON s.shop_id = oi.shop_id \
                     WHERE o.pay_status = 2 AND o.is_del = 0 AND o.pay_time >= $1 \
                       AND oi.shop_id > 0 AND (s.is_delete IS NULL OR s.is_delete = 0) \
                     GROUP BY oi.shop_id \
                     HAVING COUNT(DISTINCT oi.order_id) >= $2 \
                     ORDER BY value DESC, samples DESC, oi.shop_id ASC \
                     LIMIT $3",
                )
                .bind(since)
                .bind(rule.min_value)
                .bind(i64::from(rule.rank_top))
                .fetch_all(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?
            }
            METRIC_RATING => {
                sqlx::query_as(
                    "SELECT c.shop_id AS shop_id, \
                            AVG(c.comment_rank)::numeric AS value, \
                            COUNT(*) AS samples \
                     FROM comment c \
                     JOIN shop s ON s.shop_id = c.shop_id \
                     WHERE c.status = 1 AND c.parent_id = 0 AND c.shop_id > 0 \
                       AND c.add_time >= $1 \
                       AND (s.is_delete IS NULL OR s.is_delete = 0) \
                     GROUP BY c.shop_id \
                     HAVING AVG(c.comment_rank) >= $2 AND COUNT(*) >= $3 \
                     ORDER BY value DESC, samples DESC, c.shop_id ASC \
                     LIMIT $4",
                )
                .bind(since)
                .bind(rule.min_value)
                .bind(i64::from(min_samples))
                .bind(i64::from(rule.rank_top))
                .fetch_all(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?
            }
            METRIC_COLLECTORS => {
                sqlx::query_as(
                    "SELECT cs.shop_id AS shop_id, \
                            COUNT(DISTINCT cs.user_id)::numeric AS value, \
                            COUNT(DISTINCT cs.user_id) AS samples \
                     FROM collect_shop cs \
                     JOIN shop s ON s.shop_id = cs.shop_id \
                     WHERE cs.shop_id > 0 AND (s.is_delete IS NULL OR s.is_delete = 0) \
                     GROUP BY cs.shop_id \
                     HAVING COUNT(DISTINCT cs.user_id) >= $1 \
                     ORDER BY value DESC, cs.shop_id ASC \
                     LIMIT $2",
                )
                .bind(rule.min_value)
                .bind(i64::from(rule.rank_top))
                .fetch_all(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?
            }
            other => {
                return Err(Error::biz(
                    SERVICE_DATA_ERROR,
                    format!("指标类型不合法: {other}"),
                ))
            }
        };
        Ok(rows)
    }

    /// 重算一条规则（定时任务与手动触发共用）。trigger: schedule / manual。
    /// 幂等差集：并发两轮同跑也只会收敛到同一名单（唯一约束兜底）。
    pub async fn recompute_rule(&self, rule_id: i32, trigger: &str) -> Result<RuleRunSummary> {
        let rule = self
            .get_rule(rule_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "规则不存在"))?;
        // 标签被删除后规则悬空：明确报错而不是静默授空（流水也不落，避免误读）。
        let label_found: Option<(i32,)> =
            sqlx::query_as("SELECT label_id FROM shop_label WHERE label_id = $1")
                .bind(rule.label_id)
                .fetch_optional(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?;
        if label_found.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "规则绑定的标签已被删除"));
        }

        let winner_rows = self.compute_winners(&rule).await?;
        let winner_ids: Vec<i32> = winner_rows.iter().map(|w| w.shop_id).collect();
        let now = now_i32();

        let mut tx = self.pool().begin().await.map_err(|e| Error::Database(e.to_string()))?;
        let current: Vec<(i32,)> = sqlx::query_as(
            "SELECT shop_id FROM shop_label_rel WHERE source = 1 AND rule_id = $1",
        )
        .bind(rule_id)
        .fetch_all(&mut *tx)
        .await.map_err(|e| Error::Database(e.to_string()))?;
        let current_ids: Vec<i32> = current.into_iter().map(|(id,)| id).collect();
        let (to_add, to_remove) = diff_winners(&current_ids, &winner_ids);

        if !to_remove.is_empty() {
            sqlx::query(
                "DELETE FROM shop_label_rel WHERE source = 1 AND rule_id = $1 \
                 AND shop_id = ANY($2)",
            )
            .bind(rule_id)
            .bind(&to_remove)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        }
        for shop_id in &to_add {
            sqlx::query(
                "INSERT INTO shop_label_rel (shop_id, label_id, source, rule_id, add_time) \
                 VALUES ($1, $2, 1, $3, $4) \
                 ON CONFLICT (shop_id, label_id) DO NOTHING",
            )
            .bind(shop_id)
            .bind(rule.label_id)
            .bind(rule_id)
            .bind(now)
            .execute(&mut *tx)
            .await.map_err(|e| Error::Database(e.to_string()))?;
        }
        let awarded: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shop_label_rel WHERE source = 1 AND rule_id = $1",
        )
        .bind(rule_id)
        .fetch_one(&mut *tx)
        .await.map_err(|e| Error::Database(e.to_string()))?;

        let winners_view: Vec<RuleWinner> = winner_rows
            .iter()
            .map(|w| RuleWinner {
                shop_id: w.shop_id,
                value: w.value.to_string(),
                samples: w.samples,
            })
            .collect();
        let winners_json = serde_json::to_string(&winners_view).unwrap_or_else(|_| "[]".into());

        sqlx::query(
            "UPDATE shop_label_rule SET last_run_time = $2, last_award_count = $3, \
                last_remove_count = $4 WHERE rule_id = $1",
        )
        .bind(rule_id)
        .bind(now)
        .bind(i32::try_from(awarded.0).unwrap_or(0))
        .bind(i32::try_from(to_remove.len()).unwrap_or(0))
        .execute(&mut *tx)
        .await.map_err(|e| Error::Database(e.to_string()))?;
        sqlx::query(
            "INSERT INTO shop_label_rule_log (rule_id, run_time, trigger_type, awarded, removed, winners) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(rule_id)
        .bind(now)
        .bind(trigger)
        .bind(i32::try_from(awarded.0).unwrap_or(0))
        .bind(i32::try_from(to_remove.len()).unwrap_or(0))
        .bind(&winners_json)
        .execute(&mut *tx)
        .await.map_err(|e| Error::Database(e.to_string()))?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;

        Ok(RuleRunSummary {
            rule_id,
            added: i32::try_from(to_add.len()).unwrap_or(0),
            removed: i32::try_from(to_remove.len()).unwrap_or(0),
            awarded: i32::try_from(awarded.0).unwrap_or(0),
            winners: winners_view,
        })
    }

    /// 重算全部启用规则（定时任务入口）。单条失败只记日志不中断整轮，
    /// 返回成功重算的规则数。
    pub async fn recompute_all_enabled(&self, trigger: &str) -> Result<usize> {
        let ids: Vec<(i32,)> = sqlx::query_as(
            "SELECT rule_id FROM shop_label_rule WHERE status = 1 ORDER BY rule_id ASC",
        )
        .fetch_all(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        let mut done = 0usize;
        for (rule_id,) in ids {
            match self.recompute_rule(rule_id, trigger).await {
                Ok(_) => done += 1,
                Err(e) => {
                    tracing::warn!("店铺标签自动授标：规则 {rule_id} 重算失败: {e}");
                }
            }
        }
        Ok(done)
    }
}

#[cfg(test)]
mod tests {
    use super::{diff_winners, effective_min_samples, validate_metric, validate_rule_fields};
    use rust_decimal::Decimal;

    #[test]
    fn metric_whitelist() {
        assert!(validate_metric("sales_amount").is_ok());
        assert!(validate_metric("rating").is_ok());
        assert!(validate_metric("collectors").is_ok());
        assert!(validate_metric("sales_orders").is_ok());
        assert!(validate_metric("gmv").is_err());
        assert!(validate_metric("").is_err());
    }

    #[test]
    fn rule_field_rules() {
        let ok = validate_rule_fields("近30天销量榜", "sales_amount", 30, 10, Decimal::ZERO, 0);
        assert!(ok.is_ok());
        assert!(validate_rule_fields("  ", "sales_amount", 30, 10, Decimal::ZERO, 0).is_err());
        assert!(validate_rule_fields("榜", "sales_amount", 0, 10, Decimal::ZERO, 0).is_err());
        assert!(validate_rule_fields("榜", "sales_amount", 366, 10, Decimal::ZERO, 0).is_err());
        assert!(validate_rule_fields("榜", "sales_amount", 30, 0, Decimal::ZERO, 0).is_err());
        assert!(validate_rule_fields("榜", "sales_amount", 30, 101, Decimal::ZERO, 0).is_err());
        assert!(validate_rule_fields("榜", "rating", 30, 10, Decimal::new(-1, 0), 0).is_err());
        assert!(validate_rule_fields("榜", "rating", 30, 10, Decimal::ZERO, -1).is_err());
    }

    #[test]
    fn samples_gate_only_for_windowed_metrics() {
        assert_eq!(effective_min_samples("sales_amount", 5), 5);
        assert_eq!(effective_min_samples("rating", 5), 5);
        assert_eq!(effective_min_samples("sales_orders", 5), 0);
        assert_eq!(effective_min_samples("collectors", 5), 0);
    }

    #[test]
    fn diff_awards_and_revokes() {
        let (add, remove) = diff_winners(&[1, 2, 3], &[2, 3, 4]);
        assert_eq!(add, vec![4]);
        assert_eq!(remove, vec![1]);
        let (add, remove) = diff_winners(&[], &[7]);
        assert_eq!(add, vec![7]);
        assert!(remove.is_empty());
        let (add, remove) = diff_winners(&[7], &[]);
        assert!(add.is_empty());
        assert_eq!(remove, vec![7]);
        let (add, remove) = diff_winners(&[1, 2], &[1, 2]);
        assert!(add.is_empty() && remove.is_empty());
    }
}
