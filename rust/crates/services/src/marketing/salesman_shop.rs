//! 分销员外壳店「店中店」服务（marketing::salesman_shop，我方新增）。
//!
//! 形态与红线见 models 层 [`nextrade_core::models::marketing::salesman_shop`]
//! 文件头：本服务只是呈现 + 归因层——
//! - 买家侧详情只读组装（自定义三项 + 分销员昵称/头像回退）；
//! - 分销员自助保存只写 `salesman_shop` 呈现表；
//! - 平台启停只翻 `is_enabled`，**不触碰** salesman 身份、
//!   salesman_customer 绑定与 salesman_order 佣金（与分销员禁用缓做项
//!   是两套概念，禁用外壳后其分享链归因与佣金生成完全不受影响）。
//!
//! 归因不在本服务落库：买家落地由 H5 外壳页写既有 `bindSalesmanId`
//! 本地标记，注册绑定 / 订单归因 / 佣金生成全部走既有链
//! （`salesman_generation::SalesmanGenerationService`）。

use std::sync::Arc;

use nextrade_core::models::marketing::salesman_admin::{MSG_NOT_SALESMAN, MSG_UNKNOWN_SALESMAN};
use nextrade_core::models::marketing::salesman_shop::{
    normalize_save, resolve_customized, resolve_display_name, resolve_enabled, ShellShopAdminQuery,
    ShellShopAdminVO, ShellShopMineVO, ShellShopPublicVO, ShellShopSaveDTO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::salesman_shop::SalesmanShopRepository;
use nextrade_infrastructure::db::Repositories;

const DEFAULT_PAGE_SIZE: u64 = 20;
const MAX_PAGE_SIZE: u64 = 100;

fn now_i32() -> i32 {
    nextrade_common::utils::now_seconds() as i32
}

/// 分销员外壳店服务（public 端挂 `PublicState.marketing.salesman_shop`，
/// admin 端挂 `AdminState.marketing.salesman_shop`，同一实例）。
pub struct SalesmanShopService {
    repos: Arc<Repositories>,
}

impl SalesmanShopService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn shop_repo(&self) -> &SalesmanShopRepository {
        &self.repos.marketing.salesman_shop
    }

    /// 买家侧外壳店详情（公开，含游客）。
    ///
    /// # Errors
    /// 分销员不存在 → `未知的分销员`（与 `userInfo` 同文案）。
    pub async fn public_detail(&self, salesman_id: i32) -> Result<ShellShopPublicVO> {
        let admin_repo = &self.repos.marketing.salesman_admin;
        let salesman = admin_repo
            .find_salesman(salesman_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_UNKNOWN_SALESMAN))?;
        let (nickname, avatar) = match salesman.user_id.filter(|id| *id > 0) {
            Some(uid) => match admin_repo.find_public_user(uid).await? {
                Some(u) => (u.nickname.unwrap_or_default(), u.avatar),
                None => (String::new(), String::new()),
            },
            None => (String::new(), String::new()),
        };
        let row = self.shop_repo().find_by_salesman_id(salesman_id).await?;
        let (raw_name, banner, intro) = match &row {
            Some(r) => (r.shop_name.clone(), r.banner.clone(), r.intro.clone()),
            None => (String::new(), String::new(), String::new()),
        };
        Ok(ShellShopPublicVO {
            salesman_id,
            shop_name: resolve_display_name(&raw_name, &nickname),
            banner,
            intro,
            avatar,
            nickname,
            enabled: resolve_enabled(row.as_ref().map(|r| r.is_enabled)),
            customized: resolve_customized(
                row.as_ref()
                    .map(|r| (r.shop_name.as_str(), r.banner.as_str(), r.intro.as_str())),
            ),
        })
    }

    /// 当前登录用户的分销员行（自助接口共用前置）。
    async fn current_salesman(&self, user_id: i32) -> Result<nextrade_core::entities::Salesman> {
        if user_id <= 0 {
            return Err(Error::Unauthorized("未登录".into()));
        }
        self.repos
            .marketing
            .salesman_admin
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_SALESMAN))
    }

    /// 分销员自助视图：返回**原始自定义值**（未自定义三项为空串）。
    ///
    /// # Errors
    /// 未登录 → `Unauthorized`；非分销员 → `当前用户不是分销员`。
    pub async fn my_shop(&self, user_id: i32) -> Result<ShellShopMineVO> {
        let salesman = self.current_salesman(user_id).await?;
        let row = self
            .shop_repo()
            .find_by_salesman_id(salesman.salesman_id)
            .await?;
        Ok(Self::mine_vo(salesman.salesman_id, row.as_ref()))
    }

    fn mine_vo(
        salesman_id: i32,
        row: Option<&nextrade_infrastructure::db::repo::marketing::salesman_shop::SalesmanShopRow>,
    ) -> ShellShopMineVO {
        let (shop_name, banner, intro) = match row {
            Some(r) => (r.shop_name.clone(), r.banner.clone(), r.intro.clone()),
            None => (String::new(), String::new(), String::new()),
        };
        ShellShopMineVO {
            salesman_id,
            shop_name,
            banner,
            intro,
            enabled: resolve_enabled(row.map(|r| r.is_enabled)),
            customized: resolve_customized(
                row.map(|r| (r.shop_name.as_str(), r.banner.as_str(), r.intro.as_str())),
            ),
        }
    }

    /// 分销员保存三项自定义（不动平台启停开关）。
    ///
    /// # Errors
    /// 未登录 / 非分销员同 [`Self::my_shop`]；字段超长 → `Validation`。
    pub async fn save_my_shop(
        &self,
        user_id: i32,
        dto: &ShellShopSaveDTO,
    ) -> Result<ShellShopMineVO> {
        let salesman = self.current_salesman(user_id).await?;
        let (shop_name, banner, intro) =
            normalize_save(dto).map_err(|msg| Error::Validation(msg.into()))?;
        self.shop_repo()
            .upsert_profile(salesman.salesman_id, &shop_name, &banner, &intro, now_i32())
            .await?;
        self.my_shop(user_id).await
    }

    /// 平台外壳店列表（以分销员为主表，无配置行也在列）。
    ///
    /// `scope_shop_id` 为租户作用域（见 `AuthUser::scoped_shop_id`：
    /// None = 平台不限；Some(-1) 哨兵天然空集 fail-closed）。
    pub async fn admin_list(
        &self,
        scope_shop_id: Option<i32>,
        q: &ShellShopAdminQuery,
    ) -> Result<PageResult<ShellShopAdminVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let keyword = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let enabled = q.enabled.filter(|v| *v == 0 || *v == 1);
        let (total, rows) = self
            .shop_repo()
            .admin_list(scope_shop_id, keyword, enabled, limit, offset)
            .await?;
        let list = rows
            .into_iter()
            .map(|r| ShellShopAdminVO {
                salesman_id: r.salesman_id,
                user_id: r.user_id,
                nickname: r.nickname,
                mobile: r.mobile,
                salesman_shop_id: r.salesman_shop_id,
                shop_name: r.shop_name.clone(),
                banner: r.banner.clone(),
                intro: r.intro.clone(),
                enabled: resolve_enabled(r.is_enabled),
                customized: resolve_customized(Some((
                    r.shop_name.as_str(),
                    r.banner.as_str(),
                    r.intro.as_str(),
                ))),
                update_time: r.update_time,
            })
            .collect();
        Ok(PageResult::new(total.max(0) as u64, list, &page))
    }

    /// 平台启停某外壳店：只翻展示开关，不动身份 / 绑定 / 佣金。
    ///
    /// # Errors
    /// 分销员不存在 → `未知的分销员`；租户作用域外 → `Forbidden`。
    pub async fn admin_set_enabled(
        &self,
        scope_shop_id: Option<i32>,
        salesman_id: i32,
        enabled: bool,
    ) -> Result<()> {
        let salesman = self
            .repos
            .marketing
            .salesman_admin
            .find_salesman(salesman_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_UNKNOWN_SALESMAN))?;
        if let Some(scope) = scope_shop_id {
            if salesman.shop_id.unwrap_or(0) != scope {
                return Err(Error::Forbidden("无权操作其他店铺的分销员小店".into()));
            }
        }
        self.shop_repo()
            .set_enabled(salesman_id, enabled, now_i32())
            .await
    }
}
