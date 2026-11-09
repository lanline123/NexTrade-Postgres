//! C 端批发服务（wholesale::wholesale_client）。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.wholesale.WholesaleClientController`（3 个 C 端端点）
//! - `com.nextrade.service.wholesale.impl.WholesaleSettingServiceImpl#isEnabled`
//! - `com.nextrade.service.wholesale.impl.WholesalerServiceImpl#isWholesaleBuyer / getByUserId`
//! - `com.nextrade.service.wholesale.WholesalerGradeService#getById`
//! - `com.nextrade.service.wholesale.impl.WholesaleProductQueryServiceImpl#buildBuyRule`
//! - `com.nextrade.service.wholesale.WholesaleBuyRuleValidator`（起批量 / 倍数纯函数）
//! - `com.nextrade.service.wholesale.resolver.WholesalePriceResolver#resolve`（批价命中纯函数）
//! - `com.nextrade.service.wholesale.impl.WholesaleCartBatchServiceImpl#addBatch`
//! - `com.nextrade.service.wholesale.CartWholesaleEnhancer#markAndValidateOnAdd`（批发侧校验）
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`WholesaleClientService::me`] | `WholesaleClientController#me` |
//! | [`WholesaleClientService::buy_rule`] | `WholesaleProductQueryServiceImpl#buildBuyRule` |
//! | [`WholesaleClientService::add_batch`] | `WholesaleCartBatchServiceImpl#addBatch` |
//! | [`resolve_wholesale_price`] | `WholesalePriceResolver#resolve` |
//! | [`check_moq_spu`] / [`check_moq_sku`] / [`check_multiple`] / [`align_up_to_multiple`] | `WholesaleBuyRuleValidator` |
//!
//! ## 与 Java 的行为差异（逐条，均已在 `uncertain_points` 中列明）
//!
//! 1. **Redis 身份缓存未迁移**（Java `WholesalerServiceImpl` 用
//!    `wholesale:isBuyer:{userId}` TTL 600s）：本实现每次都读库。功能等价，
//!    仅性能差异；`invalidateAllBuyerCache` 类逻辑一并消失。
//! 2. **`CartServiceImpl#addProductToCart` 只复刻了批发相关链路**。原方法
//!    440 余行，还包含实名认证校验、限购、秒杀/拼团人群、赠品卡/兑换卡、
//!    快速购买临时行清理、预约商品日历库存、分销员、附加属性、店铺状态、
//!    SKU 状态、营销价装配等。本切片**只**复刻：
//!    商品存在/未删除、商品未下架、SKU 启用、数量 > 0、库存充足、
//!    批发命中标记（`is_wholesale_line` / `wholesale_grade_id`）、
//!    起批量 / 倍数硬校验（5320 等）、同款行合并落库。
//!    因此这些未迁移校验造成的**行级失败在 Rust 侧不会出现**。
//! 3. **购物车行零售价**取「有 SKU → `product_sku.sku_price`，无 SKU →
//!    `Product::current_price()`（促销价优先）」，而 Java 走
//!    `productService.getProductFinalPrice(...)`（含会员折扣 / 活动价）。
//!    批发价本身**不写入** `cart.original_price`（与 Java 一致：Java 只在
//!    `markAndValidateOnAdd` 里打标记，批发价由购物车列表 `applyLine` 实时解析）。
//! 4. **`is_o2o`** 由环境变量 `TIG_IS_O2O` 驱动（Java 读
//!    `nextradeProperties.isO2o()`；Rust 配置服务未迁移），与 merchant::shop_manage 同口径。
//! 5. **DTO 校验错误码**：Java `@Valid` 失败由
//!    `GlobalExceptionHandler#handleMethodArgumentNotValidException` 统一转成
//!    `SERVICE_DATA_ERROR(1001)`（**不是**控制器 javadoc 写的 400）。
//!    本实现按运行时真实行为取 1001。
//! 6. **SPU 维度起批量只统计「本项」数量**（含同款已存在行合并后的数量）。
//!    Java `WholesaleCartBatchServiceImpl#addBatch` 是**逐项**调用
//!    `cartService.addProductToCart`，每次调用的 `addCarts` 只有本项一行，
//!    故 `CartWholesaleEnhancer#markAndValidateOnAdd` 的 `spuTotalQty`
//!    永远等于本行数量——**不是整批合计**。本实现照抄该口径。

