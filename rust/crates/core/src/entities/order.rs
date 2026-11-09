//! 订单域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `aftersales`, `aftersales_item`, `aftersales_log`, `logistics_api_log`, `order`, `order_amount_detail`, `order_coupon_detail`, `order_cycle`, `order_cycle_card`, `order_cycle_card_redeem`, `order_cycle_delivery`, `order_invoice`, `order_item`, `order_log`, `order_presale`, `order_reservations`, `order_split_log`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 售后申请表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Aftersales {
    /// [NOT NULL]
    pub aftersale_id: i32,
    /// 申请类型：1、退货退款，2、换货，3、维修，4、其它，5，仅退款 [NOT NULL]
    pub aftersale_type: i32,
    /// [NOT NULL]
    pub status: i32,
    /// 用户凭证 [NOT NULL]
    pub pics: String,
    /// 描述 [NOT NULL]
    pub description: String,
    /// 回复 [NOT NULL]
    pub reply: String,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 物流编号 [NOT NULL]
    pub tracking_no: String,
    /// 物流公司 [NOT NULL]
    pub logistics_name: String,
    /// 退款地址和联系电话。
    pub return_address: Option<String>,
    /// 售后原因
    pub aftersale_reason: Option<String>,
    /// 售后编号
    pub aftersales_sn: Option<String>,
    /// 订单id
    pub order_id: Option<i32>,
    /// 用户id
    pub user_id: Option<i32>,
    /// 可退款金额 [NOT NULL]
    pub refund_amount: Decimal,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 审核时间 [NOT NULL]
    pub audit_time: i32,
    /// 处理时间 [NOT NULL]
    pub deal_time: i32,
    /// 完结时间 [NOT NULL]
    pub final_time: i32,
    /// 售后退货提示 [NOT NULL]
    pub return_goods_tip: String,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 关闭/拒绝子类型：0 无，1 商家审核驳回，2 商家或供应商拒收 [NOT NULL]
    pub close_type: i32,
}

impl Aftersales {
    /// 数据库表名。
    pub const TABLE: &'static str = "aftersales";
}


/// 售后申请商品表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AftersalesItem {
    /// [NOT NULL]
    pub aftersales_item_id: i32,
    /// 订单商品id
    pub order_item_id: Option<i32>,
    /// 申请数量
    pub number: Option<i32>,
    /// 售后申请id
    pub aftersale_id: Option<i32>,
}

impl AftersalesItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "aftersales_item";
}


/// 售后申请日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AftersalesLog {
    /// [NOT NULL]
    pub log_id: i32,
    /// 售后申请id [NOT NULL]
    pub aftersale_id: i32,
    /// 申请原因 [NOT NULL]
    pub log_info: String,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 后台操作人 [NOT NULL]
    pub admin_name: String,
    /// 退款金额 [NOT NULL]
    pub refund_money: Decimal,
    /// 售后类型 [NOT NULL]
    pub refund_type: i32,
    /// 售后描述 [NOT NULL]
    pub refund_desc: String,
    /// 用户名
    pub user_name: Option<String>,
    /// 回复图片
    pub return_pic: Option<String>,
    /// shopId
    pub shop_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
}

impl AftersalesLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "aftersales_log";
}


/// 快递接口调用日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct LogisticsApiLog {
    /// [NOT NULL]
    pub id: i32,
    /// 商城订单号
    pub order_id: Option<i32>,
    /// 实际创建面单的物流服务商：0未知，1快递鸟，2快递100 [NOT NULL]
    pub provider_type: i32,
    /// 快递订单编号
    pub order_code: Option<String>,
    /// 物流单号
    pub logistic_code: Option<String>,
    /// 发起时间
    pub add_time: Option<i32>,
    /// 返回模板
    pub print_template: Option<String>,
}

impl LogisticsApiLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "logistics_api_log";
}


