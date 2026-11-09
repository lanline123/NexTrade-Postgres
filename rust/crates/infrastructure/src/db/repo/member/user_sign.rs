//! 用户签到仓储（member::user_sign）。表 `sign` / `sign_in_setting` / `product` / `user`。

use sqlx::PgPool;
use nextrade_core::models::member::user_sign::{RecommendProductVO, SignInSettingVO};
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct UserSignRepository {
    pool: PgPool,
}

impl UserSignRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 签到配置（Java：sort_order/create_time/id 升序）。
    pub async fn settings(&self) -> Result<Vec<SignInSettingVO>> {
        let rows: Vec<(i32, Option<String>, Option<i32>, Option<i32>, i32, i64)> = sqlx::query_as(
            "SELECT id, name, points, day_num, sort_order, CAST(create_time AS BIGINT) AS create_time FROM sign_in_setting ORDER BY sort_order ASC, create_time ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(id, name, points, day_num, sort_order, create_time)| SignInSettingVO {
                id,
                name,
                points,
                day_num,
                sort_order,
                create_time,
            })
            .collect())
    }

    /// 最近一次签到次数（Java `getSignCount`）。
    pub async fn last_sign_num(&self, user_id: i32) -> Result<i32> {
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT sign_num FROM sign WHERE user_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.and_then(|(n,)| n).unwrap_or(0))
    }

    /// 今日是否已签到（Java：`add_time > 今日 00:00`）。
    pub async fn signed_since(&self, user_id: i32, since: i64) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM sign WHERE user_id = $1 AND add_time > $2 ORDER BY id DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(since)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.is_some())
    }

    /// 插入签到记录。
    pub async fn insert(&self, user_id: i32, sign_num: i32, now: i32) -> Result<bool> {
        let r = sqlx::query("INSERT INTO sign (user_id, add_time, sign_num) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(now)
            .bind(sign_num)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(r.rows_affected() > 0)
    }

    /// 增加用户积分（Java `userService.incPoints`）。
    pub async fn inc_points(&self, user_id: i32, points: i32) -> Result<()> {
        sqlx::query("UPDATE \"user\" SET points = points + $1 WHERE user_id = $2")
            .bind(points)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    /// 推荐商品（Java：热销 + 上架 + 未删除，limit 8）。
    pub async fn recommend_products(&self) -> Result<Vec<RecommendProductVO>> {
        // `product_price` 是 NUMERIC，而 VO 里是 String（Java BigDecimal 序列化成字符串），
        // 直接解成 String 会报 `String (TEXT) is not compatible with NUMERIC`，故显式转 text。
        let rows: Vec<(i32, String, String, String)> = sqlx::query_as(
            "SELECT product_id, product_name, pic_thumb, CAST(product_price AS TEXT) AS product_price FROM product WHERE is_hot = 1 AND product_status = 1 AND is_delete = 0 LIMIT 8",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|(product_id, product_name, pic_thumb, product_price)| RecommendProductVO {
                product_id,
                product_name,
                pic_thumb,
                product_price,
            })
            .collect())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
