//! 店铺小程序「模式②」后半程：代提审 / 代发布编排服务
//! （merchant::shop_mini_program_release）—— **本平台新增**（论坛帖
//! 487946315732211118 三期；设计稿 `~/workspace/shop-miniprogram-binding/
//! DESIGN.md` §二模式②的模板流水线部分）。
//!
//! 模式②授权基建（二期，[`super::shop_mini_program_auth`]）就绪后，平台
//! 凭 `authorizer_access_token` 代商家走完微信代码管理全链（官方《代商家
//! 管理小程序 · 小程序代码管理》，权限集 18）：
//!
//! ```text
//! 模板库草稿 → 加入模板库 → 代提交代码 commit（模板 + 按店 ext_json）
//!   → 提交审核 submit_audit → 同步审核状态 → 发布 release
//!   →（异常时）撤回审核 undo / 线上版本回退 revert
//! ```
//!
//! 状态落库口径：
//! - `shop_mini_program_version`：每个授权小程序一行的状态快照。`stage`
//!   是本地阶段（见常量），只由本模块的动作推导；`audit_status` 直接存
//!   微信口径（-1 未提审 / 0 成功 / 1 拒绝 / 2 审核中 / 3 已撤回 /
//!   4 审核延后），不另造映射表。
//! - `shop_mini_program_release_log`：动作流水只追加，成功失败都落行
//!   （失败带微信 errcode 文案），`operator_type` 区分平台代操作与商家
//!   自助发起——商家在管理端可对**本店**授权发起同一套动作（提审申请
//!   即商家自助提交，平台可代操作），作用域由路由层强制。
//!
//! 诚实口径：第三方平台未配置 / 授权已撤销时，令牌获取阶段即明确报错
//! （沿 [`super::shop_mini_program_auth::authorizer_access_token`] 与
//! [`crate::content::wechat_component`] 既有口径），微信 errcode 原样
//! 上抛（如 61039 代码检测未结束、87012 该版本不可回退），绝不假成功。
//!
//! 迁移：`rust/migrations/039_shop_mini_program_release.sql`
//! （部署前必须先执行，迁移先于二进制）。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use super::shop_mini_program_auth::{self, AuthorizationRow, AUTH_STATUS_AUTHORIZED};
use crate::content::wechat_component::{
    self, AuditCategory, AuditItemInput, AuditStatusInfo, CodeTemplate, HistoryVersion,
    TemplateDraft,
};

/// 全局业务错误码（与同域服务口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 本地阶段：未上传代码。
pub const STAGE_NONE: i32 = 0;
/// 本地阶段：已上传（体验版）待提审。
pub const STAGE_UPLOADED: i32 = 1;
/// 本地阶段：审核中（含审核延后）。
pub const STAGE_AUDITING: i32 = 2;
/// 本地阶段：审核通过待发布。
pub const STAGE_AUDIT_PASSED: i32 = 3;
/// 本地阶段：审核被拒。
pub const STAGE_AUDIT_REJECTED: i32 = 4;
/// 本地阶段：已发布（线上版本在运行）。
pub const STAGE_RELEASED: i32 = 5;

/// 审核状态：未提审（本地哨兵，微信无此值）。
pub const AUDIT_NONE: i32 = -1;
/// 审核状态：审核成功（微信口径）。
pub const AUDIT_PASSED: i32 = 0;
/// 审核状态：审核被拒绝（微信口径）。
pub const AUDIT_REJECTED: i32 = 1;
/// 审核状态：审核中（微信口径）。
pub const AUDIT_PENDING: i32 = 2;
/// 审核状态：已撤回（微信口径）。
pub const AUDIT_WITHDRAWN: i32 = 3;
/// 审核状态：审核延后（微信口径）。
pub const AUDIT_DELAYED: i32 = 4;