/// 订单表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Order {
    /// 自增ID [NOT NULL]
    pub order_id: i32,
    /// [NOT NULL]
    pub order_sn: String,
    /// [NOT NULL]
    pub user_id: i32,
    /// 父订单ID（周期购子订单关联主订单）
    pub parent_order_id: Option<i32>,
    /// 父订单号（周期购子订单关联主订单）
    pub parent_order_sn: Option<String>,
    /// 订单的状态：0 待确定，待支付 1 已确定，待发货 2处理中，已发货 3 已取消 4 无效 5 已完成 [NOT NULL]
    pub order_status: i32,
    /// [NOT NULL]
    pub shipping_status: i32,
    /// 支付状态：0未支付 1支付中 2已支付 3退款中 4 已退款 [NOT NULL]
    pub pay_status: i32,
    /// 订单生成时间 [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub consignee: String,
    /// 详细地址 [NOT NULL]
    pub address: String,
    /// 【JSON】地址id，number[] [NOT NULL]
    pub region_ids: String,
    /// [NOT NULL]
    pub region_names: String,
    /// 【JSON】收货人信息
    pub address_data: Option<String>,
    /// 收货人的手机 [NOT NULL]
    pub mobile: String,
    /// 收货人的Email [NOT NULL]
    pub email: String,
    /// 买家备注 [NOT NULL]
    pub buyer_note: String,
    /// 商家备注，仅商家可见 [NOT NULL]
    pub admin_note: String,
    /// 发货方式：1：物流，2：商家自提，3：无需配送 [NOT NULL]
    pub shipping_method: i32,
    /// 快递公司id/为0则表示商家配送或无需配送 [NOT NULL]
    pub logistics_id: i32,
    /// 物流服务商：0历史未知或不适用，1快递鸟，2快递100 [NOT NULL]
    pub logistics_provider: i32,
    /// 具体物流公司名称/商家配送/无需配送 [NOT NULL]
    pub logistics_name: String,
    /// 配送类型id [NOT NULL]
    pub shipping_type_id: i32,
    /// 配送类型名称 [NOT NULL]
    pub shipping_type_name: String,
    /// 发货单号 [NOT NULL]
    pub tracking_no: String,
    /// 订单配送时间 [NOT NULL]
    pub shipping_time: i32,
    /// 订单收货时间 [NOT NULL]
    pub received_time: i32,
    /// [NOT NULL]
    pub pay_type_id: i32,
    /// 订单支付时间 [NOT NULL]
    pub pay_time: i32,
    /// 使用的积分的数量 [NOT NULL]
    pub use_points: i32,
    /// 是否需要分佣 [NOT NULL]
    pub is_need_commisson: i32,
    /// 是否已分销分成 [NOT NULL]
    pub distribution_status: i32,
    /// 推广人userId [NOT NULL]
    pub referrer_user_id: i32,
    /// 是否删除 [NOT NULL]
    pub is_del: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// [NOT NULL]
    pub is_store_splited: i32,
    /// 订单是否有效:1有效;0无效(支付后拆单父订单) [NOT NULL]
    pub is_valid: i32,
    /// 是否已评价 [NOT NULL]
    pub comment_status: i32,
    /// 订单总金额（商品总价 + 运费等 - 优惠券 - 各种优惠活动） [NOT NULL]
    pub total_amount: Decimal,
    /// 已支付金额(包含使用余额+线上支付金额+线下支付金额) [NOT NULL]
    pub paid_amount: Decimal,
    /// 未付款金额（计算方式为：total_amount - paid_amount） [NOT NULL]
    pub unpaid_amount: Decimal,
    /// 未退款金额（一般出现在订单取消或修改金额、商品后） [NOT NULL]
    pub unrefund_amount: Decimal,
    /// 商品的总金额 [NOT NULL]
    pub product_amount: Decimal,
    /// 使用优惠券金额 [NOT NULL]
    pub coupon_amount: Decimal,
    /// 使用积分金额 [NOT NULL]
    pub points_amount: Decimal,
    /// 参加优惠活动金额，如折扣、满减等 [NOT NULL]
    pub discount_amount: Decimal,
    /// 使用余额金额 [NOT NULL]
    pub balance: Decimal,
    /// 账期支付金额（单位：元） [NOT NULL]
    pub credit_amount: Decimal,
    /// 是否账期订单：0-否，1-是 [NOT NULL]
    pub is_credit_order: i32,
    /// 线上支付金额（支付宝、微信等） [NOT NULL]
    pub online_paid_amount: Decimal,
    /// 线下支付金额（银行汇款等） [NOT NULL]
    pub offline_paid_amount: Decimal,
    /// 服务费 [NOT NULL]
    pub service_fee: Decimal,
    /// 配送费用 [NOT NULL]
    pub shipping_fee: Decimal,
    /// 发票费用 [NOT NULL]
    pub invoice_fee: Decimal,
    /// 【JSON】记录订单使用的优惠券、优惠活动、不同店铺配送等的具体细节信息 [NOT NULL]
    pub order_extension: String,
    /// 下单来源设备，APP|PC|H5|微信公众号|微信小程序 [NOT NULL]
    pub order_source: String,
    /// 【JSON】发票信息
    pub invoice_data: Option<String>,
    /// 订单支付平台单号
    pub out_trade_no: Option<String>,
    /// 是否结算0未结算1已结算
    pub is_settlement: Option<i32>,
    /// 是否是积分抵扣订单
    pub is_exchange_order: Option<i32>,
    /// 订单类型：1普通订单，3积分订单，2拼团，5砍一砍 6 虚拟订单 7 付费商品订单 8 卡密订单 9预约到店 10 预约上门
    pub order_type: Option<i32>,
    /// 是否周期购订单：0-否，1-是 [NOT NULL]
    pub is_cycle_order: i32,
    /// 是否次卡订单：0-否，1-是 [NOT NULL]
    pub is_card_order: i32,
    /// 是否预售订单：0-否，1-是 [NOT NULL]
    pub is_presale: i32,
    /// 预售类型：1-全款预售，2-定金预售
    pub presale_type: Option<i32>,
    /// 标记 12345
    pub mark: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 取消原因
    pub cancel_remark: Option<String>,
    /// 现金金额 [NOT NULL]
    pub cash_amount: Decimal,
    /// 现金金额(找零) [NOT NULL]
    pub cash_change_amount: Decimal,
    /// 抹角金额 [NOT NULL]
    pub round_off_dime_amount: Decimal,
    /// 抹分金额 [NOT NULL]
    pub round_off_cents_amount: Decimal,
    /// 后台管理员id
    pub pos_user_id: Option<i32>,
    /// 收银台支付时选择的打印设备码
    pub device_code: Option<String>,
    pub discount_change_price: Option<Decimal>,
    /// 税率(百分比)
    pub tax_rate: Option<Decimal>,
    /// 税率金额
    pub tax_rate_amount: Option<Decimal>,
    pub change_price_data: Option<String>,
    /// 礼品卡兑换金额 [NOT NULL]
    pub gift_card_amount: Decimal,
    /// 计划配送日期（周期购子订单）
    pub delivery_date: Option<String>,
    /// 配送时间段（周期购子订单）
    pub delivery_time_slot: Option<String>,
    /// 第几期（周期购子订单）
    pub period_index: Option<i32>,
    /// 期内第几次（周期购子订单）
    pub time_index: Option<i32>,
}

