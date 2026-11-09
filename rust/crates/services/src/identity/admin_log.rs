//! 管理员日志 域服务（identity::admin_log）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//! 详见 `rust/docs/prompts/identity::admin_log.md`。
//!
//! 对应 Java `AdminLogController.page`
//! （`java/nextrade-adminapi/src/main/java/com/nextrade/adminapi/controller/authority/AdminLogController.java:51`，
//! `@PreAuthorize("@pms.hasPermission('adminLogManage','EmployeeLogManagement')")`）
//! → `AdminLogServiceImpl#adminLogPage`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/authority/impl/AdminLogServiceImpl.java:90`）。
//!
//! ## 权限语义（与 Java 的已知缺口）
//!
//! Java 有**方法级** `@PreAuthorize`，要求当前账号具备
//! `adminLogManage` 或 `EmployeeLogManagement` 权限点。Rust 侧
//! `require_auth`（`api/admin/src/lib.rs::router` 统一套在 `/adminapi` 外层）
//! 只校验登录态，**权限矩阵尚未实现**（与 product_manage / 002 / 005 / 012 / 014 /
//! 018 / 027 同一已知缺口）：当前任何已登录管理员都能调用本接口。
//! 数据越权风险由下面已迁移的 **作用域过滤** 兜住一半：
//! 店铺 / 供应商账号只能看到自己 `scope_id` 的日志，
//! 但**平台账号（`admin`）可看全量**这一点与 Java 的
//! 「有权限点才能看」不同。
//!
//! ## 已迁移的作用域过滤（重要，容易漏）
//!
//! Java `applyScopeFilter` 读 `x-admin-type` 决定可见范围，
//! 且**在拿不到合法作用域 ID 时直接返回空页**（不是返回全量）：
//! `admin` → 全量；`shop` / `store` / `pickup` / `pos` →
//! `scope_type='shop' AND scope_id=x-shop-id`；`vendor` →
//! `scope_type='vendor' AND scope_id=x-vendor-id`；
//! 其它类型 / 缺失 / ID 非正 / ID 非数字（`NumberFormatException`）→ **空页**。
//! 详见 [`nextrade_core::models::identity::admin_log::Scope::resolve`]。

use std::sync::Arc;

use nextrade_core::models::identity::admin_log::{
    format_log_time, AdminLogDTO, AdminLogPageQuery, AdminLogRequestLogVO, Scope,
};
use nextrade_core::types::PageResult;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::identity::admin_log::{resolve_sort, AdminLogRow};
use nextrade_infrastructure::db::Repositories;

/// 管理员日志 域服务。
pub struct AdminLogService {
    repos: Arc<Repositories>,
}

