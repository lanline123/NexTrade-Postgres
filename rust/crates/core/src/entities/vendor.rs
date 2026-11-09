//! 供应商域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `admin_user_vendor`, `vendor`, `vendor_account`, `vendor_account_log`, `vendor_product`, `vendor_product_audit_log`, `vendor_product_gallery`, `vendor_product_sku`, `vendor_product_sku_attr`, `vendor_product_sku_stock_log`, `vendor_product_stock_adjust_record`, `vendor_product_stock_record`, `vendor_product_stock_record_detail`, `vendor_product_video`, `vendor_settlement_order`, `vendor_shop_bind`, `vendor_stock_warning_rule`, `vendor_stock_warning_setting`, `vendor_withdraw`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 供应商员工列表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminUserVendor {
    /// 自增主键 [NOT NULL]
    pub id: i32,
    /// 管理员ID [NOT NULL]
    pub admin_id: i32,
    /// 会员ID [NOT NULL]
    pub user_id: i32,
    /// 供应商ID [NOT NULL]
    pub vendor_id: i32,
    /// 员工名称 [NOT NULL]
    pub username: String,
    /// 邮箱 [NOT NULL]
    pub email: String,
    /// 头像
    pub avatar: Option<String>,
    /// 员工权限 [JSON]
    pub auth_list: Option<String>,
    /// 是否停用：1 是，0 否 [NOT NULL]
    pub is_using: i32,
    /// 是否为管理员：1 是，0 否 [NOT NULL]
    pub is_admin: i32,
    /// 创建时间
    pub add_time: Option<i32>,
    /// 权限组ID [NOT NULL]
    pub role_id: i32,
}

impl AdminUserVendor {
    /// 数据库表名。
    pub const TABLE: &'static str = "admin_user_vendor";
}


/// 供应商表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Vendor {
    /// 自增ID，供应商ID [NOT NULL]
    pub vendor_id: i32,
    /// 来源类型：1入驻供应商，2自建供应商 [NOT NULL]
    pub source_type: i32,
    /// 自建供应商所属组织 [NOT NULL]
    pub shop_id: i32,
    /// 自建供应商编码
    pub vendor_code: Option<String>,
    /// 采购供应商分类ID
    pub purchase_category_id: Option<i64>,
    /// 供应商logo
    pub vendor_logo: Option<String>,
    /// 供应商创建时间
    pub add_time: Option<i32>,
    /// 联系电话
    pub contact_mobile: Option<String>,
    /// 供应商信息JSON
    pub vendor_data: Option<String>,
    /// 个人信息JSON
    pub person_data: Option<String>,
    /// 状态 1开启 2关闭
    pub status: Option<i32>,
    /// 售后审核 0关闭 1开启
    pub after_sales_review: Option<i32>,
    /// 类型 1个人 2企业
    pub r#type: Option<i32>,
    /// 供应商名称
    pub vendor_name: Option<String>,
    /// 客服电话
    pub kefu_phone: Option<String>,
    /// 发货信息JSON
    pub ship_info: Option<String>,
    /// 收货信息JSON
    pub receive_info: Option<String>,
    /// 供应商资金
    pub vendor_money: Option<Decimal>,
    /// 冻结资金
    pub frozen_money: Option<Decimal>,
    /// 简介
    pub description: Option<String>,
    /// 最后登录时间 [NOT NULL]
    pub last_login_time: i32,
    /// 服务费比例
    pub service_fee_rate: Option<Decimal>,
    /// 手续费比例
    pub fee_rate: Option<Decimal>,
    pub self_vendor_name_key: Option<String>,
    /// [NOT NULL]
    pub self_vendor_code_key: String,
}

impl Vendor {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor";
}


/// 供应商账户表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorAccount {
    /// 账号ID [NOT NULL]
    pub account_id: i32,
    /// 供应商ID [NOT NULL]
    pub vendor_id: i32,
    /// 提现账号类型，1：银行卡，2：支付宝，3：微信 [NOT NULL]
    pub account_type: i32,
    /// 姓名 [NOT NULL]
    pub account_name: String,
    /// 银行卡号或账号 [NOT NULL]
    pub account_no: String,
    /// 银行名称 [NOT NULL]
    pub bank_name: String,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 分行名称
    pub bank_branch: Option<String>,
    /// 开户地区ID列表JSON
    pub bank_region_ids: Option<String>,
    /// 付款方式原始值
    pub payment_method: Option<String>,
}

