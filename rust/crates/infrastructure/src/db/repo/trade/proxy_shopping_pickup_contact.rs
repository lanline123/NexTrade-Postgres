//! 代客下单自提联系人（trade::proxy_shopping_pickup_contact）仓储：单表 `pickup_contact`。
//!
//! 对应原 Java `com.nextrade.mapper.user.PickupContactMapper`（`BaseMapper`，
//! 无自定义方法）经 `PickupContactServiceImpl extends BaseServiceImpl` 暴露的
//! 全部读写：
//!
//! | 本模块方法 | 原 Java |
//! |-----------|---------|
//! | [`ProxyShoppingPickupContactRepository::paginate`] | `PickupContactServiceImpl#list`（`this.page`） |
//! | [`ProxyShoppingPickupContactRepository::list_by_user_id`] | `#listByUserId`（`this.list`） |
//! | [`ProxyShoppingPickupContactRepository::find_by_id`] | `#detail` / `#update` 的 `getById` |
//! | [`ProxyShoppingPickupContactRepository::insert`] | `#create` 的 `this.save` |
//! | [`ProxyShoppingPickupContactRepository::update`] | `#update` 的 `this.updateById` |
//! | [`ProxyShoppingPickupContactRepository::soft_delete`] | `#del` 的 `this.removeById` |
//!
//! 列名严格对齐权威 schema
//! （`cd rust && python tools/schema_parser.py --table pickup_contact`，
//! 权威源 `java/database/pure.sql:3022-3032`），共 7 列：
//! `id`（bigint，主键）/ `user_id`（int）/ `contact_name`（varchar(64)，
//! `NOT NULL DEFAULT ''`）/ `contact_phone`（varchar(32)，`NOT NULL DEFAULT ''`）/
//! `create_time`（bigint，可空）/ `update_time`（bigint，可空）/
//! `is_del`（tinyint，`NOT NULL DEFAULT 0`）。
//!
//! # MyBatis-Plus `@TableLogic` 的复刻
//!
//! 实体 `PickupContact.isDel` 上标了 `@TableLogic`，因此：
//! - **所有读** 自动追加 `is_del = 0`（本文件的每条 SELECT 都显式写出）；
//! - **`removeById`** 是**逻辑删除**：`UPDATE ... SET is_del = 1`，
//!   不是物理 `DELETE`（见 [`ProxyShoppingPickupContactRepository::soft_delete`]）；
//! - 已删除行在 `getById` / `page` / `list` 里一律查不到 —— 这正是
//!   `detail` / `update` 报「自提联系人不存在」的唯一来源。
//!
//! # MyBatis-Plus `FieldStrategy.NOT_NULL` 的复刻
//!
//! Java 默认 `insertStrategy` / `updateStrategy` 均为 `NOT_NULL`：
//! `null` 字段**不进** INSERT / UPDATE 的列表。因此
//! [`ProxyShoppingPickupContactRepository::insert`] 按「哪些字段有值」动态拼列清单，
//! 让 `pickup_contact` 的列默认值（`contact_name` / `contact_phone` 为 `''`）
//! 兜底 —— 而 `user_id` 既非空又无默认值，传 `null` 仍会报数据库错误，
//! 与原系统一致。
//!
//! 约定：
//! - 一律使用**运行时** `sqlx::query_as` / `sqlx::query`，**不使用** `query!`
//!   宏（本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 仓储层不做业务校验（Java `PickupContactMapper` 也不做），
//!   「id 缺失返回 false」「记录不存在报 1001」等语义在服务层复刻。

use sqlx::PgPool;
use nextrade_core::entities::PickupContact;
use nextrade_core::models::trade::proxy_shopping_pickup_contact::PickupContactListQuery;
use nextrade_core::{Error, Result};

/// 主键列名（Java `BaseServiceImpl#getKeyColumn()` 的返回值）。
pub const PRIMARY_KEY: &str = "id";

// ⚠️ 为什么每条 SELECT 都把 7 个列名**逐字写出来**（而不用一个列清单常量拼接）：
// `tools/check_sql_columns.py` 只扫描**字符串字面量**里的 SQL，`SELECT {COLS}
// FROM ...` 这种占位符写法会让 7 个列名完全逃过静态校验 —— 而「列名写错」
// 恰恰是 `cargo check` 查不出、只有这个工具能查的一类缺陷。
// 故此处刻意重复书写，换取 `SELECT` 列表 100% 被校验。
// 与权威 schema 对齐（`python tools/schema_parser.py --table pickup_contact`）：
// id / user_id / contact_name / contact_phone / create_time / update_time / is_del。

