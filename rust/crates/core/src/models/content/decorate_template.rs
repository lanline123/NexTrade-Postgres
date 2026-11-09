//! 装修模板域（content::decorate_template）DTO / 纯函数。
//!
//! 对应原 Java `DecorateTemplateController` 的 3 个 GET 端点：
//! `category`、`list`、`detail`（`/adminapi/decorate/template/...`）。
//!
//! 装修模板依赖模板资源包 + 装修渲染引擎（模板文件/DIY 组件/预览数据），
//! Rust 侧尚未迁移 → 保留端点与本地校验，返回明确「未迁移」业务错误，不编造模板数据。

use serde::{Deserialize, Serialize};

/// `id不能为空`
pub const MSG_ID_REQUIRED: &str = "id不能为空";
/// 装修模板未迁移。
pub const MSG_NOT_MIGRATED: &str =
    "装修模板依赖模板资源包与装修渲染引擎（模板文件/DIY 组件/预览数据），当前 Rust 侧尚未迁移，本接口暂不可用";

/// 模板列表查询（Java `DecorateTemplatePageQuery` 子集）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DecorateTemplateListQuery {
    pub keyword: Option<String>,
    pub page_type: Option<String>,
    pub category_ids: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

/// 模板详情查询（Java `@RequestParam id`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DecorateTemplateDetailQuery {
    pub sn: Option<String>,
}

/// 模板分类（Java `DecorateTemplateCategoryVO` 子集）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DecorateTemplateCategoryVO {
    pub category_id: i32,
    pub category_name: String,
}

/// 模板（Java `DecorateTemplateVO` 子集；未迁移时不会返回数据）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DecorateTemplateVO {
    pub template_id: i32,
    pub template_name: String,
    pub cover: Option<String>,
    pub category_id: Option<i32>,
    pub page_count: Option<i32>,
    pub add_time: Option<i64>,
}

/// 分页归一（Java `BasePage`）。
#[must_use]
pub fn page_params(page: Option<u64>, size: Option<u64>) -> (u64, u64) {
    (page.unwrap_or(1).max(1), size.unwrap_or(15).clamp(1, 100))
}

/// 详情 sn 校验。
#[must_use]
pub fn validate_detail_sn(sn: Option<&str>) -> Result<String, String> {
    sn.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| MSG_ID_REQUIRED.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_requires_sn() {
        assert_eq!(validate_detail_sn(None).unwrap_err(), MSG_ID_REQUIRED);
        assert_eq!(validate_detail_sn(Some("  ")).unwrap_err(), MSG_ID_REQUIRED);
        assert_eq!(validate_detail_sn(Some("page_home")).unwrap(), "page_home");
    }

    #[test]
    fn page_params_and_message() {
        assert_eq!(page_params(None, None), (1, 15));
        assert_eq!(page_params(Some(2), Some(1000)), (2, 100));
        assert!(MSG_NOT_MIGRATED.contains("尚未迁移"));
    }

    #[test]
    fn query_accepts_java_names() {
        let q: DecorateTemplateListQuery =
            serde_json::from_str(r#"{"categoryIds":"2","keyword":"首页","page":1}"#).unwrap();
        assert_eq!(q.category_ids.as_deref(), Some("2"));
        let q: DecorateTemplateDetailQuery = serde_json::from_str(r#"{"sn":"page_5"}"#).unwrap();
        assert_eq!(q.sn.as_deref(), Some("page_5"));
    }
}
