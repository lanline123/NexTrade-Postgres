//! 站点搬家导入引擎（site_clone::importer）：来源凭据解析、对源站
//! `/clone/v1` 的只读拉取、导入预览与执行（快照 → 结构域对齐 → 商品 →
//! 设置覆盖）、撤销本次导入。
//!
//! 执行口径（设计报告第六节）：
//! - 失败隔离粒度 = 域内逐条：单条商品失败不回滚整批，任务报告逐条列明，
//!   重跑同一口令即续跑（import_map 已完成条目自动跳过）；
//! - 设置覆盖前整包快照（`site_clone_snapshot`），回滚见 clone.rs；
//! - 商品默认下架（防呆），预览/执行参数可改回保持源状态；
//! - 已导入商品默认跳过防翻倍，显式 `forceDuplicate` 才再建一份。

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use nextrade_common::utils;
use nextrade_core::models::site_clone::clone::{
    CloneCatalogVO, CloneExecuteDTO, CloneManifestVO, ClonePreviewDTO, ClonePreviewVO,
    CloneProduct, CloneSettingsPackage, CloneSourceCred, SiteCloneImportJob, DOMAIN_ASSETS,
    DOMAIN_MAIL, DOMAIN_MARKETING, DOMAIN_PRODUCTS, DOMAIN_SETTINGS, DOMAIN_SHOPS,
    EXPORTER_VERSION,
};
use nextrade_core::models::system::config_basic::find_endpoint;
use nextrade_core::{Error, Result};

use super::clone::{
    build_save_dto, resolve_source, settings_group_infos, SiteCloneService,
};
use super::whitelist;

/// 商品拉取分页大小。
const PRODUCT_FETCH_SIZE: u64 = 50;
/// 商品拉取页数上限（防源站异常数据打爆：50 × 200 = 1 万件）。
const PRODUCT_FETCH_MAX_PAGES: u64 = 200;

impl SiteCloneService {
    // ———————————————————— 预览 ————————————————————

    /// 导入预览：拉 manifest + 设置包，组装「源站有什么 / 本站将新增什么 /
    /// 将覆盖什么 / 已导入过多少」的清单（不落任何数据）。
    pub async fn preview(&self, dto: &ClonePreviewDTO) -> Result<ClonePreviewVO> {
        let cred = resolve_source(&dto.source)?;
        let src = CloneSource::remote(cred.clone());
        let manifest = fetch_manifest(&src).await?;
        let has_assets = manifest.domains.iter().any(|d| d == DOMAIN_ASSETS);
        let mut warnings = vec![
            if has_assets {
                "素材将转存到本站存储（服务端下载后上传），商品/装修/设置中的源站图片地址会一并改写为本站地址".to_string()
            } else {
                "本次密钥未授权素材域：商品图片为外链直用（源站删图/防盗链会导致图片失效）".to_string()
            },
            "支付渠道、各类 API 密钥、站点身份与会员/订单/财务数据不在分享范围内，不会被复制".to_string(),
            "装修页一律新建为未发布草稿、不会抢占本站首页；导入后请在装修编辑器检查后再发布".to_string(),
        ];
        let mut groups = Vec::new();
        let mut blocks: Vec<String> = Vec::new();
        if manifest.domains.iter().any(|d| d == DOMAIN_SETTINGS) {
            let pkg = fetch_settings(&src).await?;
            groups = settings_group_infos(&pkg);
            blocks = pkg.blocks.iter().map(|b| b.code.clone()).collect();
            if groups.iter().any(|g| g.risk) {
                warnings.push("费率类设置组（提现/分销）默认不导入，需在执行时显式勾选确认".to_string());
            }
        }
        if manifest.domains.iter().any(|d| d == DOMAIN_MARKETING) {
            warnings.push("营销只搬活动模板（券/秒杀/拼团/促销/礼品卡/抽奖等定义），已发放的券/卡与参与记录不搬；导入的活动一律停用态，需在本站检查后手动启用".to_string());
        }
        if manifest.domains.iter().any(|d| d == DOMAIN_SHOPS) {
            warnings.push("店铺壳只重建店铺基础资料：新建店铺默认暂停运营、不带商户账号/资金/费率，绑定商户后由平台开启".to_string());
        }
        if manifest.domains.iter().any(|d| d == DOMAIN_MAIL) {
            warnings.push("邮件模板按模板代码覆盖本站同名模板，覆盖前原值会进导入前快照可回滚；微信模板消息 ID 属源站公众号签发，不在搬运范围".to_string());
        }
        let already = self.repo().count_maps(&cred.fingerprint, "product").await?;
        let decorate_already = self.repo().count_maps(&cred.fingerprint, "decorate").await?;
        let mut marketing_already = 0i64;
        for d in super::p3_importer::MARKETING_MAP_DOMAINS {
            marketing_already += self.repo().count_maps(&cred.fingerprint, d).await?;
        }
        let content_already = self.repo().count_maps(&cred.fingerprint, "article").await?;
        let shops_already = self.repo().count_maps(&cred.fingerprint, "shop").await?;
        Ok(ClonePreviewVO {
            source_label: manifest.site_name.clone(),
            source_origin: cred.origin.clone(),
            exporter_version: manifest.exporter_version,
            key_domains: manifest.domains.clone(),
            source_shop_id: manifest.source_shop_id,
            product_total: manifest.product_total,
            product_already_imported: already,
            category_total: manifest.category_total,
            brand_total: manifest.brand_total,
            shipping_tpl_total: manifest.shipping_tpl_total,
            decorate_total: manifest.decorate_total,
            discrete_total: manifest.discrete_total,
            nav_total: manifest.nav_total,
            asset_pic_total: manifest.asset_pic_total,
            asset_video_total: manifest.asset_video_total,
            decorate_already_imported: decorate_already,
            marketing_total: manifest.marketing_total,
            article_total: manifest.article_total,
            friend_link_total: manifest.friend_link_total,
            mail_template_total: manifest.mail_template_total,
            shop_shell_total: manifest.shop_shell_total,
            marketing_already_imported: marketing_already,
            content_already_imported: content_already,
            shops_already_imported: shops_already,
            settings_groups: groups,
            settings_blocks: blocks,
            warnings,
        })
    }

