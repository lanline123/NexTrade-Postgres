//! 财务域实体（支付流水/退款/结算/提现/发票/分账）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `category_service_fee_rule`, `category_service_fee_rule_category`, `category_service_fee_rule_target`, `credit_order`, `credit_settlement`, `credit_user`, `merchant_application`, `merchant_application_media`, `paylog`, `paylog_refund`, `profit_sharing_order`, `profit_sharing_return_order`, `record_rate`, `record_rate_old`, `refund_apply`, `refund_apply_paylog_refund`, `refund_log`, `statement`, `statement_download`, `statement_output`, `statement_output_old`, `sub_merchant`, `user_balance_log`, `user_cash_log`, `user_invoice`, `user_recharge_order`, `user_withdraw_apply`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 类目服务费率规则
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CategoryServiceFeeRule {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// 规则名称 [NOT NULL]
    pub rule_name: String,
    /// 适用对象类型:1店铺 2供应商 [NOT NULL]
    pub target_type: i32,
    /// 适用范围:1全部 2指定 3不包含 [NOT NULL]
    pub scope_type: i32,
    /// 平台服务费率(%) [NOT NULL]
    pub service_fee_rate: Decimal,
    /// 备注 [NOT NULL]
    pub remark: String,
    /// 状态:1启用 0禁用 [NOT NULL]
    pub status: i32,
    /// 逻辑删除:0未删 非0已删 [NOT NULL]
    pub deleted: i32,
    /// 创建时间
    pub create_time: Option<String>,
    /// 修改时间
    pub modify_time: Option<String>,
}

impl CategoryServiceFeeRule {
    /// 数据库表名。
    pub const TABLE: &'static str = "category_service_fee_rule";
}


/// 费率规则品类关联
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CategoryServiceFeeRuleCategory {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// 规则ID [NOT NULL]
    pub rule_id: i32,
    /// 品类ID [NOT NULL]
    pub category_id: i32,
    /// 品类名称(冗余) [NOT NULL]
    pub category_name: String,
    /// 品类路径(冗余) [NOT NULL]
    pub category_path: String,
    /// 逻辑删除 [NOT NULL]
    pub deleted: i32,
    pub create_time: Option<String>,
    pub modify_time: Option<String>,
}

impl CategoryServiceFeeRuleCategory {
    /// 数据库表名。
    pub const TABLE: &'static str = "category_service_fee_rule_category";
}


/// 费率规则目标关联
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CategoryServiceFeeRuleTarget {
    /// 主键 [NOT NULL]
    pub id: i32,
    /// 规则ID [NOT NULL]
    pub rule_id: i32,
    /// 目标ID(店铺ID或供应商ID) [NOT NULL]
    pub target_id: i32,
    /// 逻辑删除 [NOT NULL]
    pub deleted: i32,
    pub create_time: Option<String>,
    pub modify_time: Option<String>,
}

impl CategoryServiceFeeRuleTarget {
    /// 数据库表名。
    pub const TABLE: &'static str = "category_service_fee_rule_target";
}


/// 账期支付订单表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CreditOrder {
    /// 账期订单ID [NOT NULL]
    pub credit_order_id: i32,
    /// 订单ID [NOT NULL]
    pub order_id: i32,
    /// 订单号 [NOT NULL]
    pub order_sn: String,
    /// 会员ID [NOT NULL]
    pub user_id: i32,
    /// 授信会员ID [NOT NULL]
    pub credit_user_id: i32,
    /// 账期支付金额（单位：元） [NOT NULL]
    pub credit_amount: Decimal,
    /// 结算状态：0-未结算，1-已结算，2-已取消，3-退款中 [NOT NULL]
    pub settlement_status: i32,
    /// 结算记录ID
    pub settlement_id: Option<i32>,
    /// 回款流水单号
    pub settlement_sn: Option<String>,
    /// 下单时间（秒级时间戳）
    pub order_time: Option<i64>,
    /// 结算时间（秒级时间戳）
    pub settlement_time: Option<i64>,
    /// 创建时间（秒级时间戳）
    pub create_time: Option<i64>,
    /// 更新时间（秒级时间戳）
    pub update_time: Option<i64>,
}

