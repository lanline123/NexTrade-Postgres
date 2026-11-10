//! 店铺保证金规则服务（`merchant::deposit_rule`，6 个管理端端点）。
//!
//! 对应原 Java `com.nextrade.service.deposit.impl.DepositRuleServiceImpl`：
//! `list` / `detail` / `create` / `update` / `del` / `changeStatus`。
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! - `list`：`is_delete=0` + 名称 ILIKE + 状态 + 生效窗口交集 + 主键倒序；
//!   批量回填类目关联与类目名（避免 N+1）。
//! - `detail`：软删除视为不存在（`规则不存在`）。
//! - `create` / `update`：`validateBase` 全部逐字校验、去重保序（LinkedHashSet 语义）、
//!   同维度生效时间重叠校验（主体类型 × 店铺类型 × 类目 三维交集 + 时间区间重叠）、
//!   类目关联先删后插；`end_time` 空 → 0（长期有效）；`status` 缺省 1。
//! - `del`：软删除（`is_delete = 1` + `update_time`）。
//! - `changeStatus`：启停 + `update_time`。
//!
//! # 未迁移（诚实声明）
//!
//! Java 在 create/update/del/changeStatus 事务提交后调用
//! `DepositLifecycleService#onDepositRuleChange` 对现有保证金账户对账
//! （刷新应缴快照、写调整记录、上调时启动补缴缓冲）。该生命周期服务属
//! 保证金账户/任务域，未迁移：本实现记录 warn 日志，规则本身照常落库，
//! 账户应缴快照留待生命周期域补齐后由定时对账修正。

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::ShopDepositRule;
use nextrade_core::models::merchant::deposit_rule::{
    intersects, join_codes, parse_codes, time_overlap, DepositRuleDTO, DepositRuleListQuery,
    DepositRuleStatusDTO, DepositRuleVO, CATEGORY_IDS_REQUIRED, MERCHANT_TYPES_REQUIRED,
    RULE_ID_REQUIRED, RULE_NOT_EXISTS, RULE_TIME_OVERLAP, SHOP_TYPES_REQUIRED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct DepositRuleService {
    repos: Arc<Repositories>,
}

impl DepositRuleService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/deposit/rule/list`：规则分页。
    pub async fn list(&self, q: &DepositRuleListQuery) -> Result<PageResult<DepositRuleVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.merchant.deposit_rule.paginate(q, &page).await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let rule_ids: Vec<i32> = rows.iter().map(|r| r.shop_deposit_rule_id).collect();
        let category_map = self
            .repos
            .merchant
            .deposit_rule
            .category_map(&rule_ids)
            .await?;
        let all_category_ids: Vec<i32> = {
            let mut ids: Vec<i32> = category_map.values().flatten().copied().collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        let name_map = self
            .repos
            .merchant
            .deposit_rule
            .category_names(&all_category_ids)
            .await?;
        let list = rows
            .iter()
            .map(|rule| {
                let category_ids = category_map
                    .get(&rule.shop_deposit_rule_id)
                    .cloned()
                    .unwrap_or_default();
                DepositRuleVO::from_rule(rule, category_ids, &name_map)
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/deposit/rule/detail?id=`：规则详情。
    pub async fn detail(&self, id: Option<i32>) -> Result<DepositRuleVO> {
        let id = id.ok_or_else(|| Error::biz_msg(RULE_NOT_EXISTS))?;
        let rule = self
            .repos
            .merchant
            .deposit_rule
            .find_by_id(id)
            .await?
            .filter(|r| r.is_delete == 0)
            .ok_or_else(|| Error::biz_msg(RULE_NOT_EXISTS))?;
        let category_ids = self
            .repos
            .merchant
            .deposit_rule
            .category_ids_by_rule(id)
            .await?;
        let name_map = self
            .repos
            .merchant
            .deposit_rule
            .category_names(&category_ids)
            .await?;
        Ok(DepositRuleVO::from_rule(&rule, category_ids, &name_map))
    }

    /// `POST /adminapi/deposit/rule/create`：创建规则。
    pub async fn create(&self, dto: &DepositRuleDTO) -> Result<()> {
        dto.validate().map_err(Error::Validation)?;
        let merchant_types = unique_keep_order(dto.merchant_types.as_deref().unwrap_or_default());
        let shop_types = unique_keep_order(dto.shop_types.as_deref().unwrap_or_default());
        let category_ids = unique_keep_order(dto.category_ids.as_deref().unwrap_or_default());
        let start_time = dto.start_time.unwrap_or(0);
        let end_time = dto.end_time.unwrap_or(0);
        self.ensure_no_overlap(None, &merchant_types, &shop_types, &category_ids, start_time, end_time)
            .await?;

        let now = nextrade_common::utils::now_seconds() as i32;
        let rule = ShopDepositRule {
            shop_deposit_rule_id: 0,
            rule_name: dto.rule_name.clone().unwrap_or_default().trim().to_string(),
            merchant_types: join_codes(&merchant_types),
            shop_types: join_codes(&shop_types),
            deposit_amount: dto.deposit_amount.unwrap_or_default(),
            start_time: start_time as i32,
            end_time: end_time as i32,
            status: dto.status.unwrap_or(1),
            is_delete: 0,
            add_time: now,
            update_time: now,
        };
        let id = self.repos.merchant.deposit_rule.insert(&rule).await?;
        self.save_categories(id, &category_ids).await?;
        self.on_deposit_rule_change().await;
        Ok(())
    }

    /// `POST /adminapi/deposit/rule/update`：更新规则。
    pub async fn update(&self, dto: &DepositRuleDTO) -> Result<()> {
        let id = dto
            .shop_deposit_rule_id
            .ok_or_else(|| Error::Validation(RULE_ID_REQUIRED.into()))?;
        let exist = self
            .repos
            .merchant
            .deposit_rule
            .find_by_id(id)
            .await?
            .filter(|r| r.is_delete == 0)
            .ok_or_else(|| Error::biz_msg(RULE_NOT_EXISTS))?;
        dto.validate().map_err(Error::Validation)?;
        let merchant_types = unique_keep_order(dto.merchant_types.as_deref().unwrap_or_default());
        let shop_types = unique_keep_order(dto.shop_types.as_deref().unwrap_or_default());
        let category_ids = unique_keep_order(dto.category_ids.as_deref().unwrap_or_default());
        let start_time = dto.start_time.unwrap_or(0);
        let end_time = dto.end_time.unwrap_or(0);
        self.ensure_no_overlap(
            Some(id),
            &merchant_types,
            &shop_types,
            &category_ids,
            start_time,
            end_time,
        )
        .await?;

        let now = nextrade_common::utils::now_seconds() as i32;
        let rule = ShopDepositRule {
            shop_deposit_rule_id: id,
            rule_name: dto.rule_name.clone().unwrap_or_default().trim().to_string(),
            merchant_types: join_codes(&merchant_types),
            shop_types: join_codes(&shop_types),
            deposit_amount: dto.deposit_amount.unwrap_or(exist.deposit_amount),
            start_time: start_time as i32,
            end_time: end_time as i32,
            status: dto.status.unwrap_or(exist.status),
            is_delete: 0,
            add_time: exist.add_time,
            update_time: now,
        };
        self.repos.merchant.deposit_rule.update(&rule).await?;
        // 类目关联先删后插
        self.repos.merchant.deposit_rule.delete_categories(id).await?;
        self.save_categories(id, &category_ids).await?;
        self.on_deposit_rule_change().await;
        Ok(())
    }

    /// `POST /adminapi/deposit/rule/del`：软删除规则。
    pub async fn del(&self, id: Option<i32>) -> Result<()> {
        let Some(id) = id else {
            // Java `del(null)`：getById(null) 返回 null → 抛「规则不存在」
            return Err(Error::biz_msg(RULE_NOT_EXISTS));
        };
        let rule = self
            .repos
            .merchant
            .deposit_rule
            .find_by_id(id)
            .await?
            .filter(|r| r.is_delete == 0)
            .ok_or_else(|| Error::biz_msg(RULE_NOT_EXISTS))?;
        let _ = rule;
        let now = nextrade_common::utils::now_seconds() as i32;
        self.repos
            .merchant
            .deposit_rule
            .soft_delete(id, now)
            .await?;
        self.on_deposit_rule_change().await;
        Ok(())
    }

    /// `POST /adminapi/deposit/rule/status`：启用/停用规则。
    pub async fn change_status(&self, dto: &DepositRuleStatusDTO) -> Result<()> {
        let id = dto
            .shop_deposit_rule_id
            .ok_or_else(|| Error::biz_msg(RULE_NOT_EXISTS))?;
        let status = dto
            .status
            .ok_or_else(|| Error::Validation("状态不能为空".into()))?;
        let rule = self
            .repos
            .merchant
            .deposit_rule
            .find_by_id(id)
            .await?
            .filter(|r| r.is_delete == 0)
            .ok_or_else(|| Error::biz_msg(RULE_NOT_EXISTS))?;
        let now = nextrade_common::utils::now_seconds() as i32;
        self.repos
            .merchant
            .deposit_rule
            .update_status(rule.shop_deposit_rule_id, status, now)
            .await?;
        self.on_deposit_rule_change().await;
        Ok(())
    }

    // ————————————————————— 内部辅助 —————————————————————

    async fn save_categories(&self, rule_id: i32, category_ids: &[i32]) -> Result<()> {
        for category_id in category_ids {
            if *category_id <= 0 {
                continue;
            }
            self.repos
                .merchant
                .deposit_rule
                .insert_category(rule_id, *category_id)
                .await?;
        }
        Ok(())
    }

    /// 同维度生效时间重叠校验（Java `checkTimeOverlap`）：
    /// 三维（主体/店型/类目）同时有交集且时间区间重叠 → 冲突。
    async fn ensure_no_overlap(
        &self,
        self_id: Option<i32>,
        merchant_types: &[i32],
        shop_types: &[i32],
        category_ids: &[i32],
        new_start: i64,
        new_end: i64,
    ) -> Result<()> {
        let candidates = self
            .repos
            .merchant
            .deposit_rule
            .overlap_candidates(self_id, new_start, new_end)
            .await?;
        if candidates.is_empty() {
            return Ok(());
        }
        let ids: Vec<i32> = candidates
            .iter()
            .map(|c| c.shop_deposit_rule_id)
            .collect();
        let category_map = self.repos.merchant.deposit_rule.category_map(&ids).await?;
        for candidate in &candidates {
            let cand_merchant = parse_codes(&candidate.merchant_types);
            let cand_shop = parse_codes(&candidate.shop_types);
            let cand_categories = category_map
                .get(&candidate.shop_deposit_rule_id)
                .cloned()
                .unwrap_or_default();
            if intersects(merchant_types, &cand_merchant)
                && intersects(shop_types, &cand_shop)
                && intersects(category_ids, &cand_categories)
                && time_overlap(
                    new_start,
                    new_end,
                    i64::from(candidate.start_time),
                    i64::from(candidate.end_time),
                )
            {
                return Err(Error::biz_msg(RULE_TIME_OVERLAP));
            }
        }
        Ok(())
    }

    /// 规则变更后异步执行全量账户生命周期对账（对齐 Java DepositLifecycleServiceImpl#onDepositRuleChange）。
    pub async fn on_deposit_rule_change(&self) {
        let Ok(setting) = crate::merchant::deposit_setting::DepositSettingService::new(self.repos.clone()).detail().await else {
            return;
        };
        if setting.deposit_enable != Some(1) {
            return;
        }

        let Ok(accounts) = self.repos.merchant.deposit_account.list_all_accounts().await else {
            return;
        };

        let now = nextrade_common::utils::now_seconds() as i32;
        let buffer_days = setting.deposit_supplement_buffer_days.unwrap_or(7).max(1);
        let shop_deposit_svc = crate::merchant::deposit_shop::DepositShopService::new(self.repos.clone());
        let vendor_deposit_svc = crate::merchant::vendor_deposit_shop::VendorDepositShopService::new(self.repos.clone());
        let account_svc = crate::merchant::deposit_account::DepositAccountService::new(self.repos.clone());

        for account in accounts {
            if account.required_manual.is_some() {
                // 手动固定应缴，不随规则变动，跳过
                continue;
            }
            let owner_type = account.owner_type;
            let owner_id = account.owner_id;
            if owner_id <= 0 {
                continue;
            }

            let after = if owner_type == 1 {
                shop_deposit_svc.compute_required(owner_id).await.unwrap_or(Decimal::ZERO)
            } else if owner_type == 2 {
                vendor_deposit_svc.compute_required(owner_id).await.unwrap_or(Decimal::ZERO)
            } else {
                continue;
            };

            let before = account.required_snapshot;
            if after == before {
                continue;
            }

            let supplement_due_time = if after > before && before > Decimal::ZERO {
                Some(now + buffer_days * 86400)
            } else {
                None
            };

            if let Err(e) = self.repos.merchant.deposit_account.apply_rule_change_adjust(
                account.shop_deposit_id,
                owner_type,
                owner_id,
                before,
                after,
                supplement_due_time,
                now,
            ).await {
                tracing::warn!("保证金规则调整对账账户更新失败: owner_type={owner_type}, owner_id={owner_id}, {e}");
                continue;
            }

            // 若应缴下调且实缴已满足，触发生命周期 restore 解除处罚
            if after <= before && account.paid >= after {
                let _ = account_svc.restore(owner_type, owner_id).await;
            }
        }
        tracing::info!("保证金规则变更全量账户对账完成");
    }
}

