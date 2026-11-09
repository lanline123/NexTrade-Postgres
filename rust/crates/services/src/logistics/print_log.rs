//! 打印日志域服务（logistics::print_log）。
//!
//! 对应原 Java `com.nextrade.service.print.impl.PrintLogServiceImpl#page`
//! 与 `com.nextrade.adminapi.controller.print.PrintLogController#list`。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java | 方法 |
//! |---------------------------|---------|------|
//! | `GET /print/printLog/list` | `PrintLogController.list` | [`PrintLogService::page`] |
//!
//! # 语义（逐条对照 Java）
//!
//! `PrintLogServiceImpl#page`：
//!
//! ```text
//! Page<PrintLog> page = buildSortOrder(query);          // Page.orders
//! wrapper.eq(shopId, getShopId());
//! if (platform != null)     wrapper.eq(platform, ...);
//! if (status != null)       wrapper.eq(status, ...);
//! if (triggerType != null)  wrapper.eq(triggerType, ...);
//! if (orderId != null)      wrapper.eq(orderId, ...);
//! if (isNotBlank(printSn))  wrapper.like(printSn, ...);
//! if (startTime != null)    wrapper.ge(addTime, startTime);
//! if (endTime != null)      wrapper.le(addTime, endTime);
//! if (sortField 为空)       wrapper.orderByDesc(addTime);
//! Page<PrintLog> data = this.page(page, wrapper);
//! → data.records.map(PrintLogPageVO::new)
//! ```
//!
//! 排序细节见仓储层
//! `nextrade_infrastructure::db::repo::logistics::print_log::order_clause`。
//!
//! # 与 Java 的已知差异（`uncertain_points`）
//!
//! 1. **多租户**：Java 用请求头 `HeaderUtils.getShopId()`；Rust JWT 尚未
//!    签发 `shop_id`，由路由层传 [`DEFAULT_SHOP_ID`]（平台 `0`）。
//! 2. **分页返回结构**：Java 是 MyBatis-Plus `Page<PrintLogPageVO>`
//!    （`records/total/size/current/pages`），Rust 统一 `PageResult`
//!    （`list/total/totalPage/pageSize/page`），与同域 print / print_config
//!    的既有约定一致。
//! 3. **写路径不在本 feature 范围**：`createPending` / `markPushed` /
//!    `markFailed` / `markCallback` / `findByTaskNo` 服务于 MQ 消费端与
//!    商米云回调，本仓 `queue` 尚无 publish（RabbitMQ 客户端未迁移），
//!    因此本 feature 只做管理端列表查询。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use std::sync::Arc;

use nextrade_core::entities::PrintLog;
use nextrade_core::models::logistics::print_config::DEFAULT_SHOP_ID;
use nextrade_core::models::logistics::print_log::{
    log_status_desc, platform_name, trigger_type_desc, PrintLogPageQuery, PrintLogVO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::logistics::print_log::{PrintLogFilters, PrintLogRepository};
use nextrade_infrastructure::db::Repositories;

use crate::logistics::print::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

/// 打印日志域服务（挂载于 `AdminState.logistics.print_log`）。
pub struct PrintLogService {
    repos: Arc<Repositories>,
}

impl PrintLogService {
    /// 构造（与同域 `print` / `print_config` 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &PrintLogRepository {
        &self.repos.logistics.print_log
    }

    /// 路由层默认使用的店铺 ID（平台 `0`，见模块头 `uncertain_points`）。
    #[must_use]
    pub const fn default_shop_id() -> i32 {
        DEFAULT_SHOP_ID
    }

    /// 打印日志分页列表（对应 Java `PrintLogServiceImpl#page`）。
    ///
    /// 每页条数缺省 15、上限 100（Java `BasePage#getSize()`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn page(
        &self,
        q: &PrintLogPageQuery,
        shop_id: i32,
    ) -> Result<PageResult<PrintLogVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q
                .page_size
                .unwrap_or(DEFAULT_PAGE_SIZE)
                .min(MAX_PAGE_SIZE),
        };
        let filters = PrintLogFilters {
            platform: q.platform,
            status: q.status,
            trigger_type: q.trigger_type,
            order_id: q.order_id,
            print_sn: q.print_sn.as_deref(),
            start_time: q.start_time,
            end_time: q.end_time,
        };
        let (total, rows) = self
            .repo()
            .paginate(
                shop_id,
                &filters,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;
        let list: Vec<PrintLogVO> = rows.iter().map(to_vo).collect();
        Ok(PageResult::new(total, list, &page))
    }
}

