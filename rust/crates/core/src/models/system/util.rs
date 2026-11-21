//! 通用工具（system::util）模型层。
//!
//! 对应 Java `UtilController`
//! （`java/nextrade-api/.../controller/common/UtilController.java`，
//! `@RequestMapping("/api/common/util")`），3 个 GET 端点，契约 `auth=optional`：
//!
//! | 端点 | Java 实现 | 本实现 |
//! |------|-----------|--------|
//! | `GET /api/common/util/qrCode?url=` | `QrCodeUtil.generateAsBase64`（hutool） | 本地生成二维码 PNG Base64 |
//! | `GET /api/common/util/miniCode?path=&id=&salesmanId=&shopId=` | `WechatOAuthService.getMiniCode` | 小程序凭据调 `getwxacodeunlimit`；未配置/失败时降级为 H5 链接二维码 |
//! | `GET /api/common/util/miniScene?scene=` | `WechatOAuthService.resolveMiniScene` | query 形式直解 + 短码查缓存映射 |
//! | `GET /api/common/util/shareLink?path=&id=&salesmanId=&shopId=` | 无（我方新增） | 结构化分享结果：小程序 URL Link + 小程序码，降级时 `degraded/notice` 明示 |
//!
//! 历史口径（已废止）：二维码/小程序码曾因外部依赖未迁移而返回 1001
//! 「未迁移」；现均已落地，降级路径一律如实标注，不伪造成功。

use serde::{Deserialize, Serialize};

/// 二维码生成未迁移（Java `QrCodeUtil.generateAsBase64`）。
pub const MSG_QRCODE_NOT_MIGRATED: &str = "二维码生成功能未迁移（QrCodeUtil.generateAsBase64）";
/// 小程序码生成未迁移（Java `WechatOAuthService.getMiniCode`）。
pub const MSG_MINI_CODE_NOT_MIGRATED: &str =
    "小程序码生成功能未迁移（WechatOAuthService.getMiniCode）";
/// 小程序码 scene 解析未迁移（Java `WechatOAuthService.resolveMiniScene`）。
pub const MSG_MINI_SCENE_NOT_MIGRATED: &str =
    "小程序码 scene 解析未迁移（WechatOAuthService.resolveMiniScene）";
/// `url` 参数缺失（Java `@RequestParam String url` 必填）。
pub const MSG_URL_REQUIRED: &str = "url不能为空";
/// `path` 参数缺失（Java `@RequestParam String path` 必填）。
pub const MSG_PATH_REQUIRED: &str = "path不能为空";
/// `id` 参数缺失（Java `@RequestParam Integer id` 必填）。
pub const MSG_ID_REQUIRED: &str = "id不能为空";
/// `scene` 参数缺失（Java `@RequestParam String scene` 必填）。
pub const MSG_SCENE_REQUIRED: &str = "scene不能为空";

/// `GET /api/common/util/qrCode` 查询参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeQuery {
    /// 待编码 URL
    pub url: Option<String>,
}

/// `GET /api/common/util/miniCode` 查询参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniCodeQuery {
    /// 小程序页面路径
    pub path: Option<String>,
    /// 业务 ID（商品/活动等）
    pub id: Option<i32>,
    /// 分销员 ID（可空）
    pub salesman_id: Option<i32>,
    /// 店铺 ID（可空）
    pub shop_id: Option<i32>,
}

/// `GET /api/common/util/miniScene` 查询参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniSceneQuery {
    /// 小程序码 scene 短码
    pub scene: Option<String>,
}

/// 分享通道：小程序直达（`miniProgram`）。
pub const SHARE_CHANNEL_MINI_PROGRAM: &str = "miniProgram";
/// 分享通道：H5 回退（`h5`）。
pub const SHARE_CHANNEL_H5: &str = "h5";

/// `GET /api/common/util/shareLink` 响应（分享直达小程序，我方新增接口）。
///
/// 降级铁律的接口口径：小程序未配置或微信接口失败时**不伪造成功**——
/// `channel` 如实回 `h5`、`degraded=true`、`notice` 写明降级原因，
/// `urlLink`/`miniCode` 为 `null`，分享目标改用恒有值的 `h5Url`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkVO {
    /// 实际下发通道：`miniProgram` 小程序直达 / `h5` 降级回退。
    pub channel: String,
    /// 是否发生降级（未配置小程序或微信接口失败时为 true）。
    pub degraded: bool,
    /// 降级原因（`degraded=true` 时必有值；正常时为 null）。
    pub notice: Option<String>,
    /// 小程序 URL Link（`https://wxaurl.cn/...`；降级时为 null）。
    pub url_link: Option<String>,
    /// 小程序码图片 data URL（降级时为 null）。
    pub mini_code: Option<String>,
    /// H5 分享链接（恒有值；降级时即为实际分享目标）。
    pub h5_url: String,
    /// 本次分享实际使用的小程序 scene（全量参数串或短码）。
    pub scene: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_query_is_optional_camel_case() {
        let q: QrCodeQuery =
            serde_json::from_value(serde_json::json!({"url":"https://a"})).unwrap();
        assert_eq!(q.url.as_deref(), Some("https://a"));
        let empty: QrCodeQuery = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(empty.url.is_none());
    }

    #[test]
    fn mini_code_query_reads_camel_case_ids() {
        let q: MiniCodeQuery = serde_json::from_value(serde_json::json!({
            "path":"pages/index","id":7,"salesmanId":8,"shopId":9
        }))
        .unwrap();
        assert_eq!(q.path.as_deref(), Some("pages/index"));
        assert_eq!(q.id, Some(7));
        assert_eq!(q.salesman_id, Some(8));
        assert_eq!(q.shop_id, Some(9));
    }

    #[test]
    fn share_link_vo_serializes_camel_case_with_degrade_fields() {
        let vo = ShareLinkVO {
            channel: SHARE_CHANNEL_H5.into(),
            degraded: true,
            notice: Some("微信小程序未配置".into()),
            url_link: None,
            mini_code: None,
            h5_url: "https://m.example.com/mobile/pages/product/index?id=1".into(),
            scene: "id=1&salesmanId=0&shopId=0".into(),
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["channel"], "h5");
        assert_eq!(v["degraded"], true);
        assert_eq!(v["notice"], "微信小程序未配置");
        assert!(v["urlLink"].is_null());
        assert!(v["miniCode"].is_null());
        assert!(v["h5Url"].as_str().unwrap().contains("pages/product/index"));
        assert_eq!(v["scene"], "id=1&salesmanId=0&shopId=0");
    }

    #[test]
    fn not_migrated_messages_are_explicit() {
        assert!(MSG_QRCODE_NOT_MIGRATED.contains("未迁移"));
        assert!(MSG_MINI_CODE_NOT_MIGRATED.contains("未迁移"));
        assert!(MSG_MINI_SCENE_NOT_MIGRATED.contains("未迁移"));
    }
}
