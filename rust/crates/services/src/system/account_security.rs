//! 账号安全：登录失败计数与临时锁定（论坛 B 类第 6 项「账号安全锁定」，
//! 用户 2026-11-21 拍板要做；**我方新增**，原版 Java 无此机制，其对应
//! 批评帖 747915 的 6 项中本批补齐 ① 失败计数带 TTL、⑤ 临时锁定；
//! ② 密码强度与 ⑥ 重置禁复用在 `nextrade_security::password` 同批落地）。
//!
//! 存储：`config` 表单行 `biz_code = "accountSecurity"`，`biz_val` 为整块
//! JSON（与 `buyStyle` / `priceDisplay` 同属「单 code 整块 JSON」型配置）。
//! 未配置时返回内置默认：**默认开启**、5 次 / 15 分钟——锁定是安全基线，
//! 且锁定到期自动解除、验证码登录不受影响，不存在把用户永久关在门外的
//! 路径，故默认开；管理员可在后台「设置 → 商城设置 → 账号安全」调整
//! 阈值 / 时长或关闭。
//!
//! 计数与锁定状态存缓存（现网 Redis，`CacheService` 抽象，自带 TTL）：
//! - `login-fail:{scope}:{account_id}`：失败计数，窗口 = 锁定时长
//!   （窗口内累计失败达阈值即锁定；窗口与时长同 knob，避免多一个
//!   互相打架的设置项，设置页文案按此口径说明）；
//! - `login-lock:{scope}:{account_id}`：锁定标记，值为锁定到期
//!   Unix 秒（剩余时长由值算出，不依赖缓存 TTL 查询），TTL = 锁定时长。
//!
//! 作用域 `scope`：`member`（会员）/ `admin`（后台账号，含平台 / 商家 /
//! 供应商——三者共用 `AuthService::login` 同一认证链，一并覆盖）。
//! 只按**账号维度**计数，不做 IP 封禁（共享出口误伤）；**密码登录**才
//! 计数与拦截，短信验证码 / 微信登录不计入、也不受锁定拦截（验证码
//! 本身是一次性凭据且绑定本人手机，锁定防的是密码爆破）。

use serde_json::{json, Value};

use nextrade_core::{Error, Result};
use nextrade_infrastructure::cache::CacheService;
use nextrade_infrastructure::db::Repositories;

/// config 表中的业务编码。
pub const ACCOUNT_SECURITY_CODE: &str = "accountSecurity";

/// 会员作用域。
pub const SCOPE_MEMBER: &str = "member";
/// 后台账号作用域（平台 / 商家 / 供应商共用后台登录链）。
pub const SCOPE_ADMIN: &str = "admin";

/// 默认失败阈值（深评建议值）。
pub const DEFAULT_MAX_FAILURES: u32 = 5;
/// 默认锁定时长（分钟，深评建议值）。
pub const DEFAULT_LOCK_MINUTES: u32 = 15;
/// 阈值清洗范围。
pub const MAX_FAILURES_MIN: u32 = 1;
pub const MAX_FAILURES_MAX: u32 = 20;
/// 锁定时长清洗范围（分钟）。
pub const LOCK_MINUTES_MIN: u32 = 1;
pub const LOCK_MINUTES_MAX: u32 = 1440;

/// 生效中的账号安全设置（清洗后口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountSecuritySettings {
    /// 总开关：关 = 不计数、不锁定（登录链行为与本批之前一致）。
    pub enabled: bool,
    /// 窗口内允许的最大失败次数，达到即锁定。
    pub max_failures: u32,
    /// 锁定时长（分钟），同时也是失败计数窗口。
    pub lock_minutes: u32,
}

impl Default for AccountSecuritySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_failures: DEFAULT_MAX_FAILURES,
            lock_minutes: DEFAULT_LOCK_MINUTES,
        }
    }
}

impl AccountSecuritySettings {
    /// 计数窗口 / 锁定时长（秒）。
    #[must_use]
    pub fn lock_secs(&self) -> u64 {
        u64::from(self.lock_minutes) * 60
    }
}

