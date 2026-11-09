//! 买家秀域（marketing::buyer_show）仓储：`buyer_show` / `buyer_show_gallery` /
//! `buyer_show_video` / `buyer_show_comment` / `buyer_show_like` /
//! `buyer_show_favorite` / `buyer_show_group` / `comment_like` /
//! `virtual_user_info` / `user`。
//!
//! 列名严格对齐 `java/database/pure.sql`（`python tools/schema_parser.py --table <表名>`）：
//! - `buyer_show`：`buyer_show_id` / `range_data` / `user_id` / `buyer_photo` /
//!   `buyer_name` / `content` / `group_data` / `add_time` / `sort_order` /
//!   `status` / `is_show`
//! - `buyer_show_gallery`：`pic_id` / `buyer_show_id` / `pic_url` / `pic_desc` /
//!   `pic_thumb` / `pic_original` / `pic_large` / `sort_order`
//! - `buyer_show_video`：`id` / `video_id` / `buyer_show_id` / `video_url` /
//!   `video_cover` / `format` / `sort_order`
//! - `buyer_show_comment`：`buyer_show_comment_id` / `parent_id` / `buyer_show_id` /
//!   `status` / `is_top` / `is_show` / `user_id` / `user_name` / `user_photo` /
//!   `content` / `add_time` / `comment_pic`
//! - `buyer_show_like` / `buyer_show_favorite`：`id` / `buyer_show_id` / `user_id` / `add_time`
//! - `comment_like`：`id` / `comment_id` / `user_id` / `add_time`
//! - `buyer_show_group`：`buyer_show_group_id` / `group_name` / `product_group_id` / `shop_id`
//! - `virtual_user_info`：`id` / `photo` / `nickname`
//! - `user`（只读 2 列）：`user_id` / `username`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏，本机无 `DATABASE_URL`）。
//!
//! ## 写法约定（工具限制）
//!
//! `tools/check_sql_columns.py` 只能解析「闭合在同一行 / 连续字面量」中的 SQL，
//! 因此本文件的静态 SQL 一律**单行书写**；含 `format!` 插值的语句该脚本会跳过
//! （无法解析命名插值），这些语句的列名由本文件头部清单 + `pure.sql` 人工核对保证。
//!
//! ## `add_time` 口径
//!
//! `buyer_show.add_time` / `buyer_show_comment.add_time` 是**秒级** `bigint unsigned`
//! （不是 MySQL `datetime`），与 Java `StringUtils.getCurrentTime()` 一致。
//!
//! ## 事务边界
//!
//! 原 Java 的 `create` / `update` / `del` / `batchDel` 都带
//! `@Transactional(rollbackFor = Exception.class)`。多语句的组合操作因此**在本层
//! 内部开启事务**（[`BuyerShowRepository::create_buyer_show_with_assets`] /
//! [`BuyerShowRepository::replace_assets`] /
//! [`BuyerShowRepository::delete_buyer_shows_cascade`]），
//! 与 `repo::nextrade_core::models::merchant::shop_manage::insert_store_tips` 的既有做法一致。

use sqlx::PgPool;
use nextrade_core::entities::{
    BuyerShow, BuyerShowGallery, BuyerShowGroup, BuyerShowVideo, VirtualUserInfo,
};
use nextrade_core::models::marketing::buyer_show::BuyerShowListQuery;
use nextrade_core::models::marketing::buyer_show::ListStatusView;
use nextrade_core::{Error, Result};

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
///
/// 与 `crates/services/src/buyer_show.rs::SERVICE_DATA_ERROR` 同值；仓储层在
/// 级联删除里直接判定业务错误（行数校验 / 状态门），故在此重复声明一份常量。
const SERVICE_DATA_ERROR: i32 = 1001;

/// 待绑定参数（按 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    I32(i32),
    I64(i64),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

/// 新建买家秀的落库字段（服务层已把 JSON / 默认值算好）。
#[derive(Debug, Clone)]
pub struct NewBuyerShow {
    /// 关联商品 JSON 字符串（`range_data` NOT NULL）
    pub range_data: String,
    /// 用户 ID（管理端新建恒 0）
    pub user_id: i32,
    /// 买家头像
    pub buyer_photo: String,
    /// 买家名称
    pub buyer_name: String,
    /// 文字内容
    pub content: Option<String>,
    /// 分组 JSON 字符串（空则 `NULL`，与 Java 一致）
    pub group_data: Option<String>,
    /// 创建时间（秒）
    pub add_time: i64,
    /// 排序（可空：Java `create` 不写默认值，列 `DEFAULT 0`）
    pub sort_order: Option<i32>,
    /// 审核状态（`create` 恒 1）
    pub status: i32,
    /// 公开标记（`create` 恒 1）
    pub is_show: i32,
}

/// 更新买家秀的落库字段（对应 Java `BeanUtils.copyProperties(param, buyerShow)`
/// + `updateById`）。
#[derive(Debug, Clone)]
pub struct BuyerShowUpdateFields {
    /// 关联商品 JSON 字符串
    pub range_data: String,
    /// 买家头像
    pub buyer_photo: String,
    /// 买家名称
    pub buyer_name: String,
    /// 文字内容
    pub content: Option<String>,
    /// 分组 JSON 字符串
    pub group_data: Option<String>,
    /// 排序
    pub sort_order: Option<i32>,
}

