//! 商品域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `brand`, `category`, `comment`, `e_card`, `e_card_group`, `price_inquiry`, `price_inquiry_reply_record`, `product`, `product_article`, `product_attributes`, `product_attributes_tpl`, `product_auto_status`, `product_cycle`, `product_cycle_spec`, `product_gallery`, `product_group`, `product_inventory_log`, `product_member_price`, `product_org`, `product_package`, `product_presale`, `product_related`, `product_services`, `product_sku`, `product_stock_adjust_record`, `product_stock_allocate_operation_record`, `product_stock_allocate_record`, `product_stock_allocate_record_detail`, `product_stock_record`, `product_stock_record_detail`, `product_style`, `product_style_item`, `product_video`, `stock_warning_rule`, `stock_warning_setting`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 商品品牌表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Brand {
    /// 自增id号 [NOT NULL]
    pub brand_id: i32,
    /// 品牌名称 [NOT NULL]
    pub brand_name: String,
    /// 上传的该公司Logo图片
    pub brand_logo: Option<String>,
    /// 品牌描述
    pub brand_desc: Option<String>,
    /// 否 /	品牌的网址 [NOT NULL]
    pub site_url: String,
    /// [NOT NULL]
    pub sort_order: i32,
    /// 该品牌是否显示;0否1显示 [NOT NULL]
    pub is_show: i32,
    /// 品牌分类
    pub brand_type: Option<String>,
    /// [NOT NULL]
    pub brand_is_hot: i32,
    /// 首字母
    pub first_word: Option<String>,
    /// (已废弃) [NOT NULL]
    pub is_store_brand: i32,
    /// [NOT NULL]
    pub check_status: i32,
    /// 品牌英文名
    pub brand_en_name: Option<String>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub status: i32,
    /// 拒绝原因
    pub reject_remark: Option<String>,
}

impl Brand {
    /// 数据库表名。
    pub const TABLE: &'static str = "brand";
}


/// 商品分类表，记录商品分类信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Category {
    /// 自增id号 [NOT NULL]
    pub category_id: i32,
    /// 分类名称 [NOT NULL]
    pub category_name: String,
    /// SEO标识
    pub seo_identification: Option<String>,
    /// [NOT NULL]
    pub keywords: String,
    /// 分类描述 [NOT NULL]
    pub category_desc: String,
    /// [NOT NULL]
    pub parent_id: i32,
    /// [NOT NULL]
    pub sort_order: i32,
    /// 该分类的计量单位 [NOT NULL]
    pub measure_unit: String,
    /// 是否在前台页面显示 1显示; 0不显示 [NOT NULL]
    pub is_show: i32,
    /// SEO标题 [NOT NULL]
    pub seo_title: String,
    /// 分类短名 [NOT NULL]
    pub short_name: String,
    /// 分类图片 [NOT NULL]
    pub category_pic: String,
    /// 分类图标 [NOT NULL]
    pub category_ico: String,
    /// 是否热门 [NOT NULL]
    pub is_hot: i32,
    /// 搜索关键字 [NOT NULL]
    pub search_keywords: String,
}

impl Category {
    /// 数据库表名。
    pub const TABLE: &'static str = "category";
}


/// 用户对文章和产品的评论列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Comment {
    /// 用户评论的自增id [NOT NULL]
    pub comment_id: i32,
    /// [NOT NULL]
    pub user_id: i32,
    /// [NOT NULL]
    pub username: String,
    /// 是否匿名评价；0-否，1-是 [NOT NULL]
    pub is_anonymous: i32,
    /// 会员头像 [NOT NULL]
    pub avatar: String,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 订单id [NOT NULL]
    pub order_id: i32,
    /// 订单商品itemId [NOT NULL]
    pub order_item_id: i32,
    /// 星级，5代表5星 [NOT NULL]
    pub comment_rank: i32,
    /// 评论的内容 [NOT NULL]
    pub content: String,
    /// 评论的时间 [NOT NULL]
    pub add_time: i32,
    /// 状态，0：待审核，1：审核通过，2：无效 [NOT NULL]
    pub status: i32,
    /// 是否为回复 [NOT NULL]
    pub parent_id: i32,
    /// 评价有用 [NOT NULL]
    pub usefull: i32,
    /// 评价无用 [NOT NULL]
    pub useless: i32,
    /// 【JSON】标签[string]
    pub comment_tag: Option<String>,
    pub show_pics: Option<String>,
    /// 是否推荐
    pub is_recommend: Option<i32>,
    /// 是否置顶 [NOT NULL]
    pub is_top: i32,
    /// 是否已晒单 [NOT NULL]
    pub is_showed: i32,
    /// 系统默认评价 [NOT NULL]
    pub is_default: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 所属店铺 [NOT NULL]
    pub shop_id: i32,
}