/// 内置默认配置（未保存过设置时生效）。
#[must_use]
pub fn default_account_security() -> Value {
    let d = AccountSecuritySettings::default();
    json!({
        "enabled": d.enabled,
        "maxFailures": d.max_failures,
        "lockMinutes": d.lock_minutes,
    })
}

fn clean_u32(input: Option<&Value>, fallback: u32, min: u32, max: u32) -> u32 {
    input
        .and_then(Value::as_f64)
        .filter(|v| v.fract() == 0.0 && *v >= f64::from(min) && *v <= f64::from(max))
        .map_or(fallback, |v| v as u32)
}

/// 校验/清洗一份账号安全配置（保存与读取共用）。越界 / 类型不对回落
/// 默认；输入不是对象时返回默认配置。
#[must_use]
pub fn sanitize_account_security(input: &Value) -> Value {
    let Some(obj) = input.as_object() else {
        return default_account_security();
    };
    let enabled = obj
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    json!({
        "enabled": enabled,
        "maxFailures": clean_u32(
            obj.get("maxFailures"),
            DEFAULT_MAX_FAILURES,
            MAX_FAILURES_MIN,
            MAX_FAILURES_MAX,
        ),
        "lockMinutes": clean_u32(
            obj.get("lockMinutes"),
            DEFAULT_LOCK_MINUTES,
            LOCK_MINUTES_MIN,
            LOCK_MINUTES_MAX,
        ),
    })
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认配置。
#[must_use]
pub fn effective_account_security(stored: Option<&str>) -> Value {
    let Some(raw) = stored else {
        return default_account_security();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_account_security(&v),
        _ => default_account_security(),
    }
}

/// 把生效配置 JSON 转成结构体口径（经清洗，字段恒合法）。
#[must_use]
pub fn settings_from_value(value: &Value) -> AccountSecuritySettings {
    let clean = sanitize_account_security(value);
    AccountSecuritySettings {
        enabled: clean["enabled"].as_bool().unwrap_or(true),
        max_failures: clean["maxFailures"].as_u64().unwrap_or(u64::from(DEFAULT_MAX_FAILURES)) as u32,
        lock_minutes: clean["lockMinutes"].as_u64().unwrap_or(u64::from(DEFAULT_LOCK_MINUTES)) as u32,
    }
}

/// 读取生效设置（登录链调用）：配置行缺失 ⇒ 默认；读库失败 ⇒ 默认并
/// 告警（安全基线不因配置读失败而整体失效，口径见模块注释）。
pub async fn load_settings(repos: &Repositories) -> AccountSecuritySettings {
    match repos
        .system
        .config_basic
        .find_by_code(ACCOUNT_SECURITY_CODE)
        .await
    {
        Ok(stored) => settings_from_value(&effective_account_security(stored.as_deref())),
        Err(e) => {
            tracing::warn!("读取账号安全设置失败，按默认口径执行: {e}");
            AccountSecuritySettings::default()
        }
    }
}

/// 失败计数键。
#[must_use]
pub fn fail_key(scope: &str, account_id: i32) -> String {
    format!("login-fail:{scope}:{account_id}")
}

/// 锁定标记键。
#[must_use]
pub fn lock_key(scope: &str, account_id: i32) -> String {
    format!("login-lock:{scope}:{account_id}")
}

/// 锁定状态快照（管理端查询用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockStatus {
    /// 是否锁定中。
    pub locked: bool,
    /// 锁定剩余秒数（未锁定为 0）。
    pub remaining_seconds: u64,
    /// 当前窗口内已累计失败次数。
    pub fail_count: i64,
}

/// 一次失败记录的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureOutcome {
    /// 功能关闭，未计数。
    Disabled,
    /// 已计数，未达阈值（`count` 为当前累计）。
    Counted { count: i64 },
    /// 本次失败触发锁定（值为锁定到期 Unix 秒）。
    Locked { locked_until: u64 },
}

fn now_secs() -> u64 {
    u64::try_from(nextrade_common::utils::now_seconds()).unwrap_or(0)
}

