//! 会员等级域服务（member::user_rank）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.UserRankServiceImpl`
//! 与 `com.nextrade.adminapi.controller.user.UserRankController` 的 7 条链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /user/userRank/listByPro`   | `UserRankController.listByPro`   | [`UserRankService::list_by_pro`] |
//! | `POST /user/userRank/create`      | `UserRankController.create`      | [`UserRankService::create`] |
//! | `GET  /user/userRank/detail`      | `UserRankController.detail`      | [`UserRankService::detail`] |
//! | `POST /user/userRank/update`      | `UserRankController.update`      | [`UserRankService::update`] |
//! | `POST /user/userRank/updateField` | `UserRankController.updateField` | [`UserRankService::update_field`] |
//! | `POST /user/userRank/del`         | `UserRankController.del`         | [`UserRankService::del`] |
//! | `POST /user/userRank/batch`       | `UserRankController.batch`       | [`UserRankService::batch`] |
//!
//! `updateField` / `del` / `batch` 直接复用 Java `BaseServiceImpl` 的通用方法，
//! 本文件按同一语义实现。
//!
//! # ⚠️ 只实现 Pro 分支（`nextrade.is-pro = 1`）
//!
//! `listByPro` / `detail` / `update` 在原 Java 里都有「非 Pro 版」分支
//! （`nextradeProperties.getIsPro()`），而本仓库随包配置
//! `application.yaml` 的 `nextrade.is-pro: 1`（见
//! [`nextrade_core::models::member::user_rank::IS_PRO`]）。因此：
//! - `listByPro` 的非 Pro 分支（返回两个空数组）不可达；
//! - `detail` 的非 Pro 分支（`growUpSetting = ""`、`userRankConfig` 为空对象）不可达；
//! - `update` 的「非 Pro 直接 `setRankType(0)`」分支不可达。
//!
//! 若部署为**非 Pro 版**，必须改 `IS_PRO` 常量并补齐上述分支（见 `uncertain_points`）。
//!
//! # 与 Java 的已知差异（均列入 `uncertain_points`）
//!
//! 1. `rank_config.data` / `grow_config.data` / `rights` 的**落库 JSON 是紧凑格式**，
//!    而 Java 的 `JsonUtil` 开了 `INDENT_OUTPUT`（带缩进）。读回后解析结果一致，
//!    仅数据库里的空白不同。
//! 2. `create` 在 `rankName == null` 时**跳过查重**：Java 的
//!    `eq(rankName, null)` 生成 `rank_name = NULL`（恒不命中），等价于不查重。
//! 3. `update` 把入参校验错误统一映射为业务码
//!    [`nextrade_core::models::member::user_rank::SERVICE_DATA_ERROR`]（1001），
//!    与 Java `GlobalException` 的默认码一致；`@Size` / NPE 的边界见下。
//! 4. `data == null` / `rankType == null` 时 Java 会抛
//!    `NullPointerException`（HTTP 500），本实现提前返回 400（可诊断）。

use std::sync::Arc;

