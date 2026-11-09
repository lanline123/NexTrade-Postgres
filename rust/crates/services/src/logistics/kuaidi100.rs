//! 快递100（kuaidi100）HTTP 客户端：实时轨迹查询 + 电子面单 V2。
//!
//! 对照 Java `Kuaidi100LogisticsProvider` + `Kuaidi100PollClient` / `Kuaidi100LabelClient`：
//! - 轨迹：`POST https://poll.kuaidi100.com/poll/query.do`
//!   （form：`customer` / `sign` / `param`；签名 `MD5(param+key+customer)` 32 位大写）
//! - 面单：`POST https://api.kuaidi100.com/label/order`
//!   （form：`method=order` / `key` / `sign` / `t` / `param`；
//!   签名 `MD5(param+t+key+secret)` 32 位大写）
//! - 配置项：`kuaidi100Customer` / `kuaidi100Key` / `kuaidi100Secret` /
//!   `kuaidi100PrintType`（HTML/IMAGE/CLOUD）/ `kuaidi100Siid`（云打印设备码）
//! - 物流公司专属字段（partnerId/partnerKey/partnerSecret/net/checkMan/
//!   expType/tempId 等）来自 `logistics_company` 行，与 Java 一致。

use std::collections::HashMap;

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 实时查询接口。
pub const POLL_QUERY_URL: &str = "https://poll.kuaidi100.com/poll/query.do";
/// 电子面单 V2 下单接口。
pub const LABEL_ORDER_URL: &str = "https://api.kuaidi100.com/label/order";

/// 32 位大写 MD5（Java `SecureUtil.md5(...).toUpperCase()`）。
#[must_use]
pub fn md5_upper(input: &str) -> String {
    let digest = Md5::digest(input.as_bytes());
    digest.iter().map(|b| format!("{b:02X}")).collect()
}

/// 轨迹查询结果（字段名对齐 `ShippingResult` 消费端）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kuaidi100QueryResult {
    pub success: bool,
    pub reason: Option<String>,
    pub state: Option<String>,
    pub state_ex: Option<String>,
    pub location: Option<String>,
    pub shipper_code: Option<String>,
    pub logistic_code: Option<String>,
    pub traces: Vec<Kuaidi100Trace>,
}

/// 轨迹节点（快递100 `data[]`：`context` / `ftime` / `location` / `status`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kuaidi100Trace {
    pub accept_time: Option<String>,
    pub accept_station: Option<String>,
    pub location: Option<String>,
}

/// 电子面单结果（对齐 Java 返回结构：`Order` + `PrintTemplate`）。
#[derive(Debug, Clone, Default)]
pub struct Kuaidi100WaybillResult {
    pub success: bool,
    pub reason: String,
    pub order_code: String,
    pub logistic_code: String,
    pub print_template: String,
}

/// 收/发货人（快递100 `recMan` / `sendMan`：`name` / `mobile` / `printAddr`）。
#[derive(Debug, Clone, Default)]
pub struct Kuaidi100Man {
    pub name: String,
    pub mobile: String,
    pub print_addr: String,
}

/// 电子面单请求（字段逐条对齐 Java `createElectronicWaybill` 的 param）。
#[derive(Debug, Clone, Default)]
pub struct Kuaidi100WaybillRequest {
    pub kuaidicom: String,
    pub print_type: String,
    pub order_code: String,
    pub rec_man: Kuaidi100Man,
    pub send_man: Kuaidi100Man,
    pub weight: f64,
    pub remark: String,
    /// 物流公司专属字段（可空则不发送）。
    pub partner_id: Option<String>,
    pub partner_key: Option<String>,
    pub partner_secret: Option<String>,
    pub net: Option<String>,
    pub check_man: Option<String>,
    pub exp_type: Option<String>,
    pub temp_id: Option<String>,
    pub child_temp_id: Option<String>,
    pub back_temp_id: Option<String>,
}

/// 快递100 客户端。
#[derive(Clone)]
pub struct Kuaidi100Client {
    customer: String,
    key: String,
    secret: String,
    print_type: String,
    siid: String,
    http: reqwest::Client,
}

impl Kuaidi100Client {
    /// 从 `config` 表读取凭据构造（Java `isConfigured`：customer + key 非空）。
    pub async fn from_repos(repos: &Repositories) -> Result<Self> {
        let rows = repos
            .system
            .config_basic
            .find_by_codes(&[
                "kuaidi100Customer",
                "kuaidi100Key",
                "kuaidi100Secret",
                "kuaidi100PrintType",
                "kuaidi100Siid",
            ])
            .await?;
        let map: HashMap<String, String> = rows.into_iter().collect();
        let get = |k: &str| map.get(k).cloned().unwrap_or_default();
        Ok(Self {
            customer: get("kuaidi100Customer"),
            key: get("kuaidi100Key"),
            secret: get("kuaidi100Secret"),
            print_type: get("kuaidi100PrintType"),
            siid: get("kuaidi100Siid"),
            http: reqwest::Client::new(),
        })
    }

