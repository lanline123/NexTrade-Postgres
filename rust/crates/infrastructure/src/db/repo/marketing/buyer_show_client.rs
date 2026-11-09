//! 买家秀前台（marketing::buyer_show_client）仓储：`buyer_show` / `buyer_show_video` /
//! `buyer_show_gallery` / `buyer_show_like` / `buyer_show_favorite` /
//! `buyer_show_comment` / `comment_like` / `buyer_show_group` / `user_follow` /
//! `product_group` / `product` / `product_sku` / `user` / `config`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table <表名>` 可复核）：
//! - `buyer_show`：`buyer_show_id` / `range_data` / `user_id` / `buyer_photo` /
//!   `buyer_name` / `content` / `group_data` / `add_time` / `sort_order` /
//!   `status` / `is_show`
//! - `buyer_show_video`：`id` / `video_id` / `buyer_show_id` / `video_url` /
//!   `video_cover` / `format` / `sort_order`
//! - `buyer_show_gallery`：`pic_id` / `buyer_show_id` / `pic_url` / `pic_desc` /
//!   `pic_thumb` / `pic_original` / `pic_large` / `sort_order`
//! - `buyer_show_like`：`id` / `buyer_show_id` / `user_id` / `add_time`
//! - `buyer_show_favorite`：`id` / `buyer_show_id` / `user_id` / `add_time`
//! - `buyer_show_comment`：`buyer_show_comment_id` / `parent_id` / `buyer_show_id` /
//!   `status` / `is_top` / `is_show` / `user_id` / `user_name` / `user_photo` /
//!   `content` / `add_time` / `comment_pic`
//! - `comment_like`：`id` / `comment_id` / `user_id` / `add_time`
//! - `buyer_show_group`：`buyer_show_group_id` / `group_name` / `product_group_id` /
//!   `shop_id`
//! - `product_group`：`product_group_id` / `product_group_name` / `product_ids` / …
//! - `user_follow`：`id` / `user_id` / `author_id` / `add_time`
//! - `config`：`biz_code` / `biz_val` / `is_del`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//! 动态片段只拼接**固定字面量**，所有外部值一律 `?` 占位 + 有序绑定
//! （[`Bind`]），不做字符串插值，避免注入。
//!
//! # 两个必须照抄的 MySQL 细节
//!
//! 1. `JSON_CONTAINS` 的候选值必须是**可解析为 JSON 的字符串**
//!    （Java XML 注释原文：「绑定为数字会报 Invalid data type for JSON data」）。
//!    Java 用 `<bind name="groupIdJson" value="groupId.toString()"/>`，
//!    本实现绑定 `String` 并显式 `CAST(? AS JSON)` 让 MySQL 按 JSON 标量比较，
//!    与 Java 行为一致（数值型 JSON 标量相等即命中）。
//! 2. `buyer_show_video` / `buyer_show_gallery` 的素材排序必须**稳定**：
//!    Java `getBuyerShowDTO` 显式
//!    `orderByAsc(sortOrder, id)` / `orderByAsc(sortOrder, picId)`，
//!    否则同一 `sort_order` 下首视频/图片数组会漂移。

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{
    BuyerShow, BuyerShowComment, BuyerShowFavorite, BuyerShowGallery, BuyerShowGroup, BuyerShowLike,
    BuyerShowVideo, CommentLike, User, UserFollow,
};
use nextrade_core::models::marketing::buyer_show_client::{
    BuyerShowCommentPageClientQuery, BuyerShowPageClientQuery, BuyerShowProductVO,
    BuyerShowVideoFeedClientQuery,
};
use nextrade_core::{Error, Result};

/// 待绑定参数（按 SQL 中 `?` 的出现顺序 1:1 对应）。
#[derive(Debug, Clone)]
enum Bind {
    /// 整型
    I32(i32),
    /// 大整数（`add_time` / 分组 id）
    I64(i64),
    /// 字符串（含 JSON 标量文本）
    Str(String),
    /// 无符号整数（`LIMIT` / `OFFSET`）
    U64(u64),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以适配不同返回行类型）。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::I64(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
                Bind::U64(v) => q.bind(*v as i64),
            };
        }
        q
    }};
}

/// `id IN (...)` 占位串（`?` 个数由调用方保证与绑定值一致）。
fn placeholders(n: usize) -> String {
    let mut s = String::with_capacity(n.saturating_mul(2));
    for i in 0..n {
        if i > 0 {
            s.push_str(", ");
        }
        s.push('?');
    }
    s
}

/// 视频流复合游标锚点（Java `BuyerShowServiceImpl#buildAnchor`）。
///
/// 排序键三元组：`sort_order ASC, add_time DESC, buyer_show_id DESC`；
/// `inclusive` 由调用方给出（首屏含锚点自身，续页严格取锚点之后）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedAnchor {
    /// 锚点行的 `sort_order`（Java `buildAnchor` 要求非 `null`）
    pub sort_order: i32,
    /// 锚点行的 `add_time`
    pub add_time: i64,
    /// 锚点行的主键
    pub buyer_show_id: i32,
}

