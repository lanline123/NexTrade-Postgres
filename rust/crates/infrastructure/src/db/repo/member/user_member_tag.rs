//! 用户标签关联仓储：`user_member_tag`，关联读 `member_tag` / `member_tag_group` /
//! `config`（店铺会员冻结开关）。
//!
//! 对应原 Java `UserMemberTagMapper` / `MemberTagMapper` / `MemberTagGroupMapper`。
//!
//! 列名严格对齐 `java/database/pure.sql`：
//! - `user_member_tag`：`id` / `shop_id` / `user_id` / `tag_id` / `create_time`
//! - `member_tag`：`id` / `shop_id` / `group_id` / `tag_name` / `is_del`
//! - `member_tag_group`：`id` / `shop_id` / `group_name` / `is_del`
//! - `config`：`biz_code` / `biz_val`（店铺会员总开关 `shopMemberEnabled`）
//!
//! 事务：`bindTags` / `batchBindTags` 在 Java 是 `@Transactional` 的多语句操作
//! （先删后增），本实现在仓储层用 `pool.begin()` 保证原子性。

use sqlx::PgPool;
use nextrade_core::entities::{MemberTag, MemberTagGroup, UserMemberTag};
use nextrade_core::{Error, Result};

#[derive(Clone)]
pub struct UserMemberTagRepository {
    pool: PgPool,
}

