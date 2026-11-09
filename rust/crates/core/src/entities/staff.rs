//! 服务人员域实体（上门服务/钱包/提现）
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `service_staff`, `service_staff_wallet`, `service_staff_wallet_log`, `service_staff_withdraw`

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 服务人员表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ServiceStaff {
    /// 主键ID [NOT NULL]
    pub id: i32,
    /// 姓名 [NOT NULL]
    pub name: String,
    /// 昵称
    pub nickname: Option<String>,
    /// 头像
    pub avatar: Option<String>,
    /// 联系电话 [NOT NULL]
    pub phone: String,
    /// 关联会员用户ID
    pub member_id: Option<i32>,
    /// 所属门店ID [NOT NULL]
    pub store_id: i32,
    /// 类型：1-配送员，2-服务员 [NOT NULL]
    pub r#type: i32,
    /// 账号状态：0-禁用，1-启用 [NOT NULL]
    pub status: i32,
    /// 工作状态：0-已下班，1-空闲中，2-配送中/服务中 [NOT NULL]
    pub work_status: i32,
    /// 已完成订单数 [NOT NULL]
    pub completed_order_count: i32,
    /// 已完成订单金额 [NOT NULL]
    pub completed_order_amount: Decimal,
    /// 创建时间 [NOT NULL]
    pub create_time: i32,
    /// 更新时间 [NOT NULL]
    pub update_time: i32,
    /// 是否删除：0-否，1-是 [NOT NULL]
    pub is_delete: i32,
}

impl ServiceStaff {
    /// 数据库表名。
    pub const TABLE: &'static str = "service_staff";
}


/// O2O配送员/服务员钱包
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ServiceStaffWallet {
    /// 钱包ID [NOT NULL]
    pub wallet_id: i32,
    /// 关联 service_staff.staff_id [NOT NULL]
    pub staff_id: i32,
    /// 可用余额 [NOT NULL]
    pub balance: Decimal,
    /// 冻结余额 [NOT NULL]
    pub frozen_balance: Decimal,
    /// 累计收入 [NOT NULL]
    pub total_income: Decimal,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
}

impl ServiceStaffWallet {
    /// 数据库表名。
    pub const TABLE: &'static str = "service_staff_wallet";
}


/// O2O配送员/服务员钱包流水
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ServiceStaffWalletLog {
    /// 流水ID [NOT NULL]
    pub log_id: i64,
    /// 配送员/服务员ID [NOT NULL]
    pub staff_id: i32,
    /// 关联订单ID(周期购为子订单ID)
    pub order_id: Option<i32>,
    /// 关联提现单ID
    pub withdraw_id: Option<i32>,
    /// 交易编号：订单编号或提现编号
    pub transaction_sn: Option<String>,
    /// 变动金额：正=入账 负=扣减 [NOT NULL]
    pub amount: Decimal,
    /// 变动后余额 [NOT NULL]
    pub balance_after: Decimal,
    /// 变动后冻结余额 [NOT NULL]
    pub frozen_after: Decimal,
    /// 业务来源：1=配送服务费 2=服务员服务费 [NOT NULL]
    pub business_type: i32,
    /// 类型：1=服务费入账 2=提现完成(实扣冻结) 3=扣减 4=提现申请(余额→冻结) 5=提现驳回(冻结→回退余额) [NOT NULL]
    pub r#type: i32,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    pub create_time: Option<i64>,
}

impl ServiceStaffWalletLog {
    /// 数据库表名。
    pub const TABLE: &'static str = "service_staff_wallet_log";
}


/// 配送员/服务员提现申请
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ServiceStaffWithdraw {
    /// 提现单ID [NOT NULL]
    pub withdraw_id: i32,
    /// 交易编号（业务唯一） [NOT NULL]
    pub withdraw_sn: String,
    /// 配送员/服务员ID [NOT NULL]
    pub staff_id: i32,
    /// 业务来源：1=配送服务费 2=服务员服务费 [NOT NULL]
    pub business_type: i32,
    /// 申请提现金额 [NOT NULL]
    pub amount: Decimal,
    /// 0=待审核 1=已打款 2=已驳回 [NOT NULL]
    pub status: i32,
    /// 关联 user_withdraw_account.account_id [NOT NULL]
    pub account_id: i32,
    /// 冗余：1银行卡 2支付宝 3微信 [NOT NULL]
    pub account_type: i32,
    /// 冗余收款人姓名 [NOT NULL]
    pub account_name: String,
    /// 冗余收款账号 [NOT NULL]
    pub account_no: String,
    /// 冗余开户行
    pub bank_name: Option<String>,
    /// 管理员备注
    pub admin_remark: Option<String>,
    /// 打款凭证图片URL
    pub payment_voucher: Option<String>,
    /// 拒绝理由
    pub reject_reason: Option<String>,
    /// 审核管理员ID
    pub audit_admin_id: Option<i32>,
    /// 审核时间
    pub audit_time: Option<i64>,
    /// 申请时间 [NOT NULL]
    pub apply_time: i64,
    /// 快照：申请时的频次单位 day/week/month [NOT NULL]
    pub frequency_unit: String,
    /// 快照：所属计数周期起（含） [NOT NULL]
    pub period_start: i64,
    /// 快照：所属计数周期止（不含） [NOT NULL]
    pub period_end: i64,
    /// 创建时间
    pub create_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
    /// 0=有效 1=删除 [NOT NULL]
    pub is_delete: i32,
}

impl ServiceStaffWithdraw {
    /// 数据库表名。
    pub const TABLE: &'static str = "service_staff_withdraw";
}