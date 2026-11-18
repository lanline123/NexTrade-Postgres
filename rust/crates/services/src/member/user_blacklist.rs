//! 客户黑名单域服务（member::user_blacklist）。
//!
//! 设计定稿见 `~/workspace/blacklist-design/proposal.md` §4/§5：
//! - 校验侧：[`UserBlacklistService::find_restriction`] / `ensure_allowed` /
//!   `find_order_restrictions`，供 trade/catalog 等域在拦截点即时构造调用
//!   （先例：`AdminLogService::new(repos.clone())`），不改各服务构造签名。
//! - 管理侧：list/detail/add/batchAdd/remove/batchRemove/logList/setting。
//!   归属一律由 [`BlacklistOperator`]（路由层从 AuthUser 推导）决定，
//!   请求体不设 owner 字段。
//! - 留痕双写：结构化流水 `user_blacklist_log` 为权威记录；每个写操作另写
//!   一条 `admin_log` 文本（失败只 warn，同 shop_member 先例）。
//! - 店铺条目生效条件：平台开关 `shopBlacklistAllow = 1`（§9 定死项 2：
//!   关闭后店铺条目暂停生效，平台条目不受影响）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::shop_member::{
    config_int_or, mask_mobile, mask_name, should_mask, CONFIG_DESENSITIZE, CONFIG_ENABLED,
};
use nextrade_core::models::member::user_blacklist::{
    any_candidate_matches, behavior_name, commission_shop_context, is_valid_combo,
    match_import_user, merge_rule_keys, parse_behavior_code, parse_import_shop_ids,
    parse_product_code, product_name, BlacklistAddParam, BlacklistAddResultVO,
    BlacklistBatchAddParam, BlacklistBatchAddResultVO, BlacklistBatchRemoveParam,
    BlacklistBatchRemoveResultVO, BlacklistEntryVO, BlacklistFailureVO, BlacklistImportErrorVO,
    BlacklistImportResultVO, BlacklistListQuery, BlacklistLogQuery, BlacklistLogVO,
    BlacklistOperator, BlacklistRemoveParam, BlacklistRuleDTO, BlacklistSettingParam,
    BlacklistSettingVO, BlacklistUserVO, RestrictionCandidate, APPLY_ALL, APPLY_SELECTED,
    BEHAVIOR_BECOME_SALESMAN, BEHAVIOR_BUYER_SHOW_COMMENT, BEHAVIOR_BUYER_SHOW_PUBLISH,
    BEHAVIOR_COMMISSION_GRANT, BEHAVIOR_PLACE_ORDER, BEHAVIOR_PRODUCT_COMMENT,
    CONFIG_SHOP_BLACKLIST_ALLOW, IMPORT_HEADERS, IMPORT_MAX_ROWS, MSG_ENTRY_ALREADY_REMOVED,
    MSG_ENTRY_NOT_EXISTS, MSG_NOT_SHOP_MEMBER, MSG_PLATFORM_CANNOT_REMOVE_SHOP, MSG_REASON_REQUIRED,
    MSG_RULES_REQUIRED, MSG_SCOPE_SHOPS_REQUIRED, MSG_SETTING_PLATFORM_ONLY,
    MSG_SHOP_BLACKLIST_DISABLED, MSG_SHOP_CANNOT_REMOVE_OTHERS, MSG_USER_NOT_EXISTS, OP_ADD,
    OP_BATCH_ADD, OP_BATCH_REMOVE, OP_IMPORT_ADD, OP_MERGE, OP_REMOVE, OP_SETTING, OWNER_PLATFORM,
    OWNER_SHOP, PRODUCT_MALL, SOURCE_IMPORT, SOURCE_MANUAL, SOURCE_MEMBER_BATCH, SOURCE_PAGE_BATCH,
    STATUS_ACTIVE,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_blacklist::{
    BlacklistEntry, BlacklistListFilter, BlacklistLogFilter, UserBlacklistRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 命中的限制（组装提示文案与日志用，§4.1）。
#[derive(Debug, Clone)]
pub struct RestrictionHit {
    pub blacklist_id: i64,
    /// 1 平台 / 2 店铺。
    pub owner_type: i32,
    pub owner_shop_id: i32,
    pub product_code: String,
    pub behavior_code: String,
    /// 加入时的限制原因（仅 admin 端可见，用户端文案不引用，§7）。
    pub reason: String,
    /// 命中的店铺上下文（下单批量判定时回填，单点判定为入参 shop_id）。
    pub shop_id: i32,
}

/// 客户黑名单域服务。
pub struct UserBlacklistService {
    repos: Arc<Repositories>,
}

impl UserBlacklistService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &UserBlacklistRepository {
        &self.repos.member.user_blacklist
    }

    // ————————————————————— 配置开关 —————————————————————

    /// 读平台开关 `shopBlacklistAllow`（缺失/非法 ⇒ 0）。
    pub async fn shop_allow(&self) -> Result<bool> {
        let raw = self
            .repos
            .member
            .shop_member
            .config_value(CONFIG_SHOP_BLACKLIST_ALLOW)
            .await?;
        Ok(config_int_or(raw.as_deref(), 0) == 1)
    }

    /// 读取开关 VO（平台/店铺都可读）。
    pub async fn setting_get(&self) -> Result<BlacklistSettingVO> {
        Ok(BlacklistSettingVO {
            shop_blacklist_allow: i32::from(self.shop_allow().await?),
        })
    }

    /// 保存开关（仅平台，路由层 + 服务层双重校验）。
    pub async fn setting_save(
        &self,
        param: &BlacklistSettingParam,
        op: &BlacklistOperator,
    ) -> Result<()> {
        if !op.is_platform {
            return Err(Error::Forbidden(MSG_SETTING_PLATFORM_ONLY.into()));
        }
        let value = param
            .shop_blacklist_allow
            .ok_or_else(|| Error::Validation("shopBlacklistAllow 不能为空".into()))?;
        if !matches!(value, 0 | 1) {
            return Err(Error::Validation("取值 0/1".into()));
        }
        let now = now_seconds();
        self.repos
            .member
            .shop_member
            .set_config_value(CONFIG_SHOP_BLACKLIST_ALLOW, &value.to_string(), now)
            .await?;
        let content = if value == 1 {
            "开启「允许店铺将客户加入店铺黑名单」"
        } else {
            "关闭「允许店铺将客户加入店铺黑名单」（店铺条目暂停生效）"
        };
        self.repo()
            .insert_log(
                0,
                0,
                OP_SETTING,
                content,
                &serde_json::json!([]),
                &scope_snapshot(op, APPLY_ALL, &[]),
                op,
                &current_ip(),
                now,
            )
            .await?;
        self.write_admin_log(op, content).await;
        Ok(())
    }

    // ————————————————————— 校验侧（拦截点调用） —————————————————————

    /// 单点判定：user 在 shop 上下文下是否被限制 (product, behavior)。
    /// `shop_id = None` 表示无店铺上下文（仅平台全平台规则可命中）。
    pub async fn find_restriction(
        &self,
        user_id: i32,
        product: &str,
        behavior: &str,
        shop_id: Option<i32>,
    ) -> Result<Option<RestrictionHit>> {
        if user_id <= 0 {
            return Ok(None);
        }
        let rows = self
            .repo()
            .find_restriction_candidates(user_id, product, behavior)
            .await?;
        if rows.is_empty() {
            return Ok(None);
        }
        // 店铺条目是否生效取决于平台开关；无店铺候选时不查配置。
        let shop_enabled = if rows.iter().any(|r| i32::from(r.owner_type) == OWNER_SHOP) {
            self.shop_allow().await?
        } else {
            false
        };
        let selected_ids: Vec<i64> = rows
            .iter()
            .filter(|r| {
                i32::from(r.owner_type) == OWNER_PLATFORM && i32::from(r.apply_mode) == APPLY_SELECTED
            })
            .map(|r| r.blacklist_id)
            .collect();
        let mut scope_map: HashMap<i64, Vec<i32>> = HashMap::new();
        for (bid, sid) in self.repo().scopes_for(&selected_ids).await? {
            scope_map.entry(bid).or_default().push(sid);
        }
        for row in &rows {
            let candidate = RestrictionCandidate {
                owner_type: i32::from(row.owner_type),
                owner_shop_id: row.owner_shop_id,
                apply_mode: i32::from(row.apply_mode),
                scope_shop_ids: scope_map.get(&row.blacklist_id).cloned().unwrap_or_default(),
            };
            if nextrade_core::models::member::user_blacklist::candidate_matches(
                &candidate,
                shop_id,
                shop_enabled,
            ) {
                return Ok(Some(RestrictionHit {
                    blacklist_id: row.blacklist_id,
                    owner_type: i32::from(row.owner_type),
                    owner_shop_id: row.owner_shop_id,
                    product_code: product.to_string(),
                    behavior_code: behavior.to_string(),
                    reason: row.reason.clone(),
                    shop_id: shop_id.unwrap_or(0),
                }));
            }
        }
        Ok(None)
    }

    /// 守卫版：命中即返回业务错误（文案按 §7 定稿，用户端不出现「黑名单」）。
    pub async fn ensure_allowed(
        &self,
        user_id: i32,
        product: &str,
        behavior: &str,
        shop_id: Option<i32>,
    ) -> Result<()> {
        if let Some(hit) = self.find_restriction(user_id, product, behavior, shop_id).await? {
            return Err(Error::biz(1001, user_facing_message(behavior, &hit)));
        }
        Ok(())
    }

    /// 下单专用批量判定：lines 为 (product_id, shop_id) 列表（shop_id 取商品
    /// 归属），按店铺逐个判定，返回全部命中（一条 per 命中店铺）。
    pub async fn find_order_restrictions(
        &self,
        user_id: i32,
        lines: &[(i32, i32)],
    ) -> Result<Vec<RestrictionHit>> {
        let mut shops: Vec<i32> = Vec::new();
        for (_, shop_id) in lines {
            if !shops.contains(shop_id) {
                shops.push(*shop_id);
            }
        }
        let mut hits = Vec::new();
        for shop_id in shops {
            if let Some(hit) = self
                .find_restriction(user_id, PRODUCT_MALL, BEHAVIOR_PLACE_ORDER, Some(shop_id))
                .await?
            {
                hits.push(hit);
            }
        }
        Ok(hits)
    }

    /// 佣金发放专用判定（§4.2/§4.5）：限制对象是**收佣人** user_id；
    /// 店铺上下文按 [`commission_shop_context`] 定死——分销员店铺优先、
    /// 订单店铺兜底。命中即应跳过入账且不置已结算（自动路径）或整批
    /// 拒绝（人工路径），由调用方按各自口径处理。
    pub async fn find_commission_restriction(
        &self,
        user_id: i32,
        salesman_shop_id: Option<i32>,
        order_shop_id: Option<i32>,
    ) -> Result<Option<RestrictionHit>> {
        let shop_id = commission_shop_context(salesman_shop_id, order_shop_id);
        self.find_restriction(user_id, "distribution", BEHAVIOR_COMMISSION_GRANT, shop_id)
            .await
    }

    /// 批量取店铺名（下单拦截组装文案用）。
    pub async fn shop_titles(&self, shop_ids: &[i32]) -> Result<HashMap<i32, String>> {
        Ok(self.repo().shop_titles(shop_ids).await?.into_iter().collect())
    }

    // ————————————————————— 管理侧：列表 / 详情 —————————————————————

    /// 条目列表（店铺操作人只看本店条目 + 平台全平台条目只读展示，§4.4）。
    pub async fn list(
        &self,
        q: &BlacklistListQuery,
        op: &BlacklistOperator,
    ) -> Result<PageResult<BlacklistEntryVO>> {
        let (limit, offset) = q.limit_offset();
        let filter = entry_filter(q, op);
        let (total, entries) = self.repo().paginate_entries(&filter, limit, offset).await?;
        let vos = self.assemble_vos(&entries, op).await?;
        let page = nextrade_core::types::PageQuery {
            page: u64::try_from(q.page.unwrap_or(1).max(1)).unwrap_or(1),
            page_size: u64::try_from(limit).unwrap_or(15),
        };
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), vos, &page))
    }

    /// 条目详情（平台可看全部条目含店铺条目，待拍板默认口径；店铺只看可见范围）。
    pub async fn detail(&self, id: i64, op: &BlacklistOperator) -> Result<BlacklistEntryVO> {
        let entry = self
            .repo()
            .find_entry(id)
            .await?
            .ok_or_else(|| Error::biz(1001, MSG_ENTRY_NOT_EXISTS))?;
        if !op.is_platform && !entry_visible_to_shop(&entry, op.shop_id) {
            return Err(Error::Forbidden("无权查看该黑名单条目".into()));
        }
        let vos = self.assemble_vos(std::slice::from_ref(&entry), op).await?;
        vos.into_iter()
            .next()
            .ok_or_else(|| Error::biz(1001, MSG_ENTRY_NOT_EXISTS))
    }

    /// 操作记录（详情抽屉时间线与独立查询共用）。
    pub async fn log_list(
        &self,
        q: &BlacklistLogQuery,
        op: &BlacklistOperator,
    ) -> Result<PageResult<BlacklistLogVO>> {
        let (limit, offset) = q.limit_offset();
        let filter = BlacklistLogFilter {
            user_id: q.user_id,
            blacklist_id: q.blacklist_id,
            op_type: q.op_type,
            create_time_start: q.create_time_start,
            create_time_end: q.create_time_end,
            visible_shop: if op.is_platform { None } else { Some(op.shop_id) },
        };
        let (total, rows) = self.repo().paginate_logs(&filter, limit, offset).await?;
        let list: Vec<BlacklistLogVO> = rows
            .into_iter()
            .map(|r| BlacklistLogVO {
                id: r.id,
                blacklist_id: r.blacklist_id,
                user_id: r.user_id,
                op_type: i32::from(r.op_type),
                op_content: r.op_content,
                rules_snapshot: r.rules_snapshot,
                scope_snapshot: r.scope_snapshot,
                operator_id: r.operator_id,
                operator_name: r.operator_name,
                operator_type: r.operator_type,
                operator_shop_id: r.operator_shop_id,
                create_time: r.create_time,
            })
            .collect();
        let page = nextrade_core::types::PageQuery {
            page: u64::try_from(q.page.unwrap_or(1).max(1)).unwrap_or(1),
            page_size: u64::try_from(limit).unwrap_or(15),
        };
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    // ————————————————————— 管理侧：加入 / 合并 —————————————————————

    /// 单个加入（重复加入 = 合并规则，§4.4）。
    pub async fn add(
        &self,
        param: &BlacklistAddParam,
        op: &BlacklistOperator,
    ) -> Result<BlacklistAddResultVO> {
        self.ensure_shop_can_write(op).await?;
        let user_id = self.resolve_user_id(param).await?;
        let batch = matches!(param.source, Some(s) if s == SOURCE_PAGE_BATCH || s == SOURCE_MEMBER_BATCH);
        let log_op = if batch { OP_BATCH_ADD } else { OP_ADD };
        self.add_for_user(user_id, param, op, log_op).await
    }

    /// 批量加入：逐个合并语义，部分失败计入 failures 不整批失败。
    pub async fn batch_add(
        &self,
        param: &BlacklistBatchAddParam,
        op: &BlacklistOperator,
    ) -> Result<BlacklistBatchAddResultVO> {
        self.ensure_shop_can_write(op).await?;
        validate_reason(&param.reason)?;
        validate_rules(&param.rules)?;
        let single = BlacklistAddParam {
            user_id: None,
            mobile: None,
            username: None,
            rules: param.rules.clone(),
            apply_mode: param.apply_mode,
            shop_ids: param.shop_ids.clone(),
            reason: param.reason.clone(),
            source: param.source.or(Some(SOURCE_PAGE_BATCH)),
        };
        let mut result = BlacklistBatchAddResultVO::default();
        for user_id in &param.user_ids {
            match self.add_for_user(*user_id, &single, op, OP_BATCH_ADD).await {
                Ok(r) => {
                    result.success_count += 1;
                    if r.merged {
                        result.merged_count += 1;
                    }
                }
                Err(e) => result.failures.push(BlacklistFailureVO {
                    target_id: i64::from(*user_id),
                    msg: e.to_string(),
                }),
            }
        }
        Ok(result)
    }

    /// 加入核心：归属校验 → 已有生效条目则合并，否则新建。
    /// `log_op` 为新建条目时的流水类型（单个 OP_ADD / 批量 OP_BATCH_ADD /
    /// 导入 OP_IMPORT_ADD；合并恒走 OP_MERGE）。
    async fn add_for_user(
        &self,
        user_id: i32,
        param: &BlacklistAddParam,
        op: &BlacklistOperator,
        log_op: i32,
    ) -> Result<BlacklistAddResultVO> {
        validate_reason(&param.reason)?;
        let rules = validate_rules(&param.rules)?;
        let user = self
            .repo()
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::biz(1001, MSG_USER_NOT_EXISTS))?;

        let owner_type = op.owner_type();
        let owner_shop_id = op.owner_shop_id();
        let (apply_mode, scope_shops) = if op.is_platform {
            let mode = param.apply_mode.unwrap_or(APPLY_ALL);
            if mode == APPLY_SELECTED {
                if param.shop_ids.is_empty() {
                    return Err(Error::Validation(MSG_SCOPE_SHOPS_REQUIRED.into()));
                }
                for shop_id in &param.shop_ids {
                    if !self.repo().shop_exists(*shop_id).await? {
                        return Err(Error::biz(1001, format!("店铺 {shop_id} 不存在")));
                    }
                }
                (APPLY_SELECTED, param.shop_ids.clone())
            } else {
                (APPLY_ALL, Vec::new())
            }
        } else {
            // 店铺条目：范围恒本店；目标必须是本店客户（防滥拉黑）。
            let in_shop = self
                .repos
                .member
                .user_manage
                .user_in_shop(user_id, op.shop_id)
                .await?;
            if !in_shop {
                return Err(Error::biz(1001, MSG_NOT_SHOP_MEMBER));
            }
            (APPLY_ALL, Vec::new())
        };

        let now = now_seconds();
        let source = param.source.unwrap_or(SOURCE_MANUAL);
        if let Some(existing) = self
            .repo()
            .find_active_entry(user_id, owner_type, owner_shop_id)
            .await?
        {
            // 合并：规则并入既有条目，原因以新原因为准，范围按本次入参覆盖。
            let existing_rules = self
                .repo()
                .rules_for(&[existing.id])
                .await?
                .into_iter()
                .map(|r| (r.product_code, r.behavior_code))
                .collect::<Vec<_>>();
            let (merged_rules, _added) = merge_rule_keys(&existing_rules, &rules);
            let new_rules: Vec<(String, String)> = merged_rules
                .iter()
                .filter(|r| !existing_rules.contains(r))
                .cloned()
                .collect();
            self.repo().insert_rules(existing.id, &new_rules, now).await?;
            self.repo()
                .update_entry_after_merge(existing.id, param.reason.trim(), apply_mode, now)
                .await?;
            if op.is_platform {
                self.repo().replace_scope(existing.id, &scope_shops).await?;
            }
            self.repo()
                .insert_log(
                    existing.id,
                    user_id,
                    OP_MERGE,
                    param.reason.trim(),
                    &rules_snapshot(&merged_rules),
                    &scope_snapshot(op, apply_mode, &scope_shops),
                    op,
                    &current_ip(),
                    now,
                )
                .await?;
            self.write_admin_log(
                op,
                &format!(
                    "更新客户 {} 的黑名单规则（{}）：{}",
                    mask_mobile(&user.mobile),
                    owner_label(owner_type),
                    rules_label(&merged_rules)
                ),
            )
            .await;
            return Ok(BlacklistAddResultVO {
                id: existing.id,
                merged: true,
            });
        }

        let id = self
            .repo()
            .insert_entry(
                user_id,
                owner_type,
                owner_shop_id,
                apply_mode,
                param.reason.trim(),
                source,
                op.admin_id,
                &op.admin_name,
                &op.admin_type,
                now,
            )
            .await?;
        self.repo().insert_rules(id, &rules, now).await?;
        if op.is_platform && apply_mode == APPLY_SELECTED {
            self.repo().replace_scope(id, &scope_shops).await?;
        }
        self.repo()
            .insert_log(
                id,
                user_id,
                log_op,
                param.reason.trim(),
                &rules_snapshot(&rules),
                &scope_snapshot(op, apply_mode, &scope_shops),
                op,
                &current_ip(),
                now,
            )
            .await?;
        self.write_admin_log(
            op,
            &format!(
                "将客户 {} 加入黑名单（{}）：{}",
                mask_mobile(&user.mobile),
                owner_label(owner_type),
                rules_label(&rules)
            ),
        )
        .await;
        Ok(BlacklistAddResultVO { id, merged: false })
    }

    // ————————————————————— 管理侧：移除 —————————————————————

    /// 单个移除（只能移除自己归属的条目，§4.4）。
    pub async fn remove(
        &self,
        param: &BlacklistRemoveParam,
        op: &BlacklistOperator,
    ) -> Result<()> {
        self.remove_one(param.id, param.remove_reason.as_deref(), op, false)
            .await
    }

    /// 批量移除：含他人归属条目时该条计入 failures，不整批失败。
    pub async fn batch_remove(
        &self,
        param: &BlacklistBatchRemoveParam,
        op: &BlacklistOperator,
    ) -> Result<BlacklistBatchRemoveResultVO> {
        let mut result = BlacklistBatchRemoveResultVO::default();
        for id in &param.ids {
            match self
                .remove_one(*id, param.remove_reason.as_deref(), op, true)
                .await
            {
                Ok(()) => result.success_count += 1,
                Err(e) => result.failures.push(BlacklistFailureVO {
                    target_id: *id,
                    msg: e.to_string(),
                }),
            }
        }
        Ok(result)
    }

    async fn remove_one(
        &self,
        id: i64,
        remove_reason: Option<&str>,
        op: &BlacklistOperator,
        batch: bool,
    ) -> Result<()> {
        let entry = self
            .repo()
            .find_entry(id)
            .await?
            .ok_or_else(|| Error::biz(1001, MSG_ENTRY_NOT_EXISTS))?;
        // 归属隔离：平台只移除平台条目，店铺只移除本店条目。
        if op.is_platform {
            if i32::from(entry.owner_type) != OWNER_PLATFORM {
                return Err(Error::Forbidden(MSG_PLATFORM_CANNOT_REMOVE_SHOP.into()));
            }
        } else if i32::from(entry.owner_type) != OWNER_SHOP || entry.owner_shop_id != op.shop_id {
            return Err(Error::Forbidden(MSG_SHOP_CANNOT_REMOVE_OTHERS.into()));
        }
        if i32::from(entry.status) != STATUS_ACTIVE {
            return Err(Error::biz(1001, MSG_ENTRY_ALREADY_REMOVED));
        }
        let now = now_seconds();
        let reason = remove_reason.unwrap_or_default().trim();
        let affected = self
            .repo()
            .mark_removed(id, op.admin_id, &op.admin_name, reason, now)
            .await?;
        if affected == 0 {
            return Err(Error::biz(1001, MSG_ENTRY_ALREADY_REMOVED));
        }
        let rules = self
            .repo()
            .rules_for(&[id])
            .await?
            .into_iter()
            .map(|r| (r.product_code, r.behavior_code))
            .collect::<Vec<_>>();
        let scope_shops: Vec<i32> = self
            .repo()
            .scopes_for(&[id])
            .await?
            .into_iter()
            .map(|(_, sid)| sid)
            .collect();
        self.repo()
            .insert_log(
                id,
                entry.user_id,
                if batch { OP_BATCH_REMOVE } else { OP_REMOVE },
                reason,
                &rules_snapshot(&rules),
                &scope_snapshot(op, i32::from(entry.apply_mode), &scope_shops),
                op,
                &current_ip(),
                now,
            )
            .await?;
        self.write_admin_log(
            op,
            &format!(
                "将客户（userId={}）移出黑名单（{}）",
                entry.user_id,
                owner_label(i32::from(entry.owner_type))
            ),
        )
        .await;
        Ok(())
    }

    // ————————————————————— 管理侧：导入 / 导出（Phase 3） —————————————————————

    /// 批量导入（§5.2）：`sheet` 为 `read_xlsx` 原始行（含表头行）。
    /// 部分失败不以 Err 表达，逐行原因进 `errors`（§5.1）；开关未开等
    /// 整批性问题才走 Err。落库按人归并、走合并语义，流水 op=导入加入。
    pub async fn import_rows(
        &self,
        sheet: &[Vec<String>],
        op: &BlacklistOperator,
    ) -> Result<BlacklistImportResultVO> {
        self.ensure_shop_can_write(op).await?;
        let (groups, mut errors, total) = self.parse_import_sheet(sheet, op).await?;
        let mut result = BlacklistImportResultVO {
            total,
            ..Default::default()
        };
        for group in &groups {
            let param = BlacklistAddParam {
                user_id: Some(group.user_id),
                mobile: None,
                username: None,
                rules: group
                    .rules
                    .iter()
                    .map(|(p, b)| BlacklistRuleDTO {
                        product: p.clone(),
                        behavior: b.clone(),
                    })
                    .collect(),
                apply_mode: if op.is_platform {
                    Some(if group.shop_ids.is_empty() {
                        APPLY_ALL
                    } else {
                        APPLY_SELECTED
                    })
                } else {
                    None
                },
                shop_ids: group.shop_ids.clone(),
                reason: group.reason.clone(),
                source: Some(SOURCE_IMPORT),
            };
            match self.add_for_user(group.user_id, &param, op, OP_IMPORT_ADD).await {
                Ok(r) => {
                    result.success_count += i32::try_from(group.rows.len()).unwrap_or(0);
                    if r.merged {
                        result.merged_count += 1;
                    }
                }
                Err(e) => {
                    // 归属类失败（店铺导入非本店客户等）：该会员的全部行计失败。
                    for row in &group.rows {
                        let mut err = row.raw.clone();
                        err.error_msg = e.to_string();
                        errors.push(err);
                    }
                }
            }
        }
        errors.sort_by_key(|e| e.row);
        result.fail_count = i32::try_from(errors.len()).unwrap_or(0);
        result.errors = errors;
        Ok(result)
    }

    /// 失败明细数据源（§5.1 定死端点版）：重传原文件 → 只校验不落库 →
    /// 返回全部失败行，路由层组 xlsx 下载。
    pub async fn import_error_rows(
        &self,
        sheet: &[Vec<String>],
        op: &BlacklistOperator,
    ) -> Result<Vec<BlacklistImportErrorVO>> {
        let (_groups, errors, _total) = self.parse_import_sheet(sheet, op).await?;
        Ok(errors)
    }

    /// 导入解析：表头校验 → 逐行校验（会员匹配/规则/店铺/原因）→ 按人归并。
    /// 返回 (归并组, 失败行, 有效数据行数)；表头错版/超行数上限走 Err 整批拒绝。
    async fn parse_import_sheet(
        &self,
        sheet: &[Vec<String>],
        op: &BlacklistOperator,
    ) -> Result<(Vec<GroupedImportUser>, Vec<BlacklistImportErrorVO>, i32)> {
        let header = sheet
            .first()
            .ok_or_else(|| Error::Validation("表格数据为空".into()))?;
        let header_ok = IMPORT_HEADERS
            .iter()
            .enumerate()
            .all(|(i, name)| header.get(i).map(|c| c.trim()) == Some(*name));
        if !header_ok {
            return Err(Error::Validation(
                "导入模板表头不正确，请使用最新模板重新填写".into(),
            ));
        }
        let mut parsed: Vec<ParsedImportRow> = Vec::new();
        let mut errors: Vec<BlacklistImportErrorVO> = Vec::new();
        let mut total = 0i32;
        for (idx, raw_row) in sheet.iter().enumerate().skip(1) {
            if raw_row.iter().all(|c| c.trim().is_empty()) {
                continue; // 全空行不计入总数
            }
            total += 1;
            if total > i32::try_from(IMPORT_MAX_ROWS).unwrap_or(1000) {
                return Err(Error::Validation(format!(
                    "单次导入不能超过 {IMPORT_MAX_ROWS} 行"
                )));
            }
            let row_no = i64::try_from(idx + 1).unwrap_or(0);
            match self.parse_one_import_row(raw_row, row_no, op).await? {
                Ok(row) => parsed.push(row),
                Err(err) => errors.push(err),
            }
        }
        Ok((group_import_rows(parsed), errors, total))
    }

    /// 单行校验（会员三标识匹配在 core 纯函数 `match_import_user` 定死口径）。
    async fn parse_one_import_row(
        &self,
        raw_row: &[String],
        row_no: i64,
        op: &BlacklistOperator,
    ) -> Result<std::result::Result<ParsedImportRow, BlacklistImportErrorVO>> {
        let cell = |i: usize| -> String {
            raw_row.get(i).map(|s| s.trim().to_string()).unwrap_or_default()
        };
        let raw = BlacklistImportErrorVO {
            row: row_no,
            user_id: cell(0),
            mobile: cell(1),
            username: cell(2),
            product: cell(3),
            behavior: cell(4),
            shop_ids: cell(5),
            error_msg: String::new(),
        };
        let fail = |msg: String| {
            let mut e = raw.clone();
            e.error_msg = msg;
            std::result::Result::Err(e)
        };

        // 会员定位：ID > 手机号 > 用户名（精确匹配，三者须指向同一人）。
        let user_id = match self.resolve_import_user_id(&raw).await? {
            Ok(id) => id,
            Err(e) => return Ok(Err(e)),
        };

        // 规则：产品/行为中文名或码双认 + 组合白名单。
        let Some(product) = parse_product_code(&raw.product) else {
            return Ok(fail("限制产品不合法".to_string()));
        };
        let Some(behavior) = parse_behavior_code(&raw.behavior) else {
            return Ok(fail("限制行为不合法".to_string()));
        };
        if !is_valid_combo(product, behavior) {
            return Ok(fail(format!(
                "限制规则不合法：{}-{}",
                product_name(product),
                behavior_name(behavior)
            )));
        }

        // 生效店铺：仅平台 selected 语义使用；店铺导入忽略此列（恒本店）。
        let shop_ids = match self.resolve_import_shop_ids(&raw, op).await? {
            Ok(ids) => ids,
            Err(e) => return Ok(Err(e)),
        };

        let reason = cell(6);
        if reason.is_empty() {
            return Ok(fail(MSG_REASON_REQUIRED.to_string()));
        }
        if reason.chars().count() > 500 {
            return Ok(fail("限制原因不能超过 500 字".to_string()));
        }

        Ok(Ok(ParsedImportRow {
            raw,
            user_id,
            rule: (product.to_string(), behavior.to_string()),
            shop_ids,
            reason,
        }))
    }

    /// 导入行会员定位：三标识分别精确查人后交 core 纯函数按优先级/一致性裁决。
    async fn resolve_import_user_id(
        &self,
        raw: &BlacklistImportErrorVO,
    ) -> Result<std::result::Result<i32, BlacklistImportErrorVO>> {
        let fail = |msg: &str| {
            let mut e = raw.clone();
            e.error_msg = msg.to_string();
            std::result::Result::Err(e)
        };
        let by_id = if raw.user_id.is_empty() {
            None
        } else {
            match raw.user_id.parse::<i32>() {
                Ok(id) => Some(self.repo().find_user_by_id(id).await?.map(|u| u.user_id)),
                Err(_) => return Ok(fail("会员ID格式不正确")),
            }
        };
        let by_mobile = if raw.mobile.is_empty() {
            None
        } else {
            Some(
                self.repo()
                    .find_user_by_mobile(&raw.mobile)
                    .await?
                    .map(|u| u.user_id),
            )
        };
        let by_username = if raw.username.is_empty() {
            None
        } else {
            Some(
                self.repo()
                    .find_user_by_username(&raw.username)
                    .await?
                    .map(|u| u.user_id),
            )
        };
        Ok(match match_import_user(by_id, by_mobile, by_username) {
            Ok(id) => Ok(id),
            Err(msg) => fail(msg),
        })
    }

    /// 导入行生效店铺解析 + 存在性校验（仅平台；店铺导入恒本店、忽略此列）。
    async fn resolve_import_shop_ids(
        &self,
        raw: &BlacklistImportErrorVO,
        op: &BlacklistOperator,
    ) -> Result<std::result::Result<Vec<i32>, BlacklistImportErrorVO>> {
        if !op.is_platform {
            return Ok(Ok(Vec::new()));
        }
        let fail = |msg: String| {
            let mut e = raw.clone();
            e.error_msg = msg;
            std::result::Result::Err(e)
        };
        let shop_ids = match parse_import_shop_ids(&raw.shop_ids) {
            Ok(ids) => ids,
            Err(msg) => return Ok(fail(msg.to_string())),
        };
        for shop_id in &shop_ids {
            if !self.repo().shop_exists(*shop_id).await? {
                return Ok(fail(format!("店铺 {shop_id} 不存在")));
            }
        }
        Ok(Ok(shop_ids))
    }

    /// 导出（§5.1）：与列表同一筛选口径全量查（无分页），VO 装配与列表一致，
    /// xlsx 组装在路由层。
    pub async fn export_entries(
        &self,
        q: &BlacklistListQuery,
        op: &BlacklistOperator,
    ) -> Result<Vec<BlacklistEntryVO>> {
        let filter = entry_filter(q, op);
        let entries = self.repo().list_all_entries(&filter).await?;
        self.assemble_vos(&entries, op).await
    }

    // ————————————————————— 内部 —————————————————————

    /// 店铺操作人写前置：开关未开直接 403（平台不受影响）。
    async fn ensure_shop_can_write(&self, op: &BlacklistOperator) -> Result<()> {
        if !op.is_platform && !self.shop_allow().await? {
            return Err(Error::Forbidden(MSG_SHOP_BLACKLIST_DISABLED.into()));
        }
        Ok(())
    }

    /// 加入入参的会员定位：ID > 手机号 > 用户名（三选一，精确匹配，§5.1）。
    async fn resolve_user_id(&self, param: &BlacklistAddParam) -> Result<i32> {
        if let Some(user_id) = param.user_id {
            if self.repo().find_user_by_id(user_id).await?.is_some() {
                return Ok(user_id);
            }
            return Err(Error::biz(1001, MSG_USER_NOT_EXISTS));
        }
        if let Some(mobile) = param.mobile.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(u) = self.repo().find_user_by_mobile(mobile).await? {
                return Ok(u.user_id);
            }
            return Err(Error::biz(1001, MSG_USER_NOT_EXISTS));
        }
        if let Some(name) = param.username.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(u) = self.repo().find_user_by_username(name).await? {
                return Ok(u.user_id);
            }
            return Err(Error::biz(1001, MSG_USER_NOT_EXISTS));
        }
        Err(Error::Validation("请提供会员ID、手机号或用户名".into()))
    }

    /// 条目行 → VO 批量装配（规则/范围/会员/店铺名一次批量回填，禁止逐行查）。
    async fn assemble_vos(
        &self,
        entries: &[BlacklistEntry],
        op: &BlacklistOperator,
    ) -> Result<Vec<BlacklistEntryVO>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        let mut rules_map: HashMap<i64, Vec<BlacklistRuleDTO>> = HashMap::new();
        for r in self.repo().rules_for(&ids).await? {
            rules_map.entry(r.blacklist_id).or_default().push(BlacklistRuleDTO {
                product: r.product_code,
                behavior: r.behavior_code,
            });
        }
        let mut scope_map: HashMap<i64, Vec<i32>> = HashMap::new();
        for (bid, sid) in self.repo().scopes_for(&ids).await? {
            scope_map.entry(bid).or_default().push(sid);
        }
        let user_ids: Vec<i32> = entries.iter().map(|e| e.user_id).collect();
        let users = self
            .repos
            .member
            .shop_member
            .find_users_by_ids(&user_ids)
            .await?;
        let mut user_map: HashMap<i32, BlacklistUserVO> = users
            .into_iter()
            .map(|u| {
                (
                    u.user_id,
                    BlacklistUserVO {
                        user_id: u.user_id,
                        username: Some(u.username),
                        nickname: u.nickname,
                        mobile: Some(u.mobile),
                        avatar: Some(u.avatar),
                    },
                )
            })
            .collect();
        // 店铺视角脱敏：口径同 shop_member should_mask（平台不脱敏）。
        if !op.is_platform {
            let enabled = self.read_config_int(CONFIG_ENABLED).await?;
            let desensitize = self.read_config_int(CONFIG_DESENSITIZE).await?;
            if should_mask(op.shop_id, enabled == 1, desensitize == 1) {
                for u in user_map.values_mut() {
                    if let Some(mobile) = &u.mobile {
                        u.mobile = Some(mask_mobile(mobile));
                    }
                    if let Some(nickname) = &u.nickname {
                        u.nickname = Some(mask_name(nickname));
                    }
                }
            }
        }
        let mut shop_ids_all: Vec<i32> = entries.iter().map(|e| e.owner_shop_id).collect();
        for v in scope_map.values() {
            shop_ids_all.extend(v.iter().copied());
        }
        let titles = self.shop_titles(&shop_ids_all).await?;
        Ok(entries
            .iter()
            .map(|e| {
                let shop_ids = scope_map.get(&e.id).cloned().unwrap_or_default();
                BlacklistEntryVO {
                    id: e.id,
                    user_id: e.user_id,
                    user: user_map.get(&e.user_id).cloned(),
                    owner_type: i32::from(e.owner_type),
                    owner_shop_id: e.owner_shop_id,
                    owner_shop_name: if i32::from(e.owner_type) == OWNER_SHOP {
                        titles.get(&e.owner_shop_id).cloned()
                    } else {
                        None
                    },
                    apply_mode: i32::from(e.apply_mode),
                    shop_names: shop_ids
                        .iter()
                        .filter_map(|sid| titles.get(sid).cloned())
                        .collect(),
                    shop_ids,
                    rules: rules_map.get(&e.id).cloned().unwrap_or_default(),
                    reason: e.reason.clone(),
                    source: i32::from(e.source),
                    status: i32::from(e.status),
                    create_admin_id: e.create_admin_id,
                    create_admin_name: e.create_admin_name.clone(),
                    create_admin_type: e.create_admin_type.clone(),
                    create_time: e.create_time,
                    update_time: e.update_time,
                    remove_admin_id: e.remove_admin_id,
                    remove_admin_name: e.remove_admin_name.clone(),
                    remove_time: e.remove_time,
                    remove_reason: e.remove_reason.clone(),
                    can_remove: can_remove(e, op),
                }
            })
            .collect())
    }

    async fn read_config_int(&self, biz_code: &str) -> Result<i32> {
        let raw = self.repos.member.shop_member.config_value(biz_code).await?;
        Ok(config_int_or(raw.as_deref(), 0))
    }

    /// 双写 admin_log（失败只 warn，不阻断主流程）。
    async fn write_admin_log(&self, op: &BlacklistOperator, info: &str) {
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let (scope_type, scope_id) = if op.is_platform {
            ("admin", 0)
        } else {
            ("shop", op.shop_id)
        };
        if let Err(e) = admin_log.record_current_log(scope_type, scope_id, info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
    }
}

