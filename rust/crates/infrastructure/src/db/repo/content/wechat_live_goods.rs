//! 微信直播商品库域仓储（content::wechat_live_goods）。
//!
//! 微信侧商品库 API 未迁移；本地 `wechat_live_goods` 缓存表未同步，故本仓储
//! 仅提供缓存行存在性检查（不返回数据），避免编造商品信息。

use sqlx::{Postgres,PgPool, QueryBuilder};

use nextrade_core::entities::WechatLiveGoods;
use nextrade_core::{Error, Result};

/// 微信直播商品库仓储（缓存表只读）。
#[derive(Clone)]
pub struct WechatLiveGoodsRepository {
    pool: PgPool,
}

impl WechatLiveGoodsRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 分页读取本地缓存。
    pub async fn paginate(
        &self,
        room_id: Option<i32>,
        audit_status: Option<i32>,
        keyword: Option<&str>,
        page: u64,
        size: u64,
    ) -> Result<(u64, Vec<WechatLiveGoods>)> {
        let mut count = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM wechat_live_goods WHERE 1=1");
        let mut rows = QueryBuilder::<Postgres>::new("SELECT * FROM wechat_live_goods WHERE 1=1");
        for qb in [&mut count, &mut rows] {
            if let Some(room_id) = room_id {
                qb.push(" AND room_id = ").push_bind(room_id);
            }
            if let Some(status) = audit_status {
                qb.push(" AND audit_status = ").push_bind(status);
            }
            if let Some(kw) = keyword.filter(|v| !v.trim().is_empty()) {
                qb.push(" AND goods_name LIKE ").push_bind(format!("%{kw}%"));
            }
        }
        let total: (i64,) = count
            .build_query_as()
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        let offset = (page.max(1) - 1) * size;
        rows.push(" ORDER BY id DESC LIMIT ")
            .push_bind(i64::try_from(size).unwrap_or(20))
            .push(" OFFSET ")
            .push_bind(i64::try_from(offset).unwrap_or(0));
        let list = rows
            .build_query_as::<WechatLiveGoods>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), list))
    }

    /// 按微信商品 ID 或系统商品 ID 查一条缓存。
    pub async fn find_one(
        &self,
        wechat_goods_id: Option<i32>,
        product_id: Option<i64>,
    ) -> Result<Option<WechatLiveGoods>> {
        let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM wechat_live_goods WHERE 1=1");
        if let Some(id) = wechat_goods_id.filter(|v| *v > 0) {
            qb.push(" AND wechat_goods_id = ").push_bind(id);
        } else if let Some(id) = product_id.filter(|v| *v > 0) {
            qb.push(" AND product_id = ").push_bind(id);
        } else {
            return Ok(None);
        }
        qb.push(" ORDER BY id DESC LIMIT 1");
        Ok(qb
            .build_query_as::<WechatLiveGoods>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?)
    }

    /// 缓存行数量（仅诊断用；商品库数据以微信侧为准）。
    pub async fn count_cached(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM wechat_live_goods")
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.0)
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
        assert!(std::any::type_name::<WechatLiveGoodsRepository>().contains("WechatLiveGoods"));
    }
}
