//! 页面装修域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `decorate`, `decorate_discrete`, `decorate_share`, `mobile_cat_nav`, `pc_cat_floor`, `pc_navigation`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 装修页面表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Decorate {
    /// [NOT NULL]
    pub decorate_id: i32,
    /// 模板名称
    pub decorate_title: Option<String>,
    /// 已发布的-页面模块数据
    pub data: Option<String>,
    /// 草稿-页面模块数据
    pub draft_data: Option<String>,
    /// 页面类型 1:h5  2:PC
    pub decorate_type: Option<i32>,
    /// 页面类型:1首页 2个人中心 3活动 4分类 5登录 6搜索 8商品详情页 [NOT NULL]
    pub page_type: i32,
    /// 是否是首页
    pub is_home: Option<i32>,
    /// 所属店铺 [NOT NULL]
    pub shop_id: i32,
    pub status: Option<i32>,
    /// 更新时间
    pub update_time: Option<i32>,
    /// 关联语言默认为0
    pub locale_id: Option<i32>,
    /// 父级id
    pub parent_id: Option<i32>,
    /// 排序
    pub sort_order: Option<i32>,
    /// 绑定的h5页面id（decorate_type = 3 生效）
    pub binded_h5_id: Option<i32>,
    /// 分享信息（JSON：shareTitle/shareDesc/shareImage）
    pub share_info: Option<String>,
}

impl Decorate {
    /// 数据库表名。
    pub const TABLE: &'static str = "decorate";
}


/// 装修组件
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct DecorateDiscrete {
    /// 装修单模块（非页面） [NOT NULL]
    pub id: i32,
    /// 模块sn [NOT NULL]
    pub decorate_sn: String,
    /// 模块名称
    pub decorate_name: Option<String>,
    /// [JOSN]模块数据 [NOT NULL]
    pub data: String,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl DecorateDiscrete {
    /// 数据库表名。
    pub const TABLE: &'static str = "decorate_discrete";
}


/// 装修页面分享表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct DecorateShare {
    /// 自增ID号 [NOT NULL]
    pub share_id: i32,
    /// 分享sn码 [NOT NULL]
    pub share_sn: String,
    /// 分享验证token [NOT NULL]
    pub share_token: String,
    /// 装修页面id [NOT NULL]
    pub decorate_id: i32,
    /// token有效时间 [NOT NULL]
    pub valid_time: i32,
    /// 添加时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl DecorateShare {
    /// 数据库表名。
    pub const TABLE: &'static str = "decorate_share";
}


/// 手机分类
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MobileCatNav {
    /// [NOT NULL]
    pub mobile_cat_nav_id: i32,
    /// 选择分类 [NOT NULL]
    pub category_id: i32,
    /// 分类颜色（颜色选择器） [NOT NULL]
    pub cat_color: String,
    /// [JSON]推荐子分类（多选） [NOT NULL]
    pub child_cat_ids: String,
    /// 推荐品牌 [NOT NULL]
    pub brand_ids: String,
    /// 是否显示：1显示，0不显示 [NOT NULL]
    pub is_show: i32,
    /// 分类banner字符串，英文符号'｜' 分割 [NOT NULL]
    pub img_url: String,
    /// [NOT NULL]
    pub sort_order: i32,
    /// 自定义分类名
    pub cat_name_alias: Option<String>,
}

impl MobileCatNav {
    /// 数据库表名。
    pub const TABLE: &'static str = "mobile_cat_nav";
}


/// pc分类楼层
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PcCatFloor {
    /// 分类抽屉id [NOT NULL]
    pub cat_floor_id: i32,
    /// 最终显示的分类名 [NOT NULL]
    pub cat_floor_name: String,
    /// 【JSON】分类ID：[category_id] [NOT NULL]
    pub category_ids: String,
    /// 【JSON】分类名称：[category_name] [NOT NULL]
    pub category_names: String,
    /// 分类ICO图片 [NOT NULL]
    pub floor_ico: String,
    /// 是否热门分类 [NOT NULL]
    pub hot_cat: String,
    /// 是否显示 [NOT NULL]
    pub is_show: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 分类ICO图标 [NOT NULL]
    pub floor_ico_font: String,
    /// 【JSON】推荐的品牌 [NOT NULL]
    pub brand_ids: String,
}

impl PcCatFloor {
    /// 数据库表名。
    pub const TABLE: &'static str = "pc_cat_floor";
}


/// 上中下3个导航栏的显示配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PcNavigation {
    /// 导航配置自增id [NOT NULL]
    pub id: i32,
    /// 导航显示标题 [NOT NULL]
    pub title: String,
    /// 是否显示 [NOT NULL]
    pub is_show: i32,
    /// 页面显示顺序，数字越大越靠后 [NOT NULL]
    pub sort_order: i32,
    /// 导航链接页面是否在新窗口打开，1，是；其他，否 [NOT NULL]
    pub is_blank: i32,
    /// [NOT NULL]
    pub link: String,
    /// 1：导航条，2：顶部站点地图导航，3：底部快捷导航 [NOT NULL]
    pub r#type: i32,
    /// 父导航（需具体模板支持） [NOT NULL]
    pub parent_id: i32,
    /// 图标 [NOT NULL]
    pub icon: String,
}

impl PcNavigation {
    /// 数据库表名。
    pub const TABLE: &'static str = "pc_navigation";
}