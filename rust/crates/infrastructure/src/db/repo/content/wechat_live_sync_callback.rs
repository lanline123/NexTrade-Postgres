//! 微信直播订单同步回调 域仓储（content::wechat_live_sync_callback）。
//!
//! 表名/列名均来自权威 schema `java/database/pure.sql`
//! （核对命令：`cd rust && python tools/schema_parser.py --table <表名>`）：
//! - `order`：`order_id` / `order_sn` / `order_extension`（`order` 是 MySQL 保留字，
//!   必须反引号）
//! - `wechat_live_order_rel`：`id` / `order_id` / `room_id` / `sync_status` /
//!   `last_sync_time` / `update_time`
//! - `wechat_live_sync_log`：`order_id` / `aftersale_id` / `refund_id` / `room_id` /
//!   `event_type` / `direction` / `sync_status` / `idempotent_key` / `request_data` /
//!   `response_data` / `add_time` / `update_time` / `last_sync_time`
//!   （`id` 为 `AUTO_INCREMENT`，故 INSERT 不写 `id`）
//!
//! SQL 一律运行时 `sqlx::query_as` / `query`（**不用** `query!` 宏），
//! 每条 SQL 写成单行字面量。

use sqlx::postgres::PgQueryResult;
use sqlx::PgPool;
use nextrade_core::models::content::wechat_live_sync_callback::{
    apply_last_callback, CALLBACK_ACCEPTED_RESPONSE, DIRECTION_INBOUND, SYNC_STATUS_PARTIAL,
    SYNC_STATUS_SUCCESS,
};
use nextrade_core::{Error, Result};

/// 一次入站回调的落库参数（字段与 Java `handleInboundCallback` 里
/// `new WechatLiveSyncLog()` 的 setter 一一对应）。
#[derive(Debug, Clone)]
pub struct InboundLogWrite {
    /// 归因到的订单ID（可为 `None` → 与 Java 一致写 NULL）
    pub order_id: Option<i32>,
    /// 售后ID
    pub aftersale_id: Option<i32>,
    /// 退款ID
    pub refund_id: Option<i32>,
    /// 直播房间ID（DTO 优先，缺失时由 rel 表兜底）
    pub room_id: Option<i32>,
    /// 事件类型（已通过 `@NotBlank` 校验）
    pub event_type: String,
    /// 幂等键（已通过 `@NotBlank` 校验）
    pub idempotent_key: String,
    /// `JSONUtil.toJsonStr(callbackDTO)` 等价文本
    pub request_data: String,
    /// 回调快照（写入 `order_extension.wechatLiveLastCallback`）
    pub callback_snapshot: serde_json::Value,
    /// 秒级时间戳（Java `StringUtils.getCurrentTime()`，三个时间列同值）
    pub now: i64,
}

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct WechatLiveSyncCallbackRepository {
    pool: PgPool,
}

impl WechatLiveSyncCallbackRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按订单号反查订单ID。
    ///
    /// 对应 Java `resolveOrderId` 的 `orderSn` 分支
    /// （`WechatLiveOrderSyncServiceImpl.java:291-294`）：
    /// `eq(Order::getOrderSn, sn).last("limit 1")` → 取第一行。
    pub async fn find_order_id_by_sn(&self, order_sn: &str) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT order_id FROM \"order\" WHERE order_sn = $1 LIMIT 1")
                .bind(order_sn)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    /// 取该订单最新一条归因关系的房间ID。
    ///
    /// 对应 Java `handleInboundCallback` 的 roomId 兜底
    /// （`WechatLiveOrderSyncServiceImpl.java:242-246`）：
    /// `listByOrderId(orderId)`（`ORDER BY id DESC`）
    /// `.map(getRoomId).filter(nonNull).findFirst()`
    /// → 即「id 最大且 room_id 非空的那一行」。
    pub async fn find_latest_room_id_by_order(&self, order_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT room_id FROM wechat_live_order_rel WHERE order_id = $1 AND room_id IS NOT NULL ORDER BY id DESC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(room,)| room))
    }

    /// 入站回调落库（对应 Java `handleInboundCallback` 的写部分，
    /// 整体在 Java `@Transactional(rollbackFor = Exception.class)` 之内）：
    ///
    /// 1. `INSERT wechat_live_sync_log`（`direction=INBOUND`、
    ///    `sync_status=1成功`、`response_data={"message":"callback accepted"}`）；
    /// 2. 若归因到订单：把该订单**所有**归因关系置为
    ///    `sync_status=4部分同步` + 刷新 `last_sync_time` / `update_time`
    ///    （Java `updateRelationStatus(orderId, SYNC_STATUS_PARTIAL)` 的
    ///    `updateBatchById` 等价于一条按 `order_id` 的 UPDATE）；
    /// 3. 若订单行存在：把回调快照并入 `order_extension`
    ///    （Java `WechatLiveOrderContextUtils.applyLastCallback` + `updateById`）。
    ///
    /// 三步同事务：任一步失败整体回滚（与 Java 一致，不会残留日志行）。
    pub async fn write_inbound_callback(&self, w: &InboundLogWrite) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        // —— 1. 同步日志 —— //
        let _: PgQueryResult = sqlx::query(
            "INSERT INTO wechat_live_sync_log (order_id, aftersale_id, refund_id, room_id, event_type, direction, sync_status, idempotent_key, request_data, response_data, add_time, update_time, last_sync_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(w.order_id)
        .bind(w.aftersale_id)
        .bind(w.refund_id)
        .bind(w.room_id)
        .bind(w.event_type.as_str())
        .bind(DIRECTION_INBOUND)
        .bind(SYNC_STATUS_SUCCESS)
        .bind(w.idempotent_key.as_str())
        .bind(w.request_data.as_str())
        .bind(CALLBACK_ACCEPTED_RESPONSE)
        .bind(w.now)
        .bind(w.now)
        .bind(w.now)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        // —— 2 / 3. 仅在归因到订单时执行 —— //
        if let Some(order_id) = w.order_id {
            sqlx::query(
                "UPDATE wechat_live_order_rel SET sync_status = $1, last_sync_time = $2, update_time = $3 WHERE order_id = $4",
            )
            .bind(SYNC_STATUS_PARTIAL)
            .bind(w.now)
            .bind(w.now)
            .bind(order_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

            // Java `orderMapper.selectById(orderId)`：订单行不存在则跳过第 3 步
            let extension: Option<(Option<String>,)> =
                sqlx::query_as("SELECT order_extension FROM \"order\" WHERE order_id = $1 LIMIT 1")
                    .bind(order_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_err)?;

            if let Some((current,)) = extension {
                let merged = apply_last_callback(current.as_deref(), &w.callback_snapshot);
                sqlx::query("UPDATE \"order\" SET order_extension = $1 WHERE order_id = $2")
                    .bind(merged)
                    .bind(order_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(map_err)?;
            }
        }

        tx.commit().await.map_err(map_err)?;
        Ok(())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}