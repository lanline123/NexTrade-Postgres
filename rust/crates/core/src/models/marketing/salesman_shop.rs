//! 分销员外壳店「店中店」（marketing::salesman_shop）DTO / VO / 纯逻辑。
//!
//! **我方新增**（论坛帖 257876041323848173，用户 2026-11-21 拍板）：
//! 原版微商分销只有分销员身份 / 名片 / 海报，没有「一人一店面外壳」形态。
//!
//! 形态定义：每个分销员生成一个个性化店面外壳——可自定义店名、店招图、
//! 简介三项；买家从专属链接 / 二维码进入后看到他的店面皮，商品 / 库存 /
//! 下单 / 售后仍走主商城既有链，订单归因到该分销员、佣金走既有分销
//! 生成链。本模块只是**呈现 + 归因层**：
//! - 不建独立店铺实体，不复制商品 / 库存 / 订单体系；
//! - 归因复用既有公开标识 `salesmanId`（分享链 / 小程序码同款）与既有
//!   落地写 `bindSalesmanId` + 注册绑定（`salesman_customer`，
//!   ON CONFLICT DO NOTHING 先绑为准），不另造绑定 / 覆盖逻辑；
//! - 平台禁用（`is_enabled=0`）只关外壳展示，不动分销员身份、客户
//!   绑定与佣金（与「分销员禁用」缓做项是两套概念）。
//!
//! 表：`salesman_shop`（迁移 `rust/deploy/pg/2026-11-21_salesman_shop.sql`，
//! 一分销员一行，无行 = 未自定义、默认启用、店名回退分销员昵称）。
//!
//! 端点（均为我方新增，不进 Java 契约覆盖统计）：
//! - public：`GET /api/salesman/shop/detail?salesmanId=`（optional）、
//!   `GET /api/salesman/shop/myShop`（登录）、`POST /api/salesman/shop/save`（登录）
//! - admin：`GET /adminapi/salesman/shop/list`、`POST /adminapi/salesman/shop/setEnabled`

use serde::{Deserialize, Serialize};

/// 店名最大长度（字符）。
pub const SHOP_NAME_MAX_CHARS: usize = 50;
/// 简介最大长度（字符）。
pub const INTRO_MAX_CHARS: usize = 200;
/// 店招图 URL 最大长度（字符）。
pub const BANNER_MAX_CHARS: usize = 1000;

pub const MSG_SHOP_NAME_TOO_LONG: &str = "店名不能超过50个字";
pub const MSG_INTRO_TOO_LONG: &str = "简介不能超过200个字";
pub const MSG_BANNER_TOO_LONG: &str = "店招图地址过长";

/// 买家侧外壳店详情（公开端 `detail` 出参）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellShopPublicVO {
    pub salesman_id: i32,
    /// 展示店名：自定义店名优先，未自定义回退分销员昵称。
    pub shop_name: String,
    /// 店招图 URL（未自定义为空串，前台渲染默认头图区）。
    pub banner: String,
    pub intro: String,
    /// 分销员头像 / 昵称（身份回退展示用）。
    pub avatar: String,
    pub nickname: String,
    /// 是否可展示：平台禁用时为 false，前台只出关闭提示、不写归因。
    pub enabled: bool,
    /// 是否已做过自定义（存在配置行且三项任一非空）。
    pub customized: bool,
}

/// 分销员自助视图（`myShop` / `save` 出参）：回的是**原始自定义值**
/// （未自定义时三项为空串），与买家侧的回退展示口径分开，避免把回退
/// 昵称当成已存店名写回。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellShopMineVO {
    pub salesman_id: i32,
    pub shop_name: String,
    pub banner: String,
    pub intro: String,
    pub enabled: bool,
    pub customized: bool,
}

/// 保存外壳店入参（三项均可缺省，缺省按空串处理 = 清除该项自定义）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellShopSaveDTO {
    pub shop_name: Option<String>,
    pub banner: Option<String>,
    pub intro: Option<String>,
}

/// 平台外壳店列表查询。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellShopAdminQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    /// 关键词：匹配分销员昵称 / 手机号 / 自定义店名。
    pub keyword: Option<String>,
    /// 展示状态过滤：1 只看启用、0 只看禁用，缺省全部。
    pub enabled: Option<i32>,
}

/// 平台外壳店列表行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellShopAdminVO {
    pub salesman_id: i32,
    pub user_id: i32,
    pub nickname: String,
    pub mobile: String,
    /// 分销员所属店铺（salesman.shop_id）。
    pub salesman_shop_id: i32,
    pub shop_name: String,
    pub banner: String,
    pub intro: String,
    pub enabled: bool,
    pub customized: bool,
    pub update_time: i32,
}

/// 平台启停入参。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellShopSetEnabledDTO {
    pub salesman_id: Option<i32>,
    pub enabled: Option<bool>,
}

