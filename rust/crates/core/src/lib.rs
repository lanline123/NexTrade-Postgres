//! NexTrade 核心领域层
//!
//! 对应原 Java 项目中的 `nextrade-bean` + `nextrade-common` 模块：
//! - [`error`]：全局错误类型（替代原 Java 的 ResultWrapper / BizException）
//! - [`types`]：通用类型（分页、响应包装、状态枚举）
//! - [`entities`]：数据库实体（替代 MyBatis Plus 的 DO/Entity）
//! - [`models`]：业务模型与 DTO（替代原 Java 的 DTO/BO）

pub mod entities;
pub mod error;
pub mod models;
pub mod types;

pub use error::{ApiResponse, Error};

/// 全局 Result 别名，等价于原 Java 侧的 `Result<T, BizException>`。
pub type Result<T> = std::result::Result<T, Error>;