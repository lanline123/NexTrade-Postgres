//! 商户域服务（merchant::merchant）。
//!
//! 对应原 Java `MerchantServiceImpl` 的管理端 8 个端点：
//! `uploadLicense` / `list` / `detail` / `create` / `update` / `del` /
//! `updateField` / `batch`。
//!
//! # 语义（逐条对照 Java）
//! - `list`：`(is_delete is null or 0)` + `merchantId` 精确 + `keyword`
//!   （companyName/corporateName LIKE）→ 回填 `user`（会员精简）、`admin`
//!   （`merchant_user.is_admin=1` + `admin_user.username`）、`shopCount`
//!   （`shop` 表按 merchant，排除自提点）、`statusText` / `typeText` / `addTime`。
//! - `detail`：同 VO + 解析 `baseData`/`shopData`/`merchantData` JSON，并用
//!   `region` 回填 `fullBusinessLicenseAddress` / `fullDetailedAddress`。
//! - `create`：落 `merchant` 主表（`add_time=now`、`merchant_apply_id=0`、
//!   `status` 缺省 1；`type/companyName/corporateName` 取 `baseData` 或 DTO）。
//! - `update`：不存在抛 `商户不存在`；整行合并更新（JSON 原样落库）。
//! - `del` / `batch`：软删（`is_delete=1`）；`batch` 仅支持 `del`。
//! - `updateField`：白名单 `status`（Java `GENERAL_FIELDS` 中 `merchant`
//!   表唯一实存列），非法字段抛 `field 的值有误`。
//! - `uploadLicense`：水印 / 内容类型 / 对象存储依赖未迁移 → 业务错误
//!   （CLIENT 应改用既有文件上传后回填 `merchantData.businessLicenseImg`）。
//!
//! # 与 Java 的差异
//! 1. `create` 不创建 `admin_user` / `merchant_user` 关联（Java 涉及邀请码、
//!    角色、密码加密、短信等未迁移链路）；
//! 2. 删除为软删；Java `delMerchant` 语义同为软删（`is_delete=1`）；
//! 3. 列表 `sortField/sortOrder` 支持白名单列，缺省 `merchant_id DESC`。

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use nextrade_core::entities::Merchant;
use nextrade_core::models::merchant::merchant::{
    base_field_i32, base_field_str, format_time, normalize_field, status_text, type_text,
    BatchDTO, MerchantAdminVO, MerchantCreateDTO, MerchantListQuery, MerchantUpdateDTO,
    MerchantUserVO, MerchantVO, OperateDTO, UpdateFieldDTO, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
    MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_LICENSE_UNAVAILABLE, MSG_MERCHANT_NOT_FOUND,
    STATUS_NORMAL,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::merchant::MerchantRepository;
use nextrade_infrastructure::db::Repositories;

/// 商户域服务（挂载于 `AdminState.merchant.merchant`）。
pub struct MerchantService {
    repos: Arc<Repositories>,
}

impl MerchantService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &MerchantRepository {
        &self.repos.merchant.merchant
    }

    /// 商户分页（Java `MerchantServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list(&self, q: &MerchantListQuery) -> Result<PageResult<MerchantVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .paginate(
                q.merchant_id,
                q.status,
                q.keyword.as_deref(),
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                limit,
                offset,
            )
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let merchant_ids: Vec<i32> = rows.iter().map(|m| m.merchant_id).collect();
        let links = self.repo().find_admin_links(&merchant_ids).await?;
        let admin_ids: Vec<i32> = links.iter().filter_map(|l| l.admin_user_id).collect();
        let admin_names: HashMap<i32, String> =
            self.repo().find_admin_usernames(&admin_ids).await?.into_iter().collect();
        let user_ids: Vec<i32> = rows.iter().filter_map(|m| m.user_id).collect();
        let users: HashMap<i32, MerchantUserVO> = self
            .repo()
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, MerchantUserVO {
                user_id: u.user_id,
                username: u.username,
                nickname: u.nickname,
                mobile: u.mobile,
                avatar: u.avatar,
            }))
            .collect();

        let mut list = Vec::with_capacity(rows.len());
        for m in &rows {
            let mut vo = base_vo(m);
            vo.shop_count = self.repo().shop_count_by_merchant(m.merchant_id).await?;
            if let Some(link) = links.iter().find(|l| l.merchant_id == m.merchant_id) {
                let admin = MerchantAdminVO {
                    merchant_id: link.merchant_id,
                    admin_user_id: link.admin_user_id,
                    user_id: link.user_id,
                    is_admin: link.is_admin,
                    admin_id: link.admin_user_id,
                    username: link
                        .admin_user_id
                        .and_then(|id| admin_names.get(&id).cloned()),
                };
                vo.admin = Some(admin);
            }
            if let Some(uid) = m.user_id {
                vo.user = users.get(&uid).cloned();
            }
            // Java：若存在 admin 关联，`vo.user` 覆盖为管理员用户名（历史行为）
            if let Some(admin) = &vo.admin {
                if let Some(name) = &admin.username {
                    vo.user = Some(MerchantUserVO {
                        username: name.clone(),
                        ..Default::default()
                    });
                }
            }
            list.push(vo);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 商户详情（Java `MerchantServiceImpl#detail`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn detail(&self, id: i32) -> Result<MerchantVO> {
        let m = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_MERCHANT_NOT_FOUND))?;
        let mut vo = base_vo(&m);
        let base = parse_json(m.base_data.as_deref());
        let merchant_data = parse_json(m.merchant_data.as_deref());
        // Java：merchantData.businessAddress + detailedAddress → fullDetailedAddress
        let mut merchant_data = merchant_data;
        let mut base = base;
        if let Some(data) = merchant_data.as_mut().and_then(Value::as_object_mut) {
            let ids = json_id_array(data.get("businessAddress"));
            let detailed = data
                .get("detailedAddress")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if !ids.is_empty() {
                let names = self.repo().find_region_names(&ids).await?;
                let full = join_full_address(&ids, &names, &detailed);
                data.insert("fullDetailedAddress".into(), Value::String(full));
            }
        }
        if let Some(data) = base.as_mut().and_then(Value::as_object_mut) {
            let ids = json_id_array(data.get("licenseAddrProvince"));
            let detail = data
                .get("businessLicenseAddress")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if !ids.is_empty() {
                let names = self.repo().find_region_names(&ids).await?;
                let full = join_full_address(&ids, &names, &detail);
                data.insert("fullBusinessLicenseAddress".into(), Value::String(full));
            }
        }
        vo.base_data = base;
        vo.merchant_data = merchant_data;
        vo.shop_data = parse_json(m.shop_data.as_deref());
        let mut links = self.repo().find_admin_links(&[m.merchant_id]).await?;
        if let Some(link) = links.pop() {
            let admin = MerchantAdminVO {
                merchant_id: link.merchant_id,
                admin_user_id: link.admin_user_id,
                user_id: link.user_id,
                is_admin: link.is_admin,
                admin_id: link.admin_user_id,
                username: None,
            };
            vo.admin = Some(admin);
        }
        vo.shop_count = self.repo().shop_count_by_merchant(m.merchant_id).await?;
        if let Some(uid) = m.user_id {
            vo.user = self
                .repo()
                .find_users(&[uid])
                .await?
                .into_iter()
                .next()
                .map(|u| MerchantUserVO {
                    user_id: u.user_id,
                    username: u.username,
                    nickname: u.nickname,
                    mobile: u.mobile,
                    avatar: u.avatar,
                });
        }
        Ok(vo)
    }

    /// 新增商户（Java `MerchantServiceImpl#create` 主表部分）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn create(&self, dto: &MerchantCreateDTO) -> Result<i32> {
        let base = dto.base_data.as_ref();
        let now = now_secs_i32();
        let entity = Merchant {
            merchant_id: 0,
            merchant_apply_id: Some(0),
            user_id: dto.user_id,
            add_time: Some(now),
            base_data: dto.base_data.as_ref().map(json_to_string),
            shop_data: dto.shop_data.as_ref().map(json_to_string),
            merchant_data: dto.merchant_data.as_ref().map(json_to_string),
            status: Some(dto.status.unwrap_or(STATUS_NORMAL)),
            is_delete: Some(0),
            r#type: dto.r#type.or_else(|| base_field_i32(base, "type")),
            company_name: dto
                .company_name
                .clone()
                .or_else(|| base_field_str(base, "companyName")),
            corporate_name: dto
                .corporate_name
                .clone()
                .or_else(|| base_field_str(base, "corporateName")),
            settlement_cycle: dto.settlement_cycle,
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建商户: merchant_id={id}");
        Ok(id)
    }

    /// 编辑商户（Java `MerchantServiceImpl#update`）。
    ///
    /// # Errors
    /// 商户不存在 → `Error::biz(500, 商户不存在)`；数据库错误 → `Database`。
    pub async fn update(&self, dto: &MerchantUpdateDTO) -> Result<()> {
        let Some(merchant_id) = dto.merchant_id else {
            return Err(Error::Validation(MSG_ID_REQUIRED.into()));
        };
        let Some(mut entity) = self.repo().find_by_id(merchant_id).await? else {
            return Err(Error::biz(500, MSG_MERCHANT_NOT_FOUND));
        };
        let base = dto.base_data.as_ref();
        entity.base_data = dto.base_data.as_ref().map(json_to_string);
        entity.merchant_data = dto.merchant_data.as_ref().map(json_to_string);
        entity.shop_data = dto.shop_data.as_ref().map(json_to_string);
        entity.r#type = dto.r#type.or_else(|| base_field_i32(base, "type"));
        entity.company_name = dto
            .company_name
            .clone()
            .or_else(|| base_field_str(base, "companyName"));
        entity.corporate_name = dto
            .corporate_name
            .clone()
            .or_else(|| base_field_str(base, "corporateName"));
        if let Some(v) = dto.settlement_cycle {
            entity.settlement_cycle = Some(v);
        }
        if let Some(v) = dto.status {
            entity.status = Some(v);
        }
        self.repo().update(&entity).await?;
        Ok(())
    }

    /// 删除商户（软删，Java `delMerchant`）。
    ///
    /// # Errors
    /// 缺 id → `Validation`；不存在 → `商户不存在`。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        if self.repo().find_by_id(id).await?.is_none() {
            return Err(Error::biz(500, MSG_MERCHANT_NOT_FOUND));
        }
        self.repo().soft_delete(&[id], now_secs_i32()).await?;
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

    /// 批量操作（Java `BaseServiceImpl#batch`：仅 `del`）。
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
        let affected = self.repo().soft_delete(ids, now_secs_i32()).await?;
        Ok(affected > 0)
    }

    /// 营业执照上传（Java `uploadLicense`）：依赖水印 / 对象存储，未迁移。
    ///
    /// # Errors
    /// 恒定返回业务错误（文案见 [`MSG_LICENSE_UNAVAILABLE`]）。
    pub fn upload_license(&self) -> Result<String> {
        Err(Error::biz(501, MSG_LICENSE_UNAVAILABLE))
    }
}

