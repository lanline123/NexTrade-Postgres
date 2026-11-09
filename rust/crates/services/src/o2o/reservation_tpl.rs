//! 预约模板域服务（o2o::reservation_tpl）。
//!
//! 对应原 Java（overseas）：`com.nextrade.bean.model.o2o.ReservationTpl`
//! + `ReservationTplMapper`（BaseMapper）；参考工程无控制器，
//! 端点路径以线上前端契约为准（`java/view/admin/src/api/setting/appointmentTpl.ts`）。

use std::sync::Arc;

use nextrade_core::entities::o2o::ReservationTpl;
use nextrade_core::models::o2o::reservation_tpl::{
    ReservationTplDelDTO, ReservationTplListQuery, ReservationTplVO, ReservationTplWritePayload,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 预约模板域服务（挂载于 `AdminState.o2o.reservation_tpl`）。
pub struct ReservationTplService {
    repos: Arc<Repositories>,
}

impl ReservationTplService {
    /// 构造。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::o2o::reservation_tpl::ReservationTplRepository {
        &self.repos.o2o.reservation_tpl
    }

    /// `GET /o2o/reservationTpl/pageList`。
    pub async fn page_list(
        &self,
        q: &ReservationTplListQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<ReservationTplVO>> {
        let (total, rows) = self.repo().paginate(q, shop_scope).await?;
        let query = PageQuery {
            page: q.page,
            page_size: q.size,
        };
        Ok(PageResult::new(
            total,
            rows.into_iter().map(to_vo).collect(),
            &query,
        ))
    }

    /// `GET /o2o/reservationTpl/list`（下拉全量）。
    pub async fn list_all(&self, shop_scope: Option<i32>) -> Result<Vec<ReservationTplVO>> {
        Ok(self
            .repo()
            .list_all(shop_scope)
            .await?
            .into_iter()
            .map(to_vo)
            .collect())
    }

    /// `GET /o2o/reservationTpl/detail?id=`。
    pub async fn detail(&self, id: i32) -> Result<ReservationTplVO> {
        let tpl = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("预约模板不存在".into()))?;
        Ok(to_vo(tpl))
    }

    /// `POST /o2o/reservationTpl/create`。
    pub async fn create(&self, p: &ReservationTplWritePayload, shop_scope: Option<i32>) -> Result<i32> {
        if p.template_name.as_deref().unwrap_or("").trim().is_empty() {
            return Err(Error::Validation("模板名称不能为空".into()));
        }
        let shop_id = match shop_scope {
            Some(scope) => scope,
            None => p.shop_id.filter(|x| *x > 0).ok_or_else(|| {
                Error::Forbidden("平台账号创建预约模板时必须指定 shopId".into())
            })?,
        };
        self.repo().insert(p, shop_id).await
    }

    /// `POST /o2o/reservationTpl/update`。
    pub async fn update(&self, p: &ReservationTplWritePayload, shop_scope: Option<i32>) -> Result<()> {
        let id = p
            .reservation_tpl_id
            .filter(|x| *x > 0)
            .ok_or_else(|| Error::Validation("reservationTplId 不能为空".into()))?;
        let exist = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("预约模板不存在".into()))?;
        if let Some(scope) = shop_scope {
            if exist.shop_id != scope {
                return Err(Error::Forbidden("无权修改其它店铺的预约模板".into()));
            }
        }
        self.repo().update(p).await?;
        Ok(())
    }

    /// `POST /o2o/reservationTpl/del`（物理删除）。
    pub async fn delete(&self, dto: &ReservationTplDelDTO, shop_scope: Option<i32>) -> Result<()> {
        let id = dto
            .target_id()
            .ok_or_else(|| Error::Validation("reservationTplId 不能为空".into()))?;
        let exist = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound("预约模板不存在".into()))?;
        if let Some(scope) = shop_scope {
            if exist.shop_id != scope {
                return Err(Error::Forbidden("无权删除其它店铺的预约模板".into()));
            }
        }
        self.repo().delete(id).await?;
        Ok(())
    }
}

/// 实体 → VO（字段一一对应）。
fn to_vo(t: ReservationTpl) -> ReservationTplVO {
    ReservationTplVO {
        reservation_tpl_id: t.reservation_tpl_id,
        template_name: Some(t.template_name),
        reservation_type: t.reservation_type,
        advance_number: t.advance_number,
        advance_unit: t.advance_unit,
        max_days: t.max_days,
        shop_id: t.shop_id,
        is_default: t.is_default,
    }
}
