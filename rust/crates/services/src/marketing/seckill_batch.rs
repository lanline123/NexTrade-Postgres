//! 秒杀批处理 / 装修预览服务（marketing::seckill_batch）。
//!
//! 对应原 Java `SeckillServiceImpl` 的 3 个管理端端点：
//! `batch` / `listForDecorate` / `listForHomePreview`（见 models 模块头差异）。
//!
//! # 语义（逐条对照 Java）
//! - `batch`：注解校验（`未选择项目` / `操作类型不能为空`）；`type=del`
//!   时级联删除 `seckill` → `seckill_item` → `promotion(type=1)`；
//!   活动删除 0 行时跳过后续级联（等价 Java `isMove` 保护）。
//! - `listForDecorate`：进行中且 `seckill_stock > 0` 的秒杀商品列表
//!   （不分页，`Vec<SeckillPromotionVO>`），回填活动名/限购数与商品名。
//! - `listForHomePreview`：同窗口分页（默认 `page=1,size=15`），
//!   返回 `PageResult<SeckillListVO>`。
//!
//! # 金额
//! `seckill_price` 为 `Decimal`；`seckill_start_time/end_time` 为 Unix 秒。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_core::entities::{Product, SeckillItem};
use nextrade_core::models::marketing::seckill_batch::{
    is_delete, validate_batch, BatchDTO, HomePreviewQuery, SeckillListVO, SeckillPromotionVO,
    SeckkillData,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::seckill_batch::SeckillBatchRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 秒杀批处理服务（挂载于 `AdminState.marketing.seckill_batch`）。
pub struct SeckillBatchService {
    repos: Arc<Repositories>,
}

impl SeckillBatchService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.marketing.seckill_batch`）。
    #[must_use]
    pub fn repo(&self) -> &SeckillBatchRepository {
        &self.repos.marketing.seckill_batch
    }

    /// 批量操作（Java `SeckillServiceImpl#batch`）。
    ///
    /// # Errors
    /// 空 ids / 缺 type → `Validation`；数据库错误 → `Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let (ids, _op_type) = validate_batch(dto).map_err(Error::Validation)?;
        if !is_delete(dto) {
            // Java 非 del 分支直接 return false
            return Ok(false);
        }
        // 多租户：批量删除前逐条校验活动归属（seckill.shop_id）
        for id in &ids {
            if let Some(activity) = self.repos.marketing.seckill.find_by_id(*id).await? {
                TenantScope::current_ensure_access(activity.shop_id)?;
            }
        }
        let activities = self.repo().delete_activities(&ids).await?;
        if activities == 0 {
            // Java `isMove == false` 时不再清理子表
            return Ok(false);
        }
        let items = self.repo().delete_items(&ids).await?;
        let promotions = self.repo().delete_promotions(&ids).await?;
        tracing::info!(
            "秒杀批量删除: ids={ids:?} activities={activities} items={items} promotions={promotions}"
        );
        Ok(true)
    }

    /// 装修用进行中秒杀商品列表（Java `listForDecorate`，不分页）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list_for_decorate(&self) -> Result<Vec<SeckillPromotionVO>> {
        let now = now_secs_i32();
        // Java 不分页（`Page` 默认 size 较大）；取前 200 条以贴合装修场景。
        let (_total, items) = self.repo().find_active_items(now, 200, 0).await?;
        let list = self.assemble(&items).await?;
        self.enrich_for_preview(list).await
    }

    /// 首页装修预览（Java `listForHomePreview` → C 端 `getSeckillProductList`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list_for_home_preview(
        &self,
        q: &HomePreviewQuery,
    ) -> Result<PageResult<SeckillListVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.size.unwrap_or(15),
        };
        let now = now_secs_i32();
        let (limit, offset) = page.limit_offset();
        let (total, items) = self.repo().find_active_items(now, limit, offset).await?;
        let list = self.assemble(&items).await?;
        let list = self.enrich_for_preview(list).await?;
        Ok(PageResult::new(total, list, &page))
    }

    /// 回填装修预览扩展字段：商品图 / 商品价 / 市场价 + 秒杀活动概要
    /// （格式化起止时间、剩余秒数、开始状态）。
    ///
    /// Java `listForHomePreview` 复用 C 端 `getSeckillProductList` 的富结构
    /// （`SeckillListVO` 30+ 字段），编辑器预览的商品图、划线价、折扣角标、
    /// 倒计时都依赖这些字段；此前 Rust 精简 VO 缺失导致预览图空白、
    /// 倒计时恒 00:00:00。此处按 id 批量回填（避免 N+1），缺数据时保持
    /// `None`（JSON 省略），不影响既有消费方。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    async fn enrich_for_preview(
        &self,
        list: Vec<SeckillPromotionVO>,
    ) -> Result<Vec<SeckillPromotionVO>> {
        if list.is_empty() {
            return Ok(list);
        }
        let product_ids: Vec<i32> = list
            .iter()
            .map(|v| v.product_id)
            .filter(|id| *id > 0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let products: HashMap<i32, Product> = if product_ids.is_empty() {
            HashMap::new()
        } else {
            self.repos
                .catalog
                .product
                .find_by_ids(&product_ids)
                .await?
                .into_iter()
                .map(|p| (p.product_id, p))
                .collect()
        };
        let seckill_ids: Vec<i32> = list
            .iter()
            .map(|v| v.seckill_id)
            .filter(|id| *id > 0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let mut activities: HashMap<i32, nextrade_core::entities::Seckill> = HashMap::new();
        for sid in &seckill_ids {
            if let Some(a) = self.repos.marketing.seckill.find_by_id(*sid).await? {
                activities.insert(a.seckill_id, a);
            }
        }
        let now = now_secs_i32();
        Ok(list
            .into_iter()
            .map(|mut vo| {
                if let Some(p) = products.get(&vo.product_id) {
                    vo.pic_url = Some(p.pic_url.clone());
                    vo.pic_thumb = Some(p.pic_thumb.clone());
                    vo.product_price = Some(p.product_price);
                    vo.market_price = Some(p.market_price);
                }
                if let Some(a) = activities.get(&vo.seckill_id) {
                    let start = a.seckill_start_time.unwrap_or(0);
                    let end = a.seckill_end_time.unwrap_or(0);
                    let started = now >= start;
                    let remain_seconds = if now > end {
                        0
                    } else if started {
                        i64::from(end - now)
                    } else {
                        i64::from(start - now)
                    };
                    vo.seckkill_data = Some(SeckkillData {
                        seckill_id: a.seckill_id,
                        seckill_name: a.seckill_name.clone(),
                        seckill_start_time: a.seckill_start_time.and_then(format_secs),
                        seckill_end_time: a.seckill_end_time.and_then(format_secs),
                        seckill_limit_num: a.seckill_limit_num,
                        remain_seconds: Some(remain_seconds),
                        started: Some(started),
                    });
                }
                vo
            })
            .collect())
    }

    /// 批量组装 VO：回填活动名/限购数 + 商品名。
    async fn assemble(&self, items: &[SeckillItem]) -> Result<Vec<SeckillPromotionVO>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let activity_ids: Vec<i32> = items.iter().filter_map(|i| i.seckill_id).collect();
        let activities: HashMap<i32, (Option<String>, Option<i32>)> = self
            .repo()
            .find_activities(&activity_ids)
            .await?
            .into_iter()
            .map(|(id, name, limit)| (id, (name, limit)))
            .collect();
        let product_ids: Vec<i32> = items.iter().filter_map(|i| i.product_id).collect();
        let names = self.repo().find_product_names(&product_ids).await?;
        Ok(items
            .iter()
            .map(|it| {
                let seckill_id = it.seckill_id.unwrap_or(0);
                let product_id = it.product_id.unwrap_or(0);
                let (activity_name, activity_limit) =
                    activities.get(&seckill_id).cloned().unwrap_or((None, None));
                SeckillPromotionVO {
                    seckill_id,
                    product_id,
                    product_name: it
                        .product_id
                        .and_then(|pid| names.get(&pid).cloned())
                        .or(activity_name),
                    sku_id: it.sku_id.unwrap_or(0),
                    seckill_price: it.seckill_price,
                    seckill_stock: it.seckill_stock,
                    seckill_sales: it.seckill_sales,
                    seckill_start_time: it.seckill_start_time,
                    seckill_end_time: it.seckill_end_time,
                    seckill_limit_num: activity_limit,
                    ..Default::default()
                }
            })
            .collect())
    }
}

fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

/// Unix 秒 → `YYYY-MM-DD HH:mm:ss`（本地时区，与 Java 返回口径一致）。
fn format_secs(secs: i32) -> Option<String> {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(i64::from(secs), 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_validation_texts() {
        let empty = BatchDTO::default();
        assert_eq!(
            validate_batch(&empty)
                .map_err(Error::Validation)
                .unwrap_err()
                .to_string(),
            "参数校验失败: 未选择项目"
        );
    }

    #[test]
    fn non_delete_returns_false_like_java() {
        let dto = BatchDTO {
            ids: Some(vec![1]),
            r#type: Some("enable".into()),
            ..Default::default()
        };
        assert!(!is_delete(&dto));
    }

    #[tokio::test]
    async fn tenant_scope_blocks_cross_shop_batch_delete() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        assert!(
            with_tenant_scope(shop, async { TenantScope::current_ensure_access(9) })
                .await
                .is_err()
        );
        assert!(
            with_tenant_scope(shop, async { TenantScope::current_ensure_access(8) })
                .await
                .is_ok()
        );
    }

    #[test]
    fn home_preview_defaults() {
        let q = HomePreviewQuery::default();
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.size.unwrap_or(15),
        };
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 15);
    }
}
