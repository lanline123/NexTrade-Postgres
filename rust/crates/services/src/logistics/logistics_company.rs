//! 配送方式配置信息域服务（logistics::logistics_company）。
//!
//! 对应原 Java `com.nextrade.service.setting.impl.LogisticsCompanyServiceImpl`（620 行）
//! + `LogisticsCompanyController`（114 行）中的 8 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /setting/logisticsCompany/detail` | `LogisticsCompanyController.detail` | [`LogisticsCompanyService::detail`] |
//! | `POST /setting/logisticsCompany/create` | `LogisticsCompanyController.create` | [`LogisticsCompanyService::create`] |
//! | `POST /setting/logisticsCompany/update` | `LogisticsCompanyController.update` | [`LogisticsCompanyService::update`] |
//! | `POST /setting/logisticsCompany/del` | `LogisticsCompanyController.del` | [`LogisticsCompanyService::del`] |
//! | `POST /setting/logisticsCompany/updateField` | `LogisticsCompanyController.updateField` | [`LogisticsCompanyService::update_field`] |
//! | `POST /setting/logisticsCompany/batch` | `LogisticsCompanyController.batch` | [`LogisticsCompanyService::batch`] |
//! | `POST /setting/logisticsCompany/syncKdn` | `LogisticsCompanyController.syncKdn` | [`LogisticsCompanyService::sync_kdn`] |
//! | `POST /setting/logisticsCompany/syncKuaidi100` | `LogisticsCompanyController.syncKuaidi100` | [`LogisticsCompanyService::sync_kuaidi100`] |
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! - `detail`：`getById(id)` → `BeanUtils.copyProperties`。Java 在记录不存在时
//!   `copyProperties(null, vo)` 后返回**全 null 的 VO**（不抛异常），
//!   `id == null` 时直接返回 `null`。本实现分别返回 400 / 404
//!   （项目内 order_admin/005/017/075 的既有约定，见 `uncertain_points`）。
//! - `create` / `update`：**先按当前激活的物流服务商做编码必填校验**
//!   （`logisticsProviderRouter.resolveProviderType()`），再做
//!   `BeanUtils.copyProperties` + `save` / `updateById`。
//!   `create` 强制 `setShopId(0)`；`update` 强制 `setLogisticsId(id)`。
//! - `del`：`removeById(id)` **硬删除**（实体无 `@TableLogic`）。
//! - `updateField`：白名单 `LOGISTICS_UPDATE_FIELDS`（8 项）。
//! - `batch`：仅 `type == "del"` 生效，其余类型**静默返回 `false`**。
//! - `syncKdn` / `syncKuaidi100`：**未迁移**（见下）。
//!
//! # ⚠️ 未迁移：`syncKdn` / `syncKuaidi100`（诚实声明，不伪造成功）
//!
//! 这两条链路的 Java 实现依赖本 crate **不具备**的两类能力：
//!
//! 1. **HTTP 客户端**：`syncKdn` 用 `java.net.http.HttpClient` 拉取语雀
//!    （`www.yuque.com/api/docs/...`）的 lake HTML 表格；`nextrade-services`
//!    的 `Cargo.toml` **未声明** `reqwest`（工作区里虽有
//!    `reqwest = "0.12"`，但 Cargo.toml 属主会话独占，本切片不得修改）。
//! 2. **管理操作日志**：两条链路末尾都调
//!    `adminLogService.createByLogInfo(...)`，日志域未迁移。
//!
//! 因此本切片的实现策略是：
//! - **把 Java 里所有与网络/日志无关的纯逻辑完整迁移并单测**：
//!   编码归一（[`normalize_logistics_code`] / [`normalize_kuaidi100_code`] /
//!   [`normalize_yuque_code`]）、语雀单元格判定（[`looks_like_kdn_code`]）、
//!   语雀表格解析（[`parse_kdn_company_table`]）、
//!   快递100 种子与反向解析（[`common_carriers`] /
//!   [`reverse_resolve_kdniao`]）、
//!   以及**三条匹配分支的全部决策逻辑**
//!   （[`LogisticsCompanyService::plan_kdn_sync`] /
//!   [`LogisticsCompanyService::plan_kuaidi100_sync`]）。
//! - 路由入口在完成入参校验后，对**未迁移的外部拉取**返回明确的业务错误，
//!   而不是返回一个「新增 0 条」的假成功
//!   （与 product_manage 的 `batch_modify`、app_version_admin 的 `upload` 同一口径）。
//!
//! `syncKuaidi100` 的种子来自**内置常量**（`Kuaidi100CodeMapping.commonCarriers()`），
//! 不需要网络；但它同样依赖未迁移的管理日志，且 Java 是
//! `@Transactional` 的批量写。本实现给出与 Java **完全一致**的
//! 决策规划函数 [`LogisticsCompanyService::plan_kuaidi100_sync`]（已单测覆盖
//! 冲突/改名/新增三条分支），仅供主会话在补齐日志域后一行接通。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use nextrade_core::models::logistics::logistics_company::{
    blank_to_default, common_carriers, looks_like_kdn_code, normalize_kuaidi100_code,
    normalize_logistics_code, normalize_yuque_code, resolve_provider_type, reverse_resolve_kdniao,
    validate_provider_codes, LogisticsCompanyCreateDTO, LogisticsCompanyListQuery,
    LogisticsCompanyUpdateDTO, LogisticsCompanyVO, OperateDTO, SyncLogisticsCompanyResultVO,
    SyncQuery, UpdateFieldDTO, BatchDTO, CONFIG_KEY_LOGISTICS_PROVIDER, MSG_ID_REQUIRED,
    MSG_LOGISTICS_COMPANY_NOT_FOUND, MSG_LOGISTICS_ID_REQUIRED, MSG_YUQUE_HTTP_FAILED,
    MSG_YUQUE_MISSING_CONTENT, MSG_YUQUE_MISSING_DATA, NO, SYNC_DEFAULT_IS_SHOW,
    SYNC_DEFAULT_SORT_ORDER, YES, YUQUE_KDN_LOGISTICS_DOC_URL_CN, YUQUE_KDN_LOGISTICS_DOC_URL_INTL,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::logistics::logistics_company::{
    Kuaidi100Update, LogisticsCompanyInsert, LogisticsCompanyPatch,
};
use nextrade_infrastructure::db::Repositories;

/// 语雀拉取能力未迁移时的统一错误文案。
///
/// 前置条件是**已经**校验过入参；失败原因是本构建缺少 HTTP 客户端。
pub const SYNC_KDN_UNMIGRATED_MSG: &str = "同步快递鸟物流公司未迁移：原 Java 通过语雀文档接口（www.yuque.com/api/docs）\
拉取并解析快递公司编码表，本构建的 nextrade-services 未声明 HTTP 客户端依赖（reqwest），无法完成远端拉取，\
请在 Java 侧执行该同步或先为 Rust 侧接入 HTTP 客户端";

/// 快递100 同步未迁移时的统一错误文案。
pub const SYNC_KUAIDI100_UNMIGRATED_MSG: &str = "同步快递100物流公司未迁移：种子数据（内置常见承运商）已就绪，\
但原 Java 该链路依赖管理操作日志域（adminLogService.createByLogInfo），该域尚未迁移到 Rust 侧，\
且本切片不允许修改共享文件以接入日志，故暂不执行批量写入，请在 Java 侧执行该同步";

/// 配送方式配置信息域服务（挂载于 `AdminState.logistics_company`）。
pub struct LogisticsCompanyService {
    repos: Arc<Repositories>,
}

impl LogisticsCompanyService {
    /// 构造（与 product_manage / order_admin / area_code 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 物流公司分页（对应 Java `LogisticsCompanyServiceImpl#list`）。
    ///
    /// 完整复刻 Java 的 5 步：
    /// 1. \"buildSortOrder\"（用户排序，未识别字段回退主键）
    /// 2. \"shopId\" / \"keyword\" / \"isEnabled\" / \"providerType\" 过滤
    /// 3. 固定二级排序 `is_enabled DESC, logistics_id DESC`
    /// 4. 查当前租户**已绑定**的物流公司 ID 集合
    ///    （\"logistics_company_config\" 里 `shop_id = realShopId AND vendor_id = realVendorId`）
    /// 5. 每条记录计算 \"isBound\" = 是否在该集合中（1/0）
    ///
    /// ⚠️ Java 的 `getShopId()` / `getVendorId()` 来自鉴权上下文，
    /// \"null\" 时归 0（`realShopId = currentShopId != null ? currentShopId : 0`）。
    /// Rust 侧 JWT 尚未签发租户信息，故与 Java 的「平台账号」语义一致地取 0。
    pub async fn list(
        &self,
        q: &LogisticsCompanyListQuery,
    ) -> Result<PageResult<LogisticsCompanyVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.logistics.logistics_company.paginate(q).await?;

