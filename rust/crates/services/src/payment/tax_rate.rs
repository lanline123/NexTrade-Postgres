//! 税率管理服务（payment::tax_rate）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.TaxRateServiceImpl`（168 行）
//! + `TaxRateController`（75 行）中的 5 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /setting/taxRate/list` | `TaxRateController.list` | [`TaxRateService::list`] |
//! | `GET  /setting/taxRate/detail` | `TaxRateController.detail` | [`TaxRateService::detail`] |
//! | `POST /setting/taxRate/create` | `TaxRateController.create` | [`TaxRateService::create`] |
//! | `POST /setting/taxRate/update` | `TaxRateController.update` | [`TaxRateService::update`] |
//! | `POST /setting/taxRate/del` | `TaxRateController.del` | [`TaxRateService::del`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### `list`
//! - `regionName` 走 `LIKE %regionName%`（`StrUtil.isNotBlank` 口径：
//!   `"   "` 视为空，**不加**过滤）。
//! - `isActive != null` 才 `eq`。
//! - `startTime` / `endTime` 经 `StringUtils.dateToTimestampExample` 转
//!   **秒级时间戳**后做 `ge` / `le`。
//! - 排序是**二选一**的分支（不是「默认排序 + 可选覆盖」）：
//!   `sortField` 非空 → `buildSortOrder`；为空 →
//!   `orderByDesc(isActive).orderByDesc(createTime)`。
//!   **两条路径互斥**，务必按 [`TaxRateListQuery::uses_default_order`] 分支。
//!
//! ### `detail`
//! - Java 记录不存在时抛 `GlobalException("税率不存在")`，故 Rust 返回 404
//!   同文案（项目内 order_admin/005/017 的既有约定）。
//!
//! ### `create`
//! - `regionName` 为 blank **且** `regionId != null` 时，用 `regionId`
//!   回查 `region` 表补全名称（Java 调 `regionService.getById`）。
//!   注意条件里 `dto.getRegionId() != null` 是多余的（`@NotNull` 已保证），
//!   但保留该判断与 Java 一致。
//! - `isActive == null` → 默认 [`YES`]（`Constants.YES = 1`）。
//! - `createTime` / `updateTime` 都置为**当前秒级时间戳**
//!   （`StringUtils.getCurrentTime() = System.currentTimeMillis() / 1000`）。
//!
//! ### `update`
//! - **先判 `taxRateId` 是否为空**，为空抛 `"税率ID不能为空"`
//!   （这是 Java 的**首条**语句，先于「记录不存在」检查）。
//! - `getById` 取旧行，不存在抛 `"税率不存在"`。
//! - 记住旧 `regionId`，`BeanUtils.copyProperties(dto, taxRate)`
//!   把 DTO 的非 null 字段覆盖到**旧实体**上
//!   ⇒ **DTO 未传的字段保留旧值**（本实现显式复刻，见下）。
//! - `dto.regionId != null && !dto.regionId.equals(oldRegionId)` 时回查
//!   `region` 表刷新 `regionName`。**注意是「变了才刷新」**：
//!   若 `regionId` 没变，即便请求里带了 `regionName`，也会被
//!   `copyProperties` 的结果覆盖（请求值优先）。
//! - `updateTime` 置当前时间戳；`createTime` **不动**。
//!
//! ### `del`
//! - `removeById(id)` 硬删除（表无软删列）。
//!
//! ## ⚠️ 相对 Java 的一处**有意增强**（已列入 `uncertain_points`）
//!
//! Java 的 `BeanUtils.copyProperties(dto, taxRate)` 会把 DTO 里
//! **为 null 的字段也写成 null**，随后 MyBatis-Plus 的
//! `FieldStrategy.NOT_NULL` 又把 null 列跳过 —— 净效果是「保留旧值」。
//! 本实现**不依赖**这个「先置 null 再跳过」的巧合，而是显式做
//! 「非空字段覆盖旧值」（[`merge_update`]），得到相同净结果且意图更清晰。
//!
//! 但有一处**真实差异**：`region_name` 在 SQL 层可空，Java 的
//! copyProperties 会把 `regionName = null` 写进实体，被 NOT_NULL 跳过
//! ⇒ 保留旧值；本实现同样保留旧值。**两者一致**。
//!
//! ## ⚠️ 已确认的 Java 缺陷（本实现按「可观测行为」复刻）
//!
//! `create` 里 `rate` 有 `@NotNull`，但 `update` **复用同一个 DTO**，
//! 因此 `update` 也要求 `rate` 非空 —— 无法「只改备注」。
//! 本实现逐字符复刻该约束（`update` 同样校验 `rate` 必填与范围），
//! 这是 Java 契约的真实行为，不是 bug 修复。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::entities::setting::TaxRate;
use nextrade_core::models::payment::tax_rate::{
    default_order_clause, validate_create, validate_rate_range, validate_rate_scale, TaxRateDTO,
    TaxRateListQuery, TaxRateVO, MSG_ID_REQUIRED, MSG_RATE_REQUIRED, MSG_TAX_RATE_ID_REQUIRED,
    MSG_TAX_RATE_NOT_FOUND, YES,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::tax_rate::{TaxRateInsert, TaxRatePatch};
use nextrade_infrastructure::db::Repositories;

/// 税率管理域服务（挂载于 `AdminState.tax_rate`）。
pub struct TaxRateService {
    repos: Arc<Repositories>,
}

impl TaxRateService {
    /// 构造（与 area_code / order_admin / admin_user 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 税率分页（对应 Java `TaxRateServiceImpl#list`）。
    ///
    /// 返回 `PageResult<TaxRateVO>`（Java 返回 MyBatis-Plus `Page<TaxRateVO>`，
    /// Rust 统一使用项目既有 `PageResult`，与 product_manage/002/075 同一约定）。
    pub async fn list(&self, q: &TaxRateListQuery) -> Result<PageResult<TaxRateVO>> {
        // Java 在 service 里做字符串→时间戳转换（`StringUtils.dateToTimestampExample`）。
        // 转换失败时 Java 抛 DateTimeParseException（HTTP 500）；本实现返回可
        // 诊断的 400（与项目内其它「非法日期入参」的处理一致）。
        let start_ts = match q.effective_start_time() {
            Some(s) => Some(parse_date_to_timestamp_secs(s)?),
            None => None,
        };
        let end_ts = match q.effective_end_time() {
            Some(s) => Some(parse_date_to_timestamp_secs(s)?),
            None => None,
        };

        // 排序二选一：Java 是 `if (isNotBlank(sortField)) {...} else {...}`。
        let order_clause: &str = if q.uses_default_order() {
            default_order_clause()
        } else {
            &q.order_clause()
        };

        let (total, rows) = self
            .repos.payment.tax_rate
            .paginate(q, start_ts, end_ts, order_clause)
            .await?;
        let list: Vec<TaxRateVO> = rows.iter().map(TaxRateVO::from).collect();
        Ok(PageResult::new(total, list, &q.page_query()))
    }

    /// 税率详情（对应 Java `TaxRateServiceImpl#detail`）。
    ///
    /// Java 记录不存在时抛 `GlobalException("税率不存在")` ⇒ 本实现 404 同文案。
    pub async fn detail(&self, tax_rate_id: i32) -> Result<TaxRateVO> {
        let row = self
            .repos.payment.tax_rate
            .find_by_id(tax_rate_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_TAX_RATE_NOT_FOUND.to_string()))?;
        Ok(TaxRateVO::from(&row))
    }

    // ————————————————————— 写入 —————————————————————

    /// 新增税率（对应 Java `TaxRateServiceImpl#create`）。
    ///
    /// 步骤与 Java 逐条对齐：
    /// 1. `@Valid` 校验（`regionId` / `rate` 必填，`rate` ∈ [0, 100]）；
    /// 2. `regionName` blank 且 `regionId != null` → 用 `regionId` 回查
    ///    `region` 表补全名称（查不到则保持 blank，与 Java 一致）；
    /// 3. `isActive` 为空 → 默认 [`YES`]；
    /// 4. `createTime` / `updateTime` = 当前秒级时间戳；
    /// 5. 入库，返回自增主键（Java 返回 `boolean`，此处回传主键便于前端跳转，
    ///    与 shop_manage/017/075 的处理方式一致）。
    pub async fn create(&self, dto: &TaxRateDTO) -> Result<i32> {
        // 1) 入参校验（对照 Java `@Valid @RequestBody TaxRateDTO`）
        let rate = validate_create(dto).map_err(Error::Validation)?;
        let region_id = dto
            .region_id
            .ok_or_else(|| Error::Validation("国家和地区ID不能为空".to_string()))?;

        // 2) regionName 兜底：Java `StrUtil.isBlank(dto.getRegionName()) && regionId != null`
        let region_name = match nextrade_core::models::payment::tax_rate::non_blank(dto.region_name.as_deref())
        {
            Some(name) => Some(name.to_string()),
            None => self.lookup_region_name(region_id).await?,
        };

        // 3) `isActive == null` → Constants.YES
        let is_active = dto.is_active.unwrap_or(YES);

        // 4) 双时间戳同一个「当前秒」
        //    ⚠️ Java 两次独立调用 `getCurrentTime()`，理论上可能跨秒；
        //    本实现取一次，保证两列一致（见 `uncertain_points`）。
        let now = now_secs();

        let insert = TaxRateInsert {
            region_id,
            rate,
            is_active,
            region_name,
            remark: dto.remark.clone(),
            create_time: Some(now),
            update_time: Some(now),
        };
        self.repos.payment.tax_rate.insert(&insert).await
    }

    /// 更新税率（对应 Java `TaxRateServiceImpl#update`）。
    ///
    /// ⚠️ 校验顺序严格复刻 Java：**先判 `taxRateId` 空**，
    /// 再查记录是否存在，最后做 `@Valid` 字段校验。
    /// 这个顺序对错误码有影响（缺 id 时是 `"税率ID不能为空"` 而不是
    /// 字段校验文案），因此不能改。
    pub async fn update(&self, dto: &TaxRateDTO) -> Result<u64> {
        // ① Java 第一条语句：`if (dto.getTaxRateId() == null) throw ...`
        let id = dto
            .tax_rate_id
            .ok_or_else(|| Error::Validation(MSG_TAX_RATE_ID_REQUIRED.to_string()))?;

        // ② `getById` 取旧行；不存在 → "税率不存在"
        let old = self
            .repos.payment.tax_rate
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_TAX_RATE_NOT_FOUND.to_string()))?;

        // ③ `@Valid` 字段校验（update 与 create 共用 DTO ⇒ rate 同样必填）
        let rate = dto.rate.ok_or_else(|| Error::Validation(MSG_RATE_REQUIRED.to_string()))?;
        validate_rate_range(rate).map_err(Error::Validation)?;
        validate_rate_scale(rate).map_err(Error::Validation)?;

        let old_region_id = old.region_id;

        // ④ `BeanUtils.copyProperties(dto, taxRate)` 的净语义：
        //    DTO 非空字段覆盖旧值，未传字段保留旧值。
        let merged = merge_update(&old, dto, rate);

        // ⑤ `dto.regionId != null && !dto.regionId.equals(oldRegionId)`
        //    → 回查 region 表刷新 regionName
        //
        //    注意：条件用的是**请求值**与**旧值**比较，不是与合并结果比较。
        let region_name = match dto.region_id {
            Some(new_region_id) if new_region_id != old_region_id => {
                // 查不到时保持 copyProperties 后的结果（与 Java 一致：region == null 就不 set）
                self.lookup_region_name(new_region_id)
                    .await?
                    .or(merged.region_name.clone())
            }
            // regionId 未变（或未传）⇒ 保留请求里的 regionName（copyProperties 的结果）
            _ => merged.region_name.clone(),
        };

        // ⑥ `updateTime` = 当前；`createTime` 保持旧值不动
        let patch = TaxRatePatch {
            region_id: Some(Some(merged.region_id)),
            region_name: Some(region_name),
            rate: Some(Some(merged.rate)),
            is_active: Some(Some(merged.is_active)),
            remark: Some(merged.remark.clone()),
            create_time: Some(old.create_time),
            update_time: Some(Some(now_secs())),
        };
        self.repos.payment.tax_rate.update(id, &patch).await
    }

    /// 删除税率（对应 Java `TaxRateServiceImpl#del` → `removeById`）。
    ///
    /// 硬删除（表无软删列）。Java 的 `removeById` 返回 `boolean`，
    /// 本实现同样返回「是否删掉了行」。
    pub async fn del(&self, id: i32) -> Result<bool> {
        self.repos.payment.tax_rate.delete(id).await
    }

    // ————————————————————— 内部辅助 —————————————————————

    /// 按 `region_id` 回查 `region` 表的名称
    /// （对应 Java `regionService.getById(regionId).getRegionName()`）。
    ///
    /// 记录不存在或 `region_name` 为 NULL 时返回 `Ok(None)` —— 与 Java
    /// 「`region == null` 就不 `setRegionName`」的可观测行为一致，
    /// **不**报错（Java 此处不抛异常）。
    async fn lookup_region_name(&self, region_id: i32) -> Result<Option<String>> {
        self.repos.payment.tax_rate.find_region_name(region_id).await
    }
}

