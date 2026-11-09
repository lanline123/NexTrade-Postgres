//! 分销员管理域（marketing::salesman_admin）仓储。
//!
//! 对应原 Java `SalesmanMapper` / `SalesmanOrderMapper`（含 `ranking` XML）/
//! `SalesmanCustomerMapper`（含 `customerList` XML）/ `SalesmanGroupMapper` /
//! `UserMapper`，服务实现见 `SalesmanServiceImpl`。
//!
//! 表 / 列（`python tools/schema_parser.py --table <表名>`）：
//! - `salesman`（8）：`salesman_id` / `user_id` / `level` / `group_id` /
//!   `pid` / `add_time` / `shop_id` / `sale_amount`
//! - `salesman_order`（12）：本域只读/聚合 `order_id` / `salesman_id` /
//!   `amount` / `status` / `add_time` / `order_amount` / `settlement_time`
//! - `salesman_customer`（6）：`salesman_customer_id` / `salesman_id` /
//!   `user_id` / `add_time` / `channel` / `remark`
//! - `salesman_group`（5）：`group_id` / `group_name` / `describe` /
//!   `add_time` / `shop_id`
//! - `user` / `user_rank`：只读回填（会员基础信息、等级名）
//!
//! 查询统一使用运行时 `sqlx::query_as`；所有聚合都带 `salesman_id IN (...)`
//! 批量条件，避免 N+1。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};

use nextrade_core::entities::{Salesman, SalesmanCustomer, SalesmanGroup, SalesmanOrder};
use nextrade_core::models::marketing::salesman_admin::MSG_UNKNOWN_SALESMAN;
use nextrade_core::{Error, Result};

/// `salesman` 固定 SELECT 列（8 列）。
pub const SALESMAN_COLUMNS: &str =
    "salesman_id, user_id, level, group_id, pid, add_time, shop_id, sale_amount";
/// `salesman_order` 固定 SELECT 列（12 列）。
pub const SALESMAN_ORDER_COLUMNS: &str = "salesman_order_id, order_id, salesman_id, amount, status, add_time, item_id, salesman_product_data, order_amount, salesman_settlement_data, settlement_time, product_id";
/// `salesman_customer` 固定 SELECT 列（6 列）。
pub const SALESMAN_CUSTOMER_COLUMNS: &str =
    "salesman_customer_id, salesman_id, user_id, add_time, channel, remark";
/// `salesman_group` 固定 SELECT 列（5 列）。
///
/// ⚠️ `describe` 是 MySQL 保留字，必须加反引号
/// （2026-10-21 `/adminapi/salesman/group/list` 500 的根因）。
pub const SALESMAN_GROUP_COLUMNS: &str = "group_id, group_name, \"describe\", add_time, shop_id";

/// 会员基础信息行。
#[derive(Debug, Clone, Default, FromRow)]
pub struct SalesmanUserRow {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub mobile: String,
    pub avatar: String,
    pub reg_time: Option<i32>,
    pub rank_id: i32,
}

/// 聚合统计行（按 salesman_id 聚合 `salesman_order`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct SalesmanAggRow {
    pub salesman_id: i32,
    pub order_num: i64,
    pub pay_money: Decimal,
    pub commission: Decimal,
    pub product_amount: Decimal,
}

/// 排行聚合行（`salesman_order` group by salesman）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct RankingAggRow {
    pub salesman_id: i32,
    pub total_sale_amount: Decimal,
    pub order_num: i64,
    pub buy_user_num: i64,
}

/// 客户列表行（`salesman_customer JOIN user LEFT JOIN user_rank`）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct CustomerRow {
    pub user_id: i32,
    pub username: String,
    pub avatar: String,
    pub mobile: String,
    pub nickname: Option<String>,
    pub reg_time: Option<i32>,
    pub rank_name: Option<String>,
    pub channel: String,
    pub bind_time: Option<i32>,
}

/// 分销员管理仓储。
#[derive(Clone)]
pub struct SalesmanAdminRepository {
    pool: PgPool,
}