    // ———————————————————— 执行 ————————————————————

    /// 执行导入：建任务行 → 按域执行 → 落任务报告并返回任务行。
    /// 来源解析失败直接报错（不建任务）；执行期错误记入任务报告。
    pub async fn execute(
        &self,
        dto: &CloneExecuteDTO,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<SiteCloneImportJob> {
        // 数据来源：在线（口令拉取）或离线包（dto.package 非空时先验签，
        // 验签失败直接报错不建任务）；两者共用同一导入编排，无第二通道。
        let src = match &dto.package {
            Some(pkg) => self.verify_offline_source(dto, pkg)?,
            None => CloneSource::remote(resolve_source(&dto.source)?),
        };
        let cred = &src.cred;
        let target_shop = dto.target_shop_id.unwrap_or(0);
        let now = utils::now_seconds() as i32;
        let domains: Vec<&str> = [
            (dto.import_products.unwrap_or(true), "products"),
            (dto.import_settings.unwrap_or(true), "settings"),
            (dto.import_decorate.unwrap_or(true), "decorate"),
            (dto.import_assets.unwrap_or(true), "assets"),
            (dto.import_marketing.unwrap_or(true), "marketing"),
            (dto.import_content.unwrap_or(true), "content"),
            (dto.import_mail.unwrap_or(true), "mail"),
            (dto.import_shops.unwrap_or(true), "shops"),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, d)| d)
        .collect();
        let job_id = self
            .repo()
            .insert_job(
                &cred.fingerprint,
                &cred.origin,
                target_shop,
                &domains.join(","),
                &serde_json::to_string(dto).unwrap_or_default(),
                operator_id,
                operator_name,
                now,
            )
            .await?;

        let mut counts = serde_json::Map::new();
        let mut failures: Vec<Value> = Vec::new();
        let mut source_label = String::new();
        let mut snapshot_id = 0;

        match self
            .run_import(dto, &src, job_id, target_shop, operator_id, operator_name, &mut counts, &mut failures)
            .await
        {
            Ok(label) => source_label = label,
            Err(e) => failures.push(json!({
                "domain": "system", "sourceId": 0, "name": "", "error": e.to_string(),
            })),
        }
        if let Some(sid) = counts.get("__snapshot_id").and_then(Value::as_i64) {
            snapshot_id = i32::try_from(sid).unwrap_or(0);
            counts.remove("__snapshot_id");
        }

        let created: i64 = counts.values().filter_map(Value::as_i64).sum();
        let status = if failures.is_empty() {
            "success"
        } else if created > 0 {
            "partial"
        } else {
            "failed"
        };
        let finish = utils::now_seconds() as i32;
        self.repo()
            .finish_job(
                job_id,
                status,
                &source_label,
                snapshot_id,
                &Value::Object(counts).to_string(),
                &Value::Array(failures).to_string(),
                finish,
            )
            .await?;
        self.repo().find_job(job_id).await?.ok_or_else(|| Error::Internal("任务落库后丢失".into()))
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_import(
        &self,
        dto: &CloneExecuteDTO,
        src: &CloneSource,
        job_id: i32,
        target_shop: i32,
        operator_id: i32,
        operator_name: &str,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) -> Result<String> {
        let manifest = fetch_manifest(src).await?;
        let want_products = dto.import_products.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_PRODUCTS);
        let want_settings = dto.import_settings.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_SETTINGS);

