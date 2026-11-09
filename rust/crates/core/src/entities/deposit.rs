//! 保证金域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `shop_deposit`, `shop_deposit_adjust_log`, `shop_deposit_log`, `shop_deposit_offshelf_log`, `shop_deposit_payee_account`, `shop_deposit_recharge`, `shop_deposit_refund`, `shop_deposit_rule`, `shop_deposit_rule_category`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 保证金账户(店铺/供应商)
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDeposit {
    /// [NOT NULL]
    pub shop_deposit_id: i32,
    /// 持有者类型 1店铺 2供应商 [NOT NULL]
    pub owner_type: i32,
    /// 持有者id(店铺为shopId,供应商为vendorId) [NOT NULL]
    pub owner_id: i32,
    /// 实缴余额(净额)=充值入账-提现-退还-平台扣除 [NOT NULL]
    pub paid: Decimal,
    pub required_manual: Option<Decimal>,
    /// 上次应缴快照(类目变更diff用,记录调整前后值) [NOT NULL]
    pub required_snapshot: Decimal,
    /// [NOT NULL]
    pub collect_due_time: i32,
    /// [NOT NULL]
    pub supplement_due_time: i32,
    /// [NOT NULL]
    pub collect_activated: i32,
    /// 惩罚状态 0正常 1限制上架 2全店下架(DepositPenaltyAction) [NOT NULL]
    pub penalty_status: i32,
    /// 账户状态 1正常 [NOT NULL]
    pub status: i32,
    /// 创建时间 [NOT NULL]
    pub add_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
}

impl ShopDeposit {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit";
}


/// 保证金应缴调整记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositAdjustLog {
    /// [NOT NULL]
    pub shop_deposit_adjust_id: i32,
    /// 持有者类型 1店铺 2供应商 [NOT NULL]
    pub owner_type: i32,
    /// 持有者id(店铺为shopId,供应商为vendorId) [NOT NULL]
    pub owner_id: i32,
    /// 调整前应缴 [NOT NULL]
    pub before_required: Decimal,
    /// 调整后应缴 [NOT NULL]
    pub after_required: Decimal,
    /// 来源 1按类目最高 2手动固定(DepositAdjustSource) [NOT NULL]
    pub source: i32,
    /// 操作管理员 [NOT NULL]
    pub admin_id: i32,
    /// 调整原因
    pub reason: Option<String>,
    /// [NOT NULL]
    pub add_time: i32,
}

impl ShopDepositAdjustLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_adjust_log";
}


/// 保证金流水(店铺/供应商)
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositLog {
    /// [NOT NULL]
    pub shop_deposit_log_id: i32,
    /// 持有者类型 1店铺 2供应商 [NOT NULL]
    pub owner_type: i32,
    /// 持有者id(店铺为shopId,供应商为vendorId) [NOT NULL]
    pub owner_id: i32,
    /// 流水类型 1充值入账 2提现出账 3退还出账 4平台扣除 5平台调整(双向) [NOT NULL]
    pub r#type: i32,
    /// 变动金额(带符号) [NOT NULL]
    pub change_amount: Decimal,
    /// 变动前实缴 [NOT NULL]
    pub before_paid: Decimal,
    /// 变动后实缴 [NOT NULL]
    pub after_paid: Decimal,
    /// 关联单id(充值/退还/订单/售后) [NOT NULL]
    pub related_id: i32,
    /// 关联订单(平台扣除/赔付时) [NOT NULL]
    pub order_id: i32,
    /// 操作管理员 [NOT NULL]
    pub admin_id: i32,
    pub remark: Option<String>,
    /// [NOT NULL]
    pub add_time: i32,
}

impl ShopDepositLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_log";
}


/// 保证金惩罚下架记录
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositOffshelfLog {
    /// [NOT NULL]
    pub shop_deposit_offshelf_id: i32,
    /// 持有者类型 1店铺 2供应商 [NOT NULL]
    pub owner_type: i32,
    /// 持有者id(店铺为shopId,供应商为vendorId) [NOT NULL]
    pub owner_id: i32,
    /// 被惩罚下架的商品id [NOT NULL]
    pub product_id: i32,
    /// 1已下架待恢复 2已恢复 [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub off_time: i32,
    /// [NOT NULL]
    pub restore_time: i32,
    /// [NOT NULL]
    pub add_time: i32,
}

impl ShopDepositOffshelfLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_offshelf_log";
}


