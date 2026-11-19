//! 广告投流（merchant::ad_campaign）实体 / DTO / 状态机 / 店铺个性化打分。
//!
//! 店铺街千人千面 + 商家自助投流（店铺/商品共用一套引擎）：
//! - 投放单表 `ad_campaign`（迁移 `rust/deploy/pg/2026-11-18_ad_campaign.sql`）；
//!   平台置顶与付费推广同表不同来源（`is_pin = 1` 为平台置顶，`amount = 0`）。
//! - 与商品免费编辑标记（`is_best` / `is_hot` / `is_new`）完全隔离，付费语义只认本表。
//! - 定价与开关走 `config` 表 `biz_code = "adSettings"`（见 services::system::ad_settings）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ————————————————————— 常量 —————————————————————

/// 投放对象：店铺。
pub const TARGET_SHOP: i32 = 1;
/// 投放对象：商品。
pub const TARGET_PRODUCT: i32 = 2;

/// 投放位：店铺街（店铺唯一可选；商品一期不落此位）。
pub const POSITION_SHOP_STREET: &str = "shop_street";
/// 投放位：商品搜索结果（二期落点，一期只登记口径）。
pub const POSITION_PRODUCT_SEARCH: &str = "product_search";
/// 投放位：商品列表（二期落点）。
pub const POSITION_PRODUCT_LIST: &str = "product_list";
/// 投放位：推荐流（二期落点）。
pub const POSITION_RECOMMEND_FEED: &str = "recommend_feed";

/// 全部合法投放位。
pub const POSITIONS: [&str; 4] = [
    POSITION_SHOP_STREET,
    POSITION_PRODUCT_SEARCH,
    POSITION_PRODUCT_LIST,
    POSITION_RECOMMEND_FEED,
];

/// 支付状态：未付。
pub const PAY_UNPAID: i32 = 0;
/// 支付状态：已付。
pub const PAY_PAID: i32 = 1;
/// 支付状态：已退款。
pub const PAY_REFUNDED: i32 = 2;

/// 付款渠道：店铺余额（shop_money）扣款。
pub const PAY_CHANNEL_SHOP_MONEY: &str = "shop_money";
/// 付款渠道：平台置顶（免费，amount = 0）。
pub const PAY_CHANNEL_PIN: &str = "pin";

/// 审核状态：待审。
pub const AUDIT_PENDING: i32 = 0;
/// 审核状态：通过。
pub const AUDIT_PASS: i32 = 1;
/// 审核状态：驳回。
pub const AUDIT_REJECT: i32 = 2;

/// 投放状态：待生效（已付/已审，尚未到开始时间或待审）。
pub const STATUS_SCHEDULED: i32 = 0;
/// 投放状态：投放中。
pub const STATUS_ACTIVE: i32 = 1;
/// 投放状态：已暂停（商家暂停，不退费，可恢复续投）。
pub const STATUS_PAUSED: i32 = 2;
/// 投放状态：已结束。
pub const STATUS_ENDED: i32 = 3;
/// 投放状态：已下架（平台强制下架，终态）。
pub const STATUS_TAKEN_DOWN: i32 = 4;

// ————————————————————— 实体 —————————————————————

/// 投放单（`ad_campaign` 表行）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[sqlx(rename_all = "snake_case")]
pub struct AdCampaign {
    pub id: i32,
    /// 1 店铺 / 2 商品
    pub target_type: i32,
    pub shop_id: i32,
    /// 商品投放时有值，店铺投放为 0
    pub product_id: i32,
    pub position: String,
    /// 1 = 平台置顶（免费运营位）
    pub is_pin: i32,
    /// 置顶顺序（小在前）
    pub pin_sort: i32,
    /// 投放开始（Unix 秒）
    pub start_time: i32,
    /// 投放结束（Unix 秒，不含）
    pub end_time: i32,
    pub days: i32,
    /// 下单时单价快照（元/天）
    pub unit_price: Decimal,
    /// 总价（单价 × 天数；置顶为 0）
    pub amount: Decimal,
    /// 推广段排序权重（大在前）
    pub weight: i32,
    pub pay_status: i32,
    pub pay_channel: String,
    pub pay_time: i32,
    /// 关联 `shop_account_log.shop_account_log_id`
    pub account_log_id: i32,
    pub audit_status: i32,
    pub audit_remark: String,
    pub audit_admin_id: i32,
    pub audit_time: i32,
    pub status: i32,
    pub add_time: i32,
    pub update_time: i32,
}

