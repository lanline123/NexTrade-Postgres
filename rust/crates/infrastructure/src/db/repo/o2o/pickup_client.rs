//! o2o H5 前台仓储（o2o::pickup_client）。
//!
//! b2b2b 5.8.57 移动端新增接口（无 Java 控制器对应），按表实现：
//! - `user_pickup_info`：会员自提点/自提人信息 CRUD
//! - `shop_pickup_tpl`：门店自提模板读
//! - `order_pickup_shipment`：订单自提配送读
//! - `product_reservation_stock`：到店预约库存日历读（复用 admin 侧仓储逻辑）

use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};
use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 自提人信息（Java `UserPickupInfo`）。
#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserPickupVO {
    pub user_pickup_id: i32,
    pub user_id: i32,
    pub consignee: String,
    pub mobile: String,
    pub mobile_area_code: String,
    pub is_default: i32,
    pub is_selected: i32,
}

/// 自提模板（Java `ShopPickupTpl` 子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShopPickupTplVO {
    pub shop_pickup_tpl_id: i64,
    pub shop_id: i64,
    pub tpl_name: String,
    pub stocking_status: i32,
    pub stocking_json: Option<String>,
    pub pickup_time_status: i32,
    pub pickup_time_json: Option<String>,
    pub pickup_end_status: i32,
    pub pickup_end_json: Option<String>,
    pub is_default: i32,
}

/// 自提配送单（Java `OrderPickupShipment` 子集）。
#[derive(Debug, Clone, Default, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrderPickupShipmentVO {
    pub shipment_id: i32,
    pub order_id: i32,
    pub pickup_sn: String,
    pub pickup_qr_code: Option<String>,
    pub shipment_status: i32,
    pub check_time: i32,
}

/// 库存日历行（到店预约：日期 × 余量）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockDayVO {
    pub day: String,
    pub total: i32,
    pub used: i32,
    pub available: i32,
}

/// 服务人员上下文（骑手 MVP：抢单大厅/抢单/动作的作用域与状态校验入参）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct StaffContext {
    pub id: i32,
    pub name: String,
    pub phone: String,
    /// 1=配送员 2=服务员。
    #[sqlx(rename = "type")]
    pub staff_type: i32,
    /// 所属店铺（店铺作用域权威维度）。
    pub store_id: i32,
    /// 0=禁用 1=启用。
    pub status: i32,
    /// 0=已下班 1=空闲 2=忙碌。
    pub work_status: i32,
}

impl StaffContext {
    /// 可接单：已启用且未下班。
    #[must_use]
    pub fn can_take_orders(&self) -> bool {
        self.status == 1 && self.work_status >= 1
    }
}

/// 门店聚合搜索候选商品行（自有 + 分配两路合并去重，带命中标记
/// 与同店命中总数）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct StoreSearchCandidateRow {
    pub shop_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub pic_thumb: String,
    pub pic_url: String,
    /// 商品价（`product.product_price`）
    pub product_price: Option<Decimal>,
    /// 门店覆盖价（`store_product.product_price`），自有商品为 NULL
    pub store_price: Option<Decimal>,
    /// 是否命中关键词（关键词为空时恒 true）
    pub is_hit: bool,
    /// 同店去重后的命中总数（窗口回填每一行）
    pub hit_total: i64,
}

#[derive(Clone)]
pub struct PickupClientRepository {
    pool: PgPool,
}

