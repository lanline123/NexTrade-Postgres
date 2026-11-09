//! 店铺域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `admin_user_shop`, `collect_shop`, `order_config`, `shop`, `shop_account_log`, `shop_home_service_distance_pricing`, `shop_local_delivery_distance_pricing`, `shop_meituan_delivery_config`, `shop_product_category`, `shop_withdraw`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 店铺员工列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminUserShop {
    /// [NOT NULL]
    pub id: i32,
    /// 管理员id [NOT NULL]
    pub admin_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 店铺id [NOT NULL]
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
    /// 是否为管理员：1 是 0 否 [NOT NULL]
    pub is_admin: i32,
    /// 创建时间
    pub add_time: Option<i32>,
    /// 权限组id [NOT NULL]
    pub role_id: i32,
    /// 收银上班/交班起始时间
    pub work_on_time: Option<i64>,
    /// 登录后是否默认进收银台 1是0否 [NOT NULL]
    pub default_pos_login: i32,
    /// 上次登录端(admin/shop/store/pos) [NOT NULL]
    pub last_login_system: String,
    /// 收银员绑定的收银台设备码
    pub device_code: Option<String>,
}

impl AdminUserShop {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_user_shop";
}


/// 会员收藏店铺的记录列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CollectShop {
    /// 收藏记录的自增id [NOT NULL]
    pub collect_id: i32,
    /// 该条收藏记录的会员id，取值于users的user_id [NOT NULL]
    pub user_id: i32,
    /// 收藏的店铺id [NOT NULL]
    pub shop_id: i32,
    /// 收藏时间 [NOT NULL]
    pub add_time: i32,
}

impl CollectShop {
    /// 数据库表名。
    pub const TABLE: &'static str = "collect_shop";
}


/// 店铺订单配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderConfig {
    /// [NOT NULL]
    pub id: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 编码 [NOT NULL]
    pub code: String,
    /// [json]配置数据
    pub data: Option<String>,
}

impl OrderConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_config";
}


/// 店铺表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Shop {
    /// 店铺id(随机分配) [NOT NULL]
    pub shop_id: i32,
    /// 店铺名称
    pub shop_title: Option<String>,
    /// 店铺创建时间
    pub add_time: Option<i32>,
    /// 店铺logo
    pub shop_logo: Option<String>,
    /// 点击量
    pub click_count: Option<i32>,
    /// 店铺状态1开业  4暂停运营 10关店
    pub status: Option<i32>,
    /// 商户id
    pub merchant_id: Option<i32>,
    /// 店铺资金
    pub shop_money: Option<Decimal>,
    /// 冻结资金
    pub frozen_money: Option<Decimal>,
    /// 联系电话
    pub contact_mobile: Option<String>,
    /// 简介
    pub description: Option<String>,
    /// 客服电话
    pub kefu_phone: Option<String>,
    /// 客服微信
    pub kefu_weixin: Option<String>,
    /// 客服链接
    pub kefu_link: Option<String>,
    /// 是否联系客服：1 是 0 否 [NOT NULL]
    pub is_contact_kefu: i32,
    /// [json]客服入口信息页面： 1 商品详情页 2 订单页
    pub kefu_inlet: Option<String>,
    /// 发货信息
    pub ship_info: Option<String>,
    /// 收货信息
    pub receive_info: Option<String>,
    /// 最后登录时间 [NOT NULL]
    pub last_login_time: i32,
    /// 供应商设价方式(1按比例，2-按固定数值加价，3-默认售价)
    pub vendor_set_price_type: Option<i32>,
    /// 智能设价（百分比或固定数值）
    pub vendor_set_price_auto_value: Option<Decimal>,
    /// 服务费比例
    pub service_fee_rate: Option<Decimal>,
    /// 手续费比例
    pub fee_rate: Option<Decimal>,
    /// 1店铺，2门店，3自提点
    pub shop_type: Option<i32>,
    /// 店铺父级id
    pub store_parent_id: Option<i32>,
    /// 门店封面
    pub shop_cover_picture: Option<String>,
    /// 门店相册
    pub shop_show_picture: Option<String>,
    /// 联系方式
    pub shop_contact_json: Option<String>,
    /// 运营时间
    pub shop_open_close_json: Option<String>,
    /// region表id
    pub shop_region_id: Option<String>,
    /// 门店详细地址
    pub shop_detailed_address: Option<String>,
    /// 店铺经度
    pub shop_longitude: Option<Decimal>,
    /// 店铺纬度
    pub shop_latitude: Option<Decimal>,
    /// 评星
    pub shop_star: Option<Decimal>,
    /// 总评价分数
    pub shop_total_star: Option<i32>,
    /// 评价数量
    pub shop_star_num: Option<i32>,
    /// 销量
    pub shop_sales: Option<i32>,
    /// 是否使用门店分类
    pub use_shop_category: Option<i32>,
    /// 店铺选择展示分类
    pub shop_show_category: Option<String>,
    /// 店内收银自动抹零：0关闭 1开启 [NOT NULL]
    pub auto_erase_zero: i32,
    /// 收银积分基于订单自动抵扣：0关闭 1开启 [NOT NULL]
    pub auto_deduction_points: i32,
    /// 收银机默认收款方式：(默认)1-现金支付 2-扫码支付 3-余额支付 4-积分支付 [NOT NULL]
    pub default_collection_method: i32,
    /// 扫码后自动加入购物车：0关闭 1开启 [NOT NULL]
    pub auto_add_cart_after_scan: i32,
    /// 线上订单自动打印：0关闭 1开启 [NOT NULL]
    pub auto_print_online_order: i32,
    /// 线下订单自动打印：0关闭 1开启 [NOT NULL]
    pub auto_print_offline_order: i32,
    /// 店铺常用备注列表(JSON数组)
    pub shop_common_remarks: Option<String>,
    /// 收银台商品来源(JSON数组)：0-全部商品，1-门店自建商品，2-平台分配商品，3-供应商商品
    pub pos_product_source: Option<String>,
    /// 是否删除  null或者0标识未删除  1 表示已删除
    pub is_delete: Option<i32>,
}