impl SalesmanAdminRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— salesman —————————————————————

    /// 按 ID 查询分销员（Java `getById`）。
    pub async fn find_salesman(&self, salesman_id: i32) -> Result<Option<Salesman>> {
        let sql = format!("SELECT {SALESMAN_COLUMNS} FROM salesman WHERE salesman_id = $1 LIMIT 1");
        sqlx::query_as::<_, Salesman>(&sql)
            .bind(salesman_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按 user_id 查询分销员（Java `eq(userId).count()` / `getOne`）。
    pub async fn find_by_user_id(&self, user_id: i32) -> Result<Option<Salesman>> {
        let sql = format!("SELECT {SALESMAN_COLUMNS} FROM salesman WHERE user_id = $1 LIMIT 1");
        sqlx::query_as::<_, Salesman>(&sql)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量按 ID 取分销员（排行回填 user）。
    pub async fn find_salesmen_by_ids(&self, ids: &[i32]) -> Result<Vec<Salesman>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {SALESMAN_COLUMNS} FROM salesman WHERE salesman_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, Salesman>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 统计明细分页（Java `statistical` 的 salesman 分页条件）。
    pub async fn paginate_salesmen(
        &self,
        shop_id: Option<i32>,
        user_id: Option<i32>,
        group_id: Option<i32>,
        level: Option<i32>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<Salesman>)> {
        let mut where_parts: Vec<String> = vec!["1 = 1".into()];
        if shop_id.is_some() {
            where_parts.push("shop_id = ?".into());
        }
        if user_id.is_some() {
            where_parts.push("user_id = ?".into());
        }
        if group_id.is_some() {
            where_parts.push("group_id = ?".into());
        }
        if level.is_some() {
            where_parts.push("level = ?".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let column = resolve_sort_column(sort_field);
        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM salesman {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_salesman_filters(cq, shop_id, user_id, group_id, level);
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {SALESMAN_COLUMNS} FROM salesman {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, Salesman>(&list_sql);
        lq = bind_salesman_filters(lq, shop_id, user_id, group_id, level);
        let rows = lq
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 新增分销员（Java `save`）。
    pub async fn insert_salesman(&self, s: &Salesman) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO salesman (user_id, level, group_id, pid, add_time, shop_id, sale_amount) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING salesman_id",
        )
        .bind(s.user_id)
        .bind(s.level)
        .bind(s.group_id)
        .bind(s.pid)
        .bind(s.add_time)
        .bind(s.shop_id)
        .bind(s.sale_amount)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新分销员（Java `updateById`）。
    pub async fn update_salesman(&self, s: &Salesman) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE salesman SET user_id = $1, level = $2, group_id = $3, pid = $4, add_time = $5, shop_id = $6, sale_amount = $7 WHERE salesman_id = $8",
        )
        .bind(s.user_id)
        .bind(s.level)
        .bind(s.group_id)
        .bind(s.pid)
        .bind(s.add_time)
        .bind(s.shop_id)
        .bind(s.sale_amount)
        .bind(s.salesman_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    // ————————————————————— user / group 回填 —————————————————————

    /// 批量会员基础信息。
    pub async fn find_users(&self, user_ids: &[i32]) -> Result<Vec<SalesmanUserRow>> {
        let ids: Vec<i32> = user_ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_id, username, nickname, mobile, avatar, reg_time, rank_id FROM \"user\" WHERE user_id IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, SalesmanUserRow>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按手机号/用户名精确查会员（Java `getUserByMobileOrUsername`）。
    pub async fn find_user_by_mobile_or_username(&self, keyword: &str) -> Result<Option<(i32,)>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT user_id FROM \"user\" WHERE mobile = $1 OR username = $2 LIMIT 1",
        )
        .bind(keyword)
        .bind(keyword)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 批量分组。
    pub async fn find_groups(&self, ids: &[i32]) -> Result<Vec<SalesmanGroup>> {
        let ids: Vec<i32> = ids.iter().copied().filter(|id| *id > 0).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT {SALESMAN_GROUP_COLUMNS} FROM salesman_group WHERE group_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, SalesmanGroup>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 聚合 —————————————————————

    /// 按分销员批量聚合订单（Java `statistical`/`commissionDetails` 的统计口径）：
    /// `ORDER BY` 无（纯 group by）；时间范围可选。
    pub async fn aggregate_orders(
        &self,
        salesman_ids: &[i32],
        start: Option<i64>,
        end: Option<i64>,
    ) -> Result<Vec<SalesmanAggRow>> {
        if salesman_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; salesman_ids.len()].join(", ");
        let mut where_parts = vec![format!("salesman_id IN ({ph})")];
        if start.is_some() {
            where_parts.push("add_time >= ?".into());
        }
        if end.is_some() {
            where_parts.push("add_time <= ?".into());
        }
        let sql = format!(
            "SELECT salesman_id, COUNT(DISTINCT order_id) AS order_num, COALESCE(SUM(amount),0) AS pay_money, COALESCE(SUM(amount),0) AS commission, COALESCE(SUM(order_amount),0) AS product_amount FROM salesman_order WHERE {} GROUP BY salesman_id",
            where_parts.join(" AND ")
        );
        let mut q = sqlx::query_as::<_, SalesmanAggRow>(&sql);
        for id in salesman_ids {
            q = q.bind(*id);
        }
        if let Some(v) = start {
            q = q.bind(v);
        }
        if let Some(v) = end {
            q = q.bind(v);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量统计客户数（Java 客户数口径）。
    pub async fn count_customers(&self, salesman_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if salesman_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; salesman_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT salesman_id, COUNT(*) FROM salesman_customer WHERE salesman_id IN ({ph}) GROUP BY salesman_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in salesman_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量统计邀请数（`salesman.pid = 该分销员` 的下级数）。
    pub async fn count_invites(&self, salesman_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM salesman WHERE pid = $1")
            .bind(salesman_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
    }

    /// 排行聚合（Java `SalesmanOrderMapper#ranking` 的等价 group by）：
    /// 区间内按 salesman 汇总销售额/订单数/购买客户数（订单维度去重）。
    pub async fn ranking(
        &self,
        start: Option<i64>,
        end: Option<i64>,
        sort_order: Option<&str>,
        shop_id: Option<i32>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<RankingAggRow>)> {
        let mut where_parts: Vec<String> = vec!["1 = 1".into()];
        if start.is_some() {
            where_parts.push("add_time >= ?".into());
        }
        if end.is_some() {
            where_parts.push("add_time <= ?".into());
        }
        // 多租户：店铺账号只看本店分销员的订单聚合（i32 内部值）
        if let Some(shop_id) = shop_id {
            where_parts.push(format!(
                "salesman_id IN (SELECT salesman_id FROM salesman WHERE shop_id = {shop_id})"
            ));
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(DISTINCT salesman_id) FROM salesman_order {where_sql}"
        ));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(v) = start {
            cq = cq.bind(v);
        }
        if let Some(v) = end {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let dir = if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT salesman_id, COALESCE(SUM(amount),0) AS total_sale_amount, COUNT(DISTINCT order_id) AS order_num, COUNT(DISTINCT order_id) AS buy_user_num FROM salesman_order {where_sql} GROUP BY salesman_id ORDER BY total_sale_amount {dir} LIMIT ? OFFSET ?"
        ));
        let mut q = sqlx::query_as::<_, RankingAggRow>(&sql);
        if let Some(v) = start {
            q = q.bind(v);
        }
        if let Some(v) = end {
            q = q.bind(v);
        }
        let rows = q
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 下拉列表：`salesman JOIN user`，按 nickname/username/mobile LIKE 过滤
    /// （Java 为全表 user 扫描 + 逐条查询，本实现纠偏为 JOIN，见 models 差异 5）。
    pub async fn select_salesmen(
        &self,
        nickname: Option<&str>,
        shop_id: Option<i32>,
    ) -> Result<Vec<(Salesman, SalesmanUserRow)>> {
        let kw = nickname
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|k| format!("%{k}%"));
        let mut conds: Vec<String> = Vec::new();
        if kw.is_some() {
            conds.push("(u.nickname LIKE ? OR u.username LIKE ? OR u.mobile LIKE ?)".into());
        }
        // 多租户：店铺账号只返回本店分销员（i32 内部值）
        if let Some(shop_id) = shop_id {
            conds.push(format!("s.shop_id = {shop_id}"));
        }
        let where_sql = if conds.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conds.join(" AND "))
        };
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT s.salesman_id, s.user_id, s.level, s.group_id, s.pid, s.add_time, s.shop_id, s.sale_amount, u.user_id AS uid, u.username AS uname, u.nickname AS unickname, u.mobile AS umobile, u.avatar AS uavatar, u.reg_time AS ureg_time, u.rank_id AS urank_id FROM salesman s JOIN \"user\" u ON u.user_id = s.user_id {where_sql} ORDER BY s.salesman_id DESC LIMIT 200"
        ));
        #[derive(FromRow)]
        struct Row {
            salesman_id: i32,
            user_id: Option<i32>,
            level: Option<i32>,
            group_id: Option<i32>,
            pid: i32,
            add_time: Option<i32>,
            shop_id: Option<i32>,
            sale_amount: Option<Decimal>,
            uid: i32,
            uname: String,
            unickname: Option<String>,
            umobile: String,
            uavatar: String,
            ureg_time: Option<i32>,
            urank_id: i32,
        }
        let mut q = sqlx::query_as::<_, Row>(&sql);
        if let Some(k) = kw.as_deref() {
            q = q.bind(k).bind(k).bind(k);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    Salesman {
                        salesman_id: r.salesman_id,
                        user_id: r.user_id,
                        level: r.level,
                        group_id: r.group_id,
                        pid: r.pid,
                        add_time: r.add_time,
                        shop_id: r.shop_id,
                        sale_amount: r.sale_amount,
                    },
                    SalesmanUserRow {
                        user_id: r.uid,
                        username: r.uname,
                        nickname: r.unickname,
                        mobile: r.umobile,
                        avatar: r.uavatar,
                        reg_time: r.ureg_time,
                        rank_id: r.urank_id,
                    },
                )
            })
            .collect())
    }

    /// 客户列表（Java `SalesmanMapper#customerList` 的简化版：
    /// `salesman_customer JOIN user LEFT JOIN user_rank`）。
    ///
    /// `orderCount` / `orderAmount` / `lastOrderTime` 依赖订单域聚合，
    /// 本实现返回 0/null（见 models 差异 6）。
    pub async fn paginate_customers(
        &self,
        salesman_id: i32,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<CustomerRow>)> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM salesman_customer WHERE salesman_id = $1",
        )
        .bind(salesman_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        let sql = "SELECT u.user_id, u.username, u.avatar, u.mobile, u.nickname, u.reg_time, r.rank_name, c.channel, c.add_time AS bind_time FROM salesman_customer c JOIN \"user\" u ON u.user_id = c.user_id LEFT JOIN user_rank r ON r.rank_id = u.rank_id WHERE c.salesman_id = $1 ORDER BY c.salesman_customer_id DESC LIMIT $2 OFFSET $3";
        let rows = sqlx::query_as::<_, CustomerRow>(sql)
            .bind(salesman_id)
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((count.0.max(0) as u64, rows))
    }
}

