//! 充值赠品领取记录仓储（marketing::recharge_gift）【我方新增 2026-11-21】。
//! 表 `recharge_gift_record`（20 列，主键 record_id，recharge_order_id 唯一）。
//!
//! 发放（充值单服务在充值入账事务内调用 insert_in_tx）、领取（服务层事务内
//! 扣赠品池库存 + 建履约单后 update_in_tx 置已领取）、撤销联动作废与后台
//! 处置都在本仓储完成；事务由服务层开启并传入。

use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use nextrade_core::entities::RechargeGiftRecord;
use nextrade_core::models::marketing::recharge_gift::{
    limit_offset, RechargeGiftListQuery, STATUS_ABNORMAL, STATUS_PENDING, STATUS_VOID,
};
use nextrade_core::{Error, Result};

const SELECT: &str = "SELECT record_id, recharge_order_id, recharge_id, user_id, gift_id, gift_name, product_id, sku_id, shop_id, quantity, status, order_id, address_id, consignee, mobile, address_text, admin_note, add_time, claim_time, update_time FROM recharge_gift_record";

/// 充值赠品领取记录仓储。
#[derive(Clone)]
pub struct RechargeGiftRepository {
    pool: PgPool,
}

impl RechargeGiftRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 开启事务（发放/领取/处置的原子边界由服务层持有）。
    pub async fn begin(&self) -> Result<Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    /// 发放：事务内插入待领取记录；recharge_order_id 唯一冲突时静默跳过
    /// （重复置已支付不重复发放），返回是否真正插入。
    pub async fn insert_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: &RechargeGiftRecord,
    ) -> Result<bool> {
        let res = sqlx::query(
"INSERT INTO recharge_gift_record (recharge_order_id, recharge_id, user_id, gift_id, gift_name, product_id, sku_id, shop_id, quantity, status, add_time, update_time) \
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11) \
ON CONFLICT (recharge_order_id) DO NOTHING",
)
.bind(record.recharge_order_id)
.bind(record.recharge_id)
.bind(record.user_id)
.bind(record.gift_id)
.bind(&record.gift_name)
.bind(record.product_id)
.bind(record.sku_id)
.bind(record.shop_id)
.bind(record.quantity)
.bind(record.status)
.bind(record.add_time)
.execute(&mut **tx)
.await
.map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 事务内锁定读取（领取/处置防并发重复操作）。
    pub async fn find_by_id_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record_id: i64,
    ) -> Result<Option<RechargeGiftRecord>> {
        sqlx::query_as::<_, RechargeGiftRecord>(&format!(
            "{SELECT} WHERE record_id = $1 FOR UPDATE"
        ))
        .bind(record_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)
    }

    pub async fn find_by_id(&self, record_id: i64) -> Result<Option<RechargeGiftRecord>> {
        sqlx::query_as::<_, RechargeGiftRecord>(&format!("{SELECT} WHERE record_id = $1"))
            .bind(record_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 事务内全字段回写（领取落单、异常标记、作废、恢复共用）。
    pub async fn update_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        record: &RechargeGiftRecord,
    ) -> Result<u64> {
        let res = sqlx::query(
"UPDATE recharge_gift_record SET status = $1, order_id = $2, address_id = $3, consignee = $4, mobile = $5, address_text = $6, admin_note = $7, claim_time = $8, update_time = $9 WHERE record_id = $10",
)
.bind(record.status)
.bind(record.order_id)
.bind(record.address_id)
.bind(&record.consignee)
.bind(&record.mobile)
.bind(&record.address_text)
.bind(&record.admin_note)
.bind(record.claim_time)
.bind(record.update_time)
.bind(record.record_id)
.execute(&mut **tx)
.await
.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 充值撤销联动：事务内把某充值单下未领取/异常的记录批量作废
    /// （已领取不动——履约单已生成，处置口径见服务层注释）。
    pub async fn void_open_by_recharge_order_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recharge_order_id: i32,
        note: &str,
        now: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE recharge_gift_record SET status = $1, admin_note = $2, update_time = $3 \
WHERE recharge_order_id = $4 AND status IN ($5, $6)",
        )
        .bind(STATUS_VOID)
        .bind(note)
        .bind(now)
        .bind(recharge_order_id)
        .bind(STATUS_PENDING)
        .bind(STATUS_ABNORMAL)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 列表（后台全量 / 会员端由服务层强制 user_id）。
    pub async fn paginate(
        &self,
        q: &RechargeGiftListQuery,
    ) -> Result<(i64, Vec<RechargeGiftRecord>)> {
        let (limit, offset) = limit_offset(q.page, q.size);
        let mut cq: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM recharge_gift_record WHERE 1 = 1");
        Self::push_filters(&mut cq, q);
        let total: (i64,) = cq
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let mut rq: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!("{SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut rq, q);
        rq.push(format!(
            " ORDER BY record_id DESC LIMIT {limit} OFFSET {offset}"
        ));
        let rows: Vec<RechargeGiftRecord> = rq
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, q: &RechargeGiftListQuery) {
        if let Some(user_id) = q.user_id {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
    }

    /// 赠品图片装配：按 product_id 批量取 product.pic_thumb（会员端/后台展示用）。
    pub async fn product_pics(&self, product_ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(i32, Option<String>)> =
            sqlx::query_as("SELECT product_id, pic_thumb FROM product WHERE product_id = ANY($1)")
                .bind(product_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, pic)| (id, pic.unwrap_or_default()))
            .collect())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::recharge_gift::COLUMNS;

    #[test]
    fn select_matches_columns() {
        let selected = SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, COLUMNS.join(", "));
    }

    #[test]
    fn static_guards_for_sql() {
        let src = include_str!("recharge_gift.rs");
        assert!(src.contains("ON CONFLICT (recharge_order_id) DO NOTHING"));
        assert!(src.contains("FOR UPDATE"));
    }
}
