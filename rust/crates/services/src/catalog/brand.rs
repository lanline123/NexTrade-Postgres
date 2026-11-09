//! 品牌域服务：品牌 CRUD、字段更新、审核流、首字母维护。
//! 对应原 Java `BrandServiceImpl`（`com.nextrade.service.product.impl`）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `checkBrand`：同店铺内品牌名唯一，更新时排除自身
//! - `create` / `update`：店铺账号（`shop_id > 0`）创建后进入「待审核」，
//!   平台账号（`shop_id = 0`）创建直接「审核通过」
//! - `getFirstWord`：品牌名缺省时自动推导首字母
//!   （ASCII 字母/数字直取；中文取拼音首字母大写，无拼音库时回退 `#`）
//! - `audit`：仅允许 1 通过 / 2 拒绝；拒绝必须填写理由
//! - `auditList`：与 `list` 共用查询，`isAudit=1` 时只看店铺品牌
//!   （`shop_id <> 0`）
//! - `updateField`：白名单校验（`brand_name`/`first_word`/`brand_is_hot`/
//!   `is_show`/`sort_order`），店铺品牌改动后重新进入待审核
//!
//! **MyBatis-Plus 字段策略对齐（重要）**：Java 用 `save` / `updateById`，
//! 默认 `FieldStrategy.NOT_NULL` 会把 DTO 中为 `null` 的字段从 SQL 中剔除，
//! 因此「未传的列」在 `create` 时套用 `brand` 表 DDL 默认值
//! （`sort_order=50`、`is_show=1`、…），在 `update` 时保留原值。
//! Rust 侧 INSERT 显式列出全部列，故在此显式补齐同样的默认值，
//! 见 [`create_defaults`] 与 [`dto_or_keep`]。
//!
//! **不确定点（已在 `uncertain_points` 中列明）**：
//! - 品牌列表/创建/更新的 `shop_id` 来源：Java 取自登录态请求头
//!   （`HeaderUtils.getShopId()`），Rust JWT 尚未签发 `shop_id`，
//!   当前改由调用方显式传入（路由层取请求体 / 查询参数的 `shopId`）。
//!   这**不是等价实现**：平台账号不传 `shop_id` 时按 `0`（平台）处理，
//!   与 Java「`shopId == null` 时唯一性校验条件退化为 `shop_id IS NULL`
//!   因而永不冲突」一致，但店铺账号不传时会被当成平台账号。
//! - `updateField` 的店铺品牌「先置待审核再更新字段」与 Java 的第 390/396 行
//!   顺序一致；但 Java 未命中白名单时会回退主键静默更新，Rust 直接报错。
//! - `updateFirstWord` 的管理操作日志（`adminLogService`，日志域未迁移）
//! - `getBrandId` / `getBrandNameByIds`（`BrandService` 内部方法，
//!   不由 `BrandController` 暴露，本切片不实现）

use std::sync::Arc;

