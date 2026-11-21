//! 拍卖域 DTO/VO（一期新增，Rust 版原创域，无 Java 对应契约）。
//!
//! JSON 字段名保持 camelCase 与全站前端接口一致。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::payment::user_balance_log::format_change_time;

/// 拍品状态（`auction_item.status`）。
pub mod item_status {
    /// 草稿（待排期）
    pub const DRAFT: i32 = 0;
    /// 预告中
    pub const PREVIEW: i32 = 1;
    /// 竞价中
    pub const RUNNING: i32 = 2;
    /// 已成交
    pub const SOLD: i32 = 3;
    /// 流拍
    pub const UNSOLD: i32 = 4;
    /// 已撤拍
    pub const CANCELED: i32 = 5;
    /// 强制结束
    pub const FORCE_ENDED: i32 = 6;

    /// 状态中文名。
    #[must_use]
    pub fn name(status: i32) -> &'static str {
        match status {
            DRAFT => "草稿",
            PREVIEW => "预告中",
            RUNNING => "竞价中",
            SOLD => "已成交",
            UNSOLD => "流拍",
            CANCELED => "已撤拍",
            FORCE_ENDED => "强制结束",
            _ => "未知",
        }
    }
}

/// 保证金状态（`auction_deposit.status`）。
pub mod deposit_status {
    /// 冻结中
    pub const FROZEN: i32 = 1;
    /// 已解冻
    pub const RELEASED: i32 = 2;
    /// 已抵扣货款
    pub const OFFSET: i32 = 3;
    /// 已扣罚（全额）
    pub const PENALIZED: i32 = 4;
    /// 部分扣罚后退回
    pub const PARTIAL_PENALIZED: i32 = 5;

    /// 状态中文名。
    #[must_use]
    pub fn name(status: i32) -> &'static str {
        match status {
            FROZEN => "冻结中",
            RELEASED => "已解冻",
            OFFSET => "已抵扣",
            PENALIZED => "已扣罚",
            PARTIAL_PENALIZED => "部分扣罚后退回",
            _ => "未知",
        }
    }
}

/// 出价状态（`auction_bid.status`）。
pub mod bid_status {
    /// 有效
    pub const VALID: i32 = 1;
    /// 已作废
    pub const VOID: i32 = 2;
}

/// 拍卖订单的 `order_type` 枚举值（新增）。
pub const AUCTION_ORDER_TYPE: i32 = 11;

/// 秒级时间戳 → `YYYY-MM-DD HH:mm:ss`（0/负数返回空串，与全站口径一致）。
#[must_use]
pub fn fmt_time(secs: i32) -> String {
    format_change_time(secs)
}

/// 出价人名称脱敏：保留首尾各一字，中间以 `**` 代替（如「万**新」）。
/// 单字名返回「*户」，两字名返回首字 + `*`。
#[must_use]
pub fn mask_name(name: &str) -> String {
    let chars: Vec<char> = name.trim().chars().collect();
    match chars.len() {
        0 => "匿名用户".to_string(),
        1 => "*户".to_string(),
        2 => format!("{}*", chars[0]),
        n => format!("{}**{}", chars[0], chars[n - 1]),
    }
}

// ———————————————————————— 查询入参 ————————————————————————

/// 管理端拍品分页查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionItemListQuery {
    pub keyword: Option<String>,
    pub session_id: Option<i32>,
    pub status: Option<i32>,
    pub shop_id: Option<i32>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
}

/// 前台拍品列表查询。`tab`：all / preview / running / finished。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionPublicListQuery {
    pub session_id: Option<i32>,
    pub tab: Option<String>,
    /// 排序：ending（即将结束）/ newest（最新开始）/ priceAsc / priceDesc
    pub sort: Option<String>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
}

/// 出价记录分页查询（管理端与「我的」共用过滤口径）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionBidListQuery {
    pub item_id: Option<i32>,
    pub user_id: Option<i32>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
}

