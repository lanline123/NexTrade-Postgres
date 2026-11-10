//! 应用配置。
//!
//! 对应原 Java `application.yaml` / `application-{profile}.yaml`，
//! 支持 `config.yaml` + 环境变量覆盖（`TIG_` 前缀），替代 Spring 的 profile 机制。

use anyhow::{Context, Result};
use serde::Deserialize;

/// 全局应用配置。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default, rename = "datasource")]
    pub datasource: DatasourceConfig,
    #[serde(default)]
    pub redis: RedisConfig,
    #[serde(default)]
    pub jwt: JwtConfig,
    #[serde(default)]
    pub rabbitmq: RabbitMqConfig,
    #[serde(default)]
    pub elasticsearch: ElasticsearchConfig,
    #[serde(default)]
    pub file: FileConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub internal: InternalConfig,
}

/// 内部服务间调用配置（Java `internal.api.key`，`InternalApiFilter`）。
///
/// 环境变量覆盖：`TIG_INTERNAL__API_KEY`。密钥为空时受保护接口
/// （`/api/socket/send`）按 Java 语义一律 403。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct InternalConfig {
    #[serde(default)]
    pub api_key: String,
}

/// 服务配置。
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 管理端 API 端口（原 adminapi）
    #[serde(default = "default_admin_port")]
    pub admin_port: u16,
    /// 商城端 API 端口（原 api）
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    /// 超时与并发
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_admin_port() -> u16 {
    8080
}
fn default_api_port() -> u16 {
    8181
}
fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_workers() -> usize {
    num_cpus_default()
}

fn num_cpus_default() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8)
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            admin_port: default_admin_port(),
            api_port: default_api_port(),
            host: default_host(),
            workers: default_workers(),
        }
    }
}

/// 数据源（PostgreSQL），对应 Spring `spring.datasource`。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DatasourceConfig {
    /// `postgres://user:pass@host:5432/dbname`
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    /// 连接池最大连接数（原 hikari maximum-pool-size）
    #[serde(default = "default_pool_max")]
    pub max_connections: u32,
    /// 是否执行数据库迁移（启动时自动 migrate）
    #[serde(default)]
    pub auto_migrate: bool,
}

fn default_pool_max() -> u32 {
    20
}

/// Redis 配置，对应 Spring `spring.data.redis`。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RedisConfig {
    #[serde(default = "default_redis_host")]
    pub host: String,
    #[serde(default = "default_redis_port")]
    pub port: u16,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub database: i64,
}

fn default_redis_host() -> String {
    "127.0.0.1".into()
}
fn default_redis_port() -> u16 {
    6379
}

impl RedisConfig {
    /// 生成 redis:// 连接串。
    pub fn url(&self) -> String {
        match &self.password {
            Some(p) if !p.is_empty() => {
                format!("redis://:{}@{}:{}/{}", p, self.host, self.port, self.database)
            }
            _ => format!("redis://{}:{}/{}", self.host, self.port, self.database),
        }
    }
}

/// JWT 配置（原 Java java-jwt/jjwt 的 secret 与过期时间）。
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    /// 签名密钥
    #[serde(default = "default_jwt_secret")]
    pub secret: String,
    /// 访问令牌过期秒数（默认 2 小时）
    #[serde(default = "default_access_exp")]
    pub access_expire_secs: i64,
    /// 刷新令牌过期秒数（默认 7 天）
    #[serde(default = "default_refresh_exp")]
    pub refresh_expire_secs: i64,
    /// 签发者
    #[serde(default = "default_issuer")]
    pub issuer: String,
}

/// 内置默认 JWT 密钥。**源码里公开可见**，只允许用于本地开发。
///
/// 生产若没覆盖它，等于签名密钥公开——任何人都能自签 token 冒充任意用户。
/// release 构建下由 [`assert_secret_is_overridden`] 拒绝启动。
pub const DEFAULT_JWT_SECRET: &str = "nextrade-rust-dev-secret-change-me-in-production";

