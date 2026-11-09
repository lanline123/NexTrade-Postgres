//! 权限与账号实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `admin_api_token`, `admin_log`, `admin_role`, `admin_user`, `admin_user_pos`, `authority`, `suppliers`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 管理员API Token表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminApiToken {
    /// [NOT NULL]
    pub id: i32,
    /// 关联管理员ID [NOT NULL]
    pub admin_id: i32,
    /// Token的SHA-256哈希 [NOT NULL]
    pub token_hash: String,
    /// Token前8位，用于展示标识 [NOT NULL]
    pub token_prefix: String,
    /// Token名称/备注 [NOT NULL]
    pub name: String,
    /// 创建时间戳 [NOT NULL]
    pub created_at: i64,
    /// 最后使用时间戳
    pub last_used_at: Option<i64>,
    /// 过期时间戳，NULL表示永不过期
    pub expires_at: Option<i64>,
    /// 是否有效：1 有效 0 已吊销 [NOT NULL]
    pub is_active: i32,
}

impl AdminApiToken {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_api_token";
}


/// 后台管理日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminLog {
    /// 自增ID号 [NOT NULL]
    pub log_id: i32,
    /// 写日志时间 [NOT NULL]
    pub log_time: i32,
    /// 该日志所记录的操作者id,同admin_user的user_id [NOT NULL]
    pub user_id: i32,
    /// 操作作用域：admin-平台，shop-店铺，vendor-供应商，legacy-历史未标注 [NOT NULL]
    pub scope_type: String,
    /// 作用域ID：平台/历史为0，店铺为shop_id，供应商为vendor_id [NOT NULL]
    pub scope_id: i32,
    /// 管理操作内容 [NOT NULL]
    pub log_info: String,
    /// 登录者登录IP [NOT NULL]
    pub ip_address: String,
    /// 关联的请求日志ID
    pub request_log_id: Option<i32>,
}

impl AdminLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_log";
}


/// 后台角色表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminRole {
    /// 角色ID [NOT NULL]
    pub role_id: i32,
    /// 名称 [NOT NULL]
    pub role_name: String,
    /// 系统角色稳定标识(如 pos_cashier)
    pub role_code: Option<String>,
    /// 描述
    pub role_desc: Option<String>,
    /// [JSON]权限 [NOT NULL]
    pub authority_list: String,
    /// admin（平台）/ shop（店铺）/ vendor（供应商）
    pub admin_type: Option<String>,
    /// 商户id [NOT NULL]
    pub merchant_id: i32,
    /// [NOT NULL]
    pub shop_id: i32,
    /// 供应商ID [NOT NULL]
    pub vendor_id: i32,
    /// 是否系统内置角色 0-否 1-是 [NOT NULL]
    pub is_system: i32,
}

impl AdminRole {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_role";
}


/// 后台用户表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminUser {
    /// [NOT NULL]
    pub admin_id: i32,
    /// 管理员登录名 [NOT NULL]
    pub username: String,
    /// 管理员类型，admin:管理员，shop:店铺，suppliers:供应商，vendor:供货商 [NOT NULL]
    pub admin_type: String,
    /// 手机号
    pub mobile: Option<String>,
    /// 头像
    pub avatar: Option<String>,
    /// 管理员登录密码 [NOT NULL]
    pub password: String,
    /// 管理员邮箱 [NOT NULL]
    pub email: String,
    /// 管理员添加时间 [NOT NULL]
    pub add_time: i32,
    /// [JSON]管理员管理权限列表 [NOT NULL]
    pub auth_list: String,
    /// 绑定的会员id [NOT NULL]
    pub user_id: i32,
    /// 供应商id [NOT NULL]
    pub suppliers_id: i32,
    /// 角色id [NOT NULL]
    pub role_id: i32,
    /// 商户id [NOT NULL]
    pub merchant_id: i32,
    /// 父管理员id，表明当前为子管理员 [NOT NULL]
    pub parent_id: i32,
    /// [JSON]店铺存的标签
    pub menu_tag: Option<String>,
    /// [JSON]订单导出存的标签
    pub order_export: Option<String>,
    /// [JSON]扩展信息
    pub extra: Option<String>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 是否停用：1 是 0 否 [NOT NULL]
    pub is_using: i32,
    /// 初始密码 [NOT NULL]
    pub initial_password: String,
    /// 是否删除：1 是 0 否 [NOT NULL]
    pub is_deleted: i32,
}

impl AdminUser {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_user";
}


/// 收银员工列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminUserPos {
    /// [NOT NULL]
    pub id: i32,
    /// 管理员id [NOT NULL]
    pub admin_id: i32,
    /// 收银关联门店id [NOT NULL]
    pub shop_id: i32,
    /// 员工名称 [NOT NULL]
    pub username: String,
    /// 邮箱 [NOT NULL]
    pub email: String,
    /// 头像
    pub avatar: Option<String>,
    /// 员工权限[json]
    pub auth_list: Option<String>,
    /// 是否停用：1 是 0 否 [NOT NULL]
    pub is_using: i32,
    /// 创建时间
    pub add_time: Option<i32>,
    /// 权限组id [NOT NULL]
    pub role_id: i32,
    /// 上班开始时间（交班使用）
    pub work_on_time: Option<i32>,
}

impl AdminUserPos {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_user_pos";
}


/// 商城后台权限表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Authority {
    /// 自增id号 [NOT NULL]
    pub authority_id: i32,
    /// 权限编号 [NOT NULL]
    pub authority_sn: String,
    /// 权限名称 [NOT NULL]
    pub authority_name: String,
    /// 该权限的父类ID [NOT NULL]
    pub parent_id: i32,
    /// [NOT NULL]
    pub sort_order: i32,
    /// 是否在目录显示 1显示; 0不显示 [NOT NULL]
    pub is_show: i32,
    /// JSON:子权限[{authName|authSn}]
    pub child_auth: Option<String>,
    /// 路由链接 [NOT NULL]
    pub route_link: String,
    /// \r\nICO图标 [NOT NULL]
    pub authority_ico: String,
    /// 是否系统目录，是的话只能有限编辑或隐藏，不能删除 [NOT NULL]
    pub is_system: i32,
    /// admin管理后台使用shop店铺使用 [NOT NULL]
    pub admin_type: String,
}

impl Authority {
    /// 数据库表名。
    pub const TABLE: &'static str = "authority";
}


/// 供应商
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Suppliers {
    /// [NOT NULL]
    pub suppliers_id: i32,
    /// 供应商名称
    pub suppliers_name: Option<String>,
    /// 简介
    pub suppliers_desc: Option<String>,
    /// 1启用 0 停用 [NOT NULL]
    pub is_check: i32,
    /// 国家 [NOT NULL]
    pub country: i32,
    /// 省份 [NOT NULL]
    pub province: i32,
    /// 城市 [NOT NULL]
    pub city: i32,
    /// 地区 [NOT NULL]
    pub district: i32,
    /// 联系人
    pub contact_name: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 联系地址
    pub contact_address: Option<String>,
    /// 是否显示
    pub is_show: Option<i32>,
    /// 店铺id
    pub shop_id: Option<i32>,
}

impl Suppliers {
    /// 数据库表名。
    pub const TABLE: &'static str = "suppliers";
}