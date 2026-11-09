//! 营销域实体（优惠券/秒杀/拼团/积分/抽奖/礼品卡/买家秀）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `bargain`, `bargain_group`, `bargain_log`, `buyer_show`, `buyer_show_comment`, `buyer_show_favorite`, `buyer_show_gallery`, `buyer_show_group`, `buyer_show_like`, `buyer_show_video`, `comment_like`, `coupon`, `coupon_issue_batch`, `coupon_issue_log`, `gift_card_groups`, `gift_card_order`, `gift_card_products`, `gift_cards`, `groupon`, `groupon_item`, `groupon_promotion`, `groupon_promotion_detail`, `groupon_promotion_item`, `groupon_promotion_record`, `groupon_promotion_shops`, `lottery_activity`, `lottery_prize`, `lottery_record`, `lottery_user_stat`, `points_exchange`, `product_gift`, `product_promotion`, `promotion`, `recharge_setting`, `seckill`, `seckill_item`, `short_url`, `sign`, `sign_in_setting`, `time_discount`, `time_discount_item`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 砍一砍
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Bargain {
    /// [NOT NULL]
    pub bargain_id: i32,
    /// 砍一砍活动名称 [NOT NULL]
    pub bargain_name: String,
    /// 砍一砍活动图片 [NOT NULL]
    pub bargain_pic: String,
    /// 砍一砍活动开始时间 [NOT NULL]
    pub start_time: i32,
    /// 砍一砍活动结束时间 [NOT NULL]
    pub end_time: i32,
    /// 砍到多少钱 [NOT NULL]
    pub cut_price_limit: Decimal,
    /// 邀请多少人砍 [NOT NULL]
    pub cut_num_limit: i32,
    /// 首次砍价范围 [NOT NULL]
    pub first_cut_range: String,
    /// 砍价范围 [NOT NULL]
    pub cut_range: String,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 商品sku_id [NOT NULL]
    pub sku_id: i32,
    /// 是否展示 [NOT NULL]
    pub is_show: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 状态：1:进行中，2:成功 [NOT NULL]
    pub success_type: i32,
    /// 所属店铺 [NOT NULL]
    pub shop_id: i32,
}

impl Bargain {
    /// 数据库表名。
    pub const TABLE: &'static str = "bargain";
}


/// 砍一砍活动组
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BargainGroup {
    /// [NOT NULL]
    pub group_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 砍一砍主键id [NOT NULL]
    pub bargain_id: i32,
    /// 砍一砍活动名 [NOT NULL]
    pub bargain_name: String,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 商品规格id [NOT NULL]
    pub sku_id: i32,
    /// 状态 [NOT NULL]
    pub status: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 商品价格 [NOT NULL]
    pub price: Decimal,
    /// 订单id [NOT NULL]
    pub order_id: i32,
}

impl BargainGroup {
    /// 数据库表名。
    pub const TABLE: &'static str = "bargain_group";
}


/// 砍一砍日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BargainLog {
    /// [NOT NULL]
    pub log_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 砍一砍id [NOT NULL]
    pub bargain_id: i32,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 商品规格id [NOT NULL]
    pub sku_id: i32,
    /// [NOT NULL]
    pub cut_price: Decimal,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 砍一砍组id [NOT NULL]
    pub group_id: i32,
}

impl BargainLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "bargain_log";
}


/// 买家秀
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShow {
    /// 自增ID，买家秀ID [NOT NULL]
    pub buyer_show_id: i32,
    /// 关联商品数据 （json） [NOT NULL]
    pub range_data: String,
    /// 用户id（非真实用户为0） [NOT NULL]
    pub user_id: i32,
    /// 买家头像 [NOT NULL]
    pub buyer_photo: String,
    /// 买家名称 [NOT NULL]
    pub buyer_name: String,
    /// 文字内容
    pub content: Option<String>,
    /// 所属分组（json）
    pub group_data: Option<String>,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
    /// 排序
    pub sort_order: Option<i32>,
    /// 状态（0：待审核，1：通过，2：拒绝） [NOT NULL]
    pub status: i32,
    /// 是否公开（0：隐藏，1：公开） [NOT NULL]
    pub is_show: i32,
}

impl BuyerShow {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show";
}


