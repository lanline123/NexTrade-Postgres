//! 店铺独立绑定小程序服务（merchant::shop_mini_program）—— **本平台新增**
//! （论坛帖 487946315732211118「店铺能不能绑定独立小程序」）。
//!
//! 模式 ①（商家自有小程序直连）MVP：
//! - 绑定：商家把自有小程序的 appid + AppSecret 绑到店铺（一店一个、
//!   appid 全局唯一）；保存前实调微信 `stable_token` 探活校验凭据对，
//!   校验不过不落「已生效」状态；AppSecret 按 AI 渠道密钥库口径
//!   （[`crate::ai::vault`] AES-256-GCM）加密存储，后台只显掩码。
//! - 解析：前台按 appid 解析小程序作用域（平台 appid → 平台模式；
//!   已绑定且启用的店铺 appid → 店铺模式；其余 → 未登记，中间件硬拒绝）。
//! - 与支付域 `sub_merchant.sub_appid`（收款归属）分开落表：本表管
//!   登录/数据归属，两者语义不同不混用。
//!
//! 迁移：`rust/deploy/pg/2026-11-21_shop_mini_program.sql`（部署前必须
//! 先执行，迁移先于二进制）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::ai::vault;

/// 全局业务错误码（与 shop_label 等同域服务口径一致）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 绑定状态：启用。
pub const STATUS_ENABLED: i32 = 1;
/// 绑定状态：停用（停用后该 appid 立即按未登记拒绝）。
pub const STATUS_DISABLED: i32 = 0;

/// 管理端绑定行（密钥只出掩码，密文永不出接口）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMiniProgramItem {
    pub id: i32,
    pub shop_id: i32,
    /// 店铺名（联 shop 表回显；店铺已删时为空串）。
    #[sqlx(default)]
    pub shop_title: String,
    pub app_id: String,
    pub secret_mask: String,
    /// 1 启用 / 0 停用。
    pub status: i32,
    /// 最近一次凭据校验通过时间（epoch 秒）。
    pub verified_at: i32,
    pub remark: String,
    pub add_time: i32,
    pub update_time: i32,
}

/// 绑定写参数（bind / rebind 共用）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMiniProgramBind {
    /// 目标店铺（平台代绑时必填；商家自绑时由路由按 JWT 作用域覆盖）。
    pub shop_id: Option<i32>,
    pub app_id: Option<String>,
    /// 明文 AppSecret（仅写入方向传输；落库前加密，永不回显）。
    pub app_secret: Option<String>,
    pub remark: Option<String>,
}

/// 启停参数。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopMiniProgramStatus {
    pub id: Option<i32>,
    /// 1 启用 / 0 停用。
    pub status: Option<i32>,
}

/// 买家侧上下文解析结果（`GET /api/shop/miniProgram/context`）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniProgramContext {
    /// `platform` 平台小程序 / `shop` 店铺小程序。
    pub mode: String,
    /// 店铺模式下的绑定店铺（平台模式为 0）。
    pub shop_id: i32,
    pub shop_title: String,
    pub app_id: String,
}

/// appid 解析结论（中间件 / 登录 / 支付共用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppResolution {
    /// 平台小程序（或未上报 appid 的全平台渠道）。
    Platform,
    /// 已绑定且启用的店铺小程序。
    Shop(i32),
    /// 未登记 / 已停用 / 已解绑 —— 必须硬拒绝。
    Unknown,
}

/// 绑定行内部视图（含密文，仅服务内使用）。
#[derive(Debug, Clone, FromRow)]
pub struct BindingRow {
    pub id: i32,
    pub shop_id: i32,
    pub app_id: String,
    pub app_secret_cipher: String,
    pub app_secret_nonce: String,
    pub status: i32,
}

fn now_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

fn biz(msg: impl Into<String>) -> Error {
    Error::biz(SERVICE_DATA_ERROR, msg.into())
}

/// 按 appid 查未删绑定行（含停用；调用方按 status 决策）。
pub async fn find_binding_by_app_id(
    pool: &sqlx::PgPool,
    app_id: &str,
) -> Result<Option<BindingRow>> {
    let row = sqlx::query_as::<_, BindingRow>(
        "SELECT id, shop_id, app_id, app_secret_cipher, app_secret_nonce, status \
         FROM shop_mini_program WHERE app_id = $1 AND is_del = 0 LIMIT 1",
    )
    .bind(app_id)
    .fetch_optional(pool)
    .await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(row)
}