use nextrade_core::entities::Brand;
use nextrade_core::models::catalog::brand::{
    BrandAuditDTO, BrandDTO, BrandListQuery, BrandSearchItem, BrandSearchVO, BrandShopVO,
    BrandStatus, BrandVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 品牌名长度上限，对应 Java `@Size(min = 1, max = 30)` 与
/// `BrandConstants.BRAND_LENGTH_ERROR`（“品牌名称长度为1-30位”）。
pub const BRAND_NAME_MAX_LEN: usize = 30;

/// `updateField` 允许更新的列白名单，对应 Java
/// `CheckFieldConstants.BRAND_FIELDS`。
pub const BRAND_FIELDS: [&str; 5] = [
    "brand_name",
    "first_word",
    "brand_is_hot",
    "is_show",
    "sort_order",
];

/// 审核状态码，对应 Java `CheckStatus`（与 `BrandStatusEnum` 数值一致）。
pub const CHECK_STATUS_PENDING: i32 = BrandStatus::Pending as i32;
pub const CHECK_STATUS_APPROVED: i32 = BrandStatus::Approved as i32;
pub const CHECK_STATUS_REJECTED: i32 = BrandStatus::Rejected as i32;

/// `brand` 表 DDL 默认值（`java/database/pure.sql`）。
///
/// **为什么需要这些常量**：Java 走 MyBatis-Plus 的 `save`，默认字段策略
/// `FieldStrategy.NOT_NULL` 会把 DTO 里为 `null` 的字段**排除出 INSERT 语句**，
/// 于是由 MySQL 套用列默认值：
/// `sort_order DEFAULT 50`、`is_show DEFAULT 1`、`brand_is_hot DEFAULT 0`、
/// `is_store_brand DEFAULT 0`、`check_status DEFAULT 0`、`site_url DEFAULT ''`、
/// `shop_id DEFAULT 0`。
///
/// Rust 侧 `INSERT` 显式列出全部列，`None` 会写成真正的 `NULL`（而不是列默认值），
/// 因此必须在这里把同样的默认值显式补齐，否则「不传 sortOrder/isShow 建品牌」
/// 会得到 `0/0`（前台不显示、排序垫底），与原系统行为不一致。
pub const DEFAULT_SORT_ORDER: i32 = 50;
pub const DEFAULT_IS_SHOW: i32 = 1;

pub struct BrandService {
    repos: Arc<Repositories>,
}

impl BrandService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 品牌分页列表。
    ///
    /// 对应 Java `BrandServiceImpl#list`：
    /// - `shop_id = Some(0)` 且请求带 `shopId` → 按请求店铺过滤（平台管理员）
    /// - `shop_id = Some(n>0)` → 强制只看本店（店铺/门店/收银账号）
    /// - `shop_id = None` → 不做店铺过滤（语义与 Java `getShopId()==null` 一致）
    /// - `status`：`-1` / `None` 表示不过滤
    /// - `is_audit = 1`（审核列表）：强制 `shop_id <> 0`
    pub async fn list(&self, q: &BrandListQuery) -> Result<PageResult<BrandVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            // Java `BasePage.getSize()` 默认 15，上限 100
            page_size: q.page_size.unwrap_or(15).min(100),
        };

        let (total, rows) = self
            .repos.catalog.brand
            .paginate(
                q.keyword.as_deref(),
                q.is_show,
                q.brand_is_hot,
                q.first_word.as_deref(),
                q.status.filter(|s| *s > -1),
                q.shop_id,
                q.is_audit == Some(1),
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;

        // 批量取店铺名（Java `shopMapper.selectBatchIds`），缺失时回退空对象
        let shop_ids: Vec<i32> = rows
            .iter()
            .map(|b| b.shop_id)
            .filter(|id| *id > 0)
            .collect();
        let mut shops = std::collections::HashMap::new();
        if !shop_ids.is_empty() {
            for s in self.repos.catalog.brand.find_shops(&shop_ids).await? {
                shops.insert(s.shop_id, s.shop_title.clone().unwrap_or_default());
            }
        }

        let list: Vec<BrandVO> = rows
            .iter()
            .map(|b| {
                let title = shops.get(&b.shop_id).cloned().unwrap_or_default();
                to_vo(b, title)
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 品牌详情（返回与 Java `BrandDTO` 同构的对象）。
    ///
    /// 对应 Java `BrandServiceImpl#detail`：查不到抛 `BRAND_NOT_FOUND`
    /// （“品牌不存在”）。
    pub async fn detail(&self, brand_id: i32) -> Result<BrandDTO> {
        let b = self
            .repos.catalog.brand
            .find_by_id(brand_id)
            .await?
            .ok_or_else(|| Error::biz(500, "品牌不存在"))?;
        Ok(to_dto(&b))
    }

    /// 品牌搜索（商品选择器的下拉数据）。
    ///
    /// 对应 Java `BrandServiceImpl#search`：
    /// - 品牌项取 `status = 1（审核通过）AND is_show = 1`
    /// - 另返回去重的 `first_word` 列表
    ///
    /// 说明：Java 的 `BrandSearchDTO` 由 `BeanUtils.copyProperties` 从实体拷贝
    /// 全部同名字段，实际存在额外字段；这里只保留 `brandId`/`brandName`/
    /// `firstWord`（前端选择器实际使用的三项），见 `uncertain_points`。
    pub async fn search(&self) -> Result<BrandSearchVO> {
        let brands = self.repos.catalog.brand.find_searchable().await?;
        let first_words = self.repos.catalog.brand.distinct_first_words().await?;
        Ok(BrandSearchVO {
            brand_list: brands
                .iter()
                .map(|b| BrandSearchItem {
                    brand_id: b.brand_id,
                    brand_name: b.brand_name.clone(),
                    first_word: b.first_word.clone(),
                })
                .collect(),
            first_word_list: first_words,
        })
    }

    /// 待审核数量（`status = 0`）。
    ///
    /// 对应 Java `BrandServiceImpl#auditWaitNum`：`count(status = PENDING)`，
    /// 不叠加店铺过滤（Java 该查询也未用 `HeaderUtils.getShopId()`）。
    pub async fn audit_wait_num(&self) -> Result<i64> {
        self.repos.catalog.brand.count_by_status(CHECK_STATUS_PENDING).await
    }

    // ————————————————————— 写操作 —————————————————————

    /// 创建品牌，返回 `brand_id`。
    ///
    /// 对应 Java `BrandServiceImpl#create`：
    /// - 名称必填且 1-30 字
    /// - 同店铺内名称唯一（`checkBrand`）
    /// - 平台账号（`shop_id = 0`）→ 直接审核通过；店铺账号 → 待审核
    ///
    /// DTO 未提供的字段按 Java `BeanUtils.copyProperties` + MyBatis-Plus
    /// NOT_NULL 策略处理：**不写入列**，由 `brand` 表 DDL 默认值兜底
    /// （见 [`DEFAULT_SORT_ORDER`] / [`DEFAULT_IS_SHOW`]）。
    /// 注意 Java 侧 `BrandDTO.status` **不会**被写入（实体 `Brand` 无该字段），
    /// 状态只由 `shop_id` 决定。
    pub async fn create(&self, dto: &BrandDTO, shop_id: i32) -> Result<i32> {
        let name = validate_name(dto.brand_name.as_deref())?;
        self.check_name_unique(&name, shop_id, None).await?;

        let d = create_defaults(dto, shop_id);
        let brand = Brand {
            brand_id: 0,
            brand_name: name.clone(),
            brand_en_name: dto.brand_en_name.clone(),
            brand_logo: dto.brand_logo.clone(),
            brand_desc: dto.brand_desc.clone(),
            site_url: d.site_url,
            sort_order: d.sort_order,
            is_show: d.is_show,
            brand_type: dto.brand_type.clone(),
            brand_is_hot: d.brand_is_hot,
            first_word: Some(first_word_or_auto(dto.first_word.as_deref(), &name)),
            is_store_brand: d.is_store_brand,
            check_status: d.check_status,
            shop_id,
            status: d.status,
            reject_remark: dto.reject_remark.clone(),
        };
        let id = self.repos.catalog.brand.insert(&brand).await?;
        tracing::info!("创建品牌成功: id={id}, name={name}, shop_id={shop_id}");
        Ok(id)
    }

    /// 更新品牌。
    ///
    /// 对应 Java `BrandServiceImpl#update`：同名校验后 `updateById(brand)`。
    ///
    /// MyBatis-Plus 的 NOT_NULL 策略使 `null` 字段**不进入 SET 子句**，
    /// 因此这里对每个可空字段都做「DTO 有值则改、无值则保留原值」的等价处理
    /// （`reject_remark` 尤其重要：平台账号改名不应把历史驳回理由清空）。
    /// 店铺账号修改后重新进入「待审核」；平台账号保留原审核状态。
    pub async fn update(&self, dto: &BrandDTO, shop_id: i32) -> Result<()> {
        let brand_id = dto
            .brand_id
            .ok_or_else(|| Error::Validation("品牌ID不能为空".into()))?;
        let name = validate_name(dto.brand_name.as_deref())?;
        self.check_name_unique(&name, shop_id, Some(brand_id)).await?;

        // 必须已存在（Java `updateById` 影响 0 行会静默失败，这里显式报错）
        let existing = self
            .repos.catalog.brand
            .find_by_id(brand_id)
            .await?
            .ok_or_else(|| Error::biz(500, "品牌不存在"))?;

        let status = update_target_status(shop_id, existing.status);

        let brand = Brand {
            brand_id,
            // Java `getFirstWord`：仅当 DTO 的 firstWord 为空白才按名称重新推导
            first_word: Some(resolve_first_word(
                dto.first_word.as_deref(),
                existing.first_word.as_deref(),
                &name,
            )),
            brand_name: name,
            brand_en_name: dto_or_keep_nullable(dto.brand_en_name.clone(), &existing.brand_en_name),
            brand_logo: dto_or_keep_nullable(dto.brand_logo.clone(), &existing.brand_logo),
            brand_desc: dto_or_keep_nullable(dto.brand_desc.clone(), &existing.brand_desc),
            site_url: dto_or_keep(dto.site_url.clone(), &existing.site_url),
            sort_order: dto_or_keep(dto.sort_order, &existing.sort_order),
            is_show: dto_or_keep(dto.is_show, &existing.is_show),
            brand_type: dto_or_keep_nullable(dto.brand_type.clone(), &existing.brand_type),
            brand_is_hot: dto_or_keep(dto.brand_is_hot, &existing.brand_is_hot),
            // Java `updateById` 不含 `check_status`（废弃列）→ 保留原值
            is_store_brand: existing.is_store_brand,
            check_status: existing.check_status,
            shop_id: existing.shop_id,
            status,
            // Java：DTO 未传 rejectRemark 时不更新该列 → 保留原值
            reject_remark: dto_or_keep_nullable(dto.reject_remark.clone(), &existing.reject_remark),
        };
        self.repos.catalog.brand.update(&brand).await?;
        tracing::info!("更新品牌成功: id={brand_id}, shop_id={shop_id}");
        Ok(())
    }

    /// 删除品牌。
    ///
    /// 对应 Java `BrandServiceImpl#del`（`BaseServiceImpl#del` →
    /// `removeById`）：物理 DELETE（`brand` 表无删除标记列）。
    pub async fn delete(&self, brand_id: i32) -> Result<()> {
        let deleted = self.repos.catalog.brand.delete(brand_id).await?;
        if !deleted {
            return Err(Error::biz(500, "品牌不存在"));
        }
        tracing::info!("删除品牌: id={brand_id}");
        Ok(())
    }

    /// 批量操作。
    ///
    /// 对应 Java `BaseServiceImpl#batch`：仅支持 `type = "del"`，
    /// 其余类型 Java 直接返回 `false`（这里报参数错误）。
    ///
    /// 注意 Java 的 `batch` 是**物理删除**且不校验品牌是否被商品引用，
    /// Rust 保持一致（未额外加引用检查）。
    pub async fn batch(&self, ids: &[i32], op_type: Option<&str>) -> Result<u64> {
        if !op_type.is_some_and(|t| t == "del") {
            return Err(Error::Validation("不支持的操作类型".into()));
        }
        if ids.is_empty() {
            return Err(Error::Validation("未选中任何条目".into()));
        }
        let n = self.repos.catalog.brand.delete_batch(ids).await?;
        tracing::info!("批量删除品牌: {n} 条");
        Ok(n)
    }

    /// 更新单个字段。
    ///
    /// 对应 Java `BrandServiceImpl#updateField`：
    /// - 品牌必须存在（Java `Assert.notNull(brand, "品牌不存在")`）
    /// - 字段必须在 `BRAND_FIELDS` 白名单内
    /// - `brandName` 需在同店铺内唯一（Java 先比较 `"brandName"` 字面量）
    /// - 店铺品牌（`shop_id > 0`）改动后进入待审核
    ///
    /// 返回字段更新是否影响行（Java `super.updateField` 的 boolean）。
    pub async fn update_field(&self, brand_id: i32, field: &str, val: &str) -> Result<bool> {
        let field = normalize_field(field)?;
        let brand = self
            .repos.catalog.brand
            .find_by_id(brand_id)
            .await?
            .ok_or_else(|| Error::biz(500, "品牌不存在"))?;

        if field == "brand_name" {
            let name = validate_name(Some(val))?;
            self.check_name_unique(&name, brand.shop_id, Some(brand_id))
                .await?;
        }

        let updated = self
            .repos.catalog.brand
            .update_field(brand_id, field, val)
            .await?;
        if updated && brand.shop_id > 0 {
            self.repos.catalog.brand
                .set_status(brand_id, CHECK_STATUS_PENDING)
                .await?;
        }
        Ok(updated)
    }

    /// 批量重算首字母。
    ///
    /// 对应 Java `BrandServiceImpl#updateFirstWord`：遍历全部品牌，
    /// 首字母取品牌名推导值（推导不出则为 `#`）。返回更新条数（Java 不返回值）。
    ///
    /// **未迁移**：Java 末尾还会写一条管理操作日志
    /// （`adminLogService.createByLogInfo("品牌名称批量更新")`），日志域未迁移。
    pub async fn update_first_word(&self) -> Result<u64> {
        let brands = self.repos.catalog.brand.find_all().await?;
        let mut n = 0u64;
        for b in &brands {
            let fw = first_pinyin_or_hash(&b.brand_name);
            n += self
                .repos.catalog.brand
                .update_first_word(b.brand_id, &fw)
                .await?;
        }
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, "品牌名称批量更新").await {
            tracing::warn!("写管理操作日志失败: {e}");
        }
        tracing::info!("批量更新品牌首字母: {n} 条");
        Ok(n)
    }

    /// 品牌审核。
    ///
    /// 对应 Java `BrandServiceImpl#audit`：
    /// - 状态只允许 1（通过）/ 2（拒绝），否则「审核状态错误」
    /// - 拒绝必须填写理由（`StringUtils.isBlank`），否则「请填写拒绝理由」
    /// - 品牌必须存在（Java 先 `getById`，不存在则在 `brand.getBrandName()` 处 NPE）
    /// - 仅当理由非空白时才写入 `reject_remark` 列
    pub async fn audit(&self, dto: &BrandAuditDTO) -> Result<()> {
        let status = dto.status;
        if status != CHECK_STATUS_APPROVED && status != CHECK_STATUS_REJECTED {
            return Err(Error::biz(500, "审核状态错误"));
        }
        if status == CHECK_STATUS_REJECTED && is_blank(dto.reject_remark.as_deref()) {
            return Err(Error::biz(500, "请填写拒绝理由"));
        }

        let brand = self
            .repos.catalog.brand
            .find_by_id(dto.brand_id)
            .await?
            .ok_or_else(|| Error::biz(500, "品牌不存在"))?;

        self.repos.catalog.brand
            .audit(
                dto.brand_id,
                status,
                dto.reject_remark.as_deref().filter(|s| !s.trim().is_empty()),
            )
            .await?;

        let log_info = format!("审核品牌{}", brand.brand_name);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log(if brand.shop_id > 0 { "shop" } else { "admin" }, brand.shop_id, &log_info).await {
            tracing::warn!("写管理操作日志失败: {e}");
        }

        tracing::info!(
            "审核品牌: id={} name={} status={status}",
            dto.brand_id,
            brand.brand_name
        );
        Ok(())
    }

    // ————————————————————— 内部 —————————————————————

    /// 品牌名同店铺唯一性校验。
    ///
    /// 对应 Java `checkBrand`：`brand_name` + `shop_id` 组合唯一，
    /// 更新时排除自身。Java 用 `assert` 返回 200 业务码 + `SERVICE_DATA_ERROR`
    /// 文案「品牌名称已存在」。
    async fn check_name_unique(
        &self,
        name: &str,
        shop_id: i32,
        exclude_id: Option<i32>,
    ) -> Result<()> {
        if self
            .repos.catalog.brand
            .exists_same_name(name, shop_id, exclude_id)
            .await?
        {
            return Err(Error::biz(500, "品牌名称已存在"));
        }
        Ok(())
    }
}

/// 空白判定（对齐 Java `StrUtil.isBlank` / `StringUtils.isBlank`）。
fn is_blank(s: Option<&str>) -> bool {
    s.map(str::trim).is_none_or(str::is_empty)
}

/// 品牌名基础校验：非空且 1-30 字（按字符数，对齐 Java `@Size` 语义）。
fn validate_name(raw: Option<&str>) -> Result<String> {
    let name = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Validation("品牌名称不能为空".into()))?;
    let len = name.chars().count();
    if len == 0 || len > BRAND_NAME_MAX_LEN {
        return Err(Error::Validation("品牌名称长度为1-30位".into()));
    }
    Ok(name.to_string())
}

