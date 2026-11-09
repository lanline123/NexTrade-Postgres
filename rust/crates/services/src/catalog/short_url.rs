//! 短链接域服务（catalog::short_url）。
//!
//! 对应原 Java `com.nextrade.api.controller.product.ShortUrlController#redirect`
//! （`@GetMapping("/s/{code}")`，**不带** `/api` 前缀，故挂在应用根路径）。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`ShortUrlService::resolve_redirect`] | `ShortUrlController.redirect` 全链路 |
//!
//! # 与 Java 的对应关系（逐条）
//!
//! 1. **查短链**：Java `ShortUrlServiceImpl#getByCode` 用 `like`（模糊匹配）
//!    `short_code` + `limit 1`，仓储 [`ShortUrlRepository::find_by_code`] 照抄。
//! 2. **未命中**：Java `Assert.notNull(shortUrl, () -> new GlobalException("短链接暂时无法访问，请稍后再试..."))`
//!    —— 本实现返回同一文案的业务错误。
//! 3. **H5 分支**（`type == null || type <= 1`）：取 `config` 表 `h5Domain`，
//!    为空则报 `请先配置H5域名`；目标地址为
//!    `h5Domain + longUrl.replace("/mobile", "")`（Java `StrUtil.replace` 为
//!    **全量**替换，故用 `str::replace`）。
//! 4. **PC 分支**（`type > 1`）：取 `pcDomain`，为空报 `请先配置PC域名`；
//!    目标地址为 `pcDomain + longUrl`（不做替换）。
//! 5. **跳转语义**：Java `response.sendRedirect(url)` 是 **HTTP 302** + `Location`，
//!    见路由层 `crates/api/public/src/root/short_url.rs`（不是 JSON 包装）。
//! 6. **免登录**：`/s/**` 在 Java `WhiteListConfig.URL_WHITELIST` 内，
//!    Rust 侧由 `root::merge_public` 挂载，不加 `require_auth`。
//! 7. **域名拼接**：Java 直接字符串相加，不做「是否以 `/` 结尾」的规范化，
//!    本实现同样直接相加，避免擅自改写历史短链的落地地址。

use std::sync::Arc;

use nextrade_infrastructure::db::Repositories;
use nextrade_core::{Error, Result};

/// Java `SettingsEnum.H5_DOMAIN` 的 `biz_code`。
const H5_DOMAIN: &str = "h5Domain";
/// Java `SettingsEnum.PC_DOMAIN` 的 `biz_code`。
const PC_DOMAIN: &str = "pcDomain";

/// 短链接服务。
pub struct ShortUrlService {
    repos: Arc<Repositories>,
}

impl ShortUrlService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 解析短码 → 302 目标地址。
    pub async fn resolve_redirect(&self, code: &str) -> Result<String> {
        let row = self
            .repos.catalog.short_url
            .find_by_code(code)
            .await?
            .ok_or_else(|| Error::biz_msg("短链接暂时无法访问，请稍后再试..."))?;

        Ok(resolve_target(
            row.url_type,
            &row.long_url,
            &self.config_val(H5_DOMAIN).await?,
            &self.config_val(PC_DOMAIN).await?,
        )?)
    }

    /// Java `ConfigService.getConfigVal`：缺行 → 空串。
    async fn config_val(&self, biz_code: &str) -> Result<String> {
        Ok(self
            .repos.catalog.short_url
            .find_config_val(biz_code)
            .await?
            .unwrap_or_default())
    }
}

/// 纯逻辑：按 Java 的分支口径拼出目标地址（拆出来便于单测）。
///
/// `h5` / `pc` 分别对应 `h5Domain` / `pcDomain` 的配置值（缺配置为空串）。
pub fn resolve_target(
    url_type: Option<i32>,
    long_url: &str,
    h5: &str,
    pc: &str,
) -> Result<String> {
    if url_type.unwrap_or_default() <= 1 {
        if h5.trim().is_empty() {
            return Err(Error::biz_msg("请先配置H5域名"));
        }
        Ok(format!("{}{}", h5, long_url.replace("/mobile", "")))
    } else {
        if pc.trim().is_empty() {
            return Err(Error::biz_msg("请先配置PC域名"));
        }
        Ok(format!("{}{}", pc, long_url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_type_uses_h5_and_strips_mobile_segment() {
        let got = resolve_target(None, "/mobile/product/detail?id=1", "https://h5.demo", "").unwrap();
        assert_eq!(got, "https://h5.demo/product/detail?id=1");
    }

    #[test]
    fn type_one_is_still_h5() {
        let got = resolve_target(Some(1), "/mobile/a", "https://h5.demo", "").unwrap();
        assert_eq!(got, "https://h5.demo/a");
    }

    #[test]
    fn type_two_uses_pc_without_replace() {
        let got = resolve_target(Some(2), "/mobile/a", "https://h5.demo", "https://pc.demo").unwrap();
        assert_eq!(got, "https://pc.demo/mobile/a");
    }

    #[test]
    fn missing_h5_domain_keeps_java_message() {
        let err = resolve_target(Some(0), "/mobile/a", "  ", "").unwrap_err();
        assert!(err.to_string().contains("请先配置H5域名"));
    }

    #[test]
    fn missing_pc_domain_keeps_java_message() {
        let err = resolve_target(Some(3), "/a", "https://h5.demo", "").unwrap_err();
        assert!(err.to_string().contains("请先配置PC域名"));
    }
}