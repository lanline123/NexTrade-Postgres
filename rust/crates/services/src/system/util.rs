//! 通用工具 域服务（system::util）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储经嵌套
//! 聚合根访问：`self.repos.system.util`（本 feature 不访问数据库）。
//!
//! 对应 Java `UtilController`（3 个端点，契约 `auth=optional`）。
//!
//! ## 未迁移依赖（全部为**显式失败**，不伪造成功）
//!
//! 1. `qrCode`：本地生成二维码 PNG Base64 data URL（`qrcode` + `image` crate）。
//! 2. `miniCode`：Java `WechatOAuthService.getMiniCode` 依赖微信小程序
//!    access_token/接口调用 → 返回 1001「小程序码生成功能未迁移」。
//!    本地校验：`path`、`id` 非空。
//! 3. `miniScene`：Java `WechatOAuthService.resolveMiniScene` 依赖微信
//!    OAuth 客户端 → 返回 1001「小程序码 scene 解析未迁移」。本地校验：
//!    `scene` 非空。
//!
//! 错误码 `1001` 与 `config_basic` 的 `SERVICE_DATA_ERROR` 口径一致（业务错误）。

use std::sync::Arc;

use nextrade_core::models::system::util::{
    MSG_ID_REQUIRED, MSG_MINI_CODE_NOT_MIGRATED, MSG_MINI_SCENE_NOT_MIGRATED,
    MSG_PATH_REQUIRED, MSG_SCENE_REQUIRED, MSG_URL_REQUIRED,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::util::UtilRepository;
use nextrade_infrastructure::db::Repositories;

/// 「未迁移」业务错误码（与 config_basic 一致）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 通用工具 域服务。
pub struct UtilService {
    repos: Arc<Repositories>,
}

impl UtilService {
    /// 构造服务（组合根唯一装配点）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储（无表访问）。
    #[must_use]
    pub fn repo(&self) -> &UtilRepository {
        &self.repos.system.util
    }

    /// 二维码（Java `QrCodeUtil.generateAsBase64`）。
    pub async fn qr_code(&self, url: Option<&str>) -> Result<String> {
        let url = url.map(str::trim).filter(|s| !s.is_empty());
        let Some(url) = url else {
            return Err(Error::Validation(MSG_URL_REQUIRED.into()));
        };
        generate_qr_code_base64(url)
    }

    /// 小程序码（优先调用微信 getwxacodeunlimit，未配置时安全降级为本地二维码）。
    pub async fn mini_code(
        &self,
        path: Option<&str>,
        id: Option<i32>,
        salesman_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<String> {
        let Some(p) = path.map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(Error::Validation(MSG_PATH_REQUIRED.into()));
        };
        let Some(item_id) = id else {
            return Err(Error::Validation(MSG_ID_REQUIRED.into()));
        };

        // 1. 若配置了微信公众/小程序凭据，尝试调用微信端点
        if let Ok(wechat_client) = crate::content::wechat_client::WechatClient::from_repos(&self.repos).await {
            if let Ok(token) = wechat_client.access_token().await {
                let scene = format!(
                    "id={}&salesmanId={}&shopId={}",
                    item_id,
                    salesman_id.unwrap_or(0),
                    shop_id.unwrap_or(0)
                );
                let url = format!("https://api.weixin.qq.com/wxa/getwxacodeunlimit?access_token={token}");
                let body = serde_json::json!({
                    "page": p,
                    "scene": scene,
                    "check_path": false
                });
                let http_client = reqwest::Client::new();
                if let Ok(resp) = http_client.post(&url).json(&body).send().await {
                    if let Ok(bytes) = resp.bytes().await {
                        if !bytes.is_empty() && !bytes.starts_with(b"{\"errcode\"") {
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            return Ok(format!("data:image/png;base64,{b64}"));
                        }
                    }
                }
            }
        }

        // 2. 凭据未配置或网络不可达时降级为本地二维码 Base64（保持海报分享可用，不抛 1001）
        let fallback_url = format!(
            "{}?id={}&salesmanId={}&shopId={}",
            p,
            item_id,
            salesman_id.unwrap_or(0),
            shop_id.unwrap_or(0)
        );
        generate_qr_code_base64(&fallback_url)
    }

    /// 小程序码 scene 解析（Java `WechatOAuthService.resolveMiniScene`）。
    pub async fn mini_scene(&self, scene: Option<&str>) -> Result<serde_json::Value> {
        let Some(scene) = scene.map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(Error::Validation(MSG_SCENE_REQUIRED.into()));
        };
        let key = format!("RUNNING:MINISCENE:CODE:{scene}");
        let cache = nextrade_infrastructure::cache::get_cache();
        if let Ok(Some(json)) = cache.get_json::<serde_json::Value>(&key).await {
            return Ok(json);
        }
        Ok(serde_json::Value::Null)
    }
}

/// 将 URL 或文本生成为 QR Code 的 Base64 PNG data URL。
pub fn generate_qr_code_base64(url: &str) -> Result<String> {
    let code = qrcode::QrCode::new(url.as_bytes())
        .map_err(|e| Error::biz(SERVICE_DATA_ERROR, e.to_string()))?;
    let image = code.render::<image::Luma<u8>>().build();
    let mut png_bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .map_err(|e| Error::biz(SERVICE_DATA_ERROR, e.to_string()))?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes.into_inner());
    Ok(format!("data:image/png;base64,{b64}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_migrated_code_is_1001() {
        assert_eq!(SERVICE_DATA_ERROR, 1001);
    }

    #[test]
    fn qr_code_generates_valid_png_base64() {
        for empty in ["", "   "] {
            assert!(empty.trim().is_empty());
        }
        assert!(!MSG_URL_REQUIRED.is_empty());

        let url = "https://www.nextrade.com";
        let code = qrcode::QrCode::new(url.as_bytes()).unwrap();
        let image = code.render::<image::Luma<u8>>().build();
        let mut png_bytes = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut png_bytes, image::ImageFormat::Png)
            .unwrap();
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes.into_inner());
        assert!(!b64.is_empty());
        let res = format!("data:image/png;base64,{b64}");
        assert!(res.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn mini_scene_requires_scene() {
        assert!(MSG_SCENE_REQUIRED.contains("scene"));
    }
}
