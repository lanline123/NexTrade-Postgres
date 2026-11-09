//! 店铺资金域服务（merchant::shop_funds）。
//!
//! 对应原 Java `ShopFundsServiceImpl` 的 8 个管理端端点：
//! `list` / `detail` / `create` / `update` / `del` / `updateField` / `batch` /
//! `applyShopAgreement`。
//!
//! # 语义（逐条对照 Java）
//! - `list`：排除自提点（`shop_type<>3`）与已归档（`is_delete` 空/0）→
//!   `status>0` / `shop_type` / `keyword`(shop_title LIKE) → 白名单列排序
//!   （缺省 `shop_id DESC`；`unSettlementOrder` 派生排序未迁移，回退主键）。
//! - `detail`：`getById`，缺失返回 `null`（Java 同样返回 null）。
//! - `create`：`ShopFundsCreateDTO` → `shop` 表插入（Java `save` 语义）。
//! - `update`：按 `shopId` 局部更新（`None` 保留原值）；店铺不存在 → 业务错误。
//! - `del` / `batch`：软删（`is_delete=1`）；`batch` 仅支持 `del`。
//! - `updateField`：白名单 `status`（Java `GENERAL_FIELDS` 中 `shop` 实存列）。
//! - `applyShopAgreement`：返回 `config.shopAgreement` 原文（缺失空串）。
//!
//! # 金额
//! `shop_money` / `frozen_money` 为 `rust_decimal::Decimal`。

use std::sync::Arc;

use nextrade_core::models::merchant::shop_funds::{
    format_time, normalize_field, status_text, BatchDTO, OperateDTO, ShopFundsCreateDTO,
    ShopFundsListQuery, ShopFundsUpdateDTO, ShopFundsVO, UpdateFieldDTO, BIZ_SHOP_AGREEMENT,
    DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, MSG_ID_REQUIRED,
    MSG_INVALID_FIELD_VALUE, MSG_SHOP_NOT_FOUND,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::shop_funds::{
    ShopFundsRepository, ShopFundsRow, ShopFundsUpdateFields,
};
use nextrade_infrastructure::db::Repositories;

/// 店铺资金域服务（挂载于 `AdminState.merchant.shop_funds`）。
pub struct ShopFundsService {
    repos: Arc<Repositories>,
}

impl ShopFundsService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &ShopFundsRepository {
        &self.repos.merchant.shop_funds
    }

    /// 资金列表分页（Java `ShopFundsServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list(&self, q: &ShopFundsListQuery) -> Result<PageResult<ShopFundsVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .paginate(
                q.status.filter(|s| *s > 0),
                q.shop_type,
                q.keyword.as_deref(),
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                limit,
                offset,
            )
            .await?;
        let list: Vec<ShopFundsVO> = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 资金详情（Java `detail`：缺失返回 null）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn detail(&self, id: i32) -> Result<Option<ShopFundsVO>> {
        Ok(self.repo().find_by_id(id).await?.as_ref().map(to_vo))
    }

    /// 创建店铺资金记录（Java `ShopFundsServiceImpl#create`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn create(&self, dto: &ShopFundsCreateDTO) -> Result<()> {
        let shop_id = dto
            .shop_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation("店铺ID不能为空".into()))?;
        if self.repo().find_by_id(shop_id).await?.is_some() {
            return Err(Error::biz(500, "店铺账户已存在"));
        }
        let id = self.repo().insert_shop_funds(dto).await?;
        tracing::info!("创建店铺资金记录: shop_id={id}");
        Ok(())
    }

    /// 编辑资金字段（局部更新，`None` 保留原值）。
    ///
    /// # Errors
    /// - 缺 `shopId` → `Validation(id不能为空)`
    /// - 店铺不存在 → `Error::biz(500, 店铺不存在)`
    pub async fn update(&self, dto: &ShopFundsUpdateDTO) -> Result<()> {
        let shop_id = dto
            .shop_id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        if self.repo().find_by_id(shop_id).await?.is_none() {
            return Err(Error::biz(500, MSG_SHOP_NOT_FOUND));
        }
        let fields = ShopFundsUpdateFields {
            shop_title: dto.shop_title.clone(),
            shop_logo: dto.shop_logo.clone(),
            shop_money: dto.shop_money,
            frozen_money: dto.frozen_money,
            status: dto.status,
            merchant_id: dto.merchant_id,
            description: dto.description.clone(),
            contact_mobile: dto.contact_mobile.clone(),
        };
        self.repo().update_fields(shop_id, &fields).await?;
        Ok(())
    }

    /// 删除（软删；Java `BaseServiceImpl#del`）。
    ///
    /// # Errors
    /// 缺 id → `Validation`；数据库错误 → `Database`。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repo().soft_delete(&[id]).await?;
        Ok(())
    }

    /// 更新单个字段。
    ///
    /// # Errors
    /// 缺参数 / 白名单外字段 → `Validation`。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("值不能为空".into()))?;
        let column = normalize_field(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        self.repo().update_field(id, column, val).await
    }

    /// 批量操作（仅 `del` → 软删）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !dto.r#type.as_deref().is_some_and(|t| t == "del") {
            return Ok(false);
        }
        let Some(ids) = dto.ids.as_deref() else {
            return Ok(false);
        };
        if ids.is_empty() {
            return Ok(false);
        }
        Ok(self.repo().soft_delete(ids).await? > 0)
    }

    /// 入驻协议（Java `configService.getConfigVal(SettingsEnum.SHOP_AGREEMENT)`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn apply_shop_agreement(&self) -> Result<String> {
        self.repo().config_val(BIZ_SHOP_AGREEMENT).await
    }
}

