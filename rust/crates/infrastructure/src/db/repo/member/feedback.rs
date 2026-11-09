//! 会员留言域（member::feedback）仓储：`feedback` + 关联回查表。
//!
//! 对应原 Java `com.nextrade.mapper.user.FeedbackMapper`
//! （`BaseMapper<Feedback>`，无自定义 XML SQL）以及
//! `com.nextrade.service.user.impl.FeedbackServiceImpl` 里通过
//! `userService` / `orderService` / `productService` / `shopService` /
//! `adminUserService` 做的关联表回查。
//!
//! # 列名对齐（权威来源）
//!
//! 全部表名/列名来自 `java/database/pure.sql`，逐条用
//! `cd rust && python tools/schema_parser.py --table <表名>` 核对：
//!
//! - `feedback`（16 列，主键 `id`）：
//!   `id` / `parent_id` / `user_id` / `username` / `email` / `mobile` /
//!   `title` / `type` / `status` / `content` / `add_time` / `feedback_pics` /
//!   `product_id` / `order_id` / `complaint_info` / `shop_id`
//! - `user`：只回查 `user_id` → `nickname`（`nickname varchar(80)` **可空**）
//! - `order`：只回查 `order_id` → `order_sn`（`varchar(20)` NOT NULL；
//!   表名 `order` 是 MySQL 保留字，**必须加反引号**）
//! - `product`：只回查 `product_id` → `product_name`（NOT NULL）
//! - `shop`：只回查 `shop_id` → `shop_title`（`varchar(255)` **可空**）
//! - `admin_user`：只回查 `admin_id` → `username`（NOT NULL）
//!
//! # 软删除
//!
//! `feedback` 表**没有** `is_del` / `is_delete` 列（schema 已核对），
//! 因此 Java 的 `removeById` / `removeByIds` 就是**物理删除**，本仓储同样写
//! `DELETE`。
//!
//! # 事务
//!
//! Java 的 `update` / `batch` 带 `@Transactional(rollbackFor = Exception.class)`
//! 且是「多条写语句」组合，因此本仓储的写方法统一接收
//! `&mut sqlx::Transaction<'_, sqlx::Postgres>`，由**服务层**用
//! [`FeedbackRepository::begin`] 开启事务并编排（与 user_company / member_tag_group 一致）。
//! `del` 在 Java 里**没有**事务，本仓储提供连接池版本的 [`Self::delete_by_id`]。
//!
//! # 金额
//!
//! `feedback` 的 16 列里**没有** `decimal` 列（schema 已核对），本域不涉及金额，
//! 因此全文**没有任何浮点类型**（`f64` / `f32` 一律禁用；若将来新增金额列，
//! 必须用 `rust_decimal::Decimal`）。
//!
//! # SQL 形式
//!
//! 统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//! 所有 `IN (...)` 的占位符数量由传入列表的长度决定；列表为空时
//! **服务层已提前返回**，不会生成 `IN ()` 这种非法 SQL。

use std::collections::HashMap;

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::user::Feedback;
use nextrade_core::models::member::feedback::{
    feedback_status, FeedbackListQuery, MSG_FEEDBACK_NOT_FOUND, SHOP_FEEDBACK_TYPES,
};
use nextrade_core::{Error, Result};

/// `feedback` 的固定 SELECT 列清单（避免 `SELECT *` 与实体字段顺序耦合；
/// 与 [`nextrade_core::entities::user::Feedback`] 的 16 个字段一一对应，
/// 顺序也一致）。
///
/// ⚠️ 刻意写成**单行**字符串字面量：`tools/check_sql_columns.py` 按「行内
/// 引号字面量」提取 SQL，多行续行写法会让它整条跳过，从而失去列名校验。
const FEEDBACK_SELECT: &str = "SELECT id, parent_id, user_id, username, email, mobile, title, type, status, content, add_time, feedback_pics, product_id, order_id, complaint_info, shop_id FROM feedback";

/// 生成 `?, ?, ?` 占位串（长度由调用方保证与绑定参数个数一致）。
fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

/// 把补丁字段追加为 `col = ?` 赋值片段。
///
/// `$field` 的类型是 `&Option<Option<T>>`：
/// - 外层 `None`（调用方明确「不写该列」）与内层 `None`（值为 null）都**不写**
///   —— 对应 MyBatis-Plus 的 `FieldStrategy.NOT_NULL`
/// - `Some(Some(v))` 才绑定参数
///
/// 用宏而非泛型函数是为了避开 `sqlx::Encode` + `QueryBuilder` 生命周期
/// 在辅助函数签名里的书写困难（与 admin_user / pc_cat_floor / area_code / tax_rate
/// 同一思路）。
///
/// 注意：`macro_rules!` 必须**先定义后使用**（按文本顺序展开），
/// 故本宏置于导入区之后、`impl` 之前，不能挪到文件末尾。
macro_rules! push_assign {
    ($qb:expr, $wrote_any:expr, $column:literal, $field:expr) => {
        if let Some(Some(v)) = $field.as_ref() {
            if $wrote_any {
                $qb.push(", ");
            }
            $qb.push($column);
            $qb.push(" = ");
            $qb.push_bind(v.clone());
            $wrote_any = true;
        }
    };
}

