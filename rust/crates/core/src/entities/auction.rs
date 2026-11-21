//! 商品拍卖实体（一期新增，规划见 `~/workspace/auction-design/proposal.md` 第 8 节）。
//!
//! 本文件为手写新增（非 schema_parser 生成）：拍卖是 Rust 版原创域，
//! 无 Java 对应表。字段与 `rust/migrations/004_auction.sql` 严格对齐：
//! 秒级 INT 时间戳、NUMERIC(12,2) 金额（映射 `Decimal`）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 拍卖场次表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionSession {
    /// [NOT NULL]
    pub session_id: i32,
    /// 场次名称
    pub session_name: String,
    /// 封面图
    pub cover_pic: String,
    /// 场次开始时间（展示用，拍品各自计时）
    pub start_time: i32,
    /// 场次结束时间（展示用）
    pub end_time: i32,
    /// 排序
    pub sort: i32,
    /// 状态：0 禁用 1 启用
    pub status: i32,
    /// 创建时间
    pub add_time: i32,
}

impl AuctionSession {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_session";
}

/// 拍品表（核心表）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionItem {
    /// [NOT NULL]
    pub item_id: i32,
    /// 所属场次（可空）
    pub session_id: Option<i32>,
    /// 商品 id
    pub product_id: i32,
    /// 商品归属：1 = 建拍品时系统新建的拍卖专属商品（下架态，删除草稿
    /// 拍品时一并软删）；0 = 关联的既有商品（存量拍品，删除拍品不动商品）。
    /// 迁移 010 加列；`sqlx(default)` 兼容迁移未执行时缺列读回 0。
    #[sqlx(default)]
    pub own_product: i32,
    /// sku id（无 SKU 商品为 0）
    pub sku_id: i32,
    /// 店铺 id（0 = 平台自营）
    pub shop_id: i32,
    /// 拍品标题（默认取商品名，可覆盖）
    pub title: String,
    /// 起拍价
    pub start_price: Decimal,
    /// 保留价（可空，对外保密）
    pub reserve_price: Option<Decimal>,
    /// 一口价（可空）
    pub buyout_price: Option<Decimal>,
    /// 保证金（建拍品时按设置规则算好落值）
    pub deposit_amount: Decimal,
    /// 开始时间
    pub start_time: i32,
    /// 原始结束时间
    pub end_time: i32,
    /// 含顺延的实际结束时间
    pub real_end_time: i32,
    /// 顺延次数
    pub extend_count: i32,
    /// 状态：0 草稿 1 预告 2 竞价中 3 已成交 4 流拍 5 已撤拍 6 强制结束
    pub status: i32,
    /// 当前最高价（冗余，出价时同事务更新）
    pub current_price: Option<Decimal>,
    /// 当前领先出价 id
    pub current_bid_id: Option<i32>,
    /// 有效出价数
    pub bid_count: i32,
    /// 中标人
    pub winner_user_id: Option<i32>,
    /// 成交价
    pub final_price: Option<Decimal>,
    /// 成交订单 id
    pub order_id: Option<i32>,
    /// 代理出价上限（二期预留）
    pub proxy_max_price: Option<Decimal>,
    /// 创建管理员 id
    pub create_admin_id: i32,
    /// 创建时间
    pub add_time: i32,
    /// 更新时间
    pub update_time: i32,
}

impl AuctionItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_item";

    /// 展示用当前价：有出价取当前价，否则起拍价。
    #[must_use]
    pub fn display_price(&self) -> Decimal {
        self.current_price.unwrap_or(self.start_price)
    }

    /// 是否终态（成交/流拍/撤拍/强制结束）。
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, 3 | 4 | 5 | 6)
    }
}

/// 出价记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionBid {
    /// [NOT NULL]
    pub bid_id: i32,
    /// 拍品 id
    pub item_id: i32,
    /// 出价人
    pub user_id: i32,
    /// 出价金额
    pub bid_price: Decimal,
    /// 是否一口价成交出价
    pub is_buyout: i32,
    /// 是否代理出价（二期预留）
    pub is_proxy: i32,
    /// 状态：1 有效 2 已作废
    pub status: i32,
    /// 出价 IP（风控留痕）
    pub ip: String,
    /// 出价 UA（风控留痕）
    pub user_agent: String,
    /// 设备指纹（风控留痕，可空）
    pub device_fp: String,
    /// 出价时间
    pub add_time: i32,
}

impl AuctionBid {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_bid";
}

/// 保证金记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionDeposit {
    /// [NOT NULL]
    pub deposit_id: i32,
    /// 拍品 id
    pub item_id: i32,
    /// 用户 id
    pub user_id: i32,
    /// 保证金金额
    pub amount: Decimal,
    /// 状态：1 冻结中 2 已解冻 3 已抵扣 4 已扣罚 5 部分扣罚后退回
    pub status: i32,
    /// 关联 user_balance_log.log_id（冻结那笔）
    pub balance_log_id: Option<i32>,
    /// 抵扣的订单 id
    pub order_id: Option<i32>,
    /// 扣罚金额
    pub penalty_amount: Decimal,
    /// 冻结时间
    pub add_time: i32,
    /// 终态处理时间
    pub handle_time: i32,
    /// 备注
    pub remark: String,
}

impl AuctionDeposit {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_deposit";
}

/// 违约记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionBreach {
    /// [NOT NULL]
    pub breach_id: i32,
    /// 违约用户
    pub user_id: i32,
    /// 拍品 id
    pub item_id: i32,
    /// 违约订单 id
    pub order_id: i32,
    /// 当时保证金金额
    pub deposit_amount: Decimal,
    /// 实际扣罚金额
    pub penalty_amount: Decimal,
    /// 违约时间
    pub breach_time: i32,
    /// 是否被管理员改判豁免
    pub waived: i32,
    /// 改判理由
    pub waive_reason: String,
    /// 操作管理员 id
    pub operator_id: i32,
}

impl AuctionBreach {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_breach";
}

/// 参拍封禁表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionBan {
    /// [NOT NULL]
    pub ban_id: i32,
    /// 被封禁用户
    pub user_id: i32,
    /// 类型：1 临时 2 永久
    pub ban_type: i32,
    /// 封禁开始时间
    pub start_time: i32,
    /// 封禁结束时间（永久为 0）
    pub end_time: i32,
    /// 封禁原因
    pub reason: String,
    /// 状态：1 生效 0 已解封
    pub status: i32,
    /// 操作管理员 id（系统自动封禁为 0）
    pub operator_id: i32,
    /// 创建时间
    pub add_time: i32,
}

impl AuctionBan {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_ban";
}

/// 开拍提醒订阅表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AuctionRemind {
    /// [NOT NULL]
    pub remind_id: i32,
    /// 拍品 id
    pub item_id: i32,
    /// 订阅用户
    pub user_id: i32,
    /// 是否已发送开拍提醒
    pub notified: i32,
    /// 订阅时间
    pub add_time: i32,
}

impl AuctionRemind {
    /// 数据库表名。
    pub const TABLE: &'static str = "auction_remind";
}
