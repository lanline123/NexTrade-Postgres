//! 微信公众号域实体（关键词回复/菜单）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `wechat_keyword_rule`, `wechat_keyword_rule_keyword`, `wechat_keyword_rule_reply`, `wechat_menu`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 公众号关键字回复-规则主表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatKeywordRule {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 规则名称，≤60，全局唯一（不区分大小写，依赖 utf8mb4_general_ci + uk_rule_name） [NOT NULL]
    pub rule_name: String,
    /// 回复模式：1=随机回复一条 2=回复全部 [NOT NULL]
    pub reply_mode: i32,
    /// 启用状态：0=停用 1=启用 [NOT NULL]
    pub enabled: i32,
    /// [NOT NULL]
    pub created_at: String,
    /// [NOT NULL]
    pub updated_at: String,
}

impl WechatKeywordRule {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_keyword_rule";
}


/// 公众号关键字回复-关键字子表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatKeywordRuleKeyword {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 所属规则 id（逻辑外键 wechat_keyword_rule.id） [NOT NULL]
    pub rule_id: i64,
    /// 关键字，≤30 [NOT NULL]
    pub keyword: String,
    /// 匹配方式：1=全匹配 2=半匹配 [NOT NULL]
    pub match_type: i32,
    /// 规则内排序，0 起递增 [NOT NULL]
    pub sort_order: i32,
}

impl WechatKeywordRuleKeyword {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_keyword_rule_keyword";
}


/// 公众号关键字回复-回复内容子表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatKeywordRuleReply {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 所属规则 id（逻辑外键 wechat_keyword_rule.id） [NOT NULL]
    pub rule_id: i64,
    /// 规则内排序，0 起递增 [NOT NULL]
    pub sort_order: i32,
    /// 消息类型：1=文本 2=图文 [NOT NULL]
    pub msg_type: i32,
    /// 文本内容（msg_type=1 必填，≤300 字符）
    pub content_text: Option<String>,
    /// 图文标题（msg_type=2）
    pub news_title: Option<String>,
    /// 图文封面图 URL（msg_type=2，HTTPS 公网可访问）
    pub news_cover_url: Option<String>,
    /// 图文摘要（msg_type=2，≤120 字符）
    pub news_description: Option<String>,
    /// 图文跳转链接（msg_type=2 必填）
    pub news_url: Option<String>,
}

impl WechatKeywordRuleReply {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_keyword_rule_reply";
}


/// 公众号自定义菜单（单例）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatMenu {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 后台自有结构的菜单树 JSON（含 id/排序字段），发布时转微信官方 button 结构 [NOT NULL]
    pub menu_json: String,
    /// 最近一次成功推送到微信的时间；NULL 表示仅保存未发布
    pub published_at: Option<String>,
    /// [NOT NULL]
    pub created_at: String,
    /// [NOT NULL]
    pub updated_at: String,
}

impl WechatMenu {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_menu";
}