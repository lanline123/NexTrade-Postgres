//! 仓储层：替代原 Java MyBatis Plus Mapper。
//!
//! 按限界上下文分组（`repo/<ctx>/<feature>.rs`）。

pub mod identity;
pub mod merchant;
pub mod vendor;
pub mod pos;
pub mod o2o;
pub mod wholesale;
pub mod catalog;
pub mod trade;
pub mod payment;
pub mod marketing;
pub mod member;
pub mod logistics;
pub mod content;
pub mod system;
pub mod im;
pub mod analytics;
pub mod openapi;
pub mod lang;
