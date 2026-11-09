//! 会员域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `collect_product`, `feedback`, `member_tag`, `member_tag_group`, `pickup_contact`, `rank_growth_log`, `shop_member`, `user`, `user_address`, `user_authorize`, `user_company`, `user_coupon`, `user_follow`, `user_growth_points_log`, `user_member_tag`, `user_message`, `user_message_log`, `user_points_log`, `user_premium_card`, `user_rank`, `user_rank_config`, `user_rank_log`, `user_type`, `user_type_account`, `user_withdraw_account`, `virtual_user_info`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 会员收藏商品的记录列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CollectProduct {
    /// 收藏记录的自增id [NOT NULL]
    pub collect_id: i32,
    /// 该条收藏记录的会员id，取值于users的user_id [NOT NULL]
    pub user_id: i32,
    /// 收藏的商品id，取值于goods的goods_id [NOT NULL]
    pub product_id: i32,
    /// 收藏时间 [NOT NULL]
    pub add_time: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl CollectProduct {
    /// 数据库表名。
    pub const TABLE: &'static str = "collect_product";
}


/// 会员留言表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Feedback {
    /// 反馈ID [NOT NULL]
    pub id: i32,
    /// 是否为回复 [NOT NULL]
    pub parent_id: i32,
    /// 反馈会员id [NOT NULL]
    pub user_id: i32,
    /// 反馈会员名 [NOT NULL]
    pub username: String,
    /// 邮箱
    pub email: Option<String>,
    /// 手机号
    pub mobile: Option<String>,
    /// 反馈标题 [NOT NULL]
    pub title: String,
    /// 类型 [NOT NULL]
    pub r#type: i32,
    /// 状态，0：待回复，1：已回复，2：无效 [NOT NULL]
    pub status: i32,
    /// 内容 [NOT NULL]
    pub content: String,
    /// 时间 [NOT NULL]
    pub add_time: i32,
    /// 【JSON】图片上传
    pub feedback_pics: Option<String>,
    /// 商品ID
    pub product_id: Option<i32>,
    /// 订单ID
    pub order_id: Option<i32>,
    /// 投诉项
    pub complaint_info: Option<String>,
    /// 店铺id
    pub shop_id: Option<i32>,
}

impl Feedback {
    /// 数据库表名。
    pub const TABLE: &'static str = "feedback";
}


/// 会员标签
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MemberTag {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 店铺ID，0=平台 [NOT NULL]
    pub shop_id: i32,
    /// 所属标签组ID [NOT NULL]
    pub group_id: i64,
    /// 标签名称 [NOT NULL]
    pub tag_name: String,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 创建人主键
    pub create_by_id: Option<i64>,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间
    pub update_time: Option<i64>,
    /// 编辑人主键
    pub update_by_id: Option<i64>,
    /// 更新人名称
    pub update_by_name: Option<String>,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl MemberTag {
    /// 数据库表名。
    pub const TABLE: &'static str = "member_tag";
}


/// 会员标签组
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MemberTagGroup {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 店铺ID，0=平台 [NOT NULL]
    pub shop_id: i32,
    /// 组名称 [NOT NULL]
    pub group_name: String,
    /// 置顶权重；0-未置顶，正整数越大表示越新置顶 [NOT NULL]
    pub is_top: i32,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 创建人主键
    pub create_by_id: Option<i64>,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间
    pub update_time: Option<i64>,
    /// 编辑人主键
    pub update_by_id: Option<i64>,
    /// 更新人名称
    pub update_by_name: Option<String>,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl MemberTagGroup {
    /// 数据库表名。
    pub const TABLE: &'static str = "member_tag_group";
}


/// 自提联系人表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PickupContact {
    /// 主键ID [NOT NULL]
    pub id: i64,
    /// 客户ID [NOT NULL]
    pub user_id: i32,
    /// 联系人名称 [NOT NULL]
    pub contact_name: String,
    /// 联系人电话 [NOT NULL]
    pub contact_phone: String,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
    /// 删除标志；0-未删除，1-已删除 [NOT NULL]
    pub is_del: i32,
}

impl PickupContact {
    /// 数据库表名。
    pub const TABLE: &'static str = "pickup_contact";
}


/// 成长值日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RankGrowthLog {
    /// [NOT NULL]
    pub id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 成长值类型：1 完成下单 2 退款 [NOT NULL]
    pub r#type: i32,
    /// 成长值 [NOT NULL]
    pub growth_points: i32,
    /// 变化类型：1 增加 2 减少 [NOT NULL]
    pub change_type: i32,
    /// 创建时间
    pub create_time: Option<i32>,
}

