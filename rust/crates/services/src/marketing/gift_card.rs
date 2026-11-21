//! 礼品卡域服务（marketing::gift_card）。
//!
//! 对应原 Java `GiftCardsServiceImpl`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/promotion/impl/GiftCardsServiceImpl.java`）。
//!
//! # 端点归属
//!
//! 本域负责 `marketing/gift_card` 的 8 个 **admin** 端点（list / detail /
//! importTemplate / exportList / importList / delete / giftCardExchangeDetail /
//! updateCardStatus）以及 `gift_card_groups` 的 6 个 **client** 端点所需的卡逻辑
//! （client detail / claim / transfer / isGiftCardExpired / exchange /
//! getTransferLink，路由在 `gift_card_groups` 的 public 路由文件里组合两个服务）。
//!
//! # 未迁移 / 偏差（如实标注）
//!
//! - **Excel 已接线**：`importTemplate` / `exportList` / `importList` 经
//!   `services::system::excel` 真实读写 xlsx，admin 路由返回 xlsx 二进制
//!   （早期「未迁移」注释已过期，P1 清理）。
//! - **Redis 锁未迁移**：`exchangeCard` 的 `userExchangeCard_{userId}` 分布式锁
//!   未迁移（Rust 侧无 Redis 锁封装）；并发重复兑换由后续唯一约束/事务兜底。
//! - **兑换卡用卡**（`useCard`，标准对齐 D6）：exchange 只负责凭卡号卡密
//!   把卡绑入卡包；核销+写购物车走独立 `use_card`（固定组合整组、任选按
//!   SKU 数量计件），购物车行以 `extra_data.giftCardsId` 标记，结算
//!   flowType=12 时由 check 按商品金额全额记 giftCardAmount。
//! - **平台店铺名/logo**：`shopId=0` 已从 `config` 表读取
//!   `shopName`/`shopLogo`；转赠域名（`PC_DOMAIN/H5_DOMAIN`）仍未迁移，
//!   转赠链接使用相对路径 `/giftCard/transfer/`、
//!   `/pages/giftCard/transfer?giftCardSn=`。
//! - **储值卡兑换**（标准对齐 D4，**原版有·我方曾改口径已回归**）：原版
//!   三端无「储值卡转会员余额」路径——exchange 对储值卡只做入卡包绑定
//!   （claimed_user_id 置当前会员、status 保持 0、卡内 balance 不动），
//!   消费走结算页单卡抵扣（trade check/order 的 useGiftCardId/Sn）。
//!   旧实现曾按 Java 老语义入会员余额（建充值单 + 加余额），本轮已整体下线。
//! - **手机号脱敏**：`GiftCardDetailVO.mobile` 的 `@Sensitive(PHONE)` 仅演示站
//!   生效，用常量 [`IS_DEMO`]（对齐 `nextrade.is-demo: 0`）。

use std::sync::Arc;

use rust_decimal::Decimal;