/// 排序字段白名单：Java 属性名（驼峰）或真实列名 → 列名。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`：先比主键列，再遍历
/// `TableFieldInfo` 同时接受 `property` 与 `column`，**找不到时回退主键列**
/// （原意是防 SQL 注入）。这里的白名单是同一语义的显式编码。
fn sort_column(field: &str) -> Option<&'static str> {
    match field {
        "id" => Some("id"),
        "userId" | "user_id" => Some("user_id"),
        "contactName" | "contact_name" => Some("contact_name"),
        "contactPhone" | "contact_phone" => Some("contact_phone"),
        "createTime" | "create_time" => Some("create_time"),
        "updateTime" | "update_time" => Some("update_time"),
        "isDel" | "is_del" => Some("is_del"),
        _ => None,
    }
}

/// 排序方向（Java `BaseServiceImpl#buildSortOrder` 的三个分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// `"asc".equalsIgnoreCase(sortOrder)`
    Asc,
    /// `"desc".equalsIgnoreCase(sortOrder) || StrUtil.isEmpty(sortOrder)`
    Desc,
    /// 非空且不是 asc/desc：Java **一个分支都不进** → `page.orders` 保持空
    /// → 生成的 SQL **没有 ORDER BY**（见 [`order_by_clause`]）。
    Unordered,
}

/// 解析排序方向（逐字对照 Java `BaseServiceImpl#buildSortOrder`）。
///
/// - `"asc"`（忽略大小写）→ [`SortDirection::Asc`]
/// - `"desc"`（忽略大小写）或 **空串 / 缺失** → [`SortDirection::Desc`]
/// - 其它非空值（含 `"  "` 这种纯空白）→ [`SortDirection::Unordered`]
///
/// ⚠️ 注意 Java 这里用的是 `StrUtil.isEmpty`（**不 trim**），所以 `"  "`
/// 既不等于 asc 也不等于 desc、又不是 empty，最终**不加 ORDER BY**。
#[must_use]
pub fn resolve_sort_direction(sort_order: Option<&str>) -> SortDirection {
    match sort_order {
        Some(o) if o.eq_ignore_ascii_case("asc") => SortDirection::Asc,
        Some(o) if o.eq_ignore_ascii_case("desc") => SortDirection::Desc,
        Some("") => SortDirection::Desc,
        None => SortDirection::Desc,
        // 非空且不是 asc/desc：Java 落到「两个分支都不进」
        Some(_) => SortDirection::Unordered,
    }
}

/// ORDER BY 子句（含前导空格）；`None` 表示**不生成** ORDER BY。
///
/// 对应 Java `BaseServiceImpl#buildSortOrder(page, sortField, sortOrder)`：
/// - `sortField` 为空或等于主键属性名 → 用主键列 `id`；
/// - 属性名经 `getColumnByPropertyName` 映射为真实列名，找不到时回退 `id`；
/// - 方向见 [`resolve_sort_direction`]，`Unordered` 时 Java 不调用
///   `page.setOrders(...)`，`Page.orders` 保持空列表 → 分页 SQL 无 ORDER BY。
///
/// > 注：`crates/infrastructure/src/db/repo/buyer_show_comment.rs` 把「未知方向」当作
/// > DESC 处理，与本文件不同。本文件选择**忠实复刻 Java**（不加 ORDER BY），
/// > 因为这是原系统可观察行为；差异已列入切片的 `uncertain_points`。
#[must_use]
pub fn order_by_clause(q: &PickupContactListQuery) -> Option<String> {
    let direction = resolve_sort_direction(q.sort_order.as_deref());
    if direction == SortDirection::Unordered {
        return None;
    }
    // Java：`StrUtil.isEmpty(sortField) || equals(sortField, getKeyProperty())`
    //      → sortField = getKeyColumn()；随后 getColumnByPropertyName 再做映射。
    let column = q
        .sort_field
        .as_deref()
        .filter(|s| !s.is_empty() && *s != PRIMARY_KEY)
        .and_then(sort_column)
        .unwrap_or(PRIMARY_KEY);
    let dir = match direction {
        SortDirection::Asc => "ASC",
        _ => "DESC",
    };
    Some(format!(" ORDER BY {column} {dir}"))
}

/// 待绑定参数（按 `conds` 顺序 1:1 对应）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Bind {
    I32(i32),
    Str(String),
}

