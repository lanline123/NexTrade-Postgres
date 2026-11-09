//! 系统设置域实体（配置/地区/物流/模板）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `app_version`, `area_code`, `config`, `crons`, `example`, `friend_links`, `gallery`, `gallery_pic`, `keywords`, `logistics_company`, `logistics_company_config`, `mail_templates`, `message_template`, `message_type`, `print_config`, `region`, `sensitive_words`, `shipping_tpl`, `shipping_tpl_info`, `shipping_type`, `tax_rate`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// app版本管理
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AppVersion {
    /// [NOT NULL]
    pub id: i32,
    /// ios版本号
    pub ios_version: Option<String>,
    /// Android版本号
    pub android_version: Option<String>,
    /// ios下载链接
    pub ios_link: Option<String>,
    /// Android下载链接
    pub android_link: Option<String>,
}

impl AppVersion {
    /// 数据库表名。
    pub const TABLE: &'static str = "app_version";
}


/// 区号维护表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AreaCode {
    /// [NOT NULL]
    pub id: i32,
    /// 区号 [NOT NULL]
    pub code: String,
    /// 名称 [NOT NULL]
    pub name: String,
    /// 是否开启：1 是 0 否 [NOT NULL]
    pub is_available: i32,
    /// 是否默认：1 是 0 否 [NOT NULL]
    pub is_default: i32,
}

impl AreaCode {
    /// 数据库表名。
    pub const TABLE: &'static str = "area_code";
}


/// 系统配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Config {
    /// 主键 [NOT NULL]
    pub id: i64,
    /// 业务编码 [NOT NULL]
    pub biz_code: String,
    /// 业务值
    pub biz_val: Option<String>,
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
    /// 是否删除；0-否，1-是
    pub is_del: Option<i32>,
}

impl Config {
    /// 数据库表名。
    pub const TABLE: &'static str = "config";
}


/// 定时任务表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Crons {
    /// 计划任务ID [NOT NULL]
    pub cron_id: i32,
    /// 计划任务编号 [NOT NULL]
    pub cron_sn: String,
    /// 名称 [NOT NULL]
    pub cron_name: String,
    /// 描述
    pub cron_desc: Option<String>,
    /// 上次执行时间 [NOT NULL]
    pub last_runtime: i32,
    /// 下次执行时间 [NOT NULL]
    pub next_runtime: i32,
    /// 定时方式：1：每年，2：每月，3：每周，4：每天，5：每小时，6：每分钟\n11：每隔N年，12：每隔N月，13：每隔N周，14：每隔N天，15：每隔N小时，16：每隔N分钟 [NOT NULL]
    pub cron_type: i32,
    /// [JSON]存对应cron_type数据 [NOT NULL]
    pub cron_config: String,
    /// 是否开启 [NOT NULL]
    pub is_enabled: i32,
    /// 是否仅执行一次 [NOT NULL]
    pub just_once: i32,
    /// 白名单 [NOT NULL]
    pub white_ip_list: String,
}

impl Crons {
    /// 数据库表名。
    pub const TABLE: &'static str = "crons";
}


/// 演示参考表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Example {
    /// 自增id号 [NOT NULL]
    pub example_id: i32,
    /// 名称 [NOT NULL]
    pub example_name: String,
    /// 图片 [NOT NULL]
    pub example_pic: String,
    /// 描述 [NOT NULL]
    pub example_desc: String,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 是否显示 [NOT NULL]
    pub is_show: i32,
    /// 状态
    pub status: Option<i32>,
}

impl Example {
    /// 数据库表名。
    pub const TABLE: &'static str = "example";
}


/// 友情链接
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct FriendLinks {
    /// id [NOT NULL]
    pub link_id: i32,
    /// 友情链接标题 [NOT NULL]
    pub link_title: String,
    /// 友情链接LOGO [NOT NULL]
    pub link_logo: String,
    /// 链接地址 [NOT NULL]
    pub link_url: String,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
}

impl FriendLinks {
    /// 数据库表名。
    pub const TABLE: &'static str = "friend_links";
}


/// 相册表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Gallery {
    /// [NOT NULL]
    pub gallery_id: i32,
    /// [NOT NULL]
    pub parent_id: i32,
    /// [NOT NULL]
    pub gallery_admin_id: i32,
    /// [NOT NULL]
    pub gallery_name: String,
    /// [NOT NULL]
    pub gallery_sort: i32,
    /// [NOT NULL]
    pub gallery_thumb: String,
    /// [NOT NULL]
    pub shop_id: i32,
    /// 供应商 ID
    pub vendor_id: Option<i32>,
}

impl Gallery {
    /// 数据库表名。
    pub const TABLE: &'static str = "gallery";
}


