//! 分销域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `salesman`, `salesman_config`, `salesman_content`, `salesman_customer`, `salesman_group`, `salesman_material`, `salesman_material_category`, `salesman_order`, `salesman_product`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 分销员
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Salesman {
    /// [NOT NULL]
    pub salesman_id: i32,
    /// 用户id
    pub user_id: Option<i32>,
    /// 等级
    pub level: Option<i32>,
    /// 分组id
    pub group_id: Option<i32>,
    /// 上级分销员 [NOT NULL]
    pub pid: i32,
    pub add_time: Option<i32>,
    /// 所属店铺
    pub shop_id: Option<i32>,
    /// 累计销售额
    pub sale_amount: Option<Decimal>,
}

impl Salesman {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman";
}


/// salesman_config
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanConfig {
    /// [NOT NULL]
    pub id: i32,
    /// 店铺ID
    pub shop_id: Option<i32>,
    pub code: Option<String>,
    pub data: Option<String>,
}

impl SalesmanConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_config";
}


/// 分销内容管理
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanContent {
    /// [NOT NULL]
    pub id: i32,
    /// 标题 [NOT NULL]
    pub title: String,
    /// 封面图片 [NOT NULL]
    pub img: String,
    /// 推送时间 [NOT NULL]
    pub start_time: i32,
    /// 失效时间
    pub end_time: Option<i32>,
    /// 页面描述 [NOT NULL]
    pub describe: String,
    /// 是否置顶：1 是 0 否 [NOT NULL]
    pub is_top: i32,
    /// 正文内容
    pub content: Option<String>,
    /// 是否有效：1 是 0 否
    pub is_available: Option<i32>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 多图
    pub pics: Option<String>,
}

impl SalesmanContent {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_content";
}


/// 分销客户记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanCustomer {
    /// [NOT NULL]
    pub salesman_customer_id: i32,
    pub salesman_id: Option<i32>,
    pub user_id: Option<i32>,
    pub add_time: Option<i32>,
    /// 客户绑定渠道 [NOT NULL]
    pub channel: String,
    /// 分销员对客户的备注 [NOT NULL]
    pub remark: String,
}

impl SalesmanCustomer {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_customer";
}


/// 分销员分组
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanGroup {
    /// [NOT NULL]
    pub group_id: i32,
    /// 分组名称
    pub group_name: Option<String>,
    /// 分组描述
    pub describe: Option<String>,
    pub add_time: Option<i32>,
    /// 所属店铺
    pub shop_id: Option<i32>,
}

impl SalesmanGroup {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_group";
}


/// 分销内容管理
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanMaterial {
    /// [NOT NULL]
    pub id: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 是否置顶：1 是 0 否 [NOT NULL]
    pub is_top: i32,
    /// 正文内容
    pub content: Option<String>,
    /// 是否有效：1 是 0 否
    pub is_available: Option<i32>,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 多图
    pub pics: Option<String>,
    /// 类型
    pub category_id: Option<i32>,
    pub share_num: Option<i32>,
    pub product_id: Option<i32>,
}

impl SalesmanMaterial {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_material";
}


/// 素材分类
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanMaterialCategory {
    /// [NOT NULL]
    pub category_id: i32,
    /// 名称
    pub category_name: Option<String>,
    pub add_time: Option<i32>,
    /// 所属店铺
    pub shop_id: Option<i32>,
    /// 排序
    pub sort_order: Option<i32>,
}

impl SalesmanMaterialCategory {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_material_category";
}


/// 分销业绩订单结算表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanOrder {
    /// [NOT NULL]
    pub salesman_order_id: i32,
    /// 订单ID
    pub order_id: Option<i32>,
    /// 分销员ID
    pub salesman_id: Option<i32>,
    /// 收益总额
    pub amount: Option<Decimal>,
    /// 结算状态0待结算1已结算2已失效
    pub status: Option<i32>,
    /// 生成时间
    pub add_time: Option<i32>,
    /// 订单明细ID
    pub item_id: Option<i32>,
    /// 商品运营数据
    pub salesman_product_data: Option<String>,
    /// 订单商品金额
    pub order_amount: Option<Decimal>,
    /// 结算配置数据
    pub salesman_settlement_data: Option<String>,
    /// 结算时间
    pub settlement_time: Option<i32>,
    /// 商品id
    pub product_id: Option<i32>,
}

impl SalesmanOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_order";
}


/// 分销商品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SalesmanProduct {
    /// [NOT NULL]
    pub salesman_product_id: i32,
    /// 商品id
    pub product_id: Option<i32>,
    /// 是否参与0不参与1参与
    pub is_join: Option<i32>,
    /// 佣金计算方式1默认2自定义比例3自定义金额
    pub commission_type: Option<i32>,
    /// 佣金数据
    pub commission_data: Option<String>,
    pub add_time: Option<i32>,
    pub update_time: Option<i32>,
    /// 店铺ID
    pub shop_id: Option<i32>,
}

impl SalesmanProduct {
    /// 数据库表名。
    pub const TABLE: &'static str = "salesman_product";
}