/// `updateField` 字段白名单校验（对应 Java `getColumnByPropertyName` +
/// `BRAND_FIELDS`；Java 未命中时回退主键静默更新，这里更严格直接报错）。
fn normalize_field(field: &str) -> Result<&'static str> {
    let snake = to_snake(field);
    BRAND_FIELDS
        .iter()
        .copied()
        .find(|f| *f == snake)
        .ok_or_else(|| Error::Validation(format!("不允许更新的字段: {field}")))
}

/// 驼峰 → 下划线（允许前端传驼峰或数据库列名）。
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// 首字母推导（对应 Java `getFirstPinyin`）：
/// - ASCII 字母 → 大写
/// - ASCII 数字 → 原字符
/// - 其他（中文等）→ 拼音首字母大写；无拼音库时回退 `#`
///
/// Java 用 Hutool `PinyinUtil`；Rust 工作区无拼音依赖（禁止改 Cargo.toml），
/// 中文按「无拼音」处理，与 Java 返回 `null` → `#` 的兜底一致。
fn first_pinyin_or_hash(name: &str) -> String {
    match first_pinyin(name) {
        Some(c) => c,
        None => "#".to_string(),
    }
}

/// 单字符首字母推导；ASCII 字母/数字直接命中，其余返回 `None`。
pub fn first_pinyin(name: &str) -> Option<String> {
    let c = name.chars().next()?;
    if c.is_ascii_alphabetic() {
        return Some(c.to_ascii_uppercase().to_string());
    }
    if c.is_ascii_digit() {
        return Some(c.to_string());
    }
    None
}

