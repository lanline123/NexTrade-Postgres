//! o2o H5 前台服务（o2o::pickup_client）。
//!
//! b2b2b 5.8.57 移动端新增接口（无 Java 控制器对应）：自提人信息 CRUD、
//! 门店自提模板/订单自提单读取、到店预约库存日历。

use rust_decimal::Decimal;
use serde_json::Value;
use nextrade_core::entities::ShopLocalDeliveryConfig;
use nextrade_core::types::PageQuery;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::merchant::shop_client::{StoreListFilter, StoreListRow};
use nextrade_infrastructure::db::repo::o2o::pickup_client::StoreSearchCandidateRow;
use nextrade_infrastructure::db::Repositories;
pub use nextrade_infrastructure::db::repo::o2o::pickup_client::{
    OrderPickupShipmentVO, ShopPickupTplVO, StaffContext, StockDayVO, UserPickupVO,
};

/// 门店聚合搜索参数（`getStoreProduct` 门店维度模式，我方新增接线；
/// 字段与前端 `StoreSearchParams` 契约一一对应）。
#[derive(Debug, Clone, Default)]
pub struct StoreSearchParams {
    pub keyword: String,
    pub longitude: Option<Decimal>,
    pub latitude: Option<Decimal>,
    pub region_id: Option<String>,
    pub group_ids: Vec<i32>,
    pub shop_ids: Vec<i32>,
    pub delivery_types: Vec<i32>,
    pub within_delivery_range: bool,
    pub page: u64,
    pub size: u64,
}

/// 配置项 `regionIsolation`（我方新增，就近门店二期帖 357858；
/// 与 `services::merchant::shop_client` 的同名常量为同一 biz_code）。
const CFG_REGION_ISOLATION: &str = "regionIsolation";

#[derive(Clone)]
pub struct PickupClientService {
    repos: std::sync::Arc<Repositories>,
}