impl RankGrowthLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "rank_growth_log";
}


/// 店铺会员绑定表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopMember {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 关联 user.user_id [NOT NULL]
    pub user_id: i32,
    /// 关联 shop.shop_id（门店/店铺统一） [NOT NULL]
    pub shop_id: i32,
    /// 该店累计已支付订单数 [NOT NULL]
    pub order_count: i32,
    /// 该店累计消费金额（已支付） [NOT NULL]
    pub order_amount: Decimal,
    /// 入会时间（首次成为会员，Unix秒）
    pub join_time: Option<i64>,
    /// 最近消费时间（最近一次已支付，Unix秒）
    pub last_consume_time: Option<i64>,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
    /// 逻辑删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl ShopMember {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_member";
}


/// 用户表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct User {
    /// 会员资料自增id [NOT NULL]
    pub user_id: i32,
    /// 用户名 [NOT NULL]
    pub username: String,
    /// 头像 [NOT NULL]
    pub avatar: String,
    /// 手机号 [NOT NULL]
    pub mobile: String,
    /// 手机号是否已验证 [NOT NULL]
    pub mobile_validated: i32,
    /// 会员邮箱 [NOT NULL]
    pub email: String,
    /// 邮箱是否已验证 [NOT NULL]
    pub email_validated: i32,
    /// 昵称
    pub nickname: Option<String>,
    /// 用户密码 [NOT NULL]
    pub password: String,
    /// 出生日期
    pub birthday: Option<String>,
    /// 用户现有余额 [NOT NULL]
    pub balance: Decimal,
    /// 用户冻结余额 [NOT NULL]
    pub frozen_balance: Decimal,
    /// 分销佣金可提现余额（上线后新结算佣金累加，老数据归余额） [NOT NULL]
    pub commission_balance: Decimal,
    /// 分销佣金提现冻结余额 [NOT NULL]
    pub frozen_commission_balance: Decimal,
    /// 积分 [NOT NULL]
    pub points: i32,
    /// 成长值 [NOT NULL]
    pub growth_points: i32,
    /// 当前会员地址 [NOT NULL]
    pub address_id: i32,
    /// 注册时间 [NOT NULL]
    pub reg_time: i32,
    /// 最后一次登录时间 [NOT NULL]
    pub last_login: i32,
    /// 最后一次登录IP [NOT NULL]
    pub last_ip: String,
    /// 会员等级id [NOT NULL]
    pub rank_id: i32,
    /// 推荐人会员id [NOT NULL]
    pub referrer_user_id: i32,
    /// [NOT NULL]
    pub from_tag: i32,
    /// 是否是超级会员 1是 0不是 2过期
    pub is_svip: Option<i32>,
    /// 会员过期时间
    pub svip_expire_time: Option<i32>,
    /// 累积消费次数 [NOT NULL]
    pub order_count: i32,
    /// 累积消费金额 [NOT NULL]
    pub order_amount: Decimal,
    /// 【JSON】历史浏览记录id：number[]
    pub history_product_ids: Option<String>,
    /// 是否是分销员（已弃用）
    pub is_distribution: Option<i32>,
    /// 分销员注册时间（已弃用）
    pub distribution_register_time: Option<i32>,
    /// 微信二维码图片 [NOT NULL]
    pub wechat_img: String,
    /// 是否企业认证：1 是 0 否 [NOT NULL]
    pub is_company_auth: i32,
    /// 状态；0-禁用，1-启用
    pub status: Option<i32>,
    /// 0会员，1游客
    pub user_type: Option<i32>,
    /// 会员标签 [NOT NULL]
    pub admin_note: String,
    /// 标记12345 [NOT NULL]
    pub mark: i32,
}

impl User {
    /// 数据库表名。
    pub const TABLE: &'static str = "user";
}


