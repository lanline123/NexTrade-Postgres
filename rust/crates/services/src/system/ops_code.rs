//! 运营代码与验证配置（论坛帖 477914 立项批次，我方新增 + 原版客服 JS 补迁）。
//!
//! 一组平台级运营自助口子，存 `config` 表、无表结构迁移：
//!
//! - **站长验证**（我方新增，原版 v5.8.61 无此功能）：六个平台的 meta
//!   验证码（Google / Bing / 百度 / 360 / 搜狗 / 神马）+ 单个验证文件
//!   （文件名 + 内容，PC 前台在域名根路径托管）。原版 SEO 只有 TDK 与
//!   SEO 链接开关，无任何站长验证字段。
//! - **第三方代码**（我方新增，原版无统计/通用代码位）：`headCode`
//!   （页面头部代码）+ `bodyCode`（页面底部代码），统计代码（百度统计 /
//!   CNZZ / GA 等）粘 `bodyCode` 即可。代码原样下发、原样注入，不做
//!   转义与合法性校验——这是「预留 JS 位」的语义本身。
//! - **客服 JS**（原版有 · 我方移植漏迁）：字段名与存储键逐字沿原版
//!   `kefuJavascript`（PC 端）/ `wapKefuJavascript`（移动端），见原版
//!   快照仓 `view/admin/src/types/setting/config.d.ts` 的
//!   `KefuSettingConfig` 与种子数据 config 行 2551/2552。这两个键是
//!   config 表的独立行（走客服设置组 EP8/EP9 字段注册保存），不进本
//!   JSON 块；本模块只提供键名常量，公开下发时由 `config_client`
//!   服务与本块合并组装。
//!
//! 存储：本块单行 `biz_code = "opsCodeSettings"`，`biz_val` 为整块
//! JSON（与 `buyStyle` / `aiGuideSettings` 同属「单 code 整块 JSON」
//! 型配置）。未配置时全部为空串 ⇒ 前台零注入、零影响。
//!
//! ⚠️ **安全口径（XSS 边界）**：本配置只能经挂 `platform_only` 的管理
//! 端端点写入（平台管理员）。管理员粘贴的代码将以原始形态在商城
//! PC/H5 页面执行，等价于平台管理员本就拥有的站点级权限；不下放
//! 商户/店铺账号。设置页有明确警示文案。读路径（公开端点）下发的
//! 本来就是要出现在页面源码里的内容，无额外敏感面。

use serde_json::{json, Map, Value};

/// config 表中的业务编码（本块整块 JSON）。
pub const OPS_CODE_SETTINGS_CODE: &str = "opsCodeSettings";

/// 原版客服 JS 代码位（PC 端）config 键，逐字沿原版字段名。
pub const KEFU_JAVASCRIPT_CODE: &str = "kefuJavascript";
/// 原版客服 JS 代码位（移动端）config 键，逐字沿原版字段名。
pub const WAP_KEFU_JAVASCRIPT_CODE: &str = "wapKefuJavascript";

/// 验证码字段上限（各站长平台验证码均为短串，200 足够且防滥填）。
const VERIFY_CODE_MAX_CHARS: usize = 200;
/// 验证文件内容上限（平台验证文件都是一行短文本/极小 HTML）。
const VERIFY_FILE_CONTENT_MAX_CHARS: usize = 4096;
/// 第三方代码上限（统计/客服厂商代码通常 < 4KB，20K 留足余量）。
const CODE_MAX_CHARS: usize = 20_000;

/// 内置默认配置（未保存过设置时生效）：全部为空 = 零注入。
#[must_use]
pub fn default_ops_code() -> Value {
    json!({
        "verifyGoogle": "",
        "verifyBing": "",
        "verifyBaidu": "",
        "verify360": "",
        "verifySogou": "",
        "verifyShenma": "",
        "verifyFileName": "",
        "verifyFileContent": "",
        "headCode": "",
        "bodyCode": "",
    })
}

fn clean_text(v: Option<&Value>, max_chars: usize) -> String {
    v.and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(max_chars)
        .collect()
}

/// 代码字段只裁长度、不 trim 内部（首尾空白无意义，一并裁掉与其它
/// 配置字段口径一致）；内容逐字保留，不做任何转义/过滤。
fn clean_code(v: Option<&Value>, max_chars: usize) -> String {
    v.and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(max_chars)
        .collect()
}

