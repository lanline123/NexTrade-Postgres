//! 积分签到配置域服务（marketing::sign_in_setting）。
//!
//! Java `SignInSettingController`（7 端点）→ `SignInSettingServiceImpl`。
//! 创建时 `day_num = 现有最大值 + 1`、`sort_order` 缺省 0、`create_time` 当前秒；
//! 更新先校验存在（「该设置不存在」）；`updateField` 白名单仅 `sort_order`。

//! # 多租户（N/A 判定）
//!
//! 本域为**平台级数据**，不做店铺隔离（N/A），证据：
//! - `java/database/pure.sql` 中 `sign_in_setting` 表**没有 `shop_id` 列**；
//! - 原 Java `SignInSettingServiceImpl` 中**没有任何** `getShopId()` / `shop_id` 引用；
//! - 因此管理端列表/详情/写操作均以平台身份执行，接入 `TenantScope`
//!   不会改变行为（平台作用域默认放行，店铺账号访问时按平台数据可见性处理）。
//!
//! 若未来产品要求店铺级隔离，需先给 `sign_in_setting` 增加 `shop_id` 列并回填归属。
//!
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::promotion::SignInSetting;
use nextrade_core::models::marketing::sign_in_setting::{
    BatchDTO, OperateDTO, SignInSettingCreateDTO, SignInSettingListQuery, SignInSettingUpdateDTO,
    SignInSettingVO, UpdateFieldDTO, MSG_NOT_EXISTS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::sign_in_setting::SignInSettingRepository;
use nextrade_infrastructure::db::Repositories;

/// 积分签到配置域服务。
pub struct SignInSettingService {
    repos: Arc<Repositories>,
}

impl SignInSettingService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &SignInSettingRepository {
        &self.repos.marketing.sign_in_setting
    }

    pub async fn list(&self, q: &SignInSettingListQuery) -> Result<PageResult<SignInSettingVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, limit, offset).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    pub async fn detail(&self, id: i32) -> Result<SignInSettingVO> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        Ok(to_vo(&entity))
    }

    pub async fn create(&self, dto: &SignInSettingCreateDTO) -> Result<i32> {
        let input = dto.validate().map_err(Error::Validation)?;
        let next_day_num = self.repo().max_day_num().await?.map(|v| v + 1).unwrap_or(1);
        let entity = SignInSetting {
            id: 0,
            name: Some(input.name),
            points: Some(input.points),
            day_num: Some(next_day_num),
            sort_order: input.sort_order.unwrap_or(0),
            create_time: i32::try_from(now_seconds()).unwrap_or_default(),
        };
        Ok(self.repo().insert(&entity).await?)
    }

    pub async fn update(&self, dto: &SignInSettingUpdateDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let mut entity = self
            .repo()
            .find_by_id(input.id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        entity.name = Some(input.name);
        entity.points = Some(input.points);
        if let Some(sort_order) = input.sort_order {
            entity.sort_order = sort_order;
        }
        self.repo().update(&entity).await?;
        Ok(())
    }

    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.validate_id().map_err(Error::Validation)?;
        let _ = self.repo().delete(id).await?;
        Ok(())
    }

    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        self.repo().update_field(input.id, input.column, &input.val).await?;
        Ok(())
    }

    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type.as_deref() == Some("del") {
            let n = self.repo().delete_batch(dto.ids.as_deref().unwrap_or_default()).await?;
            return Ok(n > 0);
        }
        Ok(false)
    }
}

fn to_vo(entity: &SignInSetting) -> SignInSettingVO {
    SignInSettingVO {
        id: entity.id,
        name: entity.name.clone(),
        points: entity.points,
        day_num: entity.day_num,
        sort_order: entity.sort_order,
        create_time: entity.create_time,
    }
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = SignInSettingRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::sign_in_setting::MSG_NAME_REQUIRED;

    #[test]
    fn validation_wired() {
        assert_eq!(
            SignInSettingCreateDTO::default().validate().unwrap_err(),
            MSG_NAME_REQUIRED
        );
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(20));
        assert_eq!((p.page, p.page_size), (2, 20));
    }
}
