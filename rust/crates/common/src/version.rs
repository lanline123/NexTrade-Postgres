//! 产品版本戳（admin 自助热更新 P1 版本感知，我方新增）。
//!
//! 设计见 `~/workspace/hot-update-design/REPORT.md` §1.2 / §3：四面统一产品
//! 版本号，后端在编译期把「产品版本 + git commit + CI 构建号 + 构建时刻」
//! 嵌入二进制，经 `/version` 端点（admin 与商城两端各一）自报，updater 的
//! 健康检查与 admin「系统更新」页据此核对线上版本，终结靠外部实测指纹。
//!
//! 版本戳来源：
//! - 产品版本 [`PRODUCT_VERSION`]：与 `CHANGELOG.md` 最新版本节保持一致，
//!   发版时两处同改（`tools/hot-update/make_manifest.py` 会交叉校验并告警）；
//! - git sha / 构建号 / 构建时刻：由本 crate 的 `build.rs` 在编译期注入
//!   （见该文件注释），缺失时为 `unknown` / `0`，不阻塞启动。

use serde::Serialize;

/// 当前产品版本号（语义版本，与 CHANGELOG.md 最新版本节一致；现行 0.0.2）。
pub const PRODUCT_VERSION: &str = "0.0.2";

/// 编译期嵌入的 git commit（完整 40 位；无 `.git` 构建时为 `unknown`）。
#[must_use]
pub fn git_sha() -> &'static str {
    option_env!("NEXTRADE_GIT_SHA").unwrap_or("unknown")
}

/// git commit 短哈希（前 12 位；`unknown` 时原样返回）。
#[must_use]
pub fn git_sha_short() -> String {
    let sha = git_sha();
    if sha.len() > 12 {
        sha[..12].to_string()
    } else {
        sha.to_string()
    }
}

/// CI 构建号（`GITHUB_RUN_NUMBER`；本地构建为 `unknown`）。
#[must_use]
pub fn build_number() -> &'static str {
    option_env!("NEXTRADE_BUILD_NUMBER").unwrap_or("unknown")
}

/// 构建时刻（Unix 秒；未知为 0）。
#[must_use]
pub fn build_time_unix() -> i64 {
    option_env!("NEXTRADE_BUILD_TIME")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

/// 后端版本自报（`/version` 端点与系统更新状态接口共用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendVersion {
    /// 固定为 `backend`。
    pub surface: &'static str,
    /// 产品版本号（四面统一口径）。
    pub product_version: &'static str,
    /// 源码 commit（完整）。
    pub git_sha: String,
    /// 源码 commit（短）。
    pub git_sha_short: String,
    /// CI 构建号。
    pub build_number: &'static str,
    /// 构建时刻（Unix 秒）。
    pub build_time: i64,
}

/// 当前运行二进制的版本戳。
#[must_use]
pub fn backend_version() -> BackendVersion {
    BackendVersion {
        surface: "backend",
        product_version: PRODUCT_VERSION,
        git_sha: git_sha().to_string(),
        git_sha_short: git_sha_short(),
        build_number: build_number(),
        build_time: build_time_unix(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_version_is_semver_like() {
        let parts: Vec<&str> = PRODUCT_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "产品版本应为 x.y.z: {PRODUCT_VERSION}");
        assert!(parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())));
    }

    #[test]
    fn backend_version_serializes_camel_case() {
        let v = backend_version();
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["surface"], "backend");
        assert_eq!(json["productVersion"], PRODUCT_VERSION);
        assert!(json["gitSha"].is_string());
        assert!(json["buildTime"].is_number());
    }

    #[test]
    fn short_sha_never_panics() {
        let s = git_sha_short();
        assert!(!s.is_empty());
        assert!(s.len() <= 12);
    }
}