impl Comment {
    /// 数据库表名。
    pub const TABLE: &'static str = "comment";
}


/// 电子卡券表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ECard {
    /// 卡券分组id [NOT NULL]
    pub card_id: i32,
    /// 卡券分组id [NOT NULL]
    pub group_id: i32,
    /// 卡号 [NOT NULL]
    pub card_number: String,
    /// 卡号密码 [NOT NULL]
    pub card_pwd: String,
    /// [NOT NULL]
    pub is_use: i32,
    /// 订单id [NOT NULL]
    pub order_id: i32,
    /// 订单item_id [NOT NULL]
    pub order_item_id: i32,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 更新时间
    pub up_time: Option<i32>,
}

impl ECard {
    /// 数据库表名。
    pub const TABLE: &'static str = "e_card";
}


/// 电子卡券分组表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ECardGroup {
    /// 卡券分组id [NOT NULL]
    pub group_id: i32,
    /// 分组名称 [NOT NULL]
    pub group_name: String,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 备注
    pub remark: Option<String>,
    /// [NOT NULL]
    pub is_use: i32,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 更新时间
    pub up_time: Option<i32>,
}

impl ECardGroup {
    /// 数据库表名。
    pub const TABLE: &'static str = "e_card_group";
}


/// 商品询价表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PriceInquiry {
    /// [NOT NULL]
    pub id: i32,
    /// 联系电话 [NOT NULL]
    pub mobile: String,
    /// 联系邮箱 [NOT NULL]
    pub email: String,
    /// 需求信息 [NOT NULL]
    pub content: String,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 回复备注 [NOT NULL]
    pub remark: String,
    /// 回复状态：0 未回复 1 已回复 [NOT NULL]
    pub status: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 用户侧是否删除：0否 1是 [NOT NULL]
    pub user_is_del: i32,
    /// 创建时间
    pub create_time: Option<i32>,
}

impl PriceInquiry {
    /// 数据库表名。
    pub const TABLE: &'static str = "price_inquiry";
}


/// 询价回复记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PriceInquiryReplyRecord {
    /// [NOT NULL]
    pub id: i32,
    /// 询价id [NOT NULL]
    pub inquiry_id: i32,
    /// 回复类型：1商家回复 2用户回复 [NOT NULL]
    pub reply_type: i32,
    /// 回复人名称 [NOT NULL]
    pub reply_name: String,
    /// 回复内容 [NOT NULL]
    pub content: String,
    /// 后台管理员id [NOT NULL]
    pub admin_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 回复时间 [NOT NULL]
    pub create_time: i32,
}

impl PriceInquiryReplyRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "price_inquiry_reply_record";
}


