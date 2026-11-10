//! 分销域仓储：`salesman` / `salesman_group` / `salesman_customer` /
//! `salesman_order` / `salesman_product` / `salesman_material` /
//! `salesman_material_category` / `salesman_content` / `salesman_config`，
//! 另含 `user` / `order` / `order_item` / `product` 的只读辅助查询
//! （原 Java 对应 `SalesmanMapper`、`SalesmanOrderMapper`、
//! `SalesmanMaterialMapper` 等 XML/注解 SQL 与各 ServiceImpl 的条件构造）。
//!
//! 全部使用函数式 `sqlx::query_as`（不使用 `query_as!` 宏，列名不参与编译期校验，
//! 与实体 `#[sqlx(rename_all = "snake_case")]` 对齐）。
//! 所有分销相关表（含 user/order 辅助查询）的方法都写在本文件，
//! 不新增仓储，也不修改共享的 `Repositories`。

use rust_decimal::Decimal;
use sqlx::{FromRow,PgPool};

use nextrade_core::entities::{
    OrderItem, Salesman, SalesmanConfig, SalesmanContent, SalesmanGroup, SalesmanMaterial,
    SalesmanMaterialCategory, SalesmanOrder, SalesmanProduct, User,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

// ————————————————— 动态 SQL 基础设施 —————————————————

/// 动态 SQL 的绑定值（按序绑定，规避动态 IN 列表与可选条件问题）。
#[derive(Clone)]
enum Bind {
    I(i32),
    I64(i64),
    S(String),
    D(Decimal),
}

/// 追加一个 `AND` 条件与对应绑定值。
fn push_cond(sql: &mut String, params: &mut Vec<Bind>, clause: &str, binds: &[Bind]) {
    sql.push_str(" AND ");
    sql.push_str(clause);
    params.extend_from_slice(binds);
}

/// 追加 `IN (...)` 条件；列表为空时追加恒假条件（保证结果为空）。
fn push_in_or_false(sql: &mut String, params: &mut Vec<Bind>, column: &str, values: &[i32]) {
    if values.is_empty() {
        sql.push_str(" AND 1=0");
        return;
    }
    let placeholders = std::iter::repeat_n("?", values.len())
        .collect::<Vec<_>>()
        .join(",");
    let binds: Vec<Bind> = values.iter().copied().map(Bind::I).collect();
    push_cond(
        sql,
        params,
        &format!("{column} IN ({placeholders})"),
        &binds,
    );
}

/// 按序绑定参数并取回多行。
async fn fetch_rows<T>(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<Vec<T>>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let pg_sql = crate::db::to_pg_placeholders(sql);
    let mut q = sqlx::query_as::<_, T>(&pg_sql);
    for p in params {
        q = match p {
            Bind::I(v) => q.bind(*v),
            Bind::I64(v) => q.bind(*v),
            Bind::S(v) => q.bind(v.clone()),
            Bind::D(v) => q.bind(*v),
        };
    }
    q.fetch_all(pool).await.map_err(map_err)
}

/// 取回单行（可选）。
async fn fetch_one<T>(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<Option<T>>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let pg_sql = crate::db::to_pg_placeholders(sql);
    let mut q = sqlx::query_as::<_, T>(&pg_sql);
    for p in params {
        q = match p {
            Bind::I(v) => q.bind(*v),
            Bind::I64(v) => q.bind(*v),
            Bind::S(v) => q.bind(v.clone()),
            Bind::D(v) => q.bind(*v),
        };
    }
    q.fetch_optional(pool).await.map_err(map_err)
}

/// 统计（复用 `fetch_rows`，取 `(i64,)` 首行）。
async fn count(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<u64> {
    let rows = fetch_rows::<(i64,)>(pool, sql, params).await?;
    Ok(rows.first().map(|r| r.0).unwrap_or(0).max(0) as u64)
}

/// 执行写语句，返回影响行数。
async fn exec(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<u64> {
    let pg_sql = crate::db::to_pg_placeholders(sql);
    let mut q = sqlx::query(&pg_sql);
    for p in params {
        q = match p {
            Bind::I(v) => q.bind(*v),
            Bind::I64(v) => q.bind(*v),
            Bind::S(v) => q.bind(v.clone()),
            Bind::D(v) => q.bind(*v),
        };
    }
    let r = q.execute(pool).await.map_err(map_err)?;
    Ok(r.rows_affected())
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("分销数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

// ————————————————— 过滤条件 / 行结构 —————————————————

/// 分销员分页过滤条件（由服务层从 `SalesmanListQuery` 组装）。
#[derive(Debug, Clone, Default)]
pub struct SalesmanFilter {
    /// 手机号/用户名/昵称模糊
    pub mobile: Option<String>,
    /// 同上（`keyword`）
    pub keyword: Option<String>,
    /// 上级分销员手机号
    pub pid_mobile: Option<String>,
    pub add_time_start: Option<i32>,
    /// 含当天：原 Java 为 `addTimeEnd + 24h`
    pub add_time_end: Option<i32>,
    pub group_id: Option<i32>,
    pub level: Option<i32>,
    pub user_id: Option<i32>,
    pub salesman_id: Option<i32>,
}

/// 分销业绩订单分页过滤条件。
#[derive(Debug, Clone, Default)]
pub struct SalesmanOrderFilter {
    pub order_sn: Option<String>,
    pub status: Option<i32>,
    pub salesman_id: Option<i32>,
    pub order_time_start: Option<i32>,
    pub order_time_end: Option<i32>,
}

/// 分销内容分页过滤条件。
#[derive(Debug, Clone, Default)]
pub struct SalesmanContentFilter {
    /// 0 未开始 / 1 展示中 / 2 已失效
    pub status: Option<i32>,
    pub title: Option<String>,
    pub start_time: Option<i32>,
    pub end_time: Option<i32>,
    /// 前台视角（`from=user`）
    pub from_user: bool,
    /// 管理端店铺过滤
    pub shop_id: Option<i32>,
    pub now: i32,
}

/// 成交客户分页过滤条件。
#[derive(Debug, Clone, Default)]
pub struct CustomerFilter {
    pub salesman_id: Option<i32>,
    pub start_time: Option<i32>,
    pub end_time: Option<i32>,
}

/// 成交客户行（`salesman_customer` 关联 `user` + 订单聚合）。
#[derive(Debug, Clone, FromRow)]
pub struct CustomerRow {
    pub salesman_customer_id: i32,
    pub salesman_id: Option<i32>,
    pub user_id: Option<i32>,
    pub add_time: Option<i32>,
    pub channel: String,
    pub remark: String,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub avatar: Option<String>,
    pub reg_time: Option<i32>,
    pub order_count: Option<i64>,
    pub order_amount: Option<Decimal>,
    pub last_order_time: Option<i32>,
}

/// 订单精简行（用于分销订单列表补充订单信息）。
#[derive(Debug, Clone, FromRow)]
pub struct OrderBrief {
    pub order_id: i32,
    pub order_sn: String,
    pub user_id: i32,
    pub order_status: i32,
    pub shipping_status: i32,
    pub pay_status: i32,
    pub add_time: i32,
    pub pay_time: i32,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub order_source: String,
    pub shop_id: i32,
}

/// 用户精简行。
#[derive(Debug, Clone, FromRow)]
pub struct UserBrief {
    pub user_id: i32,
    pub username: String,
    pub nickname: Option<String>,
    pub mobile: String,
    pub avatar: String,
    pub email: String,
    pub reg_time: i32,
}

/// 商品精简行。
#[derive(Debug, Clone, FromRow)]
pub struct ProductBrief {
    pub product_id: i32,
    pub product_name: String,
    pub product_sn: String,
    pub pic_thumb: String,
    pub product_price: Decimal,
    pub product_status: i32,
}

/// 客户成交行（`salesman_order` 关联 `order` + 下单用户 + 分销员用户）。
#[derive(Debug, Clone, FromRow)]
pub struct CustomerTransactionRow {
    pub order_id: Option<i32>,
    pub order_sn: Option<String>,
    pub user_id: Option<i32>,
    pub order_status: Option<i32>,
    pub pay_status: Option<i32>,
    pub total_amount: Option<Decimal>,
    pub paid_amount: Option<Decimal>,
    pub order_add_time: Option<i32>,
    pub pay_time: Option<i32>,
    pub salesman_id: Option<i32>,
    pub amount: Option<Decimal>,
    pub status: Option<i32>,
    pub username: Option<String>,
    pub nickname: Option<String>,
    pub mobile: Option<String>,
    pub salesman_user_id: Option<i32>,
    pub salesman_nickname: Option<String>,
    pub salesman_username: Option<String>,
}

#[derive(Clone)]
pub struct SalesmanRepository {
    pool: PgPool,
}

impl SalesmanRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— salesman 分销员 —————————————————————

    /// 分销员分页。
    ///
    /// 对应 `SalesmanServiceImpl#list(SalesmanPageQuery)` 的条件构造：
    /// 手机号/关键字 → `user` 子查询过滤 `user_id`；上级手机号 → `pid IN (...)`；
    /// 时间范围（结束含当天）+ 分组/等级/分销员/用户精确过滤。
    pub async fn paginate_salesmen(
        &self,
        f: &SalesmanFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Salesman>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();

        for kw in [&f.mobile, &f.keyword].into_iter().flatten() {
            if kw.trim().is_empty() {
                continue;
            }
            let like = format!("%{kw}%");
            push_cond(
                &mut wh,
                &mut params,
                "s.user_id IN (SELECT user_id FROM \"user\" \
                 WHERE mobile ILIKE $1 OR username ILIKE $2 OR nickname ILIKE $3)",
                &[
                    Bind::S(like.clone()),
                    Bind::S(like.clone()),
                    Bind::S(like),
                ],
            );
        }
        if let Some(pid_kw) = f.pid_mobile.as_ref().filter(|k| !k.trim().is_empty()) {
            let like = format!("%{pid_kw}%");
            push_cond(
                &mut wh,
                &mut params,
                "s.pid IN (SELECT s2.salesman_id FROM salesman s2 \
                 JOIN \"user\" u2 ON u2.user_id = s2.user_id WHERE u2.mobile ILIKE $1)",
                &[Bind::S(like)],
            );
        }
        if let Some(start) = f.add_time_start {
            push_cond(&mut wh, &mut params, "s.add_time >= ?", &[Bind::I(start)]);
        }
        if let Some(end) = f.add_time_end {
            push_cond(
                &mut wh,
                &mut params,
                "s.add_time <= ?",
                &[Bind::I(end + 86_400)],
            );
        }
        if let Some(v) = f.group_id {
            push_cond(&mut wh, &mut params, "s.group_id = ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.level {
            push_cond(&mut wh, &mut params, "s.level = ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.user_id {
            push_cond(&mut wh, &mut params, "s.user_id = ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.salesman_id {
            push_cond(&mut wh, &mut params, "s.salesman_id = ?", &[Bind::I(v)]);
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman s{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<Salesman>(
            &self.pool,
            &format!(
                "SELECT s.* FROM salesman s{wh} \
                 ORDER BY s.salesman_id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 分销员分页（服务层简化入口：仅 `keyword` 单条件）。
    ///
    /// `keyword` 对应原 Java `SalesmanPageQuery#keyword`：对 \"user\" 表的
    /// 手机号/用户名/昵称做模糊匹配（见 `paginate_salesmen` 中的同一条件）。
    /// 需要完整筛选（`mobile` / `pid_mobile` / 时间范围 / 分组/等级等）时
    /// 直接调用 `paginate_salesmen`。
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Salesman>)> {
        let filter = SalesmanFilter {
            keyword: keyword.map(str::to_string),
            ..SalesmanFilter::default()
        };
        self.paginate_salesmen(&filter, page).await
    }

    /// 按 ID 查分销员。
    pub async fn find_by_id(&self, salesman_id: i32) -> Result<Option<Salesman>> {
        fetch_one::<Salesman>(
            &self.pool,
            "SELECT * FROM salesman WHERE salesman_id = $1",
            &[Bind::I(salesman_id)],
        )
        .await
    }

    /// 按 ID 批量查分销员。
    pub async fn find_salesmen_by_ids(&self, ids: &[i32]) -> Result<Vec<Salesman>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut params, "salesman_id", ids);
        fetch_rows::<Salesman>(
            &self.pool,
            &format!("SELECT * FROM salesman{wh}"),
            &params,
        )
        .await
    }

    /// 按会员 ID 查分销员。
    pub async fn find_by_user_id(&self, user_id: i32) -> Result<Option<Salesman>> {
        fetch_one::<Salesman>(
            &self.pool,
            "SELECT * FROM salesman WHERE user_id = $1 LIMIT 1",
            &[Bind::I(user_id)],
        )
        .await
    }

    /// 分组下的分销员数量（分组删除前校验）。
    pub async fn count_by_group(&self, group_id: i32) -> Result<u64> {
        count(
            &self.pool,
            "SELECT COUNT(*) FROM salesman WHERE group_id = $1",
            &[Bind::I(group_id)],
        )
        .await
    }

    /// 下级分销员数量（上级为 `pid`）。
    pub async fn count_by_pid(&self, pid: i32) -> Result<u64> {
        count(
            &self.pool,
            "SELECT COUNT(*) FROM salesman WHERE pid = $1",
            &[Bind::I(pid)],
        )
        .await
    }

    /// 用户是否已是分销员。
    pub async fn count_by_user(&self, user_id: i32) -> Result<u64> {
        count(
            &self.pool,
            "SELECT COUNT(*) FROM salesman WHERE user_id = $1",
            &[Bind::I(user_id)],
        )
        .await
    }

    /// 新增分销员，返回自增 ID。
    pub async fn insert_salesman(
        &self,
        user_id: i32,
        level: Option<i32>,
        group_id: Option<i32>,
        pid: i32,
        shop_id: Option<i32>,
        add_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman (user_id, level, group_id, pid, add_time, shop_id, sale_amount) \
             VALUES ($1, $2, $3, $4, $5, $6, 0) RETURNING salesman_id",
        )
        .bind(user_id)
        .bind(level)
        .bind(group_id)
        .bind(pid)
        .bind(add_time)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新分销员（对应 MyBatis-Plus \"updateById\" 的非空更新语义）。
    pub async fn update_salesman(&self, s: &Salesman) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman SET user_id = $1, level = $2, group_id = $3, pid = $4, \
             add_time = $5, shop_id = $6, sale_amount = $7 WHERE salesman_id = $8",
            &[
                Bind::I(s.user_id.unwrap_or(0)),
                Bind::I(s.level.unwrap_or(0)),
                Bind::I(s.group_id.unwrap_or(0)),
                Bind::I(s.pid),
                Bind::I(s.add_time.unwrap_or(0)),
                Bind::I(s.shop_id.unwrap_or(0)),
                Bind::D(s.sale_amount.unwrap_or(Decimal::ZERO)),
                Bind::I(s.salesman_id),
            ],
        )
        .await
    }

    // ————————————————————— user 辅助 —————————————————————

    /// 按 ID 查会员（完整实体，用于前台分销员中心等）。
    pub async fn find_user(&self, user_id: i32) -> Result<Option<User>> {
        fetch_one::<User>(
            &self.pool,
            "SELECT * FROM \"user\" WHERE user_id = $1",
            &[Bind::I(user_id)],
        )
        .await
    }

    /// 批量查会员（精简字段，用于列表回填昵称/手机号/头像）。
    pub async fn find_users_by_ids(&self, ids: &[i32]) -> Result<Vec<UserBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut params, "user_id", ids);
        fetch_rows::<UserBrief>(
            &self.pool,
            &format!(
                "SELECT user_id, username, nickname, mobile, avatar, email, reg_time \
                 FROM \"user\"{wh}"
            ),
            &params,
        )
        .await
    }

    /// 会员自购累计实付金额（`SalesmanServiceImpl#detailForApi` 的 selfBuySum）。
    pub async fn sum_paid_amount_by_user(&self, user_id: i32) -> Result<Decimal> {
        let rows = fetch_rows::<(Option<Decimal>,)>(
            &self.pool,
            "SELECT SUM(paid_amount) FROM \"order\" WHERE user_id = $1",
            &[Bind::I(user_id)],
        )
        .await?;
        Ok(rows.first().and_then(|r| r.0).unwrap_or(Decimal::ZERO))
    }

    // ————————————————————— salesman_group 分组 —————————————————————

    /// 分组分页（按添加时间升序，同原 Java）。
    pub async fn paginate_groups(
        &self,
        group_name: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanGroup>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(name) = group_name.filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "group_name ILIKE ?",
                &[Bind::S(format!("%{name}%"))],
            );
        }
        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_group{wh}"),
            &params,
        )
        .await?;
        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<SalesmanGroup>(
            &self.pool,
            &format!(
                "SELECT * FROM salesman_group{wh} \
                 ORDER BY add_time ASC, group_id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查分组。
    pub async fn find_group(&self, group_id: i32) -> Result<Option<SalesmanGroup>> {
        fetch_one::<SalesmanGroup>(
            &self.pool,
            "SELECT * FROM salesman_group WHERE group_id = $1",
            &[Bind::I(group_id)],
        )
        .await
    }

    /// 新增分组，返回自增 ID。
    pub async fn insert_group(
        &self,
        group_name: Option<&str>,
        describe: Option<&str>,
        shop_id: Option<i32>,
        add_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_group (group_name, \"describe\", shop_id, add_time) \
             VALUES ($1, $2, $3, $4) RETURNING group_id",
        )
        .bind(group_name)
        .bind(describe)
        .bind(shop_id)
        .bind(add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新分组。
    pub async fn update_group(&self, g: &SalesmanGroup) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman_group SET group_name = $1, \"describe\" = $2, shop_id = $3, \
             add_time = $4 WHERE group_id = $5",
            &[
                Bind::S(g.group_name.clone().unwrap_or_default()),
                Bind::S(g.describe.clone().unwrap_or_default()),
                Bind::I(g.shop_id.unwrap_or(0)),
                Bind::I(g.add_time.unwrap_or(0)),
                Bind::I(g.group_id),
            ],
        )
        .await
    }

    /// 删除分组。
    pub async fn delete_group(&self, group_id: i32) -> Result<u64> {
        exec(
            &self.pool,
            "DELETE FROM salesman_group WHERE group_id = $1",
            &[Bind::I(group_id)],
        )
        .await
    }

    // ————————————————————— salesman_customer 客户 —————————————————————

    /// 客户数量。
    pub async fn count_customers(&self, salesman_id: i32) -> Result<u64> {
        count(
            &self.pool,
            "SELECT COUNT(*) FROM salesman_customer WHERE salesman_id = $1",
            &[Bind::I(salesman_id)],
        )
        .await
    }

    /// 客户列表分页（关联用户资料 + 订单聚合统计）。
    ///
    /// 对应原 `SalesmanMapper.customerList` 的统计口径：已支付（`pay_status=2`）
    /// 且未删除订单数 / 实付净额 / 最后成交时间。
    pub async fn paginate_customers(
        &self,
        f: &CustomerFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<CustomerRow>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(v) = f.salesman_id {
            push_cond(&mut wh, &mut params, "sc.salesman_id = ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.start_time {
            push_cond(&mut wh, &mut params, "sc.add_time >= ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.end_time {
            push_cond(&mut wh, &mut params, "sc.add_time <= ?", &[Bind::I(v + 86_400)]);
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_customer sc{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<CustomerRow>(
            &self.pool,
            &format!(
                "SELECT sc.salesman_customer_id, sc.salesman_id, sc.user_id, sc.add_time, \
                        sc.channel, sc.remark, \
                        u.username, u.nickname, u.mobile, u.avatar, u.reg_time, \
                        (SELECT COUNT(*) FROM \"order\" o \
                          WHERE o.user_id = sc.user_id AND o.pay_status = 2 AND o.is_del = 0) \
                            AS order_count, \
                        (SELECT SUM(o.paid_amount) FROM \"order\" o \
                          WHERE o.user_id = sc.user_id AND o.pay_status = 2 AND o.is_del = 0) \
                            AS order_amount, \
                        (SELECT MAX(o.add_time) FROM \"order\" o \
                          WHERE o.user_id = sc.user_id AND o.pay_status = 2 AND o.is_del = 0) \
                            AS last_order_time \
                 FROM salesman_customer sc \
                 LEFT JOIN \"user\" u ON u.user_id = sc.user_id{wh} \
                 ORDER BY sc.add_time DESC, sc.salesman_customer_id DESC \
                 LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    // ————————————————————— salesman_order 订单 —————————————————————

    /// 分销业绩订单分页。
    ///
    /// 对应 `SalesmanOrderServiceImpl#list` 的条件构造：订单号 → \"order\" 子查询、
    /// 状态、时间范围（按订单下单时间）。
    pub async fn paginate_orders(
        &self,
        f: &SalesmanOrderFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanOrder>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();

        if let Some(sn) = f.order_sn.as_ref().filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "so.order_id IN (SELECT order_id FROM \"order\" WHERE order_sn ILIKE $1)",
                &[Bind::S(format!("%{sn}%"))],
            );
        }
        if let Some(v) = f.status {
            push_cond(&mut wh, &mut params, "so.status = ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.salesman_id {
            push_cond(&mut wh, &mut params, "so.salesman_id = ?", &[Bind::I(v)]);
        }
        if let Some(start) = f.order_time_start {
            push_cond(
                &mut wh,
                &mut params,
                "so.order_id IN (SELECT order_id FROM \"order\" WHERE add_time >= $1)",
                &[Bind::I(start)],
            );
        }
        if let Some(end) = f.order_time_end {
            push_cond(
                &mut wh,
                &mut params,
                "so.order_id IN (SELECT order_id FROM \"order\" WHERE add_time <= $1)",
                &[Bind::I(end)],
            );
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_order so{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<SalesmanOrder>(
            &self.pool,
            &format!(
                "SELECT so.* FROM salesman_order so{wh} \
                 ORDER BY so.salesman_order_id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 某分销员的全部业绩订单（详情/佣金详情用）。
    pub async fn find_orders_by_salesman(&self, salesman_id: i32) -> Result<Vec<SalesmanOrder>> {
        fetch_rows::<SalesmanOrder>(
            &self.pool,
            "SELECT * FROM salesman_order WHERE salesman_id = $1 \
             ORDER BY salesman_order_id DESC",
            &[Bind::I(salesman_id)],
        )
        .await
    }

    /// 按 ID 批量查业绩订单。
    pub async fn find_orders_by_ids(&self, ids: &[i32]) -> Result<Vec<SalesmanOrder>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut params, "salesman_order_id", ids);
        fetch_rows::<SalesmanOrder>(
            &self.pool,
            &format!("SELECT * FROM salesman_order{wh}"),
            &params,
        )
        .await
    }

    /// 人工结算：仅把「待结算」置为「已结算」。
    pub async fn settle_orders(&self, ids: &[i32], now: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        // SET 子句的占位符在 WHERE 之前，绑定值必须按同样顺序排列。
        let mut wh = String::from(" WHERE status = 0");
        let mut where_params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut where_params, "salesman_order_id", ids);
        let mut params = vec![Bind::I(now)];
        params.append(&mut where_params);
        exec(
            &self.pool,
            &("UPDATE salesman_order SET status = 1, settlement_time = $1".to_string() + &wh),
            &params,
        )
        .await
    }

    /// 批量更新状态（\"batch\" 的 \"del\" 语义外的自定义状态批量处理）。
    pub async fn update_order_status(&self, ids: &[i32], status: i32) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut where_params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut where_params, "salesman_order_id", ids);
        let mut params = vec![Bind::I(status)];
        params.append(&mut where_params);
        exec(
            &self.pool,
            &("UPDATE salesman_order SET status = $1".to_string() + &wh),
            &params,
        )
        .await
    }

    /// 客户成交分页（订单口径：按 \"order\" 聚合 \"salesman_order\"）。
    pub async fn paginate_customer_transactions(
        &self,
        f: &CustomerFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<CustomerTransactionRow>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(v) = f.salesman_id {
            push_cond(&mut wh, &mut params, "so.salesman_id = ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.start_time {
            push_cond(&mut wh, &mut params, "o.add_time >= ?", &[Bind::I(v)]);
        }
        if let Some(v) = f.end_time {
            push_cond(&mut wh, &mut params, "o.add_time <= ?", &[Bind::I(v + 86_400)]);
        }

        let joins = " FROM salesman_order so \
             JOIN \"order\" o ON o.order_id = so.order_id \
             LEFT JOIN \"user\" cu ON cu.user_id = o.user_id \
             LEFT JOIN salesman s ON s.salesman_id = so.salesman_id \
             LEFT JOIN \"user\" su ON su.user_id = s.user_id";

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*){joins}{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<CustomerTransactionRow>(
            &self.pool,
            &format!(
                "SELECT o.order_id, o.order_sn, o.user_id, o.order_status, o.pay_status, \
                        o.total_amount, o.paid_amount, o.add_time AS order_add_time, \
                        o.pay_time, so.salesman_id, so.amount, so.status, \
                        cu.username, cu.nickname, cu.mobile, \
                        s.user_id AS salesman_user_id, su.nickname AS salesman_nickname, \
                        su.username AS salesman_username \
                 {joins}{wh} \
                 ORDER BY o.order_id DESC, so.salesman_order_id DESC \
                 LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    // ————————————————————— order / order_item 辅助 —————————————————————

    /// 批量查订单（精简字段）。
    pub async fn find_order_briefs(&self, ids: &[i32]) -> Result<Vec<OrderBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut params, "order_id", ids);
        fetch_rows::<OrderBrief>(
            &self.pool,
            &format!(
                "SELECT order_id, order_sn, user_id, order_status, shipping_status, \
                        pay_status, add_time, pay_time, total_amount, paid_amount, \
                        order_source, shop_id \
                 FROM \"order\"{wh}"
            ),
            &params,
        )
        .await
    }

    /// 按订单项 ID 查 `order_item`。
    pub async fn find_order_item(&self, item_id: i32) -> Result<Option<OrderItem>> {
        fetch_one::<OrderItem>(
            &self.pool,
            "SELECT \"item_id\", \"order_id\", \"order_sn\", \"user_id\", \"price\", \"quantity\", \"product_id\", \"product_name\", \"product_sn\", \"pic_thumb\", \"sku_id\", \"sku_data\", \"delivery_quantity\", \"product_type\", \"is_gift\", \"shop_id\", \"is_pin\", \"prepay_price\", \"commission\", \"origin_price\", \"promotion_data\", \"is_seckill\", \"extra_sku_data\", \"suppliers_id\", \"card_group_name\", \"vendor_product_id\", \"vendor_product_sku_id\", \"vendor_id\", \"vendor_product_supply_price\", \"change_price_data\"::text AS \"change_price_data\", \"admin_change_pre_price\", \"gift_card_id\", \"delivery_service_fee\", \"server_service_fee\", \"is_wholesale_line\", \"wholesale_grade_id\", \"wholesale_base_price\" FROM order_item WHERE item_id = $1",
            &[Bind::I(item_id)],
        )
        .await
    }

    // ————————————————————— salesman_product 分销商品 —————————————————————

    /// 分销商品分页（关联商品基础信息）。
    pub async fn paginate_products(
        &self,
        product_name: Option<&str>,
        is_join: Option<i32>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanProduct>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(v) = is_join {
            push_cond(&mut wh, &mut params, "sp.is_join = ?", &[Bind::I(v)]);
        }
        if let Some(name) = product_name.filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "sp.product_id IN (SELECT product_id FROM product WHERE product_name ILIKE $1)",
                &[Bind::S(format!("%{name}%"))],
            );
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_product sp{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<SalesmanProduct>(
            &self.pool,
            &format!(
                "SELECT sp.* FROM salesman_product sp{wh} \
                 ORDER BY sp.salesman_product_id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 `salesman_product_id` 查分销商品。
    pub async fn find_product(&self, id: i32) -> Result<Option<SalesmanProduct>> {
        fetch_one::<SalesmanProduct>(
            &self.pool,
            "SELECT * FROM salesman_product WHERE salesman_product_id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    /// 按商品 ID 查分销商品配置。
    pub async fn find_product_by_product_id(
        &self,
        product_id: i32,
    ) -> Result<Option<SalesmanProduct>> {
        fetch_one::<SalesmanProduct>(
            &self.pool,
            "SELECT * FROM salesman_product WHERE product_id = $1",
            &[Bind::I(product_id)],
        )
        .await
    }

    /// 批量查商品（精简字段）。
    pub async fn find_products_by_ids(&self, ids: &[i32]) -> Result<Vec<ProductBrief>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut params, "product_id", ids);
        fetch_rows::<ProductBrief>(
            &self.pool,
            &format!(
                "SELECT product_id, product_name, product_sn, pic_thumb, product_price, \
                        product_status FROM product{wh}"
            ),
            &params,
        )
        .await
    }

    /// 商品是否存在（绑定前校验）。
    pub async fn product_exists(&self, product_id: i32) -> Result<bool> {
        Ok(
            fetch_one::<(i32,)>(
                &self.pool,
                "SELECT product_id FROM product WHERE product_id = $1",
                &[Bind::I(product_id)],
            )
            .await?
            .is_some(),
        )
    }

    /// 新增分销商品，返回自增 ID。
    pub async fn insert_product(
        &self,
        product_id: i32,
        is_join: i32,
        commission_type: Option<i32>,
        commission_data: Option<&str>,
        shop_id: Option<i32>,
        now: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_product \
             (product_id, is_join, commission_type, commission_data, add_time, update_time, shop_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING salesman_product_id",
        )
        .bind(product_id)
        .bind(is_join)
        .bind(commission_type)
        .bind(commission_data)
        .bind(now)
        .bind(now)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新分销商品。
    pub async fn update_product(&self, p: &SalesmanProduct) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman_product SET product_id = $1, is_join = $2, commission_type = $3, \
             commission_data = $4, add_time = $5, update_time = $6, shop_id = $7 \
             WHERE salesman_product_id = $8",
            &[
                Bind::I(p.product_id.unwrap_or(0)),
                Bind::I(p.is_join.unwrap_or(0)),
                Bind::I(p.commission_type.unwrap_or(0)),
                Bind::S(p.commission_data.clone().unwrap_or_default()),
                Bind::I(p.add_time.unwrap_or(0)),
                Bind::I(p.update_time.unwrap_or(0)),
                Bind::I(p.shop_id.unwrap_or(0)),
                Bind::I(p.salesman_product_id),
            ],
        )
        .await
    }

    // ————————————————————— salesman_material 素材 —————————————————————

    /// 素材分页。
    ///
    /// `from_user=true` 时只看有效素材（原 Java \"SalesmanMaterialServiceImpl\"）。
    pub async fn paginate_materials(
        &self,
        shop_id: Option<i32>,
        category_id: Option<i32>,
        from_user: bool,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanMaterial>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(v) = shop_id {
            push_cond(&mut wh, &mut params, "shop_id = ?", &[Bind::I(v)]);
        }
        if let Some(v) = category_id {
            push_cond(&mut wh, &mut params, "category_id = ?", &[Bind::I(v)]);
        }
        if from_user {
            push_cond(&mut wh, &mut params, "is_available = ?", &[Bind::I(1)]);
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_material{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<SalesmanMaterial>(
            &self.pool,
            &format!(
                "SELECT * FROM salesman_material{wh} \
                 ORDER BY is_top DESC, id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查素材。
    pub async fn find_material(&self, id: i32) -> Result<Option<SalesmanMaterial>> {
        fetch_one::<SalesmanMaterial>(
            &self.pool,
            "SELECT * FROM salesman_material WHERE id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    /// 新增素材，返回自增 ID。
    pub async fn insert_material(
        &self,
        is_top: i32,
        content: Option<&str>,
        pics: Option<&str>,
        category_id: Option<i32>,
        product_id: Option<i32>,
        is_available: Option<i32>,
        shop_id: i32,
        add_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_material \
             (add_time, is_top, content, is_available, shop_id, pics, category_id, product_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
        )
        .bind(add_time)
        .bind(is_top)
        .bind(content)
        .bind(is_available)
        .bind(shop_id)
        .bind(pics)
        .bind(category_id)
        .bind(product_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新素材。
    pub async fn update_material(&self, m: &SalesmanMaterial) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman_material SET add_time = $1, is_top = $2, content = $3, \
             is_available = $4, shop_id = $5, pics = $6, category_id = $7, share_num = $8, \
             product_id = $9 WHERE id = $10",
            &[
                Bind::I(m.add_time),
                Bind::I(m.is_top),
                Bind::S(m.content.clone().unwrap_or_default()),
                Bind::I(m.is_available.unwrap_or(0)),
                Bind::I(m.shop_id),
                Bind::S(m.pics.clone().unwrap_or_default()),
                Bind::I(m.category_id.unwrap_or(0)),
                Bind::I(m.share_num.unwrap_or(0)),
                Bind::I(m.product_id.unwrap_or(0)),
                Bind::I(m.id),
            ],
        )
        .await
    }

    /// 删除素材。
    pub async fn delete_material(&self, id: i32) -> Result<u64> {
        exec(
            &self.pool,
            "DELETE FROM salesman_material WHERE id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    /// 分类下的素材数量（分类删除前校验）。
    pub async fn count_materials_by_category(&self, category_id: i32) -> Result<u64> {
        count(
            &self.pool,
            "SELECT COUNT(*) FROM salesman_material WHERE category_id = $1",
            &[Bind::I(category_id)],
        )
        .await
    }

    // ————————————————————— salesman_material_category 素材分类 —————————————————————

    /// 素材分类分页。
    pub async fn paginate_categories(
        &self,
        category_name: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanMaterialCategory>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(name) = category_name.filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "category_name ILIKE ?",
                &[Bind::S(format!("%{name}%"))],
            );
        }
        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_material_category{wh}"),
            &params,
        )
        .await?;
        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<SalesmanMaterialCategory>(
            &self.pool,
            &format!(
                "SELECT * FROM salesman_material_category{wh} \
                 ORDER BY sort_order DESC, category_id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查素材分类。
    pub async fn find_category(&self, id: i32) -> Result<Option<SalesmanMaterialCategory>> {
        fetch_one::<SalesmanMaterialCategory>(
            &self.pool,
            "SELECT * FROM salesman_material_category WHERE category_id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    /// 批量查素材分类。
    pub async fn find_categories_by_ids(
        &self,
        ids: &[i32],
    ) -> Result<Vec<SalesmanMaterialCategory>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        push_in_or_false(&mut wh, &mut params, "category_id", ids);
        fetch_rows::<SalesmanMaterialCategory>(
            &self.pool,
            &format!("SELECT * FROM salesman_material_category{wh}"),
            &params,
        )
        .await
    }

    /// 同名分类数量（\"exclude_id\" 用于更新时排除自身）。
    pub async fn count_categories_by_name(
        &self,
        category_name: &str,
        shop_id: Option<i32>,
        exclude_id: Option<i32>,
    ) -> Result<u64> {
        let mut wh = String::from(" WHERE category_name = $1");
        let mut params: Vec<Bind> = vec![Bind::S(category_name.to_string())];
        if let Some(v) = shop_id {
            push_cond(&mut wh, &mut params, "shop_id = ?", &[Bind::I(v)]);
        }
        if let Some(v) = exclude_id {
            push_cond(&mut wh, &mut params, "category_id <> ?", &[Bind::I(v)]);
        }
        count(&self.pool, &format!("SELECT COUNT(*) FROM salesman_material_category{wh}"), &params).await
    }

    /// 新增素材分类，返回自增 ID。
    pub async fn insert_category(
        &self,
        category_name: Option<&str>,
        sort_order: Option<i32>,
        shop_id: Option<i32>,
        add_time: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_material_category \
             (category_name, sort_order, shop_id, add_time) VALUES ($1, $2, $3, $4) RETURNING category_id",
        )
        .bind(category_name)
        .bind(sort_order)
        .bind(shop_id)
        .bind(add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新素材分类。
    pub async fn update_category(&self, c: &SalesmanMaterialCategory) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman_material_category SET category_name = $1, sort_order = $2, \
             shop_id = $3, add_time = $4 WHERE category_id = $5",
            &[
                Bind::S(c.category_name.clone().unwrap_or_default()),
                Bind::I(c.sort_order.unwrap_or(0)),
                Bind::I(c.shop_id.unwrap_or(0)),
                Bind::I(c.add_time.unwrap_or(0)),
                Bind::I(c.category_id),
            ],
        )
        .await
    }

    /// 删除素材分类。
    pub async fn delete_category(&self, id: i32) -> Result<u64> {
        exec(
            &self.pool,
            "DELETE FROM salesman_material_category WHERE category_id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    // ————————————————————— salesman_content 分销内容 —————————————————————

    /// 分销内容分页。
    pub async fn paginate_contents(
        &self,
        f: &SalesmanContentFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanContent>)> {
        let (limit, offset) = page.limit_offset();
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();

        if let Some(title) = f.title.as_ref().filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "title ILIKE ?",
                &[Bind::S(format!("%{title}%"))],
            );
        }
        if let (Some(start), Some(end)) = (f.start_time, f.end_time) {
            push_cond(
                &mut wh,
                &mut params,
                "start_time BETWEEN ? AND ?",
                &[Bind::I(start), Bind::I(end)],
            );
        } else if let Some(start) = f.start_time {
            push_cond(&mut wh, &mut params, "start_time >= ?", &[Bind::I(start)]);
        } else if let Some(end) = f.end_time {
            push_cond(&mut wh, &mut params, "end_time <= ?", &[Bind::I(end)]);
        }

        let now = f.now;
        if let Some(status) = f.status {
            if status == 0 {
                push_cond(&mut wh, &mut params, "start_time > ?", &[Bind::I(now)]);
            } else if status == 1 {
                push_cond(&mut wh, &mut params, "start_time <= ?", &[Bind::I(now)]);
                push_cond(
                    &mut wh,
                    &mut params,
                    "(end_time >= ? OR end_time = 0)",
                    &[Bind::I(now)],
                );
            } else {
                push_cond(&mut wh, &mut params, "end_time < ?", &[Bind::I(now)]);
                push_cond(&mut wh, &mut params, "end_time > ?", &[Bind::I(0)]);
            }
        }

        if f.from_user {
            push_cond(&mut wh, &mut params, "start_time <= ?", &[Bind::I(now)]);
            push_cond(
                &mut wh,
                &mut params,
                "(end_time >= ? OR end_time = 0)",
                &[Bind::I(now)],
            );
            push_cond(&mut wh, &mut params, "is_available = ?", &[Bind::I(1)]);
        }
        if let Some(v) = f.shop_id {
            push_cond(&mut wh, &mut params, "shop_id = ?", &[Bind::I(v)]);
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM salesman_content{wh}"),
            &params,
        )
        .await?;

        params.push(Bind::I64(limit));
        params.push(Bind::I64(offset));
        let rows = fetch_rows::<SalesmanContent>(
            &self.pool,
            &format!(
                "SELECT * FROM salesman_content{wh} \
                 ORDER BY is_top DESC, id DESC LIMIT ? OFFSET ?"
            ),
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查分销内容。
    pub async fn find_content(&self, id: i32) -> Result<Option<SalesmanContent>> {
        fetch_one::<SalesmanContent>(
            &self.pool,
            "SELECT * FROM salesman_content WHERE id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    /// 新增分销内容，返回自增 ID。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_content(
        &self,
        title: &str,
        img: &str,
        start_time: i32,
        end_time: Option<i32>,
        describe: &str,
        is_top: i32,
        content: Option<&str>,
        is_available: Option<i32>,
        shop_id: i32,
        pics: Option<&str>,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_content \
             (title, img, start_time, end_time, \"describe\", is_top, content, is_available, \
              shop_id, pics) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
        )
        .bind(title)
        .bind(img)
        .bind(start_time)
        .bind(end_time)
        .bind(describe)
        .bind(is_top)
        .bind(content)
        .bind(is_available)
        .bind(shop_id)
        .bind(pics)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新分销内容。
    pub async fn update_content(&self, c: &SalesmanContent) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman_content SET title = $1, img = $2, start_time = $3, end_time = $4, \
             \"describe\" = $5, is_top = $6, content = $7, is_available = $8, shop_id = $9, pics = $10 \
             WHERE id = $11",
            &[
                Bind::S(c.title.clone()),
                Bind::S(c.img.clone()),
                Bind::I(c.start_time),
                Bind::I(c.end_time.unwrap_or(0)),
                Bind::S(c.describe.clone()),
                Bind::I(c.is_top),
                Bind::S(c.content.clone().unwrap_or_default()),
                Bind::I(c.is_available.unwrap_or(0)),
                Bind::I(c.shop_id),
                Bind::S(c.pics.clone().unwrap_or_default()),
                Bind::I(c.id),
            ],
        )
        .await
    }

    /// 删除分销内容。
    pub async fn delete_content(&self, id: i32) -> Result<u64> {
        exec(
            &self.pool,
            "DELETE FROM salesman_content WHERE id = $1",
            &[Bind::I(id)],
        )
        .await
    }

    // ————————————————————— salesman_config 配置 —————————————————————

    /// 按 code 查配置（原 Java 未按 shop 过滤，这里保持一致）。
    pub async fn find_config(&self, code: &str) -> Result<Option<SalesmanConfig>> {
        fetch_one::<SalesmanConfig>(
            &self.pool,
            "SELECT * FROM salesman_config WHERE code = $1 LIMIT 1",
            &[Bind::S(code.to_string())],
        )
        .await
    }

    /// 按 code + 店铺查配置（保存时定位记录）。
    pub async fn find_config_in_shop(
        &self,
        code: &str,
        shop_id: Option<i32>,
    ) -> Result<Option<SalesmanConfig>> {
        fetch_one::<SalesmanConfig>(
            &self.pool,
            "SELECT * FROM salesman_config WHERE code = $1 AND shop_id = $2 LIMIT 1",
            &[Bind::S(code.to_string()), Bind::I(shop_id.unwrap_or(0))],
        )
        .await
    }

    /// 新增配置。
    pub async fn insert_config(
        &self,
        code: &str,
        shop_id: Option<i32>,
        data: &str,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_config (code, shop_id, data) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(code)
        .bind(shop_id)
        .bind(data)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新配置内容。
    pub async fn update_config(&self, id: i32, data: &str) -> Result<u64> {
        exec(
            &self.pool,
            "UPDATE salesman_config SET data = $1 WHERE id = $2",
            &[Bind::S(data.to_string()), Bind::I(id)],
        )
        .await
    }
}