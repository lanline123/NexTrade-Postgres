//! 文件下载响应辅助（zip 等二进制）。
use axum::http::header::{self, HeaderValue};
use axum::response::{IntoResponse, Response};

/// 构造 zip 下载响应（`Content-Disposition: attachment`）。
#[must_use]
pub fn zip_response(bytes: Vec<u8>, filename: &str) -> Response {
    let mut resp = bytes.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    let safe = if filename.is_ascii() {
        filename.to_string()
    } else {
        "download.zip".to_string()
    };
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{safe}\"")) {
        resp.headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn zip_response_sets_headers() {
        let resp = zip_response(vec![1, 2, 3], "a.zip");
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/zip"
        );
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], &[1, 2, 3]);
    }
}