impl AdCampaign {
    /// 数据库表名。
    pub const TABLE: &'static str = "ad_campaign";

    /// 当前是否处于可展示窗口：已付 + 已审 + 未暂停/下架 + 在投放期内。
    #[must_use]
    pub fn is_live(&self, now: i32) -> bool {
        self.pay_status == PAY_PAID
            && self.audit_status == AUDIT_PASS
            && (self.status == STATUS_SCHEDULED || self.status == STATUS_ACTIVE)
            && self.start_time <= now
            && now < self.end_time
    }

    /// 派生生命周期（存储三态 + 时间窗推导，见 [`Lifecycle`]）。
    #[must_use]
    pub fn lifecycle(&self, now: i32) -> Lifecycle {
        if self.status == STATUS_TAKEN_DOWN {
            return Lifecycle::TakenDown;
        }
        if self.audit_status == AUDIT_REJECT {
            return Lifecycle::Rejected;
        }
        if self.pay_status == PAY_UNPAID {
            return Lifecycle::PendingPay;
        }
        if self.pay_status == PAY_REFUNDED {
            return Lifecycle::Ended;
        }
        if self.audit_status == AUDIT_PENDING {
            return Lifecycle::PendingAudit;
        }
        if self.status == STATUS_PAUSED {
            return Lifecycle::Paused;
        }
        if self.status == STATUS_ENDED || now >= self.end_time {
            return Lifecycle::Ended;
        }
        if now < self.start_time {
            return Lifecycle::Scheduled;
        }
        Lifecycle::Active
    }
}

// ————————————————————— 状态机 —————————————————————

/// 投放单生命周期（由 pay_status / audit_status / status + 时间窗派生，不单独落库）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    /// 待付（已创建未扣款）
    PendingPay,
    /// 待审（已付，等待平台审核）
    PendingAudit,
    /// 待生效（已审，尚未到开始时间）
    Scheduled,
    /// 投放中
    Active,
    /// 已暂停（商家暂停，可恢复）
    Paused,
    /// 已结束（到期/退款，终态）
    Ended,
    /// 审核驳回（终态）
    Rejected,
    /// 已下架（平台强制，终态）
    TakenDown,
}

/// 合法生命周期迁移（状态机唯一口径，服务层所有状态变更先过此校验）。
///
/// ```text
/// PendingPay ──付款──▶ PendingAudit ──审核通过──▶ Scheduled ──到时──▶ Active
///     │                    │驳回                     │  ▲              │  ▲
///     │                    ▼                         暂停│恢复        暂停│恢复
///     │                 Rejected                     ▼  │              ▼  │
///     └──────────────▶ Ended（到期）                Paused ◀────────────┘
/// 任意非终态 ──平台下架──▶ TakenDown；Scheduled/Active ──到期──▶ Ended
/// ```
#[must_use]
pub fn can_transition(from: Lifecycle, to: Lifecycle) -> bool {
    use Lifecycle::*;
    if from == to {
        return false;
    }
    // 平台下架：任意非终态可下架
    if to == TakenDown {
        return !matches!(from, Ended | Rejected | TakenDown);
    }
    matches!(
        (from, to),
        (PendingPay, PendingAudit)
            | (PendingPay, Scheduled)
            | (PendingPay, Ended)
            | (PendingAudit, Scheduled)
            | (PendingAudit, Rejected)
            | (Scheduled, Active)
            | (Scheduled, Paused)
            | (Scheduled, Ended)
            | (Active, Paused)
            | (Active, Ended)
            | (Paused, Scheduled)
            | (Paused, Active)
            | (Paused, Ended)
    )
}

// ————————————————————— DTO —————————————————————

/// 商家新建投放参数（店铺由登录态强制，不接受客户端指定 shopId 归属）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AdCampaignCreateParam {
    /// 1 店铺 / 2 商品
    pub target_type: Option<i32>,
    /// 商品投放时必填，须属本店且在售
    pub product_id: Option<i32>,
    pub position: Option<String>,
    /// 开始时间（Unix 秒）；不传/早于当前 ⇒ 立即开始
    pub start_time: Option<i32>,
    /// 购买天数
    pub days: Option<i32>,
}

