//! 会员登录域（member::login）DTO / VO / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.user.LoginController`（`@RequestMapping("/api/user/login")`）
//! - `com.nextrade.adminapi.controller.user.LoginController`（`@RequestMapping("/adminapi/login")`）
//! - `com.nextrade.service.user.impl.UserServiceImpl#checkMobile/#checkEmail/
//!   #modifyPassword/#bindMobile/#wechatEvent/#getMobile/#updateUserOpenId/
//!   #getWxLoginInfoByCode/#bindWechat/#unbindWechat`
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl#sendSms/#sendEmail/
//!   #getQuickLoginSetting/#getWechatLoginUrl`
//! - `com.nextrade.security.api.filter.ClientLoginFilter`（C 端 `/api/user/login/signin`
//!   **不在本特性契约内**；本特性的数据契约见 [`crate paths`] 的 16 条）
//!
//! JSON 字段保持 camelCase；Java `LoginWechatEventVO.type` / `WxLoginInfoByCodeVO.type`
//! 用 `#[serde(rename = "type")]` 保留关键字。
//!
//! # 与原 Java 的已知差异
//!
//! 1. **验证码 / 行为验证**：Java `sendSms` / `sendEmail` 先调用
//!    `CaptchaHandler#validateCaptcha`（AJ-Captcha 滑动拼图，Redis + 第三方组件），
//!    本构建无该组件 → 无法校验 `verifyToken`。本实现**跳过行为验证**并在
//!    服务层注释/报告中标注（否则所有发送验证码接口恒失败）。
//! 2. **短信 / 邮件通道**：Java 经 RabbitMQ + 服务商发送；本构建未迁移发送通道，
//!    验证码只写入缓存（Redis 键与 Java 相同：`event + mobile/email`，TTL 5 分钟），
//!    不实际发送。部署时接入短信/邮件服务后即可复用本实现。
//! 3. **重置令牌哈希**：Java 用 `SecureUtil.sha256(token)`；services crate 没有
//!    `sha2` 依赖（不可改 Cargo.toml），本实现改用 **MD5**（`md-5` 已是现有依赖）
//!    作为缓存键后缀 —— 令牌本身仍是 64 位小写十六进制（32 字节随机），TTL 10 分钟。
//! 4. **JWT**：本特性 16 条里只有 `bindMobile` 返回会员访问令牌（Java
//!    `jwtUtil.generateToken`）；本实现用既有 `JwtService#issue_pair`
//!    （`sub=userId` / `usr=username` / `typ="member"`，与 `api/public/src/lib.rs`
//!    的会员登录完全一致），只返回 access token。
//! 5. **微信生态**：`wechatEvent` / `getMobile` / `updateUserOpenid` /
//!    `getWxLoginInfoByCode` / `bindWechat` / `wechatServer` 依赖微信开放平台
//!    （OAuth/code2Session/小程序手机号/二维码/回调验签）——本段为早期构建
//!    登记，上述端点已于后续批次全部迁移落地。
//! 6. **首次微信登录建号口径（2026-11-21，我方改口径）**：原版在微信注册
//!    开启时对未知 openid 直接新建空手机号账号（论坛帖 437883310890615995
//!    实证为缺陷：手机号老用户扫码后被建成无手机号新号）。我方改为未知
//!    openid 一律先绑手机：服务端签发一次性临时凭证（[`WechatBindData`]，
//!    600 秒），`bindMobile` 以凭证载荷为权威身份完成合并/建号，空手机号
//!    账号不再产生。详见 services `member::login` 模块文档。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `UserServiceImpl.PASSWORD_RESET_PREFIX`。
pub const PASSWORD_RESET_PREFIX: &str = "password-reset:";
/// Java `getWechatLoginUrl` 的 state 缓存键前缀。
pub const WECHAT_STATE_PREFIX: &str = "wechat::state:";
/// 微信首次登录「先绑手机」临时凭证的缓存键前缀（我方改口径新增，见
/// [`WechatBindData`]）：`wechat-bind:{ticket}`。
pub const WECHAT_BIND_PREFIX: &str = "wechat-bind:";

