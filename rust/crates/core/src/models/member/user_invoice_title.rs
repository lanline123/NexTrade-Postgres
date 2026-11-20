//! 用户常用发票抬头客户端域（member::user_invoice_title）。
//!
//! Tigshop v5.8.65「常用发票抬头管理」跟进移植（原版有，我方自研实现）。
//! 端点挂 `/api/user/invoiceTitle`（list / create / update / del / setDefault），需登录。
//! 表 `user_invoice_title`（迁移 `006_invoice_titles.sql`）。
//!
//! 与既有 `user_invoice`（单条专票资质、带审核流）并行存在：抬头簿是用户
//! 自助信息簿（多条 + 默认 + 普票/专票通用 + 申请时自动填充），不走审核、
//! 不合并、不迁移资质数据。
//!
//! 规则：
//! - 同用户未删除抬头上限 [`MAX_TITLES_PER_USER`] 条。
//! - 首条自动设默认；默认在未删除范围内唯一（DB 部分唯一索引兜底 +
//!   服务层事务内先清后置）。
//! - 删除默认抬头后，自动递补「最近更新的一条」未删除抬头为默认
//!   （无剩余抬头则无默认）。
//! - 企业抬头（title_type=2）税号必填且须通过 [`validate_tax_no`]；
//!   个人抬头（title_type=1）不要求税号（填了也校验格式）。

use serde::{Deserialize, Serialize};

pub const SERVICE_DATA_ERROR: i32 = 1001;
/// 同用户抬头数量上限。
pub const MAX_TITLES_PER_USER: i32 = 10;
/// 抬头类型：个人。
pub const TITLE_TYPE_PERSON: i32 = 1;
/// 抬头类型：企业。
pub const TITLE_TYPE_COMPANY: i32 = 2;

pub const TITLE_NAME_REQUIRED: &str = "抬头名称不能为空";
pub const TITLE_TYPE_INVALID: &str = "抬头类型不正确";
pub const TAX_NO_REQUIRED: &str = "企业抬头必须填写纳税人识别号";
pub const TAX_NO_INVALID: &str = "纳税人识别号格式不正确";
pub const TITLE_NOT_EXIST: &str = "发票抬头不存在";
pub const TITLE_ID_REQUIRED: &str = "抬头ID不能为空";
pub const TITLE_LIMIT_EXCEEDED: &str = "发票抬头最多保存10条";

/// 校验纳税人识别号格式：统一社会信用代码 18 位（大写字母+数字，
/// 首位为登记管理部门代码 1/5/9 等数字或字母），兼容 15 位老税号
/// （15 位纯数字或字母数字组合）。入参应已 trim 并转大写。
#[must_use]
pub fn validate_tax_no(raw: &str) -> bool {
    let code = raw.trim().to_ascii_uppercase();
    let chars: Vec<char> = code.chars().collect();
    let all_alnum = chars.iter().all(|c| c.is_ascii_alphanumeric());
    if !all_alnum {
        return false;
    }
    match chars.len() {
        // 统一社会信用代码：18 位，含校验位（末位可为数字或字母）
        18 => chars.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
        // 15 位老税号
        15 => true,
        _ => false,
    }
}

/// 抬头保存入参（create/update 共用；update 时 id 必填）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserInvoiceTitleDTO {
    pub id: Option<i32>,
    pub title_type: Option<i32>,
    pub title_name: Option<String>,
    pub tax_no: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub is_default: Option<i32>,
}

/// 抬头出参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserInvoiceTitleVO {
    pub id: i32,
    pub user_id: i32,
    pub title_type: i32,
    pub title_name: String,
    pub tax_no: String,
    pub company_address: String,
    pub company_phone: String,
    pub bank_name: String,
    pub bank_account: String,
    pub is_default: i32,
    pub add_time: i32,
    pub update_time: i32,
}

