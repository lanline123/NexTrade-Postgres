//! 商户入驻域（merchant::merchant_client）仓储：`merchant_apply` 为主，
//! 写入链路涉及 `admin_msg`，只读关联 `config` / `region` / `admin_user`。
//!
//! 对应原 Java MyBatis-Plus Mapper 与若干 `Service` 内部查询：
//! - `MerchantApplyMapper`（`MerchantApplyServiceImpl extends
//!   BaseServiceImpl<MerchantApplyMapper, MerchantApply>`）
//! - `AdminUserService#getById` / `#getAdminUserByUseMobile` / `#getOne(eq(mobile))`
//! - `ConfigServiceImpl#getConfigByCode`
//! - `RegionServiceImpl#list(in(regionId, ...))`
//! - `AdminMsgServiceImpl#createMessage`
//!
//! 列名严格对齐 `java/database/pure.sql`
//! （`cd rust && python tools/schema_parser.py --table <表名>` 核对）：
//!
//! - `merchant_apply`（16 列，主键 `merchant_apply_id` AUTO_INCREMENT）：
//!   `merchant_apply_id` / `user_id` / `add_time` / `shop_title` /
//!   `status` / `type` / `contact_name` / `contact_mobile` / `audit_time` /
//!   `audit_admin_id` / `shop_data` / `base_data` / `merchant_data` /
//!   `company_name` / `corporate_name` / `audit_remark`
//!   —— 除主键外全部可空；库默认值：`add_time` `0`、`status` `1`、
//!   `type` `1`、`audit_time` `0`、`audit_admin_id` `0`，varchar 默认 `''`。
//! - `admin_msg`（14 列，主键 `msg_id` AUTO_INCREMENT）：
//!   `msg_type` / `send_time` / `is_readed` / `shop_id` / `admin_id` /
//!   `order_id` / `product_id` / `title` / `content` / `msg_link` /
//!   `related_data` / `is_broadcast` / `vendor_id`
//! - 只读：`config`（`biz_code` / `biz_val` / `is_del`）、
//!   `region`（`region_id` / `region_name` / `parent_id` / `level` /
//!   `is_hot` / `sort_order` / `first_word` / `standard_code`）、
//!   `admin_user`（21 列，整行映射）
//!
//! ⚠️ **未落到本仓储的写入**：Java `apply()` 在「免审核」分支还会创建
//! `merchant` / `admin_user` / `merchant_user` / `shop` / `admin_user_shop` /
//! `merchant_account` 六张表的记录（`createMerchant` → `createAdminUser` →
//! `createMerchantUser` → `createShop` → `createAdminUserShop` →
//! `createMerchantAccount`），并调用 `shippingTplService.initShippingTpl` /
//! `decorateService.initDecorate` / `adminRoleService.initAdminRole` /
//! `initEmployeeRoles`。后四项依赖的服务在本仓库**尚未迁移**，
//! 只建店铺不建运费/装修/角色会留下半成品数据，故服务层在该分支
//! **写入前**就返回明确的「未迁移」错误（见 `services/src/merchant_client.rs` 模块头
//! 第 5 条）；本文件因此**不实现**这六张表的 INSERT，避免「看似可用」的
//! 死代码。待上述初始化服务就绪后，按 Java 的 builder 逐列补齐即可。
//!
//! 查询统一使用运行时 `sqlx::query_as` / `sqlx::query`
//! （**不使用** `query!` / `query_as!` 宏：本机无 `DATABASE_URL`）。
//!
//! 金额：本域**不涉及任何金额计算**（无 `Decimal` 列参与）。

use sqlx::PgPool;

use nextrade_core::entities::{AdminUser, MerchantApply, Region};
use nextrade_core::{Error, Result};

/// `merchant_apply` 固定列清单（避免 `SELECT *` 与实体字段顺序耦合；
/// 与 `entities::MerchantApply` 的 16 个字段一一对应，顺序也一致）。
const MERCHANT_APPLY_SELECT: &str = "SELECT merchant_apply_id, user_id, add_time, shop_title, \
     status, \"type\", contact_name, contact_mobile, audit_time, audit_admin_id, \
     shop_data, base_data, merchant_data, company_name, corporate_name, audit_remark \
     FROM merchant_apply";

/// \"region\" 的列清单（与 `entities::Region` 的 8 个字段一一对应）。
const REGION_SELECT: &str = "SELECT region_id, region_name, parent_id, level, is_hot, \
     sort_order, first_word, standard_code FROM region";

