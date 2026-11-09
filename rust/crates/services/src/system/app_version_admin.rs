//! app 版本管理服务（system::app_version_admin）：版本详情、版本更新、更新包上传。
//!
//! 对应原 Java `AppVersionServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/setting/impl/AppVersionServiceImpl.java`）
//! 中由 `AppVersionController`（`@RequestMapping("/adminapi/setting/appVersion")`）暴露的三条链路：
//!
//! | 本模块方法 | 原 Java | 说明 |
//! |-----------|--------|------|
//! | [`AppVersionAdminService::detail`] | `AppVersionServiceImpl#getDetail` | 读 `config` 表 4 个配置项 |
//! | [`AppVersionAdminService::update`] | `AppVersionServiceImpl#update` | 版本号格式校验 + 整体替换配置项 |
//! | [`AppVersionAdminService::upload_file`] | `AppVersionServiceImpl#upload` | **未迁移**：对象存储 |
//!
//! ## 配置项（`config.biz_code`，来自 Java `SettingsEnum`）
//!
//! `androidVersion`（`ANDROID_VERSION`）、`iosVersion`（`IOS_VERSION`）、
//! `iosLink`（`IOS_LINK`）、`androidLink`（`ANDROID_LINK`）。
//! Java `getDetail` 用 `ConfigServiceImpl#getConfigMapBySettings` 取这 4 项并组装
//! `AppVersionDetailVO`；`update` 走 `reflectConfigs(AppVersionEditParam)`
//! （该 param 无 `@ConfigIgnoreField` / `@ConfigInnerParam` 注解，故字段名即 `biz_code`），
//! 先按 `biz_code` 逻辑删除旧行，再批量插入新行。
//!
//! ## 与原 Java 的已知差异（详见 `uncertain_points`）
//!
//! 1. **空值处理**：Java `reflectConfigs` 用 `String.valueOf(bizVal)`，字段为 `null`
//!    时会写入字面量字符串 `"null"`（Java 缺陷：详情接口随后返回 `"null"`）。
//!    本实现改为**跳过 `null` 字段**（不插入该行，旧行仍被删除），
//!    即「未提交的字段」等价于「空配置」。
//! 2. **操作日志**：Java `update` 会调用 `adminLogService.save(...)` 写 `admin_log`。
//!    本移植版与其余切片一致，**不写** `admin_log`（全仓尚无写入该表的仓储能力）。
//! 3. **错误码**：Java `GlobalException(msg)` 携带 `code = 1001`；本移植版沿用项目
//!    既有映射（`Error::biz_msg` → 500），仅保证消息文案逐字一致。
//! 4. **配置缓存**：Java 侧 `SettingsHelper` 有 Redis 缓存与失效广播，
//!    `update` 这条链路本身也未做缓存失效（直接操作 `config` 表）；本移植版无缓存层，
//!    行为一致。
//!
//! ## 未迁移
//!
//! `upload`（`POST /adminapi/setting/appVersion/uploadFile`）依赖本地存储 /
//! 阿里云 OSS / 腾讯云 COS / 七牛 / AWS S3 五套对象存储服务（Java
//! `LocalStorageService`、`OssService`、`CosService`、`QiniuService`、`AwsS3Service`），
//! 本移植版未接通任何对象存储，返回明确业务错误而**不伪造**上传结果。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::system::app_version_admin::{
    AppVersionDetailVO, AppVersionEditParam, AppVersionParam,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// `SettingsEnum.ANDROID_VERSION.getBizCode()`。
pub const BIZ_CODE_ANDROID_VERSION: &str = "androidVersion";
/// `SettingsEnum.IOS_VERSION.getBizCode()`。
pub const BIZ_CODE_IOS_VERSION: &str = "iosVersion";
/// `SettingsEnum.IOS_LINK.getBizCode()`。
pub const BIZ_CODE_IOS_LINK: &str = "iosLink";
/// `SettingsEnum.ANDROID_LINK.getBizCode()`。
pub const BIZ_CODE_ANDROID_LINK: &str = "androidLink";

/// `getDetail` 读取的配置项（Java `List.of(ANDROID_VERSION, IOS_VERSION, ANDROID_LINK, IOS_LINK)`）。
const DETAIL_BIZ_CODES: [&str; 4] = [
    BIZ_CODE_ANDROID_VERSION,
    BIZ_CODE_IOS_VERSION,
    BIZ_CODE_ANDROID_LINK,
    BIZ_CODE_IOS_LINK,
];

/// 版本号格式错误文案（Java `GlobalException("版本号格式不正确，应为 x.y.z，例如 1.0.0")`）。
pub const ERR_VERSION_FORMAT: &str = "版本号格式不正确，应为 x.y.z，例如 1.0.0";