/// 公共端会员信息行（`user` + `user_rank` 子集）。
#[derive(Debug, Clone, Default, FromRow)]
pub struct PublicUserRow {
    pub user_id: i32,
    pub username: String,
    pub avatar: String,
    pub mobile: String,
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub balance: Option<Decimal>,
    pub points: Option<i32>,
    pub rank_name: Option<String>,
}

impl SalesmanAdminRepository {
    /// 分销配置原始 JSON（Java `salesmanConfigService.detail(code)` →
    /// `salesman_config.data`；按 code 取首条）。
    pub async fn find_salesman_config(&self, code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT data FROM salesman_config WHERE code = $1 ORDER BY id ASC LIMIT 1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }

    /// 分销员全部业绩订单（Java `salesmanOrderMapper.selectList(eq(salesmanId))`）。
    pub async fn list_salesman_orders(&self, salesman_id: i32) -> Result<Vec<SalesmanOrder>> {
        let sql = format!(
            "SELECT {SALESMAN_ORDER_COLUMNS} FROM salesman_order WHERE salesman_id = $1"
        );
        sqlx::query_as::<_, SalesmanOrder>(&sql)
            .bind(salesman_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 分销员全部客户绑定（Java `salesmanCustomerService.lambdaQuery().eq(salesmanId).list()`）。
    pub async fn list_salesman_customers(&self, salesman_id: i32) -> Result<Vec<SalesmanCustomer>> {
        let sql = format!(
            "SELECT {SALESMAN_CUSTOMER_COLUMNS} FROM salesman_customer WHERE salesman_id = $1"
        );
        sqlx::query_as::<_, SalesmanCustomer>(&sql)
            .bind(salesman_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 自购金额（Java `orderService.query().eq(userId).select("sum(paid_amount)")`）。
    pub async fn sum_self_buy(&self, user_id: i32) -> Result<Decimal> {
        let row: (Decimal,) = sqlx::query_as(
            "SELECT COALESCE(SUM(paid_amount), 0) FROM \"order\" WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 用户订单合计 `(orderCount, orderAmount)`（公共端客户详情聚合）。
    pub async fn order_totals_for_user(&self, user_id: i32) -> Result<(i64, Decimal)> {
        let row: (i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(paid_amount), 0) FROM \"order\" WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((row.0, row.1.unwrap_or_default()))
    }

    /// 客户绑定（Java `updateCustomerRemark` 的定位：salesmanId + userId）。
    pub async fn find_customer_binding(
        &self,
        salesman_id: i32,
        user_id: i32,
    ) -> Result<Option<SalesmanCustomer>> {
        let sql = format!(
            "SELECT {SALESMAN_CUSTOMER_COLUMNS} FROM salesman_customer WHERE salesman_id = $1 AND user_id = $2 LIMIT 1"
        );
        sqlx::query_as::<_, SalesmanCustomer>(&sql)
            .bind(salesman_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 更新客户备注（Java `binding.setRemark(trim); updateById`）。
    pub async fn update_customer_remark(&self, id: i32, remark: &str) -> Result<bool> {
        let res = sqlx::query("UPDATE salesman_customer SET remark = $1 WHERE salesman_customer_id = $2")
            .bind(remark)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 公共端会员信息（`/api/salesman/salesman/userInfo`）。
    pub async fn find_public_user(&self, user_id: i32) -> Result<Option<PublicUserRow>> {
        sqlx::query_as::<_, PublicUserRow>(
            "SELECT u.user_id, u.username, u.avatar, u.mobile, u.email, u.nickname, u.balance, u.points, r.rank_name FROM \"user\" u LEFT JOIN user_rank r ON r.rank_id = u.rank_id WHERE u.user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 公共端客户列表（Java `customerApiList`）：
    /// - `all` → `salesman_customer JOIN user`
    /// - `ordered` → 同上 + `EXISTS(order)`（Java 按最后下单时间筛选，见服务层说明）
    /// - `salesman` → 下级分销员（`salesman.pid = 当前分销员`）
    pub async fn paginate_api_customers(
        &self,
        salesman_id: i32,
        customer_type: &str,
        start: Option<i64>,
        end: Option<i64>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<CustomerRow>)> {
        if customer_type == "salesman" {
            return self
                .paginate_sub_salesmen(salesman_id, start, end, limit, offset)
                .await;
        }
        let ordered = customer_type == "ordered";
        let order_exists = if ordered {
            "AND EXISTS (SELECT 1 FROM \"order\" o WHERE o.user_id = c.user_id)"
        } else {
            ""
        };
        let mut where_parts = vec!["c.salesman_id = ?".to_string()];
        if start.is_some() {
            where_parts.push("c.add_time >= ?".into());
        }
        if end.is_some() {
            where_parts.push("c.add_time <= ?".into());
        }
        let where_sql = format!("WHERE {} {order_exists}", where_parts.join(" AND "));

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM salesman_customer c {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(salesman_id);
        if let Some(v) = start {
            cq = cq.bind(v);
        }
        if let Some(v) = end {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT u.user_id, u.username, u.avatar, u.mobile, u.nickname, u.reg_time, r.rank_name, c.channel, c.add_time AS bind_time FROM salesman_customer c JOIN \"user\" u ON u.user_id = c.user_id LEFT JOIN user_rank r ON r.rank_id = u.rank_id {where_sql} ORDER BY c.salesman_customer_id DESC LIMIT ? OFFSET ?"
        ));
        let mut q = sqlx::query_as::<_, CustomerRow>(&sql).bind(salesman_id);
        if let Some(v) = start {
            q = q.bind(v);
        }
        if let Some(v) = end {
            q = q.bind(v);
        }
        let rows = q
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 下级分销员分页（公共端 `customerType=salesman`）。
    async fn paginate_sub_salesmen(
        &self,
        salesman_id: i32,
        start: Option<i64>,
        end: Option<i64>,
        limit: u64,
        offset: u64,
    ) -> Result<(u64, Vec<CustomerRow>)> {
        let mut where_parts = vec!["s.pid = ?".to_string()];
        if start.is_some() {
            where_parts.push("s.add_time >= ?".into());
        }
        if end.is_some() {
            where_parts.push("s.add_time <= ?".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM salesman s {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql).bind(salesman_id);
        if let Some(v) = start {
            cq = cq.bind(v);
        }
        if let Some(v) = end {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT u.user_id, u.username, u.avatar, u.mobile, u.nickname, u.reg_time, r.rank_name, 'salesman' AS channel, s.add_time AS bind_time FROM salesman s JOIN \"user\" u ON u.user_id = s.user_id LEFT JOIN user_rank r ON r.rank_id = u.rank_id {where_sql} ORDER BY s.salesman_id DESC LIMIT ? OFFSET ?"
        ));
        let mut q = sqlx::query_as::<_, CustomerRow>(&sql).bind(salesman_id);
        if let Some(v) = start {
            q = q.bind(v);
        }
        if let Some(v) = end {
            q = q.bind(v);
        }
        let rows = q
            .bind(limit.max(1) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }
}

/// 排序白名单（`salesman` 实存列；未知回退主键）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some("salesmanId") | Some("salesman_id") => "salesman_id",
        Some("addTime") | Some("add_time") => "add_time",
        Some("saleAmount") | Some("sale_amount") => "sale_amount",
        Some("level") => "level",
        Some("groupId") | Some("group_id") => "group_id",
        _ => "salesman_id",
    }
}

fn bind_salesman_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    shop_id: Option<i32>,
    user_id: Option<i32>,
    group_id: Option<i32>,
    level: Option<i32>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q;
    for v in [shop_id, user_id, group_id, level].into_iter().flatten() {
        q = q.bind(v);
    }
    q
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_UNKNOWN_SALESMAN.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_lists_cover_entities() {
        assert_eq!(SALESMAN_COLUMNS.split(", ").count(), 8);
        assert_eq!(SALESMAN_ORDER_COLUMNS.split(", ").count(), 12);
        assert_eq!(SALESMAN_CUSTOMER_COLUMNS.split(", ").count(), 6);
        assert_eq!(SALESMAN_GROUP_COLUMNS.split(", ").count(), 5);
    }

    #[test]
    fn sort_whitelist_falls_back_to_primary_key() {
        assert_eq!(resolve_sort_column(None), "salesman_id");
        assert_eq!(resolve_sort_column(Some("saleAmount")), "sale_amount");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE salesman")), "salesman_id");
    }
}
