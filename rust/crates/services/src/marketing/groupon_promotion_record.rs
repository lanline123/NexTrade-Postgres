//! 拼团活动记录（管理端）域服务（marketing::groupon_promotion_record）。
//!
//! Java `GrouponPromotionRecordController`（5 端点）→ `GrouponPromotionRecordServiceImpl`。
//!
//! # 偏差
//!
//! 1. `manualGroup`/`batchGroup` 只做本地 DB 更新（开团记录+同团成员置「拼团成功」，
//!    并按差额生成虚拟团员记录）；Java 还会
//!    `writeGrouponStatementOutput`（分账输出）、
//!    `orderService.createDeliveryOrderIfNeeded`（派送订单）与虚拟自动发货，
//!    这些依赖订单/分账域，未迁移 ⇒ 跳过并在本模块记录。
//! 2. 虚拟团员：Java `getVirtualUser` 读配置生成头像/昵称；Rust 用
//!    「虚拟用户N」+ 空头像（配置域未迁移）。
//! 3. `orderList.addTime`：Java mapper 将 BIGINT 映射为 String（数字串）；
//!    Rust 直接格式化为 `yyyy-MM-dd HH:mm:ss`（前端展示口径）。
//! 4. `orderList` 的 keyword：Java XML 的 OR 未加括号（优先级缺陷），Rust 用括号
//!    保证「商品名 OR 用户名 OR 订单号」整体生效。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::marketing::groupon_promotion_record::{
    format_time, join_status_desc, order_status_name, GroupMemberQuery, GrouponRecordOpenJoinMemberVO,
    GrouponRecordOpenJonPageVO, GrouponRecordOrderPageVO, ManualGroupQuery, OpenJoinListQuery,
    OrderListQuery, MSG_NOT_EXISTS_OR_FAILED, SELF_OPERATED_SHOP_NAME,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::groupon_promotion_record::GrouponPromotionRecordRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 拼团活动记录（管理端）域服务。
pub struct GrouponPromotionRecordService {
    repos: Arc<Repositories>,
}