/// 流水动作：代提交代码。
pub const ACTION_COMMIT: &str = "commit";
/// 流水动作：提交审核。
pub const ACTION_SUBMIT_AUDIT: &str = "submit_audit";
/// 流水动作：同步审核状态。
pub const ACTION_AUDIT_SYNC: &str = "audit_sync";
/// 流水动作：发布。
pub const ACTION_RELEASE: &str = "release";
/// 流水动作：撤回审核。
pub const ACTION_WITHDRAW_AUDIT: &str = "withdraw_audit";
/// 流水动作：版本回退。
pub const ACTION_REVERT: &str = "revert";

fn now_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

fn biz(msg: impl Into<String>) -> Error {
    Error::biz(SERVICE_DATA_ERROR, msg.into())
}

/// 动作发起方（路由层按租户上下文构造）：平台代操作 / 商家自助。
#[derive(Debug, Clone)]
pub struct ReleaseOperator {
    /// `platform` / `merchant`。
    pub operator_type: String,
    /// 商家自助时为其店铺 id；平台代操作为 0。
    pub shop_id: i32,
}

impl ReleaseOperator {
    #[must_use]
    pub fn platform() -> Self {
        Self {
            operator_type: "platform".to_string(),
            shop_id: 0,
        }
    }

    #[must_use]
    pub fn merchant(shop_id: i32) -> Self {
        Self {
            operator_type: "merchant".to_string(),
            shop_id,
        }
    }
}

/// 版本状态视图（状态行 + 授权/店铺回显字段；无状态行时版本字段为默认）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionState {
    /// 状态行 id（0 = 尚无状态行，从未代提交过代码）。
    pub id: i32,
    pub authorizer_id: i32,
    pub shop_id: i32,
    #[sqlx(default)]
    pub shop_title: String,
    pub authorizer_app_id: String,
    #[sqlx(default)]
    pub nick_name: String,
    #[sqlx(default)]
    pub head_img: String,
    /// 授权状态（1 授权中 / 0 已撤销 / 2 已解绑），来自授权表。
    #[sqlx(default)]
    pub auth_status: i32,
    pub template_id: i64,
    pub user_version: String,
    pub user_desc: String,
    pub ext_json: String,
    pub audit_id: i64,
    /// -1 未提审 / 0 成功 / 1 拒绝 / 2 审核中 / 3 已撤回 / 4 审核延后。
    pub audit_status: i32,
    pub audit_reason: String,
    pub audit_screenshot: String,
    pub audit_user_version: String,
    pub submit_audit_time: i32,
    pub online_user_version: String,
    /// 本地阶段（见 STAGE_* 常量）。
    pub stage: i32,
    pub update_time: i32,
}

/// 发布动作流水行。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReleaseLogItem {
    pub id: i32,
    pub authorizer_id: i32,
    pub shop_id: i32,
    pub authorizer_app_id: String,
    pub action: String,
    pub detail: String,
    pub success: i32,
    pub err_msg: String,
    pub operator_type: String,
    pub operator_shop_id: i32,
    pub add_time: i32,
}

/// 按店生成代提交代码的 `ext_json`（官方 ext 配置口径）：`extEnable`
/// 开启模板 ext 模式、`extAppid` 固定为该授权小程序自身，自定义字段只
/// 放 `ext` 内（小程序端 `wx.getExtConfigSync()` 读取）——下发店铺归属
/// 作展示加速；**信任锚仍是后端按 appid 的解析**（设计稿 §3.3），ext
/// 内的 shopId 被篡改也不影响数据作用域。纯函数，便于测试。
#[must_use]
pub fn build_ext_json(shop_id: i32, authorizer_app_id: &str) -> String {
    serde_json::json!({
        "extEnable": true,
        "extAppid": authorizer_app_id,
        "directCommit": false,
        "ext": {
            "shopId": shop_id,
            "authorizerAppId": authorizer_app_id,
        },
    })
    .to_string()
}

