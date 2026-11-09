//! 抽奖活动域服务（marketing::lottery_activity）。
//!
//! 对应原 Java `LotteryActivityServiceImpl`
//! （`java/nextrade-service/.../promotion/impl/LotteryActivityServiceImpl.java`，8 个 admin 端点）。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - **装修配置 / 奖品入口图**：Java `DecorateConfig` / `prizeEntryImage` 是强类型
//!   VO；本实现把 `decorateConfig` 作为**原样 JSON**（`serde_json::Value`）透传，
//!   `prizeEntryImage` 未持久化（`lottery_activity` 无该列，Java 亦未落库，仅
//!   VO 字段，恒为 null）。
//! - **奖品库存统计**：编辑时按 Java `savePrizeList` 语义「保留已使用库存」
//!   （`prize_stock_used` 读旧值回写）。
//! - **活动详情 winCount**：Java `LotteryActivityVO.winCount` 未赋值（恒 null）；
//!   本实现同。
//! - **用户来源标签**：`FromTag` 枚举文案本地映射（1 公众号 / 2 小程序 / 4 PC /
//!   5 Android / 6 IOS / 7 Admin / 8 APP，其它空串）。
//! - **i18n / JsonTranslate 未迁移**：商品名/图片等不翻译（与既有切片一致）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{LotteryActivity, LotteryPrize};
use nextrade_core::models::marketing::lottery_activity::{
    activity_status, format_datetime, parse_datetime, prize_display_name, prize_status_text,
    status_text, GridPositionParam, GridPositionVO, LotteryActivityIdParam, LotteryActivityListQuery,
    LotteryActivityListVO, LotteryActivityParam, LotteryActivityVO, LotteryPrizeItemParam,
    LotteryPrizeVO, LotteryUserManageQuery, LotteryUserManageVO, LotteryUserRecordQuery,
    LotteryUserRecordVO, MSG_ACTIVITY_ALREADY_ENDED, MSG_ACTIVITY_ID_REQUIRED,
    MSG_ACTIVITY_NOT_EXISTS, MSG_ACTIVITY_NOT_STARTED, MSG_ACTIVITY_RUNNING_CANNOT_DELETE,
    MSG_GRID_DUPLICATE, MSG_GRID_NEED_PRIZE, MSG_GRID_SIZE, MSG_PRIZE_PROBABILITY_MAX,
    MSG_PRIZE_PROBABILITY_POSITIVE, MSG_TIME_CONFLICT,
    MSG_TOTAL_PROBABILITY, PROMOTION_TYPE_LOTTERY,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::lottery_activity::{
    LotteryActivityRepository, UserBrief,
};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 存库的九宫格位置（Java `GridPosition`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridPosition {
    pub position: i32,
    pub prize_id: Option<i32>,
    pub probability: Option<Decimal>,
}

/// 抽奖活动域服务。
pub struct LotteryActivityService {
    repos: Arc<Repositories>,
}

impl LotteryActivityService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &LotteryActivityRepository {
        &self.repos.marketing.lottery_activity
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 活动分页（Java `getList`）。
    pub async fn list(
        &self,
        q: &LotteryActivityListQuery,
    ) -> Result<nextrade_core::types::PageResult<LotteryActivityListVO>> {
        let now = now_seconds();
        let (limit, offset) = q.limit_offset();
        let page = to_page_query(q.page, q.size);
        let (total, rows) = self.repo().paginate(q, now, scoped_list_shop_id(), limit, offset).await?;
        let mut list = Vec::with_capacity(rows.len());
        for a in &rows {
            let status = activity_status(now, a.start_time, a.end_time);
            let participant_count = self.repo().participant_count(a.activity_id).await?;
            list.push(LotteryActivityListVO {
                activity_id: a.activity_id,
                activity_name: a.activity_name.clone(),
                status,
                status_text: status_text(status).to_string(),
                participant_count: i32::try_from(participant_count).unwrap_or(i32::MAX),
                create_time: format_datetime(a.create_time),
                start_time: format_datetime(a.start_time),
                end_time: format_datetime(a.end_time),
                can_edit: status == 0,
                can_delete: status != 1,
            });
        }
        Ok(nextrade_core::types::PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 活动详情（Java `getDetail`：含奖品列表 + 九宫格奖品详情）。
    pub async fn detail(&self, activity_id: i32) -> Result<LotteryActivityVO> {
        let a = self.find_activity(activity_id).await?;
        let now = now_seconds();
        let status = activity_status(now, a.start_time, a.end_time);
        let prizes = self.repo().prizes_by_activity(activity_id).await?;
        let prize_map: HashMap<i32, &LotteryPrize> =
            prizes.iter().map(|p| (p.prize_id, p)).collect();
        let grid = self.build_grid_vo(&a, &prize_map);
        let participant_count = self.repo().participant_count(activity_id).await?;
        Ok(LotteryActivityVO {
            activity_id: a.activity_id,
            activity_name: a.activity_name.clone(),
            start_time: format_datetime(a.start_time),
            end_time: format_datetime(a.end_time),
            cost_type: a.cost_type,
            cost_points: a.cost_points,
            daily_limit: a.daily_limit,
            total_limit: a.total_limit,
            win_limit: a.win_limit,
            status,
            status_text: status_text(status).to_string(),
            decorate_config: a
                .decorate_config
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            prize_list: prizes.iter().map(prize_to_vo).collect(),
            grid_config: grid,
            no_win_name: a.no_win_name.clone(),
            no_win_image: a.no_win_image.clone(),
            no_win_tips: a.no_win_tips.clone(),
            prize_entry_image: String::new(),
            participant_count: i32::try_from(participant_count).unwrap_or(i32::MAX),
            win_count: 0,
        })
    }

    // ————————————————————— 创建 / 编辑 / 删除 / 结束 —————————————————————

    /// 创建（Java `create`）。
    pub async fn create(&self, param: &LotteryActivityParam) -> Result<i32> {
        let (start_ts, end_ts) = self.parse_times(param)?;
        param.validate_basic(start_ts, end_ts).map_err(Error::biz_msg)?;
        if self
            .repo()
            .time_conflict_count(None, start_ts, end_ts)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_TIME_CONFLICT));
        }
        let now = now_seconds_i32();
        let mut activity = LotteryActivity {
            activity_id: 0,
            activity_name: param.activity_name.clone().unwrap_or_default(),
            start_time: start_ts,
            end_time: end_ts,
            cost_points: param.cost_points.unwrap_or(0),
            daily_limit: param.daily_limit.unwrap_or(0),
            total_limit: param.total_limit.unwrap_or(0),
            win_limit: param.win_limit.unwrap_or(0),
            decorate_config: param
                .decorate_config
                .as_ref()
                .map(ToString::to_string),
            grid_config: Some("[]".to_string()),
            no_win_name: param.no_win_name.clone().unwrap_or_default(),
            no_win_image: param.no_win_image.clone().unwrap_or_default(),
            no_win_tips: param.no_win_tips.clone().unwrap_or_default(),
            shop_id: TenantScope::current().write_shop_id(None)?,
            is_delete: 0,
            create_time: now,
            update_time: now,
            cost_type: normalize_cost_type(param),
        };
        let activity_id = self.repo().insert(&activity).await?;
        activity.activity_id = activity_id;
        let grid = self.save_prizes_and_grid(&activity, param).await?;
        activity.grid_config = Some(serde_json::to_string(&grid).unwrap_or_else(|_| "[]".into()));
        self.repo().update(&activity).await?;
        self.sync_promotion(&activity).await?;
        tracing::info!("创建抽奖活动: id={activity_id}");
        Ok(activity_id)
    }

    /// 编辑（Java `update`：仅活动基本信息与奖品/九宫格）。
    pub async fn update(&self, param: &LotteryActivityParam) -> Result<()> {
        let activity_id = param
            .activity_id
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_ID_REQUIRED))?;
        let mut activity = self.find_activity(activity_id).await?;
        let (start_ts, end_ts) = self.parse_times(param)?;
        param.validate_basic(start_ts, end_ts).map_err(Error::biz_msg)?;
        if self
            .repo()
            .time_conflict_count(Some(activity_id), start_ts, end_ts)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_TIME_CONFLICT));
        }
        let grid = self.save_prizes_and_grid(&activity, param).await?;
        activity.activity_name = param.activity_name.clone().unwrap_or_default();
        activity.start_time = start_ts;
        activity.end_time = end_ts;
        activity.cost_points = param.cost_points.unwrap_or(0);
        activity.daily_limit = param.daily_limit.unwrap_or(0);
        activity.total_limit = param.total_limit.unwrap_or(0);
        activity.win_limit = param.win_limit.unwrap_or(0);
        activity.decorate_config = param.decorate_config.as_ref().map(ToString::to_string);
        activity.grid_config = Some(serde_json::to_string(&grid).unwrap_or_else(|_| "[]".into()));
        activity.no_win_name = param.no_win_name.clone().unwrap_or_default();
        activity.no_win_image = param.no_win_image.clone().unwrap_or_default();
        activity.no_win_tips = param.no_win_tips.clone().unwrap_or_default();
        activity.cost_type = normalize_cost_type(param);
        activity.update_time = now_seconds_i32();
        self.repo().update(&activity).await?;
        self.sync_promotion(&activity).await?;
        tracing::info!("更新抽奖活动: id={activity_id}");
        Ok(())
    }

    /// 删除（Java `delete`：软删除；进行中拒绝）。
    pub async fn delete(&self, param: &LotteryActivityIdParam) -> Result<()> {
        let activity_id = param
            .activity_id
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_ID_REQUIRED))?;
        let mut activity = self.find_activity(activity_id).await?;
        let status = activity_status(now_seconds(), activity.start_time, activity.end_time);
        if status == 1 {
            return Err(Error::biz(500, MSG_ACTIVITY_RUNNING_CANNOT_DELETE));
        }
        activity.is_delete = 1;
        activity.update_time = now_seconds_i32();
        self.repo().update(&activity).await?;
        self.repo().soft_delete_promotion(activity_id).await?;
        tracing::info!("删除抽奖活动: id={activity_id}");
        Ok(())
    }

    /// 立即结束（Java `endActivity`）。
    pub async fn end(&self, param: &LotteryActivityIdParam) -> Result<()> {
        let activity_id = param
            .activity_id
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_ID_REQUIRED))?;
        let mut activity = self.find_activity(activity_id).await?;
        let now = now_seconds();
        let status = activity_status(now, activity.start_time, activity.end_time);
        if status == 0 {
            return Err(Error::biz(500, MSG_ACTIVITY_NOT_STARTED));
        }
        if status == 2 {
            return Err(Error::biz(500, MSG_ACTIVITY_ALREADY_ENDED));
        }
        // Java `(System.currentTimeMillis() - 1000) / 1000`
        let end_time = i32::try_from(now - 1).unwrap_or(0);
        activity.end_time = end_time;
        activity.update_time = end_time;
        self.repo().update(&activity).await?;
        self.repo()
            .update_promotion_end(activity_id, end_time)
            .await?;
        tracing::info!("结束抽奖活动: id={activity_id}");
        Ok(())
    }

    // ————————————————————— 用户管理 / 中奖记录 —————————————————————

    /// 用户管理列表（Java `getUserManageList`）。
    pub async fn user_manage(
        &self,
        q: &LotteryUserManageQuery,
    ) -> Result<nextrade_core::types::PageResult<LotteryUserManageVO>> {
        let activity_id = q
            .activity_id
            .ok_or_else(|| Error::biz(500, "活动ID不能为空"))?;
        let start_ts = q
            .start_time
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(parse_datetime)
            .transpose()
            .map_err(Error::Validation)?;
        let end_ts = q
            .end_time
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(parse_datetime)
            .transpose()
            .map_err(Error::Validation)?;
        let (limit, offset) = q.limit_offset();
        let page = to_page_query(q.page, q.size);
        let (total, stats) = self
            .repo()
            .user_stats_paginate(q, start_ts, end_ts, limit, offset)
            .await?;
        let user_ids: Vec<i32> = stats.iter().map(|s| s.user_id).collect();
        let briefs: HashMap<i32, UserBrief> = self
            .repo()
            .user_briefs(&user_ids)
            .await?
            .into_iter()
            .collect();
        let mut list: Vec<LotteryUserManageVO> = stats
            .iter()
            .map(|s| {
                let user = briefs.get(&s.user_id);
                LotteryUserManageVO {
                    user_id: s.user_id,
                    user_name: user
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "未知用户".to_string()),
                    avatar: user.and_then(|u| u.avatar.clone()),
                    from_tag_name: user
                        .map(|u| from_tag_name(u.from_tag))
                        .unwrap_or_default(),
                    total_count: s.total_count,
                    last_lottery_time: s.last_lottery_time.map(format_datetime),
                }
            })
            .collect();
        // Java：userName 过滤发生在分页之后（records 过滤 + total 覆盖）
        let mut effective_total = u64::try_from(total).unwrap_or(0);
        if let Some(name) = q.user_name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            list.retain(|v| v.user_name.contains(name));
            effective_total = u64::try_from(list.len()).unwrap_or(0);
        }
        Ok(nextrade_core::types::PageResult::new(
            effective_total,
            list,
            &page,
        ))
    }

    /// 用户中奖记录（Java `getUserRecordList`）。
    pub async fn user_record(
        &self,
        q: &LotteryUserRecordQuery,
    ) -> Result<nextrade_core::types::PageResult<LotteryUserRecordVO>> {
        let activity_id = q
            .activity_id
            .ok_or_else(|| Error::biz(500, "活动ID不能为空"))?;
        let user_id = q.user_id.ok_or_else(|| Error::biz(500, "用户ID不能为空"))?;
        let (limit, offset) = q.limit_offset();
        let page = to_page_query(q.page, q.size);
        let (total, records) = self.repo().records_paginate(q, limit, offset).await?;
        let user = self.repo().user_brief(user_id).await?;
        let list: Vec<LotteryUserRecordVO> = records
            .iter()
            .map(|r| LotteryUserRecordVO {
                record_id: r.record_id,
                user_name: user
                    .as_ref()
                    .map(|u| u.username.clone())
                    .unwrap_or_else(|| "未知用户".to_string()),
                avatar: user.as_ref().and_then(|u| u.avatar.clone()),
                from_tag_name: user
                    .as_ref()
                    .map(|u| from_tag_name(u.from_tag))
                    .unwrap_or_default(),
                create_time: if r.create_time != 0 {
                    Some(format_datetime(r.create_time))
                } else {
                    None
                },
                prize_name: prize_display_name(
                    r.is_win,
                    Some(r.prize_type),
                    &r.prize_value,
                    &r.prize_name,
                ),
                status_text: prize_status_text(r.is_win, Some(r.prize_type), r.status),
            })
            .collect();
        Ok(nextrade_core::types::PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 内部 —————————————————————

    async fn find_activity(&self, activity_id: i32) -> Result<LotteryActivity> {
        let a = self
            .repo()
            .find_by_id(activity_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ACTIVITY_NOT_EXISTS))?;
        if a.is_delete == 1 {
            return Err(Error::biz(500, MSG_ACTIVITY_NOT_EXISTS));
        }
        // 多租户：店铺账号只能操作/查看本店活动
        TenantScope::current_ensure_access(a.shop_id)?;
        Ok(a)
    }

    fn parse_times(&self, param: &LotteryActivityParam) -> Result<(i32, i32)> {
        let start = parse_datetime(param.start_time.as_deref().unwrap_or(""))
            .map_err(Error::Validation)?;
        let end =
            parse_datetime(param.end_time.as_deref().unwrap_or("")).map_err(Error::Validation)?;
        Ok((start, end))
    }

    /// 保存奖品列表（保留已用库存）+ 转换九宫格（tempIndex → prizeId）。
    async fn save_prizes_and_grid(
        &self,
        activity: &LotteryActivity,
        param: &LotteryActivityParam,
    ) -> Result<Vec<GridPosition>> {
        let items = param.prize_list.as_deref().unwrap_or_default();
        // 1) 保留 prizeId 的已有奖品（更新），其余删除
        let mut kept_ids: Vec<i32> = Vec::new();
        let mut temp_map: HashMap<i32, i32> = HashMap::new();
        let now = now_seconds_i32();
        for item in items {
            if let Some(prize_id) = item.prize_id {
                if let Some(mut old) = self.repo().prize_by_id(prize_id).await? {
                    old.prize_name = item.prize_name.clone().unwrap_or_default();
                    old.prize_type = item.prize_type.unwrap_or(0);
                    old.prize_value = item.prize_value.clone().unwrap_or_default();
                    old.prize_image = item.prize_image.clone().unwrap_or_default();
                    old.prize_stock = item.prize_stock.unwrap_or(0);
                    old.update_time = now;
                    self.repo().update_prize(&old).await?;
                    kept_ids.push(prize_id);
                    if let Some(temp) = item.temp_index {
                        temp_map.insert(temp, prize_id);
                    }
                }
            }
        }
        // 新增奖品：tempIndex → 新 prizeId
        for item in items.iter().filter(|i| i.prize_id.is_none()) {
            let prize = LotteryPrize {
                prize_id: 0,
                activity_id: activity.activity_id,
                prize_name: item.prize_name.clone().unwrap_or_default(),
                prize_type: item.prize_type.unwrap_or(0),
                prize_value: item.prize_value.clone().unwrap_or_default(),
                prize_image: item.prize_image.clone().unwrap_or_default(),
                prize_stock: item.prize_stock.unwrap_or(0),
                prize_stock_used: 0,
                create_time: now,
                update_time: now,
            };
            let prize_id = self.repo().insert_prize(&prize).await?;
            kept_ids.push(prize_id);
            if let Some(temp) = item.temp_index {
                temp_map.insert(temp, prize_id);
            }
        }
        self.repo()
            .delete_prizes_except(activity.activity_id, &kept_ids)
            .await?;
        // 2) 九宫格：tempIndex → prizeId
        let grid_params = param.grid_config.as_deref().unwrap_or_default();
        if grid_params.len() != 8 {
            return Err(Error::biz(500, MSG_GRID_SIZE));
        }
        let mut positions: Vec<i32> = grid_params.iter().map(|g| g.position.unwrap_or(-1)).collect();
        positions.sort_unstable();
        positions.dedup();
        if positions.len() != 8 {
            return Err(Error::biz(500, MSG_GRID_DUPLICATE));
        }
        let mut grid: Vec<GridPosition> = Vec::with_capacity(8);
        let mut total = Decimal::ZERO;
        let hundred = Decimal::from(100);
        let mut prize_count = 0;
        for g in grid_params {
            let prize_id = match g.temp_index {
                Some(temp) => Some(*temp_map.get(&temp).ok_or_else(|| {
                    Error::biz(
                        500,
                        format!(
                            "位置{}关联的奖品不存在，tempIndex:{temp}",
                            g.position.unwrap_or(0)
                        ),
                    )
                })?),
                None => g.prize_id,
            };
            if let Some(id) = prize_id {
                prize_count += 1;
                if self.repo().prize_by_id(id).await?.is_none() {
                    return Err(Error::biz(
                        500,
                        format!("位置{}配置的奖品不存在", g.position.unwrap_or(0)),
                    ));
                }
                let probability = g.probability.unwrap_or_default();
                if probability <= Decimal::ZERO {
                    return Err(Error::biz(500, MSG_PRIZE_PROBABILITY_POSITIVE));
                }
                if probability > hundred {
                    return Err(Error::biz(500, MSG_PRIZE_PROBABILITY_MAX));
                }
                total += probability;
            }
            grid.push(GridPosition {
                position: g.position.unwrap_or(0),
                prize_id,
                probability: g.probability,
            });
        }
        if prize_count == 0 {
            return Err(Error::biz(500, MSG_GRID_NEED_PRIZE));
        }
        if total >= hundred {
            return Err(Error::biz(500, MSG_TOTAL_PROBABILITY));
        }
        Ok(grid)
    }

    /// 九宫格 → VO（Java `buildGridPositionVO`：奖品缺失时回落未中奖配置）。
    fn build_grid_vo(
        &self,
        activity: &LotteryActivity,
        prize_map: &HashMap<i32, &LotteryPrize>,
    ) -> Vec<GridPositionVO> {
        let Some(raw) = activity.grid_config.as_deref() else {
            return Vec::new();
        };
        let Ok(grid) = serde_json::from_str::<Vec<GridPosition>>(raw) else {
            return Vec::new();
        };
        grid.into_iter()
            .map(|g| {
                let prize = g.prize_id.and_then(|id| prize_map.get(&id).copied());
                match prize {
                    Some(p) => GridPositionVO {
                        position: g.position,
                        prize_id: g.prize_id,
                        probability: g.probability,
                        prize_name: Some(p.prize_name.clone()),
                        prize_image: Some(p.prize_image.clone()),
                        prize_type: Some(p.prize_type),
                        prize_value: Some(p.prize_value.clone()),
                    },
                    None => GridPositionVO {
                        position: g.position,
                        prize_id: g.prize_id,
                        probability: g.probability,
                        prize_name: Some(activity.no_win_name.clone()),
                        prize_image: Some(activity.no_win_image.clone()),
                        prize_type: Some(0),
                        prize_value: None,
                    },
                }
            })
            .collect()
    }

    async fn sync_promotion(&self, activity: &LotteryActivity) -> Result<()> {
        let status = activity_status(now_seconds(), activity.start_time, activity.end_time);
        let is_available = if status == 1 { 1 } else { 0 };
        self.repo()
            .upsert_promotion(activity, is_available)
            .await?;
        let _ = PROMOTION_TYPE_LOTTERY;
        Ok(())
    }
}

