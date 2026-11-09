//! 商品关键词联想仓储（`catalog/product_search`）。
//!
//! 对应原 Java `ProductSearchServiceImpl#searchGuess` 中的：
//!
//! ```java
//! queryWrapper.select(productName, keywords, productId);
//! queryWrapper.eq(Product::getIsDelete, 0);
//! queryWrapper.and(w -> { for (kw : keywordsArr) w.like(Product::getProductName, kw); });
//! productMapper.selectList(queryWrapper);
//! ```
//!
//! 即：`is_delete = 0` 且 `product_name LIKE %词元1% AND ... AND %词元n%`。
//! 全部 SQL 为运行时 `sqlx::query_as` / `QueryBuilder`（禁止 `query!` 宏）。

use sqlx::PgPool;
use sqlx::{Postgres, QueryBuilder};

use nextrade_core::{Error, Result};

/// 联想候选商品投影（Java 只 select `product_name, keywords, product_id`）。
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct GuessProductRow {
    pub product_id: i32,
    pub product_name: String,
    pub keywords: Option<String>,
}

#[derive(Clone)]
pub struct ProductSearchRepository {
    pool: PgPool,
}

impl ProductSearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按商品名多词元 AND 模糊匹配（Java `searchGuess` 的商品查询，无 LIMIT）。
    pub async fn find_by_name_tokens(&self, tokens: &[String]) -> Result<Vec<GuessProductRow>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT product_id, product_name, keywords FROM product WHERE is_delete = 0",
        );
        for token in tokens {
            qb.push(" AND product_name LIKE ");
            qb.push_bind(format!("%{token}%"));
        }
        qb.push(" ORDER BY product_id ASC");
        qb.build_query_as::<GuessProductRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("商品不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_query_binds_each_token() {
        let mut qb = QueryBuilder::<Postgres>::new(
            "SELECT product_id, product_name, keywords FROM product WHERE is_delete = 0",
        );
        for token in ["苹果", "手机"] {
            qb.push(" AND product_name LIKE ");
            qb.push_bind(format!("%{token}%"));
        }
        let sql = qb.sql().to_string();
        qb.build();
        assert!(sql.contains("is_delete = 0"));
        assert_eq!(sql.matches("product_name LIKE ?").count(), 2);
        assert!(!sql.contains("苹果"), "LIKE 值必须走绑定参数");
    }
}
