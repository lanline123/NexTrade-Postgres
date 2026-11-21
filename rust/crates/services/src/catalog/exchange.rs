//! 积分兑换域服务（catalog::exchange）。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.product.ExchangeController`（3 个 `optional`
//!   公开端点）
//! - `com.nextrade.service.promotion.impl.PointsExchangeServiceImpl#list` /
//!   `#detail`
//! - `com.nextrade.service.product.impl.ProductSkuServiceImpl#checkSkuEnabled`
//!   / `#getProductStock`
//! - `com.nextrade.service.cart.impl.CartServiceImpl#clearCart` /
//!   `#addProductToCart`（兑换场景所需子集）
//! - `com.nextrade.service.user.impl.UserServiceImpl#detail`（积分校验）
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/api`） | 原 Java | 方法 |
//! |-----------------------|---------|------|
//! | `GET  /product/exchange/detail` | `ExchangeController.detail` | [`ExchangeService::detail`] |
//! | `GET  /product/exchange/list` | `ExchangeController.list` | [`ExchangeService::list`] |
//! | `POST /product/exchange/addToCart` | `ExchangeController.addExchangeProductToCart` | [`ExchangeService::add_to_cart`] |
//! | `POST /product/exchange/exchangeCoupon` | **我方新增**（积分兑换优惠券） | [`ExchangeService::exchange_coupon`] |
//!
//! # 依赖复用（不重复实现既有域逻辑）
//!
//! - 商品前台详情：`crate::catalog::product_manage::ProductManageService::
//!   client_detail`（已迁移，含 SKU / 属性 / 图集 / 服务）；
//! - 商品与 SKU 读：`Repositories::catalog_product_manage`
//!   （`find_product` / `find_sku` / `find_skus`）；
//! - 会员积分：`Repositories::member_user.find_by_id`；
//! - 购物车写入：`Repositories::trade_cart.insert`（既有验证过的 `cart` 插入）；
//!   兑换类清空走本域仓储 [`ExchangeRepository::clear_exchange_cart`]。
//!
//! # 与 Java 的差异（`uncertain_points`）
//!
//! 1. **`addToCart` 的购物车写入是「兑换场景子集」**：Java 调
//!    `CartServiceImpl#addProductToCart`（含实名认证、店铺营业状态、付费
//!    商品已购校验、抽奖记录、促销/秒杀价、快速购买行清理等 100+ 行）；
//!    本实现只做兑换相关校验 + 「清空 `type=3` 行 + 插入一条 `type=3`
//!    兑换行」。其余通用加购校验未迁移（见
//!    `nextrade_core::models::catalog::exchange` 模块头差异第 1 条）。
//! 2. **库存口径**：只取 `product.product_stock` / `product_sku.sku_stock`；
//!    Java 对周期购返回上限、对卡密商品按卡密库存计数，这两类特殊商品
//!    未迁移（差异第 2 条）。
//! 3. **活动不存在**：Java `detail` 路径会 NPE（500），`addToCart` 抛
//!    `活动不存在`；本实现统一返回业务错误 `活动不存在`。
//! 4. **详情返回结构**：返回既有 `ClientProductDetailVO`（非 Java 的
//!    `ProductDetailVO` 全量字段），`exchangeInfo` 承载本域
//!    [`PointsExchangeVO`]；未迁移域字段（评论/秒杀/拼团/批发）保持
//!    `client_detail` 的既有空值。
//! 5. **游客**：Java `SecurityUtils.getCurrentUserId()` 游客返回 `0`，
//!    本实现由路由层同样传 `0`；积分校验按 0 处理（几乎必触发
//!    `您的积分不足`）。
//!
//! # 金额
//!
//! `points_deducted_amount` / 商品价 / SKU 价均为 `rust_decimal::Decimal`。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::business::{NEW_USER_MEMBER, NEW_USER_ONLY};
use nextrade_core::entities::{Cart, PointsExchange, Product, ProductSku, UserCoupon};
use nextrade_core::models::catalog::exchange::{
    discounts_price, effective_product_price, filter_spe, normalize_sku_id, retain_exchange_sku,
    validate_add_to_cart, AddToCartDTO, ExchangeCouponDTO, ExchangeCouponVO,
    ExchangeProductToCartVO, PointsExchangeListQuery, PointsExchangeVO, CART_TYPE_EXCHANGE,
    DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MSG_ACTIVITY_NOT_EXIST, MSG_ACTIVITY_OFF_SHELF,
    MSG_COUPON_LIMIT_REACHED, MSG_COUPON_NEW_USER_ONLY, MSG_COUPON_NOT_EXIST,
    MSG_COUPON_NOT_IN_TIME, MSG_COUPON_PRICE_NOT_SET, MSG_COUPON_RANK_LIMITED,
    MSG_COUPON_SOLD_OUT, MSG_COUPON_USE_EXCHANGE_API, MSG_EXCHANGE_PRODUCT_NOT_FOUND, MSG_ID_REQUIRED,
    MSG_POINTS_NOT_ENOUGH, MSG_PRODUCT_NOT_EXIST, MSG_SKU_DISABLED, MSG_STOCK_NOT_ENOUGH,
    POINTS_LOG_DESC_COUPON_EXCHANGE, SOURCE_POINTS_EXCHANGE,
};
use nextrade_core::models::marketing::points_exchange::{
    resolve_coupon_points, resolve_effective_coupon_scale, EXCHANGE_TYPE_COUPON,
    KEY_INTEGRAL_COUPON_SCALE, KEY_INTEGRAL_SCALE,
};
use nextrade_core::models::catalog::product_manage::ClientProductDetailVO;
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::exchange::{
    ExchangeFilters, ExchangeRepository, PointsExchangeRow,
};
use nextrade_infrastructure::db::Repositories;