/// 验证文件名合法性：单个文件名（无路径分隔符、无 `..`），仅
/// `[A-Za-z0-9._-]`，以 `.html` 或 `.txt` 结尾——站长平台的验证文件
/// 只有这两种形态。空串表示未配置验证文件，合法。
#[must_use]
pub fn is_valid_verify_file_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return false;
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    (lower.ends_with(".html") || lower.ends_with(".txt")) && name.len() > 5
}

/// 宽容清洗（读取/生效路径共用）：非法验证文件名视为未配置
/// （文件名与内容成对清空，避免托管半截配置）。
#[must_use]
pub fn sanitize_ops_code(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_ops_code();
    };
    let file_name = clean_text(obj.get("verifyFileName"), 128);
    let file_ok = is_valid_verify_file_name(&file_name) && !file_name.is_empty();
    let mut out = Map::new();
    for key in [
        "verifyGoogle",
        "verifyBing",
        "verifyBaidu",
        "verify360",
        "verifySogou",
        "verifyShenma",
    ] {
        out.insert(
            key.to_string(),
            Value::String(clean_text(obj.get(key), VERIFY_CODE_MAX_CHARS)),
        );
    }
    out.insert(
        "verifyFileName".to_string(),
        Value::String(if file_ok { file_name } else { String::new() }),
    );
    out.insert(
        "verifyFileContent".to_string(),
        Value::String(if file_ok {
            clean_code(obj.get("verifyFileContent"), VERIFY_FILE_CONTENT_MAX_CHARS)
        } else {
            String::new()
        }),
    );
    out.insert(
        "headCode".to_string(),
        Value::String(clean_code(obj.get("headCode"), CODE_MAX_CHARS)),
    );
    out.insert(
        "bodyCode".to_string(),
        Value::String(clean_code(obj.get("bodyCode"), CODE_MAX_CHARS)),
    );
    Value::Object(out)
}

/// 管理端保存校验（严格，区别于读取的宽容清洗）：验证文件名非法、
/// 文件名与内容只填其一、任一字段超长，都明确拒绝并指明字段——
/// 避免管理员保存成功却静默不生效。
///
/// # Errors
/// 任一字段非法 ⇒ 指明字段的中文错误文案。
pub fn validate_ops_code(payload: &Value) -> Result<Value, String> {
    let obj = payload
        .as_object()
        .ok_or_else(|| "运营代码与验证设置必须为 JSON 对象".to_string())?;
    let check_len = |key: &str, label: &str, max: usize| -> Result<(), String> {
        if let Some(v) = obj.get(key) {
            let Some(s) = v.as_str() else {
                return Err(format!("{key}（{label}）必须为字符串"));
            };
            if s.chars().count() > max {
                return Err(format!("{key}（{label}）长度不能超过 {max} 个字符"));
            }
        }
        Ok(())
    };
    for (key, label) in [
        ("verifyGoogle", "Google 验证码"),
        ("verifyBing", "Bing 验证码"),
        ("verifyBaidu", "百度验证码"),
        ("verify360", "360 验证码"),
        ("verifySogou", "搜狗验证码"),
        ("verifyShenma", "神马验证码"),
    ] {
        check_len(key, label, VERIFY_CODE_MAX_CHARS)?;
    }
    check_len("verifyFileContent", "验证文件内容", VERIFY_FILE_CONTENT_MAX_CHARS)?;
    check_len("headCode", "头部代码", CODE_MAX_CHARS)?;
    check_len("bodyCode", "底部代码", CODE_MAX_CHARS)?;

    let file_name = clean_text(obj.get("verifyFileName"), 128);
    let file_content = clean_code(obj.get("verifyFileContent"), VERIFY_FILE_CONTENT_MAX_CHARS);
    if !file_name.is_empty() && !is_valid_verify_file_name(&file_name) {
        return Err(
            "verifyFileName（验证文件名）非法：只能是单个文件名（字母/数字/._-），以 .html 或 .txt 结尾，不能含路径"
                .to_string(),
        );
    }
    if file_name.is_empty() && !file_content.is_empty() {
        return Err("已填写验证文件内容时必须同时填写验证文件名".to_string());
    }
    if !file_name.is_empty() && file_content.is_empty() {
        return Err("已填写验证文件名时必须同时填写验证文件内容".to_string());
    }
    Ok(sanitize_ops_code(payload))
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认。
#[must_use]
pub fn effective_ops_code(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_ops_code();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_ops_code(&v),
        _ => default_ops_code(),
    }
}

