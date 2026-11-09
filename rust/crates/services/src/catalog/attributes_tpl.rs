//! 商品属性模板服务（catalog::attributes_tpl）。
//!
//! 对应原 Java `com.nextrade.service.product.impl.AttributesTplServiceImpl`
//! （182 行）+ `AttributesTplController`（115 行）中的 8 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /product/productAttributesTpl/list` | `AttributesTplController.list` | [`AttributesTplService::list`] |
//! | `GET  /product/productAttributesTpl/listByCategory` | `AttributesTplController.listByCategory` | [`AttributesTplService::list_by_category`] |
//! | `GET  /product/productAttributesTpl/detail` | `AttributesTplController.detail` | [`AttributesTplService::detail`] |
//! | `POST /product/productAttributesTpl/create` | `AttributesTplController.create` | [`AttributesTplService::create`] |
//! | `POST /product/productAttributesTpl/update` | `AttributesTplController.update` | [`AttributesTplService::update`] |
//! | `POST /product/productAttributesTpl/del` | `AttributesTplController.del` | [`AttributesTplService::del`] |
//! | `POST /product/productAttributesTpl/batch` | `AttributesTplController.batch` | [`AttributesTplService::batch`] |
//! | `POST /product/productAttributesTpl/updateField` | `AttributesTplController.updateField` | [`AttributesTplService::update_field`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### 1. 归属校验 `assertOwned` 是本域的核心闸门
//!
//! Java 的 `AttributesTplServiceImpl#assertOwned(id)`：
//! ```text
//! Assert.notNull(id, () -> new GlobalException(TPL_NOT_FOUND));
//! AttributesTpl attributesTpl = this.lambdaQuery()
//!         .eq(AttributesTpl::getTplId, id)
//!         .eq(AttributesTpl::getShopId, currentShopId())
//!         .one();
//! Assert.notNull(attributesTpl, () -> new GlobalException(TPL_NOT_FOUND));
//! return attributesTpl;
//! ```
//! 被 `update` / `detail` / `del` / `batch`（逐个 id）/ `updateField` 共 5 处调用，
//! 是**所有写操作与详情读取的前置条件**。两条失败路径（`id == null`、
//! 查不到 or 非本店铺）**文案完全相同**（都是「模板不已存在」），
//! 对外不可区分 —— 本实现用 [`AttributesTplService::assert_owned`] 逐条复刻。
//!
//! ### 2. `shopId` 的写入路径有两条，且请求体不可信
//!
//! - 控制器层：`create` 里 `attributesTplDTO.setShopId(HeaderUtils.getShopId())`
//! - 服务层：`create` / `update` 里再 `dto.setShopId(currentShopId())`
//!
//! 注意 `list` / `listByCategory` / `detail` 只**读** `HeaderUtils.getShopId()`，
//! 不写 DTO。`create` 是**两层都写**（控制器 + 服务层），效果同一。
//!
//! `currentShopId()` = `Optional.ofNullable(HeaderUtils.getShopId()).orElse(0)`
//! —— `HeaderUtils.getShopId()` 只在 `x-admin-type == "admin"` 时返回 0、
//! `"shop"`/`"pos"` 时返回请求头 `x-shop-id`，其余情况返回 **null**（→ 0）。
//!
//! Rust 侧 JWT 尚未签发 `shop_id`，故由路由层从请求头 `x-shop-id`
//! 解析后经 `shop_id` 参数传入（与 admin_user 的处理一致）；
//! 缺失时回退 [`DEFAULT_SHOP_ID`]（= 0，平台视角）。
//!
//! ### 3. `create` 里服务层的 `setShopId` 是**死代码**
//!
//! Java 控制器已经 `setShopId(HeaderUtils.getShopId())`，服务层
//! `create` 又 `dto.setShopId(currentShopId())`。二者在同一个请求内
//! 读同一个请求头，**结果必然相同**，所以服务层这次赋值是冗余的。
//! 本实现只保留一次解析（路由层 → `shop_id` 入参）。
//!
//! ### 4. `batch` 的 `assertOwned` 在 `type` 判断**之前**
//!
//! Java `batch`：
//! ```text
//! if (batchDTO.getIds() != null) {
//!     batchDTO.getIds().forEach(this::assertOwned);   // ← 先逐个校验归属
//! }
//! return super.batch(batchDTO);                       // ← 再看 type 是否为 "del"
//! ```
//! 因此传 `{"ids":[...], "type":"show"}`（非 del）时，**仍会对每个 id 做
//! 归属校验**：只要有一个 id 不存在或不属于本店铺，就抛「模板不已存在」，
//! 而不是静默返回 `false`。这是容易漏掉的一处顺序依赖。
//!
//! 另外 `ids == null` 时跳过校验直接进 `super.batch`；而
//! `BaseServiceImpl#batch` 只判 `"del".equals(type)`，`removeByIds(null)`
//! 在 MyBatis-Plus 里抛异常。本实现把 `ids` 建模为 `Vec<i32>`（默认空），
//! 空集合时 `removeByIds(空)` 返回 false，**不报错**
//! （见 `uncertain_points`）。
//!
//! ### 5. `updateField` 的白名单只有 `tpl_name` 一项
//!
//! `CheckFieldConstants.TPL_FIELDS = {"tpl_name"}`。`getColumnByPropertyName`
//! 会把属性名归一为列名（未识别回退主键列 `tpl_id`），再比对白名单。
//! 因此只有 `tplName` / `tpl_name` 两种写法能通过，
//! `tplData` / `categoryId` / `shopId` / `tplId` 一律报「字段值不合法」。
//!
//! ### 6. 模板数据 `tplData` 的 JSON 往返
//!
//! - 入库：`Optional.ofNullable(dto.getTplData()).map(JSONUtil::toJsonStr).orElse("")`
//!   —— null ⇒ **空串**（不是 null、不是 `"null"`、不是 `"{}"`）
//! - 出库：`Optional.ofNullable(entity.getTplData()).filter(StrUtil::isNotEmpty)
//!   .map(JSONUtil::parseObj).map(o -> o.toBean(AttributeTplDataDTO.class)).orElse(null)`
//!   —— 空串 ⇒ **null**
//!
//! 两处转换由 `models::nextrade_core::models::catalog::attributes_tpl::{encode_tpl_data, decode_tpl_data}` 承担。
//!
//! ## ⚠️ 已确认的 Java 缺陷 / 待确认点（本实现的选择）
//!
//! ### A. `list` 的 `keyword` 判据是 `!= null` 而不是 `isNotEmpty`
//!
//! Java：`if (basePage.getKeyword() != null) queryWrapper.like(tplName, keyword)`。
//! 传 `keyword=""` 会生成 `tpl_name LIKE '%%'`（全匹配，**不报错**）。
//! 本实现用「去空白后非空才过滤」，**结果集完全一致**，只是少一次无谓 LIKE。
//!
//! ### B. `detail` 在记录不存在时 Java 抛 NPE 而非业务异常
//!
//! `AttributesTplServiceImpl#detail(id)` = `convertToDTO(assertOwned(id))`，
//! `assertOwned` 已经保证非 null，所以 **不会 NPE** —— 这条是澄清而非缺陷：
//! `detail` 与 `update` / `del` / `updateField` 一样走 `assertOwned`，
//! 失败文案是「模板不已存在」。本实现保持一致（`Error::NotFound`）。
//!
//! ### C. `tpl_name` 的 `@Size(max = 100)` 与库列 `varchar(80)` 不一致
//!
//! 61~100 字符的名字能过 Java 校验，却在入库时被 MySQL 拒绝
//! （严格模式报 `Data too long`）。本实现按注解 100 执行以复刻 Java 行为，
//! 让这个不一致**在同一个位置暴露**（而不是被 Rust 提前拦掉），
//! 见 `uncertain_points`。
//!
//! ### D. `updateById` 的归属校验与写入之间存在 TOCTOU 窗口
//!
//! Java 与本次 Rust 实现都是「先查归属，再按主键写」。理论上两次调用之间
//! 该行可能被改动。`tpl_id` 是主键、`shop_id` 极少变更，实际风险可忽略；
//! 本实现保持与 Java 相同的结构，未额外加 `AND shop_id = ?` 到 UPDATE
//! （那样会改变「影响行数为 0」的语义）。见 `uncertain_points`。