impl CreditOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "credit_order";
}


/// 账期结算表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CreditSettlement {
    /// 结算ID [NOT NULL]
    pub settlement_id: i32,
    /// 回款流水单号 [NOT NULL]
    pub settlement_sn: String,
    /// 会员ID [NOT NULL]
    pub user_id: i32,
    /// 授信会员ID [NOT NULL]
    pub credit_user_id: i32,
    /// 结算金额（单位：元） [NOT NULL]
    pub settlement_amount: Decimal,
    /// 结算订单数 [NOT NULL]
    pub order_count: i32,
    /// 备注
    pub remark: Option<String>,
    /// 结算时间（秒级时间戳）
    pub settlement_time: Option<i64>,
    /// 结算状态：1-已结算，2-已撤销 [NOT NULL]
    pub settlement_status: i32,
    /// 撤销时间（秒级时间戳）
    pub cancel_time: Option<i64>,
    /// 撤销原因
    pub cancel_reason: Option<String>,
    /// 操作人ID
    pub operator_id: Option<i32>,
    /// 操作人名称
    pub operator_name: Option<String>,
    /// 创建时间（秒级时间戳）
    pub create_time: Option<i64>,
    /// 更新时间（秒级时间戳）
    pub update_time: Option<i64>,
}

impl CreditSettlement {
    /// 数据库表名。
    pub const TABLE: &'static str = "credit_settlement";
}


/// 授信会员表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct CreditUser {
    /// 授信会员ID [NOT NULL]
    pub credit_user_id: i32,
    /// 会员ID [NOT NULL]
    pub user_id: i32,
    /// 信用额度（单位：元） [NOT NULL]
    pub credit_limit: Decimal,
    /// 已使用额度（单位：元） [NOT NULL]
    pub used_amount: Decimal,
    /// 可用额度（单位：元） [NOT NULL]
    pub available_amount: Decimal,
    /// 结算账期类型：1-月结30天，2-季结90天 [NOT NULL]
    pub settlement_type: i32,
    /// 状态：0-禁用，1-启用 [NOT NULL]
    pub status: i32,
    /// 当前结算周期起点（秒级时间戳），结算清空后重置
    pub period_start_time: Option<i64>,
    /// 创建时间（秒级时间戳）
    pub create_time: Option<i64>,
    /// 更新时间（秒级时间戳）
    pub update_time: Option<i64>,
}

impl CreditUser {
    /// 数据库表名。
    pub const TABLE: &'static str = "credit_user";
}


/// 商户进件申请
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MerchantApplication {
    /// 进件申请ID [NOT NULL]
    pub id: i64,
    /// 店铺ID [NOT NULL]
    pub shop_id: i64,
    /// 申请主体类型:SHOP店铺/门店 VENDOR供应商 [NOT NULL]
    pub applicant_type: String,
    /// 申请主体ID:SHOP=shopId VENDOR=vendorId [NOT NULL]
    pub applicant_id: i64,
    /// 申请主体名称(店铺名/供应商名,冗余展示) [NOT NULL]
    pub applicant_name: String,
    /// 支付渠道 [NOT NULL]
    pub pay_channel: String,
    /// 业务申请编号(幂等键) [NOT NULL]
    pub business_code: String,
    /// 微信申请单号
    pub applyment_id: Option<String>,
    /// 组织类型 [NOT NULL]
    pub organization_type: String,
    /// 商户名称 [NOT NULL]
    pub merchant_name: String,
    /// 联系人姓名 [NOT NULL]
    pub contact_name: String,
    /// 联系人电话 [NOT NULL]
    pub contact_phone: String,
    /// 身份证号(加密)
    pub id_card_number_enc: Option<String>,
    /// 银行账号(加密)
    pub bank_account_enc: Option<String>,
    /// 状态:DRAFT草稿/PENDING_REVIEW平台待审核/SUBMITTED已提交/CHECKING资料校验中/AUDITING审核中/ACCOUNT_NEED_VERIFY待账户验证/NEED_SIGN待签约/FINISH完成/REJECTED已驳回 [NOT NULL]
    pub status: String,
    /// 成功后返回的子商户号
    pub sub_mch_id: Option<String>,
    /// 签约链接
    pub sign_url: Option<String>,
    /// 驳回原因 [NOT NULL]
    pub reject_reason: String,
    /// 审核人ID(平台管理员) [NOT NULL]
    pub audit_admin_id: i32,
    /// 审核时间
    pub audit_time: Option<String>,
    /// 提交请求快照(脱敏JSON)
    pub request_snapshot: Option<String>,
    /// 逻辑删除:0未删 非0已删 [NOT NULL]
    pub is_del: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: String,
    /// 修改时间 [NOT NULL]
    pub modify_time: String,
}

