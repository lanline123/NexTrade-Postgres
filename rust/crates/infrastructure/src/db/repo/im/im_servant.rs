//! IM 客服（ImServantController） 域仓储（im::im_servant）。
//!
//! 对应原 Java `com.nextrade.mapper.im.ImServantMapper`（`im_servant`）与
//! `com.nextrade.mapper.authority.AdminUserMapper`（`admin_user`，本仓储只读
//! `admin_id` / `username` / `avatar`）。
//!
//! 列名严格对齐权威 schema
//! （`cd rust && python tools/schema_parser.py --table im_servant`）：
//! - `im_servant`：`id` / `servant_id` / `last_update_time` / `add_time` /
//!   `status` / `shop_id` / `conversation_num`
//! - `admin_user`（只读）：`admin_id` / `username` / `avatar`
//!
//! 约定：一律用**运行时** `sqlx::query_as` / `sqlx::query`，**不用** `query!` 宏
//! （本机无 `DATABASE_URL`）；每条 SQL 写成单行字面量，便于
//! `python tools/check_sql_columns.py --strict` 静态校验。

use sqlx::PgPool;
use nextrade_core::entities::ImServant;
use nextrade_core::{Error, Result};

/// `im_servant` 全部列（顺序与实体 [`ImServant`] 字段一致，供 `query_as` 映射）。
const IM_SERVANT_COLUMNS: &str =
    "id, servant_id, last_update_time, add_time, status, shop_id, conversation_num";

/// `im_servant` 可排序列白名单（对应 Java `BaseServiceImpl.getColumnByPropertyName`
/// 的防注入兜底：未知道段回退主键 `id`）。
const SORTABLE_COLUMNS: [&str; 7] = [
    "id",
    "servant_id",
    "last_update_time",
    "add_time",
    "status",
    "shop_id",
    "conversation_num",
];

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct ImServantRepository {
    pool: PgPool,
}