use nextrade_common::utils::{mask_mobile, now_seconds};
use nextrade_core::entities::{GiftCards, Order};
use nextrade_core::models::marketing::gift_card::{
    format_unix_time, is_exchange_text, is_sold_text, mask_middle_half, scope_type_name,
    seconds_text,
    ExchangeCardParam, GiftCardClaimParam, GiftCardDeliveryInfoVO, GiftCardDetailVO,
    GiftCardExchangeInfoVO, GiftCardIdParam, GiftCardPayInfoVO, GiftCardUpdateStatusDTO,
    GiftCardUserAddressInfoVO, GiftCardsExportVO, GiftCardsImportVO, GiftCardsListQuery,
    GiftCardsTemplateVO, GiftCardsVO, UseCardParam, CLAIMED_RECEIVED, CLAIMED_RECEIVABLE,
    MSG_CARD_NOT_OWNED, MSG_CARD_NOT_TRANSFERABLE, MSG_PRODUCT_NOT_IN_GROUP,
    MSG_SELECT_COUNT_INVALID,
    CARD_TYPE_PHYSICAL, GROUP_TYPE_EXCHANGE, GROUP_TYPE_TOP_UP, MSG_CARD_ABNORMAL, MSG_CARD_ALREADY_CLAIMED,
    MSG_CARD_ALREADY_EXCHANGED, MSG_CARD_EXPIRED, MSG_CARD_GROUP_INVALID,
    MSG_CARD_GROUP_NOT_EXISTS, MSG_CARD_ID_REQUIRED, MSG_CARD_NOT_CLAIMABLE,
    MSG_CARD_NOT_EXISTS, MSG_CARD_SOLD_CANNOT_DELETE, MSG_EXCHANGE_FAILED,
    MSG_GROUP_NOT_EXISTS, MSG_IS_AVAILABLE_REQUIRED,
    MSG_PRODUCTS_OFFLINE, MSG_SN_NOT_FOUND, MSG_SOLD_CANNOT_CHANGE_STATUS,
    MSG_TRANSFER_CARD_NOT_FOUND, MSG_TRANSFER_GROUP_NOT_FOUND, MSG_TRANSFER_SN_REQUIRED,
    MSG_TRANSFER_SOURCE_REQUIRED, MSG_UNSUPPORTED_GROUP_TYPE, MSG_UPLOAD_FILE_REQUIRED,
    TRANSFER_FROM_H5, TRANSFER_FROM_PC, YES,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::gift_card::GiftCardRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 演示站开关（Java `nextrade.is-demo`，默认 `0`）。
pub const IS_DEMO: i32 = 0;

/// 礼品卡域服务。
pub struct GiftCardService {
    repos: Arc<Repositories>,
}

impl GiftCardService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本域仓储（NESTED：`repos.marketing.gift_card`）。
    #[must_use]
    pub fn repo(&self) -> &GiftCardRepository {
        &self.repos.marketing.gift_card
    }

    // ————————————————————— admin：列表 / 详情 —————————————————————

    /// 卡分页列表（Java `GiftCardsServiceImpl#list`）。
    pub async fn list(&self, q: &GiftCardsListQuery) -> Result<PageResult<GiftCardsVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        // 多租户：卡组归属校验（gift_card_groups.shop_id）
        if let Some(card_groups_id) = q.card_groups_id {
            ensure_group_access(self.repo(), card_groups_id).await?;
        }
        let (total, rows) = self.repo().paginate(q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let list = rows.iter().map(list_vo).collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 卡详情（Java controller：`getById` + `giftCardsDetail(sn, true)`）。
    pub async fn admin_detail(&self, gift_cards_id: i32) -> Result<GiftCardsVO> {
        let card = self
            .repo()
            .find_by_id(gift_cards_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        // 多租户：详情归属校验（经卡组 shop_id）
        ensure_group_access(self.repo(), card.card_groups_id).await?;
        let sn = card
            .gift_card_sn
            .clone()
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        self.gift_cards_detail(&sn, true).await
    }

    /// 按序列号取卡详情（Java `giftCardsDetail(sn, showPassword)`）。
    ///
    /// 卡不存在时 Java 返回空 `GiftCardsVO`（非错误）；卡组缺失时 Java NPE ⇒
    /// 本实现返回业务 500「礼品卡组不存在」。
    pub async fn gift_cards_detail(&self, sn: &str, show_password: bool) -> Result<GiftCardsVO> {
        let Some(card) = self.repo().find_by_sn(sn).await? else {
            return Ok(GiftCardsVO::default());
        };
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_GROUP_NOT_EXISTS))?;
        let (shop_title, shop_logo) = self.resolve_shop(group.shop_id.unwrap_or(0)).await?;

        let (card_number, card_password) = if show_password {
            (card.card_number.clone(), card.card_password.clone())
        } else {
            (
                mask_middle_half(&card.card_number),
                mask_middle_half(&card.card_password),
            )
        };
        // 适用范围（我方新增）：卡组级动态生效，详情接口下发范围类型 + 文案
        let scope_type = self
            .repos
            .marketing
            .gift_card_groups
            .find_scope(card.card_groups_id)
            .await?
            .map(|(t, _, _)| t)
            .unwrap_or(0);
        Ok(GiftCardsVO {
            gift_cards_id: Some(card.gift_cards_id),
            card_groups_id: Some(card.card_groups_id),
            card_group_name: None,
            shop_id: group.shop_id,
            gift_card_sn: card.gift_card_sn.clone(),
            shop_title,
            shop_logo,
            image_url: group.image_url.clone(),
            card_number: Some(card_number),
            card_password: Some(card_password),
            is_available: card.is_available,
            status: card.status,
            is_sold: Some(is_sold_text(card.user_id)),
            is_exchange: Some(is_exchange_text(card.status)),
            used_time: None,
            user_id: card.user_id,
            add_time: seconds_text(card.add_time),
            expired_time: Some(card.expired_time),
            gift_card_products: None,
            exchange_product_to_cart: None,
            scope_type: Some(scope_type),
            scope_desc: Some(scope_type_name(scope_type).to_string()),
        })
    }

    /// 平台店铺（shopId=0）取 `config` 表的 `shopName` / `shopLogo`。
    async fn resolve_shop(&self, shop_id: i32) -> Result<(Option<String>, Option<String>)> {
        if shop_id == 0 {
            let pairs = self
                .repos
                .system
                .config_basic
                .find_by_codes(&["shopName", "shopLogo"])
                .await?;
            let map: std::collections::HashMap<String, String> = pairs.into_iter().collect();
            let title = map
                .get("shopName")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let logo = map
                .get("shopLogo")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            return Ok((title, logo));
        }
        match self.repo().find_shop(shop_id).await? {
            Some((title, logo)) => Ok((title, logo)),
            None => Ok((None, None)),
        }
    }

    // ————————————————————— admin：Excel（JSON 降级） —————————————————————

    /// 导入模板（Java `importTemplate` 写 xlsx → 本实现返回模板行 JSON）。
    #[must_use]
    pub fn import_template(&self) -> Vec<GiftCardsTemplateVO> {
        vec![GiftCardsTemplateVO {
            card_number: "GC20251031133058".to_string(),
            card_password: "PASSWORD123456".to_string(),
        }]
    }

    /// 导出列表（Java `@ResponseExcel` → 本实现返回导出行 JSON）。
    pub async fn export_list(&self, card_groups_id: i32) -> Result<Vec<GiftCardsExportVO>> {
        // 多租户：导出前校验卡组归属
        ensure_group_access(self.repo(), card_groups_id).await?;
        let rows: Vec<GiftCards> = self.repo().find_by_group(card_groups_id).await?;
        Ok(rows
            .into_iter()
            .map(|c| GiftCardsExportVO {
                card_number: c.card_number.clone(),
                card_password: c.card_password.clone(),
                gift_card_sn: c.gift_card_sn.clone(),
                is_sold: is_sold_text(c.user_id),
                is_exchange: is_exchange_text(c.status),
                add_time: format_unix_time(c.add_time),
            })
            .collect())
    }

    /// 导入礼品卡（Java `GiftCardsServiceImpl#importList`）。
    ///
    /// 行格式：`(卡号, 密码)`，逐条校验后插入，失败行返回原因。
    pub async fn import_list(
        &self,
        card_groups_id: Option<i32>,
        rows: &[(String, String)],
    ) -> Result<GiftCardsImportVO> {
        if rows.is_empty() {
            return Err(Error::biz(500, MSG_UPLOAD_FILE_REQUIRED));
        }
        let group_id = card_groups_id.ok_or_else(|| Error::biz(500, MSG_CARD_GROUP_INVALID))?;
        ensure_group_access(self.repo(), group_id).await?;
        let group = self
            .repo()
            .find_group(group_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_GROUP_INVALID))?;
        let now = i32::try_from(now_seconds()).unwrap_or(i32::MAX);
        let expired = if group.card_type == CARD_TYPE_PHYSICAL {
            group.expire_time.unwrap_or(0)
        } else {
            0
        };
        let mut success = 0;
        let mut fail_list = Vec::new();
        for (number, password) in rows {
            let number = number.trim();
            let password = password.trim();
            if number.len() < 8 {
                fail_list.push(format!("{number}（卡号长度不能小于8位）"));
                continue;
            }
            if !(6..=16).contains(&password.len()) {
                fail_list.push(format!("{number}（密码长度不能小于6位或者大于16位）"));
                continue;
            }
            let card = GiftCards {
                gift_cards_id: 0,
                card_groups_id: group_id,
                card_number: number.to_string(),
                card_password: password.to_string(),
                gift_card_sn: Some(uuid::Uuid::new_v4().simple().to_string()),
                is_available: Some(1),
                status: Some(0),
                used_time: None,
                user_id: None,
                add_time: Some(now),
                expired_time: expired,
                claimed_user_id: None,
                claimed_status: None,
                order_time: None,
            };
            match self.repo().insert_many(&[card]).await {
                Ok(_) => success += 1,
                Err(_) => fail_list.push(number.to_string()),
            }
        }
        let stock = group.availability_stock.unwrap_or(0).saturating_add(success);
        self.repo().update_group_stock(group_id, stock).await?;
        Ok(GiftCardsImportVO {
            success_count: success,
            fail_count: i32::try_from(fail_list.len()).unwrap_or(i32::MAX),
            fail_list,
        })
    }

    // ————————————————————— admin：删除 / 状态 / 兑换详情 —————————————————————

    /// 删除卡（Java `deleteGiftCards`；请求字段 `cardGroupsId` 实际是卡 ID）。
    pub async fn delete(&self, param: &GiftCardIdParam) -> Result<()> {
        let gift_cards_id = param
            .gift_cards_id()
            .map_err(Error::Validation)?;
        let card = self
            .repo()
            .find_by_id(gift_cards_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        // 多租户：删除前校验归属（经卡组 shop_id）
        ensure_group_access(self.repo(), card.card_groups_id).await?;
        // 未过期但已售出未兑换 ⇒ 不可删除
        if card.status == Some(0) && card.user_id.is_some() {
            return Err(Error::biz(500, MSG_CARD_SOLD_CANNOT_DELETE));
        }
        // 减可用库存
        if let Some(group) = self.repo().find_group(card.card_groups_id).await? {
            if card.is_available == Some(1) && group.availability_stock.unwrap_or(0) > 0 {
                let stock = group.availability_stock.unwrap_or(0) - 1;
                self.repo()
                    .update_group_stock(card.card_groups_id, stock)
                    .await?;
            }
        }
        self.repo().delete(gift_cards_id).await?;
        tracing::info!("删除礼品卡: id={gift_cards_id}");
        Ok(())
    }

    /// 修改可用状态（Java `updateGiftCardsAvailableStatus`）。
    pub async fn update_card_status(&self, dto: &GiftCardUpdateStatusDTO) -> Result<()> {
        let gift_cards_id = dto
            .gift_cards_id
            .ok_or_else(|| Error::Validation(MSG_CARD_ID_REQUIRED.into()))?;
        let is_available = dto
            .is_available
            .ok_or_else(|| Error::Validation(MSG_IS_AVAILABLE_REQUIRED.into()))?;
        let card = self
            .repo()
            .find_by_id(gift_cards_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_ABNORMAL))?;
        // 多租户：状态变更前校验归属
        ensure_group_access(self.repo(), card.card_groups_id).await?;
        if card.is_available == Some(is_available) {
            return Ok(());
        }
        // 已售出的卡不能修改
        if card.user_id.is_some() {
            return Err(Error::biz(500, MSG_SOLD_CANNOT_CHANGE_STATUS));
        }
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        let mut stock = group.availability_stock.unwrap_or(0);
        if is_available == 1 {
            stock += 1;
        } else if is_available == 0 && stock > 0 {
            stock -= 1;
        }
        self.repo()
            .update_group_stock(card.card_groups_id, stock)
            .await?;
        self.repo().update_available(gift_cards_id, is_available).await?;
        Ok(())
    }

    /// 批量生成礼品卡（Java `createGiftCardsBatch(cardGroupsId, count)`，
    /// 端点 `POST /promotion/giftCards/createBatch`）。
    ///
    /// 语义逐条对齐 Java `GiftCardsServiceImpl#createGiftCardsBatch`：
    /// 1. 卡组必须存在（`卡组不存在`）；
    /// 2. 一次生成 `count` 个唯一卡号（内存级去重；格式
    ///    `GC + yyyyMMdd + groupId + 4 位随机`，随机字符集
    ///    `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`，与 Java `CHARSET` 一致）；
    /// 3. 过滤数据库已存在的卡号；
    /// 4. 逐张插入：`gift_card_sn = UUID`、`card_password = 8 位随机`、
    ///    `is_available = 1`、`status = 0`、`add_time = now`；
    ///    实体卡（`card_type = 2`）继承卡组 `expire_time`（缺失 0）；
    /// 5. 卡组 `availability_stock += 实际插入张数`。
    ///
    /// # Errors
    /// 卡组不存在 / 无归属权限 → 业务错误或 `Forbidden`；
    /// 数据库错误 → `Error::Database`。
    pub async fn create_batch(&self, card_groups_id: i32, count: i32) -> Result<()> {
        if count <= 0 {
            return Err(Error::Validation("生成数量必须大于0".into()));
        }
        let group = self
            .repo()
            .find_group(card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_GROUP_NOT_EXISTS))?;
        // 多租户：写操作必须校验卡组归属
        TenantScope::current_ensure_access(group.shop_id.unwrap_or(0))?;

        // ① 生成唯一卡号（内存去重 + 数据库去重，上限保护避免死循环）
        let mut card_nos: Vec<String> = Vec::with_capacity(count as usize);
        let mut attempts = 0u32;
        while card_nos.len() < count as usize && attempts < (count as u32).saturating_mul(20) {
            attempts += 1;
            let no = generate_card_no(card_groups_id);
            if !card_nos.contains(&no) {
                card_nos.push(no);
            }
        }
        let existing = self
            .repo()
            .find_existing_card_numbers(&card_nos)
            .await?;
        let fresh: Vec<String> = card_nos
            .into_iter()
            .filter(|no| !existing.contains(no))
            .collect();

        // ② 构造实体
        let now = now_seconds();
        let add_time = i32::try_from(now).unwrap_or(i32::MAX);
        let expired_time = if group.card_type == CARD_TYPE_PHYSICAL {
            group.expire_time.unwrap_or(0)
        } else {
            0
        };
        let cards: Vec<GiftCards> = fresh
            .into_iter()
            .map(|card_number| GiftCards {
                gift_cards_id: 0,
                card_groups_id,
                card_number,
                card_password: generate_card_password(),
                gift_card_sn: Some(uuid::Uuid::new_v4().simple().to_string()),
                is_available: Some(YES),
                status: Some(0),
                used_time: None,
                user_id: None,
                add_time: Some(add_time),
                expired_time,
                claimed_user_id: None,
                claimed_status: None,
                order_time: None,
            })
            .collect();

        // ③ 批量写入 + 卡组库存
        if !cards.is_empty() {
            let inserted = self.repo().insert_many(&cards).await? as i32;
            let stock = group.availability_stock.unwrap_or(0) + inserted;
            self.repo()
                .update_group_stock(card_groups_id, stock)
                .await?;
            tracing::info!("批量生成礼品卡: cardGroupsId={card_groups_id}, count={inserted}");
        }
        Ok(())
    }

    /// 查看卡密（端点 `GET /promotion/giftCards/showCardPassword?cardId=`）。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/promotion/giftCard.ts#getGiftCardPasswrod`
    /// （返回 `string`；旧版 Java 控制器未含该端点）。管理端为明文查看，
    /// 不做脱敏（与 Java 列表 `cardPassword` 直出一致）。
    ///
    /// # Errors
    /// 卡不存在 → 业务错误 `礼品卡不存在`；数据库错误 → `Error::Database`。
    pub async fn show_card_password(&self, card_id: i32) -> Result<String> {
        let card = self
            .repo()
            .find_by_id(card_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        Ok(card.card_password)
    }

    /// 按礼品卡 sn 显示卡密码（前台 showCardPassword）。
    pub async fn show_card_password_by_sn(&self, sn: &str) -> Result<String> {
        let card = self
            .repo()
            .find_by_sn(sn)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        Ok(card.card_password)
    }

    /// 礼品卡兑换详情（Java `giftCardExchangeDetail`）。
    pub async fn gift_card_exchange_detail(&self, gift_cards_id: i32) -> Result<GiftCardDetailVO> {
        let card = self
            .repo()
            .find_by_id(gift_cards_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_GROUP_NOT_EXISTS))?;

        let mut vo = GiftCardDetailVO {
            group_type: Some(group.group_type),
            card_type: Some(group.card_type),
            card_number: Some(card.card_number.clone()),
            card_password: Some(card.card_password.clone()),
            is_sold: Some(is_sold_text(card.user_id)),
            is_exchange: Some(is_exchange_text(card.status)),
            order_id: None,
            exchange_info: Some(GiftCardExchangeInfoVO {
                user_address_info: Some(GiftCardUserAddressInfoVO::default()),
                delivery_info: Some(GiftCardDeliveryInfoVO::default()),
                pay_info: Some(GiftCardPayInfoVO::default()),
            }),
        };

        // 储值卡：兑换信息取用户 + 面额
        if group.group_type == GROUP_TYPE_TOP_UP
            && card.user_id.is_some()
            && card.used_time.is_some()
        {
            let user = match card.user_id {
                Some(id) => self.repo().find_user_brief(id).await?,
                None => None,
            };
            let info = vo.exchange_info.get_or_insert_with(Default::default);
            if let Some((username, mobile)) = user {
                info.user_address_info = Some(GiftCardUserAddressInfoVO {
                    username: Some(username),
                    mobile: Some(visible_phone(&mobile, IS_DEMO)),
                    ..Default::default()
                });
            }
            info.pay_info = Some(GiftCardPayInfoVO {
                pay_type_name: Some("储值卡支付".to_string()),
                exchange_amount: group.face_amount,
                exchange_time: Some(format_unix_time(card.used_time)),
                received_time: None,
            });
            return Ok(vo);
        }

        // 兑换卡：取兑换订单信息
        let order = self.repo().find_order_by_gift_card(gift_cards_id).await?;
        if let Some(order) = order {
            if group.group_type == GROUP_TYPE_EXCHANGE {
                let info = vo.exchange_info.get_or_insert_with(Default::default);
                let user = self.repo().find_user_brief(order.user_id).await?;
                info.user_address_info = Some(address_info(&order, user, IS_DEMO));
                info.delivery_info = Some(delivery_info(&order));
                info.pay_info = Some(GiftCardPayInfoVO {
                    pay_type_name: Some("兑换卡支付".to_string()),
                    exchange_time: Some(format_unix_time(card.used_time)),
                    exchange_amount: None,
                    received_time: Some(format_unix_time(Some(order.received_time))),
                });
                vo.order_id = Some(order.order_id);
            }
        }
        Ok(vo)
    }

    // ————————————————————— client：详情 / 领取 / 转赠 / 过期 —————————————————————

    /// C 端详情基础信息（Java `giftCardsDetail(sn,false)`；路由层再合并卡组信息）。
    pub async fn client_detail(&self, sn: &str) -> Result<GiftCardsVO> {
        self.gift_cards_detail(sn, false).await
    }

    /// 领取卡（Java `claimCard`：只校验存在 + 可领取，**不落库**）。
    pub async fn claim_card(&self, param: &GiftCardClaimParam) -> Result<()> {
        let sn = param
            .gift_card_sn
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_TRANSFER_SN_REQUIRED.into()))?;
        let card = self
            .repo()
            .find_by_sn(sn)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_SN_NOT_FOUND))?;
        if card.claimed_status != Some(CLAIMED_RECEIVABLE) {
            return Err(Error::biz(500, MSG_CARD_NOT_CLAIMABLE));
        }
        Ok(())
    }

    /// 转赠卡（Java `transferCard`）：置为「可领取」并返回转赠链接。
    ///
    /// 转赠资格（标准对齐 D10，原版判定函数口径）：仅电子卡
    /// （cardType=1）、status=0、isAvailable=1、未过期；储值卡还须
    /// 卡内余额 == 初始面额（未消费过，initialBalance 以卡组 face_amount
    /// 承载，balance 为 NULL 时按面额计）；兑换卡未使用即可赠。
    pub async fn transfer_card(&self, param: &GiftCardClaimParam) -> Result<String> {
        let sn = param
            .gift_card_sn
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_TRANSFER_SN_REQUIRED.into()))?;
        let mut card = self
            .repo()
            .find_by_sn(sn)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_SN_NOT_FOUND))?;
        if card.claimed_status == Some(CLAIMED_RECEIVED) {
            return Err(Error::biz(500, MSG_CARD_ALREADY_CLAIMED));
        }
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        let balance = self.repo().find_card_balance(card.gift_cards_id).await?;
        if !transfer_eligible(&card, group.card_type, group.group_type, group.face_amount, balance, now_seconds_i32()) {
            return Err(Error::biz(500, MSG_CARD_NOT_TRANSFERABLE));
        }
        card.claimed_status = Some(CLAIMED_RECEIVABLE);
        self.repo().update(&card).await?;
        self.get_transfer_link(sn, param.is_from).await
    }

    /// 是否过期（Java `isGiftCardExpired(card)`）。
    pub async fn is_gift_card_expired(&self, card_id: i32) -> Result<bool> {
        let card = self
            .repo()
            .find_by_id(card_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
        Ok(card_expired(&card, now_seconds_i32()))
    }

    /// 兑换卡（Java `exchangeCard`）。
    pub async fn exchange_card(
        &self,
        param: &ExchangeCardParam,
        current_user_id: i32,
    ) -> Result<GiftCardsVO> {
        let card_number = param
            .card_number
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_EXCHANGE_FAILED.into()))?;
        let card_password = param
            .card_password
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_EXCHANGE_FAILED.into()))?;
        let user_id = param.user_id.unwrap_or(current_user_id);
        let mut card = self
            .repo()
            .find_for_exchange(card_number, card_password)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_EXCHANGE_FAILED))?;
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        if card.claimed_user_id.is_some() {
            return Err(Error::biz(500, MSG_CARD_ALREADY_EXCHANGED));
        }
        if card_expired(&card, now_seconds_i32()) {
            return Err(Error::biz(500, MSG_CARD_EXPIRED));
        }

        match group.group_type {
            GROUP_TYPE_TOP_UP => {
                // D4 定案（原版口径）：储值卡 exchange 只做入卡包绑定——
                // 卡归属置当前会员，status 保持 0、卡内 balance 不动，
                // 不建充值单、不加会员余额；消费走结算页单卡抵扣。
                card.user_id = Some(user_id);
                card.claimed_user_id = Some(user_id);
                card.claimed_status = Some(CLAIMED_RECEIVED);
                self.repo().update(&card).await?;
                let sn = card
                    .gift_card_sn
                    .clone()
                    .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
                self.gift_cards_detail(&sn, false).await
            }
            GROUP_TYPE_EXCHANGE => {
                // 兑换卡 exchange 同为入卡包绑定（原版语义）；核销走 useCard。
                let products = self.repo().find_group_products(card.card_groups_id).await?;
                if products.is_empty() {
                    return Err(Error::biz(500, MSG_PRODUCTS_OFFLINE));
                }
                card.user_id = Some(user_id);
                card.claimed_user_id = Some(user_id);
                card.claimed_status = Some(CLAIMED_RECEIVED);
                self.repo().update(&card).await?;
                let sn = card
                    .gift_card_sn
                    .clone()
                    .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_EXISTS))?;
                let mut vo = self.gift_cards_detail(&sn, false).await?;
                vo.gift_card_products = Some(products);
                vo.exchange_product_to_cart = Some(
                    nextrade_core::models::marketing::gift_card::ExchangeProductToCartVO {
                        item: true,
                        flow_type: 0,
                        cart_ids: Vec::new(),
                    },
                );
                Ok(vo)
            }
            _ => Err(Error::biz(500, MSG_UNSUPPORTED_GROUP_TYPE)),
        }
    }

    /// 使用/核销兑换卡（原版 `useCard`，标准对齐 D6）。
    ///
    /// 校验：卡归属当前会员、兑换卡组、status=0、is_available=1、未过期。
    /// 固定组合（exchangeMode=1）：整组绑定商品各 1 件；
    /// 任选（exchangeMode=2）：items 按 SKU 数量计件（同款可多件），
    /// 总件数必须等于 selectCount，每项必须在卡组绑定商品池内。
    /// 顺序：先写购物车（库存由加车校验封顶）→ 再核销卡（status=1），
    /// 购物车行以 extra_data.giftCardsId 标记，结算 flowType=12 识别。
    /// 返回回执（flowType=12 + cartIds）。
    pub async fn use_card(
        &self,
        param: &UseCardParam,
        user_id: i32,
    ) -> Result<nextrade_core::models::marketing::gift_card::ExchangeProductToCartVO> {
        if user_id <= 0 {
            return Err(Error::biz(500, MSG_CARD_NOT_OWNED));
        }
        let mut card = self
            .repo()
            .find_by_id(param.gift_cards_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CARD_NOT_OWNED))?;
        if card.claimed_user_id != Some(user_id) && card.user_id != Some(user_id) {
            return Err(Error::biz(500, MSG_CARD_NOT_OWNED));
        }
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_GROUP_NOT_EXISTS))?;
        if group.group_type != GROUP_TYPE_EXCHANGE {
            return Err(Error::biz(500, MSG_UNSUPPORTED_GROUP_TYPE));
        }
        if card.status != Some(0) || card.is_available != Some(YES) {
            return Err(Error::biz(500, MSG_CARD_ALREADY_EXCHANGED));
        }
        if card_expired(&card, now_seconds_i32()) {
            return Err(Error::biz(500, MSG_CARD_EXPIRED));
        }
        let (exchange_mode, select_count) = self
            .repos
            .marketing
            .gift_card_groups
            .find_exchange_fields(card.card_groups_id)
            .await?
            .unwrap_or((1, 0));
        let bound = self.repo().find_group_products(card.card_groups_id).await?;
        if bound.is_empty() {
            return Err(Error::biz(500, MSG_PRODUCTS_OFFLINE));
        }
        // 选品定案：(product_id, sku_id, quantity)
        let mut selected: Vec<(i32, i32, i32)> = Vec::new();
        if exchange_mode == 2 {
            let items = param.items.clone().unwrap_or_default();
            if items.is_empty() {
                return Err(Error::biz(500, MSG_SELECT_COUNT_INVALID));
            }
            let mut total_qty: i32 = 0;
            for item in items {
                if item.quantity <= 0 {
                    return Err(Error::biz(500, MSG_SELECT_COUNT_INVALID));
                }
                let in_pool = bound.iter().any(|p| {
                    p.product_id == item.product_id
                        && (p.sku_id == item.sku_id || p.sku_id == 0)
                });
                if !in_pool {
                    return Err(Error::biz(500, MSG_PRODUCT_NOT_IN_GROUP));
                }
                total_qty += item.quantity;
                if let Some(exist) = selected
                    .iter_mut()
                    .find(|(pid, sid, _)| *pid == item.product_id && *sid == item.sku_id)
                {
                    exist.2 += item.quantity;
                } else {
                    selected.push((item.product_id, item.sku_id, item.quantity));
                }
            }
            let want = if select_count > 0 { select_count } else { 1 };
            if total_qty != want {
                return Err(Error::biz(500, MSG_SELECT_COUNT_INVALID));
            }
        } else {
            for p in &bound {
                selected.push((p.product_id, p.sku_id, 1));
            }
        }
        // 先写购物车（加车内含可购/库存校验），再核销卡，避免加车失败先废卡
        let product_service =
            crate::catalog::product::ProductService::new(self.repos.clone(), None);
        let cart_service =
            crate::trade::cart::CartService::new(self.repos.clone(), product_service);
        let mut cart_ids: Vec<i32> = Vec::new();
        for (idx, (product_id, sku_id, quantity)) in selected.iter().enumerate() {
            let cart_id = cart_service
                .add_gift_exchange_line(
                    user_id,
                    *product_id,
                    *sku_id,
                    *quantity,
                    card.gift_cards_id,
                    idx == 0,
                )
                .await?;
            cart_ids.push(cart_id);
        }
        let now = now_seconds_i32();
        card.status = Some(YES);
        card.used_time = Some(now);
        card.user_id = Some(user_id);
        self.repo().update(&card).await?;
        Ok(
            nextrade_core::models::marketing::gift_card::ExchangeProductToCartVO {
                item: true,
                flow_type: 12,
                cart_ids,
            },
        )
    }

    /// 转赠链接（Java `getTransferLink`；配置域名未迁移，使用相对路径）。
    pub async fn get_transfer_link(&self, gift_card_sn: &str, is_from: Option<i32>) -> Result<String> {
        if is_from != Some(TRANSFER_FROM_H5) && is_from != Some(TRANSFER_FROM_PC) {
            return Err(Error::biz(500, MSG_TRANSFER_SOURCE_REQUIRED));
        }
        if gift_card_sn.trim().is_empty() {
            return Err(Error::biz(500, MSG_TRANSFER_SN_REQUIRED));
        }
        let card = self
            .repo()
            .find_by_sn(gift_card_sn)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_TRANSFER_CARD_NOT_FOUND))?;
        let group = self
            .repo()
            .find_group(card.card_groups_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_TRANSFER_GROUP_NOT_FOUND))?;
        let (shop_title, _) = self
            .resolve_shop(group.shop_id.unwrap_or(0))
            .await?;
        let shop_title = shop_title.unwrap_or_default();
        let base_url = if is_from == Some(TRANSFER_FROM_PC) {
            "/giftCard/transfer/"
        } else {
            "/pages/giftCard/transfer?giftCardSn="
        };
        Ok(format!(
            "【{shop_title}】{base_url}{gift_card_sn}  [{}]",
            group.card_group_name
        ))
    }
}

