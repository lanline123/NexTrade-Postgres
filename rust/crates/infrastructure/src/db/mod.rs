//! 数据库连接池与仓储入口。
//!
//! 对应原 Java HikariCP + MyBatis Plus：
//! - 连接池：SQLx `PgPool`（替代 HikariCP）
//! - 迁移：SQLx migrate（替代 Flyway/手工 SQL 脚本）

use sqlx::postgres::{PgPool, PgPoolOptions};
use nextrade_common::AppConfig;

pub mod repo;

/// 将 MySQL 风格的动态 SQL（含 `?` 占位符）转换为 PostgreSQL 原生序号占位符 `$1`, `$2`, ...
///
/// 用于**运行时拼接**的 SQL（条件分支、动态 `WHERE`/`IN` 片段），这类 SQL 的占位符数量
/// 在编译期无法确定，只能在实际执行前转换。静态 SQL 已在源码中直接写成 `$N`。
///
/// 行为约定：
/// - 若 SQL 中已存在 `$N`（静态段落），新占位符从 `max($N) + 1` 续编号，保证混合 SQL 不冲突；
/// - 完全没有 `?` 时原样返回（幂等，重复调用无副作用）；
/// - 跳过 SQL 单引号字面量内的 `?`（含 `''` 转义）；
/// - 跳过 PG jsonb 操作符 `?|` / `?&`（不是占位符）。
pub fn to_pg_placeholders(sql: &str) -> String {
    use std::fmt::Write;

    // 先扫描已有的 $N 最大值，作为续编号起点
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut max_idx = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            let mut v = 0usize;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                v = v.saturating_mul(10).saturating_add((bytes[j] - b'0') as usize);
                j += 1;
            }
            if v > max_idx {
                max_idx = v;
            }
            i = j;
            continue;
        }
        i += 1;
    }

    let mut out = String::with_capacity(sql.len() + 16);
    let mut idx = max_idx + 1;
    let mut in_quote = false;
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            // SQL 中 '' 表示一个字面单引号，不改变引号状态
            if in_quote && chars.peek() == Some(&'\'') {
                out.push('\'');
                out.push('\'');
                chars.next();
                continue;
            }
            in_quote = !in_quote;
            out.push(c);
        } else if c == '?' && !in_quote {
            // PG jsonb 存在性操作符 ?| / ?& 不是占位符
            if matches!(chars.peek(), Some('|') | Some('&')) {
                out.push(c);
                continue;
            }
            let _ = write!(&mut out, "${idx}");
            idx += 1;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod to_pg_placeholders_tests {
    use super::to_pg_placeholders;

    #[test]
    fn plain_placeholders() {
        assert_eq!(to_pg_placeholders("SELECT * FROM t WHERE a=? AND b=?"),
                   "SELECT * FROM t WHERE a=$1 AND b=$2");
    }

    #[test]
    fn continues_after_existing_dollar() {
        assert_eq!(to_pg_placeholders("SELECT * FROM t WHERE a=$1 AND b=?"),
                   "SELECT * FROM t WHERE a=$1 AND b=$2");
    }

    #[test]
    fn skips_quoted_literal() {
        assert_eq!(to_pg_placeholders("SELECT 'a?b', x=? FROM t"),
                   "SELECT 'a?b', x=$1 FROM t");
        assert_eq!(to_pg_placeholders("SELECT 'it''s ?', x=? FROM t"),
                   "SELECT 'it''s ?', x=$1 FROM t");
    }

    #[test]
    fn skips_jsonb_operators() {
        assert_eq!(to_pg_placeholders("SELECT * FROM t WHERE j ?| array['a'] AND x=?"),
                   "SELECT * FROM t WHERE j ?| array['a'] AND x=$1");
    }

    #[test]
    fn idempotent() {
        let once = to_pg_placeholders("SELECT * FROM t WHERE a=? AND b=?");
        assert_eq!(to_pg_placeholders(&once), once);
    }
}

/// SQLx 连接池类型别名。
pub type DbPool = PgPool;

/// 初始化数据库连接池，并执行可选的迁移。
pub async fn init_pool(cfg: &AppConfig) -> anyhow::Result<DbPool> {
    let url = if cfg.datasource.url.is_empty() {
        // 回退到环境变量组合（TIG_DATASOURCE_URL 优先）
        let host = std::env::var("TIG_DATASOURCE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = std::env::var("TIG_DATASOURCE_PORT").unwrap_or_else(|_| "5432".into());
        let db = std::env::var("TIG_DATASOURCE_DATABASE").unwrap_or_else(|_| "nextrade".into());
        let user = cfg.datasource.username.clone();
        let pass = cfg.datasource.password.clone();
        format!("postgres://{user}:{pass}@{host}:{port}/{db}")
    } else {
        cfg.datasource.url.clone()
    };

    let pool = PgPoolOptions::new()
        .max_connections(cfg.datasource.max_connections.max(1))
        .acquire_timeout(std::time::Duration::from_secs(10))
        .test_before_acquire(true)
        .connect(&url)
        .await
        .map_err(|e| anyhow::anyhow!("数据库连接失败: {e}"))?;

    tracing::info!("数据库连接池初始化完成 (max_connections={})", cfg.datasource.max_connections);
    Ok(pool)
}

/// 执行 migrations/ 目录下的 SQL 迁移（按文件名排序，幂等记录于 _tig_migrations 表）。
pub async fn run_migrations(pool: &DbPool, dir: &str) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _tig_migrations (
            id VARCHAR(255) PRIMARY KEY,
            applied_at BIGINT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("读取迁移目录失败 {dir}: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "sql").unwrap_or(false))
        .collect();
    entries.sort();

    for path in entries {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let already: Option<(String,)> =
            sqlx::query_as("SELECT id FROM _tig_migrations WHERE id = $1")
                .bind(&name)
                .fetch_optional(pool)
                .await?;
        if already.is_some() {
            continue;
        }
        let sql = std::fs::read_to_string(&path)?;
        // 去掉 SQL 行注释（-- 开头），避免被当作语句执行
        let cleaned: String = sql
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut tx = pool.begin().await?;
        // sqlx 默认不执行多语句，这里按分号逐条执行
        for stmt in cleaned.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query(stmt).execute(&mut *tx).await.map_err(|e| {
                anyhow::anyhow!("迁移 {name} 执行失败 [{stmt}...]: {e}")
            })?;
        }
        sqlx::query("INSERT INTO _tig_migrations (id, applied_at) VALUES ($1, $2)")
            .bind(&name)
            .bind(nextrade_common::utils::now_millis())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        tracing::info!("已应用迁移: {name}");
    }
    Ok(())
}

/// 全仓储聚合根（按限界上下文分组）。
#[derive(Clone)]
pub struct Repositories {
    pub pool: DbPool,
    pub identity: repo::identity::Repositories,
    pub merchant: repo::merchant::Repositories,
    pub vendor: repo::vendor::Repositories,
    pub pos: repo::pos::Repositories,
    pub o2o: repo::o2o::Repositories,
    pub wholesale: repo::wholesale::Repositories,
    pub catalog: repo::catalog::Repositories,
    pub trade: repo::trade::Repositories,
    pub payment: repo::payment::Repositories,
    pub marketing: repo::marketing::Repositories,
    pub member: repo::member::Repositories,
    pub logistics: repo::logistics::Repositories,
    pub content: repo::content::Repositories,
    pub system: repo::system::Repositories,
    pub im: repo::im::Repositories,
    pub analytics: repo::analytics::Repositories,
    pub lang: repo::lang::Repositories,
}

impl Repositories {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool: pool.clone(),
            identity: repo::identity::Repositories::new(pool.clone()),
            merchant: repo::merchant::Repositories::new(pool.clone()),
            vendor: repo::vendor::Repositories::new(pool.clone()),
            pos: repo::pos::Repositories::new(pool.clone()),
            o2o: repo::o2o::Repositories::new(pool.clone()),
            wholesale: repo::wholesale::Repositories::new(pool.clone()),
            catalog: repo::catalog::Repositories::new(pool.clone()),
            trade: repo::trade::Repositories::new(pool.clone()),
            payment: repo::payment::Repositories::new(pool.clone()),
            marketing: repo::marketing::Repositories::new(pool.clone()),
            member: repo::member::Repositories::new(pool.clone()),
            logistics: repo::logistics::Repositories::new(pool.clone()),
            content: repo::content::Repositories::new(pool.clone()),
            system: repo::system::Repositories::new(pool.clone()),
            im: repo::im::Repositories::new(pool.clone()),
            analytics: repo::analytics::Repositories::new(pool.clone()),
            lang: repo::lang::Repositories::new(pool.clone()),
        }
    }
}
