//! 会员用户服务：注册、登录、资料、余额操作。
//! 对应原 Java `UserServiceImpl`。

use rust_decimal::Decimal;
use std::sync::Arc;

use nextrade_core::entities::User;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;
use nextrade_security::{hash_password, verify_password};

pub struct UserService {
    repos: Arc<Repositories>,
}

impl UserService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 按 ID 查询会员。
    pub async fn find(&self, user_id: i32) -> Result<Option<User>> {
        self.repos.member.user.find_by_id(user_id).await
    }

    /// 手机号注册（密码 BCrypt 存储，与原 Java 兼容）。
    pub async fn register(&self, mobile: &str, password: &str, nickname: Option<&str>) -> Result<i32> {
        if mobile.len() < 6 {
            return Err(Error::Validation("手机号格式错误".into()));
        }
        if password.len() < 6 {
            return Err(Error::Validation("密码长度不能少于6位".into()));
        }
        if self.repos.member.user.find_by_mobile(mobile).await?.is_some() {
            return Err(Error::biz(400, "该手机号已注册"));
        }
        let hash = hash_password(password)?;
        let user = User {
            username: mobile.to_string(),
            mobile: mobile.to_string(),
            nickname: nickname.map(String::from),
            password: hash,
            status: Some(1),
            mobile_validated: 1,
            // reg_time 为 int unsigned 秒级时间戳（非毫秒）
            reg_time: nextrade_common::utils::now_seconds() as i32,
            balance: Decimal::ZERO,
            points: 0,
            order_count: 0,
            ..Default::default()
        };
        let user_id = self.repos.member.user.insert(&user).await?;
        // 新会员自动成为批发商（Java autoRegisterIfEnabled；双开关 +
        // 默认等级，失败不抛错。register_with_salesman 经本方法建号，
        // 一并覆盖，与其分销绑定同为尽力而为口径）
        crate::wholesale::wholesaler::WholesalerService::new(self.repos.clone())
            .auto_register_if_enabled(user_id)
            .await;
        Ok(user_id)
    }

    /// 带分销归因的注册（原版 `UserServiceImpl#registerByMobile` 口径）：
    /// 注册参数带 salesmanId 时，注册成功后写入客户绑定（salesman_customer）。
    /// 绑定为尽力而为：失败仅告警，不回滚已建成的账号。
    pub async fn register_with_salesman(
        &self,
        mobile: &str,
        password: &str,
        nickname: Option<&str>,
        salesman_id: Option<i32>,
    ) -> Result<i32> {
        let user_id = self.register(mobile, password, nickname).await?;
        if let Some(sid) = salesman_id.filter(|&id| id > 0) {
            let svc = crate::marketing::salesman_generation::SalesmanGenerationService::new(
                self.repos.clone(),
            );
            if let Err(e) = svc.on_user_registered(user_id, sid, "注册绑定").await {
                tracing::warn!("注册分销绑定失败 user={user_id} salesman={sid}: {e}");
            }
        }
        Ok(user_id)
    }

    /// 账号密码登录。兼容用户名、手机号和跨境版的邮箱账号。
    pub async fn login(&self, identity: &str, password: &str) -> Result<User> {
        let users = &self.repos.member.user;
        let user = if let Some(user) = users.find_by_mobile(identity).await? {
            Some(user)
        } else if let Some(user) = users.find_by_username(identity).await? {
            Some(user)
        } else if identity.contains('@') {
            users.find_by_email(identity).await?
        } else {
            None
        }
        .ok_or_else(|| Error::biz(400, "账号或密码错误"))?;
        if !user.can_login() {
            return Err(Error::biz(403, "账号已被禁用"));
        }
        // password 为 NOT NULL 裸 String，无需 Option 解包
        let hash = user.password.as_str();
        if !verify_password(password, hash)? {
            return Err(Error::biz(400, "账号或密码错误"));
        }
        Ok(user)
    }

    /// 修改资料（昵称/头像）。
    pub async fn update_profile(&self, user_id: i32, nickname: Option<&str>, avatar: Option<&str>) -> Result<()> {
        self.repos.member.user.update_profile(user_id, nickname, avatar).await
    }

    /// 余额入账（充值/退款），带非负约束。
    pub async fn credit_balance(&self, user_id: i32, amount: Decimal) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Err(Error::Validation("入账金额必须大于0".into()));
        }
        self.repos.member.user.adjust_balance(user_id, amount).await
    }

    /// 余额出账（下单支付），余额不足返回错误。
    pub async fn debit_balance(&self, user_id: i32, amount: Decimal) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Err(Error::Validation("扣款金额必须大于0".into()));
        }
        let affected = self
            .repos.member.user
            .try_debit_balance(user_id, amount)
            .await?;
        if !affected {
            return Err(Error::biz(400, "余额不足"));
        }
        Ok(())
    }
}
