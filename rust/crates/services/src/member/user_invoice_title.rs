//! 用户常用发票抬头服务（member::user_invoice_title）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.user_invoice_title`。
//!
//! 规则（见 core 模型文件头）：上限 10 条、首条自动默认、默认唯一
//! （仓储事务保证）、删默认自动递补最近更新的一条、全部按 user 归属校验
//! （他人/不存在统一「发票抬头不存在」，与资质域 `INVOICE_NOT_EXIST` 口径一致）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_invoice_title::{
    validate_title, UserInvoiceTitleDTO, UserInvoiceTitleVO, MAX_TITLES_PER_USER,
    SERVICE_DATA_ERROR, TITLE_ID_REQUIRED, TITLE_LIMIT_EXCEEDED, TITLE_NOT_EXIST,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_invoice_title::{
    TitleRow, UserInvoiceTitleRepository,
};
use nextrade_infrastructure::db::Repositories;

pub struct UserInvoiceTitleService {
    repos: Arc<Repositories>,
}

impl UserInvoiceTitleService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &UserInvoiceTitleRepository {
        &self.repos.member.user_invoice_title
    }

    /// `GET /api/user/invoiceTitle/list`：本人抬头列表（默认在前）。
    pub async fn list(&self, user_id: i32) -> Result<Vec<UserInvoiceTitleVO>> {
        Ok(self.repo().list(user_id).await?.into_iter().map(to_vo).collect())
    }

    /// `POST /api/user/invoiceTitle/create`：校验 + 上限 + 首条自动默认。
    pub async fn create(&self, user_id: i32, dto: &UserInvoiceTitleDTO) -> Result<i32> {
        let (title_type, tax_no) = validate_title(dto)?;
        let repo = self.repo();
        let count = repo.count_active(user_id).await?;
        if count >= i64::from(MAX_TITLES_PER_USER) {
            return Err(Error::biz(SERVICE_DATA_ERROR, TITLE_LIMIT_EXCEEDED));
        }
        // 首条自动设默认；显式 is_default=1 也设默认
        let make_default = count == 0 || dto.is_default == Some(1);
        repo.insert(
            user_id,
            title_type,
            trim(dto.title_name.as_deref()),
            &tax_no,
            trim(dto.company_address.as_deref()),
            trim(dto.company_phone.as_deref()),
            trim(dto.bank_name.as_deref()),
            trim(dto.bank_account.as_deref()),
            make_default,
            now_i32(),
        )
        .await
    }

    /// `POST /api/user/invoiceTitle/update`：仅本人抬头。
    pub async fn update(&self, user_id: i32, dto: &UserInvoiceTitleDTO) -> Result<()> {
        let id = dto
            .id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, TITLE_ID_REQUIRED))?;
        let (title_type, tax_no) = validate_title(dto)?;
        let repo = self.repo();
        if repo.find_own(id, user_id).await?.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, TITLE_NOT_EXIST));
        }
        repo.update(
            id,
            user_id,
            title_type,
            trim(dto.title_name.as_deref()),
            &tax_no,
            trim(dto.company_address.as_deref()),
            trim(dto.company_phone.as_deref()),
            trim(dto.bank_name.as_deref()),
            trim(dto.bank_account.as_deref()),
            dto.is_default == Some(1),
            now_i32(),
        )
        .await?;
        Ok(())
    }

    /// `POST /api/user/invoiceTitle/del`：软删（删默认自动递补，仓储事务内）。
    pub async fn del(&self, user_id: i32, id: i32) -> Result<()> {
        if id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, TITLE_ID_REQUIRED));
        }
        if !self.repo().soft_delete(id, user_id, now_i32()).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, TITLE_NOT_EXIST));
        }
        Ok(())
    }

    /// `POST /api/user/invoiceTitle/setDefault`：设为默认（仅本人）。
    pub async fn set_default(&self, user_id: i32, id: i32) -> Result<()> {
        if id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, TITLE_ID_REQUIRED));
        }
        if !self.repo().set_default(id, user_id, now_i32()).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, TITLE_NOT_EXIST));
        }
        Ok(())
    }
}

fn trim(v: Option<&str>) -> &str {
    v.map(str::trim).unwrap_or("")
}

fn now_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(0)
}

fn to_vo(row: TitleRow) -> UserInvoiceTitleVO {
    let (id, user_id, title_type, title_name, tax_no, company_address, company_phone, bank_name, bank_account, is_default, add_time, update_time) = row;
    UserInvoiceTitleVO {
        id,
        user_id,
        title_type,
        title_name,
        tax_no,
        company_address,
        company_phone,
        bank_name,
        bank_account,
        is_default,
        add_time,
        update_time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vo_maps_row() {
        let vo = to_vo((
            1, 2, 2, "某某公司".into(), "91310115MA1K3XXXXX".into(), "地址".into(),
            "021-0000".into(), "招行".into(), "6222".into(), 1, 100, 200,
        ));
        assert_eq!(vo.id, 1);
        assert_eq!(vo.is_default, 1);
        assert_eq!(vo.tax_no, "91310115MA1K3XXXXX");
    }

    #[test]
    fn trim_handles_none() {
        assert_eq!(trim(None), "");
        assert_eq!(trim(Some(" x ")), "x");
    }
}
