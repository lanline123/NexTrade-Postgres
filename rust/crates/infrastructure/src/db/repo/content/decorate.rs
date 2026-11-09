//! 页面装修域仓储：`decorate` / `decorate_discrete` / `decorate_share` /
//! `pc_navigation` / `mobile_cat_nav` / `pc_cat_floor`，另含 `locales` 语言查询。
//! 对应 Java `DecorateMapper`、`PcNavigationMapper`、`PcCatFloorMapper`、
//! `MobileCatNavMapper`、`DecorateShareMapper`、`DecorateDiscreteMapper`
//! 与各 ServiceImpl 中的条件构造。
//!
//! 全部使用函数式 `sqlx::query_as`（不使用 `query_as!` 宏，列名不参与编译期校验，
//! 与实体 `#[sqlx(rename_all = "snake_case")]` 对齐）。

use sqlx::PgPool;
use nextrade_core::entities::{
    Decorate, DecorateDiscrete, DecorateShare, MobileCatNav, PcCatFloor, PcNavigation,
};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// 装修页列表过滤条件（由服务层从 `DecorateListQuery` 组装）。
#[derive(Debug, Clone, Default)]
pub struct DecorateFilter {
    /// 店铺：`None` 不过滤（JWT 未携带 shop_id，单店铺场景下不过滤等价于全量）
    pub shop_id: Option<i32>,
    /// 标题模糊关键词
    pub keyword: Option<String>,
    /// 空表示不过滤
    pub decorate_types: Vec<i32>,
    /// 空表示不过滤
    pub page_types: Vec<i32>,
    pub parent_id: Option<i32>,
    pub status: Option<i32>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
}

/// 装修页可排序列白名单（`sortField` 非法时回退 `decorate_id`）。
const DECORATE_SORTABLE: &[&str] = &[
    "decorate_id",
    "decorate_title",
    "update_time",
    "sort_order",
    "page_type",
];

/// `updateField` 允许更新的列（对应 Java `CheckFieldConstants.DECORATE_FIELDS`）。
///
/// 注：Java 白名单里的 `is_show` 在 `decorate` 表并无该列（Java 侧会退化为
/// 更新主键而静默无效），这里改为真实存在的 `is_home` / `status`。
const DECORATE_FIELDS: &[&str] = &["decorate_title", "sort_order", "is_home", "status"];

/// PC 导航可更新列（对应 Java `PC_NAVIGATION_FIELDS`，
/// 其中 `example_name` 并非表列，这里换成真实的 `title`）。
const PC_NAVIGATION_FIELDS: &[&str] = &["title", "is_show", "is_blank", "sort_order"];

/// 动态 SQL 的绑定值（按序绑定，规避 `query_as!` 宏与动态 IN 列表问题）。
#[derive(Clone)]
enum Bind {
    I(i32),
    I64(i64),
    S(String),
}

/// 追加一个 `AND` 条件与对应绑定值。
fn push_cond(sql: &mut String, params: &mut Vec<Bind>, clause: &str, binds: &[Bind]) {
    sql.push_str(" AND ");
    sql.push_str(clause);
    params.extend_from_slice(binds);
}

