//! 会员登录域服务（member::login）。
//!
//! 构造签名：`pub fn new(repos: Arc<Repositories>, cache: Option<CacheService>, jwt: Arc<JwtService>)`
//! —— 与既有 `catalog::product`（`cache: Option<CacheService>`）和
//! `identity::auth`（`jwt: Arc<JwtService>`）保持一致的装配风格；Redis 不可用时
//! `cache = None` 降级为「验证码/重置令牌不可用」，不阻断进程启动。
//!
//! 仓储访问为**嵌套**命名：`self.repos.member.login.<method>()`。
//!
//! 对应原 Java：
//! - `ConfigServiceImpl#sendSms` / `#sendEmail` / `#getQuickLoginSetting` / `#getWechatLoginUrl`
//! - `UserServiceImpl#checkMobile` / `#checkEmail` / `#issuePasswordReset` /
//!   `#modifyPassword` / `#bindMobile` / `#unbindWechat`
//! - `ClientLoginFilter`（C 端 JWT 签发口径：`typ = "member"`）
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! | 端点 | 语义 |
//! |------|------|
//! | `sendMobileCode`（/api + /adminapi 两条） | 校验手机号和行为验证 → 生成并发送 6 位短信码 → 写入缓存（TTL 5 分钟） |
//! | `signin` 短信登录 | 校验并消费会员短信码 → 登录已有会员或按注册开关创建新会员 |
//! | `sendEmailCode` | 校验邮箱 → 同上（`event+email`） |
//! | `checkMobile` / `checkEmail` | 校验缓存验证码 → 要求账号唯一且启用 → 生成 64 位 hex 重置令牌 → 缓存 `password-reset:{md5(token)}` → user_id（TTL 10 分钟），返回令牌 |
//! | `forgetPassword` | 校验令牌 → 校验账号启用 → `bcrypt` 重新哈希密码 → 更新 `user.password` → 消费令牌 |
//! | `bindMobile` | 校验 `login{mobile}` 验证码 → 已有账号则置 `mobile_validated=1`，否则注册新用户（受 `shopRegClosed` 开关约束）→ 处理微信授权绑定 → 返回会员 JWT |
//! | `getQuickLoginSetting` | 按 `x-client-type` 返回 `{wechatLogin, showOauth}`（`pc/wechat` 取 `openWechatOauth`，`miniProgram` 恒 1） |
//! | `getWxLoginUrl` | `x-client-type=wechat` 时拼公众号 OAuth URL 并缓存 state（300s）；`pc` 扫码需要微信二维码 API（未迁移 → 明确报错） |
//! | `unbindWechat` | 登录态；删除当前会员的 `user_authorize` 记录；未绑定 → 「该账号未绑定微信公众号」 |
//!
//! # 微信生态端点迁移状态 + 2026-11-21 建号口径变更
//!
//! `wechatServer` / `wechatEvent` / `getMobile` / `updateUserOpenid` /
//! `getWxLoginInfoByCode` / `bindWechat` 已于后续批次全部迁移落地（早期
//! 「未迁移」登记作废）。
//!
//! **建号口径变更（原版有·我方改口径，用户 2026-11-21 拍板方案 A；论坛
//! 帖 437883310890615995）**：原版 `wechatEvent` / `getWxLoginInfoByCode`
//! 对未知 openid 在微信注册开启时直接新建**空手机号**账号——手机号老
//! 用户扫码后被建成无手机号新号、与原账号分裂（原版同病）。我方改为：
//! 未知 openid 一律返回 `type=2` 先绑手机，服务端签发一次性临时凭证
//! `bindTicket`（缓存 `wechat-bind:{ticket}` → `WechatBindData`，600
//! 秒），`bindMobile` 以凭证载荷为权威身份，复用既有合并逻辑（授权改
//! 归属到手机号已有账号；无账号才以该手机号建号），空号不再产生。
//! 开关语义：`openWechatRegister` 在扫码路径不再参与决策（原「是否静默
//! 建号」已废除；其余组合此前也都回 type=2，行为不变），在 OAuth 路径
//! 保留为新用户准入门（关 → 明确拒绝，与原口径一致）；
//! `wechatRegisterBindPhone` 并入统一口径、不再单独控制绑手机流程。
//! 防重复建号：原版在建号外层有 Redis 分布式锁，我方此前漏迁；本次按
//! 新架构把锁落在唯一建号点 `bindMobile`（手机号 + openid 双键、30 秒
//! TTL、令牌释放），防并发绑手机建出重复账号/重复授权记录。
//!
//! # 其它已知差异
//!
//! - 行为验证：登录侧已接入（失败 5 次触发滑块，见 `system::captcha`）；发码侧
//!   `verifyToken` 带了必校验（一次性令牌）、没带不拦截（移动端流程无令牌），
//!   另加同目标 60 秒冷却 + 每日 10 次上限防轰炸（`guard_code_send`）；
//! - 重置令牌摘要用 **MD5**（services crate 无 `sha2` 依赖，不可改 Cargo.toml），
//!   Java 用 SHA-256；令牌本身仍为 64 位小写 hex、TTL 与 Java 相同；
//! - `bindMobile` 的微信授权绑定做了「按 openid/按用户」两段 upsert 的等价实现，
//!   但 `open_name` / `open_photo` 用 openData 里的 `nickname` / `headimgurl` 填充
//!   （Java 只写 `open_data`，两列取实体默认值）；
//! - `bindMobile` 的 `wholesalerService.autoRegisterIfEnabled`（自动成为批发商）
//!   已接线（2026-11-21 补齐：bindMobile 建号分支与短信注册登录建号分支
//!   均已调用，失败不抛错）；分销员初始化仍未迁移；
//! - Java 登录成功后会把 token 写入 Redis（供 logout 吊销）；Rust 侧 C 端登录
//!   （含 `api/public/src/lib.rs::user_login`）尚未实现该吊销表，这里保持一致不写。

use std::sync::Arc;