/// 相册图片表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GalleryPic {
    /// [NOT NULL]
    pub pic_id: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 相册id [NOT NULL]
    pub gallery_id: i32,
    /// 操作人员id [NOT NULL]
    pub pic_ower_id: i32,
    /// 图片地址 [NOT NULL]
    pub pic_url: String,
    /// 图片名称 [NOT NULL]
    pub pic_name: String,
    /// 缩略图 [NOT NULL]
    pub pic_thumb: String,
    /// 添加时间 [NOT NULL]
    pub add_time: String,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 0 或者 null 不隐藏 1  表示隐藏
    pub hidden: Option<i32>,
    /// 图片宽度
    pub pic_width: Option<i32>,
    /// 图片高度
    pub pic_height: Option<i32>,
    /// 图片大小（字节）
    pub pic_size: Option<i64>,
    /// 文件格式（如png、jpg、webp）
    pub file_type: Option<String>,
    /// 手动排序值
    pub pic_sort: Option<i32>,
    /// 来源：upload=本地上传, external=外部直链 [NOT NULL]
    pub source: String,
}

impl GalleryPic {
    /// 数据库表名。
    pub const TABLE: &'static str = "gallery_pic";
}


/// 关键词
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Keywords {
    /// 搜索日期 [NOT NULL]
    pub date: String,
    /// 搜索引擎，默认是ecshop [NOT NULL]
    pub searchengine: String,
    /// 搜索关键字，即用户填写的搜索内容 [NOT NULL]
    pub keyword: String,
    /// 搜索次数，按天累加 [NOT NULL]
    pub count: i32,
}

impl Keywords {
    /// 数据库表名。
    pub const TABLE: &'static str = "keywords";
}


/// 配送方式配置信息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LogisticsCompany {
    /// 自增id号 [NOT NULL]
    pub logistics_id: i32,
    /// 配送方式的快递公司代号 [NOT NULL]
    pub logistics_code: String,
    /// 配送方式名称 [NOT NULL]
    pub logistics_name: String,
    /// 配送方式描述 [NOT NULL]
    pub logistics_desc: String,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 是否展示 [NOT NULL]
    pub is_show: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 客户编码
    pub customer_name: Option<String>,
    /// 密码
    pub customer_pwd: Option<String>,
    /// 月结号
    pub month_code: Option<String>,
    /// 网点编码
    pub send_site: Option<String>,
    /// 网点名称
    pub send_staff: Option<String>,
    /// 快递业务类型
    pub exp_type: Option<String>,
    /// 无服务调用物流编号
    pub api_logistics_code: Option<String>,
    /// 快递100快递公司编码（小写，如 shunfeng/yuantong；留空则走内置映射） [NOT NULL]
    pub kuaidi100_code: String,
    /// 快递100电子面单-电子面单账户/月结账号 partnerId [NOT NULL]
    pub kuaidi100_partner_id: String,
    /// 快递100电子面单-电子面单密码 partnerKey [NOT NULL]
    pub kuaidi100_partner_key: String,
    /// 快递100电子面单-电子面单密钥 partnerSecret [NOT NULL]
    pub kuaidi100_partner_secret: String,
    /// 快递100电子面单-收件网点名 net [NOT NULL]
    pub kuaidi100_net: String,
    /// 快递100电子面单-承载快递员名 checkMan [NOT NULL]
    pub kuaidi100_check_man: String,
    /// 快递100电子面单-产品类型 expType [NOT NULL]
    pub kuaidi100_exp_type: String,
    /// 快递100电子面单-主单模板ID tempId（IMAGE/HTML/CLOUD 网点面单必填） [NOT NULL]
    pub kuaidi100_temp_id: String,
    /// 快递100电子面单-子单模板ID childTempId [NOT NULL]
    pub kuaidi100_child_temp_id: String,
    /// 快递100电子面单-回单模板ID backTempId [NOT NULL]
    pub kuaidi100_back_temp_id: String,
    /// 是否启用：1启用，0禁用 [NOT NULL]
    pub is_enabled: i32,
}

impl LogisticsCompany {
    /// 数据库表名。
    pub const TABLE: &'static str = "logistics_company";
}


