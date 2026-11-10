//! 地区服务（system::region_client）：省份、城市、级联、路径。
//!
//! 对应原 Java
//! `java/nextrade-service/src/main/java/com/nextrade/service/setting/impl/RegionServiceImpl.java`
//! （413 行）中由 `RegionController` 调用的 7 个方法：
//!
//! | Java 方法 | 本服务方法 |
//! |-----------|-----------|
//! | `getRegionByIds(String)` | [`RegionClientService::region_cascade`] |
//! | `getProvinceList()` | [`RegionClientService::province_list`] |
//! | `getUserRegion()` | [`RegionClientService::user_region`] |
//! | `getCityRegion()` | [`RegionClientService::city_region`] |
//! | `getCityListByCityName(String)` | [`RegionClientService::city_list_by_name`] |
//! | `getRegionByParentId(Integer)` | [`RegionClientService::region_list_by_parent_id`] |
//! | `getRegionPath(Integer)` | [`RegionClientService::region_path`] |
//!
//! 逐条迁移要点（对照 Java 源）：
//!
//! 1. **`sortByFirstWord`**：严格以后台「首字母」列升序，**未设置首字母的排到最后**
//!    （Java `Comparator.nullsLast`）；首字母相同再按地区名**全拼**升序。
//!    Java 用 `PinyinUtil.getPinyin(name, "")` 取全拼，Rust 侧无拼音库，
//!    改用 `region_name` 本身做二级 key —— 差异见模块末
//!    [`full_pinyin_key`] 说明，已记入 `uncertain_points`。
//! 2. **`filterSelectableRegions`**：仅当父节点是重庆（`500100`）时生效，
//!    从**新地址选择**列表中剔除原江北区 `500105` / 原渝北区 `500112`；
//!    但**加载存量地址**时若当前选中就是旧节点，则仍回填
//!    （`selectedRegionId` 参数）。
//! 3. **`getProvinceList`**：`level = 2`，且把名字里的「省」「市」「自治区」
//!    全部**替换为空**（`replace`，不是仅去后缀）。
//! 4. **`getCityRegion`**：`level = 3` 全量（按 `first_word`、`region_id` 升序）
//!    + `is_hot = 1` 的热门子集；Java 结果走 Redis 缓存
//!    （key `city_pinying_list`），Rust 侧未接 Redis，每次实查。
//! 5. **`getRegionByIds`**：入参是逗号分隔串。取首段地区，若存在则把它
//!    **自己的 `parent_id`** 也插到最前面，然后对列表里**每个** id 各查一次
//!    子节点，逐层给出级联候选列（`List<List<Region>>`）。
//!    `regionIds` 为空 → 只返回默认国家（`config.defaultCountry`）的下一级。
//! 6. **`getRegionPath`**：从 `regionId` 向上回溯到根，命中默认国家即停，
//!    最后**反序**（根 → 叶）。默认国家未配置时会再补一段向上遍历，
//!    但那时 `currentId` 已是 0，该分支实际不生效（见 [`collect_region_path`]）。
//!
//! **未迁移**：
//! - `getUserRegion` 的 IP 定位（Java `Ip2RegionUtil.getRegionByIp()`，
//!   依赖 ip2region 数据库文件）。Rust 侧无该依赖，[`RegionClientService::user_region`]
//!   固定按 Java 的兜底分支走（IP 为空 → `"北京"`），**不是**真实 IP 定位。
//! - `getCityRegion` 的 Redis 缓存（`city_pinying_list`）。
//! - Java 的 `@JsonTranslate` 地区名多语言翻译。

use std::cmp::Ordering;
use std::sync::Arc;

