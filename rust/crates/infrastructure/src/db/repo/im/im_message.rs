//! IM 消息（ImMessageController） 域仓储（im::im_message）。
//!
//! 表 / 列名严格对齐权威 schema `java/database/pure.sql`
//! （`python tools/schema_parser.py --table im_message` 等可复核）：
//! - `im_message`：`id` / `conversation_id` / `content` / `message_type` /
//!   `type` / `user_id` / `servant_id` / `send_time` / `status` / `extend` /
//!   `push_status` / `is_read` / `shop_id` / `user_from`
//! - `im_conversation`：`id` / `user_id` / `last_servant_id` / `add_time` /
//!   `shop_id` / `user_from` / `status` / `last_update_time` / `is_delete` /
//!   `remark` / `summary`
//! - `im_servant`：`id` / `servant_id` / `status` / `shop_id` /
//!   `conversation_num` / `add_time` / `last_update_time`
//! - `shop_kefu_config`：`shop_id` / `auto_access_mode` / `return_customer_access` / ...
//! - `user`：`user_id` / `username` / `nickname` / `avatar`
//! - `admin_user`：`admin_id` / `username` / `avatar`
//! - `config`：`biz_code` / `biz_val` / `is_del`
//!
//! 查询统一使用运行时 `sqlx::query_as::<_, T>(...)` / `sqlx::query(...)`，
//! **不使用 `query!` 宏**（本机无 `DATABASE_URL`）。每条 SQL 写成**单行**字面量，
//! 动态 WHERE 片段只拼接固定字面量，条件值一律 `?` 占位绑定。

use sqlx::{FromRow,PgPool, Row};
use nextrade_core::entities::{ImConversation, ImMessage, ImServant, ShopKefuConfig};
use nextrade_core::{Error, Result};

/// `im_message` 全列（顺序与 [`ImMessage`] 实体一致，供 `query_as` 映射）。
const IM_MESSAGE_COLUMNS: &str = "id, conversation_id, content, message_type, \"type\", user_id, servant_id, send_time, status, extend, push_status, is_read, shop_id, user_from";

/// 列表 / 发送 VO 需要的消息行（复用实体）。
pub type MessageRow = ImMessage;

/// 用户简信息（Java `ImMessageListVO.UserVO` 装配来源）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct UserBrief {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar: String,
}

/// 客服（管理员）简信息（Java `ImMessageListVO.ServantVO` 装配来源）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct AdminBrief {
    pub admin_id: i32,
    pub username: String,
    pub avatar: Option<String>,
}

/// 列表查询过滤条件（对应 Java `buildQueryWrapper` 的入参集合）。
#[derive(Debug, Clone, Default)]
pub struct MessageListFilter {
    pub conversation_id: Option<i32>,
    pub shop_id: Option<i32>,
    pub user_id: Option<i32>,
    pub first_id: Option<i32>,
    pub servant_id: Option<i32>,
    pub msg_type: Option<i32>,
    pub is_read: Option<i32>,
}

/// 落库新消息（发送流程）。
#[derive(Debug, Clone, Default)]
pub struct NewMessage {
    pub conversation_id: Option<i32>,
    pub content: Option<String>,
    pub message_type: String,
    pub msg_type: i32,
    pub user_id: Option<i32>,
    pub servant_id: Option<i32>,
    pub send_time: i32,
    pub shop_id: i32,
    pub user_from: Option<String>,
}

/// IM 消息域仓储。
#[derive(Clone)]
pub struct ImMessageRepository {
    pool: PgPool,
}