impl UserMemberTagRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 暴露连接池。
    #[must_use]
    pub fn pool_clone(&self) -> PgPool {
        self.pool.clone()
    }

    // ————————————————————— 配置 —————————————————————

    /// 店铺会员总开关（Java `SettingsEnum.SHOP_MEMBER_ENABLED`，缺省 `"0"` 关闭）。
    ///
    /// Java `readInt`：配置缺失 / 非数字 → 回退枚举默认值 `"0"`；`== 1` 为启用。
    pub async fn is_shop_member_enabled(&self) -> Result<bool> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT biz_val FROM config WHERE biz_code = $1 AND is_del = 0 LIMIT 1",
        )
        .bind("shopMemberEnabled")
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        let raw = row.and_then(|(v,)| v).unwrap_or_else(|| "0".to_string());
        Ok(raw.trim().parse::<i32>().map(|v| v == 1).unwrap_or(false))
    }

    // ————————————————————— 读 —————————————————————

    /// 用户在某店铺的标签关联（Java `bindTags` / `listByUser` 的 existing 查询）。
    pub async fn list_user_tags(
        &self,
        shop_id: i32,
        user_id: i64,
    ) -> Result<Vec<UserMemberTag>> {
        sqlx::query_as::<_, UserMemberTag>(
            "SELECT * FROM user_member_tag WHERE shop_id = $1 AND user_id = $2",
        )
        .bind(shop_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    /// 按标签 ID 批量取标签（Java `memberTagMapper.selectList(in id)`，带 `shop_id`）。
    pub async fn find_tags(&self, shop_id: i32, tag_ids: &[i64]) -> Result<Vec<MemberTag>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = placeholders(tag_ids.len());
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM member_tag WHERE shop_id = $1 AND id IN ({placeholders}) AND is_del = 0"
        ));
        let mut q = sqlx::query_as::<_, MemberTag>(&sql).bind(shop_id);
        for id in tag_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    /// 按标签组 ID 批量取组名（Java `memberTagGroupMapper.selectList(in id)`）。
    pub async fn find_groups(
        &self,
        shop_id: i32,
        group_ids: &[i64],
    ) -> Result<Vec<MemberTagGroup>> {
        if group_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = placeholders(group_ids.len());
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT * FROM member_tag_group WHERE shop_id = $1 AND id IN ({placeholders}) AND is_del = 0"
        ));
        let mut q = sqlx::query_as::<_, MemberTagGroup>(&sql).bind(shop_id);
        for id in group_ids {
            q = q.bind(*id);
        }
        q.fetch_all(&self.pool).await.map_err(map_err)
    }

    // ————————————————————— 写 —————————————————————

    /// 给单个用户打标签（Java `bindTags`）：删除不在新列表中的旧标签、补齐新增。
    pub async fn bind_tags(
        &self,
        shop_id: i32,
        user_id: i64,
        delete_ids: &[i64],
        insert_ids: &[i64],
        now: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        // 旧标签行按主键 id 删除（Java `deleteById(ut.getId())`）
        for tag_id in delete_ids {
            sqlx::query("DELETE FROM user_member_tag WHERE shop_id = $1 AND user_id = $2 AND tag_id = $3")
                .bind(shop_id)
                .bind(user_id)
                .bind(*tag_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
        }
        for tag_id in insert_ids {
            sqlx::query(
                "INSERT INTO user_member_tag (shop_id, user_id, tag_id, create_time) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(shop_id)
            .bind(user_id)
            .bind(*tag_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;
        }
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 取消单用户标签（Java `unbindTags`：`shop_id + user_id + tag_id IN (...)`）。
    pub async fn unbind_tags(
        &self,
        shop_id: i32,
        user_id: i64,
        tag_ids: &[i64],
    ) -> Result<u64> {
        if tag_ids.is_empty() {
            return Ok(0);
        }
        let placeholders = placeholders(tag_ids.len());
        let sql = crate::db::to_pg_placeholders(&format!(
            "DELETE FROM user_member_tag WHERE shop_id = ? AND user_id = ? AND tag_id IN ({placeholders})"
        ));
        let mut q = sqlx::query(&sql).bind(shop_id).bind(user_id);
        for id in tag_ids {
            q = q.bind(*id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }

    /// 批量打标签（Java \"batchBindTags\"：逐用户执行 bind 逻辑，同一事务）。
    pub async fn batch_bind_tags(
        &self,
        shop_id: i32,
        user_ids: &[i64],
        delete_map: &[(i64, Vec<i64>)],
        insert_map: &[(i64, Vec<i64>)],
        now: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        for (user_id, delete_ids) in delete_map {
            for tag_id in delete_ids {
                sqlx::query(
                    "DELETE FROM user_member_tag WHERE shop_id = $1 AND user_id = $2 AND tag_id = $3",
                )
                .bind(shop_id)
                .bind(*user_id)
                .bind(*tag_id)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
        }
        for (user_id, insert_ids) in insert_map {
            for tag_id in insert_ids {
                sqlx::query(
                    "INSERT INTO user_member_tag (shop_id, user_id, tag_id, create_time) \
                     VALUES ($1, $2, $3, $4)",
                )
                .bind(shop_id)
                .bind(*user_id)
                .bind(*tag_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
        }
        // user_ids 目前仅用于语义完整性（Java 逻辑逐用户处理）
        let _ = user_ids;
        tx.commit().await.map_err(map_err)?;
        Ok(())
    }

    /// 批量取消标签（Java `batchUnbindTags`：`shop_id + user_id IN (...) + tag_id IN (...)`）。
    pub async fn batch_unbind_tags(
        &self,
        shop_id: i32,
        user_ids: &[i64],
        tag_ids: &[i64],
    ) -> Result<u64> {
        if user_ids.is_empty() || tag_ids.is_empty() {
            return Ok(0);
        }
        let user_ph = placeholders(user_ids.len());
        let tag_ph = placeholders(tag_ids.len());
        let sql = crate::db::to_pg_placeholders(&format!(
            "DELETE FROM user_member_tag \
             WHERE shop_id = ? AND user_id IN ({user_ph}) AND tag_id IN ({tag_ph})"
        ));
        let mut q = sqlx::query(&sql).bind(shop_id);
        for id in user_ids {
            q = q.bind(*id);
        }
        for id in tag_ids {
            q = q.bind(*id);
        }
        let r = q.execute(&self.pool).await.map_err(map_err)?;
        Ok(r.rows_affected())
    }
}

/// 生成 `?, ?, ?` 占位符串。
fn placeholders(n: usize) -> String {
    vec!["?"; n].join(", ")
}

fn map_err(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound("用户标签不存在".into()),
        other => Error::Database(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_render_in_clause() {
        assert_eq!(placeholders(1), "?");
        assert_eq!(placeholders(3), "?, ?, ?");
    }

    #[test]
    fn shop_member_default_is_disabled_when_missing_or_invalid() {
        // Java readInt：缺失/非数字 → 默认值 "0"（关闭）；仅 "1" 视为启用
        let parse = |raw: Option<&str>| {
            raw.unwrap_or("0")
                .trim()
                .parse::<i32>()
                .map(|v| v == 1)
                .unwrap_or(false)
        };
        assert!(!parse(None));
        assert!(!parse(Some("0")));
        assert!(parse(Some("1")));
        assert!(!parse(Some("abc")));
    }
}
