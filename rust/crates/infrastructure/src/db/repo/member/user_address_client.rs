//! 用户收货地址仓储：`user_address`，兼查 `region` 解析区名。
//!
//! 对应原 Java `UserAddressMapper` / `RegionMapper`（仅用到的部分）。
//!
//! 列名严格对齐 `java/database/pure.sql`（`python tools/schema_parser.py
//! --table user_address` 核对）：27 列，主键 `address_id`。
//!
//! 列表过滤逐字复刻 `UserAddressServiceImpl#list`：
//! `user_id = ?` + `(is_system = 0 OR is_system IS NULL)`（控制器固定
//! `isSystem = 0`）；排序缺省 `is_selected DESC, is_default DESC`，
//! 客户端显式 `sortField` 时单列排序（Java `buildSortOrder`）。
//!
//! 事务：`create` / `update` / `setSelected` 在 Java 里是「先清默认/选中，再写
//! 当前行」的多语句操作，本实现收敛在仓储层用 `pool.begin()` 保证原子性。

use rust_decimal::Decimal;
use sqlx::{Postgres,PgPool, QueryBuilder};
use nextrade_core::entities::UserAddress;
use nextrade_core::models::member::user_address_client::{
    normalize_column, UserAddressListQuery, SORTABLE_COLUMNS,
};
use nextrade_core::{Error, Result};

/// `user_address` 表全部列（顺序与 [`UserAddress`] 字段一致）。
const ADDRESS_COLUMNS: &str = "address_id, country_code, country_name, address_tag, user_id, \
     consignee, email, region_ids, region_names, address, address_line_two, building_name, floor, \
     unit, address_data, formatted_address, position_address, postcode, telephone, mobile, \
     is_default, is_system, is_selected, mobile_area_code, international_mobile, latitude, longitude";

#[derive(Clone)]
pub struct UserAddressRepository {
    pool: PgPool,
}

