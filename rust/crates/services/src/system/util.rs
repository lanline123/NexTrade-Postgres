//! 通用工具 域服务（system::util）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储经嵌套
//! 聚合根访问：`self.repos.system.util`（本 feature 不访问数据库）。
//!
//! 对应 Java `UtilController`（3 个端点，契约 `auth=optional`），另新增
//! 我方 `shareLink` 端点（分享直达小程序，论坛帖 907763）。
//!
//! ## 小程序分享口径（2026-11-21 本批落地）
//!
//! 1. `miniCode`：经 [`crate::content::wechat_mini`] 以**小程序凭据**
//!    （`wechatMiniProgramAppId`/`wechatMiniProgramSecret`）调
//!    `getwxacodeunlimit` 生成真小程序码。此前实现误用公众号凭据
//!    （`wechatAppId`）调 wxa 接口必然被拒，且失败被静默吞掉、回退成
//!    相对路径二维码——属假成功，本批纠正。
//! 2. `shareLink`（新增）：结构化返回小程序 URL Link + 小程序码 +
//!    H5 链接，并以 `channel/degraded/notice` **明示降级**：未配置
//!    小程序或微信接口失败时 `channel=h5`、链接/码为 null、分享目标
//!    改用 H5 链接，绝不伪造小程序直达成功。
//! 3. `miniCode` 旧端点只回图片字符串、无处标注降级：未配置/失败时
//!    回退为 **H5 链接的二维码**（扫码落 H5 页面，与标注口径一致的
//!    真降级），标注面在 `shareLink` 与前端分享面板。
//! 4. `miniScene`：query 形式 scene 直接解析；短码查
//!    `RUNNING:MINISCENE:CODE:{短码}` 映射（生成码时由本服务写入，
//!    此前只有读者没有写者，短码恒解析为空，本批补齐写端）。
//!
//! 错误码 `1001` 与 `config_basic` 的 `SERVICE_DATA_ERROR` 口径一致（业务错误）。

use std::sync::Arc;