use crate::catalog::product_manage::ProductManageService;

/// 商品类型缺省值：实物商品（`cart.product_type` 为 `NOT NULL`，
/// Java `Product#getProductType()` 为空时按实物处理）。
pub const DEFAULT_PRODUCT_TYPE_PHYSICAL: i32 = 1;

/// 积分兑换域服务（挂载于 `PublicState.catalog.exchange`）。
pub struct ExchangeService {
    repos: Arc<Repositories>,
}

impl ExchangeService {
    /// 构造（与同域其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &ExchangeRepository {
        &self.repos.catalog.exchange
    }

    /// 商品域服务（内部构造，仅借用 `Arc<Repositories>`，无额外状态）。
    fn product_manage(&self) -> ProductManageService {
        ProductManageService::new(self.repos.clone())
    }

    // ————————————————————— 列表 —————————————————————

    /// 积分兑换分页列表（对应 Java `PointsExchangeServiceImpl#list`）。
    ///
    /// 控制器层会强制 `isEnabled = 1`（见 `GET /api/product/exchange/list`
    /// 的 handler），服务层按传入值原样处理，以保持方法本身可复用。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list(
        &self,
        q: &PointsExchangeListQuery,
    ) -> Result<PageResult<PointsExchangeVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let filters = ExchangeFilters {
            keyword: q.keyword.as_deref(),
            is_hot: q.is_hot,
            is_enabled: q.is_enabled,
        };
        let (total, rows) = self
            .repo()
            .paginate(
                &filters,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;
        // 券型行（我方新增）的展示积分价 = 生效价（自设或平台统一比折算）。
        let coupon_scale = if rows.iter().any(|r| r.exchange_type == EXCHANGE_TYPE_COUPON) {
            self.coupon_scale().await?
        } else {
            Decimal::ONE
        };
        let list: Vec<PointsExchangeVO> = rows.iter().map(|row| row_to_vo(row, coupon_scale)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 生效的积分兑券统一比（integralCouponScale → integralScale → 1）。
    async fn coupon_scale(&self) -> Result<Decimal> {
        let values = self
            .repos
            .system
            .config_client
            .find_config_values(&[
                KEY_INTEGRAL_COUPON_SCALE.to_string(),
                KEY_INTEGRAL_SCALE.to_string(),
            ])
            .await?;
        let map: HashMap<String, String> = values
            .into_iter()
            .map(|c| (c.biz_code, c.biz_val.unwrap_or_default()))
            .collect();
        Ok(resolve_effective_coupon_scale(&map))
    }

    // ————————————————————— 详情 —————————————————————

    /// 积分兑换详情（对应 Java `ExchangeController#detail`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. 取活动 VO（含商品 / SKU 价）
    /// 2. `checkSkuEnabled(productId, skuId)`
    /// 3. `productService.clientDetail(productSn, shopId, nearestShopId)`
    /// 4. `skuList` 收敛到活动 SKU → `filterSpe`
    /// 5. `exchangeInfo = 活动 VO`
    ///
    /// # Errors
    /// - 活动不存在 → `Error::biz(500, 活动不存在)`
    /// - 关联商品不存在 → `Error::biz(500, 未找到兑换商品)`
    /// - SKU 未启用 → `Error::biz(500, 商品规格已禁用，请重新选择)`
    /// - 商品已下架 / 已删 → 由 `client_detail` 返回 `商品不存在` 业务错误
    /// - 数据库执行失败 → `Error::Database`
    pub async fn detail(
        &self,
        id: i32,
        shop_id: Option<i32>,
        nearest_shop_id: Option<i32>,
    ) -> Result<ClientProductDetailVO> {
        let points = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_NOT_EXIST))?;
        // 券型兑换品（我方新增）不走商品详情链路。
        if points.exchange_type == EXCHANGE_TYPE_COUPON {
            return Err(Error::biz(500, MSG_COUPON_USE_EXCHANGE_API));
        }
        // Java `productSkuService.getById(pointsExchange.getSkuId())`（skuId > 0 才查）
        let sku = self.load_sku(points.sku_id).await?;
        let product = self
            .repos.catalog.product_manage
            .find_product(points.product_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_EXCHANGE_PRODUCT_NOT_FOUND))?;
        let exchange_vo = build_vo(&points, &product, sku.as_ref());

        // Java：productSkuService.checkSkuEnabled(...)
        self.check_sku_enabled(points.product_id, points.sku_id).await?;

        // Java：productService.clientDetail(pointsExchange.getProductSn(), ...)
        // 积分兑换详情无买家已购语义，user_id 传 None（isBuy 恒 0、付费全文不下发）
        let product_sn = exchange_vo.product_sn.clone().unwrap_or_default();
        let mut detail = self
            .product_manage()
            .client_detail(&product_sn, shop_id, nearest_shop_id, None)
            .await?;

        // Java：skuList 收敛到活动 SKU，再 filterSpe（空列表则不处理属性）
        if !detail.sku_list.is_empty() {
            retain_exchange_sku(&mut detail.sku_list, points.sku_id);
            if !detail.sku_list.is_empty() {
                filter_spe(&detail.sku_list, &mut detail.attr_list);
            }
        }

        // Java：productDetailVO.setExchangeInfo(pointsExchangeService.detail(id))
        detail.exchange_info = Some(
            serde_json::to_value(&exchange_vo)
                .map_err(|e| Error::Internal(format!("序列化积分兑换信息失败: {e}")))?,
        );
        Ok(detail)
    }

    // ————————————————————— 加入兑换购物车 —————————————————————

    /// 添加积分商品到购物车（对应 Java
    /// `ExchangeController#addExchangeProductToCart`）。
    ///
    /// 校验顺序与 Java 一致：活动存在 → 活动生效 → SKU 可用 → 商品存在
    /// → 商品上架未删 → 库存 → 会员积分；全部通过后才清空并写入兑换购物车。
    ///
    /// `user_id` 为游客时传 `0`（Java `SecurityUtils.getCurrentUserId()`）。
    ///
    /// # Errors
    /// - 入参校验失败 → `Error::Validation(<Java 注解文案>)`
    /// - 业务校验失败 → `Error::biz(500, <Java 文案>)`（活动不存在 / 已下架 /
    ///   未找到兑换商品 / 商品不存在 / 库存不足 / 您的积分不足 /
    ///   商品规格已禁用，请重新选择）
    /// - 数据库执行失败 → `Error::Database`
    pub async fn add_to_cart(
        &self,
        dto: &AddToCartDTO,
        user_id: i32,
    ) -> Result<ExchangeProductToCartVO> {
        let (id, number) = validate_add_to_cart(dto).map_err(Error::Validation)?;

        // Java：pointsExchangeService.detail(id) → null → 活动不存在
        let points = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_NOT_EXIST))?;
        // 券型兑换品（我方新增）不进购物车链路，走即时兑换端点。
        if points.exchange_type == EXCHANGE_TYPE_COUPON {
            return Err(Error::biz(500, MSG_COUPON_USE_EXCHANGE_API));
        }
        // Java：pointsExchange.getIsEnabled() == 0 → 活动已下架
        if points.is_enabled == 0 {
            return Err(Error::biz(500, MSG_ACTIVITY_OFF_SHELF));
        }

        // Java：productSkuService.checkSkuEnabled(productId, pointsExchange.getSkuId())
        self.check_sku_enabled(points.product_id, points.sku_id).await?;

        // Java：productService.detail(productId) → null → 未找到兑换商品
        let product = self
            .repos.catalog.product_manage
            .find_product(points.product_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_EXCHANGE_PRODUCT_NOT_FOUND))?;
        // Java：productStatus == 0 || isDelete == 1 → 商品不存在
        if product.product_status == 0 || product.is_delete == 1 {
            return Err(Error::biz(500, MSG_PRODUCT_NOT_EXIST));
        }

        // 客户黑名单（下单限制）：积分兑换加购同口径拦截（提交时仍过订单复检）。
        crate::member::user_blacklist::UserBlacklistService::new(self.repos.clone())
            .ensure_allowed(
                user_id,
                nextrade_core::models::member::user_blacklist::PRODUCT_MALL,
                nextrade_core::models::member::user_blacklist::BEHAVIOR_PLACE_ORDER,
                Some(product.shop_id),
            )
            .await?;

        // Java：skuId = dto.getSkuId() != null ? dto.getSkuId() : 0
        let sku_id = normalize_sku_id(dto.sku_id);
        let stock = self.stock_of(&product, sku_id).await?;
        if stock < number {
            return Err(Error::biz(500, MSG_STOCK_NOT_ENOUGH));
        }

        // Java：userService.detail(userId).getPoints()；游客 userId=0
        let user_points = self
            .repos.member.user
            .find_by_id(user_id)
            .await?
            .map_or(0, |u| u.points);
        // Java：points < exchangeIntegral * number → 您的积分不足
        // （用 i64 计算，避免 Java int 乘法溢出差异）
        if i64::from(user_points) < i64::from(points.exchange_integral) * i64::from(number) {
            return Err(Error::biz(500, MSG_POINTS_NOT_ENOUGH));
        }

        // Java：cartService.clearCart(userId, TYPE_EXCHANGE)
        self.repo().clear_exchange_cart(user_id).await?;

        // Java：cartService.addProductToCart(userId, dto)（兑换子集，见模块头差异 1）
        let sku = if sku_id > 0 {
            self.repos.catalog.product_manage.find_sku(sku_id).await?
        } else {
            None
        };
        let original_price: Decimal = sku
            .as_ref()
            .and_then(|s| s.sku_price)
            .unwrap_or(product.product_price);
        let cart = Cart {
            user_id,
            product_id: product.product_id,
            product_sn: product.product_sn.clone(),
            pic_thumb: Some(product.pic_thumb.clone()),
            market_price: Some(product.market_price),
            original_price,
            quantity: number,
            sku_id: Some(sku_id),
            sku_data: sku.as_ref().and_then(|s| s.sku_data.clone()),
            product_type: product.product_type.unwrap_or(DEFAULT_PRODUCT_TYPE_PHYSICAL),
            is_checked: 1,
            shop_id: product.shop_id,
            r#type: CART_TYPE_EXCHANGE,
            update_time: now_secs_i32(),
            ..Default::default()
        };
        let cart_id = self.repos.trade.cart.insert(&cart).await?;
        tracing::info!(
            "积分兑换加入购物车: user_id={user_id}, exchange_id={id}, product_id={}, sku_id={sku_id}, cart_id={cart_id}",
            product.product_id
        );
        Ok(ExchangeProductToCartVO {
            item: true,
            flow_type: CART_TYPE_EXCHANGE,
        })
    }

    // ————————————————— 积分兑换优惠券（我方新增） —————————————————

    /// 积分兑换优惠券（论坛帖 517907；原版无此端点）。
    ///
    /// 与商品型兑换不同，券型是**即时兑换**：不走购物车/订单链，在单个
    /// 数据库事务内完成「守卫扣积分 + 写积分流水 + 发券」，任一步失败
    /// 整体回滚（积分不足不发券、券已兑完不扣分）。
    ///
    /// 校验口径与会员领券（`marketing::coupon::CouponService::claim`）一致，
    /// 仅两点不同：① 兑换品自身的 `is_enabled` 是上架门（不看模板的
    /// `is_show` / `enabled_click_get` 领券开关——积分商城是独立陈列面）；
    /// ② 每人限兑/发放总量在事务内锁定券模板行后计数，防并发超兑。
    /// 积分价 = 单品自设优先，未自设按平台统一比折算（见 core 纯函数）。
    ///
    /// # Errors
    /// - 未登录 → `Error::Unauthorized`
    /// - 兑换品不存在/已下架/非券型、券模板异常、资格不符、积分不足 →
    ///   `Error::biz(500, <文案>)`
    pub async fn exchange_coupon(
        &self,
        dto: &ExchangeCouponDTO,
        user_id: i32,
    ) -> Result<ExchangeCouponVO> {
        if user_id <= 0 {
            return Err(Error::Unauthorized("用户未登录".into()));
        }
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.to_string()))?;
        let item = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_NOT_EXIST))?;
        if item.exchange_type != EXCHANGE_TYPE_COUPON {
            return Err(Error::biz(500, MSG_COUPON_USE_EXCHANGE_API));
        }
        if item.is_enabled == 0 {
            return Err(Error::biz(500, MSG_ACTIVITY_OFF_SHELF));
        }
        let coupon_id = item
            .coupon_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::biz(500, MSG_COUPON_NOT_EXIST))?;

        // 生效积分价（自设优先 / 平台统一比折算）在事务内按锁定行计算。
        let scale = self.coupon_scale().await?;
        let now = nextrade_common::utils::now_seconds();
        let coupon_repo = &self.repos.marketing.coupon;
        let mut tx = self.repo().begin().await?;
        // 锁定券模板行：同券并发兑换串行化，总量/限兑计数在锁内进行。
        let coupon = coupon_repo
            .find_by_id_for_update(&mut tx, coupon_id)
            .await?
            .filter(|c| !c.is_deleted())
            .ok_or_else(|| Error::biz(500, MSG_COUPON_NOT_EXIST))?;
        let (points_price, source) =
            resolve_coupon_points(item.exchange_integral, coupon.coupon_money, scale);
        if points_price <= 0 {
            return Err(Error::biz(500, MSG_COUPON_PRICE_NOT_SET));
        }

        // 1) 每人限兑（limit_num，含历史领取/兑换计数，与领券同口径）
        let user_count = coupon_repo
            .count_user_claimed_in_tx(&mut tx, user_id, coupon_id)
            .await?;
        let limit_per_user = i64::from(coupon.claim_limit_per_user());
        if limit_per_user > 0 && user_count >= limit_per_user {
            return Err(Error::biz(500, MSG_COUPON_LIMIT_REACHED));
        }
        // 2) 发放总量（send_num，与领券共享同一库存池）
        let issued = coupon_repo.count_issued_in_tx(&mut tx, coupon_id).await?;
        let total_supply = i64::from(coupon.total_supply());
        if total_supply > 0 && issued >= total_supply {
            return Err(Error::biz(500, MSG_COUPON_SOLD_OUT));
        }
        // 3) 新人专享（口径同 claim：已持有过该券或已有已付订单均非新人）
        if coupon.is_new_user == NEW_USER_ONLY {
            if user_count > 0 {
                return Err(Error::biz(500, MSG_COUPON_LIMIT_REACHED));
            }
            let paid_orders = self.repos.trade.order.count_paid_by_user(user_id).await?;
            if paid_orders > 0 {
                return Err(Error::biz(500, MSG_COUPON_NEW_USER_ONLY));
            }
        }
        // 4) 会员等级专享
        if coupon.is_new_user == NEW_USER_MEMBER {
            let rank_ids = coupon.limited_rank_ids();
            if !rank_ids.is_empty() {
                let user_rank = self
                    .repos
                    .member
                    .user
                    .find_by_id(user_id)
                    .await?
                    .map(|u| u.rank_id);
                let ok = user_rank.is_some_and(|r| rank_ids.contains(&r));
                if !ok {
                    return Err(Error::biz(500, MSG_COUPON_RANK_LIMITED));
                }
            }
        }
        // 5) 用券时间窗（口径同 claim 的 time_range_at）
        let (start_date, end_date) = coupon.time_range_at(now);
        if end_date <= now {
            return Err(Error::biz(500, MSG_COUPON_NOT_IN_TIME));
        }

        // 6) 守卫扣积分（余额不足 → 0 行 → 回滚报错，券不发）
        let deducted = self
            .repo()
            .deduct_points_in_tx(
                &mut tx,
                user_id,
                points_price,
                POINTS_LOG_DESC_COUPON_EXCHANGE,
                item.id,
                i32::try_from(now).unwrap_or(i32::MAX),
            )
            .await?;
        if !deducted {
            return Err(Error::biz(500, MSG_POINTS_NOT_ENOUGH));
        }

        // 7) 发券（source_type = 积分兑换，与领券/手动发放区分）
        let uc = UserCoupon {
            id: 0,
            coupon_id,
            coupon_sn: nextrade_common::utils::gen_sn("UC"),
            user_id,
            used_time: 0,
            order_id: 0,
            start_date: i32::try_from(start_date).unwrap_or(i32::MAX),
            end_date: i32::try_from(end_date).unwrap_or(i32::MAX),
            source_type: Some(SOURCE_POINTS_EXCHANGE),
        };
        let user_coupon_id = coupon_repo.insert_user_coupon_in_tx(&mut tx, &uc).await?;
        let remaining = self.repo().user_points_in_tx(&mut tx, user_id).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!(
            "积分兑换优惠券: user_id={user_id}, exchange_id={id}, coupon_id={coupon_id}, points={points_price}, source={source}, user_coupon_id={user_coupon_id}"
        );
        Ok(ExchangeCouponVO {
            user_coupon_id,
            coupon_id,
            points_spent: points_price,
            remaining_points: remaining,
            integral_source: source.to_string(),
        })
    }

    // ————————————————————— 内部 —————————————————————

    /// 活动指定 SKU 的实体（Java `productSkuService.getById(skuId)`）。
    ///
    /// `sku_id` 为 `None` / `<= 0` 时 Java 不查 SKU，这里返回 `None`。
    async fn load_sku(&self, sku_id: Option<i32>) -> Result<Option<ProductSku>> {
        let Some(sku_id) = sku_id.filter(|s| *s > 0) else {
            return Ok(None);
        };
        self.repos.catalog.product_manage.find_sku(sku_id).await
    }

    /// SKU 可用校验（对应 Java `ProductSkuServiceImpl#checkSkuEnabled`）：
    /// `sku_id` 为空 / `<= 0` 直接通过；否则要求存在
    /// `(sku_id, product_id, is_enabled = 1)` 的 SKU。
    async fn check_sku_enabled(&self, product_id: i32, sku_id: Option<i32>) -> Result<()> {
        let Some(sku_id) = sku_id.filter(|s| *s > 0) else {
            return Ok(());
        };
        let skus = self
            .repos.catalog.product_manage
            .find_skus(product_id, false)
            .await?;
        if skus.iter().any(|s| s.sku_id == sku_id) {
            Ok(())
        } else {
            Err(Error::biz(500, MSG_SKU_DISABLED))
        }
    }

    /// 商品库存（对应 Java `ProductSkuServiceImpl#getProductStock` 的普通商品
    /// 口径，见模块头差异第 2 条）：
    /// - `sku_id == 0` → `product.product_stock`
    /// - 否则取该 SKU 的 `sku_stock`（不存在 / 未启用按 0）
    async fn stock_of(&self, product: &Product, sku_id: i32) -> Result<i32> {
        if sku_id == 0 {
            return Ok(product.product_stock);
        }
        let skus = self
            .repos.catalog.product_manage
            .find_skus(product.product_id, false)
            .await?;
        Ok(skus
            .iter()
            .find(|s| s.sku_id == sku_id)
            .and_then(|s| s.sku_stock)
            .unwrap_or(0))
    }
}

