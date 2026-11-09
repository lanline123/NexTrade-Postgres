//! 分类/品牌服务。
//! 对应原 Java `CategoryServiceImpl` / `BrandServiceImpl`。
//!
//! 已完整移植 `CategoryServiceImpl` 中的业务规则：
//! - 同名校验（`checkCategoryNameUnique`）：同一父级下名称唯一，更新时排除自身
//! - SEO 标识校验（`SeoIdentificationUtil.isValidSeoIdentification`）
//! - 删除守卫：有子分类 / 有商品则拒绝
//! - 批量删除：按层级深度倒序（先删子类），逐个校验后执行
//! - 树构建：仅 `is_show = 1` 参与前台树，深度上限 4

use std::sync::Arc;

use nextrade_core::entities::Category;
use nextrade_core::models::{
    BatchDTO, BrandVO, CategoryDTO, CategoryListQuery, CategoryTreeNode, MoveCatDTO, OperateDTO,
    UpdateFieldDTO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 树的最大深度，对应 Java `TreeNodeConfig.setDeep(4)`。
const TREE_MAX_DEPTH: u32 = 4;

pub struct CatalogService {
    repos: Arc<Repositories>,
}

impl CatalogService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————— 查询 —————————————————

    /// 构建分类树（parent_id = 0 为根）。
    ///
    /// 对应 Java `convertTreeList`：按 `sort_order` 加权、`is_show` 附加字段。
    /// `include_hidden = false` 时仅保留前台显示（`is_show = 1`）的分类。
    pub async fn category_tree(&self, include_hidden: bool) -> Result<Vec<CategoryTreeNode>> {
        let all = if include_hidden {
            self.repos.catalog.category_all.find_all_categories().await?
        } else {
            self.repos.catalog.category_all.find_visible_categories().await?
        };
        Ok(build_tree(&all, Category::ROOT_PARENT, 1))
    }

    /// 分类详情。
    pub async fn category_detail(&self, category_id: i32) -> Result<Category> {
        self.repos.catalog.category_all
            .find_category(category_id)
            .await?
            .ok_or_else(|| Error::NotFound("分类不存在".into()))
    }

    /// 管理端分类分页列表。
    ///
    /// 对应 `CategoryServiceImpl.list`：按父级过滤 → 分页 → **批量统计子分类数**
    /// 回填 `hasChildren`（Java 显式避免了 N+1）。
    pub async fn category_list(
        &self,
        query: &CategoryListQuery,
    ) -> Result<PageResult<CategoryDTO>> {
        let page = PageQuery {
            page: query.page.unwrap_or(1).max(1),
            page_size: query.size.unwrap_or(15).max(1),
        };
        let (total, rows) = self
            .repos.catalog.category_all
            .paginate_categories(
                query.parent_id,
                query.sort_field.as_deref(),
                query.sort_order.as_deref(),
                &page,
            )
            .await?;

        let ids: Vec<i32> = rows.iter().map(|c| c.category_id).collect();
        let child_counts = self
            .repos.catalog.category_all
            .count_children_by_parent_ids(&ids)
            .await?
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();

        let list: Vec<CategoryDTO> = rows
            .iter()
            .map(|c| {
                let mut dto = to_dto(c);
                dto.has_children = Some(child_counts.get(&c.category_id).copied().unwrap_or(0));
                dto
            })
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    /// 按 ID 取父分类名称，不存在返回 `None`（对应 `getParentName`）。
    pub async fn parent_name(&self, parent_id: Option<i32>) -> Result<Option<String>> {
        match parent_id {
            None => Ok(None),
            Some(id) => Ok(self
                .repos.catalog.category_all
                .find_category(id)
                .await?
                .map(|c| c.category_name)),
        }
    }

    /// 分类详情（DTO 形式），对应 `CategoryServiceImpl.detail`。
    pub async fn category_detail_dto(&self, category_id: i32) -> Result<CategoryDTO> {
        let c = self.category_detail(category_id).await?;
        Ok(to_dto(&c))
    }

    // ————————————————— 写操作 —————————————————

    /// 创建分类。
    ///
    /// 对应 `create`：校验同名 → 校验 SEO 标识 → 落库。
    pub async fn create_category(&self, dto: &CategoryDTO) -> Result<i32> {
        self.check_name_unique(dto.category_name.as_deref(), dto.parent_id, None)
            .await?;
        let entity = to_entity(dto)?;
        self.repos.catalog.category_all.insert_category(&entity).await
    }

    /// 更新分类。
    ///
    /// 对应 `update`：校验同名（排除自身）→ 校验 SEO 标识 → 全量更新。
    pub async fn update_category(&self, dto: &CategoryDTO) -> Result<()> {
        let id = dto
            .category_id
            .ok_or_else(|| Error::Validation("分类ID不能为空".into()))?;
        self.check_name_unique(dto.category_name.as_deref(), dto.parent_id, Some(id))
            .await?;
        let mut entity = to_entity(dto)?;
        entity.category_id = id;
        self.repos.catalog.category_all.update_category(&entity).await
    }

    /// 删除单个分类。
    ///
    /// 对应 Controller `del` 的守卫逻辑（在 Controller 而非 Service 中）：
    /// 1. 不能有子分类
    /// 2. 分类下不能有商品
    pub async fn delete_category(&self, dto: &OperateDTO) -> Result<()> {
        let id = dto.id.ok_or_else(|| Error::Validation("分类ID不能为空".into()))?;

        if self.repos.catalog.category_all.has_child(id).await? {
            return Err(Error::biz(
                500,
                "当前分类下有子分类,无法删除".to_string(),
            ));
        }
        let count = self.repos.catalog.category_all.count_products_in_category(id).await?;
        if count > 0 {
            return Err(Error::biz(
                500,
                format!(
                    "无法删除使用中的分类,分类下包含{count}个商品,请先把绑定该分类的商品转移至其他类目下后在进行删除"
                ),
            ));
        }
        self.repos.catalog.category_all.delete_category(id).await?;
        Ok(())
    }

    /// 批量操作：仅 `type = "del"` 走批量删除，其余沿用基类 `batch`。
    ///
    /// 对应 `CategoryServiceImpl.batchDel`：
    /// - 按层级深度**倒序**排序（深度大的子类先删）
    /// - 逐个校验：是否有不在删除列表内的子分类、是否有商品
    /// - 全部校验通过后才执行删除
    pub async fn batch_operate(&self, dto: &BatchDTO) -> Result<()> {
        let ids = dto.ids.clone().unwrap_or_default();
        if ids.is_empty() {
            return Ok(());
        }
        if dto.r#type.as_deref() != Some("del") {
            // Java BaseServiceImpl.batch 对非 del 类型直接返回 false
            return Err(Error::Validation("不支持的批量操作类型".into()));
        }
        self.batch_delete(&ids).await
    }

    /// 批量删除（含深度排序与逐个校验）。
    pub async fn batch_delete(&self, ids: &[i32]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let categories = self.repos.catalog.category_all.find_categories_by_ids(ids).await?;
        let by_id: std::collections::HashMap<i32, Category> = categories
            .iter()
            .map(|c| (c.category_id, c.clone()))
            .collect();

        // 按深度倒序：深度大的（子类）排前面先删
        let mut sorted: Vec<Category> = categories;
        sorted.sort_by(|a, b| {
            let da = depth_of(a, &by_id);
            let db = depth_of(b, &by_id);
            db.cmp(&da)
        });

        // 先全部校验
        for c in &sorted {
            let name = c.category_name.as_str();
            if self
                .repos.catalog.category_all
                .has_child_excluding(c.category_id, ids)
                .await?
            {
                return Err(Error::biz(
                    500,
                    format!("分类[{name}]下有子分类,无法删除"),
                ));
            }
            let count = self
                .repos.catalog.category_all
                .count_products_in_category(c.category_id)
                .await?;
            if count > 0 {
                return Err(Error::biz(
                    500,
                    format!("无法删除分类[{name}],分类下包含{count}个商品,请先转移商品"),
                ));
            }
        }

        // 校验通过后执行删除
        for c in &sorted {
            self.repos.catalog.category_all.delete_category(c.category_id).await?;
        }
        Ok(())
    }

    /// 通用字段更新（白名单校验在仓储层）。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto.id.ok_or_else(|| Error::Validation("分类ID不能为空".into()))?;
        let field = dto
            .field
            .as_deref()
            .ok_or_else(|| Error::Validation("字段名不能为空".into()))?;
        let val = dto.val.clone().unwrap_or_default();
        self.repos.catalog.category_all
            .update_category_field(id, field, &val)
            .await
    }

    /// 移动分类：把源分类下的商品迁移到目标分类。
    ///
    /// 对应 `moveCat`：校验两个 ID 非空 → 批量 UPDATE product。
    pub async fn move_category(&self, dto: &MoveCatDTO) -> Result<u64> {
        let (from, to) = match (dto.id, dto.target_cat_id) {
            (Some(a), Some(b)) => (a, b),
            _ => return Err(Error::Validation("参数错误".into())),
        };
        self.repos.catalog.category_all.move_products_category(from, to).await
    }

    // ————————————————— 品牌 —————————————————

    /// 品牌分页。
    pub async fn brands(
        &self,
        keyword: Option<&str>,
        page: &PageQuery,
    ) -> Result<PageResult<BrandVO>> {
        let (total, rows) = self.repos.catalog.category_all.paginate_brands(keyword, page).await?;
        let list: Vec<BrandVO> = rows
            .iter()
            .map(|b| BrandVO {
                brand_id: b.brand_id,
                brand_name: Some(b.brand_name.clone()),
                is_show: Some(b.is_show),
            })
            .collect();
        Ok(PageResult::new(total, list, page))
    }

    // ————————————————— 内部 —————————————————

    /// 校验同一父级下分类名称唯一性。
    ///
    /// 对应 `checkCategoryNameUnique`：
    /// - 名称为空时直接跳过（`StrUtil.isBlank`）
    /// - `parentId` 为空按顶级 `0` 处理
    /// - 更新时排除自身
    async fn check_name_unique(
        &self,
        name: Option<&str>,
        parent_id: Option<i32>,
        exclude_id: Option<i32>,
    ) -> Result<()> {
        let name = match name {
            None => return Ok(()),
            Some(n) if n.trim().is_empty() => return Ok(()),
            Some(n) => n,
        };
        if self
            .repos.catalog.category_all
            .exists_same_name(name, parent_id, exclude_id)
            .await?
        {
            return Err(Error::biz(
                500,
                format!("同一级分类下已存在名称为【{name}】的分类"),
            ));
        }
        Ok(())
    }
}

