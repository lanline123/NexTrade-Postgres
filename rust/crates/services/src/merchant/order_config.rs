//! 订单配置域服务（merchant::order_config）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.shop.OrderConfigController`
//! + `com.nextrade.service.shop.impl.OrderConfigServiceImpl`
//! （146 行）+ `com.nextrade.service.common.impl.BaseServiceImpl`
//! （通用 `updateField` / `batch` / `del` / `buildSortOrder`）
//! 中的 7 条管理端链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /shop/orderConfig/list` | `OrderConfigController.list` | [`OrderConfigService::list`] |
//! | `GET  /shop/orderConfig/detail` | `OrderConfigController.detail` | [`OrderConfigService::detail`] |
//! | `POST /shop/orderConfig/create` | `OrderConfigController.create` | [`OrderConfigService::create`] |
//! | `POST /shop/orderConfig/update` | `OrderConfigController.update` | [`OrderConfigService::update`] |
//! | `POST /shop/orderConfig/del` | `OrderConfigController.del` | [`OrderConfigService::del`] |
//! | `POST /shop/orderConfig/updateField` | `OrderConfigController.updateField` | [`OrderConfigService::update_field`] |
//! | `POST /shop/orderConfig/batch` | `OrderConfigController.batch` | [`OrderConfigService::batch`] |
//!
//! ## ⚠️ 本域的核心结论：这是一个**脚手架复制粘贴后没有改字段**的控制器
//!
//! 表 `order_config` 只有 4 列（`python tools/schema_parser.py --table order_config`）：
//! `id` / `shop_id` / `code` / `data`。
//!
//! 而 DTO / VO 声明的却是另一张表的字段：
//! - `OrderConfigVO`：`orderConfigId` / `orderConfigName` / `orderConfigPic`
//!   / `sortOrder` / `isShow` / `status` / `orderConfigDesc`
//! - `OrderConfigCreateDTO`：`orderConfigName` / `orderConfigPic` / `sortOrder`
//!   / `isShow` / `status` / `orderConfigDesc`
//!
//! 两边**没有任何一个字段名能对上** `order_config` 的列。于是：
//!
//! 1. **`list` / `detail`**：`BeanUtils.copyProperties(orderConfig, vo)`
//!    按属性名拷贝，4 个属性与 VO 的 7 个属性**无一重名** ⇒ 返回的是
//!    **7 个字段全为 `null`** 的空壳 VO。
//! 2. **`create`**：先按 `@NotBlank` / `@Size(max=50)` 校验 `orderConfigName`
//!    （这一步**会真实报错**），通过后 `copyProperties` 拷不到任何列 ⇒
//!    实体 `shopId` / `code` / `data` 全为 `null` ⇒ `save` 因
//!    `shop_id` / `code` 两列 `NOT NULL` 而抛
//!    `DataIntegrityViolationException`（SQL 异常）。
//! 3. **`update`**：唯一校验是 `orderConfigId` 非空；通过后实体除主键外全
//!    `null`，MyBatis-Plus 的 `FieldStrategy.NOT_NULL` 跳过所有 null 列 ⇒
//!    生成 `UPDATE order_config SET WHERE id = ?`（**无 SET 项**）⇒
//!    MySQL 语法错误。
//! 4. **`updateField`**：`CheckFieldConstants.ORDER_CONFIG_FIELDS` 是
//!    **空数组 `{}`**（`CheckFieldConstants.java:76`）⇒
//!    `Arrays.asList(allowFields).contains(field)` 恒为 false ⇒
//!    **任何**字段都抛 `字段值不合法`，该接口在 Java 里恒定失败。
//! 5. **`del` / `batch`**：这两条是唯一「正常」的 —— 走
//!    `BaseServiceImpl` 的通用 `removeById` / `removeByIds`，与表结构无关
//!    （`batch` 同样只认 `type == "del"`）。
//!
//! ## 本实现的取舍
//!
//! - 对 4、5 两条：**逐字符复刻 Java 的可观测行为**（白名单恒拒、`batch`
//!   非 del 静默 false、`del` 删不存在的 id 也返回成功）。
//! - 对 1：默认返回**按表真实列**的投影（[`OrderConfigVO`]），
//!   因为「7 个字段全 null」对任何调用方都没有信息量；
//!   同时把 Java 的原始形状保留在
//!   [`nextrade_core::models::merchant::order_config::OrderConfigJavaShapeVO`]，一行可切。
//! - 对 2、3：**不伪造一次必然违反数据库约束的 INSERT/UPDATE**。
//!   `create` 在通过 `@NotBlank` / `@Size` 校验后，要求调用方提供
//!   `shop_id` / `code`（表的两列 `NOT NULL`）—— 这两个字段是
//!   **本实现为让接口可用而新增的入参**（Java DTO 里没有），
//!   缺失时返回可诊断的 `Validation` 错误而不是 500 级 SQL 异常；
//!   `update` 在补丁为空时返回「无可更新字段」而不生成非法 SQL。
//!
//! 以上每一条都列入返回值 `uncertain_points`。

