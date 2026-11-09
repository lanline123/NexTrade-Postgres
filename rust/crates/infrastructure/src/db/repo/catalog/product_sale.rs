//! `product_sale_control` / `vendor_product_sale_control` 仓储（v5.8.61 商品禁售）。

use sqlx::{PgPool, Row};
use nextrade_core::models::catalog::product_sale::{ProductSaleParam, ProductSaleState};
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct ProductSaleRepository {
    pool: PgPool,
}

impl ProductSaleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    /// 定时禁售/恢复的读时结算；与 Java `SaleControl.settle` 一致。
    fn settle(mut sale_status: i32, mut ban_at: i64, mut recover_at: i64, now: i64) -> (i32, i64, i64) {
        loop {
            if sale_status == 1 && ban_at > 0 && ban_at <= now {
                sale_status = 0;
                ban_at = 0;
                continue;
            }
            if sale_status == 0 && recover_at > 0 && recover_at <= now {
                sale_status = 1;
                ban_at = 0;
                recover_at = 0;
                continue;
            }
            break;
        }
        (sale_status, ban_at, recover_at)
    }

    fn banned(sale_status: i32, ban_at: i64, recover_at: i64, now: i64) -> bool {
        (sale_status == 0 && (recover_at == 0 || recover_at > now))
            || (sale_status == 1 && ban_at > 0 && ban_at <= now)
    }

    /// 读取商品所属店铺和供应商商品 ID。
    async fn product_scope(&self, product_id: i32) -> Result<Option<(i32, Option<i32>)>> {
        sqlx::query_as::<_, (i32, Option<i32>)>(
            "SELECT shop_id, vendor_product_id FROM product WHERE product_id = $1 LIMIT 1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    async fn control_row(
        &self,
        product_id: i32,
        shop_id: i32,
    ) -> Result<Option<(i32, i64, i64)>> {
        let row = sqlx::query(
            "SELECT sale_status, ban_at, recover_at FROM product_sale_control WHERE product_id = $1 AND shop_id = $2",
        )
        .bind(product_id)
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| {
            (
                r.get::<i32, _>("sale_status"),
                r.get::<i64, _>("ban_at"),
                r.try_get::<Option<i64>, _>("recover_at").ok().flatten().unwrap_or(0),
            )
        }))
    }

    async fn vendor_row(&self, vendor_product_id: i32) -> Result<Option<(i32, i64, i64)>> {
        let row = sqlx::query(
            "SELECT sale_status, ban_at, recover_at FROM vendor_product_sale_control WHERE vendor_product_id = $1",
        )
        .bind(vendor_product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| {
            (
                r.get::<i32, _>("sale_status"),
                r.get::<i64, _>("ban_at"),
                r.try_get::<Option<i64>, _>("recover_at").ok().flatten().unwrap_or(0),
            )
        }))
    }

    /// 平台/门店维度的最终可售状态。
    pub async fn setting(&self, product_id: i32, shop_id: Option<i32>) -> Result<ProductSaleState> {
        let (product_shop, vendor_product_id) = self
            .product_scope(product_id)
            .await?
            .ok_or_else(|| Error::biz(400, "商品不存在".to_string()))?;
        let target_shop = shop_id.unwrap_or(product_shop);
        if target_shop < 0 {
            return Err(Error::biz(400, "门店ID无效".to_string()));
        }
        if target_shop != product_shop {
            self.assert_shop_assigned(product_id, product_shop, target_shop).await?;
        }

        let now = Self::now();
        let global = self.control_row(product_id, -1).await?;
        let local = if target_shop != product_shop {
            self.control_row(product_id, target_shop).await?
        } else {
            None
        };
        let setting = local.or(global);
        let (mut manual, mut ban_at, mut recover_at) =
            setting.unwrap_or((1, 0, 0));
        (manual, ban_at, recover_at) = Self::settle(manual, ban_at, recover_at, now);

        let mut reason = None;
        if let Some((status, ban, recover)) = global {
            let (s, b, r) = Self::settle(status, ban, recover, now);
            if Self::banned(s, b, r, now) {
                reason = Some("platform_banned".to_string());
            }
        }
        if reason.is_none() {
            if let Some((status, ban, recover)) = local {
                let (s, b, r) = Self::settle(status, ban, recover, now);
                if Self::banned(s, b, r, now) {
                    reason = Some("shop_banned".to_string());
                }
            }
        }
        if reason.is_none() {
            if let Some(vendor_id) = vendor_product_id.filter(|v| *v > 0) {
                if let Some((status, ban, recover)) = self.vendor_row(vendor_id).await? {
                    let (s, b, r) = Self::settle(status, ban, recover, now);
                    if Self::banned(s, b, r, now) {
                        reason = Some("vendor_product_banned".to_string());
                    }
                }
            }
        }

        Ok(ProductSaleState {
            sale_status: if reason.is_some() { 0 } else { 1 },
            manual_sale_status: manual,
            sale_ban_time: ban_at,
            sale_recover_time: recover_at,
            sale_reason: reason,
        })
    }

    /// 保存平台/门店维度的禁售设置。
    pub async fn save(&self, param: &ProductSaleParam) -> Result<()> {
        if param.sale_status != 0 && param.sale_status != 1 {
            return Err(Error::biz(400, "商品可售状态或定时禁售时间无效".to_string()));
        }
        if param.sale_ban_time.unwrap_or(0) < 0 || param.sale_recover_time.unwrap_or(0) < 0 {
            return Err(Error::biz(400, "商品可售状态或定时禁售时间无效".to_string()));
        }
        let (product_shop, vendor_product_id) = self
            .product_scope(param.product_id)
            .await?
            .ok_or_else(|| Error::biz(400, "商品不存在".to_string()))?;
        let mut scope = param.shop_id.unwrap_or(-1);
        if param.shop_id == Some(product_shop) {
            scope = -1;
        }
        if scope >= 0 {
            self.assert_shop_assigned(param.product_id, product_shop, scope).await?;
        }
        let now = Self::now();
        let (sale_status, ban_at, recover_at) = Self::settle(
            param.sale_status,
            param.sale_ban_time.unwrap_or(0),
            param.sale_recover_time.unwrap_or(0),
            now,
        );
        if sale_status == 1 {
            if let Some(vendor_id) = vendor_product_id.filter(|v| *v > 0) {
                if let Some((status, ban, recover)) = self.vendor_row(vendor_id).await? {
                    let (s, b, r) = Self::settle(status, ban, recover, now);
                    if Self::banned(s, b, r, now) {
                        return Err(Error::biz(400, "供应商商品已经禁用，不能进行解禁".to_string()));
                    }
                }
            }
        }
        sqlx::query(
            "INSERT INTO product_sale_control(product_id, shop_id, sale_status, ban_at, recover_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (product_id, shop_id) DO UPDATE SET sale_status=EXCLUDED.sale_status, ban_at=EXCLUDED.ban_at, \
              recover_at=EXCLUDED.recover_at, updated_at=EXCLUDED.updated_at",
        )
        .bind(param.product_id)
        .bind(scope)
        .bind(sale_status)
        .bind(ban_at)
        .bind(recover_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn assert_shop_assigned(
        &self,
        product_id: i32,
        product_shop: i32,
        shop_id: i32,
    ) -> Result<()> {
        if product_shop != 0 {
            return Err(Error::biz(400, "商品未分配给该门店".to_string()));
        }
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM store_product WHERE product_id = $1 AND shop_id = $2 AND is_delete = 0",
        )
        .bind(product_id)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        if count == 0 {
            return Err(Error::biz(400, "商品未分配给该门店".to_string()));
        }
        Ok(())
    }

    /// 供应商商品自身的禁售状态（作用于导入它的所有店铺/自营）。
    pub async fn vendor_setting(&self, vendor_product_id: i32) -> Result<ProductSaleState> {
        let (exists,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM vendor_product WHERE id = $1 AND is_del = 0")
                .bind(vendor_product_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        if exists == 0 {
            return Err(Error::biz(400, "供应商商品不存在".to_string()));
        }
        let now = Self::now();
        let (mut manual, mut ban_at, mut recover_at) =
            self.vendor_row(vendor_product_id).await?.unwrap_or((1, 0, 0));
        (manual, ban_at, recover_at) = Self::settle(manual, ban_at, recover_at, now);
        let banned = Self::banned(manual, ban_at, recover_at, now);
        Ok(ProductSaleState {
            sale_status: if banned { 0 } else { 1 },
            manual_sale_status: manual,
            sale_ban_time: ban_at,
            sale_recover_time: recover_at,
            sale_reason: banned.then(|| "vendor_product_banned".to_string()),
        })
    }

    /// 保存供应商商品禁售设置。
    pub async fn save_vendor(
        &self,
        vendor_product_id: i32,
        sale_status: i32,
        ban_time: Option<i64>,
        recover_time: Option<i64>,
    ) -> Result<()> {
        if sale_status != 0 && sale_status != 1 {
            return Err(Error::biz(400, "商品可售状态或定时禁售时间无效".to_string()));
        }
        if ban_time.unwrap_or(0) < 0 || recover_time.unwrap_or(0) < 0 {
            return Err(Error::biz(400, "商品可售状态或定时禁售时间无效".to_string()));
        }
        let (exists,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM vendor_product WHERE id = $1 AND is_del = 0")
                .bind(vendor_product_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        if exists == 0 {
            return Err(Error::biz(400, "供应商商品不存在".to_string()));
        }
        let now = Self::now();
        let (sale_status, ban_at, recover_at) =
            Self::settle(sale_status, ban_time.unwrap_or(0), recover_time.unwrap_or(0), now);
        sqlx::query(
            "INSERT INTO vendor_product_sale_control(vendor_product_id, sale_status, ban_at, recover_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (vendor_product_id) DO UPDATE SET sale_status=EXCLUDED.sale_status, ban_at=EXCLUDED.ban_at, \
              recover_at=EXCLUDED.recover_at, updated_at=EXCLUDED.updated_at",
        )
        .bind(vendor_product_id)
        .bind(sale_status)
        .bind(ban_at)
        .bind(recover_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 前台列表/搜索统一使用的禁售过滤 SQL。
    ///
    /// 定时条件在分页前生效；门店商品按 `product.shop_id` 读取本店设置，
    /// 供应商商品单独继承供应商禁售设置。
    #[must_use]
    pub fn sale_condition() -> &'static str {
        " AND NOT EXISTS (SELECT 1 FROM product_sale_control sale_ctl \
           WHERE sale_ctl.product_id = product.product_id \
             AND (sale_ctl.shop_id = -1 OR sale_ctl.shop_id = product.shop_id) \
             AND ((sale_ctl.sale_status = 0 AND (sale_ctl.recover_at = 0 OR sale_ctl.recover_at > CAST(EXTRACT(EPOCH FROM NOW()) AS BIGINT))) \
               OR (sale_ctl.sale_status = 1 AND sale_ctl.ban_at > 0 AND sale_ctl.ban_at <= CAST(EXTRACT(EPOCH FROM NOW()) AS BIGINT)))) \
         AND NOT EXISTS (SELECT 1 FROM vendor_product_sale_control vendor_sale_ctl \
           INNER JOIN product vendor_product_ref ON vendor_sale_ctl.vendor_product_id = vendor_product_ref.vendor_product_id \
           WHERE vendor_product_ref.product_id = product.product_id \
             AND ((vendor_sale_ctl.sale_status = 0 AND (vendor_sale_ctl.recover_at = 0 OR vendor_sale_ctl.recover_at > CAST(EXTRACT(EPOCH FROM NOW()) AS BIGINT))) \
               OR (vendor_sale_ctl.sale_status = 1 AND vendor_sale_ctl.ban_at > 0 AND vendor_sale_ctl.ban_at <= CAST(EXTRACT(EPOCH FROM NOW()) AS BIGINT))))"
    }
}

fn map_err(error: sqlx::Error) -> Error {
    Error::Database(error.to_string())
}
