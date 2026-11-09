//! 供应商商品（`/adminapi/vendor/product`、`/adminapi/vendor/productBatch`、
//! `/adminapi/product/product` 的 vendor 导入三件套）DTO。
//!
//! Java 来源（overseas）：
//! - `com.nextrade.bean.query.vendor.VendorProductPageQuery` / `VendorProductClientPageQuery`
//! - `com.nextrade.bean.model.vendor.product.VendorProduct`（行结构）
//! - `com.nextrade.bean.dto.vendor.VendorProductBatchDealDTO` / `VendorProductBatchEditDTO`
//! - `com.nextrade.bean.vo.vendor.product.VendorProductImportExcelVO`（导出/模板列）
//! - `com.nextrade.bean.param.product.VendorProductImportParam` / `VendorProductReImportParam`

use serde::{Deserialize, Serialize};

/// `GET /adminapi/vendor/product/pageList` 查询参数（Java `VendorProductPageQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductPageQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_size", alias = "pageSize")]
    pub size: u64,
    /// 商品 ID 集合（逗号分隔字符串）。
    pub ids: Option<String>,
    pub brand_id: Option<i32>,
    pub category_id: Option<i32>,
    /// 商品状态；0-断供，1-在售。
    pub product_state: Option<i32>,
    /// 审核状态；0-待审核，1-通过，2-失败。
    pub audit_state: Option<i32>,
    /// 是否回收站。
    pub is_recycle: Option<i32>,
    /// 商品名称模糊（前端列表页的关键字）。
    pub keyword: Option<String>,
    /// 指定供应商（平台查看某供应商；供应商账号登录时以 JWT 为准）。
    pub vendor_id: Option<i32>,
}

fn default_page() -> u64 {
    1
}

fn default_size() -> u64 {
    20
}

impl VendorProductPageQuery {
    /// `ids` 逗号分隔 → `Vec<i64>`。
    #[must_use]
    pub fn id_list(&self) -> Vec<i64> {
        self.ids
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|x| x.trim().parse::<i64>().ok())
                    .filter(|x| *x > 0)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 分页 LIMIT/OFFSET。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let size = self.size.clamp(1, 200) as i64;
        let page = self.page.max(1) as i64;
        (size, (page - 1) * size)
    }
}

/// `GET /adminapi/vendor/product/clientPage` 查询参数（Java `VendorProductClientPageQuery`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductClientPageQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_size", alias = "pageSize")]
    pub size: u64,
    pub vendor_id: Option<i32>,
    pub product_brand_id: Option<i32>,
    pub product_category_id: Option<i32>,
    /// 是否可导入；1 时附加「审核通过 + 在售 + 未回收」条件（Java 语义的可用子集）。
    pub is_can_import: Option<i32>,
}

impl VendorProductClientPageQuery {
    /// 分页 LIMIT/OFFSET。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let size = self.size.clamp(1, 200) as i64;
        let page = self.page.max(1) as i64;
        (size, (page - 1) * size)
    }
}

/// 供应商商品行 VO（Java `VendorProductPageVO` 的字段超集，camelCase）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductPageVO {
    pub id: i64,
    pub product_name: Option<String>,
    pub product_brand_id: Option<i32>,
    pub product_category_id: Option<i32>,
    pub product_brief: Option<String>,
    /// 0-断供，1-在售。
    pub product_state: Option<i32>,
    /// 1-单规格，2-多规格。
    pub sku_type: Option<i32>,
    pub product_desc: Option<String>,
    /// 0-待审核，1-通过，2-失败。
    pub audit_state: Option<i32>,
    pub pic_url: Option<String>,
    pub pic_thumb: Option<String>,
    pub pic_original: Option<String>,
    pub pic_large: Option<String>,
    pub is_recycle: Option<i32>,
    pub vendor_id: Option<i32>,
    pub vendor_name: Option<String>,
    pub source_product_id: Option<i32>,
    /// 创建时间（毫秒，bigint 原样）。
    pub create_time: Option<i64>,
    pub create_by_name: Option<String>,
    pub update_time: Option<i64>,
    pub update_by_name: Option<String>,
}

/// `vendorProductBatchDeal` 导出范围 DTO（Java `VendorProductBatchDealDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductBatchDealDTO {
    /// 数据类型；1-分类，2-品牌，3-供应商商品，4-不包含供应商商品。
    pub deal_range: Option<i32>,
    /// 数据主键（逗号分隔字符串，deal_range=3 时为 vendor_product.id）。
    pub range_ids: Option<String>,
}

impl VendorProductBatchDealDTO {
    /// `rangeIds` 逗号分隔 → `Vec<i64>`。
    #[must_use]
    pub fn id_list(&self) -> Vec<i64> {
        self.range_ids
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .filter_map(|x| x.trim().parse::<i64>().ok())
            .filter(|x| *x > 0)
            .collect()
    }
}

