//! 拼团下单记录（开团 / 参团）状态机。
//!
//! 对应 Java `GrouponPromotionRecordServiceImpl#saveRecord`：
//! - 无 `grouponPromotionRecordId` → 开团（`join_type=1`），成团人数取阶梯配置；
//!   1 人团立即成功（`join_status=2`），否则待成团（`1`）；
//! - 带开团记录 ID → 参团（`join_type=2`），人数达到 `min_group_num` 时整团成功；
//! - 记录与订单同事务写入；过期由定时任务置失败（`3`）。
use sqlx::{Postgres, Transaction};

use nextrade_core::entities::GrouponPromotionRecord;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::trade::check::GrouponPrice;

const JOIN_TYPE_OPEN: i32 = 1;
const JOIN_TYPE_JOIN: i32 = 2;
const JOIN_STATUS_WAIT: i32 = 1;
const JOIN_STATUS_SUCCESS: i32 = 2;
const DEFAULT_EXPIRATION_SECONDS: i64 = 86_400;

/// 下单事务内写入拼团记录，返回「开团记录 ID」（前端跳转拼团详情用）。
///
/// # Errors
/// 团已结束/不存在、数据库错误。
pub(crate) async fn create_record_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    repos: &Repositories,
    user_id: i32,
    username: &str,
    order_id: i32,
    order_sn: &str,
    product_name: &str,
    line: &GrouponPrice,
) -> Result<i64> {
    let now = nextrade_common::utils::now_seconds() as i64;
    let data = &line.data;
    let join_expiration = now
        + if line.promotion.expiration_time > 0 {
            line.promotion.expiration_time
        } else {
            DEFAULT_EXPIRATION_SECONDS
        };
    let repo = &repos.marketing.groupon_promotion_record;

    match data.groupon_promotion_record_id.filter(|id| *id > 0) {
        None => {
            // —— 开团 ——
            let join_status = if line.min_group_num <= 1 {
                JOIN_STATUS_SUCCESS
            } else {
                JOIN_STATUS_WAIT
            };
            let record = GrouponPromotionRecord {
                groupon_promotion_record_id: 0,
                groupon_promotion_id: data.groupon_promotion_id.unwrap_or(0),
                groupon_promotion_item_id: data.groupon_promotion_item_id.unwrap_or(0),
                product_id: line.item.product_id,
                product_name: product_name.to_string(),
                join_type: JOIN_TYPE_OPEN,
                join_status,
                avatar: None,
                username: Some(username.to_string()),
                user_id: i64::from(user_id),
                order_id: i64::from(order_id),
                order_sn: order_sn.to_string(),
                is_virtual: 0,
                paid_amount: line.price,
                open_group_record_id: 0,
                open_group_time: now,
                group_captain_name: username.to_string(),
                join_expiration_time: join_expiration,
                min_group_num: line.min_group_num,
                shop_id: line.promotion.shop_id.unwrap_or(0),
                add_time: now,
            };
            let id = repo.insert_record_in_tx(tx, &record).await?;
            repo.set_open_group_id_in_tx(tx, id, id).await?;
            tracing::info!(
                "拼团开团: record={id} promotion={} min_group_num={} status={join_status}",
                record.groupon_promotion_id,
                line.min_group_num
            );
            Ok(id)
        }
        Some(open_id) => {
            // —— 参团 ——
            let open = repo
                .find_by_id(open_id)
                .await?
                .ok_or_else(|| Error::biz_msg("拼团记录不存在"))?;
            if open.join_type != JOIN_TYPE_OPEN || open.join_status != JOIN_STATUS_WAIT {
                return Err(Error::biz_msg("该团已结束，请重新开团"));
            }
            if open.join_expiration_time > 0 && open.join_expiration_time < now {
                return Err(Error::biz_msg("该团已过期，请重新开团"));
            }
            if open.groupon_promotion_item_id
                != data.groupon_promotion_item_id.unwrap_or(0)
            {
                return Err(Error::biz_msg("拼团活动商品不匹配"));
            }
            let member_count = repo.count_members_in_tx(tx, open_id).await?;
            let join_status = if member_count + 1 >= i64::from(open.min_group_num) {
                JOIN_STATUS_SUCCESS
            } else {
                JOIN_STATUS_WAIT
            };
            if join_status == JOIN_STATUS_SUCCESS {
                repo.mark_success_in_tx(tx, open_id).await?;
            }
            let record = GrouponPromotionRecord {
                groupon_promotion_record_id: 0,
                groupon_promotion_id: open.groupon_promotion_id,
                groupon_promotion_item_id: open.groupon_promotion_item_id,
                product_id: open.product_id,
                product_name: product_name.to_string(),
                join_type: JOIN_TYPE_JOIN,
                join_status,
                avatar: None,
                username: Some(username.to_string()),
                user_id: i64::from(user_id),
                order_id: i64::from(order_id),
                order_sn: order_sn.to_string(),
                is_virtual: 0,
                paid_amount: line.price,
                open_group_record_id: open_id,
                open_group_time: open.open_group_time,
                group_captain_name: open.group_captain_name.clone(),
                join_expiration_time: open.join_expiration_time,
                min_group_num: open.min_group_num,
                shop_id: open.shop_id,
                add_time: now,
            };
            repo.insert_record_in_tx(tx, &record).await?;
            tracing::info!(
                "拼团参团: open={open_id} members={} status={join_status}",
                member_count + 1
            );
            Ok(open_id)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn join_status_constants_match_java_enum() {
        assert_eq!(super::JOIN_TYPE_OPEN, 1);
        assert_eq!(super::JOIN_TYPE_JOIN, 2);
        assert_eq!(super::JOIN_STATUS_WAIT, 1);
        assert_eq!(super::JOIN_STATUS_SUCCESS, 2);
    }
}
