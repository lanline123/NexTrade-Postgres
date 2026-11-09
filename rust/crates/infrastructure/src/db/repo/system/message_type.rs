//! 消息分类（`MessageTypeController`）域仓储（system::message_type）。
//! 对应原 Java `MessageTypeMapper` / `MessageTemplateMapper` /
//! `ConfigService#getConfigVal`。
//!
//! 列名严格对齐权威 schema `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table message_type` 可复核）：
//! - `message_type`：`message_id`（主键）/ `name` / `describe` / `send_type` /
//!   `is_wechat` / `is_mini_program` / `is_message` / `is_msg` / `is_app` /
//!   `is_ding` / `add_time` / `is_wx_work` / `is_mail`
//! - `message_template`：`id`（主键）/ `message_id` / `type` / `template_name` /
//!   `to_userid` / `template_id` / `template_num` / `content` / `content_oversea`
//! - `config`：`biz_code` / `biz_val` / `is_del`（短信服务商等设置项）
//!
//! 查询统一使用运行时 `sqlx::query_as::<_, T>(...)` / `sqlx::query(...)`，
//! **不使用 `query!` 宏**（本机无 `DATABASE_URL`）。
//!
//! 说明：`describe` 是 MySQL 保留字，但 `pure.sql` 中该列未加反引号即为
//! `describe`，MySQL 允许其作为列名（仅 `DESCRIBE` 语句有关键字角色）。
//! 为与实体 [`nextrade_core::entities::MessageType`] 的 `SELECT *` 映射一致，
//! 本仓储也统一用 `SELECT *`，避免显式列举列名时的引号歧义。

use sqlx::{Postgres,PgPool};
use nextrade_core::entities::{MessageTemplate, MessageType};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `message_type` 表名（供服务层事务编排与文档引用）。
pub const MESSAGE_TYPE_TABLE: &str = "message_type";

/// `message_template` 表名。
pub const MESSAGE_TEMPLATE_TABLE: &str = "message_template";

/// 构造一个**惰性**（不建立实际连接）的 PostgreSQL 池。
///
/// 仅供单元测试实例化 [`nextrade_core`] 之外的 `Repositories` 使用：
/// `nextrade-services` 不直接依赖 `sqlx`，又需要构造 `Repositories` 才能
/// 触达服务层结构与「校验先于 IO」的断言。任何真实 SQL 在此池上都会
/// 以连接错误失败 —— 这正是那些测试想验证的行为。
///
/// 生产代码**不得**使用本函数。
#[doc(hidden)]
#[must_use]
pub fn lazy_test_pool() -> PgPool {
    PgPool::connect_lazy("postgres://NexTrade:NexTrade@127.0.0.1:5432/nextrade")
        .expect("惰性池构造不应失败：URL 为静态合法值")
}

/// `message_type` 表可排序的列白名单（对应 Java
/// `BaseServiceImpl#getColumnByPropertyName` 在 `MessageType` 实体上的映射）。
pub const MESSAGE_TYPE_SORT_COLUMNS: &[&str] = &[
    "message_id",
    "name",
    "describe",
    "send_type",
    "is_wechat",
    "is_mini_program",
    "is_message",
    "is_msg",
    "is_app",
    "is_ding",
    "add_time",
    "is_wx_work",
    "is_mail",
];

/// 列表排序方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    /// SQL 片段（`ASC` / `DESC`），仅用于拼接**已白名单校验**的字面量。
    #[must_use]
    pub fn as_sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

#[derive(Clone)]
pub struct MessageTypeRepository {
    pool: PgPool,
}

