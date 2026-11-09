//! 短信 / 邮件通道（替代 Java `SmsUtils` / JavaMail）。
//!
//! - 短信：读取 `config` 表 `smsProvider` / `smsKeyId` / `smsKeySecret` / `smsSignName`
//!   与模板 ID（`TIG_SMS__TEMPLATE_CODE` 环境变量，或 config `smsVerificationTemplate`），
//!   目前实现**阿里云短信 SendSms**（RPC 签名 HMAC-SHA1）；
//!   `TIG_SMS__MOCK=1` 时仅记录日志（本地联调）。
//! - 邮件：读取 `smtpHost/smtpPort/smtpUser/smtpPass/smtpMail/smtpSsl`，用 `lettre` 发送。
//!
//! 未配置凭据时返回明确业务错误（不伪造发送成功）。
use std::collections::HashMap;

use hmac::{Hmac, Mac};
use sha1::Sha1;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

type HmacSha1 = Hmac<Sha1>;

const CFG_SMS_PROVIDER: &str = "smsProvider";
const CFG_SMS_KEY_ID: &str = "smsKeyId";
const CFG_SMS_KEY_SECRET: &str = "smsKeySecret";
const CFG_SMS_SIGN_NAME: &str = "smsSignName";
const CFG_SMS_TEMPLATE: &str = "smsVerificationTemplate";

const CFG_SMS_TENCENT_SECRET_ID: &str = "smsTencentSecretId";
const CFG_SMS_TENCENT_SECRET_KEY: &str = "smsTencentSecretKey";
const CFG_SMS_TENCENT_SDK_APP_ID: &str = "smsTencentSdkAppId";
const CFG_SMS_TENCENT_SIGN_NAME: &str = "smsTencentSignName";

const CFG_SMS_HUAWEI_APP_KEY: &str = "smsHuaweiAppKey";
const CFG_SMS_HUAWEI_APP_SECRET: &str = "smsHuaweiAppSecret";
const CFG_SMS_HUAWEI_CHANNEL: &str = "smsHuaweiChannel";
const CFG_SMS_HUAWEI_SIGN_NAME: &str = "smsHuaweiSignName";
const CFG_SMS_HUAWEI_ENDPOINT: &str = "smsHuaweiEndpoint";

const CFG_SMS_VOLCENGINE_ACCESS_KEY: &str = "smsVolcengineAccessKey";
const CFG_SMS_VOLCENGINE_SECRET_KEY: &str = "smsVolcengineSecretKey";
const CFG_SMS_VOLCENGINE_SMS_ACCOUNT: &str = "smsVolcengineSmsAccount";
const CFG_SMS_VOLCENGINE_SIGN_NAME: &str = "smsVolcengineSignName";

const CFG_SMTP_HOST: &str = "smtpHost";
const CFG_SMTP_PORT: &str = "smtpPort";
const CFG_SMTP_USER: &str = "smtpUser";
const CFG_SMTP_PASS: &str = "smtpPass";
const CFG_SMTP_MAIL: &str = "smtpMail";
const CFG_SMTP_SSL: &str = "smtpSsl";

const SMS_KEYS: [&str; 19] = [
    CFG_SMS_PROVIDER,
    CFG_SMS_KEY_ID,
    CFG_SMS_KEY_SECRET,
    CFG_SMS_SIGN_NAME,
    CFG_SMS_TEMPLATE,
    CFG_SMS_TENCENT_SECRET_ID,
    CFG_SMS_TENCENT_SECRET_KEY,
    CFG_SMS_TENCENT_SDK_APP_ID,
    CFG_SMS_TENCENT_SIGN_NAME,
    CFG_SMS_HUAWEI_APP_KEY,
    CFG_SMS_HUAWEI_APP_SECRET,
    CFG_SMS_HUAWEI_CHANNEL,
    CFG_SMS_HUAWEI_SIGN_NAME,
    CFG_SMS_HUAWEI_ENDPOINT,
    CFG_SMS_VOLCENGINE_ACCESS_KEY,
    CFG_SMS_VOLCENGINE_SECRET_KEY,
    CFG_SMS_VOLCENGINE_SMS_ACCOUNT,
    CFG_SMS_VOLCENGINE_SIGN_NAME,
    CFG_SMTP_SSL,
];
const MAIL_KEYS: [&str; 7] = [
    CFG_SMTP_HOST,
    CFG_SMTP_PORT,
    CFG_SMTP_USER,
    CFG_SMTP_PASS,
    CFG_SMTP_MAIL,
    CFG_SMTP_SSL,
    CFG_SMS_TEMPLATE,
];