/// 仓库实现。
#[derive(Clone)]
pub struct BuyerShowClientRepository {
    pool: PgPool,
}

impl BuyerShowClientRepository {
    /// 构造仓库。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ═════════════════════ 买家秀主表 ═════════════════════

    /// 按主键取一条（Java `this.getById(id)`）。
    pub async fn find_by_id(&self, buyer_show_id: i32) -> Result<Option<BuyerShow>> {
        sqlx::query_as::<_, BuyerShow>("SELECT * FROM buyer_show WHERE buyer_show_id = $1 LIMIT 1")
            .bind(buyer_show_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// C 端买家秀列表分页（Java `BuyerShowServiceImpl#clientList`）。
    ///
    /// Java 查询构造器（`LambdaQueryWrapper`）：
    /// ```text
    /// (user_id = currentUserId OR is_show = 1)
    /// [keyword 非空] content LIKE %keyword%
    /// [goodsId 非空] JSON_CONTAINS(group_data, goodsId)
    /// [productId 非空] JSON_CONTAINS(range_data, productId)
    /// ORDER BY sort_order ASC, add_time DESC, buyer_show_id DESC
    /// LIMIT size OFFSET (page-1)*size
    /// ```
    ///
    /// ⚠️ `user_id = currentUserId` 在**游客**场景下等价于
    /// `user_id = 0`（Java `getCurrentUserId()` 返回 `0`，不是 `null`），
    /// 即非真实用户（`user_id = 0`）的买家秀对游客可见——原实现如此，照抄。
    ///
    /// ⚠️ 排序**固定**：Java 显式 `page.setOrders(buildClientFeedOrders())`，
    /// 不接受 `sortField` / `sortOrder` 覆盖（否则宫格点入视频流的
    /// `startId` 锚点会落错位置）。
    pub async fn paginate_client_list(
        &self,
        q: &BuyerShowPageClientQuery,
        current_user_id: i32,
    ) -> Result<(u64, Vec<BuyerShow>)> {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();

        conds.push("(user_id = ? OR is_show = 1)".to_string());
        binds.push(Bind::I32(current_user_id));

        if let Some(keyword) = q.keyword.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
            conds.push("content LIKE ?".to_string());
            binds.push(Bind::Str(format!("%{keyword}%")));
        }
        if let Some(goods_id) = q.goods_id {
            conds.push("JSON_CONTAINS(group_data, CAST(? AS JSON))".to_string());
            binds.push(Bind::Str(goods_id.to_string()));
        }
        if let Some(product_id) = q.product_id {
            conds.push("JSON_CONTAINS(range_data, CAST(? AS JSON))".to_string());
            binds.push(Bind::Str(product_id.to_string()));
        }

        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        let total_sql = format!("SELECT COUNT(*) FROM buyer_show{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let page_size = q.page_size();
        let offset = (q.page_no() - 1).saturating_mul(page_size);
        let rows_sql = format!(
            "SELECT * FROM buyer_show{where_sql} \
             ORDER BY sort_order ASC, add_time DESC, buyer_show_id DESC LIMIT ? OFFSET ?"
        );
        let rows = bind_all!(sqlx::query_as::<_, BuyerShow>(&rows_sql), binds)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((u64::try_from(total.0.max(0)).unwrap_or(0), rows))
    }

    /// 视频流游标查询（Java `BuyerShowMapper.selectVideoFeed`，
    /// XML 位于 `mapper/promotion/BuyerShowMapper.xml`）。
    ///
    /// 视频类判定（与 Java XML 逐字对齐）：
    /// ```text
    /// EXISTS (SELECT 1 FROM buyer_show_video v WHERE v.buyer_show_id = bs.buyer_show_id)
    /// AND (SELECT MIN(COALESCE(v.sort_order,0)) ...) <= COALESCE((SELECT MIN(COALESCE(g.sort_order,0)) ...), 2147483647)
    /// ```
    ///
    /// 可见性（`<choose>` 两分支）：
    /// - `user_id > 0`：`(user_id = $2 AND status IN (0,1)) OR (is_show = 1 AND status = 1)`
    /// - 否则：`is_show = 1 AND status = 1`
    ///
    /// 复合游标（\"anchorId\" 非空时）：
    /// ```text
    /// sort_order > $3 OR (sort_order = $4 AND add_time < $5)
    ///   OR (sort_order = $6 AND add_time = $7 AND buyer_show_id <=|< $8)
    /// ```
    ///
    /// ⚠️ `buyer_show.sort_order` 在库中可空：Java 的 \"buildAnchor\" 在该列为
    /// \"null\" 时返回 \"null\" 锚点（视为从头开始），故本函数只在
    /// [\"FeedAnchor\"] 已构造成功时拼接该片段。
    pub async fn fetch_video_feed(
        &self,
        q: &BuyerShowVideoFeedClientQuery,
        user_id: i32,
        anchor: Option<FeedAnchor>,
        inclusive: bool,
        limit: i32,
    ) -> Result<Vec<BuyerShow>> {
        let mut sql = String::from(
            "SELECT bs.buyer_show_id, bs.range_data, bs.user_id, bs.buyer_photo, bs.buyer_name, \
             bs.content, bs.group_data, bs.add_time, bs.sort_order, bs.is_show, bs.status \
             FROM buyer_show bs \
             WHERE EXISTS (SELECT 1 FROM buyer_show_video v WHERE v.buyer_show_id = bs.buyer_show_id) \
             AND (SELECT MIN(COALESCE(v.sort_order, 0)) FROM buyer_show_video v \
                  WHERE v.buyer_show_id = bs.buyer_show_id) \
                 <= COALESCE((SELECT MIN(COALESCE(g.sort_order, 0)) FROM buyer_show_gallery g \
                              WHERE g.buyer_show_id = bs.buyer_show_id), 2147483647)",
        );
        let mut binds: Vec<Bind> = Vec::new();

        if user_id > 0 {
            sql.push_str(
                " AND ((bs.user_id = ? AND bs.status IN (0, 1)) \
                  OR (bs.is_show = 1 AND bs.status = 1))",
            );
            binds.push(Bind::I32(user_id));
        } else {
            sql.push_str(" AND bs.is_show = 1 AND bs.status = 1");
        }

        if let Some(keyword) = q.keyword.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
            sql.push_str(" AND bs.content LIKE ?");
            binds.push(Bind::Str(format!("%{keyword}%")));
        }
        if let Some(group_id) = q.group_id {
            sql.push_str(" AND JSON_CONTAINS(bs.group_data, CAST(? AS JSON))");
            binds.push(Bind::Str(group_id.to_string()));
        }
        if let Some(a) = anchor {
            // inclusive=true 时包含锚点自身（在比较符处切换，**不是**两条 SQL）
            let id_op = if inclusive { "<=" } else { "<" };
            sql.push_str(&format!(
                " AND (bs.sort_order > ? \
                   OR (bs.sort_order = ? AND bs.add_time < ?) \
                   OR (bs.sort_order = ? AND bs.add_time = ? AND bs.buyer_show_id {id_op} ?))"
            ));
            binds.push(Bind::I32(a.sort_order));
            binds.push(Bind::I32(a.sort_order));
            binds.push(Bind::I64(a.add_time));
            binds.push(Bind::I32(a.sort_order));
            binds.push(Bind::I64(a.add_time));
            binds.push(Bind::I32(a.buyer_show_id));
        }

        sql.push_str(" ORDER BY bs.sort_order ASC, bs.add_time DESC, bs.buyer_show_id DESC LIMIT ?");
        let rows = bind_all!(sqlx::query_as::<_, BuyerShow>(&sql), binds)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows)
    }

