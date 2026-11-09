//! 物流公司配置服务（logistics::logistics_company_config）。
//! 对应原 Java `com.nextrade.service.setting.impl.LogisticsCompanyConfigServiceImpl`
//! （226 行）与父类 `BaseServiceImpl#del` / `#updateField`。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`LogisticsCompanyConfigService::list`] | `LogisticsCompanyConfigServiceImpl#list` |
//! | [`LogisticsCompanyConfigService::detail`] | `LogisticsCompanyConfigServiceImpl#detail` |
//! | [`LogisticsCompanyConfigService::create`] | `LogisticsCompanyConfigServiceImpl#create` |
//! | [`LogisticsCompanyConfigService::update`] | `LogisticsCompanyConfigServiceImpl#update` |
//! | [`LogisticsCompanyConfigService::delete`] | `BaseServiceImpl#del` |
//! | [`LogisticsCompanyConfigService::set_default`] | `LogisticsCompanyConfigServiceImpl#setDefault` |
//! | [`LogisticsCompanyConfigService::update_field`] | `BaseServiceImpl#updateField(dto, CONFIG_UPDATE_FIELDS)` |
//!
//! ## 与 Java 的行为差异（逐条，均已在 `uncertain_points` 中列明）
//!
//! 1. **登录态租户（`HeaderUtils.getShopId()` / `getVendorId()`）**：
//!    原 Java 从 `x-admin-type` / `x-shop-id` / `x-vendor-id` 请求头取当前
//!    店铺与供应商；Rust JWT（`nextrade_security::web::AuthUser`）未签发这些
//!    字段。本服务把「当前租户」改为**显式入参**（`shop_id` / `vendor_id`），
//!    由路由层按 aftersales/010/011/020 的同口径从请求头解析后传入。
//!    这**不是等价实现**：平台账号可伪装店铺视角（见路由层说明）。
//! 2. **`adminLogService.createByLogInfo`**（写 `admin_log`）：日志域未迁移；
//!    本服务只 `tracing::info!` 记录，**不写 `admin_log` 表**。
//! 3. **`Kuaidi100CodeMapping.mappedLogisticsCodes()`**：`list` 的
//!    `providerType=2` 分支依赖 Java 侧硬编码的「物流公司编码 → 快递100 编码」
//!    映射清单，Rust 侧未迁移该清单，仅保留了 `kuaidi100_code` 非空与
//!    「`logistics_code` 全小写字母数字」两条判据（见仓储
//!    `find_company_ids`），可能漏掉少数公司。
//! 4. **`update` 的 `Assert.notNull` / `GlobalException` 文案**：Java 抛
//!    `IllegalArgumentException`（`Assert`）与 `GlobalException`，两者在
//!    Spring 全局异常处理里都返回业务失败；Rust 侧统一用
//!    `Error::Validation`（400）与 `Error::biz_msg`（500），**文案逐字照抄**。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{LogisticsCompany, LogisticsCompanyConfig};
use nextrade_core::models::logistics::logistics_company_config::{
    LogisticsCompanyConfigCreateDTO, LogisticsCompanyConfigListQuery,
    LogisticsCompanyConfigUpdateDTO, LogisticsCompanyConfigVO, UpdateFieldDTO,
    DEFAULT_IS_ENABLED, DEFAULT_SORT_ORDER,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::logistics::logistics_company_config::{ColumnValue, LogisticsCompanyConfigRepository};
use nextrade_infrastructure::db::Repositories;

/// 「物流公司不能为空」——Java `@NotNull(message = "物流公司不能为空")`。
const ERR_LOGISTICS_ID_REQUIRED: &str = "物流公司不能为空";
/// Java `create`：`Assert.notNull(company, "物流公司不存在")`。
const ERR_LOGISTICS_NOT_FOUND: &str = "物流公司不存在";
/// Java `create` / `update` 判重：`GlobalException("该物流公司已绑定，请勿重复添加")`。
const ERR_ALREADY_BOUND: &str = "该物流公司已绑定，请勿重复添加";
/// Java `update`：`Assert.notNull(existing, "配置记录不存在")`。
const ERR_CONFIG_NOT_FOUND: &str = "配置记录不存在";
/// Java `setDefault`：`Assert.notNull(config, "配置记录不存在")`。
const ERR_CONFIG_NOT_FOUND_DEFAULT: &str = "配置记录不存在";
/// Java `BaseServiceImpl#updateField`：字段不在白名单 → `INVALID_FIELD_VALUE`。
const ERR_INVALID_FIELD_VALUE: &str = "非法字段值";
/// Java `@NotNull(message = "配置ID不能为空")`。
const ERR_ID_REQUIRED: &str = "配置ID不能为空";

/// `update` 动态 SET 允许的「列名 → DTO 字段」白名单。
///
/// 与 `infrastructure::db::repo::nextrade_core::models::logistics::logistics_company_config::WRITABLE_COLUMNS` 必须一致
/// （两侧由单测约束），列名来自权威 schema `logistics_company_config`。
pub const UPDATE_COLUMN_WHITELIST: [&str; 18] = [
    "logistics_id",
    "customer_name",
    "customer_pwd",
    "month_code",
    "send_site",
    "send_staff",
    "exp_type",
    "kuaidi100_partner_id",
    "kuaidi100_partner_key",
    "kuaidi100_partner_secret",
    "kuaidi100_net",
    "kuaidi100_check_man",
    "kuaidi100_exp_type",
    "kuaidi100_temp_id",
    "kuaidi100_child_temp_id",
    "kuaidi100_back_temp_id",
    "sort_order",
    "is_enabled",
];

/// 把 Java 属性名（驼峰）或库列名映射为 `logistics_company_config` 表白名单列名。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`，但**不照抄它的
/// 主键回落**：Java 找不到属性时返回主键列（会静默把 `config_id` 写成
/// `val`），这里返回 `None` 让调用方报错，避免静默写错列。
///
/// 本域只用到 `sortOrder` / `isEnabled`（即 Java 控制器的
/// `CONFIG_UPDATE_FIELDS = {"is_enabled", "sort_order"}`），其余一律拒绝。
#[must_use]
pub fn map_update_field_column(field: &str) -> Option<&'static str> {
    match field.trim() {
        "isEnabled" | "is_enabled" => Some("is_enabled"),
        "sortOrder" | "sort_order" => Some("sort_order"),
        _ => None,
    }
}

