//! 公众号关键字回复规则仓储：`wechat_keyword_rule` + 关键字/回复子表。
//!
//! 列名严格对齐 schema：
//! - `wechat_keyword_rule`：`id` / `rule_name` / `reply_mode` / `enabled` /
//!   `created_at` / `updated_at`
//! - `wechat_keyword_rule_keyword`：`id` / `rule_id` / `keyword` / `match_type` / `sort_order`
//! - `wechat_keyword_rule_reply`：`id` / `rule_id` / `sort_order` / `msg_type` /
//!   `content_text` / `news_title` / `news_cover_url` / `news_description` / `news_url`
//!
//! 分页 SQL 复刻 `WechatKeywordRuleMapper.xml#pageWithCount`：四个标量子查询
//! 统计关键字/回复数，`ORDER BY r.created_at DESC, r.id DESC`。
//!
//! 事务：`save`（主表 upsert + 子表先删后插）与 `delete`（三表级联）在 Java 为
//! `@Transactional`，本实现在仓储层用 `pool.begin()` 保证原子性。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::{
    WechatKeywordRule, WechatKeywordRuleKeyword, WechatKeywordRuleReply,
};
use nextrade_core::models::content::wechat_keyword_rule::{
    now_datetime_string, KeywordRuleSaveDTO, MAX_RULES,
};
use nextrade_core::{Error, Result};

/// 列表行（含四个统计列，Java `KeywordRuleVO` 的 SQL 投影）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KeywordRuleListRow {
    /// 主键。
    pub id: i64,
    /// 规则名。
    pub rule_name: String,
    /// 回复模式。
    pub reply_mode: i32,
    /// 启用状态。
    pub enabled: i32,
    /// 创建时间。
    pub created_at: String,
    /// 更新时间。
    pub updated_at: String,
    /// 关键字数量。
    pub keyword_count: i64,
    /// 回复数量。
    pub reply_count: i64,
    /// 文本回复数量。
    pub text_reply_count: i64,
    /// 图文回复数量。
    pub news_reply_count: i64,
}

#[derive(Clone)]
pub struct WechatKeywordRuleRepository {
    pool: PgPool,
}