/// 买家秀相册插入行（服务层已算好 `pic_url` / `pic_large` 的尺寸后缀）。
#[derive(Debug, Clone)]
pub struct NewGalleryRow {
    /// 实际图片 url（已拼尺寸后缀）
    pub pic_url: String,
    /// 图片说明（Java 不写 → 空串，列 `DEFAULT ''`）
    pub pic_desc: String,
    /// 微缩图 url
    pub pic_thumb: String,
    /// 原图 url
    pub pic_original: String,
    /// 高清图 url（已拼尺寸后缀）
    pub pic_large: String,
    /// 排序
    pub sort_order: i32,
}

/// 买家秀视频插入行（服务层已把 `id` 置空、`sort_order` 按下标兜底）。
#[derive(Debug, Clone)]
pub struct NewVideoRow {
    /// 视频 ID
    pub video_id: i32,
    /// 视频 url
    pub video_url: String,
    /// 视频封面
    pub video_cover: String,
    /// 视频格式后缀
    pub format: String,
    /// 排序
    pub sort_order: i32,
}

/// 单个买家秀的素材（图片 + 视频），用于「整体替换」。
#[derive(Debug, Clone, Default)]
pub struct BuyerShowAssets {
    /// 图片（顺序即前端传入顺序，`sort_order` 由服务层按下标写好）
    pub gallery: Vec<NewGalleryRow>,
    /// 视频（同上）
    pub videos: Vec<NewVideoRow>,
}

/// 买家秀管理端评论插入行（`comment` 接口，管理端代发）。
#[derive(Debug, Clone)]
pub struct NewAdminComment {
    /// 买家秀 ID
    pub buyer_show_id: i32,
    /// 用户名称
    pub user_name: String,
    /// 用户头像
    pub user_photo: String,
    /// 评论内容
    pub content: Option<String>,
    /// 发布时间（秒）
    pub add_time: i64,
}

/// 买家秀域仓储。
#[derive(Clone)]
pub struct BuyerShowRepository {
    pool: PgPool,
}

impl BuyerShowRepository {
    /// 构造（与其它切片同一约定）。
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— 事务型组合操作 —————————————————————

    /// 在一个事务里**新建买家秀 + 写入素材**（对应 Java `create` 的
    /// `@Transactional`：`save(buyerShow)` + `saveOrUpdateBuyerShowVideo` +
    /// `saveOrUpdateBuyerShowGallery`）。
    ///
    /// 返回新建的 `buyer_show_id`；素材写入失败整体回滚，不留「有主记录无素材」的脏数据。
    pub async fn create_buyer_show_with_assets(
        &self,
        row: &NewBuyerShow,
        assets: &BuyerShowAssets,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let res: i32 = sqlx::query_scalar(
            "INSERT INTO \"buyer_show\" (range_data, user_id, buyer_photo, buyer_name, content, group_data, add_time, sort_order, status, is_show) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING buyer_show_id",
        )
        .bind(&row.range_data)
        .bind(row.user_id)
        .bind(&row.buyer_photo)
        .bind(&row.buyer_name)
        .bind(row.content.as_deref())
        .bind(row.group_data.as_deref())
        .bind(row.add_time)
        .bind(row.sort_order)
        .bind(row.status)
        .bind(row.is_show)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        let buyer_show_id = res;

        write_assets(&mut tx, buyer_show_id, assets).await?;

        tx.commit().await.map_err(map_err)?;
        Ok(buyer_show_id)
    }