impl Shop {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop";
}


/// 店铺资金变化
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopAccountLog {
    /// [NOT NULL]
    pub shop_account_log_id: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 店铺资金 [NOT NULL]
    pub shop_money: Decimal,
    /// 店铺冻结资金 [NOT NULL]
    pub frozen_money: Decimal,
    /// 1，提现 [NOT NULL]
    pub r#type: String,
    /// 备注
    pub remarks: Option<String>,
    /// 变动后余额；线上历史余额未知时为空
    pub new_shop_money: Option<Decimal>,
    /// 现店铺冻结资金 [NOT NULL]
    pub new_frozen_money: Decimal,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 新老数据兼容，0:旧版本，1:新版本 [NOT NULL]
    pub is_new: i32,
    /// 资金账户：1线下，2线上 [NOT NULL]
    pub balance_type: i32,
    /// 线上渠道WECHAT/HUIFU/ALIPAY
    pub pay_channel: Option<String>,
    /// 资金变动额，历史空值按旧日志规则计算
    pub change_amount: Option<Decimal>,
    /// 资金事件唯一键，防止重复记账
    pub event_key: Option<String>,
}

impl ShopAccountLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_account_log";
}


/// 门店预约上门服务距离溢价表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopHomeServiceDistancePricing {
    /// 主键ID [NOT NULL]
    pub shop_home_service_distance_pricing_id: i32,
    /// 预约上门服务配置ID，关联 shop_home_service_config.shop_home_service_config_id [NOT NULL]
    pub shop_home_service_config_id: i32,
    /// 超过多少公里 [NOT NULL]
    pub distance_threshold: Decimal,
    /// 每增加多少公里 [NOT NULL]
    pub distance_unit: Decimal,
    /// 上门费增加多少元 [NOT NULL]
    pub additional_fee: Decimal,
    /// 排序，数值越小越优先 [NOT NULL]
    pub sort_order: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl ShopHomeServiceDistancePricing {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_home_service_distance_pricing";
}


/// 门店同城配送距离叠加溢价表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopLocalDeliveryDistancePricing {
    /// 自增ID
    pub shop_local_delivery_distance_pricing_id: Option<i64>,
    /// 门店配送配置ID，关联 shop_local_delivery_config.shop_local_delivery_config_id [NOT NULL]
    pub shop_local_delivery_config_id: i64,
    /// 超过多少公里 [NOT NULL]
    pub distance_threshold: Decimal,
    /// 每增加多少公里 [NOT NULL]
    pub distance_unit: Decimal,
    /// 配送增加多少元 [NOT NULL]
    pub additional_fee: Decimal,
    /// 排序，数值越小越优先 [NOT NULL]
    pub sort_order: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl ShopLocalDeliveryDistancePricing {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_local_delivery_distance_pricing";
}


