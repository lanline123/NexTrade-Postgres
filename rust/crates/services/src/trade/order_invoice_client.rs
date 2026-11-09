//! 前台用户订单发票域服务（trade::order_invoice_client）。
//!
//! 对应 Java `OrderInvoiceServiceImpl#clientDetail/clientCreate/clientUpdate`。
//!
//! # 逐端点口径
//!
//! - `detail`：id 必填；按 id + 当前会员查 `order_invoice`，回填订单快照
//!   （订单号/金额/收货地址）；`regionNames`、`invoiceTypeName`、`statusName`
//!   等展示名依赖字典/翻译域，未迁移 → 空数组/空字符串（不编造）。
//! - `create`：校验「单位/个人名称不能为空」+ 订单归属（「订单不存在」）+
//!   重复申请（「该订单已申请发票」）→ 落库 `status = 0` 待审核。
//! - `update`：id 必填；仅 `status = 0` 可修改（否则「发票状态不允许修改」）。
//! - **未迁移**：电子发票开具/红冲/状态同步（第三方发票平台对接）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::trade::order_invoice_client::{
    validate_create, validate_edit, OrderInvoiceDetailVO, OrderInvoiceEditParam,
    OrderInvoiceSaveParam, INVOICE_STATUS_PENDING, MSG_ID_REQUIRED, MSG_INVOICE_ALREADY_APPLIED,
    MSG_INVOICE_NOT_EXIST, MSG_INVOICE_STATUS_INVALID, MSG_ORDER_NOT_EXIST,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::order_invoice_client::{
    InvoiceRow, InvoiceUpdatePatch, OrderInvoiceClientRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 前台订单发票服务。
pub struct OrderInvoiceClientService {
    repos: Arc<Repositories>,
}

impl OrderInvoiceClientService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`self.repos.trade.order_invoice_client`）。
    #[must_use]
    pub fn repo(&self) -> &OrderInvoiceClientRepository {
        &self.repos.trade.order_invoice_client
    }

    /// 发票详情（Java `clientDetail`）。
    pub async fn detail(
        &self,
        user_id: i32,
        id: Option<i32>,
    ) -> Result<Option<OrderInvoiceDetailVO>> {
        let id = id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let repo = self.repo();
        let Some(invoice) = repo.find_invoice(id, user_id).await? else {
            return Ok(None);
        };
        let order = repo
            .find_order_brief(invoice.order_id, user_id)
            .await?;
        Ok(Some(build_detail_vo(&invoice, order.as_ref())))
    }

    /// 发票申请（Java `clientCreate`）。
    pub async fn create(&self, user_id: i32, param: &OrderInvoiceSaveParam) -> Result<()> {
        validate_create(param).map_err(Error::Validation)?;
        let order_id = param
            .order_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz_msg(MSG_ORDER_NOT_EXIST))?;
        let repo = self.repo();
        let Some(order) = repo.find_order_brief(order_id, user_id).await? else {
            return Err(Error::biz_msg(MSG_ORDER_NOT_EXIST));
        };
        if repo.exists_invoice_for_order(user_id, order_id).await? {
            return Err(Error::biz_msg(MSG_INVOICE_ALREADY_APPLIED));
        }
        let amount = param
            .amount
            .unwrap_or(order.total_amount);
        repo.insert_invoice(
            user_id,
            order_id,
            param.invoice_type.unwrap_or(0),
            param.title_type.unwrap_or(0),
            param.company_name.as_deref().unwrap_or(""),
            param.company_code.as_deref(),
            param.company_address.as_deref(),
            param.company_phone.as_deref(),
            param.company_bank.as_deref(),
            param.company_account.as_deref(),
            param.invoice_content.as_deref(),
            amount,
            param.mobile.as_deref(),
            param.email.as_deref(),
            param
                .add_time
                .map(|v| i32::try_from(v).unwrap_or(i32::MAX))
                .unwrap_or_else(now_i32),
        )
        .await?;
        Ok(())
    }

    /// 修改发票申请（Java `clientUpdate`，仅待审核可改）。
    pub async fn update(&self, user_id: i32, param: &OrderInvoiceEditParam) -> Result<()> {
        let id = validate_edit(param).map_err(Error::Validation)?;
        let repo = self.repo();
        let Some(invoice) = repo.find_invoice(id, user_id).await? else {
            return Err(Error::biz_msg(MSG_INVOICE_NOT_EXIST));
        };
        if invoice.status != INVOICE_STATUS_PENDING {
            return Err(Error::biz_msg(MSG_INVOICE_STATUS_INVALID));
        }
        let patch = InvoiceUpdatePatch {
            invoice_type: param.invoice_type,
            title_type: param.title_type,
            company_code: param.company_code.clone(),
            company_name: param.company_name.clone(),
            company_address: param.company_address.clone(),
            company_phone: param.company_phone.clone(),
            company_bank: param.company_bank.clone(),
            company_account: param.company_account.clone(),
            invoice_content: param.invoice_content.clone(),
            mobile: param.mobile.clone(),
            email: param.email.clone(),
            amount: param.amount,
        };
        repo.update_invoice(id, &patch).await?;
        Ok(())
    }

    /// 删除发票申请（b2b2b 5.8.57 H5 前端新增，Java `OrderInvoiceClientController`
    /// 无 `del` 端点；`order_invoice` 表无 `is_del` 列 → 物理删除）。
    ///
    /// 仅删除**当前会员**的申请（`id + user_id` 双条件）；不存在时按
    /// Java 通用口径返回「发票不存在」业务错误。
    /// 仓储文件由并行任务持有（禁改），此处经共享连接池直查。
    pub async fn del(&self, user_id: i32, id: i32) -> Result<()> {
        let repo = self.repo();
        if repo.find_invoice(id, user_id).await?.is_none() {
            return Err(Error::biz_msg(MSG_INVOICE_NOT_EXIST));
        }
        let res = sqlx::query("DELETE FROM order_invoice WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(Error::biz_msg(MSG_INVOICE_NOT_EXIST));
        }
        Ok(())
    }
}