/// 管理员用户名缺失时的兜底值（Java `MybatisPlusMetaObjectHandler`：`"unknown"`）。
pub const UNKNOWN_ADMIN_NAME: &str = "unknown";

// ——————————————————————————————————————————————————————————————
// 详情组装
// ——————————————————————————————————————————————————————————————

/// 由 `config` 配置映射组装详情 VO
/// （Java `AppVersionDetailVO(Map<String, String>)` 构造器：按 `biz_code` 取值，
/// 取不到即 `null`）。
fn build_detail(map: &HashMap<String, String>) -> AppVersionDetailVO {
    AppVersionDetailVO {
        android_version: map.get(BIZ_CODE_ANDROID_VERSION).cloned(),
        ios_version: map.get(BIZ_CODE_IOS_VERSION).cloned(),
        ios_link: map.get(BIZ_CODE_IOS_LINK).cloned(),
        android_link: map.get(BIZ_CODE_ANDROID_LINK).cloned(),
    }
}

// ——————————————————————————————————————————————————————————————
// 纯函数（可单测）
// ——————————————————————————————————————————————————————————————

/// 校验版本号是否为 `x.y.z` 三段纯数字
/// （Java `AppVersionServiceImpl#isValidVersionFormat`：`^\d+\.\d+\.\d+$` 整串匹配）。
///
/// Java `String.matches` 为**整串匹配**，`\d` 默认仅匹配 ASCII 数字；
/// 此处逐段校验，语义等价（允许前导零，如 `01.2.3`）。
#[must_use]
pub fn is_valid_version_format(version: Option<&str>) -> bool {
    let Some(version) = version else {
        return false;
    };
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// 版本号格式校验，失败返回 Java 同文案的业务错误
/// （Java `update` 内 `throw new GlobalException(...)`；Android / iOS 复用同一文案）。
pub fn validate_version(version: Option<&str>) -> Result<()> {
    if is_valid_version_format(version) {
        Ok(())
    } else {
        Err(Error::biz_msg(ERR_VERSION_FORMAT))
    }
}

/// 把编辑参数摊平成待写入的 `config` 键值对（Java `reflectConfigs(AppVersionEditParam)`）。
///
/// 顺序与 Java 字段声明顺序一致：`androidVersion` / `iosVersion` / `iosLink` / `androidLink`。
/// **差异**：Java 对 `null` 字段写入字面量 `"null"`，此处跳过（见模块头注释第 1 条）；
/// 空字符串仍然保留（Java 中 `String.valueOf("") == ""`）。
#[must_use]
pub fn build_config_entries(param: &AppVersionEditParam) -> Vec<(String, String)> {
    [
        (BIZ_CODE_ANDROID_VERSION, param.android_version.as_ref()),
        (BIZ_CODE_IOS_VERSION, param.ios_version.as_ref()),
        (BIZ_CODE_IOS_LINK, param.ios_link.as_ref()),
        (BIZ_CODE_ANDROID_LINK, param.android_link.as_ref()),
    ]
    .into_iter()
    .filter_map(|(code, val)| val.map(|v| (code.to_string(), v.clone())))
    .collect()
}

/// 管理员用户名兜底（Java `MybatisPlusMetaObjectHandler`：
/// `StrUtil.isNotBlank(username) ? username : "unknown"`）。
#[must_use]
pub fn normalize_admin_name(admin_name: &str) -> String {
    let trimmed = admin_name.trim();
    if trimmed.is_empty() {
        UNKNOWN_ADMIN_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}

/// 校验上传文件非空（Java `MultipartFile.isEmpty()` / 缺少 `file` 参数时的 400）。
pub fn validate_upload_file(file_name: Option<&str>, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Err(Error::Validation("上传文件不能为空".to_string()));
    }
    if file_name.is_none_or(|n| n.trim().is_empty()) {
        return Err(Error::Validation("上传文件名不能为空".to_string()));
    }
    Ok(())
}

/// 上传 APP 更新包（Java `AppVersionServiceImpl#upload`）。
///
/// 校验文件非空后写入本地存储并返回 URL。
pub fn upload_app_file(file_name: Option<&str>, bytes: &[u8]) -> Result<String> {
    validate_upload_file(file_name, bytes)?;
    let original = file_name.unwrap_or("app-release.apk");
    nextrade_infrastructure::storage::save_local_upload_blocking("app", original, bytes)
}

// ——————————————————————————————————————————————————————————————
// 服务
// ——————————————————————————————————————————————————————————————

pub struct AppVersionAdminService {
    repos: Arc<Repositories>,
}

impl AppVersionAdminService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 版本详情（Java `AppVersionServiceImpl#getDetail`）。
    ///
    /// Java 返回 `AppVersionDetailVO`（`config` 中缺失项为 `null`），不抛错。
    pub async fn detail(&self) -> Result<AppVersionDetailVO> {
        let map = self
            .repos.system.app_version_admin
            .find_config_vals(&DETAIL_BIZ_CODES)
            .await?;
        Ok(build_detail(&map))
    }

    /// 更新版本配置（Java `AppVersionServiceImpl#update`）。
    ///
    /// 步骤与 Java 一致：
    /// 1. 校验 `androidVersion` / `iosVersion` 为 `x.y.z`；
    /// 2. 摊平参数为 `config` 键值对；
    /// 3. 按 `biz_code` 逻辑删除旧行并插入新行（单事务）。
    ///
    /// `admin_id` / `admin_name` 用于填充审计字段（Java 从 `SecurityUtils` 取当前管理员）；
    /// Java 还会写 `admin_log`，本移植版不写（见模块头注释第 2 条）。
    pub async fn update(
        &self,
        param: &AppVersionEditParam,
        admin_id: Option<i64>,
        admin_name: &str,
    ) -> Result<()> {
        validate_version(param.android_version.as_deref())?;
        validate_version(param.ios_version.as_deref())?;

        let entries = build_config_entries(param);
        let name = normalize_admin_name(admin_name);
        self.repos.system.app_version_admin
            .replace_configs(&entries, admin_id, &name)
            .await?;

        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = admin_log.record_current_log("admin", 0, "更新APP版本管理").await {
            tracing::warn!("写管理操作日志失败（APP 版本）: {e}");
        }
        Ok(())
    }

    /// 上传更新文件（Java `AppVersionServiceImpl#upload`）。**未迁移**：对象存储。
    pub fn upload_file(&self, file_name: Option<&str>, bytes: &[u8]) -> Result<String> {
        upload_app_file(file_name, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ————————————————— 版本号格式 —————————————————

    #[test]
    fn version_format_accepts_three_numeric_segments() {
        assert!(is_valid_version_format(Some("1.0.0")));
        assert!(is_valid_version_format(Some("10.20.300")));
        // Java `\d+` 允许前导零
        assert!(is_valid_version_format(Some("01.2.3")));
    }

    #[test]
    fn version_format_rejects_malformed() {
        for bad in [
            "1.0",
            "1.0.0.0",
            "1.0.0-beta",
            "",
            "1..0",
            ".1.0",
            "1.0.",
            "v1.0.0",
            "1.0.0 ",
        ] {
            assert!(
                !is_valid_version_format(Some(bad)),
                "应判为非法版本号: {bad:?}"
            );
        }
        assert!(!is_valid_version_format(None));
    }

    #[test]
    fn validate_version_uses_java_message() {
        let err = validate_version(Some("1.0")).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("业务错误 [500]: {}", ERR_VERSION_FORMAT)
        );
        assert!(validate_version(Some("2.3.4")).is_ok());
        assert!(validate_version(None).is_err());
    }

    // ————————————————— 参数摊平 —————————————————

    #[test]
    fn entries_keep_declaration_order_and_empty_strings() {
        let param = AppVersionEditParam {
            android_version: Some("1.0.1".into()),
            ios_version: Some("1.0.2".into()),
            ios_link: Some(String::new()),
            android_link: Some("https://cdn.example.com/app/update/1.0.1.apk".into()),
        };
        let entries = build_config_entries(&param);
        assert_eq!(
            entries,
            vec![
                (BIZ_CODE_ANDROID_VERSION.to_string(), "1.0.1".to_string()),
                (BIZ_CODE_IOS_VERSION.to_string(), "1.0.2".to_string()),
                (BIZ_CODE_IOS_LINK.to_string(), String::new()),
                (
                    BIZ_CODE_ANDROID_LINK.to_string(),
                    "https://cdn.example.com/app/update/1.0.1.apk".to_string()
                ),
            ]
        );
    }

    #[test]
    fn entries_skip_null_fields() {
        let param = AppVersionEditParam {
            android_version: Some("1.0.0".into()),
            ios_version: Some("1.0.0".into()),
            ios_link: None,
            android_link: None,
        };
        let entries = build_config_entries(&param);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, BIZ_CODE_ANDROID_VERSION);
        assert_eq!(entries[1].0, BIZ_CODE_IOS_VERSION);
    }

    // ————————————————— 详情组装 —————————————————

    #[test]
    fn detail_maps_missing_keys_to_none() {
        let mut map = HashMap::new();
        map.insert(BIZ_CODE_ANDROID_VERSION.to_string(), "1.2.3".to_string());
        map.insert(BIZ_CODE_IOS_LINK.to_string(), "https://ios".to_string());
        let vo = build_detail(&map);
        assert_eq!(vo.android_version.as_deref(), Some("1.2.3"));
        assert_eq!(vo.ios_link.as_deref(), Some("https://ios"));
        assert_eq!(vo.ios_version, None);
        assert_eq!(vo.android_link, None);
    }

    #[test]
    fn detail_serializes_camel_case_with_nulls() {
        let vo = AppVersionDetailVO {
            android_version: Some("1.2.3".into()),
            ..AppVersionDetailVO::default()
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["androidVersion"], "1.2.3");
        assert!(json["iosVersion"].is_null());
        assert!(json["iosLink"].is_null());
    }

    #[test]
    fn edit_param_deserializes_camel_case() {
        let param: AppVersionEditParam = serde_json::from_str(
            r#"{"androidVersion":"1.0.0","iosVersion":"1.0.1","iosLink":null,"androidLink":"u"}"#,
        )
        .unwrap();
        assert_eq!(param.android_version.as_deref(), Some("1.0.0"));
        assert_eq!(param.ios_version.as_deref(), Some("1.0.1"));
        assert_eq!(param.ios_link, None);
        assert_eq!(param.android_link.as_deref(), Some("u"));
    }

    // ————————————————— 审计字段兜底 —————————————————

    #[test]
    fn admin_name_falls_back_to_unknown() {
        assert_eq!(normalize_admin_name("admin"), "admin");
        assert_eq!(normalize_admin_name("  admin  "), "admin");
        assert_eq!(normalize_admin_name(""), UNKNOWN_ADMIN_NAME);
        assert_eq!(normalize_admin_name("   "), UNKNOWN_ADMIN_NAME);
    }

    // ————————————————— 上传（未迁移） —————————————————

    #[test]
    fn upload_rejects_empty_file() {
        assert!(validate_upload_file(Some("app.apk"), &[]).is_err());
        assert!(validate_upload_file(None, &[1, 2, 3]).is_err());
        assert!(validate_upload_file(Some("   "), &[1, 2, 3]).is_err());
        assert!(validate_upload_file(Some("app.apk"), &[1, 2, 3]).is_ok());
    }

    #[test]
    fn upload_app_file_persists_to_local_storage() {
        let url = upload_app_file(Some("app.apk"), &[1, 2, 3]).expect("上传成功");
        assert!(url.starts_with("/static/app/"), "返回 URL 应为本地静态路径: {url}");
        assert!(url.ends_with(".apk"));
    }
}

