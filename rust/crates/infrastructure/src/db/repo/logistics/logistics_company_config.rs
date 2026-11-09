//! 物流公司配置域仓储（logistics::logistics_company_config）：`logistics_company_config` + `logistics_company`
//! 的只读关联查询。
//! 对应原 Java `mapper.setting.LogisticsCompanyConfigMapper` /
//! `mapper.setting.LogisticsCompanyMapper`（MyBatis-Plus `BaseMapper`，
//! 无自定义 XML，全部由 `LambdaQueryWrapper` 拼装）。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `logistics_company_config`：`config_id` / `logistics_id` / `shop_id` /
//!   `vendor_id` / `customer_name` / `customer_pwd` / `month_code` /
//!   `send_site` / `send_staff` / `exp_type` / `kuaidi100_partner_id` /
//!   `kuaidi100_partner_key` / `kuaidi100_partner_secret` / `kuaidi100_net` /
//!   `kuaidi100_check_man` / `kuaidi100_exp_type` / `kuaidi100_temp_id` /
//!   `kuaidi100_child_temp_id` / `kuaidi100_back_temp_id` / `sort_order` /
//!   `is_default` / `is_enabled`
//! - `logistics_company`：`logistics_id` / `logistics_code` /
//!   `logistics_name` / `logistics_desc` / `kuaidi100_code` /
//!   `kuaidi100_partner_id` / `kuaidi100_partner_key` / `kuaidi100_partner_secret` /
//!   `kuaidi100_net` / `kuaidi100_check_man` / `kuaidi100_exp_type` /
//!   `kuaidi100_temp_id` / `kuaidi100_child_temp_id` / `kuaidi100_back_temp_id`
//!
//! 全部查询使用运行时 `sqlx::query_as::<_, T>(...)`（**不用** `query!` 宏，
//! 本机无 `DATABASE_URL`）。
//!
//! ⚠️ **`update` 的动态 SET 与 SQL 注入**：Java `updateById(config)` 依据实体
//! 非 null 属性生成动态 SET 子句。Rust 侧同样需要「只更新显式传入列」的语义
//! （否则未传字段会被清空），因此 [`LogisticsCompanyConfigRepository::update_partial`] 接受
//! `(列名, 值)` 列表拼装 SQL。**列名必须来自
//! `nextrade_core::models::logistics::logistics_company_config::CONFIG_UPDATE_FIELDS` 之外的固定白名单**
//! （由服务层 `UPDATE_COLUMN_WHITELIST` 校验，见 `services::logistics_company_config`），
//! 仓储层用 `debug_assert!` 二次兜底，绝不接受外部原始输入。

use sqlx::PgPool;
use nextrade_core::entities::{LogisticsCompany, LogisticsCompanyConfig};
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

/// `update_partial` 允许写入的列白名单（与 `services::logistics_company_config` 的
/// `UPDATE_COLUMN_WHITELIST` 必须完全一致，两者由单测双向约束）。
///
/// 这些是 `LogisticsCompanyConfigUpdateDTO` 里除 `id` 外的全部可写列。
const WRITABLE_COLUMNS: [&str; 18] = [
    "logistics_id",
    "customer_name",
    "customer_pwd",
    "month_code",
    "send_site",
    "send_staff",
    "exp_type",
    "kuaidi100_partner_id",
    "kuaidi100_partner_key",
    "kuaidi100_partner_secret",
    "kuaidi100_net",
    "kuaidi100_check_man",
    "kuaidi100_exp_type",
    "kuaidi100_temp_id",
    "kuaidi100_child_temp_id",
    "kuaidi100_back_temp_id",
    "sort_order",
    "is_enabled",
];

/// 待写入的一列（列名 + 已按列类型转好的 SQL 字面量绑定值）。
///
/// 之所以不用 `sqlx::query_builder`：本 crate 未启用该特性，且 `PageQuery`
/// 等既有仓储一律手写占位符；此处沿用同样风格，仅把 SET 片段参数化。
#[derive(Debug, Clone)]
pub enum ColumnValue {
    /// 文本列（含可空文本：`None` 写成 `NULL`）
    Text(Option<String>),
    /// 整型列（含可空整型）
    Int(Option<i32>),
}

