//! 客户黑名单域仓储（member::user_blacklist）。
//!
//! 表：`user_blacklist` / `user_blacklist_rule` / `user_blacklist_scope_shop` /
//! `user_blacklist_log`（迁移 `rust/migrations/003_customer_blacklist.sql`）。
//! 新表无 Java 对应实体，按方案 §1.11 不进 entities 生成文件，行结构在本层用
//! `sqlx::FromRow` 本地定义。查询统一运行时 sqlx（不用 `query!` 宏）。

use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

use nextrade_core::{Error, Result};

/// 条目主表行（`user_blacklist` 全列）。
///
/// 注意：迁移 003 把 `owner_type` / `apply_mode` / `source` / `status` 与流水表
/// `op_type` 建为 SMALLINT(INT2)，解码必须用 i16——按 i32 解码 sqlx 会严格
/// 拒绝（mismatched types），候选/列表查询只要返回 ≥1 行就整体 500。
/// 写入与过滤参数仍用 i32 绑定（PG 对 INT4 参数有到 INT2 的赋值/比较转换，
/// 与字面量口径一致），只在解码侧对齐列类型。
#[derive(Debug, Clone, Default, FromRow)]
pub struct BlacklistEntry {
    pub id: i64,
    pub user_id: i32,
    pub owner_type: i16,
    pub owner_shop_id: i32,
    pub apply_mode: i16,
    pub reason: String,
    pub source: i16,
    pub status: i16,
    pub create_admin_id: i32,
    pub create_admin_name: String,
    pub create_admin_type: String,
    pub create_time: i64,
    pub update_time: i64,
    pub remove_admin_id: i32,
    pub remove_admin_name: String,
    pub remove_time: i64,
    pub remove_reason: String,
}

/// 规则行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct BlacklistRuleRow {
    pub blacklist_id: i64,
    pub product_code: String,
    pub behavior_code: String,
}

/// 判定候选行（条目 + 命中的原因；范围店铺另查后组装）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct RestrictionCandidateRow {
    pub blacklist_id: i64,
    pub owner_type: i16,
    pub owner_shop_id: i32,
    pub apply_mode: i16,
    pub reason: String,
}

/// 流水行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct BlacklistLogRow {
    pub id: i64,
    pub blacklist_id: i64,
    pub user_id: i32,
    pub op_type: i16,
    pub op_content: String,
    pub rules_snapshot: serde_json::Value,
    pub scope_snapshot: serde_json::Value,
    pub operator_id: i32,
    pub operator_name: String,
    pub operator_type: String,
    pub operator_shop_id: i32,
    pub create_time: i64,
}

/// 会员简行（加入时定位会员用，口径同 shop_member 的 MemberUserRow）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct BlacklistUserRow {
    pub user_id: i32,
    pub username: String,
    pub mobile: String,
    pub email: String,
    pub nickname: Option<String>,
    pub avatar: String,
}

/// 条目列表过滤（可见性由服务层折算为 `visible_shop`）。
#[derive(Debug, Clone, Default)]
pub struct BlacklistListFilter {
    pub keyword: Option<String>,
    pub user_id: Option<i32>,
    pub product_code: Option<String>,
    pub behavior_code: Option<String>,
    pub owner_type: Option<i32>,
    pub shop_id: Option<i32>,
    pub status: Option<i32>,
    pub create_time_start: Option<i64>,
    pub create_time_end: Option<i64>,
    /// 店铺操作人的可见范围：Some(本店 shop_id) 时只看本店条目 + 平台全平台条目。
    pub visible_shop: Option<i32>,
}

/// 流水列表过滤。
#[derive(Debug, Clone, Default)]
pub struct BlacklistLogFilter {
    pub user_id: Option<i32>,
    pub blacklist_id: Option<i64>,
    pub op_type: Option<i32>,
    pub create_time_start: Option<i64>,
    pub create_time_end: Option<i64>,
    /// 同 [`BlacklistListFilter::visible_shop`]。
    pub visible_shop: Option<i32>,
}

const ENTRY_SELECT: &str = "SELECT b.id, b.user_id, b.owner_type, b.owner_shop_id, b.apply_mode, b.reason, b.source, b.status, b.create_admin_id, b.create_admin_name, b.create_admin_type, b.create_time, b.update_time, b.remove_admin_id, b.remove_admin_name, b.remove_time, b.remove_reason FROM user_blacklist b";