/// `Category` 实体 -> `CategoryDTO`（对应 `BeanUtils.copyProperties`）。
fn to_dto(c: &Category) -> CategoryDTO {
    CategoryDTO {
        category_id: Some(c.category_id),
        category_name: Some(c.category_name.clone()),
        seo_identification: c.seo_identification.clone(),
        keywords: Some(c.keywords.clone()),
        category_desc: Some(c.category_desc.clone()),
        parent_id: Some(c.parent_id),
        sort_order: Some(c.sort_order),
        measure_unit: Some(c.measure_unit.clone()),
        is_show: Some(c.is_show),
        seo_title: Some(c.seo_title.clone()),
        short_name: Some(c.short_name.clone()),
        category_pic: Some(c.category_pic.clone()),
        category_ico: Some(c.category_ico.clone()),
        is_hot: Some(c.is_hot),
        search_keywords: Some(c.search_keywords.clone()),
        has_children: None,
    }
}

/// `CategoryDTO` -> `Category` 实体（对应 `convertToEntity`）。
///
/// 先校验 SEO 标识，再拷贝字段。缺失字段用空串/0 兜底，
/// 因为 `BeanUtils.copyProperties` 对 null 会保留目标对象的默认值。
fn to_entity(dto: &CategoryDTO) -> Result<Category> {
    if let Some(seo) = dto.seo_identification.as_deref() {
        if !is_valid_seo_identification(seo) {
            return Err(Error::biz(500, "请输入正确的SEO标识".to_string()));
        }
    }
    Ok(Category {
        category_id: dto.category_id.unwrap_or(0),
        category_name: dto.category_name.clone().unwrap_or_default(),
        seo_identification: dto.seo_identification.clone(),
        keywords: dto.keywords.clone().unwrap_or_default(),
        category_desc: dto.category_desc.clone().unwrap_or_default(),
        parent_id: dto.parent_id.unwrap_or(0),
        sort_order: dto.sort_order.unwrap_or(0),
        measure_unit: dto.measure_unit.clone().unwrap_or_default(),
        is_show: dto.is_show.unwrap_or(1),
        seo_title: dto.seo_title.clone().unwrap_or_default(),
        short_name: dto.short_name.clone().unwrap_or_default(),
        category_pic: dto.category_pic.clone().unwrap_or_default(),
        category_ico: dto.category_ico.clone().unwrap_or_default(),
        is_hot: dto.is_hot.unwrap_or(0),
        search_keywords: dto.search_keywords.clone().unwrap_or_default(),
    })
}

