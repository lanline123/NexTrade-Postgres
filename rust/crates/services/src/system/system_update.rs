//! admin 自助热更新服务（system::system_update，我方新增）。
//!
//! 设计见 `~/workspace/hot-update-design/REPORT.md`：
//! - **P1 版本感知**：拉取统一产品 Release 的 `release-manifest.json`（缓存于
//!   进程内，手动刷新 + 超过 24 小时在状态查询时惰性重拉 = 每日自动检查的
//!   落地形态，只提醒不执行），与四面本地版本戳、迁移登记表比对，产出
//!   [`SystemUpdateStatusVO`]；
//! - **P2 静态三面一键更新**：本服务只负责**发起**——校验（平台口径在路由层、
//!   密码再认证、基线就绪、版本钉死、minSchema 门、破坏性知情确认、全局单
//!   job）后把 job 审计行落库并把 job 描述以 spool 文件交给独立 updater
//!   执行（下载/校验/备份/换面/健康检查/回滚全在 updater，本进程不自改）；
//! - **P3 后端 + 迁移一键更新**：选中面可含 `backend`；含 backend 的 job
//!   在 spool 文件中带 `includeMigrations=true`，updater 在换任何面之前
//!   先经迁移阶段（pg_dump 备份门 → 逐文件事务执行 → 登记，见
//!   `tools/hot-update/migrate-runner.sh`），迁移失败 job 落终态
//!   `migration_failed`、不换任何面；后端换版由 updater 走
//!   `tools/hot-update/backend-swap.sh`（版本端点严验、失败自动回滚
//!   程序面；schema 只进不退）。破坏性确认位自 P3 起同时是 updater 的
//!   执行门（spool 文件 `destructiveAck`）。
//!   job 进度由 updater 写 `status/<job_id>.json`，本服务读回同步落库供页面轮询。
//!
//! 安全红线（设计 §8 对照）：
//! - 更新源只认配置仓库的 Releases，job 钉死创建时的清单快照与 sha256；
//! - GitHub 令牌只从 `token_file` 指向的 root-only 文件读，永不回显/落库/进日志；
//! - 无基线（登记表为空）严禁发起任何 job；后端面与迁移执行不在 P2 范围，
//!   传入即明确拒绝，不静默降级。
//!
//! 本地版本戳读取口径（与 `tools/hot-update/write_version_json.mjs` 契约一致）：
//! 优先 `<根>/version.json` 的 `stamp`；缺失时降级指纹——admin 读 `index.html`
//! 入口文件名、PC 读 `DEPLOY_INFO.txt` 的 buildId（再降 `.next/BUILD_ID`）、
//! H5 读 `index.html` 的 `app.<hash>.js` 入口名。降级口径在 `currentDetail`
//! 中如实标注。

use std::collections::HashSet;
use std::sync::Arc;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use nextrade_common::config::UpdateConfig;
use nextrade_core::models::system::system_update::{
    CreateUpdateJobParam, PendingMigrationVO, ReleaseManifest, ShaMismatchVO, SurfaceVersionVO,
    SystemUpdateStatusVO, UpdateJobRow, UpdateJobVO,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;
use nextrade_security::verify_password;

/// 清单资产文件名（统一产品 Release 必备资产）。
const MANIFEST_ASSET: &str = "release-manifest.json";
/// 产品 Release 的 tag 前缀（与各面 `*-build-<run#>` 构建 Release 区分）。
const PRODUCT_TAG_PREFIX: &str = "v";
/// 清单缓存有效期：24 小时（每日自动检查口径）。
const MANIFEST_TTL_SECS: i64 = 24 * 3600;
/// 允许一键更新的面（P2 静态三面 + P3 后端）。
const UPDATABLE_SURFACES: [&str; 4] = ["admin", "pc", "h5", "backend"];

/// 进程内清单缓存。
#[derive(Clone)]
struct CachedManifest {
    manifest: ReleaseManifest,
    /// 清单原文（job 快照与 sha 计算的唯一依据）。
    raw: String,
    /// 清单原文 sha256。
    sha256: String,
    fetched_at: i64,
}

/// updater 写回的 job 状态文件（`status/<job_id>.json`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct UpdaterStatus {
    status: String,
    stage: String,
    message: String,
    updated_at: i64,
}