/// 物流公司配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LogisticsCompanyConfig {
    /// 配置ID [NOT NULL]
    pub config_id: i32,
    /// 物流公司ID（关联 logistics_company.logistics_id） [NOT NULL]
    pub logistics_id: i32,
    /// 店铺ID（平台=0，店铺/门店>0） [NOT NULL]
    pub shop_id: i32,
    /// 供应商ID（非供应商=0，供应商>0） [NOT NULL]
    pub vendor_id: i32,
    /// 客户编码
    pub customer_name: Option<String>,
    /// 密码
    pub customer_pwd: Option<String>,
    /// 月结号
    pub month_code: Option<String>,
    /// 网点编码
    pub send_site: Option<String>,
    /// 网点名称
    pub send_staff: Option<String>,
    /// 快递业务类型
    pub exp_type: Option<String>,
    /// 快递100电子面单-电子面单账户/月结账号 partnerId [NOT NULL]
    pub kuaidi100_partner_id: String,
    /// 快递100电子面单-电子面单密码 partnerKey [NOT NULL]
    pub kuaidi100_partner_key: String,
    /// 快递100电子面单-电子面单密钥 partnerSecret [NOT NULL]
    pub kuaidi100_partner_secret: String,
    /// 快递100电子面单-收件网点名 net [NOT NULL]
    pub kuaidi100_net: String,
    /// 快递100电子面单-承载快递员名 checkMan [NOT NULL]
    pub kuaidi100_check_man: String,
    /// 快递100电子面单-产品类型 expType [NOT NULL]
    pub kuaidi100_exp_type: String,
    /// 快递100电子面单-主单模板ID tempId（IMAGE/HTML/CLOUD 网点面单必填） [NOT NULL]
    pub kuaidi100_temp_id: String,
    /// 快递100电子面单-子单模板ID childTempId [NOT NULL]
    pub kuaidi100_child_temp_id: String,
    /// 快递100电子面单-回单模板ID backTempId [NOT NULL]
    pub kuaidi100_back_temp_id: String,
    /// 排序
    pub sort_order: Option<i32>,
    /// 是否默认：1默认，0非默认
    pub is_default: Option<i32>,
    /// 是否启用：1启用，0禁用
    pub is_enabled: Option<i32>,
}

impl LogisticsCompanyConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "logistics_company_config";
}


/// 各种邮件的模板配置模板
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MailTemplates {
    /// 邮件模板自增id [NOT NULL]
    pub template_id: i32,
    /// 模板字符串名称，主要用于插件言语包时匹配语言包文件等用途 [NOT NULL]
    pub template_code: String,
    /// 邮件是否是html格式；0，否；1，是 [NOT NULL]
    pub is_html: i32,
    /// 该邮件模板的邮件主题 [NOT NULL]
    pub template_subject: String,
    /// 邮件模板的内容 [NOT NULL]
    pub template_content: String,
    /// 最后一次修改模板的时间 [NOT NULL]
    pub last_modify: i32,
    /// 最近一次发送的时间 [NOT NULL]
    pub last_send: i32,
    /// 该邮件模板的邮件类型；共2个类型；magazine，杂志订阅；template，关注订阅 [NOT NULL]
    pub r#type: String,
}

impl MailTemplates {
    /// 数据库表名。
    pub const TABLE: &'static str = "mail_templates";
}


/// 模板消息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MessageTemplate {
    /// [NOT NULL]
    pub id: i32,
    /// 消息类型ID
    pub message_id: Option<i32>,
    /// 消息类型 1模板消息 2订阅消息 3短信 4站内信 5app 6钉钉
    pub r#type: Option<i32>,
    /// 模板名称
    pub template_name: Option<String>,
    /// 发送对象(暂只用于钉钉)
    pub to_userid: Option<String>,
    /// 模板ID
    pub template_id: Option<String>,
    /// 模板编号
    pub template_num: Option<String>,
    /// 通知内容
    pub content: Option<String>,
    /// 海外短信内容
    pub content_oversea: Option<String>,
}

impl MessageTemplate {
    /// 数据库表名。
    pub const TABLE: &'static str = "message_template";
}


/// 消息类型
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MessageType {
    /// [NOT NULL]
    pub message_id: i32,
    /// 通知类型
    pub name: Option<String>,
    /// 通知描述
    pub describe: Option<String>,
    /// 发送类型  1会员 2商家
    pub send_type: Option<i32>,
    /// 是否支持微信公众号
    pub is_wechat: Option<i32>,
    /// 是否支持小程序
    pub is_mini_program: Option<i32>,
    /// 是否支持站内信
    pub is_message: Option<i32>,
    /// 是否支持短信
    pub is_msg: Option<i32>,
    /// 是否支付app
    pub is_app: Option<i32>,
    /// 是否支持钉钉
    pub is_ding: Option<i32>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 是否支持企业微信 [NOT NULL]
    pub is_wx_work: i32,
    /// 是否支持邮件 [NOT NULL]
    pub is_mail: i32,
}

impl MessageType {
    /// 数据库表名。
    pub const TABLE: &'static str = "message_type";
}


