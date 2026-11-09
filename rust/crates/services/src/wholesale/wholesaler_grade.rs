//! 批发商等级域服务（wholesale::wholesaler_grade）。
//!
//! 对应原 Java `com.nextrade.service.wholesale.impl.WholesalerGradeServiceImpl`
//! （189 行）。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java | 方法 |
//! |---------------------------|---------|------|
//! | `GET  /wholesale/grade/list` | `WholesalerGradeController.list` | [`WholesalerGradeService::list`] |
//! | `POST /wholesale/grade/save` | `WholesalerGradeController.save` | [`WholesalerGradeService::save`] |
//! | `POST /wholesale/grade/setStatus` | `WholesalerGradeController.setStatus` | [`WholesalerGradeService::set_status`] |
//! | `POST /wholesale/grade/delete` | `WholesalerGradeController.delete` | [`WholesalerGradeService::delete`] |
//!
//! # 语义（逐条对照 Java）
//!
//! - `list`：可选 `status` 过滤 → 回填 `userCount`（含禁用批发商，
//!   `wholesaler.is_delete = 0`）与 `isDefault`（= 配置
//!   `wholesaleDefaultGradeId`）→ `addTime` 东八区格式化。
//! - `save`：等级名 trim 必填、≤50；同名（未删，编辑排除自身）→ 409；
//!   新增默认 `sort_order=50` / `status=1` / `remark=''`；编辑仅覆盖传入字段。
//! - `setStatus`：仅允许 0/1，等级不存在 → 5303。
//! - `delete`：软删；默认等级 → 5306；等级下仍有批发商 → 5305。
//!
//! # 与 Java 的差异（`uncertain_points`）
//!
//! 1. **默认等级读取**：Java 经 `ObjectProvider<WholesaleSettingService>`
//!    惰性取值；本实现由仓储直接读 `config.wholesaleDefaultGradeId`，
//!    语义等价且无循环依赖。
//! 2. **`delete` 的两次写**：Java 先 `updateById` 刷 `update_time` 再
//!    `deleteById`；本实现合并为一条 `UPDATE ... SET is_delete=1,
//!    update_time=?`，净效果相同（少一次写）。
//! 3. **错误码**：`Error::biz(code, msg)` 保留 Java `GlobalException`
//!    的业务码（5303/5304/5305/5306/409）。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use std::sync::Arc;

