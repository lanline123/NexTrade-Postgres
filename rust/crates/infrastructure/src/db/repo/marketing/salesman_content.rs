//! 分销内容域仓储：`salesman_content`（兼查 `salesman` 解析 C 端店铺归属）。
//!
//! 对应原 Java `SalesmanContentMapper` / `SalesmanServiceImpl#lambdaQuery()`。
//!
//! 列名严格对齐 `java/database/pure.sql`（已用
//! `python tools/schema_parser.py --table salesman_content` 核对）：
//! `id` / `title` / `img` / `start_time` / `end_time` / `` `describe` `` /
//! `is_top` / `content` / `is_available` / `shop_id` / `pics`。
//!
//! 列表过滤条件逐字复刻 `SalesmanContentServiceImpl#list`：
//! 1. `shop_id = ?`（Java 无条件 `eq`，`null` 时 SQL `= NULL` 恒不命中）
//! 2. `title LIKE ?`
//! 3. 时间范围：两者都给 → `start_time BETWEEN`；只给开始 → `start_time >=`；
//!    只给结束 → `end_time <=`（Java 原样，注意结束条件落在 `end_time`）
//! 4. 状态筛选：
//!    - `0` 未开始 → `start_time > now`
//!    - `1` 展示中 → `start_time <= now AND (end_time >= now OR end_time = 0)`
//!    - 其它 → `end_time < now AND end_time > 0`
//! 5. C 端（`from == "user"`）→ `start_time <= now AND (end_time >= now OR
//!    end_time = 0) AND is_available = 1`
//!
//! 排序由服务层按 Java `BaseServiceImpl#buildSortOrder` 归一后传入白名单列，
//! 这里只负责拼 `ORDER BY`（列名不来自用户输入，无注入面）。

use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::SalesmanContent;
use nextrade_core::models::marketing::salesman_content::SalesmanContentFilter;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `salesman_content` 表全部列（顺序与 [`SalesmanContent`] 字段一致）。
const CONTENT_COLUMNS: &str = "id, title, img, start_time, end_time, \"describe\", is_top, \
     content, is_available, shop_id, pics";

#[derive(Clone)]
pub struct SalesmanContentRepository {
    pool: PgPool,
}

