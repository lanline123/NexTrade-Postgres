//! 店铺分类 域服务（merchant::shop_client_category）。
//!
//! 对应原 Java `ShopProductCategoryServiceImpl`（本片只涉及前台两个只读方法）：
//! - `tree(Integer shopId)` → [`ShopClientCategoryService::category_tree`]
//! - `parentTree(Integer id)` → [`ShopClientCategoryService::parent_tree`]
//!
//! 管理端 CRUD（`create` / `update` / `del` / `batch` / `updateField` /
//! `list` / `detail`）不在本片契约内，未迁移。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::ShopProductCategory;
use nextrade_core::models::merchant::shop::{ShopCategoryTreeNode, ShopParentTreeVO};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 原 Java `messageHelper.get("error.shop.select.required")` = 「请选择店铺」。
const ERR_SHOP_REQUIRED: &str = "请选择店铺";

/// `parentTree` 的循环上限。
///
/// 原 Java：`if (result.size() > 5) break;` 在**入栈之后**判断，
/// 故链路最多保留 **6** 个节点（自底向上入栈，随后反转）。
const PARENT_CHAIN_MAX: usize = 6;

/// 店铺分类 域服务。
pub struct ShopClientCategoryService {
    repos: Arc<Repositories>,
}

impl ShopClientCategoryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::merchant::shop_client_category::ShopClientCategoryRepository {
        &self.repos.merchant.shop_client_category
    }

    /// `GET /api/shop/category/tree?shopId=`：店铺经营类目树。
    ///
    /// 对应 Java `ShopProductCategoryServiceImpl#tree(Integer shopId)`：
    /// 1. `shopId == null` → 抛 `error.shop.select.required`（「请选择店铺」）。
    ///    Rust 侧 `shopId` 为必填 `i32`，缺失时由 axum 直接 400；此处对
    ///    `<= 0` 同样报该错（与 `services/shop.rs::category_tree` 口径一致）。
    /// 2. 查 `shop_id = ? AND is_show = 1`，按 `parent_id, sort_order,
    ///    category_id` 升序。
    /// 3. `convertTreeList(cats, 0)`：以 `parent_id = 0` 为根递归建树，
    ///    节点键对齐 hutool `Tree`（`categoryId/parentId/categoryName/
    ///    categoryPic/isStore=1/children`）。
    ///
    /// 与原 Java 的差异：`convertTreeList` 在结果为空时返回 `null`，
    /// 本实现返回空数组（与 `services/shop.rs::category_tree` 一致，
    /// 前端对 `null` 与 `[]` 通常等价处理）。
    pub async fn category_tree(&self, shop_id: i32) -> Result<Vec<ShopCategoryTreeNode>> {
        if shop_id <= 0 {
            return Err(Error::Validation(ERR_SHOP_REQUIRED.into()));
        }
        let cats = self.repo().find_categories_for_tree(shop_id).await?;
        Ok(build_category_tree(&cats, 0))
    }

    /// `GET /api/shop/category/parentTree?id=`：某分类的父级链（含同层兄弟）。
    ///
    /// 对应 Java `ShopProductCategoryServiceImpl#parentTree(Integer id)`：
    /// 1. 以 `id` 为起点逐级 `getById` 上溯（**不过滤 `is_show`**），
    ///    最多 6 级；中途查不到即终止。
    /// 2. 反转 → **根在前、叶子在后**。
    /// 3. 对链上每一级，按其 `parent_id` 查同层分类（`is_show = 1`，
    ///    仅按 `sort_order` 升序）作为该级 `catList`。
    ///
    /// `id <= 0` 时原 Java 的 `while (parentId > 0)` 直接跳过，返回空列表。
    pub async fn parent_tree(&self, id: i32) -> Result<Vec<ShopParentTreeVO>> {
        if id <= 0 {
            return Ok(Vec::new());
        }

        // 自底向上收集祖先链（叶子在前），与 Java 的 while 循环逐行对应。
        let mut chain: Vec<ShopProductCategory> = Vec::new();
        let mut cursor = id;
        while cursor > 0 {
            match self.repo().find_by_id(cursor).await? {
                Some(node) => {
                    cursor = node.parent_id;
                    chain.push(node);
                }
                // getById 返回 null → parentId 置 0，循环结束
                None => break,
            }
            if chain_full(chain.len()) {
                break;
            }
        }
        if chain.is_empty() {
            return Ok(Vec::new());
        }

        // 每个 `parent_id` 只查一次同层列表（原 Java 逐项查询且不去重）。
        let mut siblings: HashMap<i32, Vec<ShopParentTreeVO>> = HashMap::new();
        for node in &chain {
            if siblings.contains_key(&node.parent_id) {
                continue;
            }
            let rows = self.repo().find_siblings(node.parent_id).await?;
            siblings.insert(node.parent_id, rows.iter().map(flat_parent_vo).collect());
        }

        Ok(build_parent_tree(&chain, &siblings))
    }
}