/// 新增入驻申请的 SQL（列清单与占位符一一对应；提取为常量以便单测锁定）。
///
/// `COALESCE($1, 默认值)` 复刻 MyBatis-Plus \"NOT_NULL\" 字段策略下的列默认值，
/// 详见 [`MerchantClientRepository::insert_apply_in_tx`]。
const INSERT_APPLY_SQL: &str = "INSERT INTO merchant_apply
       (user_id, add_time, shop_title, status, \"type\",
        audit_time, audit_admin_id, shop_data, base_data,
        merchant_data, company_name, corporate_name)
     VALUES ($1, $2, COALESCE($3, ''), $4, COALESCE($5, 1),
             $6, $7, $8, $9, $10, COALESCE($11, ''), COALESCE($12, ''))";

/// 后台站内信 SQL（\"admin_msg\" 的非空列写死为 Java 实体默认值）。
const INSERT_ADMIN_MSG_SQL: &str = "INSERT INTO admin_msg
       (msg_type, send_time, is_readed, shop_id, admin_id, order_id,
        product_id, title, content, msg_link, related_data, is_broadcast, vendor_id)
     VALUES ($1, $2, 0, 0, 0, 0, 0, $3, $4, '', $5, 1, 0)";

#[derive(Clone)]
pub struct MerchantClientRepository {
    pool: PgPool,
}

