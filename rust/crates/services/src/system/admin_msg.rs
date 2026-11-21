//! 后台消息域服务（system::admin_msg）。
//!
//! 构造 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储访问为**嵌套**命名：
//! `self.repos.system.admin_msg.<method>()`。对应 Java `AdminMsgServiceImpl`。
//!
//! # 已实现
//! `list`（身份可见性 + msgType + title ILIKE + 排序）、`msgTypeArr`（分类树 + 未读累加）、
//! `homeSummary`（未读总数 + 最新一条）、`setReaded`、`setAllReaded`。
//! 平台公告（我方新增）：`publish_announcement`（主行 + 全店铺分发）、
//! `announcement_list`（主行 + 已读统计）、`delete_announcement`（主行 + 分发行全删），
//! 已读状态复用本域既有 `is_readed` 口径，不另建已读体系。
//!
//! # 已实现 / 简化
//! - `msgCount`：路由层已实现 SSE，周期性调用
//!   [`AdminMsgService::get_msg_count`] 推送未读数与业务指标。
//! - `broadcastList`：复用 `list` 的可见性与分页口径；订单/商品摘要聚合
//!   （`BroadcastProductVO`/金额汇总）未迁移，返回基础字段 + 空 products / 0 金额。
//! - `related_data` 的默认 route 补全（`applyDefaultRoute`）未迁移：仅透传已存在 route。
//! - 身份可见性来自 JWT `admin_type` + 查询参数 `shopId`/`vendorId` 兜底。

use std::sync::Arc;