        // —— 设置域：先快照、后覆盖（快照 id 经 counts 暗键回传） ——
        if want_settings {
            self.import_settings(dto, src, job_id, &manifest, operator_id, operator_name, counts, failures)
                .await?;
        }
        // —— 商品域：结构对齐 → 逐条建品 ——
        if want_products {
            self.import_products(dto, src, job_id, target_shop, counts, failures).await?;
        }
        // —— P2：素材转存/重写 → 装修导入（内部按 manifest 授权域再判） ——
        self.run_p2(dto, src, &manifest, job_id, target_shop, counts, failures)
            .await?;
        // —— P3：店铺壳 → 内容 → 邮件 → 营销（依赖顺序，内部再判授权域；
        // 店铺壳先行使营销的店铺归属能命中壳映射，商品/分类/品牌映射
        // 已由 P1 阶段与历史任务落入 import_map） ——
        self.run_p3(dto, src, &manifest, job_id, target_shop, operator_id, operator_name, counts, failures)
            .await?;
        Ok(manifest.site_name.clone())
    }

    #[allow(clippy::too_many_arguments)]
    async fn import_settings(
        &self,
        dto: &CloneExecuteDTO,
        src: &CloneSource,
        job_id: i32,
        manifest: &CloneManifestVO,
        operator_id: i32,
        operator_name: &str,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) -> Result<()> {
        let cred = &src.cred;
        let pkg = fetch_settings(src).await?;
        let include_risk = dto.include_risk_groups.unwrap_or(false);
        // 勾选口径：显式列表优先；无列表时 = 全部非风险组（+风险组当 include_risk）。
        let selected: Vec<String> = match &dto.settings_groups {
            Some(list) => list
                .iter()
                .filter(|k| whitelist::group_rule(k).is_some())
                .cloned()
                .collect(),
            None => whitelist::GROUP_RULES
                .iter()
                .filter(|r| include_risk || !r.risk)
                .map(|r| r.key.to_string())
                .collect(),
        };
        let selected: Vec<String> = selected
            .into_iter()
            .filter(|k| {
                let rule = whitelist::group_rule(k);
                match rule {
                    Some(r) if r.risk && !include_risk => {
                        failures.push(json!({
                            "domain": "settings", "sourceId": 0, "name": r.label,
                            "error": "费率类设置组未确认，已跳过（执行时勾选风险组后重跑）",
                        }));
                        false
                    }
                    _ => true,
                }
            })
            .collect();

        // —— 快照：被覆盖组/块的本站当前原值 ——
        let mut snap_groups = serde_json::Map::new();
        for key in &selected {
            let Some(rule) = whitelist::group_rule(key) else { continue };
            let Some(spec) = find_endpoint(rule.get_path) else { continue };
            if let Ok(current) = self.config_basic.get(spec, None).await {
                snap_groups.insert(key.clone(), current);
            }
        }
        let mut snap_blocks = serde_json::Map::new();
        for block in &pkg.blocks {
            if !whitelist::is_block_code(&block.code) {
                continue;
            }
            let current = self.config_repo().find_by_code(&block.code).await?;
            snap_blocks.insert(
                block.code.clone(),
                current.map_or(Value::Null, Value::String),
            );
        }
        let payload = json!({ "groups": snap_groups, "blocks": snap_blocks }).to_string();
        let now = utils::now_seconds() as i32;
        let snapshot_id = self
            .repo()
            .insert_snapshot(
                job_id,
                operator_id,
                operator_name,
                &manifest.site_name,
                &cred.fingerprint,
                &payload,
                &selected.join(","),
                now,
            )
            .await?;
        let _ = self.repo().prune_snapshots(10).await;
        counts.insert("__snapshot_id".into(), json!(snapshot_id));

        // —— 覆盖：组走规格表保存（导入侧同白名单二次过滤），块走原始行 ——
        let mut applied_groups = 0i64;
        for group in &pkg.groups {
            if !selected.iter().any(|k| k == &group.key) {
                continue;
            }
            let Some(rule) = whitelist::group_rule(&group.key) else { continue };
            let Some(spec) = find_endpoint(rule.save_path) else { continue };
            let filtered = whitelist::filter_group_value(&group.data, rule.deny_fields);
            let body: Map<String, Value> = match filtered {
                Value::Object(m) => m,
                _ => Map::new(),
            };
            match self.config_basic.save(spec, &body).await {
                Ok(_) => applied_groups += 1,
                Err(e) => failures.push(json!({
                    "domain": "settings", "sourceId": 0, "name": rule.label, "error": e.to_string(),
                })),
            }
        }
        counts.insert("settingsGroupsApplied".into(), json!(applied_groups));

        let mut applied_blocks = 0i64;
        for block in &pkg.blocks {
            if !whitelist::is_block_code(&block.code) {
                continue;
            }
            let raw = match &block.value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            match self.config_repo().save_code(&block.code, &raw).await {
                Ok(()) => applied_blocks += 1,
                Err(e) => failures.push(json!({
                    "domain": "settings", "sourceId": 0, "name": block.code, "error": e.to_string(),
                })),
            }
        }
        counts.insert("settingsBlocksApplied".into(), json!(applied_blocks));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn import_products(
        &self,
        dto: &CloneExecuteDTO,
        src: &CloneSource,
        job_id: i32,
        target_shop: i32,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) -> Result<()> {
        let cred = &src.cred;
        let catalog = fetch_catalog(src).await?;
        let repo = self.repo();
        let cat = self.catalog_repo();
        let now = utils::now_seconds() as i32;
        let fp = &cred.fingerprint;

        // —— 分类：按深度（父先于子）逐个对齐 ——
        let mut cat_target: HashMap<i32, i32> = HashMap::new();
        let mut pending: Vec<_> = catalog.categories.clone();
        for _ in 0..12 {
            if pending.is_empty() {
                break;
            }
            let mut rest = Vec::new();
            for c in pending {
                if let Some(m) = repo.find_map(fp, "category", c.category_id).await? {
                    cat_target.insert(c.category_id, m.target_id);
                    continue;
                }
                let parent_target = if c.parent_id == 0 {
                    Some(0)
                } else {
                    cat_target.get(&c.parent_id).copied()
                };
                let Some(parent_target) = parent_target else {
                    rest.push(c);
                    continue;
                };
                let target_id = match cat.find_category_by_name_parent(&c.category_name, parent_target).await? {
                    Some(id) => id,
                    None => {
                        let id = cat.insert_category(&c, parent_target).await?;
                        bump(counts, "categoriesCreated");
                        id
                    }
                };
                let path = source_category_path(&catalog, c.category_id);
                repo.insert_map(fp, "category", c.category_id, &path, target_id, job_id, now).await?;
                cat_target.insert(c.category_id, target_id);
            }
            pending = rest;
        }
        for c in &pending {
            failures.push(json!({
                "domain": "category", "sourceId": c.category_id, "name": c.category_name,
                "error": "父分类未能对齐，已跳过",
            }));
        }

        // —— 品牌：按名对齐 ——
        let mut brand_target: HashMap<i32, i32> = HashMap::new();
        for b in &catalog.brands {
            if let Some(m) = repo.find_map(fp, "brand", b.brand_id).await? {
                brand_target.insert(b.brand_id, m.target_id);
                continue;
            }
            let target_id = match cat.find_brand_by_name(&b.brand_name).await? {
                Some(id) => id,
                None => {
                    let id = cat.insert_brand(b, target_shop).await?;
                    bump(counts, "brandsCreated");
                    id
                }
            };
            repo.insert_map(fp, "brand", b.brand_id, &b.brand_name, target_id, job_id, now).await?;
            brand_target.insert(b.brand_id, target_id);
        }

        // —— 配送方式：按名对齐 ——
        let mut type_target: HashMap<i32, i32> = HashMap::new();
        for t in &catalog.shipping_types {
            if let Some(m) = repo.find_map(fp, "shipping_type", t.shipping_type_id).await? {
                type_target.insert(t.shipping_type_id, m.target_id);
                continue;
            }
            let target_id = match cat.find_shipping_type_by_name(&t.shipping_type_name, target_shop).await? {
                Some(id) => id,
                None => cat.insert_shipping_type(t, target_shop).await?,
            };
            repo.insert_map(fp, "shipping_type", t.shipping_type_id, &t.shipping_type_name, target_id, job_id, now).await?;
            type_target.insert(t.shipping_type_id, target_id);
        }

        // —— 运费模板：按名对齐（新建时连明细行，type id 走映射） ——
        let mut tpl_target: HashMap<i32, i32> = HashMap::new();
        for t in &catalog.shipping_tpls {
            if let Some(m) = repo.find_map(fp, "shipping_tpl", t.shipping_tpl_id).await? {
                tpl_target.insert(t.shipping_tpl_id, m.target_id);
                continue;
            }
            let target_id = match cat.find_shipping_tpl_by_name(&t.shipping_tpl_name, target_shop).await? {
                Some(id) => id,
                None => {
                    let id = cat
                        .insert_shipping_tpl(&t.shipping_tpl_name, &t.shipping_time, t.is_free, t.pricing_type, t.is_default, target_shop)
                        .await?;
                    for info in &t.infos {
                        let type_id = type_target.get(&info.shipping_type_id).copied().unwrap_or(0);
                        if type_id > 0 {
                            cat.insert_shipping_tpl_info(id, type_id, info).await?;
                        }
                    }
                    bump(counts, "shippingTplsCreated");
                    id
                }
            };
            repo.insert_map(fp, "shipping_tpl", t.shipping_tpl_id, &t.shipping_tpl_name, target_id, job_id, now).await?;
            tpl_target.insert(t.shipping_tpl_id, target_id);
        }

        // —— 商品：分页拉取逐条建品 ——
        let keep_status = dto.keep_product_status.unwrap_or(false);
        let force = dto.force_duplicate.unwrap_or(false);
        let mut page = 1u64;
        loop {
            let batch = fetch_products(src, page, PRODUCT_FETCH_SIZE).await?;
            if batch.is_empty() {
                break;
            }
            for p in &batch {
                self.import_one_product(p, cred, job_id, target_shop, &cat_target, &brand_target, &tpl_target, keep_status, force, counts, failures)
                    .await;
            }
            if (batch.len() as u64) < PRODUCT_FETCH_SIZE || page >= PRODUCT_FETCH_MAX_PAGES {
                break;
            }
            page += 1;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn import_one_product(
        &self,
        p: &CloneProduct,
        cred: &CloneSourceCred,
        job_id: i32,
        target_shop: i32,
        cat_target: &HashMap<i32, i32>,
        brand_target: &HashMap<i32, i32>,
        tpl_target: &HashMap<i32, i32>,
        keep_status: bool,
        force: bool,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let fp = &cred.fingerprint;
        if !force {
            match self.repo().find_map(fp, "product", p.product_id).await {
                Ok(Some(_)) => {
                    bump(counts, "productsSkipped");
                    return;
                }
                Ok(None) => {}
                Err(e) => {
                    failures.push(json!({
                        "domain": "product", "sourceId": p.product_id, "name": p.product_name,
                        "error": format!("去重查询失败: {e}"),
                    }));
                    return;
                }
            }
        }
        if p.gallery.is_empty() && p.pic_url.is_empty() {
            failures.push(json!({
                "domain": "product", "sourceId": p.product_id, "name": p.product_name,
                "error": "商品无图片，已跳过",
            }));
            return;
        }
        // 服务承诺按名对齐（缺失新建到承接店铺）。
        let cat = self.catalog_repo();
        let mut service_ids: Vec<i32> = Vec::new();
        for svc in &p.services {
            let id = match cat.find_service_by_name(&svc.product_service_name).await {
                Ok(Some(id)) => Some(id),
                Ok(None) => cat.insert_service(svc, target_shop).await.ok(),
                Err(_) => None,
            };
            if let Some(id) = id {
                service_ids.push(id);
            }
        }
        let dto = build_save_dto(
            p,
            target_shop,
            cat_target.get(&p.category_id).copied().unwrap_or(0),
            brand_target.get(&p.brand_id).copied().unwrap_or(0),
            i64::from(tpl_target.get(&p.shipping_tpl_id).copied().unwrap_or(0)),
            service_ids,
            keep_status,
        );
        if let Err(e) = self.product_manage.create(&dto).await {
            failures.push(json!({
                "domain": "product", "sourceId": p.product_id, "name": p.product_name,
                "error": e.to_string(),
            }));
            return;
        }
        bump(counts, "productsCreated");
        // 回收新 id（货号优先；无货号按同店同名最新一条兜底）。
        let new_id = if !p.product_sn.is_empty() {
            cat.find_product_id_by_sn(&p.product_sn).await.ok().flatten()
        } else {
            cat.find_latest_product_id(target_shop, &p.product_name).await.ok().flatten()
        };
        let now = utils::now_seconds() as i32;
        match new_id {
            Some(id) => {
                let _ = self
                    .repo()
                    .insert_map(fp, "product", p.product_id, &p.product_sn, id, job_id, now)
                    .await;
            }
            None => failures.push(json!({
                "domain": "product", "sourceId": p.product_id, "name": p.product_name,
                "error": "商品已创建但新 id 回收失败，未登记去重映射（重跑可能重复建品）",
            })),
        }
        // 图片登记为 external 素材（外链直用；转存为 P2）。
        let add_time = now.to_string();
        for g in &p.gallery {
            let _ = cat
                .register_external_pic(target_shop, &g.pic_url, &g.pic_desc, &g.pic_thumb, g.sort_order, &add_time)
                .await;
        }
    }

    // ———————————————————— 撤销 ————————————————————

    /// 撤销本次导入：删除该任务新建的全部商品（软删，走商品仓储既有
    /// 语义）与 P2 新建对象（装修页/离散组件/导航/楼层/移动分类导航/
    /// 转存素材行），并清掉其映射。结构域（分类/品牌/运费模板）与设置
    /// 不随撤销删除：分类等可能已被本站其他商品引用，设置回滚走快照
    /// 通道；已转存的存储文件保留（可能已被其他页面引用，删行不删文件）。
    pub async fn undo_job(&self, job_id: i32) -> Result<Value> {
        let job = self
            .repo()
            .find_job(job_id)
            .await?
            .ok_or_else(|| Error::biz(404, "导入任务不存在"))?;
        if job.undone == 1 {
            return Err(Error::biz(400, "该任务已撤销过"));
        }
        let maps = self.repo().list_maps_by_job(job_id).await?;
        let ids_of = |domain: &str| -> Vec<i32> {
            maps.iter()
                .filter(|m| m.domain == domain)
                .map(|m| m.target_id)
                .collect()
        };
        let product_ids = ids_of("product");
        let deleted = if product_ids.is_empty() {
            0
        } else {
            self.repos
                .catalog
                .product_manage
                .soft_delete_products(&product_ids)
                .await?
        };
        self.repo().delete_product_maps_by_job(job_id).await?;

        // —— P2 新建对象：按映射逐域删除并清映射 ——
        let deco = self.decorate_repo();
        let deleted_pages = deco.delete_decorate_pages(&ids_of("decorate")).await?;
        let deleted_discrete = deco.delete_discrete(&ids_of("discrete")).await?;
        let deleted_navs = deco.delete_navigations(&ids_of("pc_navigation")).await?;
        let deleted_floors = deco.delete_cat_floors(&ids_of("pc_cat_floor")).await?;
        let deleted_cat_navs = deco.delete_mobile_cat_navs(&ids_of("mobile_cat_nav")).await?;
        for domain in ["decorate", "discrete", "pc_navigation", "pc_cat_floor", "mobile_cat_nav"] {
            self.repo().delete_maps_by_job_domain(job_id, domain).await?;
        }
        let asset_maps = deco.list_asset_maps_by_job(job_id).await?;
        let pic_ids: Vec<i32> = asset_maps
            .iter()
            .map(|m| m.target_pic_id)
            .filter(|id| *id > 0)
            .collect();
        let deleted_pics = deco.delete_gallery_pics(&pic_ids).await?;
        deco.delete_asset_maps_by_job(job_id).await?;

        // —— P3 新建对象：按映射逐域删除并清映射 ——
        // 口径：券/镜像/抽奖走软删（表有 is_delete）；其余硬删本任务行；
        // 礼品卡组仅未发行过卡才删组行（已发行卡的组保留，计数差额即
        // 保留数）；店铺壳软删（关店+删除标记）并连带清其分销规则行；
        // 签到规则映射中 target 与 source 相同且非本任务新建的行在
        // 导入端以 source_key 前缀 `exist:` 登记，撤销时跳过不删；
        // 文章分类为结构域保留（同商品分类）；邮件/消息模板与平台级
        // 分销规则属覆盖型，走快照回滚通道，不在此删。
        let p3 = self.p3_repo();
        let mut deleted_marketing = 0u64;
        deleted_marketing += p3.soft_delete_promotions(&ids_of("promotion")).await?;
        deleted_marketing += p3.soft_delete_coupons(&ids_of("coupon")).await?;
        deleted_marketing += p3.delete_seckills(&ids_of("seckill")).await?;
        let groupon_ids: Vec<i64> = ids_of("groupon").iter().map(|id| i64::from(*id)).collect();
        deleted_marketing += p3.delete_groupons(&groupon_ids).await?;
        deleted_marketing += p3.delete_time_discounts(&ids_of("time_discount")).await?;
        deleted_marketing += p3.delete_product_promotions(&ids_of("product_promotion")).await?;
        deleted_marketing += p3.delete_points_exchanges(&ids_of("points_exchange")).await?;
        deleted_marketing += p3.delete_recharge_settings(&ids_of("recharge")).await?;
        deleted_marketing += p3.delete_product_gifts(&ids_of("product_gift")).await?;
        let gift_group_ids = ids_of("gift_card_group");
        let deleted_gift_groups = p3.delete_gift_card_groups(&gift_group_ids).await?;
        deleted_marketing += deleted_gift_groups;
        let kept_gift_groups = (gift_group_ids.len() as u64).saturating_sub(deleted_gift_groups);
        deleted_marketing += p3.soft_delete_lotteries(&ids_of("lottery")).await?;
        let signin_new: Vec<i32> = maps
            .iter()
            .filter(|m| m.domain == "signin" && !m.source_key.starts_with("exist:"))
            .map(|m| m.target_id)
            .collect();
        deleted_marketing += p3.delete_sign_in_settings(&signin_new).await?;
        let deleted_articles = p3.delete_articles(&ids_of("article")).await?;
        let deleted_links = p3.delete_friend_links(&ids_of("friend_link")).await?;
        // 店铺壳映射含「源主店 → 承接店铺」一条：承接店铺是本站既有
        // 店铺，撤销绝不能删它，只删本任务新建的壳。
        let shop_ids: Vec<i32> = ids_of("shop")
            .into_iter()
            .filter(|id| *id != job.target_shop_id)
            .collect();
        p3.delete_salesman_configs_for_shops(&shop_ids).await?;
        let deleted_shops = p3.soft_delete_shops(&shop_ids).await?;
        for domain in [
            "promotion", "coupon", "seckill", "groupon", "time_discount", "product_promotion",
            "points_exchange", "recharge", "product_gift", "gift_card_group", "lottery", "signin",
            "article", "friend_link", "shop",
        ] {
            self.repo().delete_maps_by_job_domain(job_id, domain).await?;
        }

        self.repo().mark_job_undone(job_id).await?;
        Ok(json!({
            "jobId": job_id,
            "deletedProducts": deleted,
            "deletedDecoratePages": deleted_pages,
            "deletedDiscrete": deleted_discrete,
            "deletedNavigations": deleted_navs + deleted_floors + deleted_cat_navs,
            "deletedAssetPics": deleted_pics,
            "deletedMarketing": deleted_marketing,
            "keptGiftCardGroups": kept_gift_groups,
            "deletedArticles": deleted_articles,
            "deletedFriendLinks": deleted_links,
            "deletedShops": deleted_shops,
            "note": "分类/品牌/运费模板/文章分类等结构域对象保留（可能已被其他商品引用）；设置与邮件模板/分销规则覆盖请用该任务的导入前快照回滚；已转存的存储文件保留不删；已发行过卡的礼品卡组保留不删（持卡人资产）",
        }))
    }
}

// ———————————————————— 源站拉取（只读） ————————————————————

pub(super) async fn remote_get(cred: &CloneSourceCred, path: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| Error::Internal(format!("HTTP 客户端初始化失败: {e}")))?;
    let resp = client
        .get(format!("{}{}", cred.origin.trim_end_matches('/'), path))
        .header("X-Clone-Key-Id", &cred.key_code)
        .header("X-Clone-Secret", &cred.secret)
        .send()
        .await
        .map_err(|e| Error::Validation(format!("连接来源站失败: {e}")))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| Error::Internal(format!("读取来源站响应失败: {e}")))?;
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = parsed.get("msg").and_then(Value::as_str).unwrap_or("来源站返回错误");
        return Err(Error::Validation(format!("来源站拒绝（HTTP {status}）：{msg}")));
    }
    if let Some(code) = parsed.get("code").and_then(Value::as_i64) {
        if code != 0 {
            let msg = parsed.get("msg").and_then(Value::as_str).unwrap_or("来源站业务错误");
            return Err(Error::Validation(format!("来源站拒绝：{msg}")));
        }
    }
    Ok(parsed.get("data").cloned().unwrap_or(Value::Null))
}