/// 商品表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Product {
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 商品的名称 [NOT NULL]
    pub product_name: String,
    /// SEO标识
    pub seo_identification: Option<String>,
    /// 商品的唯一货号 [NOT NULL]
    pub product_sn: String,
    /// 条形编码
    pub product_tsn: Option<String>,
    /// 商品库存数量 [NOT NULL]
    pub product_stock: i32,
    /// 本店售价 [NOT NULL]
    pub product_price: Decimal,
    /// 市场售价 [NOT NULL]
    pub market_price: Decimal,
    /// 运费模板id [NOT NULL]
    pub shipping_tpl_id: i32,
    /// 该商品状态，1，上架；0，下架 [NOT NULL]
    pub product_status: i32,
    /// 商品类型 1实物商品 2虚拟商品 3卡密商品 4付费内容 5预约到店 6预约上门
    pub product_type: Option<i32>,
    /// 交付模式：1-单次交付，2-多次交付（周期购） [NOT NULL]
    pub delivery_mode: i32,
    /// 销售模式：0-现货销售，1-全款预售，2-定金预售 [NOT NULL]
    pub sale_mode: i32,
    /// 商品所属商品分类id，取值category的cat_id [NOT NULL]
    pub category_id: i32,
    /// 品牌id，取值于brand 的brand_id [NOT NULL]
    pub brand_id: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 商品关键字，放在商品页的关键字中，为搜索引擎收录用
    pub keywords: Option<String>,
    /// 店铺分类id [NOT NULL]
    pub shop_category_id: i32,
    /// [NOT NULL]
    pub check_status: i32,
    /// 审核失败原因 [NOT NULL]
    pub check_reason: String,
    /// 商品点击数 [NOT NULL]
    pub click_count: i32,
    /// 商品的重量，以千克为单位 [NOT NULL]
    pub product_weight: Decimal,
    /// 单件商品体积，单位 m³（v5.8.61）
    pub product_volume: Option<Decimal>,
    /// 是否特价促销；0，否；1，是 [NOT NULL]
    pub is_promote: i32,
    /// 是否参加了秒杀 [NOT NULL]
    pub is_promote_activity: i32,
    /// 促销价格 [NOT NULL]
    pub promote_price: Decimal,
    /// 促销价格开始日期 [NOT NULL]
    pub promote_start_date: i32,
    /// 促销价格结束日期 [NOT NULL]
    pub promote_end_date: i32,
    /// 最大秒杀库存 [NOT NULL]
    pub seckill_max_num: i32,
    /// 商品的简短描述 [NOT NULL]
    pub product_brief: String,
    /// 商品的详细描述
    pub product_desc: Option<String>,
    /// 商品的实际大小图片，如进入该商品页时介绍商品属性所显示的大图片 [NOT NULL]
    pub pic_url: String,
    /// 商品在前台显示的微缩图片，如在分类筛选时显示的小图片 [NOT NULL]
    pub pic_thumb: String,
    /// 应该是上传的商品的原始图片 [NOT NULL]
    pub pic_original: String,
    /// 高清图片url [NOT NULL]
    pub pic_large: String,
    /// 标签
    pub comment_tag: Option<String>,
    /// 是否包邮 [NOT NULL]
    pub free_shipping: i32,
    /// 购买该商品可以使用的积分数量，估计应该是用积分代替金额消费；但程序好像还没有实现该功能 [NOT NULL]
    pub integral: i32,
    /// 商品的添加时间 [NOT NULL]
    pub add_time: i32,
    /// 商品的显示顺序 [NOT NULL]
    pub sort_order: i32,
    /// 店铺的排序 [NOT NULL]
    pub store_sort_order: i32,
    /// 商品是否已经删除，0，否；1，已删除 [NOT NULL]
    pub is_delete: i32,
    /// 是否是精品:0，否；1，是 [NOT NULL]
    pub is_best: i32,
    /// 是否是新品:0，否；1，是 [NOT NULL]
    pub is_new: i32,
    /// 是否热销:0，否；1，是 [NOT NULL]
    pub is_hot: i32,
    /// 最近一次更新商品配置的时间 [NOT NULL]
    pub last_update: i32,
    /// 商品的商家备注，仅商家可见 [NOT NULL]
    pub remark: String,
    /// 购买该商品时每笔成功交易赠送的积分数量 [NOT NULL]
    pub give_integral: i32,
    /// 等级积分 [NOT NULL]
    pub rank_integral: i32,
    /// 供应商id
    pub suppliers_id: Option<i32>,
    /// 虚拟销量 [NOT NULL]
    pub virtual_sales: i32,
    /// 限购数量 [NOT NULL]
    pub limit_number: i32,
    /// 温馨提示
    pub product_care: Option<String>,
    /// [JSON]关联的推荐商品Id
    pub product_related: Option<String>,
    /// [JSON]服务项ids
    pub product_service_ids: Option<String>,
    /// 是否支持无理由退货 [NOT NULL]
    pub is_support_return: i32,
    /// [NOT NULL]
    pub is_support_cod: i32,
    /// 商品视频
    pub product_video: Option<String>,
    /// 预付金额
    pub prepay_price: Option<Decimal>,
    /// 关联的电子卡券分组 [NOT NULL]
    pub card_group_id: i32,
    /// 虚拟商品
    pub virtual_sample: Option<String>,
    /// 付费内容
    pub paid_content: Option<String>,
    /// 是否无需配送；0-否，1-是
    pub no_shipping: Option<i32>,
    /// 运费类型1固定运费2运费模板
    pub fixed_shipping_type: Option<i32>,
    /// 固定运费金额
    pub fixed_shipping_fee: Option<Decimal>,
    /// 供应商产品表id
    pub vendor_product_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 到店自提模板 ID
    pub shop_pickup_tpl_id: Option<i64>,
    /// 是否到店自提；0-否，1-是 [NOT NULL]
    pub is_shop_pickup: i32,
    /// 是否物流发货0-否，1-是 [NOT NULL]
    pub is_logistics: i32,
    /// 是否门店配送0-否，1-是 [NOT NULL]
    pub is_shop_delivery: i32,
    /// 预约模板Id
    pub reservation_tpl_id: Option<i32>,
    /// 成本价
    pub cost_price: Option<Decimal>,
    /// 商品单位
    pub product_unit: Option<String>,
    /// 供应商商品是否需要重新导入
    pub is_need_re_import_vendor_product: Option<i32>,
    /// 礼品卡组id
    pub gift_card_groups_id: Option<i32>,
    /// 属性模板id
    pub attributes_tpl_id: Option<i32>,
    /// 分享信息（JSON：productName/productPrice/picUrl）
    pub share_info: Option<String>,
    /// 配送员服务费类型（平台层，仅单规格商品使用）：0=固定金额 1=按比例 [NOT NULL]
    pub delivery_service_fee_type: i32,
    /// 配送员服务费值（平台层，仅单规格商品使用）：固定金额(元)或按比例(%) [NOT NULL]
    pub delivery_service_fee_value: Decimal,
    /// 服务员服务费类型（平台层，仅单规格商品使用）：0=固定金额 1=按比例 [NOT NULL]
    pub server_service_fee_type: i32,
    /// 服务员服务费值（平台层，仅单规格商品使用）：固定金额(元)或按比例(%) [NOT NULL]
    pub server_service_fee_value: Decimal,
}

