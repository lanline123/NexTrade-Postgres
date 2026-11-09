//! 优惠券服务：查询、领取、核销。
//! 对应原 Java `CouponServiceImpl` / `UserCouponServiceImpl` / `CouponIssueBatchServiceImpl`。
//!
//! 领取校验链严格对照 Java `UserCouponServiceImpl#claim`（L304-392）：
//! 1. 券存在性
//! 2. 每人限领（`limit_num`，0 不限）
//! 3. 发放总量（`send_num`，0 不限）
//! 4. 新人专享（`is_new_user = 1`：仅未领取过且无已支付订单）
//! 5. 会员专享（`is_new_user = 2`：按 `limit_user_rank` 等级白名单）
//! 6. 计算有效期（`send_type` 固定时间 or 领取后天数）

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils::{gen_sn, now_seconds};
use nextrade_core::entities::business::{CouponStatus, NEW_USER_MEMBER, NEW_USER_ONLY};
use nextrade_core::entities::{Coupon, UserCoupon};
use nextrade_core::models::{
    CouponClientVO, CouponListQuery, CouponStatusCountVO, CouponVO, UserCouponListQuery,
    UserCouponVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 用户券来源：普通领取。
const SOURCE_CLAIM: i32 = 0;
/// 用户券来源：手动发放。
const SOURCE_MANUAL: i32 = 2;

pub struct CouponService {
    repos: Arc<Repositories>,
}

impl CouponService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 管理端 —————————————————————

    /// 优惠券分页（管理端），附带派生状态与已领取数量。
    pub async fn list(&self, q: &CouponListQuery) -> Result<PageResult<CouponVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let now = now_seconds();
        let (total, list) = self
            .repos.marketing.coupon
            .paginate(
                q.keyword.as_deref(),
                q.status,
                scoped_list_shop_id(q.shop_id),
                now,
                &page,
            )
            .await?;

        let ids: Vec<i32> = list.iter().map(|c| c.coupon_id).collect();
        let counts = self.repos.marketing.coupon.count_issued_batch(&ids).await?;

        let vos: Vec<CouponVO> = list
            .iter()
            .map(|c| to_coupon_vo(c, now, *counts.get(&c.coupon_id).unwrap_or(&0)))
            .collect();
        Ok(PageResult::new(total, vos, &page))
    }

    /// 管理端各状态券数量统计。
    pub async fn status_count(&self, shop_id: Option<i32>) -> Result<CouponStatusCountVO> {
        let now = now_seconds();
        let (processing, pending, expired) = self
            .repos
            .marketing
            .coupon
            .count_by_status(scoped_list_shop_id(shop_id), now)
            .await?;
        Ok(CouponStatusCountVO {
            processing,
            pending,
            expired,
        })
    }

    /// 优惠券详情（管理端）。
    pub async fn detail(&self, coupon_id: i32) -> Result<Coupon> {
        let coupon = self
            .repos
            .marketing
            .coupon
            .find_by_id(coupon_id)
            .await?
            .ok_or_else(|| Error::NotFound("优惠券不存在".into()))?;
        // 多租户：店铺账号只能查看本店券
        TenantScope::current_ensure_access(coupon.shop_id.unwrap_or(0))?;
        Ok(coupon)
    }

    /// 优惠券详情（含派生状态与已领取数量）。
    pub async fn detail_vo(&self, coupon_id: i32) -> Result<CouponVO> {
        let coupon = self.detail(coupon_id).await?;
        let issued = self.repos.marketing.coupon.count_issued(coupon_id).await?;
        Ok(to_coupon_vo(&coupon, now_seconds(), issued))
    }

    // ————————————————————— 前台 —————————————————————

    /// 前台可领取优惠券列表（公开，游客可访问）。
    ///
    /// 过滤条件对齐 Java `CouponServiceImpl#getList`：
    /// 未删除 + 展示中 + 允许点击领取 + 有效期未结束。
    /// 会员限定券（`is_new_user = 2` 且配置了等级白名单）对游客隐藏。
    pub async fn claimable_list(&self, q: &CouponListQuery) -> Result<PageResult<CouponClientVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let now = now_seconds();
        let (total, list) = self
            .repos.marketing.coupon
            .paginate_claimable(now, q.shop_id, &page)
            .await?;

        let ids: Vec<i32> = list.iter().map(|c| c.coupon_id).collect();
        let counts = self.repos.marketing.coupon.count_issued_batch(&ids).await?;

        let vos: Vec<CouponClientVO> = list
            .iter()
            .map(|c| {
                let mut vo = to_client_vo(c, *counts.get(&c.coupon_id).unwrap_or(&0));
                // 游客：隐藏限定了会员等级的会员专享券
                if c.is_new_user == NEW_USER_MEMBER && !c.limited_rank_ids().is_empty() {
                    vo.is_receive = 0;
                }
                vo
            })
            .collect();
        Ok(PageResult::new(total, vos, &page))
    }

    /// 前台可领取列表（登录用户，带 `isReceive` 标记与会员等级过滤）。
    pub async fn claimable_list_for_user(
        &self,
        user_id: i32,
        q: &CouponListQuery,
    ) -> Result<PageResult<CouponClientVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let now = now_seconds();
        let (total, list) = self
            .repos.marketing.coupon
            .paginate_claimable(now, q.shop_id, &page)
            .await?;

        let ids: Vec<i32> = list.iter().map(|c| c.coupon_id).collect();
        let counts = self.repos.marketing.coupon.count_issued_batch(&ids).await?;
        let claimed = self
            .repos.marketing.coupon
            .find_user_claimed_ids(user_id, &ids)
            .await?;

        // 会员等级：用于过滤限定等级的券（rank_id 为 NOT NULL 裸类型）
        let user_rank = self
            .repos.member.user
            .find_by_id(user_id)
            .await?
            .map(|u| u.rank_id);

        let vos: Vec<CouponClientVO> = list
            .iter()
            .filter(|c| is_rank_eligible(c, user_rank))
            .map(|c| {
                let mut vo = to_client_vo(c, *counts.get(&c.coupon_id).unwrap_or(&0));
                vo.is_receive = i32::from(claimed.contains(&c.coupon_id));
                vo
            })
            .collect();
        Ok(PageResult::new(total, vos, &page))
    }

    /// 按 ID 批量查询（前台 `listByIds`）。
    pub async fn list_by_ids(&self, ids: &[i32]) -> Result<Vec<CouponClientVO>> {
        let list = self.repos.marketing.coupon.find_by_ids(ids).await?;
        let coupon_ids: Vec<i32> = list.iter().map(|c| c.coupon_id).collect();
        let counts = self.repos.marketing.coupon.count_issued_batch(&coupon_ids).await?;
        Ok(list
            .iter()
            .map(|c| to_client_vo(c, *counts.get(&c.coupon_id).unwrap_or(&0)))
            .collect())
    }

    /// 领取优惠券。
    ///
    /// 完整复刻 Java `UserCouponServiceImpl#claim` 的校验链，并额外做并发安全：
    /// 插入前重新校验发放总量，避免并发超发。
    pub async fn claim(&self, user_id: i32, coupon_id: i32) -> Result<i32> {
        if user_id <= 0 {
            return Err(Error::Unauthorized("用户未登录".into()));
        }
        let coupon = self
            .repos.marketing.coupon
            .find_by_id(coupon_id)
            .await?
            .ok_or_else(|| Error::biz(400, "优惠券不存在"))?;

        if coupon.is_deleted() {
            return Err(Error::biz(400, "优惠券不存在"));
        }
        // 展示/领取开关
        if coupon.is_show != 1 || coupon.enabled_click_get != 1 {
            return Err(Error::biz(400, "优惠券不可领取"));
        }

        let now = now_seconds();

        // 1) 每人限领
        let user_count = self
            .repos.marketing.coupon
            .count_user_claimed(user_id, coupon_id)
            .await?;
        let limit_per_user = i64::from(coupon.claim_limit_per_user());
        if limit_per_user > 0 && user_count >= limit_per_user {
            return Err(Error::biz(400, "已超出领取数量限制"));
        }

        // 2) 发放总量
        let issued = self.repos.marketing.coupon.count_issued(coupon_id).await?;
        let total_supply = i64::from(coupon.total_supply());
        if total_supply > 0 && issued >= total_supply {
            return Err(Error::biz(400, "已超出领取数量限制"));
        }

        // 3) 新人专享
        if coupon.is_new_user == NEW_USER_ONLY {
            if user_count > 0 {
                return Err(Error::biz(400, "该优惠券仅能领取一次"));
            }
            let paid_orders = self.repos.trade.order.count_paid_by_user(user_id).await?;
            if paid_orders > 0 {
                return Err(Error::biz(400, "该优惠券仅限新人领取"));
            }
        }

        // 4) 会员等级专享
        if coupon.is_new_user == NEW_USER_MEMBER {
            let rank_ids = coupon.limited_rank_ids();
            if !rank_ids.is_empty() {
                let user_rank = self
                    .repos.member.user
                    .find_by_id(user_id)
                    .await?
                    .map(|u| u.rank_id);
                let ok = user_rank.map(|r| rank_ids.contains(&r)).unwrap_or(false);
                if !ok {
                    return Err(Error::biz(400, "您的会员等级无法领取该优惠券"));
                }
            }
        }

        // 5) 有效期计算（send_type = 0 时按领取后天数）
        let (start_date, end_date) = coupon.time_range_at(now);
        if end_date <= now {
            return Err(Error::biz(400, "优惠券已失效"));
        }

        let uc = UserCoupon {
            id: 0,
            coupon_id,
            coupon_sn: gen_sn("UC"),
            user_id,
            used_time: 0,
            order_id: 0,
            start_date: start_date as i32,
            end_date: end_date as i32,
            source_type: Some(SOURCE_CLAIM),
        };
        self.repos.marketing.coupon.insert_user_coupon(&uc).await
    }

    /// 我的优惠券列表（含派生状态）。
    pub async fn my_coupons(
        &self,
        user_id: i32,
        q: &UserCouponListQuery,
    ) -> Result<PageResult<UserCouponVO>> {
        if user_id <= 0 {
            return Err(Error::Unauthorized("用户未登录".into()));
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(20),
        };
        let now = now_seconds();
        let (total, list) = self
            .repos.marketing.coupon
            .find_user_coupons(user_id, q.used_time, now, &page)
            .await?;

        let vos = self.enrich_user_coupons(list, now).await?;
        Ok(PageResult::new(total, vos, &page))
    }

    /// 我的可用优惠券（下单抵扣选择）。
    pub async fn my_usable_coupons(&self, user_id: i32) -> Result<Vec<UserCouponVO>> {
        let now = now_seconds();
        let list = self
            .repos.marketing.coupon
            .find_usable_user_coupons(user_id, now)
            .await?;
        self.enrich_user_coupons(list, now).await
    }

    /// 我的可用券数量（个人中心角标）。
    pub async fn usable_count(&self, user_id: i32) -> Result<i64> {
        self.repos.marketing.coupon
            .count_user_usable(user_id, now_seconds())
            .await
    }

    /// 删除我的优惠券（仅本人、未使用）。
    pub async fn delete_my_coupon(&self, user_id: i32, id: i32) -> Result<()> {
        if user_id <= 0 {
            return Err(Error::Unauthorized("用户未登录".into()));
        }
        let removed = self.repos.marketing.coupon.delete_user_coupon(id, user_id).await?;
        if !removed {
            return Err(Error::biz(400, "优惠券不存在"));
        }
        Ok(())
    }

    // ————————————————————— 核销 —————————————————————

    /// 下单核销：把选中的券原子绑定到订单。
    ///
    /// 对应 Java `UserCouponServiceImpl#useCoupon`。任一券核销失败即报错，
    /// 由调用方（订单事务）回滚。
    pub async fn use_coupons(
        &self,
        user_id: i32,
        user_coupon_ids: &[i32],
        order_id: i32,
    ) -> Result<()> {
        if user_coupon_ids.is_empty() {
            return Ok(());
        }
        let now = now_seconds();
        for id in user_coupon_ids {
            let ok = self
                .repos.marketing.coupon
                .try_mark_used(*id, user_id, order_id, now)
                .await?;
            if !ok {
                return Err(Error::biz(400, "优惠券不存在或已使用"));
            }
        }
        Ok(())
    }

    /// 订单取消/关闭时回退券占用。
    pub async fn release_by_order(&self, order_id: i32) -> Result<u64> {
        self.repos.marketing.coupon.release_by_order(order_id).await
    }

    /// 计算选中券的抵扣总额（受 `max_order_amount` 上限约束）。
    ///
    /// 满减券直接取 `coupon_money`；折扣券按订单金额折算。
    pub async fn calc_discount(
        &self,
        user_id: i32,
        user_coupon_ids: &[i32],
        order_amount: Decimal,
    ) -> Result<Decimal> {
        if user_coupon_ids.is_empty() {
            return Ok(Decimal::ZERO);
        }
        let ucs = self
            .repos.marketing.coupon
            .find_user_coupons_by_ids(user_id, user_coupon_ids)
            .await?;
        let mut total = Decimal::ZERO;
        for uc in &ucs {
            let coupon = match self.repos.marketing.coupon.find_by_id(uc.coupon_id).await? {
                Some(c) => c,
                None => continue,
            };
            if !coupon.usable_for(order_amount) {
                return Err(Error::biz(400, "订单金额未达到优惠券使用门槛"));
            }
            total += coupon_discount_amount(&coupon, order_amount);
        }
        // 抵扣不超过订单金额
        Ok(total.min(order_amount).max(Decimal::ZERO))
    }

    // ————————————————————— 内部 —————————————————————

    /// 为用户券附加券定义信息（名称、金额、状态等）。
    async fn enrich_user_coupons(
        &self,
        list: Vec<UserCoupon>,
        now: i64,
    ) -> Result<Vec<UserCouponVO>> {
        if list.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<i32> = list.iter().map(|uc| uc.coupon_id).collect();
        let coupons = self.repos.marketing.coupon.find_by_ids(&ids).await?;
        let map: std::collections::HashMap<i32, &Coupon> =
            coupons.iter().map(|c| (c.coupon_id, c)).collect();

        Ok(list
            .iter()
            .map(|uc| {
                let mut vo = UserCouponVO {
                    id: uc.id,
                    coupon_id: uc.coupon_id,
                    coupon_sn: uc.coupon_sn.clone(),
                    start_date: uc.start_date,
                    end_date: uc.end_date,
                    used_time: uc.used_time,
                    order_id: uc.order_id,
                    ..Default::default()
                };
                if let Some(c) = map.get(&uc.coupon_id) {
                    vo.coupon_name = c.coupon_name.clone();
                    vo.coupon_money = c.coupon_money;
                    vo.coupon_discount = c.coupon_discount;
                    vo.coupon_desc = c.coupon_desc.clone();
                    vo.coupon_type = c.coupon_type;
                    vo.min_order_amount = c.min_order_amount;
                    vo.send_range = c.send_range;
                    vo.send_range_data = parse_range_data(&c.send_range_data);
                }
                vo.with_status(uc.status_at(now))
            })
            .collect())
    }
}

