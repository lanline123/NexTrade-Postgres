//! 实体业务方法（扩展 impl）。
//!
//! **本文件为手工维护**，与自动生成的 `entities/*.rs`（纯表结构）分离：
//! - `entities/*.rs`：由 `tools/schema_parser.py` 从数据库 schema 生成，**勿手改**
//! - 本文件：承载依赖业务语义的派生方法（是否可售、库存可用量、状态文案等）
//!
//! 这样重新生成实体时不会覆盖业务逻辑，也不会因表结构变化产生字段名漂移。
//!
//! 注意：Rust 的 orphan rule 要求 inherent impl 必须与类型同 crate，
//! 故本文件位于 `core` 而非 `infrastructure`。

use super::{
    AdminUser, Article, ArticleCategory, Cart, Category, Coupon, Order, Product, ProductSku, User,
    UserCoupon,
};
use crate::models::marketing::coupon::UserCouponStatus;

/// 优惠券会员限制：非专享。
pub const NEW_USER_NONE: i32 = 0;
/// 优惠券会员限制：新人专享。
pub const NEW_USER_ONLY: i32 = 1;
/// 优惠券会员限制：会员专享（按等级白名单）。
pub const NEW_USER_MEMBER: i32 = 2;

/// 优惠券使用时间方式：按领取后多少天内可用。
pub const SEND_TYPE_BY_CLAIM: i32 = 0;
/// 优惠券使用时间方式：按固定时间范围可用。
pub const SEND_TYPE_FIXED: i32 = 1;

/// 后台管理员：状态判定。
impl AdminUser {
    /// 是否为启用中的后台管理员。
    ///
    /// 注意库中 `is_using` 语义为「是否停用」：1=停用，0=启用（见建表注释）。
    pub fn is_active_admin(&self) -> bool {
        self.admin_type == "admin" && self.is_using == 0 && self.is_deleted == 0
    }

    /// 管理员ID。
    pub fn id(&self) -> i32 {
        self.admin_id
    }
}

/// 商品分类：可见性与树结构。
impl Category {
    /// 分类树根节点的 parent_id 约定值。
    pub const ROOT_PARENT: i32 = 0;

    /// 前台是否可见（`is_show` = 1）。
    pub fn is_visible(&self) -> bool {
        self.is_show == 1
    }

    /// 是否为顶级分类。
    pub fn is_root(&self) -> bool {
        self.parent_id == Self::ROOT_PARENT
    }
}


/// 商品：是否上架可售。
impl Product {
    /// 上架且未删除。
    pub fn is_on_sale(&self) -> bool {
        self.product_status == 1 && self.is_delete == 0
    }

    /// 是否包邮。
    pub fn is_free_shipping(&self) -> bool {
        self.free_shipping == 1
    }

    /// 当前售价（促销价优先于原价）。
    pub fn current_price(&self) -> rust_decimal::Decimal {
        if self.is_promote == 1 && self.promote_price > rust_decimal::Decimal::ZERO {
            self.promote_price
        } else {
            self.product_price
        }
    }

    /// 限购数量（0 表示不限购）。
    pub fn purchase_limit(&self) -> Option<i32> {
        if self.limit_number > 0 {
            Some(self.limit_number)
        } else {
            None
        }
    }
}

/// SKU：可售库存。
impl ProductSku {
    /// 启用状态下的可用库存（负数归零）。
    pub fn available_stock(&self) -> i64 {
        if self.is_enabled == 1 {
            i64::from(self.sku_stock.unwrap_or(0)).max(0)
        } else {
            0
        }
    }

    /// SKU 是否启用。
    pub fn is_available(&self) -> bool {
        self.is_enabled == 1
    }
}

/// 订单：状态判定与文案。
impl Order {
    /// 是否已支付。
    pub fn is_paid(&self) -> bool {
        self.pay_status == 2
    }

    /// 是否可取消（未支付、有效、未删除、状态为待付款）。
    pub fn is_cancellable(&self) -> bool {
        !self.is_paid()
            && self.is_valid == 1
            && self.is_del == 0
            && matches!(self.order_status, 0 | 10)
    }

    /// 状态中文名（与原后台语义一致）。
    pub fn status_label(&self) -> &'static str {
        match self.order_status {
            0 => "待付款",
            10 => "待发货",
            20 => "已发货",
            30 => "已完成",
            40 => "已取消",
            50 => "已关闭",
            _ => "未知状态",
        }
    }
}

