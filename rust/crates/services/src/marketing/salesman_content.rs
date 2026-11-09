//! 分销内容域服务（marketing::salesman_content）。
//!
//! 对应原 Java `com.nextrade.service.salesman.impl.SalesmanContentServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/salesman/impl/SalesmanContentServiceImpl.java`）
//! 与继承来的 `BaseServiceImpl#updateField` / `#batch` / `#del`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（10 条：admin 8 + public 2）
//! - `list`（admin）：多租户 `shop_id` + 标题 + 时间范围 + 状态 + 动态排序 + 分页
//! - `list`（public）：Java 控制器强制 `from="user"` / `sortField="is_top"`，
//!   服务按「当前会员 → 分销员 → 店铺」解析归属（游客/非分销员 → `-1` 空列表）
//! - `config`：状态码 → 文案（`ContentStatus`）
//! - `detail`：按主键取详情（Java 详情不返回 `statusText`）
//! - `create`：`@NotBlank content` 校验 + 时间字符串解析 + DB 默认值兜底
//! - `update`：选择性更新（未提交字段保留原值）+ 结束时间联动 `is_available`
//! - `del`：单条删除（`OperateDTO`）
//! - `updateField`：`title` / `is_top` / `is_available` 白名单 + 关闭可用时把
//!   `end_time` 置为当前时间（Java 控制器里的联动逻辑）
//! - `batch`：仅 `type == "del"` 生效（`BaseServiceImpl#batch`）
//!
//! ## 与 Java 的差异（诚实记录）
//! 1. **`update` 的 `pics` 不落库**：Java `SalesmanContentEditParam.pics` 是
//!    `List<String>`，实体是 `String`，`BeanUtils` 静默跳过（与 Java 一致）。
//! 2. **`create` 的 `shop_id` 恒为 0**：Java `SalesmanContentSaveParam` 没有
//!    `shopId`，插入时由 DB 默认值 `0` 兜底（不是取请求头店铺）。
//! 3. **非法时间字符串**：Java 抛异常（500）；本实现视为未提交（保留原值/置 0），
//!    见 models 模块头第 5 条。
//! 4. **`detail` 记录不存在**：Java `BeanUtils.copyProperties(null, ..)` 抛
//!    `IllegalArgumentException`（500）；本实现返回 `Error::NotFound`。
//! 5. **错误码**：Java `GlobalException` 默认 `code = 1001`；本实现沿用项目既有
//!    映射（`Error::biz_msg` → 500），仅保证**消息文案逐字一致**。
//! 6. **后台请求带 `from=user`**：Java 服务端会走 C 端分支，且
//!    `SecurityUtils.getCurrentUserId()` 在后台令牌下取不到会员 ID（返回 `0`），
//!    结果 `shop_id = -1` 空列表；本实现同样处理。