impl MerchantApplication {
    /// 数据库表名。
    pub const TABLE: &'static str = "merchant_application";
}


/// 商户进件媒体元数据
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MerchantApplicationMedia {
    /// 进件媒体ID [NOT NULL]
    pub id: i64,
    /// 进件申请ID，0表示未绑定 [NOT NULL]
    pub application_id: i64,
    /// 上传后台账号ID [NOT NULL]
    pub uploader_admin_id: i32,
    /// 申请主体类型:SHOP/VENDOR [NOT NULL]
    pub applicant_type: String,
    /// 申请主体ID [NOT NULL]
    pub applicant_id: i64,
    /// 媒体角色:0未绑定 1营业执照 2身份证人像面 3身份证国徽面 [NOT NULL]
    pub media_role: i32,
    /// 初始MediaID的SHA-256摘要
    pub initial_media_id_hash: Option<String>,
    /// 私有OSS随机对象键 [NOT NULL]
    pub object_name: String,
    /// 文件扩展名 [NOT NULL]
    pub file_ext: String,
    /// 文件MIME类型 [NOT NULL]
    pub content_type: String,
    /// 文件大小，单位字节 [NOT NULL]
    pub file_size: i64,
    /// 状态:0上传中 1临时 2已绑定 3待删除 4已删除 [NOT NULL]
    pub status: i32,
    /// 删除重试次数 [NOT NULL]
    pub retry_count: i32,
    /// 下次补偿时间
    pub next_retry_time: Option<String>,
    /// 当前生命周期到期时间
    pub expire_time: Option<String>,
    /// 脱敏错误分类 [NOT NULL]
    pub last_error_code: String,
    /// 逻辑删除:0未删 非0已删 [NOT NULL]
    pub is_del: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: String,
    /// 修改时间 [NOT NULL]
    pub modify_time: String,
}

impl MerchantApplicationMedia {
    /// 数据库表名。
    pub const TABLE: &'static str = "merchant_application_media";
}


/// 支付日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Paylog {
    /// 支付日志id [NOT NULL]
    pub paylog_id: i32,
    /// 支付号 [NOT NULL]
    pub pay_sn: String,
    /// 支付方式名称 [NOT NULL]
    pub pay_name: String,
    /// 订单id [NOT NULL]
    pub order_id: i32,
    /// 订单sn [NOT NULL]
    pub order_sn: String,
    /// 订单金额 [NOT NULL]
    pub order_amount: Decimal,
    /// 订单类型0订单1充值 [NOT NULL]
    pub order_type: i32,
    /// 支付金额 [NOT NULL]
    pub pay_amount: Decimal,
    /// 支付状态，0:待支付，1：已支付，2：支付失败 [NOT NULL]
    pub pay_status: i32,
    /// 支付方式code，比如wechat [NOT NULL]
    pub pay_code: String,
    /// 支付时线上分账渠道快照：WECHAT/HUIFU/ALIPAY，NULL表示未冻结分账资金
    pub profit_sharing_channel: Option<String>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 交易流水号 [NOT NULL]
    pub transaction_id: String,
    /// 回调参数 [NOT NULL]
    pub notify_data: String,
    /// 退款金额 [NOT NULL]
    pub refund_amount: Decimal,
    /// [NOT NULL]
    pub token_code: String,
    /// 微信支付返回的appid（当前部署库没有此列，允许 FromRow 缺省）
    #[sqlx(default)]
    pub appid: Option<String>,
}

