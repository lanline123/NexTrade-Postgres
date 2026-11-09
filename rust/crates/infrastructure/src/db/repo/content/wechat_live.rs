//! 微信直播仓储（`content/wechat_live`）。
//!
//! 对应原 Java `WechatLiveServiceImpl` 中直接使用的 Mapper：
//! `WechatLiveMapper`（直播间 CRUD/分页）、`WechatLiveGoodsMapper`（直播商品）、
//! `WechatLiveRoomGoodsMapper`（直播间-商品关联）、`WechatLiveOrderRelMapper`
//! （订单归因计数）、`WechatLiveSyncLogMapper`（同步日志失败计数），以及
//! `ProductMapper` / `ProductSkuMapper`（添加商品时的规格校验）。
//!
//! 表结构（`python tools/schema_parser.py --table wechat_live*`）：
//! `wechat_live` / `wechat_live_goods` / `wechat_live_room_goods` /
//! `wechat_live_order_rel` / `wechat_live_sync_log`。
//!
//! ⚠️ `wechat_live.create_time` 为 `datetime`（实体映射 `Option<String>`），
//! 创建时间范围过滤用 `?` 绑定 `yyyy-MM-dd HH:mm:ss` 字符串。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query`（禁止 `query!` 宏）。

use sqlx::PgPool;

use nextrade_core::entities::{
    Product, ProductSku, WechatLive, WechatLiveGoods, WechatLiveRoomGoods,
};
use nextrade_core::models::content::wechat_live::WechatLiveListQuery;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct WechatLiveRepository {
    pool: PgPool,
}

