//! 用例服务层（按限界上下文分组）。

pub mod identity;

/// 行为验证码（AJ-Captcha）实现，位于 system 上下文。
pub use system::captcha;
pub mod merchant;
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
pub mod vendor;
pub mod pos;
pub mod o2o;
