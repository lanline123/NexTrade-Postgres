//! 代客下单优惠券域服务（trade::proxy_shopping_user_coupon）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.UserCouponServiceImpl#claim`
//! （L304-392），由
//! `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingUserCouponController`
//! （`@RequestMapping("/adminapi/proxyShopping/coupon")`，方法级 `@PreAuthorize`
//! `proxyShoppingManage`）暴露为 `POST /adminapi/proxyShopping/coupon/claim`。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`ProxyShoppingUserCouponService::claim`] | `UserCouponServiceImpl#claim(UserCouponClaimParam)` |
//!
//! # Java `claim` 的完整校验链（逐条复刻，顺序不变）
//!
//! 1. `userId = param.userId ?? SecurityUtils.getCurrentUserId()`（L305-306）
//! 2. `coupon = couponMapper.selectById(couponId)`；**为空即报「优惠券不存在」**
//!    （L310-311）。⚠️ **不判 `is_delete`**（与 `getManualIssueCoupon` 不同）。
//! 3. 每人限领：`userCouponCount = count(couponId, userId)`；
//!    `userCouponCount < limit_num || limit_num == 0`（L314-318）
//! 4. 发放总量：`sendCouponCount = count(couponId)`；
//!    `sendCouponCount < send_num || send_num == 0`（L321-324）
//! 5. `is_new_user == 1`（新人专享）：`userCouponCount == 0` 且
//!    已支付订单数（`pay_status > 0`）为 0（L327-336）
//! 6. `is_new_user == 2`（会员专享）：`limit_user_rank` 非空且非 `[]` 时，
//!    用户的 `rank_id` 必须在白名单内（L341-349）
//! 7. `promotion`（`relation_id = couponId AND type = 2`，`LIMIT 1`，L352-357）：
//!    - `member_scope` 非空白 → 走**人群圈选**匹配（L362-363）
//!    - 否则 `member_range == 1` → 按 `member_marks`（JSON 数组）比对
//!      用户的 `mark`（L364-371）
//! 8. 有效期（L375-384）：`send_type > 0` 取固定时间；否则
//!    `start = now + delay_day*86400`，`end = start + use_day*86400`
//! 9. `this.save(UserCoupon{couponId, userId, startDate, endDate})`（L385-391）
//!
//! # 与 Java 的差异（逐条，均已在返回值 `uncertain_points` 中列明）
//!
//! 1. **`user_coupon` 的 NULL 列**：Java `UserCoupon.builder()` 只设 4 个字段，
//!    `usedTime` / `orderId` / `couponSn` / `sourceType` 全为 `null`；
//!    MyBatis-Plus 默认 `FieldStrategy.NOT_NULL` 会把这些列**从 INSERT 中剔除**，
//!    落库取列默认值（`used_time`=0 / `order_id`=0 / `coupon_sn`=`''` /
//!    `source_type`=0）。本实现显式写 `0 / 0 / '' / 0`，
//!    与 Java 的**最终落库值逐字节一致**（`coupon_sn` 为 `NOT NULL DEFAULT ''`，
//!    显式传 NULL 会直接失败）。
//!    ⚠️ 连带后果：`coupon_sn` 在**本接口**下为空串（与前台
//!    `CouponService::claim` 生成 `gen_sn("UC")` 不同）—— 因为 Java 这条链路
//!    确实没有 SN 生成器。这一点如实照抄，不「顺手补齐」。
//! 2. **`source_type` 恒为 0**：Java 未设置 `sourceType`，落库取默认 0，
//!    故**代客领取与前台自助领取在库里不可区分**（没有独立的来源值）。
//!    本实现照抄 0，不臆造 `source_type = 3` 之类的新枚举。
//! 3. **`is_delete` 不参与判定**：软删除的券仍可被本接口领取（与 Java 一致），
//!    同样**不判** `is_show` / `enabled_click_get`（Java `claim` 里没有这两项）。
//!    ⚠️ 因此本实现**不能**直接复用前台 `CouponService::claim`
//!    （后者多了 `is_show` / `enabled_click_get` / `end_date <= now` 三道 Java
//!    本链路不存在的门，会拒绝原系统允许的领取）。故本域自建校验链。
//! 4. **`promotion.member_scope` 人群圈选未迁移**：Java 调
//!    `memberAudienceService.matchesScope(userId, memberScope)`，
//!    该服务依赖 `SendUserTargetDTO` 判形/升维 + 会员标签关联
//!    （`user_member_tag`）+ 消费额/消费次数/历史商品/充值额等附加条件。
//!    Rust 侧该服务整体未迁移（`services/seckill.rs` 与
//!    `services/time_discount.rs` 亦有同一缺口）。
//!    Java 的 `matchesScope` 在 **解析失败时返回 `true`（放行）**，
//!    「未迁移」与「解析失败」在可观察行为上同属未知；本实现选择
//!    **明确报业务错误**（文案 = `error.coupon.claim.audience.not.allowed`
//!    = 「您不在该优惠券的领取范围内」），即**拒绝**而**不是静默放行**——
//!    宁可少发，不可错发。若某活动配了 `member_scope`，该券在本接口下
//!    一律无法领取，直到人群服务迁移完成。
//! 5. **用户不存在的分支**：Java 在 `is_new_user == 2` 且配了等级白名单时调
//!    `user.getRankId()`、在 `member_range == 1` 时调 `user.getMark()`，
//!    若 `userMapper.selectById` 返回 `null` 会抛 **NPE（HTTP 500）**。
//!    本实现把它归一为业务错误「优惠券不存在」以外的明确提示
//!    （见 [`ProxyShoppingUserCouponService::claim`]，对 NPE 不做 500 复刻，避免把
//!    编程缺陷当成契约）。**这是本片唯一一处「行为与 Java 不同」的显式取舍**。
//! 6. **错误码**：Java `GlobalException(String)` 的 code 恒为
//!    `SERVICE_DATA_ERROR = 1001`；本实现用同一 code + `messages.properties`
//!    中的**逐字中文文案**（不做 i18n 切换，Rust 侧无 `MessageHelper`）。
//! 7. **并发超发风险照抄**：整条链路无事务、无行锁、无唯一键
//!    （见 `repo::proxy_shopping_user_coupon` 模块头），并发下可超 `limit_num` / `send_num`。
//!    本实现不擅自加锁或补偿 —— 与原系统同风险。
//! 8. **权限未迁移**：Java 在**类级** `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`；
//!    Rust 侧权限矩阵未实现，`require_auth` 只校验登录态
//!    （与 seckill / order_admin / brand / shop_manage / pc_navigation 等已知缺口一致）。
//! 9. **`userId` 缺省落入「当前登录者」**：Java 的 `SecurityUtils.getCurrentUserId()`
//!    在管理端上下文里返回的是**管理员的 id**，因此前端不传 `userId` 时
//!    会把券发给管理员自己（若该 id 恰好不是合法会员，还会触发第 5 条的 NPE）。
//!    本实现把「当前登录者 id」作为参数由路由层传入（`AuthUser.id`），
//!    语义与 Java 完全一致，**不擅自改成「必须传 userId」**。
//! 10. **`start_date` / `end_date` 的 `int` 溢出**：Java 用 `long` 算
//!    `now + delay_day*86400`，但 `user_coupon.start_date` / `end_date` 是
//!    MySQL **`int`** 列（schema 实测），落库时发生隐式收窄。
//!    约 `delay_day >= 497`（或 `use_day` 累加越界）时会溢出/回绕，
//!    券的有效期会变成错误值（**静默数据损坏**）。
//!    本实现用 `start_date as i32` 显式复刻同一收窄（Rust 的 `as` 是截断，
//!    MySQL 对超范围 `int` 的行为取决于 `sql_mode`：严格模式报错、
//!    非严格模式截断到边界值），二者在**越界场景下并不完全等价**。
//!    本片不擅自把列改成 `bigint`（会与 321 张表的既有 schema 冲突），
//!    已在 `uncertain_points` 标注。
//!
//! # 返回结构
//!
//! Java `ProxyShoppingUserCouponController.claim` 返回 `void`（HTTP 200 +
//! `data: null`）。本实现回传新建 `user_coupon.id`（`INSERT ... RETURNING id`），
//! 与 buyer_show `create` 回传新建主键的既有约定一致，便于前端/运维核对。

