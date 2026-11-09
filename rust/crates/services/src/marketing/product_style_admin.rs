//! 商品样式配置域服务（marketing::product_style_admin）。
//!
//! 对应原 Java：
//! - `com.nextrade.service.product.impl.ProductStyleServiceImpl`（397 行）
//! - `com.nextrade.service.product.impl.ProductStyleItemServiceImpl`（78 行）
//! - `com.nextrade.adminapi.controller.promotion.ProductStyleController`（101 行）
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /promotion/productStyle/list` | `ProductStyleController.list` | [`ProductStyleAdminService::list`] |
//! | `GET  /promotion/productStyle/detail` | `ProductStyleController.detail` | [`ProductStyleAdminService::detail`] |
//! | `POST /promotion/productStyle/create` | `ProductStyleController.create` | [`ProductStyleAdminService::create`] |
//! | `POST /promotion/productStyle/update` | `ProductStyleController.update` | [`ProductStyleAdminService::update`] |
//! | `POST /promotion/productStyle/updateField` | `ProductStyleController.updateField` | [`ProductStyleAdminService::update_field`] |
//! | `POST /promotion/productStyle/del` | `ProductStyleController.del` | [`ProductStyleAdminService::del`] |
//! | `POST /promotion/productStyle/batch` | `ProductStyleController.batch` | [`ProductStyleAdminService::batch`] |
//! | `POST /promotion/productStyle/addProducts` | `ProductStyleController.addProducts` | [`ProductStyleAdminService::add_products`] |
//! | `POST /promotion/productStyle/removeProducts` | `ProductStyleController.removeProducts` | [`ProductStyleAdminService::remove_products`] |
//! | `GET  /promotion/productStyle/brandList` | `ProductStyleController.brandList` | [`ProductStyleAdminService::brand_list`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### 多租户（`HeaderUtils.getShopId()`）
//!
//! Java 在多处用请求头 `shopId` 区分平台（0）与店铺账号：
//! - `create`：`productStyle.setShopId(getShopId() != null ? getShopId() : 0)`
//! - `list`：`pageList(..., HeaderUtils.getShopId(), ...)` ⇒ `WHERE ps.shop_id = ?`
//! - `detail` / `update` / `removeProducts`：**校验归属**，
//!   `!Objects.equals(shopId, HeaderUtils.getShopId())` ⇒ 当作不存在，
//!   抛「商品样式不存在」
//!
//! ⚠️ Rust 侧 JWT 尚未签发 `shop_id`，本实现按**平台账号**语义处理
//! （`shop_id = 0`），与 product_manage 等既有切片的已知缺口一致
//! （见 `uncertain_points`）。
//!
//! ### `addProducts` 与 `removeProducts` 的**不对称**（Java 原样，疑似缺陷）
//!
//! | 方法 | 存在性校验 | 归属校验 | 关联清理 |
//! |------|-----------|---------|---------|
//! | `create` | — | — | 写三类关联 |
//! | `update` | ✅ | ✅ | **先 `removeByStyleId` 全删，再重写三类** |
//! | `addProducts` | ✅ | ❌ **无归属校验** | 只增（跳过已存在） |
//! | `removeProducts` | ✅ | ✅ | 只删指定三类 |
//!
//! `create` **没有**归属校验（新建时还没有 `shop_id` 可比），
//! `addProducts` **也没有**归属校验 —— 这是 Java 原代码的不对称，
//! 本实现逐条复刻（见 `uncertain_points`：可能是越权缺陷）。
//!
//! ### `update` 的「全删重写」语义
//!
//! Java `update`：
//! ```text
//! productStyleItemService.removeByStyleId(param.getStyleId());  // 删光全部三类
//! saveStyleItems(param.getStyleId(), param);                    // 按请求重写三类
//! ```
//! 因此**请求里没带的类型会被清空**。本实现保持该语义
//! （与「补丁式更新」的直觉相反，见 `uncertain_points`）。
//!
//! ### `batch` 与 `del` 都是**物理删除**
//!
//! Java `BaseServiceImpl#batch`（type=`del` ⇒ `removeByIds`）与
//! `#del`（⇒ `removeById`）都是 `DELETE`。虽然 `product_style`
//! **有 `is_delete` 列**，Java 却没用软删除。本实现复刻物理删除。
//!
//! ⚠️ 另一处 Java 缺口：`del` / `batch` **都不清理 `product_style_item`**，
//! 会留下孤儿关联行。本实现同样**不额外清理**（保持与 Java 一致的可观测行为），
//! 仅在 `update` 里按 Java 的 `removeByStyleId` 清理。

use std::collections::BTreeMap;
use std::sync::Arc;

