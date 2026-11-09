//! 行为验证码服务（member::captcha_api）：委托 `nextrade_services::captcha` 实现。
use std::sync::Arc;

use nextrade_core::models::member::captcha_api::{
    validate_check_params, CaptchaDTO, CaptchaVO, VERIFY_TOKEN_REQUIRED,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 验证码服务（无仓储访问；`repos` 仅为统一装配签名保留）。
pub struct CaptchaService {
    #[allow(dead_code)]
    repos: Arc<Repositories>,
}

impl CaptchaService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `POST/GET /api/common/verification/captcha`：生成滑块验证码。
    pub fn captcha(&self) -> Result<CaptchaVO> {
        crate::system::captcha::new_captcha()
    }

    /// `POST /api/common/verification/get`（废弃端点，等价 captcha）。
    pub fn get(&self) -> Result<CaptchaVO> {
        crate::system::captcha::new_captcha()
    }

    /// `POST /api/common/verification/check`：校验滑块坐标。
    pub fn check(&self, dto: &CaptchaDTO) -> Result<bool> {
        if !validate_check_params(dto) {
            return Err(Error::biz(1001, VERIFY_TOKEN_REQUIRED));
        }
        let token = dto.token.as_deref().unwrap_or_default();
        let point_json = dto.point_json.as_deref().unwrap_or_default();
        crate::system::captcha::check_captcha(token, point_json)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn captcha_generation_is_protocol_compatible() {
        let vo = crate::system::captcha::new_captcha().expect("生成验证码");
        assert_eq!(vo.captcha_type.as_deref(), Some("blockPuzzle"));
        assert_eq!(vo.secret_key.as_ref().map(String::len), Some(16));
        assert!(vo.token.as_deref().is_some_and(|t| !t.is_empty()));
        assert!(vo
            .original_image_base64
            .as_deref()
            .is_some_and(|s| !s.is_empty()));
        assert!(vo
            .jigsaw_image_base64
            .as_deref()
            .is_some_and(|s| !s.is_empty()));
    }
}
