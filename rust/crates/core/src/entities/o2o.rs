//! 门店 O2O 域实体（自提/同城配送/预约）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `area_store_manager`, `area_store_manager_shop`, `delivery_order`, `order_pickup_item`, `order_pickup_shipment`, `pos_device`, `pos_pending_order`, `product_reservation_stock`, `product_reservation_time`, `reservation_tpl`, `shop_home_service_config`, `shop_local_delivery_config`, `shop_pickup_config`, `shop_pickup_tpl`, `store_product`, `store_product_cycle_spec`, `store_product_sku`, `store_tip`, `tip`, `user_pickup_info`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 区域门店管理
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AreaStoreManager {
    /// 区域门店管理 [NOT NULL]
    pub area_store_manager_id: i32,
    /// 区域门店名称 [NOT NULL]
    pub area_store_name: String,
    /// 排序
    pub sort_order: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
}

impl AreaStoreManager {
    /// 数据库表名。
    pub const TABLE: &'static str = "area_store_manager";
}


/// 区域门店绑定门店表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AreaStoreManagerShop {
    /// 区域门店绑定id [NOT NULL]
    pub area_store_manager_shop_id: i32,
    /// 区域门店ID [NOT NULL]
    pub area_store_manager_id: i32,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
}

impl AreaStoreManagerShop {
    /// 数据库表名。
    pub const TABLE: &'static str = "area_store_manager_shop";
}


/// 派送订单表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct DeliveryOrder {
    /// 派送订单ID [NOT NULL]
    pub delivery_order_id: i32,
    /// 派送订单编号 [NOT NULL]
    pub delivery_order_sn: String,
    /// 关联商城订单ID [NOT NULL]
    pub order_id: i32,
    /// 关联商城订单编号 [NOT NULL]
    pub order_sn: String,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// 类型：1-配送订单，2-服务订单 [NOT NULL]
    pub r#type: i32,
    /// 派单模式：1-抢单模式，2-手动派单 [NOT NULL]
    pub dispatch_mode: i32,
    /// 状态：10-待派发，20-待取货/待服务，30-配送中/服务中，40-已完成，50-已取消 [NOT NULL]
    pub status: i32,
    /// 配送员/服务员ID
    pub staff_id: Option<i32>,
    /// 配送员/服务员姓名
    pub staff_name: Option<String>,
    /// 配送员/服务员电话
    pub staff_phone: Option<String>,
    /// 收货人/服务对象姓名 [NOT NULL]
    pub consignee: String,
    /// 收货人/服务对象电话 [NOT NULL]
    pub mobile: String,
    /// 配送地址/服务地址 [NOT NULL]
    pub address: String,
    /// 地区名称（省市区）
    pub region_names: Option<String>,
    /// 配送地址/服务地址纬度
    pub latitude: Option<Decimal>,
    /// 配送地址/服务地址经度
    pub longitude: Option<Decimal>,
    /// 取货地址（门店地址） 省市区
    pub pickup_region_names: Option<String>,
    /// 取货地址（门店地址）
    pub pickup_address: Option<String>,
    /// 取货电话
    pub pickup_phone: Option<String>,
    /// 取货地址纬度
    pub pickup_latitude: Option<Decimal>,
    /// 取货地址经度
    pub pickup_longitude: Option<Decimal>,
    /// 预约服务时间（Unix时间戳）
    pub service_time: Option<i32>,
    /// 订单金额 [NOT NULL]
    pub order_amount: Decimal,
    /// 买家备注
    pub buyer_note: Option<String>,
    /// 派单时间（Unix时间戳）
    pub dispatch_time: Option<i32>,
    /// 抢单时间（Unix时间戳）
    pub grab_time: Option<i32>,
    /// 取货时间/开始服务时间（Unix时间戳）
    pub pickup_time: Option<i32>,
    /// 完成时间（Unix时间戳）
    pub complete_time: Option<i32>,
    /// 取消时间（Unix时间戳）
    pub cancel_time: Option<i32>,
    /// 取消原因
    pub cancel_reason: Option<String>,
    /// 取消方：1-配送员/服务员，2-后台管理员，3-系统
    pub cancel_by: Option<i32>,
    /// 取货照片（JSON数组）
    pub pickup_photo: Option<String>,
    /// 完成照片（JSON数组）
    pub complete_photo: Option<String>,
    /// 服务前照片（JSON数组）
    pub before_service_photo: Option<String>,
    /// 服务后照片（JSON数组）
    pub after_service_photo: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间（Unix时间戳） [NOT NULL]
    pub create_time: i32,
    /// 更新时间（Unix时间戳） [NOT NULL]
    pub update_time: i32,
    /// 是否删除：0-否，1-是 [NOT NULL]
    pub is_delete: i32,
    /// 运力类型 1自建配送 2美团配送 [NOT NULL]
    pub carrier_type: i32,
    /// 合作方配送单号（发给美团的 delivery_id，本系统唯一）
    pub mt_delivery_id: Option<String>,
    /// 美团配送单号（美团返回的 mt_peisong_id）
    pub mt_peisong_id: Option<String>,
    /// 美团订单状态 0待调度 20已接单 30已取货 50已送达 99已取消
    pub mt_status: Option<i32>,
    /// 美团配送服务代码（飞速达/快速达/及时达等）
    pub mt_delivery_service_code: Option<String>,
    /// 美团取消/异常代码
    pub mt_cancel_code: Option<i32>,
    /// 美团配送距离（米）
    pub mt_distance: Option<i32>,
    /// 美团配送费（元）
    pub mt_fee: Option<Decimal>,
}

