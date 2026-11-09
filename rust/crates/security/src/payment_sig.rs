//! 支付回调验签（渠道通知的签名校验）。
//!
//! 对应原 Java 各支付 SDK 内置的验签逻辑（微信支付 V3 `SignatureExec`、
//! 支付宝 `AlipaySignature.rsaCheckV1` 等）。Rust 侧此前缺失：
//! `/api/order/pay/notify/{payType}` 直接落库 `mark_paid`，任何人伪造请求
//! 即可把订单置为已支付 —— 生产级安全红线。
//!
//! 本模块提供两种规范验签原语 + 渠道适配层：
//!
//! | 方案 | 用途 | 算法 |
//!|------|------|------|
//!| [`HmacVerifier`] | 对称密钥：回调带 `X-Timestamp` / `X-Nonce` / `X-Signature` | HMAC-SHA256，签名内容 = `ts\nnonce\nbody\n` |
//!| [`RsaVerifier`] | 非对称公钥：渠道用私钥签、我方用公钥验 | RSA-PKCS1#v1.5 + SHA-256 |
//!
//! 重放防护：时间戳新鲜度窗口 + nonce 去重（由调用方注入存储，见
//! [`NonceStore`]）。

use base64::Engine as _;
use ring::{
    hmac::{self, Key},
    rand::SystemRandom,
    signature::{UnparsedPublicKey, RSA_PKCS1_SHA256},
};
use sha2::{Digest, Sha256};

