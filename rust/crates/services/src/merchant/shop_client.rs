//! 店铺域前台服务（merchant::shop_client）：店铺列表 / 收藏 / 头部装修 / 店铺详情 / 店铺装修 /
//! 自提点列表 / 自提点详情 / 门店列表 / 门店详情 / 店铺评论列表。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.shop.ShopClientController`（10 个端点）
//! - `com.nextrade.service.shop.impl.ShopServiceImpl`
//!   （`clientList` / `clientDetail` / `pickupList` / `pickupDetail` /
//!   `storeList` / `storeDetail` 及私有辅助 `prependClientSelfOperatedStore` /
//!   `buildClientSelfOperatedShopVO` / `shouldInsertSelfOperatedStore` /
//!   `isSelfOperatedStoreSuspended` / `resolveSelfOperatedStoreLogo` /
//!   `fillSelfOperatedStoreListVO` / `resolveSelfOperatedTipNames`）
//! - `com.nextrade.service.shop.impl.CollectShopServiceImpl#collect`
//! - `com.nextrade.service.decorate.impl.DecorateServiceImpl#getShopIndexDecorateData`
//! - `com.nextrade.service.decorate.impl.DecorateDiscreteServiceImpl#getShopHead`
//! - `com.nextrade.service.product.impl.CommentServiceImpl#listByShopId`
//! - `com.nextrade.service.merchant.impl.MerchantServiceImpl#getApprovedLicensePic`
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl#getStoreSelfOperatedSettings`
//! - Mapper XML：`ShopMapper.storeListByLocation` / `storeListByRegion` /
//!   `pickupListByLocation`、`OrderItemMapper.getShopSalesByShopId`
//! - `com.nextrade.common.utils.GpsDistanceUtil`、`TigUtils.handelTime`
//!
//! **未迁移 / 已简化（详见返回值的 `uncertain_points`）**：
//! - 多语言翻译（`@JsonTranslate` / `TranslatePackage`）与 `X-Locale-Code`
//!   仅用于 `decorate` 的 locale 回退，其它字段不做翻译。
//! - 敏感信息脱敏（`@Sensitive(PHONE)`）。
//! - 门店运力「第三方运力」集合恒为空（Java 中 `thirdPartyCarrierShopIdsTmp`
//!   在 O2O 关闭时为空集，与本仓库 `nextrade.is-o2o: 0` 一致）。
//! - 门店列表的 `distance` 由 SQL 计算（与 XML 一致），随后在服务层用
//!   `GpsDistanceUtil` 口径（半径 6378137m haversine）覆盖，两者公式不同，
//!   与 Java 行为一致但数值会有差异。

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use nextrade_core::entities::{
    Comment, Product, Shop, ShopHomeServiceConfig, ShopLocalDeliveryConfig, StoreTip,
};
use nextrade_core::models::content::decorate::DecorateDiscreteVO;
use nextrade_core::models::merchant::shop_client::{
    split_ints, ClientShopDetailVO, ClientShopListQuery, ClientShopMerchantVO, ClientShopVO,
    CollectShopResultVO, CommentReplyVO, CommentShowPicVO, CommentVO, PickupListQuery, PickupListVO,
    ShopCommentListQuery, ShopContactConfigVO, ShopDecorateQuery, ShopIdQuery, ShopOpenCloseConfigVO,
    ShopTimeRangeStartEndVO, ShopTimeRangeVO, StoreDetailQuery, StoreDetailVO, StoreListQuery,
    StoreListVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::shop_client::{PickupRow, StoreListFilter, StoreListRow};
use nextrade_infrastructure::db::Repositories;

// ————————————————————— 常量（对齐 Java 枚举 / i18n / 配置项） —————————————————————

/// 店铺类型：门店（Java `ShopTypeEnum.STORE`）。
const SHOP_TYPE_STORE: i32 = 2;
/// 店铺状态：开业（Java `ShopType.OPEN` / `StoreType.OPEN`）。
const STATUS_OPEN: i32 = 1;
/// 店铺状态：暂停运营（Java `StoreType.PAUSE`）。
const STATUS_PAUSE: i32 = 4;
/// 店铺状态：关店（Java `ShopType.CLOSE`）。
const STATUS_CLOSE: i32 = 10;
/// 平台自营实体（非 `shop` 表记录）的固定 `shopId`。
const SELF_OPERATED_SHOP_ID: i32 = 0;
/// 商家入驻：审核通过（Java `MerchantApplyStatusEnum.AUDIT_PASS`）。
const MERCHANT_APPLY_AUDIT_PASS: i32 = 10;
/// 全局业务错误码（Java `HttpStatus.SERVICE_DATA_ERROR`）。
const SERVICE_DATA_ERROR: i32 = 1001;

/// `error.shop.not.exist` / `ShopConstants.SHOP_NOT_EXIST`：店铺不存在。
const ERR_SHOP_NOT_EXIST: &str = "店铺不存在";
/// `ShopServiceImpl#pickupList` 的「没有此门店」。
const ERR_STORE_NOT_EXIST: &str = "没有此门店";
/// `ShopServiceImpl#pickupDetail` 的「没有此自提点」。
const ERR_PICKUP_NOT_EXIST: &str = "没有此自提点";
/// `error.decorate.template.not.exist`：模板不存在。
const ERR_DECORATE_TEMPLATE_NOT_EXIST: &str = "模板不存在";
/// `ExceptionConstants.DATA_TYPE_ERROR`：数据类型异常。
const ERR_DATA_TYPE: &str = "数据类型异常";

/// 配置项 `shopName`（Java `SettingsEnum.SHOP_NAME`）。
const CFG_SHOP_NAME: &str = "shopName";
/// 配置项 `shopLogo`（Java `SettingsEnum.SHOP_LOGO`）。
const CFG_SHOP_LOGO: &str = "shopLogo";
/// 配置项 `defaultAvatar`（Java `SettingsEnum.DEFAULT_AVATAR`）。
const CFG_DEFAULT_AVATAR: &str = "defaultAvatar";
/// 配置项 `storeShowOtherCityStore`（Java `SettingsEnum.STORE_SHOW_OTHER_CITY_STORE`）。
const CFG_STORE_SHOW_OTHER_CITY_STORE: &str = "storeShowOtherCityStore";

/// 平台自营门店配置项（Java `SettingsEnum.STORE_SELF_OPERATED_*` 的 `biz_code`）。
const CFG_SELF_OPERATED: [&str; 12] = [
    "storeSelfOperatedName",
    "storeSelfOperatedLogo",
    "storeSelfOperatedRegion",
    "storeSelfOperatedAddress",
    "storeSelfOperatedLongitude",
    "storeSelfOperatedLatitude",
    "storeSelfOperatedPhone",
    "storeSelfOperatedDescription",
    "storeSelfOperatedStatus",
    "storeSelfOperatedContactJson",
    "storeSelfOperatedOpenCloseJson",
    "storeSelfOperatedTipsJson",
];

/// 店铺头部装修组件的固定 `decorate_sn`。
const DECORATE_SN_SHOP_HEAD: &str = "shopHead";

/// 平台自营门店配置（Java `GetStoreSelfOperatedVO` + `ConfigServiceImpl#getStoreSelfOperatedSettings`）。
///
/// 缺省值取自 `SettingsEnum` 的 `defaultValue`（`SettingsHelper#getRaw` 的回退口径）：
/// 名称为 `""`、状态为 `"1"`（开业）、联系方式为 `"[]"`、标签为 `"[]"`、
/// 营业时间为 `""`。
#[derive(Debug, Clone, Default)]
pub struct SelfOperatedSettings {
    pub name: String,
    pub logo: String,
    pub address: String,
    pub longitude: Option<Decimal>,
    pub latitude: Option<Decimal>,
    pub phone: String,
    pub description: String,
    pub status: Option<i32>,
    pub contact_json: String,
    pub open_close_json: String,
    /// `storeSelfOperatedRegion` 为空串时 Java 侧为 `null`
    pub region_names: Option<Vec<String>>,
    pub contact_configs: Vec<ShopContactConfigVO>,
    pub open_close_config: Option<ShopOpenCloseConfigVO>,
    pub tip_ids: Vec<i32>,
    pub tip_names: Vec<String>,
}

pub struct ShopClientService {
    repos: Arc<Repositories>,
}

impl ShopClientService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::merchant::shop_client::ShopClientRepository {
        &self.repos.merchant.shop_client
    }

    // ————————————————————— 1. 店铺列表 —————————————————————

    /// `GET /api/shop/shop/list`：前台店铺分页。
    ///
    /// 对应 Java `ShopServiceImpl#clientList`：
    /// 仅 `status = 1`；每店附带上架商品预览（`productNumber`，缺省 5）、
    /// 收藏人数（`COUNT(DISTINCT user_id)`）、上架商品数、商品评分
    /// （`AVG(comment_rank)`）与评星（评价数 < 100 固定 4.8）。
    ///
    /// 第 1 页且显式 `excludeSelfOperated=0` 时，在首位插入平台自营虚拟店
    /// （`shopId=0`，仅状态为「开业」时插入，且计入名称关键字匹配）。
    /// **注意**：与原 Java 一致，该占位**不计入 `total`**。
    pub async fn client_list(&self, q: &ClientShopListQuery) -> Result<PageResult<ClientShopVO>> {
        let page = q.page_query();
        let keyword = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let (total, rows) = self
            .repo()
            .paginate_client_shops(keyword, q.sort_field.as_deref(), q.sort_order.as_deref(), &page)
            .await?;

        let product_limit = q.product_limit();
        let mut list: Vec<ClientShopVO> = Vec::new();

        if !rows.is_empty() {
            let shop_ids: Vec<i32> = rows.iter().map(|s| s.shop_id).collect();

            // 批量预取，避免逐店 N+1（Java 同样是 4 条批量查询）
            let mut products_by_shop: HashMap<i32, Vec<Product>> = HashMap::new();
            for p in self.repo().find_products_by_shops(&shop_ids).await? {
                products_by_shop.entry(p.shop_id).or_default().push(p);
            }
            let counts: HashMap<i32, i64> = self
                .repo()
                .count_products_by_shops(&shop_ids)
                .await?
                .into_iter()
                .collect();
            let collects: HashMap<i32, i64> = self
                .repo()
                .count_collect_users_by_shops(&shop_ids)
                .await?
                .into_iter()
                .collect();
            let scores: HashMap<i32, Decimal> = self
                .repo()
                .avg_comment_rank_by_shops(&shop_ids)
                .await?
                .into_iter()
                .collect();

            for shop in &rows {
                let mut product_list = products_by_shop.remove(&shop.shop_id).unwrap_or_default();
                product_list.truncate(product_limit.max(0) as usize);

                let mut vo = build_client_shop_vo(shop)?;
                vo.listing_product = Some(product_list);
                vo.listing_count = Some(counts.get(&shop.shop_id).copied().unwrap_or(0));
                vo.collect_count = Some(collects.get(&shop.shop_id).copied().unwrap_or(0));
                vo.product_score = Some(scores.get(&shop.shop_id).copied().unwrap_or(Decimal::ZERO));
                list.push(vo);
            }
        }

        // —— prependClientSelfOperatedStore ——
        if q.page.unwrap_or(1) == 1
            && q.exclude_self_operated == Some(0)
            && !list.iter().any(|v| v.shop_id == Some(SELF_OPERATED_SHOP_ID))
        {
            if let Some(vo) = self
                .build_client_self_operated_vo(q.keyword.as_deref(), product_limit)
                .await?
            {
                list.insert(0, vo);
            }
        }

        Ok(PageResult::new(total, list, &page))
    }

    /// `ShopServiceImpl#prependClientSelfOperatedStore` +
    /// `buildClientSelfOperatedShopVO`（C 端自营虚拟店铺）。
    async fn build_client_self_operated_vo(
        &self,
        keyword: Option<&str>,
        product_limit: i32,
    ) -> Result<Option<ClientShopVO>> {
        let s = self.self_operated_settings().await?;
        if s.status != Some(STATUS_OPEN) {
            return Ok(None);
        }
        let title = s.name.clone();
        if let Some(kw) = keyword.map(str::trim).filter(|k| !k.is_empty()) {
            if title.is_empty() || !title.contains(kw) {
                return Ok(None);
            }
        }
        let logo = self.resolve_self_operated_store_logo(&s).await?;
        Ok(Some(ClientShopVO {
            status_text: shop_status_text(s.status).ok(),
            shop_id: Some(SELF_OPERATED_SHOP_ID),
            shop_title: Some(title),
            shop_logo: Some(logo),
            status: s.status,
            description: Some(s.description.clone()),
            contact_mobile: Some(s.phone.clone()),
            shop_star: Some(decimal("4.8")),
            listing_product: Some(
                self.repo()
                    .find_shop_products(SELF_OPERATED_SHOP_ID, i64::from(product_limit))
                    .await?,
            ),
            listing_count: Some(
                self.repo()
                    .count_shop_products_online(SELF_OPERATED_SHOP_ID)
                    .await?,
            ),
            collect_count: Some(
                self.repo()
                    .count_collect_users(SELF_OPERATED_SHOP_ID)
                    .await?,
            ),
            product_score: Some(
                self.repo()
                    .avg_comment_rank(SELF_OPERATED_SHOP_ID)
                    .await?
                    .unwrap_or(Decimal::ZERO),
            ),
            ..Default::default()
        }))
    }

    // ————————————————————— 2. 店铺收藏 —————————————————————

    /// `POST /api/shop/shop/collect`：收藏/取消收藏（切换语义）。
    ///
    /// 对应 Java `CollectShopServiceImpl#collect`：已收藏则删除，否则新增。
    /// 原接口返回 `void`，此处回传切换后的状态便于前端联动。
    pub async fn collect(&self, shop_id: i32, user_id: i32) -> Result<CollectShopResultVO> {
        match self.repo().find_collect(user_id, shop_id).await? {
            None => {
                self.repo().insert_collect(user_id, shop_id).await?;
                Ok(CollectShopResultVO { collected: true })
            }
            Some(row) => {
                self.repo().delete_collect(row.collect_id).await?;
                Ok(CollectShopResultVO { collected: false })
            }
        }
    }

    // ————————————————————— 3. 店铺头部装修 —————————————————————

    /// `GET /api/shop/shop/head?shopId=`：店铺头部装修组件。
    ///
    /// 对应 Java `DecorateDiscreteServiceImpl#getShopHead`：
    /// 取 `decorate_discrete` 中 `shop_id = ? AND decorate_sn = 'shopHead'`，
    /// 命中则回吐原样 JSON，未命中返回空 VO（而不是 404）。
    /// `GET /api/shop/shop/groupDetail`：装修选择分组详情（公开字段）。
    pub async fn group_detail(&self, group_id: i32, shop_type: i32) -> Result<serde_json::Value> {
        use sqlx::Row;
        let group = sqlx::query(
            "SELECT group_id, group_name, shop_type FROM shop_group WHERE group_id=$1 AND shop_type=$2 LIMIT 1",
        )
        .bind(group_id)
        .bind(shop_type)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let Some(group) = group else {
            return Ok(serde_json::Value::Null);
        };
        let ids: Vec<(i32,)> = sqlx::query_as(
            "SELECT shop_id FROM shop_group_relation WHERE group_id=$1 ORDER BY shop_id",
        )
        .bind(group_id)
        .fetch_all(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(serde_json::json!({
            "groupId": group.try_get::<i32, _>("group_id").unwrap_or(group_id),
            "groupName": group.try_get::<String, _>("group_name").unwrap_or_default(),
            "shopType": group.try_get::<i32, _>("shop_type").unwrap_or(shop_type),
            "shopIds": ids.into_iter().map(|(id,)| id).collect::<Vec<_>>()
        }))
    }

    pub async fn shop_head(&self, q: &ShopIdQuery) -> Result<DecorateDiscreteVO> {
        Ok(
            match self
                .repo()
                .find_discrete(q.shop_id, DECORATE_SN_SHOP_HEAD)
                .await?
            {
                Some(d) => DecorateDiscreteVO::from(&d),
                None => DecorateDiscreteVO::default(),
            },
        )
    }

    // ————————————————————— 4. 店铺详情 —————————————————————

    /// `GET /api/shop/shop/detail?shopId=`：前台店铺详情。
    ///
    /// 对应 Java `ShopServiceImpl#clientDetail`：
    /// - `shopId = 0` 为平台自营**店铺**虚拟主体：`is-o2o = 0` 时取
    ///   `shopName` / `shopLogo` 配置（`java/nextrade-api/.../application.yaml`）；
    /// - 其它：评星按 `selectByIdWithCalculatedStar`（评价数 < 100 固定 4.8），
    ///   附商户信息、商品/新品总数（含门店分配商品）、是否收藏、商品评分、
    ///   收藏人数与营业执照图。
    pub async fn client_detail(&self, shop_id: i32, user_id: i32) -> Result<ClientShopDetailVO> {
        if shop_id == SELF_OPERATED_SHOP_ID {
            // isO2o == 0 → 走「店铺」口径（`SettingsEnum.SHOP_NAME` / `SHOP_LOGO`）
            let title = self
                .repo()
                .find_config_val(CFG_SHOP_NAME)
                .await?
                .unwrap_or_default();
            let logo = self
                .repo()
                .find_config_val(CFG_SHOP_LOGO)
                .await?
                .unwrap_or_default();
            return Ok(ClientShopDetailVO {
                shop_id: Some(SELF_OPERATED_SHOP_ID),
                shop_title: Some(title),
                shop_logo: Some(logo),
                ..Default::default()
            });
        }

        let shop = self
            .repo()
            .find_shop(shop_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_SHOP_NOT_EXIST))?;

        let mut vo = ClientShopDetailVO {
            status_text: Some(shop_status_text(shop.status)?),
            shop_id: Some(shop.shop_id),
            shop_title: shop.shop_title.clone(),
            add_time: Some(fmt_datetime(shop.add_time)),
            shop_logo: shop.shop_logo.clone(),
            click_count: shop.click_count,
            status: shop.status,
            merchant_id: shop.merchant_id,
            shop_money: shop.shop_money,
            frozen_money: shop.frozen_money,
            contact_mobile: shop.contact_mobile.clone(),
            description: shop.description.clone(),
            kefu_phone: shop.kefu_phone.clone(),
            kefu_weixin: shop.kefu_weixin.clone(),
            kefu_link: shop.kefu_link.clone(),
            is_contact_kefu: Some(shop.is_contact_kefu),
            kefu_inlet: shop.kefu_inlet.clone(),
            shop_star: calculated_shop_star(&shop),
            business_license_pic: Some(
                self.approved_license_pic(shop.merchant_id).await?,
            ),
            ..Default::default()
        };

        // 商户信息（Java 仅在 merchantId > 0 时查询）
        if let Some(mid) = shop.merchant_id.filter(|m| *m > 0) {
            if let Some(m) = self.repo().find_merchant(mid).await? {
                vo.merchant = Some(ClientShopMerchantVO {
                    type_text: Some(merchant_type_text(m.r#type)?),
                    status_text: None,
                    merchant_id: Some(m.merchant_id),
                    merchant_apply_id: m.merchant_apply_id,
                    user_id: m.user_id,
                    // Java 此处取的是 **shop** 的 addTime（疑似笔误），此处照抄
                    add_time: Some(fmt_datetime(shop.add_time)),
                    merchant_data: parse_json_opt(m.merchant_data.as_deref()),
                    status: m.status,
                    r#type: m.r#type,
                    company_name: m.company_name,
                    corporate_name: m.corporate_name,
                    settlement_cycle: m.settlement_cycle,
                });
            }
        }

        // 商品总数 = 店铺自建商品数（不限状态）+ 平台分配门店商品数
        let store_products = self
            .repo()
            .find_store_products_of_shop(shop.shop_id)
            .await?;
        let product_count = self
            .repo()
            .count_shop_products_all(shop.shop_id)
            .await?
            + i64::try_from(store_products.len()).unwrap_or(0);
        vo.product_count = Some(product_count);

        // 新品数 = 店铺自建新品 + 分配商品中 is_new = 1 的数量
        let mut new_count = self.repo().count_shop_new_products(shop.shop_id).await?;
        let assigned_ids: Vec<i32> = store_products
            .iter()
            .filter_map(|sp| sp.product_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        new_count += self.repo().count_new_products_in(&assigned_ids).await?;
        vo.new_product_count = Some(new_count);

        vo.collect_shop = Some(
            self.repo()
                .find_collect(user_id, shop.shop_id)
                .await?
                .is_some(),
        );
        vo.product_score = Some(
            self.repo()
                .avg_comment_rank(shop.shop_id)
                .await?
                .unwrap_or(Decimal::ZERO),
        );
        vo.collect_count = Some(self.repo().count_collect_users(shop.shop_id).await?);
        Ok(vo)
    }

    // ————————————————————— 5. 店铺装修 —————————————————————

    /// `GET /api/shop/shop/decorate?shopId=&previewId=`：店铺首页装修数据。
    ///
    /// 对应 Java `DecorateServiceImpl#getShopIndexDecorateData`：
    /// `previewId` 优先按 `decorate_id` 取；否则取该店铺已发布首页
    /// （`is_home = 1 AND status = 1 AND decorate_type = 1`）。
    /// 命中后按 `X-Locale-Code` 做一次子装修页回退，最后在 `data` JSON 上
    /// 追加 `decorateId` 字段后原样返回。
    pub async fn shop_decorate(
        &self,
        q: &ShopDecorateQuery,
        locale_code: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut decorate = match q.preview_id {
            Some(id) => self.repo().find_decorate(id).await?,
            None => {
                self.repo()
                    .find_shop_home_decorate(q.shop_id.unwrap_or(SELF_OPERATED_SHOP_ID))
                    .await?
            }
        }
        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_DECORATE_TEMPLATE_NOT_EXIST))?;

        if let Some(code) = locale_code.map(str::trim).filter(|c| !c.is_empty()) {
            if let Some(locale) = self.repo().find_locale_by_code(code).await? {
                if locale.is_default != Some(1) {
                    if let Some(child) = self
                        .repo()
                        .find_decorate_child(decorate.decorate_id, locale.id)
                        .await?
                    {
                        decorate = child;
                    }
                }
            }
        }

        let mut data = decorate
            .data
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(map) = data.as_object_mut() {
            map.insert(
                "decorateId".to_string(),
                serde_json::json!(decorate.decorate_id),
            );
        }
        Ok(data)
    }

    // ————————————————————— 6. 店铺关联自提点 —————————————————————

    /// `GET /api/shop/shop/pickupList`：店铺关联自提点分页。
    ///
    /// 对应 Java `ShopServiceImpl#pickupList`：
    /// `shopId > 0` 且店铺不存在时报「没有此门店」；
    /// `storeShowOtherCityStore = 1` 时忽略 `regionId`；纬度为 `null`/`0`
    /// 时按 `shop_id` 排序（否则按距离）。
    pub async fn pickup_list(&self, q: &PickupListQuery) -> Result<PageResult<PickupListVO>> {
        let parent_shop_id = q.shop_id.unwrap_or(0);
        if parent_shop_id > 0 && self.repo().find_shop(parent_shop_id).await?.is_none() {
            return Err(Error::biz(SERVICE_DATA_ERROR, ERR_STORE_NOT_EXIST));
        }

        let region_id = if self.store_show_other_city().await? {
            None
        } else {
            q.region_id.as_deref()
        };
        let location = valid_location(q.longitude, q.latitude);

        let page = q.page_query();
        let (total, rows) = self
            .repo()
            .paginate_pickups(
                parent_shop_id,
                region_id,
                q.keyword.as_deref(),
                location,
                &page,
            )
            .await?;

        let mut list = Vec::with_capacity(rows.len());
        for row in &rows {
            list.push(self.pickup_vo_from_row(row).await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /api/shop/shop/pickupDetail?shopId=`：自提点详情。
    ///
    /// 对应 Java `ShopServiceImpl#pickupDetail`（不存在时报「没有此自提点」）。
    pub async fn pickup_detail(&self, q: &ShopIdQuery) -> Result<PickupListVO> {
        let shop = self
            .repo()
            .find_shop(q.shop_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_PICKUP_NOT_EXIST))?;

        let region_ids = parse_int_list_json(shop.shop_region_id.as_deref());
        let region_names = self.region_names(&region_ids).await?;
        Ok(PickupListVO {
            shop_id: Some(shop.shop_id),
            shop_title: shop.shop_title.clone(),
            distance: None,
            longitude: shop.shop_longitude,
            latitude: shop.shop_latitude,
            contact_configs: parse_contact_configs(shop.shop_contact_json.as_deref()),
            shop_open_close_config: parse_open_close_config(shop.shop_open_close_json.as_deref()),
            shop_region_ids: Some(region_ids),
            shop_region_names: region_names,
            address: shop.shop_detailed_address.clone(),
        })
    }

    /// `PickupRow` → `PickupListVO`（含地区名回填）。
    async fn pickup_vo_from_row(&self, row: &PickupRow) -> Result<PickupListVO> {
        let region_ids = parse_int_list_json(row.region_ids_json.as_deref());
        let region_names = self.region_names(&region_ids).await?;
        Ok(PickupListVO {
            shop_id: Some(row.shop_id),
            shop_title: row.shop_title.clone(),
            distance: row.distance,
            longitude: row.longitude,
            latitude: row.latitude,
            contact_configs: parse_contact_configs(row.contact_json.as_deref()),
            shop_open_close_config: parse_open_close_config(row.open_close_json.as_deref()),
            shop_region_ids: Some(region_ids),
            shop_region_names: region_names,
            address: row.address.clone(),
        })
    }

    // ————————————————————— 7. 门店列表 —————————————————————

    /// `GET /api/shop/shop/storeList`：O2O 门店分页。
    ///
    /// 对应 Java `ShopServiceImpl#storeList`（含 `storeListByLocation` /
    /// `storeListByRegion` 两个 XML 查询）：
    /// - `storeShowOtherCityStore = 1` 忽略 `regionId`；`storeRegionId` 覆盖 `regionId`；
    /// - `productId` 决定 `shopIds`（门店自建商品按归属门店，平台商品按
    ///   `store_product` 分配记录）；
    /// - 配送类型含 2/4 时按门店配置过滤；含 2/4 且带经纬度时逐店计算
    ///   `inHomeServiceRange` / `inLocalDeliveryRange`；
    /// - 第 1 页插入平台自营门店占位（`shopId=0`）与选中门店置顶；
    /// - 批量预取优惠券、标签、收藏，避免 N+1。
    pub async fn store_list(
        &self,
        q: &StoreListQuery,
        user_id: i32,
    ) -> Result<PageResult<StoreListVO>> {
        let page = q.page_query();
        let ignore_region = self.store_show_other_city().await?;
        // Java 顺序：先按 `storeShowOtherCityStore = 1` 清空 `regionId`，
        // 随后 `storeRegionId` 仍会覆盖它
        let mut region_id = if ignore_region {
            None
        } else {
            q.region_id.clone()
        };
        if let Some(store_region) = q.store_region_id.clone() {
            region_id = Some(store_region);
        }
        let region_id = region_id.filter(|s| !s.is_empty());

        let has_location = q.latitude.is_some_and(|v| v != Decimal::ZERO);
        let location = if has_location {
            Some((
                q.longitude.unwrap_or(Decimal::ZERO),
                q.latitude.unwrap_or(Decimal::ZERO),
            ))
        } else {
            None
        };

        let has_local_type = q.delivery_types.contains(&4);
        let has_home_type = q.delivery_types.contains(&2);
        let delivery_type_filter = has_local_type || has_home_type;
        let in_range_filter = q.within_delivery_range == Some(1) && has_location;
        let only_local_delivery = delivery_type_filter || in_range_filter;

        // —— productId → shopIds ——
        let mut shop_ids = q.shop_ids.clone();
        if let Some(product_id) = q.product_id {
            shop_ids = match self.repo().find_product(product_id).await? {
                Some(p) if p.shop_id > 0 => vec![p.shop_id],
                _ => self
                    .repo()
                    .find_store_products_by_product(product_id)
                    .await?
                    .into_iter()
                    .map(|sp| i32::try_from(sp.shop_id).unwrap_or(0))
                    .collect(),
            };
            if shop_ids.is_empty() {
                // Java：`shopIds` 为空时先判是否插自营占位，否则直接返回空页
                let mut list = Vec::new();
                if should_insert_self_operated(&shop_ids, q.exclude_self_operated, only_local_delivery)
                {
                    let s = self.self_operated_settings().await?;
                    if !is_self_operated_suspended(&s) {
                        list.push(self.build_self_operated_store_vo(&s).await?);
                    }
                }
                return Ok(PageResult::new(0, list, &page));
            }
        }

        let filter = StoreListFilter {
            keyword: q.keyword.clone(),
            region_id,
            shop_ids,
            delivery_types: q.delivery_types.clone(),
            within_delivery_range: in_range_filter,
            selected_shop_id: q.selected_shop_id,
            sort_field: q.sort_field.clone(),
            sort_order: q.sort_order.clone(),
        };
        let (total, rows) = self.repo().paginate_stores(&filter, location, &page).await?;

        let mut records: Vec<StoreListVO> = rows.iter().map(store_row_to_vo).collect();

        // —— 第 1 页插入平台自营门店占位 ——
        if page.page == 1
            && should_insert_self_operated(&filter.shop_ids, q.exclude_self_operated, only_local_delivery)
        {
            let s = self.self_operated_settings().await?;
            if !is_self_operated_suspended(&s) {
                let vo = self.build_self_operated_store_vo(&s).await?;
                records.insert(0, vo);
            }
        }

        // —— 选中门店置顶 ——
        if page.page == 1 {
            if let Some(selected) = q.selected_shop_id.filter(|v| *v > 0) {
                if !records.iter().any(|r| r.shop_id == Some(selected)) {
                    if let Some(shop) = self.repo().find_selected_store(selected).await? {
                        records.insert(0, selected_store_to_vo(&shop));
                    }
                }
            }
        }

        // —— 批量预取：优惠券 / 标签 / 收藏 ——
        // 注意：与 Java 一致，`shopId = 0`（自营占位）也要参与收藏数聚合
        let store_shop_ids: Vec<i32> = {
            let set: HashSet<i32> = records.iter().filter_map(|r| r.shop_id).collect();
            set.into_iter().collect()
        };
        let now = nextrade_common::utils::now_seconds();
        let coupon_shop_ids: HashSet<i32> = self
            .repo()
            .find_coupon_shop_ids(&store_shop_ids, now)
            .await?
            .into_iter()
            .collect();

        let tips = self.repo().find_store_tips(&store_shop_ids).await?;
        let tip_ids: Vec<i32> = {
            let set: HashSet<i32> = tips.iter().map(|t| t.tip_id).collect();
            set.into_iter().collect()
        };
        let tip_name_map: HashMap<i32, String> = self
            .repo()
            .find_tips(&tip_ids)
            .await?
            .into_iter()
            .map(|t| (t.tip_id, t.tip_name))
            .collect();
        let mut shop_tips_map: HashMap<i32, Vec<StoreTip>> = HashMap::new();
        for t in tips {
            shop_tips_map.entry(t.shop_id).or_default().push(t);
        }

        let collects = self.repo().find_collects_by_shops(&store_shop_ids).await?;
        let mut collect_count_map: HashMap<i32, i64> = HashMap::new();
        let mut user_collect_shop_ids: HashSet<i32> = HashSet::new();
        {
            // 每店去重 user_id 后计数（Java `Collectors.mapping(userId, toSet()).size()`）
            let mut distinct: HashMap<i32, HashSet<i32>> = HashMap::new();
            for c in &collects {
                distinct.entry(c.shop_id).or_default().insert(c.user_id);
                if user_id != 0 && c.user_id == user_id {
                    user_collect_shop_ids.insert(c.shop_id);
                }
            }
            for (shop_id, users) in distinct {
                collect_count_map.insert(shop_id, i64::try_from(users.len()).unwrap_or(0));
            }
        }

        // —— 距离（Java `GpsDistanceUtil` 口径，与 SQL 算出的值不同，会覆盖） ——
        let distances: HashMap<i32, Decimal> = match location {
            Some((lng, lat)) => {
                let ids: Vec<i32> = records.iter().filter_map(|r| r.shop_id).collect();
                self.repo().gps_distances_for_shops(lng, lat, &ids).await?
            }
            None => HashMap::new(),
        };

        // —— 配送半径配置（仅相关场景加载，避免 N+1） ——
        let need_local_range = has_location && has_local_type;
        let need_home_range = has_location && has_home_type;
        let local_cfgs: HashMap<i32, ShopLocalDeliveryConfig> = if need_local_range {
            self.repo()
                .find_local_delivery_configs(&store_shop_ids)
                .await?
                .into_iter()
                .map(|c| (c.shop_id, c))
                .collect()
        } else {
            HashMap::new()
        };
        let home_cfgs: HashMap<i32, ShopHomeServiceConfig> = if need_home_range {
            self.repo()
                .find_home_service_configs(&store_shop_ids)
                .await?
                .into_iter()
                .map(|c| (c.shop_id, c))
                .collect()
        } else {
            HashMap::new()
        };

        for vo in &mut records {
            let shop_id = vo.shop_id.unwrap_or(SELF_OPERATED_SHOP_ID);
            let self_operated_placeholder = shop_id == SELF_OPERATED_SHOP_ID;
            if !self_operated_placeholder {
                // 占位行已由 fillSelfOperatedStoreListVO 写入，不能再用源数据覆盖
                vo.contact_configs = parse_contact_configs(vo.contact_json.as_deref());
                vo.tip_names = Some(
                    shop_tips_map
                        .get(&shop_id)
                        .map(|list| {
                            list.iter()
                                .filter_map(|t| tip_name_map.get(&t.tip_id).cloned())
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default(),
                );
                vo.shop_open_close_config = parse_open_close_config(vo.open_close_json.as_deref());
            }
            vo.has_coupon = Some(i32::from(coupon_shop_ids.contains(&shop_id)));
            vo.shop_collect = Some(collect_count_map.get(&shop_id).copied().unwrap_or(0));
            vo.collect_shop = Some(user_collect_shop_ids.contains(&shop_id));
            vo.selected = Some(
                q.selected_shop_id
                    .is_some_and(|sel| sel > 0 && sel == shop_id),
            );
            if let Some(star) = vo.shop_star {
                vo.shop_star = Some(round_half_up_1(star));
            }

            if location.is_some() {
                // Java 会用 `GpsDistanceUtil` 的结果覆盖 SQL 距离
                let dist = distances.get(&shop_id).copied().unwrap_or(Decimal::ZERO);
                vo.distance = Some(dist);
            }
            let Some(dist) = vo.distance else {
                continue;
            };
            if let Some(cfg) = local_cfgs.get(&shop_id) {
                // 门店运力二选一：切到第三方运力时自建 enabled 会被置 0；
                // 本仓库 O2O 关闭，`thirdPartyCarrierShopIds` 恒为空集
                if cfg.enabled == 1
                    && cfg.delivery_radius > Decimal::ZERO
                    && dist <= cfg.delivery_radius
                {
                    vo.in_local_delivery_range = true;
                }
            }
            if let Some(cfg) = home_cfgs.get(&shop_id) {
                if cfg.enabled == 1
                    && cfg.service_radius.is_some_and(|r| r > Decimal::ZERO && dist <= r)
                {
                    vo.in_home_service_range = true;
                }
            }
        }

        // Java：分页 total 为 0 但存在记录（自营占位/选中门店）时以记录数兜底
        let total = if !records.is_empty() && total == 0 {
            u64::try_from(records.len()).unwrap_or(0)
        } else {
            total
        };
        Ok(PageResult::new(total, records, &page))
    }

    // ————————————————————— 8. 门店详情 —————————————————————

    /// `GET /api/shop/shop/storeDetail?shopId=`：门店详情。
    ///
    /// 对应 Java `ShopServiceImpl#storeDetail`：
    /// - `shopId = 0` 为平台自营虚拟门店（暂停运营时报「店铺不存在」）；
    /// - 普通门店按 `StoreDetailVO` 回吐并附标签、是否收藏、距离、
    ///   同城配送/上门服务范围标记与营业执照图。
    pub async fn store_detail(
        &self,
        q: &StoreDetailQuery,
        user_id: i32,
    ) -> Result<StoreDetailVO> {
        let shop_id = q.shop_id.unwrap_or(SELF_OPERATED_SHOP_ID);

        if shop_id == SELF_OPERATED_SHOP_ID {
            let s = self.self_operated_settings().await?;
            if is_self_operated_suspended(&s) {
                return Err(Error::biz(SERVICE_DATA_ERROR, ERR_SHOP_NOT_EXIST));
            }
            let logo = self.resolve_self_operated_store_logo(&s).await?;
            let sales = self
                .repo()
                .shop_sales_by_shop_id(SELF_OPERATED_SHOP_ID)
                .await?;
            return Ok(StoreDetailVO {
                shop_id: Some(SELF_OPERATED_SHOP_ID),
                shop_type: Some(SHOP_TYPE_STORE),
                shop_title: Some(s.name.clone()),
                shop_logo: Some(logo),
                description: Some(s.description.clone()),
                status: s.status,
                shop_contact_config: Some(s.contact_configs.clone()),
                shop_open_close_config: s.open_close_config.clone(),
                shop_region_names: s.region_names.clone(),
                tip_names: Some(s.tip_names.clone()),
                shop_sales: Some(i32::try_from(sales).unwrap_or(i32::MAX)),
                collect_shop: Some(
                    self.repo()
                        .find_collect(user_id, SELF_OPERATED_SHOP_ID)
                        .await?
                        .is_some(),
                ),
                ..Default::default()
            });
        }

        let shop = self
            .repo()
            .find_shop(shop_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, ERR_SHOP_NOT_EXIST))?;

        let tips = self.repo().find_shop_store_tips(shop.shop_id).await?;
        let tip_ids: Vec<i32> = tips.iter().map(|t| t.tip_id).collect();
        let tip_name_map: HashMap<i32, String> = self
            .repo()
            .find_tips(&tip_ids)
            .await?
            .into_iter()
            .map(|t| (t.tip_id, t.tip_name))
            .collect();

        // Java：`shopStar != null` 时，评价数 <= 100 一律回退 4.8，再保留一位小数
        let shop_star = shop.shop_star.map(|star| {
            if shop.shop_star_num.unwrap_or(0) <= 100 {
                round_half_up_1(decimal("4.8"))
            } else {
                round_half_up_1(star)
            }
        });

        let mut vo = StoreDetailVO {
            shop_id: Some(shop.shop_id),
            shop_title: shop.shop_title.clone(),
            shop_logo: shop.shop_logo.clone(),
            shop_star,
            shop_star_num: shop.shop_star_num,
            shop_sales: shop.shop_sales,
            tip_names: Some(
                tips.iter()
                    .filter_map(|t| tip_name_map.get(&t.tip_id).cloned())
                    .collect(),
            ),
            business_license_pic: Some(self.approved_license_pic(shop.merchant_id).await?),
            shop_type: shop.shop_type,
            store_parent_id: shop.store_parent_id,
            shop_cover_picture: shop.shop_cover_picture.clone(),
            description: shop.description.clone(),
            status: shop.status,
            shop_detailed_address: shop.shop_detailed_address.clone(),
            shop_longitude: shop.shop_longitude,
            shop_latitude: shop.shop_latitude,
            ..Default::default()
        };

        vo.collect_shop = Some(
            self.repo()
                .find_collect(user_id, shop.shop_id)
                .await?
                .is_some(),
        );

        if let Some((lng, lat)) = valid_location(q.longitude, q.latitude) {
            let distances = self
                .repo()
                .gps_distances_for_shops(lng, lat, &[shop.shop_id])
                .await?;
            let distance = distances
                .get(&shop.shop_id)
                .copied()
                .unwrap_or(Decimal::ZERO);
            // Java 侧为 `BigDecimal#toString()`
            vo.distance = Some(distance.normalize().to_string());

            if q.delivery_types.contains(&4) {
                if let Some(cfg) = self
                    .repo()
                    .find_local_delivery_config(shop.shop_id)
                    .await?
                {
                    if cfg.enabled == 1
                        && cfg.delivery_radius > Decimal::ZERO
                        && distance <= cfg.delivery_radius
                    {
                        vo.in_local_delivery_range = true;
                    }
                }
            }
            if q.delivery_types.contains(&2) {
                if let Some(cfg) = self
                    .repo()
                    .find_home_service_config(shop.shop_id)
                    .await?
                {
                    if cfg.enabled == 1
                        && cfg
                            .service_radius
                            .is_some_and(|r| r > Decimal::ZERO && distance <= r)
                    {
                        vo.in_home_service_range = true;
                    }
                }
            }
        }
        Ok(vo)
    }

    // ————————————————————— 9. 店铺评论列表 —————————————————————

    /// `GET /api/shop/shop/getCommentList?page=&type=&shopId=`：店铺评论分页。
    ///
    /// 对应 Java `ShopClientController#getCommentList` →
    /// `CommentServiceImpl#listByShopId`：固定 `size = 10`、按 `isTop desc`
    /// + `addTime desc` 排序；`type = 5` 时仅取 `isShowed = 1`；
    /// `type` 2/3/4 分别对应好评/中评/差评的 `comment_rank` 过滤。
    pub async fn comment_list(&self, q: &ShopCommentListQuery) -> Result<PageResult<CommentVO>> {
        let page = q.page_query();
        let shop_id = q.shop_id.unwrap_or(SELF_OPERATED_SHOP_ID);
        let is_showed = if q.r#type == Some(5) { Some(1) } else { None };

        let (total, rows) = self
            .repo()
            .paginate_comments(shop_id, is_showed, q.r#type, &page)
            .await?;
        if rows.is_empty() {
            // Java 无数据时返回 `new Page<>()`（total = 0）
            return Ok(PageResult::new(0, Vec::new(), &page));
        }

        // 批量预取商品 / 子评论 / 订单项，避免 N+1
        let product_ids: Vec<i32> = rows
            .iter()
            .map(|c| c.product_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let product_map: HashMap<i32, Product> = self
            .repo()
            .find_products_by_ids(&product_ids)
            .await?
            .into_iter()
            .map(|p| (p.product_id, p))
            .collect();

        let comment_ids: Vec<i32> = rows.iter().map(|c| c.comment_id).collect();
        let replies = self.repo().find_comment_replies(&comment_ids).await?;
        let mut reply_map: HashMap<i32, Vec<Comment>> = HashMap::new();
        for r in replies {
            reply_map.entry(r.parent_id).or_default().push(r);
        }

        let order_item_ids: Vec<i32> = rows
            .iter()
            .map(|c| c.order_item_id)
            .filter(|id| *id > 0)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let order_item_map: HashMap<i32, (i32, String)> = self
            .repo()
            .find_order_items(&order_item_ids)
            .await?
            .into_iter()
            .map(|it| (it.item_id, (it.sku_id, it.sku_data)))
            .collect();

        let default_avatar = self
            .repo()
            .find_config_val(CFG_DEFAULT_AVATAR)
            .await?
            .unwrap_or_default();

        let mut list = Vec::with_capacity(rows.len());
        for c in &rows {
            list.push(build_comment_vo(
                c,
                product_map.get(&c.product_id),
                reply_map.get(&c.comment_id).map(Vec::as_slice),
                order_item_map.get(&c.order_item_id),
                &default_avatar,
            ));
        }
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 私有辅助 —————————————————————

    /// 平台自营门店配置（Java `ConfigServiceImpl#getStoreSelfOperatedSettings`）。
    pub async fn self_operated_settings(&self) -> Result<SelfOperatedSettings> {
        let map = self.repo().find_config_vals(&CFG_SELF_OPERATED).await?;

        let s = SelfOperatedSettings {
            name: cfg_str(&map, "storeSelfOperatedName", "").unwrap_or_default(),
            logo: cfg_str(&map, "storeSelfOperatedLogo", "").unwrap_or_default(),
            address: cfg_str(&map, "storeSelfOperatedAddress", "").unwrap_or_default(),
            longitude: cfg_str(&map, "storeSelfOperatedLongitude", "")
                .and_then(|v| Decimal::from_str(v.trim()).ok()),
            latitude: cfg_str(&map, "storeSelfOperatedLatitude", "")
                .and_then(|v| Decimal::from_str(v.trim()).ok()),
            phone: cfg_str(&map, "storeSelfOperatedPhone", "").unwrap_or_default(),
            description: cfg_str(&map, "storeSelfOperatedDescription", "").unwrap_or_default(),
            status: cfg_int(&map, "storeSelfOperatedStatus", "1"),
            contact_json: cfg_str(&map, "storeSelfOperatedContactJson", "[]").unwrap_or_default(),
            open_close_json: cfg_str(&map, "storeSelfOperatedOpenCloseJson", "")
                .unwrap_or_default(),
            tip_ids: parse_int_list_json(
                cfg_str(&map, "storeSelfOperatedTipsJson", "[]").as_deref(),
            ),
            ..Default::default()
        };

        // 门店地域：`storeSelfOperatedRegion` 为逗号分隔的 regionId
        let region_raw = cfg_str(&map, "storeSelfOperatedRegion", "").unwrap_or_default();
        let region_ids = split_ints(&region_raw);
        let region_names = if region_raw.trim().is_empty() {
            None
        } else {
            Some(self.region_names(&region_ids).await?.unwrap_or_default())
        };

        let contact_configs = parse_contact_configs(Some(s.contact_json.as_str())).unwrap_or_default();
        // 解析器入参为 `Option<&str>`：空串在解析器内部被过滤为 `None`
        let open_close_config = parse_open_close_config(Some(s.open_close_json.as_str()));
        let tip_names = if s.tip_ids.is_empty() {
            Vec::new()
        } else {
            self.repo()
                .find_tips(&s.tip_ids)
                .await?
                .into_iter()
                .map(|t| t.tip_name)
                .collect()
        };

        Ok(SelfOperatedSettings {
            region_names,
            contact_configs,
            open_close_config,
            tip_names,
            ..s
        })
    }

    /// `resolveSelfOperatedStoreLogo`：门店 logo 为空时回退全站 `shopLogo`。
    async fn resolve_self_operated_store_logo(&self, s: &SelfOperatedSettings) -> Result<String> {
        if !s.logo.trim().is_empty() {
            return Ok(s.logo.clone());
        }
        Ok(self
            .repo()
            .find_config_val(CFG_SHOP_LOGO)
            .await?
            .unwrap_or_default())
    }

    /// `storeShowOtherCityStore = 1`：显示地区其他门店。
    async fn store_show_other_city(&self) -> Result<bool> {
        Ok(self
            .repo()
            .find_config_val(CFG_STORE_SHOW_OTHER_CITY_STORE)
            .await?
            .as_deref()
            == Some("1"))
    }

    /// 地区 ID → 名称（`RegionServiceImpl#getRegionNamesByRegionIds`：
    /// 入参为空返回 `null`，否则返回命中的名称列表）。
    async fn region_names(&self, ids: &[i32]) -> Result<Option<Vec<String>>> {
        if ids.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            self.repo()
                .find_region_names(ids)
                .await?
                .into_iter()
                .map(|(_, name)| name)
                .collect(),
        ))
    }

    /// `MerchantServiceImpl#getApprovedLicensePic`：营业执照图 URL，无有效资料返回空串。
    async fn approved_license_pic(&self, merchant_id: Option<i32>) -> Result<String> {
        let Some(mid) = merchant_id.filter(|m| *m > 0) else {
            return Ok(String::new());
        };
        let Some(merchant) = self.repo().find_merchant(mid).await? else {
            return Ok(String::new());
        };
        if merchant.is_delete == Some(1) {
            return Ok(String::new());
        }
        if let Some(apply_id) = merchant.merchant_apply_id.filter(|v| *v > 0) {
            let status = self.repo().find_merchant_apply_status(apply_id).await?;
            if status != Some(MERCHANT_APPLY_AUDIT_PASS) {
                return Ok(String::new());
            }
        }
        let Some(raw) = merchant.merchant_data.as_deref().filter(|s| !s.trim().is_empty()) else {
            return Ok(String::new());
        };
        let Ok(data) = serde_json::from_str::<serde_json::Value>(raw) else {
            return Ok(String::new());
        };
        let Some(pics) = data.get("businessLicenseImg").and_then(|v| v.as_array()) else {
            return Ok(String::new());
        };
        for pic in pics {
            let url = pic
                .get("picUrl")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .or_else(|| {
                    pic.get("url")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                });
            if let Some(url) = url {
                return Ok(url.to_string());
            }
        }
        Ok(String::new())
    }

    /// 平台自营门店列表占位（`fillSelfOperatedStoreListVO`）。
    async fn build_self_operated_store_vo(&self, s: &SelfOperatedSettings) -> Result<StoreListVO> {
        let logo = self.resolve_self_operated_store_logo(s).await?;
        Ok(StoreListVO {
            shop_id: Some(SELF_OPERATED_SHOP_ID),
            shop_title: Some(s.name.clone()),
            shop_logo: Some(logo),
            status: s.status,
            description: Some(s.description.clone()),
            contact_configs: Some(s.contact_configs.clone()),
            tip_names: Some(s.tip_names.clone()),
            shop_open_close_config: if s.open_close_json.trim().is_empty() {
                None
            } else {
                parse_open_close_config(Some(s.open_close_json.as_str()))
            },
            ..Default::default()
        })
    }
}

// ————————————————————— 纯函数（可单测） —————————————————————

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `SimpleDateFormat`，
/// `None` 按 0 处理 → 本地时区的 1970-01-01；Java 侧 null 会 NPE）。
#[must_use]
pub fn fmt_datetime(secs: Option<i32>) -> String {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(secs.unwrap_or(0)), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`：`0` → 空串）。
#[must_use]
pub fn handel_time(secs: i64) -> String {
    if secs == 0 {
        return String::new();
    }
    fmt_datetime(Some(i32::try_from(secs).unwrap_or(i32::MAX)))
}

/// 店铺状态文案（Java `ShopType#getTypeName`：未知状态抛「数据类型异常」/1001）。
pub fn shop_status_text(status: Option<i32>) -> Result<String> {
    match status {
        Some(STATUS_OPEN) => Ok("开业".into()),
        Some(STATUS_PAUSE) => Ok("停业".into()),
        Some(STATUS_CLOSE) => Ok("打烊".into()),
        _ => Err(Error::biz(SERVICE_DATA_ERROR, ERR_DATA_TYPE)),
    }
}

/// 商家类型文案（Java `MerchantType#getTypeName`：未知类型抛「数据类型异常」/1001）。
pub fn merchant_type_text(r#type: Option<i32>) -> Result<String> {
    match r#type {
        Some(1) => Ok("个人认证".into()),
        Some(2) => Ok("企业认证".into()),
        _ => Err(Error::biz(SERVICE_DATA_ERROR, ERR_DATA_TYPE)),
    }
}

/// `4.8`（Java `new BigDecimal("4.8")`）。
#[must_use]
pub fn decimal(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap_or(Decimal::ZERO)
}

/// 评星（Java `ShopMapper#calculateShopStar`：评价数 < 100/空/0 → 4.8，
/// 否则 `shop_star`；`shop_star` 为空时 Java 侧返回 `null`）。
#[must_use]
pub fn calculated_shop_star(shop: &Shop) -> Option<Decimal> {
    let star_num = shop.shop_star_num.unwrap_or(0);
    if star_num < 100 {
        Some(decimal("4.8"))
    } else {
        shop.shop_star
    }
}

/// 保留一位小数，`HALF_UP`（Java `setScale(1, RoundingMode.HALF_UP)`）。
#[must_use]
pub fn round_half_up_1(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(1, RoundingStrategy::MidpointAwayFromZero)
}

/// 经纬度是否有效（Java 只判纬度 `!= null && != 0`）。
#[must_use]
pub fn valid_location(lng: Option<Decimal>, lat: Option<Decimal>) -> Option<(Decimal, Decimal)> {
    match lat {
        Some(lat) if lat != Decimal::ZERO => Some((lng.unwrap_or(Decimal::ZERO), lat)),
        _ => None,
    }
}

/// Java `ShopServiceImpl#shouldInsertSelfOperatedStore`。
#[must_use]
pub fn should_insert_self_operated(
    shop_ids: &[i32],
    exclude_self_operated: Option<i32>,
    only_local_delivery: bool,
) -> bool {
    if only_local_delivery {
        return false;
    }
    if !shop_ids.is_empty() && shop_ids.contains(&SELF_OPERATED_SHOP_ID) {
        return true;
    }
    if !shop_ids.is_empty() {
        return false;
    }
    // 默认排除；前端显式传 0 才纳入自营
    exclude_self_operated == Some(0)
}

/// Java `ShopServiceImpl#isSelfOperatedStoreSuspended`（暂停运营不展示）。
#[must_use]
pub fn is_self_operated_suspended(s: &SelfOperatedSettings) -> bool {
    s.status == Some(STATUS_PAUSE)
}

/// `Shop` 实体 → `ClientShopVO`（不含需要批量预取的字段）。
fn build_client_shop_vo(shop: &Shop) -> Result<ClientShopVO> {
    Ok(ClientShopVO {
        status_text: Some(shop_status_text(shop.status)?),
        shop_id: Some(shop.shop_id),
        shop_title: shop.shop_title.clone(),
        add_time: Some(fmt_datetime(shop.add_time)),
        shop_logo: shop.shop_logo.clone(),
        click_count: shop.click_count,
        status: shop.status,
        merchant_id: shop.merchant_id,
        shop_money: shop.shop_money,
        frozen_money: shop.frozen_money,
        contact_mobile: shop.contact_mobile.clone(),
        description: shop.description.clone(),
        kefu_phone: shop.kefu_phone.clone(),
        kefu_weixin: shop.kefu_weixin.clone(),
        kefu_link: shop.kefu_link.clone(),
        is_contact_kefu: Some(shop.is_contact_kefu),
        kefu_inlet: shop.kefu_inlet.clone(),
        shop_star: calculated_shop_star(shop),
        shop_type: shop.shop_type,
        shop_detailed_address: shop.shop_detailed_address.clone(),
        shop_longitude: shop.shop_longitude,
        shop_latitude: shop.shop_latitude,
        ..Default::default()
    })
}

/// `StoreListRow` → `StoreListVO`（占位行由调用方另行构造）。
fn store_row_to_vo(row: &StoreListRow) -> StoreListVO {
    StoreListVO {
        shop_id: Some(row.shop_id),
        shop_logo: row.shop_logo.clone(),
        shop_title: row.shop_title.clone(),
        status: row.status,
        address: row.address.clone(),
        distance: row.distance,
        longitude: row.longitude,
        latitude: row.latitude,
        shop_star: row.shop_star,
        shop_star_num: row.shop_star_num,
        shop_sales: row.shop_sales,
        description: row.description.clone(),
        contact_json: row.contact_json.clone(),
        open_close_json: row.open_close_json.clone(),
        ..Default::default()
    }
}

/// 选中门店置顶 VO（Java `storeList` 内联构造）。
fn selected_store_to_vo(shop: &Shop) -> StoreListVO {
    let star = match shop.shop_star_num {
        Some(num) if num >= 100 => shop.shop_star,
        _ => Some(decimal("4.8")),
    };
    StoreListVO {
        shop_id: Some(shop.shop_id),
        shop_logo: shop.shop_logo.clone(),
        shop_title: shop.shop_title.clone(),
        status: shop.status,
        address: shop.shop_detailed_address.clone(),
        longitude: shop.shop_longitude,
        latitude: shop.shop_latitude,
        shop_star: star,
        shop_star_num: shop.shop_star_num,
        description: shop.description.clone(),
        shop_sales: shop.shop_sales,
        contact_json: shop.shop_contact_json.clone(),
        open_close_json: shop.shop_open_close_json.clone(),
        ..Default::default()
    }
}

/// `Comment` → `CommentVO`（Java `CommentVO(comment, product, replies)` 构造器）。
fn build_comment_vo(
    c: &Comment,
    product: Option<&Product>,
    replies: Option<&[Comment]>,
    order_item: Option<&(i32, String)>,
    default_avatar: &str,
) -> CommentVO {
    let (sku_id, sku_data) = match order_item {
        Some((sku_id, sku_data)) => (
            Some(*sku_id),
            serde_json::from_str::<serde_json::Value>(sku_data)
                .ok()
                .filter(|v| v.is_array())
                .and_then(|v| v.as_array().cloned()),
        ),
        None => (None, None),
    };
    CommentVO {
        comment_id: Some(c.comment_id),
        user_id: Some(c.user_id),
        username: Some(c.username.clone()),
        is_anonymous: Some(c.is_anonymous),
        avatar: Some(if c.avatar.is_empty() {
            default_avatar.to_string()
        } else {
            c.avatar.clone()
        }),
        product_id: Some(c.product_id),
        order_id: Some(c.order_id),
        order_item_id: Some(c.order_item_id),
        comment_rank: Some(c.comment_rank),
        content: Some(c.content.clone()),
        status: Some(c.status),
        parent_id: Some(c.parent_id),
        usefull: Some(c.usefull),
        useless: Some(c.useless),
        is_recommend: c.is_recommend,
        is_top: Some(c.is_top),
        is_showed: Some(c.is_showed),
        is_default: Some(c.is_default),
        sort_order: Some(c.sort_order),
        shop_id: Some(c.shop_id),
        product_name: product.map(|p| p.product_name.clone()),
        product_sn: product.map(|p| p.product_sn.clone()),
        pic_thumb: product.map(|p| p.pic_thumb.clone()),
        replies: Some(
            replies
                .unwrap_or_default()
                .iter()
                .map(|r| CommentReplyVO {
                    comment_id: Some(r.comment_id),
                    user_id: Some(r.user_id),
                    username: Some(r.username.clone()),
                    is_anonymous: Some(r.is_anonymous),
                    content: Some(r.content.clone()),
                    parent_id: Some(r.parent_id),
                    add_time: Some(handel_time(i64::from(r.add_time))),
                })
                .collect(),
        ),
        add_time: Some(handel_time(i64::from(c.add_time))),
        comment_tag: Some(parse_string_list_json(c.comment_tag.as_deref())),
        show_pics: parse_show_pics(c.show_pics.as_deref()),
        kefu_name: None,
        sku_id,
        sku_data,
    }
}

/// 配置项原始值（Java `SettingsHelper#getRaw`：命中返回 `biz_val`
/// ——可能为 `NULL`；未命中返回枚举默认值）。
fn cfg_str(map: &HashMap<String, Option<String>>, code: &str, default: &str) -> Option<String> {
    match map.get(code) {
        Some(Some(v)) => Some(v.clone()),
        Some(None) => None,
        None => Some(default.to_string()),
    }
}

/// 配置项整数值（Java `SettingsHelper#parse(INTEGER, ..)`：
/// 空白 → `0`，非法值 → `null`）。
#[must_use]
pub fn cfg_int(
    map: &HashMap<String, Option<String>>,
    code: &str,
    default: &str,
) -> Option<i32> {
    match cfg_str(map, code, default) {
        None => Some(0),
        Some(v) => {
            let t = v.trim();
            if t.is_empty() {
                Some(0)
            } else {
                t.parse::<i32>().ok()
            }
        }
    }
}

/// JSON 数组串 → `Vec<i32>`（Java `JSONUtil.toList(json, Integer.class)`）。
#[must_use]
pub fn parse_int_list_json(raw: Option<&str>) -> Vec<i32> {
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return Vec::new();
    };
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| match v {
                serde_json::Value::Number(n) => n.as_i64().and_then(|i| i32::try_from(i).ok()),
                serde_json::Value::String(s) => s.trim().parse::<i32>().ok(),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// JSON 数组串 → `Vec<String>`（评价标签；非 JSON 时 Java 返回空列表）。
#[must_use]
pub fn parse_string_list_json(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return Vec::new();
    };
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// 联系方式 JSON → `Vec<ShopContactConfigVO>`（Java `JSONUtil.toList`）。
#[must_use]
pub fn parse_contact_configs(raw: Option<&str>) -> Option<Vec<ShopContactConfigVO>> {
    let raw = raw.filter(|s| !s.trim().is_empty())?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let items = value.as_array()?;
    Some(
        items
            .iter()
            .filter_map(|v| {
                let obj = v.as_object()?;
                Some(ShopContactConfigVO {
                    r#type: obj
                        .get("type")
                        .and_then(serde_json::Value::as_i64)
                        .and_then(|i| i32::try_from(i).ok()),
                    values: obj
                        .get("values")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                })
            })
            .collect(),
    )
}

/// 营业时间 JSON → `ShopOpenCloseConfigVO`（Java `JSONUtil.toBean`）。
#[must_use]
pub fn parse_open_close_config(raw: Option<&str>) -> Option<ShopOpenCloseConfigVO> {
    let raw = raw.filter(|s| !s.trim().is_empty())?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let obj = value.as_object()?;
    let times = obj
        .get("times")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let o = t.as_object()?;
                    Some(ShopTimeRangeVO {
                        start: o.get("start").and_then(|v| v.as_str()).map(str::to_string),
                        end: o.get("end").and_then(|v| v.as_str()).map(str::to_string),
                        day_of_week: o
                            .get("dayOfWeek")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        ranges: o.get("ranges").and_then(|v| v.as_array()).map(|rs| {
                            rs.iter()
                                .filter_map(|r| {
                                    let ro = r.as_object()?;
                                    Some(ShopTimeRangeStartEndVO {
                                        start: ro
                                            .get("start")
                                            .and_then(|v| v.as_str())
                                            .map(str::to_string),
                                        end: ro
                                            .get("end")
                                            .and_then(|v| v.as_str())
                                            .map(str::to_string),
                                    })
                                })
                                .collect()
                        }),
                    })
                })
                .collect()
        });
    Some(ShopOpenCloseConfigVO {
        r#type: obj
            .get("type")
            .and_then(serde_json::Value::as_i64)
            .and_then(|i| i32::try_from(i).ok()),
        times,
    })
}

/// 晒图 JSON → `Vec<CommentShowPicVO>`（Java `getCommentShowPicsByJson`：
/// `[]`/空/非 JSON → `null`；`{...}` → 单元素列表）。
#[must_use]
pub fn parse_show_pics(raw: Option<&str>) -> Option<Vec<CommentShowPicVO>> {
    let raw = raw.filter(|s| !s.trim().is_empty() && s.trim() != "[]")?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let items = match value {
        serde_json::Value::Array(items) => items,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return None,
    };
    let out: Vec<CommentShowPicVO> = items
        .iter()
        .filter_map(|v| {
            let o = v.as_object()?;
            Some(CommentShowPicVO {
                pic_id: o
                    .get("picId")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|i| i32::try_from(i).ok()),
                pic_thumb: o.get("picThumb").and_then(|v| v.as_str()).map(str::to_string),
                pic_url: o.get("picUrl").and_then(|v| v.as_str()).map(str::to_string),
                pic_name: o.get("picName").and_then(|v| v.as_str()).map(str::to_string),
                storage_url: o
                    .get("storageUrl")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                errcode: o
                    .get("errcode")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|i| i32::try_from(i).ok()),
                message: o.get("message").and_then(|v| v.as_str()).map(str::to_string),
            })
        })
        .collect();
    Some(out)
}