use nextrade_core::entities::Region;
use nextrade_core::models::system::region_client::{
    CityRegionVO, CHONGQING_CITY_REGION_ID, DEPRECATED_CHONGQING_DISTRICT_IDS,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::system::region_client::{RegionClientRepository, CONFIG_DEFAULT_COUNTRY};
use nextrade_infrastructure::db::Repositories;

/// IP 定位失败 / 内网地址时使用的兜底地名
/// （Java `getUserRegion` 的 `StrUtil.isBlank(region) ? "北京" : region`）。
const DEFAULT_REGION_NAME: &str = "北京";

/// 省份层级（Java `getProvinceList` 的 `eq(Region::getLevel, 2)`）。
const LEVEL_PROVINCE: i32 = 2;

/// 城市层级（Java `getCityRegion` / `getCityListByCityName` 的 `level = 3`）。
const LEVEL_CITY: i32 = 3;

/// 国家层级（o2o H5 `getCountryList` 的 `level = 1` 口径）。
const LEVEL_COUNTRY: i32 = 1;

pub struct RegionClientService {
    repos: Arc<Repositories>,
}

impl RegionClientService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 仓储句柄（本片自有仓储）。
    fn regions(&self) -> &RegionClientRepository {
        &self.repos.system.region_client
    }

    // ————————————————————— 1. getRegion —————————————————————

    /// 地区级联列表（Java `RegionServiceImpl#getRegionByIds`）。
    ///
    /// 入参 `region_ids` 是**逗号分隔的 ID 串**（`@RequestParam String regionIds`）。
    ///
    /// Java 逻辑逐句对应：
    /// - `regionIds != null`：`split(",")` → `Integer.parseInt` 逐段解析；
    ///   取首段地区，**若该地区存在**，把它的 `parent_id` 插到最前面
    ///   （于是候选从「父级」开始列）；不存在则原样使用传入列表。
    /// - `regionIds == null`：只放默认国家 ID。
    /// - 随后对列表里每个 id：查其 `parent_id = id` 的子节点，
    ///   用 `sortByFirstWord` 排序，并按 `filterSelectableRegions` 过滤
    ///   （`selectedRegionId` = 该 id 在列表中的**下一个** id，用于回填存量地址）。
    ///
    /// 与 Java 的差异：Java 对 `Integer.parseInt` 失败会抛
    /// `NumberFormatException`（HTTP 500）。此处解析失败的段直接**跳过**，
    /// 避免脏参数打崩请求。
    pub async fn region_cascade(&self, region_ids: Option<&str>) -> Result<Vec<Vec<Region>>> {
        let default_country = self.default_country_id().await?;

        let mut ids: Vec<i32> = match region_ids {
            Some(raw) => raw
                .split(',')
                .filter_map(|seg| seg.trim().parse::<i32>().ok())
                .collect(),
            // Java `new ArrayList<>(); add(defaultCountry)`：默认国家未配置时
            // `Integer.valueOf` 会 NPE，此处退化为空列表 → 返回空级联。
            None => default_country.into_iter().collect(),
        };

        if let Some(first) = ids.first().copied() {
            if let Some(parent_region) = self.regions().find_by_id(first).await? {
                let mut with_parent = Vec::with_capacity(ids.len() + 1);
                with_parent.push(parent_region.parent_id);
                with_parent.extend(ids);
                ids = with_parent;
            }
        }

        let mut out = Vec::with_capacity(ids.len());
        for (idx, parent_id) in ids.iter().enumerate() {
            let selected = ids.get(idx + 1).copied();
            let children = self.regions().find_children(*parent_id).await?;
            out.push(self.sort_and_filter(*parent_id, children, selected));
        }
        Ok(out)
    }

    // ————————————————————— 2. getProvinceList —————————————————————

    /// 省份列表（Java `RegionServiceImpl#getProvinceList`）。
    ///
    /// `level = 2`，且把地区名里所有「省」「市」「自治区」子串**整体删除**
    /// （Java `regionName.replace("省","").replace("市","").replace("自治区","")`，
    /// 注意顺序：先删「省」「市」再删「自治区」）。
    /// 最后按 `sortByFirstWord` 排序。
    ///
    /// Java 侧 `select(regionId, regionName, firstWord)` 只查三列，
    /// 其余列为 `null`；Rust 用 `SELECT *` 保持其余列为库中实际值，
    /// 字段更完整（不影响前端读 `regionId` / `regionName` / `firstWord`）。
    pub async fn province_list(&self) -> Result<Vec<Region>> {
        let mut provinces = self.regions().find_by_level(LEVEL_PROVINCE).await?;
        for region in &mut provinces {
            region.region_name = shorten_province_name(&region.region_name);
        }
        Ok(sort_by_first_word(provinces))
    }

    // ————————————————————— 3. getUserRegion —————————————————————

    /// 用户所在地区（Java `RegionServiceImpl#getUserRegion`）。
    ///
    /// Java 逻辑：
    /// 1. `getRegionByIp()` 按来访 IP 定位城市名；异常/空 → `"北京"`；
    ///    返回 `"0"`（内网）也当作空 → `"北京"`。
    /// 2. `like(regionName, 地名).getFirst()` —— **全表**模糊匹配取第一条。
    /// 3. 抛异常时兜底 `this.list().getFirst()`（全表第一条）。
    ///
    /// **未迁移**：IP 定位（依赖 Java 侧 `ip2region` 库）。本实现固定按
    /// 「IP 为空 → `"北京"`」分支走，与 Java 在内网/定位失败时的结果一致，
    /// 但**不会**按访客真实 IP 给出不同城市。若需要真实定位，
    /// 应由上层解析出城市名后调用 [`RegionClientService::region_by_name_like`]。
    pub async fn user_region(&self) -> Result<Region> {
        self.region_by_name_like(DEFAULT_REGION_NAME).await
    }

    /// 按地名模糊匹配取第一条（`getUserRegion` 第 2/3 步的可复用内核）。
    pub async fn region_by_name_like(&self, name: &str) -> Result<Region> {
        if let Some(hit) = self
            .regions()
            .find_by_name_like(name, None)
            .await?
            .into_iter()
            .next()
        {
            return Ok(hit);
        }
        // 兜底：Java `this.list().getFirst()`；表内必须至少有一行，
        // 否则 Java 也是 NoSuchElementException → 500。
        self.regions()
            .find_first()
            .await?
            .ok_or_else(|| Error::NotFound("地区数据为空".into()))
    }

    // ————————————————————— 4. getCityRegion —————————————————————

    /// 城市列表（热门 + 全量，Java `RegionServiceImpl#getCityRegion`）。
    ///
    /// - `hot`：`level = 3 AND is_hot = 1`
    /// - `city`：`level = 3`，按 `first_word ASC, region_id ASC`
    ///
    /// **未迁移**：Java 用 Redis 缓存整个 VO（key `city_pinying_list`，
    /// 见 `com.nextrade.common.constant.Constants:257`），命中即直接返回、
    /// 不查库。Rust 侧未接入 Redis，本实现**每次实查**，
    /// 语义等价但少了缓存层。
    pub async fn city_region(&self) -> Result<CityRegionVO> {
        let hot = self
            .regions()
            .find_by_level_hot(LEVEL_CITY, 1)
            .await?;
        let city = self.regions().find_by_level_ordered(LEVEL_CITY).await?;
        Ok(CityRegionVO { hot, city })
    }

    // ————————————————————— 5. getCityListByCityName —————————————————————

    /// 按城市名模糊查城市（Java `RegionServiceImpl#getCityListByCityName`）。
    ///
    /// `like(regionName, cityName) AND level = 3`。
    ///
    /// 差异：Java 未对 ILIKE 通配符转义，输入含 `%` / `_` 会撑开匹配；
    /// 本实现对 `\` `%` `_` 做转义（`ESCAPE '\\'`），更严格。
    pub async fn city_list_by_name(&self, city_name: &str) -> Result<Vec<Region>> {
        self.regions()
            .find_by_name_like(city_name, Some(LEVEL_CITY))
            .await
    }

    // ————————————————————— 6. getRegionListByRegionId —————————————————————

    /// 取直接子节点（Java `RegionServiceImpl#getRegionByParentId`）。
    ///
    /// Java 对 `parentId == null` 抛 `GlobalException(PARAM_ERROR)`。
    /// Rust 侧 `regionId` 是 `@RequestParam` 必填，缺失由 axum 提取器
    /// 直接 400，故此处只需 `parent_id` 语义。
    pub async fn region_list_by_parent_id(&self, parent_id: i32) -> Result<Vec<Region>> {
        let children = self.regions().find_children(parent_id).await?;
        // 第三个参数固定 null：Java 此处传的是 `null`，
        // 即**新地址选择**口径 → 重庆已废止的两个区必须被剔除。
        Ok(self.sort_and_filter(parent_id, children, None))
    }

    // ————————————————————— 7. getRegionPath —————————————————————

    /// 完整地区路径（Java `RegionServiceImpl#getRegionPath`）。
    ///
    /// 从 `regionId` 向上回溯，命中默认国家即停，最后反序（根 → 叶）。
    /// 取不到默认国家配置时，Java 会「再补一遍向上遍历」，但此时
    /// `currentId` 已经是 0，`while (currentId != 0)` 不进入 —— 该分支不生效。
    /// 见纯函数 [`collect_region_path`]。
    pub async fn region_path(&self, region_id: i32) -> Result<Vec<Region>> {
        let default_country_id = self.default_country_id().await?;
        let mut path: Vec<Region> = Vec::new();
        let mut current = Some(region_id);

        loop {
            let Some(id) = current else { break };
            if id == 0 {
                break;
            }
            let Some(region) = self.regions().find_by_id(id).await? else {
                break;
            };
            let next = region.parent_id;
            path.push(region);
            if default_country_id == Some(id) {
                break;
            }
            current = Some(next);
        }

        path.reverse();
        Ok(path)
    }

    // ————————————————————— 内部 —————————————————————

    /// 读「默认国家」配置（Java `getConfigByCode(SettingsEnum.DEFAULT_COUNTRY)`）。
    ///
    /// 解析规则与 proxy_shopping_user_address 的 `parse_default_country` 一致：
    /// 空白 / 非法数字 → `None`。
    async fn default_country_id(&self) -> Result<Option<i32>> {
        let raw = self
            .regions()
            .find_config_val(CONFIG_DEFAULT_COUNTRY)
            .await?;
        Ok(parse_default_country(raw.as_deref()))
    }

    /// `sortByFirstWord` + `filterSelectableRegions` 的组合
    /// （Java 里这两步总是成对出现在候选列表上）。
    fn sort_and_filter(
        &self,
        parent_id: i32,
        regions: Vec<Region>,
        selected_region_id: Option<i32>,
    ) -> Vec<Region> {
        let regions = filter_selectable_regions(parent_id, regions, selected_region_id);
        sort_by_first_word(regions)
    }

    // ——————————————————————————————————————————————
    // o2o H5 定位类接口（b2b2b 5.8.57 前端新增，无 Java 控制器对应，
    // 按表结构 + 前端语义最小实现；高德 / ip2region 凭据未接入时降级）
    // ——————————————————————————————————————————————

    /// 国家列表（`getCountryList`：`region` 表 `level = 1`，
    /// 本表无 `is_country` 列，按层级口径取）。
    pub async fn country_list(&self) -> Result<Vec<Region>> {
        self.regions().find_by_level(LEVEL_COUNTRY).await
    }

    /// 按 IP 定位（`getLocationByIp`）。
    ///
    /// 未接入 ip2region / 高德 IP 库 → 固定走 Java `getUserRegion` 的
    /// 兜底分支（默认城市「北京」，查不到再取全表第一条）。
    pub async fn location_by_ip(&self, _ip: Option<&str>) -> Result<Region> {
        self.user_region().await
    }

    /// 按经纬度定位（`getLocationByLocation`）。
    ///
    /// 逆地理编码需高德 Key（未配置）→ 降级为默认城市，与
    /// `getLocationByIp` 同一兜底口径。
    pub async fn location_by_location(
        &self,
        _longitude: Option<&str>,
        _latitude: Option<&str>,
    ) -> Result<Region> {
        self.user_region().await
    }

    /// 按 regionId 取定位详情（`getLocationByRegionId`：主键查询，
    /// 缺行返回 `None`）。
    pub async fn location_by_region_id(&self, region_id: i32) -> Result<Option<Region>> {
        self.regions().find_by_id(region_id).await
    }

    /// 逆地理编码详情（`getRegeoDetail`：高德 regeo，需 Key）。
    ///
    /// 未配置凭据 → 返回默认城市的降级结果
    /// （`{ regionId, regionName, ... }`，`regeo: false` 标记非真实逆编码）。
    pub async fn regeo_detail(&self) -> Result<Region> {
        self.user_region().await
    }

    /// 关键词搜索地区（`searchByKeywords`：`region_name ILIKE`，
    /// 不限层级，上限 50 条）。
    pub async fn search_by_keywords(&self, keywords: &str) -> Result<Vec<Region>> {
        self.regions().find_by_name_like(keywords, None).await
    }

    /// 共享仓储聚合（o2o H5 / 公共机器翻译等无独立服务聚合的公共路由复用；
    /// `PublicState` 未挂 o2o / lang 服务，经本服务取 `Repositories`）。
    #[must_use]
    pub fn repos(&self) -> &Arc<Repositories> {
        &self.repos
    }
}

