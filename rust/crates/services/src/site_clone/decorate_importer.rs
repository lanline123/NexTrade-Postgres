//! 站点搬家 P2 导入引擎（site_clone::decorate_importer）：装修导入、
//! 素材服务端转存与 URL 全量重写。
//!
//! 执行口径（设计报告第八节 P2）：
//! - 装修页一律新建为**未发布**页（status=0）、非首页，父子页与 H5 绑定
//!   按 P1 的 import_map 映射重建；页面 JSON 内的商品/分类引用走映射
//!   重写，映射不到的置空并在任务报告中汇总列明；
//! - 素材转存 = 导入端服务端下载源文件 → 上传本站存储（`storage` 全局
//!   后端）→ 素材库落本地行；转存映射落 `site_clone_asset_map`，同源文件
//!   只转存一次、跨任务复用；
//! - URL 全量重写覆盖：本批新导入的装修 JSON、该来源已导入商品的媒体
//!   字段（主图/图库/属性图/视频/详情 HTML）、P1 登记的 external 素材行、
//!   白名单设置组/块中的图片 URL；
//! - 内容里出现的第三方外链（主机既非源站、也非源素材主机）不转存，
//!   防止把外部站点资源误搬进本站。

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

use nextrade_common::utils;
use nextrade_core::models::site_clone::clone::{
    CloneAssetsPackage, CloneDecoratePackage, CloneExecuteDTO, CloneManifestVO, CloneSourceCred,
    DOMAIN_ASSETS, DOMAIN_DECORATE,
};
use nextrade_core::models::system::config_basic::find_endpoint;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::storage;

use super::clone::SiteCloneService;
use super::importer::{fetch_json, CloneSource};
use super::whitelist;

/// 单个转存文件大小上限（64 MiB，视频素材余量）。
const REHOST_MAX_BYTES: usize = 64 * 1024 * 1024;

/// 引用映射（源 id → 本站 id），来自 `site_clone_import_map`（含历史任务）。
#[derive(Debug, Default)]
pub struct RefMaps {
    pub products: HashMap<i32, i32>,
    pub categories: HashMap<i32, i32>,
    pub brands: HashMap<i32, i32>,
}

/// JSON 变换上下文：引用映射 + URL 映射 + 未对齐引用收集。
pub struct TransformCtx<'a> {
    pub refs: &'a RefMaps,
    /// (源 URL, 本站 URL)，按源 URL 长度降序（防前缀互相吞）。
    pub urls: &'a [(String, String)],
    /// 未对齐引用：(类别 product/category, 源 id)。
    pub unmapped: Vec<(String, i32)>,
}

const PRODUCT_ID_KEYS: [&str; 2] = ["productId", "goodsId"];
const CATEGORY_ID_KEYS: [&str; 3] = ["categoryId", "productCategoryId", "catId"];
const PRODUCT_IDS_KEYS: [&str; 1] = ["productIds"];
const CATEGORY_IDS_KEYS: [&str; 2] = ["categoryIds", "childCategoryIds"];

impl SiteCloneService {
    // ———————————————————— 编排 ————————————————————