impl VendorAccount {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_account";
}


/// 供应商资金变化
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorAccountLog {
    /// 自增ID [NOT NULL]
    pub vendor_account_log_id: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 供应商资金 [NOT NULL]
    pub vendor_money: Decimal,
    /// 供应商冻结资金 [NOT NULL]
    pub frozen_money: Decimal,
    /// 资金变动类型，例如1-提现 [NOT NULL]
    pub r#type: String,
    /// 备注
    pub remarks: Option<String>,
    /// 现供应商资金 [NOT NULL]
    pub new_vendor_money: Decimal,
    /// 现供应商冻结资金 [NOT NULL]
    pub new_frozen_money: Decimal,
    /// 供应商ID [NOT NULL]
    pub vendor_id: i32,
}

impl VendorAccountLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_account_log";
}


/// 供应商商品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProduct {
    /// ID [NOT NULL]
    pub id: i64,
    /// 商品名称 [NOT NULL]
    pub product_name: String,
    /// 商品品牌ID
    pub product_brand_id: Option<i32>,
    /// 商品类目ID [NOT NULL]
    pub product_category_id: i32,
    /// 商品编码生成类型；1-系统生成，2-手动输入 [NOT NULL]
    pub product_sn_generate_type: i32,
    /// 商品描述
    pub product_brief: Option<String>,
    /// 商品状态；0-断供，1-在售 [NOT NULL]
    pub product_state: i32,
    /// 规格类型，1-单规格，2-多规格 [NOT NULL]
    pub sku_type: i32,
    /// 商品详情
    pub product_desc: Option<String>,
    /// 商品审核状态；0-待审核，1-审核通过，2-审核失败 [NOT NULL]
    pub audit_state: i32,
    /// 商品实际图片 [NOT NULL]
    pub pic_url: String,
    /// 商品缩略图片 [NOT NULL]
    pub pic_thumb: String,
    /// 商品原始图片 [NOT NULL]
    pub pic_original: String,
    /// 高清图片url [NOT NULL]
    pub pic_large: String,
    /// 是否回收；0-否，1-是 [NOT NULL]
    pub is_recycle: i32,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 来源店铺/门店商品ID
    pub source_product_id: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
    /// [NOT NULL]
    pub active_source_product_key: String,
}

impl VendorProduct {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product";
}


/// 供应商商品审核记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductAuditLog {
    /// ID [NOT NULL]
    pub id: i64,
    /// 供应商商品 ID
    pub vendor_product_id: Option<i64>,
    /// 审核状态；0-待审核，1-审核通过，2-审核失败
    pub audit_state: Option<i32>,
    /// 审核失败原因
    pub audit_fail_reason: Option<String>,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl VendorProductAuditLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_audit_log";
}


