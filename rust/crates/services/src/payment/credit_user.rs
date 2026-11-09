//! 授信会员域服务（payment::credit_user）。
//!
//! 对应原 Java `CreditUserController`（6 端点）→ `CreditUserServiceImpl`
//! （list / detail / create / update / delete / updateField）。
//!
//! # 已完整实现（纯 DB）
//!
//! - `list`：会员类型（`user_company.type` 且 `status = 2`）/ 用户名 / 状态过滤，
//!   `create_time DESC` 排序，用户与企业认证装配。
//! - `detail`：`credit_user` + 用户 + 企业认证 + 账期天数。
//! - `create`：会员存在 → 已通过认证 → 非重复授信 → 账期类型合法，初始化
//!   `used_amount = 0` / `available_amount = creditLimit` / 周期起点 = 当天 0 点。
//! - `update`：额度调整（`available = limit - used`，不足报「信用额度不能小于
//!   已使用额度（…）」）、账期类型校验、状态；未传字段按 MyBatis-Plus NOT_NULL
//!   语义保留原值。
//! - `delete`：存在待结算账期订单（`credit_order.settlement_status = 0`）时拒绝。
//! - `updateField`：白名单仅 `status`，非法字段「field 的值有误」、非法状态值
//!   「无效的状态值」、不存在「授信会员不存在」。
//!
//! # 偏差（如实标注）
//!
//! 1. `@Sensitive(PHONE)` 仅 demo 模式脱敏；Rust 无 demo 开关 ⇒ 手机号原样返回。
//! 2. 列表排序：Java `buildSortOrder` 后追加 `create_time DESC`；本实现固定
//!    `create_time DESC` 主序、`sortField` 次序。
//! 3. `delete` 为物理删除（Java `removeById`，表无逻辑删除列）。
//! 4. `create` 的 `status` 为 null 时回退库默认 1（Java 由 MP 省略列 + DB 默认）。
//! 5. 并发创建同一会员由 `uk_user_id` 唯一索引兜底（数据库错误）。
//! 6. 货币名称读 `config.dollarSignCn`，缺失/空白回退「元」（Java
//!    `getCurrencyName`，但 Rust 无配置缓存层，直读库）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::CreditUser;
use nextrade_core::models::payment::credit_user::{
    build_credit_user_for_create, company_type_name, currency_name, merge_credit_user_update,
    settlement_type_name, settlement_type_days, status_text, validate_settlement_type,
    validate_status, CreditUserCreateDTO, CreditUserListQuery, CreditUserListVO,
    CreditUserUpdateDTO, CreditUserVO, OperateDTO, UpdateFieldDTO, CONFIG_DOLLAR_SIGN_CN,
    MSG_ALREADY_CREDIT_USER, MSG_COMPANY_NOT_VERIFIED, MSG_CREDIT_USER_NOT_FOUND,
    MSG_HAS_UNSETTLED_ORDERS, MSG_MEMBER_NOT_EXISTS, today_start_epoch,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::credit_user::{
    CreditUserCompanyRow, CreditUserRepository, CreditUserUserRow,
};
use nextrade_infrastructure::db::Repositories;

/// 授信会员域服务。
pub struct CreditUserService {
    repos: Arc<Repositories>,
}

impl CreditUserService {
    /// 构造服务（组合根唯一装配点）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &CreditUserRepository {
        &self.repos.payment.credit_user
    }

    // ————————————————————— 1) list —————————————————————

    /// 授信会员列表（Java `CreditUserServiceImpl#list`）。
    pub async fn list(
        &self,
        q: &CreditUserListQuery,
    ) -> Result<PageResult<CreditUserListVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();

        // 会员类型（user_company.type + status=2）
        let company_user_ids = match q.r#type {
            Some(company_type) => {
                let ids = self
                    .repo()
                    .find_user_ids_by_company_type(company_type)
                    .await?;
                if ids.is_empty() {
                    return Ok(PageResult::empty(&page));
                }
                Some(ids)
            }
            None => None,
        };
        // 用户名模糊
        let username_user_ids = match q.effective_username() {
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
            .paginate(
                q,
                company_user_ids.as_deref(),
                username_user_ids.as_deref(),
                limit,
                offset,
            )
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let user_ids = distinct(rows.iter().map(|u| u.user_id));
        let users: HashMap<i32, CreditUserUserRow> = self
            .repo()
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        let companies: HashMap<i32, CreditUserCompanyRow> = self
            .repo()
            .find_companies(&user_ids)
            .await?
            .into_iter()
            .map(|c| (c.user_id, c))
            .collect();

        let list: Vec<CreditUserListVO> = rows
            .iter()
            .map(|credit_user| to_list_vo(credit_user, &users, &companies))
            .collect();
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    // ————————————————————— 2) detail —————————————————————

    /// 授信会员详情（Java `detail`）。
    pub async fn detail(&self, credit_user_id: i32) -> Result<CreditUserVO> {
        let credit_user = self
            .repo()
            .find_by_id(credit_user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CREDIT_USER_NOT_FOUND))?;
        let users: HashMap<i32, CreditUserUserRow> = self
            .repo()
            .find_users(&[credit_user.user_id])
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        let companies: HashMap<i32, CreditUserCompanyRow> = self
            .repo()
            .find_companies(&[credit_user.user_id])
            .await?
            .into_iter()
            .map(|c| (c.user_id, c))
            .collect();
        Ok(to_detail_vo(&credit_user, &users, &companies))
    }

    // ————————————————————— 3) create —————————————————————

    /// 添加授信会员（Java `create`）。
    pub async fn create(&self, dto: &CreditUserCreateDTO) -> Result<i32> {
        let input = dto.validate().map_err(Error::Validation)?;

        // 会员必须存在
        if self.repo().find_users(&[input.user_id]).await?.is_empty() {
            return Err(Error::biz(500, MSG_MEMBER_NOT_EXISTS));
        }
        // 会员必须已通过企业认证（user_company.status = 2）
        if self
            .repo()
            .find_companies(&[input.user_id])
            .await?
            .is_empty()
        {
            return Err(Error::biz(500, MSG_COMPANY_NOT_VERIFIED));
        }
        // 不能重复授信
        if self
            .repo()
            .find_by_user_id(input.user_id)
            .await?
            .is_some()
        {
            return Err(Error::biz(500, MSG_ALREADY_CREDIT_USER));
        }
        // 账期类型合法（Java 在重复校验之后）
        validate_settlement_type(input.settlement_type).map_err(Error::biz_msg)?;

        let now = now_seconds();
        let entity = build_credit_user_for_create(&input, now, today_start_epoch());
        let id = self.repo().insert(&entity).await?;
        tracing::info!("添加授信会员: credit_user_id={id}, user_id={}", input.user_id);
        Ok(id)
    }

    // ————————————————————— 4) update —————————————————————

    /// 修改授信会员（Java `update`）。
    pub async fn update(&self, dto: &CreditUserUpdateDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        let existing = self
            .repo()
            .find_by_id(input.credit_user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CREDIT_USER_NOT_FOUND))?;

        // 货币名称（Java `configService.getCurrencyName()`）
        let currency = currency_name(
            self.repo()
                .config_value(CONFIG_DOLLAR_SIGN_CN)
                .await?
                .as_deref(),
        );
        // 额度校验优先（Java 先算 available，再校验账期类型）
        let merged = merge_credit_user_update(&existing, &input, now_seconds(), &currency)
            .map_err(Error::biz_msg)?;
        if let Some(settlement_type) = input.settlement_type {
            validate_settlement_type(settlement_type).map_err(|e| Error::biz(500, e))?;
        }
        self.repo().update(&merged).await?;
        tracing::info!("修改授信会员: credit_user_id={}", input.credit_user_id);
        Ok(())
    }

    // ————————————————————— 5) delete —————————————————————

    /// 删除授信会员（Java `delete`：有待结算账期订单时拒绝）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let credit_user_id = dto.validate_id().map_err(Error::Validation)?;
        self.repo()
            .find_by_id(credit_user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CREDIT_USER_NOT_FOUND))?;
        if self
            .repo()
            .count_unsettled_orders(credit_user_id)
            .await?
            > 0
        {
            return Err(Error::biz(500, MSG_HAS_UNSETTLED_ORDERS));
        }
        self.repo().delete(credit_user_id).await?;
        tracing::info!("删除授信会员: credit_user_id={credit_user_id}");
        Ok(())
    }

    // ————————————————————— 6) updateField —————————————————————

    /// 更新授信会员字段（Java `updateField` + `setStatus`）。
    ///
    /// 校验顺序与 Java 一致：`@Valid`（id/field/val）→ 白名单/状态值解析 →
    /// 存在性（「授信会员不存在」）→ 状态合法性（「无效的状态值」）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<()> {
        let input = dto.validate().map_err(Error::Validation)?;
        self.repo()
            .find_by_id(input.credit_user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CREDIT_USER_NOT_FOUND))?;
        let status = validate_status(input.status).map_err(|e| Error::biz(500, e))?;
        self.repo()
            .update_status(input.credit_user_id, status, now_seconds())
            .await?;
        tracing::info!(
            "更新授信会员状态: credit_user_id={}, status={status}",
            input.credit_user_id
        );
        Ok(())
    }

    /// 更新所有授信会员的账期起点（对应 Java `CreditUserServiceImpl#updateAllPeriodStartTime`）。
    ///
    /// 每天 0 点执行，根据当前时间与结算周期天数更新 `period_start_time`。
    pub async fn update_all_period_start_time(&self) -> Result<u64> {
        let users = self.repo().find_enabled_with_period().await?;
        if users.is_empty() {
            return Ok(0);
        }

        let now = now_seconds();
        let today_start = today_start_epoch();
        let mut updated = 0_u64;

        for user in users {
            let days = settlement_type_days(user.settlement_type) as i64;
            if days <= 0 {
                continue;
            }
            let Some(period_start) = user.period_start_time.filter(|&v| v > 0) else {
                continue;
            };

            // 经过的整天数
            let seconds_passed = today_start.saturating_sub(period_start);
            let days_passed = seconds_passed / 86400;

            if days_passed < days {
                // 还在第一个周期内
                continue;
            }

            let period_index = days_passed / days;
            let new_period_start = period_start + period_index * days * 86400;

            if new_period_start != period_start {
                if let Ok(affected) = self
                    .repo()
                    .update_period_start_time(user.credit_user_id, new_period_start, now)
                    .await
                {
                    if affected > 0 {
                        updated += 1;
                    }
                }
            }
        }

        tracing::info!("授信会员周期更新完成: 更新 {updated} 位会员");
        Ok(updated)
    }
}

