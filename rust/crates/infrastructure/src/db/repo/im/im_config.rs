//! IM 配置域（im::im_config）仓储：`im_config`。
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`python tools/schema_parser.py --table im_config` 可复核）：
//! - `im_config`：`id`（int unsigned，主键）/ `code`（varchar(255)）/ `data`
//!   （text，可空）/ `shop_id`（int unsigned）
//!
//! 查询统一使用运行时 `sqlx::query_as::<_, T>(...)`（**不使用 `query!` 宏**，
//! 本机无 `DATABASE_URL`）。动态 WHERE 片段只拼接固定字面量，条件值一律
//! `?` 占位绑定，不做字符串插值。

use sqlx::PgPool;
use nextrade_core::entities::ImConfig;
use nextrade_core::{Error, Result};

/// `im_config` 仓储。
#[derive(Clone)]
pub struct ImConfigRepository {
    pool: PgPool,
}

impl ImConfigRepository {
    /// 构造仓储。
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按 `code`（+ 可选 `shop_id`）取 IM 配置行。
    ///
    /// 对应 Java `ImConfigServiceImpl#detail` / `#create` 里的
    /// `LambdaQueryWrapper` + `getOne`：
    /// `shop_id == None` 时不加该条件（Java `eq(ObjectUtil.isNotEmpty(shopId), ...)`）。
    ///
    /// **顺序说明**：Java `getOne` 在命中多行时抛
    /// `TooManyResultsException`；此处固定 `ORDER BY id ASC LIMIT 1` 取首条，
    /// 差异已记入切片的 `uncertain_points`。
    pub async fn find_by_code(
        &self,
        code: &str,
        shop_id: Option<i32>,
    ) -> Result<Option<ImConfig>> {
        let (sql, has_shop) = match shop_id {
            Some(_) => (
                "SELECT id, code, data, shop_id FROM im_config \
                 WHERE code = ? AND shop_id = ? ORDER BY id ASC LIMIT 1",
                true,
            ),
            None => (
                "SELECT id, code, data, shop_id FROM im_config \
                 WHERE code = ? ORDER BY id ASC LIMIT 1",
                false,
            ),
        };
        let mut q = sqlx::query_as::<_, ImConfig>(sql).bind(code);
        if has_shop {
            q = q.bind(shop_id);
        }
        q.fetch_optional(&self.pool).await.map_err(map_err)
    }

    /// 新建 IM 配置行，返回自增主键。
    ///
    /// 对应 Java `ImConfigServiceImpl#create` 的 `this.save(createConfig)`
    /// （`BeanUtils.copyProperties(createDTO, createConfig)` → \"code\" / \"shop_id\"，
    /// 随后 `setData(JsonUtil.toJson(createDTO.getData()))`）。
    ///
    /// \"data\" 为 \"None\" 时落库 \"NULL\"（Java `JsonUtil.toJson(null)` 返回 \"null\"）。
    pub async fn insert_config(
        &self,
        code: &str,
        data: Option<&str>,
        shop_id: i32,
    ) -> Result<i32> {
        let r: i32 = sqlx::query_scalar("INSERT INTO im_config (code, data, shop_id) VALUES (?, ?, ?) RETURNING id")
            .bind(code)
            .bind(data)
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 覆盖已有 IM 配置行的 \"data\"。
    ///
    /// 对应 Java `ImConfigServiceImpl#create` 的 else 分支：
    /// 只 `config.setData(...)` 后 `updateById(config)`，故**只更新 \"data\"**，
    /// 不动 \"code\" / \"shop_id\"（避免 \"BeanUtils\" 语义外扩）。
    ///
    /// 返回受影响行数（\"0\" 表示行已被并发删除）。
    pub async fn update_config_data(&self, id: i32, data: Option<&str>) -> Result<u64> {
        let r = sqlx::query("UPDATE im_config SET data = ? WHERE id = ?")
            .bind(data)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// `sqlx::Error` → 项目统一错误。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("IM 配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_err_maps_row_not_found_to_not_found() {
        match map_err(sqlx::Error::RowNotFound) {
            Error::NotFound(msg) => assert_eq!(msg, "IM 配置不存在"),
            other => panic!("期望 Error::NotFound，实际 {other:?}"),
        }
    }

    #[test]
    fn map_err_keeps_other_errors_as_database() {
        let err = map_err(sqlx::Error::Protocol("boom".into()));
        assert!(matches!(err, Error::Database(_)));
    }
}"