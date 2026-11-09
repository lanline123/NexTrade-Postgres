//! 前台用户询价服务（member::user_price_inquiry）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.user_price_inquiry`。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_price_inquiry::{
    PriceInquiryDetailVO, PriceInquiryListQuery, PriceInquiryReplyDTO, PriceInquiryVO, INQUIRY_NOT_EXIST,
    REMARK_REQUIRED, SERVICE_DATA_ERROR,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_price_inquiry::UserPriceInquiryRepository;
use nextrade_infrastructure::db::Repositories;

pub struct UserPriceInquiryService {
    repos: Arc<Repositories>,
}

impl UserPriceInquiryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &UserPriceInquiryRepository {
        &self.repos.member.user_price_inquiry
    }

    /// `GET /api/user/priceInquiry/list`。
    pub async fn list(
        &self,
        user_id: i32,
        q: &PriceInquiryListQuery,
    ) -> Result<PageResult<PriceInquiryVO>> {
        let (total, list) = self.repo().paginate(user_id, q).await?;
        let (page, size) = q.page_params();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// `GET /api/user/priceInquiry/detail`。
    pub async fn detail(
        &self,
        id: Option<i32>,
        user_id: i32,
    ) -> Result<PriceInquiryDetailVO> {
        let id = id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, INQUIRY_NOT_EXIST))?;
        self.repo()
            .detail(id, user_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, INQUIRY_NOT_EXIST))
    }

    /// `POST /api/user/priceInquiry/reply`：用户回复商家。
    pub async fn reply(&self, user_id: i32, dto: &PriceInquiryReplyDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, INQUIRY_NOT_EXIST))?;
        let remark = dto
            .remark
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, REMARK_REQUIRED))?;
        if !self
            .repo()
            .reply(id, user_id, remark, i32::try_from(now_seconds()).unwrap_or(0))
            .await?
        {
            return Err(Error::biz(SERVICE_DATA_ERROR, INQUIRY_NOT_EXIST));
        }
        Ok(())
    }

    /// `POST /api/user/priceInquiry/del`：仅用户侧隐藏。
    pub async fn del(&self, user_id: i32, id: Option<i32>) -> Result<()> {
        let id = id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, INQUIRY_NOT_EXIST))?;
        if !self.repo().soft_delete(id, user_id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, INQUIRY_NOT_EXIST));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remark_validation() {
        assert_eq!(REMARK_REQUIRED, "回复内容不能为空");
        assert!(INQUIRY_NOT_EXIST.contains("询价"));
    }
}
