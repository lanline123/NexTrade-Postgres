//! 电子卡券组域服务：列表 / 详情 / 创建 / 更新 / 删除 / 字段更新 / 批量操作
//! / Excel 批量导入（未迁移，明确报错）。
//! 对应原 Java `ECardGroupServiceImpl`
//! （`com.nextrade.service.product.impl`，管理端 8 个端点背后的方法）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：`keyword` 对 `group_name` 做 `ILIKE`（`isNotEmpty`，不 trim）+
//!   **恒定** `shop_id = getShopId()` + 三参 `buildSortOrder`（不读
//!   `sortItems`）；`isDownload > 0` 时 Java 向响应写 xlsx 模板流。
//! - `detail`：`getById`；不存在时 Java 在 `BeanUtils.copyProperties(null)`
//!   处抛异常（500），本实现返回「电子卡券组不存在」。
//! - `create`：名称**全局**唯一（`checkCardGroupUnique` **不带 shop_id**）
//!   + `addTime`/`upTime` = now + `shopId = getShopId()` → `save`。
//! - `update`：先 `getCount(groupId)`（等价 `find_by_id`）判存在，不存在抛
//!   「电子卡券组不存在」；再 `copyProperties` + `upTime = now` →
//!   `updateById`（`groupName` 必填，`remark` 未传则不写，`shopId`/`isUse`
//!   不在 DTO 里 ⇒ 保留原值）。**没有**唯一性校验（Java 原样）。
//! - `del`：被**上架商品**（`product_status = 1 AND is_delete = 0`）绑定过的
//!   分组不能删（「商品表中有关联该分组的上架商品…」）→ `removeById`。
//! - `updateField`：`field` 字面量等于 `"isUse"` 时先做同样的占用校验，
//!   再走 `BaseServiceImpl#updateField`（白名单 `{"is_use"}`）。
//! - `batch`：仅 `type == "del"` 时 `removeByIds`（**不**做占用校验）。
//! - `import`：Java 解析 xlsx 批量插 `e_card`；Rust 侧已用 `calamine` 实现。
//!
//! # 未迁移 / 偏差
//!
//! - `isDownload > 0` 的模板导出：Java 直接写 `HttpServletResponse` 的 xlsx
//!   二进制流；本实现返回明确业务错误 [`EXCEL_TEMPLATE_UNMIGRATED_MSG`]，
//!   不伪造二进制响应。
//! - `list?isDownload>0` 模板导出仍未接 xlsx 响应流。
//! - 删除日志已通过 `AdminLogService::record_log` 写入 `admin_log`（actor/IP 由路由层注入）。
//! - Java `Assert`/`GlobalException` → 业务 500；注解校验 → `Error::Validation`
//!   （HTTP 400），与既有域一致。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::catalog::e_card_group::{
    group_to_vo, is_allowed_update_field, map_field_column, BatchDTO, ECardGroupCreateDTO,
    ECardGroupListQuery, ECardGroupUpdateDTO, ECardGroupVO, UpdateFieldDTO,
    EXCEL_TEMPLATE_UNMIGRATED_MSG, FIELD_MAX_CHARS,
    FIELD_NAME_REQUIRED, FIELD_VALUE_REQUIRED, FILE_REQUIRED, GROUP_BOUND_PRODUCT_EXISTS,
    GROUP_ID_REQUIRED, GROUP_NAME_EXISTS, GROUP_NAME_OVER_LENGTH, GROUP_NAME_REQUIRED,
    GROUP_NOT_EXISTS, ID_REQUIRED, INVALID_FIELD_VALUE, OPERATION_TYPE_REQUIRED, UNSELECTED_ITEM,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::e_card_group::ECardGroupInsert;
use nextrade_infrastructure::db::Repositories;

/// 电子卡券组域服务。
pub struct ECardGroupService {
    repos: Arc<Repositories>,
}

