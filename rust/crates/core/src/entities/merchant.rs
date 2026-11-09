//! 商户域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `merchant`, `merchant_account`, `merchant_apply`, `merchant_user`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 商家表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Merchant {
    /// [NOT NULL]
    pub merchant_id: i32,
    /// 入驻申请id
    pub merchant_apply_id: Option<i32>,
    /// 入驻申请用户id
    pub user_id: Option<i32>,
    /// 商户创建时间
    pub add_time: Option<i32>,
    /// 基础数据json
    pub base_data: Option<String>,
    /// 店铺数据json
    pub shop_data: Option<String>,
    /// 商户数据json
    pub merchant_data: Option<String>,
    /// 状态1正常2取消资格
    pub status: Option<i32>,
    /// null或者0表示未删除 1 表示删除
    pub is_delete: Option<i32>,
    /// 类型1个人2企业
    pub r#type: Option<i32>,
    /// 企业名称
    pub company_name: Option<String>,
    /// 申请主体信息
    pub corporate_name: Option<String>,
    /// 结算周期单位天
    pub settlement_cycle: Option<i32>,
}

impl Merchant {
    /// 数据库表名。
    pub const TABLE: &'static str = "merchant";
}


/// 商家账户表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MerchantAccount {
    /// 账号ID [NOT NULL]
    pub account_id: i32,
    /// 商户id [NOT NULL]
    pub merchant_id: i32,
    /// 提现账号类型，1：银行卡，2：支付宝，3：微信 [NOT NULL]
    pub account_type: i32,
    /// 姓名 [NOT NULL]
    pub account_name: String,
    /// 银行卡号或账号 [NOT NULL]
    pub account_no: String,
    /// 银行名称 [NOT NULL]
    pub bank_name: String,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 分行名称
    pub bank_branch: Option<String>,
    /// 店铺ID
    pub shop_id: Option<i32>,
}

impl MerchantAccount {
    /// 数据库表名。
    pub const TABLE: &'static str = "merchant_account";
}


/// 商家入驻表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MerchantApply {
    /// [NOT NULL]
    pub merchant_apply_id: i32,
    /// 用户id
    pub user_id: Option<i32>,
    /// 添加时间
    pub add_time: Option<i32>,
    /// 店铺名称
    pub shop_title: Option<String>,
    /// 1待审核10审核通过20审核未通过
    pub status: Option<i32>,
    /// 1个人2企业
    pub r#type: Option<i32>,
    /// 联系人名称
    pub contact_name: Option<String>,
    /// 联系人手机
    pub contact_mobile: Option<String>,
    /// 审核时间
    pub audit_time: Option<i32>,
    /// 审核管理员ID(admin_user.admin_id)
    pub audit_admin_id: Option<i32>,
    /// 申请资料shop信息json
    pub shop_data: Option<String>,
    /// 申请资料基本信息json
    pub base_data: Option<String>,
    /// 申请资料的商户信息json
    pub merchant_data: Option<String>,
    /// 企业名称
    pub company_name: Option<String>,
    /// 申请主体信息
    pub corporate_name: Option<String>,
    /// 审核备注
    pub audit_remark: Option<String>,
}

impl MerchantApply {
    /// 数据库表名。
    pub const TABLE: &'static str = "merchant_apply";
}


/// 商户用户表
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MerchantUser {
    /// [NOT NULL]
    pub merchant_user_id: i32,
    /// 前台用户id
    pub user_id: Option<i32>,
    /// 后台用户id
    pub admin_user_id: Option<i32>,
    /// 商户id
    pub merchant_id: Option<i32>,
    /// 是否管理员1是0不是
    pub is_admin: Option<i32>,
}

impl MerchantUser {
    /// 数据库表名。
    pub const TABLE: &'static str = "merchant_user";
}