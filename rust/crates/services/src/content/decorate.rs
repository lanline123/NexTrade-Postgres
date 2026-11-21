//! 页面装修域服务：装修页、PC 导航、分类楼层、手机导航、装修分享、
//! 装修组件与前台装修数据读取。
//!
//! 对照原 Java：
//! - `DecorateServiceImpl`（装修页 CRUD / 草稿 / 发布 / 复制 / 设首页 / 分享信息）
//! - `DecorateDiscreteServiceImpl`（装修组件）
//! - `DecorateShareServiceImpl`（页面分享 sn/token）
//! - `PcNavigationServiceImpl` / `PcCatFloorServiceImpl` / `MobileCatNavServiceImpl`
//! - `HomeController#index` / `#pcIndex`、`ShareController#import` 的前台读取路径

use std::sync::Arc;

use nextrade_common::utils::{now_seconds, random_hex};
use nextrade_core::entities::{Decorate, DecorateDiscrete, DecorateShare, MobileCatNav};
use nextrade_core::models::content::decorate::{
    parse_json_opt, DecorateDiscreteMenuItemDTO, DecorateDiscreteUpdateParam, DecorateDiscreteVO,
    DecorateChildVO, DecorateCreateDTO, DecorateListQuery, DecorateMemberDataVO,
    DecorateModuleListVO, DecorateSaveDTO, DecorateShareInfoDTO, DecorateShareInfoVO,
    DecorateShareListQuery, DecorateShareVO, DecorateShopOptionVO, DecorateUpdateDTO, DecorateVO,
    HomeDecorateQuery,
    MobileCatNavCreateDTO, MobileCatNavListQuery, MobileCatNavUpdateDTO, MobileCatNavVO,
    PcCatFloorCreateDTO, PcCatFloorListQuery, PcCatFloorUpdateDTO, PcCatFloorVO, PcNavTreeVO,
    PcNavigationCreateDTO, PcNavigationListQuery, PcNavigationUpdateDTO, PcNavigationVO,
    share_api_url,
};
use nextrade_core::models::{BatchDTO, OperateDTO, UpdateFieldDTO};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::decorate::DecorateFilter;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

use crate::content::tenant_scope::{
    ensure_media_access, ensure_media_write, media_write_shop_id,
};

/// 分享 token 有效期：7 天（对应 `DecorateShareConstants.EXPIRE_TIME`）。
const SHARE_EXPIRE_SECONDS: i32 = 60 * 60 * 24 * 7;
/// 分享 sn 长度（`DecorateShareConstants.CODE_LENGTH`）。
const SHARE_CODE_LENGTH: usize = 6;
/// 分享 token 长度（`DecorateShareConstants.TOKEN_CODE_LENGTH`）。
const SHARE_TOKEN_LENGTH: usize = 5;
/// 平台默认店铺 id：Java `HeaderUtils.getShopId()` 单店铺场景为 0。
const DEFAULT_SHOP_ID: i32 = 0;
/// 复制页标题后缀（`DecorateServiceImpl#generateCopyTitle`）。
const COPY_SUFFIX: &str = "-副本";

/// 默认页判定：`is_home = 1` 的装修页为其页面类型的默认页。
/// 批删护栏（我方改口径，见 `DecorateService::batch`）与前端勾选禁用共用此口径。
fn is_home_page(row: &Decorate) -> bool {
    row.is_home.unwrap_or(0) == 1
}

pub struct DecorateService {
    repos: Arc<Repositories>,
}