impl Product {
    /// 数据库表名。
    pub const TABLE: &'static str = "product";

    /// 是否为无需库存的虚拟类商品。
    ///
    /// 商品类型：2 虚拟商品、3 卡密商品、4 付费内容、7/8 礼品卡。
    /// 这些商品在库中 `product_stock` 可能为 0，但不能按售罄处理。
    #[must_use]
    pub fn is_virtual_product(&self) -> bool {
        matches!(self.product_type, Some(2 | 3 | 4 | 7 | 8))
    }
}


/// 文章关联产品表 
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductArticle {
    /// 商品id，取自goods的goods_id [NOT NULL]
    pub goods_id: i32,
    /// 否 / 0 	文章id，取自 article 的article_id [NOT NULL]
    pub article_id: i32,
}

impl ProductArticle {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_article";
}


/// 商品属性表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductAttributes {
    /// 自增ID号 [NOT NULL]
    pub attributes_id: i32,
    /// 取值于goods的goods_id [NOT NULL]
    pub product_id: i32,
    /// 属性类型，0:普通属性，1：规格属性，2：附加规格 [NOT NULL]
    pub attr_type: i32,
    /// 属性所属类型的名称 [NOT NULL]
    pub attr_name: String,
    /// 具体属性的值 [NOT NULL]
    pub attr_value: String,
    /// 属性价格
    pub attr_price: Option<Decimal>,
    /// 属性颜色
    pub attr_color: Option<String>,
    /// 属性图片
    pub attr_pic: Option<String>,
    /// 属性缩略图
    pub attr_pic_thumb: Option<String>,
    /// 排序
    pub sort_order: Option<i32>,
}

impl ProductAttributes {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_attributes";
}


/// 商品属性模板表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductAttributesTpl {
    /// 自增ID号 [NOT NULL]
    pub tpl_id: i32,
    /// 属性模板名称 [NOT NULL]
    pub tpl_name: String,
    /// 属性内容 [NOT NULL]
    pub tpl_data: String,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 关联商品类目ID
    pub category_id: Option<i32>,
}

impl ProductAttributesTpl {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_attributes_tpl";
}


/// 商品自动上下架表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductAutoStatus {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// 门店ID：0-平台商品，>0-门店商品 [NOT NULL]
    pub shop_id: i32,
    /// 操作类型：0-下架，1-上架 [NOT NULL]
    pub operation_type: i32,
    /// 计划执行时间（时间戳） [NOT NULL]
    pub scheduled_time: i64,
    /// 实际执行时间（时间戳）
    pub execute_time: Option<i64>,
    /// 执行状态：0-待执行，1-执行成功，2-执行失败 [NOT NULL]
    pub status: i32,
    /// 失败原因
    pub error_message: Option<String>,
    /// 重试次数 [NOT NULL]
    pub retry_count: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 是否删除：0-未删除，1-已删除 [NOT NULL]
    pub is_deleted: i32,
}

impl ProductAutoStatus {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_auto_status";
}