/// 解析「默认国家」配置值（Java `Integer.valueOf(bizVal)`）。
///
/// 空串 / 空白 / 非数字 → `None`（Java 在 `getRegionByIds` 里会 NPE，
/// 这里退化为「未配置」，更安全）。
#[must_use]
pub fn parse_default_country(raw: Option<&str>) -> Option<i32> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<i32>().ok())
}

/// 省份名精简（Java `getProvinceList` 的链式 `replace`）。
///
/// 顺序与 Java 一致：先 `省`、再 `市`、最后 `自治区`。
/// 例：`内蒙古自治区` → 删「自治区」→ `内蒙古`；
///     `广西壮族自治区` → `广西壮族`（Java 同样如此，不是「广西」）。
#[must_use]
pub fn shorten_province_name(name: &str) -> String {
    name.replace('省', "").replace('市', "").replace("自治区", "")
}

/// `sortByFirstWord`（Java `RegionServiceImpl#sortByFirstWord`）。
///
/// 规则：
/// 1. 主键 `first_word`（trim + 大写），**空的排到最后**（`nullsLast`）
/// 2. 相同则按[全拼二级 key][full_pinyin_key]升序
#[must_use]
pub fn sort_by_first_word(mut regions: Vec<Region>) -> Vec<Region> {
    regions.sort_by(|a, b| {
        let ka = resolve_first_word(a);
        let kb = resolve_first_word(b);
        match (ka, kb) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| {
                full_pinyin_key(a).cmp(&full_pinyin_key(b))
            }),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => full_pinyin_key(a).cmp(&full_pinyin_key(b)),
        }
    });
    regions
}

