//! 拍卖服务（一期）：场次/拍品管理、出价与保证金、结拍与违约处理。
//!
//! Rust 版原创域，设计见 `~/workspace/auction-design/proposal.md`。
//! 策略数字一律取自「拍卖设置」（`auction_settings` 模块，config 表
//! `auctionSettings` 行），本服务不写死：
//! - 出价：拍品行锁串行化 → 阶梯加价校验 → 延时顺延 →（一口价立即结拍）
//! - 保证金：余额冻结（一品一冻，`(item_id, user_id)` 唯一约束幂等），
//!   出局/流拍/撤拍解冻，中标抵扣货款，违约按设置扣罚
//! - 结拍：定时任务扫描推进（`scheduler` 的 `auction_loop` 调用
//!   `run_start_scan` / `run_close_scan` / `run_overdue_scan`），幂等
//! - 成交订单：`order_type = 11`，保证金抵扣记 `paid_amount`，
//!   `order_extension` JSON 记 `auctionItemId` / `depositAmount` / `payDeadline`

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use serde_json::{json, Value};

use nextrade_common::utils;
use nextrade_core::entities::{
    AuctionBan, AuctionBid, AuctionBreach, AuctionDeposit, AuctionItem, AuctionSession, Order,
    OrderItem, Product, ProductGallery, User, UserAddress,
};
use nextrade_core::models::marketing::auction::{
    bid_status, deposit_status, fmt_time, item_status, mask_name, AuctionAdminBidVO,
    AuctionBanSaveDTO, AuctionBanVO, AuctionBidDTO, AuctionBidListQuery, AuctionBidVO,
    AuctionBreachListQuery, AuctionBreachVO, AuctionDepositListQuery, AuctionDepositVO,
    AuctionItemAdminDetailVO, AuctionItemDetailVO, AuctionItemListQuery, AuctionItemSaveDTO,
    AuctionItemVO, AuctionMyJoinVO, AuctionPollVO, AuctionPublicListQuery, AuctionSessionSaveDTO,
    AuctionSessionVO, AuctionStatsVO, AUCTION_ORDER_TYPE,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::auction::WalletLogParams;
use nextrade_infrastructure::db::Repositories;

use super::auction_settings::{
    effective_settings, sanitize_settings, settings_to_json, AuctionSettings, BanAction,
    AUCTION_SETTINGS_CODE,
};

/// 出价结果（出价接口返回，含最新状态供前端立即刷新）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidResultVO {
    pub item_id: i32,
    pub current_price: Decimal,
    pub is_leader: bool,
    pub is_buyout: bool,
    pub status: i32,
    pub real_end_time: i32,
    pub remain_seconds: i64,
    pub order_id: Option<i32>,
}

/// 待发送的站内信（事务提交后统一发送）。
struct Notice {
    user_id: i32,
    title: String,
    content: String,
}

pub struct AuctionService {
    repos: Arc<Repositories>,
}