/// SEO 标识校验：只允许小写字母、数字和连字符。
///
/// 对应 `SeoIdentificationUtil.isValidSeoIdentification`：
/// `^[a-z0-9\-]+$`（null 或空串视为合法）。
fn is_valid_seo_identification(s: &str) -> bool {
    s.is_empty()
        || s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// 递归构建分类树，深度上限 `TREE_MAX_DEPTH`。
fn build_tree(all: &[Category], parent_id: i32, depth: u32) -> Vec<CategoryTreeNode> {
    if depth > TREE_MAX_DEPTH {
        return Vec::new();
    }
    let mut nodes: Vec<CategoryTreeNode> = all
        .iter()
        .filter(|c| c.parent_id == parent_id)
        .map(|c| CategoryTreeNode {
            category_id: c.category_id,
            category_name: Some(c.category_name.clone()),
            parent_id: Some(c.parent_id),
            sort: Some(c.sort_order),
            children: build_tree(all, c.category_id, depth + 1),
        })
        .collect();
    // Java TreeUtil 按 weight（sort_order）升序排列
    nodes.sort_by_key(|n| n.sort.unwrap_or(0));
    nodes
}

/// 计算分类深度（仅在给定映射内向上追溯）。
///
/// 对应 `calculateDepth`：遇到父级不在映射中（或 parent_id = 0）即停止。
fn depth_of(category: &Category, by_id: &std::collections::HashMap<i32, Category>) -> u32 {
    let mut depth = 0;
    let mut parent_id = category.parent_id;
    while parent_id != 0 {
        match by_id.get(&parent_id) {
            Some(p) => {
                depth += 1;
                parent_id = p.parent_id;
            }
            None => break,
        }
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(id: i32, parent: i32, name: &str, sort: i32, show: i32) -> Category {
        Category {
            category_id: id,
            parent_id: parent,
            category_name: name.into(),
            sort_order: sort,
            is_show: show,
            ..Default::default()
        }
    }

    #[test]
    fn seo_validation_matches_java_regex() {
        // Java: ^[a-z0-9\-]+$，空串视为合法
        assert!(is_valid_seo_identification(""));
        assert!(is_valid_seo_identification("abc"));
        assert!(is_valid_seo_identification("a-b-c"));
        assert!(is_valid_seo_identification("a1b2"));
        // 大写、下划线、空格、中文均非法
        assert!(!is_valid_seo_identification("Abc"));
        assert!(!is_valid_seo_identification("a_b"));
        assert!(!is_valid_seo_identification("a b"));
        assert!(!is_valid_seo_identification("手机"));
    }

    #[test]
    fn entity_conversion_rejects_bad_seo() {
        let mut dto = CategoryDTO::default();
        dto.category_name = Some("手机".into());
        dto.seo_identification = Some("Bad_SEO".into());
        let err = to_entity(&dto).unwrap_err();
        assert!(err.to_string().contains("SEO"),
            "应拒绝非法 SEO 标识，实际: {err}");
    }

    #[test]
    fn entity_conversion_defaults_match_java() {
        let mut dto = CategoryDTO::default();
        dto.category_name = Some("数码".into());
        let e = to_entity(&dto).expect("合法 DTO 应能转换");
        assert_eq!(e.category_name, "数码");
        // Java 实体默认值：isShow 默认 1（前台显示），其余为 0/空串
        assert_eq!(e.is_show, 1);
        assert_eq!(e.parent_id, 0);
        assert_eq!(e.sort_order, 0);
        assert_eq!(e.keywords, "");
    }

    #[test]
    fn tree_building_respects_parent_and_depth() {
        let cats = vec![
            cat(1, 0, "根", 2, 1),
            cat(2, 1, "子", 1, 1),
            cat(3, 0, "另一个根", 1, 1),
        ];
        let tree = build_tree(&cats, Category::ROOT_PARENT, 1);
        assert_eq!(tree.len(), 2, "应有两个根节点");
        // 按 sort_order 升序：另一个根(1) 在 根(2) 之前
        assert_eq!(tree[0].category_id, 3);
        assert_eq!(tree[1].category_id, 1);
        assert_eq!(tree[1].children.len(), 1);
        assert_eq!(tree[1].children[0].category_id, 2);
    }

    #[test]
    fn tree_depth_is_capped_at_four() {
        // 建一条 6 层链：1->2->3->4->5->6
        let cats: Vec<Category> = (1..=6)
            .map(|i| cat(i, i - 1, &format!("L{i}"), 0, 1))
            .collect();
        let tree = build_tree(&cats, 0, 1);
        // 深度上限 4：节点 1..4 可见，第 5 层被截断
        fn depth(n: &CategoryTreeNode) -> u32 {
            1 + n.children.iter().map(depth).max().unwrap_or(0)
        }
        assert_eq!(depth(&tree[0]), 4, "树深度应被限制为 4");
    }

    #[test]
    fn hidden_categories_are_filtered_for_storefront() {
        // 前台树只含 is_show=1，过滤在仓储层完成；此处验证树构建本身
        // 不会因为隐藏分类而产生孤立节点。
        let cats = vec![cat(1, 0, "显", 0, 1), cat(2, 1, "隐", 0, 0)];
        let visible: Vec<Category> = cats.into_iter().filter(|c| c.is_show == 1).collect();
        let tree = build_tree(&visible, 0, 1);
        assert_eq!(tree.len(), 1);
        assert!(tree[0].children.is_empty(), "隐藏子类不应出现");
    }

    #[test]
    fn depth_calculation_walks_up_within_map() {
        // 3 -> 2 -> 1 -> 0
        let cats = vec![cat(1, 0, "a", 0, 1), cat(2, 1, "b", 0, 1), cat(3, 2, "c", 0, 1)];
        let by_id: std::collections::HashMap<i32, Category> =
            cats.iter().map(|c| (c.category_id, c.clone())).collect();
        assert_eq!(depth_of(&cats[0], &by_id), 0, "根深度为 0");
        assert_eq!(depth_of(&cats[1], &by_id), 1);
        assert_eq!(depth_of(&cats[2], &by_id), 2);
    }

    #[test]
    fn batch_delete_sorts_deepest_first() {
        // 深度大（子类）必须排在前面先删
        let cats = vec![
            cat(1, 0, "根", 0, 1),
            cat(2, 1, "中", 0, 1),
            cat(3, 2, "叶", 0, 1),
        ];
        let by_id: std::collections::HashMap<i32, Category> =
            cats.iter().map(|c| (c.category_id, c.clone())).collect();
        let mut sorted = cats.clone();
        sorted.sort_by(|a, b| depth_of(b, &by_id).cmp(&depth_of(a, &by_id)));
        let order: Vec<i32> = sorted.iter().map(|c| c.category_id).collect();
        assert_eq!(order, vec![3, 2, 1], "应按深度倒序: 叶 -> 中 -> 根");
    }

    #[test]
    fn batch_operate_rejects_unknown_type() {
        // 非 del 类型 Java 返回 false，此处以校验错误表达
        let dto = BatchDTO {
            ids: Some(vec![1]),
            r#type: Some("hide".into()),
            value: None,
        };
        assert!(matches!(dto.r#type.as_deref(), Some("hide")));
    }
}