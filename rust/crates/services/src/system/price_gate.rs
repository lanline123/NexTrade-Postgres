//! 价格门控执行层（第 2 期：登录 + 认证看价）。
//!
//! 口径见 `inquiry-design/proposal.md` 第四节：后台「会员认证设置」的
//! `isIdentity` 开启时，看价权谓词 = 已登录 && `user.is_company_auth = 1`；
//! 关闭时人人可见（行为与门控上线前完全一致）。谓词在**接口层**执行：
//! 公众读接口在谓词不满足时不下发真实价格（价格键置空），只藏 UI 无效。
//! 询价商品/型号本身不出价，其载荷的价格键同样置空（混合商品的
//! `minPricedPrice` 是真实价，必须脱敏）。
//!
//! 购买侧连带（用户决策 3，2026-11-15）：未认证用户不能加购/下单，
//! 否则购物车会漏价绕过门控——见 [`ensure_purchase_allowed`]。
//!
//! 前端契约：被脱敏的商品载荷带 `priceVisible: false` 与
//! `priceGate: "login" | "certification"`，前端据此渲染四态文案
//! （审核中/已驳回的子状态由前端查 `/user/company/detail` 细化，
//! 后端只给到「需认证」为止）。

use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;
use serde_json::Value;

use super::auth_gate::{effective_auth_gate, AUTH_GATE_CODES};

/// 当前访客的看价权判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceAccess {
    /// 可见真实价格（门控关闭，或已登录且认证通过）。
    Visible,
    /// 门控开启且未登录。
    NeedLogin,
    /// 门控开启、已登录但未认证通过（含审核中/驳回/未申请）。
    NeedCertification,
}

impl PriceAccess {
    /// 是否可下发真实价格。
    #[must_use]
    pub fn can_view(self) -> bool {
        matches!(self, Self::Visible)
    }

    /// 前端契约码：`none` / `login` / `certification`。
    #[must_use]
    pub fn gate_code(self) -> &'static str {
        match self {
            Self::Visible => "none",
            Self::NeedLogin => "login",
            Self::NeedCertification => "certification",
        }
    }

    /// 金额类接口（实时金额/预售定金）在门控拦截时使用的文案；
    /// `Visible` 时为 `None`（不拦截）。
    #[must_use]
    pub fn block_message(self) -> Option<&'static str> {
        match self {
            Self::Visible => None,
            Self::NeedLogin => Some("请先登录后查看价格"),
            Self::NeedCertification => Some("认证后查看价格"),
        }
    }
}

/// 纯谓词（无 IO，可单测）：`is_identity != 1` 时人人可见；
/// 开启时 = 已登录 && `is_company_auth == 1`。
/// `user` 为 `(user_id, is_company_auth)`，`None` 表示未登录。
#[must_use]
pub fn decide_price_access(is_identity: i64, user: Option<(i32, i32)>) -> PriceAccess {
    if is_identity != 1 {
        return PriceAccess::Visible;
    }
    match user {
        None => PriceAccess::NeedLogin,
        Some((_, is_company_auth)) if is_company_auth == 1 => PriceAccess::Visible,
        Some(_) => PriceAccess::NeedCertification,
    }
}

/// 解析当前访客的看价权：读认证门控配置 + （必要时）查用户认证标志。
///
/// # Errors
/// - 配置/用户仓储失败 ⇒ `Error::Database`
pub async fn resolve_price_access(
    repos: &Repositories,
    user_id: Option<i32>,
) -> Result<PriceAccess> {
    let rows = repos
        .system
        .config_client
        .find_config_values(&AUTH_GATE_CODES)
        .await?;
    let pairs: Vec<(String, String)> = rows
        .iter()
        .filter_map(|c| c.biz_val.clone().map(|v| (c.biz_code.clone(), v)))
        .collect();
    let gate = effective_auth_gate(&pairs);
    let is_identity = gate["isIdentity"].as_i64().unwrap_or(0);
    if is_identity != 1 {
        return Ok(PriceAccess::Visible);
    }
    let Some(uid) = user_id.filter(|v| *v > 0) else {
        return Ok(PriceAccess::NeedLogin);
    };
    let user = repos.member.user_center.find_user_by_id(uid).await?;
    Ok(decide_price_access(
        is_identity,
        user.map(|u| (u.user_id, u.is_company_auth)),
    ))
}

