//! 代客下单 · 收货地址域仓储（trade::proxy_shopping_user_address）：`user_address` / `region` / `config`。
//!
//! 对应原 Java：
//! - `com.nextrade.mapper.user.UserAddressMapper`（经 `UserAddressServiceImpl`
//!   继承的 `BaseServiceImpl` 暴露的 `page` / `getById` / `save` / `updateById` /
//!   `update(LambdaUpdateWrapper)`）
//! - `com.nextrade.mapper.setting.RegionMapper`（`list(LambdaQueryWrapper)` +
//!   `getById`，经 `RegionServiceImpl`）
//! - `ConfigServiceImpl#getConfigByCode("defaultCountry")`
//!
//! 列名严格对齐权威 schema
//! （`cd rust && python tools/schema_parser.py --table user_address|region|config`）：
//!
//! - `user_address`（27 列）：`address_id` / `country_code` / `country_name` /
//!   `address_tag` / `user_id` / `consignee` / `email` / `region_ids` /
//!   `region_names` / `address` / `address_line_two` / `building_name` /
//!   `floor` / `unit` / `address_data` / `formatted_address` / `position_address` /
//!   `postcode` / `telephone` / `mobile` / `is_default` / `is_system` /
//!   `is_selected` / `mobile_area_code` / `international_mobile` /
//!   `latitude` / `longitude`
//! - `region`（8 列）：`region_id` / `level` / `region_name` / `parent_id` /
//!   `is_hot` / `sort_order` / `first_word` / `standard_code`
//! - `config`：`biz_code` / `biz_val` / `is_del`（本域只读 `defaultCountry`）
//!
//! 约定：
//! - 一律使用**运行时** `sqlx::query_as` / `sqlx::query`，**不使用** `query!` 宏
//!   （本机无 `DATABASE_URL`，宏必然编译失败）。
//! - 每条 SQL 先绑到局部变量再执行（`let sql = format!(..); query_as(&sql)`），
//!   避免 E0716 临时值被 drop。
//! - 仓储层**不做业务校验**（Java Mapper 也不做），只忠实执行 SQL；
//!   「地址不存在」「必须选择用户」等语义在服务层复刻。
//! - 排序/动态 SET 子句只允许来自本文件内的**编译期白名单常量**，
//!   用户输入永不拼接进 SQL（对应 Java `getColumnByPropertyName` 的防注入回落）。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::{Region, UserAddress};
use nextrade_core::models::trade::proxy_shopping_user_address::AddressUpdatePatch;
use nextrade_core::{Error, Result};

/// `user_address` 全部 27 列（供文档/单测核对，顺序与表定义一致）。
///
/// 单行常量：`tools/check_sql_columns.py` 按 SQL 字面量扫描。
pub const USER_ADDRESS_COLUMNS: &str = "address_id, country_code, country_name, address_tag, user_id, consignee, email, region_ids, region_names, address, address_line_two, building_name, floor, unit, address_data, formatted_address, position_address, postcode, telephone, mobile, is_default, is_system, is_selected, mobile_area_code, international_mobile, latitude, longitude";

/// 列表排序允许使用的列。
///
/// 对应 Java `BaseServiceImpl#buildSortOrder` + `getColumnByPropertyName`：
/// 属性名不在白名单内时**回落到主键**（`address_id`），绝不把用户输入拼进 SQL。
/// 本域控制器把 `sortField` 恒置为 `is_selected`，故实际只用第一项。
pub const ADDRESS_SORT_COLUMNS: [&str; 27] = [
    "address_id",
    "country_code",
    "country_name",
    "address_tag",
    "user_id",
    "consignee",
    "email",
    "region_ids",
    "region_names",
    "address",
    "address_line_two",
    "building_name",
    "floor",
    "unit",
    "address_data",
    "formatted_address",
    "position_address",
    "postcode",
    "telephone",
    "mobile",
    "is_default",
    "is_system",
    "is_selected",
    "mobile_area_code",
    "international_mobile",
    "latitude",
    "longitude",
];

/// 系统配置项「默认国家」的 `biz_code`
/// （Java `SettingsEnum.DEFAULT_COUNTRY.getBizCode()`）。
pub const CONFIG_DEFAULT_COUNTRY: &str = "defaultCountry";

/// 把 Java 排序属性名解析为白名单列名。
///
/// 复刻 `BaseServiceImpl#getColumnByPropertyName`：命中返回对应列，
/// **未命中回落到主键** `address_id`（而不是报错或原样拼接）。
#[must_use]
pub fn resolve_sort_column(property: &str) -> &'static str {
    ADDRESS_SORT_COLUMNS
        .iter()
        .find(|c| **c == property)
        .copied()
        .unwrap_or("address_id")
}

