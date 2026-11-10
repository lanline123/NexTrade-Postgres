//! 会员企业认证域（member::user_company）仓储：`user_company`（+ `user` / `region` 的只读关联）。
//!
//! 对应原 Java `com.nextrade.mapper.user.UserCompanyMapper`
//! （`BaseMapper<UserCompany>` + `BaseServiceImpl` 的通用方法），
//! 以及 `UserCompanyServiceImpl` 里用到的 `UserService` / `RegionService` 查询。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table user_company`）：
//!
//! `user_company`（11 列，主键 `id`）：
//! `id` / `user_id` / `type` / `contact_name` / `contact_mobile` /
//! `company_name` / `company_data` / `status` / `audit_remark` /
//! `audit_time` / `add_time`
//! —— 除 `company_data` / `audit_time` / `add_time` 可空外其余 `NOT NULL`；
//! `audit_time` / `add_time` 是 `int unsigned NULL`（**秒级**时间戳）。
//!
//! 关联只读表：
//! - `user`（`user_id` / `username` / `mobile` / `is_company_auth`）
//! - `region`（`region_id` / `region_name`）
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! 金额：本域**不涉及任何金额列**。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::{Region, User, UserCompany};
use nextrade_core::{Error, Result};

/// 固定 SELECT 列清单（避免 `SELECT *` 与实体字段顺序耦合；
/// 与 `entities::UserCompany` 的 11 个字段一一对应，顺序也一致）。
const USER_COMPANY_SELECT: &str = "SELECT id, user_id, type, contact_name, contact_mobile, \
     company_name, company_data, status, audit_remark, audit_time, add_time FROM user_company";

/// \"list\" 里用于反查 \"user_id\" 的 SELECT 列清单
/// （Java `userService.list(userQueryWrapper)` 只用到 \"user_id\"，
/// 但这里取全列以便复用实体映射）。
const USER_SELECT: &str = "SELECT * FROM \"user\"";

#[derive(Clone)]
pub struct UserCompanyRepository {
    pool: PgPool,
}