/// 列表/导出共用过滤构造（同一筛选口径，防两处漂移，§8 Phase 3）。
fn entry_filter(q: &BlacklistListQuery, op: &BlacklistOperator) -> BlacklistListFilter {
    BlacklistListFilter {
        keyword: q.effective_keyword().map(ToString::to_string),
        user_id: q.user_id,
        product_code: q.product_code.clone().filter(|s| !s.is_empty()),
        behavior_code: q.behavior_code.clone().filter(|s| !s.is_empty()),
        owner_type: q.owner_type,
        shop_id: q.shop_id,
        status: q.status,
        create_time_start: q.create_time_start,
        create_time_end: q.create_time_end,
        visible_shop: if op.is_platform { None } else { Some(op.shop_id) },
    }
}

/// 已通过单行校验的导入行（§5.2：一行 = 一个 (产品,行为) 组合）。
#[derive(Debug, Clone)]
struct ParsedImportRow {
    /// 原始单元格快照（失败回填错误信息用）。
    raw: BlacklistImportErrorVO,
    user_id: i32,
    rule: (String, String),
    shop_ids: Vec<i32>,
    reason: String,
}

/// 按人归并后的导入组（同人多行合并为一条条目，§5.2）。
#[derive(Debug, Clone)]
struct GroupedImportUser {
    user_id: i32,
    /// 规则并集（保持首次出现顺序、去重）。
    rules: Vec<(String, String)>,
    /// 生效店铺并集（平台 selected 语义）。
    shop_ids: Vec<i32>,
    /// 原因取该会员首行。
    reason: String,
    rows: Vec<ParsedImportRow>,
}