/// 行 → VO（Java `convertToVO`）。
#[must_use]
fn to_vo(row: &ShopFundsRow) -> ShopFundsVO {
    ShopFundsVO {
        shop_id: row.shop_id,
        shop_title: row.shop_title.clone(),
        shop_logo: row.shop_logo.clone(),
        add_time: format_time(row.add_time),
        click_count: row.click_count,
        status: row.status,
        status_text: status_text(row.status),
        merchant_id: row.merchant_id,
        shop_money: row.shop_money,
        frozen_money: row.frozen_money,
        contact_mobile: row.contact_mobile.clone(),
        description: row.description.clone(),
        kefu_phone: row.kefu_phone.clone(),
        kefu_weixin: row.kefu_weixin.clone(),
        kefu_link: row.kefu_link.clone(),
        is_contact_kefu: row.is_contact_kefu,
        kefu_inlet: row.kefu_inlet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_vo_maps_money_and_status_text() {
        let row = ShopFundsRow {
            shop_id: 3,
            shop_title: Some("旗舰店".into()),
            add_time: Some(1_700_000_000),
            status: Some(1),
            shop_money: Some("100.50".parse().unwrap()),
            frozen_money: Some("0.00".parse().unwrap()),
            merchant_id: Some(9),
            ..Default::default()
        };
        let vo = to_vo(&row);
        assert_eq!(vo.shop_title.as_deref(), Some("旗舰店"));
        assert_eq!(vo.status_text, "开业");
        assert_eq!(vo.shop_money, Some("100.50".parse().unwrap()));
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.merchant_id, Some(9));
    }

    #[test]
    fn create_dto_accepts_optional_fields() {
        let dto = ShopFundsCreateDTO {
            shop_title: Some("测试店铺".into()),
            shop_money: Some(rust_decimal::Decimal::new(100, 2)),
            ..Default::default()
        };
        assert_eq!(dto.shop_title.as_deref(), Some("测试店铺"));
    }

    #[test]
    fn update_field_whitelist_error_is_verbatim() {
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
        assert_eq!(MSG_SHOP_NOT_FOUND, "店铺不存在");
    }
}
