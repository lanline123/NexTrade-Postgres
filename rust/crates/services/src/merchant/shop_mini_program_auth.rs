//! 店铺小程序「模式②」第三方平台授权关系服务（merchant::shop_mini_program_auth）
//! —— **本平台新增**（论坛帖 487946315732211118 二期；设计稿
//! `~/workspace/shop-miniprogram-binding/DESIGN.md` §二模式②）。
//!
//! 与模式①（[`super::shop_mini_program`]，商家自填 appid + AppSecret 直连）
//! **共用同一套 appid 隔离口径**：授权方 appid 经
//! [`super::shop_mini_program::resolve_app_id`] 的授权表回落分支解析到店铺，
//! 前台作用域中间件、登录分键、会员手机号归一全部不感知模式差异。两种
//! 模式的差异只在凭据形态：
//!
//! - 模式①：AppSecret（商家自有，加密落 `shop_mini_program`）；
//! - 模式②：`authorizer_refresh_token`（微信授权下发，加密落本表）。它是
//!   代调用的根凭据，**丢失 = 商家必须重新授权**，按 AI 渠道密钥库口径
//!   （[`crate::ai::vault`] AES-256-GCM）保管，后台只显掩码。
//!
//! 授权链（微信官方《授权流程技术说明》）：
//! 后台发起 → 预授权码 + 待授权会话（缓存，含目标店铺）→ 商家扫码授权 →
//! 授权页跳回公开回调（`auth_code` + `state`）**或**微信向授权事件接收
//! URL 推 `authorized` 事件（带同一 `auth_code` 与 `PreAuthCode`）→ 两条
//! 路径都收敛到 [`complete_authorization`] 用授权码换 refresh token 落库
//! （幂等：重复完成只更新同一行）。商家在 mp 后台取消授权 → 微信推
//! `unauthorized` 事件 → 本表置已撤销 → 该 appid 立即按未登记拒绝
//! （与模式①停用/解绑同口径，绝不回落平台全量）。
//!
//! 本地「解绑」只解除本系统绑定（status=已解绑）：微信侧授权关系的最终
//! 撤销动作在商家 mp 后台，本系统无法代为取消（官方无此接口），后台
//! 文案已明示。代提审/代发布（模板流水线）不在本期范围：授权与 token
//! 基建就绪后另行立项。
//!
//! 迁移：`rust/migrations/028_shop_mini_program_thirdparty.sql`
//! （部署前必须先执行，迁移先于二进制）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::get_cache;
use nextrade_infrastructure::db::Repositories;

use crate::ai::vault;
use crate::content::wechat_component::{self, ComponentConfig};

/// 全局业务错误码（与同域服务口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 授权状态：授权中。
pub const AUTH_STATUS_AUTHORIZED: i32 = 1;
/// 授权状态：已撤销（微信 unauthorized 事件）。
pub const AUTH_STATUS_REVOKED: i32 = 0;
/// 授权状态：已解绑（后台本地解除）。
pub const AUTH_STATUS_DETACHED: i32 = 2;

/// 待授权会话缓存键前缀（按 state 查）。
const PENDING_KEY_PREFIX: &str = "wechat:component:pending:";
/// 待授权会话缓存键前缀（按 pre_auth_code 查，供授权事件路径收敛）。
const PENDING_BY_PRECODE_PREFIX: &str = "wechat:component:pending_by_precode:";
/// 待授权会话有效期（秒；pre_auth_code 本身 600 秒过期，会话留冗余供轮询）。
const PENDING_TTL_SECS: u64 = 1_800;
/// 授权方 access_token 缓存键前缀。
const AUTHORIZER_TOKEN_CACHE_PREFIX: &str = "wechat:component:authorizer_token:";
/// token 提前过期边距（秒，与 component 令牌口径一致）。
const TOKEN_REFRESH_MARGIN_SECS: i64 = 300;

fn now_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