impl DeliveryOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "delivery_order";
}


/// 订单提货关联表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderPickupItem {
    /// [NOT NULL]
    pub order_pickup_id: i32,
    /// 门店id
    pub shop_id: Option<i32>,
    /// 提货点id
    pub pickup_id: Option<i32>,
    /// 订单id
    pub order_id: Option<i32>,
    /// 自提时间
    pub expect_pickup_time: Option<String>,
    /// 用户自提信息
    pub user_pickup_id: Option<i32>,
}

impl OrderPickupItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_pickup_item";
}


/// order_pickup_shipment
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderPickupShipment {
    /// [NOT NULL]
    pub shipment_id: i32,
    /// 订单id
    pub order_id: Option<i32>,
    /// 门店id
    pub shop_id: Option<i32>,
    /// 自提点id
    pub pickup_id: Option<i32>,
    /// 提货码
    pub pickup_sn: Option<String>,
    /// 提货二维码
    pub pickup_qr_code: Option<String>,
    /// 发货状态
    pub shipment_status: Option<i32>,
    /// 核销时间
    pub check_time: Option<i32>,
}

impl OrderPickupShipment {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_pickup_shipment";
}


/// 收银台设备
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PosDevice {
    /// [NOT NULL]
    pub id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 设备名称 [NOT NULL]
    pub device_name: String,
    /// 店铺内唯一设备码 [NOT NULL]
    pub device_code: String,
    /// 1启用 2停用 [NOT NULL]
    pub status: i32,
    /// 最后使用时间
    pub last_used_time: Option<i64>,
    /// [NOT NULL]
    pub add_time: i64,
    /// [NOT NULL]
    pub update_time: i64,
}

impl PosDevice {
    /// 数据库表名。
    pub const TABLE: &'static str = "pos_device";
}