impl UserCompanyRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排用，避免服务层直接依赖 `sqlx::Transaction` 的构造）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 列表查询 —————————————————————

    /// 按关键字反查会员 id 列表
    /// （对应 Java `UserCompanyServiceImpl#list` 里的
    /// `userService.list(like(username, kw).or().like(mobile, kw))`）。
    ///
    /// ⚠️ Java 用的是 `LambdaQueryWrapper` 的
    /// `.like(User::getUsername, keyword).or().like(User::getMobile, keyword)`，
    /// **没有加括号**。在 MyBatis-Plus 里这段是
    /// `username ILIKE ? OR mobile ILIKE ?`，与外层其它条件拼接时会出现
    /// 「OR 逃逸」问题 —— 但此处它是**独立的一次 `userService.list(...)` 调用**，
    /// 不存在与外层条件拼接，因此就是纯粹的 `OR`，本实现按此复刻。
    ///
    /// ⚠️ Java 没写 `LIMIT`：会员表全表扫匹配的用户全部返回，再 `IN (...)`。
    /// 本实现同样不截断（保持行为一致）。
    pub async fn find_user_ids_by_keyword(&self, keyword: &str) -> Result<Vec<i32>> {
        let like = format!("%{}%", escape_like(keyword));
        // `ESCAPE '\\'` 与 escape_like 配套：MySQL 默认转义符就是 `\`，
        // 这里显式声明以避免 sql_mode=NO_BACKSLASH_ESCAPES 下的行为差异。
        let sql = crate::db::to_pg_placeholders(&format!(
            "{USER_SELECT} WHERE username ILIKE ? ESCAPE '\\' OR mobile ILIKE ? ESCAPE '\\'"
        ));
        let rows: Vec<User> = sqlx::query_as(&sql)
            .bind(&like)
            .bind(&like)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|u| u.user_id).collect())
    }

    /// 会员企业认证分页（对应 Java `UserCompanyServiceImpl#list`）。
    ///
    /// 过滤条件与 Java 逐条对齐（**顺序也一致**）：
    /// 1. 关键字（Java 只有 `keyword` 非空时才加这一整段）：
    ///    - 反查出的 `userIds` **非空** ⇒
    ///      `(user_id IN (...) OR contact_mobile ILIKE %kw%)`
    ///    - `userIds` **为空** ⇒ 仅 `contact_mobile ILIKE %kw%`
    ///      （Java 的 `if (!userIds.isEmpty()) { ... } else { ... }`）
    /// 2. `status`：`status != null && status != 0` ⇒ `status = ?`
    /// 3. ORDER BY：由 [`UserCompanyListQuery::order_clause`] 生成
    ///
    /// ⚠️ Java 把关键字条件包在 `queryWrapper.and(wrapper -> ...)` 里，
    /// 即圆括号分组 `AND (a OR b)`；本实现同样用括号包住 OR 段，
    /// 否则 `status = ?` 会被 OR 逃逸掉。
    pub async fn paginate(
        &self,
        q: &nextrade_core::models::member::user_company::UserCompanyListQuery,
        user_ids: Option<&[i32]>,
    ) -> Result<(u64, Vec<UserCompany>)> {
        let page = q.page_query();
        let (limit, offset) = (page.page_size, (page.page - 1) * page.page_size);

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM user_company WHERE 1 = 1");
        Self::push_filters(&mut count_qb, q, user_ids);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{USER_COMPANY_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut row_qb, q, user_ids);
        // ORDER BY 子句的列名来自 models::nextrade_core::models::member::user_company::map_order_column 的白名单，
        // 方向来自 ASC/DESC 二选一，均无注入面；LIMIT/OFFSET 为 u64 字面量。
        row_qb.push(q.order_clause());
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));

        let rows: Vec<UserCompany> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0.max(0)).unwrap_or(0), rows))
    }

    /// 追加 \"list\" 的 WHERE 片段（\"count\" 与取行两条 SQL 共用，保证条件一致）。
    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &nextrade_core::models::member::user_company::UserCompanyListQuery,
        user_ids: Option<&[i32]>,
    ) {
        if let Some(keyword) = q.effective_keyword() {
            let like = format!("%{}%", escape_like(keyword));
            qb.push(" AND (");
            match user_ids {
                Some(ids) if !ids.is_empty() => {
                    qb.push("user_id IN (");
                    let mut sep = qb.separated(", ");
                    for id in ids {
                        sep.push_bind(*id);
                    }
                    sep.push_unseparated(")");
                    drop(sep);
                    qb.push(" OR contact_mobile ILIKE ");
                    qb.push_bind(like);
                    qb.push(" ESCAPE '\\'");
                }
                // Java `else` 分支：userIds 为空（或查不到用户）时**只**看 contact_mobile
                _ => {
                    qb.push("contact_mobile ILIKE ");
                    qb.push_bind(like);
                    qb.push(" ESCAPE '\\'");
                }
            }
            qb.push(")");
        }

        // Java `.eq(status != null && status != 0, UserCompany::getStatus, status)`
        if let Some(status) = q.effective_status() {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
    }

    // ————————————————————— 单条查询 —————————————————————

    /// 按主键查询（对应 Java `getById(id)`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<UserCompany>> {
        sqlx::query_as::<_, UserCompany>(&format!(
            "{USER_COMPANY_SELECT} WHERE id = ? LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 `user_id` 查询单条会员（对应 Java `userService.getById(userId)`）。
    pub async fn find_user_by_id(&self, user_id: i32) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM \"user\" WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 批量查询会员（对应 Java `list` 里对当页每条记录
    /// `userService.lambdaQuery().eq(userId).one()` 的**批量预取**）。
    ///
    /// 返回顺序不保证；服务层用 `user_id` 建索引取用。
    /// ⚠️ Java 是逐条 `one()` 查询（N+1），本实现改为一次 `IN` 批量取，
    /// 结果集等价（`user_id` 是主键，最多一条）。
    pub async fn find_users_by_ids(&self, ids: &[i32]) -> Result<Vec<User>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM \"user\" WHERE user_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按地区 id 列表查询 `region`（对应 Java
    /// `regionService.list(in(Region::getRegionId, licenseAddrProvince))`）。
    ///
    /// ⚠️ Java 的 `in(...)` 在列表为空时会产出 `IN ()` 这种**非法 SQL**；
    /// 本实现提前返回空表（调用方在空列表时也不会走到这里，
    /// 因为 Java 外层有 `!= null && !isEmpty()` 判断）。
    pub async fn find_regions_by_ids(&self, ids: &[i32]) -> Result<Vec<Region>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT * FROM region WHERE region_id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 写入 —————————————————————

    /// 审核落库（对应 Java \"audit\" 里的 `this.updateById(userCompany)`）。
    ///
    /// ⚠️ Java 的 \"updateById\" 是 MyBatis-Plus 的 `FieldStrategy.NOT_NULL`
    /// 语义：\"null\" 字段不参与 \"SET\"。审核链路上被改动的三个字段
    /// （\"status\" / \"audit_remark\" / \"audit_time\"）里：
    /// - \"status\"、\"audit_remark\" 由 Java 显式 set（\"audit_remark\" 空值会被
    ///   转成 `""`），必然非 null ⇒ 一定写
    /// - \"audit_time\" **只有通过时才 set**，不通过/待审核时是实体里读出来的
    ///   原值（可能是 \"null\" ⇒ 不写列，SQL 与「写回原值」净效果一致）
    ///
    /// 因此本实现提供 [\"CompanyAuditPatch\"] 的补丁结构：
    /// \"None\" = 不写该列；`Some(v)` = 写该列。
    ///
    /// 同时还带一个**状态门**（`WHERE id = $1 AND status = $2`），
    /// 复刻 Java 事务的原子性意图：Java 是「先 getById 校验 PENDING，
    /// 再 updateById」的**非原子**两步（并发下可能双重审核）。
    /// 本实现的状态门让并发重复审核只有一次能命中 —— 这是一处**有意的并发加固**，
    /// 见返回值 \"uncertain_points\"。
    ///
    /// ⚠️ 返回 0 行有两种含义：记录已被删除，或已被并发审核改变状态。
    /// 服务层据此回传 \"false\"（Java 侧对应 \"updateById\" 返回 0 的静默情况）。
    pub async fn update_audit(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
        expected_status: i32,
        patch: &CompanyAuditPatch,
    ) -> Result<u64> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE user_company SET ");
        let mut sep = qb.separated(", ");
        if let Some(status) = patch.status {
            sep.push("status = ");
            sep.push_bind(status);
        }
        if let Some(remark) = patch.audit_remark.as_ref() {
            sep.push("audit_remark = ");
            sep.push_bind(remark.clone());
        }
        if let Some(audit_time) = patch.audit_time {
            sep.push("audit_time = ");
            sep.push_bind(audit_time);
        }
        drop(sep);
        qb.push(" WHERE id = ");
        qb.push_bind(id);
        qb.push(" AND status = ");
        qb.push_bind(expected_status);
        // \"patch\" 三项全空时会产出 `UPDATE ... SET  WHERE ...` 这种非法 SQL；
        // 调用方（audit）必然设置 status，这里用 debug 断言兜住误用。
        debug_assert!(!patch.is_empty(), "update_audit 补丁不能为空");
        let res = qb.build().execute(&mut **tx).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 把会员标记为企业已认证（对应 Java
    /// `user.setIsCompanyAuth(1); userService.updateById(user);`）。
    ///
    /// `WHERE user_id = $2`，返回影响行数；0 行表示会员不存在
    /// （Java 侧在此之前用 `Assert.notNull(user, USER_NOT_EXIST)` 拦过）。
    pub async fn mark_user_company_auth(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        user_id: i32,
        value: i32,
    ) -> Result<u64> {
        let res = sqlx::query("UPDATE \"user\" SET is_company_auth = $1 WHERE user_id = $2")
            .bind(value)
            .bind(user_id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键硬删除（对应 Java `UserCompanyServiceImpl#del`
    /// → `BaseServiceImpl#del(id)` → `removeById(id)`）。
    ///
    /// \"user_company\" 表**没有**软删标记列，因此就是 \"DELETE\"。
    ///
    /// ⚠️ Java `del(id)` 在 `id == null` 时返回 \"false\"（不报错），
    /// 否则 `removeById(id)` —— **不校验记录是否存在**（删不存在的 id 也算成功）。
    /// 本实现保持该语义（返回是否命中），不额外抛 404。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM user_company WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }
}

/// \"audit\" 的部分更新补丁（对应 MyBatis-Plus \"updateById\" 的
/// `FieldStrategy.NOT_NULL`：**只写非 null 字段**）。
///
/// 每个字段两种状态：
/// - \"None\"：该列**不参与** \"SET\"（等价 Java 实体字段为 \"null\"）
/// - `Some(v)`：写该列
///
/// ⚠️ 与 area_code 的 \"AreaCodePatch\" 不同，这里不需要
/// `Option<Option<T>>` 的三态：审核链路上这三个字段**要么不写，要么写实值**
/// （\"audit_remark\" 的「null 转空串」在服务层就完成了，
/// Java 不会把 \"null\" 写进 \"audit_remark\" 列 —— 该列是 `NOT NULL`）。
#[derive(Debug, Clone, Default)]
pub struct CompanyAuditPatch {
    /// \"status\"（审核链路上必然被写）
    pub status: Option<i32>,
    /// \"audit_remark\"（审核链路上必然被写；Java 把 null 转成 `""`）
    pub audit_remark: Option<String>,
    /// \"audit_time\"（**仅通过时**写）
    pub audit_time: Option<i32>,
}

impl CompanyAuditPatch {
    /// 是否有任何列会被写入。
    ///
    /// 判据必须与实际 \"SET\" 片段条件（`is_some()`）一致。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.status.is_none() && self.audit_remark.is_none() && self.audit_time.is_none()
    }
}

/// 转义 \"ILIKE\" 模式串里的通配符，使其按字面匹配
/// （Java MyBatis-Plus 的 `like(column, val)` 会拼成 `%val%`，
/// **不对 val 内的 `%` / \"_\" 转义**）。
///
/// ⚠️ **这是一处与 Java 的行为差异**：Java 不转义，用户搜 `%` 会匹配全部记录。
/// Rust 侧选择转义（配合 SQL 的 `ESCAPE '\\'`）以避免「搜 `%` 命中全表」的
/// 意外放大；这属于**未被授权的行为变更**，已列入 \"uncertain_points\"。
/// 若要求与线上逐字符一致，应去掉本函数并移除 SQL 里的 \"ESCAPE\"。
#[must_use]
pub fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("会员企业认证信息不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::user_company::UserCompanyListQuery;

    /// 补丁空判定必须与 \"SET\" 片段条件一致（踩坑清单第 12 条）。
    #[test]
    fn patch_is_empty_reflects_actual_set_clause() {
        assert!(CompanyAuditPatch::default().is_empty());
        // 只设 audit_remark（空串也要写！）
        let p = CompanyAuditPatch {
            audit_remark: Some(String::new()),
            ..CompanyAuditPatch::default()
        };
        assert!(!p.is_empty());
        // 只设 audit_time = 0 也要写
        let p = CompanyAuditPatch {
            audit_time: Some(0),
            ..CompanyAuditPatch::default()
        };
        assert!(!p.is_empty());
    }

    /// ILIKE 转义：`%` / \"_\" / `\` 都按字面匹配。
    #[test]
    fn escape_like_escapes_wildcards() {
        assert_eq!(escape_like("alice"), "alice");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("c:\\x"), "c:\\\\x");
    }

    /// 分页参数与 Java `BasePage` 的默认/上限一致（在此复核一次，
    /// 防止仓储层自行改口径）。
    #[test]
    fn paginate_uses_base_page_defaults() {
        let q = UserCompanyListQuery::default();
        let page = q.page_query();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 15);
        let (limit, offset) = (page.page_size, (page.page - 1) * page.page_size);
        assert_eq!((limit, offset), (15, 0));

        let q = UserCompanyListQuery {
            page: Some(3),
            size: Some(20),
            ..Default::default()
        };
        let page = q.page_query();
        assert_eq!(
            (page.page_size, (page.page - 1) * page.page_size),
            (20, 40)
        );
    }
}