/// 品牌首字母兜底逻辑（对应 Java `getFirstWord`）：
/// 未填写时按名称推导，推导不出为 `#`。
fn first_word_or_auto(provided: Option<&str>, brand_name: &str) -> String {
    match provided.map(str::trim).filter(|s| !s.is_empty()) {
        Some(w) => w.to_string(),
        None => first_pinyin_or_hash(brand_name),
    }
}

/// `update` 时某一 `NOT NULL` 列的取值（实体字段是 `T`）：DTO 传了就用 DTO，
/// 没传就保留原值。
///
/// 等价于 MyBatis-Plus `FieldStrategy.NOT_NULL` 下 `updateById` 生成的
/// `SET` 子句只包含非 `null` 字段。
fn dto_or_keep<T: Clone>(from_dto: Option<T>, existing: &T) -> T {
    from_dto.unwrap_or_else(|| existing.clone())
}

/// `update` 时某一**可空列**的取值（实体字段是 `Option<T>`）：DTO 未传
/// （`None`）→ 保留库中原值（原值本身也可能是 `NULL`）；DTO 传了则覆盖。
///
/// 与 [`dto_or_keep`] 的唯一差别是「原值」的类型：可空列读回来就是
/// `Option<T>`，所以「保留原值」必须返回 `Option<T>`，不能写成 `None`
/// （那会把历史值清空）。注意 DTO 显式传空串仍是「传了值」，会照原样写入
/// —— Java 里空串不等于 `null`，不会从 `SET` 子句中剔除。
fn dto_or_keep_nullable<T: Clone>(from_dto: Option<T>, existing: &Option<T>) -> Option<T> {
    from_dto.or_else(|| existing.clone())
}