/// 配置读取（`config` 表）。
async fn load(repos: &Repositories, keys: &[&str]) -> Result<HashMap<String, String>> {
    let pairs = repos.system.config_basic.find_by_codes(keys).await?;
    Ok(pairs.into_iter().collect())
}

fn env_override(map: &mut HashMap<String, String>, key: &str, env: &str) {
    if let Ok(v) = std::env::var(env) {
        if !v.trim().is_empty() {
            map.insert(key.to_string(), v);
        }
    }
}

/// 阿里云短信 RPC 签名（HMAC-SHA1，`SignatureVersion=1.0`）。
///
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// 返回带签名与公共参数的完整 query 字符串（测试可对固定输入断言）。
#[must_use]
pub fn aliyun_signed_query(
    access_key_id: &str,
    access_key_secret: &str,
    params: &[(&str, String)],
) -> String {

    let mut all: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect();
    all.push(("AccessKeyId".into(), access_key_id.to_string()));
    all.push(("Format".into(), "JSON".into()));
    all.push(("SignatureMethod".into(), "HMAC-SHA1".into()));
    all.push(("SignatureVersion".into(), "1.0".into()));
    all.push(("Version".into(), "2017-05-25".into()));
    all.push(("RegionId".into(), "cn-hangzhou".into()));
    all.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical = all
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let string_to_sign = format!("GET&%2F&{}", percent_encode(&canonical));
    let mut mac = HmacSha1::new_from_slice(format!("{access_key_secret}&").as_bytes())
        .expect("HMAC 支持任意长度 key");
    mac.update(string_to_sign.as_bytes());
    let signature = base64_encode(&mac.finalize().into_bytes());
    format!("{canonical}&Signature={}", percent_encode(&signature))
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// 发送短信验证码。
///
/// # Errors
/// 未配置短信通道 / 模板缺失 / 网络或网关返回失败。
pub async fn send_sms_code(repos: &Repositories, mobile: &str, code: &str) -> Result<()> {
    if std::env::var("TIG_SMS__MOCK").is_ok_and(|v| v == "1") {
        tracing::warn!("TIG_SMS__MOCK=1：短信仅记录日志（mobile={mobile} code={code}）");
        return Ok(());
    }
    let mut cfg = load(repos, &SMS_KEYS).await?;
    env_override(&mut cfg, CFG_SMS_KEY_ID, "TIG_SMS__KEY_ID");
    env_override(&mut cfg, CFG_SMS_KEY_SECRET, "TIG_SMS__KEY_SECRET");
    env_override(&mut cfg, CFG_SMS_SIGN_NAME, "TIG_SMS__SIGN_NAME");
    env_override(&mut cfg, CFG_SMS_TEMPLATE, "TIG_SMS__TEMPLATE_CODE");

    // 原版后台「消息类型 → 获取验证码」把模板 ID 写在
    // `message_template.template_id`；优先读这里，兼容旧的 config 兜底。
    let template = repos
        .system
        .message_type
        .find_sms_template_id_by_message_name("获取验证码")
        .await?
        .or_else(|| cfg.get(CFG_SMS_TEMPLATE).cloned())
        .unwrap_or_default();

    let provider = cfg.get(CFG_SMS_PROVIDER).map(String::as_str).unwrap_or("1");
    match provider {
        "1" | "aliyun" => {
            let key_id = cfg.get(CFG_SMS_KEY_ID).map(String::as_str).unwrap_or_default();
            let key_secret = cfg
                .get(CFG_SMS_KEY_SECRET)
                .map(String::as_str)
                .unwrap_or_default();
            let sign_name = cfg
                .get(CFG_SMS_SIGN_NAME)
                .map(String::as_str)
                .unwrap_or_default();
            let template = template.as_str();
            if key_id.is_empty() || key_secret.is_empty() || sign_name.is_empty() || template.is_empty() {
                return Err(Error::biz(
                    1001,
                    "短信通道未配置：请在设置中填写 smsKeyId/smsKeySecret/smsSignName 与短信模板 ID",
                ));
            }
            send_aliyun_sms(key_id, key_secret, sign_name, template, mobile, code).await
        }
        "2" | "tencent" => {
            let secret_id = cfg.get(CFG_SMS_TENCENT_SECRET_ID).map(String::as_str).unwrap_or_default();
            let secret_key = cfg.get(CFG_SMS_TENCENT_SECRET_KEY).map(String::as_str).unwrap_or_default();
            let sdk_app_id = cfg.get(CFG_SMS_TENCENT_SDK_APP_ID).map(String::as_str).unwrap_or_default();
            let sign_name = cfg.get(CFG_SMS_TENCENT_SIGN_NAME).map(String::as_str).unwrap_or_default();
            let template = template.as_str();
            if secret_id.is_empty() || secret_key.is_empty() || sdk_app_id.is_empty() || sign_name.is_empty() || template.is_empty() {
                return Err(Error::biz(
                    1001,
                    "腾讯云短信通道未配置：请填写 smsTencentSecretId/smsTencentSecretKey/smsTencentSdkAppId/smsTencentSignName",
                ));
            }
            send_tencent_sms(secret_id, secret_key, sdk_app_id, sign_name, template, mobile, code).await
        }
        "3" | "huawei" => {
            let app_key = cfg.get(CFG_SMS_HUAWEI_APP_KEY).map(String::as_str).unwrap_or_default();
            let app_secret = cfg.get(CFG_SMS_HUAWEI_APP_SECRET).map(String::as_str).unwrap_or_default();
            let channel = cfg.get(CFG_SMS_HUAWEI_CHANNEL).map(String::as_str).unwrap_or_default();
            let sign_name = cfg.get(CFG_SMS_HUAWEI_SIGN_NAME).map(String::as_str).unwrap_or_default();
            let endpoint = cfg.get(CFG_SMS_HUAWEI_ENDPOINT).map(String::as_str).unwrap_or_default();
            let template = template.as_str();
            if app_key.is_empty() || app_secret.is_empty() || channel.is_empty() || sign_name.is_empty() || template.is_empty() {
                return Err(Error::biz(
                    1001,
                    "华为云短信通道未配置：请填写 smsHuaweiAppKey/smsHuaweiAppSecret/smsHuaweiChannel/smsHuaweiSignName",
                ));
            }
            send_huawei_sms(endpoint, app_key, app_secret, channel, sign_name, template, mobile, code).await
        }
        "4" | "volcengine" => {
            let access_key = cfg.get(CFG_SMS_VOLCENGINE_ACCESS_KEY).map(String::as_str).unwrap_or_default();
            let secret_key = cfg.get(CFG_SMS_VOLCENGINE_SECRET_KEY).map(String::as_str).unwrap_or_default();
            let sms_account = cfg.get(CFG_SMS_VOLCENGINE_SMS_ACCOUNT).map(String::as_str).unwrap_or_default();
            let sign_name = cfg.get(CFG_SMS_VOLCENGINE_SIGN_NAME).map(String::as_str).unwrap_or_default();
            let template = template.as_str();
            if access_key.is_empty() || secret_key.is_empty() || sms_account.is_empty() || sign_name.is_empty() || template.is_empty() {
                return Err(Error::biz(
                    1001,
                    "火山引擎短信通道未配置：请填写 smsVolcengineAccessKey/smsVolcengineSecretKey/smsVolcengineSmsAccount/smsVolcengineSignName",
                ));
            }
            send_volcengine_sms(access_key, secret_key, sms_account, sign_name, template, mobile, code).await
        }
        other => Err(Error::biz(
            1001,
            format!("不支持的短信服务商标识: {other}，可选：1(阿里云), 2(腾讯云), 3(华为云), 4(火山引擎)"),
        )),
    }
}

async fn send_aliyun_sms(
    key_id: &str,
    key_secret: &str,
    sign_name: &str,
    template: &str,
    mobile: &str,
    code: &str,
) -> Result<()> {
    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let params = [
        ("Action", "SendSms".to_string()),
        ("PhoneNumbers", mobile.to_string()),
        ("SignName", sign_name.to_string()),
        ("TemplateCode", template.to_string()),
        ("TemplateParam", format!("{{\"code\":\"{code}\"}}")),
        ("SignatureNonce", nonce),
        ("Timestamp", timestamp),
    ];
    let query = aliyun_signed_query(key_id, key_secret, &params);
    let url = format!("https://dysmsapi.aliyuncs.com/?{query}");
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("阿里云短信发送网络失败: {e}")))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::Internal(format!("阿里云短信网关返回 {status}: {body}")));
    }
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    if json.get("Code").and_then(serde_json::Value::as_str) == Some("OK") {
        Ok(())
    } else {
        Err(Error::Internal(format!("阿里云短信发送失败: {body}")))
    }
}

