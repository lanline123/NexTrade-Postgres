//! 用户收货地址（C 端）服务（member::user_address_client）。
//!
//! 对应原 Java `com.nextrade.service.user.impl.UserAddressServiceImpl`
//! （`java/nextrade-service/.../user/impl/UserAddressServiceImpl.java`）
//! 与继承的 `BaseServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（7）
//! - `tagList`：固定标签「家,公司,学校」
//! - `list`：当前用户 + 非系统地址 + 默认/自定义排序 + 分页
//! - `detail`：本人地址校验 + 区名/区 ID JSON 解析
//! - `create`：区名回查 + 默认地址互斥 + 可选设为选中（事务）
//! - `update`：`addressId` 定位 + 本人校验 + 区名回查 + 默认互斥（事务）
//! - `del`：本人校验后删除
//! - `setSelected`：先清后设（事务）
//!
//! ## 与 Java 的差异（诚实记录）
//! - `create` 的 `user_id` 兜底、区名排序、`sortItems` 默认排序等见
//!   [`crate::models::member::user_address_client`] 模块头。
//! - 业务错误码沿用项目惯例 500（Java `GlobalException` 为 1001），消息逐字。
//! - `update` 的 `id` 缺失：Java 用 `addressId` 定位、缺失时 `getById(null)`
//!   抛异常；本实现返回参数校验错误「收货地址id不能为空」。
//! - `pics`/金额/时间：本域无。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_core::entities::UserAddress;
use nextrade_core::models::member::user_address_client::{
    address_tag_list, join_region_names, join_region_names_dedup, normalize_postcode,
    parse_i32_list, parse_string_list, resolve_sort, to_json, UserAddressCreateDTO,
    UserAddressIdDTO, UserAddressListQuery, UserAddressUpdateDTO, UserAddressVO,
    MSG_ADDRESS_ID_REQUIRED, MSG_ADDRESS_NOT_EXIST, MSG_ADDRESS_NOT_EXIST_UPDATE,
    MSG_DEL_NO_PERMISSION, MSG_USER_NOT_LOGIN, MSG_VIEW_NO_PERMISSION,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 收货地址服务。
pub struct UserAddressClientService {
    repos: Arc<Repositories>,
}

impl UserAddressClientService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.member.user_address_client`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::member::user_address_client::UserAddressRepository
    {
        &self.repos.member.user_address_client
    }

    /// 常用标签（Java `getAddressTagList`，无 DB 访问）。
    #[must_use]
    pub fn tag_list(&self) -> Vec<&'static str> {
        address_tag_list()
    }

    /// 地址列表（Java `#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        user_id: i32,
        q: &UserAddressListQuery,
    ) -> Result<PageResult<UserAddressVO>> {
        let page = q.page_query();
        // Java：`userId = listDTO.userId ?? getCurrentUserId()`
        let user_id = q.user_id.unwrap_or(user_id);
        let sort = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let (total, rows) = self.repo().paginate(user_id, sort, q).await?;
        let list = rows.iter().map(|e| to_vo(e, true)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 地址详情（Java `#detail(id, userId)`）。
    ///
    /// `user_id == 0`（游客）由路由层拦截「用户未登录」；这里只校验归属。
    ///
    /// # Errors
    /// - 不存在 → 业务错误「用户地址不存在」
    /// - 非本人 → 业务错误「无权限查看此收货地址」
    pub async fn detail(&self, user_id: i32, id: Option<i32>) -> Result<UserAddressVO> {
        let id = require_address_id(id)?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_ADDRESS_NOT_EXIST))?;
        if user_id != 0 && row.user_id != user_id {
            return Err(Error::biz_msg(MSG_VIEW_NO_PERMISSION));
        }
        Ok(to_vo(&row, false))
    }

    /// 创建地址（Java `#create`，`@Transactional`）。
    ///
    /// # Errors
    /// - 邮编超长 → 业务错误「邮政编码长度不能超过60个字符」
    /// - 数据库失败 → `Error::Database`
    pub async fn create(&self, current_user: i32, dto: &UserAddressCreateDTO) -> Result<i32> {
        let region_ids = dto.region_ids.clone().unwrap_or_default();
        let region_names = self.repo().find_region_names(&region_ids).await?;
        let postcode = normalize_postcode(dto.postcode.as_deref()).map_err(Error::biz_msg)?;
        let entity = UserAddress {
            address_id: 0,
            country_code: None,
            country_name: None,
            address_tag: dto.address_tag.clone().unwrap_or_default(),
            user_id: dto.user_id.unwrap_or(current_user),
            consignee: dto.consignee.clone().unwrap_or_default(),
            email: dto.email.clone(),
            region_ids: to_json(&region_ids),
            region_names: to_json(&region_names),
            address: dto.address.clone().unwrap_or_default(),
            address_line_two: None,
            building_name: None,
            floor: None,
            unit: None,
            address_data: None,
            formatted_address: None,
            position_address: dto.position_address.clone(),
            postcode,
            telephone: dto.telephone.clone(),
            mobile: dto.mobile.clone().unwrap_or_default(),
            is_default: dto.is_default.unwrap_or(0),
            is_system: dto.is_system,
            is_selected: 0,
            mobile_area_code: dto.mobile_area_code.clone(),
            international_mobile: None,
            latitude: decimal_from_f64(dto.latitude),
            longitude: decimal_from_f64(dto.longitude),
        };
        // Java：`isDefault == 1` 时清当前用户其它默认（注意用的是 getCurrentUserId）
        let clear_default_user = if dto.is_default == Some(1) {
            Some(current_user)
        } else {
            None
        };
        let set_selected = dto.is_selected == Some(1);
        let id = self
            .repo()
            .create(&entity, clear_default_user, set_selected, current_user)
            .await?;
        tracing::info!("创建收货地址: id={id}, user_id={current_user}");
        Ok(id)
    }

    /// 更新地址（Java `#update`）。
    ///
    /// # Errors
    /// - `addressId` 缺失 → 参数校验「收货地址id不能为空」
    /// - 不存在 / 非本人 → 业务错误「地址不存在」
    /// - 邮编超长 → 业务错误「邮政编码长度不能超过60个字符」
    pub async fn update(&self, user_id: i32, dto: &UserAddressUpdateDTO) -> Result<()> {
        let address_id = dto
            .address_id
            .ok_or_else(|| Error::Validation(MSG_ADDRESS_ID_REQUIRED.into()))?;
        let existing = self.repo().find_by_id(address_id).await?;
        let existing = match existing {
            Some(row) if row.user_id == user_id => row,
            // Java 控制器：不存在或非本人 → `error.address.not.exist`（「地址不存在」）
            _ => return Err(Error::biz_msg(MSG_ADDRESS_NOT_EXIST_UPDATE)),
        };

        let region_ids = match &dto.region_ids {
            Some(ids) if !ids.is_empty() => ids.clone(),
            _ => parse_i32_list(&existing.region_ids),
        };
        let region_names = if dto.region_ids.as_ref().is_some_and(|v| !v.is_empty()) {
            self.repo().find_region_names(&region_ids).await?
        } else {
            parse_string_list(&existing.region_names)
        };
        let postcode = match dto.postcode.as_deref() {
            Some(raw) => normalize_postcode(Some(raw)).map_err(Error::biz_msg)?,
            None => existing.postcode.clone(),
        };

        let merged = UserAddress {
            address_id,
            country_code: existing.country_code,
            country_name: existing.country_name,
            address_tag: dto
                .address_tag
                .clone()
                .unwrap_or(existing.address_tag),
            user_id: existing.user_id,
            consignee: dto.consignee.clone().unwrap_or(existing.consignee),
            email: dto.email.clone().or(existing.email),
            region_ids: to_json(&region_ids),
            region_names: to_json(&region_names),
            address: dto.address.clone().unwrap_or(existing.address),
            address_line_two: existing.address_line_two,
            building_name: existing.building_name,
            floor: existing.floor,
            unit: existing.unit,
            address_data: existing.address_data,
            formatted_address: existing.formatted_address,
            position_address: dto
                .position_address
                .clone()
                .or(existing.position_address),
            postcode,
            telephone: dto.telephone.clone().or(existing.telephone),
            mobile: dto.mobile.clone().unwrap_or(existing.mobile),
            is_default: dto.is_default.unwrap_or(existing.is_default),
            // Java 控制器显式保留原 isSystem（前端不可改）
            is_system: existing.is_system,
            is_selected: dto.is_selected.unwrap_or(existing.is_selected),
            mobile_area_code: dto
                .mobile_area_code
                .clone()
                .or(existing.mobile_area_code),
            international_mobile: existing.international_mobile,
            latitude: dto
                .latitude
                .map_or(existing.latitude, |v| decimal_from_f64(Some(v))),
            longitude: dto
                .longitude
                .map_or(existing.longitude, |v| decimal_from_f64(Some(v))),
        };
        // Java：isDefault > 0 时清当前用户其它地址默认（ne addressId）
        let clear_default_of = if dto.is_default.unwrap_or(0) > 0 {
            Some((user_id, address_id))
        } else {
            None
        };
        self.repo().update(&merged, clear_default_of).await?;
        tracing::info!("更新收货地址: id={address_id}, user_id={user_id}");
        Ok(())
    }

    /// 删除地址（Java `#del`）。
    ///
    /// # Errors
    /// - `id` 缺失 → 参数校验「收货地址id不能为空」
    /// - 不存在 / 非本人 → 业务错误「地址不存在或无权限删除」
    pub async fn delete(&self, user_id: i32, dto: &UserAddressIdDTO) -> Result<()> {
        let id = require_address_id(dto.id)?;
        let row = self.repo().find_by_id(id).await?;
        match row {
            Some(row) if row.user_id == user_id => {}
            _ => return Err(Error::biz_msg(MSG_DEL_NO_PERMISSION)),
        }
        self.repo().delete(id).await?;
        tracing::info!("删除收货地址: id={id}, user_id={user_id}");
        Ok(())
    }

    /// 设置选中地址（Java `#setSelected`）。
    ///
    /// # Errors
    /// - `id` 缺失 → 参数校验「收货地址id不能为空」
    /// - 数据库失败 → `Error::Database`
    pub async fn set_selected(&self, user_id: i32, dto: &UserAddressIdDTO) -> Result<()> {
        let id = require_address_id(dto.id)?;
        self.repo().set_selected(user_id, id).await?;
        tracing::info!("设置选中地址: id={id}, user_id={user_id}");
        Ok(())
    }

    /// 路由层登录校验（Java 控制器 `userId == null || userId == 0 → 用户未登录`）。
    ///
    /// # Errors
    /// 游客 → 业务错误「用户未登录」。
    pub fn ensure_logged_in(user_id: i32) -> Result<()> {
        if user_id == 0 {
            return Err(Error::biz_msg(MSG_USER_NOT_LOGIN));
        }
        Ok(())
    }
}