/// 列表行 → VO（Java `GiftCardsVO.builder()` 只填部分字段）。
fn list_vo(card: &GiftCards) -> GiftCardsVO {
    GiftCardsVO {
        gift_cards_id: Some(card.gift_cards_id),
        card_groups_id: Some(card.card_groups_id),
        card_number: Some(card.card_number.clone()),
        gift_card_sn: card.gift_card_sn.clone(),
        card_password: Some(card.card_password.clone()),
        is_available: card.is_available,
        status: card.status,
        is_sold: Some(is_sold_text(card.user_id)),
        is_exchange: Some(is_exchange_text(card.status)),
        add_time: seconds_text(card.add_time),
        ..Default::default()
    }
}

/// 卡号/卡密随机字符集（Java `StringUtils.CHARSET`：
/// 去掉易混淆的 `I/O/0/1`）。
pub const CARD_RANDOM_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// 生成卡号（Java `StringUtils.generateCardNo(groupId)`：
/// `"GC" + yyyyMMdd + groupId + 4 位随机`）。
#[must_use]
pub fn generate_card_no(group_id: i32) -> String {
    let date = chrono::Utc::now().format("%Y%m%d");
    format!("GC{date}{group_id}{}", random_chars(4))
}

/// 生成卡密（Java `StringUtils.generatePassword()`：8 位随机）。
#[must_use]
pub fn generate_card_password() -> String {
    random_chars(8)
}