use nextrade_core::entities::WholesalerGrade;
use nextrade_core::models::wholesale::wholesaler_grade::{
    format_time, validate_grade_name, validate_remark, validate_status, WholesalerGradeListQuery,
    WholesalerGradeSaveDTO, WholesalerGradeVO, CODE_BAD_REQUEST, CODE_CONFLICT,
    CODE_GRADE_CANNOT_DELETE, CODE_GRADE_HAS_BINDING, CODE_GRADE_NOT_FOUND,
    DEFAULT_SORT_ORDER, MSG_GRADE_CANNOT_DELETE, MSG_GRADE_HAS_BINDING, MSG_GRADE_NAME_DUPLICATED,
    MSG_GRADE_NOT_FOUND,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::wholesale::wholesaler_grade::{
    GradeListRow, WholesalerGradeRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 批发商等级域服务（挂载于 `AdminState.wholesale.wholesaler_grade`）。
pub struct WholesalerGradeService {
    repos: Arc<Repositories>,
}

impl WholesalerGradeService {
    /// 构造（与同域其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &WholesalerGradeRepository {
        &self.repos.wholesale.wholesaler_grade
    }

    /// 等级列表（Java `WholesalerGradeServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn list(&self, q: &WholesalerGradeListQuery) -> Result<Vec<WholesalerGradeVO>> {
        let mut rows = self.repo().list(q.status, q.with_user_count()).await?;
        if q.with_user_count() {
            let ids: Vec<i32> = rows.iter().map(|r| r.grade_id).collect();
            let counts: std::collections::HashMap<i32, i64> = self
                .repo()
                .count_by_grade_ids(&ids)
                .await?
                .into_iter()
                .collect();
            for row in &mut rows {
                row.user_count = counts.get(&row.grade_id).copied().unwrap_or(0);
            }
        }
        let default_grade_id = self.repo().default_grade_id().await?;
        Ok(rows
            .iter()
            .map(|row| to_vo(row, default_grade_id))
            .collect())
    }

    /// 新增或编辑等级（Java `WholesalerGradeServiceImpl#save`），返回 `gradeId`。
    ///
    /// # Errors
    /// - 名称 / 备注校验失败 → `Error::Validation(<Java 注解文案>)`
    /// - 同名重复 → `Error::biz(409, 等级名已存在)`
    /// - 编辑目标不存在 → `Error::biz(5303, 批发等级不存在)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn save(&self, dto: &WholesalerGradeSaveDTO) -> Result<i32> {
        let grade_name = validate_grade_name(dto.grade_name.as_deref())
            .map_err(Error::Validation)?;
        validate_remark(dto.remark.as_deref()).map_err(Error::Validation)?;

        // 同名重复校验（编辑排除自身）
        if self
            .repo()
            .name_exists(&grade_name, dto.grade_id)
            .await?
        {
            return Err(Error::biz(CODE_CONFLICT, MSG_GRADE_NAME_DUPLICATED));
        }

        let now = now_secs();
        let Some(grade_id) = dto.grade_id else {
            // 新增：status=1 / is_delete=0 / sort_order 默认 50 / remark 默认空串
            let entity = WholesalerGrade {
                grade_id: 0,
                grade_name,
                status: 1,
                sort_order: dto.sort_order.unwrap_or(DEFAULT_SORT_ORDER),
                remark: dto.remark.clone().unwrap_or_default(),
                is_delete: 0,
                create_time: Some(now),
                update_time: Some(now),
            };
            return self.repo().insert(&entity).await;
        };

        let Some(mut entity) = self.repo().find_by_id(grade_id).await? else {
            return Err(Error::biz(CODE_GRADE_NOT_FOUND, MSG_GRADE_NOT_FOUND));
        };
        // 编辑：仅覆盖传入字段（Java `if (dto.getSortOrder() != null)` 同口径）
        entity.grade_name = grade_name;
        if let Some(sort_order) = dto.sort_order {
            entity.sort_order = sort_order;
        }
        if let Some(remark) = dto.remark.clone() {
            entity.remark = remark;
        }
        entity.update_time = Some(now);
        self.repo().update(&entity).await?;
        Ok(entity.grade_id)
    }

    /// 等级启停（Java `WholesalerGradeServiceImpl#setStatus`）。
    ///
    /// # Errors
    /// - 非法 status → `Error::biz(400, status 必须为 0 或 1)`
    /// - 等级不存在 → `Error::biz(5303, 批发等级不存在)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn set_status(&self, grade_id: i32, status: Option<i32>) -> Result<()> {
        let status = validate_status(status).map_err(|msg| Error::biz(CODE_BAD_REQUEST, msg))?;
        let Some(mut entity) = self.repo().find_by_id(grade_id).await? else {
            return Err(Error::biz(CODE_GRADE_NOT_FOUND, MSG_GRADE_NOT_FOUND));
        };
        entity.status = status;
        entity.update_time = Some(now_secs());
        self.repo().update(&entity).await?;
        tracing::info!("批发等级启停: grade_id={grade_id}, status={status}");
        Ok(())
    }

    /// 删除等级（软删，Java `WholesalerGradeServiceImpl#delete`）。
    ///
    /// # Errors
    /// - 等级不存在 → `Error::biz(5303, 批发等级不存在)`
    /// - 默认等级 → `Error::biz(5306, 默认批发等级不可删除，...)`
    /// - 仍有绑定批发商 → `Error::biz(5305, 该等级下已有批发商用户。...)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn delete(&self, grade_id: i32) -> Result<()> {
        let Some(_entity) = self.repo().find_by_id(grade_id).await? else {
            return Err(Error::biz(CODE_GRADE_NOT_FOUND, MSG_GRADE_NOT_FOUND));
        };
        // 默认等级保护（CR-20260610-2210 用户决策 block_default）
        let default_grade_id = self.repo().default_grade_id().await?;
        if default_grade_id > 0 && default_grade_id == grade_id {
            return Err(Error::biz(CODE_GRADE_CANNOT_DELETE, MSG_GRADE_CANNOT_DELETE));
        }
        // 绑定批发商拦截（含禁用；is_delete = 0 口径与列表 userCount 一致）
        if self.repo().count_wholesalers(grade_id).await? > 0 {
            return Err(Error::biz(CODE_GRADE_HAS_BINDING, MSG_GRADE_HAS_BINDING));
        }
        self.repo().soft_delete(grade_id, now_secs()).await?;
        tracing::info!("删除批发等级（软删）: grade_id={grade_id}");
        Ok(())
    }
}

/// 列表行 → VO（Java `BeanUtils.copyProperties` + userCount / isDefault /
/// addTime 回填）。
#[must_use]
fn to_vo(row: &GradeListRow, default_grade_id: i32) -> WholesalerGradeVO {
    WholesalerGradeVO {
        grade_id: row.grade_id,
        grade_name: row.grade_name.clone(),
        status: row.status,
        sort_order: row.sort_order,
        user_count: i32::try_from(row.user_count).unwrap_or(i32::MAX),
        is_default: default_grade_id > 0 && default_grade_id == row.grade_id,
        remark: row.remark.clone(),
        add_time: format_time(row.create_time),
    }
}

/// 当前 Unix 秒（`wholesaler_grade.create_time` / `update_time` 为 bigint 秒级）。
fn now_secs() -> i64 {
    i64::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(grade_id: i32) -> GradeListRow {
        GradeListRow {
            grade_id,
            grade_name: "一级批发商".into(),
            status: 1,
            sort_order: 50,
            remark: "默认".into(),
            is_delete: 0,
            create_time: Some(1_700_000_000),
            update_time: Some(1_700_000_000),
            user_count: 3,
        }
    }

    /// VO 映射：isDefault 仅等于配置默认等级时为 true。
    #[test]
    fn to_vo_marks_default_grade() {
        let vo = to_vo(&row(2), 2);
        assert!(vo.is_default);
        assert_eq!(vo.user_count, 3);
        assert_eq!(vo.add_time.as_deref(), Some("2023-11-15 06:13:20"));

        let vo2 = to_vo(&row(2), 5);
        assert!(!vo2.is_default, "非默认等级");
        // 未配置默认等级（0）时全部非默认
        assert!(!to_vo(&row(2), 0).is_default);
    }

    /// userCount 的 long → int 饱和转换（Entity 侧是 i64 计数）。
    #[test]
    fn to_vo_saturates_user_count() {
        let mut r = row(1);
        r.user_count = i64::from(i32::MAX) + 10;
        assert_eq!(to_vo(&r, 0).user_count, i32::MAX);
    }
}
