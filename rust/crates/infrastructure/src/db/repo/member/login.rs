//! 会员登录域仓储（member::login）。
//!
//! 表/列来自权威 schema `java/database/pure.sql`
//! （核对：`cd rust && python tools/schema_parser.py --table user` /
//! `--table user_authorize` / `--table config`）：
//! - `user`：`user_id` / `username` / `avatar` / `mobile` / `mobile_validated` /
//!   `email` / `nickname` / `password` / `reg_time` / `referrer_user_id` / `status` …
//! - `user_authorize`：`authorize_id` / `authorize_type` / `user_id` / `open_id` /
//!   `open_data` / `open_name` / `open_photo` / `unionid` / `add_time`
//! - `config`：`biz_code` / `biz_val`（登录相关开关：`shopRegClosed` /
//!   `openWechatOauth` / `wechatAppId` / `usernamePrefix` …）
//!
//! 对应 Java：
//! - `UserServiceImpl#issuePasswordReset`（`selectList(...LIMIT 2)`）
//! - `UserServiceImpl#modifyPassword`（`updateById(user)`）
//! - `UserServiceImpl#bindMobile`（`getOne(mobile)` / `save(user)` /
//!   `userAuthorizeService.save/updateById`）
//! - `UserServiceImpl#unbindWechat`（`userAuthorizeService.remove(...)`）
//! - `ConfigServiceImpl#getConfigVal`（`config` 表按 `biz_code` 取值）
//!
//! 运行时 `sqlx`（**不用** `query!` 宏）；每条 SQL 单行字面量。

use sqlx::PgPool;
use nextrade_core::{Error, Result};

/// `user` 中「可自动重置密码」的候选行（Java `LIMIT 2` 用于判定唯一性）。
const RESET_MOBILE_SQL: &str =
    "SELECT user_id, status FROM \"user\" WHERE mobile = $1 LIMIT 2";
const RESET_EMAIL_SQL: &str =
    "SELECT user_id, status FROM \"user\" WHERE email = $1 LIMIT 2";

/// 会员登录域仓储（持有连接池）。
#[derive(Clone)]
pub struct LoginRepository {
    pool: PgPool,
}