/// `list` 的过滤条件（由**服务层**按 Java `FeedbackServiceImpl#list` 的
/// 租户口径预先算好后传入，仓储只负责拼 SQL）。
#[derive(Debug, Clone)]
pub struct FeedbackListFilter {
    /// 类型筛选结果：
    /// - 平台账号 = [`nextrade_core::models::member::feedback::resolve_platform_feedback_types`]
    ///   （hutool `split` 语义）
    /// - 店铺账号 = [`nextrade_core::models::member::feedback::resolve_shop_feedback_types`]
    ///   （只保留 `"2"`，可能为空）
    ///
    /// 空列表 ⇒ **不加** `type IN (...)`（Java 的 `in(CollUtil.isNotEmpty(...))`）
    pub types: Vec<i32>,
    /// 是否追加恒假条件 `1 = 0`
    /// （Java：`apply(scopedTypeRequested && CollUtil.isEmpty(types), "1 = 0")`）
    pub force_empty: bool,
    /// 是否平台账号（Java `tenantScope.platformAdmin()`）：
    /// 追加 `AND (shop_id = 0 OR type NOT IN (2))`
    pub platform_admin: bool,
    /// 生效的店铺过滤值（Java `queryShopId`）：
    /// 店铺账号 = 当前店铺 ID；平台账号 = 请求参数 `shopId`；`None` ⇒ 不加条件
    pub shop_id: Option<i32>,
}

impl FeedbackListFilter {
    /// 平台账号（Java `TenantScope(false, true, 0)`）：
    /// 不加 `shop_id` 强制值，但叠加平台可见性条件；
    /// `shop_id` 取请求参数（可为 `None`）。
    #[must_use]
    pub fn platform(types: Vec<i32>, shop_id: Option<i32>) -> Self {
        Self {
            types,
            force_empty: false,
            platform_admin: true,
            shop_id,
        }
    }

    /// 店铺 / 门店账号（Java `TenantScope(true, false, shopId)`）。
    ///
    /// `force_empty` = 请求指定了类型但筛选后为空集
    /// （Java 会追加 `1 = 0` 得到空页）。
    #[must_use]
    pub fn shop(types: Vec<i32>, shop_id: i32, force_empty: bool) -> Self {
        Self {
            types,
            force_empty,
            platform_admin: false,
            shop_id: Some(shop_id),
        }
    }
}

#[derive(Clone)]
pub struct FeedbackRepository {
    pool: PgPool,
}

