//! im 领域仓储。

pub mod admin_shop_kefu_config;
pub mod admin_user_kefu;
pub mod im_config;
pub mod im_conversation;
pub mod im_message;
pub mod im_servant;
pub mod web_socket;

use crate::db::DbPool;

#[derive(Clone)]
pub struct Repositories {
    pub im_config: im_config::ImConfigRepository,
    pub im_message: im_message::ImMessageRepository,
    pub im_servant: im_servant::ImServantRepository,
    pub web_socket: web_socket::WebSocketRepository,
    pub im_conversation: im_conversation::ImConversationRepository,
    pub admin_user_kefu: admin_user_kefu::AdminUserKefuRepository,
    pub admin_shop_kefu_config: admin_shop_kefu_config::AdminShopKefuConfigRepository,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            im_config: im_config::ImConfigRepository::new(pool.clone()),
            im_message: im_message::ImMessageRepository::new(pool.clone()),
            im_servant: im_servant::ImServantRepository::new(pool.clone()),
            web_socket: web_socket::WebSocketRepository::new(pool.clone()),
            im_conversation: im_conversation::ImConversationRepository::new(pool.clone()),
            admin_user_kefu: admin_user_kefu::AdminUserKefuRepository::new(pool.clone()),
            admin_shop_kefu_config: admin_shop_kefu_config::AdminShopKefuConfigRepository::new(pool.clone()),
        }
    }
}
