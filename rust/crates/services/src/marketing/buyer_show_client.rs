//! 买家秀前台服务（marketing::buyer_show_client）。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.buyershow.BuyerShowController`
//!   （`@RequestMapping("/api/buyerShow/buyerShow")`，12 个端点）
//! - `com.nextrade.service.promotion.impl.BuyerShowServiceImpl`
//! - `com.nextrade.service.promotion.impl.BuyerShowGroupServiceImpl#list` /
//!   `#getBuyerShowGroupByProductIds`
//! - `com.nextrade.mapper.promotion.BuyerShowMapper#selectVideoFeed`
//!   （`mapper/promotion/BuyerShowMapper.xml`）
//! - `com.nextrade.common.utils.ShareHtmlUtil`（分享落地页 HTML）
//! - `com.nextrade.common.utils.TigUtils#handelTime`（时间格式化）
//! - `com.nextrade.common.utils.StringUtils#getCurrentTime`（秒级时间戳）
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`BuyerShowClientService::client_list`] | `BuyerShowServiceImpl#clientList` |
//! | [`BuyerShowClientService::video_feed`] | `BuyerShowServiceImpl#videoFeed` |
//! | [`BuyerShowClientService::like`] | `BuyerShowServiceImpl#like` |
//! | [`BuyerShowClientService::favorite`] | `BuyerShowServiceImpl#favorite` |
//! | [`BuyerShowClientService::follow_author`] | `BuyerShowServiceImpl#followAuthor` |
//! | [`BuyerShowClientService::client_add`] | `BuyerShowServiceImpl#clientAdd` |
//! | [`BuyerShowClientService::group_list`] | `BuyerShowGroupServiceImpl#list()`（无参重载） |
//! | [`BuyerShowClientService::client_detail`] | `BuyerShowServiceImpl#clientDetail` |
//! | [`BuyerShowClientService::get_share_page_html`] | `BuyerShowServiceImpl#getSharePageHtml` |
//! | [`BuyerShowClientService::client_comment`] | `BuyerShowServiceImpl#clientComment` |
//! | [`BuyerShowClientService::client_comment_page_list`] | `BuyerShowServiceImpl#clientCommentPageList` |
//! | [`BuyerShowClientService::comment_like`] | `BuyerShowServiceImpl#commentLike` |
//!
//! # 契约标 `auth = optional` 的落地口径
//!
//! Java 侧本域**全部**挂在 `permitAll` 之下，但 JWT 过滤器会在带令牌时写入
//! 用户上下文；`SecurityUtils.getCurrentUserId()` 在**未登录时返回 `0`**
//! （不是 `null`，见 `SecurityUtils`）。Java 原实现因此有两类行为：
//!
//! 1. **读接口**（`clientList` / `videoFeed` / `clientDetail` /
//!    `clientCommentPageList`）：游客口径 = `userId = 0`，
//!    可见性判定与点赞态按 `userId = 0` 计算（确实会匹配
//!    `user_id = 0` 的非真实用户数据）。**本实现照抄**，
//!    路由层用 `optional_auth` 中间件把游客归一为 `0`。
//! 2. **写接口**（`favorite` / `followAuthor`）：Java 显式
//!    `if (userId == null || userId <= 0) throw new GlobalException("用户未登录")`
//!    → 本实现返回业务码 `401`。
//!
//! **`like` / `commentLike` / `clientAdd` / `clientComment` 是例外**：
//! Java **没有**做登录校验，直接 `setUserId(getCurrentUserId())`（游客为 `0`）
//! 并 `save()`，即**游客匿名操作会落一条 `user_id = 0` 的脏数据**
//! （`like` / `commentLike` 还没有查重，重复点赞会插入多行）。
//! 本实现为**不复制脏数据写入**，对这几个接口同样要求登录（业务码 `401`），
//! 差异记入 `uncertain_points`。
//!
//! # 未迁移依赖（如实标注，不伪造实现）
//!
//! 1. **对象存储 / 图片上传**（`GalleryPicService#uploadImg` 等）：
//!    整体未迁移。Java 落相册时给 `pic_url` / `pic_large` 追加存储尺寸后缀
//!    （`config.storageType` 决定：本地 = 空串、COS = `?imageMogr2/thumbnail/500x/`、
//!    七牛 = `?imageView2/2/w/400`、默认 OSS = `?x-oss-process=image/resize,m_pad,h_500,h_500`）；
//!    本实现读取同一份配置复刻该映射，但**不做任何上传**。
//! 2. **`@JsonTranslate` 多语言翻译**（`TranslatePackage`）：`groupList`
//!    返回的 `groupName` 在 Java 侧带 `@JsonTranslate(dataType = 16)`，
//!    翻译服务未迁移 → 本实现返回原文（单语言部署下 Java 行为亦是原文）。
//! 3. **虚拟用户 / 随机昵称随机头像**（`VirtualUserInfoService`、
//!    `RandomInfoUtil`）：属于后台 `create` / `info` / `getBuyerInfo`
//!    链路，**不在本切片契约内**，未迁移。
//! 4. **H5 域名配置**（`SettingsEnum.H5_DOMAIN` → `config.h5Domain`）：
//!    本实现读取同一配置键；配置为空时 `targetUrl` 退化为
//!    `/pages/buyerShow/buyerShowDetail?buyerShowId=...`（与 Java 一致）。
//! 5. **Redis 缓存**：本域 Java 侧无 Redis 缓存，无差异。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use nextrade_core::entities::{
    BuyerShow, BuyerShowComment, BuyerShowFavorite, BuyerShowGallery, BuyerShowLike, BuyerShowVideo,
    User,
};
use nextrade_core::models::marketing::buyer_show_client::{
    format_price, handel_time, handel_time_with, is_video_buyer_show, parse_comment_images,
    parse_i32_array_json, BuyerShowClientVO, BuyerShowCommentClientParam,
    BuyerShowCommentLikeClientParam, BuyerShowCommentPageClientQuery,
    BuyerShowCommentPageClientVO, BuyerShowFavoriteClientParam, BuyerShowFeedAuthorVO,
    BuyerShowFeedImageVO, BuyerShowFeedProductVO, BuyerShowFeedVideoVO,
    BuyerShowFollowAuthorClientParam, BuyerShowGallerySaveItem, BuyerShowGalleryVO,
    BuyerShowLikeClientParam, BuyerShowPageClientQuery, BuyerShowPageClientVO,
    BuyerShowProductVO, BuyerShowSaveClientParam, BuyerShowVideoFeedClientQuery,
    BuyerShowVideoFeedItemClientVO, BuyerShowVideoFeedPageClientVO, BuyerShowVideoSaveItem,
    BuyerShowVideoVO, CreatedIdVO, ToggleResultVO, FEED_TIME_FORMAT, NO, YES,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::buyer_show_client::{FeedAnchor, BuyerShowClientRepository};
use nextrade_infrastructure::db::Repositories;

// ————————————————————— 业务码（Java `HttpStatus`） —————————————————————
//
// `GlobalException(message)` 的 code 恒为 `SERVICE_DATA_ERROR`（1001），
// 见 `com.nextrade.common.exception.GlobalException`。

/// 通用业务失败（Java `SERVICE_DATA_ERROR`，也是 `GlobalException` 默认码）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// 未登录业务码（Java `GlobalException("用户未登录")`）。
pub const CODE_UNAUTHORIZED: i32 = 401;

/// Java `Assert.notNull(buyerShow, "买家秀不存在")` 的文案。
const ERR_BUYER_SHOW_NOT_FOUND: &str = "买家秀不存在";
/// Java `GlobalException("买家秀图片和视频不能同时为空")`。
const ERR_EMPTY_MEDIA: &str = "买家秀图片和视频不能同时为空";
/// Java `GlobalException("用户未登录")`。
const ERR_NOT_LOGGED_IN: &str = "用户未登录";
/// Java `GlobalException("不能关注自己")`。
const ERR_FOLLOW_SELF: &str = "不能关注自己";
/// 会员不存在的错误文案（Java 侧 `clientAdd` 会在此 NPE）。
const ERR_MEMBER_NOT_EXIST: &str = "会员不存在";

/// `buyer_show.sort_order` 归一默认值（Java `DEFAULT_SORT_ORDER = 0`，
/// 与 `videoFeed` SQL 的 `COALESCE` 一致）。
const DEFAULT_SORT_ORDER: i32 = 0;

/// `config` 表里存储类型的配置键（Java `SettingsEnum.STORAGE_TYPE.bizCode`）。
const CFG_STORAGE_TYPE: &str = "storageType";
/// H5 域名配置键（Java `SettingsEnum.H5_DOMAIN.bizCode`）。
const CFG_H5_DOMAIN: &str = "h5Domain";

/// 存储类型：本地存储（Java `StorageType.STORAGE_LOCAL`，`code = "0"`）。
const STORAGE_LOCAL: &str = "0";
/// 存储类型：腾讯云 COS（`code = "2"`）。
const STORAGE_COS: &str = "2";
/// 存储类型：七牛云 Kodo（`code = "3"`）。
const STORAGE_QINIU: &str = "3";

/// Java `PicConstants.OSS_SIZE_400`。
const OSS_SIZE_400: &str = "?x-oss-process=image/resize,m_pad,h_500,h_500";
/// Java `PicConstants.OSS_SIZE_800`。
const OSS_SIZE_800: &str = "?x-oss-process=image/resize,m_pad,h_800,h_800";
/// Java `PicConstants.COS_SIZE_400`。
const COS_SIZE_400: &str = "?imageMogr2/thumbnail/500x/";
/// Java `PicConstants.COS_SIZE_800`。
const COS_SIZE_800: &str = "?imageMogr2/thumbnail/800x/";
/// Java 七牛云 `size_400`（`GalleryPicServiceImpl#getImageSize`）。
const QINIU_SIZE_400: &str = "?imageView2/2/w/400";
/// Java 七牛云 `size_800`。
const QINIU_SIZE_800: &str = "?imageView2/2/w/800";

/// 买家秀前台服务。
pub struct BuyerShowClientService {
    repos: Arc<Repositories>,
}

impl BuyerShowClientService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    fn repo(&self) -> &BuyerShowClientRepository {
        &self.repos.marketing.buyer_show_client
    }

    /// 当前秒级时间戳（Java `StringUtils.getCurrentTime()`）。
    fn now_seconds() -> i64 {
        nextrade_common::utils::now_seconds()
    }

    // ═════════════════════ GET /list ═════════════════════

    /// `GET /api/buyerShow/buyerShow/list`：获取买家秀列表。
    ///
    /// 对应 Java `BuyerShowController.clientList` →
    /// `BuyerShowServiceImpl#clientList`。
    ///
    /// `current_user_id`：登录会员 id；**游客传 `0`**
    /// （Java `SecurityUtils.getCurrentUserId()` 的游客返回值）。
    pub async fn client_list(
        &self,
        q: &BuyerShowPageClientQuery,
        current_user_id: i32,
    ) -> Result<PageResult<BuyerShowPageClientVO>> {
        let page_query = PageQuery {
            page: q.page_no(),
            page_size: q.page_size(),
        };
        let (total, rows) = self.repo().paginate_client_list(q, current_user_id).await?;
        if rows.is_empty() {
            // Java：`return new Page<>()`（空页，total = 0）
            return Ok(PageResult::new(0, Vec::new(), &page_query));
        }
        let ids: Vec<i32> = rows.iter().map(|r| r.buyer_show_id).collect();
        let bundle = self.load_bundle(&ids).await?;

        let list: Vec<BuyerShowPageClientVO> = rows
            .iter()
            .map(|bs| {
                let img_vo = gallery_to_vo(bundle.gallery_of(bs.buyer_show_id));
                let video_vo = video_to_vo(bundle.videos_of(bs.buyer_show_id));

                // Java：`if (buyerShow.getUserId() > 0)` 才用真实会员覆盖
                let (buyer_name, buyer_photo) = if bs.user_id > 0 {
                    match bundle.users.get(&bs.user_id) {
                        Some(u) => (user_name(u), u.avatar.clone()),
                        None => (bs.buyer_name.clone(), bs.buyer_photo.clone()),
                    }
                } else {
                    (bs.buyer_name.clone(), bs.buyer_photo.clone())
                };

                // Java：`likeCount` / `isLike` 仅在 `buyerShowLikeList` 非空时回填
                let (like_count, is_like) = if bundle.likes.is_empty() {
                    (None, None)
                } else {
                    let of_show: Vec<&BuyerShowLike> = bundle
                        .likes
                        .iter()
                        .filter(|l| l.buyer_show_id == bs.buyer_show_id)
                        .collect();
                    let count = i64::try_from(of_show.len()).unwrap_or(i64::MAX);
                    let mine = of_show.iter().any(|l| l.user_id == current_user_id);
                    (Some(count), Some(if mine { YES } else { NO }))
                };

                BuyerShowPageClientVO {
                    buyer_show_id: bs.buyer_show_id,
                    is_video: i32::from(is_video_buyer_show(&img_vo, &video_vo)),
                    img_list: img_vo,
                    video_list: video_vo,
                    content: bs.content.clone(),
                    buyer_photo,
                    buyer_name,
                    like_count,
                    is_like,
                }
            })
            .collect();

        Ok(PageResult::new(total, list, &page_query))
    }

    // ═════════════════════ GET /videoFeed ═════════════════════

    /// `GET /api/buyerShow/buyerShow/videoFeed`：买家秀视频流（游标翻页）。
    ///
    /// 对应 Java `BuyerShowController.videoFeed` →
    /// `BuyerShowServiceImpl#videoFeed`。
    ///
    /// 游标语义（与 Java 逐条对齐）：
    /// - `cursor` 与 `startId` 都是**买家秀 id**；
    /// - `cursor` 续页（严格取锚点之后，`inclusive = false`）；
    /// - `startId` 首屏（含锚点自身，`inclusive = true`），
    ///   且**仅在未传 `cursor` 时生效**；
    /// - 多取一条判断 `hasMore`，**不做环绕补齐**；
    /// - `nextCursor` 为当前页最后一行的 id（无更多时为空串）。
    pub async fn video_feed(
        &self,
        q: &BuyerShowVideoFeedClientQuery,
        current_user_id: i32,
    ) -> Result<BuyerShowVideoFeedPageClientVO> {
        // Java：游客归一为 0，后续可见性判定与点赞/收藏态均以 > 0 为登录标志
        let user_id = if current_user_id > 0 { current_user_id } else { 0 };
        let size = q.page_size();

        let cursor_id = parse_cursor(q.cursor.as_deref());
        let inclusive = cursor_id.is_none() && q.start_id.is_some();
        let anchor = self.build_anchor(cursor_id.or(q.start_id)).await?;

        // size + 1：额外取一条判断是否还有下一页
        let fetched = self
            .repo()
            .fetch_video_feed(q, user_id, anchor, inclusive, size.saturating_add(1))
            .await?;

        let (rows, has_more) = split_feed_window(fetched, size);
        if rows.is_empty() {
            // Java：空结果 → list = []、nextCursor = ""、hasMore = false
            return Ok(BuyerShowVideoFeedPageClientVO {
                list: Vec::new(),
                next_cursor: String::new(),
                has_more: false,
            });
        }

        let ids: Vec<i32> = rows.iter().map(|r| r.buyer_show_id).collect();
        let bundle = self.load_bundle(&ids).await?;
        let favorites = self
            .repo()
            .find_favorites_by_buyer_show_ids(&ids)
            .await?;
        let comment_counts: HashMap<i32, i64> = self
            .repo()
            .count_visible_comments_by_buyer_show_ids(&ids, user_id)
            .await?
            .into_iter()
            .collect();

        // 关联商品：失败时降级为空 Map（Java `getVideoFeedProductMap` 的 try/catch）
        let product_map = self.video_feed_product_map(&rows).await;
        let followed = self.followed_author_ids(&rows, user_id).await?;

        let list: Vec<BuyerShowVideoFeedItemClientVO> = rows
            .iter()
            .map(|bs| {
                build_feed_item(
                    bs,
                    user_id,
                    &bundle,
                    &favorites,
                    &comment_counts,
                    &product_map,
                    &followed,
                )
            })
            .collect();

        let next_cursor = if has_more {
            rows.last()
                .map_or_else(String::new, |r| r.buyer_show_id.to_string())
        } else {
            String::new()
        };

        Ok(BuyerShowVideoFeedPageClientVO {
            list,
            next_cursor,
            has_more,
        })
    }

    /// 反查复合游标锚点（Java `BuyerShowServiceImpl#buildAnchor`）。
    ///
    /// `anchorId` 为空、记录不存在、或 `sort_order` / `add_time`
    /// 缺失时返回 `None`（视为从头开始）。
    ///
    /// ⚠️ `sort_order` 在库中可空：Java `buildAnchor` 明确要求它非 `null`
    /// （`row.getSortOrder() == null` 时返回 `null` 锚点），本实现照抄。
    async fn build_anchor(&self, anchor_id: Option<i32>) -> Result<Option<FeedAnchor>> {
        let Some(id) = anchor_id else {
            return Ok(None);
        };
        let Some(row) = self.repo().find_by_id(id).await? else {
            return Ok(None);
        };
        let Some(sort_order) = row.sort_order else {
            return Ok(None);
        };
        Ok(Some(FeedAnchor {
            sort_order,
            add_time: row.add_time,
            buyer_show_id: row.buyer_show_id,
        }))
    }

    /// 批量拉取视频流各买家秀的关联商品（Java `getVideoFeedProductMap`）。
    ///
    /// Java 用 try/catch 把失败降级为空 Map（不阻塞视频流）；
    /// 本实现同样降级，仅记 `tracing::warn`。
    async fn video_feed_product_map(&self, rows: &[BuyerShow]) -> HashMap<i32, BuyerShowProductVO> {
        let mut product_ids: HashSet<i32> = HashSet::new();
        for bs in rows {
            product_ids.extend(parse_i32_array_json(Some(bs.range_data.as_str())));
        }
        if product_ids.is_empty() {
            return HashMap::new();
        }
        let mut ids: Vec<i32> = product_ids.into_iter().collect();
        ids.sort_unstable();
        match self.repo().find_products_by_ids(&ids).await {
            Ok(products) => products
                .into_iter()
                .map(|p| (p.product_id, p))
                .collect::<HashMap<_, _>>(),
            Err(e) => {
                tracing::warn!("视频流批量获取商品信息失败: {e}");
                HashMap::new()
            }
        }
    }

    /// 批量判断当前用户对各作者的关注状态（Java `getFollowedAuthorIds`）。
    ///
    /// 未登录（`user_id <= 0`）→ 空集合（不查库）。
    async fn followed_author_ids(&self, rows: &[BuyerShow], user_id: i32) -> Result<HashSet<i32>> {
        if user_id <= 0 {
            return Ok(HashSet::new());
        }
        let mut author_ids: Vec<i32> = rows
            .iter()
            .map(|r| r.user_id)
            .filter(|id| *id > 0 && *id != user_id)
            .collect();
        author_ids.sort_unstable();
        author_ids.dedup();
        if author_ids.is_empty() {
            return Ok(HashSet::new());
        }
        Ok(self
            .repo()
            .find_followed_author_ids(user_id, &author_ids)
            .await?
            .into_iter()
            .collect())
    }

    // ═════════════════════ POST /like ═════════════════════

    /// `POST /api/buyerShow/buyerShow/like`：买家秀点赞 / 取消点赞。
    ///
    /// 对应 Java `BuyerShowController.like` → `BuyerShowServiceImpl#like`。
    ///
    /// ⚠️ **与 Java 的差异（安全收紧）**：Java **没有**校验登录，
    /// 游客（`userId = 0`）点赞会落一条 `user_id = 0` 的脏数据，
    /// 且因为没有查重，重复点赞会插入多行。本实现要求登录
    /// （否则业务码 `401`），避免写入无法归属的脏数据。
    pub async fn like(
        &self,
        param: &BuyerShowLikeClientParam,
        user_id: i32,
    ) -> Result<ToggleResultVO> {
        let buyer_show_id = require_i32(param.buyer_show_id, "买家秀信息错误")?;
        let is_like = require_i32(param.is_like, "操作失败")?;
        if user_id <= 0 {
            return Err(Error::biz(CODE_UNAUTHORIZED, ERR_NOT_LOGGED_IN));
        }

        if is_like == YES {
            self.repo()
                .insert_like(buyer_show_id, user_id, Self::now_seconds())
                .await?;
            return Ok(ToggleResultVO { active: true });
        }
        self.repo().delete_like(buyer_show_id, user_id).await?;
        Ok(ToggleResultVO { active: false })
    }

    // ═════════════════════ POST /favorite ═════════════════════

    /// `POST /api/buyerShow/buyerShow/favorite`：买家秀收藏 / 取消收藏。
    ///
    /// 对应 Java `BuyerShowController.favorite` →
    /// `BuyerShowServiceImpl#favorite`（该方法带 `@Transactional`；
    /// 本实现的两个写步骤均为幂等「先查后写」，未额外包事务）。
    pub async fn favorite(
        &self,
        param: &BuyerShowFavoriteClientParam,
        user_id: i32,
    ) -> Result<ToggleResultVO> {
        let buyer_show_id = require_i32(param.buyer_show_id, "买家秀信息错误")?;
        let is_favorite = require_i32(param.is_favorite, "操作失败")?;
        if user_id <= 0 {
            return Err(Error::biz(CODE_UNAUTHORIZED, ERR_NOT_LOGGED_IN));
        }
        if self.repo().find_by_id(buyer_show_id).await?.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_BUYER_SHOW_NOT_FOUND));
        }

        let exists = self
            .repo()
            .count_favorite_of_user(buyer_show_id, user_id)
            .await?
            > 0;

        if is_favorite == YES {
            if !exists {
                self.repo()
                    .insert_favorite(buyer_show_id, user_id, Self::now_seconds())
                    .await?;
            }
            return Ok(ToggleResultVO { active: true });
        }
        if exists {
            self.repo().delete_favorite(buyer_show_id, user_id).await?;
        }
        Ok(ToggleResultVO { active: false })
    }

    // ═════════════════════ POST /followAuthor ═════════════════════

    /// `POST /api/buyerShow/buyerShow/followAuthor`：关注 / 取关买家秀作者。
    ///
    /// 对应 Java `BuyerShowController.followAuthor` →
    /// `BuyerShowServiceImpl#followAuthor`。
    pub async fn follow_author(
        &self,
        param: &BuyerShowFollowAuthorClientParam,
        user_id: i32,
    ) -> Result<ToggleResultVO> {
        let author_id = require_i32(param.author_id, "作者信息错误")?;
        let is_follow = require_i32(param.is_follow, "操作失败")?;
        if user_id <= 0 {
            return Err(Error::biz(CODE_UNAUTHORIZED, ERR_NOT_LOGGED_IN));
        }
        if user_id == author_id {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_FOLLOW_SELF));
        }

        let exists = self.repo().count_follow(user_id, author_id).await? > 0;
        if is_follow == YES {
            if !exists {
                self.repo()
                    .insert_follow(user_id, author_id, Self::now_seconds())
                    .await?;
            }
            return Ok(ToggleResultVO { active: true });
        }
        if exists {
            self.repo().delete_follow(user_id, author_id).await?;
        }
        Ok(ToggleResultVO { active: false })
    }

    // ═════════════════════ POST /add ═════════════════════

    /// `POST /api/buyerShow/buyerShow/add`：添加买家秀信息。
    ///
    /// 对应 Java `BuyerShowController.clientAdd` →
    /// `BuyerShowServiceImpl#clientAdd`。
    ///
    /// Java 落库口径（**逐条照抄**）：
    /// - `status = 0`（待审核）、`is_show = 0`（隐藏）、`sort_order = 0`；
    /// - `userId = getCurrentUserId()`、`addTime = now`；
    /// - `buyerName` / `buyerPhoto` 从 `user` 表快照；
    /// - `groupData` = 「按 `range_data` 反查到的买家秀分组 id」的 JSON 数组，
    ///   反查为空时落 `NULL`；
    /// - `rangeData` = 入参 `rangeData` 的 JSON 数组文本；
    /// - 素材全部**先删后插**（新建时删除步骤无副作用）。
    pub async fn client_add(
        &self,
        param: &BuyerShowSaveClientParam,
        user_id: i32,
    ) -> Result<CreatedIdVO> {
        if user_id <= 0 {
            return Err(Error::biz(CODE_UNAUTHORIZED, ERR_NOT_LOGGED_IN));
        }
        if param.range_data.is_empty() {
            // Java `@NotEmpty(message = "关联商品不能为空")`
            return Err(Error::biz(SERVICE_DATA_ERROR, "关联商品不能为空"));
        }
        if param.img_list.is_empty() && param.buyer_show_video_list.is_empty() {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_EMPTY_MEDIA));
        }

        let user = self
            .repo()
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_MEMBER_NOT_EXIST))?;

        let group_ids = self
            .repo()
            .find_group_ids_by_product_ids(&param.range_data)
            .await?;
        let group_data = if group_ids.is_empty() {
            None
        } else {
            Some(to_json_i32_array(&group_ids)?)
        };
        let range_data = to_json_i32_array(&param.range_data)?;

        let buyer_show_id = self
            .repo()
            .insert_buyer_show(
                &range_data,
                user_id,
                &user.avatar,
                &user_name(&user),
                param.content.as_deref(),
                group_data.as_deref(),
                Self::now_seconds(),
                0,
                NO, // status：待审核
                NO, // is_show：隐藏
            )
            .await?;

        self.save_media(buyer_show_id, &param.buyer_show_video_list, &param.img_list)
            .await?;

        Ok(CreatedIdVO { id: buyer_show_id })
    }

    /// 素材先删后插（Java `saveOrUpdateBuyerShowVideo` +
    /// `saveOrUpdateBuyerShowGallery`）。
    ///
    /// 视频：`sort_order` 为 `null` 时按**列表下标**兜底；
    /// 相册：`sort_order` 恒为列表下标，
    /// `pic_url` / `pic_large` 追加存储尺寸后缀，
    /// `pic_original` 存原始 url，`pic_desc` 置空串，`pic_thumb` 存入参 `picThumb`。
    async fn save_media(
        &self,
        buyer_show_id: i32,
        videos: &[BuyerShowVideoSaveItem],
        imgs: &[BuyerShowGallerySaveItem],
    ) -> Result<()> {
        self.repo().delete_videos_of(buyer_show_id).await?;
        for (i, v) in videos.iter().enumerate() {
            let fallback = i32::try_from(i).unwrap_or(i32::MAX);
            self.repo()
                .insert_video(
                    buyer_show_id,
                    v.video_id.unwrap_or(0),
                    v.video_url.as_deref().unwrap_or_default(),
                    v.video_cover.as_deref().unwrap_or_default(),
                    v.format.as_deref().unwrap_or_default(),
                    v.sort_order.unwrap_or(fallback),
                )
                .await?;
        }

        self.repo().delete_gallery_of(buyer_show_id).await?;
        if imgs.is_empty() {
            return Ok(());
        }
        let (size_400, _size_800) = self.image_size_suffixes().await?;
        for (i, g) in imgs.iter().enumerate() {
            let pic_url = g.pic_url.as_deref().unwrap_or_default();
            let sort_order = i32::try_from(i).unwrap_or(i32::MAX);
            // Java：`buyerShowId != null` 分支下 picLarge 也用 size_400
            // （原实现的两个分支 body 完全相同，疑为笔误，这里照抄该行为）
            let sized = format!("{pic_url}{size_400}");
            self.repo()
                .insert_gallery(
                    buyer_show_id,
                    &sized,
                    "",
                    g.pic_thumb.as_deref().unwrap_or_default(),
                    pic_url,
                    &sized,
                    sort_order,
                )
                .await?;
        }
        Ok(())
    }

    // ═════════════════════ GET /groupList ═════════════════════

    /// `GET /api/buyerShow/buyerShow/groupList`：分组列表。
    ///
    /// 对应 Java `BuyerShowController.groupList` →
    /// `BuyerShowGroupServiceImpl#list()`（**无参重载**，
    /// 即 MyBatis Plus 的 `list()`：无过滤、无排序）。
    ///
    /// 未迁移：`groupName` 上的 `@JsonTranslate(dataType = 16)` 翻译（原文返回）。
    pub async fn group_list(&self) -> Result<Vec<BuyerShowGroupVO>> {
        Ok(self
            .repo()
            .list_all_groups()
            .await?
            .into_iter()
            .map(|g| BuyerShowGroupVO {
                buyer_show_group_id: g.buyer_show_group_id,
                group_name: g.group_name,
                product_group_id: g.product_group_id,
                shop_id: g.shop_id,
            })
            .collect())
    }

    // ═════════════════════ GET /detail ═════════════════════

    /// `GET /api/buyerShow/buyerShow/detail?buyerShowId=`：买家秀详情。
    ///
    /// 对应 Java `BuyerShowController.clientDetail` →
    /// `BuyerShowServiceImpl#clientDetail`。
    ///
    /// 商品库存口径（Java）：先用 `product_sku` 中该商品的
    /// `sku_stock` 之和覆盖 `product.product_stock`；
    /// 再**直接把 `productStock` 映射给 `storeProductStock`**
    /// （买家秀客户端沿用门店库存字段展示售罄状态，此处不查询门店）。
    pub async fn client_detail(
        &self,
        buyer_show_id: i32,
        current_user_id: i32,
    ) -> Result<BuyerShowClientVO> {
        let Some(bs) = self.repo().find_by_id(buyer_show_id).await? else {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_BUYER_SHOW_NOT_FOUND));
        };

        let imgs = self.repo().find_gallery_of(buyer_show_id).await?;
        let videos = self.repo().find_videos_of(buyer_show_id).await?;

        let (buyer_name, buyer_photo) = if bs.user_id > 0 {
            match self.repo().find_user_by_id(bs.user_id).await? {
                Some(u) => (user_name(&u), u.avatar.clone()),
                None => (bs.buyer_name.clone(), bs.buyer_photo.clone()),
            }
        } else {
            (bs.buyer_name.clone(), bs.buyer_photo.clone())
        };

        let comment_count = self
            .repo()
            .count_visible_comments(buyer_show_id, current_user_id)
            .await?;
        let like_count = self.repo().count_likes(buyer_show_id).await?;
        let is_like = self
            .repo()
            .count_like_of_user(buyer_show_id, current_user_id)
            .await?
            > 0;

        let product_ids = parse_i32_array_json(Some(bs.range_data.as_str()));
        let mut product_list = self.repo().find_products_by_ids(&product_ids).await?;
        if !product_list.is_empty() {
            let ids: Vec<i32> = product_list.iter().map(|p| p.product_id).collect();
            let stock_map: HashMap<i32, i64> = self
                .repo()
                .sum_sku_stock_by_product_ids(&ids)
                .await?
                .into_iter()
                .collect();
            for p in &mut product_list {
                if let Some(sum) = stock_map.get(&p.product_id) {
                    let stock = i32::try_from(*sum).unwrap_or(i32::MAX);
                    p.product_stock = stock;
                    // Java：买家秀客户端把 productStock 直接映射给 storeProductStock
                    p.store_product_stock = stock;
                }
            }
        }

        // 所属分组：Java 仅在 `group_data` 非空白时才设置 groupNameList / groupIdList
        let (group_name_list, group_id_list) = if bs
            .group_data
            .as_deref()
            .map(str::trim)
            .is_some_and(|g| !g.is_empty())
        {
            let group_ids = parse_i32_array_json(bs.group_data.as_deref());
            if group_ids.is_empty() {
                (None, None)
            } else {
                // Java `.in(buyerShowGroupId, groupIdList)` 后按查询返回顺序取名称
                let groups = self.repo().find_groups_by_ids(&group_ids).await?;
                let names: Vec<String> = groups.into_iter().map(|g| g.group_name).collect();
                (Some(names), Some(group_ids))
            }
        } else {
            (None, None)
        };

        Ok(BuyerShowClientVO {
            buyer_show_id: bs.buyer_show_id,
            buyer_photo,
            buyer_name,
            img_list: gallery_to_vo(&imgs),
            video_list: video_to_vo(&videos),
            product_list,
            content: bs.content.clone(),
            add_time: handel_time(bs.add_time),
            like_count,
            comment_count,
            is_like: i32::from(is_like),
            group_name_list,
            group_id_list,
        })
    }

    // ═════════════════════ GET /share ═════════════════════

    /// `GET /api/buyerShow/buyerShow/share?buyerShowId=`：买家秀分享落地页。
    ///
    /// 对应 Java `BuyerShowController.sharePage` →
    /// `BuyerShowServiceImpl#getSharePageHtml` +
    /// `ShareHtmlUtil#buildOgHtml` / `#toAbsoluteUrl`。
    ///
    /// Java 行为（**逐条照抄**）：
    /// - 标题默认 `买家秀`；命中作者（`user_id > 0` 且昵称非空）时
    ///   改为 `<作者名>的买家秀`；
    /// - 描述取 `buyer_show.content`（`null` → 空串）；
    /// - 预览图优先取相册首图（`pic_url`），为空则回退首个视频封面；
    /// - H5 域名取 `config.h5Domain`；空则目标地址退化为
    ///   `/pages/buyerShow/buyerShowDetail?buyerShowId=<id>`；
    /// - 查询异常被 try/catch 吞掉 → 仍返回带兜底文案的 HTML。
    ///
    /// 与 Java 的差异：Java 用 `response.getWriter().write(html)` 绕过统一响应包装
    /// （`@IgnoreResponseAdvice`），本实现由路由层返回 `text/html` 响应体，
    /// 不套 `ApiResponse` 包装（同语义）。
    pub async fn get_share_page_html(&self, buyer_show_id: i32) -> String {
        let mut name = "买家秀".to_string();
        let mut desc = String::new();
        let mut pic = String::new();

        if let Ok(Some(bs)) = self.repo().find_by_id(buyer_show_id).await {
            desc = bs.content.clone().unwrap_or_default();
            if bs.user_id > 0 {
                if let Ok(Some(user)) = self.repo().find_user_by_id(bs.user_id).await {
                    let real_name = user_name(&user);
                    if !real_name.trim().is_empty() {
                        name = format!("{real_name}的买家秀");
                    }
                }
            }
            if let Ok(Some(g)) = self.repo().find_first_gallery(buyer_show_id).await {
                if !g.pic_url.trim().is_empty() {
                    pic = g.pic_url.clone();
                }
            }
            if pic.trim().is_empty() {
                if let Ok(Some(v)) = self.repo().find_first_video(buyer_show_id).await {
                    if !v.video_cover.trim().is_empty() {
                        pic = v.video_cover.clone();
                    }
                }
            }
        }

        let h5_domain = self
            .repo()
            .find_config_val(CFG_H5_DOMAIN)
            .await
            .ok()
            .flatten()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_default();

        let target_url =
            format!("{h5_domain}/pages/buyerShow/buyerShowDetail?buyerShowId={buyer_show_id}");
        let og_image = to_absolute_url(&pic, &h5_domain);
        build_og_html(&name, &desc, &og_image, &target_url, "article")
    }

    // ═════════════════════ POST /comment ═════════════════════

    /// `POST /api/buyerShow/buyerShow/comment`：评论买家秀。
    ///
    /// 对应 Java `BuyerShowController.clientComment` →
    /// `BuyerShowServiceImpl#clientComment`。
    ///
    /// Java 落库口径：`status = 0`（待审核，源码写的是 `Constants.NO`）、
    /// `is_top = 0`、`is_show = 0`；`userName` 取 `user.username`
    /// （**不是** `getName()`，即不做昵称回退）。
    pub async fn client_comment(
        &self,
        param: &BuyerShowCommentClientParam,
        user_id: i32,
    ) -> Result<CreatedIdVO> {
        let buyer_show_id = require_i32(param.buyer_show_id, "买家秀信息错误")?;
        if user_id <= 0 {
            return Err(Error::biz(CODE_UNAUTHORIZED, ERR_NOT_LOGGED_IN));
        }
        let user = self.repo().find_user_by_id(user_id).await?;
        let (user_name_raw, user_photo) = match &user {
            Some(u) => (u.username.clone(), u.avatar.clone()),
            // Java：user 为 null 时跳过 userName/userPhoto 赋值，
            // 命中 NOT NULL 列会 500；这里退回空串记录，不伪造业务规则。
            None => (String::new(), String::new()),
        };

        let id = self
            .repo()
            .insert_comment(
                param.parent_id.unwrap_or(0),
                buyer_show_id,
                NO, // status：待审核（Java `setStatus(Constants.NO)`）
                NO, // is_top
                NO, // is_show
                user_id,
                &user_name_raw,
                &user_photo,
                param.content.as_deref(),
                Self::now_seconds(),
                param.comment_pic.as_deref(),
            )
            .await?;
        Ok(CreatedIdVO { id })
    }

    // ═════════════════════ GET /comment/pageList ═════════════════════

    /// `GET /api/buyerShow/buyerShow/comment/pageList`：评论分页列表。
    ///
    /// 对应 Java `BuyerShowController.clientCommentPageList` →
    /// `BuyerShowServiceImpl#clientCommentPageList`。
    ///
    /// ⚠️ **必须同时传 `buyerShowId` 与 `parentId`**：Java 的
    /// `queryWrapper.eq(BuyerShowComment::getBuyerShowId, query.getBuyerShowId())`
    /// 与 `.eq(...getParentId(), ...)` 都是**无条件**等值匹配，
    /// 任一为 `null` 时生成 `x = NULL`（恒不成立）→ 返回空页。
    /// 本实现用 `1 = 0` 复刻该语义（见仓储 `paginate_comments`）。
    pub async fn client_comment_page_list(
        &self,
        q: &BuyerShowCommentPageClientQuery,
        current_user_id: i32,
    ) -> Result<PageResult<BuyerShowCommentPageClientVO>> {
        let page_query = PageQuery {
            page: q.page_no(),
            page_size: q.page_size(),
        };
        let (total, rows) = self.repo().paginate_comments(q, current_user_id).await?;
        if rows.is_empty() {
            return Ok(PageResult::new(0, Vec::new(), &page_query));
        }

        let comment_ids: Vec<i32> = rows.iter().map(|c| c.buyer_show_comment_id).collect();

        // 查回复评论（parentId > 0）时无需统计回复数
        let reply_counts: HashMap<i32, i64> = if q.parent_id.unwrap_or(0) > 0 {
            HashMap::new()
        } else {
            self.repo()
                .count_visible_replies_by_parent_ids(&comment_ids, current_user_id)
                .await?
                .into_iter()
                .collect()
        };

        // 用户信息（只查 userId > 0 的评论）
        let mut user_ids: Vec<i32> = rows
            .iter()
            .map(|c| c.user_id)
            .filter(|id| *id > 0)
            .collect();
        user_ids.sort_unstable();
        user_ids.dedup();
        let user_map: HashMap<i32, User> = self
            .repo()
            .find_users_by_ids(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();

        let comment_likes = self
            .repo()
            .find_comment_likes_by_comment_ids(&comment_ids)
            .await?;

        // `isAuthor` 需要买家秀作者 id（Java 在这里 `this.getById(buyerShowId)` 后
        // 直接 `buyerShow.getUserId()`；买家秀不存在会 NPE）。
        // 本实现降级为「都不是作者」，不伪造作者归属。
        let author_id: Option<i32> = match q.buyer_show_id {
            Some(raw) => match i32::try_from(raw) {
                Ok(id) => self.repo().find_by_id(id).await?.map(|bs| bs.user_id),
                Err(_) => None,
            },
            None => None,
        };

        let list: Vec<BuyerShowCommentPageClientVO> = rows
            .iter()
            .map(|c| {
                let (display_name, display_photo) = if c.user_id > 0 {
                    match user_map.get(&c.user_id) {
                        Some(u) => (user_name(u), u.avatar.clone()),
                        None => (c.user_name.clone(), c.user_photo.clone()),
                    }
                } else {
                    (c.user_name.clone(), c.user_photo.clone())
                };

                let (like_count, is_like) = if comment_likes.is_empty() {
                    (None, None)
                } else {
                    let of_comment: Vec<_> = comment_likes
                        .iter()
                        .filter(|l| l.comment_id == c.buyer_show_comment_id)
                        .collect();
                    let count = i64::try_from(of_comment.len()).unwrap_or(i64::MAX);
                    let mine = of_comment.iter().any(|l| l.user_id == current_user_id);
                    (Some(count), Some(if mine { YES } else { NO }))
                };

                BuyerShowCommentPageClientVO {
                    buyer_show_comment_id: c.buyer_show_comment_id,
                    content: c.content.clone(),
                    comment_pic: c.comment_pic.clone(),
                    images: parse_comment_images(c.comment_pic.as_deref()),
                    user_name: display_name,
                    user_photo: display_photo,
                    user_id: c.user_id,
                    like_count,
                    is_like,
                    is_author: i32::from(c.user_id != 0 && Some(c.user_id) == author_id),
                    is_me: i32::from(c.user_id != 0 && c.user_id == current_user_id),
                    reply_count: reply_counts
                        .get(&c.buyer_show_comment_id)
                        .copied()
                        .unwrap_or(0),
                    add_time: handel_time(c.add_time),
                }
            })
            .collect();

        Ok(PageResult::new(total, list, &page_query))
    }

    // ═════════════════════ POST /comment/like ═════════════════════

    /// `POST /api/buyerShow/buyerShow/comment/like`：评论点赞 / 取消点赞。
    ///
    /// 对应 Java `BuyerShowController.commentLike` →
    /// `BuyerShowServiceImpl#commentLike`。
    ///
    /// ⚠️ 与 `like` 同因：Java 未校验登录也未查重，
    /// 游客会落 `user_id = 0` 的脏数据；本实现要求登录。
    pub async fn comment_like(
        &self,
        param: &BuyerShowCommentLikeClientParam,
        user_id: i32,
    ) -> Result<ToggleResultVO> {
        let comment_id = require_i32(param.buyer_show_comment_id, "买家秀评论信息错误")?;
        let is_like = require_i32(param.is_like, "操作失败")?;
        if user_id <= 0 {
            return Err(Error::biz(CODE_UNAUTHORIZED, ERR_NOT_LOGGED_IN));
        }

        if is_like == YES {
            self.repo()
                .insert_comment_like(comment_id, user_id, Self::now_seconds())
                .await?;
            return Ok(ToggleResultVO { active: true });
        }
        self.repo().delete_comment_like(comment_id, user_id).await?;
        Ok(ToggleResultVO { active: false })
    }

    // ————————————————————— 聚合查询（列表 / 视频流共用） —————————————————————

    /// 一次取回一批买家秀的会员 / 点赞 / 视频 / 相册
    /// （对应 Java `BuyerShowServiceImpl#getBuyerShowDTO`）。
    ///
    /// 会员只查 `user_id > 0` 的行（Java
    /// `.map(BuyerShow::getUserId).filter(userId -> userId > 0)`）。
    async fn load_bundle(&self, ids: &[i32]) -> Result<BuyerShowBundle> {
        let (videos, galleries) = self.repo().find_assets_by_buyer_show_ids(ids).await?;
        let likes = self.repo().find_likes_by_buyer_show_ids(ids).await?;

        let mut user_ids = self.repo().find_author_ids_of_buyer_shows(ids).await?;
        user_ids.sort_unstable();
        user_ids.dedup();
        let users: HashMap<i32, User> = self
            .repo()
            .find_users_by_ids(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();

        Ok(BuyerShowBundle {
            videos: group_by(videos, |v| v.buyer_show_id),
            galleries: group_by(galleries, |g| g.buyer_show_id),
            likes,
            users,
        })
    }

    /// 按 Java `GalleryPicServiceImpl#getImageSize` 推导
    /// `(size_400, size_800)` 后缀。
    ///
    /// `storageType`：`"0"` 本地（两者皆空串）、`"2"` COS、`"3"` 七牛、
    /// 其它/缺省按阿里云 OSS（Java `switch` 的 `default` 分支）。
    async fn image_size_suffixes(&self) -> Result<(&'static str, &'static str)> {
        let storage_type = self
            .repo()
            .find_config_val(CFG_STORAGE_TYPE)
            .await?
            .unwrap_or_default();
        Ok(match storage_type.as_str() {
            STORAGE_LOCAL => ("", ""),
            STORAGE_COS => (COS_SIZE_400, COS_SIZE_800),
            STORAGE_QINIU => (QINIU_SIZE_400, QINIU_SIZE_800),
            _ => (OSS_SIZE_400, OSS_SIZE_800),
        })
    }
}

// ————————————————————— 聚合结构 —————————————————————

/// 一批买家秀的关联数据（对应 Java `BuyerShowDTO`）。
#[derive(Debug, Default)]
struct BuyerShowBundle {
    /// `buyer_show_id -> 视频素材`（SQL 已按 `sort_order, id` 稳定排序）
    videos: HashMap<i32, Vec<BuyerShowVideo>>,
    /// `buyer_show_id -> 相册`（SQL 已按 `sort_order, pic_id` 稳定排序）
    galleries: HashMap<i32, Vec<BuyerShowGallery>>,
    /// 全部点赞行（Java `buyerShowLikeList`，**未按买家秀分组**）
    likes: Vec<BuyerShowLike>,
    /// `user_id -> User`
    users: HashMap<i32, User>,
}

impl BuyerShowBundle {
    /// 某买家秀的视频素材（无 → 空切片，对应 Java `getOrDefault(id, new ArrayList<>())`）。
    fn videos_of(&self, id: i32) -> &[BuyerShowVideo] {
        self.videos.get(&id).map_or(&[], Vec::as_slice)
    }

    /// 某买家秀的相册（无 → 空切片）。
    fn gallery_of(&self, id: i32) -> &[BuyerShowGallery] {
        self.galleries.get(&id).map_or(&[], Vec::as_slice)
    }
}

// ————————————————————— 纯逻辑（可单测） —————————————————————

/// 把 `size + 1` 条查询结果裁剪为当前页，并用额外一条判断是否还有下一页
/// （Java `BuyerShowServiceImpl#buildVideoFeedPageWindow`）。
///
/// 禁止环绕补齐：多取的那条只用来判 `hasMore`，**不**放进结果。
#[must_use]
fn split_feed_window(mut fetched: Vec<BuyerShow>, size: i32) -> (Vec<BuyerShow>, bool) {
    let size_usize = usize::try_from(size.max(0)).unwrap_or(0);
    if fetched.len() <= size_usize {
        return (fetched, false);
    }
    let _extra = fetched.split_off(size_usize);
    (fetched, true)
}

/// 解析前端回传的游标（纯数字买家秀 id）。
///
/// 空白 / 非法数字 → `None`（视为从头开始），对应 Java `parseCursor`
/// 捕获 `NumberFormatException` 后返回 `null`。
#[must_use]
fn parse_cursor(cursor: Option<&str>) -> Option<i32> {
    let raw = cursor?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i32>().ok()
}

/// `Option<i32>` 必填校验（对应 Java `@NotNull` 的默认文案包装）。
fn require_i32(value: Option<i32>, message: &str) -> Result<i32> {
    value.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, message.to_string()))
}