impl PickupClientService {
    pub fn new(repos: std::sync::Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 当前会员自提人列表。
    pub async fn list_pickups(&self, user_id: i32) -> Result<Vec<UserPickupVO>> {
        self.repos.o2o.pickup_client.list_by_user(user_id).await
    }

    /// 自提人详情（校验归属）。
    pub async fn pickup_detail(&self, user_id: i32, id: i32) -> Result<Option<UserPickupVO>> {
        self.repos.o2o.pickup_client.find_owned(user_id, id).await
    }

    /// 新增自提人。
    pub async fn create_pickup(
        &self,
        user_id: i32,
        consignee: &str,
        mobile: &str,
        area_code: &str,
        is_default: bool,
    ) -> Result<i32> {
        if consignee.trim().is_empty() || mobile.trim().is_empty() {
            return Err(nextrade_core::Error::Validation("联系人与手机号不能为空".into()));
        }
        self.repos
            .o2o
            .pickup_client
            .create(user_id, consignee.trim(), mobile.trim(), area_code.trim(), is_default)
            .await
    }

    /// 更新自提人。
    #[allow(clippy::too_many_arguments)]
    pub async fn update_pickup(
        &self,
        user_id: i32,
        id: i32,
        consignee: Option<&str>,
        mobile: Option<&str>,
        area_code: Option<&str>,
        is_default: Option<bool>,
    ) -> Result<()> {
        if self.repos.o2o.pickup_client.find_owned(user_id, id).await?.is_none() {
            return Err(nextrade_core::Error::NotFound("自提人信息不存在".into()));
        }
        self.repos
            .o2o
            .pickup_client
            .update(user_id, id, consignee, mobile, area_code, is_default)
            .await
    }

    /// 删除自提人。
    pub async fn delete_pickup(&self, user_id: i32, id: i32) -> Result<()> {
        self.repos.o2o.pickup_client.delete(user_id, id).await
    }

    /// 门店自提模板（门店级 → 平台默认回退）。
    pub async fn shop_pickup_tpl(&self, shop_id: i64) -> Result<Option<ShopPickupTplVO>> {
        self.repos.o2o.pickup_client.find_pickup_tpl(shop_id).await
    }

    /// 订单自提配送单。
    pub async fn order_shipment(
        &self,
        order_id: i32,
    ) -> Result<Option<OrderPickupShipmentVO>> {
        self.repos.o2o.pickup_client.find_shipment_by_order(order_id).await
    }

    /// 门店商品搜索（分页）。
    pub async fn store_product_search(
        &self,
        shop_id: i64,
        keyword: &str,
        page: u64,
        size: u64,
    ) -> Result<(i64, Vec<serde_json::Value>)> {
        let offset = (page.max(1) - 1) * size;
        self.repos
            .o2o
            .pickup_client
            .store_product_search(shop_id, keyword, size, offset)
            .await
    }

    /// 门店聚合搜索（`getStoreProduct` 门店维度模式，我方新增接线）：
    /// 按门店聚合的搜索结果。候选门店复用门店列表口径
    /// （`paginate_stores`：门店类型/状态/地区/配送方式/配送范围），
    /// 带有效坐标时按距离升序——最近门店在前（帖 377777/567745）；
    /// 无坐标时沿用门店列表的评星序（无定位降级，行为与改造前
    /// 一致）。每店商品 = 关键词命中（最新序，最多 10）在前 + 未命中
    /// 按有效价升序补齐，命中总数以 `matchedProductCount` 返回。
    /// 候选商品含门店自有商品与 `store_product` 分配商品。
    pub async fn store_product_search_aggregate(
        &self,
        params: &StoreSearchParams,
    ) -> Result<(i64, Vec<serde_json::Value>)> {
        let shop_repo = &self.repos.merchant.shop_client;
        // 显式 shopIds / groupIds → 候选门店白名单（未传 = 不限制）
        let restrict: Option<Vec<i32>> = if !params.shop_ids.is_empty() {
            Some(params.shop_ids.clone())
        } else if !params.group_ids.is_empty() {
            Some(shop_repo.find_shop_ids_by_groups(&params.group_ids).await?)
        } else {
            None
        };
        if restrict.as_ref().is_some_and(Vec::is_empty) {
            // 显式指定了分组/门店但解析为空 → 无候选门店（不能放行为全部门店）
            return Ok((0, Vec::new()));
        }
        let location =
            crate::merchant::shop_client::valid_location(params.longitude, params.latitude);
        let page = PageQuery {
            page: params.page.max(1),
            page_size: params.size.clamp(1, 50),
        };
        // 区域硬隔离（二期，帖 357858）：开启且带数值地区时，候选
        // 门店改走精确成员匹配（与 storeList 同口径，经
        // `harden_region` 收口）；关闭（缺省）时行为与改造前一致。
        let isolation = shop_repo
            .find_config_val(CFG_REGION_ISOLATION)
            .await?
            .as_deref()
            .map(str::trim)
            == Some("1");
        let (region_id, region_hard) =
            crate::merchant::shop_client::harden_region(isolation, params.region_id.clone());
        let filter = StoreListFilter {
            keyword: None,
            region_id,
            region_hard,
            shop_ids: restrict.unwrap_or_default(),
            delivery_types: params.delivery_types.clone(),
            within_delivery_range: params.within_delivery_range && location.is_some(),
            selected_shop_id: None,
            sort_field: None,
            sort_order: None,
        };
        let (total, rows) = shop_repo.paginate_stores(&filter, location, &page).await?;
        if rows.is_empty() {
            return Ok((total as i64, Vec::new()));
        }
        let shop_ids: Vec<i32> = rows.iter().map(|r| r.shop_id).collect();
        // 展示距离以 GpsDistanceUtil 口径覆盖（与 storeList 一致）
        let mut distances: std::collections::HashMap<i32, Decimal> = Default::default();
        if let Some((lng, lat)) = location {
            distances = shop_repo.gps_distances_for_shops(lng, lat, &shop_ids).await?;
        }
        let local_cfgs: std::collections::HashMap<i32, ShopLocalDeliveryConfig> = shop_repo
            .find_local_delivery_configs(&shop_ids)
            .await?
            .into_iter()
            .map(|c| (c.shop_id, c))
            .collect();
        let candidates = self
            .repos
            .o2o
            .pickup_client
            .store_search_candidates(&shop_ids, &params.keyword)
            .await?;
        let mut by_shop: std::collections::HashMap<i32, Vec<&StoreSearchCandidateRow>> =
            Default::default();
        for c in &candidates {
            by_shop.entry(c.shop_id).or_default().push(c);
        }
        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            let shop_rows: Vec<&StoreSearchCandidateRow> = by_shop
                .get(&row.shop_id)
                .map(|v| v.iter().copied().collect())
                .unwrap_or_default();
            let (matched, products) = assemble_store_products(&shop_rows);
            let distance = distances.get(&row.shop_id).copied().or(row.distance);
            records.push(build_store_search_record(
                row,
                local_cfgs.get(&row.shop_id),
                distance,
                matched,
                products,
            ));
        }
        Ok((total as i64, records))
    }

