//! IM 消息（ImMessageController） 域服务（im::im_message）。
//!
//! 对应原 Java `com.nextrade.service.im.impl.ImMessageServiceImpl`
//! 的 `list` / `receivedList` / `send` / `setRead` 四条链路
//! （控制器：`nextrade-adminapi` 与 `nextrade-api` 的 `ImMessageController`）。
//!
//! 行为对齐 Java：
//! - **身份分派**：Java 用 `TigUtils.getClientTypeByToken()` 判断前/后台。
//!   Rust 侧两条通道天然区分——admin 根级路由即后台（`is_admin=true`），
//!   public 根级路由即前台（`is_admin=false`），故本服务提供
//!   `*_as_admin` / `*_as_user` 两组方法分别复刻两个分支。
//! - **多租户 / 归属**：`shopId`、`servantId`、`userId` 由路由层按登录态注入，
//!   口径与 Java 一致。
//! - **分页 / 排序**：`list` 走 `buildSortOrder`（空 sortField → 主键降序），
//!   取页后再按 `sendTime, id` 升序重排（Java 原样）；`receivedList`
//!   固定 `type=1 AND status=1`，最新在前、不做会话内重排。
//! - **异常文案**：`会话不存在` / `token数据验证失败`(401) /
//!   `创建会话失败` / `视频消息缺少视频地址` / `会话id和用户id不能为空` /
//!   `error.user.not.login` 逐字照抄 Java。
//!
//! **未迁移（依赖外部基础设施，本构建不具备，绝不伪造）**：
//! 1. **WebSocket 推送**：Java `sentMessage` 通过 `APIClient.sendMessage`
//!    向 `false::{userId}` / `true::{servantId}` 实时推送消息，
//!    `setRead` 通过 `sendReadToClient` 推送已读事件。Rust 侧无 WS 通道，
//!    故只保证「落库正确 + 返回体形状正确」，推送环节跳过。
//! 2. **自动回复**：Java `send` 首接 / 重新激活触发 RabbitMQ 延迟 2s；本实现即时写入并 WS 推送，
//!    无在线客服时投递 `autoWelcome` / `autoWaiting` / `autoOffWork`
//!    自动消息（`sendAutoMessage` → RabbitMQ）。Rust 侧无 MQ，故会话接入的
//!    **库表写入**（客服分配、状态、接待数）照常执行，但**不投递**自动回复。
//! 3. **XSS 过滤**：Java 对 `text` 消息 `XssUtils.clean(content)`。Rust 侧
//!    无对应实现，按原样落库（见 `uncertain_points`）。
//! 4. **kefuHistoryList 权限**：Java `list`（后台）在
//!    `conversationId>0 && shopId>-1 && hasPermission("kefuHistoryList")`
//!    时放开 `servantId` 过滤（查看整段历史）。Rust 未迁移权限位校验，
//!    统一走**最小权限**分支（`servantId = 当前登录客服`），见 `uncertain_points`。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_core::entities::ImMessage;
use nextrade_core::models::im::im_message::{
    message_type_text, ContentVO, ImMessageListDTO, ImMessageListVO, ImMessageReceivedListDTO,
    ImMessageSendDTO, ImMessageSendVO, ImSetReadDTO, MessageServantVO, MessageUserVO,
    UploadVideoInfoVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::im::im_message::{
    AdminBrief, MessageListFilter, NewMessage, ImMessageRepository, UserBrief,
};
use nextrade_infrastructure::db::Repositories;

/// IM 消息（ImMessageController） 域服务。
pub struct ImMessageService {
    repos: Arc<Repositories>,
}

impl ImMessageService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &ImMessageRepository {
        &self.repos.im.im_message
    }

    // ————————————————————— 列表 —————————————————————

    /// 后台消息列表（对应 adminapi `ImMessageController.list` 的 `isAdmin` 分支）。
    ///
    /// - `shop_id`：来自请求头（`HeaderUtils.getShopId()`）；`Some(v)`（平台 0 /
    ///   店铺 v，且 v > -1）→ 过滤 `shop_id`；`None`（未知账号）→ 不过滤。
    /// - `admin_id`：当前登录客服，作为 `servant_id` 过滤
    ///   （未迁移 `kefuHistoryList` 的历史放开分支）。
    pub async fn list_as_admin(
        &self,
        dto: &ImMessageListDTO,
        shop_id: Option<i32>,
        admin_id: i32,
    ) -> Result<PageResult<ImMessageListVO>> {
        let filter = MessageListFilter {
            conversation_id: dto.conversation_id,
            shop_id: dto.shop_id.or(shop_id).filter(|v| *v > -1),
            user_id: dto.user_id,
            first_id: dto.first_id,
            servant_id: Some(admin_id),
            msg_type: dto.r#type,
            is_read: dto.is_read,
        };
        self.page_messages(dto, &filter).await
    }

    /// 前台消息列表（对应 api `ImMessageController.list` 的 `!isAdmin` 分支）。
    ///
    /// `user_id` 由登录态覆盖（`SecurityUtils.getCurrentUserId()`）。
    pub async fn list_as_user(
        &self,
        dto: &ImMessageListDTO,
        user_id: i32,
    ) -> Result<PageResult<ImMessageListVO>> {
        let filter = MessageListFilter {
            conversation_id: dto.conversation_id,
            shop_id: dto.shop_id.filter(|v| *v > -1),
            user_id: Some(user_id),
            first_id: dto.first_id,
            servant_id: None,
            msg_type: dto.r#type,
            is_read: dto.is_read,
        };
        self.page_messages(dto, &filter).await
    }

    /// 客服消息播报记录列表（对应 adminapi `ImMessageController.receivedList`）。
    ///
    /// 与 Java 同口径：`servant_id = 当前客服`、`type = 1`（用户发来）、
    /// `status = 1`（正常），`shop_id` 非空且 > -1 时过滤；最新在前、不做重排。
    pub async fn received_list(
        &self,
        dto: &ImMessageReceivedListDTO,
        shop_id: Option<i32>,
        servant_id: i32,
    ) -> Result<PageResult<ImMessageListVO>> {
        let paging = dto.paging();
        let (limit, offset) = paging.limit_offset();
        let (total, rows) = self
            .repo()
            .page_received(servant_id, shop_id, limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(empty_page(&paging));
        }
        let vos = self.assemble_list_vos(&rows).await?;
        Ok(build_page(total, vos, &paging))
    }

    /// 两个列表分支共用的分页 + 装配（`list` 语义）。
    async fn page_messages(
        &self,
        dto: &ImMessageListDTO,
        filter: &MessageListFilter,
    ) -> Result<PageResult<ImMessageListVO>> {
        let paging = dto.paging();
        let (limit, offset) = paging.limit_offset();
        let order_by = list_order_by(dto.sort_field.as_deref(), dto.sort_order.as_deref());
        let (total, rows) = self
            .repo()
            .page_messages(filter, &order_by, limit, offset)
            .await?;
        // Java：records 为空 → `CollUtil.isEmpty(userIds)` → 返回空页
        if rows.is_empty() {
            return Ok(empty_page(&paging));
        }
        let mut vos = self.assemble_list_vos(&rows).await?;
        // Java 取页后按 sendTime, id 升序重排当前页
        vos.sort_by(|a, b| (&a.send_time, a.id).cmp(&(&b.send_time, b.id)));
        Ok(build_page(total, vos, &paging))
    }

    /// 批量装配列表 VO（用户 / 客服回填 + 头像默认 + 内容解析）。
    async fn assemble_list_vos(&self, rows: &[ImMessage]) -> Result<Vec<ImMessageListVO>> {
        let user_ids: Vec<i32> = rows.iter().filter_map(|r| r.user_id).collect();
        let servant_ids: Vec<i32> = rows.iter().filter_map(|r| r.servant_id).collect();
        let default_avatar = self.default_avatar().await?;
        let users = to_user_map(
            self.repo().find_users_by_ids(&dedup(&user_ids)).await?,
            &default_avatar,
        );
        let servants = to_admin_map(self.repo().find_admins_by_ids(&dedup(&servant_ids)).await?);
        Ok(rows
            .iter()
            .map(|r| {
                let mut vo = message_core(r);
                vo.user = r.user_id.and_then(|id| users.get(&id)).cloned();
                vo.servant = r.servant_id.and_then(|id| servants.get(&id)).cloned();
                vo
            })
            .collect())
    }

    // ————————————————————— 发送 —————————————————————

    /// 后台发送消息（对应 adminapi `ImMessageController.send` 的 `isAdmin` 分支）。
    ///
    /// Java：会话必须已存在（`selectById`，否则 `会话不存在`）；
    /// `type=2`、`servantId=当前客服`、`shopId`（请求头非空时覆盖）。
    pub async fn send_as_admin(
        &self,
        dto: &ImMessageSendDTO,
        admin_id: i32,
        header_shop_id: Option<i32>,
    ) -> Result<ImMessageSendVO> {
        let conversation_id = match dto.conversation_id {
            Some(id) if id > 0 => id,
            _ => return Err(Error::biz_msg("会话不存在")),
        };
        let conversation = self
            .repo()
            .find_conversation(conversation_id)
            .await?
            .ok_or_else(|| Error::biz_msg("会话不存在"))?;

        let user_from = conversation.user_from.clone();
        // 请求头 shopId 非空则覆盖，否则回退请求体 / 会话所属店铺（避免 NOT NULL 落空）
        let shop_id = header_shop_id
            .or(dto.shop_id)
            .or(conversation.shop_id)
            .unwrap_or(0);
        let user_id = dto.user_id.or(conversation.user_id);

        self.persist_message(
            dto,
            conversation_id,
            user_id,
            Some(admin_id), // servantId = 当前客服
            2,              // type = 2（客服发）
            shop_id,
            user_from.as_deref(),
        )
        .await
    }

    /// 前台发送消息（对应 api `ImMessageController.send` 的 `!isAdmin` 分支）。
    ///
    /// `type=1`、`userId=当前登录用户`、`userFrom=x-client-type` 请求头。
    /// 会话接入 / 创建 / 重新激活（客服分配、状态、接待数）按 Java 落库；
    /// **自动回复（RabbitMQ）与 WS 推送未迁移**。
    pub async fn send_as_user(
        &self,
        dto: &ImMessageSendDTO,
        user_id: i32,
        user_from_header: Option<String>,
    ) -> Result<ImMessageSendVO> {
        if user_id == 0 {
            return Err(Error::biz(401, "token数据验证失败"));
        }
        // shop_id：前台由请求体带入；NOT NULL，缺失回退 0（见 uncertain_points）
        let shop_id = dto.shop_id.unwrap_or(0);

        let mut auto_reply = false;
        let (conversation_id, servant_id, user_from) = match dto.conversation_id {
            Some(cid) if cid > 0 => {
                let conv = self
                    .repo()
                    .find_conversation(cid)
                    .await?
                    .ok_or_else(|| Error::biz_msg("会话不存在"))?;
                let was_active = conv.status == Some(1);
                let conv = self.activate_conversation_if_needed(conv, shop_id).await?;
                if !was_active {
                    auto_reply = true;
                }
                (conv.id, conv.last_servant_id.unwrap_or(0), conv.user_from)
            }
            _ => {
                let existing = self
                    .repo()
                    .find_conversation_by_user_shop(user_id, shop_id)
                    .await?;
                match existing {
                    Some(conv) => {
                        let was_active = conv.status == Some(1);
                        let conv = self.activate_conversation_if_needed(conv, shop_id).await?;
                        if !was_active {
                            auto_reply = true;
                        }
                        (conv.id, conv.last_servant_id.unwrap_or(0), conv.user_from)
                    }
                    None => {
                        let created = self
                            .create_conversation(user_id, shop_id, user_from_header.as_deref())
                            .await?;
                        auto_reply = true;
                        created
                    }
                }
            }
        };

        let vo = self
            .persist_message(
                dto,
                conversation_id,
                Some(user_id),
                Some(servant_id),
                1, // type = 1（用户发）
                shop_id,
                user_from.as_deref(),
            )
            .await?;

        if auto_reply {
            if let Err(e) = self
                .send_auto_reply(conversation_id, user_id, shop_id, servant_id, user_from.as_deref())
                .await
            {
                tracing::warn!("IM 自动回复失败: {e}");
            }
        }
        Ok(vo)
    }

    /// 自动回复（替代 Java RabbitMQ 延迟队列，立即写入并 WS 推送）。
    ///
    /// 规则对齐 Java：
    /// - 已分配客服 → `welcome`
    /// - 无客服且存在忙碌客服 → `waiting`
    /// - 其余 → `off_work`
    async fn send_auto_reply(
        &self,
        conversation_id: i32,
        user_id: i32,
        shop_id: i32,
        servant_id: i32,
        user_from: Option<&str>,
    ) -> Result<()> {
        use nextrade_core::models::im::im_config::ConfigDataDTO;
        let code = if servant_id > 0 {
            "welcome"
        } else if self
            .repo()
            .shop_has_servant_status(shop_id, 2)
            .await?
        {
            "waiting"
        } else {
            "off_work"
        };
        let Some(row) = self
            .repos
            .im
            .im_config
            .find_by_code(code, Some(shop_id))
            .await?
        else {
            return Ok(());
        };
        let Some(data) = ConfigDataDTO::from_json_str(row.data.as_deref()) else {
            return Ok(());
        };
        if !data.activate {
            return Ok(());
        }
        let now = now_seconds();
        let mut sent_any = false;
        if data.send_text {
            if let Some(content) = data.reply_content.as_deref().filter(|s| !s.trim().is_empty()) {
                let json = serde_json::json!({
                    "messageType": "text",
                    "content": content,
                });
                let msg = NewMessage {
                    conversation_id: Some(conversation_id),
                    content: Some(json.to_string()),
                    message_type: "text".to_string(),
                    msg_type: 2,
                    user_id: Some(user_id),
                    servant_id: Some(servant_id),
                    send_time: now,
                    shop_id,
                    user_from: user_from.map(ToString::to_string),
                };
                let id = self.repo().insert_message(&msg).await?;
                sent_any = true;
                if let Ok(Some(row)) = self.repo().find_message(id).await {
                    let vo = self.build_send_vo(&row).await?;
                    let payload = serde_json::json!({"type":"message","data":[vo]}).to_string();
                    nextrade_security::ws_registry::send_to_user(
                        &nextrade_security::ws_registry::user_key(false, &user_id.to_string()),
                        &payload,
                    );
                    if servant_id > 0 {
                        nextrade_security::ws_registry::send_to_user(
                            &nextrade_security::ws_registry::user_key(true, &servant_id.to_string()),
                            &payload,
                        );
                    }
                }
            }
        }
        if data.send_wechat {
            if let Some(pic) = data.wechat_image.filter(|s| !s.trim().is_empty()) {
                let json = serde_json::json!({"messageType":"image","pic":pic});
                let msg = NewMessage {
                    conversation_id: Some(conversation_id),
                    content: Some(json.to_string()),
                    message_type: "image".to_string(),
                    msg_type: 2,
                    user_id: Some(user_id),
                    servant_id: Some(servant_id),
                    send_time: now,
                    shop_id,
                    user_from: user_from.map(ToString::to_string),
                };
                self.repo().insert_message(&msg).await?;
                sent_any = true;
            }
        }
        if sent_any {
            let _ = self.repo().touch_conversation(conversation_id, now).await;
            tracing::info!("IM 自动回复已发送: code={code}, conversation_id={conversation_id}");
        }
        Ok(())
    }

    /// 会话重新接入（复刻 Java `status != 1` 重激活 / 进行中客服离线改派分支；
    /// **不含**自动回复投递）。
    async fn activate_conversation_if_needed(
        &self,
        mut conv: nextrade_core::entities::ImConversation,
        shop_id: i32,
    ) -> Result<nextrade_core::entities::ImConversation> {
        if conv.status == Some(1) {
            // 进行中：当前客服缺失或离线时按接入规则改派
            let current = conv.last_servant_id.unwrap_or(0);
            if current == 0 || !self.repo().is_servant_online(current).await? {
                if let Some(servant) = self.resolve_servant(shop_id, Some(current)).await? {
                    let sid = servant.servant_id.unwrap_or(0);
                    self.repo().update_conversation_servant(conv.id, sid, 1).await?;
                    conv.last_servant_id = Some(sid);
                    conv.status = Some(1);
                } else {
                    self.repo().update_conversation_servant(conv.id, 0, 0).await?;
                    conv.last_servant_id = Some(0);
                    conv.status = Some(0);
                }
            }
            return Ok(conv);
        }
        // 非进行中：重新接入
        if let Some(servant) = self.resolve_servant(shop_id, conv.last_servant_id).await? {
            let sid = servant.servant_id.unwrap_or(0);
            self.repo().update_conversation_servant(conv.id, sid, 1).await?;
            conv.last_servant_id = Some(sid);
            conv.status = Some(1);
        } else {
            self.repo().update_conversation_servant(conv.id, 0, 0).await?;
            conv.last_servant_id = Some(0);
            conv.status = Some(0);
        }
        Ok(conv)
    }

    /// 新建会话（Java send 的 `imConversation == null` 分支），返回
    /// `(conversationId, servantId, userFrom)`。客服接待数同步刷新。
    async fn create_conversation(
        &self,
        user_id: i32,
        shop_id: i32,
        user_from: Option<&str>,
    ) -> Result<(i32, i32, Option<String>)> {
        let servant = self.resolve_servant(shop_id, None).await?;
        let servant_id = servant.and_then(|s| s.servant_id).unwrap_or(0);
        let status = if servant_id > 0 { 1 } else { 0 };
        let now = now_seconds();
        let new_id = self
            .repo()
            .insert_conversation(user_id, shop_id, user_from, servant_id, now, status)
            .await?;
        // Java：insert 返回受影响行数，0 视为失败
        if new_id == 0 {
            return Err(Error::biz_msg("创建会话失败"));
        }
        if servant_id > 0 {
            let count = self.repo().count_active_conversations(servant_id).await?;
            let _ = self
                .repo()
                .update_servant_conversation_num(servant_id, shop_id, clamp_i32(count))
                .await;
        }
        Ok((new_id, servant_id, user_from.map(ToString::to_string)))
    }

    /// 按接入规则解析应分配的客服（Java `resolveServant`）。
    ///
    /// - `auto_access_mode == 1`（全部手动接入）→ `None`（进待接入列表）；
    /// - `return_customer_access == 1` 且上次客服在线 → 复用；
    /// - 否则 → 在线客服中接待数最少者。
    ///
    /// 缺 `shop_kefu_config` 行时按默认（`0 / 0`）处理，不落库创建默认行
    /// （Java `getByShopId` 会 `createDefault`，见 `uncertain_points`）。
    async fn resolve_servant(
        &self,
        shop_id: i32,
        previous: Option<i32>,
    ) -> Result<Option<nextrade_core::entities::ImServant>> {
        let (auto_access_mode, return_customer_access) =
            match self.repo().find_kefu_config(shop_id).await? {
                Some(c) => (c.auto_access_mode, c.return_customer_access),
                None => (0, 0),
            };
        if auto_access_mode == 1 {
            return Ok(None);
        }
        if return_customer_access == 1 {
            if let Some(prev) = previous.filter(|v| *v > 0) {
                if let Some(last) = self.repo().find_online_servant_by_id(prev, shop_id).await? {
                    return Ok(Some(last));
                }
            }
        }
        self.repo().find_least_loaded_online_servant(shop_id).await
    }

    /// 落库消息 + 刷新会话更新时间 + 组装发送 VO。
    ///
    /// 内容处理对齐 Java：文本内容原样（XSS 未迁移）、视频消息必须带 `videoUrl`
    /// （否则 `视频消息缺少视频地址`）；`is_read=0`、`servant_id` 缺省 0。
    async fn persist_message(
        &self,
        dto: &ImMessageSendDTO,
        conversation_id: i32,
        user_id: Option<i32>,
        servant_id: Option<i32>,
        msg_type: i32,
        shop_id: i32,
        user_from: Option<&str>,
    ) -> Result<ImMessageSendVO> {
        let content = dto.content.clone().unwrap_or_default();
        let message_type = content.effective_message_type();
        if message_type == "video" && content.video_url.as_deref().unwrap_or("").trim().is_empty() {
            return Err(Error::biz_msg("视频消息缺少视频地址"));
        }
        let content_json = content.to_json_string()?;

        let new_msg = NewMessage {
            conversation_id: Some(conversation_id),
            content: Some(content_json),
            message_type,
            msg_type,
            user_id,
            servant_id: Some(servant_id.unwrap_or(0)),
            send_time: now_seconds(),
            shop_id,
            user_from: user_from.map(ToString::to_string),
        };
        let id = self.repo().insert_message(&new_msg).await?;

        // 刷新会话最后更新时间（Java sentMessage）
        let _ = self.repo().touch_conversation(conversation_id, new_msg.send_time).await;

        // 回读落库行以组装与 Java 完全一致的发送 VO
        let row = self
            .repo()
            .find_message(id)
            .await?
            .ok_or_else(|| Error::biz_msg("消息发送失败"))?;

        let vo = self.build_send_vo(&row).await?;
        self.push_message_ws(&vo, &row);
        Ok(vo)
    }

    /// 消息落库后按 Java `sentMessage` 语义推送 WebSocket。
    ///
    /// - 客服发（`msg_type=2`）→ 推送给会话会员 `false::{userId}`；
    /// - 会员发（`msg_type=1`）→ 有客服则推给该客服 `true::{servantId}`，
    ///   否则广播给所有在线管理员（等价 Java 未分配客服时的兜底）。
    fn push_message_ws(
        &self,
        vo: &nextrade_core::models::im::im_message::ImMessageSendVO,
        row: &nextrade_core::entities::ImMessage,
    ) {
        use nextrade_security::ws_registry;
        let payload = match serde_json::to_string(&serde_json::json!({
            "type": "message",
            "data": [vo],
        })) {
            Ok(payload) => payload,
            Err(e) => {
                tracing::warn!("IM WS 推送序列化失败: {e}");
                return;
            }
        };
        let sent = if row.r#type.unwrap_or(0) == 2 {
            row.user_id
                .map(|uid| {
                    ws_registry::send_to_user(
                        &ws_registry::user_key(false, &uid.to_string()),
                        &payload,
                    )
                })
                .unwrap_or(0)
        } else {
            let servant_id = row.servant_id.unwrap_or(0);
            if servant_id > 0 {
                ws_registry::send_to_user(
                    &ws_registry::user_key(true, &servant_id.to_string()),
                    &payload,
                )
            } else {
                ws_registry::send_to_admins(&payload)
            }
        };
        tracing::info!(
            sent,
            conversation_id = row.conversation_id,
            msg_type = row.r#type.unwrap_or(0),
            "IM 消息 WebSocket 推送"
        );
    }

    /// 组装发送 VO（Java `sendMessageVO`：消息字段 + 用户 + 客服）。
    async fn build_send_vo(&self, row: &ImMessage) -> Result<ImMessageSendVO> {
        let default_avatar = self.default_avatar().await?;
        let core = message_core(row);
        let user = match row.user_id {
            Some(id) => self
                .repo()
                .find_user(id)
                .await?
                .map(|u| apply_user_avatar(u, &default_avatar)),
            None => None,
        };
        let servant = match row.servant_id {
            Some(id) if id > 0 => self.repo().find_admin(id).await?,
            _ => None,
        };
        let servant_vo = servant.map(|a| MessageServantVO {
            admin_id: Some(a.admin_id),
            username: Some(a.username),
            avatar: a.avatar,
        });
        Ok(ImMessageSendVO {
            message_type_text: core.message_type_text,
            id: core.id,
            conversation_id: core.conversation_id,
            content: core.content,
            message_type: core.message_type,
            r#type: core.r#type,
            user_id: core.user_id,
            servant_id: core.servant_id,
            send_time: core.send_time,
            status: core.status,
            extend: core.extend,
            push_status: core.push_status,
            is_read: core.is_read,
            shop_id: core.shop_id,
            user_from: core.user_from,
            user,
            servant: servant_vo,
        })
    }

    // ————————————————————— 设置已读 —————————————————————

    /// 后台设置已读（对应 adminapi `ImMessageController.setRead` 的 `isAdmin` 分支）。
    ///
    /// Java：`conversationId` 与 `userId` 任一为空即 `会话id和用户id不能为空`；
    /// `shopId` 为空回退请求头；命中行 `is_read=1`（`type=1`）。
    /// `sendReadToClient`（WS）未迁移。
    pub async fn set_read_as_admin(
        &self,
        dto: &ImSetReadDTO,
        header_shop_id: Option<i32>,
    ) -> Result<bool> {
        if dto.conversation_id.is_none() || dto.user_id.is_none() {
            return Err(Error::biz_msg("会话id和用户id不能为空"));
        }
        let shop_id = dto.shop_id.or(header_shop_id);
        // shop_id 为空时 Java 生成 `shop_id = NULL` 匹配不到任何行 → 直接返回成功
        if let (Some(shop_id), Some(user_id)) = (shop_id, dto.user_id) {
            self.repo().mark_read_for_admin(shop_id, user_id).await?;
            // Java `sendReadToClient`：通知会员「客服已读」
            let payload = read_receipt_payload(Some(shop_id), dto.conversation_id, user_id);
            nextrade_security::ws_registry::send_to_user(
                &nextrade_security::ws_registry::user_key(false, &user_id.to_string()),
                &payload,
            );
        }
        Ok(true)
    }

    /// 前台设置已读（对应 api `ImMessageController.setRead` 的 `!isAdmin` 分支）。
    ///
    /// Java：`userId` 取登录态，为 0 抛 `error.user.not.login`；命中行
    /// `is_read=1`（`type=2`、按 `shop_id`），并向客服端推送 `type=read` 已读回执。
    pub async fn set_read_as_user(&self, dto: &ImSetReadDTO, user_id: i32) -> Result<bool> {
        if user_id == 0 {
            return Err(Error::Unauthorized("error.user.not.login".into()));
        }
        if let Some(shop_id) = dto.shop_id {
            self.repo().mark_read_for_user(user_id, shop_id).await?;
            // Java `sendReadToClient`：通知客服「会员已读」
            let payload = read_receipt_payload(Some(shop_id), dto.conversation_id, user_id);
            let servant_id = match dto.conversation_id {
                Some(cid) => self
                    .repos
                    .im
                    .im_conversation
                    .find_by_id(cid)
                    .await?
                    .and_then(|c| c.last_servant_id)
                    .filter(|id| *id > 0),
                None => None,
            };
            match servant_id {
                Some(id) => {
                    nextrade_security::ws_registry::send_to_user(
                        &nextrade_security::ws_registry::user_key(true, &id.to_string()),
                        &payload,
                    );
                }
                None => {
                    nextrade_security::ws_registry::send_to_admins(&payload);
                }
            }
        }
        Ok(true)
    }

    // ————————————————————— 上传视频（未迁移） —————————————————————

    /// 上传视频（对应 adminapi `ImMessageController.uploadVideo` →
    /// `GalleryVideoInfoService.uploadVideo`）。
    ///
    /// 本地存储落盘并返回 URL/文件名/格式；首帧封面依赖 FFmpeg，
    /// 当前环境未安装时返回空封面（不影响上传与消息发送）。
    ///
    /// # Errors
    /// - 文件为空 / 文件名为空 → `Error::Validation`；
    /// - 存储写入失败 → `Error::Database`。
    pub async fn upload_video(
        &self,
        file_name: Option<&str>,
        bytes: &[u8],
    ) -> Result<UploadVideoInfoVO> {
        require_upload_file(file_name, bytes)?;
        let original = file_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("video.mp4");
        let url = nextrade_infrastructure::storage::save_local_upload(
            "im", original, bytes,
        )
        .await?;
        let path = std::path::Path::new(original);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video")
            .to_string();
        let format = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mp4")
            .to_string();
        Ok(UploadVideoInfoVO {
            video_url: url,
            video_name: stem,
            format,
            video_cover: String::new(),
        })
    }

    // ————————————————————— 辅助 —————————————————————

    /// 会员默认头像配置（`config.biz_code='defaultAvatar'`）。缺行回退空串。
    async fn default_avatar(&self) -> Result<String> {
        Ok(self.repo().default_avatar_config().await?.unwrap_or_default())
    }
}