async fn send_tencent_sms(
    secret_id: &str,
    secret_key: &str,
    sdk_app_id: &str,
    sign_name: &str,
    template: &str,
    mobile: &str,
    code: &str,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    type HmacSha256 = Hmac<Sha256>;

    let phone = if mobile.starts_with('+') {
        mobile.to_string()
    } else {
        format!("+86{mobile}")
    };
    let payload = serde_json::json!({
        "PhoneNumberSet": [phone],
        "SmsSdkAppId": sdk_app_id,
        "SignName": sign_name,
        "TemplateId": template,
        "TemplateParamSet": [code],
    })
    .to_string();

    let now = chrono::Utc::now();
    let timestamp = now.timestamp();
    let date = now.format("%Y-%m-%d").to_string();

    let canonical_headers = "content-type:application/json; charset=utf-8\nhost:sms.tencentcloudapi.com\nx-tc-action:sendsms\n";
    let signed_headers = "content-type;host;x-tc-action";
    let hashed_payload = hex::encode(Sha256::digest(payload.as_bytes()));
    let canonical_request = format!("POST\n/\n\n{canonical_headers}\n{signed_headers}\n{hashed_payload}");

    let credential_scope = format!("{date}/sms/tc3_request");
    let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign = format!("TC3-HMAC-SHA256\n{timestamp}\n{credential_scope}\n{hashed_canonical_request}");

    let mut mac1 = HmacSha256::new_from_slice(format!("TC3{secret_key}").as_bytes())
        .map_err(|e| Error::Internal(e.to_string()))?;
    mac1.update(date.as_bytes());
    let secret_date = mac1.finalize().into_bytes();

    let mut mac2 = HmacSha256::new_from_slice(&secret_date)
        .map_err(|e| Error::Internal(e.to_string()))?;
    mac2.update(b"sms");
    let secret_service = mac2.finalize().into_bytes();

    let mut mac3 = HmacSha256::new_from_slice(&secret_service)
        .map_err(|e| Error::Internal(e.to_string()))?;
    mac3.update(b"tc3_request");
    let secret_signing = mac3.finalize().into_bytes();

    let mut mac4 = HmacSha256::new_from_slice(&secret_signing)
        .map_err(|e| Error::Internal(e.to_string()))?;
    mac4.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac4.finalize().into_bytes());

    let authorization = format!(
        "TC3-HMAC-SHA256 Credential={secret_id}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let client = reqwest::Client::new();
    let resp = client
        .post("https://sms.tencentcloudapi.com")
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Host", "sms.tencentcloudapi.com")
        .header("X-TC-Action", "SendSms")
        .header("X-TC-Version", "2021-01-11")
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Region", "ap-guangzhou")
        .header("Authorization", authorization)
        .body(payload)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("腾讯云短信发送失败: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::Internal(format!("腾讯云短信网关返回 {status}: {body}")));
    }
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    if let Some(err) = json.get("Response").and_then(|r| r.get("Error")) {
        return Err(Error::Internal(format!("腾讯云短信发送失败: {err}")));
    }
    if let Some(statuses) = json.get("Response").and_then(|r| r.get("SendStatusSet")).and_then(|s| s.as_array()) {
        if let Some(first) = statuses.first() {
            if first.get("Code").and_then(|c| c.as_str()) == Some("Ok") {
                return Ok(());
            }
            let msg = first.get("Message").and_then(|m| m.as_str()).unwrap_or("未知错误");
            return Err(Error::Internal(format!("腾讯云短信状态失败: {msg}")));
        }
    }
    Ok(())
}