fn now_secs() -> i64 {
    nextrade_common::utils::now_seconds()
}

fn biz(msg: impl Into<String>) -> Error {
    Error::biz(SERVICE_DATA_ERROR, msg.into())
}

/// 授权行内部视图（含密文，仅服务内使用）。
#[derive(Debug, Clone, FromRow)]
pub struct AuthorizationRow {
    pub id: i32,
    pub shop_id: i32,
    pub authorizer_app_id: String,
    pub refresh_token_cipher: String,
    pub refresh_token_nonce: String,
    pub status: i32,
}

/// 管理端授权行（refresh token 只出掩码，密文永不出接口）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuthorizationItem {
    pub id: i32,
    pub shop_id: i32,
    #[sqlx(default)]
    pub shop_title: String,
    pub authorizer_app_id: String,
    pub refresh_token_mask: String,
    pub nick_name: String,
    pub head_img: String,
    pub user_name: String,
    pub principal_name: String,
    /// 权限集 id 的 JSON 数组文本（如 `[1,17]`）。
    pub func_info: String,
    /// 1 授权中 / 0 已撤销 / 2 已解绑。
    pub status: i32,
    pub authorized_at: i32,
    pub last_refresh_at: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 按授权方 appid 查未删授权行（含已撤销/已解绑；调用方按 status 决策）。
pub async fn find_authorization_by_app_id(
    pool: &sqlx::PgPool,
    app_id: &str,
) -> Result<Option<AuthorizationRow>> {
    let row = sqlx::query_as::<_, AuthorizationRow>(
        "SELECT id, shop_id, authorizer_app_id, refresh_token_cipher, refresh_token_nonce, status \
         FROM shop_mini_program_authorizer WHERE authorizer_app_id = $1 AND is_del = 0 LIMIT 1",
    )
    .bind(app_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(row)
}

/// 按 id 查未删授权行（路由层作用域校验用）。
pub async fn find_authorization_by_id(
    pool: &sqlx::PgPool,
    id: i32,
) -> Result<Option<AuthorizationRow>> {
    let row = sqlx::query_as::<_, AuthorizationRow>(
        "SELECT id, shop_id, authorizer_app_id, refresh_token_cipher, refresh_token_nonce, status \
         FROM shop_mini_program_authorizer WHERE id = $1 AND is_del = 0 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(row)
}

/// 解密授权行 refresh token（主密钥缺失/密文损坏 → 明确报错，不回落）。
pub fn decrypt_refresh_token(row: &AuthorizationRow) -> Result<String> {
    vault::decrypt(&row.refresh_token_cipher, &row.refresh_token_nonce)
        .map_err(|e| Error::Internal(format!("授权 refresh token 解密失败: {e}")))
}

// ---------------------------------------------------------------------------
// 待授权会话（发起授权 → 回调/事件收敛）
// ---------------------------------------------------------------------------

/// 待授权会话（缓存存储；授权完成时回填结果供后台轮询）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingAuth {
    pub shop_id: i32,
    pub pre_auth_code: String,
    /// `pending` / `authorized`。
    pub status: String,
    pub authorizer_app_id: String,
}

/// 发起授权的产物（授权链接 + 会话 state）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLink {
    pub pc_url: String,
    pub h5_url: String,
    pub state: String,
    /// pre_auth_code 有效期（秒，官方口径 600）。
    pub expires_in: u64,
}

