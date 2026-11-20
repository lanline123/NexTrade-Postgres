//! 认证服务：后台登录、令牌刷新、密码管理。
//! 对应原 Java `AdminUserServiceImpl.login` + Spring Security 过滤器链。

use std::sync::Arc;

use nextrade_common::config::JwtConfig;
use nextrade_core::entities::AdminUser;
use nextrade_core::models::{LoginDTO, LoginVO, MobileLoginDTO};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::CacheService;
use nextrade_infrastructure::db::Repositories;
use nextrade_security::{hash_password, verify_password, JwtService};
use validator::Validate;

use crate::identity::permission::is_shop_family_type;

pub struct AuthService {
    repos: Arc<Repositories>,
    jwt: Arc<JwtService>,
    jwt_cfg: JwtConfig,
    cache: Option<CacheService>,
}

impl AuthService {
    pub fn new(
        repos: Arc<Repositories>,
        jwt: Arc<JwtService>,
        jwt_cfg: JwtConfig,
        cache: Option<CacheService>,
    ) -> Self {
        Self {
            repos,
            jwt,
            jwt_cfg,
            cache,
        }
    }

    /// 后台登录：校验参数 → 查用户 → 校验密码 → 签发令牌。
    pub async fn login(&self, dto: &LoginDTO, client_ip: &str) -> Result<LoginVO> {
        dto.validate()
            .map_err(|e| Error::Validation(e.to_string()))?;

        // 失败次数达到阈值时要求滑块验证码（Java `CaptchaHandler` + `ERROR_THRESHOLD`）
        crate::system::captcha::validate_login_captcha(
            &dto.username,
            dto.captcha_verification.as_deref(),
        )?;

        let user = self
            .repos
            .identity
            .auth
            .find_by_username(&dto.username)
            .await?
            .ok_or_else(|| Error::biz(400, "用户名或密码错误"))?;

        if !user.is_active_admin() {
            return Err(Error::biz(403, "账号已被停用"));
        }

        let hash = user.password.as_str();
        if !verify_password(&dto.password, hash)? {
            let count = crate::system::captcha::increment_login_error(&dto.username);
            tracing::warn!(
                "登录失败: username={}, ip={}, fail_count={}",
                dto.username,
                client_ip,
                count
            );
            return Err(Error::biz(400, "用户名或密码错误"));
        }
        crate::system::captcha::reset_login_error(&dto.username);

        self.issue_for_user(&user, client_ip).await
    }

    /// 后台手机验证码登录。
    ///
    /// 验证码由 `POST /adminapi/login/sendMobileCode` 写入缓存
    /// `login + mobile`，与 Java `SmsCodeAuthenticationProvider` 一致。
    pub async fn login_mobile(&self, dto: &MobileLoginDTO, client_ip: &str) -> Result<LoginVO> {
        dto.validate()
            .map_err(|e| Error::Validation(e.to_string()))?;

        let mobile = dto.mobile.trim();
        let code = dto.mobile_code.trim();
        if mobile.is_empty() || code.is_empty() {
            return Err(Error::biz(400, "手机号或验证码不能为空"));
        }

        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| Error::biz(1001, "验证码服务不可用，请稍后重试"))?;
        let key = format!("login{mobile}");
        let expected: Option<String> = cache.get_json(&key).await?;
        if expected.as_deref().map(str::trim) != Some(code) {
            return Err(Error::biz(400, "验证码错误或已过期"));
        }

        let user = self
            .repos
            .identity
            .auth
            .find_by_mobile(mobile)
            .await?
            .ok_or_else(|| Error::biz(400, "该手机号未绑定后台账号"))?;

        if !user.is_active_admin() {
            return Err(Error::biz(403, "账号已被停用"));
        }

