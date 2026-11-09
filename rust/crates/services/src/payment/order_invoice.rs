//! 发票申请域服务（payment::order_invoice）。
//!
//! 对应原 Java `OrderInvoiceServiceImpl`
//! （`com.nextrade.service.finance.impl`，7 个管理端端点）。
//!
//! # 迁移说明
//!
//! - `update` 中对齐 Java `OrderInvoiceServiceImpl#update`：把发票快照回写 `order.invoice_data`，
//!   并在审核通过（状态=1，STATUS_PASS）时向用户发送站内信通知。
//! - `detail` 对不存在 ID：Java 在 `BeanUtils.copyProperties(null, ...)` 处抛
//!   NPE（500）；本实现返回业务 500「发票申请不存在」。
//! - `create` 忽略入参 `id`（Java `BeanUtils.copyProperties` 会带过去，若前端
//!   传了 ID 会显式插入该主键；管理端正常不传）。
//! - `updateField` 白名单 `GENERAL_FIELDS = {"is_show","sort_order","status"}`，
//!   本表只有 `status` 真实存在；其它字段归一后不在白名单 ⇒「field 的值有误」。
//! - `invoice_attachment` 非法 JSON：Java `JSON.parseArray` 抛异常（500）；
//!   本实现返回空列表（容错），见 `uncertain_points`。
//! - `@Validated` 注解校验 → `Error::Validation`（HTTP 400）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::OrderInvoice;
use nextrade_core::models::payment::order_invoice::{
    attachment_to_json, format_unix_time, invoice_type_name, parse_attachment,
    resolve_update_field, status_name, BatchDTO, OperateDTO, OrderInvoiceEditParam,
    OrderInvoicePageQuery, OrderInvoiceSaveParam, OrderInvoiceVO, UpdateFieldDTO, MSG_FIELD_NAME_REQUIRED,
    MSG_FIELD_VALUE_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_NOT_EXISTS, MSG_OPERATION_TYPE_REQUIRED, MSG_UNSELECTED_ITEM, STATUS_PASS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::order_invoice::OrderInvoiceRepository;
use nextrade_infrastructure::db::Repositories;

/// 发票申请域服务。
pub struct OrderInvoiceService {
    repos: Arc<Repositories>,
}

impl OrderInvoiceService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &OrderInvoiceRepository {
        &self.repos.payment.order_invoice
    }

    // ————————————————————— 查询 —————————————————————

    /// 列表（Java `OrderInvoiceServiceImpl#list`）。
    pub async fn list(
        &self,
        q: &OrderInvoicePageQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<OrderInvoiceVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, shop_scope, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let user_ids = distinct(rows.iter().map(|r| r.user_id));
        let order_ids = distinct(rows.iter().map(|r| r.order_id));
        let mut usernames: HashMap<i32, String> = HashMap::new();
        for (id, name) in self.repo().find_usernames(&user_ids).await? {
            usernames.insert(id, name);
        }
        let mut order_sns: HashMap<i32, String> = HashMap::new();
        for (id, sn) in self.repo().find_order_sns(&order_ids).await? {
            order_sns.insert(id, sn);
        }
        let list: Vec<OrderInvoiceVO> = rows
            .iter()
            .map(|row| {
                let mut vo = to_vo(row);
                vo.username = usernames.get(&row.user_id).cloned();
                vo.order_sn = order_sns.get(&row.order_id).cloned();
                vo.add_time = format_unix_time(Some(row.add_time));
                vo
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 详情（Java `OrderInvoiceServiceImpl#detail`）。
    /// 校验会员归属（create 用）。
    pub async fn ensure_user_scope(
        &self,
        user_id: Option<i32>,
        shop_scope: Option<i32>,
    ) -> Result<()> {
        let Some(shop_id) = shop_scope else {
            return Ok(());
        };
        let user_id = user_id.ok_or_else(|| Error::Validation("会员ID不能为空".into()))?;
        if !self.repo().user_in_shop(user_id, shop_id).await? {
            return Err(Error::Forbidden(format!(
                "无权访问店铺 {shop_id} 的会员 {user_id}"
            )));
        }
        Ok(())
    }

    /// 校验发票申请归属：店铺账号只能访问本店会员的发票。
    pub async fn ensure_scope(&self, id: i32, shop_scope: Option<i32>) -> Result<()> {
        let Some(shop_id) = shop_scope else {
            return Ok(());
        };
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        if !self.repo().user_in_shop(row.user_id, shop_id).await? {
            return Err(Error::Forbidden(format!(
                "无权访问店铺 {shop_id} 的发票申请 {id}"
            )));
        }
        Ok(())
    }

    pub async fn detail(&self, id: i32) -> Result<OrderInvoiceVO> {
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let mut vo = to_vo(&row);
        vo.username = self
            .repo()
            .find_usernames(&[row.user_id])
            .await?
            .into_iter()
            .next()
            .map(|(_, name)| name);
        vo.invoice_attachment = parse_attachment(row.invoice_attachment.as_deref());
        // detail 的 addTime 原样透传（Java 未格式化）
        vo.add_time = if row.add_time == 0 {
            String::new()
        } else {
            row.add_time.to_string()
        };
        Ok(vo)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 创建（Java `OrderInvoiceServiceImpl#create`：`BeanUtils.copyProperties` + save）。
    pub async fn create(&self, param: &OrderInvoiceSaveParam) -> Result<i32> {
        param.validate().map_err(Error::Validation)?;
        let entity = OrderInvoice {
            id: 0,
            user_id: param.user_id.unwrap_or(0),
            order_id: param.order_id.unwrap_or(0),
            invoice_type: param.invoice_type.unwrap_or_default(),
            status: param.status.unwrap_or_default(),
            apply_reply: param.apply_reply.clone().unwrap_or_default(),
            title_type: param.title_type.unwrap_or_default(),
            company_code: param.company_code.clone().unwrap_or_default(),
            company_name: param.company_name.clone().unwrap_or_default(),
            company_address: param.company_address.clone().unwrap_or_default(),
            company_phone: param.company_phone.clone().unwrap_or_default(),
            company_bank: param.company_bank.clone().unwrap_or_default(),
            company_account: param.company_account.clone().unwrap_or_default(),
            invoice_content: param.invoice_content.clone().unwrap_or_default(),
            invoice_no: param.invoice_no.clone().unwrap_or_default(),
            amount: param.amount.unwrap_or_default(),
            mobile: param.mobile.clone().unwrap_or_default(),
            email: param.email.clone().unwrap_or_default(),
            add_time: param
                .add_time
                .and_then(|v| i32::try_from(v).ok())
                .unwrap_or_else(|| now_seconds() as i32),
            invoice_attachment: None,
            invoicing_time: Some(0),
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建发票申请: id={id}");
        Ok(id)
    }

    /// 更新（Java `OrderInvoiceServiceImpl#update`）。
    ///
    /// 先读旧值并按 MyBatis-Plus NOT_NULL 语义合并入参（未传字段保留原值），
    /// 订单存在时写入 `invoice_attachment` JSON + `invoicing_time = now`。
    /// **未迁移**：回写 `order.invoice_data`、开票成功站内信。
    pub async fn update(&self, param: &OrderInvoiceEditParam) -> Result<()> {
        let id = param.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let existing = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let now = now_seconds() as i32;
        let mut entity = existing.clone();
        if let Some(v) = param.user_id {
            entity.user_id = v;
        }
        if let Some(v) = param.order_id {
            entity.order_id = v;
        }
        if let Some(v) = param.invoice_type {
            entity.invoice_type = v;
        }
        if let Some(v) = param.status {
            entity.status = v;
        }
        if let Some(v) = param.apply_reply.clone() {
            entity.apply_reply = v;
        }
        if let Some(v) = param.title_type {
            entity.title_type = v;
        }
        if let Some(v) = param.company_code.clone() {
            entity.company_code = v;
        }
        if let Some(v) = param.company_name.clone() {
            entity.company_name = v;
        }
        if let Some(v) = param.company_address.clone() {
            entity.company_address = v;
        }
        if let Some(v) = param.company_phone.clone() {
            entity.company_phone = v;
        }
        if let Some(v) = param.company_bank.clone() {
            entity.company_bank = v;
        }
        if let Some(v) = param.company_account.clone() {
            entity.company_account = v;
        }
        if let Some(v) = param.invoice_content.clone() {
            entity.invoice_content = v;
        }
        if let Some(v) = param.invoice_no.clone() {
            entity.invoice_no = v;
        }
        if let Some(v) = param.amount {
            entity.amount = v;
        }
        if let Some(v) = param.mobile.clone() {
            entity.mobile = v;
        }
        if let Some(v) = param.email.clone() {
            entity.email = v;
        }
        if let Some(v) = param.add_time {
            entity.add_time = v;
        }
        if let Some(attachments) = param.invoice_attachment.as_deref() {
            // Java：`JSONArray.toJSONString(param.getInvoiceAttachment())`
            entity.invoice_attachment = Some(attachment_to_json(attachments));
            entity.invoicing_time = Some(now);
        }
        self.repo().update(&entity).await?;

        // 对齐 Java OrderInvoiceServiceImpl#update：
        // 1. 若关联订单存在，回写 order.invoice_data
        // 2. 若审核通过（status == 1），发送站内信通知用户
        if entity.order_id > 0 {
            if let Ok(Some(order)) = self.repos.trade.order.find_by_id(entity.order_id).await {
                let invoice_json = serde_json::to_string(&entity).unwrap_or_default();
                let _ = self.repos.trade.order.update_invoice_data(order.order_id, &invoice_json).await;
                if entity.status == STATUS_PASS && order.user_id > 0 {
                    let title = "发票开具成功";
                    let content = format!("您关于订单 {} 的发票已开具完成，请前往查看。", order.order_sn);
                    let _ = self.repos.member.user_message.insert_user_message(
                        order.user_id,
                        title,
                        &content,
                        1,
                        Some(&format!("/order/invoice?id={}", entity.id)),
                    ).await;
                }
            }
        }

        tracing::info!("更新发票申请: id={id}");
        Ok(())
    }

    /// 删除（Java `del(id)` → `removeById`；控制器无 `@Valid`，id 为空静默成功）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        if let Some(id) = dto.id {
            let _ = self.repo().delete(id).await?;
            tracing::info!("删除发票申请: id={id}");
        }
        Ok(())
    }

    /// 字段更新（Java `updateField(dto, GENERAL_FIELDS)`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_NAME_REQUIRED.into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_VALUE_REQUIRED.into()))?;
        let column = resolve_update_field(field)
            .ok_or_else(|| Error::biz(500, MSG_INVALID_FIELD_VALUE))?;
        self.repo().update_field(id, column, val).await?;
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`；控制器无 `@Valid`，本实现补防御性校验）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Err(Error::Validation(MSG_UNSELECTED_ITEM.into()));
        }
        let op_type = dto
            .r#type
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_OPERATION_TYPE_REQUIRED.into()))?;
        if op_type == "del" {
            let n = self.repo().delete_batch(ids).await?;
            tracing::info!("批量删除发票申请: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }
}

/// 实体 → VO（Java `BeanUtils.copyProperties(orderInvoice, VO)` 的对应字段）。
fn to_vo(row: &OrderInvoice) -> OrderInvoiceVO {
    OrderInvoiceVO {
        id: row.id,
        user_id: row.user_id,
        username: None,
        order_id: row.order_id,
        order_sn: None,
        invoice_type: row.invoice_type,
        invoice_type_name: invoice_type_name(row.invoice_type).to_string(),
        status: row.status,
        status_name: status_name(row.status).to_string(),
        apply_reply: row.apply_reply.clone(),
        title_type: row.title_type,
        company_code: row.company_code.clone(),
        company_name: row.company_name.clone(),
        company_address: row.company_address.clone(),
        company_phone: row.company_phone.clone(),
        company_bank: row.company_bank.clone(),
        company_account: row.company_account.clone(),
        invoice_content: row.invoice_content.clone(),
        invoice_no: row.invoice_no.clone(),
        amount: row.amount,
        mobile: row.mobile.clone(),
        email: row.email.clone(),
        order_invoice_add_time: None,
        add_time: format_unix_time(Some(row.add_time)),
        invoice_attachment: None,
        invoice_time: None,
    }
}

/// 去重（保持出现顺序）。
fn distinct(ids: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// `BasePage` → `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    nextrade_core::types::PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::order_invoice::{STATUS_PASS, STATUS_WAIT};

    #[test]
    fn page_query_conversion_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(0));
        assert_eq!((p.page, p.page_size), (2, 1));
    }

    #[test]
    fn distinct_keeps_order() {
        assert_eq!(distinct([3, 1, 3, 2].into_iter()), vec![3, 1, 2]);
    }

    #[test]
    fn vo_maps_status_and_type_names() {
        let row = OrderInvoice {
            id: 7,
            user_id: 1,
            order_id: 2,
            invoice_type: 2,
            status: STATUS_PASS,
            company_name: "NexTrade".into(),
            amount: rust_decimal::Decimal::new(1234, 2),
            add_time: 1_700_000_000,
            ..Default::default()
        };
        let vo = to_vo(&row);
        assert_eq!(vo.status_name, "已开");
        assert_eq!(vo.invoice_type_name, "增值税专用发票");
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.amount, rust_decimal::Decimal::new(1234, 2));
        assert_eq!(vo.company_name, "NexTrade");
    }

    #[test]
    fn update_field_guard_only_allows_status() {
        // 通过纯白名单函数验证（服务层据此报「field 的值有误」）
        assert_eq!(
            nextrade_core::models::payment::order_invoice::resolve_update_field("status"),
            Some("status")
        );
        assert_eq!(
            nextrade_core::models::payment::order_invoice::resolve_update_field("companyName"),
            None
        );
    }

    #[test]
    fn status_constants_distinct() {
        assert_ne!(STATUS_WAIT, STATUS_PASS);
    }
}
