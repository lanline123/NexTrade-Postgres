//! 价格门控执行层（第 2 期：登录 + 认证看价；2026-11-21 拆双开关）。
//!
//! 口径见 `inquiry-design/proposal.md` 第四节 + 2026-11-21 用户拍板：
//! 价格查看门控拆为两个**独立**开关（后台「会员认证设置」）——
//! - `isLogin`（开关①，我方新增）：开启后匿名用户看不到价格，
//!   任意已登录用户可查；**只管查看，不管购买**。
//! - `isIdentity`（开关②，原有）：开启后看价权谓词 = 已登录 &&
//!   `user.is_company_auth = 1`。
//!
//! 两开关都关时人人可见（行为与门控上线前完全一致）。谓词在**接口层**
//! 执行：公众读接口在谓词不满足时不下发真实价格（价格键置空），
//! 只藏 UI 无效。询价商品/型号本身不出价，其载荷的价格键同样置空
//! （混合商品的 `minPricedPrice` 是真实价，必须脱敏）。
//!
//! 购买侧连带（用户决策 3，2026-11-15；2026-11-21 拆分时确认不变）：
//! 未认证用户不能加购/下单，否则购物车会漏价绕过门控——见
//! [`ensure_purchase_allowed`]。购买拦截**只跟随开关②**：购买裁决走
//! [`resolve_purchase_access`]（登录开关强制关闭），开关①开、②关时
//! 已登录未认证用户照常购买。
//! 例外（2026-11-20 裁定）：询价开启的商品不走购买门拦截——询价流程
//! 本就是为未认证用户设计的获价通道，命中询价时由可购校验给出
//! 询价一期文案「该商品为询价商品，请提交询价」，见
//! [`ensure_purchase_allowed_for`]。
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

/// 纯谓词（无 IO，可单测）：看价权按两开关分层判定——
/// `is_identity == 1` 时 = 已登录 && `is_company_auth == 1`（优先级最高）；
/// 否则 `is_login == 1` 时 = 已登录即可；两开关都关时人人可见。
/// `user` 为 `(user_id, is_company_auth)`，`None` 表示未登录。
#[must_use]
pub fn decide_price_access(
    is_login: i64,
    is_identity: i64,
    user: Option<(i32, i32)>,
) -> PriceAccess {
    if is_identity == 1 {
        return match user {
            None => PriceAccess::NeedLogin,
            Some((_, is_company_auth)) if is_company_auth == 1 => PriceAccess::Visible,
            Some(_) => PriceAccess::NeedCertification,
        };
    }
    if is_login == 1 {
        return match user {
            None => PriceAccess::NeedLogin,
            Some(_) => PriceAccess::Visible,
        };
    }
    PriceAccess::Visible
}

/// 解析门控裁决的共用实现：读认证门控配置 + （任一相关开关开启时）
/// 查用户认证标志。`include_login_gate = false` 时登录开关强制按关
/// 处理（购买裁决专用口径，见 [`resolve_purchase_access`]）。
async fn resolve_access(
    repos: &Repositories,
    user_id: Option<i32>,
    include_login_gate: bool,
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
    let is_login = if include_login_gate {
        gate["isLogin"].as_i64().unwrap_or(0)
    } else {
        0
    };
    if is_identity != 1 && is_login != 1 {
        return Ok(PriceAccess::Visible);
    }
    let Some(uid) = user_id.filter(|v| *v > 0) else {
        return Ok(PriceAccess::NeedLogin);
    };
    let user = repos.member.user_center.find_user_by_id(uid).await?;
    Ok(decide_price_access(
        is_login,
        is_identity,
        user.map(|u| (u.user_id, u.is_company_auth)),
    ))
}

/// 解析当前访客的看价权（查看口径：两开关都参与判定）。
///
/// # Errors
/// - 配置/用户仓储失败 ⇒ `Error::Database`
pub async fn resolve_price_access(
    repos: &Repositories,
    user_id: Option<i32>,
) -> Result<PriceAccess> {
    resolve_access(repos, user_id, true).await
}

/// 解析购买裁决用的看价权（购买口径：**只跟随开关② `isIdentity`**，
/// 登录开关强制按关——开关①只管查看、不拦购买，2026-11-21 拍板）。
///
/// # Errors
/// - 配置/用户仓储失败 ⇒ `Error::Database`
pub async fn resolve_purchase_access(repos: &Repositories, user_id: i32) -> Result<PriceAccess> {
    resolve_access(repos, Some(user_id), false).await
}

