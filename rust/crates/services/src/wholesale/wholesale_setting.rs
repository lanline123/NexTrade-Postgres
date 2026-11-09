//! 批发设置域服务（wholesale::wholesale_setting）。
//!
//! 对应原 Java `com.nextrade.service.wholesale.impl.WholesaleSettingServiceImpl`
//! （174 行）。
//!
//! # 端点 → 方法
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java | 方法 |
//! |---------------------------|---------|------|
//! | `GET  /wholesale/setting/get` | `WholesaleSettingController.get` | [`WholesaleSettingService::get`] |
//! | `POST /wholesale/setting/save` | `WholesaleSettingController.save` | [`WholesaleSettingService::save`] |
//!
//! # 语义（逐条对照 Java）
//!
//! - `get`：读 4 个配置项（缺失 / 非法 → 默认 `0`）→ 回填 `defaultGradeName`
//!   （默认等级存在**且启用**才回填，查询异常静默忽略，保证设置页可进入修复）
//!   → `isMultiShop`（v2 恒 `false`）。
//! - `save`：`@Valid` 4 项必填 → 3 项前置校验（5330 多店铺 / 5331 未配置默认
//!   等级 / 5303·5304 默认等级不存在或禁用）→ 原子写入 4 项配置。
//!
//! # 与 Java 的差异（已写入 models 模块头）
//!
//! 1. 多店铺 `isMultiShop()` 在 v2 恒 `false`，5330 链路保留但不可达；
//! 2. `ConfigValCache` / `wholesalerService.invalidateAllBuyerCache()` 无
//!    Redis 写路径，未迁移（写后直接读库即新值）；
//! 3. 4 项写入未显式包事务（Java `@Transactional`）：单表 4 行更新，
//!    逐条执行；任一失败返回错误，已写行保留（见 `uncertain_points`）。
//!
//! # 金额
//!
//! 本域不涉及任何金额列，故无 `rust_decimal::Decimal`。

use std::sync::Arc;

use nextrade_core::models::wholesale::wholesale_setting::{
    parse_int_or_default, validate_business_rules, validate_save, WholesaleSettingSaveDTO,
    WholesaleSettingVO,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::wholesale::wholesale_setting::WholesaleSettingRepository;
use nextrade_infrastructure::db::Repositories;

/// 批发设置域服务（挂载于 `AdminState.wholesale.wholesale_setting`）。
pub struct WholesaleSettingService {
    repos: Arc<Repositories>,
}

impl WholesaleSettingService {
    /// 构造（与同域其它 feature 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &WholesaleSettingRepository {
        &self.repos.wholesale.wholesale_setting
    }

    /// 查询当前批发设置（对应 Java `WholesaleSettingServiceImpl#get`）。
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn get(&self) -> Result<WholesaleSettingVO> {
        let [enabled, auto_register, default_grade_id, give_points] = self.repo().read_all().await?;
        let values = [enabled, auto_register, default_grade_id, give_points];
        let mut vo = build_vo(values.each_ref().map(|v| v.as_deref()));
        // 默认等级名称回填：找不到 / 已禁用 / 查询异常 → 保持 null（Java try/catch 静默）
        if vo.wholesale_default_grade_id > 0 {
            if let Ok(Some((_, name, status))) =
                self.repo().find_grade(vo.wholesale_default_grade_id).await
            {
                if status == 1 {
                    vo.default_grade_name = Some(name);
                }
            }
        }
        Ok(vo)
    }

    /// 保存批发设置（对应 Java `WholesaleSettingServiceImpl#save`）。
    ///
    /// # Errors
    /// - `@Valid` 缺字段 → `Error::Validation(<Java 注解文案>)`
    /// - 业务校验 → `Error::biz(5303/5304/5330/5331, <Java 文案>)`
    /// - 数据库执行失败 → `Error::Database`
    pub async fn save(&self, dto: &WholesaleSettingSaveDTO) -> Result<()> {
        validate_save(dto).map_err(Error::Validation)?;

        // 默认等级有效性（仅 defaultGradeId > 0 时才查）
        let default_grade_id = dto.wholesale_default_grade_id.unwrap_or(0);
        let grade_enabled = if default_grade_id > 0 {
            self.repo().grade_enabled(default_grade_id).await?
        } else {
            None
        };
        validate_business_rules(dto, false, grade_enabled)
            .map_err(|(code, msg)| Error::biz(code, msg))?;

        let now = now_secs();
        self.repo()
            .write_int(
                nextrade_core::models::wholesale::wholesale_setting::BIZ_WHOLESALE_ENABLED,
                dto.wholesale_enabled.unwrap_or_default(),
                now,
            )
            .await?;
        self.repo()
            .write_int(
                nextrade_core::models::wholesale::wholesale_setting::BIZ_WHOLESALE_AUTO_REGISTER,
                dto.wholesale_auto_register.unwrap_or_default(),
                now,
            )
            .await?;
        self.repo()
            .write_int(
                nextrade_core::models::wholesale::wholesale_setting::BIZ_WHOLESALE_DEFAULT_GRADE_ID,
                dto.wholesale_default_grade_id.unwrap_or_default(),
                now,
            )
            .await?;
        self.repo()
            .write_int(
                nextrade_core::models::wholesale::wholesale_setting::BIZ_WHOLESALE_GIVE_POINTS,
                dto.wholesale_give_points.unwrap_or_default(),
                now,
            )
            .await?;
        tracing::info!("批发设置已保存: enabled={:?}", dto.wholesale_enabled);
        Ok(())
    }
}

