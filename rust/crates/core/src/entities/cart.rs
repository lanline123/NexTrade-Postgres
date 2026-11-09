//! 购物车实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `cart`, `cart_global_change_price`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 购物车购物信息记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Cart {
    /// 自增id号 [NOT NULL]
    pub cart_id: i32,
    /// 用户登录ID;取自session [NOT NULL]
    pub user_id: i32,
    /// 商品id [NOT NULL]
    pub product_id: i32,
    /// 货号 [NOT NULL]
    pub product_sn: String,
    /// 商品缩略图
    pub pic_thumb: Option<String>,
    /// 市场价
    pub market_price: Option<Decimal>,
    /// 添加时的价格 [NOT NULL]
    pub original_price: Decimal,
    /// 数量 [NOT NULL]
    pub quantity: i32,
    /// 规格ID
    pub sku_id: Option<i32>,
    pub sku_data: Option<String>,
    /// 商品类型，1：实体，2：虚拟 [NOT NULL]
    pub product_type: i32,
    /// 是否选中 [NOT NULL]
    pub is_checked: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub r#type: i32,
    /// 购物车商品更新时间 [NOT NULL]
    pub update_time: i32,
    /// 分销员 [NOT NULL]
    pub salesman_id: i32,
    pub extra_sku_data: Option<String>,
    /// 扩展数据
    pub extra_data: Option<String>,
    /// 改价数据
    pub change_price_data: Option<String>,
    /// 周期购规格ID（周期购商品时使用）
    pub cycle_spec_id: Option<i32>,
    /// 兑换次卡ID（非空即兑换模式）
    pub redeem_card_id: Option<i32>,
    /// B2B批发:1=批发行 0=零售行 [NOT NULL]
    pub is_wholesale_line: i32,
    /// B2B批发:加车时等级快照(0=无) [NOT NULL]
    pub wholesale_grade_id: i32,
}

impl Cart {
    /// 数据库表名。
    pub const TABLE: &'static str = "cart";
}


/// cart_global_change_price
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CartGlobalChangePrice {
    /// [NOT NULL]
    pub user_id: i32,
    /// [NOT NULL]
    pub change_price_type: i32,
    /// [NOT NULL]
    pub change_price_value: Decimal,
}

impl CartGlobalChangePrice {
    /// 数据库表名。
    pub const TABLE: &'static str = "cart_global_change_price";
}