//! 代客下单接单大厅（二期，我方新增，论坛帖 547750 余项）模型。
//!
//! 一期（跨店履约 + 三方分成）在下单时即按地址选定履约门店；二期在
//! 同一 `proxy_order_ext` 记录上加接单状态机：代客单以「接单大厅」
//! 方式提交后先进待接单池，同体系营业门店抢单、或由下单店/平台
//! 派单，接单后履约店落 `fulfill_shop_id`，结算三方分成沿一期口径。
//!
//! 状态（`proxy_order_ext.hall_status`）：
//! 0 非大厅单（一期直选/同店，恒定）/ 1 待接单 / 2 已接单 /
//! 3 履约店已备货 / 4 已关闭（订单取消联动）。
//! 接单方式（`assign_type`）：0 无 / 1 抢单 / 2 派单 / 3 超时兜底改派。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 接单大厅消息类型（`admin_msg.msg_type`；58 买家催菜、59 平台公告已占用）。
pub const HALL_MSG_TYPE: i32 = 60;

/// hall_status：非大厅单（一期直选模式）。
pub const HALL_STATUS_NONE: i32 = 0;
/// hall_status：待接单（在池中）。
pub const HALL_STATUS_PENDING: i32 = 1;
/// hall_status：已接单（履约店已落 fulfill_shop_id）。
pub const HALL_STATUS_ACCEPTED: i32 = 2;
/// hall_status：履约店已备货（交接完成标记）。
pub const HALL_STATUS_READY: i32 = 3;
/// hall_status：已关闭（订单取消联动关闭）。
pub const HALL_STATUS_CLOSED: i32 = 4;

/// assign_type：抢单。
pub const ASSIGN_TYPE_GRAB: i32 = 1;
/// assign_type：派单。
pub const ASSIGN_TYPE_ASSIGN: i32 = 2;
/// assign_type：超时兜底改派（自动落下单店自履约）。
pub const ASSIGN_TYPE_FALLBACK: i32 = 3;

/// 大厅列表行（待接单池 / 我的接单共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HallOrderVO {
    /// 订单 id。
    pub order_id: i32,
    /// 订单号。
    pub order_sn: String,
    /// 订单状态（`order.order_status`）。
    pub order_status: i32,
    /// 支付状态（`order.pay_status`）。
    pub pay_status: i32,
    /// 下单店 id（代下单店）。
    pub proxy_shop_id: i32,
    /// 下单店名称。
    pub proxy_shop_title: Option<String>,
    /// 履约门店 id（待接单为 0）。
    pub fulfill_shop_id: i32,
    /// 履约门店名称。
    pub fulfill_shop_title: Option<String>,
    /// 收货人。
    pub consignee: String,
    /// 收货地址全文。
    pub address: String,
    /// 收货手机号。
    pub mobile: String,
    /// 应付金额（订单 paid/unpaid 口径：取 total_amount 展示）。
    pub total_amount: Decimal,
    /// 商品件数（明细数量合计）。
    pub item_count: i32,
    /// 商品摘要（首件商品名，多件带「等 N 件」由前端拼装）。
    pub first_product_name: Option<String>,
    /// 与当前门店的距离（米；无坐标为 None）。
    pub distance_meters: Option<i64>,
    /// 接单状态（见模块头）。
    pub hall_status: i32,
    /// 接单截止时间（epoch 秒；0 = 不限时）。
    pub hall_deadline: i32,
    /// 接单时间（epoch 秒）。
    pub accept_time: Option<i32>,
    /// 接单方式（见模块头）。
    pub assign_type: i32,
    /// 改派次数（拒单回池次数）。
    pub reassign_count: i32,
    /// 下单时间（epoch 秒）。
    pub add_time: i32,
}

/// 抢单/备货/关闭等单订单动作入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HallOrderActionParam {
    /// 订单 id。
    pub order_id: Option<i32>,
}

/// 派单入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HallAssignParam {
    /// 订单 id。
    pub order_id: Option<i32>,
    /// 目标履约门店 id（必须与下单店同体系且营业）。
    pub shop_id: Option<i32>,
}

/// 拒单（退回大厅）入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HallRejectParam {
    /// 订单 id。
    pub order_id: Option<i32>,
    /// 拒单原因（可选，截断 255 字）。
    pub reason: Option<String>,
}

/// 动作结果（前端据此刷新列表并提示）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HallActionVO {
    /// 订单 id。
    pub order_id: i32,
    /// 动作后的接单状态。
    pub hall_status: i32,
    /// 动作后的履约门店 id。
    pub fulfill_shop_id: i32,
}