/// 验证码有效期（Java `5, TimeUnit.MINUTES`）。
pub const CODE_TTL_SECS: u64 = 300;
/// 密码重置令牌有效期（Java `10, TimeUnit.MINUTES`）。
pub const RESET_TOKEN_TTL_SECS: u64 = 600;
/// 微信 state 有效期（Java `300` 秒）。
pub const WECHAT_STATE_TTL_SECS: u64 = 300;
/// 微信绑手机临时凭证有效期（600 秒）：须覆盖「跳绑手机页 → 获取短信验证码
/// （300 秒有效）→ 提交绑定」的完整操作窗口，取值与重置令牌 TTL 同档。
pub const WECHAT_BIND_TTL_SECS: u64 = 600;

/// 会员 JWT 的 `typ`（与 `api/public/src/lib.rs::user_login` 一致）。
pub const MEMBER_TOKEN_TYPE: &str = "member";

// —— Java 逐字错误文案 ——
/// `RegisterSmsDTO` / `CheckMobileDTO` 的 `@NotNull` 文案。
pub const MOBILE_REQUIRED: &str = "手机号码不能为空";
/// `RegisterEmailDTO` / `CheckEmailParam` 的 `@NotNull` 文案。
pub const EMAIL_REQUIRED: &str = "邮箱不能为空";
/// `CheckMobileDTO` / `CheckEmailParam` 的 `@NotNull` 文案。
pub const CODE_REQUIRED: &str = "验证码不能为空";
/// `ForgetPasswordDTO` 的 `@NotNull` 文案。
pub const RESET_KEY_REQUIRED: &str = "手机号/邮箱不能为空";
/// `ForgetPasswordDTO.password` 的 `@NotNull` 文案。
pub const PASSWORD_REQUIRED: &str = "新密码不能为空";
/// `BindMobileDTO.password` 的 `@NotNull` 文案（绑定手机时密码字段）。
pub const BIND_PASSWORD_REQUIRED: &str = "密码不能为空";
/// `ConfigServiceImpl#sendEmail` 的邮箱格式校验文案。
pub const EMAIL_INVALID: &str = "邮箱地址不正确";
/// `UserServiceImpl#validateVerificationCode`。
pub const CODE_INVALID: &str = "验证码错误或已过期，请重试";
/// `UserServiceImpl#issuePasswordReset`：手机号/邮箱为空。
pub const IDENTITY_REQUIRED: &str = "手机号/邮箱不能为空";
/// `UserServiceImpl#issuePasswordReset`：账号不唯一/禁用。
pub const ACCOUNT_CANNOT_RESET: &str = "账号无法自动重置密码，请联系客服";
/// `UserServiceImpl#modifyPassword`：令牌非法/过期。
pub const RESET_TOKEN_INVALID: &str = "重置凭据无效或已过期，请重新获取";
/// `UserServiceImpl#modifyPassword`：写库失败。
pub const PASSWORD_UPDATE_FAILED: &str = "密码修改失败，请重新获取验证码";
/// `UserServiceImpl#bindMobile`：注册关闭。
pub const REGISTER_CLOSED: &str = "系统已关闭注册";
/// `UserServiceImpl#bindWechat`：已绑定。
pub const WECHAT_ALREADY_BOUND: &str = "该账号已绑定微信公众号";
/// `UserServiceImpl#bindWechat`：微信账号异常。
pub const WECHAT_ACCOUNT_ABNORMAL: &str = "该账号异常";
/// `UserServiceImpl#bindWechat`：openid 已绑定其他账号。
pub const WECHAT_BOUND_OTHER: &str = "该微信号已绑定其他账号，请解绑后再重试";
/// `UserServiceImpl#unbindWechat`：未绑定。
pub const WECHAT_NOT_BOUND: &str = "该账号未绑定微信公众号";
/// `UserServiceImpl#wechatEvent`：key 为空/openid 缺失。
pub const NOT_LOGIN: &str = "未登录";
/// 绑手机临时凭证缺失/过期/已被消费（我方改口径新增文案）。
pub const WECHAT_BIND_EXPIRED: &str = "微信授权已过期，请重新扫码或授权后再绑定";
/// 绑手机防并发锁在重试窗口内未取得（我方新增文案）。
pub const BIND_IN_PROGRESS: &str = "绑定处理中，请稍后重试";
/// `BindWechatParam` / `LoginGetMobileParam` 的 `@NotBlank` 文案。
pub const CODE_PARAM_REQUIRED: &str = "code不能为空";
/// `ConfigServiceImpl#getQuickLoginSetting` 使用 `x-client-type`。
pub const CLIENT_TYPE_HEADER: &str = "x-client-type";