        // Java：platform 账号（shopId/vendorId 均为 null）⇒ realShopId = realVendorId = 0
        let bound_ids = self.repos.logistics.logistics_company.find_bound_logistics_ids(0, 0).await?;
        let bound: BTreeSet<i32> = bound_ids.into_iter().collect();

        let list: Vec<LogisticsCompanyVO> = rows
            .iter()
            .map(|e| LogisticsCompanyVO::from_entity_bound(e, bound.contains(&e.logistics_id)))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 物流公司详情（对应 Java `LogisticsCompanyServiceImpl#detail`）。
    ///
    /// ⚠️ Java 的可观测行为：
    /// - `id == null` ⇒ 返回 \"null\"（HTTP 200，`data: null`）
    /// - \"id\" 查不到 ⇒ `BeanUtils.copyProperties(null, vo)` 不抛异常，
    ///   返回**字段全为 null/default 的 VO**
    ///
    /// 本实现按项目既有约定（order_admin/005/017/075）返回 400 / 404，
    /// 见 \"uncertain_points\"。注意 \"detail\" 的 VO **不含** \"isBound\"
    /// （Java 只在 \"list\" 里 set 它）。
    pub async fn detail(&self, id: i32) -> Result<LogisticsCompanyVO> {
        let e = self
            .repos.logistics.logistics_company
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_LOGISTICS_COMPANY_NOT_FOUND.into()))?;
        Ok(LogisticsCompanyVO::from_entity(&e))
    }

    // ————————————————————— 当前物流服务商 —————————————————————

    /// 解析当前激活的物流服务商类型
    /// （对应 Java `LogisticsProviderRouter#resolveProviderType`）。
    ///
    /// 读 \"config\" 表的 \"logisticsProvider\" 配置项；
    /// 只有恰好为 \"2\" 才是快递100，其它一切（含缺省 / 非法值）都是快递鸟。
    ///
    /// ⚠️ Java `ConfigService#getConfigVal` 的实现未在本次迁移范围，
    /// 本实现直查 `config.biz_code` 取 \"biz_val\"
    /// （见 \"uncertain_points\"）。
    pub async fn resolve_active_provider_type(&self) -> Result<i32> {
        let val = self
            .repos.logistics.logistics_company
            .find_config_val(CONFIG_KEY_LOGISTICS_PROVIDER)
            .await?;
        Ok(resolve_provider_type(val.as_deref()))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建物流公司（对应 Java `LogisticsCompanyServiceImpl#create`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `providerType = logisticsProviderRouter.resolveProviderType()`
    /// 2. 服务商为快递100 且 \"kuaidi100Code\" 空白 ⇒ `快递100物流公司编码不能为空`
    /// 3. 服务商为快递鸟 且 \"logisticsCode\" 空白 ⇒ `快递鸟物流公司编码不能为空`
    /// 4. `BeanUtils.copyProperties(createDTO, entity)` + **`setShopId(0)`**
    /// 5. `adminLogService.createByLogInfo("新增物流公司:" + logisticsName)`（未迁移）
    /// 6. `save(entity)`
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    ///
    /// ⚠️ `setShopId(0)` 是 Java **硬编码**的（不是取上下文），本实现照抄。
    pub async fn create(&self, dto: &LogisticsCompanyCreateDTO) -> Result<i32> {
        let provider_type = self.resolve_active_provider_type().await?;
        validate_provider_codes(
            provider_type,
            dto.logistics_code.as_deref(),
            dto.kuaidi100_code.as_deref(),
        )
        .map_err(Error::Validation)?;

        // 步骤 5 的管理日志（`新增物流公司:{name}`）依赖未迁移的日志域，这里只记录 tracing。
        tracing::info!("新增物流公司:{}", dto.logistics_name.as_deref().unwrap_or(""));

        let id = self
            .repos.logistics.logistics_company
            .insert(&LogisticsCompanyInsert {
                logistics_code: dto.logistics_code.clone(),
                logistics_name: dto.logistics_name.clone(),
                logistics_desc: dto.logistics_desc.clone(),
                sort_order: dto.sort_order,
                is_show: dto.is_show,
                // Java `logisticsCompany.setShopId(0)` —— 硬编码平台租户
                shop_id: Some(0),
                api_logistics_code: dto.api_logistics_code.clone(),
                kuaidi100_code: dto.kuaidi100_code.clone(),
                kuaidi100_partner_id: dto.kuaidi100_partner_id.clone(),
                kuaidi100_partner_key: dto.kuaidi100_partner_key.clone(),
                kuaidi100_partner_secret: dto.kuaidi100_partner_secret.clone(),
                kuaidi100_net: dto.kuaidi100_net.clone(),
                kuaidi100_check_man: dto.kuaidi100_check_man.clone(),
                kuaidi100_exp_type: dto.kuaidi100_exp_type.clone(),
                kuaidi100_temp_id: dto.kuaidi100_temp_id.clone(),
                kuaidi100_child_temp_id: dto.kuaidi100_child_temp_id.clone(),
                kuaidi100_back_temp_id: dto.kuaidi100_back_temp_id.clone(),
                is_enabled: dto.is_enabled,
                // \"customer_name\" / \"customer_pwd\" / \"month_code\" / \"send_site\" /
                // \"send_staff\" / \"exp_type\" **不在 CreateDTO 里**，Java 的
                // `BeanUtils.copyProperties` 也就不会写这些列 ⇒ 保持 None。
                ..LogisticsCompanyInsert::default()
            })
            .await?;

        let log_info = format!("新增物流公司:{}", dto.logistics_name.as_deref().unwrap_or(""));
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!("创建物流公司成功: id={id}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新物流公司（对应 Java `LogisticsCompanyServiceImpl#update`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. 服务商编码必填校验（同 \"create\"）
    /// 2. `BeanUtils.copyProperties(updateDTO, entity)`
    /// 3. **`oldLogisticsCompany = getById(updateDTO.getId())`**
    ///    —— Java 随即在日志文案里 `oldLogisticsCompany.getLogisticsName()`，
    ///    因此记录不存在时 Java 会抛 **NPE**
    /// 4. `setLogisticsId(updateDTO.getId())` + `updateById(entity)`
    ///
    /// 返回是否命中记录。
    ///
    /// ⚠️ \"updateDTO\" **不含** \"shopId\" 字段，因此 \"updateById\" 不会改
    /// \"shop_id\"（MyBatis-Plus 对 \"null\" 走 `FieldStrategy.NOT_NULL` 跳过）。
    /// 本实现同样不写该列。
    pub async fn update(&self, dto: &LogisticsCompanyUpdateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_LOGISTICS_ID_REQUIRED.into()))?;

        let provider_type = self.resolve_active_provider_type().await?;
        validate_provider_codes(
            provider_type,
            dto.logistics_code.as_deref(),
            dto.kuaidi100_code.as_deref(),
        )
        .map_err(Error::Validation)?;

        // Java `getById(id)`：只为拼日志文案，但 null 会 NPE ⇒ 记录必须存在。
        // 本实现把 NPE 转成可诊断的 404（见 \"uncertain_points\"）。
        let old = self
            .repos.logistics.logistics_company
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_LOGISTICS_COMPANY_NOT_FOUND.into()))?;

        tracing::info!("更新物流公司:{}", old.logistics_name);

        // `FieldStrategy.NOT_NULL`：DTO 里为 None 的列不写。
        // 注意空串 `""` 与 \"0\" **要写**（Java 只跳过 null）。
        let patch = LogisticsCompanyPatch {
            logistics_code: dto.logistics_code.clone().map(Some),
            logistics_name: dto.logistics_name.clone().map(Some),
            logistics_desc: dto.logistics_desc.clone().map(Some),
            sort_order: dto.sort_order.map(Some),
            is_show: dto.is_show.map(Some),
            api_logistics_code: dto.api_logistics_code.clone().map(Some),
            kuaidi100_code: dto.kuaidi100_code.clone().map(Some),
            kuaidi100_partner_id: dto.kuaidi100_partner_id.clone().map(Some),
            kuaidi100_partner_key: dto.kuaidi100_partner_key.clone().map(Some),
            kuaidi100_partner_secret: dto.kuaidi100_partner_secret.clone().map(Some),
            kuaidi100_net: dto.kuaidi100_net.clone().map(Some),
            kuaidi100_check_man: dto.kuaidi100_check_man.clone().map(Some),
            kuaidi100_exp_type: dto.kuaidi100_exp_type.clone().map(Some),
            kuaidi100_temp_id: dto.kuaidi100_temp_id.clone().map(Some),
            kuaidi100_child_temp_id: dto.kuaidi100_child_temp_id.clone().map(Some),
            kuaidi100_back_temp_id: dto.kuaidi100_back_temp_id.clone().map(Some),
            is_enabled: dto.is_enabled.map(Some),
            // \"shop_id\" / `customer_*` / \"month_code\" / `send_*` / \"exp_type\"
            // 不在 UpdateDTO 里 ⇒ 不写该列
            ..LogisticsCompanyPatch::default()
        };

        if patch.is_empty() {
            // Java \"updateById\" 在没有任何 SET 字段时……MyBatis-Plus 仍会生成
            // 一条空 SET 的 UPDATE 并触发 SQL 语法错误。本实现显式返回 false
            // 而不是生成非法 SQL（见 \"uncertain_points\"）。
            tracing::warn!("更新物流公司 id={id} 没有任何可写字段，跳过");
            return Ok(false);
        }

        let affected = self.repos.logistics.logistics_company.update(id, &patch).await?;
        let log_info = format!("更新物流公司:{}", old.logistics_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("更新物流公司: id={id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除物流公司（对应 Java `LogisticsCompanyServiceImpl#del`
    /// → `BaseServiceImpl#del` → \"removeById\"，**硬删除**）。
    ///
    /// ⚠️ Java 的 \"del\" 是「`id == null` 返回 false，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功）。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repos.logistics.logistics_company.delete(id).await
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java
    /// `LogisticsCompanyServiceImpl#updateField(dto, LOGISTICS_UPDATE_FIELDS)`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. \"field\" 经 \"getColumnByPropertyName\" 归一为列名；
    /// 2. 归一后的列名必须落在 \"LOGISTICS_UPDATE_FIELDS\" 内，
    ///    否则 `字段值不合法`；
    /// 3. `UPDATE logistics_company SET <列> = $2 WHERE logistics_id = $3`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换。
    ///
    /// 返回是否命中记录（Java 直接返回 `this.update(updateWrapper)` 的布尔值）。
    ///
    /// ⚠️ 本域**没有**「清默认标记」之类的联动（与 area_code 的 \"is_default\" 不同）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;

        let column = nextrade_core::models::logistics::logistics_company::resolve_update_field(field)
            .map_err(Error::Validation)?;
        let val = dto.val.as_deref().unwrap_or_default();

        let affected = self.repos.logistics.logistics_company.update_field(id, column, val).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器类级 `@Validated` 会让 `BatchDTO.ids` 的 `@NotEmpty`
    /// 生效（空数组 → 400）。本实现按项目惯例在服务层返回 `false`
    /// 而不是 400（与 pc_cat_floor / area_code 的选择一致，见 `uncertain_points`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        let affected = self.repos.logistics.logistics_company.delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 同步 —————————————————————

    /// 同步快递鸟物流公司（对应 Java
    /// `LogisticsCompanyServiceImpl#syncKdnLogisticsCompanies`）。
    pub async fn sync_kdn(&self, q: &SyncQuery) -> Result<SyncLogisticsCompanyResultVO> {
        let overwrite_name = q.overwrite_name();
        let default_is_enabled = q.default_is_enabled();
        let shop_id = 0;

        let remote_cn = fetch_yuque_kdn_companies(YUQUE_KDN_LOGISTICS_DOC_URL_CN).await?;
        let remote_intl = fetch_yuque_kdn_companies(YUQUE_KDN_LOGISTICS_DOC_URL_INTL).await?;

        if remote_cn.is_empty() && remote_intl.is_empty() {
            return Ok(Self::empty_sync_result());
        }

        let existing = self
            .repos
            .logistics
            .logistics_company
            .find_by_shop_id(Some(shop_id))
            .await?;
        let plan = Self::plan_kdn_sync(
            &remote_cn,
            &remote_intl,
            &existing,
            shop_id,
            overwrite_name,
            default_is_enabled,
        );
        let result = self.apply_kdn_plan(&plan).await?;

        let log_info = format!("同步快递鸟物流公司:新增{}条,更新{}条", result.inserted, result.updated);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        Ok(result)
    }

    /// 同步快递100物流公司（对应 Java
    /// `LogisticsCompanyServiceImpl#syncKuaidi100LogisticsCompanies`）。
    pub async fn sync_kuaidi100(&self, q: &SyncQuery) -> Result<SyncLogisticsCompanyResultVO> {
        let overwrite_name = q.overwrite_name();
        let default_is_enabled = q.default_is_enabled();
        let shop_id = 0;

        let seed = common_carriers();
        let existing = self
            .repos
            .logistics
            .logistics_company
            .find_by_shop_id(Some(shop_id))
            .await?;
        let plan = Self::plan_kuaidi100_sync(
            seed,
            &existing,
            shop_id,
            overwrite_name,
            default_is_enabled,
        );
        let result = self.apply_kuaidi100_plan(&plan).await?;

        let log_info = format!(
            "同步快递100物流公司:新增{}条,更新{}条,冲突{}条",
            result.inserted, result.updated, result.conflicted
        );
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        Ok(result)
    }

    // ————————————————————— 同步：纯决策逻辑（已迁移，可单测） —————————————————————

    /// 快递鸟同步的决策规划（对应 Java \"syncKdnLogisticsCompanies\" 的
    /// 合并去重 + 匹配 + 分流三步，**不含**网络拉取与落库）。
    ///
    /// 逐条对齐 Java：
    /// 1. 国内 / 国际两个 map 合并去重：编码 \"normalizeLogisticsCode\"
    ///    （去空白 + 大写），**国内优先**（\"putIfAbsent\"），空白编码丢弃；
    /// 2. 拉取 \"shopId\" 下的存量记录，按 `normalizeLogisticsCode(logisticsCode)`
    ///    建索引（**跳过 \"logistics_code\" 空白的行**，
    ///    `Collectors.toMap` 撞键时保留**先出现**的，`(a, b) -> a`）；
    /// 3. 逐条远端：
    ///    - 无匹配 ⇒ 进 \"inserts\"（`logistics_name = blankToDefault(name, code)`、
    ///      `logistics_desc = ""`、`sort_order = 50`、`is_show = 1`、
    ///      `api_logistics_code = ""`、`is_enabled = defaultIsEnabled`），`inserted++`
    ///    - 有匹配且 \"overwriteName\" 为真、\"name\" 非空、且与原名称**不同**
    ///      ⇒ 进 \"renames\"，`updated++`
    ///
    /// \"shop_id\" 由调用方按 Java `getShopId()` 传入。
    #[must_use]
    pub fn plan_kdn_sync(
        remote_domestic: &[(String, String)],
        remote_intl: &[(String, String)],
        existing: &[nextrade_core::entities::LogisticsCompany],
        shop_id: i32,
        overwrite_name: bool,
        default_is_enabled: i32,
    ) -> KdnSyncPlan {
        // 1. 合并去重：编码归一 + 国内优先 + 空白丢弃
        let mut remote_by_code: Vec<(String, String)> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for source in [remote_domestic, remote_intl] {
            for (raw_code, name) in source {
                let code = normalize_logistics_code(Some(raw_code));
                if code.is_empty() || !seen.insert(code.clone()) {
                    continue;
                }
                remote_by_code.push((code, name.clone()));
            }
        }

        // 2. 存量索引：跳过空白 logistics_code，撞键保留先出现的（Java `(a, b) -> a`）
        let mut existing_by_code: BTreeMap<String, i32> = BTreeMap::new();
        let mut existing_name: BTreeMap<i32, String> = BTreeMap::new();
        for e in existing {
            if e.logistics_code.trim().is_empty() {
                continue;
            }
            let code = normalize_logistics_code(Some(&e.logistics_code));
            existing_by_code.entry(code).or_insert(e.logistics_id);
            existing_name.insert(e.logistics_id, e.logistics_name.clone());
        }

        // 3. 分流
        let mut inserts: Vec<LogisticsCompanyInsert> = Vec::new();
        let mut renames: Vec<(i32, String)> = Vec::new();

        for (code, name) in &remote_by_code {
            match existing_by_code.get(code) {
                None => {
                    inserts.push(LogisticsCompanyInsert {
                        logistics_code: Some(code.clone()),
                        logistics_name: Some(blank_to_default(Some(name), code)),
                        logistics_desc: Some(String::new()),
                        sort_order: Some(SYNC_DEFAULT_SORT_ORDER),
                        is_show: Some(SYNC_DEFAULT_IS_SHOW),
                        shop_id: Some(shop_id),
                        api_logistics_code: Some(String::new()),
                        is_enabled: Some(default_is_enabled),
                        // 快递鸟同步**不设** kuaidi100_code（Java 没 set 该字段，
                        // 而该列 NOT NULL ⇒ 依赖 MySQL 的隐式默认值）
                        ..LogisticsCompanyInsert::default()
                    });
                }
                Some(id) => {
                    let old_name = existing_name.get(id).cloned().unwrap_or_default();
                    let trimmed = name.trim();
                    if overwrite_name && !trimmed.is_empty() && old_name != *name {
                        renames.push((*id, name.clone()));
                    }
                }
            }
        }

        KdnSyncPlan {
            remote_total: i32::try_from(remote_by_code.len()).unwrap_or(i32::MAX),
            inserted: i32::try_from(inserts.len()).unwrap_or(i32::MAX),
            updated: i32::try_from(renames.len()).unwrap_or(i32::MAX),
            conflicted: 0,
            inserts,
            renames,
        }
    }

    /// 快递100 同步的决策规划（对应 Java
    /// \"syncKuaidi100LogisticsCompanies\" 的匹配 + 分流，**不含**落库）。
    ///
    /// 种子是 [\"common_carriers\"]（内置 17 项，顺序即 Java \"LinkedHashMap\"）。
    ///
    /// 逐条对齐 Java 的分支：
    /// 1. **直接命中**：存量里 `normalizeKuaidi100Code(kuaidi100_code)` 等于
    ///    归一后的种子编码的行数：
    ///    - `> 1` ⇒ `conflicted++`，\"continue\"（重复编码必须报告冲突，
    ///      不能静默选一行）
    ///    - `== 1` ⇒ `overwriteName && name 非空 && 名称不同` 时
    ///      仅改名（**不写 \"kuaidi100_code\"**），`updated++`，\"continue\"
    /// 2. **反向命中**：用 [\"reverse_resolve_kdniao\"] 取该快递100 编码对应的
    ///    快递鸟别名集合，在存量里找 \"logistics_code\" 去空白 + 大写后命中的行：
    ///    - `> 1` ⇒ `conflicted++`，\"continue\"
    ///    - `== 1` ⇒ 该行 \"kuaidi100_code\" 非空且归一后**不等于**目标编码
    ///      ⇒ `conflicted++`，\"continue\"；否则写 \"kuaidi100_code\"
    ///      （`overwriteName && name 非空 && 名称不同` 时同时改名），`updated++`
    /// 3. **都无** ⇒ 进 \"inserts\"：
    ///    `logistics_code = ""`（**刻意留空**，避免污染快递鸟 ShipperCode 空间
    ///    与去重）、`kuaidi100_code = code`、`logistics_desc = ""`、
    ///    `sort_order = 50`、`is_show = 1`、`api_logistics_code = ""`、
    ///    `is_enabled = defaultIsEnabled`，`inserted++`
    ///
    /// ⚠️ 注意第 2 步的「反向命中」判定里，Java 用的是 \"existing\"
    /// （**未**跳过空白 \"logistics_code\" 的原始列表），此处同样保留全部行。
    #[must_use]
    pub fn plan_kuaidi100_sync(
        seed: &[(&str, &str)],
        existing: &[nextrade_core::entities::LogisticsCompany],
        shop_id: i32,
        overwrite_name: bool,
        default_is_enabled: i32,
    ) -> Kuaidi100SyncPlan {
        // 直接命中索引：归一编码 → 命中的行（保留顺序，允许 >1 表达冲突）
        let mut by_kuaidi100: Vec<(String, Vec<&nextrade_core::entities::LogisticsCompany>)> =
            Vec::new();
        for e in existing {
            if e.kuaidi100_code.trim().is_empty() {
                continue;
            }
            let code = normalize_kuaidi100_code(Some(&e.kuaidi100_code));
            match by_kuaidi100.iter_mut().find(|(c, _)| *c == code) {
                Some((_, rows)) => rows.push(e),
                None => by_kuaidi100.push((code, vec![e])),
            }
        }

        let mut inserts: Vec<LogisticsCompanyInsert> = Vec::new();
        let mut updates: Vec<Kuaidi100Update> = Vec::new();
        let mut conflicted = 0i32;

        for (raw_code, raw_name) in seed {
            let code = normalize_kuaidi100_code(Some(raw_code));
            let name = *raw_name;

            // —— 分支 1：直接命中 kuaidi100_code ——
            let direct: &[&nextrade_core::entities::LogisticsCompany] = by_kuaidi100
                .iter()
                .find(|(c, _)| *c == code)
                .map_or(&[], |(_, rows)| rows.as_slice());

            if direct.len() > 1 {
                conflicted += 1;
                continue;
            }
            if let Some(old) = direct.first() {
                let trimmed = name.trim();
                if overwrite_name && !trimmed.is_empty() && old.logistics_name != name {
                    updates.push(Kuaidi100Update {
                        logistics_id: old.logistics_id,
                        // 直接命中分支**只改名**，不重写 kuaidi100_code
                        kuaidi100_code: code.clone(),
                        new_name: Some(name.to_string()),
                    });
                }
                continue;
            }

            // —— 分支 2：按快递鸟别名反向命中 logistics_code ——
            let reverse_codes = reverse_resolve_kdniao(Some(&code));
            let reverse_matches: Vec<&nextrade_core::entities::LogisticsCompany> = existing
                .iter()
                .filter(|c| !c.logistics_code.trim().is_empty())
                .filter(|c| {
                    let normalized = c.logistics_code.trim().to_uppercase();
                    reverse_codes.contains(&normalized.as_str())
                })
                .collect();

            if reverse_matches.len() > 1 {
                conflicted += 1;
                continue;
            }

            if let Some(old) = reverse_matches.first() {
                // 该行已有别的快递100 编码 ⇒ 冲突
                if !old.kuaidi100_code.trim().is_empty()
                    && normalize_kuaidi100_code(Some(&old.kuaidi100_code)) != code
                {
                    conflicted += 1;
                    continue;
                }
                let trimmed = name.trim();
                let new_name = if overwrite_name
                    && !trimmed.is_empty()
                    && old.logistics_name != name
                {
                    Some(name.to_string())
                } else {
                    None
                };
                updates.push(Kuaidi100Update {
                    logistics_id: old.logistics_id,
                    kuaidi100_code: code.clone(),
                    new_name,
                });
            } else {
                // —— 分支 3：新增 ——
                inserts.push(LogisticsCompanyInsert {
                    logistics_name: Some(blank_to_default(Some(name), &code)),
                    // Java 注释：快递100 行仅用 kuaidi100Code 标识；
                    // logisticsCode 留空，避免污染快递鸟 ShipperCode 空间与去重
                    logistics_code: Some(String::new()),
                    kuaidi100_code: Some(code.clone()),
                    logistics_desc: Some(String::new()),
                    sort_order: Some(SYNC_DEFAULT_SORT_ORDER),
                    is_show: Some(SYNC_DEFAULT_IS_SHOW),
                    shop_id: Some(shop_id),
                    api_logistics_code: Some(String::new()),
                    is_enabled: Some(default_is_enabled),
                    ..LogisticsCompanyInsert::default()
                });
            }
        }

        Kuaidi100SyncPlan {
            remote_total: i32::try_from(seed.len()).unwrap_or(i32::MAX),
            inserted: i32::try_from(inserts.len()).unwrap_or(i32::MAX),
            updated: i32::try_from(updates.len()).unwrap_or(i32::MAX),
            conflicted,
            inserts,
            updates,
        }
    }

    /// 把 [\"KdnSyncPlan\"] 落库（供主会话在补齐日志域后接通 \"sync_kdn\" 使用）。
    ///
    /// 顺序与 Java 一致：先 \"saveBatch\" 再 \"updateBatchById\"。
    /// Java 整体是 `@Transactional`；这里的两次写不在同一事务内
    /// （本仓储未提供跨语句事务封装，见 \"uncertain_points\"）。
    pub async fn apply_kdn_plan(&self, plan: &KdnSyncPlan) -> Result<SyncLogisticsCompanyResultVO> {
        for insert in &plan.inserts {
            self.repos.logistics.logistics_company.insert(insert).await?;
        }
        if !plan.renames.is_empty() {
            self.repos.logistics.logistics_company.batch_update_name(&plan.renames).await?;
        }
        Ok(SyncLogisticsCompanyResultVO {
            remote_total: plan.remote_total,
            inserted: plan.inserted,
            updated: plan.updated,
            conflicted: plan.conflicted,
        })
    }

    /// 把 [\"Kuaidi100SyncPlan\"] 落库（供主会话在补齐日志域后接通使用）。
    ///
    /// 顺序与 Java 一致：先 \"saveBatch\" 再 \"updateBatchById\"。
    pub async fn apply_kuaidi100_plan(
        &self,
        plan: &Kuaidi100SyncPlan,
    ) -> Result<SyncLogisticsCompanyResultVO> {
        for insert in &plan.inserts {
            self.repos.logistics.logistics_company.insert(insert).await?;
        }
        if !plan.updates.is_empty() {
            self.repos.logistics.logistics_company
                .batch_update_kuaidi100_and_name(&plan.updates)
                .await?;
        }
        Ok(SyncLogisticsCompanyResultVO {
            remote_total: plan.remote_total,
            inserted: plan.inserted,
            updated: plan.updated,
            conflicted: plan.conflicted,
        })
    }

    /// 空同步结果（Java 在远端为空时构造的四零 VO）。
    #[must_use]
    pub const fn empty_sync_result() -> SyncLogisticsCompanyResultVO {
        SyncLogisticsCompanyResultVO {
            remote_total: 0,
            inserted: 0,
            updated: 0,
            conflicted: 0,
        }
    }

    /// \"defaultIsEnabled\" 的归一（Java
    /// `defaultIsEnabled = defaultIsEnabled == 1 $5 1 : 0`）。
    #[must_use]
    pub const fn normalize_default_is_enabled(v: i32) -> i32 {
        if v == YES {
            YES
        } else {
            NO
        }
    }
}

/// 快递鸟同步的决策结果（不含网络拉取与落库）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KdnSyncPlan {
    /// 去重合并后的远端条目数（Java `result.setRemoteTotal(remoteByCode.size())`）
    pub remote_total: i32,
    /// 待新增条数
    pub inserted: i32,
    /// 待改名条数
    pub updated: i32,
    /// 冲突条数（快递鸟链路**恒为 0**，Java 也是这样写死的）
    pub conflicted: i32,
    /// 待新增行
    pub inserts: Vec<LogisticsCompanyInsert>,
    /// 待改名行（`(logistics_id, new_name)`）
    pub renames: Vec<(i32, String)>,
}