/// 导入行按人归并（纯逻辑）：会员首次出现顺序即组顺序。
fn group_import_rows(rows: Vec<ParsedImportRow>) -> Vec<GroupedImportUser> {
    let mut groups: Vec<GroupedImportUser> = Vec::new();
    for row in rows {
        if let Some(group) = groups.iter_mut().find(|g| g.user_id == row.user_id) {
            if !group.rules.contains(&row.rule) {
                group.rules.push(row.rule.clone());
            }
            for shop_id in &row.shop_ids {
                if !group.shop_ids.contains(shop_id) {
                    group.shop_ids.push(*shop_id);
                }
            }
            group.rows.push(row);
        } else {
            groups.push(GroupedImportUser {
                user_id: row.user_id,
                rules: vec![row.rule.clone()],
                shop_ids: row.shop_ids.clone(),
                reason: row.reason.clone(),
                rows: vec![row],
            });
        }
    }
    groups
}

/// 条目对店铺操作人的可见性：本店条目或平台全平台条目。
fn entry_visible_to_shop(entry: &BlacklistEntry, shop_id: i32) -> bool {
    (i32::from(entry.owner_type) == OWNER_SHOP && entry.owner_shop_id == shop_id)
        || (i32::from(entry.owner_type) == OWNER_PLATFORM
            && i32::from(entry.apply_mode) == APPLY_ALL)
}

