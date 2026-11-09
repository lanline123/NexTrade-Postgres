//! 汇付斗拱设置服务（payment::huifu_config，2 个管理端端点）。
//!
//! GET  `huifuSettings` / POST `saveHuifu`。
//! biz_code 与字段 camelCase 属性同名（Java `SettingsEnum`）；敏感字段
//! `huifuRsaPrivateKey` / `huifuPublicKey` 中段掩码，掩码未变不回写。
//! 默认值按 `SettingsEnum`：huifuProfitSharingEnabled=0、huifuDelayAcctEnabled=0、
//! huifuProfitSharingMaxRatio=20.00、huifuProfitSharingRuleOrigin=01。

use std::sync::Arc;

use serde_json::Value;

use nextrade_core::models::payment::apple_pay_config::{
    load_settings, pick_updates, SettingsField, SettingsSpec,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 汇付设置字段（与 Java `HuifuSettingsVO` 一致）。
pub const HUIFU_FIELDS: &[SettingsField] = &[
    SettingsField { json: "useHuifu", biz: "useHuifu", default: None },
    SettingsField { json: "huifuSysId", biz: "huifuSysId", default: None },
    SettingsField { json: "huifuProductId", biz: "huifuProductId", default: None },
    SettingsField { json: "huifuMerchantId", biz: "huifuMerchantId", default: None },
    SettingsField { json: "huifuRsaPrivateKey", biz: "huifuRsaPrivateKey", default: None },
    SettingsField { json: "huifuPublicKey", biz: "huifuPublicKey", default: None },
    SettingsField { json: "huifuGateway", biz: "huifuGateway", default: None },
    SettingsField { json: "huifuProfitSharingEnabled", biz: "huifuProfitSharingEnabled", default: Some("0") },
    SettingsField { json: "huifuDelayAcctEnabled", biz: "huifuDelayAcctEnabled", default: Some("0") },
    SettingsField { json: "huifuProfitSharingNotifyUrl", biz: "huifuProfitSharingNotifyUrl", default: None },
    SettingsField { json: "huifuProfitSharingMaxRatio", biz: "huifuProfitSharingMaxRatio", default: Some("20.00") },
    SettingsField { json: "huifuProfitSharingRuleOrigin", biz: "huifuProfitSharingRuleOrigin", default: Some("01") },
    SettingsField { json: "huifuHostingProjectId", biz: "huifuHostingProjectId", default: None },
];

/// 汇付设置规格。
pub const HUIFU_SPEC: SettingsSpec = SettingsSpec {
    fields: HUIFU_FIELDS,
    masked: &["huifuRsaPrivateKey", "huifuPublicKey"],
    derived_arrays: &[],
};

pub struct HuifuConfigService {
    repos: Arc<Repositories>,
}

impl HuifuConfigService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/setting/config/huifuSettings`：读取汇付设置。
    pub async fn get_settings(&self) -> Result<Value> {
        let codes: Vec<&str> = HUIFU_SPEC.fields.iter().map(|f| f.biz).collect();
        let rows = self.repos.payment.huifu_config.load(&codes).await?;
        Ok(Value::Object(load_settings(&HUIFU_SPEC, &rows)))
    }

    /// `POST /adminapi/setting/config/saveHuifu`：保存汇付设置（部分更新）。
    pub async fn save_settings(&self, payload: &Value) -> Result<()> {
        if payload.as_object().is_none() {
            return Err(Error::Validation("参数缺失".into()));
        }
        let current = self.get_settings().await?;
        let empty = serde_json::Map::new();
        let current_masked = current.as_object().unwrap_or(&empty);
        for (biz_code, biz_val) in pick_updates(&HUIFU_SPEC, payload, current_masked) {
            self.repos
                .payment
                .huifu_config
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
    fn masked_fields_are_not_overwritten_when_unchanged() {
        let current = serde_json::json!({"huifuRsaPrivateKey": "ab****gh"});
        let payload = serde_json::json!({
            "huifuRsaPrivateKey": "ab****gh",
            "huifuSysId": "SYS1",
        });
        let updates = pick_updates(&HUIFU_SPEC, &payload, current.as_object().unwrap());
        assert_eq!(updates, vec![("huifuSysId".to_string(), "SYS1".to_string())]);
    }

    #[test]
    fn defaults_match_settings_enum() {
        let out = load_settings(&HUIFU_SPEC, &std::collections::HashMap::new());
        assert_eq!(out["huifuProfitSharingEnabled"], Value::String("0".into()));
        assert_eq!(out["huifuDelayAcctEnabled"], Value::String("0".into()));
        assert_eq!(out["huifuProfitSharingMaxRatio"], Value::String("20.00".into()));
        assert_eq!(out["huifuProfitSharingRuleOrigin"], Value::String("01".into()));
        assert_eq!(out["huifuSysId"], Value::Null);
    }

    #[test]
    fn masking_helper_shared() {
        assert_eq!(mask_middle_half(Some("abcdefgh")), Some("ab****gh".into()));
    }
}
