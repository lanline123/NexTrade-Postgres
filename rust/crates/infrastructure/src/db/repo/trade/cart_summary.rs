//! 购物车数量/优惠券试算域仓储（trade::cart_summary）。
//!
//! 表：`cart`；查询使用运行时 `sqlx::query_as`。

use rust_decimal::Decimal;
use sqlx::PgPool;

use nextrade_core::{Error, Result};

/// 购物车汇总仓储。
#[derive(Clone)]
pub struct CartSummaryRepository {
    pool: PgPool,
}

impl CartSummaryRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 汇总用户购物车数量（Java `cartService.getCartCount(shopId, userId)`）：
    /// 排除代客下单购物车（`type = 15`），可选门店过滤。
    pub async fn sum_quantity(&self, user_id: i32, shop_id: Option<i32>) -> Result<i64> {
        let row: Option<(Option<Decimal>,)> = sqlx::query_as(
            "SELECT SUM(quantity) FROM cart
             WHERE user_id = $1 AND type <> 15 AND ($2 IS NULL OR shop_id = $3)",
        )
        .bind(user_id)
        .bind(shop_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row
            .and_then(|(sum,)| sum)
            .map(|sum| i64::try_from(sum).unwrap_or(0))
            .unwrap_or(0))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_type_exists() {
        assert!(std::any::type_name::<CartSummaryRepository>().contains("CartSummary"));
    }
}