/// 查询锁定剩余秒数：锁定中 → `Some(剩余秒)`；未锁定 / 已过期 → `None`
/// （过期标记顺手清理）。值损坏按未锁定处理（不因脏数据锁人）。
///
/// # Errors
/// 缓存读取失败 → `Error::Cache`。
pub async fn check_lock(
    cache: &CacheService,
    scope: &str,
    account_id: i32,
) -> Result<Option<u64>> {
    let key = lock_key(scope, account_id);
    let Some(raw) = cache.get_str(&key).await? else {
        return Ok(None);
    };
    let now = now_secs();
    match raw.trim().parse::<u64>() {
        Ok(until) if until > now => Ok(Some(until - now)),
        _ => {
            let _ = cache.del(&key).await;
            Ok(None)
        }
    }
}

/// 记录一次密码登录失败：窗口内计数 +1，达阈值则写锁定标记并清计数。
///
/// # Errors
/// 缓存读写失败 → `Error::Cache`。
pub async fn record_failure(
    cache: &CacheService,
    scope: &str,
    account_id: i32,
    settings: &AccountSecuritySettings,
) -> Result<FailureOutcome> {
    if !settings.enabled {
        return Ok(FailureOutcome::Disabled);
    }
    let window = settings.lock_secs().max(60);
    let count = cache.incr(&fail_key(scope, account_id), window).await?;
    if count >= i64::from(settings.max_failures) {
        let locked_until = now_secs() + settings.lock_secs();
        cache
            .set_str(
                &lock_key(scope, account_id),
                &locked_until.to_string(),
                settings.lock_secs(),
            )
            .await?;
        let _ = cache.del(&fail_key(scope, account_id)).await;
        return Ok(FailureOutcome::Locked { locked_until });
    }
    Ok(FailureOutcome::Counted { count })
}

/// 登录成功 / 管理员手动解锁：清除失败计数与锁定标记。
///
/// # Errors
/// 缓存删除失败 → `Error::Cache`。
pub async fn clear_account_state(
    cache: &CacheService,
    scope: &str,
    account_id: i32,
) -> Result<()> {
    cache.del(&fail_key(scope, account_id)).await?;
    cache.del(&lock_key(scope, account_id)).await?;
    Ok(())
}

/// 读取锁定状态快照（管理端会员详情用）。
///
/// # Errors
/// 缓存读取失败 → `Error::Cache`。
pub async fn lock_status(
    cache: &CacheService,
    scope: &str,
    account_id: i32,
) -> Result<LockStatus> {
    let remaining = check_lock(cache, scope, account_id).await?;
    let fail_count = cache
        .get_str(&fail_key(scope, account_id))
        .await?
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0);
    Ok(LockStatus {
        locked: remaining.is_some(),
        remaining_seconds: remaining.unwrap_or(0),
        fail_count,
    })
}

/// 锁定提示错误（剩余时长向上取整到分钟，至少 1 分钟）。
#[must_use]
pub fn locked_error(remaining_seconds: u64) -> Error {
    let minutes = remaining_seconds.div_ceil(60).max(1);
    Error::biz(403, format!("账号已被锁定，请 {minutes} 分钟后重试"))
}

// ————————————————— 登录链容错封装 —————————————————
// 登录链对缓存故障一律「放行 + 告警」（fail-open）：密码校验本身仍在，
// 缓存不可用不应演变为全站登录不可用。管理端查询/解锁走严格版本，
// 故障要显性报错，不在此列。

/// 登录前锁定检查（容错版）：锁定中 → `Some(剩余秒)`；未锁定 / 功能
/// 关闭 / 缓存故障 → `None`。
pub async fn guard_locked(
    cache: &CacheService,
    scope: &str,
    account_id: i32,
    settings: &AccountSecuritySettings,
) -> Option<u64> {
    if !settings.enabled {
        return None;
    }
    match check_lock(cache, scope, account_id).await {
        Ok(remaining) => remaining,
        Err(e) => {
            tracing::warn!("登录锁定检查失败（放行）scope={scope} id={account_id}: {e}");
            None
        }
    }
}