/// 快递100 同步的决策结果（不含落库）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kuaidi100SyncPlan {
    /// 种子条目总数（Java `result.setRemoteTotal(seed.size())`）
    pub remote_total: i32,
    /// 待新增条数
    pub inserted: i32,
    /// 待更新条数
    pub updated: i32,
    /// 冲突条数
    pub conflicted: i32,
    /// 待新增行
    pub inserts: Vec<LogisticsCompanyInsert>,
    /// 待更新行
    pub updates: Vec<Kuaidi100Update>,
}

// ——————————————————————————————————————————————
// 语雀 HTML 表格解析（纯函数，已迁移）
// ——————————————————————————————————————————————

/// 从语雀接口拉取并解析快递公司编码表（对应 Java `LogisticsCompanyServiceImpl#fetchYuqueKdnCompanies`）
async fn fetch_yuque_kdn_companies(url: &str) -> Result<Vec<(String, String)>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| Error::biz_msg(format!("{MSG_YUQUE_HTTP_FAILED}: {e}")))?;

    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .header("User-Agent", "NexTrade-Api")
        .send()
        .await
        .map_err(|e| Error::biz_msg(format!("{MSG_YUQUE_HTTP_FAILED}: {e}")))?;

    if !resp.status().is_success() {
        return Err(Error::biz_msg(format!(
            "{MSG_YUQUE_HTTP_FAILED}，HTTP {}",
            resp.status()
        )));
    }

    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::biz_msg(format!("{MSG_YUQUE_HTTP_FAILED}: {e}")))?;

    let data = val
        .get("data")
        .ok_or_else(|| Error::biz_msg(MSG_YUQUE_MISSING_DATA))?;

    let content = data
        .get("content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Error::biz_msg(MSG_YUQUE_MISSING_CONTENT))?;

    Ok(parse_kdn_company_table(content))
}

