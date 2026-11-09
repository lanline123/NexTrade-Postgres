//! 语言地区维护表（`locales`）管理端模型（lang::locales）。
//!
//! Java 来源（overseas）：
//! - `LocalesController.list/detail/create/update/updateField/del/batch`
//! - `LocalesListDTO` / `LocalesDTO` / `LocalesVO`（`bean/vo/lang/LocalesVO`）

use serde::{Deserialize, Serialize};

use super::common::BasePageQuery;

/// `GET /adminapi/lang/locales/list` 查询参数（Java `LocalesListDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalesListQuery {
    /// 基础分页字段平铺（Java `LocalesListDTO extends BasePage`）
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
    /// 是否启用（1/0；缺省不过滤）
    pub is_enabled: Option<i32>,
}

impl LocalesListQuery {
    /// 分页参数（Java `BasePage` 口径）。
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        BasePageQuery {
            page: self.page,
            size: self.size,
            ..Default::default()
        }
        .page_params()
    }
}

/// `create` / `update` 请求体（Java `LocalesDTO`，字段同实体）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalesDTO {
    pub id: Option<i32>,
    /// 语言地区代码，如 `en_US`
    pub locale_code: Option<String>,
    /// 语言，如 `English` / `中文`
    pub language: Option<String>,
    /// 访问域名（b2b2b 表增量列）
    pub domain: Option<String>,
    /// 国旗图片
    pub flag_picture: Option<String>,
    pub is_enabled: Option<i32>,
    pub is_default: Option<i32>,
    /// 货币 id（`currency.id`）
    pub currency_id: Option<i32>,
    pub sort: Option<i32>,
}

/// 货币摘要（`LocalesVO.currency`，Java `bean.model.lang.Currency` 直出）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyEmbedVO {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub is_default: Option<i32>,
    pub rate: Option<String>,
    /// ISO 4217 代号（b2b2b 表增量列）
    pub code: Option<String>,
}

/// 语言地区 VO（Java `LocalesVO`；`lastUpdated` 为 `yyyy-MM-dd HH:mm:ss` 文本）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalesVO {
    pub id: Option<i32>,
    pub locale_code: Option<String>,
    pub language: Option<String>,
    pub domain: Option<String>,
    pub flag_picture: Option<String>,
    /// 最后更新时间（`last_updated` 秒级时间戳格式化；Java 空值取当前时间）
    pub last_updated: Option<String>,
    pub is_enabled: Option<i32>,
    pub is_default: Option<i32>,
    pub currency_id: Option<i32>,
    pub sort: Option<i32>,
    /// 关联货币（`list` 时批量补齐）
    pub currency: Option<CurrencyEmbedVO>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_vo_serializes_camel_case() {
        let vo = LocalesVO {
            id: Some(1),
            locale_code: Some("en_US".into()),
            is_default: Some(1),
            currency: Some(CurrencyEmbedVO {
                id: Some(2),
                symbol: Some("$".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&vo).unwrap();
        assert!(json.contains(r#""localeCode":"en_US""#));
        assert!(json.contains(r#""isDefault":1"#));
        assert!(json.contains(r#""currency""#));
    }

    #[test]
    fn list_query_accepts_camel_case() {
        let q: LocalesListQuery =
            serde_json::from_str(r#"{"page":1,"size":20,"isEnabled":1}"#).unwrap();
        assert_eq!(q.is_enabled, Some(1));
        assert_eq!(q.page_params(), (1, 20));
    }
}