/// 连表行 → `PointsExchangeVO`（Java `PointsExchangeServiceImpl#list` 的回填逻辑）。
///
/// 券型行（我方新增）：无商品字段，回填券模板字段，`exchangeIntegral`
/// 输出**生效价**（自设或按 `coupon_scale` 平台统一比折算），
/// `integralSource` 标明按哪一级比定价。
#[must_use]
fn row_to_vo(row: &PointsExchangeRow, coupon_scale: Decimal) -> PointsExchangeVO {
    if row.exchange_type == EXCHANGE_TYPE_COUPON {
        let (points, source) = resolve_coupon_points(
            row.exchange_integral,
            row.coupon_money.unwrap_or(Decimal::ZERO),
            coupon_scale,
        );
        return PointsExchangeVO {
            id: row.id,
            product_id: row.product_id,
            exchange_integral: points,
            points_deducted_amount: row.points_deducted_amount,
            is_hot: row.is_hot,
            is_enabled: row.is_enabled,
            sku_id: row.sku_id,
            product_name: None,
            pic_thumb: None,
            pic_url: None,
            market_price: None,
            cost_price: None,
            product_price: None,
            discounts_price: None,
            product_stock: None,
            virtual_sales: None,
            product_sn: None,
            exchange_type: row.exchange_type,
            coupon_id: row.coupon_id,
            coupon_name: row.coupon_name.clone(),
            coupon_money: row.coupon_money,
            coupon_discount: row.coupon_discount,
            min_order_amount: row.min_order_amount,
            coupon_shop_id: row.coupon_shop_id,
            integral_source: Some(source.to_string()),
        };
    }
    // Java：productSkuMap 命中且 skuId > 0 才用 SKU 价覆盖
    let sku_found = row.sku_id.is_some_and(|id| id > 0) && row.sku_row_id.is_some();
    let price = effective_product_price(row.product_price, sku_found, row.sku_price);
    PointsExchangeVO {
        id: row.id,
        product_id: row.product_id,
        exchange_integral: row.exchange_integral,
        points_deducted_amount: row.points_deducted_amount,
        is_hot: row.is_hot,
        is_enabled: row.is_enabled,
        sku_id: row.sku_id,
        product_name: row.product_name.clone(),
        pic_thumb: row.pic_thumb.clone(),
        pic_url: row.pic_url.clone(),
        market_price: row.market_price,
        cost_price: None,
        product_price: price,
        discounts_price: Some(discounts_price(price, row.points_deducted_amount)),
        product_stock: row.product_stock,
        virtual_sales: row.virtual_sales,
        product_sn: row.product_sn.clone(),
        exchange_type: row.exchange_type,
        coupon_id: row.coupon_id,
        coupon_name: None,
        coupon_money: None,
        coupon_discount: None,
        min_order_amount: None,
        coupon_shop_id: None,
        integral_source: None,
    }
}