/// 合并后的可写字段快照（`BeanUtils.copyProperties(dto, old)` 的净结果）。
struct MergedTaxRate {
    region_id: i32,
    region_name: Option<String>,
    rate: Decimal,
    is_active: i32,
    remark: Option<String>,
}

/// 复刻 Java `BeanUtils.copyProperties(dto, taxRate)` 的净语义。
///
/// Spring 的 `copyProperties` 会把**源属性为 null 的字段也写成 null**，
/// 但随后 MyBatis-Plus 的 `FieldStrategy.NOT_NULL` 会跳过为 null 的列
/// ⇒ 净效果是「DTO 非空字段覆盖旧值，未传字段保留旧值」。
///
/// 本函数直接表达该净效果，不依赖两次取反的巧合。
///
/// `rate` 单独作为参数传入：它已在上游按「必填」校验过，
/// 因此是裸 `Decimal`（Java 侧同理：`@NotNull` 保证非空）。
fn merge_update(old: &TaxRate, dto: &TaxRateDTO, rate: Decimal) -> MergedTaxRate {
    MergedTaxRate {
        region_id: dto.region_id.unwrap_or(old.region_id),
        region_name: dto.region_name.clone().or_else(|| old.region_name.clone()),
        rate,
        is_active: dto.is_active.unwrap_or(old.is_active),
        remark: dto.remark.clone().or_else(|| old.remark.clone()),
    }
}