/// 当前操作人是否可移除该条目（与 remove_one 的服务层校验同口径）。
fn can_remove(entry: &BlacklistEntry, op: &BlacklistOperator) -> bool {
    if i32::from(entry.status) != STATUS_ACTIVE {
        return false;
    }
    if op.is_platform {
        i32::from(entry.owner_type) == OWNER_PLATFORM
    } else {
        i32::from(entry.owner_type) == OWNER_SHOP && entry.owner_shop_id == op.shop_id
    }
}

/// 原因校验：必填、≤500 字。
fn validate_reason(reason: &str) -> Result<()> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(Error::Validation(MSG_REASON_REQUIRED.into()));
    }
    if reason.chars().count() > 500 {
        return Err(Error::Validation("限制原因不能超过 500 字".into()));
    }
    Ok(())
}

/// 规则校验：非空、组合合法（§3.2 白名单六组合，Phase 2 起全量开放）、去重。
fn validate_rules(rules: &[BlacklistRuleDTO]) -> Result<Vec<(String, String)>> {
    if rules.is_empty() {
        return Err(Error::Validation(MSG_RULES_REQUIRED.into()));
    }
    let mut out: Vec<(String, String)> = Vec::new();
    for r in rules {
        if !is_valid_combo(&r.product, &r.behavior) {
            return Err(Error::Validation(format!(
                "限制规则不合法：{}-{}",
                product_name(&r.product),
                behavior_name(&r.behavior)
            )));
        }
        let key = (r.product.clone(), r.behavior.clone());
        if !out.contains(&key) {
            out.push(key);
        }
    }
    Ok(out)
}