        // 验证码一次性使用
        let _ = cache.del(&key).await;
        self.issue_for_user(&user, client_ip).await
    }

    /// 登录签发时的店铺作用域（选项 B：登录派生默认店）。
    ///
    /// - 账号列 `admin_user.shop_id > 0` 时直接采用（历史口径）；
    /// - 店铺型账号列为空时按绑定表派生：启用绑定（`is_using = 0`）中取
    ///   第一条（仓储已按店主优先排序），即店主的唯一店；
    /// - 平台 / 供应商账号不派生（平台账号在绑定表里也有行，绝不能因此
    ///   被写上店铺作用域）。
    ///
    /// 生产现状下商家账号与绑定 `is_using` 全为 1（口径待单独批准的数据
    /// 修正），派生恒为空——仅部署本代码不会提前打开商家登录与作用域。
    async fn resolve_login_shop_scope(&self, user: &AdminUser) -> Result<Option<i32>> {
        if user.shop_id > 0 {
            return Ok(Some(user.shop_id));
        }
        if !is_shop_family_type(&user.admin_type) {
            return Ok(None);
        }
        let bindings = self
            .repos
            .merchant
            .shop_manage
            .find_admin_bindings(user.admin_id)
            .await?;
        Ok(bindings
            .into_iter()
            .find(|b| b.is_using == 0 && b.shop_id > 0)
            .map(|b| b.shop_id))
    }

    /// 登录成功后统一签发令牌、写登录日志。
    async fn issue_for_user(&self, user: &AdminUser, _client_ip: &str) -> Result<LoginVO> {
        let scoped_shop_id = self.resolve_login_shop_scope(user).await?;
        // 供应商/供货商账号的租户 ID 存在 admin_user.suppliers_id；店铺账号存 shop_id。
        let scoped_vendor_id = match user.admin_type.as_str() {
            "vendor" | "suppliers" if user.suppliers_id > 0 => Some(user.suppliers_id),
            _ => None,
        };

        let (access, refresh, expires_in) = self.jwt.issue_pair_with_tenant(
            user.admin_id.to_string(),
            user.username.clone(),
            Some(user.role_id),
            Some(user.admin_type.clone()),
            scoped_shop_id,
            scoped_vendor_id,
        )?;

        let log_info = format!("管理员登录:{}", user.username);
        let admin_log = crate::identity::admin_log::AdminLogService::new(self.repos.clone());
        let (scope_type, scope_id) = match scoped_shop_id {
            Some(id) if is_shop_family_type(&user.admin_type) => ("shop", id),
            _ => ("admin", 0),
        };
        let _ = admin_log
            .record_log(user.admin_id, scope_type, scope_id, &log_info, None, None)
            .await;

        tracing::info!(
            "管理员登录成功: {} ({}, shop_id={:?})",
            user.username,
            user.admin_id,
            scoped_shop_id
        );

        Ok(LoginVO {
            token: access.clone(),
            admin_type: Some(user.admin_type.clone()),
            access_token: access,
            refresh_token: refresh,
            token_type: "Bearer".into(),
            expires_in,
            admin_id: user.admin_id,
            username: user.username.clone(),
        })
    }

    /// 选店（choose）后换发带店铺作用域的令牌对（选项 A：切换即换发）。
    ///
    /// 与登录签发同一出口；在此重新加载账号并复核启用状态与绑定有效性
    /// （纵深防御，不单信调用方的校验结果）。角色取 choose 解析出的绑定
    /// 角色（无效时回退账号自身角色）。刷新链原样复制 claims，作用域在
    /// 令牌有效期内保持；绑定被停用/解绑后旧令牌的残留窗口与一切 JWT
    /// 方案相同（TTL 内有效）。
    ///
    /// # Errors
    /// 账号不存在 / 已停用、无绑定或绑定已停用、签发失败。
    pub async fn reissue_scoped_token(
        &self,
        admin_id: i32,
        shop_id: i32,
        role_id: Option<i32>,
    ) -> Result<LoginVO> {
        let user = self
            .repos
            .identity
            .auth
            .find_by_id(admin_id)
            .await?
            .ok_or_else(|| Error::biz(400, "管理员不存在"))?;
        if !user.is_active_admin() {
            return Err(Error::biz(403, "账号已被停用"));
        }
        let binding = self
            .repos
            .merchant
            .shop_manage
            .find_admin_shop_binding(shop_id, admin_id)
            .await?
            .ok_or_else(|| Error::biz_msg("没有此店铺权限"))?;
        if binding.is_using != 0 {
            return Err(Error::biz_msg("用户已经被禁用"));
        }
        let role = role_id.filter(|r| *r > 0).or(Some(user.role_id));
        let (access, refresh, expires_in) = self.jwt.issue_pair_with_tenant(
            user.admin_id.to_string(),
            user.username.clone(),
            role,
            Some(user.admin_type.clone()),
            Some(shop_id),
            None,
        )?;
        tracing::info!(
            "选店换发作用域令牌: {} ({}, shop_id={})",
            user.username,
            user.admin_id,
            shop_id
        );
        Ok(LoginVO {
            token: access.clone(),
            admin_type: Some(user.admin_type.clone()),
            access_token: access,
            refresh_token: refresh,
            token_type: "Bearer".into(),
            expires_in,
            admin_id: user.admin_id,
            username: user.username.clone(),
        })
    }

    /// 刷新访问令牌：校验刷新令牌 → 重新签发。
    pub fn refresh(&self, refresh_token: &str) -> Result<LoginVO> {
        let claims = self.jwt.decode(refresh_token)?;
        if claims.ttype != nextrade_security::jwt::TokenType::Refresh {
            return Err(Error::Unauthorized("需要刷新令牌".into()));
        }
        let (access, refresh, expires_in) = self.jwt.issue_pair_with_tenant(
            claims.sub.clone(),
            claims.usr.clone(),
            claims.role,
            claims.typ.clone(),
            claims.shop_id,
            claims.vendor_id,
        )?;
        Ok(LoginVO {
            token: access.clone(),
            admin_type: claims.typ.clone(),
            access_token: access,
            refresh_token: refresh,
            token_type: "Bearer".into(),
            expires_in,
            admin_id: claims.sub.parse().unwrap_or_default(),
            username: claims.usr,
        })
    }

    /// 修改密码（校验旧密码）。
    pub async fn change_password(
        &self,
        admin_id: i32,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        if new_password.len() < 6 {
            return Err(Error::Validation("新密码长度不能少于6位".into()));
        }
        let user = self
            .repos
            .identity
            .auth
            .find_by_id(admin_id)
            .await?
            .ok_or_else(|| Error::NotFound("管理员不存在".into()))?;
        let hash = user.password.as_str();
        if !verify_password(old_password, hash)? {
            return Err(Error::biz(400, "原密码错误"));
        }
        let new_hash = hash_password(new_password)?;
        self.repos
            .identity
            .auth
            .update_password(admin_id, &new_hash)
            .await?;
        tracing::info!("管理员修改密码成功: {admin_id}");
        Ok(())
    }

    /// 根据访问令牌解析当前管理员（供中间件/handler 使用）。
    pub fn current_admin(&self, authorization: Option<&str>) -> Result<AdminUserClaims> {
        let claims = self.jwt.from_authorization(authorization)?;
        let admin_id = claims
            .sub
            .parse::<i32>()
            .map_err(|_| Error::Unauthorized("令牌主体无效".into()))?;
        Ok(AdminUserClaims {
            admin_id,
            username: claims.usr,
            role_id: claims.role,
            admin_type: claims.typ,
        })
    }

    /// 后台用户分页（管理后台列表）。
    pub async fn list_admin_users(
        &self,
        keyword: Option<&str>,
        admin_type: Option<&str>,
        page: &nextrade_core::types::PageQuery,
    ) -> Result<(u64, Vec<AdminUser>)> {
        let (size, _) = page.limit_offset();
        self.repos
            .identity
            .auth
            .paginate(keyword, admin_type, page.page as i64, size)
            .await
    }

    /// JWT 配置（供上层注入）。
    pub fn jwt(&self) -> &JwtService {
        &self.jwt
    }

    #[allow(dead_code)]
    fn jwt_cfg(&self) -> &JwtConfig {
        &self.jwt_cfg
    }
}

/// 已认证的管理员身份。
#[derive(Debug, Clone)]
pub struct AdminUserClaims {
    pub admin_id: i32,
    pub username: String,
    pub role_id: Option<i32>,
    pub admin_type: Option<String>,
}
