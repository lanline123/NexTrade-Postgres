//! 库存预警服务（catalog::stock_warning）：通知总开关、预警列表/导出、单个与批量设置、
//! 导入模板与导入。
//!
//! 对应原 Java `com.nextrade.service.product.impl.StockWarningServiceImpl`
//! （754 行）与 `StockWarningController` 的 10 个端点。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `getSetting` / `saveSetting`：主体级开关以 `shop_id` 为主键，无记录视为开启；
//! - `list` / `export`：列表与导出共用同一份 SQL（口径不会两处漂移），
//!   库存状态筛选与分页总数都在 SQL 内完成；导出严格按勾选行取数、
//!   不沿用页面筛选条件；相关商品按当前主体可见性过滤；
//! - `save` / `batchSave`：上下限规则「下限 ≥ 0 且 上限 > 下限」，
//!   批量接口对整批统一校验、逐条落库并汇总成功/失败明细；
//! - 规则保存：存在则更新、不存在则新增，且**阈值变更时重置
//!   `last_warn_status`**（使新规则对后续库存变动立即生效）；
//! - 导入：逐行校验（编码至少填一个、上下限必须为整数、上限大于下限）
//!   与编码解析（商品编码 / SKU 编码分列匹配、SKU 必须属于该商品、
//!   多规格商品必须填 SKU 编码）与 Java 完全一致。
//!
//! **未迁移（本切片明确不做，已在 `uncertain_points` 中列明）**：
//! - **xlsx 解析与写出**：`importExcel` / `getImportErrorData` /
//!   `downloadTemplate` / `export` 的文件形态依赖 Excel 库
//!   （Java 用 EasyExcel），本 crate 未启用 `calamine` 等依赖，
//!   故：导入两接口在完成文件校验后返回明确业务错误、模板接口返回列定义
//!   JSON、导出接口返回行数据 JSON（业务数据本身已完整迁移，仅缺文件包装）；
//! - **配置服务**（`ConfigService`）：`useStoreStock`（依赖 `nextrade.is-o2o`
//!   + `storeUseSoloProductStock`）、`useTotalStock`
//!   （`storeUseTotalProductStock`）与自营门店名称（`getStoreSelfOperatedSettings`）
//!   均依赖配置读取，Rust 侧尚无配置服务，故开关恒为关闭、自营名称回退「自营」；
//! - **多语言翻译**（`TranslatePackage` / `SkuValueTranslateUtils`）：
//!   商品名 / 规格 / 主图按原文返回（单语环境下与 Java 一致）；
//! - **预警发送链路**（`evaluate` → `AdminMsgService.createMessage`）：
//!   库存变动触发的预警判定与站内消息未迁移，本切片只提供纯判定函数
//!   [`StockWarningService::resolve_status`] 供后续接入。

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{StockWarningRule, StockWarningSetting};
use nextrade_core::models::catalog::stock_warning::{
    StockWarningBatchResultVO, StockWarningBatchSaveDTO, StockWarningExcelVO,
    StockWarningExportQuery, StockWarningImportErrorRow, StockWarningImportResultVO,
    StockWarningImportRow, StockWarningListQuery, StockWarningOwnerType, StockWarningPageVO,
    StockWarningSaveDTO, StockWarningSettingSaveDTO, StockWarningSettingVO, StockWarningStatus,
    DEFAULT_LOWER_LIMIT, DISABLED, ENABLED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::stock_warning::{
    StockWarningRow, WarningQuerySpec, WarningSnTarget,
};
use nextrade_infrastructure::db::Repositories;

/// `shop.shop_type = 1` 为店铺（Java `ShopTypeEnum.SHOP`），其余（门店 2、自提点 3）
/// 一并按「门店」展示。
const SHOP_TYPE_SHOP: i32 = 1;

/// 导入文件后缀（Java `Constants.XLSX`）。
const XLSX: &str = ".xlsx";

/// 导出中未配置项的占位符，与页面展示保持一致（Java `EMPTY_CELL`）。
const EMPTY_CELL: &str = "--";

/// 编码命中方式：命中 `product_sku.sku_sn`（Java `MATCH_TYPE_SKU_SN`）。
const MATCH_TYPE_SKU_SN: i32 = 1;

/// 编码命中方式：命中 `product.product_sn`（Java `MATCH_TYPE_PRODUCT_SN`）。
const MATCH_TYPE_PRODUCT_SN: i32 = 2;

/// 编码批量查询的批次大小（Java `SN_QUERY_BATCH_SIZE`）。
const SN_QUERY_BATCH_SIZE: usize = 500;

/// Excel 第 1 行是表头，数据从第 2 行开始（Java `FIRST_DATA_ROW`）。
const FIRST_DATA_ROW: i32 = 2;

/// 导入模板列（Java `TEMPLATE_COLUMNS`，顺序与 `StockWarningImportExcelVO`
/// 的字段声明一致；表头文案取自该 VO 的 `@ExcelProperty`）。
const TEMPLATE_COLUMNS: [(&str, &str); 4] = [
    ("productSn", "商品编码"),
    ("skuSn", "SKU编码"),
    ("lowerLimit", "下限"),
    ("upperLimit", "上限"),
];

/// 上传文件的只读视图（对应 Java `MultipartFile` 在本切片用到的部分）。
#[derive(Debug, Clone, Copy)]
pub struct ImportFile<'a> {
    /// 原始文件名（Java `getOriginalFilename`）
    pub file_name: Option<&'a str>,
    /// 文件字节（Java `isEmpty` → `bytes.is_empty()`）
    pub bytes: &'a [u8],
}

/// 列表「仓库」列展示的主体（对应 Java 内部 record `Owner`）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Owner {
    name: String,
    owner_type: StockWarningOwnerType,
}

/// 校验通过的导入行（对应 Java `ValidatedImportRow` record）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValidatedImportRow {
    lower_limit: i32,
    upper_limit: i32,
    product_id: i32,
    sku_id: i32,
}

