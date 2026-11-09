//! PC 导航栏服务（content::pc_navigation）。
//!
//! 对应原 Java `com.nextrade.service.decorate.impl.PcNavigationServiceImpl` 中由
//! `PcNavigationController`（`/adminapi/decorate/pcNavigation`）暴露的 9 条链路，
//! 以及其父类 `BaseServiceImpl` 的 `del` / `updateField` / `batch` 通用实现。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`PcNavigationService::list`] | `PcNavigationServiceImpl#list` |
//! | [`PcNavigationService::detail`] | `PcNavigationServiceImpl#detail` |
//! | [`PcNavigationService::create`] | `PcNavigationServiceImpl#create` + `#getLink` |
//! | [`PcNavigationService::update`] | `PcNavigationServiceImpl#update` + `#getLink` |
//! | [`PcNavigationService::delete`] | `BaseServiceImpl#del` |
//! | [`PcNavigationService::update_field`] | `BaseServiceImpl#updateField(updateField, PC_NAVIGATION_FIELDS)` |
//! | [`PcNavigationService::batch`] | `BaseServiceImpl#batch` |
//! | [`PcNavigationService::select_link`] | `PcNavigationServiceImpl#selectLink` |
//! | [`PcNavigationService::parent_nav`] | `PcNavigationServiceImpl#getParentNav` |
//!
//! ## 与 Java 的差异（逐条）
//!
//! 1. **翻译服务未迁移**：Java `getParentNav` 用 `translatePackage.translate(title)`
//!    做多语言标题替换（依赖 Redis + 翻译表）。Rust 侧**直接返回原始
//!    `title`**，不做翻译。多语言站点下返回的是库中原文，而非目标语言文案。
//! 2. **配置服务未迁移**：Java 用 `nextradeProperties.getIsMerchant()` /
//!    `getIsO2o()` 决定「是否商户端」（影响 `selectLink` 是否含「店铺列表」、
//!    `getParentNav` 是否排除 id=42「商家入驻」）。Rust 侧改读环境变量：
//!    `TIG_IS_MERCHANT`（默认 **1**，与 `application.yaml` 的 `is-merchant: 1`
//!    一致）、`TIG_IS_O2O`（默认 0）。**未配置时按默认值走**，与发客户版一致。
//! 3. **DTO 校验注解**：Java 控制器**没有** `@Validated`，因此
//!    `@NotBlank` / `@Size` / `@NotNull` 实际不生效。本实现按注解语义在
//!    `create` 校验 `title`、在 `updateField` 校验 `val` 非空（更严格），
//!    其余「注解未声明但非法」的入参（如 `detail` 的 id）与原实现一致地报错。
//! 4. **`updateField` 白名单**：Java `CheckFieldConstants.PC_NAVIGATION_FIELDS`
//!    为 `{"example_name", "is_show", "is_blank", "sort_order"}`，其中
//!    `example_name` 在 `pc_navigation` 表不存在（复制粘贴笔误，命中时 Java
//!    会在 SQL 层抛「未知列」）。本实现把该字段直接判为非法字段，
//!    返回与 Java 同一个业务提示「field 的值有误」(code 1001)。
//!    注意 `title` **不在**白名单内 —— 与 Java 一致，改标题要走 `update`。
//! 5. **`batch` 非 `del` 类型**：Java `BaseServiceImpl#batch` 对非 `del`
//!    直接 `return false`（不报错），本实现保持一致。
//! 6. **`del` 缺 id**：Java `BaseServiceImpl#del(null)` 返回 `false`（不报错），
//!    本实现保持一致。
//! 7. **`update` 未传 `link` 会写 `[]`**：Java `getLink` 在 `link == null` 时
//!    置 `"[]"`，且 MyBatis-Plus 只跳过 `null` 字段、不跳过 `"[]"`，
//!    因此**未传 link 的更新会把链接清空为 `[]`**。本实现照抄该行为
//!    （见 [`normalize_link`]），前端全量提交时无差异。
//! 8. **`detail` 查不到记录**：Java 对 `null` 实体做 `BeanUtils.copyProperties`
//!    会抛 `IllegalArgumentException` → 500；本实现返回 `Error::NotFound`(404)。
//! 9. **`hasChildren`**：Java `list` 恒置 0（仅 `type == 2` 回填真实计数），
//!    `detail` 不设置该字段（JSON `null`）；本实现用 `Option<i64>` 精确复刻。
//!    计数由 N+1 合并为一次 `GROUP BY parent_id`，结果等价。