///  收货人信息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserAddress {
    /// 自增id [NOT NULL]
    pub address_id: i32,
    /// 国家或地区标准代码
    pub country_code: Option<String>,
    /// 国家或地区名称
    pub country_name: Option<String>,
    /// 收货地址别名 [NOT NULL]
    pub address_tag: String,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 收货人的名字 [NOT NULL]
    pub consignee: String,
    /// 收货人的email
    pub email: Option<String>,
    /// [JSON]收货地址id:number[] [NOT NULL]
    pub region_ids: String,
    /// [JSON]地区名称:string[] [NOT NULL]
    pub region_names: String,
    /// 收货人的详细地址 [NOT NULL]
    pub address: String,
    /// 补充地址
    pub address_line_two: Option<String>,
    /// 建筑名称
    pub building_name: Option<String>,
    /// 楼层
    pub floor: Option<String>,
    /// 单元
    pub unit: Option<String>,
    /// 结构化地址 JSON
    pub address_data: Option<String>,
    /// 服务端生成的完整地址
    pub formatted_address: Option<String>,
    /// 定位地址
    pub position_address: Option<String>,
    /// 收货人的邮编
    pub postcode: Option<String>,
    /// 收货人的电话
    pub telephone: Option<String>,
    /// 收货人的手机号 [NOT NULL]
    pub mobile: String,
    /// 是否默认 [NOT NULL]
    pub is_default: i32,
    pub is_system: Option<i32>,
    /// 是否选中 [NOT NULL]
    pub is_selected: i32,
    /// 区号
    pub mobile_area_code: Option<String>,
    /// E.164 国际手机号
    pub international_mobile: Option<String>,
    /// 纬度
    pub latitude: Option<Decimal>,
    /// 经度
    pub longitude: Option<Decimal>,
}

impl UserAddress {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_address";
}


/// 用户第三方授权信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserAuthorize {
    /// 会员登录绑定 [NOT NULL]
    pub authorize_id: i32,
    /// 绑定平台：1:微信，2:小程序 [NOT NULL]
    pub authorize_type: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// openid [NOT NULL]
    pub open_id: String,
    /// 返回数据
    pub open_data: Option<String>,
    /// 微信用户名 [NOT NULL]
    pub open_name: String,
    /// 用户头像 [NOT NULL]
    pub open_photo: String,
    /// 微信开放平台唯一识别码
    pub unionid: Option<String>,
    /// 授权时间（留作登录时效）
    pub add_time: Option<i32>,
}

impl UserAuthorize {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_authorize";
}


/// 会员企业认证表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserCompany {
    /// [NOT NULL]
    pub id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 认证类型：1 个人 2 企业 [NOT NULL]
    pub r#type: i32,
    /// 联系人名称 [NOT NULL]
    pub contact_name: String,
    /// 联系人手机 [NOT NULL]
    pub contact_mobile: String,
    /// 企业名称 [NOT NULL]
    pub company_name: String,
    /// 企业认证信息[json]
    pub company_data: Option<String>,
    /// 审核状态：1 待审核 2 审核通过 3 审核未通过 [NOT NULL]
    pub status: i32,
    /// 审核备注 [NOT NULL]
    pub audit_remark: String,
    /// 审核时间
    pub audit_time: Option<i32>,
    /// 添加时间
    pub add_time: Option<i32>,
}

impl UserCompany {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_company";
}


/// 用户优惠券表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserCoupon {
    /// 会员优惠券id [NOT NULL]
    pub id: i32,
    /// 优惠券类型ID [NOT NULL]
    pub coupon_id: i32,
    /// 优惠券SN [NOT NULL]
    pub coupon_sn: String,
    /// 会员id [NOT NULL]
    pub user_id: i32,
    /// 优惠券使用的时间 [NOT NULL]
    pub used_time: i32,
    /// 优惠券所使用的订单ID [NOT NULL]
    pub order_id: i32,
    /// 优惠券使用的开始时间 [NOT NULL]
    pub start_date: i32,
    /// 优惠券可使用的结束时间 [NOT NULL]
    pub end_date: i32,
    /// 来源类型:0-普通领取,1-抽奖中奖
    pub source_type: Option<i32>,
}

impl UserCoupon {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_coupon";
}