/// 产品预约日历库存
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductReservationStock {
    /// [NOT NULL]
    pub product_reservation_stock_id: i64,
    /// 产品ID [NOT NULL]
    pub product_id: i32,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// SKU ID [NOT NULL]
    pub sku_id: i32,
    /// 预约时间段ID，对应 product_reservation_time.id [NOT NULL]
    pub product_reservation_time_id: i32,
    /// 库存日期，比如 2025-09-18 [NOT NULL]
    pub stock_date: String,
    /// 总库存数 [NOT NULL]
    pub total_stock: i32,
    /// 已用库存数 [NOT NULL]
    pub used_stock: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl ProductReservationStock {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_reservation_stock";
}


/// 产品预约时间段配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductReservationTime {
    /// 主键ID [NOT NULL]
    pub product_reservation_time_id: i32,
    /// 产品ID [NOT NULL]
    pub product_id: i32,
    /// 开始时间，格式HH:mm [NOT NULL]
    pub start_time: String,
    /// 结束时间，格式HH:mm [NOT NULL]
    pub end_time: String,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl ProductReservationTime {
    /// 数据库表名。
    pub const TABLE: &'static str = "product_reservation_time";
}


/// 产品预约配置
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ReservationTpl {
    /// 预约模板ID [NOT NULL]
    pub reservation_tpl_id: i32,
    /// 模板名称 [NOT NULL]
    pub template_name: String,
    /// [NOT NULL]
    pub reservation_type: i32,
    /// 需提前预约的天数，仅当reservation_type=1时有效
    pub advance_number: Option<i32>,
    pub advance_unit: Option<String>,
    /// 最长允许预约天数 [NOT NULL]
    pub max_days: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 是否默认，1表示默认 [NOT NULL]
    pub is_default: i32,
}

impl ReservationTpl {
    /// 数据库表名。
    pub const TABLE: &'static str = "reservation_tpl";
}


/// 门店预约上门服务配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopHomeServiceConfig {
    /// 主键ID [NOT NULL]
    pub shop_home_service_config_id: i32,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// 是否开启预约上门服务：0-关闭，1-开启 [NOT NULL]
    pub enabled: i32,
    /// 服务半径（公里）
    pub service_radius: Option<Decimal>,
    /// 上门费（元） [NOT NULL]
    pub home_fee: Decimal,
    /// 是否开启距离溢价：0-关闭，1-开启 [NOT NULL]
    pub distance_pricing_enabled: i32,
    /// 派单模式：1-抢单模式，2-手动派单 [NOT NULL]
    pub order_dispatch_mode: i32,
    /// 是否开启服务前拍照：0-关闭，1-开启 [NOT NULL]
    pub before_service_photo_enabled: i32,
    /// 是否开启服务后拍照：0-关闭，1-开启 [NOT NULL]
    pub after_service_photo_enabled: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl ShopHomeServiceConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_home_service_config";
}


/// 门店同城配送配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopLocalDeliveryConfig {
    /// 自增ID
    pub shop_local_delivery_config_id: Option<i64>,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// 是否开启同城配送 0=关闭 1=开启 [NOT NULL]
    pub enabled: i32,
    /// 配送半径（公里） [NOT NULL]
    pub delivery_radius: Decimal,
    /// 起送价（元） [NOT NULL]
    pub min_order_amount: Decimal,
    /// 基础运费（元） [NOT NULL]
    pub base_delivery_fee: Decimal,
    /// 是否开启叠加溢价 0=关闭 1=开启 [NOT NULL]
    pub distance_pricing_enabled: i32,
    /// 派单模式：1-抢单模式，2-手动派单 [NOT NULL]
    pub order_dispatch_mode: i32,
    /// 是否开启取货时拍照：0-关闭，1-开启 [NOT NULL]
    pub pickup_photo_enabled: i32,
    /// 是否开启配送完成拍照：0-关闭，1-开启 [NOT NULL]
    pub delivery_complete_photo_enabled: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl ShopLocalDeliveryConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_local_delivery_config";
}


/// 门店自提配置表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopPickupConfig {
    /// ID
    pub shop_pickup_config_id: Option<i64>,
    /// 门店ID，关联shop表 [NOT NULL]
    pub shop_id: i64,
    /// 自定义名称（该名称会显示在买家可选择的配送方式中，默认名称"到店自提"）
    pub pickup_name: Option<String>,
    /// 状态：0-禁用，1-启用 [NOT NULL]
    pub status: i32,
    /// 提货字段设置
    pub pickup_filed: Option<String>,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
}

impl ShopPickupConfig {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_pickup_config";
}


/// 门店自提模板表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopPickupTpl {
    /// ID
    pub shop_pickup_tpl_id: Option<i64>,
    /// 门店ID，关联shop表 [NOT NULL]
    pub shop_id: i64,
    /// 模板名称 [NOT NULL]
    pub tpl_name: String,
    /// 完成备货状态：0-禁用，1-启用 [NOT NULL]
    pub stocking_status: i32,
    /// 完成备货JSON
    pub stocking_json: Option<String>,
    /// 自提时间状态：0-禁用，1-启用 [NOT NULL]
    pub pickup_time_status: i32,
    /// 自提时间JSON
    pub pickup_time_json: Option<String>,
    /// 提货有效期状态：0-禁用，1-启用 [NOT NULL]
    pub pickup_end_status: i32,
    /// 提货有效期JSON
    pub pickup_end_json: Option<String>,
    /// 是否默认 [NOT NULL]
    pub is_default: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i64,
    /// 更新时间 [NOT NULL]
    pub update_time: i64,
}

