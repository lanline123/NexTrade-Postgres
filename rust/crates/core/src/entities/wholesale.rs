//! 批发域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `product_wholesale_config`, `product_wholesale_price`, `product_wholesale_sku_config`, `wholesaler`, `wholesaler_grade`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// B2B批发商品级配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductWholesaleConfig {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// product.product_id [NOT NULL]
    pub product_id: i32,
    /// 归属店铺：0=平台 >0=店铺（随 product.shop_id 冗余） [NOT NULL]
    pub shop_id: i32,
    /// 多规格价格模式：1=商品级混批 2=SKU级独立 [NOT NULL]
    pub multi_spec_price_mode: i32,
    /// 起批维度：1=SPU 2=SKU [NOT NULL]
    pub moq_dimension: i32,
    /// 起批数(moq_dimension=1时为加总起批,=2时为每SKU起批) [NOT NULL]
    pub moq: i32,
    /// 0=禁用 1=启用 [NOT NULL]
    pub status: i32,
    /// 逻辑删除：0=有效 1=删除 [NOT NULL]
    pub is_delete: i32,
    /// 创建时间(Unix秒)
    pub create_time: Option<i64>,
    /// 更新时间(Unix秒)
    pub update_time: Option<i64>,
}

impl ProductWholesaleConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_wholesale_config";
}


/// B2B批发价(SKU×Grade×Tier)
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductWholesalePrice {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// product.product_id [NOT NULL]
    pub product_id: i32,
    /// 批发配置作用域：0=平台，>0=门店 [NOT NULL]
    pub shop_id: i32,
    /// 0=SPU兜底 >0=具体SKU [NOT NULL]
    pub sku_id: i32,
    /// 0=通用价 >0=特定等级 [NOT NULL]
    pub grade_id: i32,
    /// 阶梯起量 [NOT NULL]
    pub tier_min_qty: i32,
    /// 1=折扣换算 2=直接金额 [NOT NULL]
    pub price_mode: i32,
    /// [NOT NULL]
    pub discount: Decimal,
    /// 固定批发价 price_mode=2时用 [NOT NULL]
    pub wholesale_price: Decimal,
    /// 0=禁用 1=启用 [NOT NULL]
    pub status: i32,
    /// 逻辑删除：0=有效 1=删除 [NOT NULL]
    pub is_delete: i32,
    /// 创建时间(Unix秒)
    pub create_time: Option<i64>,
    /// 更新时间(Unix秒)
    pub update_time: Option<i64>,
}

impl ProductWholesalePrice {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_wholesale_price";
}


/// B2B批发SKU级配置(倍数购买)
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductWholesaleSkuConfig {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// product.product_id [NOT NULL]
    pub product_id: i32,
    /// 批发配置作用域：0=平台，>0=门店 [NOT NULL]
    pub shop_id: i32,
    /// product_sku.sku_id [NOT NULL]
    pub sku_id: i32,
    /// 倍数购买(1=不限制) 任何dimension下都按SKU [NOT NULL]
    pub multiple: i32,
    /// 逻辑删除：0=有效 1=删除 [NOT NULL]
    pub is_delete: i32,
    /// 创建时间(Unix秒)
    pub create_time: Option<i64>,
    /// 更新时间(Unix秒)
    pub update_time: Option<i64>,
}

impl ProductWholesaleSkuConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_wholesale_sku_config";
}


/// B2B批发商身份表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Wholesaler {
    /// 批发商ID [NOT NULL]
    pub wholesaler_id: i32,
    /// 关联 user.user_id [NOT NULL]
    pub user_id: i32,
    /// 关联 wholesaler_grade.grade_id [NOT NULL]
    pub grade_id: i32,
    /// 0=禁用 1=启用 [NOT NULL]
    pub status: i32,
    /// 来源：1=注册自动 2=后台导入 3=手动添加 [NOT NULL]
    pub source: i32,
    /// 多店铺隔离（一期固定0） [NOT NULL]
    pub shop_id: i32,
    /// 备注 [NOT NULL]
    pub remark: String,
    /// 逻辑删除：0=有效 1=删除 [NOT NULL]
    pub is_delete: i32,
    /// 创建时间(Unix秒)
    pub create_time: Option<i64>,
    /// 更新时间(Unix秒)
    pub update_time: Option<i64>,
}

impl Wholesaler {
    /// 数据库表名。
    pub const TABLE: &'static str = "wholesaler";
}


/// B2B批发商等级
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct WholesalerGrade {
    /// 等级ID [NOT NULL]
    pub grade_id: i32,
    /// 等级名称 [NOT NULL]
    pub grade_name: String,
    /// 0=禁用 1=启用 [NOT NULL]
    pub status: i32,
    /// 排序值，升序 [NOT NULL]
    pub sort_order: i32,
    /// 备注 [NOT NULL]
    pub remark: String,
    /// 逻辑删除：0=有效 1=删除 [NOT NULL]
    pub is_delete: i32,
    /// 创建时间(Unix秒)
    pub create_time: Option<i64>,
    /// 更新时间(Unix秒)
    pub update_time: Option<i64>,
}

impl WholesalerGrade {
    /// 数据库表名。
    pub const TABLE: &'static str = "wholesaler_grade";
}