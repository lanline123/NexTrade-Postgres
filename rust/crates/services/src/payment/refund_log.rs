//! 退款记录域服务（payment::refund_log）。
//!
//! 对应原 Java `RefundLogServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/finance/impl/RefundLogServiceImpl.java`，
//! 7 个管理端端点）。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - **`refundId` 映射**：Java 实体属性是 `refundApplyId`，而 `RefundLogCreateDTO`
//!   属性是 `refundId`，`BeanUtils.copyProperties` 因属性名不匹配**不会**拷贝，
//!   即 Java 的 create 实际会忽略入参 `refundId`；`RefundLogVO.refundId` 同理
//!   在 list/detail 中恒为 `null`。Rust 侧按**意图**映射：`refundId` →
//!   `refund_apply_id`，VO 的 `refundId` 回填外键值（差异已记录）。
//! - **`detail` 空记录**：Java 对不存在的 id 会在 `BeanUtils.copyProperties(null, vo)`
//!   处 NPE（500）；本实现返回业务 500「退款记录不存在」。
//! - **`detail` 不装配 `refund`**：与 Java 一致（Java detail 只 copy 主表字段）。
//! - **list 的 `refund` 装配**：Java 逐条 `selectById`（N+1），本实现批量 IN 查询；
//!   输出 JSON 形状一致。若 `refund_apply` 缺失，Java 会在
//!   `BeanUtils.copyProperties(null, ...)` 处 NPE；本实现降级为 `refund = null`
//!   （容错，见 `uncertain_points`）。
//! - **`updateField`**：本表没有 `GENERAL_FIELDS`（`is_show/sort_order/status`）
//!   中的任何列，因此任何字段名都会返回「field 的值有误」，与 Java 行为一致。
//! - **`batch`**：仅 `type = "del"` 执行 `removeByIds`，其余静默返回 `false`
//!   （控制器为 `void`）。
//! - **权限**：Java 该控制器**没有** `@PreAuthorize`（契约 `auth=jwt`），
//!   只要登录即可访问，Rust 侧由 `require_auth` 覆盖，无需额外处理。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::{Aftersales, RefundApply, RefundLog};
use nextrade_core::models::payment::refund_log::{
    format_unix_time, refund_type_text, resolve_update_field, AftersalesVO, BatchDTO, OperateDTO,
    RefundLogCreateDTO, RefundLogListQuery, RefundLogRefundVO, RefundLogUpdateDTO, RefundLogVO,
    UpdateFieldDTO, MSG_FIELD_NAME_REQUIRED, MSG_FIELD_VALUE_REQUIRED, MSG_ID_REQUIRED,
    MSG_INVALID_FIELD_VALUE, MSG_NOT_EXISTS, MSG_OPERATION_TYPE_REQUIRED, MSG_UNSELECTED_ITEM,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::refund_log::RefundLogRepository;
use nextrade_infrastructure::db::Repositories;

/// 退款记录域服务。
pub struct RefundLogService {
    repos: Arc<Repositories>,
}

impl RefundLogService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.payment.refund_log`）。
    #[must_use]
    pub fn repo(&self) -> &RefundLogRepository {
        &self.repos.payment.refund_log
    }

    // ————————————————————— 查询 —————————————————————

    /// 分页列表（Java `RefundLogServiceImpl#list`）。
    pub async fn list(&self, q: &RefundLogListQuery) -> Result<PageResult<RefundLogVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, limit, offset).await?;
        if rows.is_empty() {
            // Java：`PageUtil.convertPage(refundLogPage, refundLogVOList)`（空页 total 照传）
            return Ok(PageResult::empty(&page));
        }

        // 批量装配 refund（Java 逐条 selectById）
        let apply_ids = distinct(rows.iter().map(|r| r.refund_apply_id));
        let applies = self.repo().find_refund_applies(&apply_ids).await?;
        let apply_map: HashMap<i32, RefundApply> =
            applies.into_iter().map(|a| (a.refund_id, a)).collect();
        let aftersale_ids = distinct(apply_map.values().map(|a| a.aftersale_id));
        let aftersales = self.repo().find_aftersales(&aftersale_ids).await?;
        let aftersale_map: HashMap<i32, Aftersales> =
            aftersales.into_iter().map(|a| (a.aftersale_id, a)).collect();

        let list: Vec<RefundLogVO> = rows
            .iter()
            .map(|row| {
                let mut vo = to_vo(row);
                // Java：setAddTime(TigUtils.handelTime(...)) —— 0 → ""
                vo.add_time = Some(format_unix_time(row.add_time));
                vo.refund_type_text = refund_type_text(row.refund_type).map(str::to_string);
                vo.refund = apply_map.get(&row.refund_apply_id).map(|apply| {
                    build_refund_vo(apply, aftersale_map.get(&apply.aftersale_id))
                });
                vo
            })
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 详情（Java `RefundLogServiceImpl#detail`：只 copy 主表字段）。
    pub async fn detail(&self, id: i32) -> Result<RefundLogVO> {
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        Ok(to_vo(&row))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 创建（Java `RefundLogServiceImpl#create`：copy + `addTime = now` + save）。
    pub async fn create(&self, dto: &RefundLogCreateDTO) -> Result<i32> {
        let entity = RefundLog {
            log_id: 0,
            refund_apply_id: dto.refund_id.unwrap_or(0),
            refund_type: dto.refund_type.unwrap_or_default(),
            refund_pay_code: dto.refund_pay_code.clone().unwrap_or_default(),
            transaction_id: dto.transaction_id.clone().unwrap_or_default(),
            refund_amount: dto.refund_amount.unwrap_or_default(),
            // Java：`refundLog.setAddTime(StringUtils.getCurrentTime())`（秒级）
            add_time: now_seconds_i32(),
            description: dto.description.clone(),
            user_id: dto.user_id.unwrap_or(0),
            order_id: dto.order_id.unwrap_or(0),
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建退款记录: id={id}");
        Ok(id)
    }

    /// 更新（Java `RefundLogServiceImpl#update`；MP `updateById` 忽略 null 字段，
    /// 本实现先读旧值再合并，等价 NOT_NULL 语义）。
    pub async fn update(&self, dto: &RefundLogUpdateDTO) -> Result<()> {
        let id = dto
            .log_id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let mut entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;

        if let Some(v) = dto.refund_id {
            entity.refund_apply_id = v;
        }
        if let Some(v) = dto.refund_type {
            entity.refund_type = v;
        }
        if let Some(v) = dto.refund_pay_code.clone() {
            entity.refund_pay_code = v;
        }
        if let Some(v) = dto.transaction_id.clone() {
            entity.transaction_id = v;
        }
        if let Some(v) = dto.refund_amount {
            entity.refund_amount = v;
        }
        if let Some(v) = dto.add_time {
            entity.add_time = v;
        }
        if let Some(v) = dto.description.clone() {
            entity.description = Some(v);
        }
        if let Some(v) = dto.user_id {
            entity.user_id = v;
        }
        if let Some(v) = dto.order_id {
            entity.order_id = v;
        }

        self.repo().update(&entity).await?;
        tracing::info!("更新退款记录: id={id}");
        Ok(())
    }

    /// 删除（Java `del(id)` → `removeById`；id 为空静默成功）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        if let Some(id) = dto.id {
            self.repo().delete(id).await?;
            tracing::info!("删除退款记录: id={id}");
        }
        Ok(())
    }

    /// 字段更新（Java `updateField(dto, GENERAL_FIELDS)`）。
    ///
    /// 本表没有白名单列 ⇒ 任何字段名都会报「field 的值有误」。
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
            tracing::info!("批量删除退款记录: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }
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

/// 实体 → VO（Java `BeanUtils.copyProperties(refundLog, vo)`；list 再补派生字段）。
///
/// `refund_id` 由外键 `refund_apply_id` 回填（Java 因属性名不匹配为 null，
/// 见模块头偏差说明）。
#[must_use]
pub fn to_vo(row: &RefundLog) -> RefundLogVO {
    RefundLogVO {
        log_id: row.log_id,
        refund_id: row.refund_apply_id,
        refund_type: row.refund_type,
        refund_type_text: None,
        refund_pay_code: row.refund_pay_code.clone(),
        transaction_id: row.transaction_id.clone(),
        refund_amount: row.refund_amount,
        add_time: None,
        description: row.description.clone(),
        user_id: row.user_id,
        order_id: row.order_id,
        salesman_id: None,
        refund: None,
    }
}

/// 退款申请 + 售后单 → 嵌套 `refund` 对象（Java `RefundApplyVO` 的 JSON 形状）。
#[must_use]
pub fn build_refund_vo(apply: &RefundApply, aftersales: Option<&Aftersales>) -> RefundLogRefundVO {
    RefundLogRefundVO {
        refund_id: apply.refund_id,
        refund_type: apply.refund_type,
        order_id: apply.order_id,
        user_id: apply.user_id,
        aftersale_id: apply.aftersale_id,
        refund_status: apply.refund_status,
        refund_note: apply.refund_note.clone(),
        online_balance: apply.online_balance,
        offline_balance: apply.offline_balance,
        refund_balance: apply.refund_balance,
        is_online: apply.is_online,
        is_offline: apply.is_offline,
        payment_voucher: apply.payment_voucher.clone(),
        is_receive: apply.is_receive,
        paylog_refund_id: apply.paylog_refund_id,
        shop_id: apply.shop_id,
        // Java `BeanUtils` 类型不符未拷贝 / 未调用 assembleData ⇒ null
        add_time: None,
        refund_type_name: None,
        refund_status_name: None,
        aftersales: aftersales.map(aftersales_to_vo),
        is_credit_order: None,
        credit_amount: None,
        credit_refund_amount: None,
    }
}

/// 售后单实体 → VO。
#[must_use]
pub fn aftersales_to_vo(a: &Aftersales) -> AftersalesVO {
    AftersalesVO {
        aftersale_id: a.aftersale_id,
        aftersale_type: a.aftersale_type,
        status: a.status,
        pics: a.pics.clone(),
        description: a.description.clone(),
        reply: a.reply.clone(),
        add_time: a.add_time,
        tracking_no: a.tracking_no.clone(),
        logistics_name: a.logistics_name.clone(),
        return_address: a.return_address.clone(),
        aftersale_reason: a.aftersale_reason.clone(),
        aftersales_sn: a.aftersales_sn.clone(),
        order_id: a.order_id,
        user_id: a.user_id,
        refund_amount: a.refund_amount,
        shop_id: a.shop_id,
        audit_time: a.audit_time,
        deal_time: a.deal_time,
        final_time: a.final_time,
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
    use rust_decimal::Decimal;

    fn refund_log() -> RefundLog {
        RefundLog {
            log_id: 9,
            refund_apply_id: 3,
            refund_type: 2,
            refund_pay_code: "balance".into(),
            transaction_id: "T-1".into(),
            refund_amount: Decimal::new(1299, 2),
            add_time: 1_700_000_000,
            description: Some("余额退回".into()),
            user_id: 5,
            order_id: 6,
        }
    }

    #[test]
    fn detail_vo_keeps_java_null_fields_null() {
        let vo = to_vo(&refund_log());
        // 主表字段照抄
        assert_eq!(vo.log_id, 9);
        assert_eq!(vo.refund_id, 3); // 外键回填（偏差见模块头）
        assert_eq!(vo.refund_amount, Decimal::new(1299, 2));
        assert_eq!(vo.refund_pay_code, "balance");
        // detail Java 不设置的三项保持 null
        assert!(vo.add_time.is_none());
        assert!(vo.refund_type_text.is_none());
        assert!(vo.refund.is_none());
        assert!(vo.salesman_id.is_none());
    }

    #[test]
    fn list_derives_time_and_type_text() {
        let mut vo = to_vo(&refund_log());
        vo.add_time = Some(format_unix_time(1_700_000_000));
        vo.refund_type_text = refund_type_text(2).map(str::to_string);
        assert_eq!(vo.add_time.as_deref(), Some("2023-11-15 06:13:20"));
        assert_eq!(vo.refund_type_text.as_deref(), Some("余额退回"));
        // add_time=0 → "" （Java handelTime 语义，非 null）
        assert_eq!(format_unix_time(0), "");
    }

    #[test]
    fn refund_vo_carries_apply_and_aftersales() {
        let apply = RefundApply {
            refund_id: 3,
            refund_type: 1,
            order_id: 6,
            user_id: 5,
            aftersale_id: 77,
            refund_status: 2,
            online_balance: Decimal::new(500, 2),
            offline_balance: Decimal::new(799, 2),
            refund_balance: Decimal::new(1299, 2),
            ..Default::default()
        };
        let aftersales = Aftersales {
            aftersale_id: 77,
            aftersales_sn: Some("SH202412010001".into()),
            order_id: Some(6),
            refund_amount: Decimal::new(1299, 2),
            ..Default::default()
        };
        let vo = build_refund_vo(&apply, Some(&aftersales));
        assert_eq!(vo.refund_id, 3);
        assert_eq!(vo.refund_balance, Decimal::new(1299, 2));
        // Java 未调用 assembleData / BeanUtils 类型不符 ⇒ 这些仍是 null
        assert!(vo.refund_type_name.is_none());
        assert!(vo.add_time.is_none());
        let a = vo.aftersales.expect("aftersales 应装配");
        assert_eq!(a.aftersales_sn.as_deref(), Some("SH202412010001"));
        assert_eq!(a.refund_amount, Decimal::new(1299, 2));
        // 没有售后单时为 null
        assert!(build_refund_vo(&apply, None).aftersales.is_none());
    }

    #[test]
    fn page_query_conversion_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(3), Some(500));
        assert_eq!((p.page, p.page_size), (3, 100));
        // page/size 为 0 时按 Java getPage/getSize 兜底
        let p = to_page_query(Some(0), Some(0));
        assert_eq!((p.page, p.page_size), (1, 1));
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