    /// P2 编排（在 P1 的设置/商品之后执行）：拉装修包 → 素材转存 →
    /// 候选 URL 补转存 → 全站重写 → 装修导入。源站为 v1（无 P2 端点）
    /// 时对应域记一条说明并跳过，不影响其他域。
    #[allow(clippy::too_many_arguments)]
    pub async fn run_p2(
        &self,
        dto: &CloneExecuteDTO,
        src: &CloneSource,
        manifest: &CloneManifestVO,
        job_id: i32,
        target_shop: i32,
        counts: &mut Map<String, Value>,
        failures: &mut Vec<Value>,
    ) -> Result<()> {
        let cred = &src.cred;
        let want_decorate = dto.import_decorate.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_DECORATE);
        let want_assets = dto.import_assets.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_ASSETS);
        if !want_decorate && !want_assets {
            return Ok(());
        }
        let refs = self.load_ref_maps(&cred.fingerprint).await?;

        let decorate_pkg = if want_decorate {
            match fetch_decorate(src).await {
                Ok(p) => Some(p),
                Err(e) if is_endpoint_missing(&e) => {
                    failures.push(json!({
                        "domain": "decorate", "sourceId": 0, "name": "",
                        "error": "来源站版本过低，不支持装修域（需导出版本 v2），已跳过",
                    }));
                    None
                }
                Err(e) => {
                    failures.push(json!({
                        "domain": "decorate", "sourceId": 0, "name": "", "error": e.to_string(),
                    }));
                    None
                }
            }
        } else {
            None
        };

        let mut url_map: HashMap<String, String> = HashMap::new();
        let mut failed_urls: HashSet<String> = HashSet::new();
        if want_assets {
            match fetch_assets(src).await {
                Ok(pkg) => {
                    self.import_assets_library(
                        &pkg, cred, job_id, target_shop, &mut url_map, counts, failures,
                        &mut failed_urls,
                    )
                    .await;
                }
                Err(e) if is_endpoint_missing(&e) => {
                    failures.push(json!({
                        "domain": "asset", "sourceId": 0, "name": "",
                        "error": "来源站版本过低，不支持素材域（需导出版本 v2），已跳过",
                    }));
                }
                Err(e) => failures.push(json!({
                    "domain": "asset", "sourceId": 0, "name": "", "error": e.to_string(),
                })),
            }
            // 内容引用但不在素材库的 URL 补转存（限源站/源素材主机，防误搬第三方）。
            self.rehost_content_candidates(
                cred, job_id, target_shop, decorate_pkg.as_ref(), &mut url_map, failures,
                &mut failed_urls,
            )
            .await;
            // P1 登记的 external 素材（商品图）无条件补转存。
            let external_rows = self
                .decorate_repo()
                .list_external_pics(target_shop)
                .await
                .unwrap_or_default();
            for (_, url, thumb) in &external_rows {
                self.rehost_url(&cred.fingerprint, job_id, url, &mut url_map, failures, &mut failed_urls)
                    .await;
                if !thumb.is_empty() && thumb != url {
                    self.rehost_url(&cred.fingerprint, job_id, thumb, &mut url_map, failures, &mut failed_urls)
                        .await;
                }
            }
            let sorted = sorted_url_pairs(&url_map);
            self.rewrite_imported_products(&cred.fingerprint, &sorted, counts).await;
            self.rewrite_settings(&sorted, counts).await;
            // external 素材行本身换本地地址。
            let mut pics_localized = 0i64;
            for (pic_id, url, thumb) in &external_rows {
                if let Some(local) = url_map.get(url) {
                    let local_thumb = url_map
                        .get(thumb)
                        .cloned()
                        .unwrap_or_else(|| local.clone());
                    if self
                        .decorate_repo()
                        .update_pic_local(*pic_id, local, &local_thumb)
                        .await
                        .is_ok()
                    {
                        pics_localized += 1;
                    }
                }
            }
            if pics_localized > 0 {
                counts.insert("externalPicsLocalized".into(), json!(pics_localized));
            }
        }

        if let Some(pkg) = &decorate_pkg {
            let sorted = sorted_url_pairs(&url_map);
            self.import_decorate(
                pkg, dto, cred, job_id, target_shop, &refs, &sorted, counts, failures,
            )
            .await;
        }
        Ok(())
    }

    async fn load_ref_maps(&self, fingerprint: &str) -> Result<RefMaps> {
        let repo = self.repo();
        let mut refs = RefMaps::default();
        for m in repo.list_maps_by_fingerprint_domain(fingerprint, "product").await? {
            refs.products.insert(m.source_id, m.target_id);
        }
        for m in repo.list_maps_by_fingerprint_domain(fingerprint, "category").await? {
            refs.categories.insert(m.source_id, m.target_id);
        }
        for m in repo.list_maps_by_fingerprint_domain(fingerprint, "brand").await? {
            refs.brands.insert(m.source_id, m.target_id);
        }
        Ok(refs)
    }

    // ———————————————————— 素材转存 ————————————————————

    /// 转存单个 URL：已转存过的直接命中映射；否则下载 → 上传本站存储 →
    /// 落转存映射。返回本站 URL（失败记 failures 并返回 None）。
    /// `url_map` 同步累积（供后续重写使用）。
    async fn rehost_url(
        &self,
        fingerprint: &str,
        job_id: i32,
        source_url: &str,
        url_map: &mut HashMap<String, String>,
        failures: &mut Vec<Value>,
        failed_urls: &mut HashSet<String>,
    ) -> Option<String> {
        let url = source_url.trim();
        if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
            return None;
        }
        if let Some(local) = url_map.get(url) {
            return Some(local.clone());
        }
        if failed_urls.contains(url) {
            return None;
        }
        let repo = self.decorate_repo();
        if let Ok(Some(m)) = repo.find_asset_map(fingerprint, url).await {
            url_map.insert(url.to_string(), m.local_url.clone());
            return Some(m.local_url);
        }
        let outcome = download_and_store(url).await;
        match outcome {
            Ok(local) => {
                let kind = media_kind_of(url);
                let now = utils::now_seconds() as i32;
                let _ = repo
                    .insert_asset_map(fingerprint, url, &local, kind, 0, job_id, now)
                    .await;
                url_map.insert(url.to_string(), local.clone());
                Some(local)
            }
            Err(e) => {
                failed_urls.insert(url.to_string());
                failures.push(json!({
                    "domain": "asset", "sourceId": 0, "name": url, "error": e.to_string(),
                }));
                None
            }
        }
    }

    /// 素材库导入：分组按名对齐（父先于子）、图片/视频逐条转存后落本地行。
    #[allow(clippy::too_many_arguments)]
    async fn import_assets_library(
        &self,
        pkg: &CloneAssetsPackage,
        cred: &CloneSourceCred,
        job_id: i32,
        target_shop: i32,
        url_map: &mut HashMap<String, String>,
        counts: &mut Map<String, Value>,
        failures: &mut Vec<Value>,
        failed_urls: &mut HashSet<String>,
    ) {
        let repo = self.decorate_repo();
        let fp = &cred.fingerprint;
        let now = utils::now_seconds() as i32;
        let add_time = now.to_string();

        // —— 图片分组：按（名, 父）对齐，父先于子 ——
        let mut group_target: HashMap<i32, i32> = HashMap::new();
        let mut pending: Vec<_> = pkg.groups.iter().collect();
        for _ in 0..12 {
            if pending.is_empty() {
                break;
            }
            let mut rest = Vec::new();
            for g in pending {
                let parent_target = if g.parent_id == 0 {
                    Some(0)
                } else {
                    group_target.get(&g.parent_id).copied()
                };
                let Some(parent_target) = parent_target else {
                    rest.push(g);
                    continue;
                };
                let id = match repo
                    .find_gallery_group(target_shop, &g.gallery_name, parent_target)
                    .await
                {
                    Ok(Some(id)) => id,
                    _ => {
                        let thumb = if g.gallery_thumb.starts_with("http") {
                            self.rehost_url(fp, job_id, &g.gallery_thumb, url_map, failures, failed_urls)
                                .await
                                .unwrap_or_else(|| g.gallery_thumb.clone())
                        } else {
                            g.gallery_thumb.clone()
                        };
                        match repo
                            .insert_gallery_group(
                                target_shop,
                                parent_target,
                                &g.gallery_name,
                                g.gallery_sort,
                                &thumb,
                            )
                            .await
                        {
                            Ok(id) => {
                                bump(counts, "assetGroupsCreated");
                                id
                            }
                            Err(_) => continue,
                        }
                    }
                };
                group_target.insert(g.gallery_id, id);
            }
            pending = rest;
        }

        // —— 图片：转存后落本地行（已落过库的同源图跳过，防重复行） ——
        for pic in &pkg.pics {
            let existing = repo.find_asset_map(fp, &pic.pic_url).await.ok().flatten();
            let local = match &existing {
                Some(m) => {
                    url_map.insert(pic.pic_url.clone(), m.local_url.clone());
                    Some(m.local_url.clone())
                }
                None => {
                    self.rehost_url(fp, job_id, &pic.pic_url, url_map, failures, failed_urls)
                        .await
                }
            };
            let Some(local_url) = local else { continue };
            if let Some(m) = &existing {
                if m.target_pic_id > 0 {
                    bump(counts, "assetsSkipped");
                    continue;
                }
            }
            let local_thumb = if pic.pic_thumb.is_empty() || pic.pic_thumb == pic.pic_url {
                local_url.clone()
            } else if let Some(t) = url_map.get(&pic.pic_thumb) {
                t.clone()
            } else {
                self.rehost_url(fp, job_id, &pic.pic_thumb, url_map, failures, failed_urls)
                    .await
                    .unwrap_or_else(|| local_url.clone())
            };
            let gallery_id = group_target.get(&pic.gallery_id).copied().unwrap_or(0);
            match repo
                .insert_gallery_pic(
                    target_shop,
                    gallery_id,
                    &local_url,
                    &pic.pic_name,
                    &local_thumb,
                    pic.pic_width,
                    pic.pic_height,
                    pic.pic_size,
                    pic.file_type.as_deref(),
                    pic.pic_sort,
                    &add_time,
                )
                .await
            {
                Ok(pic_id) => {
                    let _ = repo.set_asset_map_pic(fp, &pic.pic_url, pic_id).await;
                    bump(counts, "assetPicsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "asset", "sourceId": pic.pic_id, "name": pic.pic_name,
                    "error": e.to_string(),
                })),
            }
        }

        // —— 视频分组：按（名, 父）对齐 ——
        let mut vgroup_target: HashMap<i32, i32> = HashMap::new();
        let mut pending: Vec<_> = pkg.video_groups.iter().collect();
        for _ in 0..12 {
            if pending.is_empty() {
                break;
            }
            let mut rest = Vec::new();
            for g in pending {
                let parent_target = if g.parent_id == 0 {
                    Some(0)
                } else {
                    vgroup_target.get(&g.parent_id).copied()
                };
                let Some(parent_target) = parent_target else {
                    rest.push(g);
                    continue;
                };
                let id = match repo.find_video_group(target_shop, &g.name, parent_target).await {
                    Ok(Some(id)) => id,
                    _ => match repo
                        .insert_video_group(target_shop, parent_target, &g.name, g.sort, now)
                        .await
                    {
                        Ok(id) => {
                            bump(counts, "assetGroupsCreated");
                            id
                        }
                        Err(_) => continue,
                    },
                };
                vgroup_target.insert(g.id, id);
            }
            pending = rest;
        }

        // —— 视频：转存后落本地行 ——
        for v in &pkg.videos {
            let Some(local_url) = self
                .rehost_url(fp, job_id, &v.video_url, url_map, failures, failed_urls)
                .await
            else {
                continue;
            };
            let cover = match v.video_cover.as_deref().filter(|s| !s.is_empty()) {
                Some(c) => self
                    .rehost_url(fp, job_id, c, url_map, failures, failed_urls)
                    .await
                    .or_else(|| Some(c.to_string())),
                None => None,
            };
            let frame = match v.video_first_frame.as_deref().filter(|s| !s.is_empty()) {
                Some(f) => self
                    .rehost_url(fp, job_id, f, url_map, failures, failed_urls)
                    .await
                    .or_else(|| Some(f.to_string())),
                None => None,
            };
            let gallery_id = vgroup_target.get(&v.gallery_id).copied().unwrap_or(0);
            match repo
                .insert_video_info(
                    target_shop,
                    gallery_id,
                    &local_url,
                    &v.video_name,
                    cover.as_deref(),
                    v.format.as_deref(),
                    frame.as_deref(),
                    v.duration.as_deref(),
                    v.size.as_deref(),
                    v.description.as_deref(),
                    now,
                )
                .await
            {
                Ok(_) => bump(counts, "assetVideosCreated"),
                Err(e) => failures.push(json!({
                    "domain": "asset", "sourceId": v.id, "name": v.video_name,
                    "error": e.to_string(),
                })),
            }
        }
    }

    /// 内容候选 URL 补转存：从装修包文本、已导入商品的详情 HTML、设置
    /// 组/块现值中抽取 URL，仅转存主机属于源站或源素材集合的（内容外链
    /// 口径，见文件头）。商品媒体字段本身在 external/商品重写环节处理。
    async fn rehost_content_candidates(
        &self,
        cred: &CloneSourceCred,
        job_id: i32,
        target_shop: i32,
        decorate_pkg: Option<&CloneDecoratePackage>,
        url_map: &mut HashMap<String, String>,
        failures: &mut Vec<Value>,
        failed_urls: &mut HashSet<String>,
    ) {
        let _ = target_shop;
        let mut allowed_hosts: HashSet<String> = HashSet::new();
        allowed_hosts.insert(url_host(&cred.origin));
        for src in url_map.keys() {
            allowed_hosts.insert(url_host(src));
        }
        let mut candidates: HashSet<String> = HashSet::new();
        let mut collect = |text: &str, candidates: &mut HashSet<String>| {
            for u in extract_urls(text) {
                if allowed_hosts.contains(&url_host(&u)) {
                    candidates.insert(u);
                }
            }
        };
        if let Some(pkg) = decorate_pkg {
            for p in &pkg.pages {
                if let Some(d) = &p.data {
                    collect(d, &mut candidates);
                }
                if let Some(d) = &p.draft_data {
                    collect(d, &mut candidates);
                }
                if let Some(s) = &p.share_info {
                    collect(s, &mut candidates);
                }
            }
            for d in &pkg.discrete {
                collect(&d.data, &mut candidates);
            }
            for n in &pkg.navigations {
                collect(&n.link, &mut candidates);
                collect(&n.icon, &mut candidates);
                collect(&n.qr_image, &mut candidates);
            }
            for f in &pkg.cat_floors {
                collect(&f.floor_ico, &mut candidates);
            }
            for n in &pkg.mobile_cat_navs {
                collect(&n.img_url, &mut candidates);
            }
        }
        // 已导入商品的详情 HTML（图片字段在商品重写环节按映射替换，
        // 这里只补详情内嵌图）。
        if let Ok(maps) = self
            .repo()
            .list_maps_by_fingerprint_domain(&cred.fingerprint, "product")
            .await
        {
            for m in maps {
                if let Ok(Some(media)) = self.decorate_repo().load_product_media(m.target_id).await {
                    if let Some(desc) = &media.product_desc {
                        collect(desc, &mut candidates);
                    }
                }
            }
        }
        // 设置组/块现值（P1 已覆盖进本站的源站图片）。
        for rule in whitelist::GROUP_RULES {
            if let Some(spec) = find_endpoint(rule.get_path) {
                if let Ok(v) = self.config_basic.get(spec, None).await {
                    collect(&v.to_string(), &mut candidates);
                }
            }
        }
        for code in whitelist::JSON_BLOCK_CODES.iter().chain(whitelist::RAW_BLOCK_CODES.iter()) {
            if let Ok(Some(raw)) = self.config_repo().find_by_code(code).await {
                collect(&raw, &mut candidates);
            }
        }
        for u in candidates {
            if !url_map.contains_key(&u) {
                self.rehost_url(&cred.fingerprint, job_id, &u, url_map, failures, failed_urls)
                    .await;
            }
        }
    }

    // ———————————————————— URL 全量重写 ————————————————————

    /// 已导入商品的媒体字段重写（主图四件套/图库/属性图/视频/详情 HTML）。
    async fn rewrite_imported_products(
        &self,
        fingerprint: &str,
        url_pairs: &[(String, String)],
        counts: &mut Map<String, Value>,
    ) {
        if url_pairs.is_empty() {
            return;
        }
        let Ok(maps) = self
            .repo()
            .list_maps_by_fingerprint_domain(fingerprint, "product")
            .await
        else {
            return;
        };
        let repo = self.decorate_repo();
        let mut changed_products = 0i64;
        for m in maps {
            let pid = m.target_id;
            let mut changed = false;
            if let Ok(Some(mut media)) = repo.load_product_media(pid).await {
                let before = media.clone();
                media.pic_url = apply_url_map(&media.pic_url, url_pairs);
                media.pic_thumb = apply_url_map(&media.pic_thumb, url_pairs);
                media.pic_original = apply_url_map(&media.pic_original, url_pairs);
                media.pic_large = apply_url_map(&media.pic_large, url_pairs);
                media.product_desc = media
                    .product_desc
                    .map(|d| apply_url_map(&d, url_pairs));
                if media.pic_url != before.pic_url
                    || media.pic_thumb != before.pic_thumb
                    || media.pic_original != before.pic_original
                    || media.pic_large != before.pic_large
                    || media.product_desc != before.product_desc
                {
                    if repo.update_product_media(pid, &media).await.is_ok() {
                        changed = true;
                    }
                }
            }
            if let Ok(rows) = repo.list_product_gallery_media(pid).await {
                for mut row in rows {
                    let before = row.clone();
                    row.pic_url = apply_url_map(&row.pic_url, url_pairs);
                    row.pic_thumb = apply_url_map(&row.pic_thumb, url_pairs);
                    row.pic_original = apply_url_map(&row.pic_original, url_pairs);
                    row.pic_large = apply_url_map(&row.pic_large, url_pairs);
                    if row.pic_url != before.pic_url
                        || row.pic_thumb != before.pic_thumb
                        || row.pic_original != before.pic_original
                        || row.pic_large != before.pic_large
                    {
                        if repo.update_product_gallery_media(&row).await.is_ok() {
                            changed = true;
                        }
                    }
                }
            }
            if let Ok(rows) = repo.list_product_attr_pics(pid).await {
                for mut row in rows {
                    let before = row.clone();
                    row.attr_pic = row.attr_pic.map(|s| apply_url_map(&s, url_pairs));
                    row.attr_pic_thumb =
                        row.attr_pic_thumb.map(|s| apply_url_map(&s, url_pairs));
                    if row.attr_pic != before.attr_pic
                        || row.attr_pic_thumb != before.attr_pic_thumb
                    {
                        if repo.update_product_attr_pics(&row).await.is_ok() {
                            changed = true;
                        }
                    }
                }
            }
            if let Ok(rows) = repo.list_product_video_media(pid).await {
                for mut row in rows {
                    let before = row.clone();
                    row.video_url = apply_url_map(&row.video_url, url_pairs);
                    row.video_cover = apply_url_map(&row.video_cover, url_pairs);
                    if row.video_url != before.video_url
                        || row.video_cover != before.video_cover
                    {
                        if repo.update_product_video_media(&row).await.is_ok() {
                            changed = true;
                        }
                    }
                }
            }
            if changed {
                changed_products += 1;
            }
        }
        if changed_products > 0 {
            counts.insert("productsMediaRewritten".into(), json!(changed_products));
        }
    }

    /// 白名单设置组/块现值的 URL 重写（只在确有替换时才回写）。
    async fn rewrite_settings(
        &self,
        url_pairs: &[(String, String)],
        counts: &mut Map<String, Value>,
    ) {
        if url_pairs.is_empty() {
            return;
        }
        let mut changed = 0i64;
        for rule in whitelist::GROUP_RULES {
            let (Some(get_spec), Some(save_spec)) = (
                find_endpoint(rule.get_path),
                find_endpoint(rule.save_path),
            ) else {
                continue;
            };
            let Ok(current) = self.config_basic.get(get_spec, None).await else {
                continue;
            };
            let raw = current.to_string();
            let replaced = apply_url_map(&raw, url_pairs);
            if replaced != raw {
                if let Ok(Value::Object(body)) = serde_json::from_str::<Value>(&replaced) {
                    if self.config_basic.save(save_spec, &body).await.is_ok() {
                        changed += 1;
                    }
                }
            }
        }
        for code in whitelist::JSON_BLOCK_CODES.iter().chain(whitelist::RAW_BLOCK_CODES.iter()) {
            if let Ok(Some(raw)) = self.config_repo().find_by_code(code).await {
                let replaced = apply_url_map(&raw, url_pairs);
                if replaced != raw
                    && self.config_repo().save_code(code, &replaced).await.is_ok()
                {
                    changed += 1;
                }
            }
        }
        if changed > 0 {
            counts.insert("settingsUrlsRewritten".into(), json!(changed));
        }
    }

    // ———————————————————— 装修导入 ————————————————————

    #[allow(clippy::too_many_arguments)]
    async fn import_decorate(
        &self,
        pkg: &CloneDecoratePackage,
        dto: &CloneExecuteDTO,
        cred: &CloneSourceCred,
        job_id: i32,
        target_shop: i32,
        refs: &RefMaps,
        url_pairs: &[(String, String)],
        counts: &mut Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let repo = self.repo();
        let deco = self.decorate_repo();
        let fp = &cred.fingerprint;
        let now = utils::now_seconds() as i32;
        let force = dto.force_duplicate.unwrap_or(false);
        let mut unmapped_agg: HashMap<(String, i32), i32> = HashMap::new();

        // —— 页面：先全量落库（关系占位），再回填父子/绑定 ——
        let mut page_target: HashMap<i32, i32> = HashMap::new();
        if let Ok(maps) = repo.list_maps_by_fingerprint_domain(fp, "decorate").await {
            for m in maps {
                page_target.insert(m.source_id, m.target_id);
            }
        }
        let mut inserted: Vec<i32> = Vec::new();
        for page in &pkg.pages {
            if !force && page_target.contains_key(&page.decorate_id) {
                bump(counts, "decorateSkipped");
                continue;
            }
            let raw_content = page
                .data
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .or(page.draft_data.as_deref())
                .unwrap_or("");
            let content = if raw_content.is_empty() {
                String::new()
            } else {
                let mut ctx = TransformCtx { refs, urls: url_pairs, unmapped: Vec::new() };
                let out = transform_json_text(raw_content, &mut ctx);
                absorb_unmapped(&ctx, &mut unmapped_agg);
                out
            };
            let share = page.share_info.as_deref().map(|s| {
                let mut ctx = TransformCtx { refs, urls: url_pairs, unmapped: Vec::new() };
                let out = transform_json_text(s, &mut ctx);
                absorb_unmapped(&ctx, &mut unmapped_agg);
                out
            });
            let title = page
                .decorate_title
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("导入页面");
            let dtype = page.decorate_type.filter(|t| matches!(t, 1 | 2 | 3)).unwrap_or(1);
            let content_opt = if content.is_empty() { None } else { Some(content.as_str()) };
            match deco
                .insert_decorate_page(
                    title,
                    content_opt,
                    content_opt,
                    dtype,
                    page.page_type,
                    target_shop,
                    page.locale_id.unwrap_or(0),
                    page.sort_order,
                    share.as_deref(),
                    now,
                )
                .await
            {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fp, "decorate", page.decorate_id, title, new_id, job_id, now)
                        .await;
                    page_target.insert(page.decorate_id, new_id);
                    inserted.push(page.decorate_id);
                    bump(counts, "decorateCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "decorate", "sourceId": page.decorate_id, "name": title,
                    "error": e.to_string(),
                })),
            }
        }
        for src_id in &inserted {
            let Some(page) = pkg.pages.iter().find(|p| p.decorate_id == *src_id) else {
                continue;
            };
            let Some(&new_id) = page_target.get(src_id) else { continue };
            let parent_t = page
                .parent_id
                .filter(|p| *p > 0)
                .and_then(|p| page_target.get(&p).copied())
                .unwrap_or(0);
            let bind_t = page
                .binded_h5_id
                .filter(|p| *p > 0)
                .and_then(|p| page_target.get(&p).copied())
                .unwrap_or(0);
            if parent_t > 0 || bind_t > 0 {
                let _ = deco.update_decorate_links(new_id, parent_t, bind_t).await;
            }
        }

        // —— 离散组件：本站已有同 sn 的不覆盖，跳过并明示 ——
        for d in &pkg.discrete {
            if !force {
                if let Ok(Some(_)) = repo.find_map(fp, "discrete", d.id).await {
                    bump(counts, "discreteSkipped");
                    continue;
                }
            }
            if let Ok(Some(_)) = deco.find_discrete(target_shop, &d.decorate_sn).await {
                failures.push(json!({
                    "domain": "discrete", "sourceId": d.id, "name": d.decorate_sn,
                    "error": "本站已有同名离散组件，已跳过（不覆盖本站配置）",
                }));
                bump(counts, "discreteSkipped");
                continue;
            }
            let mut ctx = TransformCtx { refs, urls: url_pairs, unmapped: Vec::new() };
            let data = transform_json_text(&d.data, &mut ctx);
            absorb_unmapped(&ctx, &mut unmapped_agg);
            match deco
                .insert_discrete(target_shop, &d.decorate_sn, d.decorate_name.as_deref(), &data)
                .await
            {
                Ok(id) => {
                    let _ = repo
                        .insert_map(fp, "discrete", d.id, &d.decorate_sn, id, job_id, now)
                        .await;
                    bump(counts, "discreteCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "discrete", "sourceId": d.id, "name": d.decorate_sn,
                    "error": e.to_string(),
                })),
            }
        }

        // —— PC 导航：父先于子 ——
        let mut nav_target: HashMap<i32, i32> = HashMap::new();
        if let Ok(maps) = repo.list_maps_by_fingerprint_domain(fp, "pc_navigation").await {
            for m in maps {
                nav_target.insert(m.source_id, m.target_id);
            }
        }
        let mut pending: Vec<_> = pkg
            .navigations
            .iter()
            .filter(|n| force || !nav_target.contains_key(&n.id))
            .collect();
        let skipped_nav = pkg.navigations.len() - pending.len();
        if skipped_nav > 0 {
            counts.insert(
                "navigationsSkipped".into(),
                json!(i64::try_from(skipped_nav).unwrap_or(0)),
            );
        }
        for _ in 0..12 {
            if pending.is_empty() {
                break;
            }
            let mut rest = Vec::new();
            for n in pending {
                let parent_t = if n.parent_id == 0 {
                    Some(0)
                } else {
                    nav_target.get(&n.parent_id).copied()
                };
                let Some(parent_t) = parent_t else {
                    rest.push(n);
                    continue;
                };
                let mut ctx = TransformCtx { refs, urls: url_pairs, unmapped: Vec::new() };
                let link = transform_text(&n.link, &mut ctx);
                let icon = transform_text(&n.icon, &mut ctx);
                let qr = transform_text(&n.qr_image, &mut ctx);
                absorb_unmapped(&ctx, &mut unmapped_agg);
                match deco.insert_navigation(n, parent_t, &link, &icon, &qr).await {
                    Ok(id) => {
                        let _ = repo
                            .insert_map(fp, "pc_navigation", n.id, &n.title, id, job_id, now)
                            .await;
                        nav_target.insert(n.id, id);
                        bump(counts, "navigationsCreated");
                    }
                    Err(e) => failures.push(json!({
                        "domain": "pc_navigation", "sourceId": n.id, "name": n.title,
                        "error": e.to_string(),
                    })),
                }
            }
            pending = rest;
        }
        for n in &pending {
            failures.push(json!({
                "domain": "pc_navigation", "sourceId": n.id, "name": n.title,
                "error": "父导航未能对齐，已跳过",
            }));
        }

        // —— PC 分类楼层 ——
        for f in &pkg.cat_floors {
            if !force {
                if let Ok(Some(_)) = repo.find_map(fp, "pc_cat_floor", f.cat_floor_id).await {
                    bump(counts, "catFloorsSkipped");
                    continue;
                }
            }
            let ids = parse_id_array(&f.category_ids);
            let names = parse_string_array(&f.category_names);
            let mut kept_ids: Vec<i32> = Vec::new();
            let mut kept_names: Vec<String> = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                match refs.categories.get(id) {
                    Some(t) => {
                        kept_ids.push(*t);
                        if names.len() == ids.len() {
                            kept_names.push(names[i].clone());
                        }
                    }
                    None => {
                        *unmapped_agg.entry(("category".to_string(), *id)).or_default() += 1;
                    }
                }
            }
            let category_ids = Value::Array(kept_ids.iter().map(|i| json!(i)).collect()).to_string();
            let category_names = if names.len() == ids.len() {
                Value::Array(kept_names.iter().map(|s| json!(s)).collect()).to_string()
            } else {
                f.category_names.clone()
            };
            let brand_ids = remap_id_array(&f.brand_ids, &refs.brands, "brand", &mut unmapped_agg);
            let mut ctx = TransformCtx { refs, urls: url_pairs, unmapped: Vec::new() };
            let ico = transform_text(&f.floor_ico, &mut ctx);
            absorb_unmapped(&ctx, &mut unmapped_agg);
            match deco
                .insert_cat_floor(f, &category_ids, &category_names, &ico, &brand_ids)
                .await
            {
                Ok(id) => {
                    let _ = repo
                        .insert_map(fp, "pc_cat_floor", f.cat_floor_id, &f.cat_floor_name, id, job_id, now)
                        .await;
                    bump(counts, "catFloorsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "pc_cat_floor", "sourceId": f.cat_floor_id, "name": f.cat_floor_name,
                    "error": e.to_string(),
                })),
            }
        }

        // —— 移动分类导航：主分类未对齐整行跳过 ——
        for n in &pkg.mobile_cat_navs {
            if !force {
                if let Ok(Some(_)) = repo.find_map(fp, "mobile_cat_nav", n.mobile_cat_nav_id).await {
                    bump(counts, "mobileCatNavsSkipped");
                    continue;
                }
            }
            let Some(&cat_t) = refs.categories.get(&n.category_id) else {
                failures.push(json!({
                    "domain": "mobile_cat_nav", "sourceId": n.mobile_cat_nav_id, "name": n.cat_name_alias.clone().unwrap_or_default(),
                    "error": format!("主分类（源 id {}）未对齐，已跳过", n.category_id),
                }));
                continue;
            };
            let child = remap_id_array(&n.child_cat_ids, &refs.categories, "category", &mut unmapped_agg);
            let brands = remap_id_array(&n.brand_ids, &refs.brands, "brand", &mut unmapped_agg);
            let img = n
                .img_url
                .split('｜')
                .map(|part| apply_url_map(part, url_pairs))
                .collect::<Vec<_>>()
                .join("｜");
            match deco.insert_mobile_cat_nav(n, cat_t, &child, &brands, &img).await {
                Ok(id) => {
                    let _ = repo
                        .insert_map(fp, "mobile_cat_nav", n.mobile_cat_nav_id, "", id, job_id, now)
                        .await;
                    bump(counts, "mobileCatNavsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "mobile_cat_nav", "sourceId": n.mobile_cat_nav_id, "name": "",
                    "error": e.to_string(),
                })),
            }
        }

        // —— 未对齐引用汇总（置空明细进任务报告，不逐条刷屏） ——
        if !unmapped_agg.is_empty() {
            let total: i32 = unmapped_agg.values().sum();
            counts.insert("refsBlanked".into(), json!(i64::from(total)));
            for kind in ["product", "category", "brand"] {
                let mut ids: Vec<i32> = unmapped_agg
                    .iter()
                    .filter(|((k, _), _)| k == kind)
                    .map(|((_, id), _)| *id)
                    .collect();
                if ids.is_empty() {
                    continue;
                }
                ids.sort_unstable();
                let label = match kind {
                    "product" => "商品",
                    "category" => "分类",
                    _ => "品牌",
                };
                failures.push(json!({
                    "domain": "decorate-ref", "sourceId": 0, "name": format!("{label}引用"),
                    "error": format!(
                        "装修数据中有 {} 处{label}引用未在导入映射中（源 id: {}），已置空；如需保留，请先导入对应{label}后重跑装修域",
                        ids.len(),
                        ids.iter().take(20).map(i32::to_string).collect::<Vec<_>>().join(", "),
                    ),
                }));
            }
        }
    }
}