// —— 未迁移能力（行为验证）的统一错误文案 ——
/// 行为验证未迁移。
pub const CAPTCHA_NOT_MIGRATED: &str = "行为验证（AJ-Captcha）未迁移：本构建无法校验 verifyToken，已按 Java 同款流程跳过该步（见模块文档）";
/// Redis 不可用（服务以 `Option<CacheService>` 降级）。
pub const CACHE_UNAVAILABLE: &str =
    "缓存服务不可用（Redis 未连接），无法处理验证码/重置令牌相关请求";

/// `POST /api/user/login/sendMobileCode` / `POST /adminapi/login/sendMobileCode`
/// 入参（Java `com.nextrade.bean.dto.login.RegisterSmsDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegisterSmsDTO {
    /// 验证码行为（Java 用作缓存键前缀，如 `forgetPassword` / `login`）
    pub event: Option<String>,
    /// 手机号码
    pub mobile: Option<String>,
    /// 行为验证码加密验签（本构建未接入 AJ-Captcha，见模块文档）
    pub verify_token: Option<String>,
}

/// `POST /api/user/login/sendEmailCode` 入参（Java `RegisterEmailDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RegisterEmailDTO {
    /// 验证码行为
    pub event: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 行为验证码加密验签
    pub verify_token: Option<String>,
}

/// `POST /api/user/login/checkMobile` 入参（Java `CheckMobileDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CheckMobileDTO {
    /// 手机号码
    pub mobile: Option<String>,
    /// 验证码
    pub code: Option<String>,
}

/// `POST /api/user/login/checkEmail` 入参（Java `CheckEmailParam`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CheckEmailDTO {
    /// 邮箱
    pub email: Option<String>,
    /// 验证码
    pub code: Option<String>,
}

/// `POST /api/user/login/forgetPassword` 入参（Java `ForgetPasswordDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ForgetPasswordDTO {
    /// 加密号码 / 重置令牌（由 `checkMobile` / `checkEmail` 返回）
    pub mobile_key: Option<String>,
    /// 新密码
    pub password: Option<String>,
}

/// `POST /api/user/login/bindMobile` 入参（Java `BindMobileDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BindMobileDTO {
    /// 手机号码
    pub mobile: Option<String>,
    /// 短信验证码
    pub mobile_code: Option<String>,
    /// 密码
    pub password: Option<String>,
    /// 第三方数据（微信 openid / unionid / nickname / headimgurl）
    pub open_data: Option<Map<String, Value>>,
    /// 推荐人会员 ID
    pub referrer_user_id: Option<i32>,
}

/// `POST /api/user/login/wechatEvent` 入参（Java `WechatEventDTO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatEventDTO {
    /// 扫码事件 key（Java 用它在 Redis 取 openid）
    pub key: Option<String>,
}

