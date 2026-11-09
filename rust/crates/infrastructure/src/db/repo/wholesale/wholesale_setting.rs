//! 批发设置域（wholesale::wholesale_setting）仓储。
//!
//! 对应原 Java：
//! - `com.nextrade.service.setting.impl.ConfigServiceImpl#getConfigVal` /
//!   `saveOrUpdate`（批发设置只读写 `config` 表的 4 个 biz_code）
//! - `com.nextrade.service.wholesale.impl.WholesaleSettingServiceImpl#readInt`
//!   / `#writeInt`
//! - `com.nextrade.service.wholesale.impl.WholesalerGradeServiceImpl#getById`
//!   （设置页回填默认等级名称 + `requireEnabled`）
//!
//! # 表 / 列（权威来源）
//!
//! - `config`（10 列，主键 `id`）：本域只碰 `biz_code` / `biz_val` /
//!   `is_del` / `create_time` / `update_time`
//! - `wholesaler_grade`（8 列，主键 `grade_id`）：本域只读
//!   `grade_id` / `grade_name` / `status` / `is_delete`
//!
//! 查询统一使用运行时 `sqlx::query_as`（**不使用** `query!` 宏）。
//!
//! # 说明
//!
//! Java `ConfigService` 自带缓存层（`ConfigValCache`），本域未迁移缓存，
//! 直接读写数据库；语义等价（写后下一次读即为新值），见模块头差异第 2 条。

use sqlx::PgPool;

use nextrade_core::models::wholesale::wholesale_setting::{
    BIZ_WHOLESALE_AUTO_REGISTER, BIZ_WHOLESALE_DEFAULT_GRADE_ID, BIZ_WHOLESALE_ENABLED,
    BIZ_WHOLESALE_GIVE_POINTS, DEFAULT_INT_VALUE, MSG_GRADE_NOT_FOUND,
};
use nextrade_core::{Error, Result};

/// `config` 表读取单个 biz_code 的 `biz_val`。
///
/// 对应 Java `ConfigService#getConfigVal`：`biz_code = ? AND is_del = 0`
/// `LIMIT 1`；缺失返回 `None`。
///
/// # Errors
/// 数据库执行失败时返回 `Error::Database`。
pub async fn read_config_val(pool: &PgPool, biz_code: &str) -> Result<Option<String>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
            .bind(biz_code)
            .fetch_optional(pool)
            .await
            .map_err(map_err)?;
    Ok(row.and_then(|(v,)| v))
}

/// 批发设置仓储（持有连接池）。
#[derive(Clone)]
pub struct WholesaleSettingRepository {
    pool: PgPool,
}

impl WholesaleSettingRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— config（4 个批发配置项） —————————————————————

    /// 读取单个批发配置项的原始字符串值（Java `readInt` 的第一步）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn read_val(&self, biz_code: &str) -> Result<Option<String>> {
        read_config_val(&self.pool, biz_code).await
    }

    /// 读取 4 个批发配置项的原始值。
    ///
    /// 返回顺序：`wholesaleEnabled` / `wholesaleAutoRegister` /
    /// `wholesaleDefaultGradeId` / `wholesaleGivePoints`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn read_all(&self) -> Result<[Option<String>; 4]> {
        Ok([
            self.read_val(BIZ_WHOLESALE_ENABLED).await?,
            self.read_val(BIZ_WHOLESALE_AUTO_REGISTER).await?,
            self.read_val(BIZ_WHOLESALE_DEFAULT_GRADE_ID).await?,
            self.read_val(BIZ_WHOLESALE_GIVE_POINTS).await?,
        ])
    }

    /// 写入单个配置项（Java `writeInt` → `configService.saveOrUpdate`）。
    ///
    /// 语义：`biz_code` 命中未删除行 → `UPDATE biz_val/update_time`；
    /// 未命中 → `INSERT`（`create_time = update_time = now`）。
    ///
    /// 返回 `true` = 更新已有行，`false` = 新建行。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn write_int(&self, biz_code: &str, value: i32, now: i64) -> Result<bool> {
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1")
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        let val = value.to_string();
        match existing {
            Some((id,)) => {
                sqlx::query("UPDATE config SET biz_val = $1, update_time = $2 WHERE id = $3")
                    .bind(&val)
                    .bind(now)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
                Ok(false)
            }
            None => {
                sqlx::query(
                    "INSERT INTO config (biz_code, biz_val, is_del, create_time, update_time) VALUES ($1, $2, 0, $3, $4)",
                )
                .bind(biz_code)
                .bind(&val)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(map_err)?;
                Ok(true)
            }
        }
    }

    // ————————————————————— wholesaler_grade（只读） —————————————————————

    /// 按 ID 查询批发等级（`is_delete = 0` 才可见，对应 Java `@TableLogic`）。
    ///
    /// 返回 `(grade_id, grade_name, status)`。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn find_grade(&self, grade_id: i32) -> Result<Option<(i32, String, i32)>> {
        let row: Option<(i32, String, i32)> = sqlx::query_as(
            "SELECT grade_id, grade_name, status FROM wholesaler_grade WHERE grade_id = $1 AND is_delete = 0 LIMIT 1",
        )
        .bind(grade_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 默认等级有效性（Java `requireEnabled` 的只读版本）：
    /// `Some(true)` 存在且启用、`Some(false)` 存在但禁用、`None` 不存在。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn grade_enabled(&self, grade_id: i32) -> Result<Option<bool>> {
        Ok(self.find_grade(grade_id).await?.map(|(_, _, status)| status == 1))
    }
}

/// 配置项默认值兜底（java `parseIntOrNull(defaultValue)`）。
#[must_use]
pub const fn default_int() -> i32 {
    DEFAULT_INT_VALUE
}

/// 数据库错误映射（与其它域仓储同一约定）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_GRADE_NOT_FOUND.into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认值兜底与 Java `SettingsEnum#getDefaultValue()` 一致（"0"）。
    #[test]
    fn default_int_matches_settings_enum() {
        assert_eq!(default_int(), 0);
        assert_eq!(DEFAULT_INT_VALUE, 0);
    }

    /// 4 个配置项 biz_code 与 Java `SettingsEnum` 完全同名。
    #[test]
    fn biz_codes_are_stable() {
        assert_eq!(BIZ_WHOLESALE_ENABLED, "wholesaleEnabled");
        assert_eq!(BIZ_WHOLESALE_AUTO_REGISTER, "wholesaleAutoRegister");
        assert_eq!(BIZ_WHOLESALE_DEFAULT_GRADE_ID, "wholesaleDefaultGradeId");
        assert_eq!(BIZ_WHOLESALE_GIVE_POINTS, "wholesaleGivePoints");
    }
}
