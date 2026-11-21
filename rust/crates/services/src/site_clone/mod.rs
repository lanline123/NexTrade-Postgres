//! 站点搬家服务组（site_clone 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/site-clone-design/REPORT.md`，
//! 用户 2026-11-21 立项并定名「站点搬家」；P1 = 快速开店 MVP）。
//! 与万象互联（hub）零代码交叉：只借其 AK/SK 哈希鉴权与调用审计的模式，
//! 不共用代码路径；联邦总开关与本功能互不影响。

pub mod clone;
pub mod decorate_importer;
pub mod importer;
pub mod whitelist;

use std::sync::Arc;

/// 站点搬家服务聚合（挂载于 `Services.site_clone` / 两端 State）。
#[derive(Clone)]
pub struct Services {
    pub clone: Arc<clone::SiteCloneService>,
}
