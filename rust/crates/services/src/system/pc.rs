//! PC 前台公共接口服务（system::pc）：导航栏（`getNav`）与分类抽屉（`getCatFloor`）。
//!
//! 对应原 Java `PCController`（`@RequestMapping("/api/common/pc")`）：
//!
//! | 本切片端点 | Java 实现链 |
//! |------------|-------------|
//! | `GET /api/common/pc/getNav` | `PCController#getNav` → `PcNavigationServiceImpl#getAllNav(0)` |
//! | `GET /api/common/pc/getCatFloor` | `PCController#getCatFloor` → `PcCatFloorServiceImpl#getCatFloor` + `ConfigService#getConfigVal(ICO_DEFINED_CSS)` |
//!
//! ## `getNav` 的等价实现要点
//!
//! 1. 查 `pc_navigation`：`is_show = 1`（`type = 0` → **不过滤 type**），
//!    `is-merchant == 0 && is-o2o == 0` 时排除 `id = 42`（商家入驻），
//!    `ORDER BY sort_order ASC, parent_id ASC`。
//!    本项目 `application.yaml` 为 `is-merchant: 1` / `is-o2o: 0`，
//!    故 42 **不**被排除（常量 [`IS_MERCHANT`] / [`IS_O2O`]）。
//! 2. `TreeUtil.build(list, 0, config, parser)`（`deep = 2`）建树：
//!    - hutool 先把所有节点按 `weight`（= `sort_order`）**升序稳定排序**
//!      （`MapUtil.sortByValue(idTreeMap, false)`，`Tree.compareTo` 比较 weight），
//!      相等时保持 SQL 返回顺序；
//!    - 再依次挂到 `parent_id` 对应的父节点上，**父节点不在结果集中的节点被丢弃**
//!      （例如父导航 `is_show = 0`）；
//!    - `deep = 2` 的 `cutTree` 会把「第 2 层节点」的子节点剪成 `null`，
//!      即最终输出「顶级导航 + 其直接子导航」，更深层被裁掉。
//! 3. 按 `PcNavigationType`（1/2/3/4）把**顶级节点**分到
//!    `mainNav` / `topBarNav` / `bottomNav` / `sidebarNav`；
//!    未知 type（枚举查不到）的顶级节点被丢弃（Java `switch` 无 default）。
//!
//! ## `getCatFloor` 的等价实现要点
//!
//! 1. `ICO_DEFINED_CSS`（`biz_code = "icoDefinedCss"`）→ `icoDefinedCss` 字段
//!    （配置缺失 → 空串，同 Java `getConfigVal` 返回 `""`）。
//! 2. 查 `pc_cat_floor`：`is_show = 1`，`ORDER BY sort_order ASC`。
//! 3. 每个楼层：
//!    - `category_ids` / `category_names` 为 JSON 文本列，按
//!      `JsonUtil.fromJson(..., Integer[]/String[].class)` 解析；
//!    - `catList` 来自 `CategoryService#getCategoryByIds`（`IN (...)`，
//!      **不过滤 `is_show`**，按主键顺序）；
//!    - 分类名：自定义名非空则用自定义名，否则用分类表名
//!      （`translatePackage.translate` 未迁移 → 原文）；
//!    - 仅当分类 `parent_id == 0` 且**本次请求首次**遇到该分类 id 时，
//!      才回填 `children`（可见分类树切片，深度 ≤ 4）；
//!      其余情况 `children` 保持 `null`（不是空数组）。
//!    - `catFloorName` 按 `buildTranslatedCatFloorName` 逐段拼接，段间用「，」。
//!
//! ## 未迁移依赖（如实标注，不伪造实现）
//!
//! 1. **Redis 缓存**：Java 里 `getCatFloor` 先按
//!    `cat_floor[::{x-locale-code}]` 命中缓存（TTL 1 天）再回源，并写入缓存；
//!    本构建服务层未接 Redis，**每次请求直接回源**。结果内容与 Java 冷启动
//!    一致（翻译差异见下条），差异仅在「无缓存加速」与「不会返回 1 天前的旧值」。
//! 2. **多语言翻译**（`TranslatePackage`）：Java 对导航标题、楼层标题、
//!    分类名逐段调用 `translate(...)`；该服务未迁移，本实现按原文返回
//!    （单语言部署下 Java 行为亦是原文）。因此 `x-locale-code` 头在本实现中
//!    无实际影响（Java 仅用它区分缓存键与翻译词条）。
//! 3. **`SeoResponseAdvice`（SEO 响应改写）**：`PcCatFloorServiceImpl`
//!    的缓存清理里会删 `seo_link_category::getCatFloor*`（历史 SEO 链接改写
//!    缓存），但当前 Java 实现（`SeoResponseAdvice#beforeBodyWrite`）是
//!    **直通 no-op**，并不改写响应体；本实现同样不做 SEO 改写。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use nextrade_core::entities::{Category, PcNavigation};
use nextrade_core::models::system::pc::{
    build_cat_floor_name, display_category_name, parse_i32_array, parse_nav_link,
    parse_opt_string_array, pc_navigation_type, CatFloorCategoryTreeNodeVO, CatFloorCategoryVO,
    CatFloorListVO, CatFloorWrapperVO, NavTreeNode, NavVO,
};
use nextrade_core::Result;
use nextrade_infrastructure::db::repo::system::pc::PcRepository;
use nextrade_infrastructure::db::Repositories;