/// 购买门控（决策 3）：门控开启且未认证通过 ⇒ 拒绝加购/改量/下单。
/// 门控关闭时直接放行（多一次配置查询，与黑名单校验同量级）。
///
/// # Errors
/// - 未认证 ⇒ 业务错误「请先完成实名认证后再购买」
/// - 仓储失败 ⇒ `Error::Database`
pub async fn ensure_purchase_allowed(repos: &Repositories, user_id: i32) -> Result<()> {
    match resolve_price_access(repos, Some(user_id)).await? {
        PriceAccess::Visible => Ok(()),
        // 购买链本身要求登录，NeedLogin 只会在令牌用户已不存在时出现
        PriceAccess::NeedLogin => Err(Error::biz(401, "请先登录".to_string())),
        PriceAccess::NeedCertification => Err(Error::biz(
            400,
            "请先完成实名认证后再购买".to_string(),
        )),
    }
}

/// 公众商品载荷中的价格键集合（脱敏时逐一置空）。覆盖：商品/型号价、
/// 市场价、成本价、混合询价最低价、兼容价、搜索原价、门店价、促销价、
/// 秒杀价、拼团价、会员价、批发价与筛选价格带。
///
/// 除驼峰键外同时列 snake_case 变体：个别公众载荷直接序列化实体行
/// （如店铺街 `listingProduct` 的 product 实体，字段为 `product_price`），
/// 只认驼峰会整行漏脱（2026-11-19 补漏审计发现）。
pub const PRICE_KEYS: [&str; 31] = [
    "productPrice",
    "marketPrice",
    "costPrice",
    "minPricedPrice",
    "price",
    "orgProductPrice",
    "storeProductPrice",
    "skuPrice",
    "promotePrice",
    "seckillPrice",
    "grouponPrice",
    "originPrice",
    "memberPrice",
    "wholesalePrice",
    "maxPrice",
    "minPrice",
    "product_price",
    "market_price",
    "cost_price",
    "min_priced_price",
    "org_product_price",
    "store_product_price",
    "sku_price",
    "promote_price",
    "seckill_price",
    "groupon_price",
    "origin_price",
    "member_price",
    "wholesale_price",
    "max_price",
    "min_price",
];

/// 递归把对象/数组中的价格键置 `Null`（原地修改）。
/// 已是 `Null` 的键保持不变；非价格键与标量不动。
pub fn mask_price_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if PRICE_KEYS.contains(&k.as_str()) {
                    // 已是 Null 时赋值仍为 Null，语义不变
                    *v = Value::Null;
                } else {
                    mask_price_keys(v);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                mask_price_keys(item);
            }
        }
        _ => {}
    }
}

/// 给单个商品载荷打门控标记：`priceVisible` + `priceGate`。
/// 前端据此渲染四态文案，不必自行拼谓词。
pub fn stamp_price_gate(item: &mut Value, access: PriceAccess) {
    if let Some(obj) = item.as_object_mut() {
        obj.insert("priceVisible".into(), Value::Bool(access.can_view()));
        obj.insert(
            "priceGate".into(),
            Value::String(access.gate_code().to_string()),
        );
    }
}

