//! 会员用户实体 `user`。
//! 对应 `com.nextrade.bean.model.user.User`。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct User {
    /// 用户ID
    pub user_id: Option<i32>,
    /// 是否企业认证
    pub is_company_auth: Option<i32>,
    pub username: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
    /// 手机号是否已验证：1 是 0 否
    pub mobile_validated: Option<i32>,
    pub email: Option<String>,
    /// 邮箱是否已验证：1 是 0 否
    pub email_validated: Option<i32>,
    pub nickname: Option<String>,
    /// 登录密码（BCrypt）
    pub password: Option<String>,
    /// 生日 yyyy-MM-dd
    pub birthday: Option<String>,
    /// 余额
    pub balance: Option<Decimal>,
    /// 冻结余额
    pub frozen_balance: Option<Decimal>,
    /// 佣金余额
    pub commission_balance: Option<Decimal>,
    /// 冻结佣金余额
    pub frozen_commission_balance: Option<Decimal>,
    /// 积分
    pub points: Option<i32>,
    /// 成长值
    pub growth_points: Option<i32>,
    /// 默认收货地址ID
    pub address_id: Option<i32>,
    /// 注册时间（秒/毫秒时间戳，与原库一致）
    pub reg_time: Option<i64>,
    /// 最后登录时间
    pub last_login: Option<i32>,
    /// 最后登录IP
    pub last_ip: Option<String>,
    /// 会员等级ID
    pub rank_id: Option<i32>,
    /// 推荐人用户ID
    pub referrer_user_id: Option<i32>,
    pub from_tag: Option<i32>,
    /// 是否超级会员
    pub is_svip: Option<i32>,
    pub svip_expire_time: Option<i32>,
    /// 订单数
    pub order_count: Option<i32>,
    /// 订单金额
    pub order_amount: Option<Decimal>,
    /// 浏览历史商品ID JSON
    pub history_product_ids: Option<String>,
    /// 是否分销员
    pub is_distribution: Option<i32>,
    pub distribution_register_time: Option<i32>,
    pub wechat_img: Option<String>,
    /// 状态：1 正常 0 禁用
    pub status: Option<i32>,
    /// 用户类型
    pub user_type: Option<i32>,
    /// 后台备注
    pub admin_note: Option<String>,
    pub mark: Option<i32>,
}

impl User {
    pub const TABLE: &'static str = "user";

    /// 是否可登录（状态=1）。
    pub fn can_login(&self) -> bool {
        self.status.unwrap_or(0) == 1
    }

    /// 可用余额（非负校验）。
    pub fn available_balance(&self) -> Decimal {
        self.balance.unwrap_or(Decimal::ZERO).max(Decimal::ZERO)
    }

    /// 展示名：昵称优先，其次用户名/手机号。
    pub fn display_name(&self) -> String {
        self.nickname
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| self.username.clone())
            .or_else(|| self.mobile.clone())
            .unwrap_or_else(|| format!("user_{}", self.user_id.unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn display_name_priority() {
        let u = User {
            user_id: Some(7),
            nickname: Some("小明".into()),
            username: Some("mingming".into()),
            ..Default::default()
        };
        assert_eq!(u.display_name(), "小明");
    }

    #[test]
    fn balance_never_negative() {
        let u = User {
            balance: Some(Decimal::from_str("-1.00").unwrap()),
            ..Default::default()
        };
        assert_eq!(u.available_balance(), Decimal::ZERO);
    }
}