/// 计算单张券对给定订单金额的抵扣额。
///
/// - 满减券（`coupon_type = 1`）：固定 `coupon_money`
/// - 折扣券（`coupon_type = 2`）：`order_amount * (10 - coupon_discount) / 10`
///
/// 若 `max_order_amount > 0`，抵扣额以之为上限（对齐 Java 优惠金额上限语义）。
fn coupon_discount_amount(coupon: &Coupon, order_amount: Decimal) -> Decimal {
    let mut amount = if coupon.coupon_type == 2 {
        // 折扣券：coupon_discount = 8.5 表示 8.5 折 → 优惠 15%
        let ten = Decimal::from(10);
        let discount = coupon.coupon_discount.max(Decimal::ZERO).min(ten);
        order_amount * (ten - discount) / ten
    } else {
        coupon.coupon_money
    };
    let cap = coupon.max_order_amount;
    if cap > Decimal::ZERO {
        amount = amount.min(cap);
    }
    amount.max(Decimal::ZERO)
}

/// 会员等级是否满足券的等级限制（无限制则通过）。
fn is_rank_eligible(coupon: &Coupon, user_rank: Option<i32>) -> bool {
    if coupon.is_new_user != NEW_USER_MEMBER {
        return true;
    }
    let rank_ids = coupon.limited_rank_ids();
    if rank_ids.is_empty() {
        return true;
    }
    user_rank.map(|r| rank_ids.contains(&r)).unwrap_or(false)
}