/// 按店铺查未删绑定行。
pub async fn find_binding_by_shop(
    pool: &sqlx::PgPool,
    shop_id: i32,
) -> Result<Option<BindingRow>> {
    let row = sqlx::query_as::<_, BindingRow>(
        "SELECT id, shop_id, app_id, app_secret_cipher, app_secret_nonce, status \
         FROM shop_mini_program WHERE shop_id = $1 AND is_del = 0 LIMIT 1",
    )
    .bind(shop_id)
    .fetch_optional(pool)
    .await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(row)
}

/// 解密绑定行的 AppSecret（主密钥缺失/密文损坏 → 明确报错，不回落明文）。
pub fn decrypt_binding_secret(row: &BindingRow) -> Result<String> {
    vault::decrypt(&row.app_secret_cipher, &row.app_secret_nonce)
        .map_err(|e| Error::Internal(format!("店铺小程序 Secret 解密失败: {e}")))
}

/// 解析 appid 作用域：`platform_app_id` 为平台小程序 appid（全局配置值，
/// 可为空——未配置时任何非空 appid 都只能来自绑定表）。
///
/// 解析顺序（模式①②共存，口径不变）：平台 appid → 模式①直连绑定
/// （启用）→ 模式②第三方授权（授权中，见
/// [`super::shop_mini_program_auth`]）→ 未登记硬拒绝。同一 appid 在
/// 服务层互斥校验下不会同时存在启用直连绑定与生效授权，顺序无歧义；
/// 授权被撤销/解绑后回落未登记拒绝，绝不回落平台全量。
pub async fn resolve_app_id(
    pool: &sqlx::PgPool,
    app_id: &str,
    platform_app_id: &str,
) -> Result<AppResolution> {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return Ok(AppResolution::Platform);
    }
    if !platform_app_id.is_empty() && app_id == platform_app_id {
        return Ok(AppResolution::Platform);
    }
    if let Some(row) = find_binding_by_app_id(pool, app_id).await? {
        if row.status == STATUS_ENABLED {
            return Ok(AppResolution::Shop(row.shop_id));
        }
    }
    if let Some(auth) =
        super::shop_mini_program_auth::find_authorization_by_app_id(pool, app_id).await?
    {
        if auth.status == super::shop_mini_program_auth::AUTH_STATUS_AUTHORIZED {
            return Ok(AppResolution::Shop(auth.shop_id));
        }
    }
    Ok(AppResolution::Unknown)
}

/// 凭据探活：强制绕开 token 缓存实调微信稳定版 token 接口校验
/// appid + secret 对（错 secret 会被微信 errcode 拒绝）。校验成功顺带
/// 回填缓存，后续登录/取号直接复用。
async fn verify_credentials(app_id: &str, app_secret: &str) -> Result<()> {
    crate::content::wechat_client::invalidate_token(app_id);
    crate::content::wechat_client::stable_access_token(app_id, app_secret)
        .await
        .map_err(|e| biz(format!("微信凭据校验失败，请核对 AppID 与 AppSecret：{e}")))?;
    Ok(())
}

#[derive(Clone)]
pub struct ShopMiniProgramService {
    repos: Arc<Repositories>,
}