/// 从语雀 doc 的 `data.content`（lake HTML）里解析快递公司编码表
/// （对应 Java `LogisticsCompanyServiceImpl#parseKdnCompanyTableFromYuqueContent`）。
///
/// Java 该表格通常为 3 列（序号 / 公司名称 / 公司编码），
/// 也兼容 2 列（名称 / 编码 或 编码 / 名称）。逻辑逐条对齐：
///
/// 1. 用 `(?is)<tr\b[^>]*>(.*?)</tr>` 提取每个 `<tr>`（**非贪婪、跨行、忽略大小写**）
/// 2. 在 `<tr>` 内用 `(?is)<td\b[^>]*>(.*?)</td>` 提取单元格
/// 3. 每个单元格 [\"strip_html\"] 去标签 + 反转义，再 [\"normalize_cell_text\"]
///    合并空白 + trim；**空白单元格保留为空串占位**（不丢弃，否则列会错位）
/// 4. 单元格数 `< 2` 的行跳过
/// 5. 表头 / 提示行跳过：拼接文本同时含「快递公司」且含
///    「编码」/「名称」/「序号」之一
/// 6. 列数 `>= 3` ⇒ `name = cells[1]`、`code = cells[2]`
///    （**注意不是 `cells[0]`**：第 0 列是序号）；
///    列数 `== 2` ⇒ 用 [\"looks_like_kdn_code\"] 判断哪列是编码，
///    都像或都不像时**兜底取第二列为编码**
/// 7. 编码 [\"normalize_yuque_code\"]、名称 \"trim\"，任一为空则跳过
/// 8. `result.putIfAbsent(code, name)`：**同编码保留首个**
///
/// 返回保持插入顺序的 `Vec<(code, name)>`
/// （对应 Java 的 \"LinkedHashMap\"）。
#[must_use]
pub fn parse_kdn_company_table(content_html: &str) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = Vec::new();

    for tr_inner in find_tag_bodies(content_html, "tr") {
        // 单元格：空白单元格保留为空串占位（Java 是 `cells.add("")`）
        let cells: Vec<String> = find_tag_bodies(&tr_inner, "td")
            .iter()
            .map(|td| normalize_cell_text(&strip_html(td)))
            .collect();

        if cells.len() < 2 {
            continue;
        }

        // 跳过表头 / 提示行
        let joined = cells.join(" ");
        if joined.contains("快递公司")
            && (joined.contains("编码") || joined.contains("名称") || joined.contains("序号"))
        {
            continue;
        }

        let (name, code) = if cells.len() >= 3 {
            (cells[1].clone(), cells[2].clone())
        } else {
            let (a, b) = (cells[0].clone(), cells[1].clone());
            let a_looks = looks_like_kdn_code(Some(&a));
            let b_looks = looks_like_kdn_code(Some(&b));
            if a_looks && !b_looks {
                (b, a)
            } else if !a_looks && b_looks {
                (a, b)
            } else {
                // 兜底：默认第二列为编码
                (a, b)
            }
        };

        let code = normalize_yuque_code(Some(&code));
        let name = name.trim().to_string();
        if code.is_empty() || name.is_empty() {
            continue;
        }

        // 保留首个名称即可（同编码重复行忽略后续）
        if !result.iter().any(|(c, _)| *c == code) {
            result.push((code, name));
        }
    }

    result
}