impl ECardGroupService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 分组分页列表（对应 Java `ECardGroupServiceImpl#list`）。
    ///
    /// `isDownload > 0` 走模板导出分支 —— 未迁移，返回明确业务错误。
    pub async fn list(
        &self,
        q: &ECardGroupListQuery,
        shop_id: i32,
    ) -> Result<PageResult<ECardGroupVO>> {
        if q.wants_download_template() {
            return Err(Error::biz_msg(EXCEL_TEMPLATE_UNMIGRATED_MSG));
        }
        let page = q.page_query();
        let (total, rows) = self
            .repos.catalog.e_card_group
            .paginate(q, shop_id, &page)
            .await?;
        let list: Vec<ECardGroupVO> = rows.iter().map(group_to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 分组详情（对应 Java `ECardGroupServiceImpl#detail`）。
    pub async fn detail(&self, group_id: i32) -> Result<ECardGroupVO> {
        let group = self
            .repos.catalog.e_card_group
            .find_by_id(group_id)
            .await?
            .ok_or_else(|| Error::biz(500, GROUP_NOT_EXISTS))?;
        Ok(group_to_vo(&group))
    }

    // ————————————————————— 写操作 —————————————————————

    /// 创建分组（对应 Java `ECardGroupServiceImpl#create`），返回自增主键。
    pub async fn create(&self, dto: &ECardGroupCreateDTO, shop_id: i32) -> Result<i32> {
        let group_name = validate_group_name(dto.group_name.as_deref())?;
        // Java `checkCardGroupUnique`：同名分组数量 > 0 ⇒ 已存在（全局，不分店铺）
        if self
            .repos.catalog.e_card_group
            .count_by_group_name(group_name)
            .await?
            > 0
        {
            return Err(Error::biz(500, GROUP_NAME_EXISTS));
        }
        let now = now_seconds() as i32;
        let row = ECardGroupInsert {
            group_name: group_name.to_string(),
            shop_id,
            remark: dto.remark.clone(),
            // Java DTO 无 isUse 字段 ⇒ 走列默认值 0，这里显式写 0，等价
            is_use: 0,
            add_time: now,
            up_time: now,
        };
        let id = self.repos.catalog.e_card_group.insert(&row).await?;
        tracing::info!("创建电子卡券组成功: id={id}, shop_id={shop_id}");
        Ok(id)
    }

    /// 更新分组（对应 Java `ECardGroupServiceImpl#update`）。
    pub async fn update(&self, dto: &ECardGroupUpdateDTO) -> Result<()> {
        let (group_id, group_name) = validate_update(dto)?;
        // Java `getCount(groupId) == 0` ⇒ 不存在
        self.repos.catalog.e_card_group
            .find_by_id(group_id)
            .await?
            .ok_or_else(|| Error::biz(500, GROUP_NOT_EXISTS))?;
        let affected = self
            .repos.catalog.e_card_group
            .update(
                group_id,
                group_name,
                dto.remark.as_deref(),
                now_seconds() as i32,
            )
            .await?;
        tracing::info!("更新电子卡券组: id={group_id}, affected={affected}");
        Ok(())
    }

    /// 删除分组（对应 Java `ECardGroupServiceImpl#del`）。
    ///
    /// 被上架商品绑定的分组不可删除（Java 原样）。
    pub async fn delete(&self, group_id: i32, admin_id: i32, client_ip: &str) -> Result<()> {
        if self
            .repos.catalog.e_card_group
            .count_on_sale_products_bound(group_id)
            .await?
            > 0
        {
            return Err(Error::biz(500, GROUP_BOUND_PRODUCT_EXISTS));
        }
        let deleted = self
            .repos.catalog.e_card_group
            .delete(group_id)
            .await?;
        let log_info = format!("删除电子卡券分组信息：id:{group_id}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_log(admin_id, "admin", 0, &log_info, Some(client_ip), None)
            .await
        {
            tracing::warn!("写管理操作日志失败（删除电子卡券组）: {e}");
        }
        tracing::info!("删除电子卡券组: id={group_id}, deleted={deleted}");
        Ok(())
    }

    /// 更新单个字段（对应 Java `BaseServiceImpl#updateField` + 占用前置校验）。
    ///
    /// 顺序与 Java 一致：
    /// 1. 注解校验（id / field / val 必填）
    /// 2. `field == "isUse"`（**字面量**，不是归一后的列名）⇒ 占用校验
    /// 3. `getColumnByPropertyName` + `E_CARD_GROUP_FIELDS` 白名单
    /// 4. `UPDATE e_card_group SET <列> = ?`
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let (group_id, field, val) = require_update_field_parts(dto)?;
        if needs_product_bind_check(field)
            && self
                .repos.catalog.e_card_group
                .count_on_sale_products_bound(group_id)
                .await?
                > 0
        {
            return Err(Error::biz(500, GROUP_BOUND_PRODUCT_EXISTS));
        }
        let column = resolve_update_column(field)?;
        let affected = self
            .repos.catalog.e_card_group
            .update_field(group_id, column, val)
            .await?;
        tracing::info!("更新电子卡券组字段: id={group_id}, field={column}");
        Ok(affected > 0)
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// - `ids` 为空 ⇒「未选择项目」；`type` 缺失 ⇒「操作类型不能为空」
    /// - `type == "del"` ⇒ `removeByIds`（**不**做占用校验）；否则 `false`
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let Some(ids) = batch_delete_ids(dto)? else {
            return Ok(false);
        };
        let n = self
            .repos.catalog.e_card_group
            .delete_batch(ids)
            .await?;
        tracing::info!("批量删除电子卡券组: {n} 条");
        Ok(n > 0)
    }

    /// Excel 批量导入（对应 Java `ECardGroupServiceImpl#batchImport`）。
    ///
    /// Java 流程：`file == null` ⇒ 抛「文件不能为空」；否则解析 xlsx
    /// （列：卡券名称 / 卡号密码）→ 逐条构造 `ECard`（`groupId` 来自参数、
    /// `addTime` = now）→ `saveBatch`。
    pub async fn batch_import(
        &self,
        group_id: Option<i32>,
        file_present: bool,
        file_bytes: Option<&[u8]>,
    ) -> Result<()> {
        let group_id = validate_import_args(group_id, file_present, file_bytes)?;
        let bytes = file_bytes.unwrap();

        let sheet = crate::system::excel::read_xlsx(bytes)
            .map_err(|e| Error::biz(500, format!("Excel 解析失败: {e}")))?;

        let cards = parse_e_card_rows(sheet);

        let now = now_seconds() as i32;
        let count = self
            .repos
            .catalog
            .e_card_group
            .insert_e_cards(group_id, &cards, now)
            .await?;
        tracing::info!("电子卡券批量导入成功: group_id={group_id}, count={count}");
        Ok(())
    }
}

/// 解析电子卡券 Excel 数据行（第一列：卡券名称，第二列：卡号密码，跳过表头）
pub fn parse_e_card_rows(sheet: Vec<Vec<String>>) -> Vec<(String, String)> {
    let mut cards = Vec::new();
    for row in sheet.into_iter().skip(1) {
        let card_number = row.first().map(String::as_str).unwrap_or("").trim();
        let card_pwd = row.get(1).map(String::as_str).unwrap_or("").trim();
        if !card_number.is_empty() {
            cards.push((card_number.to_string(), card_pwd.to_string()));
        }
    }
    cards
}

/// `import` 入参及非空校验
pub fn validate_import_args(
    group_id: Option<i32>,
    file_present: bool,
    file_bytes: Option<&[u8]>,
) -> Result<i32> {
    if !file_present {
        return Err(Error::biz(500, FILE_REQUIRED));
    }
    let group_id = group_id.ok_or_else(|| Error::Validation(GROUP_ID_REQUIRED.into()))?;
    let bytes = file_bytes
        .filter(|b| !b.is_empty())
        .ok_or_else(|| Error::biz(500, FILE_REQUIRED))?;
    let _ = bytes;
    Ok(group_id)
}

/// 分组名称必填 + 长度校验（Java `@NotNull` + `@Size(max = 255)`）。
///
/// ⚠️ 空字符串**合法**（`@NotNull` 不拦空串），不要改成 `is_blank`。
fn validate_group_name(raw: Option<&str>) -> Result<&str> {
    let name = raw.ok_or_else(|| Error::Validation(GROUP_NAME_REQUIRED.into()))?;
    if name.chars().count() > FIELD_MAX_CHARS {
        return Err(Error::Validation(GROUP_NAME_OVER_LENGTH.into()));
    }
    Ok(name)
}

/// 更新入参校验：`groupId` / `groupName` 必填（Java 注解语义）。
fn validate_update(dto: &ECardGroupUpdateDTO) -> Result<(i32, &str)> {
    let group_id = dto
        .group_id
        .ok_or_else(|| Error::Validation(GROUP_ID_REQUIRED.into()))?;
    let group_name = validate_group_name(dto.group_name.as_deref())?;
    Ok((group_id, group_name))
}

/// 提取并校验 `UpdateFieldDTO` 的三要素（Java `@Valid` 注解语义）。
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

/// 是否需要在更新前做「上架商品占用」校验。
///
/// Java 用 `ObjectUtil.equals(updateField.getField(), "isUse")` —— **只认
/// 字面量 `isUse`**，传 `is_use` 会跳过占用校验（随后仍能通过白名单更新）。
fn needs_product_bind_check(field: &str) -> bool {
    field == "isUse"
}

/// 归一 + 白名单校验（对应 Java `getColumnByPropertyName` +
/// `E_CARD_GROUP_FIELDS`）。
fn resolve_update_column(field: &str) -> Result<&'static str> {
    let column = map_field_column(field);
    if is_allowed_update_field(field) {
        Ok(column)
    } else {
        Err(Error::biz(500, INVALID_FIELD_VALUE))
    }
}

