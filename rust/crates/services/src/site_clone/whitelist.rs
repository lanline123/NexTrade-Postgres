//! 设置白名单注册表（站点搬家安全核心，设计报告第四节）。
//!
//! 原则：**白名单导出，不是黑名单剔除**。只有本注册表登记的设置组/块才
//! 允许进搬家包；规格表（`config_basic::ENDPOINTS`）里新增的任何字段默认
//! 不导出，必须人工评审后登记才放行。导出端按本表过滤一次，导入端落库前
//! 按同一份表再过滤一次（防源站被篡改/版本差异发来不该要的字段）。
//!
//! 三档处置（与 `DOMAINS.md` 一/二节逐项对应）：
//! - ✅ 整组可复制（`risk=false, deny_fields=[]`）；
//! - 🔑 字段级过滤后可复制（`deny_fields` 显式名单 + 全局敏感名模式兜底）；
//! - ⚠️ 可复制但默认不勾（`risk=true`，费率类，预览带 ⚠️、须显式纳入）。
//!
//! ⛔ 豁免组（全部支付渠道、mailSettings、profitSharingSettings、
//! offlinePaySettings、getWXWorkConfig、getDingConfig、wechatLiveSettings
//! 与独立块 federationSettings / aiMcpSettings / aiGuideSettings /
//! pcDomain / opsCodeSettings / paySettings / apiSettings）**根本不出现
//! 在本注册表中**——未登记即不可导出，这是白名单的题中之义。

use serde_json::Value;

/// 一个可复制设置组的规则。
#[derive(Debug, Clone, Copy)]
pub struct GroupRule {
    /// 注册表 key（设置包/预览/勾选的统一标识）。
    pub key: &'static str,
    /// 中文名（预览清单展示）。
    pub label: &'static str,
    /// 规格表 GET 端点路径。
    pub get_path: &'static str,
    /// 规格表 POST（保存）端点路径。
    pub save_path: &'static str,
    /// ⚠️ 风险组（费率类）：默认不勾选，导入须显式纳入。
    pub risk: bool,
    /// 字段级剔除名单（按导出的 JSON 字段名匹配，递归生效）。
    pub deny_fields: &'static [&'static str],
}

const fn rule(
    key: &'static str,
    label: &'static str,
    get_path: &'static str,
    save_path: &'static str,
    risk: bool,
    deny_fields: &'static [&'static str],
) -> GroupRule {
    GroupRule { key, label, get_path, save_path, risk, deny_fields }
}

/// 登录组剔除名单（设计实证：AppId/Secret 与第三方登录 client 凭据类）。
const LOGIN_DENY: &[&str] = &[
    "wechatAppId",
    "wechatAppSecret",
    "wechatServerToken",
    "wechatServerSecret",
    "facebookClientId",
    "facebookClientSecret",
    "googleClientId",
    "googleClientSecret",
    "twitterClientId",
    "twitterClientSecret",
];

/// 通知组剔除名单（短信通道凭据类；服务商选择/签名/开关保留）。
const NOTIFY_DENY: &[&str] = &[
    "smsKeyId",
    "smsKeySecret",
    "smsTencentSecretId",
    "smsTencentSecretKey",
    "smsTencentSdkAppId",
    "smsHuaweiAppKey",
    "smsHuaweiAppSecret",
    "smsVolcengineAccessKey",
    "smsVolcengineSecretKey",
    "smsVolcengineSmsAccount",
];

/// 地图组剔除名单（高德/天地图 key 与 tk 全部剔除；服务商选择保留）。
const MAP_DENY: &[&str] = &[
    "amapWebKey",
    "amapWebSecret",
    "amapJsKey",
    "amapJsSecret",
    "amapMiniKey",
    "amapMiniSecret",
    "tiandituWebTk",
    "tiandituJsTk",
];

/// 商户组剔除名单（对接密钥字段，代码实证存在于保存规格中）。
const MERCHANT_DENY: &[&str] = &["merchantApplicationKey"];

/// 全局组剔除名单（存储/翻译通道凭据混在全局组字段里，代码实证；
/// 敏感名模式亦可兜底，此处显式登记为纵深防御）。
const GLOBAL_DENY: &[&str] = &[
    "storageOssAccessKeyId",
    "storageOssAccessKeySecret",
    "storageCosSecretId",
    "storageCosSecretKey",
    "storageQiniuAccessKey",
    "storageQiniuSecretKey",
    "storageS3AccessKey",
    "storageS3SecretKey",
    "langVolcengineAccessKey",
    "langVolcengineSecret",
];

/// 配送组剔除名单（快递鸟/快递100 账号 key，代码实证）。
const SHIPPING_DENY: &[&str] = &["kdniaoApiKey", "kuaidi100Key", "kuaidi100Secret"];