    /// 凭据是否齐备（轨迹查询需要 customer+key）。
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.customer.trim().is_empty() && !self.key.trim().is_empty()
    }

    /// 面单凭据是否齐备（customer+key+secret，Java 同款三门）。
    #[must_use]
    pub fn is_label_configured(&self) -> bool {
        self.is_configured() && !self.secret.trim().is_empty()
    }

    /// 实时轨迹查询（Java `queryTrack`）。
    pub async fn query_track(
        &self,
        com: &str,
        num: &str,
        phone: Option<&str>,
    ) -> Result<Kuaidi100QueryResult> {
        if !self.is_configured() {
            return Err(Error::biz(1001, "快递100凭证未配置（kuaidi100Customer/kuaidi100Key）"));
        }
        let mut param = json!({ "com": com, "num": num.trim(), "resultv2": "4" });
        if let Some(phone) = phone.map(str::trim).filter(|p| !p.is_empty()) {
            param["phone"] = Value::String(phone.to_string());
        }
        let param_json = param.to_string();
        let sign = md5_upper(&format!("{}{}{}", param_json, self.key, self.customer));

        let resp = self
            .http
            .post(POLL_QUERY_URL)
            .form(&[
                ("customer", self.customer.as_str()),
                ("sign", sign.as_str()),
                ("param", param_json.as_str()),
            ])
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("快递100轨迹查询失败：{e}")))?;
        let text = resp.text().await.unwrap_or_default();
        Ok(parse_query_response(&text, com, num))
    }

    /// 电子面单下单（Java `createElectronicWaybill` 的 HTTP 段）。
    ///
    /// 调用方需先完成 Java 同款校验（打印类型/模板/凭据），本方法只负责
    /// 组 param、签名、请求与响应解析。
    pub async fn create_waybill(&self, req: &Kuaidi100WaybillRequest) -> Result<Kuaidi100WaybillResult> {
        if !self.is_label_configured() {
            return Err(Error::biz_msg(
                "快递100凭证未配置完整（customer/key/secret），请先在配送设置中填写",
            ));
        }
        let mut param = json!({
            "printType": req.print_type,
            "kuaidicom": req.kuaidicom,
            "recMan": { "name": req.rec_man.name, "mobile": req.rec_man.mobile, "printAddr": req.rec_man.print_addr },
            "sendMan": { "name": req.send_man.name, "mobile": req.send_man.mobile, "printAddr": req.send_man.print_addr },
            "cargo": "电商产品",
            "count": 1,
            "weight": req.weight,
            "payType": "SHIPPER",
            "remark": req.remark,
            "orderId": req.order_code,
        });
        let optional_fields = [
            ("partnerId", &req.partner_id),
            ("partnerKey", &req.partner_key),
            ("partnerSecret", &req.partner_secret),
            ("net", &req.net),
            ("checkMan", &req.check_man),
            ("expType", &req.exp_type),
            ("tempId", &req.temp_id),
            ("childTempId", &req.child_temp_id),
            ("backTempId", &req.back_temp_id),
        ];
        for (field, value) in optional_fields {
            if let Some(v) = value.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                param[field] = Value::String(v.to_string());
            }
        }
        if !self.siid.trim().is_empty() {
            param["siid"] = Value::String(self.siid.trim().to_string());
        }

        let param_json = param.to_string();
        let t = nextrade_common::utils::now_millis().to_string();
        let sign = md5_upper(&format!("{}{}{}{}", param_json, t, self.key, self.secret));
        let resp = self
            .http
            .post(LABEL_ORDER_URL)
            .form(&[
                ("method", "order"),
                ("key", self.key.as_str()),
                ("sign", sign.as_str()),
                ("t", t.as_str()),
                ("param", param_json.as_str()),
            ])
            .send()
            .await
            .map_err(|e| Error::biz_msg(format!("快递100电子面单下单失败：{e}")))?;
        let text = resp.text().await.unwrap_or_default();
        parse_label_response(&text, &req.order_code)
    }

    /// 当前打印类型（空回退 HTML，Java `StrUtil.blankToDefault`）。
    #[must_use]
    pub fn print_type(&self) -> &str {
        if self.print_type.trim().is_empty() {
            "HTML"
        } else {
            self.print_type.trim()
        }
    }

    /// 云打印设备码是否已配置（CLOUD 模式必填）。
    #[must_use]
    pub fn siid(&self) -> &str {
        &self.siid
    }
}