/// 买家秀评论
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShowComment {
    /// 自增ID，买家秀评论ID [NOT NULL]
    pub buyer_show_comment_id: i32,
    /// 上级评论id [NOT NULL]
    pub parent_id: i32,
    /// 买家秀ID [NOT NULL]
    pub buyer_show_id: i32,
    /// 状态（0：待审核，1：通过，2：拒绝） [NOT NULL]
    pub status: i32,
    /// 是否置顶（0：否，1：是） [NOT NULL]
    pub is_top: i32,
    /// 是否显示（0：隐藏，1：公开） [NOT NULL]
    pub is_show: i32,
    /// 用户id（非真实用户为0） [NOT NULL]
    pub user_id: i32,
    /// 用户名称 [NOT NULL]
    pub user_name: String,
    /// 用户头像 [NOT NULL]
    pub user_photo: String,
    /// 评论内容
    pub content: Option<String>,
    /// 发布时间 [NOT NULL]
    pub add_time: i64,
    /// 图片评论
    pub comment_pic: Option<String>,
}

impl BuyerShowComment {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show_comment";
}


/// 买家秀收藏表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShowFavorite {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 买家秀ID [NOT NULL]
    pub buyer_show_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 收藏时间 [NOT NULL]
    pub add_time: i64,
}

impl BuyerShowFavorite {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show_favorite";
}


/// 买家秀相册
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShowGallery {
    /// 相册ID [NOT NULL]
    pub pic_id: i32,
    /// 买家秀id [NOT NULL]
    pub buyer_show_id: i32,
    /// 实际图片url [NOT NULL]
    pub pic_url: String,
    /// 图片说明信息 [NOT NULL]
    pub pic_desc: String,
    /// 微缩图片url [NOT NULL]
    pub pic_thumb: String,
    /// 原图url [NOT NULL]
    pub pic_original: String,
    /// 高清图片url [NOT NULL]
    pub pic_large: String,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
}

impl BuyerShowGallery {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show_gallery";
}


/// 买家秀分组
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShowGroup {
    /// 自增ID，买家秀分组ID [NOT NULL]
    pub buyer_show_group_id: i32,
    /// 分组名称 [NOT NULL]
    pub group_name: String,
    /// 商品分组id（json） [NOT NULL]
    pub product_group_id: String,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl BuyerShowGroup {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show_group";
}


/// 买家秀点赞表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShowLike {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 买家秀ID [NOT NULL]
    pub buyer_show_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 发布时间 [NOT NULL]
    pub add_time: i64,
}

impl BuyerShowLike {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show_like";
}


/// 买家秀视频
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct BuyerShowVideo {
    /// 主键id [NOT NULL]
    pub id: i32,
    /// 视频ID [NOT NULL]
    pub video_id: i32,
    /// 买家秀d [NOT NULL]
    pub buyer_show_id: i32,
    /// 视频url [NOT NULL]
    pub video_url: String,
    /// 视频封面 [NOT NULL]
    pub video_cover: String,
    /// 视频格式后缀 [NOT NULL]
    pub format: String,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
}

impl BuyerShowVideo {
    /// 数据库表名。
    pub const TABLE: &'static str = "buyer_show_video";
}


/// 评论点赞表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CommentLike {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 评论ID [NOT NULL]
    pub comment_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 发布时间 [NOT NULL]
    pub add_time: i64,
}

impl CommentLike {
    /// 数据库表名。
    pub const TABLE: &'static str = "comment_like";
}