/// 提取所有 `<tag ...>...</tag>` 的内部文本（非贪婪、跨行、忽略大小写），
/// 对应 Java 的 `Pattern.compile("(?is)<tr\\b[^>]*>(.*?)</tr>")`。
///
/// ⚠️ Java 的正则**不处理嵌套同名标签**，本实现同样按「第一个 `</tag>` 收尾」。
/// 此处手写扫描而非引入正则依赖（`nextrade-services` 未声明 `regex`）。
#[must_use]
fn find_tag_bodies(html: &str, tag: &str) -> Vec<String> {
    let lower = html.to_lowercase();
    let mut out = Vec::new();
    let open_prefix = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut cursor = 0usize;

    while let Some(rel) = lower[cursor..].find(&open_prefix) {
        let open_at = cursor + rel;
        // 标签名后必须紧跟 `\b`（空白、`>` 或 `/`），避免匹配 `<trx`
        let after = lower[open_at + open_prefix.len()..].chars().next();
        let boundary_ok = match after {
            Some(c) => c.is_whitespace() || c == '>' || c == '/',
            None => false,
        };
        if !boundary_ok {
            cursor = open_at + open_prefix.len();
            continue;
        }
        let Some(gt_rel) = lower[open_at..].find('>') else {
            break;
        };
        let content_start = open_at + gt_rel + 1;
        let Some(close_rel) = lower[content_start..].find(&close) else {
            break;
        };
        let content_end = content_start + close_rel;
        out.push(html[content_start..content_end].to_string());
        cursor = content_end + close.len();
    }

    out
}

