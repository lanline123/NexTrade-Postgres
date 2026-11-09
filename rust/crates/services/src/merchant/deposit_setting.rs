//! 店铺保证金设置 域服务（merchant::deposit_setting）—— wave9。

use std::sync::Arc;

use nextrade_core::models::merchant::deposit_setting::DepositSetting;
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::merchant::deposit_setting::DepositSettingRepository;
use nextrade_infrastructure::db::Repositories;

/// 保证金设置 域服务。
pub struct DepositSettingService {
    repos: Arc<Repositories>,
}

impl DepositSettingService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &DepositSettingRepository {
        &self.repos.merchant.deposit_setting
    }

    /// 设置详情（Java `DepositSettingController.detail`）。
    pub async fn detail(&self) -> Result<DepositSetting> {
        let rows = self.repo().find_settings().await?;
        Ok(DepositSetting::from_rows(&rows))
    }

    /// 保存设置（Java `DepositSettingController.save`；仅写非空字段）。
    pub async fn save(&self, dto: &DepositSetting) -> Result<()> {
        self.repo().save_settings(&dto.to_pairs()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::merchant::deposit_setting::{
        CODE_DEPOSIT_ENABLE, CODE_PROTOCOL_CONTENT,
    };

    #[test]
    fn from_rows_defaults_missing_fields() {
        let s = DepositSetting::from_rows(&[]);
        assert_eq!(s.deposit_enable, None);
        assert_eq!(s.deposit_protocol_content, None);
    }

    #[test]
    fn to_pairs_only_non_null() {
        let mut s = DepositSetting::default();
        s.deposit_enable = Some(1);
        s.deposit_protocol_content = Some("内容".into());
        let pairs = s.to_pairs();
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|(c, _)| c == CODE_DEPOSIT_ENABLE));
        assert!(pairs.iter().any(|(c, _)| c == CODE_PROTOCOL_CONTENT));
    }

    #[test]
    fn int_fields_parse_from_rows() {
        let rows = vec![
            (CODE_DEPOSIT_ENABLE.to_string(), "1".to_string()),
            ("depositCollectBufferDays".to_string(), "7".to_string()),
        ];
        let s = DepositSetting::from_rows(&rows);
        assert_eq!(s.deposit_enable, Some(1));
        assert_eq!(s.deposit_collect_buffer_days, Some(7));
    }
}