use nextrade_core::models::system::admin_msg::{
    build_type_tree, format_time, AdminMsgBroadcastVO, AdminMsgCountVO, AdminMsgListQuery,
    AdminMsgTypeListVO, AdminMsgVO, AnnouncementPublishDTO, AnnouncementVO, HomeMsgLatestVO,
    HomeMsgSummaryVO, MsgScope, ANNOUNCE_MSG_TYPE, SERVICE_DATA_ERROR,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::admin_msg::{AdminMsgRepository, AdminMsgRow};
use nextrade_infrastructure::db::Repositories;

/// 后台消息域服务。
pub struct AdminMsgService {
    repos: Arc<Repositories>,
}

impl AdminMsgService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &AdminMsgRepository {
        &self.repos.system.admin_msg
    }

    /// 查询管理员消息与业务指标统计（对齐 Java SseServiceImpl#msgCount）。
    pub async fn get_msg_count(
        &self,
        admin_id: i32,
        scope: MsgScope,
        start_time: i64,
    ) -> Result<AdminMsgCountVO> {
        self.repo().get_msg_count_vo(admin_id, scope, start_time).await
    }

    /// `GET /adminapi/msg/adminMsg/msgCount` 占位兼容。
    pub fn msg_count(&self) -> Result<()> {
        Ok(())
    }

    /// `GET /adminapi/msg/adminMsg/msgTypeArr`：分类树 + 未读数。
    pub async fn msg_type_arr(&self, scope: MsgScope) -> Result<Vec<AdminMsgTypeListVO>> {
        let rows = self.repo().unread_by_type(scope).await?;
        let unread: std::collections::HashMap<i32, i32> = rows
            .into_iter()
            .map(|(t, n)| (t, i32::try_from(n).unwrap_or(0)))
            .collect();
        Ok(build_type_tree(scope.is_vendor(), &unread))
    }

    /// `GET /adminapi/msg/adminMsg/list`：消息分页。
    pub async fn list(
        &self,
        scope: MsgScope,
        q: &AdminMsgListQuery,
    ) -> Result<PageResult<AdminMsgVO>> {
        let (total, rows) = self.repo().paginate(scope, q).await?;
        let (page, size) = q.page_params();
        let list = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// `GET /adminapi/msg/adminMsg/broadcastList`（商品摘要未迁移）。
    pub async fn broadcast_list(
        &self,
        scope: MsgScope,
        q: &AdminMsgListQuery,
    ) -> Result<PageResult<AdminMsgBroadcastVO>> {
        let (total, rows) = self.repo().paginate(scope, q).await?;
        let (page, size) = q.page_params();
        let list = rows
            .iter()
            .map(|r| AdminMsgBroadcastVO {
                msg_id: r.msg_id,
                msg_type: r.msg_type,
                msg_type_name: nextrade_core::models::system::admin_msg::msg_type_name(r.msg_type)
                    .to_string(),
                send_time: format_time(Some(r.send_time)),
                title: r.title.clone(),
                content: r.content.clone(),
                msg_link: r.msg_link.clone(),
                products: Vec::new(),
                total_amount: "0".to_string(),
                refund_amount: "0".to_string(),
            })
            .collect();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// `GET /adminapi/msg/adminMsg/homeSummary`：未读总数 + 最新一条。
    pub async fn home_summary(&self, scope: MsgScope) -> Result<HomeMsgSummaryVO> {
        let tree = self.msg_type_arr(scope).await?;
        let unread_count = tree.iter().map(|t| t.unread_count).sum();
        let latest = self.repo().latest(scope).await?.map(|r| HomeMsgLatestVO {
            msg_id: r.msg_id,
            msg_type: r.msg_type,
            type_name: nextrade_core::models::system::admin_msg::msg_type_name(r.msg_type)
                .to_string(),
            title: r.title.clone(),
            send_time: Some(format_time(Some(r.send_time))),
        });
        Ok(HomeMsgSummaryVO {
            unread_count,
            latest_message: latest,
        })
    }

    /// `POST /adminapi/msg/adminMsg/setReaded`：单条已读。
    pub async fn set_readed(&self, msg_id: Option<i32>, scope: MsgScope) -> Result<bool> {
        let msg_id = msg_id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "msgId不能为空"))?;
        let row = self
            .repo()
            .find_by_id(msg_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "消息不存在"))?;
        if !message_visible_in(&row, scope) {
            return Err(Error::Forbidden("无权操作该消息".into()));
        }
        self.repo().set_readed(msg_id).await
    }

    /// `POST /adminapi/msg/adminMsg/setAllReaded`：全部已读。
    pub async fn set_all_readed(&self, scope: MsgScope) -> Result<u64> {
        self.repo().set_all_readed(scope).await
    }

    /// 发布平台公告（我方新增，论坛帖 587859 余子项；路由层限平台管理员）。
    ///
    /// 落一行平台主行（广播行，平台消息中心可见）+ 向全部未删除店铺各分发
    /// 一行（`related_data.announcementId` 回指主行）。每家店铺的分发行有
    /// 独立 `is_readed`，已读/未读提醒完全复用消息中心既有口径
    /// （`msgTypeArr` 分类树未读累加、SSE 未读总数、铃铛下拉），不另建已读表。
    pub async fn publish_announcement(
        &self,
        admin_id: i32,
        dto: &AnnouncementPublishDTO,
    ) -> Result<AnnouncementVO> {
        let title = dto
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "公告标题不能为空"))?;
        if title.chars().count() > 100 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "公告标题不能超过 100 字"));
        }
        let content = dto
            .content
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "公告内容不能为空"))?;
        if content.chars().count() > 2000 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "公告内容不能超过 2000 字"));
        }
        let master_related =
            serde_json::json!({ "route": "platformAnnouncement" }).to_string();
        let master_id = self
            .repo()
            .insert_announcement(admin_id, 0, 1, title, content, &master_related)
            .await?;
        let shop_ids = self.repo().all_shop_ids().await?;
        let copy_related = serde_json::json!({
            "announcementId": master_id,
            "route": "platformAnnouncement",
        })
        .to_string();
        for shop_id in &shop_ids {
            self.repo()
                .insert_announcement(admin_id, *shop_id, 0, title, content, &copy_related)
                .await?;
        }
        let row = self
            .repo()
            .find_by_id(master_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "公告发布失败"))?;
        Ok(AnnouncementVO {
            msg_id: master_id,
            title: row.title.clone(),
            content: row.content.clone(),
            send_time: format_time(Some(row.send_time)),
            admin_id: row.admin_id,
            delivered_count: i64::try_from(shop_ids.len()).unwrap_or(0),
            read_count: 0,
        })
    }

    /// 平台公告列表（平台视角）：公告主行分页 + 逐条分发/已读统计。
    pub async fn announcement_list(
        &self,
        page: u64,
        size: u64,
    ) -> Result<PageResult<AnnouncementVO>> {
        let q = AdminMsgListQuery {
            msg_type: Some(ANNOUNCE_MSG_TYPE),
            page: Some(page),
            size: Some(size),
            ..Default::default()
        };
        let scope = MsgScope::Shop {
            shop_id: 0,
            platform: true,
        };
        let (total, rows) = self.repo().paginate(scope, &q).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let (delivered, read) = self.repo().announcement_read_stats(r.msg_id).await?;
            list.push(AnnouncementVO {
                msg_id: r.msg_id,
                title: r.title.clone(),
                content: r.content.clone(),
                send_time: format_time(Some(r.send_time)),
                admin_id: r.admin_id,
                delivered_count: delivered,
                read_count: read,
            });
        }
        let (page, size) = q.page_params();
        Ok(PageResult::new(
            total,
            list,
            &PageQuery {
                page,
                page_size: size,
            },
        ))
    }

    /// 删除平台公告（主行 + 全部分发行）；目标不是公告主行时按不存在处理。
    pub async fn delete_announcement(&self, msg_id: Option<i32>) -> Result<u64> {
        let msg_id = msg_id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "msgId不能为空"))?;
        let row = self
            .repo()
            .find_by_id(msg_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "公告不存在"))?;
        if row.msg_type != ANNOUNCE_MSG_TYPE || row.is_broadcast != 1 || row.shop_id != 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, "公告不存在"));
        }
        self.repo().delete_announcement(msg_id).await
    }
}

