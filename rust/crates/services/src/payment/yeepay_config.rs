//! 易宝设置服务（payment::yeepay_config，2 个管理端端点）。
//!
//! GET `yeepaySettings` / POST `saveYeepay`。敏感字段 `yeepayIsvPrivateKey`
//! 中段掩码，掩码未变不回写。默认值：useYeepay=0（其余空）。

use std::sync::Arc;

use serde_json::Value;

use nextrade_core::models::payment::apple_pay_config::{
    load_settings, pick_updates, SettingsField, SettingsSpec,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 易宝设置字段（与 Java `YeepaySettingsVO` 一致）。
pub const YEEPAY_FIELDS: &[SettingsField] = &[
    SettingsField { json: "useYeepay", biz: "useYeepay", default: Some("0") },
    SettingsField { json: "yeepayAppKey", biz: "yeepayAppKey", default: None },
    SettingsField { json: "yeepayParentMerchantNo", biz: "yeepayParentMerchantNo", default: None },
    SettingsField { json: "yeepayMerchantNo", biz: "yeepayMerchantNo", default: None },
    SettingsField { json: "yeepayIsvPrivateKey", biz: "yeepayIsvPrivateKey", default: None },
    SettingsField { json: "yeepayServerRoot", biz: "yeepayServerRoot", default: None },
];

/// 易宝设置规格。
pub const YEEPAY_SPEC: SettingsSpec = SettingsSpec {
    fields: YEEPAY_FIELDS,
    masked: &["yeepayIsvPrivateKey"],
    derived_arrays: &[],
};

pub struct YeepayConfigService {
    repos: Arc<Repositories>,
}

impl YeepayConfigService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/setting/config/yeepaySettings`：读取易宝设置。
    pub async fn get_settings(&self) -> Result<Value> {
        let codes: Vec<&str> = YEEPAY_SPEC.fields.iter().map(|f| f.biz).collect();
        let rows = self.repos.payment.yeepay_config.load(&codes).await?;
        Ok(Value::Object(load_settings(&YEEPAY_SPEC, &rows)))
    }

    /// `POST /adminapi/setting/config/saveYeepay`：保存易宝设置（部分更新）。
    pub async fn save_settings(&self, payload: &Value) -> Result<()> {
        if payload.as_object().is_none() {
            return Err(Error::Validation("参数缺失".into()));
        }
        let current = self.get_settings().await?;
        let empty = serde_json::Map::new();
        let current_masked = current.as_object().unwrap_or(&empty);
        for (biz_code, biz_val) in pick_updates(&YEEPAY_SPEC, payload, current_masked) {
            self.repos
                .payment
                .yeepay_config
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
    fn private_key_is_masked_on_read() {
        let mut rows = std::collections::HashMap::new();
        rows.insert(
            "yeepayIsvPrivateKey".to_string(),
            Some("abcdefgh".to_string()),
        );
        let out = load_settings(&YEEPAY_SPEC, &rows);
        assert_eq!(out["yeepayIsvPrivateKey"], Value::String("ab****gh".into()));
    }

    #[test]
    fn unchanged_masked_key_is_not_rewritten() {
        let current = serde_json::json!({"yeepayIsvPrivateKey": "ab****gh"});
        let payload = serde_json::json!({
            "yeepayIsvPrivateKey": "ab****gh",
            "yeepayAppKey": "app_xxx",
        });
        let updates = pick_updates(&YEEPAY_SPEC, &payload, current.as_object().unwrap());
        assert_eq!(updates, vec![("yeepayAppKey".to_string(), "app_xxx".to_string())]);
    }

    #[test]
    fn defaults_and_helper() {
        let out = load_settings(&YEEPAY_SPEC, &std::collections::HashMap::new());
        assert_eq!(out["useYeepay"], Value::String("0".into()));
        assert_eq!(out["yeepayAppKey"], Value::Null);
        assert_eq!(mask_middle_half(Some("ab")), Some("*b".into()));
    }
}
