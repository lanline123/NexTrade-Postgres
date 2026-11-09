//! 库存日志服务（1 个管理端接口）。
//!
//! 对应原 Java `ProductInventoryLogServiceImpl`（135 行）的 `list`：
//!
//! 1. `pageQuery.init()`：`sortField` / `sortOrder` 都为空时写入
//!    `product_inventory_log.add_time` / `desc`。⚠️ 由于 Java
//!    `getColumnByPropertyName` 认不出带表前缀的属性，实际回退主键 → 最终按
//!    `log_id desc` 排序（Rust 仓储如实复刻该兜底）。
//! 2. 联表分页 `productInventoryLog(page, pageQuery)`：
//!    `shop_id` 来自请求头（`HeaderUtils.getShopId()`），keyword 匹配
//!    `product.product_name`，`type != 0` 时按 `type` 过滤。
//! 3. 空页时 Java 直接返回**未填充记录的 page**（total=0），本实现返回空分页。
//! 4. 逐行补全：
//!    - `spec_id > 0` → 查 SKU，`skuValue = "name:value,name:value"`、`sn = skuSn`；
//!    - 否则 → 查商品，`sn = productSn`；
//!    - `productName` 非空时做 `translatePackage.translate`。
//!
//! # 未迁移 / 不确定点
//!
//! - **翻译域**：`translatePackage.translate(productName / name / value)` 未迁移，
//!   返回原始文案（与 `catalog/product_manage` 的既有口径一致）。
//! - **多租户**：Java 用请求头店铺强制 `shop_id = ?`；Rust JWT 无 `shop_id`，
//!   改由查询参数 `shopId` 表达，不传则**不过滤**（平台视角）。
//!   这不是等价实现：店铺账号不传 `shopId` 时会看到全部店铺日志。
//! - Java 在 `spec_id > 0` 但 SKU 查询不到时不会回退 `sn`（保持 null），
//!   本实现同样不写 `sn`；`skuData` 解析失败时 `skuValue` 为空串（Java 会拼接
//!   不到值 → 空串）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_core::entities::{Product, ProductSku};
use nextrade_core::models::catalog::product_inventory_log::{
    fmt_seconds, format_sku_value, ProductInventoryLogQuery, ProductInventoryLogVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::catalog::product_inventory_log::{
    InventoryLogFilters, InventoryLogRow,
};
use nextrade_infrastructure::db::Repositories;

pub struct ProductInventoryLogService {
    repos: Arc<Repositories>,
}

impl ProductInventoryLogService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 库存日志分页（`GET /adminapi/product/productInventoryLog/list`）。
    pub async fn list(
        &self,
        q: &ProductInventoryLogQuery,
    ) -> Result<PageResult<ProductInventoryLogVO>> {
        let page = q.page_query();
        let filters = InventoryLogFilters {
            keyword: q.keyword.as_deref(),
            r#type: q.r#type,
            shop_id: q.shop_id,
            sort_field: q.sort_field.as_deref(),
            sort_order: q.sort_order.as_deref(),
        };
        let (total, rows) = self
            .repos.catalog.product_inventory_log
            .paginate(&filters, &page)
            .await?;
        if rows.is_empty() {
            // Java：`if (CollUtil.isEmpty(...)) return page;`（records 未填充，total=0）
            return Ok(PageResult::empty(&page));
        }

        // 批量预取 SKU / 商品，避免逐行查询（Java 为循环内逐条 selectById）
        let sku_ids: Vec<i32> = rows
            .iter()
            .filter_map(|r| r.spec_id)
            .filter(|id| *id > 0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let product_ids: Vec<i32> = rows
            .iter()
            .filter(|r| r.spec_id.is_none_or(|id| id <= 0))
            .filter_map(|r| r.product_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let sku_map: HashMap<i32, ProductSku> = self
            .repos.catalog.product_inventory_log
            .find_skus(&sku_ids)
            .await?
            .into_iter()
            .map(|s| (s.sku_id, s))
            .collect();
        let product_map: HashMap<i32, Product> = self
            .repos.catalog.product_inventory_log
            .find_products(&product_ids)
            .await?
            .into_iter()
            .map(|p| (p.product_id, p))
            .collect();

        let list = rows
            .iter()
            .map(|row| build_vo(row, &sku_map, &product_map))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }
}

/// 行 → VO（对应 Java `list` 内的补全循环）。
fn build_vo(
    row: &InventoryLogRow,
    sku_map: &HashMap<i32, ProductSku>,
    product_map: &HashMap<i32, Product>,
) -> ProductInventoryLogVO {
    let mut vo = ProductInventoryLogVO {
        log_id: Some(row.log_id),
        product_id: row.product_id,
        spec_id: row.spec_id,
        number: row.number,
        old_number: row.old_number,
        r#type: row.r#type,
        change_number: row.change_number,
        desc: row.desc.clone(),
        shop_id: Some(row.shop_id),
        product_name: row.product_name.clone(),
        add_time: Some(fmt_seconds(row.add_time)),
        ..Default::default()
    };

    if let Some(spec_id) = row.spec_id.filter(|id| *id > 0) {
        if let Some(sku) = sku_map.get(&spec_id) {
            // Java：只有 skuData 非空时才写 skuValue / sn（否则两列保持 null）
            let sku_value = format_sku_value(sku.sku_data.as_deref());
            if !sku_value.is_empty() {
                vo.sku_value = Some(sku_value);
                vo.sn = sku.sku_sn.clone();
            }
        }
    } else if let Some(product_id) = row.product_id {
        if let Some(product) = product_map.get(&product_id) {
            vo.sn = Some(product.product_sn.clone());
        }
    }
    vo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(spec_id: Option<i32>, product_id: Option<i32>) -> InventoryLogRow {
        InventoryLogRow {
            log_id: 7,
            product_id,
            spec_id,
            number: Some(10),
            add_time: Some(1_700_000_000),
            old_number: Some(12),
            r#type: Some(2),
            change_number: Some(-2),
            desc: Some("下单扣减".into()),
            shop_id: 3,
            product_name: Some("测试商品".into()),
        }
    }

    #[test]
    fn sku_level_log_fills_sku_value_and_sn() {
        let mut sku_map = HashMap::new();
        sku_map.insert(
            11,
            ProductSku {
                sku_id: 11,
                product_id: 5,
                sku_value: "红-L".into(),
                sku_data: Some(r#"[{"name":"颜色","value":"红"}]"#.into()),
                sku_sn: Some("SKU-11".into()),
                ..Default::default()
            },
        );
        let vo = build_vo(&row(Some(11), Some(5)), &sku_map, &HashMap::new());
        assert_eq!(vo.sku_value.as_deref(), Some("颜色:红"));
        assert_eq!(vo.sn.as_deref(), Some("SKU-11"));
        assert_eq!(vo.add_time.as_deref().map(str::len), Some(19));
        assert_eq!(vo.product_name.as_deref(), Some("测试商品"));
    }

    #[test]
    fn product_level_log_falls_back_to_product_sn() {
        let mut product_map = HashMap::new();
        product_map.insert(
            5,
            Product {
                product_id: 5,
                product_sn: "P-5".into(),
                ..Default::default()
            },
        );
        let vo = build_vo(&row(None, Some(5)), &HashMap::new(), &product_map);
        assert_eq!(vo.sn.as_deref(), Some("P-5"));
        assert!(vo.sku_value.is_none());
    }

    #[test]
    fn missing_sku_keeps_sn_null_like_java() {
        let vo = build_vo(&row(Some(99), Some(5)), &HashMap::new(), &HashMap::new());
        assert!(vo.sn.is_none(), "Java 在 SKU 查不到时不回退商品编号");
        assert!(vo.sku_value.is_none());
    }
}
