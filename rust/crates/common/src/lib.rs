//! 通用基础模块：配置、日志、工具函数、中间件。
//!
//! 对应原 Java `nextrade-common` 模块。

pub mod config;
pub mod log;
pub mod utils;

pub use config::AppConfig;