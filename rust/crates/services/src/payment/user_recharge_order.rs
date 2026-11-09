//! 充值记录域服务（payment::user_recharge_order）。
//!
//! 对应原 Java `UserRechargeOrderController`（7 端点）→
//! `UserRechargeOrderServiceImpl` 的 admin 部分。
//!
//! # 已完整实现
//!
//! `list` / `detail` / `create` / `update` / `del` / `updateField` / `batch`。
//!
//! `update` 置「已支付」时在同一事务内：`user.balance += amount + 赠送金额`
//! → 写 `user_balance_log`（`wallet_type=0`、`change_type=1`、备注
//! 「充值订单审核通过增加余额」）→ 更新充值单（与 Java `@Transactional` 等价）。
//!
//! # 偏差
//!
//! 1. `detail` 不存在 ID：业务 500「充值记录不存在」（Java 为异常 500）。
//! 2. `detail.addTime` 返回秒数文本（Java 未格式化）；`list.addTime` 格式化。
//! 3. `update.status` 缺失视为不修改（Java 拆箱 NPE）。
//! 4. `update` 未传 `amount` 时按旧金额入账（Java NPE）；`user_id` 同理保留旧值。
//! 5. 后台消息 / 站内信未迁移（本域 Java 未发送，故无影响）。
//! 6. `@Sensitive(PHONE)` demo 脱敏未迁移。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::UserRechargeOrder;
use nextrade_core::models::payment::user_recharge_order::{
    merge_recharge_update, recharge_credit_amount, status_type, validate_status_change,
    UserRechargeOrderCreateDTO, UserRechargeOrderListQuery,
    UserRechargeOrderUpdateDTO, UserRechargeOrderVO, UserRechargeOrderUserVO, BatchDTO, OperateDTO,
    UpdateFieldDTO, DESC_RECHARGE_INC, MSG_NOT_EXISTS, STATUS_SUCCESS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::user_recharge_order::{
    RechargeUserRow, UserRechargeOrderRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 充值记录域服务。
pub struct UserRechargeOrderService {
    repos: Arc<Repositories>,
}

impl UserRechargeOrderService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &UserRechargeOrderRepository {
        &self.repos.payment.user_recharge_order
    }

    // ————————————————————— 1) list —————————————————————

    /// 列表（Java `list`）。
    pub async fn list(
        &self,
        q: &UserRechargeOrderListQuery,
    ) -> Result<PageResult<UserRechargeOrderVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let user_ids = match q.effective_keyword() {
            Some(keyword) => {
                let ids = self.repo().find_user_ids_by_keyword(keyword).await?;
                if ids.is_empty() {
                    return Ok(PageResult::empty(&page));
                }
                Some(ids)
            }
            None => None,
        };
        let (total, rows) = self
            .repo()
            .paginate(q, user_ids.as_deref(), limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let uids: Vec<i32> = distinct(rows.iter().map(|r| r.user_id));
        let users = UserRechargeOrderRepository::index_users(self.repo().find_users(&uids).await?);
        let list: Vec<UserRechargeOrderVO> = rows
            .iter()
            .map(|entity| to_list_vo(entity, &users))
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 2) detail —————————————————————

    /// 详情（Java `detail`；`addTime` 未格式化，见模块偏差 2）。
    pub async fn detail(&self, id: i32) -> Result<UserRechargeOrderVO> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let users = UserRechargeOrderRepository::index_users(
            self.repo().find_users(&[entity.user_id]).await?,
        );
        Ok(to_detail_vo(&entity, &users))
    }

    // ————————————————————— 3) create —————————————————————

    /// 创建（Java `create`）。
    pub async fn create(&self, dto: &UserRechargeOrderCreateDTO) -> Result<i32> {
        let input = dto.validate().map_err(Error::Validation)?;
        let entity = UserRechargeOrder {
            order_id: 0,
            user_id: input.user_id,
            amount: input.amount,
            discount_money: input.discount_money,
            add_time: i32::try_from(now_seconds()).unwrap_or_default(),
            paid_time: input.paid_time.unwrap_or(0),
            postscript: input.postscript,
            status: input.status,
        };
        let id = self.repo().insert(&entity).await?;
        tracing::info!("创建充值记录: order_id={id}");
        Ok(id)
    }

    // ————————————————————— 4) update —————————————————————

    /// 更新（Java `update`：状态校验 + 置已支付时入账 + 更新）。
    pub async fn update(&self, dto: &UserRechargeOrderUpdateDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let existing = self
            .repo()
            .find_by_id(input.order_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let new_status = input.status.unwrap_or(existing.status);
        validate_status_change(new_status, existing.status).map_err(Error::biz_msg)?;

        let merged = merge_recharge_update(&existing, &input);
        if new_status == STATUS_SUCCESS {
            let amount = input.amount.unwrap_or(existing.amount);
            let discount = input.discount_money.or(existing.discount_money);
            let credit = recharge_credit_amount(amount, discount);
            let now = i32::try_from(now_seconds()).unwrap_or_default();
            let mut tx = self.repo().begin().await?;
            if !self
                .repo()
                .inc_balance_in_tx(&mut tx, merged.user_id, credit, DESC_RECHARGE_INC, now)
                .await?
            {
                return Err(Error::biz(500, "会员不存在"));
            }
            self.repo().update(&merged).await?;
            tx.commit()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
        } else {
            self.repo().update(&merged).await?;
        }
        tracing::info!("更新充值记录: order_id={}", input.order_id);
        Ok(())
    }

    // ————————————————————— 5) del —————————————————————

    /// 删除（Java `del`：id 为空静默返回）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        if let Some(id) = dto.id {
            let _ = self.repo().delete(id).await?;
            tracing::info!("删除充值记录: order_id={id}");
        }
        Ok(())
    }

    // ————————————————————— 6) updateField —————————————————————

    /// 字段更新（Java `updateField(dto, GENERAL_FIELDS)`）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        self.repo()
            .update_field(input.id, input.column, &input.val)
            .await?;
        Ok(())
    }

    // ————————————————————— 7) batch —————————————————————

    /// 批量操作（Java `BaseServiceImpl#batch`：仅 del）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.as_deref().unwrap_or_default();
        if dto.r#type.as_deref() == Some("del") {
            let n = self.repo().delete_batch(ids).await?;
            tracing::info!("批量删除充值记录: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }
}

