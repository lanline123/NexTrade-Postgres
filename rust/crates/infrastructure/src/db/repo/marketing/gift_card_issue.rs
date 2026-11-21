//! 礼品卡发放域仓储（marketing::gift_card_issue，P1 发放闭环）。
//!
//! 涉及表：
//! - `gift_card_issue_batch` / `gift_card_issue_log` / `gift_card_usage_log`
//!   （迁移 `rust/deploy/pg/2026-11-18_gift_card_issue.sql`）
//! - `gift_cards`（含 P1 新增 `balance` 列）/ `gift_card_groups`
//!   （含 P1 新增 `exchange_mode` / `select_count` 列）
//! - `gift_card_order` / `order_item`（我的卡来源与兑换订单判定）
//! - `user`（Excel 名单匹配）
//!
//! 发放分配在单个事务内完成：`SELECT ... FOR UPDATE` 锁定可用卡 →
//! 写批次 → 逐人分配（`claimed_user_id` + `claimed_status = 2`）→ 写明细 →
//! 扣减卡组 `availability_stock`；可用卡不足时整单回滚（服务层转库存不足文案）。

use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, QueryBuilder};

use nextrade_core::{Error, Result};

/// 卡组发放视图行（含 P1 新增兑换字段）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct GroupIssueRow {
    pub card_groups_id: i32,
    pub group_type: i32,
    pub card_type: i32,
    pub card_group_name: String,
    pub image_url: Option<String>,
    pub face_amount: Option<Decimal>,
    pub status: i32,
    pub shop_id: Option<i32>,
    pub availability_stock: Option<i32>,
    pub exchange_mode: i32,
    pub select_count: i32,
}

/// 会员匹配行（Excel 名单解析用，不过滤 user_type，服务层分类）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct MemberRow {
    pub user_id: i32,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: String,
    pub status: Option<i32>,
    pub user_type: i32,
}

/// 发放批次联表行。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct IssueBatchRow {
    pub id: i64,
    pub card_groups_id: i32,
    pub card_group_name: Option<String>,
    pub shop_id: i32,
    pub issue_type: String,
    pub target_desc: serde_json::Value,
    pub cards_per_user: i32,
    pub target_count: i32,
    pub success_count: i32,
    pub fail_count: i32,
    pub admin_id: i32,
    pub remark: String,
    pub add_time: i64,
}

/// 发放明细联表行（卡号由服务层脱敏后回填 VO）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct IssueLogRow {
    pub id: i64,
    pub batch_id: i64,
    pub card_groups_id: i32,
    pub card_group_name: Option<String>,
    pub gift_cards_id: i32,
    pub card_number: Option<String>,
    pub user_id: i32,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub issue_time: i64,
}

/// 我的礼品卡联表行。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct MyCardRow {
    pub gift_cards_id: i32,
    pub card_groups_id: i32,
    pub card_group_name: Option<String>,
    pub image_url: Option<String>,
    pub group_type: i32,
    pub card_type: i32,
    pub card_number: String,
    pub gift_card_sn: Option<String>,
    pub face_amount: Option<Decimal>,
    pub balance: Option<Decimal>,
    pub status: Option<i32>,
    pub expired_time: i32,
    pub used_time: Option<i32>,
    pub add_time: Option<i32>,
    pub has_issue: bool,
    pub has_order: bool,
    /// 适用范围（我方新增，卡组 scope_type 列）：0 全部 / 1 指定商品 / 2 指定品类
    pub scope_type: i32,
}

/// 消费流水行。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct UsageLogRow {
    pub id: i64,
    pub gift_cards_id: i32,
    pub user_id: i32,
    pub order_id: i32,
    pub r#type: i16,
    pub amount: Decimal,
    pub balance_before: Option<Decimal>,
    pub balance_after: Option<Decimal>,
    pub add_time: i64,
}

/// Excel 发卡指定卡行（卡号+卡密命中后校验用）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct SpecifiedCardRow {
    pub gift_cards_id: i32,
    pub status: Option<i32>,
    pub is_available: Option<i32>,
    pub claimed_user_id: Option<i32>,
    pub user_id: Option<i32>,
}

/// 可用储值卡行（结算选择器 / 抵扣锁定共用；balance 已 COALESCE 面额）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct AvailableStoredCardRow {
    pub gift_cards_id: i32,
    pub card_groups_id: i32,
    pub card_number: String,
    pub gift_card_sn: Option<String>,
    pub card_group_name: Option<String>,
    pub face_amount: Option<Decimal>,
    pub balance: Option<Decimal>,
    pub expired_time: i32,
    /// 适用范围（我方新增，卡组 scope_type 列）：0 全部 / 1 指定商品 / 2 指定品类
    pub scope_type: i32,
}

/// 结算抵扣结果。
#[derive(Debug, Clone, Copy, Default)]
pub struct GiftCardDeductResult {
    pub gift_cards_id: i32,
    /// 本次抵扣金额（正数）
    pub amount: Decimal,
    pub balance_before: Decimal,
    pub balance_after: Decimal,
}

/// 分配结果。
#[derive(Debug, Clone, Default)]
pub struct IssueAllocation {
    pub batch_id: i64,
    /// `(user_id, 分到的 gift_cards_id 列表)`，顺序与入参会员顺序一致
    pub assignments: Vec<(i32, Vec<i32>)>,
}

/// 分配失败（可用卡不足，整单未落库）。
#[derive(Debug, Clone, Copy)]
pub struct InsufficientStock {
    pub available: i64,
    pub need: i64,
}

