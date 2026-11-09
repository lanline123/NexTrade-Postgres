//! 采购域实体（进货单/收货/对账/退货）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `purchase_operation_log`, `purchase_order`, `purchase_order_item`, `purchase_receipt`, `purchase_receipt_item`, `purchase_reconciliation`, `purchase_reconciliation_item`, `purchase_reconciliation_source`, `purchase_return`, `purchase_return_item`, `purchase_setting`, `purchase_supplier_category`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 采购业务操作日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseOperationLog {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub business_type: String,
    /// [NOT NULL]
    pub business_id: i64,
    /// [NOT NULL]
    pub action: String,
    pub from_status: Option<i32>,
    pub to_status: Option<i32>,
    pub remark: Option<String>,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseOperationLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_operation_log";
}


/// 采购订单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseOrder {
    /// [NOT NULL]
    pub id: i64,
    /// 采购订单号 [NOT NULL]
    pub order_sn: String,
    /// 单据类型：1采购订单，2采购入库单 [NOT NULL]
    pub r#type: i32,
    /// 入库凭证图片地址，仅采购入库单使用
    pub voucher_url: Option<String>,
    /// 组织ID，0为平台自营 [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub vendor_id: i32,
    /// 供应商名称快照 [NOT NULL]
    pub vendor_name: String,
    /// 供应商编码快照 [NOT NULL]
    pub vendor_code: String,
    /// 供应商主联系方式快照 [NOT NULL]
    pub vendor_contact: String,
    /// 银行账户快照JSON
    pub vendor_account_snapshot: Option<String>,
    /// 1待入库，2部分入库，3已完成，4已关闭 [NOT NULL]
    pub status: i32,
    /// 预计送达日期 [NOT NULL]
    pub expected_delivery_date: i64,
    /// 1物流，2无需物流 [NOT NULL]
    pub shipping_method: i32,
    pub logistics_id: Option<i32>,
    pub logistics_name: Option<String>,
    pub tracking_no: Option<String>,
    pub remark: Option<String>,
    pub finish_reason: Option<String>,
    /// [NOT NULL]
    pub product_amount: Decimal,
    /// [NOT NULL]
    pub ordered_quantity: i32,
    /// [NOT NULL]
    pub received_quantity: i32,
    pub finish_time: Option<i64>,
    pub finish_by_id: Option<i64>,
    pub finish_by_name: Option<String>,
    /// [NOT NULL]
    pub version: i32,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_order";
}


/// 采购订单明细
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseOrderItem {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub order_id: i64,
    /// [NOT NULL]
    pub vendor_product_id: i64,
    /// [NOT NULL]
    pub vendor_product_sku_id: i64,
    /// 来源商品ID [NOT NULL]
    pub product_id: i32,
    /// 来源SKU ID，无规格为0 [NOT NULL]
    pub sku_id: i32,
    /// [NOT NULL]
    pub product_name: String,
    /// [NOT NULL]
    pub sku_name: String,
    /// [NOT NULL]
    pub product_sn: String,
    /// [NOT NULL]
    pub sku_sn: String,
    /// [NOT NULL]
    pub pic_thumb: String,
    /// [NOT NULL]
    pub product_unit: String,
    /// [NOT NULL]
    pub quantity: i32,
    /// [NOT NULL]
    pub received_quantity: i32,
    /// [NOT NULL]
    pub purchase_price: Decimal,
    /// [NOT NULL]
    pub amount: Decimal,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseOrderItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_order_item";
}


/// 采购入库单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReceipt {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub receipt_sn: String,
    /// [NOT NULL]
    pub order_id: i64,
    /// [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub vendor_id: i32,
    /// [NOT NULL]
    pub business_date: i64,
    /// 1待确认，2已确认，3已失效 [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub total_quantity: i32,
    /// [NOT NULL]
    pub total_amount: Decimal,
    pub stock_record_id: Option<i32>,
    /// 对账占用ID
    pub reconciliation_id: Option<i64>,
    pub remark: Option<String>,
    pub confirm_time: Option<i64>,
    pub confirm_by_id: Option<i64>,
    pub confirm_by_name: Option<String>,
    /// [NOT NULL]
    pub version: i32,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseReceipt {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_receipt";
}


/// 采购入库明细
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReceiptItem {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub receipt_id: i64,
    /// [NOT NULL]
    pub order_item_id: i64,
    /// [NOT NULL]
    pub product_id: i32,
    /// [NOT NULL]
    pub sku_id: i32,
    /// [NOT NULL]
    pub product_name: String,
    /// [NOT NULL]
    pub sku_name: String,
    /// [NOT NULL]
    pub product_sn: String,
    /// [NOT NULL]
    pub sku_sn: String,
    /// [NOT NULL]
    pub pic_thumb: String,
    /// [NOT NULL]
    pub product_unit: String,
    /// [NOT NULL]
    pub quantity: i32,
    /// [NOT NULL]
    pub returned_quantity: i32,
    /// [NOT NULL]
    pub purchase_price: Decimal,
    /// [NOT NULL]
    pub amount: Decimal,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
    pub active_order_item_key: Option<String>,
}

impl PurchaseReceiptItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_receipt_item";
}


/// 采购对账单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReconciliation {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub reconciliation_sn: String,
    /// [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub vendor_id: i32,
    /// 1待对账，2已对账，3已驳回，4已作废 [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub receipt_amount: Decimal,
    /// [NOT NULL]
    pub return_amount: Decimal,
    /// [NOT NULL]
    pub total_amount: Decimal,
    /// 供应商与联系人快照JSON [NOT NULL]
    pub supplier_snapshot: String,
    /// 银行账户快照JSON
    pub bank_snapshot: Option<String>,
    pub remark: Option<String>,
    pub review_remark: Option<String>,
    pub review_time: Option<i64>,
    pub review_by_id: Option<i64>,
    pub review_by_name: Option<String>,
    /// [NOT NULL]
    pub version: i32,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseReconciliation {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_reconciliation";
}