/// 保证金流水分页查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionDepositListQuery {
    pub item_id: Option<i32>,
    pub user_id: Option<i32>,
    pub status: Option<i32>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
}

/// 违约记录分页查询。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionBreachListQuery {
    pub user_id: Option<i32>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
}

// ———————————————————————— 写操作入参 ————————————————————————

/// 场次保存入参（`sessionId` 为空 = 新增）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionSessionSaveDTO {
    pub session_id: Option<i32>,
    pub session_name: Option<String>,
    pub cover_pic: Option<String>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub start_time: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub end_time: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub sort: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub status: Option<i32>,
}

/// 拍品保存入参（`itemId` 为空 = 新增）。
///
/// 商品口径（2026-11-21 用户变更）：拍品不再从既有商品中选择——新建时
/// 系统按 `productName/pics/marketPrice/productBrief/productDesc` 同步新建
/// 一个拍卖专属商品（下架态），编辑时把这组字段同步回拍品关联商品。
/// `productId/skuId` 为旧契约字段，现已不读（保留解析兼容旧客户端），
/// 拍品关联商品一律以服务端落库为准。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionItemSaveDTO {
    pub item_id: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub session_id: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub product_id: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub sku_id: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub shop_id: Option<i32>,
    pub title: Option<String>,
    pub start_price: Option<Decimal>,
    pub reserve_price: Option<Decimal>,
    pub buyout_price: Option<Decimal>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub start_time: Option<i32>,
    #[serde(default, deserialize_with = "crate::models::serde_helpers::deserialize_opt_i32")]
    pub end_time: Option<i32>,
    /// 商品名称（新建专属商品 / 编辑同步关联商品）
    pub product_name: Option<String>,
    /// 商品简介（同步至商品 `product_brief`）
    pub product_brief: Option<String>,
    /// 商品详情 HTML（同步至商品 `product_desc`）
    pub product_desc: Option<String>,
    /// 市场价（同步至商品 `product_price` 与 `market_price` 两列：
    /// 前台拍品详情以 `product_price` 展示为「市场价」并据此做高价二次确认）
    pub market_price: Option<Decimal>,
    /// 商品图集（首图为主图；同步至商品 `pic_*` 与 `product_gallery`）
    pub pics: Option<Vec<String>>,
}

/// 单 id 入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionIdDTO {
    #[serde(alias = "id")]
    pub item_id: Option<i32>,
    pub session_id: Option<i32>,
    pub bid_id: Option<i32>,
    pub breach_id: Option<i32>,
    pub ban_id: Option<i32>,
    pub deposit_id: Option<i32>,
    pub reason: Option<String>,
}

/// 出价入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionBidDTO {
    pub item_id: i32,
    pub bid_price: Decimal,
    /// 是否按一口价拍下（金额须等于拍品一口价）
    pub is_buyout: Option<i32>,
    /// 高价二次确认标记（单笔超过当前价 10 倍时须为 1）
    pub confirm_high: Option<i32>,
    /// 设备指纹（风控留痕，可空）
    pub device_fp: Option<String>,
}

/// 封禁入参。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuctionBanSaveDTO {
    pub user_id: i32,
    /// 1 临时 2 永久
    pub ban_type: Option<i32>,
    /// 临时封禁天数（banType=1 时有效）
    pub days: Option<i32>,
    pub reason: Option<String>,
}

// ———————————————————————— VO ————————————————————————

/// 场次 VO。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionSessionVO {
    pub session_id: i32,
    pub session_name: String,
    pub cover_pic: String,
    pub start_time: i32,
    pub end_time: i32,
    pub start_time_text: String,
    pub end_time_text: String,
    pub sort: i32,
    pub status: i32,
    /// 场次下拍品数（管理端列表用）
    pub item_count: i64,
}