    /// 在一个事务里**整体替换某买家秀的素材**（对应 Java
    /// `saveOrUpdateBuyerShowVideo` / `saveOrUpdateBuyerShowGallery`：
    /// 先按 `buyer_show_id` 全删，再按下标逐条插入）。
    ///
    /// Java `update` 的顺序是「先素材、后 `updateById`」，本方法只负责素材，
    /// 主记录更新由 [`BuyerShowRepository::update_buyer_show`] 完成。
    pub async fn replace_assets(&self, buyer_show_id: i32, assets: &BuyerShowAssets) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        write_assets(&mut tx, buyer_show_id, assets).await?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 在**一个事务**里级联删除买家秀（对应 Java `deleteAll`）。
    ///
    /// 逐条对齐 Java 的删除顺序：
    /// 1. `SELECT ... WHERE buyer_show_id IN (...) ORDER BY buyer_show_id ASC FOR UPDATE`
    /// 2. 行数不一致 → `买家秀不存在`
    /// 3. 存在 `status` 非 1/2 的行 → `待审核买家秀不允许删除`
    /// 4. 取评论 ID → 删 `comment_like` → 删 `buyer_show_comment`
    /// 5. 删 `buyer_show_gallery` / `buyer_show_video` / `buyer_show_like` /
    ///    `buyer_show_favorite`
    /// 6. 删 `buyer_show` 主记录
    ///
    /// `is_deletable` 由服务层提供（Java `BuyerShowServiceImpl#isDeletableStatus`），
    /// 避免仓储层依赖服务层的业务判定。
    pub async fn delete_buyer_shows_cascade(
        &self,
        ids: &[i32],
        is_deletable: fn(i32) -> bool,
    ) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let sorted = sort_dedup(ids);
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let select_sql = format!(
            "SELECT * FROM \"buyer_show\" WHERE buyer_show_id IN ({}) ORDER BY buyer_show_id ASC FOR UPDATE",
            placeholders(sorted.len())
        );
        let mut q = sqlx::query_as::<_, BuyerShow>(&select_sql);
        for id in &sorted {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&mut *tx).await.map_err(map_err)?;
        if rows.len() != sorted.len() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "买家秀不存在"));
        }
        if rows.iter().any(|r| !is_deletable(r.status)) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "待审核买家秀不允许删除"));
        }

        // 评论 ID → 先删评论收到的点赞
        let comment_sql = format!(
            "SELECT buyer_show_comment_id FROM \"buyer_show_comment\" WHERE buyer_show_id IN ({})",
            placeholders(sorted.len())
        );
        let mut cq = sqlx::query_as::<_, (i32,)>(&comment_sql);
        for id in &sorted {
            cq = cq.bind(*id);
        }
        let comment_ids: Vec<i32> = cq
            .fetch_all(&mut *tx)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|r| r.0)
            .collect();
        if !comment_ids.is_empty() {
            let sql = format!(
                "DELETE FROM \"comment_like\" WHERE comment_id IN ({})",
                placeholders(comment_ids.len())
            );
            let mut d = sqlx::query(&sql);
            for id in &comment_ids {
                d = d.bind(*id);
            }
            d.execute(&mut *tx).await.map_err(map_err)?;
        }

        for table in [
            "buyer_show_comment",
            "buyer_show_gallery",
            "buyer_show_video",
            "buyer_show_like",
            "buyer_show_favorite",
        ] {
            let sql = format!(
                "DELETE FROM \"{table}\" WHERE buyer_show_id IN ({})",
                placeholders(sorted.len())
            );
            let mut d = sqlx::query(&sql);
            for id in &sorted {
                d = d.bind(*id);
            }
            d.execute(&mut *tx).await.map_err(map_err)?;
        }

        let del_sql = format!(
            "DELETE FROM \"buyer_show\" WHERE buyer_show_id IN ({})",
            placeholders(sorted.len())
        );
        let mut d = sqlx::query(&del_sql);
        for id in &sorted {
            d = d.bind(*id);
        }
        let res = d.execute(&mut *tx).await.map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— buyer_show —————————————————————

    /// 按 ID 查买家秀（Java `BaseServiceImpl#getById`）。
    pub async fn find_buyer_show(&self, buyer_show_id: i32) -> Result<Option<BuyerShow>> {
        sqlx::query_as::<_, BuyerShow>("SELECT * FROM \"buyer_show\" WHERE buyer_show_id = $1 LIMIT 1")
            .bind(buyer_show_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 买家秀分页（对应 Java `BuyerShowServiceImpl#list` 的查询构造器部分）。
    ///
    /// 条件顺序与 Java 完全一致：
    /// 1. `status` 三态（见 [`ListStatusView`]）
    /// 2. `content LIKE %keyword%`（仅当 keyword 非空白）
    /// 3. `JSON_CONTAINS(group_data, '<goodsId>')`（仅当 `goodsId != null`）
    /// 4. `add_time BETWEEN toTimestampYmd(startTime) AND toTimestampYmd(endTime)`
    ///    （仅当 start/end **同时**非空白）
    ///
    /// `order_sql` 由服务层按 [`resolve_sort_column`] 的结果拼好，
    /// 用户输入**不会**被直接拼进 SQL。
    pub async fn paginate_buyer_shows(
        &self,
        q: &BuyerShowListQuery,
        start_ts: Option<i64>,
        end_ts: Option<i64>,
        order_sql: &str,
    ) -> Result<(u64, Vec<BuyerShow>)> {
        let (where_sql, binds) = build_filter(q, start_ts, end_ts);
        let (limit, offset) = (q.page_size(), (q.page_no() - 1) * q.page_size());

        let total_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"buyer_show\"{where_sql}"));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let rows_sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM \"buyer_show\"{where_sql}{order_sql} LIMIT ? OFFSET ?"));
        let rows = bind_all!(sqlx::query_as::<_, BuyerShow>(&rows_sql), binds)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 更新买家秀（Java `updateById(buyerShow)`；影响行数 < 1 视为不存在）。
    pub async fn update_buyer_show(
        &self,
        buyer_show_id: i32,
        row: &BuyerShowUpdateFields,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE \"buyer_show\" SET range_data = $1, buyer_photo = $2, buyer_name = $3, content = $4, group_data = $5, sort_order = $6 WHERE buyer_show_id = $7",
        )
        .bind(&row.range_data)
        .bind(&row.buyer_photo)
        .bind(&row.buyer_name)
        .bind(row.content.as_deref())
        .bind(row.group_data.as_deref())
        .bind(row.sort_order)
        .bind(buyer_show_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 审核（Java `audit`：`set(status, param.status).set(isShow, status == 1 ? 1 : 0)`）。
    pub async fn update_audit(&self, buyer_show_id: i32, status: i32, is_show: i32) -> Result<u64> {
        let res =
            sqlx::query("UPDATE \"buyer_show\" SET status = $1, is_show = $2 WHERE buyer_show_id = $3")
                .bind(status)
                .bind(is_show)
                .bind(buyer_show_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量审核（Java `batch` 的 `type == "audit"` 分支）。
    pub async fn batch_audit(&self, ids: &[i32], status: i32, is_show: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "UPDATE \"buyer_show\" SET status = $1, is_show = $2 WHERE buyer_show_id IN ({})",
            placeholders(ids.len())
        );
        let mut q = sqlx::query(&sql).bind(status).bind(is_show);
        for id in ids {
            q = q.bind(*id);
        }
        let res = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 公开 / 隐藏（Java `showOrHide`：仅改 `is_show`）。
    pub async fn update_is_show(&self, buyer_show_id: i32, is_show: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE \"buyer_show\" SET is_show = $1 WHERE buyer_show_id = $2")
            .bind(is_show)
            .bind(buyer_show_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新 `sort_order`（Java `BaseServiceImpl#updateField`，白名单只有这一列）。
    ///
    /// Java 把 `UpdateFieldDTO.val`（String）直接 set 到列上，由 MySQL 做隐式转换；
    /// 本实现要求 val 是合法 `i32`（服务层先解析），非法即报
    /// `field 的值有误`，不会产生 MySQL 的截断告警。
    pub async fn update_sort_order(&self, buyer_show_id: i32, val: i32) -> Result<u64> {
        let res = sqlx::query("UPDATE \"buyer_show\" SET sort_order = $1 WHERE buyer_show_id = $2")
            .bind(val)
            .bind(buyer_show_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— buyer_show_gallery / buyer_show_video —————————————————————

    /// 查某买家秀的全部图片（Java `detail` 用：`eq(buyerShowId, id).list()`，无排序）。
    pub async fn find_gallery(&self, buyer_show_id: i32) -> Result<Vec<BuyerShowGallery>> {
        sqlx::query_as::<_, BuyerShowGallery>(
            "SELECT * FROM \"buyer_show_gallery\" WHERE buyer_show_id = $1",
        )
        .bind(buyer_show_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量查图片，排序口径与 Java `getBuyerShowDTO` 一致：
    /// `ORDER BY sort_order ASC, pic_id ASC`（同一 `sort_order` 下顺序稳定）。
    pub async fn find_gallery_in(&self, ids: &[i32]) -> Result<Vec<BuyerShowGallery>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM \"buyer_show_gallery\" WHERE buyer_show_id IN ({}) ORDER BY sort_order ASC, pic_id ASC",
            placeholders(ids.len())
        );
        let mut q = sqlx::query_as::<_, BuyerShowGallery>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 查某买家秀的全部视频（Java `detail` 用，无排序）。
    pub async fn find_videos(&self, buyer_show_id: i32) -> Result<Vec<BuyerShowVideo>> {
        sqlx::query_as::<_, BuyerShowVideo>(
            "SELECT * FROM \"buyer_show_video\" WHERE buyer_show_id = $1",
        )
        .bind(buyer_show_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量查视频，排序口径与 Java `getBuyerShowDTO` 一致：
    /// `ORDER BY sort_order ASC, id ASC`。
    pub async fn find_videos_in(&self, ids: &[i32]) -> Result<Vec<BuyerShowVideo>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM \"buyer_show_video\" WHERE buyer_show_id IN ({}) ORDER BY sort_order ASC, id ASC",
            placeholders(ids.len())
        );
        let mut q = sqlx::query_as::<_, BuyerShowVideo>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— buyer_show_comment / comment_like —————————————————————

    /// 评论数按 `buyer_show_id` 分组统计（Java `list` 的 `groupingBy(counting)`）。
    pub async fn count_comments_by_show(&self, ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT buyer_show_id, COUNT(*) FROM \"buyer_show_comment\" WHERE buyer_show_id IN ({}) GROUP BY buyer_show_id",
            placeholders(ids.len())
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 插入管理端代发评论（Java `comment`：`user_id = 0`、`status = 1`、
    /// `is_top = 0`、`is_show = 1`、`parent_id = 0`）。
    pub async fn insert_admin_comment(&self, row: &NewAdminComment) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO \"buyer_show_comment\" (parent_id, buyer_show_id, status, is_top, is_show, user_id, user_name, user_photo, content, add_time) VALUES (0, $1, 1, 0, 1, 0, $2, $3, $4, $5) RETURNING buyer_show_comment_id",
        )
        .bind(row.buyer_show_id)
        .bind(&row.user_name)
        .bind(&row.user_photo)
        .bind(row.content.as_deref())
        .bind(row.add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    // ————————————————————— buyer_show_like / favorite —————————————————————

    /// 点赞数按 `buyer_show_id` 分组统计。
    ///
    /// Java `getBuyerShowDTO` 把 `buyer_show_like` 整表按 ID 集合拉回内存再
    /// `filter(...).count()`，本实现改为 SQL 聚合 —— 结果**数值等价**，
    /// 且避免把当页所有点赞行拉进内存。
    pub async fn count_likes_by_show(&self, ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT buyer_show_id, COUNT(*) FROM \"buyer_show_like\" WHERE buyer_show_id IN ({}) GROUP BY buyer_show_id",
            placeholders(ids.len())
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— buyer_show_group —————————————————————

    /// 全部分组（Java `buyerShowGroupService.list()`，无过滤、无排序）。
    pub async fn find_all_groups(&self) -> Result<Vec<BuyerShowGroup>> {
        sqlx::query_as::<_, BuyerShowGroup>("SELECT * FROM \"buyer_show_group\"")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 ID 集合查分组（Java `detail`：`in(buyerShowGroupId, groupIdList)`）。
    ///
    /// ⚠️ 返回顺序是**数据库顺序**，Java 亦然（`in(...).list()` 无 `orderBy`），
    /// 因此 `groupNameList` 的顺序不保证与 `groupIdList` 一致 —— 这是原系统行为。
    pub async fn find_groups_in(&self, ids: &[i32]) -> Result<Vec<BuyerShowGroup>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT * FROM \"buyer_show_group\" WHERE buyer_show_group_id IN ({})",
            placeholders(ids.len())
        );
        let mut q = sqlx::query_as::<_, BuyerShowGroup>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— virtual_user_info / user —————————————————————

    /// 写入虚拟用户（Java `VirtualUserInfoService#save`）。
    pub async fn insert_virtual_user(&self, photo: &str, nickname: &str) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO \"virtual_user_info\" (photo, nickname) VALUES ($1, $2) RETURNING id",
        )
        .bind(photo)
        .bind(nickname)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 随机取一个虚拟用户（对应 Java `getBuyerInfo` 的
    /// `lambdaQuery().last("ORDER BY RAND() LIMIT 1").one()`）。
    ///
    /// **实现差异**：不使用 `ORDER BY RAND()`，改为「先 `COUNT(*)`，再取
    /// `LIMIT 1 OFFSET <0..count)` 的随机偏移」。两者都是均匀随机取一行，
    /// 但后者不需要对整表做随机排序（`ORDER BY RAND()` 在 `virtual_user_info`
    /// 这种小表上无所谓，在大表上是全表排序）。
    ///
    /// 另外 `tools/check_sql_columns.py` 会把 `ORDER BY RAND()` 里的 `RAND`
    /// 误报为未知列，改用 `LIMIT/OFFSET` 也顺带消除了该噪音。
    pub async fn find_random_virtual_user(&self) -> Result<Option<VirtualUserInfo>> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM \"virtual_user_info\"")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let total = count.0.max(0) as u64;
        if total == 0 {
            return Ok(None);
        }
        let offset = random_offset(total);
        sqlx::query_as::<_, VirtualUserInfo>(
            "SELECT id, photo, nickname FROM \"virtual_user_info\" ORDER BY id ASC LIMIT 1 OFFSET $1",
        )
        .bind(offset as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 批量查用户（Java `userService.lambdaQuery().in(userId, ...).list()`）。
    ///
    /// 只取展示需要的 `user_id` / `username` / `avatar`
    /// （Java 用的是 `User.getName()` 与 `User.getAvatar()`，在本项目里就是
    /// `user.username` 与 `user.avatar`），避免拉回 `user` 表全部 36 列。
    ///
    /// 返回 `(user_id, username, avatar)`。
    pub async fn find_users(&self, ids: &[i32]) -> Result<Vec<(i32, String, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT user_id, username, avatar FROM \"user\" WHERE user_id IN ({})",
            placeholders(ids.len())
        );
        let mut q = sqlx::query_as::<_, (i32, String, String)>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// `?,?,?` 占位符（长度由调用方控制，恒为合法 SQL 片段）。
fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

/// 在给定事务里「全删 + 重插」某买家秀的素材。
///
/// 顺序与 Java `create` / `update` 一致：**先视频、后图片**
/// （`saveOrUpdateBuyerShowVideo` → `saveOrUpdateBuyerShowGallery`）。
async fn write_assets(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    buyer_show_id: i32,
    assets: &BuyerShowAssets,
) -> Result<()> {
    sqlx::query("DELETE FROM \"buyer_show_video\" WHERE buyer_show_id = $1")
        .bind(buyer_show_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
    for row in &assets.videos {
        sqlx::query(
            "INSERT INTO \"buyer_show_video\" (video_id, buyer_show_id, video_url, video_cover, format, sort_order) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(row.video_id)
        .bind(buyer_show_id)
        .bind(&row.video_url)
        .bind(&row.video_cover)
        .bind(&row.format)
        .bind(row.sort_order)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
    }

    sqlx::query("DELETE FROM \"buyer_show_gallery\" WHERE buyer_show_id = $1")
        .bind(buyer_show_id)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
    for row in &assets.gallery {
        sqlx::query(
            "INSERT INTO \"buyer_show_gallery\" (buyer_show_id, pic_url, pic_desc, pic_thumb, pic_original, pic_large, sort_order) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(buyer_show_id)
        .bind(&row.pic_url)
        .bind(&row.pic_desc)
        .bind(&row.pic_thumb)
        .bind(&row.pic_original)
        .bind(&row.pic_large)
        .bind(row.sort_order)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
    }
    Ok(())
}

/// 去重 + 升序（对应 Java `deleteAll` 的 `ids.stream().distinct().sorted()`）。
#[must_use]
pub fn sort_dedup(ids: &[i32]) -> Vec<i32> {
    let mut v = ids.to_vec();
    v.sort_unstable();
    v.dedup();
    v
}

/// `[0, total)` 内的伪随机偏移（用于 [`BuyerShowRepository::find_random_virtual_user`]）。
///
/// 不引入新 crate（`rand` 只在 `nextrade-common` 内部使用），
/// 用「系统时钟纳秒 + 进程内递增计数」做种子即可满足「随机挑一个虚拟买家」
/// 这个展示性需求（Java 侧也只是 `ORDER BY RAND()`）。
fn random_offset(total: u64) -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    // splitmix64 收尾，打散时间低位
    let mut z = nanos
        .wrapping_add(seq.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    z % total
}

/// 把 Java 属性名 / 列名映射为 `buyer_show` 的排序列名。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`：遍历 MyBatis-Plus
/// `TableInfo` 的字段列表，**属性名或列名**匹配即返回列名，匹配不到则回落主键列
/// （原码注释：防 SQL 注入）。
///
/// ⚠️ 只放行 `buyer_show` 表真实存在的列；未知字段与 Java 一样回落
/// `buyer_show_id`，因此不存在注入面。
#[must_use]
pub fn resolve_sort_column(sort_field: Option<&str>) -> &'static str {
    let f = sort_field.map(str::trim).unwrap_or_default();
    match f {
        // 主键（Java `getKeyProperty()` / `getKeyColumn()`）
        "" | "buyerShowId" | "buyer_show_id" => "buyer_show_id",
        // 其余 `buyer_show` 列（属性名 camelCase ↔ 列名 snake_case）
        "rangeData" | "range_data" => "range_data",
        "userId" | "user_id" => "user_id",
        "buyerPhoto" | "buyer_photo" => "buyer_photo",
        "buyerName" | "buyer_name" => "buyer_name",
        "content" => "content",
        "groupData" | "group_data" => "group_data",
        "addTime" | "add_time" => "add_time",
        "sortOrder" | "sort_order" => "sort_order",
        "status" => "status",
        "isShow" | "is_show" => "is_show",
        // 未知 → 回落主键（与 Java 的防注入回落一致）
        _ => "buyer_show_id",
    }
}

/// 构造 `ORDER BY` 片段（对应 Java `BaseServiceImpl#buildSortOrder(page, sortField, sortOrder)`）。
///
/// Java 语义：`sortOrder` 为 `asc`（大小写不敏感）时升序，否则（含空）**降序**。
#[must_use]
pub fn order_by_clause(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
    let column = resolve_sort_column(sort_field);
    let asc = sort_order
        .map(|s| s.eq_ignore_ascii_case("asc"))
        .unwrap_or(false);
    if asc {
        format!(" ORDER BY {column} ASC")
    } else {
        format!(" ORDER BY {column} DESC")
    }
}

/// 构造买家秀列表的 `WHERE` 片段与绑定值（对应 Java `list` 的查询构造器）。
///
/// 返回的 SQL 片段以 ` WHERE ...` 开头（无条件时为空串），绑定顺序与片段中
/// `?` 的出现顺序一致；条件之间的先后也与 Java 的链式调用顺序一致。
///
/// ⚠️ `goods_id` 以十进制文本插进 `JSON_CONTAINS(group_data, '...')`
/// —— 这正是 Java `queryWrapper.apply("JSON_CONTAINS(group_data, {0})", goodsId.toString())`
/// 的行为：`goodsId` 是 `Long`，`to_string()` 只可能产出数字（或 `-`），无注入面。
#[must_use]
pub fn build_filter(
    q: &BuyerShowListQuery,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    match q.status_view() {
        ListStatusView::All => {}
        ListStatusView::Pending => {
            conds.push("status = ?".into());
            binds.push(Bind::I32(0));
        }
        ListStatusView::ByIsShow(is_show) => {
            conds.push("status <> ?".into());
            binds.push(Bind::I32(0));
            conds.push("is_show = ?".into());
            binds.push(Bind::I32(is_show));
        }
    }

    if let Some(kw) = q.keyword_opt() {
        conds.push("content LIKE CONCAT('%', ?, '%')".into());
        binds.push(Bind::Str(kw.to_string()));
    }

    if let Some(goods_id) = q.goods_id {
        conds.push(format!("group_data::jsonb @> '{goods_id}'::jsonb"));
    }

    if let (Some(start), Some(end)) = (start_ts, end_ts) {
        conds.push("add_time BETWEEN ? AND ?".into());
        binds.push(Bind::I64(start));
        binds.push(Bind::I64(end));
    }

    if conds.is_empty() {
        (String::new(), binds)
    } else {
        (format!(" WHERE {}", conds.join(" AND ")), binds)
    }
}

/// 解析 `yyyy-MM-dd`（或带时间部分）为**当日 00:00:00**（`end_of_day = false`）
/// 或 **当日 23:59:59**（`end_of_day = true`）的秒级时间戳。
///
/// 两种口径都按**本机时区**换算，对应 Java `TigUtils.toTimestampYmd`
/// （Hutool `DateUtil.parse(str, "yyyy-MM-dd").getTime() / 1000`，默认时区）。
///
/// ⚠️ Java `list` 的时间条件是
/// `between(addTime, toTimestampYmd(startTime), toTimestampYmd(endTime))`，
/// **两端都取当日 00:00:00** —— 因此「结束日期当天」的记录实际会被排除。
/// 这是原系统的既有（可疑）行为，本实现用 `end_of_day = false` 逐字复刻，
/// 详见 `crates/services/src/buyer_show.rs` 的 `uncertain_points` 说明。
#[must_use]
pub fn parse_ymd_seconds(s: &str, end_of_day: bool) -> Option<i64> {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let date_part = s.split([' ', 'T']).next().unwrap_or(s);
    let date = NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()?;
    let time = if end_of_day {
        NaiveTime::from_hms_opt(23, 59, 59)?
    } else {
        NaiveTime::from_hms_opt(0, 0, 0)?
    };
    let naive = NaiveDateTime::new(date, time);
    match chrono::Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Some(dt.timestamp()),
        chrono::LocalResult::Ambiguous(a, _) => Some(a.timestamp()),
        chrono::LocalResult::None => None,
    }
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（对应 Java `TigUtils.handelTime`）。
///
/// Java：`time == null || time == 0` 返回空串；其余按**本机时区**格式化。
#[must_use]
pub fn format_time_seconds(secs: i64) -> String {
    if secs <= 0 {
        return String::new();
    }
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 解析 `range_data` / `group_data`（JSON 数组字符串）为 `Vec<i32>`。
///
/// 对应 Java `JSONUtil.toList(str, Integer.class)`。**差异**：Java 在 `list` 里
/// 对 `range_data` 直接 `JSONUtil.toList(...)`，遇到脏数据会抛异常打挂整页；
/// 本实现对空/非法 JSON 返回空表（`detail` / `list` 都不再因为一条脏数据 500）。
/// 见 `uncertain_points`。
#[must_use]
pub fn parse_i32_json_array(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<i32>>(trimmed).unwrap_or_default()
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("买家秀不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_count() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?,?,?");
    }

    #[test]
    fn sort_dedup_matches_java_distinct_sorted() {
        assert_eq!(sort_dedup(&[3, 1, 3, 2, 1]), vec![1, 2, 3]);
        assert!(sort_dedup(&[]).is_empty());
    }

    #[test]
    fn random_offset_stays_in_range() {
        for total in [1u64, 2, 7, 100] {
            for _ in 0..50 {
                assert!(random_offset(total) < total, "total={total}");
            }
        }
        // 多次调用应能取到不止一个值（极小概率全等，用 200 次采样降低误报）
        let picks: std::collections::HashSet<u64> = (0..200).map(|_| random_offset(1000)).collect();
        assert!(picks.len() > 1, "随机偏移应具有变化，实际 {picks:?}");
    }

    #[test]
    fn sort_column_maps_property_and_column_and_falls_back_to_pk() {
        assert_eq!(resolve_sort_column(None), "buyer_show_id");
        assert_eq!(resolve_sort_column(Some("")), "buyer_show_id");
        assert_eq!(resolve_sort_column(Some(" buyerShowId ")), "buyer_show_id");
        assert_eq!(resolve_sort_column(Some("buyer_show_id")), "buyer_show_id");
        assert_eq!(resolve_sort_column(Some("sortOrder")), "sort_order");
        assert_eq!(resolve_sort_column(Some("sort_order")), "sort_order");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("isShow")), "is_show");
        // 未知属性回落主键（Java `getColumnByPropertyName` 的防注入回落）
        assert_eq!(
            resolve_sort_column(Some("1; DROP TABLE x")),
            "buyer_show_id"
        );
        assert_eq!(resolve_sort_column(Some("nonexistent")), "buyer_show_id");
    }

    #[test]
    fn order_clause_defaults_to_desc() {
        assert_eq!(order_by_clause(None, None), " ORDER BY buyer_show_id DESC");
        assert_eq!(
            order_by_clause(Some("sortOrder"), Some("")),
            " ORDER BY sort_order DESC"
        );
        assert_eq!(
            order_by_clause(Some("sortOrder"), Some("ASC")),
            " ORDER BY sort_order ASC"
        );
        assert_eq!(
            order_by_clause(Some("sortOrder"), Some("DESC")),
            " ORDER BY sort_order DESC"
        );
        // Java 只认 "asc"，其余一律降序
        assert_eq!(
            order_by_clause(Some("addTime"), Some("ascending")),
            " ORDER BY add_time DESC"
        );
    }

    fn query_with(
        status: Option<i32>,
        keyword: Option<&str>,
        goods_id: Option<i64>,
    ) -> BuyerShowListQuery {
        BuyerShowListQuery {
            status,
            keyword: keyword.map(String::from),
            goods_id,
            ..Default::default()
        }
    }

    #[test]
    fn filter_no_conditions_is_empty() {
        let (sql, binds) = build_filter(&query_with(None, None, None), None, None);
        assert!(sql.is_empty());
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_status_pending_equals_zero() {
        let (sql, binds) = build_filter(&query_with(Some(-1), None, None), None, None);
        assert_eq!(sql, " WHERE status = $1");
        assert_eq!(binds.len(), 1);
        assert!(matches!(binds[0], Bind::I32(0)));
    }

    #[test]
    fn filter_status_public_is_ne_zero_and_is_show_eq() {
        let (sql, binds) = build_filter(&query_with(Some(1), None, None), None, None);
        assert_eq!(sql, " WHERE status <> $1 AND is_show = $2");
        assert_eq!(binds.len(), 2);
        assert!(matches!(binds[0], Bind::I32(0)));
        assert!(matches!(binds[1], Bind::I32(1)));

        let (sql, binds) = build_filter(&query_with(Some(0), None, None), None, None);
        assert_eq!(sql, " WHERE status <> $1 AND is_show = $2");
        assert!(matches!(binds[1], Bind::I32(0)));
    }

    #[test]
    fn filter_keyword_uses_concat_like() {
        let (sql, binds) = build_filter(&query_with(None, Some("鞋"), None), None, None);
        assert_eq!(sql, " WHERE content LIKE CONCAT('%', $1, '%')");
        match &binds[0] {
            Bind::Str(s) => assert_eq!(s, "鞋"),
            other => panic!("应为字符串绑定，实际 {other:?}"),
        }
    }

    #[test]
    fn filter_goods_id_uses_json_contains_literal() {
        let (sql, binds) = build_filter(&query_with(None, None, Some(42)), None, None);
        assert_eq!(sql, " WHERE group_data::jsonb @> '42'::jsonb");
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_time_range_requires_both_bounds() {
        let q = query_with(None, None, None);
        let (sql, binds) = build_filter(&q, Some(100), Some(200));
        assert_eq!(sql, " WHERE add_time BETWEEN $1 AND $2");
        assert_eq!(binds.len(), 2);

        // Java 只在 startTime 与 endTime 都非空时才 between
        let (sql, binds) = build_filter(&q, Some(100), None);
        assert!(sql.is_empty());
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_combines_all_conditions_in_java_order() {
        let (sql, binds) =
            build_filter(&query_with(Some(1), Some("k"), Some(7)), Some(10), Some(20));
        assert_eq!(
            sql,
            " WHERE status <> $1 AND is_show = $2 AND content LIKE CONCAT('%', $3, '%') \
             AND group_data::jsonb @> '7'::jsonb AND add_time BETWEEN $4 AND $5"
        );
        assert_eq!(binds.len(), 5);
    }

    #[test]
    fn parse_ymd_seconds_accepts_plain_and_datetime() {
        let a = parse_ymd_seconds("2025-01-01", false).expect("parse");
        let b = parse_ymd_seconds("2025-01-01 00:00:00", false).expect("parse");
        assert_eq!(a, b);
        assert_eq!(parse_ymd_seconds("", false), None);
        assert_eq!(parse_ymd_seconds("not-a-date", false), None);
        let end = parse_ymd_seconds("2025-01-01", true).expect("parse");
        assert_eq!(end - a, 86_399);
    }

    #[test]
    fn format_time_zero_is_empty_like_java() {
        assert_eq!(format_time_seconds(0), "");
        assert_eq!(format_time_seconds(-5), "");
        // 与 Java 一样按**本机时区**格式化，故期望值由本地时间反推（跨时区稳定）
        let naive = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .expect("date")
            .and_hms_opt(8, 0, 0)
            .expect("time");
        use chrono::TimeZone;
        let secs = match chrono::Local.from_local_datetime(&naive) {
            chrono::LocalResult::Single(dt) => dt.timestamp(),
            chrono::LocalResult::Ambiguous(a, _) => a.timestamp(),
            chrono::LocalResult::None => panic!("本机时区不支持该本地时间"),
        };
        assert_eq!(format_time_seconds(secs), "2025-01-01 08:00:00");
        assert_eq!(format_time_seconds(secs + 1), "2025-01-01 08:00:01");
    }

    #[test]
    fn parse_json_array_variants() {
        assert_eq!(parse_i32_json_array(Some("[1,2,3]")), vec![1, 2, 3]);
        assert!(parse_i32_json_array(Some("[]")).is_empty());
        assert!(parse_i32_json_array(Some("")).is_empty());
        assert!(parse_i32_json_array(Some("not json")).is_empty());
        assert!(parse_i32_json_array(None).is_empty());
    }
}
