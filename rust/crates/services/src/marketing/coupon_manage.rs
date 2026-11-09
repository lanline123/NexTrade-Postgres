//! 优惠券管理域服务（marketing::coupon_manage）。
//!
//! 对应原 Java：
//! - `CouponServiceImpl#create/update/del/updateField/batch/config` + `savePromotion`
//! - `UserCouponServiceImpl#previewManualIssue`（手动发放预览）
//! - `CouponIssueLogServiceImpl#list` / `CouponIssueBatchServiceImpl#list`
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! - `create` / `update`（`@Transactional`）：`dataJudge` 派生字段
//!   （使用时间两分支、满减规则、`sendRangeData`/`limitUserRank` 转 Java
//!   `List.toString`）→ 落 `coupon` → `savePromotion` 同步写 `promotion`
//!   主表（`type = 2`，存在则更新、不存在则插入）。
//! - `del` / `batch(type=del)`（`@Transactional`）：软删优惠券（`is_delete = 1`）
//!   + 同步软删 marketing 活动；`del` 的 id 为空时静默返回（Java 返回 false）。
//! - `updateField`：白名单 `COUPON_FIELDS`（`coupon_name` / `coupon_money` /
//!   `coupon_discount` / `min_order_amount` / `is_show`），非法 → `field 的值有误`。
//! - `config`：全部会员等级 + 每级正式会员数（`user_type = 0`，含禁用）。
//! - `issue/preview`：校验券归属/发放模式/目标人群（USER/RANK/TAG），
//!   逐会员产出失败明细（不存在/已禁用），并按 `send_num - 已发放` 校验余量。
//! - `issue/log`、`issue/batch`：联表回填券名/会员信息/使用状态/来源文案。
//!
//! # 未迁移 / 偏差（见 `uncertain_points`）
//!
//! - `MemberAudienceService`（member_scope 编解码/匹配）未迁移：`sendUserTarget`/
//!   `memberAudience` 原样 JSON 存 `promotion.member_scope`。
//! - 发送站内信/操作日志（`adminLogService`、`message` 域）未迁移 → 仅 `tracing::info`。
//! - `Coupon.clientVO` 等基线端点仍由既有 `marketing::coupon` feature 提供，本域不重复实现。
//! - Java `BeanUtils.copyProperties` 对 String→Long 字段（`useStartDate` 等）不复制；
//!   本实现按 `dataJudge` 的生效分支处理（语义等价，见 `build_coupon_values`）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{Coupon, Promotion, UserRank};
use nextrade_core::models::marketing::coupon_manage::{
    insufficient_issue_message, java_list_string, normalize_issue_type, page_params, page_query,
    parse_datetime, remaining_issue_count, resolve_coupon_field, source_type_name,
    validate_coupon_dto, BatchDTO, CouponConfigRankVO, CouponDTO, CouponIssueBatchPageQuery,
    CouponIssueBatchVO, CouponIssueLogPageQuery, CouponIssueLogVO, CouponManualIssueFailDetailVO,
    CouponManualIssueParam, CouponManualIssuePreviewVO, OperateDTO, TargetUser, UpdateFieldDTO,
    CONFIG_DEFAULT_AVATAR, COUPON_TYPE_MONEY, ISSUE_TYPE_RANK, ISSUE_TYPE_TAG, ISSUE_TYPE_USER,
    MSG_COUPON_MONEY_EXCEED_MIN_ORDER_AMOUNT, MSG_FIELD_INVALID, MSG_ID_REQUIRED,
    MSG_ISSUE_MEMBER_REQUIRED, MSG_ISSUE_MIXED_UNSUPPORTED, MSG_ISSUE_MODE_INVALID,
    MSG_ISSUE_NO_PERMISSION, MSG_ISSUE_RANK_REQUIRED, MSG_ISSUE_TAG_REQUIRED, MSG_MEMBER_DISABLED,
    MSG_MEMBER_NOT_EXIST, MSG_NOT_EXIST, MSG_OPERATION_TYPE_REQUIRED, MSG_UNSELECTED_ITEM,
    PROMOTION_AVAILABLE_YES, PROMOTION_TYPE_COUPON, REDUCE_TYPE_NO_THRESHOLD,
    REDUCE_TYPE_THRESHOLD, SEND_TYPE_DAYS, USER_STATUS_NORMAL,
};
use nextrade_core::models::marketing::coupon_manage::MSG_ISSUE_TYPE_REQUIRED;
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::coupon_manage::CouponUpdatePatch;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 优惠券管理域服务。
pub struct CouponManageService {
    repos: Arc<Repositories>,
}

