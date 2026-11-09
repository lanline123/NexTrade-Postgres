//! 客服 IM 域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `admin_msg`, `im_config`, `im_conversation`, `im_message`, `im_servant`, `shop_kefu_config`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 后台消息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminMsg {
    /// 自增id [NOT NULL]
    pub msg_id: i32,
    /// 消息类型：11新订单、12已付款订单、13订单完成、21商品库存预警、22商品无货、23商品下架、24商品审核通知、25商品咨询、26商品询价、31订单取消、32售后申请、33提现申请、34发票资质审核、35发票申请、36售后退款、37线下支付审核、41店铺入驻申请、42店铺资质修改、43店铺违规、44保证金预警、51系统消息、52待办任务、53意见反馈（历史兼容）、54发货提醒、55建议、56投诉、57其它反馈；15/16订单问题/订单咨询（历史兼容，不展示分类） [NOT NULL]
    pub msg_type: i32,
    /// 消息发送时间 [NOT NULL]
    pub send_time: i32,
    /// [NOT NULL]
    pub is_readed: i32,
    /// 消息是否发向指定店铺 [NOT NULL]
    pub shop_id: i32,
    /// 消息是否发向指定管理员 [NOT NULL]
    pub admin_id: i32,
    /// 订单消息 [NOT NULL]
    pub order_id: i32,
    /// 订单商品消息 [NOT NULL]
    pub product_id: i32,
    /// 消息主题 [NOT NULL]
    pub title: String,
    /// 消息内容 [NOT NULL]
    pub content: String,
    /// 消息链接
    pub msg_link: Option<String>,
    /// [json]跳转数据信息
    pub related_data: Option<String>,
    /// 是否参与语音播报: 1-播报, 0-不播报 [NOT NULL]
    pub is_broadcast: i32,
    /// 供应商id
    pub vendor_id: Option<i32>,
}

impl AdminMsg {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_msg";
}


/// 客服配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ImConfig {
    /// [NOT NULL]
    pub id: i32,
    /// 配置名称 [NOT NULL]
    pub code: String,
    /// 配置信息[json]
    pub data: Option<String>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl ImConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "im_config";
}


/// im_conversation
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ImConversation {
    /// [NOT NULL]
    pub id: i32,
    pub user_id: Option<i32>,
    pub last_servant_id: Option<i32>,
    pub add_time: Option<i32>,
    pub shop_id: Option<i32>,
    pub user_from: Option<String>,
    /// 会话状态0待接入1会话中2已结束
    pub status: Option<i32>,
    pub last_update_time: Option<i32>,
    /// 0未删除1已删除
    pub is_delete: Option<i32>,
    /// 会话备注 [NOT NULL]
    pub remark: String,
    /// 会话总结 [NOT NULL]
    pub summary: String,
}

impl ImConversation {
    /// 数据库表名。
    pub const TABLE: &'static str = "im_conversation";
}


/// 会话消息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ImMessage {
    /// [NOT NULL]
    pub id: i32,
    /// 来自会话
    pub conversation_id: Option<i32>,
    /// 内容
    pub content: Option<String>,
    /// text文本 image图片 custom自定义 sound语音 video视频 file文件
    pub message_type: Option<String>,
    /// 类型1用户发的，2客服发的，3系统
    pub r#type: Option<i32>,
    /// 用户id
    pub user_id: Option<i32>,
    /// 客服id
    pub servant_id: Option<i32>,
    /// 发送时间
    pub send_time: Option<i32>,
    /// 状态，1正常，2删除，3撤回
    pub status: Option<i32>,
    /// 消息扩展信息
    pub extend: Option<String>,
    /// 推送状态0未推送，1已推送
    pub push_status: Option<i32>,
    /// 是否已读0未读，1已读
    pub is_read: Option<i32>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 来源
    pub user_from: Option<String>,
}

impl ImMessage {
    /// 数据库表名。
    pub const TABLE: &'static str = "im_message";
}


/// im_servant
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ImServant {
    /// [NOT NULL]
    pub id: i32,
    /// 客服id
    pub servant_id: Option<i32>,
    /// 最后更新时间
    pub last_update_time: Option<i32>,
    /// 进入时间
    pub add_time: Option<i32>,
    /// 状态1在线2忙3离开
    pub status: Option<i32>,
    /// 所属店铺
    pub shop_id: Option<i32>,
    /// 当前进行的会话数量
    pub conversation_num: Option<i32>,
}

impl ImServant {
    /// 数据库表名。
    pub const TABLE: &'static str = "im_servant";
}


/// 店铺客服接入配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopKefuConfig {
    /// [NOT NULL]
    pub id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 客服选择:0无客服 1腾讯云客服 2企业微信客服 3自定义客服 4TigChart客服 [NOT NULL]
    pub kefu_type: i32,
    /// 腾讯云客服打开方式:0链接 1弹窗 [NOT NULL]
    pub kefu_yzf_type: i32,
    /// 腾讯云客服sign [NOT NULL]
    pub kefu_yzf_sign: String,
    /// 企业微信客服ID [NOT NULL]
    pub kefu_workwx_id: String,
    /// 企业微信企业ID [NOT NULL]
    pub corp_id: String,
    /// 客服链接 [NOT NULL]
    pub kefu_code: String,
    /// 客服链接打开方式:0链接 1弹窗 [NOT NULL]
    pub kefu_code_blank: i32,
    /// WhatsApp客服号码 [NOT NULL]
    pub kefu_whatsapp_phone: String,
    /// 客服热线电话 [NOT NULL]
    pub kefu_phone: String,
    /// 服务时间 [NOT NULL]
    pub kefu_time: String,
    /// 接入方式:0全部自动接入 1全部手动接入 [NOT NULL]
    pub auto_access_mode: i32,
    /// 回头客接入:0关闭 1优先接入上次客服 [NOT NULL]
    pub return_customer_access: i32,
    /// 创建时间
    pub create_time: Option<i32>,
    /// 更新时间
    pub modify_time: Option<i32>,
}

impl ShopKefuConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_kefu_config";
}