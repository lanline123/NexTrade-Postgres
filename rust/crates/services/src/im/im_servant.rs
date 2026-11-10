//! IM 客服（ImServantController） 域服务（im::im_servant）。
//!
//! 对应原 Java `com.nextrade.service.im.impl.ImServantServiceImpl` 的
//! `list` / `modifyStatus` / `getStatus` 三个方法。构造签名固定
//! `pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! **多租户 / 登录态口径**（在路由层解析后传入，本层不做请求头解析）：
//! - `servant_id` ← Java `SecurityUtils.getCurrentAdminId()`（JWT 的 `admin_id`）；
//! - `shop_id` ← Java `HeaderUtils.getShopId()` 三态（平台 `0` / 店铺
//!   `x-shop-id` / 未知 `null`），以 `Option<i32>`（`None` == Java `null`）传入。
//!
//! **未迁移 / 已简化**（详见汇报 `uncertain_points`）：
//! - Java 的 `detail` / `create` / `update`（`ImServantServiceImpl` 继承
//!   `BaseServiceImpl` 附带）不在本切片契约内，未实现；
//! - Java `list` 在「用户名无匹配」或「分页结果为空」时 `return new Page<>()`
//!   （默认 size=10/current=1/total=0）；Rust 统一用项目 `PageResult`，
//!   空结果同样返回 `total=0` + 空 `list`，但保留**请求页码/页长**而非 Java
//!   的 `new Page<>()` 默认值（差异已在汇报说明）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::ImServant;
use nextrade_core::models::im::im_servant::{
    ImServantListQuery, ImServantPage, ImServantScope, ImServantUserVO, ImServantVO,
    DEFAULT_OFFLINE_STATUS,
};
use nextrade_core::types::PageResult;
use nextrade_infrastructure::db::repo::im::im_servant::{resolve_sort_column, resolve_sort_dir};
use nextrade_infrastructure::db::Repositories;
use nextrade_core::Result;

/// IM 客服（ImServantController） 域服务。
pub struct ImServantService {
    repos: Arc<Repositories>,
}

