//! 轻量定时任务：替代 Java 的 RabbitMQ 延迟队列（订单超时取消/自动收货）。
//!
//! Java 在 `OrderCheckServiceImpl` / `OrderServiceImpl` 里通过 RabbitMQ
//! `x-delay` 延迟消息 + `RabbitMqConsumer` 实现「未付款订单超时取消」
//! 「发货后自动收货」。本实现改为**轮询扫描 + 幂等条件更新**，无需 MQ：
//!
//! - `order_auto_cancel`：默认每分钟扫描 `pay_status=1 && order_status=0` 且
//!   `add_time <= now - autoCancelOrderMinute*60` 的订单，调用订单取消（回滚库存）；
//! - `order_auto_receive`：默认每 5 分钟扫描 `order_status=2` 且
//!   `shipping_time <= now - autoDeliveryDays*86400` 的订单，置为已完成。
//!
//! - `auto_comment`：默认每 5 分钟扫 `order_auto_comment` 到期快照生成系统
//!   自动评价（v5.8.65 收货后自动评价，我方新增 loop；店铺未开启不落快照）。
//!
//! 配置项与 Java 一致：`config` 表 `autoCancelOrderMinute` / `autoDeliveryDays`；
//! 值为空或 <= 0 时该任务跳过（与 Java 默认关闭一致）。
//!
//! 运维/测试可通过环境变量调整节奏：
//! `NEXTTRADE_SCHEDULER__ENABLED=false` 关闭；`NEXTTRADE_SCHEDULER__DELAY_SECS`（默认 30）、
//! `NEXTTRADE_SCHEDULER__CANCEL_SECS`（默认 60）、`NEXTTRADE_SCHEDULER__RECEIVE_SECS`（默认 300）。
//!
//! 多实例说明：当前无分布式锁（单实例部署）。多实例时需加 Redis `SET NX`
//! 或改用 MQ；任务本身幂等，重复执行不会重复扣库存/改状态。
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use nextrade_common::utils;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

use crate::trade::order::OrderService;

/// 未付款订单超时时间（分钟）配置项。
pub const CFG_AUTO_CANCEL_ORDER_MINUTE: &str = "autoCancelOrderMinute";
/// 自动收货天数配置项。
pub const CFG_AUTO_DELIVERY_DAYS: &str = "autoDeliveryDays";

/// 每轮扫描的最大订单数，避免一次锁太多行。
const SCAN_BATCH: u64 = 100;