/// `POST /api/user/login/getMobile` / `updateUserOpenid` 入参
/// （Java `LoginGetMobileParam` / `LoginUpdateUserOpenIdParam`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CodeParamDTO {
    /// 微信 code
    pub code: Option<String>,
    /// 发起请求的小程序 AppID（店铺独立绑定小程序，我方新增）：
    /// 店铺小程序构建会上报自身 appid，服务端按绑定表取该店 Secret
    /// 换取 openid/手机号；缺省/平台 appid 走平台全局配置。
    pub app_id: Option<String>,
}

/// `POST /api/user/login/bindWechat` 入参（Java `BindWechatParam`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BindWechatParam {
    /// 微信公众号 code
    pub code: Option<String>,
}

/// `GET /api/user/login/getWxLoginUrl?url=` 查询参数。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WxLoginUrlQuery {
    /// 回跳地址（Java `@RequestParam(value="url", required=false)`）
    pub url: Option<String>,
}

/// 微信授权 URL（Java `WechatLoginUrlVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatLoginUrlVO {
    /// 授权 URL
    pub url: String,
    /// 二维码 ticket（PC 扫码登录时返回；公众号 URL 场景为 null）
    pub ticket: Option<String>,
}

/// 快捷登录设置（Java `ConfigServiceImpl#getQuickLoginSetting` 返回的 Map）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLoginSettingVO {
    /// 是否展示微信登录（`wechat`/`pc` 取 `openWechatOauth`；`miniProgram` 恒 1）
    pub wechat_login: i32,
    /// 是否展示授权登录（Java `showOauth` = `wechatLogin`）
    pub show_oauth: i32,
    /// 微信内 H5 是否开启手机号（验证码）登录（`wechatH5MobileLogin`，默认 1；
    /// 仅 `x-client-type = wechat` 时按配置下发，其它端恒 1）
    pub wechat_h5_mobile_login: i32,
    /// 微信内 H5 是否开启账号密码登录（`wechatH5PasswordLogin`，默认 1；
    /// 仅 `x-client-type = wechat` 时按配置下发，其它端恒 1）
    pub wechat_h5_password_login: i32,
}

/// 微信扫码登录事件返回（Java `LoginWechatEventVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginWechatEventVO {
    /// 会员 JWT（`type=1` 时返回）
    pub token: Option<String>,
    /// 1-正常用户登录 2-跳转到绑定手机号
    #[serde(rename = "type")]
    pub r#type: i32,
    /// openData（`type=2` 时返回，供绑定手机号使用）
    pub open_data: Option<Value>,
    /// 登录后前端跳转地址
    pub return_url: Option<String>,
}

/// 微信首次登录「先绑手机」临时凭证的服务端载荷（我方改口径新增，
/// 原版无此结构：原版在微信注册开启时对未知 openid 直接建空手机号账号）。
///
/// 扫码（`wechatEvent`）/ 公众号 OAuth（`getWxLoginInfoByCode`）遇到未知
/// openid 时，服务端把微信身份写入缓存（键 [`wechat_bind_key`]，TTL
/// [`WECHAT_BIND_TTL_SECS`]），只把随机 `bindTicket` 随 `type=2` 的
/// openData 下发给前端；前端在绑手机页回传 ticket，`bindMobile` 以缓存
/// 载荷为权威身份（不信任客户端回传的 openid 等字段），并在绑定成功时
/// 一次性消费该凭证。`authorize_type` 由发证路径写死（公众号 = 1），
/// 避免 PC 扫码端按 `x-client-type=pc` 推导出错误的小程序类型。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatBindData {
    /// 微信 openid（公众号维度）。
    pub openid: String,
    /// 微信 unionid（OAuth 路径才有；扫码路径为空）。
    pub unionid: Option<String>,
    /// 微信昵称（OAuth 路径来自 sns/userinfo；扫码路径为空串）。
    pub nickname: String,
    /// 微信头像 URL（同上）。
    pub headimgurl: String,
    /// 授权类型（Java `AuthorizeTypeEnum`：1 微信公众号）。
    pub authorize_type: i32,
    /// 授权作用域键（店铺独立绑定小程序，我方新增）：小程序路径
    /// （`authorize_type = 2`）发证时写入——店铺小程序为其 appid 本身，
    /// 平台小程序与公众号路径为空串（与 `user_authorize.app_id` 的哨兵
    /// 口径一致）。旧凭证无此字段，serde default 兜底为空串。
    #[serde(default)]
    pub app_id: String,
}