// ———————————————————— 源站拉取 ————————————————————

async fn fetch_decorate(src: &CloneSource) -> Result<CloneDecoratePackage> {
    let data = fetch_json(src, "decorate", "/api/clone/v1/decorate").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站装修包解析失败: {e}")))
}

async fn fetch_assets(src: &CloneSource) -> Result<CloneAssetsPackage> {
    let data = fetch_json(src, "assets", "/api/clone/v1/assets").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站素材包解析失败: {e}")))
}

/// 源站无此端点（v1 导出端）的判定：remote_get 的错误文案含 HTTP 状态。
pub(super) fn is_endpoint_missing(e: &Error) -> bool {
    e.to_string().contains("HTTP 404")
}

// ———————————————————— 转存下载 ————————————————————

/// 下载源文件并写入本站存储，返回本站 URL。
async fn download_and_store(source_url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| Error::Internal(format!("HTTP 客户端初始化失败: {e}")))?;
    let resp = client
        .get(source_url)
        .send()
        .await
        .map_err(|e| Error::Validation(format!("下载源文件失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Validation(format!(
            "下载源文件失败（HTTP {}）",
            resp.status()
        )));
    }
    if let Some(len) = resp.content_length() {
        if len > REHOST_MAX_BYTES as u64 {
            return Err(Error::Validation(format!(
                "源文件过大（{len} 字节，上限 {REHOST_MAX_BYTES}），已跳过"
            )));
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Error::Internal(format!("读取源文件失败: {e}")))?;
    if bytes.len() > REHOST_MAX_BYTES {
        return Err(Error::Validation("源文件过大，已跳过".into()));
    }
    let name = file_name_of(source_url);
    storage::save_local_upload("clone", &name, &bytes).await
}