use base64::Engine;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::Sha256;
use nextrade_common::utils::now_seconds;
use nextrade_core::entities::User;
use nextrade_core::models::member::login::{
    authorize_type_from_client, auto_nickname, email_code_key, extract_bind_ticket,
    password_reset_key, quick_login_flags, sms_code_key, unwrap_open_data, validate_code_format,
    validate_email_basic, validate_reset_token, validate_sms_mobile, wechat_bind_key,
    wechat_h5_login_flags, wechat_state_key, BindMobileDTO, CheckEmailDTO, CheckMobileDTO,
    CodeParamDTO, ForgetPasswordDTO, LoginWechatEventVO, QuickLoginSettingVO, RegisterEmailDTO,
    RegisterSmsDTO, WechatBindData, WechatLoginUrlVO, WxLoginInfoByCodeVO, ACCOUNT_CANNOT_RESET,
    BIND_IN_PROGRESS, BIND_PASSWORD_REQUIRED, CACHE_UNAVAILABLE, CODE_INVALID, CODE_TTL_SECS,
    EMAIL_REQUIRED, IDENTITY_REQUIRED, MEMBER_TOKEN_TYPE, MOBILE_REQUIRED, PASSWORD_REQUIRED,
    PASSWORD_UPDATE_FAILED, REGISTER_CLOSED, RESET_TOKEN_INVALID, SERVICE_DATA_ERROR, SWITCH_OFF,
    SWITCH_ON, USER_STATUS_OK, WECHAT_BIND_EXPIRED, WECHAT_BIND_TTL_SECS, WECHAT_NOT_BOUND,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::CacheService;
use nextrade_infrastructure::db::repo::member::login::LoginRepository;
use nextrade_infrastructure::db::Repositories;
use nextrade_security::{hash_password, JwtService};

use super::wechat_mp;

/// 微信公众号服务器回调查询参数（GET 验证 / POST 安全模式共用）。
#[derive(Debug, Clone, Default)]
pub struct WechatServerQuery {
    pub signature: Option<String>,
    pub timestamp: Option<String>,
    pub nonce: Option<String>,
    pub echostr: Option<String>,
    pub encrypt_type: Option<String>,
    pub msg_signature: Option<String>,
}

/// 会员登录域服务。
pub struct LoginService {
    repos: Arc<Repositories>,
    cache: Option<CacheService>,
    jwt: Arc<JwtService>,
}

impl LoginService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>, cache: Option<CacheService>, jwt: Arc<JwtService>) -> Self {
        Self { repos, cache, jwt }
    }

    /// 本特性仓储（嵌套访问：`repos.member.login`）。
    #[must_use]
    pub fn repo(&self) -> &LoginRepository {
        &self.repos.member.login
    }

    // ————————————————————— 验证码发送 —————————————————————

    /// 后台发码沿用既有缓存键，会员登录验证码使用独立命名空间。
    pub async fn send_mobile_code(&self, dto: &RegisterSmsDTO) -> Result<()> {
        self.send_sms(dto, false).await
    }

    pub async fn send_member_mobile_code(&self, dto: &RegisterSmsDTO) -> Result<()> {
        let for_login = matches!(dto.event.as_deref(), None | Some("" | "login"));
        self.send_sms(dto, for_login).await
    }

    async fn send_sms(&self, dto: &RegisterSmsDTO, for_member_login: bool) -> Result<()> {
        let mobile = validate_sms_mobile(dto.mobile.as_deref())?;
        validate_optional_verify_token(dto.verify_token.as_deref())?;
        self.guard_code_send("mobile", mobile).await?;
        let code = generate_sms_code();
        let key = if for_member_login {
            member_sms_login_key(mobile)
        } else {
            sms_code_key(dto.event.as_deref(), mobile)
        };
        // 真实通道：阿里云短信（未配置 → 明确业务错误；NEXTTRADE_SMS__MOCK=1 时仅日志）
        crate::system::notify::send_sms_code(&self.repos, mobile, &code).await?;
        self.cache()?.set_json(&key, &code, CODE_TTL_SECS).await?;
        tracing::info!("短信验证码已发送: event={:?}", dto.event);
        Ok(())
    }

    /// `ClientLoginFilter` 的短信登录分支，按注册开关自动创建新会员。
    pub async fn signin_by_mobile(&self, mobile: Option<&str>, code: Option<&str>) -> Result<User> {
        let mobile = validate_sms_mobile(mobile)?;
        self.consume_member_sms_code(mobile, code).await?;
        let users = &self.repos.member.user;
        let user = if let Some(user) = users.find_by_mobile(mobile).await? {
            user
        } else {
            let closed = self
                .repo()
                .config_value("shopRegClosed")
                .await?
                .and_then(|v| parse_switch(Some(&v)))
                .unwrap_or(SWITCH_OFF);
            if closed != SWITCH_OFF {
                return Err(Error::biz(SERVICE_DATA_ERROR, REGISTER_CLOSED));
            }
            let username = self.generate_unique_username().await?;
            let nickname = auto_nickname(None, &username);
            let hash = hash_password(&uuid::Uuid::new_v4().to_string())?;
            let now = i32::try_from(now_seconds()).unwrap_or(0);
            let id = self
                .repo()
                .insert_registered_user(&username, &nickname, "", mobile, &hash, now, 0)
                .await?;
            // 新会员自动成为批发商（Java autoRegisterIfEnabled；双开关 +
            // 默认等级，失败不抛错；仅建号分支调用，存量会员登录不触发）
            crate::wholesale::wholesaler::WholesalerService::new(self.repos.clone())
                .auto_register_if_enabled(id)
                .await;
            users.find_by_id(id).await?.ok_or_else(|| Error::NotFound("用户不存在".into()))?
        };
        if !user.can_login() {
            return Err(Error::biz(403, "账号已被禁用"));
        }
        if user.mobile_validated != 1 {
            self.repo().set_mobile_validated(user.user_id).await?;
        }
        Ok(user)
    }

    /// `POST /api/user/login/sendEmailCode`。
    ///
    /// Java `ConfigServiceImpl#sendEmail`：邮箱格式校验 → 行为验证 → 生成 6 位码 →
    /// SMTP 发送 → 写缓存 `event+email`（5 分钟）。同样只写缓存。
    ///
    /// # Errors
    /// 邮箱缺失/非法 → 「邮箱不能为空」/「邮箱地址不正确」；缓存不可用 → 1001。
    pub async fn send_email_code(&self, dto: &RegisterEmailDTO) -> Result<()> {
        let email = dto
            .email
            .as_deref()
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, EMAIL_REQUIRED))?;
        let email = validate_email_basic(Some(email))?;
        validate_optional_verify_token(dto.verify_token.as_deref())?;
        self.guard_code_send("email", email).await?;
        let code = generate_sms_code();
        let key = email_code_key(dto.event.as_deref(), email);
        self.cache()?
            .set_json(&key, &code, CODE_TTL_SECS)
            .await?;
        // 真实通道：SMTP（未配置 → 明确业务错误）
        crate::system::notify::send_email_code(&self.repos, &email, "验证码", &code).await?;
        tracing::info!("邮箱验证码已发送: email={email}");
        Ok(())
    }

    // ————————————————————— 重置密码 —————————————————————

    /// `POST /api/user/login/checkMobile`：校验短信验证码并签发重置令牌。
    ///
    /// Java `UserServiceImpl#checkMobile` → `issuePasswordReset(mobile, code, false)`。
    ///
    /// # Errors
    /// 手机号/验证码为空或错误、账号不唯一或被禁用 → 1001 业务错误（Java 逐字文案）。
    pub async fn check_mobile(&self, dto: &CheckMobileDTO) -> Result<String> {
        let mobile = require_identity(dto.mobile.as_deref(), MOBILE_REQUIRED)?;
        let reset = self
            .issue_password_reset(mobile, dto.code.as_deref(), false)
            .await?;
        Ok(reset)
    }

    /// `POST /api/user/login/checkEmail`：校验邮箱验证码并签发重置令牌。
    ///
    /// # Errors
    /// 同上（邮箱维度）。
    pub async fn check_email(&self, dto: &CheckEmailDTO) -> Result<String> {
        let email = require_identity(dto.email.as_deref(), EMAIL_REQUIRED)?;
        let reset = self
            .issue_password_reset(email, dto.code.as_deref(), true)
            .await?;
        Ok(reset)
    }

    /// Java `UserServiceImpl#issuePasswordReset`。
    ///
    /// 1. `validateVerificationCode("forgetPassword" + identity, code)`；
    /// 2. 账号必须**恰好 1 个**且 `status=1`；
    /// 3. 消费验证码（删除缓存键）；
    /// 4. 生成 32 字节随机令牌（64 位小写 hex），缓存 `password-reset:{摘要}` → user_id（10 分钟）。
    async fn issue_password_reset(
        &self,
        identity: &str,
        code: Option<&str>,
        email: bool,
    ) -> Result<String> {
        if identity.is_empty() {
            return Err(Error::biz(SERVICE_DATA_ERROR, IDENTITY_REQUIRED));
        }
        let code = validate_code_format(code)?;
        let code_key = format!("forgetPassword{identity}");
        let cache = self.cache()?;
        let expected: Option<String> = cache.get_json(&code_key).await?;
        match expected {
            Some(expected) if expected == code => {}
            _ => return Err(Error::biz(SERVICE_DATA_ERROR, CODE_INVALID)),
        }

        let candidates = if email {
            self.repo().reset_candidates_by_email(identity).await?
        } else {
            self.repo().reset_candidates_by_mobile(identity).await?
        };
        if candidates.len() != 1 || candidates[0].1 != USER_STATUS_OK {
            return Err(Error::biz(SERVICE_DATA_ERROR, ACCOUNT_CANNOT_RESET));
        }
        let user_id = candidates[0].0;

        // 消费验证码（Java 用 Lua 脚本原子 GET+DEL；这里先删再写令牌，失败不恢复）
        cache.del(&code_key).await?;

        let token = generate_reset_token();
        cache
            .set_json(&password_reset_key(&md5_hex(&token)), &user_id, 600)
            .await?;
        Ok(token)
    }

    /// `POST /api/user/login/forgetPassword`：用重置令牌修改密码。
    ///
    /// Java `UserServiceImpl#modifyPassword`：令牌格式 64 hex → 缓存取 user_id →
    /// 账号启用 → 消费令牌 → `passwordEncoder.encode` → `updateById`。
    ///
    /// # Errors
    /// 令牌非法/过期、账号被禁用、写库失败 → 1001 业务错误。
    pub async fn forget_password(&self, dto: &ForgetPasswordDTO) -> Result<()> {
        let token = validate_reset_token(dto.mobile_key.as_deref())?;
        let password = dto
            .password
            .as_deref()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, PASSWORD_REQUIRED))?;

        let cache = self.cache()?;
        let key = password_reset_key(&md5_hex(token));
        let user_id: Option<i32> = cache.get_json(&key).await?;
        let user_id = user_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, RESET_TOKEN_INVALID))?;
        if self.repo().user_status(user_id).await? != Some(USER_STATUS_OK) {
            return Err(Error::biz(SERVICE_DATA_ERROR, ACCOUNT_CANNOT_RESET));
        }
        // 先一次性消费再写库（Java 注释：写库失败也不恢复凭据）
        cache.del(&key).await?;
        let hash = hash_password(password)?;
        if !self.repo().update_password(user_id, &hash).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, PASSWORD_UPDATE_FAILED));
        }
        Ok(())
    }

    // ————————————————————— 绑定手机 / 微信 —————————————————————

    /// `POST /api/user/login/bindMobile`：绑定手机号并返回会员 JWT。
    ///
    /// Java `UserServiceImpl#bindMobile` 的等价流程 + 2026-11-21 口径变更：
    /// 1. `validateVerificationCode("login" + mobile, mobileCode)`；
    /// 2. 解析微信身份：openData 带服务端签发的 `bindTicket` 时，以缓存中
    ///    的凭证载荷为权威身份（扫码/OAuth 首次登录的统一入口，不信任客户
    ///    端回传的 openid 等字段）；无票时走旧口径（客户端 openData 直传，
    ///    小程序等存量流程，行为不变）；
    /// 3. 在手机号（+openid）防并发锁内完成写入（[`Self::bind_mobile_under_lock`]）：
    ///    一次性消费凭证 → 已有该手机号则仅置 `mobile_validated=1` 并把授
    ///    权改归属到该账号（账号合并），否则检查 `shopRegClosed`（「系统已
    ///    关闭注册」）后以该手机号创建用户 → `user_authorize` 两段 upsert；
    /// 4. 返回会员访问令牌（`JwtService#issue_pair`，`typ = "member"`）。
    ///
    /// # Errors
    /// 验证码错误、授权凭证过期、注册关闭、密码为空、锁忙、数据库/缓存
    /// 失败 → 业务错误或 `Error::Database`。
    pub async fn bind_mobile(
        &self,
        dto: &BindMobileDTO,
        client_type: Option<&str>,
    ) -> Result<String> {
        let mobile = validate_sms_mobile(dto.mobile.as_deref())?.to_string();
        let password = dto
            .password
            .as_deref()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, BIND_PASSWORD_REQUIRED))?;

        // 1. 校验并一次性消费会员短信验证码（Java `validateVerificationCode`）。
        self.consume_member_sms_code(&mobile, dto.mobile_code.as_deref())
            .await?;

        // 2. 解析微信身份（凭证优先，旧 openData 直传兜底）
        let identity = self.resolve_bind_identity(dto, client_type).await?;

        // 3. 防并发锁内写入（建号/授权 upsert 的唯一入口，双键串行化）
        let cache = self.cache()?.clone();
        let locks = acquire_bind_locks(&cache, &mobile, identity.openid.as_deref()).await?;
        let outcome = self
            .bind_mobile_under_lock(
                &mobile,
                password,
                dto.referrer_user_id.unwrap_or(0),
                &identity,
            )
            .await;
        locks.release().await;
        let (user_id, username) = outcome?;

        // 4. 签发会员访问令牌（与 `api/public/src/lib.rs::user_login` 同口径）
        self.issue_member_token(user_id, &username)
    }

    /// 解析 `bindMobile` 的微信身份（二选一，见 [`ResolvedBindIdentity`]）。
    ///
    /// 带 `bindTicket`：预读服务端凭证（此处不消费，消费在持锁写入段原子
    /// 完成），载荷即权威身份，授权类型用发证路径写死的值（公众号 = 1，
    /// 不按 `x-client-type` 推导——PC 扫码端传的是 `pc`，推导会误落小程
    /// 序类型 2，导致后续扫码按类型 1 查不到授权记录）。
    /// 无票：沿用旧口径从客户端 openData 取字段，授权类型按客户端推导。
    async fn resolve_bind_identity(
        &self,
        dto: &BindMobileDTO,
        client_type: Option<&str>,
    ) -> Result<ResolvedBindIdentity> {
        // openData（兼容前端多套一层 openData）
        let open_data = unwrap_open_data(dto.open_data.as_ref());
        if let Some(ticket) = open_data.and_then(extract_bind_ticket) {
            let ticket = ticket.to_string();
            let (raw, data) = self
                .peek_wechat_bind_ticket(&ticket)
                .await?
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, WECHAT_BIND_EXPIRED))?;
            // 落库 open_data 由载荷合成（不含 ticket 本身，nickname 按 Java
            // 口径不入 open_data、单独走 open_name/昵称通道）
            let mut stored = Map::new();
            stored.insert("openid".into(), Value::String(data.openid.clone()));
            if let Some(unionid) = &data.unionid {
                stored.insert("unionid".into(), Value::String(unionid.clone()));
            }
            if !data.headimgurl.is_empty() {
                stored.insert(
                    "headimgurl".into(),
                    Value::String(data.headimgurl.clone()),
                );
            }
            return Ok(ResolvedBindIdentity {
                openid: Some(data.openid.clone()),
                unionid: data.unionid.clone(),
                nickname: Some(data.nickname.clone()).filter(|n| !n.trim().is_empty()),
                avatar: data.headimgurl.clone(),
                authorize_type: data.authorize_type,
                app_id: data.app_id.clone(),
                open_data_json: Some(Value::Object(stored).to_string()),
                ticket_consume: Some((wechat_bind_key(&ticket), raw)),
            });
        }
        // 旧口径：客户端 openData 直传（小程序等存量流程，行为与改前一致）
        let openid = open_data
            .and_then(|d| d.get("openid"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let unionid = open_data
            .and_then(|d| d.get("unionid"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let nickname = open_data
            .and_then(|d| d.get("nickname"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let avatar = open_data
            .and_then(|d| d.get("headimgurl"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let open_data_json = open_data.map(|d| {
            let mut cleaned = d.clone();
            // Java 为了避免特殊字符把 nickname 从 open_data 里移除；
            // bindTicket 属服务端凭证，同样不得落库
            cleaned.remove("nickname");
            cleaned.remove("bindTicket");
            Value::Object(cleaned).to_string()
        });
        // 店铺小程序：openData 携带自身 appId 时按绑定表推导作用域键
        // （平台/未上报/未登记 → 空串键，口径见 scope_key_for_reported_app_id）
        let reported_app_id = open_data
            .and_then(|d| d.get("appId"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let app_id = self
            .scope_key_for_reported_app_id(reported_app_id.as_deref())
            .await;
        Ok(ResolvedBindIdentity {
            openid,
            unionid,
            nickname,
            avatar,
            authorize_type: authorize_type_from_client(client_type),
            app_id,
            open_data_json,
            ticket_consume: None,
        })
    }

    /// `bind_mobile` 的写入段（调用方已持防并发锁）：一次性消费凭证 →
    /// 按手机号查/建用户 → 微信授权 upsert。返回 `(user_id, username)`。
    async fn bind_mobile_under_lock(
        &self,
        mobile: &str,
        password: &str,
        referrer_user_id: i32,
        identity: &ResolvedBindIdentity,
    ) -> Result<(i32, String)> {
        // 0. 一次性消费绑手机凭证：预读后被并发请求抢先消费 → 授权过期、
        //    整体中止（防同一凭证被两路请求重复使用）。
        if let Some((key, raw)) = &identity.ticket_consume {
            if !self.cache()?.del_if_eq(key, raw).await? {
                return Err(Error::biz(SERVICE_DATA_ERROR, WECHAT_BIND_EXPIRED));
            }
        }

        // 1. 已注册 → 只更新手机验证状态；否则注册新用户
        let existing = self.repo().find_user_by_mobile(mobile).await?;
        let (user_id, username) = match existing {
            Some((user_id, username, _status)) => {
                self.repo().set_mobile_validated(user_id).await?;
                (user_id, username)
            }
            None => {
                let closed = self
                    .repo()
                    .config_value("shopRegClosed")
                    .await?
                    .and_then(|v| parse_switch(Some(&v)))
                    .unwrap_or(SWITCH_OFF);
                if closed != SWITCH_OFF {
                    return Err(Error::biz(SERVICE_DATA_ERROR, REGISTER_CLOSED));
                }
                let username = self.generate_unique_username().await?;
                let nickname = identity
                    .nickname
                    .clone()
                    .unwrap_or_else(|| auto_nickname(None, &username));
                let hash = hash_password(password)?;
                let user_id = self
                    .repo()
                    .insert_registered_user(
                        &username,
                        &nickname,
                        &identity.avatar,
                        mobile,
                        &hash,
                        i32::try_from(now_seconds()).unwrap_or(0),
                        referrer_user_id,
                    )
                    .await?;
                // 新会员自动成为批发商（Java bindMobile 链路的
                // autoRegisterIfEnabled；仅本次新建号时调用，失败不抛错）
                crate::wholesale::wholesaler::WholesalerService::new(self.repos.clone())
                    .auto_register_if_enabled(user_id)
                    .await;
                (user_id, username)
            }
        };

        // 2. 微信授权绑定（Java `bindMobile` 的 user_authorize upsert 三支；
        //    店铺独立绑定小程序后全程按 appid 作用域键分键：第一支按
        //    (openid, 类型, app_id) 命中本维度记录；第二支只在本维度内找
        //    该用户已有行——不再像旧口径那样按 user 全局命中任意一行，
        //    避免把公众号行的 openid 改写成店铺小程序 openid；平台/公众
        //    号路径 app_id 恒为空串，与旧行为逐字一致）
        if let Some(openid) = identity.openid.as_deref() {
            let authorize_type = identity.authorize_type;
            let open_name = identity.nickname.clone().unwrap_or_default();
            let now = i32::try_from(now_seconds()).unwrap_or(0);
            if let Some((authorize_id, _owner)) = self
                .repo()
                .find_authorize_by_open_scoped(openid, authorize_type, &identity.app_id)
                .await?
            {
                self.repo()
                    .update_authorize_user(
                        authorize_id,
                        user_id,
                        identity.unionid.as_deref(),
                        identity.open_data_json.as_deref(),
                    )
                    .await?;
            } else if let Some((authorize_id, _)) = self
                .repo()
                .find_authorize_by_user_scoped(user_id, authorize_type, &identity.app_id)
                .await?
            {
                self.repo()
                    .update_authorize_open(
                        authorize_id,
                        openid,
                        identity.unionid.as_deref(),
                        identity.open_data_json.as_deref(),
                    )
                    .await?;
            } else {
                self.repo()
                    .insert_authorize_scoped(
                        authorize_type,
                        user_id,
                        openid,
                        identity.open_data_json.as_deref(),
                        &open_name,
                        &identity.avatar,
                        identity.unionid.as_deref(),
                        now,
                        &identity.app_id,
                    )
                    .await?;
            }
        }

        Ok((user_id, username))
    }

    /// 签发「先绑手机」临时凭证：微信身份写入缓存，只把随机 ticket 随
    /// `type=2` 的 openData 下发前端（凭证形态/有效期见 `WechatBindData`）。
    async fn issue_wechat_bind_ticket(&self, data: &WechatBindData) -> Result<String> {
        let ticket = uuid::Uuid::new_v4().simple().to_string();
        let raw = serde_json::to_string(data)
            .map_err(|e| Error::Internal(format!("绑手机凭证序列化失败: {e}")))?;
        self.cache()?
            .set_str(&wechat_bind_key(&ticket), &raw, WECHAT_BIND_TTL_SECS)
            .await?;
        Ok(ticket)
    }

    /// 预读绑手机凭证（不消费）：返回 `(原始缓存值, 载荷)`；真正的一次性
    /// 消费在持锁写入段用 `del_if_eq` 对原始值原子完成。
    async fn peek_wechat_bind_ticket(
        &self,
        ticket: &str,
    ) -> Result<Option<(String, WechatBindData)>> {
        let Some(raw) = self.cache()?.get_str(&wechat_bind_key(ticket)).await? else {
            return Ok(None);
        };
        let data: WechatBindData = serde_json::from_str(&raw)
            .map_err(|e| Error::Internal(format!("绑手机凭证解析失败: {e}")))?;
        Ok(Some((raw, data)))
    }

    /// `GET /api/user/login/unbindWechat`（需登录）。
    ///
    /// Java `UserServiceImpl#unbindWechat`：当前会员必须已绑定公众号，
    /// 按 openid 删除 `user_authorize` 记录。
    ///
    /// # Errors
    /// 未绑定 → 「该账号未绑定微信公众号」；数据库失败 → `Error::Database`。
    pub async fn unbind_wechat(&self, user_id: i32) -> Result<()> {
        let Some((_, open_id)) = self.repo().find_authorize_by_user(user_id).await? else {
            return Err(Error::biz(SERVICE_DATA_ERROR, WECHAT_NOT_BOUND));
        };
        if open_id.trim().is_empty() {
            return Err(Error::biz(SERVICE_DATA_ERROR, WECHAT_NOT_BOUND));
        }
        self.repo().delete_authorize_by_open(&open_id).await?;
        Ok(())
    }

    // ————————————————————— 快捷登录 / 微信登录 —————————————————————

    /// `GET /api/user/login/getQuickLoginSetting`。
    ///
    /// Java `ConfigServiceImpl#getQuickLoginSetting`：按 `x-client-type` 决定
    /// `wechatLogin` / `showOauth`（`wechat`/`pc` 取 `openWechatOauth`，
    /// `miniProgram` 恒 1，其它 0）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn quick_login_setting(
        &self,
        client_type: Option<&str>,
    ) -> Result<QuickLoginSettingVO> {
        let oauth = self
            .repo()
            .config_value("openWechatOauth")
            .await?
            .as_deref()
            .and_then(|v| parse_switch(Some(v)))
            .unwrap_or(SWITCH_OFF);
        let (wechat_login, show_oauth) = quick_login_flags(client_type, oauth);
        // 微信内 H5 登录方式（默认开启，配置缺失按 FieldSpec 默认值 1）
        let h5_mobile = self
            .repo()
            .config_value("wechatH5MobileLogin")
            .await?
            .as_deref()
            .and_then(|v| parse_switch(Some(v)))
            .unwrap_or(SWITCH_ON);
        let h5_password = self
            .repo()
            .config_value("wechatH5PasswordLogin")
            .await?
            .as_deref()
            .and_then(|v| parse_switch(Some(v)))
            .unwrap_or(SWITCH_ON);
        let (wechat_h5_mobile_login, wechat_h5_password_login) =
            wechat_h5_login_flags(client_type, h5_mobile, h5_password);
        Ok(QuickLoginSettingVO {
            wechat_login,
            show_oauth,
            wechat_h5_mobile_login,
            wechat_h5_password_login,
        })
    }

    /// `GET /api/user/login/getWxLoginUrl?url=`：微信授权 URL。
    ///
    /// Java `ConfigServiceImpl#getWechatLoginUrl`：
    /// - `x-client-type=wechat` → 拼 `buildAuthorizationUrl(redirectUrl, snsapi_userinfo, state)`
    ///   并把 `state` 缓存 300 秒；
    /// - `x-client-type=pc` → 走微信「创建二维码」API（未迁移 → 明确 1001）。
    ///
    /// # Errors
    /// 未配置 `wechatAppId`、非 wechat 客户端、缓存不可用 → 1001 业务错误。
    pub async fn wx_login_url(
        &self,
        client_type: Option<&str>,
        redirect_url: Option<&str>,
    ) -> Result<WechatLoginUrlVO> {
        let client_type = client_type.unwrap_or_default().trim().to_ascii_lowercase();
        if client_type == "pc" {
            return self.wechat_pc_qr_login().await;
        }
        if client_type != "wechat" {
            return Err(Error::biz(SERVICE_DATA_ERROR, "不支持的终端类型"));
        }
        let app_id = self
            .repo()
            .config_value("wechatAppId")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信 APPID 未配置"))?;
        let state = uuid::Uuid::new_v4().simple().to_string();
        self.cache()?
            .set_str(&wechat_state_key(&state), "valid", 300)
            .await?;
        let redirect = percent_encode_uri(redirect_url.unwrap_or(""));
        let url = format!(
            "https://open.weixin.qq.com/connect/oauth2/authorize?appid={app_id}&redirect_uri={redirect}&response_type=code&scope=snsapi_userinfo&state={state}#wechat_redirect"
        );
        Ok(WechatLoginUrlVO { url, ticket: None })
    }

    /// PC 扫码登录：调用公众号 `qrcode/create` 生成临时二维码，返回 URL + ticket。
    async fn wechat_pc_qr_login(&self) -> Result<WechatLoginUrlVO> {
        let client = crate::content::wechat_client::WechatClient::from_repos(&self.repos).await?;
        let mut token = client.access_token().await?;
        let body = serde_json::json!({
            "expire_seconds": 300,
            "action_name": "QR_STR_SCENE",
            "action_info": { "scene": { "scene_str": uuid::Uuid::new_v4().simple().to_string()[..12].to_string() } }
        });
        let make = |token: &str| {
            let url = format!(
                "https://api.weixin.qq.com/cgi-bin/qrcode/create?access_token={}",
                crate::content::wechat_client::urlencode(token)
            );
            (url, body.clone())
        };
        let (url, body) = make(&token);
        let mut raw = client.post_json(&url, &body).await?;
        let mut value = crate::content::wechat_client::parse_wechat_response(&raw)?;
        // token 失效：清缓存后重试一次（Java 对 40001/40014/42001 的处理）
        if value.get("errcode").and_then(Value::as_i64).is_some() {
            crate::content::wechat_client::invalidate_token(client.config().app_id.as_str());
            token = client.access_token().await?;
            let (url, body) = make(&token);
            raw = client.post_json(&url, &body).await?;
            value = crate::content::wechat_client::parse_wechat_response(&raw)?;
        }
        let ticket = value
            .get("ticket")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "获取微信登录二维码失败：缺少 ticket"))?;
        let qr_url = value
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "获取微信登录二维码失败：缺少 url"))?;
        Ok(WechatLoginUrlVO {
            url: qr_url.to_string(),
            ticket: Some(ticket.to_string()),
        })
    }

    /// `GET|POST /api/user/login/wechatServer`：微信公众号服务器消息回调。
    ///
    /// 对齐 Java `UserServiceImpl#wechatServer`：
    /// 1. 校验 `signature = sha1(sort(token,timestamp,nonce))`；
    /// 2. `echostr` 非空 → 原样回显（URL 验证）；
    /// 3. `encrypt_type=aes` → 校验 `msg_signature`，用 EncodingAESKey 解密 XML；
    /// 4. `subscribe` / `SCAN` 事件带 `Ticket` → 缓存 `ticket -> openid` 30 分钟；
    /// 5. 无被动回复时返回 `success`。
    ///
    /// # Errors
    /// 微信配置缺失、签名/解密失败、数据库/缓存错误。
    pub async fn wechat_server(
        &self,
        q: &WechatServerQuery,
        body: &[u8],
    ) -> Result<String> {
        let cfg = crate::content::wechat_client::load_mp_config(&self.repos)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信服务器配置未设置"))?;
        if cfg.server_token.trim().is_empty() {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                "微信服务器 Token 未配置（wechatServerToken）",
            ));
        }
        let timestamp = q.timestamp.as_deref().unwrap_or_default();
        let nonce = q.nonce.as_deref().unwrap_or_default();
        let signature = q.signature.as_deref().unwrap_or_default();
        if !wechat_mp::verify_signature(&cfg.server_token, timestamp, nonce, signature) {
            return Ok("非法请求".to_string());
        }
        if let Some(echostr) = q.echostr.as_deref().filter(|s| !s.trim().is_empty()) {
            return Ok(echostr.to_string());
        }

        let encrypt_type = q
            .encrypt_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if q.msg_signature.is_some() {
                    "aes"
                } else {
                    "raw"
                }
            });
        let xml = if encrypt_type.eq_ignore_ascii_case("aes") {
            if cfg.server_secret.trim().is_empty() {
                return Err(Error::biz(
                    SERVICE_DATA_ERROR,
                    "微信 EncodingAESKey 未配置（wechatServerSecret）",
                ));
            }
            let raw = String::from_utf8_lossy(body);
            let encrypt = wechat_mp::xml_field(&raw, "Encrypt")
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "安全模式回调缺少 Encrypt"))?;
            let msg_signature = q.msg_signature.as_deref().unwrap_or_default();
            if !wechat_mp::verify_msg_signature(
                &cfg.server_token,
                timestamp,
                nonce,
                &encrypt,
                msg_signature,
            ) {
                return Ok("非法请求".to_string());
            }
            wechat_mp::decrypt_message(&cfg.server_secret, &cfg.app_id, &encrypt)?
        } else {
            String::from_utf8_lossy(body).to_string()
        };

        if let (Some(event), Some(openid)) = (
            wechat_mp::xml_field(&xml, "Event"),
            wechat_mp::xml_field(&xml, "FromUserName"),
        ) {
            if matches!(event.as_str(), "subscribe" | "SCAN") {
                if let Some(ticket) =
                    wechat_mp::xml_field(&xml, "Ticket").filter(|s| !s.trim().is_empty())
                {
                    self.cache()?
                        .set_str(&ticket, &openid, 1800)
                        .await?;
                    tracing::info!(
                        "微信扫码登录 ticket 已缓存: event={event}, ticket={ticket}, openid={openid}"
                    );
                }
            }
        }
        Ok("success".to_string())
    }

    /// `POST /api/user/login/wechatEvent`：扫码后前端轮询登录结果。
    ///
    /// Java `UserServiceImpl#wechatEvent` 的主体流程（2026-11-21 建号口径
    /// 已按方案 A 改，见模块文档）：
    /// - `key`（二维码 ticket）取缓存 openid；
    /// - 已有 `user_authorize` → 直接签发会员 JWT（行为不变）；
    /// - 未知 openid → **一律**返回 `type=2` + openData（含服务端签发的
    ///   一次性 `bindTicket`），前端走绑定手机号合并/建号；原实现按
    ///   `openWechatRegister` 静默新建空手机号账号的分支已删除，该开关
    ///   在扫码路径不再参与决策（其余开关组合此前也都回 type=2）。
    pub async fn wechat_event(
        &self,
        dto: &nextrade_core::models::member::login::WechatEventDTO,
    ) -> Result<LoginWechatEventVO> {
        let key = dto
            .key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "未登录"))?;
        let openid: Option<String> = self.cache()?.get_str(key).await?;
        let openid = openid
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "未登录"))?;

        let user_id = self
            .repo()
            .find_authorize_by_open(&openid, 1)
            .await?
            .map(|(_, uid)| uid);
        let Some(user_id) = user_id else {
            // 未知 openid：一律先绑手机（2026-11-21 改口径，见模块文档）。
            let ticket = self
                .issue_wechat_bind_ticket(&WechatBindData {
                    openid: openid.clone(),
                    unionid: None,
                    nickname: String::new(),
                    headimgurl: String::new(),
                    authorize_type: 1,
                    app_id: String::new(),
                })
                .await?;
            return Ok(LoginWechatEventVO {
                token: None,
                r#type: 2,
                open_data: Some(serde_json::json!({
                    "openData": { "openid": openid, "bindTicket": ticket }
                })),
                return_url: None,
            });
        };
        let user = self
            .repos
            .member
            .user
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "会员不存在"))?;
        if user.status.unwrap_or(0) != USER_STATUS_OK {
            return Err(Error::biz(SERVICE_DATA_ERROR, "账号已被停用"));
        }
        let token = self.issue_member_token(user.user_id, &user.username)?;
        let now = i32::try_from(now_seconds()).unwrap_or(i32::MAX);
        let _ = self.repos.member.login.touch_last_login(user.user_id, "", now).await;
        Ok(LoginWechatEventVO {
            token: Some(token),
            r#type: 1,
            open_data: None,
            return_url: None,
        })
    }

    /// `POST /api/user/login/getMobile`：小程序获取手机号。
    ///
    /// access_token 走稳定版接口 + 进程内缓存（`stable_access_token`），
    /// 不再每次直调老 token 接口作废全局 token；`getuserphonenumber`
    /// 返回 40001/42001 时清缓存重试一次（与 PC 扫码口径一致）。
    ///
    /// # Errors
    /// 配置缺失 / 微信 errcode / 未返回手机号时返回业务错误。
    /// 解析小程序凭据与授权作用域键（店铺独立绑定小程序，我方新增）。
    ///
    /// 返回 `(appid, secret, scope_key)`：
    /// - 未上报 appid 或上报的正是平台 appid → 全局配置凭据，作用域键
    ///   为空串（与存量 `user_authorize.app_id` 哨兵口径一致）；
    /// - 上报已绑定且启用的店铺 appid → 绑定表解密 Secret，作用域键为
    ///   该 appid 本身；
    /// - 未登记 / 已停用 / 已解绑的 appid → 业务错误（硬拒绝，绝不回落
    ///   平台凭据，否则店铺小程序会串用平台身份）。
    async fn mini_program_credentials(
        &self,
        reported_app_id: Option<&str>,
    ) -> Result<(String, String, String)> {
        let platform_app_id = self
            .repo()
            .config_value("wechatMiniProgramAppId")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信小程序 APPID 未配置"))?;
        let reported = reported_app_id.map(str::trim).filter(|s| !s.is_empty());
        if let Some(app_id) = reported.filter(|a| *a != platform_app_id) {
            let row = crate::merchant::shop_mini_program::find_binding_by_app_id(
                &self.repos.pool,
                app_id,
            )
            .await?
            .filter(|r| {
                r.status == crate::merchant::shop_mini_program::STATUS_ENABLED
            })
            .ok_or_else(|| {
                Error::biz(SERVICE_DATA_ERROR, "该小程序未绑定店铺或绑定已停用")
            })?;
            let secret =
                crate::merchant::shop_mini_program::decrypt_binding_secret(&row)?;
            return Ok((app_id.to_string(), secret, app_id.to_string()));
        }
        let app_secret = self
            .repo()
            .config_value("wechatMiniProgramSecret")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信小程序 Secret 未配置"))?;
        Ok((platform_app_id, app_secret, String::new()))
    }

    /// 由客户端上报的 appid 推导授权作用域键（bindMobile 旧 openData 路径
    /// 用）：平台/未上报/未登记一律回空串键——bindMobile 是多端共用入口，
    /// 未登记 appid 的硬拒绝由前台作用域中间件负责，此处只做授权分键，
    /// 不因分键失败阻断存量绑定流。
    async fn scope_key_for_reported_app_id(&self, reported: Option<&str>) -> String {
        let Some(app_id) = reported.map(str::trim).filter(|s| !s.is_empty()) else {
            return String::new();
        };
        let platform = self
            .repo()
            .config_value("wechatMiniProgramAppId")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        if app_id == platform {
            return String::new();
        }
        match crate::merchant::shop_mini_program::find_binding_by_app_id(
            &self.repos.pool,
            app_id,
        )
        .await
        {
            Ok(Some(row))
                if row.status == crate::merchant::shop_mini_program::STATUS_ENABLED =>
            {
                app_id.to_string()
            }
            _ => String::new(),
        }
    }

    pub async fn get_mobile(&self, dto: &CodeParamDTO) -> Result<String> {
        let code = dto
            .code
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "code不能为空"))?;
        // 店铺小程序按自身 appid 取绑定 Secret（平台/未上报走全局配置）
        let (app_id, app_secret, _scope_key) = self
            .mini_program_credentials(dto.app_id.as_deref())
            .await?;
        let http = reqwest::Client::new();
        // access_token 走稳定版接口 + 进程内缓存（与公众号共用同一份缓存），
        // 不再每次直调老 token 接口作废全局上一个 token。
        let mut access_token =
            crate::content::wechat_client::stable_access_token(&app_id, &app_secret).await?;
        let body = serde_json::json!({ "code": code });
        let make_url = |token: &str| {
            format!(
                "https://api.weixin.qq.com/wxa/business/getuserphonenumber?access_token={}",
                crate::content::wechat_client::urlencode(token)
            )
        };
        let mut text = http
            .post(make_url(&access_token))
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("小程序手机号请求失败: {e}")))?
            .text()
            .await
            .map_err(|e| Error::Internal(format!("小程序手机号响应读取失败: {e}")))?;
        // token 失效（40001/42001）：清缓存后重试一次（与 PC 扫码口径一致）
        let raw_errcode = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v.get("errcode").and_then(Value::as_i64));
        if matches!(raw_errcode, Some(40001) | Some(42001)) {
            crate::content::wechat_client::invalidate_token(&app_id);
            access_token =
                crate::content::wechat_client::stable_access_token(&app_id, &app_secret)
                    .await?;
            text = http
                .post(make_url(&access_token))
                .json(&body)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| Error::Internal(format!("小程序手机号请求失败: {e}")))?
                .text()
                .await
                .map_err(|e| Error::Internal(format!("小程序手机号响应读取失败: {e}")))?;
        }
        let json = crate::content::wechat_client::parse_wechat_response(&text)?;
        let phone_info = json
            .get("phone_info")
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "小程序未返回手机号"))?;
        let phone_number = phone_info
            .get("phoneNumber")
            .and_then(Value::as_str)
            .unwrap_or("");
        let pure = phone_info
            .get("purePhoneNumber")
            .and_then(Value::as_str)
            .unwrap_or("");
        let country = phone_info
            .get("countryCode")
            .and_then(Value::as_str)
            .unwrap_or("");
        let area_code_enabled = self
            .repo()
            .config_value("isOpenMobileAreaCode")
            .await?
            .and_then(|v| v.trim().parse::<i32>().ok())
            == Some(1);
        let mobile = if area_code_enabled {
            if !country.is_empty() && !pure.is_empty() {
                format!("{country}{pure}")
            } else if !phone_number.is_empty() {
                phone_number.to_string()
            } else {
                pure.to_string()
            }
        } else {
            pure.to_string()
        };
        let mobile = mobile.trim_start_matches('+').to_string();
        if mobile.is_empty() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "未获取到手机号"));
        }
        Ok(mobile)
    }

    /// `POST /api/user/login/updateUserOpenid`（需登录）：小程序 code2Session 绑定 openid。
    ///
    /// 店铺独立绑定小程序后按 appid 分键：凭据按请求 appid 解析（店铺
    /// appid 取绑定 Secret），授权记录按 `(openid, type=2, app_id 作用域
    /// 键)` 查/写——同一买家在平台与店铺小程序下的 openid 分行存放，
    /// 不再互相覆盖。
    pub async fn update_user_openid(&self, dto: &CodeParamDTO, user_id: i32) -> Result<()> {
        let code = dto
            .code
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "缺少微信授权 code"))?;
        let (app_id, app_secret, scope_key) = self
            .mini_program_credentials(dto.app_id.as_deref())
            .await?;

        let res = crate::content::wechat_client::code2session(&app_id, &app_secret, code).await?;
        let openid = res.get("openid").and_then(|v| v.as_str()).unwrap_or("");
        if openid.is_empty() {
            let errmsg = res.get("errmsg").and_then(|v| v.as_str()).unwrap_or("微信换取 openid 失败");
            return Err(Error::biz(SERVICE_DATA_ERROR, errmsg));
        }

        let authorize_type = 2; // MINI_PROGRAM
        if let Some((auth_id, _)) = self
            .repo()
            .find_authorize_by_open_scoped(openid, authorize_type, &scope_key)
            .await?
        {
            self.repo()
                .update_authorize_user(auth_id, user_id, None, Some(&res.to_string()))
                .await?;
        } else {
            let now = now_seconds() as i32;
            self.repo()
                .insert_authorize_scoped(
                    authorize_type,
                    user_id,
                    openid,
                    Some(&res.to_string()),
                    "",
                    "",
                    None,
                    now,
                    &scope_key,
                )
                .await?;
        }
        Ok(())
    }

    /// `POST /api/user/login/getMiniLoginInfoByCode`（匿名，本平台新增）：
    /// 小程序 `wx.login` code 静默登录（店铺独立绑定小程序）。
    ///
    /// 与公众号 OAuth 路径（[`Self::wx_login_info_by_code`]）同口径：
    /// 按请求 appid 解析凭据（店铺 appid 用绑定 Secret、未登记硬拒绝）→
    /// `code2session` 换 openid → 已有 `(openid, type=2, appid 键)` 授权
    /// 直接登录（`type=1`）；未知 openid 一律回 `type=2` + 一次性
    /// `bindTicket` 先绑手机（437883 口径，会员以手机号归一——店铺小
    /// 程序与平台小程序分属不同开放平台账号，unionid 不互通，不靠它
    /// 认人），不再静默新建空手机号账号。
    ///
    /// # Errors
    /// code 为空、appid 未登记/停用、微信配置缺失、注册关闭 → 1001。
    pub async fn mini_login_info_by_code(
        &self,
        code: Option<&str>,
        app_id: Option<&str>,
    ) -> Result<WxLoginInfoByCodeVO> {
        let code = code
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "code不能为空"))?;
        let (resolved_app_id, app_secret, scope_key) =
            self.mini_program_credentials(app_id).await?;
        let res = crate::content::wechat_client::code2session(
            &resolved_app_id,
            &app_secret,
            code,
        )
        .await?;
        let openid = res
            .get("openid")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信换取 openid 失败"))?
            .to_string();
        let unionid = res
            .get("unionid")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        // 老用户（本小程序维度已授权）：直接登录
        if let Some((_, user_id)) = self
            .repo()
            .find_authorize_by_open_scoped(&openid, 2, &scope_key)
            .await?
        {
            let user = self
                .repos
                .member
                .user
                .find_by_id(user_id)
                .await?
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "会员不存在"))?;
            if user.status.unwrap_or(0) != USER_STATUS_OK {
                return Err(Error::biz(SERVICE_DATA_ERROR, "账号已被停用"));
            }
            let token = self.issue_member_token(user.user_id, &user.username)?;
            return Ok(WxLoginInfoByCodeVO {
                r#type: 1,
                token: Some(token),
                open_data: None,
            });
        }

        // 新用户：注册关闭为准入门（与 bindMobile 建号口径一致），
        // 通过后一律先绑手机（凭证 authorize_type 写死 2、作用域键写死
        // 发证时的 scope_key，不按客户端类型推导）。
        let shop_reg_closed = self
            .repo()
            .config_value("shopRegClosed")
            .await?
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        if shop_reg_closed != 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "商城已停止注册"));
        }
        let ticket = self
            .issue_wechat_bind_ticket(&WechatBindData {
                openid: openid.clone(),
                unionid: unionid.clone(),
                nickname: String::new(),
                headimgurl: String::new(),
                authorize_type: 2,
                app_id: scope_key,
            })
            .await?;
        let mut open_data = Map::new();
        open_data.insert("openid".into(), Value::String(openid));
        if let Some(unionid) = unionid {
            open_data.insert("unionid".into(), Value::String(unionid));
        }
        open_data.insert("mockMode".into(), Value::Bool(false));
        open_data.insert("bindTicket".into(), Value::String(ticket));
        Ok(WxLoginInfoByCodeVO {
            r#type: 2,
            token: None,
            open_data: Some(open_data),
        })
    }

    /// `GET /api/user/login/getWxLoginInfoByCode`：公众号 OAuth code 换用户信息。
    ///
    /// code 换 openid/unionid + sns/userinfo 后：已有授权 → 直接登录
    /// （`type=1`）；未知 openid → 注册类开关为准入门（与原口径一致），
    /// 通过后一律返回 `type=2` + openData（含一次性 `bindTicket`）走绑
    /// 手机（2026-11-21 改口径，见模块文档）。
    ///
    /// # Errors
    /// code 为空、微信配置缺失、注册关闭/授权未开/微信注册未开 → 1001。
    pub async fn wx_login_info_by_code(
        &self,
        code: Option<&str>,
    ) -> Result<WxLoginInfoByCodeVO> {
        let code = code
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "code不能为空"))?;
        let app_id = self
            .repo()
            .config_value("wechatAppId")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信 APPID 未配置"))?;
        let app_secret = self
            .repo()
            .config_value("wechatAppSecret")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信 AppSecret 未配置"))?;
        let http = reqwest::Client::new();
        let token_url = format!(
            "https://api.weixin.qq.com/sns/oauth2/access_token?appid={}&secret={}&code={}&grant_type=authorization_code",
            crate::content::wechat_client::urlencode(&app_id),
            crate::content::wechat_client::urlencode(&app_secret),
            crate::content::wechat_client::urlencode(code)
        );
        let text = http
            .get(&token_url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("微信 OAuth 请求失败: {e}")))?
            .text()
            .await
            .map_err(|e| Error::Internal(format!("微信 OAuth 响应读取失败: {e}")))?;
        let token_json = crate::content::wechat_client::parse_wechat_response(&text)?;
        let openid = token_json
            .get("openid")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信授权未返回 openid"))?
            .to_string();
        let unionid = token_json
            .get("unionid")
            .and_then(Value::as_str)
            .map(str::to_string);
        let access_token = token_json
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信授权未返回 access_token"))?
            .to_string();
        let userinfo_url = format!(
            "https://api.weixin.qq.com/sns/userinfo?access_token={}&openid={}&lang=zh_CN",
            crate::content::wechat_client::urlencode(&access_token),
            crate::content::wechat_client::urlencode(&openid)
        );
        let user_text = http
            .get(&userinfo_url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("微信用户信息请求失败: {e}")))?
            .text()
            .await
            .map_err(|e| Error::Internal(format!("微信用户信息响应读取失败: {e}")))?;
        let user_json = crate::content::wechat_client::parse_wechat_response(&user_text)?;
        let nickname = user_json
            .get("nickname")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let headimgurl = user_json
            .get("headimgurl")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        // 老用户：直接登录
        if let Some((_, user_id)) = self.repo().find_authorize_by_open(&openid, 1).await? {
            let user = self
                .repos
                .member
                .user
                .find_by_id(user_id)
                .await?
                .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "会员不存在"))?;
            if user.status.unwrap_or(0) != USER_STATUS_OK {
                return Err(Error::biz(SERVICE_DATA_ERROR, "账号已被停用"));
            }
            let token = self.issue_member_token(user.user_id, &user.username)?;
            return Ok(WxLoginInfoByCodeVO {
                r#type: 1,
                token: Some(token),
                open_data: None,
            });
        }

        // 新用户：注册类开关仍为准入门（关闭 → 明确拒绝，与原口径一致）；
        // 通过后一律先绑手机（2026-11-21 改口径，见模块文档）——原实现
        // 在 wechatRegisterBindPhone=0 时直接新建空手机号账号的分支已删
        // 除，该开关并入统一口径、不再单独决策。
        let shop_reg_closed = self
            .repo()
            .config_value("shopRegClosed")
            .await?
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let wechat_oauth = self
            .repo()
            .config_value("openWechatOauth")
            .await?
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let wechat_register = self
            .repo()
            .config_value("openWechatRegister")
            .await?
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        if shop_reg_closed != 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "商城已停止注册"));
        }
        if wechat_oauth != 1 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "公众号授权登录未开启"));
        }
        if wechat_register != 1 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "微信注册未开启"));
        }
        let ticket = self
            .issue_wechat_bind_ticket(&WechatBindData {
                openid: openid.clone(),
                unionid: unionid.clone(),
                nickname: nickname.clone(),
                headimgurl: headimgurl.clone(),
                authorize_type: 1,
                app_id: String::new(),
            })
            .await?;
        let mut open_data = Map::new();
        open_data.insert("openid".into(), Value::String(openid));
        if let Some(unionid) = unionid {
            open_data.insert("unionid".into(), Value::String(unionid));
        }
        open_data.insert("nickname".into(), Value::String(nickname));
        open_data.insert("headimgurl".into(), Value::String(headimgurl));
        open_data.insert("mockMode".into(), Value::Bool(false));
        open_data.insert("bindTicket".into(), Value::String(ticket));
        Ok(WxLoginInfoByCodeVO {
            r#type: 2,
            token: None,
            open_data: Some(open_data),
        })
    }

    /// `POST /api/user/login/bindWechat`（需登录）：公众号授权绑定。
    pub async fn bind_wechat(
        &self,
        dto: &nextrade_core::models::member::login::BindWechatParam,
        user_id: i32,
    ) -> Result<()> {
        let code = dto
            .code
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "code不能为空"))?;

        let user_auth = self.repo().find_authorize_by_user(user_id).await?;
        if let Some((_, open_id)) = user_auth {
            if !open_id.is_empty() {
                return Err(Error::biz(SERVICE_DATA_ERROR, "该账号已绑定微信公众号"));
            }
        }

        let app_id = self
            .repo()
            .config_value("wechatAppId")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信 APPID 未配置"))?;
        let app_secret = self
            .repo()
            .config_value("wechatAppSecret")
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "微信 AppSecret 未配置"))?;

        let url = format!(
            "https://api.weixin.qq.com/sns/oauth2/access_token?appid={}&secret={}&code={}&grant_type=authorization_code",
            crate::content::wechat_client::urlencode(&app_id),
            crate::content::wechat_client::urlencode(&app_secret),
            crate::content::wechat_client::urlencode(code)
        );
        let resp = reqwest::Client::new()
            .get(url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("微信 OAuth 请求失败: {e}")))?;
        let text = resp.text().await.map_err(|e| Error::Internal(e.to_string()))?;
        let json_val = crate::content::wechat_client::parse_wechat_response(&text)?;
        let open_id = json_val
            .get("openid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "该账号异常"))?;

        let authorize_type = 1; // WECHAT
        if let Some((_, existing_user_id)) = self.repo().find_authorize_by_open(open_id, authorize_type).await? {
            if existing_user_id != user_id {
                return Err(Error::biz(SERVICE_DATA_ERROR, "该微信号已绑定其他账号，请解绑后再重试"));
            }
        }

        let now = now_seconds() as i32;
        self.repo()
            .insert_authorize(
                authorize_type,
                user_id,
                open_id,
                Some(&text),
                "",
                "",
                None,
                now,
            )
            .await?;
        Ok(())
    }

    // ————————————————————— 内部 —————————————————————

    /// 缓存句柄（Redis 不可用时给出明确 1001，而不是伪造成功）。
    fn cache(&self) -> Result<&CacheService> {
        self.cache
            .as_ref()
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, CACHE_UNAVAILABLE))
    }

    async fn consume_member_sms_code(&self, mobile: &str, code: Option<&str>) -> Result<()> {
        let code = validate_code_format(code)?;
        let cache = self.cache()?;
        let key = member_sms_login_key(mobile);
        let attempts_key = format!("{mobile}:member-sms-login-attempts");
        if cache.incr(&attempts_key, CODE_TTL_SECS).await? > 5 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "验证码尝试次数过多，请稍后再试"));
        }
        if !cache.consume_matching_json(&key, code).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, CODE_INVALID));
        }
        Ok(())
    }

    /// 发码频率限制（防短信/邮件轰炸）：同一目标 60 秒冷却 + 每日上限 10 次。
    /// 计数走缓存（incr 带 TTL），缓存不可用时与发码主流程一致直接报错。
    async fn guard_code_send(&self, kind: &str, target: &str) -> Result<()> {
        let cache = self.cache()?;
        let cd_key = format!("code-send:cd:{kind}:{target}");
        if cache.get_str(&cd_key).await?.is_some() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "发送过于频繁，请稍后再试"));
        }
        let day = nextrade_common::utils::now_seconds() / 86_400;
        let cnt_key = format!("code-send:day:{kind}:{target}:{day}");
        let sent = cache.incr(&cnt_key, 86_400).await?;
        if sent > 10 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "今日发送次数已达上限，请明天再试"));
        }
        cache.set_str(&cd_key, "1", 60).await?;
        Ok(())
    }

    /// 生成唯一用户名（Java `generateUniqueUsername`：随机串 + 冲突重试）。
    async fn generate_unique_username(&self) -> Result<String> {
        for _ in 0..5 {
            let candidate = format!(
                "u{}",
                &uuid::Uuid::new_v4().simple().to_string()[..12]
            );
            if !self.repo().username_exists(&candidate).await? {
                return Ok(candidate);
            }
        }
        Err(Error::biz(
            SERVICE_DATA_ERROR,
            "生成唯一用户名失败，请稍后重试",
        ))
    }

    // ————————————————————— 第三方 OAuth 登录 —————————————————————

    /// `GET /api/user/oauth/render/{source}`：生成第三方授权地址。
    pub async fn oauth_render(
        &self,
        source: &str,
        return_url: Option<&str>,
        platform: Option<&str>,
    ) -> Result<String> {
        let source = source.to_ascii_lowercase();
        let platform = platform.unwrap_or("pc");
        let domain_code = if platform.eq_ignore_ascii_case("h5") {
            "h5Domain"
        } else {
            "pcDomain"
        };
        let domain = self
            .repo()
            .config_value(domain_code)
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "未配置站点域名"))?;
        let base = domain.trim_end_matches('/');
        let redirect_uri = if platform.eq_ignore_ascii_case("h5") {
            format!("{base}/pages/oauth/callback")
        } else {
            format!("{base}/oauth/callback")
        };
        // 第三方回调地址不能带自定义 query（必须与控制台完全一致），
        // 因此把来源编码进 state 前缀，供回调页选择正确的 provider。
        let state = format!(
            "{}:{}",
            source,
            uuid::Uuid::new_v4().simple().to_string()
        );
        let code_verifier = if source == "twitter" {
            Some(twitter_code_verifier())
        } else {
            None
        };
        let state_value = OAuthState {
            redirect_uri: redirect_uri.clone(),
            return_url: return_url.map(str::to_string),
            code_verifier: code_verifier.clone(),
        };
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, CACHE_UNAVAILABLE))?;
        cache
            .set_json(&oauth_state_key(&state), &state_value, 600)
            .await?;
        let state_encoded = oauth_pct(&state);
        let redirect_encoded = oauth_pct(&redirect_uri);
        match source.as_str() {
            "google" => {
                let client_id = self.oauth_config("googleClientId").await?;
                Ok(format!(
                    "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id={}&redirect_uri={}&scope=openid%20email%20profile&state={}&access_type=online",
                    oauth_pct(&client_id),
                    redirect_encoded,
                    state_encoded
                ))
            }
            "facebook" => {
                let client_id = self.oauth_config("facebookClientId").await?;
                Ok(format!(
                    "https://www.facebook.com/v18.0/dialog/oauth?client_id={}&redirect_uri={}&state={}&scope=public_profile,email",
                    oauth_pct(&client_id),
                    redirect_encoded,
                    state_encoded
                ))
            }
            "twitter" => {
                let client_id = self.oauth_config("twitterClientId").await?;
                let verifier = code_verifier.as_deref().unwrap_or_default();
                let challenge = twitter_code_challenge(verifier);
                Ok(format!(
                    "https://twitter.com/i/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
                    oauth_pct(&client_id),
                    redirect_encoded,
                    oauth_pct("users.read tweet.read"),
                    state_encoded,
                    oauth_pct(&challenge)
                ))
            }
            _ => Err(Error::biz(SERVICE_DATA_ERROR, "不支持的第三方登录平台")),
        }
    }

    /// `POST /api/user/oauth/callback/{source}`：第三方回调用 code 登录。
    pub async fn oauth_callback(
        &self,
        source: &str,
        code: &str,
        state: &str,
    ) -> Result<LoginWechatEventVO> {
        let source = source.to_ascii_lowercase();
        let state_key = oauth_state_key(state);
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, CACHE_UNAVAILABLE))?;
        let Some(state_value) = cache.get_json::<OAuthState>(&state_key).await? else {
            return Err(Error::biz(SERVICE_DATA_ERROR, "授权已过期，请重新登录"));
        };
        cache.del(&state_key).await?;
        let user = match source.as_str() {
            "google" => self.oauth_google_user(code, &state_value.redirect_uri).await?,
            "facebook" => self.oauth_facebook_user(code, &state_value.redirect_uri).await?,
            "twitter" => {
                self.oauth_twitter_user(
                    code,
                    &state_value.redirect_uri,
                    state_value.code_verifier.as_deref().unwrap_or_default(),
                )
                .await?
            }
            _ => return Err(Error::biz(SERVICE_DATA_ERROR, "不支持的第三方登录平台")),
        };
        let (open_id, nickname, avatar) = user;
        let authorize_type = match source.as_str() {
            "google" => 101,
            "facebook" => 102,
            _ => 103,
        };
        let mut token = None;
        if let Some((_, user_id)) = self
            .repo()
            .find_authorize_by_open(&open_id, authorize_type)
            .await?
        {
            if self.repo().user_status(user_id).await? == Some(USER_STATUS_OK) {
                token = Some(self.issue_member_token(user_id, &format!("user{user_id}"))?);
            }
        }
        let token = match token {
            Some(t) => t,
            None => {
                let username = self.generate_unique_username().await?;
                let raw_password = format!("oauth_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
                let password_hash = hash_password(&raw_password)?;
                let now = now_seconds() as i32;
                let user_id = self
                    .repo()
                    .insert_registered_user(
                        &username,
                        &nickname,
                        &avatar,
                        "",
                        &password_hash,
                        now,
                        0,
                    )
                    .await?;
                let open_data = serde_json::json!({ "source": source, "openId": open_id }).to_string();
                self.repo()
                    .insert_authorize(
                        authorize_type,
                        user_id,
                        &open_id,
                        Some(&open_data),
                        &nickname,
                        &avatar,
                        Some(&open_id),
                        now,
                    )
                    .await?;
                self.issue_member_token(user_id, &username)?
            }
        };
        Ok(LoginWechatEventVO {
            token: Some(token),
            r#type: 1,
            open_data: None,
            return_url: state_value.return_url,
        })
    }

    async fn oauth_config(&self, code: &str) -> Result<String> {
        self.repo()
            .config_value(code)
            .await?
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, format!("{code} 未配置")))
    }

    async fn oauth_google_user(&self, code: &str, redirect_uri: &str) -> Result<(String, String, String)> {
        let client_id = self.oauth_config("googleClientId").await?;
        let secret = self.oauth_config("googleClientSecret").await?;
        let client = reqwest::Client::new();
        let token: Value = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("code", code),
                ("client_id", client_id.as_str()),
                ("client_secret", secret.as_str()),
                ("redirect_uri", redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Google token 请求失败: {e}")))?
            .json()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Google token 响应解析失败: {e}")))?;
        let access = token
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "Google 未返回 access_token"))?;
        let info: Value = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access)
            .send()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Google 用户信息请求失败: {e}")))?
            .json()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Google 用户信息解析失败: {e}")))?;
        let open_id = info
            .get("id")
            .or_else(|| info.get("sub"))
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "Google 未返回用户 ID"))?;
        let nickname = info
            .get("name")
            .or_else(|| info.get("email"))
            .and_then(Value::as_str)
            .unwrap_or("Google用户");
        let avatar = info.get("picture").and_then(Value::as_str).unwrap_or("");
        Ok((open_id.to_string(), nickname.to_string(), truncate_avatar(avatar)))
    }

    async fn oauth_facebook_user(&self, code: &str, redirect_uri: &str) -> Result<(String, String, String)> {
        let client_id = self.oauth_config("facebookClientId").await?;
        let secret = self.oauth_config("facebookClientSecret").await?;
        let client = reqwest::Client::new();
        let token: Value = client
            .get("https://graph.facebook.com/v18.0/oauth/access_token")
            .query(&[
                ("client_id", client_id.as_str()),
                ("client_secret", secret.as_str()),
                ("redirect_uri", redirect_uri),
                ("code", code),
            ])
            .send()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Facebook token 请求失败: {e}")))?
            .json()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Facebook token 响应解析失败: {e}")))?;
        let access = token
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "Facebook 未返回 access_token"))?;
        let info: Value = client
            .get("https://graph.facebook.com/me")
            .query(&[("fields", "id,name,email,picture.type(large)"), ("access_token", access)])
            .send()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Facebook 用户信息请求失败: {e}")))?
            .json()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Facebook 用户信息解析失败: {e}")))?;
        let open_id = info
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "Facebook 未返回用户 ID"))?;
        let nickname = info
            .get("name")
            .or_else(|| info.get("email"))
            .and_then(Value::as_str)
            .unwrap_or("Facebook用户");
        let avatar = info
            .get("picture")
            .and_then(|p| p.get("data"))
            .and_then(|d| d.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("");
        Ok((open_id.to_string(), nickname.to_string(), truncate_avatar(avatar)))
    }

    async fn oauth_twitter_user(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<(String, String, String)> {
        let client_id = self.oauth_config("twitterClientId").await?;
        let secret = self.oauth_config("twitterClientSecret").await?;
        let basic = base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{secret}"));
        let client = reqwest::Client::new();
        let token: Value = client
            .post("https://api.twitter.com/2/oauth2/token")
            .header("Authorization", format!("Basic {basic}"))
            .form(&[
                ("code", code),
                ("grant_type", "authorization_code"),
                ("client_id", client_id.as_str()),
                ("redirect_uri", redirect_uri),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Twitter token 请求失败: {e}")))?
            .json()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Twitter token 响应解析失败: {e}")))?;
        let access = token
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "Twitter 未返回 access_token"))?;
        let info: Value = client
            .get("https://api.twitter.com/2/users/me")
            .bearer_auth(access)
            .send()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Twitter 用户信息请求失败: {e}")))?
            .json()
            .await
            .map_err(|e| Error::biz(SERVICE_DATA_ERROR, format!("Twitter 用户信息解析失败: {e}")))?;
        let data = info.get("data").unwrap_or(&Value::Null);
        let open_id = data
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "Twitter 未返回用户 ID"))?;
        let nickname = data
            .get("name")
            .or_else(|| data.get("username"))
            .and_then(Value::as_str)
            .unwrap_or("Twitter用户");
        Ok((open_id.to_string(), nickname.to_string(), String::new()))
    }

    /// 会员访问令牌（`sub=userId` / `usr=username` / `typ="member"`）。
    fn issue_member_token(&self, user_id: i32, username: &str) -> Result<String> {
        let (access, _refresh, _expires_in) = self.jwt.issue_pair(
            user_id.to_string(),
            username.to_string(),
            None,
            Some(MEMBER_TOKEN_TYPE.to_string()),
        )?;
        Ok(access)
    }
}

