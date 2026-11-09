//! 内容域实体（文章/微信直播/视频号）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `album`, `article`, `article_category`, `wechat_live`, `wechat_live_goods`, `wechat_live_order_rel`, `wechat_live_room_goods`, `wechat_live_sync_log`, `wechat_video_account`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 内置相册表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Album {
    /// [NOT NULL]
    pub id: i32,
    /// 图片地址 [NOT NULL]
    pub pic_url: String,
    /// 图片名称 [NOT NULL]
    pub pic_name: String,
    /// 图片类型：1 图片 2 图标 [NOT NULL]
    pub pic_type: i32,
    /// 添加时间
    pub add_time: Option<i32>,
}

impl Album {
    /// 数据库表名。
    pub const TABLE: &'static str = "album";
}


/// 文章内容
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Article {
    /// 自增ID号 [NOT NULL]
    pub article_id: i32,
    /// 文章题目 [NOT NULL]
    pub article_title: String,
    /// [NOT NULL]
    pub article_category_id: i32,
    /// 文章编码 [NOT NULL]
    pub article_sn: String,
    /// 缩略图 [NOT NULL]
    pub article_thumb: String,
    /// 文章标签 [NOT NULL]
    pub article_tag: String,
    /// 文章作者 [NOT NULL]
    pub article_author: String,
    /// --- [NOT NULL]
    pub article_type: i32,
    /// 文章内容 [NOT NULL]
    pub content: String,
    /// 文章描述
    pub description: Option<String>,
    /// 文章的关键字 [NOT NULL]
    pub keywords: String,
    /// 是否显示;1显示;0不显示
    pub is_show: Option<i32>,
    /// 文章添加时间 [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub is_hot: i32,
    /// 是否置顶 [NOT NULL]
    pub is_top: i32,
    /// 浏览次数 [NOT NULL]
    pub click_count: i32,
    /// 相关链接 [NOT NULL]
    pub link: String,
}

impl Article {
    /// 数据库表名。
    pub const TABLE: &'static str = "article";
}


/// 文章分类信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ArticleCategory {
    /// 自增ID [NOT NULL]
    pub article_category_id: i32,
    /// 分类名称 [NOT NULL]
    pub article_category_name: String,
    /// 分类编码 [NOT NULL]
    pub category_sn: String,
    /// 分类类型 1：普通分类，2:帮助分类 [NOT NULL]
    pub category_type: i32,
    /// 分类关键字 [NOT NULL]
    pub keywords: String,
    /// 分类描述 [NOT NULL]
    pub description: String,
    /// 分类显示顺序 [NOT NULL]
    pub sort_order: i32,
    /// 父节点id，取值于该表article_category_id字段 [NOT NULL]
    pub parent_id: i32,
}

impl ArticleCategory {
    /// 数据库表名。
    pub const TABLE: &'static str = "article_category";
}


/// 小程序直播
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatLive {
    /// [NOT NULL]
    pub wechat_live_id: i32,
    /// 直播标题 [NOT NULL]
    pub wechat_live_title: String,
    /// 直播内容 [NOT NULL]
    pub wechat_live_data: String,
    /// 活动范围 [NOT NULL]
    pub act_range: i32,
    /// 活动简介 [NOT NULL]
    pub act_range_ext: String,
    /// 房间号 [NOT NULL]
    pub room_id: i32,
    /// 分享图 [NOT NULL]
    pub share_img: String,
    /// 直播类型：0-手机直播，1-推流 [NOT NULL]
    pub live_type: i32,
    /// 是否关闭点赞：0-开启，1-关闭 [NOT NULL]
    pub close_like: i32,
    /// 是否关闭货架：0-开启，1-关闭 [NOT NULL]
    pub close_goods: i32,
    /// 是否关闭评论：0-开启，1-关闭 [NOT NULL]
    pub close_comment: i32,
    /// 是否关闭客服：0-开启，1-关闭 [NOT NULL]
    pub close_kf: i32,
    /// 是否关闭回放：0-开启，1-关闭 [NOT NULL]
    pub close_replay: i32,
    /// 是否开启官方收录：0-关闭，1-开启 [NOT NULL]
    pub is_feeds_public: i32,
    /// 官方收录封面 [NOT NULL]
    pub feeds_img: String,
    /// 封面图 [NOT NULL]
    pub cover_img: String,
    /// 主播ID [NOT NULL]
    pub anchor_id: String,
    /// 主播微信openid [NOT NULL]
    pub anchor_openid: String,
    /// 主持人姓名 [NOT NULL]
    pub anchor_name: String,
    /// 主持人图片 [NOT NULL]
    pub anchor_img: String,
    /// 创建者openid [NOT NULL]
    pub creator_openid: String,
    /// 直播状态 [NOT NULL]
    pub live_status: i32,
    /// 开始时间 [NOT NULL]
    pub start_time: i32,
    /// 结束时间 [NOT NULL]
    pub end_time: i32,
    /// 最后一次更新时间 [NOT NULL]
    pub last_update_time: i32,
    /// 产品数据 [NOT NULL]
    pub product_data: String,
    /// 缩略图 [NOT NULL]
    pub thumb_img: String,
    /// 直播编码 [NOT NULL]
    pub live_sn: String,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 副主播微信账号
    pub sub_anchor_wechat: Option<String>,
    /// 审核状态：0-未审核，1-审核中，2-审核通过，3-审核驳回 [NOT NULL]
    pub audit_status: i32,
    /// 创建时间
    pub create_time: Option<String>,
}