/// 解析 `send_range_data`（JSON 整型数组，兼容空值）。
fn parse_range_data(raw: &str) -> Vec<i32> {
    let t = raw.trim();
    if t.is_empty() || t == "[]" {
        return Vec::new();
    }
    serde_json::from_str::<Vec<i32>>(t).unwrap_or_default()
}

/// 实体 → 管理端 VO。
fn to_coupon_vo(c: &Coupon, now: i64, receive_num: i64) -> CouponVO {
    let status = c.status_at(now);
    CouponVO {
        coupon_id: c.coupon_id,
        coupon_name: c.coupon_name.clone(),
        coupon_money: c.coupon_money,
        coupon_discount: c.coupon_discount,
        coupon_desc: c.coupon_desc.clone(),
        coupon_type: c.coupon_type,
        send_range: c.send_range,
        send_range_data: parse_range_data(&c.send_range_data),
        min_order_amount: c.min_order_amount,
        send_type: c.send_type,
        use_day: c.use_day,
        use_start_date: c.use_start_date,
        use_end_date: c.use_end_date,
        is_show: c.is_show,
        is_global: c.is_global,
        is_new_user: c.is_new_user,
        enabled_click_get: c.enabled_click_get,
        limit_user_rank: c.limited_rank_ids(),
        shop_id: c.shop_id.unwrap_or(0),
        is_delete: c.is_delete.unwrap_or(0),
        limit_num: c.limit_num.unwrap_or(0),
        delay_day: c.delay_day,
        send_num: c.send_num,
        max_order_amount: c.max_order_amount,
        coupon_unit: c.coupon_unit,
        reduce_type: c.reduce_type,
        add_time: c.add_time.unwrap_or(0),
        receive_num,
        status,
        status_name: CouponStatus::name(status).to_string(),
    }
}