/// 地址 ID 必填校验。
fn require_address_id(id: Option<i32>) -> Result<i32> {
    id.ok_or_else(|| Error::Validation(MSG_ADDRESS_ID_REQUIRED.into()))
}

/// 实体 → VO（Java `BeanUtils.copyProperties` + 区名 JSON 解析 + 区名拼接）。
///
/// `for_list = true` 时复刻 Java `list` 的「前两个区名相同则去重」逻辑；
/// `false` 对应 `detail`（不去重）。
#[must_use]
fn to_vo(e: &UserAddress, for_list: bool) -> UserAddressVO {
    let region_names = parse_string_list(&e.region_names);
    let region_name = if for_list {
        join_region_names_dedup(&region_names)
    } else {
        join_region_names(&region_names)
    };
    UserAddressVO {
        address: e.address.clone(),
        position_address: e.position_address.clone(),
        address_id: e.address_id,
        consignee: e.consignee.clone(),
        email: e.email.clone(),
        is_default: e.is_default,
        is_selected: e.is_selected,
        mobile: e.mobile.clone(),
        mobile_area_code: e.mobile_area_code.clone(),
        region_name,
        region_names,
        telephone: e.telephone.clone(),
        user_id: e.user_id,
        region_ids: parse_i32_list(&e.region_ids),
        postcode: e.postcode.clone(),
        address_tag: e.address_tag.clone(),
        latitude: e.latitude.and_then(decimal_to_f64),
        longitude: e.longitude.and_then(decimal_to_f64),
        is_system: e.is_system,
    }
}

