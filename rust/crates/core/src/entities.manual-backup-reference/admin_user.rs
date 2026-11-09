//! 后台用户实体 `admin_user`。
//! 对应 `com.nextrade.bean.model.authority.AdminUser`。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct AdminUser {
    /// 自增id号，管理员代码
    pub admin_id: i32,
    /// 管理员登录名
    pub username: Option<String>,
    /// 管理员类型：admin-管理员，shop-店铺
    pub admin_type: Option<String>,
    /// 手机号
    pub mobile: Option<String>,
    /// 头像
    pub avatar: Option<String>,
    /// 管理员登录密码
    pub password: Option<String>,
    /// 管理员邮箱
    pub email: Option<String>,
    /// 管理员添加时间（毫秒时间戳）
    pub add_time: Option<i64>,
    /// [JSON] 管理员管理权限列表
    pub auth_list: Option<String>,
    /// 绑定的会员id
    pub user_id: Option<i32>,
    /// 供应商id
    pub suppliers_id: Option<i32>,
    /// 角色id
    pub role_id: Option<i32>,
    /// 商户id
    pub merchant_id: Option<i32>,
    /// 父管理员id，表明当前为子管理员
    pub parent_id: Option<i32>,
    /// [JSON] 店铺存的标签
    pub menu_tag: Option<String>,
    /// [JSON] 订单导出存的标签
    pub order_export: Option<String>,
    /// [JSON] 扩展信息
    pub extra: Option<String>,
    /// 店铺id
    pub shop_id: Option<i32>,
    /// 是否停用：1 是 0 否
    pub is_using: Option<i32>,
    /// 初始密码
    pub initial_password: Option<String>,
    /// 是否删除：1 是 0 否
    pub is_deleted: Option<i32>,
}

impl AdminUser {
    /// 表名（供仓储层复用）。
    pub const TABLE: &'static str = "admin_user";

    /// 是否为超管（admin 类型且未停用未删除）。
    pub fn is_active_admin(&self) -> bool {
        self.admin_type.as_deref() == Some("admin")
            && self.is_using.unwrap_or(1) == 0
            && self.is_deleted.unwrap_or(0) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_admin() {
        let u = AdminUser {
            admin_type: Some("admin".into()),
            is_using: Some(0),
            is_deleted: Some(0),
            ..Default::default()
        };
        assert!(u.is_active_admin());
    }
}