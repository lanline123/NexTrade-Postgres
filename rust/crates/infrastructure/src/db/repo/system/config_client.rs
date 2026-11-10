//! 配置域（system::config_client）仓储。
//!
//! 涉及的表（列名严格对齐 `java/database/pure.sql`，
//! 用 `cd rust && python tools/schema_parser.py --table <表名>` 核对）：
//!
//! - `config`（10 列，主键 `id`）—— 唯一主表：
//!   `id` / `biz_code` / `biz_val` / `create_time` / `create_by_id` /
//!   `create_by_name` / `update_time` / `update_by_id` / `update_by_name` /
//!   `is_del`
//! - `area_code`（5 列，主键 `id`）：
//!   `id` / `code` / `name` / `is_available` / `is_default`
//! - `decorate`（15 列，主键 `decorate_id`）—— 只读：
//!   `decorate_id` / `decorate_title` / `data` / `draft_data` /
//!   `decorate_type` / `page_type` / `is_home` / `shop_id` / `status` /
//!   `update_time` / `locale_id` / `parent_id` / `sort_order` /
//!   `binded_h5_id` / `share_info`
//! - `user_rank_config`（4 列，主键 `id`）—— 只读：
//!   `id` / `code` / `rank_type` / `data`
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query` / `QueryBuilder`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! 金额：本域**不涉及任何金额列**。

use sqlx::PgPool;

use nextrade_core::entities::{AreaCode, Config, Decorate, UserRankConfig};
use nextrade_core::models::system::config_client::{CONFIG_COLUMNS, IS_DEL_NO, USER_RANK_CONFIG_GROW_CODE};
use nextrade_core::{Error, Result};

/// 固定 SELECT 列清单（避免 `SELECT *` 与实体字段顺序耦合；
/// 与 `entities::Config` 的 10 个字段一一对应，顺序也一致）。
const CONFIG_SELECT: &str = "SELECT id, biz_code, biz_val, create_time, create_by_id, \
     create_by_name, update_time, update_by_id, update_by_name, is_del FROM config";

/// \"area_code\" 固定 SELECT 列清单（5 列，与 `entities::AreaCode` 一致）。
const AREA_CODE_SELECT: &str = "SELECT id, code, name, is_available, is_default FROM area_code";

/// \"decorate\" 固定 SELECT 列清单（15 列，与 `entities::Decorate` 一致）。
const DECORATE_SELECT: &str = "SELECT decorate_id, decorate_title, data, draft_data, \
     decorate_type, page_type, is_home, shop_id, status, update_time, locale_id, parent_id, \
     sort_order, binded_h5_id, share_info FROM decorate";

/// \"user_rank_config\" 固定 SELECT 列清单（4 列，与 `entities::UserRankConfig` 一致）。
const USER_RANK_CONFIG_SELECT: &str = "SELECT id, code, rank_type, data FROM user_rank_config";

#[derive(Clone)]
pub struct ConfigClientRepository {
    pool: PgPool,
}

impl ConfigClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（供服务层做事务编排，与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— config —————————————————————

    /// 全量配置行（对应 Java `ConfigServiceImpl#preloadAllConfigs` 的
    /// `lambdaQuery().eq(isDel, 0).list()`）。
    ///
    /// ⚠️ 这里**必须**过滤 `is_del = 0`：Java 的 \"preloadAllConfigs\" 显式写了
    /// `.eq(ConfigPO::getIsDel, 0)`，而 `SettingsHelper#getRaw` 的
    /// `selectOne(eq(bizCode))` **没有**过滤。
    /// 两者并存时，\"preloadAllConfigs\" 的结果优先（缓存哨兵），
    /// 因此本实现按 `is_del = 0` 过滤，与前台实际取值一致。
    pub async fn find_all_configs(&self) -> Result<Vec<Config>> {
        sqlx::query_as::<_, Config>(&format!(
            "{CONFIG_SELECT} WHERE {}.{} = $1",
            "config", "is_del"
        ))
        .bind(IS_DEL_NO)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按 \"biz_code\" 查单个配置行
    /// （对应 Java `SettingsHelper#getRaw` 的
    /// `selectOne(eq(bizCode))` 与 `ConfigServiceImpl#getConfigVal` 的
    /// `lambdaQuery().eq(bizCode).eq(isDel, 0).one()`）。
    ///
    /// \"filter_is_del\" 控制是否追加 `is_del = 0`：
    /// - `SettingsHelper#getRaw` **不过滤** ⇒ \"false\"
    /// - `ConfigServiceImpl#getConfigVal` **过滤** ⇒ \"true\"
    ///
    /// ⚠️ 两条链路在 Java 里对同一 bizCode 可能给出**不同**结果
    /// （被软删的行）。本仓库存其两种语义，由服务层按调用点选择。
    /// `.one()` 在匹配多行时 MyBatis-Plus 会抛
    /// \"TooManyResultsException\"；本实现用 `LIMIT 1` 取首行
    /// （见 \"uncertain_points\"）。
    pub async fn find_config_val(
        &self,
        biz_code: &str,
        filter_is_del: bool,
    ) -> Result<Option<String>> {
        let sql = if filter_is_del {
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = $2 LIMIT 1"
        } else {
            "SELECT biz_val FROM config WHERE biz_code = $1 LIMIT 1"
        };
        let row: Option<(Option<String>,)> = if filter_is_del {
            sqlx::query_as(sql)
                .bind(biz_code)
                .bind(IS_DEL_NO)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?
        } else {
            sqlx::query_as(sql)
                .bind(biz_code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?
        };
        Ok(row.and_then(|(v,)| v))
    }

    /// 按一组 `biz_code` 批量读取（对应 Java
    /// `SettingsHelper#getRawSnapshot` 的 `in(bizCode, bizCodes).eq(isDel, 0)`）。
    ///
    /// `biz_codes` 为空时**不发 SQL**，直接返回空列表
    /// （Java `getRawSnapshot` 对空数组显式返回 `Collections.emptyMap()`）。
    pub async fn find_config_values(&self, biz_codes: &[&str]) -> Result<Vec<Config>> {
        if biz_codes.is_empty() {
            return Ok(Vec::new());
        }
        // 用 QueryBuilder 生成 `IN ($3, $4, ...)`，占位符个数与切片长度一致。
        let mut qb: sqlx::QueryBuilder<'_, sqlx::Postgres> =
            sqlx::QueryBuilder::new(format!("{CONFIG_SELECT} WHERE is_del = "));
        qb.push_bind(IS_DEL_NO);
        qb.push(" AND biz_code IN (");
        let mut sep = qb.separated(", ");
        for code in biz_codes {
            sep.push_bind(*code);
        }
        sep.push_unseparated(")");
        drop(sep);
        qb.build_query_as::<Config>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 读取成长值配置行（`code = "grow_config"`）。
    ///
    /// 对应 Java `userRankConfigMapper.selectOne(
    /// Wrappers.lambdaQuery(UserRankConfig.class).eq(code, "grow_config"))`。
    ///
    /// ⚠️ Java `ConfigServiceImpl#getBase` 第 1642–1643 行对该结果**不判空**，
    /// 直接 `growConfig.getData()`；`user_rank_config` 表里没有
    /// `grow_config` 行时抛 **NPE** ⇒ 500。
    /// 本实现返回 `Option`，由服务层决定（见 `uncertain_points`）。
    pub async fn find_grow_config(&self) -> Result<Option<UserRankConfig>> {
        sqlx::query_as::<_, UserRankConfig>(&format!(
            "{USER_RANK_CONFIG_SELECT} WHERE code = $1 LIMIT 1"
        ))
        .bind(USER_RANK_CONFIG_GROW_CODE)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— area_code —————————————————————

    /// 启用中的区号列表，默认项排最前
    /// （对应 Java `AreaCodeServiceImpl#mobileAreaCode`：
    /// `eq(isAvailable, 1).orderByDesc(isDefault)`）。
    ///
    /// ⚠️ Java 只写了 `orderByDesc(isDefault)`，**没有**二级排序键；
    /// MySQL 对等值行的顺序不保证。本实现补 `id ASC` 作为稳定次序
    /// （与 system::area_code 的 `find_available` 完全一致，便于两处结果可比），
    /// 这是**有意的确定性收敛**，见 `uncertain_points`。
    pub async fn find_available_area_codes(&self) -> Result<Vec<AreaCode>> {
        sqlx::query_as::<_, AreaCode>(&format!(
            "{AREA_CODE_SELECT} WHERE is_available = 1 ORDER BY is_default DESC, id ASC"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ————————————————————— decorate —————————————————————

    /// 按 `decorate_id` 取装修页（对应 Java
    /// `decorateService.lambdaQuery().eq(Decorate::getDecorateId, previewId).one()`，
    /// `getInitConfigSettings` 的 `previewId` 分支）。
    pub async fn find_decorate_by_id(&self, decorate_id: i32) -> Result<Option<Decorate>> {
        sqlx::query_as::<_, Decorate>(&format!(
            "{DECORATE_SELECT} WHERE decorate_id = $1 LIMIT 1"
        ))
        .bind(decorate_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取自营 PC 首页装修页（对应 Java `getInitConfigSettings` 的
    /// `eq(shopId, 0).eq(decorateType, PC).eq(isHome, 1).eq(status, 1).last("limit 1")`）。
    ///
    /// `decorate_type` 由调用方传入（Java 常量为 `DecorateType.PC = 2`）。
    ///
    /// ⚠️ Java 这一条**没有** `.eq(pageType, 1)`，也**没有** locales 过滤；
    /// 而 `DecorateServiceImpl#getPcIndexDecoratePageConfig`（`getBase` 用）
    /// 却带了 `pageType = 1` 与 `localeId = 0`。两处条件**不一致**，
    /// 本实现逐条照抄各自的原条件（见 `uncertain_points`）。
    pub async fn find_pc_home_decorate(&self, decorate_type: i32) -> Result<Option<Decorate>> {
        sqlx::query_as::<_, Decorate>(&format!(
            "{DECORATE_SELECT} WHERE shop_id = 0 AND decorate_type = $1 AND is_home = 1 \
             AND status = 1 LIMIT 1"
        ))
        .bind(decorate_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 取 PC 首页装修页（`getBase` → `DecorateServiceImpl#getPcIndexDecoratePageConfig`
    /// 的无 \"id\" 分支）。
    ///
    /// 条件逐条照抄 Java：
    /// `shop_id = 0`、`decorate_type = DecorateType.H5(1)`、
    /// `page_type = 1`、`is_home = 1`、`status = 1`、`locale_id = 0`，
    /// 加 `limit 1`。
    ///
    /// ⚠️ 注意这里 `decorate_type` 传的是 **H5(1)** 而不是 PC(2)
    /// —— Java 原文就是 `.eq(Decorate::getDecorateType, DecorateType.H5.getCode())`，
    /// 方法名却叫 `getPcIndexDecoratePageConfig`。这看起来像 bug，
    /// 但本实现逐字符复刻（见 `uncertain_points`）。
    pub async fn find_pc_index_page_config(&self, decorate_type_h5: i32) -> Result<Option<Decorate>> {
        sqlx::query_as::<_, Decorate>(&format!(
            "{DECORATE_SELECT} WHERE shop_id = 0 AND decorate_type = $1 AND page_type = 1 \
             AND is_home = 1 AND status = 1 AND locale_id = 0 LIMIT 1"
        ))
        .bind(decorate_type_h5)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 是否存在「个人中心页」装修（对应 Java
    /// `eq(decorateType, H5).eq(pageType, 2).eq(isHome, 1).last("limit 1")`，
    /// 再判 `!= null`）。
    ///
    /// 返回 `bool`（存在与否），只做 `COUNT(*)` 以减少数据传输。
    pub async fn exists_member_center_decorate(
        &self,
        decorate_type: i32,
        page_type: i32,
    ) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM decorate WHERE decorate_type = $1 AND page_type = $2 \
             AND is_home = 1",
        )
        .bind(decorate_type)
        .bind(page_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0 > 0)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("配置不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SQL 片段里的列名逐个都在权威列清单内
    /// （防止手改 SQL 写错列名 —— `cargo check` 查不出）。
    #[test]
    fn select_clauses_only_use_known_columns() {
        for col in CONFIG_COLUMNS {
            assert!(
                CONFIG_SELECT.contains(col),
                "CONFIG_SELECT 缺少权威列 {col}"
            );
        }
        assert!(AREA_CODE_SELECT.contains("is_available"));
        assert!(AREA_CODE_SELECT.contains("is_default"));
        assert!(DECORATE_SELECT.contains("page_type"));
        assert!(DECORATE_SELECT.contains("locale_id"));
        assert!(USER_RANK_CONFIG_SELECT.contains("rank_type"));
    }

    /// `find_config_values` 的 `IN` 子句占位符个数必须与传入切片长度一致
    /// （**不手数占位符**，用 `matches('?').count()` 表达）。
    ///
    /// 这里直接执行实现里同款拼接并取 SQL 文本（`QueryBuilder::sql()`），
    /// 只构造语句、**不连库、不执行**。
    #[test]
    fn config_values_in_placeholders_match_len() {
        fn build(biz_codes: &[&str]) -> Option<String> {
            if biz_codes.is_empty() {
                return None;
            }
            let mut qb: sqlx::QueryBuilder<'_, sqlx::Postgres> =
                sqlx::QueryBuilder::new(format!("{CONFIG_SELECT} WHERE is_del = "));
            qb.push_bind(IS_DEL_NO);
            qb.push(" AND biz_code IN (");
            let mut sep = qb.separated(", ");
            for code in biz_codes {
                sep.push_bind(*code);
            }
            sep.push_unseparated(")");
            drop(sep);
            Some(qb.sql().to_string())
        }

        // 空切片 ⇒ 实现里提前返回（不发 SQL）
        assert!(build(&[]).is_none());

        for n in 1..=4usize {
            let codes: Vec<&str> = vec!["a"; n];
            let sql = build(&codes).expect("非空切片应生成 SQL");
            // 占位符 = is_del 的 1 个 + IN 的 n 个
            assert_eq!(
                sql.matches('$').count(),
                n + 1,
                "占位符个数应与绑定个数一致（is_del + n），SQL = {sql}"
            );
            assert!(sql.contains("biz_code IN ("), "SQL = {sql}");
            assert!(sql.contains("is_del = $1"), "SQL = {sql}");
        }
    }

    /// `map_err` 把 `RowNotFound` 映射为 `Error::NotFound`。
    #[test]
    fn map_err_row_not_found() {
        let e = map_err(sqlx::Error::RowNotFound);
        assert!(matches!(e, Error::NotFound(_)));
    }
}