/// 热更新服务。
pub struct SystemUpdateService {
    repos: Arc<Repositories>,
    cfg: UpdateConfig,
    cache: RwLock<Option<CachedManifest>>,
}

impl SystemUpdateService {
    pub fn new(repos: Arc<Repositories>, cfg: UpdateConfig) -> Self {
        Self {
            repos,
            cfg,
            cache: RwLock::new(None),
        }
    }

    // ————————————————————————————————
    // P1：状态总览
    // ————————————————————————————————

    /// 状态总览。`refresh=true` 时强制重拉清单；否则缓存过期（>24h）且开启
    /// 自动检查时惰性重拉。拉取失败不报错：保留旧缓存/空清单降级展示。
    pub async fn status(&self, refresh: bool) -> Result<SystemUpdateStatusVO> {
        if refresh || self.cache_stale().await {
            if let Err(e) = self.fetch_manifest().await {
                tracing::warn!("拉取更新清单失败（降级展示）: {e}");
            }
        }
        let cached = self.cache.read().await.clone();
        let applied = self
            .repos
            .system
            .system_update
            .list_applied_migrations()
            .await?;
        let applied_set: HashSet<&str> = applied.iter().map(|(f, _)| f.as_str()).collect();
        let baseline_ready = !applied.is_empty();

        let mut vo = SystemUpdateStatusVO {
            source_configured: self.token().is_some() || cached.is_some(),
            updater_configured: !self.cfg.spool_dir.trim().is_empty(),
            baseline_ready,
            current_product_version: nextrade_common::version::PRODUCT_VERSION.to_string(),
            current_git_sha: nextrade_common::version::git_sha_short(),
            applied_migration_count: applied.len() as i64,
            ..SystemUpdateStatusVO::default()
        };

        vo.surfaces = self.surface_versions(cached.as_ref());

        if let Some(c) = &cached {
            let m = &c.manifest;
            vo.latest_product_version = Some(m.product_version.clone());
            vo.update_available = m.product_version != vo.current_product_version;
            vo.last_checked_at = c.fetched_at;
            vo.release_notes = Some(m.notes.clone());
            vo.requires_mini_program_update = m.compat.requires_mini_program_update;
            vo.requires_app_update = m.compat.requires_app_update;
            vo.pending_migrations = m
                .migrations
                .iter()
                .filter(|mig| !applied_set.contains(mig.file.as_str()))
                .map(|mig| PendingMigrationVO {
                    file: mig.file.clone(),
                    sha256: mig.sha256.clone(),
                    summary: mig.summary.clone(),
                    changes: mig.changes.clone(),
                    destructive: mig.destructive,
                    requires_backend_restart: mig.requires_backend_restart,
                    sql_preview: mig.sql_preview.clone(),
                    sql_truncated: mig.sql_truncated,
                })
                .collect();
            let manifest_sha: std::collections::HashMap<&str, &str> = m
                .migrations
                .iter()
                .map(|mig| (mig.file.as_str(), mig.sha256.as_str()))
                .collect();
            vo.sha_mismatches = applied
                .iter()
                .filter_map(|(file, sha)| {
                    let expected = manifest_sha.get(file.as_str())?;
                    if !sha.eq_ignore_ascii_case(expected) {
                        Some(ShaMismatchVO {
                            file: file.clone(),
                            registered_sha256: sha.clone(),
                            manifest_sha256: (*expected).to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }

        if let Some(job) = self
            .repos
            .system
            .system_update
            .find_active_job()
            .await?
        {
            let job = self.sync_job(job).await;
            vo.active_job = Some(UpdateJobVO::from(&job));
        }
        Ok(vo)
    }

    /// 四面版本对比（顺序恒 backend/admin/pc/h5）。
    fn surface_versions(&self, cached: Option<&CachedManifest>) -> Vec<SurfaceVersionVO> {
        let manifest = cached.map(|c| &c.manifest);
        let backend_stamp = nextrade_common::version::git_sha().to_string();
        let mut out = Vec::with_capacity(4);
        out.push(SurfaceVersionVO {
            surface: "backend".into(),
            label: "后端（Rust）".into(),
            configured: true,
            current_stamp: Some(backend_stamp.clone()),
            current_detail: Some(format!(
                "内嵌版本戳 v{} · build {}",
                nextrade_common::version::PRODUCT_VERSION,
                nextrade_common::version::build_number()
            )),
            latest_stamp: manifest
                .and_then(|m| m.surfaces.backend.as_ref())
                .map(|a| a.stamp.clone()),
            update_available: manifest
                .and_then(|m| m.surfaces.backend.as_ref())
                .is_some_and(|a| !a.stamp.is_empty() && a.stamp != backend_stamp),
            updatable: true, // P3 开放（含迁移阶段；发起仍受基线/单 job 等门约束）
        });
        for (surface, label, root) in [
            ("admin", "管理后台", self.cfg.admin_root.as_str()),
            ("pc", "PC 站", self.cfg.pc_root.as_str()),
            ("h5", "H5 移动端", self.cfg.h5_root.as_str()),
        ] {
            let (current_stamp, current_detail) = if root.trim().is_empty() {
                (None, None)
            } else {
                read_surface_stamp(surface, root)
            };
            let latest_stamp = manifest
                .and_then(|m| m.surfaces.get(surface))
                .map(|a| a.stamp.clone());
            let update_available = match (&current_stamp, &latest_stamp) {
                (Some(cur), Some(lat)) => cur != lat,
                _ => false,
            };
            out.push(SurfaceVersionVO {
                surface: surface.into(),
                label: label.into(),
                configured: !root.trim().is_empty(),
                current_stamp,
                current_detail,
                latest_stamp,
                update_available,
                updatable: true,
            });
        }
        out
    }

    // ————————————————————————————————
    // P2：发起 / 查询 job
    // ————————————————————————————————

    /// 发起一次更新 job（P2 静态三面 / P3 可含后端+迁移；校验链见模块头注释）。
    pub async fn create_job(
        &self,
        param: &CreateUpdateJobParam,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<UpdateJobVO> {
        // 1) updater 未安装（spool 未配置）→ 只能感知不能执行
        if self.cfg.spool_dir.trim().is_empty() {
            return Err(Error::biz_msg(
                "updater 未安装（update.spool_dir 未配置），当前仅支持版本感知",
            ));
        }
        // 2) 密码再认证
        let user = self
            .repos
            .identity
            .auth
            .find_by_id(operator_id)
            .await?
            .ok_or_else(|| Error::NotFound("管理员不存在".into()))?;
        if !verify_password(&param.password, user.password.as_str())? {
            return Err(Error::biz(400, "密码不正确，无法发起更新"));
        }
        // 3) 基线就绪（无基线严禁执行，设计 §4.1）
        let applied = self
            .repos
            .system
            .system_update
            .list_applied_migrations()
            .await?;
        if applied.is_empty() {
            return Err(Error::biz_msg(
                "迁移登记基线未就绪（sys_schema_migration 为空），请先执行迁移 027 并完成基线回填后再发起更新",
            ));
        }
        let applied_set: HashSet<&str> = applied.iter().map(|(f, _)| f.as_str()).collect();
        // 4) 全局单 job
        if self
            .repos
            .system
            .system_update
            .find_active_job()
            .await?
            .is_some()
        {
            return Err(Error::biz_msg("已有更新任务进行中，请等待其结束"));
        }
        // 5) 清单必须在手且版本钉死一致（禁止执行时漂移）
        if self.cache.read().await.is_none() {
            self.fetch_manifest().await.map_err(|e| {
                Error::biz_msg(format!("无法获取最新版本清单：{e}"))
            })?;
        }
        let cached = self
            .cache
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::biz_msg("最新版本清单不可用，请先刷新检查"))?;
        let manifest = &cached.manifest;
        if param.target_version != manifest.product_version {
            return Err(Error::biz_msg(
                "目标版本与最新清单不一致（可能已有更新的版本），请刷新后重新确认",
            ));
        }
        // 6) 面校验：非空、仅可更新面（P3 起含 backend）、清单中必须有产物
        if param.surfaces.is_empty() {
            return Err(Error::Validation("至少选择一个要更新的面".into()));
        }
        let includes_backend = param.surfaces.iter().any(|s| s == "backend");
        for s in &param.surfaces {
            if !UPDATABLE_SURFACES.contains(&s.as_str()) {
                return Err(Error::Validation(format!("未知更新面：{s}")));
            }
            if manifest.surfaces.get(s).is_none() {
                return Err(Error::biz_msg(format!(
                    "最新版本未包含 {s} 面产物，无法更新该面"
                )));
            }
        }
        // 7) minSchema 门：清单声明的前置迁移未执行 → 拦截静态面更新。
        //    含 backend 的 job 例外：其迁移阶段会按清单序执行全部待执行
        //    迁移，minSchema 若在清单内即被本 job 满足；若 minSchema 不在
        //    本版清单且未登记，本 job 也满足不了，照样拦截。
        if let Some(min_schema) = &manifest.compat.min_schema {
            if !applied_set.contains(min_schema.as_str()) {
                let covered_by_this_job = includes_backend
                    && manifest
                        .migrations
                        .iter()
                        .any(|m| m.file == *min_schema);
                if !covered_by_this_job {
                    return Err(Error::biz_msg(format!(
                        "最新版本要求先执行迁移 {min_schema}（当前未登记），请先通过后端部署轮完成该迁移后再更新静态面"
                    )));
                }
            }
        }
        // 8) 破坏性知情确认：目标版本含未执行的破坏性迁移时必须勾选
        let destructive_pending: Vec<&str> = manifest
            .migrations
            .iter()
            .filter(|m| m.destructive && !applied_set.contains(m.file.as_str()))
            .map(|m| m.file.as_str())
            .collect();
        if !destructive_pending.is_empty() && !param.confirm_destructive {
            return Err(Error::biz_msg(format!(
                "目标版本包含破坏性迁移（{}），请在确认页单独勾选知情确认后再发起",
                destructive_pending.join("、")
            )));
        }

        // 9) 落审计行 + 写 spool job 文件（文件失败则审计行置 failed，不留假排队）
        let now = chrono::Utc::now().timestamp();
        let job_id = uuid::Uuid::new_v4().to_string();
        let row = UpdateJobRow {
            job_id: job_id.clone(),
            product_version: manifest.product_version.clone(),
            manifest_sha256: cached.sha256.clone(),
            manifest_snapshot: cached.raw.clone(),
            surfaces: param.surfaces.join(","),
            status: "queued".into(),
            stage: "queued".into(),
            message: "已创建，等待 updater 领取".into(),
            operator_id: i64::from(operator_id),
            operator_name: operator_name.to_string(),
            destructive_ack: param.confirm_destructive,
            created_at: now,
            updated_at: now,
        };
        self.repos.system.system_update.insert_job(&row).await?;

        let job_file = serde_json::json!({
            "jobId": job_id,
            "productVersion": manifest.product_version,
            "releaseTag": format!("v{}", manifest.product_version),
            "repo": self.cfg.github_repo,
            "createdAt": now,
            "operator": operator_name,
            "surfaces": param.surfaces,
            "schemaReady": true,
            // P3：含 backend 的 job 由 updater 先跑迁移阶段；破坏性确认位
            // 同时是迁移执行门（migrate-runner 复扫 + 此位双闸）。
            "includeMigrations": includes_backend,
            "destructiveAck": param.confirm_destructive,
            "manifestSha256": cached.sha256,
            "manifest": manifest,
        });
        if let Err(e) = self.write_spool_job(&job_id, &job_file) {
            let _ = self
                .repos
                .system
                .system_update
                .update_job_progress(
                    &job_id,
                    "failed",
                    "spool",
                    &format!("写入 updater 任务文件失败：{e}"),
                    chrono::Utc::now().timestamp(),
                )
                .await;
            return Err(Error::biz_msg(format!(
                "写入 updater 任务文件失败：{e}"
            )));
        }

        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_current_log(
                "admin",
                0,
                &format!(
                    "发起系统热更新至 {}（面：{}）",
                    manifest.product_version,
                    param.surfaces.join(",")
                ),
            )
            .await
        {
            tracing::warn!("写管理操作日志失败（系统更新）: {e}");
        }
        Ok(UpdateJobVO::from(&row))
    }

    /// 查单个 job（先同步 updater 状态）。
    pub async fn get_job(&self, job_id: &str) -> Result<UpdateJobVO> {
        let row = self
            .repos
            .system
            .system_update
            .find_job(job_id)
            .await?
            .ok_or_else(|| Error::NotFound("更新任务不存在".into()))?;
        let row = self.sync_job(row).await;
        Ok(UpdateJobVO::from(&row))
    }

    /// 最近 job 列表（活跃者同步状态）。
    pub async fn list_jobs(&self, limit: i64) -> Result<Vec<UpdateJobVO>> {
        let rows = self.repos.system.system_update.list_jobs(limit).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let row = if UpdateJobVO::from(&row).is_active() {
                self.sync_job(row).await
            } else {
                row
            };
            out.push(UpdateJobVO::from(&row));
        }
        Ok(out)
    }

    // ————————————————————————————————
    // 内部：清单拉取 / 状态同步 / 版本戳
    // ————————————————————————————————

    async fn cache_stale(&self) -> bool {
        if !self.cfg.auto_check {
            return false;
        }
        match self.cache.read().await.as_ref() {
            None => true,
            Some(c) => chrono::Utc::now().timestamp() - c.fetched_at > MANIFEST_TTL_SECS,
        }
    }

    /// 从 GitHub Releases 拉取最新统一产品 Release 的清单并缓存。
    ///
    /// 选版规则：按发布时间倒序取第一个 tag 以 `v` 开头且带
    /// `release-manifest.json` 资产的 Release（各面 `*-build-<run#>` 构建
    /// Release 不带清单，天然跳过）。资产下载走 API + 手工 302 两段式，
    /// 令牌不随重定向外泄（与 tools/fetch_gh_artifact.py 同流程）。
    async fn fetch_manifest(&self) -> Result<()> {
        let repo = self.cfg.github_repo.trim().trim_matches('/');
        if repo.is_empty() || !repo.contains('/') {
            return Err(Error::biz_msg("更新源仓库未配置（update.github_repo）"));
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::biz_msg(format!("HTTP 客户端初始化失败：{e}")))?;
        let token = self.token();

        let list_url = format!("https://api.github.com/repos/{repo}/releases?per_page=30");
        let mut req = client
            .get(&list_url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "nextrade-system-update");
        if let Some(t) = &token {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("拉取 Release 列表失败：{e}")))?;
        if !resp.status().is_success() {
            return Err(Error::biz_msg(format!(
                "拉取 Release 列表失败：HTTP {}",
                resp.status()
            )));
        }
        let releases: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::biz_msg(format!("解析 Release 列表失败：{e}")))?;

        let mut asset_id: Option<i64> = None;
        if let Some(arr) = releases.as_array() {
            for rel in arr {
                let tag = rel["tag_name"].as_str().unwrap_or_default();
                if !tag.starts_with(PRODUCT_TAG_PREFIX) {
                    continue;
                }
                if let Some(assets) = rel["assets"].as_array() {
                    if let Some(a) = assets
                        .iter()
                        .find(|a| a["name"].as_str() == Some(MANIFEST_ASSET))
                    {
                        asset_id = a["id"].as_i64();
                        break;
                    }
                }
            }
        }
        let asset_id = asset_id.ok_or_else(|| {
            Error::biz_msg("未找到带 release-manifest.json 的产品 Release（tag 形如 v0.0.3）")
        })?;

        let asset_url =
            format!("https://api.github.com/repos/{repo}/releases/assets/{asset_id}");
        let mut req = client
            .get(&asset_url)
            .header("Accept", "application/octet-stream")
            .header("User-Agent", "nextrade-system-update");
        if let Some(t) = &token {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("下载清单失败：{e}")))?;
        let raw = if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| Error::biz_msg("清单下载重定向缺少 Location"))?
                .to_string();
            let resp2 = client
                .get(&location)
                .header("User-Agent", "nextrade-system-update")
                .send()
                .await
                .map_err(|e| Error::biz_msg(format!("下载清单失败：{e}")))?;
            if !resp2.status().is_success() {
                return Err(Error::biz_msg(format!(
                    "下载清单失败：HTTP {}",
                    resp2.status()
                )));
            }
            resp2
                .text()
                .await
                .map_err(|e| Error::biz_msg(format!("读取清单失败：{e}")))?
        } else if resp.status().is_success() {
            resp.text()
                .await
                .map_err(|e| Error::biz_msg(format!("读取清单失败：{e}")))?
        } else {
            return Err(Error::biz_msg(format!(
                "下载清单失败：HTTP {}",
                resp.status()
            )));
        };