/// 读取环境变量秒数（缺省返回 default）。
fn env_secs(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

/// 解析整数配置（缺省/非法 → None）。
#[must_use]
pub fn parse_positive_i64(raw: Option<&str>) -> Option<i64> {
    raw.map(str::trim)
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
}

/// 解析天数配置（支持 `30` / `30.0`；缺省/非法 → None），返回秒数。
#[must_use]
pub fn parse_positive_days(raw: Option<&str>) -> Option<i64> {
    let value = raw.map(str::trim)?.parse::<f64>().ok()?;
    if value <= 0.0 {
        return None;
    }
    Some((value * 86_400.0) as i64)
}

/// 启动全部定时任务，返回 JoinHandle（由进程生命周期管理）。
pub fn spawn_all(repos: Arc<Repositories>, order: OrderService) -> Vec<JoinHandle<()>> {
    vec![
        tokio::spawn(order_auto_cancel_loop(repos.clone(), order.clone())),
        tokio::spawn(order_auto_receive_loop(repos.clone())),
        tokio::spawn(groupon_expire_loop(repos.clone())),
        tokio::spawn(credit_user_period_update_loop(repos.clone())),
        tokio::spawn(product_auto_status_loop(repos.clone())),
        tokio::spawn(aftersales_auto_receive_loop(repos.clone())),
        tokio::spawn(deposit_due_check_loop(repos.clone())),
        tokio::spawn(wechat_live_order_sync_loop(repos.clone())),
        tokio::spawn(profit_sharing_schedule_loop(repos.clone())),
        tokio::spawn(auction_loop(repos.clone())),
        tokio::spawn(auto_comment_loop(repos.clone())),
        tokio::spawn(gift_expire_loop(repos.clone())),
        tokio::spawn(purchase_auto_finish_loop(repos.clone())),
        tokio::spawn(proxy_hall_timeout_loop(repos.clone())),
        tokio::spawn(shop_label_auto_award_loop(repos.clone())),
        tokio::spawn(exchange_rate_sync_loop(repos)),
    ]
}

/// 代客接单大厅超时兜底循环（`NEXTTRADE_SCHEDULER__PROXY_HALL_SECS`，
/// 默认 60s，我方新增二期）：扫接单截止已过仍无人接单的大厅单——
/// 按订单设置 `proxyHallFallback` 改派下单店自履约（默认）或清除
/// 截止继续留池；并联动关闭订单已取消的大厅单。处置逻辑在
/// `trade::proxy_hall::ProxyHallService::run_timeout_scan`，全部走
/// 条件更新、幂等，重复执行无副作用。
async fn proxy_hall_timeout_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 75,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__PROXY_HALL_SECS",
        60,
    )));
    loop {
        ticker.tick().await;
        let service = crate::trade::proxy_hall::ProxyHallService::new(repos.clone());
        let now = utils::now_seconds() as i32;
        match service.run_timeout_scan(now).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：接单大厅超时/取消兜底处置 {n} 笔"),
            Err(e) => tracing::warn!("定时任务：接单大厅兜底扫描失败: {e}"),
/// 店铺标签自动授标循环（`NEXTTRADE_SCHEDULER__SHOP_LABEL_SECS`，默认 3600s）。
///
/// 每轮把全部启用中的自动授标规则（`shop_label_rule`）各重算一次：按
/// 销量/评分/收藏榜单授标、掉榜摘标（我方新增二期，论坛帖 207878955346177229）。
/// 重算是差集式幂等（只动规则自己的 source=1 关系行），与管理端手动
/// 「立即重算」并发同跑也收敛一致。规则表未迁移时本轮报错记日志跳过，
/// 不影响其他任务。
async fn shop_label_auto_award_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 55,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__SHOP_LABEL_SECS",
        3600,
    )));
    loop {
        ticker.tick().await;
        let service = crate::merchant::shop_label_rule::ShopLabelRuleService::new(repos.clone());
        match service.recompute_all_enabled("schedule").await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：店铺标签自动授标重算 {n} 条规则"),
            Err(e) => tracing::warn!("定时任务：店铺标签自动授标扫描失败: {e}"),
        }
    }
}

/// 送礼物领取到期循环（`NEXTTRADE_SCHEDULER__GIFT_EXPIRE_SECS`，默认 300s）。
///
/// 扫 `order_gift` 中待领取且已过领取有效期的礼物，逐笔走系统退款链
/// 原路退回买家并关单（我方新增，论坛帖 197742）。领取页预览也会对
/// 单个过期 token 即时触发同一路径，本循环是兜底全量扫描。
async fn gift_expire_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 60,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__GIFT_EXPIRE_SECS",
        300,
    )));
    loop {
        ticker.tick().await;
        let service = crate::trade::gift_order::GiftOrderService::new(repos.clone());
        let now = utils::now_seconds() as i32;
        match service.run_expire_scan(now).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：送礼物到期退款关闭 {n} 笔"),
            Err(e) => tracing::warn!("定时任务：送礼物到期扫描失败: {e}"),
        }
    }
}

/// 采购单到期自动结束循环（`NEXTTRADE_SCHEDULER__PURCHASE_FINISH_SECS`，默认 3600s）。
///
/// `purchase_setting.auto_finish_days`（预计送达后自动结束天数，空 = 继承）
/// 此前全仓无消费方：采购设置可配但采购单永不自动结束。本循环按
/// 「组织行 → 平台行」解析生效天数（均须 > 0，未配则不结束），把预计
/// 送达日 + 生效天数已过、仍未结束的采购单（待入库 1 / 部分入库 2 /
/// 已发货 5）置为已完成（3），口径与手动完结（order_finish）一致：
/// finish_reason 记「到期自动结束」。条件更新幂等，重复执行无副作用。
async fn purchase_auto_finish_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 45,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__PURCHASE_FINISH_SECS",
        3600,
    )));
    loop {
        ticker.tick().await;
        match run_purchase_auto_finish(repos.clone()).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：采购单到期自动结束 {n} 笔"),
            Err(e) => tracing::warn!("定时任务：采购单到期自动结束失败: {e}"),
        }
    }
}