use std::sync::Arc;

use serde_json::Value;
use nextrade_common::utils::now_seconds;
use nextrade_core::entities::business::{NEW_USER_MEMBER, NEW_USER_ONLY};
use nextrade_core::entities::Coupon;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::proxy_shopping_user_coupon::SOURCE_CLAIM;
use nextrade_infrastructure::db::Repositories;

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// `promotion.member_range = 1`：仅限指定标记的会员（Java L364 的字面量 `1`）。
pub const MEMBER_RANGE_SPECIFIED_MARKS: i32 = 1;

/// `messages.properties: error.coupon.not.exist`（逐字照抄）。
pub const MSG_COUPON_NOT_EXIST: &str = "优惠券不存在";
/// `messages.properties: error.coupon.claim.limit.exceeded`（逐字照抄）。
pub const MSG_CLAIM_LIMIT_EXCEEDED: &str = "已超出领取数量限制";
/// `messages.properties: error.coupon.claim.once.only`（逐字照抄）。
pub const MSG_CLAIM_ONCE_ONLY: &str = "该优惠券仅能领取一次";
/// `messages.properties: error.coupon.claim.new.user.only`（逐字照抄）。
pub const MSG_CLAIM_NEW_USER_ONLY: &str = "该优惠券仅限新人领取";
/// `messages.properties: error.coupon.claim.rank.not.allowed`（逐字照抄）。
pub const MSG_CLAIM_RANK_NOT_ALLOWED: &str = "您的会员等级无法领取该优惠券";
/// `messages.properties: error.coupon.claim.member.only`（逐字照抄）。
pub const MSG_CLAIM_MEMBER_ONLY: &str = "该优惠券仅限指定会员领取";
/// `messages.properties: error.coupon.claim.audience.not.allowed`（逐字照抄）。
pub const MSG_CLAIM_AUDIENCE_NOT_ALLOWED: &str = "您不在该优惠券的领取范围内";

