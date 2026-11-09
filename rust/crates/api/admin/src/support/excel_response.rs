//! Excel 下载响应辅助。
use axum::http::header::{self, HeaderValue};
use axum::response::{IntoResponse, Response};

/// 构造 xlsx 下载响应（`Content-Disposition: attachment`）。
#[must_use]
pub fn xlsx_response(bytes: Vec<u8>, filename: &str) -> Response {
    let mut resp = bytes.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    );
    let disposition = format!("attachment; filename=\"{}\"", safe_filename(filename));
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        resp.headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    resp
}

/// 文件名兜底：非 ASCII 文件名改用 `export_<毫秒>.xlsx`（避免 header 编码问题）。
#[must_use]
pub fn safe_filename(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "export.xlsx".to_string()
    } else if trimmed.is_ascii() {
        trimmed.to_string()
    } else {
        format!("export_{}.xlsx", nextrade_common::utils::now_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_filename_ascii_kept() {
        assert_eq!(safe_filename("order.xlsx"), "order.xlsx");
        assert_eq!(safe_filename(""), "export.xlsx");
        assert!(safe_filename("中文.xlsx").starts_with("export_"));
    }

    #[test]
    fn response_has_content_type_and_disposition() {
        let resp = xlsx_response(vec![1, 2, 3], "t.xlsx");
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(
            resp.headers().get(header::CONTENT_DISPOSITION).unwrap(),
            "attachment; filename=\"t.xlsx\""
        );
    }
}
