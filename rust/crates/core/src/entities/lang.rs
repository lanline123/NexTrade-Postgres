//! 多语言域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `ai_translate_language`, `currency`, `image_i18n`, `locales`, `locales_lang`, `locales_relation`, `translations`, `translations_data`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// AI图片翻译语言配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AiTranslateLanguage {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 语言代码，如 zh / en / zh-tw [NOT NULL]
    pub lang_code: String,
    /// 语言显示名称 [NOT NULL]
    pub lang_name: String,
    /// 是否可作为翻译源：0否 1是 [NOT NULL]
    pub is_source: i32,
    /// 是否可作为翻译目标：0否 1是 [NOT NULL]
    pub is_target: i32,
    /// 排序号（升序） [NOT NULL]
    pub sort: i32,
}

impl AiTranslateLanguage {
    /// 数据库表名。
    pub const TABLE: &'static str = "ai_translate_language";
}


/// 货币管理表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Currency {
    /// [NOT NULL]
    pub id: i32,
    /// 币种名称 [NOT NULL]
    pub name: String,
    /// 币种符号 [NOT NULL]
    pub symbol: String,
    /// 是否默认：1 是 0 否 [NOT NULL]
    pub is_default: i32,
    /// 汇率 [NOT NULL]
    pub rate: String,
    /// 国际通用代号(ISO 4217，如 USD/EUR/CNY)，汇率源匹配依据 [NOT NULL]
    pub code: String,
}

impl Currency {
    /// 数据库表名。
    pub const TABLE: &'static str = "currency";
}


/// 海外站图片本地化映射
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ImageI18n {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 语言id [NOT NULL]
    pub locale_id: i32,
    /// 数据类型：7=商品图片 8=规格图片 [NOT NULL]
    pub data_type: i32,
    /// 原图URL的MD5 [NOT NULL]
    pub original_key: String,
    /// 原图URL [NOT NULL]
    pub original_url: String,
    /// 本地化后的图URL [NOT NULL]
    pub localized_url: String,
    /// 创建时间
    pub add_time: Option<i32>,
}

impl ImageI18n {
    /// 数据库表名。
    pub const TABLE: &'static str = "image_i18n";
}


/// 语言地区维护表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Locales {
    /// [NOT NULL]
    pub id: i32,
    /// 语言地区代码，如 'en_US'
    pub locale_code: Option<String>,
    /// 语言，如 'English、中文、繁體中文'
    pub language: Option<String>,
    /// 该语言对应的访问域名，如 en.domain.com，不含协议与端口，为空表示不启用域名跳转 [NOT NULL]
    pub domain: String,
    /// 国旗图片
    pub flag_picture: Option<String>,
    /// 最后更新时间
    pub last_updated: Option<i32>,
    /// 是否启用1启用0禁用
    pub is_enabled: Option<i32>,
    /// 是否默认：1 是 0 否
    pub is_default: Option<i32>,
    /// 货币id
    pub currency_id: Option<i32>,
    /// 排序
    pub sort: Option<i32>,
}

impl Locales {
    /// 数据库表名。
    pub const TABLE: &'static str = "locales";
}


/// locales_lang
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LocalesLang {
    pub code: Option<String>,
    pub name: Option<String>,
    /// [NOT NULL]
    pub id: i32,
    pub name_en: Option<String>,
}

impl LocalesLang {
    /// 数据库表名。
    pub const TABLE: &'static str = "locales_lang";
}


/// 语言与浏览器语言标识对应表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LocalesRelation {
    /// [NOT NULL]
    pub id: i32,
    /// 地区名称
    pub name: Option<String>,
    /// 浏览器语言标识
    pub code: Option<String>,
    pub locales_id: Option<i32>,
}

impl LocalesRelation {
    /// 数据库表名。
    pub const TABLE: &'static str = "locales_relation";
}


/// 应用翻译维护表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Translations {
    /// [NOT NULL]
    pub id: i32,
    /// 待翻译标识
    pub translation_name: Option<String>,
    /// 根据name生成的md5
    pub translation_key: Option<String>,
    /// 0 页面，1 接口
    pub data_type: Option<i32>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl Translations {
    /// 数据库表名。
    pub const TABLE: &'static str = "translations";
}


/// 数据翻译维护表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct TranslationsData {
    /// [NOT NULL]
    pub id: i32,
    pub locale_id: Option<i32>,
    /// 待翻译内容
    pub translation_name: Option<String>,
    /// 根据name生成的md5
    pub translation_key: Option<String>,
    /// 翻译后的值
    pub translation_value: Option<String>,
    pub data_type: Option<i32>,
    /// 数据ID，如商品ID，分类ID，品牌ID，导航菜单ID
    pub data_id: Option<i32>,
    /// 译文来源：0-未知/人工 1-AI翻译 2-AI智能消除
    pub translation_source: Option<i32>,
}

impl TranslationsData {
    /// 数据库表名。
    pub const TABLE: &'static str = "translations_data";
}