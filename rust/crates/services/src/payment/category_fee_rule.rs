//! 品类服务费率规则域服务（payment::category_fee_rule）。
//!
//! 对应原 Java `CategoryFeeRuleServiceImpl`
//! （`com.nextrade.service.finance.impl`，7 个管理端端点）。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - `CategoryFeeRuleMatchServiceImpl#evictCacheByTargetType`（Redis `SCAN` 删
//!   `nextrade:feeRule:match:*` 缓存）未迁移：Rust 侧无匹配服务与对应缓存键，
//!   仅保留 `tracing` 日志。匹配算法本身在管理端不暴露接口，不在本切片范围。
//! - 供应商名：Java 用 `VendorMapper.selectOnboardedById`（只取「已入驻」供应商），
//!   Rust 侧统一按 `vendor_id` 取 `vendor_name`（`vendor` 表无独立入驻列，
//!   见模块头偏差说明）。
//! - `save/update/updateStatus` 的返回值 Java 恒为 `Collections.emptyList()`，
//!   本实现同样返回空 `Vec<String>`。
//! - `create_time` / `modify_time` 是 MySQL `DATETIME`：Java `LocalDateTime.now()`，
//!   本实现写 `yyyy-MM-dd HH:mm:ss`（UTC+8）字符串，与库列类型兼容。
//! - `assertNoConflict` 文案逐字符照抄（含前 5 条规则名 + 「等共N条」）。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::entities::{CategoryServiceFeeRule, CategoryServiceFeeRuleCategory};
use nextrade_core::models::payment::category_fee_rule::{
    conflict_message, has_target_overlap, now_datetime, requires_target_ids, scope_type_name,
    target_type_name, CategoryFeeRuleCategoryItem, CategoryFeeRuleConflictCheckDTO,
    CategoryFeeRuleConflictItem, CategoryFeeRuleConflictVO, CategoryFeeRuleDetailVO,
    CategoryFeeRuleIdsDTO, CategoryFeeRuleListVO, CategoryFeeRulePageQuery, CategoryFeeRuleSaveDTO,
    CategoryFeeRuleTargetItem, CategoryFeeRuleUpdateStatusDTO, MSG_CONFLICT_PREFIX,
    MSG_DUPLICATE_NAME, MSG_RULE_ID_REQUIRED, MSG_RULE_NOT_EXISTS, TARGET_TYPE_VENDOR,
    TARGET_TYPE_SHOP,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::category_fee_rule::CategoryFeeRuleRepository;
use nextrade_infrastructure::db::Repositories;

/// 冲突匹配（Java 私有类 `ConflictMatch`）。
#[derive(Debug, Clone)]
pub struct ConflictMatch {
    /// 命中的已有规则
    pub rule: CategoryServiceFeeRule,
    /// 与入参类目相交的关联行
    pub overlapping_categories: Vec<CategoryServiceFeeRuleCategory>,
}

/// 品类服务费率规则域服务。
pub struct CategoryFeeRuleService {
    repos: Arc<Repositories>,
}

impl CategoryFeeRuleService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &CategoryFeeRuleRepository {
        &self.repos.payment.category_fee_rule
    }

    // ————————————————————— 查询 —————————————————————

    /// 分页列表（Java `CategoryFeeRuleServiceImpl#page`）。
    pub async fn page(
        &self,
        q: &CategoryFeeRulePageQuery,
    ) -> Result<PageResult<CategoryFeeRuleListVO>> {
        let target_type = q
            .target_type_required()
            .map_err(Error::Validation)?;
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, target_type, limit, offset).await?;
        if rows.is_empty() {
            let empty = PageResult::empty(&page);
            return Ok(empty);
        }
        let rule_ids: Vec<i32> = rows.iter().map(|r| r.id).collect();
        let cat_counts = self.repo().category_counts(&rule_ids).await?;
        let tgt_counts = self.repo().target_counts(&rule_ids).await?;
        let list: Vec<CategoryFeeRuleListVO> = rows
            .iter()
            .map(|rule| {
                to_list_vo(
                    rule,
                    *cat_counts.get(&rule.id).unwrap_or(&0),
                    *tgt_counts.get(&rule.id).unwrap_or(&0),
                )
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 详情（Java `CategoryFeeRuleServiceImpl#detail`）。
    pub async fn detail(&self, id: i32) -> Result<CategoryFeeRuleDetailVO> {
        let rule = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RULE_NOT_EXISTS))?;
        let cats = self
            .repo()
            .categories_by_rule_ids(&[id])
            .await?
            .remove(&id)
            .unwrap_or_default();
        let tgts = self
            .repo()
            .targets_by_rule_ids(&[id])
            .await?
            .remove(&id)
            .unwrap_or_default();
        let base = to_list_vo(
            &rule,
            i64::try_from(cats.len()).unwrap_or(0),
            i64::try_from(tgts.len()).unwrap_or(0),
        );
        let mut vo = CategoryFeeRuleDetailVO {
            base,
            remark: rule.remark.clone(),
            categories: cats
                .iter()
                .map(|c| CategoryFeeRuleCategoryItem {
                    category_id: c.category_id,
                    category_name: c.category_name.clone(),
                    category_path: c.category_path.clone(),
                })
                .collect(),
            targets: tgts
                .iter()
                .map(|t| CategoryFeeRuleTargetItem {
                    target_id: t.target_id,
                    target_name: String::new(),
                })
                .collect(),
            modify_time: rule.modify_time.clone(),
        };
        self.fill_target_names(&mut vo.targets, rule.target_type)
            .await?;
        Ok(vo)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新建（Java `CategoryFeeRuleServiceImpl#save`）。
    pub async fn save(&self, dto: &CategoryFeeRuleSaveDTO) -> Result<Vec<String>> {
        dto.validate().map_err(Error::Validation)?;
        let target_type = dto.target_type.unwrap_or_default();
        let rule_name = dto.rule_name.clone().unwrap_or_default();
        self.assert_name_unique(target_type, &rule_name, None).await?;
        self.assert_no_conflict(&dto.to_conflict_dto(), dto.category_ids.clone())
            .await?;

        let now = now_datetime();
        let rule = CategoryServiceFeeRule {
            id: 0,
            rule_name,
            target_type,
            scope_type: dto.scope_type.unwrap_or_default(),
            service_fee_rate: dto.service_fee_rate.unwrap_or(Decimal::ZERO),
            remark: dto.remark.clone().unwrap_or_default(),
            status: dto.status.unwrap_or_default(),
            deleted: 0,
            create_time: Some(now.clone()),
            modify_time: Some(now.clone()),
        };
        let rule_id = self.repo().insert(&rule).await?;
        self.save_categories(rule_id, dto.category_ids.as_deref().unwrap_or(&[]))
            .await?;
        if requires_target_ids(rule.scope_type) {
            if let Some(targets) = dto.target_ids.as_deref().filter(|v| !v.is_empty()) {
                self.save_targets(rule_id, targets).await?;
            }
        }
        let log_info = format!("创建费率规则: {}", rule.rule_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("创建费率规则: id={rule_id}, name={}", rule.rule_name);
        Ok(Vec::new())
    }

    /// 编辑（Java `CategoryFeeRuleServiceImpl#update`）。
    pub async fn update(&self, dto: &CategoryFeeRuleSaveDTO) -> Result<Vec<String>> {
        let id = dto
            .id
            .ok_or_else(|| Error::biz(500, MSG_RULE_ID_REQUIRED))?;
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RULE_NOT_EXISTS))?;
        dto.validate().map_err(Error::Validation)?;
        let target_type = dto.target_type.unwrap_or_default();
        let rule_name = dto.rule_name.clone().unwrap_or_default();
        self.assert_name_unique(target_type, &rule_name, Some(id))
            .await?;
        self.assert_no_conflict(&dto.to_conflict_dto(), dto.category_ids.clone())
            .await?;

        let now = now_datetime();
        let rule = CategoryServiceFeeRule {
            id,
            rule_name: rule_name.clone(),
            target_type,
            scope_type: dto.scope_type.unwrap_or_default(),
            service_fee_rate: dto.service_fee_rate.unwrap_or(Decimal::ZERO),
            remark: dto.remark.clone().unwrap_or_default(),
            status: dto.status.unwrap_or_default(),
            deleted: 0,
            create_time: existing.create_time.clone(),
            modify_time: Some(now.clone()),
        };
        self.repo().update(&rule).await?;
        self.repo().delete_categories(id).await?;
        self.repo().delete_targets(id).await?;
        self.save_categories(id, dto.category_ids.as_deref().unwrap_or(&[]))
            .await?;
        if requires_target_ids(rule.scope_type) {
            if let Some(targets) = dto.target_ids.as_deref().filter(|v| !v.is_empty()) {
                self.save_targets(id, targets).await?;
            }
        }
        let log_info = format!("编辑费率规则: {}", rule_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("更新费率规则: id={id}");
        Ok(Vec::new())
    }

    /// 启用/禁用（Java `CategoryFeeRuleServiceImpl#updateStatus`）。
    ///
    /// 仅在启用（`status = 1`）时做冲突预检。
    pub async fn update_status(
        &self,
        dto: &CategoryFeeRuleUpdateStatusDTO,
    ) -> Result<Vec<String>> {
        let (id, status) = dto.validate().map_err(Error::Validation)?;
        let rule = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RULE_NOT_EXISTS))?;
        if status == 1 {
            let cats = self
                .repo()
                .categories_by_rule_ids(&[id])
                .await?
                .remove(&id)
                .unwrap_or_default();
            let tgts = self
                .repo()
                .targets_by_rule_ids(&[id])
                .await?
                .remove(&id)
                .unwrap_or_default();
            let chk = CategoryFeeRuleConflictCheckDTO {
                rule_id: Some(id),
                target_type: Some(rule.target_type),
                scope_type: Some(rule.scope_type),
                category_ids: Some(cats.iter().map(|c| c.category_id).collect()),
                target_ids: Some(tgts.iter().map(|t| t.target_id).collect()),
            };
            self.assert_no_conflict(&chk, chk.category_ids.clone()).await?;
        }
        self.repo().update_status(id, status, &now_datetime()).await?;
        let log_info = format!("更新费率规则状态 id={id} status={status}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("更新费率规则状态: id={id}, status={status}");
        Ok(Vec::new())
    }

    /// 逻辑删除 + 物理删除关联（Java `CategoryFeeRuleServiceImpl#delete`）。
    pub async fn delete(&self, dto: &CategoryFeeRuleIdsDTO) -> Result<()> {
        let ids = dto.validate().map_err(Error::Validation)?;
        for rid in &ids {
            self.repo().delete_categories(*rid).await?;
            self.repo().delete_targets(*rid).await?;
        }
        let now = now_datetime();
        for rid in &ids {
            self.repo().soft_delete(*rid, &now).await?;
        }
        let log_info = format!("删除费率规则 ids={ids:?}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("删除费率规则: {ids:?}");
        Ok(())
    }

    /// 冲突预检（Java `CategoryFeeRuleServiceImpl#checkConflict`）。
    pub async fn check_conflict(
        &self,
        dto: &CategoryFeeRuleConflictCheckDTO,
    ) -> Result<CategoryFeeRuleConflictVO> {
        let (target_type, scope_type, category_ids) =
            dto.validate().map_err(Error::Validation)?;
        let chk = CategoryFeeRuleConflictCheckDTO {
            rule_id: dto.rule_id,
            target_type: Some(target_type),
            scope_type: Some(scope_type),
            category_ids: Some(category_ids.clone()),
            target_ids: dto.target_ids.clone(),
        };
        let matches = self.find_conflicting(&chk, Some(category_ids)).await?;
        let mut conflicts: Vec<CategoryFeeRuleConflictItem> = Vec::new();
        for m in &matches {
            for cat in &m.overlapping_categories {
                conflicts.push(CategoryFeeRuleConflictItem {
                    category_id: cat.category_id,
                    category_name: cat.category_name.clone(),
                    existing_rule_id: m.rule.id,
                    existing_rule_name: m.rule.rule_name.clone(),
                    target_id: None,
                    target_name: None,
                });
            }
        }
        Ok(CategoryFeeRuleConflictVO {
            has_conflict: !conflicts.is_empty(),
            conflicts,
        })
    }

    // ————————————————————— 内部 —————————————————————

    /// 同类型下规则名唯一（Java `existsByRuleName`）。
    async fn assert_name_unique(
        &self,
        target_type: i32,
        rule_name: &str,
        exclude_id: Option<i32>,
    ) -> Result<()> {
        if self
            .repo()
            .count_by_rule_name(target_type, rule_name, exclude_id)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_DUPLICATE_NAME));
        }
        Ok(())
    }

    /// 冲突即报错（Java `assertNoConflict`）。
    async fn assert_no_conflict(
        &self,
        dto: &CategoryFeeRuleConflictCheckDTO,
        _category_ids: Option<Vec<i32>>,
    ) -> Result<()> {
        let matches = self.find_conflicting(dto, dto.category_ids.clone()).await?;
        if matches.is_empty() {
            return Ok(());
        }
        let names: Vec<&str> = matches
            .iter()
            .map(|m| m.rule.rule_name.as_str())
            .collect();
        let msg = conflict_message(names);
        debug_assert!(msg.starts_with(MSG_CONFLICT_PREFIX));
        Err(Error::biz(500, msg))
    }

    /// 找出与入参冲突的规则（Java `findConflictingRules`）。
    async fn find_conflicting(
        &self,
        dto: &CategoryFeeRuleConflictCheckDTO,
        category_ids: Option<Vec<i32>>,
    ) -> Result<Vec<ConflictMatch>> {
        let target_type = dto.target_type.unwrap_or_default();
        let scope_type = dto.scope_type.unwrap_or_default();
        let input_category_ids: Vec<i32> = category_ids
            .or_else(|| dto.category_ids.clone())
            .unwrap_or_default();
        if input_category_ids.is_empty() {
            return Ok(Vec::new());
        }
        let input_target_ids: Vec<i32> = dto.target_ids.clone().unwrap_or_default();
        let candidates = self
            .repo()
            .list_conflict_candidates(target_type, dto.rule_id)
            .await?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let rule_ids: Vec<i32> = candidates.iter().map(|r| r.id).collect();
        let mut categories_by_rule = self.repo().categories_by_rule_ids(&rule_ids).await?;
        let mut targets_by_rule = self.repo().targets_by_rule_ids(&rule_ids).await?;
        let mut matches = Vec::new();
        for rule in candidates {
            let rule_cats = categories_by_rule.remove(&rule.id).unwrap_or_default();
            if rule_cats.is_empty() {
                continue;
            }
            let overlapping: Vec<CategoryServiceFeeRuleCategory> = rule_cats
                .into_iter()
                .filter(|c| input_category_ids.contains(&c.category_id))
                .collect();
            if overlapping.is_empty() {
                continue;
            }
            let existing_targets: Vec<i32> = targets_by_rule
                .remove(&rule.id)
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.target_id)
                .collect();
            if !has_target_overlap(
                scope_type,
                &input_target_ids,
                rule.scope_type,
                &existing_targets,
            ) {
                continue;
            }
            matches.push(ConflictMatch {
                rule,
                overlapping_categories: overlapping,
            });
        }
        Ok(matches)
    }

    /// 保存类目关联：品类名 + 路径（含祖先闭包）。
    async fn save_categories(&self, rule_id: i32, category_ids: &[i32]) -> Result<()> {
        let closure = self.load_category_closure(category_ids).await?;
        let now = now_datetime();
        for category_id in category_ids {
            let name = closure
                .get(category_id)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            let path = build_category_path(*category_id, &closure);
            self.repo()
                .insert_category(rule_id, *category_id, &name, &path, &now, &now)
                .await?;
        }
        Ok(())
    }

    /// 品类祖先闭包（Java `loadCategoryClosure`）：逐层向上取 `parent_id`，
    /// 返回 `category_id -> (category_name, parent_id)`。
    async fn load_category_closure(&self, category_ids: &[i32]) -> Result<HashMap<i32, Node>> {
        let mut need: Vec<i32> = Vec::new();
        for id in category_ids {
            if !need.contains(id) {
                need.push(*id);
            }
        }
        let mut frontier = need.clone();
        let mut map: HashMap<i32, Node> = HashMap::new();
        let mut guard = 0;
        while !frontier.is_empty() && guard < 32 {
            guard += 1;
            let batch = self.repo().find_categories(&frontier).await?;
            frontier.clear();
            for c in batch {
                if c.parent_id > 0 && !need.contains(&c.parent_id) {
                    need.push(c.parent_id);
                    frontier.push(c.parent_id);
                }
                map.insert(
                    c.category_id,
                    Node {
                        name: c.category_name,
                        parent_id: c.parent_id,
                    },
                );
            }
        }
        Ok(map)
    }

    /// 补齐目标名称（Java `fillTargetNames`）：店铺按 `shop_title`、供应商按 `vendor_name`。
    async fn fill_target_names(
        &self,
        targets: &mut [CategoryFeeRuleTargetItem],
        target_type: i32,
    ) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        if target_type == TARGET_TYPE_SHOP {
            let ids: Vec<i32> = targets.iter().map(|t| t.target_id).collect();
            let names: HashMap<i32, String> = self
                .repo()
                .find_shop_titles(&ids)
                .await?
                .into_iter()
                .map(|s| (s.shop_id, s.shop_title.unwrap_or_default()))
                .collect();
            for t in targets.iter_mut() {
                t.target_name = names.get(&t.target_id).cloned().unwrap_or_default();
            }
        } else if target_type == TARGET_TYPE_VENDOR {
            let ids: Vec<i32> = targets.iter().map(|t| t.target_id).collect();
            let names: HashMap<i32, String> = self
                .repo()
                .find_vendor_names(&ids)
                .await?
                .into_iter()
                .map(|v| (v.vendor_id, v.vendor_name.unwrap_or_default()))
                .collect();
            for t in targets.iter_mut() {
                t.target_name = names.get(&t.target_id).cloned().unwrap_or_default();
            }
        }
        Ok(())
    }

    /// 保存目标关联。
    async fn save_targets(&self, rule_id: i32, target_ids: &[i32]) -> Result<()> {
        let now = now_datetime();
        for target_id in target_ids {
            self.repo()
                .insert_target(rule_id, *target_id, &now, &now)
                .await?;
        }
        Ok(())
    }
}

