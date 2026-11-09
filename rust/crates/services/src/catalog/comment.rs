//! 评价（商品评论 / 晒单）服务：管理端评价管理 + 前台评价/晒单/数量角标。
//!
//! 对应原 Java `CommentServiceImpl`（`com.nextrade.service.product.impl`，
//! 615 行）与 `CommentService` 接口：
//!
//! | 方法 | Java | 说明 |
//! |------|------|------|
//! | [`CommentService::list`] | `list(CommentListPageQuery)` | 管理端/前台共用分页 |
//! | [`CommentService::list_for_client`] | `listForClient` | D1146：开关守卫 + 匿名脱敏 |
//! | [`CommentService::detail`] | `detail(Integer)` | 详情 + 子回复 |
//! | [`CommentService::create`] | `create(CommentSaveParam)` | 后台补录评价 |
//! | [`CommentService::update`] | `update(CommentUpdateDTO)` | 后台编辑评价 |
//! | [`CommentService::delete`] | `del(Integer)` | 物理删除 |
//! | [`CommentService::update_field`] | `updateField(.., COMMENT_FIELDS)` | 字段白名单 |
//! | [`CommentService::batch`] | `batch(BatchDTO)` | 仅 `type = "del"` |
//! | [`CommentService::reply_comment`] | `replyComment` | 商家回复（有则改、无则增） |
//! | [`CommentService::sub_num`] | `subNum(Integer)` | 待评价 / 已评价 / 待晒单角标 |
//! | [`CommentService::evaluate`] | `evaluate` | 前台评价 / 追加晒单 |
//!
//! # 关键 Java 语义
//!
//! 1. **MyBatis-Plus `NOT_NULL` 字段策略**：Java 用 `save` / `updateById`，
//!    DTO 中为 `null` 的列会被**排除出 SQL**，于是 `create` 套用 DDL 默认值
//!    （`sort_order = 50`、`status = 0`、`is_showed = 0` …），`update` 保留库中原值。
//!    Rust 显式列出列，故在 [`CommentService::create`] / [`CommentService::update`]
//!    里显式补齐同样的默认值 / 原值。注意 `update` 里 Java 会先把
//!    `userId` / `shopId` 缺省补 **0** 再 `BeanUtils.copyProperties`，
//!    因此「不传 user/shopId 的编辑」在 Java 里会把这两列写成 0 —— 本实现如实复刻。
//! 2. **`updateField` 白名单**：Java `CheckFieldConstants.COMMENT_FIELDS` 含
//!    `is_show`，但 `comment` 表无该列；`getColumnByPropertyName` 找不到属性时
//!    回退主键 `comment_id`，随后白名单校验失败 → 「field 的值有误」。
//!    本实现等价为只允许 `COMMENT_UPDATABLE_FIELDS`。
//! 3. **评价开关（D1146）**：`productCommentEnabled` 缺失时按**开启**处理；
//!    `commentCheck = "1"` 时新评价为待审核（`status = 0`），否则直接通过（`status = 1`）。
//!    `anonymousCommentEnabled = "0"` 时强制 `isAnonymous = 0`。
//! 4. **XSS**：Java `XssUtils.clean` 用于回复 / 前台评价内容；
//!    工作区已有等价纯函数 [`nextrade_core::models::member::feedback::xss_clean`]，
//!    直接复用（同一套 9 条正则的复刻）。
//!
//! # 不确定点（未能逐字节复刻的 Java 行为）
//!
//! - **`translatePackage.translate`**：列表把 `commentTag` 逐项翻译成当前语言的
//!   标签文案；翻译域未迁移，这里只解析原始 JSON 字符串数组。
//! - **管理操作日志**：`update` 的 `adminLogService.createByLogInfo(...)` 未迁移
//!   （identity 日志域尚无写接口）。
//! - **评论送积分 / 晒单送积分 / 会员等级重算**：`evaluate` 末尾调用
//!   `userPointsLogService.incPoints` 与 `userRankService.updateUserRank`；
//!   `member::user_points_log` 仓储当前只有查询方法（`points_of_user`），
//!   无法在不改共享仓储的前提下写积分流水，故本实现**跳过**这两个副作用
//!   （`order.comment_status` 的整单已评标记仍已实现）。
//! - **多租户店铺过滤**：Java `list` 用 `HeaderUtils.getShopId()`（请求头）决定
//!   是否强制 `shop_id = 本店`；Rust JWT 尚未签发 `shop_id`，当前没有等价来源，
//!   仅能按查询参数 `shopId` 过滤。**这不是等价实现**：店铺账号不传 `shopId`
//!   时会看到全部店铺的评价。待鉴权中间件补齐后必须改回按登录态判定。
//! - **`replyComment` 的 `getOne`**：Java 命中多行会抛 TooManyResults；
//!   `comment` 表没有 `parent_id` 唯一约束，本实现取第一条（`ORDER BY comment_id`），
//!   避免历史脏数据让商家无法回复。
//! - **日期解析**：Java `DateUtil.parse` 支持更多格式；这里支持
//!   `yyyy-MM-dd HH:mm:ss` / `yyyy-MM-dd HH:mm` / `yyyy-MM-dd` 三种，
//!   解析失败返回参数校验错误。
//! - **业务码**：Java `GlobalException` 默认业务码 `SERVICE_DATA_ERROR = 1001`；
//!   本文件业务错误统一 `Error::biz(1001, ..)`，Bean Validation 类错误用
//!   `Error::Validation`（HTTP 400）。
//! - **`create` 未传 `content`**：Java `comment.content` 是 `NOT NULL` 且无默认值，
//!   原实现会以数据库错误失败；本实现写入空串（运营补录场景更宽容）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{NaiveDate, NaiveDateTime, TimeZone};