///  红包类型表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Coupon {
    /// 优惠券id [NOT NULL]
    pub coupon_id: i32,
    /// 优惠券名称 [NOT NULL]
    pub coupon_name: String,
    /// 优惠券金额 [NOT NULL]
    pub coupon_money: Decimal,
    /// 优惠券折扣 [NOT NULL]
    pub coupon_discount: Decimal,
    /// 优惠券描述 [NOT NULL]
    pub coupon_desc: String,
    /// 优惠券类型，1：满减券，2：折扣券 [NOT NULL]
    pub coupon_type: i32,
    /// [NOT NULL]
    pub send_range: i32,
    /// [json]使用范围相关信息 [NOT NULL]
    pub send_range_data: String,
    /// 最少订单金额 [NOT NULL]
    pub min_order_amount: Decimal,
    /// 优惠券可领取的开始时间 (已废弃) [NOT NULL]
    pub send_start_date: i32,
    /// 优惠券可领取的结束时间 (已废弃) [NOT NULL]
    pub send_end_date: i32,
    /// 优惠券使用时间方式，0：按领取后多少天内可用，1：按固定时间范围可用 [NOT NULL]
    pub send_type: i32,
    /// 优惠券多少天过期 [NOT NULL]
    pub use_day: i32,
    /// 优惠券可以使用的开始时间 [NOT NULL]
    pub use_start_date: i32,
    /// 优惠券可以使用的结束时间 [NOT NULL]
    pub use_end_date: i32,
    /// 是否展示 [NOT NULL]
    pub is_show: i32,
    /// [NOT NULL]
    pub is_global: i32,
    /// 是否为新人专享优惠券 [NOT NULL]
    pub is_new_user: i32,
    /// [NOT NULL]
    pub enabled_click_get: i32,
    /// [JSON]仅限会员等级
    pub limit_user_rank: Option<String>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 是否删除
    pub is_delete: Option<i32>,
    /// 限制领取数量
    pub limit_num: Option<i32>,
    /// 延迟天数：0 为立即生效 [NOT NULL]
    pub delay_day: i32,
    /// 发放总量 [NOT NULL]
    pub send_num: i32,
    /// 优惠金额上限 [NOT NULL]
    pub max_order_amount: Decimal,
    /// 优惠单位：1 元  2 件 [NOT NULL]
    pub coupon_unit: i32,
    /// 满减类型：1 满多少 2 无门槛 [NOT NULL]
    pub reduce_type: i32,
    /// 添加时间
    pub add_time: Option<i32>,
}

impl Coupon {
    /// 数据库表名。
    pub const TABLE: &'static str = "coupon";
}


/// 优惠券发放批次表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CouponIssueBatch {
    /// 批次ID [NOT NULL]
    pub id: i64,
    /// 优惠券活动ID [NOT NULL]
    pub coupon_id: i32,
    /// 发放用户数 [NOT NULL]
    pub user_count: i32,
    /// 发放张数 [NOT NULL]
    pub coupon_count: i32,
    /// 发放时间（时间戳） [NOT NULL]
    pub issue_time: i64,
    /// 发放来源：0-用户领取，1-抽奖中奖，2-手动发放 [NOT NULL]
    pub source_type: i32,
}

impl CouponIssueBatch {
    /// 数据库表名。
    pub const TABLE: &'static str = "coupon_issue_batch";
}


/// 优惠券发放日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CouponIssueLog {
    /// 主键ID [NOT NULL]
    pub id: i64,
    /// 发放批次ID [NOT NULL]
    pub batch_id: i64,
    /// 优惠券活动ID [NOT NULL]
    pub coupon_id: i32,
    /// 用户优惠券ID [NOT NULL]
    pub user_coupon_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 发放时间（时间戳） [NOT NULL]
    pub issue_time: i64,
    /// 发放来源：0-用户领取，1-抽奖中奖，2-手动发放 [NOT NULL]
    pub source_type: i32,
}

impl CouponIssueLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "coupon_issue_log";
}


/// 礼品卡表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GiftCardGroups {
    /// 卡组id [NOT NULL]
    pub card_groups_id: i32,
    /// 1-储值卡 2-兑换卡 [NOT NULL]
    pub group_type: i32,
    /// 1-电子卡 2-实体卡 [NOT NULL]
    pub card_type: i32,
    /// 卡组名称 [NOT NULL]
    pub card_group_name: String,
    /// 卡图片URL
    pub image_url: Option<String>,
    /// 有效期类型：0=永久，1=天数，2=固定日期 [NOT NULL]
    pub validity_type: i32,
    /// 有效期天数（当 validity_type=1 时）
    pub validity_days: Option<i32>,
    /// 固定到期时间（当 validity_type=2 时）
    pub expire_time: Option<i32>,
    /// 储值卡面额(当group_type=1 时)
    pub face_amount: Option<Decimal>,
    /// 1-开启 0-关闭 [NOT NULL]
    pub status: i32,
    /// 创建时间
    pub add_time: Option<i32>,
    /// 更新时间
    pub update_time: Option<i32>,
    /// 备注
    pub remark: Option<String>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 可用库存数
    pub availability_stock: Option<i32>,
}

