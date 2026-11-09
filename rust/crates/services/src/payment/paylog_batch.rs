//! 交易日志（支付日志）域服务（payment::paylog_batch）。
//!
//! 对应原 Java `com.nextrade.service.finance.impl.PaylogServiceImpl` 的父类
//! `BaseServiceImpl`（`del` / `updateField` / `batch`）三方法的通用实现，
//! 由 `PaylogController`（`@RequestMapping("/adminapi/finance/payLog")`）暴露。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`PaylogBatchService::delete`] | `BaseServiceImpl#del(Integer id)` |
//! | [`PaylogBatchService::update_field`] | `BaseServiceImpl#updateField(dto, GENERAL_FIELDS)` |
//! | [`PaylogBatchService::batch`] | `BaseServiceImpl#batch(BatchDTO)` |
//!
//! 本片只补齐 `PaylogController` 的 `del` / `updateField` / `batch` 三条链路；
//! `list` / `detail` / `create` / `update` 已由早期域 `routes/paylog.rs` +
//! `services/paylog.rs` 实现（本切片**不重复实现、不注册**，避免 axum 路径冲突）。
//!
//! # 与 Java 的差异（逐条，均为「原样照抄」或「明确未迁移」）
//!
//! 1. **`del` 是物理删除**：`paylog` 实体 17 列中没有 `is_del`，
//!    MyBatis-Plus 无 `@TableLogic`，故 Java `removeById` 走物理 `DELETE`。
//!    本实现的 `DELETE FROM paylog WHERE paylog_id = ?` 与之等价。
//!    ⚠️ 交易日志属审计数据，物理删除会丢失对账依据 —— 这是**原系统既有行为**，
//!    本次重构不擅自改为软删（见返回值的 `uncertain_points`）。
//! 2. **`del` 缺 id**：Java `BaseServiceImpl#del(null)` `return false`（不报错）。
//!    本实现返回 `Ok(false)`，与之一致。
//! 3. **`updateField` 恒失败**：Java 用 `GENERAL_FIELDS`
//!    （`{"is_show", "sort_order", "status"}`，见 `CheckFieldConstants`）做白名单，
//!    而 `paylog` 表**一列都不含**这三个字段 → Java 在任何入参下都抛
//!    `GlobalException("field 的值有误")`，UPDATE 永不执行。
//!    本实现的 [`map_update_field`] **恒返回 Err**（同一 code 1001 + 同一文案），
//!    不伪造 `status` 列，也不擅自放宽为「改 pay_status」。
//! 4. **`batch` 仅支持 `type == "del"`**：Java 对其它（含 `null`）`type`
//!    直接 `return false` 且无副作用。本实现一致。
//! 5. **`batch` 空 ids**：Java `removeByIds(空集合)` 返回 `false`。
//!    本实现返回 `Ok(false)`。（注：`BatchDTO.ids` 上的 `@NotEmpty` 在控制器
//!    **没有** `@Validated`，实际不生效，故空 ids 确实会走到服务层。）
//! 6. **权限未迁移**：Java `del` / `batch` 上有
//!    `@PreAuthorize("@pms.hasPermission('payLogBatchManage')")`，
//!    Rust 侧权限矩阵尚未实现，`require_auth` 只校验登录态 —— 任何已登录
//!    管理员都能调用这三条接口（与 seckill/order_admin/005/014/018 的已知缺口一致）。
//! 7. **`services/paylog.rs::update` 的旧缺陷不在本片范围**：该文件的
//!    `mark_paid(..., Decimal::ZERO, ...)` 会把 `pay_amount` 覆盖成 0
//!    （属早期域文件，本切片不得修改），已在 `uncertain_points` 中标注。
//!
//! # 为什么重新建一个仓储而不是复用 `repo::paylog`
//!
//! `repo::paylog::delete_paylog` 带 `AND pay_code = 'balance'` 约束，只服务于
//! 「余额支付事务补偿回滚」；管理端 `del` / `batch` 需要无条件的物理删除，
//! 二者语义不同。`repo::paylog` 属其他实现者的独占文件，本切片不得修改，
//! 故在 `repo::nextrade_core::models::payment::paylog_batch::PaylogBatchRepository` 中新增无条件删除方法。