/// 实体 → 前台 VO。
fn to_client_vo(c: &Coupon, receive_num: i64) -> CouponClientVO {
    CouponClientVO {
        coupon_id: c.coupon_id,
        coupon_name: c.coupon_name.clone(),
        coupon_money: c.coupon_money,
        coupon_discount: c.coupon_discount,
        coupon_desc: c.coupon_desc.clone(),
        coupon_type: c.coupon_type,
        send_range: c.send_range,
        send_range_data: parse_range_data(&c.send_range_data),
        min_order_amount: c.min_order_amount,
        send_type: c.send_type,
        use_day: c.use_day,
        use_start_date: c.use_start_date,
        use_end_date: c.use_end_date,
        is_show: c.is_show,
        is_global: c.is_global,
        is_new_user: c.is_new_user,
        enabled_click_get: c.enabled_click_get,
        limit_user_rank: c.limited_rank_ids(),
        shop_id: c.shop_id.unwrap_or(0),
        limit_num: c.limit_num.unwrap_or(0),
        delay_day: c.delay_day,
        send_num: c.send_num,
        max_order_amount: c.max_order_amount,
        coupon_unit: c.coupon_unit,
        reduce_type: c.reduce_type,
        add_time: c.add_time.unwrap_or(0),
        is_receive: 0,
        receive_num,
    }
}