/// 导入数据来源：在线拉取或离线包（P3）。离线包经逐文件 sha +
/// HMAC 验签后把各端点载荷放进 `files`（端点名 → data JSON），
/// 编排层一律经 `fetch_json` 取数——离线缺文件与在线 404 同语义
/// （错误文案含 HTTP 404，P2/P3 的「源站版本过低跳过」判定通用）。
pub(super) struct CloneSource {
    pub cred: CloneSourceCred,
    pub files: Option<HashMap<String, Value>>,
}

impl CloneSource {
    pub(super) fn remote(cred: CloneSourceCred) -> Self {
        Self { cred, files: None }
    }

    pub(super) fn offline(cred: CloneSourceCred, files: HashMap<String, Value>) -> Self {
        Self { cred, files: Some(files) }
    }
}

/// 统一取数：离线包命中文件直接返回；否则走在线只读拉取。
pub(super) async fn fetch_json(src: &CloneSource, name: &str, path: &str) -> Result<Value> {
    if let Some(files) = &src.files {
        return files.get(name).cloned().ok_or_else(|| {
            Error::Validation(format!(
                "来源站拒绝（HTTP 404）：离线包不含「{name}」数据（源站版本过低或密钥未授权该域）"
            ))
        });
    }
    remote_get(&src.cred, path).await
}

