//! 构建脚本：把 Java 源系统导出的 API 契约嵌入二进制。
//!
//! 契约文件由 `tools/java_api_parser.py --contract` 生成。若文件缺失，
//! 构建仍然成功（服务照常运行），但 `/openapi/contract` 会返回空清单，
//! 并在编译期打印警告——避免把「没有契约」误当成「接口都已实现」。

use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    // crates/api/openapi -> crates/api -> crates -> rust
    let root = Path::new(&manifest)
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| Path::new(&manifest).to_path_buf());
    let contract = root.join("docs").join("api-contract.json");

    println!("cargo:rerun-if-changed={}", contract.display());
    println!("cargo:rustc-check-cfg=cfg(has_api_contract)");

    if contract.is_file() {
        println!("cargo:rustc-cfg=has_api_contract");
        println!(
            "cargo:warning=NexTrade API 契约已嵌入: {}",
            contract.display()
        );
    } else {
        println!(
            "cargo:warning=未找到 API 契约 {}；请先运行 `python tools/java_api_parser.py --all`",
            contract.display()
        );
    }
}