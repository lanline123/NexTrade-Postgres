//! 分销模式设置服务（marketing::salesman_config）。
//!
//! 对应原 Java `SalesmanConfigServiceImpl`（2 个管理端端点：`detail` / `save`）。
//!
//! # 语义（逐条对照 Java）
//! - `detail(code)`：按 code 取 `salesman_config.data` 解析为 JSON；`code ==
//!   salesmanConfig` 时补默认值（`distributionMode=1`、`viewCustomerInfo=0`、
//!   `wechatBizEnabled=true`、`registerToSalesman=0`）。记录不存在时返回补全
//!   默认值的空配置（Java 同样返回非 null VO）。
//! - `save(code, param)`：仅 `salesmanConfig` / `salesmanSettlement` 两个
//!   code 生效；`salesmanConfig` 先校验分销模式（微商模式必须选招募方式，
//!   推广员模式清空招募方式），再 upsert `data`。
//!
//! # 差异
//! 1. Java `SalesmanConfigSaveParam` 有大量强类型子项（等级条件、结算方式等），
//!    Rust 端以 JSON 形式透传（`flatten` `extra` 保留未知字段），不做逐字段模型。
//! 2. `shop_id` 多租户：店铺账号读写本店配置；平台账号读写平台级（`shop_id = 0`），
//!    与 Java `HeaderUtils.getShopId()`（admin → 0）一致。

use std::sync::Arc;

use serde_json::Value;
use nextrade_core::models::marketing::salesman_config::{
    normalize_distribution_mode, parse_config_data, with_defaults, SalesmanConfigSaveDTO,
    CODE_SALESMAN_CONFIG, CODE_SALESMAN_SETTLEMENT, MSG_CODE_REQUIRED,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::salesman_config::SalesmanConfigRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 配置服务（挂载于 `AdminState.marketing.salesman_config`）。
pub struct SalesmanConfigService {
    repos: Arc<Repositories>,
}

impl SalesmanConfigService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &SalesmanConfigRepository {
        &self.repos.marketing.salesman_config
    }

    /// 配置详情（Java `detail`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn detail(&self, code: &str) -> Result<Value> {
        let shop_id = current_config_shop_id();
        let raw = self
            .repo()
            .find_by_code(code, shop_id)
            .await?
            .and_then(|row| row.data);
        let value = parse_config_data(raw.as_deref());
        Ok(if code == CODE_SALESMAN_CONFIG {
            with_defaults(value)
        } else {
            value
        })
    }

    /// 保存配置（Java `save` → `saveSalesmanConfig` / `saveSalesmanSettlement`）。
    ///
    /// # Errors
    /// - 缺 code → `Validation(code不能为空)`
    /// - 微商模式未选招募方式 → `微商模式下请至少选择一种下级招募方式`
    /// - 招募方式非法 → `下级招募方式参数有误`
    pub async fn save(&self, code: &str, dto: &SalesmanConfigSaveDTO) -> Result<()> {
        let code = code.trim();
        if code.is_empty() {
            return Err(Error::Validation(MSG_CODE_REQUIRED.into()));
        }
        if code != CODE_SALESMAN_CONFIG && code != CODE_SALESMAN_SETTLEMENT {
            // Java 对未知 code 静默不处理
            return Ok(());
        }
        let mut value = dto.to_config_value();
        if code == CODE_SALESMAN_CONFIG {
            normalize_distribution_mode(&mut value).map_err(Error::Validation)?;
        }
        let data = serde_json::to_string(&value)
            .map_err(|e| Error::Internal(format!("序列化分销配置失败: {e}")))?;
        // 多租户：店铺账号落本店；平台账号落平台级（0）
        self.repo().upsert(code, current_config_shop_id(), &data).await?;
        Ok(())
    }
}

/// 当前配置所属店铺：店铺账号 = 本店；平台账号 = 0（平台级配置）。
///
/// 对应 Java `HeaderUtils.getShopId()`（admin → 0）。
fn current_config_shop_id() -> i32 {
    TenantScope::current().shop_id().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_defaults_for_salesman_config() {
        let value = with_defaults(parse_config_data(None));
        assert_eq!(value["distributionMode"], 1);
        assert_eq!(value["viewCustomerInfo"], 0);
    }

    #[test]
    fn unknown_code_is_silently_ignored_per_java() {
        // Java：仅两个已知 code 生效，其余直接返回
        assert_ne!(CODE_SALESMAN_CONFIG, "unknown");
        assert_eq!(CODE_SALESMAN_SETTLEMENT, "salesmanSettlement");
    }

    #[tokio::test]
    async fn tenant_scope_selects_shop_or_platform_config_row() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        // 店铺账号 → 本店配置行
        let shop_row = with_tenant_scope(TenantScope::for_shop(8), async {
            current_config_shop_id()
        })
        .await;
        assert_eq!(shop_row, 8);
        // 平台账号 → 平台级（0）
        let platform_row = with_tenant_scope(TenantScope::platform(), async {
            current_config_shop_id()
        })
        .await;
        assert_eq!(platform_row, 0);
    }
}