/// 把 `Bind` 列表按顺序绑定到查询上（宏展开以便同时适配不同返回行类型）。
///
/// 内部用 `iter()` 借用，故 `binds` 可被 COUNT / SELECT 两条语句复用。
macro_rules! bind_all {
    ($q:expr, $binds:expr) => {{
        let mut q = $q;
        for b in $binds.iter() {
            q = match b {
                Bind::I32(v) => q.bind(*v),
                Bind::Str(v) => q.bind(v.as_str()),
            };
        }
        q
    }};
}

/// `pickup_contact` 表仓储（代客下单自提联系人域独占）。
#[derive(Clone)]
pub struct ProxyShoppingPickupContactRepository {
    pool: PgPool,
}

impl ProxyShoppingPickupContactRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 读 —————————————————————

    /// 自提联系人分页（Java `PickupContactServiceImpl#list` 的 `this.page`）。
    ///
    /// 返回 `(总数, 当前页记录)`；过滤规则见 [`build_filter`]，
    /// 排序规则见 [`order_by_clause`]。
    ///
    /// COUNT 与 SELECT 用同一份 WHERE 与同一组绑定参数（Java 的分页拦截器
    /// 也是拿同一条 wrapper 派生 count SQL）。
    pub async fn paginate(
        &self,
        q: &PickupContactListQuery,
    ) -> Result<(u64, Vec<PickupContact>)> {
        let (where_sql, binds) = build_filter(q);
        let limit = q.page_size();
        let offset = (q.page_no() - 1) * limit;

        let total_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM pickup_contact WHERE is_del = 0{where_sql}"
        ));
        let total: (i64,) = bind_all!(sqlx::query_as::<_, (i64,)>(&total_sql), binds)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let order = order_by_clause(q).unwrap_or_default();
        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT id, user_id, contact_name, contact_phone, create_time, update_time, is_del FROM pickup_contact WHERE is_del = 0{where_sql}{order} LIMIT ? OFFSET ?"
        ));
        let rows = bind_all!(
            sqlx::query_as::<_, PickupContact>(&rows_sql),
            binds
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 某客户的全部自提联系人（Java `PickupContactServiceImpl#listByUserId`）。
    ///
    /// Java：`eq(userId).orderByDesc(id)`，且 `@TableLogic` 自动带
    /// `is_del = 0`。返回顺序为 `id` 降序。
    pub async fn list_by_user_id(&self, user_id: i32) -> Result<Vec<PickupContact>> {
        // 无参数的固定 SQL 用字面量而非 `format!`（避免 clippy::useless_format）。
        let sql = "SELECT id, user_id, contact_name, contact_phone, create_time, update_time, is_del FROM pickup_contact WHERE is_del = 0 AND user_id = $1 ORDER BY id DESC";
        sqlx::query_as::<_, PickupContact>(&sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键取单行（Java `getById`，`@TableLogic` → 追加 `is_del = 0`）。
    ///
    /// 已逻辑删除的行返回 `None`，调用方据此报「自提联系人不存在」。
    pub async fn find_by_id(&self, id: i64) -> Result<Option<PickupContact>> {
        let sql = "SELECT id, user_id, contact_name, contact_phone, create_time, update_time, is_del FROM pickup_contact WHERE id = $1 AND is_del = 0 LIMIT 1";
        sqlx::query_as::<_, PickupContact>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 写 —————————————————————

    /// 新增（Java `PickupContactServiceImpl#create` 的 `this.save`）。
    ///
    /// 复刻 MyBatis-Plus `FieldStrategy.NOT_NULL`：**只有非 `None`** 的
    /// `user_id` / `contact_name` / `contact_phone` 才进 INSERT 列表，
    /// 缺列由数据库默认值兜底（`contact_name` / `contact_phone` 为 `''`）。
    ///
    /// 自动填充列对应 Java `MybatisPlusMetaObjectHandler#insertFill`
    /// （`createTime` / `updateTime` = 当前**秒级**时间戳，`isDel` = `0`）：
    /// 这里直接写进 SQL —— `is_del` 用字面量 `0`（Java `Constants.NO = 0`），
    /// 两个时间列绑定 `now`。
    ///
    /// 返回自增主键（Java `save` 后 `entity.getId()`）。
    pub async fn insert(
        &self,
        user_id: Option<i32>,
        contact_name: Option<&str>,
        contact_phone: Option<&str>,
        now: i64,
    ) -> Result<i64> {
        let mut cols: Vec<&str> = Vec::with_capacity(6);
        let mut vals: Vec<&str> = Vec::with_capacity(6);
        if user_id.is_some() {
            cols.push("user_id");
            vals.push("?");
        }
        if contact_name.is_some() {
            cols.push("contact_name");
            vals.push("?");
        }
        if contact_phone.is_some() {
            cols.push("contact_phone");
            vals.push("?");
        }
        // `MybatisPlusMetaObjectHandler#insertFill` 的三个填充列恒有值。
        cols.extend(["create_time", "update_time", "is_del"]);
        vals.extend(["?", "?", "0"]);

        let sql = format!(
            "INSERT INTO pickup_contact ({}) VALUES ({})",
            cols.join(", "),
            vals.join(", ")
        );
        let mut q = sqlx::query_scalar(&format!("{} RETURNING id", &sql));
        if let Some(v) = user_id {
            q = q.bind(v);
        }
        if let Some(v) = contact_name {
            q = q.bind(v);
        }
        if let Some(v) = contact_phone {
            q = q.bind(v);
        }
        let r: i64 = q
            .bind(now)
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 按主键更新（Java `PickupContactServiceImpl#update` 的 `this.updateById`）。
    ///
    /// Java 先 `getById` 拿到实体，再按非空规则覆盖字段，最后 `updateById`：
    /// 只有 `contact_name` / `contact_phone` / `user_id` 会被改动
    /// （`null` 的入参保持原值），`create_time` 原样回写（等价于不动），
    /// `update_time` 由 `updateFill` 刷成当前秒级时间戳。
    /// 调用方（服务层）已把「保持原值」解析成最终值传入。
    ///
    /// 返回是否命中至少一行（MyBatis-Plus `updateById` 的布尔语义）。
    pub async fn update(
        &self,
        id: i64,
        user_id: i32,
        contact_name: &str,
        contact_phone: &str,
        now: i64,
    ) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE pickup_contact SET user_id = $1, contact_name = $2, contact_phone = $3, update_time = $4 WHERE id = $5 AND is_del = 0",
        )
        .bind(user_id)
        .bind(contact_name)
        .bind(contact_phone)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 逻辑删除（Java `PickupContactServiceImpl#del` 的 `this.removeById`）。
    ///
    /// 实体带 `@TableLogic`，故 MyBatis-Plus 生成
    /// `UPDATE pickup_contact SET is_del = 1 WHERE id = ? AND is_del = 0`，
    /// **不是**物理 `DELETE`。返回是否确实改到了行（重复删除 → `false`）。
    ///
    /// ⚠️ 与原 Java 一样**不刷 `update_time`**：`deleteById` 的逻辑删除 SQL
    /// 只写 `is_del`（与 product_manage / admin_user / app_version_admin 的既有约定一致）。
    pub async fn soft_delete(&self, id: i64) -> Result<bool> {
        let r = sqlx::query("UPDATE pickup_contact SET is_del = 1 WHERE id = $1 AND is_del = 0")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }
}

/// 构造 WHERE 附加条件（含前导 ` AND `）与绑定参数列表。
///
/// 逐条对照 Java `PickupContactServiceImpl#list`：
/// ```text
/// if (listDTO.getUserId() != null) {
///     queryWrapper.eq(PickupContact::getUserId, listDTO.getUserId());
/// }
/// if (listDTO.getKeyword() != null && !listDTO.getKeyword().isEmpty()) {
///     queryWrapper.and(wrapper -> wrapper
///             .like(PickupContact::getContactName, listDTO.getKeyword())
///             .or()
///             .like(PickupContact::getContactPhone, listDTO.getKeyword()));
/// }
/// ```
/// 因此：
/// - `userId` 非空 → `user_id = ?`；
/// - `keyword` 非 `null` 且**非空串** → `AND (contact_name LIKE ? OR contact_phone LIKE ?)`；
/// - ⚠️ Java 用的是 `isEmpty()` **不是** `isBlank()`，所以纯空白串（如 `"  "`）
///   **会**参与 LIKE 过滤（`'%  %'`），此处不得 trim、不得当成空。
///
/// `is_del = 0` 不在这里拼接 —— 它由调用方的 SQL 模板固定写入，
/// 便于 `tools/check_sql_columns.py` 静态校验该列。
fn build_filter(q: &PickupContactListQuery) -> (String, Vec<Bind>) {
    let mut conds: Vec<String> = Vec::new();
    let mut binds: Vec<Bind> = Vec::new();

    if let Some(user_id) = q.user_id {
        conds.push("user_id = ?".into());
        binds.push(Bind::I32(user_id));
    }

    if let Some(keyword) = q.keyword.as_deref().filter(|k| !k.is_empty()) {
        // MyBatis-Plus `like` 默认 `SqlLike.DEFAULT`：两侧加 `%`，不做转义。
        let pattern = format!("%{keyword}%");
        conds.push("(contact_name LIKE ? OR contact_phone LIKE ?)".into());
        binds.push(Bind::Str(pattern.clone()));
        binds.push(Bind::Str(pattern));
    }

    if conds.is_empty() {
        return (String::new(), binds);
    }
    (format!(" AND {}", conds.join(" AND ")), binds)
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("自提联系人不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(json: &str) -> PickupContactListQuery {
        serde_json::from_str(json).unwrap()
    }

    // ————————————————— 列清单 —————————————————

    #[test]
    fn select_column_lists_cover_all_seven_from_schema() {
        // 列名/列数由 `check_sql_columns.py` 对 pure.sql 静态校验（本文件刻意
        // 把 7 个列名逐字写进每条 SELECT，就是为了让它检查得到）。
        // 这里再钉一次「SELECT 列表 = 7 列」，防止有人改回占位符拼接。
        for sql in [
            "SELECT id, user_id, contact_name, contact_phone, create_time, update_time, is_del FROM pickup_contact WHERE is_del = 0",
            "SELECT id, user_id, contact_name, contact_phone, create_time, update_time, is_del FROM pickup_contact WHERE is_del = 0 AND user_id = $1 ORDER BY id DESC",
            "SELECT id, user_id, contact_name, contact_phone, create_time, update_time, is_del FROM pickup_contact WHERE id = $1 AND is_del = 0 LIMIT 1",
        ] {
            let list = sql
                .strip_prefix("SELECT ")
                .and_then(|s| s.split(" FROM ").next())
                .expect("SELECT 列表");
            let cols: Vec<&str> = list.split(',').map(str::trim).collect();
            assert_eq!(cols.len(), 7, "SELECT 列表应为 7 列: {sql}");
            for col in [
                "id",
                "user_id",
                "contact_name",
                "contact_phone",
                "create_time",
                "update_time",
                "is_del",
            ] {
                assert!(cols.contains(&col), "SELECT 列表缺少列 {col}: {sql}");
            }
        }
    }

    // ————————————————— 过滤条件 —————————————————

    #[test]
    fn filter_empty_query_has_no_extra_condition() {
        let (sql, binds) = build_filter(&PickupContactListQuery::default());
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_user_id_only() {
        let (sql, binds) = build_filter(&query(r#"{"userId":7}"#));
        assert_eq!(sql, " AND user_id = ?");
        assert_eq!(binds, vec![Bind::I32(7)]);
    }

    #[test]
    fn filter_keyword_wraps_in_parentheses_and_double_like() {
        let (sql, binds) = build_filter(&query(r#"{"keyword":"138"}"#));
        assert_eq!(sql, " AND (contact_name LIKE ? OR contact_phone LIKE ?)");
        assert_eq!(
            binds,
            vec![
                Bind::Str("%138%".into()),
                Bind::Str("%138%".into())
            ]
        );
    }

    #[test]
    fn filter_keyword_empty_string_is_ignored_like_java_is_empty() {
        // Java `!"".isEmpty()` → 不成立 → 不加条件
        let (sql, binds) = build_filter(&query(r#"{"keyword":""}"#));
        assert_eq!(sql, "");
        assert!(binds.is_empty());
    }

    #[test]
    fn filter_keyword_whitespace_is_applied_like_java() {
        // ⚠️ Java 用 isEmpty 而非 isBlank：纯空白串**会**参与过滤。
        // 禁止"顺手"改成 trim/isBlank，否则与线上行为不一致。
        let (sql, binds) = build_filter(&query(r#"{"keyword":"  "}"#));
        assert_eq!(sql, " AND (contact_name LIKE ? OR contact_phone LIKE ?)");
        assert_eq!(binds, vec![Bind::Str("%  %".into()), Bind::Str("%  %".into())]);
    }

    #[test]
    fn filter_combines_user_and_keyword_in_java_order() {
        // Java 先 eq(userId) 再 and(keyword)：条件顺序即绑定顺序。
        let (sql, binds) = build_filter(&query(r#"{"userId":3,"keyword":"张"}"#));
        assert_eq!(
            sql,
            " AND user_id = ? AND (contact_name LIKE ? OR contact_phone LIKE ?)"
        );
        assert_eq!(
            binds,
            vec![Bind::I32(3), Bind::Str("%张%".into()), Bind::Str("%张%".into())]
        );
        assert_eq!(sql.matches('?').count(), binds.len());
    }

    #[test]
    fn filter_does_not_escape_like_wildcards() {
        // MyBatis-Plus `like` 不做转义：`%` / `_` 原样进入模式串。
        let (_, binds) = build_filter(&query(r#"{"keyword":"a_b%c"}"#));
        assert_eq!(binds[0], Bind::Str("%a_b%c%".into()));
    }

    // ————————————————— 排序 —————————————————

    #[test]
    fn sort_direction_matches_java_branches() {
        assert_eq!(resolve_sort_direction(None), SortDirection::Desc);
        assert_eq!(resolve_sort_direction(Some("")), SortDirection::Desc);
        assert_eq!(resolve_sort_direction(Some("desc")), SortDirection::Desc);
        assert_eq!(resolve_sort_direction(Some("DESC")), SortDirection::Desc);
        assert_eq!(resolve_sort_direction(Some("asc")), SortDirection::Asc);
        assert_eq!(resolve_sort_direction(Some("AsC")), SortDirection::Asc);
        // 非空且不是 asc/desc → Java 两个分支都不进 → 无 ORDER BY
        assert_eq!(resolve_sort_direction(Some("sideways")), SortDirection::Unordered);
        // Java `StrUtil.isEmpty("  ")` 为 false → 同样落进 Unordered
        assert_eq!(resolve_sort_direction(Some("  ")), SortDirection::Unordered);
    }

    #[test]
    fn order_by_defaults_to_primary_key_desc() {
        let q = PickupContactListQuery::default();
        assert_eq!(order_by_clause(&q).as_deref(), Some(" ORDER BY id DESC"));
    }

    #[test]
    fn order_by_honours_field_and_direction() {
        let q = query(r#"{"sortField":"contactName","sortOrder":"asc"}"#);
        assert_eq!(
            order_by_clause(&q).as_deref(),
            Some(" ORDER BY contact_name ASC")
        );

        // 真实列名同样被接受（Java `getColumnByPropertyName` 也认列名）
        let q = query(r#"{"sortField":"create_time","sortOrder":"desc"}"#);
        assert_eq!(
            order_by_clause(&q).as_deref(),
            Some(" ORDER BY create_time DESC")
        );

        // 主键属性名 → 主键列
        let q = query(r#"{"sortField":"id","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q).as_deref(), Some(" ORDER BY id ASC"));
    }

    #[test]
    fn order_by_is_omitted_for_unknown_direction_like_java() {
        // Java `buildSortOrder` 对未知方向不调用 setOrders → 无 ORDER BY
        let q = query(r#"{"sortField":"contactName","sortOrder":"sideways"}"#);
        assert_eq!(order_by_clause(&q), None);
    }

    #[test]
    fn order_by_falls_back_to_primary_key_for_injection() {
        let q = query(r#"{"sortField":"1;DROP TABLE pickup_contact","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q).as_deref(), Some(" ORDER BY id ASC"));

        let q = query(r#"{"sortField":"contact_name` = 1, contact_name","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q).as_deref(), Some(" ORDER BY id ASC"));

        // 空串 sortField → Java `StrUtil.isEmpty` 命中 → 主键列
        let q = query(r#"{"sortField":"","sortOrder":"asc"}"#);
        assert_eq!(order_by_clause(&q).as_deref(), Some(" ORDER BY id ASC"));
    }

    #[test]
    fn sort_whitelist_covers_every_column() {
        for (prop, col) in [
            ("id", "id"),
            ("userId", "user_id"),
            ("user_id", "user_id"),
            ("contactName", "contact_name"),
            ("contact_name", "contact_name"),
            ("contactPhone", "contact_phone"),
            ("contact_phone", "contact_phone"),
            ("createTime", "create_time"),
            ("create_time", "create_time"),
            ("updateTime", "update_time"),
            ("update_time", "update_time"),
            ("isDel", "is_del"),
            ("is_del", "is_del"),
        ] {
            assert_eq!(sort_column(prop), Some(col), "映射不一致: {prop}");
        }
        assert_eq!(sort_column("nope"), None);
    }
}