/// 对分页载荷逐项脱敏 + 打标记。
///
/// `PageResult` 的序列化同时下发 `list` 与 `records` 两份条目数组
/// （Java `Page` 兼容口径），两份都必须脱敏——只脱 `list` 时
/// `records` 里仍是明文价（2026-11-19 首页秒杀补漏审计发现的同类漏点，
/// 商品列表/营销秒杀/拼团等已接门控的分页端点同样受影响，一并在此收口）。
/// 两份数组都找不到时只做递归脱敏兜底。
pub fn mask_page_items(page: &mut Value, access: PriceAccess) {
    if access.can_view() {
        return;
    }
    let mut masked_items = false;
    if let Some(obj) = page.as_object_mut() {
        for key in ["list", "records"] {
            if let Some(list) = obj.get_mut(key).and_then(Value::as_array_mut) {
                for item in list.iter_mut() {
                    mask_price_keys(item);
                    stamp_price_gate(item, access);
                }
                masked_items = true;
            }
        }
    }
    if !masked_items {
        mask_price_keys(page);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gate_off_everyone_visible() {
        assert_eq!(decide_price_access(0, None), PriceAccess::Visible);
        assert_eq!(decide_price_access(0, Some((7, 0))), PriceAccess::Visible);
    }

    #[test]
    fn gate_on_requires_login_and_certification() {
        assert_eq!(decide_price_access(1, None), PriceAccess::NeedLogin);
        assert_eq!(
            decide_price_access(1, Some((7, 0))),
            PriceAccess::NeedCertification
        );
        assert_eq!(decide_price_access(1, Some((7, 1))), PriceAccess::Visible);
    }

    #[test]
    fn mask_nulls_price_keys_recursively() {
        let mut v = json!({
            "productId": 9,
            "productPrice": "12.30",
            "minPricedPrice": "10.00",
            "skuList": [{"skuPrice": "12.30", "skuStock": 5}],
            "nested": {"seckillPrice": 9.9, "name": "x"}
        });
        mask_price_keys(&mut v);
        assert!(v["productPrice"].is_null());
        assert!(v["minPricedPrice"].is_null());
        assert!(v["skuList"][0]["skuPrice"].is_null());
        assert_eq!(v["skuList"][0]["skuStock"], 5);
        assert!(v["nested"]["seckillPrice"].is_null());
        assert_eq!(v["nested"]["name"], "x");
        assert_eq!(v["productId"], 9);
    }

    #[test]
    fn page_masking_covers_records_and_list() {
        // PageResult 真实序列化形状：list 与 records 双份条目，价格都必须脱敏
        let item = json!({
            "productId": 1548,
            "seckillPrice": "198000.00",
            "productPrice": "220000.00",
            "seckkillData": {"seckillPrice": "198000.00", "stock": 100}
        });
        let mut page = json!({
            "total": 1,
            "list": [item.clone()],
            "records": [item]
        });
        mask_page_items(&mut page, PriceAccess::NeedLogin);
        for key in ["list", "records"] {
            let masked = &page[key][0];
            assert!(masked["seckillPrice"].is_null(), "{key} seckillPrice 未脱敏");
            assert!(masked["productPrice"].is_null(), "{key} productPrice 未脱敏");
            assert!(
                masked["seckkillData"]["seckillPrice"].is_null(),
                "{key} 嵌套 seckillPrice 未脱敏"
            );
            assert_eq!(masked["seckkillData"]["stock"], 100);
            assert_eq!(masked["priceVisible"], false);
            assert_eq!(masked["priceGate"], "login");
            assert_eq!(masked["productId"], 1548);
        }
    }

    #[test]
    fn stamp_and_page_masking() {
        let mut page = json!({
            "total": 1,
            "list": [{"productId": 1, "productPrice": "3.00", "inquiryState": "none"}]
        });
        mask_page_items(&mut page, PriceAccess::NeedLogin);
        let item = &page["list"][0];
        assert!(item["productPrice"].is_null());
        assert_eq!(item["priceVisible"], false);
        assert_eq!(item["priceGate"], "login");
        // 可见时不改载荷
        let mut page2 = json!({"list": [{"productPrice": "3.00"}]});
        mask_page_items(&mut page2, PriceAccess::Visible);
        assert_eq!(page2["list"][0]["productPrice"], "3.00");
    }
}