/// 当前**秒级**时间戳（Java `StringUtils.getCurrentTime()`
/// = `System.currentTimeMillis() / 1000`）。
#[must_use]
pub fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 把 Java `StringUtils.dateToTimestampExample(String)` 支持的日期串
/// 转为**秒级**时间戳。
///
/// 逐条对照 Java 的补全规则：
/// 1. 串里**没有** `:` → 直接拼 `" 00:00:00"`（Java 用的是未 trim 的原串）；
/// 2. 形如 `yyyy-MM-dd HH:mm`（正则 `\d{4}-\d{2}-\d{2} \d{2}:\d{2}`）→ 补 `":00"`；
/// 3. 解析格式固定 `yyyy-MM-dd HH:mm:ss`；
/// 4. 结果 = 本地时间按**东八区**换算的 epoch 秒
///    （Java 用 `ZoneId.systemDefault()`；本项目部署时区为
///    `Asia/Shanghai`，见 `uncertain_points`）。
///
/// 解析失败返回 [`Error::Validation`]（Java 抛 `DateTimeParseException` → 500；
/// 本实现给 400，更可诊断且不影响正常路径）。
pub fn parse_date_to_timestamp_secs(input: &str) -> Result<i64> {
    let padded = if !input.contains(':') {
        // Java: `dateTimeString.trim() + " 00:00:00"`（无冒号分支会 trim）
        format!("{} 00:00:00", input.trim())
    } else if is_yyyy_mm_dd_hh_mm(input.trim()) {
        format!("{}:00", input.trim())
    } else {
        input.trim().to_string()
    };

    parse_ymd_hms_secs(&padded).ok_or_else(|| {
        Error::Validation(format!(
            "时间格式不正确（应为 yyyy-MM-dd 或 yyyy-MM-dd HH:mm:ss）: {input}"
        ))
    })
}