impl Order {
    /// 数据库表名。
    pub const TABLE: &'static str = "order";
}


/// 订单店铺金额明细表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderAmountDetail {
    /// [NOT NULL]
    pub order_discount_detail_id: i32,
    pub order_id: Option<i32>,
    pub shop_id: Option<i32>,
    /// 运费金额
    pub shipping_fee: Option<Decimal>,
    /// 除优惠券之外的金额
    pub discount_amount: Option<Decimal>,
    /// 优惠券金额
    pub coupon_amount: Option<Decimal>,
    /// 限时折扣金额
    pub time_discount_amount: Option<Decimal>,
}

impl OrderAmountDetail {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_amount_detail";
}


/// 订单店铺优惠券表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderCouponDetail {
    /// [NOT NULL]
    pub order_coupon_detail_id: i32,
    pub order_id: Option<i32>,
    pub shop_id: Option<i32>,
    /// 优惠券id
    pub coupon_id: Option<i32>,
    /// 优惠金额
    pub coupon_fee: Option<Decimal>,
}

impl OrderCouponDetail {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_coupon_detail";
}


/// 订单周期购计划表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderCycle {
    /// 周期购订单ID [NOT NULL]
    pub order_cycle_id: i32,
    /// 主订单ID [NOT NULL]
    pub order_id: i32,
    /// 主订单号 [NOT NULL]
    pub order_sn: String,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// SKU ID
    pub sku_id: Option<i32>,
    /// 购买期数 [NOT NULL]
    pub period_num: i32,
    /// 每期次数 [NOT NULL]
    pub times_per_period: i32,
    /// 购买数量（每次配送件数） [NOT NULL]
    pub quantity: i32,
    /// 总配送次数（期数×每期次数） [NOT NULL]
    pub total_times: i32,
    /// 总配送商品数量（总次数×购买数量） [NOT NULL]
    pub total_quantity: i32,
    /// 已完成次数 [NOT NULL]
    pub completed_times: i32,
    /// 配送周期类型：1-每日一次，2-每周一次，3-每月一次，4-多周一次，5-多月一次，6-每周多次，7-每月多次 [NOT NULL]
    pub cycle_type: i32,
    /// 周期间隔
    pub cycle_interval: Option<i32>,
    /// 每周期期数
    pub times_per_cycle: Option<i32>,
    /// 用户选择的配送星期JSON数组
    pub selected_weekdays: Option<String>,
    /// 用户选择的配送日期JSON数组
    pub selected_days: Option<String>,
    /// 用户选择的配送时间段 [NOT NULL]
    pub selected_time_slot: String,
    /// 下次配送日期
    pub next_delivery_date: Option<String>,
    /// 状态：0-待支付，1-进行中，2-已暂停，3-已完成，4-已取消 [NOT NULL]
    pub status: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
    /// 父订单信息JSON（用于定时任务生成子订单时创建拆分日志和派送订单）
    pub parent_order_data: Option<String>,
}