/// JSON 字符串 → `serde_json::Value`（失败返回 `None`）。
fn parse_json_opt(raw: Option<&str>) -> Option<serde_json::Value> {
    serde_json::from_str(raw.filter(|s| !s.trim().is_empty())?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop(status: Option<i32>, star_num: Option<i32>, star: Option<Decimal>) -> Shop {
        Shop {
            shop_id: 7,
            status,
            shop_star_num: star_num,
            shop_star: star,
            ..Default::default()
        }
    }

    #[test]
    fn status_text_matches_java_enum() {
        assert_eq!(shop_status_text(Some(1)).unwrap(), "开业");
        assert_eq!(shop_status_text(Some(4)).unwrap(), "停业");
        assert_eq!(shop_status_text(Some(10)).unwrap(), "打烊");
        // 未知状态与 null 均抛「数据类型异常」/1001
        let err = shop_status_text(Some(3)).unwrap_err();
        assert_eq!(err.business_code(), 1001);
        assert_eq!(shop_status_text(None).unwrap_err().business_code(), 1001);
    }

    #[test]
    fn merchant_type_text_matches_java_enum() {
        assert_eq!(merchant_type_text(Some(1)).unwrap(), "个人认证");
        assert_eq!(merchant_type_text(Some(2)).unwrap(), "企业认证");
        assert_eq!(
            merchant_type_text(Some(9)).unwrap_err().business_code(),
            1001
        );
    }

    #[test]
    fn shop_star_falls_back_to_4_8_below_hundred_reviews() {
        let s = shop(Some(1), Some(99), Some(decimal("3.2")));
        assert_eq!(calculated_shop_star(&s), Some(decimal("4.8")));
        // 评价数恰好 100 → 取 shop_star
        let s = shop(Some(1), Some(100), Some(decimal("3.2")));
        assert_eq!(calculated_shop_star(&s), Some(decimal("3.2")));
        // shop_star 为空时保持 null（与 Java 一致）
        let s = shop(Some(1), Some(100), None);
        assert_eq!(calculated_shop_star(&s), None);
        let s = shop(Some(1), None, Some(decimal("3.2")));
        assert_eq!(calculated_shop_star(&s), Some(decimal("4.8")));
    }

    #[test]
    fn round_half_up_keeps_one_decimal() {
        assert_eq!(round_half_up_1(decimal("4.85")), decimal("4.9"));
        assert_eq!(round_half_up_1(decimal("4.84")), decimal("4.8"));
        assert_eq!(round_half_up_1(decimal("4")), decimal("4.0"));
    }

    #[test]
    fn valid_location_requires_non_zero_latitude() {
        assert_eq!(valid_location(Some(decimal("115.1")), Some(decimal("28.2"))),
            Some((decimal("115.1"), decimal("28.2"))));
        assert_eq!(valid_location(Some(decimal("115.1")), Some(Decimal::ZERO)), None);
        assert_eq!(valid_location(Some(decimal("115.1")), None), None);
        // 经度缺失时按 0 补齐（Java `getLongitude()` 缺省 0）
        assert_eq!(
            valid_location(None, Some(decimal("28.2"))),
            Some((Decimal::ZERO, decimal("28.2")))
        );
    }

    #[test]
    fn should_insert_self_operated_matches_java_rules() {
        // 配送过滤开启 → 不插
        assert!(!should_insert_self_operated(&[], Some(0), true));
        // 显式 shopIds 含 0 → 插
        assert!(should_insert_self_operated(&[0, 3], Some(1), false));
        // 显式 shopIds 非空不含 0 → 不插
        assert!(!should_insert_self_operated(&[3], Some(0), false));
        // 无 shopIds：仅显式 excludeSelfOperated=0
        assert!(should_insert_self_operated(&[], Some(0), false));
        assert!(!should_insert_self_operated(&[], Some(1), false));
        assert!(!should_insert_self_operated(&[], None, false));
    }

    #[test]
    fn self_operated_suspended_only_on_pause() {
        let s = SelfOperatedSettings {
            status: Some(4),
            ..Default::default()
        };
        assert!(is_self_operated_suspended(&s));
        let s = SelfOperatedSettings {
            status: Some(1),
            ..Default::default()
        };
        assert!(!is_self_operated_suspended(&s));
    }

    #[test]
    fn handel_time_is_empty_for_zero() {
        assert_eq!(handel_time(0), "");
        assert_eq!(handel_time(1_700_000_000).len(), 19);
    }

    #[test]
    fn cfg_int_uses_enum_default_and_blank_zero() {
        let mut map: HashMap<String, Option<String>> = HashMap::new();
        // 未命中 → 枚举默认值 "1"
        assert_eq!(cfg_int(&map, "storeSelfOperatedStatus", "1"), Some(1));
        // 空串 → 0
        map.insert("storeSelfOperatedStatus".into(), Some(String::new()));
        assert_eq!(cfg_int(&map, "storeSelfOperatedStatus", "1"), Some(0));
        // 非法值 → null
        map.insert("storeSelfOperatedStatus".into(), Some("abc".into()));
        assert_eq!(cfg_int(&map, "storeSelfOperatedStatus", "1"), None);
        // 行存在但值为 NULL → 0
        map.insert("storeSelfOperatedStatus".into(), None);
        assert_eq!(cfg_int(&map, "storeSelfOperatedStatus", "1"), Some(0));
    }

    #[test]
    fn parse_contact_configs_reads_type_and_values() {
        let v = parse_contact_configs(Some(r#"[{"type":3,"values":"138-0000"}]"#)).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].r#type, Some(3));
        assert_eq!(v[0].values.as_deref(), Some("138-0000"));
        assert!(parse_contact_configs(Some("")).is_none());
        assert!(parse_contact_configs(None).is_none());
    }

    #[test]
    fn parse_open_close_config_reads_times() {
        let raw = r#"{"type":2,"times":[{"start":"09:00","end":"18:00"}]}"#;
        let cfg = parse_open_close_config(Some(raw)).unwrap();
        assert_eq!(cfg.r#type, Some(2));
        assert_eq!(cfg.times.as_ref().unwrap()[0].start.as_deref(), Some("09:00"));
        assert!(parse_open_close_config(Some("")).is_none());
    }

    #[test]
    fn parse_show_pics_handles_array_object_and_empty() {
        assert!(parse_show_pics(Some("[]")).is_none());
        assert!(parse_show_pics(Some("")).is_none());
        assert!(parse_show_pics(None).is_none());
        let arr = parse_show_pics(Some(r#"[{"picId":1,"picUrl":"a.png"}]"#)).unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].pic_url.as_deref(), Some("a.png"));
        // 单对象 → 单元素列表
        let one = parse_show_pics(Some(r#"{"picId":2,"picUrl":"b.png"}"#)).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].pic_id, Some(2));
    }

    #[test]
    fn parse_int_list_json_tolerates_junk() {
        assert_eq!(parse_int_list_json(Some("[1,2,3]")), vec![1, 2, 3]);
        assert_eq!(parse_int_list_json(Some("[]")), Vec::<i32>::new());
        assert_eq!(parse_int_list_json(Some("not json")), Vec::<i32>::new());
        assert_eq!(parse_int_list_json(None), Vec::<i32>::new());
    }

    #[test]
    fn parse_string_list_json_returns_empty_for_non_json() {
        assert_eq!(
            parse_string_list_json(Some(r#"["好评","快"]"#)),
            vec!["好评".to_string(), "快".to_string()]
        );
        assert!(parse_string_list_json(Some("xyz")).is_empty());
        assert!(parse_string_list_json(None).is_empty());
    }

    #[test]
    fn comment_vo_falls_back_to_default_avatar_and_counts() {
        let c = Comment {
            comment_id: 11,
            user_id: 5,
            username: "u".into(),
            avatar: String::new(),
            product_id: 3,
            order_item_id: 9,
            comment_rank: 5,
            content: "ok".into(),
            add_time: 1_700_000_000,
            parent_id: 0,
            shop_id: 2,
            ..Default::default()
        };
        let vo = build_comment_vo(&c, None, None, Some(&(7, "[{\"name\":\"色\"}]".into())), "default.png");
        assert_eq!(vo.avatar.as_deref(), Some("default.png"));
        assert_eq!(vo.sku_id, Some(7));
        assert_eq!(vo.sku_data.as_ref().unwrap().len(), 1);
        assert!(vo.replies.as_ref().unwrap().is_empty());
        assert_eq!(vo.add_time.as_deref().map(str::len), Some(19));
        // 非数组 skuData → null
        let vo = build_comment_vo(&c, None, None, Some(&(7, "{}".into())), "d.png");
        assert!(vo.sku_data.is_none());
    }

    #[test]
    fn build_client_shop_vo_maps_star_and_times() {
        let s = Shop {
            shop_id: 1,
            shop_title: Some("店".into()),
            status: Some(1),
            add_time: Some(1_700_000_000),
            shop_star_num: Some(10),
            shop_star: Some(decimal("3.0")),
            ..Default::default()
        };
        let vo = build_client_shop_vo(&s).unwrap();
        assert_eq!(vo.shop_star, Some(decimal("4.8")));
        assert_eq!(vo.status_text.as_deref(), Some("开业"));
        assert_eq!(vo.add_time.as_deref().map(str::len), Some(19));
    }
}