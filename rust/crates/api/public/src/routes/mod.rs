//! 商城端路由，按限界上下文分组。

pub mod merchant;
pub mod o2o_client;
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

use axum::Router;

use super::PublicState;

/// 公开路由（域模块自带鉴权或匿名，外层不得再套 require_auth）。
pub fn merge_public(state: &PublicState) -> Router<PublicState> {
    let _ = state;
    Router::new()
        .merge(merchant::routes(state))
        .merge(o2o_client::routes(state))
        .merge(merchant::public_routes())
        .merge(merchant::anonymous_routes(state))
        .merge(wholesale::routes(state))
        .merge(wholesale::public_routes())
        .merge(wholesale::anonymous_routes(state))
        .merge(catalog::routes(state))
        .merge(catalog::public_routes())
        .merge(catalog::anonymous_routes(state))
        .merge(trade::routes(state))
        .merge(trade::public_routes())
        .merge(trade::anonymous_routes(state))
        .merge(payment::routes(state))
        .merge(payment::public_routes())
        .merge(payment::anonymous_routes(state))
        .merge(marketing::routes(state))
        .merge(marketing::public_routes())
        .merge(marketing::anonymous_routes(state))
        .merge(member::routes(state))
        .merge(member::public_routes())
        .merge(member::anonymous_routes(state))
        .merge(logistics::routes(state))
        .merge(logistics::public_routes())
        .merge(logistics::anonymous_routes(state))
        .merge(content::routes(state))
        .merge(content::public_routes())
        .merge(content::anonymous_routes(state))
        .merge(system::routes(state))
        .merge(system::public_routes())
        .merge(system::anonymous_routes(state))
        .merge(im::routes(state))
        .merge(im::public_routes())
        .merge(im::anonymous_routes(state))
}

/// 需登录路由（外层已套 require_auth）。
pub fn merge_authorized(_state: &PublicState) -> Router<PublicState> {
    Router::new()
        .merge(merchant::authorized_routes())
        .merge(wholesale::authorized_routes())
        .merge(catalog::authorized_routes())
        .merge(trade::authorized_routes())
        .merge(payment::authorized_routes())
        .merge(marketing::authorized_routes())
        .merge(member::authorized_routes())
        .merge(logistics::authorized_routes())
        .merge(content::authorized_routes())
        .merge(system::authorized_routes())
        .merge(im::authorized_routes())
}

/// 汇总所有域已实现路由，供覆盖率统计使用。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    let mut out = Vec::new();
    out.extend(merchant::implemented_routes());
    out.extend(o2o_client::implemented_routes());
    out.extend(wholesale::implemented_routes());
    out.extend(catalog::implemented_routes());
    out.extend(trade::implemented_routes());
    out.extend(payment::implemented_routes());
    out.extend(marketing::implemented_routes());
    out.extend(member::implemented_routes());
    out.extend(logistics::implemented_routes());
    out.extend(content::implemented_routes());
    out.extend(system::implemented_routes());
    out.extend(im::implemented_routes());
    out
}