/// 实体 → 列表 VO（Java `toListVO`）。
fn to_list_vo(rule: &CategoryServiceFeeRule, category_count: i64, target_count: i64) -> CategoryFeeRuleListVO {
    CategoryFeeRuleListVO {
        id: rule.id,
        rule_name: rule.rule_name.clone(),
        target_type: rule.target_type,
        target_type_name: target_type_name(Some(rule.target_type)).to_string(),
        scope_type: rule.scope_type,
        scope_type_name: scope_type_name(Some(rule.scope_type)).to_string(),
        precision_layer: None,
        service_fee_rate: rule.service_fee_rate,
        target_count: i32::try_from(target_count).unwrap_or(i32::MAX),
        category_count: i32::try_from(category_count).unwrap_or(i32::MAX),
        status: rule.status,
        create_time: rule.create_time.clone(),
    }
}

/// 品类闭包节点（名称 + 父 ID）。
#[derive(Debug, Clone, Default)]
struct Node {
    name: String,
    parent_id: i32,
}

/// 品类路径（Java `buildPath`：自底向上拼 `name/name/name`，最多 32 层）。
fn build_category_path(category_id: i32, by_id: &HashMap<i32, Node>) -> String {
    let mut names: Vec<String> = Vec::new();
    let mut current = category_id;
    let mut guard = 0;
    while current > 0 && guard < 32 {
        guard += 1;
        let Some(node) = by_id.get(&current) else {
            break;
        };
        names.insert(0, node.name.clone());
        current = node.parent_id;
    }
    names.join("/")
}

