//! 打印机管理域（logistics::print）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.print.PrintMapper`（`BaseMapper<Print>`）
//! - `com.nextrade.mapper.print.PrintPosDeviceMapper`
//! - `com.nextrade.mapper.print.PrintConfigMapper`（本片只读 `paper_size`、
//!   写「初始化默认配置」与「同步纸张尺寸」）
//! - `com.nextrade.service.print.impl.PrintServiceImpl`（`list` / `detail` /
//!   `create` / `update` / `del` 涉及的查询与写入）
//! - `com.nextrade.service.print.impl.PrintRoutingServiceImpl#resolve`
//! - `com.nextrade.service.print.impl.PrintPosDeviceServiceImpl#replace` /
//!   `#bindingsByPrintIds`
//! - `com.nextrade.service.print.impl.PrintConfigServiceImpl#initConfig` /
//!   `#updatePaperSizeByPrintId`
//! - `com.nextrade.service.common.impl.BaseServiceImpl#updateField`
//!
//! # 列名对齐（权威来源）
//!
//! 全部列名来自 `java/database/pure.sql`，用
//! `cd rust && python tools/schema_parser.py --table <表名>` 核对：
//!
//! - `print`（17 列，主键 `print_id`，见 [`PRINT_SELECT`]）
//! - `print_pos_device`（5 列）：`id` / `shop_id` / `print_id` /
//!   `device_code` / `add_time`
//! - `print_config`（7 列）：本片只碰 `print_id` / `template` / `type` /
//!   `paper_size` / `add_time` / `update_time`
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query`
//! （**不使用** `query!` 宏：本机无 `DATABASE_URL`）。
//!
//! # 删除语义
//!
//! Java 实体 `Print` / `PrintLog` **没有** `@TableLogic`（`delete_time`
//! 只是普通列），因此 `removeById` 是**物理 DELETE**。本仓储同样物理删除。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use sqlx::PgPool;

use nextrade_core::entities::Print;
use nextrade_core::models::logistics::print::{
    to_snake, MSG_PRINTER_NOT_FOUND, PRINT_FIELDS, STATUS_ENABLED,
};
use nextrade_core::models::logistics::print_config::PRINT_COLUMNS;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `print` 的固定 SELECT 列清单（17 列，顺序与 `entities::Print` 字段一致，
/// 避免 `SELECT *` 与实体字段顺序耦合）。
const PRINT_SELECT: &str = "SELECT print_id, print_name, print_sn, print_key, third_account, third_key, print_number, platform, shop_id, status, add_time, update_time, delete_time, auto_print, voice_enabled, voice_content_type, is_default FROM print";

/// `print_pos_device` 的固定 SELECT 列清单（5 列）。
const PRINT_POS_DEVICE_SELECT: &str =
    "SELECT id, shop_id, print_id, device_code, add_time FROM print_pos_device";

/// 列表查询过滤条件（`None` 表示不过滤）。
///
/// 由服务层从
/// [`nextrade_core::models::logistics::print::PrintPageQuery`] 归一后传入：
/// LIKE 通配符已在服务层/本仓储内部拼好，SQL 只负责拼接与绑定。
#[derive(Debug, Clone, Default)]
pub struct PrintFilters<'a> {
    /// 打印机名称模糊（`print_name LIKE ?`）
    pub print_name: Option<&'a str>,
    /// 打印机 SN 模糊（`print_sn LIKE ?`）
    pub print_sn: Option<&'a str>,
    /// 状态精确（`status = ?`）
    pub status: Option<i32>,
    /// 平台精确（`platform = ?`）
    pub platform: Option<i32>,
    /// 收银台设备码（子查询 `print_pos_device`）
    pub device_code: Option<&'a str>,
    /// 关键词（`print_name LIKE ? OR print_sn LIKE ?`）
    pub keyword: Option<&'a str>,
}

/// 仓储（持有连接池）。
#[derive(Clone)]
pub struct PrintRepository {
    pool: PgPool,
}

impl PrintRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 查询 —————————————————————

    /// 按主键 + 店铺查询打印机
    /// （对应 Java `PrintServiceImpl#detail/del/update` 的
    /// `lambdaQuery().eq(printId).eq(shopId).one()`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_by_id_and_shop(&self, print_id: i32, shop_id: i32) -> Result<Option<Print>> {
        let sql = format!("{PRINT_SELECT} WHERE print_id = $1 AND shop_id = $2 LIMIT 1");
        sqlx::query_as::<_, Print>(&sql)
            .bind(print_id)
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 打印机分页（管理端列表）。
    ///
    /// 对应 Java `PrintServiceImpl#list`：`WHERE shop_id = ?` 恒成立，
    /// 其余条件按 `PrintPageQuery` 可选叠加；排序由
    /// [`resolve_sort_column`] + [`resolve_sort_direction`] 决定
    /// （Java `BaseServiceImpl#buildSortOrder`）。
    ///
    /// `device_code` 条件复刻 Java 的 `inSql`：
    /// `print_id IN (SELECT print_id FROM print_pos_device
    ///               WHERE shop_id = ? AND device_code = ?)`
    /// —— Java 用字符串拼接（`deviceCode.replace("'", "''")` 防注入），
    /// 本实现改用绑定参数，语义等价且更安全。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn paginate(
        &self,
        shop_id: i32,
        filters: &PrintFilters<'_>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Print>)> {
        let name = like_pattern(filters.print_name);
        let sn = like_pattern(filters.print_sn);
        let keyword = like_pattern(filters.keyword);
        let device_code = filters
            .device_code
            .map(str::trim)
            .filter(|c| !c.is_empty());

        let mut where_parts: Vec<String> = vec!["shop_id = ?".into()];
        if name.is_some() {
            where_parts.push("print_name LIKE ?".into());
        }
        if sn.is_some() {
            where_parts.push("print_sn LIKE ?".into());
        }
        if filters.status.is_some() {
            where_parts.push("status = ?".into());
        }
        if filters.platform.is_some() {
            where_parts.push("platform = ?".into());
        }
        if device_code.is_some() {
            where_parts.push(
                "print_id IN (SELECT print_id FROM print_pos_device WHERE shop_id = $1 AND device_code = $2)"
                    .into(),
            );
        }
        if keyword.is_some() {
            where_parts.push("(print_name LIKE ? OR print_sn LIKE ?)".into());
        }
        let where_sql = format!("WHERE {}", where_parts.join(" AND "));

        let column = resolve_sort_column(sort_field);
        let dir = resolve_sort_direction(sort_order);
        let (limit, offset) = page.limit_offset();

        let count_sql = format!("SELECT COUNT(*) FROM print {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        cq = bind_filters(
            cq,
            shop_id,
            name.as_deref(),
            sn.as_deref(),
            filters.status,
            filters.platform,
            device_code,
            keyword.as_deref(),
        );
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let list_sql = format!(
            "{PRINT_SELECT} {where_sql} ORDER BY {column} {dir} LIMIT ? OFFSET ?"
        );
        let mut lq = sqlx::query_as::<_, Print>(&list_sql);
        lq = bind_filters(
            lq,
            shop_id,
            name.as_deref(),
            sn.as_deref(),
            filters.status,
            filters.platform,
            device_code,
            keyword.as_deref(),
        );
        let rows = lq
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量读取打印机的纸张尺寸（`print_config.paper_size`）。
    ///
    /// 对应 Java `PrintServiceImpl#loadPaperByPrintIds`：
    /// `select print_id, paper_size from print_config where print_id in (...)`。
    /// Java 用 `Collectors.toMap(..., (existing, replacement) -> existing)`
    /// 保留**先出现**的一条；本方法返回原始行序，由服务层做同样的去重。
    ///
    /// `print_ids` 为空时**不发 SQL**，直接返回空列表（与 Java
    /// `if (printIds == null || printIds.isEmpty()) return Map.of();` 一致）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_paper_sizes(&self, print_ids: &[i32]) -> Result<Vec<(i32, i32)>> {
        if print_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; print_ids.len()].join(", ");
        let sql = format!(
            "SELECT print_id, paper_size FROM print_config WHERE print_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (i32, i32)>(&sql);
        for id in print_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量读取打印机的收银台绑定
    /// （对应 Java `PrintPosDeviceServiceImpl#bindingsByPrintIds` 的
    /// `eq(shopId).in(printId, printIds)`）。
    ///
    /// Java 用 `putIfAbsent` 语义「同一打印机多条绑定取先出现的一条」；
    /// 本方法返回原始行序，由服务层去重。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_bindings(
        &self,
        shop_id: i32,
        print_ids: &[i32],
    ) -> Result<Vec<(i32, String)>> {
        if print_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; print_ids.len()].join(", ");
        let sql = format!(
            "SELECT print_id, device_code FROM print_pos_device WHERE shop_id = $1 AND print_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (i32, String)>(&sql).bind(shop_id);
        for id in print_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 某店铺全部**启用**的打印机
    /// （对应 Java `PrintRoutingServiceImpl#resolve` 的
    /// `eq(shopId).eq(status, ENABLED).orderByDesc(isDefault).orderByAsc(printId)`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_enabled(&self, shop_id: i32) -> Result<Vec<Print>> {
        let sql = format!(
            "{PRINT_SELECT} WHERE shop_id = $1 AND status = $2 ORDER BY is_default DESC, print_id ASC"
        );
        sqlx::query_as::<_, Print>(&sql)
            .bind(shop_id)
            .bind(STATUS_ENABLED)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 该店铺是否存在启用的打印机
    /// （对应 Java `PrintServiceImpl#hasEnabled` 的
    /// `.eq(shopId).eq(status, ENABLED).exists()`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn has_enabled(&self, shop_id: i32) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT print_id FROM print WHERE shop_id = $1 AND status = $2 LIMIT 1",
        )
        .bind(shop_id)
        .bind(STATUS_ENABLED)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    // ————————————————————— 写操作 —————————————————————

    /// 新增打印机，返回自增主键
    /// （对应 Java `PrintServiceImpl#create` 的 `save(print)`）。
    ///
    /// Java 走 MyBatis-Plus `save`（`FieldStrategy.NOT_NULL`），`null` 列由
    /// DDL 默认值兜底；Rust 显式列出全部 16 个可写列，因此**默认值必须由
    /// 服务层按 Java 语义补齐**（`status` / `voice_*` / `is_default` /
    /// `print_number` 等），见
    /// [`nextrade_core::models::logistics::print`] 的 `*_or_default` 系列函数。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert(&self, p: &Print) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO print (print_name, print_sn, print_key, third_account, third_key, print_number, platform, shop_id, status, add_time, update_time, delete_time, auto_print, voice_enabled, voice_content_type, is_default) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING print_id",
        )
        .bind(&p.print_name)
        .bind(&p.print_sn)
        .bind(&p.print_key)
        .bind(&p.third_account)
        .bind(&p.third_key)
        .bind(p.print_number)
        .bind(p.platform)
        .bind(p.shop_id)
        .bind(p.status)
        .bind(p.add_time)
        .bind(p.update_time)
        .bind(p.delete_time)
        .bind(p.auto_print)
        .bind(p.voice_enabled)
        .bind(p.voice_content_type)
        .bind(p.is_default)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 整行更新打印机（主键之外的全部 16 列）
    /// （对应 Java `PrintServiceImpl#update` 的 `updateById(updatePrint)`）。
    ///
    /// Java `updateById` 走 `FieldStrategy.NOT_NULL`：`null` 列不进入 SET
    /// 子句。Rust 侧由服务层先读出旧行、把「Java 不会改」的列填成旧值，
    /// 因此本方法可以安全地整行覆盖。
    ///
    /// 返回是否命中记录（Java `updateById` 的布尔值）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update(&self, p: &Print) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE print SET print_name = $1, print_sn = $2, print_key = $3, third_account = $4, third_key = $5, print_number = $6, platform = $7, shop_id = $8, status = $9, add_time = $10, update_time = $11, delete_time = $12, auto_print = $13, voice_enabled = $14, voice_content_type = $15, is_default = $16 WHERE print_id = $17",
        )
        .bind(&p.print_name)
        .bind(&p.print_sn)
        .bind(&p.print_key)
        .bind(&p.third_account)
        .bind(&p.third_key)
        .bind(p.print_number)
        .bind(p.platform)
        .bind(p.shop_id)
        .bind(p.status)
        .bind(p.add_time)
        .bind(p.update_time)
        .bind(p.delete_time)
        .bind(p.auto_print)
        .bind(p.voice_enabled)
        .bind(p.voice_content_type)
        .bind(p.is_default)
        .bind(p.print_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 物理删除打印机
    /// （对应 Java `PrintServiceImpl#del` 的 `this.removeById(id)`）。
    ///
    /// `Print` 无 `@TableLogic`，故是 DELETE 而非软删。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete(&self, print_id: i32) -> Result<bool> {
        let res = sqlx::query("DELETE FROM print WHERE print_id = $1")
            .bind(print_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 更新单个字段
    /// （对应 Java `BaseServiceImpl#updateField` 的
    /// `.eq(keyColumn, id).set(field, val)`）。
    ///
    /// ⚠️ Java 该通用方法**不**按 `shop_id` 过滤；本实现保持一致（调用方
    /// 已做存在性/归属校验，见服务层文档）。
    ///
    /// `field` 必须已由服务层按
    /// [`nextrade_core::models::logistics::print::PRINT_FIELDS`] 归一为
    /// 数据库列名；这里再校验一次，防止非法列名进入 SQL。
    ///
    /// # Errors
    /// 字段不在白名单 → `Error::Validation(field 的值有误)`；数据库失败 →
    /// `Error::Database`。
    pub async fn update_field(&self, print_id: i32, field: &str, val: &str) -> Result<bool> {
        if !PRINT_FIELDS.contains(&field) {
            return Err(Error::Validation(
                nextrade_core::models::logistics::print::MSG_INVALID_FIELD_VALUE.into(),
            ));
        }
        let sql = format!("UPDATE print SET \"{field}\" = $1 WHERE print_id = $2");
        let res = sqlx::query(&sql)
            .bind(val)
            .bind(print_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 把该店铺其它打印机的 `is_default` 置 0
    /// （对应 Java `PrintServiceImpl#clearOtherDefault`）。
    ///
    /// Java：
    /// ```text
    /// lambdaUpdate().set(isDefault, 0)
    ///     .eq(shopId, getShopId())
    ///     .ne(currentPrintId != null, printId, currentPrintId)
    ///     .update();
    /// ```
    /// 即 `current_print_id = None`（新增场景）时**不加** `print_id <> ?`，
    /// 会把全店打印机都置 0。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn clear_other_default(
        &self,
        shop_id: i32,
        current_print_id: Option<i32>,
    ) -> Result<u64> {
        let res = match current_print_id {
            Some(id) => {
                sqlx::query("UPDATE print SET is_default = 0 WHERE shop_id = $1 AND print_id <> $2")
                    .bind(shop_id)
                    .bind(id)
                    .execute(&self.pool)
                    .await
            }
            None => {
                sqlx::query("UPDATE print SET is_default = 0 WHERE shop_id = $1")
                    .bind(shop_id)
                    .execute(&self.pool)
                    .await
            }
        }
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— print_config（初始化 / 纸张尺寸） —————————————————————

    /// 统计某打印机的配置条数
    /// （对应 Java `PrintConfigServiceImpl#initConfig` 的
    /// `lambdaQuery().eq(printId).count()`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn count_configs(&self, print_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM print_config WHERE print_id = $1")
                .bind(print_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0)
    }

    /// 初始化默认购物小票配置（对应 Java `PrintConfigServiceImpl#initConfig`）。
    ///
    /// Java 写入：`print_id` / `type = RECEIPT(1)` /
    /// `template = PrintTemplateConstant.DEFAULT_RECEIPT_TEMPLATE` /
    /// `paper_size`（缺省 58）/ `add_time = update_time = 当前秒`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn insert_default_config(
        &self,
        print_id: i32,
        template_json: &str,
        paper_size: i32,
        now_secs: i32,
        config_type: i32,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO print_config (print_id, template, type, paper_size, add_time, update_time) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(print_id)
        .bind(template_json)
        .bind(config_type)
        .bind(paper_size)
        .bind(now_secs)
        .bind(now_secs)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 按打印机 ID 同步纸张尺寸
    /// （对应 Java `PrintConfigServiceImpl#updatePaperSizeByPrintId`：
    /// `UPDATE print_config SET paper_size = ?, update_time = ? WHERE print_id = ?`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn update_paper_size(
        &self,
        print_id: i32,
        paper_size: i32,
        now_secs: i32,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE print_config SET paper_size = $1, update_time = $2 WHERE print_id = $3",
        )
        .bind(paper_size)
        .bind(now_secs)
        .bind(print_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 删除某打印机的全部配置
    /// （对应 Java `PrintServiceImpl#del` 的
    /// `printConfigMapper.delete(eq(printId))`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete_configs(&self, print_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM print_config WHERE print_id = $1")
            .bind(print_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    // ————————————————————— print_pos_device（设备绑定） —————————————————————

    /// 重置打印机与收银台设备的绑定
    /// （对应 Java `PrintPosDeviceServiceImpl#replace`）。
    ///
    /// Java 语义：先 `DELETE WHERE shop_id AND print_id`，再在
    /// `deviceCode` 非空白时 `INSERT` 一条。
    ///
    /// ⚠️ **与 Java 的一处刻意差异**：Java 当前实现里的
    /// `boolean enabled = false; if (!enabled) throw new GlobalException(
    /// "设备码不存在或已停用: " + code)` 是**占位桩**（设备主数据表未迁移），
    /// 导致 `create`/`update` 只要带 `deviceCode` 就必然抛异常回滚。
    /// 本实现按注释表达的**真实意图**落库（校验通过 → 写绑定），
    /// 不复制该占位桩，见 `uncertain_points`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn replace_binding(
        &self,
        shop_id: i32,
        print_id: i32,
        device_code: Option<&str>,
        now_secs: i64,
    ) -> Result<()> {
        sqlx::query("DELETE FROM print_pos_device WHERE shop_id = $1 AND print_id = $2")
            .bind(shop_id)
            .bind(print_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        let code = device_code.map(str::trim).filter(|c| !c.is_empty());
        if let Some(code) = code {
            sqlx::query(
                "INSERT INTO print_pos_device (shop_id, print_id, device_code, add_time) VALUES ($1, $2, $3, $4)",
            )
            .bind(shop_id)
            .bind(print_id)
            .bind(code)
            .bind(now_secs)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        }
        Ok(())
    }

    /// 删除打印机的全部设备绑定
    /// （`replace(shopId, printId, null)` 的等价物，`del` 复用）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn delete_bindings(&self, shop_id: i32, print_id: i32) -> Result<u64> {
        let res = sqlx::query("DELETE FROM print_pos_device WHERE shop_id = $1 AND print_id = $2")
            .bind(shop_id)
            .bind(print_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(res.rows_affected())
    }

    /// 批量读取收银台绑定（强类型行，供需要 `PrintPosDevice` 实体的调用方）。
    ///
    /// 与 [`Self::find_bindings`] 同一条 SQL，区别是返回实体而非元组；
    /// 保留它是因为 `entities::PrintPosDevice` 已生成，避免将来再写一次 SQL。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_binding_rows(
        &self,
        shop_id: i32,
        print_ids: &[i32],
    ) -> Result<Vec<nextrade_core::entities::PrintPosDevice>> {
        if print_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; print_ids.len()].join(", ");
        let sql = format!(
            "{PRINT_POS_DEVICE_SELECT} WHERE shop_id = $1 AND print_id IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, nextrade_core::entities::PrintPosDevice>(&sql).bind(shop_id);
        for id in print_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }
}

/// LIKE 通配模式（Java `queryWrapper.like(column, value)` 等价物）。
///
/// Java 的 `like` 是 `value → %value%`；`isNotEmpty` 只排除 `null` 与空串。
/// `None` / 空串 → `None`（不加条件）。
#[must_use]
pub fn like_pattern(raw: Option<&str>) -> Option<String> {
    raw.filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"))
}

/// 排序字段白名单解析（防 SQL 注入）。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName`：
/// 属性名或列名命中实体列 → 该列；否则回退主键 `print_id`。
/// 因此白名单就是 `print` 表的全部列（[`PRINT_COLUMNS`]）。
#[must_use]
pub fn resolve_sort_column(field: Option<&str>) -> &'static str {
    match field {
        Some(f) => {
            let snake = to_snake(f);
            PRINT_COLUMNS
                .iter()
                .find(|c| **c == snake)
                .copied()
                .unwrap_or("print_id")
        }
        None => "print_id",
    }
}

/// 排序方向（Java `buildSortOrder`）：仅显式 `asc` 升序，`desc` / 空一律降序。
#[must_use]
pub fn resolve_sort_direction(order: Option<&str>) -> &'static str {
    if order.is_some_and(|o| o.eq_ignore_ascii_case("asc")) {
        "ASC"
    } else {
        "DESC"
    }
}