impl OrderCycle {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_cycle";
}


/// 次卡权益记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderCycleCard {
    /// 次卡权益ID [NOT NULL]
    pub card_id: i32,
    /// 关联主订单ID [NOT NULL]
    pub order_id: i32,
    /// 关联主订单号 [NOT NULL]
    pub order_sn: String,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// SKU ID
    pub sku_id: Option<i32>,
    /// 商品名称（冗余）
    pub product_name: Option<String>,
    /// 商品图片（冗余）
    pub product_pic: Option<String>,
    /// SKU规格值（冗余）
    pub sku_value: Option<String>,
    /// 总兑换次数 [NOT NULL]
    pub total_times: i32,
    /// 已兑换次数 [NOT NULL]
    pub redeemed_times: i32,
    /// 剩余兑换次数 [NOT NULL]
    pub remaining_times: i32,
    /// 总支付金额（冗余，用于退款计算） [NOT NULL]
    pub total_amount: Decimal,
    /// 生效时间类型：1-立即，2-次日，3-延迟 [NOT NULL]
    pub effective_type: i32,
    /// 生效时间（时间戳）
    pub effective_time: Option<i64>,
    /// 到期时间（时间戳，NULL表示长期有效）
    pub expire_time: Option<i64>,
    /// 有效期类型：1-长期，2-当天，3-N天内，4-固定区间 [NOT NULL]
    pub validity_type: i32,
    /// 状态：0-待生效，1-使用中，2-已用完，3-已过期，4-已退款 [NOT NULL]
    pub status: i32,
    /// 乐观锁版本号（防并发扣减） [NOT NULL]
    pub version: i32,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
}

