//! 客服接入规则 域仓储（im::admin_shop_kefu_config）。
//!
//! 表 `shop_kefu_config`：`id` / `shop_id` / `auto_access_mode` /
//! `return_customer_access` / `create_time` / `modify_time`（列名以
//! `java/database/pure.sql` 为准）。SQL 均为运行时 `sqlx`（**不**使用
//! `query!` 宏：本机无 `DATABASE_URL`）。

use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::{Error, Result};

/// `shop_kefu_config` 接入规则行（只取本 feature 需要的列）。
#[derive(Debug, Clone, FromRow)]
pub struct ShopKefuAccessRow {
    /// 主键
    pub id: i32,
    /// 店铺 ID
    pub shop_id: i32,
    /// 自动接入模式
    pub auto_access_mode: i32,
    /// 老客接入开关
    pub return_customer_access: i32,
}

/// 客服接入规则仓储。
#[derive(Clone)]
pub struct AdminShopKefuConfigRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl AdminShopKefuConfigRepository {
    /// 构造仓储。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按店铺查接入规则（Java `findByShopId`：`limit 1`）。
    pub async fn find_by_shop_id(&self, shop_id: i32) -> Result<Option<ShopKefuAccessRow>> {
        sqlx::query_as::<_, ShopKefuAccessRow>(
            "SELECT id, shop_id, auto_access_mode, return_customer_access \
             FROM shop_kefu_config WHERE shop_id = ? LIMIT 1",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 新增接入规则（Java \"createDefault\" + \"save\"；其它渠道字段落默认值）。
    pub async fn insert(&self, shop_id: i32, auto_access_mode: i32, return_customer_access: i32, now: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO shop_kefu_config \
             (shop_id, kefu_type, kefu_yzf_type, kefu_yzf_sign, kefu_workwx_id, corp_id, kefu_code, \
              kefu_code_blank, kefu_whatsapp_phone, kefu_phone, kefu_time, auto_access_mode, \
              return_customer_access, create_time, modify_time) \
             VALUES (?, 0, 0, '', '', '', '', 0, '', '', '', ?, ?, ?, ?)",
        )
        .bind(shop_id)
        .bind(auto_access_mode)
        .bind(return_customer_access)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 更新接入规则（只更新非空字段，Java `BeanUtil.copyProperties(ignoreNullValue)`）。
    pub async fn update(
        &self,
        id: i32,
        auto_access_mode: Option<i32>,
        return_customer_access: Option<i32>,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE shop_kefu_config SET \
             auto_access_mode = COALESCE(?, auto_access_mode), \
             return_customer_access = COALESCE(?, return_customer_access), \
             modify_time = ? WHERE id = ?",
        )
        .bind(auto_access_mode)
        .bind(return_customer_access)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}
