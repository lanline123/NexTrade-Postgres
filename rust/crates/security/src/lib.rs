//! 安全模块：JWT 令牌与密码哈希。
//!
//! 对应原 Java：
//! - `java-jwt` / `jjwt` → `jwt`
//! - `jbcrypt` (BCryptPasswordEncoder) → `bcrypt` / `argon2`

pub mod audit;
pub mod jwt;
pub mod password;
pub mod token_revocation;
pub mod web;
pub mod ws;
pub mod ws_registry;

pub use jwt::{Claims, JwtService};
pub use password::{hash_password, verify_password};
pub use web::{ok, ApiResult, AppError, AuthUser};