/// 发起一次店铺授权：校验前置 → 取预授权码 → 建会话 → 拼授权链接。
///
/// 前置口径：第三方平台须已配置；回调域名须可知（配置位
/// `wechatComponentRedirectBase` 或 `h5Domain`）；店铺须存在；本店已有
/// 生效授权时拒绝重复发起（先解绑旧授权）。模式①直连绑定与本流程的
/// appid 互斥在授权完成时校验（发起时还不知道商家会授权哪个小程序）。
///
/// # Errors
/// 未配置 / 店铺不存在 / 已有生效授权 / 微信 errcode。
pub async fn create_auth_link(repos: &Repositories, shop_id: i32) -> Result<AuthLink> {
    if shop_id <= 0 {
        return Err(biz("店铺不存在"));
    }
    let cfg = wechat_component::load_component_config(repos).await?;
    if !cfg.is_configured() {
        return Err(biz(format!(
            "微信第三方平台未配置：缺少 {}（后台「店铺小程序 → 第三方平台设置」）",
            cfg.missing().join("/")
        )));
    }
    if cfg.redirect_base.is_empty() {
        return Err(biz(
            "未配置授权回调域名：请在第三方平台设置中填写回调域名（或先配置 h5Domain）",
        ));
    }
    let pool = &repos.pool;
    let shop_exists: Option<(i32,)> =
        sqlx::query_as("SELECT shop_id FROM shop WHERE shop_id = $1 LIMIT 1")
            .bind(shop_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
    if shop_exists.is_none() {
        return Err(biz("店铺不存在"));
    }
    let active: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM shop_mini_program_authorizer \
         WHERE shop_id = $1 AND status = $2 AND is_del = 0 LIMIT 1",
    )
    .bind(shop_id)
    .bind(AUTH_STATUS_AUTHORIZED)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    if active.is_some() {
        return Err(biz("本店已有生效的第三方授权，请先解绑后再重新授权"));
    }

    let pre_auth_code = wechat_component::create_pre_auth_code(repos).await?;
    let state = uuid::Uuid::new_v4().simple().to_string();
    let pending = PendingAuth {
        shop_id,
        pre_auth_code: pre_auth_code.clone(),
        status: "pending".to_string(),
        authorizer_app_id: String::new(),
    };
    let cache = get_cache();
    cache
        .set_json(
            &format!("{PENDING_KEY_PREFIX}{state}"),
            &pending,
            PENDING_TTL_SECS,
        )
        .await
        .map_err(|e| Error::Internal(format!("待授权会话缓存失败: {e}")))?;
    let _ = cache
        .set_str(
            &format!("{PENDING_BY_PRECODE_PREFIX}{pre_auth_code}"),
            &state,
            PENDING_TTL_SECS,
        )
        .await;
    let redirect_uri = format!(
        "{}{}?state={}",
        cfg.redirect_base,
        wechat_component::AUTH_CALLBACK_PATH,
        state
    );
    let (pc_url, h5_url) =
        wechat_component::authorization_urls(&cfg.app_id, &pre_auth_code, &redirect_uri);
    Ok(AuthLink {
        pc_url,
        h5_url,
        state,
        expires_in: 600,
    })
}

/// 查待授权会话（后台轮询授权结果）。
pub async fn pending_auth_status(state: &str) -> Option<PendingAuth> {
    get_cache()
        .get_json::<PendingAuth>(&format!("{PENDING_KEY_PREFIX}{state}"))
        .await
        .ok()
        .flatten()
}

async fn pending_by_precode(pre_auth_code: &str) -> Option<(String, PendingAuth)> {
    let cache = get_cache();
    let state = cache
        .get_str(&format!("{PENDING_BY_PRECODE_PREFIX}{pre_auth_code}"))
        .await
        .ok()
        .flatten()?;
    let pending = pending_auth_status(&state).await?;
    Some((state, pending))
}

async fn mark_pending_authorized(state: &str, authorizer_app_id: &str) {
    let cache = get_cache();
    if let Some(mut pending) = pending_auth_status(state).await {
        pending.status = "authorized".to_string();
        pending.authorizer_app_id = authorizer_app_id.to_string();
        let _ = cache
            .set_json(
                &format!("{PENDING_KEY_PREFIX}{state}"),
                &pending,
                PENDING_TTL_SECS,
            )
            .await;
    }
}

// ---------------------------------------------------------------------------
// 授权完成 / 事件应用
// ---------------------------------------------------------------------------

