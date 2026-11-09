//! 日志初始化，替代原 Java `logback.xml`。
//!
//! - 支持 `RUST_LOG` 环境变量控制级别（如 `nextrade=debug,info`）
//! - 控制台 + 滚动文件双输出，按天切割，保留 30 天

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化全局日志。可重复调用（仅第一次生效）。
pub fn init(log_dir: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,nextrade=info,sqlx=warn"));

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("nextrade")
        .filename_suffix("log")
        .max_log_files(30)
        .build(log_dir)
        .unwrap_or_else(|_| RollingFileAppender::builder().build(log_dir).expect("日志目录不可写"));

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(file_appender)
        .with_target(true)
        .with_level(true);

    let console_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .compact();

    // 忽略重复初始化错误
    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .try_init();

    tracing::info!("日志系统初始化完成，输出目录: {log_dir}");
}