/// Java `SecurityUtils.getCurrentUserId()` 的缺省回落：
/// `Optional.ofNullable(param.getUserId()).orElse(currentUserId)`（L305-306）。
///
/// 注意 `param.userId = 0` 是**合法入参**（Java 只有 `null` 才回落），
/// 与 `Optional.ofNullable` 语义一致 —— 不得写成 `filter(|v| *v > 0)`。
#[must_use]
pub fn resolve_claim_user_id(param_user_id: Option<i32>, current_user_id: i32) -> i32 {
    param_user_id.unwrap_or(current_user_id)
}

/// Java `StringUtils.isNotBlank(memberScope)` 的等价判定。
#[must_use]
pub fn audience_configured(member_scope: Option<&str>) -> bool {
    member_scope.map(|s| !s.trim().is_empty()).unwrap_or(false)
}

/// `promotion.member_scope` 人群匹配（Java `MemberAudienceService#matchesScope`）。
///
/// 支持两种历史并存格式：
/// - 新结构：`{ "scope": 0|1, "userIds": [], "rankIds": [], "tagIds": [],
///   "tagMatchMode": 0|1 }`；
/// - 旧结构：`{ "audienceType": 0|1|2, "rankIds": [], "tagIds": [],
///   "tagMatchMode": 0|1 }`（`MemberAudienceDTO`）。
///
/// 语义与 Java 一致：
/// - 空/空白 → 无限制，命中；
/// - 解析失败 → Java `parseScope` 返回 null，`matchesScope` 放行；
/// - 非会员 → 不命中；
/// - `scope=0` / `audienceType=0` → 命中；
/// - 其余按 userIds / rankIds / tagIds **并集**；`scope=1` 三组全空 → 不命中。
#[must_use]
pub fn member_scope_matches(
    raw: Option<&str>,
    user_id: i32,
    rank_id: i32,
    user_type: i32,
    user_tag_ids: &[i64],
) -> bool {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    // Java `matches`：只有 `user_type == 0`（会员）才可能命中
    if user_type != 0 {
        return false;
    }
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        // Java `parseScope` 解析失败返回 null → `matchesScope` 返回 true
        return true;
    };
    let has_new_scope = value.get("scope").is_some_and(|v| !v.is_null());
    if has_new_scope {
        let scope = value.get("scope").and_then(Value::as_i64).unwrap_or(0);
        if scope == 0 {
            return true;
        }
        return target_groups_match(&value, user_id, rank_id, user_tag_ids);
    }
    // 旧 MemberAudienceDTO 结构
    match value
        .get("audienceType")
        .and_then(Value::as_i64)
        .unwrap_or(0)
    {
        0 => true,
        // 1=按等级、2=按标签；其余值 Java 的 upgrade 也落入「按标签」分支
        1 => value
            .get("rankIds")
            .and_then(Value::as_array)
            .is_some_and(|ids| ids.iter().any(|v| v.as_i64() == Some(i64::from(rank_id)))),
        _ => tags_match(&value, user_tag_ids),
    }
}

