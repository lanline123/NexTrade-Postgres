//! admin 自助热更新 DTO / VO（system::system_update，我方新增）。
//!
//! 设计见 `~/workspace/hot-update-design/REPORT.md`：
//! - §3 `release-manifest.json` 清单结构（[`ReleaseManifest`] 与之逐字段对应，
//!   由 `tools/hot-update/make_manifest.py` 在发版时生成、随统一产品 Release 发布）；
//! - §4 待执行迁移口径（清单 − 登记表；`changes` 为对 SQL 文本的语句级解析标注，
//!   前端必须如实标注「按待执行迁移解析」，不冒充实时库结构比对）；
//! - §5 任务状态机（[`UpdateJobVO::status`] 取值与 updater 的 status.json 一致）。
//!
//! JSON 字段全部 camelCase。清单中可选字段缺失时按默认处理（`#[serde(default)]`），
//! 保证旧版清单仍可被新后端读取。

use serde::{Deserialize, Serialize};

// ——————————————————————————————————————————————————————————————
// release-manifest.json（发版产物契约）
// ——————————————————————————————————————————————————————————————

/// 统一产品 Release 的版本清单（`release-manifest.json`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReleaseManifest {
    /// 产品版本号（四面统一，x.y.z）。
    pub product_version: String,
    /// 源码 commit。
    pub git_commit: String,
    /// 发布时间（ISO 8601 字符串，原样透传展示）。
    pub released_at: String,
    /// 更新说明（CHANGELOG 对应版本节全文，页面直接展示）。
    pub notes: String,
    /// 四面产物。
    pub surfaces: ManifestSurfaces,
    /// 本版本携带的迁移有序清单（执行序 = 数组序）。
    pub migrations: Vec<ManifestMigration>,
    /// 兼容约束与客户端标志。
    pub compat: ManifestCompat,
}

/// 清单内四面产物（某面本版未发则为 `null`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManifestSurfaces {
    pub backend: Option<SurfaceAsset>,
    pub admin: Option<SurfaceAsset>,
    pub pc: Option<SurfaceAsset>,
    pub h5: Option<SurfaceAsset>,
}

impl ManifestSurfaces {
    /// 按面名取产物（`backend` / `admin` / `pc` / `h5`）。
    #[must_use]
    pub fn get(&self, surface: &str) -> Option<&SurfaceAsset> {
        match surface {
            "backend" => self.backend.as_ref(),
            "admin" => self.admin.as_ref(),
            "pc" => self.pc.as_ref(),
            "h5" => self.h5.as_ref(),
            _ => None,
        }
    }
}

/// 单面产物（Release 资产）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SurfaceAsset {
    /// 资产文件名（如 `nextrade-admin.tgz`）。
    pub file: String,
    /// 资产 sha256（小写 hex）。
    pub sha256: String,
    /// 资产字节数。
    pub size: u64,
    /// 该面构建号（CI run number 等，原样展示）。
    pub build: String,
    /// 面版本戳：后端 = 内嵌版本串、PC = BUILD_ID、admin/H5 = 入口文件名。
    pub stamp: String,
}

/// 清单内一条迁移。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManifestMigration {
    /// 迁移文件名（如 `028_xxx.sql`）。
    pub file: String,
    /// 文件内容 sha256。
    pub sha256: String,
    /// 人类摘要（发版时写入，页面展示）。
    pub summary: String,
    /// 语句级解析标注（如 `加字段 product.weight`、`新增表 foo`）；
    /// 由 `tools/hot-update/classify_sql.py` 对 SQL 文本解析生成，非库结构比对。
    pub changes: Vec<String>,
    /// 是否含破坏性语句（DROP / TRUNCATE / 无 WHERE 的 DELETE/UPDATE）。
    pub destructive: bool,
    /// 执行后是否需要重启后端。
    pub requires_backend_restart: bool,
    /// SQL 预览（全文或截断，`sqlTruncated` 标明）。
    pub sql_preview: Option<String>,
    /// `sqlPreview` 是否被截断。
    pub sql_truncated: bool,
}