/// 购物车行：小计与勾选。
impl Cart {
    /// 行小计 = 单价 × 数量。
    pub fn subtotal(&self) -> rust_decimal::Decimal {
        self.original_price * rust_decimal::Decimal::from(self.quantity.max(0))
    }

    /// 是否已勾选结算。
    pub fn is_checked(&self) -> bool {
        self.is_checked == 1
    }
}

/// 优惠券定义：领取/使用校验与派生状态。
///
/// 字段按真实库可空性建模：`is_delete` / `limit_num` / `shop_id` / `add_time`
/// 为可空列，其余为 NOT NULL 裸类型。
impl Coupon {
    /// 是否已被软删除。
    pub fn is_deleted(&self) -> bool {
        self.is_delete.unwrap_or(0) == 1
    }

    /// 每人限领数量，0 表示不限。
    pub fn claim_limit_per_user(&self) -> i32 {
        self.limit_num.unwrap_or(0).max(0)
    }

    /// 发放总量，0 表示不限。
    pub fn total_supply(&self) -> i32 {
        self.send_num.max(0)
    }

    /// 订单金额是否达到使用门槛（`reduce_type = 2` 为无门槛）。
    pub fn usable_for(&self, amount: rust_decimal::Decimal) -> bool {
        self.reduce_type == 2 || amount >= self.min_order_amount
    }

    /// 派生券状态：1 进行中 2 待生效 3 已失效（对应 Java `computeCouponStatus`）。
    pub fn status_at(&self, now: i64) -> i32 {
        // send_type = 0：按领取后多少天可用，无固定结束时间，视为进行中
        if self.send_type == SEND_TYPE_BY_CLAIM {
            return CouponStatus::PROCESSING;
        }
        let start = i64::from(self.use_start_date);
        let end = i64::from(self.use_end_date);
        if start > now {
            return CouponStatus::PENDING;
        }
        if end > 0 && now > end {
            return CouponStatus::EXPIRED;
        }
        CouponStatus::PROCESSING
    }

    /// 当前是否处于可领取/可用状态（进行中且未删除）。
    pub fn is_active_at(&self, now: i64) -> bool {
        !self.is_deleted() && self.status_at(now) == CouponStatus::PROCESSING
    }

    /// 计算领取后的券有效期（秒），对应 Java `buildCouponTimeRange`。
    ///
    /// - `send_type > 0`：使用固定时间范围
    /// - 否则：`now + delay_day` 起算，`use_day` 天后失效
    pub fn time_range_at(&self, now: i64) -> (i64, i64) {
        if self.send_type > SEND_TYPE_BY_CLAIM {
            return (i64::from(self.use_start_date), i64::from(self.use_end_date));
        }
        let start = now + i64::from(self.delay_day) * 86_400;
        let end = start + i64::from(self.use_day) * 86_400;
        (start, end)
    }

    /// 解析 `limit_user_rank`（JSON 数组）为会员等级 ID 列表。
    pub fn limited_rank_ids(&self) -> Vec<i32> {
        parse_json_i32_array(self.limit_user_rank.as_deref())
    }
}

/// 优惠券状态常量（对应 Java `CouponServiceImpl` 的 1/2/3 语义）。
pub struct CouponStatus;

impl CouponStatus {
    /// 进行中
    pub const PROCESSING: i32 = 1;
    /// 待生效
    pub const PENDING: i32 = 2;
    /// 已失效
    pub const EXPIRED: i32 = 3;

    /// 状态名称。
    pub fn name(status: i32) -> &'static str {
        match status {
            Self::PENDING => "待生效",
            Self::EXPIRED => "已失效",
            _ => "进行中",
        }
    }
}

/// 用户优惠券：占用判定与派生状态。
impl UserCoupon {
    /// 是否已被订单占用（已使用）。
    pub fn is_used(&self) -> bool {
        self.order_id > 0 || self.used_time > 0
    }

    /// 是否在有效期内且未被使用。
    pub fn is_usable_at(&self, now: i64) -> bool {
        !self.is_used() && i64::from(self.start_date) <= now && i64::from(self.end_date) >= now
    }

