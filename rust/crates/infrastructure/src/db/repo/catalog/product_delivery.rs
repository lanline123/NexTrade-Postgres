//! 商品交付附件与交付令牌仓储（catalog::product_delivery）。
//! 表 `product_delivery_file` / `product_delivery_token`（迁移 007，我方新增）。
//!
//! 令牌核销走单条原子 UPDATE（有效期内且次数未用完才 +1 并 RETURNING），
//! 并发重复点击不会超发次数。时间戳秒级 INT、文件大小 BIGINT，
//! 元组字段类型与列类型严格对齐。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 附件行：(id, product_id, shop_id, file_name, file_key, file_ext, file_size, sort, add_time)
pub type DeliveryFileRow = (i32, i32, i32, String, String, String, i64, i32, i32);

/// 令牌行：(id, token, order_id, user_id, file_id, product_id, max_downloads, download_count, expires_at, add_time)
pub type DeliveryTokenRow = (i32, String, i32, i32, i32, i32, i32, i32, i32, i32);

/// 订单交付上下文行：(order_id, user_id, pay_status, shipping_status)
pub type DeliveryOrderRow = (i32, i32, i32, i32);

#[derive(Clone)]
pub struct ProductDeliveryRepository {
    pool: PgPool,
}

impl ProductDeliveryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const FILE_COLUMNS: &'static str =
        "id, product_id, shop_id, file_name, file_key, file_ext, file_size, sort, add_time";

    const TOKEN_COLUMNS: &'static str =
        "id, token, order_id, user_id, file_id, product_id, max_downloads, download_count, expires_at, add_time";

    /// 某商品全部未删除附件（排序、id 升序）。
    pub async fn list_by_product(&self, product_id: i32) -> Result<Vec<DeliveryFileRow>> {
        let sql = format!(
            "SELECT {} FROM product_delivery_file WHERE product_id = $1 AND is_del = 0 ORDER BY sort ASC, id ASC",
            Self::FILE_COLUMNS
        );
        sqlx::query_as::<_, DeliveryFileRow>(&sql)
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 多商品批量取附件（订单详情逐项注入用，避免 N+1）。
    pub async fn list_by_products(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<DeliveryFileRow>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {} FROM product_delivery_file WHERE product_id = ANY($1) AND is_del = 0 ORDER BY sort ASC, id ASC",
            Self::FILE_COLUMNS
        );
        sqlx::query_as::<_, DeliveryFileRow>(&sql)
            .bind(product_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 单个附件（含已软删判断由调用方看 is_del 列外的行存在性——本查询只取未删行）。
    pub async fn find_file(&self, file_id: i32) -> Result<Option<DeliveryFileRow>> {
        let sql = format!(
            "SELECT {} FROM product_delivery_file WHERE id = $1 AND is_del = 0",
            Self::FILE_COLUMNS
        );
        sqlx::query_as::<_, DeliveryFileRow>(&sql)
            .bind(file_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 某商品未删除附件数（每商品上限校验）。
    pub async fn count_by_product(&self, product_id: i32) -> Result<i64> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM product_delivery_file WHERE product_id = $1 AND is_del = 0",
        )
        .bind(product_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增附件元数据，返回新 id。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_file(
        &self,
        product_id: i32,
        shop_id: i32,
        file_name: &str,
        file_key: &str,
        file_ext: &str,
        file_size: i64,
        sort: i32,
        now: i32,
    ) -> Result<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO product_delivery_file \
             (product_id, shop_id, file_name, file_key, file_ext, file_size, sort, is_del, add_time, update_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $8) RETURNING id",
        )
        .bind(product_id)
        .bind(shop_id)
        .bind(file_name)
        .bind(file_key)
        .bind(file_ext)
        .bind(file_size)
        .bind(sort)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 软删附件（限定本商品，防跨商品误删），返回是否命中。
    pub async fn soft_delete_file(&self, file_id: i32, product_id: i32, now: i32) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE product_delivery_file SET is_del = 1, update_time = $3 \
             WHERE id = $1 AND product_id = $2 AND is_del = 0",
        )
        .bind(file_id)
        .bind(product_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 签发令牌（绑订单 + 用户 + 文件）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_token(
        &self,
        token: &str,
        order_id: i32,
        user_id: i32,
        file_id: i32,
        product_id: i32,
        max_downloads: i32,
        expires_at: i32,
        now: i32,
    ) -> Result<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO product_delivery_token \
             (token, order_id, user_id, file_id, product_id, max_downloads, download_count, expires_at, add_time, last_download_time) \
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, 0) RETURNING id",
        )
        .bind(token)
        .bind(order_id)
        .bind(user_id)
        .bind(file_id)
        .bind(product_id)
        .bind(max_downloads)
        .bind(expires_at)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按令牌字符串查令牌行（报错文案分流用；核销请走 `consume_token`）。
    pub async fn find_token(&self, token: &str) -> Result<Option<DeliveryTokenRow>> {
        let sql = format!(
            "SELECT {} FROM product_delivery_token WHERE token = $1",
            Self::TOKEN_COLUMNS
        );
        sqlx::query_as::<_, DeliveryTokenRow>(&sql)
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 原子核销：有效期内且次数未用完才计数 +1 并返回核销后的行；
    /// 过期/耗尽/不存在一律返回 `None`（并发安全，不会超发）。
    pub async fn consume_token(
        &self,
        token: &str,
        now: i32,
    ) -> Result<Option<DeliveryTokenRow>> {
        let sql = format!(
            "UPDATE product_delivery_token \
             SET download_count = download_count + 1, last_download_time = $2 \
             WHERE token = $1 AND expires_at > $2 AND download_count < max_downloads \
             RETURNING {}",
            Self::TOKEN_COLUMNS
        );
        sqlx::query_as::<_, DeliveryTokenRow>(&sql)
            .bind(token)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 指定订单是否包含该商品（行存在即包含），并带回订单交付上下文。
    pub async fn find_order_context(
        &self,
        order_id: i32,
        product_id: i32,
    ) -> Result<Option<DeliveryOrderRow>> {
        sqlx::query_as::<_, DeliveryOrderRow>(
            "SELECT o.order_id, o.user_id, o.pay_status, o.shipping_status \
             FROM \"order\" o \
             JOIN order_item oi ON oi.order_id = o.order_id \
             WHERE o.order_id = $1 AND oi.product_id = $2 AND o.is_del = 0 \
             LIMIT 1",
        )
        .bind(order_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 本人最近一张「已支付」且包含该商品的订单（商品页申领绑单用）。
    /// `require_shipped` 时（type=2）附加 `shipping_status != 0`。
    pub async fn find_latest_paid_order(
        &self,
        user_id: i32,
        product_id: i32,
        require_shipped: bool,
    ) -> Result<Option<DeliveryOrderRow>> {
        let shipped_cond = if require_shipped {
            " AND o.shipping_status <> 0"
        } else {
            ""
        };
        let sql = format!(
            "SELECT o.order_id, o.user_id, o.pay_status, o.shipping_status \
             FROM \"order\" o \
             JOIN order_item oi ON oi.order_id = o.order_id \
             WHERE o.user_id = $1 AND oi.product_id = $2 AND o.pay_status = 2 AND o.is_del = 0{shipped_cond} \
             ORDER BY o.order_id DESC LIMIT 1"
        );
        sqlx::query_as::<_, DeliveryOrderRow>(&sql)
            .bind(user_id)
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Internal(format!("数据库错误: {e}"))
}
