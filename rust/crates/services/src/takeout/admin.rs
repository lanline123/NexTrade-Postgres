//! 后台管理服务（takeout::admin，我方新增）：渠道配置、门店绑定、
//! SKU 映射、外部订单与出站动作（确认/物流同步/退款应答/重新处理）。
//!
//! 权限口径（走认证中间件写入的 `TenantScope`，不信任客户端租户头）：
//! - 渠道配置（含凭据）：**仅平台账号**可读写；
//! - 绑定/映射/外部订单：平台看全部，门店只看/管本店；
//! - 密钥与授权令牌只进不出：读接口一律脱敏为「是否已配置」。

use std::sync::Arc;

use serde_json::{json, Value};

use nextrade_common::utils::now_seconds;
use nextrade_core::models::takeout::{
    settings_to_value, settings_view, validate_binding, validate_mapping, BindingSaveDTO,
    ChannelConfigSaveDTO, ExternalOrderListQuery, ExternalOrderVO, IdDTO, MappingSaveDTO,
    EXT_STATUS_CONFIRMED, TAKEOUT_SETTINGS_CODE,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::repo::takeout::order::ExternalOrderRow;
use nextrade_infrastructure::db::Repositories;

use super::channel::adapter_for;
use super::ingest::TakeoutIngestService;

/// 文案：渠道配置平台专属。
pub const ERR_CONFIG_PLATFORM_ONLY: &str = "渠道配置仅平台账号可操作";
/// 文案：外部订单不存在。
pub const ERR_ORDER_NOT_FOUND: &str = "外部订单不存在";
/// 文案：绑定不存在。
pub const ERR_BINDING_NOT_FOUND: &str = "门店绑定不存在";
/// 文案：映射不存在。
pub const ERR_MAPPING_NOT_FOUND: &str = "SKU 映射不存在";

/// 后台管理服务。
pub struct TakeoutAdminService {
    repos: Arc<Repositories>,
    ingest: Arc<TakeoutIngestService>,
}

impl TakeoutAdminService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>, ingest: Arc<TakeoutIngestService>) -> Self {
        Self { repos, ingest }
    }

    fn now() -> i32 {
        i32::try_from(now_seconds()).unwrap_or(0)
    }

    // ------------------------------------------------------------------
    // 概览与渠道配置
    // ------------------------------------------------------------------

    /// 概览：渠道状态（脱敏）+ 绑定数 + 待绑定外部单数。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn overview(&self) -> Result<Value> {
        let settings = self.ingest.settings().await?;
        let scope = TenantScope::current();
        let bindings = self
            .repos
            .takeout
            .binding
            .list(scope.list_filter(), None)
            .await?;
        let (recent, _) = self
            .repos
            .takeout
            .order
            .list(scope.list_filter(), None, None, None, 1, 1000)
            .await?;
        let pending_binding = recent
            .iter()
            .filter(|r| r.status == "pending_binding")
            .count();
        let mut view = settings_view(&settings);
        view["bindingCount"] = json!(bindings.len());
        view["pendingBindingCount"] = json!(pending_binding);
        view["callbackBase"] = json!("/api/takeout/callback");
        Ok(view)
    }

    /// 保存单渠道配置（仅平台；`appSecret` 空 = 保持原值）。
    ///
    /// # Errors
    /// 非平台账号 → 业务错误；渠道非法 → 业务错误；DB 错误直传。
    pub async fn save_config(&self, dto: &ChannelConfigSaveDTO) -> Result<Value> {
        if !TenantScope::current().is_platform() {
            return Err(Error::biz_msg(ERR_CONFIG_PLATFORM_ONLY));
        }
        let mut settings = self.ingest.settings().await?;
        let current_secret = settings
            .channel(&dto.channel)
            .map(|c| c.app_secret.clone())
            .ok_or_else(|| Error::biz_msg("渠道有误"))?;
        let target = settings
            .channel_mut(&dto.channel)
            .ok_or_else(|| Error::biz_msg("渠道有误"))?;
        target.enabled = dto.enabled;
        target.app_id = dto.app_id.trim().to_string();
        if !dto.app_secret.trim().is_empty() {
            target.app_secret = dto.app_secret.trim().to_string();
        } else {
            target.app_secret = current_secret;
        }
        target.gateway_base = dto.gateway_base.trim().to_string();
        let value = settings_to_value(&settings);
        self.repos
            .system
            .config_basic
            .save_code(TAKEOUT_SETTINGS_CODE, &value.to_string())
            .await?;
        Ok(settings_view(&settings))
    }

    // ------------------------------------------------------------------
    // 门店绑定
    // ------------------------------------------------------------------

    /// 绑定列表（令牌脱敏）。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn list_bindings(&self, channel: Option<&str>) -> Result<Value> {
        let scope = TenantScope::current();
        let rows = self
            .repos
            .takeout
            .binding
            .list(scope.list_filter(), channel)
            .await?;
        let list: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "channel": r.channel,
                    "shopId": r.shop_id,
                    "appPoiCode": r.app_poi_code,
                    "externalPoiId": r.external_poi_id,
                    "externalPoiName": r.external_poi_name,
                    "authStatus": r.auth_status,
                    "accessTokenConfigured": !r.access_token.is_empty(),
                    "autoAccept": r.auto_accept == 1,
                    "enabled": r.enabled == 1,
                    "addTime": r.add_time,
                    "updateTime": r.update_time,
                })
            })
            .collect();
        Ok(json!({ "list": list, "total": list.len() }))
    }

    /// 保存绑定（门店账号强制本店；令牌空 = 保持原值）。
    ///
    /// # Errors
    /// 入参非法 / 越权 / 绑定键重复 → 业务错误；DB 错误直传。
    pub async fn save_binding(&self, dto: &BindingSaveDTO) -> Result<i32> {
        if let Some(msg) = validate_binding(dto) {
            return Err(Error::biz_msg(msg));
        }
        let scope = TenantScope::current();
        let shop_id = scope.write_shop_id(Some(dto.shop_id))?;
        if shop_id <= 0 {
            return Err(Error::biz_msg("请选择门店"));
        }
        let now = Self::now();
        let mut row = nextrade_infrastructure::db::repo::takeout::ChannelBindingRow {
            id: dto.id,
            channel: dto.channel.clone(),
            shop_id,
            app_poi_code: dto.app_poi_code.trim().to_string(),
            external_poi_id: dto.external_poi_id.trim().to_string(),
            external_poi_name: dto.external_poi_name.trim().to_string(),
            auth_status: dto.auth_status,
            access_token: dto.access_token.trim().to_string(),
            auto_accept: i32::from(dto.auto_accept),
            enabled: i32::from(dto.enabled),
            add_time: now,
            update_time: now,
        };
        if dto.id > 0 {
            let old = self
                .repos
                .takeout
                .binding
                .find_by_id(dto.id)
                .await?
                .ok_or_else(|| Error::biz_msg(ERR_BINDING_NOT_FOUND))?;
            scope.ensure_access(old.shop_id)?;
            row.add_time = old.add_time;
            if row.access_token.is_empty() {
                row.access_token = old.access_token.clone();
            }
        }
        self.repos.takeout.binding.upsert(&row).await
    }

    /// 删除绑定（归属校验）。
    ///
    /// # Errors
    /// 不存在 / 越权 → 业务错误；DB 错误直传。
    pub async fn delete_binding(&self, dto: &IdDTO) -> Result<()> {
        let old = self
            .repos
            .takeout
            .binding
            .find_by_id(dto.id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_BINDING_NOT_FOUND))?;
        TenantScope::current().ensure_access(old.shop_id)?;
        self.repos.takeout.binding.delete(dto.id).await
    }

    // ------------------------------------------------------------------
    // SKU 映射
    // ------------------------------------------------------------------

    /// 映射列表。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn list_mappings(&self, channel: Option<&str>) -> Result<Value> {
        let scope = TenantScope::current();
        let rows = self
            .repos
            .takeout
            .mapping
            .list(scope.list_filter(), channel)
            .await?;
        let list: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "channel": r.channel,
                    "shopId": r.shop_id,
                    "externalSpuId": r.external_spu_id,
                    "externalSkuId": r.external_sku_id,
                    "externalName": r.external_name,
                    "productId": r.product_id,
                    "skuId": r.sku_id,
                    "addTime": r.add_time,
                    "updateTime": r.update_time,
                })
            })
            .collect();
        Ok(json!({ "list": list, "total": list.len() }))
    }

    /// 保存映射（门店账号强制本店）。
    ///
    /// # Errors
    /// 入参非法 / 越权 / 重复 → 业务错误；DB 错误直传。
    pub async fn save_mapping(&self, dto: &MappingSaveDTO) -> Result<i32> {
        if let Some(msg) = validate_mapping(dto) {
            return Err(Error::biz_msg(msg));
        }
        let scope = TenantScope::current();
        let shop_id = scope.write_shop_id(Some(dto.shop_id))?;
        if shop_id <= 0 {
            return Err(Error::biz_msg("请选择门店"));
        }
        let now = Self::now();
        let mut row = nextrade_infrastructure::db::repo::takeout::SkuMappingRow {
            id: dto.id,
            channel: dto.channel.clone(),
            shop_id,
            external_spu_id: dto.external_spu_id.trim().to_string(),
            external_sku_id: dto.external_sku_id.trim().to_string(),
            external_name: dto.external_name.trim().to_string(),
            product_id: dto.product_id,
            sku_id: dto.sku_id,
            add_time: now,
            update_time: now,
        };
        if dto.id > 0 {
            let old = self
                .repos
                .takeout
                .mapping
                .find_by_id(dto.id)
                .await?
                .ok_or_else(|| Error::biz_msg(ERR_MAPPING_NOT_FOUND))?;
            scope.ensure_access(old.shop_id)?;
            row.add_time = old.add_time;
        }
        self.repos.takeout.mapping.upsert(&row).await
    }

    /// 删除映射（归属校验）。
    ///
    /// # Errors
    /// 不存在 / 越权 → 业务错误；DB 错误直传。
    pub async fn delete_mapping(&self, dto: &IdDTO) -> Result<()> {
        let old = self
            .repos
            .takeout
            .mapping
            .find_by_id(dto.id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_MAPPING_NOT_FOUND))?;
        TenantScope::current().ensure_access(old.shop_id)?;
        self.repos.takeout.mapping.delete(dto.id).await
    }

    // ------------------------------------------------------------------
    // 外部订单与出站动作
    // ------------------------------------------------------------------

    fn to_vo(row: &ExternalOrderRow, with_payload: bool) -> ExternalOrderVO {
        ExternalOrderVO {
            id: row.id,
            channel: row.channel.clone(),
            channel_name: nextrade_core::models::takeout::channel_name(&row.channel).to_string(),
            external_order_id: row.external_order_id.clone(),
            shop_id: row.shop_id,
            internal_order_id: row.internal_order_id,
            external_status: row.external_status,
            status: row.status.clone(),
            total_amount: row.total_amount,
            consignee: row.consignee.clone(),
            mobile: row.mobile.clone(),
            address: row.address.clone(),
            last_event: row.last_event.clone(),
            last_error: row.last_error.clone(),
            add_time: row.add_time,
            update_time: row.update_time,
            raw_payload: if with_payload {
                Some(row.raw_payload.clone())
            } else {
                None
            },
        }
    }

    /// 外部订单分页列表。
    ///
    /// # Errors
    /// DB 错误直传。
    pub async fn list_orders(
        &self,
        query: &ExternalOrderListQuery,
    ) -> Result<PageResult<ExternalOrderVO>> {
        let scope = TenantScope::current();
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
        let (rows, total) = self
            .repos
            .takeout
            .order
            .list(
                scope.list_filter(),
                query.channel.as_deref(),
                query.status.as_deref(),
                query.keyword.as_deref(),
                page,
                page_size,
            )
            .await?;
        let total_u = u64::try_from(total).unwrap_or(0);
        let size_u = u64::try_from(page_size).unwrap_or(20);
        Ok(PageResult {
            total: total_u,
            list: rows.iter().map(|r| Self::to_vo(r, false)).collect(),
            total_page: total_u.div_ceil(size_u),
            page_size: size_u,
            page: u64::try_from(page).unwrap_or(1),
        })
    }

    /// 外部订单详情（含归一载荷；归属校验，待绑定单仅平台可见）。
    ///
    /// # Errors
    /// 不存在 / 越权 → 业务错误；DB 错误直传。
    pub async fn order_detail(&self, id: i32) -> Result<ExternalOrderVO> {
        let row = self.load_order_checked(id).await?;
        Ok(Self::to_vo(&row, true))
    }

    async fn load_order_checked(&self, id: i32) -> Result<ExternalOrderRow> {
        let row = self
            .repos
            .takeout
            .order
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_ORDER_NOT_FOUND))?;
        if row.shop_id > 0 {
            TenantScope::current().ensure_access(row.shop_id)?;
        } else if !TenantScope::current().is_platform() {
            return Err(Error::biz_msg(ERR_ORDER_NOT_FOUND));
        }
        Ok(row)
    }

    /// 手动确认接单（自动确认失败/关闭时的补救；真实调用渠道 API，
    /// 失败原样返回渠道错误，不假成功）。
    ///
    /// # Errors
    /// 未绑定 / 渠道未启用 / 渠道返回失败 → 业务错误。
    pub async fn confirm_order(&self, id: i32) -> Result<()> {
        let row = self.load_order_checked(id).await?;
        let (cfg, adapter, binding) = self.action_context(&row).await?;
        adapter
            .confirm_order(
                &cfg,
                &binding.app_poi_code,
                &binding.access_token,
                &row.external_order_id,
            )
            .await?;
        let mut updated = row.clone();
        updated.status = EXT_STATUS_CONFIRMED.to_string();
        updated.last_error = String::new();
        updated.update_time = Self::now();
        self.repos.takeout.order.update(&updated).await?;
        if updated.internal_order_id > 0 {
            self.repos
                .takeout
                .order
                .update_internal_order_status(
                    updated.internal_order_id,
                    nextrade_core::models::takeout::internal_order_status(EXT_STATUS_CONFIRMED),
                    nextrade_core::models::takeout::internal_pay_status(EXT_STATUS_CONFIRMED),
                    None,
                )
                .await?;
        }
        Ok(())
    }

    /// 手动同步自配送物流状态（渠道口径状态码由前端按渠道文档传）。
    ///
    /// # Errors
    /// 未绑定 / 渠道未启用 / 渠道返回失败 → 业务错误。
    pub async fn sync_logistics(&self, id: i32, logistics_status: i32) -> Result<()> {
        let row = self.load_order_checked(id).await?;
        let (cfg, adapter, binding) = self.action_context(&row).await?;
        adapter
            .sync_logistics(
                &cfg,
                &binding.app_poi_code,
                &binding.access_token,
                &row.external_order_id,
                logistics_status,
            )
            .await
    }

    /// 退款应答（同意/驳回；执行在渠道侧）。
    ///
    /// # Errors
    /// 未绑定 / 渠道未启用 / 渠道返回失败 → 业务错误。
    pub async fn respond_refund(
        &self,
        id: i32,
        refund_id: &str,
        agree: bool,
        reason: &str,
    ) -> Result<()> {
        let row = self.load_order_checked(id).await?;
        let (cfg, adapter, binding) = self.action_context(&row).await?;
        adapter
            .respond_refund(
                &cfg,
                &binding.app_poi_code,
                &binding.access_token,
                &row.external_order_id,
                refund_id,
                agree,
                reason,
            )
            .await
    }

    /// 重新处理（补绑/补映射后补单）。
    ///
    /// # Errors
    /// 见接入服务 `reprocess`。
    pub async fn reprocess(&self, id: i32) -> Result<()> {
        let _ = self.load_order_checked(id).await?;
        self.ingest.reprocess(id).await
    }

    /// 出站动作的共同上下文：生效配置 + 适配器 + 该单绑定。
    async fn action_context(
        &self,
        row: &ExternalOrderRow,
    ) -> Result<(
        nextrade_core::models::takeout::ChannelConfig,
        Arc<dyn super::channel::OrderChannelAdapter>,
        nextrade_infrastructure::db::repo::takeout::ChannelBindingRow,
    )> {
        if row.binding_id <= 0 {
            return Err(Error::biz_msg("外部单尚未绑定门店"));
        }
        let binding = self
            .repos
            .takeout
            .binding
            .find_by_id(row.binding_id)
            .await?
            .ok_or_else(|| Error::biz_msg(ERR_BINDING_NOT_FOUND))?;
        let settings = self.ingest.settings().await?;
        let cfg = settings
            .channel(&row.channel)
            .cloned()
            .ok_or_else(|| Error::biz_msg("未知渠道"))?;
        if !cfg.active() {
            return Err(Error::biz_msg("渠道未启用或未配置凭据"));
        }
        let adapter = adapter_for(&row.channel)
            .ok_or_else(|| Error::biz_msg("渠道未接入"))?;
        Ok((cfg, adapter, binding))
    }
}
