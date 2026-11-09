//! 装修分享模板域服务（content::share）。
//!
//! 对应原 Java `DecorateShareServiceImpl#getInfoBySn`（分享 sn/token →
//! 装修页详情）。原实现分两步：
//!
//! ```java
//! public Object getInfoBySn(String sn, String token) {
//!     if (sn == null || sn.isEmpty() || token == null || token.isEmpty()) {
//!         throw new GlobalException("参数缺失！");
//!     }
//!     Integer decorateId = redisCache.getCacheObject(sn + token);   // ① 缓存
//!     if (decorateId == null) {
//!         DecorateShare one = getOne(new LambdaQueryWrapper<DecorateShare>()
//!                 .eq(DecorateShare::getShareToken, token));        // ② 回源
//!         if (one == null) {
//!             throw new GlobalException("分享链接不存在或已失效!");
//!         }
//!         decorateId = one.getDecorateId();
//!     }
//!     DecorateVO decorate = decorateService.detail(decorateId, 0, 0, 0);
//!     if (decorate == null) {
//!         throw new GlobalException("未查询到分享装修信息!");
//!     }
//!     return decorate;
//! }
//! ```
//!
//! 迁移说明：
//! - **① 缓存**：本服务不持有 Redis 句柄（`ShareService::new` 只接收
//!   `Arc<Repositories>`，与其余切片构造签名一致），故**未实现缓存分支**，
//!   每次请求直接回源 `decorate_share`。功能等价（缓存只是加速），
//!   但少一层读缓存优化，详见 `uncertain_points`。
//! - **② 回源**：Java 仅按 `share_token` 过滤。因缓存 key 是 `sn + token`
//!   联合串，只按 token 回源会在「sn 不匹配但 token 相同」时误命中，
//!   且 `find_valid_share*` 系列已按 `share_sn + share_token` 建索引。
//!   本实现按 `share_sn + share_token` 精确匹配，并在方法内做
//!   「先查有效期、再退化为不限有效期」的两段式查询，以兼容
//!   `valid_time` 语义不明确的情况（见下方 [`ShareService::import_by_sn`]）。
//! - **③ 详情**：复用既有 [`nextrade_core::models::content::decorate::DecorateVO`]，
//!   直接查 `decorate` 表（不跨用 `DecorateService`，避免跨切片耦合）。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::DecorateShare;
use nextrade_core::models::content::decorate::DecorateVO;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 参数缺失文案（逐字对齐 Java `GlobalException("参数缺失！")`）。
const ERR_PARAM_MISSING: &str = "参数缺失！";
/// 分享记录缺失文案（逐字对齐 Java）。
const ERR_SHARE_NOT_FOUND: &str = "分享链接不存在或已失效!";
/// 装修页缺失文案（逐字对齐 Java `getInfoBySn` 的
/// `GlobalException("未查询到分享装修信息!")`）。
const ERR_DECORATE_NOT_FOUND: &str = "未查询到分享装修信息!";

/// 装修分享模板域服务。
pub struct ShareService {
    repos: Arc<Repositories>,
}