use std::collections::BTreeMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::SalesmanContent;
use nextrade_core::models::marketing::salesman_content::{
    derive_status_text, format_time, normalize_field, parse_time, resolve_sort, status_config,
    validate_save, BatchDTO, OperateDTO, SalesmanContentEditDTO, SalesmanContentFilter,
    SalesmanContentPageQuery, SalesmanContentSaveDTO, SalesmanContentVO, UpdateFieldDTO,
    CONST_NO, CONST_YES, MSG_EDIT_ID_REQUIRED, MSG_FIELD_NAME_REQUIRED, MSG_FIELD_VALUE_REQUIRED,
    MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 分销内容不存在（Java 无对应常量，此文案仅用于 `Error::NotFound`）。
const MSG_CONTENT_NOT_FOUND: &str = "分销内容不存在";

/// 分销内容服务。
pub struct SalesmanContentService {
    repos: Arc<Repositories>,
}

impl SalesmanContentService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.marketing.salesman_content`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::marketing::salesman_content::SalesmanContentRepository
    {
        &self.repos.marketing.salesman_content
    }

    // ————————————————————— 管理端 —————————————————————

    /// 管理端列表（Java `SalesmanContentServiceImpl#list`，`from != "user"` 分支）。
    ///
    /// `shop_id` 来自 Java `HeaderUtils.getShopId()` 三态（`admin` → 0、
    /// `shop`/`pos` → `x-shop-id`、其它 → `None`）；`None` 时 SQL `shop_id = NULL`
    /// 恒不命中（与 Java `eq` 传 `null` 一致）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list_admin(
        &self,
        q: &SalesmanContentPageQuery,
    ) -> Result<PageResult<SalesmanContentVO>> {
        let now = now_seconds() as i32;
        // 多租户：店铺账号强制本店；平台账号按平台级（0）——Java `getShopId()` 同义
        let tenant_shop = TenantScope::current().shop_id().unwrap_or(0);
        // Java 服务按 `from == "user"` 分流；后台令牌下 `getCurrentUserId()` 取不到
        // 会员 ID（返回 0），故命中 C 端分支时解析结果必为「非分销员」→ -1。
        let (from_user, shop_id) = if q.is_client() {
            let resolved = match self.repo().find_salesman_shop_id_by_user_id(0).await? {
                None => Some(-1),
                Some(shop_opt) => shop_opt,
            };
            (true, resolved)
        } else {
            (false, Some(tenant_shop))
        };
        let filter = build_filter(q, shop_id, from_user, now);
        self.paginate(&filter, q).await
    }

    /// 商城端列表（Java public 控制器 `query.setFrom("user")` +
    /// `query.setSortField("is_top")`）。
    ///
    /// 归属解析：`salesman` 表按 `user_id` 取一条 → 用其 `shop_id`；
    /// 非分销员（含游客 `user_id = 0`）→ `-1`（空列表）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list_client(
        &self,
        user_id: i32,
        q: &SalesmanContentPageQuery,
    ) -> Result<PageResult<SalesmanContentVO>> {
        let now = now_seconds() as i32;
        let shop_id = match self.repo().find_salesman_shop_id_by_user_id(user_id).await? {
            None => Some(-1),
            Some(shop_opt) => shop_opt,
        };
        let mut effective = q.clone();
        // Java public 控制器：强制覆盖排序字段（sortOrder 仍取客户端传值）
        effective.sort_field = Some("is_top".to_string());
        let filter = build_filter(&effective, shop_id, true, now);
        self.paginate(&filter, &effective).await
    }

    /// 公共分页（admin / client 共用）。
    async fn paginate(
        &self,
        filter: &SalesmanContentFilter,
        q: &SalesmanContentPageQuery,
    ) -> Result<PageResult<SalesmanContentVO>> {
        let (column, desc) = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let page = q.page_query();
        let (total, rows) = self
            .repo()
            .paginate(filter, column, desc, &page)
            .await?;
        let list = rows
            .iter()
            .map(|e| to_vo(e, Some(filter.now)))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 状态配置（Java `GET /adminapi/salesman/content/config`）。
    ///
    /// Java 返回 `Map<Integer, String>`（`ContentStatus` 全部枚举）；
    /// JSON 键为字符串，故用 `BTreeMap` 保证输出顺序稳定。
    #[must_use]
    pub fn config(&self) -> BTreeMap<i32, String> {
        status_config()
    }

    /// 详情（Java `SalesmanContentServiceImpl#detail`）。
    ///
    /// 与列表不同：Java `detail` **不设置** `statusText`，且只在时间 `> 0` 时格式化。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`（Java `@RequestParam Integer id` 必填）
    /// - 记录不存在 → `NotFound("分销内容不存在")`（Java 为 500，见模块头差异 4）
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<SalesmanContentVO>> {
        let id = id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_CONTENT_NOT_FOUND.into()))?;
        // 多租户：店铺账号只能查看本店内容
        TenantScope::current_ensure_access(row.shop_id)?;
        Ok(Some(to_vo(&row, None)))
    }

    /// 新增（Java `#create`）。
    ///
    /// # Errors
    /// - `content` 为空 → `Validation("分销内容管理内容不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn create(&self, dto: &SalesmanContentSaveDTO) -> Result<i32> {
        validate_save(dto).map_err(Error::Validation)?;
        let mut entity = build_create_entity(dto);
        // 多租户：店铺账号强制本店；平台账号落平台级（0）
        entity.shop_id = TenantScope::current().write_shop_id(None)?;
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建分销内容成功: id={id}");
        Ok(id)
    }

    /// 更新（Java `#update`）。
    ///
    /// 选择性更新：请求体未提交的字段保留库中原值；提交 `endTime` 时按
    /// Java 规则联动 `is_available`（结束时间晚于当前 → 1，否则 → 0）。
    /// `id` 对应记录不存在时 Java 静默影响 0 行，本实现同样 `Ok(())`。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("分销内容管理id不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn update(&self, dto: &SalesmanContentEditDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_EDIT_ID_REQUIRED.into()))?;
        let existing = match self.repo().find_by_id(id).await? {
            Some(row) => row,
            // Java：`updateById` 主键不存在 → 0 行，不报错
            None => return Ok(()),
        };
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(existing.shop_id)?;
        let existing_shop = existing.shop_id;
        let now = now_seconds() as i32;
        let mut merged = merge_update(existing, dto, now);
        // 多租户：店铺账号强制本店；平台账号可指定/保留原店
        merged.shop_id =
            TenantScope::current().write_shop_id(dto.shop_id.or(Some(existing_shop)))?;
        self.repo().update(&merged).await?;
        tracing::info!("更新分销内容成功: id={id}");
        Ok(())
    }

    /// 删除单条（Java `#del` → `BaseServiceImpl#del`）。
    ///
    /// # Errors
    /// - `id` 缺失 → `Validation("id不能为空")`
    /// - 数据库失败 → `Error::Database`
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        // 多租户：删除前校验归属
        if let Some(row) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(row.shop_id)?;
        }
        self.repo().delete(id).await?;
        tracing::info!("删除分销内容: id={id}");
        Ok(())
    }

    /// 更新单个字段（Java `BaseServiceImpl#updateField` + 控制器联动）。
    ///
    /// 白名单 `title` / `is_top` / `is_available`；非白名单 → `field 的值有误`。
    /// 控制器联动：**原始** `field == "isAvailable"` 且 `val == "0"` 时，把
    /// `end_time` 置为当前秒（Java 等于再 `updateById` 一次）。
    ///
    /// # Errors
    /// - `id` / `field` / `val` 缺失 → `Validation`（对应 Java 公共 DTO 注解文案）
    /// - 非白名单字段 → `Validation("field 的值有误")`
    /// - 数据库失败 → `Error::Database`
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let (id, column, raw_field, val) = resolve_update_field(dto)?;
        // 多租户：字段更新前校验归属
        if let Some(row) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(row.shop_id)?;
        }
        self.repo()
            .update_field(id, column, Some(val.as_str()))
            .await?;
        // Java 控制器：`updateField.getField().equals("isAvailable")
        // && updateField.getVal().equals("0")` → 结束时间 = 当前时间。
        // 注意比较的是**原始入参**（传列名 `is_available` 不触发，与 Java 一致）。
        if raw_field == "isAvailable" && val == "0" {
            let now = now_seconds() as i32;
            self.repo().set_end_time(id, now).await?;
            tracing::info!("分销内容置为不可用: id={id}, end_time={now}");
        }
        Ok(())
    }

    /// 批量操作（Java `BaseServiceImpl#batch`）。
    ///
    /// 仅 `type == "del"` 生效；`ids` 为空时 `removeByIds` 返回 `false`
    /// 且不落 SQL（Java 控制器没有 `@Valid`，不会报「未选择项目」）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !is_delete_batch(dto) {
            tracing::debug!("分销内容 batch 非 del 类型或空 ids，忽略: {:?}", dto.r#type);
            return Ok(false);
        }
        // 多租户：批量删除前逐条校验归属
        for id in &dto.ids {
            if let Some(row) = self.repo().find_by_id(*id).await? {
                TenantScope::current_ensure_access(row.shop_id)?;
            }
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        tracing::info!("批量删除分销内容: {} 条", affected);
        Ok(affected > 0)
    }
}

