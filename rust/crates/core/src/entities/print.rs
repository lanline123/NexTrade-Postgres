//! 打印域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `print`, `print_log`, `print_pos_device`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 打印机表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Print {
    /// 自增ID，打印机ID [NOT NULL]
    pub print_id: i32,
    /// 打印机名称
    pub print_name: Option<String>,
    /// 打印机SN
    pub print_sn: Option<String>,
    /// 打印机key
    pub print_key: Option<String>,
    /// 第三方平台对接账号
    pub third_account: Option<String>,
    /// 第三方平台key
    pub third_key: Option<String>,
    /// 打印联数
    pub print_number: Option<i32>,
    /// 第三方平台 1=飞鹅云 2=商米云
    pub platform: Option<i32>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 状态 1开启 2关闭
    pub status: Option<i32>,
    /// 打印机创建时间
    pub add_time: Option<i32>,
    /// 打印机修改时间
    pub update_time: Option<i32>,
    /// 打印机删除时间
    pub delete_time: Option<i32>,
    /// 订单支付自动打印 1开启 2关闭
    pub auto_print: Option<i32>,
    /// 语音播报开关：1=开启，2=关闭 [NOT NULL]
    pub voice_enabled: i32,
    /// 语音播报内容：0=订单号+金额，1=仅金额 [NOT NULL]
    pub voice_content_type: i32,
    /// 店铺默认打印机 1是0否 [NOT NULL]
    pub is_default: i32,
}

impl Print {
    /// 数据库表名。
    pub const TABLE: &'static str = "print";
}


/// 打印任务日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PrintLog {
    /// 打印日志ID [NOT NULL]
    pub log_id: i64,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 打印机id [NOT NULL]
    pub print_id: i32,
    /// 平台 1=飞鹅云 2=商米云 [NOT NULL]
    pub platform: i32,
    /// 订单id [NOT NULL]
    pub order_id: i32,
    /// 打印机SN快照
    pub print_sn: Option<String>,
    /// 触发方式 1=自动 2=手动补打 [NOT NULL]
    pub trigger_type: i32,
    /// 状态 0=待推送 1=推送成功 2=打印成功 3=推送失败 4=打印机离线 [NOT NULL]
    pub status: i32,
    /// 平台错误码
    pub error_code: Option<String>,
    /// 错误消息
    pub error_msg: Option<String>,
    /// 平台任务号（商米回调匹配键）
    pub task_no: Option<String>,
    /// 推送API原始响应（截断存储）
    pub push_response: Option<String>,
    /// 回调原始报文（商米云）
    pub callback_payload: Option<String>,
    /// MQ重试次数 [NOT NULL]
    pub retry_count: i32,
    /// 推送时间（Unix时间戳，秒）
    pub pushed_time: Option<i32>,
    /// 打印成功时间（Unix时间戳，秒）
    pub printed_time: Option<i32>,
    /// 创建时间（Unix时间戳，秒）
    pub add_time: Option<i32>,
    /// 更新时间（Unix时间戳，秒）
    pub update_time: Option<i32>,
}

impl PrintLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "print_log";
}


/// 打印机收银台设备绑定（单设备）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PrintPosDevice {
    /// [NOT NULL]
    pub id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 打印机ID [NOT NULL]
    pub print_id: i32,
    /// 收银台设备码 [NOT NULL]
    pub device_code: String,
    /// [NOT NULL]
    pub add_time: i64,
}

impl PrintPosDevice {
    /// 数据库表名。
    pub const TABLE: &'static str = "print_pos_device";
}