const LOG_SELECT: &str = "SELECT l.id, l.blacklist_id, l.user_id, l.op_type, l.op_content, l.rules_snapshot, l.scope_snapshot, l.operator_id, l.operator_name, l.operator_type, l.operator_shop_id, l.create_time FROM user_blacklist_log l";

/// 客户黑名单域仓储。
#[derive(Clone)]
pub struct UserBlacklistRepository {
    pool: PgPool,
}

impl UserBlacklistRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 判定 —————————————————————

    /// 判定主查询（一条 SQL，§4.1）：某用户在 (产品, 行为) 上的全部生效候选条目。
    pub async fn find_restriction_candidates(
        &self,
        user_id: i32,
        product_code: &str,
        behavior_code: &str,
    ) -> Result<Vec<RestrictionCandidateRow>> {
        sqlx::query_as::<_, RestrictionCandidateRow>(
            "SELECT b.id AS blacklist_id, b.owner_type, b.owner_shop_id, b.apply_mode, b.reason \
             FROM user_blacklist b \
             JOIN user_blacklist_rule r ON r.blacklist_id = b.id \
             WHERE b.user_id = $1 AND b.status = 1 AND r.product_code = $2 AND r.behavior_code = $3 \
             ORDER BY b.owner_type ASC, b.id ASC",
        )
        .bind(user_id)
        .bind(product_code)
        .bind(behavior_code)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 条目读写 —————————————————————

    /// 按 ID 取条目。
    pub async fn find_entry(&self, id: i64) -> Result<Option<BlacklistEntry>> {
        sqlx::query_as::<_, BlacklistEntry>(&format!("{ENTRY_SELECT} WHERE b.id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 同归属下的生效条目（重复加入走合并，§4.4）。
    pub async fn find_active_entry(
        &self,
        user_id: i32,
        owner_type: i32,
        owner_shop_id: i32,
    ) -> Result<Option<BlacklistEntry>> {
        sqlx::query_as::<_, BlacklistEntry>(&format!(
            "{ENTRY_SELECT} WHERE b.user_id = $1 AND b.owner_type = $2 AND b.owner_shop_id = $3 AND b.status = 1 LIMIT 1"
        ))
        .bind(user_id)
        .bind(owner_type)
        .bind(owner_shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新建条目，返回新 id。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_entry(
        &self,
        user_id: i32,
        owner_type: i32,
        owner_shop_id: i32,
        apply_mode: i32,
        reason: &str,
        source: i32,
        admin_id: i32,
        admin_name: &str,
        admin_type: &str,
        now: i64,
    ) -> Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO user_blacklist (user_id, owner_type, owner_shop_id, apply_mode, reason, source, status, create_admin_id, create_admin_name, create_admin_type, create_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, $9, $10, $10) RETURNING id",
        )
        .bind(user_id)
        .bind(owner_type)
        .bind(owner_shop_id)
        .bind(apply_mode)
        .bind(reason)
        .bind(source)
        .bind(admin_id)
        .bind(admin_name)
        .bind(admin_type)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 合并时更新原因/范围模式（§4.4：原因以新原因为准）。
    pub async fn update_entry_after_merge(
        &self,
        id: i64,
        reason: &str,
        apply_mode: i32,
        now: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE user_blacklist SET reason = $1, apply_mode = $2, update_time = $3 WHERE id = $4 AND status = 1",
        )
        .bind(reason)
        .bind(apply_mode)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 批量插入规则（唯一约束兜底幂等），返回实际新增行数。
    pub async fn insert_rules(
        &self,
        blacklist_id: i64,
        rules: &[(String, String)],
        now: i64,
    ) -> Result<u64> {
        let mut added = 0u64;
        for (product, behavior) in rules {
            let res = sqlx::query(
                "INSERT INTO user_blacklist_rule (blacklist_id, product_code, behavior_code, create_time) VALUES ($1, $2, $3, $4) ON CONFLICT (blacklist_id, product_code, behavior_code) DO NOTHING",
            )
            .bind(blacklist_id)
            .bind(product)
            .bind(behavior)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            added += res.rows_affected();
        }
        Ok(added)
    }

    /// 批量取多个条目的规则行（列表装配，禁止逐行查）。
    pub async fn rules_for(&self, blacklist_ids: &[i64]) -> Result<Vec<BlacklistRuleRow>> {
        if blacklist_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT blacklist_id, product_code, behavior_code FROM user_blacklist_rule WHERE blacklist_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in blacklist_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" ORDER BY id ASC");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 覆盖写入范围店铺（平台 selected 模式）。
    pub async fn replace_scope(&self, blacklist_id: i64, shop_ids: &[i32]) -> Result<()> {
        sqlx::query("DELETE FROM user_blacklist_scope_shop WHERE blacklist_id = $1")
            .bind(blacklist_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        for shop_id in shop_ids {
            sqlx::query(
                "INSERT INTO user_blacklist_scope_shop (blacklist_id, shop_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(blacklist_id)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    /// 批量取多个条目的范围店铺：返回 (blacklist_id, shop_id) 对。
    pub async fn scopes_for(&self, blacklist_ids: &[i64]) -> Result<Vec<(i64, i32)>> {
        if blacklist_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT blacklist_id, shop_id FROM user_blacklist_scope_shop WHERE blacklist_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in blacklist_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 移除条目（status=0 + 回填 remove_*），返回受影响行数。
    pub async fn mark_removed(
        &self,
        id: i64,
        admin_id: i32,
        admin_name: &str,
        remove_reason: &str,
        now: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE user_blacklist SET status = 0, remove_admin_id = $1, remove_admin_name = $2, remove_time = $3, remove_reason = $4, update_time = $3 WHERE id = $5 AND status = 1",
        )
        .bind(admin_id)
        .bind(admin_name)
        .bind(now)
        .bind(remove_reason)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 列表 —————————————————————

    /// 条目分页（关键词经 `user` 表 JOIN 匹配）。
    pub async fn paginate_entries(
        &self,
        filter: &BlacklistListFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<BlacklistEntry>)> {
        let mut cq: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM user_blacklist b LEFT JOIN \"user\" u ON u.user_id = b.user_id WHERE 1 = 1",
        );
        Self::push_list_filters(&mut cq, filter);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "{ENTRY_SELECT} LEFT JOIN \"user\" u ON u.user_id = b.user_id WHERE 1 = 1"
        ));
        Self::push_list_filters(&mut rq, filter);
        rq.push(" ORDER BY b.create_time DESC, b.id DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<BlacklistEntry> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 按过滤全量取条目（导出用，无分页；上限 10 万行防失控）。
    pub async fn list_all_entries(
        &self,
        filter: &BlacklistListFilter,
    ) -> Result<Vec<BlacklistEntry>> {
        let (_, rows) = self.paginate_entries(filter, 100_000, 0).await?;
        Ok(rows)
    }

    fn push_list_filters(qb: &mut QueryBuilder<'_, Postgres>, f: &BlacklistListFilter) {
        if let Some(shop) = f.visible_shop {
            qb.push(" AND ((b.owner_type = 2 AND b.owner_shop_id = ");
            qb.push_bind(shop);
            qb.push(") OR (b.owner_type = 1 AND b.apply_mode = 0))");
        }
        if let Some(user_id) = f.user_id {
            qb.push(" AND b.user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(keyword) = &f.keyword {
            let like = format!("%{keyword}%");
            qb.push(" AND (u.username ILIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR u.nickname ILIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR u.mobile ILIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR u.email ILIKE ");
            qb.push_bind(like);
            qb.push(")");
        }
        if let Some(product) = &f.product_code {
            qb.push(" AND EXISTS (SELECT 1 FROM user_blacklist_rule r WHERE r.blacklist_id = b.id AND r.product_code = ");
            qb.push_bind(product.clone());
            qb.push(")");
        }
        if let Some(behavior) = &f.behavior_code {
            qb.push(" AND EXISTS (SELECT 1 FROM user_blacklist_rule r WHERE r.blacklist_id = b.id AND r.behavior_code = ");
            qb.push_bind(behavior.clone());
            qb.push(")");
        }
        if let Some(owner_type) = f.owner_type {
            qb.push(" AND b.owner_type = ");
            qb.push_bind(owner_type);
        }
        if let Some(shop_id) = f.shop_id {
            qb.push(" AND b.owner_shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(status) = f.status {
            qb.push(" AND b.status = ");
            qb.push_bind(status);
        }
        if let Some(start) = f.create_time_start {
            qb.push(" AND b.create_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = f.create_time_end {
            qb.push(" AND b.create_time <= ");
            qb.push_bind(end);
        }
    }

    // ————————————————————— 流水 —————————————————————

    /// 写一条结构化流水（只追加）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_log(
        &self,
        blacklist_id: i64,
        user_id: i32,
        op_type: i32,
        op_content: &str,
        rules_snapshot: &serde_json::Value,
        scope_snapshot: &serde_json::Value,
        operator: &nextrade_core::models::member::user_blacklist::BlacklistOperator,
        ip: &str,
        now: i64,
    ) -> Result<i64> {
        sqlx::query_scalar(
            "INSERT INTO user_blacklist_log (blacklist_id, user_id, op_type, op_content, rules_snapshot, scope_snapshot, operator_id, operator_name, operator_type, operator_shop_id, ip, create_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
        )
        .bind(blacklist_id)
        .bind(user_id)
        .bind(op_type)
        .bind(op_content)
        .bind(rules_snapshot)
        .bind(scope_snapshot)
        .bind(operator.admin_id)
        .bind(&operator.admin_name)
        .bind(&operator.admin_type)
        .bind(operator.shop_id)
        .bind(ip)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 流水分页。
    pub async fn paginate_logs(
        &self,
        filter: &BlacklistLogFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<BlacklistLogRow>)> {
        let from = " FROM user_blacklist_log l LEFT JOIN user_blacklist b ON b.id = l.blacklist_id WHERE 1 = 1";
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("SELECT COUNT(*){from}"));
        Self::push_log_filters(&mut cq, filter);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!("{LOG_SELECT}{from}"));
        Self::push_log_filters(&mut rq, filter);
        rq.push(" ORDER BY l.create_time DESC, l.id DESC");
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<BlacklistLogRow> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_log_filters(qb: &mut QueryBuilder<'_, Postgres>, f: &BlacklistLogFilter) {
        if let Some(shop) = f.visible_shop {
            qb.push(" AND ((l.blacklist_id = 0 AND l.operator_shop_id = ");
            qb.push_bind(shop);
            qb.push(") OR (b.owner_type = 2 AND b.owner_shop_id = ");
            qb.push_bind(shop);
            qb.push(") OR (b.owner_type = 1 AND b.apply_mode = 0))");
        }
        if let Some(user_id) = f.user_id {
            qb.push(" AND l.user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(blacklist_id) = f.blacklist_id {
            qb.push(" AND l.blacklist_id = ");
            qb.push_bind(blacklist_id);
        }
        if let Some(op_type) = f.op_type {
            qb.push(" AND l.op_type = ");
            qb.push_bind(op_type);
        }
        if let Some(start) = f.create_time_start {
            qb.push(" AND l.create_time >= ");
            qb.push_bind(start);
        }
        if let Some(end) = f.create_time_end {
            qb.push(" AND l.create_time <= ");
            qb.push_bind(end);
        }
    }

    // ————————————————————— 会员 / 店铺查询 —————————————————————

    /// 按 ID 取会员（会员类型不限，口径同 shop_member find_user_by_id）。
    pub async fn find_user_by_id(&self, user_id: i32) -> Result<Option<BlacklistUserRow>> {
        sqlx::query_as::<_, BlacklistUserRow>(
            "SELECT user_id, username, mobile, email, nickname, avatar FROM \"user\" WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按手机号精确取会员。
    pub async fn find_user_by_mobile(&self, mobile: &str) -> Result<Option<BlacklistUserRow>> {
        sqlx::query_as::<_, BlacklistUserRow>(
            "SELECT user_id, username, mobile, email, nickname, avatar FROM \"user\" WHERE mobile = $1 AND user_type = 0 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按用户名精确取会员（大小写不敏感，同 user 仓储 find_by_username 口径）。
    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<BlacklistUserRow>> {
        sqlx::query_as::<_, BlacklistUserRow>(
            "SELECT user_id, username, mobile, email, nickname, avatar FROM \"user\" WHERE LOWER(username) = LOWER($1) AND user_type = 0 LIMIT 1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量取店铺名：返回 (shop_id, shop_title)。
    pub async fn shop_titles(&self, shop_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let ids: Vec<i32> = shop_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT shop_id, shop_title FROM shop WHERE shop_id IN (");
        let mut sep = qb.separated(", ");
        for id in &ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let rows: Vec<(i32, Option<String>)> =
            qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, title)| (id, title.unwrap_or_default()))
            .collect())
    }

    /// 店铺是否存在。
    pub async fn shop_exists(&self, shop_id: i32) -> Result<bool> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM shop WHERE shop_id = $1")
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0 > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
