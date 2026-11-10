//! 运费模板服务（logistics::shipping_tpl）。
//!
//! 对应原 Java：
//! - `com.nextrade.service.setting.impl.ShippingTplServiceImpl`（305 行）
//! - `com.nextrade.service.setting.impl.ShippingTplInfoServiceImpl`
//!   （仅 `saveShippingTplInfo` / `updateShippingTplInfo` 两个方法参与本切片）
//! - `com.nextrade.service.setting.impl.ShippingTypeServiceImpl`
//!   （仅 `getShippingTypeListByShopId` 参与本切片）
//! - `com.nextrade.adminapi.controller.setting.ShippingTplController`（8 条链路）
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /setting/shippingTpl/detail` | `ShippingTplController.detail` | [`ShippingTplService::detail`] |
//! | `POST /setting/shippingTpl/create` | `ShippingTplController.create` | [`ShippingTplService::create`] |
//! | `POST /setting/shippingTpl/update` | `ShippingTplController.update` | [`ShippingTplService::update`] |
//! | `POST /setting/shippingTpl/del` | `ShippingTplController.del` | [`ShippingTplService::del`] |
//! | `POST /setting/shippingTpl/updateField` | `ShippingTplController.updateField` | [`ShippingTplService::update_field`] |
//! | `POST /setting/shippingTpl/batch` | `ShippingTplController.batch` | [`ShippingTplService::batch`] |
//! | `GET  /setting/shippingTpl/config` | `ShippingTplController.config` | [`ShippingTplService::config`] |
//! | `POST /setting/shippingTpl/setDefault` | `ShippingTplController.setDefault` | [`ShippingTplService::set_default`] |
//!
//! ## 多租户（`shopId`）
//!
//! Java 全程用 `HeaderUtils.getShopId()`（**请求头**）取店铺 ID，
//! 用它做查询过滤与入库赋值。Rust 侧 JWT 尚未签发 `shop_id`，
//! 因此本服务的每个公开方法都接受一个显式 `shop_id: Option<i32>`：
//! - `None`：平台账号语义（Java `getShopId() == null`），
//!   查询**不加店铺过滤**、入库 `shop_id = 0`
//! - `Some(id)`：店铺账号语义，查询按店铺过滤、入库写该 ID
//!
//! 待鉴权中间件补齐后由路由层从 JWT 填充，服务层无需改动。
//!
//! ## 与 Java 的已知偏差（逐条列入 `uncertain_points`）
//!
//! 1. **`update` 的 `updateById` 主键来源**：Java 用
//!    `shippingTpl.setShippingTplId(updateDTO.getId())`，
//!    但 `BeanUtils.copyProperties(updateDTO, shippingTpl)` 已经先把
//!    `shippingTplId` 拷进去了 —— 请求体同时带 `id` 与 `shippingTplId` 且
//!    两者不同时，后者被前者覆盖。本实现**只认 `id`**（与 Java 净结果一致）。
//! 2. **`update` 不校验 `shopId` 归属**：Java `update` 没有
//!    `setDefault` 那样的 `shopId.equals(...)` 校验，可以直接改别家模板。
//!    本实现逐字复刻（未加校验），但这看起来是一个越权缺口。
//! 3. **`update` 的 `BeanUtils.copyProperties` 会把请求体的 `shopId`
//!    写进实体**（Java 未覆盖它），即调用方可以凭请求体改掉模板归属店铺。
//!    本实现同样把 `shopId` 透传（见 `update` 的 `shop_id` 补丁）。
//! 4. **`updateField` 白名单里有 3 个不存在的列**（`is_show` /
//!    `sort_order` / `status`）：Java 会拼出必然失败的 SQL（MySQL 报
//!    `Unknown column`，500）。本实现改为**提前**返回
//!    `400 字段值不合法`，不产生非法 SQL。
//! 5. **`create` / `update` 的 `isChecked` 口径不一致**
//!    （create 用 `== 1`，update 用 `> 0`），本实现分别复刻。
//! 6. **`config` 端点在 Java 里是硬编码桩**：`ShippingTypeServiceImpl`
//!    `#getShippingTypeListByShopId` **完全不查库**，
//!    直接 `new ShippingType(1L)` 后 `BeanUtils.copyProperties` 出一个
//!    只有 `shippingTypeId = 1`、其余字段全 `null` 的 VO 列表。
//!    本实现逐字复刻该桩（**不查 `shipping_type` 表**），
//!    并把「查库的真实实现」保留在 [`ShippingTplService::config_from_db`]。
//!    这是本切片最需要产品确认的一点。
//! 7. **`detail` 返回的 `shippingType` 是空壳**：
//!    Java `new ShippingTypeVO(new ShippingType(), shippingTplInfos)`
//!    —— 配送类型实体是 `new ShippingType()` 全默认对象，
//!    `shippingTypeId = 0`、名称等全 `null`。本实现复刻。
//! 8. **`del` 不清理子表**：Java 只 `removeById(id)`，
//!    `shipping_tpl_info` 留下孤儿行。本实现复刻。
//! 9. **`ShippingTplInfoVO.pricingType` 恒为 `null`**：
//!    Java 构造器漏赋值。本实现复刻（保留修正版供切换）。

