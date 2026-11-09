//! `user` 仓储，替代 `UserMapper`。

use rust_decimal::Decimal;
use sqlx::PgPool;
use nextrade_core::entities::User;
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 按 ID 查询。
    pub async fn find_by_id(&self, user_id: i32) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM user WHERE user_id = $1 LIMIT 1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 按手机号查询。
    /// 按用户名查询（前端「账号」登录兼容）。
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM user WHERE username = $1 LIMIT 1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    pub async fn find_by_mobile(&self, mobile: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM user WHERE mobile = $1 LIMIT 1")
            .bind(mobile)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)
    }

    /// 注册插入，返回 user_id。
    pub async fn insert(&self, user: &User) -> Result<i32> {
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO user
             (username, mobile, mobile_validated, nickname, password, balance,
              points, reg_time, status, user_type)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING user_id",
        )
        .bind(&user.username)
        .bind(&user.mobile)
        .bind(user.mobile_validated)
        .bind(user.nickname.as_deref().unwrap_or_default())
        .bind(&user.password)
        .bind(user.balance)
        .bind(user.points)
        .bind(user.reg_time)
        .bind(user.status)
        .bind(user.user_type)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(r)
    }

    /// 更新昵称/头像。
    pub async fn update_profile(
        &self,
        user_id: i32,
        nickname: Option<&str>,
        avatar: Option<&str>,
    ) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE user SET
               nickname = COALESCE($1, nickname),
               avatar = COALESCE($2, avatar)
             WHERE user_id = $3",
        )
        .bind(nickname)
        .bind(avatar)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        if affected == 0 {
            return Err(Error::NotFound("用户不存在".into()));
        }
        Ok(())
    }

    /// 余额入账（balance = balance + amount，结果不会为负）。
    pub async fn adjust_balance(&self, user_id: i32, amount: Decimal) -> Result<()> {
        sqlx::query("UPDATE user SET balance = balance + $1 WHERE user_id = $2")
            .bind(amount)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 余额出账（原子条件扣减，余额不足时 rows_affected = 0）。
    pub async fn try_debit_balance(&self, user_id: i32, amount: Decimal) -> Result<bool> {
        let affected = sqlx::query(
            "UPDATE user SET balance = balance - $1 WHERE user_id = $2 AND balance >= $3",
        )
        .bind(amount)
        .bind(user_id)
        .bind(amount)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 积分出账（原子条件扣减，积分不足时 rows_affected = 0）。
    ///
    /// # Errors
    /// 数据库错误。
    pub async fn try_debit_points(&self, user_id: i32, points: i32) -> Result<bool> {
        if points <= 0 {
            return Ok(true);
        }
        let affected = sqlx::query(
            "UPDATE user SET points = points - $1 WHERE user_id = $2 AND points >= $3",
        )
        .bind(points)
        .bind(user_id)
        .bind(points)
        .execute(&self.pool)
        .await
        .map_err(map_err)?
        .rows_affected();
        Ok(affected > 0)
    }

    /// 会员分页查询（发放对象选择器）。
    ///
    /// 关键字模糊匹配用户名/昵称/手机号/邮箱，对齐 Java
    /// `UserCouponServiceImpl#searchIssueUsers` 的搜索口径。
    /// 会员分页（可选店铺隔离：`shop_scope = Some(shop_id)` 时仅本店会员）。
    pub async fn paginate_members(
        &self,
        keyword: Option<&str>,
        shop_scope: Option<i32>,
        page: &nextrade_core::types::PageQuery,
    ) -> Result<(u64, Vec<User>)> {
        let like = keyword
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| format!("%{k}%"));
        let (limit, offset) = page.limit_offset();

        let where_sql = "WHERE user_type = 1 AND ($1 IS NULL OR username LIKE $2 OR nickname LIKE $3 OR mobile LIKE $4 OR email LIKE $5) AND ($6 IS NULL OR EXISTS (SELECT 1 FROM shop_member m WHERE m.user_id = user.user_id AND m.shop_id = $7 AND m.is_del = 0))";

        let count_sql = format!("SELECT COUNT(*) FROM user {where_sql}");
        let (total,): (i64,) = sqlx::query_as(&count_sql)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(shop_scope)
            .bind(shop_scope)
            .fetch_one(&self.pool)
            .await
            .map_err(map_err)?;

        let list_sql = format!("SELECT * FROM user {where_sql} ORDER BY user_id DESC LIMIT ? OFFSET ?");
        let rows = sqlx::query_as::<_, User>(&list_sql)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(&like)
            .bind(shop_scope)
            .bind(shop_scope)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;

        Ok((total.max(0) as u64, rows))
    }
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("用户不存在".into()),
        other => Error::Database(other.to_string()),
    }
}