/// 通过微信 code 获取微信用户信息（Java `WxLoginInfoByCodeVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WxLoginInfoByCodeVO {
    /// 1-直接登录成功 2-需要绑定手机号
    #[serde(rename = "type")]
    pub r#type: i32,
    /// 会员 JWT（`type=1`）
    pub token: Option<String>,
    /// 微信授权数据（`type=2`）
    pub open_data: Option<Map<String, Value>>,
}

/// 短信验证码的 Redis 键：Java `dto.getEvent() + dto.getMobile()`。
///
/// Java 侧 `event` 可缺省（`sendSms` 里直接拼接，null 会得到 `"null"+mobile` 的字符串）；
/// 本实现 `None` 视为空串，并在服务层要求事件非空（否则不同业务会共用键）。
#[must_use]
pub fn sms_code_key(event: Option<&str>, mobile: &str) -> String {
    format!("{}{}", event.unwrap_or(""), mobile)
}

/// 国内和国际短信号码，允许国际号码保留 `+` 前缀。
pub fn validate_sms_mobile(mobile: Option<&str>) -> crate::Result<&str> {
    let mobile = mobile
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| crate::Error::biz(SERVICE_DATA_ERROR, MOBILE_REQUIRED))?;
    let digits = mobile.strip_prefix('+').unwrap_or(mobile);
    if !(6..=20).contains(&digits.len()) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, "手机号码格式错误"));
    }
    Ok(mobile)
}

/// 邮箱验证码的 Redis 键：Java `dto.getEvent() + dto.getEmail()`。
#[must_use]
pub fn email_code_key(event: Option<&str>, email: &str) -> String {
    format!("{}{}", event.unwrap_or(""), email)
}

/// 密码重置令牌的缓存键后缀：`password-reset:` + 令牌摘要（摘要由服务层计算）。
#[must_use]
pub fn password_reset_key(token_hash: &str) -> String {
    format!("{PASSWORD_RESET_PREFIX}{token_hash}")
}

/// 微信 state 的缓存键：`wechat::state:{state}`。
#[must_use]
pub fn wechat_state_key(state: &str) -> String {
    format!("{WECHAT_STATE_PREFIX}{state}")
}

/// 微信绑手机临时凭证的缓存键：`wechat-bind:{ticket}`。
#[must_use]
pub fn wechat_bind_key(ticket: &str) -> String {
    format!("{WECHAT_BIND_PREFIX}{ticket}")
}

/// 从（已 [`unwrap_open_data`] 解嵌套的）openData 中提取绑手机临时凭证
/// `bindTicket`：缺失、非字符串、空白 → `None`（走旧 openData 直传口径）。
#[must_use]
pub fn extract_bind_ticket(open_data: &Map<String, Value>) -> Option<&str> {
    open_data
        .get("bindTicket")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// 校验短信/邮件验证码格式（Java `code.matches("[0-9]{6}")`）。
///
/// # Errors
/// 非 6 位数字 → [`CODE_INVALID`]（Java 逐字文案）。
pub fn validate_code_format(code: Option<&str>) -> crate::Result<&str> {
    let code = code.ok_or_else(|| crate::Error::biz(SERVICE_DATA_ERROR, CODE_INVALID))?;
    let ok = code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit());
    if !ok {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, CODE_INVALID));
    }
    Ok(code)
}