impl GiftCardGroups {
    /// 数据库表名。
    pub const TABLE: &'static str = "gift_card_groups";
}


/// 订单卡关联
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GiftCardOrder {
    /// 订单卡id [NOT NULL]
    pub order_card_id: i32,
    /// 订单id [NOT NULL]
    pub order_id: i32,
    /// 卡id [NOT NULL]
    pub card_id: i32,
}

impl GiftCardOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "gift_card_order";
}


/// 卡兑换商品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GiftCardProducts {
    /// 卡组商品id [NOT NULL]
    pub gift_product_id: i32,
    /// 卡组ID [NOT NULL]
    pub card_groups_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// SKU ID [NOT NULL]
    pub sku_id: i32,
}

impl GiftCardProducts {
    /// 数据库表名。
    pub const TABLE: &'static str = "gift_card_products";
}


/// 卡信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GiftCards {
    /// 卡id [NOT NULL]
    pub gift_cards_id: i32,
    /// 所属卡组ID [NOT NULL]
    pub card_groups_id: i32,
    /// 卡号 [NOT NULL]
    pub card_number: String,
    /// 卡密码 [NOT NULL]
    pub card_password: String,
    /// 礼品卡唯一识别码
    pub gift_card_sn: Option<String>,
    /// 1-可用 0-不可用
    pub is_available: Option<i32>,
    /// 0-未使用 1-已使用 2-已过期
    pub status: Option<i32>,
    /// 使用时间
    pub used_time: Option<i32>,
    /// 使用者ID
    pub user_id: Option<i32>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 过期时间 [NOT NULL]
    pub expired_time: i32,
    /// 已领取该卡的用户ID
    pub claimed_user_id: Option<i32>,
    /// 是否已领取，0=不可领取，1=可领取，2=已领取
    pub claimed_status: Option<i32>,
    /// 下单时间
    pub order_time: Option<i32>,
}

impl GiftCards {
    /// 数据库表名。
    pub const TABLE: &'static str = "gift_cards";
}


/// 商品拼团表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Groupon {
    /// [NOT NULL]
    pub groupon_id: i32,
    /// 拼团活动名称
    pub groupon_name: Option<String>,
    /// 活动开始时间
    pub start_time: Option<i32>,
    /// 活动结束时间
    pub end_time: Option<i32>,
    /// 商品id
    pub product_id: Option<i32>,
    /// 所属店铺
    pub shop_id: Option<i32>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 参团人数
    pub team_num: Option<i32>,
    /// 限购数量0不限购
    pub limit_num: Option<i32>,
    /// 有效期
    pub expiration_time: Option<i32>,
}

impl Groupon {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon";
}


/// 商品拼团sku明细
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GrouponItem {
    /// [NOT NULL]
    pub groupon_item_id: i32,
    pub groupon_id: Option<i32>,
    pub product_sku_id: Option<i32>,
    /// 拼团价格
    pub price: Option<Decimal>,
    /// 商品id
    pub product_id: Option<i32>,
    /// 开始时间
    pub start_time: Option<i32>,
    /// 结束时间
    pub end_time: Option<i32>,
}

impl GrouponItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon_item";
}


/// 拼团活动
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GrouponPromotion {
    /// 拼团 ID [NOT NULL]
    pub groupon_promotion_id: i64,
    /// 活动名称 [NOT NULL]
    pub name: String,
    /// 开始时间 [NOT NULL]
    pub start_time: i64,
    /// 结束时间 [NOT NULL]
    pub end_time: i64,
    /// 拼团类型；1-普通团，2-阶梯团 [NOT NULL]
    pub group_type: i32,
    /// 成团有效时间 [NOT NULL]
    pub expiration_time: i64,
    /// h5活动图片
    pub groupon_image: Option<String>,
    /// pc活动图片
    pub pc_groupon_image: Option<String>,
    /// 分享图
    pub share_image: Option<String>,
    /// 活动说明
    pub groupon_explain: Option<String>,
    /// 所属店铺
    pub shop_id: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
}