/// 实体 → VO 公共字段（Java `BeanUtils.copyProperties` + 派生文案/时间）。
fn base_vo(m: &Merchant) -> MerchantVO {
    MerchantVO {
        merchant_id: m.merchant_id,
        merchant_apply_id: m.merchant_apply_id,
        user_id: m.user_id,
        add_time: format_time(m.add_time),
        base_data: parse_json(m.base_data.as_deref()),
        shop_data: parse_json(m.shop_data.as_deref()),
        merchant_data: parse_json(m.merchant_data.as_deref()),
        status: m.status,
        status_text: status_text(m.status),
        r#type: m.r#type,
        type_text: type_text(m.r#type),
        company_name: m.company_name.clone(),
        corporate_name: m.corporate_name.clone(),
        settlement_cycle: m.settlement_cycle,
        shop_count: 0,
        user: None,
        admin: None,
    }
}

fn parse_json(raw: Option<&str>) -> Option<Value> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
}

fn json_to_string(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".to_string())
}

fn json_id_array(v: Option<&Value>) -> Vec<i32> {
    v.and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_i64().and_then(|n| i32::try_from(n).ok()))
                .collect()
        })
        .unwrap_or_default()
}

/// 地区名 + 详细地址拼完整地址（Java `fullDetailedAddress` 口径：以空格连接）。
fn join_full_address(ids: &[i32], names: &[(i32, String)], detailed: &str) -> String {
    let map: HashMap<i32, &str> = names.iter().map(|(id, name)| (*id, name.as_str())).collect();
    let mut out = String::new();
    for id in ids {
        if let Some(name) = map.get(id) {
            out.push_str(name);
            out.push(' ');
        }
    }
    out.push_str(detailed);
    out.trim().to_string()
}

fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_full_address_matches_java() {
        let names = vec![(1, "江西省".to_string()), (2, "南昌市".to_string())];
        assert_eq!(
            join_full_address(&[1, 2], &names, "红谷滩区 1 号"),
            "江西省 南昌市 红谷滩区 1 号"
        );
        // 缺失地区跳过；无详细地址时 trim
        assert_eq!(join_full_address(&[1, 9], &names, ""), "江西省");
    }

    #[test]
    fn json_helpers_are_tolerant() {
        assert_eq!(json_id_array(Some(&serde_json::json!([1, 2, "x"]))), vec![1, 2]);
        assert!(json_id_array(None).is_empty());
        assert!(parse_json(Some("not-json")).is_none());
        assert_eq!(
            parse_json(Some(r#"{"a":1}"#)).unwrap()["a"],
            serde_json::json!(1)
        );
        assert_eq!(json_to_string(&serde_json::json!({"a":1})), r#"{"a":1}"#);
    }

    #[test]
    fn base_vo_derives_texts() {
        let m = Merchant {
            merchant_id: 7,
            status: Some(2),
            r#type: Some(2),
            add_time: Some(1_700_000_000),
            company_name: Some("ACME".into()),
            ..Default::default()
        };
        let vo = base_vo(&m);
        assert_eq!(vo.status_text, "取消资格");
        assert_eq!(vo.type_text, "企业认证");
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.company_name.as_deref(), Some("ACME"));
    }
}