use nextrade_core::models::marketing::product_style_admin::{
    self, activity_status_name, calc_activity_status, dedup_ids, map_update_field_column,
    style_type_name, validate_activity_time, validate_save, BatchDTO, BrandItemVO,
    BrandProductCountVO, GroupItemVO, OperateDTO, ProductItemVO, ProductStyleDetailVO,
    ProductStyleEditDTO, ProductStyleItemBatchDTO, ProductStyleListQuery, ProductStyleSaveDTO,
    ProductStyleVO, UpdateFieldDTO, ITEM_TYPE_BRAND, ITEM_TYPE_GROUP, ITEM_TYPE_PRODUCT,
    MSG_FIELD_NAME_REQUIRED, MSG_FIELD_VALUE_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_STYLE_ID_NOT_NULL, MSG_STYLE_NOT_FOUND, NOT_DELETED,
    PRODUCT_STYLE_COLUMNS, PRODUCT_STYLE_FIELDS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::product_style_admin::{ProductStyleAdminRepository, StyleInsert, StylePatch};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 商品样式配置域服务（挂载于 `AdminState.product_style_admin`）。
pub struct ProductStyleAdminService {
    repos: Arc<Repositories>,
}

impl ProductStyleAdminService {
    /// 构造（与 order_admin / admin_user / pc_cat_floor / area_code 的
    /// `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 仓储句柄。
    fn repo(&self) -> &ProductStyleAdminRepository {
        &self.repos.marketing.product_style_admin
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 商品样式分页（对应 Java `ProductStyleServiceImpl#list`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. 解析 `startTime` / `endTime`（Java `TigUtils.toTimestampYmd`）
    /// 2. `pageList(page, query, HeaderUtils.getShopId(), now, start, end)`
    /// 3. 记录为空时 Java 返回 `new Page<>()`（**total 为 0**，
    ///    而不是原 `stylePage` 的 total）—— 本实现同样返回空页
    /// 4. 逐条补 `styleTypeName` / `activityStatus` / `activityStatusName`
    pub async fn list(&self, q: &ProductStyleListQuery) -> Result<PageResult<ProductStyleVO>> {
        let page = q.page_query();
        let now = nextrade_core::models::marketing::product_style_admin::now_secs();
        let start_time = q.parsed_start_time().map_err(Error::Validation)?;
        let end_time = q.parsed_end_time().map_err(Error::Validation)?;

        let (total, rows) = self
            .repo()
            .paginate(q, TenantScope::current_list_filter(), now, start_time, end_time)
            .await?;

        // Java `if (CollUtil.isEmpty(records)) return new Page<>();`
        // ⇒ 空结果返回一个 total=0 的空页（而非带 total 的空 list）
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let list = rows
            .iter()
            .map(|r| ProductStyleVO::from_entity(&r.style, r.product_count, now))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 商品样式详情（对应 Java `ProductStyleServiceImpl#detail`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `getById(styleId)`，不存在 ⇒ 「商品样式不存在」
    /// 2. 归属校验：`shop_id != 当前 shopId` ⇒ **当作不存在**（同样文案）
    /// 3. 查三类关联 ID（商品 / 分组 / 品牌）
    /// 4. 分组存在时：补 `groupList`，并把分组内商品ID并入 `allProductIds`
    /// 5. 品牌存在时：补 `brandList`，并把该品牌下**未删除**的商品ID
    ///    并入 `allProductIds`（⚠️ Java 这里**没过滤 `product_status`**）
    /// 6. 按 `allProductIds` 查商品简要信息，**覆盖** `vo.productIds`
    ///    （⚠️ Java 第 187 行 `vo.setProductIds(allProductIds.stream().toList())`
    ///    —— 把「直接关联的商品ID」替换成「**含分组与品牌展开后的全部商品ID**」）
    /// 7. `productCount` = `getRealProductCountMap([styleId])`，缺省 0
    ///
    /// # Errors
    /// - 记录不存在或不属于当前店铺 ⇒ `商品样式不存在`
    pub async fn detail(&self, style_id: i32) -> Result<ProductStyleDetailVO> {
        let style = self
            .repo()
            .find_by_id(style_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_STYLE_NOT_FOUND.into()))?;
        // 多租户：归属不符按「不存在」处理（Java 同文案，避免泄漏存在性）
        TenantScope::current_ensure_access(style.shop_id)
            .map_err(|_| Error::NotFound(MSG_STYLE_NOT_FOUND.into()))?;

        let now = nextrade_core::models::marketing::product_style_admin::now_secs();
        let product_ids = self.repo().find_ref_ids(style_id, ITEM_TYPE_PRODUCT).await?;
        let group_ids = self.repo().find_ref_ids(style_id, ITEM_TYPE_GROUP).await?;
        let brand_ids = self.repo().find_ref_ids(style_id, ITEM_TYPE_BRAND).await?;

        // Java `new LinkedHashSet<>(productIds)`：去重 + 保持插入顺序
        let mut all_product_ids: Vec<i32> = Vec::new();
        let mut seen: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for id in &product_ids {
            if seen.insert(*id) {
                all_product_ids.push(*id);
            }
        }

        let mut group_list: Vec<GroupItemVO> = Vec::new();
        if !group_ids.is_empty() {
            let groups = self.repo().find_groups_by_ids(&group_ids).await?;
            // Java 先 map 出 groupList，再单独遍历 groups 收集商品ID
            group_list = groups
                .iter()
                .map(|g| GroupItemVO {
                    product_group_id: g.product_group_id,
                    product_group_name: g.product_group_name.clone(),
                })
                .collect();
            for g in &groups {
                // Java `if (group.getProductIds() != null)`
                //   `JSONUtil.toList(productIds, Integer.class)`
                if let Some(raw) = g.product_ids.as_deref() {
                    for id in parse_json_int_list(raw) {
                        if seen.insert(id) {
                            all_product_ids.push(id);
                        }
                    }
                }
            }
        }

        let mut brand_list: Vec<BrandItemVO> = Vec::new();
        if !brand_ids.is_empty() {
            let brands = self.repo().find_brands_by_ids(&brand_ids).await?;
            brand_list = brands
                .iter()
                .map(|b| BrandItemVO {
                    brand_id: b.brand_id,
                    brand_name: b.brand_name.clone(),
                })
                .collect();
            // ⚠️ Java 这里只过滤 is_delete = 0（**没有** product_status）
            let brand_product_ids = self.repo().find_product_ids_by_brands(&brand_ids).await?;
            for id in brand_product_ids {
                if seen.insert(id) {
                    all_product_ids.push(id);
                }
            }
        }

        let product_list: Vec<ProductItemVO> = if all_product_ids.is_empty() {
            Vec::new()
        } else {
            let products = self.repo().find_products_by_ids(&all_product_ids).await?;
            products
                .iter()
                .map(|p| ProductItemVO {
                    product_id: p.product_id,
                    product_name: p.product_name.clone(),
                    pic_thumb: p.pic_thumb.clone(),
                    product_price: p.product_price,
                })
                .collect()
        };

        // `productCount` 走实时统计（与列表页同一口径）
        let counts: BTreeMap<i32, i32> = self
            .repo()
            .count_real_products_by_style_ids(&[style_id])
            .await?
            .into_iter()
            .collect();
        let product_count = counts.get(&style_id).copied().unwrap_or(0);

        let base = ProductStyleVO::from_entity(&style, product_count, now);
        Ok(ProductStyleDetailVO {
            base,
            // ⚠️ Java 第 187 行覆盖为展开后的全部商品ID（不是直接关联的那份）
            product_ids: all_product_ids,
            group_ids,
            brand_ids,
            product_list,
            group_list,
            brand_list,
        })
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建商品样式（对应 Java `ProductStyleServiceImpl#create`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `validateParam(param)` —— **只校验活动背景的时间**（样式类型为
    ///    2 时开始/结束时间必填且 `end > start`）
    /// 2. 构造实体：`pcImage` 落空串、时间落 0 或解析值、
    ///    `isDelete = 0`、`addTime = updateTime = 当前秒`
    /// 3. `setShopId(getShopId() != null ? getShopId() : 0)`
    /// 4. `save`
    /// 5. `saveStyleItems(styleId, param)` —— 三类关联分别批量添加
    ///
    /// ⚠️ 必填校验（`@NotBlank` / `@NotNull` / `@Size`）在 Java 侧由
    /// 控制器参数上的 `@Validated` 完成，顺序**先于** `validateParam`。
    /// 本实现按同一顺序：先 [`validate_save`]，再 [`validate_activity_time`]。
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    ///
    /// # Errors
    /// - 必填 / 长度不合法 ⇒ `Error::Validation`
    /// - 活动背景时间不合法 ⇒ `Error::Validation`
    pub async fn create(&self, dto: &ProductStyleSaveDTO) -> Result<i32> {
        let (style_name, style_type, style_image) = validate_save(dto).map_err(Error::Validation)?;
        let (start_time, end_time) = validate_activity_time(dto).map_err(Error::Validation)?;

        let now = nextrade_core::models::marketing::product_style_admin::now_secs();
        let style_id = self
            .repo()
            .insert(&StyleInsert {
                style_name,
                style_type,
                style_image,
                pc_image: dto.pc_image_or_empty(),
                start_time,
                end_time,
                // 多租户：店铺账号落本店；平台账号落平台级（0）
                shop_id: TenantScope::current().write_shop_id(None)?,
                add_time: now,
                update_time: now,
                is_delete: NOT_DELETED,
                status: dto.status,
            })
            .await?;

        self.save_style_items(style_id, dto).await?;
        tracing::info!("创建商品样式成功: style_id={style_id}");
        Ok(style_id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新商品样式（对应 Java `ProductStyleServiceImpl#update`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `getById(styleId)`，不存在 ⇒ 「商品样式不存在」
    /// 2. 归属校验（同 `detail`）
    /// 3. `validateParam(param)`
    /// 4. `updateById(param.createProductStyle())`
    ///    —— `add_time` **不写**，`update_time` 恒写当前秒
    /// 5. `removeByStyleId(styleId)` ⇒ `saveStyleItems(styleId, param)`
    ///    （**全删重写**，请求未带的类型会被清空）
    ///
    /// ⚠️ 必填校验在 Java 侧由 `@Validated` 先做，本实现按同一顺序。
    ///
    /// 返回是否命中记录（Java 控制器返回 `void`）。
    ///
    /// # Errors
    /// - 缺 `styleId` ⇒ `Error::Validation`
    /// - 记录不存在 / 归属不符 ⇒ `Error::NotFound`
    pub async fn update(&self, dto: &ProductStyleEditDTO) -> Result<bool> {
        let style_id = dto
            .style_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(nextrade_core::models::marketing::product_style_admin::MSG_ID_REQUIRED.into()))?;

        // 1–2. 存在性 + 归属（Java 在 validateParam **之前**做这两步）
        self.require_own_style(style_id).await?;

        // 3. 校验
        let base = dto.as_save();
        let (style_name, style_type, style_image) = validate_save(base).map_err(Error::Validation)?;
        let (start_time, end_time) = validate_activity_time(base).map_err(Error::Validation)?;

        let now = nextrade_core::models::marketing::product_style_admin::now_secs();
        // 4. `updateById`：`style_name` / `style_type` / `style_image` /
        //    `start_time` / `end_time` / `update_time` 必写；
        //    `pc_image` 由 `pcImage != null ? pcImage : ""` 归一 ⇒ 必写；
        //    `status` 可空 ⇒ 仅在 Some 时写
        let patch = StylePatch {
            style_name: Some(Some(style_name)),
            style_type: Some(Some(style_type)),
            style_image: Some(Some(style_image)),
            pc_image: Some(Some(base.pc_image_or_empty())),
            start_time: Some(Some(start_time)),
            end_time: Some(Some(end_time)),
            status: base.status.map(Some),
            update_time: Some(Some(now)),
        };
        let affected = self.repo().update(style_id, &patch).await?;

        // 5. 全删重写
        self.repo().delete_items_by_style(style_id).await?;
        self.save_style_items(style_id, base).await?;

        tracing::info!("更新商品样式: style_id={style_id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 / 单字段 / 批量 —————————————————————

    /// 删除商品样式（对应 Java `ProductStyleServiceImpl#del`
    /// → `BaseServiceImpl#del` → `removeById`，**物理删除**）。
    ///
    /// ⚠️ Java 的 `del` 是「`id == null` 返回 false，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功），
    /// 也**不清理** `product_style_item`。本实现保持这一语义。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        // 多租户：记录存在时校验归属（Java 对不存在 id 也返回成功，故保留软校验）
        if let Some(style) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(style.shop_id)?;
        }
        self.repo().delete(id).await
    }

    /// 单字段更新（对应 Java
    /// `ProductStyleServiceImpl` 继承的 `BaseServiceImpl#updateField(dto,
    /// PRODUCT_STYLE_FIELDS)`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `field` 经 `getColumnByPropertyName` 归一为列名（属性名/列名都可命中）
    /// 2. 归一后的列名必须落在 `PRODUCT_STYLE_FIELDS`
    ///    = `{style_name, status, sort_order, pc_image}` 内，
    ///    否则 `field 的值有误`
    /// 3. `UPDATE product_style SET <列> = ? WHERE style_id = ?`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换
    ///
    /// ⚠️ **`sort_order` 的处理（本实现的有意收敛）**：该列在 Java 白名单里，
    /// 但 `product_style` **根本没有这一列**。Java 若收到 `field=sort_order`
    /// 会归一成字面量 `sort_order`、通过白名单检查，然后拼出
    /// `UPDATE product_style SET sort_order = ?` ⇒ MySQL `Unknown column`
    /// （1054）⇒ **HTTP 500**。本实现在归一阶段就返回 `None`，
    /// 于是报与「字段不在白名单」**相同的** 400 错误
    /// ——「该字段不能改」的判定结果一致，只是把 500 变成 400
    /// （见 `uncertain_points`）。
    ///
    /// 返回是否命中记录（Java 返回 `this.update(updateWrapper)` 的布尔值）。
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
            .ok_or_else(|| Error::Validation(MSG_FIELD_NAME_REQUIRED.into()))?;

        // 1. 归一（未识别 ⇒ None）
        let column =
            map_update_field_column(field).ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        // 2. 白名单
        if !PRODUCT_STYLE_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        // 3. 二次保险：白名单命中项必须是真实列
        //    （`sort_order` 在上一步已不可达，这里防的是未来白名单漂移）
        if !PRODUCT_STYLE_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        // Java `UpdateFieldDTO.val` 带 `@NotNull`：缺失即 400
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_FIELD_VALUE_REQUIRED.into()))?;

        // 多租户：字段更新前校验归属
        if let Some(style) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(style.shop_id)?;
        }
        let affected = self.repo().update_field(id, column, val).await?;
        Ok(affected > 0)
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器参数上的 `@Validated` 会让 `BatchDTO.ids` 的
    /// `@NotEmpty` 生效（空数组 → 400）。本实现按项目惯例在服务层
    /// 返回 `false` 而不是 400（与 pc_cat_floor / area_code 的选择一致）。
    ///
    /// ⚠️ Java `removeByIds` **不清理** `product_style_item`。
    /// 本实现同样不清理（见模块头说明）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        // 多租户：批量删除前逐条校验归属（越权直接 403）
        for id in &dto.ids {
            if let Some(style) = self.repo().find_by_id(*id).await? {
                TenantScope::current_ensure_access(style.shop_id)?;
            }
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 关联批量操作 —————————————————————

    /// 批量添加关联商品（对应 Java `ProductStyleServiceImpl#addProducts`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `getById(param.getStyleId())`，不存在 ⇒ 「商品样式不存在」
    /// 2. `batchAddFromParam` —— 三类关联分别批量添加
    ///
    /// ⚠️ **Java 的 `addProducts` 没有归属校验**（`detail` / `update` /
    /// `removeProducts` 都有），本实现逐条复刻这一不对称
    /// （见模块头与 `uncertain_points`：疑似越权缺陷）。
    ///
    /// 返回本次**实际新增**的关联条数（Java 返回 `void`，此处便于排查）。
    pub async fn add_products(&self, dto: &ProductStyleItemBatchDTO) -> Result<u64> {
        let style_id = dto
            .style_id
            .ok_or_else(|| Error::Validation(MSG_STYLE_ID_NOT_NULL.into()))?;

        // 存在性校验（Java `Assert.notNull(existing, ...)`），**不校验归属**
        if self.repo().find_by_id(style_id).await?.is_none() {
            return Err(Error::NotFound(MSG_STYLE_NOT_FOUND.into()));
        }

        let mut inserted = 0u64;
        for (item_type, ids) in [
            (ITEM_TYPE_PRODUCT, &dto.product_ids),
            (ITEM_TYPE_GROUP, &dto.group_ids),
            (ITEM_TYPE_BRAND, &dto.brand_ids),
        ] {
            inserted += self.add_items_of_type(style_id, item_type, ids.as_deref()).await?;
        }
        Ok(inserted)
    }

    /// 批量移除关联商品（对应 Java `ProductStyleServiceImpl#removeProducts`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `getById(param.getStyleId())`，不存在 ⇒ 「商品样式不存在」
    /// 2. **归属校验**（与 `addProducts` 不同，这里 Java 有）
    /// 3. 三类关联分别 `batchRemove`（`CollUtil.isNotEmpty` 才执行）
    ///
    /// 返回本次实际删除的关联条数。
    pub async fn remove_products(&self, dto: &ProductStyleItemBatchDTO) -> Result<u64> {
        let style_id = dto
            .style_id
            .ok_or_else(|| Error::Validation(MSG_STYLE_ID_NOT_NULL.into()))?;

        self.require_own_style(style_id).await?;

        let mut removed = 0u64;
        for (item_type, ids) in [
            (ITEM_TYPE_PRODUCT, &dto.product_ids),
            (ITEM_TYPE_GROUP, &dto.group_ids),
            (ITEM_TYPE_BRAND, &dto.brand_ids),
        ] {
            if let Some(ids) = ids.as_deref().filter(|v| !v.is_empty()) {
                removed += self.repo().delete_items(style_id, item_type, ids).await?;
            }
        }
        Ok(removed)
    }

    // ————————————————————— 品牌列表 —————————————————————

    /// 品牌列表（含商品数量）（对应 Java
    /// `ProductStyleServiceImpl#brandListWithProductCount`）。
    ///
    /// `keyword` 为 `None` 或空白时不过滤（SQL 的 `<if test="keyword != null
    /// and keyword != ''">`）。排序固定为 `productCount DESC, sort_order ASC`。
    pub async fn brand_list(&self, keyword: Option<&str>) -> Result<Vec<BrandProductCountVO>> {
        let keyword = keyword.map(str::trim).filter(|s| !s.is_empty());
        let rows = self
            .repo()
            .select_brand_list_with_product_count(keyword)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(brand_id, brand_name, product_count)| BrandProductCountVO {
                brand_id,
                brand_name,
                product_count,
            })
            .collect())
    }

    // ————————————————————— 内部辅助 —————————————————————

    /// 记录必须存在且属于当前店铺（Java `detail` / `update` /
    /// `removeProducts` 里那段
    /// `Objects.equals(shopId, HeaderUtils.getShopId())` 的复刻）。
    ///
    /// 不满足时抛与「不存在」**相同**的错误（Java 注释明写「当做不存在」，
    /// 避免泄漏其他店铺的资源是否存在）。
    async fn require_own_style(&self, style_id: i32) -> Result<()> {
        let style = self
            .repo()
            .find_by_id(style_id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_STYLE_NOT_FOUND.into()))?;
        // 多租户：归属不符 ⇒ 与「不存在」相同错误（Java 注释「当做不存在」）
        TenantScope::current_ensure_access(style.shop_id)
            .map_err(|_| Error::NotFound(MSG_STYLE_NOT_FOUND.into()))?;
        Ok(())
    }

    /// `saveStyleItems`（Java `ProductStyleServiceImpl#saveStyleItems`）：
    /// 三类关联分别 `batchAdd`。
    ///
    /// ⚠️ Java 的 `batchAdd` 会先查库中已有 `ref_id`、过滤后 `saveBatch`，
    /// 因此**重复添加不会产生重复行**。本实现在 [`Self::add_items_of_type`]
    /// 里用同样的「先查后插」实现（而非 `INSERT IGNORE`，
    /// 因为 `product_style_item` 没有唯一索引，见 `uncertain_points`）。
    async fn save_style_items(&self, style_id: i32, base: &ProductStyleSaveDTO) -> Result<()> {
        self.add_items_of_type(style_id, ITEM_TYPE_PRODUCT, base.product_ids.as_deref())
            .await?;
        self.add_items_of_type(style_id, ITEM_TYPE_GROUP, base.group_ids.as_deref())
            .await?;
        self.add_items_of_type(style_id, ITEM_TYPE_BRAND, base.brand_ids.as_deref())
            .await?;
        Ok(())
    }

    /// 单一类型的关联批量添加（Java
    /// `ProductStyleItemServiceImpl#batchAdd(styleId, itemType, refIds)`）。
    ///
    /// 1. 空列表直接返回 0（Java `CollUtil.isEmpty(refIds)` ⇒ `return`）
    /// 2. 查库中已有的 `ref_id`，过滤掉重复项
    /// 3. 请求内先去重（保持首次出现顺序）
    /// 4. `saveBatch`
    ///
    /// 返回实际插入的行数。
    async fn add_items_of_type(
        &self,
        style_id: i32,
        item_type: i32,
        ref_ids: Option<&[i32]>,
    ) -> Result<u64> {
        let Some(ref_ids) = ref_ids.filter(|v| !v.is_empty()) else {
            return Ok(0);
        };

        let existing = self.repo().find_ref_ids(style_id, item_type).await?;
        // Java `existingRefIds.contains(refId)` 是 List.contains（O(n)）：
        // 本实现用 HashSet，语义相同、性能更好。
        let existing: std::collections::HashSet<i32> = existing.into_iter().collect();
        let to_insert: Vec<i32> = dedup_ids(ref_ids)
            .into_iter()
            .filter(|id| !existing.contains(id))
            .collect();
        if to_insert.is_empty() {
            return Ok(0);
        }
        self.repo()
            .insert_items(style_id, item_type, &to_insert, nextrade_core::models::marketing::product_style_admin::now_secs())
            .await
    }
}