impl ShopPickupTpl {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_pickup_tpl";
}


/// 分配店铺商品
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StoreProduct {
    /// ID [NOT NULL]
    pub store_product_id: i64,
    /// 商品id
    pub product_id: Option<i32>,
    /// 商品名称
    pub product_name: Option<String>,
    /// 商品价格
    pub product_price: Option<Decimal>,
    /// 商品状态；1-上架，0-下架 [NOT NULL]
    pub product_status: i32,
    /// 产品库存
    pub product_stock: Option<i32>,
    /// 门店id [NOT NULL]
    pub shop_id: i64,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
    /// 关联的电子卡券分组
    pub card_group_id: Option<i32>,
    /// 是否删除
    pub is_delete: Option<i32>,
    /// 店铺分类id
    pub shop_category_id: Option<i32>,
    /// 预约模板Id
    pub reservation_tpl_id: Option<i32>,
    /// 配送员服务费类型（门店覆盖，仅单规格商品使用）：0=固定金额 1=按比例 [NOT NULL]
    pub delivery_service_fee_type: i32,
    /// 配送员服务费值（门店覆盖，仅单规格商品使用） [NOT NULL]
    pub delivery_service_fee_value: Decimal,
    /// 服务员服务费类型（门店覆盖，仅单规格商品使用）：0=固定金额 1=按比例 [NOT NULL]
    pub server_service_fee_type: i32,
    /// 服务员服务费值（门店覆盖，仅单规格商品使用） [NOT NULL]
    pub server_service_fee_value: Decimal,
    /// 门店覆盖主图（config=1时使用，NULL表示未覆盖）
    pub pic_url: Option<String>,
    /// 门店覆盖详情页（config=1时使用，NULL表示未覆盖）
    pub product_desc: Option<String>,
}

impl StoreProduct {
    /// 数据库表名。
    pub const TABLE: &'static str = "store_product";
}


/// 门店周期购/次卡价格覆盖表（门店独立售价）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StoreProductCycleSpec {
    /// 门店周期价覆盖ID [NOT NULL]
    pub store_cycle_spec_id: i32,
    /// 门店ID [NOT NULL]
    pub shop_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// SKU ID（统一设置时为0，与 product_cycle_spec.sku_id 的 NULL 对齐） [NOT NULL]
    pub sku_id: i32,
    /// 周期规格（期数），与 product_cycle_spec.period_num 对齐 [NOT NULL]
    pub period_num: i32,
    /// 门店覆盖周期价（实际售价） [NOT NULL]
    pub cycle_price: Decimal,
    /// 门店覆盖划线价 [NOT NULL]
    pub original_price: Decimal,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl StoreProductCycleSpec {
    /// 数据库表名。
    pub const TABLE: &'static str = "store_product_cycle_spec";
}


/// 分配店铺 sku
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StoreProductSku {
    /// ID [NOT NULL]
    pub store_product_sku_id: i64,
    /// 商品ID [NOT NULL]
    pub product_id: i64,
    /// 商品 sku ID [NOT NULL]
    pub product_sku_id: i64,
    /// 商品价格
    pub sku_price: Option<Decimal>,
    /// sku库存
    pub sku_stock: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
    /// 门店商品id [NOT NULL]
    pub store_product_id: i64,
    /// 配送员服务费类型（门店覆盖） [NOT NULL]
    pub delivery_service_fee_type: i32,
    /// 配送员服务费值（门店覆盖） [NOT NULL]
    pub delivery_service_fee_value: Decimal,
    /// 服务员服务费类型（门店覆盖） [NOT NULL]
    pub server_service_fee_type: i32,
    /// 服务员服务费值（门店覆盖） [NOT NULL]
    pub server_service_fee_value: Decimal,
}

impl StoreProductSku {
    /// 数据库表名。
    pub const TABLE: &'static str = "store_product_sku";
}


/// 门店关联标签
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StoreTip {
    /// 门店标签id [NOT NULL]
    pub store_tip_id: i32,
    /// 门店标签id [NOT NULL]
    pub tip_id: i32,
    /// 门店id [NOT NULL]
    pub shop_id: i32,
    /// 排序
    pub sort: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
}