impl Paylog {
    /// 数据库表名。
    pub const TABLE: &'static str = "paylog";
}


/// 退款日志表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct PaylogRefund {
    /// [NOT NULL]
    pub paylog_refund_id: i32,
    /// 支付日志id
    pub paylog_id: Option<i32>,
    /// 退款单号
    pub refund_sn: Option<String>,
    /// 操作备注 [NOT NULL]
    pub paylog_desc: String,
    /// 订单编号 [NOT NULL]
    pub order_id: i32,
    /// 退款金额 [NOT NULL]
    pub refund_amount: Decimal,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 操作人员id [NOT NULL]
    pub admin_id: i32,
    /// 支付方式code，比如wechat
    pub pay_code: Option<String>,
    /// 处理状态 0申请中 1成功
    pub status: Option<i32>,
    /// 通知时间
    pub notify_time: Option<i32>,
}

impl PaylogRefund {
    /// 数据库表名。
    pub const TABLE: &'static str = "paylog_refund";
}


/// 微信分账单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProfitSharingOrder {
    /// 分账单ID [NOT NULL]
    pub id: i64,
    /// 商户分账单号(幂等键) [NOT NULL]
    pub out_order_no: String,
    /// 订单ID [NOT NULL]
    pub order_id: i64,
    /// 订单编号 [NOT NULL]
    pub order_sn: String,
    /// 微信交易号 [NOT NULL]
    pub transaction_id: String,
    /// 支付渠道 [NOT NULL]
    pub pay_channel: String,
    /// 店铺ID [NOT NULL]
    pub shop_id: i64,
    /// 供应商ID(0表示店铺分账单) [NOT NULL]
    pub vendor_id: i64,
    /// 结算主体类型:1店铺 2供应商 [NOT NULL]
    pub settlement_type: i32,
    /// 子商户号 [NOT NULL]
    pub sub_mch_id: String,
    /// 关联结算单ID(statement_output) [NOT NULL]
    pub settlement_id: i64,
    /// 手动分账操作人 [NOT NULL]
    pub operator_id: i64,
    /// 订单总金额(元) [NOT NULL]
    pub total_amount: Decimal,
    /// 首次实际分账时的计费基数，历史空值沿用total_amount
    pub sharing_base_amount: Option<Decimal>,
    /// 分账金额=服务费+分销佣金(元) [NOT NULL]
    pub split_amount: Decimal,
    /// 服务费(元) [NOT NULL]
    pub service_fee: Decimal,
    /// 分销佣金(元) [NOT NULL]
    pub commission_fee: Decimal,
    /// 服务费率(%)
    pub service_fee_rate: Option<Decimal>,
    /// 状态:0待分账 1分账中 2成功 3失败 [NOT NULL]
    pub status: i32,
    /// 渠道分账单号 [NOT NULL]
    pub channel_order_id: String,
    /// 完结状态:0未完结 1已完结 [NOT NULL]
    pub finish_status: i32,
    /// 完结时间
    pub finish_time: Option<String>,
    /// 触发方式:1手动 2自动 [NOT NULL]
    pub trigger_type: i32,
    /// 结果描述 [NOT NULL]
    pub result_desc: String,
    /// 重试次数 [NOT NULL]
    pub retry_count: i32,
    /// 逻辑删除:0未删 非0已删 [NOT NULL]
    pub is_del: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: String,
    /// 修改时间 [NOT NULL]
    pub modify_time: String,
}

impl ProfitSharingOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "profit_sharing_order";
}