use nextrade_core::models::member::user_rank::{
    build_user_rank_fields, config_to_vo, init_rank_config_vo, init_rank_list_vo,
    map_update_field_column, parse_grow_setting, to_list_vo, validate_edit_param, BatchDTO,
    GrowUpSettingVO, OperateDTO, RankListByProVO, RankPageResult, UpdateFieldDTO, UserRankDTO,
    UserRankData, UserRankDetailVO, UserRankEditParam, UserRankFields, UserRankListQuery,
    UserRankListVO, GROW_CONFIG_CODE, IS_PRO, MSG_INVALID_FIELD_VALUE, MSG_RANK_CONFIG_MISSING,
    MSG_RANK_NAME_EXISTS, RANK_CONFIG_CODE, SERVICE_DATA_ERROR, USER_RANK_FIELDS,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_rank::{
    RankConfigWrite, RankEditPlan, UserRankRepository, UserRankWrite,
};
use nextrade_infrastructure::db::Repositories;

/// 会员等级域服务（挂载于 `AdminState.user_rank`）。
pub struct UserRankService {
    repos: Arc<Repositories>,
}

impl UserRankService {
    /// 构造（与 member_tag_group / vendor_deposit_shop / tax_rate 的
    /// `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &UserRankRepository {
        &self.repos.member.user_rank
    }

    // ————————————————————— 列表（listByPro） —————————————————————

    /// `GET /adminapi/user/userRank/listByPro`：会员等级列表 + 等级配置（Pro）
    /// （对应 Java `UserRankController#listByPro` → `UserRankServiceImpl#list`）。
    ///
    /// Java 逻辑：
    /// 1. `isPro != 1` ⇒ 返回两个空数组（本仓库 `IS_PRO = 1`，分支不可达）；
    /// 2. 取 `code = 'rank_config'` 的配置行，把它的 `rankType` **覆盖**到查询
    ///    参数上（取不到配置时用默认配置、`rankType = 1`）；
    /// 3. 分页查询 `user_rank`：`keyword` 模糊匹配 `rank_name`，
    ///    `rankType > 0` 时过滤 `rank_type`，默认按主键升序
    ///    （`sortOrder` 为空时补 `asc`）。
    pub async fn list_by_pro(&self, q: &UserRankListQuery) -> Result<RankListByProVO> {
        // ② 取配置行，并用它的 rankType 覆盖查询参数（Java `setRankType`）。
        let (rank_config, rank_type) = match self.repo().find_config_by_code(RANK_CONFIG_CODE).await? {
            Some(cfg) => (config_to_vo(&cfg), Some(cfg.rank_type)),
            // Java `getRankConfigInit()` 兜底：默认配置 + `rankType = 1`。
            None => (init_rank_config_vo(), Some(1)),
        };
        let mut q = q.clone();
        q.rank_type = rank_type;

        let user_rank = self.list_page(&q).await?;
        Ok(RankListByProVO {
            user_rank,
            rank_config,
        })
    }

    /// `listByPro` 的分页主体（对应 Java `UserRankServiceImpl#list`）。
    ///
    /// 结果为空时返回**一条**初始化兜底行（Java `getUserRankListInitDTO`），
    /// 但 `total` 仍为 0 —— 与 MyBatis-Plus `Page.convert` 的行为一致。
    ///
    /// 用户数用一次 `GROUP BY rank_id` 批量统计（等价 Java 的逐行
    /// `userMapper.selectCount`，避免 N+1）。
    async fn list_page(&self, q: &UserRankListQuery) -> Result<RankPageResult<UserRankListVO>> {
        let order = q.order_clause(true);
        let (total, rows) = self.repo().paginate(q, &order).await?;

        let records = if rows.is_empty() {
            vec![init_rank_list_vo()]
        } else {
            let ids: Vec<i32> = rows.iter().map(|r| r.rank_id).collect();
            let counts = self.repo().count_users_by_rank_ids(&ids).await?;
            rows.iter()
                .map(|r| {
                    let count = counts
                        .iter()
                        .find(|(id, _)| *id == r.rank_id)
                        .map_or(0, |(_, n)| i32::try_from(*n).unwrap_or(i32::MAX));
                    to_list_vo(r, count)
                })
                .collect()
        };

        Ok(RankPageResult::new(records, total, &q.page_query()))
    }

    // ————————————————————— 新增 —————————————————————

    /// `POST /adminapi/user/userRank/create`：新增会员等级
    /// （对应 Java `UserRankController#create` → `UserRankServiceImpl#create`
    /// → `#checkUserRank`）。
    ///
    /// Java 逻辑：
    /// 1. 名称查重：`count(rank_name = ?) > 0` ⇒
    ///    抛 `GlobalException("等级名称已存在", 1001)`；
    ///    ⚠️ `rankName == null` 时 MyBatis-Plus 生成 `rank_name = NULL`
    ///    （恒不命中），故本实现仅在 `rank_name` 非空时查重；
    /// 2. `BeanUtils.copyProperties(dto, entity)` 后 `save`
    ///    （`UserRankDTO` **没有** `rankLogo`，故不写该列，落库走列默认值）。
    ///
    /// 返回新增行的主键 `rank_id`。
    pub async fn create(&self, dto: &UserRankDTO) -> Result<i32> {
        if let Some(name) = dto.rank_name.as_deref() {
            if self.repo().exists_rank_name(name).await? {
                return Err(Error::biz(SERVICE_DATA_ERROR, MSG_RANK_NAME_EXISTS));
            }
        }

        let w = UserRankWrite {
            rank_id: dto.rank_id,
            rank_name: dto.rank_name.clone(),
            min_growth_points: dto.min_growth_points,
            max_growth_points: dto.max_growth_points,
            discount: dto.discount,
            show_price: dto.show_price,
            rank_type: dto.rank_type,
            rank_ico: dto.rank_ico.clone(),
            rank_bg: dto.rank_bg.clone(),
            ..Default::default()
        };
        self.repo().insert(&w).await
    }

    // ————————————————————— 详情 —————————————————————

    /// `GET /adminapi/user/userRank/detail`：会员等级详情（Pro）
    /// （对应 Java `UserRankController#detail` → `UserRankServiceImpl#detail`）。
    ///
    /// ⚠️ Java 先取 `code = 'rank_config'` 的配置行并用它的 `rankType`；
    /// 取不到该行时 `selectOne(...).getRankType()` 抛 `NullPointerException`
    /// （HTTP 500）。本实现返回同码可诊断业务错误 [`MSG_RANK_CONFIG_MISSING`]。
    ///
    /// 列表为空时返回初始化兜底行 + 默认成长值设置 + 初始配置
    /// （Java `getUserRankListInitDTO` / `getGrowUpSettingInit` / `getRankConfigInit`）。
    ///
    /// ⚠️ `detail` **不**统计 `userCount`（恒 0），与 `list` 不同。
    pub async fn detail(&self) -> Result<UserRankDetailVO<UserRankListVO>> {
        let rank_config_row = self
            .repo()
            .find_config_by_code(RANK_CONFIG_CODE)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_RANK_CONFIG_MISSING))?;
        let rank_type = rank_config_row.rank_type;

        let rows = self.repo().list_ordered_by_rank_id().await?;
        if rows.is_empty() {
            return Ok(UserRankDetailVO {
                rank_type,
                user_rank_list: vec![init_rank_list_vo()],
                user_rank_config: init_rank_config_vo(),
                grow_up_setting: Some(GrowUpSettingVO::init()),
            });
        }

        let user_rank_list = rows.iter().map(|r| to_list_vo(r, 0)).collect();

        // rank_config：Java 在正常路径里会再查一次；本实现复用上方已取到的行。
        let user_rank_config = config_to_vo(&rank_config_row);

        // grow_config：仅 `rankType == 1` 时读取（Java 同）。
        // 行存在 ⇒ 解析其 data（非法/`"null"` ⇒ None）；行缺失 ⇒ 默认值。
        let grow_up_setting = if rank_type == 1 {
            match self.repo().find_config_by_code(GROW_CONFIG_CODE).await? {
                Some(cfg) => parse_grow_setting(cfg.data.as_deref()),
                None => Some(GrowUpSettingVO::init()),
            }
        } else {
            Some(GrowUpSettingVO::init())
        };

        Ok(UserRankDetailVO {
            rank_type,
            user_rank_list,
            user_rank_config,
            grow_up_setting,
        })
    }

    // ————————————————————— 更新 —————————————————————

    /// `POST /adminapi/user/userRank/update`：整体更新会员等级
    /// （对应 Java `UserRankController#update` → `UserRankServiceImpl#update`）。
    ///
    /// Java 逻辑（同一事务，见仓储
    /// [`nextrade_infrastructure::db::repo::member::user_rank::UserRankRepository::apply_edit`]）：
    /// 1. 校验入参（[`validate_edit_param`]，等价 `@Validated` + `validParam()`）；
    /// 2. 删除「本次 `data` 里没出现」的旧等级（`removeBatchByIds`）；
    /// 3. `rankId == null` 的条目**新增**、`rankId != null` 的条目**更新**
    ///    （字段快照由 [`build_user_rank_fields`] 生成，`rankType` 用参数值覆盖）；
    /// 4. `code = 'rank_config'` 行 upsert
    ///    （`rankType` 取参数、`data` 取 `toJson(param.userRankConfig)`）；
    /// 5. `growUpSetting != null && rankType == 1` 时 `code = 'grow_config'` 行 upsert。
    pub async fn update(&self, param: &UserRankEditParam) -> Result<()> {
        // ① 校验（Java `@Validated` 的 `@Size` + `validParam()`）。
        let rank_type =
            validate_edit_param(param).map_err(|msg| Error::biz(SERVICE_DATA_ERROR, msg))?;

        // ② 计算要删除的主键。
        let all = self.repo().list_all().await?;
        let all_ids: Vec<i32> = all.iter().map(|r| r.rank_id).collect();
        let data = param.data.as_deref().unwrap_or(&[]);
        let delete_ids = compute_delete_ids(&all_ids, data);

        // ③ 组装新增 / 更新。
        let mut inserts: Vec<UserRankWrite> = Vec::new();
        let mut updates: Vec<(i32, UserRankWrite)> = Vec::new();
        for d in data {
            let fields = build_user_rank_fields(d, IS_PRO, rank_type);
            let w = fields_to_write(&fields);
            match d.rank_id {
                None => inserts.push(w),
                Some(id) => updates.push((id, w)),
            }
        }

        // ④ rank_config：Java 始终 upsert（非 Pro 版提前 return，本实现不涉及）。
        let rank_config = RankConfigWrite {
            rank_type,
            data: to_json_opt(&param.user_rank_config),
        };

        // ⑤ grow_config：仅传了 growUpSetting 且 rankType == 1 时 upsert。
        let grow_config = if param.grow_up_setting.is_some() && rank_type == 1 {
            to_json_opt(&param.grow_up_setting)
        } else {
            None
        };

        let plan = RankEditPlan {
            delete_ids,
            inserts,
            updates,
            rank_config: Some(rank_config),
            grow_config,
        };
        self.repo().apply_edit(&plan).await?;
        Ok(())
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// `POST /adminapi/user/userRank/updateField`：单字段更新
    /// （对应 Java `UserRankController#updateField` →
    /// `BaseServiceImpl#updateField(updateField, USER_RANK_FIELDS)`）。
    ///
    /// Java 逻辑：
    /// 1. `field = getColumnByPropertyName(field)`（未知 / `null` 回退主键列
    ///    `rank_id`）；
    /// 2. 结果不在 [`USER_RANK_FIELDS`] 白名单 ⇒
    ///    抛 `GlobalException("field 的值有误", 1001)`；
    /// 3. `UPDATE user_rank SET <field> = ? WHERE rank_id = ?`，
    ///    `val == null` 时即 `SET <field> = NULL`。
    ///
    /// ⚠️ `id == null` 时 Java 生成 `rank_id = NULL`（恒不命中）并返回 `false`；
    /// 本实现直接返回 `0`（不执行 SQL，语义等价）。
    ///
    /// 返回受影响行数。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<u64> {
        let column = map_update_field_column(dto.field.as_deref().unwrap_or(""));
        if !USER_RANK_FIELDS.contains(&column) {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_INVALID_FIELD_VALUE));
        }
        let Some(id) = dto.id else {
            return Ok(0);
        };
        self.repo()
            .update_field(id, column, dto.val.as_deref())
            .await
    }

    // ————————————————————— 删除 —————————————————————

    /// `POST /adminapi/user/userRank/del`：删除会员等级
    /// （对应 Java `UserRankController#del` → `BaseServiceImpl#del(id)`）。
    ///
    /// `user_rank` 没有软删标记列 ⇒ 物理删除。
    /// ⚠️ `id == null` ⇒ 返回 `false`（Java 同，不执行删除）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let Some(id) = dto.id else {
            return Ok(false);
        };
        self.repo().delete(id).await
    }

    // ————————————————————— 批量 —————————————————————

    /// `POST /adminapi/user/userRank/batch`：批量操作
    /// （对应 Java `UserRankController#batch` → `BaseServiceImpl#batch`）。
    ///
    /// 只有 `type == "del"` 会执行 `removeByIds`（物理删除）；其余类型返回 0。
    /// ids 由 [`BatchDTO::effective_ids`] 去重并按 Java `removeByIds` 的口径过滤。
    ///
    /// 返回受影响行数。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<u64> {
        if !dto.is_delete() {
            return Ok(0);
        }
        let ids = dto.effective_ids();
        self.repo().delete_many(&ids).await
    }
}