fn normalize_cost_type(param: &LotteryActivityParam) -> i32 {
    let cost_type = param.cost_type.unwrap_or(1);
    if cost_type == 0 {
        0
    } else {
        cost_type
    }
}

fn prize_to_vo(p: &LotteryPrize) -> LotteryPrizeVO {
    LotteryPrizeVO {
        prize_id: p.prize_id,
        activity_id: p.activity_id,
        prize_name: p.prize_name.clone(),
        prize_type: p.prize_type,
        prize_type_text: match p.prize_type {
            1 => "积分",
            2 => "商品",
            3 => "优惠券",
            _ => "",
        }
        .to_string(),
        prize_value: p.prize_value.clone(),
        prize_image: p.prize_image.clone(),
        prize_stock: p.prize_stock,
        prize_stock_used: p.prize_stock_used,
        remain_stock: (p.prize_stock - p.prize_stock_used).max(0),
        create_time: format_datetime(p.create_time),
    }
}

/// 来源标签文案（Java `FromTag`）。
#[must_use]
pub fn from_tag_name(from_tag: Option<i32>) -> String {
    match from_tag {
        Some(1) => "公众号",
        Some(2) => "小程序",
        Some(4) => "PC",
        Some(5) => "Android",
        Some(6) => "IOS",
        Some(7) => "Admin",
        Some(8) => "APP",
        _ => "",
    }
    .to_string()
}