/// 登录失败记账（容错版）：本次触发锁定 → `Some(锁定剩余秒)`；其余
/// 情况（未达阈值 / 功能关闭 / 缓存故障）→ `None`。
pub async fn note_failure(
    cache: &CacheService,
    scope: &str,
    account_id: i32,
    settings: &AccountSecuritySettings,
) -> Option<u64> {
    match record_failure(cache, scope, account_id, settings).await {
        Ok(FailureOutcome::Locked { locked_until }) => {
            Some(locked_until.saturating_sub(now_secs()))
        }
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("登录失败记账失败（忽略）scope={scope} id={account_id}: {e}");
            None
        }
    }
}

/// 登录成功清零（容错版）：失败仅告警，不影响已成功的登录。
pub async fn note_success(cache: &CacheService, scope: &str, account_id: i32) {
    if let Err(e) = clear_account_state(cache, scope, account_id).await {
        tracing::warn!("登录成功清零失败（忽略）scope={scope} id={account_id}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_infrastructure::cache::get_cache;

    #[test]
    fn defaults_are_enabled_five_fifteen() {
        let d = default_account_security();
        assert_eq!(d["enabled"], json!(true));
        assert_eq!(d["maxFailures"], json!(5));
        assert_eq!(d["lockMinutes"], json!(15));
        assert_eq!(AccountSecuritySettings::default().lock_secs(), 900);
    }

    #[test]
    fn effective_falls_back_to_defaults() {
        assert_eq!(effective_account_security(None), default_account_security());
        assert_eq!(effective_account_security(Some("bad")), default_account_security());
        assert_eq!(effective_account_security(Some("[1]")), default_account_security());
    }

    #[test]
    fn sanitize_clamps_and_defaults() {
        // 越界回落默认
        let v = sanitize_account_security(&json!({ "maxFailures": 0, "lockMinutes": 99999 }));
        assert_eq!(v["maxFailures"], json!(5));
        assert_eq!(v["lockMinutes"], json!(15));
        // 非整数 / 非布尔回落默认
        let v = sanitize_account_security(&json!({ "enabled": "yes", "maxFailures": 2.5 }));
        assert_eq!(v["enabled"], json!(true));
        assert_eq!(v["maxFailures"], json!(5));
        // 合法值原样保留（含关闭开关、边界值）
        let v = sanitize_account_security(&json!({ "enabled": false, "maxFailures": 20, "lockMinutes": 1 }));
        assert_eq!(v["enabled"], json!(false));
        assert_eq!(v["maxFailures"], json!(20));
        assert_eq!(v["lockMinutes"], json!(1));
        // 结构体口径与 JSON 清洗一致
        let s = settings_from_value(&json!({ "enabled": false, "maxFailures": 3, "lockMinutes": 30 }));
        assert_eq!(
            s,
            AccountSecuritySettings { enabled: false, max_failures: 3, lock_minutes: 30 }
        );
    }

    #[test]
    fn keys_are_scoped() {
        assert_eq!(fail_key(SCOPE_MEMBER, 42), "login-fail:member:42");
        assert_eq!(lock_key(SCOPE_ADMIN, 7), "login-lock:admin:7");
    }

    #[test]
    fn locked_error_rounds_up_minutes() {
        assert_eq!(locked_error(1).business_code(), 403);
        assert_eq!(locked_error(60).to_string(), "业务错误 [403]: 账号已被锁定，请 1 分钟后重试");
        assert_eq!(locked_error(61).to_string(), "业务错误 [403]: 账号已被锁定，请 2 分钟后重试");
        assert_eq!(locked_error(900).to_string(), "业务错误 [403]: 账号已被锁定，请 15 分钟后重试");
    }

    fn test_settings() -> AccountSecuritySettings {
        AccountSecuritySettings { enabled: true, max_failures: 3, lock_minutes: 15 }
    }

    #[tokio::test]
    async fn failures_below_threshold_do_not_lock() {
        let cache = get_cache();
        let id = 9_200_001;
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
        let s = test_settings();
        for expected in 1..3 {
            match record_failure(cache, SCOPE_MEMBER, id, &s).await.unwrap() {
                FailureOutcome::Counted { count } => assert_eq!(count, expected),
                other => panic!("未达阈值不应锁定: {other:?}"),
            }
        }
        assert_eq!(check_lock(cache, SCOPE_MEMBER, id).await.unwrap(), None);
        let status = lock_status(cache, SCOPE_MEMBER, id).await.unwrap();
        assert!(!status.locked);
        assert_eq!(status.fail_count, 2);
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
    }

    #[tokio::test]
    async fn threshold_failure_locks_with_remaining() {
        let cache = get_cache();
        let id = 9_200_002;
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
        let s = test_settings();
        record_failure(cache, SCOPE_MEMBER, id, &s).await.unwrap();
        record_failure(cache, SCOPE_MEMBER, id, &s).await.unwrap();
        match record_failure(cache, SCOPE_MEMBER, id, &s).await.unwrap() {
            FailureOutcome::Locked { .. } => {}
            other => panic!("第 3 次失败应触发锁定: {other:?}"),
        }
        let remaining = check_lock(cache, SCOPE_MEMBER, id).await.unwrap();
        assert!(remaining.is_some_and(|r| (1..=900).contains(&r)), "剩余应为 1..=900 秒: {remaining:?}");
        // 锁定后计数已清零（状态快照 fail_count=0、locked=true）
        let status = lock_status(cache, SCOPE_MEMBER, id).await.unwrap();
        assert!(status.locked);
        assert_eq!(status.fail_count, 0);
        // 管理员解锁后立即恢复
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
        assert_eq!(check_lock(cache, SCOPE_MEMBER, id).await.unwrap(), None);
    }

    #[tokio::test]
    async fn success_clears_failure_count() {
        let cache = get_cache();
        let id = 9_200_003;
        clear_account_state(cache, SCOPE_ADMIN, id).await.unwrap();
        let s = test_settings();
        record_failure(cache, SCOPE_ADMIN, id, &s).await.unwrap();
        record_failure(cache, SCOPE_ADMIN, id, &s).await.unwrap();
        note_success(cache, SCOPE_ADMIN, id).await;
        let status = lock_status(cache, SCOPE_ADMIN, id).await.unwrap();
        assert_eq!(status.fail_count, 0);
        assert!(!status.locked);
    }

    #[tokio::test]
    async fn disabled_settings_never_count_or_lock() {
        let cache = get_cache();
        let id = 9_200_004;
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
        let s = AccountSecuritySettings { enabled: false, ..test_settings() };
        for _ in 0..6 {
            assert_eq!(
                record_failure(cache, SCOPE_MEMBER, id, &s).await.unwrap(),
                FailureOutcome::Disabled
            );
        }
        assert_eq!(guard_locked(cache, SCOPE_MEMBER, id, &s).await, None);
        let status = lock_status(cache, SCOPE_MEMBER, id).await.unwrap();
        assert_eq!(status.fail_count, 0);
        assert!(!status.locked);
    }

    #[tokio::test]
    async fn note_failure_reports_lock_trigger() {
        let cache = get_cache();
        let id = 9_200_005;
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
        let s = test_settings();
        assert_eq!(note_failure(cache, SCOPE_MEMBER, id, &s).await, None);
        assert_eq!(note_failure(cache, SCOPE_MEMBER, id, &s).await, None);
        let remaining = note_failure(cache, SCOPE_MEMBER, id, &s).await;
        assert!(remaining.is_some_and(|r| (1..=900).contains(&r)));
        // 已锁定状态下 guard 返回剩余时长
        let guarded = guard_locked(cache, SCOPE_MEMBER, id, &s).await;
        assert!(guarded.is_some_and(|r| (1..=900).contains(&r)));
        clear_account_state(cache, SCOPE_MEMBER, id).await.unwrap();
    }
}