/// 审核状态 → 本地阶段推导（同步审核状态时用）。已发布且审核单未变时
/// 保持已发布（发布后微信侧审核状态仍为「成功」，不能把阶段打回待发布）。
#[must_use]
pub fn stage_after_audit_sync(current_stage: i32, current_audit_id: i64, info: &AuditStatusInfo) -> i32 {
    match info.status {
        x if x == i64::from(AUDIT_PASSED) => {
            if current_stage == STAGE_RELEASED && current_audit_id == info.audit_id {
                STAGE_RELEASED
            } else {
                STAGE_AUDIT_PASSED
            }
        }
        x if x == i64::from(AUDIT_REJECTED) => STAGE_AUDIT_REJECTED,
        x if x == i64::from(AUDIT_PENDING) || x == i64::from(AUDIT_DELAYED) => STAGE_AUDITING,
        x if x == i64::from(AUDIT_WITHDRAWN) => STAGE_UPLOADED,
        _ => current_stage,
    }
}

/// 取生效中的授权行（不存在 / 已撤销 / 已解绑 → 明确业务错误）。
async fn require_active_authorizer(
    repos: &Repositories,
    authorizer_id: i32,
) -> Result<AuthorizationRow> {
    let row = shop_mini_program_auth::find_authorization_by_id(&repos.pool, authorizer_id)
        .await?
        .ok_or_else(|| biz("授权记录不存在"))?;
    if row.status != AUTH_STATUS_AUTHORIZED {
        return Err(biz("该小程序授权已撤销或已解绑，不能进行版本发布操作"));
    }
    Ok(row)
}

/// 写一条动作流水（失败只记日志不掩盖主错误）。
async fn log_action(
    repos: &Repositories,
    row: &AuthorizationRow,
    action: &str,
    detail: &Value,
    success: bool,
    err_msg: &str,
    operator: &ReleaseOperator,
) {
    let res = sqlx::query(
        "INSERT INTO shop_mini_program_release_log (authorizer_id, shop_id, \
                authorizer_app_id, action, detail, success, err_msg, \
                operator_type, operator_shop_id, add_time, is_del) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0)",
    )
    .bind(row.id)
    .bind(row.shop_id)
    .bind(&row.authorizer_app_id)
    .bind(action)
    .bind(detail.to_string())
    .bind(i32::from(success))
    .bind(err_msg)
    .bind(&operator.operator_type)
    .bind(operator.shop_id)
    .bind(now_i32())
    .execute(&repos.pool)
    .await;
    if let Err(e) = res {
        tracing::warn!("发布流水落库失败（{action}）: {e}");
    }
}

const STATE_SELECT: &str = "SELECT COALESCE(v.id, 0) AS id, a.id AS authorizer_id, \
    a.shop_id, COALESCE(s.shop_title, '') AS shop_title, a.authorizer_app_id, \
    COALESCE(a.nick_name, '') AS nick_name, COALESCE(a.head_img, '') AS head_img, \
    a.status AS auth_status, COALESCE(v.template_id, 0) AS template_id, \
    COALESCE(v.user_version, '') AS user_version, \
    COALESCE(v.user_desc, '') AS user_desc, COALESCE(v.ext_json, '') AS ext_json, \
    COALESCE(v.audit_id, 0) AS audit_id, COALESCE(v.audit_status, -1) AS audit_status, \
    COALESCE(v.audit_reason, '') AS audit_reason, \
    COALESCE(v.audit_screenshot, '') AS audit_screenshot, \
    COALESCE(v.audit_user_version, '') AS audit_user_version, \
    COALESCE(v.submit_audit_time, 0) AS submit_audit_time, \
    COALESCE(v.online_user_version, '') AS online_user_version, \
    COALESCE(v.stage, 0) AS stage, COALESCE(v.update_time, 0) AS update_time \
    FROM shop_mini_program_authorizer a \
    LEFT JOIN shop s ON s.shop_id = a.shop_id \
    LEFT JOIN shop_mini_program_version v \
      ON v.authorizer_app_id = a.authorizer_app_id AND v.is_del = 0 \
    WHERE a.is_del = 0";