impl AdminLogService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::identity::admin_log::AdminLogRepository {
        &self.repos.identity.admin_log
    }

    /// 管理员日志分页（对应 Java `AdminLogServiceImpl#adminLogPage`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(
        &self,
        q: &AdminLogPageQuery,
        scope: Scope,
    ) -> Result<PageResult<AdminLogDTO>> {
        let page = q.page_query();

        // Java：`if (!applyScopeFilter(queryWrapper)) return PageUtil.convertPage(page, List.of());`
        // —— 无合法作用域直接空页，连 COUNT 都不查。
        if scope == Scope::Nothing {
            return Ok(PageResult::empty(&page));
        }

        let sort = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());
        let (total, rows) = self.repo().paginate(q, &scope, &sort).await?;

        // Java：`if (CollUtil.isEmpty(records)) return PageUtil.convertPage(resultPage, List.of());`
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        // Java `getAdminLogListWithUsernames`
        let usernames = self.load_usernames(&rows).await?;
        let request_logs = self.load_request_logs(&rows).await?;

        let list = rows
            .into_iter()
            .map(|row| {
                let user_id = row.user_id;
                let request_log_id = row.request_log_id;
                AdminLogDTO {
                    log_id: row.log_id,
                    log_time: format_log_time(row.log_time),
                    user_id,
                    log_info: row.log_info,
                    ip_address: row.ip_address,
                    username: usernames.get(&user_id).cloned(),
                    request_log: request_log_id.and_then(|id| request_logs.get(&id).cloned()),
                }
            })
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    /// 写入操作审计日志（对应 Java AdminLogService#createByLogInfo）。
    pub async fn create_by_log_info(
        &self,
        admin_id: i32,
        log_info: impl AsRef<str>,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        self.repo()
            .insert_log(
                now,
                admin_id,
                "platform",
                0,
                log_info.as_ref(),
                "",
                None,
            )
            .await
    }

    /// `admin_id → username`（Java `selectBatchIds` + `toMap`）。
    async fn load_usernames(&self, rows: &[AdminLogRow]) -> Result<std::collections::HashMap<i32, String>> {
        let mut ids: Vec<i32> = Vec::with_capacity(rows.len());
        for row in rows {
            if !ids.contains(&row.user_id) {
                ids.push(row.user_id);
            }
        }
        let pairs = self.repo().usernames_by_admin_id(&ids).await?;
        Ok(pairs.into_iter().collect())
    }

    /// `request_log_id → 请求日志`（Java 在循环里逐条 `selectById`，
    /// 这里按去重后的 ID 批量查，结果集相同、往返更少）。
    async fn load_request_logs(
        &self,
        rows: &[AdminLogRow],
    ) -> Result<std::collections::HashMap<i32, AdminLogRequestLogVO>> {
        let mut ids: Vec<i32> = Vec::new();
        for row in rows {
            if let Some(id) = row.request_log_id {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        let logs = self.repo().request_logs_by_ids(&ids).await?;
        Ok(logs.into_iter().map(|l| (l.request_log_id, l)).collect())
    }

    /// 记录管理员操作日志（对应 Java `AdminLogService#createByLogInfo` / `save`）。
    pub async fn record_log(
        &self,
        user_id: i32,
        scope_type: &str,
        scope_id: i32,
        log_info: &str,
        ip_address: Option<&str>,
        request_log_id: Option<i32>,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let ip = ip_address.unwrap_or("unknown");
        self.repo()
            .insert_log(now, user_id, scope_type, scope_id, log_info, ip, request_log_id)
            .await
    }

    /// 记录当前请求的操作日志：actor/IP 取自 [`nextrade_security::audit::current`]。
    ///
    /// 不在请求上下文（如定时任务/直接调用）时退化为 `user_id=0, ip=unknown`。
    pub async fn record_current_log(
        &self,
        scope_type: &str,
        scope_id: i32,
        log_info: &str,
    ) -> Result<i32> {
        let actor = nextrade_security::audit::current();
        let admin_id = actor.as_ref().map(|a| a.admin_id).unwrap_or(0);
        let ip = actor.map(|a| a.ip);
        self.record_log(admin_id, scope_type, scope_id, log_info, ip.as_deref(), None)
            .await
    }

    /// 根据日志文本记录操作日志（对应 Java `adminLogService.createByLogInfo(logInfo)`）。
    pub async fn create_scoped_log_info(
        &self,
        user_id: i32,
        scope_type: &str,
        scope_id: i32,
        log_info: &str,
    ) -> Result<i32> {
        self.record_log(user_id, scope_type, scope_id, log_info, None, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 作用域短路必须是「空页」而不是「全量」——这是本域最关键的越权防线。
    #[test]
    fn scope_nothing_maps_to_empty_page_semantics() {
        assert_eq!(Scope::resolve(Some("shop"), None, None), Scope::Nothing);
        assert_eq!(Scope::resolve(Some("vendor"), Some(2), None), Scope::Nothing);
        assert_eq!(Scope::resolve(Some("legacy"), Some(2), Some(2)), Scope::Nothing);
        assert_eq!(Scope::resolve(None, Some(2), Some(2)), Scope::Nothing);
    }

    /// Java `HeaderUtils.getShopId()` 只在**原始** `x-admin-type` 为
    /// `shop` / `pos` 时读 `x-shop-id`，其它类型返回 `null`。
    #[test]
    fn shop_id_resolution_matches_header_utils() {
        for (header, expected) in [
            (("shop", Some("9")), Some(9)),
            (("pos", Some("3")), Some(3)),
            (("shop", None), None),
            (("admin", Some("9")), None),
            (("vendor", Some("9")), None),
            // 非数字 → Java Integer.parseInt 抛 NumberFormatException → 空页
            (("shop", Some("abc")), None),
        ] {
            let (admin_type, raw) = header;
            let shop_id = raw.and_then(|v| v.trim().parse::<i32>().ok());
            let scope = if admin_type == "admin" {
                Scope::All
            } else {
                Scope::resolve(Some(admin_type), shop_id, None)
            };
            match expected {
                Some(id) => assert_eq!(scope, Scope::Shop { shop_id: id }, "{admin_type}/{raw:?}"),
                None if admin_type == "admin" => {
                    assert_eq!(scope, Scope::All, "{admin_type}/{raw:?}")
                }
                None => assert_eq!(scope, Scope::Nothing, "{admin_type}/{raw:?}"),
            }
        }
    }
}