use nextrade_common::utils;
use nextrade_core::entities::{Comment, OrderItem, Product};
use nextrade_core::models::catalog::comment::{
    build_comment_vo, mask_name, normalize_update_field, validate_evaluate_content,
    validate_update_content, BatchDTO, CommentEvaluateParam, CommentListQuery, CommentRankFilter,
    CommentReplyDTO, CommentSaveParam, CommentSubNumVO, CommentUpdateDTO, CommentVO,
    UpdateFieldDTO, COMMENT_STATUS_APPROVED, COMMENT_STATUS_PENDING, DEFAULT_SORT_ORDER,
    NOT_ANONYMOUS, NOT_SHOWED, SHOWED,
};
use nextrade_core::models::member::feedback::xss_clean;
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::comment::{CommentFilters, RankFilter};
use nextrade_infrastructure::db::Repositories;

/// 系统配置项（对应 Java `SettingsEnum`）。
pub const CFG_PRODUCT_COMMENT_ENABLED: &str = "productCommentEnabled";
/// 是否开启匿名评价。
pub const CFG_ANONYMOUS_COMMENT_ENABLED: &str = "anonymousCommentEnabled";
/// 评价审核开关（`"1"` = 需要审核）。
pub const CFG_COMMENT_CHECK: &str = "commentCheck";
/// 会员默认头像。
pub const CFG_DEFAULT_AVATAR: &str = "defaultAvatar";

/// `order.order_status = 5`（已完成，`OrderStatusEnum.COMPLETED`）。
pub const ORDER_STATUS_COMPLETED: i32 = 5;
/// `order.comment_status = 0`（待评价，`CommentStatus.PENDING`）。
pub const ORDER_COMMENT_STATUS_PENDING: i32 = 0;

pub struct CommentService {
    repos: Arc<Repositories>,
}

