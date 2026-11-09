//! 电子卡券域服务：列表 / 详情 / 创建 / 更新 / 删除 / 字段更新 / 批量操作。
//! 对应原 Java `ECardServiceImpl`
//! （`com.nextrade.service.product.impl`，管理端 7 个端点背后的方法）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：`groupId` 必填（`@NotNull`）→ 按 `group_id` 过滤 +
//!   `keyword` 对 `card_number` 做 `LIKE` + `buildSortOrder(BasePage)`
//!   （支持 `sortItems`）；**无店铺过滤**（`e_card` 无 `shop_id` 列）。
//!   记录为空时 Java 返回 `new Page<>()`（total = 0），本实现返回
//!   `PageResult::empty`。
//! - `detail`：Java `new ECardVO(getById(id))` 对不存在 ID 会 NPE（500）；
//!   本实现返回业务 500「电子卡券不存在」（文案取 `ECardConstants`）。
//! - `create`：卡号全局唯一（`checkCardUnique`）+ `ECardSaveParam.createECard`
//!   （`addTime`/`upTime` = now；`isUse` 未传走列默认值 0）→ `save`。
//! - `update`：先查存在（否则「电子卡券不存在」）→ 已被订单占用则
//!   「该卡券已被使用，无法修改!」→ `ECardEditParam.createECard`
//!   （`addTime` 置空 ⇒ **不更新** `add_time`）→ `updateById`。
//! - `del`：id 为 null 时 Java 返回 false（但控制器 `@Validated` 已挡）；
//!   存在性 NPE 在 Java 侧仍是 500，本实现用「电子卡券不存在」；
//!   被订单占用抛「该卡券已被使用，无法修改!」→ `removeById`。
//! - `updateField`：先查存在 + 占用校验，再按
//!   `CheckFieldConstants.E_CARD_GROUP_FIELDS = {"is_use"}` 白名单校验，
//!   最后 `UPDATE e_card SET <列> = ?`。
//! - `batch`：`BaseServiceImpl#batch` —— 仅 `type == "del"` 时 `removeByIds`
//!   （**不做**占用校验，Java 原样），否则 false。
//!
//! # 未迁移（如实标注，不伪造实现）
//!
//! - `adminLogService.createByLogInfo(...)`：管理操作日志域尚未迁移到 Rust
//!   侧（`del` / `updateField` 中调用），本实现跳过并保留 `tracing` 记录。
//! - `getCardByOrderItemId` / `getCardByOrderId` / `getNewCardByGroupId`
//!   是订单/商城端使用的内部方法，**不在**管理端 7 个端点内，本切片不实现。
//!
//! # 偏差
//!
//! - Java `Assert.isTrue/notNull` 抛 `GlobalException`（业务 500），本实现用
//!   `Error::biz(500, ...)` 保持相同业务码与文案。
//! - `@Validated` 注解类校验（必填/长度）统一映射为 `Error::Validation`
//!   （HTTP 400），与既有域一致。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::catalog::e_card::{
    card_to_vo, is_allowed_update_field, map_field_column, BatchDTO, ECardEditParam, ECardPageQuery,
    ECardSaveParam, ECardVO, UpdateFieldDTO, CARD_ID_REQUIRED, CARD_NOT_EXISTS,
    CARD_NUMBER_EXISTS, CARD_NUMBER_OVER_LENGTH, CARD_NUMBER_REQUIRED, CARD_PWD_OVER_LENGTH,
    CARD_PWD_REQUIRED, CARD_USED_BY_ORDER, FIELD_MAX_CHARS, FIELD_NAME_REQUIRED,
    FIELD_VALUE_REQUIRED, GROUP_ID_REQUIRED, ID_REQUIRED, INVALID_FIELD_VALUE,
    OPERATION_TYPE_REQUIRED, UNSELECTED_ITEM,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::e_card::{ECardInsert, ECardUpdate};
use nextrade_infrastructure::db::Repositories;

/// 电子卡券域服务。
pub struct ECardService {
    repos: Arc<Repositories>,
}

