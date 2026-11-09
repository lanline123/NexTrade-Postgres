//! catalog::product_manage client/admin 端点 域仓储扩展（wave8 batch 2）。
//!
//! 服务层不允许直接写 SQL（services crate 无 sqlx 依赖），因此把 batch 2 中
//! 需要的两段跨表查询集中到本文件：
//! 1. `paginate_select_product_list`：管理端 SKU 商品选择列表
//!    （`product_sku` JOIN `product`，shopId/productType/exclude 过滤 + 分页）。
//! 2. `insert_price_inquiry`：商城端商品询价写入 `price_inquiry`。
//!
//! 表/列名以 `java/database/pure.sql` 为准；运行时 sqlx（**不用** `query!` 宏）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use sqlx::FromRow;

use nextrade_core::models::catalog::product_manage_client::SelectProductListQuery;
use nextrade_core::{Error, Result};

/// SKU 商品选择列表行。
#[derive(Debug, Clone, FromRow)]
pub struct ProductSkuSelectRow {
    /// SKU ID
    pub sku_id: i32,
    /// 商品 ID
    pub product_id: i32,
    /// SKU 价格
    pub sku_price: Option<Decimal>,
    /// SKU 库存
    pub sku_stock: Option<i32>,
    /// 商品名称
    pub product_name: String,
    /// 商品货号
    pub product_sn: String,
    /// 商品主图
    pub pic_url: String,
    /// 商品售价
    pub product_price: Option<Decimal>,
    /// 分类 ID
    pub category_id: i32,
    /// 商品类型
    pub product_type: Option<i32>,
    /// 店铺 ID
    pub shop_id: i32,
}

/// batch 2 域仓储扩展。
#[derive(Clone)]
pub struct ProductManageClientRepository {
    pool: PgPool,
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

impl ProductManageClientRepository {
    /// 构造仓储（与其余 feature 一致，持连接池克隆）。
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// SKU 商品选择列表（Java `ProductController.selectProductList`）。
    pub async fn paginate_select_product_list(
        &self,
        q: &SelectProductListQuery,
    ) -> Result<(u64, Vec<ProductSkuSelectRow>)> {
        let mut where_sql = String::from(" WHERE p.is_delete = 0");
        let mut binds: Vec<i64> = Vec::new();
        if let Some(shop_id) = q.shop_id {
            where_sql.push_str(" AND p.shop_id = ?");
            binds.push(i64::from(shop_id));
        }
        if let Some(types) = q.product_type.as_ref().filter(|v| !v.is_empty()) {
            where_sql.push_str(&format!(
                " AND p.product_type IN ({})",
                vec!["?"; types.len()].join(",")
            ));
            for t in types {
                binds.push(i64::from(*t));
            }
        }
        if let Some(ex) = q.exclude_product_types.as_ref().filter(|v| !v.is_empty()) {
            where_sql.push_str(&format!(
                " AND p.product_type NOT IN ({})",
                vec!["?"; ex.len()].join(",")
            ));
            for t in ex {
                binds.push(i64::from(*t));
            }
        }
        let page = q.page_query();
        let size = u64::from(page.page_size);
        let offset = u64::from(page.page - 1) * size;

        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM product_sku s JOIN product p ON p.product_id = s.product_id{where_sql}"
        ));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        for b in &binds {
            cq = cq.bind(*b);
        }
        let total = cq.fetch_one(&self.pool).await.map_err(map_err)?.0;

        let rows_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT s.sku_id, s.product_id, s.sku_price, s.sku_stock, p.product_name, p.product_sn, \
             p.pic_url, p.product_price, p.category_id, p.product_type, p.shop_id \
             FROM product_sku s JOIN product p ON p.product_id = s.product_id{where_sql} \
             ORDER BY s.sku_id DESC LIMIT ? OFFSET ?"
        ));
        let mut rq = sqlx::query_as::<_, ProductSkuSelectRow>(&rows_sql);
        for b in &binds {
            rq = rq.bind(*b);
        }
        let rows = rq
            .bind(i64::try_from(size).unwrap_or(20))
            .bind(i64::try_from(offset).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total).unwrap_or(0), rows))
    }

    /// 商品询价写入（Java `ProductService.priceInquiry`）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_price_inquiry(
        &self,
        mobile: &str,
        email: &str,
        content: &str,
        product_id: i32,
        shop_id: i32,
        user_id: i32,
        create_time: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO price_inquiry (mobile, email, content, product_id, remark, status, shop_id, user_id, user_is_del, create_time) \
             VALUES ($1, $2, $3, $4, '', 0, $5, $6, 0, $7)",
        )
        .bind(mobile)
        .bind(email)
        .bind(content)
        .bind(product_id)
        .bind(shop_id)
        .bind(user_id)
        .bind(create_time)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}