    // ═════════════════════ 素材（视频 / 相册） ═════════════════════

    /// 追加一条视频素材（Java `buyerShowVideoService.save(entity)`；
    /// \"id\" 由数据库自增，\"sort_order\" 由调用方兜底为列表下标）。
    pub async fn insert_video(
        &self,
        buyer_show_id: i32,
        video_id: i32,
        video_url: &str,
        video_cover: &str,
        format: &str,
        sort_order: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO buyer_show_video \
             (video_id, buyer_show_id, video_url, video_cover, format, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(video_id)
        .bind(buyer_show_id)
        .bind(video_url)
        .bind(video_cover)
        .bind(format)
        .bind(sort_order)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除某买家秀的全部视频素材
    /// （Java \"saveOrUpdateBuyerShowVideo\" 第 1 步）。
    pub async fn delete_videos_of(&self, buyer_show_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM buyer_show_video WHERE buyer_show_id = ?")
            .bind(buyer_show_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 追加一条相册图片（Java `buyerShowGalleryService.saveBatch(list)` 的单行）。
    ///
    /// \"pic_url\" / \"pic_large\" 由服务层按存储尺寸后缀拼好传入
    /// （见服务层 \"image_size_suffix\"）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gallery(
        &self,
        buyer_show_id: i32,
        pic_url: &str,
        pic_desc: &str,
        pic_thumb: &str,
        pic_original: &str,
        pic_large: &str,
        sort_order: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO buyer_show_gallery \
             (buyer_show_id, pic_url, pic_desc, pic_thumb, pic_original, pic_large, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(buyer_show_id)
        .bind(pic_url)
        .bind(pic_desc)
        .bind(pic_thumb)
        .bind(pic_original)
        .bind(pic_large)
        .bind(sort_order)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除某买家秀的全部相册图片
    /// （Java \"saveOrUpdateBuyerShowGallery\" 第 1 步）。
    pub async fn delete_gallery_of(&self, buyer_show_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM buyer_show_gallery WHERE buyer_show_id = ?")
            .bind(buyer_show_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 新建买家秀主表行，返回自增主键（Java `this.save(buyerShow)`）。
    ///
    /// \"range_data\" / \"group_data\" 由服务层序列化为 JSON 文本；
    /// \"group_data\" 为 \"None\" 时落 \"NULL\"（Java `setGroupData(null)`）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_buyer_show(
        &self,
        range_data: &str,
        user_id: i32,
        buyer_photo: &str,
        buyer_name: &str,
        content: Option<&str>,
        group_data: Option<&str>,
        add_time: i64,
        sort_order: i32,
        status: i32,
        is_show: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO buyer_show \
             (range_data, user_id, buyer_photo, buyer_name, content, group_data, \
              add_time, sort_order, status, is_show) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING buyer_show_id",
        )
        .bind(range_data)
        .bind(user_id)
        .bind(buyer_photo)
        .bind(buyer_name)
        .bind(content)
        .bind(group_data)
        .bind(add_time)
        .bind(sort_order)
        .bind(status)
        .bind(is_show)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 批量取素材（列表 / 视频流 / 详情页共用，
    /// 对应 Java \"getBuyerShowDTO\" 的两条 `in(...)` 查询）。
    ///
    /// 排序与 Java 一致：视频 `sort_order ASC, id ASC`；
    /// 相册 `sort_order ASC, pic_id ASC`（**稳定**，避免同序漂移）。
    /// 返回 `(videos, galleries)`。
    pub async fn find_assets_by_buyer_show_ids(
        &self,
        ids: &[i32],
    ) -> Result<(Vec<BuyerShowVideo>, Vec<BuyerShowGallery>)> {
        if ids.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let ph = placeholders(ids.len());

        let sql_video = format!(
            "SELECT * FROM buyer_show_video WHERE buyer_show_id IN ({ph}) \
             ORDER BY sort_order ASC, id ASC"
        );
        let mut vq = sqlx::query_as::<_, BuyerShowVideo>(&sql_video);
        for id in ids {
            vq = vq.bind(*id);
        }
        let videos = vq.fetch_all(&self.pool).await.map_err(map_err)?;

        let sql_gallery = format!(
            "SELECT * FROM buyer_show_gallery WHERE buyer_show_id IN ({ph}) \
             ORDER BY sort_order ASC, pic_id ASC"
        );
        let mut gq = sqlx::query_as::<_, BuyerShowGallery>(&sql_gallery);
        for id in ids {
            gq = gq.bind(*id);
        }
        let galleries = gq.fetch_all(&self.pool).await.map_err(map_err)?;

        Ok((videos, galleries))
    }

    /// 单个买家秀的视频素材（Java `detail` / `share` 的
    /// `eq(buyerShowId, ?).list()`，**无 ORDER BY**；这里按主键升序返回以保证确定）。
    pub async fn find_videos_of(&self, buyer_show_id: i32) -> Result<Vec<BuyerShowVideo>> {
        sqlx::query_as::<_, BuyerShowVideo>(
            "SELECT * FROM buyer_show_video WHERE buyer_show_id = ? ORDER BY id ASC",
        )
        .bind(buyer_show_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 单个买家秀的相册（Java `detail` 的 `eq(buyerShowId, ?).list()`，
    /// 无 ORDER BY；这里按主键升序返回以保证确定）。
    pub async fn find_gallery_of(&self, buyer_show_id: i32) -> Result<Vec<BuyerShowGallery>> {
        sqlx::query_as::<_, BuyerShowGallery>(
            "SELECT * FROM buyer_show_gallery WHERE buyer_show_id = ? ORDER BY pic_id ASC",
        )
        .bind(buyer_show_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取排序第一张相册图（Java `getSharePageHtml`：
    /// `orderByAsc(sortOrder).last("limit 1")`）。
    pub async fn find_first_gallery(
        &self,
        buyer_show_id: i32,
    ) -> Result<Option<BuyerShowGallery>> {
        sqlx::query_as::<_, BuyerShowGallery>(
            "SELECT * FROM buyer_show_gallery WHERE buyer_show_id = ? \
             ORDER BY sort_order ASC, pic_id ASC LIMIT 1",
        )
        .bind(buyer_show_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取第一段视频（Java `getSharePageHtml` 的
    /// `buyerShowVideoService...last("limit 1")`；**无 ORDER BY**，
    /// 这里按主键升序取第一条以保证确定）。
    pub async fn find_first_video(&self, buyer_show_id: i32) -> Result<Option<BuyerShowVideo>> {
        sqlx::query_as::<_, BuyerShowVideo>(
            "SELECT * FROM buyer_show_video WHERE buyer_show_id = ? ORDER BY id ASC LIMIT 1",
        )
        .bind(buyer_show_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ═════════════════════ 点赞 / 收藏 / 关注 ═════════════════════

    /// 批量取点赞行（Java `getBuyerShowDTO` 的 `in(buyerShowId, ids)`）。
    pub async fn find_likes_by_buyer_show_ids(
        &self,
        ids: &[i32],
    ) -> Result<Vec<BuyerShowLike>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT * FROM buyer_show_like WHERE buyer_show_id IN ({ph}) ORDER BY id ASC"
        );
        let mut q = sqlx::query_as::<_, BuyerShowLike>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取收藏行（Java `videoFeed` 的 `in(buyerShowId, ids)`）。
    pub async fn find_favorites_by_buyer_show_ids(
        &self,
        ids: &[i32],
    ) -> Result<Vec<BuyerShowFavorite>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT * FROM buyer_show_favorite WHERE buyer_show_id IN ({ph}) ORDER BY id ASC"
        );
        let mut q = sqlx::query_as::<_, BuyerShowFavorite>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 插入点赞行（Java \"like\" 的 `buyerShowLikeService.save(...)`，
    /// **不查重**——照抄原实现）。
    pub async fn insert_like(
        &self,
        buyer_show_id: i32,
        user_id: i32,
        add_time: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO buyer_show_like (buyer_show_id, user_id, add_time) VALUES (?, ?, ?)",
        )
        .bind(buyer_show_id)
        .bind(user_id)
        .bind(add_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除点赞行（Java `lambdaUpdate().eq(buyerShowId).eq(userId).remove()`）。
    pub async fn delete_like(&self, buyer_show_id: i32, user_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM buyer_show_like WHERE buyer_show_id = ? AND user_id = ?")
            .bind(buyer_show_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 某买家秀的点赞总数（Java `detail` 的 `count()`）。
    pub async fn count_likes(&self, buyer_show_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM buyer_show_like WHERE buyer_show_id = ?")
                .bind(buyer_show_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 某买家秀 + 用户是否已点赞（Java `detail` 的 `count()`）。
    pub async fn count_like_of_user(&self, buyer_show_id: i32, user_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM buyer_show_like WHERE buyer_show_id = ? AND user_id = ?",
        )
        .bind(buyer_show_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 某买家秀 + 用户的收藏行数（Java `favorite` 的 `count() > 0` 判定）。
    pub async fn count_favorite_of_user(
        &self,
        buyer_show_id: i32,
        user_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM buyer_show_favorite WHERE buyer_show_id = ? AND user_id = ?",
        )
        .bind(buyer_show_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 插入收藏行（Java `buyerShowFavoriteService.save(...)`）。
    pub async fn insert_favorite(
        &self,
        buyer_show_id: i32,
        user_id: i32,
        add_time: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO buyer_show_favorite (buyer_show_id, user_id, add_time) VALUES (?, ?, ?)",
        )
        .bind(buyer_show_id)
        .bind(user_id)
        .bind(add_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除收藏行（Java `lambdaUpdate().eq(...).eq(...).remove()`）。
    pub async fn delete_favorite(&self, buyer_show_id: i32, user_id: i32) -> Result<u64> {
        let r =
            sqlx::query("DELETE FROM buyer_show_favorite WHERE buyer_show_id = ? AND user_id = ?")
                .bind(buyer_show_id)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 用户是否已关注该作者（Java `followAuthor` 的 `count() > 0`）。
    pub async fn count_follow(&self, user_id: i32, author_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM user_follow WHERE user_id = ? AND author_id = ?")
                .bind(user_id)
                .bind(author_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批量查询当前用户已关注的作者 id
    /// （Java `getFollowedAuthorIds`：`eq(userId).in(authorId, ids)`）。
    pub async fn find_followed_author_ids(
        &self,
        user_id: i32,
        author_ids: &[i32],
    ) -> Result<Vec<i32>> {
        if author_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(author_ids.len());
        let sql = format!(
            "SELECT * FROM user_follow WHERE user_id = ? AND author_id IN ({ph})"
        );
        let mut q = sqlx::query_as::<_, UserFollow>(&sql).bind(user_id);
        for id in author_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.author_id).collect())
    }

    /// 插入关注关系（Java `userFollowService.save(...)`）。
    pub async fn insert_follow(&self, user_id: i32, author_id: i32, add_time: i64) -> Result<()> {
        sqlx::query("INSERT INTO user_follow (user_id, author_id, add_time) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(author_id)
            .bind(add_time)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 删除关注关系（Java `lambdaUpdate().eq(userId).eq(authorId).remove()`）。
    pub async fn delete_follow(&self, user_id: i32, author_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM user_follow WHERE user_id = ? AND author_id = ?")
            .bind(user_id)
            .bind(author_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    // ═════════════════════ 评论 ═════════════════════

    /// 统计某买家秀的可见评论数
    /// （Java `detail`：`eq(buyerShowId).and(userId = ? OR isShow = 1).count()`）。
    pub async fn count_visible_comments(
        &self,
        buyer_show_id: i32,
        user_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM buyer_show_comment \
             WHERE buyer_show_id = ? AND (user_id = ? OR is_show = 1)",
        )
        .bind(buyer_show_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 批量统计各买家秀的可见评论数（Java `videoFeed` 的 `groupingBy(...counting())`）。
    ///
    /// ⚠️ Java 侧还有 `.and(wrapper -> wrapper.eq(userId).or().eq(isShow, YES))`
    /// ——与 `detail` 同一条件，这里一致。
    pub async fn count_visible_comments_by_buyer_show_ids(
        &self,
        ids: &[i32],
        user_id: i32,
    ) -> Result<Vec<(i32, i64)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT buyer_show_id, COUNT(*) FROM buyer_show_comment \
             WHERE buyer_show_id IN ({ph}) AND (user_id = ? OR is_show = 1) \
             GROUP BY buyer_show_id"
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q = q.bind(user_id);
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量统计各父评论的可见回复数
    /// （Java `clientCommentPageList` 的 `in(parentId, ids).and(...)` + `groupingBy`）。
    pub async fn count_visible_replies_by_parent_ids(
        &self,
        parent_ids: &[i32],
        user_id: i32,
    ) -> Result<Vec<(i32, i64)>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(parent_ids.len());
        let sql = format!(
            "SELECT parent_id, COUNT(*) FROM buyer_show_comment \
             WHERE parent_id IN ({ph}) AND (user_id = ? OR is_show = 1) \
             GROUP BY parent_id"
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in parent_ids {
            q = q.bind(*id);
        }
        q = q.bind(user_id);
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量取评论点赞行（Java `clientCommentPageList` 的 `in(commentId, ids)`）。
    pub async fn find_comment_likes_by_comment_ids(
        &self,
        comment_ids: &[i32],
    ) -> Result<Vec<CommentLike>> {
        if comment_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(comment_ids.len());
        let sql = format!(
            "SELECT * FROM comment_like WHERE comment_id IN ({ph}) ORDER BY id ASC"
        );
        let mut q = sqlx::query_as::<_, CommentLike>(&sql);
        for id in comment_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 删除某条评论的全部点赞（用于评论点踩场景，本切片暂未使用，
    /// 保留以便后续评论删除链路复用）。
    pub async fn delete_comment_likes(&self, comment_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM comment_like WHERE comment_id = ?")
            .bind(comment_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 插入评论点赞（Java `commentLikeService.save(...)`，**不查重**）。
    pub async fn insert_comment_like(
        &self,
        comment_id: i32,
        user_id: i32,
        add_time: i64,
    ) -> Result<()> {
        sqlx::query("INSERT INTO comment_like (comment_id, user_id, add_time) VALUES (?, ?, ?)")
            .bind(comment_id)
            .bind(user_id)
            .bind(add_time)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 删除评论点赞（Java `lambdaUpdate().eq(commentId).eq(userId).remove()`）。
    pub async fn delete_comment_like(&self, comment_id: i32, user_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM comment_like WHERE comment_id = ? AND user_id = ?")
            .bind(comment_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 新建评论（Java `buyerShowCommentService.save(entity)`）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_comment(
        &self,
        parent_id: i32,
        buyer_show_id: i32,
        status: i32,
        is_top: i32,
        is_show: i32,
        user_id: i32,
        user_name: &str,
        user_photo: &str,
        content: Option<&str>,
        add_time: i64,
        comment_pic: Option<&str>,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO buyer_show_comment \
             (parent_id, buyer_show_id, status, is_top, is_show, user_id, user_name, \
              user_photo, content, add_time, comment_pic) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING buyer_show_comment_id",
        )
        .bind(parent_id)
        .bind(buyer_show_id)
        .bind(status)
        .bind(is_top)
        .bind(is_show)
        .bind(user_id)
        .bind(user_name)
        .bind(user_photo)
        .bind(content)
        .bind(add_time)
        .bind(comment_pic)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 评论分页（Java \"clientCommentPageList\" 的 \"LambdaQueryWrapper\"）。
    ///
    /// Java 条件（**逐条照抄**）：
    /// ```text
    /// eq(buyerShowId, $2)          // 无条件等值；传 null 时 = NULL 恒不成立
    /// eq(parentId, $3)             // 同上
    /// and(userId = $4 OR isShow = 1)
    /// ORDER BY is_top DESC, add_time DESC
    /// ```
    ///
    /// ⚠️ 未同时传 \"buyerShowId\" 与 \"parentId\" 时，Java 的
    /// `eq(column, null)` 生成 `x = NULL`（恒不成立）→ 空页；
    /// 本实现用 `1 = 0` 复刻该语义（\"LIMIT\" 段仍会执行，结果为空）。
    pub async fn paginate_comments(
        &self,
        q: &BuyerShowCommentPageClientQuery,
        user_id: i32,
    ) -> Result<(u64, Vec<BuyerShowComment>)> {
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<Bind> = Vec::new();

        match (q.buyer_show_id, q.parent_id) {
            (Some(bid), Some(pid)) => {
                conds.push("buyer_show_id = ?".to_string());
                binds.push(Bind::I64(bid));
                conds.push("parent_id = ?".to_string());
                binds.push(Bind::I64(pid));
            }
            // Java `eq(column, null)` → `x = NULL` → 恒不成立
            _ => conds.push("1 = 0".to_string()),
        }
        conds.push("(user_id = ? OR is_show = 1)".to_string());
        binds.push(Bind::I32(user_id));

        let where_sql = format!(" WHERE {}", conds.join(" AND "));
        let total_sql = format!("SELECT COUNT(*) FROM buyer_show_comment{where_sql}");
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let page_size = q.page_size();
        let offset = (q.page_no() - 1).saturating_mul(page_size);
        let rows_sql = format!(
            "SELECT * FROM buyer_show_comment{where_sql} \
             ORDER BY is_top DESC, add_time DESC, buyer_show_comment_id DESC LIMIT ? OFFSET ?"
        );
        let rows = bind_all!(sqlx::query_as::<_, BuyerShowComment>(&rows_sql), binds)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((u64::try_from(total.0.max(0)).unwrap_or(0), rows))
    }

    // ═════════════════════ 用户 ═════════════════════

    /// 按 id 取用户（Java `userService.getById(id)`）。
    pub async fn find_user_by_id(&self, user_id: i32) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE user_id = ? LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量取用户（Java `getBuyerShowDTO` / `clientCommentPageList` 的
    /// `in(userId, ids)`）。
    pub async fn find_users_by_ids(&self, ids: &[i32]) -> Result<Vec<User>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT * FROM \"user\" WHERE user_id IN ({ph}) ORDER BY user_id ASC"
        );
        let mut q = sqlx::query_as::<_, User>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 一批买家秀里 `user_id > 0` 的作者 id（对应 Java
    /// `getBuyerShowDTO` 的 `.map(BuyerShow::getUserId).filter(id -> id > 0)`）。
    ///
    /// 返回去重前的原始列表（由调用方去重），便于与 Java 的
    /// `Collectors.toMap` 抛重复键风险保持一致的观察。
    pub async fn find_author_ids_of_buyer_shows(&self, ids: &[i32]) -> Result<Vec<i32>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT user_id FROM buyer_show WHERE buyer_show_id IN ({ph}) AND user_id > 0"
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().map(|(uid,)| uid).collect())
    }

    // ═════════════════════ 分组 ═════════════════════

    /// 全部分组（Java `BuyerShowGroupServiceImpl#list()`，**无任何条件与排序**；
    /// 这里按主键升序返回以保证确定）。
    pub async fn list_all_groups(&self) -> Result<Vec<BuyerShowGroup>> {
        sqlx::query_as::<_, BuyerShowGroup>(
            "SELECT * FROM buyer_show_group ORDER BY buyer_show_group_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按买家秀分组 id 取分组名（Java `detail` 的
    /// `in(buyerShowGroupId, groupIdList)`）。
    pub async fn find_groups_by_ids(&self, ids: &[i32]) -> Result<Vec<BuyerShowGroup>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT * FROM buyer_show_group WHERE buyer_show_group_id IN ({ph}) \
             ORDER BY buyer_show_group_id ASC"
        );
        let mut q = sqlx::query_as::<_, BuyerShowGroup>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按商品 id 反查买家秀分组（Java
    /// `BuyerShowGroupServiceImpl#getBuyerShowGroupByProductIds`）。
    ///
    /// 两步（逐条照抄 Java 的 OR 串接 \"JSON_CONTAINS\"）：
    /// 1. \"product_group\" 中 `JSON_CONTAINS(product_ids, productId_i)` 命中
    ///    → 取其 \"product_group_id\" 集合；
    /// 2. \"buyer_show_group\" 中 `JSON_CONTAINS(product_group_id, productGroupId_j)`
    ///    命中 → 取其 \"buyer_show_group_id\"。
    ///
    /// \"productIds\" 为空时 Java 会生成 `AND ()` 非法 SQL（`lambdaQuery().and(qw -> {})`
    /// 空块）；本实现直接返回空列表，差异见服务层 \"uncertain_points\"。
    pub async fn find_group_ids_by_product_ids(&self, product_ids: &[i32]) -> Result<Vec<i32>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }

        // 第 1 步：product_group
        let mut sql = String::from("SELECT * FROM product_group WHERE ");
        for i in 0..product_ids.len() {
            if i > 0 {
                sql.push_str(" OR ");
            }
            sql.push_str("JSON_CONTAINS(product_ids, CAST(? AS JSON))");
        }
        let mut q = sqlx::query_as::<_, ProductGroupRow>(&sql);
        for pid in product_ids {
            q = q.bind(pid.to_string());
        }
        let product_groups = q.fetch_all(&self.pool).await.map_err(map_err)?;
        if product_groups.is_empty() {
            return Ok(Vec::new());
        }

        // 第 2 步：buyer_show_group
        let group_ids: Vec<i32> = product_groups.iter().map(|g| g.product_group_id).collect();
        let mut sql = String::from("SELECT * FROM buyer_show_group WHERE ");
        for i in 0..group_ids.len() {
            if i > 0 {
                sql.push_str(" OR ");
            }
            sql.push_str("JSON_CONTAINS(product_group_id, CAST(? AS JSON))");
        }
        let mut q = sqlx::query_as::<_, BuyerShowGroup>(&sql);
        for gid in &group_ids {
            q = q.bind(gid.to_string());
        }
        let groups = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(groups.into_iter().map(|g| g.buyer_show_group_id).collect())
    }

    // ═════════════════════ 商品 ═════════════════════

    /// 按 id 批量取商品（Java `ProductServiceImpl#getProductByIds`：
    /// `in(productId, ids)`，**无 ORDER BY**，不过滤 \"is_delete\" / \"product_status\"；
    /// 这里按主键升序返回以保证确定）。
    pub async fn find_products_by_ids(&self, ids: &[i32]) -> Result<Vec<BuyerShowProductVO>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(ids.len());
        let sql = format!(
            "SELECT product_id, product_name, product_sn, product_price, market_price, \
             pic_url, pic_thumb, product_stock, is_delete, product_status, shop_id, \
             is_support_return \
             FROM product WHERE product_id IN ({ph}) ORDER BY product_id ASC"
        );
        let mut q = sqlx::query_as::<_, ProductRow>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| BuyerShowProductVO {
                product_id: r.product_id,
                product_name: r.product_name,
                product_sn: r.product_sn,
                product_price: r.product_price,
                market_price: r.market_price,
                pic_url: r.pic_url,
                pic_thumb: r.pic_thumb,
                product_stock: r.product_stock,
                // Java：买家秀客户端直接把 productStock 映射给 storeProductStock
                store_product_stock: r.product_stock,
                is_delete: r.is_delete,
                product_status: r.product_status,
                shop_id: r.shop_id,
                is_support_return: r.is_support_return,
            })
            .collect())
    }

    /// 按商品 id 汇总 `product_sku.sku_stock`
    /// （Java `ProductSkuServiceImpl#getProductSkusByProductIds` +
    /// `groupingBy(productId, summingInt(skuStock))`）。
    ///
    /// \"sku_stock\" 可空 → 按 \"0\" 参与求和（Java `Optional.ofNullable(...).orElse(0)`）。
    pub async fn sum_sku_stock_by_product_ids(
        &self,
        product_ids: &[i32],
    ) -> Result<Vec<(i32, i64)>> {
        if product_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = placeholders(product_ids.len());
        let sql = format!(
            "SELECT product_id, COALESCE(SUM(sku_stock), 0) FROM product_sku \
             WHERE product_id IN ({ph}) GROUP BY product_id"
        );
        let mut q = sqlx::query_as::<_, (i32, Decimal)>(&sql);
        for id in product_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(pid, sum)| (pid, decimal_to_i64(sum)))
            .collect())
    }

    /// 按 `biz_code` 取未删除配置值（Java `ConfigService#getConfigVal`）。
    ///
    /// `None` ↔ Java 查不到记录（Java 此时返回空串，由调用方兜底）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = ? AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }
}

/// \"Decimal\" → \"i64\"（`product_sku.sku_stock` 求和结果的截断转换）。
///
/// `SUM()` 在 MySQL 中返回 \"DECIMAL\"；\"sku_stock\" 本身是整数列，
/// 求和结果必然是整数，这里截断取整即可（不用 \"f64\"，避免精度损失）。
fn decimal_to_i64(v: Decimal) -> i64 {
    v.trunc().to_i64().unwrap_or(0)
}

/// \"product\" 表投影（对应 Java \"getProductByIds\" 返回实体的字段子集）。
#[derive(Debug, Clone, sqlx::FromRow)]
struct ProductRow {
    product_id: i32,
    product_name: String,
    product_sn: String,
    product_price: Decimal,
    market_price: Decimal,
    pic_url: String,
    pic_thumb: String,
    product_stock: i32,
    is_delete: i32,
    product_status: i32,
    shop_id: i32,
    is_support_return: i32,
}

/// \"product_group\" 投影（只取第 2 步需要的分组主键）。
#[derive(Debug, Clone, sqlx::FromRow)]
struct ProductGroupRow {
    product_group_id: i32,
}

/// SQL 错误 → `nextrade_core::Error`。
fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn placeholders_matches_bind_count() {
        assert_eq!(placeholders(0), "");
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");
    }

    #[test]
    fn decimal_to_i64_truncates_without_float() {
        assert_eq!(decimal_to_i64(Decimal::ZERO), 0);
        assert_eq!(decimal_to_i64(Decimal::new(12345, 2)), 123);
        assert_eq!(decimal_to_i64(Decimal::from_str_exact("99.9").unwrap()), 99);
        assert_eq!(decimal_to_i64(Decimal::from_str_exact("100").unwrap()), 100);
    }

    #[test]
    fn feed_anchor_is_copy_and_compares_by_value() {
        let a = FeedAnchor {
            sort_order: 1,
            add_time: 2,
            buyer_show_id: 3,
        };
        let b = a;
        assert_eq!(a, b);
    }
}