/// `update` 时 `first_word` 的取值（对应 Java `getFirstWord`）：
/// - DTO 传了非空白值 → 原样使用
/// - DTO 未传 / 传空白 → 沿用库中原值；库中也为空 → 按新名称推导（推导不出 `#`）
fn resolve_first_word(dto_value: Option<&str>, existing: Option<&str>, brand_name: &str) -> String {
    match dto_value.map(str::trim).filter(|s| !s.is_empty()) {
        Some(w) => w.to_string(),
        None => match existing.map(str::trim).filter(|s| !s.is_empty()) {
            Some(w) => w.to_string(),
            None => first_pinyin_or_hash(brand_name),
        },
    }
}

/// `create` 时由 DTO + DDL 默认值推导出的落库字段。
///
/// 与 Java `create` 逐条对齐：DTO 为 `null` 的字段**不写入**、
/// 由 `brand` 表列默认值兜底（MyBatis-Plus `FieldStrategy.NOT_NULL`）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateDefaults {
    sort_order: i32,
    is_show: i32,
    brand_is_hot: i32,
    is_store_brand: i32,
    check_status: i32,
    site_url: String,
    status: i32,
}

/// `create` 的初始审核状态（对应 Java `create`：只有 `shop_id == 0` 才显式置为通过）。
fn create_initial_status(shop_id: i32) -> i32 {
    if shop_id == 0 {
        CHECK_STATUS_APPROVED
    } else {
        CHECK_STATUS_PENDING
    }
}