/// 正则 `\d{4}-\d{2}-\d{2} \d{2}:\d{2}` 的等价判断（不引入 regex 依赖）。
fn is_yyyy_mm_dd_hh_mm(s: &str) -> bool {
    if s.len() != 16 {
        return false;
    }
    let b = s.as_bytes();
    let digit = |i: usize| b[i].is_ascii_digit();
    digit(0)
        && digit(1)
        && digit(2)
        && digit(3)
        && b[4] == b'-'
        && digit(5)
        && digit(6)
        && b[7] == b'-'
        && digit(8)
        && digit(9)
        && b[10] == b' '
        && digit(11)
        && digit(12)
        && b[13] == b':'
        && digit(14)
        && digit(15)
}

/// 解析 `yyyy-MM-dd HH:mm:ss`（东八区）为 epoch 秒。
///
/// 手写解析而不用 `chrono::NaiveDateTime::parse_from_str` 是为了：
/// 1. 不新增依赖（workspace 未启用 chrono 的 parse 特性时也能编译）；
/// 2. 明确拒绝非法日期（如 `2024-02-30`），与 Java 的严格解析一致
///    （`DateTimeFormatter` 默认 SMART resolver 会把 `02-30` 调整为
///    `02-29`，见 `uncertain_points` 的差异说明）。
fn parse_ymd_hms_secs(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 19 {
        return None;
    }
    let num = |from: usize, len: usize| -> Option<i64> {
        let slice = s.get(from..from + len)?;
        if !slice.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        slice.parse::<i64>().ok()
    };
    if b[4] != b'-' || b[7] != b'-' || b[10] != b' ' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let (year, month, day) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (hour, minute, second) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    if day < 1 || day > days_in_month(year, month) {
        return None;
    }

    // 东八区：UTC 秒 = 本地秒 - 8h
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour * 3600 + minute * 60 + second;
    Some(secs - 8 * 3600)
}