/// 发布总览（平台全量 / 商家仅本店）：每个授权小程序一行版本状态。
///
/// # Errors
/// 数据库失败。
pub async fn list_overview(
    pool: &sqlx::PgPool,
    shop_scope: Option<i32>,
) -> Result<Vec<VersionState>> {
    let sql = format!(
        "{STATE_SELECT} AND ($1::int IS NULL OR a.shop_id = $1) ORDER BY a.id DESC"
    );
    let rows: Vec<VersionState> = sqlx::query_as(&sql)
        .bind(shop_scope)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
    Ok(rows)
}

/// 单个授权小程序的版本状态（无状态行时版本字段为默认、id=0）。
///
/// # Errors
/// 授权记录不存在 / 数据库失败。
pub async fn get_state(pool: &sqlx::PgPool, authorizer_id: i32) -> Result<VersionState> {
    let sql = format!("{STATE_SELECT} AND a.id = $1");
    let row: Option<VersionState> = sqlx::query_as(&sql)
        .bind(authorizer_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
    row.ok_or_else(|| biz("授权记录不存在"))
}

/// 动作流水（最新在前，上限 100）。
///
/// # Errors
/// 数据库失败。
pub async fn list_logs(
    pool: &sqlx::PgPool,
    authorizer_id: i32,
    limit: i64,
) -> Result<Vec<ReleaseLogItem>> {
    let rows: Vec<ReleaseLogItem> = sqlx::query_as(
        "SELECT id, authorizer_id, shop_id, authorizer_app_id, action, detail, \
                success, err_msg, operator_type, operator_shop_id, add_time \
         FROM shop_mini_program_release_log \
         WHERE authorizer_id = $1 AND is_del = 0 ORDER BY id DESC LIMIT $2",
    )
    .bind(authorizer_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(rows)
}

/// 状态行 upsert（按 authorizer_app_id 唯一）：不存在则插入。
async fn ensure_state_row(repos: &Repositories, row: &AuthorizationRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO shop_mini_program_version (authorizer_id, shop_id, \
                authorizer_app_id, add_time, update_time, is_del) \
         SELECT $1, $2, $3, $4, $4, 0 \
         WHERE NOT EXISTS (SELECT 1 FROM shop_mini_program_version \
                           WHERE authorizer_app_id = $3 AND is_del = 0)",
    )
    .bind(row.id)
    .bind(row.shop_id)
    .bind(&row.authorizer_app_id)
    .bind(now_i32())
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 代提交代码 / 提审 / 同步 / 发布 / 撤回 / 回退
// ---------------------------------------------------------------------------

/// 代提交代码：以模板 + 按店 ext_json 上传代码生成体验版。成功后本地
/// 阶段置「已上传待提审」并清上一轮审核字段（新代码 = 新审核周期）。
///
/// # Errors
/// 参数不合法 / 授权失效 / 第三方平台未配置 / 微信 errcode。
pub async fn commit_code(
    repos: &Repositories,
    authorizer_id: i32,
    template_id: i64,
    user_version: &str,
    user_desc: &str,
    operator: &ReleaseOperator,
) -> Result<VersionState> {
    let user_version = user_version.trim();
    let user_desc = user_desc.trim();
    // 注意：模板库 template_id 从 0 起编号，0 是合法模板，不能按 >0 校验。
    if template_id < 0 {
        return Err(biz("请选择代码模板"));
    }
    if user_version.is_empty() || user_version.chars().count() > 64 {
        return Err(biz("代码版本号必填且不超过 64 个字符（微信官方口径）"));
    }
    if user_desc.is_empty() {
        return Err(biz("请填写代码描述"));
    }
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let ext_json = build_ext_json(row.shop_id, &row.authorizer_app_id);
    let detail = serde_json::json!({
        "templateId": template_id,
        "userVersion": user_version,
        "userDesc": user_desc,
        "extJson": ext_json,
    });
    let token = match shop_mini_program_auth::authorizer_access_token(
        repos,
        &row.authorizer_app_id,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            log_action(repos, &row, ACTION_COMMIT, &detail, false, &e.to_string(), operator).await;
            return Err(e);
        }
    };
    if let Err(e) =
        wechat_component::code_commit(&token, template_id, &ext_json, user_version, user_desc)
            .await
    {
        log_action(repos, &row, ACTION_COMMIT, &detail, false, &e.to_string(), operator).await;
        return Err(e);
    }
    ensure_state_row(repos, &row).await?;
    sqlx::query(
        "UPDATE shop_mini_program_version SET authorizer_id = $1, shop_id = $2, \
                template_id = $3, user_version = $4, user_desc = $5, ext_json = $6, \
                audit_id = 0, audit_status = $7, audit_reason = '', \
                audit_screenshot = '', audit_user_version = '', \
                submit_audit_time = 0, stage = $8, update_time = $9 \
         WHERE authorizer_app_id = $10 AND is_del = 0",
    )
    .bind(row.id)
    .bind(row.shop_id)
    .bind(template_id)
    .bind(user_version)
    .bind(user_desc)
    .bind(&ext_json)
    .bind(AUDIT_NONE)
    .bind(STAGE_UPLOADED)
    .bind(now_i32())
    .bind(&row.authorizer_app_id)
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    log_action(repos, &row, ACTION_COMMIT, &detail, true, "", operator).await;
    get_state(&repos.pool, authorizer_id).await
}

/// 提交审核：`items` 1–5 项（官方口径），页面/类目须取自该小程序的
/// 已提交页面与已通过类目（前端从 pages/categories 接口取，服务端只做
/// 形状校验）。审核中重复提审在本地先行拒绝。
///
/// # Errors
/// 未上传代码 / 正在审核中 / 参数不合法 / 微信 errcode。
pub async fn submit_audit(
    repos: &Repositories,
    authorizer_id: i32,
    items: &[AuditItemInput],
    feedback_info: Option<&str>,
    version_desc: Option<&str>,
    operator: &ReleaseOperator,
) -> Result<VersionState> {
    if items.is_empty() || items.len() > 5 {
        return Err(biz("提审项需 1–5 项（微信官方口径）"));
    }
    for item in items {
        if item.address.trim().is_empty()
            || item.first_class.trim().is_empty()
            || item.second_class.trim().is_empty()
            || item.title.trim().is_empty()
        {
            return Err(biz("提审项的页面路径、一级/二级类目与页面标题均必填"));
        }
    }
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let state = get_state(&repos.pool, authorizer_id).await?;
    if state.id == 0 || state.stage == STAGE_NONE {
        return Err(biz("尚未代提交代码，请先上传代码生成体验版后再提审"));
    }
    if state.audit_status == AUDIT_PENDING || state.audit_status == AUDIT_DELAYED {
        return Err(biz("当前版本正在审核中，如需重提请先撤回审核"));
    }
    let detail = serde_json::json!({
        "itemList": items,
        "feedbackInfo": feedback_info.unwrap_or_default(),
        "versionDesc": version_desc.unwrap_or_default(),
        "userVersion": state.user_version,
    });
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    let audit_id = match wechat_component::code_submit_audit(
        &token,
        items,
        feedback_info,
        version_desc,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log_action(repos, &row, ACTION_SUBMIT_AUDIT, &detail, false, &e.to_string(), operator)
                .await;
            return Err(e);
        }
    };
    let now = now_i32();
    sqlx::query(
        "UPDATE shop_mini_program_version SET audit_id = $1, audit_status = $2, \
                audit_reason = '', audit_screenshot = '', \
                audit_user_version = $3, submit_audit_time = $4, stage = $5, \
                update_time = $4 WHERE authorizer_app_id = $6 AND is_del = 0",
    )
    .bind(audit_id)
    .bind(AUDIT_PENDING)
    .bind(&state.user_version)
    .bind(now)
    .bind(STAGE_AUDITING)
    .bind(&row.authorizer_app_id)
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    log_action(repos, &row, ACTION_SUBMIT_AUDIT, &detail, true, "", operator).await;
    get_state(&repos.pool, authorizer_id).await
}

