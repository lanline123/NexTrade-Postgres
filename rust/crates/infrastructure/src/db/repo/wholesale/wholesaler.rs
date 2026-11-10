//! 批发商身份域（wholesale::wholesaler）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.wholesale.WholesalerMapper`（含自定义 XML：
//!   `selectByUserIdIncludeDeleted` / `reviveById` / `reviveByUserIds` /
//!   `updateGradeByUserIds` / `softDeleteByGrade` / `softDeleteByIds` /
//!   `selectByGradeForRemove` / `selectByIdsForRemove`）
//! - `com.nextrade.service.wholesale.impl.WholesalerServiceImpl`（657 行）
//! - `com.nextrade.mapper.user.UserMapper`（列表回填 / 导出来源）
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl#getConfigVal`
//!   （默认批发等级）
//!
//! # 表 / 列（权威来源）
//!
//! - `wholesaler`（10 列，主键 `wholesaler_id`）：
//!   `wholesaler_id` / `user_id` / `grade_id` / `status` / `source` /
//!   `shop_id` / `remark` / `is_delete` / `create_time` / `update_time`
//! - `wholesaler_grade`（8 列）：只读 `grade_id` / `grade_name` / `status`
//! - `user`（本域只读常用列）：`user_id` / `username` / `mobile` / `email` /
//!   `nickname` / `avatar` / `rank_id` / `from_tag` / `status` /
//!   `is_company_auth`
//! - `user_rank`：只读 `rank_id` / `rank_name`
//! - `config`：只读 `wholesaleDefaultGradeId`
//!
//! # 软删语义（重要）
//!
//! Java `Wholesaler` 的 `is_delete` 带 `@TableLogic`：普通查询/`updateById`
//! 自动追加 `is_delete = 0`，`deleteById` 变成 `UPDATE is_delete = 1`。
//! 而「添加批发商 / 批量导入」必须**读到软删行**（`uk_user_id` 唯一键不含
//! `is_delete`，重复 INSERT 会撞唯一键），因此 Java 写了
//! `selectByUserIdIncludeDeleted` 等原生 SQL。本仓储对应用法逐条复刻。
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。

use sqlx::{FromRow,PgPool};

use nextrade_core::entities::Wholesaler;
use nextrade_core::models::wholesale::wholesaler::MSG_WHOLESALER_NOT_FOUND;
use nextrade_core::{Error, Result};

/// `wholesaler` 的固定 SELECT 列清单（10 列，顺序与实体一致）。
pub const WHOLESALER_COLUMNS: &str = "wholesaler_id, user_id, grade_id, status, source, shop_id, remark, is_delete, create_time, update_time";

/// `user` 表本域读取的列（会员列表回填用）。
pub const USER_BRIEF_COLUMNS: &str =
    "user_id, username, mobile, email, nickname, avatar, rank_id, from_tag, status, is_company_auth";

/// 会员摘要行（`user` 表子集，供 `WholesalerVO` / `WholesalerSelectableUserVO` 回填）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct UserBriefRow {
    /// 会员 ID
    pub user_id: i32,
    /// 用户名
    pub username: String,
    /// 手机号
    pub mobile: String,
    /// 邮箱
    pub email: String,
    /// 昵称
    pub nickname: Option<String>,
    /// 头像
    pub avatar: String,
    /// 会员等级 ID
    pub rank_id: i32,
    /// 注册渠道
    pub from_tag: i32,
    /// 会员状态
    pub status: Option<i32>,
    /// 企业认证：0 否 1 是
    pub is_company_auth: i32,
}

/// 批发商仓储（持有连接池）。
#[derive(Clone)]
pub struct WholesalerRepository {
    pool: PgPool,
}

