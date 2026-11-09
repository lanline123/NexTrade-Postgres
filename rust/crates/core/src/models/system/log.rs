//! 统计基础表域（system::log）DTO + 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.api.controller.common.LogController`
//!   （`@RequestMapping("/api/common/log")` + `@GetMapping("")`，GET 端点）
//! - `com.nextrade.service.log.impl.StatisticsBaseServiceImpl#create`
//! - `com.nextrade.bean.dto.log.StatisticsBaseCreateDTO`
//!
//! `GET /api/common/log` 的语义（Java 原文）：
//!
//! 1. 以**当天日期**（`DateUtil.today()`，`yyyy-MM-dd`）为 key 累加
//!    `statistics_base`：先查 `date = ? LIMIT 1`（**不带店铺过滤**），
//!    没有则 insert（`click_count = 1`，`isNew != null` 时 `visitor_count = 1`），
//!    有则 `click_count + 1`（`isNew != null` 时 `visitor_count + 1`）；
//! 2. 若请求头解出 `shopId > 0`，再对 `date = ? AND shop_id = ?` 做同样的累加；
//! 3. 若 `productId != null || shopCategoryId != null`，向 `statistics_log`
//!    插入一条明细（`access_time = 当前秒`，`shop_id / product_id /
//!    shop_category_id` 缺失记 0，`user = 当前会员 ID 字符串`）。
//!
//! 注意 Java 的 `getCurrentUserId()` 返回 `Integer`，未登录时是 **0 而不是 null**，
//! 因此 `user` 列实际恒为数字字符串（游客为 `"0"`），`IpUtils.getIpAddr()` 兜底
//! 分支是死代码 —— 本实现保持一致，见 [`user_for_log`]。

use serde::{Deserialize, Serialize};

/// 统计基础表创建参数（Java `StatisticsBaseCreateDTO`）。
///
/// 三个字段全部可选；`isNew != null` 是「是否累计访客数」的开关
/// （Java 用 `!= null` 而非 `== 1`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatisticsBaseCreateDTO {
    /// 是否新访客（`isNew`，非 null 即计访客）
    pub is_new: Option<i32>,
    /// 产品 id（`productId`）
    pub product_id: Option<i32>,
    /// 店铺分类 id（`shopCategoryId`）
    pub shop_category_id: Option<i32>,
}

impl StatisticsBaseCreateDTO {
    /// 是否需要写 `statistics_log` 明细
    /// （Java `productId != null || shopCategoryId != null`）。
    #[must_use]
    pub fn needs_detail_log(&self) -> bool {
        self.product_id.is_some() || self.shop_category_id.is_some()
    }

    /// 是否累计访客数（Java `createDTO.getIsNew() != null`）。
    #[must_use]
    pub fn counts_visitor(&self) -> bool {
        self.is_new.is_some()
    }
}

/// 统计基础表的计数增减（Java `StatisticsBaseServiceImpl#create` 的核心分支）。
///
/// `current`：
/// - `None`：当天该维度还没有行 ⇒ 新建行；
/// - `Some((click, visitor))`：已有行（`visitor` 可能为 SQL NULL）。
///
/// 返回 `(click_count, visitor_count)`：
/// - 新建行：`click = 1`；`visitor = Some(1)`（计访客）或 `Some(0)`（不计访客，
///   MySQL 列默认 0，Java `save` 未赋值即落 0）；
/// - 已有行：`click + 1`；计访客时 `visitor + 1`，否则原值（可能为 `None`）。
///
/// ⚠️ Java 对 `clickCount == null` 会 NPE → 500；本实现按 0 处理（不 panic）。
#[must_use]
pub fn next_base_counts(
    current: Option<(i32, Option<i32>)>,
    counts_visitor: bool,
) -> (i32, Option<i32>) {
    match current {
        None => (1, Some(if counts_visitor { 1 } else { 0 })),
        Some((click_count, visitor_count)) => {
            let visitor_count = if counts_visitor {
                Some(visitor_count.unwrap_or(0) + 1)
            } else {
                visitor_count
            };
            (click_count + 1, visitor_count)
        }
    }
}

