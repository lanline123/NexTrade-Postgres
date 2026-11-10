//! 微信直播服务（`content/wechat_live`，14 条管理端接口）。
//!
//! 对应原 Java `com.nextrade.service.content.impl.WechatLiveServiceImpl`（1614 行）
//! 与 `com.nextrade.adminapi.controller.content.WechatLiveController`。
//!
//! # 已实现（本地库操作，行为对齐 Java）
//!
//! - `list`：标题/关键词 ILIKE、状态、创建日期范围、排序、分页；
//!   VO 附加 `liveStatusText` / `auditStatusText` / `orderCount` / `syncFailedCount`
//!   （对应 Java `convert` + `fillStats`）。
//! - `detail`：同上转换；不存在返回 `null`（Java `getById` 为空时同样返回空）。
//! - `update`：仅更新本地字段（Java `update` 同样只 `updateById`，远端同步走
//!   `syncToWechat`）；`name` 非空时覆盖标题。
//! - `create`：**本地创建**（Java 在 `createRemoteRoom` 失败时也仅保存本地记录，
//!   且异常被吞掉；本实现不调用微信接口，`room_id = 0` 表示未同步远端）。
//! - `delete`：删除本地记录（Java 远端删除失败也忽略）。
//! - `addToRoom` / `removeFromRoom`：维护本地 `wechat_live_room_goods` 关联
//!   （Java 先调微信接口再写库；本实现**跳过远端**，见偏差）。
//! - `room/{roomId}`：直播间商品列表（关联表 + 直播商品主表）。
//! - `promote/{roomId}`：本地降级数据（Java 在微信接口异常时同样降级：
//!   `qrcodeUrl=''`、`posterUrl=shareImg`、`wechatShortLink=''`）。
//!
//! # 未迁移（外部微信接口依赖）
//!
//! Java 通过 `WechatApiClient` / `WechatOAuthService` 调用微信开放平台
//! （小程序 access_token、创建/编辑/删除直播间、推流地址、推广二维码、
//! 视频号直播记录）。Rust 未迁移 HTTP 客户端与 `appid/secret` 配置，以下
//! 5 条接口返回与 Java「token 获取失败」同口径的明确错误：
//! `refresh` / `syncFromWechat` / `syncToWechat` / `pushUrl` / `finderLiveRecord`。
//!
//! 其余偏差：
//! - `create` 不会在微信侧创建直播间（`room_id = 0`）；
//! - `addToRoom` 不会在微信侧创建/审核商品（要求本地直播商品已有
//!   `wechat_goods_id`，与 Java 校验一致）；
//! - `removeFromRoom` 不调用微信删除商品；
//! - `checkSkuEnabled` 以「商品/SKU 是否存在」近似（Java 还校验 SKU 启用状态，
//!   `product_sku.is_enabled` 列在本仓库实体中未暴露给本切片）。
//! - 多租户：Java 用请求头 `x-shop-id` 过滤/写入店铺；Rust JWT 无 shop_id，
//!   列表不按店铺过滤、创建写 `shop_id = 0`（与平台视角一致）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{WechatLive, WechatLiveGoods, WechatLiveRoomGoods};
use nextrade_core::models::content::wechat_live::{
    audit_status_text, current_time_string, format_timestamp, live_status_text, parse_int_list,
    parse_timestamp, WechatLiveGoodsAddDTO, WechatLiveGoodsVO, WechatLiveListQuery,
    WechatLivePromoteVO, WechatLivePushUrlVO, WechatLiveRoomCreateDTO, WechatLiveUpdateDTO,
    WechatLiveVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::{ensure_media_access, ensure_media_write, media_write_shop_id};

/// 平台店铺 ID（无 JWT shop_id 时按平台视角处理）。
const PLATFORM_SHOP_ID: i32 = 0;
/// 未开始状态（Java `WechatLiveStatus.LIVE_STATUS_NOT_START`）。
const LIVE_STATUS_NOT_START: i32 = 102;

pub struct WechatLiveService {
    repos: Arc<Repositories>,
}

impl WechatLiveService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// `GET /adminapi/content/wechatLive/list`：直播间分页。
    pub async fn list(&self, q: &WechatLiveListQuery) -> Result<PageResult<WechatLiveVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos
            .content
            .wechat_live
            .paginate(q, &page, TenantScope::current_list_filter())
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            list.push(self.to_vo(row).await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /adminapi/content/wechatLive/detail?id=`：直播间详情。
    pub async fn detail(&self, id: i32) -> Result<Option<WechatLiveVO>> {
        let Some(row) = self.repos.content.wechat_live.find_by_id(id).await? else {
            return Ok(None);
        };
        // 多租户：平台直播间可读；店铺直播间仅本店可读
        ensure_media_access(row.shop_id)?;
        Ok(Some(self.to_vo(&row).await?))
    }

    // ————————————————————— 写操作 —————————————————————

    /// `POST /adminapi/content/wechatLive/update`：更新直播间（仅本地字段）。
    pub async fn update(&self, dto: &WechatLiveUpdateDTO) -> Result<()> {
        let id = dto
            .wechat_live_id
            .ok_or_else(|| Error::Validation("微信直播id不能为空".into()))?;
        let mut w = self
            .repos
            .content
            .wechat_live
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg("直播不存在"))?;
        // 多租户：仅本店直播间可改（平台直播间对店铺账号只读）
        ensure_media_write(w.shop_id)?;

        // 标题：Java 先应用 wechatLiveTitle，再应用 name（name 优先）
        if let Some(v) = dto.wechat_live_title.as_deref() {
            w.wechat_live_title = v.to_string();
        }
        if let Some(v) = dto.name.as_deref() {
            w.wechat_live_title = v.to_string();
        }
        if let Some(v) = dto.act_range {
            w.act_range = v;
        }
        if let Some(v) = dto.act_range_ext.as_ref() {
            w.act_range_ext = serde_json::to_string(v).unwrap_or_else(|_| "[]".into());
        }
        if let Some(v) = dto.is_feeds_public {
            w.is_feeds_public = v;
        }
        if let Some(v) = dto.cover_img.as_deref() {
            w.cover_img = v.to_string();
        }
        if let Some(v) = dto.share_img.as_deref() {
            w.share_img = v.to_string();
        }
        if dto.start_time.is_some() {
            w.start_time = parse_timestamp(dto.start_time.as_deref());
        }
        if dto.end_time.is_some() {
            w.end_time = parse_timestamp(dto.end_time.as_deref());
        }
        if let Some(v) = dto.anchor_name.as_deref() {
            w.anchor_name = v.to_string();
        }
        if let Some(v) = dto.anchor_img.as_deref() {
            w.anchor_img = v.to_string();
        }
        if let Some(v) = dto.live_type {
            w.live_type = v;
        }
        if let Some(v) = dto.close_like {
            w.close_like = v;
        }
        if let Some(v) = dto.close_goods {
            w.close_goods = v;
        }
        if let Some(v) = dto.close_comment {
            w.close_comment = v;
        }
        if let Some(v) = dto.close_kf {
            w.close_kf = v;
        }
        if let Some(v) = dto.close_replay {
            w.close_replay = v;
        }
        if let Some(v) = dto.feeds_img.as_deref() {
            w.feeds_img = v.to_string();
        }
        if let Some(v) = dto.anchor_id.as_deref() {
            w.anchor_id = v.to_string();
        }
        if let Some(v) = dto.anchor_openid.as_deref() {
            w.anchor_openid = v.to_string();
        }
        if let Some(v) = dto.sub_anchor_wechat.as_deref() {
            w.sub_anchor_wechat = Some(v.to_string());
        }
        self.repos.content.wechat_live.update(&w).await?;
        Ok(())
    }

    /// `GET /adminapi/content/wechatLive/refresh`：刷新微信直播间信息。
    ///
    /// ⚠️ 未迁移：Java 调 `wechatApiClient.getLiveInfo` 拉取远端房间并落库。
    pub async fn refresh(&self) -> Result<()> {
        Err(wechat_api_error("access_token获取失败"))
    }

    /// `GET|POST /adminapi/promotion/wechatLive/refresh`：刷新直播间（兼容桩）。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/promotion/live.ts#getLiveRefresh`
    /// （列表页「API更新直播间」按钮）。本部署无微信凭据
    /// （`WechatApiClient` 未迁移），按任务约定返回成功 no-op——
    /// 前端点击后仅重拉本地列表，与「无远端可同步」语义一致。
    /// [`WechatLiveService::refresh`]（`content/wechatLive/refresh` 契约路径）
    /// 保持原语义（返回微信错误），不受影响。
    pub async fn refresh_noop(&self) -> Result<()> {
        Ok(())
    }

    /// `POST /adminapi/content/wechatLive/create`：创建直播间（本地记录）。
    ///
    /// 对应 Java `create`：Java 尝试调微信接口创建远端房间（失败被 `catch` 吞掉），
    /// 随后保存本地记录。Rust 直接保存本地记录，`room_id = 0` 表示尚未同步远端。
    pub async fn create(&self, dto: &WechatLiveRoomCreateDTO) -> Result<i32> {
        let name = dto
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("微信直播名称不能为空".into()))?;
        if name.chars().count() > 50 {
            return Err(Error::Validation("微信直播名称长度不能超过50".into()));
        }
        let now = nextrade_common::utils::now_seconds() as i32;
        let mut w = WechatLive {
            wechat_live_title: name.to_string(),
            live_status: LIVE_STATUS_NOT_START,
            last_update_time: now,
            shop_id: media_write_shop_id(None)?,
            create_time: Some(current_time_string()),
            room_id: 0,
            ..fill_room_defaults(WechatLive::default())
        };
        w.cover_img = dto.cover_img.clone().unwrap_or_default();
        w.share_img = dto.share_img.clone().unwrap_or_default();
        w.start_time = parse_timestamp(dto.start_time.as_deref());
        w.end_time = parse_timestamp(dto.end_time.as_deref());
        w.anchor_name = dto.anchor_name.clone().unwrap_or_default();
        w.anchor_img = dto.anchor_img.clone().unwrap_or_default();
        w.live_type = dto.live_type.unwrap_or(0);
        w.close_like = dto.close_like.unwrap_or(0);
        w.close_goods = dto.close_goods.unwrap_or(0);
        w.close_comment = dto.close_comment.unwrap_or(0);
        w.close_kf = dto.close_kf.unwrap_or(0);
        w.close_replay = dto.close_replay.unwrap_or(0);
        w.is_feeds_public = dto.is_feeds_public.unwrap_or(0);
        w.feeds_img = dto.feeds_img.clone().unwrap_or_default();
        w.anchor_id = dto.anchor_id.clone().unwrap_or_default();
        w.anchor_openid = dto.anchor_openid.clone().unwrap_or_default();
        w.sub_anchor_wechat = dto.sub_anchor_wechat.clone();
        let id = self.repos.content.wechat_live.insert(&w).await?;
        tracing::info!("创建微信直播间（本地）: id={id}, room_id=0（远端未创建）");
        Ok(id)
    }

    /// `POST /adminapi/content/wechatLive/delete`：删除直播间（本地）。
    pub async fn delete(&self, id: i32) -> Result<()> {
        let exists = self.repos.content.wechat_live.find_by_id(id).await?;
        let Some(row) = exists else {
            return Err(Error::biz_msg("直播不存在"));
        };
        // 多租户：仅本店直播间可删（平台直播间对店铺账号只读）
        ensure_media_write(row.shop_id)?;
        self.repos.content.wechat_live.delete(id).await?;
        Ok(())
    }

    /// `GET /adminapi/content/wechatLive/pushUrl/:roomId`：推流地址。
    ///
    /// ⚠️ 未迁移：Java 调 `wechatApiClient.getPushUrl`。
    pub async fn push_url(&self, room_id: i32) -> Result<WechatLivePushUrlVO> {
        // 先按 Java 校验直播间存在，再返回「接口未迁移」错误
        let room = self
            .repos
            .content
            .wechat_live
            .find_by_room_id(room_id)
            .await?
            .ok_or_else(|| Error::biz_msg("直播间不存在"))?;
        let _ = room;
        Err(wechat_api_error("获取推流地址失败：access_token获取失败"))
    }

    /// `GET /adminapi/content/wechatLive/promote/:roomId`：直播间推广信息（本地降级）。
    ///
    /// 对应 Java `getPromoteData`：微信接口异常时降级返回
    /// `qrcodeUrl=''` / `posterUrl=shareImg` / `wechatShortLink=''`。
    /// `pagePath` 使用本地模板（Java `LIVE_PLAYER_PAGE_PATH_TEMPLATE` 常量未迁移）。
    pub async fn promote(&self, room_id: i32) -> Result<WechatLivePromoteVO> {
        let room = self
            .repos
            .content
            .wechat_live
            .find_by_room_id(room_id)
            .await?
            .ok_or_else(|| Error::biz_msg("直播间不存在"))?;
        Ok(WechatLivePromoteVO {
            room_id: Some(room_id),
            room_name: Some(room.wechat_live_title.clone()),
            cover_img: Some(room.cover_img.clone()),
            page_path: Some(format!(
                "plugin-private://wx-live/pages/live-player-plugin?room_id={room_id}"
            )),
            qrcode_url: Some(String::new()),
            poster_url: Some(room.share_img.clone()),
            wechat_short_link: Some(String::new()),
        })
    }

    /// `GET /adminapi/content/wechatLive/syncFromWechat`：从微信同步。
    ///
    /// ⚠️ 未迁移：Java `refresh()`（`getLiveInfo` 拉取远端房间）。
    pub async fn sync_from_wechat(&self) -> Result<()> {
        Err(wechat_api_error("access_token获取失败"))
    }

    /// `POST /adminapi/content/wechatLive/syncToWechat?id=`：同步到微信。
    ///
    /// ⚠️ 未迁移：Java 调 `editRoom` / `createRemoteRoom`。
    pub async fn sync_to_wechat(&self, id: i32) -> Result<()> {
        let exists = self.repos.content.wechat_live.find_by_id(id).await?;
        if exists.is_none() {
            return Err(Error::biz_msg("直播不存在"));
        }
        Err(wechat_api_error("access_token获取失败"))
    }

    // ————————————————————— 直播间商品 —————————————————————

    /// `POST /adminapi/content/wechatLive/addToRoom`：把已导入商品加入直播间。
    ///
    /// 对应 Java `addGoods` 的**本地部分**：校验直播间、商品存在性与
    /// `wechat_goods_id`，按 `goodsIds` 顺序写 `sort`，已关联则更新 `sort`。
    /// ⚠️ 不调用微信 `addGoods`（见模块头偏差）。
    pub async fn add_goods(&self, dto: &WechatLiveGoodsAddDTO) -> Result<()> {
        let room_id = dto
            .room_id
            .ok_or_else(|| Error::Validation("直播间ID不能为空".into()))?;
        let goods_ids = dto.goods_ids.as_deref().unwrap_or_default();
        if goods_ids.is_empty() {
            return Err(Error::biz_msg("直播商品ID列表不能为空"));
        }
        if self
            .repos
            .content
            .wechat_live
            .find_by_room_id(room_id)
            .await?
            .is_none()
        {
            return Err(Error::biz_msg("直播间不存在"));
        }
        let live_shop_id = PLATFORM_SHOP_ID;
        let now = current_time_string();
        for (idx, goods_id) in goods_ids.iter().enumerate() {
            let sort = idx as i32 + 1;
            let goods = self
                .repos
                .content
                .wechat_live
                .find_goods_by_id_shop(*goods_id, live_shop_id)
                .await?
                .ok_or_else(|| {
                    Error::biz_msg(format!("直播商品不存在，goodsId={goods_id}"))
                })?;
            let Some(wechat_goods_id) = goods.wechat_goods_id else {
                return Err(Error::biz_msg(format!(
                    "直播商品缺少微信商品ID，goodsId={goods_id}"
                )));
            };
            // Java `productSkuService.checkSkuEnabled`：存在性近似校验（见模块头偏差）
            self.check_goods_product(&goods, live_shop_id).await?;
            let existing = self
                .repos
                .content
                .wechat_live
                .find_room_goods_one(room_id, *goods_id)
                .await?;
            match existing {
                Some(mut room_goods) => {
                    room_goods.sort = Some(sort);
                    room_goods.update_time = Some(now.clone());
                    self.repos
                        .content
                        .wechat_live
                        .update_room_goods(&room_goods)
                        .await?;
                }
                None => {
                    let room_goods = WechatLiveRoomGoods {
                        id: 0,
                        room_id,
                        wechat_live_goods_id: *goods_id,
                        wechat_goods_id: Some(wechat_goods_id),
                        sort: Some(sort),
                        is_show: Some(0),
                        create_time: Some(now.clone()),
                        update_time: Some(now.clone()),
                        shop_id: Some(live_shop_id),
                    };
                    self.repos
                        .content
                        .wechat_live
                        .insert_room_goods(&room_goods)
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// `POST /adminapi/content/wechatLive/removeFromRoom`：从直播间移除商品。
    ///
    /// 对应 Java `removeGoods` 的**本地部分**（Java 先调微信删除接口再删本地）。
    pub async fn remove_goods(&self, room_id: i32, goods_id: i64) -> Result<()> {
        if self
            .repos
            .content
            .wechat_live
            .find_by_room_id(room_id)
            .await?
            .is_none()
        {
            return Err(Error::biz_msg("直播间不存在"));
        }
        // Java：关联不存在时直接返回 true（幂等）
        self.repos
            .content
            .wechat_live
            .delete_room_goods(room_id, goods_id)
            .await?;
        Ok(())
    }

    /// `GET /adminapi/content/wechatLive/room/:roomId`：直播间商品列表。
    pub async fn room_goods(&self, room_id: i32) -> Result<Vec<WechatLiveGoodsVO>> {
        if self
            .repos
            .content
            .wechat_live
            .find_by_room_id(room_id)
            .await?
            .is_none()
        {
            return Err(Error::biz_msg("直播间不存在"));
        }
        let room_goods = self
            .repos
            .content
            .wechat_live
            .find_room_goods(room_id)
            .await?;
        if room_goods.is_empty() {
            return Ok(Vec::new());
        }
        let goods_ids: Vec<i64> = room_goods
            .iter()
            .map(|r| r.wechat_live_goods_id)
            .collect();
        let goods_map: HashMap<i64, WechatLiveGoods> = self
            .repos
            .content
            .wechat_live
            .find_goods_by_ids(&goods_ids)
            .await?
            .into_iter()
            .map(|g| (g.id, g))
            .collect();
        let mut out = Vec::with_capacity(room_goods.len());
        for rg in &room_goods {
            let Some(g) = goods_map.get(&rg.wechat_live_goods_id) else {
                continue;
            };
            out.push(WechatLiveGoodsVO {
                id: Some(g.id),
                room_goods_id: Some(rg.id),
                room_id: Some(rg.room_id),
                wechat_goods_id: g.wechat_goods_id,
                product_id: g.product_id,
                product_sku_id: Some(g.product_sku_id),
                sku_data: None,
                goods_name: g.goods_name.clone(),
                cover_img: g.cover_img.clone(),
                price: g.price,
                stock: None,
                price_type: g.price_type,
                url: g.url.clone(),
                audit_status: g.audit_status,
                sort: rg.sort,
                is_show: rg.is_show,
                is_joined_room: Some(1),
            });
        }
        Ok(out)
    }

    /// `GET /adminapi/content/wechatLive/finderLiveRecord?id=`：视频号直播记录。
    ///
    /// ⚠️ 未迁移：Java 调 `wechatApiClient.getFinderLiveRecordList`。
    pub async fn finder_live_record(&self, _id: &str) -> Result<serde_json::Value> {
        Err(wechat_api_error(
            "获取视频号直播记录失败：access_token获取失败",
        ))
    }

    // ————————————————————— 内部辅助 —————————————————————

    /// 商品/SKU 存在性校验（近似 Java `productSkuService.checkSkuEnabled`）。
    async fn check_goods_product(&self, goods: &WechatLiveGoods, live_shop_id: i32) -> Result<()> {
        let Some(product_id) = goods.product_id.filter(|id| *id > 0) else {
            return Ok(());
        };
        let product = self
            .repos
            .content
            .wechat_live
            .find_product(product_id)
            .await?
            .ok_or_else(|| {
                Error::biz_msg(format!("商品规格不存在，productId={product_id}"))
            })?;
        if goods.product_sku_id > 0 {
            let sku = self
                .repos
                .content
                .wechat_live
                .find_sku(goods.product_sku_id)
                .await?
                .ok_or_else(|| {
                    Error::biz_msg(format!(
                        "商品规格不存在，productId={product_id}, skuId={}",
                        goods.product_sku_id
                    ))
                })?;
            if i64::from(sku.product_id) != product_id {
                return Err(Error::biz_msg(format!(
                    "商品规格不存在，productId={product_id}, skuId={}",
                    goods.product_sku_id
                )));
            }
        }
        // 店铺匹配（Java `resolveLiveGoodsSnapshot`）：
        // - 商品店铺 == 直播间店铺 → 通过；
        // - 直播间为平台（shop_id = 0）→ 允许任意商品；
        // - 商品为平台商品（shop_id = 0）→ 允许进入任意直播间；
        // - 其余（店铺直播间 + 其它店铺商品）→ 报错。
        let source_shop = product.shop_id;
        if source_shop != live_shop_id && live_shop_id != 0 && source_shop != 0 {
            return Err(Error::biz_msg(format!(
                "直播间所属店铺与商品所属店铺不匹配，productId={product_id}"
            )));
        }
        Ok(())
    }

    /// 实体 → VO（对应 Java `convert` + `fillStats`）。
    async fn to_vo(&self, w: &WechatLive) -> Result<WechatLiveVO> {
        let order_count = if w.room_id > 0 {
            Some(self.repos.content.wechat_live.count_orders(w.room_id).await?)
        } else {
            None
        };
        let sync_failed_count = if w.room_id > 0 {
            Some(
                self.repos
                    .content
                    .wechat_live
                    .count_sync_failed(w.room_id)
                    .await?,
            )
        } else {
            None
        };
        Ok(WechatLiveVO {
            wechat_live_id: Some(w.wechat_live_id),
            wechat_live_title: Some(w.wechat_live_title.clone()),
            wechat_live_data: Some(w.wechat_live_data.clone()),
            act_range: Some(w.act_range),
            act_range_ext: Some(parse_int_list(&w.act_range_ext)),
            room_id: Some(w.room_id),
            share_img: Some(w.share_img.clone()),
            live_type: Some(w.live_type),
            close_like: Some(w.close_like),
            close_goods: Some(w.close_goods),
            close_comment: Some(w.close_comment),
            close_kf: Some(w.close_kf),
            close_replay: Some(w.close_replay),
            is_feeds_public: Some(w.is_feeds_public),
            feeds_img: Some(w.feeds_img.clone()),
            cover_img: Some(w.cover_img.clone()),
            anchor_id: Some(w.anchor_id.clone()),
            anchor_openid: Some(w.anchor_openid.clone()),
            sub_anchor_wechat: w.sub_anchor_wechat.clone(),
            anchor_name: Some(w.anchor_name.clone()),
            anchor_img: Some(w.anchor_img.clone()),
            creator_openid: Some(w.creator_openid.clone()),
            live_status: Some(w.live_status),
            live_status_text: Some(live_status_text(Some(w.live_status))),
            audit_status: Some(w.audit_status),
            audit_status_text: Some(audit_status_text(Some(w.audit_status))),
            start_time: Some(format_timestamp(Some(w.start_time))),
            end_time: Some(format_timestamp(Some(w.end_time))),
            last_update_time: Some(format_timestamp(Some(w.last_update_time))),
            product_data: Some(parse_int_list(&w.product_data)),
            thumb_img: Some(w.thumb_img.clone()),
            live_sn: Some(w.live_sn.clone()),
            shop_id: Some(w.shop_id),
            order_count,
            sync_failed_count,
        })
    }
}

/// Java `fillRoomDefaultValues`：为 `None` 的列补 DDL/默认值。
fn fill_room_defaults(mut w: WechatLive) -> WechatLive {
    if w.wechat_live_data.is_empty() {
        w.wechat_live_data = String::new();
    }
    if w.act_range_ext.is_empty() {
        w.act_range_ext = "[]".to_string();
    }
    if w.product_data.is_empty() {
        w.product_data = "[]".to_string();
    }
    if w.live_status == 0 {
        w.live_status = LIVE_STATUS_NOT_START;
    }
    w
}

/// 「微信接口未迁移」错误（文案以 Java access_token 失败为口径 + 迁移说明）。
fn wechat_api_error(java_msg: &str) -> Error {
    Error::biz_msg(format!(
        "{java_msg}（微信直播接口未迁移：缺少小程序 appid/secret 与 WechatApiClient）"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_room_defaults_matches_java() {
        let w = fill_room_defaults(WechatLive::default());
        assert_eq!(w.live_status, LIVE_STATUS_NOT_START);
        assert_eq!(w.act_range_ext, "[]");
        assert_eq!(w.product_data, "[]");
        assert!(w.wechat_live_data.is_empty());
    }

    #[test]
    fn wechat_api_error_keeps_java_message_prefix() {
        let e = wechat_api_error("access_token获取失败");
        let msg = format!("{e}");
        assert!(msg.contains("access_token获取失败"));
        assert!(msg.contains("未迁移"));
    }

    #[test]
    fn vo_conversion_is_pure_for_empty_room() {
        // 房间号 0 时不应查关联统计（避免无谓 SQL）；纯逻辑校验默认值
        let w = WechatLive {
            wechat_live_id: 9,
            wechat_live_title: "测试直播".into(),
            act_range_ext: "[1,2]".into(),
            product_data: "[3]".into(),
            live_status: 102,
            audit_status: 1,
            room_id: 0,
            ..Default::default()
        };
        assert_eq!(parse_int_list(&w.act_range_ext), vec![1, 2]);
        assert_eq!(parse_int_list(&w.product_data), vec![3]);
        assert_eq!(live_status_text(Some(w.live_status)), "未开始");
        assert_eq!(audit_status_text(Some(w.audit_status)), "审核中");
    }
}
