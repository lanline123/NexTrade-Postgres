//! 后厨出单与催菜服务（我方新增，论坛帖 817927，用户 2026-11-21 拍板）。
//!
//! 形态与状态机（后厨子状态独立落 `order_kitchen` 表，不侵入订单状态枚举）：
//!
//! ```text
//! 订单付款(order_status=1 待发货) ──► 待制作(0) ──开始制作──► 制作中(1) ──出单──► 已出单(2)
//!                                        │（无后厨行 = 待制作，懒物化）        └─► 回归正常发货/核销链
//! 买家催菜 ──► order_kitchen_urge 流水 + 后厨行计数/未读标记 + admin_msg 类型 58 落商家消息中心
//! ```
//!
//! 口径要点：
//! - 后厨队列的订单范围 = 履约单位：已付款待发货（`order_status = 1`、
//!   有效、未删），未拆单取根订单、已拆单取子单（根单是付款壳，
//!   `is_store_splited = 1` 的根单不入板防双计），与拆单模型一致。
//! - 催菜频控：同一买家对同一订单 10 分钟冷却 + 自然日上限 5 次，
//!   判定纯函数在 core `models::trade::kitchen::urge_decision`。
//! - 催菜提醒双通道：后厨看板高亮（未读标记，后厨点「已知晓」清除）
//!   + 后台消息中心 `admin_msg`（类型 58「买家催菜」，店铺维度），
//!   商家不在后厨页时也能从消息中心看到。
//! - 已出单只代表后厨完工，不动订单状态：发货/自提核销仍走既有链。

use std::sync::Arc;