impl OrderCycleCard {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_cycle_card";
}


/// 次卡兑换记录表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderCycleCardRedeem {
    /// 兑换记录ID [NOT NULL]
    pub redeem_id: i32,
    /// 关联次卡权益ID [NOT NULL]
    pub card_id: i32,
    /// 关联主订单ID [NOT NULL]
    pub order_id: i32,
    /// 生成的子订单ID
    pub sub_order_id: Option<i32>,
    /// 子订单号
    pub sub_order_sn: Option<String>,
    /// 用户ID [NOT NULL]
    pub user_id: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 兑换序号（从1开始） [NOT NULL]
    pub redeem_index: i32,
    /// 本次兑换金额（总额÷总次数） [NOT NULL]
    pub unit_amount: Decimal,
    /// 配送方式
    pub delivery_method: Option<String>,
    /// 收货地址ID
    pub address_id: Option<i32>,
    /// 自提门店ID
    pub store_id: Option<i32>,
    /// 配送日期
    pub delivery_date: Option<String>,
    /// 配送时间段
    pub delivery_time_slot: Option<String>,
    /// 状态：0-处理中，1-已完成，2-已取消 [NOT NULL]
    pub status: i32,
    /// 兑换时间
    pub create_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
}

impl OrderCycleCardRedeem {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_cycle_card_redeem";
}


/// 周期购配送计划表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderCycleDelivery {
    /// 配送计划ID [NOT NULL]
    pub delivery_id: i32,
    /// 关联周期购计划ID [NOT NULL]
    pub order_cycle_id: i32,
    /// 主订单ID [NOT NULL]
    pub order_id: i32,
    /// 子订单ID（生成后关联）
    pub sub_order_id: Option<i32>,
    /// 子订单号
    pub sub_order_sn: Option<String>,
    /// 第几期（从1开始） [NOT NULL]
    pub period_index: i32,
    /// 期内第几次（从1开始） [NOT NULL]
    pub time_index: i32,
    /// 计划配送日期 [NOT NULL]
    pub delivery_date: String,
    /// 配送日期文本，如：12月30日（周一）
    pub delivery_date_text: Option<String>,
    /// 配送时间段 [NOT NULL]
    pub delivery_time_slot: String,
    /// 状态：0-待生成，1-已生成待发货，2-已发货，3-已完成，4-已取消 [NOT NULL]
    pub status: i32,
    /// 订单生成时间
    pub generate_time: Option<i32>,
    /// 库存是否已扣减：NULL-未知(旧数据)，0-未扣减(库存不足)，1-已扣减
    pub stock_deducted: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl OrderCycleDelivery {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_cycle_delivery";
}


/// 订单发票地址
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderInvoice {
    /// 订单发票申请id [NOT NULL]
    pub id: i32,
    /// 会员id [NOT NULL]
    pub user_id: i32,
    /// 订单id/父订单id [NOT NULL]
    pub order_id: i32,
    /// 发票类型：1:普通发票，2:专用发票 [NOT NULL]
    pub invoice_type: i32,
    /// 发票申请状态：0:待处理，1:已开，2:失败/作废 [NOT NULL]
    pub status: i32,
    /// 申请失败时的回复内容 [NOT NULL]
    pub apply_reply: String,
    /// 抬头：1：个人，2企业 [NOT NULL]
    pub title_type: i32,
    /// 纳税人识别号 [NOT NULL]
    pub company_code: String,
    /// 个人/单位名称 [NOT NULL]
    pub company_name: String,
    /// 单位地址 [NOT NULL]
    pub company_address: String,
    /// 单位电话 [NOT NULL]
    pub company_phone: String,
    /// 开户银行 [NOT NULL]
    pub company_bank: String,
    /// 银行账号 [NOT NULL]
    pub company_account: String,
    /// 发票内容，统一为商品明细 [NOT NULL]
    pub invoice_content: String,
    /// 发票号码 [NOT NULL]
    pub invoice_no: String,
    /// 发票金额 [NOT NULL]
    pub amount: Decimal,
    /// 手机号 [NOT NULL]
    pub mobile: String,
    /// 邮箱 [NOT NULL]
    pub email: String,
    /// 申请时间 [NOT NULL]
    pub add_time: i32,
    /// 发票附件地址
    pub invoice_attachment: Option<String>,
    /// 开票时间（未开票时为 NULL，对应 Java 可空 Integer）
    pub invoicing_time: Option<i32>,
}