// ——————————————————————————————————————————————
// VO 装配（纯函数，单测覆盖）
// ——————————————————————————————————————————————

/// 列表行 → VO。
#[must_use]
pub fn to_list_vo(
    credit_user: &CreditUser,
    users: &HashMap<i32, CreditUserUserRow>,
    companies: &HashMap<i32, CreditUserCompanyRow>,
) -> CreditUserListVO {
    let user = users.get(&credit_user.user_id);
    let company = companies.get(&credit_user.user_id);
    CreditUserListVO {
        credit_user_id: credit_user.credit_user_id,
        user_id: credit_user.user_id,
        username: user.map(|u| u.username.clone()),
        mobile: user.map(|u| u.mobile.clone()),
        r#type: company.map(|c| c.r#type),
        type_text: company.map(|c| company_type_name(c.r#type)),
        credit_limit: credit_user.credit_limit,
        used_amount: credit_user.used_amount,
        available_amount: credit_user.available_amount,
        settlement_type: credit_user.settlement_type,
        settlement_type_text: settlement_type_name(credit_user.settlement_type),
        contact_name: company.map(|c| c.contact_name.clone()),
        company_name: company.map(|c| c.company_name.clone()),
        status: credit_user.status,
        status_text: status_text(credit_user.status),
        create_time: format_handel_time(credit_user.create_time),
    }
}

/// 详情 → VO（多 `settlementDays` 与 `updateTime`）。
#[must_use]
pub fn to_detail_vo(
    credit_user: &CreditUser,
    users: &HashMap<i32, CreditUserUserRow>,
    companies: &HashMap<i32, CreditUserCompanyRow>,
) -> CreditUserVO {
    let user = users.get(&credit_user.user_id);
    let company = companies.get(&credit_user.user_id);
    CreditUserVO {
        credit_user_id: credit_user.credit_user_id,
        user_id: credit_user.user_id,
        username: user.map(|u| u.username.clone()),
        mobile: user.map(|u| u.mobile.clone()),
        r#type: company.map(|c| c.r#type),
        type_text: company.map(|c| company_type_name(c.r#type)),
        credit_limit: credit_user.credit_limit,
        used_amount: credit_user.used_amount,
        available_amount: credit_user.available_amount,
        settlement_type: credit_user.settlement_type,
        settlement_type_text: settlement_type_name(credit_user.settlement_type),
        settlement_days: settlement_type_days(credit_user.settlement_type),
        contact_name: company.map(|c| c.contact_name.clone()),
        company_name: company.map(|c| c.company_name.clone()),
        status: credit_user.status,
        status_text: status_text(credit_user.status),
        create_time: format_handel_time(credit_user.create_time),
        update_time: format_handel_time(credit_user.update_time),
    }
}

/// `TigUtils.handelTime` 透传（复用 settlement 域实现，保持同域口径一致）。
fn format_handel_time(secs: Option<i64>) -> String {
    nextrade_core::models::payment::credit_settlement::format_time(secs)
}

/// 去重（保持出现顺序）。
fn distinct(ids: impl Iterator<Item = i32>) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for id in ids {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// `BasePage` → `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = CreditUserRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use nextrade_core::models::payment::credit_user::{
        MSG_FIELD_VALUE_REQUIRED, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_INVALID_STATUS,
        MSG_SETTLEMENT_TYPE_REQUIRED,
    };

    fn credit_user() -> CreditUser {
        CreditUser {
            credit_user_id: 5,
            user_id: 7,
            credit_limit: Decimal::new(100000, 2),
            used_amount: Decimal::new(30000, 2),
            available_amount: Decimal::new(70000, 2),
            settlement_type: 1,
            status: 1,
            period_start_time: Some(1_768_406_400),
            create_time: Some(1_700_000_000),
            update_time: Some(1_700_000_100),
        }
    }

    fn users() -> HashMap<i32, CreditUserUserRow> {
        let mut map = HashMap::new();
        map.insert(
            7,
            CreditUserUserRow {
                user_id: 7,
                username: "alice".into(),
                mobile: "13800008000".into(),
            },
        );
        map
    }

    fn companies() -> HashMap<i32, CreditUserCompanyRow> {
        let mut map = HashMap::new();
        map.insert(
            7,
            CreditUserCompanyRow {
                user_id: 7,
                r#type: 2,
                contact_name: "张三".into(),
                company_name: "NexTrade".into(),
            },
        );
        map
    }

    #[test]
    fn list_vo_maps_fields_and_enums() {
        let vo = to_list_vo(&credit_user(), &users(), &companies());
        assert_eq!(vo.credit_user_id, 5);
        assert_eq!(vo.username.as_deref(), Some("alice"));
        assert_eq!(vo.mobile.as_deref(), Some("13800008000"));
        assert_eq!(vo.r#type, Some(2));
        assert_eq!(vo.type_text.as_deref(), Some("企业"));
        assert_eq!(vo.settlement_type_text, "月结");
        assert_eq!(vo.status_text, "启用");
        assert_eq!(vo.company_name.as_deref(), Some("NexTrade"));
        assert_eq!(vo.create_time, "2023-11-15 06:13:20");
        // 无用户/企业时不 panic，字段为 None
        let vo = to_list_vo(&credit_user(), &HashMap::new(), &HashMap::new());
        assert_eq!(vo.username, None);
        assert_eq!(vo.r#type, None);
        assert_eq!(vo.status_text, "启用");
    }

    #[test]
    fn detail_vo_maps_extra_fields() {
        let vo = to_detail_vo(&credit_user(), &users(), &companies());
        assert_eq!(vo.settlement_days, 30);
        assert_eq!(vo.update_time, "2023-11-15 06:15:00");
        assert_eq!(vo.available_amount, Decimal::new(70000, 2));
    }

    #[test]
    fn page_conversion_matches_base_page() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(20));
        assert_eq!((p.page, p.page_size), (2, 20));
        let p = to_page_query(Some(1), Some(500));
        assert_eq!((p.page, p.page_size), (1, 100));
    }

    #[test]
    fn distinct_keeps_order() {
        assert_eq!(distinct([3, 1, 3, 2].into_iter()), vec![3, 1, 2]);
    }

    #[test]
    fn validation_error_texts_are_verbatim() {
        assert_eq!(
            CreditUserCreateDTO::default().validate().unwrap_err(),
            "请选择会员"
        );
        assert_eq!(
            CreditUserUpdateDTO::default().validate().unwrap_err(),
            "授信会员ID不能为空"
        );
        assert_eq!(OperateDTO::default().validate_id().unwrap_err(), MSG_ID_REQUIRED);
        assert_eq!(
            UpdateFieldDTO::default().validate().unwrap_err(),
            MSG_ID_REQUIRED
        );
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("status".into()),
            val: None,
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_FIELD_VALUE_REQUIRED);
        let dto = UpdateFieldDTO {
            id: Some(1),
            field: Some("nickname".into()),
            val: Some("1".into()),
        };
        assert_eq!(dto.validate().unwrap_err(), MSG_INVALID_FIELD_VALUE);
        assert_eq!(
            CreditUserCreateDTO {
                user_id: Some(1),
                credit_limit: None,
                settlement_type: None,
                status: None,
            }
            .validate()
            .unwrap_err(),
            "请输入信用额度"
        );
        assert_eq!(
            CreditUserCreateDTO {
                user_id: Some(1),
                credit_limit: Some(Decimal::new(1, 2)),
                settlement_type: None,
                status: None,
            }
            .validate()
            .unwrap_err(),
            MSG_SETTLEMENT_TYPE_REQUIRED
        );
        assert_eq!(
            CreditUserCreateDTO {
                user_id: Some(1),
                credit_limit: Some(Decimal::ZERO),
                settlement_type: Some(1),
                status: None,
            }
            .validate()
            .unwrap_err(),
            "信用额度必须大于0"
        );
        assert_eq!(
            CreditUserUpdateDTO {
                credit_user_id: Some(5),
                credit_limit: Some(Decimal::new(0, 2)),
                settlement_type: None,
                status: None,
            }
            .validate()
            .unwrap_err(),
            "信用额度必须大于0"
        );
    }

    #[test]
    fn status_and_settlement_type_validation() {
        assert_eq!(validate_status(0).unwrap(), 0);
        assert_eq!(validate_status(1).unwrap(), 1);
        assert_eq!(validate_status(9).unwrap_err(), MSG_INVALID_STATUS);
        assert_eq!(validate_settlement_type(1).unwrap(), 1);
        assert_eq!(validate_settlement_type(2).unwrap(), 2);
        assert_eq!(
            validate_settlement_type(9).unwrap_err(),
            "请选择有效的结算账期类型"
        );
        assert_eq!(MSG_CREDIT_USER_NOT_FOUND, "授信会员不存在");
    }
}
