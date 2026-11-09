//! `coupon` / `user_coupon` 促销实体。
//! 对应 `com.nextrade.bean.model.promotion.Coupon` 与 `...model.user.UserCoupon`。
//!
//! 字段严格对齐 `java/database/pure.sql`：
//! - `CREATE TABLE coupon`（L958）
//! - `CREATE TABLE user_coupon`（L6526）
//!
//! 注意：`user_coupon` **没有 status 列**，状态由 `order_id` / `used_time` /
//! `start_date` / `end_date` 派生，见 [`UserCouponStatus`]。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 优惠券类型：满减券。
pub const COUPON_TYPE_MONEY: i32 = 1;
/// 优惠券类型：折扣券。
pub const COUPON_TYPE_DISCOUNT: i32 = 2;

/// 使用时间方式：按领取后多少天内可用。
pub const SEND_TYPE_BY_CLAIM: i32 = 0;
/// 使用时间方式：按固定时间范围可用。
pub const SEND_TYPE_FIXED: i32 = 1;

/// 会员限制：非专享。
pub const NEW_USER_NONE: i32 = 0;
/// 会员限制：新人专享。
pub const NEW_USER_ONLY: i32 = 1;
/// 会员限制：会员专享（按等级/标记）。
pub const NEW_USER_MEMBER: i32 = 2;

/// 优惠券定义（`coupon` 表）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Coupon {
    pub coupon_id: i32,
    /// 优惠券名称
    pub coupon_name: String,
    /// 优惠券金额（满减券）
    pub coupon_money: Decimal,
    /// 优惠券折扣（折扣券，如 8.5 表示 8.5 折）
    pub coupon_discount: Decimal,
    /// 优惠券描述
    pub coupon_desc: String,
    /// 类型：1 满减券 2 折扣券
    pub coupon_type: i32,
    /// 使用范围：0 全部商品 3 指定商品 4 不包含指定商品
    pub send_range: i32,
    /// [JSON] 使用范围相关信息
    pub send_range_data: String,
    /// 最少订单金额（使用门槛）
    pub min_order_amount: Decimal,
    /// 可领取开始时间（已废弃）
    pub send_start_date: i32,
    /// 可领取结束时间（已废弃）
    pub send_end_date: i32,
    /// 使用时间方式：0 按领取后天数 1 固定时间范围
    pub send_type: i32,
    /// 多少天过期
    pub use_day: i32,
    /// 可使用开始时间（固定时间范围，秒）
    pub use_start_date: i32,
    /// 可使用结束时间（固定时间范围，秒）
    pub use_end_date: i32,
    /// 是否展示
    pub is_show: i32,
    /// 全场通用券（跨店铺）
    pub is_global: i32,
    /// 0 非专享 1 新人专享 2 会员专享
    pub is_new_user: i32,
    /// 商城点击领取
    pub enabled_click_get: i32,
    /// [JSON] 仅限会员等级
    pub limit_user_rank: Option<String>,
    /// 店铺ID
    pub shop_id: Option<i32>,
    /// 是否删除：1 已删除
    pub is_delete: Option<i32>,
    /// 限制领取数量（0 不限）
    pub limit_num: Option<i32>,
    /// 延迟天数：0 为立即生效
    pub delay_day: i32,
    /// 发放总量（0 不限）
    pub send_num: i32,
    /// 优惠金额上限
    pub max_order_amount: Decimal,
    /// 优惠单位：1 元 2 件
    pub coupon_unit: i32,
    /// 满减类型：1 满多少 2 无门槛
    pub reduce_type: i32,
    /// 添加时间（秒）
    pub add_time: Option<i32>,
}

impl Coupon {
    /// 表名（供仓储层复用）。
    pub const TABLE: &'static str = "coupon";

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

    /// 订单金额是否达到使用门槛。
    pub fn usable_for(&self, amount: Decimal) -> bool {
        // reduce_type = 2 为无门槛
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

/// 用户优惠券（`user_coupon` 表）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserCoupon {
    /// 主键（表中列名为 `id`）
    pub id: i32,
    /// 优惠券类型ID
    pub coupon_id: i32,
    /// 优惠券SN
    pub coupon_sn: String,
    /// 会员id
    pub user_id: i32,
    /// 使用时间（秒），0 表示未使用
    pub used_time: i32,
    /// 所使用的订单ID，0 表示未使用
    pub order_id: i32,
    /// 可使用的开始时间（秒）
    pub start_date: i32,
    /// 可使用的结束时间（秒）
    pub end_date: i32,
    /// 来源类型：0 普通领取 1 抽奖中奖 2 手动发放
    pub source_type: Option<i32>,
}

impl UserCoupon {
    pub const TABLE: &'static str = "user_coupon";

