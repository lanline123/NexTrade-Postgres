//! 访问统计域服务（analytics::statistics）。
//!
//! 对应原 Java：
//! - `com.nextrade.service.panel.impl.StatisticsAccessServiceImpl`
//!   → [`StatisticsService::access_statistics`]
//! - `com.nextrade.service.panel.impl.StatisticsUserServiceImpl`
//!   → [`StatisticsService::add_user_trends`] /
//!     [`StatisticsService::user_consumption_ranking`] /
//!     [`StatisticsService::user_statistics_panel`]
//! - 轴生成：`com.nextrade.service.salesman.impl.OverviewServiceImpl`
//!   的 `getHorizontalAxis` / `getLongitudinalAxis`
//! - 工具：`TigUtils#getRangeDate` / `#getGrowthRate` / `#toTimestampYmd` /
//!   `#toTimestampYmdHms` / `#toDecimalConvert`、`StringUtils#getPrevDate`、
//!   `StatisticsBaseServiceImpl#getVisitNum`
//!
//! ## 本域为什么「不像普通切片」
//!
//! 四条接口全是**报表聚合**，没有一条是单表 CRUD。取数交给
//! [`nextrade_infrastructure::db::repo::analytics::statistics`]，这里只做：
//! 参数校验 → 区间/刻度推算 → 调用仓储 → 内存归并/比率计算。
//! 所有纯计算都抽成 `pub fn`（[`build_horizontal_axis`] / [`fill_axis`] /
//! [`period_label`] / [`visit_to_user`] / [`ranking_order_by`] /
//! [`effective_page_size`] …），以便**离线单测**（仓储需要真实 MySQL）。
//!
//! ## 必须知道的 Java「怪」行为（已逐字复刻，不是笔误）
//!
//! 1. **`userStatisticsPanel` 的 `dealUserGrowthRate` /
//!    `rechargeUserGrowthRate` 实参顺序与其余三项相反**：
//!    ```java
//!    // 访问量 / 浏览量 / 新增用户：getGrowthRate(prev, current)
//!    TigUtils.getGrowthRate(BigDecimal.valueOf(prevVisitNum), BigDecimal.valueOf(visitNum));
//!    // 成交用户 / 充值用户：getGrowthRate(current, prev)  ← 反了
//!    TigUtils.getGrowthRate(BigDecimal.valueOf(dealUserNum), BigDecimal.valueOf(prevDealUserNum));
//!    ```
//!    净效果是这两项的**正负号相反**。修正会与原接口输出不一致，故照抄。
//! 2. **访问统计（`accessStatistics`）与 `getVisitNum` 是两套店铺口径**：
//!    前者无条件 `eq(shop_id, shopId)`（平台管理员拿到 `shopId = 0`，
//!    只会命中 `shop_id = 0` 的行，而写入链路写的是 `NULL` ⇒ 平台视图恒空）；
//!    后者仅 `shopId > 0` 才过滤。两个接口分别复刻各自的口径。
//! 3. **`addUserTrends` 的横轴标签与数值桶错位**：横轴按 `dateType`
//!    生成补零标签（`"01".."24"`），而 `getLongitudinalAxis` 的归桶格式
//!    是 `MM` / `dd` / `HH` —— 按日（`dateType == 3`）时横轴是
//!    `"01".."24"` 而桶是 `"00".."23"`：`0` 点的数据会**追加到数组末尾**
//!    （纵轴因此可能有 25 个元素），`"24"` 恒为 0。本实现复刻该结构。
//! 4. **`userConsumptionRanking` 的时间条件是 `add_time`，而用户面板用的是
//!    `pay_time`**；且排行的店铺条件是 `shopId != null`（含 0），
//!    面板的成交用户数是 `shopId > 0`。
//!
//! ## 时间口径
//!
//! Java 用 `ZoneId.systemDefault()`（生产为 `Asia/Shanghai`）与
//! `FROM_UNIXTIME`（会话时区）。本实现统一按 **UTC+8**；
//! 若部署时区不是 +8，趋势分桶与日边界会有小时级偏移
//! （见切片 `uncertain_points`）。
//!
//! ## 金额 / 精度
//!
//! 消费金额与所有增长率/转化率一律 `rust_decimal::Decimal`（**禁 f64**）。
//! Java `RoundingMode.HALF_UP` 对应
//! `rust_decimal::RoundingStrategy::MidpointAwayFromZero`
//! （`rust_decimal` **没有** `HalfUp` 变体）。
//!
//! ## 未迁移
//!
//! Java 的 `isExport == 1` 会把轴/明细写成 Excel 塞进
//! `HttpServletResponse`。Rust 侧无 `ExcelUtils` 等价设施，
//! **不实现导出**（仍返回同一份 JSON），已列入 `uncertain_points`。

