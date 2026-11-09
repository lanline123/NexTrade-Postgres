//! 用户增票资质客户端域（member::user_invoice_client）。
//! Java `UserInvoiceClientController`（`/api/user/invoice/getStatus|detail|create|update`，需登录）。
//! 表 `user_invoice`；status：0 待审核 / 1 审核通过 / 2 审核未通过。

use serde::{Deserialize, Serialize};

pub const SERVICE_DATA_ERROR: i32 = 1001;
pub const COMPANY_NAME_REQUIRED: &str = "企业名称不能为空";
pub const COMPANY_CODE_REQUIRED: &str = "纳税人识别号不能为空";
pub const INVOICE_ID_REQUIRED: &str = "发票资质ID不能为空";
pub const INVOICE_NOT_EXIST: &str = "增票资质不存在";

#[must_use]
pub fn status_text(status: i32) -> &'static str {
    match status {
        0 => "待审核",
        1 => "审核通过",
        2 => "审核未通过",
        _ => "",
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserInvoiceDTO {
    pub invoice_id: Option<i32>,
    pub title_type: Option<i32>,
    pub company_name: Option<String>,
    pub company_code: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_bank: Option<String>,
    pub company_account: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub invoice_content: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserInvoiceVO {
    pub invoice_id: i32,
    pub user_id: i32,
    pub status: i32,
    pub status_text: String,
    pub apply_reply: String,
    pub title_type: i32,
    pub company_name: String,
    pub company_code: String,
    pub company_address: String,
    pub company_phone: String,
    pub company_bank: String,
    pub company_account: String,
    pub mobile: String,
    pub email: String,
    pub invoice_content: String,
    pub add_time: i32,
}

/// 本地校验（create/update 共用）。
pub fn validate(dto: &UserInvoiceDTO) -> crate::Result<()> {
    let name = dto.company_name.as_deref().map(str::trim).unwrap_or("");
    if name.is_empty() {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, COMPANY_NAME_REQUIRED));
    }
    let code = dto.company_code.as_deref().map(str::trim).unwrap_or("");
    if code.is_empty() {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, COMPANY_CODE_REQUIRED));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_texts() {
        assert_eq!(status_text(0), "待审核");
        assert_eq!(status_text(1), "审核通过");
        assert_eq!(status_text(2), "审核未通过");
    }

    #[test]
    fn validation_order_and_messages() {
        let dto = UserInvoiceDTO::default();
        let err = validate(&dto).unwrap_err();
        assert!(err.to_string().contains("企业名称不能为空"));
        let dto = UserInvoiceDTO {
            company_name: Some("A".into()),
            ..Default::default()
        };
        let err = validate(&dto).unwrap_err();
        assert!(err.to_string().contains("纳税人识别号不能为空"));
        let ok = UserInvoiceDTO {
            company_name: Some("A".into()),
            company_code: Some("91340000".into()),
            ..Default::default()
        };
        assert!(validate(&ok).is_ok());
    }

    #[test]
    fn dto_serde() {
        let dto: UserInvoiceDTO =
            serde_json::from_str(r#"{"invoiceId":3,"companyName":"A","titleType":1}"#).unwrap();
        assert_eq!(dto.invoice_id, Some(3));
        assert_eq!(dto.title_type, Some(1));
    }
}
