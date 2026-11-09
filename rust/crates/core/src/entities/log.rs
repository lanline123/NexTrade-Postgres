//! 日志与统计实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `access_log`, `distribution_log`, `mail_log`, `request_log`, `sms_log`, `statistics_base`, `statistics_log`, `unipush_log`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 访问日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AccessLog {
    /// [NOT NULL]
    pub id: i32,
    /// 访问时间 [NOT NULL]
    pub access_time: i32,
    /// ip地址 [NOT NULL]
    pub ip_address: String,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 参数id [NOT NULL]
    pub params_id: i32,
    /// 控制器
    pub controller: Option<String>,
    /// 访问域名
    pub access_domain: Option<String>,
    /// 访问地址
    pub access_path: Option<String>,
}

impl AccessLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "access_log";
}


/// 分佣日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct DistributionLog {
    /// [NOT NULL]
    pub log_id: i32,
    /// 订单编号 [NOT NULL]
    pub order_id: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 级别 [NOT NULL]
    pub level: i32,
    /// 会员id [NOT NULL]
    pub user_id: i32,
    /// 金额 [NOT NULL]
    pub money: Decimal,
    /// 积分 [NOT NULL]
    pub point: i32,
    /// [NOT NULL]
    pub r#type: i32,
    /// 备注 [NOT NULL]
    pub log_desc: String,
    /// 是否撤回 [NOT NULL]
    pub is_returned: i32,
}

impl DistributionLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "distribution_log";
}


/// 邮件发送日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MailLog {
    /// [NOT NULL]
    pub mail_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 邮箱地址 [NOT NULL]
    pub email: String,
    /// 发送时间 [NOT NULL]
    pub send_time: i32,
    /// 状态 [NOT NULL]
    pub status: i32,
    /// 内容 [NOT NULL]
    pub content: String,
}

impl MailLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "mail_log";
}


/// request_log
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RequestLog {
    /// 主键 [NOT NULL]
    pub request_log_id: i32,
    /// 地址 [NOT NULL]
    pub url: String,
    /// 请求类型 [NOT NULL]
    pub http_method: String,
    /// 类名 + 方法名 [NOT NULL]
    pub class_method: String,
    /// 请求参数
    pub request_params: Option<String>,
    /// 操作时间
    pub add_time: Option<String>,
    /// 后台用户名称
    pub admin_username: Option<String>,
    /// 后台用户id
    pub admin_id: Option<i32>,
    /// ip地址
    pub admin_ip: Option<String>,
    /// 平台信息
    pub platform: Option<String>,
}

impl RequestLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "request_log";
}


/// 短信日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SmsLog {
    /// [NOT NULL]
    pub sms_id: i32,
    /// 会员id [NOT NULL]
    pub user_id: i32,
    /// 手机号 [NOT NULL]
    pub mobile: String,
    /// 发送时间 [NOT NULL]
    pub send_time: i32,
    /// 内容 [NOT NULL]
    pub content: String,
    /// 状态 [NOT NULL]
    pub status: i32,
    /// 批量发送手机号 [NOT NULL]
    pub batch_mobile: String,
    /// 错误内容 [NOT NULL]
    pub error_content: String,
}

impl SmsLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "sms_log";
}


/// 统计基础表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StatisticsBase {
    /// [NOT NULL]
    pub id: i32,
    /// 日期
    pub date: Option<String>,
    /// 点击数
    pub click_count: Option<i32>,
    /// 店铺ID
    pub shop_id: Option<i32>,
    /// 访客数
    pub visitor_count: Option<i32>,
}

impl StatisticsBase {
    /// 数据库表名。
    pub const TABLE: &'static str = "statistics_base";
}


/// 统计明细日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StatisticsLog {
    /// [NOT NULL]
    pub id: i32,
    pub access_time: Option<i32>,
    pub shop_id: Option<i32>,
    pub product_id: Option<i32>,
    /// [NOT NULL]
    pub shop_category_id: i32,
    pub user: Option<String>,
}

impl StatisticsLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "statistics_log";
}


/// unipush推送日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UnipushLog {
    /// [NOT NULL]
    pub unipush_log_id: i32,
    /// 类型 [NOT NULL]
    pub r#type: String,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 内容 [NOT NULL]
    pub data: String,
    /// 返回内容 [NOT NULL]
    pub respond_data: String,
    /// 是否成功 [NOT NULL]
    pub is_success: i32,
    /// 用户id
    pub user_ids: Option<i32>,
    /// 消息标题
    pub message_title: Option<String>,
    /// 消息内容
    pub message_content: Option<String>,
    /// 消息发送类型
    pub send_user_type: Option<i32>,
}

impl UnipushLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "unipush_log";
}