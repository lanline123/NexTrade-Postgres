//! 抽奖（客户端）域服务（marketing::lottery）。
//!
//! Java：`LotteryController` → `LotteryDrawServiceImpl` / `LotteryRecordServiceImpl`。
//! 抽奖算法按 `grid_config` 概率加权 + 奖品库存条件扣减（DB 事务），
//! 概率随机源用毫秒时间戳取模（Java `Random`；Rust 本切片未引入 rand 依赖，见偏差）。
//!
//! # 偏差
//!
//! 1. 随机数：Java `new Random().nextInt(1000000)/10000`；Rust 用
//!    `now_millis() % 1_000_000 / 10000`（均匀性略差，测试可注入随机值）。
//! 2. 商品奖品（type=2）的**商品/SKU 库存过滤与扣减、积分/优惠券发放**依赖商品、
//!    优惠券域服务：本实现只扣 `lottery_prize.prize_stock_used` 并把奖品原文记录，
//!    商品库存不再联动（Java 会 `productService.decStock`/`productSkuService.decStock`
//!    与 `grantPrize` 发放积分/优惠券）。抽奖中奖站内信未迁移。
//! 3. `info` 的 `decorateConfig` 原样返回 JSON（Java 反序列化为对象）；`prizeEntryImage`
//!    Java 未赋值 ⇒ null。
//! 4. `confirmReceive` 的 `orderId` 参数仅接收不使用（Java 仅记日志）。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_common::utils::{now_millis, now_seconds};
use nextrade_core::entities::promotion::{LotteryActivity, LotteryPrize, LotteryRecord};
use nextrade_core::models::marketing::lottery::{
    format_time, mask_nickname, parse_grid, record_status_text, GridPosition, GridPositionVO,
    LotteryActivityInfoVO, LotteryConfirmReceiveParam, LotteryRecordQuery, LotteryRecordVO,
    LotteryResultVO, LotteryWinnerVO, LotteryWinnersQuery, COST_TYPE_POINTS, MSG_ACTIVITY_NOT_EXISTS,
    MSG_ALREADY_RECEIVED, MSG_ENDED, MSG_NOT_LOGGED_IN, MSG_NOT_STARTED, MSG_ONLY_PRODUCT_RECEIVE,
    MSG_POINTS_NOT_ENOUGH, MSG_RECORD_NOT_EXISTS, MSG_TODAY_LIMIT, MSG_TOTAL_LIMIT,
    MSG_WIN_LIMIT, NO_WIN_NAME, PRIZE_TYPE_PRODUCT,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::lottery::LotteryRepository;
use nextrade_infrastructure::db::Repositories;

/// 抽奖域服务。
pub struct LotteryService {
    repos: Arc<Repositories>,
}

impl LotteryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &LotteryRepository {
        &self.repos.marketing.lottery
    }

    /// 当前进行中的活动信息（Java `getOngoingActivityInfo`）。
    pub async fn info(&self, user_id: Option<i32>) -> Result<LotteryActivityInfoVO> {
        let user_id = require_user(user_id)?;
        let now = now_seconds();
        let activity = self
            .repo()
            .find_ongoing(now)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_NOT_EXISTS))?;
        validate_activity(&activity, now)?;
        let user = self.repo().find_user(user_id).await?.unwrap_or_default();
        let prizes = self.repo().find_prizes(activity.activity_id).await?;
        let grid_config = build_grid_vo(&activity, &prizes);
        let stat = self.repo().find_stat(activity.activity_id, user_id).await?;
        let (today_count, total_count, win_count) = stat
            .as_ref()
            .map(|s| (s.today_count, s.total_count, s.win_count))
            .unwrap_or((0, 0, 0));
        let need_points = activity.cost_type != 0;
        let available_draw_count = if need_points {
            let cost = activity.cost_points.max(1);
            user.points / cost
        } else {
            -1
        };
        Ok(LotteryActivityInfoVO {
            activity_id: activity.activity_id,
            activity_name: activity.activity_name.clone(),
            start_time: format_time(i64::from(activity.start_time)),
            end_time: format_time(i64::from(activity.end_time)),
            cost_type: activity.cost_type,
            cost_points: activity.cost_points,
            decorate_config: activity
                .decorate_config
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            no_win_name: activity.no_win_name.clone(),
            no_win_image: activity.no_win_image.clone(),
            no_win_tips: activity.no_win_tips.clone(),
            prize_entry_image: None,
            grid_config,
            user_points: user.points,
            today_remaining: remaining(activity.daily_limit, today_count),
            total_remaining: remaining(activity.total_limit, total_count),
            win_remaining: remaining(activity.win_limit, win_count),
            available_draw_count,
        })
    }

    /// 执行抽奖（Java `drawOngoingActivity`）。
    pub async fn draw(&self, user_id: Option<i32>) -> Result<LotteryResultVO> {
        let user_id = require_user(user_id)?;
        let now_i64 = now_seconds();
        let now = i32::try_from(now_i64).unwrap_or_default();
        let activity = self
            .repo()
            .find_ongoing(now_i64)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_NOT_EXISTS))?;
        validate_activity(&activity, now_i64)?;
        let stat = self.repo().find_stat(activity.activity_id, user_id).await?;
        let (today_count, total_count, win_count) = stat
            .as_ref()
            .map(|s| (s.today_count, s.total_count, s.win_count))
            .unwrap_or((0, 0, 0));
        if activity.daily_limit > 0 && today_count >= activity.daily_limit {
            return Err(Error::biz(500, MSG_TODAY_LIMIT));
        }
        if activity.total_limit > 0 && total_count >= activity.total_limit {
            return Err(Error::biz(500, MSG_TOTAL_LIMIT));
        }
        if activity.win_limit > 0 && win_count >= activity.win_limit {
            return Err(Error::biz(500, MSG_WIN_LIMIT));
        }
        let need_points = activity.cost_type != 0;
        if need_points {
            let user = self.repo().find_user(user_id).await?.unwrap_or_default();
            if user.points < activity.cost_points {
                return Err(Error::biz(500, MSG_POINTS_NOT_ENOUGH));
            }
        }

        let grid = parse_grid(activity.grid_config.as_deref());
        let prize_ids: Vec<i32> = grid.iter().filter_map(|g| g.prize_id).filter(|id| *id > 0).collect();
        let prizes = self.repo().find_prizes_by_ids(&prize_ids).await?;
        let random = Decimal::new(i64::from((now_millis() % 1_000_000) as i32), 4);
        let (mut prize, position) = pick_winner(&grid, &prizes, random);

        let today = today_string(now_i64);
        let mut tx = self.repo().begin().await?;
        if let Some(candidate) = prize.clone() {
            let affected = self
                .repo()
                .deduct_prize_stock_in_tx(&mut tx, candidate.prize_id, now)
                .await?;
            if affected == 0 {
                prize = None;
            }
        }
        let is_win = prize.is_some();
        let record = if let Some(p) = prize.as_ref() {
            LotteryRecord {
                record_id: 0,
                activity_id: activity.activity_id,
                user_id,
                prize_id: p.prize_id,
                prize_name: p.prize_name.clone(),
                prize_type: p.prize_type,
                prize_value: p.prize_value.clone(),
                prize_image: Some(p.prize_image.clone()),
                cost_points: if need_points { activity.cost_points } else { 0 },
                is_win: 1,
                status: if p.prize_type == PRIZE_TYPE_PRODUCT { 0 } else { 1 },
                receive_time: if p.prize_type == PRIZE_TYPE_PRODUCT { None } else { Some(now) },
                create_time: now,
            }
        } else {
            LotteryRecord {
                record_id: 0,
                activity_id: activity.activity_id,
                user_id,
                prize_id: 0,
                prize_name: NO_WIN_NAME.to_string(),
                prize_type: 0,
                prize_value: String::new(),
                prize_image: Some(String::new()),
                cost_points: if need_points { activity.cost_points } else { 0 },
                is_win: 0,
                status: 1,
                receive_time: Some(now),
                create_time: now,
            }
        };
        let record_id = self.repo().insert_record_in_tx(&mut tx, &record).await?;
        if need_points && activity.cost_points > 0 {
            let desc = format!("参与抽奖活动:{}", activity.activity_name);
            let affected = self
                .repo()
                .deduct_points_in_tx(&mut tx, user_id, activity.cost_points, &desc, activity.activity_id, now)
                .await?;
            if affected == 0 {
                return Err(Error::biz(500, MSG_POINTS_NOT_ENOUGH));
            }
        }
        self.repo()
            .upsert_stat_in_tx(&mut tx, activity.activity_id, user_id, is_win, &today, now)
            .await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let updated = self
            .repo()
            .find_stat(activity.activity_id, user_id)
            .await?
            .unwrap_or_default();
        Ok(LotteryResultVO {
            is_win,
            prize_id: prize.as_ref().map(|p| p.prize_id),
            prize_name: prize.as_ref().map(|p| p.prize_name.clone()),
            prize_type: prize.as_ref().map(|p| p.prize_type),
            prize_value: prize.as_ref().map(|p| p.prize_value.clone()),
            prize_image: prize.as_ref().map(|p| p.prize_image.clone()),
            position,
            today_remaining: remaining(activity.daily_limit, updated.today_count),
            total_remaining: remaining(activity.total_limit, updated.total_count),
            record_id,
        })
    }

    /// 我的中奖记录（Java `getUserRecords`）。
    pub async fn my_records(
        &self,
        user_id: Option<i32>,
        q: &LotteryRecordQuery,
    ) -> Result<PageResult<LotteryRecordVO>> {
        let user_id = require_user(user_id)?;
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate_records(user_id, q, limit, offset).await?;
        let list = rows
            .iter()
            .map(|r| LotteryRecordVO {
                record_id: r.record_id,
                activity_name: r.activity_name.clone(),
                prize_name: r.prize_name.clone(),
                prize_type: r.prize_type,
                prize_image: r.prize_image.clone(),
                prize_value: r.prize_value.clone(),
                status: r.status,
                status_text: record_status_text(r.status).to_string(),
                create_time: i64::from(r.create_time),
                receive_time: r.receive_time.map(i64::from),
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 确认领取商品奖品（Java `confirmReceive`）。
    pub async fn confirm_receive(
        &self,
        param: &LotteryConfirmReceiveParam,
        user_id: Option<i32>,
    ) -> Result<()> {
        let user_id = require_user(user_id)?;
        let record_id = param
            .record_id
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        let record = self
            .repo()
            .find_record(record_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        if record.user_id != user_id {
            return Err(Error::biz(500, "无权操作此记录"));
        }
        if record.status != 0 {
            return Err(Error::biz(500, MSG_ALREADY_RECEIVED));
        }
        if record.prize_type != PRIZE_TYPE_PRODUCT {
            return Err(Error::biz(500, MSG_ONLY_PRODUCT_RECEIVE));
        }
        let now = i32::try_from(now_seconds()).unwrap_or_default();
        self.repo().receive_record(record_id, now).await?;
        Ok(())
    }

    /// 中奖名单（Java `getWinnerList`）。
    pub async fn winners(&self, q: &LotteryWinnersQuery) -> Result<Vec<LotteryWinnerVO>> {
        let activity_id = match q.activity_id {
            Some(id) => id,
            None => {
                let now = now_seconds();
                match self.repo().find_ongoing(now).await? {
                    Some(activity) => activity.activity_id,
                    None => return Ok(Vec::new()),
                }
            }
        };
        let rows = self.repo().winners(activity_id).await?;
        Ok(rows
            .iter()
            .map(|r| LotteryWinnerVO {
                nickname: mask_nickname(r.nickname.as_deref().or(Some(r.username.as_str()))),
                avatar: r.avatar.clone(),
                prize_name: r.prize_name.clone(),
                prize_image: r.prize_image.clone(),
                prize_type: r.prize_type,
                win_time: i64::from(r.create_time),
            })
            .collect())
    }
}

/// 活动有效性（Java `validateActivity`）。
pub fn validate_activity(activity: &LotteryActivity, now: i64) -> Result<()> {
    if activity.is_delete == 1 {
        return Err(Error::biz(500, MSG_ACTIVITY_NOT_EXISTS));
    }
    if now < i64::from(activity.start_time) {
        return Err(Error::biz(500, MSG_NOT_STARTED));
    }
    if now > i64::from(activity.end_time) {
        return Err(Error::biz(500, MSG_ENDED));
    }
    Ok(())
}

/// 剩余次数（0 表示不限制 ⇒ -1）。
#[must_use]
pub fn remaining(limit: i32, used: i32) -> i32 {
    if limit > 0 {
        (limit - used).max(0)
    } else {
        -1
    }
}

/// 概率抽奖（Java `executeLotteryAlgorithm`）：返回 `(中奖奖品, 位置)`。
#[must_use]
pub fn pick_winner(
    grid: &[GridPosition],
    prizes: &std::collections::HashMap<i32, LotteryPrize>,
    random: Decimal,
) -> (Option<LotteryPrize>, i32) {
    let no_win_position = grid
        .iter()
        .find(|g| g.prize_id.filter(|id| *id > 0).is_none())
        .and_then(|g| g.position)
        .unwrap_or(1);
    if grid.is_empty() {
        return (None, 1);
    }
    let mut available: Vec<(&GridPosition, &LotteryPrize)> = Vec::new();
    let mut total = Decimal::ZERO;
    for position in grid {
        let Some(prize_id) = position.prize_id.filter(|id| *id > 0) else {
            continue;
        };
        let probability = position.probability.unwrap_or_default();
        if probability <= Decimal::ZERO {
            continue;
        }
        let Some(prize) = prizes.get(&prize_id) else {
            continue;
        };
        if prize.prize_stock > prize.prize_stock_used {
            available.push((position, prize));
            total += probability;
        }
    }
    if available.is_empty() || total <= Decimal::ZERO {
        return (None, no_win_position);
    }
    let mut cumulative = Decimal::ZERO;
    for (position, prize) in available {
        cumulative += position.probability.unwrap_or_default();
        if random < cumulative {
            return (Some(prize.clone()), position.position.unwrap_or(1));
        }
    }
    (None, no_win_position)
}

/// 九宫格 VO（Java `buildGridPositionVO`）。
#[must_use]
pub fn build_grid_vo(activity: &LotteryActivity, prizes: &[LotteryPrize]) -> Vec<GridPositionVO> {
    let by_id: std::collections::HashMap<i32, &LotteryPrize> =
        prizes.iter().map(|p| (p.prize_id, p)).collect();
    parse_grid(activity.grid_config.as_deref())
        .into_iter()
        .map(|grid| {
            let prize = grid
                .prize_id
                .filter(|id| *id > 0)
                .and_then(|id| by_id.get(&id).copied());
            GridPositionVO {
                position: grid.position,
                prize_id: grid.prize_id,
                probability: grid.probability,
                prize_name: Some(match prize {
                    Some(p) => p.prize_name.clone(),
                    None => activity.no_win_name.clone(),
                }),
                prize_image: Some(match prize {
                    Some(p) => p.prize_image.clone(),
                    None => activity.no_win_image.clone(),
                }),
                prize_type: Some(prize.map(|p| p.prize_type).unwrap_or(0)),
                prize_value: prize.map(|p| p.prize_value.clone()),
            }
        })
        .collect()
}

fn require_user(user_id: Option<i32>) -> Result<i32> {
    match user_id {
        Some(id) if id != 0 => Ok(id),
        _ => Err(Error::biz(500, MSG_NOT_LOGGED_IN)),
    }
}

fn today_string(now: i64) -> String {
    chrono::DateTime::from_timestamp(now, 0)
        .map(|utc| {
            (utc.naive_utc() + chrono::Duration::hours(8))
                .format("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default()
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = LotteryRecordQuery {
        page,
        size,
        ..Default::default()
    }
    .limit_offset();
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prize(id: i32, stock: i32, used: i32) -> LotteryPrize {
        LotteryPrize {
            prize_id: id,
            prize_name: format!("奖品{id}"),
            prize_type: 1,
            prize_value: "10".into(),
            prize_image: "img".into(),
            prize_stock: stock,
            prize_stock_used: used,
            ..Default::default()
        }
    }

    fn grid() -> Vec<GridPosition> {
        vec![
            GridPosition {
                position: Some(1),
                prize_id: Some(1),
                probability: Some(Decimal::new(5000, 2)),
            },
            GridPosition {
                position: Some(2),
                prize_id: None,
                probability: Some(Decimal::new(5000, 2)),
            },
        ]
    }

    #[test]
    fn probability_pick_and_stock_filter() {
        let prizes: std::collections::HashMap<i32, LotteryPrize> =
            [(1, prize(1, 10, 0))].into_iter().collect();
        // 0~49.99 命中位置 1；>=50 未中奖
        let (picked, pos) = pick_winner(&grid(), &prizes, Decimal::new(4999, 2));
        assert_eq!(pos, 1);
        assert!(picked.is_some());
        let (picked, pos) = pick_winner(&grid(), &prizes, Decimal::new(5000, 2));
        assert_eq!(pos, 2);
        assert!(picked.is_none());
        // 库存告罄 ⇒ 未中奖
        let sold: std::collections::HashMap<i32, LotteryPrize> =
            [(1, prize(1, 5, 5))].into_iter().collect();
        let (picked, _) = pick_winner(&grid(), &sold, Decimal::new(1, 2));
        assert!(picked.is_none());
    }

    #[test]
    fn remaining_rules() {
        assert_eq!(remaining(0, 10), -1);
        assert_eq!(remaining(5, 2), 3);
        assert_eq!(remaining(5, 9), 0);
    }

    #[test]
    fn grid_vo_fills_no_win_for_empty_position() {
        let activity = LotteryActivity {
            no_win_name: "谢谢参与".into(),
            no_win_image: "no.png".into(),
            grid_config: Some(
                r#"[{"position":1,"prizeId":1,"probability":10},{"position":2,"prizeId":null}]"#
                    .into(),
            ),
            ..Default::default()
        };
        let vo = build_grid_vo(&activity, &[prize(1, 5, 0)]);
        assert_eq!(vo.len(), 2);
        assert_eq!(vo[0].prize_name.as_deref(), Some("奖品1"));
        assert_eq!(vo[1].prize_name.as_deref(), Some("谢谢参与"));
        assert_eq!(vo[1].prize_type, Some(0));
    }
}