/// 由查询参数 + 账号归属构造仓储过滤条件（Java `list` 的 WHERE 组装）。
fn build_filter(
    q: &SalesmanContentPageQuery,
    shop_id: Option<i32>,
    from_user: bool,
    now: i32,
) -> SalesmanContentFilter {
    SalesmanContentFilter {
        title: q.title.clone(),
        start_time: parse_time_or_date(q.start_time.as_deref()),
        end_time: parse_time_or_date(q.end_time.as_deref()),
        status: q.status,
        from_user,
        shop_id,
        now,
    }
}

/// 列表时间参数解析：Java `TigUtils.toTimestampYmd` 只接受 `yyyy-MM-dd`；
/// 兼容前端偶发传 `yyyy-MM-dd HH:mm:ss`（先按日期解析，失败再按完整时间）。
fn parse_time_or_date(raw: Option<&str>) -> Option<i32> {
    use nextrade_core::models::marketing::salesman_content::parse_date;
    parse_date(raw).or_else(|| parse_time(raw))
}

/// 新增实体（Java `BeanUtils.copyProperties(param, entity)` + 时间解析）。
///
/// 缺省值与 DB 默认值对齐：`title/img/describe = ""`、`start_time = 0`、
/// `end_time = 0`、`is_top = 0`、`is_available = 1`、`shop_id = 0`、`pics = NULL`。
#[must_use]
fn build_create_entity(dto: &SalesmanContentSaveDTO) -> SalesmanContent {
    SalesmanContent {
        id: 0,
        title: dto.title.clone().unwrap_or_default(),
        img: dto.img.clone().unwrap_or_default(),
        start_time: parse_time(dto.start_time.as_deref()).unwrap_or(0),
        // Java 未传 → 由 DB 默认值 0 兜底（不是 NULL）
        end_time: Some(parse_time(dto.end_time.as_deref()).unwrap_or(0)),
        describe: dto.describe.clone().unwrap_or_default(),
        is_top: dto.is_top.unwrap_or(0),
        content: dto.content.clone(),
        // Java 未传 → DB 默认值 1
        is_available: Some(CONST_YES),
        // Java `SalesmanContentSaveParam` 没有 shopId → DB 默认值 0
        shop_id: 0,
        pics: None,
    }
}

