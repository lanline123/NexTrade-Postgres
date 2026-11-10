//! 商品询价域仓储：对应原 Java `PriceInquiryMapper` +
//! `PriceInquiryReplyRecordMapper` + `ProductMapper` 的只读查询。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table price_inquiry`）：
//! - `price_inquiry`：`id` / `mobile` / `email` / `content` / `product_id` /
//!   `remark` / `status` / `shop_id` / `user_id` / `user_is_del` / `create_time`
//! - `price_inquiry_reply_record`：`id` / `inquiry_id` / `reply_type` /
//!   `reply_name` / `content` / `admin_id` / `user_id` / `create_time`
//! - `product`（本仓储只读 4 列）：`product_id` / `product_name` /
//!   `product_sn` / `pic_url`
//!
//! 所有列表 SQL 显式列出列名，不用 `SELECT *`。
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! # 事务
//!
//! `replyByAdmin` 在 Java 里是 `@Transactional`（先更新询价状态/备注，再插入
//! 回复记录）。本仓储提供 [`PriceInquiryRepository::begin`] +
//! `*_in_tx` 方法，让服务层把两次写入放进同一事务；SQL 细节不泄漏到
//! services 层（与 member_tag_group / user_company 同一做法）。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{PriceInquiry, PriceInquiryReplyRecord};
use nextrade_core::models::catalog::price_inquiry::PriceInquiryQuery;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `price_inquiry` 固定 SELECT 列清单（顺序与实体字段一致）。
const INQUIRY_SELECT: &str = "SELECT id, mobile, email, content, product_id, remark, status, shop_id, user_id, user_is_del, create_time FROM price_inquiry";

/// `price_inquiry_reply_record` 固定 SELECT 列清单（顺序与实体字段一致）。
const REPLY_SELECT: &str = "SELECT id, inquiry_id, reply_type, reply_name, content, admin_id, user_id, create_time FROM price_inquiry_reply_record";

/// `product` 只读摘要列清单。
const PRODUCT_BRIEF_SELECT: &str =
    "SELECT product_id, product_name, product_sn, pic_url FROM product";

/// 询价商品摘要（列表 / 详情展示用，对应 Java `Product` 实体的 4 个字段）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct ProductBrief {
    /// 商品 ID
    pub product_id: i32,
    /// 商品名称
    pub product_name: String,
    /// 商品编号
    pub product_sn: String,
    /// 商品主图（Java `PriceInquiryVO.ProductVO.picThumb` 取的就是该列）
    pub pic_url: String,
}

/// 商品询价域仓储（持有连接池）。
#[derive(Clone)]
pub struct PriceInquiryRepository {
    pool: PgPool,
}