impl OrderInvoice {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_invoice";
}


/// 订单的商品信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderItem {
    /// 订单商品信息自增id [NOT NULL]
    pub item_id: i32,
    /// 订单ID [NOT NULL]
    pub order_id: i32,
    /// 订单编号 [NOT NULL]
    pub order_sn: String,
    /// 会员ID [NOT NULL]
    pub user_id: i32,
    /// 商品最终单价 [NOT NULL]
    pub price: Decimal,
    /// 商品的购买数量 [NOT NULL]
    pub quantity: i32,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// 商品的名称 [NOT NULL]
    pub product_name: String,
    /// 商品的唯一货号 [NOT NULL]
    pub product_sn: String,
    /// 商品缩略图 [NOT NULL]
    pub pic_thumb: String,
    /// 规格ID [NOT NULL]
    pub sku_id: i32,
    /// [NOT NULL]
    pub sku_data: String,
    /// 发货数量 [NOT NULL]
    pub delivery_quantity: i32,
    /// 商品类型，1：实体，2：虚拟，3赠品 [NOT NULL]
    pub product_type: i32,
    /// 是否为赠品 [NOT NULL]
    pub is_gift: i32,
    /// 店铺ID [NOT NULL]
    pub shop_id: i32,
    /// 是否拼团 [NOT NULL]
    pub is_pin: i32,
    /// 预售价格 [NOT NULL]
    pub prepay_price: Decimal,
    /// 佣金数据
    pub commission: Option<String>,
    /// 商品初始价格
    pub origin_price: Option<Decimal>,
    /// 优惠信息
    pub promotion_data: Option<String>,
    /// 是否秒杀
    pub is_seckill: Option<i32>,
    /// 附加属性
    pub extra_sku_data: Option<String>,
    /// 供应商id [NOT NULL]
    pub suppliers_id: i32,
    /// 卡密商品组名称 [NOT NULL]
    pub card_group_name: String,
    /// 供应商产品表id
    pub vendor_product_id: Option<i32>,
    /// 供应商产品sku表id
    pub vendor_product_sku_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 供应商产品供货价
    pub vendor_product_supply_price: Option<Decimal>,
    /// 改价data(涉及到的购物车的改价数据)
    pub change_price_data: Option<String>,
    /// 后台改价前价格
    pub admin_change_pre_price: Option<Decimal>,
    /// 礼品卡兑换订单(礼品卡id)
    pub gift_card_id: Option<i32>,
    /// 配送员服务费(单件快照，仅O2O写入) [NOT NULL]
    pub delivery_service_fee: Decimal,
    /// 服务员服务费(单件快照，仅O2O上门服务订单写入) [NOT NULL]
    pub server_service_fee: Decimal,
    /// B2B批发:1=批发行 0=零售行 [NOT NULL]
    pub is_wholesale_line: i32,
    /// B2B批发:等级快照(0=无) [NOT NULL]
    pub wholesale_grade_id: i32,
    /// B2B批发:批发基准价快照(冻结,审计/退款用,design §6) [NOT NULL]
    pub wholesale_base_price: Decimal,
}

impl OrderItem {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_item";
}