impl CommentService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 评价分页（管理端 `GET /adminapi/product/comment/list`，
    /// 前台 `GET /api/user/comment/list` 复用）。
    ///
    /// 对应 Java `CommentServiceImpl#list`：过滤条件见仓储
    /// [`CommentFilters`]；列表为空时 Java 返回 `new Page<>()`（total = 0），
    /// 这里同样返回空分页。
    pub async fn list(&self, q: &CommentListQuery) -> Result<PageResult<CommentVO>> {
        let page = q.page_query();
        let filters = CommentFilters {
            is_showed: q.is_showed,
            product_id: q.product_id,
            shop_id: q.shop_id,
            keyword: q.keyword.as_deref(),
            user_id: q.user_id,
            rank: rank_filter_of(q.rank_filter()),
            // Java `getShopId() > 0`：Rust 无登录态店铺，暂时置空（见模块头不确定点）。
            shop_scope: None,
            sort_field: q.sort_field.as_deref(),
            sort_order: q.sort_order.as_deref(),
        };
        let (total, rows) = self
            .repos.catalog.comment
            .paginate(&filters, &page)
            .await?;
        if rows.is_empty() {
            // Java：`if (CollUtil.isEmpty(records)) return new Page<>();`
            return Ok(PageResult::empty(&page));
        }

        // 批量预取商品（Java `productMapper.selectBatchIds`）
        let product_ids: Vec<i32> = rows
            .iter()
            .map(|c| c.product_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let product_map: HashMap<i32, Product> = self
            .repos.catalog.comment
            .find_products(&product_ids)
            .await?
            .into_iter()
            .map(|p| (p.product_id, p))
            .collect();

        // 子评论（Java `in(parentId, commentIds)` + groupingBy）
        let comment_ids: Vec<i32> = rows.iter().map(|c| c.comment_id).collect();
        let replies = self
            .repos.catalog.comment
            .find_replies(&comment_ids)
            .await?;
        let mut reply_map: HashMap<i32, Vec<Comment>> = HashMap::new();
        for r in replies {
            reply_map.entry(r.parent_id).or_default().push(r);
        }

        // 批量预取订单项（Java `batchGetOrderItemMap`，补 skuId / skuData）
        let order_item_ids: Vec<i32> = rows
            .iter()
            .map(|c| c.order_item_id)
            .filter(|id| *id > 0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let order_item_map: HashMap<i32, OrderItem> = self
            .repos.catalog.comment
            .find_order_items(&order_item_ids)
            .await?
            .into_iter()
            .map(|it| (it.item_id, it))
            .collect();

        let default_avatar = self.default_avatar().await?;
        let list = rows
            .iter()
            .map(|c| {
                build_comment_vo(
                    c,
                    product_map.get(&c.product_id),
                    reply_map.get(&c.comment_id).map(Vec::as_slice).unwrap_or(&[]),
                    order_item_map.get(&c.order_item_id),
                    &default_avatar,
                )
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 前台评价列表（D1146）：评价功能关闭 → 空分页；匿名评价用户名脱敏。
    ///
    /// 对应 Java `CommentServiceImpl#listForClient`：
    /// 先走 [`CommentService::list`]，再对 `is_anonymous = 1` 的父/子评价调用
    /// [`mask_name`]（只看记录自身 `isAnonymous`，与当前匿名开关无关）。
    pub async fn list_for_client(&self, q: &CommentListQuery) -> Result<PageResult<CommentVO>> {
        if !self.is_product_comment_enabled().await? {
            return Ok(PageResult::empty(&q.page_query()));
        }
        let mut page = self.list(q).await?;
        for vo in &mut page.list {
            mask_anonymous_comment(vo);
        }
        Ok(page)
    }

    /// 评价详情（含子回复；不含商品 / 订单项填充，与 Java `detail` 一致）。
    ///
    /// 对应 Java `detail`：查不到抛「评价不存在」（`GlobalException` → 业务码 1001）。
    pub async fn detail(&self, id: i32) -> Result<CommentVO> {
        let c = self
            .repos.catalog.comment
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(1001, "评价不存在"))?;
        let replies = self.repos.catalog.comment.find_replies(&[id]).await?;
        let default_avatar = self.default_avatar().await?;
        Ok(build_comment_vo(&c, None, &replies, None, &default_avatar))
    }

    // ————————————————————— 管理端写操作 —————————————————————

    /// 管理端新增评价（`POST /adminapi/product/comment/create`）。
    ///
    /// 对应 Java `CommentServiceImpl#create`：
    /// 1. 按 `productId` 查商品所属店铺并写入 `comment.shop_id`；
    /// 2. 其余字段走 `CommentSaveParam.createComment()`（标签 JSON、晒图 JSON、
    ///    `addTime` 秒级解析）；
    /// 3. 未传列按 DDL 默认值落库（`sort_order = 50` 等）。
    pub async fn create(&self, param: &CommentSaveParam) -> Result<i32> {
        let product_id = param
            .product_id
            .ok_or_else(|| Error::Validation("产品ID不能为空".into()))?;
        let shop_id = self
            .repos.catalog.comment
            .find_product_shop_id(product_id)
            .await?
            .ok_or_else(|| Error::biz(1001, "商品不存在"))?;

        let comment_tag = match param.comment_tag.as_ref().filter(|v| !v.is_empty()) {
            Some(tags) => Some(to_json(tags)?),
            None => None,
        };
        let show_pics = match param.show_pics.as_ref().filter(|v| !v.is_empty()) {
            Some(pics) => to_json(pics)?,
            None => "[]".to_string(),
        };

        let comment = Comment {
            comment_id: 0,
            user_id: 0,
            username: param.username.clone().unwrap_or_default(),
            is_anonymous: NOT_ANONYMOUS,
            avatar: param.avatar.clone().unwrap_or_default(),
            product_id,
            order_id: 0,
            order_item_id: 0,
            comment_rank: param.comment_rank.unwrap_or(0),
            content: param.content.clone().unwrap_or_default(),
            add_time: parse_add_time_seconds(param.add_time.as_deref())?,
            status: COMMENT_STATUS_PENDING,
            parent_id: 0,
            usefull: 0,
            useless: 0,
            comment_tag,
            show_pics: Some(show_pics),
            is_recommend: param.is_recommend.or(Some(0)),
            is_top: param.is_top.unwrap_or(0),
            is_showed: NOT_SHOWED,
            is_default: 0,
            // DDL: `sort_order mediumint NOT NULL DEFAULT 50`
            sort_order: param.sort_order.unwrap_or(DEFAULT_SORT_ORDER),
            shop_id,
        };
        let id = self.repos.catalog.comment.insert(&comment).await?;
        tracing::info!("新增评价成功: id={id}, product_id={product_id}");
        Ok(id)
    }

    /// 管理端编辑评价（`POST /adminapi/product/comment/update`）。
    ///
    /// 对应 Java `CommentServiceImpl#update`：
    /// - `commentId` 必填（否则 Java `updateById` 直接抛异常）；
    /// - `content` 未传 → 保留库中原值（Java 未加 `@Validated`，
    ///   `@NotNull` 实际不生效，`BeanUtils` 复制 null 后被 `NOT_NULL` 策略剔除）；
    /// - `userId` / `shopId` 缺省补 `0` 并写库（Java `BeanUtils` + 显式 set）；
    /// - `showPics` 非空 → 强制 `isShowed = 1`；`showPics` 为空 → 写 `"[]"`；
    /// - `addTime` 有值则解析覆盖，否则置当前时间；
    /// - 其余未传列保留原值（MyBatis `NOT_NULL` 语义）。
    pub async fn update(&self, dto: &CommentUpdateDTO) -> Result<()> {
        let comment_id = dto
            .comment_id
            .ok_or_else(|| Error::Validation("评价管理id不能为空".into()))?;

        let existing = self
            .repos.catalog.comment
            .find_by_id(comment_id)
            .await?
            .ok_or_else(|| Error::biz(1001, "评价不存在"))?;

        // Java 控制器 update 未加 @Valid/@Validated：`content` 未传时保留原值；
        // 传了则照写（仅保留 500 字符上限的防御性校验）
        let content = validate_update_content(dto.content.as_deref())?
            .unwrap_or_else(|| existing.content.clone());

        // Java：`updateDTO.setUserId(userId == null ? 0 : userId)`（shopId 同理）
        let user_id = dto.user_id.unwrap_or(0);
        let shop_id = dto.shop_id.unwrap_or(0);
        let mut is_showed = dto.is_showed.unwrap_or(existing.is_showed);
        if dto.show_pics.as_ref().is_some_and(|v| !v.is_empty()) {
            is_showed = SHOWED;
        }
        let add_time = match dto.add_time.as_deref().map(str::trim) {
            Some(s) if !s.is_empty() => parse_datetime_seconds(s)?,
            _ => utils::now_seconds() as i32,
        };
        let comment_tag = match dto.comment_tag.as_ref().filter(|v| !v.is_empty()) {
            Some(tags) => Some(to_json(tags)?),
            // Java 未传时 `BeanUtils` 复制 null → 不进入 SET 子句 → 保留原值
            None => existing.comment_tag.clone(),
        };
        let show_pics = match dto.show_pics.as_ref().filter(|v| !v.is_empty()) {
            Some(pics) => to_json(pics)?,
            None => "[]".to_string(),
        };

        let comment = Comment {
            comment_id,
            user_id,
            username: dto.username.clone().unwrap_or_else(|| existing.username.clone()),
            is_anonymous: existing.is_anonymous,
            avatar: dto.avatar.clone().unwrap_or_else(|| existing.avatar.clone()),
            product_id: dto.product_id.unwrap_or(existing.product_id),
            order_id: dto.order_id.unwrap_or(existing.order_id),
            order_item_id: dto.order_item_id.unwrap_or(existing.order_item_id),
            comment_rank: dto.comment_rank.unwrap_or(existing.comment_rank),
            content,
            add_time,
            status: dto.status.unwrap_or(existing.status),
            parent_id: dto.parent_id.unwrap_or(existing.parent_id),
            usefull: dto.usefull.unwrap_or(existing.usefull),
            useless: dto.useless.unwrap_or(existing.useless),
            comment_tag,
            show_pics: Some(show_pics),
            is_recommend: dto.is_recommend.or(existing.is_recommend),
            is_top: dto.is_top.unwrap_or(existing.is_top),
            is_showed,
            is_default: dto.is_default.unwrap_or(existing.is_default),
            sort_order: dto.sort_order.unwrap_or(existing.sort_order),
            shop_id,
        };
        let affected = self.repos.catalog.comment.update(&comment).await?;
        if affected == 0 {
            // Java `updateById` 影响 0 行会静默返回 false，接口仍 200
            tracing::warn!("更新评价未命中任何行: comment_id={comment_id}");
        } else {
            let log_info = format!("更新评论晒单:{}", comment.content);
            let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
            if let Err(e) = admin_log.record_current_log(if comment.shop_id > 0 { "shop" } else { "admin" }, comment.shop_id, &log_info).await {
                tracing::warn!("写管理操作日志失败: {e}");
            }
            tracing::info!("更新评价成功: id={comment_id}");
        }
        Ok(())
    }

    /// 删除单条评价（`POST /adminapi/product/comment/del`，物理删除）。
    pub async fn delete(&self, id: i32) -> Result<()> {
        let deleted = self.repos.catalog.comment.delete(id).await?;
        if !deleted {
            // Java `BaseServiceImpl#del` 返回 false，接口仍 200
            tracing::warn!("删除评价未命中: comment_id={id}");
        }
        Ok(())
    }

    /// 更新单个字段（`POST /adminapi/product/comment/updateField`）。
    ///
    /// 白名单：`comment_rank` / `is_recommend` / `is_top` / `sort_order`
    /// （`is_show` 虽在 Java 常量里但表无此列，等价于 Java 的「field 的值有误」）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("id不能为空".into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("值不能为空".into()))?;
        let column = normalize_update_field(field)
            .ok_or_else(|| Error::biz(1001, "field 的值有误"))?;
        self.repos.catalog.comment
            .update_field(id, column, val)
            .await?;
        Ok(())
    }

    /// 批量操作（`POST /adminapi/product/comment/batch`）。
    ///
    /// 对应 Java `BaseServiceImpl#batch`：仅支持 `type = "del"`，
    /// 其它类型返回 false（本实现返回 0 行，接口仍 200）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<u64> {
        if dto.r#type.as_deref() != Some("del") {
            tracing::warn!("批量操作类型不受支持: {:?}", dto.r#type);
            return Ok(0);
        }
        let empty: [i32; 0] = [];
        let ids = dto.ids.as_deref().unwrap_or(&empty);
        let n = self.repos.catalog.comment.delete_batch(ids).await?;
        tracing::info!("批量删除评价: {n} 条");
        Ok(n)
    }

    /// 商家回复评价（`POST /adminapi/product/comment/replyComment`）。
    ///
    /// 对应 Java `CommentServiceImpl#replyComment`：
    /// - 内容先做 XSS 清洗；
    /// - `parent_id = dto.commentId` 已有回复 → 覆盖内容 / 用户名 / 时间 / `user_id`；
    /// - 否则新增回复行，并把父评价 `status` 置 1；
    /// - 回复行的 `isAnonymous` / `isShowed` 等取 DDL 默认（父评价为回复不脱敏）。
    pub async fn reply_comment(
        &self,
        dto: &CommentReplyDTO,
        admin_id: i32,
        admin_username: &str,
    ) -> Result<()> {
        let parent_id = dto
            .comment_id
            .ok_or_else(|| Error::Validation("评价ID不能为空".into()))?;
        let content = xss_clean(dto.content.as_deref().unwrap_or_default());
        let now = utils::now_seconds() as i32;

        if let Some(mut reply) = self.repos.catalog.comment.find_one_reply(parent_id).await? {
            reply.content = content;
            reply.username = admin_username.to_string();
            reply.add_time = now;
            reply.user_id = admin_id;
            self.repos.catalog.comment.update(&reply).await?;
            tracing::info!("更新评价回复: parent_id={parent_id}");
        } else {
            let reply = Comment {
                comment_id: 0,
                user_id: admin_id,
                username: admin_username.to_string(),
                content,
                add_time: now,
                parent_id,
                // Java 未写这些列 → 取 DDL 默认值（is_recommend=0、sort_order=50）
                is_recommend: Some(0),
                sort_order: DEFAULT_SORT_ORDER,
                ..Default::default()
            };
            self.repos.catalog.comment.insert(&reply).await?;
            // Java：新增回复成功后把父评价审核状态置 1（`updateById`）
            let parent = self
                .repos.catalog.comment
                .find_by_id(parent_id)
                .await?
                .ok_or_else(|| Error::biz(1001, "评价不存在"))?;
            self.repos.catalog.comment
                .set_status(parent.comment_id, COMMENT_STATUS_APPROVED)
                .await?;
            tracing::info!("新增评价回复: parent_id={parent_id}");
        }
        Ok(())
    }

    // ————————————————————— 前台 —————————————————————

    /// 评论晒单数量（`GET /api/user/comment/subNum`）。
    ///
    /// 对应 Java `CommentServiceImpl#subNum`：
    /// - 评价功能关闭 → 三项全 0（D1146）；
    /// - 待评价 = 已完成且 `comment_status = 0` 的订单数；
    /// - 已评价 = 该会员父评价数；
    /// - 待晒单 = 该会员全部订单下 `is_showed = 0` 的父评价数。
    pub async fn sub_num(&self, user_id: i32) -> Result<CommentSubNumVO> {
        if !self.is_product_comment_enabled().await? {
            return Ok(CommentSubNumVO::default());
        }
        let order_ids = self
            .repos.catalog.comment
            .find_order_ids_by_user(user_id)
            .await?;
        let show_pics = self
            .repos.catalog.comment
            .count_pending_show_pics(&order_ids, user_id)
            .await?;
        let await_comment = self
            .repos.catalog.comment
            .count_await_comment(user_id)
            .await?;
        let commented = self
            .repos.catalog.comment
            .count_comments_by_user(user_id)
            .await?;
        Ok(CommentSubNumVO {
            await_comment,
            show_pics,
            commented,
        })
    }

    /// 前台评价 / 晒单（`POST /api/user/comment/evaluate`）。
    ///
    /// 对应 Java `CommentServiceImpl#evaluate`（`@Transactional`）：
    /// 1. 评价功能关闭 → 「评价功能已关闭」；匿名开关关闭 → 强制 `isAnonymous = 0`；
    /// 2. 会员必须存在（否则「用户不存在」）；
    /// 3. 内容 XSS 清洗；按 `(userId, orderItemId, orderId, parentId=0)` 查已有评价：
    ///    - 无 → 新增（`status` 由 `commentCheck` 决定；有晒图则 `isShowed = 1`）；
    ///    - 有且已晒单（`isShowed > 0`）→ 「您已评价完成，不能重复评价」；
    ///    - 有且未晒单但本次没传晒图 → 「请上传晒单图片」；
    ///    - 有且未晒单且传了晒图 → 追加晒图并置 `isShowed = 1`；
    /// 4. 若该订单全部订单项都已评价 → `order.comment_status = 1`。
    ///
    /// ⚠️ 评论送积分 / 晒单送积分 / 会员等级重算未迁移（见模块头不确定点）。
    pub async fn evaluate(&self, param: &CommentEvaluateParam, user_id: i32) -> Result<()> {
        if !self.is_product_comment_enabled().await? {
            return Err(Error::biz(1001, "评价功能已关闭"));
        }
        validate_evaluate_content(param.content.as_deref())?;

        let anonymous_enabled = self.is_anonymous_comment_enabled().await?;
        let is_anonymous = if anonymous_enabled {
            param.is_anonymous.unwrap_or(NOT_ANONYMOUS)
        } else {
            NOT_ANONYMOUS
        };

        let user = self
            .repos.member.user
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| Error::biz(1001, "用户不存在"))?;

        let content = xss_clean(param.content.as_deref().unwrap_or_default());
        let order_id = param.order_id.unwrap_or(0);
        let order_item_id = param.order_item_id.unwrap_or(0);
        let show_pics_json = match param.show_pics.as_ref().filter(|v| !v.is_empty()) {
            Some(pics) => Some(to_json(pics)?),
            None => None,
        };

        let existing = self
            .repos.catalog.comment
            .find_one_by_order_item(user_id, order_id, order_item_id)
            .await?;

        if let Some(comment) = existing {
            // 已有评价：只允许「追加晒单」，且晒单不可重复
            if comment.is_showed > 0 {
                return Err(Error::biz(1001, "您已评价完成，不能重复评价"));
            }
            let pics = show_pics_json.ok_or_else(|| Error::biz(1001, "请上传晒单图片"))?;
            self.repos.catalog.comment
                .update_show_pics(comment.comment_id, &pics, SHOWED)
                .await?;
        } else {
            // 新增评价：Java `param.createComment(user)` + `commentCheck` 决定 status
            let comment_tag = match param.comment_tag.as_ref().filter(|v| !v.is_empty()) {
                Some(tags) => to_json(tags)?,
                None => "[]".to_string(),
            };
            let show_pics = show_pics_json.clone().unwrap_or_else(|| "[]".to_string());
            let is_showed = if param.show_pics.as_ref().is_some_and(|v| !v.is_empty()) {
                SHOWED
            } else {
                NOT_SHOWED
            };
            let comment = Comment {
                comment_id: 0,
                user_id,
                username: user.username.clone(),
                is_anonymous,
                avatar: user.avatar.clone(),
                product_id: param.product_id.unwrap_or(0),
                order_id,
                order_item_id,
                comment_rank: param.comment_rank.unwrap_or(0),
                content,
                add_time: utils::now_seconds() as i32,
                status: self.comment_initial_status().await?,
                parent_id: 0,
                usefull: 0,
                useless: 0,
                comment_tag: Some(comment_tag),
                show_pics: Some(show_pics),
                // Java 未写该列 → DDL 默认 0
                is_recommend: Some(0),
                is_top: 0,
                is_showed,
                is_default: 0,
                // Java 未写该列 → 由 DDL 默认值 50 兜底
                sort_order: DEFAULT_SORT_ORDER,
                shop_id: param.shop_id.unwrap_or(0),
            };
            self.repos.catalog.comment.insert(&comment).await?;
        }

        // 整单评价状态：`commentCount == orderItemIds.size()`
        let item_ids = self
            .repos.catalog.comment
            .find_order_item_ids(order_id, user_id)
            .await?;
        let comment_count = self
            .repos.catalog.comment
            .count_commented_order_items(order_id, user_id, &item_ids)
            .await?;
        if comment_count == item_ids.len() as i64 {
            self.repos.catalog.comment
                .mark_order_commented(order_id)
                .await?;
            tracing::info!("订单商品已全部评价: order_id={order_id}, user_id={user_id}");
        }
        Ok(())
    }

    // ————————————————————— 配置 —————————————————————

    /// 商品评价功能是否开启（D1146）。
    ///
    /// Java：`!"0".equals(configService.getConfigVal(PRODUCT_COMMENT_ENABLED))`
    /// —— 配置缺失时按开启处理。
    pub async fn is_product_comment_enabled(&self) -> Result<bool> {
        let raw = self
            .repos.catalog.comment
            .find_config_val(CFG_PRODUCT_COMMENT_ENABLED)
            .await?;
        Ok(!is_config_zero(raw.as_deref()))
    }

    /// 匿名评价功能是否开启。
    async fn is_anonymous_comment_enabled(&self) -> Result<bool> {
        let raw = self
            .repos.catalog.comment
            .find_config_val(CFG_ANONYMOUS_COMMENT_ENABLED)
            .await?;
        Ok(!is_config_zero(raw.as_deref()))
    }

    /// 新评价初始审核状态：`commentCheck == "1"` → 待审核，否则审核通过。
    async fn comment_initial_status(&self) -> Result<i32> {
        let raw = self
            .repos.catalog.comment
            .find_config_val(CFG_COMMENT_CHECK)
            .await?;
        Ok(initial_status(raw.as_deref()))
    }

    /// 会员默认头像（Java `configService.getConfigByCode(DEFAULT_AVATAR).getBizVal()`）。
    async fn default_avatar(&self) -> Result<String> {
        Ok(self
            .repos.catalog.comment
            .find_config_val(CFG_DEFAULT_AVATAR)
            .await?
            .unwrap_or_default())
    }
}