/// 采购单到期自动结束的单轮扫描（供循环调用；返回本轮结束单数）。
pub async fn run_purchase_auto_finish(repos: Arc<Repositories>) -> Result<u64> {
    let now = utils::now_seconds();
    // 生效天数子查询在 WHERE 里求值两次（空值守卫 + 到期比较），避免
    // NULL 算术把「未配置」误判为到期。
    let res = sqlx::query(
        "UPDATE purchase_order po SET status = 3, finish_reason = '到期自动结束', finish_time = $1, update_time = $1 \
         WHERE po.is_del = 0 AND po.type = 1 AND po.status IN (1, 2, 5) AND po.expected_delivery_date > 0 \
         AND (SELECT s.auto_finish_days FROM purchase_setting s \
              WHERE s.is_del = 0 AND s.auto_finish_days > 0 AND s.shop_id IN (0, po.shop_id) \
              ORDER BY (s.shop_id = po.shop_id) DESC, s.id DESC LIMIT 1) IS NOT NULL \
         AND $1 > po.expected_delivery_date + (SELECT s.auto_finish_days FROM purchase_setting s \
              WHERE s.is_del = 0 AND s.auto_finish_days > 0 AND s.shop_id IN (0, po.shop_id) \
              ORDER BY (s.shop_id = po.shop_id) DESC, s.id DESC LIMIT 1) * 86400",
    )
    .bind(now)
    .execute(&repos.pool)
    .await
    .map_err(|e| nextrade_core::Error::Database(e.to_string()))?;
    Ok(res.rows_affected())
}

/// 自动评价到期生成循环（`NEXTTRADE_SCHEDULER__AUTO_COMMENT_SECS`，默认 300s）。
///
/// 扫 `order_auto_comment` 中已到期的收货快照，逐件生成系统自动评价
/// （`comment.is_default = 1`）；抢占式状态机 + 逐件已有评价检查防重复生成。
/// 未开启自动评价的店铺不会落快照，本循环天然无活可干。
async fn auto_comment_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 50,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__AUTO_COMMENT_SECS",
        300,
    )));
    loop {
        ticker.tick().await;
        let service = crate::catalog::auto_comment::AutoCommentService::new(repos.clone());
        let now = utils::now_seconds() as i32;
        match service.run_due_scan(now).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：自动评价生成 {n} 条"),
            Err(e) => tracing::warn!("定时任务：自动评价到期扫描失败: {e}"),
        }
    }
}

/// 拍卖循环（`NEXTTRADE_SCHEDULER__AUCTION_SECS`，默认 10s，下限 5s）：
/// 开拍推进（预告→竞价中 + 开拍提醒）、结拍结算（成交建单/流拍解冻）、
/// 尾款逾期违约（扣罚/封禁/重新上拍）。单轮内三段串行，逐件失败只记日志。
async fn auction_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 20,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__AUCTION_SECS", 10).max(5),
    ));
    loop {
        ticker.tick().await;
        let service = crate::marketing::auction::AuctionService::new(repos.clone());
        let now = utils::now_seconds() as i32;
        match service.run_start_scan(now).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：拍卖开拍推进 {n} 件"),
            Err(e) => tracing::warn!("定时任务：拍卖开拍推进失败: {e}"),
        }
        match service.run_close_scan(now).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：拍卖结拍 {n} 件"),
            Err(e) => tracing::warn!("定时任务：拍卖结拍扫描失败: {e}"),
        }
        match service.run_overdue_scan(now).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：拍卖尾款逾期违约处理 {n} 笔"),
            Err(e) => tracing::warn!("定时任务：拍卖尾款逾期扫描失败: {e}"),
        }
    }
}