// ————————————————— 客户端版本检查（wave9，追加） —————————————————

impl AppVersionAdminService {
    /// 客户端版本检查（Java `AppVersionController.getAppUpdate`）：
    /// 平台匹配且客户端版本落后 → 返回对应下载链接，否则返回空串。
    pub async fn get_app_update(&self, param: &AppVersionParam) -> Result<String> {
        let detail = self.detail().await?;
        let platform = param.r#type.as_deref().unwrap_or_default();
        let client = param.version.as_deref();
        if platform == "ios" && is_old_version(detail.ios_version.as_deref(), client) {
            return Ok(detail.ios_link.unwrap_or_default());
        }
        if platform == "android" && is_old_version(detail.android_version.as_deref(), client) {
            return Ok(detail.android_link.unwrap_or_default());
        }
        Ok(String::new())
    }
}

/// 客户端版本是否落后（Java `AppVersionServiceImpl.isOldVersion`）。
///
/// 语义：`current` 为服务端最新版本，`client` 为客户端版本；
/// 客户端版本 < 最新版本 → true（需要更新）。解析失败/缺省 → false。
#[must_use]
pub fn is_old_version(current: Option<&str>, client: Option<&str>) -> bool {
    parse_version(current) > parse_version(client) && parse_version(client).is_some()
}

fn parse_version(raw: Option<&str>) -> Option<Vec<u64>> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for part in s.split('.') {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            return None;
        }
        out.push(digits.parse().ok()?);
    }
    Some(out)
}

#[cfg(test)]
mod client_tests {
    use super::*;

    #[test]
    fn older_client_is_detected() {
        assert!(is_old_version(Some("1.2.3"), Some("1.2.2")));
        assert!(is_old_version(Some("2.0.0"), Some("1.9.9")));
        assert!(!is_old_version(Some("1.2.3"), Some("1.2.3")));
        assert!(!is_old_version(Some("1.2.3"), Some("1.2.4")));
    }

    #[test]
    fn missing_or_bad_versions_never_flag_update() {
        assert!(!is_old_version(None, Some("1.0.0")));
        assert!(!is_old_version(Some("1.0.0"), None));
        assert!(!is_old_version(Some("abc"), Some("1.0.0")));
    }

    #[test]
    fn version_parser_handles_numeric_parts() {
        assert_eq!(parse_version(Some(" 1.2.3 ")), Some(vec![1, 2, 3]));
        assert_eq!(parse_version(Some("1.2.3-beta")), Some(vec![1, 2, 3]));
        assert_eq!(parse_version(Some("")), None);
    }
}