        let manifest: ReleaseManifest = serde_json::from_str(&raw)
            .map_err(|e| Error::biz_msg(format!("清单格式非法：{e}")))?;
        if manifest.product_version.is_empty() {
            return Err(Error::biz_msg("清单缺少 productVersion"));
        }
        let sha256 = sha256_hex(raw.as_bytes());
        *self.cache.write().await = Some(CachedManifest {
            manifest,
            raw,
            sha256,
            fetched_at: chrono::Utc::now().timestamp(),
        });
        Ok(())
    }

    /// 读令牌（root-only 文件；读不到返回 None，永不回显内容）。
    fn token(&self) -> Option<String> {
        let path = self.cfg.token_file.trim();
        if path.is_empty() {
            return None;
        }
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// 原子写 spool job 文件（tmp + rename），目录不存在时创建。
    fn write_spool_job(&self, job_id: &str, job: &serde_json::Value) -> std::io::Result<()> {
        let dir = std::path::Path::new(self.cfg.spool_dir.trim()).join("jobs");
        std::fs::create_dir_all(&dir)?;
        let tmp = dir.join(format!(".{job_id}.json.tmp"));
        let dst = dir.join(format!("{job_id}.json"));
        std::fs::write(&tmp, serde_json::to_string_pretty(job).unwrap_or_default())?;
        std::fs::rename(&tmp, &dst)?;
        Ok(())
    }

    /// 把 updater 的 status.json 同步进审计行（终态行不回退，仓储层另有守卫）。
    async fn sync_job(&self, row: UpdateJobRow) -> UpdateJobRow {
        if row.status == "done" {
            return row;
        }
        let spool = self.cfg.spool_dir.trim();
        if spool.is_empty() {
            return row;
        }
        let path = std::path::Path::new(spool)
            .join("status")
            .join(format!("{}.json", row.job_id));
        let Ok(text) = std::fs::read_to_string(&path) else {
            return row;
        };
        let Ok(st) = serde_json::from_str::<UpdaterStatus>(&text) else {
            return row;
        };
        if st.status.is_empty() || (st.status == row.status && st.stage == row.stage) {
            return row;
        }
        let updated_at = if st.updated_at > 0 {
            st.updated_at
        } else {
            chrono::Utc::now().timestamp()
        };
        let _ = self
            .repos
            .system
            .system_update
            .update_job_progress(&row.job_id, &st.status, &st.stage, &st.message, updated_at)
            .await;
        UpdateJobRow {
            status: st.status,
            stage: st.stage,
            message: st.message,
            updated_at,
            ..row
        }
    }
}