/// 供应商商品相册
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductGallery {
    /// ID [NOT NULL]
    pub id: i64,
    /// 供应商商品 ID [NOT NULL]
    pub vendor_product_id: i32,
    /// 商品实际图片 [NOT NULL]
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
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl VendorProductGallery {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_gallery";
}


/// 供货商商品规格
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductSku {
    /// ID [NOT NULL]
    pub id: i64,
    /// 供应商商品 ID [NOT NULL]
    pub vendor_product_id: i32,
    /// 来源店铺/门店SKU ID，无规格为0
    pub source_sku_id: Option<i32>,
    /// 最低采购数量 [NOT NULL]
    pub min_purchase_qty: i32,
    /// 商品规格值（拼接） [NOT NULL]
    pub sku_attr_val: String,
    /// 商品规格值（json）
    pub sku_attr_json: Option<String>,
    /// 规格编码
    pub sku_sn: Option<String>,
    /// 规格条形码
    pub sku_tsn: Option<String>,
    /// 规格重量
    pub sku_weight: Option<Decimal>,
    /// 规格库存
    pub sku_stock: Option<i32>,
    /// 供货价 [NOT NULL]
    pub supply_price: Decimal,
    /// 限售金额类型；1-按金额，2-按比例 [NOT NULL]
    pub supply_price_limit_type: i32,
    /// 限售金额值 [NOT NULL]
    pub supply_price_limit_val: Decimal,
    /// 规格销量 [NOT NULL]
    pub sales_volume: i32,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
    /// [NOT NULL]
    pub active_source_sku_key: String,
}

impl VendorProductSku {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_sku";
}


/// 供应商商品规格属性
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductSkuAttr {
    /// ID [NOT NULL]
    pub id: i64,
    /// 供应商商品 ID [NOT NULL]
    pub vendor_product_id: i32,
    /// 属性类型，0-普通属性，1-规格属性，2-附加规格 [NOT NULL]
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
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl VendorProductSkuAttr {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_sku_attr";
}


/// 供货商商品规格库存日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductSkuStockLog {
    /// ID [NOT NULL]
    pub id: i64,
    /// 供货商商品ID [NOT NULL]
    pub vendor_product_id: i32,
    /// 供应商商品规格 ID [NOT NULL]
    pub vendor_product_sku_id: i32,
    /// 操作类型；1-增加，2-减少 [NOT NULL]
    pub operation_type: i32,
    /// 变更前库存 [NOT NULL]
    pub before_stock: i32,
    /// 变动数量 [NOT NULL]
    pub change_num: i32,
    /// 变更后库存 [NOT NULL]
    pub after_stock: i32,
    /// 业务类型；1-商品新增，2-库存编辑 [NOT NULL]
    pub biz_type: i32,
    /// 业务备注
    pub biz_remark: Option<String>,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl VendorProductSkuStockLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_sku_stock_log";
}


/// 供应商商品库存调整记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductStockAdjustRecord {
    /// 主键 [NOT NULL]
    pub adjust_record_id: i32,
    /// 供应商商品id（vendor_product.id）
    pub vendor_product_id: Option<i32>,
    /// 供应商商品规格id（vendor_product_sku.id）
    pub vendor_product_sku_id: Option<i64>,
    /// 调整类型（1=增量更新，0=覆盖更新）
    pub adjust_type: Option<i32>,
    /// 调整数量
    pub quantity: Option<i32>,
    /// 调整前库存
    pub change_before_quantity: Option<i32>,
    /// 调整后库存
    pub change_after_quantity: Option<i32>,
    /// 操作账号
    pub add_by: Option<String>,
    /// 创建时间
    pub add_time: Option<i64>,
    /// 供应商id
    pub vendor_id: Option<i32>,
}

impl VendorProductStockAdjustRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_stock_adjust_record";
}


/// 供应商商品出入库记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductStockRecord {
    /// [NOT NULL]
    pub product_stock_id: i32,
    /// 单据编号
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
    /// 供应商id
    pub vendor_id: Option<i32>,
}

impl VendorProductStockRecord {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_stock_record";
}


/// 供应商商品出入库记录明细
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductStockRecordDetail {
    /// 主键 [NOT NULL]
    pub record_detail_id: i32,
    /// 出入库单id
    pub stock_record_id: Option<i32>,
    /// 供应商商品id（vendor_product.id）
    pub vendor_product_id: Option<i32>,
    /// 供应商商品规格id（vendor_product_sku.id）
    pub vendor_product_sku_id: Option<i64>,
    /// 出入库数量
    pub quantity: Option<i32>,
}

impl VendorProductStockRecordDetail {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_stock_record_detail";
}