impl ECardService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 电子卡券分页列表（对应 Java `ECardServiceImpl#list`）。
    pub async fn list(&self, q: &ECardPageQuery) -> Result<PageResult<ECardVO>> {
        let group_id = validate_group_id(q.group_id)?;
        let page = q.page_query();
        let (total, rows) = self
            .repos.catalog.e_card
            .paginate(q, group_id, &page)
            .await?;
        // Java：`if (CollUtil.isEmpty(eCardRecords)) return new Page<>();`
        // 空页的 total 归零（不是查到但越界时的真实 total）
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list: Vec<ECardVO> = rows.iter().map(card_to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 电子卡券详情（对应 Java `ECardServiceImpl#detail`）。
    pub async fn detail(&self, card_id: i32) -> Result<ECardVO> {
        let card = self
            .repos.catalog.e_card
            .find_by_id(card_id)
            .await?
            .ok_or_else(|| Error::biz(500, CARD_NOT_EXISTS))?;
        Ok(card_to_vo(&card))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 创建电子卡券（对应 Java `ECardServiceImpl#create`），返回自增主键。
    ///
    /// Java 控制器返回 `void`；本实现返回主键便于日志/调试（路由层丢弃）。
    pub async fn create(&self, param: &ECardSaveParam) -> Result<i32> {
        let card = validate_save(
            param.card_number.as_deref(),
            param.card_pwd.as_deref(),
            param.group_id,
        )?;
        // Java `checkCardUnique`：卡号全局唯一
        if self
            .repos.catalog.e_card
            .count_by_card_number(&card.card_number)
            .await?
            > 0
        {
            return Err(Error::biz(500, CARD_NUMBER_EXISTS));
        }
        let now = now_seconds() as i32;
        let row = ECardInsert {
            group_id: card.group_id,
            card_number: card.card_number,
            card_pwd: card.card_pwd,
            // Java NOT_NULL：未传 isUse 时由列默认值 0 兜底
            is_use: param.is_use.unwrap_or(0),
            add_time: now,
            up_time: now,
        };
        let id = self.repos.catalog.e_card.insert(&row).await?;
        tracing::info!("创建电子卡券成功: id={id}, group_id={}", row.group_id);
        Ok(id)
    }

    /// 更新电子卡券（对应 Java `ECardServiceImpl#update`）。
    pub async fn update(&self, param: &ECardEditParam) -> Result<()> {
        let card_id = param
            .card_id
            .ok_or_else(|| Error::Validation(CARD_ID_REQUIRED.into()))?;
        let card = validate_save(
            param.card_number.as_deref(),
            param.card_pwd.as_deref(),
            param.group_id,
        )?;

        // Java `Assert.notNull(eCard, CARD_NOT_EXISTS)`
        let existing = self
            .repos.catalog.e_card
            .find_by_id(card_id)
            .await?
            .ok_or_else(|| Error::biz(500, CARD_NOT_EXISTS))?;
        assert_not_used(existing.order_id)?;

        // Java `ECardEditParam.createECard`：addTime = null ⇒ 不写 add_time
        let patch = ECardUpdate {
            group_id: card.group_id,
            card_number: card.card_number,
            card_pwd: card.card_pwd,
            is_use: param.is_use,
            up_time: now_seconds() as i32,
        };
        let affected = self
            .repos.catalog.e_card
            .update(card_id, &patch)
            .await?;
        tracing::info!("更新电子卡券: id={card_id}, affected={affected}");
        Ok(())
    }

    /// 删除电子卡券（对应 Java `ECardServiceImpl#del`）。
    ///
    /// 已被订单占用的卡券不可删除（Java 原样）。
    pub async fn delete(&self, card_id: i32) -> Result<()> {
        // Java `getById` 为 null 时在 `getOrderId()` 处 NPE ⇒ 500；
        // 本实现返回「电子卡券不存在」（文案取 ECardConstants）
        let existing = self
            .repos.catalog.e_card
            .find_by_id(card_id)
            .await?
            .ok_or_else(|| Error::biz(500, CARD_NOT_EXISTS))?;
        assert_not_used(existing.order_id)?;
        let deleted = self.repos.catalog.e_card.delete(card_id).await?;
        let log_info = format!("删除电子卡券信息：id:{card_id}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("删除电子卡券: id={card_id}, deleted={deleted}");
        Ok(())
    }

    /// 更新单个字段（对应 Java `ECardServiceImpl#updateField`）。
    ///
    /// 顺序与 Java 一致：查存在 → 占用校验 → 字段归一 + 白名单 → `UPDATE`。
    /// 返回 `update` 的布尔结果（Java 返回 boolean）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let (card_id, field, val) = require_update_field_parts(dto)?;
        let existing = self
            .repos.catalog.e_card
            .find_by_id(card_id)
            .await?
            .ok_or_else(|| Error::biz(500, CARD_NOT_EXISTS))?;
        assert_not_used(existing.order_id)?;
        let column = resolve_update_column(field)?;
        let affected = self
            .repos.catalog.e_card
            .update_field(card_id, column, val)
            .await?;
        let log_info = format!("更新电子卡券信息：id:{card_id}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("更新电子卡券字段: id={card_id}, field={column}");
        Ok(affected > 0)
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// - `ids` 为空 ⇒「未选择项目」；`type` 缺失 ⇒「操作类型不能为空」
    /// - `type == "del"` ⇒ `removeByIds`（**不做**占用校验）；否则 `false`
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let Some(ids) = batch_delete_ids(dto)? else {
            return Ok(false);
        };
        let n = self.repos.catalog.e_card.delete_batch(ids).await?;
        tracing::info!("批量删除电子卡券: {n} 条");
        Ok(n > 0)
    }
}

/// `validate_save` 通过后的必填字段。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedCardSave {
    card_number: String,
    card_pwd: String,
    group_id: i32,
}

/// 校验 `ECardSaveParam` 的三个必填字段（Java `@Validated` 注解语义）。
///
/// - `cardNumber`：非 null（`@NotNull`）+ 字符数 ≤ 255（`@Size(max=255)`）
/// - `cardPwd`：同上
/// - `groupId`：非 null
///
/// ⚠️ 空字符串**合法**（Java `@NotNull` 不拦空串、`@Size(max)` 也允许空串），
/// 与「去空白」式的校验不同，不要擅自改成 `is_blank` 判断。
fn validate_save(
    card_number: Option<&str>,
    card_pwd: Option<&str>,
    group_id: Option<i32>,
) -> Result<ValidatedCardSave> {
    let card_number = card_number.ok_or_else(|| Error::Validation(CARD_NUMBER_REQUIRED.into()))?;
    if card_number.chars().count() > FIELD_MAX_CHARS {
        return Err(Error::Validation(CARD_NUMBER_OVER_LENGTH.into()));
    }
    let card_pwd = card_pwd.ok_or_else(|| Error::Validation(CARD_PWD_REQUIRED.into()))?;
    if card_pwd.chars().count() > FIELD_MAX_CHARS {
        return Err(Error::Validation(CARD_PWD_OVER_LENGTH.into()));
    }
    let group_id = validate_group_id(group_id)?;
    Ok(ValidatedCardSave {
        card_number: card_number.to_string(),
        card_pwd: card_pwd.to_string(),
        group_id,
    })
}

/// `groupId` 必填校验（`@NotNull(message = "电子卡券分组id不能为空")`）。
fn validate_group_id(group_id: Option<i32>) -> Result<i32> {
    group_id.ok_or_else(|| Error::Validation(GROUP_ID_REQUIRED.into()))
}

/// Java `Assert.isFalse(eCard.getOrderId() > 0, CARD_USED_BY_ORDER)`。
fn assert_not_used(order_id: i32) -> Result<()> {
    if order_id > 0 {
        return Err(Error::biz(500, CARD_USED_BY_ORDER));
    }
    Ok(())
}

/// 提取并校验 `UpdateFieldDTO` 的三要素（Java `@Validated` 注解语义）。
fn require_update_field_parts(dto: &UpdateFieldDTO) -> Result<(i32, &str, &str)> {
    let id = dto.id.ok_or_else(|| Error::Validation(ID_REQUIRED.into()))?;
    let field = dto
        .field
        .as_deref()
        .ok_or_else(|| Error::Validation(FIELD_NAME_REQUIRED.into()))?;
    let val = dto
        .val
        .as_deref()
        .ok_or_else(|| Error::Validation(FIELD_VALUE_REQUIRED.into()))?;
    Ok((id, field, val))
}

/// 归一 + 白名单校验（对应 Java `getColumnByPropertyName` + `E_CARD_GROUP_FIELDS`）。
///
/// 只有 `isUse` / `is_use` 归一成 `is_use` 后能命中白名单；其它字段
/// （即使归一成真实列）一律「field 的值有误」。
fn resolve_update_column(field: &str) -> Result<&'static str> {
    let column = map_field_column(field);
    if is_allowed_update_field(field) {
        Ok(column)
    } else {
        Err(Error::biz(500, INVALID_FIELD_VALUE))
    }
}

/// `batch` 的实际删除目标（`None` ⇒ 不删除、返回 false）。
///
/// 校验顺序与 Java 注解一致：`ids @NotEmpty` → `type @NotNull`；
/// `removeByIds` 对空集合返回 false。
fn batch_delete_ids(dto: &BatchDTO) -> Result<Option<&[i32]>> {
    let ids = dto.ids.as_deref().unwrap_or_default();
    if ids.is_empty() {
        return Err(Error::Validation(UNSELECTED_ITEM.into()));
    }
    let op_type = dto
        .r#type
        .as_deref()
        .ok_or_else(|| Error::Validation(OPERATION_TYPE_REQUIRED.into()))?;
    if op_type != "del" {
        return Ok(None);
    }
    Ok(Some(ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ———————————————— 必填校验（对应 Java 注解） ————————————————

    #[test]
    fn save_requires_card_number_and_pwd_and_group() {
        let ok = validate_save(Some("SN"), Some("PW"), Some(2)).unwrap();
        assert_eq!(ok.card_number, "SN");
        assert_eq!(ok.group_id, 2);

        let err = validate_save(None, Some("PW"), Some(2)).unwrap_err();
        assert!(err.to_string().contains("电子卡券名称不能为空"), "实际: {err}");

        let err = validate_save(Some("SN"), None, Some(2)).unwrap_err();
        assert!(err.to_string().contains("电子卡券密码不能为空"), "实际: {err}");

        let err = validate_save(Some("SN"), Some("PW"), None).unwrap_err();
        assert!(err.to_string().contains("电子卡券分组id不能为空"), "实际: {err}");
    }

    #[test]
    fn empty_strings_pass_but_over_255_chars_fail() {
        // Java @NotNull 不拦空串；@Size(max=255) 允许空串
        assert!(validate_save(Some(""), Some(""), Some(1)).is_ok());
        // 255 字符合法、256 非法（message 原文写 50，但真实上限 255）
        let ok = "a".repeat(255);
        assert!(validate_save(Some(&ok), Some("PW"), Some(1)).is_ok());
        let too_long = "a".repeat(256);
        let err = validate_save(Some(&too_long), Some("PW"), Some(1)).unwrap_err();
        assert!(err.to_string().contains("电子卡券名称长度不能超过50"), "实际: {err}");
        let err = validate_save(Some("SN"), Some(&too_long), Some(1)).unwrap_err();
        assert!(err.to_string().contains("电子卡券密码长度不能超过50"), "实际: {err}");
    }

    #[test]
    fn group_id_is_required_for_list() {
        assert_eq!(validate_group_id(Some(3)).unwrap(), 3);
        let err = validate_group_id(None).unwrap_err();
        assert!(err.to_string().contains("电子卡券分组id不能为空"));
    }

    // ———————————————— 占用校验 ————————————————

    #[test]
    fn used_card_is_rejected() {
        assert!(assert_not_used(0).is_ok());
        let err = assert_not_used(9).unwrap_err();
        assert!(err.to_string().contains("该卡券已被使用，无法修改!"), "实际: {err}");
    }

    // ———————————————— updateField ————————————————

    #[test]
    fn update_field_column_whitelist() {
        assert_eq!(resolve_update_column("isUse").unwrap(), "is_use");
        assert_eq!(resolve_update_column("is_use").unwrap(), "is_use");
        for field in ["cardNumber", "card_number", "groupId", "orderId", "nope"] {
            let err = resolve_update_column(field).unwrap_err();
            assert!(err.to_string().contains("field 的值有误"), "{field}: {err}");
        }
    }

    #[test]
    fn update_field_requires_all_parts() {
        let dto = UpdateFieldDTO {
            id: None,
            field: Some("isUse".into()),
            val: Some("1".into()),
        };
        assert!(require_update_field_parts(&dto)
            .unwrap_err()
            .to_string()
            .contains("id不能为空"));
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: None,
            val: Some("1".into()),
        };
        assert!(require_update_field_parts(&dto)
            .unwrap_err()
            .to_string()
            .contains("字段名不能为空"));
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("isUse".into()),
            val: None,
        };
        assert!(require_update_field_parts(&dto)
            .unwrap_err()
            .to_string()
            .contains("值不能为空"));
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("isUse".into()),
            val: Some("1".into()),
        };
        assert_eq!(require_update_field_parts(&dto).unwrap(), (1, "isUse", "1"));
    }

    // ———————————————— batch ————————————————

    #[test]
    fn batch_validation_and_branching() {
        let empty = BatchDTO {
            ids: None,
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert!(batch_delete_ids(&empty)
            .unwrap_err()
            .to_string()
            .contains("未选择项目"));

        let no_type = BatchDTO {
            ids: Some(vec![1]),
            r#type: None,
            ..Default::default()
        };
        assert!(batch_delete_ids(&no_type)
            .unwrap_err()
            .to_string()
            .contains("操作类型不能为空"));

        let other = BatchDTO {
            ids: Some(vec![1]),
            r#type: Some("update".into()),
            ..Default::default()
        };
        assert_eq!(batch_delete_ids(&other).unwrap(), None);

        let del = BatchDTO {
            ids: Some(vec![1, 2]),
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert_eq!(batch_delete_ids(&del).unwrap(), Some(&[1, 2][..]));
    }
}