/// 计算需要删除的等级主键：`all` 中未出现在 `data.rankId`（非空）里的行。
///
/// 对应 Java `UserRankServiceImpl#update` 的
/// `allUserRanks.stream().filter(item -> !rankIds.contains(item.getRankId())).toList()`。
fn compute_delete_ids(all: &[i32], data: &[UserRankData]) -> Vec<i32> {
    let keep: Vec<i32> = data.iter().filter_map(|d| d.rank_id).collect();
    all.iter()
        .copied()
        .filter(|id| !keep.contains(id))
        .collect()
}

/// [`UserRankFields`] → 仓储 [`UserRankWrite`]（字段一一对应）。
///
/// ⚠️ `show_price` 恒为 `None`：Java `buildUserRank` **从不**设置该列
/// （新增走 `create` 的 `copyProperties`，更新不走 `showPrice`）。
fn fields_to_write(f: &UserRankFields) -> UserRankWrite {
    UserRankWrite {
        rank_id: f.rank_id,
        rank_name: f.rank_name.clone(),
        min_growth_points: f.min_growth_points,
        max_growth_points: f.max_growth_points,
        discount: f.discount,
        show_price: None,
        rank_type: f.rank_type,
        rank_logo: f.rank_logo.clone(),
        rank_ico: f.rank_ico.clone(),
        rank_bg: f.rank_bg.clone(),
        rank_point: f.rank_point.clone(),
        free_shipping: f.free_shipping,
        rank_card_type: f.rank_card_type,
        rights: f.rights.clone(),
        rank_level: f.rank_level,
    }
}