impl PriceInquiryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排 `reply` 的两次写入）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 查询（无事务） —————————————————————

    /// 按主键查询询价（对应 Java `getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<PriceInquiry>> {
        sqlx::query_as::<_, PriceInquiry>(&format!("{INQUIRY_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 询价分页（对应 Java `PriceInquiryServiceImpl#list`）。
    ///
    /// 过滤条件与 Java 逐条对齐：
    /// - `product_id IN (...)`：由服务层先用 `keyword` 查商品 ID 得到；
    ///   `Some(&[])` 表示关键词没有匹配商品 —— Java 直接返回空页，绝不发 SQL
    /// - `mobile ILIKE %mobile%`
    /// - `status = ?` / `user_id = ?` / `user_is_del = ?` / `shop_id = ?`
    ///   （均为「传了才过滤」，Java 的 `eq(x != null, ...)`）
    /// - ORDER BY 由 [`PriceInquiryQuery::order_clause`] 生成（白名单列）
    pub async fn paginate(
        &self,
        q: &PriceInquiryQuery,
        product_ids: Option<&[i32]>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PriceInquiry>)> {
        // Java：关键词匹配不到商品时 `return PageUtil.convertPage(page, List.of())`
        if let Some(ids) = product_ids {
            if ids.is_empty() {
                return Ok((0, Vec::new()));
            }
        }
        let (limit, offset) = page.limit_offset();

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM price_inquiry WHERE 1 = 1");
        Self::push_filters(&mut count_qb, q, product_ids);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{INQUIRY_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut row_qb, q, product_ids);
        // ORDER BY 的列名来自 models 白名单（未知回退主键），方向二选一，
        // LIMIT/OFFSET 为 u64 字面量，均无注入面。
        row_qb.push(q.order_clause());
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));

        let rows: Vec<PriceInquiry> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 追加 `list` 的 WHERE 片段（`count` 与取行两条 SQL 共用，保证条件一致）。
    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &PriceInquiryQuery,
        product_ids: Option<&[i32]>,
    ) {
        if let Some(ids) = product_ids {
            qb.push(" AND product_id IN (");
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(*id);
            }
            sep.push_unseparated(")");
            drop(sep);
        }
        // Java `like(StrUtil.isNotBlank(mobile), mobile)`
        if let Some(mobile) = q.effective_mobile() {
            qb.push(" AND mobile ILIKE ");
            qb.push_bind(format!("%{mobile}%"));
        }
        // Java `eq(listDTO.getStatus() != null, ...)` 等四个「传了才过滤」
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(user_id) = q.user_id {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(user_is_del) = q.user_is_del {
            qb.push(" AND user_is_del = ");
            qb.push_bind(user_is_del);
        }
        if let Some(shop_id) = q.shop_id {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
    }

    /// 关键词匹配的商品 ID（对应 Java `productService.lambdaQuery()`
    /// `.like(productName, kw).or().like(productSn, kw).or().like(productTsn, kw)`）。
    ///
    /// 只返回 `product.product_id`，供列表 `IN` 过滤。`keyword` 原样绑定
    /// （Java 判空用 `isNotBlank`，但 `ILIKE` 用原始值，不 trim）。
    pub async fn find_product_ids_by_keyword(&self, keyword: &str) -> Result<Vec<i32>> {
        let like = format!("%{keyword}%");
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT product_id FROM product WHERE product_name ILIKE $1 OR product_sn ILIKE $2 OR product_tsn ILIKE $3",
        )
        .bind(&like)
        .bind(&like)
        .bind(&like)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量取商品摘要（对应 Java `productService.lambdaQuery().in(productId, ids)`）。
    pub async fn find_products_by_ids(&self, ids: &[i32]) -> Result<Vec<ProductBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{PRODUCT_BRIEF_SELECT} WHERE product_id IN ("));
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 单个商品摘要（对应 Java `productService.getById(productId)`）。
    pub async fn find_product_by_id(&self, product_id: i32) -> Result<Option<ProductBrief>> {
        sqlx::query_as::<_, ProductBrief>(&format!(
            "{PRODUCT_BRIEF_SELECT} WHERE product_id = $1 LIMIT 1"
        ))
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 询价回复记录（对应 Java `buildReplyRecords` 的
    /// `orderByAsc(createTime, id)`）。
    pub async fn find_reply_records(&self, inquiry_id: i32) -> Result<Vec<PriceInquiryReplyRecord>> {
        sqlx::query_as::<_, PriceInquiryReplyRecord>(&format!(
            "{REPLY_SELECT} WHERE inquiry_id = $1 ORDER BY create_time ASC, id ASC"
        ))
        .bind(inquiry_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 事务内查询 / 写入 —————————————————————

    /// 事务内按主键查询询价（`reply` 的「先查后改」必须与写入同事务）。
    pub async fn find_by_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
    ) -> Result<Option<PriceInquiry>> {
        sqlx::query_as::<_, PriceInquiry>(&format!("{INQUIRY_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 事务内更新回复状态与备注（对应 Java `lambdaUpdate().set(status,1)
    /// .set(remark, dto.getRemark())`）。
    pub async fn update_reply_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
        remark: &str,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE price_inquiry SET status = 1, remark = $1 WHERE id = $2",
        )
        .bind(remark)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 事务内插入回复记录（对应 Java `priceInquiryReplyRecordMapper.insert`）。
    pub async fn insert_reply_record_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        record: &PriceInquiryReplyRecord,
    ) -> Result<u64> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO price_inquiry_reply_record (inquiry_id, reply_type, reply_name, content, admin_id, user_id, create_time) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(record.inquiry_id)
        .bind(record.reply_type)
        .bind(&record.reply_name)
        .bind(&record.content)
        .bind(record.admin_id)
        .bind(record.user_id)
        .bind(record.create_time)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(res as u64)
    }

    /// 按主键删除（对应 Java `removeById`；`price_inquiry` 无删除标记列）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM price_inquiry WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量删除（对应 Java `removeByIds`）。
    ///
    /// 返回**实际删除行数**；Java `removeByIds` 返回 `SqlHelper.retBool(rows)`
    /// （`rows >= 1`），调用方据此折算布尔值。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM price_inquiry WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let res = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => {
            Error::NotFound(nextrade_core::models::catalog::price_inquiry::INQUIRY_NOT_FOUND.into())
        }
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::price_inquiry::INQUIRY_COLUMNS;

    #[test]
    fn select_lists_cover_entity_columns() {
        // 列清单与 schema 逐列对齐（顺序即实体字段顺序）
        assert_eq!(
            INQUIRY_COLUMNS,
            [
                "id",
                "mobile",
                "email",
                "content",
                "product_id",
                "remark",
                "status",
                "shop_id",
                "user_id",
                "user_is_del",
                "create_time",
            ]
        );
        let selected = INQUIRY_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected, INQUIRY_COLUMNS.join(", "), "SELECT 列清单必须与 schema 一致");
    }

    #[test]
    fn reply_select_covers_all_eight_columns() {
        let selected = REPLY_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap();
        assert_eq!(selected.split(", ").count(), 8);
        assert!(REPLY_SELECT.ends_with("FROM price_inquiry_reply_record"));
    }

    #[test]
    fn product_brief_select_stays_minimal() {
        // 只读 4 列：product 表有 60+ 列，全量 SELECT 会拖慢列表
        assert_eq!(
            PRODUCT_BRIEF_SELECT,
            "SELECT product_id, product_name, product_sn, pic_url FROM product"
        );
    }
}
