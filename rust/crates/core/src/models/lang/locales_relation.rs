//! 语言关联表（`locales_relation` / `locales_lang`）管理端模型（lang::localesRelation）。
//!
//! Java 来源（overseas）：`LocalesRelationController.list/detail/create/
//! update/del/batch/config`（`config` = `localesLangService.list()` 直出
//! `locales_lang` 全表）。

use serde::{Deserialize, Serialize};

/// `create` / `update` 请求体（Java `LocalesRelationDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalesRelationDTO {
    pub id: Option<i32>,
    /// 地区名称
    pub name: Option<String>,
    /// 浏览器语言标识
    pub code: Option<String>,
    /// 关联语言 id（`locales.id`）
    pub locales_id: Option<i32>,
}

/// 语言关联 VO（Java `LocalesRelationVO`，字段同实体）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalesRelationVO {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub code: Option<String>,
    pub locales_id: Option<i32>,
}

/// `locales_lang` 行（Java `bean.model.lang.LocalesLang` 直出，
/// `config` 端点返回该表全量列表）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalesLangVO {
    pub id: Option<i32>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub name_en: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_vo_serializes_camel_case() {
        let vo = LocalesRelationVO {
            id: Some(1),
            name: Some("美国".into()),
            code: Some("en-US".into()),
            locales_id: Some(2),
        };
        let json = serde_json::to_string(&vo).unwrap();
        assert!(json.contains(r#""localesId":2"#));
    }

    #[test]
    fn locales_lang_vo_name_en_field() {
        let vo = LocalesLangVO {
            id: Some(1),
            code: Some("en".into()),
            name: Some("英语".into()),
            name_en: Some("English".into()),
        };
        let json = serde_json::to_string(&vo).unwrap();
        assert!(json.contains(r#""nameEn":"English""#));
    }
}