/// 用户关注关系表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserFollow {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 关注者用户ID [NOT NULL]
    pub user_id: i32,
    /// 被关注作者用户ID [NOT NULL]
    pub author_id: i32,
    /// 关注时间 [NOT NULL]
    pub add_time: i64,
}

impl UserFollow {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_follow";
}


/// 用户成长积分表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserGrowthPointsLog {
    /// 成长值自增 ID 号 [NOT NULL]
    pub log_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 增加或减少的成长值 [NOT NULL]
    pub points: i32,
    /// 该操作发生的时间 [NOT NULL]
    pub change_time: i32,
    /// 该操作的备注 [NOT NULL]
    pub change_desc: String,
    /// 增加还是减少，1：增加，2：减少 [NOT NULL]
    pub change_type: i32,
}

impl UserGrowthPointsLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_growth_points_log";
}


/// 用户会员标签关联
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserMemberTag {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 店铺ID，0=平台 [NOT NULL]
    pub shop_id: i32,
    /// 用户ID [NOT NULL]
    pub user_id: i64,
    /// 标签ID [NOT NULL]
    pub tag_id: i64,
    /// 打标时间
    pub create_time: Option<i64>,
}

impl UserMemberTag {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_member_tag";
}


/// 用户站内信
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserMessage {
    /// [NOT NULL]
    pub message_id: i32,
    /// [NOT NULL]
    pub message_log_id: i32,
    /// [NOT NULL]
    pub user_id: i32,
    /// [NOT NULL]
    pub user_rank: i32,
    /// [NOT NULL]
    pub is_read: i32,
    /// [NOT NULL]
    pub title: String,
    /// [NOT NULL]
    pub content: String,
    pub message_template: Option<String>,
    /// [NOT NULL]
    pub link: String,
    /// [NOT NULL]
    pub add_time: i32,
    /// 1:订单通知 [NOT NULL]
    pub r#type: i32,
}

impl UserMessage {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_message";
}


/// 站内信
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserMessageLog {
    /// [NOT NULL]
    pub message_log_id: i32,
    /// 消息类型 [NOT NULL]
    pub message_type: i32,
    /// 发送类型 [NOT NULL]
    pub send_user_type: i32,
    /// 用户集合 [NOT NULL]
    pub user_ids: String,
    /// 会员等级 [NOT NULL]
    pub user_rank: i32,
    /// 标题 [NOT NULL]
    pub message_title: String,
    /// 内容 [NOT NULL]
    pub message_content: String,
    /// 链接地址 [NOT NULL]
    pub message_link: String,
    /// 发送时间 [NOT NULL]
    pub send_time: i32,
    /// 是否撤回 [NOT NULL]
    pub is_recall: i32,
}

impl UserMessageLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_message_log";
}


/// 用户积分日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserPointsLog {
    /// 自增 ID 号 [NOT NULL]
    pub log_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 增加或减少的消费积分 [NOT NULL]
    pub points: i32,
    /// 该操作发生的时间 [NOT NULL]
    pub change_time: i32,
    /// 该操作的备注 [NOT NULL]
    pub change_desc: String,
    /// 增加还是减少，1：增加，2：减少 [NOT NULL]
    pub change_type: i32,
    /// 1订单
    pub relation_type: Option<i32>,
    /// 关联id
    pub relation_id: Option<i32>,
}

impl UserPointsLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_points_log";
}


/// 用户持有的付费会员卡
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserPremiumCard {
    /// [NOT NULL]
    pub id: i32,
    /// [NOT NULL]
    pub user_id: i32,
    /// [NOT NULL]
    pub card_id: i32,
    /// [NOT NULL]
    pub order_id: i32,
    /// [NOT NULL]
    pub card_no: String,
    /// 生效时间(秒级时间戳) [NOT NULL]
    pub start_time: i64,
    /// 到期时间(秒级时间戳，须用bigint：续期/长期卡会超过INT上限2038-01-19) [NOT NULL]
    pub end_time: i64,
    /// [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub remind_sent: i32,
    /// 添加时间(秒级时间戳) [NOT NULL]
    pub add_time: i64,
}

impl UserPremiumCard {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_premium_card";
}