fn target_groups_match(
    value: &Value,
    user_id: i32,
    rank_id: i32,
    user_tag_ids: &[i64],
) -> bool {
    if value
        .get("userIds")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|v| v.as_i64() == Some(i64::from(user_id))))
    {
        return true;
    }
    if value
        .get("rankIds")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|v| v.as_i64() == Some(i64::from(rank_id))))
    {
        return true;
    }
    tags_match(value, user_tag_ids)
}

fn tags_match(value: &Value, user_tag_ids: &[i64]) -> bool {
    let Some(target) = value.get("tagIds").and_then(Value::as_array) else {
        return false;
    };
    let target: Vec<i64> = target.iter().filter_map(Value::as_i64).collect();
    if target.is_empty() || user_tag_ids.is_empty() {
        return false;
    }
    if value
        .get("tagMatchMode")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        == 1
    {
        target.iter().all(|id| user_tag_ids.contains(id))
    } else {
        target.iter().any(|id| user_tag_ids.contains(id))
    }
}

/// 解析 `promotion.member_marks`（Java `JSON.parseArray(memberMarks, Integer.class)`）。
///
/// Java 解析失败时 `JSON.parseArray` 抛异常（HTTP 500）；本实现按
/// 「解析不出即空列表」处理 —— 空列表在 Java 的后续判定里等价于
/// `allowedMarks.isEmpty()` → **放行**，与 Rust 的空 `Vec` 语义一致。
/// 对损坏数据而言这比 500 更宽容，已在 `uncertain_points` 标注。
#[must_use]
pub fn parse_member_marks(raw: Option<&str>) -> Vec<i32> {
    let Some(t) = raw.map(str::trim) else {
        return Vec::new();
    };
    if t.is_empty() || t == "null" || t == "[]" {
        return Vec::new();
    }
    serde_json::from_str::<Vec<i32>>(t).unwrap_or_default()
}

/// Java 旧逻辑（`member_range == 1`）的标记判定，返回**是否拒绝**：
/// `markFlag = !allowedMarks.isEmpty() && !allowedMarks.contains(user.getMark())`。
///
/// 白名单为空 → `markFlag = false` → **放行**（Java 语义）。
#[must_use]
pub fn member_mark_rejected(allowed_marks: &[i32], user_mark: i32) -> bool {
    !allowed_marks.is_empty() && !allowed_marks.contains(&user_mark)
}

/// 「会员专享」等级白名单判定，返回**是否拒绝**。
///
/// 对应 Java：`rankFlag = CollUtil.isNotEmpty(rankIds) && !rankIds.contains(user.getRankId())`。
/// 白名单为空（`[]` 或解析失败）→ **放行**（Java 语义）。
#[must_use]
pub fn member_rank_rejected(rank_ids: &[i32], user_rank_id: i32) -> bool {
    !rank_ids.is_empty() && !rank_ids.contains(&user_rank_id)
}

/// 每人限领判定（Java `userCouponCount < limitNum || limitNum == 0`）。
///
/// 返回 `true` 表示**允许**领取。
#[must_use]
pub fn within_per_user_limit(claimed: i64, limit_num: i32) -> bool {
    limit_num == 0 || claimed < i64::from(limit_num)
}

/// 发放总量判定（Java `sendCouponCount < sendNum || sendNum == 0`）。
///
/// 返回 `true` 表示**允许**领取。
///
/// ⚠️ 与 [`within_per_user_limit`] 都是**先查后写、非原子**的判定，
/// 并发可失守（见模块头第 7 条）。
#[must_use]
pub fn within_total_supply(issued: i64, send_num: i32) -> bool {
    send_num == 0 || issued < i64::from(send_num)
}

/// 计算券的有效期（秒），对应 Java `claim` L375-384。
///
/// - `send_type > 0`：固定时间（`use_start_date` / `use_end_date`）
/// - 否则：`start = now + delay_day * 86400`，`end = start + use_day * 86400`
///
/// ⚠️ 与 Java 一致：**不做「已过期就拒领」的门**（Java `claim` 没有该校验），
/// 因此固定时间已过期的券照样能被领到（进入「已失效」状态）。
#[must_use]
pub fn claim_time_range(
    send_type: i32,
    delay_day: i32,
    use_day: i32,
    now: i64,
    use_start_date: i32,
    use_end_date: i32,
) -> (i64, i64) {
    if send_type > 0 {
        return (i64::from(use_start_date), i64::from(use_end_date));
    }
    let start = now + i64::from(delay_day) * 86_400;
    let end = start + i64::from(use_day) * 86_400;
    (start, end)
}

