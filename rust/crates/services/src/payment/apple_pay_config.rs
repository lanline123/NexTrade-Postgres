//! Apple Pay 设置服务（payment::apple_pay_config，2 个管理端端点）。
//!
//! GET  `applePaySettings`：读 `config` 表 16 个 biz_code（缺省按 `SettingsEnum` 默认值），
//!      `applePayCountryList/applePayCurrencyList` 隐藏 JSON 解析为数组，
//!      敏感字段 `applePayAcquirerMerchantNo/applePayAcquirerKey` 中段掩码。
//! POST `saveApplePay`：只写请求中非 null 字段；敏感字段若与当前掩码值相同视为未修改，
//!      跳过（避免星号覆盖真实密钥）。语义与 Java `ApplePayConfigServiceImpl` 对齐。

use std::sync::Arc;

use serde_json::Value;

use nextrade_core::models::payment::apple_pay_config::{
    derived_biz_codes, load_settings, pick_updates, APPLE_PAY_SPEC,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct ApplePayConfigService {
    repos: Arc<Repositories>,
}

impl ApplePayConfigService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/setting/config/applePaySettings`：读取 Apple Pay 设置。
    pub async fn get_settings(&self) -> Result<Value> {
        let mut codes: Vec<&str> = APPLE_PAY_SPEC.fields.iter().map(|f| f.biz).collect();
        codes.extend(derived_biz_codes(&APPLE_PAY_SPEC));
        let rows = self.repos.payment.apple_pay_config.load(&codes).await?;
        Ok(Value::Object(load_settings(&APPLE_PAY_SPEC, &rows)))
    }

    /// `POST /adminapi/setting/config/saveApplePay`：保存 Apple Pay 设置（部分更新）。
    pub async fn save_settings(&self, payload: &Value) -> Result<()> {
        let current = self.get_settings().await?;
        let empty = serde_json::Map::new();
        let current_masked = current.as_object().unwrap_or(&empty);
        let updates = pick_updates(&APPLE_PAY_SPEC, payload, current_masked);
        if updates.is_empty() && payload.as_object().is_none() {
            return Err(Error::Validation("参数缺失".into()));
        }
        for (biz_code, biz_val) in updates {
            self.repos
                .payment
                .apple_pay_config
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
    fn save_semantics_use_engine_helpers() {
        let spec = APPLE_PAY_SPEC;
        let current = serde_json::json!({"applePayAcquirerKey": "ab****gh"});
        let payload = serde_json::json!({"applePayAcquirerKey": "ab****gh", "useApplePay": 0});
        let updates = pick_updates(&spec, &payload, current.as_object().unwrap());
        assert_eq!(updates, vec![("useApplePay".to_string(), "0".to_string())]);
        assert_eq!(mask_middle_half(Some("abcdefgh")), Some("ab****gh".to_string()));
    }

    #[test]
    fn derived_arrays_are_read_only() {
        let payload = serde_json::json!({"applePayCountryList": [{"code": "CN"}]});
        let updates = pick_updates(&APPLE_PAY_SPEC, &payload, &serde_json::Map::new());
        assert!(updates.is_empty());
    }
}