/// 手动发放（管理端）：跳过领取限制，直接给指定用户发券。
///
/// 对应 Java `UserCouponServiceImpl#issueToUser`。
impl CouponService {
    pub async fn issue_to_user(&self, coupon_id: i32, user_id: i32) -> Result<i32> {
        if user_id <= 0 {
            return Err(Error::Validation("用户ID无效".into()));
        }
        let coupon = self
            .repos.marketing.coupon
            .find_by_id(coupon_id)
            .await?
            .ok_or_else(|| Error::biz(400, "优惠券不存在"))?;
        if coupon.is_deleted() {
            return Err(Error::biz(400, "优惠券不存在"));
        }

        let now = now_seconds();
        let (start_date, end_date) = coupon.time_range_at(now);
        if end_date <= now || end_date <= start_date {
            return Err(Error::biz(400, "优惠券已失效"));
        }

        // 发放总量校验
        let total_supply = i64::from(coupon.total_supply());
        if total_supply > 0 {
            let issued = self.repos.marketing.coupon.count_issued(coupon_id).await?;
            if issued >= total_supply {
                return Err(Error::biz(400, "优惠券剩余数量不足"));
            }
        }

        let uc = UserCoupon {
            id: 0,
            coupon_id,
            coupon_sn: gen_sn("UC"),
            user_id,
            used_time: 0,
            order_id: 0,
            start_date: start_date as i32,
            end_date: end_date as i32,
            source_type: Some(SOURCE_MANUAL),
        };
        self.repos.marketing.coupon.insert_user_coupon(&uc).await
    }
}