async fn send_huawei_sms(
    endpoint: &str,
    app_key: &str,
    app_secret: &str,
    channel: &str,
    sign_name: &str,
    template_code: &str,
    mobile: &str,
    code: &str,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let created = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let digest_input = format!("{nonce}{created}{app_secret}");
    let password_digest = base64_encode(&Sha256::digest(digest_input.as_bytes()));

    let wsse_header = format!(
        "UsernameToken Username=\"{app_key}\",PasswordDigest=\"{password_digest}\",Nonce=\"{nonce}\",Created=\"{created}\""
    );
    let authorization = "WSSE realm=\"SDP\",profile=\"UsernameToken\",type=\"Appkey\"";

    let template_paras = format!("[\"{code}\"]");
    let body = format!(
        "from={}&to={}&templateId={}&templateParas={}&signature={}",
        percent_encode(channel),
        percent_encode(mobile),
        percent_encode(template_code),
        percent_encode(&template_paras),
        percent_encode(sign_name),
    );

    let ep = if endpoint.is_empty() {
        "https://smsapi.cn-north-4.myhuaweicloud.com:443"
    } else {
        endpoint.trim_end_matches('/')
    };
    let url = format!("{ep}/sms/batchSendSms/v1");

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Authorization", authorization)
        .header("X-WSSE", wsse_header)
        .body(body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("华为云短信发送网络失败: {e}")))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::Internal(format!("华为云短信网关返回 {status}: {text}")));
    }
    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    if json.get("code").and_then(serde_json::Value::as_str) == Some("000000") {
        Ok(())
    } else {
        let desc = json.get("description").and_then(|d| d.as_str()).unwrap_or(&text);
        Err(Error::Internal(format!("华为云短信发送失败: {desc}")))
    }
}

