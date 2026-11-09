//! 供应商端保证金 域服务（merchant::vendor_deposit_shop）。
//!
//! 对应原 Java `com.nextrade.service.deposit.impl.VendorDepositShopServiceImpl`
//! （335 行）+ `VendorDepositShopController`（79 行）的 5 条链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET /deposit/vendor/overview` | `VendorDepositShopController.overview` | [`VendorDepositShopService::overview`] |
//! | `GET /deposit/vendor/logList` | `VendorDepositShopController.logList` | [`VendorDepositShopService::log_list`] |
//! | `GET /deposit/vendor/frozenDetailList` | `VendorDepositShopController.frozenDetailList` | [`VendorDepositShopService::frozen_detail_list`] |
//! | `GET /deposit/vendor/warning` | `VendorDepositShopController.warning` | [`VendorDepositShopService::warning`] |
//! | `GET /deposit/vendor/protocol` | `VendorDepositShopController.protocol` | [`VendorDepositShopService::protocol`] |
//!
//! 本域与店铺端 `DepositShopServiceImpl` 平行，**唯一差别**是持有者维度：
//! `owner_type = 2`（`DepositOwnerType.SUPPLIER`）、`owner_id = vendorId`，
//! 而 `vendorId` 由控制器从供应商端鉴权头解析（Java `HeaderUtils.getVendorId()`），
//! **不接收外部传入**，避免越权查询他供应商保证金。
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### `overview`
//! - 三笔金额全部**只读**获取：`computeRequired` 一次、`getPaid` 一次，
//!   再按同一组公式派生：
//!   `frozen = min(paid, required)`、`withdrawable = max(0, paid - required)`、
//!   `pending = max(0, required - paid)`。
//! - `computeRequired` 在本域的净效果是（Java `DepositAccountServiceImpl`）：
//!   1. `deposit_enable != 1` → **0**（功能关闭时全部归零）；
//!   2. `required_manual != null` → 直接返回该值（手动覆盖优先，
//!      **不受**收取激活门限约束）；
//!   3. 无账户 或 `collect_activated != 1` → **0**（首单收取语义）；
//!   4. 否则按规则匹配取最高（[`VendorDepositShopService::max_required`]）。
//! - `pendingWithdraw` = 待审核（`status = 0`）退还/提现申请金额合计，
//!   仅展示，**不参与**可提现扣减。
//! - `status` 无账户时默认 1、`penaltyStatus` 默认 0、`supplementDueTime` 默认 0。
//! - `hasProtocol = enable == true && content 非 blank`。
//!
//! ### `logList` / `frozenDetailList`
//! - 排序固定 `shop_deposit_log_id DESC`（Java 未使用 `BasePage.sortField`）。
//! - `tradeNo` **非纯数字时静默忽略**（Java `matches("\\d+")`）。
//! - `frozenDetailList` **只**查 `type = 4`（平台扣除），并忽略 `param.type`。
//! - `logList` 会回填来源/去向账户（Java `DepositShopLogEnricher`）。
//!   `frozenDetailList` **不**回填（Java 未调用 enricher）。
//!
//! ### `warning`
//! - `needWarning = pending > 0`；`bufferDays` 收取缓冲优先、否则补缴缓冲。
//! - 文案按「超期未缴足 > 补缴差额 > 收取规则」单条匹配，
//!   两缓冲均未启动时走「立即补缴」兜底。
//!
//! ### `protocol`
//! - 只读两个配置键，配置缺失/异常一律按 `null` 处理（Java `readConfig`
//!   用 `try/catch` 吞掉 `GlobalException(CONFIG_NOT_EXIST)`）。
//! - 本接口**与 vendorId 无关**（协议是平台级配置，店铺端与供应商端共用）。
//!
//! ## ⚠️ 与 Java 的已知差异（均列入 `uncertain_points`）
//!
//! 1. `required_snapshot` / `collect_activated` 等列在 Java 实体里是 `Long`，
//!    但库列是 `int(11) UNSIGNED` / `tinyint`，Rust 侧按 schema 用 `i32`。
//! 2. Java `Integer.valueOf(tradeNoStr)` 对超 `int` 范围的纯数字串会抛
//!    `NumberFormatException`（HTTP 500）；本实现按 `i64` 解析，超出即忽略过滤。
//! 3. Java `JSON.parseObject(payeeSnapshot)` 对非法 JSON 会抛异常（HTTP 500）；
//!    本实现解析失败即回退到 `payee_account_type` 文案。
//! 4. `TigUtils.handelTime` 用的是 **JVM 默认时区**，本实现用 `chrono::Local`
//!    复刻同一口径（与 im_servant 一致），未硬编码东八区。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::models::merchant::vendor_deposit_shop::{
    build_warning_messages, frozen_of, handel_time, is_blank, log_type_text, max0,
    normalize_log_type, parse_bool, pending_of, resolve_buffer_days, resolve_source_dest,
    resolve_status_text, withdrawable_of, DepositShopFrozenDetailVO, DepositShopLogQuery,
    DepositShopLogVO, DepositShopOverviewVO, DepositShopProtocolVO, DepositShopWarningVO,
    CONFIG_DEPOSIT_ENABLE, CONFIG_PROTOCOL_CONTENT, CONFIG_PROTOCOL_ENABLE,
    DEFAULT_ACCOUNT_STATUS, ENTITY_VENDOR, LOG_TYPE_DEDUCT, LOG_TYPE_RECHARGE, LOG_TYPE_REFUND,
    LOG_TYPE_WITHDRAW, OWNER_TYPE_SUPPLIER,
};
use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 类目就近匹配的最大溯源层级（Java `DepositRuleMatcher.MAX_CATEGORY_LEVEL`）。
const MAX_CATEGORY_LEVEL: usize = 3;