impl ImMessageRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 列表查询 —————————————————————

    /// 消息列表分页（对应 Java `ImMessageServiceImpl#list` 的 `this.page`）。
    ///
    /// `order_by` 为**已白名单化**的 `ORDER BY` 片段（不含 `ORDER BY` 关键字），
    /// 由服务层根据 `sortField`/`sortOrder` 生成（默认 `` `id` DESC ``，
    /// 与 Java `buildSortOrder` 空值→主键→降序一致）。
    pub async fn page_messages(
        &self,
        f: &MessageListFilter,
        order_by: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<MessageRow>)> {
        let mut clauses: Vec<String> = Vec::new();
        if f.conversation_id.unwrap_or(0) > 0 {
            clauses.push("conversation_id = ?".to_string());
        }
        if f.user_id.unwrap_or(0) > 0 {
            clauses.push("user_id = ?".to_string());
        }
        if f.shop_id.is_some_and(|v| v > -1) {
            clauses.push("shop_id = ?".to_string());
        }
        if f.first_id.is_some_and(|v| v > -1) {
            clauses.push("id < ?".to_string());
        }
        if f.servant_id.is_some() {
            clauses.push("servant_id = ?".to_string());
        }
        if f.msg_type.is_some() {
            clauses.push("\"type\" = ?".to_string());
        }
        if f.is_read.is_some() {
            clauses.push("is_read = ?".to_string());
        }
        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) AS total FROM im_message {where_sql}");
        let vals = list_bind_values(f);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for v in &vals {
            cq = cq.bind(*v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let page_sql =
            format!("SELECT {IM_MESSAGE_COLUMNS} FROM im_message {where_sql} ORDER BY {order_by} LIMIT ? OFFSET ?");
        let mut pq = sqlx::query_as::<_, MessageRow>(&page_sql);
        for v in &vals {
            pq = pq.bind(*v);
        }
        pq = pq.bind(limit).bind(offset);
        let rows = pq.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 客服消息播报记录分页（对应 Java `ImMessageServiceImpl#receivedList`）。
    ///
    /// 固定口径：`servant_id = ? AND type = 1 AND status = 1`
    /// （`shop_id` 非空且 > -1 时追加 `AND shop_id = ?`），
    /// `ORDER BY send_time DESC, id DESC`（最新在前）。
    pub async fn page_received(
        &self,
        servant_id: i32,
        shop_id: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<MessageRow>)> {
        let with_shop = shop_id.is_some_and(|v| v > -1);
        let shop_clause = if with_shop { " AND shop_id = ?" } else { "" };

        let count_sql = format!(
            "SELECT COUNT(*) AS total FROM im_message WHERE servant_id = $1 AND \"type\" = 1 AND status = 1{shop_clause}"
        );
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(servant_id);
        if with_shop {
            cq = cq.bind(shop_id.unwrap_or(0));
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let page_sql = format!(
            "SELECT {IM_MESSAGE_COLUMNS} FROM im_message WHERE servant_id = $1 AND \"type\" = 1 AND status = 1{shop_clause} ORDER BY send_time DESC, \"id\" DESC LIMIT $2 OFFSET $3"
        );
        let mut pq = sqlx::query_as::<_, MessageRow>(&page_sql).bind(servant_id);
        if with_shop {
            pq = pq.bind(shop_id.unwrap_or(0));
        }
        pq = pq.bind(limit).bind(offset);
        let rows = pq.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total.0, rows))
    }

    // ————————————————————— 会话读写 —————————————————————

    /// 按主键取会话（Java `imConversationMapper.selectById`）。
    pub async fn find_conversation(&self, id: i32) -> Result<Option<ImConversation>> {
        sqlx::query_as::<_, ImConversation>(
            "SELECT id, user_id, last_servant_id, add_time, shop_id, user_from, status, last_update_time, is_delete, remark, summary FROM im_conversation WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按「用户 + 店铺」取未删除会话（Java send 的 `selectOne` + `limit 1`）。
    pub async fn find_conversation_by_user_shop(
        &self,
        user_id: i32,
        shop_id: i32,
    ) -> Result<Option<ImConversation>> {
        sqlx::query_as::<_, ImConversation>(
            "SELECT id, user_id, last_servant_id, add_time, shop_id, user_from, status, last_update_time, is_delete, remark, summary FROM im_conversation WHERE user_id = $1 AND shop_id = $2 AND is_delete = 0 LIMIT 1",
        )
        .bind(user_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 更新会话的客服归属与状态（Java `updateById` 对 `last_servant_id`/`status`）。
    pub async fn update_conversation_servant(
        &self,
        id: i32,
        last_servant_id: i32,
        status: i32,
    ) -> Result<u64> {
        let r = sqlx::query("UPDATE im_conversation SET last_servant_id = $1, status = $2 WHERE id = $3")
            .bind(last_servant_id)
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 刷新会话最后更新时间（Java `sentMessage` 的 `setLastUpdateTime`）。
    pub async fn touch_conversation(&self, id: i32, ts: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE im_conversation SET last_update_time = $1 WHERE id = $2")
            .bind(ts)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 新建会话，返回自增主键（Java send 的 `imConversationMapper.insert`）。
    ///
    /// `remark` / `summary` 是 NOT NULL 且 Java 未显式赋值（依赖 DDL 默认空串），
    /// 这里同样落空串。
    pub async fn insert_conversation(
        &self,
        user_id: i32,
        shop_id: i32,
        user_from: Option<&str>,
        last_servant_id: i32,
        add_time: i32,
        status: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO im_conversation (user_id, shop_id, user_from, last_servant_id, add_time, status, is_delete, remark, summary) VALUES ($1, $2, $3, $4, $5, $6, 0, '', '') RETURNING id",
        )
        .bind(user_id)
        .bind(shop_id)
        .bind(user_from)
        .bind(last_servant_id)
        .bind(add_time)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 统计某客服进行中的会话数（Java send 的 `selectCount` status=1）。
    pub async fn count_active_conversations(&self, servant_id: i32) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) AS total FROM im_conversation WHERE last_servant_id = $1 AND status = 1")
            .bind(servant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        row.try_get::<i64, _>("total").map_err(map_err)
    }

    // ————————————————————— 客服 / 接入配置 —————————————————————

    /// 取店铺客服接入配置（Java `ShopKefuConfigService.getByShopId` 的读部分）。
    pub async fn find_kefu_config(&self, shop_id: i32) -> Result<Option<ShopKefuConfig>> {
        sqlx::query_as::<_, ShopKefuConfig>(
            "SELECT id, shop_id, kefu_type, kefu_yzf_type, kefu_yzf_sign, kefu_workwx_id, corp_id, kefu_code, kefu_code_blank, kefu_whatsapp_phone, kefu_phone, kefu_time, auto_access_mode, return_customer_access, create_time, modify_time FROM shop_kefu_config WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 `servant_id + shop_id + status=1` 取在线客服（回头客复用）。
    pub async fn find_online_servant_by_id(
        &self,
        servant_id: i32,
        shop_id: i32,
    ) -> Result<Option<ImServant>> {
        sqlx::query_as::<_, ImServant>(
            "SELECT id, servant_id, last_update_time, add_time, status, shop_id, conversation_num FROM im_servant WHERE servant_id = $1 AND shop_id = $2 AND status = 1 LIMIT 1",
        )
        .bind(servant_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取接待数最少的在线客服（默认自动接入分配）。
    pub async fn find_least_loaded_online_servant(&self, shop_id: i32) -> Result<Option<ImServant>> {
        sqlx::query_as::<_, ImServant>(
            "SELECT id, servant_id, last_update_time, add_time, status, shop_id, conversation_num FROM im_servant WHERE shop_id = $1 AND status = 1 ORDER BY conversation_num ASC LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 客服是否在线（Java `isServantOnline`：`servant_id = ? AND status = 1`）。
    pub async fn is_servant_online(&self, servant_id: i32) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) AS total FROM im_servant WHERE servant_id = $1 AND status = 1")
            .bind(servant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let n: i64 = row.try_get("total").map_err(map_err)?;
        Ok(n > 0)
    }

    /// 店铺是否存在指定状态的客服（`hasBusyServant` / `hasOfflineServant`）。
    pub async fn shop_has_servant_status(&self, shop_id: i32, status: i32) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) AS total FROM im_servant WHERE shop_id = $1 AND status = $2")
            .bind(shop_id)
            .bind(status)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let n: i64 = row.try_get("total").map_err(map_err)?;
        Ok(n > 0)
    }

    /// 更新客服接待会话数（Java send 的 `imServantUp.setConversationNum` + `updateById`）。
    pub async fn update_servant_conversation_num(
        &self,
        servant_id: i32,
        shop_id: i32,
        num: i32,
    ) -> Result<u64> {
        let r = sqlx::query("UPDATE im_servant SET conversation_num = $1 WHERE servant_id = $2 AND shop_id = $3")
            .bind(num)
            .bind(servant_id)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 消息写入 / 已读 —————————————————————

    /// 插入一条消息，返回自增主键（Java `this.save(imMessage)`）。
    pub async fn insert_message(&self, m: &NewMessage) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO im_message (conversation_id, content, message_type, \"type\", user_id, servant_id, send_time, shop_id, user_from, is_read, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, 1) RETURNING id",
        )
        .bind(m.conversation_id)
        .bind(m.content.as_deref())
        .bind(&m.message_type)
        .bind(m.msg_type)
        .bind(m.user_id)
        .bind(m.servant_id)
        .bind(m.send_time)
        .bind(m.shop_id)
        .bind(m.user_from.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键回读刚插入的消息行（用于组装发送 VO）。
    pub async fn find_message(&self, id: i32) -> Result<Option<MessageRow>> {
        sqlx::query_as::<_, MessageRow>(&format!(
            "SELECT {IM_MESSAGE_COLUMNS} FROM im_message WHERE id = $1 LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 后台设置已读（Java `setRead` admin 分支）：
    /// `is_read = 1 WHERE shop_id = ? AND is_read = 0 AND type = 1 AND user_id = ?`。
    pub async fn mark_read_for_admin(&self, shop_id: i32, user_id: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE im_message SET is_read = 1 WHERE shop_id = $1 AND is_read = 0 AND \"type\" = 1 AND user_id = $2",
        )
        .bind(shop_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 前台设置已读（Java `setRead` user 分支）：
    /// `is_read = 1 WHERE user_id = ? AND is_read = 0 AND type = 2 AND shop_id = ?`。
    pub async fn mark_read_for_user(&self, user_id: i32, shop_id: i32) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE im_message SET is_read = 1 WHERE user_id = $1 AND is_read = 0 AND \"type\" = 2 AND shop_id = $2",
        )
        .bind(user_id)
        .bind(shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 用户 / 客服装配 —————————————————————

    /// 批量取用户简信息（Java `getUserMap` → `selectBatchIds`）。
    pub async fn find_users_by_ids(&self, ids: &[i32]) -> Result<Vec<UserBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!("SELECT user_id, username, nickname, avatar FROM \"user\" WHERE user_id IN ({ph})");
        let mut q = sqlx::query_as::<_, UserBrief>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取客服（管理员）简信息（Java `adminUserMapper.selectBatchIds`）。
    pub async fn find_admins_by_ids(&self, ids: &[i32]) -> Result<Vec<AdminBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!("SELECT admin_id, username, avatar FROM admin_user WHERE admin_id IN ({ph})");
        let mut q = sqlx::query_as::<_, AdminBrief>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 单个用户简信息（Java `sendMessageVO` 的 `userMapper.selectById`）。
    pub async fn find_user(&self, user_id: i32) -> Result<Option<UserBrief>> {
        sqlx::query_as::<_, UserBrief>(
            "SELECT user_id, username, nickname, avatar FROM \"user\" WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 单个客服（管理员）简信息（Java `adminUserMapper.selectById`）。
    pub async fn find_admin(&self, admin_id: i32) -> Result<Option<AdminBrief>> {
        sqlx::query_as::<_, AdminBrief>(
            "SELECT admin_id, username, avatar FROM admin_user WHERE admin_id = $1 LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 会员默认头像配置（Java `getDefaultAvatar` → `configService.getConfigByCode("defaultAvatar")`）。
    ///
    /// `config` 表按 `biz_code` 取 `biz_val`（`is_del = 0`）；缺行 → `None`
    /// （调用方回退静态默认，与 user_center/011 口径一致）。
    pub async fn default_avatar_config(&self) -> Result<Option<String>> {
        let row = sqlx::query("SELECT biz_val FROM config WHERE biz_code = 'defaultAvatar' AND is_del = 0 LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.and_then(|r| r.try_get::<Option<String>, _>("biz_val").ok().flatten()))
    }
}

/// 收集列表 WHERE 的绑定值（顺序须与 `page_messages` 拼接的 WHERE 片段一致）。
fn list_bind_values(f: &MessageListFilter) -> Vec<i32> {
    let mut v = Vec::new();
    if f.conversation_id.unwrap_or(0) > 0 {
        v.push(f.conversation_id.unwrap_or(0));
    }
    if f.user_id.unwrap_or(0) > 0 {
        v.push(f.user_id.unwrap_or(0));
    }
    if f.shop_id.is_some_and(|x| x > -1) {
        v.push(f.shop_id.unwrap_or(0));
    }
    if f.first_id.is_some_and(|x| x > -1) {
        v.push(f.first_id.unwrap_or(0));
    }
    if let Some(x) = f.servant_id {
        v.push(x);
    }
    if let Some(x) = f.msg_type {
        v.push(x);
    }
    if let Some(x) = f.is_read {
        v.push(x);
    }
    v
}

/// `?` 占位符列表。
fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

/// `sqlx::Error` → 项目统一错误。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("IM 消息不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_expand() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?,?,?");
    }

    #[test]
    fn map_err_semantics() {
        assert!(matches!(map_err(sqlx::Error::RowNotFound), Error::NotFound(_)));
        assert!(matches!(map_err(sqlx::Error::Protocol("x".into())), Error::Database(_)));
    }
}