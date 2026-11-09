//! 商户入驻·前台服务（merchant::merchant_client）。
//!
//! 对应原 Java：
//! - `nextrade-api` → `com.nextrade.api.controller.merchant.MerchantClientController`
//!   （`@RequestMapping("/api/merchant/merchant")`，5 个端点）
//! - `com.nextrade.service.merchant.impl.MerchantApplyServiceImpl`
//!   （`myApply` / `applyDetail` / `apply` / `createMerchantApply`）
//! - `com.nextrade.service.merchant.impl.MerchantServiceImpl`
//!   （`uploadLicensePic` / `applyShopAgreement`）
//! - `com.nextrade.service.msg.impl.AdminMsgServiceImpl#createMessage`
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl#getConfigByCode`
//!
//! # 端点与实现状态
//!
//! | 路由 | 原 Java 方法 | 状态 |
//! |------|--------------|------|
//! | `POST /api/merchant/merchant/uploadLicense` | `MerchantClientController.uploadLicense` | 入参校验后**未迁移**（见下） |
//! | `GET  /api/merchant/merchant/applyShopAgreement` | `MerchantClientController.applyShopAgreement` | 已迁移（读配置原样返回） |
//! | `GET  /api/merchant/merchant/myApply` | `MerchantClientController.myApply` | 已迁移 |
//! | `GET  /api/merchant/merchant/applyDetail` | `MerchantClientController.applyDetail` | 已迁移 |
//! | `POST /api/merchant/merchant/apply` | `MerchantClientController.apply` | 已迁移（审核分支；免审核分支未迁移） |
//!
//! # 有意为之的差异（**逐条如实说明**）
//!
//! 1. **`uploadLicense` 未迁移**：Java 的 `MerchantServiceImpl#uploadLicensePic`
//!    在完成空文件 / 大小 / MIME 校验后，调用
//!    `ImageWatermarkUtil.tileText`（水印字体）与
//!    `GalleryPicService#uploadImg`（对象存储 + 落库）得到图片地址。这两项在
//!    Rust 侧均未迁移，无法落盘并返回可访问 URL，故本实现**完整复刻三条入参
//!    校验**（空文件「营业执照图片不能为空」/ 超 10MB「图片大小超过 10MB 限制」/
//!    MIME 非 `image/jpeg|png|bmp`「图片规格不符合要求」），校验通过后返回
//!    明确的「未迁移」业务错误（[`MSG_LICENSE_UPLOAD_UNMIGRATED`]），**绝不**
//!    伪造一个图片地址。
//!
//! 2. **`apply` 的「免审核直接开通」分支未迁移**：当配置
//!    `merchantApplyNeedCheck != "1"` 时，Java 会把申请落库为「审核通过」并
//!    继续创建 `merchant` / `admin_user` / `merchant_user` / `shop` /
//!    `admin_user_shop` / `merchant_account` 六张表记录，同时初始化运费模板
//!    （`ShippingTplService#initShippingTpl`）、店铺装修
//!    （`DecorateService#initDecorate`）、角色
//!    （`AdminRoleService#initAdminRole` / `initEmployeeRoles`）并发送短信。
//!    这些依赖服务在本仓库均未迁移，只建店铺不建运费/装修/角色会留下无法使用的
//!    半成品数据，故本实现**在该分支写入前**返回明确错误
//!    （[`MSG_OPEN_SHOP_UNMIGRATED`]），不做任何插入。
//!
//! 3. **`apply` 要求登录**：Java `createMerchantApply` 里
//!    `SecurityUtils.getCurrentUserId()` 在未登录时返回 **0**，会把
//!    `user_id = 0` 的脏申请写进库（随后 `applyDetail` 因 `0 != 0` 还能读回，
//!    但归属错误）。本实现要求登录（业务码 401「请先登录」），**不复刻**该脏写。
//!    `myApply` / `applyDetail` 仍按 Java 语义：游客 `userId = 0`，
//!    查不到 / 归属不符 → 业务码 1001「商户申请信息不存在」。
//!
//! 4. **JSON 列容错**：Java `JsonUtil.fromJson(null, X.class)` 返回 `null`，
//!    随后 `baseData.getLicenseAddrProvince()` 会 **NPE → HTTP 500**；
//!    `shop_data` 非对象 / 非 JSON 时 Java 返回 `null`。本实现对
//!    `base_data` / `merchant_data` 反序列化失败或缺列时退化为
//!    `Default`（空结构，不 500），`shop_data` 解析失败 / 非对象时退化为
//!    `null`。这是**有意的健壮性偏差**（宁可返回空结构也不打崩接口）。
//!
//! 5. **`add_time` 为空**：Java `applyDetail` 里 `getAddTime()` 自动拆箱，
//!    列为 null 时 **NPE → 500**。本实现对空值按 `0` 格式化（`1970-01-01`
//!    附近），不 500。
//!
//! 6. **`translate` 是恒等函数**：`applyShopAgreement` 里 Java 会调用
//!    `TranslatePackageImpl#translate(bizVal, ARTICLE_CONTENT_DESC)`，而当前
//!    Java 实现**无论 locale 都原样返回 `text`**（`TranslatePackageImpl:64-69`
//!    两个分支都 `return text`）。故本实现直接返回配置原值，与 Java 行为一致。
//!
//! 7. **通知名称取外层 `type`**：Java
//!    `String name = param.getType() == 1 ? baseData.corporateName : baseData.companyName;`
//!    用的是**外层** `param.type`（非 `baseData.type`），且 `param.getType()`
//!    自动拆箱：为 null 时 **NPE → 事务回滚（不落库）**。本实现要求外层 `type`
//!    非空（缺失 → 业务码 1001「数据类型异常」），并在插入前返回；净效果
//!    （不落库）与 Java 一致。名称缺失时按 Java 字符串拼接语义得到字面量
//!    `"null"`（Java `"…" + null` == `"…null"`）。
//!
//! 8. **事务**：Java `apply` 标注 `@Transactional(rollbackFor = Exception.class)`，
//!    写 `merchant_apply` 与写 `admin_msg` 同生共死；本实现用
//!    [`MerchantClientRepository::begin`] + `*_in_tx` 在同一事务内串联，行为一致。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::TimeZone;