// ——————————————————————————————————————————————
// VO 装配（纯函数）
// ——————————————————————————————————————————————

/// 列表行 → VO（`addTime` 格式化）。
#[must_use]
pub fn to_list_vo(
    entity: &UserRechargeOrder,
    users: &std::collections::HashMap<i32, RechargeUserRow>,
) -> UserRechargeOrderVO {
    let mut vo = base_vo(entity, users);
    vo.add_time = nextrade_core::models::payment::user_recharge_order::format_time(entity.add_time);
    vo
}

/// 详情行 → VO（`addTime` 秒数文本，与 Java detail 一致）。
#[must_use]
pub fn to_detail_vo(
    entity: &UserRechargeOrder,
    users: &std::collections::HashMap<i32, RechargeUserRow>,
) -> UserRechargeOrderVO {
    let mut vo = base_vo(entity, users);
    vo.add_time = entity.add_time.to_string();
    vo
}

fn base_vo(
    entity: &UserRechargeOrder,
    users: &std::collections::HashMap<i32, RechargeUserRow>,
) -> UserRechargeOrderVO {
    let user = users.get(&entity.user_id).map(|u| UserRechargeOrderUserVO {
        user_id: u.user_id,
        username: u.username.clone(),
        nickname: u.nickname.clone(),
        mobile: u.mobile.clone(),
    });
    UserRechargeOrderVO {
        order_id: entity.order_id,
        user_id: entity.user_id,
        username: user.as_ref().map(|u| u.username.clone()),
        user,
        amount: entity.amount,
        discount_money: entity.discount_money,
        add_time: String::new(),
        paid_time: entity.paid_time,
        postscript: entity.postscript.clone(),
        status: entity.status,
        status_type: status_type(entity.status).to_string(),
    }
}

fn distinct(ids: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = UserRechargeOrderRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::user_recharge_order::{
        MSG_AMOUNT_ERROR, MSG_ID_REQUIRED_DTO, MSG_STATUS_ERROR,
    };

    fn entity() -> UserRechargeOrder {
        UserRechargeOrder {
            order_id: 5,
            user_id: 7,
            amount: rust_decimal::Decimal::new(10000, 2),
            discount_money: Some(rust_decimal::Decimal::new(1000, 2)),
            add_time: 1_700_000_000,
            paid_time: 0,
            postscript: "备注".into(),
            status: 0,
        }
    }

    fn users() -> std::collections::HashMap<i32, RechargeUserRow> {
        let mut map = std::collections::HashMap::new();
        map.insert(
            7,
            RechargeUserRow {
                user_id: 7,
                username: "alice".into(),
                nickname: Some("爱丽丝".into()),
                mobile: "13800008000".into(),
            },
        );
        map
    }

    #[test]
    fn list_vo_formats_time_and_detail_keeps_seconds() {
        let list_vo = to_list_vo(&entity(), &users());
        assert_eq!(list_vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(list_vo.username.as_deref(), Some("alice"));
        assert_eq!(
            list_vo.user.as_ref().and_then(|u| u.nickname.clone()),
            Some("爱丽丝".into())
        );
        assert_eq!(list_vo.status_type, "待确认");
        let detail_vo = to_detail_vo(&entity(), &users());
        assert_eq!(detail_vo.add_time, "1700000000");
        // 无用户时不 panic
        let vo = to_list_vo(&entity(), &std::collections::HashMap::new());
        assert!(vo.user.is_none());
        assert_eq!(vo.username, None);
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(20));
        assert_eq!((p.page, p.page_size), (2, 20));
    }

    #[test]
    fn validation_texts() {
        assert_eq!(
            UserRechargeOrderCreateDTO::default().validate().unwrap_err(),
            MSG_AMOUNT_ERROR
        );
        assert_eq!(
            UserRechargeOrderUpdateDTO::default().validate().unwrap_err(),
            MSG_ID_REQUIRED_DTO
        );
        assert_eq!(
            validate_status_change(1, 1).unwrap_err(),
            MSG_STATUS_ERROR
        );
        assert_eq!(distinct([2, 1, 2].into_iter()), vec![2, 1]);
    }
}