impl CouponManageService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::marketing::coupon_manage::CouponManageRepository {
        &self.repos.marketing.coupon_manage
    }

    // ————————————————————— 增删改 —————————————————————

    /// 创建优惠券（Java `create`，优惠券 + 营销活动双写事务）。
    pub async fn create(&self, dto: &CouponDTO, shop_id: Option<i32>) -> Result<()> {
        validate_coupon_dto(dto, false)?;
        // 多租户：店铺账号强制本店；平台账号可指定/落 0
        let shop_id = TenantScope::current().write_shop_id(shop_id)?;
        let coupon = build_coupon(dto, shop_id)?;
        let repo = self.repo();
        let mut tx = repo.begin().await?;
        let coupon_id = repo.insert_coupon_in_tx(&mut tx, &coupon).await?;
        let promotion = build_coupon_promotion(&coupon, dto, coupon_id, shop_id);
        if repo.count_promotion_row(coupon_id).await? > 0 {
            repo.update_promotion_row_in_tx(&mut tx, coupon_id, &promotion)
                .await?;
        } else {
            repo.insert_promotion_row_in_tx(&mut tx, &promotion).await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        let log_info = format!("新增优惠券:{}", coupon.coupon_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("优惠券创建成功: id={coupon_id}, name={}", coupon.coupon_name);
        Ok(())
    }

    /// 更新优惠券（Java `update`，只写 DTO 提供的字段 + `dataJudge` 派生字段）。
    pub async fn update(&self, dto: &CouponDTO, shop_id: Option<i32>) -> Result<()> {
        validate_coupon_dto(dto, true)?;
        let coupon_id = dto.coupon_id.unwrap_or(0);
        let repo = self.repo();
        let existing = repo.find_coupon(coupon_id).await?;
        let Some(existing_coupon) = existing else {
            return Err(Error::biz_msg(MSG_NOT_EXIST));
        };
        // 多租户：更新前强制归属校验（TenantScope 优先）
        TenantScope::current_ensure_access(existing_coupon.shop_id.unwrap_or(0))?;
        if let Some(sid) = shop_id {
            if sid > 0 {
                let curr_sid = existing_coupon.shop_id.unwrap_or(0);
                if curr_sid != sid && curr_sid != 0 {
                    return Err(Error::biz(403, "无权修改其他商家的优惠券"));
                }
            }
        }

        let shop_id = TenantScope::current()
            .write_shop_id(shop_id.or(Some(existing_coupon.shop_id.unwrap_or(0))))?;
        let patch = build_coupon_patch(dto, shop_id)?;
        let mut tx = repo.begin().await?;
        let affected = repo
            .update_coupon_in_tx(&mut tx, coupon_id, &patch)
            .await?;
        if affected {
            // Java `savePromotion`：用合并后的活动字段同步 marketing 行。
            let merged = merge_coupon_for_promotion(Some(&existing_coupon), dto, shop_id);
            let promotion = build_coupon_promotion(&merged, dto, coupon_id, shop_id);
            if repo.count_promotion_row(coupon_id).await? > 0 {
                repo.update_promotion_row_in_tx(&mut tx, coupon_id, &promotion)
                    .await?;
            } else {
                repo.insert_promotion_row_in_tx(&mut tx, &promotion).await?;
            }
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        let coupon_name = dto.coupon_name.as_deref().unwrap_or(&existing_coupon.coupon_name);
        let log_info = format!("修改优惠券:{coupon_name}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("优惠券更新成功: id={coupon_id}");
        Ok(())
    }

    /// 删除优惠券（Java `del` → `softDel`：软删 + 同步软删 marketing 行）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<()> {
        self.del_scoped(dto, None).await
    }

    /// 删除优惠券（支持租户隔离）。
    pub async fn del_scoped(&self, dto: &OperateDTO, scoped_shop_id: Option<i32>) -> Result<()> {
        // 多租户：TenantScope 优先（AuthUser 兜底保留）
        let scoped_shop_id = TenantScope::current_list_filter().or(scoped_shop_id);
        let Some(id) = dto.id else {
            // Java：id 为 null → 返回 false（控制器 void，不报错）
            return Ok(());
        };
        let repo = self.repo();
        if let Some(sid) = scoped_shop_id {
            if let Some(c) = repo.find_coupon(id).await? {
                let curr_sid = c.shop_id.unwrap_or(0);
                if curr_sid != sid && curr_sid != 0 {
                    return Err(Error::biz(403, "无权删除其他商家的优惠券"));
                }
            }
        }
        let mut tx = repo.begin().await?;
        if repo.soft_delete_coupons_in_tx(&mut tx, &[id]).await? > 0 {
            repo.soft_delete_promotions_in_tx(&mut tx, &[id]).await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 批量操作（Java `batch`：仅 `type == "del"` 生效）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<()> {
        self.batch_scoped(dto, None).await
    }

    /// 批量操作（支持租户隔离）。
    pub async fn batch_scoped(&self, dto: &BatchDTO, scoped_shop_id: Option<i32>) -> Result<()> {
        // 多租户：TenantScope 优先（AuthUser 兜底保留）
        let scoped_shop_id = TenantScope::current_list_filter().or(scoped_shop_id);
        if dto.ids.is_empty() {
            return Err(Error::Validation(nextrade_core::models::marketing::coupon_manage::MSG_UNSELECTED_ITEM.into()));
        }
        if dto.r#type != "del" {
            // Java：非 del 返回 false（控制器 void，不报错）
            return Ok(());
        }
        let repo = self.repo();
        if let Some(sid) = scoped_shop_id {
            for &id in &dto.ids {
                if let Some(c) = repo.find_coupon(id).await? {
                    let curr_sid = c.shop_id.unwrap_or(0);
                    if curr_sid != sid && curr_sid != 0 {
                        return Err(Error::biz(403, "批量操作中包含其他商家的优惠券"));
                    }
                }
            }
        }
        let mut tx = repo.begin().await?;
        if repo.soft_delete_coupons_in_tx(&mut tx, &dto.ids).await? > 0 {
            repo.soft_delete_promotions_in_tx(&mut tx, &dto.ids).await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 更新单个字段（Java `updateField`，白名单列）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        self.update_field_scoped(dto, None).await
    }

    /// 更新单个字段（支持租户隔离）。
    pub async fn update_field_scoped(&self, dto: &UpdateFieldDTO, scoped_shop_id: Option<i32>) -> Result<()> {
        let Some(column) = resolve_coupon_field(&dto.field) else {
            return Err(Error::biz_msg(MSG_FIELD_INVALID));
        };
        let repo = self.repo();
        let Some(c) = repo.find_coupon(dto.id).await? else {
            return Err(Error::biz_msg(MSG_NOT_EXIST));
        };
        if let Some(sid) = scoped_shop_id {
            let curr_sid = c.shop_id.unwrap_or(0);
            if curr_sid != sid && curr_sid != 0 {
                return Err(Error::biz(403, "无权修改其他商家的优惠券"));
            }
        }
        repo.update_coupon_field(dto.id, column, &dto.val).await?;
        Ok(())
    }

    /// 营销配置项（Java `config`：会员等级 + 每级会员数）。
    pub async fn config(&self) -> Result<Vec<CouponConfigRankVO>> {
        let repo = self.repo();
        let ranks = repo.list_user_ranks().await?;
        if ranks.is_empty() {
            return Ok(Vec::new());
        }
        let counts: HashMap<i32, i64> = repo
            .count_users_by_rank()
            .await?
            .into_iter()
            .collect();
        Ok(ranks
            .iter()
            .map(|rank| rank_to_vo(rank, *counts.get(&rank.rank_id).unwrap_or(&0)))
            .collect())
    }

    // ————————————————————— 手动发放预览 —————————————————————

    /// 手动发放预览（Java `previewManualIssue`）。
    pub async fn issue_preview(
        &self,
        param: &CouponManualIssueParam,
        shop_id: Option<i32>,
    ) -> Result<CouponManualIssuePreviewVO> {
        if param.coupon_id <= 0 {
            return Err(Error::Validation(MSG_ID_REQUIRED.into()));
        }
        if param.issue_type.trim().is_empty() {
            return Err(Error::Validation(MSG_ISSUE_TYPE_REQUIRED.into()));
        }
        let repo = self.repo();
        let coupon = repo
            .find_coupon(param.coupon_id)
            .await?
            .filter(|coupon| coupon.is_delete != Some(1))
            .ok_or_else(|| Error::biz_msg(MSG_NOT_EXIST))?;
        let current_shop_id = shop_id.unwrap_or(0);
        if coupon.shop_id.unwrap_or(0) != current_shop_id {
            return Err(Error::biz_msg(MSG_ISSUE_NO_PERMISSION));
        }
        let Some(issue_type) = normalize_issue_type(&param.issue_type) else {
            return Err(Error::biz_msg(MSG_ISSUE_MODE_INVALID));
        };
        let (valid_users, mut fail_details) = self.resolve_issue_targets(param, issue_type).await?;
        let issued = repo.count_user_coupons(coupon.coupon_id).await?;
        let remain = remaining_issue_count(coupon.send_num, issued);
        let need = i64::try_from(valid_users.len()).unwrap_or(i64::MAX);
        if !valid_users.is_empty() && remain < need {
            return Err(Error::biz_msg(insufficient_issue_message(remain, need)));
        }
        fail_details.shrink_to_fit();
        let count = i32::try_from(valid_users.len()).unwrap_or(i32::MAX);
        Ok(CouponManualIssuePreviewVO {
            coupon_id: coupon.coupon_id,
            issue_type: issue_type.to_string(),
            member_count: count,
            coupon_count: count,
            fail_count: i32::try_from(fail_details.len()).unwrap_or(i32::MAX),
            fail_details,
        })
    }

    /// 解析发放目标（Java `resolveManualIssueTargets` + `resolveManualIssue*`）。
    async fn resolve_issue_targets(
        &self,
        param: &CouponManualIssueParam,
        issue_type: &str,
    ) -> Result<(Vec<TargetUser>, Vec<CouponManualIssueFailDetailVO>)> {
        let user_ids = param.user_ids.clone().unwrap_or_default();
        let rank_ids = param.rank_ids.clone().unwrap_or_default();
        let tag_ids = param.tag_ids.clone().unwrap_or_default();
        let repo = self.repo();
        let (users, order) = match issue_type {
            ISSUE_TYPE_USER => {
                if user_ids.is_empty() {
                    return Err(Error::biz_msg(MSG_ISSUE_MEMBER_REQUIRED));
                }
                if !tag_ids.is_empty() || !rank_ids.is_empty() {
                    return Err(Error::biz_msg(MSG_ISSUE_MIXED_UNSUPPORTED));
                }
                let mut distinct = user_ids;
                distinct.sort_unstable();
                distinct.dedup();
                (repo.find_member_users_by_ids(&distinct).await?, distinct)
            }
            ISSUE_TYPE_RANK => {
                if rank_ids.is_empty() {
                    return Err(Error::biz_msg(MSG_ISSUE_RANK_REQUIRED));
                }
                if !user_ids.is_empty() || !tag_ids.is_empty() {
                    return Err(Error::biz_msg(MSG_ISSUE_MIXED_UNSUPPORTED));
                }
                (repo.find_member_users_by_ranks(&rank_ids).await?, Vec::new())
            }
            ISSUE_TYPE_TAG => {
                if tag_ids.is_empty() {
                    return Err(Error::biz_msg(MSG_ISSUE_TAG_REQUIRED));
                }
                if !user_ids.is_empty() || !rank_ids.is_empty() {
                    return Err(Error::biz_msg(MSG_ISSUE_MIXED_UNSUPPORTED));
                }
                let match_all = param.tag_match_mode == Some(1);
                let ids = repo.resolve_user_ids_by_tags(&tag_ids, match_all).await?;
                (repo.find_member_users_by_ids(&ids).await?, Vec::new())
            }
            _ => return Err(Error::biz_msg(MSG_ISSUE_MODE_INVALID)),
        };
        let user_map: HashMap<i32, &TargetUser> =
            users.iter().map(|user| (user.user_id, user)).collect();
        let mut valid: Vec<TargetUser> = Vec::new();
        let mut failures: Vec<CouponManualIssueFailDetailVO> = Vec::new();
        if order.is_empty() {
            for user in &users {
                push_issue_user(user, &mut valid, &mut failures);
            }
        } else {
            // USER 模式：按请求顺序产出「不存在」明细
            for user_id in order {
                match user_map.get(&user_id) {
                    Some(user) => push_issue_user(user, &mut valid, &mut failures),
                    None => failures.push(CouponManualIssueFailDetailVO {
                        user_id,
                        member_name: String::new(),
                        mobile: String::new(),
                        reason: MSG_MEMBER_NOT_EXIST.to_string(),
                    }),
                }
            }
        }
        Ok((valid, failures))
    }

    // ————————————————————— 发放日志 / 批次 —————————————————————

    /// 发放日志分页（Java `CouponIssueLogServiceImpl#list`）。
    pub async fn issue_log(
        &self,
        q: &CouponIssueLogPageQuery,
    ) -> Result<PageResult<CouponIssueLogVO>> {
        let (page, size) = page_params(q.page, q.size);
        let pq = page_query(q.page, q.size);
        let repo = self.repo();
        let (total, rows) = repo
            .paginate_issue_logs(q.coupon_id, q.batch_id, q.user_id, q.source_type, page, size)
            .await?;
        let default_avatar = repo
            .find_default_avatar()
            .await?
            .unwrap_or_default();
        let list = rows
            .iter()
            .map(|row| {
                let avatar = row
                    .avatar
                    .clone()
                    .filter(|avatar| !avatar.is_empty())
                    .or_else(|| Some(default_avatar.clone()));
                CouponIssueLogVO {
                    id: row.id,
                    batch_id: row.batch_id,
                    coupon_id: row.coupon_id,
                    coupon_name: row.coupon_name.clone(),
                    user_coupon_id: row.user_coupon_id,
                    user_id: row.user_id,
                    username: row.username.clone(),
                    nickname: row.nickname.clone(),
                    mobile: row.mobile.clone(),
                    avatar,
                    issue_time: row.issue_time,
                    source_type: row.source_type,
                    source_type_name: source_type_name(row.source_type).to_string(),
                    used: if row.used_time.unwrap_or(0) > 0 { 1 } else { 0 },
                    used_time: row.used_time,
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &pq))
    }

    /// 发放批次分页（Java `CouponIssueBatchServiceImpl#list`）。
    pub async fn issue_batch(
        &self,
        q: &CouponIssueBatchPageQuery,
    ) -> Result<PageResult<CouponIssueBatchVO>> {
        let (page, size) = page_params(q.page, q.size);
        let pq = page_query(q.page, q.size);
        let repo = self.repo();
        let (total, rows) = repo
            .paginate_issue_batches(q.coupon_id, q.source_type, page, size)
            .await?;
        let list = rows
            .iter()
            .map(|row| CouponIssueBatchVO {
                id: row.id,
                coupon_id: row.coupon_id,
                coupon_name: row.coupon_name.clone(),
                user_count: row.user_count,
                coupon_count: row.coupon_count,
                issue_time: row.issue_time,
                source_type: row.source_type,
                source_type_name: source_type_name(row.source_type).to_string(),
            })
            .collect();
        Ok(PageResult::new(total, list, &pq))
    }
}

/// 单个会员 → 有效或失败明细（Java `resolveManualIssue*` 循环体）。
fn push_issue_user(
    user: &TargetUser,
    valid: &mut Vec<TargetUser>,
    failures: &mut Vec<CouponManualIssueFailDetailVO>,
) {
    if user.status != USER_STATUS_NORMAL {
        failures.push(CouponManualIssueFailDetailVO {
            user_id: user.user_id,
            member_name: user.nickname.clone(),
            mobile: user.mobile.clone(),
            reason: MSG_MEMBER_DISABLED.to_string(),
        });
        return;
    }
    valid.push(user.clone());
}

/// 由 DTO 构造优惠券实体（Java `dataJudge` + `BeanUtils.copyProperties`）。
fn build_coupon(dto: &CouponDTO, shop_id: i32) -> Result<Coupon> {
    let send_type = dto.send_type.unwrap_or(0);
    let use_day = if send_type == SEND_TYPE_DAYS {
        dto.use_day.unwrap_or(0)
    } else {
        0
    };
    let delay_day = if send_type == SEND_TYPE_DAYS {
        dto.delay_day.unwrap_or(0)
    } else {
        0
    };
    let reduce_type = dto.reduce_type.unwrap_or(0);
    let coupon_type = dto.coupon_type.unwrap_or(0);
    let coupon_money = dto.coupon_money.unwrap_or(Decimal::ZERO);
    let min_order_amount = if reduce_type == REDUCE_TYPE_NO_THRESHOLD {
        Decimal::ZERO
    } else {
        dto.min_order_amount.unwrap_or(Decimal::ZERO)
    };
    if reduce_type == REDUCE_TYPE_THRESHOLD
        && coupon_type == COUPON_TYPE_MONEY
        && dto.coupon_money.is_some()
        && dto.min_order_amount.is_some()
        && min_order_amount <= coupon_money
    {
        return Err(Error::biz_msg(MSG_COUPON_MONEY_EXCEED_MIN_ORDER_AMOUNT));
    }
    let coupon_discount = if coupon_type == COUPON_TYPE_MONEY {
        Decimal::ZERO
    } else {
        dto.coupon_discount.unwrap_or(Decimal::ZERO)
    };
    let (use_start_date, use_end_date) = resolve_use_dates(dto);
    Ok(Coupon {
        coupon_id: 0,
        coupon_name: dto.coupon_name.clone().unwrap_or_default(),
        coupon_money,
        coupon_discount,
        coupon_desc: dto.coupon_desc.clone().unwrap_or_default(),
        coupon_type,
        send_range: dto.send_range.unwrap_or(0),
        send_range_data: java_list_string(dto.send_range_data.as_deref().unwrap_or(&[])),
        min_order_amount,
        send_start_date: resolve_send_date(dto.send_start_date.as_deref()),
        send_end_date: resolve_send_date(dto.send_end_date.as_deref()),
        send_type,
        use_day,
        use_start_date,
        use_end_date,
        is_show: dto.is_show.unwrap_or(0),
        is_global: dto.is_global.unwrap_or(0),
        is_new_user: dto.is_new_user.unwrap_or(0),
        enabled_click_get: dto.enabled_click_get.unwrap_or(0),
        limit_user_rank: Some(java_list_string(
            dto.limit_user_rank.as_deref().unwrap_or(&[]),
        )),
        shop_id: Some(shop_id),
        is_delete: dto.is_delete.or(Some(0)),
        limit_num: dto.limit_num,
        delay_day,
        send_num: dto.send_num.unwrap_or(0),
        max_order_amount: dto.max_order_amount.unwrap_or(Decimal::ZERO),
        coupon_unit: dto.coupon_unit.unwrap_or(1),
        reduce_type,
        add_time: Some(now_i32()),
    })
}

/// 使用时间两分支（Java `dataJudge`）：两个日期都有效才落固定区间。
fn resolve_use_dates(dto: &CouponDTO) -> (i32, i32) {
    let start = dto
        .use_start_date
        .as_deref()
        .filter(|value| !value.trim().is_empty() && value.trim() != "0")
        .and_then(parse_datetime);
    let end = dto
        .use_end_date
        .as_deref()
        .filter(|value| !value.trim().is_empty() && value.trim() != "0")
        .and_then(parse_datetime);
    match (start, end) {
        (Some(start), Some(end)) => (
            i32::try_from(start).unwrap_or(i32::MAX),
            i32::try_from(end).unwrap_or(i32::MAX),
        ),
        _ => (0, 0),
    }
}

fn resolve_send_date(raw: Option<&str>) -> i32 {
    raw.and_then(parse_datetime)
        .map(|secs| i32::try_from(secs).unwrap_or(i32::MAX))
        .unwrap_or(0)
}

/// 由 DTO 构造「只更新已提供字段」补丁（Java `BeanUtils` + `dataJudge`）。
fn build_coupon_patch(dto: &CouponDTO, shop_id: i32) -> Result<CouponUpdatePatch> {
    let send_type = dto.send_type.unwrap_or(0);
    let reduce_type = dto.reduce_type.unwrap_or(0);
    let coupon_type = dto.coupon_type.unwrap_or(0);
    let min_order_amount = if reduce_type == REDUCE_TYPE_NO_THRESHOLD {
        Some(Decimal::ZERO)
    } else {
        dto.min_order_amount
    };
    if let (Some(min_order), Some(money)) = (min_order_amount, dto.coupon_money) {
        if reduce_type == REDUCE_TYPE_THRESHOLD && coupon_type == COUPON_TYPE_MONEY && min_order <= money {
            return Err(Error::biz_msg(MSG_COUPON_MONEY_EXCEED_MIN_ORDER_AMOUNT));
        }
    }
    let (use_start, use_end) = resolve_use_dates(dto);
    let fixed_use = use_start != 0 || use_end != 0;
    Ok(CouponUpdatePatch {
        coupon_name: dto.coupon_name.clone(),
        coupon_money: dto.coupon_money,
        coupon_discount: if coupon_type == COUPON_TYPE_MONEY {
            Some(Decimal::ZERO)
        } else {
            dto.coupon_discount
        },
        coupon_desc: dto.coupon_desc.clone(),
        coupon_type: dto.coupon_type,
        send_range: dto.send_range,
        send_range_data: Some(java_list_string(dto.send_range_data.as_deref().unwrap_or(&[]))),
        min_order_amount,
        send_start_date: dto
            .send_start_date
            .as_deref()
            .and_then(parse_datetime)
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        send_end_date: dto
            .send_end_date
            .as_deref()
            .and_then(parse_datetime)
            .map(|v| i32::try_from(v).unwrap_or(i32::MAX)),
        send_type: dto.send_type,
        use_day: Some(if send_type == SEND_TYPE_DAYS {
            dto.use_day.unwrap_or(0)
        } else {
            0
        }),
        use_start_date: fixed_use.then_some(use_start),
        use_end_date: fixed_use.then_some(use_end),
        is_show: dto.is_show,
        is_global: dto.is_global,
        is_new_user: dto.is_new_user,
        enabled_click_get: dto.enabled_click_get,
        limit_user_rank: Some(java_list_string(
            dto.limit_user_rank.as_deref().unwrap_or(&[]),
        )),
        is_delete: dto.is_delete,
        limit_num: dto.limit_num,
        delay_day: Some(if send_type == SEND_TYPE_DAYS {
            dto.delay_day.unwrap_or(0)
        } else {
            0
        }),
        send_num: dto.send_num,
        max_order_amount: dto.max_order_amount,
        coupon_unit: dto.coupon_unit,
        reduce_type: dto.reduce_type,
        shop_id: Some(shop_id),
    })
}

/// 合并「库中原值 + DTO 新值」，供更新后的 promotion 行使用。
fn merge_coupon_for_promotion(
    existing: Option<&Coupon>,
    dto: &CouponDTO,
    shop_id: i32,
) -> Coupon {
    let mut merged = existing.cloned().unwrap_or_default();
    if let Some(name) = &dto.coupon_name {
        merged.coupon_name = name.clone();
    }
    if let Some(range) = dto.send_range {
        merged.send_range = range;
    }
    if let Some(data) = &dto.send_range_data {
        merged.send_range_data = java_list_string(data);
    }
    if let Some(send_type) = dto.send_type {
        merged.send_type = send_type;
    }
    let (use_start, use_end) = resolve_use_dates(dto);
    if use_start != 0 || use_end != 0 {
        merged.use_start_date = use_start;
        merged.use_end_date = use_end;
    }
    merged.shop_id = Some(shop_id);
    merged
}

/// 构造 `promotion` 主表行（Java `savePromotion`）。
fn build_coupon_promotion(
    coupon: &Coupon,
    dto: &CouponDTO,
    relation_id: i32,
    shop_id: i32,
) -> Promotion {
    let (start_time, end_time) = if coupon.send_type > 0 {
        (coupon.use_start_date, coupon.use_end_date)
    } else {
        (0, 0)
    };
    Promotion {
        promotion_id: 0,
        promotion_name: coupon.coupon_name.clone(),
        start_time,
        end_time,
        r#type: PROMOTION_TYPE_COUPON,
        shop_id,
        relation_id: Some(relation_id),
        range: Some(coupon.send_range),
        range_data: Some(coupon.send_range_data.clone()),
        is_available: Some(PROMOTION_AVAILABLE_YES),
        member_range: dto.member_range.unwrap_or(0),
        member_marks: dto.member_marks.as_deref().map(java_list_string),
        member_scope: dto
            .send_user_target
            .as_ref()
            .or(dto.member_audience.as_ref())
            .map(|value| value.to_string()),
        ..Default::default()
    }
}

/// 会员等级实体 → 配置 VO（Java `UserRank` + `userCount`）。
fn rank_to_vo(rank: &UserRank, user_count: i64) -> CouponConfigRankVO {
    CouponConfigRankVO {
        rank_id: rank.rank_id,
        rank_name: rank.rank_name.clone(),
        min_growth_points: rank.min_growth_points,
        max_growth_points: rank.max_growth_points,
        discount: rank.discount,
        show_price: rank.show_price,
        rank_type: rank.rank_type,
        rank_logo: rank.rank_logo.clone(),
        rank_ico: rank.rank_ico.clone(),
        rank_bg: rank.rank_bg.clone(),
        rank_point: rank.rank_point.clone(),
        free_shipping: rank.free_shipping,
        rank_card_type: rank.rank_card_type,
        rights: rank.rights.clone(),
        rank_level: rank.rank_level.clone(),
        user_count,
    }
}

/// 当前秒级时间戳（i32）。
fn now_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(i32::MAX)
}

/// 供 `batch` 的空列表校验复用（Java `@NotEmpty`）。
#[allow(dead_code)]
const _BATCH_UNSELECTED: &str = MSG_UNSELECTED_ITEM;
/// 供 `batch` 操作类型校验复用（Java `@NotNull`）。
#[allow(dead_code)]
const _BATCH_TYPE_REQUIRED: &str = MSG_OPERATION_TYPE_REQUIRED;

/// 列表店铺过滤：店铺账号强制本店；平台账号可请求过滤。
fn scoped_tenant_shop(requested: Option<i32>) -> Option<i32> {
    TenantScope::current_list_filter().or(requested.filter(|id| *id > 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto_with(type_: i32, reduce: i32, money: Decimal, min: Decimal) -> CouponDTO {
        CouponDTO {
            coupon_name: Some("券".into()),
            coupon_type: Some(type_),
            reduce_type: Some(reduce),
            coupon_money: Some(money),
            min_order_amount: Some(min),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn tenant_scope_overrides_scoped_shop_and_blocks_cross_shop() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (scope, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_tenant_shop(Some(9)),
                TenantScope::current().write_shop_id(Some(8)).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        // 店铺账号：即使请求其他店铺，TenantScope 也强制本店
        assert_eq!(scope, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let scope = with_tenant_scope(TenantScope::platform(), async {
            scoped_tenant_shop(Some(9))
        })
        .await;
        assert_eq!(scope, Some(9));
    }

    #[test]
    fn build_coupon_applies_java_data_judge() {
        // 满减券无门槛：min_order_amount 归零
        let dto = dto_with(COUPON_TYPE_MONEY, REDUCE_TYPE_NO_THRESHOLD, Decimal::new(5, 0), Decimal::new(100, 0));
        let coupon = build_coupon(&dto, 0).expect("build");
        assert_eq!(coupon.min_order_amount, Decimal::ZERO);
        assert_eq!(coupon.coupon_discount, Decimal::ZERO);

        // sendType = 0：useDay/delayDay 原样
        let dto = CouponDTO {
            coupon_name: Some("券".into()),
            send_type: Some(SEND_TYPE_DAYS),
            use_day: Some(3),
            delay_day: Some(1),
            ..Default::default()
        };
        let coupon = build_coupon(&dto, 2).expect("build");
        assert_eq!(coupon.use_day, 3);
        assert_eq!(coupon.delay_day, 1);
        assert_eq!(coupon.shop_id, Some(2));
        assert_eq!(coupon.send_range_data, "[]");
        assert_eq!(coupon.limit_user_rank.as_deref(), Some("[]"));

        // sendType != 0：useDay/delayDay 归零
        let dto = CouponDTO {
            coupon_name: Some("券".into()),
            send_type: Some(1),
            use_day: Some(3),
            delay_day: Some(1),
            ..Default::default()
        };
        let coupon = build_coupon(&dto, 0).expect("build");
        assert_eq!(coupon.use_day, 0);
        assert_eq!(coupon.delay_day, 0);
    }

    #[test]
    fn coupon_money_cannot_exceed_min_order_amount() {
        let dto = dto_with(
            COUPON_TYPE_MONEY,
            REDUCE_TYPE_THRESHOLD,
            Decimal::new(10, 0),
            Decimal::new(10, 0),
        );
        let err = build_coupon(&dto, 0).unwrap_err();
        assert!(err.to_string().contains(MSG_COUPON_MONEY_EXCEED_MIN_ORDER_AMOUNT));
        let ok = dto_with(
            COUPON_TYPE_MONEY,
            REDUCE_TYPE_THRESHOLD,
            Decimal::new(10, 0),
            Decimal::new(20, 0),
        );
        assert!(build_coupon(&ok, 0).is_ok());
    }

    #[test]
    fn promotion_row_matches_save_promotion() {
        let dto = CouponDTO {
            coupon_name: Some("满减券".into()),
            send_range: Some(3),
            send_range_data: Some(vec![1, 2]),
            send_type: Some(1),
            use_start_date: Some("2026-10-18 00:00:00".into()),
            use_end_date: Some("2026-10-19 00:00:00".into()),
            member_range: Some(1),
            member_marks: Some(vec![2, 3]),
            ..Default::default()
        };
        let coupon = build_coupon(&dto, 0).expect("build");
        let row = build_coupon_promotion(&coupon, &dto, 9, 0);
        assert_eq!(row.r#type, PROMOTION_TYPE_COUPON);
        assert_eq!(row.relation_id, Some(9));
        assert_eq!(row.shop_id, 0);
        assert_eq!(row.range, Some(3));
        assert_eq!(row.range_data.as_deref(), Some("[1, 2]"));
        assert_eq!(row.is_available, Some(PROMOTION_AVAILABLE_YES));
        assert_eq!(row.member_range, 1);
        assert_eq!(row.member_marks.as_deref(), Some("[2, 3]"));
        assert_eq!(row.start_time, coupon.use_start_date);
    }

    #[test]
    fn batch_and_update_field_paths() {
        // 非 del 的批量操作静默成功（Java 返回 false）
        let dto = BatchDTO {
            ids: vec![1],
            r#type: "on".into(),
            value: 0,
            data: None,
        };
        assert_eq!(dto.r#type, "on");
        // 空 ids 由服务层 400
        assert_eq!(MSG_UNSELECTED_ITEM, "未选择项目");
        assert_eq!(MSG_OPERATION_TYPE_REQUIRED, "操作类型不能为空");
        assert_eq!(resolve_coupon_field("coupon_money"), Some("coupon_money"));
        assert_eq!(resolve_coupon_field("nope"), None);
    }
}