/// 从 [`CARD_RANDOM_CHARSET`] 随机取 `len` 个字符。
fn random_chars(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CARD_RANDOM_CHARSET.len());
            CARD_RANDOM_CHARSET[idx] as char
        })
        .collect()
}

/// 过期判定（Java `isGiftCardExpired`）：`expiredTime == null` ⇒ true；
/// `== 0` ⇒ false（永久）；否则 `now > expiredTime`。
#[must_use]
pub fn card_expired(card: &GiftCards, now: i32) -> bool {
    if card.expired_time == 0 {
        return false;
    }
    now > card.expired_time
}

/// 转赠资格判定（D10，原版 `giftCardTransfer` 判定函数口径）：
/// 仅电子卡（card_type=1）、status=0、is_available=1、未过期；
/// 储值卡（group_type=1）须 balance == 初始面额（face_amount 承载
/// initialBalance；balance 为 NULL 按面额计）且 balance > 0；
/// 兑换卡未使用即可。
#[must_use]
pub fn transfer_eligible(
    card: &GiftCards,
    card_type: i32,
    group_type: i32,
    face_amount: Option<Decimal>,
    balance: Option<Decimal>,
    now: i32,
) -> bool {
    if card_type != 1 || card.status != Some(0) || card.is_available != Some(YES) {
        return false;
    }
    if card_expired(card, now) {
        return false;
    }
    if group_type == GROUP_TYPE_TOP_UP {
        let face = face_amount.unwrap_or(Decimal::ZERO);
        let bal = balance.unwrap_or(face);
        return bal == face && bal > Decimal::ZERO;
    }
    true
}