/// 汇率自动更新循环（Java `ExchangeRateSyncTask`，每小时触发）。
///
/// `sync(false)` 内部校验 `autoSync=1` 且距上次同步达 `syncIntervalHours`；
/// 未配置 AppID / 未开启自动更新时静默跳过（debug 日志），对齐 Java 语义。
/// `NEXTTRADE_SCHEDULER__RATE_SECS` 可调节奏（默认 3600s）。
async fn exchange_rate_sync_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 45,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__RATE_SECS",
        3600,
    )));
    loop {
        ticker.tick().await;
        let service = crate::lang::currency::CurrencyService::new(repos.clone());
        match service.sync(false).await {
            Ok(result) if result.success => {
                tracing::info!("定时任务：汇率自动更新完成（{}）", result.message);
            }
            Ok(result) => {
                tracing::debug!("定时任务：汇率自动更新跳过（{}）", result.message);
            }
            Err(e) => tracing::warn!("定时任务：汇率自动更新失败: {e}"),
        }
    }
}

/// 订单超时取消循环。
async fn order_auto_cancel_loop(repos: Arc<Repositories>, order: OrderService) {
    // 启动后延迟，避开启动高峰
    tokio::time::sleep(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__DELAY_SECS",
        30,
    )))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__CANCEL_SECS",
        60,
    )));
    loop {
        ticker.tick().await;
        match run_auto_cancel(&repos, &order).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：超时取消订单 {n} 笔"),
            Err(e) => tracing::warn!("定时任务：订单超时取消失败: {e}"),
        }
    }
}

/// 自动收货循环。
async fn order_auto_receive_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 30,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__RECEIVE_SECS",
        300,
    )));
    loop {
        ticker.tick().await;
        match run_auto_receive(repos.clone()).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：自动确认收货 {n} 笔"),
            Err(e) => tracing::warn!("定时任务：自动收货失败: {e}"),
        }
    }
}

/// 拼团过期循环（每 60s；`NEXTTRADE_SCHEDULER__GROUPON_SECS` 可调）。
async fn groupon_expire_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 15,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__GROUPON_SECS",
        60,
    )));
    loop {
        ticker.tick().await;
        match run_groupon_expire(&repos).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：拼团过期置失败 {n} 团"),
            Err(e) => tracing::warn!("定时任务：拼团过期处理失败: {e}"),
        }
    }
}

/// 保证金到期扫描循环（`NEXTTRADE_SCHEDULER__DEPOSIT_DUE_SECS`，默认 86400s）。
async fn deposit_due_check_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 90,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__DEPOSIT_DUE_SECS",
        86_400,
    )));
    loop {
        ticker.tick().await;
        let service = crate::merchant::deposit_account::DepositAccountService::new(repos.clone());
        match service.run_due_check().await {
            Ok(()) => {}
            Err(e) => tracing::warn!("定时任务：保证金到期扫描失败: {e}"),
        }
    }
}

/// 微信视频号/直播订单同步循环（对应 Java `WechatLiveOrderSyncTask`）。
///
/// Java 每 15 分钟处理待同步/失败日志（`processPendingLogs`）、每日 2 点对账
/// （`reconcileLiveOrders`）。同步执行依赖微信直播/视频号订单 API 凭据，
/// 凭据未接入前本循环做**积压监控**：有待处理日志时每周期告警一次
/// （诚实降级，不伪造同步成功）；接入凭据后在
/// `services/content/wechat_live_order_sync.rs` 落地真实推送。
async fn wechat_live_order_sync_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 120,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__WECHAT_LIVE_SYNC_SECS",
        900,
    )));
    loop {
        ticker.tick().await;
        let now = utils::now_seconds();
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM wechat_live_sync_log              WHERE sync_status IN (0, 2)                AND (next_retry_time IS NULL OR next_retry_time <= $1)",
        )
        .bind(now)
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        if pending > 0 {
            tracing::warn!(
                "微信直播订单同步：{pending} 条待处理/失败日志积压                  （同步执行待微信 API 凭据接入，见 docs/SCHEDULER.md）"
            );
        }
    }
}