/// 从 URL 取文件名（仅作扩展名来源，存储层另行生成落盘名）。
fn file_name_of(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').next().unwrap_or("");
    let clean: String = last
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    if clean.is_empty() || !clean.contains('.') {
        "asset.bin".to_string()
    } else {
        clean
    }
}

/// 媒体类别粗分（统计用）。
fn media_kind_of(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    let path = lower.split(['?', '#']).next().unwrap_or("");
    if path.ends_with(".mp4") || path.ends_with(".mov") || path.ends_with(".webm")
        || path.ends_with(".m4v")
    {
        "video"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") || path.ends_with(".png")
        || path.ends_with(".gif") || path.ends_with(".webp") || path.ends_with(".svg")
        || path.ends_with(".bmp") || path.ends_with(".ico")
    {
        "pic"
    } else {
        "file"
    }
}

// ———————————————————— JSON 变换（纯函数，可单测） ————————————————————

/// URL 映射对：按源 URL 长度降序排列（长串先替换，防前缀吞短串）。
pub fn sorted_url_pairs(map: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> =
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    pairs
}

/// 对文本做 URL 精确替换（映射内的源 URL → 本站 URL）。
pub fn apply_url_map(s: &str, url_pairs: &[(String, String)]) -> String {
    let mut out = s.to_string();
    for (src, local) in url_pairs {
        if out.contains(src.as_str()) {
            out = out.replace(src.as_str(), local.as_str());
        }
    }
    out
}

/// 纯文本变换：URL 替换 + 链接串内的 id 参数重写（导航 link 等）。
pub fn transform_text(s: &str, ctx: &mut TransformCtx<'_>) -> String {
    let mut out = apply_url_map(s, ctx.urls);
    out = rewrite_query_ids(&out, "productId", &ctx.refs.products, "product", &mut ctx.unmapped);
    out = rewrite_query_ids(&out, "categoryId", &ctx.refs.categories, "category", &mut ctx.unmapped);
    out = rewrite_query_ids(&out, "catId", &ctx.refs.categories, "category", &mut ctx.unmapped);
    out
}

/// 装修 JSON 文本变换：解析后递归重写；解析失败时只做 URL 文本替换兜底。
pub fn transform_json_text(raw: &str, ctx: &mut TransformCtx<'_>) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(mut v) => {
            transform_value(&mut v, ctx);
            v.to_string()
        }
        Err(_) => apply_url_map(raw, ctx.urls),
    }
}