/// 保证金收款账户
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositPayeeAccount {
    /// [NOT NULL]
    pub shop_deposit_payee_account_id: i32,
    /// 账户类型 1银行卡 2支付宝 3微信 [NOT NULL]
    pub account_type: i32,
    /// 户名 [NOT NULL]
    pub account_name: String,
    /// 账号 [NOT NULL]
    pub account_no: String,
    /// 银行名称
    pub bank_name: Option<String>,
    /// 银行支行
    pub bank_branch: Option<String>,
    /// 1启用 0停用 [NOT NULL]
    pub status: i32,
    /// 排序 [NOT NULL]
    pub sort_order: i32,
    /// [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub update_time: i32,
}

impl ShopDepositPayeeAccount {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_payee_account";
}


/// 保证金充值申请
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositRecharge {
    /// [NOT NULL]
    pub shop_deposit_recharge_id: i32,
    /// 持有者类型 1店铺 2供应商 [NOT NULL]
    pub owner_type: i32,
    /// 持有者id(店铺为shopId,供应商为vendorId) [NOT NULL]
    pub owner_id: i32,
    /// 充值金额 [NOT NULL]
    pub amount: Decimal,
    /// 转账凭证
    pub pay_voucher: Option<String>,
    /// 充值到的收款账户id(shop_deposit_payee_account) [NOT NULL]
    pub payee_account_id: i32,
    /// 收款账户类型快照 1银行卡 2支付宝 3微信 [NOT NULL]
    pub payee_account_type: i32,
    /// 收款账户快照JSON(类型/户名/账号/银行)
    pub payee_snapshot: Option<String>,
    /// 申请备注
    pub remark: Option<String>,
    /// 0待审核 1已入账 2驳回 [NOT NULL]
    pub status: i32,
    /// [NOT NULL]
    pub audit_admin_id: i32,
    /// [NOT NULL]
    pub audit_time: i32,
    pub audit_remark: Option<String>,
    /// [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub update_time: i32,
}

impl ShopDepositRecharge {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_recharge";
}


/// 保证金提现/退还申请
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositRefund {
    /// [NOT NULL]
    pub shop_deposit_refund_id: i32,
    /// 持有者类型 1店铺 2供应商 [NOT NULL]
    pub owner_type: i32,
    /// 持有者id(店铺为shopId,供应商为vendorId) [NOT NULL]
    pub owner_id: i32,
    /// 1部分退还 2全部退还 [NOT NULL]
    pub r#type: i32,
    /// [NOT NULL]
    pub amount: Decimal,
    /// 收款信息
    pub payee_info: Option<String>,
    pub reason: Option<String>,
    /// 0待审核 1已通过 2未通过 [NOT NULL]
    pub status: i32,
    /// 打款凭证
    pub pay_voucher: Option<String>,
    /// [NOT NULL]
    pub audit_admin_id: i32,
    /// [NOT NULL]
    pub audit_time: i32,
    pub audit_remark: Option<String>,
    /// [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub update_time: i32,
}

impl ShopDepositRefund {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_refund";
}


/// 店铺保证金规则
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositRule {
    /// [NOT NULL]
    pub shop_deposit_rule_id: i32,
    /// 规则名称 [NOT NULL]
    pub rule_name: String,
    /// [NOT NULL]
    pub merchant_types: String,
    /// [NOT NULL]
    pub shop_types: String,
    /// 保证金额 [NOT NULL]
    pub deposit_amount: Decimal,
    /// 生效开始 [NOT NULL]
    pub start_time: i32,
    /// 生效结束 [NOT NULL]
    pub end_time: i32,
    /// 状态 1启用 0停用 [NOT NULL]
    pub status: i32,
    /// 是否删除 [NOT NULL]
    pub is_delete: i32,
    /// [NOT NULL]
    pub add_time: i32,
    /// [NOT NULL]
    pub update_time: i32,
}

impl ShopDepositRule {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_rule";
}


/// 店铺保证金规则适用类目
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ShopDepositRuleCategory {
    /// [NOT NULL]
    pub id: i32,
    /// 规则id [NOT NULL]
    pub rule_id: i32,
    /// 平台类目id [NOT NULL]
    pub category_id: i32,
}

impl ShopDepositRuleCategory {
    /// 数据库表名。
    pub const TABLE: &'static str = "shop_deposit_rule_category";
}