/// 授权完成（回调与 `authorized` 事件共用，幂等）：用授权码换授权信息，
/// 加密落 refresh token，并尽力同步授权方基本信息。
///
/// 目标店铺来源：`state` 对应的待授权会话；无会话时（如授权页回调丢失、
/// 只有事件到达且会话已过期）仅允许更新**已有**授权行，不新建归属。
///
/// 互斥口径：该 appid 已有模式①直连绑定（未解绑）→ 拒绝（先解绑直连）；
/// 该 appid 生效授权在其他店铺 → 拒绝；目标店铺已有其他 appid 的生效
/// 授权 → 拒绝。平台 appid 不得被授权占用（解析歧义、防隔离击穿）。
///
/// # Errors
/// 会话缺失且无存量行 / 互斥冲突 / 微信 errcode / 数据库失败。
pub async fn complete_authorization(
    repos: &Repositories,
    authorization_code: &str,
    state: Option<&str>,
) -> Result<i32> {
    let outcome = wechat_component::query_auth(repos, authorization_code).await?;
    let pool = &repos.pool;
    let authorizer_app_id = outcome.authorizer_app_id.clone();

    // 目标店铺：优先待授权会话，其次存量授权行。
    let session = match state {
        Some(s) if !s.is_empty() => pending_auth_status(s).await,
        _ => None,
    };
    let existing = find_authorization_by_app_id(pool, &authorizer_app_id).await?;
    let shop_id = if let Some(pending) = &session {
        pending.shop_id
    } else if let Some(row) = &existing {
        row.shop_id
    } else {
        return Err(biz(
            "授权会话已过期且该小程序无存量授权记录，无法确认归属店铺，请在后台重新发起授权",
        ));
    };

    // 平台 appid 不得被店铺授权占用
    let platform_app_id = repos
        .member
        .login
        .config_value("wechatMiniProgramAppId")
        .await?
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_default();
    if !platform_app_id.is_empty() && authorizer_app_id == platform_app_id {
        return Err(biz("该小程序是平台小程序，不能授权绑定到店铺"));
    }
    // 与模式①互斥：同 appid 存在未解绑的直连绑定 → 先解绑直连
    if super::shop_mini_program::find_binding_by_app_id(pool, &authorizer_app_id)
        .await?
        .is_some()
    {
        return Err(biz(
            "该小程序已通过模式①（直连）绑定，请先在绑定列表解绑后再改用第三方授权",
        ));
    }
    // 存量授权行归属校验
    if let Some(row) = &existing {
        if row.status == AUTH_STATUS_AUTHORIZED && row.shop_id != shop_id {
            return Err(biz("该小程序已授权给其他店铺，请先在原店铺解绑"));
        }
    }
    // 目标店铺不得已有其他 appid 的生效授权
    let other_active: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM shop_mini_program_authorizer \
         WHERE shop_id = $1 AND status = $2 AND is_del = 0 AND authorizer_app_id <> $3 LIMIT 1",
    )
    .bind(shop_id)
    .bind(AUTH_STATUS_AUTHORIZED)
    .bind(&authorizer_app_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    if other_active.is_some() {
        return Err(biz("本店已有其他小程序的生效授权，请先解绑后再授权"));
    }

    let (cipher, nonce) = vault::encrypt(&outcome.refresh_token)
        .map_err(|e| Error::Internal(format!("refresh token 加密失败: {e}")))?;
    let func_json = serde_json::to_string(&outcome.func_ids).unwrap_or_else(|_| "[]".to_string());
    let now = now_i32();
    let id = if let Some(row) = &existing {
        sqlx::query(
            "UPDATE shop_mini_program_authorizer SET shop_id = $1, refresh_token_cipher = $2, \
                    refresh_token_nonce = $3, refresh_token_mask = $4, func_info = $5, \
                    status = $6, authorized_at = $7, last_refresh_at = $8, update_time = $9 \
             WHERE id = $10",
        )
        .bind(shop_id)
        .bind(&cipher)
        .bind(&nonce)
        .bind(vault::mask_of(&outcome.refresh_token))
        .bind(&func_json)
        .bind(AUTH_STATUS_AUTHORIZED)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(row.id)
        .execute(pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        row.id
    } else {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shop_mini_program_authorizer (shop_id, authorizer_app_id, \
                    refresh_token_cipher, refresh_token_nonce, refresh_token_mask, func_info, \
                    status, authorized_at, last_refresh_at, add_time, update_time, is_del) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $8, $8, 0) RETURNING id",
        )
        .bind(shop_id)
        .bind(&authorizer_app_id)
        .bind(&cipher)
        .bind(&nonce)
        .bind(vault::mask_of(&outcome.refresh_token))
        .bind(&func_json)
        .bind(AUTH_STATUS_AUTHORIZED)
        .bind(now)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        row.0
    };

    // query_auth 直接带回的 access_token 先入缓存，省一次刷新。
    if !outcome.authorizer_access_token.is_empty() {
        put_authorizer_token_cache(
            &authorizer_app_id,
            &outcome.authorizer_access_token,
            outcome.expires_in,
        )
        .await;
    }
    // 基本信息同步尽力而为（失败不回滚授权，後台可手动「同步信息」）。
    if let Err(e) = sync_authorizer_info(repos, id).await {
        tracing::warn!("授权方基本信息同步失败（授权已落库）: {e}");
    }
    if let Some(s) = state {
        mark_pending_authorized(s, &authorizer_app_id).await;
    }
    Ok(id)
}