/// OAuth state 缓存值（Redis 键 `oauth_state:<state>`，TTL 10 分钟）。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthState {
    redirect_uri: String,
    return_url: Option<String>,
    code_verifier: Option<String>,
}

fn oauth_state_key(state: &str) -> String {
    format!("oauth_state:{state}")
}

fn oauth_pct(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn twitter_code_verifier() -> String {
    // RFC 7636：43-128 字符；SHA256(uuid+uuid) 的 base64url 结果固定 43 字符
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let raw = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    URL_SAFE_NO_PAD.encode(Sha256::digest(raw.as_bytes()))
}

fn twitter_code_challenge(verifier: &str) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

fn truncate_avatar(url: &str) -> String {
    if url.chars().count() <= 255 {
        url.to_string()
    } else {
        String::new()
    }
}

/// 必填身份（手机号/邮箱）：Java `StrUtil.isBlank(identity)` → 文案。
pub fn require_identity<'a>(value: Option<&'a str>, message: &str) -> Result<&'a str> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, message))
}

fn member_sms_login_key(mobile: &str) -> String {
    format!("{mobile}:member-sms-login")
}

/// `bindMobile` 解析后的微信身份（二选一，见 `LoginService::resolve_bind_identity`）。
struct ResolvedBindIdentity {
    openid: Option<String>,
    unionid: Option<String>,
    nickname: Option<String>,
    avatar: String,
    /// 授权类型：凭证路径用发证时写死的值；旧路径按客户端推导。
    authorize_type: i32,
    /// 授权作用域键（店铺独立绑定小程序）：店铺小程序 appid 本身，
    /// 平台/公众号为空串。与 `user_authorize.app_id` 落值口径一致。
    app_id: String,
    /// 落库 `user_authorize.open_data` 的 JSON（已剔除 nickname/bindTicket）。
    open_data_json: Option<String>,
    /// 一次性消费凭证：`(缓存键, 预读到的原始值)`，持锁写入段原子消费。
    ticket_consume: Option<(String, String)>,
}

