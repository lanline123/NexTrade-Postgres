//! 预约模板（`/adminapi/o2o/reservationTpl`）DTO。
//!
//! Java 来源（overseas）：
//! - `com.nextrade.bean.model.o2o.ReservationTpl`（实体，`@TableName reservation_tpl`）
//! - `com.nextrade.bean.dto.o2o.reservationtpl.ReservationTplListDTO`（分页查询）
//! - 前端契约：`java/view/admin/src/api/setting/appointmentTpl.ts`
//!   （`pageList` / `list` / `detail` / `create` / `update` / `del`）

use serde::{Deserialize, Serialize};

/// `GET /adminapi/o2o/reservationTpl/pageList` 查询参数。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationTplListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_size", alias = "pageSize")]
    pub size: u64,
    /// 模板名称模糊。
    pub keyword: Option<String>,
}

fn default_page() -> u64 {
    1
}

fn default_size() -> u64 {
    20
}

impl ReservationTplListQuery {
    /// 分页 LIMIT/OFFSET。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let size = self.size.clamp(1, 200) as i64;
        let page = self.page.max(1) as i64;
        (size, (page - 1) * size)
    }
}

/// 预约模板 VO（`reservation_tpl` 行 camelCase，字段与 Java `ReservationTpl` 一致）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationTplVO {
    pub reservation_tpl_id: i32,
    pub template_name: Option<String>,
    /// 0=无需提前，1=需提前N天/小时，2=根据下单时间配置。
    pub reservation_type: i32,
    pub advance_number: Option<i32>,
    /// d=天，h=小时。
    pub advance_unit: Option<String>,
    pub max_days: i32,
    pub shop_id: i32,
    pub is_default: i32,
}

/// 创建 / 更新入参（字段对齐 Java `ReservationTpl` 可编辑列）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationTplWritePayload {
    /// 更新时必填。
    pub reservation_tpl_id: Option<i32>,
    pub template_name: Option<String>,
    pub reservation_type: Option<i32>,
    pub advance_number: Option<i32>,
    pub advance_unit: Option<String>,
    pub max_days: Option<i32>,
    /// 创建时可指定（平台必须显式传；门店账号强制为自己的 shop_id）。
    pub shop_id: Option<i32>,
    pub is_default: Option<i32>,
}

/// 删除入参（`reservation_tpl_id`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationTplDelDTO {
    pub reservation_tpl_id: Option<i32>,
    pub id: Option<i32>,
}

impl ReservationTplDelDTO {
    /// 兼容 `reservationTplId` / `id` 两种传参。
    #[must_use]
    pub fn target_id(&self) -> Option<i32> {
        self.reservation_tpl_id.or(self.id).filter(|x| *x > 0)
    }
}