/// 保存校验（create/update 共用）。返回规范化后的 (title_type, tax_no 大写)。
pub fn validate_title(dto: &UserInvoiceTitleDTO) -> crate::Result<(i32, String)> {
    let title_type = dto.title_type.unwrap_or(TITLE_TYPE_PERSON);
    if title_type != TITLE_TYPE_PERSON && title_type != TITLE_TYPE_COMPANY {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, TITLE_TYPE_INVALID));
    }
    let name = dto.title_name.as_deref().map(str::trim).unwrap_or("");
    if name.is_empty() {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, TITLE_NAME_REQUIRED));
    }
    let tax_no = dto
        .tax_no
        .as_deref()
        .map(|v| v.trim().to_ascii_uppercase())
        .unwrap_or_default();
    if title_type == TITLE_TYPE_COMPANY && tax_no.is_empty() {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, TAX_NO_REQUIRED));
    }
    if !tax_no.is_empty() && !validate_tax_no(&tax_no) {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, TAX_NO_INVALID));
    }
    Ok((title_type, tax_no))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tax_no_accepts_uscc_18_and_legacy_15() {
        // 统一社会信用代码 18 位（示例为格式构造值，仅验格式）
        assert!(validate_tax_no("91310115MA1K3XXXXX"));
        assert!(validate_tax_no("9111000071093123XY"));
        // 15 位老税号
        assert!(validate_tax_no("310115123456789"));
        assert!(validate_tax_no("31011512345678X"));
    }

    #[test]
    fn tax_no_rejects_bad_shapes() {
        assert!(!validate_tax_no(""));
        assert!(!validate_tax_no("12345"));
        assert!(!validate_tax_no("91310115MA1K3XXXX")); // 17 位
        assert!(!validate_tax_no("91310115MA1K3XXXXXX")); // 19 位
        assert!(!validate_tax_no("91310115MA1K3XXX-X")); // 含非法字符
        assert!(!validate_tax_no("纳税人识别号一二三四五六")); // 中文
    }

    #[test]
    fn validate_title_company_requires_valid_tax_no() {
        let mut dto = UserInvoiceTitleDTO {
            title_type: Some(2),
            title_name: Some("某某公司".into()),
            ..Default::default()
        };
        assert!(validate_title(&dto)
            .unwrap_err()
            .to_string()
            .contains(TAX_NO_REQUIRED));
        dto.tax_no = Some("bad".into());
        assert!(validate_title(&dto)
            .unwrap_err()
            .to_string()
            .contains(TAX_NO_INVALID));
        // 小写输入规范化为大写后通过
        dto.tax_no = Some("91310115ma1k3xxxxx".into());
        let (ty, tax) = validate_title(&dto).unwrap();
        assert_eq!(ty, 2);
        assert_eq!(tax, "91310115MA1K3XXXXX");
    }

    #[test]
    fn validate_title_person_needs_no_tax_no() {
        let dto = UserInvoiceTitleDTO {
            title_type: Some(1),
            title_name: Some("张三".into()),
            ..Default::default()
        };
        let (ty, tax) = validate_title(&dto).unwrap();
        assert_eq!(ty, 1);
        assert_eq!(tax, "");
        // 名称为空先报名称
        let dto = UserInvoiceTitleDTO::default();
        assert!(validate_title(&dto)
            .unwrap_err()
            .to_string()
            .contains(TITLE_NAME_REQUIRED));
        // 非法类型
        let dto = UserInvoiceTitleDTO {
            title_type: Some(3),
            title_name: Some("x".into()),
            ..Default::default()
        };
        assert!(validate_title(&dto)
            .unwrap_err()
            .to_string()
            .contains(TITLE_TYPE_INVALID));
    }

    #[test]
    fn dto_serde_camel_case() {
        let dto: UserInvoiceTitleDTO =
            serde_json::from_str(r#"{"id":3,"titleType":2,"titleName":"A","taxNo":"X","isDefault":1}"#)
                .unwrap();
        assert_eq!(dto.id, Some(3));
        assert_eq!(dto.title_type, Some(2));
        assert_eq!(dto.is_default, Some(1));
    }
}