/// `nextrade.is-merchant`（本项目 `application.yaml` = 1）。
///
/// 与 `is-o2o` 一起决定「商家入驻」导航（id = 42）是否排除。
const IS_MERCHANT: bool = true;

/// `nextrade.is-o2o`（本项目 `application.yaml` = 0）。
const IS_O2O: bool = false;

/// 「商家入驻」导航 id（Java `getAllNav` 里硬编码的 42）。
const MERCHANT_ENTRY_NAV_ID: i32 = 42;

/// 导航树最大深度，对应 Java `TreeNodeConfig.setDeep(2)`。
const NAV_MAX_DEPTH: u32 = 2;

/// 分类树最大深度，对应 Java `CategoryServiceImpl` 的
/// `TreeNodeConfig.setDeep(4)`（与 `crates/services/src/category.rs` 一致）。
const CATEGORY_TREE_MAX_DEPTH: u32 = 4;

/// 自定义 ICO 图标库配置项（Java `SettingsEnum.ICO_DEFINED_CSS.bizCode`）。
const ICO_DEFINED_CSS_BIZ_CODE: &str = "icoDefinedCss";

/// Java `getCatFloor` 的缓存键前缀（`Constants.CAT_FLOOR`）。
///
/// 本构建未接 Redis，保留常量用于对照（见模块注释「未迁移依赖」）。
#[allow(dead_code)]
const CAT_FLOOR_CACHE_KEY: &str = "cat_floor";

pub struct PcService {
    repos: Arc<Repositories>,
}