impl GrouponPromotion {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon_promotion";
}


/// 拼团信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GrouponPromotionDetail {
    /// ID [NOT NULL]
    pub groupon_detail_id: i64,
    /// 拼团活动 ID [NOT NULL]
    pub groupon_promotion_id: i64,
    /// 拼团商品 ID [NOT NULL]
    pub groupon_promotion_item_id: i64,
    /// 商品ID [NOT NULL]
    pub product_id: i64,
    /// 规格 id [NOT NULL]
    pub sku_id: i64,
    /// 拼团价 [NOT NULL]
    pub group_price: String,
    /// 可售数量 [NOT NULL]
    pub limit_num: i32,
    /// 是否参加；0-否，1-是 [NOT NULL]
    pub is_join: i32,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
}

impl GrouponPromotionDetail {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon_promotion_detail";
}


/// 拼团活动商品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GrouponPromotionItem {
    /// 拼团活动商品 ID [NOT NULL]
    pub groupon_promotion_item_id: i64,
    /// 拼团活动 ID [NOT NULL]
    pub groupon_promotion_id: i64,
    /// 商品ID [NOT NULL]
    pub product_id: i64,
    /// 拼团类型；1-普通团，2-阶梯团 [NOT NULL]
    pub group_type: i32,
    /// 成团人数（json） [NOT NULL]
    pub group_num: String,
    /// 每单限购数量；0-不限购 [NOT NULL]
    pub order_limit_num: i32,
    /// 是否自动成团；0-否，1-是 [NOT NULL]
    pub is_auto_group: i32,
    /// 成团最低人数 [NOT NULL]
    pub min_group_num: i32,
    /// 排序 [NOT NULL]
    pub sort: i32,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
    /// 成团最低价
    pub min_group_price: Option<Decimal>,
}

impl GrouponPromotionItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon_promotion_item";
}


/// 拼团记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GrouponPromotionRecord {
    /// 拼团记录 ID [NOT NULL]
    pub groupon_promotion_record_id: i64,
    /// 拼团活动 ID [NOT NULL]
    pub groupon_promotion_id: i64,
    /// 拼团活动商品 ID [NOT NULL]
    pub groupon_promotion_item_id: i64,
    /// 商品ID [NOT NULL]
    pub product_id: i64,
    /// 商品名称 [NOT NULL]
    pub product_name: String,
    /// 参与类型；1-开团，2-参团 [NOT NULL]
    pub join_type: i32,
    /// 拼团状态；1-待成团，2-拼团成功，3-拼团失败 [NOT NULL]
    pub join_status: i32,
    /// 用户头像
    pub avatar: Option<String>,
    /// 用户名
    pub username: Option<String>,
    /// 用户 ID [NOT NULL]
    pub user_id: i64,
    /// 订单 ID [NOT NULL]
    pub order_id: i64,
    /// 订单号 [NOT NULL]
    pub order_sn: String,
    /// 是否虚拟；0-否，1-是 [NOT NULL]
    pub is_virtual: i32,
    /// 支付金额 [NOT NULL]
    pub paid_amount: Decimal,
    /// 开团记录 ID [NOT NULL]
    pub open_group_record_id: i64,
    /// 开团时间 [NOT NULL]
    pub open_group_time: i64,
    /// 团长名称 [NOT NULL]
    pub group_captain_name: String,
    /// 成团截止时间 [NOT NULL]
    pub join_expiration_time: i64,
    /// 成团最低人数 [NOT NULL]
    pub min_group_num: i32,
    /// 所属店铺 [NOT NULL]
    pub shop_id: i32,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
}

impl GrouponPromotionRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon_promotion_record";
}


/// groupon_promotion_shops
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GrouponPromotionShops {
    /// [NOT NULL]
    pub groupon_shop_id: i32,
    /// 活动id
    pub groupon_promotion_id: Option<i32>,
    /// 适用店铺
    pub shop_ids: Option<String>,
}

impl GrouponPromotionShops {
    /// 数据库表名。
    pub const TABLE: &'static str = "groupon_promotion_shops";
}


