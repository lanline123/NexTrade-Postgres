//! 代客下单接单大厅仓储（trade::proxy_hall，我方新增，二期）。
//!
//! 接单状态机落在 `proxy_order_ext` 的 hall_* 列上（迁移 028），
//! 本仓储只做状态机的原子迁移与池查询：抢单/派单/拒单回池/备货
//! 全部走「带前置状态的条件 UPDATE」，以 rows_affected 判定成败，
//! 并发抢单天然只有一个赢家。业务校验（体系归属、开关、权限）在
//! 服务层。SQL 约定与同域仓储一致：`?` 占位 + `to_pg_placeholders`
//! 转换、单行字符串字面量。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 大厅订单行（池列表 / 我的接单共用投影）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct HallOrderRow {
    pub order_id: i32,
    pub order_sn: String,
    pub order_status: i32,
    pub pay_status: i32,
    pub proxy_shop_id: i32,
    pub proxy_shop_title: Option<String>,
    pub fulfill_shop_id: i32,
    pub fulfill_shop_title: Option<String>,
    pub consignee: String,
    pub address: String,
    pub mobile: String,
    pub total_amount: Decimal,
    pub item_count: i64,
    pub first_product_name: Option<String>,
    pub hall_status: i32,
    pub hall_deadline: i32,
    pub accept_time: Option<i32>,
    pub assign_type: i32,
    pub reassign_count: i32,
    pub add_time: i32,
}

/// ext 接单字段行（动作前置校验用）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct HallExtRow {
    pub order_id: i32,
    pub proxy_shop_id: i32,
    pub fulfill_shop_id: i32,
    pub hall_status: i32,
    pub hall_deadline: i32,
}

const HALL_SELECT: &str = "SELECT e.order_id, o.order_sn, o.order_status, o.pay_status, e.proxy_shop_id, (SELECT shop_title FROM shop WHERE shop_id = e.proxy_shop_id) AS proxy_shop_title, e.fulfill_shop_id, (SELECT shop_title FROM shop WHERE shop_id = e.fulfill_shop_id) AS fulfill_shop_title, o.consignee, o.address, o.mobile, o.total_amount, (SELECT COALESCE(SUM(quantity), 0) FROM order_item WHERE order_id = e.order_id) AS item_count, (SELECT product_name FROM order_item WHERE order_id = e.order_id ORDER BY item_id ASC LIMIT 1) AS first_product_name, e.hall_status, e.hall_deadline, e.accept_time, e.assign_type, e.reassign_count, e.add_time FROM proxy_order_ext e JOIN \"order\" o ON o.order_id = e.order_id";

#[derive(Clone)]
pub struct ProxyHallRepository {
    pool: PgPool,
}

