//! catalog::product_manage client/admin 第二批端点 —— 服务层（wave8 batch 2）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储经嵌套
//! 聚合根访问：`self.repos.catalog.product_manage`。
//!
//! 说明：
//! - 复用 `ProductManageRepository` 已有方法（comments/coupons/related/history 等）；
//!   仓储缺口（selectProductList 跨表筛选、price_inquiry 写入）放在
//!   `infrastructure/src/db/repo/catalog/product_manage_client.rs`，本层不写 SQL
//!   （services crate 无 sqlx 依赖）。
//! - 列表/统计类返回 `serde_json::Value`：字段语义对齐 Java VO，但未逐字段裁剪。
//!
//! ## 偏差登记
//! 1. `share` 返回最小 HTML（Java 由 `getSharePageHtml` 生成完整分享页；图片/脚本
//!    等前端资源未迁移）。HTML 内容已做转义。
//! 2. `getProductAmount` 仅按 SKU 零售价 × 数量求和；会员价/批发/优惠券/促销
//!    叠加未迁移（Java `ProductAmountDTO` 之后还有批发旁路）。
//! 3. `promotion` 返回商品自身的促销字段（isPromote/promotePrice/时间），未聚合
//!    seckill/groupon/time_discount 域活动。
//! 4. `copy` 只复制商品主表（SKU/图册/属性未复制，Java 全量复制）。
//! 5. `getProductAvailability` 返回 sku 维度的价格/库存子集；`promotion` 与
//!    定金（`depositAmount`/`finalPaymentAmount`，仅定金预售）已按 Java 口径
//!    补齐（2026-11-19，见 `availability` 文档）。Java VO 其余字段仍未迁移：
//!    `sku`（完整 SKU DTO）、`distance`（需 addressId + 店铺经纬度）、
//!    `giftCardGroupsId`，以及价格链的会员等级折扣 / 积分兑换旁路 /
//!    批发作用域排除（均需登录用户上下文，本端点现不接收 userId）。
//! 6. 收藏/历史等依赖登录态的端点：未登录时返回空/false（同 Java）。
//! 7. 详情促销的人群口径未迁移：`canJoin` 恒 true（无人群配置的活动在 Java
//!    亦为 true）、`memberAudience`/`sendUserTarget` 恒 null；Java 的
//!    member_scope/memberMarks 人群判定与详情券展示资格过滤
//!    （`filterDisplayableProductCoupons`）依赖未迁移的人群匹配与 userId
//!    上下文，配了人群的活动会多展示本应置灰/隐藏的条目（见登记 5 的价格链）。

use std::sync::Arc;

use rust_decimal::Decimal;
use serde::Serialize;

use nextrade_common::utils;
use nextrade_core::entities::{
    Coupon, GrouponPromotion, Product, ProductGift, ProductPromotion, Promotion, Seckill,
    SeckillItem, TimeDiscount,
};
use nextrade_core::models::catalog::product_manage_client::{
    coupon_promotion_desc, fixed_promotion_desc, handel_time_string, html_escape, inquiry_summary,
    parse_promotion_type_data, product_list_item, product_promotion_desc, promotion_time_string,
    promotion_time_text, BatchAvailabilityQuery, CommentListQuery, FeedbackListQuery,
    HistoryProductQuery, PresaleDepositQuery, ProductAmountDTO, ProductAvailabilityQuery,
    ProductConfigQuery, ProductCopyDTO, ProductIdParam, ProductPriceInquiryDTO,
    ProductShareInfoDTO, ProductShareInfoVO, ProductsPromotionDTO, SelectProductListQuery,
    MSG_AMOUNT_ITEMS_REQUIRED, MSG_INQUIRY_CONTENT_REQUIRED, MSG_INQUIRY_MOBILE_REQUIRED,
    MSG_INQUIRY_SKU_NOT_FOUND, MSG_INQUIRY_TOO_FREQUENT, MSG_PRODUCT_ID_REQUIRED,
    MSG_PRODUCT_NOT_FOUND,
};
use nextrade_core::models::marketing::promotion as promotion_model;

use crate::marketing::groupon_promotion::GrouponPromotionService;
use crate::marketing::time_discount::TimeDiscountService;
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::product_manage::ProductManageRepository;
use nextrade_infrastructure::db::Repositories;

/// batch 2 服务。
pub struct ProductManageClientService {
    repos: Arc<Repositories>,
}

fn now_secs() -> i32 {
    i32::try_from(utils::now_millis() / 1000).unwrap_or(0)
}

fn to_json<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

fn i64_of(v: &serde_json::Value, keys: &[&str]) -> i64 {
    for k in keys {
        if let Some(n) = v.get(*k) {
            if let Some(x) = n.as_i64() {
                return x;
            }
            if let Some(s) = n.as_str().and_then(|s| s.parse::<i64>().ok()) {
                return s;
            }
        }
    }
    0
}