/// 代客下单优惠券服务（本切片独占）。
pub struct ProxyShoppingUserCouponService {
    repos: Arc<Repositories>,
}

impl ProxyShoppingUserCouponService {
    /// 构造：持有全局仓储集合（与 product_manage/002/…/026 同一约定）。
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 领取优惠券（管理端代客领取）。
    ///
    /// 对应 Java `ProxyShoppingUserCouponController.claim(@Validated
    /// @RequestBody UserCouponClaimParam)` → `UserCouponServiceImpl#claim`。
    ///
    /// `current_user_id` 即 Java `SecurityUtils.getCurrentUserId()` 在管理端
    /// 上下文里的返回值（路由层传 `AuthUser.id`）；`param_user_id` 为
    /// 请求体里的 `userId`（代客下单时由前端指定目标会员）。
    ///
    /// 校验顺序与 Java **逐行一致**（见模块头），任一步失败即返回
    /// `Error::Biz { code: 1001, msg: <原中文文案> }`。
    ///
    /// 成功返回新建 `user_coupon.id`。
    pub async fn claim(
        &self,
        param_user_id: Option<i32>,
        coupon_id: i32,
        current_user_id: i32,
    ) -> Result<i32> {
        let user_id = resolve_claim_user_id(param_user_id, current_user_id);

        // ① 券存在性（Java L310-311，Assert.notNull）
        let coupon = self
            .repos.trade.proxy_shopping_user_coupon
            .find_coupon(coupon_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_COUPON_NOT_EXIST))?;