/// `BasePage` → 内部 `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    nextrade_core::types::PageQuery {
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

/// 仅供测试：判断范围枚举常量。
#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::category_fee_rule::{
        SCOPE_ALL, SCOPE_EXCLUDE, SCOPE_SPECIFIED,
    };

    #[test]
    fn page_query_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(3), Some(500));
        assert_eq!((p.page, p.page_size), (3, 100));
    }

    #[test]
    fn conflict_match_uses_rule_name_for_message() {
        let rule = CategoryServiceFeeRule {
            id: 9,
            rule_name: "外卖费率".into(),
            ..Default::default()
        };
        let m = ConflictMatch {
            rule,
            overlapping_categories: vec![],
        };
        assert_eq!(m.rule.rule_name, "外卖费率");
        let msg = conflict_message([m.rule.rule_name.as_str()]);
        assert_eq!(msg, "存在冲突规则，无法保存：外卖费率");
    }

    #[test]
    fn target_type_branch_constants_are_distinct() {
        assert_ne!(TARGET_TYPE_SHOP, TARGET_TYPE_VENDOR);
        assert_ne!(SCOPE_ALL, SCOPE_SPECIFIED);
        assert_ne!(SCOPE_ALL, SCOPE_EXCLUDE);
        assert!(!requires_target_ids(SCOPE_ALL));
    }

    #[test]
    fn list_vo_maps_enum_names() {
        let rule = CategoryServiceFeeRule {
            id: 1,
            rule_name: "r".into(),
            target_type: TARGET_TYPE_SHOP,
            scope_type: SCOPE_EXCLUDE,
            service_fee_rate: Decimal::new(350, 2),
            status: 1,
            ..Default::default()
        };
        let vo = to_list_vo(&rule, 2, 3);
        assert_eq!(vo.target_type_name, "店铺");
        assert_eq!(vo.scope_type_name, "不包含");
        assert_eq!(vo.category_count, 2);
        assert_eq!(vo.target_count, 3);
        assert_eq!(vo.service_fee_rate, Decimal::new(350, 2));
    }
}