/// 商品周期购配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductCycle {
    /// 周期购ID [NOT NULL]
    pub cycle_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// 交付模式类型：1-按计划配送，2-按需兑换（次卡） [NOT NULL]
    pub delivery_type: i32,
    /// 兑换总次数（次卡模式，固定次数）
    pub redeem_total_times: Option<i32>,
    /// 生效时间类型：1-立即，2-次日，3-延迟
    pub effective_type: Option<i32>,
    /// 生效延迟数值
    pub effective_delay_value: Option<i32>,
    /// 生效延迟单位：1-天，2-小时
    pub effective_delay_unit: Option<i32>,
    /// 有效期类型：1-长期，2-当天，3-N天内，4-固定区间
    pub validity_type: Option<i32>,
    /// 有效期数值（N天/小时内有效时使用）
    pub validity_value: Option<i32>,
    /// 有效期单位：1-天，2-小时（validityType=3时使用）
    pub validity_unit: Option<i32>,
    /// 有效期开始时间（固定区间时使用）
    pub validity_start_date: Option<String>,
    /// 有效期结束时间（固定区间时使用）
    pub validity_end_date: Option<String>,
    /// 设置模式：1-统一设置，2-SKU单独设置 [NOT NULL]
    pub setting_mode: i32,
    /// [NOT NULL]
    pub cycle_specs: String,
    /// 每期次数 [NOT NULL]
    pub times_per_period: i32,
    /// 配送周期类型：1-每日一次，2-每周一次，3-每月一次，4-多周一次，5-多月一次，6-每周多次，7-每月多次 [NOT NULL]
    pub cycle_type: i32,
    /// 周期间隔（多周一次/多月一次时使用）
    pub cycle_interval: Option<i32>,
    /// 每周期期数（每周多次/每月多次时使用）
    pub times_per_cycle: Option<i32>,
    pub delivery_weekdays: Option<String>,
    pub delivery_days: Option<String>,
    /// [NOT NULL]
    pub delivery_time_slots: String,
    /// 截单提前天数（买家需提前X天下单） [NOT NULL]
    pub order_cutoff_days: i32,
    /// 截单时间点（如：10:35，在此时间前下单才能最近配送）
    pub order_cutoff_time: Option<String>,
    /// 状态：0-禁用，1-启用 [NOT NULL]
    pub status: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl ProductCycle {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_cycle";
}


/// 商品周期购规格明细表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductCycleSpec {
    /// 规格明细ID [NOT NULL]
    pub cycle_spec_id: i32,
    /// 关联周期购配置ID [NOT NULL]
    pub cycle_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// SKU ID（统一设置时为NULL）
    pub sku_id: Option<i32>,
    /// 周期规格（期数） [NOT NULL]
    pub period_num: i32,
    /// 周期规格名称 [NOT NULL]
    pub period_name: String,
    /// 次卡展示名称
    pub display_name: Option<String>,
    /// 划线价 [NOT NULL]
    pub original_price: Decimal,
    /// 周期价（实际售价） [NOT NULL]
    pub cycle_price: Decimal,
    /// 禁用状态：0-启用，1-禁用（SKU单独设置时，表示该SKU不支持此周期规格） [NOT NULL]
    pub disabled: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl ProductCycleSpec {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_cycle_spec";
}


/// 商品相册表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductGallery {
    /// 商品相册ID [NOT NULL]
    pub pic_id: i32,
    /// 图片属性商品的id [NOT NULL]
    pub product_id: i32,
    /// 商品的实际大小图片，如进入该商品页时介绍商品属性所显示的大图片 [NOT NULL]
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
    /// 门店ID，0=平台 [NOT NULL]
    pub shop_id: i64,
}

impl ProductGallery {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_gallery";
}


/// 商品分组表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductGroup {
    /// [NOT NULL]
    pub product_group_id: i32,
    /// 分组名称
    pub product_group_name: Option<String>,
    /// 分组别名
    pub product_group_sn: Option<String>,
    /// 分组描述
    pub product_group_description: Option<String>,
    /// 商品IDjson集合
    pub product_ids: Option<String>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 店铺id
    pub shop_id: Option<i32>,
}

impl ProductGroup {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_group";
}


/// 库存日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductInventoryLog {
    /// [NOT NULL]
    pub log_id: i32,
    /// 商品id
    pub product_id: Option<i32>,
    /// 货品ID
    pub spec_id: Option<i32>,
    /// 剩余库存
    pub number: Option<i32>,
    /// 修改时间
    pub add_time: Option<i32>,
    /// 原始库存量
    pub old_number: Option<i32>,
    /// 1增2减
    pub r#type: Option<i32>,
    /// 变化量
    pub change_number: Option<i32>,
    /// 描述
    pub desc: Option<String>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl ProductInventoryLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_inventory_log";
}


/// 商品会员价格
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductMemberPrice {
    /// 折扣价自增id [NOT NULL]
    pub price_id: i32,
    /// 商品的id [NOT NULL]
    pub product_id: i32,
    /// 会员等级id [NOT NULL]
    pub user_rank: i32,
    /// 指定商品对指定会员等级的固定定价价格，单位元 [NOT NULL]
    pub user_price: Decimal,
}

