//! 礼品卡分组域服务（marketing::gift_card_groups）。
//!
//! 对应原 Java `GiftCardGroupsServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/promotion/impl/GiftCardGroupsServiceImpl.java`）。
//!
//! 8 个 **admin** 端点：list / updateStatus / create / update / detail /
//! getGiftCardGroups / delete / giftCardGroupProductsUpdate。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - **店铺 ID**：Java `HeaderUtils.getShopId()` 由路由层 `x-admin-type` /
//!   `x-shop-id` 解析后传入（缺失 ⇒ `0` 平台，与既有域一致）。
//! - **商品信息**：Java `productMapper.selectProduct` 返回完整 `ProductSkuVO`；
//!   本实现用简化 [`GiftCardProductVO`]（商品名/图/规格/价格）。
//! - **`canDelete`**：Java 每次 `productMapper.selectCount(...)`；本实现同条件
//!   查询（`gift_card_groups_id` + `shop_id` + `product_status=1` + `is_delete=0`）。
//! - **list 的 N+1**：Java 对每行再调 `detail()` 装配商品；本实现同样逐行装配
//!   （卡组数量小，行为一致）。
//! - **`giftCardGroupProductsUpdate`**：Java 的 `if (isDelete > 0) { insert }`
//!   分支保留（无旧绑定时**不会**插入，属原实现行为，逐字复刻）。
//! - **新建/更新 `expireTime`**：Java 只接受 `yyyy-MM-dd HH:mm:ss` 并取当天
//!   `23:59:59`；非法 →「日期格式错误」。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::GiftCardGroups;
use nextrade_core::models::marketing::gift_card_groups::{
    expire_time_text, parse_end_of_day_timestamp, validate_validity_type, GiftCardGroupIdParam,
    GiftCardGroupsCreateParam, GiftCardGroupsListQuery, GiftCardGroupsUpdateParam, GiftCardGroupsVO,
    GiftCardProductDTO, MSG_GROUP_BOUND_CANNOT_DELETE, MSG_GROUP_NOT_EXISTS,
    MSG_GROUP_STATUS_INVALID, MSG_PRODUCTS_REQUIRED,
};
use nextrade_core::models::marketing::gift_card::{
    card_type_name, group_type_name, validity_type_name, GROUP_TYPE_EXCHANGE, VALIDITY_FIXED,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::gift_card_groups::GiftCardGroupsRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 礼品卡分组域服务。
pub struct GiftCardGroupsService {
    repos: Arc<Repositories>,
}

impl GiftCardGroupsService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.marketing.gift_card_groups`）。
    #[must_use]
    pub fn repo(&self) -> &GiftCardGroupsRepository {
        &self.repos.marketing.gift_card_groups
    }

    // ————————————————————— 查询 —————————————————————

    /// 卡组分页列表（Java `list`）。
    pub async fn list(&self, q: &GiftCardGroupsListQuery) -> Result<PageResult<GiftCardGroupsVO>> {
        // 多租户：店铺账号强制本店；平台账号按平台级（0）——与 Java `getShopId()` 同义
        let shop_id = scope_shop_id();
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, shop_id, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            // Java：每行再 detail() 装配商品
            let products = if row.group_type == GROUP_TYPE_EXCHANGE {
                self.repo().find_product_vos(row.card_groups_id).await?
            } else {
                Vec::new()
            };
            let bound = self
                .repo()
                .bound_product_count(row.card_groups_id, shop_id)
                .await?;
            list.push(group_to_vo(row, products, bound == 0)?);
        }
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 卡组详情（Java `detail`）。
    pub async fn detail(&self, card_groups_id: i32) -> Result<GiftCardGroupsVO> {
        let group = self
            .repo()
            .find_by_id(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        // 多租户：店铺账号只能查看本店卡组
        TenantScope::current_ensure_access(group.shop_id.unwrap_or(0))?;
        let products = if group.group_type == GROUP_TYPE_EXCHANGE {
            self.repo().find_product_vos(card_groups_id).await?
        } else {
            Vec::new()
        };
        let shop_id = group.shop_id.unwrap_or(0);
        let bound = self.repo().bound_product_count(card_groups_id, shop_id).await?;
        group_to_vo(&group, products, bound == 0)
    }

    /// 按卡类型取卡组（Java `getGiftCardGroupsByCardType`）。
    pub async fn get_gift_card_groups(
        &self,
        card_type: Option<i32>,
        gift_card_groups_id: Option<i32>,
    ) -> Result<Vec<GiftCardGroupsVO>> {
        let shop_id = scope_shop_id();
        let Some(card_type) = card_type else {
            return Ok(Vec::new());
        };
        let exclude = self.repo().bound_group_ids(gift_card_groups_id).await?;
        let groups = self
            .repo()
            .list_by_card_type(card_type, shop_id, &exclude)
            .await?;
        groups
            .iter()
            .map(|g| group_to_vo(g, Vec::new(), true))
            .collect()
    }

    // ————————————————————— 写操作 —————————————————————

    /// 修改状态（Java `updateStatus`）。
    pub async fn update_status(&self, param: &GiftCardGroupsUpdateParam) -> Result<()> {
        let card_groups_id = param
            .card_groups_id
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        let status = param
            .status
            .ok_or_else(|| Error::biz(500, MSG_GROUP_STATUS_INVALID))?;
        if status != 0 && status != 1 {
            return Err(Error::biz(500, MSG_GROUP_STATUS_INVALID));
        }
        // 多租户：启停前校验归属
        let group = self
            .repo()
            .find_by_id(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        TenantScope::current_ensure_access(group.shop_id.unwrap_or(0))?;
        self.repo()
            .update_status(card_groups_id, status, now_seconds_i32())
            .await?;
        Ok(())
    }

    /// 创建卡组（Java `createGiftCardGroups`）。
    pub async fn create(&self, param: &GiftCardGroupsCreateParam) -> Result<()> {
        param.validate().map_err(Error::biz_msg)?;
        let validity_type = param.validity_type.unwrap_or_default();
        validate_validity_type(validity_type).map_err(Error::biz_msg)?;
        let expire_time = if validity_type == VALIDITY_FIXED {
            Some(
                parse_end_of_day_timestamp(param.expire_time.as_deref().unwrap_or(""))
                    .map_err(Error::biz_msg)?,
            )
        } else {
            None
        };
        let now = now_seconds_i32();
        let entity = GiftCardGroups {
            card_groups_id: 0,
            group_type: param.group_type.unwrap_or_default(),
            card_type: param.card_type.unwrap_or_default(),
            card_group_name: param.card_group_name.clone().unwrap_or_default(),
            image_url: param.image_url.clone(),
            validity_type,
            validity_days: param.validity_days,
            expire_time,
            face_amount: param.face_amount,
            status: param.status.unwrap_or(0),
            add_time: Some(now),
            update_time: None,
            remark: param.remark.clone(),
            // 多租户：店铺账号强制本店；平台账号可指定/落 0
            shop_id: Some(TenantScope::current().write_shop_id(param.shop_id)?),
            availability_stock: Some(0),
        };
        let group_id = self.repo().insert(&entity).await?;
        if entity.group_type == GROUP_TYPE_EXCHANGE {
            let items = product_tuples(group_id, param.gift_card_products.as_deref())?;
            self.repo().insert_products(&items).await?;
        }
        tracing::info!("创建礼品卡组: id={group_id}");
        Ok(())
    }

    /// 更新卡组（Java `updateGiftCardGroups`）。
    pub async fn update(&self, param: &GiftCardGroupsUpdateParam) -> Result<()> {
        let card_groups_id = param
            .card_groups_id
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        let db = self
            .repo()
            .find_by_id(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(db.shop_id.unwrap_or(0))?;
        param.validate(db.group_type).map_err(Error::biz_msg)?;

        let mut entity = db.clone();
        if let Some(v) = param.card_group_name.clone() {
            entity.card_group_name = v;
        }
        if let Some(v) = param.image_url.clone() {
            entity.image_url = Some(v);
        }
        if let Some(v) = param.validity_type {
            validate_validity_type(v).map_err(Error::biz_msg)?;
            entity.validity_type = v;
            entity.expire_time = if v == VALIDITY_FIXED {
                Some(
                    parse_end_of_day_timestamp(param.expire_time.as_deref().unwrap_or(""))
                        .map_err(Error::biz_msg)?,
                )
            } else {
                None
            };
        }
        if let Some(v) = param.validity_days {
            entity.validity_days = Some(v);
        }
        if let Some(v) = param.face_amount {
            entity.face_amount = Some(v);
        }
        if let Some(v) = param.status {
            entity.status = v;
        }
        if let Some(v) = param.remark.clone() {
            entity.remark = Some(v);
        }
        entity.update_time = Some(now_seconds_i32());
        self.repo().update(&entity).await?;

        if db.group_type == GROUP_TYPE_EXCHANGE {
            let items = product_tuples(card_groups_id, param.gift_card_products.as_deref())?;
            self.repo().delete_products(card_groups_id).await?;
            self.repo().insert_products(&items).await?;
        }
        tracing::info!("更新礼品卡组: id={card_groups_id}");
        Ok(())
    }

    /// 删除卡组（Java `deleteGiftCardGroups`：被有效商品绑定则拒绝）。
    pub async fn delete(&self, param: &GiftCardGroupIdParam) -> Result<()> {
        let card_groups_id = param
            .gift_card_groups_id
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        // 多租户：删除前校验归属
        let group = self
            .repo()
            .find_by_id(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        TenantScope::current_ensure_access(group.shop_id.unwrap_or(0))?;
        let bound = self
            .repo()
            .bound_product_count(card_groups_id, scope_shop_id())
            .await?;
        if bound > 0 {
            return Err(Error::biz(500, MSG_GROUP_BOUND_CANNOT_DELETE));
        }
        self.repo().delete(card_groups_id).await?;
        tracing::info!("删除礼品卡组: id={card_groups_id}");
        Ok(())
    }

    /// 绑定商品管理（Java `giftCardGroupProductsUpdate`）。
    ///
    /// Java 保留 `if (isDelete > 0)` 分支：无旧绑定时**不插入**（原实现行为）。
    pub async fn products_update(&self, param: &GiftCardGroupsUpdateParam) -> Result<()> {
        let card_groups_id = param
            .card_groups_id
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        let db = self
            .repo()
            .find_by_id(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        // 多租户：商品绑定前校验归属
        TenantScope::current_ensure_access(db.shop_id.unwrap_or(0))?;
        if db.group_type != GROUP_TYPE_EXCHANGE {
            return Ok(());
        }
        let products = param
            .gift_card_products
            .as_deref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::biz(500, MSG_PRODUCTS_REQUIRED))?;
        let deleted = self.repo().delete_products(card_groups_id).await?;
        if deleted > 0 {
            let items = product_tuples(card_groups_id, Some(products))?;
            self.repo().insert_products(&items).await?;
        }
        Ok(())
    }
}

/// 商品入参 → `(card_groups_id, product_id, sku_id)` 三元组（缺 ID ⇒ 参数错误）。
fn product_tuples(
    card_groups_id: i32,
    items: Option<&[GiftCardProductDTO]>,
) -> Result<Vec<(i32, i32, i32)>> {
    let Some(items) = items else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|p| {
            let product_id = p
                .product_id
                .ok_or_else(|| Error::Validation("请选择商品".into()))?;
            let sku_id = p
                .sku_id
                .ok_or_else(|| Error::Validation("请选择商品规格".into()))?;
            Ok((card_groups_id, product_id, sku_id))
        })
        .collect()
}

/// 实体 + 商品 + 可删除标记 → VO（Java `detail` + `initTypeName`）。
fn group_to_vo(
    group: &GiftCardGroups,
    products: Vec<nextrade_core::models::marketing::gift_card::GiftCardProductVO>,
    can_delete: bool,
) -> Result<GiftCardGroupsVO> {
    // Java `getExpireTime`：未知有效期类型 ⇒「未知的卡有效期类型」
    validate_validity_type(group.validity_type).map_err(Error::biz_msg)?;
    Ok(GiftCardGroupsVO {
        card_groups_id: Some(group.card_groups_id),
        group_type: Some(group.group_type),
        card_type: Some(group.card_type),
        card_group_name: Some(group.card_group_name.clone()),
        image_url: group.image_url.clone(),
        validity_type: Some(group.validity_type),
        validity_days: group.validity_days,
        expire_time: expire_time_text(group.validity_type, group.expire_time),
        face_amount: group.face_amount,
        status: Some(group.status),
        add_time: Some(format_unix_time(group.add_time)),
        update_time: Some(format_unix_time(group.update_time)),
        group_type_name: Some(group_type_name(group.group_type).to_string()),
        card_type_name: Some(card_type_name(group.card_type).to_string()),
        validity_type_name: Some(validity_type_name(group.validity_type).to_string()),
        remark: group.remark.clone(),
        gift_card_products: products,
        can_delete: Some(can_delete),
    })
}

fn format_unix_time(secs: Option<i32>) -> String {
    nextrade_core::models::marketing::gift_card::format_unix_time(secs)
}

fn now_seconds_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(0)
}

/// `BasePage` → `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> PageQuery {
    PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

/// 当前请求的店铺 ID：店铺账号=本店，平台账号=0（与 Java HeaderUtils.getShopId() 同义）。
fn scope_shop_id() -> i32 {
    TenantScope::current().shop_id().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::gift_card::GiftCardProductVO;

    fn group() -> GiftCardGroups {
        GiftCardGroups {
            card_groups_id: 7,
            group_type: GROUP_TYPE_EXCHANGE,
            card_type: 1,
            card_group_name: "新年礼品卡".into(),
            image_url: Some("http://img".into()),
            validity_type: VALIDITY_FIXED,
            validity_days: None,
            expire_time: Some(1_733_068_799),
            face_amount: None,
            status: 1,
            add_time: Some(1_700_000_000),
            update_time: None,
            remark: Some("备注".into()),
            shop_id: Some(0),
            availability_stock: Some(5),
        }
    }

    #[test]
    fn group_to_vo_derives_type_names_and_expire_text() {
        let products = vec![GiftCardProductVO {
            product_id: 1,
            sku_id: 2,
            product_name: "商品".into(),
            pic_thumb: "p.jpg".into(),
            sku_value: Some("红色".into()),
            sku_price: None,
        }];
        let vo = group_to_vo(&group(), products, true).unwrap();
        assert_eq!(vo.group_type_name.as_deref(), Some("兑换卡"));
        assert_eq!(vo.card_type_name.as_deref(), Some("电子卡"));
        assert_eq!(vo.validity_type_name.as_deref(), Some("固定时间"));
        // FIXED → 格式化；2024-12-01 23:59:59 UTC+8
        assert_eq!(vo.expire_time.as_deref(), Some("2024-12-01 23:59:59"));
        assert_eq!(vo.add_time.as_deref(), Some("2023-11-15 06:13:20"));
        assert_eq!(vo.update_time.as_deref(), Some(""));
        assert_eq!(vo.gift_card_products.len(), 1);
        assert_eq!(vo.can_delete, Some(true));
    }

    #[test]
    fn permanent_group_has_no_expire_text() {
        let mut g = group();
        g.group_type = 1;
        g.validity_type = 1;
        g.expire_time = Some(123);
        let vo = group_to_vo(&g, Vec::new(), false).unwrap();
        assert_eq!(vo.expire_time, None);
        assert_eq!(vo.can_delete, Some(false));
    }

    #[test]
    fn unknown_validity_type_reports_java_text() {
        let mut g = group();
        g.validity_type = 9;
        assert_eq!(
            group_to_vo(&g, Vec::new(), true).unwrap_err().to_string(),
            "业务错误 [500]: 未知的卡有效期类型"
        );
    }

    #[test]
    fn product_tuples_requires_ids() {
        let items = vec![GiftCardProductDTO {
            product_id: Some(11),
            sku_id: Some(22),
            ..Default::default()
        }];
        assert_eq!(
            product_tuples(7, Some(&items)).unwrap(),
            vec![(7, 11, 22)]
        );
        let bad = vec![GiftCardProductDTO::default()];
        assert_eq!(
            product_tuples(7, Some(&bad)).unwrap_err().to_string(),
            "参数校验失败: 请选择商品"
        );
    }

    #[test]
    fn page_query_conversion_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(500));
        assert_eq!((p.page, p.page_size), (2, 100));
    }

    #[tokio::test]
    async fn tenant_scope_selects_shop_or_platform_scope_for_gift_card_groups() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (scope, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scope_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(scope, 8);
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let scope = with_tenant_scope(TenantScope::platform(), async { scope_shop_id() }).await;
        assert_eq!(scope, 0);
    }
}