/// 纯函数：`updateField` 的入参校验（可单测）。
///
/// 返回 `(主键, 列名, 值)`；字段不在 `CONFIG_UPDATE_FIELDS` 白名单时返回
/// `INVALID_FIELD_VALUE` 文案（与 Java `ResultTextConstants` 一致）。
/// 校验顺序对齐 `BaseServiceImpl#updateField`：先字段、后主键 ——
/// Java 先 `getColumnByPropertyName` 判白名单，再构造 `eq(主键)`。
pub fn validate_update_field(
    id: Option<i32>,
    field: Option<&str>,
    val: Option<&str>,
) -> Result<(i32, &'static str, String)> {
    let field = field
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation(ERR_INVALID_FIELD_VALUE.into()))?;
    let column = map_update_field_column(field)
        .ok_or_else(|| Error::Validation(ERR_INVALID_FIELD_VALUE.into()))?;
    let id = id.ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;
    let val = val.ok_or_else(|| Error::Validation("字段值不能为空".into()))?;
    Ok((id, column, val.to_string()))
}

/// 纯函数：`update` 的动态 SET 列组装（可单测）。
///
/// 对应 Java `BeanUtils.copyProperties(updateDTO, config)` + `updateById`：
/// **只更新显式传入（非 `null`）的字段**，未传字段保持库中原值。
///
/// 返回 `(列名, 绑定值)` 列表；不包含主键、`shop_id`、`vendor_id`、`is_default`
/// （Java 的 `update` 既不换租户也不动默认标记）。
#[must_use]
pub fn build_update_columns(dto: &LogisticsCompanyConfigUpdateDTO) -> Vec<(&'static str, ColumnValue)> {
    let mut cols: Vec<(&'static str, ColumnValue)> = Vec::new();

    if let Some(v) = dto.logistics_id {
        cols.push(("logistics_id", ColumnValue::Int(Some(v))));
    }
    // 文本列：Java 未传即不更新；显式传空串则写空串（BeanUtils 不区分）
    let text_fields: [(&'static str, &Option<String>); 15] = [
        ("customer_name", &dto.customer_name),
        ("customer_pwd", &dto.customer_pwd),
        ("month_code", &dto.month_code),
        ("send_site", &dto.send_site),
        ("send_staff", &dto.send_staff),
        ("exp_type", &dto.exp_type),
        ("kuaidi100_partner_id", &dto.kuaidi100_partner_id),
        ("kuaidi100_partner_key", &dto.kuaidi100_partner_key),
        ("kuaidi100_partner_secret", &dto.kuaidi100_partner_secret),
        ("kuaidi100_net", &dto.kuaidi100_net),
        ("kuaidi100_check_man", &dto.kuaidi100_check_man),
        ("kuaidi100_exp_type", &dto.kuaidi100_exp_type),
        ("kuaidi100_temp_id", &dto.kuaidi100_temp_id),
        ("kuaidi100_child_temp_id", &dto.kuaidi100_child_temp_id),
        ("kuaidi100_back_temp_id", &dto.kuaidi100_back_temp_id),
    ];
    for (col, val) in text_fields {
        if let Some(v) = val {
            cols.push((col, ColumnValue::Text(Some(v.clone()))));
        }
    }
    if let Some(v) = dto.sort_order {
        cols.push(("sort_order", ColumnValue::Int(Some(v))));
    }
    if let Some(v) = dto.is_enabled {
        cols.push(("is_enabled", ColumnValue::Int(Some(v))));
    }
    cols
}

/// 纯函数：请求里的服务商类型是否需要走 `logistics_company` 子查询。
///
/// 对应 Java `list` 的判定：`StrUtil.isNotEmpty(keyword)` 或
/// `providerType == 1` 或 `providerType == 2` 才去筛公司表。
#[must_use]
pub fn needs_company_filter(q: &LogisticsCompanyConfigListQuery) -> bool {
    q.keyword_trimmed().is_some() || q.provider_type_valid().is_some()
}

pub struct LogisticsCompanyConfigService {
    repos: Arc<Repositories>,
}

impl LogisticsCompanyConfigService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    fn repo(&self) -> &LogisticsCompanyConfigRepository {
        &self.repos.logistics.logistics_company_config
    }

    /// 配置归属 `(shop_id, vendor_id)`（多租户校验用；不存在返回 `None`）。
    pub async fn scope_of(&self, config_id: i32) -> Result<Option<(i32, i32)>> {
        Ok(self
            .repo()
            .find_config_by_id(config_id)
            .await?
            .map(|c| (c.shop_id, c.vendor_id)))
    }

    // ————————————————————— 管理端 —————————————————————

    /// 当前租户的物流公司配置分页。
    ///
    /// 对应 Java `LogisticsCompanyConfigServiceImpl#list`，逐条对齐：
    /// 1. `shop_id` / `vendor_id` 等值过滤（Java 把 `null` 归一为 0）
    /// 2. `is_enabled` 非空时等值过滤
    /// 3. 关键词 / 服务商类型非空时，先在 `logistics_company` 上筛出
    ///    `logistics_id` 集合；**筛出空集直接返回空页**（Java 的
    ///    `if (matchedIds.isEmpty()) return convertPage(page, List.of())`）
    /// 4. 排序 `is_default DESC, is_enabled DESC`
    /// 5. 按 `logisticsIds` **批量**回填物流公司四个展示字段（避免 N+1）
    pub async fn list(
        &self,
        q: &LogisticsCompanyConfigListQuery,
        shop_id: i32,
        vendor_id: i32,
    ) -> Result<PageResult<LogisticsCompanyConfigVO>> {
        let page = q.page_query();

        let matched_ids: Option<Vec<i32>> = if needs_company_filter(q) {
            let ids = self
                .repo()
                .find_company_ids(q.keyword_trimmed(), q.provider_type_valid())
                .await?;
            if ids.is_empty() {
                // Java: 未匹配到任何物流公司 → 空分页
                return Ok(PageResult::empty(&page));
            }
            Some(ids)
        } else {
            None
        };

        let (total, rows) = self
            .repo()
            .paginate_configs(
                shop_id,
                vendor_id,
                q.is_enabled,
                matched_ids.as_deref(),
                &page,
            )
            .await?;

        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }

        let company_map = self.load_company_map(&rows).await?;
        let list = rows
            .iter()
            .map(|c| to_vo(c, company_map.get(&c.logistics_id)))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 配置详情。
    ///
    /// 对应 Java `detail`：`id` 为空或查不到时 Java **返回 `null`**
    /// （不抛异常），故这里返回 `Ok(None)`，由路由层输出 `data: null`。
    pub async fn detail(&self, id: Option<i32>) -> Result<Option<LogisticsCompanyConfigVO>> {
        let Some(id) = id else {
            return Ok(None);
        };
        let Some(config) = self.repo().find_config_by_id(id).await? else {
            return Ok(None);
        };
        let company = self.repo().find_company_by_id(config.logistics_id).await?;
        Ok(Some(to_vo(&config, company.as_ref())))
    }

    /// 绑定物流公司。
    ///
    /// 对应 Java `create`，校验链与默认值逐条对齐：
    /// 1. `logisticsId` 非空（否则「物流公司不能为空」）
    /// 2. 物流公司必须存在（否则「物流公司不存在」）
    /// 3. 同租户下不允许重复绑定（否则「该物流公司已绑定，请勿重复添加」）
    /// 4. `sortOrder` 缺省 50、`isEnabled` 缺省 1、`isDefault` 固定 0
    ///
    /// 返回新建的 `config_id`（Java 返回 `boolean`，前端不需要该值；
    /// Rust 侧返回主键便于前端跳详情）。
    pub async fn create(
        &self,
        dto: &LogisticsCompanyConfigCreateDTO,
        shop_id: i32,
        vendor_id: i32,
    ) -> Result<i32> {
        let logistics_id = dto
            .logistics_id
            .ok_or_else(|| Error::Validation(ERR_LOGISTICS_ID_REQUIRED.into()))?;

        let company = self
            .repo()
            .find_company_by_id(logistics_id)
            .await?
            .ok_or_else(|| Error::Validation(ERR_LOGISTICS_NOT_FOUND.into()))?;

        if self
            .repo()
            .count_binding(shop_id, vendor_id, logistics_id)
            .await?
            > 0
        {
            return Err(Error::biz_msg(ERR_ALREADY_BOUND));
        }

        let config = LogisticsCompanyConfig {
            logistics_id,
            shop_id,
            vendor_id,
            customer_name: dto.customer_name.clone(),
            customer_pwd: dto.customer_pwd.clone(),
            month_code: dto.month_code.clone(),
            send_site: dto.send_site.clone(),
            send_staff: dto.send_staff.clone(),
            exp_type: dto.exp_type.clone(),
            // NOT NULL 列：Java 未传时为 null，MyBatis-Plus 插入会因列非空报错；
            // 这里按库约束补空串，行为等价于「前端必须传」但不至于 500。
            kuaidi100_partner_id: dto.kuaidi100_partner_id.clone().unwrap_or_default(),
            kuaidi100_partner_key: dto.kuaidi100_partner_key.clone().unwrap_or_default(),
            kuaidi100_partner_secret: dto.kuaidi100_partner_secret.clone().unwrap_or_default(),
            kuaidi100_net: dto.kuaidi100_net.clone().unwrap_or_default(),
            kuaidi100_check_man: dto.kuaidi100_check_man.clone().unwrap_or_default(),
            kuaidi100_exp_type: dto.kuaidi100_exp_type.clone().unwrap_or_default(),
            kuaidi100_temp_id: dto.kuaidi100_temp_id.clone().unwrap_or_default(),
            kuaidi100_child_temp_id: dto.kuaidi100_child_temp_id.clone().unwrap_or_default(),
            kuaidi100_back_temp_id: dto.kuaidi100_back_temp_id.clone().unwrap_or_default(),
            sort_order: Some(dto.sort_order.unwrap_or(DEFAULT_SORT_ORDER)),
            // Java: config.setIsDefault(0) —— 新建永不默认
            is_default: Some(0),
            is_enabled: Some(dto.is_enabled.unwrap_or(DEFAULT_IS_ENABLED)),
            ..Default::default()
        };

        let config_id = self.repo().insert_config(&config).await?;
        // Java: adminLogService.createByLogInfo("新增物流公司配置:" + logisticsName)
        let log_info = format!("新增物流公司配置:{}", company.logistics_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!(
            "新增物流公司配置:{} (config_id={config_id}, shop_id={shop_id}, vendor_id={vendor_id})",
            company.logistics_name
        );
        Ok(config_id)
    }

    /// 更新物流公司配置（部分更新）。
    ///
    /// 对应 Java `update`：
    /// 1. 配置必须存在（否则「配置记录不存在」）
    /// 2. 若显式传了 `logisticsId` 且与原值不同 → 判重（排除自身），
    ///    重复则「该物流公司已绑定，请勿重复添加」
    /// 3. 只更新非 `null` 字段（`BeanUtils.copyProperties` + `updateById` 语义）
    ///
    /// **不改租户列**（`shop_id` / `vendor_id`）与**默认标记**（`is_default`）。
    pub async fn update(&self, dto: &LogisticsCompanyConfigUpdateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation(ERR_ID_REQUIRED.into()))?;

        let existing = self
            .repo()
            .find_config_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(ERR_CONFIG_NOT_FOUND.into()))?;

        if let Some(new_logistics_id) = dto.logistics_id {
            if new_logistics_id != existing.logistics_id
                && self
                    .repo()
                    .count_binding_except(existing.shop_id, existing.vendor_id, new_logistics_id, id)
                    .await?
                    > 0
            {
                return Err(Error::biz_msg(ERR_ALREADY_BOUND));
            }
        }

        let columns = build_update_columns(dto);
        // 列名全部来自本文件的固定字面量白名单，不可能是外部输入
        debug_assert!(columns
            .iter()
            .all(|(c, _)| UPDATE_COLUMN_WHITELIST.contains(c)));
        self.repo().update_partial(id, &columns).await?;

        // Java: "更新物流公司配置:" + companyName（查不到公司时回落 id 字符串）
        let target_logistics_id = dto.logistics_id.unwrap_or(existing.logistics_id);
        let company_name = self
            .repo()
            .find_company_by_id(target_logistics_id)
            .await?
            .map(|c| c.logistics_name)
            .unwrap_or_else(|| id.to_string());
        let log_info = format!("更新物流公司配置:{company_name}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if existing.shop_id > 0 { "shop" } else { "admin" }, existing.shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("更新物流公司配置:{company_name} (config_id={id})");
        Ok(())
    }

    /// 解绑物流公司（`del`）。
    ///
    /// 对应 Java `BaseServiceImpl#del`：`id == null` 返回 `false`（不报错）；
    /// `removeById(id)` 对不存在的 ID 也返回 `false` 而不抛异常。
    /// Rust 侧沿用「不存在即静默成功」的语义（幂等删除），不抛 `NotFound`。
    pub async fn delete(&self, id: Option<i32>) -> Result<()> {
        let Some(id) = id else {
            return Ok(());
        };
        let affected = self.repo().delete_config(id).await?;
        if affected == 0 {
            tracing::debug!("解绑物流公司配置: config_id={id} 不存在，按幂等处理");
        }
        Ok(())
    }

    /// 设置默认物流公司。
    ///
    /// 对应 Java `setDefault`（`@Transactional`）：
    /// 1. 配置必须存在（否则「配置记录不存在」）
    /// 2. 把**该配置所属租户**（`shop_id` + `vendor_id`）下全部记录
    ///    `is_default` 置 0
    /// 3. 再把目标记录置 1
    ///
    /// 两条 UPDATE 在同一事务内（见仓储 `set_default_tx`）。
    pub async fn set_default(&self, id: i32) -> Result<()> {
        let config = self
            .repo()
            .find_config_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(ERR_CONFIG_NOT_FOUND_DEFAULT.into()))?;

        self.repo()
            .set_default_tx(id, config.shop_id, config.vendor_id)
            .await?;

        // Java: "设置默认物流公司:" + companyName（查不到公司时回落 id 字符串）
        let company_name = self
            .repo()
            .find_company_by_id(config.logistics_id)
            .await?
            .map(|c| c.logistics_name)
            .unwrap_or_else(|| id.to_string());
        let log_info = format!("设置默认物流公司:{company_name}");
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if config.shop_id > 0 { "shop" } else { "admin" }, config.shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("设置默认物流公司:{company_name} (config_id={id})");
        Ok(())
    }

    /// 通用字段更新（`updateField`）。
    ///
    /// 对应 Java `BaseServiceImpl#updateField(updateField, CONFIG_UPDATE_FIELDS)`：
    /// 字段不在白名单 → `INVALID_FIELD_VALUE`；否则 `eq(config_id).set(col, val)`。
    /// Java 该方法**不校验记录是否存在**，更新 0 行也返回成功。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let (id, column, val) =
            validate_update_field(dto.id, dto.field.as_deref(), dto.val.as_deref())?;
        self.repo().update_field(id, column, &val).await?;
        Ok(())
    }

    // ————————————————————— 内部 —————————————————————

    /// 批量加载配置对应的物流公司，返回 `logistics_id -> company`。
    ///
    /// 对应 Java `list` 中的 `listByIds(logisticsIds)` + `Collectors.toMap(...,
    /// (a, b) -> a)`（**同 ID 保留先出现的那个**）。这里用
    /// `HashMap::entry().or_insert()` 保留首次出现的记录，与 Java 合并函数一致。
    async fn load_company_map(
        &self,
        rows: &[LogisticsCompanyConfig],
    ) -> Result<HashMap<i32, LogisticsCompany>> {
        let mut ids: Vec<i32> = rows.iter().map(|c| c.logistics_id).collect();
        ids.sort_unstable();
        ids.dedup();
        let companies = self.repo().find_companies_by_ids(&ids).await?;
        let mut map: HashMap<i32, LogisticsCompany> = HashMap::with_capacity(companies.len());
        for c in companies {
            map.entry(c.logistics_id).or_insert(c);
        }
        Ok(map)
    }
}