/// 用户名（Java `User#getName()`：昵称非空白优先，否则用户名）。
#[must_use]
fn user_name(u: &User) -> String {
    match u.nickname.as_deref() {
        Some(n) if !n.trim().is_empty() => n.to_string(),
        _ => u.username.clone(),
    }
}

/// `Vec<i32>` → JSON 数组文本（Java `JSONUtil.toJsonStr(list)`）。
fn to_json_i32_array(v: &[i32]) -> Result<String> {
    serde_json::to_string(v).map_err(|e| Error::Internal(format!("买家秀 JSON 序列化失败: {e}")))
}

/// 按 key 把行分组，保持行内原有顺序（对应 Java `Collectors.groupingBy`
/// 配合同一条 SQL 的 `ORDER BY` 得到的稳定顺序）。
fn group_by<T, F>(rows: Vec<T>, key: F) -> HashMap<i32, Vec<T>>
where
    F: Fn(&T) -> i32,
{
    let mut out: HashMap<i32, Vec<T>> = HashMap::new();
    for row in rows {
        out.entry(key(&row)).or_default().push(row);
    }
    out
}

/// `BuyerShowGallery` 实体 → 返回体（字段一一对应）。
#[must_use]
fn gallery_to_vo(rows: &[BuyerShowGallery]) -> Vec<BuyerShowGalleryVO> {
    rows.iter()
        .map(|g| BuyerShowGalleryVO {
            pic_id: g.pic_id,
            buyer_show_id: g.buyer_show_id,
            pic_url: g.pic_url.clone(),
            pic_desc: g.pic_desc.clone(),
            pic_thumb: g.pic_thumb.clone(),
            pic_original: g.pic_original.clone(),
            pic_large: g.pic_large.clone(),
            sort_order: g.sort_order,
        })
        .collect()
}