use std::sync::Arc;

use nextrade_core::entities::PcNavigation;
use nextrade_core::models::content::pc_navigation::{
    parse_link_any, NavSelectLinkVO, PcNavigationCreateDTO, PcNavigationListDTO, PcNavigationPatch,
    PcNavigationTreeVO, PcNavigationUpdateDTO, PcNavigationVO, TYPE_BOTTOM, TYPE_TOP_BAR,
};
use nextrade_core::models::{BatchDTO, OperateDTO, UpdateFieldDTO};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `ResultTextConstants.INVALID_FIELD_VALUE`。
pub const INVALID_FIELD_VALUE: &str = "field 的值有误";
/// Java `ResultTextConstants.ID_CANNOT_IS_NULL`。
pub const ID_CANNOT_IS_NULL: &str = "id不能为空";
/// Java `ResultTextConstants.FIELD_VALUE_CANNOT_BE_NULL`。
pub const FIELD_VALUE_CANNOT_BE_NULL: &str = "值不能为空";
/// Java `PcNavigationConstants.TITLE_NOT_NULL`。
pub const TITLE_NOT_NULL: &str = "首页分类栏名称不能为空";
/// Java `PcNavigationConstants.TITLE_OVER_LENGTH`。
pub const TITLE_OVER_LENGTH: &str = "首页分类栏名称长度不能超过50";
/// Java `PcNavigationServiceImpl#getLink` 的异常文案。
pub const BOTTOM_NAV_ARTICLE_ONLY: &str = "底部导航只能指向文章";

/// 标题最大长度（Java `PcNavigationCreateDTO` `@Size(max = 50)`）。
pub const NAV_TITLE_MAX_LEN: usize = 50;

/// `link` 缺省值（Java `getLink`：`link == null` → `"[]"`）。
pub const EMPTY_LINK: &str = "[]";

/// 「商家入驻」导航 ID（Java 注释：42 是商家入驻，O2O 模式下也支持入驻功能）。
pub const MERCHANT_NAV_ID: i32 = 42;

/// `updateField` 允许更新的列（Java `PC_NAVIGATION_FIELDS` 的**有效**子集）。
///
/// 与 `infrastructure::db::repo::nextrade_core::models::content::pc_navigation::PC_NAVIGATION_FIELDS` 同源，务必一致。
pub const PC_NAVIGATION_FIELDS: [&str; 3] = ["is_show", "is_blank", "sort_order"];

/// 把 Java 属性名 / 列名解析为 `pc_navigation` 的白名单列名。
///
/// 对应 Java `BaseServiceImpl#getColumnByPropertyName` + 白名单校验：
/// 未知属性在 Java 会回落主键 `id`，而 `id` 不在白名单内 → 抛
/// `GlobalException(INVALID_FIELD_VALUE)`。本实现直接返回同一错误，
/// 不暴露任何用户输入。
pub fn resolve_update_column(field: Option<&str>) -> Result<&'static str> {
    let field = field.map(str::trim).unwrap_or("");
    let column = match field {
        "isShow" | "is_show" => "is_show",
        "isBlank" | "is_blank" => "is_blank",
        "sortOrder" | "sort_order" => "sort_order",
        // 含 Java 白名单里的 `example_name`：该列在 `pc_navigation` 不存在（笔误），
        // 以及 Java 未列入白名单的 `title` / `type` / `parent_id` / `icon` / `link`。
        _ => return Err(Error::biz(SERVICE_DATA_ERROR, INVALID_FIELD_VALUE)),
    };
    Ok(column)
}