use std::cmp::Ordering;
use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_core::entities::{
    ProductSku, ProductWholesaleConfig, ProductWholesalePrice, ProductWholesaleSkuConfig,
};
use nextrade_core::models::wholesale::wholesale_client::{
    ValidatedBatchItem, WholesaleCartBatchAddDTO, WholesaleCartBatchFailedItem,
    WholesaleCartBatchItem, WholesaleCartBatchResultVO, WholesaleMeVO, WholesaleProductBuyRuleVO,
    WholesaleSkuRuleVO,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::wholesale::wholesale_client::{NewCartLine, CART_TYPE_NORMAL};
use nextrade_infrastructure::db::Repositories;

// ————————————————————— 业务码（Java `com.nextrade.common.constant.HttpStatus`） —————————————————————

/// 批发功能未启用（Java `WHOLESALE_DISABLED`）。
pub const WHOLESALE_DISABLED: i32 = 5300;
/// 商品未配置批发能力（Java `WHOLESALE_PRODUCT_NOT_FOUND`）。
///
/// 说明：C 端批量加车链路**不会**抛出此码（Java 侧只有批发价 / 批发配置的
/// 后台读写会抛），这里保留常量是为了让本切片的错误码表与
/// `HttpStatus` 5300-5399 段可对照。
pub const WHOLESALE_PRODUCT_NOT_FOUND: i32 = 5310;
/// 未达起批量（Java `WHOLESALE_MOQ_NOT_MET`）。
pub const WHOLESALE_MOQ_NOT_MET: i32 = 5320;
/// 通用业务失败（Java `SERVICE_DATA_ERROR`，也是 `GlobalException` 默认码）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

// ————————————————————— 枚举常量 —————————————————————

/// 起批维度：SPU（Java `WholesaleBuyRuleValidator.MOQ_DIMENSION_SPU`）。
const MOQ_DIMENSION_SPU: i32 = 1;
/// 起批维度：SKU（Java `WholesaleBuyRuleValidator.MOQ_DIMENSION_SKU`）。
const MOQ_DIMENSION_SKU: i32 = 2;
/// 多规格价格模式：商品级混批（Java `CartWholesaleEnhancer.MULTI_SPEC_MODE_MIX`）。
const MULTI_SPEC_PRICE_MODE_MIX: i32 = 1;
/// 批发价模式：折扣换算（Java `WholesalePriceResolver` 的 `price_mode=1`）。
const PRICE_MODE_DISCOUNT: i32 = 1;
/// 批发价模式：直接金额（`price_mode=2`）。
const PRICE_MODE_FIXED: i32 = 2;
/// 商品状态：已下架（Java `ProductStatus.OFF_SALE`）。
const PRODUCT_STATUS_OFF_SALE: i32 = 0;
/// 商品类型：普通（实物）商品（Java `PRODUCT_TYPE_NORMAL`）。
const PRODUCT_TYPE_NORMAL: i32 = 1;
/// 金额保留位数（Java `RoundingMode.HALF_UP, scale=2`）。
const MONEY_SCALE: u32 = 2;
/// 批量加车单项数上限（Java `WholesaleCartBatchAddDTO` `@Size(max = 50)`）。
pub const MAX_BATCH_ITEMS: usize = 50;

/// 读取 O2O 开关（对应 Java `nextradeProperties.isO2o()`；配置服务未迁移）。
///
/// `TIG_IS_O2O=1` 或 `=true`（大小写不敏感）视为开启，其余/未设置视为关闭。
#[must_use]
fn is_o2o_from_env() -> bool {
    std::env::var("TIG_IS_O2O")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

// ═════════════════════════ 纯函数：批发价命中（WholesalePriceResolver） ═════════════════════════

/// 批发价命中入参（Java `WholesalePriceResolver.ResolveInput`）。
#[derive(Debug, Clone)]
pub struct WholesaleResolveInput<'a> {
    /// 商品 ID
    pub product_id: i32,
    /// SKU ID（`0` = SPU 兜底）
    pub sku_id: i32,
    /// 批发商等级 ID（`<= 0` = 非批发商 → 恒不命中）
    pub grade_id: i32,
    /// 数量（`<= 0` → 不命中）
    pub quantity: i32,
    /// 阶梯匹配专用数量（`None` 时回退 `quantity`）
    pub tier_match_qty: Option<i32>,
    /// SKU 零售价（`price_mode=1` 折扣换算的基数）
    pub retail_price: Decimal,
    /// 候选批发价行（已按 `product_id` 预过滤；本函数按 `status` / `is_delete` 再兜底过滤）
    pub candidates: &'a [ProductWholesalePrice],
}

/// 批发价命中结果（Java `WholesalePriceResolver.ResolveResult`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WholesaleResolveResult {
    /// 命中行 ID
    pub price_id: i32,
    /// 最终单价
    pub final_price: Decimal,
    /// `1` = 折扣 / `2` = 固定
    pub price_mode: i32,
    /// 命中行的 `discount`
    pub discount: Decimal,
    /// 命中的阶梯起量
    pub tier_min_qty: i32,
    /// 命中等级（`0` = 通用价）
    pub hit_grade_id: i32,
    /// 命中 SKU（`0` = SPU 兜底）
    pub hit_sku_id: i32,
}

/// 该行是否配了可用价格（Java `WholesalePriceResolver#hasUsablePrice`）。
///
/// `price_mode=2` 看 `wholesale_price`，其余看 `discount`，均要求 `> 0`；
/// 后台可能留下 0 / NULL 的「空壳」等级行，这类行既不参与等级锁定也不可命中。
#[must_use]
pub fn has_usable_price(row: &ProductWholesalePrice) -> bool {
    if row.price_mode == PRICE_MODE_FIXED {
        row.wholesale_price > Decimal::ZERO
    } else {
        row.discount > Decimal::ZERO
    }
}

/// 是否精确命中该 SKU（Java `isExactSku`：行 `sku_id > 0` 且相等）。
#[must_use]
pub fn is_exact_sku(row_sku_id: i32, in_sku_id: i32) -> bool {
    row_sku_id > 0 && row_sku_id == in_sku_id
}

/// 是否精确命中该等级（Java `isExactGrade`：行 `grade_id > 0` 且相等）。
#[must_use]
pub fn is_exact_grade(row_grade_id: i32, in_grade_id: i32) -> bool {
    row_grade_id > 0 && row_grade_id == in_grade_id
}

/// SKU 维度匹配（Java `matchesSku`：`sku_id = 0` 为 SPU 兜底，匹配任意 SKU）。
#[must_use]
pub fn matches_sku(row_sku_id: i32, in_sku_id: i32) -> bool {
    row_sku_id == 0 || row_sku_id == in_sku_id
}

/// 等级维度匹配（Java `matchesGrade`：`grade_id = 0` 为通用价，匹配任意等级）。
#[must_use]
pub fn matches_grade(row_grade_id: i32, in_grade_id: i32) -> bool {
    row_grade_id == 0 || row_grade_id == in_grade_id
}

/// 命中最优行的排序（Java `buildPriorityComparator`）。
///
/// 优先级：精确 SKU > SPU 兜底；精确等级 > 通用价；`tier_min_qty` 大者优先；
/// 最后按 `id` 升序（保证确定性）。
fn compare_priority(
    a: &ProductWholesalePrice,
    b: &ProductWholesalePrice,
    in_sku_id: i32,
    in_grade_id: i32,
) -> Ordering {
    let a_sku = 1 + i32::from(is_exact_sku(a.sku_id, in_sku_id));
    let b_sku = 1 + i32::from(is_exact_sku(b.sku_id, in_sku_id));
    if a_sku != b_sku {
        return b_sku.cmp(&a_sku);
    }
    let a_grade = 1 + i32::from(is_exact_grade(a.grade_id, in_grade_id));
    let b_grade = 1 + i32::from(is_exact_grade(b.grade_id, in_grade_id));
    if a_grade != b_grade {
        return b_grade.cmp(&a_grade);
    }
    let tier = b.tier_min_qty.cmp(&a.tier_min_qty);
    if tier != Ordering::Equal {
        return tier;
    }
    a.id.cmp(&b.id)
}