use std::sync::Arc;

use chrono::{Datelike, Timelike};
use rust_decimal::{Decimal, RoundingStrategy};

use nextrade_core::models::analytics::statistics::{
    date_range_days, date_start_secs, days_in_month, datetime_secs_str, get_growth_rate,
    get_prev_date, get_range_date, parse_flexible_secs, parse_ymd_prefix, to_decimal_convert,
    AccessStatisticsQuery, AxisVO, UserConsumptionRankingDTO, UserConsumptionRankingVO,
    UserStatisticsPanelDTO, UserStatisticsPanelVO, UserTrendsQuery,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::analytics::statistics::{RankingQuery, StatisticsRepository};
use nextrade_infrastructure::db::Repositories;

/// 访问统计域服务（挂载于 `AdminState.statistics`）。
pub struct StatisticsService {
    repos: Arc<Repositories>,
}

impl StatisticsService {
    /// 构造（与其它切片一致的 `new(Arc<Repositories>)`）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &StatisticsRepository {
        &self.repos.analytics.statistics
    }

    // ————————————————————— 1) 访问统计 —————————————————————

    /// 访问统计趋势（Java `StatisticsAccessServiceImpl#getAccessStatistics`，
    /// 控制器 `StatisticsAccessController.accessStatistics`）。
    ///
    /// Java 逐步（`StatisticsAccessServiceImpl.java:36`）：
    /// 1. `startTime == null || endTime == null` → `GlobalException("请选择日期")`
    ///    （**只判 null**，空串会走到日期解析并抛异常）。
    /// 2. `isHits > 0` → 取 `click_count`，否则取 `visitor_count`。
    ///    ⚠️ `isHits` 是包装 `Integer` 且 `> 0` **无 null 保护**，
    ///    缺失时 Java 直接 NPE（HTTP 500）；本实现按「缺失 = 0 = 访客数」
    ///    处理（见 `uncertain_points`）。
    /// 3. `between(date, start, end)` + **无条件** `eq(shop_id, shopId)`
    ///    —— 与 `getVisitNum` 的「`shopId > 0` 才过滤」不同；`shopId` 为
    ///    `null` 时退化成 `shop_id = NULL`（查不到任何行）。
    /// 4. 横轴 = `getHorizontalAxis(0, start, end)`（固定逐日
    ///    `yyyy-MM-dd`），纵轴 = `getLongitudinalAxis(.., 0, 2)`。
    pub async fn access_statistics(&self, q: &AccessStatisticsQuery) -> Result<AxisVO> {
        let start = q
            .start_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请选择日期".into()))?;
        let end = q
            .end_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请选择日期".into()))?;

        // 横轴固定 dateType = 0（自定义 / 逐日）
        let horizontal = build_horizontal_axis(0, start, end)?;

        // 平台管理员前端不传 shopId（Java HeaderUtils 默认 0），
        // statistics_base 的平台行 shop_id=0；None 不能直接当空结果。
        let shop_id = q.shop_id.unwrap_or(0);
        let rows = self.repo().fetch_daily_visits(start, end, shop_id).await?;

        let use_click = q.is_hits.unwrap_or(0) > 0;
        let items: Vec<(String, Decimal)> = rows
            .into_iter()
            .map(|r| {
                let value = if use_click {
                    r.click_count
                } else {
                    r.visitor_count
                };
                (r.period, Decimal::from(value))
            })
            .collect();

        Ok(AxisVO {
            horizontal_axis: horizontal.clone(),
            longitudinal_axis: fill_axis(&horizontal, 0, &items),
        })
    }

    // ————————————————————— 2) 新增会员趋势 —————————————————————

    /// 新增会员趋势（Java `StatisticsUserServiceImpl#getAddUserTrends`，
    /// 控制器 `StatisticsUserController.addUserTrends`）。
    ///
    /// Java 逐步（`StatisticsUserServiceImpl.java:136`）：
    /// 1. `@Validated`：`startEndTime` `@NotBlank("请选择日期")`、
    ///    `dateType` `@NotNull("数据类型不能为空")`。
    /// 2. `transStartEndTimeStr()` = `getRangeDate(dateType, s, s)`（字符串），
    ///    横轴 = `getHorizontalAxis(dateType, str[0], str[1])`。
    /// 3. `transStartEndTime()` = 同一对字符串经 hutool `DateUtil.parse(...)`
    ///    转**秒**（`yyyy-MM-dd HH:mm:ss` 也能解析）；明细 = `user` 表
    ///    `reg_time BETWEEN` 两端。
    ///    ⚠️ Java **完全没有** `shop_id` 条件（`query.shopId` 读了但没用）。
    /// 4. 纵轴 = `getLongitudinalAxis(horizontal, rows, dateType, 1)`；
    ///    见模块头第 3 条：按日时桶与标签会错位。
    ///
    /// ⚠️ `isExport == 1` 时 Java 额外导 Excel；Rust 侧不实现导出。
    pub async fn add_user_trends(&self, q: &UserTrendsQuery) -> Result<AxisVO> {
        let raw = q
            .start_end_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请选择日期".into()))?;
        let date_type = q
            .date_type
            .ok_or_else(|| Error::Validation("数据类型不能为空".into()))?;

        // Java：getRangeDate(dateType, startEndTime, startEndTime)
        let (start_str, end_str) = get_range_date(Some(date_type), Some(raw), Some(raw))
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {raw}")))?;

        let horizontal = build_horizontal_axis(date_type, &start_str, &end_str)?;

        let start_ts = parse_flexible_secs(&start_str)
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {start_str}")))?;
        let end_ts = parse_flexible_secs(&end_str)
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {end_str}")))?;

        let reg_times = self.repo().fetch_user_reg_times(start_ts, end_ts).await?;
        let items: Vec<(String, Decimal)> = reg_times
            .into_iter()
            .map(|t| (period_label(date_type, t), Decimal::ONE))
            .collect();

        Ok(AxisVO {
            horizontal_axis: horizontal.clone(),
            longitudinal_axis: fill_axis(&horizontal, date_type, &items),
        })
    }

    // ————————————————————— 3) 会员消费排行 —————————————————————

    /// 会员消费排行（Java `StatisticsUserServiceImpl#getUserConsumptionRanking`，
    /// 控制器 `StatisticsUserController.userConsumptionRanking`）。
    ///
    /// Java 逐步（`StatisticsUserServiceImpl.java:168`）：
    /// - `order LEFT JOIN user ON user.user_id = order.user_id`
    /// - `SELECT t1.username, t1.mobile, COUNT(t.order_id) order_num,
    ///   SUM(t.total_amount) order_amount`
    /// - `WHERE pay_status = 2`（**等于**，不是 `>=`）
    /// - `shopId != null` 才 `eq(shop_id, shopId)`（含 `0`）
    /// - `between(add_time, toTimestampYmd(start), toTimestampYmd(end))`
    /// - `keyword` 非空时 `username LIKE %k% OR mobile LIKE %k%`
    /// - `sortField` 白名单映射（`orderNum`/`order_num` → `order_num`，
    ///   `orderAmount`/`order_amount` → `order_amount`；其它原样拼进 ORDER BY）
    /// - `GROUP BY t1.user_id`
    /// - `Page<>(page, size)`
    ///
    /// 差异（见 `uncertain_points`）：
    /// - `sortField` 不在白名单内时 Java 会把用户输入直接拼进 `ORDER BY`
    ///   （注入面），Rust 侧改为**忽略排序**；
    /// - 未传 `sortField` 时 Java 不加 `ORDER BY`，本实现保持一致；
    /// - `size` 缺省按原 `BasePage` 语义取 15，`> 100` 夹到 100。
    pub async fn user_consumption_ranking(
        &self,
        dto: &UserConsumptionRankingDTO,
    ) -> Result<PageResult<UserConsumptionRankingVO>> {
        let start_raw = dto
            .start_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let end_raw = dto
            .end_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        // Java 的时间条件只校验 startTime（`isNotBlank(startTime) &&
        // isNotBlank(startTime)`，endTime 未校验）；这里：
        // startTime 为空 ⇒ 不加时间条件；startTime 有值而 endTime 为空 ⇒ 报错。
        let (start_ts, end_ts) = match start_raw {
            None => (None, None),
            Some(s) => {
                let e = end_raw.ok_or_else(|| Error::Validation("请选择结束时间".into()))?;
                let s_ts = date_start_secs(s)
                    .ok_or_else(|| Error::Validation(format!("开始时间格式不正确: {s}")))?;
                let e_ts = date_start_secs(e)
                    .ok_or_else(|| Error::Validation(format!("结束时间格式不正确: {e}")))?;
                (Some(s_ts), Some(e_ts))
            }
        };

        let page = effective_page(dto.page);
        let size = effective_page_size(dto.size);
        let query = PageQuery {
            page,
            page_size: size,
        };
        let (limit, offset) = query.limit_offset();

        let rq = RankingQuery {
            start_ts,
            end_ts,
            shop_id: dto.shop_id,
            keyword: dto
                .keyword
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string),
            order_by: ranking_order_by(dto.sort_field.as_deref(), dto.sort_order.as_deref()),
            limit,
            offset,
        };

        let (total, rows) = self.repo().fetch_consumption_ranking(&rq).await?;
        let list = rows
            .into_iter()
            .map(|r| UserConsumptionRankingVO {
                username: r.username,
                mobile: r.mobile,
                order_num: r.order_num,
                order_amount: r.order_amount,
            })
            .collect();

        Ok(PageResult::new(total, list, &query))
    }

    // ————————————————————— 4) 用户统计面板 —————————————————————

    /// 用户统计面板（Java `StatisticsUserServiceImpl#getUserStatisticsPanel`，
    /// 控制器 `StatisticsUserController.userStatisticsPanel`）。
    ///
    /// Java 逐步（`StatisticsUserServiceImpl.java:62`）：
    /// 1. `getPrevDate([startTime, endTime], 4)` 得环比区间（见
    ///    [`get_prev_date`]，会与本期重叠）。
    /// 2. `visitNum = getVisitNum([start, end], 0, 0, 0)`（访客数合计，**不过滤店铺**）；
    ///    `viewNum = getVisitNum([start, end], 1, 0, 0)`（浏览量合计）。
    /// 3. `addUserNum`：`user.reg_time BETWEEN start 00:00:00 AND end 23:59:59`
    ///    （`toTimestampYmdHms`；不过滤店铺）。
    /// 4. `dealUserNum`：`order` 去重 `user_id`，`pay_status >= 2`
    ///    （**`ge`，含退款中/已退款**）、`pay_time BETWEEN`、
    ///    `shop_id > 0` 才过滤。
    /// 5. `rechargeUserNum`：`user_recharge_order` 去重 `user_id`，
    ///    `add_time BETWEEN`（无状态过滤、无店铺过滤）。
    /// 6. `visitToUser = addUserNum / viewNum × 100`（`viewNum == 0` ⇒ 0），
    ///    两位小数 `HALF_UP`；`visitToUserRate` 是它的环比（上期为 0 ⇒ 0）。
    /// 7. 五项增长率中，**成交用户 / 充值用户两项实参顺序与 Java 其余项相反**
    ///    （见模块头第 1 条），此处逐字复刻。
    ///
    /// ⚠️ `isExport == 1` 时 Java 额外导 Excel；Rust 侧不实现导出。
    pub async fn user_statistics_panel(
        &self,
        dto: &UserStatisticsPanelDTO,
    ) -> Result<UserStatisticsPanelVO> {
        let start = dto
            .start_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请选择日期".into()))?;
        let end = dto
            .end_time
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请选择日期".into()))?;

        let (prev_start, prev_end) = get_prev_date(start, end)
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {start} ~ {end}")))?;

        // ① 访问量（访客数）/ 浏览量（点击）
        let (visit_num, view_num) = self.repo().sum_visit_counts(start, end).await?;
        let (prev_visit_num, prev_view_num) = self
            .repo()
            .sum_visit_counts(&prev_start, &prev_end)
            .await?;

        // ② 秒级边界：yyyy-MM-dd → 当日 00:00:00 / 23:59:59
        let start_ts = datetime_secs_str(&format!("{start} 00:00:00"))
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {start}")))?;
        let end_ts = datetime_secs_str(&format!("{end} 23:59:59"))
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {end}")))?;
        let prev_start_ts = datetime_secs_str(&format!("{prev_start} 00:00:00"))
            .ok_or_else(|| Error::Validation("环比区间异常".into()))?;
        let prev_end_ts = datetime_secs_str(&format!("{prev_end} 23:59:59"))
            .ok_or_else(|| Error::Validation("环比区间异常".into()))?;

        // ③ 新增用户 / 成交用户 / 充值用户（本期 + 上期）
        let add_user_num = self.repo().count_users_by_reg_time(start_ts, end_ts).await?;
        let prev_add_user_num = self
            .repo()
            .count_users_by_reg_time(prev_start_ts, prev_end_ts)
            .await?;
        let deal_user_num = self
            .repo()
            .count_paid_order_users(start_ts, end_ts, dto.shop_id)
            .await?;
        let prev_deal_user_num = self
            .repo()
            .count_paid_order_users(prev_start_ts, prev_end_ts, dto.shop_id)
            .await?;
        let recharge_user_num = self.repo().count_recharge_users(start_ts, end_ts).await?;
        let prev_recharge_user_num = self
            .repo()
            .count_recharge_users(prev_start_ts, prev_end_ts)
            .await?;

        // ④ 环比增长率
        // ⚠️ 前两项（访问量/浏览量）以及新增用户是 getGrowthRate(prev, current)，
        //    而成交用户/充值用户 Java 写成了 getGrowthRate(current, prev)，
        //    故下面两行的实参顺序刻意与上面不同（逐字复刻）。
        let visit_growth_rate = get_growth_rate(
            Some(Decimal::from(prev_visit_num)),
            Some(Decimal::from(visit_num)),
        );
        let view_growth_rate = get_growth_rate(
            Some(Decimal::from(prev_view_num)),
            Some(Decimal::from(view_num)),
        );
        let add_user_growth_rate = get_growth_rate(
            Some(Decimal::from(prev_add_user_num)),
            Some(Decimal::from(add_user_num)),
        );
        let deal_user_growth_rate = get_growth_rate(
            Some(Decimal::from(deal_user_num)),
            Some(Decimal::from(prev_deal_user_num)),
        );
        let recharge_user_growth_rate = get_growth_rate(
            Some(Decimal::from(recharge_user_num)),
            Some(Decimal::from(prev_recharge_user_num)),
        );

        // ⑤ 访客-支付转化率（Java 实为「新增用户 / 浏览量 × 100」，逐字复刻）
        let visit_to_user_value = visit_to_user(add_user_num, view_num);
        let prev_visit_to_user_value = visit_to_user(prev_add_user_num, prev_view_num);
        let visit_to_user_rate = if prev_visit_to_user_value.is_zero() {
            Decimal::ZERO
        } else {
            get_growth_rate(Some(prev_visit_to_user_value), Some(visit_to_user_value))
        };

        Ok(UserStatisticsPanelVO {
            visit_num,
            visit_growth_rate,
            view_num,
            view_growth_rate,
            add_user_num,
            add_user_growth_rate,
            deal_user_num,
            deal_user_growth_rate,
            visit_to_user: visit_to_user_value,
            visit_to_user_rate,
            recharge_user_num,
            recharge_user_growth_rate,
        })
    }
}