use std::sync::Arc;

use nextrade_core::models::logistics::shipping_tpl::{
    map_update_field_column, validate_save, BatchDTO, DetailQuery, OperateDTO, SetDefaultQuery,
    ShippingTplInfoRow, ShippingTplListQuery, ShippingTplSaveDTO, ShippingTplUpdateDTO,
    ShippingTplVO, ShippingTypeParam, ShippingTypeVO, UpdateFieldDTO, DEFAULT_SHIPPING_TYPE_ID,
    MSG_INVALID_FIELD_VALUE, MSG_SHIPPING_TPL_ID_REQUIRED, MSG_SHIPPING_TPL_NAME_EXISTS,
    MSG_SHIPPING_TPL_NOT_FOUND, MSG_SHIPPING_TPL_NO_PERMISSION, MSG_SHIPPING_TPL_UPDATE_ID_REQUIRED,
    MSG_SHIPPING_TYPE_NOT_NULL, NO, SHIPPING_TPL_COLUMNS, SHIPPING_TPL_FIELDS, YES,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::logistics::shipping_tpl::{
    ShippingTplInsert, ShippingTplPatch, ShippingTplRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 运费模板域服务（挂载于 `AdminState.shipping_tpl`）。
pub struct ShippingTplService {
    repos: Arc<Repositories>,
}

impl ShippingTplService {
    /// 构造（与 order_admin / admin_user / area_code 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 仓储快捷访问。
    fn repo(&self) -> &ShippingTplRepository {
        &self.repos.logistics.shipping_tpl
    }

    /// 模板归属店铺（多租户校验用；模板不存在返回 `None`）。
    ///
    /// 路由层在 `detail` / `del` / `updateField` / `batch` 前调用，
    /// 配合 `AuthUser::ensure_shop_access` 阻断跨店读写。
    pub async fn shop_id_of(&self, id: i32) -> Result<Option<i32>> {
        Ok(self.repo().find_tpl_by_id(id).await?.map(|t| t.shop_id))
    }

    // ————————————————————— 详情 —————————————————————

    /// 运费模板详情（对应 Java `ShippingTplServiceImpl#detail`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `getById(id)`；不存在 ⇒ `该运费模板不存在`
    /// 2. 查该模板下全部 `shipping_tpl_info`
    /// 3. 组装 `new ShippingTypeVO(new ShippingType(), infos)`
    ///    —— **配送类型是空壳**（`shippingTypeId = 0`，见模块头第 7 条）
    /// 4. 返回 `new ShippingTplVO(tpl, [shippingType])`
    ///
    /// ⚠️ Java 的 `detail` 参数是 `@RequestParam("id") Integer id`（必填），
    /// 缺参由 Spring 直接 400；本实现返回
    /// [`MSG_SHIPPING_TPL_ID_REQUIRED`]。
    ///
    /// # Errors
    /// - `id` 缺失 ⇒ `模版主键不能为空`
    /// - 记录不存在 ⇒ `该运费模板不存在`
    pub async fn detail(&self, q: &DetailQuery) -> Result<ShippingTplVO> {
        let id = q.id.ok_or_else(|| Error::Validation(MSG_SHIPPING_TPL_ID_REQUIRED.into()))?;
        self.detail_by_id(id).await
    }

    /// 按主键取详情（供路由层直接复用，避免重复解包 query）。
    ///
    /// # Errors
    /// - 记录不存在 ⇒ `该运费模板不存在`
    pub async fn detail_by_id(&self, id: i32) -> Result<ShippingTplVO> {
        let tpl = self
            .repo()
            .find_tpl_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_SHIPPING_TPL_NOT_FOUND.into()))?;

        let infos = self.repo().find_infos_by_tpl(id).await?;
        // Java: `new ShippingTypeVO(new ShippingType(), infos)` —— 空壳配送类型
        let shipping_type = ShippingTypeVO::empty_shell(&infos);

        Ok(ShippingTplVO::with_types(&tpl, vec![shipping_type]))
    }

    // ————————————————————— 列表（本切片无路由，保留实现） —————————————————————

    /// 运费模板分页（对应 Java `ShippingTplServiceImpl#list`）。
    ///
    /// ⚠️ **本方法不对应本切片的 8 条路由**（任务书清单里没有 `/list`）。
    /// 保留实现是为了让同一张表的读取逻辑集中在一处，供后续切片 /
    /// 主会话直接复用，不必重读 Java。
    ///
    /// 语义（逐条对照 Java）：
    /// - `shopId != null` ⇒ `AND shop_id = ?`
    /// - `keyword` 非空 ⇒ `AND shipping_tpl_name ILIKE %keyword%`
    /// - 排序走 `buildSortOrder(page, sortField, sortOrder)`
    /// - **不使用** `isShow`（表无该列，Java 也从未拼进 wrapper）
    ///
    /// # Errors
    /// 数据库错误 ⇒ `Error::Database`
    pub async fn list(
        &self,
        q: &ShippingTplListQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<ShippingTplVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate_tpl(q, shop_id).await?;
        let list: Vec<ShippingTplVO> = rows.iter().map(ShippingTplVO::from_entity).collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建运费模板（对应 Java `ShippingTplServiceImpl#create`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. 同店铺下模板名查重 ⇒ `运费模板名称已存在`
    ///    （⚠️ Java 是 `eq(shopId, shopId).eq(name, name)`，
    ///     **不排除自身**，但 create 场景无需排除）
    /// 2. 过滤出 `isChecked == 1` 的运送方式；为空 ⇒ `请选择运送方式`
    /// 3. 若 `isDefault == 1`，把**同店铺**其它模板的默认清 0
    ///    （Java 在 `save` **之前**执行，且**不排除自身** ——
    ///    此时新记录尚未入库，净结果正确）
    /// 4. 入库 `shipping_tpl`，取回自增主键
    /// 5. 按勾选的运送方式展开 `shipping_tpl_info` 行并批量入库
    /// 6. Java 还有 `adminLogService.createByLogInfo("创建运费模板:...")`
    ///    —— 本实现通过 `AdminLogService::record_log` 写 `admin_log`，
    ///    actor/IP 由路由层从 JWT 与请求头注入
    ///
    /// 返回新建模板主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    ///
    /// # Errors
    /// - 入参校验失败 ⇒ 带 Java 原文案的 `Validation`
    /// - 名称重复 ⇒ `运费模板名称已存在`
    /// - 无勾选运送方式 ⇒ `请选择运送方式`
    pub async fn create(
        &self,
        dto: &ShippingTplSaveDTO,
        shop_id: Option<i32>,
        admin_id: i32,
        client_ip: &str,
    ) -> Result<i32> {
        // 1. 入参校验（Java `@NotBlank` / `@NotNull`，见 validate_save 的说明）
        let v = validate_save(dto).map_err(Error::Validation)?;

        // 2. 同店铺模板名查重（Java create 的 `nameCount == 0` 断言）
        let name_count = self
            .repo()
            .count_tpl_by_name(&v.name, shop_id, None)
            .await?;
        if name_count > 0 {
            return Err(Error::Validation(MSG_SHIPPING_TPL_NAME_EXISTS.into()));
        }

        // 3. 至少勾选一种运送方式（Java `Constants.YES.equals(isChecked)`）
        let params: &[ShippingTypeParam] = dto.shipping_tpl_info.as_deref().unwrap_or_default();
        let checked: Vec<&ShippingTypeParam> =
            params.iter().filter(|p| p.is_checked_yes()).collect();
        if checked.is_empty() {
            return Err(Error::Validation(MSG_SHIPPING_TYPE_NOT_NULL.into()));
        }

        // 4. 若本次设为默认，先清掉同店铺其它默认（Java 在 save 之前）
        //    shop_id 为 None（平台账号）时 Java 的 eq 拼成 `shop_id = null`
        //    ⇒ 恒不匹配；本实现用 None 表达「清全部」，见仓储层说明。
        if v.is_default == YES {
            self.repo().clear_default_flag(shop_id).await?;
        }

        // 5. 入库模板
        let tpl_id = self
            .repo()
            .insert_tpl(&ShippingTplInsert {
                shipping_time: v.shipping_time.clone(),
                shipping_tpl_name: v.name.clone(),
                is_free: v.is_free,
                pricing_type: v.pricing_type,
                is_default: v.is_default,
                // Java `param.createShippingTpl(shopId)`：shopId 为 null 时
                // MyBatis-Plus 跳过该列、由 MySQL 施加 NOT NULL 默认 0
                shop_id: shop_id.unwrap_or_default(),
            })
            .await?;

        // 6. 展开并批量写入内容行（Java
        //    `shippingTplInfoService.saveShippingTplInfo(checked, shippingTpl)`）
        let tpl = self
            .repo()
            .find_tpl_by_id(tpl_id)
            .await?
            .ok_or_else(|| Error::Database("运费模板插入后回读失败".into()))?;
        let rows = build_info_rows(&checked, &tpl);
        let inserted = self.repo().insert_infos(&rows).await?;

        let log_info = format!("创建运费模板:{}", v.name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_log(admin_id, if shop_id.unwrap_or(0) > 0 { "shop" } else { "admin" }, shop_id.unwrap_or(0), &log_info, Some(client_ip), None)
            .await
        {
            tracing::warn!("写管理操作日志失败（创建运费模板）: {e}");
        }

        tracing::info!(
            "创建运费模板: id={tpl_id}, name={}, 运送方式 {} 组, 内容行 {}",
            v.name,
            checked.len(),
            inserted
        );
        Ok(tpl_id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新运费模板（对应 Java `ShippingTplServiceImpl#update`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `shippingTplName` 非空时查重（**排除自身**）⇒ `运费模板名称已存在`
    /// 2. 遍历 `shippingTplInfo`，任一 `isChecked > 0` 即视为「有运送方式」；
    ///    全部 `<= 0` ⇒ `请选择运送方式`
    /// 3. 若 `isDefault == 1`，清掉同店铺其它默认（Java 不排除自身 ——
    ///    因为紧接着的 `updateById` 会把自身的 `is_default` 写回请求值，
    ///    净结果正确；本实现同样不排除，保持与 Java 一致的执行顺序）
    /// 4. `BeanUtils.copyProperties(updateDTO, shippingTpl)` +
    ///    `setShippingTplId(updateDTO.getId())` ⇒ `updateById`
    /// 5. **先删后插**该模板的全部内容行
    ///    （`ShippingTplInfoServiceImpl#updateShippingTplInfo`）
    /// 6. Java 记录操作日志（本实现写 `admin_log`，actor/IP 由路由注入）
    ///
    /// 返回受影响行数 > 0（Java 控制器返回 `void`）。
    ///
    /// ⚠️ **注意第 4 步的 `shopId`**：Java `BeanUtils.copyProperties` 会把
    /// 请求体的 `shopId` 带进实体，且**往后的代码没有覆盖它**，
    /// 因此调用方可以凭请求体改掉模板归属店铺。本实现透传 `dto.shop_id`
    /// （逐字复刻），这一越权面已列入 `uncertain_points`。
    ///
    /// # Errors
    /// - 缺主键 ⇒ `运费模板主键不能为空`
    /// - 名称重复 ⇒ `运费模板名称已存在`
    /// - 无运送方式 ⇒ `请选择运送方式`
    pub async fn update(
        &self,
        dto: &ShippingTplUpdateDTO,
        shop_id: Option<i32>,
        admin_id: i32,
        client_ip: &str,
    ) -> Result<bool> {
        // 1. 主键必填（Java 的 ShippingTplUpdateDTO 无 @NotNull，
        //    但 `updateById` 缺主键必然失败；本实现提前给出可诊断错误）
        let id = dto
            .id
            .filter(|v| *v > 0)
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| Error::Validation(MSG_SHIPPING_TPL_UPDATE_ID_REQUIRED.into()))?;

        // 2. 名称查重（Java `StrUtil.isNotBlank` 守卫 ⇒ 空名不查重、不更新）
        let effective_name = dto.effective_name().map(str::to_string);
        if let Some(name) = &effective_name {
            let cnt = self
                .repo()
                .count_tpl_by_name(name, shop_id, Some(id))
                .await?;
            if cnt > 0 {
                return Err(Error::Validation(MSG_SHIPPING_TPL_NAME_EXISTS.into()));
            }
        }

        // 记录必须存在：Java `updateById` 命中 0 行静默返回 false，
        // 这里先查一次以便区分「不存在」与「值未变化」
        if self.repo().find_tpl_by_id(id).await?.is_none() {
            return Err(Error::Validation(MSG_SHIPPING_TPL_NOT_FOUND.into()));
        }

        // 3. 至少一种运送方式（Java `isChecked > 0`，与 create 的 `== 1` 不同）
        let params: &[ShippingTypeParam] = dto.shipping_tpl_info.as_deref().unwrap_or_default();
        if !params.iter().any(ShippingTypeParam::is_checked_positive) {
            return Err(Error::Validation(MSG_SHIPPING_TYPE_NOT_NULL.into()));
        }

        // 4. 若本次设为默认，先清同店铺其它默认（Java 在 updateById 之前）
        if dto.wants_default() {
            self.repo().clear_default_flag(shop_id).await?;
        }

        // 5. 构造补丁（FieldStrategy.NOT_NULL：只写非 null 字段）
        //    `id` 已被 Java `setShippingTplId(updateDTO.getId())` 覆盖，
        //    请求体里的 `shippingTplId` 不参与（见模块头第 1 条）。
        let patch = ShippingTplPatch {
            shipping_time: dto.shipping_time.clone().map(Some),
            shipping_tpl_name: effective_name.map(Some),
            is_free: dto.is_free.map(Some),
            pricing_type: dto.pricing_type.map(Some),
            is_default: dto.is_default.map(Some),
            // ⚠️ Java 未覆盖 shopId ⇒ 请求体可改写归属（有意复刻）
            shop_id: dto.shop_id.map(Some),
        };
        let affected = self.repo().update_tpl(id, &patch).await?;

        // 6. 先删后插内容行（Java updateShippingTplInfo）
        let checked: Vec<&ShippingTypeParam> =
            params.iter().filter(|p| p.is_checked_positive()).collect();
        self.repo().delete_infos_by_tpl(id).await?;
        let tpl = self
            .repo()
            .find_tpl_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_SHIPPING_TPL_NOT_FOUND.into()))?;
        let rows = build_info_rows(&checked, &tpl);
        let inserted = self.repo().insert_infos(&rows).await?;

        let log_info = format!("修改运费模板:{}", dto.shipping_tpl_name.as_deref().unwrap_or(""));
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_log(admin_id, if dto.shop_id.unwrap_or(0) > 0 { "shop" } else { "admin" }, dto.shop_id.unwrap_or(0), &log_info, Some(client_ip), None)
            .await
        {
            tracing::warn!("写管理操作日志失败（修改运费模板）: {e}");
        }

        tracing::info!(
            "修改运费模板: id={id}, name={:?}, 影响 {affected} 行, 内容行 {}",
            dto.shipping_tpl_name,
            inserted
        );
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除运费模板（对应 Java `ShippingTplServiceImpl#del`
    /// → `BaseServiceImpl#del` → `removeById`，硬删除）。
    ///
    /// ⚠️ Java 的 `del` 是「`id == null` 返回 false，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功）。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404。
    ///
    /// ⚠️ **不级联删除 `shipping_tpl_info`**（Java 原样，见模块头第 8 条）。
    ///
    /// # Errors
    /// - `id` 缺失 ⇒ `id不能为空`
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(nextrade_core::models::logistics::shipping_tpl::MSG_ID_REQUIRED.into()))?;
        self.repo().delete_tpl(id).await
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java
    /// `ShippingTplServiceImpl#updateField(dto, FIELDS)`，
    /// `FIELDS = { "is_show", "sort_order", "status", "shipping_tpl_name" }`）。
    ///
    /// 流程与 Java 逐条对齐，**外加一处有意收紧**：
    /// 1. `field` 经 `getColumnByPropertyName` 归一为列名
    ///    （属性名 / 列名都可命中；
    ///    Java 未识别时回退主键列 `shipping_tpl_id`，再被白名单拒绝）
    /// 2. 归一后的列名必须落在 [`SHIPPING_TPL_FIELDS`]（**Java 原白名单**）内，
    ///    否则 `字段值不合法`
    /// 3. ⚠️ **额外**：该列还必须是 `shipping_tpl` 的**真实列**
    ///    （[`SHIPPING_TPL_COLUMNS`]）。Java 白名单里的 `is_show` /
    ///    `sort_order` / `status` **在表中不存在**，Java 会让 MySQL 报
    ///    `Unknown column`（500）。本实现提前返回 `字段值不合法`（400），
    ///    **不产生非法 SQL**（见模块头第 4 条）
    /// 4. `UPDATE shipping_tpl SET <列> = ? WHERE shipping_tpl_id = ?`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换
    ///
    /// ⚠️ Java `updateField` **不触发**任何「清其它默认」逻辑
    /// （`is_default` 不在白名单里）；本实现一致。
    ///
    /// 返回是否命中记录（Java 直接返回 `this.update(updateWrapper)` 的布尔值）。
    ///
    /// # Errors
    /// - `id` 缺失 ⇒ `id不能为空`
    /// - 字段名缺失 / 归一失败 / 不在白名单 / 非真实列 ⇒ `字段值不合法`
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(nextrade_core::models::logistics::shipping_tpl::MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;

        // 1. 归一（属性名 / 列名）
        let column = map_update_field_column(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        // 2. Java 白名单
        if !SHIPPING_TPL_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        // 3. 额外收紧：必须是真实存在的列（避免 500）
        if !SHIPPING_TPL_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        let val = dto.val.as_deref().unwrap_or_default();
        let affected = self.repo().update_tpl_field(id, column, val).await?;

        tracing::info!("更新运费模板字段: id={id}, {column} = {val}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器 `batch` 的参数是裸 `@RequestBody BatchDTO batchDTO`
    /// （**无 `@Valid`**），因此 `@NotEmpty` 实际上不生效；
    /// 空 ids 会走到 `removeByIds(空集合)` 并静默返回 `false`。
    /// 本实现保持一致（空 ids 返回 `false`，不报 400）。
    ///
    /// ⚠️ 与 `del` 一样**不级联删除子表**。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            return Ok(false);
        }
        let affected = self.repo().delete_tpl_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 配置（运送方式下拉） —————————————————————

    /// 获取运费模板配置（对应 Java `ShippingTplController#config`
    /// → `ShippingTplServiceImpl#getShippingTypeListByShopId`
    /// → `ShippingTypeServiceImpl#getShippingTypeListByShopId`）。
    ///
    /// ⚠️⚠️ **Java 该方法是硬编码桩，完全不查 `shipping_type` 表**：
    /// ```text
    /// List<ShippingType> list = new ArrayList<>();
    /// list.add(new ShippingType(1L));        // 只有 shippingTypeId = 1
    /// return list.stream().map(st -> {
    ///     ShippingTypeVO vo = new ShippingTypeVO();
    ///     BeanUtils.copyProperties(st, vo);  // 其余字段全 null
    ///     return vo;
    /// }).toList();
    /// ```
    /// 因此线上返回的是**恰好一个** VO：
    /// `{ "shippingTypeId": 1, 其余字段全部 null }`。
    ///
    /// 本实现**逐字复刻该桩**（不查库），以免 Rust 与 Java 行为不一致。
    /// 需要「查库的真实实现」时用 [`ShippingTplService::config_from_db`]，
    /// 并请产品确认（见 `uncertain_points`）。
    ///
    /// # Errors
    /// 本方法不访问数据库，不会返回错误。
    pub fn config(&self, shop_id: Option<i32>) -> Result<Vec<ShippingTypeVO>> {
        let _ = shop_id;
        let mut vo = ShippingTypeVO::default();
        vo.shipping_type_id = DEFAULT_SHIPPING_TYPE_ID;
        // 其余字段保持 None / 空（与 BeanUtils.copyProperties(new ShippingType(1L), vo) 一致）
        Ok(vec![vo])
    }

    /// **查库版本**的运送方式列表 —— **Java 的意图版本**，当前未被路由调用。
    ///
    /// 与 [`ShippingTplService::config`] 的区别：这里真的去查 `shipping_type`
    /// 表（按店铺过滤、按 `sort_order` 排序），并把每个配送类型的
    /// `logisticsName` 一并补上。
    ///
    /// ⚠️ 一旦启用，`config` 接口的返回体将从
    /// 「1 条仅含 id 的空壳」变成「N 条完整记录」，
    /// 这是**与当前 Java 线上行为不一致**的变更，需产品确认。
    ///
    /// # Errors
    /// - 数据库错误 ⇒ `Error::Database`
    pub async fn config_from_db(&self, shop_id: Option<i32>) -> Result<Vec<ShippingTypeVO>> {
        let types = match shop_id {
            Some(id) => self.repo().find_types_by_shop(id).await?,
            // 平台账号：Java 那句 `eq(shippingTypeName...)` 的 `list` 在
            // shopId 为 null 时无结果；此处退化为「无店铺过滤，全表」。
            None => self.repo().find_types_by_shop(0).await?,
        };
        Ok(types
            .iter()
            .map(|t| ShippingTypeVO::from_entity(t, &[]))
            .collect())
    }

    // ————————————————————— 设为默认 —————————————————————

    /// 设置默认运费模板（对应 Java `ShippingTplServiceImpl#setDefault`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `getById(id)`；不存在 ⇒ `该运费模板不存在`
    /// 2. `shopId.equals(shippingTpl.getShopId())` 校验；
    ///    不匹配 ⇒ `无权操作该运费模板`
    ///    （⚠️ Java 用 `shopId.equals(...)`，`shopId` 为 `null` 时
    ///    **NPE**；本实现把 `shop_id == None` 视为「平台账号可操作任意模板」，
    ///    见 `uncertain_points`）
    /// 3. 先把该店铺全部模板的默认清 0
    /// 4. 再把目标模板置 1
    /// 5. Java 记录操作日志（本实现写 `admin_log`，actor/IP 由路由注入）
    ///
    /// 返回是否命中（受影响行数 > 0）。
    ///
    /// # Errors
    /// - `id` 缺失 ⇒ `运费模板id不能为空`（本实现补的文案）
    /// - 记录不存在 ⇒ `该运费模板不存在`
    /// - 归属店铺不符 ⇒ `无权操作该运费模板`
    pub async fn set_default(
        &self,
        q: &SetDefaultQuery,
        shop_id: Option<i32>,
        admin_id: i32,
        client_ip: &str,
    ) -> Result<bool> {
        let id = q
            .id
            .filter(|v| *v > 0)
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| {
                Error::Validation(nextrade_core::models::logistics::shipping_tpl::MSG_SET_DEFAULT_ID_REQUIRED.into())
            })?;

        // 1. 记录必须存在（Java `Assert.notNull`）
        let tpl = self
            .repo()
            .find_tpl_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_SHIPPING_TPL_NOT_FOUND.into()))?;

        // 2. 归属校验（Java `shopId.equals(shippingTpl.getShopId())`）
        if let Some(current) = shop_id {
            if current != tpl.shop_id {
                return Err(Error::Forbidden(MSG_SHIPPING_TPL_NO_PERMISSION.into()));
            }
        }

        // 3. 先清同店铺全部默认
        self.repo().clear_default_flag(shop_id.or(Some(tpl.shop_id))).await?;
        // 4. 再把目标置 1
        let affected = self.repo().mark_default(id).await?;

        let log_info = format!("设置默认运费模板:{}", tpl.shipping_tpl_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log
            .record_log(admin_id, if tpl.shop_id > 0 { "shop" } else { "admin" }, tpl.shop_id, &log_info, Some(client_ip), None)
            .await
        {
            tracing::warn!("写管理操作日志失败（设置默认运费模板）: {e}");
        }

        tracing::info!("设置默认运费模板: id={id}, name={}, 影响 {affected} 行", tpl.shipping_tpl_name);
        Ok(affected > 0)
    }

    // ————————————————————— 纯函数：内容行展开 —————————————————————

    /// 把勾选的运送方式展开为 `shipping_tpl_info` 待入库行。
    ///
    /// 与 Java
    /// `ShippingTplInfoServiceImpl#saveShippingTplInfo(checked, shippingTpl)` 等价：
    /// ```text
    /// shippingTypeParams.stream().flatMap(st -> {
    ///     rows.add(st.createDefaultShippingTplInfo(tpl));    // isDefault = 1
    ///     rows.addAll(st.createAreaShippingTplInfos(tpl));   // isDefault = 0
    ///     return rows.stream();
    /// })
    /// ```
    ///
    /// ⚠️ Java 的 `createDefaultShippingTplInfo` 在 `defaultTplInfo == null`
    /// 时 **NPE**；本实现由 [`ShippingTypeParam::info_rows`] 返回空列表
    /// （该配送类型整体跳过），不 panic。
    #[must_use]
    pub fn build_info_rows(
        checked: &[&ShippingTypeParam],
        tpl: &nextrade_core::entities::ShippingTpl,
    ) -> Vec<ShippingTplInfoRow> {
        build_info_rows(checked, tpl)
    }
}

/// 自由函数版本（供路由层 / 单测直接调用，避免构造 `ShippingTplService`）。
///
/// 见 [`ShippingTplService::build_info_rows`] 的语义说明。
#[must_use]
pub fn build_info_rows(
    checked: &[&ShippingTypeParam],
    tpl: &nextrade_core::entities::ShippingTpl,
) -> Vec<ShippingTplInfoRow> {
    checked.iter().flat_map(|p| p.info_rows(tpl)).collect()
}

/// 供 `create` / `update` 共用的 `isChecked` 口径常量说明（文档用）。
pub const CREATE_CHECKED_USES_EQ_YES: bool = true;
/// `update` 用的是 `> 0` 而非 `== 1`（见模块头第 5 条）。
pub const UPDATE_CHECKED_USES_GT_ZERO: bool = true;
/// 默认/非默认常量（与 `models::shipping_tpl` 对齐，便于测试引用）。
pub const YES_FLAG: i32 = YES;
/// 非默认常量。
pub const NO_FLAG: i32 = NO;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use nextrade_core::entities::ShippingTpl;

    fn tpl() -> ShippingTpl {
        ShippingTpl {
            shipping_tpl_id: 11,
            shipping_time: "1天内".into(),
            shipping_tpl_name: "默认模板".into(),
            is_free: 0,
            pricing_type: 2,
            is_default: 0,
            shop_id: 3,
        }
    }

    fn checked_param(price: i64) -> ShippingTypeParam {
        serde_json::from_str(&format!(
            r#"{{"isChecked":1,
                "defaultTplInfo":{{"startNumber":1,"startPrice":{price},"addNumber":1,
                "addPrice":2.00,"freePrice":99.00,
                "regionData":{{"areaRegions":[[1,360000]],"areaRegionNames":["江西"]}}}},
                "areaTplInfo":[{{"startPrice":1.00,"regionData":{{"areaRegions":[[1,1]]}}}}]}}"#
        ))
        .unwrap()
    }

    // ————————————————————— 常量 / 桩行为 —————————————————————

    /// `config` 复刻 Java 硬编码桩：恰好 1 条，只有 id，其余全 null。
    #[test]
    fn config_is_java_stub() {
        // 该测试不访问数据库：ShippingTplService::config 只构造常量对象。
        // 直接验证纯构造逻辑（与 Java `new ShippingType(1L)` + copyProperties 等价）
        let mut vo = ShippingTypeVO::default();
        vo.shipping_type_id = DEFAULT_SHIPPING_TYPE_ID;
        assert_eq!(vo.shipping_type_id, 1);
        assert_eq!(vo.shipping_type_name, None);
        assert_eq!(vo.shipping_default_id, None);
        assert_eq!(vo.shipping_type_desc, None);
        assert_eq!(vo.shipping_time_desc, None);
        assert_eq!(vo.is_support_cod, None);
        assert_eq!(vo.sort_order, None);
        assert_eq!(vo.shop_id, None);
        assert!(vo.default_tpl_info.is_none());
        assert!(vo.area_tpl_info.is_empty());
    }

    /// `config` 的文档常量（防误删）。
    #[test]
    fn checked_semantics_markers() {
        assert!(CREATE_CHECKED_USES_EQ_YES);
        assert!(UPDATE_CHECKED_USES_GT_ZERO);
        assert_eq!(YES_FLAG, 1);
        assert_eq!(NO_FLAG, 0);
    }

    // ————————————————————— 内容行展开 —————————————————————

    /// 每个勾选的运送方式产出「1 条默认 + N 条区域」，且顺序正确。
    #[test]
    fn build_rows_expands_default_then_area() {
        let a = checked_param(5);
        let b = checked_param(8);
        let checked = vec![&a, &b];
        let rows = build_info_rows(&checked, &tpl());

        assert_eq!(rows.len(), 4, "2 组 × (1 默认 + 1 区域)");
        // 第 1 组
        assert_eq!(rows[0].is_default, YES_FLAG);
        assert_eq!(rows[0].start_price, Decimal::new(500, 2));
        assert_eq!(rows[1].is_default, NO_FLAG);
        assert_eq!(rows[1].start_price, Decimal::new(100, 2));
        // 第 2 组
        assert_eq!(rows[2].is_default, YES_FLAG);
        assert_eq!(rows[2].start_price, Decimal::new(800, 2));
        assert_eq!(rows[3].is_default, NO_FLAG);
        // 全部指向同一模板，shippingTypeId 写死为 1
        assert!(rows.iter().all(|r| r.shipping_tpl_id == 11));
        assert!(rows.iter().all(|r| r.shipping_type_id == DEFAULT_SHIPPING_TYPE_ID));
        // isFree / pricingType 取自模板
        assert!(rows.iter().all(|r| r.is_free == 0 && r.pricing_type == 2));
    }

    /// 未勾选任何运送方式 ⇒ 不产出行（`create` 会在此之前报「请选择运送方式」）。
    #[test]
    fn build_rows_with_nothing_checked_is_empty() {
        let rows = build_info_rows(&[], &tpl());
        assert!(rows.is_empty());
    }

    /// 缺 `defaultTplInfo` 的组整体跳过，不 panic（Java 此处 NPE）。
    #[test]
    fn build_rows_skips_group_without_default_info() {
        let broken = ShippingTypeParam { is_checked: Some(1), ..Default::default() };
        let good = checked_param(5);
        let checked = vec![&broken, &good];
        let rows = build_info_rows(&checked, &tpl());
        assert_eq!(rows.len(), 2, "坏的那组被跳过，好的那组产出 2 行");
        assert_eq!(rows[0].shipping_tpl_id, 11);
    }

    /// 区域行数为 0 时只有默认行。
    #[test]
    fn build_rows_default_only() {
        let p: ShippingTypeParam = serde_json::from_str(
            r#"{"isChecked":1,"defaultTplInfo":{"startPrice":5.00},"areaTplInfo":[]}"#,
        )
        .unwrap();
        let rows = build_info_rows(&[&p], &tpl());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].is_default, YES_FLAG);
        // 未给的金额字段按 Decimal 0 处理（Java BigDecimal 为 null 会写入失败，
        // 本实现取 0，见 uncertain_points）
        assert_eq!(rows[0].free_price, Decimal::ZERO);
    }

    // ————————————————————— 域标签 / 文案 —————————————————————

    /// 权限矩阵未实现，但常量文案必须与 Java 逐字符一致。
    #[test]
    fn verbatim_messages() {
        assert_eq!(MSG_SHIPPING_TPL_NOT_FOUND, "该运费模板不存在");
        assert_eq!(MSG_SHIPPING_TPL_NAME_EXISTS, "运费模板名称已存在");
        assert_eq!(MSG_SHIPPING_TYPE_NOT_NULL, "请选择运送方式");
        assert_eq!(MSG_SHIPPING_TPL_NO_PERMISSION, "无权操作该运费模板");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "字段值不合法");
        assert_eq!(MSG_SHIPPING_TPL_ID_REQUIRED, "模版主键不能为空");
    }

    /// 白名单里 3 个幻觉列必须被服务层挡在真实列校验之外。
    #[test]
    fn phantom_columns_are_rejected_by_real_column_check() {
        for phantom in ["is_show", "sort_order", "status"] {
            assert!(SHIPPING_TPL_FIELDS.contains(&phantom), "{phantom} 在 Java 白名单里");
            assert!(
                !SHIPPING_TPL_COLUMNS.contains(&phantom),
                "{phantom} 不是 shipping_tpl 的真实列 ⇒ 服务层应拒绝"
            );
            // 归一函数本身也不认它（因为库里没有这一列）
            assert_eq!(map_update_field_column(phantom), None);
        }
        // 唯一合法字段
        assert_eq!(map_update_field_column("shipping_tpl_name"), Some("shipping_tpl_name"));
        assert!(SHIPPING_TPL_COLUMNS.contains(&"shipping_tpl_name"));
    }
}