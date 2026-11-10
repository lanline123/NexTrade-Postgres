//! 后台消息域仓储（system::admin_msg）。
//!
//! 表：`admin_msg`（14 列）。对应 Java `AdminMsgServiceImpl` 的
//! 身份可见性口径（平台/店铺/供应商）与分页/未读统计/已读更新。
//! 运行时 `sqlx`（不用 `query!` 宏）；`shop_id`/`vendor_id`/`msg_type` 为 `i32`
//! 直接内插（无注入面），`keyword` 统一 `?` 绑定。

use sqlx::PgPool;
use nextrade_core::models::system::admin_msg::{AdminMsgCountVO, AdminMsgListQuery, MsgScope};
use nextrade_core::{Error, Result};

/// 消息行（映射 `admin_msg` 14 列）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminMsgRow {
    pub msg_id: i32,
    pub msg_type: i32,
    pub send_time: i32,
    pub is_readed: i32,
    pub shop_id: i32,
    pub admin_id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub title: String,
    pub content: String,
    pub msg_link: Option<String>,
    pub related_data: Option<String>,
    pub is_broadcast: i32,
    pub vendor_id: Option<i32>,
}

/// 消息列表投影列。
const COLUMNS: &str = "msg_id, msg_type, send_time, is_readed, shop_id, admin_id, order_id, product_id, title, content, msg_link, related_data, is_broadcast, vendor_id";

/// 后台消息域仓储。
#[derive(Clone)]
pub struct AdminMsgRepository {
    pool: PgPool,
}