impl StoreTip {
    /// 数据库表名。
    pub const TABLE: &'static str = "store_tip";
}


/// 标签
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Tip {
    /// 门店标签id [NOT NULL]
    pub tip_id: i32,
    /// 门店标签名称 [NOT NULL]
    pub tip_name: String,
    /// 0 禁用，1 启用
    pub status: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub add_time: i64,
}

impl Tip {
    /// 数据库表名。
    pub const TABLE: &'static str = "tip";
}


///  用户自提信息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserPickupInfo {
    /// 自增id [NOT NULL]
    pub user_pickup_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 收货人的名字 [NOT NULL]
    pub consignee: String,
    /// 收货人的手机号 [NOT NULL]
    pub mobile: String,
    /// 区号
    pub mobile_area_code: Option<String>,
    /// 是否默认 [NOT NULL]
    pub is_default: i32,
    /// 是否选中 [NOT NULL]
    pub is_selected: i32,
}

impl UserPickupInfo {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_pickup_info";
}

/// POS 桌台区
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PosTableArea {
    pub area_id: i32,
    pub shop_id: i32,
    pub area_name: String,
    /// 旧字段，部分库无此列
    pub sort: Option<i32>,
    /// v5.8.61 排序
    pub sort_order: Option<i32>,
    pub status: Option<i32>,
    pub pic_url: Option<String>,
    pub area_desc: Option<String>,
    pub is_delete: Option<i32>,
    pub add_time: Option<i32>,
    pub update_time: Option<i32>,
}

impl PosTableArea {
    pub const TABLE: &'static str = "pos_table_area";
}

/// POS 桌台
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PosTable {
    pub table_id: i32,
    pub shop_id: i32,
    pub area_id: i32,
    pub table_no: String,
    pub seat_count: i32,
    pub sort_order: i32,
    /// 旧版占用标记 0 空闲 1 占用
    pub status: i32,
    /// 0 未开台 1 待下单 2 待结账 3 待清台
    pub table_status: i32,
    pub dining_count: i32,
    pub user_id: i32,
    pub open_admin_id: i32,
    pub open_time: i32,
    pub session_sn: String,
    pub last_order_id: i32,
    pub transfer_from_table_no: String,
    pub is_delete: i32,
    pub add_time: Option<i32>,
    pub update_time: Option<i32>,
}

impl PosTable {
    pub const TABLE: &'static str = "pos_table";
}

/// POS 挂单（复用线上 pos_pending_order，扩展 kind/table_id 两列）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PosPendingOrder {
    pub pending_id: i32,
    pub user_id: i32,
    pub admin_id: i32,
    pub shop_id: Option<i32>,
    pub remark: Option<String>,
    /// 0 挂单中 / 1 已取单 / 2 已取消 / 3 已过期
    pub status: Option<i32>,
    pub add_time: Option<i32>,
    pub update_time: Option<i32>,
    /// 0 工作草稿 1 已保存挂单（002 迁移新增列）
    #[sqlx(default)]
    pub kind: i32,
    #[sqlx(default)]
    pub table_id: i32,
}

impl PosPendingOrder {
    pub const TABLE: &'static str = "pos_pending_order";
}

/// POS 挂单明细（002 迁移新增表）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PosPendingOrderItem {
    pub item_id: i64,
    pub pending_id: i32,
    pub product_id: i64,
    pub sku_id: i64,
    pub product_name: Option<String>,
    pub pic_thumb: Option<String>,
    pub price: Decimal,
    pub origin_price: Decimal,
    pub quantity: i32,
    pub is_checked: i32,
    pub sku_data: Option<String>,
    pub add_time: Option<i32>,
}

impl PosPendingOrderItem {
    pub const TABLE: &'static str = "pos_pending_order_item";
}

/// POS 收银员（线上 admin_user_pos 表）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminUserPos {
    pub id: i32,
    pub admin_id: i32,
    pub shop_id: i32,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub auth_list: Option<String>,
    pub is_using: i32,
    pub add_time: Option<i32>,
    pub role_id: i32,
}

impl AdminUserPos {
    pub const TABLE: &'static str = "admin_user_pos";
}
