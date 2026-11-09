//! 收银台设备域服务（pos::device）。
//!
//! ⚠️ Java 来源：参考工程（overseas / b2b2b-pro）**无** `PosDeviceController`
//! （grep `pos/device` / `PosDevice` 仅命中打印模块 `PrintPosDevice`，不是
//! `pos_device` 表）。本服务按表 `pos_device` schema（`java/database/pure.sql`
//! `:3060`，5.8.55→5.8.56 增量）+ 线上管理端前端契约
//! （`java/view/admin/src/api/setting/cashier.ts`）实现 BaseMapper 风格 CRUD。
//!
//! 租户口径：店铺 / 门店 / POS 账号（`TenantContext::shop_scope()`）只能看 /
//! 写本店设备；平台账号可管理全量（列表不限定）。

use std::sync::Arc;

use nextrade_core::entities::o2o::PosDevice;
use nextrade_core::models::pos::device::{
    PosDeviceDelDTO, PosDeviceListQuery, PosDeviceUpdateFieldDTO, PosDeviceVO,
    PosDeviceWritePayload, POS_DEVICE_UPDATE_FIELDS,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 收银台设备域服务（挂载于 `AdminState.pos.device`）。
pub struct PosDeviceService {
    repos: Arc<Repositories>,
}

impl PosDeviceService {
    /// 构造。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::pos::device::PosDeviceRepository {
        &self.repos.pos.device
    }

    /// `GET /pos/device/list`：分页。
    pub async fn list(
        &self,
        q: &PosDeviceListQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<PosDeviceVO>> {
        let (total, rows) = self.repo().paginate(q, shop_scope).await?;
        let query = PageQuery {
            page: q.page,
            page_size: q.size,
        };
        Ok(PageResult::new(total, rows.into_iter().map(to_vo).collect(), &query))
    }

    /// `GET /pos/device/all`：全量（下拉用）。
    pub async fn list_all(&self, shop_scope: Option<i32>) -> Result<Vec<PosDeviceVO>> {
        Ok(self
            .repo()
            .list_all(shop_scope)
            .await?
            .into_iter()
            .map(to_vo)
            .collect())
    }

    /// `GET /pos/device/detail?id=`：详情（不存在报错）。
    pub async fn detail(&self, id: i32, shop_scope: Option<i32>) -> Result<PosDeviceVO> {
        let device = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("收银设备不存在".into()))?;
        if let Some(scope) = shop_scope {
            if device.shop_id != scope {
                return Err(Error::Forbidden(format!(
                    "无权访问店铺 {} 的收银设备",
                    device.shop_id
                )));
            }
        }
        Ok(to_vo(device))
    }

    /// `POST /pos/device/create`：创建设备。
    pub async fn create(&self, p: &PosDeviceWritePayload, shop_scope: Option<i32>) -> Result<i32> {
        let device_name = required(&p.device_name, "设备名称")?;
        let device_code = required(&p.device_code, "设备码")?;
        let shop_id = match shop_scope {
            Some(scope) => scope,
            None => p.shop_id.filter(|x| *x > 0).ok_or_else(|| {
                Error::Forbidden("平台账号创建设备时必须指定 shopId".into())
            })?,
        };
        let now = nextrade_common::utils::now_millis();
        self.repo()
            .insert(shop_id, &device_name, &device_code, p.status.unwrap_or(1), now)
            .await
    }

    /// `POST /pos/device/update`：编辑设备。
    pub async fn update(&self, p: &PosDeviceWritePayload, shop_scope: Option<i32>) -> Result<()> {
        let id = p.id.filter(|x| *x > 0).ok_or_else(|| Error::Validation("id 不能为空".into()))?;
        self.ensure_access(id, shop_scope).await?;
        let now = nextrade_common::utils::now_millis();
        self.repo()
            .update_fields(
                id,
                p.device_name.as_deref(),
                p.device_code.as_deref(),
                p.status,
                now,
            )
            .await?;
        Ok(())
    }

    /// `POST /pos/device/updateField`：白名单单字段更新。
    pub async fn update_field(&self, dto: &PosDeviceUpdateFieldDTO, shop_scope: Option<i32>) -> Result<()> {
        let id = dto.id.filter(|x| *x > 0).ok_or_else(|| Error::Validation("id 不能为空".into()))?;
        let field_raw = dto.field.as_deref().unwrap_or("").trim();
        let val = dto.val.as_deref().unwrap_or("");
        if val.is_empty() {
            return Err(Error::Validation("val 不能为空".into()));
        }
        // 前端字段 camelCase → 列名蛇形
        let field = camel_to_snake(field_raw);
        if !POS_DEVICE_UPDATE_FIELDS.contains(&field.as_str()) {
            return Err(Error::Validation(format!("不支持更新字段 {field_raw}")));
        }
        self.ensure_access(id, shop_scope).await?;
        let now = nextrade_common::utils::now_millis();
        self.repo().update_single_field(id, &field, val, now).await?;
        Ok(())
    }

    /// `POST /pos/device/del`：删除（物理删）。
    pub async fn delete(&self, dto: &PosDeviceDelDTO, shop_scope: Option<i32>) -> Result<u64> {
        let ids = dto.id_list();
        if ids.is_empty() {
            return Err(Error::Validation("id 不能为空".into()));
        }
        for id in &ids {
            self.ensure_access(*id, shop_scope).await?;
        }
        self.repo().delete_by_ids(&ids).await
    }

    /// 店铺账号校验设备归属（平台放行）。
    async fn ensure_access(&self, id: i32, shop_scope: Option<i32>) -> Result<()> {
        let Some(scope) = shop_scope else {
            return Ok(());
        };
        let device = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("收银设备不存在".into()))?;
        if device.shop_id != scope {
            return Err(Error::Forbidden(format!(
                "无权访问店铺 {} 的收银设备",
                device.shop_id
            )));
        }
        Ok(())
    }
}

/// 实体 → VO。
fn to_vo(d: PosDevice) -> PosDeviceVO {
    PosDeviceVO {
        id: d.id,
        shop_id: d.shop_id,
        device_name: Some(d.device_name),
        device_code: Some(d.device_code),
        status: Some(d.status),
        last_used_time: d.last_used_time,
        add_time: Some(d.add_time),
        update_time: Some(d.update_time),
    }
}

/// 必填字符串。
fn required(v: &Option<String>, name: &str) -> Result<String> {
    let s = v
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation(format!("{name}不能为空")))?;
    Ok(s.to_string())
}

/// camelCase → snake_case（updateField 字段名归一）。
fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_to_snake_basic() {
        assert_eq!(camel_to_snake("deviceName"), "device_name");
        assert_eq!(camel_to_snake("status"), "status");
        assert_eq!(camel_to_snake("lastUsedTime"), "last_used_time");
    }

    #[test]
    fn whitelist_fields_snake() {
        assert!(POS_DEVICE_UPDATE_FIELDS.contains(&"device_name"));
        assert!(!POS_DEVICE_UPDATE_FIELDS.contains(&"shop_id"));
    }
}