// ——————————————————————————————————————————————————————————————
// 纯函数：本地版本戳读取（可单测）
// ——————————————————————————————————————————————————————————————

/// 读某静态面的当前版本戳：优先 `version.json`，降级站点指纹。
/// 返回 (stamp, 来源说明)；都读不到返回 (None, None)。
fn read_surface_stamp(surface: &str, root: &str) -> (Option<String>, Option<String>) {
    let root = std::path::Path::new(root);
    if let Ok(text) = std::fs::read_to_string(root.join("version.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(stamp) = v["stamp"].as_str().filter(|s| !s.is_empty()) {
                return (Some(stamp.to_string()), Some("version.json".into()));
            }
            if let Some(sha) = v["gitSha"].as_str().filter(|s| !s.is_empty()) {
                return (Some(sha.to_string()), Some("version.json（gitSha）".into()));
            }
        }
    }
    match surface {
        "admin" => match std::fs::read_to_string(root.join("index.html")) {
            Ok(html) => match extract_entry(&html, "index-", ".js") {
                Some(e) => (
                    Some(e),
                    Some("index.html 入口指纹（version.json 缺失时的降级口径）".into()),
                ),
                None => (None, None),
            },
            Err(_) => (None, None),
        },
        "h5" => match std::fs::read_to_string(root.join("index.html")) {
            Ok(html) => match extract_entry(&html, "app.", ".js") {
                Some(e) => (
                    Some(e),
                    Some("index.html 入口指纹（version.json 缺失时的降级口径）".into()),
                ),
                None => (None, None),
            },
            Err(_) => (None, None),
        },
        "pc" => {
            if let Ok(text) = std::fs::read_to_string(root.join("DEPLOY_INFO.txt")) {
                if let Some(id) = text
                    .lines()
                    .find_map(|l| l.strip_prefix("buildId "))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                {
                    return (Some(id), Some("DEPLOY_INFO.txt".into()));
                }
            }
            match std::fs::read_to_string(root.join(".next").join("BUILD_ID")) {
                Ok(id) if !id.trim().is_empty() => {
                    (Some(id.trim().to_string()), Some(".next/BUILD_ID".into()))
                }
                _ => (None, None),
            }
        }
        _ => (None, None),
    }
}