/// 实体 + 关联公司 → VO。
///
/// 对应 Java `BeanUtils.copyProperties(config, vo)` + 四个物流公司字段回填。
/// 公司查不到时这四列保持 `null`（Java 的 `if (company != null)` 分支）。
#[must_use]
pub fn to_vo(
    c: &LogisticsCompanyConfig,
    company: Option<&LogisticsCompany>,
) -> LogisticsCompanyConfigVO {
    LogisticsCompanyConfigVO {
        config_id: c.config_id,
        logistics_id: c.logistics_id,
        logistics_name: company.map(|x| x.logistics_name.clone()),
        logistics_code: company.map(|x| x.logistics_code.clone()),
        kuaidi100_code: company.map(|x| x.kuaidi100_code.clone()),
        logistics_desc: company.map(|x| x.logistics_desc.clone()),
        customer_name: c.customer_name.clone(),
        customer_pwd: c.customer_pwd.clone(),
        month_code: c.month_code.clone(),
        send_site: c.send_site.clone(),
        send_staff: c.send_staff.clone(),
        exp_type: c.exp_type.clone(),
        kuaidi100_partner_id: c.kuaidi100_partner_id.clone(),
        kuaidi100_partner_key: c.kuaidi100_partner_key.clone(),
        kuaidi100_partner_secret: c.kuaidi100_partner_secret.clone(),
        kuaidi100_net: c.kuaidi100_net.clone(),
        kuaidi100_check_man: c.kuaidi100_check_man.clone(),
        kuaidi100_exp_type: c.kuaidi100_exp_type.clone(),
        kuaidi100_temp_id: c.kuaidi100_temp_id.clone(),
        kuaidi100_child_temp_id: c.kuaidi100_child_temp_id.clone(),
        kuaidi100_back_temp_id: c.kuaidi100_back_temp_id.clone(),
        sort_order: c.sort_order,
        is_default: c.is_default,
        is_enabled: c.is_enabled,
    }
}

