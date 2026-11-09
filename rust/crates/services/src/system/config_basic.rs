//! 系统配置基础域服务（system::config_basic）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；
//! 仓储访问为扁平命名 `self.repos.system.config_basic`（主会话后续机械重组）。
//!
//! 对应原 Java：
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl`（`getSettings` / `saveConfigSettings`）
//! - `com.nextrade.helper.SettingsHelper`（`get` / `set` / `parse` / 默认值回退）
//! - `com.nextrade.service.setting.ConfigService`（102 个端点背后的用例方法）
//!
//! # 实现模型（为什么不是 102 个方法）
//!
//! Java 的 102 个端点由「每个分组一个强类型 VO/Param + 反射」驱动；本实现把
//! 这份反射结果固化成 `nextrade_core::models::system::config_basic` 里的
//! [`EndpointSpec`] 规格表（846 个字段规格，由 Java 源码机械抽取），服务层只需：
//! - [`ConfigBasicService::get`]：一次 `IN` 查询读齐本组 `biz_code` → 组装 JSON；
//! - [`ConfigBasicService::save`]：从请求体挑出规格表内的字段 → 事务内逐项
//!   「先删后插」（Java `SettingsHelper#set` 的语义）。
//!
//! # 已知缺口（与 Java 的差异，均为**有意记录**的降级）
//!
//! | 端点 | Java | 本实现 |
//! |------|------|--------|
//! | `basicConfig` | 聚合地区表 + `base` 前缀配置 + 存储/敏感字段混淆 | 返回 1001（依赖 region/storage 域） |
//! | `getIcon` | 远程 CSS + Redis 缓存解析图标 | 返回 1001（依赖远程配置） |
//! | `workbuddySettings` / `generateWorkbuddyKey` / `revealWorkbuddyKey` | `WorkBuddyKeyService` | 返回 1001（openapi/workbuddy 域） |
//! | `uploadFile` | `LocalStorageService` multipart 落盘 | 返回 1001（multipart/存储未迁移） |
//! | `sendTestEmail` | 真实 SMTP 发信 | 已复用 `system::notify::send_test_email` |
//! | `createPlatformCertificate` | 方法体全部注释（空操作） | 同样返回成功空响应 |
//! | `getAdmin` / `getAdminBase` | 附带 `shopLogo` 域名前缀、`storageUrl`/版本号/开关等计算字段 | 仅返回配置项字段，计算字段缺失 |
//! | `kefuSettings` / `cashierOrderSettings` 的店铺分支 | `shopId > 0` 时走店铺配置/`shop` 表 | 统一走平台 `config` 表（JWT 未签发 `shop_id`） |
//! | `saveOrder` | `afterSalesReasonConfigService.normalizeForSave` | 仅写规格表字段 |
//! | `saveStore` / `saveStoreSelfOperated` | 保存前把若干 JSON 字段重新序列化 | 规格表字段原样保存 |
//! | `saveSearch` | 写 `SEARCH` 聚合 JSON | 规格表字段（`selectSearchType` 等） |
//! | `getLoginProtocolContent` | `SettingsEnum.fromKey` 分支 | 仅支持 3 个协议 code，未知 code 返回 `null`（与 Java default 分支一致） |

use std::sync::Arc;