impl WechatLiveRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 直播间 —————————————————————

    /// 直播间分页（Java `list`：关键词/标题 LIKE、状态、店铺、创建时间范围）。
    ///
    /// 排序：Java `buildSortOrder` 空排序字段 → 主键 `wechat_live_id` 降序；
    /// `sortField` 支持 `createTime` / `liveStatus` / `wechatLiveId`。
    pub async fn paginate(
        &self,
        q: &WechatLiveListQuery,
        page: &PageQuery,
        shop_id: Option<i32>,
    ) -> Result<(u64, Vec<WechatLive>)> {
        let mut wh = String::from(" WHERE 1 = 1");
        let mut binds: Vec<String> = Vec::new();
        if let Some(shop_id) = shop_id {
            // 平台素材规则：店铺账号可见 `shop_id = 0`（平台直播间）+ 本店
            wh.push_str(" AND (shop_id = 0 OR shop_id = ?)");
            binds.push(shop_id.to_string());
        }
        if let Some(kw) = q.keyword.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            push_like(&mut wh, &mut binds, " AND wechat_live_title LIKE ?", &kw);
        }
        if let Some(t) = q
            .wechat_live_title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            push_like(&mut wh, &mut binds, " AND wechat_live_title LIKE ?", t);
        }
        let live_status = q.valid_live_status();
        if let Some(s) = live_status {
            wh.push_str(" AND live_status = ?");
            binds.push(s.to_string());
        }
        if let Some(start) = q.start_time.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            wh.push_str(" AND create_time >= ?");
            binds.push(format!("{start} 00:00:00"));
        }
        if let Some(end) = q.end_time.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            wh.push_str(" AND create_time <= ?");
            binds.push(format!("{end} 23:59:59"));
        }
        let total_sql = format!("SELECT COUNT(*) FROM wechat_live{wh}");
        let mut tq = sqlx::query_as::<_, (i64,)>(&total_sql);
        for b in &binds {
            tq = tq.bind(b);
        }
        let total: (i64,) = tq.fetch_one(&self.pool).await.map_err(map_err)?;

        let column = match q.sort_field.as_deref() {
            Some("createTime") | Some("create_time") => "create_time",
            Some("liveStatus") | Some("live_status") => "live_status",
            _ => "wechat_live_id",
        };
        let dir = if q.sort_order.as_deref().is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        let (limit, offset) = page.limit_offset();
        let list_sql =
            format!("SELECT * FROM wechat_live{wh} ORDER BY {column} {dir} LIMIT $1 OFFSET $2");
        let mut lq = sqlx::query_as::<_, WechatLive>(&list_sql);
        for b in &binds {
            lq = lq.bind(b);
        }
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<Option<WechatLive>> {
        sqlx::query_as::<_, WechatLive>("SELECT * FROM wechat_live WHERE wechat_live_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_by_room_id(&self, room_id: i32) -> Result<Option<WechatLive>> {
        sqlx::query_as::<_, WechatLive>("SELECT * FROM wechat_live WHERE room_id = $1 LIMIT 1")
            .bind(room_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn insert(&self, w: &WechatLive) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO wechat_live (
                wechat_live_title, wechat_live_data, act_range, act_range_ext, room_id, share_img,
                live_type, close_like, close_goods, close_comment, close_kf, close_replay,
                is_feeds_public, feeds_img, cover_img, anchor_id, anchor_openid, anchor_name,
                anchor_img, creator_openid, live_status, start_time, end_time, last_update_time,
                product_data, thumb_img, live_sn, shop_id, sub_anchor_wechat, audit_status, create_time
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31) RETURNING wechat_live_id",
        )
        .bind(&w.wechat_live_title)
        .bind(&w.wechat_live_data)
        .bind(w.act_range)
        .bind(&w.act_range_ext)
        .bind(w.room_id)
        .bind(&w.share_img)
        .bind(w.live_type)
        .bind(w.close_like)
        .bind(w.close_goods)
        .bind(w.close_comment)
        .bind(w.close_kf)
        .bind(w.close_replay)
        .bind(w.is_feeds_public)
        .bind(&w.feeds_img)
        .bind(&w.cover_img)
        .bind(&w.anchor_id)
        .bind(&w.anchor_openid)
        .bind(&w.anchor_name)
        .bind(&w.anchor_img)
        .bind(&w.creator_openid)
        .bind(w.live_status)
        .bind(w.start_time)
        .bind(w.end_time)
        .bind(w.last_update_time)
        .bind(&w.product_data)
        .bind(&w.thumb_img)
        .bind(&w.live_sn)
        .bind(w.shop_id)
        .bind(&w.sub_anchor_wechat)
        .bind(w.audit_status)
        .bind(&w.create_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新（Java `updateById`；调用方已按 `NOT_NULL` 语义合并旧值）。
    pub async fn update(&self, w: &WechatLive) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE wechat_live SET wechat_live_title = $1, wechat_live_data = $2, act_range = $3,
                act_range_ext = $4, room_id = $5, share_img = $6, live_type = $7, close_like = $8,
                close_goods = $9, close_comment = $10, close_kf = $11, close_replay = $12,
                is_feeds_public = $13, feeds_img = $14, cover_img = $15, anchor_id = $16, anchor_openid = $17,
                anchor_name = $18, anchor_img = $19, creator_openid = $20, live_status = $21, start_time = $22,
                end_time = $23, last_update_time = $24, product_data = $25, thumb_img = $26, live_sn = $27,
                shop_id = $28, sub_anchor_wechat = $29, audit_status = $30, create_time = $31
             WHERE wechat_live_id = $32",
        )
        .bind(&w.wechat_live_title)
        .bind(&w.wechat_live_data)
        .bind(w.act_range)
        .bind(&w.act_range_ext)
        .bind(w.room_id)
        .bind(&w.share_img)
        .bind(w.live_type)
        .bind(w.close_like)
        .bind(w.close_goods)
        .bind(w.close_comment)
        .bind(w.close_kf)
        .bind(w.close_replay)
        .bind(w.is_feeds_public)
        .bind(&w.feeds_img)
        .bind(&w.cover_img)
        .bind(&w.anchor_id)
        .bind(&w.anchor_openid)
        .bind(&w.anchor_name)
        .bind(&w.anchor_img)
        .bind(&w.creator_openid)
        .bind(w.live_status)
        .bind(w.start_time)
        .bind(w.end_time)
        .bind(w.last_update_time)
        .bind(&w.product_data)
        .bind(&w.thumb_img)
        .bind(&w.live_sn)
        .bind(w.shop_id)
        .bind(&w.sub_anchor_wechat)
        .bind(w.audit_status)
        .bind(&w.create_time)
        .bind(w.wechat_live_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    pub async fn delete(&self, id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM wechat_live WHERE wechat_live_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 关联订单数（Java `fillStats`：`wechat_live_order_rel.room_id = ?`）。
    pub async fn count_orders(&self, room_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM wechat_live_order_rel WHERE room_id = $1")
                .bind(room_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 同步失败日志数（Java `fillStats`：`room_id = ? AND sync_status = 2`）。
    pub async fn count_sync_failed(&self, room_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM wechat_live_sync_log WHERE room_id = $1 AND sync_status = 2",
        )
        .bind(room_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 直播商品 / 直播间商品 —————————————————————

    /// 直播间商品关联列表（Java `wechatLiveRoomGoodsService.getGoodsByRoomId`）。
    pub async fn find_room_goods(&self, room_id: i32) -> Result<Vec<WechatLiveRoomGoods>> {
        sqlx::query_as::<_, WechatLiveRoomGoods>(
            "SELECT * FROM wechat_live_room_goods WHERE room_id = $1 ORDER BY sort ASC, id ASC",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 批量取直播商品。
    pub async fn find_goods_by_ids(&self, ids: &[i64]) -> Result<Vec<WechatLiveGoods>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!("SELECT * FROM wechat_live_goods WHERE id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, WechatLiveGoods>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 ID + 店铺取直播商品（Java `getGoodsByIdAndShopId`）。
    pub async fn find_goods_by_id_shop(
        &self,
        id: i64,
        shop_id: i32,
    ) -> Result<Option<WechatLiveGoods>> {
        sqlx::query_as::<_, WechatLiveGoods>(
            "SELECT * FROM wechat_live_goods WHERE id = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某直播间下某直播商品的关联记录（Java `getGoodsByRoomIdAndGoodsId`）。
    pub async fn find_room_goods_one(
        &self,
        room_id: i32,
        goods_id: i64,
    ) -> Result<Option<WechatLiveRoomGoods>> {
        sqlx::query_as::<_, WechatLiveRoomGoods>(
            "SELECT * FROM wechat_live_room_goods WHERE room_id = $1 AND wechat_live_goods_id = $2 LIMIT 1",
        )
        .bind(room_id)
        .bind(goods_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn insert_room_goods(&self, r: &WechatLiveRoomGoods) -> Result<i64> {
        let res: i64 = sqlx::query_scalar(
            "INSERT INTO wechat_live_room_goods (room_id, wechat_live_goods_id, wechat_goods_id,
                sort, is_show, create_time, update_time, shop_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(r.room_id)
        .bind(r.wechat_live_goods_id)
        .bind(r.wechat_goods_id)
        .bind(r.sort)
        .bind(r.is_show)
        .bind(&r.create_time)
        .bind(&r.update_time)
        .bind(r.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    pub async fn update_room_goods(&self, r: &WechatLiveRoomGoods) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE wechat_live_room_goods SET sort = $1, is_show = $2, update_time = $3 WHERE id = $4",
        )
        .bind(r.sort)
        .bind(r.is_show)
        .bind(&r.update_time)
        .bind(r.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    pub async fn delete_room_goods(&self, room_id: i32, goods_id: i64) -> Result<u64> {
        let r = sqlx::query(
            "DELETE FROM wechat_live_room_goods WHERE room_id = $1 AND wechat_live_goods_id = $2",
        )
        .bind(room_id)
        .bind(goods_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 商品（添加直播商品时的店铺/规格存在性校验）。
    pub async fn find_product(&self, product_id: i64) -> Result<Option<Product>> {
        sqlx::query_as::<_, Product>("SELECT * FROM product WHERE product_id = $1 LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// SKU（`product_sku_id > 0` 时的存在性/归属校验）。
    pub async fn find_sku(&self, sku_id: i64) -> Result<Option<ProductSku>> {
        sqlx::query_as::<_, ProductSku>("SELECT * FROM product_sku WHERE sku_id = $1 LIMIT 1")
            .bind(sku_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("直播不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

/// 追加一个 `LIKE ?` 条件并记录 `%value%` 绑定（避免闭包同时借用 wh/binds）。
fn push_like(wh: &mut String, binds: &mut Vec<String>, cond: &str, value: &str) {
    wh.push_str(cond);
    binds.push(format!("%{value}%"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginate_sort_whitelist_falls_back_to_primary_key() {
        // 纯逻辑：白名单外字段回退主键（与 Java getColumnByPropertyName 兜底一致）
        let pick = |f: Option<&str>| match f {
            Some("createTime") | Some("create_time") => "create_time",
            Some("liveStatus") | Some("live_status") => "live_status",
            _ => "wechat_live_id",
        };
        assert_eq!(pick(Some("createTime")), "create_time");
        assert_eq!(pick(Some("live_status")), "live_status");
        assert_eq!(pick(Some("1;DROP TABLE wechat_live")), "wechat_live_id");
        assert_eq!(pick(None), "wechat_live_id");
    }
}
