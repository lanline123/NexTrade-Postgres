//! 分销商品域服务（marketing::salesman_product）。
//!
//! 对应原 Java `SalesmanProductServiceImpl`（admin 6 端点 + client 2 端点）。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - **列表 VO 简化**：Java 直接 copy 商品全部字段；本实现返回核心字段
//!   （见 `SalesmanProductListVO` 文档），不返回商品描述等长尾字段。
//! - **`detailByProductId` 默认佣金**：Java `commissionType=1` 时从
//!   `salesman_config.data` 解析公共等级比例；本实现同（读 `salesman_config`），
//!   解析失败时回落为库中 `commission_data`（偏差：不抛异常）。
//! - **`analysis`**：Java 走 `TigUtils.getDateRange(dateType,...)` 的日期区间
//!   换算；本实现只接受完整 `yyyy-MM-dd HH:mm:ss` 串（解析失败不限时间），
//!   聚合口径（成交额=Σorder_amount、已结算佣金=Σamount WHERE status=1）一致。
//! - **客户端详情佣金计算**：依赖 `salesman_config` 等级比例；未找到分销员/等级
//!   时 `finalCommissionPrice` 为空串（Java 会抛「未找到匹配的等级」）。
//! - **`@JsonTranslate` 翻译**未迁移。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::SalesmanProduct;
use nextrade_core::models::marketing::lottery_activity::format_datetime;
use nextrade_core::models::marketing::salesman_product::{
    commission_type_text, parse_commission_data, product_type_map, CommissionData,
    CommissionLevelData, ProductCommission, ProductImageVO, SalesmanProductAnalysisQuery,
    SalesmanProductDetailVO, SalesmanProductEditParam, SalesmanProductListVO,
    SalesmanProductPageQuery, SalesmanProductSaveParam, SalesmanProductVO,
    COMMISSION_CUSTOM_PRICE, COMMISSION_CUSTOM_RATE, COMMISSION_DEFAULT,
    MSG_SALESMAN_PRODUCT_ID_REQUIRED, MSG_UNKNOWN_SALESMAN_PRODUCT,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::salesman_product::SalesmanProductRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 分销商品域服务。
pub struct SalesmanProductService {
    repos: Arc<Repositories>,
}

impl SalesmanProductService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &SalesmanProductRepository {
        &self.repos.marketing.salesman_product
    }

    // ————————————————————— admin —————————————————————

    /// 分销商品列表（Java `list`）。
    pub async fn list(
        &self,
        q: &SalesmanProductPageQuery,
    ) -> Result<nextrade_core::types::PageResult<SalesmanProductListVO>> {
        // 多租户：店铺账号强制本店；平台账号按平台级（0）
        let shop_id = scope_shop_id();
        let (limit, offset) = q.limit_offset();
        let page = to_page_query(q.page, q.size);
        let (total, rows) = self.repo().paginate(q, shop_id, limit, offset).await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            let config = self.repo().find_by_product(row.product_id).await?;
            let salesman_product = config
                .as_ref()
                .map(|_| self.to_vo(config.as_ref().unwrap()));
            let pics = self
                .repo()
                .galleries(row.product_id)
                .await?
                .into_iter()
                .map(|g| ProductImageVO {
                    pic_id: g.pic_id,
                    product_id: g.product_id,
                    pic_url: g.pic_url,
                    pic_desc: g.pic_desc,
                    pic_thumb: g.pic_thumb,
                    pic_original: g.pic_original,
                    pic_large: g.pic_large,
                    sort_order: g.sort_order,
                })
                .collect();
            list.push(SalesmanProductListVO {
                salesman_product_id: row.salesman_product_id,
                product_id: row.product_id,
                product_name: row.product_name.clone(),
                product_sn: row.product_sn.clone(),
                product_tsn: row.product_tsn.clone(),
                product_stock: row.product_stock,
                product_price: row.product_price,
                market_price: row.market_price,
                cost_price: row.cost_price,
                product_status: row.product_status,
                product_type: row.product_type,
                pic_url: row.pic_url.clone(),
                pic_thumb: row.pic_thumb.clone(),
                is_delete: row.is_delete,
                add_time: Some(row.add_time),
                update_time: Some(row.last_update),
                is_join: row.is_join,
                commission_type: row.commission_type,
                commission_data: row.commission_data.clone(),
                pics,
                salesman_product,
            });
        }
        Ok(nextrade_core::types::PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 按商品 ID 取分销配置（Java `detailByProductId`；找不到返回 `None`）。
    pub async fn detail_by_product_id(&self, product_id: i32) -> Result<Option<SalesmanProductVO>> {
        let Some(sp) = self.repo().find_by_product(product_id).await? else {
            return Ok(None);
        };
        // 多租户：店铺账号只能查看本店分销配置
        TenantScope::current_ensure_access(sp.shop_id.unwrap_or(0))?;
        Ok(Some(self.to_vo_default_commission(&sp).await?))
    }

    /// 创建（Java `create`）。
    pub async fn create(&self, param: &SalesmanProductSaveParam) -> Result<()> {
        // 多租户：店铺账号强制本店；平台账号落平台级（0）
        let shop_id = TenantScope::current().write_shop_id(None)?;
        let product_id = param.validate().map_err(Error::Validation)?;
        let now = now_seconds_i32();
        let entity = SalesmanProduct {
            salesman_product_id: 0,
            product_id: Some(product_id),
            is_join: Some(if param.is_join.unwrap_or(false) { 1 } else { 0 }),
            commission_type: param.commission_type,
            commission_data: Some(param.commission_json()),
            add_time: Some(now),
            update_time: Some(now),
            shop_id: Some(shop_id),
        };
        self.repo().insert(&entity).await?;
        Ok(())
    }

    /// 更新（Java `update`）。
    pub async fn update(&self, param: &SalesmanProductEditParam) -> Result<()> {
        let id = param.validate().map_err(Error::Validation)?;
        let old = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_UNKNOWN_SALESMAN_PRODUCT))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(old.shop_id.unwrap_or(0))?;
        let entity = SalesmanProduct {
            salesman_product_id: id,
            product_id: Some(param.product_id.unwrap_or_default()),
            is_join: Some(if param.is_join.unwrap_or(false) { 1 } else { 0 }),
            commission_type: param.commission_type,
            commission_data: Some(param.commission_json()),
            add_time: old.add_time,
            update_time: Some(now_seconds_i32()),
            shop_id: old.shop_id,
        };
        self.repo().update(&entity).await?;
        Ok(())
    }

    /// 配置映射（Java `config()` → `SalesmanProductType.getAll()`）。
    #[must_use]
    pub fn config() -> Vec<(i32, &'static str)> {
        product_type_map()
    }

    /// 批量操作（`POST /salesman/product/batch`）。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/salesman/product.ts#batchSubmit`
    /// （body `{ type, ids }`；Java `SalesmanProductController` 未含 `batch`）。
    /// 语义对齐各域 `BaseServiceImpl#batch`：`type = "del"` 时
    /// `removeByIds` 物理删除；其它 `type` 静默返回 `false`。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn batch(&self, batch_type: &str, ids: &[i32]) -> Result<bool> {
        if batch_type != "del" || ids.is_empty() {
            return Ok(false);
        }
        let deleted = self.repo().delete_many(ids).await?;
        Ok(deleted > 0)
    }

    /// 商品成交分析（Java `analysis`）。
    pub async fn analysis(
        &self,
        q: &SalesmanProductAnalysisQuery,
    ) -> Result<nextrade_core::types::PageResult<serde_json::Value>> {
        // 多租户：店铺账号强制本店；平台账号按平台级（0）
        let shop_id = scope_shop_id();
        let (limit, offset) = q.limit_offset();
        let page = to_page_query(q.page, q.size);
        let (total, rows) = self.repo().joined_products(shop_id, limit, offset).await?;
        let (start, end) = q.search_range();
        let mut items: Vec<serde_json::Value> = Vec::new();
        for sp in &rows {
            let Some(product_id) = sp.product_id else {
                continue;
            };
            let Some(product) = self.repo().product_by_id(product_id).await? else {
                continue;
            };
            let (total_money, commission) =
                self.repo().order_stats(product_id, start, end).await?;
            items.push(serde_json::json!({
                "productId": product_id,
                "productName": product.product_name,
                "productSn": product.product_sn,
                "picThumb": product.pic_thumb,
                "totalProductMoney": total_money,
                "commissionExpenses": commission,
            }));
        }
        // 排序（Java 仅支持 totalProductMoney / commissionExpenses）
        if let (Some(field), Some(order)) = (q.sort_field.as_deref(), q.sort_order.as_deref()) {
            if field == "totalProductMoney" || field == "commissionExpenses" {
                let key = field.to_string();
                items.sort_by(|a, b| {
                    let va = a.get(&key).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
                    let vb = b.get(&key).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
                    if order.eq_ignore_ascii_case("desc") {
                        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                    }
                });
            }
        }
        Ok(nextrade_core::types::PageResult::new(
            u64::try_from(total).unwrap_or(0),
            items,
            &page,
        ))
    }

    // ————————————————————— client —————————————————————

    /// 客户端详情（Java `detailClient`）。
    pub async fn detail_client(
        &self,
        product_id: i32,
        current_user_id: i32,
    ) -> Result<Option<SalesmanProductDetailVO>> {
        let Some(product) = self.repo().product_by_id(product_id).await? else {
            return Ok(None);
        };
        let pics = self
            .repo()
            .galleries(product_id)
            .await?
            .into_iter()
            .map(|g| ProductImageVO {
                pic_id: g.pic_id,
                product_id: g.product_id,
                pic_url: g.pic_url,
                pic_desc: g.pic_desc,
                pic_thumb: g.pic_thumb,
                pic_original: g.pic_original,
                pic_large: g.pic_large,
                sort_order: g.sort_order,
            })
            .collect();
        let sp = self.repo().find_by_product(product_id).await?;
        let final_commission_price = self
            .final_commission_price(&product, sp.as_ref(), current_user_id)
            .await?;
        Ok(Some(SalesmanProductDetailVO {
            product_id: product.product_id,
            product_name: product.product_name.clone(),
            product_sn: product.product_sn.clone(),
            product_tsn: product.product_tsn.clone(),
            product_stock: product.product_stock,
            product_price: product.product_price,
            market_price: product.market_price,
            product_status: product.product_status,
            product_type: product.product_type,
            product_brief: product.product_brief.clone(),
            product_desc: product.product_desc.clone(),
            pic_url: product.pic_url.clone(),
            pic_thumb: product.pic_thumb.clone(),
            pic_original: product.pic_original.clone(),
            pics,
            salesman_product: sp.as_ref().map(|s| self.to_vo(s)),
            final_commission_price,
        }))
    }

    /// 计算当前分销员等级的最终推广佣金（Java `detailClient` 4.1~4.3）。
    async fn final_commission_price(
        &self,
        product: &nextrade_core::entities::Product,
        sp: Option<&SalesmanProduct>,
        user_id: i32,
    ) -> Result<String> {
        let Some(sp) = sp else {
            return Ok(String::new());
        };
        let Some(commission_type) = sp.commission_type else {
            return Ok(String::new());
        };
        let Some(salesman) = self.repo().salesman_by_user(user_id).await? else {
            return Ok(String::new());
        };
        let Some(level) = salesman.level else {
            return Ok(String::new());
        };
        let config_data = self
            .repo()
            .config_by_code("salesmanConfig")
            .await?
            .and_then(|c| c.data)
            .unwrap_or_else(|| "{}".to_string());
        let rate = config_level_rate(&config_data, level);
        match commission_type {
            COMMISSION_DEFAULT => Ok(rate
                .map(|r| commission_amount(product.product_price, &r))
                .unwrap_or_default()),
            COMMISSION_CUSTOM_RATE => {
                let data = parse_commission_data(sp.commission_data.as_deref());
                let rate = data
                    .as_deref()
                    .and_then(|d| custom_level_rate(d, level))
                    .or(rate);
                Ok(rate
                    .map(|r| commission_amount(product.product_price, &r))
                    .unwrap_or_default())
            }
            COMMISSION_CUSTOM_PRICE => {
                let data = parse_commission_data(sp.commission_data.as_deref());
                Ok(data
                    .as_deref()
                    .and_then(|d| custom_level_rate(d, level))
                    .unwrap_or_default())
            }
            _ => Ok(String::new()),
        }
    }

    // ————————————————————— 内部 —————————————————————

    fn to_vo(&self, sp: &SalesmanProduct) -> SalesmanProductVO {
        SalesmanProductVO {
            salesman_product_id: Some(sp.salesman_product_id),
            product_id: sp.product_id,
            is_join: sp.is_join,
            commission_type: sp.commission_type,
            commission_data: parse_commission_data(sp.commission_data.as_deref()),
            add_time: Some(format_datetime(sp.add_time.unwrap_or(0))),
            update_time: Some(format_datetime(sp.update_time.unwrap_or(0))),
            shop_id: sp.shop_id,
            product_commission: Some(ProductCommission::default()),
        }
    }

    /// `detailByProductId`：默认佣金方式时用公共分销配置替换等级数据。
    async fn to_vo_default_commission(&self, sp: &SalesmanProduct) -> Result<SalesmanProductVO> {
        let mut vo = self.to_vo(sp);
        if sp.commission_type == Some(COMMISSION_DEFAULT) {
            let config_data = self
                .repo()
                .config_by_code("salesmanConfig")
                .await?
                .and_then(|c| c.data)
                .unwrap_or_else(|| "{}".to_string());
            let levels = config_levels(&config_data);
            if !levels.is_empty() {
                vo.commission_data = Some(vec![CommissionData {
                    level_arr: Some(levels),
                }]);
            }
        }
        Ok(vo)
    }
}

