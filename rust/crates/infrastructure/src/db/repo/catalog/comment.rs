//! 评价域仓储：对应原 Java `CommentMapper`（`BaseMapper<Comment>`）以及
//! `CommentServiceImpl` 中直接用到的 `product` / `order_item` / `order` /
//! `config` 表访问。
//!
//! 列名严格对齐权威 schema（`python tools/schema_parser.py --table comment`，
//! 23 列）：`comment_id` / `user_id` / `username` / `is_anonymous` / `avatar` /
//! `product_id` / `order_id` / `order_item_id` / `comment_rank` / `content` /
//! `add_time` / `status` / `parent_id` / `usefull` / `useless` / `comment_tag` /
//! `show_pics` / `is_recommend` / `is_top` / `is_showed` / `is_default` /
//! `sort_order` / `shop_id`。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `query` / `QueryBuilder`
//! （**禁止** `query!` 宏，工作区无 `DATABASE_URL`）。
//!
//! 注意：`ORDER` 是 MySQL 保留字，涉及订单表的 SQL 必须用反引号。
//! `comment` 在本库既有 SQL 中直接使用（非保留字，历史实现亦如此）。

use sqlx::PgPool;

use nextrade_core::entities::{Comment, OrderItem, Product};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 绑定值（动态 WHERE 的参数列表；`rank` 过滤是常量片段，不产生绑定）。
pub enum Bind {
    I32(i32),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

/// `comment` 表全部列（顺序与 [`Comment`] 实体字段一致，禁止 `SELECT *`
/// 用于列表接口，便于 SQL 列校验门禁比对）。
const COMMENT_COLUMNS: &str = "comment_id, user_id, username, is_anonymous, avatar, product_id, order_id, order_item_id, comment_rank, content, add_time, status, parent_id, usefull, useless, comment_tag, show_pics, is_recommend, is_top, is_showed, is_default, sort_order, shop_id";

/// 排序字段白名单（对应 Java `BaseServiceImpl#getColumnByPropertyName` 的
/// 防注入兜底：未知道段回退主键）。
const SORTABLE_COLUMNS: [&str; 8] = [
    "comment_id",
    "add_time",
    "comment_rank",
    "is_top",
    "is_showed",
    "is_recommend",
    "sort_order",
    "status",
];

/// 评价列表的星级过滤（对应 Java `CommentListPageQuery.type` 的 2/3/4 分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankFilter {
    /// 好评：`comment_rank >= 4`
    Good,
    /// 中评：`comment_rank = 3`
    Neutral,
    /// 差评：`comment_rank <= 2`
    Bad,
}

/// 评价列表过滤条件（管理端 `list` 与前台 `listForClient` 共用）。
///
/// `shop_scope` 对应 Java `getShopId() != null && getShopId() > 0` 的请求头店铺
/// 过滤；Rust JWT 尚未签发店铺，当前由查询参数 `shopId` 代替（见服务层注释）。
#[derive(Debug, Clone, Default)]
pub struct CommentFilters<'a> {
    /// `is_showed = ?`（`None` / `-1` 不过滤）
    pub is_showed: Option<i32>,
    pub product_id: Option<i32>,
    /// 查询参数 `shopId`
    pub shop_id: Option<i32>,
    /// 关键词（`content LIKE %keyword%`）
    pub keyword: Option<&'a str>,
    pub user_id: Option<i32>,
    pub rank: Option<RankFilter>,
    /// Java `getShopId() > 0` 的店铺范围过滤
    pub shop_scope: Option<i32>,
    pub sort_field: Option<&'a str>,
    pub sort_order: Option<&'a str>,
}

#[derive(Clone)]
pub struct CommentRepository {
    pool: PgPool,
}

