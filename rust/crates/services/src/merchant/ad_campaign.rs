//! 广告投流服务（merchant::ad_campaign）：商家自助投流 + 平台审核/置顶/下架。
//!
//! 规则（PLAN §4）：
//! - 商家只能给本店投（路由层用 `TenantContext::require_shop()` 强制本店）；
//!   店铺投放只能投 `shop_street`，商品投放的对象须属本店且在售。
//! - 计价 = `adSettings` 对应投放位单价（下单时快照）× 天数；付款一期从
//!   `shop_money` 店铺余额扣款，复用 settlement 的资金口径写 `shop_account_log`
//!   流水（type 新值 `'ad'`，`event_key = ad_campaign:{id}` 防重）。
//!   余额不足明确报错且不落单、不扣款（建单与扣款同一事务）。
//! - 状态机以 core `ad_campaign::can_transition` 为唯一口径；审核照
//!   `merchant_apply.audit`（必须待审、目标只能通过/驳回）。
//! - 平台置顶 = 平台自建 `is_pin = 1`、`amount = 0` 的投放单，与付费推广同表。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::models::merchant::ad_campaign::{
    can_transition, quote, AdCampaign, AdCampaignAuditParam, AdCampaignCreateParam,
    AdCampaignListQuery, AdCampaignPinParam, AdCampaignVO, Lifecycle, AUDIT_PASS, AUDIT_PENDING, AUDIT_REJECT,
    PAY_CHANNEL_PIN, PAY_CHANNEL_SHOP_MONEY, PAY_PAID, PAY_UNPAID, POSITIONS,
    POSITION_SHOP_STREET, STATUS_ACTIVE, STATUS_ENDED, STATUS_PAUSED, STATUS_SCHEDULED,
    STATUS_TAKEN_DOWN, TARGET_PRODUCT, TARGET_SHOP,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::ad_campaign::AdCampaignRepository;
use nextrade_infrastructure::db::Repositories;

use crate::system::ad_settings::{effective_ad_settings, unit_price, AD_SETTINGS_CODE};

/// 全局业务错误码（与 merchant_apply 同口径）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 投流服务。
pub struct AdCampaignService {
    repos: Arc<Repositories>,
}