/// 门店美团配送配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopMeituanDeliveryConfig {
    /// 自增ID [NOT NULL]
    pub shop_meituan_delivery_config_id: i32,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// 是否开启美团配送 0关闭 1开启 [NOT NULL]
    pub enabled: i32,
    /// 美团门店ID（本系统生成，注册到美团的 shop_id）
    pub mt_shop_id: Option<String>,
    /// 美团门店状态 0未注册 10创建审核驳回 20创建审核通过 30创建成功 40上线可发单 50修改审核驳回 60修改审核通过 [NOT NULL]
    pub mt_shop_status: i32,
    /// 门店经营一级品类（美团 category 代码，如 110）
    pub goods_category: Option<i32>,
    /// 门店经营二级品类（美团 second_category 代码，如 110002）
    pub second_category: Option<i32>,
    /// 可用配送服务代码，逗号分隔（飞速达4002/快速达4011/及时达4012/集中送4013 等）
    pub delivery_service_codes: Option<String>,
    /// 支付方式 1账期支付 2余额支付 [NOT NULL]
    pub pay_type: i32,
    /// 触发模式 1手动呼叫 2付款后自动呼叫 [NOT NULL]
    pub trigger_mode: i32,
    /// 美团门店名称（默认取 shop.shop_title，可覆盖）
    pub shop_name: Option<String>,
    /// 联系人
    pub contact_name: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 城市名称
    pub city_name: Option<String>,
    /// 门店详细地址（默认取 shop.shop_detailed_address，可覆盖）
    pub address: Option<String>,
    /// 门店经度
    pub longitude: Option<Decimal>,
    /// 门店纬度
    pub latitude: Option<Decimal>,
    /// 营业时间（文本或 JSON）
    pub business_hours: Option<String>,
    /// 企业全称
    pub enterprise_name: Option<String>,
    /// 企业地址
    pub enterprise_address: Option<String>,
    /// 营业执照编号
    pub business_license_no: Option<String>,
    /// 营业执照是否长期有效 0否 1是 [NOT NULL]
    pub license_long_term: i32,
    /// 营业执照有效期开始（yyyy-MM-dd）
    pub license_valid_from: Option<String>,
    /// 营业执照有效期截止（long_term=1 时可空）
    pub license_valid_to: Option<String>,
    /// 注册/成立日期
    pub establish_date: Option<String>,
    /// 发证登记机关
    pub license_authority: Option<String>,
    /// 核准日期
    pub license_approval_date: Option<String>,
    /// 经营范围
    pub business_scope: Option<String>,
    /// 法人/经营者姓名
    pub legal_person_name: Option<String>,
    /// 法人/经营者身份证号
    pub legal_person_id_card: Option<String>,
    /// 法人/经营者手机号
    pub legal_person_phone: Option<String>,
    /// 美团审核备注 / 驳回原因
    pub audit_remark: Option<String>,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl ShopMeituanDeliveryConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_meituan_delivery_config";
}


/// 店铺商品分类表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopProductCategory {
    /// 分类id [NOT NULL]
    pub category_id: i32,
    /// 分类名称 [NOT NULL]
    pub category_name: String,
    /// 所属店铺 [NOT NULL]
    pub shop_id: i32,
    /// 父级分类 [NOT NULL]
    pub parent_id: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 是否显示 [NOT NULL]
    pub is_show: i32,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 分类图片 [NOT NULL]
    pub category_pic: String,
}

impl ShopProductCategory {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_product_category";
}


/// 店铺提现记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopWithdraw {
    /// [NOT NULL]
    pub shop_withdraw_log_id: i32,
    /// 所属店铺 [NOT NULL]
    pub shop_id: i32,
    /// 提现金额 [NOT NULL]
    pub amount: Decimal,
    /// 手续费
    pub handling_fee: Option<Decimal>,
    /// 手续费率
    pub handling_rate: Option<Decimal>,
    /// 状态0待审核2审核不通过3完成4待打款 [NOT NULL]
    pub status: i32,
    /// 提现时间 [NOT NULL]
    pub add_time: i32,
    /// 提现备注 [NOT NULL]
    pub remark: String,
    /// 提现到
    pub merchant_account_id: Option<i32>,
    /// 提现时账户信息数据
    pub account_data: Option<String>,
    /// 审核备注
    pub audit_remark: Option<String>,
    /// 打款凭证
    pub payment_voucher: Option<String>,
    /// 提现单号 [NOT NULL]
    pub withdraw_sn: String,
    /// 提现方式:1线下 2线上(微信API) [NOT NULL]
    pub withdraw_type: i32,
    /// 微信提现单号 [NOT NULL]
    pub channel_withdraw_id: String,
    /// 微信侧状态 [NOT NULL]
    pub channel_status: String,
    /// 线上提现渠道：WECHAT-微信，HUIFU-汇付，ALIPAY-支付宝
    pub pay_channel: Option<String>,
    /// 本申请冻结的线下金额，新线上为0，历史NULL待核账
    pub offline_frozen_amount: Option<Decimal>,
    /// 申请时渠道子商户号快照，避免绑定变更影响出金和查单
    pub channel_sub_mch_id: Option<String>,
}

impl ShopWithdraw {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_withdraw";
}