use sqlx::Row;
use nextrade_common::utils;
use nextrade_core::models::trade::kitchen::{
    kitchen_status, kitchen_status_name, transition_allowed, urge_decision, KitchenBoardQuery,
    KitchenBoardVO, KitchenInfoVO, KitchenItemVO, KitchenOrderVO, KitchenUrgeResultVO,
    UrgeDecision, MSG_TYPE_URGE, URGE_COOLDOWN_SECONDS, URGE_DAILY_LIMIT,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 订单行最小投影（后厨校验用）。
struct OrderHead {
    order_sn: String,
    user_id: i32,
    shop_id: i32,
    order_status: i32,
    is_valid: i32,
}

pub struct KitchenService {
    repos: Arc<Repositories>,
}

impl KitchenService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————— 内部读写 —————————————————

    async fn load_order_head(&self, order_id: i32) -> Result<Option<OrderHead>> {
        let row = sqlx::query(
            "SELECT order_sn, user_id, shop_id, order_status, is_valid FROM \"order\" \
             WHERE order_id = $1 AND is_del = 0",
        )
        .bind(order_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.map(|r| OrderHead {
            order_sn: r.get::<String, _>("order_sn"),
            user_id: r.get::<i32, _>("user_id"),
            shop_id: r.get::<i32, _>("shop_id"),
            order_status: r.get::<i32, _>("order_status"),
            is_valid: r.get::<i32, _>("is_valid"),
        }))
    }

    /// 当前后厨状态（无行 = 待制作）。
    async fn current_status(&self, order_id: i32) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "SELECT kitchen_status::int FROM order_kitchen WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(v.unwrap_or(kitchen_status::PENDING))
    }

    fn ensure_shop_scope(scope: Option<i32>, order_shop_id: i32) -> Result<()> {
        if let Some(s) = scope {
            if s > 0 && s != order_shop_id {
                return Err(Error::Forbidden("无权操作其它店铺的订单".into()));
            }
        }
        Ok(())
    }

    // ————————————————— 后厨看板（管理端） —————————————————

    /// 后厨看板：计数 + 当前页订单卡（含商品行批量组装）。
    pub async fn board(
        &self,
        scope_shop_id: Option<i32>,
        q: &KitchenBoardQuery,
    ) -> Result<KitchenBoardVO> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.size.unwrap_or(60).clamp(1, 200);
        let offset = (page - 1) * size;
        let keyword = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        // 店铺视角以登录态范围为准，平台视角可按 shop_id 过滤
        let shop_filter: Option<i32> = match scope_shop_id {
            Some(s) if s > 0 => Some(s),
            _ => q.shop_id.filter(|s| *s > 0),
        };
        let status_filter: Option<i32> = q.status.filter(|s| *s >= 0);

        // 计数（与列表同一订单范围口径）
        let counts = sqlx::query(
            "SELECT COALESCE(k.kitchen_status, 0)::int AS ks, COUNT(*)::bigint AS c, \
                    COALESCE(SUM(COALESCE(k.urge_unread, 0)) , 0)::bigint AS unread_orders \
             FROM \"order\" o LEFT JOIN order_kitchen k ON k.order_id = o.order_id \
             WHERE o.order_status = 1 AND o.is_valid = 1 AND o.is_del = 0 \
               AND (o.parent_order_id IS NOT NULL OR COALESCE(o.is_store_splited, 0) = 0) \
               AND ($1::int IS NULL OR o.shop_id = $1) \
             GROUP BY COALESCE(k.kitchen_status, 0)",
        )
        .bind(shop_filter)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let (mut pending, mut cooking, mut done, mut unread) = (0i64, 0i64, 0i64, 0i64);
        for r in &counts {
            let ks: i32 = r.get("ks");
            let c: i64 = r.get("c");
            match ks {
                kitchen_status::PENDING => pending = c,
                kitchen_status::COOKING => cooking = c,
                kitchen_status::DONE => done = c,
                _ => {}
            }
            // urge_unread 是行级 0/1，SUM 即未读催菜的「行数」近似；精确口径取列表侧标记
            unread += r.get::<i64, _>("unread_orders");
        }

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM \"order\" o \
             LEFT JOIN order_kitchen k ON k.order_id = o.order_id \
             WHERE o.order_status = 1 AND o.is_valid = 1 AND o.is_del = 0 \
               AND (o.parent_order_id IS NOT NULL OR COALESCE(o.is_store_splited, 0) = 0) \
               AND ($1::int IS NULL OR o.shop_id = $1) \
               AND ($2::int IS NULL OR COALESCE(k.kitchen_status, 0) = $2) \
               AND ($3::text IS NULL OR o.order_sn ILIKE $3)",
        )
        .bind(shop_filter)
        .bind(status_filter)
        .bind(keyword.as_deref())
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let rows = sqlx::query(
            "SELECT o.order_id, o.order_sn, o.shop_id, o.consignee, o.mobile, o.buyer_note, \
                    o.shipping_type_name, o.pay_time, o.add_time, \
                    COALESCE(k.kitchen_status, 0)::int AS ks, \
                    COALESCE(k.urge_count, 0) AS urge_count, \
                    COALESCE(k.last_urge_time, 0) AS last_urge_time, \
                    COALESCE(k.urge_unread, 0)::int AS urge_unread \
             FROM \"order\" o LEFT JOIN order_kitchen k ON k.order_id = o.order_id \
             WHERE o.order_status = 1 AND o.is_valid = 1 AND o.is_del = 0 \
               AND (o.parent_order_id IS NOT NULL OR COALESCE(o.is_store_splited, 0) = 0) \
               AND ($1::int IS NULL OR o.shop_id = $1) \
               AND ($2::int IS NULL OR COALESCE(k.kitchen_status, 0) = $2) \
               AND ($3::text IS NULL OR o.order_sn ILIKE $3) \
             ORDER BY COALESCE(k.urge_unread, 0) DESC, o.pay_time ASC, o.order_id ASC \
             LIMIT $4 OFFSET $5",
        )
        .bind(shop_filter)
        .bind(status_filter)
        .bind(keyword.as_deref())
        .bind(size)
        .bind(offset)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // 商品行批量组装（防 N+1）
        let order_ids: Vec<i32> = rows.iter().map(|r| r.get::<i32, _>("order_id")).collect();
        let mut items_by_order: std::collections::HashMap<i32, Vec<KitchenItemVO>> =
            std::collections::HashMap::new();
        if !order_ids.is_empty() {
            let item_rows = sqlx::query(
                "SELECT item_id, order_id, product_name, pic_thumb, sku_data, quantity \
                 FROM order_item WHERE order_id = ANY($1) ORDER BY item_id ASC",
            )
            .bind(&order_ids)
            .fetch_all(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
            for r in &item_rows {
                items_by_order
                    .entry(r.get::<i32, _>("order_id"))
                    .or_default()
                    .push(KitchenItemVO {
                        item_id: r.get::<i32, _>("item_id"),
                        product_name: r.get::<String, _>("product_name"),
                        pic_thumb: r.get::<String, _>("pic_thumb"),
                        sku_data: r.get::<String, _>("sku_data"),
                        quantity: r.get::<i32, _>("quantity"),
                    });
            }
        }

        let now = utils::now_seconds() as i32;
        let orders = rows
            .iter()
            .map(|r| {
                let pay_time: i32 = r.get("pay_time");
                let add_time: i32 = r.get("add_time");
                let base = if pay_time > 0 { pay_time } else { add_time };
                let ks: i32 = r.get("ks");
                let last_urge: i32 = r.get("last_urge_time");
                KitchenOrderVO {
                    order_id: r.get::<i32, _>("order_id"),
                    order_sn: r.get::<String, _>("order_sn"),
                    shop_id: r.get::<i32, _>("shop_id"),
                    consignee: r.get::<String, _>("consignee"),
                    mobile: r.get::<String, _>("mobile"),
                    buyer_note: r.get::<String, _>("buyer_note"),
                    shipping_type_name: r.get::<String, _>("shipping_type_name"),
                    pay_time: fmt_time(pay_time),
                    wait_minutes: i64::from(now.saturating_sub(base).max(0)) / 60,
                    kitchen_status: ks,
                    kitchen_status_name: kitchen_status_name(ks).to_string(),
                    urge_count: r.get::<i32, _>("urge_count"),
                    last_urge_time: fmt_time(last_urge),
                    urge_unread: r.get::<i32, _>("urge_unread") > 0,
                    items: items_by_order
                        .get(&r.get::<i32, _>("order_id"))
                        .cloned()
                        .unwrap_or_default(),
                }
            })
            .collect();

        Ok(KitchenBoardVO {
            pending_count: pending,
            cooking_count: cooking,
            done_count: done,
            urge_unread_count: unread,
            total,
            orders,
        })
    }

    /// 后厨状态流转（只进不退；订单须仍为已付款待发货）。
    pub async fn transition(
        &self,
        scope_shop_id: Option<i32>,
        order_id: i32,
        target_status: i32,
    ) -> Result<KitchenInfoVO> {
        let head = self
            .load_order_head(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        Self::ensure_shop_scope(scope_shop_id, head.shop_id)?;
        if head.order_status != 1 || head.is_valid != 1 {
            return Err(Error::Biz {
                code: 1001,
                msg: "订单已不在待发货状态，后厨状态不可再流转".into(),
            });
        }
        let current = self.current_status(order_id).await?;
        if !transition_allowed(current, target_status) {
            return Err(Error::Biz {
                code: 1001,
                msg: format!(
                    "后厨状态只能向前流转（当前「{}」，目标「{}」不合法）",
                    kitchen_status_name(current),
                    kitchen_status_name(target_status)
                ),
            });
        }
        let now = utils::now_seconds() as i32;
        let start_time = if target_status == kitchen_status::COOKING {
            now
        } else {
            0
        };
        let done_time = if target_status == kitchen_status::DONE {
            now
        } else {
            0
        };
        sqlx::query(
            "INSERT INTO order_kitchen \
             (order_id, shop_id, kitchen_status, start_time, done_time, add_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $6) \
             ON CONFLICT (order_id) DO UPDATE SET \
               kitchen_status = EXCLUDED.kitchen_status, \
               start_time = CASE WHEN EXCLUDED.start_time > 0 AND order_kitchen.start_time = 0 \
                                 THEN EXCLUDED.start_time ELSE order_kitchen.start_time END, \
               done_time = CASE WHEN EXCLUDED.done_time > 0 \
                                THEN EXCLUDED.done_time ELSE order_kitchen.done_time END, \
               shop_id = EXCLUDED.shop_id, \
               update_time = EXCLUDED.update_time",
        )
        .bind(order_id)
        .bind(head.shop_id)
        .bind(target_status)
        .bind(start_time)
        .bind(done_time)
        .bind(now)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(KitchenInfoVO {
            order_id,
            in_kitchen: true,
            kitchen_status: target_status,
            kitchen_status_name: kitchen_status_name(target_status).to_string(),
            ..Default::default()
        })
    }

    /// 后厨确认已知晓催菜（清除未读标记，计数保留）。
    pub async fn ack_urge(
        &self,
        scope_shop_id: Option<i32>,
        order_id: i32,
    ) -> Result<bool> {
        let head = self
            .load_order_head(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        Self::ensure_shop_scope(scope_shop_id, head.shop_id)?;
        let now = utils::now_seconds() as i32;
        sqlx::query(
            "UPDATE order_kitchen SET urge_unread = 0, update_time = $2 WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(now)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(true)
    }

    // ————————————————— 催菜（买家端） —————————————————

    /// 该买家对该订单的催菜频控现状（上次时间 + 今日次数）。
    async fn urge_window(&self, order_id: i32, user_id: i32) -> Result<(i32, i64)> {
        let last: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(add_time), 0) FROM order_kitchen_urge \
             WHERE order_id = $1 AND user_id = $2",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let now = utils::now_seconds() as i32;
        let today_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM order_kitchen_urge \
             WHERE order_id = $1 AND user_id = $2 AND add_time >= $3",
        )
        .bind(order_id)
        .bind(user_id)
        .bind(today_start(now))
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok((last, today_count))
    }

    /// 买家催菜：校验归属与状态 → 频控 → 落流水/计数/未读 → 消息中心提醒商家。
    pub async fn urge(&self, user_id: i32, order_id: i32) -> Result<KitchenUrgeResultVO> {
        let head = self
            .load_order_head(order_id)
            .await?
            .filter(|h| h.user_id == user_id)
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        if head.order_status != 1 || head.is_valid != 1 {
            return Err(Error::Biz {
                code: 1001,
                msg: "订单不在制作阶段，无法催菜".into(),
            });
        }
        let ks = self.current_status(order_id).await?;
        if ks == kitchen_status::DONE {
            return Err(Error::Biz {
                code: 1001,
                msg: "餐品已出单，正在为您安排发货/取餐".into(),
            });
        }
        let now = utils::now_seconds() as i32;
        let (last, today_count) = self.urge_window(order_id, user_id).await?;
        match urge_decision(now, last, today_count) {
            UrgeDecision::Allowed => {}
            UrgeDecision::Cooldown { remain_seconds } => {
                return Err(Error::Biz {
                    code: 1001,
                    msg: format!(
                        "已催过啦，商家正在赶工，请 {} 分钟后再催",
                        (remain_seconds + 59) / 60
                    ),
                });
            }
            UrgeDecision::DailyLimitReached => {
                return Err(Error::Biz {
                    code: 1001,
                    msg: format!("今日催菜已达 {URGE_DAILY_LIMIT} 次上限，商家已收到您的催促"),
                });
            }
        }

        sqlx::query(
            "INSERT INTO order_kitchen_urge (order_id, shop_id, user_id, add_time) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(order_id)
        .bind(head.shop_id)
        .bind(user_id)
        .bind(now)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let urge_count: i32 = sqlx::query_scalar(
            "INSERT INTO order_kitchen \
             (order_id, shop_id, kitchen_status, urge_count, last_urge_time, urge_unread, add_time, update_time) \
             VALUES ($1, $2, 0, 1, $3, 1, $3, $3) \
             ON CONFLICT (order_id) DO UPDATE SET \
               urge_count = order_kitchen.urge_count + 1, \
               last_urge_time = EXCLUDED.last_urge_time, \
               urge_unread = 1, \
               update_time = EXCLUDED.update_time \
             RETURNING urge_count",
        )
        .bind(order_id)
        .bind(head.shop_id)
        .bind(now)
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // 商家提醒：后台消息中心（类型 58「买家催菜」，店铺维度落行）
        self.repos
            .system
            .admin_msg
            .create_message(
                MSG_TYPE_URGE,
                head.shop_id,
                order_id,
                "买家催菜",
                &format!(
                    "订单 {} 的买家已催菜（第 {} 次），请尽快出单",
                    head.order_sn, urge_count
                ),
                &serde_json::json!({
                    "orderId": order_id,
                    "orderSn": head.order_sn,
                    "urgeCount": urge_count,
                })
                .to_string(),
            )
            .await?;

        Ok(KitchenUrgeResultVO {
            order_id,
            urge_count,
            cooldown_seconds: URGE_COOLDOWN_SECONDS,
        })
    }

    /// 买家订单详情后厨面板：状态 + 可否催菜 + 冷却/剩余额度。
    pub async fn buyer_info(&self, user_id: i32, order_id: i32) -> Result<KitchenInfoVO> {
        let head = self
            .load_order_head(order_id)
            .await?
            .filter(|h| h.user_id == user_id)
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        let in_kitchen = head.order_status == 1 && head.is_valid == 1;
        let ks = self.current_status(order_id).await?;
        let urge_count: i32 = sqlx::query_scalar(
            "SELECT COALESCE(urge_count, 0) FROM order_kitchen WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .unwrap_or(0);
        let now = utils::now_seconds() as i32;
        let (last, today_count) = self.urge_window(order_id, user_id).await?;
        let decision = urge_decision(now, last, today_count);
        let (can_urge, remain) = match decision {
            UrgeDecision::Allowed => (in_kitchen && ks != kitchen_status::DONE, 0),
            UrgeDecision::Cooldown { remain_seconds } => (false, remain_seconds),
            UrgeDecision::DailyLimitReached => (false, 0),
        };
        Ok(KitchenInfoVO {
            order_id,
            in_kitchen,
            kitchen_status: ks,
            kitchen_status_name: kitchen_status_name(ks).to_string(),
            urge_count,
            can_urge,
            cooldown_remain_seconds: remain,
            daily_urges_left: (URGE_DAILY_LIMIT - today_count).max(0),
        })
    }
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（0 或负数返回空串，与订单域 `fmt_time` 同口径）。
fn fmt_time(secs: i32) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 本地时区今日零点（秒级时间戳），催菜日限窗口用。
fn today_start(now: i32) -> i32 {
    use chrono::TimeZone;
    let Some(dt) = chrono::Local.timestamp_opt(i64::from(now), 0).single() else {
        return 0;
    };
    let Some(naive) = dt.date_naive().and_hms_opt(0, 0, 0) else {
        return 0;
    };
    chrono::Local
        .from_local_datetime(&naive)
        .single()
        .map(|d| d.timestamp() as i32)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_time_zero_is_empty() {
        assert_eq!(fmt_time(0), "");
        assert_eq!(fmt_time(-5), "");
        assert!(!fmt_time(1_763_000_000).is_empty());
    }

    #[test]
    fn today_start_not_after_now() {
        let now = 1_763_000_123;
        let start = today_start(now);
        assert!(start > 0 && start <= now && now - start < 86_400);
    }
}