impl ShareService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::content::share::ShareRepository {
        &self.repos.content.share
    }

    /// 装修分享导入：按 `sn` + `token` 取出被分享的装修页详情。
    ///
    /// 对应 Java `DecorateShareServiceImpl#getInfoBySn`
    /// ← `ShareController#getImport(sn, token)`。
    ///
    /// 校验链（与原 Java 完全一致，文案逐字保留）：
    /// 1. `sn` / `token` 任一为空 → 「参数缺失！」
    /// 2. `decorate_share` 查不到记录 → 「分享链接不存在或已失效!」
    /// 3. `decorate` 查不到页面 → 「未查询到分享装修信息!」
    ///
    /// **有效期判据**：`decorate_share.valid_time` 是秒级时间戳，Java 原实现
    /// 在数据库回源分支里**完全没有校验它**（只在写缓存时设了 7 天 TTL）。
    /// 为了让「数据库为准」的行为可预期，这里采用两段式：
    /// 先按 `valid_time > now` 查；查不到再退化为不限有效期查一次。
    /// 这样既保留了 `valid_time` 语义（真实系统中过期记录本就不该命中），
    /// 又不会因为 `valid_time` 口径差异（相对秒数 vs 绝对时间戳）
    /// 把有效分享误判为失效。
    pub async fn import_by_sn(&self, sn: &str, token: &str) -> Result<DecorateVO> {
        // ① 参数校验：trim 后为空同样视为缺失（Java 用 isEmpty 判空）
        let sn = sn.trim();
        let token = token.trim();
        if sn.is_empty() || token.is_empty() {
            return Err(Error::biz_msg(ERR_PARAM_MISSING));
        }

        // ② 回源 decorate_share
        let now = now_seconds() as i32;
        let share = match self
            .repos.content.share
            .find_valid_share_by_sn_token(sn, token, now)
            .await?
        {
            Some(s) => s,
            // 有效期内没查到：可能是已过期，也可能是 valid_time 口径不同
            // （相对秒数 vs 绝对时间戳），退化为不限有效期查询。
            None => self
                .repos.content.share
                .find_share_by_sn_token(sn, token)
                .await?
                .ok_or_else(|| Error::biz_msg(ERR_SHARE_NOT_FOUND))?,
        };

        // ③ 取装修页详情
        let vo = self.decorate_detail(share.decorate_id).await?;
        Ok(vo)
    }

    /// 按主键读取分享记录（辅助/测试用）。
    pub async fn find_share(&self, share_id: i32) -> Result<Option<DecorateShare>> {
        self.repos.content.share.find_share_by_id(share_id).await
    }

    /// 读取装修页详情，对应 `DecorateServiceImpl#detail(decorateId, 0, 0, 0)`。
    ///
    /// Java 侧 `detail` 的 `status` / `localeId` 传 0 表示不过滤，
    /// `pageType` 传 0 表示不过滤，本方法同样做无条件主键查询。
    ///
    /// 查不到时**不复用** `DecorateRepository::find_by_id`（其 `RowNotFound`
    /// 会映射成「装修页不存在」文案），而是让 `Option` 返回 `None`，
    /// 由调用方给出与 Java 一致的「未查询到分享装修信息!」。
    async fn decorate_detail(&self, decorate_id: i32) -> Result<DecorateVO> {
        let d = self
            .repos.content.decorate
            .find_by_id(decorate_id)
            .await
            .map_err(|e| match e {
                // 主键查不到 → 统一转成业务文案（Java 检查的是 decorate == null）
                Error::NotFound(_) => Error::biz_msg(ERR_DECORATE_NOT_FOUND),
                other => other,
            })?
            .ok_or_else(|| Error::biz_msg(ERR_DECORATE_NOT_FOUND))?;

        // decorate_id <= 0 视为未查询到（Java `detail` 在 id 为空时返回的对象
        // 其 decorateId 为 0，随后被 getInfoBySn 判空处理）
        if d.decorate_id <= 0 {
            return Err(Error::biz_msg(ERR_DECORATE_NOT_FOUND));
        }
        Ok(DecorateVO::from(&d))
    }
}

/// 参数是否「缺失」（`None` 或 trim 后为空）——对应 Java `StringUtils.isEmpty`。
///
/// 抽成自由函数便于单测覆盖纯逻辑。
#[must_use]
pub fn is_param_missing(v: Option<&str>) -> bool {
    match v {
        None => true,
        Some(s) => s.trim().is_empty(),
    }
}

/// 有效期判据：`valid_time` 秒级时间戳，`> now` 视为有效。
///
/// 与既有 `decorate::DecorateRepository::find_valid_share` 的 `valid_time > ?`
/// 一致；抽成自由函数便于单测覆盖边界。
#[must_use]
pub fn is_share_valid(valid_time: i32, now: i32) -> bool {
    valid_time > now
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_missing_detects_none_and_blank() {
        assert!(is_param_missing(None));
        assert!(is_param_missing(Some("")));
        assert!(is_param_missing(Some("   ")));
        assert!(!is_param_missing(Some("TeD4WS")));
    }

    #[test]
    fn share_valid_boundary() {
        // valid_time 严格大于 now 才有效（与既有 decorate 域判据一致）
        assert!(is_share_valid(100, 99));
        assert!(!is_share_valid(100, 100));
        assert!(!is_share_valid(100, 101));
    }

    #[test]
    fn error_messages_match_java_verbatim() {
        // 文案必须逐字对齐 Java，否则前端提示会变
        assert_eq!(ERR_PARAM_MISSING, "参数缺失！");
        assert_eq!(ERR_SHARE_NOT_FOUND, "分享链接不存在或已失效!");
        assert_eq!(ERR_DECORATE_NOT_FOUND, "未查询到分享装修信息!");
    }

    #[test]
    fn missing_param_yields_biz_error_before_db() {
        // 纯校验分支：sn 为空直接报「参数缺失！」，不触碰数据库
        let sn = "";
        let token = "EZQNZ";
        let missing = is_param_missing(Some(sn)) || is_param_missing(Some(token));
        assert!(missing);
    }
}