    /// 派生状态（对应 Java `UserCouponServiceImpl#getStatus`）。
    ///
    /// 注意：原 Java 实现以 `start_date` 判定「即将过期」（领取后 3 天内），
    /// 此处保持行为一致以确保接口兼容。
    pub fn status_at(&self, now: i64) -> i32 {
        if self.is_used() {
            return UserCouponStatus::Used as i32;
        }
        let start = i64::from(self.start_date);
        let end = i64::from(self.end_date);
        if end < now {
            return UserCouponStatus::Expired as i32;
        }
        if start > now {
            return UserCouponStatus::NotStarted as i32;
        }
        if start < now && now - start < 3 * 86_400 {
            return UserCouponStatus::ExpiringSoon as i32;
        }
        UserCouponStatus::Normal as i32
    }
}

/// 解析 JSON 整型数组；空串 / `[]` / 非法内容均返回空列表。
pub fn parse_json_i32_array(raw: Option<&str>) -> Vec<i32> {
    let Some(text) = raw.map(str::trim) else {
        return Vec::new();
    };
    if text.is_empty() || text == "[]" {
        return Vec::new();
    }
    serde_json::from_str::<Vec<i32>>(text).unwrap_or_default()
}

/// 文章：可见性与标记判定。
impl Article {
    /// 未分类文章的 `article_category_id` 取值。
    pub const UNCATEGORIZED: i32 = 0;

    /// 是否在前台显示（`is_show` 缺省视为显示，与表默认值 1 一致）。
    pub fn is_visible(&self) -> bool {
        self.is_show.unwrap_or(1) == 1
    }

    /// 是否为热门文章。
    pub fn is_hot_article(&self) -> bool {
        self.is_hot == 1
    }

    /// 是否置顶。
    pub fn is_top_article(&self) -> bool {
        self.is_top == 1
    }

    /// 是否未分类。
    pub fn is_uncategorized(&self) -> bool {
        self.article_category_id == Self::UNCATEGORIZED
    }
}

/// 文章分类：类型与树结构。
impl ArticleCategory {
    /// 分类类型：普通分类。
    pub const TYPE_NORMAL: i32 = 1;
    /// 分类类型：帮助分类。
    pub const TYPE_HELP: i32 = 2;

    /// 顶级分类的 `parent_id` 取值。
    pub const ROOT_PARENT: i32 = 0;

    /// 是否根（顶级）分类。
    pub fn is_root(&self) -> bool {
        self.parent_id == Self::ROOT_PARENT
    }

    /// 是否为帮助分类。
    pub fn is_help(&self) -> bool {
        self.category_type == Self::TYPE_HELP
    }
}

/// 会员：登录与展示。
impl User {
    /// 是否可登录（状态=1）。
    pub fn can_login(&self) -> bool {
        self.status.unwrap_or(0) == 1
    }

    /// 可用余额（负数归零）。
    pub fn available_balance(&self) -> rust_decimal::Decimal {
        self.balance.max(rust_decimal::Decimal::ZERO)
    }

