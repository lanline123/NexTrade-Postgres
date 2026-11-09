//! 文章批量操作/前台文章域（content::article_batch）DTO / 纯函数。
//!
//! 对应 Java `ArticleController` 与文章域服务的 3 个端点：
//! - `POST /adminapi/content/article/batch`（批量删除）
//! - `GET /api/article/article/issueInfo`（文章信息，按 id；缺省取最新一篇）
//! - `GET /api/article/category/indexBzzxList`（分类 + 子文章列表，「帮助中心」样式）
//!
//! 表：`article` / `article_category`。

use serde::{Deserialize, Serialize};

/// `未选择项目`
pub const MSG_UNSELECTED_ITEM: &str = "未选择项目";
/// `操作类型不能为空`
pub const MSG_OPERATION_TYPE_REQUIRED: &str = "操作类型不能为空";
/// `id不能为空`
pub const MSG_ID_REQUIRED: &str = "id不能为空";
/// 前台分类聚合默认每类文章数（与「帮助中心」列表展示口径一致）。
pub const CATEGORY_ARTICLES_LIMIT: u64 = 5;

/// 批量操作（Java `BatchDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchDTO {
    pub ids: Vec<i32>,
    pub r#type: String,
}

/// 文章查询（Java `issueInfo` 参数）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArticleIssueQuery {
    /// 原版 PC 文章详情页使用 `?articleId=`，同时兼容 `?id=`。
    #[serde(alias = "articleId")]
    pub id: Option<i32>,
}

/// 文章详情（Java `ArticleVO` 常用字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArticleVO {
    pub article_id: i32,
    pub article_title: String,
    pub article_category_id: i32,
    pub article_sn: String,
    pub article_thumb: String,
    pub article_tag: String,
    pub article_author: String,
    pub article_type: i32,
    pub content: String,
    pub description: Option<String>,
    pub keywords: String,
    pub is_show: Option<i32>,
}

/// 文章详情页数据（原版 `issueInfo` 返回 `{ item, prev, next }`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArticleIssueInfoVO {
    pub item: ArticleVO,
    pub prev: Option<ArticleBriefVO>,
    pub next: Option<ArticleBriefVO>,
}

/// 文章摘要（分类下文章列表）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArticleBriefVO {
    pub article_id: i32,
    pub article_title: String,
    pub article_thumb: String,
    pub article_sn: String,
}

/// 分类 + 文章列表（Java `childrenArticleListWithCategory` 返回）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryWithArticlesVO {
    pub article_category_id: i32,
    pub article_category_name: String,
    pub category_sn: String,
    pub sort_order: i32,
    pub articles: Vec<ArticleBriefVO>,
}

/// 批量操作是否删除类型（Java `"del".equals(type)`）。
#[must_use]
pub fn is_delete_batch(type_: &str) -> bool {
    type_ == "del"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_type_semantics() {
        assert!(is_delete_batch("del"));
        assert!(!is_delete_batch("show"));
        assert_eq!(MSG_UNSELECTED_ITEM, "未选择项目");
        assert_eq!(MSG_OPERATION_TYPE_REQUIRED, "操作类型不能为空");
    }

    #[test]
    fn params_accept_java_names() {
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1,2],"type":"del"}"#).unwrap();
        assert_eq!(dto.ids, vec![1, 2]);
        let q: ArticleIssueQuery = serde_json::from_str(r#"{"id":3}"#).unwrap();
        assert_eq!(q.id, Some(3));
    }

    #[test]
    fn vo_serializes_camel_case() {
        let vo = ArticleVO {
            article_id: 1,
            article_title: "标题".into(),
            ..Default::default()
        };
        let value = serde_json::to_value(&vo).unwrap();
        assert_eq!(value["articleId"], 1);
        assert_eq!(value["articleTitle"], "标题");
        let cw = CategoryWithArticlesVO {
            article_category_id: 2,
            articles: vec![ArticleBriefVO {
                article_id: 3,
                ..Default::default()
            }],
            ..Default::default()
        };
        let value = serde_json::to_value(&cw).unwrap();
        assert_eq!(value["articles"][0]["articleId"], 3);
    }
}