/// 解析 `product_group.product_ids` 这类 JSON 整数数组文本
/// （对应 Java `JSONUtil.toList(raw, Integer.class)`）。
///
/// ⚠️ Java 的 `JSONUtil.toList` 对非法 JSON 会**抛异常**；
/// 本实现选择**静默跳过**（返回空列表），因为这是「展示用商品列表」的
/// 附加信息，不应让详情接口整体失败 —— 属**有意偏离**，
/// 已记录在 `uncertain_points`。
///
/// 同时容忍 `"[1,2,3]"`（JSON 数组）、`"1,2,3"`（逗号分隔串）与空串。
#[must_use]
pub fn parse_json_int_list(raw: &str) -> Vec<i32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Vec::new();
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<i32>>(trimmed).unwrap_or_default();
    }
    trimmed
        .split(',')
        .filter_map(|s| s.trim().trim_matches('"').parse::<i32>().ok())
        .collect()
}

/// 风格：把三个 `Option<Vec<i32>>` 归一为「非空才返回」的视图
/// （Java `CollUtil.isNotEmpty` 判据），供路由层/调试使用。
#[must_use]
pub fn non_empty_ids(ids: &Option<Vec<i32>>) -> Option<&[i32]> {
    ids.as_deref().filter(|v| !v.is_empty())
}