    /// 到店预约库存日历（未来 days 天）。
    pub async fn stock_calendar(
        &self,
        product_id: i32,
        shop_id: Option<i64>,
        days: u32,
    ) -> Result<Vec<StockDayVO>> {
        self.repos
            .o2o
            .pickup_client
            .stock_calendar(product_id, shop_id, days)
            .await
    }

    /// 当前会员绑定的服务人员。
    pub async fn delivery_staff(&self, member_id: i32) -> Result<Option<(i32, String, String, i32)>> {
        self.repos.o2o.pickup_client.staff_by_member(member_id).await
    }

    /// 当前会员绑定的服务人员（完整上下文：店铺作用域 + 状态）。
    pub async fn staff_context(&self, member_id: i32) -> Result<Option<StaffContext>> {
        self.repos.o2o.pickup_client.staff_context_by_member(member_id).await
    }

    pub async fn staff_info(&self, member_id: i32) -> Result<Option<Value>> {
        self.repos.o2o.pickup_client.staff_info_by_member(member_id).await
    }

    pub async fn update_work_status(&self, member_id: i32, status: i32) -> Result<bool> {
        self.repos.o2o.pickup_client.update_work_status(member_id, status).await
    }

    pub async fn my_wallet(&self, staff_id: i32) -> Result<Value> {
        self.repos.o2o.pickup_client.wallet_by_staff(staff_id).await
    }

    pub async fn my_wallet_logs(&self, staff_id: i32, page: u64, size: u64) -> Result<(i64, Vec<Value>)> {
        self.repos.o2o.pickup_client.wallet_logs(staff_id, page, size).await
    }

    pub async fn accounts(&self, user_id: i32) -> Result<Vec<Value>> {
        self.repos.o2o.pickup_client.accounts_of_user(user_id).await
    }

    pub async fn save_account(
        &self,
        user_id: i32,
        account_id: i32,
        account_type: i32,
        account_name: &str,
        account_no: &str,
        bank_name: &str,
    ) -> Result<i32> {
        self.repos
            .o2o
            .pickup_client
            .save_account(user_id, account_id, account_type, account_name, account_no, bank_name)
            .await
    }

    pub async fn delete_account(&self, user_id: i32, account_id: i32) -> Result<()> {
        self.repos.o2o.pickup_client.delete_account(user_id, account_id).await
    }

    pub async fn withdraw_list(
        &self,
        staff_id: i32,
        status: Option<i32>,
        page: u64,
        size: u64,
    ) -> Result<(i64, Vec<Value>)> {
        self.repos.o2o.pickup_client.withdraw_list(staff_id, status, page, size).await
    }

    pub async fn withdraw_detail(&self, staff_id: i32, id: i32) -> Result<Option<Value>> {
        self.repos.o2o.pickup_client.withdraw_detail(staff_id, id).await
    }

    pub async fn apply_withdraw(
        &self,
        staff_id: i32,
        business_type: i32,
        amount: rust_decimal::Decimal,
        account_id: i32,
        remark: &str,
    ) -> Result<i32> {
        self.repos
            .o2o
            .pickup_client
            .apply_withdraw(staff_id, business_type, amount, account_id, remark)
            .await
    }

    pub async fn local_delivery_config(&self, shop_id: i32) -> Result<Value> {
        self.repos.o2o.pickup_client.local_delivery_config(shop_id).await
    }

    pub async fn home_service_config(&self, shop_id: i32) -> Result<Value> {
        self.repos.o2o.pickup_client.home_service_config(shop_id).await
    }

    /// C 端派送订单列表。
    pub async fn delivery_list(
        &self,
        staff_id: i32,
        tab: Option<&str>,
        page: u64,
        size: u64,
    ) -> Result<(i64, Vec<Value>)> {
        self.repos.o2o.pickup_client.delivery_list(staff_id, tab, page, size).await
    }

    pub async fn delivery_detail(&self, staff_id: i32, id: i32) -> Result<Option<Value>> {
        self.repos.o2o.pickup_client.delivery_detail(staff_id, id).await
    }

    /// 抢单（作用域校验在仓储原子 UPDATE 内：同店 + 同类 + 待派发）。
    pub async fn grab_delivery(&self, staff: &StaffContext, id: i32) -> Result<bool> {
        self.repos.o2o.pickup_client.grab_delivery(staff, id).await
    }