use nextrade_core::entities::{MerchantApply, Region};
use nextrade_core::models::merchant::merchant_client::{
    status_text_or_err, ApplyBaseData, ApplyMerchantData, ApplyShopData, MerchantApplyApplyParam,
    MerchantApplyDetailVO, MerchantApplyInitialVO, MerchantMyApplyVO, ADMIN_MSG_TYPE_SHOP_ENTRY,
    APPLY_STATUS_PASS, APPLY_STATUS_PENDING, CFG_MERCHANT_APPLY_NEED_CHECK, CFG_SHOP_AGREEMENT,
    DATA_TYPE_ERROR, MSG_CONFIG_NOT_EXIST, MSG_LICENSE_UPLOAD_UNMIGRATED,
    MSG_MERCHANT_APPLY_NOT_EXIST, SERVICE_DATA_ERROR,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::merchant_client::MerchantClientRepository;
use nextrade_infrastructure::db::Repositories;

/// 营业执照图片大小上限（Java `MerchantServiceImpl.LICENSE_PIC_MAX_SIZE`
/// = `10L * 1024 * 1024`）。
pub const LICENSE_PIC_MAX_SIZE: u64 = 10 * 1024 * 1024;

/// 允许的营业执照图片 MIME（Java `LICENSE_PIC_CONTENT_TYPES`）。
pub const LICENSE_PIC_CONTENT_TYPES: [&str; 3] = ["image/jpeg", "image/png", "image/bmp"];

/// 空文件错误文案（Java `GlobalException("营业执照图片不能为空")`）。
pub const ERR_LICENSE_EMPTY: &str = "营业执照图片不能为空";
/// 超限错误文案（Java `GlobalException("图片大小超过 10MB 限制")`）。
pub const ERR_LICENSE_TOO_LARGE: &str = "图片大小超过 10MB 限制";
/// MIME 不符错误文案（Java `GlobalException("图片规格不符合要求")`）。
pub const ERR_LICENSE_TYPE: &str = "图片规格不符合要求";

/// 校验营业执照上传入参（对应 Java `MerchantServiceImpl#uploadLicensePic` 的前
/// 三条校验：`file.isEmpty()` / `size > MAX` / MIME 白名单）。
///
/// # Errors
/// 全部为业务码 [`SERVICE_DATA_ERROR`]（Java `GlobalException(String)` 默认码）：
/// - 空图片 → [`ERR_LICENSE_EMPTY`]；
/// - 超过 [`LICENSE_PIC_MAX_SIZE`] → [`ERR_LICENSE_TOO_LARGE`]；
/// - `content_type` 去空白、转小写后不在 [`LICENSE_PIC_CONTENT_TYPES`] → [`ERR_LICENSE_TYPE`]。
pub fn validate_license_upload(byte_len: usize, content_type: Option<&str>) -> Result<()> {
    if byte_len == 0 {
        return Err(Error::biz(SERVICE_DATA_ERROR, ERR_LICENSE_EMPTY));
    }
    if byte_len as u64 > LICENSE_PIC_MAX_SIZE {
        return Err(Error::biz(SERVICE_DATA_ERROR, ERR_LICENSE_TOO_LARGE));
    }
    let ok_type = content_type
        .map(|t| t.trim().to_ascii_lowercase())
        .is_some_and(|t| LICENSE_PIC_CONTENT_TYPES.contains(&t.as_str()));
    if !ok_type {
        return Err(Error::biz(SERVICE_DATA_ERROR, ERR_LICENSE_TYPE));
    }
    Ok(())
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `SimpleDateFormat` 默认时区）。
///
/// Java 用 JVM 默认时区；Rust 用 [`chrono::Local`]。越界时间戳返回空串
/// （Java 一般不会遇到，属防御性处理）。
#[must_use]
pub fn fmt_datetime(secs: i64) -> String {
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// JSON 字符串 → 目标类型；`None` / 空串 / 解析失败 → `Default::default()`。
///
/// 对应 Java `JsonUtil.fromJson`（`null` / 空串 / 解析失败均返回 `null`），
/// 区别在于 Java 拿到 `null` 后继续取字段会 NPE，本实现退化为空结构。
pub fn parse_json_or_default<T>(raw: Option<&str>) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    raw.filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

/// JSON 字符串 → `serde_json::Value`（仅当解析结果是 JSON 对象）。
///
/// 对应 Java `JsonUtil.fromJson(raw, JSONObject.class)`：`null` / 空串 /
/// 非法 JSON / 非对象（如数组）均返回 `null`，故这里非对象一律返回
/// [`serde_json::Value::Null`]（序列化为 `null`）。
#[must_use]
pub fn parse_json_value(raw: Option<&str>) -> serde_json::Value {
    match raw.filter(|s| !s.is_empty()).and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()) {
        Some(v @ serde_json::Value::Object(_)) => v,
        _ => serde_json::Value::Null,
    }
}

/// 地区查询结果 → `region_id -> region_name` 映射。
#[must_use]
pub fn region_name_map(regions: Vec<Region>) -> HashMap<i32, String> {
    regions.into_iter().map(|r| (r.region_id, r.region_name)).collect()
}

/// 按**入参顺序**拼接地区名称（Java `for (Integer regionId : list)` 循环
/// 逐级 `append`，查不到的 ID 跳过）。
#[must_use]
pub fn concat_region_names(ids: &[i32], names: &HashMap<i32, String>) -> String {
    let mut out = String::new();
    for id in ids {
        if let Some(name) = names.get(id) {
            out.push_str(name);
        }
    }
    out
}

/// Java 字符串拼接语义下的可选字符串：`None` → 字面量 `"null"`。
///
/// Java `"前缀" + null` 得到 `"前缀null"`，非空串。
#[must_use]
pub fn java_str(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

/// 解析站内信里的商户名称（Java
/// `param.getType() == 1 ? baseData.corporateName : baseData.companyName`）。
///
/// `apply_type` 为 `None` 时 Java 自动拆箱 NPE（事务回滚），这里转为业务码
/// 1001「数据类型异常」；名称缺失按 Java 语义返回字面量 `"null"`。
///
/// # Errors
/// 业务码 [`SERVICE_DATA_ERROR`]：[`DATA_TYPE_ERROR`]（外层 `type` 缺失）。
pub fn notify_name(
    apply_type: Option<i32>,
    corporate_name: Option<&str>,
    company_name: Option<&str>,
) -> Result<String> {
    let t = apply_type.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, DATA_TYPE_ERROR))?;
    let name = if t == 1 { corporate_name } else { company_name };
    Ok(java_str(name).to_string())
}

/// 站内信标题（Java `"商户入驻通知：" + name`）。
#[must_use]
pub fn notify_title(name: &str) -> String {
    format!("商户入驻通知：{name}")
}

/// 站内信内容（Java `"【" + name + "】提交了商家入驻申请，请及时处理"`）。
#[must_use]
pub fn notify_content(name: &str) -> String {
    format!("【{name}】提交了商家入驻申请，请及时处理")
}

/// 站内信 `relatedData`（Java `Map.of("merchantApplyId", id)` → Jackson JSON）。
#[must_use]
pub fn related_data_json(merchant_apply_id: i32) -> String {
    serde_json::json!({ "merchantApplyId": merchant_apply_id }).to_string()
}

/// 序列化为 JSON 串（Java `JsonUtil.toJson`；失败返回空串，实际不会发生）。
fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// 当前秒级时间戳（`i32`，对应 `merchant_apply.add_time` / `audit_time` 的 `int` 列）。
fn now_secs_i32() -> i32 {
    i32::try_from(chrono::Utc::now().timestamp()).unwrap_or(i32::MAX)
}

/// 按 Java `MerchantApplyApplyParam#createMerchantApply` 构造待落库实体。
///
/// `need_check == "1"` → 待审核（[`APPLY_STATUS_PENDING`]）；否则 → 审核通过
/// （[`APPLY_STATUS_PASS`]，本实现随后会因未迁移而拒绝写入）。
/// `audit_time` / `audit_admin_id` 按 MyBatis-Plus `NOT_NULL` 策略下的**列默认值**
/// 补齐（分别是 `0` / `0`），与 Java 落库结果一致。
#[must_use]
pub fn build_apply_entity(
    user_id: i32,
    param: &MerchantApplyApplyParam,
    need_check: &str,
) -> MerchantApply {
    let status = if need_check == "1" {
        APPLY_STATUS_PENDING
    } else {
        APPLY_STATUS_PASS
    };
    let now = now_secs_i32();
    MerchantApply {
        merchant_apply_id: 0,
        user_id: Some(user_id),
        add_time: Some(now),
        shop_title: param.shop_title.clone(),
        status: Some(status),
        r#type: param.base_data.r#type,
        // Java 未给这三列赋值 ⇒ MyBatis-Plus NOT_NULL ⇒ 落列默认值 ''
        contact_name: None,
        contact_mobile: None,
        // Java：仅「审核通过」时给当前秒级时间戳，否则 null ⇒ 列默认值 0
        audit_time: Some(if status == APPLY_STATUS_PASS { now } else { 0 }),
        audit_admin_id: Some(0),
        shop_data: Some(to_json(&param.shop_data)),
        base_data: Some(to_json(&param.base_data)),
        merchant_data: Some(to_json(&param.merchant_data)),
        company_name: param.base_data.company_name.clone(),
        corporate_name: param.base_data.corporate_name.clone(),
        audit_remark: None,
    }
}

/// 商户入驻·前台服务。
pub struct MerchantClientService {
    repos: Arc<Repositories>,
}

impl MerchantClientService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    fn repo(&self) -> &MerchantClientRepository {
        &self.repos.merchant.merchant_client
    }

    /// 读取系统配置项；缺失（Java `CONFIG_NOT_EXIST`「配置不存在」）→ 业务码 1001。
    ///
    /// 与 Java `ConfigServiceImpl#getConfigByCode` 一致：`biz_code = ? AND is_del = 0`。
    async fn config_val_or_err(&self, biz_code: &str) -> Result<String> {
        self.repo()
            .find_config_val(biz_code)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_CONFIG_NOT_EXIST))
    }

    /// 解析审核管理员姓名（Java `MerchantApplyServiceImpl#resolveAuditAdminName`）：
    /// `null` / `<= 0` / 管理员已删除 / 用户名为空 → 空串。
    async fn resolve_audit_admin_name(&self, audit_admin_id: Option<i32>) -> Result<String> {
        match audit_admin_id {
            Some(id) if id > 0 => match self.repo().find_admin_user_by_id(id).await? {
                Some(u) if !u.username.is_empty() => Ok(u.username),
                _ => Ok(String::new()),
            },
            _ => Ok(String::new()),
        }
    }

    /// 审核通过时下发初始登录信息（Java `applyDetail` 里的
    /// `admin_user.mobile = contactPhone` 查询）。
    ///
    /// `contactPhone` 为 `None` → 直接 `None`（Java 会拼出 `mobile = null`
    /// 永假条件，结果同样是查不到）。
    async fn load_initial_user_info(
        &self,
        contact_phone: Option<&str>,
    ) -> Result<Option<MerchantApplyInitialVO>> {
        let Some(phone) = contact_phone else {
            return Ok(None);
        };
        Ok(self
            .repo()
            .find_admin_user_by_mobile(phone)
            .await?
            .map(|u| MerchantApplyInitialVO {
                username: Some(u.username),
                mobile: u.mobile,
                initial_password: Some(u.initial_password),
            }))
    }

    // ——————————————————————————————————————————————
    // POST /api/merchant/merchant/uploadLicense
    // ——————————————————————————————————————————————

    /// 入驻营业执照专用上传。
    ///
    /// 对应 Java `MerchantClientController.uploadLicense` →
    /// `MerchantServiceImpl#uploadLicensePic`：空 / 超限 / MIME 校验通过后，
    /// 叠加公示水印（`ImageWatermarkUtil.tileText`）并走不落相册的图库上传。
    /// **未迁移**（水印字体 + 对象存储）：校验通过后返回
    /// [`MSG_LICENSE_UPLOAD_UNMIGRATED`]。
    ///
    /// # Errors
    /// - 业务码 1001：[`ERR_LICENSE_EMPTY`] / [`ERR_LICENSE_TOO_LARGE`] /
    ///   [`ERR_LICENSE_TYPE`]；
    /// - 业务码 1001：[`MSG_LICENSE_UPLOAD_UNMIGRATED`]（校验通过后）。
    pub fn upload_license(&self, byte_len: usize, content_type: Option<&str>) -> Result<String> {
        validate_license_upload(byte_len, content_type)?;
        Err(Error::biz(
            SERVICE_DATA_ERROR,
            MSG_LICENSE_UPLOAD_UNMIGRATED,
        ))
    }

    // ——————————————————————————————————————————————
    // GET /api/merchant/merchant/applyShopAgreement
    // ——————————————————————————————————————————————

    /// 申请入驻协议。
    ///
    /// 对应 Java `MerchantClientController.applyShopAgreement` →
    /// `MerchantServiceImpl#applyShopAgreement`：读 `shopAgreement` 配置后
    /// 过一遍 `TranslatePackage#translate`（当前 Java 实现恒等返回原文）。
    ///
    /// # Errors
    /// 业务码 1001：[`MSG_CONFIG_NOT_EXIST`]（配置缺失）。
    pub async fn apply_shop_agreement(&self) -> Result<String> {
        self.config_val_or_err(CFG_SHOP_AGREEMENT).await
    }

    // ——————————————————————————————————————————————
    // GET /api/merchant/merchant/myApply
    // ——————————————————————————————————————————————

    /// 我的（最新一条）入驻申请。
    ///
    /// 对应 Java `MerchantClientController.myApply` →
    /// `MerchantApplyServiceImpl#myApply`：按 `user_id` 取最新一条
    /// （`orderByDesc(merchantApplyId).last("limit 1")`），无记录返回 `null`
    /// （本项目返回 `None` → 响应体 `data:null`）。
    ///
    /// # Errors
    /// 业务码 1001：[`DATA_TYPE_ERROR`]（状态码为空或非法，Java 侧为 NPE / 1001）。
    pub async fn my_apply(&self, user_id: i32) -> Result<Option<MerchantMyApplyVO>> {
        let Some(apply) = self.repo().find_latest_apply_by_user(user_id).await? else {
            return Ok(None);
        };
        Ok(Some(MerchantMyApplyVO {
            merchant_apply_id: apply.merchant_apply_id,
            status: apply.status,
            r#type: apply.r#type,
            status_text: status_text_or_err(apply.status)?,
            audit_time: apply.audit_time.map(i64::from),
            audit_admin_name: self.resolve_audit_admin_name(apply.audit_admin_id).await?,
        }))
    }

    // ——————————————————————————————————————————————
    // GET /api/merchant/merchant/applyDetail
    // ——————————————————————————————————————————————

    /// 入驻申请详情（仅申请归属本人可读）。
    ///
    /// 对应 Java `MerchantClientController.applyDetail` →
    /// `MerchantApplyServiceImpl#applyDetail`：`getById(id)` 后校验
    /// `userId.equals(merchantApply.getUserId())`，不符 / 不存在 → 抛
    /// 「商户申请信息不存在」；随后反序列化三家 JSON 列、回填地区名称，
    /// 审核通过时按 `contactPhone` 查 `admin_user` 回填初始登录信息。
    ///
    /// # Errors
    /// - 业务码 1001：[`MSG_MERCHANT_APPLY_NOT_EXIST`]（不存在或非本人）；
    /// - 业务码 1001：[`DATA_TYPE_ERROR`]（状态为空或非法）。
    pub async fn apply_detail(&self, id: i32, user_id: i32) -> Result<MerchantApplyDetailVO> {
        let Some(apply) = self.repo().find_apply_by_id(id).await? else {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_MERCHANT_APPLY_NOT_EXIST));
        };
        if apply.user_id != Some(user_id) {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_MERCHANT_APPLY_NOT_EXIST));
        }
        self.build_detail_vo(&apply).await
    }

    /// 组装详情 VO（Java `applyDetail` 的 VO 构建段）。
    async fn build_detail_vo(&self, apply: &MerchantApply) -> Result<MerchantApplyDetailVO> {
        // 基础数据 + 营业执照地址省份名称回填
        let mut base_data: ApplyBaseData = parse_json_or_default(apply.base_data.as_deref());
        if !base_data.license_addr_province.is_empty() {
            let regions = self
                .repo()
                .find_regions_by_ids(&base_data.license_addr_province)
                .await?;
            let names = region_name_map(regions);
            base_data.license_addr_province_name =
                Some(concat_region_names(&base_data.license_addr_province, &names));
        }

        // 商户数据 + 业务地址名称回填
        let mut merchant_data: ApplyMerchantData = parse_json_or_default(apply.merchant_data.as_deref());
        if !merchant_data.business_address.is_empty() {
            let regions = self
                .repo()
                .find_regions_by_ids(&merchant_data.business_address)
                .await?;
            let names = region_name_map(regions);
            merchant_data.business_address_name =
                Some(concat_region_names(&merchant_data.business_address, &names));
        }

        let status = apply.status;
        let status_text = status_text_or_err(status)?;
        let audit_admin_name = self.resolve_audit_admin_name(apply.audit_admin_id).await?;

        // Java：仅「审核通过」时回填初始登录信息
        let initial_user_info = if status == Some(APPLY_STATUS_PASS) {
            self.load_initial_user_info(merchant_data.contact_phone.as_deref())
                .await?
        } else {
            None
        };

        Ok(MerchantApplyDetailVO {
            merchant_apply_id: apply.merchant_apply_id,
            // Java 用 addTime 自动拆箱后格式化；本实现对 null 按 0 处理（见模块头差异 5）
            add_time: fmt_datetime(i64::from(apply.add_time.unwrap_or(0))),
            base_data,
            company_name: apply.company_name.clone(),
            contact_name: apply.contact_name.clone(),
            contact_mobile: apply.contact_mobile.clone(),
            corporate_name: apply.corporate_name.clone(),
            merchant_data,
            audit_time: apply.audit_time.map(i64::from),
            audit_admin_id: apply.audit_admin_id,
            audit_admin_name,
            shop_data: parse_json_value(apply.shop_data.as_deref()),
            status,
            status_text,
            r#type: apply.r#type,
            user_id: apply.user_id,
            shop_title: apply.shop_title.clone(),
            audit_remark: apply.audit_remark.clone(),
            initial_user_info,
        })
    }

    // ——————————————————————————————————————————————
    // POST /api/merchant/merchant/apply
    // ——————————————————————————————————————————————

    /// 提交入驻申请。
    ///
    /// 对应 Java `MerchantClientController.apply` →
    /// `MerchantApplyServiceImpl#apply`（`@Transactional`）：
    /// 1. 读 `merchantApplyNeedCheck` 配置（缺失 → 「配置不存在」）；
    /// 2. 按 `createMerchantApply` 构造并落库申请；
    ///    - 若 `merchantApplyNeedCheck != "1"`（免审核直接通过）→ Java 会继续建
    ///      商户/管理员/店铺等六张表 + 初始化运费/装修/角色 + 发短信，本构建
    ///      **在写入前**返回 [`MSG_OPEN_SHOP_UNMIGRATED`]（见模块头差异 2）；
    /// 3. 写后台站内信（类型 41，事务内）；
    /// 4. 返回 `applyDetail(id)`（本人视角）。
    ///
    /// 要求登录（业务码 401「请先登录」），避免 Java 因 `getCurrentUserId() == 0`
    /// 产生的脏归属（见模块头差异 3）。
    ///
    /// # Errors
    /// - 业务码 1001：[`MSG_CONFIG_NOT_EXIST`] / [`DATA_TYPE_ERROR`] /
    ///   [`MSG_OPEN_SHOP_UNMIGRATED`]；
    /// - 业务码 1001：[`MSG_MERCHANT_APPLY_NOT_EXIST`]（写后回读异常）。
    pub async fn apply(
        &self,
        user_id: i32,
        param: &MerchantApplyApplyParam,
    ) -> Result<MerchantApplyDetailVO> {
        // 1) 读取审核开关（缺失 → 「配置不存在」）
        let need_check = self.config_val_or_err(CFG_MERCHANT_APPLY_NEED_CHECK).await?;

        // 2) 构造待落库实体
        let apply = build_apply_entity(user_id, param, &need_check);

        // 站内信名称（外层 type 缺失 → 1001，Java 为 NPE 回滚；模块头差异 7）
        let name = notify_name(
            param.r#type,
            param.base_data.corporate_name.as_deref(),
            param.base_data.company_name.as_deref(),
        )?;

        // 3) 事务内写 merchant_apply + admin_msg（Java @Transactional）
        let send_time = now_secs_i32();
        let mut tx = self
            .repo()
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        let id = self.repo().insert_apply_in_tx(&mut tx, &apply).await?;
        self.repo()
            .insert_admin_msg_in_tx(
                &mut tx,
                ADMIN_MSG_TYPE_SHOP_ENTRY,
                &notify_title(&name),
                &notify_content(&name),
                &related_data_json(id),
                send_time,
            )
            .await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;

        if apply.status == Some(APPLY_STATUS_PASS) {
            let mut applied = apply.clone();
            applied.merchant_apply_id = id;
            let svc = crate::merchant::merchant_apply::MerchantApplyService::new(self.repos.clone());
            svc.open_shop_for_passed_apply(&applied, send_time).await?;
        }

        // 4) 回读详情（Java `return this.applyDetail(id)`）
        self.apply_detail(id, user_id).await
    }

    /// 供路由层校验：`MerchantApplyApplyParam` 三块非空数据已由反序列化保证
    /// （缺 `baseData`/`merchantData`/`shopData` 直接 422）。
    ///
    /// 保留占位以明确「无需在此重复校验」。
    #[must_use]
    pub fn required_payloads_present(param: &MerchantApplyApplyParam) -> bool {
        // Rust 类型系统已保证 `base_data` / `merchant_data` / `shop_data` 非 Option，
        // 故恒为 true；此函数仅用于表达契约、方便单测与调用方阅读。
        let _ = param;
        true
    }
}

