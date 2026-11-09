//! app 版本管理域（system::app_version_admin）DTO / VO。
//!
//! 对应原 Java：
//! - `com.nextrade.bean.param.settings.appversion.AppVersionEditParam`（更新入参）
//! - `com.nextrade.bean.vo.setting.AppVersionDetailVO`（详情出参）
//!
//! JSON 字段全部保持 camelCase，与原接口一致；`config` 中缺失的配置项
//! 在原 Java 侧序列化为 `null`，故详情字段为 `Option` 且**不跳过**序列化。

use serde::{Deserialize, Serialize};

/// 版本编辑参数（Java `AppVersionEditParam`）。
///
/// 4 个字段在 Java 侧均无 `@NotNull` 约束（`@Validated` 不产生额外校验），
/// 版本号格式由 `AppVersionServiceImpl#update` 手工校验。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppVersionEditParam {
    /// Android 版本号。
    pub android_version: Option<String>,
    /// iOS 版本号。
    pub ios_version: Option<String>,
    /// iOS 下载链接。
    pub ios_link: Option<String>,
    /// Android 下载链接。
    pub android_link: Option<String>,
}

/// 版本详情（Java `AppVersionDetailVO`）。
///
/// 字段顺序与 Java 一致：`androidVersion` / `iosVersion` / `iosLink` / `androidLink`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppVersionDetailVO {
    /// Android 版本号。
    pub android_version: Option<String>,
    /// iOS 版本号。
    pub ios_version: Option<String>,
    /// iOS 下载链接。
    pub ios_link: Option<String>,
    /// Android 下载链接。
    pub android_link: Option<String>,
}

/// 客户端版本检查参数（Java `AppVersionParam`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppVersionParam {
    /// 平台：`ios` / `android`
    pub r#type: Option<String>,
    /// 客户端当前版本号（x.y.z）
    pub version: Option<String>,
}