#[derive(Clone)]
pub struct LogisticsCompanyConfigRepository {
    pool: PgPool,
}

impl LogisticsCompanyConfigRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层在事务中复用）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 配置表 —————————————————————

    /// 按 ID 查询配置。
    pub async fn find_config_by_id(&self, config_id: i32) -> Result<Option<LogisticsCompanyConfig>> {
        sqlx::query_as::<_, LogisticsCompanyConfig>(
            "SELECT * FROM logistics_company_config WHERE config_id = $1 LIMIT 1",
        )
        .bind(config_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 配置分页。
    ///
    /// 对应 Java `LogisticsCompanyConfigServiceImpl#list`：
    /// 1. `shop_id` / `vendor_id` 等值过滤（Java 把 `null` 归一为 0）
    /// 2. `is_enabled` 可选等值过滤
    /// 3. `logistics_id IN (matchedIds)` —— 由关键词 / 服务商类型先在
    ///    `logistics_company` 上筛出的 ID 集合（见
    ///    [`LogisticsCompanyConfigRepository::find_company_ids`]）
    /// 4. `ORDER BY is_default DESC, is_enabled DESC`
    ///
    /// `matched_ids` 为 `Some(vec![])`（筛出空集）时由服务层短路返回空页，
    /// 不会走到这里；这里再兜一层返回 `(0, vec![])`，避免拼出 `IN ()` 语法错误。
    pub async fn paginate_configs(
        &self,
        shop_id: i32,
        vendor_id: i32,
        is_enabled: Option<i32>,
        matched_ids: Option<&[i32]>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<LogisticsCompanyConfig>)> {
        let (limit, offset) = page.limit_offset();

        if let Some(ids) = matched_ids {
            if ids.is_empty() {
                return Ok((0, Vec::new()));
            }
            let placeholders = vec!["?"; ids.len()].join(", ");
            let where_clause = format!(
                "shop_id = ? AND vendor_id = ? AND (? IS NULL OR is_enabled = ?) \
                 AND logistics_id IN ({placeholders})"
            );

            let count_sql = format!("SELECT COUNT(*) FROM logistics_company_config WHERE {where_clause}");
            let mut q = sqlx::query_as::<_, (i64,)>(&count_sql)
                .bind(shop_id)
                .bind(vendor_id)
                .bind(is_enabled)
                .bind(is_enabled);
            for id in ids {
                q = q.bind(*id);
            }
            let total: (i64,) = q.fetch_one(&self.pool).await.map_err(map_err)?;

            let list_sql = format!(
                "SELECT * FROM logistics_company_config WHERE {where_clause} \
                 ORDER BY is_default DESC, is_enabled DESC LIMIT ? OFFSET ?"
            );
            let mut q = sqlx::query_as::<_, LogisticsCompanyConfig>(&list_sql)
                .bind(shop_id)
                .bind(vendor_id)
                .bind(is_enabled)
                .bind(is_enabled);
            for id in ids {
                q = q.bind(*id);
            }
            let rows = q
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;

            return Ok((total.0.max(0) as u64, rows));
        }

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM logistics_company_config
             WHERE shop_id = ? AND vendor_id = ?
               AND (? IS NULL OR is_enabled = ?)",
        )
        .bind(shop_id)
        .bind(vendor_id)
        .bind(is_enabled)
        .bind(is_enabled)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        let rows = sqlx::query_as::<_, LogisticsCompanyConfig>(
            "SELECT * FROM logistics_company_config
             WHERE shop_id = ? AND vendor_id = ?
               AND (? IS NULL OR is_enabled = ?)
             ORDER BY is_default DESC, is_enabled DESC
             LIMIT ? OFFSET ?",
        )
        .bind(shop_id)
        .bind(vendor_id)
        .bind(is_enabled)
        .bind(is_enabled)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 同一租户下是否已绑定该物流公司（`create` 查重）。
    ///
    /// 对应 Java `create` 的 `count()` 判重。
    pub async fn count_binding(
        &self,
        shop_id: i32,
        vendor_id: i32,
        logistics_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM logistics_company_config
             WHERE shop_id = ? AND vendor_id = ? AND logistics_id = ?",
        )
        .bind(shop_id)
        .bind(vendor_id)
        .bind(logistics_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 同一租户下是否已绑定该物流公司（排除指定配置，\"update\" 改绑判重）。
    ///
    /// 对应 Java \"update\" 的 `... .ne(config_id, id).count()`。
    pub async fn count_binding_except(
        &self,
        shop_id: i32,
        vendor_id: i32,
        logistics_id: i32,
        exclude_config_id: i32,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM logistics_company_config
             WHERE shop_id = ? AND vendor_id = ? AND logistics_id = ?
               AND config_id <> ?",
        )
        .bind(shop_id)
        .bind(vendor_id)
        .bind(logistics_id)
        .bind(exclude_config_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 新增配置，返回 \"config_id\"（自增主键）。
    ///
    /// 与 Java `this.save(config)` 一致：\"is_default\" 固定写 0（Java \"create\"
    /// 显式 `config.setIsDefault(0)`），\"sort_order\" / \"is_enabled\" 的缺省值
    /// 由服务层填好后传入。
    pub async fn insert_config(&self, c: &LogisticsCompanyConfig) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO logistics_company_config
             (logistics_id, shop_id, vendor_id, customer_name, customer_pwd, month_code,
              send_site, send_staff, exp_type, kuaidi100_partner_id, kuaidi100_partner_key,
              kuaidi100_partner_secret, kuaidi100_net, kuaidi100_check_man, kuaidi100_exp_type,
              kuaidi100_temp_id, kuaidi100_child_temp_id, kuaidi100_back_temp_id,
              sort_order, is_default, is_enabled)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING config_id",
        )
        .bind(c.logistics_id)
        .bind(c.shop_id)
        .bind(c.vendor_id)
        .bind(&c.customer_name)
        .bind(&c.customer_pwd)
        .bind(&c.month_code)
        .bind(&c.send_site)
        .bind(&c.send_staff)
        .bind(&c.exp_type)
        .bind(&c.kuaidi100_partner_id)
        .bind(&c.kuaidi100_partner_key)
        .bind(&c.kuaidi100_partner_secret)
        .bind(&c.kuaidi100_net)
        .bind(&c.kuaidi100_check_man)
        .bind(&c.kuaidi100_exp_type)
        .bind(&c.kuaidi100_temp_id)
        .bind(&c.kuaidi100_child_temp_id)
        .bind(&c.kuaidi100_back_temp_id)
        .bind(c.sort_order)
        .bind(c.is_default)
        .bind(c.is_enabled)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 部分更新配置（只写显式传入的列）。
    ///
    /// 对应 Java `updateById(config)`：`BeanUtils.copyProperties` 后实体上为
    /// \"null\" 的属性**不参与** SET 子句。
    ///
    /// **安全约束**：\"columns\" 里的列名会直接拼进 SQL，调用方必须保证其来自
    /// 固定白名单（服务层已校验）；此处用 `debug_assert!` 兜底并在 release
    /// 下返回明确错误，绝不接受任意外部字符串。
    pub async fn update_partial(
        &self,
        config_id: i32,
        columns: &[(&'static str, ColumnValue)],
    ) -> Result<u64> {
        if columns.is_empty() {
            return Ok(0);
        }

        let mut set_parts: Vec<String> = Vec::with_capacity(columns.len());
        for (col, _) in columns {
            if !WRITABLE_COLUMNS.contains(col) {
                return Err(Error::Internal(format!(
                    "update_partial 列名未在白名单内: {col}"
                )));
            }
            set_parts.push(format!("{col} = ?"));
        }
        let sql = format!(
            "UPDATE logistics_company_config SET {} WHERE config_id = ?",
            set_parts.join(", ")
        );

        let mut q = sqlx::query(&sql);
        for (_, val) in columns {
            q = match val {
                ColumnValue::Text(v) => q.bind(v),
                ColumnValue::Int(v) => q.bind(v),
            };
        }
        let affected = q
            .bind(config_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 按 ID 删除配置（Java `BaseServiceImpl#del` → \"removeById\"）。
    pub async fn delete_config(&self, config_id: i32) -> Result<u64> {
        let affected = sqlx::query("DELETE FROM logistics_company_config WHERE config_id = ?")
            .bind(config_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// 把某租户下**全部**配置置为非默认。
    ///
    /// 对应 Java \"setDefault\" 的第一步：按 \"shop_id\" + \"vendor_id\" 全量
    /// `set(is_default, 0)`。
    pub async fn clear_default(&self, shop_id: i32, vendor_id: i32) -> Result<u64> {
        let affected = sqlx::query(
            "UPDATE logistics_company_config SET is_default = 0
             WHERE shop_id = ? AND vendor_id = ?",
        )
        .bind(shop_id)
        .bind(vendor_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected)
    }

    /// 把指定配置置为默认。
    ///
    /// 对应 Java \"setDefault\" 的第二步：按主键 `set(is_default, 1)`。
    pub async fn mark_default(&self, config_id: i32) -> Result<u64> {
        let affected = sqlx::query("UPDATE logistics_company_config SET is_default = 1 WHERE config_id = ?")
            .bind(config_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    /// \"setDefault\" 的原子事务封装：先全租户清零，再把目标置 1。
    ///
    /// 对应 Java \"setDefault\" 上的 `@Transactional(rollbackFor = Exception.class)`。
    /// 用 \"BEGIN\" / \"COMMIT\" 显式包裹，保证两条 UPDATE 要么都生效要么都回滚
    /// （否则中断会导致该租户「无默认物流公司」）。
    pub async fn set_default_tx(&self, config_id: i32, shop_id: i32, vendor_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query(
            "UPDATE logistics_company_config SET is_default = 0
             WHERE shop_id = ? AND vendor_id = ?",
        )
        .bind(shop_id)
        .bind(vendor_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        sqlx::query("UPDATE logistics_company_config SET is_default = 1 WHERE config_id = ?")
            .bind(config_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 通用字段更新（\"updateField\"）。
    ///
    /// 对应 Java `BaseServiceImpl#updateField`：`eq(主键).set(column, val)`。
    /// \"column\" 必须由服务层经白名单映射得到（见 `services::nextrade_core::models::logistics::logistics_company_config::
    /// map_update_field_column`），此处再次校验。
    pub async fn update_field(&self, id: i32, column: &'static str, val: &str) -> Result<u64> {
        if !CONFIG_FIELD_COLUMNS.contains(&column) {
            return Err(Error::Internal(format!(
                "update_field 列名未经过白名单映射: {column}"
            )));
        }
        let sql = format!("UPDATE logistics_company_config SET {column} = ? WHERE config_id = ?");
        let affected = sqlx::query(&sql)
            .bind(val)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected();
        Ok(affected)
    }

    // ————————————————————— 物流公司表（关联） —————————————————————

    /// 按 ID 查询物流公司。
    ///
    /// 对应 Java `logisticsCompanyService.getById(logisticsId)`。
    pub async fn find_company_by_id(&self, logistics_id: i32) -> Result<Option<LogisticsCompany>> {
        sqlx::query_as::<_, LogisticsCompany>(
            "SELECT * FROM logistics_company WHERE logistics_id = ? LIMIT 1",
        )
        .bind(logistics_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 ID 集合批量查询物流公司（列表关联回填，替代逐条 `getById`）。
    ///
    /// 对应 Java `logisticsCompanyService.listByIds(logisticsIds)` +
    /// `Collectors.toMap(...)`。空集合直接返回空 `Vec`，避免 `IN ()`。
    pub async fn find_companies_by_ids(&self, ids: &[i32]) -> Result<Vec<LogisticsCompany>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(", ");
        let sql = format!("SELECT * FROM logistics_company WHERE logistics_id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, LogisticsCompany>(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按关键词 / 服务商类型筛选出物流公司 ID 集合。
    ///
    /// 对应 Java `list` 中在 `logistics_company` 上执行的子查询：
    /// - `keyword` 非空 → `logistics_name LIKE %keyword%`
    /// - `provider_type == 1`（快递鸟）→ `logistics_code` 非空且 `TRIM <> ''`
    /// - `provider_type == 2`（快递100）→ `kuaidi100_code` 非空且 `TRIM <> ''`
    ///   **或** `UPPER(TRIM(logistics_code))` 命中内置映射表
    ///   **或** `logistics_code` 全为小写字母数字
    ///
    /// ⚠️ 第三条分支中的「内置映射表」（Java `Kuaidi100CodeMapping.
    /// mappedLogisticsCodes()`）是 Java 侧硬编码的编码清单，Rust 侧**未迁移**
    /// 该清单，故本方法只实现了 `kuaidi100_code` 与「全小写字母数字」两条
    /// 判据；这可能让少数「`kuaidi100_code` 为空但命中内置映射」的物流公司
    /// 在 `providerType=2` 下被漏掉。详见 `uncertain_points`。
    pub async fn find_company_ids(
        &self,
        keyword: Option<&str>,
        provider_type: Option<i32>,
    ) -> Result<Vec<i32>> {
        let like = keyword.map(|k| format!("%{k}%"));
        let pt = provider_type;
        let rows: Vec<(i32,)> = sqlx::query_as(
            "SELECT logistics_id FROM logistics_company
             WHERE (? IS NULL OR logistics_name LIKE ?)
               AND (
                 ? IS NULL
                 OR (? = 1 AND logistics_code IS NOT NULL AND TRIM(logistics_code) <> '')
                 OR (? = 2 AND (
                      (kuaidi100_code IS NOT NULL AND TRIM(kuaidi100_code) <> '')
                      OR (logistics_code IS NOT NULL
                          AND TRIM(logistics_code) COLLATE utf8mb4_bin REGEXP '^[a-z0-9]+$')
                    ))
               )
             ORDER BY logistics_id ASC",
        )
        .bind(&like)
        .bind(&like)
        .bind(pt)
        .bind(pt)
        .bind(pt)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
}

/// \"updateField\" 允许的列（Java `LogisticsCompanyConfigController.CONFIG_UPDATE_FIELDS`）。
///
/// 与 `nextrade_core::models::logistics::logistics_company_config::CONFIG_UPDATE_FIELDS` 是同一份语义，
/// 此处是本 crate 内的镜像常量（\"models\" 常量是 `&str` 数组，不能直接当
/// `&'static str` 单值比较用），两者由单测约束一致。
pub const CONFIG_FIELD_COLUMNS: [&str; 2] = ["is_enabled", "sort_order"];

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("物流公司配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writable_columns_cover_update_dto_fields() {
        // 18 个可写列 + config_id 主键 == 19；DTO 里 id 之外共 18 个字段
        assert_eq!(WRITABLE_COLUMNS.len(), 18);
        // 主键与租户列不允许被 update 覆盖
        assert!(!WRITABLE_COLUMNS.contains(&"config_id"));
        assert!(!WRITABLE_COLUMNS.contains(&"shop_id"));
        assert!(!WRITABLE_COLUMNS.contains(&"vendor_id"));
        assert!(!WRITABLE_COLUMNS.contains(&"is_default"));
    }

    #[test]
    fn writable_columns_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for c in WRITABLE_COLUMNS {
            assert!(seen.insert(c), "重复列名: {c}");
        }
    }

    #[test]
    fn config_field_columns_match_model_constant() {
        assert_eq!(
            CONFIG_FIELD_COLUMNS,
            nextrade_core::models::logistics::logistics_company_config::CONFIG_UPDATE_FIELDS
        );
    }

    #[test]
    fn writable_columns_include_update_field_columns() {
        // updateField 允许的两列必须也在可写列里
        for c in CONFIG_FIELD_COLUMNS {
            assert!(WRITABLE_COLUMNS.contains(&c), "缺少列: {c}");
        }
    }
}