/// `batch` 的实际删除目标（`None` ⇒ 不删除、返回 false）。
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

    // ———————————————— 分组名 / 更新入参校验 ————————————————

    #[test]
    fn group_name_required_and_max_255() {
        assert_eq!(validate_group_name(Some("默认")).unwrap(), "默认");
        // 空串合法（@NotNull 不拦空串）
        assert_eq!(validate_group_name(Some("")).unwrap(), "");
        let err = validate_group_name(None).unwrap_err();
        assert!(err.to_string().contains("电子卡券组名称不能为空"), "实际: {err}");
        let ok = "组".repeat(255);
        assert!(validate_group_name(Some(&ok)).is_ok());
        let too_long = "组".repeat(256);
        let err = validate_group_name(Some(&too_long)).unwrap_err();
        assert!(err.to_string().contains("电子卡券组名称长度不能超过50"), "实际: {err}");
    }

    #[test]
    fn update_requires_id_and_name() {
        let dto = ECardGroupUpdateDTO {
            group_id: None,
            group_name: Some("A".into()),
            remark: None,
        };
        assert!(validate_update(&dto)
            .unwrap_err()
            .to_string()
            .contains("电子卡券组id不能为空"));
        let dto = ECardGroupUpdateDTO {
            group_id: Some(1),
            group_name: None,
            remark: None,
        };
        assert!(validate_update(&dto)
            .unwrap_err()
            .to_string()
            .contains("电子卡券组名称不能为空"));
        let dto = ECardGroupUpdateDTO {
            group_id: Some(1),
            group_name: Some("A".into()),
            remark: None,
        };
        assert_eq!(validate_update(&dto).unwrap(), (1, "A"));
    }

    // ———————————————— updateField ————————————————

    #[test]
    fn product_bind_check_only_for_literal_is_use_name() {
        // Java `ObjectUtil.equals(field, "isUse")`：只认驼峰字面量
        assert!(needs_product_bind_check("isUse"));
        assert!(!needs_product_bind_check("is_use"));
        assert!(!needs_product_bind_check("groupName"));
    }

    #[test]
    fn update_field_column_whitelist() {
        assert_eq!(resolve_update_column("isUse").unwrap(), "is_use");
        assert_eq!(resolve_update_column("is_use").unwrap(), "is_use");
        for field in ["groupName", "group_name", "shopId", "remark", "nope"] {
            let err = resolve_update_column(field).unwrap_err();
            assert!(err.to_string().contains("field 的值有误"), "{field}: {err}");
        }
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

    // ———————————————— import 校验与解析 ————————————————

    #[test]
    fn import_args_validation() {
        // 缺文件 ⇒ 「文件不能为空」
        let err = validate_import_args(Some(1), false, None).unwrap_err();
        assert!(err.to_string().contains("文件不能为空"), "实际: {err}");

        let err = validate_import_args(Some(1), true, Some(&[])).unwrap_err();
        assert!(err.to_string().contains("文件不能为空"), "实际: {err}");

        // 缺 groupId
        let err = validate_import_args(None, true, Some(b"dummy")).unwrap_err();
        assert!(err.to_string().contains("电子卡券组id不能为空"), "实际: {err}");

        // 正常
        assert_eq!(validate_import_args(Some(123), true, Some(b"dummy")).unwrap(), 123);
    }

    #[test]
    fn parse_e_card_rows_skips_header_and_empty() {
        let sheet = vec![
            vec!["卡券名称".to_string(), "卡号密码".to_string()],
            vec!["CARD001".to_string(), "PWD123".to_string()],
            vec!["".to_string(), "PWD456".to_string()], // empty card number skipped
            vec!["CARD002".to_string(), "PWD789".to_string()],
        ];
        let cards = parse_e_card_rows(sheet);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0], ("CARD001".to_string(), "PWD123".to_string()));
        assert_eq!(cards[1], ("CARD002".to_string(), "PWD789".to_string()));
    }
}