/// 校验 `create` 的标题，返回待落库的原始字符串（Java 不做 trim）。
///
/// Java `PcNavigationCreateDTO`：`@NotBlank` + `@Size(max = 50)`。
/// 长度按**字符数**计（Java `@Size` 按 UTF-16 码元数，中文等价）。
pub fn validate_create_title(title: Option<&str>) -> Result<&str> {
    let raw = title.ok_or_else(|| Error::Validation(TITLE_NOT_NULL.to_string()))?;
    if raw.trim().is_empty() {
        return Err(Error::Validation(TITLE_NOT_NULL.to_string()));
    }
    if raw.chars().count() > NAV_TITLE_MAX_LEN {
        return Err(Error::Validation(TITLE_OVER_LENGTH.to_string()));
    }
    Ok(raw)
}

/// 生成待落库的 `link` JSON 文本，对应 Java `PcNavigationServiceImpl#getLink`。
///
/// - `link` 缺失 / `null` → `"[]"`；
/// - `type == 3`（底部导航）且 `path` 非空白且不等于 `article` →
///   `GlobalException("底部导航只能指向文章")`（code 1001）；
/// - 其余情况原样序列化（Java `JSONUtil.toJsonStr(link)`）。
///
/// 差异：Java 先 `JSONUtil.parseObj(link)` 再取 `path`，对「非对象」入参
/// （如 JSON 数组）行为未定义（hutool 会包一层 Map 或抛异常）；本实现只在
/// 入参为 JSON 对象时取 `path`，非对象直接原样存储。
pub fn normalize_link(nav_type: Option<i32>, link: Option<&serde_json::Value>) -> Result<String> {
    let Some(value) = link.filter(|v| !v.is_null()) else {
        return Ok(EMPTY_LINK.to_string());
    };
    if nav_type == Some(TYPE_BOTTOM) {
        if let Some(path) = value.get("path").and_then(serde_json::Value::as_str) {
            // Java `StrUtil.isNotBlank(path) && !"article".equals(path)`
            if !path.trim().is_empty() && path != "article" {
                return Err(Error::biz(SERVICE_DATA_ERROR, BOTTOM_NAV_ARTICLE_ONLY));
            }
        }
    }
    serde_json::to_string(value).map_err(|e| Error::Validation(e.to_string()))
}

/// `selectLink` 的静态清单，对应 Java `PcNavigationServiceImpl#selectLink`。
///
/// `is_merchant` 为真时在「积分商城」之后追加「店铺列表」一项
/// （Java `nextradeProperties.getIsMerchant() == 1`）。
#[must_use]
pub fn select_link_entries(is_merchant: bool) -> Vec<NavSelectLinkVO> {
    // (name, link)；Java 中每项的 app_link 恒为空串。
    let base: [(&str, &str); 25] = [
        ("商城首页", "/"),
        ("分类页面（仅分类）", "list"),
        ("限时秒杀", "seckill/list"),
        ("购物车", "cart"),
        ("搜索页面", "search"),
        ("会员首页", "member"),
        ("储值中心", "user/account/index"),
        ("我的优惠券", "member/coupon/list"),
        ("我的足迹", "member/browsingHistory/list"),
        ("我的订单", "member/order/list"),
        ("待评价订单", "member/comment/list"),
        ("收货地址", "member/address/list"),
        ("退换货", "member/return/list"),
        ("账户余额", "member/account/detail"),
        ("我的积分", "member/point/list"),
        ("收藏商品", "member/collectProduct/list"),
        ("留言咨询", "member/feedback/list"),
        ("站内消息", "member/userMessage/list"),
        ("发票管理", "member/orderInvoice/list"),
        ("账号信息", "member/profile/info"),
        ("积分商城", "exchange/list"),
        ("拼团", "joining/list"),
        ("买家秀", "buyerShow"),
        ("礼品卡兑换", "giftCard/exchange"),
        ("抽奖大转盘", "lottery/index"),
    ];
    let mut out: Vec<NavSelectLinkVO> = base
        .iter()
        .map(|(name, link)| NavSelectLinkVO {
            name: (*name).to_string(),
            app_link: String::new(),
            link: (*link).to_string(),
        })
        .collect();
    if is_merchant {
        // Java：插在「积分商城」之后、「拼团」之前（index 21）。
        out.insert(
            21,
            NavSelectLinkVO {
                name: "店铺列表".to_string(),
                app_link: String::new(),
                link: "shop/list".to_string(),
            },
        );
    }
    out
}

