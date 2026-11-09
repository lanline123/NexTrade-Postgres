//! 会员人群圈选仓储（member::member_audience）。表 `user` / `user_rank` /
//! `member_tag` / `user_member_tag`。

use sqlx::PgPool;
use nextrade_core::models::member::member_audience::Audience;
use nextrade_core::{Error, Result};

/// Java `UserTypeEnum.MEMBER`。
const USER_TYPE_MEMBER: i32 = 0;

#[derive(Clone)]
pub struct MemberAudienceRepository {
    pool: PgPool,
}

impl MemberAudienceRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 命中人数（Java `countUsers`）。
    pub async fn count_users(&self, audience: &Audience) -> Result<i64> {
        match audience {
            Audience::All => {
                let (n,): (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM \"user\" WHERE user_type = $1")
                        .bind(USER_TYPE_MEMBER)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(map_err)?;
                Ok(n)
            }
            Audience::Ranks(rank_ids) => {
                if rank_ids.is_empty() {
                    return Ok(0);
                }
                let ph = vec!["?"; rank_ids.len()].join(", ");
                let sql = crate::db::to_pg_placeholders(&format!(
                    "SELECT COUNT(*) FROM \"user\" WHERE user_type = $1 AND rank_id IN ({ph})"
                ));
                let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(USER_TYPE_MEMBER);
                for id in rank_ids {
                    q = q.bind(*id);
                }
                Ok(q.fetch_one(&self.pool)
                    .await
                    .map_err(map_err)?
                    .0)
            }
            Audience::Tags { tag_ids, match_all } => {
                if tag_ids.is_empty() {
                    return Ok(0);
                }
                let ph = vec!["?"; tag_ids.len()].join(", ");
                let sql = if *match_all {
                    format!(
                        "SELECT COUNT(*) FROM (SELECT ut.user_id FROM user_member_tag ut JOIN \"user\" u ON u.user_id = ut.user_id AND u.user_type = ? WHERE ut.tag_id IN ({ph}) GROUP BY ut.user_id HAVING COUNT(DISTINCT ut.tag_id) = ?) t"
                    )
                } else {
                    format!(
                        "SELECT COUNT(DISTINCT ut.user_id) FROM user_member_tag ut JOIN \"user\" u ON u.user_id = ut.user_id AND u.user_type = ? WHERE ut.tag_id IN ({ph})"
                    )
                };
                let sql = crate::db::to_pg_placeholders(&sql);
                let mut q = sqlx::query_as::<_, (i64,)>(&sql).bind(USER_TYPE_MEMBER);
                for id in tag_ids {
                    q = q.bind(*id);
                }
                if *match_all {
                    q = q.bind(i64::try_from(tag_ids.len()).unwrap_or(0));
                }
                Ok(q.fetch_one(&self.pool)
                    .await
                    .map_err(map_err)?
                    .0)
            }
        }
    }

    /// 解析人群命中的会员 ID（Java `MemberAudienceService#resolveUserIds`）。
    pub async fn resolve_user_ids(&self, audience: &Audience) -> Result<Vec<i32>> {
        match audience {
            Audience::All => {
                let rows: Vec<(i32,)> =
                    sqlx::query_as("SELECT user_id FROM \"user\" WHERE user_type = $1")
                        .bind(USER_TYPE_MEMBER)
                        .fetch_all(&self.pool)
                        .await
                        .map_err(map_err)?;
                Ok(rows.into_iter().map(|(id,)| id).collect())
            }
            Audience::Ranks(rank_ids) => {
                if rank_ids.is_empty() {
                    return Ok(Vec::new());
                }
                let ph = vec!["?"; rank_ids.len()].join(", ");
                let sql = crate::db::to_pg_placeholders(&format!(
                    "SELECT user_id FROM \"user\" WHERE user_type = $1 AND rank_id IN ({ph})"
                ));
                let mut q = sqlx::query_as::<_, (i32,)>(&sql).bind(USER_TYPE_MEMBER);
                for id in rank_ids {
                    q = q.bind(*id);
                }
                Ok(q.fetch_all(&self.pool)
                    .await
                    .map_err(map_err)?
                    .into_iter()
                    .map(|(id,)| id)
                    .collect())
            }
            Audience::Tags { tag_ids, match_all } => {
                if tag_ids.is_empty() {
                    return Ok(Vec::new());
                }
                let ph = vec!["?"; tag_ids.len()].join(", ");
                let sql = if *match_all {
                    format!(
                        "SELECT ut.user_id FROM user_member_tag ut JOIN \"user\" u ON u.user_id = ut.user_id AND u.user_type = ? WHERE ut.tag_id IN ({ph}) GROUP BY ut.user_id HAVING COUNT(DISTINCT ut.tag_id) = ?"
                    )
                } else {
                    format!(
                        "SELECT DISTINCT ut.user_id FROM user_member_tag ut JOIN \"user\" u ON u.user_id = ut.user_id AND u.user_type = ? WHERE ut.tag_id IN ({ph})"
                    )
                };
                let sql = crate::db::to_pg_placeholders(&sql);
                let mut q = sqlx::query_as::<_, (i32,)>(&sql).bind(USER_TYPE_MEMBER);
                for id in tag_ids {
                    q = q.bind(*id);
                }
                if *match_all {
                    q = q.bind(i64::try_from(tag_ids.len()).unwrap_or(0));
                }
                Ok(q.fetch_all(&self.pool)
                    .await
                    .map_err(map_err)?
                    .into_iter()
                    .map(|(id,)| id)
                    .collect())
            }
        }
    }

    /// 过滤出确实存在的会员 ID（Java `filterMemberIds`）。
    pub async fn filter_member_ids(&self, user_ids: &[i32]) -> Result<Vec<i32>> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; user_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!(
            "SELECT user_id FROM \"user\" WHERE user_type = $1 AND user_id IN ({ph})"
        ));
        let mut q = sqlx::query_as::<_, (i32,)>(&sql).bind(USER_TYPE_MEMBER);
        for id in user_ids {
            q = q.bind(*id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|(id,)| id)
            .collect())
    }

    /// 某会员持有的全部标签 ID（Java `userMemberTagService.listTagIdsByUser`）。
    pub async fn tag_ids_for_user(&self, user_id: i32) -> Result<Vec<i64>> {
        Ok(sqlx::query_as::<_, (i64,)>(
            "SELECT tag_id FROM user_member_tag WHERE user_id = $1 ORDER BY tag_id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?
        .into_iter()
        .map(|(id,)| id)
        .collect())
    }

    /// 等级名称（Java `userRankService.listByIds`）。
    pub async fn rank_names(&self, rank_ids: &[i32]) -> Result<Vec<String>> {
        if rank_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; rank_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT rank_name FROM user_rank WHERE rank_id IN ({ph})"));
        let mut q = sqlx::query_as::<_, (String,)>(&sql);
        for id in rank_ids {
            q = q.bind(*id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|(name,)| name)
            .collect())
    }

    /// 标签名称（Java `memberTagMapper.selectBatchIds`）。
    pub async fn tag_names(&self, tag_ids: &[i64]) -> Result<Vec<String>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ph = vec!["?"; tag_ids.len()].join(", ");
        let sql = crate::db::to_pg_placeholders(&format!("SELECT tag_name FROM member_tag WHERE id IN ({ph})"));
        let mut q = sqlx::query_as::<_, (String,)>(&sql);
        for id in tag_ids {
            q = q.bind(*id);
        }
        Ok(q.fetch_all(&self.pool)
            .await
            .map_err(map_err)?
            .into_iter()
            .map(|(name,)| name)
            .collect())
    }
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