// ——————————————————————————————————————————————————————————————
// 纯逻辑（离线可测）
// ——————————————————————————————————————————————————————————————

/// 生成横轴刻度（复刻 Java `OverviewServiceImpl#getHorizontalAxis`）。
///
/// - `date_type == 1`（年）→ `["01".."12"]`
/// - `date_type == 2`（月）→ `["01".."{当月天数}"]`（由 `start` 的年月决定）
/// - `date_type == 3`（日）→ `["01".."24"]`（**1 到 24**，不是 0 到 23）
/// - 其它（含 `0`）→ `start` 到 `end` 的**逐日** `yyyy-MM-dd`
///
/// ⚠️ Java 的 `case 3` 不解析日期参数；只有 case 2 与 default 会解析。
/// 解析失败时 Java 抛 `GlobalException`，这里返回 [`Error::Validation`]。
pub fn build_horizontal_axis(date_type: i32, start: &str, end: &str) -> Result<Vec<String>> {
    match date_type {
        1 => Ok((1..=12).map(|i| format!("{i:02}")).collect()),
        2 => {
            let (y, m) = parse_ymd_prefix(start)
                .map(|(y, m, _)| (y, m))
                .ok_or_else(|| Error::Validation(format!("日期格式不正确: {start}")))?;
            let days = days_in_month(y, m);
            Ok((1..=days).map(|i| format!("{i:02}")).collect())
        }
        3 => Ok((1..=24).map(|i| format!("{i:02}")).collect()),
        _ => date_range_days(start, end)
            .ok_or_else(|| Error::Validation(format!("日期格式不正确: {start} ~ {end}"))),
    }
}

