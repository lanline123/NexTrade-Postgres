//! 代客下单自提联系人管理服务（trade::proxy_shopping_pickup_contact）。
//!
//! 对应原 Java
//! `com.nextrade.adminapi.controller.proxyshopping.ProxyShoppingPickupContactController`
//! （`@RequestMapping("/adminapi/proxyShopping/pickupContact")`，
//! `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`）
//! → `com.nextrade.service.user.impl.PickupContactServiceImpl`
//! （`extends BaseServiceImpl<PickupContactMapper, PickupContact>`）。
//!
//! | 本模块方法 | 原 Java | 说明 |
//! |-----------|--------|------|
//! | [`ProxyShoppingPickupContactService::list`] | `#list` | 分页 + `userId`/`keyword` 过滤 |
//! | [`ProxyShoppingPickupContactService::list_by_user_id`] | `#listByUserId` | 按客户取全部，`id` 降序 |
//! | [`ProxyShoppingPickupContactService::detail`] | `#detail` | 不存在 → 1001「自提联系人不存在」 |
//! | [`ProxyShoppingPickupContactService::create`] | `#create` | 返回自增主键 |
//! | [`ProxyShoppingPickupContactService::update`] | `#update` | 不存在 → 1001；`null` 字段保持原值 |
//! | [`ProxyShoppingPickupContactService::delete`] | `#del` | 逻辑删除；`id` 缺失 → `false`（不报错） |
//!
//! # 逐条对照 Java 的语义要点
//!
//! 1. **逻辑删除**：实体 `PickupContact.isDel` 带 `@TableLogic`，
//!    故 `removeById` 是 `UPDATE ... SET is_del = 1`，读一律带 `is_del = 0`。
//!    已删除的行在 `detail` / `update` 里等同「不存在」。
//! 2. **`detail` 不存在 → `GlobalException("自提联系人不存在")`**：
//!    `GlobalException(String)` 的 code 恒为 `HttpStatus.SERVICE_DATA_ERROR`
//!    （1001），`Error::biz` 的 `status_code()` 对非 4xx 业务码返回 HTTP 200，
//!    与原系统「HTTP 200 + 业务码 1001」一致。
//! 3. **`create` 无参数校验**：控制器**没有** `@Validated`，三个 DTO 也**没有**
//!    任何校验注解，`null` 字段由 MyBatis-Plus `FieldStrategy.NOT_NULL` 从
//!    INSERT 列表剔除。`pickup_contact.contact_name` / `contact_phone` 是
//!    `NOT NULL DEFAULT ''`，故传 `null` 会**插入空串**；`user_id` 既非空
//!    又无默认值，传 `null` 会报数据库错误（本实现同样由数据库报错）。
//! 4. **`update` 的「非空才改」**：Java 先 `getById`（拿原值），再把
//!    `contactName` / `contactPhone` **无条件**赋给实体（可能为 `null`），
//!    `userId` 仅在非 `null` 时赋值；随后 `updateById` 只写非 `null` 字段。
//!    净效果 = 「入参为 `null` → 保留数据库原值」，本实现按此解析出最终值
//!    再一次性 UPDATE。`update_time` 由 `updateFill` 刷为当前秒级时间戳。
//! 5. **`del` 的 `id == null` 分支**：Java `@RequestParam("id") Long id` 实际
//!    必填，但服务层仍写了 `if (id == null) return false;`，本实现照抄
//!    （返回 `Ok(false)`，不报错、不产生副作用）。
//!    必填参数缺失在 Spring 里会先被 `GlobalExceptionHandler` 拦成
//!    1001「接口参数缺失」——见 [`require_id`] / [`require_user_id`]。
//! 6. **分页结构**：Java 返回 MyBatis-Plus `Page<PickupContactVO>`
//!    （`records/total/size/current/pages`），Rust 统一使用项目既有
//!    `PageResult`（`list/total/totalPage/pageSize/page`），与
//!    order_admin / stock_warning / buyer_show_comment 同一约定。
//! 7. **`@Sensitive(PHONE)` 脱敏只在演示站生效**：Java
//!    `SensitiveSerializer` 的条件是 `nextrade.is-demo == 1`，
//!    而 `nextrade-adminapi/src/main/resources/application.yaml:134` 的
//!    默认值是 `is-demo: 0` → **发客户版本原样返回手机号**。
//!    Rust 侧无配置服务，故用常量 [`IS_DEMO`]（= `0`，逐字对齐该 yaml）
//!    并在 [`visible_phone`] 里复刻同一条件；若部署为演示站必须改此常量
//!    并重新编译（已列入切片的 `uncertain_points`）。
//!
//! # 未迁移 / 无依赖
//!
//! 本切片不依赖对象存储、支付、Redis 缓存、翻译服务或外部配置服务；
//! 只读写单表 `pickup_contact`。
//!
//! # 与 Java 的已知差异
//!
//! - **权限**：Java 在控制器类上标 `@PreAuthorize("@pms.hasPermission('proxyShoppingManage')")`
//!   做方法级鉴权；Rust 侧 `require_auth` 只做登录态校验，权限矩阵尚未实现
//!   （与 seckill.rs / order_admin.rs / buyer_show_group.rs 同一已知缺口）。
//!   即**任何已登录管理员**都能调用本域全部 6 条接口。
//! - **必填参数缺失的形态**：Java 由 Spring 抛
//!   `MissingServletRequestParameterException` → 1001 + 「接口参数缺失」；
//!   本实现在服务层做同样文案的校验（HTTP 200 + code 1001）。
//!   但**参数类型错误**（如 `?id=abc`）本实现由 axum `Query` 拒绝成 HTTP 400，
//!   而 Java 会落到兜底 `Exception` handler 回 1001 + 原始异常文案。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::PickupContact;
use nextrade_core::models::trade::proxy_shopping_pickup_contact::{
    PickupContactCreateDTO, PickupContactListQuery, PickupContactUpdateDTO, PickupContactVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

// ——————————————————————————————————————————————————————————————
// 常量（逐字对齐 Java）
// ——————————————————————————————————————————————————————————————

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `PickupContactServiceImpl#detail` / `#update` 的
/// `new GlobalException("自提联系人不存在")` 文案。
pub const ERR_CONTACT_NOT_FOUND: &str = "自提联系人不存在";

/// Java `MissingServletRequestParameterException` 的响应文案
/// （`messageHelper.get("error.param.interface.missing")`，与 buyer_show_group 同一串）。
pub const ERR_INTERFACE_PARAM_MISSING: &str = "接口参数缺失";

/// Java `NexTradeProperties.isDemo`：`application.yaml` 的
/// `nextrade.is-demo: 0`（发客户版本，默认不脱敏）。
///
/// **配置服务未迁移**：Rust 侧没有 `nextrade.*` 配置读取入口，
/// 这里按当前 yaml 的取值硬编码。部署演示站（`is_demo = 1`）时必须改此常量
/// 并重新编译，否则 `@Sensitive(PHONE)` 的脱敏不会生效。
pub const IS_DEMO: i32 = 0;

// ——————————————————————————————————————————————————————————————
// 纯函数（可单测）
// ——————————————————————————————————————————————————————————————

/// 必填主键参数校验（`detail` / `del` 的 `@RequestParam("id") Long id`）。
///
/// Java 缺失必填 `@RequestParam` 时由 `GlobalExceptionHandler` 统一回
/// 1001 + 「接口参数缺失」（该 handler 的 code 是 `SERVICE_DATA_ERROR`）。
pub fn require_id(id: Option<i64>) -> Result<i64> {
    id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_INTERFACE_PARAM_MISSING))
}