impl WechatKeywordRuleRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 读 —————————————————————

    /// 分页列表（Java `pageWithCount`）。
    pub async fn page(
        &self,
        rule_name: Option<&str>,
        enabled: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<KeywordRuleListRow>)> {
        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM wechat_keyword_rule WHERE 1 = 1");
        Self::push_filters(&mut count_qb, rule_name, enabled);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT r.id, r.rule_name, r.reply_mode, r.enabled, r.created_at::text AS created_at, r.updated_at::text AS updated_at, \
             (SELECT COUNT(1) FROM wechat_keyword_rule_keyword k WHERE k.rule_id = r.id) AS keyword_count, \
             (SELECT COUNT(1) FROM wechat_keyword_rule_reply p WHERE p.rule_id = r.id) AS reply_count, \
             (SELECT COUNT(1) FROM wechat_keyword_rule_reply p WHERE p.rule_id = r.id AND p.msg_type = 1) AS text_reply_count, \
             (SELECT COUNT(1) FROM wechat_keyword_rule_reply p WHERE p.rule_id = r.id AND p.msg_type = 2) AS news_reply_count \
             FROM wechat_keyword_rule r WHERE 1 = 1",
        );
        if let Some(name) = rule_name {
            row_qb.push(" AND r.rule_name LIKE ");
            row_qb.push_bind(format!("%{name}%"));
        }
        if let Some(enabled) = enabled {
            row_qb.push(" AND r.enabled = ");
            row_qb.push_bind(enabled);
        }
        row_qb.push(" ORDER BY r.created_at DESC, r.id DESC");
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<KeywordRuleListRow> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        rule_name: Option<&str>,
        enabled: Option<i32>,
    ) {
        if let Some(name) = rule_name {
            qb.push(" AND rule_name LIKE ");
            qb.push_bind(format!("%{name}%"));
        }
        if let Some(enabled) = enabled {
            qb.push(" AND enabled = ");
            qb.push_bind(enabled);
        }
    }

    /// 按主键查询规则。
    pub async fn find_by_id(&self, id: i64) -> Result<Option<WechatKeywordRule>> {
        sqlx::query_as::<_, WechatKeywordRule>(
            "SELECT id, rule_name, reply_mode, enabled, created_at::text AS created_at, updated_at::text AS updated_at \
             FROM wechat_keyword_rule WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 规则名是否已存在（忽略大小写；Java `LOWER(rule_name) = LOWER(?)`）。
    pub async fn rule_name_exists(&self, rule_name: &str, exclude_id: Option<i64>) -> Result<bool> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM wechat_keyword_rule \
             WHERE LOWER(rule_name) = LOWER(",
        );
        qb.push_bind(rule_name.to_string());
        qb.push(")");
        if let Some(exclude_id) = exclude_id {
            qb.push(" AND id <> ");
            qb.push_bind(exclude_id);
        }
        let row: (i64,) = qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 规则总数（Java `countRules`）。
    pub async fn count_rules(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM wechat_keyword_rule")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 主表创建时是否达全局上限（Java `total >= MAX_RULES`）。
    pub async fn is_rule_limit_reached(&self) -> Result<bool> {
        Ok(self.count_rules().await? >= MAX_RULES as i64)
    }

    /// 某规则的回复载荷（`ORDER BY sort_order ASC, id ASC`）。
    pub async fn replies_by_rule_id(&self, rule_id: i64) -> Result<Vec<WechatKeywordRuleReply>> {
        sqlx::query_as::<_, WechatKeywordRuleReply>(
            "SELECT * FROM wechat_keyword_rule_reply WHERE rule_id = $1 ORDER BY sort_order ASC, id ASC",
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某规则的关键字（`ORDER BY sort_order ASC, id ASC`）。
    pub async fn keywords_by_rule_id(
        &self,
        rule_id: i64,
    ) -> Result<Vec<WechatKeywordRuleKeyword>> {
        sqlx::query_as::<_, WechatKeywordRuleKeyword>(
            "SELECT * FROM wechat_keyword_rule_keyword WHERE rule_id = $1 ORDER BY sort_order ASC, id ASC",
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取关键字（列表回填用；`ORDER BY rule_id ASC, sort_order ASC, id ASC`）。
    pub async fn keywords_by_rule_ids(
        &self,
        rule_ids: &[i64],
    ) -> Result<Vec<WechatKeywordRuleKeyword>> {
        if rule_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT * FROM wechat_keyword_rule_keyword WHERE rule_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in rule_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY rule_id ASC, sort_order ASC, id ASC");
        drop(sep);
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 写 —————————————————————

    /// 主表 upsert + 子表先删后插（Java \"save\"，`@Transactional`）。
    ///
    /// 返回规则 ID。\"created_at\" / \"updated_at\" 由本方法生成。
    pub async fn save_rule(
        &self,
        dto: &KeywordRuleSaveDTO,
        rule_name: &str,
    ) -> Result<i64> {
        let now = now_datetime_string();
        let reply_mode = dto.reply_mode.unwrap_or(1);
        let enabled = dto.enabled.unwrap_or(0);
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let rule_id = match dto.id {
            Some(id) => {
                let existing: Option<(i64,)> =
                    sqlx::query_as("SELECT id FROM wechat_keyword_rule WHERE id = $1 LIMIT 1")
                        .bind(id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(map_err)?;
                if existing.is_none() {
                    return Err(Error::biz_msg("规则不存在"));
                }
                sqlx::query(
                    "UPDATE wechat_keyword_rule SET rule_name = $1, reply_mode = $2, enabled = $3, \
                     updated_at = NULLIF($4, '') WHERE id = $5",
                )
                .bind(rule_name)
                .bind(reply_mode)
                .bind(enabled)
                .bind(&now)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
                id
            }
            None => {
                let r: i64 = sqlx::query_scalar(
                    "INSERT INTO wechat_keyword_rule \
                     (rule_name, reply_mode, enabled, created_at, updated_at) \
                     VALUES ($1, $2, $3, $4, $5) RETURNING id",
                )
                .bind(rule_name)
                .bind(reply_mode)
                .bind(enabled)
                .bind(&now)
                .bind(&now)
                .fetch_one(&mut *tx)
                .await
                .map_err(map_err)?;
                r
            }
        };

        sqlx::query("DELETE FROM wechat_keyword_rule_keyword WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("DELETE FROM wechat_keyword_rule_reply WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

        for (idx, k) in dto.keywords.iter().enumerate() {
            sqlx::query(
                "INSERT INTO wechat_keyword_rule_keyword \
                 (rule_id, keyword, match_type, sort_order) VALUES ($1, $2, $3, $4)",
            )
            .bind(rule_id)
            .bind(k.keyword.as_deref().map(str::trim).unwrap_or(""))
            .bind(k.match_type.unwrap_or(1))
            .bind(idx as i32)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        for (idx, r) in dto.replies.iter().enumerate() {
            let msg_type = r.msg_type.unwrap_or(1);
            let (content_text, news_title, news_cover_url, news_description, news_url) =
                if msg_type == 1 {
                    (r.content.clone(), None, None, None, None)
                } else {
                    (
                        None,
                        r.news_title.clone(),
                        r.news_cover_url.clone(),
                        r.news_description.clone(),
                        r.news_url.clone(),
                    )
                };
            sqlx::query(
                "INSERT INTO wechat_keyword_rule_reply \
                 (rule_id, sort_order, msg_type, content_text, news_title, news_cover_url, \
                  news_description, news_url) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(rule_id)
            .bind(idx as i32)
            .bind(msg_type)
            .bind(content_text)
            .bind(news_title)
            .bind(news_cover_url)
            .bind(news_description)
            .bind(news_url)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(rule_id)
    }

    /// 批量删除（Java \"del\"，`@Transactional`）：子表 + 主表。
    pub async fn delete_rules(&self, ids: &[i64]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for table in ["wechat_keyword_rule_keyword", "wechat_keyword_rule_reply"] {
            let mut qb: QueryBuilder<'_, Postgres> =
                QueryBuilder::new(format!("DELETE FROM {table} WHERE rule_id IN ("));
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
            qb.build().execute(&mut *tx).await.map_err(map_err)?;
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "DELETE FROM wechat_keyword_rule WHERE id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let r = qb.build().execute(&mut *tx).await.map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量启停（Java `setEnabled`）。
    pub async fn set_enabled(&self, ids: &[i64], enabled: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let now = now_datetime_string();
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("UPDATE wechat_keyword_rule SET enabled = ");
        qb.push_bind(enabled);
        qb.push(", updated_at = NULLIF(");
        qb.push_bind(now);
        qb.push(", '')");
        qb.push(" WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let r = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("规则不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_row_struct_covers_vo_counts() {
        // 结构体字段与 Java `KeywordRuleVOMap` 对齐（编译期即可校验缺失）
        let row = KeywordRuleListRow {
            id: 1,
            rule_name: "r".into(),
            reply_mode: 1,
            enabled: 1,
            created_at: "2026-04-23 10:00:00".into(),
            updated_at: "2026-04-23 10:00:00".into(),
            keyword_count: 2,
            reply_count: 3,
            text_reply_count: 1,
            news_reply_count: 2,
        };
        assert_eq!(
            row.keyword_count + row.reply_count,
            row.text_reply_count + row.news_reply_count + 2
        );
    }

    #[test]
    fn max_rules_constant_used() {
        assert_eq!(MAX_RULES, 200);
    }
}
