//! 装修模板域服务（content::decorate_template）。
//!
//! category/list/detail 转发官网 OpenTemplate API（`open-template.base-url` 可配），
//! 业务字段原样透传；detail 的 `sn` 必填。

use std::sync::Arc;

use nextrade_core::models::content::decorate_template::{
    validate_detail_sn, DecorateTemplateDetailQuery, DecorateTemplateListQuery,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::decorate_template::DecorateTemplateRepository;
use nextrade_infrastructure::db::Repositories;

/// 官网模板库默认地址（Java `open-template.base-url` 缺省值）。
const DEFAULT_BASE_URL: &str = "https://api.nextrade.com/index";

/// 装修模板服务（转发官网 OpenTemplate API，业务字段原样透传）。
pub struct DecorateTemplateService {
    repos: Arc<Repositories>,
}

impl DecorateTemplateService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`self.repos.content.decorate_template`）。
    #[must_use]
    pub fn repo(&self) -> &DecorateTemplateRepository {
        &self.repos.content.decorate_template
    }

    /// 模板分类（Java `category` → `GET /open/template/category`）。
    pub async fn category(&self) -> Result<serde_json::Value> {
        self.fetch("/open/template/category", &[]).await
    }

    /// 模板列表（Java `list` → `GET /open/template/list`）。
    pub async fn list(&self, q: &DecorateTemplateListQuery) -> Result<serde_json::Value> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.size.unwrap_or(15).clamp(1, 100);
        let mut params: Vec<(&str, String)> = vec![
            ("page", page.to_string()),
            ("size", size.to_string()),
        ];
        if let Some(v) = q.keyword.as_deref().filter(|s| !s.trim().is_empty()) {
            params.push(("keyword", v.to_string()));
        }
        if let Some(v) = q.page_type.as_deref().filter(|s| !s.trim().is_empty()) {
            params.push(("pageType", v.to_string()));
        }
        if let Some(v) = q.category_ids.as_deref().filter(|s| !s.trim().is_empty()) {
            params.push(("categoryIds", v.to_string()));
        }
        self.fetch("/open/template/list", &params).await
    }

    /// 模板详情（Java `detail` → `GET /open/template/detail?sn=`）。
    pub async fn detail(&self, q: &DecorateTemplateDetailQuery) -> Result<serde_json::Value> {
        let sn = validate_detail_sn(q.sn.as_deref()).map_err(Error::Validation)?;
        self.fetch("/open/template/detail", &[("sn", sn)]).await
    }

    /// 请求官网模板库并解包 `{ code, msg, data: { errcode, message, ... } }`。
    async fn fetch(&self, path: &str, query: &[(&str, String)]) -> Result<serde_json::Value> {
        let base = std::env::var("OPEN_TEMPLATE_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let url = format!("{}{path}", base.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| Error::Internal(format!("初始化模板服务客户端失败: {e}")))?;
        let resp = client
            .get(&url)
            .query(query)
            .send()
            .await
            .map_err(|e| Error::Database(format!("模板服务请求失败: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::Database(format!(
                "模板服务返回 HTTP {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Database(format!("模板服务响应解析失败: {e}")))?;
        let data = body
            .get("data")
            .cloned()
            .ok_or_else(|| Error::Database("模板服务响应格式异常".into()))?;
        if let Some(code) = data.get("errcode").and_then(serde_json::Value::as_i64) {
            if code != 0 {
                let message = data
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("模板服务错误");
                return Err(Error::biz_msg(message.to_string()));
            }
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_validates_sn() {
        assert!(validate_detail_sn(None).is_err());
        assert!(validate_detail_sn(Some("  ")).is_err());
        assert_eq!(validate_detail_sn(Some("page_home")).unwrap(), "page_home");
    }

}
