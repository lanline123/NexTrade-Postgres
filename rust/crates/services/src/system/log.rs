//! 统计基础表域服务（system::log）。
//!
//! 对应原 Java `com.nextrade.service.log.impl.StatisticsBaseServiceImpl#create`
//! （以及 `LogController` 唯一的 `GET /api/common/log`）。
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! 1. 以**当天日期**（`yyyy-MM-dd`）为 key 查 `statistics_base`（Java 该查询
//!    **不带 shop 过滤**，`LIMIT 1` 取首行）：
//!    - 无行 ⇒ insert（`click_count = 1`，计访客时 `visitor_count = 1`，否则 0；
//!      全局行 `shop_id = 0`）
//!    - 有行 ⇒ `click_count + 1`，计访客时 `visitor_count + 1`
//! 2. 请求头解出 `shopId > 0` 时，对 `date + shop_id` 再做一遍同样的累加
//!    （新建行 `shop_id` 即该店铺）。
//! 3. `productId != null || shopCategoryId != null` ⇒ 插入一条 `statistics_log`
//!    （`access_time = 当前秒`，缺失 id 记 0，`user` 为当前会员 ID 字符串）。
//!
//! Java 该方法**没有** `@Transactional`（每次写入自动提交），本实现同样逐条
//! 下发语句，不额外包事务。
//!
//! # 未迁移 / 不确定点
//!
//! - `SecurityUtils.getCurrentUserId()` → Rust 由路由层从可选 JWT 提取后传入
//!   （游客为 0，与 Java 在匿名上下文返回 0 一致）。
//! - `getShopId()` → Rust 由路由层按 `HeaderUtils` 语义解析请求头后传入，
//!   见 `api/public` 的同名路由模块。
//! - Java `clickCount == null` / `visitorCount == null` 时会 NPE → 500；
//!   本实现按 0 处理（见 [`next_base_counts`]）。
//! - Java `getCurrentUserId()` 恒非 null，`IpUtils.getIpAddr()` 兜底是死代码；
//!   本实现保持一致（游客写 `"0"`）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::StatisticsLog;
use nextrade_core::models::system::log::{
    next_base_counts, product_id_for_log, shop_category_id_for_log, shop_id_for_log, user_for_log,
    StatisticsBaseCreateDTO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::Repositories;

/// 统计基础表域服务。
pub struct LogService {
    repos: Arc<Repositories>,
}

impl LogService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 创建日志 / 统计记录（对应 Java `StatisticsBaseServiceImpl#create`）。
    ///
    /// `user_id`：当前会员 ID（游客 / 未登录为 0，对齐 Java `getCurrentUserId()`）。
    /// `shop_id`：Java `HeaderUtils.getShopId()` 的解析结果；
    /// `Some(0)`（平台账号）不会触发店铺维度累加，`None` 同理。
    ///
    /// # Errors
    /// - 仓储访问失败 ⇒ `Error::Database`
    pub async fn create(
        &self,
        dto: &StatisticsBaseCreateDTO,
        user_id: i32,
        shop_id: Option<i32>,
    ) -> Result<()> {
        let date = today_date();
        let counts_visitor = dto.counts_visitor();

        // —— 1. 全局行（Java 查询不带 shop 过滤）——
        let current = self.repos.system.log.find_by_date(&date).await?;
        let (click_count, visitor_count) = next_base_counts(
            current
                .as_ref()
                .map(|row| (row.click_count.unwrap_or(0), row.visitor_count)),
            counts_visitor,
        );
        match current {
            Some(row) => {
                self.repos.system.log
                    .update_counts(row.id, click_count, visitor_count)
                    .await?;
            }
            None => {
                self.repos.system.log
                    .insert_base(&date, click_count, 0, visitor_count.unwrap_or(0))
                    .await?;
            }
        }

        // —— 2. 店铺维度行（仅 shopId > 0）——
        if let Some(shop_id) = shop_id.filter(|v| *v > 0) {
            let current = self
                .repos.system.log
                .find_by_date_and_shop(&date, shop_id)
                .await?;
            let (click_count, visitor_count) = next_base_counts(
                current
                    .as_ref()
                    .map(|row| (row.click_count.unwrap_or(0), row.visitor_count)),
                counts_visitor,
            );
            match current {
                Some(row) => {
                    self.repos.system.log
                        .update_counts(row.id, click_count, visitor_count)
                        .await?;
                }
                None => {
                    self.repos.system.log
                        .insert_base(&date, click_count, shop_id, visitor_count.unwrap_or(0))
                        .await?;
                }
            }
        }

        // —— 3. 明细日志（商品 / 店铺分类维度）——
        if dto.needs_detail_log() {
            let log = build_detail_log(dto, user_id, shop_id);
            self.repos.system.log.insert_log(&log).await?;
        }

        Ok(())
    }
}

/// 构造 `statistics_log` 明细行（Java `getStatisticsLog`，纯函数便于单测）。
fn build_detail_log(
    dto: &StatisticsBaseCreateDTO,
    user_id: i32,
    shop_id: Option<i32>,
) -> StatisticsLog {
    StatisticsLog {
        id: 0,
        access_time: Some(now_seconds() as i32),
        shop_id: Some(shop_id_for_log(shop_id)),
        product_id: Some(product_id_for_log(dto.product_id)),
        shop_category_id: shop_category_id_for_log(dto.shop_category_id),
        user: Some(user_for_log(user_id)),
    }
}

/// 当天日期（Java `DateUtil.today()`，`yyyy-MM-dd`）。
fn today_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::system::log::StatisticsBaseCreateDTO;

    #[test]
    fn today_date_is_yyyy_mm_dd() {
        let date = today_date();
        assert_eq!(date.len(), 10, "yyyy-MM-dd 长度应为 10: {date}");
        let parts: Vec<&str> = date.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 4);
        assert_eq!(parts[1].len(), 2);
        assert_eq!(parts[2].len(), 2);
        assert!(parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())));
    }

    #[test]
    fn detail_log_fills_java_defaults() {
        let dto = StatisticsBaseCreateDTO {
            is_new: Some(1),
            product_id: Some(7),
            shop_category_id: None,
        };
        let log = build_detail_log(&dto, 42, Some(9));
        assert_eq!(log.shop_id, Some(9));
        assert_eq!(log.product_id, Some(7));
        assert_eq!(log.shop_category_id, 0);
        assert_eq!(log.user.as_deref(), Some("42"));
        assert!(log.access_time.is_some_and(|t| t > 0));

        // 游客 / 无店铺：shop_id、product_id 记 0，user = "0"
        let dto = StatisticsBaseCreateDTO {
            is_new: None,
            product_id: None,
            shop_category_id: Some(3),
        };
        let log = build_detail_log(&dto, 0, None);
        assert_eq!(log.shop_id, Some(0));
        assert_eq!(log.product_id, Some(0));
        assert_eq!(log.shop_category_id, 3);
        assert_eq!(log.user.as_deref(), Some("0"));
    }

    /// 店铺维度累加只在 `shopId > 0` 时发生（`Some(0)` 与 `None` 都不累加）。
    #[test]
    fn shop_branch_condition_matches_java() {
        let should_bump = |shop_id: Option<i32>| shop_id.filter(|v| *v > 0).is_some();
        assert!(should_bump(Some(1)));
        assert!(!should_bump(Some(0)));
        assert!(!should_bump(None));
    }
}