impl MerchantClientRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池（与其它切片仓储一致）。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（Java `@Transactional(rollbackFor = Exception.class)`）。
    ///
    /// `MerchantApplyServiceImpl#apply` 是**一个事务**：写 `merchant_apply`
    /// 与写 `admin_msg` 必须同生共死。本仓储提供 [`Self::begin`] +
    /// `*_in_tx` 变体，由服务层在同一个事务里串联两处写入。
    pub async fn begin(
        &self,
    ) -> std::result::Result<sqlx::Transaction<'static, sqlx::Postgres>, sqlx::Error> {
        self.pool.begin().await
    }

    // ————————————————————— merchant_apply —————————————————————

    /// 按主键查申请（Java `BaseServiceImpl#getById`）。
    pub async fn find_apply_by_id(&self, id: i32) -> Result<Option<MerchantApply>> {
        let sql = crate::db::to_pg_placeholders(&format!("{MERCHANT_APPLY_SELECT} WHERE merchant_apply_id = ? LIMIT 1"));
        sqlx::query_as::<_, MerchantApply>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 「我的申请」：按 `user_id` 取最新一条
    /// （Java `myApply` 的
    /// `eq(MerchantApply::getUserId, userId).orderByDesc(merchantApplyId).last("limit 1")`）。
    pub async fn find_latest_apply_by_user(&self, user_id: i32) -> Result<Option<MerchantApply>> {
        let sql = crate::db::to_pg_placeholders(&format!(
            "{MERCHANT_APPLY_SELECT} WHERE user_id = ? ORDER BY merchant_apply_id DESC LIMIT 1"
        ));
        sqlx::query_as::<_, MerchantApply>(&sql)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 新增入驻申请（事务内），返回自增 \"merchant_apply_id\"
    /// （Java `this.save(merchantApply)`）。
    ///
    /// 显式列出除主键外的全部可写列；值来自服务层按 Java
    /// `MerchantApplyApplyParam#createMerchantApply` 构造的实体。
    ///
    /// ⚠️ Java 的 builder **没有**给 \"contactName\" / \"contactMobile\" /
    /// \"auditRemark\" 赋值，MyBatis-Plus \"save\" 对 \"null\" 字段不参与
    /// INSERT ⇒ 这三列实际落库为**列默认值 `''`**；本实现不写这三列，
    /// 交给 MySQL 默认值，结果一致。
    /// \"audit_admin_id\"（Java 同样未赋值，默认 0）与 \"audit_time\"
    /// （Java 仅「免审核直接通过」时给当前秒级时间戳，否则 null ⇒
    /// 落默认值 0）在服务层已按列默认值补齐为 `Some(0)`，
    /// 因此这里直接透传。
    /// \"shop_data\" / \"base_data\" / \"merchant_data\" 三个 \"text\" 列
    /// 无默认值，Java 侧 `JsonUtil.toJson(...)` 恒给出 JSON 串，
    /// 这里按实体原样写（None ⇒ NULL）。
    ///
    /// ⚠️ **`COALESCE($1, 默认值)` 的用意**（\"shop_title\" / \"type\" /
    /// \"company_name\" / \"corporate_name\"）：Java 侧这些字段可能为 null，
    /// MyBatis-Plus 的 \"NOT_NULL\" 字段策略会把它们从 INSERT 中剔除，
    /// 从而落到**列默认值**（\"shop_title\"/\"company_name\"/\"corporate_name\"
    /// 为 `''`，\"type\" 为 \"1\"）。本实现总是写满这些列，故必须显式补
    /// \"IFNULL\" 才与 Java 落库结果一致；否则会写入 NULL（可空列），
    /// 前端拿到 \"null\" 而非 `""`。
    pub async fn insert_apply_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        apply: &MerchantApply,
    ) -> Result<i32> {
        let res: i32 = sqlx::query_scalar(&format!("{} RETURNING merchant_apply_id", INSERT_APPLY_SQL))
            .bind(apply.user_id)
            .bind(apply.add_time)
            .bind(apply.shop_title.as_deref())
            .bind(apply.status)
            .bind(apply.r#type)
            .bind(apply.audit_time)
            .bind(apply.audit_admin_id)
            .bind(apply.shop_data.as_deref())
            .bind(apply.base_data.as_deref())
            .bind(apply.merchant_data.as_deref())
            .bind(apply.company_name.as_deref())
            .bind(apply.corporate_name.as_deref())
            .fetch_one(&mut **tx)
            .await
            .map_err(map_err)?;
        Ok(res)
    }

    // ————————————————————— admin_msg —————————————————————

    /// 写后台站内信（事务内，Java `AdminMsgServiceImpl#createMessage`）。
    ///
    /// Java 由 \"AdminMsgCreateDTO\" 只填 \"msgType\" / \"title\" / \"content\" /
    /// \"relatedData\"，其余字段为实体默认（\"isReaded\" / \"shopId\" / \"adminId\" /
    /// \"orderId\" / \"productId\" / \"vendorId\" ⇒ 0，\"msgLink\" ⇒ `''`）；
    /// \"sendTime\" = 当前秒级时间戳；\"isBroadcast\" 为 null 时补 **1**。
    pub async fn insert_admin_msg_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        msg_type: i32,
        title: &str,
        content: &str,
        related_data: &str,
        send_time: i32,
    ) -> Result<()> {
        sqlx::query(INSERT_ADMIN_MSG_SQL)
            .bind(msg_type)
            .bind(send_time)
            .bind(title)
            .bind(content)
            .bind(related_data)
            .execute(&mut **tx)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— config —————————————————————

    /// 读取系统配置项（Java `ConfigServiceImpl#getConfigByCode`：
    /// `biz_code = $2 AND is_del = 0`，**查不到时 Java 抛**
    /// \"CONFIG_NOT_EXIST\"「配置不存在」；此处返回 \"None\"，
    /// 由服务层抛出同样的业务错误）。
    ///
    /// `COALESCE(biz_val, '')`：Java 里「行不存在」（`configPO == null`
    /// ⇒ 抛「配置不存在」）与「行存在但 \"biz_val\" 为 NULL」（
    /// `getBizVal()` 返回 null，调用方做 `"1".equals(null)` / translate(null)）
    /// 是**两种不同**分支。用 `COALESCE` 把后者映射为 `Some("")`，
    /// 从而保留该区别，不会把「值为空」误判成「配置不存在」。
    pub async fn find_config_val(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT COALESCE(biz_val, '') FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| r.0))
    }

    // ————————————————————— region —————————————————————

    /// 按 ID 集合取地区（Java `applyDetail` / `detail` 里的
    /// `queryWrapper.in(Region::getRegionId, ids)` + `list`）。
    ///
    /// ⚠️ Java **没有** `ORDER BY`，且用 `Collectors.toMap` 收敛成
    /// `region_id -> region_name`（重复 key 会抛 `IllegalStateException`，
    /// 主键不会重复故不触发）；名称拼接顺序由**入参列表顺序**决定
    /// （Java 是 `for (Integer regionId : businessAddress)` 循环拼接），
    /// 与本查询的返回顺序无关 —— 服务层按入参顺序取映射。
    pub async fn find_regions_by_ids(&self, region_ids: &[i32]) -> Result<Vec<Region>> {
        if region_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; region_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("{REGION_SELECT} WHERE region_id IN ({placeholders})"));
        let mut q = sqlx::query_as::<_, Region>(&sql);
        for id in region_ids {
            q = q.bind(id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— admin_user —————————————————————

    /// 按主键取管理员（Java `resolveAuditAdminName` 的
    /// `adminUserService.getById(auditAdminId)`）。
    pub async fn find_admin_user_by_id(&self, admin_id: i32) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>("SELECT * FROM admin_user WHERE admin_id = $1 LIMIT 1")
            .bind(admin_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按手机号取管理员（Java \"applyDetail\" 里的
    /// `queryWrapper.eq(AdminUser::getMobile, contactPhone)` + \"getOne\"，
    /// 以及 `AdminUserServiceImpl#getAdminUserByUseMobile`）。
    ///
    /// ⚠️ Java 的 `getOne(wrapper)` 在多行命中时会抛
    /// \"TooManyResultsException\"（未加 `LIMIT 1`），且**不过滤**
    /// \"is_deleted\"；MyBatis-Plus 的 \"getOne\" 默认 `throwEx = true`。
    /// 本实现加 `ORDER BY admin_id ASC LIMIT 1`：既不因脏数据打崩请求，
    /// 也保证结果确定（差异已记入 \"uncertain_points\"）。
    pub async fn find_admin_user_by_mobile(&self, mobile: &str) -> Result<Option<AdminUser>> {
        sqlx::query_as::<_, AdminUser>(
            "SELECT * FROM admin_user WHERE mobile = $1 ORDER BY admin_id ASC LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound(MSG_MERCHANT_APPLY_NOT_EXIST.into()),
        other => Error::Database(other.to_string()),
    }
}

/// 与 `models::nextrade_core::models::merchant::merchant_client::MSG_MERCHANT_APPLY_NOT_EXIST` 同文案
/// （本文件不跨 crate 引用 models 常量，避免仓储层依赖 DTO 模块）。
const MSG_MERCHANT_APPLY_NOT_EXIST: &str = "商户申请信息不存在";

#[cfg(test)]
mod tests {
    use super::*;

    /// \"merchant_apply\" 的列清单必须覆盖权威 schema 的全部 16 列。
    #[test]
    fn merchant_apply_select_covers_all_columns() {
        for col in [
            "merchant_apply_id",
            "user_id",
            "add_time",
            "shop_title",
            "status",
            "contact_name",
            "contact_mobile",
            "audit_time",
            "audit_admin_id",
            "shop_data",
            "base_data",
            "merchant_data",
            "company_name",
            "corporate_name",
            "audit_remark",
        ] {
            assert!(
                MERCHANT_APPLY_SELECT.contains(col),
                "列清单缺少 {col}"
            );
        }
        // \"type\" 是 MySQL 关键字，必须反引号包裹
        assert!(MERCHANT_APPLY_SELECT.contains("\"type\""));
    }

    /// INSERT 的列数与占位符数必须一致（12 列 / 12 个 `$1`），
    /// 且必须用 \"IFNULL\" 复刻 MyBatis-Plus \"NOT_NULL\" 策略下的列默认值。
    ///
    /// 直接断言实际下发的 \"INSERT_APPLY_SQL\" 常量（而非测试里另抄一份），
    /// 这样常量一改测试就会失败。
    #[test]
    fn insert_apply_placeholder_count_matches_columns() {
        let sql = INSERT_APPLY_SQL;
        let cols = sql[sql.find('(').unwrap()..sql.find(" VALUES").unwrap()]
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();
        let marks = sql[sql.find("VALUES").unwrap()..].matches('?').count();
        assert_eq!(cols, 12);
        assert_eq!(marks, cols);
        // 四个可空列必须补列默认值：shop_title / type / company_name / corporate_name
        assert_eq!(sql.matches("IFNULL").count(), 4);
        assert!(sql.contains("COALESCE($1, '')"));
        assert!(sql.contains("COALESCE($1, 1)"));
        // 主键（自增）不能出现在列清单里
        assert!(!sql.contains("merchant_apply_id"));
    }

    /// \"admin_msg\" INSERT：13 列 / 13 个值，列数与值数必须一致；
    /// 实际下发的常量里有 5 个 `$1`（\"title\" / \"content\" / \"related_data\"
    /// 及前两个占位），其余写死。
    #[test]
    fn insert_admin_msg_column_count_matches_values() {
        let sql = INSERT_ADMIN_MSG_SQL;
        let cols = sql[sql.find('(').unwrap()..sql.find(" VALUES").unwrap()]
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count();
        let values = sql[sql.find("VALUES").unwrap()..]
            .trim()
            .trim_start_matches("VALUES (")
            .trim_end_matches(')')
            .split(',')
            .count();
        assert_eq!(cols, values, "列数与值数必须一致");
        assert_eq!(cols, 13);
        // 绑定参数个数 = 下发时 `.bind(...)` 的次数
        assert_eq!(sql.matches('?').count(), 5);
    }

    /// \"region\" 列清单与实体 8 字段一致。
    #[test]
    fn region_select_covers_all_columns() {
        for col in [
            "region_id",
            "region_name",
            "parent_id",
            "level",
            "is_hot",
            "sort_order",
            "first_word",
            "standard_code",
        ] {
            assert!(REGION_SELECT.contains(col), "列清单缺少 {col}");
        }
    }

    /// `map_err`：缺行 → NotFound，其余 → Database。
    #[test]
    fn map_err_shapes() {
        let not_found = map_err(sqlx::Error::RowNotFound);
        match &not_found {
            Error::NotFound(msg) => assert_eq!(msg, MSG_MERCHANT_APPLY_NOT_EXIST),
            other => panic!("期望 NotFound，实际 {other}"),
        }
        let other = map_err(sqlx::Error::Io(std::io::Error::other("boom")));
        assert!(matches!(other, Error::Database(_)));
    }
}