/// 拍品卡片 VO（前台列表与管理端列表共用骨架，管理端多回商品名等字段）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionItemVO {
    pub item_id: i32,
    pub session_id: Option<i32>,
    pub product_id: i32,
    pub sku_id: i32,
    pub shop_id: i32,
    pub title: String,
    pub product_name: String,
    pub product_sn: String,
    pub pic_url: String,
    pub pic_thumb: String,
    pub start_price: Decimal,
    /// 展示价：有出价为当前价，否则起拍价
    pub current_price: Decimal,
    pub has_bid: bool,
    pub buyout_price: Option<Decimal>,
    /// 是否设了保留价（只露标记，不露金额）
    pub has_reserve: bool,
    pub deposit_amount: Decimal,
    pub start_time: i32,
    pub end_time: i32,
    pub real_end_time: i32,
    pub start_time_text: String,
    pub end_time_text: String,
    /// 距开始/结束的剩余秒数（按状态取向，终态为 0）
    pub remain_seconds: i64,
    pub extend_count: i32,
    pub status: i32,
    pub status_name: String,
    pub bid_count: i32,
    pub final_price: Option<Decimal>,
    pub winner_masked_name: Option<String>,
    pub order_id: Option<i32>,
    /// 服务器当前时间（前端倒计时校准）
    pub server_time: i64,
}

/// 管理端拍品详情 VO（卡片 VO 之外补齐编辑表单所需的商品资料回读）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionItemAdminDetailVO {
    #[serde(flatten)]
    pub item: AuctionItemVO,
    /// 商品简介（商品 `product_brief`）
    pub product_brief: String,
    /// 商品详情 HTML（商品 `product_desc`）
    pub product_desc: Option<String>,
    /// 商品本店售价（前台拍品详情展示为「市场价」的那一列）
    pub product_price: Decimal,
    /// 商品市场价列
    pub market_price: Decimal,
    /// 商品图集（按相册排序，首图为主图；无相册行时回退商品主图单张）
    pub pics: Vec<String>,
    /// 是否为拍品自建的专属商品（false = 存量拍品关联的既有商品）
    pub own_product: bool,
}

/// 拍品详情 VO（前台）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionItemDetailVO {
    #[serde(flatten)]
    pub item: AuctionItemVO,
    pub product_brief: String,
    pub product_desc: Option<String>,
    /// 商品原价（本店售价）
    pub product_price: Decimal,
    pub market_price: Decimal,
    /// 当前档最小加价幅度
    pub step_amount: Decimal,
    /// 下一口最低可出价（无出价时为起拍价）
    pub min_next_bid: Decimal,
    /// 我的保证金状态：0 未冻结，其余同 deposit_status
    pub my_deposit_status: i32,
    pub my_deposit_amount: Decimal,
    /// 我的最高出价（无则为 0）
    pub my_highest_bid: Decimal,
    pub is_leader: bool,
    pub is_winner: bool,
    /// 我是否已订阅开拍提醒
    pub reminded: bool,
    /// 我是否被封禁参拍（及原因）
    pub ban_reason: Option<String>,
    /// 参拍是否要求企业认证（设置项，前端据此引导）
    pub require_certification: bool,
    /// 我是否已企业认证
    pub my_certified: bool,
    /// 出价墙首页（最近 20 条）
    pub bids: Vec<AuctionBidVO>,
}

/// 出价墙 VO（前台，脱敏）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionBidVO {
    pub bid_id: i32,
    pub item_id: i32,
    pub masked_name: String,
    pub bid_price: Decimal,
    pub is_buyout: bool,
    pub is_leader: bool,
    pub is_mine: bool,
    pub add_time: i32,
    pub add_time_text: String,
}