async fn fetch_manifest(src: &CloneSource) -> Result<CloneManifestVO> {
    let data = fetch_json(src, "manifest", "/api/clone/v1/manifest").await?;
    let manifest: CloneManifestVO = serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站 manifest 解析失败: {e}")))?;
    if manifest.exporter_version > EXPORTER_VERSION {
        return Err(Error::Validation(format!(
            "来源站导出版本 {} 高于本站支持的 {}，请先升级本站后再搬家",
            manifest.exporter_version, EXPORTER_VERSION
        )));
    }
    Ok(manifest)
}

async fn fetch_catalog(src: &CloneSource) -> Result<CloneCatalogVO> {
    let data = fetch_json(src, "catalog", "/api/clone/v1/catalog").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站目录包解析失败: {e}")))
}

async fn fetch_settings(src: &CloneSource) -> Result<CloneSettingsPackage> {
    let data = fetch_json(src, "settings", "/api/clone/v1/settings").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站设置包解析失败: {e}")))
}

async fn fetch_products(
    src: &CloneSource,
    page: u64,
    size: u64,
) -> Result<Vec<CloneProduct>> {
    if let Some(files) = &src.files {
        // 离线包内为全量商品列表，本地分页切片。
        let data = files.get("products").cloned().ok_or_else(|| {
            Error::Validation("来源站拒绝（HTTP 404）：离线包不含「products」数据".to_string())
        })?;
        let list = data
            .get("list")
            .or_else(|| data.get("records"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let start = usize::try_from((page - 1) * size).unwrap_or(0);
        if start >= list.len() {
            return Ok(Vec::new());
        }
        let end = (start + usize::try_from(size).unwrap_or(0)).min(list.len());
        return serde_json::from_value(Value::Array(list[start..end].to_vec()))
            .map_err(|e| Error::Validation(format!("离线包商品数据解析失败: {e}")));
    }
    let data = remote_get(&src.cred, &format!("/api/clone/v1/products?page={page}&size={size}")).await?;
    let list = data
        .get("list")
        .or_else(|| data.get("records"))
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    serde_json::from_value(list)
        .map_err(|e| Error::Validation(format!("来源站商品包解析失败: {e}")))
}

/// 源分类名称路径（按源 catalog 父链拼装，用于映射留痕）。
fn source_category_path(catalog: &CloneCatalogVO, category_id: i32) -> String {
    let by_id: HashMap<i32, _> = catalog.categories.iter().map(|c| (c.category_id, c)).collect();
    let mut names = Vec::new();
    let mut cur = category_id;
    for _ in 0..10 {
        match by_id.get(&cur) {
            Some(c) => {
                names.push(c.category_name.clone());
                if c.parent_id <= 0 {
                    break;
                }
                cur = c.parent_id;
            }
            None => break,
        }
    }
    names.reverse();
    names.join("/")
}

/// 计数器 +1（counts JSON 暗键约定：`__` 前缀为内部回传，落库前剔除）。
pub(super) fn bump(counts: &mut serde_json::Map<String, Value>, key: &str) {
    let cur = counts.get(key).and_then(Value::as_i64).unwrap_or(0);
    counts.insert(key.to_string(), json!(cur + 1));
}
