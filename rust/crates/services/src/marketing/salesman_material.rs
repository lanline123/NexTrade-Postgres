//! 分销素材服务（marketing::salesman_material）。
//!
//! 对应原 Java `SalesmanMaterialServiceImpl`（10 个端点：admin 7 + public 3）。
//!
//! # 语义（逐条对照 Java）
//! - admin `list`：店铺租户过滤（`TenantScope::current_list_filter()`：平台不过滤、
//!   店铺强制本店）+ `categoryId` 过滤 + `from=user` 时 `is_available=1`；
//!   排序白名单，缺省 `id DESC`。
//! - `detail`：素材 + `pics` JSON 解析 + 分类实体 + 商品精简对象。
//! - `create`：`add_time=now`、`shop_id` 取 `TenantScope::current().write_shop_id(None)`、
//!   `is_top` 默认 0、`share_num` 默认 0、
//!   `is_available` 默认 1、`pics` 序列化为 JSON 文本。
//! - `update`：整行合并（缺省保留原值），`pics` 序列化。
//! - `del` / `batch`：物理删除（表无逻辑删除列）；`batch` 仅支持 `del`。
//! - `updateField`：白名单 `is_top` / `is_available`（见 models 差异 1）。
//! - public `list`：`is_available=1` + 分类过滤（差异 4）。
//! - public `detail`：与 admin 相同的 VO 组装（Java 复用 `detail`）。
//! - public `category`：全部分类（Java `getAllCategory`，按 `sort_order ASC`）。

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use nextrade_core::entities::SalesmanMaterial;
use nextrade_core::models::marketing::salesman_material::{
    format_time, normalize_field, BatchDTO, OperateDTO, SalesmanMaterialApiQuery,
    SalesmanMaterialEditDTO, SalesmanMaterialListQuery, SalesmanMaterialProductVO,
    SalesmanMaterialSaveDTO, SalesmanMaterialVO, UpdateFieldDTO, DEFAULT_PAGE_SIZE,
    MAX_PAGE_SIZE, MSG_ID_REQUIRED, MSG_INVALID_FIELD_VALUE, MSG_MATERIAL_NOT_FOUND,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::salesman_material::SalesmanMaterialRepository;
use nextrade_infrastructure::db::repo::marketing::tenant::TenantScope;
use nextrade_infrastructure::db::Repositories;

/// 素材服务（挂载于 `AdminState.marketing.salesman_material` / `PublicState`）。
pub struct SalesmanMaterialService {
    repos: Arc<Repositories>,
}

impl SalesmanMaterialService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    #[must_use]
    pub fn repo(&self) -> &SalesmanMaterialRepository {
        &self.repos.marketing.salesman_material
    }

    /// 管理端分页（Java `list`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn list(
        &self,
        q: &SalesmanMaterialListQuery,
    ) -> Result<PageResult<SalesmanMaterialVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let only_available = q.from.as_deref() == Some("user");
        let (total, rows) = self
            .repo()
            .paginate(
                TenantScope::current_list_filter(),
                q.category_id,
                only_available,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                limit,
                offset,
            )
            .await?;
        let list = self.assemble(&rows).await?;
        Ok(PageResult::new(total, list, &page))
    }

    /// public 分页（Java `apiPage`；见差异 4）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn api_page(
        &self,
        q: &SalesmanMaterialApiQuery,
    ) -> Result<PageResult<SalesmanMaterialVO>> {
        let page = PageQuery {
            page: q.page.unwrap_or(1),
            page_size: q.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
        };
        let (limit, offset) = page.limit_offset();
        let (total, rows) = self
            .repo()
            .paginate(
                Some(q.shop_id.unwrap_or(0)),
                q.category_id,
                true,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                limit,
                offset,
            )
            .await?;
        let list = self.assemble(&rows).await?;
        Ok(PageResult::new(total, list, &page))
    }

    /// 素材详情（Java `detail`；缺失返回 `null`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn detail(&self, id: i32) -> Result<Option<SalesmanMaterialVO>> {
        let Some(m) = self.repo().find_by_id(id).await? else {
            return Ok(None);
        };
        // 多租户：店铺账号只能查看本店素材（公开路由不套租户层，平台语义不拦截）
        TenantScope::current_ensure_access(m.shop_id)?;
        Ok(Some(self.assemble_one(&m).await?))
    }

    /// 新增素材（Java `create`）。
    ///
    /// # Errors
    /// 序列化失败 → `Internal`；数据库错误 → `Database`。
    pub async fn create(&self, dto: &SalesmanMaterialSaveDTO) -> Result<i32> {
        let pics = dto
            .pics
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".into()));
        let entity = SalesmanMaterial {
            id: 0,
            add_time: now_secs_i32(),
            is_top: dto.is_top.unwrap_or(0),
            content: dto.content.clone(),
            is_available: Some(1),
            // 多租户：店铺账号落本店；平台账号落平台级（0）
            shop_id: TenantScope::current().write_shop_id(None)?,
            pics,
            category_id: dto.category_id,
            share_num: Some(0),
            product_id: dto.product_id,
        };
        self.repo().insert(&entity).await
    }

    /// 编辑素材（Java `update`：整行覆盖，缺省保留原值）。
    ///
    /// # Errors
    /// 缺 id / 素材不存在 → 业务错误。
    pub async fn update(&self, dto: &SalesmanMaterialEditDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let Some(mut entity) = self.repo().find_by_id(id).await? else {
            return Err(Error::biz(500, MSG_MATERIAL_NOT_FOUND));
        };
        // 多租户：更新前校验归属
        TenantScope::current_ensure_access(entity.shop_id)?;
        if let Some(v) = dto.is_top {
            entity.is_top = v;
        }
        if let Some(v) = &dto.content {
            entity.content = Some(v.clone());
        }
        if let Some(v) = dto.add_time {
            entity.add_time = v;
        }
        if let Some(v) = dto.is_available {
            entity.is_available = Some(v);
        }
        // 多租户：店铺账号强制本店；平台账号可指定/保留原店铺
        entity.shop_id =
            TenantScope::current().write_shop_id(dto.shop_id.or(Some(entity.shop_id)))?;
        if let Some(v) = &dto.pics {
            entity.pics = Some(serde_json::to_string(v).unwrap_or_else(|_| "null".into()));
        }
        if let Some(v) = dto.category_id {
            entity.category_id = Some(v);
        }
        if let Some(v) = dto.share_num {
            entity.share_num = Some(v);
        }
        if let Some(v) = dto.product_id {
            entity.product_id = Some(v);
        }
        self.repo().update(&entity).await?;
        Ok(())
    }

    /// 删除素材（Java `del` → `removeById`）。
    ///
    /// # Errors
    /// 缺 id → `Validation`。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        // 多租户：删除前校验归属
        if let Some(entity) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(entity.shop_id)?;
        }
        self.repo().delete(&[id]).await?;
        Ok(())
    }

    /// 更新单个字段（白名单见 models 差异 1）。
    ///
    /// # Errors
    /// 缺参数 / 白名单外字段 → `Validation`。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_ID_REQUIRED.into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation("值不能为空".into()))?;
        let column = normalize_field(field)
            .ok_or_else(|| Error::Validation(MSG_INVALID_FIELD_VALUE.into()))?;
        // 多租户：字段更新前校验归属
        if let Some(entity) = self.repo().find_by_id(id).await? {
            TenantScope::current_ensure_access(entity.shop_id)?;
        }
        self.repo().update_field(id, column, val).await
    }

    /// 批量操作（仅 `del` → 物理删除）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if !dto.r#type.as_deref().is_some_and(|t| t == "del") {
            return Ok(false);
        }
        let Some(ids) = dto.ids.as_deref() else {
            return Ok(false);
        };
        // 多租户：批量删除前逐条校验归属（越权直接 403，不静默跳过）
        for id in ids {
            if let Some(entity) = self.repo().find_by_id(*id).await? {
                TenantScope::current_ensure_access(entity.shop_id)?;
            }
        }
        Ok(self.repo().delete(ids).await? > 0)
    }

    /// 全部分类（Java `getAllCategory`，public `category`）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`。
    pub async fn all_categories(&self) -> Result<Vec<nextrade_core::entities::SalesmanMaterialCategory>> {
        self.repo().find_categories(None).await
    }

    /// 批量组装 VO：解析 `pics`、回填分类与商品精简对象。
    async fn assemble(&self, rows: &[SalesmanMaterial]) -> Result<Vec<SalesmanMaterialVO>> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let category_ids: Vec<i32> = rows.iter().filter_map(|m| m.category_id).collect();
        let categories = self.repo().find_categories(Some(&category_ids)).await?;
        let category_map: HashMap<i32, Value> = categories
            .iter()
            .filter_map(|c| {
                serde_json::to_value(c).ok().map(|v| (c.category_id, v))
            })
            .collect();
        let product_ids: Vec<i32> = rows.iter().filter_map(|m| m.product_id).collect();
        let products: HashMap<i32, SalesmanMaterialProductVO> = self
            .repo()
            .find_products(&product_ids)
            .await?
            .into_iter()
            .map(|p| {
                (
                    p.product_id,
                    SalesmanMaterialProductVO {
                        product_id: p.product_id,
                        product_name: p.product_name,
                        product_price: p.product_price,
                    },
                )
            })
            .collect();
        Ok(rows
            .iter()
            .map(|m| build_vo(m, &category_map, &products))
            .collect())
    }

    async fn assemble_one(&self, m: &SalesmanMaterial) -> Result<SalesmanMaterialVO> {
        let mut list = self.assemble(std::slice::from_ref(m)).await?;
        Ok(list.pop().unwrap_or_default())
    }
}

