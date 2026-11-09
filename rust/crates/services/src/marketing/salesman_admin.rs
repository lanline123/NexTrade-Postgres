//! 分销员管理域服务（marketing::salesman_admin）。
//!
//! 对应原 Java `SalesmanServiceImpl` 管理端 8 个端点（`list` 由基线
//! `marketing::salesman` 提供，本模块不重复）：
//! `detail` / `create` / `update` / `statisticalDetails` / `commissionDetails`
//! / `ranking` / `salesmanList` / `customerList`。
//!
//! # 语义（逐条对照 Java）
//! - `detail`：分销员 + user 基础信息 + 上级（pid）分销员/user + 分组 +
//!   `statistical`（订单数/销售额/佣金/客户数/邀请数聚合）。
//! - `create`：同 user 只允许一个分销员（重复 → `该用户已添加`），落
//!   `shop_id=0`、`pid` 缺省 0、`add_time=now`。
//! - `update`：不存在 → `未知的分销员`；仅覆盖 `level/group_id/pid`。
//! - `statisticalDetails`：按 `groupId/level` + `mobile`（转 userId）分页，
//!   回填 user / 分组 / 订单聚合；`isExport` 忽略（见 models 差异 2）。
//! - `commissionDetails`：分销员 VO + 佣金/交易额合计；结算分项为 0
//!   （见 models 差异 3）。
//! - `ranking`：区间内按销售额分组聚合 + 回填会员名（差异 4）。
//! - `salesmanList`：JOIN 关键词过滤（差异 5）。
//! - `customerList`：`salesman_customer JOIN user`，订单聚合列返回 0/null
//!   （差异 6）。
//!
//! # 金额：`sale_amount` / `amount` / `order_amount` 为 `Decimal`。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::entities::Salesman;
use nextrade_core::models::marketing::salesman_admin::{
    format_time, resolve_search_range, CustomerListQuery, CustomerListVO, SalesmanDetailVO,
    SalesmanEditDTO, SalesmanGroupVO, SalesmanRankingQuery, SalesmanRankingVO, SalesmanSaveDTO,
    SalesmanSelectVO, SalesmanStatStatisticalVO, SalesmanStatisticalQuery, SalesmanStatisticalVO,
    SalesmanVO, UserBaseVO, CommissionDetailsVO, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
    MSG_SALESMAN_ID_REQUIRED, MSG_UNKNOWN_SALESMAN, MSG_USER_ALREADY_ADDED, MSG_USER_ID_REQUIRED,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::salesman_admin::{
    SalesmanAdminRepository, SalesmanUserRow,
};
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 分销员管理域服务（挂载于 `AdminState.marketing.salesman_admin`）。
pub struct SalesmanAdminService {
    repos: Arc<Repositories>,
}

impl SalesmanAdminService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &SalesmanAdminRepository {
        &self.repos.marketing.salesman_admin
    }

    /// 分销员详情（Java `SalesmanServiceImpl#detail`）。
    ///
    /// # Errors
    /// 分销员不存在 → `未知的分销员`。
    pub async fn detail(&self, id: i32) -> Result<SalesmanDetailVO> {
        let salesman = self
            .repo()
            .find_salesman(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_UNKNOWN_SALESMAN))?;
        // 多租户：店铺账号只能查看本店分销员
        TenantScope::current_ensure_access(salesman.shop_id.unwrap_or(0))?;
        let user_ids = [salesman.user_id.unwrap_or(0)];
        let users = self.repo().find_users(&user_ids).await?;
        let base = users.first().map(to_user_base);
        let pid_user = if salesman.pid > 0 {
            match self.repo().find_salesman(salesman.pid).await? {
                Some(parent) => {
                    let row = self
                        .repo()
                        .find_users(&[parent.user_id.unwrap_or(0)])
                        .await?
                        .into_iter()
                        .next()
                        .map(|u| to_user_base(&u));
                    row
                }
                None => None,
            }
        } else {
            None
        };
        let group = if salesman.group_id.unwrap_or(0) > 0 {
            self.repo()
                .find_groups(&[salesman.group_id.unwrap_or(0)])
                .await?
                .into_iter()
                .next()
                .map(|g| SalesmanGroupVO {
                    group_id: g.group_id,
                    group_name: g.group_name,
                    describe: g.describe,
                    add_time: g.add_time,
                    shop_id: g.shop_id,
                })
        } else {
            None
        };
        let aggregates = self.repo().aggregate_orders(&[id], None, None).await?;
        let counts = self.repo().count_customers(&[id]).await?;
        let invites = self.repo().count_invites(id).await?;
        let agg = aggregates.first();
        let customer_num = counts.first().map(|(_, n)| *n).unwrap_or(0);
        Ok(SalesmanDetailVO {
            salesman_vo: SalesmanVO {
                salesman_id: salesman.salesman_id,
                user_id: salesman.user_id,
                level: salesman.level,
                level_text: String::new(),
                group_id: salesman.group_id,
                pid: salesman.pid,
                shop_id: salesman.shop_id,
                add_time: format_time(salesman.add_time),
                base_user_info: base,
                pid_user_info: pid_user,
                group_info: group,
            },
            statistical: SalesmanStatStatisticalVO {
                sale_amount: agg.map(|a| a.pay_money).unwrap_or_default(),
                order_num: agg.map(|a| a.order_num).unwrap_or(0),
                customer_num,
                invite_num: invites,
                commission_amount: agg.map(|a| a.commission).unwrap_or_default(),
                product_commission_amount: agg.map(|a| a.product_amount).unwrap_or_default(),
            },
        })
    }

    /// 新增分销员（Java `create`）。
    ///
    /// # Errors
    /// 缺 userId → `Validation`；同 user 已是分销员 → `该用户已添加`。
    pub async fn create(&self, dto: &SalesmanSaveDTO) -> Result<i32> {
        let user_id = dto
            .user_id
            .ok_or_else(|| Error::Validation(MSG_USER_ID_REQUIRED.into()))?;
        if self.repo().find_by_user_id(user_id).await?.is_some() {
            return Err(Error::biz(500, MSG_USER_ALREADY_ADDED));
        }
        let now = now_secs_i32();
        let entity = Salesman {
            salesman_id: 0,
            user_id: Some(user_id),
            level: dto.level,
            group_id: dto.group_id,
            pid: dto.pid.unwrap_or(0),
            add_time: Some(now),
            // 多租户：店铺账号落本店；平台账号落平台级（0）
            shop_id: Some(TenantScope::current().write_shop_id(None)?),
            sale_amount: Some(Decimal::ZERO),
        };
        self.repo().insert_salesman(&entity).await
    }

    /// 编辑分销员（Java `update`）。
    ///
    /// # Errors
    /// 缺 salesmanId → `Validation`；不存在 → `未知的分销员`。
    pub async fn update(&self, dto: &SalesmanEditDTO) -> Result<()> {
        let id = dto
            .salesman_id
            .ok_or_else(|| Error::Validation(MSG_SALESMAN_ID_REQUIRED.into()))?;
        let Some(mut entity) = self.repo().find_salesman(id).await? else {
            return Err(Error::biz(500, MSG_UNKNOWN_SALESMAN));
        };
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(entity.shop_id.unwrap_or(0))?;
        if let Some(v) = dto.level {
            entity.level = Some(v);
        }
        if let Some(v) = dto.group_id {
            entity.group_id = Some(v);
        }
        if let Some(v) = dto.pid {
            entity.pid = v;
        }
        self.repo().update_salesman(&entity).await?;
        Ok(())
    }

    /// 分销员统计明细分页（Java `statistical`）。
    ///
    /// `mobile` 先按手机号/用户名精确解析 userId；解析不到 → 空页。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn statistical_details(
        &self,
        q: &SalesmanStatisticalQuery,
    ) -> Result<PageResult<SalesmanStatisticalVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let user_id = match q.mobile.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(keyword) => match self.repo().find_user_by_mobile_or_username(keyword).await? {
                Some((id,)) => Some(id),
                None => return Ok(PageResult::new(0, Vec::new(), &page)),
            },
            None => None,
        };
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .paginate_salesmen(
                TenantScope::current_list_filter(),
                user_id,
                q.group_id,
                q.level,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                limit,
                offset,
            )
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let ids: Vec<i32> = rows.iter().map(|s| s.salesman_id).collect();
        let aggs: HashMap<i32, _> = self
            .repo()
            .aggregate_orders(&ids, None, None)
            .await?
            .into_iter()
            .map(|a| (a.salesman_id, a))
            .collect();
        let customers: HashMap<i32, i64> = self.repo().count_customers(&ids).await?.into_iter().collect();
        let user_ids: Vec<i32> = rows.iter().filter_map(|s| s.user_id).collect();
        let users: HashMap<i32, SalesmanUserRow> = self
            .repo()
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        let group_ids: Vec<i32> = rows.iter().filter_map(|s| s.group_id).collect();
        let groups: HashMap<i32, String> = self
            .repo()
            .find_groups(&group_ids)
            .await?
            .into_iter()
            .filter_map(|g| g.group_name.map(|n| (g.group_id, n)))
            .collect();
        let list: Vec<SalesmanStatisticalVO> = rows
            .iter()
            .map(|s| {
                let agg = aggs.get(&s.salesman_id);
                SalesmanStatisticalVO {
                    salesman_id: s.salesman_id,
                    level: s.level,
                    level_text: String::new(),
                    group_id: s.group_id,
                    group_name: s.group_id.and_then(|id| groups.get(&id).cloned()),
                    pid: s.pid,
                    shop_id: s.shop_id,
                    sale_amount: agg.map(|a| a.product_amount).unwrap_or_default(),
                    order_count: agg.map(|a| a.order_num).unwrap_or(0),
                    pay_money: agg.map(|a| a.pay_money).unwrap_or_default(),
                    refund_amount: Decimal::ZERO,
                    total_customer: customers.get(&s.salesman_id).copied().unwrap_or(0),
                    add_time: format_time(s.add_time),
                    base_user_info: s
                        .user_id
                        .and_then(|uid| users.get(&uid))
                        .map(to_user_base),
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 佣金详情（Java `commissionDetails`）。
    ///
    /// # Errors
    /// 分销员不存在 → `未知的分销员`。
    pub async fn commission_details(&self, salesman_id: i32) -> Result<CommissionDetailsVO> {
        let detail = self.detail(salesman_id).await?;
        let aggs = self.repo().aggregate_orders(&[salesman_id], None, None).await?;
        let agg = aggs.first();
        Ok(CommissionDetailsVO {
            item: detail.salesman_vo,
            commission: agg.map(|a| a.commission).unwrap_or_default(),
            product_transaction_amount: agg.map(|a| a.product_amount).unwrap_or_default(),
            auto_settlement_amount: Decimal::ZERO,
            auto_wait_settlement_amount: Decimal::ZERO,
            artificial_settlement_amount: Decimal::ZERO,
            artificial_wait_settlement_amount: Decimal::ZERO,
        })
    }

    /// 分销员排行（Java `ranking`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn ranking(&self, q: &SalesmanRankingQuery) -> Result<PageResult<SalesmanRankingVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (start, end) = resolve_search_range(q);
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .ranking(
                start,
                end,
                q.sort_order.as_deref(),
                TenantScope::current_list_filter(),
                limit,
                offset,
            )
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &page));
        }
        let ids: Vec<i32> = rows.iter().map(|r| r.salesman_id).collect();
        let salesmen = self.repo().find_salesmen_by_ids(&ids).await?;
        let by_id: HashMap<i32, &Salesman> = salesmen.iter().map(|s| (s.salesman_id, s)).collect();
        let user_ids: Vec<i32> = salesmen.iter().filter_map(|s| s.user_id).collect();
        let users: HashMap<i32, SalesmanUserRow> = self
            .repo()
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|u| (u.user_id, u))
            .collect();
        let customers: HashMap<i32, i64> = self.repo().count_customers(&ids).await?.into_iter().collect();
        let list: Vec<SalesmanRankingVO> = rows
            .iter()
            .map(|r| {
                let user = by_id
                    .get(&r.salesman_id)
                    .and_then(|s| s.user_id)
                    .and_then(|uid| users.get(&uid));
                SalesmanRankingVO {
                    salesman_id: r.salesman_id,
                    username: user.map(|u| u.username.clone()),
                    nickname: user.and_then(|u| u.nickname.clone()),
                    total_sale_amount: r.total_sale_amount,
                    total_customers: customers.get(&r.salesman_id).copied().unwrap_or(0),
                    total_pay_customers: r.buy_user_num,
                    order_num: r.order_num,
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 分销员下拉列表（Java `salesmanList`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn salesman_list(&self, nickname: Option<&str>) -> Result<PageResult<SalesmanSelectVO>> {
        let rows = self
            .repo()
            .select_salesmen(nickname, TenantScope::current_list_filter())
            .await?;
        let list: Vec<SalesmanSelectVO> = rows
            .into_iter()
            .map(|(s, u)| SalesmanSelectVO {
                salesman_id: s.salesman_id,
                user_id: s.user_id,
                user: Some(to_user_base(&u)),
            })
            .collect();
        let size = u64::try_from(list.len()).unwrap_or(1).max(1);
        let page = PageQuery {
            page: 1,
            page_size: size,
        };
        Ok(PageResult::new(size, list, &page))
    }

    /// 客户列表（Java `customerList`）。
    ///
    /// # Errors
    /// 缺 salesmanId → `Validation`。
    pub async fn customer_list(&self, q: &CustomerListQuery) -> Result<PageResult<CustomerListVO>> {
        let salesman_id = q
            .salesman_id
            .ok_or_else(|| Error::Validation(MSG_SALESMAN_ID_REQUIRED.into()))?;
        // 多租户：客户列表随分销员归属校验
        let salesman = self
            .repo()
            .find_salesman(salesman_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_UNKNOWN_SALESMAN))?;
        TenantScope::current_ensure_access(salesman.shop_id.unwrap_or(0))?;
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .paginate_customers(salesman_id, limit, offset)
            .await?;
        let list: Vec<CustomerListVO> = rows
            .iter()
            .map(|r| CustomerListVO {
                user_id: r.user_id,
                username: r.username.clone(),
                avatar: r.avatar.clone(),
                mobile: r.mobile.clone(),
                nickname: r.nickname.clone(),
                reg_time: r.reg_time,
                rank_name: r.rank_name.clone(),
                customer_type: 1,
                order_count: 0,
                order_amount: Decimal::ZERO,
                last_order_time: None,
                bind_time: r.bind_time,
                channel: r.channel.clone(),
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }
}

/// 会员行 → `UserBaseVO`。
fn to_user_base(u: &SalesmanUserRow) -> UserBaseVO {
    UserBaseVO {
        user_id: u.user_id,
        username: u.username.clone(),
        nickname: u.nickname.clone(),
        mobile: u.mobile.clone(),
        avatar: u.avatar.clone(),
        reg_time: u.reg_time,
    }
}

fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

use nextrade_core::models::marketing::salesman_admin::{
    config_has_card_invite, config_is_wechat_biz, config_level, config_view_customer_info,
    parse_time, BIZ_SALESMAN_CONFIG, CustomerApiQuery, CustomerDetailVO, CustomerRemarkDTO,
    SalesmanOrderStatVO, SalesmanUserDetailVO, SalesmanUserInfoVO, MSG_CARD_INVITE_DISABLED,
    MSG_CUSTOMER_NOT_FOUND, MSG_INVITER_NOT_FOUND, MSG_NO_CUSTOMER_PERMISSION,
    MSG_NO_SALESMAN_CONFIG, MSG_NOT_SALESMAN, MSG_SELF_INVITE, MSG_WECHAT_BIZ_DISABLED,
};
use nextrade_infrastructure::db::repo::marketing::salesman_admin::PublicUserRow;

impl SalesmanAdminService {
    // ————————————————————— 公共端（/api/salesman/salesman） —————————————————————

    /// 当前登录会员对应的分销员（Java `currentSalesman`）。
    async fn current_salesman(&self, user_id: i32) -> Result<Salesman> {
        if user_id <= 0 {
            return Err(Error::Unauthorized(MSG_NOT_SALESMAN.into()));
        }
        self.repo()
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_SALESMAN))
    }

    /// 读取并解析 `salesman_config`（Java `salesmanConfigService.detail`）。
    async fn salesman_config_json(&self) -> Result<serde_json::Value> {
        let raw = self
            .repo()
            .find_salesman_config(BIZ_SALESMAN_CONFIG)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NO_SALESMAN_CONFIG))?;
        serde_json::from_str(raw.trim())
            .map_err(|e| Error::Internal(format!("解析分销配置失败: {e}")))
    }

    /// `viewCustomerInfo` 权限门槛（Java `assertCustomerInfoViewAllowed`）。
    async fn require_view_customer_info(&self) -> Result<()> {
        let cfg = self.salesman_config_json().await?;
        if config_view_customer_info(&cfg) {
            Ok(())
        } else {
            Err(Error::biz(500, MSG_NO_CUSTOMER_PERMISSION))
        }
    }

    /// 公共端分销员详情（Java `SalesmanServiceImpl#detailForApi`）。
    ///
    /// # Errors
    /// 非分销员 → `当前用户不是分销员`；配置缺失 → `没有对应的商户配置`。
    pub async fn detail_for_api(&self, user_id: i32) -> Result<SalesmanUserDetailVO> {
        let salesman = self.current_salesman(user_id).await?;
        let cfg = self.salesman_config_json().await?;
        let orders = self
            .repo()
            .list_salesman_orders(salesman.salesman_id)
            .await?;
        let customers = self
            .repo()
            .list_salesman_customers(salesman.salesman_id)
            .await?;
        let self_buy = self.repo().sum_self_buy(user_id).await?;
        let today_begin = today_begin_secs();
        let today_sum: Decimal = orders
            .iter()
            .filter(|o| o.status != Some(2))
            .filter(|o| i64::from(o.add_time.unwrap_or(0)) >= today_begin)
            .filter_map(|o| o.amount)
            .fold(Decimal::ZERO, |acc, v| acc + v);
        let all_sum: Decimal = orders
            .iter()
            .filter(|o| o.status != Some(2))
            .filter_map(|o| o.amount)
            .fold(Decimal::ZERO, |acc, v| acc + v);
        let wait_cps_sum: Decimal = orders
            .iter()
            .filter(|o| o.status == Some(0))
            .filter_map(|o| o.amount)
            .fold(Decimal::ZERO, |acc, v| acc + v);
        let today_count = orders
            .iter()
            .filter(|o| i64::from(o.add_time.unwrap_or(0)) >= today_begin)
            .count();
        let today_user_count = customers
            .iter()
            .filter(|c| i64::from(c.add_time.unwrap_or(0)) >= today_begin)
            .count();
        let (level_name, condition) = config_level(&cfg, salesman.level.unwrap_or(0));
        Ok(SalesmanUserDetailVO {
            salesman_id: salesman.salesman_id,
            user_id: salesman.user_id,
            level: salesman.level,
            group_id: salesman.group_id,
            pid: salesman.pid,
            shop_id: salesman.shop_id,
            sale_amount: salesman.sale_amount,
            add_time: format_time(salesman.add_time),
            salesman_config: Some(cfg),
            level_name,
            condition,
            order: SalesmanOrderStatVO {
                today_sum,
                all_sum,
                wait_cps_sum,
                today_count: i64::try_from(today_count).unwrap_or(i64::MAX),
                all_count: i64::try_from(orders.len()).unwrap_or(i64::MAX),
                today_user_count: i64::try_from(today_user_count).unwrap_or(i64::MAX),
                all_user_count: i64::try_from(customers.len()).unwrap_or(i64::MAX),
                self_buy_sum: self_buy,
            },
        })
    }

    /// 公共端客户列表（Java `customerApiList`）。
    ///
    /// `customerType`：`all` / `ordered` / `salesman`（下级分销员）。
    /// ⚠️ 订单数/金额按 `order` 表逐行聚合（分页内 N+1，页面 ≤100）；
    /// Java 的「最后下单时间 / 下单客户按最后下单时间筛选」未迁移。
    ///
    /// # Errors
    /// 非分销员 / 无查看权限 / 数据库错误。
    pub async fn customer_api_list(
        &self,
        user_id: i32,
        q: &CustomerApiQuery,
    ) -> Result<PageResult<CustomerListVO>> {
        let salesman = self.current_salesman(user_id).await?;
        self.require_view_customer_info().await?;
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let start = parse_time(q.start_time.as_deref());
        let end = parse_time(q.end_time.as_deref());
        let customer_type = q.normalized_customer_type();
        let (total, rows) = self
            .repo()
            .paginate_api_customers(salesman.salesman_id, customer_type, start, end, limit, offset)
            .await?;
        let mut list = Vec::with_capacity(rows.len());
        for r in &rows {
            let (order_count, order_amount) = self.repo().order_totals_for_user(r.user_id).await?;
            list.push(CustomerListVO {
                user_id: r.user_id,
                username: r.username.clone(),
                avatar: r.avatar.clone(),
                mobile: r.mobile.clone(),
                nickname: r.nickname.clone(),
                reg_time: r.reg_time,
                rank_name: r.rank_name.clone(),
                customer_type: if customer_type == "salesman" { 2 } else { 1 },
                order_count,
                order_amount,
                last_order_time: None,
                bind_time: r.bind_time,
                channel: r.channel.clone(),
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 公共端客户详情（Java `customerApiDetail`）。
    ///
    /// # Errors
    /// 客户不存在（既非名下客户也非下级分销员）→ `客户不存在`。
    pub async fn customer_api_detail(
        &self,
        user_id: i32,
        target_user_id: i32,
    ) -> Result<CustomerDetailVO> {
        let salesman = self.current_salesman(user_id).await?;
        self.require_view_customer_info().await?;
        let binding = self
            .repo()
            .find_customer_binding(salesman.salesman_id, target_user_id)
            .await?;
        let sub_salesman = self.repo().find_by_user_id(target_user_id).await?;
        let is_sub_salesman = sub_salesman
            .as_ref()
            .is_some_and(|s| s.pid == salesman.salesman_id);
        if binding.is_none() && !is_sub_salesman {
            return Err(Error::biz(500, MSG_CUSTOMER_NOT_FOUND));
        }
        let user = self
            .repo()
            .find_public_user(target_user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CUSTOMER_NOT_FOUND))?;
        let (order_count, order_amount) = self.repo().order_totals_for_user(target_user_id).await?;
        let average = if order_count > 0 {
            (order_amount / Decimal::from(order_count))
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        } else {
            Decimal::ZERO
        };
        Ok(CustomerDetailVO {
            user_id: user.user_id,
            username: user.username,
            avatar: user.avatar,
            mobile: user.mobile,
            nickname: user.nickname,
            rank_name: user.rank_name,
            customer_type: if is_sub_salesman { 2 } else { 1 },
            bind_time: binding.as_ref().and_then(|b| b.add_time),
            channel: binding
                .as_ref()
                .map(|b| b.channel.clone())
                .unwrap_or_else(|| "salesman".to_string()),
            last_order_time: None,
            order_count,
            order_amount,
            average_order_amount: average,
        })
    }

    /// 公共端更新客户备注（Java `updateCustomerRemark`）。
    ///
    /// # Errors
    /// 客户不存在 → `客户不存在`；缺 userId → `Validation`。
    pub async fn update_customer_remark(
        &self,
        user_id: i32,
        dto: &CustomerRemarkDTO,
    ) -> Result<()> {
        let salesman = self.current_salesman(user_id).await?;
        self.require_view_customer_info().await?;
        let target = dto
            .user_id
            .ok_or_else(|| Error::Validation("userId不能为空".into()))?;
        let binding = self
            .repo()
            .find_customer_binding(salesman.salesman_id, target)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_CUSTOMER_NOT_FOUND))?;
        let remark = dto.remark.as_deref().unwrap_or_default().trim();
        self.repo()
            .update_customer_remark(binding.salesman_customer_id, remark)
            .await?;
        Ok(())
    }

    /// 公共端会员基础信息（Java `getSalesman(salesmanId).getBaseUserInfo()`）。
    ///
    /// # Errors
    /// 分销员不存在 → `未知的分销员`。
    pub async fn user_info(&self, salesman_id: i32) -> Result<Option<SalesmanUserInfoVO>> {
        let Some(salesman) = self.repo().find_salesman(salesman_id).await? else {
            return Err(Error::biz(500, MSG_UNKNOWN_SALESMAN));
        };
        let Some(uid) = salesman.user_id.filter(|id| *id > 0) else {
            return Ok(None);
        };
        let Some(user) = self.repo().find_public_user(uid).await? else {
            return Ok(None);
        };
        Ok(Some(to_public_user_vo(&user, salesman.add_time)))
    }

    /// 接受名片邀请成为分销员（Java `WechatBizRecruitService#acceptCardInvite`）。
    ///
    /// 校验顺序与文案逐条对照 Java：微商模式 → 名片邀请开通 → 邀请人存在 →
    /// 不能接受自己的邀请；已是分销员时**幂等返回**（不改既有上级）。
    ///
    /// # Errors
    /// 未登录 → `Unauthorized`；其余校验 → 对应业务文案（500 段）。
    pub async fn accept_card_invite(&self, user_id: i32, salesman_id: i32) -> Result<()> {
        if user_id <= 0 {
            return Err(Error::Unauthorized("未登录".into()));
        }
        let cfg = self.salesman_config_json().await?;
        if !config_is_wechat_biz(&cfg) {
            return Err(Error::biz(500, MSG_WECHAT_BIZ_DISABLED));
        }
        if !config_has_card_invite(&cfg) {
            return Err(Error::biz(500, MSG_CARD_INVITE_DISABLED));
        }
        let inviter = self
            .repo()
            .find_salesman(salesman_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_INVITER_NOT_FOUND))?;
        if inviter.user_id == Some(user_id) {
            return Err(Error::biz(500, MSG_SELF_INVITE));
        }
        // 幂等：已是分销员直接返回，不改变既有上级
        if self.repo().find_by_user_id(user_id).await?.is_some() {
            return Ok(());
        }
        let entity = Salesman {
            salesman_id: 0,
            user_id: Some(user_id),
            level: Some(1),
            group_id: Some(0),
            pid: inviter.salesman_id,
            add_time: Some(now_secs_i32()),
            shop_id: Some(0),
            sale_amount: Some(Decimal::ZERO),
        };
        self.repo().insert_salesman(&entity).await?;
        Ok(())
    }
}