/// 校验并归一化保存入参（去首尾空白 + 长度上限）。
///
/// # Errors
/// 任一项超长 → 对应文案（服务层转 `Error::Validation`）。
pub fn normalize_save(dto: &ShellShopSaveDTO) -> Result<(String, String, String), &'static str> {
    let shop_name = dto
        .shop_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    let banner = dto.banner.as_deref().unwrap_or_default().trim().to_string();
    let intro = dto.intro.as_deref().unwrap_or_default().trim().to_string();
    if shop_name.chars().count() > SHOP_NAME_MAX_CHARS {
        return Err(MSG_SHOP_NAME_TOO_LONG);
    }
    if banner.chars().count() > BANNER_MAX_CHARS {
        return Err(MSG_BANNER_TOO_LONG);
    }
    if intro.chars().count() > INTRO_MAX_CHARS {
        return Err(MSG_INTRO_TOO_LONG);
    }
    Ok((shop_name, banner, intro))
}

/// 买家侧展示店名口径：自定义店名（非空）优先，否则回退分销员昵称，
/// 昵称也为空时回退「分销员小店」。
#[must_use]
pub fn resolve_display_name(custom_name: &str, nickname: &str) -> String {
    let custom = custom_name.trim();
    if !custom.is_empty() {
        return custom.to_string();
    }
    let nick = nickname.trim();
    if !nick.is_empty() {
        return nick.to_string();
    }
    "分销员小店".to_string()
}

/// 展示开关口径：无配置行 = 默认启用；有行时 `is_enabled != 0` 为启用。
#[must_use]
pub fn resolve_enabled(row_is_enabled: Option<i32>) -> bool {
    row_is_enabled.map(|v| v != 0).unwrap_or(true)
}

/// 是否已自定义：存在配置行且店名 / 店招 / 简介任一非空。
#[must_use]
pub fn resolve_customized(row: Option<(&str, &str, &str)>) -> bool {
    match row {
        None => false,
        Some((name, banner, intro)) => {
            !name.trim().is_empty() || !banner.trim().is_empty() || !intro.trim().is_empty()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(name: &str, banner: &str, intro: &str) -> ShellShopSaveDTO {
        ShellShopSaveDTO {
            shop_name: Some(name.to_string()),
            banner: Some(banner.to_string()),
            intro: Some(intro.to_string()),
        }
    }

    #[test]
    fn normalize_trims_and_defaults_missing_fields() {
        let (n, b, i) = normalize_save(&dto(
            "  阿珍的电池小店  ",
            " https://x/y.png ",
            " 主营回收 ",
        ))
        .unwrap();
        assert_eq!(n, "阿珍的电池小店");
        assert_eq!(b, "https://x/y.png");
        assert_eq!(i, "主营回收");
        let (n2, b2, i2) = normalize_save(&ShellShopSaveDTO::default()).unwrap();
        assert_eq!((n2.as_str(), b2.as_str(), i2.as_str()), ("", "", ""));
    }

    #[test]
    fn normalize_rejects_overlong_fields() {
        let long_name = "店".repeat(SHOP_NAME_MAX_CHARS + 1);
        assert_eq!(
            normalize_save(&dto(&long_name, "", "")),
            Err(MSG_SHOP_NAME_TOO_LONG)
        );
        let long_intro = "简".repeat(INTRO_MAX_CHARS + 1);
        assert_eq!(
            normalize_save(&dto("", "", &long_intro)),
            Err(MSG_INTRO_TOO_LONG)
        );
        let long_banner = "x".repeat(BANNER_MAX_CHARS + 1);
        assert_eq!(
            normalize_save(&dto("", &long_banner, "")),
            Err(MSG_BANNER_TOO_LONG)
        );
        // 恰好上限放行（按字符数而非字节数）
        let exact = "店".repeat(SHOP_NAME_MAX_CHARS);
        assert!(normalize_save(&dto(&exact, "", "")).is_ok());
    }

    #[test]
    fn display_name_fallback_chain() {
        assert_eq!(resolve_display_name("我的小店", "阿珍"), "我的小店");
        assert_eq!(resolve_display_name("  ", "阿珍"), "阿珍");
        assert_eq!(resolve_display_name("", ""), "分销员小店");
    }

    #[test]
    fn enabled_defaults_true_without_row() {
        assert!(resolve_enabled(None));
        assert!(resolve_enabled(Some(1)));
        assert!(!resolve_enabled(Some(0)));
        assert!(resolve_enabled(Some(2)));
    }

    #[test]
    fn customized_requires_any_non_empty_field() {
        assert!(!resolve_customized(None));
        assert!(!resolve_customized(Some(("", " ", ""))));
        assert!(resolve_customized(Some(("小店", "", ""))));
        assert!(resolve_customized(Some(("", "https://x/y.png", ""))));
        assert!(resolve_customized(Some(("", "", "简介"))));
    }
}
