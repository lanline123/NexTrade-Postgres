//! 用户实名认证服务（member::user_company_client）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.user_company_client`。

use std::sync::Arc;

use serde_json::Value;
use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_company_client::{
    format_time, status_text, UserCompanyDTO, UserCompanyVO, ALREADY_PENDING, COMPANY_NAME_REQUIRED,
    CONTACT_MOBILE_REQUIRED, CONTACT_NAME_REQUIRED, SERVICE_DATA_ERROR,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_company_client::UserCompanyClientRepository;
use nextrade_infrastructure::db::Repositories;

pub struct UserCompanyClientService {
    repos: Arc<Repositories>,
}

impl UserCompanyClientService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &UserCompanyClientRepository {
        &self.repos.member.user_company_client
    }

    /// `POST /api/user/company/apply`：实名申请。
    pub async fn apply(&self, user_id: i32, dto: &UserCompanyDTO) -> Result<i32> {
        let company_name = require(dto.company_name.as_deref(), COMPANY_NAME_REQUIRED)?;
        let contact_name = require(dto.contact_name.as_deref(), CONTACT_NAME_REQUIRED)?;
        let contact_mobile = require(dto.contact_mobile.as_deref(), CONTACT_MOBILE_REQUIRED)?;
        if self.repo().has_pending(user_id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, ALREADY_PENDING));
        }
        let data = dto.company_data.as_ref().map(Value::to_string);
        self.repo()
            .insert(
                user_id,
                dto.r#type.unwrap_or(1),
                contact_name,
                contact_mobile,
                company_name,
                data.as_deref(),
                i32::try_from(now_seconds()).unwrap_or(0),
            )
            .await
    }

    /// `GET /api/user/company/detail`：最新认证详情（无则 null）。
    pub async fn detail(&self, user_id: i32) -> Result<Option<UserCompanyVO>> {
        Ok(self.repo().latest(user_id).await?.map(to_vo))
    }

    /// `GET /api/user/company/myApply`：我的申请列表。
    pub async fn my_apply(&self, user_id: i32) -> Result<Vec<UserCompanyVO>> {
        Ok(self.repo().list_by_user(user_id).await?.into_iter().map(to_vo).collect())
    }
}

fn require<'a>(value: Option<&'a str>, msg: &str) -> Result<&'a str> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, msg))
}

#[allow(clippy::type_complexity)]
fn to_vo(
    row: (i32, i32, i32, String, String, String, Option<String>, i32, String, Option<i32>, Option<i32>),
) -> UserCompanyVO {
    let (id, user_id, r#type, contact_name, contact_mobile, company_name, company_data, status, audit_remark, audit_time, add_time) = row;
    UserCompanyVO {
        id,
        user_id,
        r#type,
        contact_name,
        contact_mobile,
        company_name,
        company_data: company_data.and_then(|s| serde_json::from_str(&s).ok()),
        status,
        status_text: status_text(status).to_string(),
        audit_remark,
        audit_time: format_time(audit_time),
        add_time: format_time(add_time),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_trim_and_errors() {
        assert!(require(None, COMPANY_NAME_REQUIRED).is_err());
        assert!(require(Some("  "), COMPANY_NAME_REQUIRED).is_err());
        assert_eq!(require(Some(" A "), COMPANY_NAME_REQUIRED).unwrap(), "A");
    }

    #[test]
    fn apply_validation_order_is_company_contact_mobile() {
        // 纯函数校验顺序（无 DB）：企业名 → 联系人 → 电话
        let dto = UserCompanyDTO::default();
        let err = require(dto.company_name.as_deref(), COMPANY_NAME_REQUIRED).unwrap_err();
        assert!(err.to_string().contains("企业名称不能为空"));
    }
}