/// `BuyerShowVideo` 实体 → 返回体。
#[must_use]
fn video_to_vo(rows: &[BuyerShowVideo]) -> Vec<BuyerShowVideoVO> {
    rows.iter()
        .map(|v| BuyerShowVideoVO {
            id: v.id,
            video_id: v.video_id,
            buyer_show_id: v.buyer_show_id,
            video_url: v.video_url.clone(),
            video_cover: v.video_cover.clone(),
            format: v.format.clone(),
            sort_order: v.sort_order,
        })
        .collect()
}

/// 组装视频流单项（Java `BuyerShowServiceImpl#toVideoFeedItem`）。
fn build_feed_item(
    bs: &BuyerShow,
    user_id: i32,
    bundle: &BuyerShowBundle,
    favorites: &[BuyerShowFavorite],
    comment_counts: &HashMap<i32, i64>,
    product_map: &HashMap<i32, BuyerShowProductVO>,
    followed: &HashSet<i32>,
) -> BuyerShowVideoFeedItemClientVO {
    let id = bs.buyer_show_id;

    let likes: Vec<&BuyerShowLike> = bundle
        .likes
        .iter()
        .filter(|l| l.buyer_show_id == id)
        .collect();
    let like_count = i64::try_from(likes.len()).unwrap_or(i64::MAX);
    let is_like = user_id > 0 && likes.iter().any(|l| l.user_id == user_id);

    let my_favorites: Vec<&BuyerShowFavorite> = favorites
        .iter()
        .filter(|f| f.buyer_show_id == id)
        .collect();
    let favorite_count = i64::try_from(my_favorites.len()).unwrap_or(i64::MAX);
    let is_favorite = user_id > 0 && my_favorites.iter().any(|f| f.user_id == user_id);

    // 视频取排序第一个（Java：`min(comparingInt(normSortOrder))`）；
    // SQL 已按 `sort_order, id` 升序，取第一条即等价。
    let video = bundle
        .videos_of(id)
        .first()
        .map(|first| BuyerShowFeedVideoVO {
            url: first.video_url.clone(),
            cover: first.video_cover.clone(),
            // width / height / duration 暂无采集
            width: None,
            height: None,
            duration: None,
        });

    let images: Vec<BuyerShowFeedImageVO> = bundle
        .gallery_of(id)
        .iter()
        .map(|g| BuyerShowFeedImageVO {
            url: g.pic_url.clone(),
            width: None,
            height: None,
        })
        .collect();

    // 作者：先用买家秀快照，命中会员再用会员资料覆盖（Java 顺序）
    let mut author = BuyerShowFeedAuthorVO {
        id: bs.user_id,
        name: bs.buyer_name.clone(),
        avatar: bs.buyer_photo.clone(),
        is_follow: i32::from(followed.contains(&bs.user_id)),
    };
    if bs.user_id > 0 {
        if let Some(u) = bundle.users.get(&bs.user_id) {
            author.name = user_name(u);
            author.avatar = u.avatar.clone();
        }
    }

    // 关联商品：按 range_data 顺序，缺失的跳过（Java `continue`）
    let products: Vec<BuyerShowFeedProductVO> = parse_i32_array_json(Some(bs.range_data.as_str()))
        .into_iter()
        .filter_map(|pid| {
            product_map.get(&pid).map(|p| BuyerShowFeedProductVO {
                product_id: p.product_id,
                sku_id: None,
                name: p.product_name.clone(),
                price: format_price(Some(p.product_price)),
                pic_url: p.pic_url.clone(),
                stock: p.product_stock,
            })
        })
        .collect();

    BuyerShowVideoFeedItemClientVO {
        id,
        video,
        images,
        author,
        products,
        content: bs.content.clone(),
        like_count,
        is_like: i32::from(is_like),
        favorite_count,
        is_favorite: i32::from(is_favorite),
        comment_count: comment_counts.get(&id).copied().unwrap_or(0),
        create_time: handel_time_with(bs.add_time, FEED_TIME_FORMAT),
    }
}