use std::sync::Arc;

use nextrade_core::models::merchant::order_config::{
    map_update_field_column, validate_create, BatchDTO, OperateDTO, OrderConfigCreateDTO,
    OrderConfigListQuery, OrderConfigUpdateDTO, OrderConfigVO, UpdateFieldDTO,
    MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_ORDER_CONFIG_NOT_FOUND, ORDER_CONFIG_COLUMNS,
    ORDER_CONFIG_FIELDS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::order_config::{OrderConfigInsert, OrderConfigPatch};
use nextrade_infrastructure::db::Repositories;

/// 订单配置域服务（挂载于 `AdminState.order_config`）。
pub struct OrderConfigService {
    repos: Arc<Repositories>,
}

impl OrderConfigService {
    /// 构造（与 order_admin / admin_user / pc_cat_floor / area_code 的
    /// `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 订单配置分页（对应 Java `OrderConfigServiceImpl#list`）。
    ///
    /// 逐条对照 Java：
    /// - `Page<>(listDTO.getPage(), listDTO.getSize())` —— 默认 1 / 15、上限 100
    /// - `buildSortOrder(page, sortField, sortOrder)`
    /// - **`isShow` 与 `keyword` 的过滤被整段注释掉了**（Java 第 61–70 行）
    ///   ⇒ 无条件分页，本实现同样不过滤
    /// - 结果 `BeanUtils.copyProperties` 成 VO
    ///
    /// ⚠️ 返回的 VO 是**按表真实列**的投影，不是 Java `OrderConfigVO` 的
    /// 7 字段形状（Java 那 7 个字段在表里都不存在，实际全为 `null`）；
    /// 详见模块头「核心结论」与 `uncertain_points`。
    pub async fn list(&self, q: &OrderConfigListQuery) -> Result<PageResult<OrderConfigVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.merchant.order_config.paginate(q).await?;
        let list: Vec<OrderConfigVO> = rows.iter().map(OrderConfigVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 订单配置详情（对应 Java `OrderConfigServiceImpl#detail`）。
    ///
    /// Java 行为：`id != null` 时 `getById(id)` → `copyProperties` → 返回 VO
    /// （记录不存在时 `copyProperties(null, vo)` 不抛异常，返回全 null VO）；
    /// `id == null` 时返回 `null`。
    ///
    /// 本实现由路由层要求 `id` 必填（与 Java `@RequestParam Integer id`
    /// 的「缺失即 400」一致），记录不存在时返回可诊断的 404
    /// （项目内 order_admin/005/017/075 的既有约定）。
    ///
    /// ⚠️ VO 形状的取舍同 [`OrderConfigService::list`]。
    pub async fn detail(&self, id: i32) -> Result<OrderConfigVO> {
        let c = self
            .repos.merchant.order_config
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::NotFound(MSG_ORDER_CONFIG_NOT_FOUND.into()))?;
        Ok(OrderConfigVO::from(&c))
    }

    // ————————————————————— 创建 —————————————————————

    /// 创建订单配置（对应 Java `OrderConfigServiceImpl#create`）。
    ///
    /// 逐条对照 Java：
    /// 1. `@Valid @RequestBody OrderConfigCreateDTO` —— 校验
    ///    `orderConfigName` 的 `@NotBlank` 与 `@Size(max = 50)`；
    /// 2. `BeanUtils.copyProperties(createDTO, orderConfig)`；
    /// 3. `this.save(orderConfig)`。
    ///
    /// 第 1 步的文案逐字符照抄 Java 常量。
    ///
    /// ⚠️ 第 2 步拷不出任何列（见模块头），因此第 3 步在 Java 里必然因
    /// `shop_id` / `code` 的 `NOT NULL` 约束抛 `DataIntegrityViolationException`。
    /// 本实现**不伪造那次必然失败的 INSERT**，改为要求调用方显式提供
    /// `shopId` / `code`（表的两列 `NOT NULL`；这两个入参是本实现新增的，
    /// Java DTO 里没有），缺失时返回可诊断的 `Validation` 错误。
    ///
    /// 返回新建主键（Java 控制器返回 `void`，此处便于前端跳转编辑，
    /// 与 shop_manage / pc_cat_floor / area_code 的处理方式一致）。
    ///
    /// # Errors
    /// - `orderConfigName` 缺失/空白 ⇒ `订单配置名称不能为空`
    /// - `orderConfigName` 超过 50 字符 ⇒ `订单配置名称长度不能超过50`
    /// - `shopId` 缺失 ⇒ 提示补充店铺 id
    /// - `code` 缺失/空白 ⇒ 提示补充编码
    pub async fn create(&self, dto: &OrderConfigCreateDTO) -> Result<i32> {
        // 1. 复刻 Java 的 @NotBlank + @Size(max = 50)
        validate_create(dto).map_err(Error::Validation)?;

        // 2. 表的两列 NOT NULL，必须由调用方补齐（Java 的 copyProperties 补不上）
        let shop_id = dto
            .shop_id
            .ok_or_else(|| Error::Validation("店铺id不能为空".into()))?;
        let code = dto
            .code
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("编码不能为空".into()))?;

        // 3. 入库（`data` 可空 ⇒ None 时走 MySQL 列默认值 NULL）
        let id = self
            .repos.merchant.order_config
            .insert(&OrderConfigInsert {
                shop_id: Some(shop_id),
                code: Some(code.to_string()),
                data: dto.data.clone(),
            })
            .await?;

        tracing::info!("创建订单配置成功: id={id}, shop_id={shop_id}, code={code}");
        Ok(id)
    }

    // ————————————————————— 更新 —————————————————————

    /// 更新订单配置（对应 Java `OrderConfigServiceImpl#update`）。
    ///
    /// 逐条对照 Java：
    /// 1. `@Valid @RequestBody OrderConfigUpdateDTO` —— 只有
    ///    `orderConfigId` 的 `@NotNull` 生效；
    /// 2. `BeanUtils.copyProperties(updateDTO, orderConfig)`；
    /// 3. `this.updateById(orderConfig)`。
    ///
    /// ⚠️ 第 2 步拷不出任何列 ⇒ 实体除主键外全 `null` ⇒ MyBatis-Plus 的
    /// `FieldStrategy.NOT_NULL` 跳过所有 null 列 ⇒ 生成
    /// `UPDATE order_config SET WHERE id = ?`（**无 SET 项**）⇒ MySQL 语法错误。
    /// 本实现不生成非法 SQL：补丁为空时返回可诊断的 `Validation` 错误。
    ///
    /// 返回是否命中记录（Java 控制器返回 `void`，此处回传受影响行数 > 0）。
    ///
    /// # Errors
    /// - `orderConfigId` 缺失/非正 ⇒ `订单配置id不能为空`
    /// - 无任何可更新字段 ⇒ 提示无可更新字段
    /// - 记录不存在 ⇒ 404
    pub async fn update(&self, dto: &OrderConfigUpdateDTO) -> Result<bool> {
        // 1. 主键（Java `OrderConfigUpdateDTO` 上唯一的 @NotNull）
        let id = dto.require_id().map_err(Error::Validation)?;

        // 2. 构造补丁（`FieldStrategy.NOT_NULL`：None 不写列，空串/0 照写）
        let patch = OrderConfigPatch {
            shop_id: dto.shop_id.map(Some),
            code: dto.code.clone().map(Some),
            data: dto.data.clone().map(Some),
        };

        // 3. 全 null 补丁在 Java 里会拼出非法 SQL，这里明确拦截
        if patch.is_empty() {
            return Err(Error::Validation(
                "无可更新字段：请求中未包含 shopId / code / data 中的任何一项".into(),
            ));
        }

        // 4. 记录必须存在（Java `updateById` 命中 0 行静默返回 false，
        //    这里先查一次以便区分「不存在」与「值未变化」）
        if self.repos.merchant.order_config.find_by_id(id).await?.is_none() {
            return Err(Error::NotFound(MSG_ORDER_CONFIG_NOT_FOUND.into()));
        }

        let affected = self.repos.merchant.order_config.update(id, &patch).await?;
        tracing::info!("更新订单配置: id={id}, 影响 {affected} 行");
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除订单配置（对应 Java `OrderConfigServiceImpl#del` →
    /// `BaseServiceImpl#del` → `removeById`，硬删除）。
    ///
    /// ⚠️ Java 的 `del` 是「`id == null` 返回 false，否则 `removeById(id)`」，
    /// **不校验记录是否存在**（删不存在的 id 也返回成功）。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404。
    ///
    /// 入参 `OperateDTO.id` 在 Java 上有 `@NotNull`，缺失时 Java 报 400；
    /// 本实现按项目惯例在服务层返回可诊断的 `Validation` 错误
    /// （与 pc_cat_floor / area_code 的选择一致）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        self.repos.merchant.order_config.delete(id).await
    }

    // ————————————————————— 单字段更新 —————————————————————

    /// 单字段更新（对应 Java `OrderConfigServiceImpl#updateField`，
    /// 控制器传入 `CheckFieldConstants.ORDER_CONFIG_FIELDS`）。
    ///
    /// ⚠️ **`ORDER_CONFIG_FIELDS` 是空数组 `{}`**（`CheckFieldConstants.java:76`），
    /// 因此 `Arrays.asList(allowFields).contains(field)` **恒为 false**，
    /// Java 的这个接口对**任何**字段都抛 `字段值不合法`。
    ///
    /// 本实现逐字符复刻该可观测行为：归一化 → 白名单（空）→ 必然拒绝。
    /// 这是**有意保留 Java 的缺陷**，不是遗漏；「顺手修好」会造成
    /// Rust 与 Java 线上行为不一致（Java 恒失败、Rust 成功），属未被授权的
    /// 业务变更。若产品确认应启用，把
    /// `nextrade_core::models::merchant::order_config::ORDER_CONFIG_FIELDS` 改成
    /// `["code", "data"]` 即可（`shop_id` 是租户隔离列，不宜由前端直改）。
    ///
    /// # Errors
    /// - `id` 缺失/非正 ⇒ `id不能为空`
    /// - `field` 缺失/空白 ⇒ 字段名不能为空
    /// - 字段名无法归一，或不在（空的）白名单内 ⇒ `字段值不合法`
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

        // 1. 归一（Java `getColumnByPropertyName`）：
        //    未识别在 Java 里回退主键列 `id`，`id` 同样不在白名单 ⇒ 结果一致
        let column = map_update_field_column(field);
        // 2. 白名单（Java `ORDER_CONFIG_FIELDS` 为空 ⇒ 必然落到下面这一行）
        let allowed = column.is_some_and(|c| ORDER_CONFIG_FIELDS.contains(&c));
        if !allowed {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        // ——— 以下仅在白名单被启用后可达（当前为死代码，保留以免重写 SQL 层）———

        // 3. 二次保险：白名单项必须是真实列
        let column = column.unwrap_or_default();
        if !ORDER_CONFIG_COLUMNS.contains(&column) {
            return Err(Error::Validation(MSG_INVALID_FIELD_VALUE.into()));
        }

        let val = dto.val.as_deref().unwrap_or_default();
        let affected = self.repos.merchant.order_config.update_field(id, column, val).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 批量操作 —————————————————————

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// `type != "del"` → **静默返回 `false`**（Java 行为，不是报错）。
    /// `type == "del"` → `removeByIds(ids)`，返回 `rows >= 1`。
    ///
    /// ⚠️ Java 控制器类级 `@Validated` + 参数 `@Valid` 会让
    /// `BatchDTO.ids` 的 `@NotEmpty` 生效（空数组 → 400）。
    /// 本实现按项目惯例在服务层返回 `false` 而不是 400
    /// （与 pc_cat_floor / area_code 的选择一致，见 `uncertain_points`）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type != "del" {
            return Ok(false);
        }
        if dto.ids.is_empty() {
            // Java `removeByIds(空集合)` 直接返回 false，不产生 SQL
            return Ok(false);
        }
        let affected = self.repos.merchant.order_config.delete_many(&dto.ids).await?;
        Ok(affected > 0)
    }

    // ————————————————————— 非本切片路由：店铺维度查询 —————————————————————

    /// 获取店铺订单配置详情（对应 Java `OrderConfigServiceImpl#shopOrderConfigDetail`）。
    ///
    /// Java 返回 `JSON`（Hutool `JSONUtil.parse(config.getData())`），
    /// `code` / `shopId` 任一查不到或 `data` 为 null 时返回 `null`。
    ///
    /// ⚠️ **本方法不对应本切片的任何路由**：本切片只有 7 条管理端接口，
    /// 而 `shopOrderConfigDetail` 由**前台**（店铺订单设置页）调用。
    /// 保留它是为了让同一张表的读取逻辑集中在一处，前台切片可直接复用；
    /// 它**没有**被 `routes()` 注册，不影响覆盖率统计。
    pub async fn shop_order_config_detail(
        &self,
        shop_id: i32,
        code: &str,
    ) -> Result<Option<String>> {
        let c = self
            .repos.merchant.order_config
            .find_by_shop_and_code(shop_id, code)
            .await?;
        Ok(c.and_then(|c| c.data))
    }

    /// 本次入参是否提供了可用于落库的店铺 id（供路由层 / 调试观察，非 Java 概念）。
    #[must_use]
    pub fn has_shop_id(dto: &OrderConfigCreateDTO) -> bool {
        dto.shop_id.is_some_and(|v| v > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::merchant::order_config::{
        map_order_column, BatchDTO, OrderConfigCreateDTO, OrderConfigListQuery,
        OrderConfigUpdateDTO, UpdateFieldDTO, MSG_ORDER_CONFIG_ID_NOT_NULL,
        MSG_ORDER_CONFIG_NAME_NOT_BLANK, MSG_ORDER_CONFIG_NAME_OVER_LENGTH,
    };

    // ————————————————————— 常量：与 Java 逐字符对照 —————————————————————

    /// Java `CheckFieldConstants.ORDER_CONFIG_FIELDS` 是空数组 ⇒ updateField 恒失败。
    #[test]
    fn update_field_whitelist_is_empty_by_java_design() {
        assert!(ORDER_CONFIG_FIELDS.is_empty());
        // 即便是表里真实存在的列，也进不了这个（空的）白名单
        for c in ORDER_CONFIG_COLUMNS {
            assert!(!ORDER_CONFIG_FIELDS.contains(&c));
        }
    }

    /// 归一函数：四个真实列都能归一出列名，未识别的返回 None。
    #[test]
    fn update_field_column_mapping() {
        assert_eq!(map_update_field_column("id"), Some("id"));
        assert_eq!(map_update_field_column("shop_id"), Some("shop_id"));
        assert_eq!(map_update_field_column("shopId"), Some("shop_id"));
        assert_eq!(map_update_field_column(" code "), Some("code"));
        assert_eq!(map_update_field_column("data"), Some("data"));
        assert_eq!(map_update_field_column("orderConfigName"), None);
        assert_eq!(map_update_field_column(""), None);
    }

    /// 排序字段映射（`buildSortOrder` 的白名单来源）。
    #[test]
    fn order_column_mapping() {
        assert_eq!(map_order_column("shopId"), "shop_id");
        assert_eq!(map_order_column("code"), "code");
        assert_eq!(map_order_column("data"), "data");
        assert_eq!(map_order_column("bogus"), "id");
    }

    // ————————————————————— 纯逻辑：updateField 的必然失败路径 —————————————————————

    /// 归一 + 空白名单 ⇒ 任何字段都判「字段值不合法」。
    ///
    /// 这是对 Java `BaseServiceImpl#updateField` 的纯逻辑复刻，
    /// 不需要数据库即可验证。
    #[test]
    fn update_field_always_rejected() {
        let rejected = |field: &str| {
            let column = map_update_field_column(field);
            !column.is_some_and(|c| ORDER_CONFIG_FIELDS.contains(&c))
        };
        for f in ["id", "shop_id", "shopId", "code", "data", "orderConfigName"] {
            assert!(rejected(f), "字段 {f} 不应该通过（Java 白名单为空）");
        }
    }

    // ————————————————————— 纯逻辑：create 的校验顺序 —————————————————————

    /// Java 先校验 `orderConfigName`（`@NotBlank` / `@Size`），
    /// 校验通过后才轮到「表列补齐」这一步。
    #[test]
    fn create_validates_name_before_columns() {
        // 名称缺失 ⇒ 报 Java 的文案，而不是「店铺id不能为空」
        let dto = OrderConfigCreateDTO::default();
        assert_eq!(
            validate_create(&dto).unwrap_err(),
            MSG_ORDER_CONFIG_NAME_NOT_BLANK
        );

        // 名称超长 ⇒ Java 的 @Size 文案
        let dto = OrderConfigCreateDTO {
            order_config_name: Some("x".repeat(51)),
            ..Default::default()
        };
        assert_eq!(
            validate_create(&dto).unwrap_err(),
            MSG_ORDER_CONFIG_NAME_OVER_LENGTH
        );

        // 名称合法 ⇒ 校验通过（后续才检查 shopId / code）
        let dto = OrderConfigCreateDTO {
            order_config_name: Some("结算配置".into()),
            ..Default::default()
        };
        assert!(validate_create(&dto).is_ok());
        assert_eq!(validate_create(&dto).unwrap(), "结算配置");
        // 名称前后的空白被 trim 掉（Java `@NotBlank` 同样忽略纯空白）
        assert!(!OrderConfigService::has_shop_id(&dto));
    }

    /// `has_shop_id` 只在提供正整数时为真（辅助判据，非 Java 概念）。
    #[test]
    fn has_shop_id_requires_positive() {
        let mk = |v: Option<i32>| OrderConfigCreateDTO {
            shop_id: v,
            ..Default::default()
        };
        assert!(!OrderConfigService::has_shop_id(&mk(None)));
        assert!(!OrderConfigService::has_shop_id(&mk(Some(0))));
        assert!(!OrderConfigService::has_shop_id(&mk(Some(-1))));
        assert!(OrderConfigService::has_shop_id(&mk(Some(2))));
    }

    // ————————————————————— 纯逻辑：update 的主键与空补丁 —————————————————————

    /// `OrderConfigUpdateDTO` 只有主键带 `@NotNull`（Java 文案逐字符）。
    #[test]
    fn update_dto_requires_id() {
        let dto = OrderConfigUpdateDTO::default();
        assert_eq!(
            dto.require_id().unwrap_err(),
            MSG_ORDER_CONFIG_ID_NOT_NULL
        );

        let dto = OrderConfigUpdateDTO {
            order_config_id: Some(0),
            ..Default::default()
        };
        assert_eq!(
            dto.require_id().unwrap_err(),
            MSG_ORDER_CONFIG_ID_NOT_NULL
        );

        let dto = OrderConfigUpdateDTO {
            order_config_id: Some(9),
            ..Default::default()
        };
        assert_eq!(dto.require_id().unwrap(), 9);
    }

    /// Java 的 update 在「除主键外全 null」时会拼出非法 SQL；
    /// 这里验证补丁判空逻辑确实能识别出那种情况。
    #[test]
    fn empty_patch_is_detected() {
        let dto = OrderConfigUpdateDTO {
            order_config_id: Some(1),
            order_config_name: Some("只改了表里不存在的字段".into()),
            is_show: Some(1),
            status: Some(1),
            sort_order: Some(3),
            order_config_pic: Some("/a.png".into()),
            ..Default::default()
        };
        // Java DTO 的 5 个业务字段全都不是表的列 ⇒ 补丁为空
        let patch = OrderConfigPatch {
            shop_id: dto.shop_id.map(Some),
            code: dto.code.clone().map(Some),
            data: dto.data.clone().map(Some),
        };
        assert!(patch.is_empty());

        // 只要提供一个真实列就有内容
        let dto = OrderConfigUpdateDTO {
            data: Some("{}".into()),
            ..dto
        };
        let patch = OrderConfigPatch {
            shop_id: dto.shop_id.map(Some),
            code: dto.code.clone().map(Some),
            data: dto.data.clone().map(Some),
        };
        assert!(!patch.is_empty());
    }

    // ————————————————————— 纯逻辑：del / batch —————————————————————

    /// 删除请求缺主键 ⇒ Java `OperateDTO.id` 的 `@NotNull` 文案。
    #[test]
    fn operate_dto_requires_id() {
        let dto: OperateDTO = serde_json::from_str(r#"{"adminNote":"x"}"#).unwrap();
        assert_eq!(dto.id, None);
        assert_eq!(MSG_ID_REQUIRED, "id不能为空");
    }

    /// `BatchDTO.type != "del"` ⇒ 静默 false（不报错），空 ids 同样 false。
    #[test]
    fn batch_type_gate() {
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1],"type":"show"}"#).unwrap();
        assert_ne!(dto.r#type, "del");
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[],"type":"del"}"#).unwrap();
        assert!(dto.ids.is_empty());
        let dto: BatchDTO = serde_json::from_str(r#"{"ids":[1,2,3],"type":"del"}"#).unwrap();
        assert_eq!(dto.ids, vec![1, 2, 3]);
    }

    /// 字段名缺失同样要被拦下（Java `UpdateFieldDTO.field` 的 `@NotNull`）。
    #[test]
    fn update_field_requires_field_name() {
        let dto: UpdateFieldDTO =
            serde_json::from_str(r#"{"id":1,"field":"code","val":"a"}"#).unwrap();
        assert_eq!(dto.field.as_deref(), Some("code"));

        let dto: UpdateFieldDTO = serde_json::from_str(r#"{"id":1,"val":"a"}"#).unwrap();
        assert!(dto.field.is_none());
    }

    // ————————————————————— 纯逻辑：list 查询参数 —————————————————————

    /// `isShow` / `keyword` 在 Java 里被注释掉 ⇒ 不产生任何过滤差异。
    #[test]
    fn list_filters_are_inert_like_java() {
        let q = OrderConfigListQuery {
            is_show: Some(1),
            keyword: Some("结算".into()),
            sort_field: Some("code".into()),
            sort_order: Some("asc".into()),
            page: Some(2),
            size: Some(20),
        };
        assert_eq!(q.order_clause(), " ORDER BY code ASC");
        let p = q.page_query();
        assert_eq!((p.page, p.page_size), (2, 20));
    }
}