/// 供应商端保证金域服务（挂载于 `AdminState.vendor_deposit_shop`）。
pub struct VendorDepositShopService {
    repos: Arc<Repositories>,
}

impl VendorDepositShopService {
    /// 构造（与 tax_rate 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::merchant::vendor_deposit_shop::VendorDepositShopRepository {
        &self.repos.merchant.vendor_deposit_shop
    }

    // ————————————————————— 总览 —————————————————————

    /// 保证金总览（对应 Java `VendorDepositShopServiceImpl#getOverview`）。
    ///
    /// 计算顺序与 Java 逐条一致（见模块头「迁移要点」）。
    pub async fn overview(&self, vendor_id: i32) -> Result<DepositShopOverviewVO> {
        let required = self.compute_required(vendor_id).await?;
        let paid = self.get_paid(vendor_id).await?;
        let frozen = frozen_of(paid, required);
        let withdrawable = withdrawable_of(paid, required);
        let pending = pending_of(required, paid);
        let pending_withdraw = self.pending_withdraw(vendor_id).await?;

        let account = self.repo().find_account(OWNER_TYPE_SUPPLIER, vendor_id).await?;
        let status = account
            .as_ref()
            .map_or(DEFAULT_ACCOUNT_STATUS, |a| a.status);
        let penalty_status = account.as_ref().map_or(0, |a| a.penalty_status);
        let supplement_due_time = account.as_ref().map_or(0, |a| i64::from(a.supplement_due_time));

        let protocol = self.protocol().await?;
        let has_protocol = protocol.enable && !is_blank(protocol.content.as_deref());

        Ok(DepositShopOverviewVO {
            required,
            paid,
            pending,
            withdrawable,
            pending_withdraw,
            frozen,
            status,
            penalty_status,
            status_text: resolve_status_text(penalty_status, pending).to_string(),
            supplement_due_time,
            supplement_due_time_text: handel_time(Some(supplement_due_time)),
            has_protocol,
        })
    }

    // ————————————————————— 流水 / 冻结明细 —————————————————————

    /// 供应商端交易流水（对应 Java `VendorDepositShopServiceImpl#logList`）。
    ///
    /// 除分页与过滤外，还会按 `relatedId` 批量回填来源/去向账户
    /// （Java `DepositShopLogEnricher#fillSourceDestAccounts`）。
    pub async fn log_list(
        &self,
        vendor_id: i32,
        q: &DepositShopLogQuery,
    ) -> Result<PageResult<DepositShopLogVO>> {
        let (total, rows) = self.repo().paginate_logs(vendor_id, q, false).await?;

        // 收集需要回查的关联单 id（与 Java 完全一致：只认 relatedId > 0 且
        // type 属于充值 / 提现 / 退还三类）。
        let mut recharge_ids: Vec<i32> = Vec::new();
        let mut refund_ids: Vec<i32> = Vec::new();
        for row in &rows {
            if row.related_id <= 0 {
                continue;
            }
            match row.r#type {
                LOG_TYPE_RECHARGE => recharge_ids.push(row.related_id),
                LOG_TYPE_WITHDRAW | LOG_TYPE_REFUND => refund_ids.push(row.related_id),
                _ => {}
            }
        }
        recharge_ids.sort_unstable();
        recharge_ids.dedup();
        refund_ids.sort_unstable();
        refund_ids.dedup();

        let recharge_map: HashMap<i32, (i32, Option<String>)> = self
            .repo()
            .find_recharges_by_ids(&recharge_ids)
            .await?
            .into_iter()
            .map(|(id, payee_type, snapshot)| (id, (payee_type, snapshot)))
            .collect();
        let refund_map: HashMap<i32, Option<String>> = self
            .repo()
            .find_refunds_by_ids(&refund_ids)
            .await?
            .into_iter()
            .map(|(id, payee_info)| (id, payee_info))
            .collect();

        let list: Vec<DepositShopLogVO> = rows
            .iter()
            .map(|row| {
                let recharge = recharge_map
                    .get(&row.related_id)
                    .map(|(t, s)| (*t, s.as_deref()));
                let refund_payee = refund_map
                    .get(&row.related_id)
                    .and_then(|v| v.as_deref());
                let (source_account, dest_account) =
                    resolve_source_dest(row.r#type, recharge, refund_payee);
                DepositShopLogVO {
                    shop_deposit_log_id: row.shop_deposit_log_id,
                    r#type: normalize_log_type(row.r#type),
                    type_text: log_type_text(row.r#type).to_string(),
                    change_amount: row.change_amount,
                    before_paid: row.before_paid,
                    after_paid: row.after_paid,
                    order_id: row.order_id,
                    related_id: row.related_id,
                    remark: row.remark.clone(),
                    add_time: i64::from(row.add_time),
                    add_time_text: handel_time(Some(i64::from(row.add_time))),
                    source_account,
                    dest_account,
                }
            })
            .collect();

        Ok(PageResult::new(total, list, &q.page_query()))
    }

    /// 冻结明细（平台扣款记录）
    /// （对应 Java `VendorDepositShopServiceImpl#frozenDetailList`）。
    ///
    /// ⚠️ Java 的 VO 注释（CR-20260616-135500）说「扩展为全部资金类型」，
    /// 但供应商端实现**只**查 `type = 4`（`DEDUCT`），因此 `type` / `typeText`
    /// 恒为 4 / "平台扣除"。本实现按实际行为复刻。
    ///
    /// `deductAmount` = `changeAmount == null ? 0 : abs(changeAmount)`；
    /// 库列 `change_amount` 是 `NOT NULL`，因此等价于 `abs(change_amount)`。
    pub async fn frozen_detail_list(
        &self,
        vendor_id: i32,
        q: &DepositShopLogQuery,
    ) -> Result<PageResult<DepositShopFrozenDetailVO>> {
        let (total, rows) = self.repo().paginate_logs(vendor_id, q, true).await?;
        let list: Vec<DepositShopFrozenDetailVO> = rows
            .iter()
            .map(|row| DepositShopFrozenDetailVO {
                shop_deposit_log_id: row.shop_deposit_log_id,
                r#type: normalize_log_type(row.r#type),
                type_text: log_type_text(row.r#type).to_string(),
                deduct_amount: row.change_amount.abs(),
                change_amount: row.change_amount,
                reason: row.remark.clone(),
                order_id: row.order_id,
                related_id: row.related_id,
                add_time: i64::from(row.add_time),
                add_time_text: handel_time(Some(i64::from(row.add_time))),
            })
            .collect();
        Ok(PageResult::new(total, list, &q.page_query()))
    }

    // ————————————————————— 预警 —————————————————————

    /// 首页保证金预警横幅
    /// （对应 Java `VendorDepositShopServiceImpl#getWarning`）。
    pub async fn warning(&self, vendor_id: i32) -> Result<DepositShopWarningVO> {
        let pending = self.pending_amount(vendor_id).await?;
        let account = self.repo().find_account(OWNER_TYPE_SUPPLIER, vendor_id).await?;
        let supplement_due_time = account.as_ref().map_or(0, |a| i64::from(a.supplement_due_time));
        let collect_due_time = account.as_ref().map_or(0, |a| i64::from(a.collect_due_time));

        let need_warning = pending > Decimal::ZERO;
        let now = now_secs();
        let buffer_days = resolve_buffer_days(collect_due_time, supplement_due_time, now);
        let messages = build_warning_messages(
            need_warning,
            collect_due_time,
            supplement_due_time,
            ENTITY_VENDOR,
            now,
        );

        Ok(DepositShopWarningVO {
            need_warning,
            pending_amount: pending,
            supplement_due_time,
            supplement_due_time_text: handel_time(Some(supplement_due_time)),
            buffer_days,
            message: messages.join("\n"),
            messages,
        })
    }

    // ————————————————————— 协议 —————————————————————

    /// 保证金协议（对应 Java `VendorDepositShopServiceImpl#getProtocol`）。
    ///
    /// 与 vendorId 无关（平台级配置，店铺端与供应商端共用同一组键）。
    pub async fn protocol(&self) -> Result<DepositShopProtocolVO> {
        let enable_val = self.read_config(CONFIG_PROTOCOL_ENABLE).await?;
        let content = self.read_config(CONFIG_PROTOCOL_CONTENT).await?;
        Ok(DepositShopProtocolVO {
            enable: parse_bool(enable_val.as_deref()),
            content,
        })
    }

    // ————————————————————— 内部：只读账户口径 —————————————————————

    /// 待补缴（对应 Java `DepositAccountService#pending`）。
    async fn pending_amount(&self, vendor_id: i32) -> Result<Decimal> {
        let required = self.compute_required(vendor_id).await?;
        let paid = self.get_paid(vendor_id).await?;
        Ok(pending_of(required, paid))
    }

    /// 已缴（对应 Java `DepositAccountService#getPaid`）：
    /// 无账户或 `paid` 为空 → 0。
    ///
    /// 库列 `paid` 是 `NOT NULL`，故 `nullToZero` 只在「无账户」时生效。
    async fn get_paid(&self, vendor_id: i32) -> Result<Decimal> {
        Ok(self
            .repo()
            .find_account(OWNER_TYPE_SUPPLIER, vendor_id)
            .await?
            .map_or(Decimal::ZERO, |a| a.paid))
    }

    /// 应缴（对应 Java `DepositAccountService#computeRequired`）。
    ///
    /// 四个分支与 Java 逐条对齐：
    /// 1. 保证金功能关闭（`deposit_enable != 1`，配置缺失视为关闭）→ 0；
    /// 2. `required_manual != null` → 该值（手动覆盖优先，不受激活门限约束）；
    /// 3. 无账户 或 `collect_activated != 1` → 0（首单收取语义）；
    /// 4. 否则规则匹配取最高。
    pub async fn compute_required(&self, vendor_id: i32) -> Result<Decimal> {
        if !self.is_deposit_enabled().await? {
            return Ok(Decimal::ZERO);
        }
        let account = self.repo().find_account(OWNER_TYPE_SUPPLIER, vendor_id).await?;
        let Some(account) = account else {
            return Ok(Decimal::ZERO);
        };
        if let Some(manual) = account.required_manual {
            return Ok(manual);
        }
        if account.collect_activated != 1 {
            return Ok(Decimal::ZERO);
        }
        self.max_required(vendor_id).await
    }

    /// 保证金功能是否开启（Java `DepositAccountServiceImpl#isDepositEnabled`：
    /// `deposit_enable == 1`；配置缺失/非 1 视为关闭）。
    ///
    /// Java `DepositSettingServiceImpl#detail` 用
    /// `parseInt(values.get(KEY_ENABLE), 0)`：blank → 默认 **0**、
    /// 非法数字 → 默认 0、`"1"` → 1。本实现按同一口径解析。
    async fn is_deposit_enabled(&self) -> Result<bool> {
        let raw = self.read_config(CONFIG_DEPOSIT_ENABLE).await?;
        Ok(parse_int(raw.as_deref(), 0) == 1)
    }

    /// 只读读取配置键值（对应 Java `VendorDepositShopServiceImpl#readConfig`）。
    ///
    /// Java 用 `try/catch` 吞掉 `GlobalException(CONFIG_NOT_EXIST)` 并返回
    /// `null`，因此本方法把「配置不存在」也归一到 `Ok(None)`。
    async fn read_config(&self, biz_code: &str) -> Result<Option<String>> {
        self.repo().find_config(biz_code).await
    }

    /// 自动应缴：owner 全部在售商品按三级就近命中规则取最高额
    /// （对应 Java `DepositRuleMatcher#maxRequired`）。
    ///
    /// 逐条对齐 Java：
    /// 1. 取供应商在售商品类目 id（`selectVendorOnSaleCategoryIds`），空 → 0；
    /// 2. 主体类型 = `vendor.type`（仅入驻供应商），解析不到 → 0；
    /// 3. `shopType` 直接用 `ownerType` 同码值（**2 = 供应商**，不是 1）；
    /// 4. 为每个叶子类目构建「自身 → 父 → 祖父」最多三级的就近链，
    ///    汇总候选类目集合；
    /// 5. 规则匹配取每个类目的最高金额；
    /// 6. 每个商品按链**从近到远**取首个命中类目，全店取**最高**。
    async fn max_required(&self, vendor_id: i32) -> Result<Decimal> {
        let leaf_category_ids = self.repo().vendor_on_sale_category_ids(vendor_id).await?;
        if leaf_category_ids.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let Some(merchant_type) = self.repo().find_vendor_type(vendor_id).await? else {
            return Ok(Decimal::ZERO);
        };
        let shop_type = OWNER_TYPE_SUPPLIER;

        // 构建就近链（保留插入顺序，与 Java LinkedHashMap 一致）。
        let mut chains: Vec<Vec<i32>> = Vec::new();
        let mut seen_leaves: Vec<i32> = Vec::new();
        let mut candidate_category_ids: Vec<i32> = Vec::new();
        // 类目父 id 缓存，避免同一类目重复查库。
        let mut parent_cache: HashMap<i32, Option<i32>> = HashMap::new();

        for leaf in leaf_category_ids {
            if leaf <= 0 || seen_leaves.contains(&leaf) {
                continue;
            }
            seen_leaves.push(leaf);
            let chain = self
                .build_category_chain(leaf, &mut parent_cache)
                .await?;
            for id in &chain {
                if !candidate_category_ids.contains(id) {
                    candidate_category_ids.push(*id);
                }
            }
            chains.push(chain);
        }
        if candidate_category_ids.is_empty() {
            return Ok(Decimal::ZERO);
        }

        let now = now_secs();
        let rows = self
            .repo()
            .match_max_by_categories(merchant_type, shop_type, &candidate_category_ids, now)
            .await?;
        if rows.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let hit_by_category: HashMap<i32, Decimal> = rows.into_iter().collect();

        let mut max: Option<Decimal> = None;
        for chain in &chains {
            for category_id in chain {
                if let Some(amount) = hit_by_category.get(category_id) {
                    if max.is_none_or(|m| *amount > m) {
                        max = Some(*amount);
                    }
                    break;
                }
            }
        }
        Ok(max.unwrap_or(Decimal::ZERO))
    }

    /// 构建类目就近溯源链：自身 → 父 → 祖父，最多
    /// [`MAX_CATEGORY_LEVEL`] 级（对应 Java
    /// `DepositRuleMatcher#buildCategoryChain`）。
    ///
    /// Java 在 `parentId == null || parentId <= 0 || chain.contains(parentId)`
    /// 时中断（自环保护），本实现同样处理。
    async fn build_category_chain(
        &self,
        category_id: i32,
        parent_cache: &mut HashMap<i32, Option<i32>>,
    ) -> Result<Vec<i32>> {
        let mut chain: Vec<i32> = Vec::with_capacity(MAX_CATEGORY_LEVEL);
        let mut current = category_id;
        for _ in 0..MAX_CATEGORY_LEVEL {
            if current <= 0 {
                break;
            }
            chain.push(current);
            let parent = match parent_cache.get(&current) {
                Some(v) => *v,
                None => {
                    let v = self.repo().find_category_parent(current).await?;
                    parent_cache.insert(current, v);
                    v
                }
            };
            match parent {
                Some(pid) if pid > 0 && !chain.contains(&pid) => current = pid,
                _ => break,
            }
        }
        Ok(chain)
    }

    /// 待审核提现金额（对应 Java `DepositRefundService#pendingRefundSum`）。
    ///
    /// ⚠️ 该 Java 方法在 `ownerId == null || ownerId <= 0` 时直接返回
    /// `BigDecimal.ZERO`；本实现由控制器保证 `vendor_id > 0`，故不重复判断。
    async fn pending_withdraw(&self, vendor_id: i32) -> Result<Decimal> {
        self.repo()
            .pending_refund_sum(OWNER_TYPE_SUPPLIER, vendor_id)
            .await
    }
}

/// 当前**秒级**时间戳（Java `System.currentTimeMillis() / 1000`）。
#[must_use]
pub fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 解析整型配置值（Java `DepositSettingServiceImpl#parseInt`）：
/// blank → `default`；非法数字 → `default`；否则 `trim` 后解析。
#[must_use]
pub fn parse_int(raw: Option<&str>, default: i32) -> i32 {
    match raw {
        Some(v) if !v.trim().is_empty() => v.trim().parse::<i32>().unwrap_or(default),
        _ => default,
    }
}

/// `max(0, value)` 的别名（保持与 Java 辅助函数同名，便于对照）。
#[must_use]
pub fn max_zero(value: Decimal) -> Decimal {
    max0(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::ToPrimitive;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试常量必须是合法 Decimal")
    }

    // ————————————— 配置解析（对照 DepositSettingServiceImpl#parseInt） —————————————

    #[test]
    fn parse_int_uses_default_for_blank_and_garbage() {
        assert_eq!(parse_int(None, 0), 0);
        assert_eq!(parse_int(Some(""), 0), 0);
        assert_eq!(parse_int(Some("   "), 0), 0);
        assert_eq!(parse_int(Some("abc"), 0), 0);
        assert_eq!(parse_int(Some("1"), 0), 1);
        assert_eq!(parse_int(Some(" 1 "), 0), 1);
        assert_eq!(parse_int(Some("0"), 9), 0);
        assert_eq!(parse_int(Some("7"), 0), 7);
        // 非 1 的其它整数都算「关闭」
        assert_ne!(parse_int(Some("2"), 0), 1);
    }

    // ————————————— 应缴分支（computeRequired 的四个短路） —————————————

    /// 用纯函数复刻 `computeRequired` 的判定链，逐分支断言。
    fn required_from(
        enabled: bool,
        has_account: bool,
        required_manual: Option<Decimal>,
        collect_activated: i32,
        rule_max: Decimal,
    ) -> Decimal {
        if !enabled {
            return Decimal::ZERO;
        }
        if !has_account {
            return Decimal::ZERO;
        }
        if let Some(manual) = required_manual {
            return manual;
        }
        if collect_activated != 1 {
            return Decimal::ZERO;
        }
        rule_max
    }

    #[test]
    fn required_is_zero_when_feature_disabled() {
        // 功能关闭时，即便有手动覆盖值也归零（Java 第一分支在最前）
        assert_eq!(
            required_from(false, true, Some(dec("500.00")), 1, dec("999.00")),
            Decimal::ZERO
        );
    }

    #[test]
    fn required_is_zero_without_account_or_before_activation() {
        assert_eq!(required_from(true, false, None, 0, dec("300.00")), Decimal::ZERO);
        assert_eq!(required_from(true, true, None, 0, dec("300.00")), Decimal::ZERO);
        // collect_activated 只认 1（Java `Integer.valueOf(1).equals(...)`）
        assert_eq!(required_from(true, true, None, 2, dec("300.00")), Decimal::ZERO);
    }

    #[test]
    fn required_manual_overrides_rule_and_ignores_activation() {
        // 手动覆盖优先，且**不受** collect_activated 门限约束
        assert_eq!(
            required_from(true, true, Some(dec("888.00")), 0, dec("100.00")),
            dec("888.00")
        );
        assert_eq!(
            required_from(true, true, Some(dec("888.00")), 1, dec("100.00")),
            dec("888.00")
        );
        // 手动值为 0 也要覆盖（`!= null` 判断，不是 `> 0`）
        assert_eq!(
            required_from(true, true, Some(Decimal::ZERO), 1, dec("100.00")),
            Decimal::ZERO
        );
    }

    #[test]
    fn required_falls_back_to_rule_max() {
        assert_eq!(
            required_from(true, true, None, 1, dec("300.00")),
            dec("300.00")
        );
    }

    // ————————————— 派生金额（overview） —————————————

    #[test]
    fn overview_derivation_matches_java_formulas() {
        // 应缴 300 / 已缴 120 → 待补缴 180、冻结 120、可提现 0
        let required = dec("300.00");
        let paid = dec("120.00");
        assert_eq!(frozen_of(paid, required), dec("120.00"));
        assert_eq!(withdrawable_of(paid, required), Decimal::ZERO);
        assert_eq!(pending_of(required, paid), dec("180.00"));
        assert_eq!(resolve_status_text(0, pending_of(required, paid)), "待补缴");

        // 应缴 100 / 已缴 250 → 待补缴 0、冻结 100、可提现 150
        let required = dec("100.00");
        let paid = dec("250.00");
        assert_eq!(frozen_of(paid, required), dec("100.00"));
        assert_eq!(withdrawable_of(paid, required), dec("150.00"));
        assert_eq!(pending_of(required, paid), Decimal::ZERO);
        assert_eq!(resolve_status_text(0, pending_of(required, paid)), "正常");
        // 有处罚时状态文案优先
        assert_eq!(resolve_status_text(1, pending_of(required, paid)), "已处罚");
    }

    // ————————————— 预警 —————————————

    #[test]
    fn warning_pending_drives_need_warning() {
        let required = dec("500.00");
        let paid = dec("200.00");
        let pending = pending_of(required, paid);
        assert!(pending > Decimal::ZERO);
        let now = 1_000_000_i64;
        let msgs = build_warning_messages(true, 0, 0, ENTITY_VENDOR, now);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], nextrade_core::models::merchant::vendor_deposit_shop::MSG_WARNING_EXPIRED);

        // 不需预警 → 空列表 + 空 message
        let msgs = build_warning_messages(false, now + 10, now + 20, ENTITY_VENDOR, now);
        assert!(msgs.is_empty());
        assert_eq!(msgs.join("\n"), "");
    }

    #[test]
    fn warning_buffer_days_uses_collect_then_supplement() {
        let now = 1_000_000_i64;
        assert_eq!(resolve_buffer_days(0, 0, now), 0);
        assert_eq!(
            resolve_buffer_days(now + 2 * 86_400, now + 9 * 86_400, now),
            2
        );
        assert_eq!(resolve_buffer_days(0, now + 3 * 86_400, now), 3);
    }

    // ————————————— 冻结明细的扣款金额 —————————————

    #[test]
    fn deduct_amount_is_absolute_of_change_amount() {
        assert_eq!(dec("-30.50").abs(), dec("30.50"));
        assert_eq!(dec("30.50").abs(), dec("30.50"));
        assert_eq!(Decimal::ZERO.abs(), Decimal::ZERO);
    }

    // ————————————— 流水类型归一（logList / frozenDetailList） —————————————

    #[test]
    fn log_vo_type_is_normalized_but_text_uses_raw_type() {
        // 存量 type=3：对外展示 2，文案「提现出账」
        assert_eq!(normalize_log_type(LOG_TYPE_REFUND), 2);
        assert_eq!(log_type_text(LOG_TYPE_REFUND), "提现出账");
        // 冻结明细恒为 4 / 平台扣除
        assert_eq!(normalize_log_type(LOG_TYPE_DEDUCT), 4);
        assert_eq!(log_type_text(LOG_TYPE_DEDUCT), "平台扣除");
    }

    // ————————————— 来源/去向回填（logList） —————————————

    #[test]
    fn log_list_enrichment_matches_enricher() {
        // 充值：来源=快照文案，去向=保证金余额
        let (src, dst) = resolve_source_dest(
            LOG_TYPE_RECHARGE,
            Some((1, Some(r#"{"accountType":2,"accountName":"甲","accountNo":"a1"}"#))),
            None,
        );
        assert_eq!(src.as_deref(), Some("支付宝/甲/a1"));
        assert_eq!(dst.as_deref(), Some("保证金余额"));

        // 提现：去向=退还单收款信息
        let (src, dst) = resolve_source_dest(LOG_TYPE_WITHDRAW, None, Some(" 张三 "));
        assert_eq!(src.as_deref(), Some("保证金余额"));
        assert_eq!(dst.as_deref(), Some("张三"));

        // 未命中关联单的充值：来源为空串（不是 null）
        let (src, dst) = resolve_source_dest(LOG_TYPE_RECHARGE, None, None);
        assert_eq!(src.as_deref(), Some(""));
        assert_eq!(dst.as_deref(), Some("保证金余额"));

        // 平台扣除不查关联单
        let (src, dst) = resolve_source_dest(LOG_TYPE_DEDUCT, None, None);
        assert_eq!(src.as_deref(), Some("保证金余额"));
        assert_eq!(dst.as_deref(), Some("平台账户"));
    }

    #[test]
    fn recharge_and_refund_format_helpers() {
        use nextrade_core::models::merchant::vendor_deposit_shop::{
            format_recharge_pay_account, format_refund_payee_account,
        };
        assert_eq!(
            format_recharge_pay_account(3, Some(r#"{"accountType":1,"accountNo":"6222"}"#)),
            "银行卡/6222"
        );
        assert_eq!(format_recharge_pay_account(3, None), "微信");
        assert_eq!(format_refund_payee_account(Some(" 支付宝 138 ")), "支付宝 138");
        assert_eq!(format_refund_payee_account(None), "");
    }
    // ————————————— 类目就近链 —————————————

    /// 复刻 `buildCategoryChain` 的纯逻辑（不查库版本），验证层级与自环保护。
    fn chain_from(parents: &HashMap<i32, Option<i32>>, start: i32) -> Vec<i32> {
        let mut chain: Vec<i32> = Vec::new();
        let mut current = start;
        for _ in 0..MAX_CATEGORY_LEVEL {
            if current <= 0 {
                break;
            }
            chain.push(current);
            match parents.get(&current).copied().flatten() {
                Some(pid) if pid > 0 && !chain.contains(&pid) => current = pid,
                _ => break,
            }
        }
        chain
    }

    #[test]
    fn category_chain_is_self_parent_grandparent() {
        let parents = HashMap::from([(30, Some(20)), (20, Some(10)), (10, Some(0))]);
        assert_eq!(chain_from(&parents, 30), vec![30, 20, 10]);
    }

    #[test]
    fn category_chain_stops_at_missing_or_root_or_self_loop() {
        // 父不存在（查不到行）
        let parents = HashMap::new();
        assert_eq!(chain_from(&parents, 30), vec![30]);
        // 父为 0（一级类目）
        let parents = HashMap::from([(30, Some(0))]);
        assert_eq!(chain_from(&parents, 30), vec![30]);
        // 自环
        let parents = HashMap::from([(30, Some(30))]);
        assert_eq!(chain_from(&parents, 30), vec![30]);
        // 超过三级只取三级
        let parents = HashMap::from([
            (40, Some(30)),
            (30, Some(20)),
            (20, Some(10)),
            (10, Some(1)),
        ]);
        assert_eq!(chain_from(&parents, 40), vec![40, 30, 20]);
    }

    /// 每个商品按链从近到远取首个命中类目，全店取最高。
    #[test]
    fn max_required_picks_nearest_hit_then_max() {
        let hit: HashMap<i32, Decimal> = HashMap::from([
            (30, dec("100.00")),
            (20, dec("300.00")),
            (10, dec("50.00")),
        ]);
        // 商品 A：链 [30, 20, 10] → 最近命中 30 → 100
        // 商品 B：链 [20, 10]     → 最近命中 20 → 300
        let chains: Vec<Vec<i32>> = vec![vec![30, 20, 10], vec![20, 10]];
        let mut max: Option<Decimal> = None;
        for chain in &chains {
            for cid in chain {
                if let Some(amount) = hit.get(cid) {
                    if max.is_none_or(|m| *amount > m) {
                        max = Some(*amount);
                    }
                    break;
                }
            }
        }
        assert_eq!(max, Some(dec("300.00")));

        // 无任何命中 → None（调用方回落 0）
        let empty: HashMap<i32, Decimal> = HashMap::new();
        let mut max: Option<Decimal> = None;
        for chain in &chains {
            for cid in chain {
                if let Some(amount) = empty.get(cid) {
                    if max.is_none_or(|m| *amount > m) {
                        max = Some(*amount);
                    }
                    break;
                }
            }
        }
        assert_eq!(max.unwrap_or(Decimal::ZERO), Decimal::ZERO);
    }

    // ————————————— 杂项 —————————————

    #[test]
    fn now_secs_is_seconds_not_millis() {
        let now = now_secs();
        assert!(now > 1_577_836_800, "now={now}");
        assert!(now < 4_102_444_800, "now={now}");
    }

    #[test]
    fn max_zero_alias_behaves_like_max0() {
        assert_eq!(max_zero(dec("-1.00")), Decimal::ZERO);
        assert_eq!(max_zero(dec("1.00")), dec("1.00"));
    }

    #[test]
    fn owner_type_is_supplier_code_2() {
        assert_eq!(OWNER_TYPE_SUPPLIER, 2);
        // 顺带确认 Decimal → i32 转换在测试里可用（防 prelude 导入被优化掉）
        assert_eq!(dec("2").to_i32(), Some(2));
    }
}