/// 兼容约束与客户端更新标志。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManifestCompat {
    /// 静态面更新前必须已执行的迁移文件名（已应用进度须 ≥ 该文件）；为空不拦截。
    pub min_schema: Option<String>,
    /// 本版需要小程序另行发版（只能提醒，见设计 §5.4）。
    pub requires_mini_program_update: bool,
    /// 本版需要 App 另行发版（只能提醒）。
    pub requires_app_update: bool,
}

// ——————————————————————————————————————————————————————————————
// 状态页 VO
// ——————————————————————————————————————————————————————————————

/// 单面版本对比。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SurfaceVersionVO {
    /// 面名：`backend` / `admin` / `pc` / `h5`。
    pub surface: String,
    /// 展示名：后端 / 管理后台 / PC 站 / H5。
    pub label: String,
    /// 该面站点根目录是否已配置（未配置时只能显示清单侧信息）。
    pub configured: bool,
    /// 当前线上版本戳（读不到为 null）。
    pub current_stamp: Option<String>,
    /// 当前版本戳的来源说明（如「version.json」「index.html 入口指纹」「DEPLOY_INFO」）。
    pub current_detail: Option<String>,
    /// 最新 Release 中该面的版本戳。
    pub latest_stamp: Option<String>,
    /// 是否有可用更新（两边戳都可得且不同；后端比产品版本+commit）。
    pub update_available: bool,
    /// 本期是否支持一键更新（P2 仅静态三面；P3 起后端开放，含迁移阶段）。
    pub updatable: bool,
}

/// 待执行迁移（清单 − 登记表）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingMigrationVO {
    pub file: String,
    pub sha256: String,
    pub summary: String,
    /// 语句级解析标注（口径见 [`ManifestMigration::changes`]）。
    pub changes: Vec<String>,
    pub destructive: bool,
    pub requires_backend_restart: bool,
    pub sql_preview: Option<String>,
    pub sql_truncated: bool,
}

/// 已登记迁移与清单 sha 不一致（文件被改过的告警）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShaMismatchVO {
    pub file: String,
    /// 登记表中的 sha256。
    pub registered_sha256: String,
    /// 清单/仓库文件现行 sha256。
    pub manifest_sha256: String,
}

/// 「系统更新」页状态总览。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemUpdateStatusVO {
    /// 更新源是否可用（仓库已配置且能读到令牌文件或已有缓存清单）。
    pub source_configured: bool,
    /// updater 是否已安装（spool 目录已配置）。
    pub updater_configured: bool,
    /// 迁移登记基线是否就绪（登记表存在且非空）；未就绪严禁发起任何更新。
    pub baseline_ready: bool,
    /// 当前产品版本（后端内嵌戳）。
    pub current_product_version: String,
    /// 当前后端 commit 短哈希。
    pub current_git_sha: String,
    /// 最新 Release 的产品版本（未检查/无清单为 null）。
    pub latest_product_version: Option<String>,
    /// 是否有新产品版本。
    pub update_available: bool,
    /// 四面对比（恒四行，顺序 backend/admin/pc/h5）。
    pub surfaces: Vec<SurfaceVersionVO>,
    /// 待执行迁移清单（按清单序）。
    pub pending_migrations: Vec<PendingMigrationVO>,
    /// 已登记迁移数。
    pub applied_migration_count: i64,
    /// sha 不一致告警（已登记文件被改）。
    pub sha_mismatches: Vec<ShaMismatchVO>,
    /// 最新版是否要求小程序另行发版（提醒口径）。
    pub requires_mini_program_update: bool,
    /// 最新版是否要求 App 另行发版（提醒口径）。
    pub requires_app_update: bool,
    /// 最新版更新说明。
    pub release_notes: Option<String>,
    /// 最近一次成功拉取清单的时刻（Unix 秒；0 = 从未）。
    pub last_checked_at: i64,
    /// 进行中的 job（无则 null）。
    pub active_job: Option<UpdateJobVO>,
}

// ——————————————————————————————————————————————————————————————
// 更新任务
// ——————————————————————————————————————————————————————————————