/// 由 4 个原始配置串构造 VO（Java `readInt` × 4 + `isMultiShop`）。
///
/// `is_multi_shop` 在 v2 恒 `false`（Java A2 兜底方案）。
#[must_use]
fn build_vo(values: [Option<&str>; 4]) -> WholesaleSettingVO {
    WholesaleSettingVO {
        wholesale_enabled: parse_int_or_default(values[0]),
        wholesale_auto_register: parse_int_or_default(values[1]),
        wholesale_default_grade_id: parse_int_or_default(values[2]),
        wholesale_give_points: parse_int_or_default(values[3]),
        default_grade_name: None,
        is_multi_shop: false,
    }
}

/// 当前 Unix 秒（`config.create_time` / `update_time` 为 `bigint` 秒级）。
fn now_secs() -> i64 {
    i64::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::wholesale::wholesale_setting::{
        MSG_DEFAULT_GRADE_NOT_CONFIGURED, MSG_ENABLED_REQUIRED,
    };

    /// 配置缺失 / 非法 → 默认 0，字段名与 Java VO 一致。
    #[test]
    fn build_vo_uses_defaults_for_missing_values() {
        let vo = build_vo([None, Some(""), Some("abc"), Some(" 1 ")]);
        assert_eq!(vo.wholesale_enabled, 0);
        assert_eq!(vo.wholesale_auto_register, 0);
        assert_eq!(vo.wholesale_default_grade_id, 0);
        assert_eq!(vo.wholesale_give_points, 1);
        assert!(vo.default_grade_name.is_none());
        assert!(!vo.is_multi_shop);

        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["wholesaleEnabled"], 0);
        assert_eq!(json["wholesaleGivePoints"], 1);
        assert_eq!(json["isMultiShop"], false);
    }

    /// `save` 的校验顺序与文案（先 @Valid，再业务规则）。
    #[test]
    fn save_validation_messages_and_order() {
        let empty = WholesaleSettingSaveDTO::default();
        assert_eq!(validate_save(&empty).unwrap_err(), MSG_ENABLED_REQUIRED);

        let dto = WholesaleSettingSaveDTO {
            wholesale_enabled: Some(1),
            wholesale_auto_register: Some(0),
            wholesale_default_grade_id: Some(0),
            wholesale_give_points: Some(0),
        };
        assert_eq!(
            validate_business_rules(&dto, false, None).unwrap_err(),
            (5331, MSG_DEFAULT_GRADE_NOT_CONFIGURED.to_string())
        );
    }
}
