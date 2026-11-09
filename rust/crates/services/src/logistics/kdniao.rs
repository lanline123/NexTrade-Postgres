//! 快递鸟（KDNiao）HTTP 客户端：签名、即时轨迹查询（1002）、电子面单（1007）。
//!
//! 对照 Java `KdniaoLogisticsProvider`：
//! - endpoint：`https://api.kdniao.com/Ebusiness/EbusinessOrderHandle.aspx`
//! - 表单参数：`RequestData` / `EBusinessID` / `RequestType` / `DataSign` / `DataType=2`
//! - 签名：`Base64( hex( MD5(RequestData + ApiKey) ) )`
//!   （注意：Base64 的输入是 MD5 的**十六进制字符串**，不是原始 16 字节）
//! - 配置项：`kdniaoBusinessId` / `kdniaoApiKey`
//!
//! 1002 的 SF 查询需要 `CustomerName`（收件人手机号后四位）；
//! 极兔等承运商可不传。1007 需要发货人/收件人/物流公司面单账号等字段。

use base64::Engine;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 快递鸟下单/查询接口地址。
pub const KDNIAO_ENDPOINT: &str =
    "https://api.kdniao.com/Ebusiness/EbusinessOrderHandle.aspx";
/// 即时轨迹查询。
pub const REQUEST_TYPE_TRACK: &str = "1002";
/// 电子面单。
pub const REQUEST_TYPE_WAYBILL: &str = "1007";

/// 快递鸟客户端。
#[derive(Clone, Debug)]
pub struct KdniaoClient {
    business_id: String,
    api_key: String,
    endpoint: String,
}

/// 1002 轨迹节点。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct KdniaoTrace {
    pub accept_time: Option<String>,
    pub accept_station: Option<String>,
    pub location: Option<String>,
}

/// 1002 查询结果（映射 Java `ShippingResult` 所需字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KdniaoQueryResult {
    pub success: bool,
    pub reason: Option<String>,
    pub state: Option<String>,
    pub state_ex: Option<String>,
    pub location: Option<String>,
    pub shipper_code: Option<String>,
    pub logistic_code: Option<String>,
    pub traces: Vec<KdniaoTrace>,
}

/// 收件人/发货人地址块。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KdniaoAddress {
    pub name: String,
    pub mobile: String,
    pub province_name: String,
    pub city_name: String,
    pub exp_area_name: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_code: Option<String>,
}

/// 1007 电子面单请求。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KdniaoWaybillRequest {
    pub order_code: String,
    pub shipper_code: String,
    pub pay_type: i32,
    pub exp_type: String,
    pub receiver: KdniaoAddress,
    pub sender: KdniaoAddress,
    pub quantity: i32,
    pub weight: f64,
    pub volume: f64,
    pub remark: Option<String>,
    pub commodity: Vec<Value>,
    pub customer_name: Option<String>,
    pub customer_pwd: Option<String>,
    pub send_site: Option<String>,
    pub send_staff: Option<String>,
    pub month_code: Option<String>,
    pub is_notice: i32,
    pub start_date: String,
    pub end_date: String,
    pub currency_code: String,
    pub is_return_print_template: i32,
}

/// 1007 电子面单结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KdniaoWaybillResult {
    pub success: bool,
    pub reason: Option<String>,
    pub order_code: Option<String>,
    pub logistic_code: Option<String>,
    pub shipper_code: Option<String>,
    pub print_template: Option<String>,
    pub raw: Value,
}