/// 用户端拦截文案（§7 定稿，不出现「黑名单」字样、不暴露原因）。
#[must_use]
pub fn user_facing_message(behavior: &str, hit: &RestrictionHit) -> String {
    match behavior {
        BEHAVIOR_PLACE_ORDER => {
            if hit.owner_type == OWNER_SHOP {
                "该商品所在店铺暂时无法为您提供下单服务，如有疑问请联系客服".to_string()
            } else {
                "您的账号暂时无法下单，如有疑问请联系客服".to_string()
            }
        }
        BEHAVIOR_PRODUCT_COMMENT => "您的账号暂时无法发表评价，如有疑问请联系客服".to_string(),
        BEHAVIOR_BUYER_SHOW_PUBLISH => "您的账号暂时无法发布买家秀，如有疑问请联系客服".to_string(),
        BEHAVIOR_BUYER_SHOW_COMMENT => "您的账号暂时无法评论，如有疑问请联系客服".to_string(),
        BEHAVIOR_BECOME_SALESMAN => "您的账号暂时无法成为分销员，如有疑问请联系客服".to_string(),
        _ => "您的账号暂时无法进行此操作，如有疑问请联系客服".to_string(),
    }
}

/// 归属中文标签（admin_log 文本用）。
fn owner_label(owner_type: i32) -> &'static str {
    if owner_type == OWNER_PLATFORM {
        "平台"
    } else {
        "店铺"
    }
}