/// 校验重置令牌格式（Java `token.matches("[0-9a-f]{64}")`）。
///
/// # Errors
/// 非 64 位小写十六进制 → [`RESET_TOKEN_INVALID`]。
pub fn validate_reset_token(token: Option<&str>) -> crate::Result<&str> {
    let token = token.ok_or_else(|| crate::Error::biz(SERVICE_DATA_ERROR, RESET_TOKEN_INVALID))?;
    let ok = token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !ok {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, RESET_TOKEN_INVALID));
    }
    Ok(token)
}

/// 邮箱格式校验（Java `Validator.isEmail` 的**简化**等价：`x@y.z` 形态）。
///
/// # Errors
/// 不含 `@` 或域名无点 → [`EMAIL_INVALID`]。
pub fn validate_email_basic(email: Option<&str>) -> crate::Result<&str> {
    let email = email.ok_or_else(|| crate::Error::biz(SERVICE_DATA_ERROR, EMAIL_INVALID))?;
    let (local, domain) = email
        .split_once('@')
        .ok_or_else(|| crate::Error::biz(SERVICE_DATA_ERROR, EMAIL_INVALID))?;
    if local.is_empty() || !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.')
    {
        return Err(crate::Error::biz(SERVICE_DATA_ERROR, EMAIL_INVALID));
    }
    Ok(email)
}

/// Java `BindMobileDTO.openData` 的多层兼容处理：前端可能多套一层 `openData`。
#[must_use]
pub fn unwrap_open_data(open_data: Option<&Map<String, Value>>) -> Option<&Map<String, Value>> {
    let data = open_data?;
    match data.get("openData") {
        Some(Value::Object(inner)) => Some(inner),
        _ => Some(data),
    }
}

/// 客户端类型 → 微信授权类型（Java `AuthorizeTypeEnum`：1 微信 / 2 小程序）。
///
/// Java `HeaderUtils#getClientType()`：`wechat` → 公众号(1)，其余（miniProgram 等）→ 小程序(2)。
#[must_use]
pub fn authorize_type_from_client(client_type: Option<&str>) -> i32 {
    match client_type {
        Some(c) if c.eq_ignore_ascii_case("wechat") => 1,
        _ => 2,
    }
}

/// 快捷登录开关（Java `ConfigServiceImpl#getQuickLoginSetting`）。
///
/// - `x-client-type = wechat | pc` → `openWechatOauth` 配置值；
/// - `x-client-type = miniProgram` → 恒 1；
/// - 其它/缺失 → 0。
///
/// 返回 `(wechatLogin, showOauth)`；Java 里 `showOauth` 就是 `wechatLogin`。
#[must_use]
pub fn quick_login_flags(client_type: Option<&str>, open_wechat_oauth: i32) -> (i32, i32) {
    let wechat_login = match client_type {
        Some(c) if c.eq_ignore_ascii_case("wechat") || c.eq_ignore_ascii_case("pc") => {
            open_wechat_oauth
        }
        Some(c) if c.eq_ignore_ascii_case("miniProgram") => 1,
        _ => 0,
    };
    (wechat_login, wechat_login)
}

/// 微信内 H5 登录方式开关（NexTrade 扩展，对应 Tigshop v5.8.63
/// 「微信环境内的 H5 登录方式可配置」）。
///
/// 仅 `x-client-type = wechat`（微信内置浏览器）时按后台配置
/// `wechatH5MobileLogin` / `wechatH5PasswordLogin` 下发；
/// 其它客户端一律 `(1, 1)`（不限制，避免影响普通浏览器/小程序）。
///
/// 返回 `(wechatH5MobileLogin, wechatH5PasswordLogin)`。
/// 两者同时为 0 时的兜底（保留密码登录）由前端处理，后端如实下发。
#[must_use]
pub fn wechat_h5_login_flags(
    client_type: Option<&str>,
    mobile_login: i32,
    password_login: i32,
) -> (i32, i32) {
    match client_type {
        Some(c) if c.eq_ignore_ascii_case("wechat") => (mobile_login, password_login),
        _ => (1, 1),
    }
}

