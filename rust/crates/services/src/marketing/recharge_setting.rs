//! 充值余额配置域服务（marketing::recharge_setting）。
//!
//! Java `RechargeSettingController`（7 端点）→ `RechargeSettingServiceImpl`。
//! 创建校验「充值金额或赠送金额不能小于0」，更新需金额非空；`updateField`
//! 白名单 `{money, discount_money, sort_order, is_show}`；`batch` 仅 del。

//! # 多租户（N/A 判定）
//!
//! 本域为**平台级数据**，不做店铺隔离（N/A），证据：
//! - `java/database/pure.sql` 中 `recharge_setting` 表**没有 `shop_id` 列**；
//! - 原 Java `RechargeSettingServiceImpl` 中**没有任何** `getShopId()` / `shop_id` 引用；
//! - 因此管理端列表/详情/写操作均以平台身份执行，接入 `TenantScope`
//!   不会改变行为（平台作用域默认放行，店铺账号访问时按平台数据可见性处理）。
//!
//! 若未来产品要求店铺级隔离，需先给 `recharge_setting` 增加 `shop_id` 列并回填归属。
//!
use std::sync::Arc;

use nextrade_core::entities::promotion::RechargeSetting;
use nextrade_core::models::marketing::recharge_setting::{
    BatchDTO, OperateDTO, RechargeSettingCreateDTO, RechargeSettingListQuery,
    RechargeSettingUpdateDTO, RechargeSettingVO, UpdateFieldDTO, MSG_NOT_EXISTS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::recharge_setting::RechargeSettingRepository;
use nextrade_infrastructure::db::Repositories;

/// 充值余额配置域服务。
pub struct RechargeSettingService {
    repos: Arc<Repositories>,
}

impl RechargeSettingService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &RechargeSettingRepository {
        &self.repos.marketing.recharge_setting
    }

    pub async fn list(&self, q: &RechargeSettingListQuery) -> Result<PageResult<RechargeSettingVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, limit, offset).await?;
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    pub async fn detail(&self, id: i32) -> Result<RechargeSettingVO> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        Ok(to_vo(&entity))
    }

    pub async fn create(&self, dto: &RechargeSettingCreateDTO) -> Result<i32> {
        let input = dto.validate().map_err(Error::Validation)?;
        let entity = RechargeSetting {
            recharge_id: 0,
            money: Some(input.money),
            discount_money: Some(input.discount_money),
            sort_order: Some(input.sort_order),
            is_show: Some(input.is_show),
        };
        Ok(self.repo().insert(&entity).await?)
    }

    pub async fn update(&self, dto: &RechargeSettingUpdateDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let mut entity = self
            .repo()
            .find_by_id(input.recharge_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        entity.money = Some(input.money);
        entity.discount_money = Some(input.discount_money);
        if let Some(v) = input.sort_order {
            entity.sort_order = Some(v);
        }
        if let Some(v) = input.is_show {
            entity.is_show = Some(v);
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

fn to_vo(entity: &RechargeSetting) -> RechargeSettingVO {
    RechargeSettingVO {
        recharge_id: entity.recharge_id,
        money: entity.money,
        discount_money: entity.discount_money,
        sort_order: entity.sort_order,
        is_show: entity.is_show,
    }
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = RechargeSettingRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::recharge_setting::{MSG_MONEY_REQUIRED, MSG_NEGATIVE};

    #[test]
    fn validation_wired() {
        assert_eq!(
            RechargeSettingCreateDTO::default().validate().unwrap_err(),
            MSG_MONEY_REQUIRED
        );
        let dto = RechargeSettingCreateDTO {
            money: Some(rust_decimal::Decimal::new(-1, 2)),
            discount_money: Some(rust_decimal::Decimal::ZERO),
            ..Default::default()
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_NEGATIVE);
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(20));
        assert_eq!((p.page, p.page_size), (2, 20));
    }
}