/// 抽奖活动表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LotteryActivity {
    /// 活动ID [NOT NULL]
    pub activity_id: i32,
    /// 活动名称 [NOT NULL]
    pub activity_name: String,
    /// 活动开始时间(秒级时间戳) [NOT NULL]
    pub start_time: i32,
    /// 活动结束时间(秒级时间戳) [NOT NULL]
    pub end_time: i32,
    /// 每次抽奖消耗积分 [NOT NULL]
    pub cost_points: i32,
    /// 每日最大抽奖次数(0表示不限制) [NOT NULL]
    pub daily_limit: i32,
    /// 最大抽奖次数(0表示不限制) [NOT NULL]
    pub total_limit: i32,
    /// 每人最多中奖次数(0表示不限制) [NOT NULL]
    pub win_limit: i32,
    pub decorate_config: Option<String>,
    pub grid_config: Option<String>,
    /// 未中奖名称(未选择奖品的位置显示) [NOT NULL]
    pub no_win_name: String,
    /// 未中奖图片URL(未选择奖品的位置显示) [NOT NULL]
    pub no_win_image: String,
    /// 未中奖提示语(弹窗显示) [NOT NULL]
    pub no_win_tips: String,
    /// 店铺ID(平台为0) [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub is_delete: i32,
    /// 创建时间(秒级时间戳) [NOT NULL]
    pub create_time: i32,
    /// 更新时间(秒级时间戳) [NOT NULL]
    pub update_time: i32,
    /// 抽奖参与条件:0-无限制(免费),1-消耗积分 [NOT NULL]
    pub cost_type: i32,
}

impl LotteryActivity {
    /// 数据库表名。
    pub const TABLE: &'static str = "lottery_activity";
}


/// 活动奖品表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LotteryPrize {
    /// 奖品ID [NOT NULL]
    pub prize_id: i32,
    /// 活动ID [NOT NULL]
    pub activity_id: i32,
    /// 奖品名称 [NOT NULL]
    pub prize_name: String,
    /// [NOT NULL]
    pub prize_type: i32,
    /// 奖品值(积分数量/商品ID/优惠券ID) [NOT NULL]
    pub prize_value: String,
    /// 奖品图片URL [NOT NULL]
    pub prize_image: String,
    /// 奖品库存 [NOT NULL]
    pub prize_stock: i32,
    /// 已使用库存 [NOT NULL]
    pub prize_stock_used: i32,
    /// 创建时间(秒级时间戳) [NOT NULL]
    pub create_time: i32,
    /// 更新时间(秒级时间戳) [NOT NULL]
    pub update_time: i32,
}

impl LotteryPrize {
    /// 数据库表名。
    pub const TABLE: &'static str = "lottery_prize";
}


/// 抽奖记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LotteryRecord {
    /// 记录ID [NOT NULL]
    pub record_id: i32,
    /// 活动ID [NOT NULL]
    pub activity_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 中奖奖品ID(0表示未中奖) [NOT NULL]
    pub prize_id: i32,
    /// 奖品名称 [NOT NULL]
    pub prize_name: String,
    /// [NOT NULL]
    pub prize_type: i32,
    /// 奖品值 [NOT NULL]
    pub prize_value: String,
    /// 奖品图片
    pub prize_image: Option<String>,
    /// 消耗积分 [NOT NULL]
    pub cost_points: i32,
    /// [NOT NULL]
    pub is_win: i32,
    /// [NOT NULL]
    pub status: i32,
    /// 领取时间(秒级时间戳)
    pub receive_time: Option<i32>,
    /// 抽奖时间(秒级时间戳) [NOT NULL]
    pub create_time: i32,
}

impl LotteryRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "lottery_record";
}


/// 用户抽奖统计表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LotteryUserStat {
    /// 统计ID [NOT NULL]
    pub stat_id: i32,
    /// 活动ID [NOT NULL]
    pub activity_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 累计抽奖次数 [NOT NULL]
    pub total_count: i32,
    /// 累计中奖次数 [NOT NULL]
    pub win_count: i32,
    /// 今日抽奖次数 [NOT NULL]
    pub today_count: i32,
    /// 今日日期(YYYY-MM-DD) [NOT NULL]
    pub today_date: String,
    /// 最后抽奖时间(秒级时间戳)
    pub last_lottery_time: Option<i32>,
    /// 创建时间(秒级时间戳) [NOT NULL]
    pub create_time: i32,
    /// 更新时间(秒级时间戳) [NOT NULL]
    pub update_time: i32,
}