/// Java `StrUtil.trim`：`None` 与空白都归一为空串。
fn trimmed(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

/// 主体 ID 归一化（Java `normalizeShopId`：`null` 或负数视为平台自营 0）。
#[must_use]
pub fn normalize_shop_id(shop_id: Option<i32>) -> i32 {
    match shop_id {
        Some(v) if v >= 0 => v,
        _ => 0,
    }
}

/// 解析请求方主体（对应 Java `StockWarningController.resolveShopId`）。
///
/// 平台管理员（`adminType = admin`）可指定主体（不传视为平台自营 0）；
/// 店铺 / 门店 / 自营点强制限定为自身，取不到当前店铺则报
/// 「无法获取当前店铺信息」。
///
/// `admin_type` 传 Java `HeaderUtils.getAdminType()` 的取值，即请求头
/// `x-admin-type`（`shop` + `x-shop-type = 2/3` 会被映射成 `store` / `pickup`）。
/// Java 的 `getShopId()` 只看原始 `x-admin-type` 是否为 `shop` / `pos`
/// （门店账号的原始头同样是 `shop`），故这里用**同一个**取值做判断：
/// 只有 `admin` / `shop` / `pos` 能解析出主体，其余（含缺失）一律报错 ——
/// 与 Java 抛 `GlobalException("无法获取当前店铺信息")` 完全一致。
///
/// Rust 侧 JWT 尚未签发 `shop_id`，故由路由提供请求头 `x-shop-id`；
/// 放在服务层是为了让这段纯逻辑可单测。
pub fn resolve_shop_id(
    requested_shop_id: Option<i32>,
    admin_type: Option<&str>,
    header_shop_id: Option<i32>,
) -> Result<i32> {
    if admin_type == Some("admin") {
        return Ok(requested_shop_id.unwrap_or(0));
    }
    match admin_type {
        Some("shop" | "pos") => match header_shop_id {
            Some(shop_id) if shop_id > 0 => Ok(shop_id),
            _ => Err(Error::biz_msg("无法获取当前店铺信息")),
        },
        _ => Err(Error::biz_msg("无法获取当前店铺信息")),
    }
}

/// 库存状态判定（Java `resolveStatus`）。
///
/// 低于下限为低库存，高于上限为高库存，其余为正常。下限可为空（未配置规则时
/// 只需兼容存量数据的默认下限、无上限故不判高库存）。
///
/// 说明：Java 的调用点是库存变动后的 `evaluate`，该链路（站内消息）未迁移，
/// 故这里作为纯函数提供，供后续接入时复用。
#[must_use]
pub fn resolve_status(stock: i32, lower_limit: Option<i32>, upper_limit: Option<i32>) -> i32 {
    if lower_limit.is_some_and(|limit| stock < limit) {
        return StockWarningStatus::Low.code();
    }
    if upper_limit.is_some_and(|limit| stock > limit) {
        return StockWarningStatus::High.code();
    }
    StockWarningStatus::Normal.code()
}

/// 上下限必填校验（对应 Java DTO 上的 `@NotNull` 文案）。
fn require_limits(lower_limit: Option<i32>, upper_limit: Option<i32>) -> Result<(i32, i32)> {
    let lower = lower_limit.ok_or_else(|| Error::Validation("库存下限不能为空".into()))?;
    let upper = upper_limit.ok_or_else(|| Error::Validation("库存上限不能为空".into()))?;
    Ok((lower, upper))
}

/// 上下限规则校验（Java `checkLimits`）。
///
/// 空值分支与 Java 一致（「库存上下限不能为空」），但控制器上的 `@NotNull`
/// 会先由 [`require_limits`] 拦下，故实际接口不会走到该分支。
fn check_limits(lower_limit: Option<i32>, upper_limit: Option<i32>) -> Result<()> {
    let (Some(lower), Some(upper)) = (lower_limit, upper_limit) else {
        return Err(Error::biz_msg("库存上下限不能为空"));
    };
    if lower < 0 || upper < 0 {
        return Err(Error::biz_msg("库存上下限不能小于0"));
    }
    if upper <= lower {
        return Err(Error::biz_msg("库存上限必须大于库存下限"));
    }
    Ok(())
}

/// `warnEnabled` 必填（对应 Java DTO 的 `@NotNull`）。
fn require_warn_enabled(value: Option<i32>) -> Result<()> {
    if value.is_none() {
        return Err(Error::Validation("预警开关不能为空".into()));
    }
    Ok(())
}

/// Excel 数值单元格常被读成 `100.0`，先按小数解析再要求为整数
/// （Java `parseLimit`，用 `BigDecimal` + `intValueExact`，与 `f64` 无关）。
fn parse_limit(value: Option<&str>, label: &str) -> Result<i32> {
    let text = value.map(str::trim).unwrap_or_default();
    if text.is_empty() {
        return Err(Error::biz_msg(format!("库存{label}不能为空")));
    }
    let invalid = || Error::biz_msg(format!("库存{label}必须为整数：{text}"));
    match Decimal::from_str(text) {
        Ok(decimal) => {
            // `normalize()` 等价 `BigDecimal.stripTrailingZeros()`：
            // `100.0` → `1E+2`（scale = -2），`1.5` → scale = 1
            if decimal.normalize().scale() > 0 {
                return Err(invalid());
            }
            // `rust_decimal::Decimal` 没有 `to_i32()`：先转 `i64`（`scale <= 0`
            // 已保证是整数，超出 `i64` 会返回 `None`），再显式收窄到 `i32`。
            // Java `intValueExact()` 在超出 int 范围时抛 `ArithmeticException`，
            // 被同一个 catch 捕获后同样报「必须为整数」，故越界一律走 `invalid`，
            // 绝不静默截断。
            let integer = match decimal.to_i64() {
                Some(value) => value,
                None => return Err(invalid()),
            };
            i32::try_from(integer).map_err(|_| invalid())
        }
        Err(_) => Err(invalid()),
    }
}

/// 是否是空行（Java `isBlankRow`）：商品编码、SKU编码、上下限全空。
fn is_blank_row(row: &StockWarningImportRow) -> bool {
    let is_blank = |v: &Option<String>| v.as_deref().map(str::trim).unwrap_or_default().is_empty();
    is_blank(&row.product_sn)
        && is_blank(&row.sku_sn)
        && is_blank(&row.upper_limit)
        && is_blank(&row.lower_limit)
}

/// 解析勾选行标识（Java `parseTargets`）：每项格式 `productId_skuId`，
/// 单规格商品 `skuId` 为 0。例：`1001_0,1002_55`。
fn parse_targets(ids: Option<&str>) -> Result<Vec<(i32, i32)>> {
    let text = ids.map(str::trim).unwrap_or_default();
    if text.is_empty() {
        return Err(Error::biz_msg("请先选择要导出的数据"));
    }
    let mut targets: Vec<(i32, i32)> = Vec::new();
    for item in text.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let parts: Vec<&str> = item.split('_').collect();
        let format_error = || {
            Error::biz_msg(format!(
                "导出参数格式错误，应为 productId_skuId：{item}"
            ))
        };
        if parts.len() != 2 {
            return Err(format_error());
        }
        let (Ok(product_id), Ok(sku_id)) = (
            parts[0].trim().parse::<i32>(),
            parts[1].trim().parse::<i32>(),
        ) else {
            return Err(format_error());
        };
        targets.push((product_id, sku_id));
    }
    if targets.is_empty() {
        return Err(Error::biz_msg("请先选择要导出的数据"));
    }
    Ok(targets)
}

/// 解析 `[1, 2, 3]` 形式的相关商品 ID 文本（Java `StringUtils.str2IntList`）。
///
/// **行为差异（已记录）**：Java 对非法项直接抛 `NumberFormatException`，
/// 脏数据会把整个列表接口打成 500；这里跳过非法项，宁可相关商品少展示
/// 也不让列表不可用。
fn parse_int_list(raw: Option<&str>) -> Vec<i32> {
    let text = raw.unwrap_or_default().replace('[', "").replace(']', "");
    if text.trim().is_empty() {
        return Vec::new();
    }
    text.split(',')
        .filter_map(|part| part.trim().parse::<i32>().ok())
        .collect()
}

