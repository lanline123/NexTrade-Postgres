//! 保证金收款账户 域服务（merchant::deposit_payee_account）—— wave9。

use std::sync::Arc;

use nextrade_core::models::merchant::deposit_payee_account::{
    account_type_text, DepositPayeeAccountDTO, DepositPayeeAccountListQuery, DepositPayeeAccountVO,
    MSG_ACCOUNT_NOT_FOUND, MSG_ID_REQUIRED, MSG_NAME_REQUIRED, MSG_NO_REQUIRED, MSG_TYPE_REQUIRED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::deposit_payee_account::{
    DepositPayeeAccountRepository, DepositPayeeAccountRow,
};
use nextrade_infrastructure::db::Repositories;
use nextrade_common::utils;

/// 账户 域服务。
pub struct DepositPayeeAccountService {
    repos: Arc<Repositories>,
}

impl DepositPayeeAccountService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &DepositPayeeAccountRepository {
        &self.repos.merchant.deposit_payee_account
    }

    /// 分页。
    pub async fn list(
        &self,
        q: &DepositPayeeAccountListQuery,
    ) -> Result<PageResult<DepositPayeeAccountVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(q).await?;
        Ok(PageResult::new(total, rows.into_iter().map(to_vo).collect(), &page))
    }

    /// 创建。
    pub async fn create(&self, dto: &DepositPayeeAccountDTO) -> Result<()> {
        validate(dto)?;
        self.repo().insert(dto, now_millis()).await?;
        Ok(())
    }

    /// 更新。
    pub async fn update(&self, dto: &DepositPayeeAccountDTO) -> Result<()> {
        let id = dto.id.filter(|v| *v > 0).ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        validate(dto)?;
        let n = self.repo().update(dto, now_millis()).await?;
        if n == 0 {
            return Err(Error::NotFound(MSG_ACCOUNT_NOT_FOUND.into()));
        }
        Ok(())
    }

    /// 删除。
    pub async fn delete(&self, id: i32) -> Result<()> {
        if id <= 0 {
            return Err(Error::Validation(MSG_ID_REQUIRED.into()));
        }
        let n = self.repo().delete(id).await?;
        if n == 0 {
            return Err(Error::NotFound(MSG_ACCOUNT_NOT_FOUND.into()));
        }
        Ok(())
    }
}

fn validate(dto: &DepositPayeeAccountDTO) -> Result<()> {
    if dto.account_type.unwrap_or(0) <= 0 {
        return Err(Error::Validation(MSG_TYPE_REQUIRED.into()));
    }
    if dto.account_name.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_none() {
        return Err(Error::Validation(MSG_NAME_REQUIRED.into()));
    }
    if dto.account_no.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_none() {
        return Err(Error::Validation(MSG_NO_REQUIRED.into()));
    }
    Ok(())
}

fn now_millis() -> i64 {
    utils::now_millis()
}

/// 行 → VO。
#[must_use]
pub fn to_vo(r: DepositPayeeAccountRow) -> DepositPayeeAccountVO {
    DepositPayeeAccountVO {
        id: r.shop_deposit_payee_account_id,
        account_type: r.account_type,
        account_type_text: account_type_text(r.account_type),
        account_name: r.account_name,
        account_no: r.account_no,
        bank_name: r.bank_name,
        bank_branch: r.bank_branch,
        status: r.status,
        sort_order: r.sort_order,
        add_time: r.add_time,
        update_time: r.update_time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto() -> DepositPayeeAccountDTO {
        DepositPayeeAccountDTO {
            id: None,
            account_type: Some(1),
            account_name: Some("张三".into()),
            account_no: Some("6222".into()),
            bank_name: None,
            bank_branch: None,
            status: None,
            sort_order: None,
        }
    }

    #[test]
    fn validate_requires_type_name_and_no() {
        assert!(validate(&dto()).is_ok());
        let mut d = dto();
        d.account_type = Some(0);
        assert_eq!(
            validate(&d).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_TYPE_REQUIRED}")
        );
        let mut d = dto();
        d.account_name = Some(" ".into());
        assert_eq!(
            validate(&d).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_NAME_REQUIRED}")
        );
        let mut d = dto();
        d.account_no = None;
        assert_eq!(
            validate(&d).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_NO_REQUIRED}")
        );
    }

    #[test]
    fn vo_derives_type_text() {
        let vo = to_vo(DepositPayeeAccountRow {
            shop_deposit_payee_account_id: 3,
            account_type: 2,
            account_name: "张三".into(),
            account_no: "a".into(),
            bank_name: String::new(),
            bank_branch: String::new(),
            status: 1,
            sort_order: 0,
            add_time: 1_733_000_000,
            update_time: 0,
        });
        assert_eq!(vo.account_type_text, "支付宝");
        assert_eq!(vo.id, 3);
    }

    #[test]
    fn messages_are_verbatim() {
        assert_eq!(MSG_ID_REQUIRED, "账户ID不能为空");
        assert_eq!(MSG_ACCOUNT_NOT_FOUND, "收款账户不存在");
        assert_eq!(MSG_TYPE_REQUIRED, "账户类型不能为空");
        assert_eq!(MSG_NAME_REQUIRED, "账户名称不能为空");
        assert_eq!(MSG_NO_REQUIRED, "账户号不能为空");
    }
}