/// 线上分账调度循环（对应 Java `ProfitSharingScheduleTask` 的 8 个方法）。
///
/// 分账通道（微信/汇付）API 客户端待商户凭据接入（见 `docs/PAYMENT-CHANNELS.md`：
/// 协议层已就绪）。接入前本循环做**真实可做的部分**：
/// - 超时预警（Java `executeTimeoutWarning`，纯 DB）：挂起超 150 天
///   （微信 180 天冻结上限）的分账单每周期告警；
/// - 其余 7 类（自动分账/状态补偿/完结补偿/失败重试/进件轮询/提现查询/回退补偿）
///   依赖通道查单接口：有积压时每周期报告一次，不伪造成功。
async fn profit_sharing_schedule_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 150,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__PROFIT_SHARING_SECS",
        1800,
    )));
    loop {
        ticker.tick().await;

        // 1. 超时预警（纯 DB，Java executeTimeoutWarning 全量移植）
        let cutoff = chrono::Utc::now().timestamp() - 150 * 24 * 3600;
        // `profit_sharing_order.create_time` 在 MySQL 里是 `varchar(19)`，PG 迁移时曾被升成
        // `timestamp`，后又按「回退原生类型」策略改回 `text`。因此这里必须绑定同格式的字符串
        // 做字典序比较（`YYYY-MM-DD HH:MM:SS` 的字典序等于时间序），不能再绑 NaiveDateTime
        // 走 `::timestamp` 断言——否则空串会触发 `invalid input syntax for type timestamp`。
        let cutoff_text = chrono::DateTime::from_timestamp(cutoff, 0)
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        let timeout_rows: Vec<(i64, String, i64, String)> = sqlx::query_as(
            "SELECT id, out_order_no, shop_id, status FROM profit_sharing_order WHERE status = 0 AND create_time <= $1",
        )
        .bind(cutoff_text)
        .fetch_all(&repos.pool)
        .await
        .unwrap_or_default();
        for (id, out_order_no, shop_id, status) in &timeout_rows {
            tracing::warn!(
                "分账超时预警：id={id} outOrderNo={out_order_no} shopId={shop_id}                  status={status} 挂起超150天（微信180天冻结上限）"
            );
        }

        // 2. 通道依赖类的积压可见性（凭据接入后在此接真实查单/提交）
        let processing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM profit_sharing_order WHERE status = 1",
        )
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        let unfinished: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM profit_sharing_order WHERE status = 2 AND finish_status = 0",
        )
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        let failed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM profit_sharing_order WHERE status = 3 AND retry_count < 3",
        )
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        let returns: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM profit_sharing_return_order WHERE status = 1",
        )
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        let withdraws: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM shop_withdraw              WHERE withdraw_type = 2 AND status = 1                AND channel_status IN ('CREATE_SUCCESS','PROCESSING','SUBMITTING','UNKNOWN')",
        )
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        if processing + unfinished + failed + returns + withdraws > 0 {
            tracing::warn!(
                "分账调度：通道依赖积压（待商户凭据接入后自动处理，见 docs/PAYMENT-CHANNELS.md）                 —— 处理中分账 {processing}、待完结 {unfinished}、待重试 {failed}、                 回退中 {returns}、线上提现待查 {withdraws}"
            );
        }
    }
}

/// 售后自动确认收货循环（`NEXTTRADE_SCHEDULER__AFTERSALES_RECEIPT_SECS`，默认 3600s）。
async fn aftersales_auto_receive_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 60,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__AFTERSALES_RECEIPT_SECS",
        3600,
    )));
    loop {
        ticker.tick().await;
        match run_aftersales_auto_receive(&repos).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：售后自动确认收货 {n} 条"),
            Err(e) => tracing::warn!("定时任务：售后自动确认收货失败: {e}"),
        }
    }
}