/// 递归重写装修 JSON：已知 id 字段走引用映射（标量置空、数组剔除），
/// 其余字符串走 URL/链接变换，未知结构原样保留。
pub fn transform_value(v: &mut Value, ctx: &mut TransformCtx<'_>) {
    match v {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if PRODUCT_ID_KEYS.contains(&key.as_str()) {
                    *val = map_id_value(val, &ctx.refs.products, "product", &mut ctx.unmapped);
                } else if CATEGORY_ID_KEYS.contains(&key.as_str()) {
                    *val = map_id_value(val, &ctx.refs.categories, "category", &mut ctx.unmapped);
                } else if PRODUCT_IDS_KEYS.contains(&key.as_str()) {
                    *val = map_id_array(val, &ctx.refs.products, "product", &mut ctx.unmapped);
                } else if CATEGORY_IDS_KEYS.contains(&key.as_str()) {
                    *val = map_id_array(val, &ctx.refs.categories, "category", &mut ctx.unmapped);
                } else {
                    transform_value(val, ctx);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                transform_value(item, ctx);
            }
        }
        Value::String(s) => {
            *s = transform_text(s, ctx);
        }
        _ => {}
    }
}

/// 标量 id 映射：数字/数字字符串均可；未对齐 → 0 并记录。0/负数原样。
fn map_id_value(
    v: &Value,
    map: &HashMap<i32, i32>,
    kind: &str,
    unmapped: &mut Vec<(String, i32)>,
) -> Value {
    match v {
        Value::Number(n) => {
            let src = n.as_i64().and_then(|x| i32::try_from(x).ok()).unwrap_or(0);
            if src <= 0 {
                return v.clone();
            }
            match map.get(&src) {
                Some(t) => json!(t),
                None => {
                    unmapped.push((kind.to_string(), src));
                    json!(0)
                }
            }
        }
        Value::String(s) => {
            if let Ok(src) = s.trim().parse::<i32>() {
                if src <= 0 {
                    return v.clone();
                }
                match map.get(&src) {
                    Some(t) => Value::String(t.to_string()),
                    None => {
                        unmapped.push((kind.to_string(), src));
                        Value::String("0".to_string())
                    }
                }
            } else {
                v.clone()
            }
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|x| map_id_value(x, map, kind, unmapped))
                .collect(),
        ),
        _ => v.clone(),
    }
}