/// 商品编码与 SKU 编码分列匹配，SKU 必须属于该商品；无法唯一定位时让该行失败
/// （Java `resolveTargetBySn`）。
fn resolve_target_by_sn<'a>(
    sn_index: &'a HashMap<String, Vec<WarningSnTarget>>,
    product_sn: &str,
    sku_sn: &str,
) -> Result<&'a WarningSnTarget> {
    let products: Vec<&WarningSnTarget> = sn_index
        .get(product_sn)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|item| item.match_type == MATCH_TYPE_PRODUCT_SN)
        .collect();

    // 只填了 SKU 编码：直接按 SKU 编码定位
    if product_sn.is_empty() && !sku_sn.is_empty() {
        let matched: Vec<&WarningSnTarget> = sn_index
            .get(sku_sn)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|item| item.match_type == MATCH_TYPE_SKU_SN)
            .collect();
        return match matched.as_slice() {
            [] => Err(Error::biz_msg(format!("编码不存在：{sku_sn}"))),
            [only] => Ok(*only),
            _ => Err(Error::biz_msg(format!(
                "规格编码重复，无法唯一定位规格：{sku_sn}"
            ))),
        };
    }

    if products.is_empty() {
        return Err(Error::biz_msg(format!("编码不存在：{product_sn}")));
    }

    // 同时填了商品编码与 SKU 编码：SKU 必须属于该商品
    if !sku_sn.is_empty() {
        let product_ids: HashSet<i32> = products.iter().map(|item| item.product_id).collect();
        let matched: Vec<&WarningSnTarget> = sn_index
            .get(sku_sn)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|item| {
                item.match_type == MATCH_TYPE_SKU_SN && product_ids.contains(&item.product_id)
            })
            .collect();
        return match matched.as_slice() {
            [] => Err(Error::biz_msg(format!("编码不存在：{sku_sn}"))),
            [only] => Ok(*only),
            _ => Err(Error::biz_msg(format!(
                "规格编码重复，无法唯一定位规格：{sku_sn}"
            ))),
        };
    }

    if products.len() > 1 {
        return Err(Error::biz_msg(format!(
            "商品编码重复，无法唯一定位商品：{product_sn}"
        )));
    }
    let matched = products[0];
    // skuCount 为 0 时支持仅填商品编码，否则必须同时提供 SKU 编码
    if matched.sku_count > 0 {
        return Err(Error::biz_msg(format!(
            "该编码是多规格商品的商品编码，请填写具体规格的规格编码：{product_sn}"
        )));
    }
    Ok(matched)
}

/// 异常 → 面向用户的失败原因（Java：`GlobalException` 取原文，其余「导入失败」）。
fn biz_message(e: &Error) -> String {
    match e {
        Error::Biz { msg, .. } | Error::Validation(msg) => msg.clone(),
        _ => "导入失败".to_string(),
    }
}

/// `None` → 占位符 `--`（Java `emptyIfNull`）。
fn empty_if_none(value: Option<i32>) -> String {
    value.map_or_else(|| EMPTY_CELL.to_string(), |v| v.to_string())
}

/// 查询行 → 列表项（Java `toPageVO`）。
///
/// Java 在商品名 / 规格 / 主图上调用 `translatePackage`（多语言翻译），该服务
/// 未迁移；单语（无翻译配置）环境下 `translate` 原样返回，故这里直接用原文。
fn to_page_vo(
    row: &StockWarningRow,
    owner_shop_id: i32,
    owner: &Owner,
    visible_related_product_ids: &HashSet<i32>,
) -> StockWarningPageVO {
    // 相关商品只在当前库存预警主体可见时返回
    let related_product_ids: Vec<i32> =
        parse_int_list(row.product_related.as_deref())
            .into_iter()
            .filter(|id| visible_related_product_ids.contains(id))
            .collect();
    StockWarningPageVO {
        product_id: row.product_id,
        sku_id: row.sku_id,
        product_name: row.product_name.clone().unwrap_or_default(),
        product_sku_name: row.product_sku_name.clone().unwrap_or_default(),
        product_pic_thumb: row.product_pic_thumb.clone().unwrap_or_default(),
        product_sn: row.product_sn.clone(),
        sku_sn: row.sku_sn.clone(),
        product_tsn: row.product_tsn.clone(),
        shop_id: owner_shop_id,
        shop_name: owner.name.clone(),
        owner_type: owner.owner_type.code(),
        owner_type_name: owner.owner_type.name().to_string(),
        current_stock: row.current_stock.unwrap_or(0),
        transit_stock: row.transit_stock.unwrap_or(0) as i32,
        lower_limit: row.lower_limit,
        upper_limit: row.upper_limit,
        stock_status: row.stock_status,
        stock_status_name: StockWarningStatus::name_of(row.stock_status).map(str::to_string),
        warn_enabled: row.warn_enabled,
        update_time: row.update_time,
        related_product_count: related_product_ids.len() as i32,
        related_product_ids,
    }
}

/// 列表项 → 导出行（Java `toExcelVO`）：保证导出的翻译、状态名与页面完全一致。
fn to_excel_vo(page: &StockWarningPageVO) -> StockWarningExcelVO {
    StockWarningExcelVO {
        product_name: page.product_name.clone(),
        product_sku_name: page.product_sku_name.clone(),
        product_sn: page.product_sn.clone(),
        sku_sn: page.sku_sn.clone(),
        shop_name: page.shop_name.clone(),
        current_stock: page.current_stock,
        transit_stock: page.transit_stock,
        lower_limit: empty_if_none(page.lower_limit),
        upper_limit: empty_if_none(page.upper_limit),
        stock_status_name: page.stock_status_name.clone(),
        warn_enabled_name: match page.warn_enabled {
            None => "未配置".to_string(),
            Some(v) if v == ENABLED => "开启".to_string(),
            _ => "关闭".to_string(),
        },
        related_product_count: page.related_product_count,
    }
}

/// 是否按门店独立库存口径取数（Java `useStoreStock`）。
///
/// Java 要求 `nextrade.is-o2o == 1` **且** 配置项 `storeUseSoloProductStock == "1"`。
/// Rust 侧尚无配置服务与 `nextrade.is-o2o` 读取；两者在 Java 默认部署下都是关闭
/// （`application.yaml: is-o2o: 0`，配置项未配置时 `parseSwitch` 返回 0），
/// 故此处恒为 `false`。**启用 O2O 门店独立库存的环境必须先迁移配置服务**，
/// 否则列表库存口径与 Java 不一致（已记入 uncertain_points）。
#[must_use]
fn use_store_stock(_owner_shop_id: i32) -> bool {
    false
}

/// 是否同时启用总库存（Java `useTotalStock`，配置项 `storeUseTotalProductStock`）。
/// 依赖未迁移的配置服务，见 [`use_store_stock`] 的说明。
#[must_use]
fn use_total_stock() -> bool {
    false
}

pub struct StockWarningService {
    repos: Arc<Repositories>,
}