/// 会员等级配置信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserRank {
    /// 会员等级编号 [NOT NULL]
    pub rank_id: i32,
    /// 会员等级名称 [NOT NULL]
    pub rank_name: String,
    /// 该等级的最低成长值（升级条件） [NOT NULL]
    pub min_growth_points: Decimal,
    /// 该等级的最高成长值（已弃用） [NOT NULL]
    pub max_growth_points: i32,
    /// 该会员等级的商品折扣 [NOT NULL]
    pub discount: Decimal,
    /// 是否显示商品价格（已弃用） [NOT NULL]
    pub show_price: i32,
    /// 等级类型：1：根据成长值变化，2：根据消费行为 [NOT NULL]
    pub rank_type: i32,
    /// 会员图标 [NOT NULL]
    pub rank_logo: String,
    /// 会员等级图标 [NOT NULL]
    pub rank_ico: String,
    /// 会员等级背景图（移动端） [NOT NULL]
    pub rank_bg: String,
    /// 权益积分 [NOT NULL]
    pub rank_point: String,
    /// 是否包邮：1 是 0 否 [NOT NULL]
    pub free_shipping: i32,
    /// 会员卡类型：1 背景色 2自定义图片 [NOT NULL]
    pub rank_card_type: i32,
    /// 自定义权益【json】
    pub rights: Option<String>,
    /// 会员等级 [NOT NULL]
    pub rank_level: String,
}

impl UserRank {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_rank";
}


/// 会员等级配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserRankConfig {
    /// [NOT NULL]
    pub id: i32,
    /// 配置编码 [NOT NULL]
    pub code: String,
    /// 等级类型：1 成长值 2 消费行为 [NOT NULL]
    pub rank_type: i32,
    /// 等级配置[json]
    pub data: Option<String>,
}

impl UserRankConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_rank_config";
}


/// 会员等级变更日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserRankLog {
    /// [NOT NULL]
    pub id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 等级id [NOT NULL]
    pub rank_id: i32,
    /// 等级类型 [NOT NULL]
    pub rank_type: i32,
    /// 等级名称 [NOT NULL]
    pub rank_name: String,
    /// 变更时间
    pub change_time: Option<i32>,
    /// 变更备注 [NOT NULL]
    pub remark: String,
}

impl UserRankLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_rank_log";
}


/// 会员权益表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserType {
    /// [NOT NULL]
    pub user_type_id: i32,
    /// 权益名称 [NOT NULL]
    pub user_type_name: String,
    /// 有效期 [NOT NULL]
    pub period: String,
    /// 原价 [NOT NULL]
    pub original_price: Decimal,
    /// 优惠价 [NOT NULL]
    pub favourable_price: Decimal,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 是否开启 [NOT NULL]
    pub is_on: i32,
}

impl UserType {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_type";
}


/// 会员权益购买记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserTypeAccount {
    /// [NOT NULL]
    pub id: i32,
    /// 用户id
    pub user_id: Option<i32>,
    /// 会员类型id
    pub user_type_id: Option<i32>,
    /// 支付金额
    pub amount: Option<Decimal>,
    /// 支付状态 0待支付 2已支付
    pub status: Option<i32>,
    /// 支付时间
    pub pay_time: Option<i32>,
    /// 描述
    pub desc: Option<String>,
    /// 下单时间
    pub add_time: Option<i32>,
}

impl UserTypeAccount {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_type_account";
}


/// 会员提现账号
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserWithdrawAccount {
    /// 提现账号 [NOT NULL]
    pub account_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 提现账号类型，1：银行卡，2：支付宝，3：微信 [NOT NULL]
    pub account_type: i32,
    /// 姓名 [NOT NULL]
    pub account_name: String,
    /// 银行卡号或账号 [NOT NULL]
    pub account_no: String,
    /// 身份证号 [NOT NULL]
    pub identity: String,
    /// 银行名称 [NOT NULL]
    pub bank_name: String,
}

impl UserWithdrawAccount {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_withdraw_account";
}


/// 虚拟用户信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VirtualUserInfo {
    /// 自增id号 [NOT NULL]
    pub id: i32,
    /// 头像 [NOT NULL]
    pub photo: String,
    /// 名称 [NOT NULL]
    pub nickname: String,
}

impl VirtualUserInfo {
    /// 数据库表名。
    pub const TABLE: &'static str = "virtual_user_info";
}