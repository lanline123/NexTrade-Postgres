//! o2o H5 前台服务（o2o::pickup_client）。
//!
//! b2b2b 5.8.57 移动端新增接口（无 Java 控制器对应）：自提人信息 CRUD、
//! 门店自提模板/订单自提单读取、到店预约库存日历。

use serde_json::Value;
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;
pub use nextrade_infrastructure::db::repo::o2o::pickup_client::{
    OrderPickupShipmentVO, ShopPickupTplVO, StockDayVO, UserPickupVO,
};

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

    pub async fn grab_delivery(&self, staff_id: i32, name: &str, phone: &str, id: i32) -> Result<bool> {
        self.repos.o2o.pickup_client.grab_delivery(staff_id, name, phone, id).await
    }

    pub async fn confirm_pickup(&self, staff_id: i32, id: i32, photos: &str) -> Result<bool> {
        self.repos.o2o.pickup_client.confirm_pickup(staff_id, id, photos).await
    }

    pub async fn complete_delivery(&self, staff_id: i32, id: i32, photos: &str) -> Result<bool> {
        self.repos.o2o.pickup_client.complete_delivery(staff_id, id, photos).await
    }

    pub async fn cancel_delivery(&self, staff_id: i32, id: i32, reason: &str) -> Result<bool> {
        self.repos.o2o.pickup_client.cancel_delivery(staff_id, id, reason).await
    }

    pub async fn grab_list(&self, page: u64, size: u64, r#type: Option<i32>) -> Result<(i64, Vec<Value>)> {
        self.repos.o2o.pickup_client.grab_list(page, size, r#type).await
    }
}
