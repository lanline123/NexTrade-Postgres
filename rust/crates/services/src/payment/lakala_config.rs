//! 拉卡拉设置服务（payment::lakala_config，2 个管理端端点）。
//!
//! GET `lakalaSettings` / POST `saveLakala`。敏感字段 `lakalaMerchantPrivateKey` /
//! `lakalaPlatformCert` / `lakalaSm4Key` 中段掩码，掩码未变不回写。
//! 默认值：useLakala=0、lakalaGatewayUrl=https://s2.lakala.com、lakalaSandbox=0。

use std::sync::Arc;

use serde_json::Value;

use nextrade_core::models::payment::apple_pay_config::{
    load_settings, pick_updates, SettingsField, SettingsSpec,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 拉卡拉设置字段（与 Java `LakalaSettingsVO` 一致）。
pub const LAKALA_FIELDS: &[SettingsField] = &[
    SettingsField { json: "useLakala", biz: "useLakala", default: Some("0") },
    SettingsField { json: "lakalaAppId", biz: "lakalaAppId", default: None },
    SettingsField { json: "lakalaMerchantNo", biz: "lakalaMerchantNo", default: None },
    SettingsField { json: "lakalaTermNo", biz: "lakalaTermNo", default: None },
    SettingsField { json: "lakalaSerialNo", biz: "lakalaSerialNo", default: None },
    SettingsField { json: "lakalaMerchantPrivateKey", biz: "lakalaMerchantPrivateKey", default: None },
    SettingsField { json: "lakalaPlatformCert", biz: "lakalaPlatformCert", default: None },
    SettingsField { json: "lakalaSm4Key", biz: "lakalaSm4Key", default: None },
    SettingsField { json: "lakalaGatewayUrl", biz: "lakalaGatewayUrl", default: Some("https://s2.lakala.com") },
    SettingsField { json: "lakalaSandbox", biz: "lakalaSandbox", default: Some("0") },
    SettingsField { json: "lakalaWxSubAppid", biz: "lakalaWxSubAppid", default: None },
    SettingsField { json: "lakalaWxMiniAppid", biz: "lakalaWxMiniAppid", default: None },
];

/// 拉卡拉设置规格。
pub const LAKALA_SPEC: SettingsSpec = SettingsSpec {
    fields: LAKALA_FIELDS,
    masked: &[
        "lakalaMerchantPrivateKey",
        "lakalaPlatformCert",
        "lakalaSm4Key",
    ],
    derived_arrays: &[],
};

pub struct LakalaConfigService {
    repos: Arc<Repositories>,
}

impl LakalaConfigService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/setting/config/lakalaSettings`：读取拉卡拉设置。
    pub async fn get_settings(&self) -> Result<Value> {
        let codes: Vec<&str> = LAKALA_SPEC.fields.iter().map(|f| f.biz).collect();
        let rows = self.repos.payment.lakala_config.load(&codes).await?;
        Ok(Value::Object(load_settings(&LAKALA_SPEC, &rows)))
    }

    /// `POST /adminapi/setting/config/saveLakala`：保存拉卡拉设置（部分更新）。
    pub async fn save_settings(&self, payload: &Value) -> Result<()> {
        if payload.as_object().is_none() {
            return Err(Error::Validation("参数缺失".into()));
        }
        let current = self.get_settings().await?;
        let empty = serde_json::Map::new();
        let current_masked = current.as_object().unwrap_or(&empty);
        for (biz_code, biz_val) in pick_updates(&LAKALA_SPEC, payload, current_masked) {
            self.repos
                .payment
                .lakala_config
                .upsert(&biz_code, &biz_val)
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::apple_pay_config::mask_middle_half;

    #[test]
    fn masked_secrets_skip_unchanged_and_write_changed() {
        let current = serde_json::json!({
            "lakalaSm4Key": "ab****gh",
            "lakalaMerchantPrivateKey": "12******90",
        });
        let payload = serde_json::json!({
            "lakalaSm4Key": "ab****gh",
            "lakalaMerchantPrivateKey": "brand-new-key",
            "lakalaTermNo": "T001",
        });
        let updates = pick_updates(&LAKALA_SPEC, &payload, current.as_object().unwrap());
        assert!(updates.contains(&(
            "lakalaMerchantPrivateKey".to_string(),
            "brand-new-key".to_string()
        )));
        assert!(updates.contains(&("lakalaTermNo".to_string(), "T001".to_string())));
        assert!(!updates.iter().any(|(biz, _)| biz == "lakalaSm4Key"));
    }

    #[test]
    fn defaults_match_settings_enum() {
        let out = load_settings(&LAKALA_SPEC, &std::collections::HashMap::new());
        assert_eq!(out["useLakala"], Value::String("0".into()));
        assert_eq!(
            out["lakalaGatewayUrl"],
            Value::String("https://s2.lakala.com".into())
        );
        assert_eq!(out["lakalaSandbox"], Value::String("0".into()));
        assert_eq!(out["lakalaAppId"], Value::Null);
    }

    #[test]
    fn masking_helper_shared() {
        assert_eq!(mask_middle_half(Some("abcdefgh")), Some("ab****gh".into()));
    }
}
