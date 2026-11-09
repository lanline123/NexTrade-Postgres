//! POS 收银仓储（pos::cashier）。
//!
//! 表：`admin_user_pos` / `pos_pending_order`（+002 扩展列 kind/table_id）/
//! `pos_pending_order_item`（002 新表）/ `pos_table_area` / `pos_table`；
//! 轻量跨表读（product / user / user_address / order）直接走本仓储。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::o2o::{
    AdminUserPos, PosPendingOrder, PosPendingOrderItem, PosTable, PosTableArea,
};
use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

fn now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32
}

#[derive(Clone)]
pub struct PosCashierRepository {
    pool: PgPool,
}

impl PosCashierRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 收银员 —————————————————————

    pub async fn cashier_paginate(
        &self,
        keyword: Option<&str>,
        status: Option<i32>,
        limit: u64,
        offset: u64,
    ) -> Result<(i64, Vec<AdminUserPos>)> {
        let kw = format!("%{}%", keyword.unwrap_or_default().trim());
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_user_pos WHERE ($1 IS NULL OR is_using = $2) \
             AND username LIKE $3",
        )
        .bind(status)
        .bind(status)
        .bind(&kw)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AdminUserPos>(
            "SELECT id, admin_id, shop_id, username, email, avatar, auth_list, is_using, \
             add_time, role_id FROM admin_user_pos \
             WHERE ($1 IS NULL OR is_using = $2) AND username LIKE $3 \
             ORDER BY id DESC LIMIT $4 OFFSET $5",
        )
        .bind(status)
        .bind(status)
        .bind(&kw)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.max(0), rows))
    }

    pub async fn cashier_find(&self, id: i32) -> Result<Option<AdminUserPos>> {
        sqlx::query_as::<_, AdminUserPos>(
            "SELECT id, admin_id, shop_id, username, email, avatar, auth_list, is_using, \
             add_time, role_id FROM admin_user_pos WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn cashier_insert(&self, e: &AdminUserPos) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_user_pos (admin_id, shop_id, username, email, is_using, \
             role_id, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(e.admin_id)
        .bind(e.shop_id)
        .bind(&e.username)
        .bind(&e.email)
        .bind(e.is_using)
        .bind(e.role_id)
        .bind(now())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn cashier_update_field(&self, id: i32, column: &str, val: &str) -> Result<()> {
        let allowed = ["username", "email", "is_using", "role_id"];
        if !allowed.contains(&column) {
            return Err(Error::Validation("非法字段".into()));
        }
        sqlx::query(&format!(
            "UPDATE admin_user_pos SET {column} = $1 WHERE id = $2"
        ))
        .bind(val)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn cashier_delete(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM admin_user_pos WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 上下班 / 统计 —————————————————————

    /// 上下班时间戳存 config（code = posWork:{admin_id}，value = ISO 时间 JSON）。
    pub async fn work_flag(&self, admin_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(format!("posWork:{admin_id}"))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    pub async fn set_work_flag(&self, admin_id: i32, working: bool) -> Result<()> {
        let code = format!("posWork:{admin_id}");
        let value = serde_json::json!({ "working": working, "time": now() }).to_string();
        sqlx::query("DELETE FROM config WHERE biz_code = $1")
            .bind(&code)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        sqlx::query("INSERT INTO config (biz_code, biz_val) VALUES ($1, $2)")
            .bind(&code)
            .bind(&value)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 当班统计：pos_pending_order status=1（已取单=已成交）按 admin 聚合。
    pub async fn shift_statistics(
        &self,
        admin_id: i32,
        day_start: i32,
    ) -> Result<(i64, Decimal)> {
        let row: (i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(total), 0) FROM (\
               SELECT p.pending_id, COALESCE(SUM(i.price * i.quantity), 0) AS total \
               FROM pos_pending_order p \
               LEFT JOIN pos_pending_order_item i ON i.pending_id = p.pending_id \
               WHERE p.admin_id = $1 AND p.status = 1 AND p.update_time >= $2 \
               GROUP BY p.pending_id\
             ) t",
        )
        .bind(admin_id)
        .bind(day_start)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((row.0, row.1.unwrap_or_default()))
    }

    // ————————————————————— 购物车 / 挂单 —————————————————————

    /// 当前工作草稿（kind=0, status=0, 最新一条）。
    pub async fn working_draft(&self, admin_id: i32) -> Result<Option<PosPendingOrder>> {
        sqlx::query_as::<_, PosPendingOrder>(
            "SELECT pending_id, user_id, admin_id, shop_id, remark, status, add_time, \
             update_time, kind, table_id FROM pos_pending_order \
             WHERE admin_id = $1 AND kind = 0 AND status = 0 ORDER BY pending_id DESC LIMIT 1",
        )
        .bind(admin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn ensure_draft(
        &self,
        admin_id: i32,
        user_id: i32,
        shop_id: i32,
        table_id: i32,
    ) -> Result<PosPendingOrder> {
        if let Some(d) = self.working_draft(admin_id).await? {
            return Ok(d);
        }
        sqlx::query(
            "INSERT INTO pos_pending_order (user_id, admin_id, shop_id, status, kind, \
             table_id, add_time, update_time) VALUES ($1, $2, $3, 0, 0, $4, $5, $6)",
        )
        .bind(user_id)
        .bind(admin_id)
        .bind(shop_id)
        .bind(table_id)
        .bind(now())
        .bind(now())
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        self.working_draft(admin_id)
            .await?
            .ok_or_else(|| Error::Internal("挂单草稿创建失败".into()))
    }

    pub async fn items(&self, pending_id: i32) -> Result<Vec<PosPendingOrderItem>> {
        sqlx::query_as::<_, PosPendingOrderItem>(
            "SELECT item_id, pending_id, product_id, sku_id, product_name, pic_thumb, price, \
             origin_price, quantity, is_checked, sku_data, add_time \
             FROM pos_pending_order_item WHERE pending_id = $1 ORDER BY item_id",
        )
        .bind(pending_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn item_find(&self, pending_id: i32, item_id: i64) -> Result<Option<PosPendingOrderItem>> {
        sqlx::query_as::<_, PosPendingOrderItem>(
            "SELECT item_id, pending_id, product_id, sku_id, product_name, pic_thumb, price, \
             origin_price, quantity, is_checked, sku_data, add_time \
             FROM pos_pending_order_item WHERE pending_id = $1 AND item_id = $2",
        )
        .bind(pending_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn item_insert(
        &self,
        pending_id: i32,
        product_id: i64,
        sku_id: i64,
        product_name: &str,
        pic_thumb: &str,
        price: Decimal,
        origin_price: Decimal,
        quantity: i32,
        sku_data: Option<&str>,
    ) -> Result<i64> {
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO pos_pending_order_item (pending_id, product_id, sku_id, product_name, \
             pic_thumb, price, origin_price, quantity, is_checked, sku_data, add_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, $9, $10) RETURNING item_id",
        )
        .bind(pending_id)
        .bind(product_id)
        .bind(sku_id)
        .bind(product_name)
        .bind(pic_thumb)
        .bind(price)
        .bind(origin_price)
        .bind(quantity)
        .bind(sku_data)
        .bind(now())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn item_merge_same(
        &self,
        pending_id: i32,
        product_id: i64,
        sku_id: i64,
        quantity: i32,
    ) -> Result<Option<i64>> {
        let existing: Option<(i64, i32)> = sqlx::query_as(
            "SELECT item_id, quantity FROM pos_pending_order_item \
             WHERE pending_id = $1 AND product_id = $2 AND sku_id = $3 LIMIT 1",
        )
        .bind(pending_id)
        .bind(product_id)
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        if let Some((item_id, qty)) = existing {
            sqlx::query(
                "UPDATE pos_pending_order_item SET quantity = quantity + $1 WHERE item_id = $2",
            )
            .bind(quantity)
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            return Ok(Some(item_id));
        }
        Ok(None)
    }

    pub async fn item_update_quantity(&self, item_id: i64, quantity: i32) -> Result<()> {
        sqlx::query("UPDATE pos_pending_order_item SET quantity = $1 WHERE item_id = $2")
            .bind(quantity)
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn item_update_checked(&self, item_id: i64, checked: bool) -> Result<()> {
        sqlx::query("UPDATE pos_pending_order_item SET is_checked = $1 WHERE item_id = $2")
            .bind(i32::from(checked))
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn item_update_price(&self, item_id: i64, price: Decimal) -> Result<()> {
        sqlx::query("UPDATE pos_pending_order_item SET price = $1 WHERE item_id = $2")
            .bind(price)
            .bind(item_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn item_delete(&self, pending_id: i32, item_ids: &[i64]) -> Result<()> {
        if item_ids.is_empty() {
            return Ok(());
        }
        let ids = item_ids
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        sqlx::query(&format!(
            "DELETE FROM pos_pending_order_item WHERE pending_id = $1 AND item_id IN ({ids})"
        ))
        .bind(pending_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn item_clear(&self, pending_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM pos_pending_order_item WHERE pending_id = $1")
            .bind(pending_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn draft_touch(&self, pending_id: i32, user_id: i32, table_id: i32) -> Result<()> {
        sqlx::query(
            "UPDATE pos_pending_order SET update_time = $1, user_id = $2, table_id = $3 \
             WHERE pending_id = $4",
        )
        .bind(now())
        .bind(user_id)
        .bind(table_id)
        .bind(pending_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 挂单：把当前草稿转为已保存挂单（kind=1, status=0）。
    pub async fn save_pending(&self, pending_id: i32, remark: &str) -> Result<()> {
        sqlx::query(
            "UPDATE pos_pending_order SET kind = 1, remark = $1, update_time = $2 \
             WHERE pending_id = $3",
        )
        .bind(remark)
        .bind(now())
        .bind(pending_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 已保存挂单列表。
    pub async fn pending_list(
        &self,
        admin_id: i32,
        shop_id: Option<i32>,
    ) -> Result<Vec<PosPendingOrder>> {
        sqlx::query_as::<_, PosPendingOrder>(
            "SELECT pending_id, user_id, admin_id, shop_id, remark, status, add_time, \
             update_time, kind, table_id FROM pos_pending_order \
             WHERE kind = 1 AND status = 0 AND ($1 IS NULL OR admin_id = $2) \
             AND ($3 IS NULL OR shop_id = $4) ORDER BY pending_id DESC LIMIT 50",
        )
        .bind(shop_id)
        .bind(shop_id.map(|_| admin_id))
        .bind(shop_id)
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取单：把已保存挂单的明细并入当前草稿，并置 status=1（已取单）。
    pub async fn take_pending(&self, admin_id: i32, pending_id: i32) -> Result<()> {
        let src = sqlx::query_as::<_, PosPendingOrder>(
            "SELECT pending_id, user_id, admin_id, shop_id, remark, status, add_time, \
             update_time, kind, table_id FROM pos_pending_order WHERE pending_id = $1",
        )
        .bind(pending_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?
        .ok_or_else(|| Error::NotFound("挂单不存在".into()))?;
        let draft = self
            .ensure_draft(admin_id, src.user_id, src.shop_id.unwrap_or(0), src.table_id)
            .await?;
        sqlx::query(
            "INSERT INTO pos_pending_order_item (pending_id, product_id, sku_id, product_name, \
             pic_thumb, price, origin_price, quantity, is_checked, sku_data, add_time) \
             SELECT $1, product_id, sku_id, product_name, pic_thumb, price, origin_price, \
             quantity, is_checked, sku_data, $2 FROM pos_pending_order_item WHERE pending_id = $3",
        )
        .bind(draft.pending_id)
        .bind(now())
        .bind(pending_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        sqlx::query("UPDATE pos_pending_order SET status = 1, update_time = $1 WHERE pending_id = $2")
            .bind(now())
            .bind(pending_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn cancel_pending(&self, pending_id: i32) -> Result<()> {
        sqlx::query(
            "UPDATE pos_pending_order SET status = 2, update_time = $1 WHERE pending_id = $2",
        )
        .bind(now())
        .bind(pending_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 桌台 —————————————————————

    pub async fn areas(&self, shop_id: i32) -> Result<Vec<PosTableArea>> {
        sqlx::query_as::<_, PosTableArea>(
            "SELECT * FROM pos_table_area \
             WHERE shop_id = $1 AND is_delete = 0 ORDER BY sort_order, area_id",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn area_find(&self, id: i32) -> Result<Option<PosTableArea>> {
        sqlx::query_as::<_, PosTableArea>(
            "SELECT * FROM pos_table_area WHERE area_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn area_insert(&self, shop_id: i32, name: &str, sort: i32) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO pos_table_area (shop_id, area_name, sort_order, status, add_time, update_time) VALUES ($1, $2, $3, 1, $4, $5) RETURNING area_id",
        )
        .bind(shop_id)
        .bind(name)
        .bind(sort)
        .bind(now())
        .bind(now())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn area_update(&self, id: i32, name: &str, sort: i32) -> Result<()> {
        sqlx::query("UPDATE pos_table_area SET area_name = $1, sort_order = $2, update_time = $3 WHERE area_id = $4")
            .bind(name)
            .bind(sort)
            .bind(sort)
            .bind(now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn area_delete(&self, id: i32) -> Result<()> {
        sqlx::query("UPDATE pos_table_area SET is_delete = 1, update_time = $1 WHERE area_id = $2")
            .bind(now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn tables(
        &self,
        shop_id: i32,
        area_id: Option<i32>,
        status: Option<i32>,
    ) -> Result<Vec<PosTable>> {
        sqlx::query_as::<_, PosTable>(
            "SELECT CAST(table_id AS BIGINT) AS table_id, CAST(shop_id AS BIGINT) AS shop_id, CAST(area_id AS BIGINT) AS area_id, table_no, CAST(seat_count AS BIGINT) AS seat_count, CAST(sort_order AS BIGINT) AS sort_order, CAST(status AS BIGINT) AS status, CAST(table_status AS BIGINT) AS table_status, CAST(dining_count AS BIGINT) AS dining_count, CAST(user_id AS BIGINT) AS user_id, CAST(open_admin_id AS BIGINT) AS open_admin_id, CAST(open_time AS BIGINT) AS open_time, session_sn, CAST(last_order_id AS BIGINT) AS last_order_id, transfer_from_table_no, CAST(is_delete AS BIGINT) AS is_delete, CAST(add_time AS BIGINT) AS add_time, CAST(update_time AS BIGINT) AS update_time FROM pos_table \
             WHERE shop_id = $1 AND is_delete = 0 AND ($2 IS NULL OR area_id = $3) AND ($4 IS NULL OR table_status = $5) \
             ORDER BY sort_order, table_id DESC",
        )
        .bind(shop_id)
        .bind(area_id)
        .bind(area_id)
        .bind(status)
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn table_find(&self, id: i32) -> Result<Option<PosTable>> {
        sqlx::query_as::<_, PosTable>(
            "SELECT CAST(table_id AS BIGINT) AS table_id, CAST(shop_id AS BIGINT) AS shop_id, CAST(area_id AS BIGINT) AS area_id, table_no, CAST(seat_count AS BIGINT) AS seat_count, CAST(sort_order AS BIGINT) AS sort_order, CAST(status AS BIGINT) AS status, CAST(table_status AS BIGINT) AS table_status, CAST(dining_count AS BIGINT) AS dining_count, CAST(user_id AS BIGINT) AS user_id, CAST(open_admin_id AS BIGINT) AS open_admin_id, CAST(open_time AS BIGINT) AS open_time, session_sn, CAST(last_order_id AS BIGINT) AS last_order_id, transfer_from_table_no, CAST(is_delete AS BIGINT) AS is_delete, CAST(add_time AS BIGINT) AS add_time, CAST(update_time AS BIGINT) AS update_time FROM pos_table WHERE table_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn table_insert(
        &self,
        shop_id: i32,
        area_id: i32,
        name: &str,
        capacity: i32,
        sort_order: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO pos_table (shop_id, area_id, table_no, seat_count, sort_order, status, table_status, dining_count, open_admin_id, open_time, session_sn, last_order_id, transfer_from_table_no, is_delete, add_time) \
             VALUES ($1, $2, $3, $4, $5, 0, 0, 0, 0, 0, '', 0, '', 0, $6) RETURNING table_id",
        )
        .bind(shop_id)
        .bind(area_id)
        .bind(name)
        .bind(capacity)
        .bind(sort_order)
        .bind(now())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn table_update(
        &self,
        id: i32,
        name: &str,
        area_id: i32,
        capacity: i32,
        sort_order: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE pos_table SET table_no = $1, area_id = $2, seat_count = $3, sort_order = $4, update_time = $5 \
             WHERE table_id = $6",
        )
        .bind(name)
        .bind(area_id)
        .bind(capacity)
        .bind(sort_order)
        .bind(now())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn table_update_field(&self, id: i32, column: &str, val: &str) -> Result<()> {
        let allowed = ["table_no", "seat_count", "sort_order", "table_status", "dining_count", "transfer_from_table_no"];
        if !allowed.contains(&column) {
            return Err(Error::Validation("非法字段".into()));
        }
        sqlx::query(&format!("UPDATE pos_table SET {column} = $1, update_time = $2 WHERE table_id = $3"))
            .bind(val)
            .bind(now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn table_delete(&self, id: i32) -> Result<()> {
        sqlx::query("UPDATE pos_table SET is_delete = 1, update_time = $1 WHERE table_id = $2")
            .bind(now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn table_set_occupied(
        &self,
        id: i32,
        occupied: bool,
        user_id: i32,
        order_id: i32,
    ) -> Result<()> {
        let ts = now();
        let session = if occupied {
            format!("T{ts}{id}")
        } else {
            String::new()
        };
        sqlx::query(
            "UPDATE pos_table SET status = $1, table_status = $2, user_id = $3, last_order_id = $4, \
             open_time = $5, session_sn = $6, update_time = $7 WHERE table_id = $8",
        )
        .bind(i32::from(occupied))
        .bind(if occupied { 1 } else { 0 })
        .bind(user_id)
        .bind(order_id)
        .bind(if occupied { ts } else { 0 })
        .bind(&session)
        .bind(ts)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn table_transfer(&self, from: i32, to: i32) -> Result<()> {
        let src = self.table_find(from).await?;
        let dst = self.table_find(to).await?;
        let (Some(src), Some(_)) = (src, dst) else {
            return Err(Error::NotFound("桌台不存在".into()));
        };
        let ts = now();
        let session = if src.session_sn.is_empty() {
            format!("T{ts}{to}")
        } else {
            src.session_sn.clone()
        };
        sqlx::query(
            "UPDATE pos_table SET status = 0, table_status = 0, user_id = 0, last_order_id = 0, \
             session_sn = '', update_time = $1 WHERE table_id = $2",
        )
        .bind(ts)
        .bind(from)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        sqlx::query(
            "UPDATE pos_table SET status = 1, table_status = 1, user_id = $1, last_order_id = $2, \
             open_time = $3, session_sn = $4, transfer_from_table_no = $5, update_time = $6 WHERE table_id = $7",
        )
        .bind(src.user_id)
        .bind(src.last_order_id)
        .bind(ts)
        .bind(&session)
        .bind(&src.table_no)
        .bind(ts)
        .bind(to)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 商品 / 会员 / 地址 / 订单 —————————————————————

    /// 收银商品搜索（上架 + 关键词 + 可选类目）。
    pub async fn products(
        &self,
        keyword: Option<&str>,
        category_id: Option<i32>,
        limit: u64,
        offset: u64,
    ) -> Result<(i64, Vec<serde_json::Value>)> {
        let kw = format!("%{}%", keyword.unwrap_or_default().trim());
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product WHERE product_status = 1 AND product_name LIKE $1 \
             AND ($2 IS NULL OR category_id = $3)",
        )
        .bind(&kw)
        .bind(category_id)
        .bind(category_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('productId', product_id, 'productName', product_name, \
             'picThumb', pic_thumb, 'productPrice', product_price, 'marketPrice', market_price, \
             'productStock', product_stock, 'productSn', product_sn, 'categoryId', category_id) \
             FROM product WHERE product_status = 1 AND product_name LIKE $1 \
             AND ($2 IS NULL OR category_id = $3) ORDER BY product_id DESC LIMIT $4 OFFSET $5",
        )
        .bind(&kw)
        .bind(category_id)
        .bind(category_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = rows.into_iter().map(|(v,)| v).collect::<Vec<_>>();
        Ok((total.max(0), rows))
    }

    pub async fn product_one(&self, product_id: i64) -> Result<Option<serde_json::Value>> {
        let row: Option<(serde_json::Value,)> = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('productId', product_id, 'productName', product_name,              'picThumb', pic_thumb, 'productPrice', product_price, 'marketPrice', market_price,              'productStock', product_stock, 'productSn', product_sn, 'productStatus',              product_status) FROM product WHERE product_id = $1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }
    pub async fn categories(&self) -> Result<Vec<serde_json::Value>> {
        Ok(sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('categoryId', category_id, 'categoryName', category_name, \
             'parentId', parent_id) FROM category WHERE parent_id IS NOT NULL ORDER BY category_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(|(v,)| v)
        .collect::<Vec<_>>())
    }

    /// 会员搜索（手机号 / 昵称）。
    pub async fn users(
        &self,
        keyword: Option<&str>,
        user_id: Option<i32>,
        limit: u64,
        offset: u64,
    ) -> Result<(i64, Vec<serde_json::Value>)> {
        let kw = format!("%{}%", keyword.unwrap_or_default().trim());
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"user\" WHERE ($1 IS NOT NULL OR TRUE) AND (nickname LIKE $2 OR mobile LIKE $3 OR user_id = $4)",
        )
        .bind(user_id)
        .bind(&kw)
        .bind(&kw)
        .bind(user_id.unwrap_or(-1))
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('userId', user_id, 'nickname', nickname, 'mobile', mobile, \
             'balance', balance, 'points', points, 'addTime', reg_time) \
             FROM \"user\" WHERE (nickname LIKE $1 OR mobile LIKE $2 OR user_id = $3) \
             ORDER BY user_id DESC LIMIT $4 OFFSET $5",
        )
        .bind(&kw)
        .bind(&kw)
        .bind(user_id.unwrap_or(-1))
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = rows.into_iter().map(|(v,)| v).collect::<Vec<_>>();
        Ok((total.max(0), rows))
    }

    pub async fn user_insert_pos(
        &self,
        mobile: &str,
        nickname: &str,
        password_hash: &str,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO \"user\" (username, avatar, mobile, mobile_validated, nickname, password, reg_time, referrer_user_id) \
             VALUES ($1, '', $2, 1, $3, $4, $5, 0) RETURNING user_id",
        )
        .bind(mobile)
        .bind(mobile)
        .bind(nickname)
        .bind(password_hash)
        .bind(now())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    pub async fn user_addresses(&self, user_id: i32) -> Result<Vec<serde_json::Value>> {
        Ok(sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('addressId', address_id, 'consignee', consignee, \
             'mobile', mobile, 'address', address, 'isDefault', is_default) \
             FROM user_address WHERE user_id = $1 ORDER BY is_default DESC, address_id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(|(v,)| v)
        .collect::<Vec<_>>())
    }

    pub async fn order_remark(&self, user_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT buyer_note FROM \"order\" WHERE user_id = $1 ORDER BY order_id DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 充值单读取（按会员）。
    pub async fn recharge_orders(&self, user_id: i32) -> Result<Vec<serde_json::Value>> {
        Ok(sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('rechargeOrderId', order_id, 'money', amount, \
             'payStatus', status, 'addTime', add_time) \
             FROM user_recharge_order WHERE user_id = $1 ORDER BY order_id DESC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(|(v,)| v)
        .collect::<Vec<_>>())
    }
}