/// `f64` → `Decimal`（Java `BigDecimal`；非法浮点为 `None`）。
fn decimal_from_f64(value: Option<f64>) -> Option<Decimal> {
    value.and_then(|v| {
        if v.is_finite() {
            Decimal::try_from(v).ok()
        } else {
            None
        }
    })
}

/// `Decimal` → `f64`（VO 序列化为 JSON number，与 Java `BigDecimal` 一致）。
fn decimal_to_f64(value: rust_decimal::Decimal) -> Option<f64> {
    use rust_decimal::prelude::ToPrimitive;
    value.to_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address() -> UserAddress {
        UserAddress {
            address_id: 5,
            country_code: None,
            country_name: None,
            address_tag: "家".into(),
            user_id: 9,
            consignee: "张三".into(),
            email: Some("a@b.c".into()),
            region_ids: "[1,2]".into(),
            region_names: "[\"北京\",\"北京\",\"朝阳\"]".into(),
            address: "某路1号".into(),
            address_line_two: None,
            building_name: None,
            floor: None,
            unit: None,
            address_data: None,
            formatted_address: None,
            position_address: Some("某地".into()),
            postcode: Some("100000".into()),
            telephone: None,
            mobile: "13800000000".into(),
            is_default: 1,
            is_system: Some(0),
            is_selected: 1,
            mobile_area_code: Some("+86".into()),
            international_mobile: None,
            latitude: rust_decimal::Decimal::try_from(39.9).ok(),
            longitude: rust_decimal::Decimal::try_from(116.4).ok(),
        }
    }

    #[test]
    fn vo_maps_region_json_and_dedups_only_for_list() {
        let list_vo = to_vo(&address(), true);
        assert_eq!(list_vo.region_names, vec!["北京", "北京", "朝阳"]);
        assert_eq!(list_vo.region_name, "北京 朝阳");
        assert_eq!(list_vo.region_ids, vec![1, 2]);
        assert_eq!(list_vo.address_id, 5);
        assert_eq!(list_vo.user_id, 9);

        let detail_vo = to_vo(&address(), false);
        assert_eq!(detail_vo.region_name, "北京 北京 朝阳");
    }

    #[test]
    fn vo_handles_bad_json_columns() {
        let mut e = address();
        e.region_names = "not-json".into();
        e.region_ids = String::new();
        let vo = to_vo(&e, true);
        assert!(vo.region_names.is_empty());
        assert!(vo.region_ids.is_empty());
        assert_eq!(vo.region_name, "");
    }

    #[test]
    fn address_id_validation() {
        assert_eq!(
            require_address_id(None).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_ADDRESS_ID_REQUIRED}")
        );
        assert_eq!(require_address_id(Some(3)).unwrap(), 3);
    }

    #[test]
    fn login_guard_blocks_anonymous() {
        match UserAddressClientService::ensure_logged_in(0) {
            Err(Error::Biz { msg, .. }) => assert_eq!(msg, MSG_USER_NOT_LOGIN),
            other => panic!("应为业务错误，实际: {other:?}"),
        }
        assert!(UserAddressClientService::ensure_logged_in(1).is_ok());
    }
}