/// 把 WHERE 片段按 [`PrintRepository::paginate`] 的拼接顺序绑定到查询上。
///
/// 顺序：`shop_id` → `print_name` → `print_sn` → `status` → `platform`
/// → `device_code`（两条）→ `keyword`（两条，同一模式绑定两次）。
#[allow(clippy::too_many_arguments)]
fn bind_filters<'q, O>(
    q: sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    shop_id: i32,
    name: Option<&'q str>,
    sn: Option<&'q str>,
    status: Option<i32>,
    platform: Option<i32>,
    device_code: Option<&'q str>,
    keyword: Option<&'q str>,
) -> sqlx::query::QueryAs<'q, sqlx::Postgres, O, sqlx::postgres::PgArguments> {
    let mut q = q.bind(shop_id);
    if let Some(v) = name {
        q = q.bind(v);
    }
    if let Some(v) = sn {
        q = q.bind(v);
    }
    if let Some(v) = status {
        q = q.bind(v);
    }
    if let Some(v) = platform {
        q = q.bind(v);
    }
    if let Some(v) = device_code {
        // 子查询同时用到 shop_id 与 device_code
        q = q.bind(shop_id).bind(v);
    }
    if let Some(v) = keyword {
        q = q.bind(v).bind(v);
    }
    q
}

