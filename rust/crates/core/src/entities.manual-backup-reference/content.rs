//! `article` / `article_category` 内容实体。
//! 对应 `com.nextrade.bean.model.content.Article` 与 `ArticleCategory`。
//!
//! 字段严格对齐 `java/database/pure.sql`：
//! - `CREATE TABLE article`（L408）
//! - `CREATE TABLE article_category`（L440）
//!
//! 注意：表列名带 `article_` 前缀（如 `article_title`、`article_category_id`），
//! 而 `article_category` 的排序列为 `sort_order` 且**无 is_show 列**。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 文章类型：普通文章。
pub const ARTICLE_TYPE_NORMAL: i32 = 1;

/// 文章内容（表 `article`）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Article {
    /// 自增ID `article_id`
    pub article_id: i32,
    /// 文章标题 `article_title`
    pub article_title: String,
    /// 该文章的分类，对应 `article_category.article_category_id`；0 表示未分类
    pub article_category_id: i32,
    /// 文章编码 `article_sn`
    pub article_sn: String,
    /// 缩略图 `article_thumb`
    pub article_thumb: String,
    /// 文章标签 `article_tag`
    pub article_tag: String,
    /// 文章作者 `article_author`
    pub article_author: String,
    /// 文章类型 `article_type`（默认 1）
    pub article_type: i32,
    /// 文章内容（HTML）`content`
    pub content: String,
    /// 文章描述 `description`（可空）
    pub description: Option<String>,
    /// 文章关键字 `keywords`
    pub keywords: String,
    /// 是否显示：1 显示 0 不显示（可空，默认 1）
    pub is_show: Option<i32>,
    /// 添加时间（秒）`add_time`
    pub add_time: i32,
    /// 是否热门文章：0 否 1 是
    pub is_hot: i32,
    /// 是否置顶：0 否 1 是
    pub is_top: i32,
    /// 浏览次数 `click_count`
    pub click_count: i32,
    /// 相关链接 `link`
    pub link: String,
}

impl Article {
    /// 表名（供仓储层复用）。
    pub const TABLE: &'static str = "article";

    /// 未分类文章的 `article_category_id` 取值。
    pub const UNCATEGORIZED: i32 = 0;

    /// 是否在前台显示（`is_show` 缺省视为显示，与表默认值 1 一致）。
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.is_show.unwrap_or(1) == 1
    }

    /// 是否为热门文章。
    #[must_use]
    pub fn is_hot_article(&self) -> bool {
        self.is_hot == 1
    }

    /// 是否置顶。
    #[must_use]
    pub fn is_top_article(&self) -> bool {
        self.is_top == 1
    }

    /// 是否未分类。
    #[must_use]
    pub fn is_uncategorized(&self) -> bool {
        self.article_category_id == Self::UNCATEGORIZED
    }
}

/// 文章分类（表 `article_category`）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ArticleCategory {
    /// 自增ID `article_category_id`
    pub article_category_id: i32,
    /// 分类名称 `article_category_name`
    pub article_category_name: String,
    /// 分类编码 `category_sn`
    pub category_sn: String,
    /// 分类类型：1 普通分类 2 帮助分类
    pub category_type: i32,
    /// 分类关键字 `keywords`
    pub keywords: String,
    /// 分类描述 `description`
    pub description: String,
    /// 分类显示顺序 `sort_order`（默认 50）
    pub sort_order: i32,
    /// 父节点ID，取值于本表 `article_category_id`，0 为顶级
    pub parent_id: i32,
}

impl ArticleCategory {
    /// 表名（供仓储层复用）。
    pub const TABLE: &'static str = "article_category";

    /// 分类类型：普通分类。
    pub const TYPE_NORMAL: i32 = 1;
    /// 分类类型：帮助分类。
    pub const TYPE_HELP: i32 = 2;

    /// 顶级分类的 `parent_id` 取值。
    pub const ROOT_PARENT: i32 = 0;

    /// 是否根（顶级）分类。
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_id == Self::ROOT_PARENT
    }

    /// 是否为帮助分类。
    #[must_use]
    pub fn is_help(&self) -> bool {
        self.category_type == Self::TYPE_HELP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_defaults_to_shown() {
        // 列可空且表默认 1：NULL 视为显示
        let a = Article::default();
        assert!(a.is_visible());

        let hidden = Article {
            is_show: Some(0),
            ..Default::default()
        };
        assert!(!hidden.is_visible());
    }

    #[test]
    fn flags_and_uncategorized() {
        let a = Article {
            is_hot: 1,
            is_top: 1,
            article_category_id: Article::UNCATEGORIZED,
            ..Default::default()
        };
        assert!(a.is_hot_article());
        assert!(a.is_top_article());
        assert!(a.is_uncategorized());

        let categorized = Article {
            article_category_id: 7,
            ..Default::default()
        };
        assert!(!categorized.is_uncategorized());
    }

    #[test]
    fn category_root_and_type() {
        let root = ArticleCategory {
            parent_id: ArticleCategory::ROOT_PARENT,
            category_type: ArticleCategory::TYPE_NORMAL,
            ..Default::default()
        };
        assert!(root.is_root());
        assert!(!root.is_help());

        let help_child = ArticleCategory {
            parent_id: 3,
            category_type: ArticleCategory::TYPE_HELP,
            ..Default::default()
        };
        assert!(!help_child.is_root());
        assert!(help_child.is_help());
    }
}