/// 执行一轮售后自动确认收货。
pub async fn run_aftersales_auto_receive(repos: &Repositories) -> Result<u64> {
    let raw = repos
        .catalog
        .product_manage
        .config_val("autoConfirmReceiptDays")
        .await?;
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return Ok(0);
    };
    let Ok(days) = raw.trim().parse::<f64>() else {
        return Ok(0);
    };
    if days < 1.0 {
        return Ok(0);
    }
    let cutoff = nextrade_common::utils::now_seconds()
        - (days * 86_400.0) as i64;
    let cutoff = i32::try_from(cutoff).unwrap_or(i32::MIN);
    let rows = repos
        .trade
        .aftersales
        .find_due_auto_receive(cutoff, 100)
        .await?;
    let service = crate::trade::aftersales::AftersalesService::new(Arc::new(repos.clone()));
    let mut done = 0u64;
    for row in rows {
        match service.auto_receive(row.aftersale_id).await {
            Ok(true) => done += 1,
            Ok(false) => {}
            Err(e) => tracing::warn!(
                "售后自动确认收货失败: aftersale_id={}, {e}",
                row.aftersale_id
            ),
        }
    }
    Ok(done)
}

/// 商品自动上下架循环（`NEXTTRADE_SCHEDULER__PRODUCT_AUTO_SECS`，默认 60s）。
async fn product_auto_status_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 15,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__PRODUCT_AUTO_SECS",
        60,
    )));
    loop {
        ticker.tick().await;
        match run_product_auto_status(&repos).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：商品自动上下架执行 {n} 条"),
            Err(e) => tracing::warn!("定时任务：商品自动上下架失败: {e}"),
        }
    }
}

/// 执行一轮商品自动上下架（对应 Java `ProductAutoStatusServiceImpl#executeAutoStatusTasks`）。
pub async fn run_product_auto_status(repos: &Repositories) -> Result<u64> {
    let now = nextrade_common::utils::now_seconds();
    let tasks = repos
        .catalog
        .product_manage
        .find_due_auto_status(now, 50)
        .await?;
    let mut executed = 0u64;
    for task in tasks {
        let product = repos
            .catalog
            .product_manage
            .find_product(task.product_id)
            .await?;
        let Some(product) = product else {
            repos
                .catalog
                .product_manage
                .mark_auto_status(task.id, 2, Some("商品不存在"), task.retry_count + 1, now)
                .await?;
            continue;
        };
        if product.is_delete != 0 {
            repos
                .catalog
                .product_manage
                .mark_auto_status(task.id, 2, Some("商品不存在"), task.retry_count + 1, now)
                .await?;
            continue;
        }
        let online = task.operation_type == 1;
        if online && product.product_type == Some(3) && product.card_group_id > 0 {
            if let Some(group) = repos
                .catalog
                .e_card_group
                .find_by_id(product.card_group_id)
                .await?
            {
                if group.is_use == 0 {
                    let msg = format!(
                        "电子卡卷组【{}】未启用，请先启用电子卡卷组！",
                        group.group_name
                    );
                    repos
                        .catalog
                        .product_manage
                        .mark_auto_status(task.id, 2, Some(&msg), task.retry_count + 1, now)
                        .await?;
                    continue;
                }
            }
        }
        let new_status = i32::from(online);
        let affected = repos
            .catalog
            .product_manage
            .update_product_status_auto(task.product_id, new_status)
            .await?;
        let (status, err) = if affected > 0 {
            (1, None)
        } else {
            (2, Some("商品不存在"))
        };
        repos
            .catalog
            .product_manage
            .mark_auto_status(task.id, status, err, task.retry_count + 1, now)
            .await?;
        executed += 1;
    }
    Ok(executed)
}