/// 同步审核状态：拉微信最新一次提审状态回写本地（阶段随之推导）。
/// 本地从未提审且微信侧也无审核单时，微信回 86001 等错误，如实上抛。
///
/// # Errors
/// 授权失效 / 微信 errcode / 数据库失败。
pub async fn sync_audit_status(
    repos: &Repositories,
    authorizer_id: i32,
    operator: &ReleaseOperator,
) -> Result<VersionState> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    let info = match wechat_component::code_get_latest_audit_status(&token).await {
        Ok(info) => info,
        Err(e) => {
            log_action(
                repos,
                &row,
                ACTION_AUDIT_SYNC,
                &serde_json::json!({}),
                false,
                &e.to_string(),
                operator,
            )
            .await;
            return Err(e);
        }
    };
    ensure_state_row(repos, &row).await?;
    let state = get_state(&repos.pool, authorizer_id).await?;
    let stage = stage_after_audit_sync(state.stage, state.audit_id, &info);
    let submit_time = if info.submit_audit_time > 0 {
        i32::try_from(info.submit_audit_time).unwrap_or(state.submit_audit_time)
    } else {
        state.submit_audit_time
    };
    sqlx::query(
        "UPDATE shop_mini_program_version SET audit_id = $1, audit_status = $2, \
                audit_reason = $3, audit_screenshot = $4, audit_user_version = $5, \
                submit_audit_time = $6, stage = $7, update_time = $8 \
         WHERE authorizer_app_id = $9 AND is_del = 0",
    )
    .bind(info.audit_id)
    .bind(i32::try_from(info.status).unwrap_or(AUDIT_NONE))
    .bind(&info.reason)
    .bind(&info.screenshot)
    .bind(if info.user_version.is_empty() {
        state.audit_user_version.clone()
    } else {
        info.user_version.clone()
    })
    .bind(submit_time)
    .bind(stage)
    .bind(now_i32())
    .bind(&row.authorizer_app_id)
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    log_action(
        repos,
        &row,
        ACTION_AUDIT_SYNC,
        &serde_json::to_value(&info).unwrap_or(Value::Null),
        true,
        "",
        operator,
    )
    .await;
    get_state(&repos.pool, authorizer_id).await
}