/// 解析轨迹响应（Java `queryTrack` 的 status/message 门 + data[] 映射）。
fn parse_query_response(text: &str, com: &str, num: &str) -> Kuaidi100QueryResult {
    let mut out = Kuaidi100QueryResult {
        shipper_code: Some(com.to_string()),
        logistic_code: Some(num.trim().to_string()),
        ..Default::default()
    };
    let Ok(resp) = serde_json::from_str::<Value>(text) else {
        out.reason = Some(format!("快递100响应解析失败：{text}"));
        return out;
    };
    let status = resp["status"].as_str().unwrap_or("");
    let message = resp["message"].as_str().unwrap_or("");
    if status != "200" && !message.eq_ignore_ascii_case("ok") {
        out.reason = Some(format!(
            "快递100轨迹查询未成功: status={status}, message={message}"
        ));
        return out;
    }
    out.success = true;
    out.state = resp["state"].as_str().map(ToString::to_string);
    out.state_ex = out.state.clone();
    out.location = resp["location"].as_str().map(ToString::to_string);
    if let Some(data) = resp["data"].as_array() {
        for item in data {
            out.traces.push(Kuaidi100Trace {
                accept_station: item["context"].as_str().map(ToString::to_string),
                accept_time: item["ftime"].as_str().map(ToString::to_string),
                location: item["location"].as_str().map(ToString::to_string),
            });
        }
    }
    out
}

/// 解析面单响应（Java `createElectronicWaybill` 的 success 门 + data 段）。
fn parse_label_response(text: &str, order_code: &str) -> Result<Kuaidi100WaybillResult> {
    let resp: Value = serde_json::from_str(text)
        .map_err(|_| Error::biz_msg(format!("快递100电子面单响应解析失败：{text}")))?;
    let success = resp["success"].as_bool() == Some(true)
        || resp["code"].as_i64() == Some(200);
    if !success {
        let msg = resp["message"].as_str().unwrap_or("快递100电子面单下单失败");
        return Err(Error::biz_msg(msg.to_string()));
    }
    let data = resp.get("data");
    let kuaidinum = data
        .and_then(|d| d["kuaidinum"].as_str())
        .ok_or_else(|| Error::biz_msg("快递100电子面单返回缺少 data.kuaidinum"))?;
    Ok(Kuaidi100WaybillResult {
        success: true,
        reason: String::new(),
        order_code: order_code.to_string(),
        logistic_code: kuaidinum.to_string(),
        print_template: data
            .and_then(|d| d["label"].as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_sign_matches_java_secureutil() {
        // SecureUtil.md5("abc") = "900150983cd24fb0d6963f7d28e17f72" → 大写
        assert_eq!(md5_upper("abc"), "900150983CD24FB0D6963F7D28E17F72");
    }

    #[test]
    fn query_response_parses_java_fields() {
        let text = r#"{
            "status":"200","message":"ok","state":"0","location":"上海市",
            "data":[{"context":"已签收","ftime":"2026-10-25 10:00:00","location":"上海","status":"签收"}]
        }"#;
        let r = parse_query_response(text, "shunfeng", "SF123 ");
        assert!(r.success);
        assert_eq!(r.shipper_code.as_deref(), Some("shunfeng"));
        assert_eq!(r.logistic_code.as_deref(), Some("SF123"));
        assert_eq!(r.state.as_deref(), Some("0"));
        assert_eq!(r.traces.len(), 1);
        assert_eq!(r.traces[0].accept_station.as_deref(), Some("已签收"));
        assert_eq!(r.traces[0].accept_time.as_deref(), Some("2026-10-25 10:00:00"));
    }

    #[test]
    fn query_response_failure_keeps_reason() {
        let text = r#"{"status":"601","message":"查询无结果"}"#;
        let r = parse_query_response(text, "ems", "E1");
        assert!(!r.success);
        assert!(r.reason.as_deref().unwrap().contains("601"));
    }

    #[test]
    fn label_response_requires_kuaidinum() {
        let ok = r#"{"success":true,"data":{"kuaidinum":"SF888","label":"<html/>"}}"#;
        let r = parse_label_response(ok, "SN1").unwrap();
        assert_eq!(r.logistic_code, "SF888");
        assert_eq!(r.print_template, "<html/>");
        let bad = r#"{"success":false,"message":"余额不足"}"#;
        let err = parse_label_response(bad, "SN1").unwrap_err();
        assert!(err.to_string().contains("余额不足"));
    }
}