impl LoginRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    /// 开启事务（绑定手机 / 微信授权等跨表写入）。
    ///
    /// # Errors
    /// 连接池获取连接失败 → `Error::Database`。
    pub async fn begin(&self) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
        self.pool.begin().await.map_err(map_err)
    }

    // ————————————————————— 用户查询 —————————————————————

    /// 按手机号查「重置密码」候选（Java `list(eq(mobile).last("LIMIT 2"))`）。
    ///
    /// 返回 `(user_id, status)`；调用方要求**恰好 1 行且 status=1**。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn reset_candidates_by_mobile(&self, mobile: &str) -> Result<Vec<(i32, i32)>> {
        sqlx::query_as::<_, (i32, Option<i32>)>(RESET_MOBILE_SQL)
            .bind(mobile)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
            .map(|rows| rows.into_iter().map(|(id, s)| (id, s.unwrap_or(0))).collect())
    }

    /// 按邮箱查「重置密码」候选（Java `list(eq(email).last("LIMIT 2"))`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn reset_candidates_by_email(&self, email: &str) -> Result<Vec<(i32, i32)>> {
        sqlx::query_as::<_, (i32, Option<i32>)>(RESET_EMAIL_SQL)
            .bind(email)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)
            .map(|rows| rows.into_iter().map(|(id, s)| (id, s.unwrap_or(0))).collect())
    }

    /// 按手机号查用户基础信息（Java `getOne(eq(mobile))`）。
    ///
    /// 返回 `(user_id, username, status)`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_user_by_mobile(
        &self,
        mobile: &str,
    ) -> Result<Option<(i32, String, i32)>> {
        let row: Option<(i32, String, Option<i32>)> = sqlx::query_as(
            "SELECT user_id, username, status FROM \"user\" WHERE mobile = $1 LIMIT 1",
        )
        .bind(mobile)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id, username, status)| (id, username, status.unwrap_or(0))))
    }

    /// 用户名是否已存在（自动注册生成唯一用户名用；Java `generateUniqueUsername`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn username_exists(&self, username: &str) -> Result<bool> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE username = $1")
                .bind(username)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(count > 0)
    }

    /// 按 ID 查用户状态（Java `getById(userId)` 后校验 `status`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn user_status(&self, user_id: i32) -> Result<Option<i32>> {
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT status FROM \"user\" WHERE user_id = $1 LIMIT 1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.map(|(s,)| s.unwrap_or(0)))
    }

    // ————————————————————— 用户写入 —————————————————————

    /// 更新密码（Java `updateById(User{userId,password})`），返回是否影响 1 行。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_password(&self, user_id: i32, password_hash: &str) -> Result<bool> {
        let result = sqlx::query("UPDATE \"user\" SET password = $1 WHERE user_id = $2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() == 1)
    }

    /// 标记手机号已验证（Java `user.setMobileValidated(1); updateById(user)`）。
    ///
    /// 只写 `mobile_validated`，避免用旧实体覆盖其它列（与 Java `updateById`
    /// 的 `NOT_NULL` 净效果一致）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn set_mobile_validated(&self, user_id: i32) -> Result<()> {
        sqlx::query("UPDATE \"user\" SET mobile_validated = 1 WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 自动注册插入用户（Java `bindMobile` 新建分支）。
    ///
    /// 返回新 `user_id`；未列出的列使用 DDL 默认值（`email=''`、`status=1` 等）。
    ///
    /// # Errors
    /// 数据库失败（如用户名唯一键冲突）→ `Error::Database`。
    pub async fn insert_registered_user(
        &self,
        username: &str,
        nickname: &str,
        avatar: &str,
        mobile: &str,
        password_hash: &str,
        reg_time: i32,
        referrer_user_id: i32,
    ) -> Result<i32> {
        let result: i32 = sqlx::query_scalar(
            "INSERT INTO \"user\" (username, avatar, mobile, mobile_validated, nickname, password, reg_time, referrer_user_id) VALUES ($1, $2, $3, 1, $4, $5, $6, $7) RETURNING user_id",
        )
        .bind(username)
        .bind(avatar)
        .bind(mobile)
        .bind(nickname)
        .bind(password_hash)
        .bind(reg_time)
        .bind(referrer_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result)
    }

    /// 按手机号取 `reg_time` 空档用不到：批量注册时间统一由服务层 `now_secs` 提供。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn touch_last_login(&self, user_id: i32, ip: &str, now: i32) -> Result<()> {
        sqlx::query("UPDATE \"user\" SET last_login = $1, last_ip = $2 WHERE user_id = $3")
            .bind(now)
            .bind(ip)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ————————————————————— 第三方授权（user_authorize） —————————————————————

    /// 按 openid + 授权类型查授权记录（Java `getOne(eq(openId).eq(authorizeType))`）。
    ///
    /// 返回 `(authorize_id, user_id)`。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_authorize_by_open(
        &self,
        open_id: &str,
        authorize_type: i32,
    ) -> Result<Option<(i32, i32)>> {
        let row: Option<(i32, i32)> = sqlx::query_as(
            "SELECT authorize_id, user_id FROM user_authorize WHERE open_id = $1 AND authorize_type = $2 LIMIT 1",
        )
        .bind(open_id)
        .bind(authorize_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 按用户查授权记录（Java `getOne(eq(userId))`；`unbindWechat` / `bindWechat` 用）。
    ///
    /// 返回 `(authorize_id, open_id)`；`open_id` 在库中为 `NOT NULL` 列。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn find_authorize_by_user(&self, user_id: i32) -> Result<Option<(i32, String)>> {
        let row: Option<(i32, String)> = sqlx::query_as(
            "SELECT authorize_id, open_id FROM user_authorize WHERE user_id = $1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row)
    }

    /// 插入授权记录（Java `userAuthorizeService.save(...)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_authorize(
        &self,
        authorize_type: i32,
        user_id: i32,
        open_id: &str,
        open_data: Option<&str>,
        open_name: &str,
        open_photo: &str,
        unionid: Option<&str>,
        add_time: i32,
    ) -> Result<i32> {
        let result: i32 = sqlx::query_scalar(
            "INSERT INTO user_authorize (authorize_type, user_id, open_id, open_data, open_name, open_photo, unionid, add_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING authorize_id",
        )
        .bind(authorize_type)
        .bind(user_id)
        .bind(open_id)
        .bind(open_data)
        .bind(open_name)
        .bind(open_photo)
        .bind(unionid)
        .bind(add_time)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result)
    }

    /// 把已有 openid 记录改为归属当前用户（Java `existingAuthorize.setUserId(...)`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_authorize_user(
        &self,
        authorize_id: i32,
        user_id: i32,
        unionid: Option<&str>,
        open_data: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE user_authorize SET user_id = $1, unionid = $2, open_data = $3 WHERE authorize_id = $4",
        )
        .bind(user_id)
        .bind(unionid)
        .bind(open_data)
        .bind(authorize_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 把用户已有授权记录的 openid 更新为新 openid（Java 老用户换新微信）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn update_authorize_open(
        &self,
        authorize_id: i32,
        open_id: &str,
        unionid: Option<&str>,
        open_data: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE user_authorize SET open_id = $1, unionid = $2, open_data = $3 WHERE authorize_id = $4",
        )
        .bind(open_id)
        .bind(unionid)
        .bind(open_data)
        .bind(authorize_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 按 openid 删除授权记录（Java `userAuthorizeService.remove(eq(openId))`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn delete_authorize_by_open(&self, open_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM user_authorize WHERE open_id = $1")
            .bind(open_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected())
    }

    // ————————————————————— 配置（config 表） —————————————————————

    /// 读配置值（Java `ConfigService#getConfigVal` / `getConfigByCode`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn config_value(&self, biz_code: &str) -> Result<Option<String>> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND (is_del IS NULL OR is_del = 0) LIMIT 1",
        )
        .bind(biz_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(v,)| v))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_queries_use_java_limit_two() {
        // Java 用 `LIMIT 2` 判定「账号唯一」：0 行/2 行都拒绝重置
        assert!(RESET_MOBILE_SQL.contains("LIMIT 2"));
        assert!(RESET_EMAIL_SQL.contains("LIMIT 2"));
        assert!(RESET_MOBILE_SQL.contains("mobile = $1"));
        assert!(RESET_EMAIL_SQL.contains("email = $1"));
        assert!(RESET_MOBILE_SQL.contains("user_id"));
        assert!(RESET_MOBILE_SQL.contains("status"));
    }
}