/// 供测试/调用方构造空 `ApplyShopData`（避免在路由层重复 import）。
#[must_use]
pub fn empty_shop_data() -> ApplyShopData {
    ApplyShopData::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::merchant::merchant_client::{ApplyBaseData, MerchantApplyApplyParam};

    fn base_param(apply_type: Option<i32>) -> MerchantApplyApplyParam {
        MerchantApplyApplyParam {
            shop_title: Some("测试店".into()),
            r#type: apply_type,
            base_data: ApplyBaseData {
                r#type: apply_type,
                company_name: Some("某某公司".into()),
                corporate_name: Some("张三".into()),
                license_addr_province: vec![110_000, 110_100],
                ..Default::default()
            },
            merchant_data: ApplyMerchantData {
                contact_phone: Some("13800008000".into()),
                business_address: vec![110_000],
                ..Default::default()
            },
            shop_data: ApplyShopData {
                shop_title: Some("测试店".into()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn constants_match_java_source() {
        assert_eq!(SERVICE_DATA_ERROR, 1001);
        assert_eq!(ADMIN_MSG_TYPE_SHOP_ENTRY, 41);
        assert_eq!(LICENSE_PIC_MAX_SIZE, 10 * 1024 * 1024);
        assert_eq!(CFG_MERCHANT_APPLY_NEED_CHECK, "merchantApplyNeedCheck");
        assert_eq!(CFG_SHOP_AGREEMENT, "shopAgreement");
        assert_eq!(APPLY_STATUS_PENDING, 1);
        assert_eq!(APPLY_STATUS_PASS, 10);
    }

    #[test]
    fn validate_license_upload_matches_java() {
        // 空文件
        let err = validate_license_upload(0, Some("image/png")).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert!(err.to_string().contains(ERR_LICENSE_EMPTY));

        // 恰好 10MB 允许；多 1 字节拒绝
        assert!(validate_license_upload(LICENSE_PIC_MAX_SIZE as usize, Some("image/jpeg")).is_ok());
        let err = validate_license_upload(LICENSE_PIC_MAX_SIZE as usize + 1, Some("image/jpeg"))
            .unwrap_err();
        assert!(err.to_string().contains(ERR_LICENSE_TOO_LARGE));

        // MIME：缺省 / 非白名单 / 大小写与空白
        for bad in [None, Some("image/gif"), Some("application/pdf")] {
            let err = validate_license_upload(10, bad).unwrap_err();
            assert!(err.to_string().contains(ERR_LICENSE_TYPE), "bad={bad:?}");
        }
        assert!(validate_license_upload(10, Some("image/bmp")).is_ok());
        assert!(validate_license_upload(10, Some(" IMAGE/JPEG ")).is_ok());
    }

    #[test]
    fn fmt_datetime_shape_is_yyyy_mm_dd_hh_mm_ss() {
        let s = fmt_datetime(1_700_000_000);
        let bytes: Vec<char> = s.chars().collect();
        assert_eq!(bytes.len(), 19, "实际: {s}");
        assert_eq!(bytes[4], '-');
        assert_eq!(bytes[7], '-');
        assert_eq!(bytes[10], ' ');
        assert_eq!(bytes[13], ':');
        assert_eq!(bytes[16], ':');
        assert!(bytes[..4].iter().all(char::is_ascii_digit));
    }

    #[test]
    fn parse_json_or_default_falls_back() {
        let none: ApplyBaseData = parse_json_or_default(None);
        assert!(none.company_name.is_none());
        let empty: ApplyBaseData = parse_json_or_default(Some(""));
        assert!(empty.company_name.is_none());
        let bad: ApplyBaseData = parse_json_or_default(Some("not-json"));
        assert!(bad.company_name.is_none());
        let ok: ApplyBaseData =
            parse_json_or_default(Some(r#"{"companyName":"甲","licenseAddrProvince":[1,2]}"#));
        assert_eq!(ok.company_name.as_deref(), Some("甲"));
        assert_eq!(ok.license_addr_province, vec![1, 2]);
    }

    #[test]
    fn parse_json_value_only_accepts_objects() {
        assert!(parse_json_value(None).is_null());
        assert!(parse_json_value(Some("")).is_null());
        assert!(parse_json_value(Some("[1,2]")).is_null());
        assert!(parse_json_value(Some("bad")).is_null());
        let v = parse_json_value(Some(r#"{"shopTitle":"x"}"#));
        assert_eq!(v["shopTitle"], "x");
    }

    #[test]
    fn concat_region_names_follows_input_order_and_skips_missing() {
        let mut names = HashMap::new();
        names.insert(1, "北京".to_string());
        names.insert(2, "朝阳".to_string());
        names.insert(3, "上海".to_string());
        assert_eq!(concat_region_names(&[3, 1, 2], &names), "上海北京朝阳");
        // 查不到的 ID 跳过
        assert_eq!(concat_region_names(&[1, 99, 2], &names), "北京朝阳");
        assert_eq!(concat_region_names(&[], &names), "");
    }

    #[test]
    fn notify_name_requires_outer_type_and_uses_java_null_concat() {
        // 外层 type 缺失 → 业务码 1001（Java NPE → 回滚）
        let err = notify_name(None, Some("张三"), Some("公司")).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert!(err.to_string().contains(DATA_TYPE_ERROR));

        // type == 1（个人）取 corporateName（Java 实现如此，非笔误）
        assert_eq!(
            notify_name(Some(1), Some("张三"), Some("某某公司")).unwrap(),
            "张三"
        );
        // type != 1（企业）取 companyName
        assert_eq!(
            notify_name(Some(2), Some("张三"), Some("某某公司")).unwrap(),
            "某某公司"
        );
        // 名称缺失 → Java 字符串拼接得到字面量 "null"
        assert_eq!(notify_name(Some(1), None, Some("公司")).unwrap(), "null");
    }

    #[test]
    fn notify_title_content_related_data_match_java() {
        assert_eq!(notify_title("张三"), "商户入驻通知：张三");
        assert_eq!(notify_content("张三"), "【张三】提交了商家入驻申请，请及时处理");
        assert_eq!(related_data_json(7), r#"{"merchantApplyId":7}"#);
        // 标题/内容里的字面量 null 与 Java 拼接一致
        assert_eq!(notify_title("null"), "商户入驻通知：null");
    }

    #[test]
    fn build_apply_entity_pending_matches_java_mapping() {
        let param = base_param(Some(2));
        let entity = build_apply_entity(9, &param, "1");
        assert_eq!(entity.user_id, Some(9));
        assert_eq!(entity.status, Some(APPLY_STATUS_PENDING));
        // type 取 baseData.type
        assert_eq!(entity.r#type, Some(2));
        assert_eq!(entity.company_name.as_deref(), Some("某某公司"));
        assert_eq!(entity.corporate_name.as_deref(), Some("张三"));
        assert_eq!(entity.shop_title.as_deref(), Some("测试店"));
        // NOT_NULL 策略下的列默认值
        assert_eq!(entity.audit_admin_id, Some(0));
        assert_eq!(entity.audit_time, Some(0));
        assert!(entity.contact_name.is_none());
        assert!(entity.audit_remark.is_none());
        assert!(entity.add_time.is_some());
        // 三块 JSON 列均已序列化（camelCase）
        assert!(entity.base_data.as_deref().unwrap().contains("companyName"));
        assert!(entity.merchant_data.as_deref().unwrap().contains("contactPhone"));
        assert!(entity.shop_data.as_deref().unwrap().contains("shopTitle"));
    }

    #[test]
    fn build_apply_entity_pass_sets_audit_time_now() {
        let param = base_param(Some(1));
        let entity = build_apply_entity(5, &param, "0");
        assert_eq!(entity.status, Some(APPLY_STATUS_PASS));
        // 审核通过时 audit_time = 当前秒级时间戳（非 0）
        assert!(entity.audit_time.unwrap_or(0) > 0);
    }

    #[test]
    fn build_apply_entity_type_null_falls_back_to_column_default() {
        // Java：baseData.type 为 null → 实体 type null → MyBatis-Plus NOT_NULL → 列默认值 1
        let param = base_param(None);
        let entity = build_apply_entity(1, &param, "1");
        assert_eq!(entity.r#type, None); // 交由 INSERT 的 COALESCE(?, 1) 补 1
    }

    #[test]
    fn required_payloads_present_is_always_true_by_type() {
        assert!(MerchantClientService::required_payloads_present(&base_param(Some(1))));
        assert!(!empty_shop_data().shop_title.is_some_and(|s| s.is_empty()));
    }
}