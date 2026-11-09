//! 店铺端保证金聚合服务（`merchant::deposit_shop`，5 个只读端点）。
//!
//! 对应原 Java `com.nextrade.service.deposit.impl.DepositShopServiceImpl`：
//! `getOverview` / `logList` / `frozenDetailList` / `getWarning` / `getProtocol`。
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! - `overview`：应缴（`computeRequired` 四分支：功能关闭 → 0、手动应缴优先、
//!   无账户/未激活收取 → 0、否则按「主体类型 × 持有者类型 × 在售类目三级就近 ×
//!   当前生效期」规则取最高）、实缴、冻结 `min(paid, required)`、
//!   可提现 `max(0, paid-required)`、待补缴 `max(0, required-paid)`、
//!   待审核提现合计、账户状态/惩罚/补缴截止、状态文案、协议开关。
//! - `logList`：流水类型合并（2/3 → IN(2,3)）、时间区间、业务编号精确匹配、
//!   来源/去向账户富化（`DepositShopLogEnricher` 口径）。
//! - `frozenDetailList`：仅 `type = 4`（平台扣除），`deductAmount = |changeAmount|`。
//! - `warning`：待补缴 > 0 才预警；缓冲剩余天数（收取优先）+ 优先级文案。
//! - `protocol`：`deposit_protocol_enable/content` 扁平配置。
//!
//! # 偏差（诚实声明）
//!
//! 1. **店铺上下文**：Java 取请求头 `X-Shop-Id`（`HeaderUtils.getShopId()`，
//!    缺失抛 401「无法获取当前店铺信息」）；Rust JWT 无 shop_id，5 个端点均
//!    接受可选查询参数 `?shopId=`，缺失/非法时返回同一文案的业务错误。
//! 2. `DepositAccountService.computeRequired/getPaid/pending` 与
//!    `DepositRefundService.pendingRefundSum` 在本实现内按同口径重写（只读 SQL），
//!    不跨服务调用；数值结果与 Java 一致。
//! 3. 来源/去向账户展示：Java `formatPayeeDisplay` 依赖 `DepositPayeeAccountVO`；
//!    本实现解析 `payee_snapshot` / `payee_info` JSON 的
//!    `accountType/accountName/accountNo`（兼容 camelCase），缺失回退账户类型文案。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::entities::deposit::ShopDepositLog;
use nextrade_core::models::merchant::deposit_shop::{
    build_warning_messages, deposit_status_text, format_payee_display, format_ts, log_type_text,
    normalize_log_type, parse_bool, remain_days, resolve_buffer_days, DepositShopFrozenDetailVO,
    DepositShopLogQuery, DepositShopLogVO, DepositShopOverviewVO, DepositShopProtocolVO,
    DepositShopWarningVO, CONFIG_DEPOSIT_ENABLE, CONFIG_PROTOCOL_CONTENT, CONFIG_PROTOCOL_ENABLE,
    OWNER_SHOP,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 类目就近溯源最大层级（Java `MAX_CATEGORY_LEVEL`）。
const MAX_CATEGORY_LEVEL: usize = 3;
/// 站内账户名（Java `ACCOUNT_DEPOSIT_BALANCE`）。
const ACCOUNT_DEPOSIT_BALANCE: &str = "保证金余额";
/// 平台账户名（Java `ACCOUNT_PLATFORM`）。
const ACCOUNT_PLATFORM: &str = "平台账户";
/// 店铺上下文缺失文案（Java `GlobalException`）。
const SHOP_CONTEXT_REQUIRED: &str = "无法获取当前店铺信息";

pub struct DepositShopService {
    repos: Arc<Repositories>,
}

impl DepositShopService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 总览 —————————————————————

    /// `GET /adminapi/deposit/shop/overview`：保证金总览。
    pub async fn overview(&self, shop_id: i32) -> Result<DepositShopOverviewVO> {
        let required = self.compute_required(shop_id).await?;
        let paid = self.get_paid(shop_id).await?;
        let frozen = paid.min(required);
        let withdrawable = max0(paid - required);
        let pending = max0(required - paid);
        let pending_withdraw = self
            .repos
            .merchant
            .deposit_shop
            .pending_refund_sum(shop_id)
            .await?;
        let account = self
            .repos
            .merchant
            .deposit_shop
            .find_account(shop_id)
            .await?;
        let status = account.as_ref().map_or(1, |a| a.status);
        let penalty_status = account.as_ref().map_or(0, |a| a.penalty_status);
        let supplement_due_time = account
            .as_ref()
            .map_or(0, |a| i64::from(a.supplement_due_time));
        let protocol = self.protocol().await?;
        let has_protocol = protocol.enable
            && protocol
                .content
                .as_deref()
                .is_some_and(|c| !c.trim().is_empty());
        Ok(DepositShopOverviewVO {
            required,
            paid,
            pending,
            withdrawable,
            pending_withdraw,
            frozen,
            status,
            penalty_status,
            status_text: deposit_status_text(penalty_status, pending).to_string(),
            supplement_due_time,
            supplement_due_time_text: format_ts(supplement_due_time),
            has_protocol,
        })
    }

    // ————————————————————— 流水 / 冻结明细 —————————————————————

    /// `GET /adminapi/deposit/shop/logList`：店铺端交易流水。
    pub async fn log_list(
        &self,
        shop_id: i32,
        q: &DepositShopLogQuery,
    ) -> Result<PageResult<DepositShopLogVO>> {
        let page = q.page_query();
        let (total, logs) = self
            .repos
            .merchant
            .deposit_shop
            .paginate_logs(shop_id, q, false)
            .await?;
        let account_texts = self.enrich_account_texts(&logs).await?;
        let list = logs
            .iter()
            .map(|log| {
                let (source, dest) = account_texts
                    .get(&log.shop_deposit_log_id)
                    .cloned()
                    .unwrap_or_default();
                DepositShopLogVO {
                    shop_deposit_log_id: log.shop_deposit_log_id,
                    r#type: normalize_log_type(log.r#type),
                    type_text: log_type_text(log.r#type).to_string(),
                    change_amount: log.change_amount,
                    before_paid: log.before_paid,
                    after_paid: log.after_paid,
                    order_id: log.order_id,
                    related_id: log.related_id,
                    remark: log.remark.clone(),
                    add_time: i64::from(log.add_time),
                    add_time_text: format_ts(i64::from(log.add_time)),
                    source_account: source,
                    dest_account: dest,
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/deposit/shop/frozenDetailList`：冻结明细（平台扣款记录）。
    pub async fn frozen_detail_list(
        &self,
        shop_id: i32,
        q: &DepositShopLogQuery,
    ) -> Result<PageResult<DepositShopFrozenDetailVO>> {
        let page = q.page_query();
        let (total, logs) = self
            .repos
            .merchant
            .deposit_shop
            .paginate_logs(shop_id, q, true)
            .await?;
        let list = logs
            .iter()
            .map(|log| DepositShopFrozenDetailVO {
                shop_deposit_log_id: log.shop_deposit_log_id,
                r#type: normalize_log_type(log.r#type),
                type_text: log_type_text(log.r#type).to_string(),
                deduct_amount: log.change_amount.abs(),
                change_amount: log.change_amount,
                reason: log.remark.clone(),
                order_id: log.order_id,
                related_id: log.related_id,
                add_time: i64::from(log.add_time),
                add_time_text: format_ts(i64::from(log.add_time)),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 预警 / 协议 —————————————————————

    /// `GET /adminapi/deposit/shop/warning`：首页保证金预警横幅。
    pub async fn warning(&self, shop_id: i32) -> Result<DepositShopWarningVO> {
        let pending = self.pending(shop_id).await?;
        let account = self
            .repos
            .merchant
            .deposit_shop
            .find_account(shop_id)
            .await?;
        let supplement_due_time = account
            .as_ref()
            .map_or(0, |a| i64::from(a.supplement_due_time));
        let collect_due_time = account
            .as_ref()
            .map_or(0, |a| i64::from(a.collect_due_time));
        let need_warning = pending > Decimal::ZERO;
        let now = nextrade_common::utils::now_seconds();
        let buffer_days = resolve_buffer_days(collect_due_time, supplement_due_time, now);
        let messages = build_warning_messages(
            need_warning,
            collect_due_time,
            supplement_due_time,
            "店铺",
            now,
        );
        Ok(DepositShopWarningVO {
            need_warning,
            pending_amount: pending,
            supplement_due_time,
            supplement_due_time_text: format_ts(supplement_due_time),
            buffer_days,
            message: messages.join("\n"),
            messages,
        })
    }

    /// `GET /adminapi/deposit/shop/protocol`：保证金协议。
    pub async fn protocol(&self) -> Result<DepositShopProtocolVO> {
        let enable = self
            .repos
            .merchant
            .deposit_shop
            .find_config(CONFIG_PROTOCOL_ENABLE)
            .await?;
        let content = self
            .repos
            .merchant
            .deposit_shop
            .find_config(CONFIG_PROTOCOL_CONTENT)
            .await?;
        Ok(DepositShopProtocolVO {
            enable: parse_bool(enable.as_deref()),
            content,
        })
    }

    // ————————————————————— 应缴计算（Java DepositAccountService 同口径） —————————————————————

    /// 解析店铺上下文（Java `HeaderUtils.getShopId()` 缺失 → 401）。
    pub fn resolve_shop_id(shop_id: Option<i32>) -> Result<i32> {
        match shop_id {
            Some(id) if id > 0 => Ok(id),
            _ => Err(Error::Unauthorized(SHOP_CONTEXT_REQUIRED.into())),
        }
    }

    /// 应缴（Java `computeRequired`）：功能关闭 → 0；手动应缴优先；
    /// 无账户/未激活 → 0；否则规则匹配取最高。
    pub async fn compute_required(&self, shop_id: i32) -> Result<Decimal> {
        if !self.is_deposit_enabled().await? {
            return Ok(Decimal::ZERO);
        }
        let account = self
            .repos
            .merchant
            .deposit_shop
            .find_account(shop_id)
            .await?;
        let Some(account) = account else {
            return Ok(Decimal::ZERO);
        };
        if let Some(manual) = account.required_manual {
            return Ok(manual);
        }
        if account.collect_activated != 1 {
            return Ok(Decimal::ZERO);
        }
        self.max_required(shop_id).await
    }

    /// 实缴（Java `getPaid`）。
    async fn get_paid(&self, shop_id: i32) -> Result<Decimal> {
        Ok(self
            .repos
            .merchant
            .deposit_shop
            .find_account(shop_id)
            .await?
            .map_or(Decimal::ZERO, |a| a.paid))
    }

    /// 待补缴（Java `pending`）。
    async fn pending(&self, shop_id: i32) -> Result<Decimal> {
        let required = self.compute_required(shop_id).await?;
        let paid = self.get_paid(shop_id).await?;
        Ok(max0(required - paid))
    }

    /// 保证金功能是否开启（Java `isDepositEnabled`：`deposit_enable == 1`，缺失视为关闭）。
    async fn is_deposit_enabled(&self) -> Result<bool> {
        let raw = self
            .repos
            .merchant
            .deposit_shop
            .find_config(CONFIG_DEPOSIT_ENABLE)
            .await?;
        let value = raw
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        Ok(value == 1)
    }

    /// 自动应缴：全部在售商品按三级就近命中规则取最高（Java `DepositRuleMatcher#maxRequired`）。
    async fn max_required(&self, shop_id: i32) -> Result<Decimal> {
        let leaf_category_ids = self
            .repos
            .merchant
            .deposit_shop
            .on_sale_category_ids(shop_id)
            .await?;
        if leaf_category_ids.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let Some(merchant_type) = self
            .repos
            .merchant
            .deposit_shop
            .shop_merchant_type(shop_id)
            .await?
        else {
            return Ok(Decimal::ZERO);
        };
        // 规则匹配的「店铺类型」维度直接用 owner_type 同码值（1 店铺）
        let shop_type = OWNER_SHOP;

        let mut chains: Vec<Vec<i32>> = Vec::new();
        let mut candidate_category_ids: Vec<i32> = Vec::new();
        let mut seen_leaves: Vec<i32> = Vec::new();
        let mut parent_cache: HashMap<i32, Option<i32>> = HashMap::new();
        for leaf in leaf_category_ids {
            if leaf <= 0 || seen_leaves.contains(&leaf) {
                continue;
            }
            seen_leaves.push(leaf);
            let chain = self.build_category_chain(leaf, &mut parent_cache).await?;
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
        let now = nextrade_common::utils::now_seconds();
        let rows = self
            .repos
            .merchant
            .deposit_shop
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
                    if max.map_or(true, |m| *amount > m) {
                        max = Some(*amount);
                    }
                    break;
                }
            }
        }
        Ok(max.unwrap_or(Decimal::ZERO))
    }

    /// 构建类目就近链：自身 → 父 → 祖父（最多三级，去重防环）。
    async fn build_category_chain(
        &self,
        leaf: i32,
        parent_cache: &mut HashMap<i32, Option<i32>>,
    ) -> Result<Vec<i32>> {
        let mut chain: Vec<i32> = vec![leaf];
        let mut current = leaf;
        while chain.len() < MAX_CATEGORY_LEVEL {
            let parent = if let Some(p) = parent_cache.get(&current) {
                *p
            } else {
                let p = self
                    .repos
                    .merchant
                    .deposit_shop
                    .category_parent(current)
                    .await?;
                parent_cache.insert(current, p);
                p
            };
            let Some(parent) = parent.filter(|p| *p > 0) else {
                break;
            };
            if chain.contains(&parent) {
                break;
            }
            chain.push(parent);
            current = parent;
        }
        Ok(chain)
    }

    // ————————————————————— 流水账户富化（DepositShopLogEnricher） —————————————————————

    /// 批量回填来源/去向账户文案：`log_id → (source, dest)`。
    async fn enrich_account_texts(
        &self,
        logs: &[ShopDepositLog],
    ) -> Result<HashMap<i32, (String, String)>> {
        let recharge_ids: Vec<i32> = logs
            .iter()
            .filter(|l| l.r#type == 1 && l.related_id > 0)
            .map(|l| l.related_id)
            .collect();
        let refund_ids: Vec<i32> = logs
            .iter()
            .filter(|l| matches!(l.r#type, 2 | 3) && l.related_id > 0)
            .map(|l| l.related_id)
            .collect();
        let recharge_map = self
            .repos
            .merchant
            .deposit_shop
            .recharge_map(&dedup(recharge_ids))
            .await?;
        let refund_map = self
            .repos
            .merchant
            .deposit_shop
            .refund_map(&dedup(refund_ids))
            .await?;
        let mut out = HashMap::new();
        for log in logs {
            let (source, dest) = match log.r#type {
                1 => {
                    let recharge = recharge_map.get(&log.related_id);
                    (
                        recharge_pay_account(recharge),
                        ACCOUNT_DEPOSIT_BALANCE.to_string(),
                    )
                }
                4 => (
                    ACCOUNT_DEPOSIT_BALANCE.to_string(),
                    ACCOUNT_PLATFORM.to_string(),
                ),
                2 | 3 => {
                    let refund = refund_map.get(&log.related_id);
                    (
                        ACCOUNT_DEPOSIT_BALANCE.to_string(),
                        refund_payee_account(refund),
                    )
                }
                5 => (
                    ACCOUNT_DEPOSIT_BALANCE.to_string(),
                    ACCOUNT_DEPOSIT_BALANCE.to_string(),
                ),
                _ => (String::new(), String::new()),
            };
            out.insert(log.shop_deposit_log_id, (source, dest));
        }
        Ok(out)
    }
}

// ————————————————————— 纯函数 —————————————————————

fn max0(value: Decimal) -> Decimal {
    if value < Decimal::ZERO {
        Decimal::ZERO
    } else {
        value
    }
}

fn dedup(mut ids: Vec<i32>) -> Vec<i32> {
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// 充值来源账户（Java `formatRechargePayAccount`：优先收款快照 JSON）。
#[must_use]
pub fn recharge_pay_account(
    recharge: Option<&nextrade_core::entities::deposit::ShopDepositRecharge>,
) -> String {
    let Some(recharge) = recharge else {
        return String::new();
    };
    if let Some(raw) = recharge.payee_snapshot.as_deref() {
        if let Some(text) = parse_payee_json(raw) {
            return text;
        }
    }
    format_payee_display(Some(recharge.payee_account_type), None, None)
}

/// 退款/提现去向账户（Java `formatRefundPayeeAccount`：解析 `payee_info` JSON）。
#[must_use]
pub fn refund_payee_account(
    refund: Option<&nextrade_core::entities::deposit::ShopDepositRefund>,
) -> String {
    let Some(refund) = refund else {
        return String::new();
    };
    if let Some(raw) = refund.payee_info.as_deref() {
        if let Some(text) = parse_payee_json(raw) {
            return text;
        }
    }
    String::new()
}

/// 解析收款账户 JSON（兼容 camelCase/snake_case 字段）：
/// `accountType` / `accountName` / `accountNo`。
#[must_use]
pub fn parse_payee_json(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let account_type = value
        .get("accountType")
        .or_else(|| value.get("account_type"))
        .and_then(serde_json::Value::as_i64)
        .map(|v| v as i32);
    let account_name = value
        .get("accountName")
        .or_else(|| value.get("account_name"))
        .and_then(serde_json::Value::as_str);
    let account_no = value
        .get("accountNo")
        .or_else(|| value.get("account_no"))
        .and_then(serde_json::Value::as_str);
    let text = format_payee_display(account_type, account_name, account_no);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_context_resolution_requires_positive_id() {
        assert_eq!(DepositShopService::resolve_shop_id(Some(3)).unwrap(), 3);
        assert!(DepositShopService::resolve_shop_id(Some(0)).is_err());
        assert!(DepositShopService::resolve_shop_id(None).is_err());
        let err = DepositShopService::resolve_shop_id(None).unwrap_err();
        assert!(format!("{err}").contains(SHOP_CONTEXT_REQUIRED));
    }

    #[test]
    fn payee_json_parsing_is_tolerant() {
        assert_eq!(
            parse_payee_json(r#"{"accountType":1,"accountName":"张三","accountNo":"6222"}"#)
                .unwrap(),
            "银行卡 张三 6222"
        );
        assert_eq!(
            parse_payee_json(r#"{"account_type":2,"account_name":"李四"}"#).unwrap(),
            "支付宝 李四"
        );
        assert!(parse_payee_json("not-json").is_none());
        assert!(parse_payee_json("{}").is_none());
        // 无快照时回退账户类型文案
        assert_eq!(recharge_pay_account(None), "");
        assert_eq!(refund_payee_account(None), "");
    }

    #[test]
    fn max0_clamps_negative_to_zero() {
        assert_eq!(max0(Decimal::new(5, 0)), Decimal::new(5, 0));
        assert_eq!(max0(Decimal::new(-5, 0)), Decimal::ZERO);
        let _ = remain_days(1_000, 0);
    }
}