impl ImServantRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 列表分页（对应 Java `ImServantServiceImpl#list` 的 `this.page(...)`）。
    ///
    /// - `shop_id`：`Some(v)` → 追加 `shop_id = v`；`None`（Java 返回 null）→ 不加条件；
    /// - `servant_ids`：用户名 LIKE 命中的客服 id 集合。`Some(非空)` → 追加
    ///   `servant_id IN (...)`；`Some(空)` 或 `None`：
    ///   * `None` 表示未提供 username 过滤（不加 IN 条件）；
    ///   * `Some(空)` 表示有 username 但无匹配 → 直接返回 `(0, [])`
    ///     （对应 Java `if(matchedServantIds.isEmpty()) return new Page<>();`）。
    /// - `sort_column` / `dir`：已由服务层走白名单，防注入。
    pub async fn paginate(
        &self,
        shop_id: Option<i32>,
        servant_ids: Option<&[i32]>,
        sort_column: &str,
        dir: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(u64, Vec<ImServant>)> {
        // username 过滤命中为空 → 空页（不查库，对齐 Java 提前 return）
        if servant_ids.is_some_and(<[i32]>::is_empty) {
            return Ok((0, Vec::new()));
        }

        let mut where_parts: Vec<String> = Vec::new();
        if shop_id.is_some() {
            where_parts.push("shop_id = ?".to_string());
        }
        let ids_clause = match servant_ids {
            Some(ids) => {
                let placeholders = vec!["?"; ids.len()].join(", ");
                where_parts.push(format!("servant_id IN ({placeholders})"));
                true
            }
            None => false,
        };
        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        // —— COUNT ——
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM im_servant {where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if shop_id.is_some() {
            cq = cq.bind(shop_id);
        }
        if ids_clause {
            for id in servant_ids.unwrap_or_default() {
                cq = cq.bind(id);
            }
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        // —— LIST ——
        let list_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {IM_SERVANT_COLUMNS} FROM im_servant {where_sql} ORDER BY {sort_column} {dir} LIMIT ? OFFSET ?"
        ));
        let mut lq = sqlx::query_as::<_, ImServant>(&list_sql);
        if shop_id.is_some() {
            lq = lq.bind(shop_id);
        }
        if ids_clause {
            for id in servant_ids.unwrap_or_default() {
                lq = lq.bind(id);
            }
        }
        let rows = lq
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 用户名模糊匹配命中的客服 id 列表（对应 Java `list` 里
    /// `adminUserMapper.selectList(... like(username)).map(AdminId)`）。
    pub async fn find_admin_ids_by_username(&self, username: &str) -> Result<Vec<i32>> {
        let like = format!("%{username}%");
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT admin_id FROM admin_user WHERE username LIKE $1",
        )
        .bind(&like)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// 批量加载后台用户（`admin_id` / `username` / `avatar`），
    /// 对应 Java `list` 末尾的 `adminUserMapper.selectList(in(adminId, ids))`。
    ///
    /// 返回元组顺序即 `(admin_id, username, avatar)`。
    pub async fn find_users_by_ids(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(i32, String, Option<String>)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql =
            crate::db::to_pg_placeholders(&format!("SELECT admin_id, username, avatar FROM admin_user WHERE admin_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, (i32, String, Option<String>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// `modifyStatus` 的取行（对应 Java `getOne(eq(servantId).eq(shopId))`）。
    ///
    /// ⚠️ 与 `find_one_for_status` 不同：这里**恒定**拼 `shop_id = ?`，
    /// 即便 `shop_id` 为 `NULL`（未知账号类型），照抄 MyBatis-Plus
    /// `eq(col, null)` 生成的 `shop_id = NULL`（MySQL 下恒不成立 → 走 INSERT 分支）。
    pub async fn find_one_for_modify(
        &self,
        servant_id: i32,
        shop_id: Option<i32>,
    ) -> Result<Option<ImServant>> {
        let row: Option<ImServant> = sqlx::query_as::<_, ImServant>(&format!(
            "SELECT {IM_SERVANT_COLUMNS} FROM im_servant WHERE servant_id = $1 AND shop_id = $2 LIMIT 1"
        ))
        .bind(servant_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// `getStatus` 的取行（对应 Java `getOne(eq(servantId)` +
    /// `if(shopId != null) eq(shopId))`）。
    ///
    /// `shop_id` 为 `None` 时**不加** `shop_id` 条件（与 `find_one_for_modify` 相反）。
    pub async fn find_one_for_status(
        &self,
        servant_id: i32,
        shop_id: Option<i32>,
    ) -> Result<Option<ImServant>> {
        let (sql, bind_shop) = if shop_id.is_some() {
            (
                format!("SELECT {IM_SERVANT_COLUMNS} FROM im_servant WHERE servant_id = $1 AND shop_id = $2 LIMIT 1"),
                true,
            )
        } else {
            (
                format!("SELECT {IM_SERVANT_COLUMNS} FROM im_servant WHERE servant_id = $1 LIMIT 1"),
                false,
            )
        };
        let mut q = sqlx::query_as::<_, ImServant>(&sql).bind(servant_id);
        if bind_shop {
            q = q.bind(shop_id);
        }
        let row = q.fetch_optional(&self.pool).await.map_err(map_err)?;
        Ok(row)
    }

    /// 新增客服行，返回自增主键（对应 Java `this.save(imServant)`）。
    ///
    /// `add_time` / `last_update_time` 由服务层传入的当前秒级时间戳写入
    /// （对应 `StringUtils.getCurrentTime()`）。
    pub async fn insert_servant(
        &self,
        servant_id: Option<i32>,
        shop_id: Option<i32>,
        status: Option<i32>,
        now: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar("INSERT INTO im_servant (servant_id, shop_id, status, add_time, last_update_time) VALUES ($1, $2, $3, $4, $5) RETURNING id")
            .bind(servant_id)
            .bind(shop_id)
            .bind(status)
            .bind(now)
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键更新状态 / 店铺 / 最后更新时间
    /// （对应 Java `updateById`：`setStatus` + `setShopId` + `setLastUpdateTime`）。
    pub async fn update_status(
        &self,
        id: i32,
        status: Option<i32>,
        shop_id: Option<i32>,
        now: i32,
    ) -> Result<()> {
        sqlx::query("UPDATE im_servant SET status = $1, shop_id = $2, last_update_time = $3 WHERE id = $4")
            .bind(status)
            .bind(shop_id)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }
}

/// 排序列白名单解析（未知 / 缺失回退主键 `id`，防 SQL 注入）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            SORTABLE_COLUMNS
                .iter()
                .find(|c| **c == snake.as_str())
                .copied()
                .unwrap_or("id")
        }
        None => "id",
    }
}

/// Java `buildSortOrder`：仅显式 `asc` 升序，`desc` / 空 / 其它一律降序。
#[must_use]
pub fn resolve_sort_dir(order: Option<&str>) -> &'static str {
    if order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
        "ASC"
    } else {
        "DESC"
    }
}

/// 驼峰 -> 下划线（允许前端传驼峰或数据库列名）。
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("客服不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_column_whitelist_falls_back_to_id() {
        assert_eq!(resolve_sort_column(None), "id");
        assert_eq!(resolve_sort_column(Some("id")), "id");
        // 驼峰 -> 列名
        assert_eq!(resolve_sort_column(Some("servantId")), "servant_id");
        assert_eq!(resolve_sort_column(Some("lastUpdateTime")), "last_update_time");
        assert_eq!(resolve_sort_column(Some("conversationNum")), "conversation_num");
        // 注入 / 未知字段回退主键
        assert_eq!(resolve_sort_column(Some("1;DROP TABLE im_servant")), "id");
    }

    #[test]
    fn sort_dir_only_asc_is_asc_otherwise_desc() {
        assert_eq!(resolve_sort_dir(Some("asc")), "ASC");
        assert_eq!(resolve_sort_dir(Some("ASC")), "ASC");
        assert_eq!(resolve_sort_dir(Some("desc")), "DESC");
        assert_eq!(resolve_sort_dir(None), "DESC");
        assert_eq!(resolve_sort_dir(Some("")), "DESC");
    }

    #[test]
    fn snake_conversion_is_idempotent() {
        assert_eq!(to_snake("servant_id"), "servant_id");
        assert_eq!(to_snake("servantId"), "servant_id");
    }
}
