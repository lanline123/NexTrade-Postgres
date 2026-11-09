//! 商品关键词联想服务（`catalog/product_search`，1 个前台接口）。
//!
//! 对应原 Java `ProductSearchServiceImpl#searchGuess(String keyword)`（295-363 行），
//! 供 `SearchGuessController#getSearchGuessList`（`GET /api/search/searchGuess/index`）
//! 使用。
//!
//! 流程见 [`nextrade_core::models::catalog::product_search`] 模块头；
//! 本层只做「归一化 → 查商品 → 组装联想」。
//!
//! # 偏差
//!
//! - 中文分词：Java 用 HanLP `TokenizerEngine`；本工作区无分词依赖，按空白切分
//!   （近似实现，见模型层模块头）。
//! - 无 ES 参与（Java 该接口为纯 MySQL 查询，无偏差）。
//! - 商品名/关键词未做 `translatePackage` 翻译（同其它已迁移特性）。

use std::sync::Arc;

use nextrade_core::models::catalog::product_search::{
    build_guess_suggestions, cut_for_search, normalize_guess_keyword, KeywordSearchListVO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

pub struct ProductSearchService {
    repos: Arc<Repositories>,
}

impl ProductSearchService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 搜索关键词联想（`GET /api/search/searchGuess/index?keyword=`）。
    ///
    /// 对应 Java `searchGuess`：
    /// `null` / 空白 / 单数字 → 空列表；否则按商品名多词元 AND 查商品，
    /// 再按 `keywords` 双向包含统计，取计数 Top 10。
    pub async fn search_guess(&self, keyword: Option<&str>) -> Result<Vec<KeywordSearchListVO>> {
        let Some(keyword) = normalize_guess_keyword(keyword) else {
            return Ok(Vec::new());
        };
        let tokens = cut_for_search(&keyword);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let products = self
            .repos.catalog.product_search
            .find_by_name_tokens(&tokens)
            .await?;
        let rows: Vec<(i32, String, Option<String>)> = products
            .into_iter()
            .map(|p| (p.product_id, p.product_name, p.keywords))
            .collect();
        Ok(build_guess_suggestions(&rows, &keyword))
    }
}

#[cfg(test)]
mod tests {
    use nextrade_core::models::catalog::product_search::{
        build_guess_suggestions, cut_for_search, normalize_guess_keyword,
    };

    /// 服务层入口的三段前置判断（与 Java `searchGuess` 前 10 行一一对应）：
    /// 归一化失败 / 无词元时不应触达数据库。
    #[test]
    fn entry_short_circuits_without_query() {
        assert!(normalize_guess_keyword(None).is_none());
        assert!(normalize_guess_keyword(Some(" ")).is_none());
        assert!(normalize_guess_keyword(Some("3")).is_none());
        let keyword = normalize_guess_keyword(Some(" 手机 ")).unwrap();
        assert_eq!(cut_for_search(&keyword), vec!["手机"]);
    }

    #[test]
    fn end_to_end_suggestion_shape_matches_java() {
        let products = vec![
            (1, "苹果手机".to_string(), Some("苹果 手机".to_string())),
            (2, "苹果耳机".to_string(), Some("苹果 耳机".to_string())),
        ];
        let out = build_guess_suggestions(&products, "苹果手机");
        // 命中「苹果」2 个商品、「手机」1 个
        assert_eq!(out[0].keyword.as_deref(), Some("苹果"));
        assert_eq!(out[0].count, Some(2));
        assert_eq!(out[0].r#type.as_deref(), Some("search"));
        assert_eq!(out[1].keyword.as_deref(), Some("手机"));
        assert_eq!(out[1].count, Some(1));
    }
}
