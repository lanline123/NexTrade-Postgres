//! 发票资质域服务（payment::user_invoice）。
//!
//! 对应原 Java `UserInvoiceServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/finance/impl/UserInvoiceServiceImpl.java`，
//! 7 个**管理端**端点：list / detail / create / update / del / updateField / batch）。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - **只实现管理端方法**：Java `UserInvoiceService` 还有
//!   `clientCreate / clientUpdate / getStatus / clientDetail`（C 端 `/api` 发票资质，
//!   另一批端点）以及 `createAdminMsg` 站内信、`ConfigService` 开关
//!   `INVOICE_ADDED` 校验 —— 均**不在**本 feature 的 7 个 admin 端点内，未迁移。
//! - **`order.invoice_data` 快照**：Java 的管理端 `UserInvoiceServiceImpl#create/update`
//!   只维护 `user_invoice` 主表（`BeanUtils.copyProperties` + `save/updateById`），
//!   **不**回写 `order.invoice_data`（那是 `OrderInvoiceServiceImpl#update` 的行为，
//!   属 `payment::order_invoice` 范围）。因此本实现无需订单域仓储方法，无此偏差。
//! - **`@Sensitive(PHONE)`**：`companyPhone` / `mobile` 仅在演示站
//!   （`nextrade.is-demo: 1`，见 `java/nextrade-adminapi/src/main/resources/application.yaml`）
//!   序列化时脱敏。Rust 侧无配置服务，用常量 [`IS_DEMO`]（= `0`，逐字对齐该 yaml）
//!   承载；脱敏使用 `nextrade_common::utils::mask_mobile`。
//! - **`detail` 空记录**：Java 对不存在的 id 会在 `BeanUtils.copyProperties(null, vo)`
//!   处 NPE（500）；本实现返回业务 500「发票资质不存在」。
//! - **`update` 空记录**：Java `updateById` 影响 0 行、控制器 `void` ⇒ 静默成功；
//!   本实现同样静默成功（不报 404）。
//! - **`updateField`**：白名单 `GENERAL_FIELDS`，本表只有 `status` 命中；空
//!   id/field/val 用 Java 注解文案做防御性校验（控制器无 `@Valid`，Java 实际
//!   可能落到「field 的值有误」或 NPE，本实现统一 400）。
//! - **`batch`**：仅 `type = "del"` 执行 `removeByIds`，其余静默 `false`（控制器 void）。
//! - **权限**：Java `update/del/batch` 有 `@PreAuthorize`（`userInvoice*Manage`），
//!   但权限矩阵未迁移，Rust 侧只校验登录态（与既有 payment 切片一致）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::{mask_mobile, now_seconds};
use nextrade_core::entities::UserInvoice;
use nextrade_core::models::payment::user_invoice::{
    format_unix_time, resolve_update_field, status_name, BatchDTO, OperateDTO, UpdateFieldDTO,
    UserInvoiceCreateDTO, UserInvoiceListQuery, UserInvoiceUpdateDTO, UserInvoiceVO,
    MSG_FIELD_NAME_REQUIRED, MSG_FIELD_VALUE_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_INVOICE_ID_REQUIRED, MSG_NOT_EXISTS, MSG_OPERATION_TYPE_REQUIRED, MSG_UNSELECTED_ITEM,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::user_invoice::UserInvoiceRepository;
use nextrade_infrastructure::db::Repositories;

/// 演示站开关（Java `nextrade.is-demo`，默认 `0`；见
/// `java/nextrade-adminapi/src/main/resources/application.yaml`）。
///
/// 仅演示站对 `@Sensitive(type = PHONE)` 字段（`companyPhone` / `mobile`）脱敏。
pub const IS_DEMO: i32 = 0;

/// 发票资质域服务。
pub struct UserInvoiceService {
    repos: Arc<Repositories>,
}

impl UserInvoiceService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.payment.user_invoice`）。
    #[must_use]
    pub fn repo(&self) -> &UserInvoiceRepository {
        &self.repos.payment.user_invoice
    }

    // ————————————————————— 查询 —————————————————————

    /// 分页列表（Java `UserInvoiceServiceImpl#list`）。
    pub async fn list(&self, q: &UserInvoiceListQuery) -> Result<PageResult<UserInvoiceVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        // 批量装配 username（Java `selectBatchIds`）
        let user_ids = distinct(rows.iter().map(|r| r.user_id));
        let mut usernames: HashMap<i32, String> = HashMap::new();
        for (id, name) in self.repo().find_usernames(&user_ids).await? {
            usernames.insert(id, name);
        }

        let list: Vec<UserInvoiceVO> = rows
            .iter()
            .map(|row| {
                let mut vo = to_vo(row);
                vo.username = usernames.get(&row.user_id).cloned();
                // Java：setAddTime(TigUtils.handelTime(...))
                vo.add_time = Some(format_unix_time(Some(row.add_time)));
                // Java：setStatusName(STATUS_NAME.get(status)) —— 未命中为 null
                vo.status_name = status_name(row.status).map(str::to_string);
                vo
            })
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 详情（Java `UserInvoiceServiceImpl#detail`：copy + 补 `username`）。
    ///
    /// Java 不格式化 `addTime`、不设置 `statusName`（VO 类型与实体不符 ⇒ null）。
    pub async fn detail(&self, id: i32) -> Result<UserInvoiceVO> {
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
        Ok(vo)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 创建（Java `UserInvoiceServiceImpl#create`：copy + `addTime = now` + save）。
    pub async fn create(&self, dto: &UserInvoiceCreateDTO) -> Result<i32> {
        let entity = UserInvoice {
            invoice_id: 0,
            user_id: dto.user_id.unwrap_or(0),
            status: dto.status.unwrap_or_default(),
            apply_reply: dto.apply_reply.clone().unwrap_or_default(),
            // Java：`userInvoice.setAddTime(StringUtils.getCurrentTime())`（秒级）
            add_time: now_seconds_i32(),
            title_type: dto.title_type.unwrap_or_default(),
            company_name: dto.company_name.clone().unwrap_or_default(),
            company_code: dto.company_code.clone().unwrap_or_default(),
            company_address: dto.company_address.clone().unwrap_or_default(),
            company_phone: dto.company_phone.clone().unwrap_or_default(),
            company_bank: dto.company_bank.clone().unwrap_or_default(),
            company_account: dto.company_account.clone().unwrap_or_default(),
            mobile: dto.mobile.clone().unwrap_or_default(),
            email: dto.email.clone().unwrap_or_default(),
            invoice_content: dto.invoice_content.clone().unwrap_or_default(),
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建发票资质: id={id}");
        Ok(id)
    }

    /// 更新（Java `UserInvoiceServiceImpl#update`；MP `updateById` 忽略 null 字段，
    /// 本实现先读旧值再合并，等价 NOT_NULL 语义；记录不存在时静默成功）。
    pub async fn update(&self, dto: &UserInvoiceUpdateDTO) -> Result<()> {
        let id = require_invoice_id(dto)?;
        let Some(mut entity) = self.repo().find_by_id(id).await? else {
            // Java `updateById` 影响 0 行，控制器 void ⇒ 静默成功
            tracing::info!("更新发票资质: id={id} 不存在，跳过（Java updateById 0 行）");
            return Ok(());
        };

        if let Some(v) = dto.user_id {
            entity.user_id = v;
        }
        if let Some(v) = dto.status {
            entity.status = v;
        }
        if let Some(v) = dto.apply_reply.clone() {
            entity.apply_reply = v;
        }
        if let Some(v) = dto.title_type {
            entity.title_type = v;
        }
        if let Some(v) = dto.company_name.clone() {
            entity.company_name = v;
        }
        if let Some(v) = dto.company_code.clone() {
            entity.company_code = v;
        }
        if let Some(v) = dto.company_address.clone() {
            entity.company_address = v;
        }
        if let Some(v) = dto.company_phone.clone() {
            entity.company_phone = v;
        }
        if let Some(v) = dto.company_bank.clone() {
            entity.company_bank = v;
        }
        if let Some(v) = dto.company_account.clone() {
            entity.company_account = v;
        }
        if let Some(v) = dto.mobile.clone() {
            entity.mobile = v;
        }
        if let Some(v) = dto.email.clone() {
            entity.email = v;
        }
        if let Some(v) = dto.invoice_content.clone() {
            entity.invoice_content = v;
        }

        self.repo().update(&entity).await?;
        tracing::info!("更新发票资质: id={id}");
        Ok(())
    }

    /// 删除（Java `del(id)` → `removeById`；id 为空静默成功）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        if let Some(id) = dto.id {
            self.repo().delete(id).await?;
            tracing::info!("删除发票资质: id={id}");
        }
        Ok(())
    }

    /// 字段更新（Java `updateField(dto, GENERAL_FIELDS)`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
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
        let (ids, op_type) = validate_batch(dto)?;
        if op_type == "del" {
            let n = self.repo().delete_batch(&ids).await?;
            tracing::info!("批量删除发票资质: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }
}

/// 更新入参的主键校验（Java `UserInvoiceUpdateDTO.invoiceId` 的 `@NotNull` 文案）。
fn require_invoice_id(dto: &UserInvoiceUpdateDTO) -> Result<i32> {
    dto.invoice_id
        .ok_or_else(|| Error::Validation(MSG_INVOICE_ID_REQUIRED.into()))
}

/// 批量入参校验（从 `batch` 抽出，便于单测；文案逐字对齐 Java 注解）。
fn validate_batch(dto: &BatchDTO) -> Result<(Vec<i32>, String)> {
    let ids = dto.ids.clone().unwrap_or_default();
    if ids.is_empty() {
        return Err(Error::Validation(MSG_UNSELECTED_ITEM.into()));
    }
    let op_type = dto
        .r#type
        .clone()
        .ok_or_else(|| Error::Validation(MSG_OPERATION_TYPE_REQUIRED.into()))?;
    Ok((ids, op_type))
}

/// 演示站手机号脱敏（Java `@Sensitive(type = PHONE)`）：
/// 仅 `is_demo == 1` 时套 `mask_mobile`，否则原样返回。
#[must_use]
pub fn visible_phone(phone: &str, is_demo: i32) -> String {
    if is_demo == 1 {
        mask_mobile(phone)
    } else {
        phone.to_string()
    }
}

/// 实体 → VO（Java `BeanUtils.copyProperties(UserInvoice, VO)`）。
///
/// list 再补 `username` / `addTime` / `statusName`；detail 只补 `username`。
#[must_use]
pub fn to_vo(row: &UserInvoice) -> UserInvoiceVO {
    UserInvoiceVO {
        invoice_id: row.invoice_id,
        user_id: row.user_id,
        username: None,
        status: row.status,
        status_name: None,
        apply_reply: row.apply_reply.clone(),
        add_time: None,
        title_type: row.title_type,
        company_name: row.company_name.clone(),
        company_code: row.company_code.clone(),
        company_address: row.company_address.clone(),
        company_phone: visible_phone(&row.company_phone, IS_DEMO),
        company_bank: row.company_bank.clone(),
        company_account: row.company_account.clone(),
        mobile: visible_phone(&row.mobile, IS_DEMO),
        email: row.email.clone(),
        invoice_content: row.invoice_content.clone(),
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

/// 当前秒级时间戳（Java `StringUtils.getCurrentTime()`）。
fn now_seconds_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(0)
}

/// `BasePage` → `PageQuery`（`page` 缺省 1；`size` 缺省 15、>`100` 截 100）。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> PageQuery {
    PageQuery {
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
    use nextrade_core::models::payment::user_invoice::{STATUS_APPROVED, STATUS_AUDIT};

    fn invoice() -> UserInvoice {
        UserInvoice {
            invoice_id: 7,
            user_id: 3,
            status: STATUS_AUDIT,
            apply_reply: String::new(),
            add_time: 1_700_000_000,
            title_type: 2,
            company_name: "NexTrade".into(),
            company_code: "91360000".into(),
            company_address: "南昌".into(),
            company_phone: "13812348000".into(),
            company_bank: "工行".into(),
            company_account: "6222".into(),
            mobile: "13812348000".into(),
            email: "a@b.com".into(),
            invoice_content: "商品明细".into(),
        }
    }

    #[test]
    fn detail_vo_keeps_java_null_fields_null() {
        let vo = to_vo(&invoice());
        assert_eq!(vo.invoice_id, 7);
        assert_eq!(vo.company_name, "NexTrade");
        // detail Java 不设置
        assert!(vo.username.is_none());
        assert!(vo.add_time.is_none());
        assert!(vo.status_name.is_none());
    }

    #[test]
    fn list_derives_time_and_status_name() {
        let mut vo = to_vo(&invoice());
        vo.add_time = Some(format_unix_time(Some(1_700_000_000)));
        vo.status_name = status_name(STATUS_AUDIT).map(str::to_string);
        assert_eq!(vo.add_time.as_deref(), Some("2023-11-15 06:13:20"));
        assert_eq!(vo.status_name.as_deref(), Some("待审核"));
        // status=0（未申请）在 Java Map 中不存在 ⇒ null
        assert!(status_name(0).is_none());
        assert_eq!(status_name(STATUS_APPROVED), Some("审核通过"));
    }

    #[test]
    fn sensitive_phone_masks_only_on_demo_site() {
        // 生产/默认（is-demo: 0）不脱敏
        assert_eq!(visible_phone("13812348000", IS_DEMO), "13812348000");
        assert_eq!(visible_phone("13812348000", 0), "13812348000");
        // 演示站脱敏（Java `@Sensitive(PHONE)`）
        assert_eq!(visible_phone("13812348000", 1), "138****8000");
        // 非 11 位手机号走 mask_mobile 的兜底
        assert_eq!(visible_phone("123", 1), "***");
        assert_eq!(visible_phone("", 1), "");
    }

    #[test]
    fn page_query_conversion_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(500));
        assert_eq!((p.page, p.page_size), (2, 100));
        let p = to_page_query(Some(0), Some(0));
        assert_eq!((p.page, p.page_size), (1, 1));
    }

    #[test]
    fn update_requires_verbatim_invoice_id_message() {
        let dto = UserInvoiceUpdateDTO::default();
        assert_eq!(
            require_invoice_id(&dto).unwrap_err().to_string(),
            "参数校验失败: 发票资质id不能为空"
        );
        let dto = UserInvoiceUpdateDTO {
            invoice_id: Some(7),
            ..Default::default()
        };
        assert_eq!(require_invoice_id(&dto).unwrap(), 7);
    }

    #[test]
    fn batch_validation_texts_are_verbatim() {
        let mut dto = BatchDTO::default();
        assert_eq!(
            validate_batch(&dto).unwrap_err().to_string(),
            "参数校验失败: 未选择项目"
        );
        dto.ids = Some(vec![1]);
        assert_eq!(
            validate_batch(&dto).unwrap_err().to_string(),
            "参数校验失败: 操作类型不能为空"
        );
        dto.r#type = Some("del".into());
        let (ids, op) = validate_batch(&dto).unwrap();
        assert_eq!(ids, vec![1]);
        assert_eq!(op, "del");
    }

    #[test]
    fn distinct_keeps_order() {
        assert_eq!(distinct([3, 1, 3, 2].into_iter()), vec![3, 1, 2]);
    }
}
