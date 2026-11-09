//! 用户余额日志 域服务（payment::user_balance_log）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//! 详见 `rust/docs/prompts/payment::user_balance_log.md`。
//!
//! 对应 Java `UserBalanceLogController.list`
//! （`java/nextrade-adminapi/src/main/java/com/nextrade/adminapi/controller/finance/UserBalanceLogController.java:38`）
//! → `UserBalanceLogServiceImpl#list`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/finance/impl/UserBalanceLogServiceImpl.java:62`）。
//!
//! ## 口径说明
//!
//! - **分页信封**：Java 返回 MyBatis-Plus 的 `Page<UserBalanceLogVO>`
//!   （`records/total/size/current/pages`），本项目统一使用
//!   [`PageResult`]（`list/total/totalPage/pageSize/page`），与其余已迁移
//!   切片一致；
//! - **空页**：Java 在「当前页无记录」时 `return new Page<>()`
//!   （`total = 0`、`records = []`，**丢弃**排序与真实 total），
//!   此处用 [`PageResult::empty`] 复刻（见 shop_client / buyer_show_client / buyer_show_group 同口径）；
//! - **i18n 未迁移**：`changeTypeName` 走 Java 的中文兜底分支，
//!   `changeDesc` 上的 `@JsonTranslate(dataType = 0)` 翻译注解未迁移，
//!   原样返回库中文（与 user_manage 一致）；
//! - **权限**：Java 该控制器**没有** `@PreAuthorize`（契约 `auth=jwt`），
//!   只要登录即可访问，Rust 侧由 `require_auth` 覆盖，无需额外处理。

use std::sync::Arc;

use nextrade_core::models::payment::user_balance_log::{
    format_change_time, resolve_change_type_name, UserBalanceLogListQuery, UserBalanceLogVO,
};
use nextrade_core::Result;
use nextrade_core::types::PageResult;
use nextrade_infrastructure::db::repo::payment::user_balance_log::{resolve_sort, UserBalanceLogRow};
use nextrade_infrastructure::db::Repositories;

/// 用户余额日志 域服务。
pub struct UserBalanceLogService {
    repos: Arc<Repositories>,
}

impl UserBalanceLogService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::payment::user_balance_log::UserBalanceLogRepository {
        &self.repos.payment.user_balance_log
    }

    /// 余额日志分页列表（对应 Java `UserBalanceLogServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(&self, q: &UserBalanceLogListQuery) -> Result<PageResult<UserBalanceLogVO>> {
        let page = q.page_query();
        let sort = resolve_sort(q.sort_field.as_deref(), q.sort_order.as_deref());

        let (total, rows) = self.repo().paginate(q, &sort).await?;
        if rows.is_empty() {
            // Java：`if (CollUtil.isEmpty(records)) return new Page<>();`
            return Ok(PageResult::empty(&page));
        }

        let list = rows.into_iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }
}

/// 行 → VO（对应 Java 的 `BeanUtils.copyProperties` +
/// `setChangeTime(handelTime)` + `setChangeTypeName(resolveChangeTypeName)`）。
#[must_use]
pub fn to_vo(row: UserBalanceLogRow) -> UserBalanceLogVO {
    UserBalanceLogVO {
        log_id: row.log_id,
        user_id: row.user_id,
        username: row.username,
        balance: row.balance,
        before_balance: row.before_balance,
        after_balance: row.after_balance,
        frozen_balance: row.frozen_balance,
        before_frozen_balance: row.before_frozen_balance,
        after_frozen_balance: row.after_frozen_balance,
        change_time: format_change_time(row.change_time),
        change_desc: row.change_desc,
        change_type: row.change_type,
        change_type_name: resolve_change_type_name(Some(row.change_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn row(change_time: i32, change_type: i32, username: Option<&str>) -> UserBalanceLogRow {
        UserBalanceLogRow {
            log_id: 11,
            user_id: 3,
            before_balance: Decimal::new(1000, 2),
            balance: Decimal::new(500, 2),
            after_balance: Decimal::new(1500, 2),
            frozen_balance: Decimal::ZERO,
            before_frozen_balance: Decimal::ZERO,
            after_frozen_balance: Decimal::ZERO,
            change_time,
            change_desc: "余额充值".into(),
            change_type,
            username: username.map(str::to_string),
        }
    }

    #[test]
    fn vo_copies_amounts_and_derives_two_fields() {
        let vo = to_vo(row(1_733_000_000, 1, Some("bob")));
        // 直接拷自实体的金额
        assert_eq!(vo.balance, Decimal::new(500, 2));
        assert_eq!(vo.before_balance, Decimal::new(1000, 2));
        assert_eq!(vo.after_balance, Decimal::new(1500, 2));
        // 派生字段
        assert_eq!(vo.change_type_name, "增加");
        assert_eq!(vo.change_time.len(), 19);
        assert_eq!(vo.username.as_deref(), Some("bob"));
    }

    #[test]
    fn zero_change_time_becomes_empty_string_like_java() {
        // Java `TigUtils.handelTime`：0 → ""
        assert_eq!(to_vo(row(0, 2, None)).change_time, "");
    }

    #[test]
    fn change_type_names_cover_java_switch() {
        assert_eq!(to_vo(row(1, 2, None)).change_type_name, "减少");
        assert_eq!(to_vo(row(1, 3, None)).change_type_name, "其他");
        assert_eq!(to_vo(row(1, 99, None)).change_type_name, "其他");
        assert_eq!(to_vo(row(1, 8, None)).change_type_name, "未知");
    }

    #[test]
    fn left_join_missing_user_yields_null_username() {
        let v = serde_json::to_value(to_vo(row(1, 1, None))).unwrap();
        assert!(v["username"].is_null());
    }

    #[test]
    fn page_result_shape_uses_project_envelope() {
        let q: UserBalanceLogListQuery = serde_json::from_value(serde_json::json!({"page": 2, "size": 20})).unwrap();
        let page = q.page_query();
        let empty = PageResult::<UserBalanceLogVO>::empty(&page);
        assert_eq!(empty.total, 0);
        assert!(empty.list.is_empty());
        assert_eq!(empty.page_size, 20);
        assert_eq!(empty.page, 2);

        let full = PageResult::new(41, vec![to_vo(row(1, 1, Some("a")))], &page);
        assert_eq!(full.total_page, 3); // 41 / 20 向上取整
        let v = serde_json::to_value(&full).unwrap();
        assert_eq!(v["total"], 41);
        // 同时输出 Java MyBatis-Plus 形状（前端读取）与历史字段。
        assert_eq!(v["totalPage"], 3);
        assert_eq!(v["pageSize"], 20);
        assert_eq!(v["page"], 2);
        assert_eq!(v["list"].as_array().map(Vec::len), Some(1));
        assert_eq!(v["list"][0]["logId"], 11);
        assert_eq!(v["list"][0]["changeTypeName"], "增加");
        assert_eq!(v["records"].as_array().map(Vec::len), Some(1));
        assert_eq!(v["records"][0]["logId"], 11);
        assert_eq!(v["size"], 20);
        assert_eq!(v["current"], 2);
        assert_eq!(v["pages"], 3);
    }
}