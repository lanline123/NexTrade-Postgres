//! 拍卖域仓储：`auction_session` / `auction_item` / `auction_bid` /
//! `auction_deposit` / `auction_breach` / `auction_ban` / `auction_remind`。
//!
//! Rust 版原创域（无 Java 对应），表结构见 `rust/migrations/004_auction.sql`。
//! 资金动作（冻结/解冻/抵扣/扣罚）在事务内直接操作 `user` 钱包列与
//! `user_balance_log`，与佣金提现冻结（`with_draw_apply_client`）同模式：
//! 先快照、再条件更新、最后写流水，流水格式与既有行完全一致。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{
    AuctionBan, AuctionBid, AuctionBreach, AuctionDeposit, AuctionItem, AuctionSession,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 钱包变动流水入参（字段与 `user_balance_log` 列一一对应）。
#[derive(Debug, Clone)]
pub struct WalletLogParams {
    pub user_id: i32,
    pub before_balance: Decimal,
    /// 本次余额变动额（正数记绝对值，方向由 change_type 表达）
    pub balance_delta: Decimal,
    pub after_balance: Decimal,
    /// 本次冻结额变动额（正数记绝对值）
    pub frozen_delta: Decimal,
    pub before_frozen_balance: Decimal,
    pub after_frozen_balance: Decimal,
    pub change_time: i32,
    pub change_desc: String,
    /// 1 增加 / 2 减少
    pub change_type: i32,
}

/// 管理端出价列表行（带拍品标题与用户信息）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminBidRow {
    pub bid_id: i32,
    pub item_id: i32,
    pub item_title: String,
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub bid_price: Decimal,
    pub is_buyout: i32,
    pub is_proxy: i32,
    pub status: i32,
    pub ip: String,
    pub user_agent: String,
    pub device_fp: String,
    pub add_time: i32,
}

/// 保证金列表行（带拍品标题与用户信息）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DepositRow {
    pub deposit_id: i32,
    pub item_id: i32,
    pub item_title: String,
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub amount: Decimal,
    pub status: i32,
    pub balance_log_id: Option<i32>,
    pub order_id: Option<i32>,
    pub penalty_amount: Decimal,
    pub add_time: i32,
    pub handle_time: i32,
    pub remark: String,
}

/// 违约列表行（带拍品标题与用户信息）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BreachRow {
    pub breach_id: i32,
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub item_id: i32,
    pub item_title: String,
    pub order_id: i32,
    pub deposit_amount: Decimal,
    pub penalty_amount: Decimal,
    pub breach_time: i32,
    pub waived: i32,
    pub waive_reason: String,
    pub operator_id: i32,
}

/// 封禁列表行（带用户信息）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BanRow {
    pub ban_id: i32,
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub ban_type: i32,
    pub start_time: i32,
    pub end_time: i32,
    pub reason: String,
    pub status: i32,
    pub operator_id: i32,
    pub add_time: i32,
}

#[derive(Clone)]
pub struct AuctionRepository {
    pool: PgPool,
}