/// 礼品卡发放域仓储。
#[derive(Clone)]
pub struct GiftCardIssueRepository {
    pool: PgPool,
}

impl GiftCardIssueRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 卡组 / 库存 —————————————————————

    /// 卡组发放视图（含兑换字段）。
    pub async fn find_group(&self, card_groups_id: i32) -> Result<Option<GroupIssueRow>> {
        sqlx::query_as::<_, GroupIssueRow>(
            "SELECT card_groups_id, group_type, card_type, card_group_name, image_url, \
                    face_amount, status, shop_id, availability_stock, exchange_mode, select_count \
             FROM gift_card_groups WHERE card_groups_id = $1 LIMIT 1",
        )
        .bind(card_groups_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 当前可用卡数（可分配口径：可用、未使用、无归属）。
    pub async fn count_available_cards(&self, card_groups_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gift_cards \
             WHERE card_groups_id = $1 AND is_available = 1 AND status = 0 \
               AND claimed_user_id IS NULL AND user_id IS NULL",
        )
        .bind(card_groups_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— Excel 名单匹配 —————————————————————

    /// 按会员 ID 匹配（全部用户，服务层按 user_type/status 分类）。
    pub async fn find_member_by_id(&self, user_id: i32) -> Result<Option<MemberRow>> {
        sqlx::query_as::<_, MemberRow>(
            "SELECT user_id, username, nickname, mobile, status, user_type FROM \"user\" WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按手机号精确匹配。
    pub async fn find_member_by_mobile(&self, mobile: &str) -> Result<Option<MemberRow>> {
        sqlx::query_as::<_, MemberRow>(
            "SELECT user_id, username, nickname, mobile, status, user_type FROM \"user\" WHERE mobile = $1 ORDER BY user_id ASC LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按会员名精确匹配（用户名或昵称，Excel 发卡「会员名」列口径）。
    pub async fn find_member_by_username(&self, username: &str) -> Result<Option<MemberRow>> {
        sqlx::query_as::<_, MemberRow>(
            "SELECT user_id, username, nickname, mobile, status, user_type FROM \"user\" WHERE username = $1 OR nickname = $1 ORDER BY user_id ASC LIMIT 1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 发放分配（事务 + 行锁） —————————————————————

    /// 事务内分配：锁定可用卡 → 写批次/明细 → 置领取归属 → 扣卡组库存。
    ///
    /// 一人一卡（D3 定案）：可用卡不足 `user_ids.len()` 时整单回滚并返回
    /// [`InsufficientStock`]（不产生批次与明细）。
    pub async fn allocate_issue(
        &self,
        group: &GroupIssueRow,
        user_ids: &[i32],
        issue_type: &str,
        target_desc: &serde_json::Value,
        target_count: i32,
        fail_count: i32,
        admin_id: i32,
        remark: &str,
        now: i64,
    ) -> std::result::Result<IssueAllocation, AllocateError> {
        let cards_per_user = 1_i32;
        let need = user_ids.len() as i64;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AllocateError::Db(map_err(e)))?;
        // 行锁锁定可用卡（升序取卡，与方案口径一致）
        let card_ids: Vec<i32> = sqlx::query_scalar(
            "SELECT gift_cards_id FROM gift_cards \
             WHERE card_groups_id = $1 AND is_available = 1 AND status = 0 \
               AND claimed_user_id IS NULL AND user_id IS NULL \
             ORDER BY gift_cards_id ASC LIMIT $2 FOR UPDATE",
        )
        .bind(group.card_groups_id)
        .bind(need)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_err)
        .map_err(AllocateError::Db)?;
        if (card_ids.len() as i64) < need {
            // tx drop 即回滚，整单拦截
            return Err(AllocateError::Insufficient(InsufficientStock {
                available: card_ids.len() as i64,
                need,
            }));
        }
        let success_count = i32::try_from(user_ids.len()).unwrap_or(i32::MAX);
        let batch_id: i64 = sqlx::query_scalar(
            "INSERT INTO gift_card_issue_batch \
               (card_groups_id, shop_id, issue_type, target_desc, cards_per_user, \
                target_count, success_count, fail_count, admin_id, remark, add_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
        )
        .bind(group.card_groups_id)
        .bind(group.shop_id.unwrap_or(0))
        .bind(issue_type)
        .bind(target_desc)
        .bind(cards_per_user)
        .bind(target_count)
        .bind(success_count)
        .bind(fail_count)
        .bind(admin_id)
        .bind(remark)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)
        .map_err(AllocateError::Db)?;

        let mut assignments: Vec<(i32, Vec<i32>)> = Vec::with_capacity(user_ids.len());
        let mut cursor = card_ids.iter();
        for &user_id in user_ids {
            let mut mine = Vec::with_capacity(cards_per_user as usize);
            for _ in 0..cards_per_user {
                let Some(&card_id) = cursor.next() else { break };
                sqlx::query(
                    "UPDATE gift_cards SET claimed_user_id = $1, claimed_status = 2 WHERE gift_cards_id = $2",
                )
                .bind(user_id)
                .bind(card_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)
                .map_err(AllocateError::Db)?;
                sqlx::query(
                    "INSERT INTO gift_card_issue_log (batch_id, card_groups_id, gift_cards_id, user_id, issue_time) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(batch_id)
                .bind(group.card_groups_id)
                .bind(card_id)
                .bind(user_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_err)
                .map_err(AllocateError::Db)?;
                mine.push(card_id);
            }
            assignments.push((user_id, mine));
        }
        let assigned = i32::try_from(card_ids.len()).unwrap_or(i32::MAX);
        sqlx::query(
            "UPDATE gift_card_groups SET availability_stock = GREATEST(COALESCE(availability_stock, 0) - $1, 0) WHERE card_groups_id = $2",
        )
        .bind(assigned)
        .bind(group.card_groups_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)
        .map_err(AllocateError::Db)?;
        tx.commit().await.map_err(map_err).map_err(AllocateError::Db)?;
        Ok(IssueAllocation {
            batch_id,
            assignments,
        })
    }

    // ————————————————————— 批次 / 明细分页 —————————————————————

    /// 发放批次分页（卡组 / 类型 / 时间筛选）。
    pub async fn paginate_batches(
        &self,
        card_groups_id: Option<i32>,
        issue_type: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<IssueBatchRow>)> {
        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM gift_card_issue_batch b WHERE 1 = 1");
        push_batch_filters(&mut count_qb, card_groups_id, issue_type, start_time, end_time);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT b.id, b.card_groups_id, g.card_group_name, b.shop_id, b.issue_type, \
                    b.target_desc, b.cards_per_user, b.target_count, b.success_count, \
                    b.fail_count, b.admin_id, b.remark, b.add_time \
             FROM gift_card_issue_batch b \
             LEFT JOIN gift_card_groups g ON g.card_groups_id = b.card_groups_id \
             WHERE 1 = 1",
        );
        push_batch_filters(&mut qb, card_groups_id, issue_type, start_time, end_time);
        qb.push(" ORDER BY b.id DESC LIMIT ")
            .push_bind(size as i64)
            .push(" OFFSET ")
            .push_bind(((page - 1) * size) as i64);
        let rows = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 发放明细分页（批次 / 卡组 / 会员筛选，联表回填会员与卡号）。
    pub async fn paginate_logs(
        &self,
        batch_id: Option<i64>,
        card_groups_id: Option<i32>,
        user_id: Option<i32>,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<IssueLogRow>)> {
        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM gift_card_issue_log l WHERE 1 = 1");
        push_log_filters(&mut count_qb, batch_id, card_groups_id, user_id);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT l.id, l.batch_id, l.card_groups_id, g.card_group_name, l.gift_cards_id, \
                    c.card_number, l.user_id, u.username, u.nickname, u.mobile, l.issue_time \
             FROM gift_card_issue_log l \
             LEFT JOIN gift_card_groups g ON g.card_groups_id = l.card_groups_id \
             LEFT JOIN gift_cards c ON c.gift_cards_id = l.gift_cards_id \
             LEFT JOIN \"user\" u ON u.user_id = l.user_id \
             WHERE 1 = 1",
        );
        push_log_filters(&mut qb, batch_id, card_groups_id, user_id);
        qb.push(" ORDER BY l.id DESC LIMIT ")
            .push_bind(size as i64)
            .push(" OFFSET ")
            .push_bind(((page - 1) * size) as i64);
        let rows = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    // ————————————————————— 我的礼品卡 —————————————————————

    /// 我的卡分页：归属 = claimed_user_id 优先、其次 user_id（同一条件二选一命中即可）。
    pub async fn paginate_my_cards(
        &self,
        user_id: i32,
        group_type: Option<i32>,
        state: Option<&str>,
        now: i32,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<MyCardRow>)> {
        let mut count_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT COUNT(*) FROM gift_cards c \
             INNER JOIN gift_card_groups g ON g.card_groups_id = c.card_groups_id \
             WHERE (c.claimed_user_id = ",
        );
        count_qb.push_bind(user_id);
        count_qb.push(" OR c.user_id = ").push_bind(user_id).push(")");
        push_my_filters(&mut count_qb, group_type, state, now);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT c.gift_cards_id, c.card_groups_id, g.card_group_name, g.image_url, \
                    g.group_type, g.card_type, c.card_number, c.gift_card_sn, g.face_amount, \
                    c.balance, c.status, c.expired_time, c.used_time, c.add_time, g.scope_type, \
                    EXISTS(SELECT 1 FROM gift_card_issue_log l WHERE l.gift_cards_id = c.gift_cards_id) AS has_issue, \
                    EXISTS(SELECT 1 FROM gift_card_order o WHERE o.card_id = c.gift_cards_id) AS has_order \
             FROM gift_cards c \
             INNER JOIN gift_card_groups g ON g.card_groups_id = c.card_groups_id \
             WHERE (c.claimed_user_id = ",
        );
        qb.push_bind(user_id);
        qb.push(" OR c.user_id = ").push_bind(user_id).push(")");
        push_my_filters(&mut qb, group_type, state, now);
        qb.push(" ORDER BY c.gift_cards_id DESC LIMIT ")
            .push_bind(size as i64)
            .push(" OFFSET ")
            .push_bind(((page - 1) * size) as i64);
        let rows = qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 我的卡详情：按 id 查且必须归属本人（未归属 ⇒ None，服务层按不存在处理）。
    pub async fn find_my_card(&self, user_id: i32, gift_cards_id: i32) -> Result<Option<MyCardRow>> {
        sqlx::query_as::<_, MyCardRow>(
            "SELECT c.gift_cards_id, c.card_groups_id, g.card_group_name, g.image_url, \
                    g.group_type, g.card_type, c.card_number, c.gift_card_sn, g.face_amount, \
                    c.balance, c.status, c.expired_time, c.used_time, c.add_time, g.scope_type, \
                    EXISTS(SELECT 1 FROM gift_card_issue_log l WHERE l.gift_cards_id = c.gift_cards_id) AS has_issue, \
                    EXISTS(SELECT 1 FROM gift_card_order o WHERE o.card_id = c.gift_cards_id) AS has_order \
             FROM gift_cards c \
             INNER JOIN gift_card_groups g ON g.card_groups_id = c.card_groups_id \
             WHERE c.gift_cards_id = $1 AND (c.claimed_user_id = $2 OR c.user_id = $2) LIMIT 1",
        )
        .bind(gift_cards_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 消费流水（最新在前，上限 100 条）。
    pub async fn list_usage_logs(&self, gift_cards_id: i32) -> Result<Vec<UsageLogRow>> {
        sqlx::query_as::<_, UsageLogRow>(
            "SELECT id, gift_cards_id, user_id, order_id, type, amount, balance_before, balance_after, add_time \
             FROM gift_card_usage_log WHERE gift_cards_id = $1 ORDER BY id DESC LIMIT 100",
        )
        .bind(gift_cards_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— Excel 发卡（原版 issueByExcel 口径） —————————————————————

    /// 按卡号+卡密查指定卡（限本卡组；服务层校验未使用/未发放）。
    pub async fn find_specified_card(
        &self,
        card_groups_id: i32,
        card_number: &str,
        card_password: &str,
    ) -> Result<Option<SpecifiedCardRow>> {
        sqlx::query_as::<_, SpecifiedCardRow>(
            "SELECT gift_cards_id, status, is_available, claimed_user_id, user_id \
             FROM gift_cards \
             WHERE card_groups_id = $1 AND card_number = $2 AND card_password = $3 LIMIT 1",
        )
        .bind(card_groups_id)
        .bind(card_number)
        .bind(card_password)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 事务内发放指定卡（Excel 发卡）：逐张行锁复核（status=0、is_available=1、
    /// 无归属）→ 写批次 → 置领取归属 → 写明细 → 扣卡组库存。
    /// 复核不过的卡跳过（计入返回的失败数差值），其余继续。
    /// 返回 `(batch_id, 实际成功数)`；无一成功时不写批次，返回 `(0, 0)`。
    pub async fn allocate_specified_cards(
        &self,
        group: &GroupIssueRow,
        pairs: &[(i32, i32)],
        target_desc: &serde_json::Value,
        target_count: i32,
        fail_count: i32,
        admin_id: i32,
        remark: &str,
        now: i64,
    ) -> Result<(i64, i32)> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let mut ok_pairs: Vec<(i32, i32)> = Vec::with_capacity(pairs.len());
        for &(user_id, card_id) in pairs {
            let row: Option<(i32,)> = sqlx::query_as(
                "SELECT gift_cards_id FROM gift_cards \
                 WHERE gift_cards_id = $1 AND card_groups_id = $2 AND is_available = 1 AND status = 0 \
                   AND claimed_user_id IS NULL AND user_id IS NULL FOR UPDATE",
            )
            .bind(card_id)
            .bind(group.card_groups_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_err)?;
            if row.is_some() {
                ok_pairs.push((user_id, card_id));
            }
        }
        if ok_pairs.is_empty() {
            return Ok((0, 0));
        }
        let success_count = i32::try_from(ok_pairs.len()).unwrap_or(i32::MAX);
        let total_fail = fail_count + i32::try_from(pairs.len() - ok_pairs.len()).unwrap_or(0);
        let batch_id: i64 = sqlx::query_scalar(
            "INSERT INTO gift_card_issue_batch \
               (card_groups_id, shop_id, issue_type, target_desc, cards_per_user, \
                target_count, success_count, fail_count, admin_id, remark, add_time) \
             VALUES ($1, $2, 'EXCEL', $3, 1, $4, $5, $6, $7, $8, $9) RETURNING id",
        )
        .bind(group.card_groups_id)
        .bind(group.shop_id.unwrap_or(0))
        .bind(target_desc)
        .bind(target_count)
        .bind(success_count)
        .bind(total_fail)
        .bind(admin_id)
        .bind(remark)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        for &(user_id, card_id) in &ok_pairs {
            sqlx::query(
                "UPDATE gift_cards SET claimed_user_id = $1, claimed_status = 2 WHERE gift_cards_id = $2",
            )
            .bind(user_id)
            .bind(card_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
            sqlx::query(
                "INSERT INTO gift_card_issue_log (batch_id, card_groups_id, gift_cards_id, user_id, issue_time) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(batch_id)
            .bind(group.card_groups_id)
            .bind(card_id)
            .bind(user_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        sqlx::query(
            "UPDATE gift_card_groups SET availability_stock = GREATEST(COALESCE(availability_stock, 0) - $1, 0) WHERE card_groups_id = $2",
        )
        .bind(success_count)
        .bind(group.card_groups_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok((batch_id, success_count))
    }

    // ————————————————————— 结算抵扣（储值卡单卡，D5） —————————————————————

    /// 我的可用储值卡（结算页选择器数据）：归属本人、储值卡组、status=0、
    /// is_available=1、未过期、余额 > 0（balance 为 NULL 时按面额计，
    /// 兼容迁移前制卡未回填的行）。
    pub async fn list_available_stored_cards(
        &self,
        user_id: i32,
        now: i32,
    ) -> Result<Vec<AvailableStoredCardRow>> {
        sqlx::query_as::<_, AvailableStoredCardRow>(
            "SELECT c.gift_cards_id, c.card_groups_id, c.card_number, c.gift_card_sn, g.card_group_name, \
                    g.face_amount, COALESCE(c.balance, g.face_amount) AS balance, c.expired_time, g.scope_type \
             FROM gift_cards c \
             INNER JOIN gift_card_groups g ON g.card_groups_id = c.card_groups_id \
             WHERE (c.claimed_user_id = $1 OR c.user_id = $1) \
               AND g.group_type = 1 AND c.status = 0 AND c.is_available = 1 \
               AND (c.expired_time = 0 OR c.expired_time >= $2) \
               AND COALESCE(c.balance, g.face_amount, 0) > 0 \
             ORDER BY c.gift_cards_id DESC",
        )
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 查本人储值卡所属卡组（下单抵扣前算适用范围上限用；归属谓词与
    /// `deduct_stored_card` 一致。不存在/不归属返回 None）。
    pub async fn find_owned_card_group_id(
        &self,
        user_id: i32,
        gift_cards_id: Option<i32>,
        gift_card_sn: Option<&str>,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT c.card_groups_id FROM gift_cards c \
             INNER JOIN gift_card_groups g ON g.card_groups_id = c.card_groups_id \
             WHERE (c.claimed_user_id = $1 OR c.user_id = $1) AND g.group_type = 1 \
               AND (($2::int IS NOT NULL AND c.gift_cards_id = $2) \
                 OR ($3::text IS NOT NULL AND c.gift_card_sn = $3)) \
             LIMIT 1",
        )
        .bind(user_id)
        .bind(gift_cards_id)
        .bind(gift_card_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    /// 结算抵扣（事务 + 行锁）：按 id 或 sn 锁定本人储值卡 →
    /// amount = min(卡内余额, max_amount) → 扣 balance（扣空置 status=1、
    /// used_time）→ 写 gift_card_usage_log（type=1，amount 为负数表示支出）。
    /// 卡不存在/不归属/不可用/余额为 0 时返回 None（服务层决定报错或忽略）。
    pub async fn deduct_stored_card(
        &self,
        user_id: i32,
        gift_cards_id: Option<i32>,
        gift_card_sn: Option<&str>,
        order_id: i32,
        max_amount: Decimal,
        now: i64,
    ) -> Result<Option<GiftCardDeductResult>> {
        if max_amount <= Decimal::ZERO {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let row: Option<AvailableStoredCardRow> = sqlx::query_as(
            "SELECT c.gift_cards_id, c.card_groups_id, c.card_number, c.gift_card_sn, g.card_group_name, \
                    g.face_amount, COALESCE(c.balance, g.face_amount) AS balance, c.expired_time, g.scope_type \
             FROM gift_cards c \
             INNER JOIN gift_card_groups g ON g.card_groups_id = c.card_groups_id \
             WHERE (c.claimed_user_id = $1 OR c.user_id = $1) AND g.group_type = 1 \
               AND (($2::int IS NOT NULL AND c.gift_cards_id = $2) \
                 OR ($3::text IS NOT NULL AND c.gift_card_sn = $3)) \
             LIMIT 1 FOR UPDATE OF c",
        )
        .bind(user_id)
        .bind(gift_cards_id)
        .bind(gift_card_sn)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let Some(card) = row else {
            return Ok(None);
        };
        let now_i32 = i32::try_from(now).unwrap_or(i32::MAX);
        let balance = card.balance.unwrap_or(Decimal::ZERO);
        if balance <= Decimal::ZERO
            || (card.expired_time != 0 && card.expired_time < now_i32)
        {
            return Ok(None);
        }
        // 可用性复核（行锁后以最新状态为准）
        let status_row: Option<(Option<i32>, Option<i32>)> = sqlx::query_as(
            "SELECT status, is_available FROM gift_cards WHERE gift_cards_id = $1",
        )
        .bind(card.gift_cards_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        if status_row != Some((Some(0), Some(1))) {
            return Ok(None);
        }
        let amount = balance.min(max_amount);
        let after = balance - amount;
        let exhausted = after <= Decimal::ZERO;
        sqlx::query(
            "UPDATE gift_cards SET balance = $1, status = $2, used_time = $3 WHERE gift_cards_id = $4",
        )
        .bind(after)
        .bind(if exhausted { 1_i32 } else { 0_i32 })
        .bind(if exhausted { Some(now_i32) } else { None })
        .bind(card.gift_cards_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        sqlx::query(
            "INSERT INTO gift_card_usage_log (gift_cards_id, user_id, order_id, type, amount, balance_before, balance_after, add_time) \
             VALUES ($1, $2, $3, 1, $4, $5, $6, $7)",
        )
        .bind(card.gift_cards_id)
        .bind(user_id)
        .bind(order_id)
        .bind(-amount)
        .bind(balance)
        .bind(after)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(Some(GiftCardDeductResult {
            gift_cards_id: card.gift_cards_id,
            amount,
            balance_before: balance,
            balance_after: after,
        }))
    }

    /// 抵扣流水合计（按记账订单 id 直查，不做归属解析）。
    async fn deducted_sum_for_order_id(&self, order_id: i32) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT COALESCE(SUM(-amount), 0) FROM gift_card_usage_log \
             WHERE order_id = $1 AND type = 1",
        )
        .bind(order_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(Decimal::ZERO).max(Decimal::ZERO))
    }

    /// 订单头归属信息（父单 id 与礼品卡分摊额）。
    async fn order_attribution_row(
        &self,
        order_id: i32,
    ) -> Result<Option<(Option<i32>, Option<Decimal>)>> {
        let row: Option<(Option<i32>, Option<Decimal>)> = sqlx::query_as(
            "SELECT parent_order_id, gift_card_amount FROM \"order\" WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 子单份额清单（按父单 id 查子单头分摊，份额 > 0 才入列）。
    async fn child_shares(&self, parent_order_id: i32) -> Result<Vec<(i32, Decimal)>> {
        let rows: Vec<(i32, Option<Decimal>)> = sqlx::query_as(
            "SELECT order_id, gift_card_amount FROM \"order\" \
             WHERE parent_order_id = $1 AND gift_card_amount > 0 ORDER BY order_id ASC",
        )
        .bind(parent_order_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, amt)| (id, amt.unwrap_or(Decimal::ZERO)))
            .collect())
    }

    /// 归属解析（子单退款精确化）：返回 (记账订单 id, [(归属订单 id, 份额)])。
    ///
    /// 抵扣流水只记在建单订单（拆单前父单）名下，退款/取消的归属
    /// 按订单头 `gift_card_amount` 的拆单分摊落到各子单（拆单侧
    /// `compute_child_amounts` 按商品金额比例分摊、尾差归最后
    /// 子单，Σ 与父单头额恒等；父单头额与流水同源同额）：
    /// - 自身名下有流水：未拆单 → [(自身, 流水总额)]；已拆单 →
    ///   各子单份额（再以流水总额校正尾差，防两边口径漂移）；
    /// - 自身是子单（parent_order_id 指向记账订单）→ [(自身,
    ///   min(子单头分摊, 父单流水总额))]；
    /// - 无抵扣流水（未用卡/兑换流——兑换单头虽有
    ///   gift_card_amount 但不写 usage_log）→ 空，恒不退储值余额。
    async fn attribution_units(&self, order_id: i32) -> Result<(i32, Vec<(i32, Decimal)>)> {
        let own_total = self.deducted_sum_for_order_id(order_id).await?;
        if own_total > Decimal::ZERO {
            let mut children = self.child_shares(order_id).await?;
            if children.is_empty() {
                return Ok((order_id, vec![(order_id, own_total)]));
            }
            let sum: Decimal = children.iter().map(|(_, s)| *s).sum();
            if sum != own_total {
                let diff = own_total - sum;
                let last = children.last_mut().expect("children non-empty");
                last.1 = (last.1 + diff).max(Decimal::ZERO);
            }
            return Ok((order_id, children));
        }
        if let Some((parent, gift)) = self.order_attribution_row(order_id).await? {
            if let Some(parent_id) = parent.filter(|p| *p > 0) {
                let family_total = self.deducted_sum_for_order_id(parent_id).await?;
                if family_total > Decimal::ZERO {
                    let share = gift
                        .unwrap_or(Decimal::ZERO)
                        .min(family_total)
                        .max(Decimal::ZERO);
                    if share > Decimal::ZERO {
                        return Ok((parent_id, vec![(order_id, share)]));
                    }
                }
            }
        }
        Ok((order_id, Vec::new()))
    }

    /// 归属订单已返还的储值卡金额（type=2 流水按归属订单 id 合计）。
    pub async fn returned_total_for_order(&self, order_id: i32) -> Result<Decimal> {
        let row: (Option<Decimal>,) = sqlx::query_as(
            "SELECT COALESCE(SUM(amount), 0) FROM gift_card_usage_log \
             WHERE order_id = $1 AND type = 2",
        )
        .bind(order_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.unwrap_or(Decimal::ZERO).max(Decimal::ZERO))
    }

    /// 返还执行核心（单事务）：按归属单位逐个返还，`cap` 为本次
    /// 总返还上限（None = 各单位剩余额度全额返还）。
    ///
    /// 防重口径（替代旧「任一 type=2 即整单跳过」的粗幂等）：逐
    /// 单位以「该单位 type=2 合计 ≤ 其份额」硬钳制，逐卡再以
    /// 「该卡在整个归属族（记账订单 + 各归属单位）名下 type=2
    /// 合计 ≤ 其抵扣原额」钳制——存量按父单 id 整笔返还过的族
    /// 不会二次返还。先锁全部相关卡行（FOR UPDATE）再读已返还
    /// 合计，使并发完成事件在卡行锁上串行。返还写 type=2 流水
    /// （amount 为正数、order_id 记归属单位），卡 status 回 0
    /// （原已用完的卡恢复可用）。返回实际返还总额。
    async fn execute_card_return(
        &self,
        order_id: i32,
        cap: Option<Decimal>,
        now: i64,
    ) -> Result<Decimal> {
        let (root_id, units) = self.attribution_units(order_id).await?;
        if units.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let logs: Vec<UsageLogRow> = sqlx::query_as(
            "SELECT id, gift_cards_id, user_id, order_id, type, amount, balance_before, balance_after, add_time \
             FROM gift_card_usage_log WHERE order_id = $1 AND type = 1 ORDER BY id ASC",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        if logs.is_empty() {
            return Ok(Decimal::ZERO);
        }
        // 归属族 id 集合（记账订单 + 各归属单位）：逐卡已返还合计
        // 按此集合统计，兼容存量按记账订单 id 落的 type=2 流水。
        let mut family_ids: Vec<i32> = units.iter().map(|(id, _)| *id).collect();
        if !family_ids.contains(&root_id) {
            family_ids.push(root_id);
        }
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        // 先逐卡行锁（互斥点），再读已返还合计（见方法文档）。
        let mut card_balance: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for log in &logs {
            if card_balance.contains_key(&log.gift_cards_id) {
                continue;
            }
            let cur: Option<(Option<Decimal>,)> = sqlx::query_as(
                "SELECT balance FROM gift_cards WHERE gift_cards_id = $1 FOR UPDATE",
            )
            .bind(log.gift_cards_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_err)?;
            if let Some((balance,)) = cur {
                card_balance.insert(
                    log.gift_cards_id,
                    balance.unwrap_or(Decimal::ZERO),
                );
            }
        }
        // 逐卡已返还合计（归属族口径）。
        let mut card_returned: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for log in &logs {
            if card_returned.contains_key(&log.gift_cards_id) {
                continue;
            }
            let rows: Vec<(i32, Option<Decimal>)> = sqlx::query_as(
                "SELECT order_id, amount FROM gift_card_usage_log \
                 WHERE gift_cards_id = $1 AND type = 2",
            )
            .bind(log.gift_cards_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(map_err)?;
            let sum: Decimal = rows
                .into_iter()
                .filter(|(oid, _)| family_ids.contains(oid))
                .map(|(_, amt)| amt.unwrap_or(Decimal::ZERO))
                .sum();
            card_returned.insert(log.gift_cards_id, sum.max(Decimal::ZERO));
        }
        let mut budget = cap;
        let mut total = Decimal::ZERO;
        for (unit_id, share) in &units {
            if budget.map(|b| b <= Decimal::ZERO).unwrap_or(false) {
                break;
            }
            let already: (Option<Decimal>,) = sqlx::query_as(
                "SELECT COALESCE(SUM(amount), 0) FROM gift_card_usage_log \
                 WHERE order_id = $1 AND type = 2",
            )
            .bind(*unit_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_err)?;
            let already = already.0.unwrap_or(Decimal::ZERO).max(Decimal::ZERO);
            let mut want = (*share - already).max(Decimal::ZERO);
            if let Some(b) = budget {
                want = want.min(b);
            }
            if want <= Decimal::ZERO {
                continue;
            }
            // 本单位返还额按记账流水逐卡落（现行抵扣为单卡单笔，
            // 即整额落该卡；多笔时按流水顺序、逐卡不超其未返还
            // 原额）。
            let mut remaining = want;
            for log in &logs {
                if remaining <= Decimal::ZERO {
                    break;
                }
                let Some(balance) = card_balance.get(&log.gift_cards_id).copied() else {
                    continue;
                };
                let card_cap = (-log.amount
                    - card_returned
                        .get(&log.gift_cards_id)
                        .copied()
                        .unwrap_or(Decimal::ZERO))
                .max(Decimal::ZERO);
                let give = remaining.min(card_cap);
                if give <= Decimal::ZERO {
                    continue;
                }
                let after = balance + give;
                sqlx::query(
                    "UPDATE gift_cards SET balance = $1, status = 0 WHERE gift_cards_id = $2",
                )
                .bind(after)
                .bind(log.gift_cards_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
                sqlx::query(
                    "INSERT INTO gift_card_usage_log (gift_cards_id, user_id, order_id, type, amount, balance_before, balance_after, add_time) \
                     VALUES ($1, $2, $3, 2, $4, $5, $6, $7)",
                )
                .bind(log.gift_cards_id)
                .bind(log.user_id)
                .bind(*unit_id)
                .bind(give)
                .bind(balance)
                .bind(after)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
                card_balance.insert(log.gift_cards_id, after);
                *card_returned.entry(log.gift_cards_id).or_default() += give;
                remaining -= give;
                total += give;
                if let Some(b) = budget.as_mut() {
                    *b -= give;
                }
            }
        }
        tx.commit().await.map_err(map_err)?;
        Ok(total)
    }

    /// 订单取消/手工完成点返还（全额口径，幂等）：按归属单位逐个
    /// 返还各自剩余份额——未拆单即整笔原额返还（与历史口径
    /// 一致）；父单逐子单落账；子单只返本子单份额。返还后卡
    /// status 回 0。返回返还总金额。
    pub async fn return_stored_cards_for_order(
        &self,
        order_id: i32,
        now: i64,
    ) -> Result<Decimal> {
        self.execute_card_return(order_id, None, now).await
    }

    /// 退款完成点按额返还（子单/部分退款精确化）：返还金额由
    /// 服务层按累计比例算出（见 services/payment/refund_apply 的
    /// `gift_card_refund_part`），此处按归属落卡并钳制不超归属
    /// 份额的剩余额度。返回实际返还总额。
    pub async fn return_stored_cards_amount(
        &self,
        order_id: i32,
        amount: Decimal,
        now: i64,
    ) -> Result<Decimal> {
        if amount <= Decimal::ZERO {
            return Ok(Decimal::ZERO);
        }
        self.execute_card_return(order_id, Some(amount), now).await
    }

    /// 订单归属的礼品卡抵扣份额（子单退款精确化口径）。退款资金
    /// 执行按订单支付构成拆分现金部分时使用：未拆单 = 其名下
    /// type=1 流水合计（历史口径）；子单 = 子单头
    /// `gift_card_amount`（拆单分摊值）与父单流水总额的较小者；
    /// 无抵扣（兑换流等）= 0。卡部分由完成点钩子按归属返还
    /// （全额口径 `return_stored_cards_for_order` / 按额口径
    /// `return_stored_cards_amount`）。
    pub async fn deducted_total_for_order(&self, order_id: i32) -> Result<Decimal> {
        let own = self.deducted_sum_for_order_id(order_id).await?;
        if own > Decimal::ZERO {
            return Ok(own);
        }
        if let Some((parent, gift)) = self.order_attribution_row(order_id).await? {
            if let Some(parent_id) = parent.filter(|p| *p > 0) {
                let family = self.deducted_sum_for_order_id(parent_id).await?;
                if family > Decimal::ZERO {
                    return Ok(gift
                        .unwrap_or(Decimal::ZERO)
                        .min(family)
                        .max(Decimal::ZERO));
                }
            }
        }
        Ok(Decimal::ZERO)
    }

    /// 兑换订单 id（gift_card_order 与 order_item 两处关联合并去重）。
    pub async fn find_exchange_order_ids(&self, gift_cards_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT order_id FROM gift_card_order WHERE card_id = $1 \
             UNION SELECT order_id FROM order_item WHERE gift_card_id = $1",
        )
        .bind(gift_cards_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

/// 分配错误：库存不足（业务）或数据库错误。
#[derive(Debug)]
pub enum AllocateError {
    Insufficient(InsufficientStock),
    Db(Error),
}

fn push_batch_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    card_groups_id: Option<i32>,
    issue_type: Option<&str>,
    start_time: Option<i64>,
    end_time: Option<i64>,
) {
    if let Some(v) = card_groups_id {
        qb.push(" AND b.card_groups_id = ").push_bind(v);
    }
    if let Some(v) = issue_type {
        qb.push(" AND b.issue_type = ").push_bind(v.to_string());
    }
    if let Some(v) = start_time {
        qb.push(" AND b.add_time >= ").push_bind(v);
    }
    if let Some(v) = end_time {
        qb.push(" AND b.add_time <= ").push_bind(v);
    }
}

fn push_log_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    batch_id: Option<i64>,
    card_groups_id: Option<i32>,
    user_id: Option<i32>,
) {
    if let Some(v) = batch_id {
        qb.push(" AND l.batch_id = ").push_bind(v);
    }
    if let Some(v) = card_groups_id {
        qb.push(" AND l.card_groups_id = ").push_bind(v);
    }
    if let Some(v) = user_id {
        qb.push(" AND l.user_id = ").push_bind(v);
    }
}

fn push_my_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    group_type: Option<i32>,
    state: Option<&str>,
    now: i32,
) {
    if let Some(v) = group_type {
        qb.push(" AND g.group_type = ").push_bind(v);
    }
    // 有效口径与服务层派生状态一致：status = 0 且未过期
    match state {
        Some("valid") => {
            qb.push(" AND c.status = 0 AND (c.expired_time = 0 OR c.expired_time >= ")
                .push_bind(now)
                .push(")");
        }
        Some("invalid") => {
            qb.push(" AND (COALESCE(c.status, 0) <> 0 OR (c.expired_time <> 0 AND c.expired_time < ")
                .push_bind(now)
                .push("))");
        }
        _ => {}
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_need_math() {
        let stock = InsufficientStock {
            available: 2,
            need: 3,
        };
        assert!(stock.available < stock.need);
        let alloc = IssueAllocation {
            batch_id: 9,
            assignments: vec![(1, vec![10, 11]), (2, vec![12, 13])],
        };
        assert_eq!(alloc.assignments.len(), 2);
        assert_eq!(alloc.assignments[0].1.len(), 2);
    }

    #[test]
    fn sql_invariants_present() {
        let src = include_str!("gift_card_issue.rs");
        // 行锁与可分配口径
        assert!(src.contains("FOR UPDATE"));
        assert!(src.contains("claimed_user_id IS NULL AND user_id IS NULL"));
        // 唯一约束依赖：明细按 (batch_id, gift_cards_id) 写
        assert!(src.contains("INSERT INTO gift_card_issue_log (batch_id, card_groups_id, gift_cards_id, user_id, issue_time)"));
        // 归属校验在详情 SQL 内
        assert!(src.contains("(c.claimed_user_id = $2 OR c.user_id = $2)"));
    }

    /// 返还防重不变量（2026-11-21 子单退款精确化 re-pin）：旧口径
    /// 为「该订单已有任一 type=2 流水即整单跳过」的粗幂等，与
    /// 子单归属/分次返还不兼容，已升级为逐单位钳制——归属解析按
    /// parent_order_id 落子单份额、逐单位以 type=2 合计 ≤ 份额
    /// 为界、逐卡以归属族口径钳制原额。断言随新口径重钉，防重
    /// 强度不降（总额守恒由逐单位 + 逐卡双钳制保证）。
    #[test]
    fn return_hook_idempotency_invariants() {
        let src = include_str!("gift_card_issue.rs");
        let f = src
            .find("async fn order_attribution_row")
            .expect("归属解析应存在");
        let end = src[f..]
            .find("pub async fn find_exchange_order_ids")
            .expect("下一函数应存在")
            + f;
        let body = &src[f..end];
        // 归属解析必须按父单关系落子单份额
        assert!(body.contains("parent_order_id"));
        // 已返还合计按归属订单 type=2 流水统计（逐单位钳制依据）
        assert!(body.contains("WHERE order_id = $1 AND type = 2"));
        // 返还对象是记账订单全部 type=1 抵扣流水
        assert!(body.contains("WHERE order_id = $1 AND type = 1"));
        // 返还写 type=2 正额流水并恢复卡可用状态
        assert!(body.contains("VALUES ($1, $2, $3, 2,"));
        assert!(body.contains("status = 0"));
        // 逐卡行锁防并发重复返还
        assert!(body.contains("FOR UPDATE"));
        // 按额返还入口必须存在（退款完成点精确返还）
        assert!(body.contains("pub async fn return_stored_cards_amount"));
        // 全额返还入口保留（取消/手工完成点口径）
        assert!(body.contains("pub async fn return_stored_cards_for_order"));
    }
}