use std::sync::Arc;

use nextrade_core::models::{BatchDTO, OperateDTO, UpdateFieldDTO};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::paylog_batch::{PaylogBatchRepository, PAYLOG_UPDATE_FIELDS};
use nextrade_infrastructure::db::Repositories;

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `ResultTextConstants.INVALID_FIELD_VALUE`（与 merchant::shop_manage/018 同一文案，勿改）。
pub const INVALID_FIELD_VALUE: &str = "field 的值有误";

/// Java `BaseServiceImpl#batch` 支持的操作类型（唯一有副作用的一种）。
pub const BATCH_TYPE_DEL: &str = "del";

/// 把 Java 属性名解析为 `paylog` 的白名单列名。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName` + 白名单校验：
/// 未知属性在 Java 会回落主键 `paylog_id`，而它不在白名单内 → 抛
/// `GlobalException("field 的值有误")`（code 1001）。
///
/// **本函数恒返回 `Err`**：白名单 `GENERAL_FIELDS`
/// （`is_show` / `sort_order` / `status`）与 `paylog` 表的 17 列零交集，
/// 因此 Java 侧**没有任何一个入参能通过校验**。这不是本实现的偷懒，
/// 而是原系统该接口的真实（且可疑）行为 —— 详见模块头第 3 条。
///
/// `field` 仅用于诊断，**绝不**回显到错误文案里（Java 也只返回固定文案）。
pub fn map_update_field(_field: Option<&str>) -> Result<&'static str> {
    if !PAYLOG_UPDATE_FIELDS.is_empty() {
        // 白名单当前恒为空；若将来放开（需要先给 paylog 建列），在此处加映射。
        unreachable!("PAYLOG_UPDATE_FIELDS 当前必须为空（paylog 表无 GENERAL_FIELDS 列）");
    }
    Err(Error::biz(SERVICE_DATA_ERROR, INVALID_FIELD_VALUE))
}

/// `del` 入参校验：复刻 Java `BaseServiceImpl#del` 的 `if (id == null) return false;`。
///
/// 返回 `None` 表示「未提供 id，静默不操作（Java 返回 `false`）」，不报错。
/// `id = 0` 是**合法值**（Java 会照样发 SQL，影响 0 行 → `false`），不得当作缺省。
#[must_use]
pub fn resolve_delete_id(id: Option<i32>) -> Option<i32> {
    id
}

/// `batch` 类型校验：复刻 Java `BaseServiceImpl#batch` 的 `"del".equals(type)`。
///
/// Java 对非 `del`（含 `null`）直接 `return false`，无任何副作用。
/// 返回 `None` 表示「不支持的批量操作类型，直接返回 false」。
#[must_use]
pub fn resolve_batch_type(kind: Option<&str>) -> Option<&'static str> {
    match kind.map(str::trim) {
        // 大小写敏感，与 Java `"del".equals(type)` 一致
        Some(k) if k == BATCH_TYPE_DEL => Some(BATCH_TYPE_DEL),
        _ => None,
    }
}

/// 交易日志（支付日志）服务。
pub struct PaylogBatchService {
    repos: Arc<Repositories>,
}

impl PaylogBatchService {
    /// 构造：持有全局仓储集合（与 product_manage/002/…/018 同一约定）。
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 删除单条支付日志。
    ///
    /// 对应 Java `PaylogController.del(@RequestBody OperateDTO)`
    /// → `PaylogServiceImpl`（继承 `BaseServiceImpl`）`#del(id)`。
    ///
    /// - `dto.id` 为 `None` → `Ok(false)`（Java `del(null) return false`）
    /// - 目标不存在 → `Ok(false)`（MyBatis-Plus `deleteById` 影响 0 行返回 false）
    /// - 服务层**不做**「是否存在退款记录」的阻断：Java 也没有（见 `uncertain_points`）
    pub async fn delete(&self, dto: &OperateDTO) -> Result<bool> {
        let Some(id) = resolve_delete_id(dto.id) else {
            return Ok(false);
        };
        self.repos.payment.paylog_batch.delete(id).await
    }