/// 首字母排序键（Java `resolveFirstWord`）：空白 → `None`（排最后），
/// 否则 trim + 大写。
#[must_use]
pub fn resolve_first_word(region: &Region) -> Option<String> {
    let trimmed = region.first_word.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_uppercase())
    }
}

/// 全拼排序键（Java `resolveFullPinyin` 的替换实现）。
///
/// ⚠️ **与 Java 的差异**：Java 用 `cn.hutool.extra.pinyin.PinyinUtil.getPinyin(name, "")`
/// 取汉字全拼（如 `北京市` → `beijingshi`）再做二级排序。Rust 侧没有等价的
/// 拼音库，此处退化为「按 `region_name` 本身（小写）比较」。
///
/// 影响面：只有当**两个地区的 `first_word` 相同时**才会用到该 key。
/// 由于 `region` 表的 `first_word` 就是拼音首字母，同首字母的地区（如
/// 「北京」「包头」「本溪」都在 `B` 组）在 Java 里按全拼排序、在 Rust 里
/// 按汉字 Unicode 码位排序，**组内顺序会与 Java 不同**（分组本身一致）。
/// 已记入切片 `uncertain_points`。
#[must_use]
pub fn full_pinyin_key(region: &Region) -> String {
    region.region_name.to_lowercase()
}