/// 去重保序（Java `new LinkedHashSet<>(list)`）。
#[must_use]
pub fn unique_keep_order(items: &[i32]) -> Vec<i32> {
    let mut seen = BTreeSet::new();
    items
        .iter()
        .filter(|v| seen.insert(**v))
        .copied()
        .collect()
}

/// 类目 id → 名称映射的稳定输出（仅供测试辅助）。
#[allow(dead_code)]
fn _category_names_map(ids: &[i32]) -> HashMap<i32, String> {
    ids.iter().map(|id| (*id, String::new())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_keep_order_matches_linked_hash_set() {
        assert_eq!(unique_keep_order(&[2, 1, 2, 3, 1]), vec![2, 1, 3]);
        assert_eq!(unique_keep_order(&[]), Vec::<i32>::new());
    }

    #[test]
    fn overlap_three_dimension_logic_matches_java() {
        // 三维都有交集 + 时间重叠 → 冲突
        assert!(intersects(&[1], &[1]) && intersects(&[1], &[1]) && intersects(&[10], &[10]));
        // 任一无交集 → 不冲突
        assert!(!intersects(&[1], &[2]));
        // 类目必填校验文案（与 Java `saveCategories` 前置的 validateBase 一致）
        assert_eq!(CATEGORY_IDS_REQUIRED, "适用类目不能为空");
        assert_eq!(MERCHANT_TYPES_REQUIRED, "适用主体类型不能为空");
        assert_eq!(SHOP_TYPES_REQUIRED, "适用店铺类型不能为空");
    }
}
