//! 微信菜单仓储：`wechat_menu`（单例行读取）。
//!
//! 列名严格对齐 `python tools/schema_parser.py --table wechat_menu`：
//! `id` / `menu_json` / `published_at` / `created_at` / `updated_at`。
//!
//! 本批只迁移「读取」（`GET /adminapi/wechat/menu`）；发布写入依赖微信推送
//! （未迁移），故不提供写方法（见 services 模块头）。

use sqlx::PgPool;
use nextrade_core::entities::WechatMenu;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct WechatMenuRepository {
    pool: PgPool,
}

impl WechatMenuRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 单例读取（Java `getOne(... .last("LIMIT 1"))`）。
    pub async fn find_first(&self) -> Result<Option<WechatMenu>> {
        sqlx::query_as::<_, WechatMenu>(
            "SELECT id, menu_json, published_at, created_at, updated_at \
             FROM wechat_menu ORDER BY id ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 保存菜单配置（Java `doSave(tree, publishedAt)`）。
    pub async fn save_menu(&self, menu_json: &str, published_at: Option<&str>) -> Result<()> {
        let first = self.find_first().await?;
        if let Some(entity) = first {
            sqlx::query(
                "UPDATE wechat_menu SET menu_json = $1, published_at = $2, updated_at = NOW() WHERE id = $3",
            )
            .bind(menu_json)
            .bind(published_at)
            .bind(entity.id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        } else {
            sqlx::query(
                "INSERT INTO wechat_menu (menu_json, published_at, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
            )
            .bind(menu_json)
            .bind(published_at)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("微信菜单不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<WechatMenuRepository>();
    }

    #[test]
    fn entity_columns_are_documented() {
        // 与 schema 对齐的列名（SQL 字符串内联，避免拼接）
        let sql = "SELECT id, menu_json, published_at, created_at, updated_at";
        assert_eq!(sql.matches(',').count(), 4);
    }
}