/// 把「刻度 → 值」按 Java `LinkedHashMap.merge(..., BigDecimal::add)` 的
/// 语义压平成纵轴数组。
///
/// 初始键：
/// - `date_type ∈ 1..=3` → `"01"`、`"02"`…（与横轴下标对齐，即 `i + 1`）
/// - 其它 → 直接用横轴标签
///
/// 命中已有键则累加；**未命中的键会被追加到数组末尾**（这是 Java 的行为：
/// `dateType == 3` 时 `"00"` 桶不在初始键里，会导致纵轴比横轴多一个元素）。
#[must_use]
pub fn fill_axis(
    horizontal: &[String],
    date_type: i32,
    items: &[(String, Decimal)],
) -> Vec<Decimal> {
    let mut keys: Vec<String> = horizontal
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if (1..=3).contains(&date_type) {
                format!("{:02}", i + 1)
            } else {
                label.clone()
            }
        })
        .collect();
    let mut values = vec![Decimal::ZERO; keys.len()];
    for (range, value) in items {
        match keys.iter().position(|k| k == range) {
            Some(idx) => values[idx] += *value,
            None => {
                keys.push(range.clone());
                values.push(*value);
            }
        }
    }
    values
}

/// 把秒级时间戳按 `date_type` 折算成归桶键（复刻 Java
/// `getLongitudinalAxis` 里 `SimpleDateFormat` 的 `MM` / `dd` / `HH` /
/// `yyyy-MM-dd`，**UTC+8**）。
#[must_use]
pub fn period_label(date_type: i32, secs: i64) -> String {
    let Some(utc) = chrono::DateTime::from_timestamp(secs, 0) else {
        return String::new();
    };
    let local = utc.naive_utc() + chrono::Duration::hours(8);
    match date_type {
        1 => format!("{:02}", local.month()),
        2 => format!("{:02}", local.day()),
        3 => format!("{:02}", local.hour()),
        _ => local.format("%Y-%m-%d").to_string(),
    }
}