impl DecorateService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 装修页（管理端） —————————————————————

    /// 装修页分页列表，对应 `DecorateController#list` + `DecorateServiceImpl#list`。
    ///
    /// 控制器默认值：`parentId = 0`、`pageType = [1]`。
    pub async fn list(&self, q: &DecorateListQuery) -> Result<PageResult<DecorateVO>> {
        let mut f = Self::build_filter(q);
        if f.parent_id.is_none() {
            f.parent_id = Some(0);
        }
        if f.page_types.is_empty() {
            f.page_types = vec![1];
        }
        let page = q.page_query();
        let (total, rows) = self.repos.content.decorate.paginate(&f, &page).await?;
        let list = self.to_vos(rows).await?;
        Ok(PageResult::new(total, list, &page))
    }

    /// 不分页全量列表（`getH5DecorateList` / `getCustomDecorateList` 复用同一过滤）。
    ///
    /// 与主列表不同：不强制 `parentId = 0` / `pageType = [1]` 默认值
    /// （Java 该路径直接进 service，不经过控制器默认值填充）。
    pub async fn list_all(&self, q: &DecorateListQuery) -> Result<Vec<DecorateVO>> {
        let f = Self::build_filter(q);
        let page = PageQuery {
            page: 1,
            page_size: 200,
        };
        let (_, rows) = self.repos.content.decorate.paginate(&f, &page).await?;
        self.to_vos(rows).await
    }

    /// 装修页详情，对应 `DecorateServiceImpl#detail`。
    ///
    /// - `id` 为空/0 时按 `parentId + localeId + decorateType` 定位编辑中的子页
    /// - 草稿为空时用已发布数据兜底，保证编辑器有内容可编
    /// - 回填多语言子页 `children`（含语言名）
    pub async fn detail(
        &self,
        id: Option<i32>,
        locale_id: Option<i32>,
        parent_id: Option<i32>,
        decorate_type: Option<i32>,
    ) -> Result<DecorateVO> {
        let decorate = match id.filter(|v| *v > 0) {
            Some(id) => self.repos.content.decorate.find_by_id(id).await?,
            None => {
                self.repos.content.decorate
                    .find_by_parent_locale(parent_id.unwrap_or(0), locale_id, decorate_type)
                    .await?
            }
        };
        let Some(decorate) = decorate else {
            // Java 找不到时返回空 VO
            return Ok(DecorateVO::default());
        };
        // 多租户：平台装修页可读；店铺装修页仅本店可读
        ensure_media_access(decorate.shop_id)?;

        let mut vo = DecorateVO::from(&decorate);
        // 草稿为空 → 回退已发布数据作为编辑基准
        if vo.draft_data.is_none() {
            vo.draft_data = vo.data.clone();
        }
        // 绑定的 H5 页面名称
        if let Some(h5_id) = decorate.binded_h5_id.filter(|v| *v > 0) {
            if let Some(h5) = self.repos.content.decorate.find_by_id(h5_id).await? {
                vo.bingding_h5_name = h5.decorate_title.clone();
            }
        }
        // 多语言子页
        let children = self.repos.content.decorate.find_children(decorate.decorate_id).await?;
        if !children.is_empty() {
            let mut items = Vec::with_capacity(children.len());
            for child in &children {
                items.push(DecorateChildVO {
                    decorate_id: child.decorate_id,
                    locale_id: child.locale_id,
                    language: self
                        .repos.content.decorate
                        .find_locale_language(child.locale_id)
                        .await?,
                });
            }
            vo.children = Some(items);
        }
        Ok(vo)
    }

    /// 新建装修页，对应 `DecorateServiceImpl#create`。
    ///
    /// 返回新页 id（前端保存/预览立即使用）；H5 登录页（pageType=5）强制置为首页
    /// （对应 `markMobileLoginAsHome`：移动端登录页是平台唯一入口）。
    pub async fn create(&self, dto: &DecorateCreateDTO) -> Result<i32> {
        let title = dto
            .decorate_title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| Error::Validation("装修页面名称不能为空".into()))?;
        let decorate_type = dto.decorate_type.unwrap_or(1);
        if !matches!(decorate_type, 1 | 2 | 3) {
            return Err(Error::Validation("无效的页面类型".into()));
        }

        let mut d = Decorate {
            decorate_title: Some(title.to_string()),
            data: dto.data.clone(),
            draft_data: dto.draft_data.clone(),
            decorate_type: Some(decorate_type),
            page_type: dto.page_type.unwrap_or(1),
            is_home: Some(dto.is_home.unwrap_or(0)),
            // 多租户：店铺账号强制落本店；平台账号请求值优先，缺省平台（0）
            shop_id: media_write_shop_id(dto.shop_id)?,
            status: Some(dto.status.unwrap_or(0)),
            update_time: Some(now_seconds() as i32),
            locale_id: Some(dto.locale_id.unwrap_or(0)),
            parent_id: Some(dto.parent_id.unwrap_or(0)),
            sort_order: dto.sort_order,
            ..Decorate::default()
        };
        Self::mark_mobile_login_as_home(&mut d);
        self.repos.content.decorate.insert(&d).await
    }

    /// 更新装修页，对应 `DecorateServiceImpl#update`。
    ///
    /// `id > 0`：只覆盖入参非空字段（对齐 MyBatis-Plus `updateById` 忽略 null）；
    /// `id` 为空：以 `parentId` 为父新建多语言子页。
    pub async fn update(&self, dto: &DecorateUpdateDTO) -> Result<bool> {
        if let Some(id) = dto.decorate_id.filter(|v| *v > 0) {
            let Some(existing) = self.repos.content.decorate.find_by_id(id).await? else {
                return Err(Error::NotFound("装修页面不存在".into()));
            };
            // 多租户：仅本店装修页可改（平台装修页对店铺账号只读）
            ensure_media_write(existing.shop_id)?;
            let mut d = existing;
            if dto.decorate_title.is_some() {
                d.decorate_title = dto.decorate_title.clone();
            }
            if dto.data.is_some() {
                d.data = dto.data.clone();
            }
            if dto.draft_data.is_some() {
                d.draft_data = dto.draft_data.clone();
            }
            if dto.decorate_type.is_some() {
                d.decorate_type = dto.decorate_type;
            }
            if dto.page_type.is_some() {
                d.page_type = dto.page_type.unwrap_or(d.page_type);
            }
            if dto.is_home.is_some() {
                d.is_home = dto.is_home;
            }
            if dto.shop_id.is_some() {
                // 多租户：请求值必须与身份一致（店铺账号跨店 → 403）
                d.shop_id = media_write_shop_id(dto.shop_id)?;
            }
            if dto.status.is_some() {
                d.status = dto.status;
            }
            if dto.parent_id.is_some() {
                d.parent_id = dto.parent_id;
            }
            if dto.locale_id.is_some() {
                d.locale_id = dto.locale_id;
            }
            if dto.sort_order.is_some() {
                d.sort_order = dto.sort_order;
            }
            if dto.binded_h5_id.is_some() {
                d.binded_h5_id = dto.binded_h5_id;
            }
            d.update_time = Some(now_seconds() as i32);
            Self::mark_mobile_login_as_home(&mut d);
            return self.repos.content.decorate.update(&d).await.map(|_| true);
        }

        // 新建多语言子页：标题/类型/店铺继承父页
        let parent_id = dto.parent_id.unwrap_or(0);
        let parent = self
            .repos.content.decorate
            .find_by_id(parent_id)
            .await?
            .ok_or_else(|| Error::NotFound("父级装修页面不存在".into()))?;
        // 多租户：父页需属于本店（平台页对店铺账号只读）
        ensure_media_write(parent.shop_id)?;
        let d = Decorate {
            parent_id: Some(parent_id),
            decorate_title: parent.decorate_title.clone(),
            decorate_type: parent.decorate_type,
            is_home: Some(0),
            shop_id: parent.shop_id,
            locale_id: dto.locale_id.or(Some(0)),
            data: dto.data.clone(),
            update_time: Some(now_seconds() as i32),
            page_type: parent.page_type,
            ..Decorate::default()
        };
        self.repos.content.decorate.insert(&d).await.map(|_| true)
    }

    /// 自定义页快速编辑（仅标题与排序），对应 `DecorateController#updateCustomDecorate`。
    pub async fn update_custom(&self, dto: &DecorateUpdateDTO) -> Result<bool> {
        let id = dto
            .decorate_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("装修页面ID不能为空".into()))?;
        let Some(mut d) = self.repos.content.decorate.find_by_id(id).await? else {
            return Err(Error::NotFound("装修页面不存在".into()));
        };
        // 多租户：仅本店装修页可改
        ensure_media_write(d.shop_id)?;
        d.decorate_title = dto.decorate_title.clone();
        d.sort_order = dto.sort_order;
        d.update_time = Some(now_seconds() as i32);
        self.repos.content.decorate.update(&d).await.map(|_| true)
    }

    /// 删除装修页，对应 `DecorateServiceImpl#del`（多语言子页保留）。
    pub async fn del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("装修页面ID不能为空".into()))?;
        // 多租户：删除前校验归属
        if let Some(row) = self.repos.content.decorate.find_by_id(id).await? {
            ensure_media_write(row.shop_id)?;
        }
        self.repos.content.decorate.delete(id).await
    }

    /// 批量操作，对应 `BaseServiceImpl#batch`（当前支持 `type = "del"` 批量删除）。
    ///
    /// 【原版有·我方改口径】默认页批删护栏（用户 2026-11-19 拍板）：原版
    /// `DecorateServiceImpl#batch` 无任何 isHome 护栏（仅 `super.batch`），批删
    /// 勾中默认页可直接删掉；我方在 del 分支新增护栏——ids 中任一页面为默认页
    /// （`is_home = 1`）即整批拒绝、不做部分删除（与下方归属校验「任一越权 →
    /// 整批拒绝」的既有口径一致）。单删 `del` 不加此护栏（与原版一致：其默认页
    /// 保护仅靠前端不显示单删入口）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => {
                // 多租户：先整体校验归属（任一越权 → 403，不做部分删除）；
                // 同一趟顺带查默认页护栏（任一命中 → 整批拒绝，不做部分删除）
                for id in &ids {
                    if let Some(row) = self.repos.content.decorate.find_by_id(*id).await? {
                        ensure_media_write(row.shop_id)?;
                        if is_home_page(&row) {
                            return Err(Error::Validation(format!(
                                "默认页不可删除（页面ID {id}），已取消本次批量删除"
                            )));
                        }
                    }
                }
                Ok(self.repos.content.decorate.delete_batch(&ids).await? > 0)
            }
            _ => Err(Error::Validation("不支持的批量操作类型".into())),
        }
    }

    /// 通用字段更新，对应 `BaseServiceImpl#updateField` + `DECORATE_FIELDS` 白名单。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("装修页面ID不能为空".into()))?;
        // 多租户：仅本店装修页可改
        if let Some(row) = self.repos.content.decorate.find_by_id(id).await? {
            ensure_media_write(row.shop_id)?;
        }
        let field = dto.field.clone().unwrap_or_default();
        let val = dto.val.clone().unwrap_or_default();
        self.repos.content.decorate.update_field(id, &field, &val).await
    }

    /// 复制装修页，对应 `DecorateServiceImpl#copy`。
    ///
    /// 标题生成"原名-副本"～"原名-副本N"，同店铺 + 同类型 + 同页面用途范围内取最大序号 + 1。
    pub async fn copy(&self, dto: &OperateDTO) -> Result<bool> {
        let Some(id) = dto.id else {
            return Ok(false);
        };
        let Some(source) = self.repos.content.decorate.find_by_id(id).await? else {
            return Ok(false);
        };
        // 多租户：允许复制可读页面；新页面落本店（平台账号保留来源店铺）
        ensure_media_access(source.shop_id)?;
        let title = self.generate_copy_title(&source).await?;
        let d = Decorate {
            decorate_id: 0,
            decorate_title: Some(title),
            is_home: Some(0),
            shop_id: media_write_shop_id(Some(source.shop_id))?,
            update_time: Some(now_seconds() as i32),
            ..source.clone()
        };
        self.repos.content.decorate.insert(&d).await.map(|_| true)
    }

    /// 设为首页，对应 `DecorateServiceImpl#setHome`：先取消同类型同页面用途的首页标记。
    pub async fn set_home(&self, dto: &OperateDTO, page_type: Option<i32>) -> Result<bool> {
        let Some(id) = dto.id else {
            return Ok(false);
        };
        let Some(mut d) = self.repos.content.decorate.find_by_id(id).await? else {
            return Ok(false);
        };
        // 多租户：仅本店装修页可设首页
        ensure_media_write(d.shop_id)?;
        self.repos.content.decorate
            .clear_home_flags(d.decorate_type, page_type, Some(d.shop_id), id)
            .await?;
        d.is_home = Some(1);
        if page_type.is_some() {
            d.page_type = page_type.unwrap_or(d.page_type);
        }
        self.repos.content.decorate.update(&d).await.map(|_| true)
    }

    /// 保存草稿，对应 `DecorateServiceImpl#saveDecorateToDraft`。
    pub async fn save_draft(
        &self,
        dto: &DecorateSaveDTO,
    ) -> Result<bool> {
        let data = serde_json::to_string(&dto.data).map_err(|e| Error::Validation(e.to_string()))?;
        if let Some(id) = dto.id.filter(|v| *v > 0) {
            let Some(mut d) = self.repos.content.decorate.find_by_id(id).await? else {
                return Err(Error::NotFound("装修页面不存在".into()));
            };
            // 多租户：仅本店装修页可保存草稿
            ensure_media_write(d.shop_id)?;
            d.draft_data = Some(data);
            return self.repos.content.decorate.update(&d).await.map(|_| true);
        }
        self.upsert_child(dto.parent_id, dto.locale_id, |d| {
            d.draft_data = Some(data.clone());
        })
        .await
    }

    /// 发布装修，对应 `DecorateServiceImpl#publishDecorate`：
    /// 写入 `data`、置 `status = 1`、清空草稿。
    pub async fn publish(&self, dto: &DecorateSaveDTO) -> Result<bool> {
        let data = serde_json::to_string(&dto.data).map_err(|e| Error::Validation(e.to_string()))?;
        if let Some(id) = dto.id.filter(|v| *v > 0) {
            let Some(mut d) = self.repos.content.decorate.find_by_id(id).await? else {
                return Err(Error::NotFound("装修页面不存在".into()));
            };
            // 多租户：仅本店装修页可发布
            ensure_media_write(d.shop_id)?;
            d.data = Some(data);
            d.status = Some(1);
            d.update_time = Some(now_seconds() as i32);
            d.draft_data = Some(String::new());
            return self.repos.content.decorate.update(&d).await.map(|_| true);
        }
        self.upsert_child(dto.parent_id, dto.locale_id, |d| {
            d.data = Some(data.clone());
        })
        .await
    }

    /// 加载草稿数据，对应 `DecorateServiceImpl#loadDraftData`。
    pub async fn load_draft_data(&self, id: i32) -> Result<serde_json::Value> {
        let vo = self.detail(Some(id), None, None, None).await?;
        if vo.decorate_id == 0 {
            return Err(Error::biz_msg("装修页面不存在"));
        }
        Ok(vo.draft_data.unwrap_or(serde_json::Value::Null))
    }

    /// 绑定 H5/自定义页，对应 `bindingH5Decorate` / `bindingCustomDecorate`。
    pub async fn binding(&self, dto: &DecorateUpdateDTO) -> Result<bool> {
        let id = dto
            .decorate_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("装修页面ID不能为空".into()))?;
        let binded_h5_id = dto
            .binded_h5_id
            .ok_or_else(|| Error::Validation("请选择需要绑定的页面".into()))?;
        let Some(mut d) = self.repos.content.decorate.find_by_id(id).await? else {
            return Err(Error::NotFound("装修页面不存在".into()));
        };
        // 多租户：仅本店装修页可绑定
        ensure_media_write(d.shop_id)?;
        d.binded_h5_id = Some(binded_h5_id);
        self.repos.content.decorate.update(&d).await.map(|_| true)
    }

    /// 解除绑定，对应 `cancelBinding`。
    pub async fn cancel_binding(&self, dto: &DecorateUpdateDTO) -> Result<bool> {
        let id = dto
            .decorate_id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Validation("装修页面ID不能为空".into()))?;
        let Some(mut d) = self.repos.content.decorate.find_by_id(id).await? else {
            return Err(Error::NotFound("装修页面不存在".into()));
        };
        // 多租户：仅本店装修页可取消绑定
        ensure_media_write(d.shop_id)?;
        d.binded_h5_id = Some(0);
        self.repos.content.decorate.update(&d).await.map(|_| true)
    }

    // ————————————————————— 装修推广分享信息 —————————————————————

    /// 保存装修页分享信息，对应 `DecorateServiceImpl#saveShareInfo`。
    pub async fn save_share_info(&self, dto: &DecorateShareInfoDTO) -> Result<bool> {
        let Some(mut d) = self.repos.content.decorate.find_by_id(dto.decorate_id).await? else {
            return Err(Error::biz_msg("装修页面不存在"));
        };
        let json = serde_json::json!({
            "shareTitle": dto.share_title,
            "shareDesc": dto.share_desc,
            "shareImage": dto.share_image,
        });
        d.share_info = Some(json.to_string());
        self.repos.content.decorate.update(&d).await.map(|_| true)
    }

    /// 读取装修页分享信息，对应 `DecorateServiceImpl#getShareInfo`。
    pub async fn share_info(&self, decorate_id: i32) -> Result<DecorateShareInfoVO> {
        let Some(d) = self.repos.content.decorate.find_by_id(decorate_id).await? else {
            return Err(Error::biz_msg("装修页面不存在"));
        };
        let mut vo = DecorateShareInfoVO {
            decorate_id,
            ..DecorateShareInfoVO::default()
        };
        if let Some(info) = parse_json_opt(&d.share_info) {
            vo.share_title = info
                .get("shareTitle")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            vo.share_desc = info
                .get("shareDesc")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            vo.share_image = info
                .get("shareImage")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
        }
        Ok(vo)
    }

    // ————————————————————— PC 导航 —————————————————————

    /// PC 导航分页，对应 `PcNavigationServiceImpl#list`。
    pub async fn pc_navigation_list(
        &self,
        q: &PcNavigationListQuery,
    ) -> Result<PageResult<PcNavigationVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos.content.decorate
            .paginate_navigations(
                q.r#type,
                q.parent_id,
                q.keyword.as_deref(),
                &page,
            )
            .await?;
        let mut vos: Vec<PcNavigationVO> = rows.iter().map(PcNavigationVO::from).collect();
        // 批量回填子级数量
        let parent_ids: Vec<i32> = vos.iter().map(|v| v.id).collect();
        let counts = self
            .repos.content.decorate
            .count_navigation_children(&parent_ids)
            .await?;
        for v in &mut vos {
            v.has_children = counts
                .iter()
                .find(|(pid, _)| *pid == v.id)
                .map(|(_, c)| *c)
                .unwrap_or(0);
        }
        Ok(PageResult::new(total, vos, &page))
    }

    /// PC 导航详情。
    pub async fn pc_navigation_detail(&self, id: i32) -> Result<PcNavigationVO> {
        let n = self
            .repos.content.decorate
            .find_navigation(id)
            .await?
            .ok_or_else(|| Error::NotFound("导航数据不存在".into()))?;
        let mut vo = PcNavigationVO::from(&n);
        let counts = self
            .repos.content.decorate
            .count_navigation_children(&[n.id])
            .await?;
        vo.has_children = counts.first().map(|(_, c)| *c).unwrap_or(0);
        Ok(vo)
    }

    /// 新建 PC 导航。
    pub async fn pc_navigation_create(&self, dto: &PcNavigationCreateDTO) -> Result<i32> {
        let title = dto
            .title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| Error::Validation("导航标题不能为空".into()))?;
        let n = nextrade_core::entities::PcNavigation {
            id: 0,
            title: title.to_string(),
            is_show: dto.is_show.unwrap_or(1),
            sort_order: dto.sort_order.unwrap_or(0),
            is_blank: dto.is_blank.unwrap_or(0),
            link: dto
                .link
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| Error::Validation(e.to_string()))?
                .unwrap_or_else(|| "{}".into()),
            r#type: dto.r#type.unwrap_or(1),
            parent_id: dto.parent_id.unwrap_or(0),
            icon: dto.icon.clone().unwrap_or_default(),
        };
        self.repos.content.decorate.insert_navigation(&n).await
    }

    /// 更新 PC 导航（只覆盖非空字段）。
    pub async fn pc_navigation_update(&self, dto: &PcNavigationUpdateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("导航ID不能为空".into()))?;
        let Some(mut n) = self.repos.content.decorate.find_navigation(id).await? else {
            return Err(Error::NotFound("导航数据不存在".into()));
        };
        if dto.title.is_some() {
            n.title = dto.title.clone().unwrap_or(n.title);
        }
        if dto.is_show.is_some() {
            n.is_show = dto.is_show.unwrap_or(n.is_show);
        }
        if dto.sort_order.is_some() {
            n.sort_order = dto.sort_order.unwrap_or(n.sort_order);
        }
        if dto.is_blank.is_some() {
            n.is_blank = dto.is_blank.unwrap_or(n.is_blank);
        }
        if dto.link.is_some() {
            n.link = dto
                .link
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| Error::Validation(e.to_string()))?
                .unwrap_or_else(|| "{}".into());
        }
        if dto.r#type.is_some() {
            n.r#type = dto.r#type.unwrap_or(n.r#type);
        }
        if dto.parent_id.is_some() {
            n.parent_id = dto.parent_id.unwrap_or(n.parent_id);
        }
        if dto.icon.is_some() {
            n.icon = dto.icon.clone().unwrap_or(n.icon);
        }
        self.repos.content.decorate.update_navigation(&n).await.map(|_| true)
    }

    /// 删除 PC 导航。
    pub async fn pc_navigation_del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("导航ID不能为空".into()))?;
        self.repos.content.decorate.delete_navigation(id).await
    }

    /// PC 导航通用字段更新（白名单：title/is_show/is_blank/sort_order）。
    pub async fn pc_navigation_update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("导航ID不能为空".into()))?;
        let field = dto.field.clone().unwrap_or_default();
        let val = dto.val.clone().unwrap_or_default();
        self.repos.content.decorate
            .update_navigation_field(id, &field, &val)
            .await
    }

    /// PC 导航批量删除。
    pub async fn pc_navigation_batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => {
                let mut removed = false;
                for id in ids {
                    removed |= self.repos.content.decorate.delete_navigation(id).await?;
                }
                Ok(removed)
            }
            _ => Err(Error::Validation("不支持的批量操作类型".into())),
        }
    }

    /// 导航可选链接列表，对应 `PcNavigationController#selectLink`。
    ///
    /// 前端 `LinkBase` 读取 `name` / `link`，必须返回该结构；不能返回
    /// `label` / `value`，否则“其他页面”标签会渲染为空。
    pub fn pc_navigation_select_link(&self) -> Vec<serde_json::Value> {
        crate::content::pc_navigation::select_link_entries(false)
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "app_link": e.app_link,
                    "link": e.link,
                })
            })
            .collect()
    }

    /// 上级导航树，对应 `PcNavigationController#getParentNav(type, 0)`。
    pub async fn pc_navigation_parent_nav(&self, nav_type: i32) -> Result<Vec<PcNavTreeVO>> {
        let page = PageQuery {
            page: 1,
            page_size: 500,
        };
        let (_, rows) = self
            .repos.content.decorate
            .paginate_navigations(Some(nav_type), None, None, &page)
            .await?;
        // 只保留指定类型的导航，按 parent_id 组两层树
        let mut roots: Vec<PcNavTreeVO> = Vec::new();
        for row in &rows {
            if row.parent_id == 0 {
                roots.push(PcNavTreeVO {
                    id: row.id,
                    parent_id: row.parent_id,
                    title: row.title.clone(),
                    children: Vec::new(),
                });
            }
        }
        for row in &rows {
            if row.parent_id != 0 {
                if let Some(parent) = roots.iter_mut().find(|r| r.id == row.parent_id) {
                    parent.children.push(PcNavTreeVO {
                        id: row.id,
                        parent_id: row.parent_id,
                        title: row.title.clone(),
                        children: Vec::new(),
                    });
                }
            }
        }
        Ok(roots)
    }

    // ————————————————————— PC 分类楼层 —————————————————————

    /// PC 分类楼层分页，对应 `PcCatFloorServiceImpl#list`。
    pub async fn pc_cat_floor_list(
        &self,
        q: &PcCatFloorListQuery,
    ) -> Result<PageResult<PcCatFloorVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos.content.decorate
            .paginate_cat_floors(q.is_show, q.keyword.as_deref(), &page)
            .await?;
        let list: Vec<PcCatFloorVO> = rows.iter().map(PcCatFloorVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// PC 分类楼层详情。
    pub async fn pc_cat_floor_detail(&self, id: i32) -> Result<PcCatFloorVO> {
        let f = self
            .repos.content.decorate
            .find_cat_floor(id)
            .await?
            .ok_or_else(|| Error::NotFound("分类楼层不存在".into()))?;
        Ok(PcCatFloorVO::from(&f))
    }

    /// 新建 PC 分类楼层：展示名由分类名以"，"拼接（`PcCatFloor#getCatFloorName`）。
    pub async fn pc_cat_floor_create(&self, dto: &PcCatFloorCreateDTO) -> Result<i32> {
        let ids = dto.category_ids.clone().unwrap_or_default();
        let names = self.resolve_cat_floor_names(&ids, dto.category_names.clone()).await?;
        let f = nextrade_core::entities::PcCatFloor {
            cat_floor_id: 0,
            cat_floor_name: names.clone(),
            category_ids: serde_json::to_string(&ids)
                .map_err(|e| Error::Validation(e.to_string()))?,
            category_names: serde_json::to_string(
                &names
                    .split('，')
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| Error::Validation(e.to_string()))?,
            floor_ico: dto.floor_ico.clone().unwrap_or_default(),
            hot_cat: dto.hot_cat.clone().unwrap_or_default(),
            is_show: dto.is_show.unwrap_or(1),
            sort_order: dto.sort_order.unwrap_or(0),
            floor_ico_font: dto.floor_ico_font.clone().unwrap_or_default(),
            brand_ids: serde_json::to_string(&dto.brand_ids.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?,
        };
        self.repos.content.decorate.insert_cat_floor(&f).await
    }

    /// 更新 PC 分类楼层（展示名同样重算）。
    pub async fn pc_cat_floor_update(&self, dto: &PcCatFloorUpdateDTO) -> Result<bool> {
        let id = dto
            .cat_floor_id
            .ok_or_else(|| Error::Validation("楼层ID不能为空".into()))?;
        let Some(mut f) = self.repos.content.decorate.find_cat_floor(id).await? else {
            return Err(Error::NotFound("分类楼层不存在".into()));
        };
        if dto.category_ids.is_some() || dto.category_names.is_some() {
            let ids = dto.category_ids.clone().unwrap_or_default();
            let names = self
                .resolve_cat_floor_names(&ids, dto.category_names.clone())
                .await?;
            f.cat_floor_name = names.clone();
            f.category_ids =
                serde_json::to_string(&ids).map_err(|e| Error::Validation(e.to_string()))?;
            f.category_names = serde_json::to_string(
                &names
                    .split('，')
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| Error::Validation(e.to_string()))?;
        }
        if dto.floor_ico.is_some() {
            f.floor_ico = dto.floor_ico.clone().unwrap_or(f.floor_ico);
        }
        if dto.hot_cat.is_some() {
            f.hot_cat = dto.hot_cat.clone().unwrap_or(f.hot_cat);
        }
        if dto.is_show.is_some() {
            f.is_show = dto.is_show.unwrap_or(f.is_show);
        }
        if dto.sort_order.is_some() {
            f.sort_order = dto.sort_order.unwrap_or(f.sort_order);
        }
        if dto.floor_ico_font.is_some() {
            f.floor_ico_font = dto
                .floor_ico_font
                .clone()
                .unwrap_or(f.floor_ico_font);
        }
        if dto.brand_ids.is_some() {
            f.brand_ids = serde_json::to_string(&dto.brand_ids.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?;
        }
        self.repos.content.decorate.update_cat_floor(&f).await.map(|_| true)
    }

    /// 删除 PC 分类楼层。
    pub async fn pc_cat_floor_del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("楼层ID不能为空".into()))?;
        self.repos.content.decorate.delete_cat_floor(id).await
    }

    /// PC 分类楼层通用字段更新。
    pub async fn pc_cat_floor_update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("楼层ID不能为空".into()))?;
        let field = dto.field.clone().unwrap_or_default();
        let val = dto.val.clone().unwrap_or_default();
        // 白名单与 Java `PcCatFloorController` 一致：仅显示状态与排序
        match field.replace('-', "_").as_str() {
            "is_show" | "isShow" | "sort_order" | "sortOrder" => {}
            _ => return Err(Error::Validation(format!("不允许更新的字段: {field}"))),
        }
        let column = if field.eq_ignore_ascii_case("isShow") || field == "is_show" {
            "is_show"
        } else {
            "sort_order"
        };
        let sql = format!("UPDATE pc_cat_floor SET \"{column}\" = $1 WHERE cat_floor_id = $2");
        self.repos.content.decorate.update_field_raw(&sql, &val, id).await
    }

    /// PC 分类楼层批量删除。
    pub async fn pc_cat_floor_batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => {
                let mut removed = false;
                for id in ids {
                    removed |= self.repos.content.decorate.delete_cat_floor(id).await?;
                }
                Ok(removed)
            }
            _ => Err(Error::Validation("不支持的批量操作类型".into())),
        }
    }

    /// 清理楼层缓存（Rust 侧无装修楼层缓存，保持接口兼容的空实现）。
    pub fn pc_cat_floor_clear_cache(&self) {}

    // ————————————————————— 手机分类导航 —————————————————————

    /// 手机分类导航分页，对应 `MobileCatNavServiceImpl#list`。
    pub async fn mobile_cat_nav_list(
        &self,
        q: &MobileCatNavListQuery,
    ) -> Result<PageResult<MobileCatNavVO>> {
        let page = q.page_query();
        let (total, rows) = self
            .repos.content.decorate
            .paginate_mobile_navs(q.is_show, q.keyword.as_deref(), &page)
            .await?;
        let list: Vec<MobileCatNavVO> = rows.iter().map(MobileCatNavVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 手机分类导航详情。
    pub async fn mobile_cat_nav_detail(&self, id: i32) -> Result<MobileCatNavVO> {
        let n = self
            .repos.content.decorate
            .find_mobile_nav(id)
            .await?
            .ok_or_else(|| Error::NotFound("分类导航不存在".into()))?;
        Ok(MobileCatNavVO::from(&n))
    }

    /// 新建手机分类导航。
    pub async fn mobile_cat_nav_create(&self, dto: &MobileCatNavCreateDTO) -> Result<i32> {
        let category_id = dto
            .category_id
            .ok_or_else(|| Error::Validation("请选择分类".into()))?;
        let n = MobileCatNav {
            category_id,
            cat_color: dto.cat_color.clone().unwrap_or_default(),
            child_cat_ids: serde_json::to_string(&dto.child_cat_ids.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?,
            brand_ids: serde_json::to_string(&dto.brand_ids.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?,
            is_show: dto.is_show.unwrap_or(1),
            img_url: serde_json::to_string(&dto.img_url.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?,
            sort_order: dto.sort_order.unwrap_or(0),
            cat_name_alias: dto.cat_name_alias.clone(),
            ..MobileCatNav::default()
        };
        self.repos.content.decorate.insert_mobile_nav(&n).await
    }

    /// 更新手机分类导航（只覆盖非空字段）。
    pub async fn mobile_cat_nav_update(&self, dto: &MobileCatNavUpdateDTO) -> Result<bool> {
        let id = dto
            .mobile_cat_nav_id
            .ok_or_else(|| Error::Validation("导航ID不能为空".into()))?;
        let Some(mut n) = self.repos.content.decorate.find_mobile_nav(id).await? else {
            return Err(Error::NotFound("分类导航不存在".into()));
        };
        if dto.category_id.is_some() {
            n.category_id = dto.category_id.unwrap_or(n.category_id);
        }
        if dto.cat_color.is_some() {
            n.cat_color = dto.cat_color.clone().unwrap_or(n.cat_color);
        }
        if dto.child_cat_ids.is_some() {
            n.child_cat_ids =
                serde_json::to_string(&dto.child_cat_ids.clone().unwrap_or_default())
                    .map_err(|e| Error::Validation(e.to_string()))?;
        }
        if dto.brand_ids.is_some() {
            n.brand_ids = serde_json::to_string(&dto.brand_ids.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?;
        }
        if dto.is_show.is_some() {
            n.is_show = dto.is_show.unwrap_or(n.is_show);
        }
        if dto.img_url.is_some() {
            n.img_url = serde_json::to_string(&dto.img_url.clone().unwrap_or_default())
                .map_err(|e| Error::Validation(e.to_string()))?;
        }
        if dto.sort_order.is_some() {
            n.sort_order = dto.sort_order.unwrap_or(n.sort_order);
        }
        if dto.cat_name_alias.is_some() {
            n.cat_name_alias = dto.cat_name_alias.clone();
        }
        self.repos.content.decorate.update_mobile_nav(&n).await.map(|_| true)
    }

    /// 删除手机分类导航。
    pub async fn mobile_cat_nav_del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("导航ID不能为空".into()))?;
        self.repos.content.decorate.delete_mobile_nav(id).await
    }

    /// 手机分类导航通用字段更新。
    pub async fn mobile_cat_nav_update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("导航ID不能为空".into()))?;
        let field = dto.field.clone().unwrap_or_default();
        let val = dto.val.clone().unwrap_or_default();
        let column = match field.as_str() {
            "is_show" | "isShow" => "is_show",
            "sort_order" | "sortOrder" => "sort_order",
            "cat_color" | "catColor" => "cat_color",
            _ => return Err(Error::Validation(format!("不允许更新的字段: {field}"))),
        };
        let sql = format!(
            "UPDATE mobile_cat_nav SET \"{column}\" = $1 WHERE mobile_cat_nav_id = $2"
        );
        self.repos.content.decorate.update_field_raw(&sql, &val, id).await
    }

    /// 手机分类导航批量删除。
    pub async fn mobile_cat_nav_batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => {
                let mut removed = false;
                for id in ids {
                    removed |= self.repos.content.decorate.delete_mobile_nav(id).await?;
                }
                Ok(removed)
            }
            _ => Err(Error::Validation("不支持的批量操作类型".into())),
        }
    }

    // ————————————————————— 装修分享（管理端） —————————————————————

    /// 装修分享分页，对应 `DecorateShareServiceImpl#list`。
    pub async fn decorate_share_list(
        &self,
        q: &DecorateShareListQuery,
    ) -> Result<PageResult<DecorateShareVO>> {
        let page = q.page_query();
        let (total, rows) = self.repos.content.decorate.paginate_shares(&page).await?;
        let list: Vec<DecorateShareVO> = rows.iter().map(DecorateShareVO::from).collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 装修分享详情。
    pub async fn decorate_share_detail(&self, id: i32) -> Result<DecorateShareVO> {
        let s = self
            .repos.content.decorate
            .find_share(id)
            .await?
            .ok_or_else(|| Error::NotFound("分享记录不存在".into()))?;
        Ok(DecorateShareVO::from(&s))
    }

    /// 新建分享记录（sn/token 自动生成，有效期 7 天）。
    pub async fn decorate_share_create(&self, decorate_id: i32) -> Result<DecorateShareVO> {
        self.generate_share(decorate_id).await
    }

    /// 更新分享记录：重新签发 sn/token 与有效期。
    pub async fn decorate_share_update(&self, id: i32, decorate_id: i32) -> Result<bool> {
        let Some(mut s) = self.repos.content.decorate.find_share(id).await? else {
            return Err(Error::NotFound("分享记录不存在".into()));
        };
        s.share_sn = random_hex(SHARE_CODE_LENGTH);
        s.share_token = random_hex(SHARE_TOKEN_LENGTH);
        s.decorate_id = decorate_id;
        s.valid_time = now_seconds() as i32 + SHARE_EXPIRE_SECONDS;
        s.update_time = now_seconds() as i32;
        self.repos.content.decorate.update_share(&s).await.map(|_| true)
    }

    /// 删除分享记录。
    pub async fn decorate_share_del(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("分享记录ID不能为空".into()))?;
        self.repos.content.decorate.delete_share(id).await
    }

    /// 分享记录通用字段更新（仅允许 token 有效期）。
    pub async fn decorate_share_update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation("分享记录ID不能为空".into()))?;
        let field = dto.field.clone().unwrap_or_default();
        let val = dto.val.clone().unwrap_or_default();
        if !matches!(field.as_str(), "valid_time" | "validTime") {
            return Err(Error::Validation(format!("不允许更新的字段: {field}")));
        }
        let Some(mut s) = self.repos.content.decorate.find_share(id).await? else {
            return Err(Error::NotFound("分享记录不存在".into()));
        };
        s.valid_time = val.parse().map_err(|_| Error::Validation("有效期格式错误".into()))?;
        s.update_time = now_seconds() as i32;
        self.repos.content.decorate.update_share(&s).await.map(|_| true)
    }

    /// 分享记录批量删除。
    pub async fn decorate_share_batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.clone().unwrap_or_default();
        match dto.r#type.as_deref() {
            Some("del") => {
                let mut removed = false;
                for id in ids {
                    removed |= self.repos.content.decorate.delete_share(id).await?;
                }
                Ok(removed)
            }
            _ => Err(Error::Validation("不支持的批量操作类型".into())),
        }
    }

    /// 生成装修页分享，对应 `DecorateShareServiceImpl#share`：
    /// sn(6位) + token(5位)，有效期 7 天，返回含导入链接的 VO。
    pub async fn share(&self, decorate_id: i32) -> Result<DecorateShareVO> {
        self.repos.content.decorate
            .find_by_id(decorate_id)
            .await?
            .ok_or_else(|| Error::biz_msg("装修页面不存在"))?;
        self.generate_share(decorate_id).await
    }

    /// 分享导入（管理端）：校验 url 中携带的 sn/token 是否有效。
    ///
    /// TODO(域实现者)：Java `importUrl` 会抓取远程装修接口并落库（`HttpUtil.get`），
    /// 本服务无 HTTP 客户端，这里实现为校验分享有效性，导入落库待后续补充。
    pub async fn decorate_share_import(&self, url: &str) -> Result<bool> {
        let (sn, token) = Self::parse_share_url(url)
            .ok_or_else(|| Error::biz_msg("分享链接缺少 sn 或 token"))?;
        self.repos.content.decorate
            .find_valid_share(&sn, &token, now_seconds() as i32)
            .await?
            .ok_or_else(|| Error::biz_msg("分享链接不存在或已失效!"))?;
        Ok(true)
    }

    // ————————————————————— 装修组件（管理端） —————————————————————

    /// 装修组件详情，对应 `DecorateDiscreteServiceImpl#loadDetail`：
    /// `openAdvertising` 查不到返回 `null`，其余返回空 VO。
    ///
    /// TODO(域实现者)：Java 侧对招商类组件做多语言翻译（`translatePackage`），
    /// 本域不引入翻译链路，原样返回 `data`。
    pub async fn discrete_detail(&self, decorate_sn: &str) -> Result<Option<DecorateDiscreteVO>> {
        let d = self
            .repos.content.decorate
            .find_discrete(DEFAULT_SHOP_ID, decorate_sn)
            .await?;
        match d {
            None if decorate_sn == "openAdvertising" => Ok(None),
            None => Ok(Some(DecorateDiscreteVO::default())),
            Some(d) => Ok(Some(DecorateDiscreteVO::from(&d))),
        }
    }

    /// 装修组件更新（upsert），对应 `DecorateDiscreteServiceImpl#update`。
    pub async fn discrete_update(&self, param: &DecorateDiscreteUpdateParam) -> Result<bool> {
        let shop_id = param.shop_id.unwrap_or(DEFAULT_SHOP_ID);
        let data = serde_json::to_string(&param.data)
            .map_err(|e| Error::Validation(e.to_string()))?;
        match self
            .repos.content.decorate
            .find_discrete(shop_id, &param.decorate_sn)
            .await?
        {
            Some(mut d) => {
                d.decorate_name = param.decorate_name.clone();
                d.data = data;
                self.repos.content.decorate.update_discrete(&d).await.map(|_| true)
            }
            None => {
                let d = DecorateDiscrete {
                    id: 0,
                    decorate_sn: param.decorate_sn.clone(),
                    decorate_name: param.decorate_name.clone(),
                    data,
                    shop_id,
                };
                self.repos.content.decorate.insert_discrete(&d).await.map(|_| true)
            }
        }
    }

    /// 个人中心基础数据，对应 `DecorateDiscreteServiceImpl#getMemberDecorateData`。
    ///
    /// TODO(域实现者)：Java 按 O2O/B2B/海外/商户开关追加"商家入驻""实名认证"项，
    /// 并按 `CLOSE_AUTH` 配置过滤；本域无店铺配置上下文，先返回稳定菜单集合。
    pub fn member_decorate_data(&self) -> DecorateMemberDataVO {
        let menu = |title: &str, path: &str, thumb: &str| DecorateDiscreteMenuItemDTO {
            is_show: Some(1),
            r#type: Some("default".into()),
            pic_title: Some(title.to_string()),
            pic_thumb: Some(thumb.to_string()),
            pic_url: Some(thumb.to_string()),
            pic_link: Some(nextrade_core::models::content::decorate::DecorateDiscretePicLink {
                path: Some("default".into()),
                label: Some(title.to_string()),
                name: Some(title.to_string()),
                link: Some(path.to_string()),
            }),
        };
        let oss = "https://oss.nextrade.com/static/user";
        DecorateMemberDataVO {
            menus: vec![
                menu("账号管理", "/pages/user/profile/index", &format!("{oss}/zhanghaoguanli.png")),
                menu("收货地址", "/pages/address/list", &format!("{oss}/shouhuodizhi.png")),
                menu("消息中心", "/pages/im/page", &format!("{oss}/xiaoxi.png")),
                menu("帮助中心", "/pages/article/issue/list", &format!("{oss}/issue.png")),
                menu(
                    "分销员中心",
                    "/pages/salesman/index",
                    &format!("{oss}/salesmanIco.png"),
                ),
                menu("积分商城", "/pages/exchange/list", &format!("{oss}/jifen.png")),
                menu(
                    "发票管理",
                    "/pages/user/invoiceManagement/index",
                    &format!("{oss}/fapiao.png"),
                ),
                menu("我的拼团", "/pages/groupon/mylist", &format!("{oss}/fapiao.png")),
            ],
        }
    }

    // ————————————————————— 前台装修读取 —————————————————————

    /// H5 商城首页装修，对应 `HomeController#index`。
    ///
    /// `locale_code` 来自请求头 `X-Locale-Code`：非默认语言时解析语言子页
    /// （原版 `DecorateServiceImpl#loadDecorateModule` 的 parentId/localeId 逻辑）；
    /// 预览路径按确切记录取数，不做语言解析（原版同）。
    pub async fn home_index(
        &self,
        q: &HomeDecorateQuery,
        locale_code: Option<&str>,
    ) -> Result<DecorateModuleListVO> {
        let mut vo = if let Some(preview_id) = q.preview_id.filter(|v| *v > 0) {
            self.preview_decorate(Some(1), preview_id, None, 1).await?
        } else {
            self.decorate_module(Some(1), Some(1), true, Some(1), locale_code)
                .await?
        };
        convert_legacy_h5_modules(&mut vo);
        Ok(vo)
    }

    /// PC 商城首页装修，对应 `HomeController#pcIndex`。
    /// `locale_code` 语义同 `home_index`。
    pub async fn pc_index(
        &self,
        q: &HomeDecorateQuery,
        locale_code: Option<&str>,
    ) -> Result<DecorateModuleListVO> {
        let preview = q.preview_id.filter(|v| *v > 0);
        let by_id = q.id.filter(|v| *v > 0);
        if preview.is_some() || by_id.is_some() {
            if q.decorate_type == Some(3) {
                return self.custom_decorate(locale_code).await;
            }
            let target = preview.or(by_id).unwrap_or(0);
            return self.preview_decorate(None, target, by_id, 1).await;
        }
        self.decorate_module(Some(2), Some(1), true, Some(1), locale_code)
            .await
    }

    /// 装修页分享导入，对应 `ShareController#import` + `getInfoBySn`：
    /// 校验 sn/token 后返回装修页详情 VO。
    pub async fn share_import(&self, sn: &str, token: &str) -> Result<DecorateVO> {
        if sn.is_empty() || token.is_empty() {
            return Err(Error::biz_msg("参数缺失"));
        }
        let share = self
            .repos.content.decorate
            .find_valid_share(sn, token, now_seconds() as i32)
            .await?
            .ok_or_else(|| Error::biz_msg("分享链接不存在或已失效!"))?;
        let vo = self
            .detail(Some(share.decorate_id), None, None, None)
            .await?;
        if vo.decorate_id == 0 {
            return Err(Error::biz_msg("未查询到对应的装修信息!"));
        }
        Ok(vo)
    }

    /// 开屏广告装修组件，对应 `DecorateDiscreteController#getOpenAdvertising`。
    pub async fn open_advertising(&self) -> Result<serde_json::Value> {
        let d = self
            .repos.content.decorate
            .find_discrete(DEFAULT_SHOP_ID, "openAdvertising")
            .await?;
        Ok(d.and_then(|d| parse_json_opt(&Some(d.data)))
            .unwrap_or_else(|| serde_json::json!({})))
    }

    // ————————————————————— 私有工具 —————————————————————

    /// 列表过滤条件组装。
    fn build_filter(q: &DecorateListQuery) -> DecorateFilter {
        DecorateFilter {
            // 多租户：店铺账号强制本店（含平台装修页）；平台账号不过滤
            shop_id: TenantScope::current_list_filter(),
            // 店铺精确筛选【我方新增】：透传管理页下拉选中值（含 0=平台），
            // 与租户可见口径在仓储层 AND，不放宽店铺账号的既有限制
            filter_shop_id: q.shop_id,
            // 排除店铺【我方新增，店铺页签口径】：透传 excludeShopId
            // （店铺页签传 0 = 只列商家店铺页），与租户口径、精确过滤
            // 在仓储层三者 AND 叠加，不放宽租户范围
            exclude_shop_id: q.exclude_shop_id,
            keyword: q.keyword.clone(),
            decorate_types: q.decorate_types(),
            page_types: q.page_types(),
            parent_id: q.parent_id,
            status: q.status,
            sort_field: q.sort_field.clone(),
            sort_order: q.sort_order.clone(),
        }
    }

    /// 店铺展示名【我方新增】：`shop_id = 0` 固定「平台」（shop 表平台行
    /// 标题为脏数据，不可直用）；其余取 `shop.shop_title`，查不到回退 None
    /// （前端显示店铺 ID）。
    fn shop_display_name(shop_id: i32, title: Option<String>) -> Option<String> {
        if shop_id == DEFAULT_SHOP_ID {
            return Some("平台".to_string());
        }
        title.filter(|s| !s.trim().is_empty())
    }

    /// 店铺筛选选项【我方新增】（移动装修管理页店铺下拉数据源）。
    ///
    /// 只含当前账号可见范围内真正有装修页的店铺（distinct 聚合），
    /// 平台账号见全部、店铺账号只见平台 + 本店（租户口径同列表）。
    pub async fn shop_options(
        &self,
        q: &DecorateListQuery,
    ) -> Result<Vec<DecorateShopOptionVO>> {
        let rows = self
            .repos
            .content
            .decorate
            .distinct_shops(TenantScope::current_list_filter(), &q.decorate_types())
            .await?;
        Ok(rows
            .into_iter()
            .map(|(shop_id, title, page_count)| DecorateShopOptionVO {
                shop_id,
                shop_name: Self::shop_display_name(shop_id, title),
                page_count,
            })
            .collect())
    }

    /// 实体批量转 VO（回填绑定的 H5 页面名）。
    async fn to_vos(&self, rows: Vec<Decorate>) -> Result<Vec<DecorateVO>> {
        let mut vos: Vec<DecorateVO> = rows.iter().map(DecorateVO::from).collect();
        // 店铺名回填【我方新增】：按本页 distinct shop_id 批量取 shop_title
        let mut shop_ids: Vec<i32> = vos.iter().filter_map(|v| v.shop_id).collect();
        shop_ids.sort_unstable();
        shop_ids.dedup();
        if !shop_ids.is_empty() {
            let titles = self
                .repos
                .content
                .decorate
                .find_shop_titles(&shop_ids)
                .await?;
            for v in &mut vos {
                if let Some(shop_id) = v.shop_id {
                    let title = titles
                        .iter()
                        .find(|(id, _)| *id == shop_id)
                        .and_then(|(_, t)| t.clone());
                    v.shop_name = Self::shop_display_name(shop_id, title);
                }
            }
        }
        let mut h5_ids: Vec<i32> = vos
            .iter()
            .filter_map(|v| v.binded_h5_id)
            .filter(|id| *id > 0)
            .collect();
        h5_ids.sort_unstable();
        h5_ids.dedup();
        if !h5_ids.is_empty() {
            for id in h5_ids {
                if let Some(h5) = self.repos.content.decorate.find_by_id(id).await? {
                    for v in &mut vos {
                        if v.binded_h5_id == Some(id) {
                            v.bingding_h5_name = h5.decorate_title.clone();
                        }
                    }
                }
            }
        }
        Ok(vos)
    }

    /// `saveDraft` / `publish` 的 `id` 为空分支：按父页定位（不存在则新建）多语言子页。
    async fn upsert_child<F>(&self, parent_id: Option<i32>, locale_id: Option<i32>, f: F) -> Result<bool>
    where
        F: FnOnce(&mut Decorate),
    {
        let parent_id = parent_id.unwrap_or(0);
        let parent = self
            .repos.content.decorate
            .find_by_id(parent_id)
            .await?
            .ok_or_else(|| Error::NotFound("父级装修页面不存在".into()))?;
        // 多租户：父页需属于本店（平台页对店铺账号只读）
        ensure_media_write(parent.shop_id)?;
        let locale_id = locale_id.unwrap_or(0);
        let decorate_type = parent.decorate_type;
        if let Some(mut child) = self
            .repos.content.decorate
            .find_by_parent_locale(parent_id, Some(locale_id), decorate_type)
            .await?
        {
            f(&mut child);
            return self.repos.content.decorate.update(&child).await.map(|_| true);
        }
        let mut child = Decorate {
            parent_id: Some(parent_id),
            decorate_title: parent.decorate_title.clone(),
            decorate_type: parent.decorate_type,
            is_home: Some(0),
            shop_id: parent.shop_id,
            locale_id: Some(locale_id),
            page_type: parent.page_type,
            ..Decorate::default()
        };
        f(&mut child);
        self.repos.content.decorate.insert(&child).await.map(|_| true)
    }

    /// 移动端登录页强制置首页（对应 `markMobileLoginAsHome`）：
    /// H5（type=1）+ 登录页（pageType=5）+ 平台店铺（shopId=0）时 `isHome = 1`。
    fn mark_mobile_login_as_home(d: &mut Decorate) {
        if d.decorate_type == Some(1) && d.page_type == 5 && d.shop_id == DEFAULT_SHOP_ID {
            d.is_home = Some(1);
        }
    }

    /// 复制页标题生成（"-副本" / "-副本N" 无限递增）。
    async fn generate_copy_title(&self, source: &Decorate) -> Result<String> {
        let raw = source.decorate_title.clone().unwrap_or_default();
        let mut base = raw.clone();
        // 去掉尾部已有的"-副本"/"-副本N"后缀，防止无限追加
        loop {
            let Some(idx) = base.rfind(COPY_SUFFIX) else {
                break;
            };
            let tail = &base[idx + COPY_SUFFIX.len()..];
            if tail.is_empty() || tail.chars().all(|c| c.is_ascii_digit()) {
                base.truncate(idx);
            } else {
                break;
            }
        }
        if base.trim().is_empty() {
            base = if raw.trim().is_empty() {
                "页面".into()
            } else {
                raw
            };
        }
        let prefix = format!("{base}{COPY_SUFFIX}");

        let page = PageQuery {
            page: 1,
            page_size: 500,
        };
        let filter = DecorateFilter {
            shop_id: Some(source.shop_id),
            filter_shop_id: None,
            exclude_shop_id: None,
            keyword: Some(prefix.clone()),
            decorate_types: source.decorate_type.into_iter().collect(),
            page_types: vec![source.page_type],
            parent_id: None,
            status: None,
            sort_field: None,
            sort_order: None,
        };
        let (_, rows) = self.repos.content.decorate.paginate(&filter, &page).await?;
        let mut max_seq = 0i64;
        for row in rows {
            let title = row.decorate_title.unwrap_or_default();
            let Some(tail) = title.strip_prefix(&prefix) else {
                continue;
            };
            if tail.is_empty() {
                max_seq = max_seq.max(1);
            } else if let Ok(n) = tail.parse::<i64>() {
                if tail.len() <= 8 {
                    max_seq = max_seq.max(n);
                }
            }
        }
        let next = max_seq + 1;
        if next == 1 {
            Ok(prefix)
        } else {
            Ok(format!("{prefix}{next}"))
        }
    }

    /// 楼层展示名：自定义名含"-"时替换为"[分类名]"，空则用分类名，多个以"，"连接。
    async fn resolve_cat_floor_names(
        &self,
        ids: &[i32],
        alias: Option<Vec<String>>,
    ) -> Result<String> {
        let categories = self.repos.catalog.category_all.find_categories_by_ids(ids).await?;
        let mut parts: Vec<String> = Vec::new();
        for (idx, id) in ids.iter().enumerate() {
            let cat_name = categories
                .iter()
                .find(|c| c.category_id == *id)
                .map(|c| c.category_name.clone())
                .unwrap_or_default();
            let alias_name = alias
                .as_ref()
                .and_then(|list| list.get(idx))
                .cloned()
                .unwrap_or_default();
            let display = if alias_name.is_empty() {
                cat_name
            } else if let Some(pos) = alias_name.find('-') {
                format!("{}[{}]{}", &alias_name[..pos], cat_name, &alias_name[pos + 1..])
            } else {
                alias_name
            };
            if !display.is_empty() {
                parts.push(display);
            }
        }
        Ok(parts.join("，"))
    }

    /// 解析分享导入 url 中的 sn/token。
    fn parse_share_url(url: &str) -> Option<(String, String)> {
        let query = url.split_once('?').map(|(_, q)| q).unwrap_or(url);
        let mut sn = None;
        let mut token = None;
        for pair in query.split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or_default();
            let v = kv.next().unwrap_or_default();
            match k {
                "sn" => sn = Some(v.to_string()),
                "token" => token = Some(v.to_string()),
                _ => {}
            }
        }
        match (sn, token) {
            (Some(s), Some(t)) if !s.is_empty() && !t.is_empty() => Some((s, t)),
            _ => None,
        }
    }

    /// 生成分享记录（sn/token/有效期 7 天）。
    async fn generate_share(&self, decorate_id: i32) -> Result<DecorateShareVO> {
        let now = now_seconds() as i32;
        let mut s = DecorateShare {
            share_id: 0,
            share_sn: random_hex(SHARE_CODE_LENGTH),
            share_token: random_hex(SHARE_TOKEN_LENGTH),
            decorate_id,
            valid_time: now + SHARE_EXPIRE_SECONDS,
            create_time: now,
            update_time: now,
        };
        s.share_id = self.repos.content.decorate.insert_share(&s).await?;
        let mut vo = DecorateShareVO::from(&s);
        vo.api_url = Some(share_api_url(&s.share_sn, &s.share_token));
        Ok(vo)
    }

    /// 按类型/页面用途读取已发布首页装修（`shop_id = 0`、`locale_id = 0`），
    /// 再按 `locale_code` 解析语言子页（原版 `loadDecorateModule`）。
    async fn decorate_module(
        &self,
        decorate_type: Option<i32>,
        page_type: Option<i32>,
        is_home: bool,
        status: Option<i32>,
        locale_code: Option<&str>,
    ) -> Result<DecorateModuleListVO> {
        let decorate = self
            .repos.content.decorate
            .find_home_page(
                decorate_type,
                page_type,
                Some(DEFAULT_SHOP_ID),
                status,
                Some(0),
            )
            .await?;
        let Some(decorate) = decorate else {
            // TODO(域实现者)：Java 无记录时回退 `InitDecorateDataConstants` 内置模板数据
            return Ok(DecorateModuleListVO {
                is_home: Some(i32::from(is_home)),
                ..DecorateModuleListVO::default()
            });
        };
        let decorate = resolve_locale_child(&self.repos, decorate, locale_code).await?;
        let mut vo = Self::module_vo(decorate.decorate_id, &decorate.data);
        vo.bind_decorate_id = decorate.binded_h5_id;
        vo.is_home = decorate.is_home;
        Ok(vo)
    }

    /// 预览装修（草稿优先），对应 `DecorateServiceImpl#getPreviewDecorate`。
    async fn preview_decorate(
        &self,
        decorate_type: Option<i32>,
        decorate_id: i32,
        id: Option<i32>,
        _page_type: i32,
    ) -> Result<DecorateModuleListVO> {
        let decorate = self
            .repos.content.decorate
            .find_by_id(decorate_id)
            .await?
            .filter(|d| decorate_type.is_none() || d.decorate_type == decorate_type)
            .ok_or_else(|| Error::biz_msg("装修模板不存在!"))?;

        // 指定 id（已发布页面）用 data，否则草稿优先
        let source = if id.is_some() {
            decorate.data.clone()
        } else {
            decorate
                .draft_data
                .clone()
                .filter(|s| !s.trim().is_empty())
                .or_else(|| decorate.data.clone())
        };
        let mut vo = Self::module_vo(decorate.decorate_id, &source);
        vo.bind_decorate_id = decorate.binded_h5_id;
        vo.is_home = decorate.is_home;
        Ok(vo)
    }

    /// 自定义页装修读取，对应 `DecorateServiceImpl#customDecorate`
    /// （不按页面用途过滤，取该店铺最新一条自定义页），
    /// 再按 `locale_code` 解析语言子页（原版同）。
    async fn custom_decorate(
        &self,
        locale_code: Option<&str>,
    ) -> Result<DecorateModuleListVO> {
        let decorate = self
            .repos.content.decorate
            .find_home_page(Some(3), None, Some(DEFAULT_SHOP_ID), None, Some(0))
            .await?
            .ok_or_else(|| Error::biz_msg("装修模板不存在!"))?;
        let decorate = resolve_locale_child(&self.repos, decorate, locale_code).await?;
        let mut vo = Self::module_vo(decorate.decorate_id, &decorate.data);
        vo.bind_decorate_id = decorate.binded_h5_id;
        vo.is_home = decorate.is_home;
        Ok(vo)
    }

    /// 解析页面数据为 `DecorateModuleListVO`（过滤 `isShow = false` 的模块）。
    fn module_vo(decorate_id: i32, raw: &Option<String>) -> DecorateModuleListVO {
        let mut vo = DecorateModuleListVO {
            decorate_id: Some(decorate_id),
            ..DecorateModuleListVO::default()
        };
        let Some(value) = parse_json_opt(raw) else {
            return vo;
        };
        let page_module = value
            .get("pageModule")
            .or_else(|| value.get("page_module"))
            .cloned();
        let modules = value
            .get("moduleList")
            .or_else(|| value.get("module_list"))
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter(|m| Self::module_visible(m))
                    .cloned()
                    .collect::<Vec<_>>()
            });
        vo.module_list = modules;
        vo.page_module = page_module;
        vo
    }

    /// 模块是否展示（兼容 `isShow` 布尔 / 1 / 0 与 `is_show`）。
    fn module_visible(m: &serde_json::Value) -> bool {
        let fallback = serde_json::Value::Bool(true);
        let flag = m.get("isShow").or_else(|| m.get("is_show")).unwrap_or(&fallback);
        match flag {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Number(n) => n.as_i64() != Some(0),
            _ => true,
        }
    }
}

