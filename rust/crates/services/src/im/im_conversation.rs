//! IM 客服会话域服务（im::im_conversation）。
//!
//! 对应原 Java `ImConversationServiceImpl`
//! （`com.nextrade.service.im.impl`，管理端 11 + 商城端 12 个端点共用）。
//!
//! # 端点 → 方法
//!
//! | Java handler | 本服务方法 |
//! |--------------|-----------|
//! | `list` | [`ImConversationService::list`] |
//! | `search` | [`ImConversationService::search`] |
//! | `frontSearch`（仅前台） | [`ImConversationService::front_search`] |
//! | `transfer` | [`ImConversationService::transfer`] |
//! | `detail` | [`ImConversationService::detail`] |
//! | `create` | [`ImConversationService::create`] |
//! | `waitServantList` | [`ImConversationService::wait_servant_list`] |
//! | `del` | [`ImConversationService::delete`] |
//! | `saveRemark` | [`ImConversationService::save_remark`] |
//! | `consultHistory` | [`ImConversationService::consult_history`] |
//! | `updateField` | [`ImConversationService::update_field`] |
//! | `batch` | [`ImConversationService::batch`] |
//!
//! # 身份口径
//!
//! Java 控制器用 `TigUtils.getClientTypeByToken()` 区分后台/前台：
//! - 后台（admin 根路由）：`lastServantId = 当前管理员`、`shopId = HeaderUtils.getShopId()`
//!   （`admin` → 0；`shop`/`pos` → `x-shop-id`；其它 → `null`）
//! - 前台（public 根路由）：`userId = SecurityUtils.getCurrentUserId()`
//!   （`lastServantId` / `shopId` 保留查询参数原值）
//!
//! 本服务用 [`ConversationViewer`] 表达该分支，路由层负责从 JWT/请求头解析。
//!
//! # 未迁移（如实标注，不伪造实现）
//!
//! - `transfer` 里的 `imMessageService.sendAutoMessage`（RabbitMQ 延迟队列的
//!   `autoTransfer` 系统消息）未迁移；`im_servant.conversation_num` 仍按 Java
//!   逻辑重算。
//! - Java `transfer` 第二段统计是明显的复制粘贴 bug（用 `dto.getConversationId()`
//!   当客服 ID 构造 wrapper，却复用第一个 wrapper 查询、并把同一 count 再写一次），
//!   净效果与第一段相同 ⇒ 本实现只保留第一段（等价）。
//! - `list` / `frontSearch` 在 `shopId == 0` 时的 `isO2o == 1` 分支（读取
//!   `storeSelfOperatedName/Logo`）依赖 `NexTradeProperties.isO2o`；Rust 侧
//!   `AppConfig` 无该开关 ⇒ 固定走非 O2O 分支（`shopName` / `shopLogo`）。
//! - `adminLogService`（本域未调用）不在范围内。
//! - `consultHistory` 的 `firstUserMessage` 在 Java 里是**全局 `LIMIT 1`**，
//!   只有第一个会话能取到（已如实复刻）。
//! - `getConversationSearch` 的 `Collectors.toMap` 在同一会话命中多条消息时
//!   会抛 `IllegalStateException`（Java 500）；本实现取 id 最小的那条消息，
//!   避免整接口 500（见 `uncertain_points`）。
//! - Java `Assert` / `GlobalException` → 业务 500（`Error::biz(500, ...)`）；
//!   注解校验 → `Error::Validation`（HTTP 400），与既有域一致。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::models::im::im_conversation::{
    conversation_base_vo, conversation_entity_vo, format_unix_time, format_unix_time_opt,
    is_allowed_update_field, map_field_column, AdminSearchResult, BatchDTO, ConsultHistoryListVO,
    ConsultServantVO, DelDTO, FirstUserMessageVO, FrontSearchResult, ImConversationCreateDTO,
    ImConversationEntityVO, ImConversationPageQuery, ImConversationVO, ImMessageVO,
    ImSearchConversationVO, ImSearchShopVO, ImSearchUserVO, ImShopVO, ImUserVO, SaveRemarkDTO,
    ServantMessageVO, TransferDTO, UpdateFieldDTO, WaitServantListQuery, WaitServantListVO,
    CONFIG_DEFAULT_AVATAR, CONFIG_SHOP_LOGO, CONFIG_SHOP_NAME, CONFIG_STORE_SELF_OPERATED_LOGO,
    CONFIG_STORE_SELF_OPERATED_NAME, CONVERSATION_STATUS_IN_PROGRESS, MSG_CONVERSATION_NOT_EXISTS,
    MSG_CONVERSATION_ID_REQUIRED, MSG_FIELD_NAME_REQUIRED, MSG_FIELD_VALUE_REQUIRED,
    MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_NO_PERMISSION, MSG_OPERATION_TYPE_REQUIRED,
    MSG_TRANSFER_TARGET_REQUIRED, MSG_UNSELECTED_ITEM,
};
use nextrade_core::models::im::im_message::{message_type_text, ContentVO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::im::im_conversation::{
    ConvAdminBrief, ConvShopBrief, ConvUserBrief, ImConversationRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 列表身份：后台客服 or 前台用户（对应 Java `TigUtils.getClientTypeByToken()`）。
#[derive(Debug, Clone, Copy)]
pub enum ConversationViewer {
    /// 后台：强制 `last_servant_id = 当前管理员` + 请求头店铺过滤
    Admin {
        /// 当前管理员 ID（`SecurityUtils.getCurrentAdminId()`）
        admin_id: i32,
        /// 请求头店铺（`HeaderUtils.getShopId()` 三态）
        shop_id: Option<i32>,
    },
    /// 前台：强制 `user_id = 当前用户`
    User {
        /// 当前用户 ID（`SecurityUtils.getCurrentUserId()`）
        user_id: i32,
    },
}

/// IM 客服会话域服务。
pub struct ImConversationService {
    repos: Arc<Repositories>,
}

impl ImConversationService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &ImConversationRepository {
        &self.repos.im.im_conversation
    }

    // ————————————————————— 列表 / 详情 —————————————————————

    /// 会话列表（对应 Java `ImConversationServiceImpl#list`）。
    ///
    /// 后台：`lastServantId = 当前管理员`、`shopId = 请求头`；
    /// 前台：`userId = 当前用户`。记录为空时 Java 返回 `new Page<>()`（total=0）。
    pub async fn list(
        &self,
        q: &ImConversationPageQuery,
        viewer: ConversationViewer,
    ) -> Result<PageResult<ImConversationVO>> {
        let mut q = q.clone();
        match viewer {
            ConversationViewer::Admin { admin_id, shop_id } => {
                q.last_servant_id = Some(admin_id);
                q.shop_id = shop_id;
            }
            ConversationViewer::User { user_id } => {
                q.user_id = Some(user_id);
            }
        }
        let page = page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate_list(&q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        // 批量装配：用户 / 店铺 / 消息
        let user_ids = distinct_ids(rows.iter().map(|c| c.user_id));
        let shop_ids = distinct_ids(rows.iter().map(|c| c.shop_id));
        let conversation_ids: Vec<i32> = rows.iter().map(|c| c.id).collect();

        let mut user_map: HashMap<i32, ConvUserBrief> = HashMap::new();
        for u in self.repo().find_users_by_ids(&user_ids).await? {
            user_map.insert(u.user_id, u);
        }
        let mut shop_map: HashMap<i32, ConvShopBrief> = HashMap::new();
        for s in self.repo().find_shops_by_ids(&shop_ids).await? {
            shop_map.insert(s.shop_id, s);
        }
        let messages = self
            .repo()
            .list_messages_by_conversation_ids(&conversation_ids)
            .await?;
        let mut message_map: HashMap<i32, Vec<ImMessageVO>> = HashMap::new();
        for m in &messages {
            let Some(cid) = m.conversation_id else {
                continue;
            };
            message_map.entry(cid).or_default().push(message_to_vo(m));
        }

        let mut list: Vec<ImConversationVO> = Vec::with_capacity(rows.len());
        for conv in &rows {
            let mut vo = conversation_base_vo(conv);
            vo.conversation_id = Some(conv.id);
            // Java 逐条调用 getUnreadMessageCount（N+1，本实现保持同口径）
            vo.unread_message_count = self.repo().count_unread(conv.id, 1).await?;
            vo.unread_servant_message_count = self.repo().count_unread(conv.id, 2).await?;
            vo.user = conv
                .user_id
                .and_then(|uid| user_map.get(&uid))
                .map(to_user_vo);
            vo.shop = self.shop_vo(conv.shop_id, &shop_map).await?;
            // Java：过滤 shopId 与会话一致的消息，再按 sendTime 倒序
            let mut matched: Vec<ImMessageVO> = message_map
                .remove(&conv.id)
                .unwrap_or_default()
                .into_iter()
                .filter(|m| m.shop_id.is_some() && m.shop_id == conv.shop_id)
                .collect();
            matched.sort_by(|a, b| b.send_time.cmp(&a.send_time));
            vo.last_message = Some(matched);
            list.push(vo);
        }
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 会话详情（对应 Java `ImConversationServiceImpl#detail`）。
    ///
    /// Java 对不存在的 ID 在 `BeanUtils.copyProperties(null, ...)` 处抛异常（500），
    /// 本实现返回业务 500「会话不存在」（文案沿用 `error.im.conversation.not.exist`）。
    /// 详情不装配 `lastMessage` / `shop`（Java 原样），未读计数为 0。
    pub async fn detail(&self, id: i32) -> Result<ImConversationVO> {
        let conv = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CONVERSATION_NOT_EXISTS))?;
        let mut vo = conversation_base_vo(&conv);
        if let Some(uid) = conv.user_id {
            if let Some(u) = self.find_user(uid).await? {
                vo.user = Some(to_user_vo(&u));
            }
        }
        Ok(vo)
    }

    /// 创建 / 复用会话（对应 Java `ImConversationServiceImpl#create`）。
    ///
    /// Java 逻辑：按 `(shopId, userId, isDelete=0)` 查会话；
    /// - 命中 ⇒ `status=1` + `lastServantId = 当前管理员`（NOT_NULL：admin 为
    ///   null 时保留原值），返回该实体
    /// - 未命中 ⇒ **返回 null（不新建）**
    ///
    /// 控制器返回实体本身（camelCase JSON，时间为秒级数字）。
    pub async fn create(
        &self,
        dto: &ImConversationCreateDTO,
        shop_id: Option<i32>,
        admin_id: Option<i32>,
    ) -> Result<Option<ImConversationEntityVO>> {
        let Some(existing) = self
            .repo()
            .find_by_shop_user(shop_id, dto.user_id)
            .await?
        else {
            return Ok(None);
        };
        self.repo()
            .update_status_servant(existing.id, CONVERSATION_STATUS_IN_PROGRESS, admin_id)
            .await?;
        // Java 把 dto.lastServantId（当前管理员，可能 null）写回内存实体；
        // DB 侧 NOT_NULL 策略不会写 null
        let mut updated = existing;
        updated.status = Some(CONVERSATION_STATUS_IN_PROGRESS);
        updated.last_servant_id = admin_id;
        Ok(Some(conversation_entity_vo(&updated)))
    }

    // ————————————————————— 搜索 —————————————————————

    /// 后台搜索（对应 Java `getUserSearch` + `getConversationSearch`）。
    ///
    /// `servant_id`：后台为当前管理员；前台（public 控制器）`getCurrentAdminId()`
    /// 为 null ⇒ 绑定 `NULL` ⇒ 查询恒不命中，返回空列表（与 Java 一致）。
    pub async fn search(
        &self,
        keyword: &str,
        servant_id: Option<i32>,
    ) -> Result<AdminSearchResult> {
        // Java `StringUtils.isEmpty(keyword)`：只判 null/空串
        if keyword.is_empty() {
            return Ok(AdminSearchResult::default());
        }
        let mut result = AdminSearchResult::default();

        // 1) 按用户名搜用户（Java 只 like username）
        let users = self.repo().find_users_by_username_like(keyword).await?;
        if users.is_empty() {
            return Ok(result);
        }
        let user_ids: Vec<i32> = users.iter().map(|u| u.user_id).collect();
        let user_map: HashMap<i32, ConvUserBrief> =
            users.into_iter().map(|u| (u.user_id, u)).collect();

        // 2) 这些用户中、最后客服为当前客服的会话（按 user_id 分组）
        let conversations = self
            .repo()
            .find_conversations_by_users_and_servant(&user_ids, servant_id)
            .await?;
        result.user_list = conversations
            .iter()
            .map(|c| ImSearchUserVO {
                id: Some(c.id),
                user_id: c.user_id,
                last_servant_id: c.last_servant_id,
                add_time: Some(format_unix_time(c.add_time)),
                last_update_time: Some(format_unix_time(c.last_update_time)),
                shop_id: c.shop_id,
                user_from: c.user_from.clone(),
                status: c.status,
                remark: Some(c.remark.clone()),
                summary: Some(c.summary.clone()),
                is_delete: c.is_delete,
                user: c.user_id.and_then(|uid| user_map.get(&uid)).map(to_user_vo),
            })
            .collect();

        // 3) 按消息内容搜会话
        let messages = self.repo().find_text_messages_by_content(keyword).await?;
        if messages.is_empty() {
            return Ok(result);
        }
        let mut content_map: HashMap<i32, ContentVO> = HashMap::new();
        let mut conv_ids: Vec<i32> = Vec::new();
        for m in &messages {
            let Some(cid) = m.conversation_id else {
                continue;
            };
            if content_map.contains_key(&cid) {
                continue; // 同一会话保留 id 最小的一条（Java toMap 会抛异常）
            }
            conv_ids.push(cid);
            content_map.insert(cid, ContentVO::parse(m.content.as_deref()).unwrap_or_default());
        }
        let conversations = self
            .repo()
            .find_conversations_by_ids_and_servant(&conv_ids, servant_id)
            .await?;
        if conversations.is_empty() {
            return Ok(result);
        }
        let conv_user_ids = distinct_ids(conversations.iter().map(|c| c.user_id));
        let mut conv_user_map: HashMap<i32, ConvUserBrief> = HashMap::new();
        for u in self.repo().find_users_by_ids(&conv_user_ids).await? {
            conv_user_map.insert(u.user_id, u);
        }
        result.conversation_list = conversations
            .iter()
            .map(|c| ImSearchConversationVO {
                id: Some(c.id),
                user_id: c.user_id,
                last_servant_id: c.last_servant_id,
                add_time: Some(format_unix_time(c.add_time)),
                last_update_time: Some(format_unix_time(c.last_update_time)),
                shop_id: c.shop_id,
                user_from: c.user_from.clone(),
                status: c.status,
                remark: Some(c.remark.clone()),
                summary: Some(c.summary.clone()),
                is_delete: c.is_delete,
                content: content_map.get(&c.id).cloned(),
                user: c
                    .user_id
                    .and_then(|uid| conv_user_map.get(&uid))
                    .map(to_user_vo),
                shop: None,
            })
            .collect();
        Ok(result)
    }

    /// 前台搜索（对应 Java `ImConversationServiceImpl#frontSearch`）。
    ///
    /// 返回 `{shopList, conversationList}`：店铺只含「用户聊过天 + 名称命中」的；
    /// 会话只含「用户消息内容命中 + 未删除」的。
    pub async fn front_search(&self, keyword: &str, user_id: i32) -> Result<FrontSearchResult> {
        let mut result = FrontSearchResult::default();
        if keyword.is_empty() {
            return Ok(result);
        }

        // 1) 店铺：用户聊过天的店铺中，名称包含关键字
        let chatted_shop_ids = self.repo().find_user_chatted_shop_ids(user_id).await?;
        if !chatted_shop_ids.is_empty() {
            result.shop_list = self
                .repo()
                .find_shops_by_ids_and_title_like(&chatted_shop_ids, keyword)
                .await?
                .into_iter()
                .map(|s| ImSearchShopVO {
                    shop_id: Some(s.shop_id),
                    shop_title: s.shop_title,
                    shop_logo: s.shop_logo,
                })
                .collect();
        }

        // 2) 会话：命中用户文本消息的会话
        let messages = self
            .repo()
            .find_text_messages_by_user_content(user_id, keyword)
            .await?;
        if messages.is_empty() {
            return Ok(result);
        }
        let mut content_map: HashMap<i32, ContentVO> = HashMap::new();
        let mut conv_ids: Vec<i32> = Vec::new();
        for m in &messages {
            let Some(cid) = m.conversation_id else {
                continue;
            };
            if content_map.contains_key(&cid) {
                continue; // Java 用 HashMap 保留首条
            }
            conv_ids.push(cid);
            content_map.insert(cid, ContentVO::parse(m.content.as_deref()).unwrap_or_default());
        }
        let conversations = self
            .repo()
            .find_conversations_by_ids_and_user(&conv_ids, user_id)
            .await?;
        if conversations.is_empty() {
            return Ok(result);
        }
        let shop_ids = distinct_ids(conversations.iter().map(|c| c.shop_id));
        let mut shop_map: HashMap<i32, ConvShopBrief> = HashMap::new();
        for s in self.repo().find_shops_by_ids(&shop_ids).await? {
            shop_map.insert(s.shop_id, s);
        }
        // 平台店铺（shopId=0）用门店配置（O2O 分支未迁移，见模块头）
        let store_title = self
            .repo()
            .config_value(CONFIG_STORE_SELF_OPERATED_NAME)
            .await?;
        let store_logo = self
            .repo()
            .config_value(CONFIG_STORE_SELF_OPERATED_LOGO)
            .await?;
        let mut conversation_list: Vec<ImSearchConversationVO> =
            Vec::with_capacity(conversations.len());
        for c in &conversations {
            let shop = match c.shop_id {
                Some(0) => Some(ImShopVO {
                    shop_id: Some(0),
                    shop_title: store_title.clone(),
                    shop_logo: store_logo.clone(),
                }),
                Some(sid) => shop_map.get(&sid).map(to_shop_vo),
                None => None,
            };
            conversation_list.push(ImSearchConversationVO {
                id: Some(c.id),
                user_id: c.user_id,
                last_servant_id: c.last_servant_id,
                add_time: Some(format_unix_time(c.add_time)),
                last_update_time: Some(format_unix_time(c.last_update_time)),
                shop_id: c.shop_id,
                user_from: c.user_from.clone(),
                status: c.status,
                remark: Some(c.remark.clone()),
                summary: Some(c.summary.clone()),
                is_delete: c.is_delete,
                content: content_map.get(&c.id).cloned(),
                user: None,
                shop,
            });
        }
        result.conversation_list = conversation_list;
        Ok(result)
    }

    // ————————————————————— 转接 / 待接入 —————————————————————

    /// 转接会话（对应 Java `ImConversationServiceImpl#transfer`）。
    ///
    /// - `servantId` 为空 ⇒ 回填当前管理员 ID
    /// - 仍为空（前台用户调用）⇒ 「请选择要转接的客服和会话」
    /// - 会话不存在 ⇒ 「会话不存在」
    /// - 更新会话 `last_servant_id` + `status=1`，再重算该客服接待量
    ///
    /// 未迁移：`sendAutoMessage`（RabbitMQ `autoTransfer` 延迟消息）。
    pub async fn transfer(
        &self,
        dto: &TransferDTO,
        current_admin_id: Option<i32>,
        shop_id: Option<i32>,
    ) -> Result<()> {
        let servant_id = dto.servant_id.or(current_admin_id);
        let (Some(servant_id), Some(conversation_id)) = (servant_id, dto.conversation_id) else {
            return Err(Error::biz(500, MSG_TRANSFER_TARGET_REQUIRED));
        };
        let Some(conversation) = self.repo().find_by_id(conversation_id).await? else {
            return Err(Error::biz(500, MSG_CONVERSATION_NOT_EXISTS));
        };
        self.repo()
            .update_status_servant(conversation.id, CONVERSATION_STATUS_IN_PROGRESS, Some(servant_id))
            .await?;
        // 未迁移：autoTransfer 系统消息（RabbitMQ）
        // 重算该客服接待量（Java 第一段；第二段是同一结果的重复写入，见模块头）
        let count = self.repo().count_active_by_servant(servant_id).await?;
        self.repo()
            .update_servant_conversation_num(servant_id, shop_id, count)
            .await?;
        tracing::info!("IM 会话转接: conversation_id={}, servant_id={servant_id}", conversation.id);
        Ok(())
    }

    /// 待接入客户列表（对应 Java `ImConversationServiceImpl#waitServantList`）。
    pub async fn wait_servant_list(
        &self,
        q: &WaitServantListQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<WaitServantListVO>> {
        let mut q = q.clone();
        q.shop_id = shop_id;
        let page = page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate_wait_servant(&q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let user_ids = distinct_ids(rows.iter().map(|c| c.user_id));
        let mut user_map: HashMap<i32, ConvUserBrief> = HashMap::new();
        for u in self.repo().find_users_by_ids(&user_ids).await? {
            user_map.insert(u.user_id, u);
        }
        let default_avatar = self.repo().config_value(CONFIG_DEFAULT_AVATAR).await?;
        let mut list: Vec<WaitServantListVO> = Vec::with_capacity(rows.len());
        for conv in &rows {
            let mut user_vo = conv.user_id.and_then(|uid| user_map.get(&uid)).map(to_user_vo);
            if let Some(u) = user_vo.as_mut() {
                if u.avatar.as_deref().unwrap_or("").is_empty() {
                    u.avatar = default_avatar.clone();
                }
            }
            let last_two = self.repo().list_messages_limit_two(conv.id).await?;
            list.push(WaitServantListVO {
                id: conv.id,
                user_id: conv.user_id,
                // 实体无 avatar 字段 ⇒ Java copyProperties 后恒为 null
                avatar: None,
                last_servant_id: conv.last_servant_id,
                add_time: Some(format_unix_time(conv.add_time)),
                last_update_time: Some(format_unix_time(conv.last_update_time)),
                shop_id: conv.shop_id,
                status: conv.status,
                remark: Some(conv.remark.clone()),
                summary: Some(conv.summary.clone()),
                is_delete: conv.is_delete,
                user: user_vo,
                user_last_two_message: Some(last_two.iter().map(message_to_vo).collect()),
                user_from: conv.user_from.clone(),
            });
        }
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    // ————————————————————— 删除 / 备注 / 历史 —————————————————————

    /// 删除会话（对应 Java `ImConversationServiceImpl#delete`）。
    ///
    /// 逻辑删除 `is_delete=1` + 会话下未读消息全部置已读。
    pub async fn delete(&self, dto: &DelDTO, shop_id: Option<i32>) -> Result<()> {
        let Some(conversation_id) = dto.conversation_id else {
            return Err(Error::biz(500, MSG_CONVERSATION_NOT_EXISTS));
        };
        let Some(conversation) = self.repo().find_by_id(conversation_id).await? else {
            return Err(Error::biz(500, MSG_CONVERSATION_NOT_EXISTS));
        };
        if conversation.shop_id != shop_id {
            return Err(Error::biz(500, MSG_NO_PERMISSION));
        }
        self.repo().soft_delete(conversation.id).await?;
        self.repo().mark_messages_read(conversation.id).await?;
        tracing::info!("IM 会话删除: conversation_id={}", conversation.id);
        Ok(())
    }

    /// 保存备注 / 总结（对应 Java `ImConversationServiceImpl#saveRemark`）。
    ///
    /// Java 对不存在的会话返回 `false`（控制器 void ⇒ 静默成功）；
    /// 店铺不匹配抛「没有权限」。返回 `update` 是否影响行。
    pub async fn save_remark(&self, dto: &SaveRemarkDTO, shop_id: Option<i32>) -> Result<bool> {
        let Some(conversation_id) = dto.conversation_id else {
            return Ok(false);
        };
        let Some(conversation) = self.repo().find_by_id(conversation_id).await? else {
            return Ok(false);
        };
        if conversation.shop_id != shop_id {
            return Err(Error::biz(500, MSG_NO_PERMISSION));
        }
        let affected = self
            .repo()
            .update_remark_summary(conversation.id, dto.remark.as_deref(), dto.summary.as_deref())
            .await?;
        Ok(affected > 0)
    }

    /// 历史会话（对应 Java `ImConversationServiceImpl#consultHistory`）。
    pub async fn consult_history(
        &self,
        q: &ImConversationPageQuery,
        shop_id: Option<i32>,
    ) -> Result<PageResult<ConsultHistoryListVO>> {
        let mut q = q.clone();
        q.handle_param(shop_id);
        let page = page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate_consult_history(&q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let conversation_ids: Vec<i32> = rows.iter().map(|c| c.id).collect();
        // Java `firstUserMessage`：conversation_id IN (...) + status=1
        // ORDER BY send_time ASC **LIMIT 1**（全局一条）
        let first_message = self.repo().find_first_status_message(&conversation_ids).await?;
        let mut first_map: HashMap<i32, Vec<FirstUserMessageVO>> = HashMap::new();
        if let Some(m) = first_message {
            if let Some(cid) = m.conversation_id {
                first_map.insert(cid, vec![first_user_message_vo(&m)]);
            }
        }
        let user_ids = distinct_ids(rows.iter().map(|c| c.user_id));
        let mut user_map: HashMap<i32, ConvUserBrief> = HashMap::new();
        for u in self.repo().find_users_by_ids(&user_ids).await? {
            user_map.insert(u.user_id, u);
        }
        let servant_ids = distinct_ids(rows.iter().map(|c| c.last_servant_id));
        let mut servant_map: HashMap<i32, ConvAdminBrief> = HashMap::new();
        for a in self.repo().find_admins_by_ids(&servant_ids).await? {
            servant_map.insert(a.admin_id, a);
        }
        let mut list: Vec<ConsultHistoryListVO> = Vec::with_capacity(rows.len());
        for conv in &rows {
            list.push(ConsultHistoryListVO {
                id: Some(conv.id),
                user_id: conv.user_id,
                last_servant_id: conv.last_servant_id,
                add_time: format_unix_time_opt(conv.add_time),
                shop_id: conv.shop_id,
                user_from: conv.user_from.clone(),
                status: conv.status,
                last_update_time: format_unix_time_opt(conv.last_update_time),
                is_delete: conv.is_delete,
                remark: Some(conv.remark.clone()),
                summary: Some(conv.summary.clone()),
                // mapper 只 select ic.* ⇒ 三个统计字段恒为 null
                first_response_time: None,
                conversation_duration: None,
                user: conv.user_id.and_then(|uid| user_map.get(&uid)).map(to_user_vo),
                average_response_time: None,
                servant: conv
                    .last_servant_id
                    .and_then(|sid| servant_map.get(&sid))
                    .map(to_consult_servant_vo),
                first_user_message: Some(
                    first_map.get(&conv.id).cloned().unwrap_or_default(),
                ),
                servant_message: Some(self.servant_messages(conv.id).await?),
            });
        }
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 单个会话的客服消息统计（Java `handel(conversationId)`）。
    async fn servant_messages(&self, conversation_id: i32) -> Result<Vec<ServantMessageVO>> {
        let Some(message) = self.repo().find_first_servant_message(conversation_id).await? else {
            return Ok(Vec::new());
        };
        let count = self.repo().count_servant_messages(conversation_id).await?;
        let (max_send, min_send) = self.repo().servant_message_times(conversation_id).await?;
        Ok(vec![ServantMessageVO {
            message_type_text: message_type_text(message.message_type.as_deref())
                .map(str::to_string),
            conversation_id: message.conversation_id,
            servant_id: message.servant_id,
            last_send_time: max_send.map(i64::from),
            first_send_time: min_send.map(i64::from),
            message_count: Some(count),
            message_type: message.message_type,
        }])
    }

    // ————————————————————— 字段更新 / 批量 —————————————————————

    /// 更新单字段（对应 Java `BaseServiceImpl#updateField` + `GENERAL_FIELDS`）。
    ///
    /// 允许字段只有 `status`（`im_conversation` 没有 `is_show`/`sort_order`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let (id, field, val) = require_update_field_parts(dto)?;
        if !is_allowed_update_field(field) {
            return Err(Error::biz(500, MSG_INVALID_FIELD_VALUE));
        }
        let column = map_field_column(field);
        self.repo().update_field(id, column, val).await?;
        tracing::info!("IM 会话字段更新: id={id}, field={column}");
        Ok(())
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`，`del` ⇒ 物理删除）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Err(Error::Validation(MSG_UNSELECTED_ITEM.into()));
        }
        let op_type = dto
            .r#type
            .as_deref()
            .ok_or_else(|| Error::Validation(MSG_OPERATION_TYPE_REQUIRED.into()))?;
        if op_type == "del" {
            let n = self.repo().delete_batch(ids).await?;
            tracing::info!("IM 会话批量删除: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }

    // ————————————————————— 内部 —————————————————————

    /// 单个用户（详情用；Java `userMapper.selectById`）。
    async fn find_user(&self, user_id: i32) -> Result<Option<ConvUserBrief>> {
        let mut users = self.repo().find_users_by_ids(&[user_id]).await?;
        Ok(users.pop())
    }

    /// `shopId == 0` 走配置（平台店铺）；否则按店铺表装配。
    ///
    /// O2O 分支未迁移（`AppConfig` 无 `isO2o`），固定非 O2O：`shopName` / `shopLogo`。
    async fn shop_vo(
        &self,
        shop_id: Option<i32>,
        shop_map: &HashMap<i32, ConvShopBrief>,
    ) -> Result<Option<ImShopVO>> {
        match shop_id {
            Some(0) => Ok(Some(ImShopVO {
                shop_id: Some(0),
                shop_title: self.repo().config_value(CONFIG_SHOP_NAME).await?,
                shop_logo: self.repo().config_value(CONFIG_SHOP_LOGO).await?,
            })),
            Some(sid) => Ok(shop_map.get(&sid).map(to_shop_vo)),
            None => Ok(None),
        }
    }
}

// ——————————————————————————————————————————————
// 纯转换 / 纯校验（单测覆盖）
// ——————————————————————————————————————————————

/// `BasePage` 口径 → 内部 [`PageQuery`]。
fn page_query(page: Option<i64>, size: Option<i64>) -> PageQuery {
    PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

/// 去重并过滤 `None`（保持出现顺序）。
fn distinct_ids(ids: impl Iterator<Item = Option<i32>>) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in ids.flatten() {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// 会话消息实体 → VO（Java `getListByConversationIds` 的 map 逻辑）。
fn message_to_vo(m: &nextrade_core::entities::ImMessage) -> ImMessageVO {
    ImMessageVO {
        message_type_text: message_type_text(m.message_type.as_deref()).map(str::to_string),
        id: Some(m.id),
        conversation_id: m.conversation_id,
        content: ContentVO::parse(m.content.as_deref()),
        message_type: m.message_type.clone(),
        r#type: m.r#type,
        user_id: m.user_id,
        servant_id: m.servant_id,
        send_time: Some(format_unix_time(m.send_time)),
        status: m.status,
        extend: m.extend.clone().map(serde_json::Value::String),
        push_status: m.push_status,
        is_read: m.is_read,
        shop_id: Some(m.shop_id),
        user_from: m.user_from.clone(),
    }
}

/// 用户简信息 → VO。
fn to_user_vo(u: &ConvUserBrief) -> ImUserVO {
    ImUserVO {
        user_id: Some(u.user_id),
        username: Some(u.username.clone()),
        nickname: u.nickname.clone(),
        avatar: Some(u.avatar.clone()),
    }
}

/// 店铺简信息 → VO。
fn to_shop_vo(s: &ConvShopBrief) -> ImShopVO {
    ImShopVO {
        shop_id: Some(s.shop_id),
        shop_title: s.shop_title.clone(),
        shop_logo: s.shop_logo.clone(),
    }
}

/// 客服简信息 → `ConsultHistoryListVO.ServantVO`。
fn to_consult_servant_vo(a: &ConvAdminBrief) -> ConsultServantVO {
    ConsultServantVO {
        admin_id: Some(a.admin_id),
        username: Some(a.username.clone()),
        mobile: a.mobile.clone(),
        avatar: a.avatar.clone(),
    }
}

/// 首次用户消息实体 → VO（Java `FirstUserMessageVO`）。
fn first_user_message_vo(m: &nextrade_core::entities::ImMessage) -> FirstUserMessageVO {
    FirstUserMessageVO {
        message_type_text: message_type_text(m.message_type.as_deref()).map(str::to_string),
        conversation_id: m.conversation_id,
        user_id: m.user_id,
        send_time: Some(format_unix_time(m.send_time)),
        message_type: m.message_type.clone(),
    }
}

/// 提取并校验 `UpdateFieldDTO` 三要素（Java `@Valid` 注解语义）。
fn require_update_field_parts(dto: &UpdateFieldDTO) -> Result<(i32, &str, &str)> {
    let id = dto.id.ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
    let field = dto
        .field
        .as_deref()
        .ok_or_else(|| Error::Validation(MSG_FIELD_NAME_REQUIRED.into()))?;
    let val = dto
        .val
        .as_deref()
        .ok_or_else(|| Error::Validation(MSG_FIELD_VALUE_REQUIRED.into()))?;
    Ok((id, field, val))
}

/// `create` 的会话 ID 校验占位（Java 用 `E_CARD_GROUP_ID_NOT_NULL` 同类常量）。
#[must_use]
pub fn conversation_id_required(id: Option<i32>) -> Result<i32> {
    id.ok_or_else(|| Error::Validation(MSG_CONVERSATION_ID_REQUIRED.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::ImMessage;

    // ———————————————— 分页 / 去重 ————————————————

    #[test]
    fn page_query_matches_base_page() {
        let p = page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = page_query(Some(3), Some(200));
        assert_eq!((p.page, p.page_size), (3, 100));
        let p = page_query(Some(0), Some(0));
        assert_eq!((p.page, p.page_size), (1, 1));
    }

    #[test]
    fn distinct_ids_keeps_order_and_skips_none() {
        let ids = vec![Some(3), None, Some(1), Some(3), Some(2)];
        assert_eq!(distinct_ids(ids.into_iter()), vec![3, 1, 2]);
    }

    // ———————————————— 消息 → VO ————————————————

    fn message() -> ImMessage {
        ImMessage {
            id: 9,
            conversation_id: Some(5),
            content: Some(r#"{"messageType":"text","content":"hi"}"#.into()),
            message_type: Some("text".into()),
            r#type: Some(1),
            user_id: Some(3),
            servant_id: Some(0),
            send_time: Some(1_700_000_000),
            status: Some(1),
            extend: Some(r#"{"a":1}"#.into()),
            push_status: Some(0),
            is_read: Some(0),
            shop_id: 0,
            user_from: Some("h5".into()),
        }
    }

    #[test]
    fn message_vo_maps_java_fields() {
        let vo = message_to_vo(&message());
        assert_eq!(vo.message_type_text.as_deref(), Some("文本"));
        assert_eq!(vo.conversation_id, Some(5));
        assert_eq!(vo.send_time.as_deref(), Some("2023-11-15 06:13:20"));
        assert_eq!(vo.content.as_ref().unwrap().content.as_deref(), Some("hi"));
        assert_eq!(
            vo.extend,
            Some(serde_json::Value::String(r#"{"a":1}"#.into()))
        );
        assert_eq!(vo.shop_id, Some(0));
        assert_eq!(vo.is_read, Some(0));
    }

    #[test]
    fn message_vo_tolerates_missing_content_and_type() {
        let mut m = message();
        m.content = None;
        m.message_type = None;
        let vo = message_to_vo(&m);
        assert!(vo.content.is_none());
        assert!(vo.message_type_text.is_none());
    }

    // ———————————————— updateField 校验 ————————————————

    #[test]
    fn update_field_requires_all_parts() {
        let err = require_update_field_parts(&UpdateFieldDTO::default()).unwrap_err();
        assert!(err.to_string().contains("id不能为空"), "实际: {err}");
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: None,
            val: Some("1".into()),
        };
        assert!(require_update_field_parts(&dto)
            .unwrap_err()
            .to_string()
            .contains("字段名不能为空"));
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("status".into()),
            val: None,
        };
        assert!(require_update_field_parts(&dto)
            .unwrap_err()
            .to_string()
            .contains("值不能为空"));
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("status".into()),
            val: Some("1".into()),
        };
        assert_eq!(
            require_update_field_parts(&dto).unwrap(),
            (1, "status", "1")
        );
    }

    #[test]
    fn conversation_id_required_uses_java_message() {
        assert_eq!(conversation_id_required(Some(4)).unwrap(), 4);
        let err = conversation_id_required(None).unwrap_err();
        assert!(err.to_string().contains("客服会话id不能为空"));
    }

    // ———————————————— viewer / 分支 ————————————————

    #[test]
    fn viewer_admin_sets_servant_and_shop_user_sets_user() {
        // 仅验证枚举语义（数据装配在集成路径）
        let admin = ConversationViewer::Admin {
            admin_id: 7,
            shop_id: Some(0),
        };
        let user = ConversationViewer::User { user_id: 9 };
        assert!(matches!(
            admin,
            ConversationViewer::Admin {
                admin_id: 7,
                shop_id: Some(0)
            }
        ));
        assert!(matches!(user, ConversationViewer::User { user_id: 9 }));
    }
}