/// 微信分账回退单
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProfitSharingReturnOrder {
    /// 分账回退单ID [NOT NULL]
    pub id: i64,
    /// 关联分账单ID [NOT NULL]
    pub profit_sharing_order_id: i64,
    /// 商户回退单号 [NOT NULL]
    pub out_return_no: String,
    /// 原分账单号 [NOT NULL]
    pub out_order_no: String,
    /// 订单ID [NOT NULL]
    pub order_id: i64,
    /// 退款申请ID [NOT NULL]
    pub refund_apply_id: i64,
    /// 子商户号 [NOT NULL]
    pub sub_mch_id: String,
    /// 售后单ID(兼容) [NOT NULL]
    pub aftersale_id: i64,
    /// 回退金额(元) [NOT NULL]
    pub return_amount: Decimal,
    /// 支付渠道 [NOT NULL]
    pub pay_channel: String,
    /// 状态:0待处理 1处理中 2成功 3失败 [NOT NULL]
    pub status: i32,
    /// 渠道回退单号 [NOT NULL]
    pub channel_return_id: String,
    /// 微信交易号 [NOT NULL]
    pub transaction_id: String,
    /// 结果描述 [NOT NULL]
    pub result_desc: String,
    /// 重试次数 [NOT NULL]
    pub retry_count: i32,
    /// 逻辑删除:0未删 非0已删 [NOT NULL]
    pub is_del: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: String,
    /// 修改时间 [NOT NULL]
    pub modify_time: String,
}

impl ProfitSharingReturnOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "profit_sharing_return_order";
}


/// 记录费率表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RecordRate {
    /// [NOT NULL]
    pub record_rate_id: i32,
    /// 单据id
    pub record_id: Option<i32>,
    /// 单据类型
    pub record_type: Option<i32>,
    /// 店铺费率
    pub shop_service_fee: Option<Decimal>,
    /// 店铺手续费
    pub shop_withdrawal_fee: Option<Decimal>,
    /// 门店服务费
    pub storefront_service_fee: Option<Decimal>,
    /// 门店手续费
    pub storefront_withdrawal_fee: Option<Decimal>,
    /// 供应商服务费
    pub supplier_service_fee: Option<Decimal>,
    /// 供应商手续费
    pub supplier_withdrawal_fee: Option<Decimal>,
    /// 按品类的费率明细(JSON)
    pub category_rate_details: Option<String>,
    /// 记录当前时间
    pub gmt_create: Option<String>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
}

impl RecordRate {
    /// 数据库表名。
    pub const TABLE: &'static str = "record_rate";
}


/// 记录费率表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RecordRateOld {
    /// [NOT NULL]
    pub record_rate_id: i32,
    /// 单据id
    pub record_id: Option<i32>,
    /// 单据类型
    pub record_type: Option<i32>,
    /// 店铺费率
    pub shop_service_fee: Option<Decimal>,
    /// 店铺手续费
    pub shop_withdrawal_fee: Option<Decimal>,
    /// 门店服务费
    pub storefront_service_fee: Option<Decimal>,
    /// 门店手续费
    pub storefront_withdrawal_fee: Option<Decimal>,
    /// 供应商服务费
    pub supplier_service_fee: Option<Decimal>,
    /// 供应商手续费
    pub supplier_withdrawal_fee: Option<Decimal>,
    /// 按品类的费率明细(JSON)
    pub category_rate_details: Option<String>,
    /// 记录当前时间
    pub gmt_create: Option<String>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
}

impl RecordRateOld {
    /// 数据库表名。
    pub const TABLE: &'static str = "record_rate_old";
}