impl ProxyHallRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 待接单池：下单店在给定体系店铺集内、订单未取消、查看者未拒过
    /// 该单。按截止时间临近优先（不限时置后）、下单时间升序。
    pub async fn pool_list(
        &self,
        family_shop_ids: &[i32],
        viewer_shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<HallOrderRow>, i64)> {
        if family_shop_ids.is_empty() {
            return Ok((Vec::new(), 0));
        }
        let in_list = vec!["?"; family_shop_ids.len()].join(", ");
        let where_sql = format!(
            " WHERE e.hall_status = 1 AND e.proxy_shop_id IN ({in_list}) AND o.order_status NOT IN (3, 4) AND NOT EXISTS (SELECT 1 FROM proxy_hall_reject r WHERE r.order_id = e.order_id AND r.shop_id = ?)"
        );
        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM proxy_order_ext e JOIN \"order\" o ON o.order_id = e.order_id{where_sql}"
        ));
        let mut cq = sqlx::query_scalar::<_, i64>(&count_sql);
        for id in family_shop_ids {
            cq = cq.bind(id);
        }
        let total = cq.bind(viewer_shop_id).fetch_one(&self.pool).await.map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "{HALL_SELECT}{where_sql} ORDER BY (CASE WHEN e.hall_deadline > 0 THEN 0 ELSE 1 END) ASC, e.hall_deadline ASC, e.add_time ASC LIMIT ? OFFSET ?"
        ));
        let mut rq = sqlx::query_as::<_, HallOrderRow>(&rows_sql);
        for id in family_shop_ids {
            rq = rq.bind(id);
        }
        let rows = rq
            .bind(viewer_shop_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total))
    }

    /// 我的接单：当前店铺为履约店且状态为已接单/已备货。
    pub async fn mine_list(
        &self,
        shop_id: i32,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<HallOrderRow>, i64)> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxy_order_ext WHERE fulfill_shop_id = $1 AND hall_status IN (2, 3)")
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "{HALL_SELECT} WHERE e.fulfill_shop_id = ? AND e.hall_status IN (2, 3) ORDER BY e.accept_time DESC, e.add_time DESC LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, HallOrderRow>(&rows_sql)
            .bind(shop_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((rows, total))
    }

    /// 读 ext 接单字段（动作前置校验）。
    pub async fn find_hall_ext(&self, order_id: i32) -> Result<Option<HallExtRow>> {
        let row = sqlx::query_as::<_, HallExtRow>("SELECT order_id, proxy_shop_id, fulfill_shop_id, hall_status, hall_deadline FROM proxy_order_ext WHERE order_id = $1 LIMIT 1")
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row)
    }

    /// 原子接单（抢单 assign_type=1 / 派单 assign_type=2 / 兜底改派
    /// assign_type=3 共用）：仅当仍在池中（hall_status=1）才迁移，
    /// 返回是否抢到。并发下只有一个调用成功。
    pub async fn claim(
        &self,
        order_id: i32,
        fulfill_shop_id: i32,
        admin_id: i32,
        assign_type: i32,
        now: i32,
    ) -> Result<bool> {
        let affected = sqlx::query("UPDATE proxy_order_ext SET hall_status = 2, fulfill_shop_id = $1, accept_time = $2, accept_admin_id = $3, assign_type = $4 WHERE order_id = $5 AND hall_status = 1")
            .bind(fulfill_shop_id)
            .bind(now)
            .bind(admin_id)
            .bind(assign_type)
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 拒单回池：仅当前履约店可退，状态 2 → 1、履约清空、改派计数
    /// +1、截止时间按新一轮接单窗口刷新；同时落拒单记录（该店此后
    /// 不再在池中看到此单）。返回是否退池成功。
    pub async fn release_to_pool(
        &self,
        order_id: i32,
        shop_id: i32,
        admin_id: i32,
        reason: &str,
        new_deadline: i32,
        now: i32,
    ) -> Result<bool> {
        let affected = sqlx::query("UPDATE proxy_order_ext SET hall_status = 1, fulfill_shop_id = 0, accept_time = NULL, accept_admin_id = 0, assign_type = 0, reassign_count = reassign_count + 1, hall_deadline = $1 WHERE order_id = $2 AND hall_status = 2 AND fulfill_shop_id = $3")
            .bind(new_deadline)
            .bind(order_id)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        if affected == 0 {
            return Ok(false);
        }
        sqlx::query("INSERT INTO proxy_hall_reject (order_id, shop_id, admin_id, reason, add_time) VALUES ($1, $2, $3, $4, $5)")
            .bind(order_id)
            .bind(shop_id)
            .bind(admin_id)
            .bind(reason)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(true)
    }

    /// 备货完成标记：状态 2 → 3，仅当前履约店可标。
    pub async fn mark_ready(&self, order_id: i32, shop_id: i32, now: i32) -> Result<bool> {
        let affected = sqlx::query("UPDATE proxy_order_ext SET hall_status = 3, ready_time = $1 WHERE order_id = $2 AND hall_status = 2 AND fulfill_shop_id = $3")
            .bind(now)
            .bind(order_id)
            .bind(shop_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected > 0)
    }

    /// 超时待接单扫描（定时任务）：在池中且截止已过。
    pub async fn list_expired_pending(&self, now: i32, limit: i64) -> Result<Vec<HallExtRow>> {
        let rows = sqlx::query_as::<_, HallExtRow>("SELECT order_id, proxy_shop_id, fulfill_shop_id, hall_status, hall_deadline FROM proxy_order_ext WHERE hall_status = 1 AND hall_deadline > 0 AND hall_deadline <= $1 ORDER BY hall_deadline ASC LIMIT $2")
            .bind(now)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    /// 兜底关闭不限时：超时单选择继续留池时清掉截止（只提醒一次，
    /// 不再反复触发超时处置）。
    pub async fn clear_deadline(&self, order_id: i32) -> Result<()> {
        sqlx::query("UPDATE proxy_order_ext SET hall_deadline = 0 WHERE order_id = $1 AND hall_status = 1")
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 订单取消联动关闭：池中/已接单的大厅单，订单已取消/无效时
    /// 置 hall_status=4。返回关闭行数（定时任务兜底扫描）。
    pub async fn close_cancelled(&self) -> Result<u64> {
        let affected = sqlx::query("UPDATE proxy_order_ext e SET hall_status = 4 WHERE e.hall_status IN (1, 2) AND EXISTS (SELECT 1 FROM \"order\" o WHERE o.order_id = e.order_id AND o.order_status IN (3, 4))")
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }
}
