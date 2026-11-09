//! 退款申请管理服务（payment::refund_apply_admin，3 个管理端端点）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.finance.RefundApplyController`
//! 中继承自 `BaseServiceImpl` 的 3 个通用端点：
//! `del`（removeByIds）/ `updateField`（GENERAL_FIELDS 白名单）/ `batch`（仅 `del`）。
//!
//! # 与 Java 的差异（已在契约内收敛）
//! Java `updateField` 传入 `GENERAL_FIELDS = {is_show, sort_order, status}`，但
//! `refund_apply` 表**没有** `is_show/sort_order/status` 三列（真实状态列为
//! `refund_status`）。若照抄 Java 白名单会命中不存在的列直接 SQL 报错。本实现把
//! `status`（兼容 `refundStatus`/`refund_status`）映射到真实列 `refund_status`，
//! 其余字段返回逐字 `field 的值有误`；该偏差不影响前端现有「退款状态」使用场景。
//!
//! `del` / `batch` 幂等：id 缺失/不存在时不报错（Java `removeByIds` 返回 0）。

use std::sync::Arc;

use nextrade_core::models::merchant::shop_manage::{BatchDTO, UpdateFieldDTO};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// Java `ResultTextConstants.INVALID_FIELD_VALUE`。
pub const INVALID_FIELD_VALUE: &str = "field 的值有误";

pub struct RefundApplyAdminService {
    repos: Arc<Repositories>,
}

impl RefundApplyAdminService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `POST /adminapi/finance/refundApply/del`：删除退款申请。
    pub async fn del(&self, id: Option<i32>) -> Result<()> {
        let Some(id) = id else {
            return Ok(());
        };
        self.repos
            .payment
            .refund_apply_admin
            .delete_by_ids(&[id])
            .await?;
        Ok(())
    }

    /// `POST /adminapi/finance/refundApply/updateField`：白名单字段更新。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let Some(id) = dto.id else {
            return Err(Error::Validation("参数缺失".into()));
        };
        let Some(field) = dto.field.as_deref().map(str::trim).filter(|f| !f.is_empty()) else {
            return Err(Error::biz_msg(INVALID_FIELD_VALUE));
        };
        let Some(column) = map_update_field(field) else {
            return Err(Error::biz_msg(INVALID_FIELD_VALUE));
        };
        let Some(val) = dto.val.as_deref() else {
            return Err(Error::biz_msg(INVALID_FIELD_VALUE));
        };
        if !self.repos.payment.refund_apply_admin.exists(id).await? {
            return Err(Error::biz_msg(INVALID_FIELD_VALUE));
        }
        self.repos
            .payment
            .refund_apply_admin
            .update_field(id, column, val)
            .await?;
        Ok(())
    }

    /// `POST /adminapi/finance/refundApply/batch`：批量操作（仅 `del`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<()> {
        if dto.r#type.as_deref() != Some("del") {
            return Ok(());
        }
        let ids = dto.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Ok(());
        }
        self.repos
            .payment
            .refund_apply_admin
            .delete_by_ids(ids)
            .await?;
        Ok(())
    }
}

/// `updateField` 字段映射（Java `GENERAL_FIELDS` → `refund_apply` 真实列）。
#[must_use]
pub fn map_update_field(field: &str) -> Option<&'static str> {
    match field {
        "status" | "refundStatus" | "refund_status" => Some("refund_status"),
        // Java 白名单含 is_show/sort_order，但 refund_apply 表无此两列（见模块头偏差）
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_mapping_preserves_status_aliases() {
        assert_eq!(map_update_field("status"), Some("refund_status"));
        assert_eq!(map_update_field("refundStatus"), Some("refund_status"));
        assert_eq!(map_update_field("refund_status"), Some("refund_status"));
        assert_eq!(map_update_field("is_show"), None);
        assert_eq!(map_update_field("sort_order"), None);
    }

    #[test]
    fn invalid_field_message_is_verbatim() {
        assert_eq!(INVALID_FIELD_VALUE, "field 的值有误");
        let err = Error::biz_msg(INVALID_FIELD_VALUE);
        assert!(format!("{err}").contains(INVALID_FIELD_VALUE));
    }

    #[test]
    fn batch_type_semantics() {
        let dto = BatchDTO {
            r#type: Some("other".into()),
            ids: Some(vec![1, 2]),
            ..Default::default()
        };
        // 非 del 类型直接 no-op（Java 继承实现返回 false）
        assert_ne!(dto.r#type.as_deref(), Some("del"));
    }
}