/// 退款申请
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RefundApply {
    /// 退款申请id [NOT NULL]
    pub refund_id: i32,
    /// [NOT NULL]
    pub refund_type: i32,
    /// 订单id [NOT NULL]
    pub order_id: i32,
    /// 会员id [NOT NULL]
    pub user_id: i32,
    /// 售后id [NOT NULL]
    pub aftersale_id: i32,
    /// [NOT NULL]
    pub refund_status: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 回复内容 [NOT NULL]
    pub refund_note: String,
    /// 线上退款金额 [NOT NULL]
    pub online_balance: Decimal,
    /// 线下退款金额 [NOT NULL]
    pub offline_balance: Decimal,
    /// 退款余额 [NOT NULL]
    pub refund_balance: Decimal,
    /// 线上金额是否到账:0 无需处理1处理中 2已处理 [NOT NULL]
    pub is_online: i32,
    /// 线下金额是否到账:0 无需处理1处理中 2已处理 [NOT NULL]
    pub is_offline: i32,
    /// 余额是否到账:0 无需处理1处理中 2已处理 [NOT NULL]
    pub is_receive: i32,
    /// 线上退款的关联id [NOT NULL]
    pub paylog_refund_id: i32,
    /// 店铺id [NOT NULL]
    pub shop_id: i32,
    /// 打款凭证
    pub payment_voucher: Option<String>,
}

impl RefundApply {
    /// 数据库表名。
    pub const TABLE: &'static str = "refund_apply";
}


/// 退款申请与退款流水关联表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RefundApplyPaylogRefund {
    /// 关联ID [NOT NULL]
    pub refund_apply_paylog_refund_id: i32,
    /// 退款申请ID [NOT NULL]
    pub refund_apply_id: i32,
    /// 退款流水ID [NOT NULL]
    pub paylog_refund_id: i32,
}

impl RefundApplyPaylogRefund {
    /// 数据库表名。
    pub const TABLE: &'static str = "refund_apply_paylog_refund";
}


/// 退款申请日志
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct RefundLog {
    /// 接口退款日志id [NOT NULL]
    pub log_id: i32,
    /// 来自的退款申请id [NOT NULL]
    pub refund_apply_id: i32,
    /// 退款类型：1：接口原路退回，2：余额退回，3：线下退回 [NOT NULL]
    pub refund_type: i32,
    /// 接口退款时的支付方式code，比如wechat、alipay等 [NOT NULL]
    pub refund_pay_code: String,
    /// 接口退款时的支付流水号 [NOT NULL]
    pub transaction_id: String,
    /// 实际退款金额 [NOT NULL]
    pub refund_amount: Decimal,
    /// 退款时间 [NOT NULL]
    pub add_time: i32,
    /// 退款描述
    pub description: Option<String>,
    /// 退款会员id [NOT NULL]
    pub user_id: i32,
    /// 退款的订单ID [NOT NULL]
    pub order_id: i32,
}

impl RefundLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "refund_log";
}


/// 对账单明细表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Statement {
    /// 主键ID [NOT NULL]
    pub statement_id: i64,
    /// 店铺ID/门店id
    pub shop_id: Option<i32>,
    /// 单据Id
    pub record_id: Option<i32>,
    /// 单据流水号
    pub record_sn: Option<String>,
    /// 下单时间
    pub record_time: Option<i64>,
    /// 结算时间
    pub settlement_time: Option<i64>,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 账户类型1.账户余额
    pub account_type: Option<i32>,
    /// 类型：1.手续费 2.店铺提现收支 3.店铺订单收支等
    pub r#type: Option<i32>,
    /// 入账方式：1. 自动 2.手动 [NOT NULL]
    pub entry_type: String,
    /// 支付方式（如微信、支付宝等）
    pub payment_type: Option<String>,
    /// 账户余额 [NOT NULL]
    pub account_balance: Decimal,
    /// 交易金额（正为收入，负为支出） [NOT NULL]
    pub amount: Decimal,
    /// 记录创建时间
    pub gmt_create: Option<String>,
    /// 当前记录年
    pub statement_year: Option<i32>,
    /// 当前记录月
    pub statement_month: Option<i32>,
    /// 当前记录日
    pub statement_day: Option<i32>,
    /// 结算状态：1.待结算 2.已结算
    pub settlement_status: Option<i32>,
}

impl Statement {
    /// 数据库表名。
    pub const TABLE: &'static str = "statement";
}


