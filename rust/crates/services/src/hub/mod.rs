//! 平台联邦服务组（hub 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/hub-federation-design/proposal.md`，
//! 用户 2026-11-20 拍板；P1 范围见设计 §12）。

pub mod federation;

use std::sync::Arc;

/// 联邦服务聚合（挂载于 `Services.hub`）。
#[derive(Clone)]
pub struct Services {
    pub federation: Arc<federation::FederationService>,
}
