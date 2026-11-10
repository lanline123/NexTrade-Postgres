//! PC 导航栏域仓储（content::pc_navigation）：单表 `pc_navigation`。
//!
//! 列名严格对齐权威 schema
//! （`python tools/schema_parser.py --table pc_navigation`，权威源
//! `java/database/pure.sql`），共 9 列且全部 `NOT NULL`：
//! `id` / `title` / `is_show` / `sort_order` / `is_blank` / `link` /
//! `type` / `parent_id` / `icon`。
//!
//! 约定：
//! - 一律使用**运行时** `sqlx::query_as` / `sqlx::query`，**不使用** `query!` 宏
//!   （本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 每条 SQL 都是**单行字符串字面量**，便于
//!   `python tools/check_sql_columns.py` 静态校验列名。
//! - `type` 是 MySQL 关键字，SQL 中一律写作 `` `type` ``。
//! - 动态片段（排序列、`IN` 占位符）只由**编译期常量 / 白名单**生成，
//!   绝不含用户输入。
//! - `pc_navigation.link` 是 `text NOT NULL`，存 JSON 文本，故实体字段为 `String`。

use sqlx::PgPool;
use nextrade_core::entities::PcNavigation;
use nextrade_core::models::content::pc_navigation::PcNavigationPatch;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `pc_navigation` 全部 9 列（顺序与 [`PcNavigation`] 实体字段一致）。
///
/// 服务层 `NAV_COLUMNS.split(',')` 单测保证它与实体字段一一对应。
pub const NAV_COLUMNS: &str =
    "id, title, is_show, sort_order, is_blank, link, \"type\", parent_id, icon";

/// `updateField` 允许更新的列（`CheckFieldConstants.PC_NAVIGATION_FIELDS` 的有效子集）。
///
/// Java 常量为 `{"example_name", "is_show", "is_blank", "sort_order"}`，
/// 其中 `example_name` 在 `pc_navigation` 表**不存在**（Java 侧为复制粘贴笔误，
/// 命中时会在 SQL 层报未知列）；本实现直接拒绝该字段，语义见服务层注释。
/// 仓储层保留同一白名单做纵深防御（infra 不得反向依赖 services）。
pub const PC_NAVIGATION_FIELDS: [&str; 3] = ["is_show", "is_blank", "sort_order"];

/// 按属性名 / 列名解析排序字段（对应 Java `BaseServiceImpl#getColumnByPropertyName`）。
///
/// Java 语义：属性名或数据库列名都能命中；**非法/未知字段回落主键 `id`**
/// （这是 Java 的防注入兜底，本实现照抄，返回的始终是编译期常量）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field.map(str::trim).unwrap_or("") {
        "title" => "title",
        "isShow" | "is_show" => "is_show",
        "sortOrder" | "sort_order" => "sort_order",
        "isBlank" | "is_blank" => "is_blank",
        "link" => "link",
        "type" => "\"type\"",
        "parentId" | "parent_id" => "parent_id",
        "icon" => "icon",
        _ => "id",
    }
}

/// 排序方向（对应 Java `BaseServiceImpl#buildSortOrder`：仅显式 `asc` 升序，
/// 其余（含缺省）一律降序）。
#[must_use]
pub fn resolve_sort_dir(sort_order: Option<&str>) -> &'static str {
    if sort_order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
        "ASC"
    } else {
        "DESC"
    }
}

#[derive(Clone)]
pub struct PcNavigationRepository {
    pool: PgPool,
}