/// 对账单下载
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StatementDownload {
    /// [NOT NULL]
    pub statement_download_id: i64,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 申请时间
    pub gmt_create: Option<i64>,
    /// 筛选开始时间
    pub start_time: Option<i64>,
    /// 筛选结束时间
    pub end_time: Option<i64>,
    /// 备注
    pub remark: Option<String>,
}

impl StatementDownload {
    /// 数据库表名。
    pub const TABLE: &'static str = "statement_download";
}


/// statement_output
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StatementOutput {
    /// [NOT NULL]
    pub statement_output_id: i64,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 收入金额
    pub income: Option<Decimal>,
    /// 支出金额
    pub expenditure: Option<Decimal>,
    /// 出账时间
    pub gmt_create: Option<i64>,
    /// 单据sn
    pub record_sn: Option<String>,
    /// 单据类型
    pub record_type: Option<i32>,
    /// 单据id
    pub record_id: Option<i32>,
    /// 入账状态
    pub settlement_status: Option<i64>,
    /// 真实的结算状态：0未结算 1结算
    pub real_settlement: Option<i32>,
    /// 结算时间
    pub real_settlement_time: Option<i32>,
}

impl StatementOutput {
    /// 数据库表名。
    pub const TABLE: &'static str = "statement_output";
}


/// statement_output_old
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct StatementOutputOld {
    /// [NOT NULL]
    pub statement_output_id: i64,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 供应商id
    pub vendor_id: Option<i32>,
    /// 收入金额
    pub income: Option<Decimal>,
    /// 支出金额
    pub expenditure: Option<Decimal>,
    /// 出账时间
    pub gmt_create: Option<i64>,
    /// 单据sn
    pub record_sn: Option<String>,
    /// 单据类型
    pub record_type: Option<i32>,
    /// 单据id
    pub record_id: Option<i32>,
    /// 入账状态
    pub settlement_status: Option<i64>,
    /// 真实的结算状态：0未结算 1结算
    pub real_settlement: Option<i32>,
    /// 结算时间
    pub real_settlement_time: Option<i32>,
}

impl StatementOutputOld {
    /// 数据库表名。
    pub const TABLE: &'static str = "statement_output_old";
}


/// 二级商户映射
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct SubMerchant {
    /// 子商户映射ID [NOT NULL]
    pub id: i64,
    /// 店铺ID [NOT NULL]
    pub shop_id: i64,
    /// 支付渠道 [NOT NULL]
    pub pay_channel: String,
    /// 子商户号 [NOT NULL]
    pub sub_mch_id: String,
    /// 子商户AppID [NOT NULL]
    pub sub_appid: String,
    /// 商户名称 [NOT NULL]
    pub merchant_name: String,
    /// 状态:0待激活 1已激活 2已冻结 [NOT NULL]
    pub status: i32,
    /// 关联进件申请ID [NOT NULL]
    pub application_id: i64,
    /// 最大分账比例(%)
    pub max_split_ratio: Option<Decimal>,
    /// 逻辑删除:0未删 非0已删 [NOT NULL]
    pub is_del: i32,
    /// 创建时间 [NOT NULL]
    pub create_time: String,
    /// 修改时间 [NOT NULL]
    pub modify_time: String,
}

impl SubMerchant {
    /// 数据库表名。
    pub const TABLE: &'static str = "sub_merchant";
}


/// 用户资金表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserBalanceLog {
    /// 余额id [NOT NULL]
    pub log_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// [NOT NULL]
    pub before_balance: Decimal,
    /// 增加或减少的余额 [NOT NULL]
    pub balance: Decimal,
    /// [NOT NULL]
    pub after_balance: Decimal,
    /// 被冻结的余额 [NOT NULL]
    pub frozen_balance: Decimal,
    /// 增加或减少之前的被冻结的余额 [NOT NULL]
    pub before_frozen_balance: Decimal,
    /// 增加或减少之后的被冻结的余额 [NOT NULL]
    pub after_frozen_balance: Decimal,
    /// 该笔操作发生的时间 [NOT NULL]
    pub change_time: i32,
    /// 该笔操作的备注 [NOT NULL]
    pub change_desc: String,
    /// 类型，1：增加，2：减少 [NOT NULL]
    pub change_type: i32,
    /// 钱包类型：0余额账户 1佣金账户 [NOT NULL]
    pub wallet_type: i32,
    /// 操作后的余额数据 [NOT NULL]
    pub new_balance: Decimal,
    /// 操作后的冻结的余额 [NOT NULL]
    pub new_frozen_balance: Decimal,
}