impl WholesalerRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 批发商分页 —————————————————————

    /// 批发商分页（Java `WholesalerServiceImpl#page`）。
    ///
    /// `keyword` / `fromTag` 先在 `user` 表解析出 `userId` 子查询再 `IN` 限定
    /// （Java 先查 userIds 再 IN，本实现用等价子查询，total 口径一致）。
    /// 固定 `is_delete = 0`，按 `wholesaler_id DESC` 排序。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        from_tag: Option<i32>,
        grade_id: Option<i32>,
        status: Option<i32>,
        source: Option<i32>,
        create_start: Option<i64>,
        create_end: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<Wholesaler>)> {
        let keyword = keyword.map(str::trim).filter(|s| !s.is_empty());
        let like = keyword.map(|k| format!("%{k}%"));

        let mut where_parts: Vec<String> = vec!["is_delete = 0".into()];
        if like.is_some() || from_tag.is_some() {
            let mut sub = vec!["1 = 1".to_string()];
            if like.is_some() {
                sub.push(
                    "(username ILIKE ? OR mobile ILIKE ? OR email ILIKE ? OR nickname ILIKE ?)".into(),
                );
            }
            if from_tag.is_some() {
                sub.push("from_tag = ?".into());
            }
            where_parts.push(format!("user_id IN (SELECT user_id FROM \"user\" WHERE {})", sub.join(" AND ")));
        }
        if grade_id.is_some() {
            where_parts.push("grade_id = ?".into());
        }
        if status.is_some() {
            where_parts.push("status = ?".into());
        }
        if source.is_some() {
            where_parts.push("source = ?".into());
        }
        if create_start.is_some() {
            where_parts.push("create_time >= ?".into());
        }
        if create_end.is_some() {
            where_parts.push("create_time <= ?".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let (limit, offset) = (limit.max(1), offset);

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM wholesaler {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_page_filters(cq, like.as_deref(), from_tag, grade_id, status, source, create_start, create_end);
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {WHOLESALER_COLUMNS} FROM wholesaler {where_sql} ORDER BY wholesaler_id DESC LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, Wholesaler>(&list_sql);
        lq = bind_page_filters(lq, like.as_deref(), from_tag, grade_id, status, source, create_start, create_end);
        let rows = lq
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 批发商查询（含 / 不含软删） —————————————————————

    /// 按主键查询未删除批发商（Java `selectById` + `@TableLogic`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_id(&self, wholesaler_id: i32) -> Result<Option<Wholesaler>> {
        let sql = format!(
            "SELECT {WHOLESALER_COLUMNS} FROM wholesaler WHERE wholesaler_id = $1 AND is_delete = 0 LIMIT 1"
        );
        sqlx::query_as::<_, Wholesaler>(&sql)
            .bind(wholesaler_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 `user_id` 查询批发商**含软删行**（Java
    /// `selectByUserIdIncludeDeleted`；添加批发商判重 / 复活用）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_user_id_include_deleted(&self, user_id: i32) -> Result<Option<Wholesaler>> {
        let sql = format!(
            "SELECT {WHOLESALER_COLUMNS} FROM wholesaler WHERE user_id = $1 LIMIT 1"
        );
        sqlx::query_as::<_, Wholesaler>(&sql)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按 `user_id` 查询批发商**含软删行**（Java
    /// `selectByUserIdsIncludeDeleted`；导入三态判定用）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_user_ids_include_deleted(&self, user_ids: &[i32]) -> Result<Vec<Wholesaler>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {WHOLESALER_COLUMNS} FROM wholesaler WHERE user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, Wholesaler>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按等级 + 可选 ID 列表查询未删除批发商（Java `selectByGradeForRemove`，
    /// 跨等级误删保护）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_grade_for_remove(
        &self,
        grade_id: i32,
        wholesaler_ids: Option<&[i32]>,
    ) -> Result<Vec<Wholesaler>> {
        match wholesaler_ids {
            None => {
                let sql = format!(
                    "SELECT {WHOLESALER_COLUMNS} FROM wholesaler WHERE grade_id = $1 AND is_delete = 0"
                );
                sqlx::query_as::<_, Wholesaler>(&sql)
                    .bind(grade_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(map_err)
            }
            Some(ids) => {
                if ids.is_empty() {
                    return Ok(Vec::new());
                }
                let placeholders = vec!["?"; ids.len()].join(", ");
                let sql = crate::db::to_pg_placeholders(&format!(
                    "SELECT {WHOLESALER_COLUMNS} FROM wholesaler WHERE grade_id = $1 AND is_delete = 0 AND wholesaler_id IN ({placeholders})"
                ));
                let mut q = sqlx::query_as::<_, Wholesaler>(&sql).bind(grade_id);
                for id in ids {
                    q = q.bind(*id);
                }
                q.fetch_all(&self.pool).await.map_err(map_err)
            }
        }
    }

    /// 按 ID 列表查询未删除批发商（Java `selectByIdsForRemove`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_ids_for_remove(&self, wholesaler_ids: &[i32]) -> Result<Vec<Wholesaler>> {
        if wholesaler_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; wholesaler_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {WHOLESALER_COLUMNS} FROM wholesaler WHERE is_delete = 0 AND wholesaler_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, Wholesaler>(&sql);
        for id in wholesaler_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 该 `user_id` 是否已是未删除批发商（Java 导入判重）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn exists_active_by_user_id(&self, user_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT wholesaler_id FROM wholesaler WHERE user_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    // ————————————————————— 批发商写操作 —————————————————————

    /// 新增批发商，返回自增主键（Java `insert`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert(&self, w: &Wholesaler) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO wholesaler (user_id, grade_id, status, source, shop_id, remark, is_delete, create_time, update_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING wholesaler_id",
        )
        .bind(w.user_id)
        .bind(w.grade_id)
        .bind(w.status)
        .bind(w.source)
        .bind(w.shop_id)
        .bind(&w.remark)
        .bind(w.is_delete)
        .bind(w.create_time)
        .bind(w.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 复活软删行（Java `reviveById`：`is_delete=0, status=1, grade=?, source=?, remark=?, update_time=?`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn revive_by_id(
        &self,
        wholesaler_id: i32,
        grade_id: i32,
        source: i32,
        remark: &str,
        now: i64,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE wholesaler SET is_delete = 0, status = 1, grade_id = $1, source = $2, remark = $3, update_time = $4 WHERE wholesaler_id = $5",
        )
        .bind(grade_id)
        .bind(source)
        .bind(remark)
        .bind(now)
        .bind(wholesaler_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量复活软删行（Java `reviveByUserIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn revive_by_user_ids(
        &self,
        user_ids: &[i32],
        grade_id: i32,
        source: i32,
        now: i64,
    ) -> Result<u64> {
        if user_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE wholesaler SET is_delete = 0, status = 1, grade_id = $1, source = $2, update_time = $3 WHERE is_delete = 1 AND user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql).bind(grade_id).bind(source).bind(now);
        for id in user_ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按 `user_id` 批量覆盖等级（Java `updateGradeByUserIds`，导入
    /// `OVERWRITE_GRADE` 策略）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update_grade_by_user_ids(
        &self,
        user_ids: &[i32],
        grade_id: i32,
        now: i64,
    ) -> Result<u64> {
        if user_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE wholesaler SET grade_id = $1, update_time = $2 WHERE is_delete = 0 AND user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql).bind(grade_id).bind(now);
        for id in user_ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 启停（Java `updateById` 的 status 分支）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn set_status(&self, wholesaler_id: i32, status: i32, now: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE wholesaler SET status = $1, update_time = $2 WHERE wholesaler_id = $3 AND is_delete = 0",
        )
        .bind(status)
        .bind(now)
        .bind(wholesaler_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 改等级（Java `updateById` 的 grade 分支）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn set_grade(&self, wholesaler_id: i32, grade_id: i32, now: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE wholesaler SET grade_id = $1, update_time = $2 WHERE wholesaler_id = $3 AND is_delete = 0",
        )
        .bind(grade_id)
        .bind(now)
        .bind(wholesaler_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 软删（Java `softDeleteByGrade`：`is_delete=1, update_time=?`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn soft_delete_by_grade(
        &self,
        grade_id: i32,
        wholesaler_ids: Option<&[i32]>,
        now: i64,
    ) -> Result<u64> {
        match wholesaler_ids {
            None => {
                let res = sqlx::query(
                    "UPDATE wholesaler SET is_delete = 1, update_time = $1 WHERE grade_id = $2 AND is_delete = 0",
                )
                .bind(now)
                .bind(grade_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
                Ok(res.rows_affected())
            }
            Some(ids) => {
                if ids.is_empty() {
                    return Ok(0);
                }
                let placeholders = vec!["?"; ids.len()].join(", ");
                let sql = crate::db::to_pg_placeholders(&format!(
                    "UPDATE wholesaler SET is_delete = 1, update_time = $1 WHERE grade_id = $2 AND is_delete = 0 AND wholesaler_id IN ({placeholders})"
                ));
                let mut q = sqlx::query(&sql).bind(now).bind(grade_id);
                for id in ids {
                    q = q.bind(*id);
                }
                let res = q.execute(&self.pool).await.map_err(map_err)?;
                Ok(res.rows_affected())
            }
        }
    }

    /// 软删（Java `softDeleteByIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn soft_delete_by_ids(&self, wholesaler_ids: &[i32], now: i64) -> Result<u64> {
        if wholesaler_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; wholesaler_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE wholesaler SET is_delete = 1, update_time = $1 WHERE is_delete = 0 AND wholesaler_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql).bind(now);
        for id in wholesaler_ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 会员 / 等级 / 配置 —————————————————————

    /// 会员是否存在（Java `userService.getById(...) != null`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn user_exists(&self, user_id: i32) -> Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"user\" WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 批量读取会员摘要（Java `userService.listByIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_users_by_ids(&self, user_ids: &[i32]) -> Result<Vec<UserBriefRow>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {USER_BRIEF_COLUMNS} FROM \"user\" WHERE user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, UserBriefRow>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量读取会员等级名称（Java `userRankService.listByIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_ranks_by_ids(&self, rank_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if rank_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; rank_ids.len()].join(", ");
        let sql =
            crate::db::to_pg_placeholders(&format!("SELECT rank_id, rank_name FROM user_rank WHERE rank_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in rank_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量读取批发等级名称（Java `gradeService.mapByIds`，`is_delete=0`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_grade_names(&self, grade_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if grade_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; grade_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT grade_id, grade_name FROM wholesaler_grade WHERE is_delete = 0 AND grade_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in grade_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 等级有效性（Java `WholesalerGradeService#requireEnabled` 的只读版）。
    ///
    /// 返回 `Some(true)` 存在且启用、`Some(false)` 存在但禁用、`None` 不存在。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn grade_enabled(&self, grade_id: i32) -> Result<Option<bool>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT status FROM wholesaler_grade WHERE grade_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(grade_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(status,)| status == 1))
    }

    /// 默认批发等级 ID（Java `WholesaleSettingService#getDefaultGradeId`）：
    /// 缺失 / 非法 → 0。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn default_grade_id(&self) -> Result<i32> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = 'wholesaleDefaultGradeId' AND is_del = 0 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row
            .and_then(|(v,)| v)
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0))
    }

    /// 全部启用会员 userId（Java `listAllActiveUserIds`：`user.status = 1`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list_active_user_ids(&self) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT user_id FROM \"user\" WHERE status = 1 ORDER BY user_id ASC")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 指定会员等级下启用中的会员 userId
    /// （Java `listActiveUserIdsByRanks`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list_active_user_ids_by_ranks(&self, rank_ids: &[i32]) -> Result<Vec<i32>> {
        if rank_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; rank_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT DISTINCT user_id FROM \"user\" WHERE status = 1 AND rank_id IN ({placeholders}) ORDER BY user_id ASC"
        ));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        for id in rank_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 分批校验 userId 在 `user` 表真实存在（Java `loadExistingUserIds`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn load_existing_user_ids(&self, user_ids: &[i32]) -> Result<Vec<i32>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT user_id FROM \"user\" WHERE user_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 「可选为批发商的会员」分页（Java `userService.list` 的子集口径）。
    ///
    /// 过滤：`keyword`（用户名/手机/邮箱/昵称 ILIKE）+ `fromTag` + `rankId`
    /// + `status`；按 `user_id DESC` 排序（与会员列表一致）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate_users(
        &self,
        keyword: Option<&str>,
        from_tag: Option<i32>,
        rank_id: Option<i32>,
        status: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<UserBriefRow>)> {
        let like = keyword
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));
        let mut where_parts: Vec<String> = vec!["1 = 1".into()];
        if like.is_some() {
            where_parts.push(
                "(username ILIKE ? OR mobile ILIKE ? OR email ILIKE ? OR nickname ILIKE ?)".into(),
            );
        }
        if from_tag.is_some() {
            where_parts.push("from_tag = ?".into());
        }
        if rank_id.is_some() {
            where_parts.push("rank_id = ?".into());
        }
        if status.is_some() {
            where_parts.push("status = ?".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"user\" {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_user_filters(cq, like.as_deref(), from_tag, rank_id, status);
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {USER_BRIEF_COLUMNS} FROM \"user\" {where_sql} ORDER BY user_id DESC LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, UserBriefRow>(&list_sql);
        lq = bind_user_filters(lq, like.as_deref(), from_tag, rank_id, status);
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }
}

/// 绑定批发商分页过滤参数（顺序与 `paginate` 拼接 WHERE 的顺序一致）。
#[allow(clippy::too_many_arguments)]
fn bind_page_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    like: Option<&'q str>,
    from_tag: Option<i32>,
    grade_id: Option<i32>,
    status: Option<i32>,
    source: Option<i32>,
    create_start: Option<i64>,
    create_end: Option<i64>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    if let Some(like) = like {
        q = q.bind(like).bind(like).bind(like).bind(like);
    }
    if let Some(v) = from_tag {
        q = q.bind(v);
    }
    if let Some(v) = grade_id {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    if let Some(v) = source {
        q = q.bind(v);
    }
    if let Some(v) = create_start {
        q = q.bind(v);
    }
    if let Some(v) = create_end {
        q = q.bind(v);
    }
    q
}

/// 绑定会员分页过滤参数。
fn bind_user_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    like: Option<&'q str>,
    from_tag: Option<i32>,
    rank_id: Option<i32>,
    status: Option<i32>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    if let Some(like) = like {
        q = q.bind(like).bind(like).bind(like).bind(like);
    }
    if let Some(v) = from_tag {
        q = q.bind(v);
    }
    if let Some(v) = rank_id {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    q
}

/// 数据库错误映射（与其它域仓储同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_WHOLESALER_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `WHOLESALER_COLUMNS` 覆盖 `wholesaler` 全部 10 列。
    #[test]
    fn wholesaler_columns_cover_all() {
        let listed: Vec<&str> = WHOLESALER_COLUMNS.split(", ").collect();
        assert_eq!(
            listed,
            vec![
                "wholesaler_id",
                "user_id",
                "grade_id",
                "status",
                "source",
                "shop_id",
                "remark",
                "is_delete",
                "create_time",
                "update_time",
            ]
        );
    }

    /// 会员摘要列与 `UserBriefRow` 字段一一对应。
    #[test]
    fn user_brief_columns_match_row_fields() {
        let listed: Vec<&str> = USER_BRIEF_COLUMNS.split(", ").collect();
        assert_eq!(
            listed,
            vec![
                "user_id",
                "username",
                "mobile",
                "email",
                "nickname",
                "avatar",
                "rank_id",
                "from_tag",
                "status",
                "is_company_auth",
            ]
        );
    }

    /// 错误文案与 Java `HttpStatus.NOT_FOUND` 分支一致。
    #[test]
    fn not_found_message_is_verbatim() {
        assert_eq!(MSG_WHOLESALER_NOT_FOUND, "批发商不存在");
    }
}
