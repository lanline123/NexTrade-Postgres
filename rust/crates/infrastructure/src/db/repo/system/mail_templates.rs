//! 邮箱模板管理域（system::mail_templates）仓储：`mail_templates`。
//!
//! 对应原 Java `com.nextrade.mapper.setting.MailTemplatesMapper`
//! （`BaseMapper<MailTemplates>` + `BaseServiceImpl` 的通用方法）。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table mail_templates`）：
//!
//! `mail_templates`（8 列，主键 `template_id`）：
//!
//! | 列 | 类型 | 可空 |
//! |----|------|------|
//! | `template_id` | `int unsigned` | NOT NULL（AUTO_INCREMENT） |
//! | `template_code` | `varchar(30)` | NOT NULL |
//! | `is_html` | `tinyint unsigned` | NOT NULL |
//! | `template_subject` | `varchar(200)` | NOT NULL |
//! | `template_content` | `text` | NOT NULL |
//! | `last_modify` | `int unsigned` | NOT NULL |
//! | `last_send` | `int unsigned` | NOT NULL |
//! | `type` | `varchar(10)` | NOT NULL |
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//! `type` 是 MySQL 关键字，SQL 中以反引号包裹（与本仓库其它仓储一致）。
//!
//! 金额：本域**不涉及任何金额列**。

use sqlx::PgPool;

use nextrade_core::entities::setting::MailTemplates;
use nextrade_core::{Error, Result};

/// 固定 SELECT 列清单。
///
/// 不用 `SELECT *`：显式列出全部 8 列，避免与实体字段顺序/数量耦合
/// （顺序与 `entities::setting::MailTemplates` 的字段顺序一致）。
const MAIL_TEMPLATES_SELECT: &str = "SELECT template_id, template_code, is_html, template_subject, template_content, last_modify, last_send, \"type\" FROM mail_templates";

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct MailTemplatesRepository {
    pool: PgPool,
}

impl MailTemplatesRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 查询 —————————————————————

    /// 全部邮件模板（对应 Java `MailTemplatesServiceImpl#mailTemplatesList`
    /// 里的 `this.list()`，即 MyBatis-Plus 无 wrapper 的全表查询）。
    ///
    /// ⚠️ Java `this.list()` **没有** `ORDER BY`。本实现补了
    /// `ORDER BY template_id ASC`：`InnoDB` 全表扫描（走聚簇索引）本来就
    /// 按主键顺序返回，加显式排序只是把「事实上确定」变成「契约上确定」，
    /// 不改变可观测顺序（见 `uncertain_points`）。
    pub async fn find_all(&self) -> Result<Vec<MailTemplates>> {
        sqlx::query_as::<_, MailTemplates>(&format!(
            "{MAIL_TEMPLATES_SELECT} ORDER BY template_id ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按主键查询（对应 Java `getById`）。
    ///
    /// 仅用于 `updateMailTemplates` 的审计日志取「邮件主题」
    /// （Java `getTemplateSubject(templateId)`）。
    pub async fn find_by_id(&self, template_id: i32) -> Result<Option<MailTemplates>> {
        sqlx::query_as::<_, MailTemplates>(&format!(
            "{MAIL_TEMPLATES_SELECT} WHERE template_id = $1 LIMIT 1"
        ))
        .bind(template_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 写入 —————————————————————

    /// 更新邮件模板（对应 Java `MailTemplatesServiceImpl#updateMailTemplates`
    /// 里的 `this.updateById(mailTemplates)`）。
    ///
    /// MyBatis-Plus `updateById` 默认 `FieldStrategy.NOT_NULL`：**只写
    /// 非 null 字段**。Java 侧是 `new MailTemplates()` +
    /// `BeanUtils.copyProperties(dto, entity)` 后再 `setLastModify(...)`，
    /// 因此实体里非 null 的字段只有：
    /// `template_id`（走 WHERE）、`is_html`（DTO 传了才有）、
    /// `template_subject`、`template_content`、`last_modify`。
    ///
    /// 故生成的 `SET` 子句（顺序与实体字段顺序一致）为：
    /// `is_html = ?, template_subject = ?, template_content = ?, last_modify = ?`
    /// —— `is_html` 为 `None` 时整列不写，保留库中旧值。
    ///
    /// ⚠️ `updateById` 命中 0 行（主键不存在）时 Java **静默成功**
    /// （`isUpdated == false`，仅跳过审计日志），不抛异常；
    /// 本方法同样只返回受影响行数，由调用方决定是否记录。
    pub async fn update(&self, template_id: i32, patch: &MailTemplatesPatch) -> Result<u64> {
        let affected = match patch.is_html {
            Some(is_html) => {
                sqlx::query("UPDATE mail_templates SET is_html = $1, template_subject = $2, template_content = $3, last_modify = $4 WHERE template_id = $5")
                    .bind(is_html)
                    .bind(&patch.template_subject)
                    .bind(&patch.template_content)
                    .bind(patch.last_modify)
                    .bind(template_id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?
                    .rows_affected()
            }
            None => {
                sqlx::query("UPDATE mail_templates SET template_subject = $1, template_content = $2, last_modify = $3 WHERE template_id = $4")
                    .bind(&patch.template_subject)
                    .bind(&patch.template_content)
                    .bind(patch.last_modify)
                    .bind(template_id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?
                    .rows_affected()
            }
        };
        Ok(affected)
    }
}

/// 更新邮件模板的列补丁（对应 MyBatis-Plus `updateById` 的
/// `FieldStrategy.NOT_NULL` 语义）。
///
/// - `is_html`：`None` ⇒ **不写该列**（保留旧值）；`Some(v)` ⇒ 写该列
/// - `template_subject` / `template_content`：Java `@NotBlank` 保证非 null，
///   故这里是裸 `String`
/// - `last_modify`：服务层写入「当前秒级时间戳」
///   （Java `StringUtils.getCurrentTime()` = `System.currentTimeMillis() / 1000`）
#[derive(Debug, Clone)]
pub struct MailTemplatesPatch {
    /// `is_html`（`None` ⇒ 不写该列）
    pub is_html: Option<i32>,
    /// `template_subject`
    pub template_subject: String,
    /// `template_content`
    pub template_content: String,
    /// `last_modify`（秒级时间戳）
    pub last_modify: i32,
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("邮件模板不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SELECT 子句必须覆盖全部 8 列，且不得使用 `SELECT *`。
    #[test]
    fn select_clause_lists_all_schema_columns() {
        for col in [
            "template_id",
            "template_code",
            "is_html",
            "template_subject",
            "template_content",
            "last_modify",
            "last_send",
            "type",
        ] {
            assert!(
                MAIL_TEMPLATES_SELECT.contains(col),
                "SELECT 缺少列 {col}: {MAIL_TEMPLATES_SELECT}"
            );
        }
        assert!(!MAIL_TEMPLATES_SELECT.contains('*'));
        // 表名必须正确
        assert!(MAIL_TEMPLATES_SELECT.contains("FROM mail_templates"));
    }
}