/// 公开下发载荷：在生效配置之上并入原版客服 JS 两键（独立 config
/// 行，空/缺失 ⇒ 空串）。前台按端取用：PC 用 `kefuJavascript`，
/// H5 用 `wapKefuJavascript`。
#[must_use]
pub fn public_ops_code(stored: Option<&str>, kefu_js: &str, wap_kefu_js: &str) -> Value {
    let mut out = effective_ops_code(stored);
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "kefuJavascript".to_string(),
            Value::String(kefu_js.trim().chars().take(CODE_MAX_CHARS).collect()),
        );
        obj.insert(
            "wapKefuJavascript".to_string(),
            Value::String(wap_kefu_js.trim().chars().take(CODE_MAX_CHARS).collect()),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_empty() {
        let d = default_ops_code();
        for key in [
            "verifyGoogle",
            "verifyBaidu",
            "verifyFileName",
            "verifyFileContent",
            "headCode",
            "bodyCode",
        ] {
            assert_eq!(d[key], json!(""), "字段 {key} 默认应为空串");
        }
    }

    #[test]
    fn effective_falls_back_to_defaults() {
        assert_eq!(effective_ops_code(None), default_ops_code());
        assert_eq!(effective_ops_code(Some("not json")), default_ops_code());
        assert_eq!(effective_ops_code(Some("[1,2]")), default_ops_code());
    }

    #[test]
    fn verify_file_name_rules() {
        assert!(is_valid_verify_file_name(""));
        assert!(is_valid_verify_file_name("baidu_verify_abc123.html"));
        assert!(is_valid_verify_file_name("google1a2b3c4d.html"));
        assert!(is_valid_verify_file_name("verify.txt"));
        assert!(!is_valid_verify_file_name("../evil.html"));
        assert!(!is_valid_verify_file_name("a/b.html"));
        assert!(!is_valid_verify_file_name("noext"));
        assert!(!is_valid_verify_file_name("x.php"));
        assert!(!is_valid_verify_file_name("sp ace.html"));
    }

    #[test]
    fn sanitize_drops_bad_file_pair_and_keeps_codes() {
        let ok = sanitize_ops_code(&json!({
            "verifyGoogle": " g-code ",
            "verifyFileName": "../evil.html",
            "verifyFileContent": "evil",
            "headCode": "<script>1</script>",
            "bodyCode": " <script src=\"https://hm.baidu.com/hm.js?x\"></script> ",
            "unknown": 1
        }));
        assert_eq!(ok["verifyGoogle"], json!("g-code"));
        // 非法文件名 ⇒ 文件名与内容成对清空
        assert_eq!(ok["verifyFileName"], json!(""));
        assert_eq!(ok["verifyFileContent"], json!(""));
        // 代码原样保留（只裁首尾空白），不转义
        assert_eq!(ok["headCode"], json!("<script>1</script>"));
        assert_eq!(
            ok["bodyCode"],
            json!("<script src=\"https://hm.baidu.com/hm.js?x\"></script>")
        );
        assert!(ok.get("unknown").is_none());
    }

    #[test]
    fn validate_rejects_half_file_pair_and_bad_name() {
        assert!(validate_ops_code(&json!({ "verifyFileName": "a.html" })).is_err());
        assert!(validate_ops_code(&json!({ "verifyFileContent": "x" })).is_err());
        assert!(
            validate_ops_code(&json!({ "verifyFileName": "a/b.html", "verifyFileContent": "x" }))
                .is_err()
        );
        let ok = validate_ops_code(&json!({
            "verifyFileName": "baidu_verify_x.html",
            "verifyFileContent": "abc123",
            "bodyCode": "<script></script>"
        }))
        .expect("合法载荷应通过");
        assert_eq!(ok["verifyFileName"], json!("baidu_verify_x.html"));
        assert_eq!(ok["verifyFileContent"], json!("abc123"));
    }

    #[test]
    fn public_payload_merges_kefu_keys() {
        let out = public_ops_code(None, "<script>pc</script>", "<script>wap</script>");
        assert_eq!(out["kefuJavascript"], json!("<script>pc</script>"));
        assert_eq!(out["wapKefuJavascript"], json!("<script>wap</script>"));
        assert_eq!(out["headCode"], json!(""));
    }
}