impl WechatLive {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_live";
}


/// 微信直播商品主表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatLiveGoods {
    /// 主键ID [NOT NULL]
    pub id: i64,
    /// 历史字段：直播间ID
    pub room_id: Option<i32>,
    /// 微信商品ID
    pub wechat_goods_id: Option<i32>,
    /// 系统商品ID
    pub product_id: Option<i64>,
    /// 系统SKU ID，0表示无规格 [NOT NULL]
    pub product_sku_id: i64,
    /// 商品名称
    pub goods_name: Option<String>,
    /// 商品封面图
    pub cover_img: Option<String>,
    /// 商品价格（分）
    pub price: Option<i64>,
    /// 价格类型：1-一口价，2-价格区间，3-显示折扣价
    pub price_type: Option<i32>,
    /// 商品小程序路径
    pub url: Option<String>,
    /// 商品状态：0-未审核，1-审核中，2-审核通过，3-审核驳回
    pub audit_status: Option<i32>,
    /// 历史字段：直播间商品排序
    pub sort: Option<i32>,
    /// 历史字段：是否显示：0-显示，1-隐藏
    pub is_show: Option<i32>,
    /// 创建时间
    pub create_time: Option<String>,
    /// 更新时间
    pub update_time: Option<String>,
    /// 店铺ID
    pub shop_id: Option<i32>,
}

impl WechatLiveGoods {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_live_goods";
}


/// 微信直播订单归因关系表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatLiveOrderRel {
    /// 主键ID [NOT NULL]
    pub id: i64,
    /// 订单ID [NOT NULL]
    pub order_id: i32,
    /// 订单项ID [NOT NULL]
    pub order_item_id: i32,
    /// 直播主键ID
    pub wechat_live_id: Option<i32>,
    /// 直播房间ID
    pub room_id: Option<i32>,
    /// 微信直播商品ID
    pub wechat_goods_id: Option<i32>,
    /// 系统商品ID
    pub product_id: Option<i32>,
    /// 分享人OpenId
    pub share_open_id: Option<String>,
    /// 观看人OpenId
    pub viewer_open_id: Option<String>,
    /// 直播场景值
    pub scene: Option<String>,
    /// 直播透传参数
    pub custom_params: Option<String>,
    /// 渠道子来源
    pub sub_source: Option<String>,
    /// 同步状态：0待同步，1成功，2失败，3跳过，4部分同步
    pub sync_status: Option<i32>,
    /// 最近同步时间
    pub last_sync_time: Option<i64>,
    /// 店铺ID
    pub shop_id: Option<i32>,
    /// 创建时间
    pub add_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
}

impl WechatLiveOrderRel {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_live_order_rel";
}


/// 微信直播间商品关联表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatLiveRoomGoods {
    /// 主键ID [NOT NULL]
    pub id: i64,
    /// 直播间ID [NOT NULL]
    pub room_id: i32,
    /// 直播商品主表ID [NOT NULL]
    pub wechat_live_goods_id: i64,
    /// 微信商品ID
    pub wechat_goods_id: Option<i32>,
    /// 直播间商品排序
    pub sort: Option<i32>,
    /// 是否显示：0-显示，1-隐藏
    pub is_show: Option<i32>,
    /// 创建时间
    pub create_time: Option<String>,
    /// 更新时间
    pub update_time: Option<String>,
    /// 店铺ID
    pub shop_id: Option<i32>,
}

impl WechatLiveRoomGoods {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_live_room_goods";
}


/// 微信直播同步日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatLiveSyncLog {
    /// 主键ID [NOT NULL]
    pub id: i64,
    /// 订单ID
    pub order_id: Option<i32>,
    /// 售后ID
    pub aftersale_id: Option<i32>,
    /// 退款ID
    pub refund_id: Option<i32>,
    /// 直播房间ID
    pub room_id: Option<i32>,
    /// 事件类型 [NOT NULL]
    pub event_type: String,
    /// 同步方向：OUTBOUND/INBOUND [NOT NULL]
    pub direction: String,
    /// 同步状态：0待同步，1成功，2失败，3跳过，4部分同步
    pub sync_status: Option<i32>,
    /// 幂等键
    pub idempotent_key: Option<String>,
    /// 请求数据
    pub request_data: Option<String>,
    /// 响应数据
    pub response_data: Option<String>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 重试次数
    pub retry_count: Option<i32>,
    /// 最近同步时间
    pub last_sync_time: Option<i64>,
    /// 下次重试时间
    pub next_retry_time: Option<i64>,
    /// 店铺ID
    pub shop_id: Option<i32>,
    /// 创建时间
    pub add_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
}

impl WechatLiveSyncLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_live_sync_log";
}


/// 视频号账号关联表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WechatVideoAccount {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 视频号ID [NOT NULL]
    pub finder_id: String,
    /// 关联时间（创建时间） [NOT NULL]
    pub add_time: i64,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
}

impl WechatVideoAccount {
    /// 数据库表名。
    pub const TABLE: &'static str = "wechat_video_account";
}