/// 已读回执 WS 载荷（与前端 `onMessage` 的 `type=read` 协议一致）。
#[must_use]
pub fn read_receipt_payload(
    shop_id: Option<i32>,
    conversation_id: Option<i32>,
    user_id: i32,
) -> String {
    serde_json::json!({
        "type": "read",
        "data": [{
            "shopId": shop_id,
            "conversationId": conversation_id,
            "userId": user_id,
        }],
    })
    .to_string()
}

// ————————————————————— 纯逻辑辅助（可单测） —————————————————————

/// 列表 `ORDER BY` 片段（对应 Java `buildSortOrder`）。
///
/// 仅允许 `sendTime`/`send_time` 与主键 `id`，其余（含非法值）回落主键；
/// 默认降序（`sortOrder` 仅 `asc` 生效）。前导一个空格，供 `format!` 拼接。
#[must_use]
pub fn list_order_by(sort_field: Option<&str>, sort_order: Option<&str>) -> String {
    let column = match sort_field {
        Some("sendTime") | Some("send_time") => "send_time",
        _ => "\"id\"",
    };
    let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
        "ASC"
    } else {
        "DESC"
    };
    format!("{column} {dir}")
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`）。
#[must_use]
pub fn handel_time(secs: Option<i32>) -> String {
    match secs {
        None | Some(0) => String::new(),
        Some(v) => {
            use chrono::LocalResult;
            use chrono::TimeZone as _; // `timestamp_opt` 是 TimeZone trait 方法
            match chrono::Local.timestamp_opt(i64::from(v), 0) {
                LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                _ => String::new(),
            }
        }
    }
}

/// 当前秒级时间戳（Java `StringUtils.getCurrentTime`）。
#[must_use]
fn now_seconds() -> i32 {
    chrono::Utc::now().timestamp() as i32
}

/// 由消息行填充 VO 公共字段（`user` / `servant` 留空，由装配补齐）。
fn message_core(row: &ImMessage) -> ImMessageListVO {
    ImMessageListVO {
        message_type_text: message_type_text(row.message_type.as_deref()).map(ToString::to_string),
        id: Some(row.id),
        conversation_id: row.conversation_id,
        content: ContentVO::parse(row.content.as_deref()),
        message_type: row.message_type.clone(),
        r#type: row.r#type,
        user_id: row.user_id,
        servant_id: row.servant_id,
        send_time: Some(handel_time(row.send_time)),
        status: row.status,
        extend: row.extend.clone(),
        push_status: row.push_status,
        is_read: row.is_read,
        shop_id: Some(row.shop_id),
        user_from: row.user_from.clone(),
        user: None,
        servant: None,
    }
}

/// 头像为空时用默认头像回填（Java `getDefaultAvatar`）。
fn apply_user_avatar(u: UserBrief, default_avatar: &str) -> MessageUserVO {
    let avatar = if u.avatar.trim().is_empty() {
        default_avatar.to_string()
    } else {
        u.avatar
    };
    MessageUserVO {
        user_id: Some(u.user_id),
        username: Some(u.username),
        nickname: u.nickname,
        avatar: Some(avatar),
    }
}

/// 用户简信息列表 → `user_id` 索引 Map（装配时回填默认头像）。
fn to_user_map(rows: Vec<UserBrief>, default_avatar: &str) -> HashMap<i32, MessageUserVO> {
    rows.into_iter()
        .map(|u| {
            let avatar = if u.avatar.trim().is_empty() {
                default_avatar.to_string()
            } else {
                u.avatar.clone()
            };
            (
                u.user_id,
                MessageUserVO {
                    user_id: Some(u.user_id),
                    username: Some(u.username),
                    nickname: u.nickname,
                    avatar: Some(avatar),
                },
            )
        })
        .collect()
}

/// 客服（管理员）简信息列表 → `admin_id` 索引 Map。
fn to_admin_map(rows: Vec<AdminBrief>) -> HashMap<i32, MessageServantVO> {
    rows.into_iter()
        .map(|a| {
            (
                a.admin_id,
                MessageServantVO {
                    admin_id: Some(a.admin_id),
                    username: Some(a.username),
                    avatar: a.avatar,
                },
            )
        })
        .collect()
}

/// 去重（保序，过滤 <=0），供 `IN (...)` 批量查询。
fn dedup(ids: &[i32]) -> Vec<i32> {
    let mut seen = HashSet::new();
    ids.iter().copied().filter(|v| *v > 0 && seen.insert(*v)).collect()
}

/// `i64` → `i32` 夹取（客服接待数）。
fn clamp_i32(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// 上传视频未迁移文案。

/// 校验上传文件非空（Java `MultipartFile.isEmpty()` + `UPLOAD_FILE_IS_NULL`）。
fn require_upload_file(file_name: Option<&str>, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Err(Error::Validation("上传文件不能为空".to_string()));
    }
    if file_name.is_none_or(|n| n.trim().is_empty()) {
        return Err(Error::Validation("上传文件名不能为空".to_string()));
    }
    Ok(())
}

/// 由 [`BasePageParams`](nextrade_core::models::im::im_message::BasePageParams) 口径的
/// 分页参数构造 `PageQuery`。
fn page_query(paging: &nextrade_core::models::im::im_message::BasePageParams) -> PageQuery {
    PageQuery {
        page: paging.page_no().max(1) as u64,
        page_size: paging.page_size().clamp(1, 200) as u64,
    }
}

/// 组装分页结果（`list` / `receivedList` 返回体）。
fn build_page(total: i64, list: Vec<ImMessageListVO>, paging: &nextrade_core::models::im::im_message::BasePageParams) -> PageResult<ImMessageListVO> {
    PageResult::new(total.unsigned_abs(), list, &page_query(paging))
}

/// 空分页（对齐 Java `new Page<>()`）。
fn empty_page(paging: &nextrade_core::models::im::im_message::BasePageParams) -> PageResult<ImMessageListVO> {
    PageResult::empty(&page_query(paging))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::im::im_message::BasePageParams;

    #[test]
    fn order_by_defaults_to_id_desc_like_java() {
        assert_eq!(list_order_by(None, None), "\"id\" DESC");
        assert_eq!(list_order_by(Some("id"), Some("asc")), "\"id\" ASC");
        assert_eq!(list_order_by(Some("sendTime"), Some("desc")), "send_time DESC");
        assert_eq!(list_order_by(Some("sendTime"), None), "send_time DESC");
        // 非法字段回落主键（防注入）
        assert_eq!(list_order_by(Some("password"), Some("asc")), "\"id\" ASC");
    }

    #[test]
    fn handel_time_zero_or_none_is_empty() {
        assert_eq!(handel_time(None), "");
        assert_eq!(handel_time(Some(0)), "");
        assert_eq!(handel_time(Some(1_700_000_000)).len(), 19);
    }

    #[test]
    fn dedup_filters_zero_and_duplicates() {
        assert_eq!(dedup(&[1, 1, 2, 0, 3, 2]), vec![1, 2, 3]);
        assert_eq!(dedup(&[]), Vec::<i32>::new());
    }

    #[test]
    fn clamp_i32_caps() {
        assert_eq!(clamp_i32(5), 5);
        assert_eq!(clamp_i32(i64::from(u32::MAX)), i32::MAX);
    }

    #[test]
    fn apply_user_avatar_falls_back_to_default() {
        let u = UserBrief {
            user_id: 7,
            username: "x".into(),
            nickname: None,
            avatar: "  ".into(),
        };
        let vo = apply_user_avatar(u, "../assets/avatar/1.jpeg");
        assert_eq!(vo.avatar.as_deref(), Some("../assets/avatar/1.jpeg"));
    }

    #[test]
    fn build_page_total_and_shape() {
        let paging = BasePageParams {
            page: Some(2),
            size: Some(10),
        };
        let p: PageResult<ImMessageListVO> = build_page(35, Vec::new(), &paging);
        assert_eq!(p.total, 35);
        assert_eq!(p.page, 2);
        assert_eq!(p.page_size, 10);
        assert_eq!(p.total_page, 4);
    }

    #[test]
    fn read_receipt_payload_matches_frontend_protocol() {
        let payload = read_receipt_payload(Some(3), Some(9), 7);
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["type"], "read");
        assert_eq!(value["data"][0]["shopId"], 3);
        assert_eq!(value["data"][0]["conversationId"], 9);
        assert_eq!(value["data"][0]["userId"], 7);
    }
}