    /// 展示名：昵称 > 用户名 > 手机号 > 兜底。
    pub fn display_name(&self) -> String {
        self.nickname
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(self.username.clone()).filter(|s| !s.is_empty()))
            .or_else(|| Some(self.mobile.clone()).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| format!("user_{}", self.user_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn product_on_sale() {
        let p = Product {
            product_status: 1,
            is_delete: 0,
            ..Default::default()
        };
        assert!(p.is_on_sale());
        let p2 = Product {
            product_status: 0,
            is_delete: 0,
            ..Default::default()
        };
        assert!(!p2.is_on_sale());
    }

    #[test]
    fn product_current_price_prefers_promote() {
        let p = Product {
            is_promote: 1,
            promote_price: Decimal::from_str("9.90").unwrap(),
            product_price: Decimal::from_str("19.90").unwrap(),
            ..Default::default()
        };
        assert_eq!(p.current_price(), Decimal::from_str("9.90").unwrap());
    }

    #[test]
    fn product_purchase_limit_zero_means_unlimited() {
        let p = Product {
            limit_number: 0,
            ..Default::default()
        };
        assert!(p.purchase_limit().is_none());
        let p2 = Product {
            limit_number: 3,
            ..Default::default()
        };
        assert_eq!(p2.purchase_limit(), Some(3));
    }

    #[test]
    fn sku_stock_never_negative() {
        let s = ProductSku {
            sku_stock: Some(-5),
            is_enabled: 1,
            ..Default::default()
        };
        assert_eq!(s.available_stock(), 0);
        let disabled = ProductSku {
            sku_stock: Some(10),
            is_enabled: 0,
            ..Default::default()
        };
        assert_eq!(disabled.available_stock(), 0);
    }

    #[test]
    fn order_cancel_rule() {
        let o = Order {
            pay_status: 1,
            is_valid: 1,
            is_del: 0,
            order_status: 0,
            ..Default::default()
        };
        assert!(o.is_cancellable());
        let paid = Order {
            pay_status: 2,
            is_valid: 1,
            is_del: 0,
            order_status: 10,
            ..Default::default()
        };
        assert!(!paid.is_cancellable());
    }

    #[test]
    fn order_status_labels() {
        let o = Order {
            order_status: 20,
            ..Default::default()
        };
        assert_eq!(o.status_label(), "已发货");
    }

    #[test]
    fn cart_subtotal() {
        let c = Cart {
            original_price: Decimal::from_str("9.90").unwrap(),
            quantity: 3,
            ..Default::default()
        };
        assert_eq!(c.subtotal(), Decimal::from_str("29.70").unwrap());
    }

    #[test]
    fn admin_active_rule() {
        let a = AdminUser {
            admin_type: "admin".into(),
            is_using: 0,
            is_deleted: 0,
            ..Default::default()
        };
        assert!(a.is_active_admin());
        let disabled = AdminUser {
            admin_type: "admin".into(),
            is_using: 1,
            is_deleted: 0,
            ..Default::default()
        };
        assert!(!disabled.is_active_admin());
    }

    #[test]
    fn category_visibility_and_root() {
        let c = Category {
            parent_id: 0,
            is_show: 1,
            ..Default::default()
        };
        assert!(c.is_root() && c.is_visible());
        let child = Category {
            parent_id: 5,
            is_show: 0,
            ..Default::default()
        };
        assert!(!child.is_root() && !child.is_visible());
    }

    #[test]
    fn user_display_name_fallback_chain() {
        let u = User {
            user_id: 7,
            nickname: Some("小明".into()),
            username: "ming".into(),
            ..Default::default()
        };
        assert_eq!(u.display_name(), "小明");

        let u2 = User {
            user_id: 8,
            username: "ming".into(),
            ..Default::default()
        };
        assert_eq!(u2.display_name(), "ming");

        let u3 = User {
            user_id: 9,
            ..Default::default()
        };
        assert_eq!(u3.display_name(), "user_9");
    }

    #[test]
    fn user_balance_floor_zero() {
        let u = User {
            balance: Decimal::from_str("-1.00").unwrap(),
            ..Default::default()
        };
        assert_eq!(u.available_balance(), Decimal::ZERO);
    }

    // ————————————————————— 优惠券 —————————————————————

    fn coupon(send_type: i32, start: i32, end: i32) -> Coupon {
        Coupon {
            coupon_id: 1,
            send_type,
            use_start_date: start,
            use_end_date: end,
            reduce_type: 1,
            ..Default::default()
        }
    }

    #[test]
    fn coupon_threshold_respects_no_threshold_type() {
        let mut c = coupon(SEND_TYPE_BY_CLAIM, 0, 0);
        c.min_order_amount = Decimal::from(100);
        assert!(c.usable_for(Decimal::from(100)));
        assert!(!c.usable_for(Decimal::from(99)));

        // reduce_type = 2 无门槛
        c.reduce_type = 2;
        assert!(c.usable_for(Decimal::ZERO));
    }

    #[test]
    fn coupon_status_derivation() {
        // 按领取后天数：恒为进行中
        assert_eq!(
            coupon(SEND_TYPE_BY_CLAIM, 0, 0).status_at(1_000),
            CouponStatus::PROCESSING
        );
        // 固定时间：未开始 / 进行中 / 已失效
        assert_eq!(
            coupon(SEND_TYPE_FIXED, 2_000, 3_000).status_at(1_000),
            CouponStatus::PENDING
        );
        assert_eq!(
            coupon(SEND_TYPE_FIXED, 500, 3_000).status_at(1_000),
            CouponStatus::PROCESSING
        );
        assert_eq!(
            coupon(SEND_TYPE_FIXED, 500, 900).status_at(1_000),
            CouponStatus::EXPIRED
        );
        assert_eq!(CouponStatus::name(CouponStatus::PENDING), "待生效");
        assert_eq!(CouponStatus::name(CouponStatus::EXPIRED), "已失效");
        assert_eq!(CouponStatus::name(CouponStatus::PROCESSING), "进行中");
    }

    #[test]
    fn coupon_limits_default_to_unlimited() {
        // is_delete / limit_num 可空：NULL 视为未删除 / 不限领
        let c = Coupon::default();
        assert!(!c.is_deleted());
        assert_eq!(c.claim_limit_per_user(), 0);
        assert_eq!(c.total_supply(), 0);

        let deleted = Coupon {
            is_delete: Some(1),
            limit_num: Some(-5),
            send_num: -1,
            ..Default::default()
        };
        assert!(deleted.is_deleted());
        // 负数归零
        assert_eq!(deleted.claim_limit_per_user(), 0);
        assert_eq!(deleted.total_supply(), 0);
    }

    #[test]
    fn coupon_time_range_by_claim_days() {
        let mut c = coupon(SEND_TYPE_BY_CLAIM, 0, 0);
        c.delay_day = 1;
        c.use_day = 30;
        let (start, end) = c.time_range_at(0);
        assert_eq!(start, 86_400);
        assert_eq!(end, 86_400 + 30 * 86_400);

        // 固定时间范围：直接取列值
        let fixed = coupon(SEND_TYPE_FIXED, 100, 200);
        assert_eq!(fixed.time_range_at(0), (100, 200));
    }

    #[test]
    fn coupon_limited_rank_ids_parses_json() {
        let c = Coupon {
            limit_user_rank: Some("[1,3]".into()),
            ..Default::default()
        };
        assert_eq!(c.limited_rank_ids(), vec![1, 3]);

        let empty = Coupon {
            limit_user_rank: None,
            ..Default::default()
        };
        assert!(empty.limited_rank_ids().is_empty());
    }

    #[test]
    fn user_coupon_status_derivation() {
        let now = 1_000_000;
        let mut uc = UserCoupon {
            start_date: 0,
            end_date: 2_000_000,
            ..Default::default()
        };
        // 已使用优先
        uc.order_id = 7;
        assert_eq!(uc.status_at(now), UserCouponStatus::Used as i32);

        // 已过期
        uc.order_id = 0;
        uc.end_date = 999;
        assert_eq!(uc.status_at(now), UserCouponStatus::Expired as i32);

        // 未开始
        uc.end_date = 2_000_000;
        uc.start_date = 2_000_000;
        assert_eq!(uc.status_at(now), UserCouponStatus::NotStarted as i32);

        // 正常
        uc.start_date = 0;
        assert_eq!(uc.status_at(now), UserCouponStatus::Normal as i32);
    }

    // ————————————————————— 文章 —————————————————————

    #[test]
    fn article_visibility_defaults_to_shown() {
        // 列可空且表默认 1：NULL 视为显示
        assert!(Article::default().is_visible());
        let hidden = Article {
            is_show: Some(0),
            ..Default::default()
        };
        assert!(!hidden.is_visible());
    }

    #[test]
    fn article_flags_and_uncategorized() {
        let a = Article {
            is_hot: 1,
            is_top: 1,
            article_category_id: Article::UNCATEGORIZED,
            ..Default::default()
        };
        assert!(a.is_hot_article());
        assert!(a.is_top_article());
        assert!(a.is_uncategorized());

        let categorized = Article {
            article_category_id: 7,
            ..Default::default()
        };
        assert!(!categorized.is_uncategorized());
    }

    #[test]
    fn article_category_root_and_type() {
        let root = ArticleCategory {
            parent_id: ArticleCategory::ROOT_PARENT,
            category_type: ArticleCategory::TYPE_NORMAL,
            ..Default::default()
        };
        assert!(root.is_root());
        assert!(!root.is_help());

        let help_child = ArticleCategory {
            parent_id: 3,
            category_type: ArticleCategory::TYPE_HELP,
            ..Default::default()
        };
        assert!(!help_child.is_root());
        assert!(help_child.is_help());
    }
}