// ————————————————————— 纯函数（可单测，无 DB/IO） —————————————————————

/// 配置值是否为 `"0"`（Java `"0".equals(val)`；`null` / 其它值均为 false）。
#[must_use]
fn is_config_zero(raw: Option<&str>) -> bool {
    raw.map(str::trim) == Some("0")
}

/// 新评价初始审核状态（Java `"1".equals(commentCheck) ? YES : NO`）。
#[must_use]
fn initial_status(comment_check: Option<&str>) -> i32 {
    if comment_check.map(str::trim) == Some("1") {
        COMMENT_STATUS_APPROVED
    } else {
        COMMENT_STATUS_PENDING
    }
}

/// 评价类型（核心模型）→ 仓储过滤条件。
#[must_use]
fn rank_filter_of(filter: Option<CommentRankFilter>) -> Option<RankFilter> {
    match filter {
        Some(CommentRankFilter::Good) => Some(RankFilter::Good),
        Some(CommentRankFilter::Neutral) => Some(RankFilter::Neutral),
        Some(CommentRankFilter::Bad) => Some(RankFilter::Bad),
        None => None,
    }
}

/// 匿名评价脱敏（Java `maskAnonymousUsername`：只看记录自身 `isAnonymous`）。
fn mask_anonymous_comment(vo: &mut CommentVO) {
    if vo.is_anonymous == Some(1) {
        if let Some(name) = vo.username.as_deref() {
            vo.username = Some(mask_name(name));
        }
    }
    if let Some(replies) = vo.replies.as_mut() {
        for reply in replies {
            if reply.is_anonymous == Some(1) {
                if let Some(name) = reply.username.as_deref() {
                    reply.username = Some(mask_name(name));
                }
            }
        }
    }
}