/// `update` 的目标审核状态（对应 Java `update`：店铺账号重新待审核，
/// 平台账号不写 `status` 列 → 保留原值）。
fn update_target_status(shop_id: i32, existing_status: i32) -> i32 {
    if shop_id > 0 {
        CHECK_STATUS_PENDING
    } else {
        existing_status
    }
}

/// 计算 `create` 的落库默认值（纯函数，便于单测）。
fn create_defaults(dto: &BrandDTO, shop_id: i32) -> CreateDefaults {
    CreateDefaults {
        // DDL: sort_order tinyint NOT NULL DEFAULT '50'
        sort_order: dto.sort_order.unwrap_or(DEFAULT_SORT_ORDER),
        // DDL: is_show tinyint NOT NULL DEFAULT '1'
        is_show: dto.is_show.unwrap_or(DEFAULT_IS_SHOW),
        // DDL: brand_is_hot tinyint NOT NULL DEFAULT '0'
        brand_is_hot: dto.brand_is_hot.unwrap_or(0),
        // DDL: is_store_brand tinyint NOT NULL DEFAULT '0'
        is_store_brand: dto.is_store_brand.unwrap_or(0),
        // DDL: check_status smallint NOT NULL DEFAULT '0'
        check_status: dto.check_status.unwrap_or(0),
        // DDL: site_url varchar(512) NOT NULL DEFAULT ''
        site_url: dto.site_url.clone().unwrap_or_default(),
        // Java `create` 只按 shop_id 决定初始审核状态，
        // `BrandDTO.status` 不会被写入（实体 `Brand` 无该属性）
        status: create_initial_status(shop_id),
    }
}

/// 实体 → 管理端 VO（对应 Java `BrandVO` + `BrandStatusEnum.getStatusName`）。
fn to_vo(b: &Brand, shop_title: String) -> BrandVO {
    let status = BrandStatus::try_from(b.status).unwrap_or(BrandStatus::Pending);
    BrandVO {
        base: to_dto(b),
        status_text: status.name().to_string(),
        shop: Some(BrandShopVO {
            shop_id: b.shop_id,
            shop_title,
        }),
    }
}