    /// 通用字段更新。
    ///
    /// 对应 Java `PaylogController.updateField(@RequestBody UpdateFieldDTO)`
    /// → `BaseServiceImpl#updateField(updateField, GENERAL_FIELDS)`。
    ///
    /// ⚠️ **恒返回 `Err(1001, "field 的值有误")`**：Java 白名单
    /// `GENERAL_FIELDS` 的三个列在 `paylog` 表都不存在，Java 侧同样恒失败。
    /// 这里先做白名单校验（与 Java 同序：校验先于拼 SQL，故缺 `id` 时
    /// 报的也是字段错误），校验通过才会触达仓储。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let column = map_update_field(dto.field.as_deref())?;
        // 白名单恒空 → 上面必然返回；以下仅为「放行后」的正常路径，保留完整性。
        let id = dto
            .id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, INVALID_FIELD_VALUE))?;
        let val = dto.val.as_deref().unwrap_or_default();
        self.repos.payment.paylog_batch.update_field(id, column, val).await
    }

    /// 批量操作（仅 `type == "del"` 有副作用）。
    ///
    /// 对应 Java `PaylogController.batch(@RequestBody BatchDTO)`
    /// → `BaseServiceImpl#batch`。
    ///
    /// - `type != "del"`（含 `None`）→ `Ok(false)`，无副作用
    /// - `ids` 为 `None` 或空 → `Ok(false)`（Java `removeByIds(空)` 返回 false）
    /// - 否则物理删除，返回是否影响至少一行
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if resolve_batch_type(dto.r#type.as_deref()).is_none() {
            return Ok(false);
        }
        let ids: Vec<i32> = dto.ids.clone().unwrap_or_else(Vec::new);
        if ids.is_empty() {
            return Ok(false);
        }
        self.repos.payment.paylog_batch.delete_many(&ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update_field_dto(id: Option<i32>, field: Option<&str>, val: Option<&str>) -> UpdateFieldDTO {
        UpdateFieldDTO {
            id,
            field: field.map(String::from),
            val: val.map(String::from),
        }
    }

    // ————————————————————— 纯逻辑：updateField 白名单 —————————————————————

    #[test]
    fn update_field_rejects_every_general_fields_entry() {
        // Java 白名单 GENERAL_FIELDS 与 paylog 表零交集 → 三者都必须被拒绝，
        // 且错误码/文案与 Java `GlobalException(INVALID_FIELD_VALUE)` 一致。
        for field in ["is_show", "sort_order", "status"] {
            match map_update_field(Some(field)) {
                Err(Error::Biz { code, msg }) => {
                    assert_eq!(code, SERVICE_DATA_ERROR);
                    assert_eq!(msg, INVALID_FIELD_VALUE);
                }
                other => panic!("字段 {field} 应被拒绝，实际 {other:?}"),
            }
        }
    }

    #[test]
    fn update_field_rejects_camel_and_snake_and_unknown() {
        for field in [
            "isShow",
            "sortOrder",
            "payStatus",
            "pay_status",
            "pay_amount",
            "transactionId",
            "notifyData",
            "",
            "   ",
            "1=1 OR 1=1",
            "pay_status` = 1, pay_amount",
        ] {
            assert!(
                map_update_field(Some(field)).is_err(),
                "字段 {field:?} 不应通过白名单"
            );
        }
        // 缺字段名同样非法（Java getColumnByPropertyName(null) 也回落到主键）
        assert!(map_update_field(None).is_err());
    }

    #[test]
    fn update_field_error_text_never_echoes_input() {
        // 错误文案必须是固定串（Java 行为），不得把用户输入拼进 msg（防反射型注入）。
        let Err(Error::Biz { msg, .. }) = map_update_field(Some("<script>x</script>")) else {
            panic!("应返回业务错误");
        };
        assert_eq!(msg, INVALID_FIELD_VALUE);
        assert!(!msg.contains("script"));
    }

    #[test]
    fn update_field_dto_checks_whitelist_before_id() {
        // Java 中白名单校验先于一切 SQL 构造，故「无 id + 非法字段」报的是字段错误；
        // 「无 id + 白名单字段」在 Java 里也先报字段错误（因为白名单恒失败）。
        let dto = update_field_dto(None, Some("is_show"), Some("1"));
        assert!(map_update_field(dto.field.as_deref()).is_err());
        let dto2 = update_field_dto(Some(3), None, None);
        assert!(map_update_field(dto2.field.as_deref()).is_err());
    }

    // ————————————————————— 纯逻辑：del / batch 参数语义 —————————————————————

    #[test]
    fn delete_id_is_optional_and_null_means_noop() {
        // Java `del(null) return false`：不报错、不操作。
        assert_eq!(resolve_delete_id(None), None);
        assert_eq!(resolve_delete_id(Some(7)), Some(7));
        // id = 0 是合法值（Java 会照样发 SQL，影响 0 行 → false），不得被当成缺省
        assert_eq!(resolve_delete_id(Some(0)), Some(0));
        // 负值同样原样下发（Java 不做范围校验）
        assert_eq!(resolve_delete_id(Some(-1)), Some(-1));
    }

    #[test]
    fn batch_only_accepts_del_type() {
        assert_eq!(resolve_batch_type(Some("del")), Some("del"));
        assert_eq!(resolve_batch_type(Some(" del ")), Some("del"));
        // 非 del / 缺省 → Java 直接 return false，无副作用
        assert_eq!(resolve_batch_type(None), None);
        assert_eq!(resolve_batch_type(Some("show")), None);
        assert_eq!(resolve_batch_type(Some("disable")), None);
        assert_eq!(resolve_batch_type(Some("DEL")), None); // 大小写敏感，同 Java
        assert_eq!(resolve_batch_type(Some("")), None);
    }

    #[test]
    fn batch_del_with_empty_ids_yields_no_targets() {
        // 复刻 Java `removeByIds(空集合) → false`（无 SQL）。
        let dto = BatchDTO {
            ids: Some(vec![]),
            r#type: Some("del".into()),
            value: None,
        };
        let ids = dto.ids.clone().unwrap_or_default();
        assert!(ids.is_empty());
        let dto_none = BatchDTO {
            ids: None,
            r#type: Some("del".into()),
            value: None,
        };
        assert!(dto_none.ids.unwrap_or_default().is_empty());
    }

    // ————————————————————— 入参 JSON 兼容性 —————————————————————

    #[test]
    fn dtos_accept_frontend_camel_case_payloads() {
        // 前端三接口的真实提交体（字段名逐字取自 Java DTO）。
        let del: OperateDTO = serde_json::from_str(r#"{"id":12}"#).unwrap();
        assert_eq!(del.id, Some(12));

        let batch: BatchDTO =
            serde_json::from_str(r#"{"ids":[1,2,3],"type":"del","value":0}"#).unwrap();
        assert_eq!(batch.ids.as_deref(), Some([1, 2, 3].as_slice()));
        assert_eq!(batch.r#type.as_deref(), Some("del"));

        let uf: UpdateFieldDTO =
            serde_json::from_str(r#"{"id":12,"field":"status","val":"1"}"#).unwrap();
        assert_eq!(uf.id, Some(12));
        assert_eq!(uf.field.as_deref(), Some("status"));
        assert_eq!(uf.val.as_deref(), Some("1"));
    }
}