/// `vendorProductBatchEdit` 批量编辑 DTO（Java `VendorProductBatchEditDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductBatchEditDTO {
    #[serde(default)]
    pub edit_items: Vec<VendorProductBatchEditItem>,
}

/// 单条编辑项（Java `VendorProductBatchEditDTO.EditItem`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductBatchEditItem {
    /// 供应商商品 ID。
    pub id: Option<i64>,
    pub product_name: Option<String>,
    pub product_brand_id: Option<i32>,
    pub product_category_id: Option<i32>,
    pub product_brief: Option<String>,
    /// 0-断供，1-在售。
    pub product_state: Option<i32>,
}

/// 导出 / 模板 xlsx 的列（Java `VendorProductImportExcelVO` 的 `@ExcelProperty`）。
pub const IMPORT_EXCEL_HEADERS: [&str; 11] = [
    "商品图片",
    "商品名称（必须）",
    "分类（必须）",
    "品牌",
    "商品简介",
    "商品状态（默认0）",
    "规格编码",
    "供货价（必须）",
    "限售金额类型（默认1）",
    "限售金额值（默认0）",
    "库存（默认0）",
];

/// 导出 xlsx 单行（列顺序与 [`IMPORT_EXCEL_HEADERS`] 一致）。
#[derive(Debug, Clone, Default)]
pub struct VendorProductExportRow {
    pub pic_url: String,
    pub product_name: String,
    pub category_name: String,
    pub brand_name: String,
    pub product_brief: String,
    pub product_state: String,
    pub sku_sn: String,
    pub supply_price: String,
    pub supply_price_limit_type: String,
    pub supply_price_limit_val: String,
    pub sku_stock: String,
}

impl VendorProductExportRow {
    /// 展开为与表头对齐的单元格行。
    #[must_use]
    pub fn to_cells(&self) -> Vec<String> {
        vec![
            self.pic_url.clone(),
            self.product_name.clone(),
            self.category_name.clone(),
            self.brand_name.clone(),
            self.product_brief.clone(),
            self.product_state.clone(),
            self.sku_sn.clone(),
            self.supply_price.clone(),
            self.supply_price_limit_type.clone(),
            self.supply_price_limit_val.clone(),
            self.sku_stock.clone(),
        ]
    }
}

/// `POST /adminapi/product/product/vendorProductImport` 入参（Java `VendorProductImportParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductImportParam {
    #[serde(default)]
    pub vendor_product_ids: Vec<i64>,
}

/// `POST /adminapi/product/product/vendorProductReImport` 入参（Java `VendorProductReImportParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProductReImportParam {
    pub vendor_product_id: Option<i64>,
    pub product_id: Option<i32>,
}

/// `GET /adminapi/product/product/getVendorMaxPrice` 返回：
/// 最高可售价 = 供货价 + 限售额（`limit_type==1` 直加；`==2` 按比例）。
///
/// Java `VendorProductSku#getMaxPrice()`。
#[must_use]
pub fn vendor_sku_max_price(
    supply_price: rust_decimal::Decimal,
    limit_type: i32,
    limit_val: rust_decimal::Decimal,
) -> rust_decimal::Decimal {
    match limit_type {
        1 => supply_price + limit_val,
        2 => supply_price + supply_price * limit_val,
        _ => supply_price,
    }
}

#[cfg(test)]
mod tests {
    use super::{vendor_sku_max_price, VendorProductBatchDealDTO, VendorProductImportParam};
    use rust_decimal::Decimal;

    fn d(v: &str) -> Decimal {
        v.parse().unwrap()
    }

    #[test]
    fn max_price_matches_java_formula() {
        // type=1: supply + val
        assert_eq!(vendor_sku_max_price(d("10"), 1, d("5")), d("15"));
        // type=2: supply + supply * val
        assert_eq!(vendor_sku_max_price(d("10"), 2, d("0.1")), d("11"));
        // other: supply
        assert_eq!(vendor_sku_max_price(d("10"), 0, d("5")), d("10"));
    }

    #[test]
    fn batch_deal_ids_parse() {
        let dto: VendorProductBatchDealDTO =
            serde_json::from_str(r#"{"dealRange":3,"rangeIds":"1,2,3"}"#).unwrap();
        assert_eq!(dto.id_list(), vec![1, 2, 3]);
    }

    #[test]
    fn import_param_parses_ids() {
        let p: VendorProductImportParam =
            serde_json::from_str(r#"{"vendorProductIds":[1,2]}"#).unwrap();
        assert_eq!(p.vendor_product_ids, vec![1, 2]);
    }
}