impl ProductMemberPrice {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_member_price";
}


/// 商品组织分配
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductOrg {
    /// ID [NOT NULL]
    pub product_org_id: i64,
    /// 商品ID [NOT NULL]
    pub product_id: i64,
    /// 分配组织类型；1-门店，2-区域 [NOT NULL]
    pub org_type: i32,
    /// 分配组织 ID [NOT NULL]
    pub org_id: i64,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
}

impl ProductOrg {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_org";
}


/// 拍卖活动和夺宝奇兵活动配置信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductPackage {
    /// 组合套餐id [NOT NULL]
    pub package_id: i32,
    /// 促销活动的名称 [NOT NULL]
    pub package_name: String,
    /// 促销活动的描述 [NOT NULL]
    pub package_desc: String,
    /// [NOT NULL]
    pub package_type: i32,
    /// 参加活动的id，取值于goods的goods_id [NOT NULL]
    pub product_id: i32,
    /// 活动开始时间 [NOT NULL]
    pub start_time: i32,
    /// 活动结束时间 [NOT NULL]
    pub end_time: i32,
    /// 活动是否结束，0，结束；1，未结束 [NOT NULL]
    pub is_finished: i32,
    /// [NOT NULL]
    pub ext_info: String,
    /// [NOT NULL]
    pub store_id: i32,
}

impl ProductPackage {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_package";
}


/// 商品预售信息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductPresale {
    /// 预售ID [NOT NULL]
    pub presale_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// 销售模式：1-全款预售，2-定金预售 [NOT NULL]
    pub sale_mode: i32,
    /// 定金收取方式：1-按百分比，2-按固定金额（仅定金预售）
    pub deposit_type: Option<i32>,
    /// 定金比例：1-99（仅按百分比）
    pub deposit_ratio: Option<i32>,
    /// 是否向下取整：0-否，1-是（仅按百分比）
    pub is_round_down: Option<i32>,
    /// 固定定金金额（仅按固定金额）
    pub deposit_fixed_amount: Option<Decimal>,
    /// 尾款支付时间类型：1-相对时间，2-固定时间（仅用于展示）
    pub final_payment_type: Option<i32>,
    /// 支付定金后N天开始支付尾款（相对时间）
    pub final_payment_relative_days: Option<i32>,
    /// 尾款支付期限-天（相对时间）
    pub final_payment_duration_days: Option<i32>,
    /// 尾款支付期限-小时（相对时间）
    pub final_payment_duration_hours: Option<i32>,
    /// 尾款支付开始时间（固定时间）
    pub final_payment_start_time: Option<i32>,
    /// 尾款支付结束时间（固定时间）
    pub final_payment_end_time: Option<i32>,
    /// 发货时间类型：1-相对时间，2-指定日期 [NOT NULL]
    pub delivery_type: i32,
    /// 支付全款后N天发货（相对时间）
    pub delivery_relative_days: Option<i32>,
    /// 支付全款后N小时发货（相对时间）
    pub delivery_relative_hours: Option<i32>,
    /// 发货日期（指定日期）
    pub delivery_date: Option<i32>,
    /// 状态：0-禁用，1-启用 [NOT NULL]
    pub status: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl ProductPresale {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_presale";
}


/// 关联商品信息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductRelated {
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 被关联的商品的id [NOT NULL]
    pub related_product_id: i32,
}

impl ProductRelated {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_related";
}