/// 更新任务（DB 审计行与 updater status.json 合并后的视图）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateJobVO {
    pub job_id: String,
    pub product_version: String,
    pub surfaces: Vec<String>,
    /// 状态机：queued/downloading/verifying/backing_up/migrating/swapping/
    /// health_checking/done/failed/migration_failed/rolling_back/
    /// rolled_back/rollback_failed（migrating 与 migration_failed 为 P3 新增：
    /// 前者在 backing_up 与 swapping 之间执行迁移，后者为「停在迁移阶段、
    /// 未换任何面、待人工」的终态）。
    pub status: String,
    pub stage: String,
    pub message: String,
    pub operator_name: String,
    pub destructive_ack: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl UpdateJobVO {
    /// 是否处于进行中状态（前端据此轮询）。
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self.status.as_str(),
            "queued"
                | "downloading"
                | "verifying"
                | "backing_up"
                | "migrating"
                | "swapping"
                | "health_checking"
                | "rolling_back"
        )
    }
}

/// 发起更新入参（二次确认：须重输当前登录密码）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateUpdateJobParam {
    /// 目标产品版本（必须与当前最新清单一致，防执行时漂移）。
    pub target_version: String,
    /// 选中面（P2 仅允许 admin/pc/h5 的子集，非空）。
    pub surfaces: Vec<String>,
    /// 当前登录管理员密码（再认证，不落库不进日志）。
    pub password: String,
    /// 破坏性迁移二次确认位（目标版本含未执行的破坏性迁移时必须为 true，
    /// P2 不执行迁移，此位为审计留痕与前置知情确认；P3 起为执行门）。
    pub confirm_destructive: bool,
}

/// `sys_update_job` 审计行（仓储层用）。
#[derive(Debug, Clone, Default)]
pub struct UpdateJobRow {
    pub job_id: String,
    pub product_version: String,
    pub manifest_sha256: String,
    pub manifest_snapshot: String,
    /// 逗号分隔的面名。
    pub surfaces: String,
    pub status: String,
    pub stage: String,
    pub message: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub destructive_ack: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&UpdateJobRow> for UpdateJobVO {
    fn from(row: &UpdateJobRow) -> Self {
        Self {
            job_id: row.job_id.clone(),
            product_version: row.product_version.clone(),
            surfaces: row
                .surfaces
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            status: row.status.clone(),
            stage: row.stage.clone(),
            message: row.message.clone(),
            operator_name: row.operator_name.clone(),
            destructive_ack: row.destructive_ack,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_deserializes_with_missing_optionals() {
        let raw = r#"{
            "productVersion": "0.0.3",
            "gitCommit": "abc123",
            "surfaces": {"admin": {"file": "nextrade-admin.tgz", "sha256": "ff", "size": 10, "build": "42", "stamp": "index-X.js"}},
            "migrations": [{"file": "028_a.sql", "sha256": "ee", "summary": "加字段", "changes": ["加字段 product.weight"], "destructive": false, "requiresBackendRestart": true}]
        }"#;
        let m: ReleaseManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(m.product_version, "0.0.3");
        assert!(m.surfaces.backend.is_none());
        assert_eq!(m.surfaces.admin.as_ref().unwrap().stamp, "index-X.js");
        assert_eq!(m.migrations.len(), 1);
        assert!(m.migrations[0].sql_preview.is_none());
        assert!(!m.compat.requires_app_update);
    }

    #[test]
    fn job_vo_splits_surfaces_and_active_states() {
        let row = UpdateJobRow {
            job_id: "j1".into(),
            surfaces: "admin, h5".into(),
            status: "swapping".into(),
            ..UpdateJobRow::default()
        };
        let vo = UpdateJobVO::from(&row);
        assert_eq!(vo.surfaces, vec!["admin".to_string(), "h5".to_string()]);
        assert!(vo.is_active());
        let done = UpdateJobVO {
            status: "done".into(),
            ..UpdateJobVO::default()
        };
        assert!(!done.is_active());
        // P3：迁移阶段为进行中；迁移失败终态不是进行中（待人工，不轮询等待）
        let migrating = UpdateJobVO {
            status: "migrating".into(),
            ..UpdateJobVO::default()
        };
        assert!(migrating.is_active());
        let migration_failed = UpdateJobVO {
            status: "migration_failed".into(),
            ..UpdateJobVO::default()
        };
        assert!(!migration_failed.is_active());
    }
}
