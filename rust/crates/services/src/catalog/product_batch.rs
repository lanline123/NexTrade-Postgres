//! 商品批量导入/导出服务（2 个管理端接口）。
//!
//! 对应原 Java `ProductBatchServiceImpl`（1244 行）中的：
//! - [`ProductBatchService::product_batch_deal`] ← `productBatchDeal`（导出）
//! - [`ProductBatchService::services_list`] ← `ServicesService#list`
//!   （`ProductBatchController#list` 直接委托）
//!
//! # 导出语义（逐条对照 Java）
//!
//! 1. `rangeIds` 逗号拆分；`dealRange`：
//!    `1` 分类（含全部子孙分类）/ `2` 品牌 / `3` 商品 / `4`（不包含商品）；
//! 2. 固定条件 `is_delete=0 AND check_status=1 AND product_status=1`；
//! 3. 构建分类路径（`buildCategoryPath`：叶子向上回溯 `父/子`）；
//! 4. 商品有 SKU → 每个 SKU 一行；无 SKU → 单规格一行；
//! 5. 每行按 `ProductImportExcelVO.valueOfExport` 组装（配送方式、运费模板、
//!    详情图 URL、规格文本、规格图等）。
//!
//! # 偏差 / 未迁移
//!
//! - **不写 xlsx 二进制**：Java `@ResponseExcel` 输出 xlsx；本工作区 services
//!   crate 未声明 xlsx 写出依赖（`calamine` 仅读），故本接口返回同样的
//!   **行数据 JSON**（列名/列序语义不变）。前端/运维侧自行导出。
//! - **店铺隔离**：Java 用请求头 `shopId > 0` 时叠加 `shop_id = 本店`；
//!   Rust JWT 未签发店铺，当前导出平台全量（无店铺过滤）。
//! - **翻译域**：`translatePackage` 未迁移，分类名/商品名输出库中原值。
//! - `nextradeProperties` 的 O2O 标记在 Java `productBatchDeal` 中只声明未使用，
//!   本实现同样不区分 O2O。
//! - 规格图字典按 Java `specPicKey(productId, attrName, attrValue)` 精确匹配
//!   **第一个规格对**；Java 用 `\u0000` 分隔，这里用 `(product_id, name, value)`
//!   三元组 key，语义等价。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{Product, ProductServices, ProductSku};
use nextrade_core::models::catalog::product_batch::{
    build_category_path, build_desc_pic_url_text, build_shipping_tpl_text,
    build_shipping_type_text, decimal_plain, decimal_strip, parse_spec_pairs,
    to_import_spec_text, ProductBatchDealQuery, ProductImportExcelVO, ProductServiceDTO,
    ProductServiceListQuery,
};
use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

pub struct ProductBatchService {
    repos: Arc<Repositories>,
}

impl ProductBatchService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 商品导出（`GET /adminapi/product/productBatch/productBatchDeal`）。
    ///
    /// 对应 Java `ProductBatchServiceImpl#productBatchDeal`；返回行数据 JSON
    /// （不写 xlsx，见模块头「偏差」）。
    pub async fn product_batch_deal(
        &self,
        q: &ProductBatchDealQuery,
    ) -> Result<Vec<ProductImportExcelVO>> {
        let range_ids = q.parsed_range_ids();

        // 分类范围：先展开「本分类 + 全部子孙分类」（Java `getCategoryAllChildIds`）
        let mut categories = Vec::new();
        let category_ids = if q.deal_range == Some(1) {
            if range_ids.is_empty() {
                // Java：rangeIds 为空 → allCateGoryIds 为空 → products 保持空
                return Ok(Vec::new());
            }
            categories = self
                .repos.catalog.product_batch
                .find_categories()
                .await?;
            expand_category_ids(&categories, &range_ids)
        } else {
            Vec::new()
        };

        let products = self
            .repos.catalog.product_batch
            .find_products_for_export(q.deal_range, &range_ids, &category_ids)
            .await?;
        if products.is_empty() {
            return Ok(Vec::new());
        }

        // 分类路径需要全量分类（Java：products 非空时再查一次 categoryService.list）
        if categories.is_empty() {
            categories = self
                .repos.catalog.product_batch
                .find_categories()
                .await?;
        }
        let cat_tuples: Vec<(i32, i32, String)> = categories
            .iter()
            .map(|c| (c.category_id, c.parent_id, c.category_name.clone()))
            .collect();

        let product_ids: Vec<i32> = products.iter().map(|p| p.product_id).collect();
        let mut sku_map: HashMap<i32, Vec<ProductSku>> = HashMap::new();
        for sku in self
            .repos.catalog.product_batch
            .find_skus(&product_ids)
            .await?
        {
            sku_map.entry(sku.product_id).or_default().push(sku);
        }

        // 规格图字典：productId + 规格名 + 规格值 → attr_pic
        let mut spec_pic_dict: HashMap<(i32, String, String), String> = HashMap::new();
        for attr in self
            .repos.catalog.product_batch
            .find_spec_attrs(&product_ids)
            .await?
        {
            let pic = attr.attr_pic.unwrap_or_default();
            if pic.trim().is_empty() {
                continue;
            }
            spec_pic_dict
                .entry((
                    attr.product_id,
                    attr.attr_name.trim().to_string(),
                    attr.attr_value.trim().to_string(),
                ))
                .or_insert(pic);
        }

        let mut result = Vec::new();
        for p in &products {
            let cat_path = build_category_path(Some(p.category_id), &cat_tuples);
            match sku_map.get(&p.product_id) {
                Some(skus) if !skus.is_empty() => {
                    for sku in skus {
                        let mut row = export_row(p, &cat_path, Some(sku));
                        row.spec_pic_url =
                            resolve_spec_pic(&spec_pic_dict, p.product_id, &row.spec_name);
                        result.push(row);
                    }
                }
                _ => result.push(export_row(p, &cat_path, None)),
            }
        }
        tracing::info!("商品导出完成: {} 行", result.len());
        Ok(result)
    }

    /// 商品服务分页（`GET /adminapi/product/productBatch/productService/list`）。
    pub async fn services_list(
        &self,
        q: &ProductServiceListQuery,
    ) -> Result<PageResult<ProductServiceDTO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos.catalog.product_batch
            .paginate_services(
                q.keyword.as_deref(),
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;
        let list = rows.iter().map(to_service_dto).collect();
        Ok(PageResult::new(total, list, &page))
    }
}