/// 轮询 VO（详情页 5 秒轮询，响应体保持小）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionPollVO {
    pub item_id: i32,
    pub status: i32,
    pub status_name: String,
    pub current_price: Decimal,
    pub has_bid: bool,
    pub bid_count: i32,
    pub real_end_time: i32,
    pub remain_seconds: i64,
    pub extend_count: i32,
    pub is_leader: bool,
    pub min_next_bid: Decimal,
    pub step_amount: Decimal,
    pub server_time: i64,
    pub latest_bids: Vec<AuctionBidVO>,
}

/// 管理端出价 VO（含风控字段与真实用户信息）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionAdminBidVO {
    pub bid_id: i32,
    pub item_id: i32,
    pub item_title: String,
    pub user_id: i32,
    pub username: String,
    pub nickname: String,
    pub bid_price: Decimal,
    pub is_buyout: i32,
    pub status: i32,
    pub status_name: String,
    pub ip: String,
    pub user_agent: String,
    pub device_fp: String,
    pub add_time: i32,
    pub add_time_text: String,
}

/// 保证金 VO（管理端与「我的保证金」共用）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionDepositVO {
    pub deposit_id: i32,
    pub item_id: i32,
    pub item_title: String,
    pub user_id: i32,
    pub username: String,
    pub nickname: String,
    pub amount: Decimal,
    pub status: i32,
    pub status_name: String,
    pub balance_log_id: Option<i32>,
    pub order_id: Option<i32>,
    pub penalty_amount: Decimal,
    pub add_time: i32,
    pub add_time_text: String,
    pub handle_time: i32,
    pub handle_time_text: String,
    pub remark: String,
}

/// 违约记录 VO。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionBreachVO {
    pub breach_id: i32,
    pub user_id: i32,
    pub username: String,
    pub nickname: String,
    pub item_id: i32,
    pub item_title: String,
    pub order_id: i32,
    pub deposit_amount: Decimal,
    pub penalty_amount: Decimal,
    pub breach_time: i32,
    pub breach_time_text: String,
    pub waived: i32,
    pub waive_reason: String,
    /// 该用户累计未豁免违约次数
    pub breach_count: i64,
}

/// 封禁记录 VO。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionBanVO {
    pub ban_id: i32,
    pub user_id: i32,
    pub username: String,
    pub nickname: String,
    pub ban_type: i32,
    pub ban_type_name: String,
    pub start_time: i32,
    pub end_time: i32,
    pub end_time_text: String,
    pub reason: String,
    pub status: i32,
    pub operator_id: i32,
    pub add_time: i32,
    pub add_time_text: String,
}

/// 我的参拍 VO。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionMyJoinVO {
    #[serde(flatten)]
    pub item: AuctionItemVO,
    pub my_highest_bid: Decimal,
    /// 我的参拍状态：leading 领先 / outbid 被超价 / won 已中标 / lost 未中标 / quit 已退出
    pub my_status: String,
    pub my_status_name: String,
    pub my_deposit_status: i32,
    pub pay_deadline: Option<i32>,
}

/// 拍卖统计概览 VO（管理端）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionStatsVO {
    pub total_items: i64,
    pub running_items: i64,
    pub sold_items: i64,
    pub unsold_items: i64,
    /// 成交率（百分比，保留 1 位小数）
    pub deal_rate: f64,
    /// 平均溢价率（成交价/起拍价，百分比）
    pub avg_premium_rate: f64,
    pub total_bids: i64,
    pub frozen_deposit_total: Decimal,
    pub penalty_total: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_name_rules() {
        assert_eq!(mask_name("万物新生"), "万**生");
        assert_eq!(mask_name("张三"), "张*");
        assert_eq!(mask_name("张"), "*户");
        assert_eq!(mask_name(""), "匿名用户");
        assert_eq!(mask_name("  李四光  "), "李**光");
    }

    #[test]
    fn status_names() {
        assert_eq!(item_status::name(item_status::RUNNING), "竞价中");
        assert_eq!(deposit_status::name(deposit_status::OFFSET), "已抵扣");
        assert_eq!(fmt_time(0), "");
    }
}