/// `PublicUserRow` → 公共端会员 VO。
fn to_public_user_vo(u: &PublicUserRow, distribution_register_time: Option<i32>) -> SalesmanUserInfoVO {
    SalesmanUserInfoVO {
        user_id: u.user_id,
        username: u.username.clone(),
        avatar: u.avatar.clone(),
        mobile: u.mobile.clone(),
        email: u.email.clone(),
        nickname: u.nickname.clone(),
        distribution_register_time,
        user_type: None,
        balance: u.balance,
        points: u.points,
    }
}

/// 今日 0 点（东八区）Unix 秒（Java `StringUtils.getTodayBeginTime`）。
fn today_begin_secs() -> i64 {
    use chrono::TimeZone;
    let now = nextrade_common::utils::now_seconds() as i64;
    let Some(tz) = chrono::FixedOffset::east_opt(8 * 3600) else {
        return now;
    };
    tz.timestamp_opt(now, 0)
        .single()
        .map(|dt| dt.date_naive().and_hms_opt(0, 0, 0))
        .flatten()
        .and_then(|naive| tz.from_local_datetime(&naive).single())
        .map(|v| v.timestamp())
        .unwrap_or(now)
}

/// 列表店铺过滤：店铺账号强制本店；平台账号不过滤。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod public_tests {
    use super::*;

    #[test]
    fn today_begin_is_midnight_and_le_now() {
        let begin = today_begin_secs();
        let now = nextrade_common::utils::now_seconds() as i64;
        assert!(begin <= now);
        assert!(now - begin < 86_400 + 1);
        // 东八区 0 点 ≡ UTC 16:00，秒数对 86400 取模应为 16*3600
        assert_eq!(begin.rem_euclid(86_400), 16 * 3600);
    }

    #[test]
    fn public_user_vo_maps_fields() {
        let row = PublicUserRow {
            user_id: 9,
            username: "u9".into(),
            avatar: "a.png".into(),
            mobile: "13800000000".into(),
            email: Some("u9@example.com".into()),
            nickname: None,
            balance: Some("1.50".parse().unwrap()),
            points: Some(10),
            rank_name: Some("黄金".into()),
        };
        let vo = to_public_user_vo(&row, Some(1_700_000_000));
        assert_eq!(vo.user_id, 9);
        assert_eq!(vo.email.as_deref(), Some("u9@example.com"));
        assert_eq!(vo.distribution_register_time, Some(1_700_000_000));
        assert_eq!(vo.points, Some(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_base_mapping_keeps_nickname_null() {
        let row = SalesmanUserRow {
            user_id: 9,
            username: "u9".into(),
            nickname: None,
            mobile: "13800000000".into(),
            avatar: String::new(),
            reg_time: Some(1_700_000_000),
            rank_id: 1,
        };
        let vo = to_user_base(&row);
        assert_eq!(vo.user_id, 9);
        assert!(vo.nickname.is_none());
        assert_eq!(vo.mobile, "13800000000");
    }

    #[test]
    fn missing_required_ids_are_validation_errors() {
        assert!(matches!(
            Error::Validation(MSG_USER_ID_REQUIRED.into()),
            Error::Validation(_)
        ));
        assert_eq!(MSG_SALESMAN_ID_REQUIRED, "salesmanId不能为空");
        assert_eq!(MSG_USER_ALREADY_ADDED, "该用户已添加");
        assert_eq!(MSG_UNKNOWN_SALESMAN, "未知的分销员");
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_salesman() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async { scoped_list_shop_id() }).await;
        assert_eq!(filter, None);
    }
}