/// 语言子行采用判定（纯逻辑，单测覆盖）：非默认语言 + 子行已发布数据非空。
///
/// 子行只有草稿（`data` 为空）时不采用——回退基准页，避免未发布子页把前台打空；
/// 与原版 `loadDecorateModule`「子行 data 为空回退默认数据」的可观察行为一致。
#[must_use]
pub(crate) fn locale_child_usable(is_default: bool, child_data: Option<&str>) -> bool {
    !is_default && child_data.is_some_and(|d| !d.trim().is_empty())
}

/// 按请求语言解析基准页的语言子行（原版 `DecorateServiceImpl#getLocaleDecorate` /
/// `loadDecorateModule` / `customDecorate` 的 parentId+localeId 解析逻辑，前台装修
/// 读取共用）：`locale_code`（请求头 `X-Locale-Code`）为空、语言不存在或为默认
/// 语言、子行不存在或未发布数据时，一律返回基准页本身。
pub(crate) async fn resolve_locale_child(
    repos: &Repositories,
    base: Decorate,
    locale_code: Option<&str>,
) -> Result<Decorate> {
    let Some(code) = locale_code.map(str::trim).filter(|c| !c.is_empty()) else {
        return Ok(base);
    };
    let Some(locale) = repos.lang.locales.find_by_code(code).await? else {
        return Ok(base);
    };
    if locale.is_default == Some(1) {
        return Ok(base);
    }
    let child = repos
        .content
        .decorate
        .find_by_parent_locale(base.decorate_id, Some(locale.id), base.decorate_type)
        .await?;
    match child {
        Some(child) if locale_child_usable(locale.is_default == Some(1), child.data.as_deref()) => {
            Ok(child)
        }
        _ => Ok(base),
    }
}