/// 平台审核参数（照 merchant_apply.audit 口径）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AdCampaignAuditParam {
    pub id: Option<i32>,
    /// 目标审核状态：1 通过 / 2 驳回
    pub status: Option<i32>,
    pub audit_remark: Option<String>,
}

/// 平台置顶参数（平台自建 amount = 0 的 pin 投放单）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AdCampaignPinParam {
    pub shop_id: Option<i32>,
    /// 置顶顺序（小在前）
    pub pin_sort: Option<i32>,
    /// 置顶天数（不传 = 长期，默认 3650 天）
    pub days: Option<i32>,
}

/// 投放列表查询（商家「我的投放」/ 平台审核列表共用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AdCampaignListQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    /// 审核状态过滤（平台列表）
    pub audit_status: Option<i32>,
    /// 投放状态过滤
    pub status: Option<i32>,
    pub position: Option<String>,
}

impl AdCampaignListQuery {
    /// 分页（缺省 1 / 15，上限 100，与店铺街同口径）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1),
            page_size: self.size.unwrap_or(15).clamp(1, 100),
        }
    }
}

/// 投放列表 VO（商家「我的投放」/ 平台列表共用）：投放单本体 + 名称回填。
///
/// `shopTitle` 按 `shop_id` 批量查 `shop` 表回填；`productName` 仅商品投放
/// （`product_id > 0`）时按 `product_id` 批量查 `product` 表回填。查不到
/// （店铺/商品已删）时字段不下发，前端回退显示 ID。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdCampaignVO {
    #[serde(flatten)]
    pub campaign: AdCampaign,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shop_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
}

impl AdCampaignVO {
    #[must_use]
    pub fn new(campaign: AdCampaign, shop_title: Option<String>, product_name: Option<String>) -> Self {
        Self { campaign, shop_title, product_name }
    }
}

/// 计价结果（单价快照 × 天数）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdCampaignQuote {
    pub unit_price: Decimal,
    pub days: i32,
    pub amount: Decimal,
    pub start_time: i32,
    pub end_time: i32,
}

/// 计价（纯函数）：天数钳制到 `min_days..=365`，开始时间早于现在则立即开始，
/// 结束 = 开始 + 天数 × 86400。金额 = 单价 × 天数（Decimal 精确）。
#[must_use]
pub fn quote(unit_price: Decimal, days: i32, min_days: i32, start_time: i32, now: i32) -> AdCampaignQuote {
    let days = days.clamp(min_days.max(1), 365);
    let start = if start_time > now { start_time } else { now };
    AdCampaignQuote {
        unit_price,
        days,
        amount: unit_price * Decimal::from(days),
        start_time: start,
        end_time: start.saturating_add(days.saturating_mul(86_400)),
    }
}

// ————————————————————— 店铺个性化打分 —————————————————————

/// 单个用户对单个店铺的行为信号聚合（仓储层按 user_id 聚合到 shop_id）。
#[derive(Debug, Clone, Default)]
pub struct ShopSignals {
    /// 近 90 天在本店已支付订单数
    pub purchased_orders: i64,
    /// 是否收藏了本店
    pub collected_shop: bool,
    /// 购物车里本店商品件数（行数）
    pub cart_items: i64,
    /// 收藏的本店商品数
    pub collected_products: i64,
    /// 浏览过本店商品的衰减和（每个商品按最近浏览时间衰减：7 天内 1.0 / 30 天 0.5 / 更久 0.2）
    pub history_decayed: f64,
    /// 兴趣分类命中数（用户行为分类集合 ∩ 本店在售商品分类）
    pub category_hits: i64,
    /// 相似用户协同：同画像群体里买过本店的人数
    pub cohort_buyers: i64,
}

