//! 用户增票资质服务（member::user_invoice_client）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.user_invoice_client`。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_invoice_client::{
    status_text, validate, UserInvoiceDTO, UserInvoiceVO, INVOICE_NOT_EXIST,
    SERVICE_DATA_ERROR,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_invoice_client::{
    InvoiceRow, UserInvoiceClientRepository,
};
use nextrade_infrastructure::db::Repositories;

pub struct UserInvoiceClientService {
    repos: Arc<Repositories>,
}

impl UserInvoiceClientService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &UserInvoiceClientRepository {
        &self.repos.member.user_invoice_client
    }

    /// `GET /api/user/invoice/getStatus`：最新资质状态（无记录 → null）。
    pub async fn get_status(&self, user_id: i32) -> Result<Option<i32>> {
        Ok(self.repo().latest(user_id).await?.map(|r| r.2))
    }

    /// `GET /api/user/invoice/detail`：最新资质详情。
    pub async fn detail(&self, user_id: i32) -> Result<Option<UserInvoiceVO>> {
        Ok(self.repo().latest(user_id).await?.map(to_vo))
    }

    /// `POST /api/user/invoice/create`：本地校验后新建。
    pub async fn create(&self, user_id: i32, dto: &UserInvoiceDTO) -> Result<i32> {
        validate(dto)?;
        self.repo()
            .insert(
                user_id,
                dto.title_type.unwrap_or(1),
                trim(dto.company_name.as_deref()),
                trim(dto.company_code.as_deref()),
                trim(dto.company_address.as_deref()),
                trim(dto.company_phone.as_deref()),
                trim(dto.company_bank.as_deref()),
                trim(dto.company_account.as_deref()),
                trim(dto.mobile.as_deref()),
                trim(dto.email.as_deref()),
                trim(dto.invoice_content.as_deref()),
                i32::try_from(now_seconds()).unwrap_or(0),
            )
            .await
    }

    /// `POST /api/user/invoice/update`：仅本人资质，重置待审核。
    ///
    /// PC 首次进入“增票资质”页时前端可能提交 `update` 但没有 `invoiceId`；
    /// 这种情况下按新增处理，避免用户第一次填写就被“资质ID不能为空”拦截。
    pub async fn update(&self, user_id: i32, dto: &UserInvoiceDTO) -> Result<()> {
        let Some(invoice_id) = dto.invoice_id else {
            self.create(user_id, dto).await?;
            return Ok(());
        };
        validate(dto)?;
        if self.repo().find_own(invoice_id, user_id).await?.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, INVOICE_NOT_EXIST));
        }
        self.repo()
            .update(
                invoice_id,
                user_id,
                dto.title_type.unwrap_or(1),
                trim(dto.company_name.as_deref()),
                trim(dto.company_code.as_deref()),
                trim(dto.company_address.as_deref()),
                trim(dto.company_phone.as_deref()),
                trim(dto.company_bank.as_deref()),
                trim(dto.company_account.as_deref()),
                trim(dto.mobile.as_deref()),
                trim(dto.email.as_deref()),
                trim(dto.invoice_content.as_deref()),
            )
            .await?;
        Ok(())
    }
}

fn trim(v: Option<&str>) -> &str {
    v.map(str::trim).unwrap_or("")
}

fn to_vo(row: InvoiceRow) -> UserInvoiceVO {
    let (invoice_id, user_id, status, apply_reply, title_type, company_name, company_code, company_address, company_phone, company_bank, company_account, mobile, email, invoice_content, add_time) = row;
    UserInvoiceVO {
        invoice_id,
        user_id,
        status,
        status_text: status_text(status).to_string(),
        apply_reply,
        title_type,
        company_name,
        company_code,
        company_address,
        company_phone,
        company_bank,
        company_account,
        mobile,
        email,
        invoice_content,
        add_time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_handles_none() {
        assert_eq!(trim(None), "");
        assert_eq!(trim(Some(" x ")), "x");
    }

    #[test]
    fn vo_status_text() {
        let vo = to_vo((
            1, 2, 1, "ok".into(), 1, "A".into(), "C".into(), "AD".into(), "P".into(), "B".into(),
            "AC".into(), "M".into(), "E".into(), "内容".into(), 0,
        ));
        assert_eq!(vo.status_text, "审核通过");
    }
}