fn default_jwt_secret() -> String {
    DEFAULT_JWT_SECRET.into()
}
fn default_access_exp() -> i64 {
    7200
}
fn default_refresh_exp() -> i64 {
    604_800
}
fn default_issuer() -> String {
    "nextrade".into()
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: default_jwt_secret(),
            access_expire_secs: default_access_exp(),
            refresh_expire_secs: default_refresh_exp(),
            issuer: default_issuer(),
        }
    }
}

impl JwtConfig {
    /// 密钥是否仍为空、或仍等于内置默认值（即「没有真正配置过」）。
    #[must_use]
    pub fn uses_builtin_secret(&self) -> bool {
        self.secret.trim().is_empty() || self.secret == DEFAULT_JWT_SECRET
    }
}

/// 启动守卫：release 构建拒绝用内置默认 JWT 密钥启动。
///
/// 分级行为：
///
/// * 已覆盖（`TIG_JWT__SECRET` 或 `config.yaml` 的 `jwt.secret`）→ 放行；
/// * 未覆盖但设了 `TIG_JWT__ALLOW_DEFAULT_SECRET=1` → 放行 + 警告（给本地
///   release 冒烟这类场景，生产不该设）；
/// * 未覆盖且是 debug 构建（`cargo run`）→ 放行 + 警告；
/// * 未覆盖且是 release 构建 → 拒绝。
///
/// # Errors
///
/// 当 `jwt` 仍使用内置默认密钥、且不满足上面任一放行条件时，返回说明原因的错误文本。
pub fn assert_secret_is_overridden(jwt: &JwtConfig) -> Result<(), String> {
    if !jwt.uses_builtin_secret() {
        return Ok(());
    }
    let msg = "JWT 密钥仍是内置默认值（`DEFAULT_JWT_SECRET`，源码公开可见），任何人都能伪造 token。\
               请设置 TIG_JWT__SECRET 环境变量或 config.yaml 的 jwt.secret";
    let escaped = std::env::var("TIG_JWT__ALLOW_DEFAULT_SECRET")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    if escaped {
        tracing::warn!("{msg}；已由 TIG_JWT__ALLOW_DEFAULT_SECRET 放行，切勿用于生产");
        return Ok(());
    }
    if cfg!(debug_assertions) {
        tracing::warn!("{msg}；debug 构建放行（release 构建会拒绝启动）");
        return Ok(());
    }
    Err(msg.to_string())
}

/// RabbitMQ 配置，对应 Spring `spring.rabbitmq`。
#[derive(Debug, Clone, Deserialize)]
pub struct RabbitMqConfig {
    #[serde(default = "default_mq_host")]
    pub host: String,
    #[serde(default = "default_mq_port")]
    pub port: u16,
    #[serde(default = "default_mq_user")]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_vhost")]
    pub virtual_host: String,
}

fn default_mq_host() -> String {
    "127.0.0.1".into()
}
fn default_mq_port() -> u16 {
    5672
}
fn default_mq_user() -> String {
    "admin".into()
}
fn default_vhost() -> String {
    "/".into()
}

impl Default for RabbitMqConfig {
    fn default() -> Self {
        Self {
            host: default_mq_host(),
            port: default_mq_port(),
            username: default_mq_user(),
            password: String::new(),
            virtual_host: default_vhost(),
        }
    }
}

impl RabbitMqConfig {
    pub fn url(&self) -> String {
        format!(
            "amqp://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.virtual_host
        )
    }
}

/// Elasticsearch 配置。
#[derive(Debug, Clone, Deserialize)]
pub struct ElasticsearchConfig {
    #[serde(default = "default_es_url")]
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_es_index")]
    pub product_index: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_retries")]
    pub retries: u32,
}