/// 商品服务
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductServices {
    /// [NOT NULL]
    pub product_service_id: i32,
    /// 服务名称 [NOT NULL]
    pub product_service_name: String,
    /// 服务描述 [NOT NULL]
    pub product_service_desc: String,
    /// 图标 [NOT NULL]
    pub ico_img: String,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// 默认开启 [NOT NULL]
    pub default_on: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl ProductServices {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_services";
}


/// 商品属性
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductSku {
    /// 规格id [NOT NULL]
    pub sku_id: i32,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 商品规格值 [NOT NULL]
    pub sku_value: String,
    /// 商品规格值（json）
    pub sku_data: Option<String>,
    /// 货品编码
    pub sku_sn: Option<String>,
    /// 货品库存
    pub sku_stock: Option<i32>,
    /// 条形码
    pub sku_tsn: Option<String>,
    /// 单件规格重量，单位 kg；NULL 沿用商品重量（v5.8.61）
    pub sku_weight: Option<Decimal>,
    /// 单件规格体积，单位 m³；NULL 沿用商品体积（v5.8.61）
    pub sku_volume: Option<Decimal>,
    /// 货品价格
    pub sku_price: Option<Decimal>,
    /// 是否启用：0-禁用，1-启用 [NOT NULL]
    pub is_enabled: i32,
    /// 市场售价
    pub market_price: Option<Decimal>,
    /// 成本价
    pub cost_price: Option<Decimal>,
    /// 配送员服务费类型：0=固定金额 1=按比例 [NOT NULL]
    pub delivery_service_fee_type: i32,
    /// 配送员服务费值：固定金额(元)或按比例(%) [NOT NULL]
    pub delivery_service_fee_value: Decimal,
    /// 服务员服务费类型：0=固定金额 1=按比例 [NOT NULL]
    pub server_service_fee_type: i32,
    /// 服务员服务费值：固定金额(元)或按比例(%) [NOT NULL]
    pub server_service_fee_value: Decimal,
    /// 供应商商品规格 ID
    pub vendor_product_sku_id: Option<i32>,
}

impl ProductSku {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_sku";
}


/// 商品库存调整记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStockAdjustRecord {
    /// 主键 [NOT NULL]
    pub adjust_record_id: i32,
    /// 商品规格id
    pub sku_id: Option<i32>,
    /// 调整类型（1=增量更新，0=覆盖更新）
    pub adjust_type: Option<i32>,
    /// 调整数量
    pub quantity: Option<i32>,
    /// 操作账号
    pub add_by: Option<String>,
    /// 创建时间
    pub add_time: Option<i64>,
    /// 商品id
    pub product_id: Option<i32>,
    /// 门店id
    pub shop_id: Option<i32>,
    /// 调整前库存
    pub change_before_quantity: Option<i32>,
    /// 调整后库存
    pub change_after_quantity: Option<i32>,
}

impl ProductStockAdjustRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_stock_adjust_record";
}


/// 调拨操作记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStockAllocateOperationRecord {
    /// 调拨操作记录id [NOT NULL]
    pub allocate_operation_id: i32,
    /// 调拨记录id
    pub allocate_id: Option<i32>,
    /// 操作描述
    pub operation_type: Option<String>,
    /// 操作时间
    pub add_time: Option<i64>,
    /// 审核驳回原因
    pub review_rejected_reason: Option<String>,
    /// 操作人
    pub add_by: Option<String>,
}

impl ProductStockAllocateOperationRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_stock_allocate_operation_record";
}


/// 库存调拨表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStockAllocateRecord {
    /// 调拨主键 [NOT NULL]
    pub allocate_id: i32,
    /// 调拨单号
    pub allocate_code: Option<String>,
    /// 出库组织
    pub outbound_organization_id: Option<i32>,
    /// 入库组织
    pub warehousing_organization_id: Option<i32>,
    /// 单据状态（1=待审核，2待调出，3=已完成，4=已驳回，5=已作废）
    pub status: Option<i32>,
    /// 经办人
    pub add_by: Option<String>,
    /// 创建时间
    pub add_time: Option<i64>,
    /// 入库备注
    pub stock_in_comment: Option<String>,
    /// 出库备注
    pub stock_out_comment: Option<String>,
    /// 调拨备注
    pub stock_allocate_comment: Option<String>,
}

impl ProductStockAllocateRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_stock_allocate_record";
}


/// 商品调拨详情
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStockAllocateRecordDetail {
    /// 调拨详情主键 [NOT NULL]
    pub allocate_detail_id: i32,
    /// 调拨单主键
    pub allocate_id: Option<i32>,
    /// 商品id
    pub product_id: Option<i32>,
    /// 商品sku
    pub sku_id: Option<i32>,
    /// 调拨数量
    pub quantity: Option<i32>,
    /// 实际入库数量
    pub stock_in_quantity: Option<i32>,
    /// 实际出库数量
    pub stock_out_quantity: Option<i32>,
}

impl ProductStockAllocateRecordDetail {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_stock_allocate_record_detail";
}


/// 商品出入库记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStockRecord {
    /// [NOT NULL]
    pub product_stock_id: i32,
    /// 仓库编号
    pub stock_code: Option<String>,
    /// 类型（1=入库，2=出库） [NOT NULL]
    pub in_or_out: i32,
    /// 11=采购入库，12=退货入库，10=其他入库，21=过期出库，22=订单出库，23=其他出库
    pub r#type: Option<i32>,
    /// 创建时间
    pub add_time: Option<i32>,
    /// 经办人
    pub add_by: Option<String>,
    /// 备注
    pub comment: Option<String>,
    /// 门店id
    pub shop_id: Option<i32>,
}

