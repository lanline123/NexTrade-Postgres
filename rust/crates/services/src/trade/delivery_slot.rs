//! 同城配送送达时间选择服务（我方新增，论坛帖 467744 + 预约配送
//! 子项 297743，用户 2026-11-21 拍板 B 类缓做全部开工）。
//!
//! 形态：时段定义挂门店同城配送配置（`shop_local_delivery_config`
//! 增量列，迁移 025）；买家选择独立落 `order_delivery_slot` 一行
//! （挂根订单，拆单子单经 parent_order_id 解析，与 `order_gift`
//! 同模式），不侵入订单状态/金额结构。
//!
//! 口径（纯逻辑在 `models::trade::delivery_slot`，此处只做 DB 侧）：
//! - 开关默认关：店铺未开启时段选择时，结算不下发选项、下单不
//!   要求选择，存量行为逐字不变；
//! - 开启后同城配送（shipping_method = 3）下单必选时段，逐店校
//!   验（日期范围/时段命中/截单/容量）；POS 收银单豁免（商家当
//!   面单不走买家时段口径，由下单入口控制）；
//! - 容量统计：同店同时段开始时间戳下、未取消（order_status 非
//!   3/4）、未删除、有效的**根订单**数（子单不重复计；店铺归属
//!   经 order_item 逐行 shop_id 判定，跨店混单对每个涉事店铺各
//!   计一单）；下单校验在建单事务外先行、落行在事务内，极端并
//!   发下可能短暂超限一单，属可接受口径（与预约库存同级），
//!   商家可按时段总量备货；
//! - 配送费口径与本服务无关：结算运费唯一路径为运费模板
//!   （`trade::check::calculate_shipping`），同城与快递同享满额
//!   包邮判定，见该处注释与本线报告核查结论。

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::Row;
use nextrade_core::entities::o2o::ShopLocalDeliveryConfig;
use nextrade_core::models::trade::delivery_slot::{
    build_day_options, parse_slot_defs, validate_selection, DeliveryDayOption,
    DeliverySlotSelection, ResolvedSlot,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// order_delivery_slot 行（服务内投影，VO 组装共用）。
#[derive(Debug, Clone)]
pub struct DeliverySlotRow {
    pub order_id: i32,
    pub shop_id: i32,
    pub slot_start: i32,
    pub slot_end: i32,
    pub slot_text: String,
}

pub struct DeliverySlotService {
    repos: Arc<Repositories>,
}

impl DeliverySlotService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 店铺是否开启时段选择（配置存在 + enabled + time_slot_enabled）。
    fn slot_cfg(cfg: Option<&ShopLocalDeliveryConfig>) -> Option<&ShopLocalDeliveryConfig> {
        cfg.filter(|c| c.enabled == 1 && c.time_slot_enabled == 1)
    }

    /// 结算页选项：店铺开启时段选择时返回逐日选项（含已约数），
    /// 未开启返回 None（前端不渲染选择区）。
    pub async fn options_for_shop(
        &self,
        shop_id: i32,
    ) -> Result<Option<Vec<DeliveryDayOption>>> {
        let cfg = self.repos.o2o.config.find_local_delivery(shop_id).await?;
        let Some(cfg) = Self::slot_cfg(cfg.as_ref()) else {
            return Ok(None);
        };
        let defs = parse_slot_defs(&cfg.time_slots);
        if defs.is_empty() {
            return Ok(None);
        }
        let now = chrono::Local::now();
        let days = cfg.booking_days.clamp(1, 7);
        // 已约数：窗口内该店全部时段一次查回（按 slot_start 分组）
        let window_start = now.timestamp();
        let window_end = window_start + i64::from(days + 1) * 86_400;
        let rows = sqlx::query(
            "SELECT s.slot_start, COUNT(DISTINCT s.order_id) AS n FROM order_delivery_slot s \
             JOIN \"order\" o ON o.order_id = s.order_id \
             JOIN order_item oi ON oi.order_id = s.order_id AND oi.shop_id = $1 \
             WHERE s.slot_start >= $2 AND s.slot_start < $3 \
               AND o.is_valid = 1 AND o.is_del = 0 AND o.order_status NOT IN (3, 4) \
             GROUP BY s.slot_start",
        )
        .bind(shop_id)
        .bind(window_start)
        .bind(window_end)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let mut booked: HashMap<i32, i64> = HashMap::new();
        for r in rows {
            let start: i32 = r.try_get("slot_start").unwrap_or(0);
            let n: i64 = r.try_get("n").unwrap_or(0);
            booked.insert(start, n);
        }
        Ok(Some(build_day_options(
            &defs,
            cfg.booking_days,
            cfg.cutoff_minutes,
            now,
            &booked,
        )))
    }

    /// 下单校验：同城配送时对每个开启时段选择的涉单店铺逐店校验。
    ///
    /// 返回落库用时段（无任何涉单店铺开启时返回 None = 不落行）；
    /// 任一开启店铺校验不过 → 明确报错（不静默降级）。
    pub async fn validate_for_order(
        &self,
        shop_ids: &[i32],
        selection: Option<&DeliverySlotSelection>,
    ) -> Result<Option<ResolvedSlot>> {
        let now = chrono::Local::now();
        let mut resolved: Option<ResolvedSlot> = None;
        let mut any_enabled = false;
        for shop_id in shop_ids {
            let cfg = self.repos.o2o.config.find_local_delivery(*shop_id).await?;
            let Some(cfg) = Self::slot_cfg(cfg.as_ref()) else {
                continue;
            };
            any_enabled = true;
            let defs = parse_slot_defs(&cfg.time_slots);
            if defs.is_empty() {
                continue;
            }
            let sel = selection.ok_or_else(|| {
                Error::biz(400, "请选择送达时间".to_string())
            })?;
            let r = validate_selection(&defs, cfg.booking_days, cfg.cutoff_minutes, now, sel)
                .map_err(|msg| Error::biz(400, msg))?;
            // 容量校验：命中时段定义有上限时统计已约数
            if let Some(def) = defs
                .iter()
                .find(|d| d.start == sel.start.trim() && d.end == sel.end.trim())
            {
                if def.capacity > 0 {
                    let booked = self.count_booked(*shop_id, r.slot_start).await?;
                    if booked >= i64::from(def.capacity) {
                        return Err(Error::biz(
                            400,
                            "所选送达时段已约满，请选择其他时段".to_string(),
                        ));
                    }
                }
            }
            resolved = Some(r);
        }
        if any_enabled && resolved.is_none() {
            // 开启了开关但未配置任何时段：不拦截下单（配置不完整由商家自担）
            return Ok(None);
        }
        Ok(resolved)
    }

    /// 已约数（容量口径：根订单计数；店铺归属经 order_item 逐行
    /// shop_id 判定，与 options_for_shop 的已约数同口径）。
    async fn count_booked(&self, shop_id: i32, slot_start: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT s.order_id) FROM order_delivery_slot s \
             JOIN \"order\" o ON o.order_id = s.order_id \
             JOIN order_item oi ON oi.order_id = s.order_id AND oi.shop_id = $1 \
             WHERE s.slot_start = $2 \
               AND o.is_valid = 1 AND o.is_del = 0 AND o.order_status NOT IN (3, 4)",
        )
        .bind(shop_id)
        .bind(slot_start)
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.0)
    }

    /// 与订单同事务写入时段行（order_delivery_slot，迁移 025）：
    /// 「有订单必有时段行」原子成立。shop_id 落根订单头店铺
    /// （header_shop_id 口径，仅作行内标记；容量统计按 order_item
    /// 归属逐店计，不依赖此列）。
    pub async fn insert_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        order_id: i32,
        order_sn: &str,
        shop_id: i32,
        slot: &ResolvedSlot,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO order_delivery_slot \
             (order_id, order_sn, shop_id, slot_start, slot_end, slot_text, add_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (order_id) DO NOTHING",
        )
        .bind(order_id)
        .bind(order_sn)
        .bind(shop_id)
        .bind(slot.slot_start)
        .bind(slot.slot_end)
        .bind(&slot.slot_text)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 按订单解析时段行（子单经 parent_order_id 回到根单）。
    pub async fn resolve_for_order(&self, order_id: i32) -> Result<Option<DeliverySlotRow>> {
        if let Some(row) = self.find_by_order_id(order_id).await? {
            return Ok(Some(row));
        }
        let parent: Option<i32> = sqlx::query("SELECT parent_order_id FROM \"order\" WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?
            .map(|r| r.try_get::<i32, _>("parent_order_id").unwrap_or(0));
        match parent {
            Some(pid) if pid > 0 => self.find_by_order_id(pid).await,
            _ => Ok(None),
        }
    }

    /// 批量解析（订单列表组装用）：入参 (order_id, parent_order_id)，
    /// 返回 order_id → 时段行（子单命中根单行时以子单 id 为键）。
    pub async fn resolve_map(
        &self,
        keys: &[(i32, Option<i32>)],
    ) -> Result<HashMap<i32, DeliverySlotRow>> {
        let mut want: Vec<i32> = Vec::new();
        for (oid, parent) in keys {
            want.push(*oid);
            if let Some(pid) = parent {
                if *pid > 0 {
                    want.push(*pid);
                }
            }
        }
        want.sort_unstable();
        want.dedup();
        if want.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query(
            "SELECT order_id, shop_id, slot_start, slot_end, slot_text \
             FROM order_delivery_slot WHERE order_id = ANY($1)",
        )
        .bind(&want)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let mut by_root: HashMap<i32, DeliverySlotRow> = HashMap::new();
        for r in rows {
            let row = Self::map_row(&r);
            by_root.insert(row.order_id, row);
        }
        let mut out = HashMap::new();
        for (oid, parent) in keys {
            if let Some(row) = by_root.get(oid) {
                out.insert(*oid, row.clone());
            } else if let Some(pid) = parent {
                if let Some(row) = by_root.get(pid) {
                    out.insert(*oid, row.clone());
                }
            }
        }
        Ok(out)
    }

    async fn find_by_order_id(&self, order_id: i32) -> Result<Option<DeliverySlotRow>> {
        let row = sqlx::query(
            "SELECT order_id, shop_id, slot_start, slot_end, slot_text \
             FROM order_delivery_slot WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.as_ref().map(Self::map_row))
    }

    fn map_row(r: &sqlx::postgres::PgRow) -> DeliverySlotRow {
        DeliverySlotRow {
            order_id: r.try_get("order_id").unwrap_or(0),
            shop_id: r.try_get("shop_id").unwrap_or(0),
            slot_start: r.try_get("slot_start").unwrap_or(0),
            slot_end: r.try_get("slot_end").unwrap_or(0),
            slot_text: r.try_get("slot_text").unwrap_or_default(),
        }
    }
}