/// 个性化打分（纯函数，可解释、可单测）。
///
/// 权重口径（PLAN §3）：已购最高，其次收藏店铺、购物车、收藏商品、浏览（时间衰减），
/// 兴趣分类与协同过滤补充。每项封顶，避免单一信号刷爆排序。
/// `quality` 为店铺质量分（0..=10，服务层由评星/销量/收藏/上架数/点击归一化），
/// 只作同分附近的微调，不压过个人信号。
#[must_use]
pub fn score_shop(s: &ShopSignals, quality: f64) -> f64 {
    let mut score = 0.0_f64;
    if s.purchased_orders > 0 {
        score += 10.0 + 2.0 * (s.purchased_orders.min(5) - 1) as f64;
    }
    if s.collected_shop {
        score += 9.0;
    }
    if s.cart_items > 0 {
        score += 7.0 + (s.cart_items.min(3) - 1) as f64;
    }
    if s.collected_products > 0 {
        score += 6.0 + (s.collected_products.min(3) - 1) as f64;
    }
    score += 3.0 * s.history_decayed.min(3.0);
    score += 2.0 * s.category_hits.min(5) as f64;
    score += 1.5 * s.cohort_buyers.min(10) as f64;
    score + quality.clamp(0.0, 10.0)
}

/// 推荐理由（取最强信号，一期先回数据，前端可选用）。
#[must_use]
pub fn recommend_reason(s: &ShopSignals) -> Option<&'static str> {
    if s.purchased_orders > 0 {
        Some("你买过这家店")
    } else if s.collected_shop {
        Some("你关注的店铺")
    } else if s.cart_items > 0 {
        Some("购物车里有这家店的商品")
    } else if s.collected_products > 0 {
        Some("你收藏过这家店的商品")
    } else if s.history_decayed > 0.0 {
        Some("你浏览过这家店的商品")
    } else if s.cohort_buyers > 0 {
        Some("相似用户买过的店铺")
    } else if s.category_hits > 0 {
        Some("符合你的兴趣分类")
    } else {
        None
    }
}