/// 授信会员结算周期更新循环（默认每小时扫描一次，`NEXTTRADE_SCHEDULER__CREDIT_PERIOD_SECS` 可调，默认 3600s）。
async fn credit_user_period_update_loop(repos: Arc<Repositories>) {
    tokio::time::sleep(Duration::from_secs(
        env_secs("NEXTTRADE_SCHEDULER__DELAY_SECS", 30) + 45,
    ))
    .await;
    let mut ticker = tokio::time::interval(Duration::from_secs(env_secs(
        "NEXTTRADE_SCHEDULER__CREDIT_PERIOD_SECS",
        3600,
    )));
    loop {
        ticker.tick().await;
        match run_credit_user_period_update(&repos).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("定时任务：授信会员结算周期已推进 {n} 位"),
            Err(e) => tracing::warn!("定时任务：授信会员结算周期推进失败: {e}"),
        }
    }
}

/// 执行一轮授信会员账期推进（对应 Java `CreditUserPeriodUpdateTask`）。
pub async fn run_credit_user_period_update(repos: &Repositories) -> Result<u64> {
    let service = crate::payment::credit_user::CreditUserService::new(Arc::new(repos.clone()));
    service.update_all_period_start_time().await
}

/// 执行一轮拼团过期：待成团且超过截止时间的开团记录整团置失败。
///
/// # Errors
/// 数据库错误。
pub async fn run_groupon_expire(repos: &Repositories) -> Result<usize> {
    let now = utils::now_seconds() as i64;
    let records = repos
        .marketing
        .groupon_promotion_record
        .find_expired_waiting(now, SCAN_BATCH)
        .await?;
    let mut count = 0;
    for record in records {
        let mut completed = false;
        if let Some(item) = repos
            .marketing
            .groupon_promotion_item
            .find_item(record.groupon_promotion_item_id)
            .await?
        {
            let members = repos
                .marketing
                .groupon_promotion_record
                .count_members(record.groupon_promotion_record_id)
                .await?;
            if item.is_auto_group == 1 && members >= i64::from(item.min_group_num) {
                let fill = (i64::from(record.min_group_num) - members).max(0);
                repos
                    .marketing
                    .groupon_promotion_record
                    .auto_complete_group(&record, usize::try_from(fill).unwrap_or(0), now)
                    .await?;
                completed = true;
                count += 1;
                tracing::info!(
                    "拼团到期自动成团: open_record={}, 真实成员={}, 补虚拟={}",
                    record.groupon_promotion_record_id,
                    members,
                    fill
                );
            }
        }
        if completed {
            continue;
        }
        let affected = repos
            .marketing
            .groupon_promotion_record
            .mark_failed_by_open(record.groupon_promotion_record_id)
            .await?;
        if affected > 0 {
            count += 1;
            // 拼团失败：为真实成员订单自动发起退款售后（实际退款仍需财务处理）
            let members = repos
                .marketing
                .groupon_promotion_record
                .find_members(record.groupon_promotion_record_id)
                .await?;
            let aftersales = crate::trade::aftersales::AftersalesService::new(Arc::new(
                repos.clone(),
            ));
            for member in members {
                if member.is_virtual != 0 || member.user_id <= 0 || member.order_id <= 0 {
                    continue;
                }
                if let Err(e) = aftersales
                    .create_groupon_refund(
                        i32::try_from(member.order_id).unwrap_or(0),
                        i32::try_from(member.user_id).unwrap_or(0),
                        member.paid_amount,
                    )
                    .await
                {
                    tracing::warn!(
                        "拼团失败创建售后失败: open={}, order={}, {e}",
                        record.groupon_promotion_record_id,
                        member.order_id
                    );
                }
            }
            tracing::info!(
                "拼团过期: open_record={} 影响成员 {}",
                record.groupon_promotion_record_id,
                affected
            );
        }
    }
    Ok(count)
}