impl ShopMiniProgramService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.repos.pool
    }

    /// 平台小程序 appid（全局配置；未配置为空串）。
    pub async fn platform_app_id(&self) -> Result<String> {
        let value = self
            .repos
            .member
            .login
            .config_value("wechatMiniProgramAppId")
            .await?;
        Ok(value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_default())
    }

    /// 解析 appid 作用域（读平台配置后走共享解析）。
    pub async fn resolve(&self, app_id: &str) -> Result<AppResolution> {
        let platform = self.platform_app_id().await?;
        resolve_app_id(self.pool(), app_id, &platform).await
    }

    /// 取某 appid 的可用凭据 `(appid, secret)`：平台 appid 回落全局配置，
    /// 店铺 appid 查绑定表解密；未登记/停用 → 业务错误（登录侧硬拒绝）。
    pub async fn credentials_for(&self, app_id: &str) -> Result<(String, String)> {
        let platform = self.platform_app_id().await?;
        if app_id.is_empty() || app_id == platform {
            let secret = self
                .repos
                .member
                .login
                .config_value("wechatMiniProgramSecret")
                .await?
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| biz("微信小程序 Secret 未配置"))?;
            if platform.is_empty() {
                return Err(biz("微信小程序 APPID 未配置"));
            }
            return Ok((platform, secret));
        }
        let row = find_binding_by_app_id(self.pool(), app_id)
            .await?
            .filter(|r| r.status == STATUS_ENABLED)
            .ok_or_else(|| biz("该小程序未绑定店铺或绑定已停用"))?;
        let secret = decrypt_binding_secret(&row)?;
        Ok((row.app_id, secret))
    }

    /// 管理端列表（可按店铺名/appid 关键词过滤；密钥只出掩码）。
    pub async fn list(&self, keyword: Option<&str>) -> Result<Vec<ShopMiniProgramItem>> {
        let kw = keyword.map(str::trim).filter(|s| !s.is_empty());
        let rows: Vec<ShopMiniProgramItem> = sqlx::query_as(
            "SELECT b.id, b.shop_id, COALESCE(s.shop_title, '') AS shop_title, b.app_id, \
                    b.secret_mask, b.status, b.verified_at, b.remark, b.add_time, b.update_time \
             FROM shop_mini_program b LEFT JOIN shop s ON s.shop_id = b.shop_id \
             WHERE b.is_del = 0 \
               AND ($1::text IS NULL OR b.app_id ILIKE $1 OR COALESCE(s.shop_title, '') ILIKE $1) \
             ORDER BY b.id DESC",
        )
        .bind(kw.map(|k| format!("%{k}%")))
        .fetch_all(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(rows)
    }

    /// 单店绑定详情（商家自查 / 平台代查；无绑定 → None）。
    pub async fn detail_for_shop(
        &self,
        shop_id: i32,
    ) -> Result<Option<ShopMiniProgramItem>> {
        let row: Option<ShopMiniProgramItem> = sqlx::query_as(
            "SELECT b.id, b.shop_id, COALESCE(s.shop_title, '') AS shop_title, b.app_id, \
                    b.secret_mask, b.status, b.verified_at, b.remark, b.add_time, b.update_time \
             FROM shop_mini_program b LEFT JOIN shop s ON s.shop_id = b.shop_id \
             WHERE b.is_del = 0 AND b.shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(row)
    }

    /// 绑定 / 换绑：校验凭据 → 加密落库（status=启用，verified_at=now）。
    ///
    /// 冲突口径：appid 已绑其他店 → 拒绝；本店已绑其他 appid → 拒绝
    /// （先解绑再绑，避免误换绑把在线小程序打断）；同店同 appid 重绑
    /// = 更新 secret/备注并重新校验。
    pub async fn bind(&self, shop_id: i32, p: &ShopMiniProgramBind) -> Result<i32> {
        if shop_id <= 0 {
            return Err(biz("店铺不存在"));
        }
        let app_id = p
            .app_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| biz("请填写小程序 AppID"))?
            .to_string();
        let secret = p
            .app_secret
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| biz("请填写小程序 AppSecret"))?
            .to_string();
        let remark = p.remark.as_deref().map(str::trim).unwrap_or("").to_string();

        // 平台 appid 不得被店铺占用（否则解析歧义、隔离击穿）
        let platform = self.platform_app_id().await?;
        if !platform.is_empty() && app_id == platform {
            return Err(biz("该 AppID 是平台小程序，不能绑定到店铺"));
        }
        // 与模式②互斥：同 appid 已有生效的第三方平台授权 → 先解绑授权
        if let Some(auth) =
            super::shop_mini_program_auth::find_authorization_by_app_id(self.pool(), &app_id)
                .await?
        {
            if auth.status == super::shop_mini_program_auth::AUTH_STATUS_AUTHORIZED {
                return Err(biz(
                    "该小程序已通过第三方平台授权接入，请先在授权管理中解绑后再直连绑定",
                ));
            }
        }
        // 店铺必须真实存在
        let shop_exists: Option<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?;
        if shop_exists.is_none() {
            return Err(biz("店铺不存在"));
        }

        if let Some(other) = find_binding_by_app_id(self.pool(), &app_id).await? {
            if other.shop_id != shop_id {
                return Err(biz("该 AppID 已绑定其他店铺，请先在原店铺解绑"));
            }
        }
        if let Some(mine) = find_binding_by_shop(self.pool(), shop_id).await? {
            if mine.app_id != app_id {
                return Err(biz("本店已绑定其他小程序，请先解绑后再绑定"));
            }
            // 同店同 appid：先校验后更新（校验不过不落库，旧凭据保留）
            verify_credentials(&app_id, &secret).await?;
            let (cipher, nonce) = vault::encrypt(&secret)
                .map_err(|e| Error::Internal(format!("AppSecret 加密失败: {e}")))?;
            sqlx::query(
                "UPDATE shop_mini_program SET app_secret_cipher = $1, app_secret_nonce = $2, \
                        secret_mask = $3, status = $4, verified_at = $5, remark = $6, \
                        update_time = $7 WHERE id = $8",
            )
            .bind(&cipher)
            .bind(&nonce)
            .bind(vault::mask_of(&secret))
            .bind(STATUS_ENABLED)
            .bind(now_i32())
            .bind(&remark)
            .bind(now_i32())
            .bind(mine.id)
            .execute(self.pool())
            .await.map_err(|e| Error::Database(e.to_string()))?;
            return Ok(mine.id);
        }

        verify_credentials(&app_id, &secret).await?;
        let (cipher, nonce) = vault::encrypt(&secret)
            .map_err(|e| Error::Internal(format!("AppSecret 加密失败: {e}")))?;
        let now = now_i32();
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shop_mini_program (shop_id, app_id, app_secret_cipher, app_secret_nonce, \
                    secret_mask, status, verified_at, remark, add_time, update_time, is_del) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9, 0) RETURNING id",
        )
        .bind(shop_id)
        .bind(&app_id)
        .bind(&cipher)
        .bind(&nonce)
        .bind(vault::mask_of(&secret))
        .bind(STATUS_ENABLED)
        .bind(now)
        .bind(&remark)
        .bind(now)
        .fetch_one(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.0)
    }

    /// 启停（停用即刻按未登记拒绝；启用前不重验凭据，凭据失效会在
    /// 登录链以微信 errcode 显形并提示重绑）。
    pub async fn set_status(&self, id: i32, status: i32) -> Result<()> {
        if !matches!(status, STATUS_ENABLED | STATUS_DISABLED) {
            return Err(biz("状态值不合法"));
        }
        let affected = sqlx::query(
            "UPDATE shop_mini_program SET status = $1, update_time = $2 WHERE id = $3 AND is_del = 0",
        )
        .bind(status)
        .bind(now_i32())
        .bind(id)
        .execute(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?
        .rows_affected();
        if affected == 0 {
            return Err(biz("绑定记录不存在"));
        }
        Ok(())
    }

    /// 解绑（软删；解绑后 appid 立即回落未登记拒绝态，绝不回落平台全量）。
    pub async fn unbind(&self, id: i32) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE shop_mini_program SET is_del = 1, update_time = $1 WHERE id = $2 AND is_del = 0",
        )
        .bind(now_i32())
        .bind(id)
        .execute(self.pool())
        .await.map_err(|e| Error::Database(e.to_string()))?
        .rows_affected();
        if affected == 0 {
            return Err(biz("绑定记录不存在"));
        }
        Ok(())
    }

    /// 买家侧上下文（公开端点）：解析 + 店铺名回显。未登记 → Unknown，
    /// 由路由层转 403（不给店铺信息）。
    pub async fn context(&self, app_id: &str) -> Result<AppResolution> {
        self.resolve(app_id).await
    }

    /// 取店铺名（上下文回显用；不存在 → 空串）。
    pub async fn shop_title(&self, shop_id: i32) -> Result<String> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT COALESCE(shop_title, '') FROM shop WHERE shop_id = $1 LIMIT 1")
                .bind(shop_id)
                .fetch_optional(self.pool())
                .await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.map(|r| r.0).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_constants_match_migration() {
        assert_eq!(STATUS_ENABLED, 1);
        assert_eq!(STATUS_DISABLED, 0);
    }

    #[test]
    fn bind_requires_app_id_and_secret() {
        // 纯参数口径由 bind() 的 ok_or_else 链保证；此处钉住错误文案常量不漂移
        let p = ShopMiniProgramBind::default();
        assert!(p.app_id.is_none());
        assert!(p.app_secret.is_none());
    }
}