/// id 数组映射：未对齐条目剔除并记录；非 id 条目原样保留。
fn map_id_array(
    v: &Value,
    map: &HashMap<i32, i32>,
    kind: &str,
    unmapped: &mut Vec<(String, i32)>,
) -> Value {
    match v {
        Value::Array(arr) => Value::Array(
            arr.iter()
                .filter_map(|x| {
                    let id = match x {
                        Value::Number(n) => {
                            n.as_i64().and_then(|n| i32::try_from(n).ok())
                        }
                        Value::String(s) => s.trim().parse::<i32>().ok(),
                        _ => None,
                    };
                    match id {
                        Some(src) if src > 0 => match map.get(&src) {
                            Some(t) => Some(json!(t)),
                            None => {
                                unmapped.push((kind.to_string(), src));
                                None
                            }
                        },
                        Some(_) => None,
                        None => Some(x.clone()),
                    }
                })
                .collect(),
        ),
        _ => map_id_value(v, map, kind, unmapped),
    }
}

/// 链接串内 `key=<id>` 参数重写（如 `/pages/product/detail?productId=12`）。
fn rewrite_query_ids(
    text: &str,
    key: &str,
    map: &HashMap<i32, i32>,
    kind: &str,
    unmapped: &mut Vec<(String, i32)>,
) -> String {
    let pat = format!("{key}=");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(&pat) {
        out.push_str(&rest[..pos + pat.len()]);
        let after = &rest[pos + pat.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            rest = after;
            continue;
        }
        let src: i32 = digits.parse().unwrap_or(0);
        match map.get(&src) {
            Some(t) => out.push_str(&t.to_string()),
            None => {
                out.push('0');
                unmapped.push((kind.to_string(), src));
            }
        }
        rest = &after[digits.len()..];
    }
    out.push_str(rest);
    out
}

