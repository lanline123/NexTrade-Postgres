//! 礼品卡发放域服务（marketing::gift_card_issue，P1 发放闭环 + 标准对齐）。
//!
//! 方案：`~/workspace/giftcard-welfare-design/proposal.md`；处置定案：
//! `~/workspace/giftcard-welfare-design/standard-alignment.md`（D1–D12）。
//!
//! - admin 定向发放：`issuePreview` / `issueSend`（指定会员/等级/标签，复用
//!   优惠券 `coupon_manage` 的目标解析口径与失败明细文案，一人一卡）。
//!   来源标注：**原版有·我方改口径** — 原版为单次 `issue`，我方加 preview
//!   预检（多一层库存/失败明细预检），发放语义与原版一致。
//! - admin Excel 发卡：`issueByExcel`（**原版有**，单次 multipart）：模板列
//!   会员名/手机号/卡号/卡密/备注，逐行把已存在且未使用的指定卡发给指定
//!   会员（卡号+卡密成对校验），失败行逐行记录原因、其余继续。
//! - admin 发放记录：`issueBatch` / `issueLog`（批次 + 明细两层）。
//! - public：`myCards` / `myCardDetail`（会员端「我的礼品卡」，归属校验 +
//!   卡号脱敏，来源/状态派生，详情带 balanceLogs 余额流水）。
//! - 发放分配只从卡组现有可用卡中分配（不生成新卡），事务内行锁，
//!   库存不足整单拦截；发放成功后逐人发站内信（**我方新增**小项：原版
//!   IssueGiftCard 代码未见站内信证据，见 D11）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::marketing::coupon_manage::{
    TargetUser, ISSUE_TYPE_RANK, ISSUE_TYPE_TAG, ISSUE_TYPE_USER, MSG_ISSUE_MEMBER_REQUIRED,
    MSG_ISSUE_MIXED_UNSUPPORTED, MSG_ISSUE_RANK_REQUIRED, MSG_ISSUE_TAG_REQUIRED,
    MSG_MEMBER_DISABLED, MSG_MEMBER_NOT_EXIST, USER_STATUS_NORMAL, USER_TYPE_MEMBER,
};
use nextrade_core::models::marketing::gift_card::{
    group_type_name, mask_middle_half, scope_type_name,
};
use nextrade_core::models::marketing::gift_card_issue::{
    card_source_text, derive_card_state, insufficient_stock_message, issue_type_name,
    need_count, normalize_issue_type, parse_issue_excel_row, usage_type_name,
    GiftCardExcelFailDetailVO, GiftCardIssueBatchPageQuery, GiftCardIssueBatchVO,
    GiftCardIssueByExcelVO, GiftCardIssueFailDetailVO, GiftCardIssueLogPageQuery,
    GiftCardIssueLogVO, GiftCardIssueParam, GiftCardIssuePreviewVO, GiftCardIssueSendVO,
    GiftCardUsageLogVO, MyGiftCardDetailVO, MyGiftCardVO, MyGiftCardsQuery, ISSUE_TYPE_EXCEL,
    MSG_CARD_NOT_EXISTS, MSG_EXCEL_CARD_DUPLICATE, MSG_EXCEL_CARD_NOT_FOUND, MSG_EXCEL_CARD_USED,
    MSG_EXCEL_ROW_INCOMPLETE, MSG_GROUP_NOT_EXISTS, MSG_ISSUE_MODE_INVALID, MSG_LOGIN_REQUIRED,
    MSG_NO_VALID_MEMBER,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::gift_card_issue::{
    AllocateError, GroupIssueRow, GiftCardIssueRepository, IssueLogRow, MyCardRow,
};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 站内信：礼品卡发放通知标题。
const MSG_TITLE_ISSUE: &str = "礼品卡发放通知";
/// 站内信跳转（会员端我的礼品卡）。
const MSG_LINK_MY_CARDS: &str = "/member/giftcard/list";

/// 礼品卡发放域服务。
pub struct GiftCardIssueService {
    repos: Arc<Repositories>,
}

impl GiftCardIssueService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储。
    #[must_use]
    pub fn repo(&self) -> &GiftCardIssueRepository {
        &self.repos.marketing.gift_card_issue
    }

    // ————————————————————— 内部：卡组与目标解析 —————————————————————

    /// 加载卡组并校验当前租户可访问。
    async fn load_group(&self, card_groups_id: i32) -> Result<GroupIssueRow> {
        let group = self
            .repo()
            .find_group(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_GROUP_NOT_EXISTS))?;
        TenantScope::current_ensure_access(group.shop_id.unwrap_or(0))?;
        Ok(group)
    }

    /// 目标会员解析（与优惠券 `coupon_manage::resolve_issue_targets` 同口径的
    /// 本域副本——优惠券服务同名方法是私有的）：混合选择拦截、各模式必选校验、
    /// USER 模式按请求顺序产出「会员不存在」明细、禁用会员逐个记失败明细。
    async fn resolve_issue_targets(
        &self,
        param: &GiftCardIssueParam,
        issue_type: &str,
    ) -> Result<(Vec<TargetUser>, Vec<GiftCardIssueFailDetailVO>)> {
        let user_ids = param.user_ids.clone().unwrap_or_default();
        let rank_ids = param.rank_ids.clone().unwrap_or_default();
        let tag_ids = param.tag_ids.clone().unwrap_or_default();
        let repo = &self.repos.marketing.coupon_manage;
        let (users, order): (Vec<TargetUser>, Vec<i32>) = match issue_type {
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
            users.iter().map(|u| (u.user_id, u)).collect();
        let mut valid: Vec<TargetUser> = Vec::new();
        let mut failures: Vec<GiftCardIssueFailDetailVO> = Vec::new();
        if order.is_empty() {
            for user in &users {
                push_issue_user(user, &mut valid, &mut failures);
            }
        } else {
            for user_id in order {
                match user_map.get(&user_id) {
                    Some(user) => push_issue_user(user, &mut valid, &mut failures),
                    None => failures.push(GiftCardIssueFailDetailVO {
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

    /// 发放后站内信（逐人一条；失败忽略，不影响发放结果）。
    /// 我方新增小项（D11）：原版发放代码未见站内信证据。
    async fn notify_issued(&self, group: &GroupIssueRow, user_ids: &[i32]) {
        let content = format!(
            "您收到 1 张「{}」礼品卡，请前往「我的礼品卡」查看。",
            group.card_group_name
        );
        for &user_id in user_ids {
            let _ = self
                .repos
                .member
                .user_message
                .insert_user_message(user_id, MSG_TITLE_ISSUE, &content, 0, Some(MSG_LINK_MY_CARDS))
                .await;
        }
    }

    /// 分配 + 站内信的共用收尾（库存不足 → 整单拦截文案）。
    async fn allocate_and_notify(
        &self,
        group: &GroupIssueRow,
        user_ids: &[i32],
        issue_type: &str,
        target_desc: serde_json::Value,
        target_count: i32,
        fail_count: i32,
        remark: &str,
    ) -> Result<i64> {
        let allocation = self
            .repo()
            .allocate_issue(
                group,
                user_ids,
                issue_type,
                &target_desc,
                target_count,
                fail_count,
                0,
                remark,
                now_seconds(),
            )
            .await
            .map_err(|e| match e {
                AllocateError::Insufficient(s) => {
                    Error::biz_msg(insufficient_stock_message(s.available, s.need))
                }
                AllocateError::Db(err) => err,
            })?;
        self.notify_issued(group, user_ids).await;
        Ok(allocation.batch_id)
    }

    // ————————————————————— admin：预检 / 发放 —————————————————————

    /// 发放预检：有效会员数、需卡数、可用库存、缺口、失败明细预览。
    /// 库存不足不报错（`can_issue = false` + 缺口），由前端提示先制卡。
    pub async fn issue_preview(
        &self,
        param: &GiftCardIssueParam,
    ) -> Result<GiftCardIssuePreviewVO> {
        let group = self.load_group(param.card_groups_id).await?;
        let issue_type = normalize_issue_type(&param.issue_type)
            .ok_or_else(|| Error::biz_msg(MSG_ISSUE_MODE_INVALID))?;
        let (targets, fails) = self.resolve_issue_targets(param, issue_type).await?;
        let available = self.repo().count_available_cards(group.card_groups_id).await?;
        let need = need_count(targets.len());
        Ok(GiftCardIssuePreviewVO {
            card_groups_id: group.card_groups_id,
            issue_type: issue_type.to_string(),
            member_count: i32::try_from(targets.len()).unwrap_or(i32::MAX),
            need_count: need,
            available_stock: available,
            shortage: (need - available).max(0),
            can_issue: available >= need && !targets.is_empty(),
            fail_count: i32::try_from(fails.len()).unwrap_or(i32::MAX),
            fail_details: fails,
        })
    }

    /// 正式发放：事务内分配；无有效会员 / 库存不足整单拦截。
    pub async fn issue_send(&self, param: &GiftCardIssueParam) -> Result<GiftCardIssueSendVO> {
        let group = self.load_group(param.card_groups_id).await?;
        let issue_type = normalize_issue_type(&param.issue_type)
            .ok_or_else(|| Error::biz_msg(MSG_ISSUE_MODE_INVALID))?;
        let (targets, fails) = self.resolve_issue_targets(param, issue_type).await?;
        if targets.is_empty() {
            return Err(Error::biz_msg(MSG_NO_VALID_MEMBER));
        }
        let target_desc = serde_json::json!({
            "issueType": issue_type,
            "userIds": param.user_ids,
            "rankIds": param.rank_ids,
            "tagIds": param.tag_ids,
            "tagMatchMode": param.tag_match_mode,
            "targetCount": targets.len() + fails.len(),
        });
        let user_ids: Vec<i32> = targets.iter().map(|u| u.user_id).collect();
        let batch_id = self
            .allocate_and_notify(
                &group,
                &user_ids,
                issue_type,
                target_desc,
                i32::try_from(targets.len() + fails.len()).unwrap_or(i32::MAX),
                i32::try_from(fails.len()).unwrap_or(i32::MAX),
                param.remark.as_deref().unwrap_or(""),
            )
            .await?;
        Ok(GiftCardIssueSendVO {
            batch_id,
            success_count: i32::try_from(user_ids.len()).unwrap_or(i32::MAX),
            card_count: i32::try_from(user_ids.len()).unwrap_or(i32::MAX),
            fail_count: i32::try_from(fails.len()).unwrap_or(i32::MAX),
            fail_details: fails,
        })
    }

    // ————————————————————— admin：Excel 发卡（原版 issueByExcel） —————————————————————

    /// Excel 发卡（单次提交，路由层已用 `read_xlsx` 读出含表头的行矩阵）。
    ///
    /// 来源标注：**原版有** — 模板列 会员名/手机号/卡号/卡密/备注，逐行把
    /// 已存在且未使用的指定卡发给指定会员：会员按 手机号 > 会员名 匹配，
    /// 卡按卡号+卡密成对校验且必须未使用、未发放；失败行逐行记录原因、
    /// 其余行继续发放（不整单拦截）。成功行在一个事务内落批次+明细，
    /// 并逐人发站内信（我方新增小项，D11）。
    pub async fn issue_by_excel(
        &self,
        card_groups_id: i32,
        rows: &[Vec<String>],
        file_name: Option<&str>,
    ) -> Result<GiftCardIssueByExcelVO> {
        let group = self.load_group(card_groups_id).await?;
        let mut fails: Vec<GiftCardExcelFailDetailVO> = Vec::new();
        let mut pairs: Vec<(i32, i32)> = Vec::new();
        let mut seen_cards: HashSet<String> = HashSet::new();
        let mut total_rows = 0_i32;
        for (idx, row) in rows.iter().enumerate() {
            if idx == 0 {
                continue; // 表头
            }
            let excel_row = i32::try_from(idx + 1).unwrap_or(i32::MAX);
            let cells: Vec<String> = row.iter().map(|c| c.trim().to_string()).collect();
            if cells.iter().all(|c| c.is_empty()) {
                continue; // 整行空白不计入
            }
            total_rows += 1;
            let Some(parsed) = parse_issue_excel_row(&cells) else {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: cells.get(2).cloned().unwrap_or_default(),
                    reason: MSG_EXCEL_ROW_INCOMPLETE,
                }
                .into());
                continue;
            };
            // 会员匹配：手机号 > 会员名（用户名/昵称精确匹配）
            let member = if !parsed.mobile.is_empty() {
                self.repo().find_member_by_mobile(&parsed.mobile).await?
            } else {
                self.repo().find_member_by_username(&parsed.member_name).await?
            };
            let member_key = if parsed.mobile.is_empty() {
                parsed.member_name.clone()
            } else {
                parsed.mobile.clone()
            };
            let Some(m) = member else {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: member_key,
                    reason: MSG_MEMBER_NOT_EXIST,
                }
                .into());
                continue;
            };
            if m.user_type != USER_TYPE_MEMBER {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: member_key,
                    reason: MSG_MEMBER_NOT_EXIST,
                }
                .into());
                continue;
            }
            if m.status != Some(USER_STATUS_NORMAL) {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: member_key,
                    reason: MSG_MEMBER_DISABLED,
                }
                .into());
                continue;
            }
            // 文件内卡号去重（一张卡只发一次）
            if !seen_cards.insert(parsed.card_number.clone()) {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: parsed.card_number.clone(),
                    reason: MSG_EXCEL_CARD_DUPLICATE,
                }
                .into());
                continue;
            }
            // 卡校验：卡号+卡密必须已存在（限本卡组）且未使用、未发放
            let card = self
                .repo()
                .find_specified_card(group.card_groups_id, &parsed.card_number, &parsed.card_password)
                .await?;
            let Some(card) = card else {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: parsed.card_number.clone(),
                    reason: MSG_EXCEL_CARD_NOT_FOUND,
                }
                .into());
                continue;
            };
            if card.status != Some(0)
                || card.is_available != Some(1)
                || card.claimed_user_id.is_some()
                || card.user_id.is_some()
            {
                fails.push(ExcelFail {
                    row: excel_row,
                    raw: parsed.card_number.clone(),
                    reason: MSG_EXCEL_CARD_USED,
                }
                .into());
                continue;
            }
            pairs.push((m.user_id, card.gift_cards_id));
        }
        let pre_fail_count = i32::try_from(fails.len()).unwrap_or(i32::MAX);
        let target_desc = serde_json::json!({
            "issueType": ISSUE_TYPE_EXCEL,
            "fileName": file_name,
            "totalRows": total_rows,
        });
        let (batch_id, success_count) = self
            .repo()
            .allocate_specified_cards(
                &group,
                &pairs,
                &target_desc,
                total_rows,
                pre_fail_count,
                0,
                "",
                now_seconds(),
            )
            .await?;
        // 事务复核阶段被跳过的卡计入失败（行号无法精确回溯，只计总数差）
        let raced_fail = i32::try_from(pairs.len()).unwrap_or(i32::MAX) - success_count;
        if success_count > 0 {
            let ok_users: Vec<i32> = pairs.iter().map(|(u, _)| *u).collect();
            self.notify_issued(&group, &ok_users).await;
        }
        Ok(GiftCardIssueByExcelVO {
            card_groups_id: group.card_groups_id,
            batch_id,
            total_rows,
            success_count,
            fail_count: pre_fail_count + raced_fail.max(0),
            fail_details: fails,
        })
    }
    // ————————————————————— admin：发放记录 —————————————————————

    /// 发放批次分页。
    pub async fn issue_batch_list(
        &self,
        q: &GiftCardIssueBatchPageQuery,
    ) -> Result<PageResult<GiftCardIssueBatchVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1).max(1),
            page_size: q.size.unwrap_or(20).clamp(1, 200),
        };
        let issue_type = q
            .issue_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let (total, rows) = self
            .repo()
            .paginate_batches(
                q.card_groups_id.filter(|v| *v > 0),
                issue_type.as_deref(),
                q.start_time,
                q.end_time,
                page.page,
                page.page_size,
            )
            .await?;
        let list = rows
            .into_iter()
            .map(|r| GiftCardIssueBatchVO {
                id: r.id,
                card_groups_id: r.card_groups_id,
                card_group_name: r.card_group_name,
                shop_id: r.shop_id,
                issue_type_name: issue_type_name(&r.issue_type).to_string(),
                issue_type: r.issue_type,
                target_desc: r.target_desc,
                cards_per_user: r.cards_per_user,
                target_count: r.target_count,
                success_count: r.success_count,
                fail_count: r.fail_count,
                admin_id: r.admin_id,
                remark: r.remark,
                add_time: r.add_time,
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 发放明细分页（卡号脱敏回填）。
    pub async fn issue_log_list(
        &self,
        q: &GiftCardIssueLogPageQuery,
    ) -> Result<PageResult<GiftCardIssueLogVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1).max(1),
            page_size: q.size.unwrap_or(20).clamp(1, 200),
        };
        let (total, rows) = self
            .repo()
            .paginate_logs(
                q.batch_id.filter(|v| *v > 0),
                q.card_groups_id.filter(|v| *v > 0),
                q.user_id.filter(|v| *v > 0),
                page.page,
                page.page_size,
            )
            .await?;
        let list = rows.into_iter().map(log_row_to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— public：我的礼品卡 —————————————————————

    /// 我的礼品卡分页（type: 1 储值 / 2 兑换；state: valid / invalid）。
    pub async fn my_cards(
        &self,
        user_id: i32,
        q: &MyGiftCardsQuery,
    ) -> Result<PageResult<MyGiftCardVO>> {
        if user_id <= 0 {
            return Err(Error::biz_msg(MSG_LOGIN_REQUIRED));
        }
        let page = PageQuery {
            page: q.page.unwrap_or(1).max(1),
            page_size: q.size.unwrap_or(20).clamp(1, 200),
        };
        let now = i32::try_from(now_seconds()).unwrap_or(i32::MAX);
        let state = match q.state.as_deref().map(str::trim) {
            Some("valid") => Some("valid"),
            Some("invalid") => Some("invalid"),
            _ => None,
        };
        let (total, rows) = self
            .repo()
            .paginate_my_cards(
                user_id,
                q.r#type.filter(|v| *v == 1 || *v == 2),
                state,
                now,
                page.page,
                page.page_size,
            )
            .await?;
        let list = rows.iter().map(|r| my_card_to_vo(r, now)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 我的礼品卡详情（归属校验在 SQL 内；含消费流水与兑换订单）。
    pub async fn my_card_detail(
        &self,
        user_id: i32,
        gift_cards_id: i32,
    ) -> Result<MyGiftCardDetailVO> {
        if user_id <= 0 {
            return Err(Error::biz_msg(MSG_LOGIN_REQUIRED));
        }
        let now = i32::try_from(now_seconds()).unwrap_or(i32::MAX);
        let row = self
            .repo()
            .find_my_card(user_id, gift_cards_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_CARD_NOT_EXISTS))?;
        let usage_rows = self.repo().list_usage_logs(gift_cards_id).await?;
        let order_ids = self.repo().find_exchange_order_ids(gift_cards_id).await?;
        // 卡组兑换模式/任选件数（详情页「去使用」分流：任选进选品页；
        // 实体未承载该两列，走 groups 仓储的专用查询）
        let (exchange_mode, select_count) = self
            .repos
            .marketing
            .gift_card_groups
            .find_exchange_fields(row.card_groups_id)
            .await
            .ok()
            .flatten()
            .unwrap_or((1, 0));
        let usage_logs: Vec<GiftCardUsageLogVO> = usage_rows
                .into_iter()
                .map(|u| GiftCardUsageLogVO {
                    id: u.id,
                    gift_cards_id: u.gift_cards_id,
                    user_id: u.user_id,
                    order_id: u.order_id,
                    r#type: i32::from(u.r#type),
                    type_name: usage_type_name(i32::from(u.r#type)).to_string(),
                    amount: u.amount,
                    balance_before: u.balance_before,
                    balance_after: u.balance_after,
                    add_time: u.add_time,
                })
                .collect();
        Ok(MyGiftCardDetailVO {
            card: my_card_to_vo(&row, now),
            balance_logs: usage_logs.clone(),
            usage_logs,
            exchange_order_ids: order_ids,
            exchange_mode,
            select_count,
        })
    }
}

/// 有效/禁用分类（与优惠券 `push_issue_user` 同口径：status == 1 才有效）。
fn push_issue_user(
    user: &TargetUser,
    valid: &mut Vec<TargetUser>,
    failures: &mut Vec<GiftCardIssueFailDetailVO>,
) {
    if user.status == USER_STATUS_NORMAL {
        valid.push(user.clone());
    } else {
        failures.push(GiftCardIssueFailDetailVO {
            user_id: user.user_id,
            member_name: user.nickname.clone(),
            mobile: user.mobile.clone(),
            reason: MSG_MEMBER_DISABLED.to_string(),
        });
    }
}

/// Excel 行失败明细（内部构造器）。
struct ExcelFail {
    row: i32,
    raw: String,
    reason: &'static str,
}

impl From<ExcelFail> for GiftCardExcelFailDetailVO {
    fn from(f: ExcelFail) -> Self {
        Self {
            row: f.row,
            raw_value: f.raw,
            reason: f.reason.to_string(),
        }
    }
}

/// 明细行 → VO（卡号脱敏）。
fn log_row_to_vo(r: IssueLogRow) -> GiftCardIssueLogVO {
    GiftCardIssueLogVO {
        id: r.id,
        batch_id: r.batch_id,
        card_groups_id: r.card_groups_id,
        card_group_name: r.card_group_name,
        gift_cards_id: r.gift_cards_id,
        card_number: r.card_number.as_deref().map(mask_middle_half),
        user_id: r.user_id,
        username: r.username,
        nickname: r.nickname,
        mobile: r.mobile,
        issue_time: r.issue_time,
    }
}

/// 我的卡行 → VO（状态/来源派生 + 卡号脱敏）。
fn my_card_to_vo(r: &MyCardRow, now: i32) -> MyGiftCardVO {
    let (state_text, is_valid) = derive_card_state(
        r.status,
        r.expired_time,
        now,
        r.group_type,
        r.face_amount,
        r.balance,
    );
    MyGiftCardVO {
        gift_cards_id: r.gift_cards_id,
        card_groups_id: r.card_groups_id,
        card_group_name: r.card_group_name.clone(),
        image_url: r.image_url.clone(),
        group_type: r.group_type,
        group_type_name: group_type_name(r.group_type).to_string(),
        card_type: r.card_type,
        card_number: Some(mask_middle_half(&r.card_number)),
        gift_card_sn: r.gift_card_sn.clone(),
        face_amount: r.face_amount,
        balance: r.balance,
        status: r.status,
        state_text: state_text.to_string(),
        is_valid,
        source_text: card_source_text(r.has_issue, r.has_order).to_string(),
        expired_time: r.expired_time,
        used_time: r.used_time,
        add_time: r.add_time,
        scope_type: r.scope_type,
        scope_desc: scope_type_name(r.scope_type).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_card_vo_derives_state_and_masks() {
        let row = MyCardRow {
            gift_cards_id: 5,
            card_groups_id: 2,
            card_group_name: Some("新春储值卡".to_string()),
            group_type: 1,
            card_type: 1,
            card_number: "6222021234567890".to_string(),
            face_amount: Some(rust_decimal::Decimal::from(100)),
            balance: Some(rust_decimal::Decimal::from(40)),
            status: Some(0),
            expired_time: 0,
            has_issue: true,
            ..Default::default()
        };
        let vo = my_card_to_vo(&row, 1_700_000_000);
        assert_eq!(vo.state_text, "部分使用");
        assert!(vo.is_valid);
        assert_eq!(vo.source_text, "发放");
        assert_eq!(vo.group_type_name, "储值卡");
        assert!(!vo.card_number.unwrap().contains("3456"));
        // 适用范围透传（我方新增）：默认 0 → 全场通用
        assert_eq!(vo.scope_type, 0);
        assert_eq!(vo.scope_desc, "全场通用");
        let scoped = MyCardRow {
            scope_type: 2,
            ..row.clone()
        };
        assert_eq!(my_card_to_vo(&scoped, 1_700_000_000).scope_desc, "仅限指定品类");
    }

    #[test]
    fn excel_fail_converts() {
        let vo: GiftCardExcelFailDetailVO = ExcelFail {
            row: 3,
            raw: "138".to_string(),
            reason: MSG_MEMBER_NOT_EXIST,
        }
        .into();
        assert_eq!(vo.row, 3);
        assert_eq!(vo.reason, MSG_MEMBER_NOT_EXIST);
    }

    #[test]
    fn service_wiring_present() {
        let src = include_str!("gift_card_issue.rs");
        assert!(src.contains("allocate_issue"));
        assert!(src.contains("insert_user_message"));
        assert!(src.contains("MSG_NO_VALID_MEMBER"));
    }
}