/// 应用一条授权事件明文 XML（公开回调端点验签解密后调用）。
/// 未知 InfoType 忽略；`unauthorized` 置已撤销后该 appid 立即被解析层
/// 拒绝（商家侧取消授权的降级处置即在此：回落未登记拒绝态，商家可改走
/// 模式①直连或重新授权）。
///
/// # Errors
/// 数据库 / 微信接口失败（ticket 存储失败不报错——下轮推送会再来）。
pub async fn apply_component_event(repos: &Repositories, plain_xml: &str) -> Result<()> {
    use crate::member::wechat_mp::xml_field;
    let info_type = xml_field(plain_xml, "InfoType").unwrap_or_default();
    match info_type.as_str() {
        "component_verify_ticket" => {
            if let Some(ticket) =
                xml_field(plain_xml, "ComponentVerifyTicket").filter(|s| !s.trim().is_empty())
            {
                if let Err(e) = wechat_component::store_verify_ticket(&ticket).await {
                    tracing::warn!("component_verify_ticket 缓存失败: {e}");
                }
            }
        }
        "authorized" | "updateauthorized" => {
            let auth_code = xml_field(plain_xml, "AuthorizationCode").unwrap_or_default();
            let pre_auth_code = xml_field(plain_xml, "PreAuthCode").unwrap_or_default();
            if auth_code.is_empty() {
                tracing::warn!("授权事件缺少 AuthorizationCode，忽略");
                return Ok(());
            }
            // 事件带 PreAuthCode：经待授权会话收敛到目标店铺（与回调同路径）。
            let state = pending_by_precode(&pre_auth_code)
                .await
                .map(|(s, _)| s);
            match complete_authorization(repos, &auth_code, state.as_deref()).await {
                Ok(_) => {}
                Err(e) => {
                    // updateauthorized 且无会话时 complete 已按存量行处理；
                    // 走到这里的失败多为会话过期——记录待人工在后台核对，
                    // 不向微信报错（事件通道只认 success，重推无意义）。
                    tracing::warn!("授权事件应用失败（{info_type}）: {e}");
                }
            }
        }
        "unauthorized" => {
            let app_id = xml_field(plain_xml, "AuthorizerAppid").unwrap_or_default();
            if !app_id.is_empty() {
                sqlx::query(
                    "UPDATE shop_mini_program_authorizer SET status = $1, update_time = $2 \
                     WHERE authorizer_app_id = $3 AND is_del = 0 AND status = $4",
                )
                .bind(AUTH_STATUS_REVOKED)
                .bind(now_i32())
                .bind(&app_id)
                .bind(AUTH_STATUS_AUTHORIZED)
                .execute(&repos.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
                invalidate_authorizer_token(&app_id).await;
            }
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 授权方 token（代调用根能力）
// ---------------------------------------------------------------------------

/// 缓存中的 token 记录（与 wechat_component 同形态）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedToken {
    token: String,
    expires_at: i64,
}

async fn put_authorizer_token_cache(app_id: &str, token: &str, expires_in: u64) {
    let record = CachedToken {
        token: token.to_string(),
        expires_at: now_secs() + i64::try_from(expires_in).unwrap_or(7_200),
    };
    let _ = get_cache()
        .set_json(
            &format!("{AUTHORIZER_TOKEN_CACHE_PREFIX}{app_id}"),
            &record,
            expires_in.max(60),
        )
        .await;
}

/// 清除授权方 token 缓存（授权撤销/解绑/微信回 40001 类错误后调用）。
pub async fn invalidate_authorizer_token(app_id: &str) {
    let _ = get_cache()
        .del(&format!("{AUTHORIZER_TOKEN_CACHE_PREFIX}{app_id}"))
        .await;
}

/// 取授权方 `authorizer_access_token`（代调用根能力：取手机号等 wxa 接口
/// 以此为 access_token）：缓存优先，过期凭落库 refresh token 刷新；微信
/// 带回新 refresh token 时同步轮换落库。
///
/// # Errors
/// 无授权行 / 授权已撤销或解绑 / 解密失败 / 微信 errcode（refresh token
/// 失效时微信回 61023 等，文案提示重新授权）。
pub async fn authorizer_access_token(
    repos: &Repositories,
    authorizer_app_id: &str,
) -> Result<String> {
    let cache = get_cache();
    let key = format!("{AUTHORIZER_TOKEN_CACHE_PREFIX}{authorizer_app_id}");
    if let Some(cached) = cache.get_json::<CachedToken>(&key).await.ok().flatten() {
        if cached.expires_at - TOKEN_REFRESH_MARGIN_SECS > now_secs() {
            return Ok(cached.token);
        }
    }
    let row = find_authorization_by_app_id(&repos.pool, authorizer_app_id)
        .await?
        .filter(|r| r.status == AUTH_STATUS_AUTHORIZED)
        .ok_or_else(|| biz("该小程序未授权或授权已撤销/解绑"))?;
    let refresh_token = decrypt_refresh_token(&row)?;
    let (token, expires_in, new_refresh) =
        wechat_component::refresh_authorizer_token(repos, authorizer_app_id, &refresh_token)
            .await
            .map_err(|e| {
                biz(format!(
                    "授权方 token 刷新失败（refresh token 可能已失效，请商家重新授权）：{e}"
                ))
            })?;
    if let Some(new_token) = new_refresh.filter(|t| *t != refresh_token) {
        if let Ok((cipher, nonce)) = vault::encrypt(&new_token) {
            let _ = sqlx::query(
                "UPDATE shop_mini_program_authorizer SET refresh_token_cipher = $1, \
                        refresh_token_nonce = $2, refresh_token_mask = $3, \
                        last_refresh_at = $4, update_time = $5 WHERE id = $6",
            )
            .bind(&cipher)
            .bind(&nonce)
            .bind(vault::mask_of(&new_token))
            .bind(now_i32())
            .bind(now_i32())
            .bind(row.id)
            .execute(&repos.pool)
            .await;
        }
    } else {
        let _ = sqlx::query(
            "UPDATE shop_mini_program_authorizer SET last_refresh_at = $1, update_time = $2 WHERE id = $3",
        )
        .bind(now_i32())
        .bind(now_i32())
        .bind(row.id)
        .execute(&repos.pool)
        .await;
    }
    put_authorizer_token_cache(authorizer_app_id, &token, expires_in).await;
    Ok(token)
}

// ---------------------------------------------------------------------------
// 管理端：列表 / 同步 / 解绑 / 平台配置
// ---------------------------------------------------------------------------

/// 管理端授权列表（`shop_scope` 为商家作用域：Some 时只见本店）。
pub async fn list_authorizations(
    pool: &sqlx::PgPool,
    keyword: Option<&str>,
    shop_scope: Option<i32>,
) -> Result<Vec<AuthorizationItem>> {
    let kw = keyword.map(str::trim).filter(|s| !s.is_empty());
    let rows: Vec<AuthorizationItem> = sqlx::query_as(
        "SELECT a.id, a.shop_id, COALESCE(s.shop_title, '') AS shop_title, \
                a.authorizer_app_id, a.refresh_token_mask, a.nick_name, a.head_img, \
                a.user_name, a.principal_name, a.func_info, a.status, a.authorized_at, \
                a.last_refresh_at, a.add_time, a.update_time \
         FROM shop_mini_program_authorizer a LEFT JOIN shop s ON s.shop_id = a.shop_id \
         WHERE a.is_del = 0 \
           AND ($1::int IS NULL OR a.shop_id = $1) \
           AND ($2::text IS NULL OR a.authorizer_app_id ILIKE $2 \
                OR a.nick_name ILIKE $2 OR COALESCE(s.shop_title, '') ILIKE $2) \
         ORDER BY a.id DESC",
    )
    .bind(shop_scope)
    .bind(kw.map(|k| format!("%{k}%")))
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(rows)
}

/// 同步授权方基本信息（昵称/头像/原始 ID/主体/权限集）回落库。
///
/// # Errors
/// 记录不存在 / 微信 errcode / 数据库失败。
pub async fn sync_authorizer_info(repos: &Repositories, id: i32) -> Result<()> {
    let row = find_authorization_by_id(&repos.pool, id)
        .await?
        .ok_or_else(|| biz("授权记录不存在"))?;
    let info =
        wechat_component::get_authorizer_info(repos, &row.authorizer_app_id).await?;
    let func_json = serde_json::to_string(&info.func_ids).unwrap_or_else(|_| "[]".to_string());
    sqlx::query(
        "UPDATE shop_mini_program_authorizer SET nick_name = $1, head_img = $2, \
                user_name = $3, principal_name = $4, func_info = $5, update_time = $6 \
         WHERE id = $7",
    )
    .bind(&info.nick_name)
    .bind(&info.head_img)
    .bind(&info.user_name)
    .bind(&info.principal_name)
    .bind(&func_json)
    .bind(now_i32())
    .bind(id)
    .execute(&repos.pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?;
    Ok(())
}

/// 本地解绑（status=已解绑；该 appid 立即按未登记拒绝）。微信侧授权关系
/// 需商家在 mp 后台自行取消，本动作不调微信接口（官方无代取消接口）。
///
/// # Errors
/// 记录不存在 / 数据库失败。
pub async fn detach_authorization(pool: &sqlx::PgPool, id: i32) -> Result<()> {
    let row = find_authorization_by_id(pool, id)
        .await?
        .ok_or_else(|| biz("授权记录不存在"))?;
    let affected = sqlx::query(
        "UPDATE shop_mini_program_authorizer SET status = $1, update_time = $2 \
         WHERE id = $3 AND is_del = 0",
    )
    .bind(AUTH_STATUS_DETACHED)
    .bind(now_i32())
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| Error::Database(e.to_string()))?
    .rows_affected();
    if affected == 0 {
        return Err(biz("授权记录不存在"));
    }
    invalidate_authorizer_token(&row.authorizer_app_id).await;
    Ok(())
}

/// 第三方平台配置视图（管理端；凭据只出掩码与已配置标记）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ComponentConfigView {
    pub configured: bool,
    pub component_app_id: String,
    pub app_secret_mask: String,
    pub token_mask: String,
    pub aes_key_mask: String,
    pub redirect_base: String,
    pub missing: Vec<String>,
    /// 是否已收到 verify_ticket（事件通道是否已通）。
    pub ticket_received: bool,
    pub event_path: String,
    pub callback_path: String,
}

/// 读第三方平台配置视图。
///
/// # Errors
/// 数据库读取失败。
pub async fn component_config_view(repos: &Repositories) -> Result<ComponentConfigView> {
    let cfg: ComponentConfig = wechat_component::load_component_config(repos).await?;
    Ok(ComponentConfigView {
        configured: cfg.is_configured(),
        component_app_id: cfg.app_id.clone(),
        app_secret_mask: vault::mask_of(&cfg.app_secret),
        token_mask: vault::mask_of(&cfg.token),
        aes_key_mask: vault::mask_of(&cfg.aes_key),
        redirect_base: cfg.redirect_base.clone(),
        missing: cfg.missing().iter().map(|s| (*s).to_string()).collect(),
        ticket_received: wechat_component::has_verify_ticket().await,
        event_path: wechat_component::EVENT_PATH.to_string(),
        callback_path: wechat_component::AUTH_CALLBACK_PATH.to_string(),
    })
}

/// 第三方平台配置写参数（密钥类字段为空 = 不修改，防掩码回填污染）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ComponentConfigPatch {
    pub component_app_id: Option<String>,
    pub component_app_secret: Option<String>,
    pub component_token: Option<String>,
    pub component_aes_key: Option<String>,
    pub redirect_base: Option<String>,
}

/// 保存第三方平台配置（逐位 save_code；密钥位空值跳过）。保存后清
/// component token 缓存（凭据可能已换）。
///
/// # Errors
/// 数据库写入失败。
pub async fn save_component_config(
    repos: &Repositories,
    patch: &ComponentConfigPatch,
) -> Result<()> {
    let repo = &repos.system.config_basic;
    // 非密钥位：提交了就写（含清空）；密钥位：空值跳过（防掩码回填污染）。
    if let Some(v) = &patch.component_app_id {
        repo.save_code(wechat_component::COMPONENT_APP_ID_CODE, v.trim())
            .await?;
    }
    if let Some(v) = &patch.redirect_base {
        repo.save_code(wechat_component::COMPONENT_REDIRECT_BASE_CODE, v.trim())
            .await?;
    }
    for (code, value) in [
        (
            wechat_component::COMPONENT_APP_SECRET_CODE,
            &patch.component_app_secret,
        ),
        (wechat_component::COMPONENT_TOKEN_CODE, &patch.component_token),
        (
            wechat_component::COMPONENT_AES_KEY_CODE,
            &patch.component_aes_key,
        ),
    ] {
        if let Some(v) = value.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            repo.save_code(code, v).await?;
        }
    }
    wechat_component::invalidate_component_token().await;
    Ok(())
}

/// 服务装配占位（与模式①服务同域并列；实际逻辑走本模块自由函数，
/// 本结构仅为路由层留统一入口，避免改组合根）。
#[derive(Clone)]
pub struct ShopMiniProgramAuthService {
    repos: Arc<Repositories>,
}

impl ShopMiniProgramAuthService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 组合根便捷入口（等价于自由函数 [`create_auth_link`]）。
    pub async fn create_auth_link(&self, shop_id: i32) -> Result<AuthLink> {
        create_auth_link(&self.repos, shop_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_constants_match_migration() {
        assert_eq!(AUTH_STATUS_AUTHORIZED, 1);
        assert_eq!(AUTH_STATUS_REVOKED, 0);
        assert_eq!(AUTH_STATUS_DETACHED, 2);
    }
}