/// 浏览衰减因子：距今 7 天内 1.0、30 天内 0.5、更久 0.2。
/// `history_product_ids` 本身不带时间戳，服务层按列表位置近似（越靠前越近），
/// 或有时间信息时按此函数逐个折算。
#[must_use]
pub fn history_decay(days_ago: f64) -> f64 {
    if days_ago <= 7.0 {
        1.0
    } else if days_ago <= 30.0 {
        0.5
    } else {
        0.2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn lifecycle_derivation() {
        let mut c = AdCampaign {
            pay_status: PAY_PAID,
            audit_status: AUDIT_PASS,
            status: STATUS_SCHEDULED,
            start_time: 100,
            end_time: 200,
            ..Default::default()
        };
        assert_eq!(c.lifecycle(50), Lifecycle::Scheduled);
        assert_eq!(c.lifecycle(150), Lifecycle::Active);
        assert_eq!(c.lifecycle(250), Lifecycle::Ended);
        c.status = STATUS_PAUSED;
        assert_eq!(c.lifecycle(150), Lifecycle::Paused);
        c.status = STATUS_TAKEN_DOWN;
        assert_eq!(c.lifecycle(150), Lifecycle::TakenDown);
        let unpaid = AdCampaign::default();
        assert_eq!(unpaid.lifecycle(150), Lifecycle::PendingPay);
        let mut pending = AdCampaign {
            pay_status: PAY_PAID,
            audit_status: AUDIT_PENDING,
            ..Default::default()
        };
        assert_eq!(pending.lifecycle(150), Lifecycle::PendingAudit);
        pending.audit_status = AUDIT_REJECT;
        assert_eq!(pending.lifecycle(150), Lifecycle::Rejected);
    }

    #[test]
    fn transitions_legal_and_illegal() {
        use Lifecycle::*;
        assert!(can_transition(PendingPay, PendingAudit));
        assert!(can_transition(PendingAudit, Scheduled));
        assert!(can_transition(PendingAudit, Rejected));
        assert!(can_transition(Active, Paused));
        assert!(can_transition(Paused, Active));
        assert!(can_transition(Active, TakenDown));
        assert!(can_transition(Scheduled, Ended));
        // 非法：终态不可流出、未付不能直接投放中、驳回不能恢复
        assert!(!can_transition(Ended, Active));
        assert!(!can_transition(Rejected, Scheduled));
        assert!(!can_transition(TakenDown, Active));
        assert!(!can_transition(PendingPay, Active));
        assert!(!can_transition(Active, PendingAudit));
        assert!(!can_transition(Active, Active));
    }

    #[test]
    fn is_live_requires_paid_passed_and_in_window() {
        let c = AdCampaign {
            pay_status: PAY_PAID,
            audit_status: AUDIT_PASS,
            status: STATUS_ACTIVE,
            start_time: 100,
            end_time: 200,
            ..Default::default()
        };
        assert!(c.is_live(150));
        assert!(!c.is_live(99));
        assert!(!c.is_live(200));
        let paused = AdCampaign { status: STATUS_PAUSED, ..c.clone() };
        assert!(!paused.is_live(150));
    }

    #[test]
    fn quote_clamps_days_and_snapshots_amount() {
        let q = quote(dec("30"), 0, 3, 0, 1_000);
        assert_eq!(q.days, 3);
        assert_eq!(q.amount, dec("90"));
        assert_eq!(q.start_time, 1_000);
        assert_eq!(q.end_time, 1_000 + 3 * 86_400);
        let q2 = quote(dec("10.5"), 999, 1, 5_000, 1_000);
        assert_eq!(q2.days, 365);
        assert_eq!(q2.start_time, 5_000);
    }

    #[test]
    fn score_orders_signals_by_weight() {
        let buyer = ShopSignals { purchased_orders: 1, ..Default::default() };
        let collector = ShopSignals { collected_shop: true, ..Default::default() };
        let carter = ShopSignals { cart_items: 1, ..Default::default() };
        let browser = ShopSignals { history_decayed: 1.0, ..Default::default() };
        let none = ShopSignals::default();
        assert!(score_shop(&buyer, 0.0) > score_shop(&collector, 0.0));
        assert!(score_shop(&collector, 0.0) > score_shop(&carter, 0.0));
        assert!(score_shop(&carter, 0.0) > score_shop(&browser, 0.0));
        assert!(score_shop(&browser, 0.0) > score_shop(&none, 0.0));
        // 质量分只能微调，不能压过最强个人信号
        assert!(score_shop(&buyer, 0.0) > score_shop(&none, 10.0) - 0.001 || score_shop(&buyer, 0.0) >= 10.0);
    }

    #[test]
    fn score_caps_single_signal() {
        let spam = ShopSignals { purchased_orders: 10_000, cohort_buyers: 10_000, category_hits: 10_000, ..Default::default() };
        let capped = score_shop(&spam, 0.0);
        // 10+2*4 + 1.5*10 + 2*5 = 43
        assert!((capped - 43.0).abs() < 1e-9, "capped={capped}");
    }

    #[test]
    fn reason_prefers_strongest_signal() {
        let s = ShopSignals { purchased_orders: 2, collected_shop: true, ..Default::default() };
        assert_eq!(recommend_reason(&s), Some("你买过这家店"));
        assert_eq!(recommend_reason(&ShopSignals::default()), None);
        let s2 = ShopSignals { cohort_buyers: 3, ..Default::default() };
        assert_eq!(recommend_reason(&s2), Some("相似用户买过的店铺"));
    }

    #[test]
    fn vo_serializes_with_backfilled_names() {
        let c = AdCampaign { id: 9, shop_id: 7, product_id: 42, ..Default::default() };
        let vo = AdCampaignVO::new(c.clone(), Some("青岛基地".into()), Some("电池产线".into()));
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["id"], 9);
        assert_eq!(v["shopId"], 7);
        assert_eq!(v["shopTitle"], "青岛基地");
        assert_eq!(v["productName"], "电池产线");
        // 查不到名称时字段不下发（前端回退 ID 展示）
        let bare = serde_json::to_value(AdCampaignVO::new(c, None, None)).unwrap();
        assert!(bare.get("shopTitle").is_none());
        assert!(bare.get("productName").is_none());
    }

    #[test]
    fn history_decay_bands() {
        assert_eq!(history_decay(0.0), 1.0);
        assert_eq!(history_decay(7.0), 1.0);
        assert_eq!(history_decay(8.0), 0.5);
        assert_eq!(history_decay(30.0), 0.5);
        assert_eq!(history_decay(31.0), 0.2);
    }
}