impl AuctionService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::marketing::auction::AuctionRepository {
        &self.repos.marketing.auction
    }

    fn now() -> i32 {
        utils::now_seconds() as i32
    }

    // ————————————————————— 设置 —————————————————————

    /// 读取生效设置（未保存过时为出厂推荐值）。
    pub async fn get_settings(&self) -> Result<AuctionSettings> {
        let raw = self
            .repos
            .system
            .config_basic
            .find_by_code(AUCTION_SETTINGS_CODE)
            .await?;
        Ok(effective_settings(raw.as_deref()))
    }

    /// 设置 JSON（管理端设置页读写共用形状）。
    pub async fn get_settings_json(&self) -> Result<Value> {
        Ok(settings_to_json(&self.get_settings().await?))
    }

    /// 保存设置（清洗后整块 JSON 落 config 表），返回生效值。
    pub async fn save_settings(&self, payload: &Value) -> Result<Value> {
        let clean = sanitize_settings(payload);
        self.repos
            .system
            .config_basic
            .save_code(AUCTION_SETTINGS_CODE, &settings_to_json(&clean).to_string())
            .await?;
        Ok(settings_to_json(&clean))
    }

    // ————————————————————— 内部工具 —————————————————————

    async fn load_user(&self, user_id: i32) -> Result<User> {
        self.repos
            .member
            .user
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| Error::NotFound("用户不存在".into()))
    }

    /// 参拍资格前置校验（出价/冻结共用）：账号启用 + 认证门槛 + 未封禁。
    async fn ensure_can_bid(
        &self,
        user_id: i32,
        settings: &AuctionSettings,
        now: i32,
    ) -> Result<User> {
        let user = self.load_user(user_id).await?;
        if user.status.unwrap_or(1) != 1 {
            return Err(Error::Forbidden("账号已被禁用，无法参拍".into()));
        }
        if settings.require_certification && user.is_company_auth != 1 {
            return Err(Error::biz(4003, "参拍需先完成企业认证"));
        }
        if let Some(ban) = self.repo().find_active_ban(user_id, now).await? {
            let msg = if ban.ban_type == 2 {
                "您的参拍资格已被永久取消".to_string()
            } else {
                format!("您的参拍资格已封禁至 {}", fmt_time(ban.end_time))
            };
            return Err(Error::Forbidden(msg));
        }
        Ok(user)
    }

    /// 防自拍自买：拍品创建人不得对该拍品出价。
    /// （店铺归属账号的强校验待二期商户自助上拍时在商户入口补齐：
    /// 一期上拍均由平台管理员操作，`shop` 表与会员账号无直接绑定列。）
    fn ensure_not_self_deal(&self, user_id: i32, item: &AuctionItem) -> Result<()> {
        if item.create_admin_id > 0 && item.create_admin_id == user_id {
            return Err(Error::Forbidden("不能对自己上架的拍品出价".into()));
        }
        Ok(())
    }

    /// 事务内钱包动作 + 流水，返回流水 id。
    /// `balance_delta` / `frozen_delta` 为带符号变动额；流水的金额列记绝对值。
    async fn wallet_move_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: i32,
        balance_delta: Decimal,
        frozen_delta: Decimal,
        change_type: i32,
        desc: &str,
        now: i32,
    ) -> Result<i32> {
        let repo = self.repo();
        let (bal, frz) = repo
            .wallet_snapshot_in_tx(tx, user_id)
            .await?
            .ok_or_else(|| Error::NotFound("用户不存在".into()))?;
        let ok = repo
            .apply_wallet_in_tx(tx, user_id, balance_delta, frozen_delta)
            .await?;
        if !ok {
            return Err(Error::biz(400, "余额不足"));
        }
        repo.insert_balance_log_in_tx(
            tx,
            &WalletLogParams {
                user_id,
                before_balance: bal,
                balance_delta: balance_delta.abs(),
                after_balance: bal + balance_delta,
                frozen_delta: frozen_delta.abs(),
                before_frozen_balance: frz,
                after_frozen_balance: frz + frozen_delta,
                change_time: now,
                change_desc: desc.to_string(),
                change_type,
            },
        )
        .await
    }

    /// 解冻一笔冻结中的保证金（事务内）：冻结额 → 可用余额 + 流水 + 状态置已解冻。
    async fn release_deposit_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        deposit: &AuctionDeposit,
        item_title: &str,
        now: i32,
        desc_prefix: &str,
    ) -> Result<()> {
        if deposit.status != deposit_status::FROZEN {
            return Ok(());
        }
        self.wallet_move_in_tx(
            tx,
            deposit.user_id,
            deposit.amount,
            -deposit.amount,
            1,
            &format!("{desc_prefix}[拍品：{item_title}]"),
            now,
        )
        .await?;
        let mut d = deposit.clone();
        d.status = deposit_status::RELEASED;
        d.handle_time = now;
        self.repo().update_deposit_in_tx(tx, &d).await
    }

    /// 发送站内信（best-effort：失败只记日志，不回滚业务）。
    async fn send_notices(&self, notices: Vec<Notice>) {
        for n in notices {
            if n.user_id <= 0 {
                continue;
            }
            if let Err(e) = self
                .repos
                .member
                .user_message
                .insert_user_message(n.user_id, &n.title, &n.content, 1, None)
                .await
            {
                tracing::warn!("拍卖站内信发送失败 user={}: {e}", n.user_id);
            }
        }
    }

    /// 管理操作日志（best-effort，与秒杀同模式）。
    async fn admin_log(&self, shop_id: i32, info: &str) {
        let svc = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        if let Err(e) = svc
            .record_current_log(if shop_id > 0 { "shop" } else { "admin" }, shop_id, info)
            .await
        {
            tracing::warn!("写管理操作日志失败（拍卖）: {e}");
        }
    }

    // ————————————————————— 管理端：场次 —————————————————————

    pub async fn session_list(&self) -> Result<Vec<AuctionSessionVO>> {
        let rows = self.repo().list_sessions(false).await?;
        Ok(rows.into_iter().map(|(s, cnt)| session_vo(&s, cnt)).collect())
    }

    pub async fn session_save(&self, dto: &AuctionSessionSaveDTO) -> Result<i32> {
        let name = dto
            .session_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("场次名称不能为空".into()))?;
        let start = dto.start_time.unwrap_or(0);
        let end = dto.end_time.unwrap_or(0);
        if start > 0 && end > 0 && start >= end {
            return Err(Error::Validation("场次开始时间必须早于结束时间".into()));
        }
        let row = AuctionSession {
            session_id: dto.session_id.unwrap_or(0),
            session_name: name.to_string(),
            cover_pic: dto.cover_pic.clone().unwrap_or_default(),
            start_time: start,
            end_time: end,
            sort: dto.sort.unwrap_or(0),
            status: dto.status.unwrap_or(1),
            add_time: Self::now(),
        };
        if let Some(id) = dto.session_id.filter(|id| *id > 0) {
            self.repo().update_session(&row).await?;
            Ok(id)
        } else {
            self.repo().insert_session(&row).await
        }
    }

    pub async fn session_delete(&self, session_id: i32) -> Result<()> {
        let (_, items) = self
            .repo()
            .paginate_items_admin(None, Some(session_id), None, None, &PageQuery { page: 1, page_size: 1 })
            .await?;
        if !items.is_empty() {
            return Err(Error::biz(400, "该场次下已有拍品，不能删除"));
        }
        self.repo().delete_session(session_id).await
    }

    // ————————————————————— 管理端：拍品 —————————————————————

    pub async fn item_list_admin(&self, q: &AuctionItemListQuery) -> Result<PageResult<AuctionItemVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let now = Self::now();
        let (total, rows) = self
            .repo()
            .paginate_items_admin(q.keyword.as_deref(), q.session_id, q.status, scoped_shop(q.shop_id), &page)
            .await?;
        let products = self.load_products(&rows.iter().map(|i| i.product_id).collect::<Vec<_>>()).await?;
        let winners = self.winner_names(&rows).await?;
        let list = rows.iter().map(|i| self.item_vo(i, &products, &winners, now)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 管理端拍品详情（编辑表单回读）：卡片 VO + 关联商品资料
    /// （简介/详情/价格/图集），图集取相册行排序，无相册行时回退主图单张。
    pub async fn item_detail_admin(&self, item_id: i32) -> Result<AuctionItemAdminDetailVO> {
        let item = self
            .repo()
            .find_item(item_id)
            .await?
            .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        let products = self.load_products(&[item.product_id]).await?;
        let winners = self.winner_names(std::slice::from_ref(&item)).await?;
        let vo = self.item_vo(&item, &products, &winners, Self::now());
        let product = products.get(&item.product_id);
        let gallery = self.repos.catalog.product_manage.find_gallery(item.product_id).await?;
        let mut pics: Vec<String> = gallery.into_iter().map(|g| g.pic_url).filter(|u| !u.is_empty()).collect();
        if pics.is_empty() {
            if let Some(p) = product {
                if !p.pic_url.is_empty() {
                    pics.push(p.pic_url.clone());
                }
            }
        }
        Ok(AuctionItemAdminDetailVO {
            item: vo,
            product_brief: product.map(|p| p.product_brief.clone()).unwrap_or_default(),
            product_desc: product.and_then(|p| p.product_desc.clone()),
            product_price: product.map(|p| p.product_price).unwrap_or_default(),
            market_price: product.map(|p| p.market_price).unwrap_or_default(),
            pics,
            own_product: item.own_product == 1,
        })
    }

    /// 同步拍品关联商品相册（整表重写：先清后按序写入，首图即主图）。
    /// 口径与商品保存的 `sync_gallery` 一致：相册行 `shop_id` 恒 0（平台图
    /// 口径，与仓储 find/delete 的过滤一致）、缩略图/大图按 OSS 派生规则生成。
    async fn sync_item_gallery(&self, product_id: i32, pics: &[String]) -> Result<()> {
        let pm = &self.repos.catalog.product_manage;
        pm.delete_gallery(product_id).await?;
        for (idx, url) in pics.iter().enumerate() {
            pm.insert_gallery(&ProductGallery {
                product_id,
                pic_url: url.clone(),
                pic_desc: String::new(),
                pic_thumb: utils::normalize_pic_thumb(url, ""),
                pic_original: url.clone(),
                pic_large: utils::pic_sized_800(url),
                sort_order: i32::try_from(idx).unwrap_or(i32::MAX),
                shop_id: 0,
                ..Default::default()
            })
            .await?;
        }
        Ok(())
    }

    /// 新建/编辑拍品。编辑仅限草稿/预告状态；保证金按当前设置重算。
    ///
    /// 商品口径（2026-11-21 用户变更）：拍品不再选既有商品——新建时按表单
    /// 商品资料同步新建一个拍卖专属商品（下架态 `product_status = 0`：不进
    /// 普通列表/搜索、普通详情下架拦截、不可加购下单，但按 id 引用不受
    /// 状态过滤，拍品展示与结拍订单照常；库存恒 1 件），拍品行标记
    /// `own_product = 1`；编辑时把商品资料同步回拍品关联商品（存量拍品
    /// 关联的既有商品同口径回写）。
    pub async fn item_save(&self, dto: &AuctionItemSaveDTO, admin_id: i32) -> Result<i32> {
        let settings = self.get_settings().await?;
        let now = Self::now();
        let start_price = dto.start_price.unwrap_or_default();
        if start_price <= Decimal::ZERO {
            return Err(Error::Validation("起拍价必须大于 0".into()));
        }
        let reserve_price = match dto.reserve_price {
            Some(r) if r > Decimal::ZERO => {
                if !settings.reserve_price_enabled {
                    return Err(Error::biz(400, "保留价功能未开启（拍卖设置）"));
                }
                Some(r)
            }
            _ => None,
        };
        let buyout_price = match dto.buyout_price {
            Some(b) if b > Decimal::ZERO => {
                if !settings.buyout_enabled {
                    return Err(Error::biz(400, "一口价功能未开启（拍卖设置）"));
                }
                if b <= start_price {
                    return Err(Error::Validation("一口价必须大于起拍价".into()));
                }
                Some(b)
            }
            _ => None,
        };
        if let (Some(r), Some(b)) = (reserve_price, buyout_price) {
            if r > b {
                return Err(Error::Validation("保留价不得高于一口价".into()));
            }
        }
        let start_time = dto.start_time.unwrap_or(0);
        let end_time = dto.end_time.unwrap_or(0);
        if start_time <= 0 || end_time <= 0 || start_time >= end_time {
            return Err(Error::Validation("开始/结束时间不合法".into()));
        }
        if end_time <= now {
            return Err(Error::Validation("结束时间已过".into()));
        }
        // —— 商品资料校验（新建/编辑同口径，表单必填项与后端校验一一对应） ——
        let product_name = dto
            .product_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation("请填写商品名称".into()))?
            .to_string();
        let pics: Vec<String> = dto
            .pics
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if pics.is_empty() {
            return Err(Error::Validation("请至少上传一张商品图片".into()));
        }
        let market_price = dto
            .market_price
            .filter(|p| *p > Decimal::ZERO)
            .ok_or_else(|| Error::Validation("请填写市场价".into()))?;
        let product_brief = dto.product_brief.clone().unwrap_or_default();
        let product_desc = dto.product_desc.clone().filter(|s| !s.trim().is_empty());

        let editing = dto.item_id.filter(|id| *id > 0);
        let status = if start_time > now { item_status::PREVIEW } else { item_status::RUNNING };
        let title = dto
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&product_name)
            .to_string();
        if let Some(id) = editing {
            let existing = self
                .repo()
                .find_item(id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
            if !matches!(existing.status, item_status::DRAFT | item_status::PREVIEW) {
                return Err(Error::biz(400, "仅草稿/预告中的拍品可编辑"));
            }
            if existing.bid_count > 0 {
                return Err(Error::biz(400, "已有出价的拍品不可编辑"));
            }
            if self
                .repo()
                .exists_conflicting_item(existing.product_id, start_time, end_time, editing)
                .await?
            {
                return Err(Error::biz(400, "该商品在此时段已有未结束的拍品"));
            }
            // 商品资料同步回拍品关联商品（专属商品与存量关联商品同口径回写；
            // 商品归属/状态/库存不在此改动，拍品关联不可换绑）。
            let mut product = self
                .repos
                .catalog
                .product
                .find_by_id(existing.product_id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品关联商品不存在".into()))?;
            product.product_name = product_name.clone();
            product.product_brief = product_brief;
            product.product_desc = product_desc;
            product.product_price = market_price;
            product.market_price = market_price;
            product.pic_url = pics[0].clone();
            product.pic_thumb = utils::normalize_pic_thumb(&pics[0], "");
            product.pic_original = pics[0].clone();
            product.pic_large = utils::pic_sized_800(&pics[0]);
            product.last_update = now;
            self.repos.catalog.product_manage.update_product(&product).await?;
            self.sync_item_gallery(existing.product_id, &pics).await?;
            let shop_id = existing.shop_id;
            let mut row = existing;
            row.session_id = dto.session_id;
            // product_id / sku_id / shop_id / own_product 保持既有关联不变
            row.title = title;
            row.start_price = start_price;
            row.reserve_price = reserve_price;
            row.buyout_price = buyout_price;
            row.deposit_amount = settings.deposit_for(start_price);
            row.start_time = start_time;
            row.end_time = end_time;
            row.real_end_time = end_time;
            row.status = status;
            row.update_time = now;
            self.repo().update_item(&row).await?;
            self.admin_log(shop_id, &format!("编辑拍品:{}(#{})", row.title, id)).await;
            Ok(id)
        } else {
            let shop_id = dto.shop_id.unwrap_or(0);
            // 新建拍卖专属商品：下架态隔离普通售卖通道，库存恒 1 件、单规格。
            // 货号回填 AUC{product_id}（先插入拿 id，再全量回写，与商品新建同路数）。
            let mut product = Product {
                product_name: product_name.clone(),
                product_stock: 1,
                product_price: market_price,
                market_price,
                product_status: 0,
                product_type: Some(1),
                delivery_mode: 1,
                sale_mode: 0,
                shop_id,
                check_status: 1,
                product_brief,
                product_desc,
                pic_url: pics[0].clone(),
                pic_thumb: utils::normalize_pic_thumb(&pics[0], ""),
                pic_original: pics[0].clone(),
                pic_large: utils::pic_sized_800(&pics[0]),
                is_logistics: 1,
                add_time: now,
                last_update: now,
                remark: "拍卖拍品专属商品（建拍品时系统新建）".into(),
                ..Default::default()
            };
            let product_id = self.repos.catalog.product_manage.insert_product(&product).await?;
            product.product_id = product_id;
            product.product_sn = format!("AUC{product_id}");
            self.repos.catalog.product_manage.update_product(&product).await?;
            self.repos
                .catalog
                .product_manage
                .insert_inventory_log(product_id, 0, 1, 1, "库存新增", shop_id)
                .await?;
            self.sync_item_gallery(product_id, &pics).await?;
            let row = AuctionItem {
                session_id: dto.session_id,
                product_id,
                sku_id: 0,
                own_product: 1,
                shop_id,
                title,
                start_price,
                reserve_price,
                buyout_price,
                deposit_amount: settings.deposit_for(start_price),
                start_time,
                end_time,
                real_end_time: end_time,
                status,
                create_admin_id: admin_id,
                add_time: now,
                update_time: now,
                ..Default::default()
            };
            let id = self.repo().insert_item(&row).await?;
            self.admin_log(shop_id, &format!("新建拍品:{}(#{})", row.title, id)).await;
            Ok(id)
        }
    }

    pub async fn item_delete(&self, item_id: i32) -> Result<()> {
        let item = self
            .repo()
            .find_item(item_id)
            .await?
            .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        if item.status != item_status::DRAFT {
            return Err(Error::biz(400, "仅草稿拍品可删除"));
        }
        if item.bid_count > 0 {
            return Err(Error::biz(400, "已有出价的拍品不可删除"));
        }
        self.repo().delete_item(item_id).await?;
        // 拍品自建的专属商品一并清理（软删与商品侧同口径 + 清相册行），
        // 不留孤儿商品；存量拍品关联的是既有真实商品，只删拍品行、商品不动。
        if item.own_product == 1 {
            self.repos.catalog.product_manage.soft_delete_products(&[item.product_id]).await?;
            self.repos.catalog.product_manage.delete_gallery(item.product_id).await?;
        }
        Ok(())
    }

    /// 撤拍（竞价开始前）：状态置已撤拍，已冻结保证金全额解冻。
    pub async fn item_cancel(&self, item_id: i32) -> Result<()> {
        let now = Self::now();
        let mut notices = Vec::new();
        {
            let mut tx = self.repo().begin().await?;
            let item = self
                .repo()
                .find_item_for_update(&mut tx, item_id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
            if !matches!(item.status, item_status::DRAFT | item_status::PREVIEW) {
                return Err(Error::biz(400, "竞价开始后不可撤拍，请使用强制结束"));
            }
            let deposits = self.repo().find_frozen_deposits_in_tx(&mut tx, item_id).await?;
            for d in &deposits {
                self.release_deposit_in_tx(&mut tx, d, &item.title, now, "拍卖撤拍，保证金解冻").await?;
                notices.push(Notice {
                    user_id: d.user_id,
                    title: "拍卖已取消".into(),
                    content: format!("拍品「{}」已被取消，您的保证金 ¥{} 已解冻。", item.title, d.amount),
                });
            }
            let mut row = item;
            row.status = item_status::CANCELED;
            row.update_time = now;
            self.repo().update_item_in_tx(&mut tx, &row).await?;
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        }
        self.send_notices(notices).await;
        self.admin_log(0, &format!("撤拍拍品 #{item_id}")).await;
        Ok(())
    }

    /// 强制结束（竞价中异常处置）：不成交、全部保证金解冻、无人违约。
    pub async fn item_force_end(&self, item_id: i32) -> Result<()> {
        let now = Self::now();
        let mut notices = Vec::new();
        {
            let mut tx = self.repo().begin().await?;
            let item = self
                .repo()
                .find_item_for_update(&mut tx, item_id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
            if item.is_terminal() {
                return Err(Error::biz(400, "拍品已结束"));
            }
            let deposits = self.repo().find_frozen_deposits_in_tx(&mut tx, item_id).await?;
            for d in &deposits {
                self.release_deposit_in_tx(&mut tx, d, &item.title, now, "拍卖强制结束，保证金解冻").await?;
                notices.push(Notice {
                    user_id: d.user_id,
                    title: "拍卖已结束".into(),
                    content: format!("拍品「{}」已由平台强制结束，您的保证金 ¥{} 已解冻。", item.title, d.amount),
                });
            }
            let mut row = item;
            row.status = item_status::FORCE_ENDED;
            row.update_time = now;
            self.repo().update_item_in_tx(&mut tx, &row).await?;
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        }
        self.send_notices(notices).await;
        self.admin_log(0, &format!("强制结束拍品 #{item_id}")).await;
        Ok(())
    }
}

/// 场次 VO 组装。
fn session_vo(s: &AuctionSession, item_count: i64) -> AuctionSessionVO {
    AuctionSessionVO {
        session_id: s.session_id,
        session_name: s.session_name.clone(),
        cover_pic: s.cover_pic.clone(),
        start_time: s.start_time,
        end_time: s.end_time,
        start_time_text: fmt_time(s.start_time),
        end_time_text: fmt_time(s.end_time),
        sort: s.sort,
        status: s.status,
        item_count,
    }
}

/// 管理端列表的店铺过滤：一期管理端为平台视角，请求值直用（商户自助二期再收口）。
fn scoped_shop(requested: Option<i32>) -> Option<i32> {
    requested.filter(|id| *id >= 0)
}

impl AuctionService {
    // ————————————————————— VO 组装 —————————————————————

    async fn load_products(&self, ids: &[i32]) -> Result<HashMap<i32, Product>> {
        let unique: Vec<i32> = ids.iter().copied().collect::<std::collections::HashSet<_>>().into_iter().collect();
        if unique.is_empty() {
            return Ok(HashMap::new());
        }
        let products = self.repos.catalog.product.find_by_ids(&unique).await?;
        Ok(products.into_iter().map(|p| (p.product_id, p)).collect())
    }

    /// 中标人展示名（脱敏前原文），按 winner_user_id 批量取。
    async fn winner_names(&self, items: &[AuctionItem]) -> Result<HashMap<i32, String>> {
        let ids: Vec<i32> = items.iter().filter_map(|i| i.winner_user_id).filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let displays = self.repo().find_user_displays(&ids).await?;
        Ok(displays
            .into_iter()
            .map(|(id, username, nickname)| {
                let name = nickname.filter(|n| !n.trim().is_empty()).unwrap_or(username);
                (id, name)
            })
            .collect())
    }

    /// 拍品 → 卡片 VO（状态按服务器时间派生展示，终态以落库为准）。
    fn item_vo(
        &self,
        item: &AuctionItem,
        products: &HashMap<i32, Product>,
        winners: &HashMap<i32, String>,
        now: i32,
    ) -> AuctionItemVO {
        let product = products.get(&item.product_id);
        let eff = effective_status(item, now);
        let remain = match eff {
            item_status::PREVIEW => i64::from(item.start_time - now).max(0),
            item_status::RUNNING => i64::from(item.real_end_time - now).max(0),
            _ => 0,
        };
        AuctionItemVO {
            item_id: item.item_id,
            session_id: item.session_id,
            product_id: item.product_id,
            sku_id: item.sku_id,
            shop_id: item.shop_id,
            title: item.title.clone(),
            product_name: product.map(|p| p.product_name.clone()).unwrap_or_default(),
            product_sn: product.map(|p| p.product_sn.clone()).unwrap_or_default(),
            pic_url: product.map(|p| p.pic_url.clone()).unwrap_or_default(),
            pic_thumb: product.map(|p| p.pic_thumb.clone()).unwrap_or_default(),
            start_price: item.start_price,
            current_price: item.display_price(),
            has_bid: item.current_bid_id.is_some(),
            buyout_price: item.buyout_price,
            has_reserve: item.reserve_price.is_some(),
            deposit_amount: item.deposit_amount,
            start_time: item.start_time,
            end_time: item.end_time,
            real_end_time: item.real_end_time,
            start_time_text: fmt_time(item.start_time),
            end_time_text: fmt_time(item.real_end_time),
            remain_seconds: remain,
            extend_count: item.extend_count,
            status: eff,
            status_name: item_status::name(eff).to_string(),
            bid_count: item.bid_count,
            final_price: item.final_price,
            winner_masked_name: item
                .winner_user_id
                .and_then(|id| winners.get(&id))
                .map(|n| mask_name(n)),
            order_id: item.order_id,
            server_time: i64::from(now),
        }
    }

    /// 出价行 → 出价墙 VO（脱敏）。
    fn bid_vo(&self, bid: &AuctionBid, names: &HashMap<i32, String>, leader_bid_id: Option<i32>, viewer_id: i32) -> AuctionBidVO {
        AuctionBidVO {
            bid_id: bid.bid_id,
            item_id: bid.item_id,
            masked_name: names
                .get(&bid.user_id)
                .map(|n| mask_name(n))
                .unwrap_or_else(|| "匿名用户".to_string()),
            bid_price: bid.bid_price,
            is_buyout: bid.is_buyout == 1,
            is_leader: leader_bid_id == Some(bid.bid_id),
            is_mine: viewer_id > 0 && bid.user_id == viewer_id,
            add_time: bid.add_time,
            add_time_text: fmt_time(bid.add_time),
        }
    }

    async fn bid_names(&self, bids: &[AuctionBid]) -> Result<HashMap<i32, String>> {
        let ids: Vec<i32> = bids.iter().map(|b| b.user_id).collect::<std::collections::HashSet<_>>().into_iter().collect();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let displays = self.repo().find_user_displays(&ids).await?;
        Ok(displays
            .into_iter()
            .map(|(id, username, nickname)| {
                (id, nickname.filter(|n| !n.trim().is_empty()).unwrap_or(username))
            })
            .collect())
    }

    // ————————————————————— 管理端：出价/保证金/违约/封禁/统计 —————————————————————

    pub async fn bid_list_admin(&self, q: &AuctionBidListQuery) -> Result<PageResult<AuctionAdminBidVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let (total, rows) = self.repo().paginate_bids_admin(q.item_id, q.user_id, &page).await?;
        let list = rows
            .into_iter()
            .map(|r| AuctionAdminBidVO {
                bid_id: r.bid_id,
                item_id: r.item_id,
                item_title: r.item_title,
                user_id: r.user_id,
                username: r.username,
                nickname: r.nickname.unwrap_or_default(),
                bid_price: r.bid_price,
                is_buyout: r.is_buyout,
                status: r.status,
                status_name: if r.status == bid_status::VOID { "已作废".to_string() } else { "有效".to_string() },
                ip: r.ip,
                user_agent: r.user_agent,
                device_fp: r.device_fp,
                add_time: r.add_time,
                add_time_text: fmt_time(r.add_time),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 管理员作废出价（结拍前）：作废后按剩余有效出价重算当前价/领先者。
    pub async fn bid_void(&self, bid_id: i32) -> Result<()> {
        let now = Self::now();
        let bid = self
            .repo()
            .find_bid(bid_id)
            .await?
            .ok_or_else(|| Error::NotFound("出价记录不存在".into()))?;
        if bid.status != bid_status::VALID {
            return Err(Error::biz(400, "该出价已作废"));
        }
        let mut notices = Vec::new();
        {
            let mut tx = self.repo().begin().await?;
            let item = self
                .repo()
                .find_item_for_update(&mut tx, bid.item_id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
            if item.is_terminal() {
                return Err(Error::biz(400, "拍品已结拍，不能作废出价"));
            }
            let was_leader = item.current_bid_id == Some(bid_id);
            self.repo().void_bid_in_tx(&mut tx, bid_id).await?;
            let top = self.repo().find_top_valid_bid_in_tx(&mut tx, bid.item_id).await?;
            let count = self.repo().count_valid_bids_in_tx(&mut tx, bid.item_id).await?;
            let mut row = item;
            row.current_price = top.as_ref().map(|b| b.bid_price);
            row.current_bid_id = top.as_ref().map(|b| b.bid_id);
            row.bid_count = count as i32;
            row.update_time = now;
            self.repo().update_item_in_tx(&mut tx, &row).await?;
            if was_leader {
                if let Some(new_top) = &top {
                    notices.push(Notice {
                        user_id: new_top.user_id,
                        title: "您已重回领先".into(),
                        content: format!("拍品「{}」的一笔异常出价已被作废，您以 ¥{} 重新成为最高出价者。", row.title, new_top.bid_price),
                    });
                }
            }
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        }
        self.send_notices(notices).await;
        self.admin_log(0, &format!("作废出价 #{bid_id}")).await;
        Ok(())
    }

    pub async fn deposit_list_admin(&self, q: &AuctionDepositListQuery) -> Result<PageResult<AuctionDepositVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let (total, rows) = self.repo().paginate_deposits(q.item_id, q.user_id, q.status, &page).await?;
        Ok(PageResult::new(total, rows.into_iter().map(deposit_vo).collect(), &page))
    }

    pub async fn breach_list(&self, q: &AuctionBreachListQuery) -> Result<PageResult<AuctionBreachVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let (total, rows) = self.repo().paginate_breaches(q.user_id, &page).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            let count = self.repo().count_breaches_since(r.user_id, 0).await?;
            list.push(AuctionBreachVO {
                breach_id: r.breach_id,
                user_id: r.user_id,
                username: r.username,
                nickname: r.nickname.unwrap_or_default(),
                item_id: r.item_id,
                item_title: r.item_title,
                order_id: r.order_id,
                deposit_amount: r.deposit_amount,
                penalty_amount: r.penalty_amount,
                breach_time: r.breach_time,
                breach_time_text: fmt_time(r.breach_time),
                waived: r.waived,
                waive_reason: r.waive_reason,
                breach_count: count,
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 扣罚改判：豁免某笔违约，把已扣罚金额退回用户余额（封禁不自动解除，走解封操作）。
    pub async fn waive_penalty(&self, breach_id: i32, reason: &str, operator_id: i32) -> Result<()> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Error::Validation("改判必须填写理由".into()));
        }
        let breach = self
            .repo()
            .find_breach(breach_id)
            .await?
            .ok_or_else(|| Error::NotFound("违约记录不存在".into()))?;
        if breach.waived == 1 {
            return Err(Error::biz(400, "该违约已改判豁免"));
        }
        let now = Self::now();
        let mut tx = self.repo().begin().await?;
        if breach.penalty_amount > Decimal::ZERO {
            let title = self
                .repo()
                .find_item(breach.item_id)
                .await
                .ok()
                .flatten()
                .map(|i| i.title)
                .unwrap_or_default();
            self.wallet_move_in_tx(
                &mut tx,
                breach.user_id,
                breach.penalty_amount,
                Decimal::ZERO,
                1,
                &format!("拍卖违约扣罚改判退回[拍品：{title}]"),
                now,
            )
            .await?;
            // 保证金记录同步标注（按订单定位中标人那条）
            if let Some(mut d) = self.repo().find_deposit_by_order_in_tx(&mut tx, breach.order_id).await? {
                d.remark = format!("扣罚已改判豁免：{reason}");
                self.repo().update_deposit_in_tx(&mut tx, &d).await?;
            }
        }
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        self.repo().waive_breach(breach_id, reason, operator_id).await?;
        self.admin_log(0, &format!("拍卖违约改判 #{breach_id}：{reason}")).await;
        Ok(())
    }

    pub async fn ban_list(&self, q: &AuctionBreachListQuery) -> Result<PageResult<AuctionBanVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let (total, rows) = self.repo().paginate_bans(q.user_id, &page).await?;
        let list = rows
            .into_iter()
            .map(|r| AuctionBanVO {
                ban_id: r.ban_id,
                user_id: r.user_id,
                username: r.username,
                nickname: r.nickname.unwrap_or_default(),
                ban_type: r.ban_type,
                ban_type_name: if r.ban_type == 2 { "永久".to_string() } else { "临时".to_string() },
                start_time: r.start_time,
                end_time: r.end_time,
                end_time_text: if r.ban_type == 2 { "永久".to_string() } else { fmt_time(r.end_time) },
                reason: r.reason,
                status: r.status,
                operator_id: r.operator_id,
                add_time: r.add_time,
                add_time_text: fmt_time(r.add_time),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 管理员手动封禁参拍资格。
    pub async fn ban_save(&self, dto: &AuctionBanSaveDTO, operator_id: i32) -> Result<i32> {
        let _user = self.load_user(dto.user_id).await?;
        let now = Self::now();
        let ban_type = dto.ban_type.unwrap_or(1);
        let (end_time, ban_type) = if ban_type == 2 {
            (0, 2)
        } else {
            let days = dto.days.unwrap_or(30);
            if days <= 0 {
                return Err(Error::Validation("封禁天数必须大于 0".into()));
            }
            (now + days * 86_400, 1)
        };
        let id = self
            .repo()
            .insert_ban(&AuctionBan {
                user_id: dto.user_id,
                ban_type,
                start_time: now,
                end_time,
                reason: dto.reason.clone().unwrap_or_default(),
                status: 1,
                operator_id,
                add_time: now,
                ..Default::default()
            })
            .await?;
        self.admin_log(0, &format!("封禁用户 #{} 参拍资格", dto.user_id)).await;
        Ok(id)
    }

    pub async fn unban(&self, ban_id: i32) -> Result<()> {
        self.repo().unban(ban_id).await?;
        self.admin_log(0, &format!("解封参拍资格 #{ban_id}")).await;
        Ok(())
    }

    pub async fn stats(&self) -> Result<AuctionStatsVO> {
        let (total, running, sold, unsold, bids, frozen, penalty, avg_ratio) = self.repo().stats_raw().await?;
        let closed = sold + unsold;
        let deal_rate = if closed > 0 {
            (sold as f64) * 100.0 / (closed as f64)
        } else {
            0.0
        };
        let avg_premium_rate = avg_ratio
            .and_then(|d| d.to_string().parse::<f64>().ok())
            .map(|r| (r - 1.0) * 100.0)
            .unwrap_or(0.0);
        Ok(AuctionStatsVO {
            total_items: total,
            running_items: running,
            sold_items: sold,
            unsold_items: unsold,
            deal_rate: (deal_rate * 10.0).round() / 10.0,
            avg_premium_rate: (avg_premium_rate * 10.0).round() / 10.0,
            total_bids: bids,
            frozen_deposit_total: frozen,
            penalty_total: penalty,
        })
    }
}

/// 保证金行 → VO。
fn deposit_vo(r: nextrade_infrastructure::db::repo::marketing::auction::DepositRow) -> AuctionDepositVO {
    AuctionDepositVO {
        deposit_id: r.deposit_id,
        item_id: r.item_id,
        item_title: r.item_title,
        user_id: r.user_id,
        username: r.username,
        nickname: r.nickname.unwrap_or_default(),
        amount: r.amount,
        status: r.status,
        status_name: deposit_status::name(r.status).to_string(),
        balance_log_id: r.balance_log_id,
        order_id: r.order_id,
        penalty_amount: r.penalty_amount,
        add_time: r.add_time,
        add_time_text: fmt_time(r.add_time),
        handle_time: r.handle_time,
        handle_time_text: fmt_time(r.handle_time),
        remark: r.remark,
    }
}

/// 按服务器时间派生的展示状态：预告到期未推进时按竞价中展示（落库由定时任务推进）。
fn effective_status(item: &AuctionItem, now: i32) -> i32 {
    if item.status == item_status::PREVIEW && now >= item.start_time {
        item_status::RUNNING
    } else {
        item.status
    }
}

impl AuctionService {
    // ————————————————————— 前台：读 —————————————————————

    pub async fn sessions_public(&self) -> Result<Vec<AuctionSessionVO>> {
        let rows = self.repo().list_sessions(true).await?;
        Ok(rows.into_iter().map(|(s, cnt)| session_vo(&s, cnt)).collect())
    }

    pub async fn items_public(&self, q: &AuctionPublicListQuery) -> Result<PageResult<AuctionItemVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let now = Self::now();
        let statuses: Vec<i32> = match q.tab.as_deref() {
            Some("preview") => vec![item_status::PREVIEW],
            Some("running") => vec![item_status::PREVIEW, item_status::RUNNING],
            Some("finished") => vec![item_status::SOLD, item_status::UNSOLD, item_status::FORCE_ENDED],
            _ => vec![item_status::PREVIEW, item_status::RUNNING, item_status::SOLD, item_status::UNSOLD],
        };
        // 「竞价中」页签含已到开始时间的预告（展示态已推进，落库由任务推进）
        let sort = q.sort.as_deref().unwrap_or("ending");
        let (total, rows) = self.repo().paginate_items_public(q.session_id, &statuses, sort, &page).await?;
        let products = self.load_products(&rows.iter().map(|i| i.product_id).collect::<Vec<_>>()).await?;
        let winners = self.winner_names(&rows).await?;
        let list = rows
            .iter()
            .filter(|i| match q.tab.as_deref() {
                Some("running") => effective_status(i, now) == item_status::RUNNING,
                Some("preview") => effective_status(i, now) == item_status::PREVIEW,
                _ => true,
            })
            .map(|i| self.item_vo(i, &products, &winners, now))
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    pub async fn item_detail(&self, item_id: i32, viewer_id: i32) -> Result<AuctionItemDetailVO> {
        let settings = self.get_settings().await?;
        let now = Self::now();
        let item = self
            .repo()
            .find_item(item_id)
            .await?
            .filter(|i| i.status != item_status::DRAFT)
            .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        let products = self.load_products(&[item.product_id]).await?;
        let winners = self.winner_names(std::slice::from_ref(&item)).await?;
        let vo = self.item_vo(&item, &products, &winners, now);
        let product = products.get(&item.product_id);
        let (_, bids) = self
            .repo()
            .list_bids(item_id, &PageQuery { page: 1, page_size: 20 })
            .await?;
        let names = self.bid_names(&bids).await?;
        let bid_vos = bids.iter().map(|b| self.bid_vo(b, &names, item.current_bid_id, viewer_id)).collect();

        let (my_deposit_status, my_deposit_amount, my_highest, reminded, my_certified, ban_reason) =
            if viewer_id > 0 {
                let dep = self.repo().find_deposit(item_id, viewer_id).await?;
                let my_highest = self.repo().find_user_max_bid(item_id, viewer_id).await?.unwrap_or_default();
                let reminded = self.repo().has_remind(item_id, viewer_id).await?;
                let user = self.repos.member.user.find_by_id(viewer_id).await?;
                let certified = user.as_ref().map(|u| u.is_company_auth == 1).unwrap_or(false);
                let ban = self.repo().find_active_ban(viewer_id, now).await?;
                let ban_reason = ban.map(|b| {
                    if b.ban_type == 2 {
                        "参拍资格已被永久取消".to_string()
                    } else {
                        format!("参拍资格封禁至 {}", fmt_time(b.end_time))
                    }
                });
                (
                    dep.as_ref().map(|d| d.status).unwrap_or(0),
                    dep.map(|d| d.amount).unwrap_or_default(),
                    my_highest,
                    reminded,
                    certified,
                    ban_reason,
                )
            } else {
                (0, Decimal::ZERO, Decimal::ZERO, false, false, None)
            };
        let is_leader = viewer_id > 0 && my_highest > Decimal::ZERO && item.current_price == Some(my_highest);

        Ok(AuctionItemDetailVO {
            product_brief: product.map(|p| p.product_brief.clone()).unwrap_or_default(),
            product_desc: product.and_then(|p| p.product_desc.clone()),
            product_price: product.map(|p| p.product_price).unwrap_or_default(),
            market_price: product.map(|p| p.market_price).unwrap_or_default(),
            step_amount: settings.step_for(item.display_price()),
            min_next_bid: settings.min_next_bid(item.current_price, item.start_price),
            my_deposit_status,
            my_deposit_amount,
            my_highest_bid: my_highest,
            is_leader,
            is_winner: viewer_id > 0 && item.winner_user_id == Some(viewer_id),
            reminded,
            ban_reason,
            require_certification: settings.require_certification,
            my_certified,
            bids: bid_vos,
            item: vo,
        })
    }

    pub async fn item_bids(&self, item_id: i32, page_no: Option<u64>, page_size: Option<u64>, viewer_id: i32) -> Result<PageResult<AuctionBidVO>> {
        let page = PageQuery { page: page_no.unwrap_or(1), page_size: page_size.unwrap_or(20) };
        let item = self.repo().find_item(item_id).await?.ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        let (total, bids) = self.repo().list_bids(item_id, &page).await?;
        let names = self.bid_names(&bids).await?;
        let list = bids.iter().map(|b| self.bid_vo(b, &names, item.current_bid_id, viewer_id)).collect();
        Ok(PageResult::new(total, list, &page))
    }

    pub async fn item_poll(&self, item_id: i32, viewer_id: i32) -> Result<AuctionPollVO> {
        let settings = self.get_settings().await?;
        let now = Self::now();
        let item = self
            .repo()
            .find_item(item_id)
            .await?
            .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        let (_, bids) = self.repo().list_bids(item_id, &PageQuery { page: 1, page_size: 3 }).await?;
        let names = self.bid_names(&bids).await?;
        let is_leader = if viewer_id > 0 {
            match self.repo().find_user_max_bid(item_id, viewer_id).await? {
                Some(m) => item.current_price == Some(m),
                None => false,
            }
        } else {
            false
        };
        let eff = effective_status(&item, now);
        Ok(AuctionPollVO {
            item_id: item.item_id,
            status: eff,
            status_name: item_status::name(eff).to_string(),
            current_price: item.display_price(),
            has_bid: item.current_bid_id.is_some(),
            bid_count: item.bid_count,
            real_end_time: item.real_end_time,
            remain_seconds: match eff {
                item_status::PREVIEW => i64::from(item.start_time - now).max(0),
                item_status::RUNNING => i64::from(item.real_end_time - now).max(0),
                _ => 0,
            },
            extend_count: item.extend_count,
            is_leader,
            min_next_bid: settings.min_next_bid(item.current_price, item.start_price),
            step_amount: settings.step_for(item.display_price()),
            server_time: i64::from(now),
            latest_bids: bids.iter().map(|b| self.bid_vo(b, &names, item.current_bid_id, viewer_id)).collect(),
        })
    }

    // ————————————————————— 前台：保证金冻结/退出 —————————————————————

    /// 冻结保证金（幂等：一品一冻，重复调用返回既有冻结）。
    pub async fn freeze_deposit(&self, user_id: i32, item_id: i32) -> Result<AuctionDepositVO> {
        let settings = self.get_settings().await?;
        let now = Self::now();
        self.ensure_can_bid(user_id, &settings, now).await?;
        let mut tx = self.repo().begin().await?;
        let item = self
            .repo()
            .find_item_for_update(&mut tx, item_id)
            .await?
            .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        if !matches!(item.status, item_status::PREVIEW | item_status::RUNNING) || now >= item.real_end_time {
            return Err(Error::biz(400, "该拍品当前不可参拍"));
        }
        self.ensure_not_self_deal(user_id, &item)?;
        if let Some(existing) = self.repo().find_deposit_in_tx(&mut tx, item_id, user_id).await? {
            if existing.status == deposit_status::FROZEN {
                tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
                return Ok(self.deposit_vo_by_id(existing.deposit_id).await?);
            }
            // 曾退出/已解冻的记录：重新冻结并复用该行（唯一约束一人一拍品一条）
            let log_id = self
                .wallet_move_in_tx(
                    &mut tx,
                    user_id,
                    -item.deposit_amount,
                    item.deposit_amount,
                    2,
                    &format!("拍卖保证金冻结[拍品：{}]", item.title),
                    now,
                )
                .await?;
            let mut d = existing;
            d.amount = item.deposit_amount;
            d.status = deposit_status::FROZEN;
            d.balance_log_id = Some(log_id);
            d.order_id = None;
            d.penalty_amount = Decimal::ZERO;
            d.add_time = now;
            d.handle_time = 0;
            d.remark = String::new();
            self.repo().update_deposit_in_tx(&mut tx, &d).await?;
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
            return Ok(self.deposit_vo_by_id(d.deposit_id).await?);
        }
        let log_id = self
            .wallet_move_in_tx(
                &mut tx,
                user_id,
                -item.deposit_amount,
                item.deposit_amount,
                2,
                &format!("拍卖保证金冻结[拍品：{}]", item.title),
                now,
            )
            .await?;
        let deposit_id = self
            .repo()
            .insert_deposit_in_tx(
                &mut tx,
                &AuctionDeposit {
                    item_id,
                    user_id,
                    amount: item.deposit_amount,
                    status: deposit_status::FROZEN,
                    balance_log_id: Some(log_id),
                    add_time: now,
                    ..Default::default()
                },
            )
            .await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(self.deposit_vo_by_id(deposit_id).await?)
    }

    async fn deposit_vo_by_id(&self, deposit_id: i32) -> Result<AuctionDepositVO> {
        let d = self
            .repo()
            .find_deposit_by_id(deposit_id)
            .await?
            .ok_or_else(|| Error::NotFound("保证金记录不存在".into()))?;
        let item_title = self.repo().find_item(d.item_id).await?.map(|i| i.title).unwrap_or_default();
        let user = self.repos.member.user.find_by_id(d.user_id).await?;
        Ok(AuctionDepositVO {
            deposit_id: d.deposit_id,
            item_id: d.item_id,
            item_title,
            user_id: d.user_id,
            username: user.as_ref().map(|u| u.username.clone()).unwrap_or_default(),
            nickname: user.and_then(|u| u.nickname).unwrap_or_default(),
            amount: d.amount,
            status: d.status,
            status_name: deposit_status::name(d.status).to_string(),
            balance_log_id: d.balance_log_id,
            order_id: d.order_id,
            penalty_amount: d.penalty_amount,
            add_time: d.add_time,
            add_time_text: fmt_time(d.add_time),
            handle_time: d.handle_time,
            handle_time_text: fmt_time(d.handle_time),
            remark: d.remark,
        })
    }

    /// 退出参拍（非领先者）：立即解冻，本场不可再参拍以「无冻结」体现（可重新冻结再参拍，
    /// 与规划「退出后本场不可再参拍」相比一期放宽为可重进，见 REPORT 说明）。
    pub async fn quit(&self, user_id: i32, item_id: i32) -> Result<()> {
        let now = Self::now();
        let mut tx = self.repo().begin().await?;
        let item = self
            .repo()
            .find_item_for_update(&mut tx, item_id)
            .await?
            .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        if let Some(top) = self.repo().find_top_valid_bid_in_tx(&mut tx, item_id).await? {
            if top.user_id == user_id {
                return Err(Error::biz(400, "您是当前最高出价者，不能退出参拍"));
            }
        }
        let deposit = self
            .repo()
            .find_deposit_in_tx(&mut tx, item_id, user_id)
            .await?
            .filter(|d| d.status == deposit_status::FROZEN)
            .ok_or_else(|| Error::biz(400, "没有冻结中的保证金"))?;
        self.release_deposit_in_tx(&mut tx, &deposit, &item.title, now, "退出参拍，保证金解冻")
            .await?;
        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    // ————————————————————— 前台：出价 —————————————————————

    /// 出价（含一口价）。拍品行锁内完成校验、落账、顺延与（一口价）结拍。
    pub async fn place_bid(
        &self,
        user_id: i32,
        dto: &AuctionBidDTO,
        ip: &str,
        user_agent: &str,
    ) -> Result<BidResultVO> {
        let settings = self.get_settings().await?;
        let now = Self::now();
        self.ensure_can_bid(user_id, &settings, now).await?;
        let is_buyout = dto.is_buyout.unwrap_or(0) == 1;
        let mut notices: Vec<Notice> = Vec::new();
        let mut result: BidResultVO;
        {
            let mut tx = self.repo().begin().await?;
            let item = self
                .repo()
                .find_item_for_update(&mut tx, dto.item_id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
            self.ensure_not_self_deal(user_id, &item)?;
            // 状态推进：预告到点视为竞价中
            let mut item = item;
            if item.status == item_status::PREVIEW && now >= item.start_time {
                item.status = item_status::RUNNING;
            }
            if item.status != item_status::RUNNING {
                return Err(Error::biz(400, "该拍品不在竞价中"));
            }
            if now >= item.real_end_time {
                return Err(Error::biz(400, "该拍品已结束"));
            }
            // 保证金须已冻结
            let deposit = self.repo().find_deposit_in_tx(&mut tx, dto.item_id, user_id).await?;
            match &deposit {
                Some(d) if d.status == deposit_status::FROZEN => {}
                _ => return Err(Error::biz(4001, "请先冻结参拍保证金")),
            }
            // 领先者不可连出
            let prev_top = self.repo().find_top_valid_bid_in_tx(&mut tx, dto.item_id).await?;
            if let Some(top) = &prev_top {
                if top.user_id == user_id {
                    return Err(Error::biz(400, "您已是当前最高出价者"));
                }
            }
            // 金额校验
            if is_buyout {
                if !settings.buyout_enabled {
                    return Err(Error::biz(400, "一口价功能未开启"));
                }
                let buyout = item.buyout_price.ok_or_else(|| Error::biz(400, "该拍品未设置一口价"))?;
                if dto.bid_price != buyout {
                    return Err(Error::biz(400, "一口价金额不正确"));
                }
            } else {
                if dto.bid_price <= Decimal::ZERO {
                    return Err(Error::Validation("出价金额必须大于 0".into()));
                }
                if !settings.is_valid_bid(item.current_price, item.start_price, dto.bid_price) {
                    let min = settings.min_next_bid(item.current_price, item.start_price);
                    return Err(Error::biz(400, format!("出价不合法，最低可出价 ¥{min}")));
                }
                // 价格异常保护：单笔超当前价 10 倍须二次确认
                if let Some(cur) = item.current_price {
                    if dto.bid_price > cur * Decimal::from(10) && dto.confirm_high.unwrap_or(0) != 1 {
                        return Err(Error::biz(4002, "出价超过当前价 10 倍，请确认后重试"));
                    }
                }
            }
            let bid_id = self
                .repo()
                .insert_bid_in_tx(
                    &mut tx,
                    &AuctionBid {
                        item_id: dto.item_id,
                        user_id,
                        bid_price: dto.bid_price,
                        is_buyout: if is_buyout { 1 } else { 0 },
                        status: bid_status::VALID,
                        ip: ip.to_string(),
                        user_agent: user_agent.chars().take(500).collect(),
                        device_fp: dto.device_fp.clone().unwrap_or_default(),
                        add_time: now,
                        ..Default::default()
                    },
                )
                .await?;
            item.current_price = Some(dto.bid_price);
            item.current_bid_id = Some(bid_id);
            item.bid_count += 1;
            // 延时顺延（普通出价）
            if !is_buyout {
                if let Some(new_end) = settings.extended_end(
                    i64::from(now),
                    i64::from(item.start_time),
                    i64::from(item.end_time),
                    i64::from(item.real_end_time),
                ) {
                    item.real_end_time = new_end as i32;
                    item.extend_count += 1;
                }
            }
            item.update_time = now;
            self.repo().update_item_in_tx(&mut tx, &item).await?;

            // 被超价通知（上一位领先者）
            if let Some(top) = &prev_top {
                notices.push(Notice {
                    user_id: top.user_id,
                    title: "您的出价已被超越".into(),
                    content: format!("拍品「{}」的当前价已升至 ¥{}，您已不是最高出价者。", item.title, dto.bid_price),
                });
            }

            result = BidResultVO {
                item_id: item.item_id,
                current_price: dto.bid_price,
                is_leader: true,
                is_buyout,
                status: item.status,
                real_end_time: item.real_end_time,
                remain_seconds: i64::from(item.real_end_time - now).max(0),
                order_id: None,
            };

            if is_buyout {
                // 一口价：同一事务内立即结拍成交（order_id 提交后回读）
                let closed = self.close_item_in_tx(&mut tx, &item, &settings, now, &mut notices).await?;
                if closed {
                    result.status = item_status::SOLD;
                    result.remain_seconds = 0;
                }
            }
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        }
        if result.is_buyout {
            // 提交后回读成交订单号
            if let Some(item) = self.repo().find_item(dto.item_id).await? {
                result.order_id = item.order_id;
                result.status = item.status;
            }
        }
        self.send_notices(notices).await;
        Ok(result)
    }

    // ————————————————————— 前台：我的拍卖/提醒 —————————————————————

    pub async fn my_joins(&self, user_id: i32) -> Result<Vec<AuctionMyJoinVO>> {
        let now = Self::now();
        let mut item_ids: Vec<i32> = self.repo().find_user_joined_item_ids(user_id).await?;
        let (_, deposits) = self
            .repo()
            .paginate_deposits(None, Some(user_id), None, &PageQuery { page: 1, page_size: 200 })
            .await?;
        for d in &deposits {
            if !item_ids.contains(&d.item_id) {
                item_ids.push(d.item_id);
            }
        }
        let mut items = Vec::new();
        for id in &item_ids {
            if let Some(item) = self.repo().find_item(*id).await? {
                items.push(item);
            }
        }
        items.sort_by_key(|i| std::cmp::Reverse(i.item_id));
        let products = self.load_products(&items.iter().map(|i| i.product_id).collect::<Vec<_>>()).await?;
        let winners = self.winner_names(&items).await?;
        let mut out = Vec::with_capacity(items.len());
        for item in &items {
            let my_highest = self.repo().find_user_max_bid(item.item_id, user_id).await?.unwrap_or_default();
            let deposit = self.repo().find_deposit(item.item_id, user_id).await?;
            let (my_status, my_status_name) = if item.status == item_status::SOLD && item.winner_user_id == Some(user_id) {
                ("won", "已中标")
            } else if item.is_terminal() {
                ("lost", "未中标")
            } else if item.current_price.is_some() && my_highest > Decimal::ZERO && item.current_price == Some(my_highest) {
                ("leading", "领先中")
            } else if my_highest > Decimal::ZERO {
                ("outbid", "已被超越")
            } else {
                ("joined", "参拍中")
            };
            let pay_deadline = if my_status == "won" {
                match item.order_id {
                    Some(oid) => self.order_pay_deadline(oid).await.ok().flatten(),
                    None => None,
                }
            } else {
                None
            };
            out.push(AuctionMyJoinVO {
                item: self.item_vo(item, &products, &winners, now),
                my_highest_bid: my_highest,
                my_status: my_status.to_string(),
                my_status_name: my_status_name.to_string(),
                my_deposit_status: deposit.map(|d| d.status).unwrap_or(0),
                pay_deadline,
            });
        }
        Ok(out)
    }

    pub async fn my_deposits(&self, user_id: i32, q: &AuctionDepositListQuery) -> Result<PageResult<AuctionDepositVO>> {
        let page = PageQuery { page: q.page.unwrap_or(1), page_size: q.page_size.unwrap_or(20) };
        let (total, rows) = self.repo().paginate_deposits(q.item_id, Some(user_id), q.status, &page).await?;
        Ok(PageResult::new(total, rows.into_iter().map(deposit_vo).collect(), &page))
    }

    /// 订阅/取消开拍提醒。`on=true` 订阅。
    pub async fn remind(&self, user_id: i32, item_id: i32, on: bool) -> Result<()> {
        let item = self.repo().find_item(item_id).await?.ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
        if item.status == item_status::DRAFT {
            return Err(Error::NotFound("拍品不存在".into()));
        }
        if on {
            self.repo().upsert_remind(item_id, user_id, Self::now()).await
        } else {
            self.repo().delete_remind(item_id, user_id).await
        }
    }

    /// 读订单扩展里的拍卖尾款截止时间。
    async fn order_pay_deadline(&self, order_id: i32) -> Result<Option<i32>> {
        let order = self.repos.trade.order.find_by_id(order_id).await?;
        Ok(order.and_then(|o| parse_pay_deadline(&o.order_extension)))
    }
}

/// 从 `order_extension` JSON 解析 `payDeadline`。
fn parse_pay_deadline(extension: &str) -> Option<i32> {
    let v: Value = serde_json::from_str(extension).ok()?;
    v.get("payDeadline").and_then(Value::as_i64).map(|d| d as i32)
}

impl AuctionService {
    // ————————————————————— 结拍（事务内） —————————————————————

    /// 结拍结算（须在拍品行锁事务内调用）。返回是否真正执行了结拍
    ///（已终态/不在待结状态时返回 false，幂等）。
    async fn close_item_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        item: &AuctionItem,
        settings: &AuctionSettings,
        now: i32,
        notices: &mut Vec<Notice>,
    ) -> Result<bool> {
        if !matches!(item.status, item_status::PREVIEW | item_status::RUNNING) {
            return Ok(false);
        }
        let mut row = item.clone();
        let top = self.repo().find_top_valid_bid_in_tx(tx, item.item_id).await?;
        let sold = match (&top, item.reserve_price) {
            (Some(b), Some(reserve)) => b.bid_price >= reserve,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if !sold {
            // 流拍：全部冻结保证金解冻
            let deposits = self.repo().find_frozen_deposits_in_tx(tx, item.item_id).await?;
            for d in deposits {
                let user_id = d.user_id;
                self.release_deposit_in_tx(tx, &d, &row.title, now, "拍卖流拍，保证金解冻").await?;
                notices.push(Notice {
                    user_id,
                    title: "拍卖流拍".into(),
                    content: format!("拍品「{}」已流拍，您冻结的保证金已全额解冻。", row.title),
                });
            }
            row.status = item_status::UNSOLD;
            row.update_time = now;
            self.repo().update_item_in_tx(tx, &row).await?;
            return Ok(true);
        }

        let top = top.expect("sold implies top bid");
        let winner = top.user_id;
        let final_price = top.bid_price;

        // 扣库存：失败不阻塞成交（平台自营可人工履约），记告警便于人工跟进。
        let sku = (item.sku_id > 0).then_some(item.sku_id);
        match self.repos.catalog.product.deduct_stock_in_tx(tx, item.product_id, sku, 1).await {
            Ok(true) => {}
            Ok(false) => tracing::warn!(item_id = item.item_id, "拍卖成交扣库存未成功（库存不足或商品状态变化）"),
            Err(e) => tracing::warn!(item_id = item.item_id, error = %e, "拍卖成交扣库存失败"),
        }

        let product = self.repos.catalog.product.find_by_id(item.product_id).await?;
        let (product_name, product_sn, pic_thumb) = match &product {
            Some(p) => (p.product_name.clone(), p.product_sn.clone(), p.pic_thumb.clone()),
            None => (item.title.clone(), String::new(), String::new()),
        };

        // 中标人保证金：抵扣优先，剩余解冻
        let winner_deposit = self.repo().find_deposit_in_tx(tx, item.item_id, winner).await?;
        let deposit_amount = winner_deposit.as_ref().map(|d| d.amount).unwrap_or_default();
        let total = final_price; // 一期不加运费，见 REPORT
        let offset = deposit_amount.min(total);
        let unpaid = total - offset;
        let pay_deadline = now + (settings.final_pay_hours.clamp(1, 720) * 3600) as i32;

        // 中标人默认收货地址快照
        let address: Option<UserAddress> = sqlx::query_as(
            "SELECT * FROM user_address WHERE user_id = $1 AND (is_system = 0 OR is_system IS NULL)
             ORDER BY is_selected DESC, is_default DESC, address_id ASC LIMIT 1",
        )
        .bind(winner)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let (consignee, consignee_address, region_ids, region_names, mobile) = match &address {
            Some(a) => (
                a.consignee.clone(),
                a.address.clone(),
                a.region_ids.clone(),
                a.region_names.clone(),
                a.mobile.clone(),
            ),
            None => (String::new(), String::new(), String::new(), String::new(), String::new()),
        };

        let order_sn = utils::generate_order_sn();
        let paid_fully = unpaid <= Decimal::ZERO;
        let order = Order {
            order_sn: order_sn.clone(),
            user_id: winner,
            shop_id: item.shop_id,
            order_status: if paid_fully { 1 } else { 0 },
            pay_status: if paid_fully { 2 } else { 1 },
            pay_time: if paid_fully { now } else { 0 },
            total_amount: total,
            product_amount: total,
            paid_amount: offset,
            unpaid_amount: unpaid,
            balance: offset,
            consignee,
            address: consignee_address,
            region_ids,
            region_names,
            mobile,
            add_time: now,
            order_type: Some(AUCTION_ORDER_TYPE),
            order_source: "auction".to_string(),
            order_extension: json!({
                "auctionItemId": item.item_id,
                "depositAmount": offset.to_string(),
                "payDeadline": pay_deadline,
            })
            .to_string(),
            ..Default::default()
        };
        let order_item = OrderItem {
            order_sn,
            user_id: winner,
            price: final_price,
            quantity: 1,
            product_id: item.product_id,
            product_name,
            product_sn,
            pic_thumb,
            sku_id: item.sku_id,
            product_type: 1,
            shop_id: item.shop_id,
            origin_price: Some(final_price),
            ..Default::default()
        };
        let order_id = self
            .repos
            .trade
            .order
            .insert_order_in_tx(tx, &order, &[order_item])
            .await?;

        if let Some(mut d) = winner_deposit {
            if offset > Decimal::ZERO {
                self.wallet_move_in_tx(
                    tx,
                    winner,
                    Decimal::ZERO,
                    -offset,
                    2,
                    &format!("拍卖保证金抵扣货款[拍品：{}]", item.title),
                    now,
                )
                .await?;
            }
            let excess = d.amount - offset;
            if excess > Decimal::ZERO {
                self.wallet_move_in_tx(
                    tx,
                    winner,
                    excess,
                    -excess,
                    1,
                    &format!("拍卖保证金解冻（抵扣后剩余）[拍品：{}]", item.title),
                    now,
                )
                .await?;
                d.remark = format!("抵扣 ¥{offset}，剩余 ¥{excess} 已解冻");
            }
            d.status = deposit_status::OFFSET;
            d.order_id = Some(order_id);
            d.handle_time = now;
            self.repo().update_deposit_in_tx(tx, &d).await?;
        }

        // 出局者解冻（中标人保证金状态已变更，不在冻结清单内）
        let deposits = self.repo().find_frozen_deposits_in_tx(tx, item.item_id).await?;
        for d in deposits {
            let user_id = d.user_id;
            self.release_deposit_in_tx(tx, &d, &row.title, now, "拍卖结束未中标，保证金解冻").await?;
            notices.push(Notice {
                user_id,
                title: "未中标，保证金已解冻".into(),
                content: format!("拍品「{}」已成交，您冻结的保证金已全额解冻。", row.title),
            });
        }

        row.status = item_status::SOLD;
        row.winner_user_id = Some(winner);
        row.final_price = Some(final_price);
        row.order_id = Some(order_id);
        row.update_time = now;
        self.repo().update_item_in_tx(tx, &row).await?;

        notices.push(Notice {
            user_id: winner,
            title: "恭喜中标".into(),
            content: format!(
                "您已中标拍品「{}」，中标价 ¥{final_price}，保证金抵扣 ¥{offset}，尾款 ¥{unpaid}，请于 {} 前完成支付，逾期将按规则扣罚保证金。",
                row.title,
                fmt_time(pay_deadline)
            ),
        });
        Ok(true)
    }

    /// 结拍入口（自带事务）：行锁后调 close_item_in_tx。
    pub async fn close_item(&self, item_id: i32, now: i32) -> Result<bool> {
        let settings = self.get_settings().await?;
        let mut notices = Vec::new();
        let closed = {
            let mut tx = self.repo().begin().await?;
            let item = match self.repo().find_item_for_update(&mut tx, item_id).await? {
                Some(i) => i,
                None => return Ok(false),
            };
            if item.is_terminal() || now < item.real_end_time {
                return Ok(false);
            }
            let closed = self.close_item_in_tx(&mut tx, &item, &settings, now, &mut notices).await?;
            if closed {
                tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
            }
            closed
        };
        self.send_notices(notices).await;
        Ok(closed)
    }

    // ————————————————————— 定时任务扫描 —————————————————————

    /// 开拍扫描：预告到点 → 竞价中；顺带发送该拍品未通知的开拍提醒。
    pub async fn run_start_scan(&self, now: i32) -> Result<usize> {
        let due = self.repo().find_due_start(now, 200).await?;
        let mut promoted = 0usize;
        let mut notices = Vec::new();
        for stub in &due {
            let mut tx = self.repo().begin().await?;
            if let Some(mut item) = self.repo().find_item_for_update(&mut tx, stub.item_id).await? {
                if item.status == item_status::PREVIEW && now >= item.start_time {
                    item.status = item_status::RUNNING;
                    item.update_time = now;
                    self.repo().update_item_in_tx(&mut tx, &item).await?;
                    tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
                    promoted += 1;
                }
            }
            // 开拍提醒：该拍品订阅者逐人通知一次
            let user_ids = self.repo().find_unnotified_remind_users(stub.item_id).await?;
            if !user_ids.is_empty() {
                for user_id in user_ids {
                    notices.push(Notice {
                        user_id,
                        title: "拍卖开拍提醒".into(),
                        content: format!("您订阅的拍品「{}」已开拍，起拍价 ¥{}，请及时参拍。", stub.title, stub.start_price),
                    });
                }
                self.repo().mark_reminds_notified(stub.item_id).await?;
            }
        }
        self.send_notices(notices).await;
        Ok(promoted)
    }

    /// 结拍扫描：到 real_end_time 的预告/竞价中拍品逐件结拍。
    pub async fn run_close_scan(&self, now: i32) -> Result<usize> {
        let due = self.repo().find_due_close(now, 200).await?;
        let mut closed = 0usize;
        for stub in &due {
            match self.close_item(stub.item_id, now).await {
                Ok(true) => closed += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(item_id = stub.item_id, error = %e, "拍卖结拍失败，待下轮重试"),
            }
        }
        Ok(closed)
    }

    /// 尾款逾期扫描：拍卖订单待付款且超过 payDeadline → 违约处理。
    pub async fn run_overdue_scan(&self, now: i32) -> Result<usize> {
        let settings = self.get_settings().await?;
        let candidates: Vec<Order> = sqlx::query_as(
            "SELECT * FROM \"order\" WHERE order_type = $1 AND order_status = 0 AND is_del = 0
             ORDER BY order_id ASC LIMIT 100",
        )
        .bind(AUCTION_ORDER_TYPE)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let mut handled = 0usize;
        for order in candidates {
            let deadline = parse_pay_deadline(&order.order_extension).unwrap_or(0);
            if deadline <= 0 || deadline >= now {
                continue;
            }
            match self.handle_overdue(&order, &settings, now).await {
                Ok(true) => handled += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(order_id = order.order_id, error = %e, "拍卖违约处理失败，待下轮重试"),
            }
        }
        Ok(handled)
    }

    /// 单笔违约处理（事务内）：关单 → 扣罚 → 违约记录 → 封禁判定 → 回滚库存 → 拍品按设置处置。
    async fn handle_overdue(&self, order: &Order, settings: &AuctionSettings, now: i32) -> Result<bool> {
        let ext: Value = serde_json::from_str(&order.order_extension).unwrap_or(Value::Null);
        let item_id = ext.get("auctionItemId").and_then(Value::as_i64).unwrap_or(0) as i32;
        if item_id <= 0 {
            return Ok(false);
        }
        let mut notices = Vec::new();
        {
            let mut tx = self.repo().begin().await?;
            // 订单条件关单（防并发重复处理）：仍为待付款才继续
            let affected = sqlx::query("UPDATE \"order\" SET order_status = 3 WHERE order_id = $1 AND order_status = 0")
                .bind(order.order_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?
                .rows_affected();
            if affected == 0 {
                return Ok(false);
            }
            let item = self
                .repo()
                .find_item_for_update(&mut tx, item_id)
                .await?
                .ok_or_else(|| Error::NotFound("拍品不存在".into()))?;
            let winner = order.user_id;
            let final_price = item.final_price.unwrap_or(order.total_amount);
            let deposit = self.repo().find_deposit_by_order_in_tx(&mut tx, order.order_id).await?;
            let (deposit_amount, penalty) = match &deposit {
                Some(d) => (d.amount, settings.penalty_for(d.amount, final_price)),
                None => (Decimal::ZERO, Decimal::ZERO),
            };
            // 可退 = 已抵扣金额 − 扣罚（保证金已抵扣部分在订单 paid_amount 中）
            let refund = (order.paid_amount - penalty).max(Decimal::ZERO);
            if refund > Decimal::ZERO {
                self.wallet_move_in_tx(
                    &mut tx,
                    winner,
                    refund,
                    Decimal::ZERO,
                    1,
                    &format!("拍卖违约，保证金退回（已扣罚 ¥{penalty}）[拍品：{}]", item.title),
                    now,
                )
                .await?;
            }
            if let Some(mut d) = deposit {
                d.penalty_amount = penalty;
                d.status = if penalty <= Decimal::ZERO {
                    deposit_status::RELEASED
                } else if refund > Decimal::ZERO {
                    deposit_status::PARTIAL_PENALIZED
                } else {
                    deposit_status::PENALIZED
                };
                d.handle_time = now;
                d.remark = format!("尾款逾期违约，扣罚 ¥{penalty}");
                self.repo().update_deposit_in_tx(&mut tx, &d).await?;
            }
            // 违约记录
            self.repo()
                .insert_breach_in_tx(
                    &mut tx,
                    &AuctionBreach {
                        user_id: winner,
                        item_id,
                        order_id: order.order_id,
                        deposit_amount,
                        penalty_amount: penalty,
                        breach_time: now,
                        ..Default::default()
                    },
                )
                .await?;
            // 封禁判定（近 12 个月累计，含本次）
            let year_ago = now - 365 * 86_400;
            let count = self.repo().count_breaches_since_in_tx(&mut tx, winner, year_ago).await?;
            if !matches!(settings.ban_action(count), BanAction::None) {
                let (ban_type, end_time) = match settings.ban_action(count) {
                    BanAction::Temp(days) => (1, now + (days * 86_400) as i32),
                    _ => (2, 0),
                };
                self.repo()
                    .insert_ban(&AuctionBan {
                        user_id: winner,
                        ban_type,
                        start_time: now,
                        end_time,
                        reason: format!("拍卖违约累计 {count} 次，自动封禁"),
                        status: 1,
                        operator_id: 0,
                        add_time: now,
                        ..Default::default()
                    })
                    .await?;
            }
            // 回滚库存
            let sku = (item.sku_id > 0).then_some(item.sku_id);
            if let Err(e) = self.repos.catalog.product.restore_stock_in_tx(&mut tx, item.product_id, sku, 1).await {
                tracing::warn!(item_id, error = %e, "拍卖违约回滚库存失败");
            }
            // 拍品处置：默认「重新上拍」——回到草稿等待运营重新排期；roll_down 顺延预留二期
            let mut row = item;
            row.status = item_status::DRAFT;
            row.winner_user_id = None;
            row.final_price = None;
            row.order_id = None;
            row.current_price = None;
            row.current_bid_id = None;
            row.bid_count = 0;
            row.extend_count = 0;
            row.real_end_time = row.end_time;
            row.update_time = now;
            self.repo().update_item_in_tx(&mut tx, &row).await?;
            self.repo().void_all_bids_in_tx(&mut tx, item_id).await?;

            notices.push(Notice {
                user_id: winner,
                title: "拍卖订单违约关闭".into(),
                content: format!(
                    "您中标的拍品「{}」因尾款逾期未付已关闭，扣罚保证金 ¥{penalty}。如有异议请联系平台客服申诉。",
                    row.title
                ),
            });
            tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;
        }
        self.send_notices(notices).await;
        Ok(true)
    }
}