/// 自动注册昵称：`usernamePrefix` 存在则拼接前缀，否则 `user_{name}`
/// （Java `StrUtil.isNotBlank(prefix) ? prefix + name : "user_" + name`）。
#[must_use]
pub fn auto_nickname(prefix: Option<&str>, username: &str) -> String {
    match prefix.map(str::trim).filter(|p| !p.is_empty()) {
        Some(prefix) => format!("{prefix}{username}"),
        None => format!("user_{username}"),
    }
}

/// Java `Constants.YES`（用户状态正常）。
pub const USER_STATUS_OK: i32 = 1;
/// Java `Constants.NO` / 注册开关「关闭」。
pub const SWITCH_OFF: i32 = 0;
/// Java `Constants.YES` / 注册开关「开启」。
pub const SWITCH_ON: i32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_and_token_format_validation_matches_java_regex() {
        assert!(validate_code_format(Some("123456")).is_ok());
        assert!(validate_code_format(Some("12345")).is_err());
        assert!(validate_code_format(Some("12345a")).is_err());
        assert!(validate_code_format(None).is_err());

        let token = "a".repeat(64);
        assert!(validate_reset_token(Some(&token)).is_ok());
        assert!(
            validate_reset_token(Some(&"A".repeat(64))).is_err(),
            "必须小写"
        );
        assert!(validate_reset_token(Some(&"a".repeat(63))).is_err());
        assert!(validate_reset_token(None).is_err());
    }

    #[test]
    fn email_validation_is_basic_but_rejects_obvious_invalid() {
        assert!(validate_email_basic(Some("a@b.com")).is_ok());
        assert!(validate_email_basic(Some("a@b")).is_err());
        assert!(validate_email_basic(Some("ab.com")).is_err());
        assert!(validate_email_basic(Some("@b.com")).is_err());
        assert!(validate_email_basic(None).is_err());
    }

    #[test]
    fn quick_login_flags_match_java_client_type_switch() {
        assert_eq!(quick_login_flags(Some("wechat"), 1), (1, 1));
        assert_eq!(quick_login_flags(Some("pc"), 0), (0, 0));
        assert_eq!(quick_login_flags(Some("miniProgram"), 0), (1, 1));
        assert_eq!(quick_login_flags(Some("h5"), 1), (0, 0));
        assert_eq!(quick_login_flags(None, 1), (0, 0));
    }

    #[test]
    fn wechat_h5_login_flags_only_apply_to_wechat_client() {
        // 微信内：按配置下发
        assert_eq!(wechat_h5_login_flags(Some("wechat"), 0, 1), (0, 1));
        assert_eq!(wechat_h5_login_flags(Some("WECHAT"), 1, 0), (1, 0));
        assert_eq!(wechat_h5_login_flags(Some("wechat"), 0, 0), (0, 0));
        // 其它端：恒开启，不受配置影响
        assert_eq!(wechat_h5_login_flags(Some("h5"), 0, 0), (1, 1));
        assert_eq!(wechat_h5_login_flags(Some("pc"), 0, 0), (1, 1));
        assert_eq!(wechat_h5_login_flags(None, 0, 0), (1, 1));
    }

    #[test]
    fn cache_keys_match_java_string_concatenation() {
        assert_eq!(
            sms_code_key(Some("forgetPassword"), "13800000000"),
            "forgetPassword13800000000"
        );
        assert_eq!(sms_code_key(None, "13800000000"), "13800000000");
        assert_eq!(email_code_key(Some("login"), "a@b.com"), "logina@b.com");
        assert_eq!(password_reset_key("abc"), "password-reset:abc");
        assert_eq!(wechat_state_key("s1"), "wechat::state:s1");
    }

    #[test]
    fn open_data_unwraps_nested_layer() {
        let inner: Map<String, Value> =
            serde_json::from_str(r#"{"openid":"o1","unionid":"u1"}"#).unwrap();
        let mut outer = Map::new();
        outer.insert("openData".into(), Value::Object(inner.clone()));
        assert_eq!(unwrap_open_data(Some(&outer)), Some(&inner));
        // 只有一层时原样返回
        assert_eq!(unwrap_open_data(Some(&inner)), Some(&inner));
        assert_eq!(unwrap_open_data(None), None);
    }

    #[test]
    fn wechat_bind_ticket_key_payload_and_extraction() {
        assert_eq!(wechat_bind_key("t1"), "wechat-bind:t1");
        assert_eq!(WECHAT_BIND_TTL_SECS, 600);

        // 载荷 serde 往返（camelCase；缺字段走 Default）
        let data = WechatBindData {
            openid: "oABC".into(),
            unionid: Some("uXYZ".into()),
            nickname: "昵称".into(),
            headimgurl: "https://img/1.png".into(),
            authorize_type: 1,
            app_id: "wxshop123".into(),
        };
        let raw = serde_json::to_string(&data).unwrap();
        assert!(raw.contains("\"authorizeType\":1"), "{raw}");
        let back: WechatBindData = serde_json::from_str(&raw).unwrap();
        assert_eq!(back, data);
        let minimal: WechatBindData = serde_json::from_str(r#"{"openid":"o1"}"#).unwrap();
        assert_eq!(minimal.openid, "o1");
        assert_eq!(minimal.unionid, None);
        assert_eq!(minimal.authorize_type, 0);
        // 旧凭证无 appId 字段 → 作用域键默认空串（平台口径）
        assert_eq!(minimal.app_id, "");

        // bindTicket 提取：正常 / 空白 / 缺失 / 非字符串
        let mut map = Map::new();
        map.insert("bindTicket".into(), Value::String(" tk ".into()));
        assert_eq!(extract_bind_ticket(&map), Some("tk"));
        map.insert("bindTicket".into(), Value::String("  ".into()));
        assert_eq!(extract_bind_ticket(&map), None);
        map.insert("bindTicket".into(), Value::Bool(true));
        assert_eq!(extract_bind_ticket(&map), None);
        map.remove("bindTicket");
        assert_eq!(extract_bind_ticket(&map), None);
    }

    #[test]
    fn nicknames_and_authorize_type_match_java() {
        assert_eq!(auto_nickname(Some("tig_"), "abc"), "tig_abc");
        assert_eq!(auto_nickname(Some("  "), "abc"), "user_abc");
        assert_eq!(auto_nickname(None, "abc"), "user_abc");
        assert_eq!(authorize_type_from_client(Some("wechat")), 1);
        assert_eq!(authorize_type_from_client(Some("miniProgram")), 2);
        assert_eq!(authorize_type_from_client(None), 2);
    }

    #[test]
    fn dto_and_vo_serde_shapes() {
        let dto: BindMobileDTO = serde_json::from_str(
            r#"{"mobile":"138","mobileCode":"123456","password":"p",
                "openData":{"openData":{"openid":"o1"}},"referrerUserId":7}"#,
        )
        .unwrap();
        assert_eq!(dto.mobile_code.as_deref(), Some("123456"));
        assert_eq!(dto.referrer_user_id, Some(7));
        let data = unwrap_open_data(dto.open_data.as_ref()).unwrap();
        assert_eq!(data.get("openid").and_then(Value::as_str), Some("o1"));

        let vo = QuickLoginSettingVO {
            wechat_login: 1,
            show_oauth: 1,
            wechat_h5_mobile_login: 0,
            wechat_h5_password_login: 1,
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["wechatLogin"], 1);
        assert_eq!(v["showOauth"], 1);
        assert_eq!(v["wechatH5MobileLogin"], 0);
        assert_eq!(v["wechatH5PasswordLogin"], 1);

        let wx = WxLoginInfoByCodeVO {
            r#type: 2,
            token: None,
            open_data: None,
        };
        let v = serde_json::to_value(&wx).unwrap();
        assert_eq!(v["type"], 2);
        assert!(v.get("token").is_some());
    }
}