impl ImServantService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::im::im_servant::ImServantRepository {
        &self.repos.im.im_servant
    }

    /// `GET /im/servant/servant/transferList`：客服分页列表。
    ///
    /// 对应 Java `ImServantServiceImpl#list`：
    /// - `shop_id`（请求头覆盖）非空时 `shop_id = ?` 过滤；
    /// - `username` 非空时先在 `admin_user` 里 `ILIKE %username%` 取 `admin_id`，
    ///   命中为空 → 直接返回空页（对应 Java 提前 `return new Page<>()`），
    ///   否则 `servant_id IN (命中id)` 过滤；
    /// - 排序走白名单（缺省主键 `id`，缺省方向 `desc`）；
    /// - 结果批量关联 `admin_user`，VO 回填 `user`（无匹配 → `null`），
    ///   时间字段格式化为字符串。
    pub async fn list(
        &self,
        q: &ImServantListQuery,
        scope: ImServantScope,
    ) -> Result<PageResult<ImServantVO>> {
        let shop_id = scope.shop_id();
        let page = ImServantPage::from_query(q.page, q.size);
        let (limit, offset) = page.limit_offset();
        let sort_column = resolve_sort_column(q.sort_field.as_deref());
        let dir = resolve_sort_dir(q.sort_order.as_deref());

        // username 过滤：非空 → 取命中的 admin_id 列表（可能为空 → 空页）。
        let username = q.username.as_deref().filter(|s| !s.is_empty());
        let matched_ids: Option<Vec<i32>> = match username {
            Some(u) => Some(self.repo().find_admin_ids_by_username(u).await?),
            None => None,
        };

        let (total, rows) = self
            .repo()
            .paginate(shop_id, matched_ids.as_deref(), sort_column, dir, limit, offset)
            .await?;

        // Java：records 为空 → new Page<>()（total 0）。此处保留请求页码/页长。
        if rows.is_empty() {
            return Ok(PageResult {
                total: 0,
                list: Vec::new(),
                total_page: 0,
                page_size: page.size,
                page: page.page,
            });
        }

        // 批量取用户信息（servantId 去重，对应 Java `.distinct()`）。
        let mut servant_ids: Vec<i32> = rows.iter().filter_map(|r| r.servant_id).collect();
        servant_ids.sort_unstable();
        servant_ids.dedup();
        let user_map: HashMap<i32, ImServantUserVO> = self
            .repo()
            .find_users_by_ids(&servant_ids)
            .await?
            .into_iter()
            .map(|(admin_id, username, avatar)| {
                (
                    admin_id,
                    ImServantUserVO {
                        admin_id: Some(admin_id),
                        username: Some(username),
                        avatar,
                    },
                )
            })
            .collect();

        let list = rows
            .iter()
            .map(|r| ImServantVO {
                id: Some(r.id),
                servant_id: r.servant_id,
                status: r.status,
                shop_id: r.shop_id,
                conversation_num: r.conversation_num.map(i64::from),
                add_time: Some(nextrade_core::models::im::im_servant::handel_time(r.add_time)),
                last_update_time: Some(nextrade_core::models::im::im_servant::handel_time(
                    r.last_update_time,
                )),
                // Java：servantMap.get(servantId) == null → userVO = null
                user: r.servant_id.and_then(|sid| user_map.get(&sid).cloned()),
            })
            .collect();

        Ok(PageResult {
            total,
            list,
            total_page: ImServantPage::total_page(total, page.size),
            page_size: page.size,
            page: page.page,
        })
    }

    /// `POST /im/servant/servant/modifyStatus`：修改当前客服在线状态（upsert）。
    ///
    /// 对应 Java `ImServantServiceImpl#modifyStatus`：
    /// - 取行条件**恒定**为 `servant_id = ? AND shop_id = ?`（`shop_id` 为
    ///   `NULL` 时 MySQL `= NULL` 恒不成立 → 走 INSERT，与 Java 一致）；
    /// - 命中 → `UPDATE` 状态 / `shop_id` / 最后更新时间；
    /// - 未命中 → `INSERT`（add_time、last_update_time 均置当前时间）。
    ///
    /// Java 原接口返回 `void`；此处回传落库主键 `id` 便于排查。
    pub async fn modify_status(
        &self,
        status: Option<i32>,
        servant_id: i32,
        scope: ImServantScope,
    ) -> Result<i32> {
        let shop_id = scope.shop_id();
        let now = current_seconds();
        match self.repo().find_one_for_modify(servant_id, shop_id).await? {
            Some(row) => {
                self.repo().update_status(row.id, status, shop_id, now).await?;
                Ok(row.id)
            }
            None => {
                self.repo()
                    .insert_servant(Some(servant_id), shop_id, status, now)
                    .await
            }
        }
    }

    /// `GET /im/servant/servant/getStatus`：读取当前客服状态（不存在则初始化为离线）。
    ///
    /// 对应 Java `ImServantServiceImpl#getStatus`：
    /// - 取行条件 `servant_id = ?`，`shop_id` 非空时才追加 `AND shop_id = ?`；
    /// - 未命中 → 新建并落库，默认状态 `3`（离线），add/last 更新时间为当前；
    /// - 返回 VO **仅**回填 `servantId` / `status` / `lastUpdateTime` / `shopId`
    ///   （Java 只 set 这四个字段，`id`/`addTime`/`conversationNum`/`user` 均为 null）。
    pub async fn get_status(&self, servant_id: i32, scope: ImServantScope) -> Result<ImServantVO> {
        let shop_id = scope.shop_id();
        let now = current_seconds();
        let row = match self.repo().find_one_for_status(servant_id, shop_id).await? {
            Some(r) => r,
            None => {
                let new_id = self
                    .repo()
                    .insert_servant(Some(servant_id), shop_id, Some(DEFAULT_OFFLINE_STATUS), now)
                    .await?;
                // Java 用新建的内存对象构建 VO（status 固定为默认 3）
                ImServant {
                    id: new_id,
                    servant_id: Some(servant_id),
                    last_update_time: Some(now),
                    add_time: Some(now),
                    status: Some(DEFAULT_OFFLINE_STATUS),
                    shop_id,
                    conversation_num: None,
                }
            }
        };

        Ok(ImServantVO {
            id: None,
            servant_id: row.servant_id,
            status: row.status,
            shop_id: row.shop_id,
            conversation_num: None,
            add_time: None,
            last_update_time: Some(nextrade_core::models::im::im_servant::handel_time(
                row.last_update_time,
            )),
            user: None,
        })
    }
}

/// 当前秒级时间戳（对应 Java `StringUtils.getCurrentTime()`
/// = `System.currentTimeMillis() / 1000`）。
fn current_seconds() -> i32 {
    chrono::Utc::now().timestamp() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::im::im_servant::{ImServantScope, ImServantPage};

    #[test]
    fn scope_none_means_unscoped() {
        // Java HeaderUtils.getShopId() 返回 null → Unscoped → shop_id() == None
        assert_eq!(ImServantScope::Unscoped.shop_id(), None);
        assert_eq!(ImServantScope::Shop(0).shop_id(), Some(0));
    }

    #[test]
    fn page_clamps_like_java() {
        assert_eq!(ImServantPage::from_query(None, None), ImServantPage { page: 1, size: 15 });
        assert_eq!(ImServantPage::from_query(Some(2), Some(1000)).size, 100);
    }

    #[test]
    fn default_offline_status_is_three() {
        // Java getStatus 新建记录默认 setStatus(3)（注释「默认离线」）
        assert_eq!(DEFAULT_OFFLINE_STATUS, 3);
    }
}