/// 价格换算（Java `computeFinalPrice`）。
///
/// `price_mode=2` 直接返回 `wholesale_price`；`price_mode=1` 为
/// `retail_price × discount / 10`，`HALF_UP` 保留 2 位（十分制折扣）。
#[must_use]
fn compute_final_price(hit: &ProductWholesalePrice, retail_price: Decimal) -> Decimal {
    if hit.price_mode == PRICE_MODE_FIXED {
        return hit.wholesale_price;
    }
    (retail_price * hit.discount / Decimal::from(10))
        .round_dp_with_strategy(MONEY_SCALE, RoundingStrategy::MidpointAwayFromZero)
}

/// 批发价命中（Java `WholesalePriceResolver#resolve`，纯函数）。
///
/// 未命中返回 `None`（由上层决定是否回退零售价）。
///
/// 等级锁定：当前等级对该 SKU（含 SPU 兜底）存在**配了可用价格**的行时，
/// 只认本等级的行，通用价行全部剔除（即使数量未达该等级最低阶梯也**不**回退通用价）。
#[must_use]
pub fn resolve_wholesale_price(
    input: &WholesaleResolveInput<'_>,
) -> Option<WholesaleResolveResult> {
    if input.candidates.is_empty() || input.quantity <= 0 || input.grade_id <= 0 {
        return None;
    }
    let effective_qty = match input.tier_match_qty {
        Some(v) if v > 0 => v,
        _ => input.quantity,
    };

    // 1. 等级锁定判定
    let has_grade_specific = input.candidates.iter().any(|p| {
        p.product_id == input.product_id
            && matches_sku(p.sku_id, input.sku_id)
            && p.status == 1
            && p.is_delete == 0
            && is_exact_grade(p.grade_id, input.grade_id)
            && has_usable_price(p)
    });

    // 2. 过滤候选行
    let mut valid: Vec<&ProductWholesalePrice> = input
        .candidates
        .iter()
        .filter(|p| {
            p.product_id == input.product_id
                && p.status == 1
                && p.is_delete == 0
                && matches_sku(p.sku_id, input.sku_id)
                && matches_grade(p.grade_id, input.grade_id)
                && !(has_grade_specific && !is_exact_grade(p.grade_id, input.grade_id))
                && p.tier_min_qty > 0
                && effective_qty >= p.tier_min_qty
                && (p.price_mode == PRICE_MODE_DISCOUNT || p.price_mode == PRICE_MODE_FIXED)
                && has_usable_price(p)
        })
        .collect();

    if valid.is_empty() {
        return None;
    }

    // 3. 排序取最优
    valid.sort_by(|a, b| compare_priority(a, b, input.sku_id, input.grade_id));
    let hit = valid[0];
    let final_price = compute_final_price(hit, input.retail_price);

    Some(WholesaleResolveResult {
        price_id: hit.id,
        final_price,
        price_mode: hit.price_mode,
        discount: hit.discount,
        tier_min_qty: hit.tier_min_qty,
        hit_grade_id: hit.grade_id,
        hit_sku_id: hit.sku_id,
    })
}

// ═════════════════════════ 纯函数：起批量 / 倍数（WholesaleBuyRuleValidator） ═════════════════════════

/// SKU 级配置命中（Java `WholesaleBuyRuleValidator#resolveSkuConfig`）。
///
/// 精确 SKU 优先，未命中回退 SPU 兜底行（`sku_id = 0`）。
/// 背景：单规格商品加车时购物车行的 `skuId` 会被解析成真实 SKU id（`> 0`），
/// 而后台保存倍数时单规格按 `sku_id = 0` 录入。
#[must_use]
pub fn resolve_sku_config<'a>(
    rows: &'a [ProductWholesaleSkuConfig],
    product_id: i32,
    sku_id: i32,
) -> Option<&'a ProductWholesaleSkuConfig> {
    rows.iter()
        .find(|r| r.product_id == product_id && r.sku_id == sku_id)
        .or_else(|| rows.iter().find(|r| r.product_id == product_id && r.sku_id == 0))
}

/// 单行倍数购买校验（Java `WholesaleBuyRuleValidator#checkMultiple`）。
///
/// `multiple <= 1` 或未配置时返回 `None`（通过）。
#[must_use]
pub fn check_multiple(quantity: i32, sku_config: Option<&ProductWholesaleSkuConfig>) -> Option<String> {
    let multiple = sku_config.map(|c| c.multiple)?;
    if multiple <= 1 {
        return None;
    }
    if quantity % multiple != 0 {
        return Some(format!("需为 {multiple} 的倍数（当前 {quantity}）"));
    }
    None
}

/// 倍数购买向上取整对齐（Java `WholesaleBuyRuleValidator#alignUpToMultiple`）。
///
/// `multiple > 1` 时返回 `ceil(quantity / multiple) × multiple`，否则原值返回。
/// 仅对齐倍数，**不**连带起批量。
#[must_use]
pub fn align_up_to_multiple(quantity: i32, sku_config: Option<&ProductWholesaleSkuConfig>) -> i32 {
    let Some(config) = sku_config else {
        return quantity;
    };
    if quantity <= 0 || config.multiple <= 1 {
        return quantity;
    }
    let multiple = config.multiple;
    let remainder = quantity % multiple;
    if remainder == 0 {
        quantity
    } else {
        quantity + (multiple - remainder)
    }
}