async fn send_volcengine_sms(
    access_key: &str,
    secret_key: &str,
    sms_account: &str,
    sign_name: &str,
    template_code: &str,
    mobile: &str,
    code: &str,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    type HmacSha256 = Hmac<Sha256>;

    let payload = serde_json::json!({
        "SmsAccount": sms_account,
        "Sign": sign_name,
        "TemplateId": template_code,
        "PhoneNumbers": mobile,
        "TemplateParam": format!("{{\"code\":\"{code}\"}}"),
    })
    .to_string();

    let now = chrono::Utc::now();
    let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = now.format("%Y%m%d").to_string();
    let region = "cn-north-1";
    let service = "volcSMS";

    let hashed_payload = hex::encode(Sha256::digest(payload.as_bytes()));
    let canonical_querystring = "Action=SendSms&Version=2020-01-01";
    let canonical_headers = format!("content-type:application/json\nhost:sms.volcengineapi.com\nx-content-sha256:{hashed_payload}\nx-date:{x_date}\n");
    let signed_headers = "content-type;host;x-content-sha256;x-date";
    let canonical_request = format!("POST\n/\n{canonical_querystring}\n{canonical_headers}\n{signed_headers}\n{hashed_payload}");

    let credential_scope = format!("{date}/{region}/{service}/request");
    let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign = format!("HMAC-SHA256\n{x_date}\n{credential_scope}\n{hashed_canonical_request}");

    let mut mac1 = HmacSha256::new_from_slice(secret_key.as_bytes()).map_err(|e| Error::Internal(e.to_string()))?;
    mac1.update(date.as_bytes());
    let k_date = mac1.finalize().into_bytes();

    let mut mac2 = HmacSha256::new_from_slice(&k_date).map_err(|e| Error::Internal(e.to_string()))?;
    mac2.update(region.as_bytes());
    let k_region = mac2.finalize().into_bytes();

    let mut mac3 = HmacSha256::new_from_slice(&k_region).map_err(|e| Error::Internal(e.to_string()))?;
    mac3.update(service.as_bytes());
    let k_service = mac3.finalize().into_bytes();

    let mut mac4 = HmacSha256::new_from_slice(&k_service).map_err(|e| Error::Internal(e.to_string()))?;
    mac4.update(b"request");
    let k_signing = mac4.finalize().into_bytes();

    let mut mac5 = HmacSha256::new_from_slice(&k_signing).map_err(|e| Error::Internal(e.to_string()))?;
    mac5.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac5.finalize().into_bytes());

    let authorization = format!(
        "HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("https://sms.volcengineapi.com/?{canonical_querystring}"))
        .header("Content-Type", "application/json")
        .header("Host", "sms.volcengineapi.com")
        .header("X-Date", x_date)
        .header("X-Content-Sha256", hashed_payload)
        .header("Authorization", authorization)
        .body(payload)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| Error::Internal(format!("火山引擎短信发送失败: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(Error::Internal(format!("火山引擎短信网关返回 {status}: {body}")));
    }
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    if let Some(err) = json.get("ResponseMetadata").and_then(|r| r.get("Error")) {
        let msg = err.get("Message").and_then(|m| m.as_str()).unwrap_or("未知错误");
        return Err(Error::Internal(format!("火山引擎短信发送失败: {msg}")));
    }
    if json.get("Result").and_then(|r| r.get("MessageId")).is_some() {
        Ok(())
    } else {
        Err(Error::Internal(format!("火山引擎短信发送未返回有效 MessageId: {body}")))
    }
}

/// 发送邮件验证码（SMTP）。
///
/// # Errors
/// SMTP 未配置 / 发送失败。
/// 发送测试邮件（管理端「发送测试邮件」按钮）。
///
/// # Errors
/// SMTP 未配置 / 发送失败。
pub async fn send_test_email(repos: &Repositories, to: &str) -> Result<()> {
    send_email(
        repos,
        to,
        "测试邮件",
        "<p>这是一封测试邮件，收到此邮件代表着您的邮箱服务器设置正确！</p>",
    )
    .await
}

/// 发送邮件验证码（SMTP）。
///
/// # Errors
/// SMTP 未配置 / 发送失败。
pub async fn send_email_code(repos: &Repositories, to: &str, subject: &str, code: &str) -> Result<()> {
    send_email(
        repos,
        to,
        subject,
        &format!("<p>您的验证码是：<b>{code}</b>，5 分钟内有效。</p>"),
    )
    .await
}

/// 通用 SMTP 发信（HTML 正文）。
///
/// # Errors
/// SMTP 未配置 / 发送失败。
pub async fn send_email(
    repos: &Repositories,
    to: &str,
    subject: &str,
    html_body: &str,
) -> Result<()> {
    let cfg = load(repos, &MAIL_KEYS).await?;
    let host = cfg.get(CFG_SMTP_HOST).cloned().unwrap_or_default();
    let port = cfg
        .get(CFG_SMTP_PORT)
        .and_then(|v| v.trim().parse::<u16>().ok())
        .unwrap_or(465);
    let user = cfg.get(CFG_SMTP_USER).cloned().unwrap_or_default();
    let pass = cfg.get(CFG_SMTP_PASS).cloned().unwrap_or_default();
    let from = cfg
        .get(CFG_SMTP_MAIL)
        .cloned()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| user.clone());
    let ssl = cfg.get(CFG_SMTP_SSL).map(String::as_str).unwrap_or("1") == "1";
    if host.is_empty() || user.is_empty() || pass.is_empty() || from.is_empty() {
        return Err(Error::biz(
            1001,
            "邮件服务未配置：请填写 smtpHost/smtpPort/smtpUser/smtpPass/smtpMail",
        ));
    }

    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let email = Message::builder()
        .from(
            from.parse()
                .map_err(|_| Error::Internal("smtpMail 格式不正确".into()))?,
        )
        .to(to
            .parse()
            .map_err(|_| Error::Validation("邮箱地址不正确".into()))?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body.to_string())
        .map_err(|e| Error::Internal(format!("邮件构建失败: {e}")))?;

    let builder = if ssl {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .map_err(|e| Error::Internal(format!("SMTP 配置失败: {e}")))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| Error::Internal(format!("SMTP 配置失败: {e}")))?
    }
    .port(port)
    .credentials(Credentials::new(user, pass));
    builder
        .build()
        .send(email)
        .await
        .map_err(|e| Error::Internal(format!("邮件发送失败: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliyun_query_is_deterministic_and_contains_signature() {
        let params = [
            ("Action", "SendSms".to_string()),
            ("PhoneNumbers", "13800000001".to_string()),
            ("SignName", "测试签名".to_string()),
            ("TemplateCode", "SMS_123".to_string()),
            ("TemplateParam", "{\"code\":\"123456\"}".to_string()),
            ("SignatureNonce", "fixed-nonce".to_string()),
            ("Timestamp", "2026-01-01T00:00:00Z".to_string()),
        ];
        let q1 = aliyun_signed_query("key-id", "secret", &params);
        let q2 = aliyun_signed_query("key-id", "secret", &params);
        assert_eq!(q1, q2);
        assert!(q1.contains("Signature="));
        assert!(q1.contains("Action=SendSms"));
        // 中文签名必须被 percent-encode
        assert!(q1.contains("%E6%B5%8B%E8%AF%95%E7%AD%BE%E5%90%8D"));
    }

    #[test]
    fn aliyun_signature_differs_with_secret() {
        let params = [("Action", "SendSms".to_string())];
        assert_ne!(
            aliyun_signed_query("k", "s1", &params),
            aliyun_signed_query("k", "s2", &params)
        );
    }
}