/// 合并「选择性更新」并处理结束时间联动（对应 Java `#update`）。
///
/// - 未提交字段保留原值（等价 MyBatis-Plus `updateById` 的 `NOT_NULL` 策略）
/// - 提交 `endTime` 且可解析 → 覆盖 `end_time`，并联动
///   `is_available = (end_time > now ? 1 : 0)`（`Constants.YES/NO`）
/// - `pics` **不更新**（Java `BeanUtils` 类型不匹配静默跳过）
#[must_use]
fn merge_update(existing: SalesmanContent, dto: &SalesmanContentEditDTO, now: i32) -> SalesmanContent {
    let parsed_start = parse_time(dto.start_time.as_deref());
    let parsed_end = parse_time(dto.end_time.as_deref());
    let end_time = match parsed_end {
        Some(end) => Some(end),
        None => existing.end_time,
    };
    let is_available = match parsed_end {
        // Java：提交了 endTime 就按「是否晚于当前时间」重算可用状态
        Some(end) => Some(if end > now { CONST_YES } else { CONST_NO }),
        None => dto.is_available.or(existing.is_available),
    };
    SalesmanContent {
        id: existing.id,
        title: dto.title.clone().unwrap_or(existing.title),
        img: dto.img.clone().unwrap_or(existing.img),
        start_time: parsed_start.unwrap_or(existing.start_time),
        end_time,
        describe: dto.describe.clone().unwrap_or(existing.describe),
        is_top: dto.is_top.unwrap_or(existing.is_top),
        content: dto.content.clone().or(existing.content),
        is_available,
        shop_id: dto.shop_id.unwrap_or(existing.shop_id),
        // Java BeanUtils：`List<String>` → `String` 类型不匹配，跳过
        pics: existing.pics,
    }
}