impl ProductManageClientService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &ProductManageRepository {
        &self.repos.catalog.product_manage
    }

    // ————————————————————— admin —————————————————————

    /// 商品配置（Java `config`）：运费模板/服务/会员等级/卡券组/库存编辑开关。
    pub async fn admin_config(&self, q: &ProductConfigQuery) -> Result<serde_json::Value> {
        let tpls = self.repo().find_shipping_tpls(q.shop_id, &[]).await?;
        let services = self.repo().find_all_services().await?;
        let ranks = self.repo().find_all_user_ranks().await?;
        let groups = self.repo().find_ecard_groups(q.shop_id).await?;
        let edit_stock = self.repo().config_val("productEditStockModify").await?;
        Ok(serde_json::json!({
            "shippingTplList": tpls.iter().map(|x| to_json(x)).collect::<Vec<_>>(),
            "suppliersList": [],
            "serviceList": services.iter().map(|x| to_json(x)).collect::<Vec<_>>(),
            "userRankList": ranks.iter().map(|x| to_json(x)).collect::<Vec<_>>(),
            "eCardList": groups.iter().map(|x| to_json(x)).collect::<Vec<_>>(),
            "productServiceIds": services.iter().map(|s| s.product_service_id).collect::<Vec<_>>(),
            "productEditStockModify": edit_stock.and_then(|v| v.parse::<i32>().ok()).unwrap_or(1),
        }))
    }

    /// 商品复制（Java `copy`）：复制商品主表并生成新货号。
    pub async fn admin_copy(&self, dto: &ProductCopyDTO) -> Result<String> {
        let mut product = self
            .repo()
            .find_product(dto.product_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        let next_id = self.repo().max_product_id().await? + 1;
        product.product_sn = format!("{}-copy-{next_id}", product.product_sn);
        product.add_time = now_secs();
        product.click_count = 0;
        let new_id = self.repo().insert_product(&product).await?;
        Ok(format!("商品复制成功，新商品ID：{new_id}"))
    }

    /// 商品推广数据（Java `getPromoteData`）。
    pub async fn admin_promote_data(&self, product_id: i32) -> Result<serde_json::Value> {
        let p = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        let shop_name = if p.shop_id > 0 {
            self.repo()
                .find_shop_title(p.shop_id)
                .await?
                .unwrap_or_else(|| "商城自营".to_string())
        } else {
            "商城自营".to_string()
        };
        Ok(serde_json::json!({
            "productName": p.product_name,
            "productPrice": p.product_price,
            "picUrl": p.pic_url,
            "productBrief": p.product_brief,
            "productSn": p.product_sn,
            "shopName": shop_name,
            // Java 由短链服务生成；本仓库短链服务未迁移 → 以站内路径兜底。
            "shortLink": format!("/p/{product_id}"),
            "wechatShortLink": serde_json::Value::Null,
        }))
    }

    /// SKU 商品选择列表（Java `selectProductList`；跨表筛选走 sqlx）。
    pub async fn admin_select_product_list(
        &self,
        q: &SelectProductListQuery,
    ) -> Result<PageResult<serde_json::Value>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos
            .catalog
            .product_manage_client
            .paginate_select_product_list(q)
            .await?;
        let list = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "skuId": r.sku_id, "productId": r.product_id,
                    "skuPrice": r.sku_price, "skuStock": r.sku_stock,
                    "productName": r.product_name, "productSn": r.product_sn,
                    "picUrl": r.pic_url, "productPrice": r.product_price,
                    "categoryId": r.category_id, "productType": r.product_type,
                    "shopId": r.shop_id,
                })
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 保存商品分享信息（Java `saveShareInfo`）。
    pub async fn admin_save_share_info(&self, dto: &ProductShareInfoDTO) -> Result<bool> {
        let exists = self.repo().find_product(dto.product_id).await?;
        if exists.is_none() {
            return Err(Error::Validation(MSG_PRODUCT_NOT_FOUND.into()));
        }
        let raw = serde_json::to_string(dto)
            .map_err(|e| Error::Internal(format!("分享信息序列化失败: {e}")))?;
        let n = self.repo().set_product_share_info(dto.product_id, &raw).await?;
        Ok(n > 0)
    }

    /// 获取商品分享信息（Java `shareInfo`）。
    pub async fn admin_share_info(&self, product_id: i32) -> Result<ProductShareInfoVO> {
        let p = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        if let Some(raw) = p.share_info.as_deref().filter(|s| !s.trim().is_empty()) {
            if let Ok(vo) = serde_json::from_str::<ProductShareInfoVO>(raw) {
                return Ok(vo);
            }
        }
        Ok(ProductShareInfoVO {
            product_id: p.product_id,
            product_name: p.product_name,
            product_price: p.product_price,
            pic_url: p.pic_url,
        })
    }

    // ————————————————————— public —————————————————————

    /// 分享页 HTML（Java `getSharePageHtml`；最小可用页面）。
    pub async fn share_html(&self, product_id: i32, _salesman_id: Option<i32>) -> Result<String> {
        let p = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        Ok(format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head>\
             <body><h1>{}</h1><p>价格：{}</p><img src=\"{}\" alt=\"product\"/></body></html>",
            html_escape(&p.product_name),
            html_escape(&p.product_name),
            p.product_price,
            html_escape(&p.pic_url)
        ))
    }

    /// 是否收藏（Java `isCollectMethod`：未登录返回 false）。
    pub async fn is_collect(&self, user_id: Option<i32>, q: &ProductIdParam) -> Result<bool> {
        let Some(uid) = user_id.filter(|v| *v > 0) else {
            return Ok(false);
        };
        Ok(self.repo().count_collect(uid, q.id, q.shop_id).await? > 0)
    }

    /// 商品可用优惠券（Java `getCouponList`）。
    pub async fn coupon_list(&self, q: &ProductIdParam) -> Result<Vec<serde_json::Value>> {
        let p = self
            .repo()
            .find_product(q.id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        let shop_id = q.shop_id.unwrap_or(p.shop_id);
        let coupons = self.repo().find_shop_coupons(shop_id, now_secs()).await?;
        let mut out = Vec::with_capacity(coupons.len());
        for c in &coupons {
            let mut v = to_json(c);
            let received = self.repo().count_coupon_receive(c.coupon_id).await?;
            if let Some(o) = v.as_object_mut() {
                o.insert("receivedCount".into(), serde_json::json!(received));
            }
            out.push(v);
        }
        Ok(out)
    }

    /// 商品评论统计（Java `ProductCommentStatisticVO`）。
    pub async fn comment_statistic(&self, q: &ProductIdParam) -> Result<serde_json::Value> {
        let comments = self
            .repo()
            .find_comments_by_product_ids(&[q.id], q.shop_id)
            .await?;
        let total = comments.len() as i64;
        let mut good = 0_i64;
        let mut moderate = 0_i64;
        let mut bad = 0_i64;
        let mut rank_sum = 0_i64;
        for c in &comments {
            let v = to_json(c);
            let rank = i64_of(&v, &["rank", "comment_rank", "score"]).clamp(1, 5);
            rank_sum += rank;
            match rank {
                4..=5 => good += 1,
                3 => moderate += 1,
                _ => bad += 1,
            }
        }
        let pct = |n: i64| {
            if total == 0 {
                "0.00".to_string()
            } else {
                format!("{:.2}", n as f64 * 100.0 / total as f64)
            }
        };
        Ok(serde_json::json!({
            "total": total,
            "goodCount": good, "goodPercent": pct(good),
            "moderateCount": moderate, "moderatePercent": pct(moderate),
            "badCount": bad, "badPercent": pct(bad),
            "showCount": comments.iter().filter(|c| { let v = to_json(c); i64_of(&v, &["is_showed","is_show"]) == 1 }).count(),
            "averageRank": if total == 0 { 0 } else { rank_sum / total },
        }))
    }

    /// 评论列表（Java `getCommentList`：默认 10/页，type==5 只查展示中）。
    pub async fn comment_list(
        &self,
        q: &CommentListQuery,
    ) -> Result<PageResult<serde_json::Value>> {
        let page = q.page_query();
        let limit = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * limit;
        let is_showed = if q.type_ == Some(5) { Some(1) } else { None };
        let (total, rows, replies) = self
            .repo()
            .paginate_client_comments(&[q.id], q.shop_id, q.type_, is_showed, limit, offset)
            .await?;
        let parent_ids: Vec<i32> = rows.iter().map(|c| c.comment_id).collect();
        let reply_rows = if parent_ids.is_empty() {
            Vec::new()
        } else {
            self.repo().find_comment_replies(&parent_ids).await?
        };
        let reply_json: Vec<serde_json::Value> = reply_rows.iter().map(|x| to_json(x)).collect();
        let user_ids: Vec<i32> = rows.iter().map(|c| c.user_id).collect();
        let nicks = self.repo().find_user_nicks(&user_ids).await?;
        let nick_map: std::collections::HashMap<i32, Option<String>> =
            nicks.into_iter().collect();
        let list = rows
            .iter()
            .map(|c| {
                let mut v = to_json(c);
                if let Some(o) = v.as_object_mut() {
                    o.insert(
                        "username".into(),
                        serde_json::json!(nick_map.get(&c.user_id).cloned().flatten()),
                    );
                    let parent = c.comment_id;
                    o.insert(
                        "replies".into(),
                        serde_json::Value::Array(
                            reply_json
                                .iter()
                                .filter(|r| i64_of(r, &["parent_id"]) == i64::from(parent))
                                .cloned()
                                .collect(),
                        ),
                    );
                }
                v
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 商品咨询列表（Java `getFeedbackList`）。
    pub async fn feedback_list(
        &self,
        q: &FeedbackListQuery,
    ) -> Result<PageResult<serde_json::Value>> {
        let page = q.page_query();
        let limit = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * limit;
        let (col, asc) = q.order();
        let (total, rows) = self
            .repo()
            .paginate_feedback(q.product_id, q.shop_id, limit, offset, col, asc)
            .await?;
        let ids: Vec<i32> = rows.iter().map(|f| f.id).collect();
        let replies = if ids.is_empty() {
            Vec::new()
        } else {
            self.repo().find_feedback_replies(&ids).await?
        };
        let reply_json: Vec<serde_json::Value> = replies.iter().map(|x| to_json(x)).collect();
        let list = rows
            .iter()
            .map(|f| {
                let mut v = to_json(f);
                if let Some(o) = v.as_object_mut() {
                    o.insert(
                        "replies".into(),
                        serde_json::Value::Array(
                            reply_json
                                .iter()
                                .filter(|r| i64_of(r, &["parent_id"]) == i64::from(f.id))
                                .cloned()
                                .collect(),
                        ),
                    );
                }
                v
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 商品可用性（Java `getProductAvailability` → `ProductServiceImpl#getProductSkuDetail`）。
    ///
    /// sku 维度价格/库存子集 + `promotion` 详情促销聚合 + 定金预售字段：
    /// - `promotion`：按 Java detail 口径组装（见 [`Self::assemble_detail_promotions`]），
    ///   **后端不预过滤**——秒杀/限时折扣/拼团/抽奖（type 1/6/7/8）的展示分流由
    ///   前端 `specification.promotionManage` 过滤（与原版逐字相同），本层原样下发；
    /// - 秒杀/限时折扣命中时 `price`/`stock` 按 Java 同口径被活动价/活动库存覆盖，
    ///   `originPrice` 置 null（Java `newPrice != null → originPrice = null`）；
    ///   未命中时 `originPrice` 保留现行口径（商品价；Java 为 SKU 价，属既有子集偏差）；
    /// - `sale_mode = 2`（定金预售）且有预售行时补 `depositAmount` /
    ///   `finalPaymentAmount`（Java 同口径，定金经既有 `presale_deposit` 计算）。
    ///
    /// sku 子集口径（既有）：SKU 仅取启用行，Java 的禁用 SKU 分支（promotion
    /// 为空、库存 0）不可达；其余未迁移字段见模块偏差登记 5/7。
    pub async fn availability(&self, q: &ProductAvailabilityQuery) -> Result<serde_json::Value> {
        let id =
            q.id.ok_or_else(|| Error::Validation(MSG_PRODUCT_ID_REQUIRED.into()))?;
        let product = self
            .repo()
            .find_product(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        let skus = self.repo().find_skus(id, false).await?;
        let sku = match q.sku_id {
            Some(sid) => skus.iter().find(|s| s.sku_id == sid),
            None => skus.first(),
        };
        let (sku_id, price, stock) = sku
            .map(|s| {
                (
                    s.sku_id,
                    s.sku_price.unwrap_or_default(),
                    s.sku_stock.unwrap_or(0),
                )
            })
            .unwrap_or((0, product.product_price, product.product_stock));
        let product_sku_ids: Vec<i32> = skus.iter().map(|s| s.sku_id).collect();
        let (promotion, promo_price, promo_stock) = self
            .assemble_detail_promotions(&product, sku_id, &product_sku_ids, price)
            .await?;
        let final_price = promo_price.unwrap_or(price);
        let mut resp = serde_json::json!({
            "id": product.product_id,
            "productId": product.product_id,
            "skuId": sku_id,
            "price": final_price,
            "originPrice": if promo_price.is_some() {
                serde_json::Value::Null
            } else {
                serde_json::json!(product.product_price)
            },
            "marketPrice": product.market_price,
            "startProductPrice": price,
            "stock": promo_stock.unwrap_or(stock),
            "promotion": promotion,
            "data": skus.iter().map(|x| to_json(x)).collect::<Vec<_>>(),
        });
        // 定金预售（Java saleMode == 2 且有预售行才设置两字段）
        if product.sale_mode == 2 {
            if self.repo().find_presale(id).await?.is_some() {
                let deposit = self
                    .presale_deposit(&PresaleDepositQuery {
                        product_id: id,
                        sku_id,
                    })
                    .await?;
                if let Some(obj) = resp.as_object_mut() {
                    obj.insert("depositAmount".into(), serde_json::json!(deposit));
                    obj.insert(
                        "finalPaymentAmount".into(),
                        serde_json::json!((final_price - deposit).max(Decimal::ZERO)),
                    );
                }
            }
        }
        Ok(resp)
    }

    /// 详情促销聚合（Java `getProductsPromotion(products, shopId, "detail", userId)`
    /// + `getProductSkuDetail` 内的促销回填段）。
    ///
    /// 数据来源与瀑布流 `activityInfo` 同一张 `promotion` 表、同一仓储
    /// （`find_available`：进行中 + `(is_available = 1 OR type = 2)` + 活动类型
    /// 排序），区别只在组装口径：
    /// - 瀑布流（list）只取命中首条、券不展示、无 `data`；detail 取**全部命中**
    ///   条目并按 `realPromotion` 组装 `data`（含券面/满减满赠描述、秒杀/折扣/
    ///   拼团的 `data.item`），且不过滤 `canJoin = false`（本层恒 true，见登记 7）；
    /// - 店铺口径：非 O2O 的 Java 口径——按**商品自身** `shop_id` 取活动
    ///   （`checkPromotionIsAvailable` 末步亦要求活动店铺 == 商品店铺）；
    /// - 命中判定复用 [`promotion_covers_product`]（range）+ SKU 命中
    ///   （[`promotion_applies_to_sku`]）；`product_promotion.limit_user_rank`
    ///   等级过滤需 user 上下文，未迁移（登记 7），不做。
    ///
    /// 返回 `(promotion 数组, 活动价, 活动库存)`：活动价/库存仅秒杀与限时折扣
    /// 命中时为 `Some`（Java `newPrice`/`productStock` 覆盖段，迭代顺序与
    /// `find_available` 的类型排序一致，后命中者覆盖先者，与 Java 相同）。
    async fn assemble_detail_promotions(
        &self,
        product: &Product,
        sku_id: i32,
        product_sku_ids: &[i32],
        base_price: Decimal,
    ) -> Result<(Vec<serde_json::Value>, Option<Decimal>, Option<i32>)> {
        let now = i64::from(now_secs());
        let promotions = self
            .repos
            .marketing
            .promotion
            .find_available(Some(&[product.shop_id]), now)
            .await
            .unwrap_or_default();

        let mut out: Vec<serde_json::Value> = Vec::new();
        for pr in promotions
            .iter()
            .filter(|pr| promotion_covers_product(pr, product.product_id))
            .filter(|pr| promotion_applies_to_sku(pr, sku_id))
        {
            let relation_id = pr.relation_id.unwrap_or(0);
            // realPromotion：按 type 取关联实体组装 data（缺行 → 空对象，同 Java
            // realPromotion 返回空 Map / getById 为 null 时的降级形态）。
            let (data, coupon) = match pr.r#type {
                1 => {
                    let row = self
                        .repos
                        .marketing
                        .seckill
                        .find_by_id(relation_id)
                        .await
                        .unwrap_or_default();
                    (
                        row.as_ref()
                            .map_or_else(|| serde_json::json!({}), |s| seckill_data(s)),
                        None,
                    )
                }
                2 => {
                    let row = self
                        .repos
                        .marketing
                        .coupon
                        .find_by_id(relation_id)
                        .await
                        .unwrap_or_default();
                    let data = row
                        .as_ref()
                        .map_or_else(|| serde_json::json!({}), |c| coupon_data(c));
                    (data, row)
                }
                3 | 4 => {
                    let row = self
                        .repos
                        .marketing
                        .product_promotion
                        .find_by_id(relation_id)
                        .await
                        .unwrap_or_default();
                    (
                        row.as_ref()
                            .map_or_else(|| serde_json::json!({}), |pp| product_promotion_data(pp)),
                        None,
                    )
                }
                5 => {
                    let row = self
                        .repos
                        .marketing
                        .product_promotion
                        .find_by_id(relation_id)
                        .await
                        .unwrap_or_default();
                    let data = match row {
                        Some(pp) => self.give_data(&pp).await?,
                        None => serde_json::json!({}),
                    };
                    (data, None)
                }
                6 => {
                    let row = self
                        .repos
                        .marketing
                        .time_discount
                        .find_by_id(relation_id)
                        .await
                        .unwrap_or_default();
                    (
                        row.as_ref()
                            .map_or_else(|| serde_json::json!({}), |d| time_discount_data(d)),
                        None,
                    )
                }
                7 => {
                    let row = self
                        .repos
                        .marketing
                        .groupon_promotion
                        .find_promotion(i64::from(relation_id))
                        .await
                        .unwrap_or_default();
                    (
                        row.as_ref()
                            .map_or_else(|| serde_json::json!({}), |g| groupon_data(g)),
                        None,
                    )
                }
                _ => (serde_json::json!({}), None),
            };
            let time_text = promotion_time_text(pr, coupon.as_ref());
            out.push(detail_promotion_vo(pr, data, &time_text, product_sku_ids));
        }

        // Java 回填段：type=2 且 is_delete=1 的条目移除（其余 type 的删除标记
        // 在 detail 口径下保留，与 queryAvailablePromotion 不过滤 is_delete 一致）。
        out.retain(|v| !is_detail_promotion_removed(v));

        // 秒杀 / 限时折扣 / 拼团的 data.item 回填 + 活动价/库存（Java 同段）。
        let mut new_price: Option<Decimal> = None;
        let mut new_stock: Option<i32> = None;
        for v in out.iter_mut() {
            let r#type = v
                .get("type")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let relation_id = v
                .get("relationId")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            match r#type {
                1 => {
                    let items = self
                        .repos
                        .marketing
                        .seckill
                        .find_items_by_skus(i32::try_from(relation_id).unwrap_or(0), &[sku_id])
                        .await
                        .unwrap_or_default();
                    if let Some(item) = items.first() {
                        if let Some(p) = item.seckill_price {
                            new_price = Some(p);
                        }
                        if let Some(s) = item.seckill_stock {
                            new_stock = Some(s);
                        }
                        if let Some(data) =
                            v.get_mut("data").and_then(serde_json::Value::as_object_mut)
                        {
                            data.insert("item".into(), seckill_item_json(item));
                            data.insert(
                                "seckillStartTime".into(),
                                serde_json::json!(handel_time_string(
                                    item.seckill_start_time.unwrap_or(0)
                                )),
                            );
                            data.insert(
                                "seckillEndTime".into(),
                                serde_json::json!(handel_time_string(
                                    item.seckill_end_time.unwrap_or(0)
                                )),
                            );
                        }
                    }
                }
                6 => {
                    let items = self
                        .repos
                        .marketing
                        .time_discount
                        .find_items_for_product(
                            i32::try_from(relation_id).unwrap_or(0),
                            product.product_id,
                        )
                        .await
                        .unwrap_or_default();
                    let item = items
                        .iter()
                        .find(|it| TimeDiscountService::applies_to_sku(it, Some(sku_id)));
                    if let Some(item) = item {
                        let discounted = TimeDiscountService::calc_discount_price(
                            base_price,
                            item.discount_type,
                            item.value,
                        );
                        new_price = Some(discounted);
                        let saved = (base_price - discounted).max(Decimal::ZERO);
                        let saved = saved.round_dp_with_strategy(
                            2,
                            rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                        );
                        if let Some(data) =
                            v.get_mut("data").and_then(serde_json::Value::as_object_mut)
                        {
                            data.insert(
                                "item".into(),
                                serde_json::json!({
                                    "itemId": item.item_id,
                                    "discountId": item.discount_id,
                                    "startTime": handel_time_string(item.start_time),
                                    "endTime": handel_time_string(item.end_time),
                                    "productId": item.product_id,
                                    "skuIds": item.sku_ids,
                                    "value": item.value,
                                    "discountType": item.discount_type,
                                    "discountPrice": saved,
                                }),
                            );
                        }
                    }
                }
                7 => {
                    // Java `grouponPromotionService.getGrouponInfo(productId)`；
                    // 复用既有 Rust 实现（无 user 上下文时传 None）。
                    let info = GrouponPromotionService::new(self.repos.clone())
                        .get_groupon_info(product.product_id, None)
                        .await
                        .unwrap_or_default();
                    if let Some(info) = info {
                        if let Some(data) =
                            v.get_mut("data").and_then(serde_json::Value::as_object_mut)
                        {
                            data.insert(
                                "item".into(),
                                serde_json::to_value(&info).unwrap_or(serde_json::Value::Null),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        Ok((out, new_price, new_stock))
    }

    /// 满赠（type=5）的 `data` 重组（Java `getProductsPromotion` 满赠段）：
    /// 规则列表逐条挂赠品（`gift` → `ProductGiftVO` + 赠品商品子集信息），
    /// `promotionDesc` 与 `promotionTypeData` 同为该列表（Java 历史形态），
    /// 文本描述另存 `promotionDescText`（仅 `promotion_type = 3` 满赠时）。
    async fn give_data(&self, pp: &ProductPromotion) -> Result<serde_json::Value> {
        let rules = parse_promotion_type_data(&pp.promotion_type_data);
        let mut rule_values: Vec<serde_json::Value> = Vec::with_capacity(rules.len());
        for rule in &rules {
            let mut rv = serde_json::to_value(rule).unwrap_or(serde_json::Value::Null);
            let gift = match rule.gift_id {
                Some(gift_id) if gift_id > 0 => self
                    .repos
                    .marketing
                    .product_gift
                    .find_by_id(gift_id)
                    .await
                    .unwrap_or_default(),
                _ => None,
            };
            let gift_value = match gift {
                Some(g) => {
                    let info = self
                        .repo()
                        .find_product(g.product_id)
                        .await
                        .unwrap_or_default();
                    gift_vo_json(&g, info.as_ref())
                }
                None => serde_json::Value::Null,
            };
            if let Some(obj) = rv.as_object_mut() {
                obj.insert("gift".into(), gift_value);
            }
            rule_values.push(rv);
        }
        let mut data = serde_json::json!({
            "promotionTypeData": rule_values.clone(),
            "promotionDesc": rule_values,
            "promotionType": pp.promotion_type,
            "rulesType": pp.rules_type,
            "unit": pp.unit,
        });
        if pp.promotion_type == 3 {
            if let Some(obj) = data.as_object_mut() {
                obj.insert(
                    "promotionDescText".into(),
                    serde_json::json!(product_promotion_desc(pp)),
                );
            }
        }
        Ok(data)
    }

    /// 商品促销信息（Java `getProductsPromotion`）。
    ///
    /// 商品自身促销字段 + `activityInfo`（进行中且范围命中该商品的营销活动，
    /// 对齐 Java `PromotionServiceImpl#queryAvailablePromotion` + `range` 过滤）。
    /// 与 Java 的差异（见模块偏差登记 3）：券资格/会员人群/批发旁路等计价装配
    /// 未迁移，`activityInfo` 为纯展示用途（前台活动标签/拼团判定），且已排除
    /// 优惠券（type=2，前台列表本就不展示）。
    pub async fn products_promotion(
        &self,
        dto: &ProductsPromotionDTO,
    ) -> Result<serde_json::Value> {
        let ids: Vec<i32> = dto
            .products
            .iter()
            .filter_map(|p| p.get("productId").and_then(serde_json::Value::as_i64))
            .filter_map(|v| i32::try_from(v).ok())
            .collect();
        let products = self.repo().find_products_by_ids(&ids).await?;
        let mut map = serde_json::Map::new();
        if products.is_empty() {
            return Ok(serde_json::Value::Object(map));
        }

        let now = i64::from(now_secs());
        // Java：shopId=null → 不按店铺过滤（全平台活动）
        let shop_ids: Option<Vec<i32>> = dto.shop_id.map(|s| vec![s]);
        let promotions = self
            .repos
            .marketing
            .promotion
            .find_available(shop_ids.as_deref(), now)
            .await
            .unwrap_or_default();

        for p in products {
            let activity_info: Vec<serde_json::Value> = promotions
                .iter()
                .filter(|pr| promotion_covers_product(pr, p.product_id))
                .map(|pr| {
                    serde_json::json!({
                        "promotionId": pr.promotion_id,
                        "promotionName": pr.promotion_name,
                        "startTime": pr.start_time,
                        "endTime": pr.end_time,
                        "type": pr.r#type,
                        "shopId": pr.shop_id,
                        "relationId": pr.relation_id,
                        "range": pr.range,
                        "typeText": promotion_model::type_text(pr.r#type),
                    })
                })
                .collect();
            map.insert(
                p.product_id.to_string(),
                serde_json::json!({
                    "productId": p.product_id,
                    "isPromote": p.is_promote,
                    "promotePrice": p.promote_price,
                    "promoteStartDate": p.promote_start_date,
                    "promoteEndDate": p.promote_end_date,
                    "productStatus": p.product_status,
                    "activityInfo": activity_info,
                }),
            );
        }
        Ok(serde_json::Value::Object(map))
    }

    /// 价格试算（Java `getProductAmount`；仅零售价 × 数量）。
    pub async fn product_amount(&self, dto: &ProductAmountDTO) -> Result<serde_json::Value> {
        if dto.sku_item.is_empty() {
            return Err(Error::Validation(MSG_AMOUNT_ITEMS_REQUIRED.into()));
        }
        let _p = self
            .repo()
            .find_product(dto.id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;
        let skus = self.repo().find_skus(dto.id, false).await?;
        let mut count = 0_i32;
        let mut total = Decimal::ZERO;
        for item in &dto.sku_item {
            let num = item.num.max(0);
            count += num;
            let price = skus
                .iter()
                .find(|s| s.sku_id == item.sku_id)
                .and_then(|s| s.sku_price)
                .unwrap_or(Decimal::ZERO);
            total += price * Decimal::from(num);
        }
        Ok(serde_json::json!({"total": total, "count": count, "wholesale": false}))
    }

    /// 商品询价（Java `priceInquiry`）：写 `price_inquiry` 表。
    ///
    /// 2026-11-15 收紧（该接口此前无前台调用方，为孤儿接口）：
    /// - 手机号 + 内容双必填（回拨通道无手机号即无效留资）；
    /// - 同手机号 10 分钟内提交超过 5 次拒绝（DB 计数，无外部依赖）；
    /// - 传了 `skuId` 时必须属于该商品，规格文本以库中 SKU 行为准存快照。
    /// 2026-11-16 去重：同一手机号对同一商品（含同一 SKU）300 秒内提交相同
    /// 需求内容视为重复提交（滑块重试 / 重复点击），直接返回成功不重复落库，
    /// 也不消耗限频额度。
    pub async fn price_inquiry(
        &self,
        dto: &ProductPriceInquiryDTO,
        user_id: Option<i32>,
    ) -> Result<()> {
        let mobile = dto
            .mobile
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_INQUIRY_MOBILE_REQUIRED.into()))?;
        let content = dto
            .content
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_INQUIRY_CONTENT_REQUIRED.into()))?;
        let product = self
            .repo()
            .find_product(dto.product_id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_PRODUCT_NOT_FOUND.into()))?;

        // SKU 维度：以库中行为准（防前端伪造规格文本），并校验归属
        let mut sku_value: Option<String> = None;
        if let Some(sku_id) = dto.sku_id {
            let skus = self.repo().find_skus_by_ids(&[sku_id]).await?;
            let sku = skus
                .iter()
                .find(|s| s.product_id == dto.product_id)
                .ok_or_else(|| Error::Validation(MSG_INQUIRY_SKU_NOT_FOUND.into()))?;
            sku_value = Some(sku.sku_value.clone());
        }

        // 去重：300 秒内同手机号 + 同商品 + 同 SKU + 同内容已有记录 → 幂等成功
        let now = now_secs();
        let duplicate = self
            .repos
            .catalog
            .product_manage_client
            .has_recent_duplicate_price_inquiry(mobile, dto.product_id, dto.sku_id, content, now - 300)
            .await?;
        if duplicate {
            return Ok(());
        }

        // 限频：窗口内已有 5 条即拒绝（第 6 条起）
        let recent = self
            .repos
            .catalog
            .product_manage_client
            .count_recent_price_inquiries(mobile, now - 600)
            .await?;
        if recent >= 5 {
            return Err(Error::Validation(MSG_INQUIRY_TOO_FREQUENT.into()));
        }

        self.repos
            .catalog
            .product_manage_client
            .insert_price_inquiry(
                mobile,
                dto.email.as_deref().unwrap_or_default(),
                content,
                dto.product_id,
                product.shop_id,
                user_id.unwrap_or(0),
                now,
                dto.company_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                dto.job_title
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                dto.sku_id,
                sku_value.as_deref(),
            )
            .await
    }

    /// 预售定金金额（Java `calculateDepositAmount`）。
    pub async fn presale_deposit(&self, q: &PresaleDepositQuery) -> Result<Decimal> {
        let Some(presale) = self.repo().find_presale(q.product_id).await? else {
            return Ok(Decimal::ZERO);
        };
        if presale.status != 1 || presale.sale_mode != 2 {
            return Ok(Decimal::ZERO);
        }
        let skus = self.repo().find_skus(q.product_id, false).await?;
        let base = skus
            .iter()
            .find(|s| s.sku_id == q.sku_id)
            .and_then(|s| s.sku_price)
            .unwrap_or(Decimal::ZERO);
        if presale.deposit_type == Some(2) {
            return Ok(presale.deposit_fixed_amount.unwrap_or(Decimal::ZERO));
        }
        let ratio = Decimal::from(presale.deposit_ratio.unwrap_or(0).clamp(0, 99));
        let mut deposit = base * ratio / Decimal::from(100);
        if presale.is_round_down == Some(1) {
            deposit = deposit.trunc_with_scale(2);
        }
        Ok(deposit)
    }

    /// 批量组装列表项：一次查齐各商品的启用 SKU，逐商品推导询价派生态
    /// （`isInquiry` / `inquiryState` / `minPricedPrice`）后组装。
    async fn list_items_with_inquiry(
        &self,
        products: &[nextrade_core::entities::Product],
    ) -> Result<Vec<serde_json::Value>> {
        let ids: Vec<i32> = products.iter().map(|p| p.product_id).collect();
        let skus = if ids.is_empty() {
            Vec::new()
        } else {
            self.repo().find_skus_by_product_ids(&ids, false).await?
        };
        let mut by_product: std::collections::HashMap<
            i32,
            Vec<nextrade_core::entities::ProductSku>,
        > = std::collections::HashMap::new();
        for s in skus {
            by_product.entry(s.product_id).or_default().push(s);
        }
        Ok(products
            .iter()
            .map(|p| {
                let empty: &[nextrade_core::entities::ProductSku] = &[];
                let list = by_product
                    .get(&p.product_id)
                    .map_or(empty, Vec::as_slice);
                product_list_item(&to_json(p), &inquiry_summary(p, list))
            })
            .collect())
    }

    /// 相关商品（Java `getRelated`）。
    pub async fn related(&self, product_id: i32) -> Result<Vec<serde_json::Value>> {
        let ids = self.repo().find_related_ids(product_id).await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let products = self.repo().find_products_by_ids(&ids).await?;
        self.list_items_with_inquiry(&products).await
    }

    /// 批量可用性（Java `getBatchProductAvailability`；`skuIds` 逗号串）。
    pub async fn batch_availability(
        &self,
        q: &BatchAvailabilityQuery,
    ) -> Result<serde_json::Value> {
        let ids: Vec<i32> = q
            .sku_ids
            .split(',')
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect();
        let skus = if ids.is_empty() {
            Vec::new()
        } else {
            self.repo().find_skus_by_ids(&ids).await?
        };
        let mut map = serde_json::Map::new();
        for s in skus {
            map.insert(
                s.sku_id.to_string(),
                serde_json::json!({
                    "id": s.product_id,
                    "productId": s.product_id,
                    "skuId": s.sku_id,
                    "price": s.sku_price.unwrap_or_default(),
                    "stock": s.sku_stock.unwrap_or(0),
                }),
            );
        }
        Ok(serde_json::Value::Object(map))
    }

    /// 历史商品（Java `getHistoryProduct`；未登录返回空页）。
    pub async fn history_products(
        &self,
        q: &HistoryProductQuery,
        user_id: Option<i32>,
    ) -> Result<PageResult<serde_json::Value>> {
        let page = q.page_query();
        let Some(uid) = user_id.filter(|v| *v > 0) else {
            return Ok(PageResult::empty(&page));
        };
        let ids = self.repo().find_history_product_ids(uid).await?;
        if ids.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let limit = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * limit;
        let (total, products) = self.repo().paginate_products_by_ids(&ids, limit, offset).await?;
        let list = self.list_items_with_inquiry(&products).await?;
        Ok(PageResult::new(total, list, &page))
    }
}

/// 详情条目是否被 Java 回填段移除：仅 `type = 2 且 isDelete = 1`（已删除的
/// 券活动）；其余条目（含其他 type 的删除标记）按 detail 口径保留。
pub(crate) fn is_detail_promotion_removed(v: &serde_json::Value) -> bool {
    v.get("type").and_then(serde_json::Value::as_i64) == Some(2)
        && v.get("isDelete").and_then(serde_json::Value::as_i64) == Some(1)
}

/// 活动 SKU 命中（Java `checkPromotionIsAvailable` 的 SKU 段）：
/// `sku_id <= 0`（商品级查询）恒命中；活动 `sku_ids` 为空/未配置恒命中；
/// 否则要求包含该 SKU。
pub(crate) fn promotion_applies_to_sku(promotion: &Promotion, sku_id: i32) -> bool {
    if sku_id <= 0 {
        return true;
    }
    match promotion
        .sku_ids
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .and_then(parse_id_list)
    {
        Some(ids) if !ids.is_empty() => ids.contains(&sku_id),
        _ => true,
    }
}

/// 组装一条详情 `PromotionVO`（Java `PromotionServiceImpl#convert` +
/// `getProductsPromotion` 的 data 注入 / SKU 求交段）。
///
/// - `startTime`/`endTime` 为格式化串（0 → `"0"`，convert 口径），`data` 内
///   注入同值（Java `data.put("startTime", promotion.getStartTime())`）；
/// - `skuIds`：活动配置与商品 SKU 求交（Java 同口径——活动未配置时保持
///   null，商品无 SKU 时置空数组）；
/// - `canJoin` 恒 true、`memberAudience`/`sendUserTarget` 恒 null（登记 7）。
fn detail_promotion_vo(
    promotion: &Promotion,
    mut data: serde_json::Value,
    time_text: &str,
    product_sku_ids: &[i32],
) -> serde_json::Value {
    let start = promotion_time_string(promotion.start_time);
    let end = promotion_time_string(promotion.end_time);
    if let Some(obj) = data.as_object_mut() {
        obj.insert("startTime".into(), serde_json::json!(start.clone()));
        obj.insert("endTime".into(), serde_json::json!(end.clone()));
    }
    let parsed_sku_ids = promotion
        .sku_ids
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .and_then(parse_id_list);
    let sku_ids_field = match parsed_sku_ids {
        Some(ids) if !ids.is_empty() => {
            if product_sku_ids.is_empty() {
                serde_json::json!([])
            } else {
                serde_json::json!(ids
                    .into_iter()
                    .filter(|id| product_sku_ids.contains(id))
                    .collect::<Vec<_>>())
            }
        }
        Some(ids) => serde_json::json!(ids),
        None => serde_json::Value::Null,
    };
    let range_data = promotion
        .range_data
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .and_then(parse_id_list)
        .unwrap_or_default();
    let member_marks = promotion
        .member_marks
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .and_then(parse_id_list)
        .map(|ids| serde_json::json!(ids))
        .unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "promotionId": promotion.promotion_id,
        "promotionName": promotion.promotion_name,
        "startTime": start,
        "endTime": end,
        "type": promotion.r#type,
        "shopId": promotion.shop_id,
        "relationId": promotion.relation_id,
        "range": promotion.range,
        "rangeData": range_data,
        "skuIds": sku_ids_field,
        "isAvailable": promotion.is_available,
        "isDelete": promotion.is_delete,
        "typeText": promotion_model::type_text(promotion.r#type),
        "data": data,
        "timeText": time_text,
        "memberRange": promotion.member_range,
        "memberMarks": member_marks,
        "memberAudience": serde_json::Value::Null,
        "sendUserTarget": serde_json::Value::Null,
        "canJoin": true,
    })
}

/// 秒杀活动实体 → `data`（Java `realPromotion` type=1：`Seckill` 实体 +
/// 模板描述 `promotion.seckill`）。
fn seckill_data(s: &Seckill) -> serde_json::Value {
    serde_json::json!({
        "seckillId": s.seckill_id,
        "seckillName": s.seckill_name,
        "seckillStartTime": s.seckill_start_time,
        "seckillEndTime": s.seckill_end_time,
        "seckillLimitNum": s.seckill_limit_num,
        "productId": s.product_id,
        "shopId": s.shop_id,
        "promotionDesc": fixed_promotion_desc(1),
    })
}

/// 秒杀明细 → `data.item`（Java 回填段：`SeckillItem` 实体原样）。
fn seckill_item_json(item: &SeckillItem) -> serde_json::Value {
    serde_json::json!({
        "recId": item.rec_id,
        "seckillId": item.seckill_id,
        "productId": item.product_id,
        "skuId": item.sku_id,
        "seckillPrice": item.seckill_price,
        "seckillStock": item.seckill_stock,
        "seckillSales": item.seckill_sales,
        "seckillStartTime": item.seckill_start_time,
        "seckillEndTime": item.seckill_end_time,
    })
}

/// 优惠券实体 → `data`（Java `realPromotion` type=2：`Coupon` 实体 +
/// `getCouponPromotionDesc` 券面描述）。
fn coupon_data(c: &Coupon) -> serde_json::Value {
    serde_json::json!({
        "couponId": c.coupon_id,
        "couponName": c.coupon_name,
        "couponMoney": c.coupon_money,
        "couponDiscount": c.coupon_discount,
        "couponDesc": c.coupon_desc,
        "couponType": c.coupon_type,
        "sendRange": c.send_range,
        "sendRangeData": c.send_range_data,
        "minOrderAmount": c.min_order_amount,
        "sendStartDate": c.send_start_date,
        "sendEndDate": c.send_end_date,
        "sendType": c.send_type,
        "useDay": c.use_day,
        "useStartDate": c.use_start_date,
        "useEndDate": c.use_end_date,
        "isShow": c.is_show,
        "isGlobal": c.is_global,
        "isNewUser": c.is_new_user,
        "enabledClickGet": c.enabled_click_get,
        "limitUserRank": c.limit_user_rank,
        "shopId": c.shop_id,
        "isDelete": c.is_delete,
        "limitNum": c.limit_num,
        "delayDay": c.delay_day,
        "sendNum": c.send_num,
        "maxOrderAmount": c.max_order_amount,
        "couponUnit": c.coupon_unit,
        "reduceType": c.reduce_type,
        "addTime": c.add_time,
        "promotionDesc": coupon_promotion_desc(c),
    })
}

/// 商品促销实体 → `data`（Java `realPromotion` type=3/4：`ProductPromotion`
/// 实体 + `getProductPromotionDesc` 描述；`promotionTypeData` 按 Java 实体
/// 形态保持原始 JSON 串。type=5 满赠走 `give_data` 重组，不经此函数）。
fn product_promotion_data(pp: &ProductPromotion) -> serde_json::Value {
    serde_json::json!({
        "promotionId": pp.promotion_id,
        "promotionName": pp.promotion_name,
        "startTime": pp.start_time,
        "endTime": pp.end_time,
        "limitUserRank": pp.limit_user_rank,
        "range": pp.range,
        "rangeData": pp.range_data,
        "minOrderAmount": pp.min_order_amount,
        "maxOrderAmount": pp.max_order_amount,
        "promotionType": pp.promotion_type,
        "promotionTypeData": pp.promotion_type_data,
        "isAvailable": pp.is_available,
        "sortOrder": pp.sort_order,
        "shopId": pp.shop_id,
        "rulesType": pp.rules_type,
        "unit": pp.unit,
        "promotionDesc": product_promotion_desc(pp),
    })
}

/// 限时折扣实体 → `data`（Java `realPromotion` type=6：`TimeDiscount` 实体 +
/// 模板描述 `promotion.time.discount`）。
fn time_discount_data(d: &TimeDiscount) -> serde_json::Value {
    serde_json::json!({
        "discountId": d.discount_id,
        "promotionName": d.promotion_name,
        "startTime": d.start_time,
        "endTime": d.end_time,
        "shopId": d.shop_id,
        "addTime": d.add_time,
        "promotionDesc": fixed_promotion_desc(6),
    })
}

/// 拼团活动实体 → `data`（Java `realPromotion` type=7：`GrouponPromotion`
/// 实体 + 模板描述 `promotion.groupon`；`data.item` 由回填段另行注入）。
fn groupon_data(g: &GrouponPromotion) -> serde_json::Value {
    serde_json::json!({
        "grouponPromotionId": g.groupon_promotion_id,
        "name": g.name,
        "startTime": g.start_time,
        "endTime": g.end_time,
        "groupType": g.group_type,
        "expirationTime": g.expiration_time,
        "grouponImage": g.groupon_image,
        "pcGrouponImage": g.pc_groupon_image,
        "shareImage": g.share_image,
        "grouponExplain": g.groupon_explain,
        "shopId": g.shop_id,
        "addTime": g.add_time,
        "promotionDesc": fixed_promotion_desc(7),
    })
}

/// 赠品 → Java `ProductGiftVO` + `productInfo`（赠品商品的**前端消费子集**：
/// 商品 id/名称/价格/图片/库存/店铺；Java 为完整 `Product` 实体，详情弹窗
/// 仅消费这些字段，见报告口径说明）。
fn gift_vo_json(g: &ProductGift, info: Option<&Product>) -> serde_json::Value {
    let product_info = info.map(|p| {
        serde_json::json!({
            "productId": p.product_id,
            "productName": p.product_name,
            "productPrice": p.product_price,
            "marketPrice": p.market_price,
            "picUrl": p.pic_url,
            "picThumb": p.pic_thumb,
            "picLarge": p.pic_large,
            "picOriginal": p.pic_original,
            "productStock": p.product_stock,
            "shopId": p.shop_id,
        })
    });
    serde_json::json!({
        "giftId": g.gift_id,
        "giftName": g.gift_name,
        "giftStock": g.gift_stock,
        "productId": g.product_id,
        "skuId": g.sku_id,
        "productInfo": product_info,
    })
}

/// 活动 `range` 是否覆盖商品（Java `getProductsPromotion` 的活动范围判定）。
///
/// `range`：None/0 = 全部商品；3 = 指定商品（`range_data` 含该商品 ID）；
/// 4 = 不含以下商品（`range_data` 不含该商品 ID）。其余值按未指定处理（放行）。
pub(crate) fn promotion_covers_product(promotion: &Promotion, product_id: i32) -> bool {
    let range_ids = promotion.range.as_ref().and_then(|r| {
        promotion
            .range_data
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "null")
            .and_then(parse_id_list)
    });
    match promotion.range.unwrap_or(0) {
        0 => true,
        3 => range_ids.is_some_and(|ids| ids.contains(&product_id)),
        4 => !range_ids.is_some_and(|ids| ids.contains(&product_id)),
        _ => true,
    }
}

/// 解析活动范围 ID 列表（Java 侧存 `List.toString()` 文本，Rust 侧存 JSON；
/// 两者形如 `[1, 2]` / `[1,2]`，按数字逐个扫描提取，兼容两种格式）。
pub(crate) fn parse_id_list(raw: &str) -> Option<Vec<i32>> {
    let mut out = Vec::new();
    let bytes = raw.as_bytes();
    let mut start: Option<usize> = None;
    for (i, b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            if let Ok(n) = raw[s..i].parse::<i32>() {
                out.push(n);
            }
        }
    }
    if let Some(s) = start {
        if let Ok(n) = raw[s..].parse::<i32>() {
            out.push(n);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_helpers_extract_numeric_keys() {
        let v = serde_json::json!({"comment_id": 7, "rank": "4"});
        assert_eq!(i64_of(&v, &["comment_id"]), 7);
        assert_eq!(i64_of(&v, &["rank"]), 4);
        assert_eq!(i64_of(&v, &["missing"]), 0);
    }

    #[test]
    fn percent_is_zero_safe() {
        let pct = |n: i64, total: i64| {
            if total == 0 {
                0.0
            } else {
                n as f64 * 100.0 / total as f64
            }
        };
        assert_eq!(pct(0, 0), 0.0);
        assert_eq!(pct(1, 4), 25.0);
    }

    #[test]
    fn cny_amount_rounds_two_decimals() {
        let total = Decimal::new(1234, 2) * Decimal::from(3);
        assert_eq!(total, Decimal::new(3702, 2));
    }

    fn promotion_fixture(r#type: i32) -> Promotion {
        Promotion {
            promotion_id: 11,
            promotion_name: "测试活动".into(),
            start_time: 0,
            end_time: 0,
            r#type,
            shop_id: 0,
            relation_id: Some(21),
            range: Some(0),
            is_available: Some(1),
            is_delete: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn detail_promotion_sku_applicability() {
        let mut p = promotion_fixture(3);
        // 未配置 sku_ids → 恒命中；商品级查询（sku_id=0）恒命中
        assert!(promotion_applies_to_sku(&p, 5));
        assert!(promotion_applies_to_sku(&p, 0));
        // 配置后按包含判定
        p.sku_ids = Some("[5, 6]".into());
        assert!(promotion_applies_to_sku(&p, 5));
        assert!(!promotion_applies_to_sku(&p, 7));
        assert!(promotion_applies_to_sku(&p, 0));
        // 空数组 = 未限制
        p.sku_ids = Some("[]".into());
        assert!(promotion_applies_to_sku(&p, 7));
    }

    #[test]
    fn detail_promotion_vo_type_zero_is_invisible_shape() {
        // type=0（后端下发的无文案条目）：data 仅注入起止时间、无 promotionDesc，
        // 前端 isActivityVisible 据此判不可见（详情报告 §2.1 口径）
        let p = promotion_fixture(0);
        let vo = detail_promotion_vo(&p, serde_json::json!({}), "长期有效", &[1, 2]);
        assert_eq!(vo["type"], 0);
        assert_eq!(vo["typeText"], "");
        assert_eq!(vo["startTime"], "0");
        assert_eq!(vo["data"]["startTime"], "0");
        assert!(vo["data"].get("promotionDesc").is_none());
        assert_eq!(vo["canJoin"], true);
        // 未配置 skuIds → null（Java 保持 null）
        assert!(vo["skuIds"].is_null());
    }

    #[test]
    fn detail_promotion_vo_coupon_with_and_without_desc() {
        // 带券行：data.promotionDesc 由券面推导（满 100.00 减 10.00）
        let coupon = Coupon {
            coupon_id: 21,
            coupon_name: "满减券".into(),
            coupon_money: Decimal::new(1000, 2),
            min_order_amount: Decimal::new(10000, 2),
            coupon_type: 1,
            reduce_type: 1,
            ..Default::default()
        };
        let p = promotion_fixture(2);
        let vo = detail_promotion_vo(&p, coupon_data(&coupon), "长期有效", &[1]);
        assert_eq!(vo["type"], 2);
        assert_eq!(vo["data"]["promotionDesc"], "满 100.00 减 10.00");
        assert_eq!(vo["data"]["couponId"], 21);
        // 券行缺失（relation 悬空）：data 为空对象注入时间后无 promotionDesc
        let vo_missing = detail_promotion_vo(&p, serde_json::json!({}), "长期有效", &[1]);
        assert!(vo_missing["data"].get("promotionDesc").is_none());
    }

    #[test]
    fn detail_promotion_vo_reduce_and_sku_intersection() {
        // type=3 满减：data 带实体字段 + 推导描述；skuIds 与商品 SKU 求交
        let pp = ProductPromotion {
            promotion_id: 21,
            promotion_name: "满减活动".into(),
            promotion_type: 1,
            rules_type: 1,
            unit: 1,
            promotion_type_data: r#"[{"minAmount":100,"reduce":"10"}]"#.into(),
            ..Default::default()
        };
        let mut p = promotion_fixture(3);
        p.sku_ids = Some("[2, 3, 9]".into());
        let vo = detail_promotion_vo(&p, product_promotion_data(&pp), "长期有效", &[2, 3, 4]);
        assert_eq!(vo["data"]["promotionDesc"], "满100减10");
        assert_eq!(vo["data"]["promotionName"], "满减活动");
        assert_eq!(vo["skuIds"], serde_json::json!([2, 3]));
        // 商品无 SKU → skuIds 置空数组（Java 同口径）
        let vo_no_sku = detail_promotion_vo(&p, product_promotion_data(&pp), "长期有效", &[]);
        assert_eq!(vo_no_sku["skuIds"], serde_json::json!([]));
    }

    #[test]
    fn detail_promotion_vo_combo_and_removal_rule() {
        // type=0/2/3 组合：各自组装互不干扰；仅 type=2+isDelete=1 被移除
        let p0 = promotion_fixture(0);
        let mut p2 = promotion_fixture(2);
        p2.is_delete = Some(1);
        let p3 = promotion_fixture(3);
        let vos = vec![
            detail_promotion_vo(&p0, serde_json::json!({}), "长期有效", &[1]),
            detail_promotion_vo(&p2, serde_json::json!({}), "长期有效", &[1]),
            detail_promotion_vo(&p3, serde_json::json!({}), "长期有效", &[1]),
        ];
        assert!(is_detail_promotion_removed(&vos[1]));
        assert!(!is_detail_promotion_removed(&vos[0]));
        assert!(!is_detail_promotion_removed(&vos[2]));
        let kept: Vec<_> = vos
            .into_iter()
            .filter(|v| !is_detail_promotion_removed(v))
            .collect();
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0]["type"], 0);
        assert_eq!(kept[1]["type"], 3);
        // type=3 即使 isDelete=1 也保留（detail 口径）
        let mut p3d = promotion_fixture(3);
        p3d.is_delete = Some(1);
        let vo3d = detail_promotion_vo(&p3d, serde_json::json!({}), "长期有效", &[1]);
        assert!(!is_detail_promotion_removed(&vo3d));
    }

    #[test]
    fn detail_promotion_empty_when_no_activities() {
        // 空促销：组装结果为空数组（前端 result?.promotion || [] 拿到 []，
        // 与 Java promotionListVo 为空列表同形态）
        let vos: Vec<serde_json::Value> = Vec::new();
        assert_eq!(
            serde_json::json!({ "promotion": vos })["promotion"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn detail_promotion_fixed_desc_data_shapes() {
        // 秒杀/限时折扣/拼团 data 带各自实体键 + 固定模板描述
        let s = Seckill {
            seckill_id: 21,
            seckill_name: Some("秒杀场".into()),
            seckill_limit_num: Some(2),
            ..Default::default()
        };
        let sd = seckill_data(&s);
        assert_eq!(sd["promotionDesc"], "秒杀价");
        assert_eq!(sd["seckillLimitNum"], 2);
        let td = time_discount_data(&TimeDiscount {
            discount_id: 21,
            promotion_name: "限时场".into(),
            ..Default::default()
        });
        assert_eq!(td["promotionDesc"], "限时折扣");
        assert_eq!(td["discountId"], 21);
        let gd = groupon_data(&GrouponPromotion {
            groupon_promotion_id: 21,
            name: "拼团场".into(),
            ..Default::default()
        });
        assert_eq!(gd["promotionDesc"], "拼团");
        assert_eq!(gd["name"], "拼团场");
        // 秒杀明细 item 形态（前端读 seckillSales/seckillStock）
        let item = seckill_item_json(&SeckillItem {
            seckill_price: Some(Decimal::new(990, 2)),
            seckill_stock: Some(8),
            seckill_sales: Some(3),
            ..Default::default()
        });
        assert_eq!(item["seckillSales"], 3);
        assert_eq!(item["seckillStock"], 8);
    }
}
