//! 客服接入规则 域服务（im::admin_shop_kefu_config）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；仓储经嵌套
//! 聚合根访问：`self.repos.im.admin_shop_kefu_config`。
//!
//! 对应 Java `ShopKefuConfigServiceImpl`
//! （`java/nextrade-service/.../im/impl/ShopKefuConfigServiceImpl.java`）：
//! - `getAccessConfig(shopId)`：无配置时返回 `createDefault` 的 0/0 默认值（不落库）
//! - `saveAccessConfig(dto)`：无配置则插入（缺省 0），有配置则只更新非空字段

use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::models::im::admin_shop_kefu_config::{
    KefuAccessConfigSaveDTO, KefuAccessConfigVO, DEFAULT_AUTO_ACCESS_MODE,
    DEFAULT_RETURN_CUSTOMER_ACCESS,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::im::admin_shop_kefu_config::{
    AdminShopKefuConfigRepository, ShopKefuAccessRow,
};
use nextrade_infrastructure::db::Repositories;

/// 客服接入规则 域服务。
pub struct AdminShopKefuConfigService {
    repos: Arc<Repositories>,
}

impl AdminShopKefuConfigService {
    /// 构造服务（组合根唯一装配点）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本 feature 仓储。
    #[must_use]
    pub fn repo(&self) -> &AdminShopKefuConfigRepository {
        &self.repos.im.admin_shop_kefu_config
    }

    /// 获取接入规则（Java `getAccessConfig`）。
    pub async fn get_access_config(&self, shop_id: i32) -> Result<KefuAccessConfigVO> {
        let row = self.repo().find_by_shop_id(shop_id).await?;
        let (auto_access_mode, return_customer_access) = apply_access_defaults(row.as_ref());
        Ok(KefuAccessConfigVO {
            shop_id,
            auto_access_mode,
            return_customer_access,
        })
    }

    /// 保存接入规则（Java `saveAccessConfig`）。
    pub async fn save(&self, dto: &KefuAccessConfigSaveDTO, shop_id: i32) -> Result<()> {
        let now = now_secs();
        match self.repo().find_by_shop_id(shop_id).await? {
            Some(row) => {
                self.repo()
                    .update(row.id, dto.auto_access_mode, dto.return_customer_access, now)
                    .await
            }
            None => {
                self.repo()
                    .insert(
                        shop_id,
                        dto.auto_access_mode.unwrap_or(DEFAULT_AUTO_ACCESS_MODE),
                        dto.return_customer_access.unwrap_or(DEFAULT_RETURN_CUSTOMER_ACCESS),
                        now,
                    )
                    .await
            }
        }
    }
}

/// 配置行 → 两个开关值；无配置时取 Java `createDefault` 的 0/0。
#[must_use]
pub fn apply_access_defaults(row: Option<&ShopKefuAccessRow>) -> (i32, i32) {
    match row {
        Some(r) => (r.auto_access_mode, r.return_customer_access),
        None => (DEFAULT_AUTO_ACCESS_MODE, DEFAULT_RETURN_CUSTOMER_ACCESS),
    }
}

/// 当前时间（秒级时间戳，Java `StringUtils.getCurrentTime()`；本表列为 int(11)）。
fn now_secs() -> i32 {
    i32::try_from(utils::now_millis() / 1000).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(auto: i32, ret: i32) -> ShopKefuAccessRow {
        ShopKefuAccessRow {
            id: 9,
            shop_id: 3,
            auto_access_mode: auto,
            return_customer_access: ret,
        }
    }

    #[test]
    fn missing_row_yields_java_defaults() {
        assert_eq!(apply_access_defaults(None), (0, 0));
    }

    #[test]
    fn existing_row_is_returned_verbatim() {
        let r = row(2, 1);
        assert_eq!(apply_access_defaults(Some(&r)), (2, 1));
    }

    #[test]
    fn now_secs_is_epoch_seconds() {
        assert!(now_secs() > 1_600_000_000);
    }
}