    /// 是否已被订单占用（已使用）。
    pub fn is_used(&self) -> bool {
        self.order_id > 0 || self.used_time > 0
    }

    /// 是否在有效期内且未被使用。
    pub fn is_usable_at(&self, now: i64) -> bool {
        !self.is_used()
            && i64::from(self.start_date) <= now
            && i64::from(self.end_date) >= now
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

/// 用户优惠券状态（对应 Java `UserCouponStatus` 枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "i32", try_from = "i32")]
pub enum UserCouponStatus {
    /// 即将过期
    ExpiringSoon = 1,
    /// 正常
    Normal = 2,
    /// 未开始
    NotStarted = 3,
    /// 已使用
    Used = 4,
    /// 已过期
    Expired = 5,
}

impl UserCouponStatus {
    /// 状态名称（对应 Java `UserCouponStatus#getDescription`）。
    pub fn name(self) -> &'static str {
        match self {
            Self::ExpiringSoon => "即将过期",
            Self::Normal => "正常",
            Self::NotStarted => "未开始",
            Self::Used => "已使用",
            Self::Expired => "已过期",
        }
    }
}

impl From<UserCouponStatus> for i32 {
    fn from(s: UserCouponStatus) -> Self {
        s as i32
    }
}

impl TryFrom<i32> for UserCouponStatus {
    type Error = String;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::ExpiringSoon),
            2 => Ok(Self::Normal),
            3 => Ok(Self::NotStarted),
            4 => Ok(Self::Used),
            5 => Ok(Self::Expired),
            other => Err(format!("非法的用户优惠券状态: {other}")),
        }
    }
}

/// 解析 JSON 整型数组；空串/`[]`/非法内容均返回空列表。
pub fn parse_json_i32_array(raw: Option<&str>) -> Vec<i32> {
    let Some(text) = raw.map(str::trim) else {
        return Vec::new();
    };
    if text.is_empty() || text == "[]" {
        return Vec::new();
    }
    serde_json::from_str::<Vec<i32>>(text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
    fn threshold_respects_no_threshold_type() {
        let mut c = coupon(SEND_TYPE_BY_CLAIM, 0, 0);
        c.min_order_amount = Decimal::from(100);
        assert!(c.usable_for(Decimal::from(100)));
        assert!(!c.usable_for(Decimal::from(99)));

        // reduce_type = 2 无门槛
        c.reduce_type = 2;
        assert!(c.usable_for(Decimal::from(0)));
    }

    #[test]
    fn coupon_status_derivation() {
        // 按领取后天数：恒为进行中
        assert_eq!(coupon(SEND_TYPE_BY_CLAIM, 0, 0).status_at(1_000), CouponStatus::PROCESSING);
        // 固定时间：未开始 / 进行中 / 已失效
        assert_eq!(coupon(SEND_TYPE_FIXED, 2_000, 3_000).status_at(1_000), CouponStatus::PENDING);
        assert_eq!(coupon(SEND_TYPE_FIXED, 500, 3_000).status_at(1_000), CouponStatus::PROCESSING);
        assert_eq!(coupon(SEND_TYPE_FIXED, 500, 900).status_at(1_000), CouponStatus::EXPIRED);
    }

    #[test]
    fn time_range_by_claim_days() {
        let mut c = coupon(SEND_TYPE_BY_CLAIM, 0, 0);
        c.delay_day = 1;
        c.use_day = 30;
        let (start, end) = c.time_range_at(0);
        assert_eq!(start, 86_400);
        assert_eq!(end, 86_400 + 30 * 86_400);
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

    #[test]
    fn json_array_parsing_is_tolerant() {
        assert_eq!(parse_json_i32_array(Some("[1,2,3]")), vec![1, 2, 3]);
        assert!(parse_json_i32_array(Some("[]")).is_empty());
        assert!(parse_json_i32_array(Some("")).is_empty());
        assert!(parse_json_i32_array(None).is_empty());
        assert!(parse_json_i32_array(Some("not-json")).is_empty());
    }

    #[test]
    fn decimal_scale_from_db() {
        // 金额使用 Decimal，避免 f64 精度问题（对应原 Java BigDecimal）
        let d = Decimal::from_str("99.99").unwrap();
        assert_eq!(d.to_string(), "99.99");
    }
}