// ————————————————————— 纯函数（可单测） —————————————————————

/// 展开「本分类 + 全部子孙分类」（Java `CategoryService#getCategoryAllChildIds`）。
#[must_use]
pub fn expand_category_ids(categories: &[nextrade_core::entities::Category], roots: &[i32]) -> Vec<i32> {
    let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
    for c in categories {
        children.entry(c.parent_id).or_default().push(c.category_id);
    }
    let mut out: Vec<i32> = Vec::new();
    let mut stack: Vec<i32> = roots.to_vec();
    while let Some(id) = stack.pop() {
        if out.contains(&id) {
            continue;
        }
        out.push(id);
        if let Some(kids) = children.get(&id) {
            stack.extend(kids.iter().copied());
        }
    }
    out
}

/// 商品 + 分类路径 + SKU → 导出行（Java `ProductImportExcelVO.valueOfExport`）。
#[must_use]
pub fn export_row(p: &Product, cat_path: &str, sku: Option<&ProductSku>) -> ProductImportExcelVO {
    let base = ProductImportExcelVO {
        product_sn: p.product_sn.clone(),
        product_name: p.product_name.clone(),
        pic_url: if p.pic_url.trim().is_empty() {
            p.pic_original.clone()
        } else {
            p.pic_url.clone()
        },
        category_name: cat_path.to_string(),
        shipping_type: build_shipping_type_text(
            p.is_logistics,
            p.is_shop_delivery,
            p.is_shop_pickup,
        ),
        shipping_tpl: build_shipping_tpl_text(
            p.fixed_shipping_type,
            p.fixed_shipping_fee,
            p.shipping_tpl_id,
        ),
        product_status: p.product_status.to_string(),
        product_tsn: p.product_tsn.clone().unwrap_or_default(),
        keywords: p.keywords.clone().unwrap_or_default(),
        product_brief: p.product_brief.clone(),
        desc_pic_url: build_desc_pic_url_text(p.product_desc.as_deref()),
        category_id: Some(p.category_id),
        shop_id: Some(p.shop_id),
        ..Default::default()
    };
    match sku {
        Some(sku) => ProductImportExcelVO {
            spec_name: resolve_spec_text(sku),
            sku_sn: sku.sku_sn.clone().unwrap_or_default(),
            sku_tsn: sku.sku_tsn.clone().unwrap_or_default(),
            product_price: decimal_plain(sku.sku_price),
            product_stock: sku.sku_stock.unwrap_or(0).to_string(),
            market_price: decimal_strip(sku.market_price),
            cost_price: decimal_strip(sku.cost_price),
            ..base
        },
        None => ProductImportExcelVO {
            spec_name: String::new(),
            sku_sn: String::new(),
            sku_tsn: String::new(),
            product_price: decimal_plain(Some(p.product_price)),
            product_stock: p.product_stock.to_string(),
            market_price: decimal_strip(Some(p.market_price)),
            cost_price: decimal_strip(p.cost_price),
            ..base
        },
    }
}

/// 规格文本（Java `ProductImportExcelVO#resolveSpecText`）：
/// `sku_value` 非空 → `|` 替换为 `；`；否则用 `sku_data` 的 `name:value` 拼接。
#[must_use]
pub fn resolve_spec_text(sku: &ProductSku) -> String {
    if !sku.sku_value.trim().is_empty() {
        return to_import_spec_text(Some(&sku.sku_value));
    }
    let Some(raw) = sku.sku_data.as_deref().filter(|s| !s.trim().is_empty()) else {
        return String::new();
    };
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(raw) else {
        return String::new();
    };
    items
        .iter()
        .filter_map(|v| {
            let o = v.as_object()?;
            let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let value = o.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                return None;
            }
            Some(format!("{name}:{value}"))
        })
        .collect::<Vec<_>>()
        .join("；")
}