/// 实体 → DTO（字段与 Java `BrandDTO` 同构，camelCase 序列化）。
fn to_dto(b: &Brand) -> BrandDTO {
    BrandDTO {
        brand_id: Some(b.brand_id),
        brand_name: Some(b.brand_name.clone()),
        brand_en_name: b.brand_en_name.clone(),
        brand_logo: b.brand_logo.clone(),
        brand_desc: b.brand_desc.clone(),
        site_url: Some(b.site_url.clone()),
        sort_order: Some(b.sort_order),
        is_show: Some(b.is_show),
        brand_type: b.brand_type.clone(),
        brand_is_hot: Some(b.brand_is_hot),
        first_word: b.first_word.clone(),
        is_store_brand: Some(b.is_store_brand),
        check_status: Some(b.check_status),
        shop_id: Some(b.shop_id),
        status: Some(b.status),
        reject_remark: b.reject_remark.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ———————— 名称校验 ————————

    #[test]
    fn name_must_be_non_blank() {
        assert!(validate_name(None).is_err());
        assert!(validate_name(Some("   ")).is_err());
        assert_eq!(validate_name(Some(" 苹果 ")).unwrap(), "苹果".to_string());
    }

    #[test]
    fn name_length_is_one_to_thirty_chars() {
        let ok = "a".repeat(30);
        assert!(validate_name(Some(&ok)).is_ok());
        let too_long = "a".repeat(31);
        let err = validate_name(Some(&too_long)).unwrap_err();
        assert!(err.to_string().contains("1-30位"));
        // 按字符数而非字节数（对齐 Java @Size）：15 个中文 = 15 字符，合法
        let cjk = "品".repeat(15);
        assert!(validate_name(Some(&cjk)).is_ok());
        let cjk_long = "品".repeat(31);
        assert!(validate_name(Some(&cjk_long)).is_err());
    }

    // ———————— 状态流转 ————————

    #[test]
    fn create_status_depends_on_shop_id() {
        // 平台账号（shop_id=0）直接通过；店铺账号待审核（走真实 create_defaults）
        let dto = BrandDTO {
            brand_name: Some("A".into()),
            ..Default::default()
        };
        assert_eq!(create_defaults(&dto, 0).status, CHECK_STATUS_APPROVED);
        assert_eq!(create_defaults(&dto, 5).status, CHECK_STATUS_PENDING);
        assert_eq!(create_defaults(&dto, 1).status, CHECK_STATUS_PENDING);
    }

    /// 复刻 Java `audit` 的状态白名单。
    fn audit_status_ok(status: i32) -> bool {
        status == CHECK_STATUS_APPROVED || status == CHECK_STATUS_REJECTED
    }

    #[test]
    fn audit_only_accepts_approved_or_rejected() {
        assert!(audit_status_ok(1));
        assert!(audit_status_ok(2));
        assert!(!audit_status_ok(0)); // 待审核不是合法审核目标
        assert!(!audit_status_ok(3));
    }

    #[test]
    fn audit_rejected_requires_remark() {
        // Java `StringUtils.isBlank(dto.getRejectRemark())`
        assert!(is_blank(None));
        assert!(is_blank(Some("  ")), "空白拒绝理由必须触发『请填写拒绝理由』");
        assert!(!is_blank(Some("质量不达标")));
    }

    #[test]
    fn update_status_keeps_existing_for_platform_and_pends_for_shop() {
        // 店铺账号修改 → 重新待审核；平台账号 → 保留原状态
        assert_eq!(update_target_status(5, CHECK_STATUS_APPROVED), CHECK_STATUS_PENDING);
        assert_eq!(update_target_status(0, CHECK_STATUS_APPROVED), CHECK_STATUS_APPROVED);
        assert_eq!(update_target_status(0, CHECK_STATUS_PENDING), CHECK_STATUS_PENDING);
    }

    // ———————— 首字母 ————————

    #[test]
    fn first_word_ascii_rules_match_java() {
        // Java: A-Z / a-z → 大写；0-9 → 原字符；其他 → 拼音或 '#'
        assert_eq!(first_pinyin("apple"), Some("A".into()));
        assert_eq!(first_pinyin("banana"), Some("B".into()));
        assert_eq!(first_pinyin("1号店"), Some("1".into()));
        // 无拼音库时中文回退 '#'（与 Java PinyinUtil 失败路径一致）
        assert_eq!(first_pinyin("苹果"), None);
        assert_eq!(first_pinyin_or_hash("苹果"), "#");
        assert_eq!(first_pinyin(""), None);
    }

    #[test]
    fn first_word_auto_fills_when_blank() {
        assert_eq!(first_word_or_auto(None, "Apple"), "A".to_string());
        assert_eq!(first_word_or_auto(Some("X"), "Apple"), "X".to_string());
        assert_eq!(first_word_or_auto(Some(" "), "Apple"), "A".to_string());
        assert_eq!(first_word_or_auto(None, "苹果"), "#".to_string());
    }

    #[test]
    fn update_first_word_prefers_dto_then_existing_then_derive() {
        // DTO 给了非空白值 → 以 DTO 为准（即使库中已有值）
        assert_eq!(resolve_first_word(Some("Z"), Some("A"), "Apple"), "Z");
        // DTO 未传 → 沿用库中原值，不因改名而改变
        assert_eq!(resolve_first_word(None, Some("A"), "Banana"), "A");
        // DTO 传空白 → 视为未传，同样沿用库中原值
        assert_eq!(resolve_first_word(Some("  "), Some("A"), "Banana"), "A");
        // 库中也为空 → 按新名称推导
        assert_eq!(resolve_first_word(None, None, "Banana"), "B");
        assert_eq!(resolve_first_word(None, Some(" "), "Banana"), "B");
        // 库中为空且推导不出（中文无拼音库）→ '#'
        assert_eq!(resolve_first_word(None, None, "苹果"), "#");
    }

    // ———————— create 落库默认值（MyBatis NOT_NULL 语义） ————————

    #[test]
    fn create_defaults_follow_ddl_when_dto_omits_fields() {
        let dto = BrandDTO {
            brand_name: Some("Apple".into()),
            ..Default::default()
        };
        let d = create_defaults(&dto, 0);
        // 不传 sortOrder/isShow 时必须落 DDL 默认值，
        // 否则会写成 0/0（前台不显示、排序垫底），与原系统不一致
        assert_eq!(d.sort_order, DEFAULT_SORT_ORDER);
        assert_eq!(d.sort_order, 50);
        assert_eq!(d.is_show, DEFAULT_IS_SHOW);
        assert_eq!(d.is_show, 1);
        assert_eq!(d.brand_is_hot, 0);
        assert_eq!(d.is_store_brand, 0);
        assert_eq!(d.check_status, 0);
        assert_eq!(d.site_url, "");
        // 平台账号（shop_id = 0）直接审核通过
        assert_eq!(d.status, CHECK_STATUS_APPROVED);
    }

    #[test]
    fn create_defaults_honour_explicit_dto_values() {
        let dto = BrandDTO {
            brand_name: Some("Apple".into()),
            sort_order: Some(3),
            is_show: Some(0),
            brand_is_hot: Some(1),
            is_store_brand: Some(1),
            check_status: Some(1),
            site_url: Some("https://a.example".into()),
            status: Some(2), // 前端传了也必须被忽略（Java 实体无该属性）
            ..Default::default()
        };
        let d = create_defaults(&dto, 7);
        assert_eq!(d.sort_order, 3);
        assert_eq!(d.is_show, 0);
        assert_eq!(d.brand_is_hot, 1);
        assert_eq!(d.is_store_brand, 1);
        assert_eq!(d.check_status, 1);
        assert_eq!(d.site_url, "https://a.example");
        // 店铺账号（shop_id > 0）→ 待审核，与 DTO 的 status 无关
        assert_eq!(d.status, CHECK_STATUS_PENDING);
    }

    // ———————— update 的 null 跳过语义（MyBatis NOT_NULL） ————————

    #[test]
    fn update_keeps_existing_when_dto_field_absent() {
        // DTO 未传的列必须保留原值：平台账号改名不应清空驳回理由 / Logo / 描述
        let existing_remark = Some("资质不全".to_string());
        assert_eq!(
            dto_or_keep_nullable(None::<String>, &existing_remark),
            Some("资质不全".to_string())
        );
        // DTO 传了值则覆盖（含显式传空串 —— Java 空串非 null，会被写入）
        assert_eq!(
            dto_or_keep_nullable(Some(String::new()), &existing_remark),
            Some(String::new())
        );
        // 库中原值本就是 NULL 且 DTO 未传 → 保持 NULL（不能凭空造值）
        assert_eq!(dto_or_keep_nullable(None::<String>, &None), None);
        // 数值列同理
        assert_eq!(dto_or_keep(None::<i32>, &50), 50);
        assert_eq!(dto_or_keep(Some(0), &50), 0);
    }

    // ———————— updateField 白名单 ————————

    #[test]
    fn update_field_whitelist_matches_java_constant() {
        assert_eq!(
            BRAND_FIELDS,
            ["brand_name", "first_word", "brand_is_hot", "is_show", "sort_order"]
        );
    }

    #[test]
    fn update_field_accepts_camel_and_snake() {
        assert_eq!(normalize_field("brandName").unwrap(), "brand_name");
        assert_eq!(normalize_field("brand_name").unwrap(), "brand_name");
        assert_eq!(normalize_field("isShow").unwrap(), "is_show");
        assert_eq!(normalize_field("sortOrder").unwrap(), "sort_order");
        assert!(normalize_field("1;DROP TABLE brand").is_err());
        assert!(normalize_field("brand_logo").is_err(), "不在白名单内必须拒绝");
    }

    // ———————— VO 映射 ————————

    fn brand(status: i32) -> Brand {
        Brand {
            brand_id: 7,
            brand_name: "Test".into(),
            status,
            shop_id: 3,
            ..Default::default()
        }
    }

    #[test]
    fn vo_maps_status_text_and_shop() {
        let vo = to_vo(&brand(1), "自营店铺".into());
        assert_eq!(vo.status_text, "审核通过");
        assert_eq!(vo.shop.as_ref().unwrap().shop_title, "自营店铺");
        assert_eq!(vo.base.brand_id, Some(7));
    }

    #[test]
    fn vo_tolerates_unknown_status() {
        let vo = to_vo(&brand(99), String::new());
        assert_eq!(vo.status_text, "待审核", "未知状态回退待审核文案");
    }
}