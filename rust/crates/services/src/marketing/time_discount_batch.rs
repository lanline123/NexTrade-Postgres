//! 限时折扣批处理服务（marketing::time_discount_batch）。
//!
//! 对应原 Java `TimeDiscountServiceImpl#batch`：
//! `type == "del"` ⇒ 删除 `time_discount` + `time_discount_item` +
//! `promotion(type=6)`，其余类型静默返回 `true`（Java 两个分支都 `return true`）。
//!
//! > 说明：Java 控制器 `batch(@Valid BatchDTO)` 返回 `void`；本服务返回
//! > `bool`（过程结果），路由层忽略并返回 `null` 数据信封。

use std::sync::Arc;

use nextrade_core::models::marketing::time_discount_batch::{is_delete, validate_batch, BatchDTO};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::time_discount_batch::TimeDiscountBatchRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 限时折扣批处理服务。
pub struct TimeDiscountBatchService {
    repos: Arc<Repositories>,
}

impl TimeDiscountBatchService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.marketing.time_discount_batch`）。
    #[must_use]
    pub fn repo(&self) -> &TimeDiscountBatchRepository {
        &self.repos.marketing.time_discount_batch
    }

    /// 批量删除（Java `TimeDiscountServiceImpl#batch`，级联三步）。
    ///
    /// # Errors
    /// 空 ids / 缺 type 的注解校验失败；数据库失败。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let (ids, _op_type) = validate_batch(dto).map_err(Error::Validation)?;
        // 多租户：批量删除前逐条校验活动归属（time_discount.shop_id）
        for id in &ids {
            if let Some(activity) = self.repos.marketing.time_discount.find_by_id(*id).await? {
                TenantScope::current_ensure_access(activity.shop_id)?;
            }
        }
        if is_delete(dto) {
            let activities = self.repo().delete_activities(&ids).await?;
            let items = self.repo().delete_items(&ids).await?;
            let promotions = self.repo().delete_promotions(&ids).await?;
            tracing::info!(
                "限时折扣批量删除: ids={ids:?} activities={activities} items={items} promotions={promotions}"
            );
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tenant_scope_blocks_cross_shop_batch_delete() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        assert!(with_tenant_scope(shop, async { TenantScope::current_ensure_access(9) })
            .await
            .is_err());
        assert!(with_tenant_scope(shop, async { TenantScope::current_ensure_access(8) })
            .await
            .is_ok());
    }

    #[test]
    fn batch_validation_uses_java_texts() {
        // 纯校验分支（不触库）
        let dto = BatchDTO::default();
        assert_eq!(
            validate_batch(&dto).unwrap_err(),
            "未选择项目"
        );
        let dto = BatchDTO {
            ids: Some(vec![3]),
            ..Default::default()
        };
        assert_eq!(
            validate_batch(&dto).unwrap_err(),
            "操作类型不能为空"
        );
    }

    #[test]
    fn non_delete_type_is_silent_true_in_java() {
        // Java：非 del 分支不做事，最后仍 return true
        let dto = BatchDTO {
            ids: Some(vec![3]),
            r#type: Some("enable".into()),
            ..Default::default()
        };
        assert!(!is_delete(&dto));
        let del = BatchDTO {
            ids: Some(vec![3]),
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert!(is_delete(&del));
    }

    #[test]
    fn validation_error_maps_to_validation_kind() {
        let dto = BatchDTO::default();
        let err = Error::Validation(validate_batch(&dto).unwrap_err());
        assert_eq!(err.to_string(), "参数校验失败: 未选择项目");
    }
}