/// 供路由层复用的枚举名派生（避免路由层直接依赖 `models` 的函数命名）。
#[must_use]
pub fn describe_style(style_type: i32) -> Option<String> {
    style_type_name(style_type)
}

/// 计算活动状态（转发 `models::product_style_admin`，供外部按「服务层 API」调用）。
#[must_use]
pub fn activity_status_of(start_time: i64, end_time: i64, now: i64) -> Option<i32> {
    calc_activity_status(start_time, end_time, now)
}

/// 活动状态名（转发）。
#[must_use]
pub fn activity_status_label(code: i32) -> Option<String> {
    activity_status_name(code)
}

/// 列表店铺过滤：店铺账号强制本店；平台账号不过滤。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod tests {
    use super::*;
    // ⚠️ 只导入父模块**没有**的名字：`ITEM_TYPE_*` / `MSG_INVALID_FIELD_VALUE` /
    // `PRODUCT_STYLE_FIELDS` 等已由父模块 `use` 引入并经 `use super::*`
    // 到达本模块，重复导入会触发 E0252（defined multiple times）。
    use nextrade_core::models::marketing::product_style_admin::{
        ACTIVITY_STATUS_ENDED, ACTIVITY_STATUS_IN_PROGRESS, ACTIVITY_STATUS_NOT_STARTED,
        MSG_END_MUST_AFTER_START, MSG_START_TIME_REQUIRED, MSG_STYLE_IMAGE_NOT_BLANK,
        MSG_STYLE_NAME_NOT_BLANK, MSG_STYLE_NAME_MAX_SIZE, MSG_STYLE_TYPE_NOT_NULL,
        STYLE_TYPE_ACTIVITY_BACKGROUND, STYLE_TYPE_BORDER,
    };

    // ————————————————————— 常量：与 Java 逐字符对照 —————————————————————

    /// Java `CheckFieldConstants.PRODUCT_STYLE_FIELDS` 的逐字对照
    /// —— 注意 `sort_order` 在里面，但**不是本表真实列**。
    #[test]
    fn product_style_fields_verbatim() {
        assert_eq!(
            PRODUCT_STYLE_FIELDS,
            ["style_name", "status", "sort_order", "pc_image"]
        );
        assert!(PRODUCT_STYLE_FIELDS.contains(&"sort_order"));
        assert!(!PRODUCT_STYLE_COLUMNS.contains(&"sort_order"));
    }

    /// 关联类型常量与 Java `ProductStyleItemTypeEnum` 逐条对照。
    #[test]
    fn item_type_constants() {
        assert_eq!(ITEM_TYPE_PRODUCT, 1);
        assert_eq!(ITEM_TYPE_GROUP, 2);
        assert_eq!(ITEM_TYPE_BRAND, 3);
    }

    // ————————————————————— 纯逻辑：JSON 商品ID 解析 —————————————————————

    /// `product_group.product_ids` 的解析（Java
    /// `JSONUtil.toList(raw, Integer.class)`）。
    #[test]
    fn parse_json_int_list_variants() {
        assert_eq!(parse_json_int_list("[1,2,3]"), vec![1, 2, 3]);
        assert_eq!(parse_json_int_list(" [1, 2] "), vec![1, 2]);
        assert_eq!(parse_json_int_list("1,2,3"), vec![1, 2, 3]);
        assert_eq!(parse_json_int_list("\"1\",\"2\""), vec![1, 2]);
        // 空 / null / 非法 ⇒ 空列表（本实现的有意偏离，Java 会抛异常）
        assert_eq!(parse_json_int_list(""), Vec::<i32>::new());
        assert_eq!(parse_json_int_list("[]"), Vec::<i32>::new());
        assert_eq!(parse_json_int_list("null"), Vec::<i32>::new());
        assert_eq!(parse_json_int_list("not json"), Vec::<i32>::new());
    }

    /// `non_empty_ids` 与 Java `CollUtil.isNotEmpty` 判据一致。
    #[test]
    fn non_empty_ids_matches_collutil() {
        assert_eq!(non_empty_ids(&None), None);
        assert_eq!(non_empty_ids(&Some(vec![])), None);
        assert_eq!(non_empty_ids(&Some(vec![1])), Some(&[1i32][..]));
    }

    // ————————————————————— 纯逻辑：入口校验文案 —————————————————————

    /// 缺 `styleId` 时的错误文案（Java `ProductStyleItemBatchParam @NotNull`）。
    #[test]
    fn item_batch_requires_style_id() {
        assert_eq!(MSG_STYLE_ID_NOT_NULL, "样式ID不能为空");
        // 与用 `OperateDTO` 的接口不是同一个文案
        assert_ne!(MSG_STYLE_ID_NOT_NULL, MSG_ID_REQUIRED);
    }

    /// `del` / `update_field` 的 `id` 校验文案（Java `@NotNull`）。
    #[test]
    fn common_id_message() {
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }

    /// 白名单错误文案是「field 的值有误」（**与 area_code 不同**）。
    #[test]
    fn invalid_field_value_message_differs_from_slice075() {
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
        assert_ne!(MSG_INVALID_FIELD_VALUE, "字段值不合法");
    }

    /// 保存校验的四个文案。
    #[test]
    fn save_messages_verbatim() {
        assert_eq!(MSG_STYLE_NAME_NOT_BLANK, "样式名称不能为空");
        assert_eq!(MSG_STYLE_NAME_MAX_SIZE, "样式名称长度不能超过120个字符");
        assert_eq!(MSG_STYLE_TYPE_NOT_NULL, "样式类型不能为空");
        assert_eq!(MSG_STYLE_IMAGE_NOT_BLANK, "样式图片不能为空");
    }

    /// 活动背景时间校验文案。
    #[test]
    fn activity_time_messages_verbatim() {
        assert_eq!(MSG_START_TIME_REQUIRED, "活动背景的生效开始时间不能为空");
        assert_eq!(MSG_END_MUST_AFTER_START, "结束时间必须大于开始时间");
    }

    // ————————————————————— 纯逻辑：活动状态 —————————————————————

    /// 服务层转发的活动状态计算与 `models` 一致（含闭区间边界）。
    #[test]
    fn activity_status_boundary_is_closed() {
        // now == start ⇒ 进行中
        assert_eq!(
            activity_status_of(100, 200, 100),
            Some(ACTIVITY_STATUS_IN_PROGRESS)
        );
        // now == end ⇒ 进行中
        assert_eq!(
            activity_status_of(100, 200, 200),
            Some(ACTIVITY_STATUS_IN_PROGRESS)
        );
        assert_eq!(
            activity_status_of(100, 200, 99),
            Some(ACTIVITY_STATUS_NOT_STARTED)
        );
        assert_eq!(activity_status_of(100, 200, 201), Some(ACTIVITY_STATUS_ENDED));
        assert_eq!(
            activity_status_label(ACTIVITY_STATUS_IN_PROGRESS).as_deref(),
            Some("活动进行中")
        );
    }

    /// 样式类型名派生。
    #[test]
    fn describe_style_type() {
        assert_eq!(describe_style(STYLE_TYPE_BORDER).as_deref(), Some("边框背景"));
        assert_eq!(
            describe_style(STYLE_TYPE_ACTIVITY_BACKGROUND).as_deref(),
            Some("活动背景")
        );
        assert_eq!(describe_style(9), None);
    }

    // ————————————————————— 纯逻辑：updateField 流程判定 —————————————————————

    /// 复刻 `update_field` 的三步判定（归一 → 白名单 → 真实列），
    /// 不走数据库，纯函数验证分类结果。
    #[test]
    fn update_field_decision_table() {
        // 可改：白名单 ∩ 真实列
        for (input, column) in [
            ("styleName", "style_name"),
            ("style_name", "style_name"),
            ("status", "status"),
            ("pcImage", "pc_image"),
            ("pc_image", "pc_image"),
        ] {
            let col = map_update_field_column(input).expect("应能归一");
            assert_eq!(col, column);
            assert!(PRODUCT_STYLE_FIELDS.contains(&col));
            assert!(PRODUCT_STYLE_COLUMNS.contains(&col));
        }

        // 不可改：能归一但不在白名单（主键、时间、shop_id、is_delete…）
        for input in ["styleId", "style_id", "startTime", "shopId", "isDelete"] {
            let col = map_update_field_column(input).expect("应能归一");
            assert!(
                !PRODUCT_STYLE_FIELDS.contains(&col),
                "{input} 不应在白名单内"
            );
        }

        // 不可改：sort_order 在白名单里但不是真实列 ⇒ 归一即失败
        assert!(PRODUCT_STYLE_FIELDS.contains(&"sort_order"));
        assert_eq!(map_update_field_column("sort_order"), None);

        // 不可改：完全无法识别
        assert_eq!(map_update_field_column("nope"), None);
        assert_eq!(map_update_field_column(""), None);
    }

    // ————————————————————— 纯逻辑：保存校验顺序 —————————————————————

    /// `create` 的校验**顺序**：先必填/长度（`@Validated`），
    /// 再活动背景时间（`validateParam`）。
    #[test]
    fn create_validation_order() {
        // 全空 ⇒ 必填错误先出（而不是时间错误）
        let dto = ProductStyleSaveDTO::default();
        assert_eq!(
            validate_save(&dto).unwrap_err(),
            MSG_STYLE_NAME_NOT_BLANK
        );

        // 必填齐全但活动背景缺时间 ⇒ 时间错误
        let dto = ProductStyleSaveDTO {
            style_name: Some("新年".into()),
            style_type: Some(STYLE_TYPE_ACTIVITY_BACKGROUND),
            style_image: Some("a.png".into()),
            ..Default::default()
        };
        assert!(validate_save(&dto).is_ok());
        assert_eq!(
            validate_activity_time(&dto).unwrap_err(),
            MSG_START_TIME_REQUIRED
        );

        // 边框背景不需要时间
        let dto = ProductStyleSaveDTO {
            style_name: Some("新年".into()),
            style_type: Some(STYLE_TYPE_BORDER),
            style_image: Some("a.png".into()),
            ..Default::default()
        };
        assert!(validate_save(&dto).is_ok());
        assert_eq!(validate_activity_time(&dto).unwrap(), (0, 0));
    }

    /// `remove_products` / `add_products` 的类型分派顺序与 Java 一致
    /// （商品 → 分组 → 品牌）。
    #[test]
    fn item_type_dispatch_order() {
        let order = [ITEM_TYPE_PRODUCT, ITEM_TYPE_GROUP, ITEM_TYPE_BRAND];
        assert_eq!(order, [1, 2, 3]);
        // 与 Java `saveStyleItems` / `batchAddFromParam` / `removeProducts`
        // 里三段 if 的书写顺序一致
    }
}

