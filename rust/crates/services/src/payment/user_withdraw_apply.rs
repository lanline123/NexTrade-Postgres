//! 提现申请域服务（payment::user_withdraw_apply）。
//!
//! 对应原 Java `UserWithdrawApplyController`（7 端点）→
//! `UserWithdrawApplyServiceImpl`（admin 部分）。
//!
//! # 已完整实现
//!
//! `list` / `detail` / `create` / `update` / `del` / `updateField` / `batch`。
//!
//! - `create`：事务内「插入申请 → 增加冻结余额 → 减少可用余额」（余额账户
//!   `wallet_type=0` 语义；管理端创建默认 0），两条 `user_balance_log` 流水。
//! - `update`：旧状态必须 `<= 0`；置 1 扣减冻结，置其它返还可用 + 扣减冻结；
//!   `wallet_type=1` 时操作佣金账户。
//!
//! # 偏差
//!
//! 1. `detail` 不存在：业务 500「提现申请不存在」（Java NPE/500）。
//! 2. `create` 后台消息（「您有新的提现申请…」）未迁移（admin msg 域未迁移）。
//! 3. `list.keyword` 未命中用户时：Java 不加过滤返回全量（原实现缺陷），
//!    Rust 返回空页（安全优先）。
//! 4. `@Sensitive(PHONE)` demo 脱敏未迁移；`handlingFee/handlingRate` 恒 null。
//! 5. 用户不存在时 Java 可能 NPE，Rust 统一业务 500「用户不存在」。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::UserWithdrawApply;
use nextrade_core::models::payment::user_withdraw_apply::{
    format_time, merge_withdraw_update, parse_account_data, status_type, UserWithdrawApplyCreateDTO,
    UserWithdrawApplyCreate, UserWithdrawApplyListQuery, UserWithdrawApplyUpdateDTO,
    UserWithdrawApplyVO, BatchDTO, OperateDTO, UpdateFieldDTO, DESC_DEC_BALANCE,
    DESC_DEC_FROZEN_BALANCE, DESC_DEC_FROZEN_COMMISSION, DESC_DEC_FROZEN_COMMISSION_REFUND,
    DESC_DEC_FROZEN_REFUND, DESC_INC_BALANCE_REFUND, DESC_INC_COMMISSION_REFUND,
    DESC_INC_FROZEN, MSG_ALREADY_FINISHED, MSG_BALANCE_NOT_ENOUGH, MSG_NOT_EXISTS,
    MSG_PENDING_DELETE, MSG_USER_NOT_EXISTS, STATUS_WAIT, WALLET_BALANCE, WALLET_COMMISSION,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::user_withdraw_apply::{
    BalanceChange, UserWithdrawApplyRepository, WithdrawUserRow,
};
use nextrade_infrastructure::db::Repositories;

/// 提现申请域服务。
pub struct UserWithdrawApplyService {
    repos: Arc<Repositories>,
}

impl UserWithdrawApplyService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &UserWithdrawApplyRepository {
        &self.repos.payment.user_withdraw_apply
    }

    // ————————————————————— 1) list —————————————————————

    /// 列表（Java `list`）。
    pub async fn list(
        &self,
        q: &UserWithdrawApplyListQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<UserWithdrawApplyVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let user_ids = match q.effective_keyword() {
            Some(keyword) => {
                let ids = self.repo().find_user_ids_by_username_like(keyword).await?;
                if ids.is_empty() {
                    return Ok(PageResult::empty(&page));
                }
                Some(ids)
            }
            None => None,
        };
        let (total, rows) = self
            .repo()
            .paginate(q, user_ids.as_deref(), shop_scope, limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let uids: Vec<i32> = distinct(rows.iter().map(|r| r.user_id));
        let users = UserWithdrawApplyRepository::index_users(self.repo().find_users(&uids).await?);
        let list: Vec<UserWithdrawApplyVO> = rows
            .iter()
            .map(|entity| to_vo(entity, &users, true))
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 2) detail —————————————————————

    /// 详情（Java `detail`）。
    pub async fn detail(
        &self,
        id: i32,
        shop_scope: Option<i32>,
    ) -> Result<UserWithdrawApplyVO> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        self.ensure_user_scope(entity.user_id, shop_scope).await?;
        let users = UserWithdrawApplyRepository::index_users(
            self.repo().find_users(&[entity.user_id]).await?,
        );
        Ok(to_vo(&entity, &users, true))
    }

    // ————————————————————— 3) create —————————————————————

    /// 创建提现申请（Java `create`：校验余额 → 落库 → 冻结 + 扣可用）。
    pub async fn create(
        &self,
        dto: &UserWithdrawApplyCreateDTO,
        shop_scope: Option<i32>,
    ) -> Result<i32> {
        let input: UserWithdrawApplyCreate = dto.validate().map_err(Error::Validation)?;
        self.ensure_user_scope(input.user_id, shop_scope).await?;
        let now = i32::try_from(now_seconds()).unwrap_or_default();
        let mut tx = self.repo().begin().await?;
        let balance = self
            .repo()
            .find_balance_in_tx(&mut tx, input.user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_USER_NOT_EXISTS))?;
        if balance.balance < input.amount {
            return Err(Error::biz(500, MSG_BALANCE_NOT_ENOUGH));
        }
        let entity = UserWithdrawApply {
            id: 0,
            user_id: input.user_id,
            wallet_type: WALLET_BALANCE,
            amount: input.amount,
            add_time: now,
            finished_time: input.finished_time,
            postscript: input.postscript.clone(),
            status: STATUS_WAIT,
            account_data: input.account_data_json.clone(),
        };
        let id = self.repo().insert_in_tx(&mut tx, &entity).await?;
        // 增加冻结余额
        self.repo()
            .apply_balance_change_in_tx(
                &mut tx,
                &BalanceChange {
                    user_id: input.user_id,
                    balance_amount: None,
                    frozen_amount: Some(input.amount),
                    change_type: 1,
                    change_desc: DESC_INC_FROZEN.to_string(),
                    change_time: now,
                },
            )
            .await?;
        // 减少可用余额
        self.repo()
            .apply_balance_change_in_tx(
                &mut tx,
                &BalanceChange {
                    user_id: input.user_id,
                    balance_amount: Some(input.amount),
                    frozen_amount: None,
                    change_type: 2,
                    change_desc: DESC_DEC_BALANCE.to_string(),
                    change_time: now,
                },
            )
            .await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("创建提现申请: id={id}, user_id={}", input.user_id);
        Ok(id)
    }

    // ————————————————————— 4) update —————————————————————

    /// 更新提现申请（Java `update`）。
    pub async fn update(
        &self,
        dto: &UserWithdrawApplyUpdateDTO,
        shop_scope: Option<i32>,
    ) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let existing = self
            .repo()
            .find_by_id(input.id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        self.ensure_user_scope(existing.user_id, shop_scope).await?;
        if self.repo().find_users(&[existing.user_id]).await?.is_empty() {
            return Err(Error::biz(500, MSG_USER_NOT_EXISTS));
        }
        if existing.status > STATUS_WAIT {
            return Err(Error::biz(500, MSG_ALREADY_FINISHED));
        }
        let now = i32::try_from(now_seconds()).unwrap_or_default();
        let merged = merge_withdraw_update(&existing, &input, now);
        let is_commission = existing.wallet_type == WALLET_COMMISSION;
        let amount = existing.amount;
        let user_id = existing.user_id;

        let mut tx = self.repo().begin().await?;
        self.repo().update(&merged).await?;
        if input.status == Some(1) {
            // 审核通过：扣减对应账户的冻结
            let change = BalanceChange {
                user_id,
                balance_amount: None,
                frozen_amount: Some(amount),
                change_type: 2,
                change_desc: if is_commission {
                    DESC_DEC_FROZEN_COMMISSION.to_string()
                } else {
                    DESC_DEC_FROZEN_BALANCE.to_string()
                },
                change_time: now,
            };
            if is_commission {
                self.repo().apply_commission_change_in_tx(&mut tx, &change).await?;
            } else {
                self.repo().apply_balance_change_in_tx(&mut tx, &change).await?;
            }
        } else {
            // 拒绝：返还可用 + 扣减冻结
            let inc = BalanceChange {
                user_id,
                balance_amount: Some(amount),
                frozen_amount: None,
                change_type: 1,
                change_desc: if is_commission {
                    DESC_INC_COMMISSION_REFUND.to_string()
                } else {
                    DESC_INC_BALANCE_REFUND.to_string()
                },
                change_time: now,
            };
            let dec = BalanceChange {
                user_id,
                balance_amount: None,
                frozen_amount: Some(amount),
                change_type: 2,
                change_desc: if is_commission {
                    DESC_DEC_FROZEN_COMMISSION_REFUND.to_string()
                } else {
                    DESC_DEC_FROZEN_REFUND.to_string()
                },
                change_time: now,
            };
            if is_commission {
                self.repo().apply_commission_change_in_tx(&mut tx, &inc).await?;
                self.repo().apply_commission_change_in_tx(&mut tx, &dec).await?;
            } else {
                self.repo().apply_balance_change_in_tx(&mut tx, &inc).await?;
                self.repo().apply_balance_change_in_tx(&mut tx, &dec).await?;
            }
        }
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!("更新提现申请: id={}", input.id);
        Ok(())
    }

    // ————————————————————— 5) del —————————————————————

    /// 删除（Java `del`：待处理禁止删除）。
    pub async fn delete(&self, dto: &OperateDTO, shop_scope: Option<i32>) -> Result<()> {
        let Some(id) = dto.id else {
            return Ok(());
        };
        if let Some(apply) = self.repo().find_by_id(id).await? {
            self.ensure_user_scope(apply.user_id, shop_scope).await?;
            if apply.status == STATUS_WAIT {
                return Err(Error::biz(500, MSG_PENDING_DELETE));
            }
        }
        let _ = self.repo().delete(id).await?;
        tracing::info!("删除提现申请: id={id}");
        Ok(())
    }

    // ————————————————————— 6) updateField —————————————————————

    /// 字段更新（Java `updateField(dto, GENERAL_FIELDS)`）。
    pub async fn update_field(
        &self,
        dto: &UpdateFieldDTO,
        shop_scope: Option<i32>,
    ) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        if let Some(entity) = self.repo().find_by_id(input.id).await? {
            self.ensure_user_scope(entity.user_id, shop_scope).await?;
        }
        self.repo()
            .update_field(input.id, input.column, &input.val)
            .await?;
        Ok(())
    }

    /// 校验提现申请人归属：店铺账号只能操作本店会员的提现。
    async fn ensure_user_scope(&self, user_id: i32, shop_scope: Option<i32>) -> Result<()> {
        if let Some(shop_id) = shop_scope {
            if !self.repo().user_in_shop(user_id, shop_id).await? {
                return Err(Error::Forbidden(format!(
                    "无权操作店铺 {shop_id} 会员 {user_id} 的提现申请"
                )));
            }
        }
        Ok(())
    }

    // ————————————————————— 7) batch —————————————————————

    /// 批量操作（Java `batch`：del 时含待处理整批拒绝）。
    pub async fn batch(&self, dto: &BatchDTO, shop_scope: Option<i32>) -> Result<bool> {
        let ids = dto.ids.as_deref().unwrap_or_default();
        for id in ids {
            if let Some(entity) = self.repo().find_by_id(*id).await? {
                self.ensure_user_scope(entity.user_id, shop_scope).await?;
            }
        }
        if dto.r#type.as_deref() == Some("del") && !ids.is_empty() {
            if self.repo().count_pending_by_ids(ids).await? > 0 {
                return Err(Error::biz(500, MSG_PENDING_DELETE));
            }
            let n = self.repo().delete_batch(ids).await?;
            tracing::info!("批量删除提现申请: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }
}

// ——————————————————————————————————————————————
// VO 装配
// ——————————————————————————————————————————————

/// 实体 → VO（`formatted=true` 时 `addTime` 格式化；Java list/detail 均格式化）。
#[must_use]
pub fn to_vo(
    entity: &UserWithdrawApply,
    users: &std::collections::HashMap<i32, WithdrawUserRow>,
    formatted: bool,
) -> UserWithdrawApplyVO {
    let username = users
        .get(&entity.user_id)
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "--".to_string());
    UserWithdrawApplyVO {
        id: entity.id,
        user_id: entity.user_id,
        wallet_type: entity.wallet_type,
        username: Some(username),
        amount: entity.amount,
        add_time: if formatted {
            format_time(entity.add_time)
        } else {
            entity.add_time.to_string()
        },
        finished_time: entity.finished_time,
        postscript: entity.postscript.clone(),
        status: entity.status,
        status_type: status_type(entity.status).to_string(),
        account_data: parse_account_data(&entity.account_data),
        handling_fee: None,
        handling_rate: None,
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
    let (size, offset) = UserWithdrawApplyRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::user_withdraw_apply::{
        MSG_ID_REQUIRED_DTO, MSG_POSTSCRIPT_TOO_LONG, STATUS_FINISHED,
    };

    fn entity() -> UserWithdrawApply {
        UserWithdrawApply {
            id: 9,
            user_id: 7,
            wallet_type: WALLET_BALANCE,
            amount: rust_decimal::Decimal::new(10000, 2),
            add_time: 1_700_000_000,
            finished_time: 0,
            postscript: "备注".into(),
            status: STATUS_WAIT,
            account_data: r#"{"accountName":"张三","accountNo":"6222"}"#.into(),
        }
    }

    fn users() -> std::collections::HashMap<i32, WithdrawUserRow> {
        let mut map = std::collections::HashMap::new();
        map.insert(
            7,
            WithdrawUserRow {
                user_id: 7,
                username: "alice".into(),
            },
        );
        map
    }

    #[test]
    fn vo_maps_username_and_account_data() {
        let vo = to_vo(&entity(), &users(), true);
        assert_eq!(vo.username.as_deref(), Some("alice"));
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.status_type, "待处理");
        assert_eq!(
            vo.account_data.as_ref().and_then(|a| a.account_no.clone()),
            Some("6222".into())
        );
        assert_eq!(vo.handling_fee, None);
        // 用户缺失 ⇒ "--"
        let vo = to_vo(&entity(), &std::collections::HashMap::new(), true);
        assert_eq!(vo.username.as_deref(), Some("--"));
        let vo = to_vo(&entity(), &users(), false);
        assert_eq!(vo.add_time, "1700000000");
    }

    #[test]
    fn validation_messages() {
        assert_eq!(
            UserWithdrawApplyUpdateDTO::default().validate().unwrap_err(),
            MSG_ID_REQUIRED_DTO
        );
        let dto = UserWithdrawApplyUpdateDTO {
            id: Some(9),
            postscript: Some("超".repeat(81)),
            status: None,
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_POSTSCRIPT_TOO_LONG);
        let dto = UserWithdrawApplyUpdateDTO {
            id: Some(9),
            postscript: None,
            status: Some(STATUS_FINISHED),
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn page_and_distinct() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(10));
        assert_eq!((p.page, p.page_size), (2, 10));
        assert_eq!(distinct([2, 1, 2].into_iter()), vec![2, 1]);
    }
}