impl StockWarningService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 通知总开关 —————————————————————

    /// 获取库存预警通知总开关（对应 Java `getSetting`）。
    ///
    /// 无记录视为开启（升级后存量环境的预警能力不中断）。
    pub async fn get_setting(&self, shop_id: Option<i32>) -> Result<StockWarningSettingVO> {
        let owner_shop_id = normalize_shop_id(shop_id);
        let setting = self.repos.catalog.stock_warning.find_setting(owner_shop_id).await?;
        Ok(StockWarningSettingVO {
            shop_id: owner_shop_id,
            notify_enabled: setting.map_or(ENABLED, |s| s.notify_enabled),
            default_lower_limit: DEFAULT_LOWER_LIMIT,
        })
    }

    /// 保存库存预警通知总开关（对应 Java `saveSetting`）。
    pub async fn save_setting(&self, dto: &StockWarningSettingSaveDTO) -> Result<()> {
        let Some(notify_enabled) = dto.notify_enabled else {
            return Err(Error::Validation("库存预警通知开关不能为空".into()));
        };
        let owner_shop_id = normalize_shop_id(dto.shop_id);
        // Java：`Objects.equals(notifyEnabled, 1) ? 1 : 0`
        let notify_enabled = i32::from(notify_enabled == ENABLED);
        let now = utils::now_seconds();
        match self.repos.catalog.stock_warning.find_setting(owner_shop_id).await? {
            None => {
                let setting = StockWarningSetting {
                    shop_id: owner_shop_id,
                    notify_enabled,
                    create_time: now,
                    update_time: now,
                };
                self.repos.catalog.stock_warning.insert_setting(&setting).await
            }
            Some(_) => {
                self.repos.catalog.stock_warning
                    .update_setting(owner_shop_id, notify_enabled, now)
                    .await
            }
        }
    }

    // ————————————————————— 列表 / 导出 —————————————————————

    /// 库存预警列表（对应 Java `list`）。
    pub async fn list(
        &self,
        query: &StockWarningListQuery,
    ) -> Result<PageResult<StockWarningPageVO>> {
        let owner_shop_id = normalize_shop_id(query.shop_id);
        let page = query.page_query();
        let spec = WarningQuerySpec {
            shop_id: owner_shop_id,
            use_store_stock: use_store_stock(owner_shop_id),
            use_total_stock: use_total_stock(),
            targets: Vec::new(),
            product_type: query.product_type,
            keyword_like: query
                .keyword
                .as_deref()
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .map(|k| format!("%{k}%")),
            stock_status: query.stock_status,
            warn_enabled: query.warn_enabled,
            sort_field: query.sort_field.clone(),
            sort_order: query.sort_order.clone(),
        };
        let (total, rows) = self.repos.catalog.stock_warning.paginate_warning(&spec, &page).await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let owner = self.resolve_owner(owner_shop_id).await?;
        let visible = self.visible_related_product_ids(&rows, owner_shop_id).await?;
        let list = rows
            .iter()
            .map(|row| to_page_vo(row, owner_shop_id, &owner, &visible))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 导出库存预警列表（对应 Java `export`）。
    ///
    /// 只按勾选行取数：不沿用页面的筛选条件，导出结果与勾选项严格一一对应。
    ///
    /// **返回形态差异（已记录）**：Java 用 `@ResponseExcel` 直接输出 xlsx；
    /// Rust 侧无 xlsx 写出依赖，故返回同样的行数据 JSON（`data` 为数组）。
    pub async fn export(&self, query: &StockWarningExportQuery) -> Result<Vec<StockWarningExcelVO>> {
        let owner_shop_id = normalize_shop_id(query.shop_id);
        let targets = parse_targets(query.ids.as_deref())?;
        let spec = WarningQuerySpec {
            shop_id: owner_shop_id,
            use_store_stock: use_store_stock(owner_shop_id),
            use_total_stock: use_total_stock(),
            targets,
            product_type: None,
            keyword_like: None,
            stock_status: None,
            warn_enabled: None,
            sort_field: None,
            sort_order: None,
        };
        let rows = self.repos.catalog.stock_warning.list_warning(&spec).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let owner = self.resolve_owner(owner_shop_id).await?;
        let visible = self.visible_related_product_ids(&rows, owner_shop_id).await?;
        Ok(rows
            .iter()
            // 经列表 VO 中转，保证导出的翻译、状态名与页面完全一致
            .map(|row| to_excel_vo(&to_page_vo(row, owner_shop_id, &owner, &visible)))
            .collect())
    }

    // ————————————————————— 规则设置 —————————————————————

    /// 设置单个商品/SKU 库存预警（对应 Java `save`）。
    pub async fn save(&self, dto: &StockWarningSaveDTO) -> Result<()> {
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::Validation("商品ID不能为空".into()))?;
        require_warn_enabled(dto.warn_enabled)?;
        let (lower_limit, upper_limit) = require_limits(dto.lower_limit, dto.upper_limit)?;
        check_limits(Some(lower_limit), Some(upper_limit))?;
        let owner_shop_id = normalize_shop_id(dto.shop_id);
        let sku_id = dto.sku_id.unwrap_or(0);
        self.save_rule(
            owner_shop_id,
            product_id,
            sku_id,
            dto.warn_enabled,
            lower_limit,
            upper_limit,
        )
        .await
    }

    /// 批量设置商品/SKU 库存预警（对应 Java `batchSave`）。
    pub async fn batch_save(
        &self,
        dto: &StockWarningBatchSaveDTO,
    ) -> Result<StockWarningBatchResultVO> {
        let owner_shop_id = normalize_shop_id(dto.shop_id);
        let targets = dto.targets.clone().unwrap_or_default();
        if targets.is_empty() {
            return Err(Error::Validation("请至少选择一个商品".into()));
        }
        require_warn_enabled(dto.warn_enabled)?;
        // 上下限对整批统一生效，非法即整批拒绝，避免逐条产生同一个错误
        let (lower_limit, upper_limit) = require_limits(dto.lower_limit, dto.upper_limit)?;
        check_limits(Some(lower_limit), Some(upper_limit))?;

        let mut result = StockWarningBatchResultVO::default();
        for target in &targets {
            let Some(product_id) = target.product_id else {
                result.add_fail(None, None, "商品ID不能为空".to_string());
                continue;
            };
            let sku_id = target.sku_id.unwrap_or(0);
            match self
                .save_rule(
                    owner_shop_id,
                    product_id,
                    sku_id,
                    dto.warn_enabled,
                    lower_limit,
                    upper_limit,
                )
                .await
            {
                Ok(()) => result.add_success(),
                Err(e) => {
                    tracing::warn!(
                        "库存预警批量设置失败, shopId={owner_shop_id}, productId={product_id}, skuId={sku_id}: {e}"
                    );
                    result.add_fail(Some(product_id), Some(sku_id), biz_message(&e));
                }
            }
        }
        Ok(result)
    }

    /// 保存规则（对应 Java `saveRule`）：存在即更新、不存在即新增。
    ///
    /// 阈值可能已改变，重置 `last_warn_status` 为「正常」，使新规则对后续
    /// 库存变动立即生效。
    async fn save_rule(
        &self,
        owner_shop_id: i32,
        product_id: i32,
        sku_id: i32,
        warn_enabled: Option<i32>,
        lower_limit: i32,
        upper_limit: i32,
    ) -> Result<()> {
        let enabled = i32::from(warn_enabled == Some(ENABLED));
        let now = utils::now_seconds();
        match self
            .repos.catalog.stock_warning
            .find_rule(owner_shop_id, product_id, sku_id)
            .await?
        {
            None => {
                let rule = StockWarningRule {
                    id: 0,
                    shop_id: owner_shop_id,
                    product_id,
                    sku_id,
                    warn_enabled: enabled,
                    lower_limit,
                    upper_limit,
                    last_warn_status: StockWarningStatus::Normal.code(),
                    last_warn_time: 0,
                    is_delete: DISABLED,
                    create_time: now,
                    update_time: now,
                };
                self.repos.catalog.stock_warning.insert_rule(&rule).await?;
            }
            Some(mut rule) => {
                rule.warn_enabled = enabled;
                rule.lower_limit = lower_limit;
                rule.upper_limit = upper_limit;
                rule.last_warn_status = StockWarningStatus::Normal.code();
                rule.update_time = now;
                self.repos.catalog.stock_warning.update_rule(&rule).await?;
            }
        }
        Ok(())
    }

    // ————————————————————— 导入 / 模板 —————————————————————

    /// 下载库存预警导入模板（对应 Java `downloadTemplate`）。
    ///
    /// 路由层直接按 [`TEMPLATE_COLUMNS`] 生成 xlsx；此 JSON 描述保留给
    /// 前端/脚本读取列定义。
    #[must_use]
    pub fn download_template(&self) -> serde_json::Value {
        template_payload()
    }

    /// 导入库存预警配置（对应 Java `importExcel`）：解析 xlsx 后逐行导入。
    pub async fn import_excel(
        &self,
        file: Option<ImportFile<'_>>,
        shop_id: Option<i32>,
    ) -> Result<StockWarningImportResultVO> {
        Self::check_upload(file)?;
        let file = file.expect("check_upload guarantees file");
        let rows = Self::parse_import_rows(file.bytes)?;
        self.import_rows(&rows, shop_id).await
    }

    /// 获取库存预警导入失败数据（对应 Java `getImportErrorData`）。
    pub async fn get_import_error_data(
        &self,
        file: Option<ImportFile<'_>>,
        shop_id: Option<i32>,
    ) -> Result<Vec<StockWarningImportErrorRow>> {
        Self::check_upload(file)?;
        let file = file.expect("check_upload guarantees file");
        let rows = Self::parse_import_rows(file.bytes)?;
        self.collect_import_errors_from_rows(&rows, shop_id).await
    }

    /// 失败数据导出为 xlsx 字节（路由层回写下载流）。
    pub async fn get_import_error_data_xlsx(
        &self,
        file: Option<ImportFile<'_>>,
        shop_id: Option<i32>,
    ) -> Result<Vec<u8>> {
        let rows = self.get_import_error_data(file, shop_id).await?;
        let headers = ["商品编码", "SKU编码", "下限", "上限", "错误信息"];
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                vec![
                    row.product_sn.clone().unwrap_or_default(),
                    row.sku_sn.clone().unwrap_or_default(),
                    row.lower_limit.clone().unwrap_or_default(),
                    row.upper_limit.clone().unwrap_or_default(),
                    row.error_msg.clone(),
                ]
            })
            .collect();
        crate::system::excel::build_xlsx(&headers, &data, "错误数据")
            .map_err(|e| Error::Internal(format!("库存预警失败数据 Excel 生成失败: {e}")))
    }

    /// 处理已解析出的导入行（对应 Java `importRows`）。
    ///
    /// 与文件读取分离，便于直接对逐行校验与编码解析做测试。
    /// 导入意味着要用这些阈值，规则一律置为开启。
    pub async fn import_rows(
        &self,
        rows: &[StockWarningImportRow],
        shop_id: Option<i32>,
    ) -> Result<StockWarningImportResultVO> {
        if rows.is_empty() {
            return Err(Error::biz_msg("表格数据为空"));
        }
        let owner_shop_id = normalize_shop_id(shop_id);
        let sn_index = self.index_targets_by_sn(owner_shop_id, rows).await?;

        let mut result = StockWarningImportResultVO::default();
        for (index, row) in rows.iter().enumerate() {
            if is_blank_row(row) {
                // 模板尾部的空行不计入结果
                continue;
            }
            let row_number = index as i32 + FIRST_DATA_ROW;
            let product_sn = trimmed(row.product_sn.as_deref());
            let sku_sn = trimmed(row.sku_sn.as_deref());
            let outcome = match Self::validate_import_row(row, &sn_index) {
                Ok(validated) => self
                    .save_rule(
                        owner_shop_id,
                        validated.product_id,
                        validated.sku_id,
                        Some(ENABLED),
                        validated.lower_limit,
                        validated.upper_limit,
                    )
                    .await
                    .map(|()| validated),
                Err(e) => Err(e),
            };
            match outcome {
                Ok(_) => result.add_success(),
                Err(e) => {
                    tracing::warn!(
                        "库存预警导入失败, shopId={owner_shop_id}, row={row_number}, sn={sku_sn}: {e}"
                    );
                    result.add_fail(
                        row_number,
                        Some(product_sn),
                        Some(sku_sn),
                        biz_message(&e),
                    );
                }
            }
        }
        if result.total_count == 0 {
            return Err(Error::biz_msg("表格数据为空"));
        }
        Ok(result)
    }

    /// 逐行校验并汇总失败明细（对应 Java `getImportErrorData` 的循环体）。
    pub async fn collect_import_errors_from_rows(
        &self,
        rows: &[StockWarningImportRow],
        shop_id: Option<i32>,
    ) -> Result<Vec<StockWarningImportErrorRow>> {
        let owner_shop_id = normalize_shop_id(shop_id);
        let sn_index = self.index_targets_by_sn(owner_shop_id, rows).await?;
        let mut errors = Vec::new();
        for row in rows {
            if is_blank_row(row) {
                continue;
            }
            if let Err(e) = Self::validate_import_row(row, &sn_index) {
                errors.push(StockWarningImportErrorRow {
                    product_sn: row.product_sn.clone(),
                    sku_sn: row.sku_sn.clone(),
                    lower_limit: row.lower_limit.clone(),
                    upper_limit: row.upper_limit.clone(),
                    error_msg: biz_message(&e),
                });
            }
        }
        Ok(errors)
    }

    /// 文件前置校验（Java `readImportRows` 的前两步）。
    /// 解析库存预警导入 xlsx（首行表头；列序与 TEMPLATE_COLUMNS 一致）。
    fn parse_import_rows(bytes: &[u8]) -> Result<Vec<StockWarningImportRow>> {
        let sheet = crate::system::excel::read_xlsx(bytes)?;
        let mut rows = Vec::new();
        for line in sheet.iter().skip(1) {
            let get = |i: usize| {
                line.get(i)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            let row = StockWarningImportRow {
                product_sn: get(0),
                sku_sn: get(1),
                lower_limit: get(2),
                upper_limit: get(3),
            };
            if !is_blank_row(&row) {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    fn check_upload(file: Option<ImportFile<'_>>) -> Result<()> {
        let Some(file) = file else {
            return Err(Error::biz_msg("请上传文件"));
        };
        if file.bytes.is_empty() {
            return Err(Error::biz_msg("请上传文件"));
        }
        match file.file_name {
            Some(name) if name.to_ascii_lowercase().ends_with(XLSX) => Ok(()),
            _ => Err(Error::biz_msg("请上传.xlsx格式文件")),
        }
    }

    /// 一次查完全部编码后按编码分组，避免逐行查库（Java `indexTargetsBySn`）。
    async fn index_targets_by_sn(
        &self,
        owner_shop_id: i32,
        rows: &[StockWarningImportRow],
    ) -> Result<HashMap<String, Vec<WarningSnTarget>>> {
        let mut sn_list: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for row in rows {
            for sn in [
                trimmed(row.product_sn.as_deref()),
                trimmed(row.sku_sn.as_deref()),
            ] {
                if !sn.is_empty() && seen.insert(sn.clone()) {
                    sn_list.push(sn);
                }
            }
        }
        let mut index: HashMap<String, Vec<WarningSnTarget>> = HashMap::new();
        if sn_list.is_empty() {
            return Ok(index);
        }
        // 行数不可控，分批查询避免 IN 列表过长
        for batch in sn_list.chunks(SN_QUERY_BATCH_SIZE) {
            for target in self
                .repos.catalog.stock_warning
                .select_targets_by_sn(owner_shop_id, batch)
                .await?
            {
                index.entry(target.sn.clone()).or_default().push(target);
            }
        }
        Ok(index)
    }

    /// 单行校验（Java `validateImportRow`）：编码至少填一个 → 上下限为整数
    /// → 上限大于下限 → 编码能唯一定位到主体内的商品/SKU。
    fn validate_import_row(
        row: &StockWarningImportRow,
        sn_index: &HashMap<String, Vec<WarningSnTarget>>,
    ) -> Result<ValidatedImportRow> {
        let product_sn = trimmed(row.product_sn.as_deref());
        let sku_sn = trimmed(row.sku_sn.as_deref());
        if product_sn.is_empty() && sku_sn.is_empty() {
            return Err(Error::biz_msg("规格编码不能为空"));
        }
        let lower_limit = parse_limit(row.lower_limit.as_deref(), "下限")?;
        let upper_limit = parse_limit(row.upper_limit.as_deref(), "上限")?;
        check_limits(Some(lower_limit), Some(upper_limit))?;
        let target = resolve_target_by_sn(sn_index, &product_sn, &sku_sn)?;
        Ok(ValidatedImportRow {
            lower_limit,
            upper_limit,
            product_id: target.product_id,
            sku_id: target.sku_id,
        })
    }

    // ————————————————————— 主体（仓库列） —————————————————————

    /// 解析列表「仓库」列展示的主体（Java `resolveOwner`）。
    ///
    /// 自营不是 `shop` 表记录，名称取平台自营门店设置（配置服务未迁移，
    /// 故回退 Java 的兜底文案「自营」）；店铺与门店取 `shop` 表标题与类型，
    /// 自提点与门店同属线下主体，一并按门店展示。
    async fn resolve_owner(&self, owner_shop_id: i32) -> Result<Owner> {
        if owner_shop_id <= 0 {
            return Ok(Owner {
                name: StockWarningOwnerType::SelfOperated.name().to_string(),
                owner_type: StockWarningOwnerType::SelfOperated,
            });
        }
        match self.repos.catalog.stock_warning.find_shop_brief(owner_shop_id).await? {
            None => Ok(Owner {
                name: String::new(),
                owner_type: StockWarningOwnerType::Shop,
            }),
            Some(shop) => {
                let owner_type = if shop.shop_type == Some(SHOP_TYPE_SHOP) {
                    StockWarningOwnerType::Shop
                } else {
                    StockWarningOwnerType::Store
                };
                Ok(Owner {
                    name: shop.shop_title.unwrap_or_default(),
                    owner_type,
                })
            }
        }
    }

    /// 批量查询相关商品在当前主体的可见性（Java `queryVisibleRelatedProductIds`）。
    async fn visible_related_product_ids(
        &self,
        rows: &[StockWarningRow],
        owner_shop_id: i32,
    ) -> Result<HashSet<i32>> {
        let mut ids: Vec<i32> = Vec::new();
        let mut seen: HashSet<i32> = HashSet::new();
        for row in rows {
            for id in parse_int_list(row.product_related.as_deref()) {
                if seen.insert(id) {
                    ids.push(id);
                }
            }
        }
        if ids.is_empty() {
            return Ok(HashSet::new());
        }
        Ok(self
            .repos.catalog.stock_warning
            .select_visible_product_ids(owner_shop_id, &ids)
            .await?
            .into_iter()
            .collect())
    }
}

/// 导入模板的列定义（Java `TEMPLATE_COLUMNS` + `StockWarningImportExcelVO`
/// 的 `@ExcelProperty` 表头），供 [`StockWarningService::download_template`] 返回。
#[must_use]
pub fn template_payload() -> serde_json::Value {
    serde_json::json!({
        "fileName": "库存预警导入模板.xlsx",
        "sheetName": "模板",
        "columns": TEMPLATE_COLUMNS
            .iter()
            .map(|(field, title)| serde_json::json!({ "field": field, "title": title }))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(product_id: i32, sku_id: i32, match_type: i32, sku_count: i32) -> WarningSnTarget {
        WarningSnTarget {
            sn: String::new(),
            product_id,
            sku_id,
            match_type,
            sku_count,
        }
    }

    fn index(entries: &[(&str, Vec<WarningSnTarget>)]) -> HashMap<String, Vec<WarningSnTarget>> {
        entries
            .iter()
            .map(|(sn, items)| ((*sn).to_string(), items.clone()))
            .collect()
    }

    fn import_row(product_sn: &str, sku_sn: &str, lower: &str, upper: &str) -> StockWarningImportRow {
        StockWarningImportRow {
            product_sn: Some(product_sn.to_string()),
            sku_sn: Some(sku_sn.to_string()),
            lower_limit: Some(lower.to_string()),
            upper_limit: Some(upper.to_string()),
        }
    }

    // ————————————————— 主体解析 —————————————————

    #[test]
    fn resolve_shop_id_follows_java_resolve_shop() {
        // 平台管理员：可指定主体，不传视为平台自营 0
        assert_eq!(resolve_shop_id(None, Some("admin"), None).unwrap(), 0);
        assert_eq!(resolve_shop_id(Some(9), Some("admin"), Some(3)).unwrap(), 9);
        // 店铺 / POS：强制自身，取不到则报「无法获取当前店铺信息」
        assert_eq!(resolve_shop_id(Some(9), Some("shop"), Some(3)).unwrap(), 3);
        assert_eq!(resolve_shop_id(None, Some("pos"), Some(4)).unwrap(), 4);
        assert!(resolve_shop_id(None, Some("shop"), None).is_err());
        assert!(resolve_shop_id(None, Some("shop"), Some(0)).is_err());
        assert!(resolve_shop_id(None, Some("shop"), Some(-1)).is_err());
        // Java `HeaderUtils.getShopId()` 对其他账号类型返回 null → 抛异常
        assert!(resolve_shop_id(Some(9), Some("vendor"), Some(3)).is_err());
        assert!(resolve_shop_id(Some(9), Some("store"), Some(3)).is_err());
        assert!(resolve_shop_id(None, None, None).is_err());
    }

    #[test]
    fn normalize_shop_id_matches_java() {
        assert_eq!(normalize_shop_id(None), 0);
        assert_eq!(normalize_shop_id(Some(-3)), 0);
        assert_eq!(normalize_shop_id(Some(0)), 0);
        assert_eq!(normalize_shop_id(Some(12)), 12);
    }

    // ————————————————— 状态判定 —————————————————

    #[test]
    fn resolve_status_matches_java_thresholds() {
        // 未配置规则：默认下限兼容存量数据，无上限故不判高库存
        assert_eq!(resolve_status(50, Some(DEFAULT_LOWER_LIMIT), None), 1);
        assert_eq!(resolve_status(100, Some(DEFAULT_LOWER_LIMIT), None), 0);
        assert_eq!(resolve_status(150, Some(DEFAULT_LOWER_LIMIT), None), 0);
        // 已配置规则：低于下限 → 低库存；高于上限 → 高库存
        assert_eq!(resolve_status(5, Some(10), Some(20)), 1);
        assert_eq!(resolve_status(10, Some(10), Some(20)), 0);
        assert_eq!(resolve_status(20, Some(10), Some(20)), 0);
        assert_eq!(resolve_status(21, Some(10), Some(20)), 2);
        // 无任何阈值 → 正常
        assert_eq!(resolve_status(0, None, None), 0);
    }

    // ————————————————— 上下限校验 —————————————————

    #[test]
    fn check_limits_matches_java_messages() {
        assert!(check_limits(Some(10), Some(20)).is_ok());
        assert_eq!(
            check_limits(None, Some(20)).unwrap_err().to_string(),
            "业务错误 [500]: 库存上下限不能为空"
        );
        assert_eq!(
            check_limits(Some(-1), Some(20)).unwrap_err().to_string(),
            "业务错误 [500]: 库存上下限不能小于0"
        );
        // 上限必须**大于**下限
        assert_eq!(
            check_limits(Some(20), Some(20)).unwrap_err().to_string(),
            "业务错误 [500]: 库存上限必须大于库存下限"
        );
        assert_eq!(
            require_limits(None, Some(20)).unwrap_err().to_string(),
            "参数校验失败: 库存下限不能为空"
        );
        assert!(require_warn_enabled(None).is_err());
        assert!(require_warn_enabled(Some(0)).is_ok());
    }

    // ————————————————— 导入数值解析 —————————————————

    #[test]
    fn parse_limit_accepts_excel_style_integers() {
        assert_eq!(parse_limit(Some("100"), "下限").unwrap(), 100);
        // Excel 数值单元格常被读成 100.0
        assert_eq!(parse_limit(Some("100.0"), "下限").unwrap(), 100);
        assert_eq!(parse_limit(Some(" 20 "), "上限").unwrap(), 20);
        assert_eq!(parse_limit(Some("0"), "下限").unwrap(), 0);
        assert_eq!(parse_limit(Some("-5"), "下限").unwrap(), -5);

        assert_eq!(
            parse_limit(Some("1.5"), "上限").unwrap_err().to_string(),
            "业务错误 [500]: 库存上限必须为整数：1.5"
        );
        assert_eq!(
            parse_limit(Some("abc"), "下限").unwrap_err().to_string(),
            "业务错误 [500]: 库存下限必须为整数：abc"
        );
        // 超出 i32 范围：Java `intValueExact()` 会抛 `ArithmeticException`，
        // 与 `abc` 一样报「必须为整数」，绝不截断成其它数值
        assert_eq!(
            parse_limit(Some("2147483648"), "下限").unwrap_err().to_string(),
            "业务错误 [500]: 库存下限必须为整数：2147483648"
        );
        // 超出 i64 范围（`to_i64()` 返回 `None`）同理
        assert_eq!(
            parse_limit(Some("99999999999999999999"), "上限")
                .unwrap_err()
                .to_string(),
            "业务错误 [500]: 库存上限必须为整数：99999999999999999999"
        );
        assert_eq!(
            parse_limit(Some("   "), "下限").unwrap_err().to_string(),
            "业务错误 [500]: 库存下限不能为空"
        );
        assert_eq!(
            parse_limit(None, "上限").unwrap_err().to_string(),
            "业务错误 [500]: 库存上限不能为空"
        );
    }

    #[test]
    fn is_blank_row_requires_all_four_cells_blank() {
        assert!(is_blank_row(&StockWarningImportRow::default()));
        assert!(is_blank_row(&import_row("  ", "", " ", "")));
        assert!(!is_blank_row(&import_row("P1", "", "", "")));
        assert!(!is_blank_row(&import_row("", "", "1", "")));
    }

    // ————————————————— 编码解析 —————————————————

    #[test]
    fn resolve_target_by_product_sn_single_spec() {
        let idx = index(&[
            ("P1", vec![target(11, 0, MATCH_TYPE_PRODUCT_SN, 0)]),
        ]);
        let matched = resolve_target_by_sn(&idx, "P1", "").unwrap();
        assert_eq!((matched.product_id, matched.sku_id), (11, 0));
    }

    #[test]
    fn resolve_target_rejects_multi_spec_product_sn_without_sku_sn() {
        let idx = index(&[("P1", vec![target(11, 0, MATCH_TYPE_PRODUCT_SN, 3)])]);
        let err = resolve_target_by_sn(&idx, "P1", "").unwrap_err().to_string();
        assert!(err.contains("该编码是多规格商品的商品编码"));
    }

    #[test]
    fn resolve_target_by_sku_sn_only() {
        let idx = index(&[("S1", vec![target(11, 55, MATCH_TYPE_SKU_SN, 0)])]);
        let matched = resolve_target_by_sn(&idx, "", "S1").unwrap();
        assert_eq!((matched.product_id, matched.sku_id), (11, 55));

        // 编码不存在
        let err = resolve_target_by_sn(&idx, "", "S9").unwrap_err().to_string();
        assert!(err.contains("编码不存在：S9"));
        // 规格编码重复
        let idx = index(&[(
            "S1",
            vec![target(11, 55, MATCH_TYPE_SKU_SN, 0), target(12, 56, MATCH_TYPE_SKU_SN, 0)],
        )]);
        let err = resolve_target_by_sn(&idx, "", "S1").unwrap_err().to_string();
        assert!(err.contains("规格编码重复，无法唯一定位规格：S1"));
    }

    #[test]
    fn resolve_target_cross_checks_sku_belongs_to_product() {
        // SKU 属于另一个商品 → 编码不存在
        let idx = index(&[
            ("P1", vec![target(11, 0, MATCH_TYPE_PRODUCT_SN, 1)]),
            ("S1", vec![target(12, 55, MATCH_TYPE_SKU_SN, 0)]),
        ]);
        let err = resolve_target_by_sn(&idx, "P1", "S1").unwrap_err().to_string();
        assert!(err.contains("编码不存在：S1"));

        // SKU 属于该商品 → 命中
        let idx = index(&[
            ("P1", vec![target(11, 0, MATCH_TYPE_PRODUCT_SN, 1)]),
            ("S1", vec![target(11, 55, MATCH_TYPE_SKU_SN, 0)]),
        ]);
        let matched = resolve_target_by_sn(&idx, "P1", "S1").unwrap();
        assert_eq!((matched.product_id, matched.sku_id), (11, 55));
    }

    #[test]
    fn resolve_target_rejects_duplicate_product_sn_and_missing_sn() {
        let idx = index(&[
            ("P1", vec![target(11, 0, MATCH_TYPE_PRODUCT_SN, 0), target(12, 0, MATCH_TYPE_PRODUCT_SN, 0)]),
        ]);
        let err = resolve_target_by_sn(&idx, "P1", "").unwrap_err().to_string();
        assert!(err.contains("商品编码重复，无法唯一定位商品：P1"));

        let err = resolve_target_by_sn(&idx, "PX", "").unwrap_err().to_string();
        assert!(err.contains("编码不存在：PX"));
    }

    #[test]
    fn validate_import_row_orders_checks_like_java() {
        let idx = index(&[
            ("P1", vec![target(11, 0, MATCH_TYPE_PRODUCT_SN, 0)]),
            ("S1", vec![target(11, 55, MATCH_TYPE_SKU_SN, 0)]),
        ]);
        // 编码全空 → 规格编码不能为空
        let row = import_row("", "", "1", "2");
        assert_eq!(
            StockWarningService::validate_import_row(&row, &idx)
                .unwrap_err()
                .to_string(),
            "业务错误 [500]: 规格编码不能为空"
        );
        // 上下限非法先于编码解析失败
        let row = import_row("PX", "", "5", "5");
        assert_eq!(
            StockWarningService::validate_import_row(&row, &idx)
                .unwrap_err()
                .to_string(),
            "业务错误 [500]: 库存上限必须大于库存下限"
        );
        // 正常行
        let row = import_row("P1", "S1", "10", "20");
        let ok = StockWarningService::validate_import_row(&row, &idx).unwrap();
        assert_eq!(ok, ValidatedImportRow { lower_limit: 10, upper_limit: 20, product_id: 11, sku_id: 55 });
    }

    #[test]
    fn check_upload_requires_non_empty_xlsx() {
        assert_eq!(
            StockWarningService::check_upload(None).unwrap_err().to_string(),
            "业务错误 [500]: 请上传文件"
        );
        let empty = ImportFile { file_name: Some("a.xlsx"), bytes: &[] };
        assert_eq!(
            StockWarningService::check_upload(Some(empty)).unwrap_err().to_string(),
            "业务错误 [500]: 请上传文件"
        );
        let csv = ImportFile { file_name: Some("a.CSV"), bytes: b"x" };
        assert_eq!(
            StockWarningService::check_upload(Some(csv)).unwrap_err().to_string(),
            "业务错误 [500]: 请上传.xlsx格式文件"
        );
        // 大小写不敏感（Java originalName.toLowerCase().endsWith(".xlsx")）
        let ok = ImportFile { file_name: Some("模板.XLSX"), bytes: b"x" };
        assert!(StockWarningService::check_upload(Some(ok)).is_ok());
        let unnamed = ImportFile { file_name: None, bytes: b"x" };
        assert!(StockWarningService::check_upload(Some(unnamed)).is_err());
    }

    // ————————————————— 导出参数 —————————————————

    #[test]
    fn parse_targets_matches_java_format() {
        assert_eq!(
            parse_targets(Some("1001_0,1002_55")).unwrap(),
            vec![(1001, 0), (1002, 55)]
        );
        // 空白项被跳过
        assert_eq!(parse_targets(Some(" 1_2 , ")).unwrap(), vec![(1, 2)]);
        assert_eq!(
            parse_targets(Some(" ")).unwrap_err().to_string(),
            "业务错误 [500]: 请先选择要导出的数据"
        );
        assert_eq!(
            parse_targets(None).unwrap_err().to_string(),
            "业务错误 [500]: 请先选择要导出的数据"
        );
        assert!(parse_targets(Some("1001"))
            .unwrap_err()
            .to_string()
            .contains("导出参数格式错误，应为 productId_skuId：1001"));
        assert!(parse_targets(Some("a_b"))
            .unwrap_err()
            .to_string()
            .contains("导出参数格式错误"));
        assert!(parse_targets(Some("1_2_3")).is_err());
        // 逗号分隔但全为空项 → 视为未选择
        assert!(parse_targets(Some(",")).is_err());
    }

    // ————————————————— 派生字段 —————————————————

    #[test]
    fn parse_int_list_tolerates_java_array_text() {
        assert_eq!(parse_int_list(Some("[1, 2, 3]")), vec![1, 2, 3]);
        assert_eq!(parse_int_list(Some("[]")), Vec::<i32>::new());
        assert_eq!(parse_int_list(Some("")), Vec::<i32>::new());
        assert_eq!(parse_int_list(None), Vec::<i32>::new());
        assert_eq!(parse_int_list(Some("[1, x, 2]")), vec![1, 2]);
    }

    #[test]
    fn to_page_vo_maps_owner_and_visible_related_products() {
        let row = StockWarningRow {
            product_id: 7,
            sku_id: 0,
            product_name: Some("商品A".into()),
            product_sku_name: None,
            product_pic_thumb: Some("a.png".into()),
            product_sn: Some("P7".into()),
            sku_sn: None,
            product_tsn: None,
            product_shop_id: Some(0),
            store_shop_id: None,
            product_related: Some("[1, 2, 3]".into()),
            rule_id: Some(5),
            warn_enabled: Some(ENABLED),
            lower_limit: Some(10),
            upper_limit: Some(20),
            update_time: Some(1_700_000_000),
            current_stock: Some(5),
            transit_stock: Some(3),
            stock_status: Some(StockWarningStatus::Low.code()),
        };
        let owner = Owner {
            name: "自营".into(),
            owner_type: StockWarningOwnerType::SelfOperated,
        };
        let visible: HashSet<i32> = [1, 3].into_iter().collect();
        let vo = to_page_vo(&row, 0, &owner, &visible);
        assert_eq!(vo.shop_id, 0);
        assert_eq!(vo.shop_name, "自营");
        assert_eq!(vo.owner_type, 1);
        assert_eq!(vo.owner_type_name, "自营");
        // 相关商品按主体可见性过滤
        assert_eq!(vo.related_product_ids, vec![1, 3]);
        assert_eq!(vo.related_product_count, 2);
        assert_eq!(vo.stock_status_name.as_deref(), Some("低库存"));
        assert_eq!(vo.current_stock, 5);
        assert_eq!(vo.transit_stock, 3);
    }

    #[test]
    fn to_excel_vo_uses_placeholders_and_switch_names() {
        let page = StockWarningPageVO {
            product_name: "A".into(),
            product_sku_name: "红色".into(),
            product_sn: Some("P1".into()),
            sku_sn: Some("S1".into()),
            shop_name: "门店1".into(),
            current_stock: 3,
            transit_stock: 0,
            lower_limit: None,
            upper_limit: Some(30),
            stock_status_name: Some("未配置".into()),
            warn_enabled: None,
            related_product_count: 0,
            ..Default::default()
        };
        let vo = to_excel_vo(&page);
        assert_eq!(vo.lower_limit, "--");
        assert_eq!(vo.upper_limit, "30");
        assert_eq!(vo.warn_enabled_name, "未配置");

        let page = StockWarningPageVO {
            warn_enabled: Some(DISABLED),
            ..page.clone()
        };
        assert_eq!(to_excel_vo(&page).warn_enabled_name, "关闭");
        let page = StockWarningPageVO {
            warn_enabled: Some(ENABLED),
            ..page
        };
        assert_eq!(to_excel_vo(&page).warn_enabled_name, "开启");
    }

    #[test]
    fn template_payload_describes_java_columns() {
        let tpl = template_payload();
        assert_eq!(tpl["fileName"], "库存预警导入模板.xlsx");
        assert_eq!(tpl["sheetName"], "模板");
        let columns = tpl["columns"].as_array().unwrap();
        let fields: Vec<&str> = columns
            .iter()
            .map(|c| c["field"].as_str().unwrap())
            .collect();
        assert_eq!(fields, vec!["productSn", "skuSn", "lowerLimit", "upperLimit"]);
        let titles: Vec<&str> = columns
            .iter()
            .map(|c| c["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["商品编码", "SKU编码", "下限", "上限"]);
        assert_eq!(columns.len(), TEMPLATE_COLUMNS.len());
    }
}