use nextrade_core::models::system::util::{
    ShareLinkVO, MSG_ID_REQUIRED, MSG_PATH_REQUIRED, MSG_SCENE_REQUIRED, MSG_URL_REQUIRED,
    SHARE_CHANNEL_H5, SHARE_CHANNEL_MINI_PROGRAM,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::util::UtilRepository;
use nextrade_infrastructure::db::Repositories;

use crate::content::wechat_mini::{
    build_h5_url, join_scene_pairs, load_config_value, load_mini_config, parse_scene_query,
    plan_scene, scene_cache_key, scene_pairs, scene_payload_from_pairs, SceneParams, ScenePlan,
    WechatMiniClient, ENV_VERSION_RELEASE, H5_DOMAIN_CODE, SCENE_TTL_SECS, URL_LINK_EXPIRE_DAYS,
};

/// 业务错误码（与 config_basic 一致）。
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

    /// 小程序码（旧端点，契约只回图片 data URL）。
    ///
    /// 配置齐全时为真小程序码；未配置/微信失败时降级为 H5 分享链接的
    /// 二维码（扫码落 H5 页面）。降级的**标注**走 [`Self::share_link`]
    /// 的结构化响应与前端分享面板（本端点返回类型无标注位）。
    pub async fn mini_code(
        &self,
        path: Option<&str>,
        id: Option<i32>,
        salesman_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<String> {
        let link = self
            .share_link_inner(path, id, salesman_id, shop_id, false, true)
            .await?;
        if let Some(code) = link.mini_code {
            return Ok(code);
        }
        generate_qr_code_base64(&link.h5_url)
    }

    /// 分享直达小程序（我方新增）：小程序 URL Link + 小程序码 + H5 链接。
    ///
    /// 未配置小程序或微信接口失败时如实降级（`channel=h5`、
    /// `degraded=true`、`notice` 写明原因），不伪造成功。
    pub async fn share_link(
        &self,
        path: Option<&str>,
        id: Option<i32>,
        salesman_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<ShareLinkVO> {
        self.share_link_inner(path, id, salesman_id, shop_id, true, true)
            .await
    }

    /// 分享结果组装：`want_link`/`want_code` 控制向微信申请的产物
    /// （旧 `miniCode` 端点只要码，避免无谓消耗 URL Link 配额）。
    async fn share_link_inner(
        &self,
        path: Option<&str>,
        id: Option<i32>,
        salesman_id: Option<i32>,
        shop_id: Option<i32>,
        want_link: bool,
        want_code: bool,
    ) -> Result<ShareLinkVO> {
        let Some(raw_path) = path.map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(Error::Validation(MSG_PATH_REQUIRED.into()));
        };
        let Some(item_id) = id else {
            return Err(Error::Validation(MSG_ID_REQUIRED.into()));
        };
        // path 可内嵌 query（拼团海报等旧调用方）：page 不得带参数
        // （微信硬性规定），内嵌参数并入 query/scene 承载。
        let (page, embedded_query) = split_page_query(raw_path);
        let params = SceneParams {
            id: item_id,
            salesman_id: salesman_id.unwrap_or(0),
            shop_id: shop_id.unwrap_or(0),
        };
        // 参数对：内嵌页面参数在前 + 归因参数在后；拼出的全量串同时
        // 是 URL Link / H5 的落地 query，也是 scene 的规划输入。
        let pairs = scene_pairs(embedded_query.as_deref(), &params);
        let query = join_scene_pairs(&pairs);
        // scene 规划：≤32 字符直用；超限用短码并把参数映射写入缓存
        // （miniScene 的读取端此前已就绪，本批补写端）。
        let scene = match plan_scene(&query) {
            ScenePlan::Direct(full) => full,
            ScenePlan::ShortCode(code) => {
                let cache = nextrade_infrastructure::cache::get_cache();
                if let Err(e) = cache
                    .set_json(
                        &scene_cache_key(&code),
                        &scene_payload_from_pairs(&pairs),
                        SCENE_TTL_SECS,
                    )
                    .await
                {
                    tracing::warn!("小程序 scene 短码映射写入失败（{code}）: {e}");
                }
                code
            }
        };
        // H5 回退链接（恒组装，降级时即分享目标）。
        let h5_domain = load_config_value(&self.repos, H5_DOMAIN_CODE)
            .await
            .unwrap_or_default();
        let h5_url = build_h5_url(&h5_domain, &page, &query);

        let mut vo = ShareLinkVO {
            channel: SHARE_CHANNEL_H5.to_string(),
            degraded: true,
            notice: None,
            url_link: None,
            mini_code: None,
            h5_url,
            scene,
        };

        // 小程序凭据：未配置 → 明示降级（铁律：不假成功）。
        let cfg = match load_mini_config(&self.repos).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!("读取微信小程序配置失败，分享降级 H5: {e}");
                vo.notice = Some("小程序分享暂不可用，已回退 H5 分享链接".to_string());
                return Ok(vo);
            }
        };
        if !cfg.is_configured() {
            vo.notice = Some(format!(
                "微信小程序未配置（缺少 {}），已回退 H5 分享链接",
                cfg.missing().join("/")
            ));
            return Ok(vo);
        }

        // 配置齐全：真调微信。任一产物失败 → 该产物置空并标注降级原因；
        // 全部失败 → channel 保持 h5（分享目标即 h5Url）。
        let client = WechatMiniClient::new(cfg);
        let mut failures: Vec<String> = Vec::new();
        if want_link {
            match client
                .generate_url_link(&page, &query, ENV_VERSION_RELEASE, URL_LINK_EXPIRE_DAYS)
                .await
            {
                Ok(link) => vo.url_link = Some(link),
                Err(e) => failures.push(format!("URL Link 生成失败：{e}")),
            }
        }
        if want_code {
            match client
                .wxacode_unlimit(&vo.scene, &page, ENV_VERSION_RELEASE)
                .await
            {
                Ok(bytes) => vo.mini_code = Some(png_data_url(&bytes)),
                Err(e) => failures.push(format!("小程序码生成失败：{e}")),
            }
        }
        if vo.url_link.is_some() || vo.mini_code.is_some() {
            vo.channel = SHARE_CHANNEL_MINI_PROGRAM.to_string();
        }
        if failures.is_empty() {
            vo.degraded = false;
        } else {
            vo.notice = Some(format!("{}，已回退 H5 分享链接", failures.join("；")));
        }
        Ok(vo)
    }

    /// 小程序码 scene 解析（Java `WechatOAuthService.resolveMiniScene`）。
    ///
    /// query 形式（`id=..&salesmanId=..&shopId=..`）直接解析；短码查
    /// 生成时写入的缓存映射；都未命中返回 `null`（与前端容错口径一致）。
    pub async fn mini_scene(&self, scene: Option<&str>) -> Result<serde_json::Value> {
        let Some(scene) = scene.map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(Error::Validation(MSG_SCENE_REQUIRED.into()));
        };
        if let Some(parsed) = parse_scene_query(scene) {
            return Ok(parsed);
        }
        let cache = nextrade_infrastructure::cache::get_cache();
        if let Ok(Some(json)) = cache
            .get_json::<serde_json::Value>(&scene_cache_key(scene))
            .await
        {
            return Ok(json);
        }
        Ok(serde_json::Value::Null)
    }
}

/// 拆分页面路径与内嵌 query：`pages/a/b?x=1` → (`pages/a/b`, Some(`x=1`))。
fn split_page_query(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim().trim_start_matches('/');
    match trimmed.split_once('?') {
        Some((page, query)) => (
            page.to_string(),
            Some(query.to_string()).filter(|q| !q.is_empty()),
        ),
        None => (trimmed.to_string(), None),
    }
}

/// PNG 字节 → data URL（与既有小程序码返回口径一致）。
fn png_data_url(bytes: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/png;base64,{b64}")
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
    Ok(png_data_url(&png_bytes.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_data_error_code_is_1001() {
        assert_eq!(SERVICE_DATA_ERROR, 1001);
    }

    #[test]
    fn qr_code_generates_valid_png_base64() {
        for empty in ["", "   "] {
            assert!(empty.trim().is_empty());
        }
        assert!(!MSG_URL_REQUIRED.is_empty());

        let url = "https://www.nextrade.com";
        let res = generate_qr_code_base64(url).unwrap();
        assert!(res.starts_with("data:image/png;base64,"));
        assert!(res.len() > 100);
    }

    #[test]
    fn split_page_query_separates_embedded_params() {
        let (page, q) = split_page_query("/pages/product/groupBuying?grouponPromotionRecordId=9");
        assert_eq!(page, "pages/product/groupBuying");
        assert_eq!(q.as_deref(), Some("grouponPromotionRecordId=9"));
        let (page, q) = split_page_query("pages/product/index");
        assert_eq!(page, "pages/product/index");
        assert!(q.is_none());
    }

    #[test]
    fn mini_scene_requires_scene() {
        assert!(MSG_SCENE_REQUIRED.contains("scene"));
    }
}