/// 从文本中抽取 http(s) URL（JSON/HTML 通用粗抽取，供候选转存）。
fn extract_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = text[from..].find("http") {
        let start = from + rel;
        let rest = &text[start..];
        if !rest.starts_with("http://") && !rest.starts_with("https://") {
            from = start + 4;
            continue;
        }
        let mut end = rest.len();
        for (idx, c) in rest.char_indices() {
            if c.is_whitespace()
                || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']' | '\\' | '|')
            {
                end = idx;
                break;
            }
        }
        let mut url = &rest[..end];
        while url.ends_with(',') || url.ends_with('}') || url.ends_with(';') {
            url = &url[..url.len() - 1];
        }
        if url.len() > 10 {
            out.push(url.to_string());
        }
        from = start + end.max(1);
    }
    out
}

/// URL 主机名（小写，不含端口）。
fn url_host(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or("");
    rest.split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_lowercase()
}

/// 宽容解析 id 数组存储串（JSON 数组 / 逗号分隔均可）。
fn parse_id_array(raw: &str) -> Vec<i32> {
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(arr)) => arr
            .iter()
            .filter_map(|x| match x {
                Value::Number(n) => n.as_i64().and_then(|n| i32::try_from(n).ok()),
                Value::String(s) => s.trim().parse::<i32>().ok(),
                _ => None,
            })
            .collect(),
        Ok(Value::Number(n)) => {
            n.as_i64().and_then(|n| i32::try_from(n).ok()).into_iter().collect()
        }
        _ => raw
            .split(',')
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect(),
    }
}