/// SKU 维度起批量校验（Java `WholesaleBuyRuleValidator#checkMoqSku`）。
///
/// 非 SKU 维度 / `moq <= 1` 返回 `None`。
#[must_use]
pub fn check_moq_sku(quantity: i32, config: &ProductWholesaleConfig) -> Option<String> {
    if config.moq_dimension != MOQ_DIMENSION_SKU {
        return None;
    }
    let moq = config.moq;
    if moq <= 1 || quantity >= moq {
        return None;
    }
    Some(format!("该规格还差 {} 件凑齐起批量 {} 件", moq - quantity, moq))
}

/// SPU 维度起批量校验（Java `WholesaleBuyRuleValidator#checkMoqSpu`）。
///
/// 非 SPU 维度 / `moq <= 1` 返回 `None`。
#[must_use]
pub fn check_moq_spu(total_qty_in_spu: i32, config: &ProductWholesaleConfig) -> Option<String> {
    if config.moq_dimension != MOQ_DIMENSION_SPU {
        return None;
    }
    let moq = config.moq;
    if moq <= 1 || total_qty_in_spu >= moq {
        return None;
    }
    Some(format!(
        "该商品还差 {} 件凑齐起批量 {} 件",
        moq - total_qty_in_spu,
        moq
    ))
}

/// C 端渠道可用性（Java `WholesaleBuyRuleValidator#isClientEligible`）。
///
/// 非 O2O 形态或取不到店铺上下文时一律放行；O2O 下要求
/// `config.shop_id == context_shop_id`。
#[must_use]
pub fn is_client_eligible(
    config: &ProductWholesaleConfig,
    o2o_channel: bool,
    context_shop_id: Option<i32>,
) -> bool {
    if !o2o_channel {
        return true;
    }
    match context_shop_id {
        None => true,
        Some(shop_id) => config.shop_id == shop_id,
    }
}

/// 组装起批规则展示文案（Java
/// `WholesaleProductQueryServiceImpl#buildBuyRuleHint`，CR-20260604-0937）。
///
/// `dimension = 1` 为 SPU 加总，`dimension = 2` 为每 SKU 起订；倍数取所有
/// SKU 中 `multiple > 1` 的最小值。
#[must_use]
pub fn build_buy_rule_hint(config: &ProductWholesaleConfig, sku_rules: &[WholesaleSkuRuleVO]) -> String {
    let moq = config.moq;
    let mut text = if config.moq_dimension == MOQ_DIMENSION_SPU {
        format!("批发起订 {moq} 件")
    } else {
        format!("按 SKU 起订，每规格 {moq} 件起")
    };
    let multiple = sku_rules
        .iter()
        .map(|r| r.multiple)
        .filter(|m| *m > 1)
        .min();
    if let Some(multiple) = multiple {
        text.push_str(&format!("，需按 {multiple} 的倍数购买"));
    }
    text
}

/// 整批入参校验（Java `WholesaleCartBatchAddDTO` 上的 Bean Validation）。
///
/// 任一失败即整批失败（Java `@Valid` 在整批层面拦截），错误码取
/// `SERVICE_DATA_ERROR(1001)`（`GlobalExceptionHandler` 对
/// `MethodArgumentNotValidException` 的统一映射）。
pub fn validate_batch_items(items: &[WholesaleCartBatchItem]) -> Result<Vec<ValidatedBatchItem>> {
    if items.is_empty() {
        return Err(Error::biz(SERVICE_DATA_ERROR, "items 不能为空"));
    }
    if items.len() > MAX_BATCH_ITEMS {
        return Err(Error::biz(
            SERVICE_DATA_ERROR,
            format!("items 最多 {MAX_BATCH_ITEMS} 个"),
        ));
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let product_id = item
            .product_id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "productId 不能为空"))?;
        if product_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "productId 必须为正数"));
        }
        let sku_id = item
            .sku_id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "skuId 不能为空"))?;
        let quantity = item
            .quantity
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "quantity 不能为空"))?;
        if quantity <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "quantity 必须为正数"));
        }
        out.push(ValidatedBatchItem {
            product_id,
            sku_id,
            quantity,
        });
    }
    Ok(out)
}

/// 行级失败原因（Java `catch` 分支取 `e.getMessage()`）。
#[must_use]
fn failure_reason(e: &Error) -> String {
    match e {
        Error::Biz { msg, .. } => msg.clone(),
        other => {
            let text = other.to_string();
            if text.trim().is_empty() {
                "系统异常".to_string()
            } else {
                text
            }
        }
    }
}

/// 行级失败码（Java：`GlobalException` 取自身 code，其它异常一律
/// `SERVICE_DATA_ERROR`）。
#[must_use]
fn failure_code(e: &Error) -> i32 {
    match e {
        Error::Biz { code, .. } => *code,
        _ => SERVICE_DATA_ERROR,
    }
}

// ═════════════════════════ 服务 ═════════════════════════

pub struct WholesaleClientService {
    repos: Arc<Repositories>,
    /// `nextradeProperties.isO2o == 1`（见模块头第 4 条差异）。
    is_o2o: bool,
}