/// 某年某月的天数（含闰年规则）。
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// 闰年判定（公历规则）。
fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// `(year, month, day)` → 1970-01-01 起的天数（Howard Hinnant 算法）。
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试常量必须是合法 Decimal")
    }

    fn old_tax_rate() -> TaxRate {
        TaxRate {
            tax_rate_id: 9,
            region_id: 1,
            region_name: Some("旧地区".into()),
            rate: dec("10.0000"),
            is_active: 1,
            remark: Some("旧备注".into()),
            create_time: Some(1_600_000_000),
            update_time: Some(1_600_000_000),
        }
    }

    // ————————————— 日期解析（对照 StringUtils.dateToTimestampExample） —————————————

    /// 纯日期补 `00:00:00`；东八区当日零点 = 前一天 16:00 UTC。
    #[test]
    fn parse_plain_date_appends_midnight() {
        // 2024-01-02 00:00:00 +08:00 = 2024-01-01 16:00:00 UTC = 1704124800
        assert_eq!(parse_date_to_timestamp_secs("2024-01-02").unwrap(), 1_704_124_800);
        // 前后空白也应被容忍（Java 在无冒号分支 trim）
        assert_eq!(
            parse_date_to_timestamp_secs("  2024-01-02  ").unwrap(),
            1_704_124_800
        );
    }

    /// `yyyy-MM-dd HH:mm` 补秒。
    #[test]
    fn parse_minute_precision_appends_seconds() {
        // 2024-01-02 03:04:00 +08:00 = 2024-01-01 19:04:00 UTC = 1704135840
        assert_eq!(
            parse_date_to_timestamp_secs("2024-01-02 03:04").unwrap(),
            1_704_135_840
        );
    }

    /// `yyyy-MM-dd HH:mm:ss` 原样解析。
    #[test]
    fn parse_full_datetime() {
        // 2024-01-02 03:04:05 +08:00 = 2024-01-01 19:04:05 UTC = 1704135845
        assert_eq!(
            parse_date_to_timestamp_secs("2024-01-02 03:04:05").unwrap(),
            1_704_135_845
        );
    }

    /// 非法输入必须被拒绝（而不是静默算出一个错时间）。
    #[test]
    fn parse_rejects_invalid_dates() {
        assert!(parse_date_to_timestamp_secs("2024-02-30").is_err());
        assert!(parse_date_to_timestamp_secs("2024-13-01").is_err());
        assert!(parse_date_to_timestamp_secs("2023-02-29").is_err());
        assert!(parse_date_to_timestamp_secs("2024-01-02 25:00:00").is_err());
        assert!(parse_date_to_timestamp_secs("not-a-date").is_err());
        assert!(parse_date_to_timestamp_secs("").is_err());
    }

    /// 闰年 2 月 29 日必须合法。
    #[test]
    fn parse_accepts_leap_day() {
        assert!(parse_date_to_timestamp_secs("2024-02-29").is_ok());
        assert!(parse_date_to_timestamp_secs("2023-02-28").is_ok());
        assert!(parse_date_to_timestamp_secs("2000-02-29").is_ok());
        // 1900 不是闰年
        assert!(parse_date_to_timestamp_secs("1900-02-29").is_err());
    }

    /// 日期解析与时间戳格式化必须互为逆运算（同一 UTC+8 口径）。
    #[test]
    fn parse_and_format_are_inverse() {
        let ts = parse_date_to_timestamp_secs("2024-01-02 03:04:05").unwrap();
        assert_eq!(
            nextrade_core::models::payment::tax_rate::format_timestamp_secs(Some(ts)),
            "2024-01-02 03:04:05"
        );
    }

    // ————————————— update 的 copyProperties 净语义 —————————————

    /// DTO 非空字段覆盖旧值；未传字段保留旧值。
    #[test]
    fn merge_update_keeps_old_values_for_absent_fields() {
        let old = old_tax_rate();
        let dto = TaxRateDTO {
            tax_rate_id: Some(9),
            region_id: None,
            region_name: None,
            rate: Some(dec("20.0000")),
            is_active: None,
            remark: None,
        };
        let m = merge_update(&old, &dto, dec("20.0000"));
        assert_eq!(m.region_id, 1); // 保留旧值
        assert_eq!(m.region_name.as_deref(), Some("旧地区")); // 保留旧值
        assert_eq!(m.rate, dec("20.0000")); // 覆盖
        assert_eq!(m.is_active, 1); // 保留旧值
        assert_eq!(m.remark.as_deref(), Some("旧备注")); // 保留旧值
    }

    /// DTO 显式传值（含 0 与空串）必须覆盖旧值 —— 不能按「空就跳过」简化。
    #[test]
    fn merge_update_overrides_with_zero_and_empty() {
        let old = old_tax_rate();
        let dto = TaxRateDTO {
            tax_rate_id: Some(9),
            region_id: Some(5),
            region_name: Some(String::new()),
            rate: Some(dec("0")),
            is_active: Some(0),
            remark: Some(String::new()),
        };
        let m = merge_update(&old, &dto, dec("0"));
        assert_eq!(m.region_id, 5);
        assert_eq!(m.region_name.as_deref(), Some("")); // 空串要写入
        assert_eq!(m.rate, dec("0"));
        assert_eq!(m.is_active, 0); // 0 要写入
        assert_eq!(m.remark.as_deref(), Some(""));
    }

    // ————————————— 数学校验 —————————————

    /// 秒级时间戳单调且接近系统时钟。
    #[test]
    fn now_secs_is_in_seconds() {
        let now = now_secs();
        // 2020-01-01 ~ 2100-01-01 的秒级范围（毫秒会远大于此上界）
        assert!(now > 1_577_836_800, "now={now}");
        assert!(now < 4_102_444_800, "now={now}");
    }

    /// 天数/闰年辅助函数的基本正确性。
    #[test]
    fn civil_date_helpers() {
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(2024));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        // 1970-01-01 应为第 0 天
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        // 1970-01-02 应为第 1 天
        assert_eq!(days_from_civil(1970, 1, 2), 1);
    }
}