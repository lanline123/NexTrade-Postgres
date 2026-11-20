//! 商品交付附件与交付令牌模型（我方新增，原版无附件交付通道）。
//!
//! 商家为 type=2（虚拟商品）/ type=4（付费内容）商品挂附件；买家购买后
//! 在接口层申领短时效令牌（绑订单 + 用户 + 文件）下载。本模块只放
//! DTO/VO、口径常量与纯函数（校验 / 访问门 / 令牌状态），落库与存储
//! 在 infrastructure / services 层。

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// 可挂附件的商品类型：2 虚拟商品、4 付费内容。
pub const DELIVERY_PRODUCT_TYPES: [i32; 2] = [2, 4];

/// 附件扩展名白名单（小写，不含点）。压缩包 / 文档 / 电子书 / 音视频 /
/// 常见安装包；排除 html/js/svg/php 等可执行脚本类，防直链渲染风险。
pub const ALLOWED_DELIVERY_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "pdf", "doc", "docx", "xls", "xlsx", "ppt",
    "pptx", "txt", "md", "csv", "epub", "mp3", "wav", "mp4", "mov", "exe", "msi", "dmg", "pkg",
    "apk", "iso", "json",
];

/// 单个附件大小上限：200 MiB。
pub const MAX_DELIVERY_FILE_SIZE: i64 = 200 * 1024 * 1024;

/// 每件商品最多挂载的附件数。
pub const MAX_FILES_PER_PRODUCT: i64 = 10;

/// 交付令牌有效期（秒）：30 分钟。
pub const TOKEN_TTL_SECONDS: i32 = 30 * 60;

/// 单个令牌最多下载次数。
pub const TOKEN_MAX_DOWNLOADS: i32 = 5;

/// 存储层限时签名 URL 有效期（秒）：10 分钟（令牌核销后跳转用，
/// 比令牌本身更短，缩小签名 URL 的外泄窗口）。
pub const SIGNED_URL_TTL_SECONDS: i64 = 10 * 60;

/// 附件下发视图（买家侧与商家侧共用）。**永不包含 file_key**——
/// 存储对象 key 一旦下发即可绕过令牌直链下载。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeliveryFileVO {
    pub file_id: i32,
    pub product_id: i32,
    pub file_name: String,
    pub file_ext: String,
    pub file_size: i64,
    pub add_time: i32,
}

/// 买家申领下载令牌入参。商品由附件本身确定（`product_id` 仅作
/// 前端透传，服务端不信任）；订单页传 `order_id` 精确绑单并校验
/// 归属，商品页不传时服务端取本人最近一张符合条件的已付订单绑单。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RequestDownloadDTO {
    pub order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub file_id: i32,
}

/// 申领成功回执：前端打开 `download_url` 触发下载。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadGrantVO {
    /// 站内下载端点路径（带令牌），如 `/api/product/delivery/download?token=…`。
    pub download_url: String,
    /// 令牌过期时刻（秒级时间戳）。
    pub expires_at: i32,
    pub file_name: String,
    pub file_size: i64,
}

/// 商家删除附件入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeliveryFileDeleteDTO {
    pub id: i32,
}

/// 校验附件文件名与大小，返回小写扩展名。
///
/// # Errors
/// 文件名为空、无扩展名、扩展名不在白名单、大小非法或超限时返回校验错误。
pub fn validate_delivery_file(file_name: &str, file_size: i64) -> Result<String> {
    let name = file_name.trim();
    if name.is_empty() {
        return Err(Error::Validation("文件名不能为空".into()));
    }
    if file_size <= 0 {
        return Err(Error::Validation("文件内容为空".into()));
    }
    if file_size > MAX_DELIVERY_FILE_SIZE {
        return Err(Error::Validation(format!(
            "单个附件不能超过 {} MB",
            MAX_DELIVERY_FILE_SIZE / 1024 / 1024
        )));
    }
    let ext = name
        .rsplit('.')
        .next()
        .filter(|_| name.contains('.'))
        .map(str::to_lowercase)
        .unwrap_or_default();
    if !ALLOWED_DELIVERY_EXTENSIONS.contains(&ext.as_str()) {
        return Err(Error::Validation(format!(
            "不支持的文件类型 .{ext}，仅允许压缩包、文档、电子书、音视频与常见安装包"
        )));
    }
    Ok(ext)
}