impl CommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键查询评价。
    pub async fn find_by_id(&self, comment_id: i32) -> Result<Option<Comment>> {
        let sql = format!("SELECT {COMMENT_COLUMNS} FROM comment WHERE comment_id = $1 LIMIT 1");
        sqlx::query_as::<_, Comment>(&sql)
            .bind(comment_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 评价分页（管理端 `list` / 前台 `listForClient` 主查询）。
    ///
    /// 对应 Java `CommentServiceImpl#list` 的 `LambdaQueryWrapper`：
    /// - `is_showed != null && != -1` → `eq`
    /// - `product_id != null` / `user_id != null` → `eq`
    /// - `keyword` 非空白 → `content LIKE`
    /// - `type` 2/3/4 → `comment_rank >= 4` / `= 3` / `<= 2`
    /// - 请求头店铺 `> 0` → `eq(shop_id)`
    /// - **恒有** `parent_id = 0`（只列父评价）
    ///
    /// 排序：Java 先 `buildSortOrder(pageQuery)`（缺省主键降序）再
    /// `.orderByDesc(addTime)`；MyBatis-Plus 分页插件把 `page.orders`
    /// 追加到 SQL 末尾，因此实际顺序是 `add_time DESC, <排序列>`。
    /// 无 `sortField` 时排序列回退主键。
    pub async fn paginate(
        &self,
        f: &CommentFilters<'_>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Comment>)> {
        let kw = f
            .keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));

        let mut where_sql = String::from(" WHERE parent_id = 0");
        if f.is_showed.is_some_and(|v| v != -1) {
            where_sql.push_str(" AND is_showed = ?");
        }
        if f.product_id.is_some() {
            where_sql.push_str(" AND product_id = ?");
        }
        if f.shop_id.is_some() {
            where_sql.push_str(" AND shop_id = ?");
        }
        if kw.is_some() {
            where_sql.push_str(" AND content LIKE ?");
        }
        if f.user_id.is_some() {
            where_sql.push_str(" AND user_id = ?");
        }
        match f.rank {
            Some(RankFilter::Good) => where_sql.push_str(" AND comment_rank >= 4"),
            Some(RankFilter::Neutral) => where_sql.push_str(" AND comment_rank = 3"),
            Some(RankFilter::Bad) => where_sql.push_str(" AND comment_rank <= 2"),
            None => {}
        }
        if f.shop_scope.is_some_and(|s| s > 0) {
            where_sql.push_str(" AND shop_id = ?");
        }

        let column = resolve_sort_column(f.sort_field);
        // Java `buildSortOrder`：仅显式 asc 升序，desc/空一律降序。
        let dir = if f.sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        // Java 显式 `orderByDesc(addTime)` 在前，分页插件的主键/排序列在后。
        let order_sql = format!(" ORDER BY add_time DESC, {column} {dir}");

        let binds = build_binds(f, kw.clone());

        let count_sql = format!("SELECT COUNT(*) FROM comment{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&count_sql), &binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let list_sql = format!("SELECT {COMMENT_COLUMNS} FROM comment{where_sql}{order_sql} LIMIT ? OFFSET ?");
        let rows = bind_all!(sqlx::query_as::<_, Comment>(&list_sql), &binds)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 子评论（`parent_id IN (...)`，Java `in(Comment::getParentId, commentIds)`）。
    pub async fn find_replies(&self, parent_ids: &[i32]) -> Result<Vec<Comment>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; parent_ids.len()].join(", ");
        let sql = format!(
            "SELECT {COMMENT_COLUMNS} FROM comment WHERE parent_id IN ({placeholders}) ORDER BY comment_id ASC"
        );
        let mut q = sqlx::query_as::<_, Comment>(&sql);
        for id in parent_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某个父评价下的第一条回复（Java `replyComment` 里 `getOne(parent_id = ?)`）。
    ///
    /// Java `getOne` 在命中多行时抛异常；本实现取 `LIMIT 1`（表无唯一约束，
    /// 历史脏数据不应阻断回复，见服务层不确定点）。
    pub async fn find_one_reply(&self, parent_id: i32) -> Result<Option<Comment>> {
        let sql =
            format!("SELECT {COMMENT_COLUMNS} FROM comment WHERE parent_id = $1 ORDER BY comment_id ASC LIMIT 1");
        sqlx::query_as::<_, Comment>(&sql)
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键批量取商品（列表回填 `productName` / `productSn` / `picThumb`，
    /// 对应 Java `productMapper.selectBatchIds`，**不限商品状态**）。
    pub async fn find_products(&self, product_ids: &[i32]) -> Result<Vec<Product>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; product_ids.len()].join(", ");
        let sql = format!("SELECT * FROM product WHERE product_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, Product>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 商品所属店铺（Java `create`：`productMapper.selectById(..).getShopId()`）。
    pub async fn find_product_shop_id(&self, product_id: i32) -> Result<Option<i32>> {
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT shop_id FROM product WHERE product_id = $1 LIMIT 1")
                .bind(product_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    /// 批量取订单商品（回填 `skuId` / `skuData`，Java `orderItemMapper.selectBatchIds`）。
    pub async fn find_order_items(&self, item_ids: &[i32]) -> Result<Vec<OrderItem>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; item_ids.len()].join(", ");
        let sql = format!("SELECT * FROM order_item WHERE item_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, OrderItem>(&sql);
        for id in item_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按 `biz_code` 取未删除配置值（Java `ConfigService#getConfigVal`）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ————————————————————— 评论晒单数量统计（subNum） —————————————————————

    /// 待评价订单数：`user_id = ? AND is_del = 0 AND order_status = 5
    /// AND comment_status = 0`（Java `subNum` 第一段查询）。
    pub async fn count_await_comment(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM \"order\" WHERE user_id = $1 AND is_del = 0 AND order_status = 5 AND comment_status = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 已评价数：`user_id = ? AND parent_id = 0`。
    pub async fn count_comments_by_user(&self, user_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM comment WHERE user_id = $1 AND parent_id = 0")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 会员全部有效订单 ID（Java `subNum`：`eq(userId).eq(isDel, 0)`）。
    pub async fn find_order_ids_by_user(&self, user_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> =
            sqlx::query_as("SELECT order_id FROM \"order\" WHERE user_id = $1 AND is_del = 0")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 待晒单数：`order_id IN (...) AND user_id = ? AND parent_id = 0 AND is_showed = 0`。
    pub async fn count_pending_show_pics(
        &self,
        order_ids: &[i32],
        user_id: i32,
    ) -> Result<i64> {
        if order_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; order_ids.len()].join(", ");
        let sql = format!(
            "SELECT COUNT(*) FROM comment WHERE order_id IN ({placeholders}) AND user_id = $1 AND parent_id = 0 AND is_showed = 0"
        );
        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        for id in order_ids {
            q = q.bind(*id);
        }
        let row = q.bind(user_id).fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    // ————————————————————— 前台评价（evaluate）辅助 —————————————————————

    /// 查「该会员对该订单项」的父评价（Java `evaluate` 的 `getOne`）。
    pub async fn find_one_by_order_item(
        &self,
        user_id: i32,
        order_id: i32,
        order_item_id: i32,
    ) -> Result<Option<Comment>> {
        let sql = format!(
            "SELECT {COMMENT_COLUMNS} FROM comment WHERE user_id = $1 AND order_item_id = $2 AND order_id = $3 AND parent_id = 0 LIMIT 1"
        );
        sqlx::query_as::<_, Comment>(&sql)
            .bind(user_id)
            .bind(order_item_id)
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 订单全部订单项 ID（Java `orderItemMapper.selectList(eq(orderId).eq(userId))`）。
    pub async fn find_order_item_ids(&self, order_id: i32, user_id: i32) -> Result<Vec<i32>> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT item_id FROM order_item WHERE order_id = $1 AND user_id = $2 ORDER BY item_id ASC",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 该订单下某会员已评价的订单项数（用于判断「整单已评完」）。
    pub async fn count_commented_order_items(
        &self,
        order_id: i32,
        user_id: i32,
        item_ids: &[i32],
    ) -> Result<i64> {
        if item_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; item_ids.len()].join(", ");
        let sql = format!(
            "SELECT COUNT(*) FROM comment WHERE order_id = $1 AND parent_id = 0 AND user_id = $2 AND order_item_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (i64,)>(&sql)
            .bind(order_id)
            .bind(user_id);
        for id in item_ids {
            q = q.bind(*id);
        }
        let row = q.fetch_one(&self.pool).await.map_err(map_err)?;
        Ok(row.0)
    }

    /// 整单商品已全部评价 → 置 `order.comment_status = 1`（Java `evaluate` 末尾）。
    pub async fn mark_order_commented(&self, order_id: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE \"order\" SET comment_status = 1 WHERE order_id = $1")
            .bind(order_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增评价，返回自增主键。
    pub async fn insert(&self, c: &Comment) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO comment (user_id, username, is_anonymous, avatar, product_id, order_id, order_item_id, comment_rank, content, add_time, status, parent_id, usefull, useless, comment_tag, show_pics, is_recommend, is_top, is_showed, is_default, sort_order, shop_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22) RETURNING comment_id",
        )
        .bind(c.user_id)
        .bind(&c.username)
        .bind(c.is_anonymous)
        .bind(&c.avatar)
        .bind(c.product_id)
        .bind(c.order_id)
        .bind(c.order_item_id)
        .bind(c.comment_rank)
        .bind(&c.content)
        .bind(c.add_time)
        .bind(c.status)
        .bind(c.parent_id)
        .bind(c.usefull)
        .bind(c.useless)
        .bind(&c.comment_tag)
        .bind(&c.show_pics)
        .bind(c.is_recommend)
        .bind(c.is_top)
        .bind(c.is_showed)
        .bind(c.is_default)
        .bind(c.sort_order)
        .bind(c.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新（对应 Java `updateById`；调用方已按 MyBatis `NOT_NULL` 策略
    /// 把「DTO 未传」的列解析为库中原值，故这里是全列 UPDATE）。
    pub async fn update(&self, c: &Comment) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE comment SET user_id = $1, username = $2, is_anonymous = $3, avatar = $4, product_id = $5, order_id = $6, order_item_id = $7, comment_rank = $8, content = $9, add_time = $10, status = $11, parent_id = $12, usefull = $13, useless = $14, comment_tag = $15, show_pics = $16, is_recommend = $17, is_top = $18, is_showed = $19, is_default = $20, sort_order = $21, shop_id = $22 WHERE comment_id = $23",
        )
        .bind(c.user_id)
        .bind(&c.username)
        .bind(c.is_anonymous)
        .bind(&c.avatar)
        .bind(c.product_id)
        .bind(c.order_id)
        .bind(c.order_item_id)
        .bind(c.comment_rank)
        .bind(&c.content)
        .bind(c.add_time)
        .bind(c.status)
        .bind(c.parent_id)
        .bind(c.usefull)
        .bind(c.useless)
        .bind(&c.comment_tag)
        .bind(&c.show_pics)
        .bind(c.is_recommend)
        .bind(c.is_top)
        .bind(c.is_showed)
        .bind(c.is_default)
        .bind(c.sort_order)
        .bind(c.shop_id)
        .bind(c.comment_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 只更新「晒单」相关两列（前台 `evaluate` 的「追加晒单」分支）。
    pub async fn update_show_pics(
        &self,
        comment_id: i32,
        show_pics: &str,
        is_showed: i32,
    ) -> Result<u64> {
        let r = sqlx::query("UPDATE comment SET show_pics = $1, is_showed = $2 WHERE comment_id = $3")
            .bind(show_pics)
            .bind(is_showed)
            .bind(comment_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 置评价状态（`replyComment` 首次回复时把父评价置为审核通过）。
    pub async fn set_status(&self, comment_id: i32, status: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE comment SET status = $1 WHERE comment_id = $2")
            .bind(status)
            .bind(comment_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 删除单条评价（`comment` 无删除标记列，物理 DELETE）。
    pub async fn delete(&self, comment_id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM comment WHERE comment_id = $1")
            .bind(comment_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除（对应 Java `removeByIds`）。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!("DELETE FROM comment WHERE comment_id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 更新单个字段（列名来自白名单，防注入）。
    pub async fn update_field(&self, comment_id: i32, field: &str, val: &str) -> Result<bool> {
        if resolve_update_column(field).is_none() {
            return Err(Error::Validation(format!("field 的值有误: {field}")));
        }
        let column = resolve_update_column(field).unwrap_or("comment_rank");
        let sql = format!("UPDATE comment SET \"{column}\" = $1 WHERE comment_id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(comment_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

/// 生成与 [`CommentRepository::paginate`] 的 WHERE 片段顺序严格一致的绑定列表：
/// is_showed → product_id → shop_id → keyword → user_id → shop_scope。
fn build_binds(f: &CommentFilters<'_>, kw: Option<String>) -> Vec<Bind> {
    let mut binds = Vec::new();
    if let Some(v) = f.is_showed.filter(|v| *v != -1) {
        binds.push(Bind::I32(v));
    }
    if let Some(v) = f.product_id {
        binds.push(Bind::I32(v));
    }
    if let Some(v) = f.shop_id {
        binds.push(Bind::I32(v));
    }
    if let Some(k) = kw {
        binds.push(Bind::Str(k));
    }
    if let Some(v) = f.user_id {
        binds.push(Bind::I32(v));
    }
    if let Some(v) = f.shop_scope.filter(|s| *s > 0) {
        binds.push(Bind::I32(v));
    }
    binds
}

/// 排序字段解析（未知/缺失回退主键，防 SQL 注入）。
fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            SORTABLE_COLUMNS
                .iter()
                .find(|c| **c == snake)
                .copied()
                .unwrap_or("comment_id")
        }
        None => "comment_id",
    }
}

/// `updateField` 字段白名单（Java `COMMENT_FIELDS` ∩ `comment` 真实列；
/// 对应 Java 先 `getColumnByPropertyName` 再白名单校验的等价结果——
/// `is_show` 不在表中，会回退主键从而被拒绝）。
fn resolve_update_column(field: &str) -> Option<&'static str> {
    const UPDATABLE: [&str; 4] = ["comment_rank", "is_recommend", "is_top", "sort_order"];
    let snake = to_snake(field);
    UPDATABLE.iter().copied().find(|c| *c == snake)
}

/// 驼峰 -> 下划线（允许前端传驼峰或数据库列名）。
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("评价不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_columns_match_entity_field_count() {
        assert_eq!(COMMENT_COLUMNS.split(',').count(), 23);
        // 首尾列与主键位置固定
        assert!(COMMENT_COLUMNS.starts_with("comment_id, user_id"));
        assert!(COMMENT_COLUMNS.ends_with(", sort_order, shop_id"));
    }

    #[test]
    fn sort_column_whitelist_blocks_injection() {
        assert_eq!(resolve_sort_column(None), "comment_id");
        assert_eq!(resolve_sort_column(Some("sortOrder")), "sort_order");
        assert_eq!(resolve_sort_column(Some("sort_order")), "sort_order");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("isTop")), "is_top");
        assert_eq!(resolve_sort_column(Some("commentRank")), "comment_rank");
        // 未知字段回退主键（Java getColumnByPropertyName 同语义）
        assert_eq!(
            resolve_sort_column(Some("1;DROP TABLE comment")),
            "comment_id"
        );
    }

    #[test]
    fn update_column_whitelist_rejects_unknown_and_is_show() {
        assert_eq!(resolve_update_column("sortOrder"), Some("sort_order"));
        assert_eq!(resolve_update_column("comment_rank"), Some("comment_rank"));
        assert_eq!(resolve_update_column("isTop"), Some("is_top"));
        assert_eq!(resolve_update_column("isRecommend"), Some("is_recommend"));
        // Java COMMENT_FIELDS 含 is_show，但 comment 表无该列 → 必拒绝
        assert_eq!(resolve_update_column("is_show"), None);
        assert_eq!(resolve_update_column("content"), None);
    }

    #[test]
    fn snake_conversion_is_idempotent_for_snake_input() {
        assert_eq!(to_snake("comment_rank"), "comment_rank");
        assert_eq!(to_snake("commentRank"), "comment_rank");
        assert_eq!(to_snake("isShowed"), "is_showed");
    }
}