/// 活动 + 商品 + （可选）SKU → `PointsExchangeVO`
/// （Java `PointsExchangeServiceImpl#detail` 的回填逻辑）。
///
/// `sku` 为 `None` 表示：活动未指定 SKU（`sku_id` 为空 / `<= 0`），
/// 或该 SKU 行已不存在 —— 两种情况下 Java 都保留商品价。
#[must_use]
fn build_vo(
    points: &PointsExchange,
    product: &Product,
    sku: Option<&ProductSku>,
) -> PointsExchangeVO {
    let sku_found = points.sku_id.is_some_and(|id| id > 0) && sku.is_some();
    let sku_price = sku.and_then(|s| s.sku_price);
    let price = effective_product_price(Some(product.product_price), sku_found, sku_price);
    PointsExchangeVO {
        id: points.id,
        product_id: points.product_id,
        exchange_integral: points.exchange_integral,
        points_deducted_amount: points.points_deducted_amount,
        is_hot: points.is_hot,
        is_enabled: points.is_enabled,
        sku_id: points.sku_id,
        product_name: Some(product.product_name.clone()),
        pic_thumb: Some(product.pic_thumb.clone()),
        pic_url: Some(product.pic_url.clone()),
        market_price: Some(product.market_price),
        cost_price: None,
        product_price: price,
        discounts_price: Some(discounts_price(price, points.points_deducted_amount)),
        product_stock: Some(product.product_stock),
        virtual_sales: Some(product.virtual_sales),
        product_sn: Some(product.product_sn.clone()),
        exchange_type: points.exchange_type,
        coupon_id: points.coupon_id,
        coupon_name: None,
        coupon_money: None,
        coupon_discount: None,
        min_order_amount: None,
        coupon_shop_id: None,
        integral_source: None,
    }
}