/// 订单操作日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderLog {
    /// 流水号 [NOT NULL]
    pub log_id: i32,
    /// 被操作的订单id [NOT NULL]
    pub order_id: i32,
    /// 订单号 [NOT NULL]
    pub order_sn: String,
    /// 操作管理员（后台） [NOT NULL]
    pub admin_id: i32,
    /// 操作会员（前台） [NOT NULL]
    pub user_id: i32,
    /// 操作备注 [NOT NULL]
    pub description: String,
    /// 操作时间 [NOT NULL]
    pub log_time: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
}

impl OrderLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_log";
}


/// 线下支付认款单（v5.8.61）
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OfflinePaymentClaim {
    pub claim_id: i32,
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub shop_id: i32,
    pub claimed_amount: Decimal,
    pub pay_stage: i32,
    pub transfer_time: i64,
    pub payer_name: String,
    pub payer_account: String,
    pub bank_serial_no: String,
    pub voucher_urls: Option<String>,
    pub remark: String,
    pub status: i32,
    pub review_admin_id: i32,
    pub review_time: i64,
    pub paid_time: i64,
    pub audit_remark: String,
    pub reject_reason: String,
    pub add_time: i64,
    pub update_time: i64,
}

impl OfflinePaymentClaim {
    pub const TABLE: &'static str = "offline_payment_claim";
    pub const STATUS_PENDING: i32 = 0;
    pub const STATUS_APPROVED: i32 = 1;
    pub const STATUS_REJECTED: i32 = 2;
    pub const STAGE_FULL: i32 = 0;
    pub const STAGE_DEPOSIT: i32 = 1;
    pub const STAGE_FINAL: i32 = 2;
}

/// 订单预售信息表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderPresale {
    /// 预售订单ID [NOT NULL]
    pub presale_order_id: i32,
    /// 订单ID [NOT NULL]
    pub order_id: i32,
    /// 订单号 [NOT NULL]
    pub order_sn: String,
    /// 商品ID [NOT NULL]
    pub product_id: i32,
    /// 销售模式：1-全款预售，2-定金预售 [NOT NULL]
    pub sale_mode: i32,
    /// 定金金额（定金预售） [NOT NULL]
    pub deposit_amount: Decimal,
    /// 尾款金额（定金预售） [NOT NULL]
    pub final_payment_amount: Decimal,
    /// 定金支付时间
    pub deposit_paid_time: Option<i32>,
    /// 尾款开始支付时间（实际计算值）
    pub final_payment_start_time: Option<i32>,
    /// 尾款支付时间
    pub final_payment_paid_time: Option<i32>,
    /// 预售状态：0-待支付定金，1-待支付尾款，2-已完成支付 [NOT NULL]
    pub presale_status: i32,
    /// 发货天数（相对时间）
    pub delivery_days: Option<i32>,
    /// 发货日期（指定日期）
    pub delivery_date: Option<i32>,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl OrderPresale {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_presale";
}


/// 产品预约信息
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderReservations {
    /// 主键ID [NOT NULL]
    pub order_reservations_id: i32,
    /// 产品ID [NOT NULL]
    pub order_id: i32,
    /// 预约时间段ID
    pub product_reservation_time_id: Option<i32>,
    /// SKU ID [NOT NULL]
    pub sku_id: i32,
    /// 库存日期，比如 2025-09-18 [NOT NULL]
    pub reservation_date: String,
    /// 开始时间，格式HH:mm [NOT NULL]
    pub start_time: String,
    /// 结束时间，格式HH:mm [NOT NULL]
    pub end_time: String,
    /// 时间段描述 [NOT NULL]
    pub time_slot_description: String,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
}

impl OrderReservations {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_reservations";
}


/// 订单拆单记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct OrderSplitLog {
    /// 拆份订单 [NOT NULL]
    pub log_id: i32,
    /// 父订单id [NOT NULL]
    pub parent_order_id: i32,
    /// 拆分后的订单id [NOT NULL]
    pub order_id: i32,
    /// 拆分时间 [NOT NULL]
    pub split_time: i32,
    /// 父订单信息
    pub parent_order_data: Option<String>,
}

impl OrderSplitLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "order_split_log";
}