impl AdCampaignService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &AdCampaignRepository {
        &self.repos.merchant.ad_campaign
    }

    fn now() -> i32 {
        i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
    }

    /// 生效中的投流配置。
    pub async fn settings(&self) -> Result<serde_json::Value> {
        let stored = self
            .repos
            .system
            .config_basic
            .find_by_code(AD_SETTINGS_CODE)
            .await?;
        Ok(effective_ad_settings(stored.as_deref()))
    }

    // ————————————————————— 商家：新建投放（建单 + 余额扣款，同一事务） —————————————————————

    /// 商家新建投放并从店铺余额扣款。
    ///
    /// # Errors
    /// - 对象不属本店 / 商品不在售 / 投放位非法 ⇒ 业务错误，不落单；
    /// - 余额不足 ⇒ 业务错误「店铺余额不足…」，事务回滚，不落单不扣款。
    pub async fn create_for_shop(
        &self,
        shop_id: i32,
        param: &AdCampaignCreateParam,
    ) -> Result<AdCampaign> {
        let target_type = param.target_type.unwrap_or(TARGET_SHOP);
        if target_type != TARGET_SHOP && target_type != TARGET_PRODUCT {
            return Err(Error::biz(SERVICE_DATA_ERROR, "投放对象类型不正确"));
        }
        let position = param
            .position
            .clone()
            .unwrap_or_else(|| POSITION_SHOP_STREET.to_string());
        if !POSITIONS.contains(&position.as_str()) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "投放位不正确"));
        }
        let product_id = if target_type == TARGET_PRODUCT {
            let pid = param
                .product_id
                .filter(|v| *v > 0)
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "商品投放必须选择商品"))?;
            if !self.repo().product_is_sellable_by_shop(pid, shop_id).await? {
                return Err(Error::biz(SERVICE_DATA_ERROR, "商品不存在、不属本店或未在售"));
            }
            pid
        } else {
            if position != POSITION_SHOP_STREET {
                return Err(Error::biz(SERVICE_DATA_ERROR, "店铺投放只能投店铺街"));
            }
            if !self.repo().shop_is_open(shop_id).await? {
                return Err(Error::biz(SERVICE_DATA_ERROR, "店铺不存在或未开业"));
            }
            0
        };

        let settings = self.settings().await?;
        let price = unit_price(&settings, &position);
        let min_days = settings["minDays"].as_i64().unwrap_or(3) as i32;
        let need_audit = settings["needAudit"].as_bool().unwrap_or(true);
        let now = Self::now();
        let q = quote(price, param.days.unwrap_or(min_days), min_days, param.start_time.unwrap_or(0), now);

        let mut campaign = AdCampaign {
            target_type,
            shop_id,
            product_id,
            position,
            start_time: q.start_time,
            end_time: q.end_time,
            days: q.days,
            unit_price: q.unit_price,
            amount: q.amount,
            // 一期权重 = 金额（整数元）：花得多排前，同额按 id 先后（先买先排）
            weight: q.amount.to_string().parse::<f64>().map(|v| v as i32).unwrap_or(0),
            pay_status: PAY_UNPAID,
            audit_status: if need_audit { AUDIT_PENDING } else { AUDIT_PASS },
            status: STATUS_SCHEDULED,
            add_time: now,
            update_time: now,
            ..Default::default()
        };

        let repo = self.repo();
        let mut tx = repo.begin().await?;
        let id = repo.insert_in_tx(&mut tx, &campaign).await?;
        campaign.id = id;

        if q.amount > Decimal::ZERO {
            // event_key 防重：同单已扣过款直接视为已付（幂等重入保护）
            if !repo.account_log_exists_in_tx(&mut tx, id).await? {
                let Some((old_money, frozen)) = repo.lock_shop_money_in_tx(&mut tx, shop_id).await?
                else {
                    return Err(Error::biz(SERVICE_DATA_ERROR, "店铺不存在"));
                };
                if old_money < q.amount {
                    return Err(Error::biz(
                        SERVICE_DATA_ERROR,
                        "店铺余额不足，请先通过销售结算积累余额或联系平台",
                    ));
                }
                if !repo.debit_shop_money_in_tx(&mut tx, shop_id, q.amount).await? {
                    return Err(Error::biz(
                        SERVICE_DATA_ERROR,
                        "店铺余额不足，请先通过销售结算积累余额或联系平台",
                    ));
                }
                let log_id = repo
                    .insert_account_log_in_tx(&mut tx, shop_id, old_money, frozen, q.amount, id, now)
                    .await?;
                campaign.account_log_id = log_id;
            }
        }
        campaign.pay_status = PAY_PAID;
        campaign.pay_channel = PAY_CHANNEL_SHOP_MONEY.to_string();
        campaign.pay_time = now;
        campaign.update_time = now;
        repo.update_in_tx(&mut tx, &campaign).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(campaign)
    }

    // ————————————————————— 商家：我的投放 / 暂停 / 恢复 —————————————————————

    /// 列表行 → VO：按本页 shop_id / product_id 各做一次批量 IN 查询回填名称（不 N+1）。
    async fn to_vo_list(&self, rows: Vec<AdCampaign>) -> Result<Vec<AdCampaignVO>> {
        let mut shop_ids: Vec<i32> = rows.iter().map(|c| c.shop_id).collect();
        shop_ids.sort_unstable();
        shop_ids.dedup();
        let mut product_ids: Vec<i32> =
            rows.iter().map(|c| c.product_id).filter(|id| *id > 0).collect();
        product_ids.sort_unstable();
        product_ids.dedup();
        let shop_titles: std::collections::HashMap<i32, String> = self
            .repo()
            .shop_titles(&shop_ids)
            .await?
            .into_iter()
            .filter_map(|(id, t)| t.map(|t| (id, t)))
            .collect();
        let product_names: std::collections::HashMap<i32, String> = self
            .repo()
            .product_names(&product_ids)
            .await?
            .into_iter()
            .filter_map(|(id, t)| t.map(|t| (id, t)))
            .collect();
        Ok(rows
            .into_iter()
            .map(|c| {
                let shop_title = shop_titles.get(&c.shop_id).cloned();
                let product_name = if c.product_id > 0 {
                    product_names.get(&c.product_id).cloned()
                } else {
                    None
                };
                AdCampaignVO::new(c, shop_title, product_name)
            })
            .collect())
    }

    /// 商家「我的投放」列表（强制本店）。
    pub async fn list_for_shop(
        &self,
        shop_id: i32,
        q: &AdCampaignListQuery,
    ) -> Result<PageResult<AdCampaignVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repo()
            .paginate(Some(shop_id), q.audit_status, q.status, q.position.as_deref(), &page)
            .await?;
        Ok(PageResult::new(total, self.to_vo_list(rows).await?, &page))
    }

    /// 商家暂停（已开始的暂停不退费，恢复续投）。
    pub async fn pause(&self, shop_id: i32, id: i32) -> Result<AdCampaign> {
        let mut tx = self.repo().begin().await?;
        let mut c = self
            .repo()
            .find_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "投放单不存在"))?;
        if c.shop_id != shop_id {
            return Err(Error::Forbidden("无权操作其他店铺的投放单".into()));
        }
        let now = Self::now();
        let from = c.lifecycle(now);
        if !can_transition(from, Lifecycle::Paused) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "当前状态不允许暂停"));
        }
        c.status = STATUS_PAUSED;
        c.update_time = now;
        self.repo().update_in_tx(&mut tx, &c).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(c)
    }

    /// 商家恢复（回到待生效/投放中；已到期不可恢复）。
    pub async fn resume(&self, shop_id: i32, id: i32) -> Result<AdCampaign> {
        let mut tx = self.repo().begin().await?;
        let mut c = self
            .repo()
            .find_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "投放单不存在"))?;
        if c.shop_id != shop_id {
            return Err(Error::Forbidden("无权操作其他店铺的投放单".into()));
        }
        let now = Self::now();
        let to = if now >= c.start_time { Lifecycle::Active } else { Lifecycle::Scheduled };
        if c.lifecycle(now) != Lifecycle::Paused || !can_transition(Lifecycle::Paused, to) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "当前状态不允许恢复"));
        }
        if now >= c.end_time {
            return Err(Error::biz(SERVICE_DATA_ERROR, "投放已结束，无法恢复"));
        }
        c.status = if to == Lifecycle::Active { STATUS_ACTIVE } else { STATUS_SCHEDULED };
        c.update_time = now;
        self.repo().update_in_tx(&mut tx, &c).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(c)
    }

    // ————————————————————— 平台：审核 / 置顶 / 下架 —————————————————————

    /// 平台投放列表（可按审核状态过滤）。
    pub async fn list_for_platform(
        &self,
        q: &AdCampaignListQuery,
    ) -> Result<PageResult<AdCampaignVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repo()
            .paginate(None, q.audit_status, q.status, q.position.as_deref(), &page)
            .await?;
        Ok(PageResult::new(total, self.to_vo_list(rows).await?, &page))
    }

    /// 平台审核（照 merchant_apply.audit：必须待审，目标只能 1 通过 / 2 驳回）。
    pub async fn audit(&self, admin_id: i32, param: &AdCampaignAuditParam) -> Result<AdCampaign> {
        let id = param
            .id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "id不能为空"))?;
        let status = param
            .status
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "审核状态不正确"))?;
        if status != AUDIT_PASS && status != AUDIT_REJECT {
            return Err(Error::biz(SERVICE_DATA_ERROR, "审核状态不正确"));
        }
        let mut tx = self.repo().begin().await?;
        let mut c = self
            .repo()
            .find_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "投放单不存在"))?;
        if c.audit_status != AUDIT_PENDING {
            return Err(Error::biz(SERVICE_DATA_ERROR, "审核状态不正确"));
        }
        let now = Self::now();
        let to = if status == AUDIT_PASS { Lifecycle::Scheduled } else { Lifecycle::Rejected };
        if !can_transition(c.lifecycle(now), to) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "审核状态不正确"));
        }
        c.audit_status = status;
        c.audit_remark = param.audit_remark.clone().unwrap_or_default();
        c.audit_admin_id = admin_id;
        c.audit_time = now;
        if status == AUDIT_REJECT {
            c.status = STATUS_ENDED;
        }
        c.update_time = now;
        self.repo().update_in_tx(&mut tx, &c).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(c)
    }

    /// 平台置顶：自建一条 is_pin 的店铺投放单（amount = 0、免审、即时生效）。
    pub async fn create_pin(&self, param: &AdCampaignPinParam) -> Result<AdCampaign> {
        let shop_id = param
            .shop_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "店铺ID不能为空"))?;
        if !self.repo().shop_is_open(shop_id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, "店铺不存在或未开业"));
        }
        let settings = self.settings().await?;
        let pin_limit = settings["pinLimit"].as_i64().unwrap_or(3);
        if pin_limit <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "置顶位已关闭"));
        }
        let now = Self::now();
        let days = param.days.unwrap_or(3650).clamp(1, 3650);
        let mut campaign = AdCampaign {
            target_type: TARGET_SHOP,
            shop_id,
            position: POSITION_SHOP_STREET.to_string(),
            is_pin: 1,
            pin_sort: param.pin_sort.unwrap_or(0),
            start_time: now,
            end_time: now.saturating_add(days.saturating_mul(86_400)),
            days,
            pay_status: PAY_PAID,
            pay_channel: PAY_CHANNEL_PIN.to_string(),
            pay_time: now,
            audit_status: AUDIT_PASS,
            status: STATUS_SCHEDULED,
            add_time: now,
            update_time: now,
            ..Default::default()
        };
        let mut tx = self.repo().begin().await?;
        campaign.id = self.repo().insert_in_tx(&mut tx, &campaign).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(campaign)
    }

    /// 平台强制下架（任意非终态可下架；费用处理一期人工，见 PLAN §4.3）。
    pub async fn take_down(&self, id: i32) -> Result<AdCampaign> {
        let mut tx = self.repo().begin().await?;
        let mut c = self
            .repo()
            .find_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "投放单不存在"))?;
        let now = Self::now();
        if !can_transition(c.lifecycle(now), Lifecycle::TakenDown) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "当前状态不允许下架"));
        }
        c.status = STATUS_TAKEN_DOWN;
        c.update_time = now;
        self.repo().update_in_tx(&mut tx, &c).await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_requires_scheduled_or_active() {
        // 状态机口径在 core 已覆盖；这里锁服务层依赖的几条关键迁移
        assert!(can_transition(Lifecycle::Scheduled, Lifecycle::Paused));
        assert!(can_transition(Lifecycle::Active, Lifecycle::Paused));
        assert!(!can_transition(Lifecycle::PendingAudit, Lifecycle::Paused));
        assert!(!can_transition(Lifecycle::Ended, Lifecycle::Paused));
    }

    #[test]
    fn takedown_blocked_from_terminal_states() {
        assert!(!can_transition(Lifecycle::Ended, Lifecycle::TakenDown));
        assert!(!can_transition(Lifecycle::Rejected, Lifecycle::TakenDown));
        assert!(can_transition(Lifecycle::Active, Lifecycle::TakenDown));
        assert!(can_transition(Lifecycle::PendingAudit, Lifecycle::TakenDown));
    }
}