/// 发布：本地审核状态必须为「审核成功」（先同步再发布，避免凭旧状态
/// 误发）；发布后线上版本 = 本次审核通过的版本。
///
/// # Errors
/// 未通过审核 / 微信 errcode / 数据库失败。
pub async fn release(
    repos: &Repositories,
    authorizer_id: i32,
    operator: &ReleaseOperator,
) -> Result<VersionState> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let state = get_state(&repos.pool, authorizer_id).await?;
    if state.audit_status != AUDIT_PASSED {
        return Err(biz("当前版本尚未通过审核，不能发布（请先同步审核状态确认已通过）"));
    }
    let detail = serde_json::json!({
        "auditId": state.audit_id,
        "userVersion": state.audit_user_version,
    });
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    if let Err(e) = wechat_component::code_release(&token).await {
        log_action(repos, &row, ACTION_RELEASE, &detail, false, &e.to_string(), operator).await;
        return Err(e);
    }
    let online = if state.audit_user_version.is_empty() {
        state.user_version.clone()
    } else {
        state.audit_user_version.clone()
    };
    sqlx::query(
        "UPDATE shop_mini_program_version SET stage = $1, online_user_version = $2, \
                update_time = $3 WHERE authorizer_app_id = $4 AND is_del = 0",
    )
    .bind(STAGE_RELEASED)
    .bind(&online)
    .bind(now_i32())
    .bind(&row.authorizer_app_id)
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    log_action(repos, &row, ACTION_RELEASE, &detail, true, "", operator).await;
    get_state(&repos.pool, authorizer_id).await
}

