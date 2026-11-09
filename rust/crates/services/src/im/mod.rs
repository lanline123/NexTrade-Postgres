//! im 领域服务。

pub mod admin_shop_kefu_config;
pub mod admin_user_kefu;
pub mod im_config;
pub mod im_conversation;
pub mod im_message;
pub mod im_servant;
pub mod web_socket;

use std::sync::Arc;

/// im 领域服务集合（由组合根装配一次，api 层只持有 `Arc<Services>`）。
#[derive(Clone)]
pub struct Services {
    pub im_config: Arc<im_config::ImConfigService>,
    pub im_message: Arc<im_message::ImMessageService>,
    pub im_servant: Arc<im_servant::ImServantService>,
    pub web_socket: Arc<web_socket::WebSocketService>,
    pub im_conversation: Arc<im_conversation::ImConversationService>,
    pub admin_user_kefu: Arc<admin_user_kefu::AdminUserKefuService>,
    pub admin_shop_kefu_config: Arc<admin_shop_kefu_config::AdminShopKefuConfigService>,
}