/// 访客-支付转化率（复刻 Java `getUserStatisticsPanel` 里的
/// `toDecimalConvert(addUserNum / viewNum × 100)`）。
///
/// `view_num == 0` ⇒ `0`（Java 不进入计算分支，返回 `BigDecimal.ZERO`）。
/// 除法先按 2 位 `HALF_UP`，乘 100 后再按 2 位 `HALF_UP`。
#[must_use]
pub fn visit_to_user(add_user_num: i64, view_num: i64) -> Decimal {
    if view_num == 0 {
        return Decimal::ZERO;
    }
    let ratio = Decimal::from(add_user_num) / Decimal::from(view_num);
    let ratio = ratio.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    to_decimal_convert(ratio * Decimal::from(100))
}

/// `sortField` / `sortOrder` → `(列名, 是否升序)` 白名单映射。
///
/// Java 只把 `orderNum`/`order_num` 与 `orderAmount`/`order_amount`
/// 归一化，其它值**原样拼进 `ORDER BY`**（注入面）；Rust 侧只认白名单，
/// 其余一律不加排序（见 `uncertain_points`）。
#[must_use]
pub fn ranking_order_by(
    sort_field: Option<&str>,
    sort_order: Option<&str>,
) -> Option<(&'static str, bool)> {
    let field = sort_field.map(str::trim).filter(|s| !s.is_empty())?;
    let column = match field {
        "orderNum" | "order_num" => "order_num",
        "orderAmount" | "order_amount" => "order_amount",
        _ => return None,
    };
    Some((column, sort_order == Some("asc")))
}