/// `bindMobile` 防并发锁的持有令牌组（手机号键 + openid 键）。
///
/// 原版在建号外层用 Redis 分布式锁（`wechat:register:lock:{openid}`，
/// 30s）防重复建号；我方建号点已收敛到 `bindMobile`，锁随建号点落在此
/// 处并扩到手机号维度（防同手机号并发建出两个账号）。释放按持有令牌
/// `del_if_eq`（不误删他人锁），TTL 到期自动失效兜底。
struct BindLockGuard {
    cache: CacheService,
    held: Vec<(String, String)>,
}

impl BindLockGuard {
    /// 按持有令牌逆序逐个释放；失败仅记日志（TTL 兜底，不阻断主流程）。
    async fn release(self) {
        for (key, token) in self.held.iter().rev() {
            if let Err(e) = self.cache.del_if_eq(key, token).await {
                tracing::warn!("释放绑手机并发锁失败: key={key}, err={e}");
            }
        }
    }
}

/// 绑手机防并发锁的 TTL（秒，与原版 `wechat:register:lock` 的 30s 同值）。
const BIND_LOCK_TTL_SECS: u64 = 30;
/// 单键抢锁重试次数（间隔 150ms，窗口约 3 秒，覆盖一次绑定的写入耗时）。
const BIND_LOCK_ATTEMPTS: u32 = 20;