/// 购买门控裁决（纯函数，可单测）：给定看价权与「目标是否询价开启」，
/// 返回拦截错误 `(业务码, 文案)`；放行返回 `None`。
///
/// 询价豁免（2026-11-20 裁定）：命中询价商品时购买门不拦截——调用方
/// 后续的可购校验（`ProductService::check_purchasable`）会以询价一期
/// 文案「该商品为询价商品，请提交询价」拒绝加购/改量，询价通道对
/// 未认证用户保持可用；普通商品维持决策 3 的无条件拦截。
#[must_use]
pub fn purchase_gate_error(
    access: PriceAccess,
    inquiry_exempt: bool,
) -> Option<(i32, &'static str)> {
    if inquiry_exempt {
        return None;
    }
    match access {
        PriceAccess::Visible => None,
        // 购买链本身要求登录，NeedLogin 只会在令牌用户已不存在时出现
        PriceAccess::NeedLogin => Some((401, "请先登录")),
        PriceAccess::NeedCertification => Some((400, "请先完成实名认证后再购买")),
    }
}

/// 购买门控（决策 3）：认证开关（`isIdentity`）开启且未认证通过 ⇒
/// 拒绝加购/改量/下单。只跟随认证开关，登录开关不拦购买
/// （裁决走 [`resolve_purchase_access`]）；认证开关关闭时直接放行
/// （多一次配置查询，与黑名单校验同量级）。
///
/// # Errors
/// - 未认证 ⇒ 业务错误「请先完成实名认证后再购买」
/// - 仓储失败 ⇒ `Error::Database`
pub async fn ensure_purchase_allowed(repos: &Repositories, user_id: i32) -> Result<()> {
    ensure_purchase_allowed_for(repos, user_id, false).await
}

/// 购买门控（带询价豁免）：`inquiry_exempt = true` 时直接放行（不读
/// 门控配置），由调用方的可购校验接管询价文案；裁决表见
/// [`purchase_gate_error`]。
///
/// # Errors
/// - 未认证且非询价豁免 ⇒ 业务错误「请先完成实名认证后再购买」
/// - 仓储失败 ⇒ `Error::Database`
pub async fn ensure_purchase_allowed_for(
    repos: &Repositories,
    user_id: i32,
    inquiry_exempt: bool,
) -> Result<()> {
    if inquiry_exempt {
        return Ok(());
    }
    let access = resolve_purchase_access(repos, user_id).await?;
    match purchase_gate_error(access, inquiry_exempt) {
        None => Ok(()),
        Some((code, message)) => Err(Error::biz(code, message.to_string())),
    }
}