/// 小程序风控组剔除名单（直播小程序 AppSecret，代码实证）。
const MINI_RISK_DENY: &[&str] = &["wechatLiveMiniAppSecret"];

/// P1 白名单设置组（32 组，与 DOMAINS.md 第一节逐项对应；
/// `shoppingSettings` 交易设置在设计表中未单列，经字段级核查无凭据字段，
/// 按 ✅ 纳入——见实现报告说明）。
pub const GROUP_RULES: &[GroupRule] = &[
    rule("basic", "基础信息", "/setting/config/basicSettings", "/setting/config/saveBasic", false, &[]),
    rule("global", "全局设置", "/setting/config/globalSettings", "/setting/config/saveGlobal", false, GLOBAL_DENY),
    rule("shop", "店铺设置", "/setting/config/shopSettings", "/setting/config/saveShop", false, &[]),
    rule("shopping", "交易设置", "/setting/config/shoppingSettings", "/setting/config/saveShopping", false, &[]),
    rule("store", "门店设置", "/setting/config/storeSettings", "/setting/config/saveStore", false, &[]),
    rule("storeSelfOperated", "自营门店设置", "/setting/config/storeSelfOperatedSettings", "/setting/config/saveStoreSelfOperated", false, &[]),
    rule("merchant", "商户入驻设置", "/setting/config/merchantSettings", "/setting/config/saveMerchant", false, MERCHANT_DENY),
    rule("vendor", "供应商设置", "/setting/config/vendorSettings", "/setting/config/saveVendor", false, &[]),
    rule("themeStyle", "主题风格", "/setting/config/themeStyleSettings", "/setting/config/saveThemeStyle", false, &[]),
    rule("login", "登录设置", "/setting/config/loginSettings", "/setting/config/saveLogin", false, LOGIN_DENY),
    rule("auth", "认证设置", "/setting/config/authSettings", "/setting/config/saveAuth", false, &[]),
    rule("search", "搜索设置", "/setting/config/searchSettings", "/setting/config/saveSearch", false, &[]),
    rule("searchStyle", "搜索样式", "/setting/config/searchStyleSettings", "/setting/config/saveSearchStyle", false, &[]),
    rule("mobileSearchStyle", "移动搜索样式", "/setting/config/mobileSearchStyleSettings", "/setting/config/saveMobileSearchStyle", false, &[]),
    rule("product", "商品设置", "/setting/config/productSettings", "/setting/config/saveProduct", false, &[]),
    rule("productDetailStyle", "商品详情样式", "/setting/config/productDetailStyleSettings", "/setting/config/saveProductDetailStyle", false, &[]),
    rule("categoryDecorate", "分类页装修设置", "/setting/config/categoryDecorateSettings", "/setting/config/saveCategoryDecorate", false, &[]),
    rule("memberCenterDecorate", "会员中心装修设置", "/setting/config/memberCenterDecorateSettings", "/setting/config/saveMemberCenterDecorate", false, &[]),
    rule("buyerShow", "买家秀设置", "/setting/config/buyerShowSettings", "/setting/config/saveBuyerShow", false, &[]),
    rule("order", "订单设置", "/setting/config/orderSettings", "/setting/config/saveOrder", false, &[]),
    rule("afterSales", "售后设置", "/setting/config/afterSalesSettings", "/setting/config/saveAfterSales", false, &[]),
    rule("shipping", "配送设置", "/setting/config/shippingSettings", "/setting/config/saveShipping", false, SHIPPING_DENY),
    rule("kefu", "客服设置", "/setting/config/kefuSettings", "/setting/config/saveKefu", false, &[]),
    rule("notify", "通知设置", "/setting/config/notifySettings", "/setting/config/saveNotify", false, NOTIFY_DENY),
    rule("map", "地图设置", "/setting/config/mapSettings", "/setting/config/saveMapSettings", false, MAP_DENY),
    rule("amapAll", "高德地图设置", "/setting/config/amapAllSettings", "/setting/config/saveAllAmapSettings", false, MAP_DENY),
    rule("personalized", "个性化推荐设置", "/setting/config/getPersonalizedSetting", "/setting/config/savePersonalizedSetting", false, &[]),
    rule("miniProgramRisk", "小程序风控设置", "/setting/config/miniProgramRiskSettings", "/setting/config/saveMiniProgramRisk", false, MINI_RISK_DENY),
    rule("cashierOrder", "收银台点单设置", "/setting/config/getCashierOrderSettings", "/setting/config/saveCashierOrderSettings", false, &[]),
    rule("withdrawal", "提现设置", "/setting/config/withdrawalSettings", "/setting/config/saveWithdrawal", true, &[]),
    rule("memberWithdrawal", "会员提现设置", "/setting/config/memberWithdrawalSettings", "/setting/config/saveMemberWithdrawal", true, &[]),
    rule("commissionWithdrawal", "分销提现设置", "/setting/config/commissionWithdrawalSettings", "/setting/config/saveCommissionWithdrawal", true, &[]),
];

