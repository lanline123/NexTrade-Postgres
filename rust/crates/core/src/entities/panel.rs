//! 面板实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `panel_shortcut_menu`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 工作台首页快捷功能配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PanelShortcutMenu {
    /// 自增ID [NOT NULL]
    pub id: i32,
    /// 商家管理员ID（取自 token） [NOT NULL]
    pub admin_id: i32,
    /// 店铺ID（取自请求头 X-Shop-Id） [NOT NULL]
    pub shop_id: i32,
    /// [json]有序的功能 key 字符串数组，数组顺序=首页展示顺序
    pub menu_keys: Option<String>,
    /// 更新时间（秒级时间戳）
    pub update_time: Option<i64>,
}

impl PanelShortcutMenu {
    /// 数据库表名。
    pub const TABLE: &'static str = "panel_shortcut_menu";
}