impl PcService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 便捷访问本切片独占仓储。
    fn repo(&self) -> &PcRepository {
        &self.repos.system.pc
    }

    // ——————————————————————————————————————————————
    // GET /api/common/pc/getNav
    // ——————————————————————————————————————————————

    /// 获取前台导航（对应 Java `PCController#getNav` →
    /// `PcNavigationServiceImpl#getAllNav(0)`）。
    ///
    /// `type` 传 `0` → Java 不做类型过滤；树按 `sort_order` 升序构建后
    /// 按顶级节点类型分桶。
    pub async fn get_nav(&self) -> Result<NavVO> {
        // `is-merchant == 0 && is-o2o == 0` 才排除「商家入驻」
        let exclude_id = if !IS_MERCHANT && !IS_O2O {
            Some(MERCHANT_ENTRY_NAV_ID)
        } else {
            None
        };
        let rows = self.repo().find_client_navs(exclude_id).await?;
        Ok(group_nav_forest(build_nav_forest(&rows, NAV_MAX_DEPTH)))
    }

    // ——————————————————————————————————————————————
    // GET /api/common/pc/getCatFloor
    // ——————————————————————————————————————————————

    /// 获取阶梯菜单（分类抽屉），对应 Java `PCController#getCatFloor` →
    /// `PcCatFloorServiceImpl#getCatFloor`。
    ///
    /// 见模块注释：无 Redis 缓存（每次回源）、无多语言翻译（原文）。
    pub async fn get_cat_floor(&self) -> Result<CatFloorWrapperVO> {
        let ico_defined_css = self
            .repo()
            .find_config_val(ICO_DEFINED_CSS_BIZ_CODE)
            .await?
            .unwrap_or_default();

        let floors = self.repo().find_shown_cat_floors().await?;
        if floors.is_empty() {
            return Ok(CatFloorWrapperVO {
                cat_floor: Vec::new(),
                ico_defined_css,
            });
        }

        // 可见分类树（`is_show = 1`，深度 4）——Java 侧由
        // `CategoryTreeCache` 快照承载，这里一次载入供所有父级切片。
        let visible = self.repo().find_visible_categories().await?;
        let forest = build_category_forest(&visible);

        // ⚠️ 与 Java 一致：`queriedParentIds` 定义在**楼层循环之外**，
        // 因此同一父级分类在后续楼层中 **不会**再回填 `children`
        // （Java 原实现即如此，疑似笔误，但为保证响应一致这里照抄）。
        let mut queried_parent_ids: HashSet<i32> = HashSet::new();
        let mut cat_floor = Vec::with_capacity(floors.len());

        for floor in &floors {
            let category_ids = parse_i32_array(&floor.category_ids);
            let category_names = parse_opt_string_array(&floor.category_names);
            let categories = self.repo().find_categories_by_ids(&category_ids).await?;

            let real_name_by_id: HashMap<i32, String> = categories
                .iter()
                .map(|c| (c.category_id, c.category_name.clone()))
                .collect();
            let cat_floor_name =
                build_cat_floor_name(&category_ids, &category_names, &real_name_by_id);

            // Java `categoryNameMap`：`categoryIds[i] -> categoryNames[i]`（可能为 null）
            let custom_by_id: HashMap<i32, Option<String>> = category_ids
                .iter()
                .enumerate()
                .map(|(i, id)| (*id, category_names.get(i).cloned().flatten()))
                .collect();

            let cat_list: Vec<CatFloorCategoryVO> = categories
                .iter()
                .map(|c| {
                    let name = display_category_name(
                        custom_by_id.get(&c.category_id).and_then(|v| v.as_deref()),
                        &c.category_name,
                    );
                    let children = if c.parent_id == 0 && queried_parent_ids.insert(c.category_id) {
                        // Java `getChildrenCategoryTreeById` 在找不到节点时返回空列表
                        Some(
                            find_category_node(&forest, c.category_id)
                                .map(|n| n.children.clone())
                                .unwrap_or_default(),
                        )
                    } else {
                        // 非顶级分类，或该父级已在前面的楼层取过 → Java 保持 null
                        None
                    };
                    CatFloorCategoryVO {
                        id: c.category_id,
                        name: name.clone(),
                        pic: c.category_pic.clone(),
                        category_id: c.category_id,
                        category_name: name,
                        children,
                    }
                })
                .collect();

            cat_floor.push(CatFloorListVO {
                cat_floor_id: floor.cat_floor_id,
                cat_floor_name,
                floor_ico_font: floor.floor_ico_font.clone(),
                cat_list,
            });
        }

        Ok(CatFloorWrapperVO {
            cat_floor,
            ico_defined_css,
        })
    }
}

// ——————————————————————————————————————————————
// 树构建（可单测纯函数，对应 hutool `TreeUtil.build`）
// ——————————————————————————————————————————————

/// 导航树根 id（Java `TreeUtil.build(list, 0, config, parser)` 的 `rootId`）。
const NAV_ROOT_ID: i32 = 0;

/// 构建导航森林（`TreeUtil.build(rows, 0, config{deep}, parser)` 的输出）。
///
/// 步骤与 hutool 一致：
/// 1. 按 `sort_order`（hutool 的 weight）**稳定**升序排序（相等保持 SQL 顺序）；
/// 2. 按 `parent_id` 挂接；`parent_id = 0` 为顶级；父节点不在集合中 → 丢弃；
/// 3. 深度超过 `max_depth` 的层级不再展开（对应 `cutTree`）。
///
/// 与 hutool 的细节对齐：若结果集里存在 `id = 0` 的行，hutool 的
/// `TreeBuilder` 会把它当作**根容器**（`rootId == node.getId()` 时替换 root），
/// 该行本身不出现，`parent_id = 0` 的行改为挂在它下面——输出层数不变，
/// 故这里同样把它当容器处理。
///
/// 节点 `children`：无子节点时省略该键（Java 叶子节点本就没有该键）。
#[must_use]
fn build_nav_forest(rows: &[PcNavigation], max_depth: u32) -> Vec<NavTreeNode> {
    if rows.is_empty() {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..rows.len()).collect();
    // 稳定排序：`sort_by_key` 是稳定排序，等值项保持 SQL 返回顺序
    order.sort_by_key(|&i| rows[i].sort_order);

    let index_by_id: HashMap<i32, usize> =
        rows.iter().enumerate().map(|(i, r)| (r.id, i)).collect();
    let root_index = index_by_id.get(&NAV_ROOT_ID).copied();
    let mut child_indices: Vec<Vec<usize>> = vec![Vec::new(); rows.len()];
    let mut roots: Vec<usize> = Vec::new();
    for &i in &order {
        // `id = 0` 的行是 hutool 的根容器，不参与输出
        if Some(i) == root_index {
            continue;
        }
        let parent_id = rows[i].parent_id;
        if parent_id == NAV_ROOT_ID {
            match root_index {
                Some(container) => child_indices[container].push(i),
                None => roots.push(i),
            }
        } else if let Some(&parent) = index_by_id.get(&parent_id) {
            child_indices[parent].push(i);
        }
    }

    let top_level: &[usize] = match root_index {
        Some(container) => &child_indices[container],
        None => &roots,
    };
    let mut visited: HashSet<usize> = HashSet::new();
    top_level
        .iter()
        .filter_map(|&r| nav_node(rows, &child_indices, r, 1, max_depth, &mut visited))
        .collect()
}