/// 复刻 Java `JsonUtil.toJson(Object)`：入参为 `None` ⇒ `None`；
/// 序列化失败（理论不可达）⇒ `None`。
fn to_json_opt<T: serde::Serialize>(v: &Option<T>) -> Option<String> {
    v.as_ref().and_then(|x| serde_json::to_string(x).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use rust_decimal::Decimal;
    use nextrade_core::models::member::user_rank::{GrowUpSettingInput, RankConfigInput};

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).expect("测试常量必须是合法 Decimal")
    }

    // ————————————— to_json_opt（对照 JsonUtil.toJson） —————————————

    #[test]
    fn to_json_opt_returns_none_for_none() {
        let v: Option<RankConfigInput> = None;
        assert_eq!(to_json_opt(&v), None);
        let v: Option<GrowUpSettingInput> = None;
        assert_eq!(to_json_opt(&v), None);
    }

    #[test]
    fn to_json_opt_serializes_some_in_camel_case() {
        let v = Some(RankConfigInput {
            r#type: Some(1),
            rank_after_month: Some(12),
            use_month: Some(12),
        });
        assert_eq!(
            to_json_opt(&v).as_deref(),
            Some(r#"{"type":1,"rankAfterMonth":12,"useMonth":12}"#)
        );

        // None 字段也输出 null（Jackson 默认包含 null）
        let v = Some(RankConfigInput {
            r#type: None,
            rank_after_month: Some(3),
            use_month: None,
        });
        assert_eq!(
            to_json_opt(&v).as_deref(),
            Some(r#"{"type":null,"rankAfterMonth":3,"useMonth":null}"#)
        );
    }

    // ————————————— fields_to_write —————————————

    #[test]
    fn fields_to_write_maps_every_field_except_show_price() {
        let f = UserRankFields {
            rank_id: Some(7),
            rank_name: Some("铂金会员".into()),
            min_growth_points: Some(dec("100.00")),
            max_growth_points: Some(999),
            discount: Some(dec("9.5")),
            rank_type: Some(2),
            rank_logo: Some("logo.png".into()),
            rank_ico: Some("card2".into()),
            rank_bg: Some("bg.png".into()),
            rank_point: Some("3".into()),
            free_shipping: Some(1),
            rank_card_type: Some(2),
            rights: Some("[]".into()),
            rank_level: Some(2),
        };
        let w = fields_to_write(&f);
        assert_eq!(w.rank_id, Some(7));
        assert_eq!(w.rank_name.as_deref(), Some("铂金会员"));
        assert_eq!(w.min_growth_points, Some(dec("100.00")));
        assert_eq!(w.max_growth_points, Some(999));
        assert_eq!(w.discount, Some(dec("9.5")));
        assert_eq!(w.rank_type, Some(2));
        assert_eq!(w.rank_logo.as_deref(), Some("logo.png"));
        assert_eq!(w.rank_ico.as_deref(), Some("card2"));
        assert_eq!(w.rank_bg.as_deref(), Some("bg.png"));
        assert_eq!(w.rank_point.as_deref(), Some("3"));
        assert_eq!(w.free_shipping, Some(1));
        assert_eq!(w.rank_card_type, Some(2));
        assert_eq!(w.rights.as_deref(), Some("[]"));
        assert_eq!(w.rank_level, Some(2));
        // Java buildUserRank 不设置 showPrice
        assert_eq!(w.show_price, None);
    }

    // ————————————— compute_delete_ids —————————————

    #[test]
    fn compute_delete_ids_drops_rows_present_in_data() {
        // 已有 [1,2,3]；data 保留 2（带 id）与一个新增（id 为空）⇒ 删除 1、3
        let data = vec![
            UserRankData {
                rank_id: Some(2),
                ..Default::default()
            },
            UserRankData {
                rank_id: None,
                ..Default::default()
            },
        ];
        assert_eq!(compute_delete_ids(&[1, 2, 3], &data), vec![1, 3]);
    }

    #[test]
    fn compute_delete_ids_deletes_all_when_data_has_only_inserts() {
        let data = vec![UserRankData {
            rank_id: None,
            ..Default::default()
        }];
        assert_eq!(compute_delete_ids(&[1, 2], &data), vec![1, 2]);
    }

    #[test]
    fn compute_delete_ids_keeps_all_when_data_covers_every_row() {
        let data = vec![
            UserRankData {
                rank_id: Some(1),
                ..Default::default()
            },
            UserRankData {
                rank_id: Some(2),
                ..Default::default()
            },
        ];
        assert!(compute_delete_ids(&[1, 2], &data).is_empty());
        // 表为空 ⇒ 无删除
        assert!(compute_delete_ids(&[], &data).is_empty());
    }
}