/// `statistics_log.shop_id` 落库值（Java `shopId != null ? shopId : 0`）。
#[must_use]
pub fn shop_id_for_log(shop_id: Option<i32>) -> i32 {
    shop_id.unwrap_or(0)
}

/// `statistics_log.product_id` 落库值（Java `productId != null ? productId : 0`）。
#[must_use]
pub fn product_id_for_log(product_id: Option<i32>) -> i32 {
    product_id.unwrap_or(0)
}

/// `statistics_log.shop_category_id` 落库值
/// （Java `shopCategoryId != null ? shopCategoryId : 0`）。
#[must_use]
pub fn shop_category_id_for_log(shop_category_id: Option<i32>) -> i32 {
    shop_category_id.unwrap_or(0)
}

/// `statistics_log.user` 落库值。
///
/// Java：`userId != null ? userId.toString() : IpUtils.getIpAddr()`；
/// 而 `SecurityUtils.getCurrentUserId()` 未登录返回 **0（非 null）**，
/// 所以常量分支恒命中，实际写的是 `"0"` 这类数字串（IP 兜底是死代码）。
#[must_use]
pub fn user_for_log(user_id: i32) -> String {
    user_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ———————————————— 计数增减 ————————————————

    #[test]
    fn new_row_counts_start_from_one() {
        assert_eq!(next_base_counts(None, false), (1, Some(0)));
        assert_eq!(next_base_counts(None, true), (1, Some(1)));
    }

    #[test]
    fn existing_row_increments_click_and_optional_visitor() {
        // 不计访客：只 +点击，visitor 原值保留
        assert_eq!(next_base_counts(Some((5, Some(2))), false), (6, Some(2)));
        // 计访客：点击 +1、访客 +1
        assert_eq!(next_base_counts(Some((5, Some(2))), true), (6, Some(3)));
        // visitor 为 NULL 且计访客：按 0 + 1（Java 会 NPE，本实现更宽容）
        assert_eq!(next_base_counts(Some((5, None)), true), (6, Some(1)));
        // visitor 为 NULL 且不计访客：保持 NULL
        assert_eq!(next_base_counts(Some((5, None)), false), (6, None));
    }

    // ———————————————— 明细字段兜底 ————————————————

    #[test]
    fn detail_log_fields_fall_back_to_zero() {
        assert_eq!(shop_id_for_log(None), 0);
        assert_eq!(shop_id_for_log(Some(9)), 9);
        assert_eq!(product_id_for_log(None), 0);
        assert_eq!(product_id_for_log(Some(7)), 7);
        assert_eq!(shop_category_id_for_log(None), 0);
        assert_eq!(shop_category_id_for_log(Some(3)), 3);
    }

    #[test]
    fn log_user_is_member_id_string_even_for_guests() {
        assert_eq!(user_for_log(0), "0");
        assert_eq!(user_for_log(42), "42");
    }

    // ———————————————— DTO 字段名 ————————————————

    #[test]
    fn dto_accepts_java_camel_case_names() {
        let dto: StatisticsBaseCreateDTO =
            serde_json::from_str(r#"{"isNew":1,"productId":7,"shopCategoryId":3}"#).unwrap();
        assert_eq!(dto.is_new, Some(1));
        assert_eq!(dto.product_id, Some(7));
        assert_eq!(dto.shop_category_id, Some(3));
        assert!(dto.needs_detail_log());
        assert!(dto.counts_visitor());

        let dto: StatisticsBaseCreateDTO = serde_json::from_str("{}").unwrap();
        assert_eq!(dto.is_new, None);
        assert!(!dto.needs_detail_log());
        assert!(!dto.counts_visitor());
    }

    #[test]
    fn is_new_zero_still_counts_visitor() {
        // Java 判 `!= null`，`isNew = 0` 同样累计访客
        let dto: StatisticsBaseCreateDTO = serde_json::from_str(r#"{"isNew":0}"#).unwrap();
        assert!(dto.counts_visitor());
    }
}