fn default_es_url() -> String {
    "http://127.0.0.1:9200".into()
}
fn default_es_index() -> String {
    "nextrade_products".into()
}
fn default_timeout() -> u64 {
    30
}
fn default_retries() -> u32 {
    3
}

impl Default for ElasticsearchConfig {
    fn default() -> Self {
        Self {
            url: default_es_url(),
            username: None,
            password: None,
            product_index: default_es_index(),
            timeout_secs: default_timeout(),
            retries: default_retries(),
        }
    }
}

/// 本地文件上传配置（原 `file.upload-dir`）。
#[derive(Debug, Clone, Deserialize)]
pub struct FileConfig {
    #[serde(default = "default_upload_dir")]
    pub upload_dir: String,
}

fn default_upload_dir() -> String {
    "./uploads".into()
}

impl Default for FileConfig {
    fn default() -> Self {
        Self { upload_dir: default_upload_dir() }
    }
}

/// 对象存储配置（OSS/COS/S3 任选其一）。
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StorageConfig {
    /// local | oss | cos | s3 | qiniu
    #[serde(default = "default_storage_type")]
    pub kind: String,
    #[serde(default)]
    pub oss: OssConfig,
    #[serde(default)]
    pub cos: CosConfig,
    #[serde(default)]
    pub s3: S3Config,
}

fn default_storage_type() -> String {
    "local".into()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OssConfig {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub access_key_secret: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CosConfig {
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub secret_id: String,
    #[serde(default)]
    pub secret_key: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct S3Config {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub secret_key: String,
}

impl AppConfig {
    /// 加载配置：`config.yaml`（可选）→ 环境变量覆盖（`TIG_` 前缀）。
    pub fn load() -> Result<Self> {
        // 兼容 .env
        let _ = dotenvy::dotenv();

        let builder = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(
                config::Environment::with_prefix("TIG")
                    .prefix_separator("_")
                    .separator("__"),
            );

        let cfg = builder
            .build()
            .context("加载配置失败（config.yaml / TIG_* 环境变量）")?;
        cfg.try_deserialize()
            .context("配置反序列化失败，请检查 config.yaml 结构")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_url_with_password() {
        let c = RedisConfig {
            host: "h".into(),
            port: 6379,
            password: Some("p".into()),
            database: 2,
        };
        assert_eq!(c.url(), "redis://:p@h:6379/2");
    }

    #[test]
    fn defaults_load() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.server.admin_port, 8080);
        assert_eq!(cfg.server.api_port, 8181);
        assert_eq!(cfg.jwt.access_expire_secs, 7200);
    }

    #[test]
    fn builtin_secret_is_detected() {
        // 默认构造出来的就是内置默认值 → 视为「没配过」
        assert!(JwtConfig::default().uses_builtin_secret());

        // 空串 / 纯空白同样算没配过
        let blank = JwtConfig {
            secret: "   ".into(),
            ..JwtConfig::default()
        };
        assert!(blank.uses_builtin_secret());

        // 任意自定义值都算配过
        let custom = JwtConfig {
            secret: "a".repeat(48),
            ..JwtConfig::default()
        };
        assert!(!custom.uses_builtin_secret());
    }

    #[test]
    fn guard_rejects_builtin_secret_only_in_release() {
        let builtin = JwtConfig::default();
        let res = assert_secret_is_overridden(&builtin);
        if cfg!(debug_assertions) {
            // debug 构建放行，只警告
            assert!(res.is_ok(), "debug 构建应放行内置默认密钥");
        } else {
            let err = res.expect_err("release 构建必须拒绝内置默认密钥");
            assert!(err.contains("JWT"), "错误信息应说明是 JWT 密钥问题: {err}");
        }

        // 自定义密钥在任何构建下都放行
        let custom = JwtConfig {
            secret: "x".repeat(48),
            ..JwtConfig::default()
        };
        assert!(assert_secret_is_overridden(&custom).is_ok());
    }
}