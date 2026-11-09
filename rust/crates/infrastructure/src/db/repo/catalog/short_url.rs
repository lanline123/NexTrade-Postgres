//! 短链接域仓储（catalog::short_url）。
//!
//! 对应原 Java `com.nextrade.service.promotion.impl.ShortUrlServiceImpl#getByCode`：
//!
//! ```java
//! this.lambdaQuery().like(ShortUrl::getShortCode, code).last("limit 1").one();
//! ```
//!
//! ⚠️ Java 用的是 **`like`（模糊匹配）** 而不是 `eq`，即
//! `short_code LIKE '%<code>%' LIMIT 1`。这是原系统既有行为（短码通常是
//! 定长随机串，模糊匹配在实践中等价于精确匹配，但确实不是 `=`）。
//! 本仓储**忠实照抄**，不擅自改成 `=`（改了会让历史短链的解析结果与 Java 不一致）。
//!
//! 列名对齐权威 schema（`python tools/schema_parser.py --table short_url`）：
//! `short_url_id` / `short_code` / `long_url` / `type` / `add_time`（共 5 列）。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// 查询结果（只取控制器需要的两列）。
#[derive(Debug, Clone)]
pub struct ShortUrlRow {
    pub long_url: String,
    /// Java `ShortUrl.type`：`null` 或 `<= 1` 走 H5 域名，否则走 PC 域名。
    pub url_type: Option<i32>,
}

/// 短链接仓储。
#[derive(Clone)]
pub struct ShortUrlRepository {
    pool: PgPool,
}

impl ShortUrlRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// Java `lambdaQuery().like(short_code, code).last("limit 1").one()`。
    ///
    /// 只取控制器实际用到的两列（`long_url` / `type`），元组列数必须与
    /// `query_as::<_, (String, Option<i32>)>` 一致。
    pub async fn find_by_code(&self, code: &str) -> Result<Option<ShortUrlRow>> {
        let sql = "SELECT long_url, type FROM short_url WHERE short_code LIKE CONCAT('%', $1, '%') LIMIT 1";
        let row: Option<(String, Option<i32>)> = sqlx::query_as(sql)
            .bind(code)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(|(long_url, url_type)| ShortUrlRow { long_url, url_type }))
    }

    /// 读系统配置（Java `ConfigService.getConfigVal`，缺行 → `None`）。
    ///
    /// 与 `repo::nextrade_core::models::member::user_center::find_config_val` 同一口径（`config` 表带 `is_del` 软删标记）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }
}

/// `sqlx::Error` → `nextrade_core::Error`（`Error` 未提供 `From<sqlx::Error>`）。
fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("记录不存在".into()),
        other => Error::Database(other.to_string()),
    }
}