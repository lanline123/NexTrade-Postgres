//! IM 客服会话域仓储（im::im_conversation）。
//!
//! 对应原 Java `ImConversationMapper`（含 `ImConversationMapper.xml#consultHistory`）
//! 以及服务层直接使用的 `UserMapper` / `ShopMapper` / `AdminUserMapper` /
//! `ConfigService` 只读查询。
//!
//! 表 / 列名严格对齐权威 schema（`python tools/schema_parser.py --table im_conversation`
//! / `--table im_message` / `--table user` / `--table shop` / `--table admin_user`
//! / `--table im_servant` / `--table config`）：
//! - `im_conversation`：`id` / `user_id` / `last_servant_id` / `add_time` /
//!   `shop_id` / `user_from` / `status` / `last_update_time` / `is_delete` /
//!   `remark` / `summary`
//! - `im_message`：`id` / `conversation_id` / `content` / `message_type` /
//!   `type` / `user_id` / `servant_id` / `send_time` / `status` / `extend` /
//!   `push_status` / `is_read` / `shop_id` / `user_from`
//! - `user`（只读）：`user_id` / `username` / `nickname` / `avatar` / `mobile`
//! - `shop`（只读）：`shop_id` / `shop_title` / `shop_logo`
//! - `admin_user`（只读）：`admin_id` / `username` / `mobile` / `avatar`
//! - `im_servant`（只读/更新接待数）：`servant_id` / `shop_id` / `conversation_num`
//! - `config`（只读）：`biz_code` / `biz_val` / `is_del`
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!`/`query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! # Java 语义保留说明
//!
//! - `eq(column, null)` 在 MyBatis-Plus 3.5.7 里生成 `column = ?` 且绑定
//!   `null`（SQL 三值逻辑 ⇒ 永不命中）。本仓储对可空过滤值直接
//!   `bind(Option<T>)`，行为一致（例如请求头店铺缺失时 `shop_id = NULL`
//!   ⇒ 查不到行，而不是「不过滤」）。
//! - `getConversationSearch` 对 `type` 列绑定了字符串 `"text"`（列是 tinyint，
//!   MySQL 隐式转 0）；本实现保留 `type = ?` + 绑定 `"text"` 的等价写法。
//! - `consultHistory` 的 mapper 只 `select ic.*`，因此
//!   `first_response_time` / `conversation_duration` / `average_response_time`
//!   恒为 `NULL`（VO 侧不填）。

use sqlx::{FromRow,Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{ImConversation, ImMessage};
use nextrade_core::models::im::im_conversation::{
    parse_timestamp_datetime, status_filter, ImConversationPageQuery, StatusFilter,
    WaitServantListQuery,
};
use nextrade_core::{Error, Result};

/// `im_conversation` 固定 SELECT 列清单（顺序与实体字段一致）。
const CONV_SELECT: &str = "SELECT id, user_id, last_servant_id, add_time, shop_id, user_from, status, last_update_time, is_delete, remark, summary FROM im_conversation";

/// `im_message` 固定 SELECT 列清单（顺序与实体字段一致；`type` 是关键字，用反引号）。
const MESSAGE_SELECT: &str = "SELECT id, conversation_id, content, message_type, \"type\", user_id, servant_id, send_time, status, extend, push_status, is_read, shop_id, user_from FROM im_message";

/// 用户简要信息（列表 / 搜索 VO 装配来源）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ConvUserBrief {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: String,
    pub mobile: String,
}

/// 店铺简要信息。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ConvShopBrief {
    pub shop_id: i32,
    pub shop_title: Option<String>,
    pub shop_logo: Option<String>,
}

/// 客服（管理员）简要信息（`consultHistory` 的 ServantVO）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct ConvAdminBrief {
    pub admin_id: i32,
    pub username: String,
    pub mobile: Option<String>,
    pub avatar: Option<String>,
}

/// IM 客服会话域仓储。
#[derive(Clone)]
pub struct ImConversationRepository {
    pool: PgPool,
}