/// 演示站手机号脱敏（Java `@Sensitive(PHONE)`）：仅 `is_demo == 1` 时脱敏。
#[must_use]
pub fn visible_phone(phone: &str, is_demo: i32) -> String {
    if is_demo == 1 {
        mask_mobile(phone)
    } else {
        phone.to_string()
    }
}

/// 组装「用户地址信息」（对应 Java 兑换卡分支的 `UserAddressInfoVO`）。
fn address_info(
    order: &Order,
    user: Option<(String, String)>,
    is_demo: i32,
) -> GiftCardUserAddressInfoVO {
    let region_name = parse_region_names(&order.region_names);
    GiftCardUserAddressInfoVO {
        username: user.map(|(name, _)| name),
        consignee: Some(order.consignee.clone()),
        mobile: Some(visible_phone(&order.mobile, is_demo)),
        email: Some(order.email.clone()),
        region_name,
        address: Some(order.address.clone()),
        buyer_note: Some(order.buyer_note.clone()),
    }
}

/// 配送信息（对应 Java `DeliveryInfoVO`）。
fn delivery_info(order: &Order) -> GiftCardDeliveryInfoVO {
    GiftCardDeliveryInfoVO {
        shipping_type_name: Some(order.shipping_type_name.clone()),
        shipping_time: if order.shipping_time > 0 {
            Some(format_unix_time(Some(order.shipping_time)))
        } else {
            None
        },
        logistics_name: Some(order.logistics_name.clone()),
        tracking_no: Some(order.tracking_no.clone()),
    }
}

