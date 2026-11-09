//! 会员等级/标签/积分流水实体：`user_rank` / `member_tag` /
//! `member_tag_group` / `user_points_log`。
//! 对应 `com.nextrade.bean.model.user.*`。
//!
//! TODO(域实现者)：对照 `java/database/pure.sql` 中各表的完整列补全字段。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 会员等级。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserRank {
    pub rank_id: i32,
    pub rank_name: Option<String>,
    /// 升级所需经验/消费额
    pub upgrade_amount: Option<rust_decimal::Decimal>,
    /// 折扣率（百分比，100 表示无折扣）
    pub discount: Option<i32>,
}

impl UserRank {
    pub const TABLE: &'static str = "user_rank";

    /// 是否享受折扣（discount 在 1~99 之间）。
    pub fn has_discount(&self) -> bool {
        matches!(self.discount, Some(d) if d > 0 && d < 100)
    }
}

/// 会员标签组。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MemberTagGroup {
    pub group_id: i32,
    pub group_name: Option<String>,
    pub sort: Option<i32>,
}

impl MemberTagGroup {
    pub const TABLE: &'static str = "member_tag_group";
}

/// 会员标签。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct MemberTag {
    pub tag_id: i32,
    pub tag_name: Option<String>,
    pub group_id: Option<i32>,
}

impl MemberTag {
    pub const TABLE: &'static str = "member_tag";
}

/// 积分流水。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct UserPointsLog {
    pub log_id: i32,
    pub user_id: Option<i32>,
    /// 变动积分（正增负减）
    pub points: Option<i32>,
    /// 变动后余额
    pub balance: Option<i32>,
    pub add_time: Option<i64>,
}

impl UserPointsLog {
    pub const TABLE: &'static str = "user_points_log";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discount_check() {
        let r = UserRank {
            discount: Some(95),
            ..Default::default()
        };
        assert!(r.has_discount());
        let r2 = UserRank {
            discount: Some(100),
            ..Default::default()
        };
        assert!(!r2.has_discount());
    }
}