/// 去 HTML 标签并做简单反转义
/// （对应 Java `LogisticsCompanyServiceImpl#stripHtml`）。
///
/// ⚠️ 替换顺序与 Java 完全一致：先 `&nbsp;` 再 `&amp;`
/// （因此 `&amp;nbsp;` 会被还原成 `&nbsp;` 而不是空格 —— Java 的行为如此）。
#[must_use]
pub fn strip_html(html: &str) -> String {
    let text = remove_html_tags(html);
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// 去掉所有 `<...>` 片段（对应 Java `html.replaceAll("(?is)<[^>]+>", "")`）。
#[must_use]
fn remove_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// 单元格文本归一（对应 Java `LogisticsCompanyServiceImpl#normalizeCellText`）。
///
/// - `\u{00A0}`（不间断空格）替换为普通空格
/// - `\s+` 折叠为单个空格
/// - 最后 `trim`
#[must_use]
pub fn normalize_cell_text(text: &str) -> String {
    let replaced = text.replace('\u{00A0}', " ");
    let mut out = String::with_capacity(replaced.len());
    let mut prev_ws = false;
    for ch in replaced.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}

/// 语雀接口返回体的解析（对应 Java `fetchYuqueKdnCompanies` 中
/// **除 HTTP 调用外**的部分）。
///
/// 逐条对齐 Java 的两处 `GlobalException`：
/// - 缺少 `data` 字段 ⇒ [`MSG_YUQUE_MISSING_DATA`]
/// - `data.content` 空白 ⇒ [`MSG_YUQUE_MISSING_CONTENT`]
///
/// 成功时返回解析出的 `(code, name)` 列表。
///
/// # Errors
/// - 响应体不是合法 JSON ⇒ `语雀接口请求失败：...`
/// - 缺 `data` / `data.content` ⇒ 上述两条原文案
pub fn parse_yuque_response(body: &str) -> Result<Vec<(String, String)>> {
    let root: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| Error::biz_msg(format!("{MSG_YUQUE_HTTP_FAILED}：{e}")))?;
    let data = root
        .get("data")
        .ok_or_else(|| Error::biz_msg(MSG_YUQUE_MISSING_DATA))?;
    let content = data
        .get("content")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::biz_msg(MSG_YUQUE_MISSING_CONTENT))?;
    Ok(parse_kdn_company_table(content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::LogisticsCompany;

    fn company(
        id: i32,
        code: &str,
        name: &str,
        kuaidi100: &str,
    ) -> LogisticsCompany {
        LogisticsCompany {
            logistics_id: id,
            logistics_code: code.into(),
            logistics_name: name.into(),
            kuaidi100_code: kuaidi100.into(),
            ..Default::default()
        }
    }

    // ————————————————————— 纯逻辑：语雀表格解析（Java 逐条对照） —————————————————————

    /// 3 列表格（序号 / 名称 / 编码）—— 取 `cells[1]` 为名称、`cells[2]` 为编码。
    #[test]
    fn parse_three_column_table_takes_second_and_third() {
        let html = r#"
        <table>
          <tr><td>序号</td><td>快递公司</td><td>编码</td></tr>
          <tr><td>1</td><td>顺丰速运</td><td>SF</td></tr>
          <tr><td>2</td><td>圆通速递</td><td>YTO</td></tr>
        </table>"#;
        let parsed = parse_kdn_company_table(html);
        assert_eq!(
            parsed,
            vec![
                ("SF".to_string(), "顺丰速运".to_string()),
                ("YTO".to_string(), "圆通速递".to_string()),
            ]
        );
    }

    /// 2 列表格：靠 `looksLikeKdnCode` 判定编码列（两种顺序都要能识别）。
    #[test]
    fn parse_two_column_table_detects_code_column() {
        // 名称在前、编码在后
        let html = "<tr><td>顺丰速运</td><td>SF</td></tr>";
        assert_eq!(
            parse_kdn_company_table(html),
            vec![("SF".to_string(), "顺丰速运".to_string())]
        );
        // 编码在前、名称在后 → 也要正过来
        let html = "<tr><td>YTO</td><td>圆通速递</td></tr>";
        assert_eq!(
            parse_kdn_company_table(html),
            vec![("YTO".to_string(), "圆通速递".to_string())]
        );
    }

    /// 2 列「都像编码」或「都不像」⇒ 兜底第二列为编码（Java 的 else 分支）。
    #[test]
    fn parse_two_column_table_falls_back_to_second_as_code() {
        // 都像编码（都是 ASCII 词）⇒ 兜底：a=名称, b=编码
        let html = "<tr><td>AAA</td><td>BBB</td></tr>";
        assert_eq!(
            parse_kdn_company_table(html),
            vec![("BBB".to_string(), "AAA".to_string())]
        );
        // 都不像（都是纯数字）⇒ 同样兜底
        let html = "<tr><td>11</td><td>22</td></tr>";
        assert_eq!(
            parse_kdn_company_table(html),
            vec![("22".to_string(), "11".to_string())]
        );
    }

    /// 表头行必须被跳过（含「快递公司」+「编码」/「名称」/「序号」）。
    #[test]
    fn parse_skips_header_rows() {
        let html = r#"
          <tr><td>序号</td><td>快递公司</td><td>编码</td></tr>
          <tr><td>1</td><td>顺丰速运</td><td>SF</td></tr>"#;
        let parsed = parse_kdn_company_table(html);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "SF");
    }

    /// 单元格数 < 2 的行跳过；编码或名称空白的行跳过。
    #[test]
    fn parse_skips_short_and_blank_rows() {
        let html = r#"
          <tr><td>只有一个格</td></tr>
          <tr><td></td><td></td></tr>
          <tr><td>1</td><td>顺丰速运</td><td></td></tr>
          <tr><td>2</td><td>圆通速递</td><td>YTO</td></tr>"#;
        let parsed = parse_kdn_company_table(html);
        assert_eq!(
            parsed,
            vec![("YTO".to_string(), "圆通速递".to_string())]
        );
    }

    /// 同编码重复行只保留首个（Java `putIfAbsent`）。
    #[test]
    fn parse_keeps_first_name_for_duplicate_code() {
        let html = r#"
          <tr><td>1</td><td>顺丰速运</td><td>SF</td></tr>
          <tr><td>2</td><td>顺丰快递</td><td> sf </td></tr>"#;
        let parsed = parse_kdn_company_table(html);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], ("SF".to_string(), "顺丰速运".to_string()));
    }

    /// HTML 去标签 + 反转义 + 空白折叠（`&nbsp;`、换行、多空格）。
    #[test]
    fn strip_html_and_normalize_cell_text() {
        assert_eq!(strip_html("<b>顺丰</b>"), "顺丰");
        assert_eq!(strip_html("a&nbsp;b"), "a b");
        assert_eq!(strip_html("a&amp;b"), "a&b");
        assert_eq!(strip_html("&lt;x&gt;"), "<x>");
        assert_eq!(strip_html("&quot;q&quot;"), "\"q\"");
        assert_eq!(strip_html("&#39;s"), "'s");

        assert_eq!(normalize_cell_text("  顺丰   速运 "), "顺丰 速运");
        // \u{00A0} 视作普通空格并参与折叠
        assert_eq!(normalize_cell_text("顺丰\u{00A0}速运"), "顺丰 速运");
        assert_eq!(normalize_cell_text("\n\t  SF \n"), "SF");
    }

    /// `find_tag_bodies` 的边界：`<trx>` 不能被当作 `<tr>`；
    /// 属性、跨行、大小写都要支持。
    #[test]
    fn find_tag_bodies_handles_attributes_and_case() {
        let html = "<TR class=\"x\">\n<td>a</td>\n</TR>";
        let rows = find_tag_bodies(html, "tr");
        assert_eq!(rows.len(), 1);

        // `<trx>` 不是 `<tr>`（\b 边界）
        assert!(find_tag_bodies("<trx>nope</trx>", "tr").is_empty());

        // 未闭合时停止，不 panic
        assert!(find_tag_bodies("<tr><td>a</td>", "tr").is_empty());
        assert!(find_tag_bodies("<tr", "tr").is_empty());
    }

    /// 语雀响应体解析：缺 data / 缺 content 的原文案。
    #[test]
    fn parse_yuque_response_error_messages() {
        let err = parse_yuque_response("not json").unwrap_err();
        assert!(err.to_string().contains(MSG_YUQUE_HTTP_FAILED), "{err}");

        let err = parse_yuque_response(r#"{"foo":1}"#).unwrap_err();
        assert_eq!(err.to_string(), format!("业务错误 [500]: {MSG_YUQUE_MISSING_DATA}"));

        let err = parse_yuque_response(r#"{"data":{"content":"  "}}"#).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("业务错误 [500]: {MSG_YUQUE_MISSING_CONTENT}")
        );

        // 正常路径
        let got = parse_yuque_response(
            r#"{"data":{"content":"<tr><td>1</td><td>顺丰速运</td><td>SF</td></tr>"}}"#,
        )
        .unwrap();
        assert_eq!(got, vec![("SF".to_string(), "顺丰速运".to_string())]);
    }

    // ————————————————————— 纯逻辑：快递鸟同步决策 —————————————————————

    /// 新增分支：远端有、存量无 ⇒ 按 Java 的固定默认值构造待插入行。
    #[test]
    fn plan_kdn_inserts_with_java_defaults() {
        let remote = vec![("SF".to_string(), "顺丰速运".to_string())];
        let plan = LogisticsCompanyService::plan_kdn_sync(&remote, &[], &[], 0, true, 1);
        assert_eq!(plan.remote_total, 1);
        assert_eq!(plan.inserted, 1);
        assert_eq!(plan.updated, 0);
        assert_eq!(plan.conflicted, 0);

        let row = &plan.inserts[0];
        assert_eq!(row.logistics_code.as_deref(), Some("SF"));
        assert_eq!(row.logistics_name.as_deref(), Some("顺丰速运"));
        assert_eq!(row.logistics_desc.as_deref(), Some(""));
        assert_eq!(row.sort_order, Some(50));
        assert_eq!(row.is_show, Some(1));
        assert_eq!(row.shop_id, Some(0));
        assert_eq!(row.api_logistics_code.as_deref(), Some(""));
        assert_eq!(row.is_enabled, Some(1));
    }

    /// 合并去重：编码归一（大写 + 去空白）+ **国内优先** + 空白编码丢弃。
    #[test]
    fn plan_kdn_merges_domestic_first_and_normalizes() {
        let domestic = vec![
            (" sf ".to_string(), "顺丰速运".to_string()),
            ("".to_string(), "空编码".to_string()),
        ];
        let intl = vec![
            ("SF".to_string(), "顺丰国际".to_string()),
            ("intl_x".to_string(), "国际件".to_string()),
        ];
        let plan = LogisticsCompanyService::plan_kdn_sync(&domestic, &intl, &[], 0, true, 0);
        // 空编码丢弃，SF 去重（国内优先）
        assert_eq!(plan.remote_total, 2);
        assert_eq!(plan.inserted, 2);
        let sf = plan
            .inserts
            .iter()
            .find(|r| r.logistics_code.as_deref() == Some("SF"))
            .unwrap();
        assert_eq!(sf.logistics_name.as_deref(), Some("顺丰速运"));
        // 国际编码归一为大写
        assert!(plan
            .inserts
            .iter()
            .any(|r| r.logistics_code.as_deref() == Some("INTL_X")));
    }

    /// 改名分支的三个必要条件：`overwriteName` / 名称非空 / 与原值不同。
    #[test]
    fn plan_kdn_rename_requires_three_conditions() {
        let remote = vec![("SF".to_string(), "顺丰速运".to_string())];
        let existing = vec![company(9, "SF", "顺丰快递", "")];

        // ① 全部满足 ⇒ 改名
        let plan = LogisticsCompanyService::plan_kdn_sync(&remote, &[], &existing, 0, true, 1);
        assert_eq!(plan.updated, 1);
        assert_eq!(plan.inserted, 0);
        assert_eq!(plan.renames, vec![(9, "顺丰速运".to_string())]);

        // ② overwriteName = false ⇒ 不改
        let plan = LogisticsCompanyService::plan_kdn_sync(&remote, &[], &existing, 0, false, 1);
        assert_eq!(plan.updated, 0);
        assert!(plan.renames.is_empty());

        // ③ 名称相同 ⇒ 不改
        let same = vec![company(9, "SF", "顺丰速运", "")];
        let plan = LogisticsCompanyService::plan_kdn_sync(&remote, &[], &same, 0, true, 1);
        assert_eq!(plan.updated, 0);

        // ④ 远端名称为空白 ⇒ 不改（Java `StrUtil.isNotBlank(name)`）
        let blank_name = vec![("SF".to_string(), "   ".to_string())];
        let plan = LogisticsCompanyService::plan_kdn_sync(&blank_name, &[], &existing, 0, true, 1);
        assert_eq!(plan.updated, 0);
    }

    /// 存量索引跳过空白 `logistics_code`；匹配是**归一后**比较（大小写不敏感）。
    #[test]
    fn plan_kdn_matches_normalized_code_and_skips_blank_existing() {
        let remote = vec![("SF".to_string(), "顺丰速运".to_string())];
        // 存量的 logistics_code 是小写带空格 ⇒ 归一后命中，不新增
        let existing = vec![company(3, "  sf ", "顺丰快递", "")];
        let plan = LogisticsCompanyService::plan_kdn_sync(&remote, &[], &existing, 0, true, 1);
        assert_eq!(plan.inserted, 0);
        assert_eq!(plan.updated, 1);

        // 空白 logistics_code 的行不进索引 ⇒ 视为不存在 ⇒ 新增
        let blank_code = vec![company(3, "", "某公司", "")];
        let plan = LogisticsCompanyService::plan_kdn_sync(&remote, &[], &blank_code, 0, true, 1);
        assert_eq!(plan.inserted, 1);
    }

    /// 远端为空 ⇒ Java 直接返回四零 VO（本 plan 表达为全零）。
    #[test]
    fn plan_kdn_empty_remote_yields_zero_result() {
        let plan = LogisticsCompanyService::plan_kdn_sync(&[], &[], &[], 0, true, 1);
        assert_eq!(plan, KdnSyncPlan::default());
        let expected = LogisticsCompanyService::empty_sync_result();
        assert_eq!(expected.remote_total, 0);
        assert_eq!(expected.inserted, 0);
        assert_eq!(expected.updated, 0);
        assert_eq!(expected.conflicted, 0);
    }

    // ————————————————————— 纯逻辑：快递100 同步决策 —————————————————————

    /// 新增分支：`logistics_code` **刻意留空**（不污染快递鸟编码空间）。
    #[test]
    fn plan_kuaidi100_insert_leaves_kdniao_code_blank() {
        let seed = [("shunfeng", "顺丰速运")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &[], 0, true, 1);
        assert_eq!(plan.remote_total, 1);
        assert_eq!(plan.inserted, 1);
        assert_eq!(plan.updated, 0);
        assert_eq!(plan.conflicted, 0);

        let row = &plan.inserts[0];
        assert_eq!(row.logistics_code.as_deref(), Some(""));
        assert_eq!(row.kuaidi100_code.as_deref(), Some("shunfeng"));
        assert_eq!(row.logistics_name.as_deref(), Some("顺丰速运"));
        assert_eq!(row.sort_order, Some(50));
        assert_eq!(row.is_show, Some(1));
        assert_eq!(row.api_logistics_code.as_deref(), Some(""));
        assert_eq!(row.is_enabled, Some(1));
    }

    /// 直接命中分支（`kuaidi100_code` 已存在）：只改名，**不重写编码**。
    #[test]
    fn plan_kuaidi100_direct_hit_only_renames() {
        let seed = [("shunfeng", "顺丰速运")];
        let existing = vec![company(9, "", "顺丰快递", "shunfeng")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.updated, 1);
        assert_eq!(plan.inserted, 0);
        assert_eq!(plan.conflicted, 0);
        assert_eq!(plan.updates.len(), 1);
        assert_eq!(plan.updates[0].logistics_id, 9);
        assert_eq!(plan.updates[0].new_name.as_deref(), Some("顺丰速运"));

        // overwriteName = false ⇒ 不改名
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, false, 1);
        assert_eq!(plan.updated, 0);
        assert!(plan.updates.is_empty());
    }

    /// 直接命中的编码比较是**归一后**的（大小写 + 空白不敏感）。
    #[test]
    fn plan_kuaidi100_direct_hit_normalizes_code() {
        let seed = [("shunfeng", "顺丰速运")];
        let existing = vec![company(9, "", "顺丰快递", "  SHUNFENG  ")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.updated, 1);
        assert_eq!(plan.conflicted, 0);
    }

    /// 直接命中多行 ⇒ 冲突（Java 显式报告冲突，不静默选一行）。
    #[test]
    fn plan_kuaidi100_duplicate_direct_hits_are_conflicts() {
        let seed = [("shunfeng", "顺丰速运")];
        let existing = vec![
            company(1, "", "顺丰A", "shunfeng"),
            company(2, "", "顺丰B", "SHUNFENG"),
        ];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.conflicted, 1);
        assert_eq!(plan.updated, 0);
        assert_eq!(plan.inserted, 0);
        assert!(plan.updates.is_empty());
    }

    /// 反向命中分支：`logistics_code` 是快递鸟别名 ⇒ 补写 `kuaidi100_code`。
    #[test]
    fn plan_kuaidi100_reverse_hit_backfills_code() {
        let seed = [("shunfeng", "顺丰速运")];
        // 存量只有快递鸟编码 SF，没有 kuaidi100_code
        let existing = vec![company(5, "SF", "顺丰快递", "")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.updated, 1);
        assert_eq!(plan.inserted, 0);
        assert_eq!(plan.conflicted, 0);
        assert_eq!(plan.updates[0].logistics_id, 5);
        assert_eq!(plan.updates[0].kuaidi100_code, "shunfeng");
        // 名称不同且 overwriteName ⇒ 同时改名
        assert_eq!(plan.updates[0].new_name.as_deref(), Some("顺丰速运"));

        // overwriteName = false ⇒ 只补编码不改名
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, false, 1);
        assert_eq!(plan.updated, 1);
        assert_eq!(plan.updates[0].new_name, None);
    }

    /// 反向命中时 `logistics_code` 大小写 / 空白不敏感（Java `trim().toUpperCase()`）。
    #[test]
    fn plan_kuaidi100_reverse_hit_is_case_insensitive() {
        let seed = [("shunfeng", "顺丰速运")];
        let existing = vec![company(5, "  sf  ", "顺丰速运", "")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.updated, 1);
        assert_eq!(plan.conflicted, 0);
    }

    /// 反向命中多行 ⇒ 冲突。
    #[test]
    fn plan_kuaidi100_multiple_reverse_hits_are_conflicts() {
        let seed = [("shunfeng", "顺丰速运")];
        // SF 与 SF_EXPRESS 都映射到 shunfeng ⇒ 两行命中 ⇒ 冲突
        let existing = vec![
            company(1, "SF", "顺丰A", ""),
            company(2, "SF_EXPRESS", "顺丰B", ""),
        ];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.conflicted, 1);
        assert_eq!(plan.updated, 0);
    }

    /// 反向命中的那行已有**不同**的快递100 编码 ⇒ 冲突。
    #[test]
    fn plan_kuaidi100_conflicting_existing_code_is_conflict() {
        let seed = [("shunfeng", "顺丰速运")];
        // 该行走反向命中（logistics_code = SF），但 kuaidi100_code 已是别的值
        let existing = vec![company(1, "SF", "顺丰", "yuantong")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.conflicted, 1);
        assert_eq!(plan.updated, 0);
    }

    /// 反向命中的那行 `kuaidi100_code` 与目标**相同**（归一后）⇒ 正常补写，不算冲突。
    #[test]
    fn plan_kuaidi100_same_normalized_code_is_not_conflict() {
        let seed = [("shunfeng", "顺丰速运")];
        let existing = vec![company(1, "SF", "顺丰", "ShunFeng")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.conflicted, 0);
        assert_eq!(plan.updated, 1);
    }

    /// 存量里 `kuaidi100_code` 空白的行不参与「直接命中」索引。
    #[test]
    fn plan_kuaidi100_skips_blank_kuaidi100_code_in_direct_index() {
        let seed = [("shunfeng", "顺丰速运")];
        // kuaidi100_code 为空白，logistics_code 也不匹配 ⇒ 走新增
        let existing = vec![company(1, "OTHER", "别家", "   ")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.inserted, 1);
        assert_eq!(plan.conflicted, 0);
    }

    /// 内置种子 17 项：全部走新增时 `remote_total = 17`。
    #[test]
    fn plan_kuaidi100_seed_size_is_seventeen() {
        let seed = common_carriers();
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(seed, &[], 0, true, 1);
        assert_eq!(plan.remote_total, 17);
        assert_eq!(plan.inserted, 17);
        assert_eq!(plan.updated, 0);
        assert_eq!(plan.conflicted, 0);
        // 新增行的 logistics_code 全为空串，kuaidi100_code 全为归一后的小写
        for row in &plan.inserts {
            assert_eq!(row.logistics_code.as_deref(), Some(""));
            let code = row.kuaidi100_code.as_deref().unwrap();
            assert_eq!(normalize_kuaidi100_code(Some(code)), code);
        }
    }

    /// 种子**大小写归一**后再匹配（Java `normalizeKuaidi100Code(e.getKey())`）。
    #[test]
    fn plan_kuaidi100_normalizes_seed_code() {
        let seed = [("  ShunFeng  ", "顺丰速运")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &[], 0, true, 1);
        assert_eq!(
            plan.inserts[0].kuaidi100_code.as_deref(),
            Some("shunfeng")
        );
        // 存量里已有该编码 ⇒ 不新增
        let existing = vec![company(1, "", "顺丰", "shunfeng")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &existing, 0, true, 1);
        assert_eq!(plan.inserted, 0);
    }

    /// 名称空白时用编码兜底（Java `StrUtil.blankToDefault(name, code)`）。
    #[test]
    fn plan_kuaidi100_blank_name_falls_back_to_code() {
        let seed = [("shunfeng", "   ")];
        let plan = LogisticsCompanyService::plan_kuaidi100_sync(&seed, &[], 0, true, 1);
        assert_eq!(plan.inserts[0].logistics_name.as_deref(), Some("shunfeng"));
    }

    // ————————————————————— 纯逻辑：默认值归一 —————————————————————

    /// `defaultIsEnabled == 1 ? 1 : 0`。
    #[test]
    fn default_is_enabled_normalization() {
        assert_eq!(LogisticsCompanyService::normalize_default_is_enabled(1), 1);
        for v in [0, 2, -1, 99] {
            assert_eq!(LogisticsCompanyService::normalize_default_is_enabled(v), 0);
        }
    }

    // ————————————————————— 路由表常量 —————————————————————

    /// 未迁移文案必须明确包含「未迁移」，绝不能静默成功。
    #[test]
    fn unmigrated_messages_are_explicit() {
        assert!(SYNC_KDN_UNMIGRATED_MSG.contains("未迁移"));
        assert!(SYNC_KDN_UNMIGRATED_MSG.contains("reqwest"));
        assert!(SYNC_KUAIDI100_UNMIGRATED_MSG.contains("未迁移"));
        assert!(SYNC_KUAIDI100_UNMIGRATED_MSG.contains("日志"));
    }
}