/// `updateField` 入参归一 + 校验，返回 `(id, 列名, 原始字段名, 值)`。
fn resolve_update_field(dto: &UpdateFieldDTO) -> Result<(i32, &'static str, String, String)> {
    let id = dto
        .id
        .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
    let raw_field = dto
        .field
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation(MSG_FIELD_NAME_REQUIRED.into()))?;
    let val = dto
        .val
        .as_deref()
        .ok_or_else(|| Error::Validation(MSG_FIELD_VALUE_REQUIRED.into()))?;
    let column = normalize_field(raw_field)
        .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
    Ok((id, column, raw_field.to_string(), val.to_string()))
}

/// `batch` 是否执行删除（Java `BaseServiceImpl#batch`：仅 `type == "del"`，
/// 空 `ids` 时 `removeByIds` 不落 SQL 且返回 `false`）。
fn is_delete_batch(dto: &BatchDTO) -> bool {
    dto.r#type.as_deref() == Some("del") && !dto.ids.is_empty()
}

/// 实体 → VO（Java `BeanUtils.copyProperties` + 时间格式化 + `statusText`）。
///
/// - `status_text_now = Some(now)` → 计算 `statusText`（列表）；
///   `None` → 不赋值（详情，Java 原样为 `null`）
/// - `startTime` / `endTime` 仅当 `> 0` 才格式化（否则 `null`）
/// - `sortOrder` / `isShow` / `status` / `pics` 恒 `null`（见 models 模块头）
#[must_use]
fn to_vo(e: &SalesmanContent, status_text_now: Option<i32>) -> SalesmanContentVO {
    SalesmanContentVO {
        id: e.id,
        title: e.title.clone(),
        describe: e.describe.clone(),
        content: e.content.clone(),
        img: e.img.clone(),
        start_time: format_time(Some(e.start_time)),
        end_time: format_time(e.end_time),
        sort_order: None,
        is_show: None,
        is_available: e.is_available,
        is_top: e.is_top,
        status: None,
        status_text: status_text_now
            .map(|now| derive_status_text(now, e.start_time, e.end_time).to_string()),
        shop_id: e.shop_id,
        pics: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::salesman_content::{
        MSG_CONTENT_REQUIRED, MSG_STATUS_END, MSG_STATUS_NOT_START, MSG_STATUS_SHOWING,
    };

    fn content() -> SalesmanContent {
        SalesmanContent {
            id: 7,
            title: "旧标题".into(),
            img: "old.png".into(),
            start_time: 1_700_000_000,
            end_time: Some(1_800_000_000),
            describe: "旧描述".into(),
            is_top: 0,
            content: Some("旧正文".into()),
            is_available: Some(CONST_YES),
            shop_id: 3,
            pics: Some("[\"a.png\"]".into()),
        }
    }

    #[test]
    fn create_entity_uses_java_defaults() {
        let dto = SalesmanContentSaveDTO {
            title: Some("标题".into()),
            content: Some("正文".into()),
            ..Default::default()
        };
        let e = build_create_entity(&dto);
        assert_eq!(e.title, "标题");
        assert_eq!(e.img, "");
        assert_eq!(e.describe, "");
        assert_eq!(e.start_time, 0);
        // Java 未传 endTime → DB 默认 0（不是 NULL）
        assert_eq!(e.end_time, Some(0));
        assert_eq!(e.is_top, 0);
        // Java 未传 isAvailable → DB 默认 1
        assert_eq!(e.is_available, Some(CONST_YES));
        // Java SaveParam 无 shopId → DB 默认 0
        assert_eq!(e.shop_id, 0);
        assert_eq!(e.pics, None);

        // 时间字符串解析（东八区）
        let dto = SalesmanContentSaveDTO {
            content: Some("正文".into()),
            start_time: Some("2023-11-15 06:13:20".into()),
            end_time: Some("2023-11-16 06:13:20".into()),
            is_top: Some(1),
            ..Default::default()
        };
        let e = build_create_entity(&dto);
        assert_eq!(e.start_time, 1_700_000_000);
        assert_eq!(e.end_time, Some(1_700_086_400));
        assert_eq!(e.is_top, 1);
    }

    #[test]
    fn create_validates_required_content() {
        let dto = SalesmanContentSaveDTO::default();
        assert_eq!(validate_save(&dto).unwrap_err(), MSG_CONTENT_REQUIRED);
    }

    #[test]
    fn vo_formats_times_and_status_text() {
        let e = content();
        // now 在 [start, end) 内 → 展示中；时间已格式化
        let vo = to_vo(&e, Some(1_750_000_000));
        assert_eq!(vo.status_text.as_deref(), Some(MSG_STATUS_SHOWING));
        assert_eq!(
            vo.start_time.as_deref(),
            Some("2023-11-15 06:13:20")
        );
        assert!(vo.end_time.is_some());
        // Java 恒为 null 的字段
        assert_eq!(vo.sort_order, None);
        assert_eq!(vo.is_show, None);
        assert_eq!(vo.status, None);
        assert_eq!(vo.pics, None);

        // now < start → 未开始
        let vo = to_vo(&e, Some(1_600_000_000));
        assert_eq!(vo.status_text.as_deref(), Some(MSG_STATUS_NOT_START));
        // now >= end → 已失效
        let vo = to_vo(&e, Some(1_900_000_000));
        assert_eq!(vo.status_text.as_deref(), Some(MSG_STATUS_END));

        // 详情：不设置 statusText
        let vo = to_vo(&e, None);
        assert_eq!(vo.status_text, None);

        // start/end <= 0 → null；endTime = None → statusText ""（Java endTime==null 分支）
        let mut zero = content();
        zero.start_time = 0;
        zero.end_time = None;
        let vo = to_vo(&zero, Some(1_750_000_000));
        assert_eq!(vo.start_time, None);
        assert_eq!(vo.end_time, None);
        assert_eq!(vo.status_text.as_deref(), Some(""));
    }

    #[test]
    fn merge_update_keeps_unsent_fields() {
        let dto = SalesmanContentEditDTO {
            id: Some(7),
            title: Some("新标题".into()),
            ..Default::default()
        };
        let merged = merge_update(content(), &dto, 1_750_000_000);
        assert_eq!(merged.title, "新标题");
        assert_eq!(merged.img, "old.png");
        assert_eq!(merged.describe, "旧描述");
        assert_eq!(merged.start_time, 1_700_000_000);
        assert_eq!(merged.end_time, Some(1_800_000_000));
        assert_eq!(merged.is_available, Some(CONST_YES));
        assert_eq!(merged.shop_id, 3);
        // pics 不更新（Java BeanUtils 跳过）
        assert_eq!(merged.pics, Some("[\"a.png\"]".into()));
    }

    #[test]
    fn merge_update_links_end_time_to_availability() {
        // 结束时间晚于当前 → is_available = 1
        let dto = SalesmanContentEditDTO {
            id: Some(7),
            end_time: Some("2027-01-01 00:00:00".into()),
            is_available: Some(CONST_NO),
            ..Default::default()
        };
        let merged = merge_update(content(), &dto, 1_750_000_000);
        assert_eq!(merged.end_time, parse_time(Some("2027-01-01 00:00:00")));
        assert_eq!(merged.is_available, Some(CONST_YES));

        // 结束时间早于当前 → is_available = 0
        let dto = SalesmanContentEditDTO {
            id: Some(7),
            end_time: Some("2024-01-01 00:00:00".into()),
            ..Default::default()
        };
        let merged = merge_update(content(), &dto, 1_750_000_000);
        assert_eq!(merged.is_available, Some(CONST_NO));

        // 未提交 endTime → 尊重显式 is_available
        let dto = SalesmanContentEditDTO {
            id: Some(7),
            is_available: Some(CONST_NO),
            shop_id: Some(9),
            ..Default::default()
        };
        let merged = merge_update(content(), &dto, 1_750_000_000);
        assert_eq!(merged.is_available, Some(CONST_NO));
        assert_eq!(merged.shop_id, 9);
    }

    #[test]
    fn update_field_resolution_matches_java_whitelist_and_messages() {
        let dto = UpdateFieldDTO {
            id: Some(7),
            field: Some("isTop".into()),
            val: Some("1".into()),
        };
        let (id, column, raw, val) = resolve_update_field(&dto).unwrap();
        assert_eq!(id, 7);
        assert_eq!(column, "is_top");
        assert_eq!(raw, "isTop");
        assert_eq!(val, "1");

        // id 缺失 → id不能为空
        let dto = UpdateFieldDTO {
            field: Some("title".into()),
            val: Some("x".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_update_field(&dto).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_ID_REQUIRED}")
        );
        // 字段缺失 → 字段名不能为空
        let dto = UpdateFieldDTO {
            id: Some(1),
            val: Some("x".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_update_field(&dto).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_FIELD_NAME_REQUIRED}")
        );
        // 值缺失 → 值不能为空
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("title".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_update_field(&dto).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_FIELD_VALUE_REQUIRED}")
        );
        // 非白名单字段 → field 的值有误
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("content".into()),
            val: Some("x".into()),
        };
        assert_eq!(
            resolve_update_field(&dto).unwrap_err().to_string(),
            format!("参数校验失败: {MSG_INVALID_FIELD_VALUE}")
        );
        // 列名形式也能通过
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("is_available".into()),
            val: Some("0".into()),
        };
        assert_eq!(resolve_update_field(&dto).unwrap().1, "is_available");
    }

    #[test]
    fn batch_only_deletes_for_del_type() {
        let dto = BatchDTO {
            ids: vec![1, 2],
            r#type: Some("del".into()),
            value: 0,
        };
        assert!(is_delete_batch(&dto));
        assert!(!is_delete_batch(&BatchDTO {
            ids: vec![1],
            r#type: Some("show".into()),
            value: 0,
        }));
        assert!(!is_delete_batch(&BatchDTO {
            ids: vec![],
            r#type: Some("del".into()),
            value: 0,
        }));
        assert!(!is_delete_batch(&BatchDTO {
            ids: vec![1],
            r#type: None,
            value: 0,
        }));
    }

    #[test]
    fn filters_follow_java_list_semantics() {
        let q = SalesmanContentPageQuery {
            title: Some("  活动 ".into()),
            start_time: Some("2026-01-01".into()),
            end_time: Some("2026-02-01".into()),
            status: Some(1),
            ..Default::default()
        };
        let f = build_filter(&q, Some(0), false, 1_750_000_000);
        assert_eq!(f.title.as_deref(), Some("  活动 "));
        assert_eq!(f.start_time, Some(1_767_196_800)); // 2026-01-01 00:00 +08
        assert_eq!(f.end_time, Some(1_769_875_200)); // 2026-02-01 00:00 +08
        assert_eq!(f.status, Some(1));
        assert!(!f.from_user);
        assert_eq!(f.shop_id, Some(0));
        assert_eq!(f.now, 1_750_000_000);

        // 兼容完整时间字符串
        let q = SalesmanContentPageQuery {
            start_time: Some("2023-11-15 06:13:20".into()),
            ..Default::default()
        };
        let f = build_filter(&q, Some(-1), true, 0);
        assert_eq!(f.start_time, Some(1_700_000_000));
        assert!(f.from_user);
        assert_eq!(f.shop_id, Some(-1));
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_content() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (scope, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                TenantScope::current().shop_id().unwrap_or(0),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(scope, 8);
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let scope = with_tenant_scope(TenantScope::platform(), async {
            TenantScope::current().shop_id().unwrap_or(0)
        })
        .await;
        assert_eq!(scope, 0);
    }

    #[test]
    fn config_returns_three_states() {
        // config 是纯逻辑（不触库），直接构造 checks
        let cfg = status_config();
        assert_eq!(cfg[&0], MSG_STATUS_NOT_START);
        assert_eq!(cfg[&1], MSG_STATUS_SHOWING);
        assert_eq!(cfg[&2], MSG_STATUS_END);
    }
}