impl PcNavigationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— pc_navigation —————————————————————

    /// PC 导航分页（对应 Java `PcNavigationServiceImpl#list` 的查询构造器部分）。
    ///
    /// - `type_filter`：已按 `PcNavigationType.isValidCode` 过滤后的类型；
    /// - `parent_id_filter`：仅 `type == 2` 且 `parent_id >= 0` 时由服务层传入；
    /// - `keyword`：`title ILIKE %kw%`；
    /// - 排序：`buildSortOrder(page, sortField, sortOrder)` 单列排序。
    ///
    /// 三个可选条件用 `(? IS NULL OR col = ?)` 表达（与 `repo::decorate` 的
    /// `find_shops_for_picker` 同一手法），使 SQL 保持**静态单行**、
    /// 绑定位置固定，避免动态拼 `WHERE` 引入注入面。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate(
        &self,
        type_filter: Option<i32>,
        parent_id_filter: Option<i32>,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PcNavigation>)> {
        let like = keyword.map(|k| format!("%{k}%"));
        let column = resolve_sort_column(sort_field);
        let dir = resolve_sort_dir(sort_order);
        let (limit, offset) = page.limit_offset();

        let total: (i64,) = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM pc_navigation WHERE ($1 IS NULL OR \"type\" = $2) AND ($3 IS NULL OR parent_id = $4) AND ($5 IS NULL OR title ILIKE $6)",
        )
        .bind(type_filter)
        .bind(type_filter)
        .bind(parent_id_filter)
        .bind(parent_id_filter)
        .bind(like.as_deref())
        .bind(like.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows_sql = format!(
            "SELECT {NAV_COLUMNS} FROM pc_navigation WHERE ($1 IS NULL OR \"type\" = $2) AND ($3 IS NULL OR parent_id = $4) AND ($5 IS NULL OR title ILIKE $6) ORDER BY {column} {dir} LIMIT $7 OFFSET $8"
        );
        let rows = sqlx::query_as::<_, PcNavigation>(&rows_sql)
            .bind(type_filter)
            .bind(type_filter)
            .bind(parent_id_filter)
            .bind(parent_id_filter)
            .bind(like.as_deref())
            .bind(like.as_deref())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询（Java `BaseServiceImpl#getById`）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<PcNavigation>> {
        let sql = format!("SELECT {NAV_COLUMNS} FROM pc_navigation WHERE id = $1 LIMIT 1");
        sqlx::query_as::<_, PcNavigation>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 统计给定父级下的子导航数量（列表 `hasChildren` 用）。
    ///
    /// Java 是**每行一次** `lambdaQuery().eq(parentId, id).count()`（N+1）；
    /// 这里合并为一次 `GROUP BY parent_id`，结果等价。
    pub async fn count_children(&self, parent_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", parent_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT parent_id, COUNT(*) FROM pc_navigation WHERE parent_id IN ({placeholders}) GROUP BY parent_id"
        ));
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in parent_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 新增导航，返回自增主键（Java `BaseServiceImpl#save`）。
    ///
    /// `pc_navigation` 所有列均 `NOT NULL` 且带 DDL 默认值，因此 Java 的
    /// MyBatis-Plus NOT_NULL 策略（`null` 不入列）与「显式写入默认值」等价。
    pub async fn insert(&self, n: &PcNavigation) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO pc_navigation (title, is_show, sort_order, is_blank, link, \"type\", parent_id, icon) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
        )
        .bind(&n.title)
        .bind(n.is_show)
        .bind(n.sort_order)
        .bind(n.is_blank)
        .bind(&n.link)
        .bind(n.r#type)
        .bind(n.parent_id)
        .bind(&n.icon)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键局部更新（Java `BaseServiceImpl#updateById`）。
    ///
    /// MyBatis-Plus 的 NOT_NULL 策略让 `null` 字段不进入 `SET`；
    /// 这里用 `COALESCE(?, col)` 复刻该语义（绑定 `NULL` 即保留原值），
    /// 既避免「读-改-写」的并发覆盖，也保持 SQL 静态单行。
    /// `link` 恒写入（Java `getLink` 在未传 `link` 时写入 `"[]"`）。
    pub async fn update(&self, patch: &PcNavigationPatch) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE pc_navigation SET title = COALESCE($1, title), is_show = COALESCE($2, is_show), sort_order = COALESCE($3, sort_order), is_blank = COALESCE($4, is_blank), link = $5, \"type\" = COALESCE($6, \"type\"), parent_id = COALESCE($7, parent_id), icon = COALESCE($8, icon) WHERE id = $9",
        )
        .bind(patch.title.as_deref())
        .bind(patch.is_show)
        .bind(patch.sort_order)
        .bind(patch.is_blank)
        .bind(&patch.link)
        .bind(patch.r#type)
        .bind(patch.parent_id)
        .bind(patch.icon.as_deref())
        .bind(patch.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 删除单条（Java `BaseServiceImpl#del` / `removeById`）。
    pub async fn delete(&self, id: i32) -> Result<bool> {
        let r = sqlx::query("DELETE FROM pc_navigation WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 批量删除（Java `BaseServiceImpl#batch` → `removeByIds`）。
    pub async fn delete_many(&self, ids: &[i32]) -> Result<bool> {
        if ids.is_empty() {
            return Ok(false);
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = crate::db::to_pg_placeholders(&format!("DELETE FROM pc_navigation WHERE id IN ({placeholders})"));
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 通用单字段更新（Java `BaseServiceImpl#updateField`）。
    ///
    /// `column` **必须**来自 [`PC_NAVIGATION_FIELDS`]（`&'static str`，
    /// 编译期常量），因此 `format!` 拼接不存在注入面。
    pub async fn update_field(&self, id: i32, column: &'static str, val: &str) -> Result<bool> {
        if !PC_NAVIGATION_FIELDS.contains(&column) {
            return Err(Error::Validation(format!("不允许更新的字段: {column}")));
        }
        let sql = format!("UPDATE pc_navigation SET \"{column}\" = $1 WHERE id = $2");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// `getParentNav` 的全量查询。
    ///
    /// 对应 Java `PcNavigationServiceImpl#getParentNav(type, 0)`：
    /// `type > 0` 才按类型过滤、恒 `is_show = 1`、可选排除
    /// 「商家入驻」导航（`id = 42`），按 `parent_id ASC, sort_order ASC` 排序。
    pub async fn find_for_parent_tree(
        &self,
        type_filter: Option<i32>,
        exclude_id: Option<i32>,
    ) -> Result<Vec<PcNavigation>> {
        let sql = format!(
            "SELECT {NAV_COLUMNS} FROM pc_navigation WHERE is_show = 1 AND ($1 IS NULL OR \"type\" = $2) AND ($3 IS NULL OR id <> $4) ORDER BY parent_id ASC, sort_order ASC"
        );
        sqlx::query_as::<_, PcNavigation>(&sql)
            .bind(type_filter)
            .bind(type_filter)
            .bind(exclude_id)
            .bind(exclude_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("导航数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_columns_cover_all_nine_columns_in_entity_order() {
        assert_eq!(
            NAV_COLUMNS,
            "id, title, is_show, sort_order, is_blank, link, \"type\", parent_id, icon"
        );
        assert_eq!(NAV_COLUMNS.split(',').count(), 9);
    }

    #[test]
    fn sort_column_whitelist_and_primary_key_fallback() {
        assert_eq!(resolve_sort_column(None), "id");
        assert_eq!(resolve_sort_column(Some("")), "id");
        assert_eq!(resolve_sort_column(Some("sortOrder")), "sort_order");
        assert_eq!(resolve_sort_column(Some("sort_order")), "sort_order");
        assert_eq!(resolve_sort_column(Some("isShow")), "is_show");
        assert_eq!(resolve_sort_column(Some("parentId")), "parent_id");
        assert_eq!(resolve_sort_column(Some("type")), "\"type\"");
        // 未知字段回落主键（Java 防注入兜底），且绝不回显用户输入
        assert_eq!(
            resolve_sort_column(Some("1;DROP TABLE pc_navigation")),
            "id"
        );
        assert_eq!(resolve_sort_column(Some("title DESC, id")), "id");
    }

    #[test]
    fn sort_dir_defaults_to_desc_like_java() {
        assert_eq!(resolve_sort_dir(Some("asc")), "ASC");
        assert_eq!(resolve_sort_dir(Some("ASC")), "ASC");
        assert_eq!(resolve_sort_dir(Some("desc")), "DESC");
        assert_eq!(resolve_sort_dir(Some("")), "DESC");
        assert_eq!(resolve_sort_dir(None), "DESC");
    }

    #[test]
    fn update_field_whitelist_excludes_java_copy_paste_column() {
        // Java 白名单里的 `example_name` 在 `pc_navigation` 不存在；
        // 这里必须**不在**允许集合内，否则会写出未知列。
        assert!(!PC_NAVIGATION_FIELDS.contains(&"example_name"));
        assert!(PC_NAVIGATION_FIELDS.contains(&"is_show"));
        assert!(PC_NAVIGATION_FIELDS.contains(&"is_blank"));
        assert!(PC_NAVIGATION_FIELDS.contains(&"sort_order"));
        // `title` 在 Java 白名单内**不存在**，故不可通过 updateField 改
        assert!(!PC_NAVIGATION_FIELDS.contains(&"title"));
    }
}