/// 供应商商品视频
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorProductVideo {
    /// ID [NOT NULL]
    pub id: i64,
    /// 供货商商品 ID [NOT NULL]
    pub vendor_product_id: i32,
    /// 视频url [NOT NULL]
    pub video_url: String,
    /// 视频封面 [NOT NULL]
    pub video_cover: String,
    /// 视频格式后缀 [NOT NULL]
    pub format: String,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 创建人主键 [NOT NULL]
    pub create_by_id: i64,
    /// 创建人名称
    pub create_by_name: Option<String>,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
    /// 编辑人主键 [NOT NULL]
    pub update_by_id: i64,
    /// 更新人名称 [NOT NULL]
    pub update_by_name: String,
    /// 是否删除；0-否，1-是 [NOT NULL]
    pub is_del: i32,
}

impl VendorProductVideo {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_product_video";
}


/// 供应商结算订单表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorSettlementOrder {
    /// [NOT NULL]
    pub vendor_settlement_order_id: i32,
    /// 订单ID
    pub order_id: Option<i32>,
    /// 店铺ID
    pub shop_id: Option<i32>,
    /// 供应商ID
    pub vendor_id: Option<i32>,
    /// 收益总额
    pub amount: Option<Decimal>,
    /// 生成时间
    pub add_time: Option<i32>,
}

impl VendorSettlementOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_settlement_order";
}


/// 供应商店铺绑定表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorShopBind {
    /// 自增主键 [NOT NULL]
    pub id: i32,
    /// 供应商ID [NOT NULL]
    pub vendor_id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 创建时间
    pub add_time: Option<i32>,
}

impl VendorShopBind {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_shop_bind";
}


/// 供应商库存预警规则
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorStockWarningRule {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// vendor_product.id [NOT NULL]
    pub product_id: i64,
    /// vendor_product_sku.id；单规格商品规则为0 [NOT NULL]
    pub sku_id: i64,
    /// 预警开关：0=关闭，1=开启 [NOT NULL]
    pub warn_enabled: i32,
    /// 库存下限 [NOT NULL]
    pub lower_limit: i32,
    /// 库存上限 [NOT NULL]
    pub upper_limit: i32,
    /// 最近一次预警状态：0=正常，1=低库存，2=高库存，3=无货 [NOT NULL]
    pub last_warn_status: i32,
    /// 最近一次预警时间（Unix秒） [NOT NULL]
    pub last_warn_time: i64,
    /// 逻辑删除：0=有效，1=删除 [NOT NULL]
    pub is_del: i32,
    /// 创建时间（Unix秒） [NOT NULL]
    pub create_time: i64,
    /// 更新时间（Unix秒） [NOT NULL]
    pub update_time: i64,
}

impl VendorStockWarningRule {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_stock_warning_rule";
}


/// 供应商库存预警通知总开关
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorStockWarningSetting {
    /// 供应商 ID [NOT NULL]
    pub vendor_id: i32,
    /// 库存预警通知总开关：0=关闭，1=开启 [NOT NULL]
    pub notify_enabled: i32,
    /// 创建时间（Unix秒） [NOT NULL]
    pub create_time: i64,
    /// 更新时间（Unix秒） [NOT NULL]
    pub update_time: i64,
}

impl VendorStockWarningSetting {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_stock_warning_setting";
}


/// 供应商提现记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct VendorWithdraw {
    /// 提现记录ID [NOT NULL]
    pub vendor_withdraw_log_id: i32,
    /// 所属供应商 [NOT NULL]
    pub vendor_id: i32,
    /// 提现金额 [NOT NULL]
    pub amount: Decimal,
    /// 手续费
    pub handling_fee: Option<Decimal>,
    /// 手续费率
    pub handling_rate: Option<Decimal>,
    /// 状态：0待审核，2审核不通过，3完成，4待打款 [NOT NULL]
    pub status: i32,
    /// 提现时间 [NOT NULL]
    pub add_time: i32,
    /// 提现备注 [NOT NULL]
    pub remark: String,
    /// 提现到账户ID
    pub vendor_account_id: Option<i32>,
    /// 提现时账户信息数据
    pub account_data: Option<String>,
    /// 审核备注
    pub audit_remark: Option<String>,
    /// 打款凭证
    pub payment_voucher: Option<String>,
    /// 提现单号
    pub withdraw_sn: Option<String>,
}

impl VendorWithdraw {
    /// 数据库表名。
    pub const TABLE: &'static str = "vendor_withdraw";
}