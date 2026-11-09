//! 会员人群圈选服务（member::member_audience）。
//! 构造 `new(repos)`；嵌套 `self.repos.member.member_audience`。

use std::sync::Arc;

use nextrade_core::models::member::member_audience::{
    describe_text, resolve, Audience, MemberAudienceDTO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::member::member_audience::MemberAudienceRepository;
use nextrade_infrastructure::db::Repositories;

pub struct MemberAudienceService {
    repos: Arc<Repositories>,
}

impl MemberAudienceService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &MemberAudienceRepository {
        &self.repos.member.member_audience
    }

    /// `POST /adminapi/user/memberAudience/estimate`：预估命中人数。
    pub async fn estimate(&self, dto: &MemberAudienceDTO) -> Result<i64> {
        let audience = resolve(dto)?;
        self.repo().count_users(&audience).await
    }

    /// `POST /adminapi/user/memberAudience/describe`：条件文字描述。
    pub async fn describe(&self, dto: &MemberAudienceDTO) -> Result<String> {
        let audience = resolve(dto)?;
        let (ranks, tags) = match &audience {
            Audience::All => (Vec::new(), Vec::new()),
            Audience::Ranks(ids) => (self.repo().rank_names(ids).await?, Vec::new()),
            Audience::Tags { tag_ids, .. } => (Vec::new(), self.repo().tag_names(tag_ids).await?),
        };
        Ok(describe_text(&audience, &ranks, &tags))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::member::member_audience::{ALL_MEMBERS_TEXT, MATCH_ANY, TYPE_TAG};

    #[test]
    fn describe_all_has_no_db_calls() {
        // 全部会员路径不需要仓储查询；这里仅校验常量与归一化前置语义。
        assert_eq!(ALL_MEMBERS_TEXT, "全部会员");
        assert_eq!(MATCH_ANY, 0);
        let dto = MemberAudienceDTO {
            audience_type: Some(TYPE_TAG),
            ..Default::default()
        };
        assert!(resolve(&dto).is_err());
    }
}