/// 每页大小（原 `BasePage.getSize()` 语义：缺省 15，上限 100）。
///
/// ⚠️ Java `BasePage.getSize()` 只夹上界（`> 100` 变 100），不夹下界；
/// 这里为 `PageQuery` 的 `LIMIT` 安全起见把下界收到 1（`0` 会返回空页）。
#[must_use]
pub fn effective_page_size(size: Option<u64>) -> u64 {
    match size {
        None => 15,
        Some(v) => v.clamp(1, 100),
    }
}

/// 页码（缺省 1）。
#[must_use]
pub fn effective_page(page: Option<u64>) -> u64 {
    page.unwrap_or(1).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        s.parse().expect("合法 Decimal 字面量")
    }

    /// 年：固定 12 个月，且**不解析日期参数**。
    #[test]
    fn horizontal_axis_year_is_fixed_twelve() {
        let axis = build_horizontal_axis(1, "", "").expect("年分支不解析日期");
        assert_eq!(axis.len(), 12);
        assert_eq!(axis.first().map(String::as_str), Some("01"));
        assert_eq!(axis.last().map(String::as_str), Some("12"));
    }

    /// 月：天数由 start 的年月决定（闰年感知）。
    #[test]
    fn horizontal_axis_month_uses_start_month() {
        assert_eq!(build_horizontal_axis(2, "2024-02-01", "").unwrap().len(), 29);
        assert_eq!(build_horizontal_axis(2, "2023-02-01", "").unwrap().len(), 28);
        assert_eq!(build_horizontal_axis(2, "2024-05-01", "").unwrap().len(), 31);
        let axis = build_horizontal_axis(2, "2024-05-01", "").unwrap();
        assert_eq!(axis.first().map(String::as_str), Some("01"));
        assert_eq!(axis.last().map(String::as_str), Some("31"));
        assert!(build_horizontal_axis(2, "not-a-date", "").is_err());
    }

    /// 日：**1..24**（不是 0..23），且不解析日期。
    #[test]
    fn horizontal_axis_day_is_one_to_twenty_four() {
        let axis = build_horizontal_axis(3, "ignored", "ignored").unwrap();
        assert_eq!(axis.len(), 24);
        assert_eq!(axis.first().map(String::as_str), Some("01"));
        assert_eq!(axis.last().map(String::as_str), Some("24"));
        assert!(!axis.iter().any(|s| s == "00"), "横轴没有 00 这一格");
    }

    /// 自定义（0）：逐日，含两端。
    #[test]
    fn horizontal_axis_custom_is_daily() {
        let axis = build_horizontal_axis(0, "2024-05-01", "2024-05-03").unwrap();
        assert_eq!(axis, vec!["2024-05-01", "2024-05-02", "2024-05-03"]);
        assert!(build_horizontal_axis(0, "2024-05-03", "2024-05-01")
            .unwrap()
            .is_empty());
        assert!(build_horizontal_axis(0, "x", "2024-05-01").is_err());
    }

    /// 纵轴：刻度命中累加，未命中追加到末尾。
    #[test]
    fn fill_axis_sums_and_appends() {
        let horizontal: Vec<String> = vec!["2024-05-01".into(), "2024-05-02".into()];
        let items = vec![
            ("2024-05-02".to_string(), dec("5")),
            ("2024-05-02".to_string(), dec("2")),
            ("2024-05-01".to_string(), dec("1")),
        ];
        assert_eq!(
            fill_axis(&horizontal, 0, &items),
            vec![dec("1"), dec("7")],
            "同刻度应累加"
        );

        // 未命中的键追加到末尾（Java LinkedHashMap.merge 行为）
        let items = vec![("2029-01-01".to_string(), dec("9"))];
        assert_eq!(
            fill_axis(&horizontal, 0, &items),
            vec![dec("0"), dec("0"), dec("9")]
        );
    }

    /// ⚠️ `dateType == 3` 的错位：桶是 `"00".."23"`，横轴是 `"01".."24"`，
    /// 因此 0 点会被追加到数组最后一个位置（纵轴长度 25）。
    #[test]
    fn fill_axis_hour_bucket_off_by_one() {
        let horizontal = build_horizontal_axis(3, "2024-05-06", "2024-05-06").unwrap();
        assert_eq!(horizontal.len(), 24);

        // 1 点 → 桶 "01"（横轴第 1 格，下标 0）
        let items = vec![("01".to_string(), dec("7"))];
        let axis = fill_axis(&horizontal, 3, &items);
        assert_eq!(axis.len(), 24);
        assert_eq!(axis[0], dec("7"));
        assert_eq!(axis[23], dec("0"), "\"24\" 恒为 0");

        // 0 点 → 桶 "00" 不在初始键里 ⇒ 追加到末尾（长度 25）
        let items = vec![("00".to_string(), dec("3"))];
        let axis = fill_axis(&horizontal, 3, &items);
        assert_eq!(axis.len(), 25);
        assert_eq!(axis[24], dec("3"));
        assert_eq!(axis[0], dec("0"), "横轴第一格其实是 1 点");
    }

    /// 年 / 月的桶键与横轴标签恰好一致（`MM` / `dd`）。
    #[test]
    fn fill_axis_year_and_month_align() {
        let year_axis = build_horizontal_axis(1, "2024", "2024").unwrap();
        let axis = fill_axis(&year_axis, 1, &[("12".to_string(), dec("4"))]);
        assert_eq!(axis.len(), 12);
        assert_eq!(axis[11], dec("4"));

        let month_axis = build_horizontal_axis(2, "2024-05-01", "").unwrap();
        let axis = fill_axis(&month_axis, 2, &[("31".to_string(), dec("2"))]);
        assert_eq!(axis.len(), 31);
        assert_eq!(axis[30], dec("2"));
    }

    /// 归桶键按 **UTC+8** 折算。
    #[test]
    fn period_label_uses_utc8() {
        // 0 秒 == 1970-01-01 08:00:00 (UTC+8)
        assert_eq!(period_label(0, 0), "1970-01-01");
        assert_eq!(period_label(1, 0), "01");
        assert_eq!(period_label(2, 0), "01");
        assert_eq!(period_label(3, 0), "08");

        // 2024-05-06 10:30:00 (UTC+8)
        let secs = datetime_secs_str("2024-05-06 10:30:00").expect("合法时间");
        assert_eq!(period_label(0, secs), "2024-05-06");
        assert_eq!(period_label(1, secs), "05");
        assert_eq!(period_label(2, secs), "06");
        assert_eq!(period_label(3, secs), "10");
    }

    /// 转化率：先除（2 位 HALF_UP）再乘 100（再 2 位 HALF_UP）。
    #[test]
    fn visit_to_user_matches_java() {
        assert_eq!(visit_to_user(1, 2), dec("50.00"));
        assert_eq!(visit_to_user(1, 3), dec("33.00"));
        assert_eq!(visit_to_user(10, 4), dec("250.00"));
        assert_eq!(visit_to_user(0, 5), dec("0.00"));
        // viewNum == 0 ⇒ 0（Java 不进入计算分支）
        assert_eq!(visit_to_user(7, 0), Decimal::ZERO);
        // 除法的 HALF_UP：2/3 = 0.67 → 67.00
        assert_eq!(visit_to_user(2, 3), dec("67.00"));
        // 结果保留 2 位
        assert_eq!(visit_to_user(1, 3).scale(), 2);
    }

    /// `sortField` 白名单映射。
    #[test]
    fn ranking_order_by_whitelist() {
        assert_eq!(
            ranking_order_by(Some("orderNum"), Some("asc")),
            Some(("order_num", true))
        );
        assert_eq!(
            ranking_order_by(Some("order_num"), Some("desc")),
            Some(("order_num", false))
        );
        assert_eq!(
            ranking_order_by(Some("orderAmount"), Some("asc")),
            Some(("order_amount", true))
        );
        assert_eq!(
            ranking_order_by(Some("order_amount"), None),
            Some(("order_amount", false))
        );
        // 非白名单 → 不加排序（Java 会原样拼接，存在注入面）
        assert_eq!(ranking_order_by(Some("username"), Some("asc")), None);
        assert_eq!(ranking_order_by(Some("order_num; DROP TABLE x"), None), None);
        assert_eq!(ranking_order_by(Some("   "), Some("asc")), None);
        assert_eq!(ranking_order_by(None, Some("asc")), None);
    }

    /// 分页夹取。
    #[test]
    fn paging_bounds() {
        assert_eq!(effective_page_size(None), 15);
        assert_eq!(effective_page_size(Some(25)), 25);
        assert_eq!(effective_page_size(Some(100)), 100);
        assert_eq!(effective_page_size(Some(10_000)), 100);
        assert_eq!(effective_page_size(Some(0)), 1);

        assert_eq!(effective_page(None), 1);
        assert_eq!(effective_page(Some(0)), 1);
        assert_eq!(effective_page(Some(3)), 3);
    }

    /// 增长率实参顺序：成交/充值用户与 Java 一样是「反的」，
    /// 因此「增长」会显示成负数。
    #[test]
    fn growth_rate_argument_order_for_deal_users() {
        // 正常口径：prev=10, current=20 ⇒ +100
        assert_eq!(
            get_growth_rate(Some(dec("10")), Some(dec("20"))),
            dec("100.0000")
        );
        // Java 的反向口径（getGrowthRate(current, prev)）：prev 位置传 current
        assert_eq!(
            get_growth_rate(Some(dec("20")), Some(dec("10"))),
            dec("-50.0000")
        );
    }

    /// 环比区间按 `getPrevDate(.., 4)`：区间天数不变但会重叠。
    #[test]
    fn panel_prev_range_overlaps() {
        assert_eq!(
            get_prev_date("2024-05-01", "2024-05-31"),
            Some(("2024-04-01".to_string(), "2024-05-01".to_string()))
        );
        // 由环比区间再算秒级边界，仍然是合法时间
        let start_ts = datetime_secs_str("2024-04-01 00:00:00").expect("合法");
        let end_ts = datetime_secs_str("2024-05-01 23:59:59").expect("合法");
        assert!(end_ts > start_ts);
    }
}