impl ImConversationRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（服务层事务编排用）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 会话查询 —————————————————————

    /// 按主键查询（对应 Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<ImConversation>> {
        sqlx::query_as::<_, ImConversation>(&format!("{CONV_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按「店铺 + 用户 + 未删除」查一条会话（对应 `create` 的 `getOne`）。
    ///
    /// Java 的 `getOne` 命中多行会抛 `TooManyResultsException`；这里 `LIMIT 1`
    /// 取首行（`id` 升序），见 `uncertain_points`。
    pub async fn find_by_shop_user(
        &self,
        shop_id: Option<i32>,
        user_id: Option<i32>,
    ) -> Result<Option<ImConversation>> {
        sqlx::query_as::<_, ImConversation>(&format!(
            "{CONV_SELECT} WHERE shop_id = $1 AND user_id = $2 AND is_delete = 0 ORDER BY id ASC LIMIT 1"
        ))
        .bind(shop_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 会话列表分页（对应 Java `ImConversationServiceImpl#list` 的 `this.page`）。
    ///
    /// 过滤条件 = `is_delete = 0` + `buildQueryWrapper` 全部分支；
    /// `username` 分支先查 `user` 表，**用户列表为空时不追加过滤**（Java 原样）。
    pub async fn paginate_list(
        &self,
        q: &ImConversationPageQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ImConversation>)> {
        let user_ids = match q.username.as_deref() {
            Some(name) => self.find_user_ids_by_keyword(name).await?,
            None => Vec::new(),
        };
        let username_filter: Option<&[i32]> =
            if q.username.is_some() && !user_ids.is_empty() { Some(&user_ids) } else { None };

        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM im_conversation WHERE 1 = 1");
        Self::push_list_filters(&mut cq, q, username_filter);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CONV_SELECT} WHERE 1 = 1"));
        Self::push_list_filters(&mut rq, q, username_filter);
        rq.push(q.list_order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ImConversation> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// `buildQueryWrapper` 的 WHERE 片段（count 与取行共用）。
    fn push_list_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &ImConversationPageQuery,
        username_user_ids: Option<&[i32]>,
    ) {
        qb.push(" AND is_delete = 0");
        if let Some(shop_id) = q.shop_id.filter(|v| *v >= 0) {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(user_id) = q.user_id.filter(|v| *v > 0) {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        match status_filter(q.status) {
            StatusFilter::None => {}
            StatusFilter::In(list) => {
                debug_assert_eq!(list, vec![1, 2]);
                qb.push(" AND status IN (1, 2)");
            }
            StatusFilter::Eq(v) => {
                qb.push(" AND status = ");
                qb.push_bind(v);
            }
        }
        if let Some(servant_id) = q.last_servant_id.filter(|v| *v != -1) {
            qb.push(" AND last_servant_id = ");
            qb.push_bind(servant_id);
        }
        if let Some(user_from) = q.user_from.as_deref() {
            qb.push(" AND user_from = ");
            qb.push_bind(user_from.to_string());
        }
        if let Some(is_delete) = q.is_delete {
            qb.push(" AND is_delete = ");
            qb.push_bind(is_delete);
        }
        let start = q
            .start_time
            .as_deref()
            .and_then(parse_timestamp_datetime);
        let end = q.end_time.as_deref().and_then(parse_timestamp_datetime);
        if let (Some(start), Some(end)) = (start, end) {
            qb.push(" AND add_time BETWEEN ");
            qb.push_bind(start);
            qb.push(" AND ");
            qb.push_bind(end);
        }
        if let Some(remark) = q.remark.as_deref() {
            qb.push(" AND remark ILIKE ");
            qb.push_bind(format!("%{remark}%"));
        }
        if let Some(ids) = username_user_ids {
            qb.push(" AND user_id IN (");
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
        }
    }

    /// 待接入客户分页（对应 Java `ImConversationServiceImpl#waitServantList`）。
    pub async fn paginate_wait_servant(
        &self,
        q: &WaitServantListQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ImConversation>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM im_conversation WHERE 1 = 1");
        Self::push_wait_filters(&mut cq, q);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CONV_SELECT} WHERE 1 = 1"));
        Self::push_wait_filters(&mut rq, q);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ImConversation> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_wait_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &WaitServantListQuery) {
        qb.push(" AND status = 0 AND last_servant_id = 0 AND is_delete = 0");
        // Java `eq(shopId, dto.getShopId())`：null 时 `shop_id = NULL` ⇒ 无行
        qb.push(" AND shop_id = ");
        qb.push_bind(q.shop_id);
        if let Some(user_from) = q
            .user_from
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            qb.push(" AND user_from = ");
            qb.push_bind(user_from.to_string());
        }
    }

    /// 历史会话分页（对应 `ImConversationMapper.xml#consultHistory`）。
    pub async fn paginate_consult_history(
        &self,
        q: &ImConversationPageQuery,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<ImConversation>)> {
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM im_conversation WHERE 1 = 1");
        Self::push_consult_filters(&mut cq, q);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| db_err(e))?;

        let mut rq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CONV_SELECT} WHERE 1 = 1"));
        Self::push_consult_filters(&mut rq, q);
        rq.push(q.order_clause());
        rq.push(format!(" LIMIT {limit} OFFSET {offset}"));
        let rows: Vec<ImConversation> =
            rq.build_query_as().fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// `consultHistory` 的 `<where>` 片段（`count` 与取行共用）。
    fn push_consult_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &ImConversationPageQuery) {
        qb.push(" AND is_delete = 0 AND shop_id = ");
        qb.push_bind(q.shop_id);
        match q.time_type {
            Some(1) => Self::push_time_range(qb, "add_time", q.search_start_time, q.search_end_time),
            Some(3) => Self::push_time_range(
                qb,
                "last_update_time",
                q.search_start_time,
                q.search_end_time,
            ),
            _ => {}
        }
        if let Some(username) = q.username.as_deref().filter(|s| !s.is_empty()) {
            qb.push(
                " AND EXISTS (SELECT 1 FROM \"user\" u WHERE u.user_id = im_conversation.user_id AND u.username ILIKE ",
            );
            qb.push_bind(format!("%{username}%"));
            qb.push(")");
        }
        if let Some(servant_id) = q.last_servant_id.filter(|v| *v != -1) {
            qb.push(" AND last_servant_id = ");
            qb.push_bind(servant_id);
        }
        if let Some(user_from) = q.user_from.as_deref() {
            qb.push(" AND user_from = ");
            qb.push_bind(user_from.to_string());
        }
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(remark) = q.remark.as_deref() {
            qb.push(" AND remark ILIKE ");
            qb.push_bind(format!("%{remark}%"));
        }
    }

    fn push_time_range(
        qb: &mut QueryBuilder<'_, Postgres>,
        column: &'static str,
        start: Option<i64>,
        end: Option<i64>,
    ) {
        match (start, end) {
            (Some(s), Some(e)) => {
                qb.push(format!(" AND {column} BETWEEN "));
                qb.push_bind(s);
                qb.push(" AND ");
                qb.push_bind(e);
            }
            (Some(s), None) => {
                qb.push(format!(" AND {column} >= "));
                qb.push_bind(s);
            }
            (None, Some(e)) => {
                qb.push(format!(" AND {column} <= "));
                qb.push_bind(e);
            }
            (None, None) => {}
        }
    }

    // ————————————————————— 会话写入 —————————————————————

    /// 更新状态 / 最后客服（对应 Java `updateById(imConversation)` 的核心列）。
    ///
    /// `last_servant_id` 为 `None` ⇒ 不写该列（MyBatis-Plus NOT_NULL 策略）。
    pub async fn update_status_servant(
        &self,
        id: i32,
        status: i32,
        last_servant_id: Option<i32>,
    ) -> Result<u64> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE im_conversation SET status = ");
        qb.push_bind(status);
        if let Some(servant_id) = last_servant_id {
            qb.push(", last_servant_id = ");
            qb.push_bind(servant_id);
        }
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 保存备注 / 总结（`None` ⇒ 不写该列，Java NOT_NULL 策略）。
    pub async fn update_remark_summary(
        &self,
        id: i32,
        remark: Option<&str>,
        summary: Option<&str>,
    ) -> Result<u64> {
        if remark.is_none() && summary.is_none() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE im_conversation SET ");
        let mut sep = qb.separated(", ");
        if let Some(remark) = remark {
            sep.push("remark = ");
            sep.push_bind(remark.to_string());
        }
        if let Some(summary) = summary {
            sep.push("summary = ");
            sep.push_bind(summary.to_string());
        }
        drop(sep);
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 逻辑删除会话（`is_delete = 1`）。
    pub async fn soft_delete(&self, id: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE im_conversation SET is_delete = 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 单字段更新（列名由服务层白名单归一后传入）。
    pub async fn update_field(&self, id: i32, column: &str, val: &str) -> Result<u64> {
        debug_assert!(
            nextrade_core::models::im::im_conversation::GENERAL_FIELDS.contains(&column),
            "update_field 列名未经过白名单映射: {column}"
        );
        let sql = format!("UPDATE im_conversation SET {column} = $1 WHERE id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量物理删除（对应 Java `removeByIds`）。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("DELETE FROM im_conversation WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 某客服进行中的会话数（`transfer` 更新接待量用）。
    pub async fn count_active_by_servant(&self, servant_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM im_conversation WHERE last_servant_id = $1 AND status = 1",
        )
        .bind(servant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 更新客服接待量（`UPDATE im_servant ... LIMIT 1`）。
    pub async fn update_servant_conversation_num(
        &self,
        servant_id: i32,
        shop_id: Option<i32>,
        num: i64,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE im_servant SET conversation_num = $1 WHERE servant_id = (SELECT servant_id FROM im_servant WHERE servant_id = $2 AND shop_id = $3 LIMIT 1)",
        )
        .bind(num)
        .bind(servant_id)
        .bind(shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— 消息查询 —————————————————————

    /// 会话消息（对应 `getListByConversationIds`）。
    ///
    /// Java 无 `ORDER BY`（依赖 MySQL 默认主键序）；这里显式 `ORDER BY id ASC`
    /// 保证可测（与默认行为一致）。
    pub async fn list_messages_by_conversation_ids(
        &self,
        ids: &[i32],
    ) -> Result<Vec<ImMessage>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "{MESSAGE_SELECT} WHERE conversation_id IN ("
        ));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" ORDER BY id ASC");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 会话最近两条消息（对应 `getListByConversationIdLimitTwo`）。
    pub async fn list_messages_limit_two(&self, conversation_id: i32) -> Result<Vec<ImMessage>> {
        sqlx::query_as::<_, ImMessage>(&format!(
            "{MESSAGE_SELECT} WHERE conversation_id = $1 ORDER BY id DESC LIMIT 2"
        ))
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 未读数（对应 `getUnreadMessageCount(conversationId, type)`）。
    pub async fn count_unread(&self, conversation_id: i32, msg_type: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM im_message WHERE conversation_id = $1 AND is_read = 0 AND \"type\" = $2",
        )
        .bind(conversation_id)
        .bind(msg_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 首次用户消息（Java `consultHistory` 的 `status=1 order by send_time asc limit 1`）。
    ///
    /// ⚠️ Java 的 wrapper 是「`conversation_id IN (全部) ... limit 1`」——
    /// **全局只取一条**，因此只有第一个会话能拿到 `firstUserMessage`。
    /// 本实现如实复刻（含该缺陷），见 `uncertain_points`。
    pub async fn find_first_status_message(
        &self,
        conversation_ids: &[i32],
    ) -> Result<Option<ImMessage>> {
        if conversation_ids.is_empty() {
            return Ok(None);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{MESSAGE_SELECT} WHERE conversation_id IN ("));
        let mut sep = qb.separated(", ");
        for id in conversation_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" AND status = 1 ORDER BY send_time ASC LIMIT 1");
        qb.build_query_as()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 某会话首条客服消息（`type=2 AND status=1 ORDER BY send_time ASC LIMIT 1`）。
    pub async fn find_first_servant_message(
        &self,
        conversation_id: i32,
    ) -> Result<Option<ImMessage>> {
        sqlx::query_as::<_, ImMessage>(&format!(
            "{MESSAGE_SELECT} WHERE conversation_id = $1 AND \"type\" = 2 AND status = 1 ORDER BY send_time ASC LIMIT 1"
        ))
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 客服消息数（`type=2 AND status=1`）。
    pub async fn count_servant_messages(&self, conversation_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM im_message WHERE conversation_id = $1 AND \"type\" = 2 AND status = 1",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 客服消息的 `MAX(send_time)` / `MIN(send_time)`（`type=2 AND status=1`）。
    pub async fn servant_message_times(
        &self,
        conversation_id: i32,
    ) -> Result<(Option<i32>, Option<i32>)> {
        let row: (Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT MAX(send_time), MIN(send_time) FROM im_message WHERE conversation_id = $1 AND \"type\" = 2 AND status = 1",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 把会话下未读消息标记已读（`delete` 流程）。
    pub async fn mark_messages_read(&self, conversation_id: i32) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE im_message SET is_read = 1 WHERE conversation_id = $1 AND is_read = 0",
        )
        .bind(conversation_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按内容搜文本消息（后台 `getConversationSearch`）。
    ///
    /// Java 对 tinyint 列 `type` 绑定字符串 `"text"`（MySQL 隐式转 0），
    /// 本实现保留同样绑定，见模块头说明。
    pub async fn find_text_messages_by_content(&self, keyword: &str) -> Result<Vec<ImMessage>> {
        sqlx::query_as::<_, ImMessage>(&format!(
            "{MESSAGE_SELECT} WHERE content ILIKE $1 AND \"type\" = $2 ORDER BY id ASC"
        ))
        .bind(format!("%{keyword}%"))
        .bind("text")
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按内容搜用户文本消息（前台 `frontSearch`）：
    /// `user_id = ? AND content ILIKE ? AND status = 1 AND message_type = 'text'`。
    pub async fn find_text_messages_by_user_content(
        &self,
        user_id: i32,
        keyword: &str,
    ) -> Result<Vec<ImMessage>> {
        sqlx::query_as::<_, ImMessage>(&format!(
            "{MESSAGE_SELECT} WHERE user_id = $1 AND content ILIKE $2 AND status = 1 AND message_type = 'text' ORDER BY id ASC"
        ))
        .bind(user_id)
        .bind(format!("%{keyword}%"))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 用户聊过天的店铺 ID（`frontSearch` 第 1 步：`group by shop_id`）。
    pub async fn find_user_chatted_shop_ids(&self, user_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT shop_id FROM im_message WHERE user_id = $1 GROUP BY shop_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().filter_map(|(id,)| id).collect())
    }

    // ————————————————————— 会话按条件批量 —————————————————————

    /// 后台用户搜索：`user_id IN (...) AND last_servant_id = ? GROUP BY user_id`。
    ///
    /// `servant_id` 为 `None` ⇒ 绑定 `NULL` ⇒ MySQL `= NULL` 恒不命中
    /// （与 Java public 控制器 `getCurrentAdminId() == null` 时一致）。
    pub async fn find_conversations_by_users_and_servant(
        &self,
        user_ids: &[i32],
        servant_id: Option<i32>,
    ) -> Result<Vec<ImConversation>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CONV_SELECT} WHERE user_id IN ("));
        let mut sep = qb.separated(", ");
        for id in user_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" AND last_servant_id = ");
        qb.push_bind(servant_id);
        qb.push(" GROUP BY user_id");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 后台会话搜索：`id IN (...) AND last_servant_id = ? AND status = 1`。
    pub async fn find_conversations_by_ids_and_servant(
        &self,
        ids: &[i32],
        servant_id: Option<i32>,
    ) -> Result<Vec<ImConversation>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CONV_SELECT} WHERE id IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" AND last_servant_id = ");
        qb.push_bind(servant_id);
        qb.push(" AND status = 1");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 前台会话搜索：`id IN (...) AND user_id = ? AND is_delete = 0`。
    pub async fn find_conversations_by_ids_and_user(
        &self,
        ids: &[i32],
        user_id: i32,
    ) -> Result<Vec<ImConversation>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{CONV_SELECT} WHERE id IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" AND user_id = ");
        qb.push_bind(user_id);
        qb.push(" AND is_delete = 0");
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 用户 / 店铺 / 客服 / 配置 —————————————————————

    /// 用户 ID 列表（`username` / `mobile` / `nickname` 模糊）。
    pub async fn find_user_ids_by_keyword(&self, keyword: &str) -> Result<Vec<i32>> {
        let like = format!("%{keyword}%");
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT user_id FROM \"user\" WHERE username ILIKE $1 OR mobile ILIKE $2 OR nickname ILIKE $3",
        )
        .bind(&like)
        .bind(&like)
        .bind(&like)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 用户简要信息（`selectBatchIds`）。
    pub async fn find_users_by_ids(&self, ids: &[i32]) -> Result<Vec<ConvUserBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT user_id, username, nickname, avatar, mobile FROM \"user\" WHERE user_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 前台搜索用户（`username ILIKE`，返回 user_id + 展示字段）。
    pub async fn find_users_by_username_like(&self, keyword: &str) -> Result<Vec<ConvUserBrief>> {
        sqlx::query_as::<_, ConvUserBrief>(
            "SELECT user_id, username, nickname, avatar, mobile FROM \"user\" WHERE username ILIKE $1",
        )
        .bind(format!("%{keyword}%"))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 店铺简要信息（`selectBatchIds`）。
    pub async fn find_shops_by_ids(&self, ids: &[i32]) -> Result<Vec<ConvShopBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT shop_id, shop_title, shop_logo FROM shop WHERE shop_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 在给定店铺中按名称模糊搜索（`frontSearch` 第 1.2 步）。
    pub async fn find_shops_by_ids_and_title_like(
        &self,
        ids: &[i32],
        keyword: &str,
    ) -> Result<Vec<ConvShopBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT shop_id, shop_title, shop_logo FROM shop WHERE shop_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.push(" AND shop_title ILIKE ");
        qb.push_bind(format!("%{keyword}%"));
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 客服（管理员）简要信息（`consultHistory` 的 ServantVO）。
    pub async fn find_admins_by_ids(&self, ids: &[i32]) -> Result<Vec<ConvAdminBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(
            "SELECT admin_id, username, mobile, avatar FROM admin_user WHERE admin_id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as().fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 读取 `config.biz_val`（`biz_code` 维度，`is_del = 0`）。
    ///
    /// 对应 Java `ConfigServiceImpl#getConfigByCode` / `getConfigVal`；
    /// 缺行 ⇒ `None`（Java 会抛 `CONFIG_NOT_EXIST`，本实现容错，见
    /// `uncertain_points`）。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }
}

/// 把 `sqlx` 错误转成项目错误（查询不存在不算 RowNotFound 语义）。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// `consultHistory` 查询的 `db_err` 别名（与普通查询一致，便于阅读）。
fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::im::im_conversation::CONVERSATION_COLUMNS;

    #[test]
    fn select_lists_cover_schema_columns() {
        let selected = CONV_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, CONVERSATION_COLUMNS.join(", "));
        assert_eq!(CONVERSATION_COLUMNS.len(), 11);
    }

    #[test]
    fn message_select_has_fourteen_columns() {
        let selected = MESSAGE_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected.split(", ").count(), 14);
        assert!(MESSAGE_SELECT.ends_with("FROM im_message"));
    }

    #[test]
    fn brief_selects_read_only_needed_columns() {
        // 静态守卫：这些 SQL 只读少量列，避免 SELECT * 拖慢列表
        let src = include_str!("im_conversation.rs");
        assert!(src.contains(
            r#"SELECT user_id, username, nickname, avatar, mobile FROM \"user\" WHERE user_id IN ("#
        ));
        assert!(src.contains("SELECT shop_id, shop_title, shop_logo FROM shop WHERE shop_id IN ("));
        assert!(src.contains(
            "SELECT admin_id, username, mobile, avatar FROM admin_user WHERE admin_id IN ("
        ));
    }
}