/// HTML 属性转义（Java `ShareHtmlUtil#escapeHtmlAttr`）。
#[must_use]
fn escape_html_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 16);
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 把可能的相对路径补全为绝对 URL（Java `ShareHtmlUtil#toAbsoluteUrl`）。
///
/// 已是 `http://` / `https://` 开头 → 原样返回；
/// 否则取 `h5Domain` 的 **origin**（协议 + host，去掉 path）后拼接。
#[must_use]
fn to_absolute_url(url: &str, h5_domain: &str) -> String {
    if url.trim().is_empty() {
        return String::new();
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    let mut origin = h5_domain;
    if let Some(scheme_end) = origin.find("://") {
        if let Some(path_start) = origin[scheme_end + 3..].find('/') {
            origin = &origin[..scheme_end + 3 + path_start];
        }
    }
    if origin.trim().is_empty() {
        return url.to_string();
    }
    if url.starts_with('/') {
        format!("{origin}{url}")
    } else {
        format!("{origin}/{url}")
    }
}

/// 构建带 Open Graph 标签的分享落地页（Java `ShareHtmlUtil#buildOgHtml`）。
///
/// 逐字符对齐 Java 的标签顺序与 JS 跳转脚本；`image` 为空时不输出
/// `og:image`；`ogType` 为空白时回落 `website`。
#[must_use]
fn build_og_html(
    title: &str,
    description: &str,
    image: &str,
    target_url: &str,
    og_type: &str,
) -> String {
    let safe_title = escape_html_attr(title);
    let safe_desc = escape_html_attr(description);
    let safe_image = escape_html_attr(image);
    let safe_url = escape_html_attr(target_url);
    let safe_type = escape_html_attr(if og_type.trim().is_empty() {
        "website"
    } else {
        og_type
    });
    // JS 字符串上下文转义（Java：`\\` → `\\\\`、`'` → `\'`、`<` → `\u003c`）
    let js_url = target_url
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('<', "\\u003c");

    let mut sb = String::with_capacity(512);
    sb.push_str("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    sb.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    sb.push_str("<meta property=\"og:type\" content=\"");
    sb.push_str(&safe_type);
    sb.push_str("\">");
    sb.push_str("<meta property=\"og:title\" content=\"");
    sb.push_str(&safe_title);
    sb.push_str("\">");
    sb.push_str("<meta property=\"og:description\" content=\"");
    sb.push_str(&safe_desc);
    sb.push_str("\">");
    if !safe_image.is_empty() {
        sb.push_str("<meta property=\"og:image\" content=\"");
        sb.push_str(&safe_image);
        sb.push_str("\">");
    }
    sb.push_str("<meta property=\"og:url\" content=\"");
    sb.push_str(&safe_url);
    sb.push_str("\">");
    sb.push_str("<meta name=\"twitter:card\" content=\"summary_large_image\">");
    sb.push_str("<title>");
    sb.push_str(&safe_title);
    sb.push_str("</title><script>location.replace('");
    sb.push_str(&js_url);
    sb.push_str("');</script></head><body><a href=\"");
    sb.push_str(&safe_url);
    sb.push_str("\">");
    sb.push_str(&safe_title);
    sb.push_str("</a></body></html>");
    sb
}

/// 分组列表返回体（Java `BuyerShowGroup` 实体的 Jackson 输出）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyerShowGroupVO {
    /// 自增 ID，买家秀分组 ID
    pub buyer_show_group_id: i32,
    /// 分组名称（Java 侧带 `@JsonTranslate(dataType = 16)`，未迁移 → 原文）
    pub group_name: String,
    /// 商品分组 id（json 文本）
    pub product_group_id: String,
    /// 店铺 id
    pub shop_id: i32,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use nextrade_core::entities::BuyerShow;

    use super::*;

    fn show(id: i32) -> BuyerShow {
        BuyerShow {
            buyer_show_id: id,
            range_data: "[]".into(),
            user_id: 0,
            buyer_photo: String::new(),
            buyer_name: String::new(),
            content: None,
            group_data: None,
            add_time: 0,
            sort_order: Some(0),
            status: 1,
            is_show: 1,
        }
    }

    // ————————————— 视频流分页窗口 —————————————

    #[test]
    fn split_feed_window_marks_more_when_extra_row_present() {
        // size = 3，取回 4 条 → 当前页 3 条 + hasMore = true
        let (rows, more) = split_feed_window(vec![show(1), show(2), show(3), show(4)], 3);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows.iter().map(|r| r.buyer_show_id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(more);

        // 恰好 size 条 → hasMore = false
        let (rows, more) = split_feed_window(vec![show(1), show(2), show(3)], 3);
        assert_eq!(rows.len(), 3);
        assert!(!more);

        // 少于 size 条 → hasMore = false
        let (rows, more) = split_feed_window(vec![show(1)], 3);
        assert_eq!(rows.len(), 1);
        assert!(!more);

        // 空结果
        let (rows, more) = split_feed_window(Vec::new(), 3);
        assert!(rows.is_empty());
        assert!(!more);
    }

    #[test]
    fn split_feed_window_handles_non_positive_size() {
        // size <= 0 时当前页为空，但有多余行 → hasMore = true（不 panic）
        let (rows, more) = split_feed_window(vec![show(1)], 0);
        assert!(rows.is_empty());
        assert!(more);
        let (rows, more) = split_feed_window(vec![show(1)], -5);
        assert!(rows.is_empty());
        assert!(more);
    }

    // ————————————— 游标解析 —————————————

    #[test]
    fn parse_cursor_follows_java_parse_int_semantics() {
        assert_eq!(parse_cursor(None), None);
        assert_eq!(parse_cursor(Some("")), None);
        assert_eq!(parse_cursor(Some("   ")), None);
        assert_eq!(parse_cursor(Some("abc")), None);
        assert_eq!(parse_cursor(Some("12abc")), None);
        assert_eq!(parse_cursor(Some(" 42 ")), Some(42));
        assert_eq!(parse_cursor(Some("-7")), Some(-7));
        // 超出 i32 → Java Integer.parseInt 抛 NumberFormatException → null
        assert_eq!(parse_cursor(Some("99999999999")), None);
    }

    // ————————————— 用户名校验 —————————————

    #[test]
    fn user_name_prefers_non_blank_nickname() {
        let mut u = User {
            username: "alice".into(),
            ..Default::default()
        };
        assert_eq!(user_name(&u), "alice");
        u.nickname = Some("   ".into());
        assert_eq!(user_name(&u), "alice", "空白昵称回退用户名");
        u.nickname = Some("小 A".into());
        assert_eq!(user_name(&u), "小 A");
    }

    // ————————————— 分享页 —————————————

    #[test]
    fn to_absolute_url_matches_java_rules() {
        assert_eq!(to_absolute_url("", "https://h5.example.com"), "");
        assert_eq!(
            to_absolute_url("https://cdn.example.com/a.jpg", "https://h5.example.com"),
            "https://cdn.example.com/a.jpg"
        );
        assert_eq!(
            to_absolute_url("/a.jpg", "https://h5.example.com"),
            "https://h5.example.com/a.jpg"
        );
        assert_eq!(
            to_absolute_url("a.jpg", "https://h5.example.com"),
            "https://h5.example.com/a.jpg"
        );
        // h5Domain 带 path → 取 origin
        assert_eq!(
            to_absolute_url("/a.jpg", "https://h5.example.com/app/"),
            "https://h5.example.com/a.jpg"
        );
        // h5Domain 为空 → 原样返回相对路径
        assert_eq!(to_absolute_url("/a.jpg", ""), "/a.jpg");
        // 无 scheme 的域名 → 不裁剪（Java `schemeEnd > 0` 判定）
        assert_eq!(to_absolute_url("a.jpg", "h5.example.com"), "h5.example.com/a.jpg");
    }

    #[test]
    fn escape_html_attr_covers_java_switch() {
        assert_eq!(
            escape_html_attr(r#"<a href="x">&'y'"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;y&#39;"
        );
        assert_eq!(escape_html_attr(""), "");
    }

    #[test]
    fn build_og_html_matches_java_structure() {
        let html = build_og_html(
            "标题\"X\"",
            "描述",
            "https://cdn/a.jpg",
            "https://h5/pages/buyerShow/buyerShowDetail?buyerShowId=5",
            "article",
        );
        assert!(html.starts_with("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">"));
        assert!(html.contains("<meta property=\"og:type\" content=\"article\">"));
        assert!(html.contains("<meta property=\"og:title\" content=\"标题&quot;X&quot;\">"));
        assert!(html.contains("<meta property=\"og:description\" content=\"描述\">"));
        assert!(html.contains("<meta property=\"og:image\" content=\"https://cdn/a.jpg\">"));
        assert!(html.contains("<meta name=\"twitter:card\" content=\"summary_large_image\">"));
        assert!(html.ends_with("</a></body></html>"));
        assert!(html.contains("location.replace('https://h5/pages/buyerShow/buyerShowDetail?buyerShowId=5')"));

        // 无图 → 不输出 og:image
        let html = build_og_html("t", "d", "", "https://h5/x", "");
        assert!(!html.contains("og:image"));
        // ogType 空白 → website
        assert!(html.contains("<meta property=\"og:type\" content=\"website\">"));
    }

    #[test]
    fn build_og_html_escapes_js_string_context() {
        let html = build_og_html("t", "d", "", "https://h5/a'b<c", "article");
        assert!(html.contains(r"location.replace('https://h5/a\'b\u003cc')"));
    }

    // ————————————— 分组聚合 —————————————

    #[test]
    fn group_by_keeps_sql_order_within_group() {
        let rows = vec![show(1), show(2), show(1)];
        let grouped = group_by(rows, |r| r.buyer_show_id);
        assert_eq!(grouped.get(&1).map(Vec::len), Some(2));
        assert_eq!(grouped.get(&2).map(Vec::len), Some(1));
        let one = grouped.get(&1).expect("有分组 1");
        assert_eq!(
            one.iter().map(|r| r.buyer_show_id).collect::<Vec<_>>(),
            vec![1, 1]
        );
    }

    // ————————————— 图片尺寸后缀 —————————————

    #[test]
    fn image_size_suffix_constants_match_java() {
        // Java `PicConstants`
        assert_eq!(OSS_SIZE_400, "?x-oss-process=image/resize,m_pad,h_500,h_500");
        assert_eq!(OSS_SIZE_800, "?x-oss-process=image/resize,m_pad,h_800,h_800");
        assert_eq!(COS_SIZE_400, "?imageMogr2/thumbnail/500x/");
        assert_eq!(COS_SIZE_800, "?imageMogr2/thumbnail/800x/");
        // Java `GalleryPicServiceImpl#getImageSize` 的七牛分支
        assert_eq!(QINIU_SIZE_400, "?imageView2/2/w/400");
        assert_eq!(QINIU_SIZE_800, "?imageView2/2/w/800");
        // 存储类型编码（Java `StorageType`）
        assert_eq!(STORAGE_LOCAL, "0");
        assert_eq!(STORAGE_COS, "2");
        assert_eq!(STORAGE_QINIU, "3");
        // 本项目 `pure.sql` 种子值：storageType = '1'（阿里云 OSS）→ default 分支
        assert_eq!(CFG_STORAGE_TYPE, "storageType");
        assert_eq!(CFG_H5_DOMAIN, "h5Domain");
    }

    #[test]
    fn default_sort_order_matches_java() {
        assert_eq!(DEFAULT_SORT_ORDER, 0);
    }

    // ————————————— 校验 —————————————

    #[test]
    fn require_i32_maps_missing_to_service_data_error() {
        let e = require_i32(None, "买家秀信息错误").expect_err("应报错");
        match e {
            Error::Biz { code, msg } => {
                assert_eq!(code, SERVICE_DATA_ERROR);
                assert_eq!(msg, "买家秀信息错误");
            }
            other => panic!("期望 Biz 错误，实际 {other:?}"),
        }
        assert_eq!(require_i32(Some(7), "x").expect("应有值"), 7);
    }

    #[test]
    fn to_json_i32_array_matches_jackson_output() {
        assert_eq!(to_json_i32_array(&[]).unwrap(), "[]");
        assert_eq!(to_json_i32_array(&[1, 2, 3]).unwrap(), "[1,2,3]");
    }

    #[test]
    fn business_codes_match_java() {
        assert_eq!(SERVICE_DATA_ERROR, 1001);
        assert_eq!(CODE_UNAUTHORIZED, 401);
    }

    #[test]
    fn format_price_is_reused_from_models() {
        assert_eq!(format_price(None), "0.00");
        assert_eq!(
            format_price(Some(Decimal::from_str_exact("12.5").unwrap())),
            "12.50"
        );
    }

    // ————————————— 加载聚合（纯结构） —————————————

    #[test]
    fn bundle_getters_fall_back_to_empty_slices() {
        let bundle = BuyerShowBundle::default();
        assert!(bundle.videos_of(99).is_empty());
        assert!(bundle.gallery_of(99).is_empty());
    }
}