/// 递归构造单个导航节点（环保护 + 深度上限）。
fn nav_node(
    rows: &[PcNavigation],
    child_indices: &[Vec<usize>],
    index: usize,
    depth: u32,
    max_depth: u32,
    visited: &mut HashSet<usize>,
) -> Option<NavTreeNode> {
    if !visited.insert(index) {
        return None;
    }
    let row = &rows[index];
    let children = if depth < max_depth {
        let kids: Vec<NavTreeNode> = child_indices[index]
            .iter()
            .filter_map(|&c| nav_node(rows, child_indices, c, depth + 1, max_depth, visited))
            .collect();
        if kids.is_empty() {
            None
        } else {
            Some(kids)
        }
    } else {
        None
    };
    Some(NavTreeNode {
        id: row.id,
        parent_id: row.parent_id,
        title: row.title.clone(),
        sort_order: row.sort_order,
        r#type: row.r#type,
        is_show: row.is_show,
        is_blank: row.is_blank,
        link: parse_nav_link(&row.link),
        icon: row.icon.clone(),
        children,
    })
}

/// 按顶级节点类型分桶（Java `getAllNav` 的 `switch (pcNavigationType)`）。
#[must_use]
fn group_nav_forest(forest: Vec<NavTreeNode>) -> NavVO {
    let mut vo = NavVO::default();
    for node in forest {
        match node.r#type {
            pc_navigation_type::MAIN => vo.main_nav.push(node),
            pc_navigation_type::TOP_BAR => vo.top_bar_nav.push(node),
            pc_navigation_type::BOTTOM => vo.bottom_nav.push(node),
            pc_navigation_type::SIDEBAR => vo.sidebar_nav.push(node),
            // 枚举查不到 → Java 直接跳过
            _ => {}
        }
    }
    vo
}

/// 构建可见分类森林（`is_show = 1`，深度 ≤ 4）。
///
/// 与 `crates/services/src/category.rs::build_tree` 等价（同一 Java
/// `CategoryServiceImpl.convertTreeList` 配置）。
#[must_use]
fn build_category_forest(rows: &[Category]) -> Vec<CatFloorCategoryTreeNodeVO> {
    build_category_level(rows, 0, 1)
}

/// 递归构建一层分类节点；`depth > CATEGORY_TREE_MAX_DEPTH` 即截断。
fn build_category_level(
    rows: &[Category],
    parent_id: i32,
    depth: u32,
) -> Vec<CatFloorCategoryTreeNodeVO> {
    if depth > CATEGORY_TREE_MAX_DEPTH {
        return Vec::new();
    }
    let mut nodes: Vec<CatFloorCategoryTreeNodeVO> = rows
        .iter()
        .filter(|r| r.parent_id == parent_id)
        .map(|r| CatFloorCategoryTreeNodeVO {
            category_id: r.category_id,
            parent_id: r.parent_id,
            category_name: r.category_name.clone(),
            sort_order: r.sort_order,
            is_show: r.is_show,
            category_pic: r.category_pic.clone(),
            children: build_category_level(rows, r.category_id, depth + 1),
        })
        .collect();
    // hutool 按 weight（= sort_order）稳定升序；SQL 已按
    // `sort_order, category_id` 返回，用同样的 key 使结果确定。
    nodes.sort_by_key(|n| (n.sort_order, n.category_id));
    nodes
}