/// 规则中文标签串（admin_log 文本用），如「商城-下单、商城-评价」。
fn rules_label(rules: &[(String, String)]) -> String {
    rules
        .iter()
        .map(|(p, b)| format!("{}-{}", product_name(p), behavior_name(b)))
        .collect::<Vec<_>>()
        .join("、")
}

/// 规则快照 JSON：[{product, behavior}]。
fn rules_snapshot(rules: &[(String, String)]) -> serde_json::Value {
    serde_json::Value::Array(
        rules
            .iter()
            .map(|(p, b)| serde_json::json!({ "product": p, "behavior": b }))
            .collect(),
    )
}

/// 范围快照 JSON：{ownerType, ownerShopId, applyMode, shopIds}。
fn scope_snapshot(op: &BlacklistOperator, apply_mode: i32, shop_ids: &[i32]) -> serde_json::Value {
    serde_json::json!({
        "ownerType": op.owner_type(),
        "ownerShopId": op.owner_shop_id(),
        "applyMode": apply_mode,
        "shopIds": shop_ids,
    })
}

/// 当前请求 IP（审计上下文；不在请求内时为空串）。
fn current_ip() -> String {
    nextrade_security::audit::current().map(|a| a.ip).unwrap_or_default()
}

/// 候选叠加的纯逻辑复核入口（供集成自测引用核心纯函数）。
#[must_use]
pub fn overlay_matches(
    candidates: &[RestrictionCandidate],
    shop_id: Option<i32>,
    shop_enabled: bool,
) -> bool {
    any_candidate_matches(candidates, shop_id, shop_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(owner_type: i32) -> RestrictionHit {
        RestrictionHit {
            blacklist_id: 1,
            owner_type,
            owner_shop_id: 0,
            product_code: PRODUCT_MALL.to_string(),
            behavior_code: BEHAVIOR_PLACE_ORDER.to_string(),
            reason: "测试".to_string(),
            shop_id: 0,
        }
    }

    #[test]
    fn user_facing_messages_follow_copy_rules() {
        let shop_msg = user_facing_message(BEHAVIOR_PLACE_ORDER, &hit(OWNER_SHOP));
        assert_eq!(shop_msg, "该商品所在店铺暂时无法为您提供下单服务，如有疑问请联系客服");
        let platform_msg = user_facing_message(BEHAVIOR_PLACE_ORDER, &hit(OWNER_PLATFORM));
        assert_eq!(platform_msg, "您的账号暂时无法下单，如有疑问请联系客服");
        let comment_msg = user_facing_message(BEHAVIOR_PRODUCT_COMMENT, &hit(OWNER_PLATFORM));
        assert_eq!(comment_msg, "您的账号暂时无法发表评价，如有疑问请联系客服");
        for msg in [shop_msg, platform_msg, comment_msg] {
            assert!(!msg.contains("黑名单"), "用户端文案不得出现「黑名单」: {msg}");
        }
    }

    #[test]
    fn validate_rules_rejects_bad_combo_and_accepts_all_six() {
        assert!(validate_rules(&[]).is_err());
        let bad = vec![BlacklistRuleDTO {
            product: "mall".into(),
            behavior: "become_salesman".into(),
        }];
        assert!(validate_rules(&bad).is_err(), "非法组合应拒绝");
        // Phase 2 起六个组合全量开放（分销/买家秀不再被门控拒绝）。
        for (product, behavior) in [
            ("mall", "place_order"),
            ("mall", "product_comment"),
            ("distribution", "become_salesman"),
            ("distribution", "commission_grant"),
            ("buyer_show", "buyer_show_publish"),
            ("buyer_show", "buyer_show_comment"),
        ] {
            let rules = vec![BlacklistRuleDTO {
                product: product.into(),
                behavior: behavior.into(),
            }];
            assert!(validate_rules(&rules).is_ok(), "{product}-{behavior} 应开放");
        }
        let ok = vec![
            BlacklistRuleDTO {
                product: "mall".into(),
                behavior: "place_order".into(),
            },
            BlacklistRuleDTO {
                product: "mall".into(),
                behavior: "place_order".into(),
            },
        ];
        let deduped = validate_rules(&ok).unwrap();
        assert_eq!(deduped.len(), 1, "重复规则去重");
    }

    #[test]
    fn phase2_user_facing_messages_follow_copy_rules() {
        let publish = user_facing_message(BEHAVIOR_BUYER_SHOW_PUBLISH, &hit(OWNER_PLATFORM));
        assert_eq!(publish, "您的账号暂时无法发布买家秀，如有疑问请联系客服");
        let comment = user_facing_message(BEHAVIOR_BUYER_SHOW_COMMENT, &hit(OWNER_PLATFORM));
        assert_eq!(comment, "您的账号暂时无法评论，如有疑问请联系客服");
        let salesman = user_facing_message(BEHAVIOR_BECOME_SALESMAN, &hit(OWNER_PLATFORM));
        assert_eq!(salesman, "您的账号暂时无法成为分销员，如有疑问请联系客服");
        for msg in [publish, comment, salesman] {
            assert!(!msg.contains("黑名单"), "用户端文案不得出现「黑名单」: {msg}");
        }
    }

    fn parsed_row(user_id: i32, product: &str, behavior: &str, shops: &[i32]) -> ParsedImportRow {
        ParsedImportRow {
            raw: BlacklistImportErrorVO {
                row: 2,
                ..Default::default()
            },
            user_id,
            rule: (product.to_string(), behavior.to_string()),
            shop_ids: shops.to_vec(),
            reason: "测试原因".to_string(),
        }
    }

    #[test]
    fn group_import_rows_merges_same_user() {
        let rows = vec![
            parsed_row(9, "mall", "place_order", &[3]),
            parsed_row(10, "mall", "place_order", &[]),
            parsed_row(9, "buyer_show", "buyer_show_publish", &[5]),
            parsed_row(9, "mall", "place_order", &[3]),
        ];
        let groups = group_import_rows(rows);
        assert_eq!(groups.len(), 2, "同人多行归并为一组，组序按首次出现");
        assert_eq!(groups[0].user_id, 9);
        assert_eq!(groups[0].rules.len(), 2, "规则并集去重");
        assert_eq!(groups[0].shop_ids, vec![3, 5], "生效店铺取并集");
        assert_eq!(groups[0].rows.len(), 3);
        assert_eq!(groups[1].user_id, 10);
        assert_eq!(groups[1].rows.len(), 1);
    }
}
