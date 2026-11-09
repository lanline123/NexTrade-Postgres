//! 统一错误类型。
//!
//! 映射关系：
//! | Java 原实现 | Rust 实现 |
//! |------------|----------|
//! | `ResultWrapper.fail(code, msg)` | `Error::Biz { code, msg }` |
//! | `BizException` | `Error::Biz` |
//! | `DataIntegrityViolationException` | `Error::Database` |
//! | `IllegalArgumentException` | `Error::Validation` |

use http::StatusCode;
use serde::Serialize;
use thiserror::Error;

/// 业务/基础错误类型。
#[derive(Debug, Error)]
pub enum Error {
    /// 业务错误：带业务码与提示信息，对应原 Java `ResultWrapper.fail`。
    #[error("业务错误 [{code}]: {msg}")]
    Biz { code: i32, msg: String },

    /// 参数校验失败，对应 `IllegalArgumentException` / `BindException`。
    #[error("参数校验失败: {0}")]
    Validation(String),

    /// 未登录 / Token 失效，HTTP 200 + 业务码 401（对齐 Java JwtAuthenticationEntryPoint）。
    #[error("未登录或会话已过期: {0}")]
    Unauthorized(String),

    /// 无权限，HTTP 403。
    #[error("无权限执行该操作: {0}")]
    Forbidden(String),

    /// 资源不存在，HTTP 404。
    #[error("资源不存在: {0}")]
    NotFound(String),

    /// 库存不足（电商核心场景）。
    #[error("商品 {product_id} 库存不足，剩余 {stock}")]
    InsufficientStock { product_id: String, stock: i64 },

    /// 并发冲突（乐观锁/唯一约束），HTTP 409。
    #[error("操作冲突: {0}")]
    Conflict(String),

    /// 数据库错误。
    #[error("数据库错误: {0}")]
    Database(String),

    /// 缓存（Redis）错误。
    #[error("缓存错误: {0}")]
    Cache(String),

    /// 第三方集成错误（支付、短信、OSS 等）。
    #[error("第三方服务错误[{service}]: {reason}")]
    ThirdParty { service: String, reason: String },

    /// 支付失败。
    #[error("支付失败: {reason}")]
    PaymentFailed { reason: String },

    /// 内部未预期错误。
    #[error("内部错误: {0}")]
    Internal(String),
}

impl Error {
    /// 便捷构造业务错误，等价于 Java 侧 `ResultWrapper.fail(code, msg)`。
    pub fn biz(code: i32, msg: impl Into<String>) -> Self {
        Self::Biz { code, msg: msg.into() }
    }

    /// 业务失败（通用码 500）。
    pub fn biz_msg(msg: impl Into<String>) -> Self {
        Self::Biz { code: 500, msg: msg.into() }
    }

    /// 转换为 HTTP 状态码。
    ///
    /// 未登录对齐原 Java `JwtAuthenticationEntryPoint`：HTTP 200 + 业务码 401
    /// （PC/后台前端只在 body code==401 时跳登录页，HTTP 401 会走 $fetch 错误分支）。
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::OK,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) | Self::InsufficientStock { .. } => StatusCode::CONFLICT,
            Self::Biz { code, .. } => match code {
                401 => StatusCode::OK,
                403 => StatusCode::FORBIDDEN,
                404 => StatusCode::NOT_FOUND,
                400..=499 => StatusCode::BAD_REQUEST,
                _ => StatusCode::OK, // 原 Java 风格：HTTP 200 + 业务错误码
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 业务码（用于响应体中的 code 字段）。
    pub fn business_code(&self) -> i32 {
        match self {
            Self::Biz { code, .. } => *code,
            Self::Validation(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::Forbidden(_) => 403,
            Self::NotFound(_) => 404,
            Self::Conflict(_) | Self::InsufficientStock { .. } => 409,
            Self::PaymentFailed { .. } => 600,
            Self::ThirdParty { .. } => 502,
            _ => 500,
        }
    }
}

/// 统一 API 响应体。
///
/// Java 的 `AjaxResult` 字段为 `{code,message,data}`，早期 Rust 骨架用的是
/// `{code,msg,data,timestamp}`。前端两套都读过（错误提示读 `message || msg`），
/// 因此这里同时输出 `msg` 与 `message`，保持双向兼容。
#[derive(Debug)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
    pub timestamp: i64,
}

impl<T: Serialize> Serialize for ApiResponse<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut st = serializer.serialize_struct("ApiResponse", 5)?;
        st.serialize_field("code", &self.code)?;
        st.serialize_field("msg", &self.msg)?;
        st.serialize_field("message", &self.msg)?;
        if let Some(data) = &self.data {
            st.serialize_field("data", data)?;
        }
        st.serialize_field("timestamp", &self.timestamp)?;
        st.end()
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            msg: "success".into(),
            data: Some(data),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn fail(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

impl ApiResponse<()> {
    /// 无 data 的成功响应。
    pub fn ok() -> Self {
        Self::success(())
    }
}

impl From<Error> for ApiResponse<()> {
    fn from(e: Error) -> Self {
        // 业务错误直接透传原始文案，与原 Java ResultWrapper.fail(code, msg) 完全一致；
        // 其余错误使用 Display 文本。
        match &e {
            Error::Biz { code, msg } => ApiResponse::fail(*code, msg.clone()),
            other => ApiResponse::fail(other.business_code(), other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biz_error_serializes_like_java_wrapper() {
        let resp: ApiResponse<()> = Error::biz(400, "参数错误").into();
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"code\":400"));
        assert!(json.contains("\"msg\":\"参数错误\""));
    }

    #[test]
    fn success_shape() {
        let resp = ApiResponse::success(vec![1, 2, 3]);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["code"], 0);
        assert!(json["data"].is_array());
    }
}