/// 行 → 列表 VO（Java `BeanUtils.copyProperties` + 类型名/route/时间）。
fn to_vo(row: &AdminMsgRow) -> AdminMsgVO {
    let related_data: Option<serde_json::Value> = row
        .related_data
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let route = related_data
        .as_ref()
        .and_then(|v| v.get("route"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    AdminMsgVO {
        msg_id: row.msg_id,
        msg_type: row.msg_type,
        msg_type_name: nextrade_core::models::system::admin_msg::msg_type_name(row.msg_type)
            .to_string(),
        route,
        send_time: format_time(Some(row.send_time)),
        is_readed: row.is_readed,
        shop_id: row.shop_id,
        admin_id: row.admin_id,
        order_id: row.order_id,
        product_id: row.product_id,
        title: row.title.clone(),
        content: row.content.clone(),
        msg_link: row.msg_link.clone(),
        related_data,
    }
}

/// 消息归属校验（严格租户）：
/// - 
    #[test]
    fn message_visibility_is_tenant_strict() {
        use nextrade_infrastructure::db::repo::system::admin_msg::AdminMsgRow;
        let shop_row = AdminMsgRow {
            msg_id: 1,
            msg_type: 1,
            send_time: 0,
            is_readed: 0,
            shop_id: 5,
            admin_id: 0,
            order_id: 0,
            product_id: 0,
            title: String::new(),
            content: String::new(),
            msg_link: None,
            related_data: None,
            is_broadcast: 0,
            vendor_id: None,
        };
        assert!(message_visible_in(
            &shop_row,
            MsgScope::Shop { shop_id: 5, platform: false }
        ));
        assert!(!message_visible_in(
            &shop_row,
            MsgScope::Shop { shop_id: 6, platform: false }
        ));
        assert!(!message_visible_in(
            &shop_row,
            MsgScope::Shop { shop_id: 0, platform: true }
        ));
        assert!(!message_visible_in(
            &shop_row,
            MsgScope::Vendor { vendor_id: 9 }
        ));

        let mut vendor_row = shop_row.clone();
        vendor_row.vendor_id = Some(9);
        assert!(message_visible_in(
            &vendor_row,
            MsgScope::Vendor { vendor_id: 9 }
        ));
        assert!(!message_visible_in(
            &vendor_row,
            MsgScope::Vendor { vendor_id: 8 }
        ));

        let mut broadcast = shop_row.clone();
        broadcast.is_broadcast = 1;
        assert!(message_visible_in(
            &broadcast,
            MsgScope::Shop { shop_id: 0, platform: true }
        ));
    }
/// 消息归属校验（严格租户）。
///
/// - 供应商：`vendor_id` 必须等于可信 vendor scope；
/// - 平台：仅广播消息（`is_broadcast = 1`）；
/// - 店铺/门店：`shop_id` 必须等于本店且非供应商消息。
#[must_use]
pub fn message_visible_in(
    row: &nextrade_infrastructure::db::repo::system::admin_msg::AdminMsgRow,
    scope: MsgScope,
) -> bool {
    match scope {
        MsgScope::Vendor { vendor_id } => row.vendor_id == Some(vendor_id),
        MsgScope::Shop { shop_id, platform } => {
            if platform {
                row.is_broadcast == 1
            } else {
                row.shop_id == shop_id && row.vendor_id.unwrap_or(0) == 0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::system::admin_msg::{msg_type_name, MsgScope};

    #[test]
    fn scope_flags_match_identity_model() {
        assert!(MsgScope::Shop { shop_id: 0, platform: true }.is_platform());
        assert!(MsgScope::Vendor { vendor_id: 1 }.is_vendor());
        assert!(!MsgScope::Shop { shop_id: 1, platform: false }.is_vendor());
    }

    #[test]
    fn msg_type_names_are_java_verbatim() {
        assert_eq!(msg_type_name(11), "新订单");
        assert_eq!(msg_type_name(44), "保证金预警");
    }
}