/// 晒单 / 标签等嵌套结构序列化为 JSON 字符串（Java `JSON.toJSONString`）。
fn to_json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|e| Error::Internal(format!("JSON 序列化失败: {e}")))
}

/// `add_time` 解析：Java `CommentSaveParam.createComment()`
/// `StrUtil.isNotBlank(addTime) ? DateUtil.parse(addTime).getTime()/1000
/// : StringUtils.getCurrentTime()`。
fn parse_add_time_seconds(raw: Option<&str>) -> Result<i32> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => parse_datetime_seconds(s),
        None => Ok(utils::now_seconds() as i32),
    }
}

/// 日期时间字符串 → 秒级时间戳（本地时区）。
///
/// 支持 `yyyy-MM-dd HH:mm:ss` / `yyyy-MM-dd HH:mm` / `yyyy-MM-dd` 三种
/// （Java `DateUtil.parse` 的常用子集）。
fn parse_datetime_seconds(s: &str) -> Result<i32> {
    let trimmed = s.trim();
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return local_seconds(dt)
                .ok_or_else(|| Error::Validation(format!("时间不合法: {trimmed}")));
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        if let Some(dt) = date.and_hms_opt(0, 0, 0) {
            return local_seconds(dt)
                .ok_or_else(|| Error::Validation(format!("时间不合法: {trimmed}")));
        }
    }
    Err(Error::Validation(format!("时间格式不正确: {trimmed}")))
}

