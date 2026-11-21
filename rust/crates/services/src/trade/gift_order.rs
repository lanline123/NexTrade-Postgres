//! 送礼物订单流服务（我方新增，论坛帖 197742，用户 2026-11-21 拍板）。
//!
//! 形态与状态机（礼物子状态独立落 `order_gift` 表，不侵入订单状态枚举）：
//!
//! ```text
//! 下单(送礼物) ──► gift_status=0 待付款 ──付款成功──► 1 待领取
//!                    │                                │
//!                    │ 买家取消(未付款)                ├─ 好友领取 ─► 2 已领取 ─► 正常履约
//!                    ▼                                │
//!                 4 未付款取消                        ├─ 买家领取前取消 ─► 退款 ─► 3 已关闭
//!                                                     └─ 超时未领取(定时任务) ─► 退款 ─► 3 已关闭
//! ```
//!
//! 口径要点：
//! - 领取单位 = **整单**：一个根订单一个领取链接；跨店拆单后领取把
//!   领取人地址同步写入全部有效子单（子单是履约/退款单位，根单是
//!   付款壳），与既有拆单模型一致。
//! - token：48 位随机十六进制串，库内只存 SHA-256 摘要；明文仅在
//!   建单与「重新生成分享链接」时一次性返回买家，重新生成即作废
//!   旧链接（防旧链接外泄后仍可领取）。
//! - 价格不可见：领取页与领取人视图的 VO 刻意不含任何金额字段。
//! - 退款走既有资金链：`RefundApplyService::create_system_refund`
//!   （原路退回买家支付账户），不新造资金通道；积分/优惠券不随
//!   礼物关闭返还（与主动退款口径一致：主动退款同样不返积分券）。
//! - 售后：领取后由领取人（收货人）发起，退款单的收款归属仍为买家
//!   （见 aftersales 域对礼物订单的归属改写）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use sqlx::Row;
use nextrade_common::utils;
use nextrade_core::models::trade::gift_order::{
    gift_status, gift_status_name, GiftClaimPreviewVO, GiftInfoVO, GiftItemVO,
    GiftOrderSettings, GiftReceivedUnitVO, GiftReceivedVO, GiftShareTokenVO,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::catalog::product::ProductService;
use crate::payment::refund_apply::RefundApplyService;
use crate::trade::order::status as order_status;

/// 平台设置 biz_code（config 表一行 JSON）。
pub const GIFT_SETTINGS_CODE: &str = "giftOrderSettings";

/// order_gift 行（服务内私有投影）。
#[derive(Debug, Clone)]
pub struct GiftRow {
    pub gift_id: i32,
    pub order_id: i32,
    pub order_sn: String,
    pub buyer_user_id: i32,
    pub gift_status: i32,
    pub gift_message: String,
    pub expire_time: i32,
    pub claim_user_id: i32,
    pub claim_time: i32,
    pub close_reason: String,
}

pub struct GiftOrderService {
    repos: Arc<Repositories>,
}

impl GiftOrderService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————— 设置 —————————————————

    /// 平台礼物设置（缺行/坏 JSON 回默认：有效期 7 天）。
    pub async fn settings(&self) -> GiftOrderSettings {
        let raw = self
            .repos
            .system
            .config_basic
            .find_by_code(GIFT_SETTINGS_CODE)
            .await
            .ok()
            .flatten();
        match raw
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        {
            Some(v) => GiftOrderSettings::sanitized(&v),
            None => GiftOrderSettings::default(),
        }
    }

    /// 保存平台礼物设置（管理端专用；入参先清洗再落库）。
    pub async fn save_settings(&self, value: &serde_json::Value) -> Result<GiftOrderSettings> {
        let settings = GiftOrderSettings::sanitized(value);
        let json = serde_json::to_string(&serde_json::json!({
            "claimExpireDays": settings.claim_expire_days,
        }))
        .map_err(|e| Error::Internal(e.to_string()))?;
        self.repos
            .system
            .config_basic
            .save_code(GIFT_SETTINGS_CODE, &json)
            .await?;
        Ok(settings)
    }

    // ————————————————— 行读写 —————————————————

    async fn find_by_order_id(&self, order_id: i32) -> Result<Option<GiftRow>> {
        let row = sqlx::query(
            "SELECT gift_id, order_id, order_sn, buyer_user_id, gift_status, \
             gift_message, expire_time, claim_user_id, claim_time, close_reason \
             FROM order_gift WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.map(|r| GiftRow {
            gift_id: r.get("gift_id"),
            order_id: r.get("order_id"),
            order_sn: r.get("order_sn"),
            buyer_user_id: r.get("buyer_user_id"),
            gift_status: r.get("gift_status"),
            gift_message: r.get("gift_message"),
            expire_time: r.get("expire_time"),
            claim_user_id: r.get("claim_user_id"),
            claim_time: r.get("claim_time"),
            close_reason: r.get("close_reason"),
        }))
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<GiftRow>> {
        let hash = utils::sha256_hex(token.trim());
        let row = sqlx::query(
            "SELECT gift_id, order_id, order_sn, buyer_user_id, gift_status, \
             gift_message, expire_time, claim_user_id, claim_time, close_reason \
             FROM order_gift WHERE token_hash = $1",
        )
        .bind(hash)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.map(|r| GiftRow {
            gift_id: r.get("gift_id"),
            order_id: r.get("order_id"),
            order_sn: r.get("order_sn"),
            buyer_user_id: r.get("buyer_user_id"),
            gift_status: r.get("gift_status"),
            gift_message: r.get("gift_message"),
            expire_time: r.get("expire_time"),
            claim_user_id: r.get("claim_user_id"),
            claim_time: r.get("claim_time"),
            close_reason: r.get("close_reason"),
        }))
    }

    /// 按任意订单（根单或子单）解析其礼物行：先按自身查，再按父单查。
    pub async fn resolve_for_order(&self, order_id: i32) -> Result<Option<GiftRow>> {
        if let Some(g) = self.find_by_order_id(order_id).await? {
            return Ok(Some(g));
        }
        let parent: Option<i32> = sqlx::query("SELECT parent_order_id FROM \"order\" WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
            .and_then(|r| r.get::<Option<i32>, _>("parent_order_id"));
        match parent {
            Some(pid) if pid > 0 => self.find_by_order_id(pid).await,
            _ => Ok(None),
        }
    }

    /// 批量解析（列表页标记用）：入参为本页订单（id, parent_id），
    /// 返回 order_id → 礼物行（子单经父单命中时以子单 id 为键回填）。
    pub async fn resolve_map(
        &self,
        orders: &[(i32, Option<i32>)],
    ) -> Result<HashMap<i32, GiftRow>> {
        let mut root_ids: Vec<i32> = Vec::new();
        for (id, parent) in orders {
            root_ids.push(*id);
            if let Some(p) = parent {
                if *p > 0 {
                    root_ids.push(*p);
                }
            }
        }
        root_ids.sort_unstable();
        root_ids.dedup();
        if root_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            "SELECT gift_id, order_id, order_sn, buyer_user_id, gift_status, \
             gift_message, expire_time, claim_user_id, claim_time, close_reason \
             FROM order_gift WHERE order_id = ANY($1)",
        )
        .bind(&root_ids)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let mut by_root: HashMap<i32, GiftRow> = HashMap::new();
        for r in rows {
            let g = GiftRow {
                gift_id: r.get("gift_id"),
                order_id: r.get("order_id"),
                order_sn: r.get("order_sn"),
                buyer_user_id: r.get("buyer_user_id"),
                gift_status: r.get("gift_status"),
                gift_message: r.get("gift_message"),
                expire_time: r.get("expire_time"),
                claim_user_id: r.get("claim_user_id"),
                claim_time: r.get("claim_time"),
                close_reason: r.get("close_reason"),
            };
            by_root.insert(g.order_id, g);
        }
        let mut out = HashMap::new();
        for (id, parent) in orders {
            if let Some(g) = by_root.get(id) {
                out.insert(*id, g.clone());
            } else if let Some(p) = parent {
                if let Some(g) = by_root.get(p) {
                    out.insert(*id, g.clone());
                }
            }
        }
        Ok(out)
    }

    async fn set_status(
        &self,
        gift_id: i32,
        from: i32,
        to: i32,
        close_reason: Option<&str>,
    ) -> Result<bool> {
        let now = utils::now_seconds() as i32;
        let affected = if let Some(reason) = close_reason {
            sqlx::query(
                "UPDATE order_gift SET gift_status = $1, close_reason = $2, update_time = $3 \
                 WHERE gift_id = $4 AND gift_status = $5",
            )
            .bind(to)
            .bind(reason)
            .bind(now)
            .bind(gift_id)
            .bind(from)
            .execute(&self.repos.pool)
            .await
        } else {
            sqlx::query(
                "UPDATE order_gift SET gift_status = $1, update_time = $2 \
                 WHERE gift_id = $3 AND gift_status = $4",
            )
            .bind(to)
            .bind(now)
            .bind(gift_id)
            .bind(from)
            .execute(&self.repos.pool)
            .await
        }
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(affected.rows_affected() > 0)
    }

    // ————————————————— 建单 / 付款 —————————————————

    /// 建单事务内落礼物行（由 OrderService::create 在订单事务里调用，
    /// 保证「有订单必有礼物行」原子性）。返回领取 token 明文（仅此一次）。
    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        order_id: i32,
        order_sn: &str,
        buyer_user_id: i32,
        gift_message: &str,
        now: i32,
    ) -> Result<String> {
        let token = utils::random_hex(48);
        let hash = utils::sha256_hex(&token);
        sqlx::query(
            "INSERT INTO order_gift (order_id, order_sn, buyer_user_id, gift_status, \
             gift_message, token_hash, expire_time, claim_user_id, claim_time, \
             close_reason, add_time, update_time) \
             VALUES ($1, $2, $3, 0, $4, $5, 0, 0, 0, '', $6, $6)",
        )
        .bind(order_id)
        .bind(order_sn)
        .bind(buyer_user_id)
        .bind(gift_message)
        .bind(hash)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(token)
    }

    /// 付款成功激活（OrderService 付款钩子调用，best-effort 语义由
    /// 调用方兜底）：待付款 → 待领取，并按平台设置起算领取有效期。
    /// 幂等：已激活/已领取/已关闭的行直接跳过。
    pub async fn activate_on_paid(&self, order_id: i32) -> Result<()> {
        let Some(gift) = self.find_by_order_id(order_id).await? else {
            return Ok(());
        };
        if gift.gift_status != gift_status::PENDING_PAYMENT {
            return Ok(());
        }
        let days = self.settings().await.claim_expire_days;
        let now = utils::now_seconds() as i32;
        let expire = now + days * 86_400;
        sqlx::query(
            "UPDATE order_gift SET gift_status = 1, expire_time = $1, update_time = $2 \
             WHERE gift_id = $3 AND gift_status = 0",
        )
        .bind(expire)
        .bind(now)
        .bind(gift.gift_id)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 未付款取消联动（OrderService::cancel 成功后调用）：待付款 → 已取消。
    pub async fn mark_canceled_unpaid(&self, order_id: i32) -> Result<()> {
        if let Some(gift) = self.find_by_order_id(order_id).await? {
            if gift.gift_status == gift_status::PENDING_PAYMENT {
                let _ = self
                    .set_status(gift.gift_id, gift_status::PENDING_PAYMENT, gift_status::CANCELED_UNPAID, None)
                    .await?;
            }
        }
        Ok(())
    }

    // ————————————————— 买家侧 —————————————————

    /// 买家订单详情的礼物面板数据。
    pub async fn buyer_info(&self, order_id: i32, user_id: i32) -> Result<Option<GiftInfoVO>> {
        let Some(gift) = self.resolve_for_order(order_id).await? else {
            return Ok(None);
        };
        if gift.buyer_user_id != user_id {
            return Ok(None);
        }
        Ok(Some(self.to_info_vo(&gift).await))
    }

    async fn to_info_vo(&self, gift: &GiftRow) -> GiftInfoVO {
        let claim_name = if gift.claim_user_id > 0 {
            Some(mask_user_name(
                self.user_display_name(gift.claim_user_id).await,
            ))
        } else {
            None
        };
        GiftInfoVO {
            order_id: gift.order_id,
            gift_status: gift.gift_status,
            gift_status_name: gift_status_name(gift.gift_status).to_string(),
            gift_message: gift.gift_message.clone(),
            expire_time: (gift.expire_time > 0).then(|| fmt_time(gift.expire_time)),
            claim_user_name_masked: claim_name,
            claim_time: (gift.claim_time > 0).then(|| fmt_time(gift.claim_time)),
            close_reason: gift.close_reason.clone(),
        }
    }

    /// 重新生成分享 token（旧链接立即作废）。仅买家本人、且礼物仍在
    /// 待付款/待领取时可生成（付款后分享是常态；待付款时生成的是
    /// 付款后才生效的链接）。
    pub async fn regenerate_share_token(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<GiftShareTokenVO> {
        let gift = self
            .find_by_order_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("该订单不是送礼物订单".into()))?;
        if gift.buyer_user_id != user_id {
            return Err(Error::Forbidden("无权操作该订单".into()));
        }
        if !matches!(
            gift.gift_status,
            gift_status::PENDING_PAYMENT | gift_status::PENDING_CLAIM
        ) {
            return Err(Error::Conflict("礼物已领取或已关闭，不能再生成分享链接".into()));
        }
        let token = utils::random_hex(48);
        let hash = utils::sha256_hex(&token);
        let now = utils::now_seconds() as i32;
        sqlx::query("UPDATE order_gift SET token_hash = $1, update_time = $2 WHERE gift_id = $3")
            .bind(hash)
            .bind(now)
            .bind(gift.gift_id)
            .execute(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(GiftShareTokenVO {
            claim_path: format!("/gift/claim?token={token}"),
            token,
        })
    }

    /// 买家取消礼物订单：
    /// - 待付款：转交订单取消链（回滚库存），礼物行转「未付款取消」；
    /// - 待领取：走系统退款关闭（见 `refund_and_close`）。
    pub async fn buyer_cancel(
        &self,
        order_id: i32,
        user_id: i32,
        order_service: &crate::trade::order::OrderService,
    ) -> Result<()> {
        let gift = self
            .find_by_order_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("该订单不是送礼物订单".into()))?;
        if gift.buyer_user_id != user_id {
            return Err(Error::Forbidden("无权操作该订单".into()));
        }
        match gift.gift_status {
            gift_status::PENDING_PAYMENT => {
                order_service
                    .cancel(order_id, Some(user_id), Some("买家取消送礼物订单"))
                    .await?;
                // cancel 成功路径已联动 mark_canceled_unpaid；此处兜底再推一次（幂等）
                self.mark_canceled_unpaid(order_id).await?;
                Ok(())
            }
            gift_status::PENDING_CLAIM => {
                self.refund_and_close(&gift, "买家在好友领取前取消礼物订单，款项原路退回")
                    .await
            }
            gift_status::CLAIMED => Err(Error::Conflict(
                "礼物已被领取，不能取消；如需售后请由领取人发起".into(),
            )),
            _ => Err(Error::Conflict("礼物订单已关闭".into())),
        }
    }

    // ————————————————— 领取 —————————————————

    /// 履约单元解析：有效子单（拆单后）或根单自身。
    async fn fulfillment_units(&self, root_order_id: i32) -> Result<Vec<i32>> {
        let rows = sqlx::query(
            "SELECT order_id FROM \"order\" WHERE parent_order_id = $1 AND is_valid = 1 \
             ORDER BY order_id",
        )
        .bind(root_order_id)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let units: Vec<i32> = rows.iter().map(|r| r.get("order_id")).collect();
        if units.is_empty() {
            Ok(vec![root_order_id])
        } else {
            Ok(units)
        }
    }

    async fn unit_items(&self, unit_ids: &[i32]) -> Result<Vec<GiftItemVO>> {
        let mut items = Vec::new();
        for unit in unit_ids {
            let rows = self.repos.trade.order_admin.find_items(*unit).await?;
            for it in rows {
                items.push(GiftItemVO {
                    item_id: it.item_id,
                    order_id: it.order_id,
                    product_id: it.product_id,
                    product_name: it.product_name.clone(),
                    pic_thumb: it.pic_thumb.clone(),
                    sku_data: it.sku_data.clone(),
                    quantity: it.quantity,
                });
            }
        }
        Ok(items)
    }

    async fn user_display_name(&self, user_id: i32) -> String {
        match self.repos.member.user.find_by_id(user_id).await {
            Ok(Some(u)) => u
                .nickname
                .filter(|n| !n.trim().is_empty())
                .unwrap_or(u.username),
            _ => String::new(),
        }
    }

    /// 领取页预览（可匿名）：只下发商品/赠言/状态，**无任何金额**。
    /// 已过期的待领取礼物在预览时即触发超时关闭（与定时任务同路径，
    /// 保证好友打开过期链接立刻看到「已关闭」而不是等下一轮扫描）。
    pub async fn claim_preview(
        &self,
        token: &str,
        viewer_user_id: Option<i32>,
    ) -> Result<GiftClaimPreviewVO> {
        let mut gift = self
            .find_by_token(token)
            .await?
            .ok_or_else(|| Error::NotFound("领取链接无效或已被买家重新生成".into()))?;
        let now = utils::now_seconds() as i32;
        if gift.gift_status == gift_status::PENDING_CLAIM
            && gift.expire_time > 0
            && now > gift.expire_time
        {
            let _ = self
                .refund_and_close(&gift, "超过领取有效期未领取，款项原路退回买家")
                .await;
            gift = self
                .find_by_order_id(gift.order_id)
                .await?
                .unwrap_or(gift);
        }
        let units = self.fulfillment_units(gift.order_id).await?;
        let items = self.unit_items(&units).await?;
        let claimed_units = if gift.gift_status == gift_status::CLAIMED {
            self.received_units(&units).await?
        } else {
            Vec::new()
        };
        let can_claim = gift.gift_status == gift_status::PENDING_CLAIM
            && viewer_user_id.is_some_and(|uid| uid != gift.buyer_user_id);
        Ok(GiftClaimPreviewVO {
            gift_status: gift.gift_status,
            gift_status_name: gift_status_name(gift.gift_status).to_string(),
            gift_message: gift.gift_message.clone(),
            buyer_name_masked: mask_user_name(self.user_display_name(gift.buyer_user_id).await),
            expire_time: (gift.expire_time > 0).then(|| fmt_time(gift.expire_time)),
            expired: gift.gift_status == gift_status::PENDING_CLAIM
                && gift.expire_time > 0
                && now > gift.expire_time,
            can_claim,
            items,
            units: claimed_units,
        })
    }

    /// 领取（必须登录）：校验状态/有效期/领取人资格，把领取人所选
    /// 地址快照写入根单与全部有效子单，礼物转「已领取」，订单自此
    /// 进入正常履约链（商家按子单发货给领取人）。
    pub async fn claim(&self, token: &str, user_id: i32, address_id: i32) -> Result<GiftInfoVO> {
        let gift = self
            .find_by_token(token)
            .await?
            .ok_or_else(|| Error::NotFound("领取链接无效或已被买家重新生成".into()))?;
        if gift.buyer_user_id == user_id {
            return Err(Error::biz(400, "不能领取自己送出的礼物".to_string()));
        }
        if gift.gift_status == gift_status::CLAIMED {
            return Err(Error::Conflict("这份礼物已经被领取啦".into()));
        }
        if gift.gift_status != gift_status::PENDING_CLAIM {
            return Err(Error::Conflict("这份礼物当前不可领取".into()));
        }
        let now = utils::now_seconds() as i32;
        if gift.expire_time > 0 && now > gift.expire_time {
            let _ = self
                .refund_and_close(&gift, "超过领取有效期未领取，款项原路退回买家")
                .await;
            return Err(Error::Conflict("领取已过期，款项将原路退回送礼人".into()));
        }
        // 地址必须属于领取人本人（防越权用他人地址簿条目）
        let address = self
            .repos
            .member
            .user_address_client
            .find_by_id(address_id)
            .await?
            .ok_or_else(|| Error::NotFound("收货地址不存在".into()))?;
        if address.user_id != user_id {
            return Err(Error::Forbidden("只能使用自己的收货地址领取".into()));
        }
        // 状态抢占：先把礼物行置已领取（条件更新防并发双领），再落地址
        let claimed = sqlx::query(
            "UPDATE order_gift SET gift_status = 2, claim_user_id = $1, claim_time = $2, \
             update_time = $2 WHERE gift_id = $3 AND gift_status = 1",
        )
        .bind(user_id)
        .bind(now)
        .bind(gift.gift_id)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        if claimed.rows_affected() == 0 {
            return Err(Error::Conflict("这份礼物已经被领取啦".into()));
        }
        // 地址快照同步：根单（买家详情/父单展示）+ 全部有效子单（履约）
        let mut targets = vec![gift.order_id];
        targets.extend(self.fulfillment_units(gift.order_id).await?);
        targets.sort_unstable();
        targets.dedup();
        for oid in targets {
            sqlx::query(
                "UPDATE \"order\" SET consignee = $1, address = $2, region_ids = $3, \
                 region_names = $4, mobile = $5, email = $6 WHERE order_id = $7",
            )
            .bind(&address.consignee)
            .bind(&address.address)
            .bind(&address.region_ids)
            .bind(&address.region_names)
            .bind(&address.mobile)
            .bind(address.email.clone().unwrap_or_default())
            .bind(oid)
            .execute(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
            let _ = self
                .repos
                .trade
                .user_order
                .insert_order_log(oid, &gift.order_sn, user_id, 0, "礼物已被领取，收货地址已确认", now)
                .await;
        }
        let gift = self.find_by_order_id(gift.order_id).await?.unwrap_or(gift);
        Ok(self.to_info_vo(&gift).await)
    }

    async fn received_units(
        &self,
        unit_ids: &[i32],
    ) -> Result<Vec<GiftReceivedUnitVO>> {
        let mut out = Vec::new();
        for unit in unit_ids {
            let Some(order) = self.repos.trade.order.find_by_id(*unit).await? else {
                continue;
            };
            let items = self.unit_items(&[*unit]).await?;
            out.push(GiftReceivedUnitVO {
                order_id: order.order_id,
                order_sn: order.order_sn.clone(),
                shop_id: order.shop_id,
                order_status: order.order_status,
                order_status_name: order_status_name(order.order_status),
                tracking_no: order.tracking_no.clone(),
                logistics_name: order.logistics_name.clone(),
                items,
            });
        }
        Ok(out)
    }

    /// 「我收到的礼物」列表（领取人视角，无价格）。
    pub async fn received_list(&self, user_id: i32) -> Result<Vec<GiftReceivedVO>> {
        let rows = sqlx::query(
            "SELECT gift_id, order_id, order_sn, buyer_user_id, gift_status, \
             gift_message, expire_time, claim_user_id, claim_time, close_reason \
             FROM order_gift WHERE claim_user_id = $1 ORDER BY claim_time DESC, gift_id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            let gift = GiftRow {
                gift_id: r.get("gift_id"),
                order_id: r.get("order_id"),
                order_sn: r.get("order_sn"),
                buyer_user_id: r.get("buyer_user_id"),
                gift_status: r.get("gift_status"),
                gift_message: r.get("gift_message"),
                expire_time: r.get("expire_time"),
                claim_user_id: r.get("claim_user_id"),
                claim_time: r.get("claim_time"),
                close_reason: r.get("close_reason"),
            };
            let units = self.fulfillment_units(gift.order_id).await?;
            let unit_vos = self.received_units(&units).await?;
            out.push(GiftReceivedVO {
                order_id: gift.order_id,
                order_sn: gift.order_sn.clone(),
                gift_status: gift.gift_status,
                gift_status_name: gift_status_name(gift.gift_status).to_string(),
                gift_message: gift.gift_message.clone(),
                buyer_name_masked: mask_user_name(
                    self.user_display_name(gift.buyer_user_id).await,
                ),
                claim_time: (gift.claim_time > 0).then(|| fmt_time(gift.claim_time)),
                units: unit_vos,
            });
        }
        Ok(out)
    }

    // ————————————————— 退款关闭 / 到期任务 —————————————————

    /// 退款关闭（超时未领取、买家领取前取消共用）：
    /// 逐履约单元（子单）走系统退款链全额退剩余可退额 → 单元订单
    /// 待发货 → 已取消 + 回滚库存 → 礼物行转「已关闭」并注明原因。
    /// 幂等：礼物行状态条件更新抢占，重复调用直接返回。
    pub async fn refund_and_close(&self, gift: &GiftRow, reason: &str) -> Result<()> {
        let moved = self
            .set_status(
                gift.gift_id,
                gift_status::PENDING_CLAIM,
                gift_status::CLOSED_REFUNDED,
                Some(reason),
            )
            .await?;
        if !moved {
            return Ok(());
        }
        let units = self.fulfillment_units(gift.order_id).await?;
        let refund_svc = RefundApplyService::new(self.repos.clone());
        let product = ProductService::new(self.repos.clone(), None);
        let now = utils::now_seconds() as i32;
        for unit_id in units {
            let Some(order) = self.repos.trade.order.find_by_id(unit_id).await? else {
                continue;
            };
            // 退款（仅已付款且有可退余额时；0 元单/已全退单跳过资金步）
            if order.pay_status == 2 {
                let occupied = self
                    .repos
                    .payment
                    .refund_apply
                    .sum_occupied_by_order(unit_id)
                    .await
                    .unwrap_or(Decimal::ZERO);
                let refundable = (order.paid_amount - occupied).max(Decimal::ZERO);
                if refundable > Decimal::ZERO {
                    if let Err(e) = refund_svc
                        .create_system_refund(unit_id, refundable, reason)
                        .await
                    {
                        // 资金步失败不阻断关单状态落定：退款单缺失会由
                        // 财务侧按关闭原因人工兜底（日志留痕便于对账）
                        tracing::error!(
                            "礼物订单退款失败 gift={} unit={unit_id}: {e}",
                            gift.gift_id
                        );
                    }
                }
            }
            // 关单 + 回滚库存（仅仍处待发货的单元）
            if order.order_status == order_status::PAID {
                let ok = self
                    .repos
                    .trade
                    .order
                    .transition_status(unit_id, &[order_status::PAID], order_status::CANCELED)
                    .await
                    .unwrap_or(false);
                if ok {
                    sqlx::query("UPDATE \"order\" SET cancel_remark = $1 WHERE order_id = $2")
                        .bind(reason)
                        .bind(unit_id)
                        .execute(&self.repos.pool)
                        .await
                        .map_err(|e| Error::Database(e.to_string()))?;
                    let items = self.repos.trade.order.find_items(unit_id).await?;
                    for item in items {
                        if let Err(e) = product
                            .restore_stock(item.product_id, Some(item.sku_id), item.quantity)
                            .await
                        {
                            tracing::error!(
                                "礼物关单回滚库存失败 unit={unit_id} product={}: {e}",
                                item.product_id
                            );
                        }
                    }
                }
            }
            let _ = self
                .repos
                .trade
                .user_order
                .insert_order_log(unit_id, &order.order_sn, order.user_id, order.shop_id, reason, now)
                .await;
        }
        // 根单备注（未拆单时根单即单元，已在上面写过；拆单时补根单壳）
        sqlx::query("UPDATE \"order\" SET cancel_remark = $1 WHERE order_id = $2")
            .bind(reason)
            .bind(gift.order_id)
            .execute(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 到期扫描（定时任务调用）：待领取且已过期的礼物逐笔退款关闭。
    /// 返回本轮处理笔数；单笔失败只记日志不中断整轮。
    pub async fn run_expire_scan(&self, now: i32) -> Result<usize> {
        let rows = sqlx::query(
            "SELECT gift_id, order_id, order_sn, buyer_user_id, gift_status, \
             gift_message, expire_time, claim_user_id, claim_time, close_reason \
             FROM order_gift WHERE gift_status = 1 AND expire_time > 0 AND expire_time < $1 \
             ORDER BY gift_id LIMIT 100",
        )
        .bind(now)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let mut done = 0usize;
        for r in rows {
            let gift = GiftRow {
                gift_id: r.get("gift_id"),
                order_id: r.get("order_id"),
                order_sn: r.get("order_sn"),
                buyer_user_id: r.get("buyer_user_id"),
                gift_status: r.get("gift_status"),
                gift_message: r.get("gift_message"),
                expire_time: r.get("expire_time"),
                claim_user_id: r.get("claim_user_id"),
                claim_time: r.get("claim_time"),
                close_reason: r.get("close_reason"),
            };
            match self
                .refund_and_close(&gift, "超过领取有效期未领取，款项原路退回买家")
                .await
            {
                Ok(()) => done += 1,
                Err(e) => {
                    tracing::warn!("礼物到期退款关闭失败 gift={}: {e}", gift.gift_id);
                }
            }
        }
        Ok(done)
    }
}

/// 发货前守卫（商家发货 / 买家侧发货入口共用）：订单所属礼物仍在
/// 「待领取」时禁止发货（地址还是占位空快照，发出去必然错）。
pub async fn ensure_deliverable(repos: Arc<Repositories>, order_id: i32) -> Result<()> {
    let svc = GiftOrderService::new(repos);
    if let Some(gift) = svc.resolve_for_order(order_id).await? {
        if gift.gift_status == gift_status::PENDING_CLAIM {
            return Err(Error::biz(
                400,
                "送礼物订单待好友领取，领取后方可发货".to_string(),
            ));
        }
        if gift.gift_status == gift_status::PENDING_PAYMENT {
            return Err(Error::biz(400, "送礼物订单尚未付款".to_string()));
        }
    }
    Ok(())
}

/// 昵称脱敏：首字 + `***`（空名回「好友」）。
#[must_use]
pub fn mask_user_name(name: String) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "好友".to_string();
    }
    let first: String = trimmed.chars().take(1).collect();
    format!("{first}***")
}

/// 订单状态文案（与买家侧列表同口径的最小集）。
fn order_status_name(status: i32) -> String {
    match status {
        0 => "待付款",
        1 => "待发货",
        2 => "待收货",
        3 => "已取消",
        4 => "已失效",
        5 => "已完成",
        _ => "",
    }
    .to_string()
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（本地时区，与订单 VO 同口径）。
fn fmt_time(secs: i32) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_name_keeps_first_char_only() {
        assert_eq!(mask_user_name("王小明".into()), "王***");
        assert_eq!(mask_user_name(String::new()), "好友");
        assert_eq!(mask_user_name(" A ".into()), "A***");
    }

    #[test]
    fn source_pins_gift_flow_guards() {
        let src = include_str!("gift_order.rs");
        for needle in [
            "create_system_refund",
            "PENDING_CLAIM",
            "sha256_hex",
            "sum_occupied_by_order",
            "restore_stock",
        ] {
            assert!(src.contains(needle), "送礼物服务缺少关键环节: {needle}");
        }
    }
}