/// 独立 JSON 块（整块覆盖，✅）：biz_val 为整块 JSON。
pub const JSON_BLOCK_CODES: &[&str] = &[
    "adSettings",
    "buyStyle",
    "priceDisplay",
    "fontSettings",
    "auctionSettings",
    "icoDefinedCss",
    "systemMode",
];

/// 独立原始字符串块（整行覆盖）：App 版本四码 + 登录协议显隐与正文。
/// 协议正文是文本内容（设计口径 ✅），显隐开关与之一并搬运。
pub const RAW_BLOCK_CODES: &[&str] = &[
    "androidVersion",
    "iosVersion",
    "iosLink",
    "androidLink",
    "termsOfServiceShow",
    "privacyPolicyShow",
    "afterSalesServiceShow",
    "termsOfService",
    "privacyPolicy",
    "afterSalesService",
];

/// 按 key 查组规则。
#[must_use]
pub fn group_rule(key: &str) -> Option<&'static GroupRule> {
    GROUP_RULES.iter().find(|r| r.key == key)
}

/// code 是否为白名单独立块（JSON 或原始字符串）。
#[must_use]
pub fn is_block_code(code: &str) -> bool {
    JSON_BLOCK_CODES.contains(&code) || RAW_BLOCK_CODES.contains(&code)
}

/// 字段名是否命中敏感模式（凭据类命名的兜底护栏，导出/导入双侧共用）。
///
/// 模式（小写包含匹配）：`secret` / `password` / `passwd` / `privatekey` /
/// `private_key` / `token` / `apikey` / `api_key` / `accesskey` /
/// `access_key` / `secretkey` / `credential` / 含 `key`（但 `keyword(s)`
/// 是搜索词不是密钥，显式豁免——这是对 DOMAINS.md `*Key*` 模式的误伤
/// 修正，断言测试与本函数共用同一谓词，保证两侧口径一致）。
#[must_use]
pub fn is_sensitive_field(field: &str) -> bool {
    let lower = field.to_ascii_lowercase();
    if lower.contains("keyword") {
        return false;
    }
    lower.contains("secret")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("privatekey")
        || lower.contains("private_key")
        || lower.contains("token")
        || lower.contains("apikey")
        || lower.contains("api_key")
        || lower.contains("accesskey")
        || lower.contains("access_key")
        || lower.contains("secretkey")
        || lower.contains("credential")
        || lower.contains("key")
}