impl SalesmanContentRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层需要时复用；本域无跨语句事务需求）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 读 —————————————————————

    /// 分销内容分页（对应 Java `SalesmanContentServiceImpl#list`）。
    ///
    /// `sort_column` 必须来自
    /// [`nextrade_core::models::marketing::salesman_content::resolve_sort`] 的白名单；
    /// `desc == None` 表示 Java 未设置 `ORDER BY`（不拼排序子句）。
    pub async fn paginate(
        &self,
        f: &SalesmanContentFilter,
        sort_column: &'static str,
        desc: Option<bool>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<SalesmanContent>)> {
        let (limit, offset) = page.limit_offset();

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM salesman_content WHERE 1 = 1");
        Self::push_filters(&mut count_qb, f);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> = QueryBuilder::new(format!(
            "SELECT {CONTENT_COLUMNS} FROM salesman_content WHERE 1 = 1"
        ));
        Self::push_filters(&mut row_qb, f);
        row_qb.push(order_clause(sort_column, desc));
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));

        let rows: Vec<SalesmanContent> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询分销内容（对应 Java `getById(id)`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<SalesmanContent>> {
        let sql = format!("SELECT {CONTENT_COLUMNS} FROM salesman_content WHERE id = ? LIMIT 1");
        sqlx::query_as::<_, SalesmanContent>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按会员 ID 查分销员所属店铺（Java `salesmanService.lambdaQuery()
    /// .eq(userId, currentUserId).last("limit 1").one()`）。
    ///
    /// 返回三态：
    /// - `Ok(None)`：该会员不是分销员（Java `salesman == null` → 调用方用 `-1`）
    /// - `Ok(Some(None))`：是分销员但 \"shop_id\" 为 `NULL`（Java 会把 `null`
    ///   直接塞进 `eq(shop_id, null)` → 恒不命中）
    /// - `Ok(Some(Some(shop_id)))`：正常店铺归属
    pub async fn find_salesman_shop_id_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<Option<Option<i32>>> {
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT shop_id FROM salesman WHERE user_id = ? LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(shop_id,)| shop_id))
    }

    // ————————————————————— 写 —————————————————————

    /// 新增分销内容，返回自增主键（对应 Java \"save\"）。
    ///
    /// \"end_time\" / \"is_available\" 为 \"None\" 时按 DB 默认值写入
    /// （`end_time=0`、`is_available=1`），与 Java 省略 \"null\" 列后由 DB 默认
    /// 值兜底的行为一致。
    pub async fn insert(&self, c: &SalesmanContent) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO salesman_content \
             (title, img, start_time, end_time, \"describe\", is_top, content, is_available, \
              shop_id, pics) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&c.title)
        .bind(&c.img)
        .bind(c.start_time)
        .bind(c.end_time.unwrap_or(0))
        .bind(&c.describe)
        .bind(c.is_top)
        .bind(c.content.as_deref())
        .bind(c.is_available.unwrap_or(1))
        .bind(c.shop_id)
        .bind(c.pics.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 整行更新分销内容（对应 Java \"updateById\"；未提交字段由服务层合并原值）。
    pub async fn update(&self, c: &SalesmanContent) -> Result<u64> {
        let r = sqlx::query(
            "UPDATE salesman_content SET title = ?, img = ?, start_time = ?, end_time = ?, \
             \"describe\" = ?, is_top = ?, content = ?, is_available = ?, shop_id = ?, pics = ? \
             WHERE id = ?",
        )
        .bind(&c.title)
        .bind(&c.img)
        .bind(c.start_time)
        .bind(c.end_time)
        .bind(&c.describe)
        .bind(c.is_top)
        .bind(c.content.as_deref())
        .bind(c.is_available)
        .bind(c.shop_id)
        .bind(c.pics.as_deref())
        .bind(c.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 更新单个字段（对应 Java `BaseServiceImpl#updateField`）。
    ///
    /// \"column\" **必须**来自
    /// [`nextrade_core::models::marketing::salesman_content::SALESMAN_CONTENT_FIELDS`]
    /// 白名单；这里再做一次本地兜底断言（非白名单列直接拒绝）。
    /// `val == None` 时绑定 SQL \"NULL\"（Java `set(column, null)` 同义）。
    pub async fn update_field(
        &self,
        id: i32,
        column: &'static str,
        val: Option<&str>,
    ) -> Result<u64> {
        let safe = match column {
            "title" => "title",
            "is_top" => "is_top",
            "is_available" => "is_available",
            _ => return Err(Error::Validation("field 的值有误".into())),
        };
        let sql = format!("UPDATE salesman_content SET {safe} = ? WHERE id = ?");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 只更新失效时间（`updateField(isAvailable, "0")` 的联动副作用）。
    pub async fn set_end_time(&self, id: i32, end_time: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE salesman_content SET end_time = ? WHERE id = ?")
            .bind(end_time)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 按主键删除（对应 `BaseServiceImpl#del` → \"removeById\"）。
    pub async fn delete(&self, id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM salesman_content WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量删除（对应 `BaseServiceImpl#batch` → \"removeByIds\"）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("DELETE FROM salesman_content WHERE id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(")");
        drop(sep);
        let r = qb.build().execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 追加列表 WHERE 片段（count 与取行共用，保证条件一致）。
    fn push_filters(qb: &mut QueryBuilder<'_, Postgres>, f: &SalesmanContentFilter) {
        // Java：`queryWrapper.eq(SalesmanContent::getShopId, shopId)`（无 null 保护）
        qb.push(" AND shop_id = ");
        qb.push_bind(f.shop_id);

        if let Some(title) = f.title.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            qb.push(" AND title LIKE ");
            qb.push_bind(format!("%{title}%"));
        }

        match (f.start_time, f.end_time) {
            (Some(start), Some(end)) => {
                qb.push(" AND start_time BETWEEN ");
                qb.push_bind(start);
                qb.push(" AND ");
                qb.push_bind(end);
            }
            (Some(start), None) => {
                qb.push(" AND start_time >= ");
                qb.push_bind(start);
            }
            // Java 原样：只给结束日期时过滤的是 `end_time`
            (None, Some(end)) => {
                qb.push(" AND end_time <= ");
                qb.push_bind(end);
            }
            (None, None) => {}
        }

        if let Some(status) = f.status {
            if status == 0 {
                qb.push(" AND start_time > ");
                qb.push_bind(f.now);
            } else if status == 1 {
                qb.push(" AND start_time <= ");
                qb.push_bind(f.now);
                qb.push(" AND (end_time >= ");
                qb.push_bind(f.now);
                qb.push(" OR end_time = 0)");
            } else {
                qb.push(" AND end_time < ");
                qb.push_bind(f.now);
                qb.push(" AND end_time > 0");
            }
        }

        if f.from_user {
            qb.push(" AND start_time <= ");
            qb.push_bind(f.now);
            qb.push(" AND (end_time >= ");
            qb.push_bind(f.now);
            qb.push(" OR end_time = 0)");
            qb.push(" AND is_available = 1");
        }
    }
}

/// `ORDER BY` 子句（列名已由 models 白名单归一，无注入面）。
fn order_clause(column: &'static str, desc: Option<bool>) -> String {
    match desc {
        Some(true) => format!(" ORDER BY {column} DESC"),
        Some(false) => format!(" ORDER BY {column} ASC"),
        // Java：既非 asc 也非 desc → 不设置排序
        None => String::new(),
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("分销内容不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_clause_matches_java_build_sort_order() {
        // 缺省 desc（Java sortOrder 为空 → desc）
        assert_eq!(order_clause("id", Some(true)), " ORDER BY id DESC");
        assert_eq!(
            order_clause("is_top", Some(false)),
            " ORDER BY is_top ASC"
        );
        // 非 asc/desc → 不拼 ORDER BY
        assert_eq!(order_clause("title", None), "");
    }

    #[test]
    fn order_clause_only_uses_whitelisted_columns() {
        // 列名来自 models::resolve_sort 的白名单，这里做契约式断言：
        for column in nextrade_core::models::marketing::salesman_content::SORTABLE_COLUMNS {
            let sql = order_clause(column, Some(true));
            assert_eq!(sql, format!(" ORDER BY {column} DESC"));
        }
    }
}
