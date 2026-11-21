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
//! - `getCityRegion` 的 Redis 缓存（`city_pinying_list`）。
//! - Java 的 `@JsonTranslate` 地区名多语言翻译。
//!
//! **IP 定位（2026-11-13 补齐）**：原 Java `Ip2RegionUtil.getRegionByIp()`
//! 依赖 ip2region 数据库文件；本切片现接入 Rust 侧 ip2region v2 xdb
//! （crate `ip2region`），xdb 文件在运行时按以下顺序查找：
//!
//! 1. 环境变量 `NEXTRADE_IP2REGION_XDB` 指定的路径；
//! 2. `/www/wwwroot/nextrade/ip2region.xdb`（生产部署目录）；
//! 3. `./data/ip2region.xdb`（本地/容器相对路径）。
//!
//! 文件不存在或查询失败时，沿用 Java 的兜底分支（`"北京"`），
//! 与迁移期行为一致 —— 即**定位失败永远不报错，只降级**。

use std::cmp::Ordering;
use std::sync::{Arc, OnceLock};

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
///
/// 历史坑（2026-11-16 实测）：存量国家行曾为 level = 0（中国 id=1 等），
/// 与本口径错位，`getCountryList` 一度漏掉中国。2026-11-21 已由迁移
/// `2026-11-21_region_level_fix.sql` 统一：带 standard_code 的根行一律
/// level = 1（同轮另有 243 国导入，国家行共 246）；仅一行无代码的遗留
/// 空根行（id=820218「美国」，无子级）刻意保留 level = 0，不参与国家匹配。
const LEVEL_COUNTRY: i32 = 1;

/// 地图服务商配置项的 `biz_code`（与 `config_basic` 注册表 EP88 同码，
/// 经地区仓储的 `find_config_val` 读 `config` 表活行）。
const CONFIG_MAP_PROVIDER: &str = "mapProvider";