/// 递归构建分类树（对齐 hutool `TreeUtil.build`，根为 `parent_id = 0`）。
///
/// 入参 `cats` 已按 `parent_id, sort_order, category_id` 升序，
/// 因此同层节点保持库序，无需二次排序。
fn build_category_tree(cats: &[ShopProductCategory], parent: i32) -> Vec<ShopCategoryTreeNode> {
    cats.iter()
        .filter(|c| c.parent_id == parent)
        .map(|c| ShopCategoryTreeNode {
            category_id: c.category_id,
            parent_id: c.parent_id,
            category_name: c.category_name.clone(),
            category_pic: c.category_pic.clone(),
            // 原 Java `putExtra("isStore", 1)` 恒为 1
            is_store: 1,
            children: build_category_tree(cats, c.category_id),
        })
        .collect()
}

/// 父级链是否已达上限（Java `if (result.size() > 5) break;`）。
#[must_use]
fn chain_full(len: usize) -> bool {
    len >= PARENT_CHAIN_MAX
}

/// 扁平父级 VO（无 `catList`，用于 `catList` 内元素）。
fn flat_parent_vo(c: &ShopProductCategory) -> ShopParentTreeVO {
    ShopParentTreeVO {
        category_id: c.category_id,
        parent_id: c.parent_id,
        category_name: c.category_name.clone(),
        cat_list: None,
    }
}