/// 文件名展示口径：去掉路径分隔与控制字符后的基础名（原始文件名只作
/// 展示与下载命名，永不参与存储路径拼接）。
#[must_use]
pub fn display_file_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("").trim();
    let cleaned: String = base.chars().filter(|c| !c.is_control()).collect();
    if cleaned.is_empty() {
        "未命名文件".to_string()
    } else {
        cleaned.chars().take(200).collect()
    }
}

/// 交付附件可见/可下载门（与订单详情 virtualSample 掩码同构）：
/// - type=4 付费内容：已支付即可；
/// - type=2 虚拟商品：已支付且已发货（含部分发货，shipping_status != 0）才可；
/// - 其余类型恒不可。
#[must_use]
pub fn delivery_access(product_type: i32, pay_status: i32, shipping_status: i32) -> bool {
    if pay_status != 2 {
        return false;
    }
    match product_type {
        4 => true,
        2 => shipping_status != 0,
        _ => false,
    }
}

/// 令牌状态（纯函数口径；下载端点的计数核销走仓储层单条原子 UPDATE，
/// 本函数用于申领/报错文案分流与单测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// 有效期内且次数未用完。
    Usable,
    /// 已过期。
    Expired,
    /// 次数已用完。
    Exhausted,
}

#[must_use]
pub fn token_state(expires_at: i32, download_count: i32, max_downloads: i32, now: i32) -> TokenState {
    if now >= expires_at {
        return TokenState::Expired;
    }
    if download_count >= max_downloads {
        return TokenState::Exhausted;
    }
    TokenState::Usable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_whitelisted_extensions() {
        assert_eq!(validate_delivery_file("资料包.ZIP", 1024).unwrap(), "zip");
        assert_eq!(validate_delivery_file("安装程序.exe", 1024).unwrap(), "exe");
        assert_eq!(validate_delivery_file("说明书.pdf", 1024).unwrap(), "pdf");
    }

    #[test]
    fn validate_rejects_scripts_and_unknown() {
        assert!(validate_delivery_file("page.html", 1024).is_err());
        assert!(validate_delivery_file("run.sh", 1024).is_err());
        assert!(validate_delivery_file("noext", 1024).is_err());
        assert!(validate_delivery_file("", 1024).is_err());
    }

    #[test]
    fn validate_rejects_empty_and_oversize() {
        assert!(validate_delivery_file("a.zip", 0).is_err());
        assert!(validate_delivery_file("a.zip", MAX_DELIVERY_FILE_SIZE + 1).is_err());
        assert!(validate_delivery_file("a.zip", MAX_DELIVERY_FILE_SIZE).is_ok());
    }

    #[test]
    fn display_name_strips_path() {
        assert_eq!(display_file_name("a/b\\c.zip"), "c.zip");
        assert_eq!(display_file_name("  "), "未命名文件");
    }

    #[test]
    fn access_gate_matches_virtual_sample_caliber() {
        // 付费内容：已付即可，未付不可
        assert!(delivery_access(4, 2, 0));
        assert!(!delivery_access(4, 1, 0));
        // 虚拟商品：已付 + 已发货
        assert!(delivery_access(2, 2, 1));
        assert!(!delivery_access(2, 2, 0));
        // 其它类型恒不可
        assert!(!delivery_access(1, 2, 1));
        assert!(!delivery_access(3, 2, 1));
    }

    #[test]
    fn token_state_boundaries() {
        assert_eq!(token_state(1000, 0, 5, 999), TokenState::Usable);
        assert_eq!(token_state(1000, 0, 5, 1000), TokenState::Expired);
        assert_eq!(token_state(1000, 5, 5, 999), TokenState::Exhausted);
        // 过期优先于次数耗尽（文案分流）
        assert_eq!(token_state(1000, 5, 5, 1000), TokenState::Expired);
    }
}