impl LotteryUserStat {
    /// 数据库表名。
    pub const TABLE: &'static str = "lottery_user_stat";
}


/// 积分兑换表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PointsExchange {
    /// id [NOT NULL]
    pub id: i32,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 所需积分 [NOT NULL]
    pub exchange_integral: i32,
    /// 积分抵扣金额 [NOT NULL]
    pub points_deducted_amount: Decimal,
    /// 是否热门 [NOT NULL]
    pub is_hot: i32,
    /// 是否生效 [NOT NULL]
    pub is_enabled: i32,
    /// 属性ID
    pub sku_id: Option<i32>,
}

impl PointsExchange {
    /// 数据库表名。
    pub const TABLE: &'static str = "points_exchange";
}


/// 活动赠品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductGift {
    /// [NOT NULL]
    pub gift_id: i32,
    /// 赠品名称 [NOT NULL]
    pub gift_name: String,
    /// 赠品库存 [NOT NULL]
    pub gift_stock: i32,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 规格ID [NOT NULL]
    pub sku_id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
}

impl ProductGift {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_gift";
}


/// 商品促销
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductPromotion {
    /// 活动id [NOT NULL]
    pub promotion_id: i32,
    /// 活动名称 [NOT NULL]
    pub promotion_name: String,
    /// 开始时间 [NOT NULL]
    pub start_time: i32,
    /// 结束时间 [NOT NULL]
    pub end_time: i32,
    /// [JSON]仅限会员等级 [NOT NULL]
    pub limit_user_rank: String,
    /// [NOT NULL]
    pub range: i32,
    /// [json]活动范围相关信息 [NOT NULL]
    pub range_data: String,
    /// 订单金额下限 [NOT NULL]
    pub min_order_amount: Decimal,
    /// 订单金额上限，可用于阶梯设置 [NOT NULL]
    pub max_order_amount: Decimal,
    /// 优惠类型，1：满减，2：折扣，3：满赠 [NOT NULL]
    pub promotion_type: i32,
    /// [data]类型相关数据，如满减金额、折扣金额、赠品等 [NOT NULL]
    pub promotion_type_data: String,
    /// 是否启用 [NOT NULL]
    pub is_available: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 优惠规则：1 阶梯优惠 0 循环优惠 [NOT NULL]
    pub rules_type: i32,
    /// 优惠单位：1 元 2 件 [NOT NULL]
    pub unit: i32,
}

impl ProductPromotion {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_promotion";
}


/// 活动表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Promotion {
    /// 活动id [NOT NULL]
    pub promotion_id: i32,
    /// 活动名称 [NOT NULL]
    pub promotion_name: String,
    /// 开始时间 [NOT NULL]
    pub start_time: i32,
    /// 结束时间 [NOT NULL]
    pub end_time: i32,
    /// 类型1秒杀2优惠券3满减4折扣5满赠6限时折扣 [NOT NULL]
    pub r#type: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 活动对应的id
    pub relation_id: Option<i32>,
    pub range: Option<i32>,
    /// [json]活动范围相关信息
    pub range_data: Option<String>,
    /// 允许的sku
    pub sku_ids: Option<String>,
    /// 是否启用
    pub is_available: Option<i32>,
    /// 是否删除
    pub is_delete: Option<i32>,
    /// 适用会员范围：0-全部会员，1-指定标记的会员 [NOT NULL]
    pub member_range: i32,
    /// 选择的会员标记列表（JSON格式，如[1,2,3]）
    pub member_marks: Option<String>,
    /// 会员标签二期人群圈选(MemberAudienceDTO序列化JSON)，非空则优先于 member_range/member_marks
    pub member_scope: Option<String>,
}

impl Promotion {
    /// 数据库表名。
    pub const TABLE: &'static str = "promotion";
}


/// 用户充值金额快捷表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RechargeSetting {
    /// [NOT NULL]
    pub recharge_id: i32,
    /// 充值金额
    pub money: Option<Decimal>,
    /// 赠送金额
    pub discount_money: Option<Decimal>,
    /// 排序
    pub sort_order: Option<i32>,
    /// 显示或者隐藏
    pub is_show: Option<i32>,
}

