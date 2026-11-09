//! 行为验证码 API（member::captcha_api）DTO / 常量。
//!
//! Java `com.nextrade.api.controller.user.CaptchaApiController`
//! （`/api/common/verification/captcha|get|check`，匿名白名单）。
//!
//! 实现见 `nextrade_services::captcha`（AJ-Captcha blockPuzzle 协议兼容）。
use serde::{Deserialize, Serialize};

/// 验证码缺失。
pub const VERIFY_TOKEN_REQUIRED: &str = "验证码不能为空";
/// 验证码校验失败。
pub const CAPTCHA_CHECK_FAILED: &str = "验证码校验失败，请刷新重试";

/// 验证码请求（AJ-Captcha `CaptchaVO` 的常用字段子集）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CaptchaDTO {
    /// 登录二次校验串（AES 加密的 `token---pointJson`）。
    pub captcha_verification: Option<String>,
    /// 兼容旧字段。
    pub verify_token: Option<String>,
    /// 验证码 token（`/check` 使用）。
    pub token: Option<String>,
    /// 坐标密文（AES-ECB，key=`secretKey`）。
    pub point_json: Option<String>,
    /// 验证码类型，固定 `blockPuzzle`。
    pub captcha_type: Option<String>,
    /// 浏览器指纹（Java 透传，本实现忽略）。
    pub browser_info: Option<String>,
}

/// 验证码返回（对应 AJ-Captcha `CaptchaVO` 的 repData）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CaptchaVO {
    pub rep_code: i32,
    pub rep_msg: String,
    /// 背景图 base64（PNG，不含 data: 前缀）。
    pub original_image_base64: Option<String>,
    /// 拼图块 base64（PNG，47x155 透明条）。
    pub jigsaw_image_base64: Option<String>,
    /// 验证码 token。
    pub token: Option<String>,
    /// AES 密钥（16 字符）。
    pub secret_key: Option<String>,
    /// 验证码类型。
    pub captcha_type: Option<String>,
}

/// 本地校验：`/check` 需要 token + pointJson。
#[must_use]
pub fn validate_check_params(dto: &CaptchaDTO) -> bool {
    dto.token.as_deref().is_some_and(|v| !v.trim().is_empty())
        && dto
            .point_json
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty())
}

/// 校验验证串必填（旧接口兼容）。
#[must_use]
pub fn validate_verify_token(dto: &CaptchaDTO) -> bool {
    dto.verify_token
        .as_deref()
        .map(str::trim)
        .is_some_and(|v| !v.is_empty())
        || dto
            .captcha_verification
            .as_deref()
            .map(str::trim)
            .is_some_and(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_params_validation() {
        let dto = CaptchaDTO::default();
        assert!(!validate_check_params(&dto));
        let dto = CaptchaDTO {
            token: Some("t".into()),
            point_json: Some("p".into()),
            ..Default::default()
        };
        assert!(validate_check_params(&dto));
        assert_eq!(VERIFY_TOKEN_REQUIRED, "验证码不能为空");
        assert_eq!(CAPTCHA_CHECK_FAILED, "验证码校验失败，请刷新重试");
    }

    #[test]
    fn dto_serde_shapes() {
        let dto: CaptchaDTO = serde_json::from_str(
            r#"{"captchaVerification":"x","verifyToken":"y","token":"t","pointJson":"p","captchaType":"blockPuzzle"}"#,
        )
        .unwrap();
        assert_eq!(dto.captcha_verification.as_deref(), Some("x"));
        assert_eq!(dto.verify_token.as_deref(), Some("y"));
        assert_eq!(dto.token.as_deref(), Some("t"));
        assert_eq!(dto.point_json.as_deref(), Some("p"));
    }
}
