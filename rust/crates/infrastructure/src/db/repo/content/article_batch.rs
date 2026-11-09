//! 文章批量操作/前台文章域仓储（content::article_batch）。
//!
//! 表：`article` / `article_category`；查询使用运行时 `sqlx::query_as`。

use sqlx::PgPool;

use nextrade_core::models::content::article_batch::{
    ArticleBriefVO, ArticleVO, CategoryWithArticlesVO,
};
use nextrade_core::{Error, Result};

/// 文章域仓储。
#[derive(Clone)]
pub struct ArticleBatchRepository {
    pool: PgPool,
}

impl ArticleBatchRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 批量删除文章（Java `removeByIds`）。
    pub async fn delete_articles(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!("DELETE FROM article WHERE article_id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(map_err)
    }

    /// 按 id 查文章。
    pub async fn find_article(&self, id: i32) -> Result<Option<ArticleVO>> {
        let row = sqlx::query_as::<_, (i32, String, i32, String, String, String, String, i32, String, Option<String>, String, Option<i32>)>(
            "SELECT article_id, article_title, article_category_id, article_sn, article_thumb,
                    article_tag, article_author, article_type, content, description, keywords, is_show
             FROM article WHERE article_id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| ArticleVO {
            article_id: r.0,
            article_title: r.1,
            article_category_id: r.2,
            article_sn: r.3,
            article_thumb: r.4,
            article_tag: r.5,
            article_author: r.6,
            article_type: r.7,
            content: r.8,
            description: r.9,
            keywords: r.10,
            is_show: r.11,
        }))
    }

    /// 最新一篇文章（`issueInfo` 无 id 时）。
    pub async fn find_latest_article(&self) -> Result<Option<ArticleVO>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT article_id FROM article ORDER BY article_id DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        match row {
            Some((id,)) => self.find_article(id).await,
            None => Ok(None),
        }
    }

    /// 同分类上一篇 / 下一篇（按 article_id 顺序）。
    pub async fn find_adjacent_articles(
        &self,
        article_id: i32,
        category_id: i32,
    ) -> Result<(Option<ArticleBriefVO>, Option<ArticleBriefVO>)> {
        let prev: Option<(i32, String, String, String)> = sqlx::query_as(
            "SELECT article_id, article_title, article_thumb, article_sn FROM article
             WHERE article_category_id = $1 AND article_id < $2 AND is_show = 1
             ORDER BY article_id DESC LIMIT 1",
        )
        .bind(category_id)
        .bind(article_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let next: Option<(i32, String, String, String)> = sqlx::query_as(
            "SELECT article_id, article_title, article_thumb, article_sn FROM article
             WHERE article_category_id = $1 AND article_id > $2 AND is_show = 1
             ORDER BY article_id ASC LIMIT 1",
        )
        .bind(category_id)
        .bind(article_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((prev.map(brief_from_row), next.map(brief_from_row)))
    }

    /// 全部分类（按 sort_order 升序）。
    pub async fn list_categories(&self) -> Result<Vec<(i32, String, String, i32)>> {
        let rows: Vec<(i32, String, String, i32)> = sqlx::query_as(
            "SELECT article_category_id, article_category_name, category_sn, sort_order
             FROM article_category ORDER BY sort_order ASC, article_category_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows)
    }

    /// 分类下文章（上架优先，按 article_id 倒序取前 N）。
    pub async fn list_articles_by_category(&self, category_id: i32, limit: u64) -> Result<Vec<ArticleBriefVO>> {
        let rows: Vec<(i32, String, String, String)> = sqlx::query_as(
            "SELECT article_id, article_title, article_thumb, article_sn
             FROM article WHERE article_category_id = $1 AND (is_show = 1 OR is_show IS NULL)
             ORDER BY article_id DESC LIMIT $2",
        )
        .bind(category_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(article_id, article_title, article_thumb, article_sn)| ArticleBriefVO {
                article_id,
                article_title,
                article_thumb,
                article_sn,
            })
            .collect())
    }

    /// 分类 + 文章列表（`indexBzzxList`）。
    pub async fn list_categories_with_articles(
        &self,
        limit: u64,
    ) -> Result<Vec<CategoryWithArticlesVO>> {
        let categories = self.list_categories().await?;
        let mut result = Vec::with_capacity(categories.len());
        for (article_category_id, article_category_name, category_sn, sort_order) in categories {
            let articles = self
                .list_articles_by_category(article_category_id, limit)
                .await?;
            result.push(CategoryWithArticlesVO {
                article_category_id,
                article_category_name,
                category_sn,
                sort_order,
                articles,
            });
        }
        Ok(result)
    }
}

fn brief_from_row(row: (i32, String, String, String)) -> ArticleBriefVO {
    ArticleBriefVO {
        article_id: row.0,
        article_title: row.1,
        article_thumb: row.2,
        article_sn: row.3,
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_type_exists() {
        assert!(std::any::type_name::<ArticleBatchRepository>().contains("ArticleBatch"));
    }
}