impl WholesaleClientService {
    /// 标准构造（与项目其它切片一致）：`is_o2o` 从环境变量 `TIG_IS_O2O` 读取。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self::with_o2o(repos, is_o2o_from_env())
    }

    /// 显式指定 O2O 开关（便于单测 / 后续接入真实配置）。
    #[must_use]
    pub fn with_o2o(repos: Arc<Repositories>, is_o2o: bool) -> Self {
        Self { repos, is_o2o }
    }

    // ————————————————————— C1：我是否批发商 —————————————————————

    /// `GET /api/wholesale/me`（Java `WholesaleClientController#me`）。
    ///
    /// 未登录（`user_id <= 0`）、总开关关闭、非批发商三种情形一律返回
    /// `is_wholesaler = false`（不报错），并回填 `global_enabled`。
    pub async fn me(&self, user_id: i32) -> Result<WholesaleMeVO> {
        let global_enabled = self.repos.wholesale.wholesale_client.wholesale_enabled().await?;
        let mut vo = WholesaleMeVO {
            is_wholesaler: false,
            grade_id: None,
            grade_name: None,
            global_enabled,
        };
        if user_id <= 0 || !global_enabled {
            return Ok(vo);
        }
        let Some(wholesaler) = self.repos.wholesale.wholesale_client.find_active_wholesaler(user_id).await? else {
            return Ok(vo);
        };
        let grade_id = wholesaler.grade_id;
        let mut grade_name = None;
        if grade_id > 0 {
            grade_name = self.repos.wholesale.wholesale_client.find_grade_name(grade_id).await?;
        }
        vo.is_wholesaler = true;
        vo.grade_id = Some(grade_id);
        vo.grade_name = grade_name;
        Ok(vo)
    }

    /// 当前会员是否为启用中的批发商（Java
    /// `WholesalerServiceImpl#isWholesaleBuyer`）。
    ///
    /// 短路顺序与 Java 一致：总开关未启用 → `false`（不查库）。
    pub async fn is_wholesale_buyer(&self, user_id: i32) -> Result<bool> {
        if user_id <= 0 || !self.repos.wholesale.wholesale_client.wholesale_enabled().await? {
            return Ok(false);
        }
        Ok(self
            .repos.wholesale.wholesale_client
            .find_active_wholesaler(user_id)
            .await?
            .is_some())
    }

    // ————————————————————— C2：商品起批规则 —————————————————————

    /// `GET /api/wholesale/product/buyRule`（Java
    /// `WholesaleProductQueryServiceImpl#buildBuyRule`）。
    ///
    /// 不做身份判定（非批发商也可查）；未配置批发能力时
    /// `configured = false`（不抛错）。
    pub async fn buy_rule(&self, product_id: i32) -> Result<WholesaleProductBuyRuleVO> {
        let Some(config) = self.repos.wholesale.wholesale_client.find_active_config(product_id).await? else {
            return Ok(WholesaleProductBuyRuleVO {
                product_id,
                configured: false,
                dimension: None,
                moq: None,
                sku_rules: Vec::new(),
                hint: None,
            });
        };

        let sku_configs = self.repos.wholesale.wholesale_client.list_sku_configs(product_id).await?;
        let sku_rules: Vec<WholesaleSkuRuleVO> = sku_configs
            .iter()
            .map(|sc| WholesaleSkuRuleVO {
                sku_id: sc.sku_id,
                multiple: sc.multiple,
            })
            .collect();
        let hint = build_buy_rule_hint(&config, &sku_rules);

        Ok(WholesaleProductBuyRuleVO {
            product_id,
            configured: true,
            dimension: Some(config.moq_dimension),
            moq: Some(config.moq),
            sku_rules,
            hint: Some(hint),
        })
    }

    // ————————————————————— C3：批量加车 —————————————————————

    /// `POST /api/wholesale/cart/addBatch`（Java
    /// `WholesaleCartBatchServiceImpl#addBatch` + 控制器整批级校验）。
    ///
    /// 整批级失败（任意一项 → 整批失败，直接返回 `Err`）：
    /// 未登录 → 401；`wholesaleEnabled = 0` → 5300；
    /// 非批发商 → 403；DTO 校验 → 1001。
    ///
    /// 行级失败不阻断整批：收集进 `failed_items`，成功行照常落库。
    pub async fn add_batch(
        &self,
        user_id: i32,
        dto: &WholesaleCartBatchAddDTO,
    ) -> Result<WholesaleCartBatchResultVO> {
        if user_id <= 0 {
            return Err(Error::biz(401, "用户未登录"));
        }
        if !self.repos.wholesale.wholesale_client.wholesale_enabled().await? {
            return Err(Error::biz(WHOLESALE_DISABLED, "批发功能未启用"));
        }
        if !self.is_wholesale_buyer(user_id).await? {
            return Err(Error::biz(403, "非批发商，无权使用批发批量加车"));
        }

        let items = validate_batch_items(&dto.items)?;

        let grade_id = self
            .repos.wholesale.wholesale_client
            .find_active_wholesaler(user_id)
            .await?
            .map_or(0, |w| w.grade_id);

        let mut result = WholesaleCartBatchResultVO {
            success_count: 0,
            failed_count: 0,
            failed_items: Vec::new(),
        };
        for item in &items {
            match self.add_one_item(user_id, item, grade_id).await {
                Ok(()) => result.success_count += 1,
                Err(e) => {
                    tracing::warn!(
                        "[wholesaleCartBatch] item failed userId={} productId={} skuId={} qty={} code={} reason={}",
                        user_id,
                        item.product_id,
                        item.sku_id,
                        item.quantity,
                        failure_code(&e),
                        failure_reason(&e)
                    );
                    result.failed_items.push(WholesaleCartBatchFailedItem {
                        product_id: item.product_id,
                        sku_id: Some(item.sku_id),
                        reason: failure_reason(&e),
                        code: failure_code(&e),
                    });
                }
            }
        }
        result.failed_count = i32::try_from(result.failed_items.len()).unwrap_or(i32::MAX);
        Ok(result)
    }

    /// 单项加车（Java `CartServiceImpl#addProductToCart` 的批发相关子集 +
    /// `CartWholesaleEnhancer#markAndValidateOnAdd`）。
    async fn add_one_item(
        &self,
        user_id: i32,
        item: &ValidatedBatchItem,
        grade_id: i32,
    ) -> Result<()> {
        // 1. 商品存在性 / 上架状态
        let product = self
            .repos.catalog.product
            .find_by_id(item.product_id)
            .await?
            .filter(|p| p.is_delete == 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "商品不存在"))?;
        if product.product_status == PRODUCT_STATUS_OFF_SALE {
            return Err(Error::biz(SERVICE_DATA_ERROR, "该商品已下架！"));
        }

        // 2. SKU 解析（Java：单规格且未传 skuId 时解析唯一 SKU；无 SKU 行归一为 0）
        let skus = self.repos.catalog.product.find_skus(item.product_id).await?;
        let mut sku_id = item.sku_id;
        if sku_id <= 0 && skus.len() == 1 {
            sku_id = skus[0].sku_id;
        }
        let sku: Option<&ProductSku> = skus.iter().find(|s| s.sku_id == sku_id);
        if sku_id > 0 && !matches!(sku, Some(s) if s.is_enabled == 1) {
            // Java `ProductSkuServiceImpl#checkSkuEnabled`
            return Err(Error::biz(SERVICE_DATA_ERROR, "商品规格已禁用，请重新选择"));
        }
        let sku_id = sku.map_or(0, |s| s.sku_id);

        // 批发行所属店铺（Java `addProductToCart`：非 O2O → `product.shopId`；
        // O2O 且 `dto.shopId != null` → `dto.shopId`。`WholesaleCartBatchServiceImpl`
        // 从不设置 `dto.shopId`，故两种形态下都是 `product.shop_id`）
        let shop_id = product.shop_id;

        // 3. 合并同款已存在行（Java：addCarts 的 quantity = 本项 + 已存在）
        let shop_filter = if self.is_o2o { Some(shop_id) } else { None };
        let existing = self
            .repos.wholesale.wholesale_client
            .find_cart_lines(user_id, item.product_id, CART_TYPE_NORMAL, sku_id, shop_filter)
            .await?;
        let existing_qty: i32 = existing.iter().map(|(_, qty)| *qty).sum();
        let mut quantity = item.quantity.saturating_add(existing_qty);
        let old_cart_ids: Vec<i32> = existing.iter().map(|(id, _)| *id).collect();

        // 4. 零售价 / 规格快照（见模块头第 3 条差异）
        let retail_price = match sku {
            Some(s) => s.sku_price.unwrap_or(product.product_price),
            None => product.current_price(),
        };
        let sku_data = sku
            .and_then(|s| s.sku_data.clone())
            .unwrap_or_else(|| "[]".to_string());

        // 5. 库存校验（Java 在批发校验之前按合并不对齐数量校验一次）
        let stock = match sku {
            Some(s) => s.sku_stock.unwrap_or(0),
            None => product.product_stock,
        };
        if quantity > stock {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                format!("商品:{} 库存不足", product.product_name),
            ));
        }

        // 6. 批发命中标记 + 起批量 / 倍数硬校验
        let mut is_wholesale_line = 0;
        let mut wholesale_grade_id = 0;
        if let Some(config) = self.repos.wholesale.wholesale_client.find_active_config(item.product_id).await? {
            if is_client_eligible(&config, self.is_o2o, Some(shop_id)) {
                let candidates = self.repos.wholesale.wholesale_client.list_prices(item.product_id).await?;
                let sku_configs = self.repos.wholesale.wholesale_client.list_sku_configs(item.product_id).await?;

                // CR-20260604-1430 混批：mode=1 且 SPU 维度时用 Q_total 查档
                let tier_match_qty = if config.multi_spec_price_mode == MULTI_SPEC_PRICE_MODE_MIX
                    && config.moq_dimension == MOQ_DIMENSION_SPU
                {
                    let others = self
                        .repos.wholesale.wholesale_client
                        .sum_other_sku_quantity(
                            user_id,
                            item.product_id,
                            CART_TYPE_NORMAL,
                            shop_id,
                            sku_id,
                        )
                        .await?;
                    Some(others.saturating_add(quantity))
                } else {
                    None
                };

                let hit = resolve_wholesale_price(&WholesaleResolveInput {
                    product_id: item.product_id,
                    sku_id,
                    grade_id,
                    quantity,
                    tier_match_qty,
                    retail_price,
                    candidates: &candidates,
                });

                if hit.is_some() {
                    let sku_config = resolve_sku_config(&sku_configs, item.product_id, sku_id);

                    // 6.1 倍数向上取整对齐（CR-20260612-1622：不再硬抛 5321）
                    let aligned = align_up_to_multiple(quantity, sku_config);
                    if aligned != quantity {
                        if aligned > stock {
                            let multiple = sku_config.map_or(String::new(), |c| c.multiple.to_string());
                            return Err(Error::biz(
                                SERVICE_DATA_ERROR,
                                format!("库存不足，无法满足{multiple}倍购买"),
                            ));
                        }
                        quantity = aligned;
                    }

                    // 6.2 起批量（SPU 维度只统计本项，见模块头第 6 条差异）
                    let moq_error = if config.moq_dimension == MOQ_DIMENSION_SPU {
                        check_moq_spu(quantity, &config)
                    } else {
                        check_moq_sku(quantity, &config)
                    };
                    if let Some(message) = moq_error {
                        return Err(Error::biz(WHOLESALE_MOQ_NOT_MET, message));
                    }

                    is_wholesale_line = 1;
                    wholesale_grade_id = grade_id;
                }
            }
        }

        // 7. 落库：删除同款旧行 + 插入合并后的新行
        let line = NewCartLine {
            user_id,
            product_id: item.product_id,
            product_sn: product.product_sn.clone(),
            pic_thumb: product.pic_thumb.clone(),
            market_price: product.market_price,
            original_price: retail_price,
            quantity,
            sku_id,
            sku_data,
            product_type: product.product_type.unwrap_or(PRODUCT_TYPE_NORMAL),
            shop_id,
            is_wholesale_line,
            wholesale_grade_id,
        };
        self.repos.wholesale.wholesale_client
            .replace_cart_line(user_id, &old_cart_ids, &line)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(moq_dimension: i32, moq: i32, multi_spec_price_mode: i32, shop_id: i32) -> ProductWholesaleConfig {
        ProductWholesaleConfig {
            id: 1,
            product_id: 7,
            shop_id,
            multi_spec_price_mode,
            moq_dimension,
            moq,
            status: 1,
            is_delete: 0,
            create_time: None,
            update_time: None,
        }
    }

    fn price(
        id: i32,
        sku_id: i32,
        grade_id: i32,
        tier_min_qty: i32,
        price_mode: i32,
        discount: Decimal,
        wholesale_price: Decimal,
    ) -> ProductWholesalePrice {
        ProductWholesalePrice {
            id,
            product_id: 7,
            shop_id: 0,
            sku_id,
            grade_id,
            tier_min_qty,
            price_mode,
            discount,
            wholesale_price,
            status: 1,
            is_delete: 0,
            create_time: None,
            update_time: None,
        }
    }

    fn discount_row(id: i32, sku_id: i32, grade_id: i32, tier: i32, discount: i64) -> ProductWholesalePrice {
        price(
            id,
            sku_id,
            grade_id,
            tier,
            PRICE_MODE_DISCOUNT,
            Decimal::new(discount, 2),
            Decimal::ZERO,
        )
    }

    fn input<'a>(
        sku_id: i32,
        grade_id: i32,
        quantity: i32,
        tier_match_qty: Option<i32>,
        candidates: &'a [ProductWholesalePrice],
    ) -> WholesaleResolveInput<'a> {
        WholesaleResolveInput {
            product_id: 7,
            sku_id,
            grade_id,
            quantity,
            tier_match_qty,
            retail_price: Decimal::new(10000, 2), // 100.00
            candidates,
        }
    }

    // ————————————— 批价命中 —————————————

    #[test]
    fn resolve_picks_highest_satisfied_tier() {
        let rows = vec![
            discount_row(1, 0, 3, 10, 800),
            discount_row(2, 0, 3, 50, 600),
            discount_row(3, 0, 3, 100, 400),
        ];
        let hit = resolve_wholesale_price(&input(5, 3, 60, None, &rows)).expect("应命中");
        assert_eq!(hit.price_id, 2);
        assert_eq!(hit.tier_min_qty, 50);
        assert_eq!(hit.final_price, Decimal::new(6000, 2)); // 100.00 × 6.00 / 10 = 60.00
    }

    #[test]
    fn resolve_applies_grade_lock_without_universal_fallback() {
        // 等级 3 配了可用价（tier 100），数量只到 10：等级锁定后通用价被剔除 → 未命中
        let rows = vec![discount_row(1, 0, 3, 100, 400), discount_row(2, 0, 0, 10, 900)];
        assert!(resolve_wholesale_price(&input(5, 3, 10, None, &rows)).is_none());
    }

    #[test]
    fn resolve_falls_back_to_universal_when_grade_rows_are_empty_shells() {
        // 等级 3 只有空壳行（discount = 0）→ 不参与等级锁定，通用价可用
        let rows = vec![
            discount_row(1, 0, 3, 1, 0),
            discount_row(2, 0, 0, 10, 900),
        ];
        let hit = resolve_wholesale_price(&input(5, 3, 10, None, &rows)).expect("应命中通用价");
        assert_eq!(hit.price_id, 2);
        assert_eq!(hit.hit_grade_id, 0);
    }

    #[test]
    fn resolve_prefers_exact_sku_over_spu_fallback() {
        let rows = vec![
            discount_row(1, 0, 3, 1, 500),
            discount_row(2, 9, 3, 1, 300),
        ];
        let hit = resolve_wholesale_price(&input(9, 3, 1, None, &rows)).expect("应命中");
        assert_eq!(hit.price_id, 2);
        assert_eq!(hit.hit_sku_id, 9);
    }

    #[test]
    fn resolve_uses_fixed_price_mode() {
        let rows = vec![price(1, 0, 3, 1, PRICE_MODE_FIXED, Decimal::ZERO, Decimal::new(6099, 2))];
        let hit = resolve_wholesale_price(&input(5, 3, 1, None, &rows)).expect("应命中");
        assert_eq!(hit.final_price, Decimal::new(6099, 2));
        assert_eq!(hit.price_mode, PRICE_MODE_FIXED);
    }

    #[test]
    fn resolve_discount_mode_rounds_half_up_to_two_places() {
        // 10.05 × 3.33 / 10 = 3.34665 → 3.35
        let rows = vec![discount_row(1, 0, 3, 1, 333)];
        let mut inp = input(5, 3, 1, None, &rows);
        inp.retail_price = Decimal::new(1005, 2);
        let hit = resolve_wholesale_price(&inp).expect("应命中");
        assert_eq!(hit.final_price, Decimal::new(335, 2));
    }

    #[test]
    fn resolve_tier_match_qty_overrides_quantity_for_tier_selection() {
        let rows = vec![
            discount_row(1, 0, 3, 10, 900),
            discount_row(2, 0, 3, 50, 600),
        ];
        // 本行数量 5，但混批 Q_total = 50 → 命中 50 档
        let hit = resolve_wholesale_price(&input(5, 3, 5, Some(50), &rows)).expect("应命中");
        assert_eq!(hit.price_id, 2);
    }

    #[test]
    fn resolve_returns_none_for_non_wholesaler_or_empty_candidates() {
        let rows = vec![discount_row(1, 0, 3, 1, 500)];
        assert!(resolve_wholesale_price(&input(5, 0, 10, None, &rows)).is_none());
        assert!(resolve_wholesale_price(&input(5, 3, 0, None, &rows)).is_none());
        assert!(resolve_wholesale_price(&input(5, 3, 10, None, &[])).is_none());
    }

    #[test]
    fn resolve_ignores_disabled_or_deleted_rows() {
        let mut disabled = discount_row(1, 0, 3, 1, 500);
        disabled.status = 0;
        let mut deleted = discount_row(2, 0, 3, 1, 500);
        deleted.is_delete = 1;
        let rows = vec![disabled, deleted];
        assert!(resolve_wholesale_price(&input(5, 3, 10, None, &rows)).is_none());
    }

    // ————————————— SKU 配置 / 起批量 / 倍数 —————————————

    #[test]
    fn resolve_sku_config_prefers_exact_then_spu_fallback() {
        let rows = vec![
            ProductWholesaleSkuConfig { id: 1, product_id: 7, shop_id: 0, sku_id: 0, multiple: 3, is_delete: 0, create_time: None, update_time: None },
            ProductWholesaleSkuConfig { id: 2, product_id: 7, shop_id: 0, sku_id: 9, multiple: 6, is_delete: 0, create_time: None, update_time: None },
        ];
        assert_eq!(resolve_sku_config(&rows, 7, 9).map(|c| c.multiple), Some(6));
        assert_eq!(resolve_sku_config(&rows, 7, 5).map(|c| c.multiple), Some(3));
        assert!(resolve_sku_config(&rows, 8, 5).is_none());
    }

    #[test]
    fn align_up_to_multiple_matches_java_examples() {
        let row = ProductWholesaleSkuConfig { id: 1, product_id: 7, shop_id: 0, sku_id: 0, multiple: 3, is_delete: 0, create_time: None, update_time: None };
        assert_eq!(align_up_to_multiple(2, Some(&row)), 3);
        assert_eq!(align_up_to_multiple(4, Some(&row)), 6);
        assert_eq!(align_up_to_multiple(6, Some(&row)), 6);
        assert_eq!(align_up_to_multiple(6, None), 6);
        assert_eq!(align_up_to_multiple(0, Some(&row)), 0);
    }

    #[test]
    fn check_multiple_only_rejects_when_multiple_gt_one() {
        let expect_six = ProductWholesaleSkuConfig { id: 1, product_id: 7, shop_id: 0, sku_id: 0, multiple: 6, is_delete: 0, create_time: None, update_time: None };
        let unlimited = ProductWholesaleSkuConfig { id: 2, product_id: 7, shop_id: 0, sku_id: 0, multiple: 1, is_delete: 0, create_time: None, update_time: None };
        assert_eq!(check_multiple(5, Some(&expect_six)).as_deref(), Some("需为 6 的倍数（当前 5）"));
        assert!(check_multiple(12, Some(&expect_six)).is_none());
        assert!(check_multiple(5, Some(&unlimited)).is_none());
        assert!(check_multiple(5, None).is_none());
    }

    #[test]
    fn check_moq_routes_by_dimension() {
        let spu = config(MOQ_DIMENSION_SPU, 100, 2, 0);
        let sku = config(MOQ_DIMENSION_SKU, 10, 2, 0);
        assert_eq!(check_moq_spu(60, &spu).as_deref(), Some("该商品还差 40 件凑齐起批量 100 件"));
        assert!(check_moq_spu(100, &spu).is_none());
        // 维度不符 → 交另一分支处理
        assert!(check_moq_spu(1, &sku).is_none());
        assert_eq!(check_moq_sku(4, &sku).as_deref(), Some("该规格还差 6 件凑齐起批量 10 件"));
        assert!(check_moq_sku(10, &sku).is_none());
        assert!(check_moq_sku(1, &spu).is_none());
    }

    #[test]
    fn check_moq_is_noop_when_moq_is_one_or_less() {
        let spu = config(MOQ_DIMENSION_SPU, 1, 2, 0);
        let sku = config(MOQ_DIMENSION_SKU, 0, 2, 0);
        assert!(check_moq_spu(1, &spu).is_none());
        assert!(check_moq_sku(1, &sku).is_none());
    }

    #[test]
    fn client_eligibility_follows_o2o_rules() {
        let cfg = config(MOQ_DIMENSION_SPU, 10, 2, 66);
        assert!(is_client_eligible(&cfg, false, Some(1)));
        assert!(is_client_eligible(&cfg, true, None));
        assert!(is_client_eligible(&cfg, true, Some(66)));
        assert!(!is_client_eligible(&cfg, true, Some(0)));
    }

    #[test]
    fn buy_rule_hint_matches_java_wording() {
        let spu = config(MOQ_DIMENSION_SPU, 100, 2, 0);
        let sku = config(MOQ_DIMENSION_SKU, 10, 2, 0);
        let rules = vec![
            WholesaleSkuRuleVO { sku_id: 1, multiple: 1 },
            WholesaleSkuRuleVO { sku_id: 2, multiple: 6 },
        ];
        assert_eq!(build_buy_rule_hint(&spu, &rules), "批发起订 100 件，需按 6 的倍数购买");
        assert_eq!(build_buy_rule_hint(&sku, &[]), "按 SKU 起订，每规格 10 件起");
    }

    // ————————————— 入参校验 —————————————

    #[test]
    fn validate_batch_items_accepts_valid_payload() {
        let items = vec![
            WholesaleCartBatchItem { product_id: Some(30140), sku_id: Some(5120), quantity: Some(100) },
            WholesaleCartBatchItem { product_id: Some(30141), sku_id: Some(0), quantity: Some(1) },
        ];
        let ok = validate_batch_items(&items).expect("应通过");
        assert_eq!(ok.len(), 2);
        assert_eq!(ok[1].sku_id, 0);
    }

    #[test]
    fn validate_batch_items_rejects_bad_payloads() {
        assert!(validate_batch_items(&[]).is_err());
        let too_many: Vec<_> = (0..MAX_BATCH_ITEMS + 1)
            .map(|_| WholesaleCartBatchItem { product_id: Some(1), sku_id: Some(1), quantity: Some(1) })
            .collect();
        assert!(validate_batch_items(&too_many).is_err());

        let missing_product = vec![WholesaleCartBatchItem { product_id: None, sku_id: Some(1), quantity: Some(1) }];
        assert!(validate_batch_items(&missing_product).is_err());

        let missing_sku = vec![WholesaleCartBatchItem { product_id: Some(1), sku_id: None, quantity: Some(1) }];
        assert!(validate_batch_items(&missing_sku).is_err());

        let zero_quantity = vec![WholesaleCartBatchItem { product_id: Some(1), sku_id: Some(1), quantity: Some(0) }];
        assert!(validate_batch_items(&zero_quantity).is_err());

        let negative_product = vec![WholesaleCartBatchItem { product_id: Some(-1), sku_id: Some(1), quantity: Some(1) }];
        assert!(validate_batch_items(&negative_product).is_err());
    }

    #[test]
    fn failure_code_uses_biz_code_and_defaults_to_service_data_error() {
        assert_eq!(failure_code(&Error::biz(WHOLESALE_MOQ_NOT_MET, "x")), 5320);
        assert_eq!(failure_code(&Error::Validation("x".into())), SERVICE_DATA_ERROR);
        assert_eq!(failure_reason(&Error::biz(5320, "该规格还差 1 件")), "该规格还差 1 件");
    }
}