impl ProductStockRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_stock_record";
}


/// 商品库存记录明细表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStockRecordDetail {
    /// 主键 [NOT NULL]
    pub record_detail_id: i32,
    /// 库存记录id
    pub stock_record_id: Option<i32>,
    /// 商品id
    pub product_id: Option<i32>,
    /// 商品规格id
    pub sku_id: Option<i32>,
    /// 出入库数量
    pub quantity: Option<i32>,
}

impl ProductStockRecordDetail {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_stock_record_detail";
}


/// 商品样式配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStyle {
    /// 样式ID [NOT NULL]
    pub style_id: i32,
    /// 样式名称 [NOT NULL]
    pub style_name: String,
    /// 样式类型 1=边框背景 2=活动背景 [NOT NULL]
    pub style_type: i32,
    /// 样式图片URL [NOT NULL]
    pub style_image: String,
    /// PC端样式图片URL [NOT NULL]
    pub pc_image: String,
    /// 生效开始时间（活动背景必填） [NOT NULL]
    pub start_time: i32,
    /// 生效结束时间（活动背景必填） [NOT NULL]
    pub end_time: i32,
    /// 状态 0=禁用 1=启用 [NOT NULL]
    pub status: i32,
    /// 店铺ID，0=平台 [NOT NULL]
    pub shop_id: i32,
    /// 创建时间 [NOT NULL]
    pub add_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
    /// 是否删除 0=否 1=是 [NOT NULL]
    pub is_delete: i32,
}

impl ProductStyle {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_style";
}


/// 商品样式关联表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductStyleItem {
    /// 关联ID [NOT NULL]
    pub item_id: i32,
    /// 样式ID [NOT NULL]
    pub style_id: i32,
    /// 关联类型 1=商品 2=分组 3=品牌 [NOT NULL]
    pub item_type: i32,
    /// 关联目标ID（商品ID/分组ID/品牌ID） [NOT NULL]
    pub ref_id: i32,
    /// 创建时间 [NOT NULL]
    pub add_time: i32,
}

impl ProductStyleItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_style_item";
}


/// 产品视频关联表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductVideo {
    /// 主键id [NOT NULL]
    pub id: i32,
    /// 商品视频ID [NOT NULL]
    pub video_id: i32,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 视频url [NOT NULL]
    pub video_url: String,
    /// 视频封面 [NOT NULL]
    pub video_cover: String,
    /// 视频格式后缀 [NOT NULL]
    pub format: String,
}

impl ProductVideo {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_video";
}


/// 库存预警规则（商品/SKU 维度上下限）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StockWarningRule {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// 归属主体：0=平台自营，>0=店铺或门店（shop.shop_id） [NOT NULL]
    pub shop_id: i32,
    /// product.product_id [NOT NULL]
    pub product_id: i32,
    /// product_sku.sku_id；0=单规格商品 [NOT NULL]
    pub sku_id: i32,
    /// 预警开关：0=关闭，1=开启；关闭时保留已填上下限 [NOT NULL]
    pub warn_enabled: i32,
    /// 库存下限：实时库存小于该值为低库存 [NOT NULL]
    pub lower_limit: i32,
    /// 库存上限：实时库存大于该值为高库存 [NOT NULL]
    pub upper_limit: i32,
    /// 最近一次预警状态：0=正常，1=低库存，2=高库存，3=无货；用于防重复发送 [NOT NULL]
    pub last_warn_status: i32,
    /// 最近一次预警发送时间（Unix秒） [NOT NULL]
    pub last_warn_time: i64,
    /// 逻辑删除：0=有效，1=删除 [NOT NULL]
    pub is_delete: i32,
    /// 创建时间（Unix秒） [NOT NULL]
    pub create_time: i64,
    /// 更新时间（Unix秒） [NOT NULL]
    pub update_time: i64,
}

impl StockWarningRule {
    /// 数据库表名。
    pub const TABLE: &'static str = "stock_warning_rule";
}


/// 库存预警通知总开关（按经营主体隔离）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StockWarningSetting {
    /// 归属主体：0=平台自营，>0=店铺或门店（shop.shop_id） [NOT NULL]
    pub shop_id: i32,
    /// 库存预警通知总开关：0=关闭，1=开启 [NOT NULL]
    pub notify_enabled: i32,
    /// 创建时间（Unix秒） [NOT NULL]
    pub create_time: i64,
    /// 更新时间（Unix秒） [NOT NULL]
    pub update_time: i64,
}

impl StockWarningSetting {
    /// 数据库表名。
    pub const TABLE: &'static str = "stock_warning_setting";
}