/// 实体 → 列表 VO（对应 Java `new PrintLogPageVO(PrintLog)`）。
///
/// 逐字段照抄 Java 构造器：状态 / 平台 / 触发方式三组「码 + 文案」、
/// 原始秒级时间戳、不返回大报文列。
#[must_use]
fn to_vo(row: &PrintLog) -> PrintLogVO {
    PrintLogVO {
        log_id: row.log_id,
        print_id: row.print_id,
        print_sn: row.print_sn.clone(),
        platform: row.platform,
        platform_name: platform_name(row.platform).to_string(),
        order_id: row.order_id,
        trigger_type: row.trigger_type,
        trigger_type_desc: trigger_type_desc(row.trigger_type).to_string(),
        status: row.status,
        status_desc: log_status_desc(row.status).to_string(),
        error_code: row.error_code.clone(),
        error_msg: row.error_msg.clone(),
        retry_count: row.retry_count,
        pushed_time: row.pushed_time,
        printed_time: row.printed_time,
        add_time: row.add_time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_row() -> PrintLog {
        PrintLog {
            log_id: 9_000_000_001,
            shop_id: 0,
            print_id: 3,
            platform: 2,
            order_id: 88,
            print_sn: Some("SN-1".into()),
            trigger_type: 1,
            status: 2,
            error_code: None,
            error_msg: Some("ok".into()),
            task_no: Some("task-1".into()),
            push_response: Some("{}".into()),
            callback_payload: Some("{}".into()),
            retry_count: 0,
            pushed_time: Some(1_700_000_001),
            printed_time: Some(1_700_000_002),
            add_time: Some(1_700_000_000),
            update_time: Some(1_700_000_002),
        }
    }

    /// 默认店铺 ID 是平台 0（Java `HeaderUtils.getShopId()` 在 `admin` 下取 0）。
    #[test]
    fn default_shop_id_is_platform() {
        assert_eq!(PrintLogService::default_shop_id(), 0);
        assert_eq!(PrintLogService::default_shop_id(), DEFAULT_SHOP_ID);
    }

    /// 行映射：三组「码 + 文案」、原始时间戳、大报文列不进 VO。
    #[test]
    fn to_vo_maps_row_like_java() {
        let vo = to_vo(&log_row());
        assert_eq!(vo.log_id, 9_000_000_001);
        assert_eq!(vo.print_id, 3);
        assert_eq!(vo.print_sn.as_deref(), Some("SN-1"));
        assert_eq!(vo.platform, 2);
        assert_eq!(vo.platform_name, "商米云");
        assert_eq!(vo.order_id, 88);
        assert_eq!(vo.trigger_type_desc, "自动");
        assert_eq!(vo.status_desc, "打印成功");
        assert_eq!(vo.retry_count, 0);
        // 原始 Unix 秒（Java VO 未格式化）
        assert_eq!(vo.add_time, Some(1_700_000_000));
        assert_eq!(vo.pushed_time, Some(1_700_000_001));
        assert_eq!(vo.printed_time, Some(1_700_000_002));
        // `task_no` / `push_response` / `callback_payload` / `update_time`
        // 不在 Java `PrintLogPageVO` 里，本 VO 同样不返回
        let json = serde_json::to_value(&vo).unwrap();
        assert!(json.get("taskNo").is_none());
        assert!(json.get("pushResponse").is_none());
        assert!(json.get("updateTime").is_none());
    }

    /// 未知枚举码兜底「未知」，不 panic（Java `getDescByCode` default 分支）。
    #[test]
    fn to_vo_tolerates_unknown_enum_codes() {
        let mut row = log_row();
        row.platform = 99;
        row.trigger_type = 99;
        row.status = 99;
        let vo = to_vo(&row);
        assert_eq!(vo.platform_name, "未知平台");
        assert_eq!(vo.trigger_type_desc, "未知");
        assert_eq!(vo.status_desc, "未知");
    }

    /// 缺省分页参数走 Java `BasePage` 的 15 条 / 100 上限。
    #[test]
    fn page_size_defaults_match_base_page() {
        assert_eq!(DEFAULT_PAGE_SIZE, 15);
        assert_eq!(MAX_PAGE_SIZE, 100);
        let q = PrintLogPageQuery::default();
        assert_eq!(q.page, None);
        assert_eq!(q.page_size, None);
        // 缺省换算与 Java `getPage()/getSize()` 一致
        assert_eq!(q.page.unwrap_or(1), 1);
        assert_eq!(
            q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
            15
        );
        assert_eq!(Some(500_u64).unwrap_or(15).min(MAX_PAGE_SIZE), 100);
    }
}