/// 把导航列表组装为「两层 + 扁平」的树（Java `convertTreeList` + `flattenTree`）。
///
/// hutool `TreeUtil.build(list, 0, config{deep = 2})` 的行为：
/// 根为 `parent_id == 0`；只有能挂到根上的节点（深度 2）保留；
/// 父级不在结果集中的孤立节点与更深层节点**被丢弃**。
/// `flattenTree` 随后清空孙级（deep = 2 时本就为空）。
#[must_use]
pub fn build_parent_tree(rows: &[PcNavigation]) -> Vec<PcNavigationTreeVO> {
    let node = |n: &PcNavigation| PcNavigationTreeVO {
        id: n.id,
        parent_id: n.parent_id,
        title: n.title.clone(),
        sort_order: n.sort_order,
        r#type: n.r#type,
        is_show: n.is_show,
        is_blank: n.is_blank,
        link: parse_link_any(&n.link),
        icon: n.icon.clone(),
        children: Vec::new(),
    };

    let mut roots: Vec<PcNavigationTreeVO> = rows
        .iter()
        .filter(|n| n.parent_id == 0)
        .map(&node)
        .collect();
    for row in rows.iter().filter(|n| n.parent_id != 0) {
        if let Some(parent) = roots.iter_mut().find(|p| p.id == row.parent_id) {
            parent.children.push(node(row));
        }
    }
    roots
}