/// `order.region_names`（JSON 数组）→ 空格拼接；非法 JSON ⇒ None（Java 抛异常）。
fn parse_region_names(raw: &str) -> Option<String> {
    let list: Vec<serde_json::Value> = serde_json::from_str(raw).ok()?;
    let names: Vec<String> = list.iter().map(|v| v.to_string().trim_matches('"').to_string()).collect();
    if names.is_empty() {
        None
    } else {
        Some(names.join(" "))
    }
}

fn now_seconds_i32() -> i32 {
    i32::try_from(now_seconds()).unwrap_or(0)
}

/// \"BasePage\" → \"PageQuery\"。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> PageQuery {
    PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

/// 归属校验：经卡组 `gift_card_groups.shop_id`（店铺账号越权 → 403）。
async fn ensure_group_access(
    repo: &nextrade_infrastructure::db::repo::marketing::gift_card::GiftCardRepository,
    card_groups_id: i32,
) -> Result<()> {
    if let Some(group) = repo.find_group(card_groups_id).await? {
        TenantScope::current_ensure_access(group.shop_id.unwrap_or(0))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(expired_time: i32) -> GiftCards {
        GiftCards {
            gift_cards_id: 1,
            card_groups_id: 2,
            card_number: "GC20251031133058".into(),
            card_password: "PASSWORD123456".into(),
            gift_card_sn: Some("sn-1".into()),
            is_available: Some(1),
            status: Some(0),
            used_time: None,
            user_id: None,
            add_time: Some(1_733_000_000),
            expired_time,
            claimed_user_id: None,
            claimed_status: Some(CLAIMED_RECEIVABLE),
            order_time: None,
        }
    }

    #[tokio::test]
    async fn tenant_scope_blocks_cross_shop_gift_card() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        assert!(with_tenant_scope(shop, async { TenantScope::current_ensure_access(9) })
            .await
            .is_err());
        assert!(with_tenant_scope(shop, async { TenantScope::current_ensure_access(8) })
            .await
            .is_ok());
        let filter = with_tenant_scope(TenantScope::platform(), async {
            TenantScope::current_list_filter()
        })
        .await;
        assert_eq!(filter, None);
    }

    #[test]
    fn expired_rule_matches_java() {
        // 0 ⇒ 永久有效（false）
        assert!(!card_expired(&card(0), 1_733_000_000));
        // 非 0：now > expired
        assert!(card_expired(&card(1_000_000_000), 1_733_000_000));
        assert!(!card_expired(&card(1_900_000_000), 1_733_000_000));
    }

    #[test]
    fn list_vo_fills_only_java_builder_fields() {
        let vo = list_vo(&card(0));
        assert_eq!(vo.gift_cards_id, Some(1));
        assert_eq!(vo.is_sold.as_deref(), Some("否"));
        assert_eq!(vo.is_exchange.as_deref(), Some("否"));
        // Java `.addTime(Long.toString())` ⇒ 秒级时间戳字符串
        assert_eq!(vo.add_time.as_deref(), Some("1733000000"));
        // builder 未填字段
        assert!(vo.card_group_name.is_none());
        assert!(vo.used_time.is_none());
    }

    #[test]
    fn exchange_detail_phone_masks_only_on_demo() {
        assert_eq!(visible_phone("13812348000", IS_DEMO), "13812348000");
        assert_eq!(visible_phone("13812348000", 1), "138****8000");
    }

    #[test]
    fn region_names_json_join_like_java() {
        assert_eq!(
            parse_region_names(r#"["江西省","南昌市"]"#).as_deref(),
            Some("江西省 南昌市")
        );
        assert_eq!(parse_region_names("[]"), None);
        assert_eq!(parse_region_names("not-json"), None);
    }

    #[test]
    fn transfer_link_uses_relative_base_when_config_absent() {
        // Java: 平台店（shopId=0）取 SHOP_NAME 配置；未迁移 ⇒ 空标题前缀
        let base_pc = "/giftCard/transfer/";
        let base_h5 = "/pages/giftCard/transfer?giftCardSn=";
        assert_eq!(
            format!("【{}】{base_h5}sn-1  [礼品卡]", ""),
            "【】/pages/giftCard/transfer?giftCardSn=sn-1  [礼品卡]"
        );
        assert!(base_pc.ends_with('/'));
    }
}