/// 依次抢占绑手机防并发锁（固定顺序：手机号 → openid，避免交叉死锁）。
/// 任一键在重试窗口内未取得 → 释放已持有键并报「绑定处理中」。
async fn acquire_bind_locks(
    cache: &CacheService,
    mobile: &str,
    openid: Option<&str>,
) -> Result<BindLockGuard> {
    let mut keys = vec![format!("wechat:bind:lock:mobile:{mobile}")];
    if let Some(openid) = openid.filter(|s| !s.is_empty()) {
        keys.push(format!("wechat:bind:lock:openid:{openid}"));
    }
    let mut held: Vec<(String, String)> = Vec::new();
    for key in keys {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut acquired = false;
        for _ in 0..BIND_LOCK_ATTEMPTS {
            if cache.set_if_absent(&key, &token, BIND_LOCK_TTL_SECS).await? {
                acquired = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
        if !acquired {
            BindLockGuard {
                cache: cache.clone(),
                held,
            }
            .release()
            .await;
            return Err(Error::biz(SERVICE_DATA_ERROR, BIND_IN_PROGRESS));
        }
        held.push((key, token));
    }
    Ok(BindLockGuard {
        cache: cache.clone(),
        held,
    })
}

/// 发码行为验证：请求带了 verifyToken 就必须校验通过（一次性令牌，防重放）；
/// 没带令牌的调用方（如移动端原生流程）不拦截，改由频率限制兜底。
fn validate_optional_verify_token(token: Option<&str>) -> Result<()> {
    if let Some(t) = token.map(str::trim).filter(|t| !t.is_empty()) {
        if !crate::system::captcha::verify_captcha_verification(t) {
            return Err(Error::biz(1002, "行为验证已失效，请重新验证"));
        }
    }
    Ok(())
}

/// 生成 6 位数字验证码（Java `RandomUtil.randomInt(100000, 999999)`）。
#[must_use]
pub fn generate_sms_code() -> String {
    let n = uuid::Uuid::new_v4().as_u128() % 900_000 + 100_000;
    n.to_string()
}

/// 生成 32 字节随机重置令牌：64 位小写十六进制
/// （Java `SecureRandom.nextBytes(32)` + `HexFormat.of().formatHex`）。
#[must_use]
pub fn generate_reset_token() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

/// MD5 小写十六进制（替代 Java `SecureUtil.sha256`，见模块文档「已知差异」）。
#[must_use]
pub fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// 解析配置开关：Java `Integer.parseInt(bizVal)`（空 → `None`，非法 → `None`）。
#[must_use]
pub fn parse_switch(raw: Option<&str>) -> Option<i32> {
    raw.map(str::trim).filter(|v| !v.is_empty())?.parse().ok()
}

/// 简化版 URI 组件编码（Java `buildAuthorizationUrl` 会对 redirect_uri 编码）。
///
/// 保留 RFC 3986 unreserved 字符，其余按 `%XX` 编码（UTF-8）。
#[must_use]
pub fn percent_encode_uri(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        let keep = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// 未迁移能力判定（供路由/文档对齐；纯函数便于测试）。
#[must_use]
pub fn is_unmigrated_wechat_endpoint(handler: &str) -> bool {
    matches!(
        handler,
        "wechatServer"
            | "wechatEvent"
            | "getMobile"
            | "updateUserOpenId"
            | "getWxLoginInfoByCode"
            | "bindWechat"
    )
}

/// 让错误类型与 Java 的 `GlobalException` 默认 code 对齐（1001）的便捷构造。
#[must_use]
pub fn biz_error(message: &'static str) -> Error {
    Error::biz(SERVICE_DATA_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::login::CODE_INVALID;

    #[test]
    fn md5_hex_matches_known_vector() {
        // RFC 1321: md5("abc") = 900150983cd24fb0d6963f7d28e17f72
        assert_eq!(md5_hex("abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn generated_codes_and_tokens_have_java_shapes() {
        for _ in 0..20 {
            let code = generate_sms_code();
            assert_eq!(code.len(), 6, "{code}");
            assert!(code.bytes().all(|b| b.is_ascii_digit()));
            let n: i32 = code.parse().unwrap();
            assert!((100_000..=999_999).contains(&n));
        }
        for _ in 0..20 {
            let token = generate_reset_token();
            assert_eq!(token.len(), 64);
            assert!(token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        }
    }

    #[test]
    fn switch_parsing_matches_java_integer_parse_behavior() {
        assert_eq!(parse_switch(Some("1")), Some(1));
        assert_eq!(parse_switch(Some("0")), Some(0));
        assert_eq!(parse_switch(Some("  1 ")), Some(1));
        assert_eq!(parse_switch(Some("")), None);
        assert_eq!(parse_switch(None), None);
        assert_eq!(parse_switch(Some("abc")), None);
    }

    #[test]
    fn percent_encoding_matches_oauth_redirect_requirement() {
        assert_eq!(
            percent_encode_uri("https://a.com/cb?x=1&y=中"),
            "https%3A%2F%2Fa.com%2Fcb%3Fx%3D1%26y%3D%E4%B8%AD"
        );
        assert_eq!(percent_encode_uri(""), "");
        assert_eq!(percent_encode_uri("abc-_.~"), "abc-_.~");
    }

    #[test]
    fn error_helpers_use_business_code_1001() {
        let e = biz_error(CODE_INVALID);
        match e {
            Error::Biz { code, msg } => {
                assert_eq!(code, 1001);
                assert_eq!(msg, "验证码错误或已过期，请重试");
            }
            other => panic!("应为业务错误: {other:?}"),
        }
        let e = require_identity(Some("  "), MOBILE_REQUIRED).unwrap_err();
        assert!(e.to_string().contains(MOBILE_REQUIRED));
        assert_eq!(require_identity(Some(" 138 "), MOBILE_REQUIRED).unwrap(), "138");
    }

    /// 函数体切片（与 refund_apply 结构测试同款思路）：取 `async fn <name>`
    /// 到下一个同级函数定义之间的源码（本文件含私有 async fn，故不带 pub
    /// 前缀匹配；同级项均为 4 空格缩进，函数体内部不会出现该形态）。
    fn fn_body<'a>(src: &'a str, name: &str) -> &'a str {
        let start = src
            .find(&format!("async fn {name}"))
            .unwrap_or_else(|| panic!("{name} 应存在"));
        let tail = &src[start..];
        let mut end = tail.len();
        for pat in ["\n    pub async fn ", "\n    async fn "] {
            if let Some(i) = tail[1..].find(pat) {
                end = end.min(i + 1);
            }
        }
        &tail[..end]
    }

    #[test]
    fn wechat_first_login_never_auto_registers_empty_mobile_user() {
        // 2026-11-21 口径（论坛帖 437883，方案 A）：扫码/OAuth 首次登录对
        // 未知 openid 不得再直接建号，必须签发 bindTicket 先走绑手机。
        let src = include_str!("login.rs");
        for name in ["wechat_event", "wx_login_info_by_code"] {
            let body = fn_body(src, name);
            assert!(
                !body.contains("insert_registered_user"),
                "{name} 不得直接新建用户（空手机号新号缺陷口径）"
            );
            assert!(
                body.contains("issue_wechat_bind_ticket"),
                "{name} 未知 openid 应签发 bindTicket 走先绑手机流程"
            );
        }
        // 全文件合法建号点恰为三处：signin_by_mobile（短信登录注册）、
        // bind_mobile_under_lock（绑手机建号）、oauth_callback（第三方
        // OAuth 注册，独立流程不在本次口径内）。新增第四处建号调用前，
        // 必须复核微信首登口径并同步改钉本断言与理由。
        // 模式串用 \u{28} 转义左括号，避免断言行自身的字面文本被计入。
        assert_eq!(
            src.matches(".insert_registered_user\u{28}").count(),
            3,
            "建号点收敛：微信首登两路径不得再建号"
        );
    }

    #[test]
    fn bind_mobile_consumes_ticket_and_writes_under_lock() {
        let src = include_str!("login.rs");
        let body = fn_body(src, "bind_mobile");
        assert!(
            body.contains("acquire_bind_locks"),
            "bind_mobile 写入前应先取得防并发锁"
        );
        assert!(
            body.contains("bind_mobile_under_lock"),
            "bind_mobile 写入应收敛到持锁写入段"
        );
        let identity = fn_body(src, "resolve_bind_identity");
        assert!(identity.contains("extract_bind_ticket"));
        assert!(identity.contains("peek_wechat_bind_ticket"));
        let write = fn_body(src, "bind_mobile_under_lock");
        assert!(
            write.contains("del_if_eq"),
            "持锁写入段应一次性原子消费 bindTicket"
        );
        assert!(
            write.contains("find_user_by_mobile"),
            "写入段应按手机号合并已有账号/建号"
        );
    }

    #[test]
    fn mini_program_login_is_app_scoped_and_never_falls_back() {
        // 店铺独立绑定小程序（论坛帖 487946）结构不变量：
        // 1) 小程序登录按 (openid, type=2, appid 作用域键) 查授权，
        //    未知 openid 签发 authorize_type=2 的 bindTicket 先绑手机，
        //    且不得直接建号（与 437883 口径一致）；
        // 2) 凭据解析对未登记 appid 必须报错，绝不回落平台 Secret；
        // 3) 绑定写入段三支全部走 *_scoped 仓储函数（appid 分键）。
        let src = include_str!("login.rs");
        let body = fn_body(src, "mini_login_info_by_code");
        assert!(
            body.contains("find_authorize_by_open_scoped"),
            "小程序登录应按 appid 作用域键查授权"
        );
        assert!(
            body.contains("issue_wechat_bind_ticket"),
            "小程序登录未知 openid 应签发 bindTicket"
        );
        assert!(
            body.contains("authorize_type: 2"),
            "小程序 bindTicket 的 authorize_type 应写死 2"
        );
        assert!(
            !body.contains("insert_registered_user"),
            "小程序登录不得直接建号"
        );
        let creds = fn_body(src, "mini_program_credentials");
        assert!(
            creds.contains("find_binding_by_app_id"),
            "店铺 appid 凭据应查绑定表"
        );
        assert!(
            creds.contains("该小程序未绑定店铺或绑定已停用"),
            "未登记 appid 应硬拒绝而非回落平台凭据"
        );
        let write = fn_body(src, "bind_mobile_under_lock");
        assert!(
            write.contains("find_authorize_by_open_scoped")
                && write.contains("find_authorize_by_user_scoped")
                && write.contains("insert_authorize_scoped"),
            "绑定写入段三支应全部按 appid 分键"
        );
    }

    #[test]
    fn unmigrated_wechat_endpoints_are_exactly_the_six_java_flows() {
        for h in [
            "wechatServer",
            "wechatEvent",
            "getMobile",
            "updateUserOpenId",
            "getWxLoginInfoByCode",
            "bindWechat",
        ] {
            assert!(is_unmigrated_wechat_endpoint(h), "{h}");
        }
        for h in ["bindMobile", "unbindWechat", "checkMobile", "getWxLoginUrl"] {
            assert!(!is_unmigrated_wechat_endpoint(h), "{h}");
        }
    }
}