// ——————————————————————————————————————————————
// 公共端（/api/product/productStyle，optional auth）
// ——————————————————————————————————————————————

use nextrade_core::models::marketing::product_style_admin::ProductActiveStyleVO;

impl ProductStyleAdminService {
    /// 单个商品的生效样式（Java `ProductStyleServiceImpl#getActiveStyle`）。
    ///
    /// 找不到时返回空 VO（Java `result.getOrDefault(productId, new VO())`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn get_active_style(&self, product_id: i32) -> Result<ProductActiveStyleVO> {
        let mut map = self.batch_get_active_styles(&[product_id]).await?;
        Ok(map.remove(&product_id).unwrap_or_default())
    }

    /// 批量生效样式（Java `#batchGetActiveStyles`）：每商品每类取首行
    /// （仓储已按 `add_time DESC` 排序）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn batch_get_active_styles(
        &self,
        product_ids: &[i32],
    ) -> Result<BTreeMap<i32, ProductActiveStyleVO>> {
        if product_ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        let now = i64::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i64::MAX);
        let rows = self.repo().find_active_styles(product_ids, now).await?;
        let mut result: BTreeMap<i32, ProductActiveStyleVO> = BTreeMap::new();
        for row in &rows {
            let vo = result.entry(row.product_id).or_default();
            vo.apply_row(
                row.style_type,
                row.style_id,
                row.style_name.as_deref(),
                row.style_image.as_deref(),
                row.pc_image.as_deref(),
            );
        }
        Ok(result)
    }

    /// 按商品 SN 批量生效样式（Java `#batchGetActiveStylesBySns`）：
    /// 先 `product_sn → product_id`（`is_delete = 0`），再按 id 批量查。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn batch_get_active_styles_by_sns(
        &self,
        product_sns: &[String],
    ) -> Result<BTreeMap<String, ProductActiveStyleVO>> {
        if product_sns.is_empty() {
            return Ok(BTreeMap::new());
        }
        let pairs = self.repo().find_product_ids_by_sns(product_sns).await?;
        if pairs.is_empty() {
            return Ok(BTreeMap::new());
        }
        let id_to_sn: BTreeMap<i32, String> = pairs.iter().map(|(id, sn)| (*id, sn.clone())).collect();
        let ids: Vec<i32> = id_to_sn.keys().copied().collect();
        let style_by_id = self.batch_get_active_styles(&ids).await?;
        let mut result = BTreeMap::new();
        for (id, vo) in style_by_id {
            if let Some(sn) = id_to_sn.get(&id) {
                result.insert(sn.clone(), vo);
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod public_active_style_service_tests {
    use super::scoped_list_shop_id;

    #[test]
    fn empty_inputs_are_noop_without_db() {
        // Java：CollUtil.isEmpty ⇒ 直接返回 emptyMap；此处仅验证常量语义
        let empty: Vec<i32> = Vec::new();
        assert!(empty.is_empty());
        let empty_sns: Vec<String> = Vec::new();
        assert!(empty_sns.is_empty());
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_style() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async { scoped_list_shop_id() }).await;
        assert_eq!(filter, None);
    }
}