/// 规格图（Java `resolveExportSpecPic`：取第一个规格对的图）。
#[must_use]
pub fn resolve_spec_pic(
    dict: &HashMap<(i32, String, String), String>,
    product_id: i32,
    spec_text: &str,
) -> String {
    if dict.is_empty() || spec_text.trim().is_empty() {
        return String::new();
    }
    let pairs = parse_spec_pairs(spec_text);
    let Some((name, value)) = pairs.first() else {
        return String::new();
    };
    dict.get(&(product_id, name.clone(), value.clone()))
        .cloned()
        .unwrap_or_default()
}

/// `product_services` 实体 → `ServicesDTO`。
fn to_service_dto(s: &ProductServices) -> ProductServiceDTO {
    ProductServiceDTO {
        product_service_id: Some(s.product_service_id),
        product_service_name: Some(s.product_service_name.clone()),
        product_service_desc: Some(s.product_service_desc.clone()),
        ico_img: Some(s.ico_img.clone()),
        sort_order: Some(s.sort_order),
        default_on: Some(s.default_on),
        shop_id: Some(s.shop_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product() -> Product {
        Product {
            product_id: 5,
            product_name: "测试商品".into(),
            product_sn: "P-5".into(),
            pic_url: "".into(),
            pic_original: "orig.png".into(),
            product_price: rust_decimal::Decimal::new(1990, 2),
            market_price: rust_decimal::Decimal::new(2990, 2),
            product_stock: 8,
            product_status: 1,
            category_id: 3,
            shop_id: 2,
            is_logistics: 1,
            is_shop_pickup: 1,
            fixed_shipping_type: Some(2),
            shipping_tpl_id: 9,
            cost_price: Some(rust_decimal::Decimal::new(1000, 2)),
            ..Default::default()
        }
    }

    #[test]
    fn single_spec_row_matches_java_value_of_export() {
        let row = export_row(&product(), "一级/二级", None);
        assert_eq!(row.pic_url, "orig.png", "picUrl 为空时回退 picOriginal");
        assert_eq!(row.shipping_type, "快递发货；到店自提");
        assert_eq!(row.shipping_tpl, "【运费模板】9");
        assert_eq!(row.product_price, "19.90");
        assert_eq!(row.market_price, "29.9");
        assert_eq!(row.cost_price, "10");
        assert_eq!(row.product_stock, "8");
        assert_eq!(row.category_name, "一级/二级");
        assert!(row.spec_name.is_empty());
    }

    #[test]
    fn sku_row_uses_sku_prices_and_spec_text() {
        let sku = ProductSku {
            sku_id: 11,
            product_id: 5,
            sku_value: "颜色:红|尺寸:L".into(),
            sku_sn: Some("S-11".into()),
            sku_price: Some(rust_decimal::Decimal::new(2550, 2)),
            sku_stock: Some(3),
            market_price: Some(rust_decimal::Decimal::new(3000, 2)),
            cost_price: Some(rust_decimal::Decimal::new(1200, 2)),
            ..Default::default()
        };
        let row = export_row(&product(), "", Some(&sku));
        assert_eq!(row.spec_name, "颜色:红；尺寸:L");
        assert_eq!(row.sku_sn, "S-11");
        assert_eq!(row.product_price, "25.50");
        assert_eq!(row.product_stock, "3");
        assert_eq!(row.market_price, "30");
    }

    #[test]
    fn category_expansion_includes_self_and_descendants() {
        use nextrade_core::entities::Category;
        let cats: Vec<Category> = vec![
            Category { category_id: 1, parent_id: 0, ..Default::default() },
            Category { category_id: 2, parent_id: 1, ..Default::default() },
            Category { category_id: 3, parent_id: 2, ..Default::default() },
            Category { category_id: 9, parent_id: 0, ..Default::default() },
        ];
        let mut ids = expand_category_ids(&cats, &[1]);
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2, 3]);
        let mut ids = expand_category_ids(&cats, &[9, 2]);
        ids.sort_unstable();
        assert_eq!(ids, vec![2, 3, 9]);
    }

    #[test]
    fn spec_pic_resolves_first_pair() {
        let mut dict = HashMap::new();
        dict.insert((5, "颜色".to_string(), "红".to_string()), "red.png".to_string());
        assert_eq!(
            resolve_spec_pic(&dict, 5, "颜色:红；尺寸:L"),
            "red.png"
        );
        assert_eq!(resolve_spec_pic(&dict, 6, "颜色:红"), "");
        assert_eq!(resolve_spec_pic(&dict, 5, ""), "");
    }
}
