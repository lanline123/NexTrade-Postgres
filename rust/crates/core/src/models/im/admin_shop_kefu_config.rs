//! 客服接入规则（im::admin_shop_kefu_config）模型层。
//!
//! 对应 Java `AdminShopKefuConfigController`
//! （`java/nextrade-adminapi/.../controller/im/AdminShopKefuConfigController.java`，
//! `@RequestMapping("/adminapi/im/shopKefuConfig")`），2 个端点：
//! - `GET  /adminapi/im/shopKefuConfig/accessConfig/detail`（auth=jwt+perm `kefuAccessSetting`）
//! - `POST /adminapi/im/shopKefuConfig/accessConfig/save`
//!
//! 数据表 `shop_kefu_config`（列名以 `java/database/pure.sql` 为准）：
//! `id` / `shop_id` / `auto_access_mode` / `return_customer_access` / `create_time` /
//! `modify_time` 及客服渠道字段（本 feature 只读写前两列）。
//!
//! 说明：Java `getAccessConfig` 对不存在的配置返回 `createDefault(shopId)` 的
//! 默认值（0/0）而不落库；本实现保持一致。

use serde::{Deserialize, Serialize};

/// 默认自动接入模式（Java `autoAccessMode == null ? 0 : …`）。
pub const DEFAULT_AUTO_ACCESS_MODE: i32 = 0;
/// 默认老客接入开关（Java `returnCustomerAccess == null ? 0 : …`）。
pub const DEFAULT_RETURN_CUSTOMER_ACCESS: i32 = 0;
/// 无法获取当前店铺信息（Java `GlobalException("无法获取当前店铺信息")`）。
pub const MSG_SHOP_REQUIRED: &str = "无法获取当前店铺信息";

/// 保存接入规则（Java `KefuAccessConfigSaveDTO`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KefuAccessConfigSaveDTO {
    /// 店铺 ID（平台账号可显式指定；店铺账号由请求头决定）
    pub shop_id: Option<i32>,
    /// 自动接入模式
    pub auto_access_mode: Option<i32>,
    /// 老客接入开关
    pub return_customer_access: Option<i32>,
}

/// 接入规则 VO（Java `KefuAccessConfigVO`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KefuAccessConfigVO {
    /// 店铺 ID
    pub shop_id: i32,
    /// 自动接入模式
    pub auto_access_mode: i32,
    /// 老客接入开关
    pub return_customer_access: i32,
}

/// 接入规则详情查询参数（Java `@RequestParam(value="shopId", required=false)`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KefuAccessConfigQuery {
    /// 店铺 ID（可空）
    pub shop_id: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_dto_is_camel_case() {
        let d: KefuAccessConfigSaveDTO = serde_json::from_value(serde_json::json!({
            "shopId": 3,
            "autoAccessMode": 1,
            "returnCustomerAccess": 2
        }))
        .unwrap();
        assert_eq!(d.shop_id, Some(3));
        assert_eq!(d.auto_access_mode, Some(1));
        assert_eq!(d.return_customer_access, Some(2));
    }

    #[test]
    fn vo_serializes_camel_case_with_defaults() {
        let v = KefuAccessConfigVO {
            shop_id: 3,
            auto_access_mode: DEFAULT_AUTO_ACCESS_MODE,
            return_customer_access: DEFAULT_RETURN_CUSTOMER_ACCESS,
        };
        let j = serde_json::to_value(v).unwrap();
        assert_eq!(j["shopId"], 3);
        assert_eq!(j["autoAccessMode"], 0);
        assert_eq!(j["returnCustomerAccess"], 0);
    }
}
