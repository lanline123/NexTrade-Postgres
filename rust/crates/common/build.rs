//! 构建期版本戳采集（admin 自助热更新 P1，我方新增）。
//!
//! 在编译 `nextrade-common` 时把三项信息以 `cargo:rustc-env` 注入本 crate，
//! 供 [`crate::version`] 读取：
//! - `NEXTRADE_GIT_SHA`：当前 checkout 的 commit（`git rev-parse HEAD`）；
//!   无 `.git`（如源码包构建）时为 `unknown`，不让构建失败。
//! - `NEXTRADE_BUILD_NUMBER`：CI 的 `GITHUB_RUN_NUMBER`（本地构建为空 → unknown）。
//! - `NEXTRADE_BUILD_TIME`：构建时刻的 Unix 秒。
//!
//! 注意：`rustc-env` 只作用于本 crate，故版本常量统一收口在 `common::version`，
//! 其他 crate 一律经它读取，不要在别处再写 `option_env!`。

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_RUN_NUMBER");
    // .git 位于仓库根；本地增量构建时 HEAD 变化也应刷新版本戳。
    println!("cargo:rerun-if-changed=../../../.git/HEAD");

    let sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=NEXTRADE_GIT_SHA={sha}");

    let build_number = std::env::var("GITHUB_RUN_NUMBER")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=NEXTRADE_BUILD_NUMBER={build_number}");

    let build_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=NEXTRADE_BUILD_TIME={build_time}");
}