fn now_seconds_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(0)
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> PageQuery {
    PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

/// 列表店铺过滤：店铺账号强制本店；平台账号不过滤。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_tag_names_match_java_enum() {
        assert_eq!(from_tag_name(Some(1)), "公众号");
        assert_eq!(from_tag_name(Some(2)), "小程序");
        assert_eq!(from_tag_name(Some(4)), "PC");
        assert_eq!(from_tag_name(Some(8)), "APP");
        assert_eq!(from_tag_name(Some(99)), "");
        assert_eq!(from_tag_name(None), "");
    }

    #[test]
    fn prize_vo_remain_stock_never_negative() {
        let p = LotteryPrize {
            prize_id: 1,
            activity_id: 2,
            prize_name: "积分".into(),
            prize_type: 1,
            prize_value: "10".into(),
            prize_image: "i.png".into(),
            prize_stock: 5,
            prize_stock_used: 8,
            create_time: 1_700_000_000,
            update_time: 0,
        };
        let vo = prize_to_vo(&p);
        assert_eq!(vo.remain_stock, 0);
        assert_eq!(vo.prize_type_text, "积分");
        assert_eq!(vo.create_time, "2023-11-15 06:13:20");
    }

    #[test]
    fn cost_type_normalization_matches_java() {
        let mut p = LotteryActivityParam::default();
        assert_eq!(normalize_cost_type(&p), 1);
        p.cost_type = Some(0);
        assert_eq!(normalize_cost_type(&p), 0);
        p.cost_type = Some(1);
        assert_eq!(normalize_cost_type(&p), 1);
    }

    #[test]
    fn grid_position_json_roundtrip() {
        let grid = vec![GridPosition {
            position: 0,
            prize_id: Some(9),
            probability: Some(Decimal::new(1250, 2)),
        }];
        let raw = serde_json::to_string(&grid).unwrap();
        assert!(raw.contains("\"prizeId\":9"));
        let back: Vec<GridPosition> = serde_json::from_str(&raw).unwrap();
        assert_eq!(back[0].position, 0);
        assert_eq!(back[0].probability, Some(Decimal::new(1250, 2)));
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_activity() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async { scoped_list_shop_id() }).await;
        assert_eq!(filter, None);
    }
}
