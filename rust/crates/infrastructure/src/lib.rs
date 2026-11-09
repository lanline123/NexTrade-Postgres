//! 基础设施层：数据库连接池、Repository、Redis 缓存、消息队列、对象存储。
//!
//! 对应原 Java `nextrade-service` 中的 Mapper 层与 Spring 数据配置。

pub mod cache;
pub mod db;
pub mod queue;
pub mod storage;

pub use cache::CacheService;
pub use db::{DbPool, Repositories};