/// 数据库错误映射（与 brand / member_tag / print_config 同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_PRINTER_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::logistics::print::MSG_INVALID_FIELD_VALUE;
    use nextrade_core::models::logistics::print_config::PRINT_POS_DEVICE_COLUMNS;

    /// `PRINT_SELECT` 覆盖 `print` 全部 17 列，顺序与实体一致，且不用 `SELECT *`。
    #[test]
    fn print_select_covers_all_columns_in_entity_order() {
        let listed: Vec<&str> = PRINT_SELECT
            .trim_start_matches("SELECT ")
            .split(" FROM ")
            .next()
            .unwrap()
            .split(", ")
            .collect();
        assert_eq!(listed, PRINT_COLUMNS.to_vec());
        assert_eq!(listed.len(), 17);
        assert!(!PRINT_SELECT.contains('*'));
    }

    /// 设备绑定 SELECT 覆盖 5 列。
    #[test]
    fn print_pos_device_select_covers_all_columns() {
        for column in PRINT_POS_DEVICE_COLUMNS {
            assert!(
                PRINT_POS_DEVICE_SELECT.contains(column),
                "SELECT 清单缺少列 {column}"
            );
        }
        assert!(PRINT_POS_DEVICE_SELECT.contains("FROM print_pos_device"));
        assert!(!PRINT_POS_DEVICE_SELECT.contains('*'));
    }

    /// 排序字段白名单：属性名 / 列名都认，未知回退主键（防注入）。
    #[test]
    fn sort_column_whitelist_and_fallback() {
        assert_eq!(resolve_sort_column(None), "print_id");
        assert_eq!(resolve_sort_column(Some("printId")), "print_id");
        assert_eq!(resolve_sort_column(Some("print_id")), "print_id");
        assert_eq!(resolve_sort_column(Some("printName")), "print_name");
        assert_eq!(resolve_sort_column(Some("isDefault")), "is_default");
        assert_eq!(resolve_sort_column(Some("addTime")), "add_time");
        // 未知字段回退主键，防 SQL 注入
        assert_eq!(
            resolve_sort_column(Some("1;DROP TABLE print")),
            "print_id"
        );
    }

    /// 排序方向：仅 `asc` 升序，其余降序。
    #[test]
    fn sort_direction_defaults_to_desc() {
        assert_eq!(resolve_sort_direction(Some("asc")), "ASC");
        assert_eq!(resolve_sort_direction(Some("ASC")), "ASC");
        assert_eq!(resolve_sort_direction(Some("desc")), "DESC");
        assert_eq!(resolve_sort_direction(None), "DESC");
    }

    /// LIKE 模式：`None` / 空串不加条件，其余包 `%`。
    #[test]
    fn like_pattern_matches_java_like() {
        assert_eq!(like_pattern(None), None);
        assert_eq!(like_pattern(Some("")), None);
        assert_eq!(like_pattern(Some("前台")).as_deref(), Some("%前台%"));
        // Java `isNotEmpty` 不 trim：空白串仍会生成 LIKE
        assert_eq!(like_pattern(Some(" ")).as_deref(), Some("% %"));
    }

    /// 驼峰转换幂等（列名原样传入也命中白名单）。
    #[test]
    fn snake_conversion_is_idempotent_for_snake_input() {
        assert_eq!(to_snake("print_name"), "print_name");
        assert_eq!(to_snake("printName"), "print_name");
    }

    /// `update_field` 的白名单与 Java `PRINT_FIELDS` 一致（列名级）。
    #[test]
    fn update_field_whitelist_is_status_and_is_default() {
        assert_eq!(PRINT_FIELDS, ["status", "is_default"]);
        assert_eq!(MSG_INVALID_FIELD_VALUE, "field 的值有误");
    }
}