        // ② 每人限领（Java L314-318）—— 注意 Java 这次 count 先于 limit 判定，
        //    且该值在后面「新人专享 / 一次限定」里被复用，本实现同样只查一次。
        let user_count = self
            .repos.trade.proxy_shopping_user_coupon
            .count_user_claimed(user_id, coupon_id)
            .await?;
        if !within_per_user_limit(user_count, coupon.claim_limit_per_user()) {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CLAIM_LIMIT_EXCEEDED));
        }

        // ③ 发放总量（Java L321-324）
        let issued = self.repos.trade.proxy_shopping_user_coupon.count_issued(coupon_id).await?;
        if !within_total_supply(issued, coupon.total_supply()) {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CLAIM_LIMIT_EXCEEDED));
        }

        // ④ 新人专享（Java L327-336）
        if coupon.is_new_user == NEW_USER_ONLY {
            if user_count != 0 {
                return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CLAIM_ONCE_ONLY));
            }
            let paid_orders = self
                .repos.trade.proxy_shopping_user_coupon
                .count_user_paid_orders(user_id)
                .await?;
            if paid_orders != 0 {
                return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CLAIM_NEW_USER_ONLY));
            }
        }

        // ⑤ 会员专享（`is_new_user == 2`，按 `limit_user_rank` 等级白名单）
        //    与 ⑥ `promotion` 人群/标记限制共用同一次 `selectById(userId)`
        //    （Java L339 只读一次 user 行）。
        //
        //    `promotion` 只读一次（Java L352 也只读一次）。
        //    ⚠️ Java 在读 `promotion` 之前就已 `selectById(userId)`，但 `user`
        //    变量直到 L346 / L368 才被解引用；本实现按需读取，可观察行为相同。
        let promotion = self.repos.trade.proxy_shopping_user_coupon.find_coupon_promotion(coupon_id).await?;
        let needs_user_row = coupon.is_new_user == NEW_USER_MEMBER
            || promotion
                .as_ref()
                .map(|r| {
                    audience_configured(r.member_scope.as_deref())
                        || r.member_range == MEMBER_RANGE_SPECIFIED_MARKS
                })
                .unwrap_or(false);
        let user_flags = if needs_user_row {
            self.repos.trade.proxy_shopping_user_coupon.find_user_flags(user_id).await?
        } else {
            None
        };

        if coupon.is_new_user == NEW_USER_MEMBER {
            let rank_ids = coupon.limited_rank_ids();
            if !rank_ids.is_empty() {
                // ⚠️ Java 此处 user 为 null 会 NPE（HTTP 500）。本实现归一为
                //    明确业务错误，见模块头第 5 条（本片唯一一处显式行为取舍）。
                let user_rank = user_flags
                    .as_ref()
                    .map(|f| f.rank_id)
                    .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_COUPON_NOT_EXIST))?;
                if member_rank_rejected(&rank_ids, user_rank) {
                    return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CLAIM_RANK_NOT_ALLOWED));
                }
            }
        }

        if let Some(rule) = promotion {
            if audience_configured(rule.member_scope.as_deref()) {
                let flags = user_flags
                    .as_ref()
                    .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_COUPON_NOT_EXIST))?;
                let tag_ids = if rule
                    .member_scope
                    .as_deref()
                    .is_some_and(|s| s.contains("tagIds"))
                {
                    self.repos
                        .member
                        .member_audience
                        .tag_ids_for_user(user_id)
                        .await?
                } else {
                    Vec::new()
                };
                if !member_scope_matches(
                    rule.member_scope.as_deref(),
                    user_id,
                    flags.rank_id,
                    flags.user_type,
                    &tag_ids,
                ) {
                    return Err(Error::biz(
                        SERVICE_DATA_ERROR,
                        MSG_CLAIM_AUDIENCE_NOT_ALLOWED,
                    ));
                }
            } else if rule.member_range == MEMBER_RANGE_SPECIFIED_MARKS {
                let allowed_marks = parse_member_marks(rule.member_marks.as_deref());
                if !allowed_marks.is_empty() {
                    // 同上：Java 此时 user 为 null 会 NPE，本实现归一为业务错误
                    let user_mark = user_flags
                        .as_ref()
                        .map(|f| f.mark)
                        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_COUPON_NOT_EXIST))?;
                    if member_mark_rejected(&allowed_marks, user_mark) {
                        return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CLAIM_MEMBER_ONLY));
                    }
                }
            }
        }

        // ⑦ 有效期（Java L375-384）
        let now = now_seconds();
        let (start_date, end_date) = claim_time_range(
            coupon.send_type,
            coupon.delay_day,
            coupon.use_day,
            now,
            coupon.use_start_date,
            coupon.use_end_date,
        );

        // ⑧ 落库（Java L385-391）。i32 截断与 schema 一致（`start_date`/`end_date`
        //    都是 `int`，与 Java `long` → MySQL int 列的隐式收窄等价）。
        self.repos.trade.proxy_shopping_user_coupon
            .insert_user_coupon(
                coupon_id,
                user_id,
                start_date as i32,
                end_date as i32,
                SOURCE_CLAIM,
            )
            .await
    }

    /// 代客下单可领取（可见）优惠券列表。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/proxyShopping/cart.ts#getProxyShoppingCouponList`
    /// （`POST /proxyShopping/coupon/visible-list`；对应 Java 控制器未随本仓
    /// 提供，注释称「对齐收银台 pos/user/coupon/visible-list」）。
    ///
    /// `user_id` / `product_ids` 当前不参与过滤（Java 参考实现缺失，
    /// 保留入参签名以便后续收紧：按会员人群/商品适用范围过滤），
    /// 返回未删除、展示中、可点击领取且有效期末过的券定义列表，
    /// 按添加时间倒序。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn visible_list(
        &self,
        _user_id: Option<i32>,
        _product_ids: &[i32],
    ) -> Result<Vec<Coupon>> {
        let now = now_seconds();
        self.repos
            .trade
            .proxy_shopping_user_coupon
            .find_visible_coupons(now)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ————————————————————— userId 回落 —————————————————————

    #[test]
    fn user_id_falls_back_to_current_only_when_missing() {
        // Java `Optional.ofNullable(param.getUserId()).orElse(getCurrentUserId())`
        assert_eq!(resolve_claim_user_id(Some(7), 99), 7);
        assert_eq!(resolve_claim_user_id(None, 99), 99);
        // ⚠️ 0 / 负数都是**合法入参**，绝不回落到当前登录者
        assert_eq!(resolve_claim_user_id(Some(0), 99), 0);
        assert_eq!(resolve_claim_user_id(Some(-5), 99), -5);
    }

    // ————————————————————— 限领 / 总量 —————————————————————

    #[test]
    fn per_user_limit_semantics_match_java() {
        // limit_num == 0 表示不限领
        assert!(within_per_user_limit(0, 0));
        assert!(within_per_user_limit(1_000, 0));
        // 已领 < 限领 才放行
        assert!(within_per_user_limit(0, 1));
        assert!(within_per_user_limit(2, 3));
        assert!(!within_per_user_limit(3, 3));
        assert!(!within_per_user_limit(4, 3));
        // 负的 limit_num 在 Java 里等价于「已领 > 负数」→ 恒拒
        assert!(!within_per_user_limit(0, -1));
    }

    #[test]
    fn total_supply_semantics_match_java() {
        // send_num == 0 表示不限量
        assert!(within_total_supply(0, 0));
        assert!(within_total_supply(9_999, 0));
        assert!(within_total_supply(99, 100));
        assert!(!within_total_supply(100, 100));
        assert!(!within_total_supply(101, 100));
    }

    // ————————————————————— 会员等级 / 标记 —————————————————————

    #[test]
    fn rank_rejection_matches_java() {
        // 白名单为空 → 放行（Java CollUtil.isNotEmpty 前置条件）
        assert!(!member_rank_rejected(&[], 5));
        // 命中 → 放行
        assert!(!member_rank_rejected(&[1, 3], 3));
        // 未命中 → 拒绝
        assert!(member_rank_rejected(&[1, 3], 2));
        // rank 0 也要参与判定（用户 rank_id 是 NOT NULL 裸类型）
        assert!(member_rank_rejected(&[1, 3], 0));
    }

    #[test]
    fn member_mark_rejection_matches_java() {
        // 白名单为空 → 放行
        assert!(!member_mark_rejected(&[], 4));
        assert!(!member_mark_rejected(&[1, 4], 4));
        assert!(member_mark_rejected(&[1, 2], 4));
        // mark = 0 是合法标记
        assert!(!member_mark_rejected(&[0], 0));
    }

    #[test]
    fn member_marks_json_parsing_is_tolerant() {
        assert_eq!(parse_member_marks(Some("[1,2,3]")), vec![1, 2, 3]);
        assert_eq!(parse_member_marks(Some("[ 1 , 2 ]")), vec![1, 2]);
        // 空 / 缺省 / "[]" / "null" → 空列表（等价 Java 的「无限制」）
        assert!(parse_member_marks(None).is_empty());
        assert!(parse_member_marks(Some("")).is_empty());
        assert!(parse_member_marks(Some("   ")).is_empty());
        assert!(parse_member_marks(Some("[]")).is_empty());
        assert!(parse_member_marks(Some("null")).is_empty());
        // 损坏数据 → 空列表（比 Java 抛 500 宽容，已标注 uncertain）
        assert!(parse_member_marks(Some("[1,2")).is_empty());
        assert!(parse_member_marks(Some("abc")).is_empty());
        // 字符串元素数组解析不出 i32 列表 → 空（Java 同样会抛）
        assert!(parse_member_marks(Some("[\"1\"]")).is_empty());
    }

    // ————————————————————— member_scope 判定 —————————————————————

    #[test]
    fn audience_configured_matches_is_not_blank() {
        assert!(!audience_configured(None));
        assert!(!audience_configured(Some("")));
        assert!(!audience_configured(Some("   ")));
        assert!(!audience_configured(Some("\t\n")));
        assert!(audience_configured(Some("{}")));
        assert!(audience_configured(Some("{\"scope\":0}")));
    }

    #[test]
    fn member_scope_new_structure_union_semantics() {
        assert!(member_scope_matches(
            Some(r#"{"scope":0}"#),
            7,
            1,
            0,
            &[]
        ));
        assert!(member_scope_matches(
            Some(r#"{"scope":1,"userIds":[7]}"#),
            7,
            1,
            0,
            &[]
        ));
        assert!(member_scope_matches(
            Some(r#"{"scope":1,"rankIds":[1]}"#),
            7,
            1,
            0,
            &[]
        ));
        assert!(member_scope_matches(
            Some(r#"{"scope":1,"tagIds":[9]}"#),
            7,
            1,
            0,
            &[9]
        ));
        assert!(!member_scope_matches(
            Some(r#"{"scope":1,"userIds":[8]}"#),
            7,
            1,
            0,
            &[]
        ));
        // scope=1 三组全空 → 不命中任何人
        assert!(!member_scope_matches(Some(r#"{"scope":1}"#), 7, 1, 0, &[]));
    }

    #[test]
    fn member_scope_tags_any_all_and_legacy() {
        assert!(member_scope_matches(
            Some(r#"{"scope":1,"tagIds":[9,10],"tagMatchMode":0}"#),
            7,
            1,
            0,
            &[10]
        ));
        assert!(!member_scope_matches(
            Some(r#"{"scope":1,"tagIds":[9,10],"tagMatchMode":1}"#),
            7,
            1,
            0,
            &[10]
        ));
        assert!(member_scope_matches(
            Some(r#"{"scope":1,"tagIds":[9,10],"tagMatchMode":1}"#),
            7,
            1,
            0,
            &[9, 10]
        ));
        // 旧结构
        assert!(member_scope_matches(
            Some(r#"{"audienceType":1,"rankIds":[3]}"#),
            7,
            3,
            0,
            &[]
        ));
        assert!(!member_scope_matches(
            Some(r#"{"audienceType":1,"rankIds":[3]}"#),
            7,
            2,
            0,
            &[]
        ));
    }

    #[test]
    fn member_scope_rejects_non_member_and_allows_broken_json_like_java() {
        assert!(!member_scope_matches(Some(r#"{"scope":0}"#), 7, 1, 1, &[]));
        // Java parseScope 解析失败返回 null → matchesScope 放行
        assert!(member_scope_matches(Some("not-json"), 7, 1, 0, &[]));
        assert!(member_scope_matches(None, 7, 1, 0, &[]));
    }

    // ————————————————————— 有效期 —————————————————————

    #[test]
    fn fixed_send_type_uses_coupon_dates() {
        let (s, e) = claim_time_range(1, 3, 7, 1_700_000_000, 1_600_000_000, 1_900_000_000);
        assert_eq!(s, 1_600_000_000);
        assert_eq!(e, 1_900_000_000);
    }

    #[test]
    fn by_claim_send_type_adds_delay_and_use_days() {
        let now = 1_700_000_000_i64;
        // delay_day = 2 → start = now + 2 天；use_day = 3 → end = start + 3 天
        let (s, e) = claim_time_range(0, 2, 3, now, 0, 0);
        assert_eq!(s, now + 2 * 86_400);
        assert_eq!(e, s + 3 * 86_400);
        // 60 天偏移仍在 i32 范围内（schema 里 start/end 是 `int`）
        let (s2, _) = claim_time_range(0, 60, 0, now, 0, 0);
        assert!(i32::try_from(s2).is_ok(), "60 天偏移必须仍在 i32 范围内");
        // i32::MAX ≈ 2.147e9 秒 ≈ 24 855 天；600 天仅约 5.18e7 秒，**远未**溢出
        // （now 本身约 1.7e9，加 5.18e7 仍 < 2.147e9）。
        let (s3, _) = claim_time_range(0, 600, 0, now, 0, 0);
        assert!(
            i32::try_from(s3).is_ok(),
            "600 天偏移不应溢出 i32（约 5.18e7 秒，远小于 i32::MAX）"
        );
        // 真正的溢出阈值：超过 (i32::MAX - now) / 86400 ≈ 5178 天时收窄会失败。
        // 与 Java `long` → MySQL `int` 列的隐式收窄同风险，本实现不擅自改变类型。
        let (s4, _) = claim_time_range(0, 20_000, 0, now, 0, 0);
        assert!(
            i32::try_from(s4).is_err(),
            "20000 天应超出 i32（原样照抄 Java 的收窄风险）"
        );
    }

    #[test]
    fn by_claim_with_zero_delay_is_immediately_valid() {
        let now = 1_700_000_000_i64;
        let (s, e) = claim_time_range(0, 0, 5, now, 0, 0);
        assert_eq!(s, now);
        assert_eq!(e, now + 5 * 86_400);
    }

    // ————————————————————— 文案 —————————————————————

    #[test]
    fn error_messages_match_messages_properties() {
        // 逐字对照 `java/nextrade-common/src/main/resources/i18n/messages.properties` L95-101
        assert_eq!(MSG_COUPON_NOT_EXIST, "优惠券不存在");
        assert_eq!(MSG_CLAIM_LIMIT_EXCEEDED, "已超出领取数量限制");
        assert_eq!(MSG_CLAIM_ONCE_ONLY, "该优惠券仅能领取一次");
        assert_eq!(MSG_CLAIM_NEW_USER_ONLY, "该优惠券仅限新人领取");
        assert_eq!(MSG_CLAIM_RANK_NOT_ALLOWED, "您的会员等级无法领取该优惠券");
        assert_eq!(MSG_CLAIM_MEMBER_ONLY, "该优惠券仅限指定会员领取");
        assert_eq!(MSG_CLAIM_AUDIENCE_NOT_ALLOWED, "您不在该优惠券的领取范围内");
        // Java `GlobalException(String)` 默认 code
        assert_eq!(SERVICE_DATA_ERROR, 1001);
        // promotion.member_range 指定的字面量
        assert_eq!(MEMBER_RANGE_SPECIFIED_MARKS, 1);
    }
}