impl RechargeSetting {
    /// 数据库表名。
    pub const TABLE: &'static str = "recharge_setting";
}


/// 秒杀活动表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Seckill {
    /// [NOT NULL]
    pub seckill_id: i32,
    /// 活动名称
    pub seckill_name: Option<String>,
    /// 起始时间
    pub seckill_start_time: Option<i32>,
    /// 结束时间
    pub seckill_end_time: Option<i32>,
    /// 限购数量
    pub seckill_limit_num: Option<i32>,
    /// 商品id
    pub product_id: Option<i32>,
    /// 店铺ID，平台为0 [NOT NULL]
    pub shop_id: i32,
}

impl Seckill {
    /// 数据库表名。
    pub const TABLE: &'static str = "seckill";
}


/// 秒杀活动商品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SeckillItem {
    /// [NOT NULL]
    pub rec_id: i32,
    /// 关联活动id
    pub seckill_id: Option<i32>,
    /// 商品id
    pub product_id: Option<i32>,
    /// sku id
    pub sku_id: Option<i32>,
    /// 秒杀价格
    pub seckill_price: Option<Decimal>,
    /// 活动商品库存
    pub seckill_stock: Option<i32>,
    /// 活动销量
    pub seckill_sales: Option<i32>,
    /// 开始时间（此处重复字段是为了提高查询效率）
    pub seckill_start_time: Option<i32>,
    /// 结束时间（此处重复字段是为了提高查询效率）
    pub seckill_end_time: Option<i32>,
}

impl SeckillItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "seckill_item";
}


/// short_url
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShortUrl {
    /// 主键 [NOT NULL]
    pub short_url_id: i32,
    /// 短码，比如 aB12Xz [NOT NULL]
    pub short_code: String,
    /// 商品详情真实地址 [NOT NULL]
    pub long_url: String,
    /// Null和1 是h5       2是pc
    pub r#type: Option<i32>,
    /// 生成时间
    pub add_time: Option<String>,
}

impl ShortUrl {
    /// 数据库表名。
    pub const TABLE: &'static str = "short_url";
}


/// 签到表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Sign {
    /// [NOT NULL]
    pub id: i32,
    /// 会员id
    pub user_id: Option<i32>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 当前签到次数
    pub sign_num: Option<i32>,
}

impl Sign {
    /// 数据库表名。
    pub const TABLE: &'static str = "sign";
}


/// 签到类型
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SignInSetting {
    /// [NOT NULL]
    pub id: i32,
    /// 名称
    pub name: Option<String>,
    /// 赠送积分
    pub points: Option<i32>,
    /// 天数
    pub day_num: Option<i32>,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 添加时间 [NOT NULL]
    pub create_time: i32,
}

impl SignInSetting {
    /// 数据库表名。
    pub const TABLE: &'static str = "sign_in_setting";
}


/// 限时折扣
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct TimeDiscount {
    /// 活动id [NOT NULL]
    pub discount_id: i32,
    /// 活动名称 [NOT NULL]
    pub promotion_name: String,
    /// 开始时间 [NOT NULL]
    pub start_time: i32,
    /// 结束时间 [NOT NULL]
    pub end_time: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 创建时间
    pub add_time: Option<i32>,
}

impl TimeDiscount {
    /// 数据库表名。
    pub const TABLE: &'static str = "time_discount";
}


/// 限时折扣商品规格
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct TimeDiscountItem {
    /// 规格ID [NOT NULL]
    pub item_id: i32,
    /// 关联折扣活动ID [NOT NULL]
    pub discount_id: i32,
    /// 开始时间 [NOT NULL]
    pub start_time: i32,
    /// 结束时间 [NOT NULL]
    pub end_time: i32,
    /// 产品ID [NOT NULL]
    pub product_id: i32,
    /// 规格ID
    pub sku_ids: Option<String>,
    /// 促销价格，减的金额或打的折数 [NOT NULL]
    pub value: Decimal,
    /// 1打折2减钱3促销 [NOT NULL]
    pub discount_type: i32,
}

impl TimeDiscountItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "time_discount_item";
}