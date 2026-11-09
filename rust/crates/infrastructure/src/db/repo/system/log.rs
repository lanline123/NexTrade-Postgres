//! 统计基础表域（system::log）仓储。
//!
//! 只读写两张统计表（列名严格对齐 `schema_parser`）：
//! - `statistics_base`：`id` / `date` / `click_count` / `shop_id` / `visitor_count`
//! - `statistics_log`：`id` / `access_time` / `shop_id` / `product_id` /
//!   `shop_category_id` / `user`
//!
//! 查询使用运行时 `sqlx::query_as`（**不使用** `query!` 宏，本机无 `DATABASE_URL`）。
//!
//! # 事务
//!
//! Java `StatisticsBaseServiceImpl#create` **没有** `@Transactional`，
//! 每次 `save` / `updateById` 都自动提交；本仓储因此也不提供事务编排
//! （服务层按相同顺序逐条下发）。

use sqlx::PgPool;

use nextrade_core::entities::{StatisticsBase, StatisticsLog};
use nextrade_core::{Error, Result};

/// 统计基础表域仓储（持有连接池）。
#[derive(Clone)]
pub struct LogRepository {
    pool: PgPool,
}

impl LogRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 按日期查全局行（Java `eq(date, today).last("LIMIT 1")`）。
    ///
    /// ⚠️ Java 的这条查询 **不带 `shop_id` 过滤**：只要有当天任意行（含店铺行）
    /// 就会命中并累加。本实现逐字保留该语义（`LIMIT 1` 取存储顺序首行）。
    pub async fn find_by_date(&self, date: &str) -> Result<Option<StatisticsBase>> {
        sqlx::query_as::<_, StatisticsBase>(
            "SELECT * FROM statistics_base WHERE date = $1 LIMIT 1",
        )
        .bind(date)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按「日期 + 店铺」查行（Java `eq(date).eq(shop_id)` + `getOne`）。
    ///
    /// Java `getOne` 在命中多行时会抛 `TooManyResultsException`；这里 `LIMIT 1`。
    pub async fn find_by_date_and_shop(
        &self,
        date: &str,
        shop_id: i32,
    ) -> Result<Option<StatisticsBase>> {
        sqlx::query_as::<_, StatisticsBase>(
            "SELECT * FROM statistics_base WHERE date = $1 AND shop_id = $2 LIMIT 1",
        )
        .bind(date)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 插入统计基础行（Java `save`），返回自增 id。
    ///
    /// Java 只给 `date` / `click_count`（及可选 `visitor_count` / `shop_id`）赋值，
    /// 其余列落 DDL 默认；本实现显式绑定同样的值（`shop_id` 缺省 0、
    /// `visitor_count` 缺省 0），保证「不传即 0」而不是 NULL。
    pub async fn insert_base(
        &self,
        date: &str,
        click_count: i32,
        shop_id: i32,
        visitor_count: i32,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO statistics_base (date, click_count, shop_id, visitor_count) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(date)
        .bind(click_count)
        .bind(shop_id)
        .bind(visitor_count)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }

    /// 更新计数（Java \"updateById\"：写 \"click_count\"，`isNew != null` 时写
    /// \"visitor_count\"）。
    ///
    /// `visitor_count == None` 时**不更新该列**（保留库中原值），与
    /// MyBatis-Plus \"NOT_NULL\" 策略下「null 字段不入 SET」一致。
    pub async fn update_counts(
        &self,
        id: i32,
        click_count: i32,
        visitor_count: Option<i32>,
    ) -> Result<bool> {
        let affected = match visitor_count {
            Some(visitor_count) => sqlx::query(
                "UPDATE statistics_base SET click_count = $1, visitor_count = $2 WHERE id = $3",
            )
            .bind(click_count)
            .bind(visitor_count)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected(),
            None => sqlx::query("UPDATE statistics_base SET click_count = $1 WHERE id = $2")
                .bind(click_count)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_err)?
                .rows_affected(),
        };
        Ok(affected > 0)
    }

    /// 插入统计明细（Java `statisticsLogService.save(statisticsLog)`）。
    pub async fn insert_log(&self, log: &StatisticsLog) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(
            "INSERT INTO statistics_log \
             (access_time, shop_id, product_id, shop_category_id, user) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(log.access_time)
        .bind(log.shop_id)
        .bind(log.product_id)
        .bind(log.shop_category_id)
        .bind(&log.user)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("统计数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::entities::StatisticsBase;

    /// 统计基础行实体字段名与 schema 对齐（FromRow 列映射的前提）。
    #[test]
    fn statistics_base_entity_columns_exist() {
        let row = StatisticsBase {
            id: 1,
            date: Some("2026-01-01".into()),
            click_count: Some(2),
            shop_id: Some(0),
            visitor_count: Some(1),
        };
        assert_eq!(row.date.as_deref(), Some("2026-01-01"));
        assert_eq!(StatisticsBase::TABLE, "statistics_base");
    }

    /// 统计明细实体字段名与 schema 对齐。
    #[test]
    fn statistics_log_entity_columns_exist() {
        let log = StatisticsLog {
            id: 0,
            access_time: Some(1_700_000_000),
            shop_id: Some(0),
            product_id: Some(7),
            shop_category_id: 0,
            user: Some("42".into()),
        };
        assert_eq!(log.product_id, Some(7));
        assert_eq!(StatisticsLog::TABLE, "statistics_log");
    }
}