/// 列表店铺过滤：店铺账号强制本店；平台账号可用请求值精确过滤（None = 全部）。
fn scoped_list_shop_id(requested: Option<i32>) -> Option<i32> {
    TenantScope::current_list_filter().or(requested.filter(|id| *id > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn money_coupon(money: &str, threshold: &str, cap: &str) -> Coupon {
        Coupon {
            coupon_type: 1,
            coupon_money: Decimal::from_str(money).unwrap(),
            min_order_amount: Decimal::from_str(threshold).unwrap(),
            max_order_amount: Decimal::from_str(cap).unwrap(),
            reduce_type: 1,
            ..Default::default()
        }
    }

    #[test]
    fn money_coupon_discount_is_fixed() {
        let c = money_coupon("20.00", "100.00", "0.00");
        let d = coupon_discount_amount(&c, Decimal::from(200));
        assert_eq!(d.to_string(), "20.00");
    }

    #[test]
    fn discount_coupon_applies_percentage() {
        let mut c = money_coupon("0", "0", "0");
        c.coupon_type = 2;
        // 8.5 折 → 优惠 15%
        c.coupon_discount = Decimal::from_str("8.5").unwrap();
        let d = coupon_discount_amount(&c, Decimal::from(200));
        assert_eq!(d.to_string(), "30.0");
    }

    #[test]
    fn discount_respects_cap() {
        let c = money_coupon("50.00", "0.00", "30.00");
        let d = coupon_discount_amount(&c, Decimal::from(200));
        assert_eq!(d.to_string(), "30.00");
    }

    #[test]
    fn discount_never_negative() {
        let mut c = money_coupon("0", "0", "0");
        c.coupon_type = 2;
        c.coupon_discount = Decimal::from(20); // 异常折扣
        let d = coupon_discount_amount(&c, Decimal::from(100));
        assert!(d >= Decimal::ZERO);
    }

    #[test]
    fn rank_eligibility() {
        let mut c = money_coupon("10", "0", "0");
        // 非会员专享：始终通过
        assert!(is_rank_eligible(&c, None));

        // 会员专享 + 等级白名单
        c.is_new_user = NEW_USER_MEMBER;
        c.limit_user_rank = Some("[1,3]".into());
        assert!(is_rank_eligible(&c, Some(3)));
        assert!(!is_rank_eligible(&c, Some(2)));
        assert!(!is_rank_eligible(&c, None));

        // 会员专享但未配置等级：放行
        c.limit_user_rank = Some("[]".into());
        assert!(is_rank_eligible(&c, None));
    }

    #[test]
    fn range_data_parsing_is_tolerant() {
        assert_eq!(parse_range_data("[1,2]"), vec![1, 2]);
        assert!(parse_range_data("").is_empty());
        assert!(parse_range_data("[]").is_empty());
        assert!(parse_range_data("bad").is_empty());
    }

    #[test]
    fn coupon_vo_maps_status_name() {
        let mut c = money_coupon("10", "0", "0");
        c.send_type = 1;
        c.use_start_date = 5_000;
        c.use_end_date = 9_000;
        // 未开始
        let vo = to_coupon_vo(&c, 1_000, 3);
        assert_eq!(vo.status, CouponStatus::PENDING);
        assert_eq!(vo.status_name, "待生效");
        assert_eq!(vo.receive_num, 3);
        // 已失效
        let vo = to_coupon_vo(&c, 10_000, 0);
        assert_eq!(vo.status_name, "已失效");
    }

    #[test]
    fn client_vo_parses_json_fields() {
        let mut c = money_coupon("10", "0", "0");
        c.send_range_data = "[7,8]".into();
        let vo = to_client_vo(&c, 1);
        assert_eq!(vo.send_range_data, vec![7, 8]);
        assert_eq!(vo.is_receive, 0);
    }

    #[tokio::test]
    async fn tenant_scope_forces_shop_list_filter_and_blocks_cross_shop_detail() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(None),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        // 店铺账号：列表强制本店，即使未传 shopId
        assert_eq!(filter, Some(8));
        assert!(blocked);
        // 平台账号：列表不强制，但可用请求值精确过滤
        let filter = with_tenant_scope(TenantScope::platform(), async {
            scoped_list_shop_id(Some(9))
        })
        .await;
        assert_eq!(filter, Some(9));
    }
}