/// 打印机配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PrintConfig {
    /// 自增ID [NOT NULL]
    pub id: i32,
    /// 打印机ID [NOT NULL]
    pub print_id: i32,
    /// 打印模版JSON
    pub template: Option<String>,
    /// 类型 1购物小票
    pub r#type: Option<i32>,
    /// 纸张尺寸（毫米）：58=58mm，80=80mm。历史数据默认 58mm [NOT NULL]
    pub paper_size: i32,
    /// 创建时间
    pub add_time: Option<i32>,
    /// 修改时间
    pub update_time: Option<i32>,
}

impl PrintConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "print_config";
}


/// 地区列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Region {
    /// id [NOT NULL]
    pub region_id: i32,
    /// 层级
    pub level: Option<i32>,
    /// 地区的名字 [NOT NULL]
    pub region_name: String,
    /// 该地区的上一个节点的地区id [NOT NULL]
    pub parent_id: i32,
    /// 是否热门 [NOT NULL]
    pub is_hot: i32,
    /// 排序值，数值越小越靠前 [NOT NULL]
    pub sort_order: i32,
    /// 首字母 [NOT NULL]
    pub first_word: String,
    /// 国家或地区稳定标准代码
    pub standard_code: Option<String>,
}

impl Region {
    /// 数据库表名。
    pub const TABLE: &'static str = "region";
}


/// 违禁词
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SensitiveWords {
    /// [NOT NULL]
    pub id: i32,
    /// 违禁词 [NOT NULL]
    pub word: String,
}

impl SensitiveWords {
    /// 数据库表名。
    pub const TABLE: &'static str = "sensitive_words";
}


/// 运费模板
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingTpl {
    /// id [NOT NULL]
    pub shipping_tpl_id: i32,
    /// 配送时间 [NOT NULL]
    pub shipping_time: String,
    /// 模板名称 [NOT NULL]
    pub shipping_tpl_name: String,
    /// 是否包邮 [NOT NULL]
    pub is_free: i32,
    /// 1：件数，2：重量 [NOT NULL]
    pub pricing_type: i32,
    /// 是否默认 [NOT NULL]
    pub is_default: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl ShippingTpl {
    /// 数据库表名。
    pub const TABLE: &'static str = "shipping_tpl";
}


/// 运费模板内容
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingTplInfo {
    /// [NOT NULL]
    pub id: i32,
    /// 配送类型ID [NOT NULL]
    pub shipping_type_id: i32,
    /// 运费模板ID [NOT NULL]
    pub shipping_tpl_id: i32,
    /// 是否包邮 [NOT NULL]
    pub is_free: i32,
    /// 是否默认 [NOT NULL]
    pub is_default: i32,
    /// [JSON]地区信息，含id和name
    pub region_data: Option<String>,
    /// 起重或起件 [NOT NULL]
    pub start_number: Decimal,
    /// 起价 [NOT NULL]
    pub start_price: Decimal,
    /// 每增重或增价 [NOT NULL]
    pub add_number: Decimal,
    /// 增价 [NOT NULL]
    pub add_price: Decimal,
    /// 满多少包邮件 [NOT NULL]
    pub free_price: Decimal,
    /// 按重还是按件，1:按件，2：按重 [NOT NULL]
    pub pricing_type: i32,
}

impl ShippingTplInfo {
    /// 数据库表名。
    pub const TABLE: &'static str = "shipping_tpl_info";
}


/// 配送类型
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShippingType {
    /// 配送类型id [NOT NULL]
    pub shipping_type_id: i32,
    /// 类型名称 [NOT NULL]
    pub shipping_type_name: String,
    /// 默认发货物流公司ID [NOT NULL]
    pub shipping_default_id: i32,
    /// 描述 [NOT NULL]
    pub shipping_type_desc: String,
    /// 发货时间描述 [NOT NULL]
    pub shipping_time_desc: String,
    /// 是否支持货到付款 [NOT NULL]
    pub is_support_cod: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl ShippingType {
    /// 数据库表名。
    pub const TABLE: &'static str = "shipping_type";
}


/// 税率管理表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct TaxRate {
    /// 税率ID [NOT NULL]
    pub tax_rate_id: i32,
    /// 国家和地区ID [NOT NULL]
    pub region_id: i32,
    /// 国家和地区名称
    pub region_name: Option<String>,
    /// 税率(百分比) [NOT NULL]
    pub rate: Decimal,
    /// [NOT NULL]
    pub is_active: i32,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间(秒级时间戳)
    pub create_time: Option<i64>,
    /// 更新时间(秒级时间戳)
    pub update_time: Option<i64>,
}

impl TaxRate {
    /// 数据库表名。
    pub const TABLE: &'static str = "tax_rate";
}