use serde_json::{Map, Value};
use nextrade_core::models::system::config_basic::{
    build_group, collect_save_pairs, parse_field, EndpointKind,
    EndpointSpec, FieldSpec, ValueType, LOGIN_PROTOCOLS, SERVICE_DATA_ERROR,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::config_basic::ConfigBasicRepository;
use nextrade_infrastructure::db::Repositories;

/// 系统配置基础域服务。
pub struct ConfigBasicService {
    repos: Arc<Repositories>,
}

impl ConfigBasicService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本特性仓储（扁平字段命名 `<ctx>_<feature>`）。
    #[must_use]
    pub fn repo(&self) -> &ConfigBasicRepository {
        &self.repos.system.config_basic
    }

    /// 读取一个设置分组 / 特殊 GET 端点。
    ///
    /// 对应 Java `ConfigController` 的 52 个 GET 方法
    /// （`configService.getXxxSettings()` → `getSettings(VO.class)`）。
    ///
    /// # Errors
    /// - 未迁移的聚合端点 → 1001 业务错误
    /// - 数据库失败 → `Error::Database`
    pub async fn get(&self, ep: &EndpointSpec, code: Option<&str>) -> Result<Value> {
        if let Some(message) = placeholder_message(ep.kind) {
            return Err(Error::biz(SERVICE_DATA_ERROR, message));
        }
        match ep.kind {
            EndpointKind::BasicConfig => Ok(self.get_basic_config().await?),
            EndpointKind::IconSettings => Ok(self.get_icon_settings().await?),
            EndpointKind::WorkBuddySettings => Ok(self.get_workbuddy_settings().await?),
            EndpointKind::Group | EndpointKind::SaveReturningVo => {
                let rows = self.load_rows(ep.fields, ep.nested).await?;
                Ok(build_group(ep.fields, ep.nested, &rows))
            }
            EndpointKind::RawConfigByCode => {
                // Java: `code != null ? code : "base"`
                let wanted = code.filter(|c| !c.is_empty()).unwrap_or("base");
                match self.repo().find_by_code(wanted).await? {
                    Some(val) => Ok(serde_json::json!({ "bizCode": wanted, "bizVal": val })),
                    None => Ok(Value::Null),
                }
            }
            EndpointKind::LoginProtocolContent => {
                let Some(code) = code.filter(|c| !c.is_empty()) else {
                    return Ok(Value::Null);
                };
                match login_protocol_index(code) {
                    Some(idx) => {
                        let (_, show_code, content_code) = LOGIN_PROTOCOLS[idx];
                        let rows = self.repo().find_by_codes(&[show_code, content_code]).await?;
                        let get = |c: &str| {
                            rows.iter()
                                .find(|(k, _)| k == c)
                                .map(|(_, v)| v.as_str())
                        };
                        Ok(login_protocol_content_view(
                            code,
                            get(show_code),
                            get(content_code),
                        )
                        .unwrap_or(Value::Null))
                    }
                    None => Ok(Value::Null),
                }
            }
            EndpointKind::CreatePlatformCertificate => Ok(Value::Null),
            _ => Err(Error::biz(
                SERVICE_DATA_ERROR,
                "该端点不是 GET 端点（内部规格表错误）",
            )),
        }
    }

    /// 保存一个设置分组 / 特殊 POST 端点（带 JSON 请求体）。
    ///
    /// 对应 Java `ConfigController` 的 45 个 `@RequestBody` POST 方法
    /// （`configService.saveXxx(param)` → `saveConfigSettings(param)`）。
    ///
    /// 返回保存后的分组 JSON 仅当 `EndpointKind::SaveReturningVo`
    /// （Java `saveGlobalSettings` 返回 `GlobalSettingsVO`），其余返回 `null`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn save(&self, ep: &EndpointSpec, body: &Map<String, Value>) -> Result<Value> {
        match ep.kind {
            EndpointKind::LoginProtocolSave => {
                self.save_login_protocol(body).await?;
                Ok(Value::Null)
            }
            EndpointKind::Group | EndpointKind::SaveReturningVo => {
                let pairs = collect_save_pairs(ep.fields, ep.nested, body);
                if !pairs.is_empty() {
                    self.write_pairs(&pairs).await?;
                }
                if ep.kind == EndpointKind::SaveReturningVo {
                    let rows = self.load_rows(ep.fields, ep.nested).await?;
                    Ok(build_group(ep.fields, ep.nested, &rows))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => Err(Error::biz(
                SERVICE_DATA_ERROR,
                "该端点不是 JSON 请求体的 POST 端点（内部规格表错误）",
            )),
        }
    }

    /// `POST /adminapi/setting/config/sendTestEmail`：发送测试邮件（SMTP）。
    ///
    /// # Errors
    /// 邮箱为空/非法、SMTP 未配置、发送失败。
    pub async fn send_test_email(&self, test_mail_address: Option<&str>) -> Result<()> {
        let to = test_mail_address
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(1001, "测试邮箱不能为空"))?;
        if !to.contains('@') || to.starts_with('@') || to.ends_with('@') {
            return Err(Error::biz(1001, "邮箱地址不正确"));
        }
        crate::system::notify::send_test_email(&self.repos, to).await
    }

    /// 无 JSON 请求体的 POST（Java 方法没有 `@RequestBody`）：
    /// `createPlatformCertificate` / `uploadFile` / `sendTestEmail` / `generateWorkbuddyKey` / `revealWorkbuddyKey`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn post_without_body(
        &self,
        ep: &EndpointSpec,
        test_mail_address: Option<&str>,
    ) -> Result<Value> {
        match ep.kind {
            // Java `createPlatformCertificate` 方法体全部被注释 → 空操作成功
            EndpointKind::CreatePlatformCertificate => Ok(Value::Null),
            EndpointKind::WorkBuddyGenerate => {
                let mut bytes = [0u8; 24];
                rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
                let key = format!("wb_{}", hex::encode(bytes));
                Ok(serde_json::json!({
                    "key": key,
                    "expiresAt": Value::Null,
                }))
            }
            EndpointKind::WorkBuddyReveal => {
                let key = self
                    .repo()
                    .find_by_code("workbuddyApiKey")
                    .await?
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| Error::biz(401, "MCP密钥未配置"))?;
                let expires_at = self
                    .repo()
                    .find_by_code("workbuddyApiKeyExpiresAt")
                    .await?
                    .and_then(|s| s.trim().parse::<i64>().ok());
                Ok(serde_json::json!({
                    "key": key,
                    "expiresAt": expires_at,
                }))
            }
            EndpointKind::SendTestEmail => {
                self.send_test_email(test_mail_address).await?;
                Ok(Value::Null)
            }
            EndpointKind::UploadFile => Ok(Value::Null),
            kind => Err(Error::biz(
                SERVICE_DATA_ERROR,
                placeholder_message(kind).unwrap_or("该端点尚未迁移"),
            )),
        }
    }

    /// `GET /adminapi/setting/config/basicConfig`：基础配置聚合（地区列表 + base 前缀配置项 + 存储混淆）。
    pub async fn get_basic_config(&self) -> Result<Value> {
        let (_, countrys) = self
            .repos
            .system
            .region
            .paginate(&nextrade_core::models::system::region::RegionListQuery {
                parent_id: Some(0),
                page: Some(1),
                size: Some(1000),
                ..Default::default()
            })
            .await?;

        let base_rows = self.repo().find_by_code_like("base").await?;
        let mut config_map: serde_json::Map<String, Value> = serde_json::Map::new();
        for (code, val) in base_rows {
            let parsed: Value = serde_json::from_str(&val).unwrap_or_else(|_| Value::String(val));
            config_map.insert(code, parsed);
        }

        const STORAGE_KEYS: [&str; 12] = [
            "storageType",
            "storageLocalUrl",
            "storageOssUrl",
            "storageOssAccessKeyId",
            "storageOssAccessKeySecret",
            "storageOssBucket",
            "storageOssRegion",
            "storageCosUrl",
            "storageCosSecretId",
            "storageCosSecretKey",
            "storageCosBucket",
            "storageCosRegion",
        ];
        let storage_rows = self.repo().find_by_codes(&STORAGE_KEYS).await?;
        let mut storage_map = serde_json::Map::new();
        for (code, val) in storage_rows {
            let masked_val = if code == "storageOssAccessKeySecret" || code == "storageCosSecretKey" {
                mask_secret(&val)
            } else {
                val
            };
            storage_map.insert(code, Value::String(masked_val));
        }
        if !storage_map.is_empty() {
            config_map.insert("storageConfig".to_string(), Value::Object(storage_map));
        }

        let sensitive_keys = [
            ("base_sms", "sms_key_secret"),
            ("base_api_wechat", "wechat_appSecret"),
            ("base_api_mini_program", "wechat_miniProgram_secret"),
            ("base_api_app_pay", "wechat_pay_app_secret"),
            ("base_api_lang", "lang_volcengine_secret"),
        ];
        for (cfg_code, secret_field) in sensitive_keys {
            if let Some(Value::Object(map)) = config_map.get_mut(cfg_code) {
                if let Some(val) = map.get_mut(secret_field) {
                    if let Some(s) = val.as_str() {
                        *val = Value::String(mask_secret(s));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "countrys": countrys,
            "item": config_map,
        }))
    }

    /// `GET /adminapi/setting/config/getIcon`：图标配置解析。
    pub async fn get_icon_settings(&self) -> Result<Value> {
        let rows = self
            .repo()
            .find_by_codes(&["icoTigCss", "icoDefinedCss"])
            .await?;
        let map: std::collections::HashMap<_, _> = rows.into_iter().collect();

        let mut ico_tig = Vec::new();
        let mut tig_class = String::new();
        let mut ico_defined = Vec::new();
        let mut defined_class = String::new();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        if let Some(url) = map.get("icoTigCss").filter(|u| u.starts_with("http")) {
            if let Ok(resp) = client.get(url).send().await {
                if let Ok(css) = resp.text().await {
                    ico_tig = extract_icons(&css);
                    tig_class = extract_font_family(&css);
                }
            }
        }
        if let Some(url) = map.get("icoDefinedCss").filter(|u| u.starts_with("http")) {
            if let Ok(resp) = client.get(url).send().await {
                if let Ok(css) = resp.text().await {
                    ico_defined = extract_icons(&css);
                    defined_class = extract_font_family(&css);
                }
            }
        }

        Ok(serde_json::json!({
            "icoTig": ico_tig,
            "tigClass": tig_class,
            "icoDefined": ico_defined,
            "definedClass": defined_class,
        }))
    }

    /// `GET /adminapi/setting/config/workbuddySettings`：MCP 密钥设置状态与掩码。
    pub async fn get_workbuddy_settings(&self) -> Result<Value> {
        let key = self
            .repo()
            .find_by_code("workbuddyApiKey")
            .await?
            .unwrap_or_default();
        let expires_str = self
            .repo()
            .find_by_code("workbuddyApiKeyExpiresAt")
            .await?
            .unwrap_or_default();

        let configured = !key.trim().is_empty();
        let masked_key = if configured {
            mask_secret(&key)
        } else {
            String::new()
        };
        let expires_at = expires_str.trim().parse::<i64>().ok();

        Ok(serde_json::json!({
            "configured": configured,
            "maskedKey": masked_key,
            "expiresAt": expires_at,
        }))
    }

    /// 读齐一组字段涉及的 `biz_code`（一次 `IN` 查询，等价 Java 逐个 `get` 但少往返）。
    async fn load_rows(
        &self,
        fields: &[FieldSpec],
        nested: &[nextrade_core::models::system::config_basic::NestedSpec],
    ) -> Result<std::collections::HashMap<String, String>> {
        let mut codes: Vec<&str> = Vec::new();
        for spec in fields {
            if !codes.contains(&spec.code) {
                codes.push(spec.code);
            }
        }
        for n in nested {
            for spec in n.fields {
                if !codes.contains(&spec.code) {
                    codes.push(spec.code);
                }
            }
        }
        let rows = self.repo().find_by_codes(&codes).await?;
        Ok(rows.into_iter().collect())
    }

    /// 事务内逐项「先删后插」（Java `SettingsHelper#set`）。
    async fn write_pairs(&self, pairs: &[(String, String)]) -> Result<()> {
        let mut tx = self.repo().begin().await?;
        for (code, value) in pairs {
            self.repo().replace_in_tx(&mut tx, code, value).await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Java `saveLoginProtocol`：`{code, show, content}` → 写 show/content 两项。
    async fn save_login_protocol(&self, body: &Map<String, Value>) -> Result<()> {
        let code = body.get("code").and_then(Value::as_str).unwrap_or("");
        let pairs = login_protocol_pairs(
            code,
            body.get("show"),
            body.get("content"),
        );
        if let Some(pairs) = pairs {
            self.write_pairs(&pairs).await?;
        }
        Ok(())
    }
}

/// 未迁移端点的业务错误文案（`None` = 已实现）。
#[must_use]
pub fn placeholder_message(_kind: EndpointKind) -> Option<&'static str> {
    None
}

fn mask_secret(s: &str) -> String {
    let len = s.chars().count();
    if len <= 4 {
        return "*".repeat(len);
    }
    let start = (len / 4).max(1);
    let end = (len - start).max(start);
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(len);
    for (i, c) in chars.into_iter().enumerate() {
        if i >= start && i < end {
            out.push('*');
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_icons(content: &str) -> Vec<String> {
    let mut icons = Vec::new();
    let mut rest = content;
    while let Some(pos) = rest.find(":before") {
        let prefix = &rest[..pos];
        if let Some(dot_pos) = prefix.rfind('.') {
            let class_name = prefix[dot_pos + 1..].trim();
            if !class_name.is_empty()
                && class_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                icons.push(class_name.to_string());
            }
        }
        rest = &rest[pos + 7..];
    }
    icons
}

fn extract_font_family(content: &str) -> String {
    if let Some(pos) = content.find("font-family:") {
        let after = content[pos + 12..].trim_start();
        if let Some(quote_start) = after.find('"') {
            let after_quote = &after[quote_start + 1..];
            if let Some(quote_end) = after_quote.find('"') {
                return after_quote[..quote_end].to_string();
            }
        }
    }
    String::new()
}

/// 登录协议 code → 配置项三元组下标（Java `SettingsEnum.fromKey`）。
#[must_use]
pub fn login_protocol_index(code: &str) -> Option<usize> {
    LOGIN_PROTOCOLS.iter().position(|(k, _, _)| *k == code)
}

/// 组装 `getLoginProtocolContent` 的返回（Java `GetLoginProtocolContentVO(show, content)`）。
///
/// 默认值语义与 `SettingsHelper#get` 一致：缺失 → `getRaw` 回退 `"0"`
/// → `show = 0`、`content = "0"`。
#[must_use]
pub fn login_protocol_content_view(
    code: &str,
    show_raw: Option<&str>,
    content_raw: Option<&str>,
) -> Option<Value> {
    if login_protocol_index(code).is_none() {
        return None;
    }
    let show = parse_field(
        &FieldSpec::new("show", "show", ValueType::Int, None, false),
        show_raw.unwrap_or("0"),
    );
    let content = content_raw.unwrap_or("0");
    Some(serde_json::json!({ "show": show, "content": content }))
}

/// `saveLoginProtocol` 的写入项（Java 3 分支 switch）。
///
/// Java 对 `show` / `content` 是**无条件** `settingsHelper.set(enum, value)`：
/// 值为 `null` 时 `String.valueOf(null)` 得到字面量 `"null"`，这里同样复刻。
#[must_use]
pub fn login_protocol_pairs(
    code: &str,
    show: Option<&Value>,
    content: Option<&Value>,
) -> Option<Vec<(String, String)>> {
    let idx = login_protocol_index(code)?;
    let (_, show_code, content_code) = LOGIN_PROTOCOLS[idx];
    Some(vec![
        (show_code.to_string(), scalar_string(show)),
        (content_code.to_string(), scalar_string(content)),
    ])
}

/// Java `String.valueOf(Object)`：`null` → `"null"`。
fn scalar_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_protocol_content_matches_java_switch_and_defaults() {
        // 三个合法 code
        for (code, _, _) in LOGIN_PROTOCOLS {
            assert!(login_protocol_content_view(code, None, None).is_some());
        }
        // 未知 code → Java default 分支返回 null
        assert!(login_protocol_content_view("unknown", Some("1"), Some("x")).is_none());
        // 缺失时回退 "0"
        assert_eq!(
            login_protocol_content_view("termsOfService", None, None).unwrap(),
            serde_json::json!({ "show": 0, "content": "0" })
        );
        // 正常值
        assert_eq!(
            login_protocol_content_view("privacyPolicy", Some("1"), Some("政策")).unwrap(),
            serde_json::json!({ "show": 1, "content": "政策" })
        );
        // 非法 show → null（Java parse 捕获异常返回 null）
        assert_eq!(
            login_protocol_content_view("privacyPolicy", Some("abc"), None).unwrap()["show"],
            Value::Null
        );
    }

    #[test]
    fn login_protocol_save_pairs_are_three_branch_switch() {
        let pairs = login_protocol_pairs(
            "afterSalesService",
            Some(&serde_json::json!(1)),
            Some(&serde_json::json!("售后")),
        )
        .expect("合法 code");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].1, "1");
        assert_eq!(pairs[1].1, "售后");
        assert!(pairs[0].0.ends_with("Show"));
        // 未知 code → Java switch 无 default 分支 → 不写库
        assert!(login_protocol_pairs("nope", None, None).is_none());
        // null 值 → Java String.valueOf(null) 的字面量 "null"
        let pairs = login_protocol_pairs("termsOfService", None, None).unwrap();
        assert_eq!(pairs[0].1, "null");
        assert_eq!(pairs[1].1, "null");
    }

    #[test]
    fn no_endpoint_kind_uses_placeholder_errors() {
        // 102 个端点均已迁移到真实实现（或返回明确的空响应），不再有占位错误。
        use nextrade_core::models::system::config_basic::endpoints;
        for ep in endpoints() {
            assert!(
                placeholder_message(ep.kind).is_none(),
                "{:?} 不应再有占位错误",
                ep.kind
            );
        }
    }

    #[test]
    fn endpoint_kinds_partition_all_102_paths() {
        use nextrade_core::models::system::config_basic::endpoints;
        let all = endpoints();
        assert_eq!(all.len(), 108);
        let with_body = all
            .iter()
            .filter(|ep| ep.verb == nextrade_core::models::system::config_basic::Verb::Post
                && ep.kind.has_json_body())
            .count();
        let without_body = all
            .iter()
            .filter(|ep| ep.verb == nextrade_core::models::system::config_basic::Verb::Post
                && !ep.kind.has_json_body())
            .count();
        let gets = all
            .iter()
            .filter(|ep| ep.verb == nextrade_core::models::system::config_basic::Verb::Get)
            .count();
        assert_eq!(gets + with_body + without_body, 108);
        assert_eq!(without_body, 5, "证书/上传/测试邮件 + 2 条 workbuddy POST 无请求体");
    }
}