impl UserBalanceLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_balance_log";
}


/// 用户现金日志记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserCashLog {
    /// 现金日志id [NOT NULL]
    pub cash_log_id: i32,
    /// 订单ID [NOT NULL]
    pub order_id: i32,
    /// 1 充值订单 2 订单 [NOT NULL]
    pub order_type: i32,
    /// 现金实际金额 [NOT NULL]
    pub cash_amount: Decimal,
    /// 找零金额 [NOT NULL]
    pub cash_change_amount: Decimal,
}

impl UserCashLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_cash_log";
}


/// 用户发票
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserInvoice {
    /// 会员发票信息id [NOT NULL]
    pub invoice_id: i32,
    /// 会员id [NOT NULL]
    pub user_id: i32,
    /// 专用发票审核状态：0：未申请专用发票，1：审核通过，2：已申请，待审核，3：审核未通过 [NOT NULL]
    pub status: i32,
    /// 审核失败时的回复内容 [NOT NULL]
    pub apply_reply: String,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 抬头：1：个人，2企业 [NOT NULL]
    pub title_type: i32,
    /// 单位名称 [NOT NULL]
    pub company_name: String,
    /// 纳税人识别号 [NOT NULL]
    pub company_code: String,
    /// 单位地址 [NOT NULL]
    pub company_address: String,
    /// 单位电话 [NOT NULL]
    pub company_phone: String,
    /// 开户银行 [NOT NULL]
    pub company_bank: String,
    /// 开户账号 [NOT NULL]
    pub company_account: String,
    /// 手机号 [NOT NULL]
    pub mobile: String,
    /// 邮箱 [NOT NULL]
    pub email: String,
    /// 发票内容，统一为商品明细 [NOT NULL]
    pub invoice_content: String,
}

impl UserInvoice {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_invoice";
}


/// 用户充值订单表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserRechargeOrder {
    /// 充值订单id [NOT NULL]
    pub order_id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 充值的金额 [NOT NULL]
    pub amount: Decimal,
    /// 赠送金额
    pub discount_money: Option<Decimal>,
    /// 订单添加时间 [NOT NULL]
    pub add_time: i32,
    /// 支付时间 [NOT NULL]
    pub paid_time: i32,
    /// 管理员的备注 [NOT NULL]
    pub postscript: String,
    /// 状态，0：待支付，1：已支付，2：无效 [NOT NULL]
    pub status: i32,
}

impl UserRechargeOrder {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_recharge_order";
}


/// 用户提现申请表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserWithdrawApply {
    /// 提现申请id [NOT NULL]
    pub id: i32,
    /// 用户id [NOT NULL]
    pub user_id: i32,
    /// 提现钱包：0余额 1佣金 [NOT NULL]
    pub wallet_type: i32,
    /// 申请提现的金额 [NOT NULL]
    pub amount: Decimal,
    /// 记录插入时间 [NOT NULL]
    pub add_time: i32,
    /// 完成时间 [NOT NULL]
    pub finished_time: i32,
    /// 管理员的备注 [NOT NULL]
    pub postscript: String,
    /// 处理状态，0：待处理，1：已完成，2：拒绝申请 [NOT NULL]
    pub status: i32,
    /// [JSON]提现人卡号等信息 [NOT NULL]
    pub account_data: String,
}

impl UserWithdrawApply {
    /// 数据库表名。
    pub const TABLE: &'static str = "user_withdraw_apply";
}