/// 公众商品载荷中的价格键集合（脱敏时逐一置空）。覆盖：商品/型号价、
/// 市场价、成本价、混合询价最低价、兼容价、搜索原价、门店价、促销价、
/// 秒杀价、拼团价、会员价、批发价（含阶梯价与首条批发价）与筛选价格带。
///
/// 除驼峰键外同时列 snake_case 变体：个别公众载荷直接序列化实体行
/// （如店铺街 `listingProduct` 的 product 实体，字段为 `product_price`），
/// 只认驼峰会整行漏脱（2026-11-19 补漏审计发现）。
///
/// 团购阶梯价键（2026-11-20 裁定三实测补）：`minGrouponPrice` 是团购列表/信息
/// VO 的最低团购价字段，`groupPrice` 是明细行内嵌的阶梯价 JSON 文本——两者都曾在
/// 种子数据三腿实测中对匿名明文泄漏（itemList / getGrouponInfo），故整键置 null。
pub const PRICE_KEYS: [&str; 37] = [
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
    "minGrouponPrice",
    "groupPrice",
    "originPrice",
    "memberPrice",
    "wholesalePrice",
    "firstWholesalePrice",
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
    "min_groupon_price",
    "group_price",
    "origin_price",
    "member_price",
    "wholesale_price",
    "first_wholesale_price",
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
    fn both_gates_off_everyone_visible() {
        assert_eq!(decide_price_access(0, 0, None), PriceAccess::Visible);
        assert_eq!(
            decide_price_access(0, 0, Some((7, 0))),
            PriceAccess::Visible
        );
        assert_eq!(
            decide_price_access(0, 0, Some((7, 1))),
            PriceAccess::Visible
        );
    }

    #[test]
    fn login_gate_only_any_logged_in_user_visible() {
        // 开关①（isLogin）单开：匿名拦登录，任意已登录用户可见（不问认证）
        assert_eq!(decide_price_access(1, 0, None), PriceAccess::NeedLogin);
        assert_eq!(
            decide_price_access(1, 0, Some((7, 0))),
            PriceAccess::Visible
        );
        assert_eq!(
            decide_price_access(1, 0, Some((7, 1))),
            PriceAccess::Visible
        );
    }

    #[test]
    fn identity_gate_requires_login_and_certification() {
        // 开关②（isIdentity）单开与双开同态：未认证一律拦（与拆分前口径一致）
        for is_login in [0, 1] {
            assert_eq!(
                decide_price_access(is_login, 1, None),
                PriceAccess::NeedLogin
            );
            assert_eq!(
                decide_price_access(is_login, 1, Some((7, 0))),
                PriceAccess::NeedCertification
            );
            assert_eq!(
                decide_price_access(is_login, 1, Some((7, 1))),
                PriceAccess::Visible
            );
        }
    }

    #[test]
    fn purchase_decision_ignores_login_gate() {
        // 购买口径的纯函数构成（resolve_purchase_access 把登录开关强制
        // 按关传入谓词）：开关①开、②关时已登录未认证用户购买不拦；
        // ②开时未认证照旧拦（purchase_gate_error 裁决不变）。
        let access = decide_price_access(0, 0, Some((7, 0)));
        assert_eq!(purchase_gate_error(access, false), None);
        let access = decide_price_access(0, 1, Some((7, 0)));
        assert_eq!(
            purchase_gate_error(access, false),
            Some((400, "请先完成实名认证后再购买"))
        );
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

    #[test]
    fn purchase_gate_blocks_uncertified_for_normal_product() {
        // 三腿之二：未认证 + 普通商品 ⇒ 购买门文案（决策 3 不变）
        assert_eq!(
            purchase_gate_error(PriceAccess::NeedCertification, false),
            Some((400, "请先完成实名认证后再购买"))
        );
        assert_eq!(
            purchase_gate_error(PriceAccess::NeedLogin, false),
            Some((401, "请先登录"))
        );
    }

    #[test]
    fn purchase_gate_exempts_inquiry_product() {
        // 三腿之一：未认证 + 询价商品 ⇒ 购买门放行，交由可购校验
        // 给出询价一期文案「该商品为询价商品，请提交询价」
        assert_eq!(
            purchase_gate_error(PriceAccess::NeedCertification, true),
            None
        );
        assert_eq!(purchase_gate_error(PriceAccess::NeedLogin, true), None);
    }

    #[test]
    fn purchase_gate_certified_unchanged() {
        // 三腿之三：已认证 ⇒ 与现状一致，询价/普通商品门均不拦截
        // （询价商品随后由可购校验按一期文案拒绝，与豁免前行为一致）
        assert_eq!(purchase_gate_error(PriceAccess::Visible, true), None);
        assert_eq!(purchase_gate_error(PriceAccess::Visible, false), None);
    }

    #[test]
    fn wholesale_payload_masks_all_price_keys() {
        // 裁定（2026-11-20）：批发域纳入价格门控，与全站同口径。
        // 批发阶梯价（详情 wholesalePriceList）与首条批发价
        // （列表/分类 firstWholesalePrice）的承载端点走同一遮罩层，
        // 此处锁住共享层对批发价形状的覆盖（含 snake_case 变体、
        // 阶梯内嵌 price、首条批发价键——后者曾不在名单内）。
        let mut v = json!({
            "productId": 7,
            "firstWholesalePrice": "88.00",
            "first_wholesale_price": "88.00",
            "wholesalePriceList": [
                {"minQty": 10, "price": "90.00", "wholesalePrice": "90.00"},
                {"minQty": 50, "price": "85.00", "wholesale_price": "85.00"}
            ]
        });
        mask_price_keys(&mut v);
        assert!(v["firstWholesalePrice"].is_null());
        assert!(v["first_wholesale_price"].is_null());
        assert!(v["wholesalePriceList"][0]["price"].is_null());
        assert!(v["wholesalePriceList"][0]["wholesalePrice"].is_null());
        assert!(v["wholesalePriceList"][1]["price"].is_null());
        assert!(v["wholesalePriceList"][1]["wholesale_price"].is_null());
        // 非价格键不动
        assert_eq!(v["wholesalePriceList"][0]["minQty"], 10);
        assert_eq!(v["productId"], 7);
    }
    /// 裁定三实测回归锁：团购 itemList / getGrouponInfo 的价格载体形状——
    /// `minGrouponPrice` 与明细 `groupPrice` JSON 文本在遮罩后必须为 null。
    #[test]
    fn mask_price_keys_covers_groupon_payload_shapes() {
        let mut item = json!({
            "productId": 1576,
            "productPrice": "45000.00",
            "minGrouponPrice": "39999",
            "min_groupon_price": "39999",
            "grouponPromotionDetails": [
                { "groupPrice": "[{\"groupNumKey\":2,\"price\":\"39999\"}]",
                  "group_price": "[{\"groupNumKey\":2,\"price\":\"39999\"}]" }
            ]
        });
        mask_price_keys(&mut item);
        assert_eq!(item["productPrice"], Value::Null);
        assert_eq!(item["minGrouponPrice"], Value::Null);
        assert_eq!(item["min_groupon_price"], Value::Null);
        assert_eq!(item["grouponPromotionDetails"][0]["groupPrice"], Value::Null);
        assert_eq!(item["grouponPromotionDetails"][0]["group_price"], Value::Null);
    }
}