impl GrouponPromotionRecordService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &GrouponPromotionRecordRepository {
        &self.repos.marketing.groupon_promotion_record
    }

    /// 开团记录（Java `openJoinList`）。
    pub async fn open_join_list(
        &self,
        q: &OpenJoinListQuery,
    ) -> Result<PageResult<GrouponRecordOpenJonPageVO>> {
        let page = to_page(q.page, q.size);
        let (limit, offset) = nextrade_core::models::marketing::groupon_promotion_record::limit_offset(q.page, q.size);
        let (total, rows) = self
            .repo()
            .open_join_list(q, TenantScope::current_list_filter(), limit, offset)
            .await?;
        let list = rows
            .iter()
            .map(|r| GrouponRecordOpenJonPageVO {
                groupon_promotion_record_id: r.groupon_promotion_record_id,
                open_group_time: r.open_group_time,
                product_name: r.product_name.clone(),
                join_status: r.join_status,
                group_captain_name: r.group_captain_name.clone(),
                join_expiration_time: r.join_expiration_time,
                min_group_num: r.min_group_num,
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 订单记录（Java `orderList`）。
    pub async fn order_list(
        &self,
        q: &OrderListQuery,
    ) -> Result<PageResult<GrouponRecordOrderPageVO>> {
        let page = to_page(q.page, q.size);
        let (limit, offset) = nextrade_core::models::marketing::groupon_promotion_record::limit_offset(q.page, q.size);
        let (total, rows) = self
            .repo()
            .order_list(q, TenantScope::current_list_filter(), limit, offset)
            .await?;
        let list = rows
            .iter()
            .map(|r| {
                let mut status_name = order_status_name(r.order_status);
                if r.join_status == 3 {
                    status_name = "拼团失败，待退款".to_string();
                }
                GrouponRecordOrderPageVO {
                    join_type: r.join_type,
                    group_captain_name: r.group_captain_name.clone(),
                    product_name: r.product_name.clone(),
                    order_sn: r.order_sn.clone(),
                    add_time: format_time(r.add_time),
                    join_status: r.join_status,
                    username: r.username.clone(),
                    order_status: r.order_status,
                    order_status_name: status_name,
                }
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 团员列表（Java `groupMemberList`）。
    pub async fn group_member_list(
        &self,
        q: &GroupMemberQuery,
    ) -> Result<Vec<GrouponRecordOpenJoinMemberVO>> {
        let Some(record_id) = q.groupon_promotion_record_id else {
            return Ok(Vec::new());
        };
        // 多租户：团员列表随开团记录归属校验
        if let Some(record) = self.repo().find_by_id(record_id).await? {
            TenantScope::current_ensure_access(record.shop_id)?;
        }
        let rows = self.repo().members(record_id).await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let user_ids: Vec<i64> = rows.iter().map(|r| r.user_id).collect();
        let users = self.repo().find_users(&dedup(user_ids)).await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            let shop_title = if r.shop_id == 0 {
                SELF_OPERATED_SHOP_NAME.to_string()
            } else {
                self.repo()
                    .find_shop_title(r.shop_id)
                    .await?
                    .unwrap_or_default()
            };
            let user = users.get(&r.user_id);
            list.push(GrouponRecordOpenJoinMemberVO {
                join_type: r.join_type,
                shop_id: r.shop_id,
                shop_title,
                add_time: r.add_time,
                username: user.map(|u| u.username.clone()),
                avatar: user.map(|u| u.avatar.clone()),
                user_id: r.user_id,
            });
        }
        Ok(list)
    }

    /// 手动成团（Java `manualGroup(Long)`）。
    pub async fn manual_group(&self, q: &ManualGroupQuery) -> Result<()> {
        let Some(id) = q.groupon_promotion_record_id else {
            return Err(Error::biz(500, MSG_NOT_EXISTS_OR_FAILED));
        };
        self.manual_group_id(id).await
    }

    async fn manual_group_id(&self, id: i64) -> Result<()> {
        let record = self
            .repo()
            .find_by_id(id)
            .await?
            .filter(|r| r.join_status != 3)
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS_OR_FAILED))?;
        // 多租户：成团操作前校验归属
        TenantScope::current_ensure_access(record.shop_id)?;
        let member_count = self.repo().count_members(id).await?;
        let virtual_count = (i64::from(record.min_group_num) - member_count).max(0);
        let now = now_seconds();
        let mut tx = self.repo().begin().await?;
        self.repo().mark_success_in_tx(&mut tx, id).await?;
        for index in 0..usize::try_from(virtual_count).unwrap_or(0) {
            self.repo()
                .insert_virtual_in_tx(&mut tx, &record, index, now)
                .await?;
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("拼团手动成团: record_id={id}, virtual={virtual_count}");
        Ok(())
    }

    /// 批量手动成团（Java `batchManualGroup(List<Long>)`）。
    pub async fn batch_group(&self, ids: &[i64]) -> Result<()> {
        for id in ids {
            self.manual_group_id(*id).await?;
        }
        Ok(())
    }

    /// 拼团状态描述（供外部/测试使用）。
    #[must_use]
    pub fn status_desc(status: i32) -> &'static str {
        join_status_desc(status)
    }
}

fn dedup(ids: Vec<i64>) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

fn to_page(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = GrouponPromotionRecordRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

/// 列表店铺过滤：店铺账号强制本店；平台账号不过滤。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_helper() {
        assert_eq!(GrouponPromotionRecordService::status_desc(1), "待成团");
        assert_eq!(GrouponPromotionRecordService::status_desc(2), "拼团成功");
    }

    #[test]
    fn page_conversion() {
        let p = to_page(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page(Some(2), Some(10));
        assert_eq!((p.page, p.page_size), (2, 10));
    }

    #[test]
    fn message_and_dedup() {
        assert_eq!(MSG_NOT_EXISTS_OR_FAILED, "拼团记录不存在或已失败");
        assert_eq!(dedup(vec![2, 1, 2]), vec![2, 1]);
    }
}