/// 撤回审核：仅审核中/审核延后可撤；撤回后阶段回到「已上传待提审」，
/// 可修改后重新提审（微信对撤回次数有限制，超限错误如实上抛）。
///
/// # Errors
/// 不在审核中 / 微信 errcode / 数据库失败。
pub async fn withdraw_audit(
    repos: &Repositories,
    authorizer_id: i32,
    operator: &ReleaseOperator,
) -> Result<VersionState> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let state = get_state(&repos.pool, authorizer_id).await?;
    if state.audit_status != AUDIT_PENDING && state.audit_status != AUDIT_DELAYED {
        return Err(biz("当前没有进行中的审核单，无需撤回"));
    }
    let detail = serde_json::json!({ "auditId": state.audit_id });
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    if let Err(e) = wechat_component::code_undo_audit(&token).await {
        log_action(repos, &row, ACTION_WITHDRAW_AUDIT, &detail, false, &e.to_string(), operator)
            .await;
        return Err(e);
    }
    sqlx::query(
        "UPDATE shop_mini_program_version SET audit_status = $1, stage = $2, \
                update_time = $3 WHERE authorizer_app_id = $4 AND is_del = 0",
    )
    .bind(AUDIT_WITHDRAWN)
    .bind(STAGE_UPLOADED)
    .bind(now_i32())
    .bind(&row.authorizer_app_id)
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    log_action(repos, &row, ACTION_WITHDRAW_AUDIT, &detail, true, "", operator).await;
    get_state(&repos.pool, authorizer_id).await
}

/// 线上版本回退：`app_version` 为 None = 回退到上一个版本。回退前先取
/// 历史版本列表把目标 app_version 映射回 user_version 回写线上版本
/// （映射不到时保留原值并在流水 detail 注明）。回退过的版本不能再次
/// 回退，微信 87012 如实上抛。
///
/// # Errors
/// 微信 errcode / 数据库失败。
pub async fn revert(
    repos: &Repositories,
    authorizer_id: i32,
    app_version: Option<i64>,
    operator: &ReleaseOperator,
) -> Result<VersionState> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    // 映射尽力而为：取不到历史列表不阻断回退本身（微信是事实源）。
    let history = wechat_component::code_history_versions(&token)
        .await
        .unwrap_or_default();
    let target = match app_version {
        Some(v) => history.iter().find(|h| h.app_version == v),
        None => history.first(),
    };
    let target_user_version = target.map(|h| h.user_version.clone());
    let mut detail = serde_json::json!({
        "appVersion": app_version,
        "targetUserVersion": target_user_version,
    });
    if target_user_version.is_none() {
        detail["onlineVersionUnknown"] = Value::Bool(true);
    }
    if let Err(e) = wechat_component::code_revert(&token, app_version).await {
        log_action(repos, &row, ACTION_REVERT, &detail, false, &e.to_string(), operator).await;
        return Err(e);
    }
    if let Some(user_version) = &target_user_version {
        sqlx::query(
            "UPDATE shop_mini_program_version SET online_user_version = $1, \
                    update_time = $2 WHERE authorizer_app_id = $3 AND is_del = 0",
        )
        .bind(user_version)
        .bind(now_i32())
        .bind(&row.authorizer_app_id)
        .execute(&repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
    }
    log_action(repos, &row, ACTION_REVERT, &detail, true, "", operator).await;
    get_state(&repos.pool, authorizer_id).await
}

// ---------------------------------------------------------------------------
// 提审表单取数（页面 / 类目 / 历史版本）与模板库（平台）
// ---------------------------------------------------------------------------

/// 已提交代码的页面列表（提审项页面取数）。
///
/// # Errors
/// 授权失效 / 微信 errcode（86001 = 尚未提交代码）。
pub async fn page_list(repos: &Repositories, authorizer_id: i32) -> Result<Vec<String>> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    wechat_component::code_get_page(&token).await
}

/// 已申请通过的类目列表（提审项类目取数）。
///
/// # Errors
/// 授权失效 / 微信 errcode。
pub async fn category_list(
    repos: &Repositories,
    authorizer_id: i32,
) -> Result<Vec<AuditCategory>> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    wechat_component::code_get_category(&token).await
}

