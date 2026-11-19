//! 广告投流域仓储（merchant::ad_campaign）。
//!
//! 涉及表：
//! - `ad_campaign`（投放单主表，迁移 `rust/deploy/pg/2026-11-18_ad_campaign.sql`）
//! - `shop`（扣款时行锁读余额）/ `shop_account_log`（扣款流水，event_key 防重）
//! - 个性化信号（只读）：`cart` / `collect_product` / `collect_shop` /
//!   `"order"` + `order_item` + `product` / `"user".history_product_ids`
//!
//! 事务口径照 `deposit_recharge`：[`AdCampaignRepository::begin`] + `*_in_tx`，
//! 服务层把「建单 + 扣款 + 流水」收进同一事务。

use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres};

use nextrade_core::models::merchant::ad_campaign::AdCampaign;
use nextrade_core::{Error, Result};

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

/// 投流域仓储。
#[derive(Clone)]
pub struct AdCampaignRepository {
    pool: PgPool,
}

impl AdCampaignRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 开启事务（建单扣款 / 状态变更编排用）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 投放单 CRUD —————————————————————

    /// 插入投放单（事务内），返回新 id。
    pub async fn insert_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        c: &AdCampaign,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO ad_campaign \
             (target_type, shop_id, product_id, position, is_pin, pin_sort, \
              start_time, end_time, days, unit_price, amount, weight, \
              pay_status, pay_channel, pay_time, account_log_id, \
              audit_status, audit_remark, audit_admin_id, audit_time, \
              status, add_time, update_time) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23) \
             RETURNING id",
        )
        .bind(c.target_type)
        .bind(c.shop_id)
        .bind(c.product_id)
        .bind(&c.position)
        .bind(c.is_pin)
        .bind(c.pin_sort)
        .bind(c.start_time)
        .bind(c.end_time)
        .bind(c.days)
        .bind(c.unit_price)
        .bind(c.amount)
        .bind(c.weight)
        .bind(c.pay_status)
        .bind(&c.pay_channel)
        .bind(c.pay_time)
        .bind(c.account_log_id)
        .bind(c.audit_status)
        .bind(&c.audit_remark)
        .bind(c.audit_admin_id)
        .bind(c.audit_time)
        .bind(c.status)
        .bind(c.add_time)
        .bind(c.update_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 按 id 查投放单。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<AdCampaign>> {
        sqlx::query_as::<_, AdCampaign>("SELECT * FROM ad_campaign WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 id 查投放单（事务内加行锁，状态变更防并发）。
    pub async fn find_by_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
    ) -> Result<Option<AdCampaign>> {
        sqlx::query_as::<_, AdCampaign>("SELECT * FROM ad_campaign WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 更新投放单的可变字段（支付/审核/状态/流水关联），事务内。
    pub async fn update_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        c: &AdCampaign,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE ad_campaign SET pay_status=$2, pay_channel=$3, pay_time=$4, \
             account_log_id=$5, audit_status=$6, audit_remark=$7, audit_admin_id=$8, \
             audit_time=$9, status=$10, weight=$11, update_time=$12 WHERE id=$1",
        )
        .bind(c.id)
        .bind(c.pay_status)
        .bind(&c.pay_channel)
        .bind(c.pay_time)
        .bind(c.account_log_id)
        .bind(c.audit_status)
        .bind(&c.audit_remark)
        .bind(c.audit_admin_id)
        .bind(c.audit_time)
        .bind(c.status)
        .bind(c.weight)
        .bind(c.update_time)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 分页列表：`shop_id` 有值 = 商家「我的投放」；`None` = 平台全量（可按审核状态过滤）。
    pub async fn paginate(
        &self,
        shop_id: Option<i32>,
        audit_status: Option<i32>,
        status: Option<i32>,
        position: Option<&str>,
        page: &nextrade_core::types::PageQuery,
    ) -> Result<(u64, Vec<AdCampaign>)> {
        let (limit, offset) = page.limit_offset();
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ad_campaign WHERE \
             ($1::int IS NULL OR shop_id = $1) AND \
             ($2::int IS NULL OR audit_status = $2) AND \
             ($3::int IS NULL OR status = $3) AND \
             ($4::text IS NULL OR position = $4)",
        )
        .bind(shop_id)
        .bind(audit_status)
        .bind(status)
        .bind(position)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let rows = sqlx::query_as::<_, AdCampaign>(
            "SELECT * FROM ad_campaign WHERE \
             ($1::int IS NULL OR shop_id = $1) AND \
             ($2::int IS NULL OR audit_status = $2) AND \
             ($3::int IS NULL OR status = $3) AND \
             ($4::text IS NULL OR position = $4) \
             ORDER BY id DESC LIMIT $5 OFFSET $6",
        )
        .bind(shop_id)
        .bind(audit_status)
        .bind(status)
        .bind(position)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量查店铺名（列表 VO 回填 `shopTitle` 用，一次 IN 查询避免 N+1）。
    pub async fn shop_titles(&self, shop_ids: &[i32]) -> Result<Vec<(i32, Option<String>)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=shop_ids.len()).map(|i| format!("${i}")).collect();
        let sql = format!(
            "SELECT shop_id, shop_title FROM shop WHERE shop_id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量查商品名（列表 VO 回填 `productName` 用，一次 IN 查询避免 N+1）。
    pub async fn product_names(&self, product_ids: &[i32]) -> Result<Vec<(i32, Option<String>)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=product_ids.len()).map(|i| format!("${i}")).collect();
        let sql = format!(
            "SELECT product_id, product_name FROM product WHERE product_id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某投放位当前可展示的投放单（已付+已审+未暂停/下架+在期内）。
    /// 置顶段与推广段由服务层按 `is_pin` / `weight` 再分段排序。
    pub async fn live_campaigns(&self, position: &str, now: i32) -> Result<Vec<AdCampaign>> {
        sqlx::query_as::<_, AdCampaign>(
            "SELECT * FROM ad_campaign WHERE position = $1 AND pay_status = 1 \
             AND audit_status = 1 AND status IN (0, 1) \
             AND start_time <= $2 AND end_time > $2 \
             ORDER BY is_pin DESC, pin_sort ASC, weight DESC, id ASC",
        )
        .bind(position)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 店铺余额扣款（事务内） —————————————————————

    /// 事务内锁定店铺行并读余额：返回 (shop_money, frozen_money)，店铺不存在为 None。
    pub async fn lock_shop_money_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        shop_id: i32,
    ) -> Result<Option<(Decimal, Decimal)>> {
        let row: Option<(Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
            "SELECT shop_money, frozen_money FROM shop WHERE shop_id = $1 FOR UPDATE",
        )
        .bind(shop_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(m, f)| (m.unwrap_or(Decimal::ZERO), f.unwrap_or(Decimal::ZERO))))
    }

    /// 事务内扣减店铺余额（调用方已校验余额充足；WHERE 再兜底防扣成负数）。
    pub async fn debit_shop_money_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        shop_id: i32,
        amount: Decimal,
    ) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE shop SET shop_money = shop_money - $1 WHERE shop_id = $2 AND shop_money >= $1",
        )
        .bind(amount)
        .bind(shop_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(n.rows_affected() == 1)
    }

    /// 事务内写店铺资金流水（照 settlement 的 shop_account_log 口径；
    /// `type` 取新值 `'ad'` 表示投流扣款，`event_key = ad_campaign:{id}` 防重）。
    /// 返回流水 id。
    pub async fn insert_account_log_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        shop_id: i32,
        old_money: Decimal,
        frozen: Decimal,
        amount: Decimal,
        campaign_id: i32,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO shop_account_log \
             (add_time, shop_money, frozen_money, \"type\", remarks, new_shop_money, \
              new_frozen_money, shop_id, is_new, balance_type, change_amount, event_key) \
             VALUES ($1, $2, $3, 'ad', $4, $5, $6, $7, 1, 2, $8, $9) \
             RETURNING shop_account_log_id",
        )
        .bind(now)
        .bind(old_money)
        .bind(frozen)
        .bind(format!("店铺投流扣款（投放单 #{campaign_id}）"))
        .bind(old_money - amount)
        .bind(frozen)
        .bind(shop_id)
        .bind(-amount)
        .bind(format!("ad_campaign:{campaign_id}"))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 该投放单是否已扣款（event_key 幂等检查）。
    pub async fn account_log_exists_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        campaign_id: i32,
    ) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT shop_account_log_id FROM shop_account_log WHERE event_key = $1 LIMIT 1",
        )
        .bind(format!("ad_campaign:{campaign_id}"))
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    // ————————————————————— 对象校验（只读） —————————————————————

    /// 店铺是否在营（status = 1）。
    pub async fn shop_is_open(&self, shop_id: i32) -> Result<bool> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM shop WHERE shop_id = $1 AND status = 1")
                .bind(shop_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 商品是否属本店且在售（未删 + 上架）。
    pub async fn product_is_sellable_by_shop(&self, product_id: i32, shop_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE product_id = $1 AND shop_id = $2 \
             AND is_delete = 0 AND product_status = 1",
        )
        .bind(product_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    // ————————————————————— 个性化信号（只读，按 shop_id 聚合） —————————————————————

    /// 已购：近 `days` 天已支付订单按店铺计数（JOIN product 取 shop_id，
    /// order_item 无店铺列时以商品归属为准；与 cohort 口径一致 is_del=0 AND pay_status>0）。
    pub async fn purchased_orders_by_shop(
        &self,
        user_id: i32,
        days: i32,
    ) -> Result<Vec<(i32, i64)>> {
        sqlx::query_as(
            "SELECT p.shop_id, COUNT(DISTINCT o.order_id)::bigint FROM \"order\" o \
             JOIN order_item i ON i.order_id = o.order_id \
             JOIN product p ON p.product_id = i.product_id \
             WHERE o.user_id = $1 AND o.is_del = 0 AND o.pay_status > 0 \
             AND o.add_time >= (EXTRACT(EPOCH FROM NOW())::bigint - $2::bigint * 86400) \
             GROUP BY p.shop_id",
        )
        .bind(user_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 收藏店铺集合。
    pub async fn collected_shops(&self, user_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM collect_shop WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 购物车按店铺计数（cart 自带 shop_id）。
    pub async fn cart_items_by_shop(&self, user_id: i32) -> Result<Vec<(i32, i64)>> {
        sqlx::query_as(
            "SELECT shop_id, COUNT(*)::bigint FROM cart WHERE user_id = $1 GROUP BY shop_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 收藏商品按店铺计数（collect_product 自带 shop_id）。
    pub async fn collected_products_by_shop(&self, user_id: i32) -> Result<Vec<(i32, i64)>> {
        sqlx::query_as(
            "SELECT shop_id, COUNT(*)::bigint FROM collect_product WHERE user_id = $1 \
             GROUP BY shop_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 用户行为涉及的分类集合（已购 + 购物车 + 收藏商品的 category_id）。
    pub async fn user_category_ids(&self, user_id: i32) -> Result<Vec<i32>> {
        sqlx::query_as(
            "SELECT DISTINCT category_id FROM ( \
               SELECT p.category_id FROM \"order\" o \
                 JOIN order_item i ON i.order_id = o.order_id \
                 JOIN product p ON p.product_id = i.product_id \
                 WHERE o.user_id = $1 AND o.is_del = 0 AND o.pay_status > 0 \
               UNION \
               SELECT p.category_id FROM cart c \
                 JOIN product p ON p.product_id = c.product_id WHERE c.user_id = $1 \
               UNION \
               SELECT p.category_id FROM collect_product cp \
                 JOIN product p ON p.product_id = cp.product_id WHERE cp.user_id = $1 \
             ) t WHERE category_id > 0",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
        .map(|rows: Vec<(i32,)>| rows.into_iter().map(|(id,)| id).collect())
    }

    /// 浏览记录商品 ID（复用 recommend 仓储的历史解析口径：JSON 数组，兼容逗号分隔）。
    pub async fn history_product_ids(&self, user_id: i32, limit: u32) -> Result<Vec<i32>> {
        self.repos_history_ids(user_id, limit).await
    }

    async fn repos_history_ids(&self, user_id: i32, limit: u32) -> Result<Vec<i32>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT history_product_ids FROM \"user\" WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        let Some((Some(raw),)) = row else {
            return Ok(Vec::new());
        };
        let raw = raw.trim();
        if raw.is_empty() {
            return Ok(Vec::new());
        }
        let mut ids: Vec<i32> = serde_json::from_str::<Vec<i32>>(raw).unwrap_or_else(|_| {
            raw.split(',')
                .filter_map(|s| s.trim().trim_matches('"').parse::<i32>().ok())
                .collect()
        });
        ids.truncate(limit as usize);
        Ok(ids)
    }

    /// 一批商品的 (product_id, shop_id, category_id) 映射（浏览信号解析用）。
    pub async fn product_shop_categories(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<(i32, i32, i32)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=product_ids.len()).map(|i| format!("${i}")).collect();
        let sql = format!(
            "SELECT product_id, shop_id, category_id FROM product WHERE product_id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query_as::<_, (i32, i32, i32)>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 各店铺在售商品的分类集合（兴趣分类命中计算用）。
    pub async fn shop_category_ids(&self, shop_ids: &[i32]) -> Result<Vec<(i32, i32)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=shop_ids.len()).map(|i| format!("${i}")).collect();
        let sql = format!(
            "SELECT DISTINCT shop_id, category_id FROM product \
             WHERE is_delete = 0 AND product_status = 1 AND category_id > 0 \
             AND shop_id IN ({})",
            placeholders.join(",")
        );
        let mut q = sqlx::query_as::<_, (i32, i32)>(&sql);
        for id in shop_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 相似用户协同（按 shop_id 聚合）：复用 recommend.cohort_buys 的 CTE 写法——
    /// 与当前用户买过相同分类的其他用户，近 `days` 天买过的店铺及购买人数。
    pub async fn cohort_buyers_by_shop(
        &self,
        user_id: i32,
        days: i32,
    ) -> Result<Vec<(i32, i64)>> {
        sqlx::query_as(
            "WITH my_cats AS ( \
               SELECT DISTINCT p.category_id AS cid FROM order_item i \
               JOIN \"order\" o ON o.order_id = i.order_id \
               JOIN product p ON p.product_id = i.product_id \
               WHERE o.user_id = $1 AND o.is_del = 0 AND o.pay_status > 0 AND p.category_id > 0 \
             ), cohort AS ( \
               SELECT DISTINCT o.user_id AS uid FROM order_item i \
               JOIN \"order\" o ON o.order_id = i.order_id \
               JOIN product p ON p.product_id = i.product_id \
               WHERE o.is_del = 0 AND o.pay_status > 0 AND o.user_id <> $1 \
               AND o.add_time >= (EXTRACT(EPOCH FROM NOW())::bigint - $2::bigint * 86400) \
               AND p.category_id IN (SELECT cid FROM my_cats) \
             ) \
             SELECT pr.shop_id, COUNT(DISTINCT o.user_id)::bigint FROM order_item i \
             JOIN \"order\" o ON o.order_id = i.order_id \
             JOIN product pr ON pr.product_id = i.product_id \
             WHERE o.user_id IN (SELECT uid FROM cohort) \
             AND o.is_del = 0 AND o.pay_status > 0 \
             AND o.add_time >= (EXTRACT(EPOCH FROM NOW())::bigint - $2::bigint * 86400) \
             GROUP BY pr.shop_id",
        )
        .bind(user_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }
}