/// 宽容解析字符串数组存储串（分类名数组等）。
fn parse_string_array(raw: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(arr)) => arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// id 数组存储串重写（楼层品牌、子分类等）：未对齐剔除并汇总记录。
fn remap_id_array(
    raw: &str,
    map: &HashMap<i32, i32>,
    kind: &str,
    unmapped_agg: &mut HashMap<(String, i32), i32>,
) -> String {
    let ids = parse_id_array(raw);
    let kept: Vec<i32> = ids
        .iter()
        .filter_map(|id| match map.get(id) {
            Some(t) => Some(*t),
            None => {
                *unmapped_agg.entry((kind.to_string(), *id)).or_default() += 1;
                None
            }
        })
        .collect();
    Value::Array(kept.iter().map(|i| json!(i)).collect()).to_string()
}

/// 计数器 +1（与 importer 同约定）。
fn bump(counts: &mut Map<String, Value>, key: &str) {
    let cur = counts.get(key).and_then(Value::as_i64).unwrap_or(0);
    counts.insert(key.to_string(), json!(cur + 1));
}

/// 把一次变换收集的未对齐引用并入全局汇总（按 类别+源 id 累计次数）。
fn absorb_unmapped(ctx: &TransformCtx<'_>, agg: &mut HashMap<(String, i32), i32>) {
    for (kind, id) in &ctx.unmapped {
        *agg.entry((kind.clone(), *id)).or_default() += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs() -> RefMaps {
        let mut r = RefMaps::default();
        r.products.insert(12, 101);
        r.products.insert(13, 102);
        r.categories.insert(7, 201);
        r.brands.insert(3, 301);
        r
    }

    #[test]
    fn transform_remaps_ids_and_blanks_unmapped() {
        let refs = refs();
        let urls: Vec<(String, String)> = vec![];
        let mut ctx = TransformCtx { refs: &refs, urls: &urls, unmapped: Vec::new() };
        let raw = r#"{"moduleList":[{"type":"product","module":{"productIds":[12,99],"categoryId":7,"productCategoryId":8,"title":"x"}}]}"#;
        let out = transform_json_text(raw, &mut ctx);
        let v: Value = serde_json::from_str(&out).unwrap();
        let module = &v["moduleList"][0]["module"];
        assert_eq!(module["productIds"], json!([101]));
        assert_eq!(module["categoryId"], json!(201));
        assert_eq!(module["productCategoryId"], json!(0));
        assert_eq!(ctx.unmapped.len(), 2);
    }

    #[test]
    fn transform_rewrites_urls_and_link_params() {
        let refs = refs();
        let urls = vec![(
            "https://src.example.com/a.jpg".to_string(),
            "/static/clone/b.jpg".to_string(),
        )];
        let mut ctx = TransformCtx { refs: &refs, urls: &urls, unmapped: Vec::new() };
        let raw = r#"{"picList":[{"picUrl":"https://src.example.com/a.jpg","picLink":{"link":"/pages/product/detail?productId=12"}}]}"#;
        let out = transform_json_text(raw, &mut ctx);
        assert!(out.contains("/static/clone/b.jpg"));
        assert!(out.contains("productId=101"));
    }

    #[test]
    fn extract_urls_from_json_and_html() {
        let text = r#"{"a":"https://x.com/1.jpg","b":"<img src=\"https://y.com/2.png\">"}"#;
        let urls = extract_urls(text);
        assert!(urls.contains(&"https://x.com/1.jpg".to_string()));
        assert!(urls.contains(&"https://y.com/2.png".to_string()));
    }

    #[test]
    fn parse_id_array_accepts_json_and_csv() {
        assert_eq!(parse_id_array("[1, 2,3]"), vec![1, 2, 3]);
        assert_eq!(parse_id_array("4,5"), vec![4, 5]);
        assert_eq!(parse_id_array(""), Vec::<i32>::new());
    }
}