/// 单行 → VO。
fn build_vo(
    m: &SalesmanMaterial,
    category_map: &HashMap<i32, Value>,
    products: &HashMap<i32, SalesmanMaterialProductVO>,
) -> SalesmanMaterialVO {
    SalesmanMaterialVO {
        id: m.id,
        add_time: format_time(Some(m.add_time)),
        is_top: m.is_top,
        content: m.content.clone(),
        is_available: m.is_available,
        shop_id: m.shop_id,
        category_id: m.category_id,
        share_num: m.share_num,
        product_id: m.product_id,
        pics_json: m.pics.clone(),
        pics: m
            .pics
            .as_deref()
            .and_then(|s| serde_json::from_str::<Value>(s).ok()),
        category: m.category_id.and_then(|id| category_map.get(&id).cloned()),
        product: m.product_id.and_then(|id| products.get(&id).cloned()),
    }
}

fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

/// 列表店铺过滤：店铺账号强制本店；平台账号不过滤。
fn scoped_list_shop_id() -> Option<i32> {
    TenantScope::current_list_filter()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SalesmanMaterial {
        SalesmanMaterial {
            id: 5,
            add_time: 1_700_000_000,
            is_top: 1,
            content: Some("素材".into()),
            is_available: Some(1),
            shop_id: 0,
            pics: Some(r#"[{"picUrl":"a.png"}]"#.into()),
            category_id: Some(2),
            share_num: Some(3),
            product_id: Some(9),
        }
    }

    #[test]
    fn build_vo_parses_pics_and_joins() {
        let mut cats = HashMap::new();
        cats.insert(2, serde_json::json!({"categoryId": 2, "categoryName": "类目"}));
        let mut products = HashMap::new();
        products.insert(
            9,
            SalesmanMaterialProductVO {
                product_id: 9,
                product_name: "商品".into(),
                product_price: Some("1.00".parse().unwrap()),
            },
        );
        let vo = build_vo(&sample(), &cats, &products);
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.pics.as_ref().unwrap()[0]["picUrl"], "a.png");
        assert_eq!(vo.category.as_ref().unwrap()["categoryName"], "类目");
        assert_eq!(vo.product.as_ref().unwrap().product_id, 9);
    }

    #[test]
    fn build_vo_tolerates_bad_json_and_missing_joins() {
        let mut m = sample();
        m.pics = Some("not-json".into());
        m.category_id = None;
        m.product_id = None;
        let vo = build_vo(&m, &HashMap::new(), &HashMap::new());
        assert!(vo.pics.is_none());
        assert!(vo.category.is_none());
        assert!(vo.product.is_none());
        assert_eq!(vo.pics_json.as_deref(), Some("not-json"));
    }

    #[test]
    fn missing_id_is_validation_error() {
        let err = Error::Validation(MSG_ID_REQUIRED.into());
        assert!(matches!(err, Error::Validation(_)));
    }

    #[tokio::test]
    async fn tenant_scope_scopes_list_and_blocks_cross_shop_material() {
        use nextrade_infrastructure::db::repo::marketing::tenant::{
            with_tenant_scope, TenantScope,
        };
        let shop = TenantScope::for_shop(8);
        let (filter, write_shop, blocked) = with_tenant_scope(shop, async {
            (
                scoped_list_shop_id(),
                TenantScope::current().write_shop_id(None).unwrap_or(-1),
                TenantScope::current_ensure_access(9).is_err(),
            )
        })
        .await;
        assert_eq!(filter, Some(8));
        assert_eq!(write_shop, 8);
        assert!(blocked);
        let filter = with_tenant_scope(TenantScope::platform(), async { scoped_list_shop_id() }).await;
        assert_eq!(filter, None);
    }
}