/// 递归过滤一个设置组 JSON：剔除 `deny_fields` 命中与敏感模式命中的字段
/// （嵌套对象同样过滤）。标量/数组原样保留（数组内对象元素亦过滤）。
#[must_use]
pub fn filter_group_value(value: &Value, deny_fields: &[&str]) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                if deny_fields.contains(&k.as_str()) || is_sensitive_field(k) {
                    continue;
                }
                out.insert(k.clone(), filter_group_value(v, deny_fields));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(
            items.iter().map(|v| filter_group_value(v, deny_fields)).collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::system::config_basic::{find_endpoint, EndpointKind, Verb};

    #[test]
    fn sensitive_pattern_catches_credentials_not_keywords() {
        for f in [
            "wechatAppSecret",
            "smsKeySecret",
            "stripeSecretKey",
            "xenditSecretKey",
            "payChannelGatewayKey",
            "antomMerchantPrivateKey",
            "smtpPassword",
            "wechatServerToken",
            "amapWebKey",
            "smsVolcengineAccessKey",
            "apiKey",
        ] {
            assert!(is_sensitive_field(f), "应拦截: {f}");
        }
        for f in ["shopName", "themeColor", "searchKeywords", "hotKeyword", "integralScale"] {
            assert!(!is_sensitive_field(f), "不应拦截: {f}");
        }
    }

    #[test]
    fn filter_strips_denied_and_sensitive_recursively() {
        let v = serde_json::json!({
            "shopName": "A站",
            "wechatAppSecret": "s3cr3t",
            "nested": { "smsKeyId": "k", "smsSignName": "签名" },
            "list": [{ "accessToken": "t", "label": "x" }]
        });
        let out = filter_group_value(&v, LOGIN_DENY);
        assert_eq!(out["shopName"], serde_json::json!("A站"));
        assert!(out.get("wechatAppSecret").is_none());
        assert!(out["nested"].get("smsKeyId").is_none());
        assert_eq!(out["nested"]["smsSignName"], serde_json::json!("签名"));
        assert!(out["list"][0].get("accessToken").is_none());
        assert_eq!(out["list"][0]["label"], serde_json::json!("x"));
    }

    /// DOMAINS.md 附册二的断言模式（*Secret* / *Key* / *Password* /
    /// *PrivateKey* / *Token* 的小写包含匹配；keyword 是搜索词不是密钥，
    /// 与运行时谓词同一豁免）。
    fn matches_design_pattern(field: &str) -> bool {
        let lower = field.to_ascii_lowercase();
        if lower.contains("keyword") {
            return false;
        }
        lower.contains("secret")
            || lower.contains("password")
            || lower.contains("privatekey")
            || lower.contains("token")
            || lower.contains("key")
    }

    /// 防回归护栏：遍历注册表全部组，把规格表全部字段拼成一份模拟导出
    /// JSON 过一遍导出过滤，断言**过滤后**没有任何字段名再命中设计模式，
    /// 且已知的凭据字段（存储 key/短信 key/支付类）确实被剔除。
    /// 同时校验注册表的 GET/POST 规格存在且形态正确（Group 组）。
    #[test]
    fn no_credential_field_survives_export_filter() {
        for rule in GROUP_RULES {
            let spec = find_endpoint(rule.get_path)
                .unwrap_or_else(|| panic!("GET 规格应存在: {}", rule.get_path));
            assert_eq!(spec.verb, Verb::Get, "{} 应为 GET 端点", rule.get_path);
            assert_eq!(spec.kind, EndpointKind::Group, "{} 应为 Group 组", rule.get_path);
            let save = find_endpoint(rule.save_path)
                .unwrap_or_else(|| panic!("POST 规格应存在: {}", rule.save_path));
            assert_eq!(save.verb, Verb::Post, "{} 应为 POST 端点", rule.save_path);
            let mut obj = serde_json::Map::new();
            for f in spec.fields {
                obj.insert(f.field.to_string(), Value::String("x".into()));
            }
            for n in spec.nested {
                for f in n.fields {
                    obj.insert(f.field.to_string(), Value::String("x".into()));
                }
            }
            let filtered = filter_group_value(&Value::Object(obj), rule.deny_fields);
            let out = filtered.as_object().expect("组导出应为对象");
            for name in out.keys() {
                assert!(
                    !matches_design_pattern(name),
                    "组 {} 过滤后仍残留凭据模式字段 {name}",
                    rule.key
                );
            }
        }
        // 已知凭据字段的定向断言（组, 字段）：必须被剔除名单或模式过滤命中。
        const KNOWN_CREDENTIALS: &[(&str, &str)] = &[
            ("global", "storageOssAccessKeySecret"),
            ("global", "storageS3SecretKey"),
            ("login", "wechatAppSecret"),
            ("notify", "smsKeySecret"),
            ("notify", "smsTencentSecretKey"),
            ("shipping", "kdniaoApiKey"),
            ("map", "amapWebKey"),
            ("miniProgramRisk", "wechatLiveMiniAppSecret"),
        ];
        for (key, field) in KNOWN_CREDENTIALS {
            let rule = group_rule(key).unwrap_or_else(|| panic!("组 {key} 应在注册表"));
            assert!(
                rule.deny_fields.contains(field) || is_sensitive_field(field),
                "组 {key} 的凭据字段 {field} 未被剔除名单/敏感模式覆盖"
            );
        }
    }

    /// 豁免组绝不出现在白名单中（支付/凭据/身份类整组豁免的设计断言）。
    #[test]
    fn exempt_groups_are_not_whitelisted() {
        const EXEMPT_GET_PATHS: &[&str] = &[
            "/setting/config/basicPaySettings",
            "/setting/config/wechatPaySettings",
            "/setting/config/aliPaySettings",
            "/setting/config/yaBandPaySettings",
            "/setting/config/offlinePaySettings",
            "/setting/config/payPalSettings",
            "/setting/config/yunPaySettings",
            "/setting/config/stripeSettings",
            "/setting/config/xenditSettings",
            "/setting/config/antomSettings",
            "/setting/config/mailSettings",
            "/setting/config/profitSharingSettings",
            "/setting/config/getWXWorkConfig",
            "/setting/config/getDingConfig",
            "/setting/config/wechatLiveSettings",
        ];
        for path in EXEMPT_GET_PATHS {
            assert!(
                GROUP_RULES.iter().all(|r| r.get_path != *path),
                "豁免组 {path} 不得出现在白名单"
            );
        }
        const EXEMPT_BLOCKS: &[&str] = &[
            "federationSettings",
            "aiMcpSettings",
            "aiGuideSettings",
            "pcDomain",
            "opsCodeSettings",
            "paySettings",
            "apiSettings",
        ];
        for code in EXEMPT_BLOCKS {
            assert!(!is_block_code(code), "豁免块 {code} 不得出现在白名单");
        }
    }
}