/// 从 `salesman_config.data` JSON 提取等级数组（`level: [{id, rate, downSalesmanRate}]`）。
#[must_use]
pub fn config_levels(data_json: &str) -> Vec<CommissionLevelData> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(data_json) else {
        return Vec::new();
    };
    value
        .get("level")
        .and_then(|v| v.as_array())
        .map(|levels| {
            levels
                .iter()
                .filter_map(|level| {
                    Some(CommissionLevelData {
                        level: level.get("id").and_then(serde_json::Value::as_i64).map(|v| v as i32),
                        rate: level
                            .get("rate")
                            .and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string()))),
                        down_salesman_rate: level.get("downSalesmanRate").and_then(|v| {
                            v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string()))
                        }),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 某个等级的比例（字符串）。
#[must_use]
pub fn config_level_rate(data_json: &str, level: i32) -> Option<String> {
    config_levels(data_json)
        .into_iter()
        .find(|l| l.level == Some(level))
        .and_then(|l| l.rate)
}

fn custom_level_rate(data: &[CommissionData], level: i32) -> Option<String> {
    data.iter()
        .flat_map(|d| d.level_arr.iter().flatten())
        .find(|l| l.level == Some(level))
        .and_then(|l| l.rate.clone())
}

/// 按比例计算佣金（Java：price × rate% ，HALF_UP 保留 2 位）。
#[must_use]
pub fn commission_amount(price: Decimal, rate: &str) -> String {
    use rust_decimal::prelude::FromPrimitive;
    let Ok(rate_value) = rate.trim().parse::<f64>() else {
        return String::new();
    };
    let Some(percent) = Decimal::from_f64(rate_value / 100.0) else {
        return String::new();
    };
    let amount = (price * percent).round_dp_with_strategy(
        2,
        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
    );
    amount.to_string()
}

fn now_seconds_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(0)
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    nextrade_core::types::PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

/// 当前请求的店铺 ID：店铺账号=本店，平台账号=0（Java HeaderUtils.getShopId() 同义）。
fn scope_shop_id() -> i32 {
    TenantScope::current().shop_id().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tenant_scope_selects_shop_or_platform_scope_for_salesman_product() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (scope, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scope_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(scope, 8);
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let scope = with_tenant_scope(TenantScope::platform(), async { scope_shop_id() }).await;
        assert_eq!(scope, 0);
    }

    #[test]
    fn commission_amount_rounds_half_up() {
        assert_eq!(
            commission_amount(Decimal::new(1999, 2), "10"),
            "2.00"
        );
        assert_eq!(commission_amount(Decimal::new(10000, 2), "12.5"), "12.50");
        assert_eq!(commission_amount(Decimal::new(100, 2), "bad"), "");
    }

    #[test]
    fn config_levels_parse_matches_java_structure() {
        let data = r#"{"level":[{"id":1,"rate":"10","downSalesmanRate":"2"},{"id":2,"rate":"15"}]}"#;
        let levels = config_levels(data);
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].level, Some(1));
        assert_eq!(levels[0].rate.as_deref(), Some("10"));
        assert_eq!(levels[0].down_salesman_rate.as_deref(), Some("2"));
        assert_eq!(config_level_rate(data, 2).as_deref(), Some("15"));
        assert!(config_level_rate(data, 9).is_none());
        assert!(config_levels("not-json").is_empty());
    }

    #[test]
    fn custom_level_rate_finds_by_level() {
        let data = vec![CommissionData {
            level_arr: Some(vec![
                CommissionLevelData {
                    level: Some(1),
                    rate: Some("8".into()),
                    down_salesman_rate: None,
                },
                CommissionLevelData {
                    level: Some(2),
                    rate: Some("12".into()),
                    down_salesman_rate: None,
                },
            ]),
        }];
        assert_eq!(custom_level_rate(&data, 2).as_deref(), Some("12"));
        assert!(custom_level_rate(&data, 3).is_none());
    }

    #[test]
    fn config_map_and_names() {
        let config = SalesmanProductService::config();
        assert_eq!(config.len(), 4);
        assert_eq!(commission_type_text(COMMISSION_DEFAULT), "默认");
        assert_eq!(commission_type_text(COMMISSION_CUSTOM_PRICE), "自定义金额");
    }
}