/// 从 HTML 中提取首个 `prefix...suffix` 形态的资源文件名
///（如 `index-AbC123.js` / `app.627b4a2c.js`），不依赖正则。
fn extract_entry(html: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = html.find(prefix)?;
    let tail = &html[start..];
    let end = tail.find(suffix)? + suffix.len();
    let candidate = &tail[..end];
    if candidate
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        Some(candidate.to_string())
    } else {
        None
    }
}

/// sha256 小写 hex。
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_entry_finds_hashed_names() {
        let html = r#"<script src="/assets/index-DyO5HJEx.js"></script>"#;
        assert_eq!(
            extract_entry(html, "index-", ".js").as_deref(),
            Some("index-DyO5HJEx.js")
        );
        let h5 = r#"<script src="js/app.627b4a2c.js"></script>"#;
        assert_eq!(
            extract_entry(h5, "app.", ".js").as_deref(),
            Some("app.627b4a2c.js")
        );
        assert_eq!(extract_entry("<html></html>", "index-", ".js"), None);
        // 含路径分隔的候选不是纯文件名 → 拒绝
        assert_eq!(extract_entry("index-a/b.js", "index-", ".js"), None);
    }

    #[test]
    fn sha256_hex_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn pc_stamp_reads_deploy_info() {
        let dir = std::env::temp_dir().join(format!("nt-su-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("DEPLOY_INFO.txt"),
            "commit abc\nbuild 42\nbuildId XH_O18PRVhb9FErYZoHZH\n",
        )
        .unwrap();
        let (stamp, detail) = read_surface_stamp("pc", dir.to_str().unwrap());
        assert_eq!(stamp.as_deref(), Some("XH_O18PRVhb9FErYZoHZH"));
        assert_eq!(detail.as_deref(), Some("DEPLOY_INFO.txt"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn version_json_takes_priority() {
        let dir = std::env::temp_dir().join(format!("nt-su2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("version.json"),
            r#"{"surface":"admin","stamp":"index-NEW.js","gitSha":"abc"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("index.html"), "<script src=\"index-OLD.js\">").unwrap();
        let (stamp, detail) = read_surface_stamp("admin", dir.to_str().unwrap());
        assert_eq!(stamp.as_deref(), Some("index-NEW.js"));
        assert_eq!(detail.as_deref(), Some("version.json"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