/// 执行一轮超时取消，返回处理笔数。
///
/// # Errors
/// 数据库错误或配置读取失败时返回错误（loop 侧只记日志）。
pub async fn run_auto_cancel(repos: &Repositories, order_service: &OrderService) -> Result<u64> {
    let raw = repos
        .system
        .config_basic
        .find_by_code(CFG_AUTO_CANCEL_ORDER_MINUTE)
        .await?;
    let Some(minutes) = parse_positive_i64(raw.as_deref()) else {
        return Ok(0);
    };
    let now = i64::from(utils::now_seconds());
    let cutoff = now
        .saturating_sub(minutes * 60)
        .clamp(0, i64::from(i32::MAX)) as i32;
    let ids = repos
        .trade
        .order
        .find_unpaid_before(cutoff, SCAN_BATCH)
        .await?;
    let mut done = 0_u64;
    for order_id in ids {
        // 复用订单取消逻辑（状态条件更新 + 回滚库存）；并发下可能返回冲突，忽略
        match order_service.cancel(order_id, None, None).await {
            Ok(()) => done += 1,
            Err(e) => tracing::debug!("自动取消订单 {order_id} 跳过: {e}"),
        }
    }
    Ok(done)
}

/// 执行一轮自动收货，返回处理笔数。
pub async fn run_auto_receive(repos: Arc<Repositories>) -> Result<u64> {
    let raw = repos
        .system
        .config_basic
        .find_by_code(CFG_AUTO_DELIVERY_DAYS)
        .await?;
    let Some(seconds) = parse_positive_days(raw.as_deref()) else {
        return Ok(0);
    };
    let now = i64::from(utils::now_seconds());
    let cutoff = now
        .saturating_sub(seconds)
        .clamp(0, i64::from(i32::MAX)) as i32;
    let ids = repos
        .trade
        .order
        .find_shipped_before(cutoff, SCAN_BATCH)
        .await?;
    let mut done = 0_u64;
    for order_id in ids {
        // Java `receiveOrderConfirmReceiptMessage` 的防护：售后审核处理中
        // （状态 1）不自动收货，避免售后中订单被错误完成并触发结算
        let in_review: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM aftersales WHERE order_id = $1 AND status = 1",
        )
        .bind(order_id)
        .fetch_one(&repos.pool)
        .await
        .unwrap_or(0);
        if in_review > 0 {
            continue;
        }
        if repos
            .trade
            .order
            .mark_received(order_id, now as i32)
            .await?
        {
            done += 1;
            // 自动收货 = Java confirmReceipt 同款收尾：完成日志 + 积分/成长值/
            // 次卡/出账单/结算级联（店铺结算、分销佣金、分销员升级）
            if let Ok(Some(order)) = repos.trade.order.find_by_id(order_id).await {
                let desc = if order.order_type == Some(10) {
                    "完成上门服务"
                } else {
                    "确认收货"
                };
                let _ = repos
                    .trade
                    .user_order
                    .insert_order_log(
                        order_id,
                        &order.order_sn,
                        order.user_id,
                        order.shop_id,
                        desc,
                        now as i32,
                    )
                    .await;
                crate::trade::settlement::SettlementService::new(repos.clone())
                    .after_order_completed(&order)
                    .await;
                // 自动评价收货快照（与买家手动确认收货共用同一入口）：
                // 自动收货同样是「确认收货」，须按店铺当时设置落到期快照。
                crate::catalog::auto_comment::AutoCommentService::new(repos.clone())
                    .snapshot_on_received_quiet(
                        order.order_id,
                        order.shop_id,
                        order.user_id,
                        now as i32,
                    )
                    .await;
            }
        }
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minutes_rules() {
        assert_eq!(parse_positive_i64(Some("30")), Some(30));
        assert_eq!(parse_positive_i64(Some(" 0 ")), None);
        assert_eq!(parse_positive_i64(Some("-5")), None);
        assert_eq!(parse_positive_i64(Some("abc")), None);
        assert_eq!(parse_positive_i64(None), None);
    }

    #[test]
    fn parse_days_rules() {
        assert_eq!(parse_positive_days(Some("7")), Some(7 * 86_400));
        assert_eq!(parse_positive_days(Some("0.5")), Some(43_200));
        assert_eq!(parse_positive_days(Some("0")), None);
        assert_eq!(parse_positive_days(None), None);
    }

    #[test]
    fn env_secs_falls_back() {
        assert_eq!(env_secs("NEXTTRADE_SCHEDULER__NOT_EXISTS", 42), 42);
    }
}