/// 采购对账商品快照
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReconciliationItem {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub reconciliation_id: i64,
    /// [NOT NULL]
    pub source_type: i32,
    /// [NOT NULL]
    pub source_id: i64,
    /// [NOT NULL]
    pub source_item_id: i64,
    /// [NOT NULL]
    pub product_id: i32,
    /// [NOT NULL]
    pub sku_id: i32,
    /// [NOT NULL]
    pub product_name: String,
    /// [NOT NULL]
    pub sku_name: String,
    /// [NOT NULL]
    pub product_sn: String,
    /// [NOT NULL]
    pub sku_sn: String,
    /// [NOT NULL]
    pub pic_thumb: String,
    /// [NOT NULL]
    pub product_unit: String,
    /// [NOT NULL]
    pub quantity: i32,
    /// [NOT NULL]
    pub price: Decimal,
    /// [NOT NULL]
    pub amount: Decimal,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseReconciliationItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_reconciliation_item";
}


/// 采购对账来源单据快照
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReconciliationSource {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub reconciliation_id: i64,
    /// 1入库单，2退货单 [NOT NULL]
    pub source_type: i32,
    /// [NOT NULL]
    pub source_id: i64,
    /// [NOT NULL]
    pub source_sn: String,
    /// [NOT NULL]
    pub business_date: i64,
    /// [NOT NULL]
    pub source_amount: Decimal,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
    pub active_source_key: Option<String>,
}

impl PurchaseReconciliationSource {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_reconciliation_source";
}


/// 采购退货单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReturn {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub return_sn: String,
    /// [NOT NULL]
    pub order_id: i64,
    /// [NOT NULL]
    pub receipt_id: i64,
    /// [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub vendor_id: i32,
    /// [NOT NULL]
    pub business_date: i64,
    /// 1待确认，2已确认 [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub total_quantity: i32,
    /// [NOT NULL]
    pub total_amount: Decimal,
    pub stock_record_id: Option<i32>,
    /// 对账占用ID
    pub reconciliation_id: Option<i64>,
    pub reason: Option<String>,
    pub remark: Option<String>,
    pub confirm_time: Option<i64>,
    pub confirm_by_id: Option<i64>,
    pub confirm_by_name: Option<String>,
    /// [NOT NULL]
    pub version: i32,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseReturn {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_return";
}


/// 采购退货明细
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseReturnItem {
    /// [NOT NULL]
    pub id: i64,
    /// [NOT NULL]
    pub return_id: i64,
    /// [NOT NULL]
    pub receipt_item_id: i64,
    /// [NOT NULL]
    pub product_id: i32,
    /// [NOT NULL]
    pub sku_id: i32,
    /// [NOT NULL]
    pub product_name: String,
    /// [NOT NULL]
    pub sku_name: String,
    /// [NOT NULL]
    pub product_sn: String,
    /// [NOT NULL]
    pub sku_sn: String,
    /// [NOT NULL]
    pub pic_thumb: String,
    /// [NOT NULL]
    pub product_unit: String,
    /// [NOT NULL]
    pub quantity: i32,
    /// 退货前可退数量：入库数量-已确认退货数量
    pub returnable_quantity: Option<i32>,
    /// 退货前可用数量：可退数量再扣同入库单其他待确认退货占用
    pub available_quantity: Option<i32>,
    /// 退货后剩余可退数量：退货前可退数量-本次退货数量
    pub returnable_quantity_after: Option<i32>,
    /// [NOT NULL]
    pub original_price: Decimal,
    /// [NOT NULL]
    pub return_price: Decimal,
    /// [NOT NULL]
    pub amount: Decimal,
    /// 单项退货原因
    pub reason: Option<String>,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
    pub active_receipt_item_key: Option<String>,
}

impl PurchaseReturnItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_return_item";
}


/// 组织采购设置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseSetting {
    /// [NOT NULL]
    pub id: i64,
    /// 组织ID；仅存shop_id大于0的覆盖配置 [NOT NULL]
    pub shop_id: i32,
    /// 预计送达后自动结束天数，空为继承
    pub auto_finish_days: Option<i32>,
    /// 是否允许超收，空为继承
    pub allow_over_receive: Option<i32>,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
}

impl PurchaseSetting {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_setting";
}


/// 采购供应商分类
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PurchaseSupplierCategory {
    /// [NOT NULL]
    pub id: i64,
    /// 组织ID，0为平台自营 [NOT NULL]
    pub shop_id: i32,
    /// 分类名称 [NOT NULL]
    pub category_name: String,
    /// [NOT NULL]
    pub sort_order: i32,
    /// 状态：1启用，2停用 [NOT NULL]
    pub status: i32,
    /// 备注
    pub remark: Option<String>,
    /// [NOT NULL]
    pub create_time: i64,
    /// [NOT NULL]
    pub create_by_id: i64,
    /// [NOT NULL]
    pub create_by_name: String,
    /// [NOT NULL]
    pub update_time: i64,
    /// [NOT NULL]
    pub update_by_id: i64,
    /// [NOT NULL]
    pub update_by_name: String,
    /// [NOT NULL]
    pub is_del: i32,
    pub active_name_key: Option<String>,
}

impl PurchaseSupplierCategory {
    /// 数据库表名。
    pub const TABLE: &'static str = "purchase_supplier_category";
}