impl MessageTypeRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排用，避免服务层直接依赖 `sqlx` 类型）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 消息分类 —————————————————————

    /// 按 ID 查询消息分类，对应 Java `MessageTypeServiceImpl#getById`。
    ///
    /// 对应 Java `detail(Integer id)` 的 `this.getById(id)`；
    /// Java 在查不到时会把 `null` 拷进 DTO（返回**全空对象**，不抛异常）。
    pub async fn find_by_id(&self, message_id: i32) -> Result<Option<MessageType>> {
        sqlx::query_as::<_, MessageType>(
            "SELECT * FROM message_type WHERE message_id = $1 LIMIT 1",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 取消息名称，对应 Java `MessageTypeServiceImpl#getName(Integer)`：
    /// `eq(MessageType::getMessageId, messageId)` 后取 `name`，仅用于
    /// `update` 末尾的操作日志文案 `更新消息类型:{name}`。
    ///
    /// 返回 `Option<String>`：Java 的 `getOne(...).getName()` 在无记录时
    /// 会抛 `NullPointerException`，这里用 `None` 表达「取不到」。
    pub async fn find_name_by_id(&self, message_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT name FROM message_type WHERE message_id = $1 LIMIT 1")
                .bind(message_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(name,)| name))
    }

    /// 消息分类分页，对应 Java `list(page, queryWrapper)`。
    ///
    /// `keyword` 走 `LIKE '%kw%'`（Java `like(MessageType::getName, keyword)`），
    /// `send_type` 精确匹配（Java `eq(MessageType::getSendType, sendType)`）。
    ///
    /// `order_column` / `dir` 必须由调用方经
    /// [`MESSAGE_TYPE_SORT_COLUMNS`] 白名单校验后传入（防 SQL 注入）。
    ///
    /// **注意**：返回的 `total` 是**过滤前**的库内总数，与 Java 一致 ——
    /// Java 用 MyBatis-Plus 的 `Page` 计数（不带版本差异过滤器），
    /// 版本相关的过滤是在取回当页 records **之后**用 `Stream.filter` 做的，
    /// 因此当页被过滤掉的条目不会回填，`total` 也不重算。
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        send_type: Option<i32>,
        order_column: &str,
        dir: SortDir,
        page: &PageQuery,
    ) -> Result<(u64, Vec<MessageType>)> {
        let like = keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));
        let (limit, offset) = page.limit_offset();

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM message_type
             WHERE ($1 IS NULL OR name LIKE $2)
               AND ($3 IS NULL OR send_type = $4)",
        )
        .bind(&like)
        .bind(&like)
        .bind(send_type)
        .bind(send_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        // `order_column` / `dir` 为白名单内字面量（调用方已校验），可用 format! 拼接。
        let sql = format!(
            "SELECT * FROM message_type
             WHERE ($1 IS NULL OR name LIKE $2)
               AND ($3 IS NULL OR send_type = $4)
             ORDER BY {order_column} {}
             LIMIT $5 OFFSET $6",
            dir.as_sql()
        );
        let rows = sqlx::query_as::<_, MessageType>(&sql)
            .bind(&like)
            .bind(&like)
            .bind(send_type)
            .bind(send_type)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((u64::try_from(total.0.max(0)).unwrap_or(0), rows))
    }

    /// 更新消息分类基础字段（**只更新非 `None` 的列**）。
    ///
    /// 对应 Java `updateById(MessageType)`：MyBatis-Plus 默认
    /// `FieldStrategy.NOT_NULL`，因此 `null` 字段不参与 `SET`，
    /// 从而不会清空库中已有值。这里用逐列 `COALESCE` 达到同样效果：
    /// `SET is_msg = COALESCE(?, is_msg)` —— 绑定 `None` 时保持原值。
    ///
    /// Java `setMessageTypeFields` 的额外语义：各 `isXxx` 字段仅当
    /// `> -1` 时才赋值，`-1` 表示「不支持」由上层拦截（服务层处理）。
    pub async fn update_fields(
        &self,
        message_id: i32,
        name: Option<&str>,
        describe: Option<&str>,
        send_type: Option<i32>,
        is_wechat: Option<i32>,
        is_mini_program: Option<i32>,
        is_message: Option<i32>,
        is_msg: Option<i32>,
        is_app: Option<i32>,
        is_ding: Option<i32>,
        is_mail: Option<i32>,
        is_wx_work: Option<i32>,
    ) -> Result<u64> {
        self.update_fields_via(
            &mut *self.pool.acquire().await.map_err(map_err)?,
            message_id,
            name,
            describe,
            send_type,
            is_wechat,
            is_mini_program,
            is_message,
            is_msg,
            is_app,
            is_ding,
            is_mail,
            is_wx_work,
        )
        .await
    }

    /// [`Self::update_fields`] 的事务内版本（由服务层 `update` 在显式事务中调用，
    /// 复刻 Java `@Transactional(rollbackFor = Exception.class)`）。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_fields_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        message_id: i32,
        name: Option<&str>,
        describe: Option<&str>,
        send_type: Option<i32>,
        is_wechat: Option<i32>,
        is_mini_program: Option<i32>,
        is_message: Option<i32>,
        is_msg: Option<i32>,
        is_app: Option<i32>,
        is_ding: Option<i32>,
        is_mail: Option<i32>,
        is_wx_work: Option<i32>,
    ) -> Result<u64> {
        self.update_fields_via(
            &mut **tx,
            message_id,
            name,
            describe,
            send_type,
            is_wechat,
            is_mini_program,
            is_message,
            is_msg,
            is_app,
            is_ding,
            is_mail,
            is_wx_work,
        )
        .await
    }

    /// 共用的 SQL 实现：`executor` 既可以是池连接，也可以是事务。
    #[allow(clippy::too_many_arguments)]
    async fn update_fields_via<'e, E>(
        &self,
        executor: E,
        message_id: i32,
        name: Option<&str>,
        describe: Option<&str>,
        send_type: Option<i32>,
        is_wechat: Option<i32>,
        is_mini_program: Option<i32>,
        is_message: Option<i32>,
        is_msg: Option<i32>,
        is_app: Option<i32>,
        is_ding: Option<i32>,
        is_mail: Option<i32>,
        is_wx_work: Option<i32>,
    ) -> Result<u64>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        let r = sqlx::query(
            "UPDATE message_type SET
               name = COALESCE($1, name),
               describe = COALESCE($2, describe),
               send_type = COALESCE($3, send_type),
               is_wechat = COALESCE($4, is_wechat),
               is_mini_program = COALESCE($5, is_mini_program),
               is_message = COALESCE($6, is_message),
               is_msg = COALESCE($7, is_msg),
               is_app = COALESCE($8, is_app),
               is_ding = COALESCE($9, is_ding),
               is_mail = COALESCE($10, is_mail),
               is_wx_work = COALESCE($11, is_wx_work)
             WHERE message_id = $12",
        )
        .bind(name)
        .bind(describe)
        .bind(send_type)
        .bind(is_wechat)
        .bind(is_mini_program)
        .bind(is_message)
        .bind(is_msg)
        .bind(is_app)
        .bind(is_ding)
        .bind(is_mail)
        .bind(is_wx_work)
        .bind(message_id)
        .execute(executor)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 单字段更新，对应 Java `BaseServiceImpl#updateField`。
    ///
    /// `column` 必须由调用方经白名单（Java `CheckFieldConstants.MESSAGE_TYPE_FIELDS`）
    /// 校验后传入；值统一以字符串绑定，交由 MySQL 隐式转换
    /// （与 Java `UpdateWrapper.set(field, val)` 的 String 绑定一致）。
    pub async fn update_field(&self, message_id: i32, column: &str, val: &str) -> Result<u64> {
        let sql = format!("UPDATE message_type SET {column} = $1 WHERE message_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(message_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 模板消息 —————————————————————

    /// 某消息分类下的全部模板，对应 Java
    /// `MessageTemplateServiceImpl#listByMessageId`（`eq(message_id, ?)`）。
    pub async fn find_templates_by_message_id(
        &self,
        message_id: i32,
    ) -> Result<Vec<MessageTemplate>> {
        sqlx::query_as::<_, MessageTemplate>(
            "SELECT * FROM message_template WHERE message_id = $1 ORDER BY id ASC",
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按消息类型名称取短信模板 ID（`message_template.type = 3`）。
    ///
    /// 原版后台「消息类型」页面把短信模板 ID 存在
    /// `message_template.template_id`，而不是 `config.smsVerificationTemplate`。
    /// 这里按名称前缀匹配，例如「获取验证码」。
    pub async fn find_sms_template_id_by_message_name(
        &self,
        message_name: &str,
    ) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT t.template_id
               FROM message_template t
               JOIN message_type mt ON mt.message_id = t.message_id
              WHERE t.type = 3
                AND mt.name LIKE CONCAT($1, '%')
                AND t.template_id IS NOT NULL
                AND t.template_id <> ''
              ORDER BY t.id ASC
              LIMIT 1",
        )
        .bind(message_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 按主键更新单个模板（**只更新非 `None` 的列**），对应 Java
    /// `MessageTemplateService#updateBatchById` 的单条语义
    /// （MyBatis-Plus `updateById` 为 `NOT_NULL` 策略）。
    ///
    /// Java `MessageTypeServiceImpl#updateTemplate` 对不同类型的模板只设置
    /// 部分字段（`template_name` / `content` / `template_id` /
    /// `content_oversea`），其余字段保持 `null` 因而**不覆盖**库中值 ——
    /// 这里用逐列 `COALESCE` 复刻该行为。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_template_fields(
        &self,
        id: i32,
        template_name: Option<&str>,
        template_id: Option<&str>,
        content: Option<&str>,
        content_oversea: Option<&str>,
    ) -> Result<u64> {
        self.update_template_fields_via(
            &mut *self.pool.acquire().await.map_err(map_err)?,
            id,
            template_name,
            template_id,
            content,
            content_oversea,
        )
        .await
    }

    /// [`Self::update_template_fields`] 的事务内版本。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_template_fields_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
        template_name: Option<&str>,
        template_id: Option<&str>,
        content: Option<&str>,
        content_oversea: Option<&str>,
    ) -> Result<u64> {
        self.update_template_fields_via(
            &mut **tx,
            id,
            template_name,
            template_id,
            content,
            content_oversea,
        )
        .await
    }

    /// 共用的 SQL 实现：`executor` 既可以是池连接，也可以是事务。
    #[allow(clippy::too_many_arguments)]
    async fn update_template_fields_via<'e, E>(
        &self,
        executor: E,
        id: i32,
        template_name: Option<&str>,
        template_id: Option<&str>,
        content: Option<&str>,
        content_oversea: Option<&str>,
    ) -> Result<u64>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        let r = sqlx::query(
            "UPDATE message_template SET
               template_name = COALESCE($1, template_name),
               template_id = COALESCE($2, template_id),
               content = COALESCE($3, content),
               content_oversea = COALESCE($4, content_oversea)
             WHERE id = $5",
        )
        .bind(template_name)
        .bind(template_id)
        .bind(content)
        .bind(content_oversea)
        .bind(id)
        .execute(executor)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 系统配置 —————————————————————

    /// 按 `biz_code` 取未删除配置值，对应 Java `ConfigService#getConfigVal`
    /// （`biz_code = ? AND is_del = 0 LIMIT 1`）。
    ///
    /// Java 缺行返回 `null`（调用方兜默认值），这里返回 `None`。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
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

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("消息分类不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_dir_sql_literals() {
        assert_eq!(SortDir::Asc.as_sql(), "ASC");
        assert_eq!(SortDir::Desc.as_sql(), "DESC");
    }

    #[test]
    fn sort_whitelist_covers_all_message_type_columns() {
        // 与 schema_parser 输出逐一对应，防止列名漂移。
        for col in [
            "message_id",
            "name",
            "describe",
            "send_type",
            "is_wechat",
            "is_mini_program",
            "is_message",
            "is_msg",
            "is_app",
            "is_ding",
            "add_time",
            "is_wx_work",
            "is_mail",
        ] {
            assert!(
                MESSAGE_TYPE_SORT_COLUMNS.contains(&col),
                "缺少列 {col}"
            );
        }
        assert_eq!(MESSAGE_TYPE_SORT_COLUMNS.len(), 13);
    }
}