/// 当前 H5 首页渲染器仍使用旧装修模块类型；后台新类型需要降级转换，
/// 否则整块被忽略（例如 `mixedNotice` 不会渲染）。
///
/// 字段映射依据：
/// - 旧组件：`java/view/uniapp/src/pages/customPage/modules/<type>/index.vue`
/// - 新编辑器默认值：`java/view/admin/src/views/decorate/decorate/editor/modules/*`
fn convert_legacy_h5_modules(vo: &mut DecorateModuleListVO) {
    let Some(modules) = vo.module_list.as_mut() else {
        return;
    };
    for item in modules.iter_mut() {
        let Some(kind) = item.get("type").and_then(serde_json::Value::as_str) else {
            continue;
        };
        // H5 已有原版 mixedNotice 组件，保留原类型和 noticeGroups，禁止降级成旧公告
        if kind == "mixedNotice" {
            continue;
        }
        match kind {
            // mixedNotice → notice：旧 notice 组件固定读取
            // icoPic.picUrl / nociceEffect(scroll|still) / frame{...} / text。
            // still 模式渲染 `$t(module.text)`（字符串），scroll 模式渲染
            // `copyArray(module.text, 3)`（数组）——text 的类型必须跟随模式。
            "mixedNotice" => {
                let Some(module) = item.get("module") else {
                    continue;
                };
                let mut texts = Vec::new();
                if let Some(groups) = module.get("noticeGroups").and_then(serde_json::Value::as_array) {
                    for group in groups {
                        for notice in group
                            .get("notices")
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            let text = notice
                                .get("picDesc")
                                .or_else(|| notice.get("picTitle"))
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default();
                            if !text.is_empty() {
                                texts.push(serde_json::Value::String(text.to_string()));
                            }
                        }
                    }
                }
                // 演示装修数据里同一条公告可能重复多遍；旧 notice 组件
                // scroll 模式还会再复制 3 份，同句连刷观感异常 —— 去重（保序）
                let mut seen = std::collections::HashSet::new();
                texts.retain(|t| {
                    t.as_str()
                        .map(|s| seen.insert(s.to_string()))
                        .unwrap_or(true)
                });
                let scroll = texts.len() > 1;
                // H5 新版公告组件 noticeA3：竖排轮播，样式与“公告混排”一致
                let notice_list: Vec<serde_json::Value> = texts
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "title": t,
                            "prefix": "",
                            "link": {},
                        })
                    })
                    .collect();
                let module_style = module.get("moduleStyle").cloned().unwrap_or_default();
                let content_style = module.get("contentStyle").cloned().unwrap_or_default();
                let module_color = module.get("moduleColor").cloned().unwrap_or_default();
                let icon = module
                    .pointer("/noticeGroups/0/notices/0/picThumb")
                    .or_else(|| module.pointer("/noticeGroups/0/notices/0/picUrl"))
                    .cloned()
                    .unwrap_or_default();
                let mut new_module = module.clone();
                new_module["text"] = if scroll {
                    serde_json::Value::Array(texts)
                } else {
                    texts
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| serde_json::Value::String(String::new()))
                };
                new_module["icoPic"] = serde_json::json!({
                    "picUrl": icon,
                    "picThumb": icon,
                });
                // 旧 formatFrame 还会读 innerPadding / titleFontSize / titleFontWeight，
                // 缺省时避免出现 "padding:undefinedpx" 之类的脏样式。
                new_module["frame"] = serde_json::json!({
                    "backgroundColor": module_style.get("backgroundColor").cloned().unwrap_or_else(|| serde_json::json!("")),
                    "innerPadding": content_style.get("padding").cloned().unwrap_or_else(|| serde_json::json!(0)),
                    "boxPadding": module_style.get("boxPadding").cloned().unwrap_or_else(|| serde_json::json!(10)),
                    "boxPaddingTop": module_style.get("boxPaddingTop").cloned().unwrap_or_else(|| serde_json::json!(5)),
                    "boxPaddingBottom": module_style.get("boxPaddingBottom").cloned().unwrap_or_else(|| serde_json::json!(0)),
                    "boxRadius": content_style.get("boxRadius").cloned().unwrap_or_else(|| serde_json::json!(0)),
                    "itemBackgroundColor": content_style.get("backgroundColor").cloned().unwrap_or_else(|| serde_json::json!("")),
                    "itemRadius": content_style.get("boxRadius").cloned().unwrap_or_else(|| serde_json::json!(0)),
                    "itemHeight": 35,
                    "textColor": module_color.get("titleColor").cloned().unwrap_or_else(|| serde_json::json!("#333333")),
                    "titleFontSize": 14,
                    "titleFontWeight": "normal",
                });
                new_module["nociceEffect"] =
                    serde_json::Value::String(if scroll { "scroll" } else { "still" }.into());
                // noticeA3 读取的字段（新版公告混排样式）
                new_module["noticeList"] = serde_json::Value::Array(notice_list);
                new_module["showHeader"] = serde_json::json!(0);
                new_module["showPrefix"] = serde_json::json!(0);
                new_module["showBorder"] = serde_json::json!(0);
                new_module["headerImage"] = serde_json::json!({ "picUrl": icon });
                new_module["textColor"] = module_color
                    .get("titleColor")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!("#666666"));
                new_module["contentBgColor"] = content_style
                    .get("backgroundColor")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!(""));
                item["type"] = serde_json::Value::String("noticeA3".into());
                item["module"] = new_module;
            }
            // 新版装修类型 → H5 仍只识别的旧类型
            "searchBar2" => item["type"] = serde_json::Value::String("searchBar".into()),
            "imageAd2" => item["type"] = serde_json::Value::String("imageAd".into()),
            "seckill2" => {
                if let Some(module) = item.get_mut("module") {
                    if module.get("title").is_none() {
                        // 旧 commonTitle 读取 title 对象（showTitle/titleText/...）
                        module["title"] = legacy_default_title("限时秒杀", true);
                    }
                }
                item["type"] = serde_json::Value::String("seckill".into());
            }
            // product2（新编辑器）只有 9 个字段；旧 product 组件额外读取
            // style/goodsStyle/goodsRadioStyle/textAlign/textWeight/goodsNameRow/
            // goodsNamePadding/showName/showBrief/showPrice/goodsPadding/imgPadding/
            // waterfall/swiperPageColor/title 对象等，缺字段会导致名称/价格不渲染
            // 或样式类为 undefined。
            "product2" => {
                if let Some(module) = item.get_mut("module") {
                    let round =
                        module.get("goodsRadio").and_then(serde_json::Value::as_i64).unwrap_or(0) == 1;
                    let defaults = serde_json::json!({
                        "style": 1,
                        "goodsStyle": 1,
                        "goodsRadioStyle": if round { 2 } else { 1 },
                        "textAlign": 1,
                        "textWeight": 1,
                        "goodsNameRow": 2,
                        "goodsNamePadding": 1,
                        "showName": 1,
                        "showBrief": 0,
                        "showPrice": 1,
                        "goodsPadding": 5,
                        "imgPadding": 1,
                        "waterfall": 0,
                        "swiperPageColor": "",
                        "backgroundColor": "",
                        "title": legacy_default_title("", false),
                        "frame": {
                            "textColor": "",
                            "itemBackgroundColor": "",
                            "backgroundColor": "",
                            "innerPadding": 0,
                            "itemHeight": 35,
                            "itemRadius": 3,
                            "boxRadius": 0,
                            "boxPadding": 10,
                            "leftAndRightPadding": 0,
                            "boxPaddingTop": 5,
                            "boxPaddingBottom": 5,
                        },
                    });
                    if let (Some(dst), Some(src)) = (module.as_object_mut(), defaults.as_object()) {
                        for (k, v) in src {
                            dst.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                }
                item["type"] = serde_json::Value::String("product".into());
            }
            // categoryNavA3 → categoryNavA2：旧组件额外读取指示条/动画相关字段，
            // 缺省时 indicatorBar 关闭，避免 NaN 样式。
            "categoryNavA3" => {
                if let Some(module) = item.get_mut("module") {
                    let defaults = serde_json::json!({
                        "indicatorBar": 0,
                        "animation": 0,
                        "barColor": "#e6e6e6",
                        "barColorOn": "#333333",
                        "barWidth": 30,
                        "barWidthOn": 20,
                        "barHeight": 4,
                        "rowPicPercent": 90,
                    });
                    if let (Some(dst), Some(src)) = (module.as_object_mut(), defaults.as_object()) {
                        for (k, v) in src {
                            dst.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                }
                item["type"] = serde_json::Value::String("categoryNavA2".into());
            }
            // storeOrderCard（点单卡片，mobilePro/门店专业版）→ imageNav：
            // H5 渲染器（uniapp customPage v-if 链）没有该组件，未知 type 整块
            // 跳过。降级为图片导航网格（picUrl + picTitle + picLink），
            // desc/卡片背景/卡片模板细节不渲染。cardList.link 与 picList.picLink
            // 同为 admin 链接选择器的 {path,data} 对象，直接透传。
            "storeOrderCard" => {
                if let Some(module) = item.get_mut("module") {
                    let cards = module
                        .get("cardList")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let pic_list: Vec<serde_json::Value> = cards
                        .iter()
                        .filter_map(|c| {
                            let url = ["picUrl", "picThumb"].iter().find_map(|k| {
                                c.get(*k)
                                    .and_then(serde_json::Value::as_str)
                                    .filter(|s| !s.is_empty())
                            })?;
                            Some(serde_json::json!({
                                "picUrl": url,
                                "picThumb": url,
                                "picTitle": c.get("title").cloned().unwrap_or_default(),
                                "picLink": c.get("link").cloned().unwrap_or(serde_json::Value::Null),
                            }))
                        })
                        .collect();
                    // 编辑器模板 1~6 对应 1/2/3/3/4/5 张卡；按实际张数映射
                    // imageNav 网格的 rowNum × colNum
                    let (row, col) = match pic_list.len() {
                        0 | 1 => (1, 1),
                        2 => (2, 1),
                        3 => (3, 1),
                        4 => (2, 2),
                        _ => (3, 2),
                    };
                    let card_gap = module
                        .get("cardGap")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(8));
                    let frame_default = serde_json::json!({
                        "textColor": "",
                        "itemBackgroundColor": "",
                        "backgroundColor": "",
                        "innerPadding": 10,
                        "itemHeight": 35,
                        "itemRadius": 3,
                        "boxRadius": 8,
                        "boxPadding": 10,
                        "leftAndRightPadding": 0,
                        "boxPaddingTop": 5,
                        "boxPaddingBottom": 5,
                    });
                    let defaults = serde_json::json!({
                        "picList": pic_list,
                        "navType": 1,
                        "navStyle": 1,
                        "rowNum": row,
                        "colNum": col,
                        "picPageType": 1,
                        "radioStyle": 1,
                        "imgPadding": card_gap,
                        "swiperPageColor": "#333333",
                        "frame": frame_default,
                    });
                    if let (Some(dst), Some(src)) = (module.as_object_mut(), defaults.as_object()) {
                        for (k, v) in src {
                            if k == "frame" {
                                // 编辑器保存的 frame 已含完整样式，仅回填缺失键
                                if let (Some(dst_f), Some(src_f)) = (
                                    dst.get_mut("frame").and_then(serde_json::Value::as_object_mut),
                                    v.as_object(),
                                ) {
                                    for (fk, fv) in src_f {
                                        dst_f.entry(fk.clone()).or_insert(fv.clone());
                                    }
                                } else {
                                    dst.insert(k.clone(), v.clone());
                                }
                            } else {
                                dst.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
                item["type"] = serde_json::Value::String("imageNav".into());
            }
            _ => {}
        }
    }
}

/// 旧版 commonTitle 组件要求的标题对象（对齐 admin 编辑器 `defaultTitle`）。
fn legacy_default_title(title_text: &str, show_title: bool) -> serde_json::Value {
    serde_json::json!({
        "showTitle": if show_title { 1 } else { 0 },
        "titleStyle": 1,
        "titleAlign": 1,
        "titleBackground": "",
        "titleBackground2": "",
        "titleBackgroundPic": { "picUrl": "", "picThumb": "" },
        "titleRadius": 0,
        "titleText": title_text,
        "titleColor": "",
        "titleWeight": "normal",
        "descText": "",
        "descColor": "#aaaaaa",
        "showMore": 0,
        "moreType": 1,
        "countdownLabel": "",
        "moreLink": [],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vo_with(module: serde_json::Value) -> DecorateModuleListVO {
        DecorateModuleListVO {
            module_list: Some(vec![module]),
            ..Default::default()
        }
    }

    #[test]
    fn store_order_card_downgrades_to_image_nav() {
        let mut vo = vo_with(serde_json::json!({
            "type": "storeOrderCard",
            "isShow": true,
            "module": {
                "picType": 2,
                "cardStyle": 1,
                "cardGap": 6,
                "frame": { "boxRadius": 8 },
                "cardList": [
                    {
                        "picUrl": "/a.jpg",
                        "picThumb": "/a_t.jpg",
                        "title": "堂食点单",
                        "desc": "ignored",
                        "link": { "path": "store", "data": { "id": 3 } }
                    },
                    {
                        "picUrl": "",
                        "picThumb": "/b_t.jpg",
                        "title": "外带点单",
                        "link": {}
                    }
                ]
            }
        }));
        convert_legacy_h5_modules(&mut vo);
        let item = &vo.module_list.unwrap()[0];
        assert_eq!(item["type"], "imageNav");
        let m = &item["module"];
        // picUrl 为空时回退 picThumb
        assert_eq!(m["picList"][0]["picUrl"], "/a.jpg");
        assert_eq!(m["picList"][1]["picUrl"], "/b_t.jpg");
        assert_eq!(m["picList"][0]["picTitle"], "堂食点单");
        assert_eq!(m["picList"][0]["picLink"]["path"], "store");
        // 2 张卡 → 2 列 1 行；cardGap 透传 imgPadding；编辑器 frame 保留
        assert_eq!(m["rowNum"], 2);
        assert_eq!(m["colNum"], 1);
        assert_eq!(m["imgPadding"], 6);
        assert_eq!(m["frame"]["boxRadius"], 8);
        // frame 缺失键被回填
        assert_eq!(m["frame"]["boxPadding"], 10);
    }

    #[test]
    fn shop_display_name_platform_and_fallback() {
        // 店铺列展示口径【我方新增】：平台固定「平台」，不取 shop 表脏标题
        assert_eq!(
            DecorateService::shop_display_name(0, Some("string".into())).as_deref(),
            Some("平台")
        );
        assert_eq!(
            DecorateService::shop_display_name(0, None).as_deref(),
            Some("平台")
        );
        // 商家店铺取真实店名；查不到/空名回退 None（前端显示店铺 ID）
        assert_eq!(
            DecorateService::shop_display_name(1667501125, Some("锂源循环·济南".into())).as_deref(),
            Some("锂源循环·济南")
        );
        assert_eq!(DecorateService::shop_display_name(9, None), None);
        assert_eq!(DecorateService::shop_display_name(9, Some("  ".into())), None);
    }

    #[test]
    fn batch_del_home_page_guard_predicate() {
        // 我方改口径护栏（用户 2026-11-19 拍板）：仅 is_home = 1 视为默认页受保护
        let mut row = Decorate::default();
        row.is_home = Some(1);
        assert!(is_home_page(&row));
        row.is_home = Some(0);
        assert!(!is_home_page(&row));
        row.is_home = None;
        assert!(!is_home_page(&row));
    }

    #[test]
    fn store_order_card_without_pics_gets_empty_list() {
        let mut vo = vo_with(serde_json::json!({
            "type": "storeOrderCard",
            "module": { "cardList": [ { "title": "no pic" } ] }
        }));
        convert_legacy_h5_modules(&mut vo);
        let item = &vo.module_list.unwrap()[0];
        assert_eq!(item["type"], "imageNav");
        // imageNav 对空 picList 不渲染（v-if length > 0），安全降级
        assert_eq!(item["module"]["picList"].as_array().map(Vec::len), Some(0));
    }

    /// 语言子行采用判定：默认语言永不采用；非默认语言仅当子行已发布
    /// 数据非空时采用（草稿子页/空白数据回退基准页）。
    #[test]
    fn locale_child_usable_only_for_published_non_default() {
        let data = Some("{\"moduleList\":[]}");
        assert!(locale_child_usable(false, data));
        assert!(!locale_child_usable(true, data));
        assert!(!locale_child_usable(false, None));
        assert!(!locale_child_usable(false, Some("")));
        assert!(!locale_child_usable(false, Some("   ")));
    }
}