impl KdniaoClient {
    #[must_use]
    pub fn new(business_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            business_id: business_id.into(),
            api_key: api_key.into(),
            endpoint: KDNIAO_ENDPOINT.to_string(),
        }
    }

    /// 覆盖 endpoint（测试/代理用）。
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// 从 `config` 表读取 `kdniaoBusinessId` / `kdniaoApiKey`。
    pub async fn from_repos(repos: &Repositories) -> Result<Self> {
        let rows = repos
            .system
            .config_basic
            .find_by_codes(&["kdniaoBusinessId", "kdniaoApiKey"])
            .await?;
        let map: std::collections::HashMap<String, String> = rows.into_iter().collect();
        let business_id = map
            .get("kdniaoBusinessId")
            .map(String::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(1001, "快递鸟服务ID未配置（kdniaoBusinessId）"))?;
        let api_key = map
            .get("kdniaoApiKey")
            .map(String::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(1001, "快递鸟API Key未配置（kdniaoApiKey）"))?;
        Ok(Self::new(business_id, api_key))
    }

    /// 快递鸟 `DataSign`：`Base64(hex(MD5(request_data + api_key)))`。
    #[must_use]
    pub fn sign(&self, request_data: &str) -> String {
        sign_with_key(request_data, &self.api_key)
    }

    /// 1002 即时轨迹查询。
    ///
    /// `customer_name` 对 SF/KYSY/ZTO 等必填（Java 传手机号后四位）。
    pub async fn query_track(
        &self,
        shipper_code: &str,
        logistic_code: &str,
        customer_name: Option<&str>,
    ) -> Result<KdniaoQueryResult> {
        let mut request = serde_json::Map::new();
        request.insert("ShipperCode".into(), Value::String(shipper_code.to_string()));
        request.insert(
            "LogisticCode".into(),
            Value::String(logistic_code.trim().to_string()),
        );
        if let Some(name) = customer_name.map(str::trim).filter(|s| !s.is_empty()) {
            request.insert("CustomerName".into(), Value::String(name.to_string()));
        }
        let request_data = Value::Object(request).to_string();
        let raw = self
            .post(REQUEST_TYPE_TRACK, &request_data)
            .await?;
        let success = kdn_success(&raw);
        let reason = raw
            .get("Reason")
            .and_then(Value::as_str)
            .map(str::to_string);
        let traces = raw
            .get("Traces")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value::<KdniaoTrace>(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(KdniaoQueryResult {
            success,
            reason,
            state: raw.get("State").and_then(Value::as_str).map(str::to_string),
            state_ex: raw.get("StateEx").and_then(Value::as_str).map(str::to_string),
            location: raw
                .get("Location")
                .and_then(Value::as_str)
                .map(str::to_string),
            shipper_code: raw
                .get("ShipperCode")
                .and_then(Value::as_str)
                .map(str::to_string),
            logistic_code: raw
                .get("LogisticCode")
                .and_then(Value::as_str)
                .map(str::to_string),
            traces,
        })
    }

    /// 1007 电子面单。
    pub async fn create_waybill(
        &self,
        request: &KdniaoWaybillRequest,
    ) -> Result<KdniaoWaybillResult> {
        let request_data = serde_json::to_string(request)
            .map_err(|e| Error::Internal(format!("快递鸟面单参数序列化失败: {e}")))?;
        let raw = self
            .post(REQUEST_TYPE_WAYBILL, &request_data)
            .await?;
        let success = kdn_success(&raw);
        let order = raw.get("Order");
        let result = KdniaoWaybillResult {
            success,
            reason: raw
                .get("Reason")
                .and_then(Value::as_str)
                .map(str::to_string),
            order_code: order
                .and_then(|o| o.get("OrderCode"))
                .and_then(Value::as_str)
                .map(str::to_string),
            logistic_code: order
                .and_then(|o| o.get("LogisticCode"))
                .and_then(Value::as_str)
                .map(str::to_string),
            shipper_code: order
                .and_then(|o| o.get("ShipperCode"))
                .and_then(Value::as_str)
                .map(str::to_string),
            print_template: raw
                .get("PrintTemplate")
                .and_then(Value::as_str)
                .map(str::to_string),
            raw,
        };
        if !result.success {
            return Err(Error::biz(
                1001,
                result
                    .reason
                    .clone()
                    .unwrap_or_else(|| "快递鸟电子面单失败".to_string()),
            ));
        }
        if result.logistic_code.as_deref().unwrap_or("").trim().is_empty() {
            return Err(Error::biz(1001, "快递鸟电子面单未返回运单号"));
        }
        Ok(result)
    }

    async fn post(&self, request_type: &str, request_data: &str) -> Result<Value> {
        let sign = self.sign(request_data);
        let form = [
            ("RequestData", request_data),
            ("EBusinessID", self.business_id.as_str()),
            ("RequestType", request_type),
            ("DataSign", sign.as_str()),
            ("DataType", "2"),
        ];
        let resp = reqwest::Client::new()
            .post(&self.endpoint)
            .form(&form)
            .timeout(std::time::Duration::from_secs(25))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("快递鸟请求失败: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Internal(format!("快递鸟响应读取失败: {e}")))?;
        if !status.is_success() {
            return Err(Error::Internal(format!("快递鸟 HTTP {status}: {body}")));
        }
        serde_json::from_str::<Value>(&body)
            .map_err(|e| Error::Internal(format!("快递鸟响应解析失败: {e}; body={body}")))
    }
}

/// 无客户端状态的签名函数（便于单测）。
#[must_use]
pub fn sign_with_key(request_data: &str, api_key: &str) -> String {
    let hex = hex::encode(Md5::digest(format!("{request_data}{api_key}").as_bytes()));
    base64::engine::general_purpose::STANDARD.encode(hex.as_bytes())
}

/// KDN `Success` 既可能是布尔也可能字符串。
fn kdn_success(value: &Value) -> bool {
    match value.get("Success") {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s.eq_ignore_ascii_case("true") || s == "1",
        Some(Value::Number(n)) => n.as_i64() == Some(1),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_uses_base64_of_hex_md5() {
        // 固定向量由 Python 以 Java 规则计算：
        // Base64(hex(MD5('{"ShipperCode":"JTSD","LogisticCode":"JT3175428264802"}' + "test-key")))
        assert_eq!(
            sign_with_key(
                r#"{"ShipperCode":"JTSD","LogisticCode":"JT3175428264802"}"#,
                "test-key"
            ),
            "NGRkMzIwODQ0MjE2Nzg3MDBkNWNmYTU4NDQzZDVkMTY="
        );
    }

    #[test]
    fn signature_is_not_base64_of_raw_digest() {
        let raw = base64::engine::general_purpose::STANDARD
            .encode(Md5::digest(b"{}key"));
        assert_ne!(sign_with_key("{}", "key"), raw);
    }

    #[test]
    fn success_accepts_bool_string_and_number() {
        assert!(kdn_success(&serde_json::json!({"Success": true})));
        assert!(kdn_success(&serde_json::json!({"Success": "true"})));
        assert!(kdn_success(&serde_json::json!({"Success": 1})));
        assert!(!kdn_success(&serde_json::json!({"Success": false})));
        assert!(!kdn_success(&serde_json::json!({"Success": "false"})));
        assert!(!kdn_success(&serde_json::json!({})));
    }

    #[test]
    fn trace_deserializes_pascal_case() {
        let trace: KdniaoTrace = serde_json::from_value(serde_json::json!({
            "AcceptTime": "2026-10-20 10:00:00",
            "AcceptStation": "已揽收",
            "Location": "青岛"
        }))
        .unwrap();
        assert_eq!(trace.accept_time.as_deref(), Some("2026-10-20 10:00:00"));
        assert_eq!(trace.accept_station.as_deref(), Some("已揽收"));
        assert_eq!(trace.location.as_deref(), Some("青岛"));
    }

    #[test]
    fn waybill_request_serializes_pascal_case() {
        let req = KdniaoWaybillRequest {
            order_code: "SN1".into(),
            shipper_code: "JTSD".into(),
            receiver: KdniaoAddress {
                name: "张三".into(),
                mobile: "13800000000".into(),
                province_name: "山东省".into(),
                city_name: "青岛市".into(),
                exp_area_name: "市南区".into(),
                address: "测试路1号".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["OrderCode"], "SN1");
        assert_eq!(value["Receiver"]["ProvinceName"], "山东省");
    }
}