/// 在分类森林中按 id 查找节点（覆盖全部层级）。
#[must_use]
fn find_category_node<'a>(
    forest: &'a [CatFloorCategoryTreeNodeVO],
    id: i32,
) -> Option<&'a CatFloorCategoryTreeNodeVO> {
    for node in forest {
        if node.category_id == id {
            return Some(node);
        }
        if let Some(found) = find_category_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn nav(id: i32, parent_id: i32, nav_type: i32, sort_order: i32) -> PcNavigation {
        PcNavigation {
            id,
            title: format!("nav{id}"),
            is_show: 1,
            sort_order,
            is_blank: 0,
            link: r#"{"path":"index"}"#.into(),
            r#type: nav_type,
            parent_id,
            icon: String::new(),
        }
    }

    fn cat(
        category_id: i32,
        parent_id: i32,
        sort_order: i32,
        is_show: i32,
        name: &str,
    ) -> Category {
        Category {
            category_id,
            category_name: name.to_string(),
            parent_id,
            sort_order,
            is_show,
            category_pic: format!("pic{category_id}.png"),
            ..Default::default()
        }
    }

    // —————————————— 导航树 ——————————————

    #[test]
    fn nav_forest_builds_two_levels_and_sorts_by_sort_order() {
        // 顶级 2 个（sort_order 2 与 1），子级挂在 id=1 下
        let rows = vec![
            nav(1, 0, 1, 2),
            nav(2, 0, 1, 1),
            nav(3, 1, 1, 1),
            nav(4, 1, 1, 0),
        ];
        let forest = build_nav_forest(&rows, NAV_MAX_DEPTH);
        // 顶级按 sort_order 升序：id=2(1) 在前，id=1(2) 在后
        assert_eq!(forest.len(), 2);
        assert_eq!(forest[0].id, 2);
        assert_eq!(forest[1].id, 1);
        // 子级按 sort_order 升序：id=4(0) 在 id=3(1) 前
        let children = forest[1].children.as_ref().expect("id=1 应有子节点");
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].id, 4);
        assert_eq!(children[1].id, 3);
        // deep=2：第 2 层不再展开（即使它自己还有子节点）
        assert!(children[0].children.is_none());
        // link 已解析成 JSON 对象
        assert_eq!(forest[0].link, json!({"path": "index"}));
    }

    #[test]
    fn nav_forest_keeps_sql_order_for_equal_weight() {
        // 相同 sort_order → 稳定排序保持 SQL 顺序（hutool 稳定排序）
        let rows = vec![nav(9, 0, 1, 5), nav(8, 0, 1, 5), nav(7, 0, 1, 5)];
        let forest = build_nav_forest(&rows, NAV_MAX_DEPTH);
        assert_eq!(
            forest.iter().map(|n| n.id).collect::<Vec<_>>(),
            vec![9, 8, 7]
        );
    }

    #[test]
    fn nav_forest_drops_orphans_like_hutool() {
        // id=5 的父级 404 不存在 → 被丢弃；id=6 的父级 is_show=0 未查出 → 同样丢弃
        let rows = vec![nav(1, 0, 1, 1), nav(5, 404, 1, 2), nav(6, 999, 1, 3)];
        let forest = build_nav_forest(&rows, NAV_MAX_DEPTH);
        assert_eq!(forest.len(), 1);
        assert_eq!(forest[0].id, 1);
        assert!(forest[0].children.is_none());
    }

    #[test]
    fn nav_forest_deeper_levels_are_cut() {
        // 1 -> 2 -> 3 -> 4：deep=2 时只保留 1、2 两层
        let rows = vec![
            nav(1, 0, 1, 1),
            nav(2, 1, 1, 1),
            nav(3, 2, 1, 1),
            nav(4, 3, 1, 1),
        ];
        let forest = build_nav_forest(&rows, NAV_MAX_DEPTH);
        assert_eq!(forest.len(), 1);
        let level2 = forest[0].children.as_ref().unwrap();
        assert_eq!(level2.len(), 1);
        assert_eq!(level2[0].id, 2);
        assert!(level2[0].children.is_none());
    }

    #[test]
    fn nav_forest_empty_input() {
        assert!(build_nav_forest(&[], NAV_MAX_DEPTH).is_empty());
    }

    #[test]
    fn nav_forest_treats_id_zero_row_as_root_container() {
        // hutool `TreeBuilder`：id = rootId(0) 的行替换 root，自身不输出；
        // `parent_id = 0` 的行改为挂在它下面（输出层数不变）
        let rows = vec![nav(0, 0, 1, 0), nav(1, 0, 1, 1), nav(2, 1, 1, 1)];
        let forest = build_nav_forest(&rows, NAV_MAX_DEPTH);
        assert_eq!(forest.len(), 1);
        assert_eq!(forest[0].id, 1);
        assert_eq!(forest[0].children.as_ref().unwrap()[0].id, 2);
    }

    #[test]
    fn group_nav_forest_buckets_by_top_level_type() {
        let rows = vec![
            nav(1, 0, pc_navigation_type::MAIN, 1),
            nav(2, 0, pc_navigation_type::TOP_BAR, 2),
            nav(3, 0, pc_navigation_type::BOTTOM, 3),
            nav(4, 0, pc_navigation_type::SIDEBAR, 4),
            // 未知 type（99）→ 丢弃
            nav(5, 0, 99, 5),
            // 子节点类型不参与分桶（跟随父节点留在 mainNav）
            nav(6, 1, pc_navigation_type::BOTTOM, 1),
        ];
        let vo = group_nav_forest(build_nav_forest(&rows, NAV_MAX_DEPTH));
        assert_eq!(vo.main_nav.len(), 1);
        assert_eq!(vo.top_bar_nav.len(), 1);
        assert_eq!(vo.bottom_nav.len(), 1);
        assert_eq!(vo.sidebar_nav.len(), 1);
        assert_eq!(vo.main_nav[0].id, 1);
        assert_eq!(
            vo.main_nav[0].children.as_ref().unwrap()[0].id,
            6,
            "子节点不重新分桶"
        );
    }

    // —————————————— 分类树 ——————————————

    #[test]
    fn category_forest_is_limited_to_four_levels() {
        let rows = vec![
            cat(1, 0, 1, 1, "一级"),
            cat(2, 1, 1, 1, "二级"),
            cat(3, 2, 1, 1, "三级"),
            cat(4, 3, 1, 1, "四级"),
            cat(5, 4, 1, 1, "五级"),
        ];
        let forest = build_category_forest(&rows);
        assert_eq!(forest.len(), 1);
        let l2 = &forest[0].children; // 二级
        assert_eq!(l2.len(), 1);
        let l3 = &l2[0].children; // 三级
        assert_eq!(l3.len(), 1);
        let l4 = &l3[0].children; // 四级
        assert_eq!(l4.len(), 1);
        assert_eq!(l4[0].category_id, 4, "第 4 层仍存在");
        // 第 5 层被 `deep = 4` 截断
        assert!(l4[0].children.is_empty());
    }

    #[test]
    fn category_forest_sorts_siblings_and_skips_missing_parents() {
        let rows = vec![
            cat(1, 0, 2, 1, "A"),
            cat(2, 0, 1, 1, "B"),
            cat(3, 1, 1, 1, "A1"),
            cat(4, 1, 0, 1, "A0"),
            // 父级 77 不在可见集合中 → 丢弃
            cat(5, 77, 1, 1, "孤儿"),
        ];
        let forest = build_category_forest(&rows);
        assert_eq!(
            forest.iter().map(|n| n.category_id).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(
            forest[1]
                .children
                .iter()
                .map(|n| n.category_id)
                .collect::<Vec<_>>(),
            vec![4, 3]
        );
        assert_eq!(forest[1].children[0].category_pic, "pic4.png");
    }

    #[test]
    fn find_category_node_searches_all_levels() {
        let rows = vec![cat(1, 0, 1, 1, "A"), cat(2, 1, 1, 1, "A1")];
        let forest = build_category_forest(&rows);
        assert_eq!(
            find_category_node(&forest, 2).map(|n| n.category_name.clone()),
            Some("A1".to_string())
        );
        assert!(find_category_node(&forest, 999).is_none());
    }

    #[test]
    fn cat_floor_endpoint_constants_match_java() {
        // 本项目 application.yaml：is-merchant: 1 / is-o2o: 0
        assert!(IS_MERCHANT);
        assert!(!IS_O2O);
        // 42 只在 is-merchant == 0 && is-o2o == 0 时排除
        let exclude = if !IS_MERCHANT && !IS_O2O {
            Some(MERCHANT_ENTRY_NAV_ID)
        } else {
            None
        };
        assert_eq!(exclude, None);
        assert_eq!(MERCHANT_ENTRY_NAV_ID, 42);
        assert_eq!(NAV_MAX_DEPTH, 2);
        assert_eq!(CATEGORY_TREE_MAX_DEPTH, 4);
        assert_eq!(ICO_DEFINED_CSS_BIZ_CODE, "icoDefinedCss");
        assert_eq!(CAT_FLOOR_CACHE_KEY, "cat_floor");
    }
}