impl PickupClientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 当前会员的自提人列表。
    pub async fn list_by_user(&self, user_id: i32) -> Result<Vec<UserPickupVO>> {
        sqlx::query_as::<_, UserPickupVO>(
            "SELECT user_pickup_id, user_id, consignee, mobile, mobile_area_code, \
             is_default, is_selected FROM user_pickup_info \
             WHERE user_id = $1 ORDER BY is_default DESC, user_pickup_id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 单条（校验归属）。
    pub async fn find_owned(&self, user_id: i32, id: i32) -> Result<Option<UserPickupVO>> {
        sqlx::query_as::<_, UserPickupVO>(
            "SELECT user_pickup_id, user_id, consignee, mobile, mobile_area_code, \
             is_default, is_selected FROM user_pickup_info \
             WHERE user_id = $1 AND user_pickup_id = $2",
        )
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增；`is_default=1` 时清除其它默认。
    pub async fn create(
        &self,
        user_id: i32,
        consignee: &str,
        mobile: &str,
        area_code: &str,
        is_default: bool,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if is_default {
            sqlx::query("UPDATE user_pickup_info SET is_default = 0 WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_pickup_info (user_id, consignee, mobile, mobile_area_code, \
             is_default, is_selected) VALUES ($1, $2, $3, $4, $5, 0) RETURNING user_pickup_id",
        )
        .bind(user_id)
        .bind(consignee)
        .bind(mobile)
        .bind(area_code)
        .bind(i32::from(is_default))
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(r)
    }

    /// 更新（只更新提供的字段）；`is_default=1` 时清除其它默认。
    pub async fn update(
        &self,
        user_id: i32,
        id: i32,
        consignee: Option<&str>,
        mobile: Option<&str>,
        area_code: Option<&str>,
        is_default: Option<bool>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if is_default == Some(true) {
            sqlx::query("UPDATE user_pickup_info SET is_default = 0 WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        if let Some(v) = consignee {
            sqlx::query("UPDATE user_pickup_info SET consignee = $1 WHERE user_id = $2 AND user_pickup_id = $3")
                .bind(v).bind(user_id).bind(id)
                .execute(&mut *tx).await.map_err(map_err)?;
        }
        if let Some(v) = mobile {
            sqlx::query("UPDATE user_pickup_info SET mobile = $1 WHERE user_id = $2 AND user_pickup_id = $3")
                .bind(v).bind(user_id).bind(id)
                .execute(&mut *tx).await.map_err(map_err)?;
        }
        if let Some(v) = area_code {
            sqlx::query("UPDATE user_pickup_info SET mobile_area_code = $1 WHERE user_id = $2 AND user_pickup_id = $3")
                .bind(v).bind(user_id).bind(id)
                .execute(&mut *tx).await.map_err(map_err)?;
        }
        if let Some(v) = is_default {
            sqlx::query("UPDATE user_pickup_info SET is_default = $1 WHERE user_id = $2 AND user_pickup_id = $3")
                .bind(i32::from(v)).bind(user_id).bind(id)
                .execute(&mut *tx).await.map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)
    }

    /// 删除（校验归属）。
    pub async fn delete(&self, user_id: i32, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM user_pickup_info WHERE user_id = $1 AND user_pickup_id = $2")
            .bind(user_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 门店自提模板：门店级优先，缺省回退平台默认（`shop_id=0`）。
    pub async fn find_pickup_tpl(&self, shop_id: i64) -> Result<Option<ShopPickupTplVO>> {
        let row = sqlx::query_as::<_, ShopPickupTplVO>(
            "SELECT shop_pickup_tpl_id, shop_id, tpl_name, stocking_status, stocking_json, \
             pickup_time_status, pickup_time_json, pickup_end_status, pickup_end_json, is_default \
             FROM shop_pickup_tpl WHERE shop_id = $1 ORDER BY is_default DESC LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        if row.is_some() {
            return Ok(row);
        }
        sqlx::query_as::<_, ShopPickupTplVO>(
            "SELECT shop_pickup_tpl_id, shop_id, tpl_name, stocking_status, stocking_json, \
             pickup_time_status, pickup_time_json, pickup_end_status, pickup_end_json, is_default \
             FROM shop_pickup_tpl WHERE shop_id = 0 ORDER BY is_default DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 订单自提配送单。
    pub async fn find_shipment_by_order(
        &self,
        order_id: i32,
    ) -> Result<Option<OrderPickupShipmentVO>> {
        sqlx::query_as::<_, OrderPickupShipmentVO>(
            "SELECT shipment_id, order_id, pickup_sn, pickup_qr_code, shipment_status, check_time \
             FROM order_pickup_shipment WHERE order_id = $1 ORDER BY shipment_id DESC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 门店商品搜索（按 shop_id 过滤上架商品，名称模糊）。
    ///
    /// 返回 `(total, records)`；records 为前端消费的 camelCase JSON 行。
    pub async fn store_product_search(
        &self,
        shop_id: i64,
        keyword: &str,
        limit: u64,
        offset: u64,
    ) -> Result<(i64, Vec<serde_json::Value>)> {
        let kw = format!("%{}%", keyword.trim());
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product WHERE shop_id = $1 AND product_status = 1              AND product_name ILIKE $2",
        )
        .bind(shop_id)
        .bind(&kw)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT json_build_object('productId', product_id, 'productName', product_name,              'picThumb', pic_thumb, 'productPrice', product_price, 'marketPrice', market_price,              'productStock', product_stock)              FROM product WHERE shop_id = $1 AND product_status = 1 AND product_name ILIKE $2              ORDER BY product_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(shop_id)
        .bind(&kw)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = rows.into_iter().map(|(v,)| v).collect::<Vec<_>>();
        Ok((total.max(0), rows))
    }

    /// 门店聚合搜索（`getStoreProduct`）：一批门店的候选商品。
    ///
    /// 候选 = 门店自有在售商品（`product.shop_id`）∪ 经 `store_product`
    /// 分配到该店且分配行在售、商品本体在售的商品；同店同商品两路都
    /// 有时保留分配行（带门店覆盖价）。SQL 内按店截断：命中取最新
    /// 10、未命中取最便宜 10（服务层再组装「命中在前、补齐在后」），
    /// `hit_total` 为同店去重后的命中总数。关键词为空时全部视为命中。
    pub async fn store_search_candidates(
        &self,
        shop_ids: &[i32],
        keyword: &str,
    ) -> Result<Vec<StoreSearchCandidateRow>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let keyword = keyword.trim();
        let empty = keyword.is_empty();
        let pattern = format!("%{keyword}%");
        let mut qb = QueryBuilder::<Postgres>::new(
            "WITH cand AS (\
               SELECT p.shop_id AS shop_id, p.product_id AS product_id, \
                      p.product_name AS product_name, p.pic_thumb AS pic_thumb, \
                      p.pic_url AS pic_url, p.product_price AS product_price, \
                      NULL::numeric AS store_price, (",
        );
        qb.push_bind(empty);
        qb.push(" OR p.product_name ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(
            ") AS is_hit FROM product p \
             WHERE p.product_status = 1 AND p.is_delete = 0 AND p.shop_id IN (",
        );
        for (i, id) in shop_ids.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            qb.push_bind(*id);
        }
        qb.push(
            ") UNION ALL \
             SELECT sp.shop_id::int AS shop_id, p.product_id AS product_id, \
                    p.product_name AS product_name, p.pic_thumb AS pic_thumb, \
                    p.pic_url AS pic_url, p.product_price AS product_price, \
                    sp.product_price AS store_price, (",
        );
        qb.push_bind(empty);
        qb.push(" OR p.product_name ILIKE ");
        qb.push_bind(pattern.clone());
        qb.push(" OR sp.product_name ILIKE ");
        qb.push_bind(pattern);
        qb.push(
            ") AS is_hit FROM store_product sp \
             JOIN product p ON p.product_id = sp.product_id \
             WHERE sp.product_status = 1 AND sp.is_delete = 0 \
               AND p.product_status = 1 AND p.is_delete = 0 AND sp.shop_id IN (",
        );
        for (i, id) in shop_ids.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            qb.push_bind(i64::from(*id));
        }
        qb.push(
            ")), \
             dedup AS (SELECT *, ROW_NUMBER() OVER (\
               PARTITION BY shop_id, product_id ORDER BY store_price NULLS LAST\
             ) AS rn FROM cand), \
             ranked AS (SELECT *, \
               COUNT(*) FILTER (WHERE is_hit) OVER (PARTITION BY shop_id) AS hit_total, \
               ROW_NUMBER() OVER (\
                 PARTITION BY shop_id, is_hit ORDER BY product_id DESC\
               ) AS hit_rank, \
               ROW_NUMBER() OVER (\
                 PARTITION BY shop_id, is_hit \
                 ORDER BY COALESCE(store_price, product_price) ASC NULLS LAST, product_id DESC\
               ) AS fill_rank \
             FROM dedup WHERE rn = 1) \
             SELECT shop_id, product_id, product_name, pic_thumb, pic_url, \
                    product_price, store_price, is_hit, hit_total \
             FROM ranked \
             WHERE (is_hit AND hit_rank <= 10) OR (NOT is_hit AND fill_rank <= 10) \
             ORDER BY shop_id ASC, is_hit DESC, product_id DESC",
        );
        qb.build_query_as::<StoreSearchCandidateRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 到店预约库存日历：某商品（+可选门店）未来 \"days\" 天的余量。
    ///
    /// \"product_reservation_stock\" 行语义：available = total - used（见 admin 侧
    /// StockCalendarRepository 同口径）。
    pub async fn stock_calendar(
        &self,
        product_id: i32,
        shop_id: Option<i64>,
        days: u32,
    ) -> Result<Vec<StockDayVO>> {
        let today = chrono::Utc::now().date_naive();
        let end = today + chrono::Duration::days(i64::from(days.min(31)));
        let rows: Vec<(String, i32, i32)> = match shop_id {
            Some(sid) => sqlx::query_as(
                "SELECT stock_date, total_stock, used_stock FROM product_reservation_stock \
                 WHERE product_id = $1 AND shop_id = $2 AND stock_date BETWEEN $3 AND $4 \
                 ORDER BY stock_date",
            )
            .bind(product_id)
            .bind(sid)
            .bind(today.format("%Y-%m-%d").to_string())
            .bind(end.format("%Y-%m-%d").to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?,
            None => sqlx::query_as(
                "SELECT stock_date, SUM(total_stock), SUM(used_stock) \
                 FROM product_reservation_stock \
                 WHERE product_id = $1 AND stock_date BETWEEN $2 AND $3 \
                 GROUP BY stock_date ORDER BY stock_date",
            )
            .bind(product_id)
            .bind(today.format("%Y-%m-%d").to_string())
            .bind(end.format("%Y-%m-%d").to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?,
        };
        Ok(rows
            .into_iter()
            .map(|(day, total, used)| StockDayVO {
                available: (total - used).max(0),
                day,
                total,
                used,
            })
            .collect())
    }

    // ————————————————————— C 端派送员/服务员 —————————————————————

    /// 当前会员绑定的服务人员。
    pub async fn staff_by_member(&self, member_id: i32) -> Result<Option<(i32, String, String, i32)>> {
        sqlx::query_as(
            "SELECT id, name, phone, type FROM service_staff WHERE member_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 当前会员绑定的服务人员（完整上下文：店铺作用域 + 状态 + 工作状态）。
    ///
    /// 骑手 MVP 抢单大厅/抢单的作用域校验以此为准（service_staff.store_id 为
    /// 店铺维度权威，不另起权限体系）。
    pub async fn staff_context_by_member(
        &self,
        member_id: i32,
    ) -> Result<Option<StaffContext>> {
        sqlx::query_as::<_, StaffContext>(
            "SELECT id, name, phone, type, store_id, status, work_status \
             FROM service_staff WHERE member_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 服务人员资料（C 端 myInfo）。
    pub async fn staff_info_by_member(&self, member_id: i32) -> Result<Option<Value>> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('id', id, 'name', name, 'nickname', nickname, 'avatar', avatar, 'phone', phone, 'storeId', store_id, 'type', type, 'status', status, 'workStatus', work_status, 'completedOrderCount', completed_order_count, 'completedOrderAmount', completed_order_amount) FROM service_staff WHERE member_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    /// 更新工作状态（C 端上下班/空闲/忙碌）。
    pub async fn update_work_status(&self, member_id: i32, work_status: i32) -> Result<bool> {
        let r = sqlx::query("UPDATE service_staff SET work_status = $1 WHERE member_id = $2 AND is_delete = 0")
            .bind(work_status)
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 个人钱包。
    pub async fn wallet_by_staff(&self, staff_id: i32) -> Result<Value> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('walletId', wallet_id, 'staffId', staff_id, 'balance', balance, 'frozenBalance', frozen_balance, 'totalIncome', total_income, 'createTime', create_time, 'updateTime', update_time) FROM service_staff_wallet WHERE staff_id = $1",
        )
        .bind(staff_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row
            .map(|(v,)| v)
            .unwrap_or_else(|| serde_json::json!({"walletId": 0, "staffId": staff_id, "balance": 0, "frozenBalance": 0, "totalIncome": 0})))
    }

    /// 个人钱包流水。
    pub async fn wallet_logs(&self, staff_id: i32, page: u64, size: u64) -> Result<(i64, Vec<Value>)> {
        let offset = (page.max(1) - 1) * size;
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM service_staff_wallet_log WHERE staff_id = $1",
        )
        .bind(staff_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('logId', log_id, 'staffId', staff_id, 'orderId', order_id, 'withdrawId', withdraw_id, 'transactionSn', transaction_sn, 'amount', amount, 'balanceAfter', balance_after, 'frozenAfter', frozen_after, 'businessType', business_type, 'type', type, 'remark', remark, 'createTime', create_time) FROM service_staff_wallet_log WHERE staff_id = $1 ORDER BY log_id DESC LIMIT $2 OFFSET $3",
        )
        .bind(staff_id)
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total, rows.into_iter().map(|(v,)| v).collect()))
    }

    // ————————————————————— 收款账号 / 提现查询 —————————————————————

    pub async fn accounts_of_user(&self, user_id: i32) -> Result<Vec<Value>> {
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('accountId', account_id, 'accountType', account_type, 'accountName', account_name, 'accountNo', account_no, 'bankName', bank_name, 'identity', identity) FROM user_withdraw_account WHERE user_id = $1 ORDER BY account_id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    pub async fn save_account(
        &self,
        user_id: i32,
        account_id: i32,
        account_type: i32,
        account_name: &str,
        account_no: &str,
        bank_name: &str,
    ) -> Result<i32> {
        if account_id > 0 {
            sqlx::query("UPDATE user_withdraw_account SET account_type = $1, account_name = $2, account_no = $3, bank_name = $4 WHERE account_id = $5 AND user_id = $6")
                .bind(account_type)
                .bind(account_name)
                .bind(account_no)
                .bind(bank_name)
                .bind(account_id)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
            Ok(account_id)
        } else {
            let r: i32 = sqlx::query_scalar("INSERT INTO user_withdraw_account (user_id, account_type, account_name, account_no, bank_name) VALUES ($1, $2, $3, $4, $5) RETURNING account_id")
                .bind(user_id)
                .bind(account_type)
                .bind(account_name)
                .bind(account_no)
                .bind(bank_name)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
            Ok(r)
        }
    }

    pub async fn delete_account(&self, user_id: i32, account_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM user_withdraw_account WHERE account_id = $1 AND user_id = $2")
            .bind(account_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    pub async fn withdraw_list(
        &self,
        staff_id: i32,
        status: Option<i32>,
        page: u64,
        size: u64,
    ) -> Result<(i64, Vec<Value>)> {
        let offset = (page.max(1) - 1) * size;
        let status = status.unwrap_or(-1);
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM service_staff_withdraw WHERE staff_id = $1 AND is_delete = 0 AND ($2 = -1 OR status = $3)",
        )
        .bind(staff_id)
        .bind(status)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('withdrawId', withdraw_id, 'withdrawSn', withdraw_sn, 'staffId', staff_id, 'businessType', business_type, 'amount', amount, 'status', status, 'accountId', account_id, 'accountType', account_type, 'accountName', account_name, 'accountNo', account_no, 'bankName', bank_name, 'adminRemark', admin_remark, 'paymentVoucher', payment_voucher, 'rejectReason', reject_reason, 'applyTime', apply_time, 'auditTime', audit_time) FROM service_staff_withdraw WHERE staff_id = $1 AND is_delete = 0 AND ($2 = -1 OR status = $3) ORDER BY withdraw_id DESC LIMIT $4 OFFSET $5",
        )
        .bind(staff_id)
        .bind(status)
        .bind(status)
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total, rows.into_iter().map(|(v,)| v).collect()))
    }

    pub async fn withdraw_detail(&self, staff_id: i32, id: i32) -> Result<Option<Value>> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('withdrawId', withdraw_id, 'withdrawSn', withdraw_sn, 'staffId', staff_id, 'businessType', business_type, 'amount', amount, 'status', status, 'accountId', account_id, 'accountType', account_type, 'accountName', account_name, 'accountNo', account_no, 'bankName', bank_name, 'adminRemark', admin_remark, 'paymentVoucher', payment_voucher, 'rejectReason', reject_reason, 'applyTime', apply_time, 'auditTime', audit_time) FROM service_staff_withdraw WHERE withdraw_id = $1 AND staff_id = $2 AND is_delete = 0",
        )
        .bind(id)
        .bind(staff_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    /// 申请提现：冻结余额 + 提现单 + 流水，单事务。
    pub async fn apply_withdraw(
        &self,
        staff_id: i32,
        business_type: i32,
        amount: rust_decimal::Decimal,
        account_id: i32,
        remark: &str,
    ) -> Result<i32> {
        if amount <= rust_decimal::Decimal::ZERO {
            return Err(Error::Validation("提现金额必须大于 0".into()));
        }
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let account: Option<(i32, String, String, String)> = sqlx::query_as(
            "SELECT account_type, account_name, account_no, bank_name FROM user_withdraw_account WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let (account_type, account_name, account_no, bank_name) =
            account.ok_or_else(|| Error::Validation("收款账号不存在".into()))?;
        let balance: Option<(rust_decimal::Decimal,)> = sqlx::query_as(
            "SELECT balance FROM service_staff_wallet WHERE staff_id = $1 FOR UPDATE",
        )
        .bind(staff_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let Some((balance,)) = balance else {
            return Err(Error::Validation("钱包不存在".into()));
        };
        if balance < amount {
            return Err(Error::Validation("可提现余额不足".into()));
        }
        let now = nextrade_common::utils::now_seconds() as i64;
        let withdraw_sn = format!("WD{now}{staff_id}");
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO service_staff_withdraw (withdraw_sn, staff_id, business_type, amount, status, account_id, account_type, account_name, account_no, bank_name, apply_time, frequency_unit, period_start, period_end, create_time, update_time) VALUES ($1, $2, $3, $4, 0, $5, $6, $7, $8, $9, $10, 'day', $11, $12, $13, $14) RETURNING withdraw_id",
        )
        .bind(&withdraw_sn)
        .bind(staff_id)
        .bind(business_type)
        .bind(amount)
        .bind(account_id)
        .bind(account_type)
        .bind(&account_name)
        .bind(&account_no)
        .bind(&bank_name)
        .bind(now)
        .bind(now / 86400 * 86400)
        .bind(now / 86400 * 86400 + 86400)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        let withdraw_id = r;
        sqlx::query("UPDATE service_staff_wallet SET balance = balance - $1, frozen_balance = frozen_balance + $2, update_time = $3 WHERE staff_id = $4")
            .bind(amount)
            .bind(amount)
            .bind(now)
            .bind(staff_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("INSERT INTO service_staff_wallet_log (staff_id, amount, balance_after, frozen_after, business_type, type, remark, create_time) VALUES ($1, $2, $3, $4, $5, 4, $6, $7)")
            .bind(staff_id)
            .bind(-amount)
            .bind(balance - amount)
            .bind(amount)
            .bind(business_type)
            .bind(remark)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(withdraw_id)
    }

    /// 门店同城配送配置 + 距离溢价。
    pub async fn local_delivery_config(&self, shop_id: i32) -> Result<Value> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('shopId', shop_id, 'enabled', enabled, 'deliveryRadius', delivery_radius, 'minOrderAmount', min_order_amount, 'baseDeliveryFee', base_delivery_fee, 'distancePricingEnabled', distance_pricing_enabled) FROM shop_local_delivery_config WHERE shop_id = $1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let Some((mut obj,)) = row else { return Ok(Value::Null) };
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('distanceThreshold', p.distance_threshold, 'distanceUnit', p.distance_unit, 'additionalFee', p.additional_fee) FROM shop_local_delivery_distance_pricing p JOIN shop_local_delivery_config c ON c.shop_local_delivery_config_id = p.shop_local_delivery_config_id WHERE c.shop_id = $1 ORDER BY p.sort_order",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        obj["distancePricing"] = Value::Array(rows.into_iter().map(|(v,)| v).collect());
        Ok(obj)
    }

    /// 门店上门服务配置 + 距离溢价。
    pub async fn home_service_config(&self, shop_id: i32) -> Result<Value> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('shopId', shop_id, 'enabled', enabled, 'serviceRadius', service_radius, 'homeFee', home_fee, 'distancePricingEnabled', distance_pricing_enabled, 'orderDispatchMode', order_dispatch_mode, 'beforeServicePhotoEnabled', before_service_photo_enabled, 'afterServicePhotoEnabled', after_service_photo_enabled) FROM shop_home_service_config WHERE shop_id = $1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let Some((mut obj,)) = row else { return Ok(Value::Null) };
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('distanceThreshold', p.distance_threshold, 'distanceUnit', p.distance_unit, 'additionalFee', p.additional_fee) FROM shop_home_service_distance_pricing p JOIN shop_home_service_config c ON c.shop_home_service_config_id = p.shop_home_service_config_id WHERE c.shop_id = $1 ORDER BY p.sort_order",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        obj["distancePricing"] = Value::Array(rows.into_iter().map(|(v,)| v).collect());
        Ok(obj)
    }

    pub async fn delivery_list(
        &self,
        staff_id: i32,
        tab: Option<&str>,
        page: u64,
        size: u64,
    ) -> Result<(i64, Vec<Value>)> {
        let (status_cond, status_val) = match tab.unwrap_or_default() {
            "pending" => ("AND status = ?", Some(20)),
            "processing" => ("AND status = ?", Some(30)),
            "completed" => ("AND status = ?", Some(40)),
            _ => ("", None),
        };
        let offset = (page.max(1) - 1) * size;
        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM delivery_order WHERE staff_id = ? {status_cond}"
        ));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql)
        .bind(staff_id);
        if let Some(v) = status_val {
            count_q = count_q.bind(v);
        }
        let (total,) = count_q.fetch_one(&self.pool).await.map_err(map_err)?;
        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT json_build_object('deliveryOrderId', delivery_order_id, 'deliveryOrderSn', delivery_order_sn, 'orderId', order_id, 'orderSn', order_sn, 'shopId', shop_id, 'type', type, 'dispatchMode', dispatch_mode, 'status', status, 'staffId', staff_id, 'staffName', staff_name, 'staffPhone', staff_phone, 'consignee', consignee, 'mobile', mobile, 'address', address, 'regionNames', region_names, 'orderAmount', order_amount, 'buyerNote', buyer_note, 'dispatchTime', dispatch_time, 'grabTime', grab_time, 'pickupTime', pickup_time, 'completeTime', complete_time, 'cancelTime', cancel_time, 'cancelReason', cancel_reason, 'createTime', create_time) FROM delivery_order WHERE staff_id = ? {status_cond} ORDER BY delivery_order_id DESC LIMIT ? OFFSET ?"
        ));
        let mut q = sqlx::query_as::<_, (Value,)>(&list_sql)
        .bind(staff_id);
        if let Some(v) = status_val {
            q = q.bind(v);
        }
        let rows = q.bind(size as i64).bind(offset as i64).fetch_all(&self.pool).await.map_err(map_err)?;
        Ok((total, rows.into_iter().map(|(v,)| v).collect()))
    }

    /// C 端派送订单详情（只允许本人订单）。
    pub async fn delivery_detail(&self, staff_id: i32, id: i32) -> Result<Option<Value>> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('deliveryOrderId', delivery_order_id, 'deliveryOrderSn', delivery_order_sn, 'orderId', order_id, 'orderSn', order_sn, 'shopId', shop_id, 'type', type, 'dispatchMode', dispatch_mode, 'status', status, 'staffId', staff_id, 'staffName', staff_name, 'staffPhone', staff_phone, 'consignee', consignee, 'mobile', mobile, 'address', address, 'regionNames', region_names, 'latitude', latitude, 'longitude', longitude, 'pickupAddress', pickup_address, 'pickupPhone', pickup_phone, 'serviceTime', service_time, 'orderAmount', order_amount, 'buyerNote', buyer_note, 'dispatchTime', dispatch_time, 'grabTime', grab_time, 'pickupTime', pickup_time, 'completeTime', complete_time, 'cancelTime', cancel_time, 'cancelReason', cancel_reason, 'createTime', create_time) FROM delivery_order WHERE delivery_order_id = $1 AND staff_id = $2",
        )
        .bind(id)
        .bind(staff_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }

    /// 抢单：仅待派发且抢单模式的订单。
    ///
    /// 骑手 MVP 作用域修复（原实现为全平台大厅、无任何校验）：原子 UPDATE 内
    /// 同时校验 ① 订单店铺 == 人员所属店铺；② 订单类型 == 人员类型
    /// （配送员只抢配送单、服务员只抢服务单）；③ 人员在职（status=1）。
    /// 调用方须先经 `StaffContext::can_take_orders` 拦截下班/禁用人员。
    pub async fn grab_delivery(&self, staff: &StaffContext, id: i32) -> Result<bool> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let r = sqlx::query(
            "UPDATE delivery_order SET staff_id = $1, staff_name = $2, staff_phone = $3, status = 20, grab_time = $4 \
             WHERE delivery_order_id = $5 AND status = 10 AND dispatch_mode = 1 AND staff_id IS NULL \
             AND shop_id = $6 AND type = $7 AND is_delete = 0",
        )
        .bind(staff.id)
        .bind(&staff.name)
        .bind(&staff.phone)
        .bind(now)
        .bind(id)
        .bind(staff.store_id)
        .bind(staff.staff_type)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 确认取货/开始服务（20→30）；成功后人员工作状态置忙碌（2，仅从空闲迁移）。
    pub async fn confirm_pickup(&self, staff_id: i32, id: i32, photos: &str) -> Result<bool> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let r = sqlx::query(
            "UPDATE delivery_order SET status = 30, pickup_time = $1, pickup_photo = $2 WHERE delivery_order_id = $3 AND staff_id = $4 AND status = 20",
        )
        .bind(now)
        .bind(photos)
        .bind(id)
        .bind(staff_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        if r.rows_affected() > 0 {
            sqlx::query(
                "UPDATE service_staff SET work_status = 2 WHERE id = $1 AND work_status = 1 AND is_delete = 0",
            )
            .bind(staff_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 完成配送/服务（30→40）+ 服务费入账，单事务。
    ///
    /// 骑手 MVP 核心激活（原实现只翻状态、从不入账）：状态翻转成功即订单级
    /// 幂等门（重复完成翻不动 → 返回 None 不重复入账）。费率三级链见
    /// [`compute_service_fee`]。入账同事务完成：钱包 upsert（无钱包行则建）、
    /// 流水 type=1（transaction_sn = 派送单号）、人员完成计数/金额累加、
    /// 派送单 service_fee 快照、无在途单时工作状态 2→1 回落。
    /// 返回 `Some(入账金额)`（0 = 无服务费，仅翻状态）；`None` = 状态不允许。
    pub async fn complete_delivery(
        &self,
        staff_id: i32,
        id: i32,
        photos: &str,
    ) -> Result<Option<rust_decimal::Decimal>> {
        use rust_decimal::Decimal;
        let now = nextrade_common::utils::now_seconds() as i64;
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let flipped: Option<(String, i32, i32, i32, Decimal)> = sqlx::query_as(
            "UPDATE delivery_order SET status = 40, complete_time = $1, complete_photo = $2 \
             WHERE delivery_order_id = $3 AND staff_id = $4 AND status = 30 \
             RETURNING delivery_order_sn, order_id, shop_id, type, order_amount",
        )
        .bind(now as i32)
        .bind(photos)
        .bind(id)
        .bind(staff_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let Some((delivery_sn, order_id, shop_id, order_type, order_amount)) = flipped else {
            tx.rollback().await.map_err(map_err)?;
            return Ok(None);
        };

        // ——— 费率三级链入参 ———
        let items: Vec<(i32, i32, Decimal, i32)> = sqlx::query_as(
            "SELECT product_id, sku_id, price, quantity FROM order_item WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_err)?;
        // 费率列按派送单类型二选一（type=1 配送列 / type=2 服务列）。
        let (p_type_col, p_val_col, s_type_col, s_val_col) = if order_type == 2 {
            (
                "server_service_fee_type",
                "server_service_fee_value",
                "server_service_fee_type",
                "server_service_fee_value",
            )
        } else {
            (
                "delivery_service_fee_type",
                "delivery_service_fee_value",
                "delivery_service_fee_type",
                "delivery_service_fee_value",
            )
        };
        let product_sql = format!(
            "SELECT {p_type_col}, {p_val_col} FROM store_product \
             WHERE product_id = $1 AND shop_id = $2 AND is_delete = 0 LIMIT 1"
        );
        let sku_sql = format!(
            "SELECT {s_type_col}, {s_val_col} FROM store_product_sku \
             WHERE product_id = $1 AND sku_id = $2 AND shop_id = $3 AND is_delete = 0 LIMIT 1"
        );
        let mut fee_items: Vec<FeeItem> = Vec::with_capacity(items.len());
        for (product_id, sku_id, price, quantity) in items {
            let (fee_type, fee_value): (i32, Decimal) = sqlx::query_as(&product_sql)
                .bind(product_id)
                .bind(shop_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_err)?
                .unwrap_or((0, Decimal::ZERO));
            let mut item = FeeItem {
                price,
                quantity,
                product_fee: (fee_value > Decimal::ZERO).then_some((fee_type, fee_value)),
                sku_fee: None,
            };
            if sku_id > 0 {
                let sku_row: Option<(i32, Decimal)> = sqlx::query_as(&sku_sql)
                    .bind(product_id)
                    .bind(sku_id)
                    .bind(shop_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_err)?;
                if let Some((t, v)) = sku_row {
                    if v > Decimal::ZERO {
                        item.sku_fee = Some((t, v));
                    }
                }
            }
            fee_items.push(item);
        }
        let (assign_code, enabled_code, rate_code) = if order_type == 2 {
            (
                "storeAssignServerServiceFee",
                "serverServiceFeeEnabled",
                "defaultServerServiceFeeRate",
            )
        } else {
            (
                "storeAssignDeliveryServiceFee",
                "deliveryServiceFeeEnabled",
                "defaultDeliveryServiceFeeRate",
            )
        };
        let cfg_rows: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT biz_code, biz_val FROM config WHERE biz_code = ANY($1) AND (is_del IS NULL OR is_del = 0)",
        )
        .bind([assign_code, enabled_code, rate_code])
        .fetch_all(&mut *tx)
        .await
        .map_err(map_err)?;
        let cfg_get = |code: &str| -> Option<String> {
            cfg_rows
                .iter()
                .find(|(c, _)| c == code)
                .and_then(|(_, v)| v.clone())
        };
        // 平台配置缺省口径与 config_basic FieldSpec 默认一致：开关开、费率 0、总部统一。
        let fee_enabled = cfg_get(enabled_code).as_deref() != Some("0");
        let store_assign = cfg_get(assign_code).as_deref() == Some("1");
        let platform_rate: Decimal = cfg_get(rate_code)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(Decimal::ZERO);
        let shop_default: (i32, Decimal) = if order_type == 2 {
            sqlx::query_as(
                "SELECT default_service_fee_type, default_service_fee_value \
                 FROM shop_home_service_config WHERE shop_id = $1 LIMIT 1",
            )
            .bind(shop_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_err)?
            .unwrap_or((0, Decimal::ZERO))
        } else {
            sqlx::query_as(
                "SELECT default_service_fee_type, default_service_fee_value \
                 FROM shop_local_delivery_config WHERE shop_id = $1 LIMIT 1",
            )
            .bind(shop_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_err)?
            .unwrap_or((0, Decimal::ZERO))
        };
        let fee = compute_service_fee(&fee_items, shop_default, store_assign, fee_enabled, platform_rate);

        // ——— 入账（fee > 0 才写钱包/流水） ———
        if fee > Decimal::ZERO {
            let (wallet_id,): (i32,) = sqlx::query_as(
                "INSERT INTO service_staff_wallet (staff_id, balance, frozen_balance, total_income, create_time, update_time) \
                 VALUES ($1, 0, 0, 0, $2, $2) \
                 ON CONFLICT (staff_id) DO UPDATE SET staff_id = EXCLUDED.staff_id \
                 RETURNING wallet_id",
            )
            .bind(staff_id)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_err)?;
            let _ = wallet_id;
            let (balance_after, frozen_after): (Decimal, Decimal) = sqlx::query_as(
                "UPDATE service_staff_wallet SET balance = balance + $1, total_income = total_income + $1, update_time = $2 \
                 WHERE staff_id = $3 RETURNING balance, frozen_balance",
            )
            .bind(fee)
            .bind(now)
            .bind(staff_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_err)?;
            sqlx::query(
                "INSERT INTO service_staff_wallet_log (staff_id, order_id, transaction_sn, amount, balance_after, frozen_after, business_type, type, remark, create_time) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $9)",
            )
            .bind(staff_id)
            .bind(order_id)
            .bind(&delivery_sn)
            .bind(fee)
            .bind(balance_after)
            .bind(frozen_after)
            .bind(order_type)
            .bind(if order_type == 2 { "上门服务费" } else { "配送服务费" })
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
            sqlx::query("UPDATE delivery_order SET service_fee = $1 WHERE delivery_order_id = $2")
                .bind(fee)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }

        // ——— 人员完成统计 + 工作状态回落 ———
        sqlx::query(
            "UPDATE service_staff SET completed_order_count = completed_order_count + 1, \
             completed_order_amount = completed_order_amount + $1 WHERE id = $2 AND is_delete = 0",
        )
        .bind(order_amount)
        .bind(staff_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        let (in_flight,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM delivery_order WHERE staff_id = $1 AND status = 30 AND is_delete = 0",
        )
        .bind(staff_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        if in_flight == 0 {
            sqlx::query(
                "UPDATE service_staff SET work_status = 1 WHERE id = $1 AND work_status = 2 AND is_delete = 0",
            )
            .bind(staff_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(Some(fee))
    }

    /// C 端取消（补 cancel_by=1 人员口径；原实现漏写）。
    pub async fn cancel_delivery(&self, staff_id: i32, id: i32, reason: &str) -> Result<bool> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let r = sqlx::query(
            "UPDATE delivery_order SET status = 50, cancel_reason = $1, cancel_time = $2, cancel_by = 1 WHERE delivery_order_id = $3 AND staff_id = $4 AND status IN (20,30)",
        )
        .bind(reason)
        .bind(now)
        .bind(id)
        .bind(staff_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 待抢单列表（抢单模式、未分配）。
    ///
    /// 骑手 MVP 作用域修复（原实现不验身份、全平台一个大厅）：强制
    /// shop_id = 人员所属店铺 AND type = 人员类型；`type` 入参只能在此基础上
    /// 再收窄（传异类返回空）。跨店单在本店大厅不可见（负例验收点）。
    pub async fn grab_list(
        &self,
        staff: &StaffContext,
        page: u64,
        size: u64,
        r#type: Option<i32>,
    ) -> Result<(i64, Vec<Value>)> {
        let offset = (page.max(1) - 1) * size;
        let want_type = r#type.unwrap_or(staff.staff_type);
        if want_type != staff.staff_type {
            return Ok((0, Vec::new()));
        }
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM delivery_order WHERE status = 10 AND dispatch_mode = 1 \
             AND staff_id IS NULL AND shop_id = $1 AND type = $2 AND is_delete = 0",
        )
        .bind(staff.store_id)
        .bind(staff.staff_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('deliveryOrderId', delivery_order_id, 'deliveryOrderSn', delivery_order_sn, 'orderId', order_id, 'orderSn', order_sn, 'shopId', shop_id, 'type', type, 'status', status, 'consignee', consignee, 'mobile', mobile, 'address', address, 'regionNames', region_names, 'latitude', latitude, 'longitude', longitude, 'pickupAddress', pickup_address, 'pickupRegionNames', pickup_region_names, 'orderAmount', order_amount, 'serviceTime', service_time, 'createTime', create_time) FROM delivery_order WHERE status = 10 AND dispatch_mode = 1 AND staff_id IS NULL AND shop_id = $1 AND type = $2 AND is_delete = 0 ORDER BY delivery_order_id DESC LIMIT $3 OFFSET $4",
        )
        .bind(staff.store_id)
        .bind(staff.staff_type)
        .bind(size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total, rows.into_iter().map(|(v,)| v).collect()))
    }

    // ————————————————————— 骑手 MVP：生产者 / 买家进度 —————————————————————

    /// 某主订单是否已有未删除的同类派送单（生产者订单级幂等查）。
    pub async fn find_delivery_by_order(
        &self,
        order_id: i32,
        order_type: i32,
    ) -> Result<Option<i32>> {
        sqlx::query_scalar(
            "SELECT delivery_order_id FROM delivery_order \
             WHERE order_id = $1 AND type = $2 AND is_delete = 0 LIMIT 1",
        )
        .bind(order_id)
        .bind(order_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新建派送单（生产者专用；字段由服务层按订单/店铺/配置组装）。
    #[allow(clippy::too_many_arguments)]
    pub async fn create_delivery_order(
        &self,
        sn: &str,
        order_id: i32,
        order_sn: &str,
        shop_id: i32,
        order_type: i32,
        dispatch_mode: i32,
        consignee: &str,
        mobile: &str,
        address: &str,
        region_names: &str,
        latitude: Option<rust_decimal::Decimal>,
        longitude: Option<rust_decimal::Decimal>,
        pickup_address: &str,
        pickup_region_names: &str,
        pickup_phone: &str,
        pickup_latitude: Option<rust_decimal::Decimal>,
        pickup_longitude: Option<rust_decimal::Decimal>,
        service_time: Option<i32>,
        order_amount: rust_decimal::Decimal,
        buyer_note: &str,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        sqlx::query_scalar(
            "INSERT INTO delivery_order (delivery_order_sn, order_id, order_sn, shop_id, type, \
             dispatch_mode, status, consignee, mobile, address, region_names, latitude, longitude, \
             pickup_address, pickup_region_names, pickup_phone, pickup_latitude, pickup_longitude, \
             service_time, order_amount, buyer_note, carrier_type, create_time, update_time, is_delete) \
             VALUES ($1,$2,$3,$4,$5,$6,10,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,1,$21,$21,0) \
             RETURNING delivery_order_id",
        )
        .bind(sn)
        .bind(order_id)
        .bind(order_sn)
        .bind(shop_id)
        .bind(order_type)
        .bind(dispatch_mode)
        .bind(consignee)
        .bind(mobile)
        .bind(address)
        .bind(region_names)
        .bind(latitude)
        .bind(longitude)
        .bind(pickup_address)
        .bind(pickup_region_names)
        .bind(pickup_phone)
        .bind(pickup_latitude)
        .bind(pickup_longitude)
        .bind(service_time)
        .bind(order_amount)
        .bind(buyer_note)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 骑手 MVP 阶段二：入驻申请 —————————————————————

    /// 提交入驻申请（会员 → 某店铺的配送员/服务员）。
    ///
    /// 守卫：同会员同店同类型已有待审核申请 → 拒绝重复提交；已是该店同类型
    /// 在册人员 → 拒绝。返回新申请 id。
    #[allow(clippy::too_many_arguments)]
    pub async fn create_apply(
        &self,
        member_id: i32,
        shop_id: i32,
        apply_type: i32,
        name: &str,
        phone: &str,
        id_card: &str,
        remark: &str,
    ) -> Result<i32> {
        let (pending,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM service_staff_apply WHERE member_id = $1 AND shop_id = $2 AND type = $3 AND status = 0 AND is_delete = 0",
        )
        .bind(member_id)
        .bind(shop_id)
        .bind(apply_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        if pending > 0 {
            return Err(Error::Validation("已有待审核的入驻申请，请勿重复提交".into()));
        }
        let (bound,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM service_staff WHERE member_id = $1 AND store_id = $2 AND type = $3 AND is_delete = 0",
        )
        .bind(member_id)
        .bind(shop_id)
        .bind(apply_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        if bound > 0 {
            return Err(Error::Validation("您已是该店铺的服务人员".into()));
        }
        let now = nextrade_common::utils::now_seconds();
        sqlx::query_scalar(
            "INSERT INTO service_staff_apply (member_id, shop_id, type, name, phone, id_card, remark, status, create_time, update_time) VALUES ($1,$2,$3,$4,$5,$6,$7,0,$8,$8) RETURNING apply_id",
        )
        .bind(member_id)
        .bind(shop_id)
        .bind(apply_type)
        .bind(name)
        .bind(phone)
        .bind(id_card)
        .bind(remark)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 我的入驻申请（会员侧，含审核状态与驳回原因）。
    pub async fn applies_of_member(&self, member_id: i32) -> Result<Vec<Value>> {
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('applyId', apply_id, 'shopId', shop_id, 'type', type, 'name', name, 'phone', phone, 'status', status, 'auditRemark', audit_remark, 'staffId', staff_id, 'createTime', create_time, 'updateTime', update_time) FROM service_staff_apply WHERE member_id = $1 AND is_delete = 0 ORDER BY apply_id DESC",
        )
        .bind(member_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    /// 主订单归属会员（买家进度接口的归属校验）。
    pub async fn order_owner(&self, order_id: i32) -> Result<Option<i32>> {
        sqlx::query_scalar("SELECT user_id FROM \"order\" WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 状态动作点定位打点（阶段三：仅接单/取货/送达三个动作点，无连续轨迹）
    /// + 最新位置 upsert。经纬度缺失时静默跳过（H5 定位授权失败不阻塞动作）。
    pub async fn record_track_point(
        &self,
        staff_id: i32,
        delivery_order_id: i32,
        action: &str,
        longitude: Option<rust_decimal::Decimal>,
        latitude: Option<rust_decimal::Decimal>,
    ) -> Result<()> {
        let (Some(lng), Some(lat)) = (longitude, latitude) else {
            return Ok(());
        };
        let now = nextrade_common::utils::now_seconds();
        sqlx::query(
            "INSERT INTO staff_track_point (staff_id, delivery_order_id, action, longitude, latitude, create_time) VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(staff_id)
        .bind(delivery_order_id)
        .bind(action)
        .bind(lng)
        .bind(lat)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        sqlx::query(
            "INSERT INTO staff_location_latest (staff_id, longitude, latitude, update_time) VALUES ($1,$2,$3,$4) \
             ON CONFLICT (staff_id) DO UPDATE SET longitude = EXCLUDED.longitude, latitude = EXCLUDED.latitude, update_time = EXCLUDED.update_time",
        )
        .bind(staff_id)
        .bind(lng)
        .bind(lat)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 买家侧派送进度（按主订单取最新一张派送单；归属校验在服务层做）。
    pub async fn buyer_delivery_by_order(&self, order_id: i32) -> Result<Option<Value>> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT json_build_object('deliveryOrderId', delivery_order_id, 'deliveryOrderSn', delivery_order_sn, 'orderId', order_id, 'type', type, 'status', status, 'staffName', staff_name, 'staffPhone', staff_phone, 'createTime', create_time, 'dispatchTime', dispatch_time, 'grabTime', grab_time, 'pickupTime', pickup_time, 'completeTime', complete_time, 'cancelTime', cancel_time, 'serviceFee', service_fee) FROM delivery_order WHERE order_id = $1 AND is_delete = 0 ORDER BY delivery_order_id DESC LIMIT 1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(v,)| v))
    }
}

/// 费率计算入参行（订单商品行 × 费率解析结果）。
#[derive(Debug, Clone, Copy)]
pub struct FeeItem {
    /// 商品行成交单价。
    pub price: rust_decimal::Decimal,
    /// 购买数量。
    pub quantity: i32,
    /// 商品级费率 (type, value)：type 0=固定元/件，1=比例%（value<=0 视为未设置，不入参）。
    pub product_fee: Option<(i32, rust_decimal::Decimal)>,
    /// SKU 级费率（优先于商品级）。
    pub sku_fee: Option<(i32, rust_decimal::Decimal)>,
}

/// 服务费三级费率计算（骑手 MVP 完成入账口径，我方新增规则层；费率字段为原版有）。
///
/// - 总开关关（`fee_enabled=false`）→ 0；
/// - 总部统一配置（`store_assign=false`，平台 `storeAssign*ServiceFee=0`）：
///   货款总额 × 平台默认费率% / 100（`default*ServiceFeeRate`，百分比口径与本仓
///   分销佣金 rate/100 一致），四舍五入 2 位；
/// - 门店独立配置（`store_assign=true`）：逐行取 SKU 级 → 商品级 → 门店默认
///   （`shop_*_config.default_service_fee_*`，我方新增第二级）；固定 = 值 × 数量，
///   比例 = 行金额 × 值% / 100（逐行四舍五入 2 位后求和）。
#[must_use]
pub fn compute_service_fee(
    items: &[FeeItem],
    shop_default: (i32, rust_decimal::Decimal),
    store_assign: bool,
    fee_enabled: bool,
    platform_rate: rust_decimal::Decimal,
) -> rust_decimal::Decimal {
    use rust_decimal::{Decimal, RoundingStrategy};
    if !fee_enabled {
        return Decimal::ZERO;
    }
    if !store_assign {
        let goods: Decimal = items.iter().map(|i| i.price * Decimal::from(i.quantity)).sum();
        return (goods * platform_rate / Decimal::from(100))
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    }
    let mut total = Decimal::ZERO;
    for item in items {
        let Some((fee_type, fee_value)) = item
            .sku_fee
            .or(item.product_fee)
            .or((shop_default.1 > Decimal::ZERO).then_some(shop_default))
        else {
            continue;
        };
        let line = match fee_type {
            0 => fee_value * Decimal::from(item.quantity),
            1 => (item.price * Decimal::from(item.quantity) * fee_value / Decimal::from(100))
                .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
            _ => Decimal::ZERO,
        };
        total += line;
    }
    total.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

#[cfg(test)]
mod fee_tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn item(price: &str, qty: i32, product: Option<(i32, &str)>, sku: Option<(i32, &str)>) -> FeeItem {
        FeeItem {
            price: d(price),
            quantity: qty,
            product_fee: product.map(|(t, v)| (t, d(v))),
            sku_fee: sku.map(|(t, v)| (t, d(v))),
        }
    }

    #[test]
    fn fee_switch_off_yields_zero() {
        let items = [item("100", 1, Some((0, "5")), None)];
        assert_eq!(
            compute_service_fee(&items, (0, d("0")), true, false, d("10")),
            Decimal::ZERO
        );
    }

    #[test]
    fn platform_unified_rate_on_goods_total() {
        // 总部统一：货款 250 × 4% = 10.00（商品级费率被忽略）
        let items = [
            item("100", 2, Some((0, "5")), None),
            item("50", 1, None, None),
        ];
        assert_eq!(
            compute_service_fee(&items, (0, d("3")), false, true, d("4")),
            d("10.00")
        );
    }

    #[test]
    fn store_assign_fixed_and_ratio_lines() {
        // 行1 固定 3 元/件 × 2 = 6；行2 比例 200 × 5% = 10 → 16.00
        let items = [
            item("80", 2, Some((0, "3")), None),
            item("200", 1, Some((1, "5")), None),
        ];
        assert_eq!(
            compute_service_fee(&items, (0, d("0")), true, true, d("0")),
            d("16.00")
        );
    }

    #[test]
    fn sku_fee_wins_over_product_fee() {
        let items = [item("100", 1, Some((0, "3")), Some((0, "7")))];
        assert_eq!(
            compute_service_fee(&items, (0, d("0")), true, true, d("0")),
            d("7.00")
        );
    }

    #[test]
    fn shop_default_is_fallback_tier() {
        // 商品无费率 → 门店默认 固定 2.5/件 × 3 = 7.50
        let items = [item("10", 3, None, None)];
        assert_eq!(
            compute_service_fee(&items, (0, d("2.5")), true, true, d("0")),
            d("7.50")
        );
    }

    #[test]
    fn no_fee_anywhere_yields_zero() {
        let items = [item("10", 3, None, None)];
        assert_eq!(
            compute_service_fee(&items, (0, d("0")), true, true, d("0")),
            Decimal::ZERO
        );
    }

    #[test]
    fn ratio_rounds_per_line() {
        // 33.33 × 3 × 3% = 2.9997 → 逐行四舍五入 3.00
        let items = [item("33.33", 3, Some((1, "3")), None)];
        assert_eq!(
            compute_service_fee(&items, (0, d("0")), true, true, d("0")),
            d("3.00")
        );
    }
}