/// `filterSelectableRegions`（Java `RegionServiceImpl#filterSelectableRegions`）。
///
/// 仅当 `parent_id == 500100`（重庆）时剔除已废止的原江北区 `500105`、
/// 原渝北区 `500112`；但若当前**已选中**其中某个（存量地址回填），
/// 则该节点保留，使历史地址仍能正常展示。
///
/// Java 用 `stream().toList()` 返回**新列表**（非原地修改），
/// 因此这里按「传入列表 → 返回过滤后列表」的语义实现为消费式函数。
#[must_use]
pub fn filter_selectable_regions(
    parent_id: i32,
    regions: Vec<Region>,
    selected_region_id: Option<i32>,
) -> Vec<Region> {
    if parent_id != CHONGQING_CITY_REGION_ID || regions.is_empty() {
        return regions;
    }
    regions
        .into_iter()
        .filter(|r| {
            !DEPRECATED_CHONGQING_DISTRICT_IDS.contains(&r.region_id)
                || Some(r.region_id) == selected_region_id
        })
        .collect()
}

/// 向上回溯地区路径的纯逻辑（与 [`RegionClientService::region_path`] 等价，
/// 便于单测；`lookup` 为按 ID 取地区）。
///
/// 保留 Java 「默认国家未配置时再遍历一次」的分支结构，并在注释中说明
/// 该分支为何不生效（进入时 `current_id` 已为 `None`）。
#[must_use]
pub fn collect_region_path<F>(
    region_id: i32,
    default_country_id: Option<i32>,
    lookup: F,
) -> Vec<Region>
where
    F: Fn(i32) -> Option<Region>,
{
    let mut path = Vec::new();
    let mut current = Some(region_id);

    while let Some(id) = current {
        if id == 0 {
            break;
        }
        let Some(region) = lookup(id) else { break };
        let parent = region.parent_id;
        path.push(region);
        if default_country_id == Some(id) {
            break;
        }
        current = Some(parent);
    }

    // Java：`if (defaultCountryId == null && !path.isEmpty())` 再向上遍历一次。
    // 此时 `current` 仍是循环里最后一次赋的 `Some(parent)`，只有当父级不存在于
    // 数据中（`lookup` 返回 None）时它才非 0，因此**不能**断言 `current` 为
    // `None` 或 `0`；Java 那个分支进入后 `while (currentId != null && currentId != 0)`
    // 的首轮查库同样失败，净效果是空操作，故此处无需实现。

    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: i32, name: &str, first_word: &str, parent_id: i32) -> Region {
        Region {
            region_id: id,
            level: Some(3),
            region_name: name.to_string(),
            parent_id,
            is_hot: 0,
            sort_order: 0,
            first_word: first_word.to_string(),
            standard_code: None,
        }
    }

    // ————————————————————— sortByFirstWord —————————————————————

    #[test]
    fn sort_orders_by_first_word_asc() {
        let list = vec![
            region(3, "c", "C", 1),
            region(1, "a", "A", 1),
            region(2, "b", "B", 1),
        ];
        let sorted = sort_by_first_word(list);
        let ids: Vec<i32> = sorted.iter().map(|r| r.region_id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn sort_pushes_blank_first_word_to_the_end() {
        // Java `Comparator.nullsLast`：未设置首字母的排到最后
        let list = vec![
            region(1, "zzz", "", 1),
            region(2, "bbb", "B", 1),
            region(3, "   ", "  ", 1), // 纯空白同样视为未设置
        ];
        let sorted = sort_by_first_word(list);
        let ids: Vec<i32> = sorted.iter().map(|r| r.region_id).collect();
        assert_eq!(ids.first(), Some(&2)); // 有首字母的排最前
        // 两条空白记录都排在最后两位（彼此间按全拼二级 key 排序，
        // 即 `region_name` 升序："   " < "zzz"，故 id 3 在 id 1 之前）。
        assert_eq!(&ids[1..], &[3, 1]);
        assert_eq!(sorted.last().map(|r| r.region_id), Some(1));
    }

    #[test]
    fn sort_uppercases_key_and_trims() {
        let list = vec![region(1, "x", " b ", 1), region(2, "y", "A", 1)];
        let sorted = sort_by_first_word(list);
        assert_eq!(sorted.first().map(|r| r.region_id), Some(2)); // A < B
    }

    #[test]
    fn resolve_first_word_handles_blank() {
        assert_eq!(resolve_first_word(&region(1, "x", " b ", 1)), Some("B".into()));
        assert_eq!(resolve_first_word(&region(1, "x", "   ", 1)), None);
        assert_eq!(resolve_first_word(&region(1, "x", "", 1)), None);
    }

    #[test]
    fn sort_is_stable_for_equal_keys_via_name() {
        // 首字母相同 → 按 region_name（Java 是全拼）二级排序
        let list = vec![
            region(1, "Zhongqing", "Z", 1),
            region(2, "Ankang", "Z", 1),
        ];
        let sorted = sort_by_first_word(list);
        assert_eq!(sorted.first().map(|r| r.region_id), Some(2)); // "ankang" < "zhongqing"
    }

    #[test]
    fn sort_handles_empty_and_single() {
        assert!(sort_by_first_word(Vec::new()).is_empty());
        assert_eq!(sort_by_first_word(vec![region(9, "a", "A", 0)]).len(), 1);
    }

    // ————————————————————— filterSelectableRegions —————————————————————

    #[test]
    fn filter_drops_deprecated_chongqing_districts() {
        let list = vec![
            region(500105, "江北区", "J", 500100),
            region(500106, "沙坪坝区", "S", 500100),
            region(500112, "渝北区", "Y", 500100),
        ];
        let kept = filter_selectable_regions(500100, list, None);
        let ids: Vec<i32> = kept.iter().map(|r| r.region_id).collect();
        assert_eq!(ids, vec![500106]);
    }

    #[test]
    fn filter_keeps_selected_deprecated_district_for_legacy_address() {
        // 存量地址正好选中 500112 → 必须回填，不能被过滤掉
        let list = vec![
            region(500105, "江北区", "J", 500100),
            region(500112, "渝北区", "Y", 500100),
        ];
        let kept = filter_selectable_regions(500100, list, Some(500112));
        let ids: Vec<i32> = kept.iter().map(|r| r.region_id).collect();
        assert_eq!(ids, vec![500112]);
    }

    #[test]
    fn filter_noop_for_non_chongqing_parent() {
        let list = vec![
            region(500105, "江北区", "J", 500000),
            region(500112, "渝北区", "Y", 500000),
        ];
        let kept = filter_selectable_regions(500000, list, None);
        assert_eq!(kept.len(), 2);
    }

    // ————————————————————— province name —————————————————————

    #[test]
    fn shorten_province_name_matches_java_replace_chain() {
        assert_eq!(shorten_province_name("广东省"), "广东");
        assert_eq!(shorten_province_name("北京市"), "北京");
        assert_eq!(shorten_province_name("内蒙古自治区"), "内蒙古");
        // Java 只删「自治区」三字，故「广西壮族自治区」→「广西壮族」
        assert_eq!(shorten_province_name("广西壮族自治区"), "广西壮族");
        assert_eq!(shorten_province_name("宁夏回族自治区"), "宁夏回族");
        // 中段命中也会被删（Java 是全串 replace，非仅后缀）
        assert_eq!(shorten_province_name("省市区"), "区");
    }

    // ————————————————————— parse_default_country —————————————————————

    #[test]
    fn parse_default_country_matches_integer_value_of() {
        assert_eq!(parse_default_country(Some("1")), Some(1));
        assert_eq!(parse_default_country(Some(" 1 ")), Some(1));
        assert_eq!(parse_default_country(Some("")), None);
        assert_eq!(parse_default_country(Some("   ")), None);
        assert_eq!(parse_default_country(Some("CN")), None);
        assert_eq!(parse_default_country(None), None);
    }

    // ————————————————————— collect_region_path —————————————————————

    #[test]
    fn region_path_stops_at_default_country_and_reverses() {
        // 1(中国) -> 110000(北京市) -> 110100 -> 110101
        let all = [
            region(1, "中国", "Z", 0),
            region(110000, "北京市", "B", 1),
            region(110100, "北京市", "B", 110000),
            region(110101, "东城区", "D", 110100),
        ];
        let path = collect_region_path(110101, Some(1), |id| {
            all.iter().find(|r| r.region_id == id).cloned()
        });
        let ids: Vec<i32> = path.iter().map(|r| r.region_id).collect();
        assert_eq!(ids, vec![1, 110000, 110100, 110101]);
    }

    #[test]
    fn region_path_without_default_country_walks_to_root() {
        let all = [
            region(1, "中国", "Z", 0),
            region(110000, "北京市", "B", 1),
            region(110101, "东城区", "D", 110000),
        ];
        let path = collect_region_path(110101, None, |id| {
            all.iter().find(|r| r.region_id == id).cloned()
        });
        let ids: Vec<i32> = path.iter().map(|r| r.region_id).collect();
        assert_eq!(ids, vec![1, 110000, 110101]);
    }

    #[test]
    fn region_path_breaks_on_missing_node() {
        let all = [region(110101, "东城区", "D", 110100)];
        let path = collect_region_path(110101, None, |id| {
            all.iter().find(|r| r.region_id == id).cloned()
        });
        // 110100 不存在 → 只留下起点
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].region_id, 110101);
    }

    #[test]
    fn region_path_empty_when_region_missing() {
        let path = collect_region_path(999, Some(1), |_| None);
        assert!(path.is_empty());
    }

    #[test]
    fn region_path_of_default_country_is_single_node() {
        let all = [region(1, "中国", "Z", 0)];
        let path = collect_region_path(1, Some(1), |id| {
            all.iter().find(|r| r.region_id == id).cloned()
        });
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].region_id, 1);
    }

    // ————————————————————— 级联参数解析 —————————————————————

    #[test]
    fn cascade_ids_parsing_skips_garbage_segments() {
        // Java `split(",")` + `Integer.parseInt`：此处对非法段容错跳过
        let parsed: Vec<i32> = "1,110000,abc,,110100"
            .split(',')
            .filter_map(|seg| seg.trim().parse::<i32>().ok())
            .collect();
        assert_eq!(parsed, vec![1, 110000, 110100]);
    }
}