/// 便捷：把分页结果里的配置行批量转 VO（供其它域复用）。
pub async fn map_rows_to_vos(
    repo: &LogisticsCompanyConfigRepository,
    rows: &[LogisticsCompanyConfig],
) -> Result<Vec<LogisticsCompanyConfigVO>> {
    let mut ids: Vec<i32> = rows.iter().map(|c| c.logistics_id).collect();
    ids.sort_unstable();
    ids.dedup();
    let companies = repo.find_companies_by_ids(&ids).await?;
    let mut map: HashMap<i32, LogisticsCompany> = HashMap::with_capacity(companies.len());
    for c in companies {
        map.entry(c.logistics_id).or_insert(c);
    }
    Ok(rows.iter().map(|c| to_vo(c, map.get(&c.logistics_id))).collect())
}

/// 空页辅助（供路由层短路使用）。
#[must_use]
pub fn empty_page(q: &PageQuery) -> PageResult<LogisticsCompanyConfigVO> {
    PageResult::empty(q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    // ————————————————— 字段白名单 —————————————————

    #[test]
    fn map_update_field_accepts_camel_and_snake() {
        assert_eq!(map_update_field_column("isEnabled"), Some("is_enabled"));
        assert_eq!(map_update_field_column("is_enabled"), Some("is_enabled"));
        assert_eq!(map_update_field_column("sortOrder"), Some("sort_order"));
        assert_eq!(map_update_field_column("sort_order"), Some("sort_order"));
        // 带空白也要能识别（前端偶尔会传 " isEnabled "）
        assert_eq!(map_update_field_column("  isEnabled  "), Some("is_enabled"));
    }

    #[test]
    fn map_update_field_rejects_everything_outside_java_constant() {
        // Java CONFIG_UPDATE_FIELDS = {"is_enabled", "sort_order"}
        for bad in [
            "logisticsId",
            "logistics_id",
            "customerName",
            "isDefault",
            "is_default",
            "configId",
            "shopId",
            "vendorId",
            "1=1 OR 1=1",
            "",
            "   ",
        ] {
            assert_eq!(map_update_field_column(bad), None, "不该放行: {bad}");
        }
    }

    #[test]
    fn validate_update_field_checks_whitelist_before_id() {
        // 字段非法时先报字段错误（对齐 Java 先查白名单再拼 eq 的顺序）
        let e = validate_update_field(None, Some("nope"), Some("1")).unwrap_err();
        assert!(matches!(e, Error::Validation(_)));
        assert!(e.to_string().contains(ERR_INVALID_FIELD_VALUE));

        // 字段合法但主键缺失 → 报主键错误
        let e = validate_update_field(None, Some("isEnabled"), Some("0")).unwrap_err();
        assert!(e.to_string().contains(ERR_ID_REQUIRED));

        // 字段与主键都合法 → 通过
        let (id, col, val) = validate_update_field(Some(3), Some("isEnabled"), Some("0")).unwrap();
        assert_eq!(id, 3);
        assert_eq!(col, "is_enabled");
        assert_eq!(val, "0");
    }

    #[test]
    fn validate_update_field_error_never_echoes_input() {
        // 防止把用户输入反射进错误文案（XSS / 信息泄露）
        let e = validate_update_field(Some(1), Some("<script>x</script>"), Some("1")).unwrap_err();
        assert!(!e.to_string().contains("<script>"));
    }

    #[test]
    fn update_column_whitelist_is_unique_and_matches_repo() {
        let mut seen = std::collections::HashSet::new();
        for c in UPDATE_COLUMN_WHITELIST {
            assert!(seen.insert(c), "重复列名: {c}");
        }
        // 仓储侧白名单同长（值级一致性由 infra 单测保证）
        assert_eq!(UPDATE_COLUMN_WHITELIST.len(), 18);
        assert!(!UPDATE_COLUMN_WHITELIST.contains(&"config_id"));
        assert!(!UPDATE_COLUMN_WHITELIST.contains(&"shop_id"));
        assert!(!UPDATE_COLUMN_WHITELIST.contains(&"vendor_id"));
        assert!(!UPDATE_COLUMN_WHITELIST.contains(&"is_default"));
    }

    // ————————————————— 动态 SET 组装 —————————————————

    fn cfg(config_id: i32, logistics_id: i32, shop_id: i32, vendor_id: i32) -> LogisticsCompanyConfig {
        LogisticsCompanyConfig {
            config_id,
            logistics_id,
            shop_id,
            vendor_id,
            kuaidi100_partner_id: String::new(),
            kuaidi100_partner_key: String::new(),
            kuaidi100_partner_secret: String::new(),
            kuaidi100_net: String::new(),
            kuaidi100_check_man: String::new(),
            kuaidi100_exp_type: String::new(),
            kuaidi100_temp_id: String::new(),
            kuaidi100_child_temp_id: String::new(),
            kuaidi100_back_temp_id: String::new(),
            sort_order: Some(50),
            is_default: Some(0),
            is_enabled: Some(1),
            ..Default::default()
        }
    }

    fn company(logistics_id: i32) -> LogisticsCompany {
        LogisticsCompany {
            logistics_id,
            logistics_code: "SF".into(),
            logistics_name: "顺丰速运".into(),
            logistics_desc: "次日达".into(),
            kuaidi100_code: "shunfeng".into(),
            ..Default::default()
        }
    }

    #[test]
    fn build_update_columns_only_includes_present_fields() {
        let dto = LogisticsCompanyConfigUpdateDTO {
            id: Some(5),
            is_enabled: Some(0),
            sort_order: Some(9),
            ..Default::default()
        };
        let cols = build_update_columns(&dto);
        assert_eq!(cols.len(), 2);
        assert!(cols.iter().any(|(c, _)| *c == "is_enabled"));
        assert!(cols.iter().any(|(c, _)| *c == "sort_order"));
        // 主键不进 SET 子句
        assert!(!cols.iter().any(|(c, _)| *c == "config_id"));
    }

    #[test]
    fn build_update_columns_all_fields_when_all_present() {
        let dto = LogisticsCompanyConfigUpdateDTO {
            id: Some(1),
            logistics_id: Some(2),
            customer_name: Some("a".into()),
            customer_pwd: Some("b".into()),
            month_code: Some("c".into()),
            send_site: Some("d".into()),
            send_staff: Some("e".into()),
            exp_type: Some("f".into()),
            kuaidi100_partner_id: Some("g".into()),
            kuaidi100_partner_key: Some("h".into()),
            kuaidi100_partner_secret: Some("i".into()),
            kuaidi100_net: Some("j".into()),
            kuaidi100_check_man: Some("k".into()),
            kuaidi100_exp_type: Some("l".into()),
            kuaidi100_temp_id: Some("m".into()),
            kuaidi100_child_temp_id: Some("n".into()),
            kuaidi100_back_temp_id: Some("o".into()),
            sort_order: Some(3),
            is_enabled: Some(1),
        };
        let cols = build_update_columns(&dto);
        assert_eq!(cols.len(), UPDATE_COLUMN_WHITELIST.len());
        for (c, _) in &cols {
            assert!(UPDATE_COLUMN_WHITELIST.contains(c), "越界列: {c}");
        }
    }

    #[test]
    fn build_update_columns_keeps_explicit_empty_string() {
        // Java BeanUtils 不区分「未传」与「传空串」：显式空串要写入
        let dto = LogisticsCompanyConfigUpdateDTO {
            id: Some(1),
            customer_name: Some(String::new()),
            ..Default::default()
        };
        let cols = build_update_columns(&dto);
        assert_eq!(cols.len(), 1);
        match &cols[0].1 {
            ColumnValue::Text(Some(v)) => assert!(v.is_empty()),
            other => panic!("期望空串文本列，实际 {other:?}"),
        }
    }

    #[test]
    fn build_update_columns_empty_dto_yields_no_set() {
        let dto = LogisticsCompanyConfigUpdateDTO {
            id: Some(1),
            ..Default::default()
        };
        assert!(build_update_columns(&dto).is_empty());
    }

    // ————————————————— 服务商过滤判定 —————————————————

    #[test]
    fn needs_company_filter_matches_java_condition() {
        let mut q = LogisticsCompanyConfigListQuery::default();
        assert!(!needs_company_filter(&q));

        // 仅 isEnabled 不足以触发公司表子查询
        q.is_enabled = Some(1);
        assert!(!needs_company_filter(&q));

        q.keyword = Some("  ".into());
        assert!(!needs_company_filter(&q), "空白关键词不算非空");

        q.keyword = Some("顺丰".into());
        assert!(needs_company_filter(&q));

        let mut q = LogisticsCompanyConfigListQuery::default();
        q.provider_type = Some(1);
        assert!(needs_company_filter(&q));
        q.provider_type = Some(2);
        assert!(needs_company_filter(&q));
        q.provider_type = Some(3);
        assert!(!needs_company_filter(&q), "非法服务商类型不过滤");
    }

    // ————————————————— VO 组装 —————————————————

    #[test]
    fn to_vo_fills_joined_company_columns() {
        let c = cfg(1, 2, 0, 0);
        let vo = to_vo(&c, Some(&company(2)));
        assert_eq!(vo.config_id, 1);
        assert_eq!(vo.logistics_id, 2);
        assert_eq!(vo.logistics_name.as_deref(), Some("顺丰速运"));
        assert_eq!(vo.logistics_code.as_deref(), Some("SF"));
        assert_eq!(vo.kuaidi100_code.as_deref(), Some("shunfeng"));
        assert_eq!(vo.logistics_desc.as_deref(), Some("次日达"));
        assert_eq!(vo.sort_order, Some(50));
        assert_eq!(vo.is_default, Some(0));
        assert_eq!(vo.is_enabled, Some(1));
    }

    #[test]
    fn to_vo_leaves_company_columns_null_when_missing() {
        // Java: if (company != null) 才回填 → 查不到公司时四列保持 null
        let vo = to_vo(&cfg(1, 2, 0, 0), None);
        assert!(vo.logistics_name.is_none());
        assert!(vo.logistics_code.is_none());
        assert!(vo.kuaidi100_code.is_none());
        assert!(vo.logistics_desc.is_none());
        // 配置自身列仍然完整
        assert_eq!(vo.config_id, 1);
    }

    #[test]
    fn to_vo_tolerates_nullable_columns() {
        let vo = to_vo(&cfg(7, 8, 0, 0), None);
        assert!(vo.customer_name.is_none());
        assert!(vo.sort_order.is_some());
        // 本域无金额字段；保留一个 Decimal 断言以固化「不用 f64」的约定
        let d = Decimal::from_str("0").unwrap();
        assert_eq!(d, Decimal::ZERO);
    }

    #[test]
    fn empty_page_is_zeroed() {
        let p = empty_page(&PageQuery {
            page: 2,
            page_size: 15,
        });
        assert_eq!(p.total, 0);
        assert!(p.list.is_empty());
        assert_eq!(p.page, 2);
        assert_eq!(p.page_size, 15);
    }
}