/// 组装父级链：`chain` 为自底向上（叶子在前）的祖先序列，
/// 输出反转为**根在前**；每级 `catList` 取同层节点列表。
///
/// 原 Java 对链上每一级都执行了同层查询并把结果（可能为空列表）
/// 写入 `catList`，故此处用 `unwrap_or_default()` 保证恒为 `Some(...)`。
fn build_parent_tree(
    chain: &[ShopProductCategory],
    siblings: &HashMap<i32, Vec<ShopParentTreeVO>>,
) -> Vec<ShopParentTreeVO> {
    let mut out: Vec<ShopParentTreeVO> = chain
        .iter()
        .map(|c| ShopParentTreeVO {
            category_id: c.category_id,
            parent_id: c.parent_id,
            category_name: c.category_name.clone(),
            cat_list: Some(siblings.get(&c.parent_id).cloned().unwrap_or_default()),
        })
        .collect();
    out.reverse();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一条分类记录（列值可空性对齐 schema）。
    fn cat(id: i32, parent: i32, name: &str, sort: i32) -> ShopProductCategory {
        ShopProductCategory {
            category_id: id,
            category_name: name.to_string(),
            shop_id: 7,
            parent_id: parent,
            sort_order: sort,
            is_show: 1,
            add_time: Some(0),
            category_pic: String::new(),
        }
    }

    // ————————————————————— tree —————————————————————

    #[test]
    fn tree_nests_direct_children() {
        // 根 1 → 子 2、3；子 3 下再挂 4
        let cats = vec![
            cat(1, 0, "酒水", 1),
            cat(2, 1, "白酒", 1),
            cat(3, 1, "红酒", 2),
            cat(4, 3, "干红", 1),
        ];
        let tree = build_category_tree(&cats, 0);
        assert_eq!(tree.len(), 1);
        let root = &tree[0];
        assert_eq!(root.category_id, 1);
        assert_eq!(root.is_store, 1);
        assert_eq!(root.children.len(), 2);
        // 同层保持入参（= 库序）顺序：sort 1 在前、2 在后
        assert_eq!(root.children[0].category_id, 2);
        assert_eq!(root.children[1].category_id, 3);
        // 三级嵌套
        assert_eq!(root.children[1].children.len(), 1);
        assert_eq!(root.children[1].children[0].category_id, 4);
        // 叶子无子节点
        assert!(root.children[0].children.is_empty());
    }

    #[test]
    fn tree_skips_nodes_under_other_parents() {
        // 只从 parent_id = 0 出发；挂在不存在父级下的孤儿不会成为根
        let cats = vec![cat(1, 0, "根", 1), cat(9, 42, "孤儿", 1)];
        let tree = build_category_tree(&cats, 0);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].category_id, 1);
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn tree_from_specific_subtree() {
        // `build_category_tree(cats, parent)` 取 `parent_id == parent` 的节点为根。
        // 节点 2 的 parent 是 1 ⇒ 传 1 时根为 2，其子为 3。
        let cats = vec![
            cat(1, 0, "酒水", 1),
            cat(2, 1, "白酒", 1),
            cat(3, 2, "酱香", 1),
        ];
        let tree = build_category_tree(&cats, 1);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].category_id, 2);
        assert_eq!(tree[0].children[0].category_id, 3);
    }

    #[test]
    fn tree_empty_input_yields_empty() {
        assert!(build_category_tree(&[], 0).is_empty());
    }

    // ————————————————————— parentTree —————————————————————

    #[test]
    fn chain_full_matches_java_break_boundary() {
        // Java `if (result.size() > 5) break;`：size 达 6（>5）才中断，
        // 故最多保留 6 个节点。
        assert!(!chain_full(0));
        assert!(!chain_full(1));
        assert!(!chain_full(5));
        assert!(chain_full(PARENT_CHAIN_MAX));
        assert!(chain_full(7));
    }

    #[test]
    fn parent_tree_is_root_first_and_attaches_siblings() {
        // 库序（叶子在前，模拟上溯收集结果）：4(父3) → 3(父1) → 1(父0)
        let chain = vec![
            cat(4, 3, "干红", 1),
            cat(3, 1, "红酒", 2),
            cat(1, 0, "酒水", 1),
        ];
        let mut siblings: HashMap<i32, Vec<ShopParentTreeVO>> = HashMap::new();
        // parent_id = 3 的同层
        siblings.insert(3, vec![flat_parent_vo(&cat(4, 3, "干红", 1))]);
        // parent_id = 1 的同层（自身 + 兄弟）
        siblings.insert(
            1,
            vec![
                flat_parent_vo(&cat(2, 1, "白酒", 1)),
                flat_parent_vo(&cat(3, 1, "红酒", 2)),
            ],
        );
        // parent_id = 0 的同层（根节点集合）
        siblings.insert(
            0,
            vec![
                flat_parent_vo(&cat(1, 0, "酒水", 1)),
                flat_parent_vo(&cat(5, 0, "食品", 2)),
            ],
        );

        let out = build_parent_tree(&chain, &siblings);
        // 反转为根在前
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].category_id, 1);
        assert_eq!(out[1].category_id, 3);
        assert_eq!(out[2].category_id, 4);
        // catList 与各自 parent_id 对应
        assert_eq!(out[0].parent_id, 0);
        assert_eq!(out[0].cat_list.as_ref().unwrap().len(), 2);
        assert_eq!(out[1].parent_id, 1);
        assert_eq!(out[1].cat_list.as_ref().unwrap().len(), 2);
        assert_eq!(out[2].parent_id, 3);
        let leaf_siblings = out[2].cat_list.as_ref().unwrap();
        assert_eq!(leaf_siblings.len(), 1);
        assert_eq!(leaf_siblings[0].category_id, 4);
        // catList 内元素自身不带 catList
        assert!(leaf_siblings[0].cat_list.is_none());
    }

    #[test]
    fn parent_tree_missing_siblings_map_yields_empty_list() {
        let chain = vec![cat(1, 0, "酒水", 1)];
        let out = build_parent_tree(&chain, &HashMap::new());
        assert_eq!(out.len(), 1);
        // 原 Java 恒设置 catList（可能为空列表），故为 Some(空)
        assert_eq!(out[0].cat_list.as_ref().map(Vec::len), Some(0));
    }

    #[test]
    fn parent_tree_empty_chain_yields_empty() {
        assert!(build_parent_tree(&[], &HashMap::new()).is_empty());
    }
}