/// 天地图 Web 服务 TK 配置项的 `biz_code`（同上）。
const CONFIG_TIANDITU_WEB_TK: &str = "tiandituWebTk";
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
    /// 2026-11-13 起补齐第 1 步：xdb 存在时按 IP 离线定位，
    /// 取 xdb 的 `city`（缺则 `province`）做模糊匹配；xdb 缺失 /
    /// IP 为内网或 `unknown` / 查询失败时，与 Java 一致回落 `"北京"`。
    ///
    /// 2026-11-16 补国外口径：xdb 国家段非「中国」时只认国家级定位，
    /// 在地区表按国家名（level=1，2026-11-21 口径统一后）匹配返回国家行
    /// （如「美国」）；2026-11-21 全量导入后国家行共 246 行，匹配不到的
    /// 国家合成一个仅含国家名的地区返回——国外访客顶栏显示真实国家名，
    /// 不落国内省份、更不兜底成北京。
    pub async fn user_region(&self, ip: Option<&str>) -> Result<Region> {
        if let Some(ip) = ip {
            if let Some((country, name)) = lookup_region_parts(ip) {
                if let Some(country) = country.as_deref().filter(|c| *c != "中国") {
                    if let Some(hit) = self
                        .regions()
                        .find_by_name_like(country, Some(LEVEL_COUNTRY))
                        .await?
                        .into_iter()
                        .next()
                    {
                        return Ok(hit);
                    }
                    // 国家行已全量导入（246 行），仍匹配不到的国家不在表中：
                    // 合成一个仅含国家名的地区返回，保证国外访客显示
                    // 真实国家名，而不是落到国内兜底城市。
                    return Ok(Region {
                        region_id: 0,
                        level: Some(LEVEL_COUNTRY),
                        region_name: country.to_string(),
                        parent_id: 0,
                        is_hot: 0,
                        sort_order: 0,
                        first_word: String::new(),
                        standard_code: None,
                    });
                }
                if let Some(name) = name {
                    if let Some(hit) = self
                        .regions()
                        .find_by_name_like(&name, None)
                        .await?
                        .into_iter()
                        .next()
                    {
                        return Ok(hit);
                    }
                }
            }
        }
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
    /// xdb 已接入：显式传入 `ip` 或缺省（`None`）时均可离线定位；
    /// 失败 → 兜底城市（Java `getUserRegion` 的兜底分支）。
    pub async fn location_by_ip(&self, ip: Option<&str>) -> Result<Region> {
        self.user_region(ip).await
    }

    /// 按经纬度定位（`getLocationByLocation`）。
    ///
    /// 逆地理编码按地图服务商分流：
    /// - `mapProvider = tianditu` 且已配 `tiandituWebTk` → 调天地图
    ///   Web 服务逆地理编码（见 [`Self::tianditu_regeo_region`]），
    ///   结果地名回表匹配为地区行；
    /// - 其余情形（高德未接服务端逆编码 / 未配置 / 调用失败）→
    ///   降级为默认城市，与 `getLocationByIp` 同一兜底口径，
    ///   行为与补齐前一致。
    pub async fn location_by_location(
        &self,
        longitude: Option<&str>,
        latitude: Option<&str>,
    ) -> Result<Region> {
        if let (Some(lng), Some(lat)) = (longitude, latitude) {
            if let Some(region) = self.tianditu_regeo_region(lng, lat).await {
                return Ok(region);
            }
        }
        self.user_region(None).await
    }

    /// 按 regionId 取定位详情（`getLocationByRegionId`：主键查询，
    /// 缺行返回 `None`）。
    pub async fn location_by_region_id(&self, region_id: i32) -> Result<Option<Region>> {
        self.regions().find_by_id(region_id).await
    }

    /// 逆地理编码详情（`getRegeoDetail`）。
    ///
    /// 返回 `(地区, 是否真实逆编码)`：
    /// - 天地图分支命中（服务商 = tianditu、Web TK 已配、调用成功且
    ///   地名回表命中）→ `(地区, true)`；
    /// - 其余情形 → 默认城市的降级结果 + `false`（路由层据此回
    ///   `regeo: false`，与补齐前口径一致）。
    pub async fn regeo_detail(
        &self,
        longitude: Option<&str>,
        latitude: Option<&str>,
    ) -> Result<(Region, bool)> {
        if let (Some(lng), Some(lat)) = (longitude, latitude) {
            if let Some(region) = self.tianditu_regeo_region(lng, lat).await {
                return Ok((region, true));
            }
        }
        Ok((self.user_region(None).await?, false))
    }

    /// 天地图 Web 服务逆地理编码 -> 地区表行（本批补齐，消费
    /// 此前无消费方的 `tiandituWebTk`）。
    ///
    /// 触发条件：`mapProvider = "tianditu"` 且 `tiandituWebTk` 已配、
    /// 经纬度可解析。调用天地图 geocoder（`type=geocode`），取
    /// `result.addressComponent` 的市/区县/省名依次回表匹配
    /// （先城市层级、再不限层级，与 `user_region` 的模糊匹配口径一致）。
    /// 任何失败（未触发 / 网络错误 / 状态非 0 / 无地名 / 回表未命中）
    /// 一律返回 `None`，由调用方走原降级路径——定位失败永不报错。
    ///
    /// 坐标口径：入参按前端接口边界约定为天地图坐标系（CGCS2000）；
    /// 前端 `api/region.ts` 在 provider=tianditu 时已做 GCJ-02 转换
    /// （小程序端），H5 直传的 GCJ-02 与之差数百米，仅影响城市级
    /// 匹配的边界情形。
    async fn tianditu_regeo_region(&self, longitude: &str, latitude: &str) -> Option<Region> {
        let provider = self
            .regions()
            .find_config_val(CONFIG_MAP_PROVIDER)
            .await
            .ok()??;
        if provider.trim() != "tianditu" {
            return None;
        }
        let tk = self
            .regions()
            .find_config_val(CONFIG_TIANDITU_WEB_TK)
            .await
            .ok()??;
        let tk = tk.trim();
        if tk.is_empty() || tk == "0" {
            return None;
        }
        let lng: f64 = longitude.trim().parse().ok()?;
        let lat: f64 = latitude.trim().parse().ok()?;
        if !lng.is_finite() || !lat.is_finite() {
            return None;
        }

        let url = tianditu_geocoder_url(lng, lat, tk);
        let value: serde_json::Value = reqwest::Client::new()
            .get(url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        let names = parse_tianditu_geocoder_names(&value)?;

        for name in names {
            if let Ok(hits) = self
                .regions()
                .find_by_name_like(&name, Some(LEVEL_CITY))
                .await
            {
                if let Some(hit) = hits.into_iter().next() {
                    return Some(hit);
                }
            }
            if let Ok(hits) = self.regions().find_by_name_like(&name, None).await {
                if let Some(hit) = hits.into_iter().next() {
                    return Some(hit);
                }
            }
        }
        None
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

/// 拼天地图 geocoder 逆地理编码 URL（纯函数，便于单测）。
///
/// 官方口径：`GET https://api.tianditu.gov.cn/geocoder?postStr=
/// {"lon":<lng>,"lat":<lat>,"ver":1}&type=geocode&tk=<tk>`，
/// postStr 需 URL 编码（此处手工拼串后整段 percent-encode，
/// 避免为单个调用点引入 urlencoding 依赖）。
#[must_use]
pub fn tianditu_geocoder_url(lng: f64, lat: f64, tk: &str) -> String {
    let post_str = format!("{{\"lon\":{lng},\"lat\":{lat},\"ver\":1}}");
    format!(
        "https://api.tianditu.gov.cn/geocoder?postStr={}&type=geocode&tk={}",
        percent_encode(&post_str),
        percent_encode(tk),
    )
}

/// 最小 percent-encode：非保留字符按 UTF-8 字节编码
/// （postStr 仅含 ASCII 结构字符与数字，tk 为字母数字串）。
fn percent_encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() * 2);
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// 解析天地图 geocoder 响应，取可回表匹配的地名候选（市 -> 区县 -> 省）。
///
/// 响应形态：`{"status":"0","result":{"addressComponent":
/// {"province":"山东省","city":"青岛市","county":"市南区",...}}}`。
/// 状态非 `"0"`、结构缺失或三段地名全空 -> `None`。
/// 直辖市等情形 city 可能为空，此时以 county / province 兜底。
#[must_use]
pub fn parse_tianditu_geocoder_names(value: &serde_json::Value) -> Option<Vec<String>> {
    if value.get("status").and_then(serde_json::Value::as_str) != Some("0") {
        return None;
    }
    let component = value.get("result")?.get("addressComponent")?;
    let pick = |key: &str| {
        component
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let names: Vec<String> = ["city", "county", "province"]
        .into_iter()
        .filter_map(|key| pick(key))
        .collect();
    if names.is_empty() {
        None
    } else {
        Some(names)
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

// ————————————————————— IP 离线定位（ip2region xdb） —————————————————————

/// xdb 文件的候选路径（按序尝试）。第一个来自环境变量
/// `NEXTRADE_IP2REGION_XDB`；其余为部署目录与本地/容器相对路径的约定值。
const XDB_PATH_ENV: &str = "NEXTRADE_IP2REGION_XDB";
const XDB_PATH_DEPLOY: &str = "/www/wwwroot/nextrade/ip2region.xdb";
const XDB_PATH_LOCAL: &str = "./data/ip2region.xdb";

/// 进程内单例：xdb 整体载入内存后可安全并发查询。
/// `None` 表示「找不到 xdb 或载入失败」，此后不再重试读盘。
static IP_XDB: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// 取（或初始化）进程级 xdb 内容。找不到文件返回 `None` 并记忆，
/// 避免每个请求都空跑一遍文件系统。
fn ip_xdb() -> Option<&'static [u8]> {
    IP_XDB
        .get_or_init(|| {
            let mut candidates = Vec::new();
            if let Ok(p) = std::env::var(XDB_PATH_ENV) {
                let p = p.trim().to_string();
                if !p.is_empty() {
                    candidates.push(p);
                }
            }
            candidates.push(XDB_PATH_DEPLOY.to_string());
            candidates.push(XDB_PATH_LOCAL.to_string());

            for path in candidates {
                match std::fs::read(&path) {
                    Ok(bytes) if bytes.len() > XDB_VECTOR_INDEX_END => {
                        tracing::info!("ip2region xdb 已加载: {path}");
                        return Some(bytes);
                    }
                    Ok(_) => {
                        tracing::debug!("ip2region xdb 文件过小 {path}");
                    }
                    Err(e) => {
                        tracing::debug!("ip2region xdb 加载失败 {path}: {e}");
                    }
                }
            }
            tracing::warn!("ip2region xdb 未找到（候选路径均失败），IP 定位降级为默认城市");
            None
        })
        .as_deref()
}

/// xdb 布局常量（v2 格式）：256B 头 + 256×256×8B 向量索引，之后是段索引。
const XDB_HEADER_LEN: usize = 256;
const XDB_VECTOR_INDEX_END: usize = XDB_HEADER_LEN + 256 * 256 * 8;
const XDB_SEGMENT_LEN: usize = 14;

/// 在 xdb 内容中按官方算法检索 IPv4（向量索引定位 /16 段块，块内二分）。
///
/// 此前用第三方 `ip2region` 0.1 crate，其二分实现有缺陷：多段块中
/// 相当一部分 IP 恒返回「No results found」（实测 8.8.8.8 / 1.1.1.1 /
/// 193.0.0.1 / 210.0.0.1 等均漏检，官方算法同文件全命中），故自研。
fn xdb_search(content: &[u8], ip: &str) -> Option<String> {
    let v4: std::net::Ipv4Addr = ip.trim().parse().ok()?;
    let o = v4.octets();
    let target = u32::from_be_bytes(o);
    let cell = XDB_HEADER_LEN + ((o[0] as usize) * 256 + o[1] as usize) * 8;
    let s_ptr = u32::from_le_bytes(content.get(cell..cell + 4)?.try_into().ok()?) as usize;
    let e_ptr = u32::from_le_bytes(content.get(cell + 4..cell + 8)?.try_into().ok()?) as usize;
    if e_ptr <= s_ptr || e_ptr > content.len() {
        return None;
    }
    let count = (e_ptr - s_ptr) / XDB_SEGMENT_LEN;
    if count == 0 {
        return None;
    }
    let (mut lo, mut hi) = (0usize, count - 1);
    while lo <= hi {
        let m = (lo + hi) / 2;
        let p = s_ptr + m * XDB_SEGMENT_LEN;
        let seg = content.get(p..p + XDB_SEGMENT_LEN)?;
        let start_ip = u32::from_be_bytes(seg[0..4].try_into().ok()?);
        let end_ip = u32::from_be_bytes(seg[4..8].try_into().ok()?);
        if target < start_ip {
            if m == 0 {
                return None;
            }
            hi = m - 1;
        } else if target > end_ip {
            lo = m + 1;
        } else {
            let data_len = u16::from_le_bytes(seg[8..10].try_into().ok()?) as usize;
            let data_ptr = u32::from_le_bytes(seg[10..14].try_into().ok()?) as usize;
            let raw = content.get(data_ptr..data_ptr + data_len)?;
            return String::from_utf8(raw.to_vec()).ok();
        }
    }
    None
}

/// 取 xdb 搜索器并查一个 IP（组合 `ip_xdb` + `xdb_search`）。
fn lookup_xdb(ip: &str) -> Option<String> {
    xdb_search(ip_xdb()?, ip)
}

/// 判定「值得查询」的 IP：仅公网 IPv4。`unknown` / 回环 / 内网段 /
/// IPv6（v2 xdb 的 0.1 crate 只支持 v4）一律返回 `false` → 走兜底分支。
#[must_use]
pub fn is_public_ipv4(ip: &str) -> bool {
    let Ok(v4) = ip.trim().parse::<std::net::Ipv4Addr>() else {
        return false;
    };
    let o = v4.octets();
    !(o[0] == 10
        || o[0] == 127
        || (o[0] == 172 && (16..=31).contains(&o[1]))
        || (o[0] == 192 && o[1] == 168)
        || (o[0] == 169 && o[1] == 254)
        || o[0] == 0)
}

/// 解析 xdb 查询结果（v2 文本格式 `国家|区域|省份|城市|ISP`），
/// 取「城市，缺则省份」作为用于模糊匹配的地名；`0` / 空段 /
/// `内网IP` 等占位值视为缺失。
///
/// 例：`"中国|0|山东省|青岛市|电信"` → `Some("青岛市")`；
///     `"中国|0|北京市||"`（直辖市城市段可能为空）→ `Some("北京市")`。
#[must_use]
pub fn parse_xdb_region_name(raw: &str) -> Option<&str> {
    let seg = |i: usize| {
        raw.split('|')
            .nth(i)
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "0" && *s != "内网IP")
    };
    seg(3).or_else(|| seg(2))
}

/// xdb 国家段（第 0 段）：国外定位只认国家。空 / `0` / 内网标记 → `None`。
pub fn parse_xdb_country(raw: &str) -> Option<&str> {
    raw.split('|')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "0" && *s != "内网IP")
}

/// IP →（国家，地名）（xdb 离线查询）。任一环节失败 → `None`（调用方走兜底）。
fn lookup_region_parts(ip: &str) -> Option<(Option<String>, Option<String>)> {
    if !is_public_ipv4(ip) {
        return None;
    }
    let raw = lookup_xdb(ip)?;
    let country = parse_xdb_country(&raw).map(str::to_owned);
    let name = parse_xdb_region_name(&raw).map(str::to_owned);
    if country.is_none() && name.is_none() {
        return None;
    }
    Some((country, name))
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

    // ————————————————————— 天地图 geocoder —————————————————————

    #[test]
    fn tianditu_geocoder_url_encodes_post_str_and_tk() {
        let url = tianditu_geocoder_url(120.3826, 36.0671, "abc123");
        assert!(url.starts_with("https://api.tianditu.gov.cn/geocoder?postStr="));
        assert!(url.contains("%7B%22lon%22%3A120.3826%2C%22lat%22%3A36.0671%2C%22ver%22%3A1%7D"));
        assert!(url.ends_with("&type=geocode&tk=abc123"));
    }

    #[test]
    fn tianditu_geocoder_names_prefers_city_then_county_then_province() {
        let value = serde_json::json!({
            "status": "0",
            "result": {
                "addressComponent": {
                    "province": "山东省",
                    "city": "青岛市",
                    "county": "市南区"
                }
            }
        });
        assert_eq!(
            parse_tianditu_geocoder_names(&value),
            Some(vec!["青岛市".to_string(), "市南区".to_string(), "山东省".to_string()])
        );
    }

    #[test]
    fn tianditu_geocoder_names_falls_back_when_city_missing() {
        let value = serde_json::json!({
            "status": "0",
            "result": {
                "addressComponent": { "province": "北京市", "city": "", "county": "海淀区" }
            }
        });
        assert_eq!(
            parse_tianditu_geocoder_names(&value),
            Some(vec!["海淀区".to_string(), "北京市".to_string()])
        );
    }

    #[test]
    fn tianditu_geocoder_names_rejects_error_status_and_empty() {
        let err = serde_json::json!({ "status": "1", "msg": "tk invalid" });
        assert_eq!(parse_tianditu_geocoder_names(&err), None);
        let empty = serde_json::json!({
            "status": "0",
            "result": { "addressComponent": { "province": "", "city": "", "county": "" } }
        });
        assert_eq!(parse_tianditu_geocoder_names(&empty), None);
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

    // ————————————————————— IP 定位 —————————————————————

    #[test]
    fn is_public_ipv4_accepts_only_public_v4() {
        assert!(is_public_ipv4("120.24.78.129"));
        assert!(is_public_ipv4("223.104.3.7"));
        assert!(!is_public_ipv4("unknown"));
        assert!(!is_public_ipv4(""));
        assert!(!is_public_ipv4("127.0.0.1")); // 回环
        assert!(!is_public_ipv4("10.0.8.13")); // A 内网
        assert!(!is_public_ipv4("172.16.0.1")); // B 内网（16..=31）
        assert!(is_public_ipv4("172.32.0.1")); // B 段之外（32）→ 公网
        assert!(!is_public_ipv4("192.168.1.1")); // C 内网
        assert!(!is_public_ipv4("169.254.1.1")); // 链路本地
        assert!(!is_public_ipv4("::1")); // IPv6 暂不支持
        assert!(!is_public_ipv4("0.1.2.3")); // 保留段
    }

    #[test]
    fn parse_xdb_region_name_prefers_city_then_province() {
        assert_eq!(parse_xdb_region_name("中国|0|山东省|青岛市|电信"), Some("青岛市"));
        assert_eq!(parse_xdb_region_name("中国|0|北京市|北京市|联通"), Some("北京市"));
        // 直辖市城市段可能为空/0 → 回落省份段
        assert_eq!(parse_xdb_region_name("中国|0|北京市||联通"), Some("北京市"));
        assert_eq!(parse_xdb_region_name("中国|0|天津市|0|电信"), Some("天津市"));
        // 内网 IP 的 xdb 原始返回：全 0 → None
        assert_eq!(parse_xdb_region_name("0|0|0|内网IP|内网IP"), None);
        assert_eq!(parse_xdb_region_name(""), None);
    }

    /// 构造一个最小合成 xdb：向量索引 (1,1) 单元指向单个段
    /// 1.1.0.0–1.1.255.255 → 「泰国|0|四色菊府|0|TOT」。
    fn synthetic_xdb() -> Vec<u8> {
        let mut buf = vec![0u8; XDB_VECTOR_INDEX_END];
        let record = "泰国|0|四色菊府|0|TOT".as_bytes();
        let seg_ptr = XDB_VECTOR_INDEX_END;
        let data_ptr = seg_ptr + XDB_SEGMENT_LEN;
        // 向量索引单元 (1,1)
        let cell = XDB_HEADER_LEN + (1 * 256 + 1) * 8;
        buf[cell..cell + 4].copy_from_slice(&(seg_ptr as u32).to_le_bytes());
        buf[cell + 4..cell + 8]
            .copy_from_slice(&((seg_ptr + XDB_SEGMENT_LEN) as u32).to_le_bytes());
        // 段索引：start/end 为大端，dataLen/dataPtr 为小端
        let mut seg = Vec::with_capacity(XDB_SEGMENT_LEN);
        seg.extend_from_slice(&[1, 1, 0, 0]);
        seg.extend_from_slice(&[1, 1, 255, 255]);
        seg.extend_from_slice(&(record.len() as u16).to_le_bytes());
        seg.extend_from_slice(&(data_ptr as u32).to_le_bytes());
        buf.extend_from_slice(&seg);
        buf.extend_from_slice(record);
        buf
    }

    #[test]
    fn xdb_search_hits_and_misses_on_synthetic_file() {
        let buf = synthetic_xdb();
        assert_eq!(
            xdb_search(&buf, "1.1.1.1").as_deref(),
            Some("泰国|0|四色菊府|0|TOT")
        );
        assert_eq!(xdb_search(&buf, "1.1.0.0").as_deref(), Some("泰国|0|四色菊府|0|TOT"));
        assert_eq!(xdb_search(&buf, "1.2.3.4"), None);
        assert_eq!(xdb_search(&buf, "8.8.8.8"), None);
        assert_eq!(xdb_search(&buf, "not-an-ip"), None);
    }

    #[test]
    fn parse_xdb_country_reads_first_segment() {
        assert_eq!(parse_xdb_country("中国|0|山东省|青岛市|电信"), Some("中国"));
        assert_eq!(parse_xdb_country("美国|0|加利福尼亚州|洛杉矶|AT&T"), Some("美国"));
        assert_eq!(parse_xdb_country("日本|0|东京都|东京|NTT"), Some("日本"));
        assert_eq!(parse_xdb_country("0|0|0|内网IP|内网IP"), None);
        assert_eq!(parse_xdb_country(""), None);
    }
}