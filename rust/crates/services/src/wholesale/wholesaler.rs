//! 批发商身份域服务（wholesale::wholesaler）。
//!
//! 对应原 Java `com.nextrade.service.wholesale.impl.WholesalerServiceImpl`
//! （657 行，管理端 8 个端点部分）。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java | 方法 |
//! |---------------------------|---------|------|
//! | `GET  /wholesale/wholesaler/page` | `WholesalerController.page` | [`WholesalerService::page`] |
//! | `GET  /wholesale/wholesaler/selectableUserPage` | `WholesalerController.selectableUserPage` | [`WholesalerService::selectable_user_page`] |
//! | `POST /wholesale/wholesaler/add` | `WholesalerController.add` | [`WholesalerService::add`] |
//! | `POST /wholesale/wholesaler/import` | `WholesalerController.importBatch` | [`WholesalerService::import_batch`] |
//! | `POST /wholesale/wholesaler/setStatus` | `WholesalerController.setStatus` | [`WholesalerService::set_status`] |
//! | `POST /wholesale/wholesaler/changeGrade` | `WholesalerController.changeGrade` | [`WholesalerService::change_grade`] |
//! | `POST /wholesale/wholesaler/removeByGrade` | `WholesalerController.removeByGrade` | [`WholesalerService::remove_by_grade`] |
//! | `POST /wholesale/wholesaler/remove` | `WholesalerController.remove` | [`WholesalerService::remove`] |
//!
//! # 语义要点（逐条对照 Java）
//!
//! - `page`：`keyword`/`fromTag` 走 user 表子查询 → `gradeId`/`status`/`source`
//!   等值过滤 → `create_time` 区间（`addTimeStart/End` 按东八区解析，失败忽略）
//!   → `wholesaler_id DESC`；分页后批量回填 user / user_rank / grade。
//! - `add`：用户存在（5301）→ 等级启用（5303/5304）→ 判重**含软删行**
//!   （未删 → 5302；软删 → 复活并保留 `wholesaler_id`）→ 新增
//!   `status=1, source=3(手动添加), shop_id=0`。
//! - `import`：等级缺省回退默认等级（未配置 → 5331）→ 目标集合优先级
//!   全体 > 按会员等级 > 自选会员（自选为空 → 400）→ 已存在按
//!   `OVERWRITE_GRADE`（默认，覆盖等级、计入 skipped）或 `SKIP` 处理；
//!   软删行复活并计入 success；不存在的 userId 计入 failed。
//! - `setStatus` / `changeGrade`：status 仅 0/1；批发商不存在 → 404；
//!   `changeGrade` 先校验目标等级启用。
//! - `removeByGrade`：`removeAll=true` 忽略 ids；否则 ids 必填（400），
//!   并按 `gradeId + is_delete=0` 过滤（跨等级误删保护）；软删。
//! - `remove`：ids 去重、必填非空（400）；软删。
//!
//! # 与 Java 的差异（已写入 models 模块头）
//!
//! 1. Redis `wholesale:isBuyer:*` 缓存未迁移（写后读库即时生效）；
//! 2. `selectableUserPage` 只支持 `keyword`/`fromTag`/`rankId`/`status` 子集；
//! 3. 默认头像配置回填未迁移；
//! 4. 900+ 条会员的导入分批（Java `BATCH_SIZE=500`）简化为逐条/逐批 SQL，
//!    单批上限由 `@Size(max=50)`（批量添加）与导入入口控制。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_core::entities::Wholesaler;
use nextrade_core::models::wholesale::wholesaler::{
    auth_status_name, distinct_ids, format_time, from_tag_name, is_skip_strategy,
    parse_time_to_unix, source_name, validate_add, WholesalerAddDTO, WholesalerImportDTO,
    WholesalerImportFailedItem, WholesalerImportResultVO, WholesalerPageQuery,
    WholesalerRemoveByGradeDTO, WholesalerRemoveDTO, WholesalerSelectableUserQuery,
    WholesalerSelectableUserVO, WholesalerVO, CODE_BAD_REQUEST, CODE_DEFAULT_GRADE_NOT_CONFIGURED,
    CODE_GRADE_DISABLED, CODE_GRADE_NOT_FOUND, CODE_NOT_FOUND, CODE_USER_ALREADY_EXISTS,
    CODE_USER_NOT_FOUND, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MSG_DEFAULT_GRADE_NOT_CONFIGURED,
    MSG_GRADE_DISABLED, MSG_GRADE_ID_REQUIRED, MSG_GRADE_NOT_FOUND, MSG_REMOVE_BY_GRADE_REQUIRED,
    MSG_REMOVE_REQUIRED, MSG_STATUS_INVALID, MSG_USER_ALREADY_EXISTS, MSG_USER_IDS_REQUIRED,
    MSG_USER_NOT_FOUND, MSG_WHOLESALER_NOT_FOUND, SOURCE_IMPORT, SOURCE_MANUAL,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::wholesale::wholesaler::{
    UserBriefRow, WholesalerRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 批发商身份域服务（挂载于 `AdminState.wholesale.wholesaler`）。
pub struct WholesalerService {
    repos: Arc<Repositories>,
}

impl WholesalerService {
    /// 构造（与同域其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &WholesalerRepository {
        &self.repos.wholesale.wholesaler
    }

    // ————————————————————— 分页 —————————————————————

    /// 批发商分页（Java `WholesalerServiceImpl#page`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn page(
        &self,
        q: &WholesalerPageQuery,
    ) -> Result<PageResult<WholesalerVO>> {
        let page = nextrade_core::types::PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let (total, records) = self
            .repo()
            .paginate(
                q.keyword.as_deref(),
                q.from_tag,
                q.grade_id,
                q.status,
                q.source,
                parse_time_to_unix(q.add_time_start.as_deref()),
                parse_time_to_unix(q.add_time_end.as_deref()),
                limit,
                offset,
            )
            .await?;
        if records.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }

        // 批量回填 user / user_rank / grade（Java CR-VOS-1：IN 查询，不写 JOIN）
        let user_ids: Vec<i32> = records.iter().map(|w| w.user_id).collect();
        let users: HashMap<i32, UserBriefRow> = self
            .repo()
            .find_users_by_ids(&dedup(&user_ids))
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        let rank_ids: Vec<i32> = users.values().map(|u| u.rank_id).collect();
        let ranks: HashMap<i32, String> = self
            .repo()
            .find_ranks_by_ids(&dedup(&rank_ids))
            .await?
            .into_iter()
            .collect();
        let grade_ids: Vec<i32> = records.iter().map(|w| w.grade_id).collect();
        let grades: HashMap<i32, String> = self
            .repo()
            .find_grade_names(&dedup(&grade_ids))
            .await?
            .into_iter()
            .collect();

        let list: Vec<WholesalerVO> = records
            .iter()
            .map(|w| to_vo(w, users.get(&w.user_id), &ranks, &grades))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 「可选为批发商的会员」分页（Java `WholesalerServiceImpl#selectableUserPage`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn selectable_user_page(
        &self,
        q: &WholesalerSelectableUserQuery,
    ) -> Result<PageResult<WholesalerSelectableUserVO>> {
        let page = nextrade_core::types::PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let (total, users) = self
            .repo()
            .paginate_users(
                q.keyword.as_deref(),
                q.from_tag,
                q.rank_id,
                q.status,
                limit,
                offset,
            )
            .await?;
        if users.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }

        // 本页 userId → 未删除批发商（含禁用；Java selectList @TableLogic 口径）
        let user_ids: Vec<i32> = users.iter().map(|u| u.user_id).collect();
        let wholesalers: Vec<Wholesaler> = self
            .repo()
            .find_by_user_ids_include_deleted(&dedup(&user_ids))
            .await?
            .into_iter()
            .filter(|w| w.is_delete == 0)
            .collect();
        let grade_ids: Vec<i32> = wholesalers.iter().map(|w| w.grade_id).collect();
        let grades: HashMap<i32, String> = self
            .repo()
            .find_grade_names(&dedup(&grade_ids))
            .await?
            .into_iter()
            .collect();
        let by_user: HashMap<i32, &Wholesaler> =
            wholesalers.iter().map(|w| (w.user_id, w)).collect();

        let list: Vec<WholesalerSelectableUserVO> = users
            .iter()
            .map(|u| {
                let w = by_user.get(&u.user_id);
                WholesalerSelectableUserVO {
                    user_id: u.user_id,
                    username: u.username.clone(),
                    nickname: u.nickname.clone(),
                    mobile: u.mobile.clone(),
                    email: u.email.clone(),
                    avatar: u.avatar.clone(),
                    rank_id: u.rank_id,
                    from_tag: u.from_tag,
                    from_tag_name: from_tag_name(u.from_tag).to_string(),
                    status: u.status,
                    is_wholesaler: w.is_some(),
                    grade_id: w.map(|w| w.grade_id),
                    grade_name: w.and_then(|w| grades.get(&w.grade_id).cloned()),
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 添加批发商（Java `WholesalerServiceImpl#add`），返回 `wholesalerId`。
    ///
    /// # Errors
    /// - 缺字段 → `Error::Validation(<Java 注解文案>)`
    /// - 用户不存在 → `Error::biz(5301, 用户不存在)`
    /// - 等级不存在 / 禁用 → 5303 / 5304
    /// - 已是批发商 → `Error::biz(5302, 该用户已是批发商)`
    pub async fn add(&self, dto: &WholesalerAddDTO) -> Result<i32> {
        validate_add(dto).map_err(Error::Validation)?;
        let user_id = dto.user_id.unwrap_or_default();
        let grade_id = dto.grade_id.unwrap_or_default();
        let remark = dto.remark.clone().unwrap_or_default();

        if !self.repo().user_exists(user_id).await? {
            return Err(Error::biz(CODE_USER_NOT_FOUND, MSG_USER_NOT_FOUND));
        }
        self.require_grade_enabled(grade_id).await?;

        let now = now_secs();
        // 判重含软删行：uk_user_id 不含 is_delete，软删行仍占 user_id
        if let Some(existing) = self.repo().find_by_user_id_include_deleted(user_id).await? {
            if existing.is_delete == 0 {
                return Err(Error::biz(CODE_USER_ALREADY_EXISTS, MSG_USER_ALREADY_EXISTS));
            }
            self.repo()
                .revive_by_id(existing.wholesaler_id, grade_id, SOURCE_MANUAL, &remark, now)
                .await?;
            tracing::info!("复活批发商: user_id={user_id}, wholesaler_id={}", existing.wholesaler_id);
            return Ok(existing.wholesaler_id);
        }

        let entity = Wholesaler {
            wholesaler_id: 0,
            user_id,
            grade_id,
            status: 1,
            source: SOURCE_MANUAL,
            shop_id: 0,
            remark,
            is_delete: 0,
            create_time: Some(now),
            update_time: Some(now),
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("添加批发商: user_id={user_id}, wholesaler_id={id}");
        Ok(id)
    }

    /// 批量导入批发商（Java `WholesalerServiceImpl#importBatch`）。
    ///
    /// # Errors
    /// - 目标集合为空（自选会员）→ `Error::biz(400, userIds 不能为空)`
    /// - 默认等级未配置 → `Error::biz(5331, 未配置默认批发等级)`
    /// - 等级不存在 / 禁用 → 5303 / 5304
    pub async fn import_batch(&self, dto: &WholesalerImportDTO) -> Result<WholesalerImportResultVO> {
        // 1) 等级：缺省回退配置的默认批发等级
        let mut grade_id = dto.grade_id.unwrap_or(0);
        if grade_id <= 0 {
            grade_id = self.repo().default_grade_id().await?;
            if grade_id == 0 {
                return Err(Error::biz(
                    CODE_DEFAULT_GRADE_NOT_CONFIGURED,
                    MSG_DEFAULT_GRADE_NOT_CONFIGURED,
                ));
            }
        }
        self.require_grade_enabled(grade_id).await?;

        // 2) 目标 userIds 优先级：全体 > 按会员等级 > 自选会员
        let import_all = dto.import_all.unwrap_or(false);
        let rank_ids = distinct_ids(dto.rank_ids.as_deref().unwrap_or(&[]));
        let by_rank = !import_all && !rank_ids.is_empty();
        let user_ids: Vec<i32> = if import_all {
            self.repo().list_active_user_ids().await?
        } else if by_rank {
            self.repo()
                .list_active_user_ids_by_ranks(&rank_ids)
                .await?
        } else {
            let raw = dto.user_ids.as_deref().unwrap_or(&[]);
            if raw.is_empty() {
                return Err(Error::biz(CODE_BAD_REQUEST, MSG_USER_IDS_REQUIRED));
            }
            distinct_ids(raw)
        };

        let skip = is_skip_strategy(dto.exists_strategy.as_deref());

        // 3) 存在性：来源为 user 表时无需再校验；自选会员需过滤不存在者
        let existing_users: HashSet<i32> = if import_all || by_rank {
            user_ids.iter().copied().collect()
        } else {
            self.repo()
                .load_existing_user_ids(&user_ids)
                .await?
                .into_iter()
                .collect()
        };
        // 含软删的已存在批发商（三态：未删 / 软删复活 / 全新）
        let existing_map: HashMap<i32, Wholesaler> = self
            .repo()
            .find_by_user_ids_include_deleted(&user_ids)
            .await?
            .into_iter()
            .map(|w| (w.user_id, w))
            .collect();

        let mut result = WholesalerImportResultVO {
            total_requested: i32::try_from(user_ids.len()).unwrap_or(i32::MAX),
            ..Default::default()
        };
        let now = now_secs();
        let mut to_insert: Vec<Wholesaler> = Vec::new();
        let mut to_revive: Vec<i32> = Vec::new();
        let mut to_overwrite: Vec<i32> = Vec::new();

        for uid in &user_ids {
            if !existing_users.contains(uid) {
                result.failed_count += 1;
                result.failed_details.push(WholesalerImportFailedItem {
                    user_id: *uid,
                    reason: MSG_USER_NOT_FOUND.to_string(),
                });
                continue;
            }
            match existing_map.get(uid) {
                Some(w) if w.is_delete == 0 => {
                    // 已是批发商：一律计入 skipped；非 SKIP 时覆盖等级
                    result.skipped_count += 1;
                    if !skip {
                        to_overwrite.push(*uid);
                    }
                }
                Some(_) => to_revive.push(*uid), // 软删行 → 复活（计入 success）
                None => to_insert.push(Wholesaler {
                    wholesaler_id: 0,
                    user_id: *uid,
                    grade_id,
                    status: 1,
                    source: SOURCE_IMPORT,
                    shop_id: 0,
                    remark: String::new(),
                    is_delete: 0,
                    create_time: Some(now),
                    update_time: Some(now),
                }),
            }
        }

        for entity in &to_insert {
            self.repo().insert(entity).await?;
        }
        let revived = self
            .repo()
            .revive_by_user_ids(&to_revive, grade_id, SOURCE_IMPORT, now)
            .await?;
        let overwritten = self
            .repo()
            .update_grade_by_user_ids(&to_overwrite, grade_id, now)
            .await?;
        result.success_count = i32::try_from(to_insert.len() + to_revive.len()).unwrap_or(i32::MAX);
        tracing::info!(
            "批量导入批发商: total={}, success={}, skipped={}, failed={}, revived={revived}, overwritten={overwritten}",
            result.total_requested,
            result.success_count,
            result.skipped_count,
            result.failed_count
        );
        Ok(result)
    }

    /// 批发商启停（Java `WholesalerServiceImpl#setStatus`）。
    ///
    /// # Errors
    /// - 非法 status → `Error::biz(400, status 必须为 0 或 1)`
    /// - 批发商不存在 → `Error::biz(404, 批发商不存在)`
    pub async fn set_status(&self, wholesaler_id: i32, status: Option<i32>) -> Result<()> {
        let status = match status {
            Some(v) if v == 0 || v == 1 => v,
            _ => return Err(Error::biz(CODE_BAD_REQUEST, MSG_STATUS_INVALID)),
        };
        if self.repo().find_by_id(wholesaler_id).await?.is_none() {
            return Err(Error::biz(CODE_NOT_FOUND, MSG_WHOLESALER_NOT_FOUND));
        }
        self.repo()
            .set_status(wholesaler_id, status, now_secs())
            .await?;
        tracing::info!("批发商启停: wholesaler_id={wholesaler_id}, status={status}");
        Ok(())
    }

    /// 批发商改等级（Java `WholesalerServiceImpl#changeGrade`）。
    ///
    /// # Errors
    /// - 目标等级不存在 / 禁用 → 5303 / 5304
    /// - 批发商不存在 → `Error::biz(404, 批发商不存在)`
    pub async fn change_grade(&self, wholesaler_id: i32, grade_id: i32) -> Result<()> {
        self.require_grade_enabled(grade_id).await?;
        if self.repo().find_by_id(wholesaler_id).await?.is_none() {
            return Err(Error::biz(CODE_NOT_FOUND, MSG_WHOLESALER_NOT_FOUND));
        }
        self.repo()
            .set_grade(wholesaler_id, grade_id, now_secs())
            .await?;
        tracing::info!("批发商改等级: wholesaler_id={wholesaler_id}, grade_id={grade_id}");
        Ok(())
    }

    /// 按等级移除批发商（Java `WholesalerServiceImpl#removeByGrade`）。
    ///
    /// `removeAll=true` → 移除该等级全部；否则按 `wholesalerIds` 且仅限属于
    /// 该等级的记录。返回实际软删条数。
    ///
    /// # Errors
    /// - 缺 `gradeId` → `Error::Validation(gradeId 不能为空)`
    /// - 未选目标且未勾选全部 → `Error::biz(400, 请选择要移除的批发商或勾选全部)`
    pub async fn remove_by_grade(&self, dto: &WholesalerRemoveByGradeDTO) -> Result<i32> {
        let grade_id = dto
            .grade_id
            .ok_or_else(|| Error::Validation(MSG_GRADE_ID_REQUIRED.into()))?;
        let remove_all = dto.remove_all.unwrap_or(false);
        let ids: Option<Vec<i32>> = if remove_all {
            None
        } else {
            let ids = distinct_ids(dto.wholesaler_ids.as_deref().unwrap_or(&[]));
            if ids.is_empty() {
                return Err(Error::biz(CODE_BAD_REQUEST, MSG_REMOVE_BY_GRADE_REQUIRED));
            }
            Some(ids)
        };
        let targets = self
            .repo()
            .find_by_grade_for_remove(grade_id, ids.as_deref())
            .await?;
        if targets.is_empty() {
            return Ok(0);
        }
        self.repo()
            .soft_delete_by_grade(grade_id, ids.as_deref(), now_secs())
            .await?;
        let removed = i32::try_from(targets.len()).unwrap_or(i32::MAX);
        tracing::info!("按等级移除批发商: grade_id={grade_id}, removed={removed}");
        Ok(removed)
    }

    /// 按 ID 移除批发商身份（Java `WholesalerServiceImpl#remove`）。
    ///
    /// 已删除 / 不存在的 ID 自动跳过，不计入返回值。
    ///
    /// # Errors
    /// `wholesalerIds` 为空 → `Error::biz(400, 请选择要移除的批发商)`
    pub async fn remove(&self, dto: &WholesalerRemoveDTO) -> Result<i32> {
        let raw: Vec<i32> = dto
            .wholesaler_ids
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|id| *id > 0)
            .collect();
        let ids = distinct_ids(&raw);
        if ids.is_empty() {
            return Err(Error::biz(CODE_BAD_REQUEST, MSG_REMOVE_REQUIRED));
        }
        let targets = self.repo().find_by_ids_for_remove(&ids).await?;
        if targets.is_empty() {
            return Ok(0);
        }
        self.repo().soft_delete_by_ids(&ids, now_secs()).await?;
        let removed = i32::try_from(targets.len()).unwrap_or(i32::MAX);
        tracing::info!("按 ID 移除批发商: removed={removed}");
        Ok(removed)
    }

    // ————————————————————— 内部 —————————————————————

    /// 等级必须存在且启用（Java `WholesalerGradeService#requireEnabled`）。
    async fn require_grade_enabled(&self, grade_id: i32) -> Result<()> {
        match self.repo().grade_enabled(grade_id).await? {
            Some(true) => Ok(()),
            Some(false) => Err(Error::biz(CODE_GRADE_DISABLED, MSG_GRADE_DISABLED)),
            None => Err(Error::biz(CODE_GRADE_NOT_FOUND, MSG_GRADE_NOT_FOUND)),
        }
    }
}

/// 批发商实体 → VO（Java `WholesalerServiceImpl#toVO`）。
#[must_use]
fn to_vo(
    w: &Wholesaler,
    user: Option<&UserBriefRow>,
    ranks: &HashMap<i32, String>,
    grades: &HashMap<i32, String>,
) -> WholesalerVO {
    let mut vo = WholesalerVO {
        wholesaler_id: w.wholesaler_id,
        user_id: w.user_id,
        grade_id: w.grade_id,
        status: w.status,
        source: w.source,
        source_name: source_name(w.source).to_string(),
        remark: w.remark.clone(),
        add_time: format_time(w.create_time),
        grade_name: grades.get(&w.grade_id).cloned(),
        ..Default::default()
    };
    if let Some(u) = user {
        vo.username = Some(u.username.clone());
        vo.mobile = Some(u.mobile.clone());
        vo.email = Some(u.email.clone());
        vo.nickname = u.nickname.clone();
        vo.avatar = Some(u.avatar.clone());
        vo.rank_id = Some(u.rank_id);
        vo.rank_name = ranks.get(&u.rank_id).cloned();
        vo.is_company_auth = Some(u.is_company_auth);
        vo.auth_status_name = auth_status_name(u.is_company_auth).to_string();
        vo.from_tag = Some(u.from_tag);
        vo.from_tag_name = from_tag_name(u.from_tag).to_string();
        vo.user_status = u.status;
    }
    vo
}

/// 去重（保序之外的简单去重；Java 用 `distinct()`）。
fn dedup(ids: &[i32]) -> Vec<i32> {
    let mut seen = HashSet::new();
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}

/// 当前 Unix 秒（`wholesaler.create_time` / `update_time` 为 bigint 秒级）。
fn now_secs() -> i64 {
    i64::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> UserBriefRow {
        UserBriefRow {
            user_id: 100,
            username: "buyer".into(),
            mobile: "13800000000".into(),
            email: "b@example.com".into(),
            nickname: Some("老王".into()),
            avatar: String::new(),
            rank_id: 2,
            from_tag: 1,
            status: Some(1),
            is_company_auth: 1,
        }
    }

    fn wholesaler() -> Wholesaler {
        Wholesaler {
            wholesaler_id: 5,
            user_id: 100,
            grade_id: 1,
            status: 1,
            source: SOURCE_MANUAL,
            shop_id: 0,
            remark: "备注".into(),
            is_delete: 0,
            create_time: Some(1_700_000_000),
            update_time: Some(1_700_000_000),
        }
    }

    /// VO 映射：user / rank / grade / 派生文案全部回填。
    #[test]
    fn to_vo_backfills_user_and_names() {
        let mut ranks = HashMap::new();
        ranks.insert(2, "黄金会员".to_string());
        let mut grades = HashMap::new();
        grades.insert(1, "一级批发商".to_string());
        let vo = to_vo(&wholesaler(), Some(&user()), &ranks, &grades);
        assert_eq!(vo.wholesaler_id, 5);
        assert_eq!(vo.username.as_deref(), Some("buyer"));
        assert_eq!(vo.rank_name.as_deref(), Some("黄金会员"));
        assert_eq!(vo.grade_name.as_deref(), Some("一级批发商"));
        assert_eq!(vo.source_name, "手动添加");
        assert_eq!(vo.auth_status_name, "已认证");
        assert_eq!(vo.from_tag_name, "公众号");
        assert_eq!(vo.add_time.as_deref(), Some("2023-11-15 06:13:20"));
    }

    /// 会员缺失时只保留 wholesaler 主表字段（Java 部分回填）。
    #[test]
    fn to_vo_tolerates_missing_user() {
        let vo = to_vo(&wholesaler(), None, &HashMap::new(), &HashMap::new());
        assert_eq!(vo.user_id, 100);
        assert!(vo.username.is_none());
        assert_eq!(vo.auth_status_name, "");
        assert_eq!(vo.from_tag_name, "");
    }

    /// 去重保序 / 空入参。
    #[test]
    fn dedup_keeps_order() {
        assert_eq!(dedup(&[3, 1, 3, 2]), vec![3, 1, 2]);
        assert!(dedup(&[]).is_empty());
    }
}