/// 必填客户ID参数校验（`listByUserId` 的 `@RequestParam("userId") Integer userId`）。
pub fn require_user_id(user_id: Option<i32>) -> Result<i32> {
    user_id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_INTERFACE_PARAM_MISSING))
}

/// `del` 的入参解析：复刻 Java `PickupContactServiceImpl#del` 的
/// `if (id == null) return false;`。
///
/// 返回 `None` 表示「没有可用主键，静默不操作（返回 `false`）」。
/// `id = 0` / 负数都是**合法值**（Java 会照样发 SQL，影响 0 行 → `false`），
/// 不得当作缺省。
#[must_use]
pub fn resolve_delete_id(id: Option<i64>) -> Option<i64> {
    id
}

/// Java `SensitiveSerializer#maskPhone` 的逐字复刻。
///
/// ```text
/// if (phone.contains("-")) {
///     areaCode = phone.substring(0, idx); localNum = phone.substring(idx + 1); len = localNum.length();
///     len >= 8 → areaCode + "-****" + localNum.substring(len - 4)
///     len >= 7 → areaCode + "-****" + localNum.substring(len - 3)
///     len >= 2 → areaCode + "-****" + localNum.substring(len - len / 2)
///     else     → areaCode + "-****"
/// }
/// phone.length() < 7 → phone.charAt(0) + "****" + phone.charAt(phone.length() - 1)
/// else               → phone.substring(0, 3) + "****" + phone.substring(phone.length() - 4)
/// ```
///
/// 差异说明：Java 按 UTF-16 code unit 切分，这里按 `char`（Unicode 标量）切分。
/// 对手机号（ASCII 数字）结果完全一致；只有含代理对字符的异常数据才可能不同。
///
/// 另：Java 对**空串**会在 `phone.charAt(0)` 处抛
/// `StringIndexOutOfBoundsException`，但 `SensitiveSerializer#serialize`
/// 已用 `!value.isEmpty()` 挡在前面，空串永不进入 `mask`。
/// 本函数对空串返回空串（保持纯函数不 panic），行为差异不可达。
#[must_use]
pub fn mask_phone_java(phone: &str) -> String {
    if phone.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = phone.chars().collect();

    if let Some(idx) = phone.find('-') {
        // `find` 返回的是**字节**下标，需换算成字符下标（ASCII `-` 为 1 字节，
        // 但前导区号可能含多字节字符）。
        let char_idx = phone[..idx].chars().count();
        let area_code: String = chars[..char_idx].iter().collect();
        let local: &[char] = &chars[char_idx + 1..];
        let len = local.len();
        if len >= 8 {
            let tail: String = local[len - 4..].iter().collect();
            return format!("{area_code}-****{tail}");
        }
        if len >= 7 {
            let tail: String = local[len - 3..].iter().collect();
            return format!("{area_code}-****{tail}");
        }
        if len >= 2 {
            let keep = len / 2;
            let tail: String = local[len - keep..].iter().collect();
            return format!("{area_code}-****{tail}");
        }
        return format!("{area_code}-****");
    }

    if chars.len() < 7 {
        return format!("{}****{}", chars[0], chars[chars.len() - 1]);
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
}

/// `contactPhone` 的最终返回形态：仅当 `is_demo == 1`（演示站）且非空时脱敏。
///
/// 对应 Java `SensitiveSerializer#serialize`：
/// `sensitiveType != null && isDemoEnabled() && value != null && !value.isEmpty()`。
#[must_use]
pub fn visible_phone(phone: &str, is_demo: i32) -> String {
    if is_demo == 1 && !phone.is_empty() {
        mask_phone_java(phone)
    } else {
        phone.to_string()
    }
}

/// 实体 → 出参（对应 Java `BeanUtils.copyProperties(entity, vo)` +
/// `@Sensitive(PHONE)` 序列化）。
#[must_use]
pub fn to_vo(entity: &PickupContact) -> PickupContactVO {
    PickupContactVO {
        id: entity.id,
        user_id: entity.user_id,
        contact_name: entity.contact_name.clone(),
        contact_phone: visible_phone(&entity.contact_phone, IS_DEMO),
        create_time: entity.create_time,
        update_time: entity.update_time,
    }
}

/// `update` 的字段合并：复刻「Java 把入参无条件赋给实体、再 `updateById`
/// 只写非 `null` 字段」的净效果。
///
/// - `contact_name` / `contact_phone` 为 `None` → 保留 `current` 里的原值；
/// - `user_id` 为 `None` → 保留原值。
#[must_use]
pub fn merge_update_fields(
    current: &PickupContact,
    dto: &PickupContactUpdateDTO,
) -> (i32, String, String) {
    let user_id = dto.user_id.unwrap_or(current.user_id);
    let contact_name = dto
        .contact_name
        .clone()
        .unwrap_or_else(|| current.contact_name.clone());
    let contact_phone = dto
        .contact_phone
        .clone()
        .unwrap_or_else(|| current.contact_phone.clone());
    (user_id, contact_name, contact_phone)
}

// ——————————————————————————————————————————————————————————————
// 服务
// ——————————————————————————————————————————————————————————————

/// 代客下单自提联系人服务。
pub struct ProxyShoppingPickupContactService {
    repos: Arc<Repositories>,
}

impl ProxyShoppingPickupContactService {
    /// 构造：持有全局仓储集合（与 product_manage/002/…/026 同一约定）。
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 自提联系人分页（Java `ProxyShoppingPickupContactController.list` → `#list`）。
    ///
    /// 与 Java 一致：当前页无数据时直接返回空分页。
    pub async fn list(
        &self,
        q: &PickupContactListQuery,
    ) -> Result<PageResult<PickupContactVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.trade.proxy_shopping_pickup_contact.paginate(q).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 某客户的全部自提联系人（Java `#listByUserId`）。
    ///
    /// Java `eq(userId).orderByDesc(id)`；不校验 `userId` 是否真实存在
    /// （不存在的客户返回空数组）。
    pub async fn list_by_user_id(&self, user_id: i32) -> Result<Vec<PickupContactVO>> {
        let rows = self.repos.trade.proxy_shopping_pickup_contact.list_by_user_id(user_id).await?;
        Ok(rows.iter().map(to_vo).collect())
    }

    /// 自提联系人详情（Java `#detail`）。
    ///
    /// 不存在（含已逻辑删除）→ `Err(1001, "自提联系人不存在")`。
    pub async fn detail(&self, id: i64) -> Result<PickupContactVO> {
        let entity = self
            .repos.trade.proxy_shopping_pickup_contact
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_CONTACT_NOT_FOUND))?;
        Ok(to_vo(&entity))
    }

    /// 新建自提联系人（Java `#create`），返回自增主键。
    ///
    /// `user_id` / `contact_name` / `contact_phone` 为 `None` 时不进 INSERT
    /// 列表（`FieldStrategy.NOT_NULL`），由列默认值兜底；`user_id` 无默认值，
    /// 传 `None` 会由数据库报错（与 Java 一致）。
    pub async fn create(&self, dto: &PickupContactCreateDTO) -> Result<i64> {
        let now = now_seconds();
        self.repos.trade.proxy_shopping_pickup_contact
            .insert(
                dto.user_id,
                dto.contact_name.as_deref(),
                dto.contact_phone.as_deref(),
                now,
            )
            .await
    }

    /// 修改自提联系人（Java `#update`），返回是否更新成功。
    ///
    /// 先 `getById`（等价于 `find_by_id`，带 `is_del = 0`）：
    /// 不存在 → `Err(1001, "自提联系人不存在")`。
    /// `id` 为 `None` 时 Java 的 `getById(null)` 同样查不到 → 同一错误。
    pub async fn update(&self, dto: &PickupContactUpdateDTO) -> Result<bool> {
        let Some(id) = dto.id else {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_CONTACT_NOT_FOUND));
        };
        let current = self
            .repos.trade.proxy_shopping_pickup_contact
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_CONTACT_NOT_FOUND))?;
        let (user_id, contact_name, contact_phone) = merge_update_fields(&current, dto);
        self.repos.trade.proxy_shopping_pickup_contact
            .update(id, user_id, &contact_name, &contact_phone, now_seconds())
            .await
    }

    /// 删除自提联系人（Java `#del`，逻辑删除），返回是否删掉了一行。
    ///
    /// - `id` 为 `None` → `Ok(false)`（Java `if (id == null) return false;`）
    /// - 目标不存在 / 已删除 → `Ok(false)`
    pub async fn delete(&self, id: Option<i64>) -> Result<bool> {
        let Some(id) = resolve_delete_id(id) else {
            return Ok(false);
        };
        self.repos.trade.proxy_shopping_pickup_contact.soft_delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::trade::proxy_shopping_pickup_contact::{PickupContactIdQuery, PickupContactUserIdQuery};

    fn contact() -> PickupContact {
        PickupContact {
            id: 9,
            user_id: 42,
            contact_name: "张三".into(),
            contact_phone: "13812348000".into(),
            create_time: Some(1_700_000_000),
            update_time: Some(1_700_000_100),
            is_del: 0,
        }
    }

    // ————————————————— 错误码与文案 —————————————————

    #[test]
    fn business_code_matches_java_global_exception() {
        assert_eq!(SERVICE_DATA_ERROR, 1001);
        let err = Error::biz(SERVICE_DATA_ERROR, ERR_CONTACT_NOT_FOUND);
        assert_eq!(err.business_code(), 1001);
        // 非 4xx 业务码 → HTTP 200（原 Java 的 ResultWrapper 形态）
        assert_eq!(err.status_code().as_u16(), 200);
    }

    #[test]
    fn not_found_text_is_verbatim_from_java() {
        assert_eq!(ERR_CONTACT_NOT_FOUND, "自提联系人不存在");
        match Error::biz(SERVICE_DATA_ERROR, ERR_CONTACT_NOT_FOUND) {
            Error::Biz { code, msg } => {
                assert_eq!(code, 1001);
                assert_eq!(msg, "自提联系人不存在");
            }
            other => panic!("应为业务错误，实际 {other:?}"),
        }
    }

    // ————————————————— 必填参数校验 —————————————————

    #[test]
    fn require_id_reports_interface_param_missing() {
        let err = require_id(None).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert_eq!(err.to_string(), format!("业务错误 [1001]: {ERR_INTERFACE_PARAM_MISSING}"));

        assert_eq!(require_id(Some(0)).unwrap(), 0);
        assert_eq!(require_id(Some(-1)).unwrap(), -1);
        assert_eq!(require_id(Some(7)).unwrap(), 7);
    }

    #[test]
    fn require_user_id_reports_interface_param_missing() {
        assert!(require_user_id(None).is_err());
        // 0 与负数不被当成缺省（Java 只判 null）
        assert_eq!(require_user_id(Some(0)).unwrap(), 0);
        assert_eq!(require_user_id(Some(-3)).unwrap(), -3);
    }

    #[test]
    fn delete_id_is_optional_and_null_means_noop() {
        // Java `del(null) return false`：不报错、不操作。
        assert_eq!(resolve_delete_id(None), None);
        assert_eq!(resolve_delete_id(Some(7)), Some(7));
        // 0 / 负数原样下发（Java 不做范围校验，影响 0 行 → false）
        assert_eq!(resolve_delete_id(Some(0)), Some(0));
        assert_eq!(resolve_delete_id(Some(-1)), Some(-1));
    }

    // ————————————————— 脱敏 —————————————————

    #[test]
    fn mask_phone_matches_java_algorithms() {
        // 无区号、长度 >= 7：前 3 + **** + 后 4
        assert_eq!(mask_phone_java("13812348000"), "138****8000");
        assert_eq!(mask_phone_java("1234567"), "123****4567");
        // 无区号、长度 < 7：首字符 + **** + 末字符
        assert_eq!(mask_phone_java("123456"), "1****6");
        assert_eq!(mask_phone_java("12"), "1****2");
        assert_eq!(mask_phone_java("1"), "1****1");
        // 含区号：areaCode + "-****" + 尾部，按本地号长度分支
        assert_eq!(mask_phone_java("010-12345678"), "010-****5678"); // len 8 → 尾 4
        assert_eq!(mask_phone_java("010-1234567"), "010-****567"); // len 7 → 尾 3
        assert_eq!(mask_phone_java("010-123456"), "010-****456"); // len 6 → keep 3
        assert_eq!(mask_phone_java("010-1"), "010-****"); // len 1 → 无尾
        assert_eq!(mask_phone_java("010-"), "010-****"); // 本地号空
        // 空串：Java 会在 charAt(0) 抛异常，但序列化器已挡在前面；
        // 本实现返回空串（不可达差异）
        assert_eq!(mask_phone_java(""), "");
    }

    #[test]
    fn visible_phone_only_masks_on_demo_site() {
        // `application.yaml` 的 `nextrade.is-demo: 0` → 发客户版本不脱敏
        assert_eq!(visible_phone("13812348000", 0), "13812348000");
        assert_eq!(visible_phone("13812348000", IS_DEMO), "13812348000");
        // 演示站才脱敏
        assert_eq!(visible_phone("13812348000", 1), "138****8000");
        // 空串在任何模式下都不脱敏
        assert_eq!(visible_phone("", 1), "");
    }

    // ————————————————— VO 映射 —————————————————

    #[test]
    fn to_vo_maps_all_fields_and_hides_is_del() {
        let vo = to_vo(&contact());
        assert_eq!(vo.id, 9);
        assert_eq!(vo.user_id, 42);
        assert_eq!(vo.contact_name, "张三");
        assert_eq!(vo.contact_phone, "13812348000");
        assert_eq!(vo.create_time, Some(1_700_000_000));
        assert_eq!(vo.update_time, Some(1_700_000_100));

        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["userId"], 42);
        assert_eq!(json["contactPhone"], "13812348000");
        assert!(json.get("isDel").is_none());
    }

    #[test]
    fn to_vo_keeps_null_timestamps() {
        let mut c = contact();
        c.create_time = None;
        c.update_time = None;
        let vo = to_vo(&c);
        assert_eq!(vo.create_time, None);
        assert_eq!(vo.update_time, None);
    }

    // ————————————————— update 字段合并 —————————————————

    fn update_dto(
        id: Option<i64>,
        user_id: Option<i32>,
        name: Option<&str>,
        phone: Option<&str>,
    ) -> PickupContactUpdateDTO {
        PickupContactUpdateDTO {
            id,
            user_id,
            contact_name: name.map(str::to_string),
            contact_phone: phone.map(str::to_string),
        }
    }

    #[test]
    fn merge_update_keeps_existing_values_for_none() {
        // Java：null 的 contactName/contactPhone/userId 不会被 updateById 写库
        let cur = contact();
        let (uid, name, phone) = merge_update_fields(&cur, &update_dto(Some(9), None, None, None));
        assert_eq!(uid, 42);
        assert_eq!(name, "张三");
        assert_eq!(phone, "13812348000");
    }

    #[test]
    fn merge_update_applies_present_values_including_empty_string() {
        let cur = contact();
        let (uid, name, phone) =
            merge_update_fields(&cur, &update_dto(Some(9), Some(7), Some(""), Some("010-1")));
        assert_eq!(uid, 7);
        // 空串是「有值」：Java 的 NOT_NULL 策略会照写空串
        assert_eq!(name, "");
        assert_eq!(phone, "010-1");
    }

    #[test]
    fn merge_update_user_id_zero_is_applied() {
        // Java `if (updateDTO.getUserId() != null)`：0 不是 null，会被写入
        let cur = contact();
        let (uid, _, _) = merge_update_fields(&cur, &update_dto(Some(9), Some(0), None, None));
        assert_eq!(uid, 0);
    }

    // ————————————————— 入参 JSON 兼容性 —————————————————

    #[test]
    fn dtos_accept_frontend_payloads() {
        let create: PickupContactCreateDTO = serde_json::from_str(
            r#"{"userId":42,"contactName":"李四","contactPhone":"13900000000"}"#,
        )
        .unwrap();
        assert_eq!(create.user_id, Some(42));
        assert_eq!(create.contact_name.as_deref(), Some("李四"));
        assert_eq!(create.contact_phone.as_deref(), Some("13900000000"));

        let update: PickupContactUpdateDTO =
            serde_json::from_str(r#"{"id":9,"contactName":"李四"}"#).unwrap();
        assert_eq!(update.id, Some(9));
        assert_eq!(update.user_id, None);

        let list: PickupContactListQuery =
            serde_json::from_str(r#"{"page":1,"size":15,"userId":42,"keyword":"138"}"#).unwrap();
        assert_eq!(list.user_id, Some(42));
        assert_eq!(list.keyword.as_deref(), Some("138"));

        let by_user: PickupContactUserIdQuery =
            serde_json::from_str(r#"{"userId":42}"#).unwrap();
        assert_eq!(require_user_id(by_user.user_id).unwrap(), 42);

        let detail: PickupContactIdQuery = serde_json::from_str(r#"{"id":9}"#).unwrap();
        assert_eq!(require_id(detail.id).unwrap(), 9);
    }
}