/// 本地 `NaiveDateTime` → 秒级时间戳（DST 歧义取最早，Java 默认时区语义）。
fn local_seconds(dt: NaiveDateTime) -> Option<i32> {
    chrono::Local
        .from_local_datetime(&dt)
        .earliest()
        .map(|d| d.timestamp() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::comment::{fmt_time_seconds, CommentReplyVO};

    // ————————————————————— 配置开关语义（D1146） —————————————————————

    #[test]
    fn config_zero_semantics_matches_java_equals() {
        // Java `!"0".equals(val)`：缺失 / null / 其它值都按开启
        assert!(!is_config_zero(None));
        assert!(!is_config_zero(Some("1")));
        assert!(!is_config_zero(Some("true")));
        // 空白 + "0" 视为关闭（Java 未 trim，这里 trim 后比较更宽容，见不确定点）
        assert!(is_config_zero(Some("0")));
        assert!(is_config_zero(Some(" 0 ")));
    }

    #[test]
    fn initial_status_follows_comment_check() {
        // Java `"1".equals(commentCheck) ? Constants.YES : Constants.NO`
        assert_eq!(initial_status(Some("1")), COMMENT_STATUS_APPROVED);
        assert_eq!(initial_status(Some("0")), COMMENT_STATUS_PENDING);
        assert_eq!(initial_status(None), COMMENT_STATUS_PENDING);
        assert_eq!(initial_status(Some("")), COMMENT_STATUS_PENDING);
    }

    // ————————————————————— 类型过滤 —————————————————————

    #[test]
    fn rank_filter_maps_to_repository_enum() {
        assert_eq!(
            rank_filter_of(Some(CommentRankFilter::Good)),
            Some(RankFilter::Good)
        );
        assert_eq!(
            rank_filter_of(Some(CommentRankFilter::Neutral)),
            Some(RankFilter::Neutral)
        );
        assert_eq!(
            rank_filter_of(Some(CommentRankFilter::Bad)),
            Some(RankFilter::Bad)
        );
        assert_eq!(rank_filter_of(None), None);
    }

    // ————————————————————— 时间解析（Java DateUtil.parse） —————————————————————

    #[test]
    fn parse_datetime_supports_java_common_formats() {
        // 往返校验（与运行机器时区无关）：解析后再格式化应还原原串
        for s in [
            "2023-11-15 06:13:20",
            "2023-11-15 06:13",
            "2023-11-15",
        ] {
            let secs = parse_datetime_seconds(s).unwrap();
            let back = fmt_time_seconds(secs);
            if s.len() == 10 {
                assert!(back.starts_with(s), "{s} → {back}");
            } else if s.len() == 16 {
                assert!(back.starts_with(s), "{s} → {back}");
            } else {
                assert_eq!(back, s);
            }
        }
    }

    #[test]
    fn parse_add_time_falls_back_to_now_when_blank() {
        let before = utils::now_seconds() as i32;
        let secs = parse_add_time_seconds(None).unwrap();
        let secs_blank = parse_add_time_seconds(Some("   ")).unwrap();
        let after = utils::now_seconds() as i32;
        assert!((before..=after).contains(&secs));
        assert!((before..=after).contains(&secs_blank));
        // 非法格式必须报错（Java DateUtil.parse 抛异常）
        assert!(parse_add_time_seconds(Some("not-a-date")).is_err());
    }

    // ————————————————————— 匿名脱敏（listForClient） —————————————————————

    #[test]
    fn mask_anonymous_comment_masks_parent_and_replies() {
        let mut vo = CommentVO {
            username: Some("张三丰".into()),
            is_anonymous: Some(1),
            replies: Some(vec![
                CommentReplyVO {
                    username: Some("李四".into()),
                    is_anonymous: Some(1),
                    ..Default::default()
                },
                CommentReplyVO {
                    username: Some("客服小王".into()),
                    is_anonymous: Some(0),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        mask_anonymous_comment(&mut vo);
        assert_eq!(vo.username.as_deref(), Some("张**丰"));
        let replies = vo.replies.unwrap();
        // Java `AnonymousMaskUtils.maskName`：2 个码点 -> 首码点 + "**"（没有尾码点）
        assert_eq!(replies[0].username.as_deref(), Some("李**"));
        // 商家回复 isAnonymous 恒 0，不脱敏
        assert_eq!(replies[1].username.as_deref(), Some("客服小王"));
    }

    #[test]
    fn non_anonymous_comment_is_untouched() {
        let mut vo = CommentVO {
            username: Some("张三".into()),
            is_anonymous: Some(0),
            ..Default::default()
        };
        mask_anonymous_comment(&mut vo);
        assert_eq!(vo.username.as_deref(), Some("张三"));
    }
}