impl UserAddressRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（服务层无跨语句事务需求，事务都在本仓储内完成）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 读 —————————————————————

    /// 收货地址分页（对应 Java `UserAddressServiceImpl#list`）。
    ///
    /// \"sort\" 为 \"None\" → Java 控制器注入的默认两列排序
    /// （`is_selected DESC, is_default DESC`）。
    pub async fn paginate(
        &self,
        user_id: i32,
        sort: Option<(&'static str, Option<bool>)>,
        q: &UserAddressListQuery,
    ) -> Result<(u64, Vec<UserAddress>)> {
        let page = q.page_query();
        let (limit, offset) = page.limit_offset();
        // count 与取行共用同一 WHERE
        let where_sql = " WHERE user_id = ? AND (is_system = 0 OR is_system IS NULL)";

        let count_sql = format!("SELECT COUNT(*) FROM user_address{where_sql}");
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let order = order_clause(sort);
        let list_sql = format!(
            "SELECT {ADDRESS_COLUMNS} FROM user_address{where_sql}{order} LIMIT ? OFFSET ?"
        );
        let rows = sqlx::query_as::<_, UserAddress>(&list_sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0.max(0) as u64, rows))
    }

    /// 按主键查询（Java `getById`）。
    pub async fn find_by_id(&self, address_id: i32) -> Result<Option<UserAddress>> {
        let sql = format!("SELECT {ADDRESS_COLUMNS} FROM user_address WHERE address_id = ? LIMIT 1");
        sqlx::query_as::<_, UserAddress>(&sql)
            .bind(address_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按地区 ID 查区名（Java `regionService.list(in regionIds)`）。
    ///
    /// Java 未显式排序；本实现按 `region_id ASC` 稳定输出（见 models 差异 1）。
    pub async fn find_region_names(&self, region_ids: &[i32]) -> Result<Vec<String>> {
        if region_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb: QueryBuilder<'_, Postgres> =
            QueryBuilder::new("SELECT region_name FROM region WHERE region_id IN (");
        let mut sep = qb.separated(", ");
        for id in region_ids {
            sep.push_bind(*id);
        }
        sep.push_unseparated(") ORDER BY region_id ASC");
        drop(sep);
        let rows: Vec<(String,)> = qb
            .build_query_as()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    // ————————————————————— 写 —————————————————————

    /// 新增地址（Java `#create`，`@Transactional`）。
    ///
    /// - `clear_default_user = Some(uid)`：插入前把该用户其它地址 `is_default = 0`
    /// - `set_selected = true`：插入后把该用户其它地址 `is_selected = 0`，并把
    ///   新行 `is_selected = 1`（Java `setSelected(newId)`）
    pub async fn create(
        &self,
        entity: &UserAddress,
        clear_default_user: Option<i32>,
        set_selected: bool,
        current_user: i32,
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if let Some(uid) = clear_default_user {
            sqlx::query("UPDATE user_address SET is_default = 0 WHERE user_id = ?")
                .bind(uid)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user_address \
             (country_code, country_name, address_tag, user_id, consignee, email, region_ids, \
              region_names, address, address_line_two, building_name, floor, unit, address_data, \
              formatted_address, position_address, postcode, telephone, mobile, is_default, \
              is_system, is_selected, mobile_area_code, international_mobile, latitude, longitude) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING address_id",
        )
        .bind(&entity.country_code)
        .bind(&entity.country_name)
        .bind(&entity.address_tag)
        .bind(entity.user_id)
        .bind(&entity.consignee)
        .bind(&entity.email)
        .bind(&entity.region_ids)
        .bind(&entity.region_names)
        .bind(&entity.address)
        .bind(&entity.address_line_two)
        .bind(&entity.building_name)
        .bind(&entity.floor)
        .bind(&entity.unit)
        .bind(&entity.address_data)
        .bind(&entity.formatted_address)
        .bind(&entity.position_address)
        .bind(&entity.postcode)
        .bind(&entity.telephone)
        .bind(&entity.mobile)
        .bind(entity.is_default)
        .bind(entity.is_system)
        .bind(entity.is_selected)
        .bind(&entity.mobile_area_code)
        .bind(&entity.international_mobile)
        .bind(entity.latitude)
        .bind(entity.longitude)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_err)?;
        let new_id = r;
        if set_selected {
            sqlx::query("UPDATE user_address SET is_selected = 0 WHERE user_id = ?")
                .bind(current_user)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            sqlx::query("UPDATE user_address SET is_selected = 1 WHERE address_id = ? AND user_id = ?")
                .bind(new_id)
                .bind(current_user)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(new_id)
    }

    /// 整行更新（Java \"updateById\"；未提交字段由服务层合并原值）。
    ///
    /// \"clear_default_of\"：Java 在「本次提交 isDefault > 0」时先把当前用户其它
    /// 地址默认置 0（`ne address_id`）。
    pub async fn update(
        &self,
        entity: &UserAddress,
        clear_default_of: Option<(i32, i32)>,
    ) -> Result<u64> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        if let Some((uid, exclude_id)) = clear_default_of {
            sqlx::query(
                "UPDATE user_address SET is_default = 0 \
                 WHERE user_id = ? AND address_id <> ?",
            )
            .bind(uid)
            .bind(exclude_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        let r = sqlx::query(
            "UPDATE user_address SET country_code = ?, country_name = ?, address_tag = ?, \
                user_id = ?, consignee = ?, email = ?, region_ids = ?, region_names = ?, \
                address = ?, address_line_two = ?, building_name = ?, floor = ?, unit = ?, \
                address_data = ?, formatted_address = ?, position_address = ?, postcode = ?, \
                telephone = ?, mobile = ?, is_default = ?, is_system = ?, is_selected = ?, \
                mobile_area_code = ?, international_mobile = ?, latitude = ?, longitude = ? \
             WHERE address_id = ?",
        )
        .bind(&entity.country_code)
        .bind(&entity.country_name)
        .bind(&entity.address_tag)
        .bind(entity.user_id)
        .bind(&entity.consignee)
        .bind(&entity.email)
        .bind(&entity.region_ids)
        .bind(&entity.region_names)
        .bind(&entity.address)
        .bind(&entity.address_line_two)
        .bind(&entity.building_name)
        .bind(&entity.floor)
        .bind(&entity.unit)
        .bind(&entity.address_data)
        .bind(&entity.formatted_address)
        .bind(&entity.position_address)
        .bind(&entity.postcode)
        .bind(&entity.telephone)
        .bind(&entity.mobile)
        .bind(entity.is_default)
        .bind(entity.is_system)
        .bind(entity.is_selected)
        .bind(&entity.mobile_area_code)
        .bind(&entity.international_mobile)
        .bind(entity.latitude)
        .bind(entity.longitude)
        .bind(entity.address_id)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 设置选中地址（Java `#setSelected`，`@Transactional`）：先清后设。
    pub async fn set_selected(&self, user_id: i32, address_id: i32) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        sqlx::query("UPDATE user_address SET is_selected = 0 WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        sqlx::query("UPDATE user_address SET is_selected = 1 WHERE address_id = ? AND user_id = ?")
            .bind(address_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 删除单条（Java \"del\" → \"removeById\"）。
    pub async fn delete(&self, address_id: i32) -> Result<u64> {
        let r = sqlx::query("DELETE FROM user_address WHERE address_id = ?")
            .bind(address_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// 列表排序子句。
///
/// - \"None\"（客户端未传 sortField）→ Java 控制器默认：
///   `ORDER BY is_selected DESC, is_default DESC`
/// - `Some(column, desc)` → 单列；`desc == None` 时 Java 不设置 ORDER BY
fn order_clause(sort: Option<(&'static str, Option<bool>)>) -> String {
    match sort {
        None => " ORDER BY is_selected DESC, is_default DESC".to_string(),
        Some((column, Some(true))) => format!(" ORDER BY {column} DESC"),
        Some((column, Some(false))) => format!(" ORDER BY {column} ASC"),
        Some((_, None)) => String::new(),
    }
}

/// 解析列表排序（供仓储单测；复用 models 白名单）。
#[must_use]
pub fn resolve_list_sort(
    sort_field: Option<&str>,
    sort_order: Option<&str>,
) -> Option<(&'static str, Option<bool>)> {
    let field = sort_field.map(str::trim).filter(|s| !s.is_empty())?;
    let column = normalize_column(field);
    let column = SORTABLE_COLUMNS
        .iter()
        .copied()
        .find(|c| *c == column)
        .unwrap_or("address_id");
    let order = sort_order.map(str::trim).filter(|s| !s.is_empty());
    let desc = match order {
        None => Some(true),
        Some(o) if o.eq_ignore_ascii_case("asc") => Some(false),
        Some(o) if o.eq_ignore_ascii_case("desc") => Some(true),
        Some(_) => None,
    };
    Some((column, desc))
}

/// \"f64\" → \"Decimal\"（Java \"BigDecimal\"；精度过低时为 \"None\"）。
#[must_use]
pub fn decimal_from_f64(value: Option<f64>) -> Option<Decimal> {
    value.and_then(|v| {
        if v.is_finite() {
            Decimal::try_from(v).ok()
        } else {
            None
        }
    })
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("收货地址不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_order_matches_java_controller() {
        assert_eq!(
            order_clause(None),
            " ORDER BY is_selected DESC, is_default DESC"
        );
        assert_eq!(
            order_clause(Some(("is_default", Some(false)))),
            " ORDER BY is_default ASC"
        );
        assert_eq!(order_clause(Some(("address_id", None))), "");
    }

    #[test]
    fn resolve_list_sort_normalizes_and_falls_back() {
        assert_eq!(resolve_list_sort(None, None), None);
        assert_eq!(
            resolve_list_sort(Some("isDefault"), Some("ASC")),
            Some(("is_default", Some(false)))
        );
        assert_eq!(
            resolve_list_sort(Some("unknown"), None),
            Some(("address_id", Some(true)))
        );
    }

    #[test]
    fn decimal_conversion_handles_invalid_values() {
        assert!(decimal_from_f64(None).is_none());
        assert!(decimal_from_f64(Some(f64::NAN)).is_none());
        assert!(decimal_from_f64(Some(f64::INFINITY)).is_none());
        use rust_decimal::prelude::ToPrimitive;
        let lat = decimal_from_f64(Some(39.9)).unwrap();
        assert!((lat.to_f64().unwrap() - 39.9).abs() < 1e-9);
    }
}