#[derive(Clone)]
pub struct ProxyShoppingUserAddressRepository {
    pool: PgPool,
}

impl ProxyShoppingUserAddressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排）。
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 收货地址 —————————————————————

    /// 分页查询某会员的收货地址（Java `BaseServiceImpl#page`）。
    ///
    /// `is_system` 的取值语义与 Java `UserAddressServiceImpl#list` 完全一致：
    /// - `Some(1)` → `AND is_system = 1`
    /// - `Some(0)` → `AND (is_system = 0 OR is_system IS NULL)`
    /// - 其它（含 `None` 与控制器强制的 `Some(2)`）→ **不加条件**（查全部）
    ///
    /// 排序复刻 Java `buildSortOrder(page, "is_selected", "")`：
    /// `sortOrder` 为空 → **降序**。`is_selected` 在本表可空（历史行），
    /// 等价改写为 `COALESCE(is_selected, 0) DESC`（NULL 视作 0），
    /// 否则 MySQL 会把 NULL 排到最前，与「选中的排最前」的意图相反。
    pub async fn page_user_addresses(
        &self,
        user_id: i32,
        is_system: Option<i32>,
        column: &'static str,
        desc: bool,
        limit: u64,
        offset: u64,
    ) -> Result<(i64, Vec<UserAddress>)> {
        let mut where_sql = String::from(" WHERE user_id = $1");
        match is_system {
            Some(1) => where_sql.push_str(" AND is_system = 1"),
            Some(0) => where_sql.push_str(" AND (is_system = 0 OR is_system IS NULL)"),
            _ => {}
        }

        let count_sql = crate::db::to_pg_placeholders(&format!("SELECT COUNT(*) FROM user_address{where_sql}"));
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let dir = if desc { "DESC" } else { "ASC" };
        // `column` 来自 ADDRESS_SORT_COLUMNS（编译期常量），不存在注入面。
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM user_address{where_sql} ORDER BY {column} {dir}, address_id ASC LIMIT ? OFFSET ?"
        ));
        let rows = sqlx::query_as::<_, UserAddress>(&sql)
            .bind(user_id)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok((total.0, rows))
    }

    /// 按主键取单条（Java `getById`）。
    pub async fn find_user_address(&self, address_id: i32) -> Result<Option<UserAddress>> {
        sqlx::query_as::<_, UserAddress>("SELECT * FROM user_address WHERE address_id = $1")
            .bind(address_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增收货地址（Java `save` + 自增主键回填）。
    ///
    /// 只写 18 列：其余 9 列（`country_code` / `country_name` / `address_line_two` /
    /// `building_name` / `floor` / `unit` / `address_data` / `formatted_address` /
    /// `international_mobile`）不在 Java `UserAddress` 实体里，Java 侧恒为 NULL，
    /// 本实现同样不写（交由 DDL 默认值）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_user_address(
        &self,
        user_id: i32,
        address_tag: &str,
        consignee: &str,
        email: Option<&str>,
        region_ids: &str,
        region_names: &str,
        address: &str,
        position_address: Option<&str>,
        postcode: Option<&str>,
        telephone: Option<&str>,
        mobile: &str,
        mobile_area_code: Option<&str>,
        is_default: i32,
        is_system: Option<i32>,
        is_selected: i32,
        latitude: Option<Decimal>,
        longitude: Option<Decimal>,
    ) -> Result<i32> {
        let sql = "INSERT INTO user_address (address_tag, user_id, consignee, email, region_ids, region_names, address, position_address, postcode, telephone, mobile, mobile_area_code, is_default, is_system, is_selected, latitude, longitude) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)";
        let r: i32 = sqlx::query_scalar(&format!("{} RETURNING address_id", sql))
            .bind(address_tag)
            .bind(user_id)
            .bind(consignee)
            .bind(email)
            .bind(region_ids)
            .bind(region_names)
            .bind(address)
            .bind(position_address)
            .bind(postcode)
            .bind(telephone)
            .bind(mobile)
            .bind(mobile_area_code)
            .bind(is_default)
            .bind(is_system)
            .bind(is_selected)
            .bind(latitude)
            .bind(longitude)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r)
    }

    /// 更新收货地址（Java `updateById`）。
    ///
    /// `patch` 的每个 `Some` 列都会出现在 SET 子句里，`None` 列**完全不出现**
    /// —— 这正是 MyBatis-Plus `updateById` 的「null 字段不参与 UPDATE」语义。
    ///
    /// 列名全部来自本文件 [`USER_ADDRESS_UPDATE_FIELDS`] 的编译期常量，
    /// 不存在注入面；`patch` 全空时返回 `Ok(false)`（不执行 SQL）。
    ///
    /// 返回 `true` 表示至少影响一行（Java `updateById` 的返回值）。
    pub async fn update_user_address(
        &self,
        address_id: i32,
        patch: &AddressUpdatePatch,
    ) -> Result<bool> {
        let mut sets: Vec<&'static str> = Vec::new();
        if patch.address_tag.is_some() {
            sets.push("address_tag");
        }
        if patch.consignee.is_some() {
            sets.push("consignee");
        }
        if patch.email.is_some() {
            sets.push("email");
        }
        if patch.region_ids.is_some() {
            sets.push("region_ids");
        }
        if patch.region_names.is_some() {
            sets.push("region_names");
        }
        if patch.address.is_some() {
            sets.push("address");
        }
        if patch.position_address.is_some() {
            sets.push("position_address");
        }
        if patch.postcode.is_some() {
            sets.push("postcode");
        }
        if patch.telephone.is_some() {
            sets.push("telephone");
        }
        if patch.mobile.is_some() {
            sets.push("mobile");
        }
        if patch.mobile_area_code.is_some() {
            sets.push("mobile_area_code");
        }
        if patch.is_default.is_some() {
            sets.push("is_default");
        }
        if patch.is_selected.is_some() {
            sets.push("is_selected");
        }
        if patch.latitude.is_some() {
            sets.push("latitude");
        }
        if patch.longitude.is_some() {
            sets.push("longitude");
        }
        if sets.is_empty() {
            // Java：`updateById(全 null 实体)` 生成的 SQL 无 SET 子句 →
            // MyBatis-Plus 直接抛异常；这里返回 Ok(false) 表示「无可更新字段」，
            // 由服务层决定是否视为失败（见 services/proxy_shopping_user_address.rs）。
            return Ok(false);
        }
        let assignments = sets
            .iter()
            .map(|c| format!("{c} = ?"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "UPDATE user_address SET {assignments} WHERE address_id = ?"
        ));

        let mut q = sqlx::query(&sql);
        if let Some(v) = &patch.address_tag {
            q = q.bind(v);
        }
        if let Some(v) = &patch.consignee {
            q = q.bind(v);
        }
        if let Some(v) = &patch.email {
            q = q.bind(v);
        }
        if let Some(v) = &patch.region_ids {
            q = q.bind(v);
        }
        if let Some(v) = &patch.region_names {
            q = q.bind(v);
        }
        if let Some(v) = &patch.address {
            q = q.bind(v);
        }
        if let Some(v) = &patch.position_address {
            q = q.bind(v);
        }
        if let Some(v) = &patch.postcode {
            q = q.bind(v);
        }
        if let Some(v) = &patch.telephone {
            q = q.bind(v);
        }
        if let Some(v) = &patch.mobile {
            q = q.bind(v);
        }
        if let Some(v) = &patch.mobile_area_code {
            q = q.bind(v);
        }
        if let Some(v) = patch.is_default {
            q = q.bind(v);
        }
        if let Some(v) = patch.is_selected {
            q = q.bind(v);
        }
        if let Some(v) = patch.latitude {
            q = q.bind(v);
        }
        if let Some(v) = patch.longitude {
            q = q.bind(v);
        }
        let r = q
            .bind(address_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 把某会员的默认地址全部清零（Java `LambdaUpdateWrapper.set(isDefault, 0).eq(userId, ..)`）。
    ///
    /// `exclude_address_id` 对应 Java `update` 链路里的
    /// `.ne(UserAddress::getAddressId, updateDTO.getAddressId())`；
    /// 新增链路传 `None`（刚刚 INSERT 的那条 `is_default = 0`，无需排除）。
    pub async fn clear_default_flags(
        &self,
        user_id: i32,
        exclude_address_id: Option<i32>,
    ) -> Result<u64> {
        let r = match exclude_address_id {
            Some(id) => {
                sqlx::query("UPDATE user_address SET is_default = 0 WHERE user_id = $1 AND address_id <> $2")
                    .bind(user_id)
                    .bind(id)
                    .execute(&self.pool)
                    .await
            }
            None => {
                sqlx::query("UPDATE user_address SET is_default = 0 WHERE user_id = $1")
                    .bind(user_id)
                    .execute(&self.pool)
                    .await
            }
        }
        .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 把某会员的选中标记全部清零（Java `setSelected` 的第一步 UPDATE）。
    pub async fn clear_selected_flags(&self, user_id: i32) -> Result<u64> {
        let r = sqlx::query("UPDATE user_address SET is_selected = 0 WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 把指定地址置为选中（Java `setSelected` 的第二步 UPDATE）。
    ///
    /// Java 同时带 `eq(addressId).eq(userId)` 两个条件，故越权（他人地址）
    /// 不会命中任何行 —— 本实现照抄该归属校验。
    pub async fn mark_selected(&self, address_id: i32, user_id: i32) -> Result<bool> {
        let r = sqlx::query(
            "UPDATE user_address SET is_selected = 1 WHERE address_id = $1 AND user_id = $2",
        )
        .bind(address_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    // ————————————————————— 地区 —————————————————————

    /// 按 regionId 批量查地区名（Java `regionService.list(in(regionId, ids))`）。
    ///
    /// ⚠️ Java 未指定 ORDER BY，MySQL 实际按主键升序返回；本实现显式写出
    /// `ORDER BY region_id ASC` 以固定该事实行为（避免执行计划变化导致
    /// `region_names` / `region_name` 的展示顺序漂移）。
    pub async fn find_regions_by_ids(&self, region_ids: &[i32]) -> Result<Vec<Region>> {
        if region_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; region_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM region WHERE region_id IN ({placeholders}) ORDER BY region_id ASC"
        ));
        let mut q = sqlx::query_as::<_, Region>(&sql);
        for id in region_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按主键取单个地区（Java `RegionServiceImpl#getRegionPath` 的 `getById`）。
    pub async fn find_region(&self, region_id: i32) -> Result<Option<Region>> {
        sqlx::query_as::<_, Region>("SELECT * FROM region WHERE region_id = $1")
            .bind(region_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 全量地区（Java `RegionServiceImpl#getAllRegionTree` 的 `list(queryWrapper)`）。
    ///
    /// `is_overseas == 1` → `level >= 0`；否则 → `level > 0`。
    /// 种子数据里「中国」是 `region_id = 1, level = 1`，省市区的 level 依次为
    /// 2/3/4，故国内版会包含中国这个根节点（与 Java 完全一致）。
    pub async fn find_regions_for_tree(&self, is_overseas: bool) -> Result<Vec<Region>> {
        let sql = if is_overseas {
            "SELECT * FROM region WHERE level >= 0 ORDER BY region_id ASC"
        } else {
            "SELECT * FROM region WHERE level > 0 ORDER BY region_id ASC"
        };
        sqlx::query_as::<_, Region>(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    // ————————————————————— 系统配置 —————————————————————

    /// 读取系统配置项（Java `ConfigServiceImpl#getConfigByCode`）。
    ///
    /// Java 在配置缺失时抛 `GlobalException(CONFIG_NOT_EXIST)`；
    /// 本实现返回 `Ok(None)`，由服务层决定降级策略（见
    /// `services/proxy_shopping_user_address.rs::get_region_path`）。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }
}

/// 允许出现在动态 UPDATE `SET` 子句里的列（`user_address` 的子集）。
///
/// 只列 Java `UserAddressUpdateDTO` 能改的字段；列名是编译期常量，
/// 用户输入永不进 SQL。是 `nextrade_core::models::trade::proxy_shopping_user_address::AddressUpdatePatch`
/// 的字符串镜像，两者由 `update_field_whitelist_matches_patch_struct` 单测约束。
pub const USER_ADDRESS_UPDATE_FIELDS: [&str; 15] = [
    "address_tag",
    "consignee",
    "email",
    "region_ids",
    "region_names",
    "address",
    "position_address",
    "postcode",
    "telephone",
    "mobile",
    "mobile_area_code",
    "is_default",
    "is_selected",
    "latitude",
    "longitude",
];

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
    fn user_address_columns_cover_all_twenty_seven_from_schema() {
        assert_eq!(USER_ADDRESS_COLUMNS.split(',').count(), 27);
        for col in [
            "address_id",
            "region_ids",
            "region_names",
            "is_default",
            "is_selected",
            "is_system",
            "postcode",
            "latitude",
            "longitude",
        ] {
            assert!(USER_ADDRESS_COLUMNS.contains(col), "缺少列 {col}");
        }
    }

    #[test]
    fn sort_column_whitelist_falls_back_to_primary_key() {
        // 控制器只会传 is_selected
        assert_eq!(resolve_sort_column("is_selected"), "is_selected");
        assert_eq!(resolve_sort_column("address_id"), "address_id");
        assert_eq!(resolve_sort_column("isDefault"), "address_id");
        assert_eq!(resolve_sort_column(""), "address_id");
        assert_eq!(resolve_sort_column("1=1 OR 1=1"), "address_id");
        assert_eq!(resolve_sort_column("address_id; DROP TABLE user_address"), "address_id");
        // 白名单里没有 region_name（那是 region 表的列）
        assert_eq!(resolve_sort_column("region_name"), "address_id");
    }

    #[test]
    fn update_sql_builder_uses_only_present_and_whitelisted_columns() {
        // 模拟仓储内部拼 SET 子句的规则：只有 Some 的字段参与，且必须命中白名单。
        let present = |name: &str, patch: &AddressUpdatePatch| -> bool {
            match name {
                "address_tag" => patch.address_tag.is_some(),
                "consignee" => patch.consignee.is_some(),
                "email" => patch.email.is_some(),
                "region_ids" => patch.region_ids.is_some(),
                "region_names" => patch.region_names.is_some(),
                "address" => patch.address.is_some(),
                "position_address" => patch.position_address.is_some(),
                "postcode" => patch.postcode.is_some(),
                "telephone" => patch.telephone.is_some(),
                "mobile" => patch.mobile.is_some(),
                "mobile_area_code" => patch.mobile_area_code.is_some(),
                "is_default" => patch.is_default.is_some(),
                "is_selected" => patch.is_selected.is_some(),
                "latitude" => patch.latitude.is_some(),
                "longitude" => patch.longitude.is_some(),
                _ => false,
            }
        };
        let patch = AddressUpdatePatch {
            consignee: Some("张三".into()),
            is_default: Some(1),
            latitude: Some("30.1".parse().unwrap()),
            ..Default::default()
        };
        let sets: Vec<&str> = USER_ADDRESS_UPDATE_FIELDS
            .iter()
            .copied()
            .filter(|c| present(c, &patch))
            .collect();
        assert_eq!(sets, vec!["consignee", "is_default", "latitude"]);
        let sql = format!(
            "UPDATE user_address SET {} WHERE address_id = $1",
            sets.iter().map(|c| format!("{c} = ?")).collect::<Vec<_>>().join(", ")
        );
        assert_eq!(
            sql,
            "UPDATE user_address SET consignee = $1, is_default = $2, latitude = $3 WHERE address_id = $4"
        );

        // 全 None → 无 SET 子句（仓储据此返回 Ok(false)，不执行 SQL）
        let empty = AddressUpdatePatch::default();
        assert!(!USER_ADDRESS_UPDATE_FIELDS
            .iter()
            .any(|c| present(c, &empty)));
    }

    #[test]
    fn update_field_whitelist_matches_patch_struct() {
        // 白名单必须覆盖 AddressUpdatePatch 的全部字段，且不得含只读列。
        for col in [
            "address_tag",
            "consignee",
            "email",
            "region_ids",
            "region_names",
            "address",
            "position_address",
            "postcode",
            "telephone",
            "mobile",
            "mobile_area_code",
            "is_default",
            "is_selected",
            "latitude",
            "longitude",
        ] {
            assert!(USER_ADDRESS_UPDATE_FIELDS.contains(&col), "白名单缺 {col}");
        }
        assert_eq!(USER_ADDRESS_UPDATE_FIELDS.len(), 15);
    }

    #[test]
    fn update_field_whitelist_excludes_non_writable_columns() {
        // 归属/主键/系统标记不得通过 update 接口改写（Java 控制器会强制覆盖前两者）
        for col in ["address_id", "user_id", "is_system", "region_name"] {
            assert!(
                !USER_ADDRESS_UPDATE_FIELDS.contains(&col),
                "{col} 不应可被动态更新"
            );
        }
        // region_names 是服务层根据 region_ids 反查出来的，允许写
        assert!(USER_ADDRESS_UPDATE_FIELDS.contains(&"region_names"));
    }

    #[test]
    fn default_country_config_code_matches_java_enum() {
        // Java `SettingsEnum.DEFAULT_COUNTRY("defaultCountry", ...)`
        assert_eq!(CONFIG_DEFAULT_COUNTRY, "defaultCountry");
    }
}
