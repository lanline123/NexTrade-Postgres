//! 商户/店铺域仓储：`shop` / `shop_account_log` / `shop_withdraw` /
//! `shop_product_category` / `admin_user_shop` / `collect_shop` /
//! `order_config`。替代 `ShopMapper` / `AccountMapper` /
//! `ShopWithdrawMapper` / `ShopProductCategoryMapper` 等。
//!
//! 过滤条件一律写成 `(? IS NULL OR col = ?)` 形态，绑定 `Option`
//! 即可表达"未传该条件"，避免动态拼 SQL。
//! 排序字段走白名单（[`order_by`]），未知字段回落主键，
//! 对齐原 `BaseServiceImpl#getColumnByPropertyName` 的防注入回退。

use std::collections::HashMap;

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{Merchant, Shop, ShopAccountLog, ShopProductCategory, ShopWithdraw};
use nextrade_core::models::merchant::shop::ShopUpdateDTO;
use nextrade_core::types::PageQuery;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct ShopRepository {
    pool: PgPool,
}

impl ShopRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ===== 店铺（`shop`） =====

    /// 店铺分页（`ShopServiceImpl#list`）。
    ///
    /// `status` 传 `None` 表示不按状态过滤（原 Java 仅 `status > 0` 才过滤）；
    /// `add_start` / `add_end` 为秒时间戳。
    pub async fn paginate(
        &self,
        keyword: Option<&str>,
        shop_type: Option<i32>,
        status: Option<i32>,
        add_start: Option<i64>,
        add_end: Option<i64>,
        store_parent_id: Option<i32>,
        shop_region_id: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<Shop>)> {
        let where_sql = "
            WHERE (is_delete IS NULL OR is_delete = 0)
              AND ($1 IS NULL OR shop_type = $2)
              AND ($3 IS NULL OR status = $4)
              AND ($5 IS NULL OR shop_title LIKE CONCAT('%', $6, '%'))
              AND ($7 IS NULL OR store_parent_id = $8)
              AND ($9 IS NULL OR shop_region_id LIKE CONCAT('%', $10, '%'))
              AND ($11 IS NULL OR add_time >= $12)
              AND ($13 IS NULL OR add_time <= $14)";
        let order = order_by(
            sort_field,
            sort_order,
            "shop_id",
            &[
                ("shopId", "shop_id"),
                ("addTime", "add_time"),
                ("status", "status"),
                ("clickCount", "click_count"),
                ("shopSales", "shop_sales"),
                ("shopStar", "shop_star"),
            ],
        );
        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM \"shop\" {where_sql}"));

        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(shop_type)
            .bind(shop_type)
            .bind(status)
            .bind(status)
            .bind(keyword)
            .bind(keyword)
            .bind(store_parent_id)
            .bind(store_parent_id)
            .bind(shop_region_id)
            .bind(shop_region_id)
            .bind(add_start)
            .bind(add_start)
            .bind(add_end)
            .bind(add_end)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let rows_sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM \"shop\" {where_sql} {order} LIMIT ? OFFSET ?"));
        let rows = sqlx::query_as::<_, Shop>(&rows_sql)
            .bind(shop_type)
            .bind(shop_type)
            .bind(status)
            .bind(status)
            .bind(keyword)
            .bind(keyword)
            .bind(store_parent_id)
            .bind(store_parent_id)
            .bind(shop_region_id)
            .bind(shop_region_id)
            .bind(add_start)
            .bind(add_start)
            .bind(add_end)
            .bind(add_end)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 按 ID 查店铺。
    pub async fn find_by_id(&self, shop_id: i32) -> Result<Option<Shop>> {
        sqlx::query_as::<_, Shop>("SELECT * FROM \"shop\" WHERE shop_id = $1")
            .bind(shop_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 更新店铺可写字段（`ShopServiceImpl#update` → `updateById`）。
    ///
    /// `COALESCE` 对应 MyBatis-Plus 忽略 `null` 字段的更新语义。
    /// 返回是否命中行。
    pub async fn update_shop(&self, dto: &ShopUpdateDTO) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE \"shop\" SET
                shop_title = COALESCE($1, shop_title),
                shop_logo = COALESCE($2, shop_logo),
                contact_mobile = COALESCE($3, contact_mobile),
                shop_type = COALESCE($4, shop_type),
                store_parent_id = COALESCE($5, store_parent_id),
                shop_cover_picture = COALESCE($6, shop_cover_picture),
                \"description\" = COALESCE($7, \"description\"),
                status = COALESCE($8, status),
                shop_detailed_address = COALESCE($9, shop_detailed_address),
                shop_longitude = COALESCE($10, shop_longitude),
                shop_latitude = COALESCE($11, shop_latitude)
             WHERE shop_id = $12",
        )
        .bind(dto.shop_title.as_deref())
        .bind(dto.shop_logo.as_deref())
        .bind(dto.contact_mobile.as_deref())
        .bind(dto.shop_type)
        .bind(dto.store_parent_id)
        .bind(dto.shop_cover_picture.as_deref())
        .bind(dto.description.as_deref())
        .bind(dto.status)
        .bind(dto.shop_detailed_address.as_deref())
        .bind(dto.shop_longitude)
        .bind(dto.shop_latitude)
        .bind(dto.shop_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 批量查商户（店铺列表/详情回填 `merchant`，替代循环 N+1 查询）。
    pub async fn find_merchants(&self, ids: &[i32]) -> Result<Vec<Merchant>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // SQLx 不支持动态 IN 列表绑定，需内联占位符。i32 无法被注入利用。
        let sql = format!(
            "SELECT * FROM \"merchant\" WHERE merchant_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, Merchant>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 批量查店铺管理员绑定（`admin_user_shop` ⋈ `admin_user`），
    /// 返回 `(shop_id, admin_id, username)`。对应原 Java 列表页的
    /// `shopAdminBindMap` / `shopAdminUserMap` 预取。
    pub async fn find_shop_admins(
        &self,
        shop_ids: &[i32],
    ) -> Result<Vec<(i32, i32, String)>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT s.shop_id, s.admin_id, u.username
             FROM admin_user_shop s
             JOIN admin_user u ON u.admin_id = s.admin_id
             WHERE s.is_admin = 1 AND s.shop_id IN ({})",
            placeholders(shop_ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, i32, String)>(&sql);
        for id in shop_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 店铺商品总数（`ShopServiceImpl#clientDetail` 的 `productService.count`）。
    pub async fn count_products(&self, shop_id: i32) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM product WHERE shop_id = $1")
            .bind(shop_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(n.max(0))
    }

    /// 店铺新品总数（`is_new = 1`）。
    pub async fn count_new_products(&self, shop_id: i32) -> Result<i64> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product WHERE shop_id = $1 AND is_new = 1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(n.max(0))
    }

    /// 店铺收藏人数（`COUNT(DISTINCT user_id)`）。
    pub async fn count_collect_users(&self, shop_id: i32) -> Result<i64> {
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT user_id) FROM collect_shop WHERE shop_id = $1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(n.max(0))
    }

    /// 商品评分 `AVG(comment_rank)`；无评价时为 `None`（Java 侧回落 0）。
    pub async fn avg_comment_rank(&self, shop_id: i32) -> Result<Option<Decimal>> {
        let row: Option<(Option<Decimal>,)> = sqlx::query_as(
            "SELECT AVG(comment_rank) FROM comment WHERE shop_id = $1 GROUP BY shop_id",
        )
        .bind(shop_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|r| r.0))
    }

    // ===== 资金流水（`shop_account_log`） =====

    /// 资金流水分页（`AccountServiceImpl#list`）。
    ///
    /// `balance_type` 为 1 线下 / 2 线上；`start` / `end` 为秒时间戳。
    /// 默认排序 `add_time DESC, shop_account_log_id DESC`（原 Java 无 sortField 时）。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate_account_logs(
        &self,
        balance_type: Option<i32>,
        pay_channel: Option<&str>,
        shop_id: Option<i32>,
        shop_type: Option<i32>,
        keyword: Option<&str>,
        start: Option<i64>,
        end: Option<i64>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ShopAccountLog>)> {
        let where_sql = "
            WHERE ($1 IS NULL OR balance_type = $2)
              AND ($3 IS NULL OR pay_channel = $4)
              AND ($5 IS NULL OR shop_id = $6)
              AND ($7 IS NULL OR shop_id IN (SELECT shop_id FROM shop WHERE shop_type = $8))
              AND ($9 IS NULL OR shop_id IN
                        (SELECT shop_id FROM shop WHERE shop_title LIKE CONCAT('%', $10, '%')))
              AND ($11 IS NULL OR add_time >= $12)
              AND ($13 IS NULL OR add_time <= $14)";
        let order = if sort_field.map(str::trim).is_some_and(|f| !f.is_empty()) {
            order_by(
                sort_field,
                sort_order,
                "shop_account_log_id",
                &[
                    ("shopAccountLogId", "shop_account_log_id"),
                    ("addTime", "add_time"),
                    ("balanceType", "balance_type"),
                    ("shopMoney", "shop_money"),
                ],
            )
        } else {
            "ORDER BY add_time DESC, shop_account_log_id DESC".to_string()
        };

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop_account_log {where_sql}"));
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(balance_type)
            .bind(balance_type)
            .bind(pay_channel)
            .bind(pay_channel)
            .bind(shop_id)
            .bind(shop_id)
            .bind(shop_type)
            .bind(shop_type)
            .bind(keyword)
            .bind(keyword)
            .bind(start)
            .bind(start)
            .bind(end)
            .bind(end)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let rows_sql =
            crate::db::to_pg_placeholders(&format!("SELECT * FROM shop_account_log {where_sql} {order} LIMIT ? OFFSET ?"));
        let rows = sqlx::query_as::<_, ShopAccountLog>(&rows_sql)
            .bind(balance_type)
            .bind(balance_type)
            .bind(pay_channel)
            .bind(pay_channel)
            .bind(shop_id)
            .bind(shop_id)
            .bind(shop_type)
            .bind(shop_type)
            .bind(keyword)
            .bind(keyword)
            .bind(start)
            .bind(start)
            .bind(end)
            .bind(end)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 批量取店铺标题（资金流水列表回填 `shopTitle`）。
    pub async fn find_shop_titles(&self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT shop_id, shop_title FROM shop WHERE shop_id IN ({})",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, Option<String>)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, title)| (id, title.unwrap_or_default()))
            .collect())
    }

    // ===== 提现（`shop_withdraw`） =====

    /// 提现记录分页（`ShopWithdrawServiceImpl#list`）。
    #[allow(clippy::too_many_arguments)]
    pub async fn paginate_withdraws(
        &self,
        status: Option<i32>,
        withdraw_type: Option<i32>,
        pay_channel: Option<&str>,
        shop_id: Option<i32>,
        start: Option<i64>,
        end: Option<i64>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ShopWithdraw>)> {
        // 渠道过滤：微信兼容存量 `pay_channel IS NULL` 的线上记录
        //（对应原 Java `eq(WECHAT).or().isNull(payChannel)`）；
        // 未传渠道时不附加条件。
        let channel_sql = match pay_channel {
            None => "",
            Some("WECHAT") => {
                "AND withdraw_type = 2 AND (pay_channel = 'WECHAT' OR pay_channel IS NULL)"
            }
            Some(_) => "AND withdraw_type = 2 AND pay_channel = ?",
        };
        let where_sql = crate::db::to_pg_placeholders(&format!(
            "WHERE (? IS NULL OR status = ?)
              AND (? IS NULL OR withdraw_type = ?)
              AND (? IS NULL OR shop_id = ?)
              AND (? IS NULL OR add_time >= ?)
              AND (? IS NULL OR add_time <= ?)
              {channel_sql}"
        ));
        let order = order_by(
            sort_field,
            sort_order,
            "shop_withdraw_log_id",
            &[
                ("shopWithdrawLogId", "shop_withdraw_log_id"),
                ("addTime", "add_time"),
                ("amount", "amount"),
                ("status", "status"),
            ],
        );

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM shop_withdraw {where_sql}"));
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
        count_q = count_q
            .bind(status)
            .bind(status)
            .bind(withdraw_type)
            .bind(withdraw_type)
            .bind(shop_id)
            .bind(shop_id)
            .bind(start)
            .bind(start)
            .bind(end)
            .bind(end);
        if let Some(ch) = pay_channel.filter(|c| *c != "WECHAT") {
            count_q = count_q.bind(ch);
        }
        let total = count_q
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?
            .0
            .max(0) as u64;

        let (limit, offset) = page.limit_offset();
        let rows_sql = crate::db::to_pg_placeholders(&format!("SELECT * FROM shop_withdraw {where_sql} {order} LIMIT ? OFFSET ?"));
        let mut rows_q = sqlx::query_as::<_, ShopWithdraw>(&rows_sql);
        rows_q = rows_q
            .bind(status)
            .bind(status)
            .bind(withdraw_type)
            .bind(withdraw_type)
            .bind(shop_id)
            .bind(shop_id)
            .bind(start)
            .bind(start)
            .bind(end)
            .bind(end);
        if let Some(ch) = pay_channel.filter(|c| *c != "WECHAT") {
            rows_q = rows_q.bind(ch);
        }
        let rows = rows_q
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total, rows))
    }

    /// 按 ID 查提现记录。
    pub async fn find_withdraw(&self, id: i32) -> Result<Option<ShopWithdraw>> {
        sqlx::query_as::<_, ShopWithdraw>(
            "SELECT * FROM shop_withdraw WHERE shop_withdraw_log_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 线下提现审核通过/不通过（仅 `status = 0` 待审核可推进），
    /// 对应原 Java `updateById`（先校验待审核状态）。
    /// 返回是否命中行；未命中由服务层报"不是待审核状态"。
    pub async fn approve_withdraw(
        &self,
        id: i32,
        status: i32,
        audit_remark: Option<&str>,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE shop_withdraw SET status = $1, audit_remark = $2
             WHERE shop_withdraw_log_id = $3 AND status = 0",
        )
        .bind(status)
        .bind(audit_remark)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(res.rows_affected() > 0)
    }

    /// 审核拒绝：返还冻结金额并写资金流水（单事务）。
    ///
    /// 对应 `ShopWithdrawServiceImpl#audit` 命中 `AUDIT_NOT_PASS` 时调用的
    /// `AccountServiceImpl#refundWithDrawLog`：
    /// `shop_money += amount`、`frozen_money -= amount`（释放不加硬守卫，
    /// 避免脏数据卡死流转），流水记 `type=1 / is_new=1`。
    pub async fn reject_withdraw(
        &self,
        withdraw: &ShopWithdraw,
        audit_remark: Option<&str>,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let before: Option<(Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
            "SELECT shop_money, frozen_money FROM \"shop\" WHERE shop_id = $1 FOR UPDATE",
        )
        .bind(withdraw.shop_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let (old_shop, old_frozen) = match before {
            Some((Some(shop_money), Some(frozen_money))) => (shop_money, frozen_money),
            _ => return Err(Error::NotFound("店铺数据不存在".into())),
        };

        sqlx::query(
            "UPDATE \"shop\" SET shop_money = shop_money + $1, frozen_money = frozen_money - $2
             WHERE shop_id = $3",
        )
        .bind(withdraw.amount)
        .bind(withdraw.amount)
        .bind(withdraw.shop_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        let res = sqlx::query(
            "UPDATE shop_withdraw SET status = 2, audit_remark = $1
             WHERE shop_withdraw_log_id = $2 AND status = 0",
        )
        .bind(audit_remark)
        .bind(withdraw.shop_withdraw_log_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        if res.rows_affected() == 0 {
            // 状态已被并发修改，回滚
            return Err(Error::Conflict("不是待审核状态，请刷新后重试".into()));
        }

        let new_shop = old_shop + withdraw.amount;
        let new_frozen = old_frozen - withdraw.amount;
        sqlx::query(
            "INSERT INTO shop_account_log
                (add_time, shop_money, frozen_money, \"type\", remarks,
                 new_shop_money, new_frozen_money, shop_id, is_new, balance_type, change_amount)
             VALUES ($1, $2, $3, '1', $4, $5, $6, $7, 1, 1, $8)",
        )
        .bind(now_seconds())
        .bind(old_shop)
        .bind(old_frozen)
        .bind(audit_remark)
        .bind(new_shop)
        .bind(new_frozen)
        .bind(withdraw.shop_id)
        .bind(withdraw.amount)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(true)
    }

    /// 对应 `ShopWithdrawServiceImpl#uploadPayVoucher` 命中线下打款凭证完成时调用的
    /// `AccountServiceImpl#completeWithDrawLog`：
    /// `frozen_money -= amount`（释放冻结金额），流水记 `type=1 / is_new=1`。
    pub async fn complete_withdraw(
        &self,
        withdraw: &ShopWithdraw,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let before: Option<(Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
            "SELECT shop_money, frozen_money FROM \"shop\" WHERE shop_id = $1 FOR UPDATE",
        )
        .bind(withdraw.shop_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let (old_shop, old_frozen) = match before {
            Some((Some(shop_money), Some(frozen_money))) => (shop_money, frozen_money),
            _ => return Err(Error::NotFound("店铺数据不存在".into())),
        };

        sqlx::query(
            "UPDATE \"shop\" SET frozen_money = frozen_money - $1 \
             WHERE shop_id = $2",
        )
        .bind(withdraw.amount)
        .bind(withdraw.shop_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        let new_shop = old_shop;
        let new_frozen = old_frozen - withdraw.amount;

        sqlx::query(
            "INSERT INTO shop_account_log \
                (add_time, shop_money, frozen_money, \"type\", remarks, \
                 new_shop_money, new_frozen_money, shop_id, is_new, balance_type, change_amount) \
             VALUES ($1, $2, $3, '1', '线下提现打款完成', $4, $5, $6, 1, 1, $7)",
        )
        .bind(now_seconds() as i64)
        .bind(old_shop)
        .bind(old_frozen)
        .bind(new_shop)
        .bind(new_frozen)
        .bind(withdraw.shop_id)
        .bind(withdraw.amount)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;

        tx.commit().await.map_err(map_err)?;
        Ok(true)
    }

    // ===== 店铺经营类目（\"shop_product_category\"） =====

    /// 经营类目分页（`ShopProductCategoryServiceImpl#list`）。
    pub async fn paginate_product_categories(
        &self,
        parent_id: Option<i32>,
        shop_id: Option<i32>,
        keyword: Option<&str>,
        sort_field: Option<&str>,
        sort_order: Option<&str>,
        page: &PageQuery,
    ) -> Result<(u64, Vec<ShopProductCategory>)> {
        let where_sql = "
            WHERE (? IS NULL OR parent_id = ?)
              AND (? IS NULL OR shop_id = ?)
              AND (? IS NULL OR category_name LIKE CONCAT('%', ?, '%'))";
        let order = order_by(
            sort_field,
            sort_order,
            "category_id",
            &[
                ("categoryId", "category_id"),
                ("parentId", "parent_id"),
                ("sortOrder", "sort_order"),
                ("addTime", "add_time"),
                ("categoryName", "category_name"),
            ],
        );

        let count_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT COUNT(*) FROM shop_product_category {where_sql}"
        ));
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(parent_id)
            .bind(parent_id)
            .bind(shop_id)
            .bind(shop_id)
            .bind(keyword)
            .bind(keyword)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let (limit, offset) = page.limit_offset();
        let rows_sql =
            crate::db::to_pg_placeholders(&format!("SELECT * FROM shop_product_category {where_sql} {order} LIMIT ? OFFSET ?"));
        let rows = sqlx::query_as::<_, ShopProductCategory>(&rows_sql)
            .bind(parent_id)
            .bind(parent_id)
            .bind(shop_id)
            .bind(shop_id)
            .bind(keyword)
            .bind(keyword)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.0.max(0) as u64, rows))
    }

    /// 按 ID 查经营类目。
    pub async fn find_product_category(
        &self,
        id: i32,
    ) -> Result<Option<ShopProductCategory>> {
        sqlx::query_as::<_, ShopProductCategory>(
            "SELECT * FROM shop_product_category WHERE category_id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 批量统计下级数量（列表页 `hasChildren`，替代逐行 count）。
    pub async fn count_children_map(&self, ids: &[i32]) -> Result<HashMap<i32, i64>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT parent_id, COUNT(*) FROM shop_product_category
             WHERE parent_id IN ({}) GROUP BY parent_id",
            placeholders(ids.len())
        );
        let sql = crate::db::to_pg_placeholders(&sql);
        let mut q = sqlx::query_as::<_, (i32, i64)>(&sql);
        for id in ids {
            q = q.bind(id);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(map_err)?;
        Ok(rows.into_iter().collect())
    }

    /// 经营类目树数据（`is_show = 1`，按 `parent_id, sort_order, category_id` 升序）。
    pub async fn find_categories_for_tree(
        &self,
        shop_id: i32,
    ) -> Result<Vec<ShopProductCategory>> {
        sqlx::query_as::<_, ShopProductCategory>(
            "SELECT * FROM shop_product_category
             WHERE shop_id = $1 AND is_show = 1
             ORDER BY parent_id ASC, sort_order ASC, category_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }
}

/// 当前秒级时间戳（`shop_account_log.add_time` 等库列单位为秒）。
fn now_seconds() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0)
}

/// 排序白名单：属性名/列名 → 列名；未知回落主键，方向非 `asc` 即 \"desc\"。
/// 对齐原 `BaseServiceImpl#buildSortOrder` + `getColumnByPropertyName`。
fn order_by(
    field: Option<&str>,
    order: Option<&str>,
    key: &str,
    pairs: &[(&str, &str)],
) -> String {
    let f = field.unwrap_or("").trim();
    let col = pairs
        .iter()
        .find(|(p, c)| p.eq_ignore_ascii_case(f) || c.eq_ignore_ascii_case(f))
        .map(|(_, c)| *c)
        .unwrap_or(key);
    let dir = if order
        .map(|o| o.eq_ignore_ascii_case("asc"))
        .unwrap_or(false)
    {
        "ASC"
    } else {
        "DESC"
    };
    format!("ORDER BY \"{col}\" {dir}")
}

/// `IN (?,?,?)` 占位符（长度由调用方控制，恒为合法 SQL 片段）。
fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("店铺数据不存在".into()),
        other => Error::Database(other.to_string()),
    }
}