/// 当前 Unix 秒（`cart.update_time` 为 `int unsigned` 秒级时间戳）。
fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::exchange::MSG_ACTIVITY_NOT_EXIST;

    fn product() -> Product {
        Product {
            product_id: 88,
            product_name: "蓝牙耳机".into(),
            product_sn: "SN-1".into(),
            product_stock: 10,
            product_price: "99.00".parse().unwrap(),
            market_price: "199.00".parse().unwrap(),
            product_status: 1,
            product_type: Some(1),
            shop_id: 0,
            pic_url: "https://oss.example/p.png".into(),
            pic_thumb: "https://oss.example/t.png".into(),
            virtual_sales: 5,
            ..Default::default()
        }
    }

    fn points(sku_id: Option<i32>) -> PointsExchange {
        PointsExchange {
            id: 3,
            product_id: 88,
            exchange_integral: 500,
            points_deducted_amount: "20.00".parse().unwrap(),
            is_hot: 1,
            is_enabled: 1,
            sku_id,
            exchange_type: 1,
            coupon_id: None,
        }
    }

    fn row(sku_id: Option<i32>, sku_row_id: Option<i32>, sku_price: Option<Decimal>) -> PointsExchangeRow {
        PointsExchangeRow {
            id: 3,
            product_id: 88,
            exchange_integral: 500,
            points_deducted_amount: "20.00".parse().unwrap(),
            is_hot: 1,
            is_enabled: 1,
            sku_id,
            exchange_type: 1,
            coupon_id: None,
            product_name: Some("蓝牙耳机".into()),
            pic_thumb: Some("https://oss.example/t.png".into()),
            pic_url: Some("https://oss.example/p.png".into()),
            market_price: Some("199.00".parse().unwrap()),
            product_price: Some("99.00".parse().unwrap()),
            product_stock: Some(10),
            virtual_sales: Some(5),
            product_sn: Some("SN-1".into()),
            sku_row_id,
            sku_price,
            coupon_name: None,
            coupon_money: None,
            coupon_discount: None,
            min_order_amount: None,
            coupon_shop_id: None,
        }
    }

    fn sku(id: i32, sku_price: Option<Decimal>) -> ProductSku {
        ProductSku {
            sku_id: id,
            product_id: 88,
            sku_price,
            ..Default::default()
        }
    }

    /// 列表行映射：SKU 价命中时覆盖商品价，折后价 = 有效价 - 抵扣金额。
    #[test]
    fn row_to_vo_applies_sku_price_and_discount() {
        let vo = row_to_vo(&row(Some(11), Some(11), Some("88.00".parse().unwrap())), Decimal::ONE);
        assert_eq!(vo.product_price, Some("88.00".parse().unwrap()));
        assert_eq!(vo.discounts_price, Some("68.00".parse().unwrap()));
        assert_eq!(vo.product_name.as_deref(), Some("蓝牙耳机"));
        assert_eq!(vo.product_sn.as_deref(), Some("SN-1"));
        assert!(vo.cost_price.is_none());

        // SKU 行不存在 → 保留商品价
        let vo2 = row_to_vo(&row(Some(11), None, None), Decimal::ONE);
        assert_eq!(vo2.product_price, Some("99.00".parse().unwrap()));
        assert_eq!(vo2.discounts_price, Some("79.00".parse().unwrap()));

        // sku_id = 0（SPU 维度）→ 商品价
        let vo3 = row_to_vo(&row(Some(0), None, None), Decimal::ONE);
        assert_eq!(vo3.product_price, Some("99.00".parse().unwrap()));
    }

    /// 券型行映射（我方新增）：生效价与来源标记；自设优先于平台比。
    #[test]
    fn row_to_vo_coupon_branch() {
        let mut r = row(None, None, None);
        r.exchange_type = 2;
        r.product_id = 0;
        r.coupon_id = Some(9);
        r.coupon_name = Some("满100减10".into());
        r.coupon_money = Some("10.00".parse().unwrap());
        r.min_order_amount = Some("100.00".parse().unwrap());
        r.exchange_integral = 0;
        // 未自设：面额 10 元、平台比 2 → 500 积分，来源 platform
        let vo = row_to_vo(&r, "2".parse().unwrap());
        assert_eq!(vo.exchange_type, 2);
        assert_eq!(vo.exchange_integral, 500);
        assert_eq!(vo.integral_source.as_deref(), Some("platform"));
        assert_eq!(vo.coupon_name.as_deref(), Some("满100减10"));
        assert!(vo.product_name.is_none());
        // 自设 300 → 原值，来源 self
        r.exchange_integral = 300;
        let vo2 = row_to_vo(&r, "2".parse().unwrap());
        assert_eq!(vo2.exchange_integral, 300);
        assert_eq!(vo2.integral_source.as_deref(), Some("self"));
    }

    /// 活动 VO 映射：商品平铺字段 + 折扣价；SKU 价优先。
    #[test]
    fn build_vo_maps_product_and_sku_price() {
        let p = product();
        let s = sku(11, Some("88.00".parse().unwrap()));
        let vo = build_vo(&points(Some(11)), &p, Some(&s));
        assert_eq!(vo.id, 3);
        assert_eq!(vo.product_id, 88);
        assert_eq!(vo.exchange_integral, 500);
        assert_eq!(vo.is_hot, 1);
        assert_eq!(vo.sku_id, Some(11));
        assert_eq!(vo.product_price, Some("88.00".parse().unwrap()));
        assert_eq!(vo.discounts_price, Some("68.00".parse().unwrap()));
        assert_eq!(vo.product_stock, Some(10));
        assert_eq!(vo.virtual_sales, Some(5));

        // SKU 价为 NULL → Java 覆盖成 null；折后价走 `productPrice != null` 的
        // else 分支 → `BigDecimal.ZERO`
        let null_price_sku = sku(11, None);
        let vo_null = build_vo(&points(Some(11)), &p, Some(&null_price_sku));
        assert_eq!(vo_null.product_price, None);
        assert_eq!(vo_null.discounts_price, Some(Decimal::ZERO));

        // 未指定 SKU → 商品价
        let vo2 = build_vo(&points(Some(0)), &p, None);
        assert_eq!(vo2.product_price, Some("99.00".parse().unwrap()));
        assert_eq!(vo2.discounts_price, Some("79.00".parse().unwrap()));
    }

    /// 活动不存在文案与 Java `addExchangeProductToCart` 一致。
    #[test]
    fn activity_messages_are_verbatim() {
        assert_eq!(MSG_ACTIVITY_NOT_EXIST, "活动不存在");
        assert_eq!(MSG_ACTIVITY_OFF_SHELF, "活动已下架");
        assert_eq!(MSG_SKU_DISABLED, "商品规格已禁用，请重新选择");
        assert_eq!(MSG_EXCHANGE_PRODUCT_NOT_FOUND, "未找到兑换商品");
        assert_eq!(MSG_PRODUCT_NOT_EXIST, "商品不存在");
        assert_eq!(MSG_STOCK_NOT_ENOUGH, "库存不足");
        assert_eq!(MSG_POINTS_NOT_ENOUGH, "您的积分不足");
    }

    /// 兑换购物车类型 / 商品类型缺省值。
    #[test]
    fn cart_type_and_product_type_defaults() {
        assert_eq!(CART_TYPE_EXCHANGE, 3);
        assert_eq!(DEFAULT_PRODUCT_TYPE_PHYSICAL, 1);
    }
}
