//! 用户签到服务（member::user_sign）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.user_sign`。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::member::user_sign::{
    next_sign_num, sign_points, UserSignInfoVO, ALREADY_SIGNED, SERVICE_DATA_ERROR, SIGN_FAILED,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_sign::UserSignRepository;
use nextrade_infrastructure::db::Repositories;

pub struct UserSignService {
    repos: Arc<Repositories>,
}

impl UserSignService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &UserSignRepository {
        &self.repos.member.user_sign
    }

    /// `GET /api/user/sign/index`。
    pub async fn index(&self, user_id: i32) -> Result<UserSignInfoVO> {
        let now = now_seconds();
        let today_start = Self::today_start(now);
        let days = self.repo().settings().await?;
        let record = self.repo().last_sign_num(user_id).await?;
        let is_sign = if self.repo().signed_since(user_id, today_start).await? {
            1
        } else {
            0
        };
        let next = next_sign_num(record, i32::try_from(days.len()).unwrap_or(0));
        let points = sign_points(&days, next, is_sign);
        let recommend_goods = self.repo().recommend_products().await?;
        Ok(UserSignInfoVO {
            days,
            record,
            is_sign,
            sign_points: points,
            recommend_goods,
        })
    }

    /// `GET /api/user/sign/sign`：今日签到 + 积分奖励。
    pub async fn sign(&self, user_id: i32) -> Result<()> {
        let now = now_seconds();
        let today_start = Self::today_start(now);
        if self.repo().signed_since(user_id, today_start).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, ALREADY_SIGNED));
        }
        let days = self.repo().settings().await?;
        let total = i32::try_from(days.len()).unwrap_or(0);
        let record = self.repo().last_sign_num(user_id).await?;
        let sign_num = next_sign_num(record, total);
        if !self
            .repo()
            .insert(user_id, sign_num, i32::try_from(now).unwrap_or(0))
            .await?
        {
            return Err(Error::biz(SERVICE_DATA_ERROR, SIGN_FAILED));
        }
        let points = sign_points(&days, sign_num, 0);
        if points > 0 {
            self.repo().inc_points(user_id, points).await?;
        }
        Ok(())
    }

    /// 东八区今日 00:00 秒级时间戳（关联函数，便于纯逻辑测试）。
    fn today_start(now: i64) -> i64 {
        let shifted = now + 8 * 3600;
        shifted - shifted % 86_400 - 8 * 3600
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_start_is_local_midnight() {
        let start = UserSignService::today_start(1_700_000_000);
        assert_eq!(start % 86_400, 16 * 3600);
        assert!(start <= 1_700_000_000);
        assert!(1_700_000_000 - start < 86_400);
        let start2 = UserSignService::today_start(1_700_000_000 + 86_400);
        assert_eq!(start2 - start, 86_400);
    }
}