/// 发票行 + 订单快照 → 详情 VO（展示名/地区名未迁移 → 空）。
fn build_detail_vo(
    invoice: &InvoiceRow,
    order: Option<&nextrade_infrastructure::db::repo::trade::order_invoice_client::OrderBrief>,
) -> OrderInvoiceDetailVO {
    let attachment = invoice
        .invoice_attachment
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(raw).ok())
        .unwrap_or_default();
    OrderInvoiceDetailVO {
        id: invoice.id,
        user_id: invoice.user_id,
        order_id: invoice.order_id,
        address: order.map(|o| o.address.clone()),
        amount: Some(invoice.amount),
        apply_reply: Some(invoice.apply_reply.clone()),
        company_account: Some(invoice.company_account.clone()),
        company_address: Some(invoice.company_address.clone()),
        company_bank: Some(invoice.company_bank.clone()),
        company_code: Some(invoice.company_code.clone()),
        company_name: Some(invoice.company_name.clone()),
        company_phone: Some(invoice.company_phone.clone()),
        email: Some(invoice.email.clone()),
        invoice_content: Some(invoice.invoice_content.clone()),
        invoice_no: Some(invoice.invoice_no.clone()),
        invoice_type: invoice.invoice_type,
        invoice_type_name: None,
        mobile: Some(invoice.mobile.clone()),
        order_sn: order.map(|o| o.order_sn.clone()),
        region_names: Vec::new(),
        status: invoice.status,
        status_name: None,
        title_type: invoice.title_type,
        title_type_name: None,
        total_amount: order.map(|o| o.total_amount),
        user_address: order.map(|o| format!("{} {}", o.consignee, o.mobile)),
        invoice_attachment: attachment,
    }
}

fn now_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_status_guard_constant() {
        assert_eq!(INVOICE_STATUS_PENDING, 0);
        assert!(MSG_INVOICE_STATUS_INVALID.contains("不允许"));
    }

    #[test]
    fn detail_vo_handles_missing_order() {
        let invoice = InvoiceRow::default();
        let vo = build_detail_vo(&invoice, None);
        assert!(vo.order_sn.is_none());
        assert!(vo.region_names.is_empty());
    }
}