    pub async fn confirm_pickup(&self, staff_id: i32, id: i32, photos: &str) -> Result<bool> {
        self.repos.o2o.pickup_client.confirm_pickup(staff_id, id, photos).await
    }

    /// 完成配送/服务 + 服务费入账。`None` = 状态不允许；`Some(fee)` = 已入账金额。
    pub async fn complete_delivery(
        &self,
        staff_id: i32,
        id: i32,
        photos: &str,
    ) -> Result<Option<rust_decimal::Decimal>> {
        self.repos.o2o.pickup_client.complete_delivery(staff_id, id, photos).await
    }

    pub async fn cancel_delivery(&self, staff_id: i32, id: i32, reason: &str) -> Result<bool> {
        self.repos.o2o.pickup_client.cancel_delivery(staff_id, id, reason).await
    }

    /// 抢单大厅（按人员店铺作用域 + 类型过滤；调用方须先验人员身份）。
    pub async fn grab_list(
        &self,
        staff: &StaffContext,
        page: u64,
        size: u64,
        r#type: Option<i32>,
    ) -> Result<(i64, Vec<Value>)> {
        self.repos.o2o.pickup_client.grab_list(staff, page, size, r#type).await
    }

    /// 买家侧派送进度（归属校验由路由层以订单 user_id 做）。
    pub async fn buyer_delivery(&self, order_id: i32) -> Result<Option<Value>> {
        self.repos.o2o.pickup_client.buyer_delivery_by_order(order_id).await
    }

    /// 提交入驻申请（骑手 MVP 阶段二）。
    #[allow(clippy::too_many_arguments)]
    pub async fn staff_apply(
        &self,
        member_id: i32,
        shop_id: i32,
        apply_type: i32,
        name: &str,
        phone: &str,
        id_card: &str,
        remark: &str,
    ) -> Result<i32> {
        if shop_id <= 0 {
            return Err(nextrade_core::Error::Validation("请选择入驻店铺".into()));
        }
        if !matches!(apply_type, 1 | 2) {
            return Err(nextrade_core::Error::Validation("申请类型不正确".into()));
        }
        if name.trim().is_empty() || phone.trim().is_empty() {
            return Err(nextrade_core::Error::Validation("姓名与手机号不能为空".into()));
        }
        self.repos
            .o2o
            .pickup_client
            .create_apply(member_id, shop_id, apply_type, name.trim(), phone.trim(), id_card.trim(), remark.trim())
            .await
    }

    /// 我的入驻申请列表。
    pub async fn my_applies(&self, member_id: i32) -> Result<Vec<Value>> {
        self.repos.o2o.pickup_client.applies_of_member(member_id).await
    }

    /// 主订单归属会员（买家进度归属校验）。
    pub async fn order_owner(&self, order_id: i32) -> Result<Option<i32>> {
        self.repos.o2o.pickup_client.order_owner(order_id).await
    }

    /// 状态动作点定位打点（best-effort，失败不影响业务动作）。
    pub async fn record_track_point(
        &self,
        staff_id: i32,
        delivery_order_id: i32,
        action: &str,
        longitude: Option<rust_decimal::Decimal>,
        latitude: Option<rust_decimal::Decimal>,
    ) -> Result<()> {
        self.repos
            .o2o
            .pickup_client
            .record_track_point(staff_id, delivery_order_id, action, longitude, latitude)
            .await
    }
}

/// 商品有效价：门店覆盖价（`store_product.product_price`）优先，
/// 否则商品本体价。
fn effective_price(r: &StoreSearchCandidateRow) -> Option<Decimal> {
    r.store_price.or(r.product_price)
}

/// 单店商品组装（纯函数）：命中在前（保持仓储返回的 `product_id`
/// 降序）、未命中按有效价升序补齐，合计最多 10 条；返回
/// (同店命中总数, 商品 JSON 列表)。命中总数取候选行回填的
/// `hit_total`（截断前的真实总数）。
fn assemble_store_products(rows: &[&StoreSearchCandidateRow]) -> (i64, Vec<serde_json::Value>) {
    let matched = rows.first().map_or(0, |r| r.hit_total);
    let mut fills: Vec<&&StoreSearchCandidateRow> =
        rows.iter().filter(|r| !r.is_hit).collect();
    fills.sort_by(|a, b| {
        effective_price(a)
            .partial_cmp(&effective_price(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let hits = rows.iter().filter(|r| r.is_hit);
    let mut out: Vec<serde_json::Value> = Vec::with_capacity(10);
    for r in hits.chain(fills.into_iter()) {
        if out.len() >= 10 {
            break;
        }
        out.push(serde_json::json!({
            "productId": r.product_id,
            "productName": r.product_name,
            "picThumb": r.pic_thumb,
            "picUrl": r.pic_url,
            "productPrice": effective_price(r),
        }));
    }
    (matched, out)
}

/// 单店聚合记录组装（纯函数，字段对齐前端 `StoreSearchRecord`）。
/// 同城配送三项（开关/起送价/基础运费）取 `shop_local_delivery_config`
/// 且 `enabled = 1` 时才有值；`inLocalDeliveryRange` 与门店列表
/// `storeList` 口径一致：`enabled = 1`、半径 > 0 且展示距离
/// （米）≤ 半径数值直比。单位已实证同为米（就近门店二期专项
/// 核查：原 schema「公里」注释系误标，实体注释已更正）。
fn build_store_search_record(
    row: &StoreListRow,
    cfg: Option<&ShopLocalDeliveryConfig>,
    distance: Option<Decimal>,
    matched: i64,
    products: Vec<serde_json::Value>,
) -> serde_json::Value {
    let local = cfg.filter(|c| c.enabled == 1);
    let in_range = match (local, distance) {
        (Some(c), Some(d)) => c.delivery_radius > Decimal::ZERO && d <= c.delivery_radius,
        _ => false,
    };
    serde_json::json!({
        "shopId": row.shop_id,
        "shopName": row.shop_title,
        "shopLogo": row.shop_logo,
        "shopAddress": row.address,
        "shopRegionNames": Vec::<String>::new(),
        "longitude": row.longitude,
        "latitude": row.latitude,
        "distance": distance,
        "shopStar": row.shop_star,
        "shopSales": row.shop_sales,
        "localDeliveryEnabled": local.is_some(),
        "minOrderAmount": local.map(|c| c.min_order_amount),
        "deliveryFee": local.map(|c| c.base_delivery_fee),
        "inLocalDeliveryRange": in_range,
        "matchedProductCount": matched,
        "productList": products,
    })
}

#[cfg(test)]
mod store_search_tests {
    use super::*;

    fn cand(
        product_id: i32,
        price: Option<i64>,
        store_price: Option<i64>,
        is_hit: bool,
        hit_total: i64,
    ) -> StoreSearchCandidateRow {
        StoreSearchCandidateRow {
            shop_id: 1,
            product_id,
            product_name: format!("商品{product_id}"),
            pic_thumb: String::new(),
            pic_url: String::new(),
            product_price: price.map(|v| Decimal::from(v)),
            store_price: store_price.map(|v| Decimal::from(v)),
            is_hit,
            hit_total,
        }
    }

    #[test]
    fn hits_first_then_cheapest_fill_capped_at_10() {
        let rows_owned: Vec<StoreSearchCandidateRow> = vec![
            cand(30, Some(300), None, true, 2),
            cand(20, Some(200), None, true, 2),
            cand(9, Some(90), None, false, 2),
            cand(8, Some(50), None, false, 2),
            cand(7, Some(70), None, false, 2),
        ];
        let rows: Vec<&StoreSearchCandidateRow> = rows_owned.iter().collect();
        let (matched, products) = assemble_store_products(&rows);
        assert_eq!(matched, 2);
        let ids: Vec<i64> = products
            .iter()
            .map(|p| p["productId"].as_i64().unwrap())
            .collect();
        // 命中（保持入参序）→ 未命中按价升序 50、70、90
        assert_eq!(ids, vec![30, 20, 8, 7, 9]);
    }

    #[test]
    fn store_price_overrides_product_price() {
        let row = cand(1, Some(100), Some(80), true, 1);
        assert_eq!(effective_price(&row), Some(Decimal::from(80)));
        let (_, products) = assemble_store_products(&[&row]);
        // 工作区 rust_decimal 启用 serde-with-str：Decimal 序列化为字符串
        assert_eq!(products[0]["productPrice"], serde_json::json!("80"));
    }

    #[test]
    fn empty_candidates_yield_zero_matched() {
        let (matched, products) = assemble_store_products(&[]);
        assert_eq!(matched, 0);
        assert!(products.is_empty());
    }
}