/// 追加 `IN (...)` 条件（占位符数量 = 列表长度，值为 `i32` 无注入风险）。
fn push_in(sql: &mut String, params: &mut Vec<Bind>, column: &str, values: &[i32]) {
    if values.is_empty() {
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
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let pg_sql = crate::db::to_pg_placeholders(sql);
    let mut q = sqlx::query_as::<_, T>(&pg_sql);
    for p in params {
        q = match p {
            Bind::I(v) => q.bind(*v),
            Bind::I64(v) => q.bind(*v),
            Bind::S(v) => q.bind(v.clone()),
        };
    }
    q.fetch_all(pool).await.map_err(map_err)
}

/// 取回单行（可选）。
async fn fetch_one<T>(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<Option<T>>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let pg_sql = crate::db::to_pg_placeholders(sql);
    let mut q = sqlx::query_as::<_, T>(&pg_sql);
    for p in params {
        q = match p {
            Bind::I(v) => q.bind(*v),
            Bind::I64(v) => q.bind(*v),
            Bind::S(v) => q.bind(v.clone()),
        };
    }
    q.fetch_optional(pool).await.map_err(map_err)
}

/// 统计（复用 `fetch_rows`，取 `(i64,)` 首行）。
async fn count(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<u64> {
    let rows = fetch_rows::<(i64,)>(pool, sql, params).await?;
    Ok(rows.first().map(|r| r.0).unwrap_or(0).max(0) as u64)
}

/// 驼峰 → 下划线（`updateTime` → `update_time`）。
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in s.char_indices() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// 排序列白名单：非法或缺失回退 `decorate_id`。
fn resolve_sort_column(field: Option<&str>) -> &'static str {
    let snake = field.map(to_snake).unwrap_or_default();
    DECORATE_SORTABLE
        .iter()
        .find(|c| **c == snake)
        .copied()
        .unwrap_or("decorate_id")
}

#[derive(Clone)]
pub struct DecorateRepository {
    pool: PgPool,
}

impl DecorateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ————————————————————— decorate 装修页 —————————————————————

    /// 装修页分页列表。
    ///
    /// 对应 Java `DecorateServiceImpl#list(DecorateListDTO)`：
    /// 店铺 + 标题模糊 + 类型/页面用途多值 IN + 父级 + 状态过滤，
    /// 首页（`is_home`）置顶，随后按排序值/更新时间倒序。
    pub async fn paginate(
        &self,
        f: &DecorateFilter,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Decorate>)> {
        let offset = (page.page.max(1) - 1) * page.page_size;
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();

        if let Some(shop_id) = f.shop_id {
            // 平台素材规则：店铺账号可见 `shop_id = 0`（平台装修页）+ 本店
            push_cond(
                &mut wh,
                &mut params,
                "(shop_id = 0 OR shop_id = ?)",
                &[Bind::I(shop_id)],
            );
        }
        if let Some(kw) = f.keyword.as_ref().filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "decorate_title LIKE ?",
                &[Bind::S(format!("%{kw}%"))],
            );
        }
        push_in(&mut wh, &mut params, "decorate_type", &f.decorate_types);
        push_in(&mut wh, &mut params, "page_type", &f.page_types);
        if let Some(parent_id) = f.parent_id {
            push_cond(
                &mut wh,
                &mut params,
                "parent_id = ?",
                &[Bind::I(parent_id)],
            );
        }
        if let Some(status) = f.status {
            push_cond(&mut wh, &mut params, "status = ?", &[Bind::I(status)]);
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM decorate{wh}"),
            &params,
        )
        .await?;

        let column = resolve_sort_column(f.sort_field.as_deref());
        let dir = if f
            .sort_order
            .as_deref()
            .is_some_and(|o| o.eq_ignore_ascii_case("asc"))
        {
            "ASC"
        } else {
            "DESC"
        };
        // 首页置顶，其次按所选列排序（默认更新时间倒序）。
        let order = if column == "decorate_id" {
            "is_home DESC, decorate_id DESC".to_string()
        } else {
            format!("is_home DESC, {column} {dir}, decorate_id DESC")
        };
        let mut row_params = params.clone();
        row_params.push(Bind::I64(page.page_size as i64));
        row_params.push(Bind::I64(offset as i64));
        let rows = fetch_rows::<Decorate>(
            &self.pool,
            &format!("SELECT * FROM decorate{wh} ORDER BY {order} LIMIT ? OFFSET ?"),
            &row_params,
        )
        .await?;

        Ok((total, rows))
    }

    /// 按 ID 查装修页。
    pub async fn find_by_id(&self, decorate_id: i32) -> Result<Option<Decorate>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM decorate WHERE decorate_id = $1",
            &[Bind::I(decorate_id)],
        )
        .await
    }

    /// 按父级 + 语言 + 页面类型定位编辑中的页面（多语言详情）。
    ///
    /// 对应 `DecorateServiceImpl#detail` 中 `id` 为空时的回退查询。
    pub async fn find_by_parent_locale(
        &self,
        parent_id: i32,
        locale_id: Option<i32>,
        decorate_type: Option<i32>,
    ) -> Result<Option<Decorate>> {
        let mut wh = String::from(" WHERE parent_id = ?");
        let mut params = vec![Bind::I(parent_id)];
        if let Some(locale_id) = locale_id {
            push_cond(&mut wh, &mut params, "locale_id = ?", &[Bind::I(locale_id)]);
        }
        if let Some(t) = decorate_type {
            push_cond(&mut wh, &mut params, "decorate_type = ?", &[Bind::I(t)]);
        }
        wh.push_str(" ORDER BY locale_id ASC, decorate_id DESC LIMIT 1");
        fetch_one(&self.pool, &format!("SELECT * FROM decorate{wh}"), &params).await
    }

    /// 查询父级下的全部子页面（详情返回 `children`）。
    pub async fn find_children(&self, parent_id: i32) -> Result<Vec<Decorate>> {
        fetch_rows(
            &self.pool,
            "SELECT * FROM decorate WHERE parent_id = $1 ORDER BY locale_id ASC, decorate_id ASC",
            &[Bind::I(parent_id)],
        )
        .await
    }

    /// 统计父级子页面数量（删除前校验用）。
    pub async fn count_children(&self, parent_id: i32) -> Result<u64> {
        count(
            &self.pool,
            "SELECT COUNT(*) FROM decorate WHERE parent_id = $1",
            &[Bind::I(parent_id)],
        )
        .await
    }

    /// 查同类型/页面用途下的首页（`is_home = 1`），前台装修数据读取用。
    ///
    /// 对应 `DecorateServiceImpl#getDecorateModule` / `#customDecorate`：
    /// `shop_id = 0`、`locale_id = 0`、`is_home = 1`、`status = 1`、`page_type` 精确匹配。
    /// `page_type` 为 `None` 时不参与过滤（自定义页不按页面用途过滤）。
    pub async fn find_home_page(
        &self,
        decorate_type: Option<i32>,
        page_type: Option<i32>,
        shop_id: Option<i32>,
        status: Option<i32>,
        locale_id: Option<i32>,
    ) -> Result<Option<Decorate>> {
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(t) = decorate_type {
            push_cond(&mut wh, &mut params, "decorate_type = ?", &[Bind::I(t)]);
        }
        if let Some(pt) = page_type {
            push_cond(&mut wh, &mut params, "page_type = ?", &[Bind::I(pt)]);
        }
        if let Some(shop_id) = shop_id {
            push_cond(&mut wh, &mut params, "shop_id = ?", &[Bind::I(shop_id)]);
        }
        if let Some(status) = status {
            push_cond(&mut wh, &mut params, "status = ?", &[Bind::I(status)]);
        }
        if let Some(locale_id) = locale_id {
            if locale_id == 0 {
                // 历史/新建页面 locale_id 可能为 NULL，语义等同默认语言 0
                wh.push_str(" AND (locale_id = ? OR locale_id IS NULL)");
                params.push(Bind::I(locale_id));
            } else {
                push_cond(&mut wh, &mut params, "locale_id = ?", &[Bind::I(locale_id)]);
            }
        }
        wh.push_str(" ORDER BY is_home DESC, update_time DESC, decorate_id DESC LIMIT 1");
        fetch_one(&self.pool, &format!("SELECT * FROM decorate{wh}"), &params).await
    }

    /// 按 ID 直接查已发布页面数据（预览/详情读取 `data` 列）。
    pub async fn find_published_by_id(&self, decorate_id: i32) -> Result<Option<Decorate>> {
        self.find_by_id(decorate_id).await
    }

    /// 取消同组其它页面的首页标记（`setHome` 时调用）。
    ///
    /// 对应 `DecorateServiceImpl#setHome`：先把同类型同页面用途的
    /// `is_home` 全部归零，再把目标页置 1。
    pub async fn clear_home_flags(
        &self,
        decorate_type: Option<i32>,
        page_type: Option<i32>,
        shop_id: Option<i32>,
        except_id: i32,
    ) -> Result<u64> {
        let mut wh = String::from(" WHERE is_home <> 0 AND decorate_id <> ?");
        let mut params = vec![Bind::I(except_id)];
        if let Some(t) = decorate_type {
            push_cond(&mut wh, &mut params, "decorate_type = ?", &[Bind::I(t)]);
        }
        if let Some(pt) = page_type {
            push_cond(&mut wh, &mut params, "page_type = ?", &[Bind::I(pt)]);
        }
        if let Some(shop_id) = shop_id {
            push_cond(&mut wh, &mut params, "shop_id = ?", &[Bind::I(shop_id)]);
        }
        let sql = format!("UPDATE decorate SET is_home = 0{wh}");
        exec(&self.pool, &sql, &params).await
    }

    /// 模糊取同类型页面标题（`copy` 生成 "xxx-副本" 时做去重后缀）。
    ///
    /// 对应 `DecorateServiceImpl#copy` 中遍历既有标题找空闲副本名。
    pub async fn find_titles_by_prefix(
        &self,
        shop_id: Option<i32>,
        decorate_type: Option<i32>,
        prefix: &str,
    ) -> Result<Vec<String>> {
        let mut wh = String::from(" WHERE decorate_title LIKE ?");
        let mut params = vec![Bind::S(format!("{prefix}%"))];
        if let Some(shop_id) = shop_id {
            push_cond(&mut wh, &mut params, "shop_id = ?", &[Bind::I(shop_id)]);
        }
        if let Some(t) = decorate_type {
            push_cond(&mut wh, &mut params, "decorate_type = ?", &[Bind::I(t)]);
        }
        let rows = fetch_rows::<(Option<String>,)>(
            &self.pool,
            &format!("SELECT decorate_title FROM decorate{wh}"),
            &params,
        )
        .await?;
        Ok(rows.into_iter().filter_map(|r| r.0).collect())
    }

    /// 新增装修页，返回自增主键。
    pub async fn insert(&self, d: &Decorate) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO decorate (decorate_title, data, draft_data, decorate_type, page_type, \
             is_home, shop_id, status, update_time, locale_id, parent_id, sort_order, \
             binded_h5_id, share_info) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?) RETURNING decorate_id",
        )
        .bind(&d.decorate_title)
        .bind(&d.data)
        .bind(&d.draft_data)
        .bind(d.decorate_type)
        .bind(d.page_type)
        .bind(d.is_home)
        .bind(d.shop_id)
        .bind(d.status)
        .bind(d.update_time)
        .bind(d.locale_id)
        .bind(d.parent_id)
        .bind(d.sort_order)
        .bind(d.binded_h5_id)
        .bind(&d.share_info)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 全量更新装修页（除主键外的所有列）。
    pub async fn update(&self, d: &Decorate) -> Result<()> {
        sqlx::query(
            "UPDATE decorate SET decorate_title=?, data=?, draft_data=?, decorate_type=?, \
             page_type=?, is_home=?, shop_id=?, status=?, update_time=?, locale_id=?, \
             parent_id=?, sort_order=?, binded_h5_id=?, share_info=? \
             WHERE decorate_id=?",
        )
        .bind(&d.decorate_title)
        .bind(&d.data)
        .bind(&d.draft_data)
        .bind(d.decorate_type)
        .bind(d.page_type)
        .bind(d.is_home)
        .bind(d.shop_id)
        .bind(d.status)
        .bind(d.update_time)
        .bind(d.locale_id)
        .bind(d.parent_id)
        .bind(d.sort_order)
        .bind(d.binded_h5_id)
        .bind(&d.share_info)
        .bind(d.decorate_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除装修页。
    pub async fn delete(&self, decorate_id: i32) -> Result<bool> {
        Ok(
            exec(
                &self.pool,
                "DELETE FROM decorate WHERE decorate_id = ?",
                &[Bind::I(decorate_id)],
            )
            .await?
                > 0,
        )
    }

    /// 批量删除装修页。
    pub async fn delete_batch(&self, ids: &[i32]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("DELETE FROM decorate WHERE decorate_id IN ({placeholders})");
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 通用字段更新：只更新白名单内的列（对应 `BaseServiceImpl#updateField`）。
    pub async fn update_field(&self, decorate_id: i32, field: &str, val: &str) -> Result<bool> {
        let column = to_snake(field);
        if !DECORATE_FIELDS.contains(&column.as_str()) {
            return Err(Error::Validation(format!(
                "不允许更新的字段: {field}"
            )));
        }
        // 列名来自常量白名单，非用户输入拼接。
        let sql = format!("UPDATE decorate SET \"{column}\" = ? WHERE decorate_id = ?");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(decorate_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 通用字段更新：`sql` 必须由服务层白名单常量拼出（列名不可来自用户输入），
    /// 形如 `UPDATE xxx SET \`col\` = ? WHERE pk = ?`，`val`/`id` 为绑定参数。
    pub async fn update_field_raw(&self, sql: &str, val: &str, id: i32) -> Result<bool> {
        let r = sqlx::query(sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    // ————————————————————— pc_navigation PC 导航 —————————————————————

    /// PC 导航分页。
    ///
    /// 对应 `PcNavigationServiceImpl#list`：`type` 过滤（1导航条 2顶部站点地图
    /// 3底部快捷 4侧边），`type = 2` 时按 `parent_id >= 0` 细分，`title` 模糊。
    pub async fn paginate_navigations(
        &self,
        nav_type: Option<i32>,
        parent_id: Option<i32>,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PcNavigation>)> {
        let offset = (page.page.max(1) - 1) * page.page_size;
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();

        if let Some(t) = nav_type {
            // `type` 为 SQL 关键字，必须加反引号。
            push_cond(&mut wh, &mut params, "\"type\" = ?", &[Bind::I(t)]);
        }
        // 仅 `type = 2`（顶部站点地图）才按父级细分。
        if nav_type == Some(2) {
            if let Some(p) = parent_id.filter(|p| *p >= 0) {
                push_cond(&mut wh, &mut params, "parent_id = ?", &[Bind::I(p)]);
            }
        }
        if let Some(kw) = keyword.filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                // `pc_navigation` 的标题列是 `title`（**不是** `decorate_title`，
                // 那是 `decorate` 表的列）。写错会在运行时报 Unknown column。
                "title LIKE ?",
                &[Bind::S(format!("%{kw}%"))],
            );
        }

        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM pc_navigation{wh}"),
            &params,
        )
        .await?;
        let mut row_params = params.clone();
        row_params.push(Bind::I64(page.page_size as i64));
        row_params.push(Bind::I64(offset as i64));
        let rows = fetch_rows::<PcNavigation>(
            &self.pool,
            &format!(
                "SELECT * FROM pc_navigation{wh} ORDER BY sort_order ASC, id ASC \
                 LIMIT ? OFFSET ?"
            ),
            &row_params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查 PC 导航。
    pub async fn find_navigation(&self, id: i32) -> Result<Option<PcNavigation>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM pc_navigation WHERE id = ?",
            &[Bind::I(id)],
        )
        .await
    }

    /// 统计父导航下的子导航数量（列表 `hasChildren` 用）。
    pub async fn count_navigation_children(&self, parent_ids: &[i32]) -> Result<Vec<(i32, i64)>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", parent_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT parent_id, COUNT(*) FROM pc_navigation \
             WHERE parent_id IN ({placeholders}) GROUP BY parent_id"
        );
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in parent_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 新增 PC 导航，返回自增主键。
    pub async fn insert_navigation(&self, n: &PcNavigation) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO pc_navigation (title, is_show, sort_order, is_blank, link, \"type\", \
             parent_id, icon) VALUES (?,?,?,?,?,?,?,?) RETURNING id",
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

    /// 全量更新 PC 导航。
    pub async fn update_navigation(&self, n: &PcNavigation) -> Result<()> {
        sqlx::query(
            "UPDATE pc_navigation SET title=?, is_show=?, sort_order=?, is_blank=?, link=?, \
             \"type\"=?, parent_id=?, icon=? WHERE id=?",
        )
        .bind(&n.title)
        .bind(n.is_show)
        .bind(n.sort_order)
        .bind(n.is_blank)
        .bind(&n.link)
        .bind(n.r#type)
        .bind(n.parent_id)
        .bind(&n.icon)
        .bind(n.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除 PC 导航（对应 `BaseServiceImpl#del`）。
    pub async fn delete_navigation(&self, id: i32) -> Result<bool> {
        Ok(
            exec(
                &self.pool,
                "DELETE FROM pc_navigation WHERE id = ?",
                &[Bind::I(id)],
            )
            .await?
                > 0,
        )
    }

    /// PC 导航通用字段更新（白名单见 \"PC_NAVIGATION_FIELDS\"）。
    pub async fn update_navigation_field(
        &self,
        id: i32,
        field: &str,
        val: &str,
    ) -> Result<bool> {
        let column = to_snake(field);
        if !PC_NAVIGATION_FIELDS.contains(&column.as_str()) {
            return Err(Error::Validation(format!(
                "不允许更新的字段: {field}"
            )));
        }
        let sql = format!("UPDATE pc_navigation SET \"{column}\" = ? WHERE id = ?");
        let r = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    // ————————————————————— pc_cat_floor PC 分类楼层 —————————————————————

    /// PC 分类楼层分页（`is_show` + 名称模糊）。
    ///
    /// 对应 `PcCatFloorServiceImpl#list`。
    pub async fn paginate_cat_floors(
        &self,
        is_show: Option<i32>,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<PcCatFloor>)> {
        let offset = (page.page.max(1) - 1) * page.page_size;
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(v) = is_show {
            push_cond(&mut wh, &mut params, "is_show = ?", &[Bind::I(v)]);
        }
        if let Some(kw) = keyword.filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "cat_floor_name LIKE ?",
                &[Bind::S(format!("%{kw}%"))],
            );
        }
        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM pc_cat_floor{wh}"),
            &params,
        )
        .await?;
        let mut row_params = params.clone();
        row_params.push(Bind::I64(page.page_size as i64));
        row_params.push(Bind::I64(offset as i64));
        let rows = fetch_rows::<PcCatFloor>(
            &self.pool,
            &format!(
                "SELECT * FROM pc_cat_floor{wh} ORDER BY sort_order ASC, cat_floor_id DESC \
                 LIMIT ? OFFSET ?"
            ),
            &row_params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查 PC 分类楼层。
    pub async fn find_cat_floor(&self, id: i32) -> Result<Option<PcCatFloor>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM pc_cat_floor WHERE cat_floor_id = ?",
            &[Bind::I(id)],
        )
        .await
    }

    /// 新增 PC 分类楼层，返回自增主键。
    pub async fn insert_cat_floor(&self, f: &PcCatFloor) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO pc_cat_floor (cat_floor_name, category_ids, category_names, floor_ico, \
             hot_cat, is_show, sort_order, floor_ico_font, brand_ids) \
             VALUES (?,?,?,?,?,?,?,?,?) RETURNING cat_floor_id",
        )
        .bind(&f.cat_floor_name)
        .bind(&f.category_ids)
        .bind(&f.category_names)
        .bind(&f.floor_ico)
        .bind(&f.hot_cat)
        .bind(f.is_show)
        .bind(f.sort_order)
        .bind(&f.floor_ico_font)
        .bind(&f.brand_ids)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 全量更新 PC 分类楼层。
    pub async fn update_cat_floor(&self, f: &PcCatFloor) -> Result<()> {
        sqlx::query(
            "UPDATE pc_cat_floor SET cat_floor_name=?, category_ids=?, category_names=?, \
             floor_ico=?, hot_cat=?, is_show=?, sort_order=?, floor_ico_font=?, brand_ids=? \
             WHERE cat_floor_id=?",
        )
        .bind(&f.cat_floor_name)
        .bind(&f.category_ids)
        .bind(&f.category_names)
        .bind(&f.floor_ico)
        .bind(&f.hot_cat)
        .bind(f.is_show)
        .bind(f.sort_order)
        .bind(&f.floor_ico_font)
        .bind(&f.brand_ids)
        .bind(f.cat_floor_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除 PC 分类楼层。
    pub async fn delete_cat_floor(&self, id: i32) -> Result<bool> {
        Ok(
            exec(
                &self.pool,
                "DELETE FROM pc_cat_floor WHERE cat_floor_id = ?",
                &[Bind::I(id)],
            )
            .await?
                > 0,
        )
    }

    // ————————————————————— mobile_cat_nav 手机分类导航 —————————————————————

    /// 手机分类导航分页（\"is_show\" + 自定义分类名模糊）。
    ///
    /// 对应 `MobileCatNavServiceImpl#list`。
    pub async fn paginate_mobile_navs(
        &self,
        is_show: Option<i32>,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<MobileCatNav>)> {
        let offset = (page.page.max(1) - 1) * page.page_size;
        let mut wh = String::from(" WHERE 1=1");
        let mut params: Vec<Bind> = Vec::new();
        if let Some(v) = is_show {
            push_cond(&mut wh, &mut params, "is_show = ?", &[Bind::I(v)]);
        }
        if let Some(kw) = keyword.filter(|k| !k.trim().is_empty()) {
            push_cond(
                &mut wh,
                &mut params,
                "cat_name_alias LIKE ?",
                &[Bind::S(format!("%{kw}%"))],
            );
        }
        let total = count(
            &self.pool,
            &format!("SELECT COUNT(*) FROM mobile_cat_nav{wh}"),
            &params,
        )
        .await?;
        let mut row_params = params.clone();
        row_params.push(Bind::I64(page.page_size as i64));
        row_params.push(Bind::I64(offset as i64));
        let rows = fetch_rows::<MobileCatNav>(
            &self.pool,
            &format!(
                "SELECT * FROM mobile_cat_nav{wh} ORDER BY sort_order ASC, mobile_cat_nav_id DESC \
                 LIMIT ? OFFSET ?"
            ),
            &row_params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查手机分类导航。
    pub async fn find_mobile_nav(&self, id: i32) -> Result<Option<MobileCatNav>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM mobile_cat_nav WHERE mobile_cat_nav_id = ?",
            &[Bind::I(id)],
        )
        .await
    }

    /// 新增手机分类导航，返回自增主键。
    pub async fn insert_mobile_nav(&self, n: &MobileCatNav) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO mobile_cat_nav (category_id, cat_color, child_cat_ids, brand_ids, \
             is_show, img_url, sort_order, cat_name_alias) VALUES (?,?,?,?,?,?,?,?) RETURNING mobile_cat_nav_id",
        )
        .bind(n.category_id)
        .bind(&n.cat_color)
        .bind(&n.child_cat_ids)
        .bind(&n.brand_ids)
        .bind(n.is_show)
        .bind(&n.img_url)
        .bind(n.sort_order)
        .bind(&n.cat_name_alias)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 全量更新手机分类导航。
    pub async fn update_mobile_nav(&self, n: &MobileCatNav) -> Result<()> {
        sqlx::query(
            "UPDATE mobile_cat_nav SET category_id=?, cat_color=?, child_cat_ids=?, brand_ids=?, \
             is_show=?, img_url=?, sort_order=?, cat_name_alias=? WHERE mobile_cat_nav_id=?",
        )
        .bind(n.category_id)
        .bind(&n.cat_color)
        .bind(&n.child_cat_ids)
        .bind(&n.brand_ids)
        .bind(n.is_show)
        .bind(&n.img_url)
        .bind(n.sort_order)
        .bind(&n.cat_name_alias)
        .bind(n.mobile_cat_nav_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除手机分类导航。
    pub async fn delete_mobile_nav(&self, id: i32) -> Result<bool> {
        Ok(
            exec(
                &self.pool,
                "DELETE FROM mobile_cat_nav WHERE mobile_cat_nav_id = ?",
                &[Bind::I(id)],
            )
            .await?
                > 0,
        )
    }

    // ————————————————————— decorate_share 装修分享 —————————————————————

    /// 装修分享分页（对应 `DecorateShareServiceImpl#list`，无过滤条件）。
    pub async fn paginate_shares(&self, page: &PageQuery) -> Result<(u64, Vec<DecorateShare>)> {
        let offset = (page.page.max(1) - 1) * page.page_size;
        let total = count(&self.pool, "SELECT COUNT(*) FROM decorate_share", &[]).await?;
        let params = [Bind::I64(page.page_size as i64), Bind::I64(offset as i64)];
        let rows = fetch_rows::<DecorateShare>(
            &self.pool,
            "SELECT * FROM decorate_share ORDER BY share_id DESC LIMIT ? OFFSET ?",
            &params,
        )
        .await?;
        Ok((total, rows))
    }

    /// 按 ID 查分享记录。
    pub async fn find_share(&self, share_id: i32) -> Result<Option<DecorateShare>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM decorate_share WHERE share_id = ?",
            &[Bind::I(share_id)],
        )
        .await
    }

    /// 按 sn 查分享记录（导入校验第一步）。
    pub async fn find_share_by_sn(&self, sn: &str) -> Result<Option<DecorateShare>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM decorate_share WHERE share_sn = ? LIMIT 1",
            &[Bind::S(sn.to_string())],
        )
        .await
    }

    /// 按 sn + token + 有效期内查分享记录（对应 `DecorateShareServiceImpl#get`）。
    pub async fn find_valid_share(
        &self,
        sn: &str,
        token: &str,
        now: i32,
    ) -> Result<Option<DecorateShare>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM decorate_share WHERE share_sn = ? AND share_token = ? \
             AND valid_time > ? LIMIT 1",
            &[Bind::S(sn.to_string()), Bind::S(token.to_string()), Bind::I(now)],
        )
        .await
    }

    /// 新增分享记录，返回自增主键。
    pub async fn insert_share(&self, s: &DecorateShare) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO decorate_share (share_sn, share_token, decorate_id, valid_time, \
             create_time, update_time) VALUES (?,?,?,?,?,?) RETURNING share_id",
        )
        .bind(&s.share_sn)
        .bind(&s.share_token)
        .bind(s.decorate_id)
        .bind(s.valid_time)
        .bind(s.create_time)
        .bind(s.update_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 全量更新分享记录（主键不变）。
    pub async fn update_share(&self, s: &DecorateShare) -> Result<()> {
        sqlx::query(
            "UPDATE decorate_share SET share_sn=?, share_token=?, decorate_id=?, valid_time=?, \
             create_time=?, update_time=? WHERE share_id=?",
        )
        .bind(&s.share_sn)
        .bind(&s.share_token)
        .bind(s.decorate_id)
        .bind(s.valid_time)
        .bind(s.create_time)
        .bind(s.update_time)
        .bind(s.share_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除分享记录。
    pub async fn delete_share(&self, share_id: i32) -> Result<bool> {
        Ok(
            exec(
                &self.pool,
                "DELETE FROM decorate_share WHERE share_id = ?",
                &[Bind::I(share_id)],
            )
            .await?
                > 0,
        )
    }

    // ————————————————————— decorate_discrete 装修组件 —————————————————————

    /// 按 sn + 店铺查装修组件（对应 `DecorateDiscreteServiceImpl#detail`）。
    pub async fn find_discrete(
        &self,
        shop_id: i32,
        decorate_sn: &str,
    ) -> Result<Option<DecorateDiscrete>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM decorate_discrete WHERE shop_id = ? AND decorate_sn = ? LIMIT 1",
            &[Bind::I(shop_id), Bind::S(decorate_sn.to_string())],
        )
        .await
    }

    /// 按 ID 查装修组件。
    pub async fn find_discrete_by_id(&self, id: i32) -> Result<Option<DecorateDiscrete>> {
        fetch_one(
            &self.pool,
            "SELECT * FROM decorate_discrete WHERE id = ?",
            &[Bind::I(id)],
        )
        .await
    }

    /// 全量列出某店铺的装修组件（管理端批量编辑用）。
    pub async fn list_discretes(&self, shop_id: Option<i32>) -> Result<Vec<DecorateDiscrete>> {
        match shop_id {
            Some(shop_id) => {
                fetch_rows(
                    &self.pool,
                    "SELECT * FROM decorate_discrete WHERE shop_id = ? ORDER BY id ASC",
                    &[Bind::I(shop_id)],
                )
                .await
            }
            None => {
                fetch_rows(&self.pool, "SELECT * FROM decorate_discrete ORDER BY id ASC", &[])
                    .await
            }
        }
    }

    /// 新增装修组件，返回自增主键。
    pub async fn insert_discrete(&self, d: &DecorateDiscrete) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO decorate_discrete (decorate_sn, decorate_name, data, shop_id) \
             VALUES (?,?,?,?) RETURNING id",
        )
        .bind(&d.decorate_sn)
        .bind(&d.decorate_name)
        .bind(&d.data)
        .bind(d.shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 全量更新装修组件。
    pub async fn update_discrete(&self, d: &DecorateDiscrete) -> Result<()> {
        sqlx::query(
            "UPDATE decorate_discrete SET decorate_sn=?, decorate_name=?, data=?, shop_id=? \
             WHERE id=?",
        )
        .bind(&d.decorate_sn)
        .bind(&d.decorate_name)
        .bind(&d.data)
        .bind(d.shop_id)
        .bind(d.id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 删除装修组件。
    pub async fn delete_discrete(&self, id: i32) -> Result<bool> {
        Ok(
            exec(
                &self.pool,
                "DELETE FROM decorate_discrete WHERE id = ?",
                &[Bind::I(id)],
            )
            .await?
                > 0,
        )
    }

    // ————————————————————— locales 语言 —————————————————————

    /// 查语言名称（子页面 `children.language` 回填）。
    ///
    /// 对应 `DecorateServiceImpl` 详情里按 `localeId` 取语言展示名。
    pub async fn find_locale_language(&self, locale_id: Option<i32>) -> Result<Option<String>> {
        let Some(locale_id) = locale_id else {
            return Ok(None);
        };
        let row = fetch_one::<(Option<String>,)>(
            &self.pool,
            "SELECT language FROM locales WHERE id = ?",
            &[Bind::I(locale_id)],
        )
        .await?;
        Ok(row.and_then(|r| r.0))
    }
}

/// 执行写语句并返回受影响行数。
async fn exec(pool: &PgPool, sql: &str, params: &[Bind]) -> Result<u64> {
    let pg_sql = crate::db::to_pg_placeholders(sql);
    let mut q = sqlx::query(&pg_sql);
    for p in params {
        q = match p {
            Bind::I(v) => q.bind(*v),
            Bind::I64(v) => q.bind(*v),
            Bind::S(v) => q.bind(v.clone()),
        };
    }
    let r = q.execute(pool).await.map_err(map_err)?;
    Ok(r.rows_affected())
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("装修页数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_conversion() {
        assert_eq!(to_snake("updateTime"), "update_time");
        assert_eq!(to_snake("isHome"), "is_home");
        assert_eq!(to_snake("decorate_title"), "decorate_title");
    }

    #[test]
    fn sort_column_whitelist() {
        assert_eq!(resolve_sort_column(Some("updateTime")), "update_time");
        assert_eq!(resolve_sort_column(Some("evil; DROP TABLE")), "decorate_id");
        assert_eq!(resolve_sort_column(None), "decorate_id");
    }

    #[test]
    fn in_clause_builds_placeholders() {
        let mut sql = String::from(" WHERE 1=1");
        let mut params = Vec::new();
        push_in(&mut sql, &mut params, "page_type", &[1, 3]);
        assert_eq!(sql, " WHERE 1=1 AND page_type IN (?,?)");
        assert_eq!(params.len(), 2);
    }
}"