impl AuctionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层在事务中复用）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 场次 —————————————————————

    pub async fn insert_session(&self, s: &AuctionSession) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO auction_session (session_name, cover_pic, start_time, end_time, sort, status, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING session_id",
        )
        .bind(&s.session_name)
        .bind(&s.cover_pic)
        .bind(s.start_time)
        .bind(s.end_time)
        .bind(s.sort)
        .bind(s.status)
        .bind(s.add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn update_session(&self, s: &AuctionSession) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE auction_session SET session_name = $1, cover_pic = $2, start_time = $3,
             end_time = $4, sort = $5, status = $6 WHERE session_id = $7",
        )
        .bind(&s.session_name)
        .bind(&s.cover_pic)
        .bind(s.start_time)
        .bind(s.end_time)
        .bind(s.sort)
        .bind(s.status)
        .bind(s.session_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("拍卖场次不存在".into()));
        }
        Ok(())
    }

    pub async fn delete_session(&self, session_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM auction_session WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_session(&self, session_id: i32) -> Result<Option<AuctionSession>> {
        sqlx::query_as::<_, AuctionSession>("SELECT * FROM auction_session WHERE session_id = $1 LIMIT 1")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 场次列表（带每场拍品数）。`only_enabled` 时只返回启用场次（前台用）。
    pub async fn list_sessions(&self, only_enabled: bool) -> Result<Vec<(AuctionSession, i64)>> {
        let rows = sqlx::query_as::<_, (i32, String, String, i32, i32, i32, i32, i32, i64)>(
            "SELECT s.session_id, s.session_name, s.cover_pic, s.start_time, s.end_time, s.sort, s.status, s.add_time,
                    (SELECT COUNT(*) FROM auction_item i WHERE i.session_id = s.session_id) AS item_count
             FROM auction_session s
             WHERE ($1 = FALSE OR s.status = 1)
             ORDER BY s.sort ASC, s.session_id DESC",
        )
        .bind(only_enabled)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(session_id, session_name, cover_pic, start_time, end_time, sort, status, add_time, cnt)| {
                (
                    AuctionSession { session_id, session_name, cover_pic, start_time, end_time, sort, status, add_time },
                    cnt,
                )
            })
            .collect())
    }

    // ————————————————————— 拍品 —————————————————————

    pub async fn insert_item(&self, it: &AuctionItem) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO auction_item
             (session_id, product_id, sku_id, shop_id, title, start_price, reserve_price, buyout_price,
              deposit_amount, start_time, end_time, real_end_time, extend_count, status, current_price,
              current_bid_id, bid_count, winner_user_id, final_price, order_id, proxy_max_price,
              create_admin_id, add_time, update_time, own_product)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
             RETURNING item_id",
        )
        .bind(it.session_id)
        .bind(it.product_id)
        .bind(it.sku_id)
        .bind(it.shop_id)
        .bind(&it.title)
        .bind(it.start_price)
        .bind(it.reserve_price)
        .bind(it.buyout_price)
        .bind(it.deposit_amount)
        .bind(it.start_time)
        .bind(it.end_time)
        .bind(it.real_end_time)
        .bind(it.extend_count)
        .bind(it.status)
        .bind(it.current_price)
        .bind(it.current_bid_id)
        .bind(it.bid_count)
        .bind(it.winner_user_id)
        .bind(it.final_price)
        .bind(it.order_id)
        .bind(it.proxy_max_price)
        .bind(it.create_admin_id)
        .bind(it.add_time)
        .bind(it.update_time)
        .bind(it.own_product)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn update_item(&self, it: &AuctionItem) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE auction_item SET session_id = $1, product_id = $2, sku_id = $3, shop_id = $4,
             title = $5, start_price = $6, reserve_price = $7, buyout_price = $8, deposit_amount = $9,
             start_time = $10, end_time = $11, real_end_time = $12, extend_count = $13, status = $14,
             current_price = $15, current_bid_id = $16, bid_count = $17, winner_user_id = $18,
             final_price = $19, order_id = $20, update_time = $21 WHERE item_id = $22",
        )
        .bind(it.session_id)
        .bind(it.product_id)
        .bind(it.sku_id)
        .bind(it.shop_id)
        .bind(&it.title)
        .bind(it.start_price)
        .bind(it.reserve_price)
        .bind(it.buyout_price)
        .bind(it.deposit_amount)
        .bind(it.start_time)
        .bind(it.end_time)
        .bind(it.real_end_time)
        .bind(it.extend_count)
        .bind(it.status)
        .bind(it.current_price)
        .bind(it.current_bid_id)
        .bind(it.bid_count)
        .bind(it.winner_user_id)
        .bind(it.final_price)
        .bind(it.order_id)
        .bind(it.update_time)
        .bind(it.item_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("拍品不存在".into()));
        }
        Ok(())
    }

    pub async fn delete_item(&self, item_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM auction_item WHERE item_id = $1")
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_item(&self, item_id: i32) -> Result<Option<AuctionItem>> {
        sqlx::query_as::<_, AuctionItem>("SELECT * FROM auction_item WHERE item_id = $1 LIMIT 1")
            .bind(item_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 事务内锁定拍品行（出价/结拍串行化）。
    pub async fn find_item_for_update(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i32,
    ) -> Result<Option<AuctionItem>> {
        sqlx::query_as::<_, AuctionItem>("SELECT * FROM auction_item WHERE item_id = $1 FOR UPDATE")
            .bind(item_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 事务内更新拍品（字段集同 `update_item`）。
    pub async fn update_item_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        it: &AuctionItem,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE auction_item SET session_id = $1, product_id = $2, sku_id = $3, shop_id = $4,
             title = $5, start_price = $6, reserve_price = $7, buyout_price = $8, deposit_amount = $9,
             start_time = $10, end_time = $11, real_end_time = $12, extend_count = $13, status = $14,
             current_price = $15, current_bid_id = $16, bid_count = $17, winner_user_id = $18,
             final_price = $19, order_id = $20, update_time = $21 WHERE item_id = $22",
        )
        .bind(it.session_id)
        .bind(it.product_id)
        .bind(it.sku_id)
        .bind(it.shop_id)
        .bind(&it.title)
        .bind(it.start_price)
        .bind(it.reserve_price)
        .bind(it.buyout_price)
        .bind(it.deposit_amount)
        .bind(it.start_time)
        .bind(it.end_time)
        .bind(it.real_end_time)
        .bind(it.extend_count)
        .bind(it.status)
        .bind(it.current_price)
        .bind(it.current_bid_id)
        .bind(it.bid_count)
        .bind(it.winner_user_id)
        .bind(it.final_price)
        .bind(it.order_id)
        .bind(it.update_time)
        .bind(it.item_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 管理端拍品分页。
    pub async fn paginate_items_admin(
        &self,
        keyword: Option<&str>,
        session_id: Option<i32>,
        status: Option<i32>,
        shop_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<AuctionItem>)> {
        let like = keyword.map(str::trim).filter(|k| !k.is_empty()).map(|k| format!("%{k}%"));
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_item
             WHERE ($1 IS NULL OR title ILIKE $2)
               AND ($3 IS NULL OR session_id = $4)
               AND ($5 IS NULL OR status = $6)
               AND ($7 IS NULL OR shop_id = $8)",
        )
        .bind(&like).bind(&like)
        .bind(session_id).bind(session_id)
        .bind(status).bind(status)
        .bind(shop_id).bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AuctionItem>(
            "SELECT * FROM auction_item
             WHERE ($1 IS NULL OR title ILIKE $2)
               AND ($3 IS NULL OR session_id = $4)
               AND ($5 IS NULL OR status = $6)
               AND ($7 IS NULL OR shop_id = $8)
             ORDER BY item_id DESC LIMIT $9 OFFSET $10",
        )
        .bind(&like).bind(&like)
        .bind(session_id).bind(session_id)
        .bind(status).bind(status)
        .bind(shop_id).bind(shop_id)
        .bind(limit).bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 前台拍品分页。`statuses` 为该页签的状态集合；`sort` 已由服务层归一。
    pub async fn paginate_items_public(
        &self,
        session_id: Option<i32>,
        statuses: &[i32],
        sort: &str,
        page: &PageQuery,
    ) -> Result<(u64, Vec<AuctionItem>)> {
        if statuses.is_empty() {
            return Ok((0, Vec::new()));
        }
        let placeholders = vec!["?"; statuses.len()].join(", ");
        let order = match sort {
            "newest" => "start_time DESC, item_id DESC",
            "priceAsc" => "COALESCE(current_price, start_price) ASC, item_id DESC",
            "priceDesc" => "COALESCE(current_price, start_price) DESC, item_id DESC",
            _ => "real_end_time ASC, item_id DESC", // ending：即将结束优先
        };
        let (limit, offset) = page.limit_offset();
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM auction_item WHERE status IN ({placeholders}) AND (? IS NULL OR session_id = ?) ORDER BY {order} LIMIT ? OFFSET ?"
        ));
        let total_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM auction_item WHERE status IN ({placeholders}) AND (? IS NULL OR session_id = ?)"
        ));
        let mut tq = sqlx::query_as::<_, (i64,)>(&total_sql);
        for s in statuses {
            tq = tq.bind(*s);
        }
        let total = tq.bind(session_id).bind(session_id).fetch_one(&self.pool).await.map_err(map_err)?.0;
        let mut q = sqlx::query_as::<_, AuctionItem>(&sql);
        for s in statuses {
            q = q.bind(*s);
        }
        let rows = q
            .bind(session_id)
            .bind(session_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.max(0) as u64, rows))
    }

    /// 排期冲突：同商品在时间窗内已有未结束拍品（草稿/预告/竞价中）。
    pub async fn exists_conflicting_item(
        &self,
        product_id: i32,
        start_time: i32,
        end_time: i32,
        exclude_item_id: Option<i32>,
    ) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_item
             WHERE product_id = $1 AND status IN (0, 1, 2)
               AND start_time < $3 AND end_time > $2
               AND ($4 IS NULL OR item_id <> $5)",
        )
        .bind(product_id)
        .bind(start_time)
        .bind(end_time)
        .bind(exclude_item_id)
        .bind(exclude_item_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 待结拍扫描：预告/竞价中且已到实际结束时间。
    pub async fn find_due_close(&self, now: i32, limit: u64) -> Result<Vec<AuctionItem>> {
        sqlx::query_as::<_, AuctionItem>(
            "SELECT * FROM auction_item WHERE status IN (1, 2) AND real_end_time > 0 AND real_end_time <= $1
             ORDER BY real_end_time ASC LIMIT $2",
        )
        .bind(now)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 待开拍扫描：预告中且已到开始时间（推进竞价中 + 发开拍提醒）。
    pub async fn find_due_start(&self, now: i32, limit: u64) -> Result<Vec<AuctionItem>> {
        sqlx::query_as::<_, AuctionItem>(
            "SELECT * FROM auction_item WHERE status = 1 AND start_time > 0 AND start_time <= $1
             ORDER BY start_time ASC LIMIT $2",
        )
        .bind(now)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 出价 —————————————————————

    pub async fn insert_bid_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        bid: &AuctionBid,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO auction_bid (item_id, user_id, bid_price, is_buyout, is_proxy, status, ip, user_agent, device_fp, add_time)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING bid_id",
        )
        .bind(bid.item_id)
        .bind(bid.user_id)
        .bind(bid.bid_price)
        .bind(bid.is_buyout)
        .bind(bid.is_proxy)
        .bind(bid.status)
        .bind(&bid.ip)
        .bind(&bid.user_agent)
        .bind(&bid.device_fp)
        .bind(bid.add_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 当前最高有效出价（事务内，供结拍/作废后重算）。
    pub async fn find_top_valid_bid_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i32,
    ) -> Result<Option<AuctionBid>> {
        sqlx::query_as::<_, AuctionBid>(
            "SELECT * FROM auction_bid WHERE item_id = $1 AND status = 1
             ORDER BY bid_price DESC, bid_id ASC LIMIT 1",
        )
        .bind(item_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 出价墙分页（有效出价，价高在前）。
    pub async fn list_bids(&self, item_id: i32, page: &PageQuery) -> Result<(u64, Vec<AuctionBid>)> {
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_bid WHERE item_id = $1 AND status = 1",
        )
        .bind(item_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AuctionBid>(
            "SELECT * FROM auction_bid WHERE item_id = $1 AND status = 1
             ORDER BY bid_price DESC, bid_id ASC LIMIT $2 OFFSET $3",
        )
        .bind(item_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    pub async fn find_bid(&self, bid_id: i32) -> Result<Option<AuctionBid>> {
        sqlx::query_as::<_, AuctionBid>("SELECT * FROM auction_bid WHERE bid_id = $1 LIMIT 1")
            .bind(bid_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 事务内统计某拍品有效出价数（作废/结拍重算用）。
    pub async fn count_valid_bids_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_bid WHERE item_id = $1 AND status = 1",
        )
        .bind(item_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 事务内作废一笔出价。
    pub async fn void_bid_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        bid_id: i32,
    ) -> Result<bool> {
        let affected = sqlx::query("UPDATE auction_bid SET status = 2 WHERE bid_id = $1 AND status = 1")
            .bind(bid_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 事务内作废某拍品全部有效出价（违约后拍品回待排期时清场）。
    pub async fn void_all_bids_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i32,
    ) -> Result<u64> {
        let affected = sqlx::query("UPDATE auction_bid SET status = 2 WHERE item_id = $1 AND status = 1")
            .bind(item_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 用户在某拍品的最高有效出价。
    pub async fn find_user_max_bid(&self, item_id: i32, user_id: i32) -> Result<Option<Decimal>> {
        let row: Option<(Option<Decimal>,)> = sqlx::query_as(
            "SELECT MAX(bid_price) FROM auction_bid WHERE item_id = $1 AND user_id = $2 AND status = 1",
        )
        .bind(item_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 用户参与过的拍品 id 集合（「我的参拍」用，含已作废出价的历史参与）。
    pub async fn find_user_joined_item_ids(&self, user_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT item_id FROM auction_bid WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 管理端出价分页（可按拍品/用户过滤，含作废）。
    pub async fn paginate_bids_admin(
        &self,
        item_id: Option<i32>,
        user_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<AdminBidRow>)> {
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_bid b
             WHERE ($1 IS NULL OR b.item_id = $2) AND ($3 IS NULL OR b.user_id = $4)",
        )
        .bind(item_id).bind(item_id)
        .bind(user_id).bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AdminBidRow>(
            "SELECT b.bid_id, b.item_id, COALESCE(i.title, '') AS item_title, b.user_id,
                    COALESCE(u.username, '') AS username, u.nickname, b.bid_price, b.is_buyout,
                    b.is_proxy, b.status, b.ip, b.user_agent, b.device_fp, b.add_time
             FROM auction_bid b
             LEFT JOIN auction_item i ON i.item_id = b.item_id
             LEFT JOIN \"user\" u ON u.user_id = b.user_id
             WHERE ($1 IS NULL OR b.item_id = $2) AND ($3 IS NULL OR b.user_id = $4)
             ORDER BY b.bid_id DESC LIMIT $5 OFFSET $6",
        )
        .bind(item_id).bind(item_id)
        .bind(user_id).bind(user_id)
        .bind(limit).bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 保证金 —————————————————————

    pub async fn find_deposit(&self, item_id: i32, user_id: i32) -> Result<Option<AuctionDeposit>> {
        sqlx::query_as::<_, AuctionDeposit>(
            "SELECT * FROM auction_deposit WHERE item_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(item_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn find_deposit_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i32,
        user_id: i32,
    ) -> Result<Option<AuctionDeposit>> {
        sqlx::query_as::<_, AuctionDeposit>(
            "SELECT * FROM auction_deposit WHERE item_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(item_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    pub async fn insert_deposit_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        d: &AuctionDeposit,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO auction_deposit (item_id, user_id, amount, status, balance_log_id, order_id, penalty_amount, add_time, handle_time, remark)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING deposit_id",
        )
        .bind(d.item_id)
        .bind(d.user_id)
        .bind(d.amount)
        .bind(d.status)
        .bind(d.balance_log_id)
        .bind(d.order_id)
        .bind(d.penalty_amount)
        .bind(d.add_time)
        .bind(d.handle_time)
        .bind(&d.remark)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn update_deposit_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        d: &AuctionDeposit,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE auction_deposit SET status = $1, balance_log_id = $2, order_id = $3,
             penalty_amount = $4, handle_time = $5, remark = $6 WHERE deposit_id = $7",
        )
        .bind(d.status)
        .bind(d.balance_log_id)
        .bind(d.order_id)
        .bind(d.penalty_amount)
        .bind(d.handle_time)
        .bind(&d.remark)
        .bind(d.deposit_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 某拍品全部冻结中的保证金（结拍批量解冻用，事务内）。
    pub async fn find_frozen_deposits_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item_id: i32,
    ) -> Result<Vec<AuctionDeposit>> {
        sqlx::query_as::<_, AuctionDeposit>(
            "SELECT * FROM auction_deposit WHERE item_id = $1 AND status = 1 ORDER BY deposit_id ASC",
        )
        .bind(item_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_err)
    }

    /// 按订单找保证金（违约处理用：中标人那条已抵扣记录）。
    pub async fn find_deposit_by_order_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        order_id: i32,
    ) -> Result<Option<AuctionDeposit>> {
        sqlx::query_as::<_, AuctionDeposit>(
            "SELECT * FROM auction_deposit WHERE order_id = $1 LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    pub async fn find_deposit_by_id(&self, deposit_id: i32) -> Result<Option<AuctionDeposit>> {
        sqlx::query_as::<_, AuctionDeposit>("SELECT * FROM auction_deposit WHERE deposit_id = $1 LIMIT 1")
            .bind(deposit_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 保证金分页（管理端与「我的」共用，`user_id` 传值即限定本人）。
    pub async fn paginate_deposits(
        &self,
        item_id: Option<i32>,
        user_id: Option<i32>,
        status: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<DepositRow>)> {
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_deposit d
             WHERE ($1 IS NULL OR d.item_id = $2) AND ($3 IS NULL OR d.user_id = $4) AND ($5 IS NULL OR d.status = $6)",
        )
        .bind(item_id).bind(item_id)
        .bind(user_id).bind(user_id)
        .bind(status).bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, DepositRow>(
            "SELECT d.deposit_id, d.item_id, COALESCE(i.title, '') AS item_title, d.user_id,
                    COALESCE(u.username, '') AS username, u.nickname, d.amount, d.status,
                    d.balance_log_id, d.order_id, d.penalty_amount, d.add_time, d.handle_time, d.remark
             FROM auction_deposit d
             LEFT JOIN auction_item i ON i.item_id = d.item_id
             LEFT JOIN \"user\" u ON u.user_id = d.user_id
             WHERE ($1 IS NULL OR d.item_id = $2) AND ($3 IS NULL OR d.user_id = $4) AND ($5 IS NULL OR d.status = $6)
             ORDER BY d.deposit_id DESC LIMIT $7 OFFSET $8",
        )
        .bind(item_id).bind(item_id)
        .bind(user_id).bind(user_id)
        .bind(status).bind(status)
        .bind(limit).bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    // ————————————————————— 钱包（事务内） —————————————————————

    /// 事务内锁定用户钱包行，返回 `(balance, frozen_balance)`。
    pub async fn wallet_snapshot_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: i32,
    ) -> Result<Option<(Decimal, Decimal)>> {
        let row: Option<(Decimal, Decimal)> = sqlx::query_as(
            "SELECT balance, frozen_balance FROM \"user\" WHERE user_id = $1 FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 事务内钱包等额转移/出账（条件更新：变动后余额与冻结额均不得为负）。
    /// 返回是否成功（false = 余额/冻结额不足）。
    pub async fn apply_wallet_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: i32,
        balance_delta: Decimal,
        frozen_delta: Decimal,
    ) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE \"user\" SET balance = balance + $1, frozen_balance = frozen_balance + $2
             WHERE user_id = $3 AND balance + $1 >= 0 AND frozen_balance + $2 >= 0",
        )
        .bind(balance_delta)
        .bind(frozen_delta)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 事务内写一笔余额流水（格式与既有 `user_balance_log` 行一致），返回 log_id。
    pub async fn insert_balance_log_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        p: &WalletLogParams,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_balance_log
             (user_id, before_balance, balance, after_balance, frozen_balance, before_frozen_balance,
              after_frozen_balance, change_time, change_desc, change_type, wallet_type, new_balance, new_frozen_balance)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,0,$11,$12) RETURNING log_id",
        )
        .bind(p.user_id)
        .bind(p.before_balance)
        .bind(p.balance_delta)
        .bind(p.after_balance)
        .bind(p.frozen_delta)
        .bind(p.before_frozen_balance)
        .bind(p.after_frozen_balance)
        .bind(p.change_time)
        .bind(&p.change_desc)
        .bind(p.change_type)
        .bind(p.after_balance)
        .bind(p.after_frozen_balance)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    // ————————————————————— 违约与封禁 —————————————————————

    pub async fn insert_breach_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        b: &AuctionBreach,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO auction_breach (user_id, item_id, order_id, deposit_amount, penalty_amount, breach_time, waived, waive_reason, operator_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING breach_id",
        )
        .bind(b.user_id)
        .bind(b.item_id)
        .bind(b.order_id)
        .bind(b.deposit_amount)
        .bind(b.penalty_amount)
        .bind(b.breach_time)
        .bind(b.waived)
        .bind(&b.waive_reason)
        .bind(b.operator_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 用户自 `since` 起未豁免的违约次数（阶梯封禁判定）。
    pub async fn count_breaches_since(&self, user_id: i32, since: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_breach WHERE user_id = $1 AND waived = 0 AND breach_time >= $2",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 事务内统计用户自 `since` 起未豁免违约次数（与刚插入的违约同事务口径）。
    pub async fn count_breaches_since_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: i32,
        since: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_breach WHERE user_id = $1 AND waived = 0 AND breach_time >= $2",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_breach(&self, breach_id: i32) -> Result<Option<AuctionBreach>> {
        sqlx::query_as::<_, AuctionBreach>("SELECT * FROM auction_breach WHERE breach_id = $1 LIMIT 1")
            .bind(breach_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn waive_breach(&self, breach_id: i32, reason: &str, operator_id: i32) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE auction_breach SET waived = 1, waive_reason = $1, operator_id = $2 WHERE breach_id = $3 AND waived = 0",
        )
        .bind(reason)
        .bind(operator_id)
        .bind(breach_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("违约记录不存在或已改判".into()));
        }
        Ok(())
    }

    pub async fn paginate_breaches(
        &self,
        user_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<BreachRow>)> {
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_breach b WHERE ($1 IS NULL OR b.user_id = $2)",
        )
        .bind(user_id).bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, BreachRow>(
            "SELECT b.breach_id, b.user_id, COALESCE(u.username, '') AS username, u.nickname,
                    b.item_id, COALESCE(i.title, '') AS item_title, b.order_id, b.deposit_amount,
                    b.penalty_amount, b.breach_time, b.waived, b.waive_reason, b.operator_id
             FROM auction_breach b
             LEFT JOIN auction_item i ON i.item_id = b.item_id
             LEFT JOIN \"user\" u ON u.user_id = b.user_id
             WHERE ($1 IS NULL OR b.user_id = $2)
             ORDER BY b.breach_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(user_id).bind(user_id)
        .bind(limit).bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    pub async fn insert_ban(&self, b: &AuctionBan) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO auction_ban (user_id, ban_type, start_time, end_time, reason, status, operator_id, add_time)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING ban_id",
        )
        .bind(b.user_id)
        .bind(b.ban_type)
        .bind(b.start_time)
        .bind(b.end_time)
        .bind(&b.reason)
        .bind(b.status)
        .bind(b.operator_id)
        .bind(b.add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 用户当前生效的封禁（临时封禁须未到期）。
    pub async fn find_active_ban(&self, user_id: i32, now: i32) -> Result<Option<AuctionBan>> {
        sqlx::query_as::<_, AuctionBan>(
            "SELECT * FROM auction_ban WHERE user_id = $1 AND status = 1
               AND (ban_type = 2 OR end_time > $2)
             ORDER BY ban_id DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn paginate_bans(
        &self,
        user_id: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<BanRow>)> {
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_ban b WHERE ($1 IS NULL OR b.user_id = $2)",
        )
        .bind(user_id).bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, BanRow>(
            "SELECT b.ban_id, b.user_id, COALESCE(u.username, '') AS username, u.nickname,
                    b.ban_type, b.start_time, b.end_time, b.reason, b.status, b.operator_id, b.add_time
             FROM auction_ban b
             LEFT JOIN \"user\" u ON u.user_id = b.user_id
             WHERE ($1 IS NULL OR b.user_id = $2)
             ORDER BY b.ban_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(user_id).bind(user_id)
        .bind(limit).bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    pub async fn unban(&self, ban_id: i32) -> Result<()> {
        let affected = sqlx::query("UPDATE auction_ban SET status = 0 WHERE ban_id = $1 AND status = 1")
            .bind(ban_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("封禁记录不存在或已解封".into()));
        }
        Ok(())
    }

    // ————————————————————— 提醒订阅 —————————————————————

    pub async fn upsert_remind(&self, item_id: i32, user_id: i32, now: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO auction_remind (item_id, user_id, notified, add_time) VALUES ($1, $2, 0, $3)
             ON CONFLICT (item_id, user_id) DO NOTHING",
        )
        .bind(item_id)
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn delete_remind(&self, item_id: i32, user_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM auction_remind WHERE item_id = $1 AND user_id = $2")
            .bind(item_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn has_remind(&self, item_id: i32, user_id: i32) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auction_remind WHERE item_id = $1 AND user_id = $2",
        )
        .bind(item_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }

    /// 某拍品未通知的订阅用户（开拍推进时发送提醒）。
    pub async fn find_unnotified_remind_users(&self, item_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT user_id FROM auction_remind WHERE item_id = $1 AND notified = 0",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn mark_reminds_notified(&self, item_id: i32) -> Result<()> {
        sqlx::query("UPDATE auction_remind SET notified = 1 WHERE item_id = $1 AND notified = 0")
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 辅助 —————————————————————

    /// 批量取用户展示名：`(user_id, username, nickname)`。
    pub async fn find_user_displays(&self, user_ids: &[i32]) -> Result<Vec<(i32, String, Option<String>)>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_id, username, nickname FROM \"user\" WHERE user_id IN ({placeholders})"
        ));
        let mut q = sqlx::query_as::<_, (i32, String, Option<String>)>(&sql);
        for id in user_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 统计原始数（服务层组装 VO）。
    pub async fn stats_raw(&self) -> Result<(i64, i64, i64, i64, i64, Decimal, Decimal, Option<Decimal>)> {
        let row: (i64, i64, i64, i64, i64, Option<Decimal>, Option<Decimal>, Option<Decimal>) = sqlx::query_as(
            "SELECT
               (SELECT COUNT(*) FROM auction_item),
               (SELECT COUNT(*) FROM auction_item WHERE status = 2),
               (SELECT COUNT(*) FROM auction_item WHERE status = 3),
               (SELECT COUNT(*) FROM auction_item WHERE status = 4),
               (SELECT COUNT(*) FROM auction_bid WHERE status = 1),
               (SELECT COALESCE(SUM(amount), 0) FROM auction_deposit WHERE status = 1),
               (SELECT COALESCE(SUM(penalty_amount), 0) FROM auction_breach WHERE waived = 0),
               (SELECT AVG(final_price / NULLIF(start_price, 0)) FROM auction_item WHERE status = 3 AND final_price IS NOT NULL)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((
            row.0,
            row.1,
            row.2,
            row.3,
            row.4,
            row.5.unwrap_or_default(),
            row.6.unwrap_or_default(),
            row.7,
        ))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("拍卖记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}
