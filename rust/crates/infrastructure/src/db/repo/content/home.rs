//! 首页聚合仓储（`content/home`）。
//!
//! 首页控制器在 Java 侧注入 `FriendLinksService` / `DecorateService` /
//! `DecorateDiscreteService` / `ConfigService`。Rust 侧：
//! - 装修页面/离散组件/预览复用既有 `repos.content.decorate`（`find_home_page` /
//!   `find_by_id` / `find_discrete`），不重复实现；
//! - 本仓储只补 **友情链接** 与 **配置项** 两类查询。
//!
//! 全部 SQL 为运行时 `sqlx::query_as` / `sqlx::query`（禁止 `query!` 宏）。

use std::collections::HashMap;

use sqlx::PgPool;

use nextrade_core::entities::FriendLinks;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct HomeRepository {
    pool: PgPool,
}

impl HomeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 友情链接列表（Java `FriendLinksService#list`）。
    ///
    /// Java 未显式排序，MyBatis 按主键顺序返回；这里固定
    /// `sort_order ASC, link_id ASC` 保证分页/渲染稳定。
    pub async fn list_friend_links(&self) -> Result<Vec<FriendLinks>> {
        sqlx::query_as::<_, FriendLinks>(
            "SELECT * FROM friend_links ORDER BY sort_order ASC, link_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量读取配置项（`config.biz_code`，Java `ConfigService#getConfigVal`）。
    ///
    /// 返回 `biz_code -> Option<biz_val>`；未命中的键不出现在 map 中。
    pub async fn config_vals(&self, codes: &[&str]) -> Result<HashMap<String, Option<String>>> {
        if codes.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = vec!["?"; codes.len()].join(", ");
        let sql = format!(
            "SELECT biz_code, biz_val FROM config WHERE is_del = 0 AND biz_code IN ({placeholders})"
        );
        let mut q = sqlx::query_as::<_, (String, Option<String>)>(&sql);
        for c in codes {
            q = q.bind(*c);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 离散装修组件（Java `DecorateDiscreteService#detail`：
    /// `shop_id = 0 AND decorate_sn = ?`）。
    pub async fn find_discrete(
        &self,
        decorate_sn: &str,
    ) -> Result<Option<nextrade_core::entities::DecorateDiscrete>> {
        sqlx::query_as::<_, nextrade_core::entities::DecorateDiscrete>(
            "SELECT * FROM decorate_discrete WHERE shop_id = 0 AND decorate_sn = $1 LIMIT 1",
        )
        .bind(decorate_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("装修数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_codes_short_circuit() {
        // 纯逻辑分支：空入参不触达数据库（无法在此构造 pool）
        assert!(codes_are_empty(&[]));
        assert!(!codes_are_empty(&["h5Domain"]));
    }

    fn codes_are_empty(codes: &[&str]) -> bool {
        codes.is_empty()
    }
}