/// 可回退的历史版本列表。
///
/// # Errors
/// 授权失效 / 微信 errcode。
pub async fn history_versions(
    repos: &Repositories,
    authorizer_id: i32,
) -> Result<Vec<HistoryVersion>> {
    let row = require_active_authorizer(repos, authorizer_id).await?;
    let token = shop_mini_program_auth::authorizer_access_token(repos, &row.authorizer_app_id)
        .await?;
    wechat_component::code_history_versions(&token).await
}

/// 模板库草稿箱（平台侧；component 令牌，未配置如实报错）。
///
/// # Errors
/// 第三方平台未配置 / 微信 errcode。
pub async fn list_template_drafts(repos: &Repositories) -> Result<Vec<TemplateDraft>> {
    wechat_component::template_draft_list(repos).await
}

/// 模板库列表（平台侧）。
///
/// # Errors
/// 第三方平台未配置 / 微信 errcode。
pub async fn list_code_templates(repos: &Repositories) -> Result<Vec<CodeTemplate>> {
    wechat_component::template_list(repos, None).await
}

/// 草稿加入模板库（平台侧）：`template_type` 0 普通模板 / 1 标准模板。
///
/// # Errors
/// 参数不合法 / 第三方平台未配置 / 微信 errcode。
pub async fn add_draft_to_template(
    repos: &Repositories,
    draft_id: i64,
    template_type: i64,
) -> Result<()> {
    if draft_id <= 0 {
        return Err(biz("请选择草稿"));
    }
    if template_type != 0 && template_type != 1 {
        return Err(biz("模板类型只能是 0（普通模板）或 1（标准模板）"));
    }
    wechat_component::template_add_to_library(repos, draft_id, template_type).await
}

/// 删除模板（平台侧）。
///
/// # Errors
/// 参数不合法 / 第三方平台未配置 / 微信 errcode。
pub async fn delete_code_template(repos: &Repositories, template_id: i64) -> Result<()> {
    if template_id < 0 {
        return Err(biz("模板 id 不合法"));
    }
    wechat_component::template_delete(repos, template_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_json_carries_shop_scope_under_ext() {
        let raw = build_ext_json(42, "wxauthorizer");
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["extEnable"], true);
        assert_eq!(value["extAppid"], "wxauthorizer");
        assert_eq!(value["ext"]["shopId"], 42);
        assert_eq!(value["ext"]["authorizerAppId"], "wxauthorizer");
    }

    #[test]
    fn stage_derivation_follows_audit_status() {
        let passed = AuditStatusInfo {
            audit_id: 7,
            status: 0,
            ..Default::default()
        };
        assert_eq!(stage_after_audit_sync(STAGE_AUDITING, 7, &passed), STAGE_AUDIT_PASSED);
        // 已发布且审核单未变：同步不能把阶段打回待发布
        assert_eq!(stage_after_audit_sync(STAGE_RELEASED, 7, &passed), STAGE_RELEASED);
        // 已发布但同步到新审核单通过：回到待发布
        assert_eq!(stage_after_audit_sync(STAGE_RELEASED, 6, &passed), STAGE_AUDIT_PASSED);
        let rejected = AuditStatusInfo {
            status: 1,
            ..Default::default()
        };
        assert_eq!(stage_after_audit_sync(STAGE_AUDITING, 0, &rejected), STAGE_AUDIT_REJECTED);
        let delayed = AuditStatusInfo {
            status: 4,
            ..Default::default()
        };
        assert_eq!(stage_after_audit_sync(STAGE_AUDITING, 0, &delayed), STAGE_AUDITING);
        let withdrawn = AuditStatusInfo {
            status: 3,
            ..Default::default()
        };
        assert_eq!(stage_after_audit_sync(STAGE_AUDITING, 0, &withdrawn), STAGE_UPLOADED);
    }

    #[test]
    fn stage_constants_are_stable() {
        assert_eq!(STAGE_NONE, 0);
        assert_eq!(STAGE_RELEASED, 5);
        assert_eq!(AUDIT_NONE, -1);
        assert_eq!(AUDIT_PASSED, 0);
    }
}