impl AdminMsgRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 查询管理员消息与业务指标统计（对齐 Java SseServiceImpl#msgCount）
    pub async fn get_msg_count_vo(
        &self,
        admin_id: i32,
        scope: MsgScope,
        start_time: i64,
    ) -> Result<AdminMsgCountVO> {
        let now = nextrade_common::utils::now_seconds() as i64;

        // 1. IM 统计
        let shop_id = match scope {
            MsgScope::Shop { shop_id, .. } => shop_id,
            MsgScope::Vendor { .. } => 0,
        };
        let im_msg_count: i64 = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM im_message WHERE shop_id = $1 AND servant_id = $2 AND is_read = 0 AND \"type\" = 1 AND user_id > 0",
        )
        .bind(shop_id)
        .bind(admin_id)
        .fetch_one(&self.pool)
        .await
        .map(|r| r.0)
        .unwrap_or(0);

        let customer_msg_count: i64 = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM im_message WHERE shop_id = $1 AND servant_id = $2 AND \"type\" = 1 AND user_id > 0 AND send_time >= $3",
        )
        .bind(shop_id)
        .bind(admin_id)
        .bind(start_time)
        .fetch_one(&self.pool)
        .await
        .map(|r| r.0)
        .unwrap_or(0);

        // 2. 订单与售后统计
        let (order_count, paid_order_count, cancel_order_count, aftersale_count) = match scope {
            MsgScope::Vendor { vendor_id } => {
                let oc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM \"order\" WHERE vendor_id = $1 AND add_time >= $2",
                )
                .bind(vendor_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                let poc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM \"order\" WHERE vendor_id = $1 AND pay_status = 2 AND pay_time >= $2",
                )
                .bind(vendor_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                let coc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM \"order\" WHERE vendor_id = $1 AND order_status = 2 AND add_time >= $2",
                )
                .bind(vendor_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                let asc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM aftersales WHERE vendor_id = $1 AND add_time >= $2",
                )
                .bind(vendor_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                (oc, poc, coc, asc)
            }
            MsgScope::Shop { shop_id, .. } => {
                let oc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM \"order\" WHERE shop_id = $1 AND add_time >= $2",
                )
                .bind(shop_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                let poc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM \"order\" WHERE shop_id = $1 AND pay_status = 2 AND pay_time >= $2",
                )
                .bind(shop_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                let coc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM \"order\" WHERE shop_id = $1 AND order_status = 2 AND add_time >= $2",
                )
                .bind(shop_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                let asc: i64 = sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM aftersales WHERE shop_id = $1 AND add_time >= $2",
                )
                .bind(shop_id)
                .bind(start_time)
                .fetch_one(&self.pool)
                .await
                .map(|r| r.0)
                .unwrap_or(0);

                (oc, poc, coc, asc)
            }
        };

        // 3. 管理员未读消息总数
        let where_sql = build_where(scope, &AdminMsgListQuery::default());
        let unread_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM admin_msg{where_sql} AND is_readed = 0"));
        let unread_msg_count: i64 = sqlx::query_as::<_, (i64,)>(&unread_sql)
            .fetch_one(&self.pool)
            .await
            .map(|r| r.0)
            .unwrap_or(0);

        // 4. 语音播报4类增量消息计数 (11 新订单 / 12 已付款 / 31 取消 / 32 售后)
        let broadcast_sql = crate::db::to_pg_placeholders(&format!(
            "SELECT msg_type, COUNT(*) FROM admin_msg{where_sql} AND is_broadcast = 1 AND send_time >= ? AND msg_type IN (11, 12, 31, 32) GROUP BY msg_type"
        ));
        let broadcast_rows: Vec<(i32, i64)> = sqlx::query_as::<_, (i32, i64)>(&broadcast_sql)
            .bind(start_time)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        let mut new_order_msg_count = 0;
        let mut paid_order_msg_count = 0;
        let mut cancel_order_msg_count = 0;
        let mut aftersale_msg_count = 0;
        for (m_type, count) in broadcast_rows {
            match m_type {
                11 => new_order_msg_count = count,
                12 => paid_order_msg_count = count,
                31 => cancel_order_msg_count = count,
                32 => aftersale_msg_count = count,
                _ => {}
            }
        }

        Ok(AdminMsgCountVO {
            im_msg_count,
            customer_msg_count,
            order_count,
            unread_msg_count,
            paid_order_count,
            aftersale_count,
            cancel_order_count,
            new_order_msg_count,
            paid_order_msg_count,
            aftersale_msg_count,
            cancel_order_msg_count,
            ts: now,
            version: 4,
            admin_id,
        })
    }

    /// 消息分页（Java `list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn paginate(
        &self,
        scope: MsgScope,
        q: &AdminMsgListQuery,
    ) -> Result<(u64, Vec<AdminMsgRow>)> {
        let where_sql = build_where(scope, q);
        let (page, size) = q.page_params();
        let sort = sort_column(q);
        let like = like_value(q);

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM admin_msg{where_sql}"));
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(v) = like.as_deref() {
            cq = cq.bind(v);
        }
        let total: (i64,) = cq.fetch_one(&self.pool).await.map_err(map_err)?;

        let sql =
            crate::db::to_pg_placeholders(&format!("SELECT {COLUMNS} FROM admin_msg{where_sql} ORDER BY {sort} LIMIT ? OFFSET ?"));
        let mut lq = sqlx::query_as::<_, AdminMsgRow>(&sql);
        if let Some(v) = like.as_deref() {
            lq = lq.bind(v);
        }
        let rows = lq
            .bind(i64::try_from(size).unwrap_or(15))
            .bind(i64::try_from((page - 1) * size).unwrap_or(0))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((u64::try_from(total.0).unwrap_or(0), rows))
    }

    /// 按消息类型统计未读数（Java `getAdminMsgTypeList` 未读口径）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn unread_by_type(&self, scope: MsgScope) -> Result<Vec<(i32, i64)>> {
        let where_sql = build_where(scope, &AdminMsgListQuery::default());
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT msg_type, COUNT(*) FROM admin_msg{where_sql} AND is_readed = 0 GROUP BY msg_type"
        ));
        sqlx::query_as::<_, (i32, i64)>(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 最新一条（Java `getHomeSummary`：send_time/msg_id 倒序取 1）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn latest(&self, scope: MsgScope) -> Result<Option<AdminMsgRow>> {
        let where_sql = build_where(scope, &AdminMsgListQuery::default());
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT {COLUMNS} FROM admin_msg{where_sql} ORDER BY send_time DESC, msg_id DESC LIMIT 1"
        ));
        sqlx::query_as::<_, AdminMsgRow>(&sql)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 单条已读（Java `setReaded`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    /// 按 ID 查询消息（归属校验用）。
    pub async fn find_by_id(&self, msg_id: i32) -> Result<Option<AdminMsgRow>> {
        let sql = format!("SELECT {COLUMNS} FROM admin_msg WHERE msg_id = $1 LIMIT 1");
        sqlx::query_as::<_, AdminMsgRow>(&sql)
            .bind(msg_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn set_readed(&self, msg_id: i32) -> Result<bool> {
        let r = sqlx::query("UPDATE admin_msg SET is_readed = 1 WHERE msg_id = $1")
            .bind(msg_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 同身份可见范围全部已读（Java `setAllReaded`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn set_all_readed(&self, scope: MsgScope) -> Result<u64> {
        let where_sql = build_where(scope, &AdminMsgListQuery::default());
        let sql = crate::db::to_pg_placeholders(&format!("UPDATE admin_msg SET is_readed = 1{where_sql} AND is_readed = 0"));
        Ok(sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(map_err)?
            .rows_affected())
    }

    /// 插入管理员通知消息（对齐 Java AdminMsgServiceImpl#createMessage）
    pub async fn create_message(
        &self,
        msg_type: i32,
        shop_id: i32,
        order_id: i32,
        title: &str,
        content: &str,
        related_data: &str,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let is_broadcast = if shop_id == 0 { 1 } else { 0 };
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_msg \
             (msg_type, send_time, is_readed, shop_id, admin_id, order_id, product_id, title, content, msg_link, related_data, is_broadcast, vendor_id) \
             VALUES ($1, $2, 0, $3, 0, $4, 0, $5, $6, '', $7, $8, 0) RETURNING msg_id",
        )
        .bind(msg_type)
        .bind(now)
        .bind(shop_id)
        .bind(order_id)
        .bind(title)
        .bind(content)
        .bind(related_data)
        .bind(is_broadcast)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 插入供应商维度的管理员通知消息（Java `AdminMsgCreateDTO.vendorId`）。
    pub async fn create_vendor_message(
        &self,
        msg_type: i32,
        vendor_id: i32,
        shop_id: i32,
        order_id: i32,
        title: &str,
        content: &str,
        related_data: &str,
    ) -> Result<i32> {
        let now = nextrade_common::utils::now_seconds() as i32;
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO admin_msg \
             (msg_type, send_time, is_readed, shop_id, admin_id, order_id, product_id, title, content, msg_link, related_data, is_broadcast, vendor_id) \
             VALUES ($1, $2, 0, $3, 0, $4, 0, $5, $6, '', $7, 0, $8) RETURNING msg_id",
        )
        .bind(msg_type)
        .bind(now)
        .bind(shop_id)
        .bind(order_id)
        .bind(title)
        .bind(content)
        .bind(related_data)
        .bind(vendor_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }
}

/// 可见性 + 可选过滤 WHERE（含前导空格）。
///
/// 供应商 → `vendor_id = {id}`；其它 → `shop_id = {id} AND vendor_id = 0`；
/// 平台额外 `is_broadcast = 1`；\"msg_type\" 可选；\"keyword\" → `title ILIKE $2`。
#[must_use]
fn build_where(scope: MsgScope, q: &AdminMsgListQuery) -> String {
    let mut conds: Vec<String> = Vec::new();
    match scope {
        MsgScope::Vendor { vendor_id } => conds.push(format!("vendor_id = {vendor_id}")),
        MsgScope::Shop { shop_id, .. } => {
            conds.push(format!("shop_id = {shop_id} AND vendor_id = 0"));
        }
    }
    if scope.is_platform() {
        conds.push("is_broadcast = 1".to_string());
    }
    if let Some(t) = q.msg_type {
        conds.push(format!("msg_type = {t}"));
    }
    if like_value(q).is_some() {
        conds.push("title ILIKE ?".to_string());
    }
    format!(" WHERE {}", conds.join(" AND "))
}

/// 关键字 ILIKE 绑定值。
#[must_use]
fn like_value(q: &AdminMsgListQuery) -> Option<String> {
    q.keyword
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"))
}

/// 排序字段（Java `buildSortOrder`；白名单，未知回退 `msg_id`）。
#[must_use]
fn sort_column(q: &AdminMsgListQuery) -> String {
    const SORTABLE: [(&str, &str); 4] = [
        ("msgId", "msg_id"),
        ("sendTime", "send_time"),
        ("isReaded", "is_readed"),
        ("msgType", "msg_type"),
    ];
    let field = q
        .sort_field
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("msg_id");
    let column = SORTABLE
        .iter()
        .find(|(p, c)| *p == field || *c == field)
        .map_or("msg_id", |(_, c)| *c);
    let dir = if q
        .sort_order
        .as_deref()
        .is_some_and(|o| o.eq_ignore_ascii_case("asc"))
    {
        "ASC"
    } else {
        "DESC"
    };
    format!("{column} {dir}")
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn where_clause_matches_java_visibility() {
        let shop = MsgScope::Shop { shop_id: 3, platform: false };
        assert_eq!(
            build_where(shop, &AdminMsgListQuery::default()),
            " WHERE shop_id = 3 AND vendor_id = 0"
        );
        let platform = MsgScope::Shop { shop_id: 0, platform: true };
        assert_eq!(
            build_where(platform, &AdminMsgListQuery::default()),
            " WHERE shop_id = 0 AND vendor_id = 0 AND is_broadcast = 1"
        );
        let vendor = MsgScope::Vendor { vendor_id: 9 };
        assert_eq!(build_where(vendor, &AdminMsgListQuery::default()), " WHERE vendor_id = 9");

        let q = AdminMsgListQuery {
            msg_type: Some(11),
            keyword: Some("订单".into()),
            ..Default::default()
        };
        assert_eq!(
            build_where(vendor, &q),
            " WHERE vendor_id = 9 AND msg_type = 11 AND title ILIKE ?"
        );
        assert_eq!(like_value(&q).as_deref(), Some("%订单%"));
    }

    #[test]
    fn sort_column_is_whitelisted() {
        let q = AdminMsgListQuery::default();
        assert_eq!(sort_column(&q), "msg_id DESC");
        let q = AdminMsgListQuery {
            sort_field: Some("sendTime".into()),
            sort_order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(sort_column(&q), "send_time ASC");
        let q = AdminMsgListQuery {
            sort_field: Some("x; DROP TABLE admin_msg".into()),
            ..Default::default()
        };
        assert_eq!(sort_column(&q), "msg_id DESC");
    }
}