impl FeedbackRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（服务层编排用，避免服务层直接依赖 `sqlx::Transaction`
    /// 的构造细节；与 user_company / member_tag_group 同一做法）。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 列表 —————————————————————

    /// 会员留言分页（对应 Java `FeedbackServiceImpl#list(pageQuery, tenantScope)`）。
    ///
    /// WHERE 条件见 [`Self::push_filters`]（与 Java 逐条对齐）；
    /// `ORDER BY` 由服务层用
    /// [`nextrade_core::models::member::feedback::list_order_clause`] 算好后传入，
    /// LIMIT / OFFSET 来自
    /// [`nextrade_core::models::member::feedback::FeedbackListQuery::page_query`]。
    ///
    /// ⚠️ 与 Java 相同：`parent_id = 0`（只看主留言，回复不进列表）
    /// **无条件**追加；查询参数 `excludeReply` 在本链路里**不被读取**。
    pub async fn paginate(
        &self,
        q: &FeedbackListQuery,
        filter: &FeedbackListFilter,
        order_clause: &str,
    ) -> Result<(u64, Vec<Feedback>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();

        let mut count_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM feedback WHERE 1 = 1");
        Self::push_filters(&mut count_qb, q, filter);
        let total: (i64,) = count_qb
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let mut row_qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{FEEDBACK_SELECT} WHERE 1 = 1"));
        Self::push_filters(&mut row_qb, q, filter);
        // ORDER BY 子句里的列名来自 models::nextrade_core::models::member::feedback::map_order_column 的白名单
        // （未命中回退主键列），方向只有 ASC / DESC 两种，均无注入面；
        // LIMIT / OFFSET 是 u64 字面量。
        row_qb.push(order_clause);
        row_qb.push(format!(" LIMIT {limit} OFFSET {offset}"));

        let rows: Vec<Feedback> = row_qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 追加 `list` 的 WHERE 片段（`COUNT` 与取行两条 SQL 共用，保证条件一致）。
    ///
    /// 逐条对照 Java `FeedbackServiceImpl#list`：
    /// - `like(isNotBlank(keyword), content, keyword)` → `content LIKE %kw%`
    ///   （`"   "` 视为空 ⇒ **不加**条件，`effective_keyword()` 已按该口径实现）
    /// - `in(isNotEmpty(types), type, types)` → `type IN (...)`
    /// - `apply(scopedTypeRequested && isEmpty(types), "1 = 0")` → 恒假条件
    /// - `eq(parentId, 0)` → 无条件追加
    /// - `isOrder == 1` → `order_id = 0`；`isOrder > 1` → `order_id > 0`
    ///   （`isOrder` 为 `null` / `0` / 负数时**两个条件都不加**）
    /// - `productId != null` → `product_id = ?`（**0 也要加**，Java 只判 null）
    /// - `status != null` → `status = ?`（同上）
    /// - `userId != null && userId > 0` → `user_id = ?`
    /// - `queryShopId != null` → `shop_id = ?`
    /// - `platformAdmin` → `AND (shop_id = 0 OR type NOT IN (2))`
    fn push_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        q: &FeedbackListQuery,
        f: &FeedbackListFilter,
    ) {
        if let Some(keyword) = q.effective_keyword() {
            qb.push(" AND content LIKE ");
            qb.push_bind(format!("%{keyword}%"));
        }
        if !f.types.is_empty() {
            // ⚠️ `QueryBuilder::push` 里写的 `?` 是**字面量**，不会绑定参数；
            // 占位符必须由 `push_bind` 自己追加。
            // 曾经写成 `push(" AND type IN (?)") + push_bind(..)`，
            // 生成 `AND type IN (?) ?` → MySQL 1064 `near '?'`
            // （2026-10-21 线上 `/adminapi/user/feedback/list` 500 的根因）。
            qb.push(" AND type IN (");
            let mut sep = qb.separated(", ");
            for t in &f.types {
                sep.push_bind(*t);
            }
            sep.push_unseparated(")");
        }
        if f.force_empty {
            qb.push(" AND 1 = 0");
        }
        qb.push(" AND parent_id = 0");
        match q.is_order {
            Some(v) if v == 1 => {
                qb.push(" AND order_id = 0");
            }
            Some(v) if v > 1 => {
                qb.push(" AND order_id > 0");
            }
            _ => {}
        }
        if let Some(product_id) = q.product_id {
            qb.push(" AND product_id = ");
            qb.push_bind(product_id);
        }
        if let Some(status) = q.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
        if let Some(user_id) = q.user_id.filter(|v| *v > 0) {
            qb.push(" AND user_id = ");
            qb.push_bind(user_id);
        }
        if let Some(shop_id) = f.shop_id {
            qb.push(" AND shop_id = ");
            qb.push_bind(shop_id);
        }
        if f.platform_admin {
            // Java：`.and(platformAdmin, w -> w.eq(shopId, 0).or()
            //        .notIn(type, SHOP_FEEDBACK_TYPES))`
            // `SHOP_FEEDBACK_TYPES = Set.of(2)` ⇒ `type NOT IN (2)`
            //
            // ⚠️ 同 `type IN (...)`：占位符只能由 `push_bind` 追加，
            // 不能在 `push` 的字面量里写 `?`（否则 `near '?'` 1064）。
            qb.push(" AND (shop_id = 0 OR type NOT IN (");
            let mut sep = qb.separated(", ");
            for t in SHOP_FEEDBACK_TYPES {
                sep.push_bind(t);
            }
            sep.push_unseparated("))");
        }
    }

    // ————————————————————— 单行查询 —————————————————————

    /// 按主键查询（对应 Java `BaseServiceImpl#getById`；表无软删列，不加额外条件）。
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Feedback>> {
        sqlx::query_as::<_, Feedback>(&format!("{FEEDBACK_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 事务内按主键查询（`update` / `batch` 的「先校验后写」必须同事务）。
    pub async fn find_by_id_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
    ) -> Result<Option<Feedback>> {
        sqlx::query_as::<_, Feedback>(&format!("{FEEDBACK_SELECT} WHERE id = $1 LIMIT 1"))
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)
    }

    /// 一批主留言的**全部**回复（对应 Java `list` 里的
    /// `in(parentId, feedbackIds).orderByDesc(id)`）。
    ///
    /// ⚠️ 顺序**必须**是 `ORDER BY id DESC`：Java 用
    /// `replyMap.putIfAbsent(reply.getParentId(), reply)` 取「第一条」，
    /// 靠的就是这个降序 ⇒ 每个父留言**最新**的一条回复。
    pub async fn find_replies_by_parent_ids(&self, parent_ids: &[i32]) -> Result<Vec<Feedback>> {
        if parent_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "{FEEDBACK_SELECT} WHERE parent_id IN ({}) ORDER BY id DESC",
            placeholders(parent_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut query = sqlx::query_as::<_, Feedback>(&sql);
        for id in parent_ids {
            query = query.bind(*id);
        }
        query.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某父留言的最新一条回复（对应 Java `update` 编辑分支的
    /// `getOne(eq(parentId, id).orderByDesc(id).last("LIMIT 1"))`）。
    ///
    /// 没有回复时返回 `Ok(None)` ⇒ 服务层抛 `"回复记录不存在"`。
    pub async fn find_latest_reply_by_parent_id(&self, parent_id: i32) -> Result<Option<Feedback>> {
        sqlx::query_as::<_, Feedback>(&format!(
            "{FEEDBACK_SELECT} WHERE parent_id = $1 ORDER BY id DESC LIMIT 1"
        ))
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 某留言的**全部**回复（对应 Java `detail` 里的
    /// `eq(parentId, feedback.id).orderByDesc(id)`，不带 LIMIT）。
    pub async fn find_all_replies_by_parent_id(&self, parent_id: i32) -> Result<Vec<Feedback>> {
        sqlx::query_as::<_, Feedback>(&format!(
            "{FEEDBACK_SELECT} WHERE parent_id = $1 ORDER BY id DESC"
        ))
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— 关联表批量回查（list 用，避免 N+1） —————————————————————

    /// 会员昵称批量回查（对应 Java
    /// `userService.list(in(userId, userIds))` → `Map<Integer, User>`）。
    ///
    /// `user.nickname` 可空 ⇒ 值用 `Option<String>`；
    /// 「查不到该会员」与「昵称为 NULL」在 VO 上都表现为 `nickname: null`，
    /// 与 Java 的 `if (user != null) vo.setNickname(user.getNickname())` 一致。
    pub async fn find_nicknames(&self, user_ids: &[i32]) -> Result<HashMap<i32, Option<String>>> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT user_id, nickname FROM \"user\" WHERE user_id IN ({})",
            placeholders(user_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut query = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in user_ids {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 订单号批量回查（对应 Java
    /// `orderService.list(in(orderId, orderIds))` → `Map<Integer, Order>`）。
    ///
    /// ⚠️ 表名 `order` 是 MySQL 保留字，**必须加反引号**
    /// （与 `repo/order.rs` / `repo/aftersales_client.rs` 同一写法）。
    pub async fn find_order_sns(&self, order_ids: &[i32]) -> Result<HashMap<i32, String>> {
        if order_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT order_id, order_sn FROM \"order\" WHERE order_id IN ({})",
            placeholders(order_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut query = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in order_ids {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 商品名称批量回查（对应 Java `productService.list(in(productId, ...))`）。
    ///
    /// `product.product_name` 是 `varchar(512) NOT NULL` ⇒ 裸 `String`。
    pub async fn find_product_names(&self, product_ids: &[i32]) -> Result<HashMap<i32, String>> {
        if product_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT product_id, product_name FROM product WHERE product_id IN ({})",
            placeholders(product_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut query = sqlx::query_as::<_, (i32, String)>(&sql);
        for id in product_ids {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 店铺名称批量回查（对应 Java `shopService.list(in(shopId, ...))`）。
    ///
    /// `shop.shop_title` 可空 ⇒ 值用 `Option<String>`
    /// （Java 直接 `shopVO.setShopTitle(shop.getShopTitle())`，NULL 就传 NULL）。
    pub async fn find_shop_titles(&self, shop_ids: &[i32]) -> Result<HashMap<i32, Option<String>>> {
        if shop_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT shop_id, shop_title FROM shop WHERE shop_id IN ({})",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut query = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in shop_ids {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    // ————————————————————— 关联表单行回查（detail 用） —————————————————————

    /// 单个会员昵称（对应 Java `detail` 里的
    /// `userService.getOne(eq(userId, feedback.userId))`，只取 `nickname`）。
    ///
    /// 会员不存在或昵称为 NULL 都返回 `Ok(None)` —— 与 Java
    /// 「`user == null` 就不 `setNickname`」的可观测行为一致，**不**报错。
    pub async fn find_user_nickname(&self, user_id: i32) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT nickname FROM \"user\" WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    /// 单个订单 `(order_id, order_sn)`（对应 Java `detail` 里的
    /// `orderService.getOne(eq(orderId, ...))`，只用到这两列）。
    ///
    /// ⚠️ Java 用 `BeanUtils.copyProperties(order, orderInfo)`，
    /// 目标 VO `UserFeedbackDetailVO.OrderInfo` 只有 `orderId` + `orderSn`
    /// 两个同名属性，因此这里同样只 SELECT 这两列。
    pub async fn find_order_brief(&self, order_id: i32) -> Result<Option<(i32, String)>> {
        sqlx::query_as::<_, (i32, String)>(&format!(
            "SELECT order_id, order_sn FROM \"order\" WHERE order_id = $1 LIMIT 1"
        ))
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 单个商品 `(product_id, product_name)`（对应 Java `detail` 里的
    /// `productService.getOne(eq(productId, ...))`）。
    pub async fn find_product_brief(&self, product_id: i32) -> Result<Option<(i32, String)>> {
        sqlx::query_as::<_, (i32, String)>(
            "SELECT product_id, product_name FROM product WHERE product_id = $1 LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 单个店铺 `(shop_id, shop_title)`（对应 Java `detail` 里的
    /// `shopService.getOne(eq(shopId, ...))`）。
    pub async fn find_shop_brief(&self, shop_id: i32) -> Result<Option<(i32, Option<String>)>> {
        sqlx::query_as::<_, (i32, Option<String>)>(
            "SELECT shop_id, shop_title FROM shop WHERE shop_id = $1 LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 当前登录管理员的用户名（对应 Java
    /// `adminUserService.getOne(eq(adminId, getCurrentAdminId()))`，
    /// 本域**只用到** `getUsername()`）。
    ///
    /// 查不到返回 `Ok(None)` ⇒ 服务层抛 `"管理员不存在"`。
    pub async fn find_admin_username(&self, admin_id: i32) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT username FROM admin_user WHERE admin_id = $1 LIMIT 1")
                .bind(admin_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    // ————————————————————— 写入 —————————————————————

    /// 新增一条回复（对应 Java `update` 的「`parentId == 0`」分支里的
    /// `this.save(reply)`）。
    ///
    /// 与 member_tag_group / area_code 同一思路：`Option` 字段为 `None` 时
    /// **不写该列**，由 MySQL 施加列默认值
    /// （MyBatis-Plus `FieldStrategy.NOT_NULL` 的等价语义）。
    ///
    /// ⚠️ `complaint_info` 列本域**从不写**（Java 的 `update` 也没碰它）：
    /// 该列 `NULL`，`FieldStrategy.NOT_NULL` 同样跳过。
    pub async fn insert_reply(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        v: &FeedbackReplyInsert,
    ) -> Result<i32> {
        let mut columns: Vec<&'static str> = Vec::with_capacity(15);
        let mut binds: Vec<InsertBind<'_>> = Vec::with_capacity(15);

        columns.push("parent_id");
        binds.push(InsertBind::Int(v.parent_id));
        columns.push("user_id");
        binds.push(InsertBind::Int(v.user_id));
        columns.push("username");
        binds.push(InsertBind::Text(&v.username));
        columns.push("title");
        binds.push(InsertBind::Text(&v.title));
        columns.push("type");
        binds.push(InsertBind::Int(v.r#type));
        columns.push("status");
        binds.push(InsertBind::Int(v.status));
        columns.push("content");
        binds.push(InsertBind::Text(&v.content));
        columns.push("add_time");
        binds.push(InsertBind::Int(v.add_time));
        columns.push("feedback_pics");
        binds.push(InsertBind::Text(&v.feedback_pics));
        // 可空列：None ⇒ 不写（Java 的 NOT_NULL 策略同样跳过）
        if let Some(email) = v.email.as_deref() {
            columns.push("email");
            binds.push(InsertBind::Text(email));
        }
        if let Some(mobile) = v.mobile.as_deref() {
            columns.push("mobile");
            binds.push(InsertBind::Text(mobile));
        }
        if let Some(product_id) = v.product_id {
            columns.push("product_id");
            binds.push(InsertBind::Int(product_id));
        }
        if let Some(order_id) = v.order_id {
            columns.push("order_id");
            binds.push(InsertBind::Int(order_id));
        }
        if let Some(shop_id) = v.shop_id {
            columns.push("shop_id");
            binds.push(InsertBind::Int(shop_id));
        }

        debug_assert_eq!(columns.len(), binds.len());
        let sql = format!(
            "INSERT INTO feedback ({}) VALUES ({})",
            columns.join(", "),
            placeholders(columns.len())
        );
        let sql = crate::db::to_pg_placeholders(&format!("{} RETURNING id", &sql));
        let mut query = sqlx::query_scalar(&sql);
        for b in &binds {
            query = match b {
                InsertBind::Text(t) => query.bind(*t),
                InsertBind::Int(i) => query.bind(*i),
            };
        }
        let res: i32 = query.fetch_one(&mut **tx).await.map_err(map_err)?;
        Ok(res)
    }

    /// 把父留言标记为「已回复」（对应 Java `update` 里的
    /// `parentFeedback.setStatus(REPLIED)` + `this.updateById(parentFeedback)`）。
    ///
    /// ⚠️ Java 的 `updateById` 会把实体里**所有非 null 字段**写回，
    /// 而这些值都来自刚刚 `getById` 出的同一行 ⇒ 净效果等价于
    /// **只改 `status` 一列**，因此这里只 `SET status`（见 `notes`）。
    pub async fn mark_parent_replied(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
    ) -> Result<u64> {
        let res = sqlx::query("UPDATE feedback SET status = $1 WHERE id = $2")
            .bind(feedback_status::REPLIED)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 更新一条已有回复（对应 Java `update` 的「`parentId != 0`」分支里的
    /// `reply.setXxx(...)` + `this.updateById(reply)`）。
    ///
    /// 只写 `Some(Some(v))` 的列（见 [`FeedbackReplyPatch`]）：
    /// Java 的 `updateById` 虽然会带上**全部**非 null 字段，但其中未改动的
    /// 列写回的是同一个旧值 ⇒ 与只写「被改动列」等价。
    pub async fn update_reply(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        id: i32,
        patch: &FeedbackReplyPatch,
    ) -> Result<u64> {
        let mut qb: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE feedback SET ");
        let mut wrote_any = false;
        push_assign!(qb, wrote_any, "user_id", patch.user_id);
        push_assign!(qb, wrote_any, "username", patch.username);
        push_assign!(qb, wrote_any, "email", patch.email);
        push_assign!(qb, wrote_any, "mobile", patch.mobile);
        push_assign!(qb, wrote_any, "content", patch.content);
        push_assign!(qb, wrote_any, "feedback_pics", patch.feedback_pics);
        push_assign!(qb, wrote_any, "type", patch.r#type);
        push_assign!(qb, wrote_any, "product_id", patch.product_id);
        push_assign!(qb, wrote_any, "order_id", patch.order_id);
        push_assign!(qb, wrote_any, "shop_id", patch.shop_id);

        if !wrote_any {
            // 没有任何列要写：MyBatis-Plus 恒把主键放进 SET
            // （`UPDATE feedback SET id = ? WHERE id = ?`），
            // 这里同样用「自赋值」保持 `rows_affected` 与 Java 一致，
            // 且不产生 `UPDATE feedback SET WHERE id = ?` 这种非法 SQL。
            qb.push("id = ");
            qb.push_bind(id);
        }
        qb.push(" WHERE id = ");
        qb.push_bind(id);

        let res = qb.build().execute(&mut **tx).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 按主键物理删除（对应 Java `del` → `removeById`；表无软删列）。
    pub async fn delete_by_id(&self, id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM feedback WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量物理删除（对应 Java `batch` → `removeByIds`，`@Transactional`）。
    ///
    /// 空列表由服务层提前返回；这里再兜一层，避免生成
    /// `WHERE id IN ()` 这种非法 SQL。
    pub async fn delete_by_ids(
        &self,
        tx: &mut sqlx::Transaction<'_, Postgres>,
        ids: &[i32],
    ) -> Result<u64> {
        debug_assert!(!ids.is_empty(), "delete_by_ids 不允许空列表");
        if ids.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "DELETE FROM feedback WHERE id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut query = sqlx::query(&sql);
        for id in ids {
            query = query.bind(*id);
        }
        let res = query.execute(&mut **tx).await.map_err(map_err)?;
        Ok(res.rows_affected())
    }
}

/// [`insert_reply`] 的入参（一条新回复）。
///
/// `Option` 字段为 `None` ⇒ 不写该列（走 MySQL 列默认值；
/// `feedback` 的可空列默认都是 NULL）。
#[derive(Debug, Clone)]
pub struct FeedbackReplyInsert {
    /// 父留言 ID（`NOT NULL`）
    pub parent_id: i32,
    /// 回复人（管理员）ID（Java `reply.setUserId(currentAdminId)`）
    pub user_id: i32,
    /// 回复人用户名（来自 `admin_user.username`）
    pub username: String,
    /// 标题（Java 硬编码 `"reply"`）
    pub title: String,
    /// 类型（继承父留言的 `type`）
    pub r#type: i32,
    /// 状态（Java 固定 `UserFeedbackStatus.WAIT_REPLY` = 0）
    pub status: i32,
    /// 内容（XSS 清洗后；`null` → 空串）
    pub content: String,
    /// 秒级时间戳（Java `StringUtils.getCurrentTime()` = 秒）
    pub add_time: i32,
    /// 图片 JSON（Java `toFeedbackPics`：`null` / 空数组 → 字面量 `"[]"`）
    pub feedback_pics: String,
    /// 邮箱（Java `StrUtil.blankToDefault(dto.getEmail(), "")` ⇒ 一般非 NULL）
    pub email: Option<String>,
    /// 手机号（同上）
    pub mobile: Option<String>,
    /// 商品 ID（继承父留言；父为 NULL ⇒ 不写该列）
    pub product_id: Option<i32>,
    /// 订单 ID（同上）
    pub order_id: Option<i32>,
    /// 店铺 ID（同上）
    pub shop_id: Option<i32>,
}

/// [`update_reply`] 的部分更新补丁。
///
/// 每个字段三态：`None` / `Some(None)` ⇒ 不写该列；`Some(Some(v))` ⇒ 写该列。
///
/// ⚠️ 关键差异（对齐 mybatis-plus 3.5 的 `NOT_NULL` 策略）：
/// Java 只跳过 `null`，**空串 `""` 与 `0` 都会照常写入**，
/// 因此调用方必须显式构造 `Some(Some(""))` / `Some(Some(0))`，
/// 不能用「值为空就跳过」的简化写法。
#[derive(Debug, Clone, Default)]
pub struct FeedbackReplyPatch {
    /// `user_id`（回复人改为当前管理员）
    pub user_id: Option<Option<i32>>,
    /// `username`（改为管理员用户名）
    pub username: Option<Option<String>>,
    /// `email`
    pub email: Option<Option<String>>,
    /// `mobile`
    pub mobile: Option<Option<String>>,
    /// `content`（XSS 清洗后的正文）
    pub content: Option<Option<String>>,
    /// `feedback_pics`（图片 JSON）
    pub feedback_pics: Option<Option<String>>,
    /// `type`（继承父留言）
    pub r#type: Option<Option<i32>>,
    /// `product_id`（继承父留言；父为 NULL 时**不写**该列 ⇒ 保留旧值）
    pub product_id: Option<Option<i32>>,
    /// `order_id`（同上）
    pub order_id: Option<Option<i32>>,
    /// `shop_id`（同上）
    pub shop_id: Option<Option<i32>>,
}

impl FeedbackReplyPatch {
    /// 是否没有任何列会被写入。
    ///
    /// 判据必须与 [`push_assign`] 的实际写入条件（`Some(Some(v))`）一致：
    /// 外层 `None`（不写）与内层 `None`（值为 null，MyBatis-Plus
    /// `FieldStrategy.NOT_NULL` 同样跳过）都**不会**产生 `SET` 片段。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !matches!(self.user_id, Some(Some(_)))
            && !matches!(self.username, Some(Some(_)))
            && !matches!(self.email, Some(Some(_)))
            && !matches!(self.mobile, Some(Some(_)))
            && !matches!(self.content, Some(Some(_)))
            && !matches!(self.feedback_pics, Some(Some(_)))
            && !matches!(self.r#type, Some(Some(_)))
            && !matches!(self.product_id, Some(Some(_)))
            && !matches!(self.order_id, Some(Some(_)))
            && !matches!(self.shop_id, Some(Some(_)))
    }
}

/// INSERT 的绑定值（避免为文本列与整型列定义平行 `Vec`）。
#[derive(Debug, Clone, Copy)]
enum InsertBind<'a> {
    /// 文本列
    Text(&'a str),
    /// 整型列
    Int(i32),
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_FEEDBACK_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::feedback;

    fn query(json: serde_json::Value) -> FeedbackListQuery {
        serde_json::from_value(json).expect("应能反序列化")
    }

    /// 用 QueryBuilder 拼一遍 SQL，便于断言片段（不连库）。
    fn build(q: &FeedbackListQuery, f: &FeedbackListFilter) -> String {
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new(format!("{FEEDBACK_SELECT} WHERE 1 = 1"));
        FeedbackRepository::push_filters(&mut qb, q, f);
        qb.push(" ORDER BY id DESC LIMIT 15 OFFSET 0");
        qb.sql().to_string()
    }

    /// SELECT 的列清单必须覆盖 schema 的 16 列（漏列会在运行期报
    /// `mismatched column count`，编译期查不出来）。
    #[test]
    fn select_clause_lists_all_schema_columns() {
        for col in nextrade_core::models::member::feedback::FEEDBACK_COLUMNS {
            assert!(
                FEEDBACK_SELECT.contains(col),
                "SELECT 缺少列 {col}: {FEEDBACK_SELECT}"
            );
        }
        // 不得使用 SELECT *
        assert!(!FEEDBACK_SELECT.contains('*'));
        assert!(FEEDBACK_SELECT.ends_with(" FROM feedback"));
    }

    /// 占位符数量必须与列表长度一致（`IN ()` 是非法 SQL）。
    #[test]
    fn placeholders_count_matches_list_len() {
        assert_eq!(placeholders(0), "");
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");
    }

    /// `parent_id = 0` 无条件追加（回复不进列表）。
    #[test]
    fn always_filters_top_level_feedback() {
        let sql = build(
            &query(serde_json::json!({})),
            &FeedbackListFilter::platform(vec![], None),
        );
        assert!(sql.contains(" AND parent_id = 0"), "{sql}");
        // 无类型筛选 ⇒ 不加 type IN
        assert!(!sql.contains("type IN"), "{sql}");
        // 平台账号 ⇒ 追加可见性条件，且只有一个占位符（SHOP_FEEDBACK_TYPES 长度 1）
        assert!(
            sql.contains(" AND (shop_id = 0 OR type NOT IN (?))"),
            "{sql}"
        );
        // 占位符数量必须与绑定参数数量一致：`push` 里的字面量 `?` 会导致
        // MySQL 1064 `near '?'`（2026-10-21 `/adminapi/user/feedback/list` 500）。
        assert_eq!(
            sql.matches('?').count(),
            1,
            "平台可见性条件只应有 1 个占位符: {sql}"
        );
    }

    /// `type IN (...)` 必须由 `push_bind` 生成占位符，
    /// **不能**在 `push` 的字面量里写 `?`（否则 SQL 变成 `IN (?) ?`）。
    #[test]
    fn type_in_placeholders_match_binds() {
        let f = FeedbackListFilter::platform(vec![2, 3], None);
        let sql = build(&query(serde_json::json!({})), &f);
        // `type IN (?, ?)` + `NOT IN (?)`
        assert_eq!(sql.matches('?').count(), 3, "占位符数量不符: {sql}");
        assert!(sql.contains("type IN (?, ?)"), "{sql}");
        // 不得出现 `()) ?`、`(?) ?` 这类重复占位
        assert!(!sql.contains(") ?"), "疑似重复占位符: {sql}");
    }

    /// `type` 为 `null` / `""` 时（平台账号）**不加**类型条件。
    #[test]
    fn empty_types_adds_no_condition() {
        let q = query(serde_json::json!({}));
        let types = nextrade_core::models::member::feedback::resolve_platform_feedback_types(
            q.r#type.as_deref(),
        );
        let sql = build(&q, &FeedbackListFilter::platform(types, None));
        assert!(!sql.contains("type IN ("), "{sql}");
    }

    /// 店铺账号：`type IN (...)` + `shop_id = ?`，且**不**追加平台可见性条件。
    #[test]
    fn shop_scope_conditions() {
        let q = query(serde_json::json!({"type": "2"}));
        let types = nextrade_core::models::member::feedback::resolve_shop_feedback_types(
            q.r#type.as_deref(),
        );
        let sql = build(&q, &FeedbackListFilter::shop(types, 7, false));
        // `type IN (?)` 的占位由 push_bind 生成（字面量 `?` 会重复绑定）
        assert!(sql.contains(" AND type IN (?)"), "{sql}");
        assert_eq!(
            sql.matches('?').count(),
            2,
            "IN + shop_id 共 2 个占位: {sql}"
        );
        assert!(!sql.contains(") ?"), "疑似重复占位符: {sql}");
        assert!(!sql.contains("OR type NOT IN"), "{sql}");
    }

    /// 店铺账号指定了类型但筛选后为空集 ⇒ 恒假条件 `1 = 0`（Java 行为）。
    #[test]
    fn shop_scope_empty_types_forces_empty_page() {
        let q = query(serde_json::json!({"type": "0,1"}));
        let types = nextrade_core::models::member::feedback::resolve_shop_feedback_types(
            q.r#type.as_deref(),
        );
        assert!(types.is_empty());
        let sql = build(&q, &FeedbackListFilter::shop(types, 7, true));
        assert!(sql.contains(" AND 1 = 0"), "{sql}");
    }

    /// `keyword` 走 `LIKE %kw%`；纯空白视为空 ⇒ **不加**条件
    /// （Java `StrUtil.isNotBlank` 口径）。
    #[test]
    fn keyword_like_follows_is_blank() {
        let sql = build(
            &query(serde_json::json!({"keyword": "你好"})),
            &FeedbackListFilter::platform(vec![], None),
        );
        assert!(sql.contains("content LIKE "), "{sql}");

        let sql = build(
            &query(serde_json::json!({"keyword": "   "})),
            &FeedbackListFilter::platform(vec![], None),
        );
        assert!(!sql.contains("content LIKE"), "{sql}");
    }

    /// `isOrder` 的三档：`1` → `order_id = 0`；`>1` → `order_id > 0`；
    /// `0` / 负数 / 缺失 → 两个条件都不加。
    #[test]
    fn is_order_three_way() {
        let f = FeedbackListFilter::platform(vec![], None);
        let sql = build(&query(serde_json::json!({"isOrder": 1})), &f);
        assert!(sql.contains(" AND order_id = 0"), "{sql}");
        assert!(!sql.contains("order_id > 0"), "{sql}");

        let sql = build(&query(serde_json::json!({"isOrder": 2})), &f);
        assert!(sql.contains(" AND order_id > 0"), "{sql}");

        let sql = build(&query(serde_json::json!({"isOrder": 0})), &f);
        assert!(!sql.contains("order_id = 0"), "{sql}");
        assert!(!sql.contains("order_id > 0"), "{sql}");

        let sql = build(&query(serde_json::json!({})), &f);
        assert!(!sql.contains(" AND order_id"), "{sql}");
    }

    /// `productId = 0` / `status = 0` **要加**条件（Java 只判 null）；
    /// `userId = 0` **不加**（Java 要求 `> 0`）。
    #[test]
    fn zero_valued_filters_follow_java() {
        let f = FeedbackListFilter::platform(vec![], None);
        let sql = build(&query(serde_json::json!({"productId": 0, "status": 0})), &f);
        assert!(sql.contains("product_id = "), "{sql}");
        assert!(sql.contains("status = "), "{sql}");

        let sql = build(&query(serde_json::json!({"userId": 0})), &f);
        assert!(!sql.contains("user_id = "), "{sql}");

        let sql = build(&query(serde_json::json!({"userId": 9})), &f);
        assert!(sql.contains("user_id = "), "{sql}");
    }

    /// 平台账号的 `shopId` 请求参数为 `null` ⇒ 不加 `shop_id` 条件。
    #[test]
    fn platform_shop_id_is_optional() {
        let sql = build(
            &query(serde_json::json!({"shopId": 3})),
            &FeedbackListFilter::platform(vec![], Some(3)),
        );
        assert!(sql.contains("shop_id = "), "{sql}");

        let sql = build(
            &query(serde_json::json!({})),
            &FeedbackListFilter::platform(vec![], None),
        );
        // 只有平台可见性条件里出现 shop_id，没有独立的 `AND shop_id = `
        assert!(!sql.contains(" AND shop_id = "), "{sql}");
    }

    /// 补丁三态：只有 `Some(Some(_))` 算「会写」；空串与 0 **要写**。
    #[test]
    fn patch_three_state() {
        assert!(FeedbackReplyPatch::default().is_empty());

        let p = FeedbackReplyPatch {
            content: Some(Some(String::new())),
            ..FeedbackReplyPatch::default()
        };
        assert!(!p.is_empty(), "空串也要写列");

        let p = FeedbackReplyPatch {
            product_id: Some(Some(0)),
            ..FeedbackReplyPatch::default()
        };
        assert!(!p.is_empty(), "0 也要写列");

        let p = FeedbackReplyPatch {
            user_id: None,
            email: Some(None),
            ..FeedbackReplyPatch::default()
        };
        assert!(p.is_empty(), "None / Some(None) 都不写列");
    }

    /// 每一个字段都能独立触发「会写」（防 `is_empty` 漏字段的假绿）。
    #[test]
    fn patch_each_field_triggers_write() {
        let cases = [
            FeedbackReplyPatch {
                user_id: Some(Some(1)),
                ..Default::default()
            },
            FeedbackReplyPatch {
                username: Some(Some("a".into())),
                ..Default::default()
            },
            FeedbackReplyPatch {
                email: Some(Some("".into())),
                ..Default::default()
            },
            FeedbackReplyPatch {
                mobile: Some(Some("1".into())),
                ..Default::default()
            },
            FeedbackReplyPatch {
                content: Some(Some("c".into())),
                ..Default::default()
            },
            FeedbackReplyPatch {
                feedback_pics: Some(Some("[]".into())),
                ..Default::default()
            },
            FeedbackReplyPatch {
                r#type: Some(Some(0)),
                ..Default::default()
            },
            FeedbackReplyPatch {
                product_id: Some(Some(0)),
                ..Default::default()
            },
            FeedbackReplyPatch {
                order_id: Some(Some(0)),
                ..Default::default()
            },
            FeedbackReplyPatch {
                shop_id: Some(Some(0)),
                ..Default::default()
            },
        ];
        for (i, p) in cases.iter().enumerate() {
            assert!(!p.is_empty(), "第 {i} 个补丁应被判为「会写」");
        }
    }

    /// 「已回复」/「待回复」常量（写库时依赖它们）。
    #[test]
    fn status_constants_match_java_enum() {
        assert_eq!(feedback_status::WAIT_REPLY, 0);
        assert_eq!(feedback_status::REPLIED, 1);
        assert_eq!(feedback_status::CLOSED, 2);
        assert_eq!(SHOP_FEEDBACK_TYPES.len(), 1);
        assert_eq!(
            SHOP_FEEDBACK_TYPES[0],
            nextrade_core::models::member::feedback::feedback_type::PRODUCT
        );
    }

    /// `map_err`：行不存在 → `NotFound`，文案与 Java `"留言不存在"` 一致。
    #[test]
    fn map_err_maps_row_not_found_to_business_message() {
        let err = map_err(sqlx::Error::RowNotFound);
        match &err {
            Error::NotFound(msg) => assert_eq!(msg, MSG_FEEDBACK_NOT_FOUND),
            other => panic!("应映射为 NotFound，实际为 {other:?}"),
        }
        assert_eq!(err.business_code(), 404);
    }
}