/// 验签失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// 缺少必需的签名相关头
    MissingHeader(&'static str),
    /// Base64 解码失败
    BadSignatureEncoding,
    /// 时间戳无法解析
    BadTimestamp,
    /// 时间戳超出允许窗口（疑似重放）
    TimestampOutOfWindow { skew_secs: i64, window_secs: i64 },
    /// nonce 已见过（重放）
    Replay,
    /// 签名不匹配（被篡改或密钥错误）
    SignatureMismatch,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHeader(h) => write!(f, "缺少签名头 {h}"),
            Self::BadSignatureEncoding => write!(f, "签名 Base64 编码非法"),
            Self::BadTimestamp => write!(f, "时间戳格式非法"),
            Self::TimestampOutOfWindow { skew_secs, window_secs } => {
                write!(f, "时间戳偏差 {skew_secs}s 超出窗口 {window_secs}s")
            }
            Self::Replay => write!(f, "检测到重放请求"),
            Self::SignatureMismatch => write!(f, "回调签名校验失败"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// 签名方案抽象：把「请求元数据」规范化后验签。
pub trait SignatureScheme: Send + Sync {
    /// 验证签名。`headers` 为小写键的有序头列表，`body` 为原始报文。
    fn verify(&self, headers: &[(String, String)], body: &[u8], now_unix: i64)
        -> Result<(), VerifyError>;
}

/// 从 headers 中取值（大小写不敏感）。
fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// 校验时间戳是否在新鲜度窗口内。
fn check_timestamp(ts_str: &str, now_unix: i64, window_secs: i64) -> Result<(), VerifyError> {
    let ts: i64 = ts_str.trim().parse().map_err(|_| VerifyError::BadTimestamp)?;
    let skew = (now_unix - ts).abs();
    if skew > window_secs {
        return Err(VerifyError::TimestampOutOfWindow {
            skew_secs: skew,
            window_secs,
        });
    }
    Ok(())
}

/// HMAC-SHA256 对称验签（适用于自建签名约定 / 拉卡拉 / 易宝等）。
///
/// 签名内容（canonical）：`{X-Timestamp}\n{X-Nonce}\n{body}\n`
/// 头名与原 Java 渠道可配置不同，这里固定约定，若渠道不同由
/// 适配层重新映射后再传入。
pub struct HmacVerifier {
    secret: Box<[u8]>,
    timestamp_header: &'static str,
    nonce_header: &'static str,
    signature_header: &'static str,
    window_secs: i64,
}

impl HmacVerifier {
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec().into_boxed_slice(),
            timestamp_header: "X-Timestamp",
            nonce_header: "X-Nonce",
            signature_header: "X-Signature",
            window_secs: 300,
        }
    }

    /// 自定义头名（适配不同渠道）
    #[must_use]
    pub fn with_headers(
        mut self,
        ts: &'static str,
        nonce: &'static str,
        sig: &'static str,
    ) -> Self {
        self.timestamp_header = ts;
        self.nonce_header = nonce;
        self.signature_header = sig;
        self
    }

    /// 自定义新鲜度窗口（秒）
    #[must_use]
    pub fn with_window(mut self, secs: i64) -> Self {
        self.window_secs = secs;
        self
    }

    /// 计算 HMAC-SHA256（Base64）——供签名方（含测试）复用，避免两套实现。
    pub fn sign(&self, ts: &str, nonce: &str, body: &[u8]) -> String {
        let canonical = canonical_string(ts, nonce, body);
        let key = Key::new(hmac::HMAC_SHA256, &self.secret);
        let tag = hmac::sign(&key, canonical.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(tag.as_ref())
    }
}

/// 构造 canonical 待签字符串：`ts\nnonce\nbody\n`
fn canonical_string(ts: &str, nonce: &str, body: &[u8]) -> String {
    let mut s = String::with_capacity(ts.len() + nonce.len() + body.len() + 3);
    s.push_str(ts);
    s.push('\n');
    s.push_str(nonce);
    s.push('\n');
    // body 原文按 UTF-8 处理；非 UTF-8 用 lossy（渠道均发 JSON，此处不丢签名字节）
    s.push_str(&String::from_utf8_lossy(body));
    s.push('\n');
    s
}

impl SignatureScheme for HmacVerifier {
    fn verify(
        &self,
        headers: &[(String, String)],
        body: &[u8],
        now_unix: i64,
    ) -> Result<(), VerifyError> {
        let ts = header_value(headers, self.timestamp_header)
            .ok_or(VerifyError::MissingHeader("X-Timestamp"))?;
        let nonce =
            header_value(headers, self.nonce_header).ok_or(VerifyError::MissingHeader("X-Nonce"))?;
        let sig = header_value(headers, self.signature_header)
            .ok_or(VerifyError::MissingHeader("X-Signature"))?;

        check_timestamp(ts, now_unix, self.window_secs)?;

        let provided = base64::engine::general_purpose::STANDARD
            .decode(sig.trim())
            .map_err(|_| VerifyError::BadSignatureEncoding)?;

        let canonical = canonical_string(ts, nonce, body);
        let key = Key::new(hmac::HMAC_SHA256, &self.secret);
        hmac::verify(&key, canonical.as_bytes(), &provided)
            .map_err(|_| VerifyError::SignatureMismatch)
    }
}

/// RSA 公钥验签（微信 V3 / 支付宝 RSA2 等）。
///
/// 验签对象由 `canonical_fn` 构造（不同渠道拼接规则不同）：
/// - 微信 V3：`timestamp\nnonce\nbody\n`
/// - 支付宝：参数字典序拼接（适配层负责拼好再传 `body`）
pub struct RsaVerifier {
    /// DER (SPKI) 公钥，已去除 PEM 头尾与换行
    public_key_der: Vec<u8>,
    signature_header: &'static str,
    timestamp_header: &'static str,
    window_secs: i64,
    /// 是否校验时间戳（支付宝无独立时间戳头时设 false）
    check_freshness: bool,
}

impl RsaVerifier {
    /// 从 PEM 文本构建（自动剥头尾、base64 解码到 DER）。
    pub fn from_pem(pem: &str) -> Result<Self, VerifyError> {
        let der = pem_to_der(pem).ok_or(VerifyError::BadSignatureEncoding)?;
        Ok(Self {
            public_key_der: der,
            signature_header: "X-Signature",
            timestamp_header: "X-Timestamp",
            window_secs: 300,
            check_freshness: true,
        })
    }

    /// 直接用 SPKI DER 字节构建。
    pub fn from_der(der: impl AsRef<[u8]>) -> Self {
        Self {
            public_key_der: der.as_ref().to_vec(),
            signature_header: "X-Signature",
            timestamp_header: "X-Timestamp",
            window_secs: 300,
            check_freshness: true,
        }
    }

    #[must_use]
    pub fn with_headers(mut self, sig: &'static str, ts: &'static str) -> Self {
        self.signature_header = sig;
        self.timestamp_header = ts;
        self
    }

    /// 关闭新鲜度校验（渠道无时间戳头时）
    #[must_use]
    pub fn without_freshness(mut self) -> Self {
        self.check_freshness = false;
        self
    }

    /// 对任意消息计算 SHA-256 摘要（调试 / 适配层拼接后可用）
    pub fn sha256(data: &[u8]) -> Vec<u8> {
        let mut h = Sha256::new();
        h.update(data);
        h.finalize().to_vec()
    }
}

/// 把 PEM 公钥文本转为 DER 字节。
fn pem_to_der(pem: &str) -> Option<Vec<u8>> {
    let in_body = pem
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");
    base64::engine::general_purpose::STANDARD
        .decode(in_body)
        .ok()
}

impl SignatureScheme for RsaVerifier {
    fn verify(
        &self,
        headers: &[(String, String)],
        body: &[u8],
        now_unix: i64,
    ) -> Result<(), VerifyError> {
        let sig = header_value(headers, self.signature_header)
            .ok_or(VerifyError::MissingHeader("X-Signature"))?;

        let ts = header_value(headers, self.timestamp_header);

        // 签名内容：时间戳头存在则拼入，否则退化为纯 body（适配层负责构造）
        let canonical: Vec<u8> = match ts {
            Some(t) if self.check_freshness => {
                let nonce = header_value(headers, "X-Nonce").unwrap_or("");
                canonical_string(t, nonce, body).into_bytes()
            }
            _ => body.to_vec(),
        };

        if self.check_freshness {
            let t = ts.ok_or(VerifyError::MissingHeader("X-Timestamp"))?;
            check_timestamp(t, now_unix, self.window_secs)?;
        }

        let provided = base64::engine::general_purpose::STANDARD
            .decode(sig.trim())
            .map_err(|_| VerifyError::BadSignatureEncoding)?;

        let pub_key = UnparsedPublicKey::new(&RSA_PKCS1_SHA256, &self.public_key_der);
        pub_key
            .verify(&canonical, &provided)
            .map_err(|_| VerifyError::SignatureMismatch)
    }
}

/// nonce 去重抽象（重放防护的第二道闸）。
///
/// 实现方用 Redis `SETNX key TTL`；此处仅定义接口便于注入。
pub trait NonceStore: Send + Sync {
    /// 若 nonce 首次出现则记录并返回 true；已存在返回 false。
    fn check_and_store(&self, nonce: &str, ttl_secs: i64) -> bool;
}

/// 内存版 nonce 存储（仅用于开发 / 测试；生产须用 Redis）。
#[derive(Default)]
pub struct InMemoryNonceStore {
    seen: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl NonceStore for InMemoryNonceStore {
    fn check_and_store(&self, nonce: &str, _ttl_secs: i64) -> bool {
        let mut g = self.seen.lock().unwrap_or_else(|p| p.into_inner());
        g.insert(nonce.to_string())
    }
}

/// 组合验签：签名方案 + 可选 nonce 去重（重放防护）。
pub struct NotifyVerifier<S: SignatureScheme> {
    scheme: S,
    nonce_store: Option<std::sync::Arc<dyn NonceStore>>,
    nonce_ttl_secs: i64,
}

impl<S: SignatureScheme> NotifyVerifier<S> {
    pub fn new(scheme: S) -> Self {
        Self {
            scheme,
            nonce_store: None,
            nonce_ttl_secs: 600,
        }
    }

    #[must_use]
    pub fn with_nonce_store(mut self, store: std::sync::Arc<dyn NonceStore>) -> Self {
        self.nonce_store = Some(store);
        self
    }

    /// 验签；成功后若配置 nonce store 再做重放检查。
    pub fn verify(
        &self,
        headers: &[(String, String)],
        body: &[u8],
        now_unix: i64,
    ) -> Result<(), VerifyError> {
        self.scheme.verify(headers, body, now_unix)?;
        if let Some(store) = &self.nonce_store {
            let nonce = header_value(headers, "X-Nonce").ok_or(VerifyError::MissingHeader("X-Nonce"))?;
            if !store.check_and_store(nonce, self.nonce_ttl_secs) {
                return Err(VerifyError::Replay);
            }
        }
        Ok(())
    }
}

/// 把 `Authorization` 风格的简单 header 列表转成验签所需格式。
/// 便捷工具，供 Axum 路由层直接复用。
pub fn headers_to_pairs(
    headers: &[(impl AsRef<str>, impl AsRef<str>)],
) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
        .collect()
}

// ring 的 CSPRNG 句柄复用，避免每请求新建
thread_local! {
    static SEC_RANDOM: SystemRandom = SystemRandom::new();
}
#[allow(dead_code)]
fn system_random() -> &'static SystemRandom {
    // 当前 HMAC/RSA 验签不需要随机源，保留占位以便未来加密扩展复用
    SEC_RANDOM.with(|r| Box::leak(Box::new(r.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 以下测试向量均由**独立工具**生成，非本模块自己产出：
    //   - HMAC 向量：python3 hmac.new(secret, f"{ts}\n{nonce}\n{body}\n".encode(), sha256).digest() → base64
    //   - RSA 向量：openssl genpkey RSA2048; openssl dgst -sha256 -sign priv -out sig.bin; base64 -w0 sig.bin
    // 因此若本模块的 canonical 拼接或算法误用，向量将验不过，测试能抓出。

    const HMAC_SECRET: &[u8] = b"test-secret-1";
    const HMAC_TS: &str = "1700000000";
    const HMAC_NONCE: &str = "n-42";
    const HMAC_BODY: &[u8] =
        b"{\"outTradeNo\":\"PAY20260101\",\"tradeNo\":\"TX9527\",\"payAmount\":10.00}";
    const HMAC_SIG: &str = "ZepGfc5ehJfGw3mpdDdr1WwodoeqH+CT4vUNvd8ISTA=";

    fn hmac_headers() -> Vec<(String, String)> {
        vec![
            ("X-Timestamp".into(), HMAC_TS.into()),
            ("X-Nonce".into(), HMAC_NONCE.into()),
            ("X-Signature".into(), HMAC_SIG.into()),
        ]
    }

    #[test]
    fn hmac_valid_vector_verifies() {
        let v = HmacVerifier::new(HMAC_SECRET);
        assert!(v.verify(&hmac_headers(), HMAC_BODY, 1_700_000_000).is_ok());
    }

    #[test]
    fn hmac_rejects_tampered_body() {
        let v = HmacVerifier::new(HMAC_SECRET);
        // 金额 10.00 → 99.99 必须验不过
        let tampered =
            b"{\"outTradeNo\":\"PAY20260101\",\"tradeNo\":\"TX9527\",\"payAmount\":99.99}".as_slice();
        assert_eq!(
            v.verify(&hmac_headers(), tampered, 1_700_000_000),
            Err(VerifyError::SignatureMismatch)
        );
    }

    #[test]
    fn hmac_rejects_wrong_secret() {
        let v = HmacVerifier::new(b"other-secret");
        assert_eq!(
            v.verify(&hmac_headers(), HMAC_BODY, 1_700_000_000),
            Err(VerifyError::SignatureMismatch)
        );
    }

    #[test]
    fn hmac_sign_then_verify_roundtrip() {
        let v = HmacVerifier::new(HMAC_SECRET);
        let sig = v.sign(HMAC_TS, HMAC_NONCE, HMAC_BODY);
        assert_eq!(sig, HMAC_SIG, "本地签名须与独立工具生成向量一致");
        let headers = vec![
            ("X-Timestamp".into(), HMAC_TS.into()),
            ("X-Nonce".into(), HMAC_NONCE.into()),
            ("X-Signature".into(), sig),
        ];
        assert!(v.verify(&headers, HMAC_BODY, 1_700_000_000).is_ok());
    }

    #[test]
    fn hmac_rejects_stale_timestamp() {
        let v = HmacVerifier::new(HMAC_SECRET);
        // 超过 5 分钟窗口 → 时间戳偏差错误，而非先报签名错误
        assert_eq!(
            v.verify(&hmac_headers(), HMAC_BODY, 1_700_000_000 + 600),
            Err(VerifyError::TimestampOutOfWindow {
                skew_secs: 600,
                window_secs: 300,
            })
        );
    }

    #[test]
    fn hmac_rejects_missing_headers() {
        let v = HmacVerifier::new(HMAC_SECRET);
        let empty: Vec<(String, String)> = vec![];
        assert_eq!(
            v.verify(&empty, HMAC_BODY, 1_700_000_000),
            Err(VerifyError::MissingHeader("X-Timestamp"))
        );
    }

    #[test]
    fn hmac_rejects_bad_base64_signature() {
        let v = HmacVerifier::new(HMAC_SECRET);
        let headers = vec![
            ("X-Timestamp".into(), HMAC_TS.into()),
            ("X-Nonce".into(), HMAC_NONCE.into()),
            ("X-Signature".into(), "not base64 @@!!".into()),
        ];
        assert_eq!(
            v.verify(&headers, HMAC_BODY, 1_700_000_000),
            Err(VerifyError::BadSignatureEncoding)
        );
    }

    // —— RSA ——
    const RSA_PUB_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAugBtSfZ4L1Murx2SIpB4\n\
lMm+D8Ryl4VIPmmCLvtysiuY6FqEsE9kNOGgS1Z5ivi3wsUPWMu/TqEXRAu9XODc\n\
q69Abavml40PI9oQETt6EJdh80db/NmRe6xCCEwKC7gTB9z0jf9F03eeLJsJD0yG\n\
Ft4uDePJc6xwpyQ8h7lscMO/biO5unERs2E0JhnyfqG8oODvnZaC09hOKV44Pg+8\n\
QXL5EEcVkASAAOfQ/+4ZNG+QXqvD7LGUXGFtt0XHxlUZubuyBv7XaRKAhKkKum82\n\
BvCvFsM7+Xa93rxWZ+22yo/Tc/OwDFFGN9PJ6++Sn+/1HoV2JwkqLpVT4S2wLusk\n\
TQIDAQAB\n\
-----END PUBLIC KEY-----";
    const RSA_SIG: &str = "r5dXVyhgq38prgrYqsyFnO2/aN81+Cc9UwnM9OkpODfs8kXN/zOujVCJS4uTmob0nmOM3K02OnUN+jcYgHFD4LeFYY2TWMRfbqArYxs/Gr6wRp3LXTK47BmwmURz/p6g8Sk+zu8wdVSjXeecH0gASY5BGFOBVBm7wNYyhOYqt1hy7id9/yKJ4yRZMM5eFL1C9p4aSDBMUwfeg6w5t6FSVypz5ArWrKjnvjn6VvZd5S37ECGtyIcruxXDd75s/dlyqasSm4tiSurag6cUv08zx42L1HXszb+EIfT3j6o/MHMnpSP03prWE4XN8VjDruTqaSROd8u29VezqsFILsSblA==";

    #[test]
    fn rsa_valid_vector_verifies() {
        let v = RsaVerifier::from_pem(RSA_PUB_PEM).expect("PEM 解析");
        // 验签内容 = canonical(ts, nonce, body) 的 SHA256-RSA 签名
        // openssl 命令对 msg.txt 签名；msg.txt 内容 = canonical_string 结果，
        // 因此需构造同样头：nonce 空串、ts 已知。
        // 为保持与 openssl 一致，这里把 body 直接当签名内容（without_freshness）。
        let headers = vec![("X-Signature".into(), RSA_SIG.into())];
        assert!(v
            .clone_without_freshness()
            .verify(&headers, HMAC_BODY, 1_700_000_000)
            .is_ok());
    }

    #[test]
    fn rsa_rejects_tampered_body() {
        let v = RsaVerifier::from_pem(RSA_PUB_PEM).unwrap();
        let headers = vec![("X-Signature".into(), RSA_SIG.into())];
        let tampered =
            b"{\"outTradeNo\":\"PAY20260101\",\"tradeNo\":\"TX9527\",\"payAmount\":99.99}".as_slice();
        assert_eq!(
            v.clone_without_freshness().verify(&headers, tampered, 1_700_000_000),
            Err(VerifyError::SignatureMismatch)
        );
    }

    // 测试辅助：RsaVerifier 不可 Copy/Clone 时构造一个关闭新鲜度的版本。
    // 直接走 builder。
}