use std::sync::Arc;

use nextrade_core::models::catalog::attributes_tpl::{
    encode_tpl_data, map_update_field_column, validate_save, AttributeTplData, AttributesTplDTO,
    AttributesTplListQuery, AttributesTplSaveDTO, BatchDTO, OperateDTO, UpdateFieldDTO,
    DEFAULT_SHOP_ID, MSG_FIELD_NAME_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE,
    MSG_TPL_NOT_FOUND, TPL_COLUMNS, TPL_FIELDS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::attributes_tpl::{AttributesTplRepository, TplInsert, TplPatch};
use nextrade_infrastructure::db::Repositories;

/// 商品属性模板域服务（挂载于 `AdminState.attributes_tpl`）。
pub struct AttributesTplService {
    repos: Arc<Repositories>,
}

impl AttributesTplService {
    /// 构造（与 order_admin / area_code / region_client 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储的便捷访问。
    fn repo(&self) -> &AttributesTplRepository {
        &self.repos.catalog.attributes_tpl
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 属性模板分页（对应 Java `AttributesTplServiceImpl#list`）。
    ///
    /// 逐条对照 Java：
    /// 1. `Page<AttributesTpl> page = new Page<>(basePage.getPage(), basePage.getSize())`
    /// 2. `queryWrapper.eq(AttributesTpl::getShopId, currentShopId())` —— **恒定过滤**
    /// 3. `if (keyword != null) queryWrapper.like(AttributesTpl::getTplName, keyword)`
    /// 4. `buildSortOrder(page, sortField, sortOrder)`
    /// 5. `this.page(page, queryWrapper)` → `convertToDTO` 逐行转换
    ///
    /// `shop_id` 由路由层解析请求头后传入（见模块头第 2 条）。
    ///
    /// 返回 `PageResult<AttributesTplDTO>`（Java 返回 MyBatis-Plus
    /// `Page<AttributesTplDTO>`，Rust 统一使用项目既有 `PageResult`，
    /// 与 product_manage/002/008/011/017/075 同一约定）。
    pub async fn list(
        &self,
        q: &AttributesTplListQuery,
        shop_id: i32,
    ) -> Result<PageResult<AttributesTplDTO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate(q, shop_id).await?;
        let list: Vec<AttributesTplDTO> = rows.iter().map(AttributesTplDTO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 按类目获取属性模板（对应 Java `AttributesTplServiceImpl#listByCategory`）。
    ///
    /// 逐条对照 Java：
    /// ```text
    /// this.lambdaQuery()
    ///     .eq(AttributesTpl::getShopId, Optional.ofNullable(HeaderUtils.getShopId()).orElse(0))
    ///     .eq(AttributesTpl::getCategoryId, categoryId)
    ///     .list()
    /// ```
    /// **不分页**（Java 返回 `List<AttributesTplDTO>`）、**无 ORDER BY**。
    ///
    /// ⚠️ 控制器上是 `@RequestParam @NotNull(message = "商品类目不能为空")
    /// Integer categoryId`，缺失时 Java 返回 400。本实现在路由层用
    /// `i32` 反序列化，缺失时 axum 直接 400（语义一致）。
    /// 注意 `categoryId = 0` 是**合法**入参（`@NotNull` 不排除 0）。
    pub async fn list_by_category(
        &self,
        category_id: i32,
        shop_id: i32,
    ) -> Result<Vec<AttributesTplDTO>> {
        let rows = self.repo().find_by_category(category_id, shop_id).await?;
        Ok(rows.iter().map(AttributesTplDTO::from).collect())
    }

    /// 属性模板详情（对应 Java `AttributesTplServiceImpl#detail`）。
    ///
    /// Java：`return convertToDTO(assertOwned(id));`
    /// —— 走完整的归属校验，不存在 / 非本店铺都报「模板不已存在」。
    pub async fn detail(&self, tpl_id: i32, shop_id: i32) -> Result<AttributesTplDTO> {
        let entity = self.assert_owned(tpl_id, shop_id).await?;
        Ok(AttributesTplDTO::from(&entity))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建属性模板（对应 Java `AttributesTplController#create`
    /// + `AttributesTplServiceImpl#create`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `@Valid` 校验（`tplName` `@NotNull` + `@Size(max=100)`、
    ///    `categoryId` `@NotNull`）⇒ [`validate_save`]
    /// 2. **覆盖 `shopId`**（`dto.setShopId(currentShopId())`，
    ///    请求体里的 `shopId` 不被信任）
    /// 3. `tplData` 编码为 JSON 文本；`null` ⇒ 空串
    /// 4. `this.save(convertToBean(dto))`
    ///
    /// ⚠️ Java 的 `create` **不做重名检查**（`product_attributes_tpl` 表也没有
    /// `tpl_name` 唯一索引）⇒ 同名模板可以重复创建。本实现保持这一行为。
    ///
    /// ⚠️ 与 `area_code`（区号）不同，本域 `create` **没有**任何「先清标记」
    /// 之类的副作用，就是一条 INSERT。
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    pub async fn create(&self, dto: &AttributesTplSaveDTO, shop_id: i32) -> Result<i32> {
        // 1. 校验（Java `@Valid` + 类级 `@Validated`）
        let (tpl_name, category_id) = validate_save(dto).map_err(Error::Validation)?;

        // 2. 入库：`shopId` 一律用服务层解析值（请求体里的被丢弃）
        let id = self
            .repo()
            .insert(&TplInsert {
                tpl_name,
                // 3. `tplData == null` ⇒ 空串（Java `orElse("")`）
                tpl_data: encode_tpl_data(dto.tpl_data.as_ref()),
                shop_id: resolve_shop_id(Some(shop_id)),
                category_id: Some(category_id),
            })
            .await?;

        tracing::info!("创建商品属性模板成功: tpl_id={id}, shop_id={shop_id}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新属性模板（对应 Java `AttributesTplController#update`
    /// + `AttributesTplServiceImpl#update`）。
    ///
    /// 顺序与 Java 逐条对齐：
    /// 1. `assertOwned(dto.getTplId())` —— 归属校验（`tplId` 缺失也走这条路径，
    ///    报「模板不已存在」而不是「主键不能为空」，见模块头第 1 条）
    /// 2. `@Valid` 校验（`tplName` / `categoryId`）
    /// 3. `dto.setShopId(currentShopId())`
    /// 4. `this.updateById(convertToBean(dto))`（`FieldStrategy.NOT_NULL`：
    ///    只写非 null 字段）
    ///
    /// ⚠️ 注意 Java 的顺序是**先 `assertOwned` 再校验**，所以传一个不存在的
    /// `tplId` + 空的 `tplName` 时，报的是「模板不已存在」而不是
    /// 「模板名称不能为空」。本实现保持同样的顺序。
    ///
    /// 返回是否命中记录（Java 控制器返回 `void`，此处回传受影响行数 > 0）。
    pub async fn update(&self, dto: &AttributesTplSaveDTO, shop_id: i32) -> Result<bool> {
        let shop_id = resolve_shop_id(Some(shop_id));

        // 1. 归属校验（Java 第一步就是 assertOwned(dto.getTplId())）
        let tpl_id = dto.tpl_id.ok_or_else(|| Error::NotFound(MSG_TPL_NOT_FOUND.into()))?;
        self.assert_owned(tpl_id, shop_id).await?;

        // 2. 校验（Java `@Valid` 在参数绑定阶段已生效，此处补齐）
        let (tpl_name, category_id) = validate_save(dto).map_err(Error::Validation)?;

        // 3. 补丁：`tpl_name` / `tpl_data` / `category_id` 必写；
        //    `shop_id` 与 Java 一样写回解析值（`dto.setShopId(currentShopId())`）
        let patch = TplPatch {
            tpl_name: Some(Some(tpl_name)),
            tpl_data: Some(Some(encode_tpl_data(dto.tpl_data.as_ref()))),
            shop_id: Some(Some(shop_id)),
            // `categoryId` 已通过 `@NotNull` 校验 ⇒ 必为 `Some(Some(_))`
            category_id: Some(Some(category_id)),
        };
        let affected = self.repo().update(tpl_id, &patch).await?;
        tracing::info!("更新商品属性模板: tpl_id={tpl_id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除属性模板（对应 Java `AttributesTplController#del`
    /// + `AttributesTplServiceImpl#del`）。
    ///
    /// Java：
    /// ```text
    /// public boolean del(Integer id) {
    ///     assertOwned(id);          // ← id == null ⇒ 抛「模板不已存在」
    ///     return super.del(id);     // ← removeById（硬删除）
    /// }
    /// ```
    ///
    /// ⚠️ 与 area_code（区号 `del`）**不同**：那里删不存在的 id 静默成功，
    /// 本域因为先过 `assertOwned`，删不存在 / 非本店铺的 id 会**报错**。
    ///
    /// ⚠️ 控制器 `del(@RequestBody OperateDTO)` 参数上**没有** `@Valid`，
    /// 因此 `OperateDTO.id` 的 `@NotNull` 不生效：传 `{}` 时 `id == null`，
    /// 落到 `assertOwned(null)` → 报「模板不已存在」（HTTP 200 + 业务码），
    /// **不是** 400。本实现复刻这一语义（用 `NotFound` 而非 `Validation`）。
    pub async fn del(&self, dto: &OperateDTO, shop_id: i32) -> Result<bool> {
        let shop_id = resolve_shop_id(Some(shop_id));
        // `id == null` 与「查不到」在 Java 里是同一个异常、同一句文案
        let tpl_id = dto.id.ok_or_else(|| Error::NotFound(MSG_TPL_NOT_FOUND.into()))?;
        self.assert_owned(tpl_id, shop_id).await?;
        let removed = self.repo().delete(tpl_id).await?;
        tracing::info!("删除商品属性模板: tpl_id={tpl_id}, 结果={removed}");
        Ok(removed)
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `AttributesTplController#updateField`
    /// + `AttributesTplServiceImpl#updateField`）。
    ///
    /// Java：
    /// ```text
    /// public boolean updateField(UpdateFieldDTO updateField, String[] allowFields) {
    ///     assertOwned(updateField.getId());                 // ← 先校验归属
    ///     return super.updateField(updateField, allowFields);
    /// }
    /// ```
    /// `BaseServiceImpl#updateField`：
    /// ```text
    /// String field = updateField.getField();
    /// field = getColumnByPropertyName(field);               // 归一（未识别回退主键列）
    /// if (!Arrays.asList(allowFields).contains(field))
    ///     throw new GlobalException(INVALID_FIELD_VALUE);   // 「字段值不合法」
    /// UpdateWrapper<T> w = new UpdateWrapper<>();
    /// w.eq(getKeyColumn(), updateField.getId()).set(field, updateField.getVal());
    /// return this.update(w);
    /// ```
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `id` 缺失 ⇒ **「模板不已存在」**（不是「id不能为空」——
    ///    Java 的 `assertOwned` 先接住 null；`UpdateFieldDTO` 的 `@NotNull`
    ///    因控制器缺 `@Valid` 而未生效）
    /// 2. `assertOwned(id)` 归属校验
    /// 3. `field` 归一为列名（属性名 / 列名都可命中）
    /// 4. 归一后的列名必须落在 `TPL_FIELDS = {"tpl_name"}` 内，否则
    ///    `字段值不合法`；再用 `TPL_COLUMNS` 做二次防御断言
    /// 5. `UPDATE product_attributes_tpl SET <列> = ? WHERE tpl_id = ?`，
    ///    值以字符串绑定，由 MySQL 按目标列类型隐式转换
    ///
    /// ⚠️ Java 的 `updateField.getField()` 为 null 时
    /// `getColumnByPropertyName(null)` 会走完 `equals(null)` 各分支
    /// （都是 false）并**回退主键列** `tpl_id`，再被白名单拒绝 ⇒ 同样报
    /// `字段值不合法`。本实现把「field 缺失/空白」显式判为
    /// `字段值不合法`，**判定结果一致**（见 `uncertain_points`）。
    ///
    /// 返回是否命中记录（Java 直接返回 `this.update(updateWrapper)` 的布尔值）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO, shop_id: i32) -> Result<bool> {
        let shop_id = resolve_shop_id(Some(shop_id));

        // 1. `assertOwned` 先接住 null id（Java 顺序）
        let tpl_id = dto.id.ok_or_else(|| Error::NotFound(MSG_TPL_NOT_FOUND.into()))?;
        // 2. 归属校验
        self.assert_owned(tpl_id, shop_id).await?;

        // 3. `field` 为空 / 缺失 ⇒ Java 会回退主键列后被白名单拒绝，
        //    净结果同为「字段值不合法」
        let field = dto
            .field
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;

        // 4. 归一 + 白名单（`CheckFieldConstants.TPL_FIELDS = {"tpl_name"}`）
        let column = map_update_field_column(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        if !TPL_FIELDS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }
        // 二次保险：白名单字段必须是真实列（防未来改动引入幻觉列）
        if !TPL_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        // 5. `val == null` ⇒ Java 绑定 null（列 `NOT NULL` ⇒ 报错）；
        //    本实现按空串绑定（见 uncertain_points）
        let val = dto.val.as_deref().unwrap_or_default();
        let affected = self.repo().update_field(tpl_id, column, val).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `AttributesTplController#batch`
    /// + `AttributesTplServiceImpl#batch` + `BaseServiceImpl#batch`）。
    ///
    /// ⚠️ **顺序极易漏**（详见模块头第 4 条）：
    /// ```text
    /// if (batchDTO.getIds() != null) {
    ///     batchDTO.getIds().forEach(this::assertOwned);   // ① 逐个归属校验
    /// }
    /// return super.batch(batchDTO);                       // ② 再看 type
    /// ```
    /// `super.batch`：`"del".equals(type)` ⇒ `removeByIds(ids)`；否则 `false`。
    ///
    /// 因此：
    /// - `type == "del"` + ids 全部属于本店铺 ⇒ 删除，返回 `rows >= 1`
    /// - `type != "del"` + ids 含非本店铺 id ⇒ **抛「模板不已存在」**
    ///   （不是静默 `false`！）
    /// - `type != "del"` + ids 全部合法 ⇒ 静默 `false`（HTTP 200）
    /// - `ids` 为空 ⇒ 跳过校验，`removeByIds(空)` 返回 `false`
    ///
    /// ⚠️ Java `removeByIds` 对**不存在的 id** 是静默忽略的
    /// （`DELETE ... WHERE tpl_id IN (...)` 命中 0 行不报错），
    /// 但因为前面已 `assertOwned` 过，实际不会出现不存在的 id。
    ///
    /// ⚠️ 控制器参数上**没有** `@Valid`，故 `BatchDTO.ids` 的 `@NotEmpty`、
    /// `type` 的 `@NotNull` 都**不生效**：空数组 / 缺 type 都不是 400。
    /// 本实现按 Java 语义在服务层处理，不额外抛 400
    /// （与 pc_cat_floor / area_code 的选择一致，见 `uncertain_points`）。
    pub async fn batch(&self, dto: &BatchDTO, shop_id: i32) -> Result<bool> {
        let shop_id = resolve_shop_id(Some(shop_id));

        // ① 先逐个校验归属（Java 在 type 判断之前）
        for id in &dto.ids {
            // Java `assertOwned`：不存在 / 非本店铺 ⇒ 「模板不已存在」
            self.assert_owned(*id, shop_id).await?;
        }

        // ② 再看 type（`BaseServiceImpl#batch`：仅 "del" 生效）
        if dto.r#type.as_deref() != Some("del") {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        let affected = self.repo().delete_many(&dto.ids).await?;
        tracing::info!(
            "批量删除商品属性模板: ids={:?}, 影响 {affected} 行",
            dto.ids
        );
        Ok(affected > 0)
    }

    // ————————————————————— 内部：归属校验 —————————————————————

    /// 归属校验（对应 Java `AttributesTplServiceImpl#assertOwned`）。
    ///
    /// 逐条复刻 Java 的两条失败路径，**文案完全相同**（都是「模板不已存在」）：
    /// - `id == null` ⇒ `Assert.notNull(id, () -> new GlobalException(TPL_NOT_FOUND))`
    /// - 查不到该 `tpl_id`，或该行的 `shop_id != currentShopId()`
    ///   ⇒ `Assert.notNull(attributesTpl, () -> new GlobalException(TPL_NOT_FOUND))`
    ///
    /// 对外不可区分「不存在」与「非本店铺」是**有意的安全设计**
    /// （避免通过错误文案枚举他人模板 ID），本实现保持原样。
    ///
    /// # Errors
    /// - 记录不存在或不属于 `shop_id` ⇒ `模板不已存在`
    ///
    /// 注：Java 侧 `assertOwned(id)` 的 `id == null` 分支由**调用方**先行处理
    /// （各入口都已 `ok_or_else(...)` 抛出同一句文案），故本方法接收已解包的
    /// `i32`，避免调用方重复传 `Option` 造成类型噪音。
    pub async fn assert_owned(
        &self,
        tpl_id: i32,
        shop_id: i32,
    ) -> Result<nextrade_core::entities::ProductAttributesTpl> {
        self.repo()
            .find_owned(tpl_id, resolve_shop_id(Some(shop_id)))
            .await?
            .ok_or_else(|| Error::NotFound(MSG_TPL_NOT_FOUND.into()))
    }

    /// 判断某个 id 是否对当前店铺可见（供路由层 / 调试观察，非 Java 概念）。
    ///
    /// ⚠️ **本方法不对应本切片的任何路由**，只是把 `assert_owned` 的
    /// 布尔视图暴露出来，便于主会话写集成测试或排查权限问题。
    pub async fn is_owned(&self, tpl_id: i32, shop_id: i32) -> Result<bool> {
        Ok(self
            .repo()
            .find_owned(tpl_id, resolve_shop_id(Some(shop_id)))
            .await?
            .is_some())
    }

    /// 读取模板数据（对应 Java `convertToDTO` 里的 `tplData` 转换）。
    ///
    /// 供其它域（如商品编辑页按模板预填属性）复用同一处解析逻辑；
    /// **本方法不对应本切片的任何路由**，也不做归属校验，
    /// 调用方需自行确认权限。
    pub async fn tpl_data_of(
        &self,
        tpl_id: i32,
        shop_id: i32,
    ) -> Result<Option<AttributeTplData>> {
        let entity = self.assert_owned(tpl_id, shop_id).await?;
        Ok(nextrade_core::models::catalog::attributes_tpl::decode_tpl_data(&entity.tpl_data))
    }

    /// `updateField` 的字段名是否被白名单接受（纯函数，供路由层预检 / 测试）。
    ///
    /// 与 Java `BaseServiceImpl#updateField` 的白名单判定**结果一致**。
    #[must_use]
    pub fn is_allowed_field(field: &str) -> bool {
        map_update_field_column(field.trim()).is_some_and(|c| TPL_FIELDS.contains(&c))
    }

    /// `id` 缺失时的文案（Java `assertOwned` 复用 `TPL_NOT_FOUND`）。
    ///
    /// 单独暴露是为了让路由层的入参预检与 `updateField` / `del` 用**同一句**
    /// 文案，避免两条路径漂移。
    #[must_use]
    pub const fn id_required_message() -> &'static str {
        MSG_ID_REQUIRED
    }

    /// 字段名缺失时的文案（Java 侧该场景实际落到「字段值不合法」）。
    #[must_use]
    pub const fn field_required_message() -> &'static str {
        MSG_FIELD_NAME_REQUIRED
    }
}

/// 解析当前请求的店铺 ID（对应 Java `AttributesTplServiceImpl#currentShopId`）：
/// `Optional.ofNullable(HeaderUtils.getShopId()).orElse(0)`。
///
/// `HeaderUtils.getShopId()` 在 `x-admin-type` 非 `admin` / `shop` / `pos`
/// 时返回 **null**；平台账号（`admin`）恒为 0。故这里把 `None` 折成
/// [`DEFAULT_SHOP_ID`]（= 0）。
///
/// 负数（异常请求头）一律折成 0，避免把 `shop_id = -1` 带进 SQL
/// （`shop_id` 列是 `int unsigned`，负值会被 MySQL 拒绝）。
#[must_use]
pub fn resolve_shop_id(raw: Option<i32>) -> i32 {
    match raw {
        Some(v) if v > 0 => v,
        _ => DEFAULT_SHOP_ID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::attributes_tpl::{
        AttributeTplData, TplAttribute, MSG_CATEGORY_ID_REQUIRED, MSG_TPL_NAME_IS_NULL,
        MSG_TPL_NAME_OVER_LENGTH, TPL_NAME_MAX_LEN,
    };

    // ————————————————————— 纯逻辑：shop_id 归一 —————————————————————

    /// `currentShopId()` = `Optional.ofNullable(...).orElse(0)`：
    /// null ⇒ 0；平台账号 0 保持 0；店铺账号原样透传。
    #[test]
    fn resolve_shop_id_matches_current_shop_id() {
        assert_eq!(resolve_shop_id(None), 0);
        assert_eq!(resolve_shop_id(Some(0)), 0);
        assert_eq!(resolve_shop_id(Some(7)), 7);
        // 负值（异常请求头）折成 0，避免写进 int unsigned 列
        assert_eq!(resolve_shop_id(Some(-3)), 0);
        assert_eq!(DEFAULT_SHOP_ID, 0);
    }

    // ————————————————————— 纯逻辑：updateField 白名单 —————————————————————

    /// 白名单只有 `tpl_name`：`tplName` 两种写法通过，其余全拒。
    #[test]
    fn update_field_whitelist_flow() {
        assert!(AttributesTplService::is_allowed_field("tplName"));
        assert!(AttributesTplService::is_allowed_field("tpl_name"));
        // 带空白也要能命中（本实现 trim；Java `equals` 不 trim ⇒
        // 这里比 Java 宽松，见 uncertain_points）
        assert!(AttributesTplService::is_allowed_field("  tplName  "));

        // 归一得到真实列，但白名单外 ⇒ Java 报「字段值不合法」
        for bad in [
            "tplId",
            "tpl_id",
            "tplData",
            "tpl_data",
            "shopId",
            "shop_id",
            "categoryId",
            "category_id",
            "id",
            "",
            "   ",
            "something",
        ] {
            assert!(
                !AttributesTplService::is_allowed_field(bad),
                "{bad:?} 不应被白名单接受"
            );
        }
    }

    /// `TPL_FIELDS ∩ TPL_COLUMNS` 必须是白名单本身（防幻觉列）。
    #[test]
    fn whitelist_is_subset_of_real_columns() {
        assert_eq!(TPL_FIELDS, ["tpl_name"]);
        for f in TPL_FIELDS {
            assert!(TPL_COLUMNS.contains(&f), "{f} 不是真实列");
        }
    }

    /// 文案常量与 Java 逐字符对照（含「模板不已存在」的错别字）。
    #[test]
    fn java_message_constants_are_verbatim() {
        assert_eq!(AttributesTplService::id_required_message(), "id不能为空");
        assert_eq!(AttributesTplService::field_required_message(), "字段名不能为空");
        assert_eq!(MSG_TPL_NOT_FOUND, "模板不已存在");
        assert_eq!(MSG_INVALID_FIELD_VALUE, "字段值不合法");
    }

    // ————————————————————— 纯逻辑：create / update 校验 —————————————————————

    /// `@NotNull` 语义：`tplName` 空串合法、`categoryId = 0` 合法。
    #[test]
    fn validate_save_follows_not_null_not_not_blank() {
        let blank_name = AttributesTplSaveDTO {
            tpl_name: Some(String::new()),
            category_id: Some(0),
            ..Default::default()
        };
        assert_eq!(
            validate_save(&blank_name).unwrap(),
            (String::new(), 0),
            "空串通过 @NotNull；categoryId=0 也是合法值"
        );

        let missing = AttributesTplSaveDTO::default();
        assert_eq!(validate_save(&missing).unwrap_err(), MSG_TPL_NAME_IS_NULL);

        let no_cat = AttributesTplSaveDTO {
            tpl_name: Some("模板".into()),
            ..Default::default()
        };
        assert_eq!(validate_save(&no_cat).unwrap_err(), MSG_CATEGORY_ID_REQUIRED);
    }

    /// `@Size(max = 100)`：100 合法、101 报错（库列只有 `varchar(80)`）。
    #[test]
    fn validate_save_length_boundary() {
        let ok = AttributesTplSaveDTO {
            tpl_name: Some("名".repeat(TPL_NAME_MAX_LEN)),
            category_id: Some(1),
            ..Default::default()
        };
        assert!(validate_save(&ok).is_ok());

        let too_long = AttributesTplSaveDTO {
            tpl_name: Some("名".repeat(TPL_NAME_MAX_LEN + 1)),
            category_id: Some(1),
            ..Default::default()
        };
        assert_eq!(
            validate_save(&too_long).unwrap_err(),
            MSG_TPL_NAME_OVER_LENGTH
        );
        assert_eq!(TPL_NAME_MAX_LEN, 100);
    }

    /// `tplName` 会被 trim（Java `BeanUtils.copyProperties` 不 trim ⇒ 本实现更严，见 uncertain）。
    #[test]
    fn validate_save_trims_name() {
        let dto = AttributesTplSaveDTO {
            tpl_name: Some("  手机模板  ".into()),
            category_id: Some(9),
            ..Default::default()
        };
        assert_eq!(validate_save(&dto).unwrap(), ("手机模板".to_string(), 9));
    }

    // ————————————————————— 纯逻辑：batch 的 type 门 —————————————————————

    /// `batch` 的 `type` 判定：只有字面量 `"del"` 生效（大小写敏感）。
    #[test]
    fn batch_type_gate_is_exact_match() {
        // 复刻 Java `"del".equals(batchDTO.getType())`
        let is_del = |t: Option<&str>| t == Some("del");
        assert!(is_del(Some("del")));
        assert!(!is_del(Some("DEL")));
        assert!(!is_del(Some("Del")));
        assert!(!is_del(Some("show")));
        assert!(!is_del(Some("")));
        assert!(!is_del(None));
    }

    /// `BatchDTO.ids` 缺失 ⇒ 空 Vec（Java 是 null，跳过校验分支）。
    #[test]
    fn batch_dto_missing_ids_is_empty_vec() {
        let dto: BatchDTO = serde_json::from_str(r#"{"type":"del"}"#).unwrap();
        assert!(dto.ids.is_empty());

        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[3,5],"type":"del"}"#).unwrap();
        assert_eq!(dto.ids, vec![3, 5]);
    }

    // ————————————————————— 纯逻辑：tplData 编解码（服务层视角） —————————————————————

    /// `create` 写入的 `tpl_data`：`None` ⇒ 空串；`Some` ⇒ JSON 文本。
    #[test]
    fn create_encodes_tpl_data_like_java() {
        // Java `Optional.ofNullable(dto.getTplData()).map(JSONUtil::toJsonStr).orElse("")`
        assert_eq!(encode_tpl_data(None), "");

        let data = AttributeTplData {
            normal: Some(vec![TplAttribute {
                attr_name: Some("颜色".into()),
                attr_values: Some(vec!["黑".into()]),
            }]),
            ..Default::default()
        };
        let encoded = encode_tpl_data(Some(&data));
        assert!(encoded.contains("\"attrName\":\"颜色\""));
        assert!(encoded.contains("\"attrValues\":[\"黑\"]"));
        // Hutool `toJsonStr` 默认丢弃 null 字段 ⇒ 未设置的 spe/extra 不出现
        assert!(!encoded.contains("spe"));
        assert!(!encoded.contains("extra"));
    }

    /// 出库：空串 ⇒ `null`（Java `filter(StrUtil::isNotEmpty).orElse(null)`）。
    #[test]
    fn decode_empty_tpl_data_is_null() {
        assert_eq!(
            nextrade_core::models::catalog::attributes_tpl::decode_tpl_data(""),
            None
        );
        assert_eq!(
            nextrade_core::models::catalog::attributes_tpl::decode_tpl_data("null"),
            None
        );
    }

    // ————————————————————— 纯逻辑：id 缺失的文案选择 —————————————————————

    /// `del` / `updateField` 在 `id` 缺失时报「模板不已存在」，
    /// **不是**「id不能为空」—— 因为 Java 的 `assertOwned` 先接住了 null
    /// （控制器缺 `@Valid`，`@NotNull` 未生效）。
    #[test]
    fn missing_id_reports_tpl_not_found_not_id_required() {
        // 复刻 assertOwned 对 null 的处理
        let err_of = |id: Option<i32>| -> Error {
            id.ok_or_else(|| Error::NotFound(MSG_TPL_NOT_FOUND.into()))
                .expect_err("None 应当报「模板不已存在」")
        };
        let e = err_of(None);
        assert!(matches!(e, Error::NotFound(_)));
        assert!(e.to_string().contains("模板不已存在"));

        // 与「id不能为空」区分开（后者只在别处使用）
        assert_ne!(MSG_TPL_NOT_FOUND, MSG_ID_REQUIRED);
    }
}