/// 读取「是否商户端」（Java `nextradeProperties.isMerchant`；配置服务未迁移）。
///
/// 未设置时按 `application.yaml` 的 `is-merchant: 1` 视为**开启**。
fn is_merchant_from_env() -> bool {
    std::env::var("TIG_IS_MERCHANT").map_or(true, |v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// 读取「是否多门店 O2O」（Java `nextradeProperties.isO2o`；默认 0）。
fn is_o2o_from_env() -> bool {
    std::env::var("TIG_IS_O2O").map_or(false, |v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// PC 导航栏服务。
pub struct PcNavigationService {
    repos: Arc<Repositories>,
}

impl PcNavigationService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// PC 导航分页，对应 Java `PcNavigationServiceImpl#list`。
    ///
    /// - `type` 合法（1..=4）才过滤；`type == 2` 且 `parentId >= 0` 时按父级过滤；
    /// - `keyword` 非空则 `title LIKE %kw%`；
    /// - `hasChildren`：恒 0，仅 `type == 2` 时回填真实子级数。
    pub async fn list(&self, q: &PcNavigationListDTO) -> Result<PageResult<PcNavigationVO>> {
        let page = q.page_query();
        let type_filter = q.valid_type();
        let parent_filter = q.parent_filter();
        let keyword = q
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty());

        let (total, rows) = self
            .repos.content.pc_navigation
            .paginate(
                type_filter,
                parent_filter,
                keyword,
                q.sort_field.as_deref(),
                q.sort_order.as_deref(),
                &page,
            )
            .await?;

        let counts = if type_filter == Some(TYPE_TOP_BAR) {
            let ids: Vec<i32> = rows.iter().map(|r| r.id).collect();
            self.repos.content.pc_navigation.count_children(&ids).await?
        } else {
            Vec::new()
        };

        let list: Vec<PcNavigationVO> = rows
            .iter()
            .map(|r| {
                let mut vo = PcNavigationVO::from(r);
                // Java：list 恒显式置 0
                vo.has_children = Some(0);
                if type_filter == Some(TYPE_TOP_BAR) {
                    vo.has_children = Some(
                        counts
                            .iter()
                            .find(|(pid, _)| *pid == r.id)
                            .map_or(0, |(_, c)| *c),
                    );
                }
                vo
            })
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    /// PC 导航详情，对应 Java `PcNavigationServiceImpl#detail`。
    ///
    /// Java 对不存在的 id 会抛 `IllegalArgumentException`（500）；
    /// 本实现返回 404（见模块头差异第 8 条）。
    /// Java 的 `detail` **不设置** `hasChildren`（JSON `null`）。
    pub async fn detail(&self, id: i32) -> Result<PcNavigationVO> {
        self.repos.content.pc_navigation
            .find_by_id(id)
            .await?
            .map(|row| PcNavigationVO::from(&row))
            .ok_or_else(|| Error::NotFound("导航数据不存在".into()))
    }

    /// 新建 PC 导航，对应 Java `PcNavigationServiceImpl#create`。
    ///
    /// 返回是否写入成功（Java 控制器返回 `void`）。
    pub async fn create(&self, dto: &PcNavigationCreateDTO) -> Result<bool> {
        let title = validate_create_title(dto.title.as_deref())?;
        let link = normalize_link(dto.r#type, dto.link.as_ref())?;
        let nav = PcNavigation {
            // 自增主键，插入时忽略
            id: 0,
            title: title.to_string(),
            // Java：DTO 未提供 → MyBatis-Plus 不写入该列 → DDL 默认值
            // (`is_show` / `sort_order` / `is_blank` / `type` / `parent_id` 默认 0，
            //  `icon` 默认 '')，与此处取值等价。
            is_show: dto.is_show.unwrap_or(0),
            sort_order: dto.sort_order.unwrap_or(0),
            is_blank: dto.is_blank.unwrap_or(0),
            link,
            r#type: dto.r#type.unwrap_or(0),
            parent_id: dto.parent_id.unwrap_or(0),
            icon: dto.icon.clone().unwrap_or_default(),
        };
        let id = self.repos.content.pc_navigation.insert(&nav).await?;
        tracing::info!(
            "创建 PC 导航成功: id={id}, title={title}, type={}",
            nav.r#type
        );
        Ok(id > 0)
    }

    /// 更新 PC 导航，对应 Java `PcNavigationServiceImpl#update`。
    ///
    /// ⚠️ 与 Java 一致：**未传 `link` 会写入 `"[]"`**（见模块头差异第 7 条）。
    /// 返回是否有行被更新（Java `updateById` 同理）。
    pub async fn update(&self, dto: &PcNavigationUpdateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(ID_CANNOT_IS_NULL.to_string()))?;
        let link = normalize_link(dto.r#type, dto.link.as_ref())?;
        let patch = PcNavigationPatch {
            id,
            // Java `BeanUtils.copyProperties` 只复制类型兼容的属性；`link` 为
            // `Object` → `String` 不兼容，因此链接完全由 `getLink` 决定。
            title: dto.title.clone(),
            is_show: dto.is_show,
            sort_order: dto.sort_order,
            is_blank: dto.is_blank,
            link,
            r#type: dto.r#type,
            parent_id: dto.parent_id,
            icon: dto.icon.clone(),
        };
        self.repos.content.pc_navigation.update(&patch).await
    }

    /// 删除 PC 导航，对应 Java `BaseServiceImpl#del`。
    ///
    /// Java：`id == null` 时直接返回 `false`（不报错）；本实现保持一致。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<bool> {
        match dto.id {
            None => Ok(false),
            Some(id) => self.repos.content.pc_navigation.delete(id).await,
        }
    }

    /// 通用单字段更新，对应 Java `BaseServiceImpl#updateField`。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(ID_CANNOT_IS_NULL.to_string()))?;
        let column = resolve_update_column(dto.field.as_deref())?;
        let val = dto
            .val
            .as_deref()
            .ok_or_else(|| Error::Validation(FIELD_VALUE_CANNOT_BE_NULL.to_string()))?;
        self.repos.content.pc_navigation.update_field(id, column, val).await
    }

    /// 批量操作，对应 Java `BaseServiceImpl#batch`。
    ///
    /// 仅 `type == "del"` 执行批量删除；其它类型 Java 返回 `false`（不报错）。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type.as_deref() != Some("del") {
            return Ok(false);
        }
        let ids = dto.ids.clone().unwrap_or_default();
        if ids.is_empty() {
            // Java `removeByIds(空集合)` 无法生成合法 `IN ()`，语义上等同「未删除」
            return Ok(false);
        }
        self.repos.content.pc_navigation.delete_many(&ids).await
    }

    /// 可选链接地址，对应 Java `PcNavigationServiceImpl#selectLink`。
    #[must_use]
    pub fn select_link(&self) -> Vec<NavSelectLinkVO> {
        select_link_entries(is_merchant_from_env())
    }

    /// 上级导航树，对应 Java `PcNavigationServiceImpl#getParentNav(type, 0)`。
    pub async fn parent_nav(&self, nav_type: i32) -> Result<Vec<PcNavigationTreeVO>> {
        let type_filter = if nav_type > 0 { Some(nav_type) } else { None };
        let exclude_id = if !is_merchant_from_env() && !is_o2o_from_env() {
            Some(MERCHANT_NAV_ID)
        } else {
            None
        };
        let rows = self
            .repos.content.pc_navigation
            .find_for_parent_tree(type_filter, exclude_id)
            .await?;
        Ok(build_parent_tree(&rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn nav(id: i32, parent_id: i32, title: &str, sort_order: i32, link: &str) -> PcNavigation {
        PcNavigation {
            id,
            title: title.to_string(),
            is_show: 1,
            sort_order,
            is_blank: 0,
            link: link.to_string(),
            r#type: 2,
            parent_id,
            icon: String::new(),
        }
    }

    #[test]
    fn update_field_resolution_matches_java_whitelist() {
        assert_eq!(resolve_update_column(Some("isShow")).unwrap(), "is_show");
        assert_eq!(resolve_update_column(Some("is_show")).unwrap(), "is_show");
        assert_eq!(resolve_update_column(Some("isBlank")).unwrap(), "is_blank");
        assert_eq!(
            resolve_update_column(Some("sortOrder")).unwrap(),
            "sort_order"
        );
        assert_eq!(
            resolve_update_column(Some("sort_order")).unwrap(),
            "sort_order"
        );

        // 白名单外（含 Java 笔误的 example_name、以及 title/type/...）
        for bad in [
            None,
            Some(""),
            Some("   "),
            Some("title"),
            Some("example_name"),
            Some("type"),
        ] {
            let err = resolve_update_column(bad).unwrap_err();
            assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
            assert!(err.to_string().contains(INVALID_FIELD_VALUE));
        }
    }

    #[test]
    fn whitelist_constant_covers_every_resolvable_column() {
        assert_eq!(PC_NAVIGATION_FIELDS.len(), 3);
        for name in [
            "isShow",
            "is_show",
            "isBlank",
            "is_blank",
            "sortOrder",
            "sort_order",
        ] {
            let col = resolve_update_column(Some(name)).unwrap();
            assert!(
                PC_NAVIGATION_FIELDS.contains(&col),
                "{name} -> {col} 必须在白名单内"
            );
        }
    }

    #[test]
    fn create_title_validation_follows_dto_annotations() {
        assert_eq!(validate_create_title(Some("首页")).unwrap(), "首页");
        assert_eq!(validate_create_title(Some(" 首页 ")).unwrap(), " 首页 ");
        assert!(validate_create_title(None).is_err());
        assert!(validate_create_title(Some("   ")).is_err());
        let long = "标".repeat(NAV_TITLE_MAX_LEN);
        assert!(validate_create_title(Some(&long)).is_ok());
        let too_long = "标".repeat(NAV_TITLE_MAX_LEN + 1);
        let err = validate_create_title(Some(&too_long)).unwrap_err();
        assert!(err.to_string().contains(TITLE_OVER_LENGTH));
    }

    #[test]
    fn normalize_link_defaults_to_empty_array() {
        assert_eq!(normalize_link(None, None).unwrap(), "[]");
        assert_eq!(normalize_link(Some(1), Some(&json!(null))).unwrap(), "[]");
        // 原样序列化（键顺序由 serde_json 决定，这里只比较语义）
        let stored = normalize_link(Some(1), Some(&json!({"path": "article", "id": 5}))).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&stored).unwrap();
        assert_eq!(parsed, json!({"path": "article", "id": 5}));
    }

    #[test]
    fn bottom_nav_can_only_point_to_article() {
        // type = 3 且 path 非空且 != article → 业务错误
        let err = normalize_link(Some(TYPE_BOTTOM), Some(&json!({"path": "custom"}))).unwrap_err();
        assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
        assert!(err.to_string().contains(BOTTOM_NAV_ARTICLE_ONLY));

        // path = article → 通过
        assert!(normalize_link(Some(TYPE_BOTTOM), Some(&json!({"path": "article"}))).is_ok());
        // path 空白 → 不校验（Java StrUtil.isNotBlank）
        assert!(normalize_link(Some(TYPE_BOTTOM), Some(&json!({"path": "  "}))).is_ok());
        // 无 path → 不校验
        assert!(normalize_link(Some(TYPE_BOTTOM), Some(&json!({"name": "x"}))).is_ok());
        // 非底部导航 → 不校验
        assert!(normalize_link(Some(1), Some(&json!({"path": "custom"}))).is_ok());
        // type 为空 → 不校验
        assert!(normalize_link(None, Some(&json!({"path": "custom"}))).is_ok());
    }

    #[test]
    fn select_link_has_twenty_five_entries_plus_optional_shop_list() {
        let base = select_link_entries(false);
        assert_eq!(base.len(), 25);
        assert_eq!(base[0].name, "商城首页");
        assert_eq!(base[0].link, "/");
        assert_eq!(base[20].name, "积分商城");
        assert_eq!(base[20].link, "exchange/list");
        assert_eq!(base[21].name, "拼团");
        assert!(base.iter().all(|e| e.app_link.is_empty()));
        assert!(!base.iter().any(|e| e.link == "shop/list"));

        let with_shop = select_link_entries(true);
        assert_eq!(with_shop.len(), 26);
        assert_eq!(with_shop[21].name, "店铺列表");
        assert_eq!(with_shop[21].link, "shop/list");
        // 插入位置：积分商城之后、拼团之前
        assert_eq!(with_shop[20].name, "积分商城");
        assert_eq!(with_shop[22].name, "拼团");
        assert_eq!(with_shop[25].name, "抽奖大转盘");
    }

    #[test]
    fn parent_tree_keeps_two_levels_and_drops_orphans() {
        let rows = vec![
            nav(39, 0, "站点导航", 3, "[]"),
            nav(40, 39, "商城首页", 3, "{\"path\":\"base\"}"),
            nav(41, 39, "招商加盟", 4, "{\"path\":\"custom\"}"),
            // 三级节点（父级 40 是二级）→ deep=2 丢弃
            nav(99, 40, "深层", 1, "{}"),
            // 孤立节点（父级不在结果集）→ 丢弃
            nav(98, 12345, "孤儿", 1, "{}"),
            nav(81, 0, "限时秒杀", 50, "[]"),
        ];
        let tree = build_parent_tree(&rows);
        let root_ids: Vec<i32> = tree.iter().map(|n| n.id).collect();
        assert_eq!(root_ids, vec![39, 81]);
        let children: Vec<i32> = tree[0].children.iter().map(|n| n.id).collect();
        assert_eq!(children, vec![40, 41]);
        // 孙级为空
        assert!(tree[0].children.iter().all(|c| c.children.is_empty()));
        // 根节点保留 link 原始形态（"[]" → 空数组，与 VO 的 {} 不同）
        assert_eq!(tree[0].link, json!([]));
        assert_eq!(tree[0].children[0].link, json!({"path": "base"}));
        assert_eq!(tree[1].title, "限时秒杀");
    }

    #[test]
    fn parent_tree_orders_roots_by_input_order() {
        // SQL 已按 parent_id ASC, sort_order ASC 排序，组装只保持顺序
        let rows = vec![
            nav(1, 0, "A", 10, "[]"),
            nav(2, 0, "B", 20, "[]"),
            nav(3, 1, "A-1", 5, "[]"),
            nav(4, 1, "A-2", 6, "[]"),
        ];
        let tree = build_parent_tree(&rows);
        assert_eq!(tree.iter().map(|n| n.id).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(
            tree[0].children.iter().map(|n| n.id).collect::<Vec<_>>(),
            vec![3, 4]
        );
    }

    #[test]
    fn type_helpers_match_java_enum() {
        assert!(nextrade_core::models::content::pc_navigation::is_valid_type(4));
        assert!(!nextrade_core::models::content::pc_navigation::is_valid_type(0));
        assert_eq!(
            nextrade_core::models::content::pc_navigation::pc_navigation_type_name(Some(2)),
            "顶部小导航"
        );
    }
}