//! 站点搬家 P3 导入与离线包（site_clone::p3_importer）：店铺壳 →
//! 文章/友链 → 邮件/消息模板 → 营销模板的导入编排、P3 覆盖型快照
//! （mail / salesman 两节）、离线包导出与验签导入。
//!
//! 口径（逐项见 P3 报告字段级口径表）：
//! - 顺序依赖：店铺壳先行（营销的店铺归属靠壳映射命中）；营销域内
//!   赠品先于满赠/充值档位、券先于积分兑换/抽奖奖品、活动先于
//!   promotion 镜像行；
//! - 店铺归属解析：0/NULL → 平台；== 密钥源店铺 → 承接店铺；命中壳
//!   映射 → 壳店铺；其余（未建壳的其他商户）→ 整行跳过并报告；
//! - SKU 引用按货号对齐（源 SKU 目录随营销包给出），对不齐按各表
//!   口径降级或跳行，全部汇总进任务报告（refsBlanked + 分域说明）；
//! - 覆盖型数据（邮件/消息模板、分销规则）不走 import_map 去重，
//!   靠「导入前快照」回滚（payload 的 mail/salesman 节，见
//!   clone.rs::rollback_snapshot）；新建型数据走 import_map 去重 +
//!   撤销导入删除（见 importer.rs::undo_job 的 P3 段）。

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

use nextrade_common::utils;
use nextrade_core::models::site_clone::clone::{
    CloneContentPackage, CloneExecuteDTO, CloneMailPackage, CloneManifestVO,
    CloneMarketingPackage, CloneOfflineFile, CloneOfflinePackage, CloneShopsPackage, CloneSkuRef,
    DOMAIN_ASSETS, DOMAIN_CONTENT, DOMAIN_DECORATE, DOMAIN_MAIL, DOMAIN_MARKETING,
    DOMAIN_PRODUCTS, DOMAIN_SETTINGS, DOMAIN_SHOPS, EXPORTER_VERSION, OFFLINE_PACKAGE_FORMAT,
    OFFLINE_PACKAGE_VERSION,
};
use nextrade_core::{Error, Result};

use super::clone::{constant_time_eq, parse_id_list, resolve_source, SiteCloneService};
use super::decorate_importer::{
    apply_url_map, is_endpoint_missing, sorted_url_pairs, transform_json_text, RefMaps,
    TransformCtx,
};
use super::importer::{bump, fetch_json, CloneSource};

type HmacSha256 = Hmac<Sha256>;

/// 营销子域的 import_map 域命名（去重/撤销/预览已导入计数共用）。
pub(super) const MARKETING_MAP_DOMAINS: [&str; 12] = [
    "coupon", "seckill", "groupon", "time_discount", "product_promotion", "promotion",
    "points_exchange", "product_gift", "recharge", "signin", "gift_card_group", "lottery",
];

/// 店铺归属解析器（见文件头口径）。
struct ShopResolver {
    source_shop_id: i32,
    target_shop: i32,
    map: HashMap<i32, i32>,
}

impl ShopResolver {
    /// 解析源店铺 id → 本站店铺 id；None = 未建壳的其他商户（跳行）。
    fn resolve(&self, source_shop: i32) -> Option<i32> {
        if source_shop <= 0 {
            Some(0)
        } else if source_shop == self.source_shop_id {
            Some(self.target_shop)
        } else {
            self.map.get(&source_shop).copied()
        }
    }

    fn resolve_opt(&self, source_shop: Option<i32>) -> Option<i32> {
        self.resolve(source_shop.unwrap_or(0))
    }
}

/// 未对齐引用收集（任务报告汇总用）：(类别, 源 id)。
#[derive(Default)]
struct UnmappedRefs {
    items: Vec<(String, i32)>,
}

impl UnmappedRefs {
    fn push(&mut self, kind: &str, id: i32) {
        self.items.push((kind.to_string(), id));
    }

    /// 按类别汇总成失败说明（每类一条、 id 最多列 20 个）。
    fn flush_into(&self, counts: &mut serde_json::Map<String, Value>, failures: &mut Vec<Value>) {
        if self.items.is_empty() {
            return;
        }
        counts.insert("refsBlanked".into(), json!(self.items.len() as i64));
        let mut by_kind: HashMap<String, Vec<i32>> = HashMap::new();
        for (kind, id) in &self.items {
            by_kind.entry(kind.clone()).or_default().push(*id);
        }
        let mut kinds: Vec<_> = by_kind.into_iter().collect();
        kinds.sort_by(|a, b| a.0.cmp(&b.0));
        for (kind, ids) in kinds {
            let shown: Vec<String> = ids.iter().take(20).map(i32::to_string).collect();
            failures.push(json!({
                "domain": "refs", "sourceId": 0, "name": kind.clone(),
                "error": format!(
                    "{} 类引用共 {} 个未能对齐本站（已按各表口径置 0/剔除/跳行），源 id：{}",
                    kind,
                    ids.len(),
                    shown.join(",")
                ),
            }));
        }
    }
}

impl SiteCloneService {
    // ———————————————————— P3 编排 ————————————————————

    /// P3 编排（在 P1 设置/商品与 P2 装修/素材之后执行）。源站为
    /// v1/v2（无 P3 端点）时各域记一条说明并跳过，不影响既有域。
    #[allow(clippy::too_many_arguments)]
    pub async fn run_p3(
        &self,
        dto: &CloneExecuteDTO,
        src: &CloneSource,
        manifest: &CloneManifestVO,
        job_id: i32,
        target_shop: i32,
        operator_id: i32,
        operator_name: &str,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) -> Result<()> {
        let fp = &src.cred.fingerprint;
        let want_shops = dto.import_shops.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_SHOPS);
        let want_content = dto.import_content.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_CONTENT);
        let want_mail = dto.import_mail.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_MAIL);
        let want_marketing = dto.import_marketing.unwrap_or(true)
            && manifest.domains.iter().any(|d| d == DOMAIN_MARKETING);
        if !want_shops && !want_content && !want_mail && !want_marketing {
            return Ok(());
        }

        // 基础引用映射（商品/分类/品牌，含历史任务）+ 素材转存 URL 对。
        let refs = RefMaps {
            products: self.load_map(fp, "product").await?,
            categories: self.load_map(fp, "category").await?,
            brands: self.load_map(fp, "brand").await?,
        };
        let asset_rows = self
            .decorate_repo()
            .list_asset_maps_by_fingerprint(fp)
            .await
            .unwrap_or_default();
        let url_map: HashMap<String, String> = asset_rows
            .iter()
            .map(|m| (m.source_url.clone(), m.local_url.clone()))
            .collect();
        let url_pairs = sorted_url_pairs(&url_map);

        // —— 店铺壳（先行）+ 邮件包预拉取（快照要在覆盖前建） ——
        let shops_pkg = if want_shops {
            match fetch_shops(src).await {
                Ok(p) => Some(p),
                Err(e) if is_endpoint_missing(&e) => {
                    failures.push(json!({
                        "domain": "shops", "sourceId": 0, "name": "",
                        "error": "来源站版本过低，不支持店铺壳域（需导出版本 v3），已跳过",
                    }));
                    None
                }
                Err(e) => {
                    failures.push(json!({
                        "domain": "shops", "sourceId": 0, "name": "", "error": e.to_string(),
                    }));
                    None
                }
            }
        } else {
            None
        };
        let mail_pkg = if want_mail {
            match fetch_mail(src).await {
                Ok(p) => Some(p),
                Err(e) if is_endpoint_missing(&e) => {
                    failures.push(json!({
                        "domain": "mail", "sourceId": 0, "name": "",
                        "error": "来源站版本过低，不支持邮件模板域（需导出版本 v3），已跳过",
                    }));
                    None
                }
                Err(e) => {
                    failures.push(json!({
                        "domain": "mail", "sourceId": 0, "name": "", "error": e.to_string(),
                    }));
                    None
                }
            }
        } else {
            None
        };

        // 店铺壳落库（含主店映射登记），返回归属解析器。
        let mut shop_map = self.load_map(fp, "shop").await?;
        if let Some(pkg) = &shops_pkg {
            self.import_shop_shells(pkg, manifest, fp, job_id, target_shop, &mut shop_map, counts, failures)
                .await;
        }
        let shops = ShopResolver {
            source_shop_id: manifest.source_shop_id,
            target_shop,
            map: shop_map,
        };

        // —— P3 快照：邮件模板 + 分销规则的覆盖前原值 ——
        if mail_pkg.is_some() || shops_pkg.is_some() {
            self.create_p3_snapshot(
                mail_pkg.as_ref(),
                shops_pkg.as_ref(),
                &shops,
                job_id,
                operator_id,
                operator_name,
                &manifest.site_name,
                fp,
                counts,
            )
            .await;
        }

        // 分销规则随店铺壳域应用（快照已建）。
        if let Some(pkg) = &shops_pkg {
            self.apply_salesman_configs(pkg, &shops, counts, failures).await;
        }

        // —— 文章/友链 ——
        if want_content {
            match fetch_content(src).await {
                Ok(pkg) => {
                    self.import_content(&pkg, fp, job_id, &refs, &url_pairs, counts, failures)
                        .await;
                }
                Err(e) if is_endpoint_missing(&e) => failures.push(json!({
                    "domain": "content", "sourceId": 0, "name": "",
                    "error": "来源站版本过低，不支持文章/友链域（需导出版本 v3），已跳过",
                })),
                Err(e) => failures.push(json!({
                    "domain": "content", "sourceId": 0, "name": "", "error": e.to_string(),
                })),
            }
        }

        // —— 邮件/消息模板 ——
        if let Some(pkg) = &mail_pkg {
            self.import_mail(pkg, counts, failures).await;
        }

        // —— 营销模板 ——
        if want_marketing {
            match fetch_marketing(src).await {
                Ok(pkg) => {
                    self.import_marketing(
                        &pkg, fp, job_id, &refs, &shops, &url_pairs, counts, failures,
                    )
                    .await;
                }
                Err(e) if is_endpoint_missing(&e) => failures.push(json!({
                    "domain": "marketing", "sourceId": 0, "name": "",
                    "error": "来源站版本过低，不支持营销模板域（需导出版本 v3），已跳过",
                })),
                Err(e) => failures.push(json!({
                    "domain": "marketing", "sourceId": 0, "name": "", "error": e.to_string(),
                })),
            }
        }
        Ok(())
    }

    async fn load_map(&self, fingerprint: &str, domain: &str) -> Result<HashMap<i32, i32>> {
        let rows = self
            .repo()
            .list_maps_by_fingerprint_domain(fingerprint, domain)
            .await?;
        Ok(rows.into_iter().map(|m| (m.source_id, m.target_id)).collect())
    }

    // ———————————————————— P3 快照 ————————————————————

    /// P3 覆盖型快照：邮件模板/消息类型（按将覆盖的 code/id 取本站
    /// 现值）+ 分销规则（按将应用的 店铺+code 取现值）。与设置快照
    /// 同表同回滚通道（payload 的 mail/salesman 节）；任务已有设置
    /// 快照时不抢 `__snapshot_id`（job.snapshot_id 指向先建者），
    /// 本快照仍独立出现在快照列表可单独回滚。
    #[allow(clippy::too_many_arguments)]
    async fn create_p3_snapshot(
        &self,
        mail_pkg: Option<&CloneMailPackage>,
        shops_pkg: Option<&CloneShopsPackage>,
        shops: &ShopResolver,
        job_id: i32,
        operator_id: i32,
        operator_name: &str,
        source_label: &str,
        fingerprint: &str,
        counts: &mut serde_json::Map<String, Value>,
    ) {
        let p3 = self.p3_repo();
        let mut payload = serde_json::Map::new();
        let mut sections: Vec<&str> = Vec::new();

        if let Some(pkg) = mail_pkg {
            let mut templates = Vec::new();
            for t in &pkg.templates {
                match p3.find_mail_template(&t.template_code).await {
                    Ok(Some(cur)) => templates.push(json!({
                        "templateCode": cur.template_code, "existed": true,
                        "isHtml": cur.is_html, "templateSubject": cur.template_subject,
                        "templateContent": cur.template_content, "type": cur.r#type,
                    })),
                    Ok(None) => templates.push(json!({
                        "templateCode": t.template_code, "existed": false,
                    })),
                    Err(_) => {}
                }
            }
            let mut types = Vec::new();
            for t in &pkg.message_types {
                match p3.find_message_type(t.message_id).await {
                    Ok(Some(cur)) => types.push(json!({
                        "messageId": cur.message_id, "existed": true,
                        "row": serde_json::to_value(&cur).unwrap_or(Value::Null),
                    })),
                    Ok(None) => types.push(json!({
                        "messageId": t.message_id, "existed": false,
                    })),
                    Err(_) => {}
                }
            }
            payload.insert(
                "mail".into(),
                json!({ "templates": templates, "messageTypes": types }),
            );
            sections.push("mailTemplates");
        }

        if let Some(pkg) = shops_pkg {
            let mut rows = Vec::new();
            for cfg in &pkg.salesman_configs {
                let Some(code) = cfg.code.as_deref().filter(|s| !s.is_empty()) else {
                    continue;
                };
                let Some(target) = shops.resolve(cfg.shop_id.unwrap_or(0)) else {
                    continue;
                };
                match p3.find_salesman_config(target, code).await {
                    Ok(Some((_, data))) => rows.push(json!({
                        "shopId": target, "code": code, "existed": true, "data": data,
                    })),
                    Ok(None) => rows.push(json!({
                        "shopId": target, "code": code, "existed": false,
                    })),
                    Err(_) => {}
                }
            }
            if !rows.is_empty() {
                payload.insert("salesman".into(), Value::Array(rows));
                sections.push("salesmanConfig");
            }
        }

        if sections.is_empty() {
            return;
        }
        let now = utils::now_seconds() as i32;
        match self
            .repo()
            .insert_snapshot(
                job_id,
                operator_id,
                operator_name,
                source_label,
                fingerprint,
                &Value::Object(payload).to_string(),
                &format!("P3:{}", sections.join(",")),
                now,
            )
            .await
        {
            Ok(snapshot_id) => {
                let _ = self.repo().prune_snapshots(10).await;
                if !counts.contains_key("__snapshot_id") {
                    counts.insert("__snapshot_id".into(), json!(snapshot_id));
                }
            }
            Err(e) => {
                // 快照建不起来时覆盖仍可继续，但必须在报告中明示无回滚点。
                counts.insert("p3SnapshotFailed".into(), json!(1));
                tracing::warn!("站点搬家 P3 快照创建失败: {e}");
            }
        }
    }

    // ———————————————————— 店铺壳 ————————————————————

    /// 店铺壳落库：源主店只登记映射到承接店铺（不建行、不动承接店铺
    /// 资料）；其余店铺新建暂停运营的壳；父级二遍回填。
    #[allow(clippy::too_many_arguments)]
    async fn import_shop_shells(
        &self,
        pkg: &CloneShopsPackage,
        manifest: &CloneManifestVO,
        fingerprint: &str,
        job_id: i32,
        target_shop: i32,
        shop_map: &mut HashMap<i32, i32>,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let p3 = self.p3_repo();
        let repo = self.repo();
        let now = utils::now_seconds() as i32;
        for shop in &pkg.shops {
            if shop.shop_id == manifest.source_shop_id {
                if !shop_map.contains_key(&shop.shop_id) {
                    let _ = repo
                        .insert_map(
                            fingerprint,
                            "shop",
                            shop.shop_id,
                            shop.shop_title.as_deref().unwrap_or(""),
                            target_shop,
                            job_id,
                            now,
                        )
                        .await;
                    shop_map.insert(shop.shop_id, target_shop);
                }
                continue;
            }
            if shop_map.contains_key(&shop.shop_id) {
                bump(counts, "shopsSkipped");
                continue;
            }
            match p3.insert_shop_shell(shop, now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(
                            fingerprint,
                            "shop",
                            shop.shop_id,
                            shop.shop_title.as_deref().unwrap_or(""),
                            new_id,
                            job_id,
                            now,
                        )
                        .await;
                    shop_map.insert(shop.shop_id, new_id);
                    bump(counts, "shopsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "shop", "sourceId": shop.shop_id,
                    "name": shop.shop_title.clone().unwrap_or_default(), "error": e.to_string(),
                })),
            }
        }
        // 父级回填（父店可能后于子店建壳，统一二遍处理）。
        for shop in &pkg.shops {
            let parent = shop.store_parent_id.unwrap_or(0);
            if parent <= 0 || parent == shop.shop_id {
                continue;
            }
            let (Some(child), Some(parent_target)) =
                (shop_map.get(&shop.shop_id).copied(), shop_map.get(&parent).copied())
            else {
                continue;
            };
            if child != target_shop {
                let _ = p3.update_shop_parent(child, parent_target).await;
            }
        }
    }

    /// 分销规则应用：按（归属解析后的店铺, code）upsert；归属对不齐
    /// （未建壳的商户）跳过并计数。覆盖前原值已进 P3 快照。
    async fn apply_salesman_configs(
        &self,
        pkg: &CloneShopsPackage,
        shops: &ShopResolver,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let p3 = self.p3_repo();
        for cfg in &pkg.salesman_configs {
            let Some(code) = cfg.code.as_deref().filter(|s| !s.is_empty()) else {
                continue;
            };
            let Some(target) = shops.resolve(cfg.shop_id.unwrap_or(0)) else {
                bump(counts, "salesmanConfigsSkipped");
                continue;
            };
            match p3.upsert_salesman_config(target, code, cfg.data.as_deref()).await {
                Ok(()) => bump(counts, "salesmanConfigsApplied"),
                Err(e) => failures.push(json!({
                    "domain": "salesman", "sourceId": cfg.id, "name": code, "error": e.to_string(),
                })),
            }
        }
    }

    // ———————————————————— 文章/友链 ————————————————————

    #[allow(clippy::too_many_arguments)]
    async fn import_content(
        &self,
        pkg: &CloneContentPackage,
        fingerprint: &str,
        job_id: i32,
        refs: &RefMaps,
        url_pairs: &[(String, String)],
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let p3 = self.p3_repo();
        let repo = self.repo();
        let now = utils::now_seconds() as i32;

        // —— 文章分类：按深度对齐（同商品分类口径，结构域保留） ——
        let mut cat_target: HashMap<i32, i32> = self
            .load_map(fingerprint, "article_category")
            .await
            .unwrap_or_default();
        let mut pending: Vec<_> = pkg.categories.clone();
        for _ in 0..12 {
            if pending.is_empty() {
                break;
            }
            let mut rest = Vec::new();
            for c in pending {
                if cat_target.contains_key(&c.article_category_id) {
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
                let target_id = match p3
                    .find_article_category_by_name_parent(&c.article_category_name, parent_target)
                    .await
                {
                    Ok(Some(id)) => id,
                    Ok(None) => match p3.insert_article_category(&c, parent_target).await {
                        Ok(id) => {
                            bump(counts, "articleCategoriesCreated");
                            id
                        }
                        Err(e) => {
                            failures.push(json!({
                                "domain": "article_category", "sourceId": c.article_category_id,
                                "name": c.article_category_name, "error": e.to_string(),
                            }));
                            continue;
                        }
                    },
                    Err(e) => {
                        failures.push(json!({
                            "domain": "article_category", "sourceId": c.article_category_id,
                            "name": c.article_category_name, "error": e.to_string(),
                        }));
                        continue;
                    }
                };
                let _ = repo
                    .insert_map(
                        fingerprint,
                        "article_category",
                        c.article_category_id,
                        &c.article_category_name,
                        target_id,
                        job_id,
                        now,
                    )
                    .await;
                cat_target.insert(c.article_category_id, target_id);
            }
            pending = rest;
        }
        for c in &pending {
            failures.push(json!({
                "domain": "article_category", "sourceId": c.article_category_id,
                "name": c.article_category_name, "error": "父分类未能对齐，已跳过",
            }));
        }

        // —— 文章：新建为不显示；正文/缩略图的源站素材地址按转存映射改写 ——
        let mut article_map = self.load_map(fingerprint, "article").await.unwrap_or_default();
        for a in &pkg.articles {
            if article_map.contains_key(&a.article_id) {
                bump(counts, "articlesSkipped");
                continue;
            }
            let category_id = cat_target.get(&a.article_category_id).copied().unwrap_or(0);
            let content = apply_url_map(&a.content, url_pairs);
            let thumb = apply_url_map(&a.article_thumb, url_pairs);
            match p3.insert_article(a, category_id, &content, &thumb, now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "article", a.article_id, &a.article_title, new_id, job_id, now)
                        .await;
                    article_map.insert(a.article_id, new_id);
                    bump(counts, "articlesCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "article", "sourceId": a.article_id, "name": a.article_title,
                    "error": e.to_string(),
                })),
            }
        }
        // 文章-商品关联：两端都对齐才落，对不齐的商品引用剔除并汇总。
        let mut unmapped = UnmappedRefs::default();
        for link in &pkg.article_products {
            let (Some(article_id), Some(goods_id)) = (
                article_map.get(&link.article_id).copied(),
                refs.products.get(&link.goods_id).copied(),
            ) else {
                if refs.products.get(&link.goods_id).is_none() {
                    unmapped.push("articleProduct", link.goods_id);
                }
                continue;
            };
            let _ = p3.insert_article_product(article_id, goods_id).await;
        }
        unmapped.flush_into(counts, failures);

        // —— 友链：标题+地址去重（本站已有同条时不重复建、不登记映射，
        // 撤销不会误删本站原有行） ——
        let link_map = self.load_map(fingerprint, "friend_link").await.unwrap_or_default();
        for link in &pkg.friend_links {
            if link_map.contains_key(&link.link_id) {
                bump(counts, "friendLinksSkipped");
                continue;
            }
            match p3.find_friend_link(&link.link_title, &link.link_url).await {
                Ok(Some(_)) => bump(counts, "friendLinksSkipped"),
                Ok(None) => match p3.insert_friend_link(link).await {
                    Ok(new_id) => {
                        let _ = repo
                            .insert_map(
                                fingerprint,
                                "friend_link",
                                link.link_id,
                                &link.link_title,
                                new_id,
                                job_id,
                                now,
                            )
                            .await;
                        bump(counts, "friendLinksCreated");
                    }
                    Err(e) => failures.push(json!({
                        "domain": "friend_link", "sourceId": link.link_id, "name": link.link_title,
                        "error": e.to_string(),
                    })),
                },
                Err(e) => failures.push(json!({
                    "domain": "friend_link", "sourceId": link.link_id, "name": link.link_title,
                    "error": e.to_string(),
                })),
            }
        }
    }

    // ———————————————————— 邮件/消息模板 ————————————————————

    /// 邮件模板按 code、消息类型按 id 对齐覆盖（原值已进 P3 快照；
    /// last_send 等运行态不碰）。
    async fn import_mail(
        &self,
        pkg: &CloneMailPackage,
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let p3 = self.p3_repo();
        let now = utils::now_seconds() as i32;
        for t in &pkg.templates {
            match p3.upsert_mail_template(t, now).await {
                Ok(created) => bump(
                    counts,
                    if created { "mailTemplatesCreated" } else { "mailTemplatesUpdated" },
                ),
                Err(e) => failures.push(json!({
                    "domain": "mail_template", "sourceId": t.template_id, "name": t.template_code,
                    "error": e.to_string(),
                })),
            }
        }
        for t in &pkg.message_types {
            match p3.upsert_message_type(t, now).await {
                Ok(created) => bump(
                    counts,
                    if created { "messageTypesCreated" } else { "messageTypesUpdated" },
                ),
                Err(e) => failures.push(json!({
                    "domain": "message_type", "sourceId": t.message_id,
                    "name": t.name.clone().unwrap_or_default(), "error": e.to_string(),
                })),
            }
        }
    }
}

// ———————————————————— 营销模板导入 ————————————————————

/// SKU 对齐器：源 SKU →（目录）→ 源商品 + 货号 →（商品映射）→ 本站
/// 商品 → 按货号在本站 SKU 列表中命中；货号为空时仅当源/本站均为
/// 单 SKU 才兜底命中。本站 SKU 列表按商品懒加载并缓存。
struct SkuResolver<'a> {
    dir: HashMap<i32, (i32, String)>,
    dir_by_product: HashMap<i32, Vec<i32>>,
    product_map: &'a HashMap<i32, i32>,
    target_cache: HashMap<i32, Vec<(i32, String)>>,
}

impl<'a> SkuResolver<'a> {
    fn new(directory: &[CloneSkuRef], product_map: &'a HashMap<i32, i32>) -> Self {
        let mut dir = HashMap::new();
        let mut dir_by_product: HashMap<i32, Vec<i32>> = HashMap::new();
        for r in directory {
            dir.insert(r.sku_id, (r.product_id, r.sku_sn.clone()));
            dir_by_product.entry(r.product_id).or_default().push(r.sku_id);
        }
        Self { dir, dir_by_product, product_map, target_cache: HashMap::new() }
    }

    async fn resolve(
        &mut self,
        repo: &nextrade_infrastructure::db::repo::site_clone::p3_data::SiteCloneP3Repository,
        source_sku: i32,
    ) -> Option<i32> {
        if source_sku <= 0 {
            return None;
        }
        let (source_product, sn) = self.dir.get(&source_sku)?.clone();
        let target_product = self.product_map.get(&source_product).copied()?;
        if !self.target_cache.contains_key(&target_product) {
            let skus = repo.load_target_skus(target_product).await.ok()?;
            self.target_cache.insert(target_product, skus);
        }
        let target_skus = self.target_cache.get(&target_product)?;
        if !sn.is_empty() {
            return target_skus.iter().find(|(_, s)| *s == sn).map(|(id, _)| *id);
        }
        let source_count = self.dir_by_product.get(&source_product).map_or(0, Vec::len);
        if source_count == 1 && target_skus.len() == 1 {
            return target_skus.first().map(|(id, _)| *id);
        }
        None
    }
}

/// id 列表的 Java `List.toString` 存储格式（`[1, 2]`；本站消费侧
/// serde_json / 宽容解析均可读，与 coupon_manage 的 java_list_string 同式）。
fn java_list(ids: &[i32]) -> String {
    let parts: Vec<String> = ids.iter().map(i32::to_string).collect();
    format!("[{}]", parts.join(", "))
}

/// 按范围语义重写 id 列表：1 分类 / 2 品牌 / 3、4 商品；其余（全场）
/// 原样返回（列表不参与判定）。对不齐的 id 剔除并记入 unmapped。
fn remap_ranged_ids(
    range: i32,
    raw: &str,
    refs: &RefMaps,
    unmapped: &mut UnmappedRefs,
) -> String {
    let ids = parse_id_list(raw);
    let (map, kind) = match range {
        1 => (&refs.categories, "category"),
        2 => (&refs.brands, "brand"),
        3 | 4 => (&refs.products, "product"),
        _ => return raw.to_string(),
    };
    let mut out = Vec::new();
    for id in ids {
        match map.get(&id) {
            Some(t) => out.push(*t),
            None => unmapped.push(kind, id),
        }
    }
    java_list(&out)
}

impl SiteCloneService {
    /// 营销模板导入（依赖序见文件头）。逐活动失败隔离：单活动落库
    /// 失败记 failures 不中断全域；明细行对不齐按各表口径处理。
    #[allow(clippy::too_many_arguments)]
    async fn import_marketing(
        &self,
        pkg: &CloneMarketingPackage,
        fingerprint: &str,
        job_id: i32,
        refs: &RefMaps,
        shops: &ShopResolver,
        url_pairs: &[(String, String)],
        counts: &mut serde_json::Map<String, Value>,
        failures: &mut Vec<Value>,
    ) {
        let p3 = self.p3_repo();
        let repo = self.repo();
        let now = utils::now_seconds() as i32;
        let mut unmapped = UnmappedRefs::default();
        let mut sku = SkuResolver::new(&pkg.sku_directory, &refs.products);

        // 各活动映射（历史任务 + 本次累积）。
        let mut coupon_map = self.load_map(fingerprint, "coupon").await.unwrap_or_default();
        let mut gift_map = self.load_map(fingerprint, "product_gift").await.unwrap_or_default();
        let mut seckill_map = self.load_map(fingerprint, "seckill").await.unwrap_or_default();
        let mut groupon_map: HashMap<i64, i64> = HashMap::new();
        {
            let m = self.load_map(fingerprint, "groupon").await.unwrap_or_default();
            for (k, v) in m {
                groupon_map.insert(i64::from(k), i64::from(v));
            }
        }
        let mut td_map = self.load_map(fingerprint, "time_discount").await.unwrap_or_default();
        let mut pp_map = self
            .load_map(fingerprint, "product_promotion")
            .await
            .unwrap_or_default();
        let mut gcg_map = self.load_map(fingerprint, "gift_card_group").await.unwrap_or_default();
        let mut lottery_map = self.load_map(fingerprint, "lottery").await.unwrap_or_default();

        // —— 优惠券 ——
        for c in &pkg.coupons {
            if coupon_map.contains_key(&c.coupon_id) {
                bump(counts, "couponsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve_opt(c.shop_id) else {
                failures.push(json!({
                    "domain": "coupon", "sourceId": c.coupon_id, "name": c.coupon_name,
                    "error": "所属店铺未建壳，已跳过（先导入店铺壳域或用平台密钥整站搬）",
                }));
                continue;
            };
            let range_data = remap_ranged_ids(c.send_range, &c.send_range_data, refs, &mut unmapped);
            match p3.insert_coupon(c, Some(shop), &range_data, now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "coupon", c.coupon_id, &c.coupon_name, new_id, job_id, now)
                        .await;
                    coupon_map.insert(c.coupon_id, new_id);
                    bump(counts, "couponsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "coupon", "sourceId": c.coupon_id, "name": c.coupon_name,
                    "error": e.to_string(),
                })),
            }
        }

        // —— 赠品定义（满赠阶梯/充值档位的前置） ——
        for g in &pkg.product_gifts {
            if gift_map.contains_key(&g.gift_id) {
                bump(counts, "productGiftsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve(g.shop_id) else {
                failures.push(json!({
                    "domain": "product_gift", "sourceId": g.gift_id, "name": g.gift_name,
                    "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            let Some(product) = refs.products.get(&g.product_id).copied() else {
                unmapped.push("product", g.product_id);
                failures.push(json!({
                    "domain": "product_gift", "sourceId": g.gift_id, "name": g.gift_name,
                    "error": "赠品商品未对齐本站，已跳过",
                }));
                continue;
            };
            let sku_id = if g.sku_id > 0 {
                match sku.resolve(p3, g.sku_id).await {
                    Some(s) => s,
                    None => {
                        unmapped.push("sku", g.sku_id);
                        0
                    }
                }
            } else {
                0
            };
            match p3.insert_product_gift(g, shop, product, sku_id).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "product_gift", g.gift_id, &g.gift_name, new_id, job_id, now)
                        .await;
                    gift_map.insert(g.gift_id, new_id);
                    bump(counts, "productGiftsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "product_gift", "sourceId": g.gift_id, "name": g.gift_name,
                    "error": e.to_string(),
                })),
            }
        }

        // —— 秒杀 ——
        for s in &pkg.seckills {
            if seckill_map.contains_key(&s.seckill_id) {
                bump(counts, "seckillsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve(s.shop_id) else {
                failures.push(json!({
                    "domain": "seckill", "sourceId": s.seckill_id,
                    "name": s.seckill_name.clone().unwrap_or_default(),
                    "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            let product = match s.product_id {
                Some(pid) if pid > 0 => match refs.products.get(&pid) {
                    Some(t) => Some(*t),
                    None => {
                        unmapped.push("product", pid);
                        None
                    }
                },
                _ => None,
            };
            match p3.insert_seckill(s, shop, product).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(
                            fingerprint,
                            "seckill",
                            s.seckill_id,
                            s.seckill_name.as_deref().unwrap_or(""),
                            new_id,
                            job_id,
                            now,
                        )
                        .await;
                    seckill_map.insert(s.seckill_id, new_id);
                    bump(counts, "seckillsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "seckill", "sourceId": s.seckill_id,
                    "name": s.seckill_name.clone().unwrap_or_default(), "error": e.to_string(),
                })),
            }
        }
        for it in &pkg.seckill_items {
            let Some(parent) = it.seckill_id.and_then(|id| seckill_map.get(&id).copied()) else {
                continue;
            };
            let Some(product) = it.product_id.and_then(|pid| refs.products.get(&pid).copied()) else {
                unmapped.push("product", it.product_id.unwrap_or(0));
                bump(counts, "seckillItemsSkipped");
                continue;
            };
            let sku_id = match it.sku_id {
                Some(sid) if sid > 0 => match sku.resolve(p3, sid).await {
                    Some(s) => s,
                    None => {
                        unmapped.push("sku", sid);
                        bump(counts, "seckillItemsSkipped");
                        continue;
                    }
                },
                _ => 0,
            };
            match p3.insert_seckill_item(it, parent, product, sku_id).await {
                Ok(_) => bump(counts, "seckillItemsCreated"),
                Err(e) => failures.push(json!({
                    "domain": "seckill_item", "sourceId": it.rec_id, "name": "",
                    "error": e.to_string(),
                })),
            }
        }

        // —— 拼团（i64 主键；映射表 source_id 为 i32，超界整活动跳过） ——
        let mut groupon_item_map: HashMap<i64, i64> = HashMap::new();
        for g in &pkg.groupons {
            if groupon_map.contains_key(&g.groupon_promotion_id) {
                bump(counts, "grouponsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve_opt(g.shop_id) else {
                failures.push(json!({
                    "domain": "groupon", "sourceId": 0, "name": g.name,
                    "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            let mut g2 = g.clone();
            g2.groupon_image = g.groupon_image.as_deref().map(|u| apply_url_map(u, url_pairs));
            g2.pc_groupon_image = g.pc_groupon_image.as_deref().map(|u| apply_url_map(u, url_pairs));
            g2.share_image = g.share_image.as_deref().map(|u| apply_url_map(u, url_pairs));
            match p3.insert_groupon(&g2, Some(shop), i64::from(now)).await {
                Ok(new_id) => {
                    if let (Ok(s32), Ok(t32)) = (
                        i32::try_from(g.groupon_promotion_id),
                        i32::try_from(new_id),
                    ) {
                        let _ = repo
                            .insert_map(fingerprint, "groupon", s32, &g.name, t32, job_id, now)
                            .await;
                    }
                    groupon_map.insert(g.groupon_promotion_id, new_id);
                    bump(counts, "grouponsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "groupon", "sourceId": 0, "name": g.name, "error": e.to_string(),
                })),
            }
        }
        for it in &pkg.groupon_items {
            let Some(parent) = groupon_map.get(&it.groupon_promotion_id).copied() else {
                continue;
            };
            let src_pid = i32::try_from(it.product_id).unwrap_or(0);
            let Some(product) = refs.products.get(&src_pid).copied() else {
                unmapped.push("product", src_pid);
                bump(counts, "grouponItemsSkipped");
                continue;
            };
            match p3
                .insert_groupon_item(it, parent, i64::from(product), i64::from(now))
                .await
            {
                Ok(new_item) => {
                    groupon_item_map.insert(it.groupon_promotion_item_id, new_item);
                    bump(counts, "grouponItemsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "groupon_item", "sourceId": 0, "name": "", "error": e.to_string(),
                })),
            }
        }
        for d in &pkg.groupon_details {
            let (Some(parent), Some(item)) = (
                groupon_map.get(&d.groupon_promotion_id).copied(),
                groupon_item_map.get(&d.groupon_promotion_item_id).copied(),
            ) else {
                continue;
            };
            let src_pid = i32::try_from(d.product_id).unwrap_or(0);
            let Some(product) = refs.products.get(&src_pid).copied() else {
                unmapped.push("product", src_pid);
                continue;
            };
            let src_sku = i32::try_from(d.sku_id).unwrap_or(0);
            let sku_id = match sku.resolve(p3, src_sku).await {
                Some(s) => s,
                None => {
                    if src_sku > 0 {
                        unmapped.push("sku", src_sku);
                    }
                    bump(counts, "grouponDetailsSkipped");
                    continue;
                }
            };
            if let Err(e) = p3
                .insert_groupon_detail(d, parent, item, i64::from(product), i64::from(sku_id), i64::from(now))
                .await
            {
                failures.push(json!({
                    "domain": "groupon_detail", "sourceId": 0, "name": "", "error": e.to_string(),
                }));
            } else {
                bump(counts, "grouponDetailsCreated");
            }
        }
        for row in &pkg.groupon_shops {
            let Some(src_pid) = row.groupon_promotion_id else { continue };
            let Some(parent) = groupon_map.get(&i64::from(src_pid)).copied() else {
                continue;
            };
            let mut out: Vec<i32> = Vec::new();
            for sid in parse_id_list(row.shop_ids.as_deref().unwrap_or("")) {
                match shops.resolve(sid) {
                    Some(t) => out.push(t),
                    None => unmapped.push("shop", sid),
                }
            }
            if out.is_empty() {
                continue;
            }
            let _ = p3.insert_groupon_shops(parent, &java_list(&out)).await;
        }

        // —— 限时折扣 ——
        for t in &pkg.time_discounts {
            if td_map.contains_key(&t.discount_id) {
                bump(counts, "timeDiscountsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve(t.shop_id) else {
                failures.push(json!({
                    "domain": "time_discount", "sourceId": t.discount_id, "name": t.promotion_name,
                    "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            match p3.insert_time_discount(t, shop, now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "time_discount", t.discount_id, &t.promotion_name, new_id, job_id, now)
                        .await;
                    td_map.insert(t.discount_id, new_id);
                    bump(counts, "timeDiscountsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "time_discount", "sourceId": t.discount_id, "name": t.promotion_name,
                    "error": e.to_string(),
                })),
            }
        }
        for it in &pkg.time_discount_items {
            let Some(parent) = td_map.get(&it.discount_id).copied() else {
                continue;
            };
            let Some(product) = refs.products.get(&it.product_id).copied() else {
                unmapped.push("product", it.product_id);
                bump(counts, "timeDiscountItemsSkipped");
                continue;
            };
            let src_skus = parse_id_list(it.sku_ids.as_deref().unwrap_or(""));
            let mut out_skus: Vec<i32> = Vec::new();
            for sid in &src_skus {
                match sku.resolve(p3, *sid).await {
                    Some(s) => out_skus.push(s),
                    None => unmapped.push("sku", *sid),
                }
            }
            if !src_skus.is_empty() && out_skus.is_empty() {
                // 源限定了规格而本站一个都没对齐：整行跳过，
                // 绝不放大为「全部规格生效」。
                bump(counts, "timeDiscountItemsSkipped");
                continue;
            }
            match p3.insert_time_discount_item(it, parent, product, &java_list(&out_skus)).await {
                Ok(()) => bump(counts, "timeDiscountItemsCreated"),
                Err(e) => failures.push(json!({
                    "domain": "time_discount_item", "sourceId": it.item_id, "name": "",
                    "error": e.to_string(),
                })),
            }
        }

        // —— 满减/折扣/满赠 ——
        for p in &pkg.product_promotions {
            if pp_map.contains_key(&p.promotion_id) {
                bump(counts, "productPromotionsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve(p.shop_id) else {
                failures.push(json!({
                    "domain": "product_promotion", "sourceId": p.promotion_id,
                    "name": p.promotion_name, "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            let range_data = remap_ranged_ids(p.range, &p.range_data, refs, &mut unmapped);
            // 阶梯 JSON 内的 gift_id 按赠品映射重写（对不齐置 0）。
            let mut type_data: Value =
                serde_json::from_str(&p.promotion_type_data).unwrap_or(Value::Array(Vec::new()));
            if let Some tiers) = type_data.as_array_mut() {
                for tier in tiers.iter_mut() {
                    if let Some(obj) = tier.as_object_mut() {
                        if let Some(gid) = obj.get("giftId").and_then(Value::as_i64) {
                            let gid32 = i32::try_from(gid).unwrap_or(0);
                            match gift_map.get(&gid32) {
                                Some(t) => {
                                    obj.insert("giftId".into(), json!(t));
                                }
                                None => {
                                    if gid32 > 0 {
                                        unmapped.push("product_gift", gid32);
                                    }
                                    obj.insert("giftId".into(), json!(0));
                                }
                            }
                        }
                    }
                }
            }
            match p3
                .insert_product_promotion(p, shop, &range_data, &type_data.to_string())
                .await
            {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "product_promotion", p.promotion_id, &p.promotion_name, new_id, job_id, now)
                        .await;
                    pp_map.insert(p.promotion_id, new_id);
                    bump(counts, "productPromotionsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "product_promotion", "sourceId": p.promotion_id,
                    "name": p.promotion_name, "error": e.to_string(),
                })),
            }
        }

        // —— 积分兑换 ——
        let mut exchange_map = self.load_map(fingerprint, "points_exchange").await.unwrap_or_default();
        for x in &pkg.points_exchanges {
            if exchange_map.contains_key(&x.id) {
                bump(counts, "pointsExchangesSkipped");
                continue;
            }
            let coupon = if x.exchange_type == 2 {
                match x.coupon_id.and_then(|cid| coupon_map.get(&cid).copied()) {
                    Some(c) => Some(c),
                    None => {
                        failures.push(json!({
                            "domain": "points_exchange", "sourceId": x.id, "name": "",
                            "error": "兑换目标优惠券未对齐本站，已跳过",
                        }));
                        continue;
                    }
                }
            } else {
                None
            };
            let Some(product) = refs.products.get(&x.product_id).copied() else {
                unmapped.push("product", x.product_id);
                failures.push(json!({
                    "domain": "points_exchange", "sourceId": x.id, "name": "",
                    "error": "兑换商品未对齐本站，已跳过",
                }));
                continue;
            };
            let sku_id = match x.sku_id {
                Some(sid) if sid > 0 => match sku.resolve(p3, sid).await {
                    Some(s) => Some(s),
                    None => {
                        unmapped.push("sku", sid);
                        None
                    }
                },
                _ => None,
            };
            match p3.insert_points_exchange(x, product, sku_id, coupon).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "points_exchange", x.id, "", new_id, job_id, now)
                        .await;
                    exchange_map.insert(x.id, new_id);
                    bump(counts, "pointsExchangesCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "points_exchange", "sourceId": x.id, "name": "",
                    "error": e.to_string(),
                })),
            }
        }

        // —— 充值档位 ——
        let mut recharge_map = self.load_map(fingerprint, "recharge").await.unwrap_or_default();
        for r in &pkg.recharge_settings {
            if recharge_map.contains_key(&r.recharge_id) {
                bump(counts, "rechargesSkipped");
                continue;
            }
            let gift = if r.gift_id > 0 {
                match gift_map.get(&r.gift_id) {
                    Some(t) => *t,
                    None => {
                        unmapped.push("product_gift", r.gift_id);
                        0
                    }
                }
            } else {
                0
            };
            match p3.insert_recharge_setting(r, gift).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "recharge", r.recharge_id, "", new_id, job_id, now)
                        .await;
                    recharge_map.insert(r.recharge_id, new_id);
                    bump(counts, "rechargesCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "recharge", "sourceId": r.recharge_id, "name": "",
                    "error": e.to_string(),
                })),
            }
        }

        // —— 签到规则（按 day_num 对齐，平台级） ——
        let signin_map = self.load_map(fingerprint, "signin").await.unwrap_or_default();
        for s in &pkg.sign_in_settings {
            if signin_map.contains_key(&s.id) {
                bump(counts, "signInsSkipped");
                continue;
            }
            if let Some(day) = s.day_num {
                match p3.find_sign_in_by_day(day).await {
                    Ok(Some(existing)) => {
                        // 本站已有同 day_num 规则：登记 exist: 映射防翻倍，
                        // 撤销时按前缀跳过（不删本站原有行）。
                        let _ = repo
                            .insert_map(
                                fingerprint,
                                "signin",
                                s.id,
                                &format!("exist:{day}"),
                                existing,
                                job_id,
                                now,
                            )
                            .await;
                        bump(counts, "signInsSkipped");
                        continue;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        failures.push(json!({
                            "domain": "signin", "sourceId": s.id, "name": "",
                            "error": e.to_string(),
                        }));
                        continue;
                    }
                }
            }
            match p3.insert_sign_in_setting(s, now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(
                            fingerprint,
                            "signin",
                            s.id,
                            s.name.as_deref().unwrap_or(""),
                            new_id,
                            job_id,
                            now,
                        )
                        .await;
                    bump(counts, "signInsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "signin", "sourceId": s.id, "name": "", "error": e.to_string(),
                })),
            }
        }

        // —— 礼品卡组 + 商品池 + 适用范围 ——
        for g in &pkg.gift_card_groups {
            if gcg_map.contains_key(&g.card_groups_id) {
                bump(counts, "giftCardGroupsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve_opt(g.shop_id) else {
                failures.push(json!({
                    "domain": "gift_card_group", "sourceId": g.card_groups_id,
                    "name": g.card_group_name, "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            let mut g2 = g.clone();
            g2.image_url = g.image_url.as_deref().map(|u| apply_url_map(u, url_pairs));
            match p3.insert_gift_card_group(&g2, Some(shop), now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "gift_card_group", g.card_groups_id, &g.card_group_name, new_id, job_id, now)
                        .await;
                    gcg_map.insert(g.card_groups_id, new_id);
                    bump(counts, "giftCardGroupsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "gift_card_group", "sourceId": g.card_groups_id,
                    "name": g.card_group_name, "error": e.to_string(),
                })),
            }
        }
        for gp in &pkg.gift_card_products {
            let Some(group) = gcg_map.get(&gp.card_groups_id).copied() else {
                continue;
            };
            let Some(product) = refs.products.get(&gp.product_id).copied() else {
                unmapped.push("product", gp.product_id);
                continue;
            };
            let sku_id = if gp.sku_id > 0 {
                match sku.resolve(p3, gp.sku_id).await {
                    Some(s) => s,
                    None => {
                        unmapped.push("sku", gp.sku_id);
                        0
                    }
                }
            } else {
                0
            };
            let _ = p3.insert_gift_card_product(group, product, sku_id).await;
        }
        for r in &pkg.gift_scope_products {
            let Some(group) = gcg_map.get(&r.card_groups_id).copied() else {
                continue;
            };
            match refs.products.get(&r.ref_id) {
                Some(p) => {
                    let _ = p3.insert_gift_scope_product(group, *p).await;
                }
                None => unmapped.push("product", r.ref_id),
            }
        }
        for r in &pkg.gift_scope_categories {
            let Some(group) = gcg_map.get(&r.card_groups_id).copied() else {
                continue;
            };
            match refs.categories.get(&r.ref_id) {
                Some(c) => {
                    let _ = p3.insert_gift_scope_category(group, *c).await;
                }
                None => unmapped.push("category", r.ref_id),
            }
        }

        // —— 抽奖 ——
        for a in &pkg.lotteries {
            if lottery_map.contains_key(&a.activity_id) {
                bump(counts, "lotteriesSkipped");
                continue;
            }
            let Some(shop) = shops.resolve(a.shop_id) else {
                failures.push(json!({
                    "domain": "lottery", "sourceId": a.activity_id, "name": a.activity_name,
                    "error": "所属店铺未建壳，已跳过",
                }));
                continue;
            };
            // decorate_config 走装修同款变换（商品/分类引用 + URL）。
            let decorate_config = a.decorate_config.as_deref().map(|raw| {
                let mut ctx = TransformCtx { refs, urls: url_pairs, unmapped: Vec::new() };
                let out = transform_json_text(raw, &mut ctx);
                for (kind, id) in ctx.unmapped {
                    unmapped.push(&kind, id);
                }
                out
            });
            let mut a2 = a.clone();
            a2.no_win_image = apply_url_map(&a.no_win_image, url_pairs);
            match p3.insert_lottery(&a2, shop, decorate_config.as_deref(), now).await {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "lottery", a.activity_id, &a.activity_name, new_id, job_id, now)
                        .await;
                    lottery_map.insert(a.activity_id, new_id);
                    bump(counts, "lotteriesCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "lottery", "sourceId": a.activity_id, "name": a.activity_name,
                    "error": e.to_string(),
                })),
            }
        }
        for pz in &pkg.lottery_prizes {
            let Some(activity) = lottery_map.get(&pz.activity_id).copied() else {
                continue;
            };
            let value: String = match pz.prize_type {
                2 => {
                    let src = pz.prize_value.trim().parse::<i32>().unwrap_or(0);
                    match refs.products.get(&src) {
                        Some(t) => t.to_string(),
                        None => {
                            unmapped.push("product", src);
                            failures.push(json!({
                                "domain": "lottery_prize", "sourceId": pz.prize_id,
                                "name": pz.prize_name,
                                "error": "奖品商品未对齐本站，已跳过该奖品",
                            }));
                            continue;
                        }
                    }
                }
                3 => {
                    let src = pz.prize_value.trim().parse::<i32>().unwrap_or(0);
                    match coupon_map.get(&src) {
                        Some(t) => t.to_string(),
                        None => {
                            unmapped.push("coupon", src);
                            failures.push(json!({
                                "domain": "lottery_prize", "sourceId": pz.prize_id,
                                "name": pz.prize_name,
                                "error": "奖品优惠券未对齐本站，已跳过该奖品",
                            }));
                            continue;
                        }
                    }
                }
                _ => pz.prize_value.clone(),
            };
            let mut pz2 = pz.clone();
            pz2.prize_image = apply_url_map(&pz.prize_image, url_pairs);
            match p3.insert_lottery_prize(&pz2, activity, &value, now).await {
                Ok(()) => bump(counts, "lotteryPrizesCreated"),
                Err(e) => failures.push(json!({
                    "domain": "lottery_prize", "sourceId": pz.prize_id, "name": pz.prize_name,
                    "error": e.to_string(),
                })),
            }
        }

        // —— promotion 镜像行（最后：依赖全部活动映射） ——
        let mut promotion_map = self.load_map(fingerprint, "promotion").await.unwrap_or_default();
        for row in &pkg.promotions {
            if promotion_map.contains_key(&row.promotion_id) {
                bump(counts, "promotionsSkipped");
                continue;
            }
            let Some(shop) = shops.resolve(row.shop_id) else {
                continue;
            };
            let relation = match row.r#type {
                1 => row.relation_id.and_then(|id| seckill_map.get(&id).copied()),
                2 => row.relation_id.and_then(|id| coupon_map.get(&id).copied()),
                3 | 4 | 5 => row.relation_id.and_then(|id| pp_map.get(&id).copied()),
                6 => row.relation_id.and_then(|id| td_map.get(&id).copied()),
                8 => row.relation_id.and_then(|id| lottery_map.get(&id).copied()),
                _ => None,
            };
            let Some(relation) = relation else {
                bump(counts, "promotionsSkipped");
                continue;
            };
            let range_data = match &row.range_data {
                Some(raw) => Some(remap_ranged_ids(row.range.unwrap_or(0), raw, refs, &mut unmapped)),
                None => None,
            };
            let sku_ids = match &row.sku_ids {
                Some(raw) => {
                    let src = parse_id_list(raw);
                    if src.is_empty() {
                        Some(raw.clone())
                    } else {
                        let mut out: Vec<i32> = Vec::new();
                        for sid in &src {
                            match sku.resolve(p3, *sid).await {
                                Some(s) => out.push(s),
                                None => unmapped.push("sku", *sid),
                            }
                        }
                        if out.is_empty() {
                            // 镜像行限定了 SKU 而本站全未对齐：跳过整行，
                            // 不落无范围限定的镜像（防语义放大）。
                            bump(counts, "promotionsSkipped");
                            continue;
                        }
                        Some(java_list(&out))
                    }
                }
                None => None,
            };
            match p3
                .insert_promotion(row, shop, relation, range_data.as_deref(), sku_ids.as_deref())
                .await
            {
                Ok(new_id) => {
                    let _ = repo
                        .insert_map(fingerprint, "promotion", row.promotion_id, &row.promotion_name, new_id, job_id, now)
                        .await;
                    promotion_map.insert(row.promotion_id, new_id);
                    bump(counts, "promotionsCreated");
                }
                Err(e) => failures.push(json!({
                    "domain": "promotion", "sourceId": row.promotion_id, "name": row.promotion_name,
                    "error": e.to_string(),
                })),
            }
        }

        unmapped.flush_into(counts, failures);
    }
}

// ———————————————————— 离线包 ————————————————————

/// 离线包签名规范串：文件按名排序后逐个 `name\nsha256\n` 拼接。
fn offline_canonical(files: &[CloneOfflineFile]) -> String {
    let mut sorted: Vec<&CloneOfflineFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    let mut s = String::new();
    for f in sorted {
        s.push_str(&f.name);
        s.push('\n');
        s.push_str(&f.sha256);
        s.push('\n');
    }
    s
}

/// HMAC-SHA256（键 = secret 的 SHA-256 hex 串，即落库 secret_hash）。
fn offline_sign(secret_hash: &str, canonical: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret_hash.as_bytes())
        .expect("HMAC 接受任意长度密钥");
    mac.update(canonical.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

impl SiteCloneService {
    /// 导出离线包（管理端）：按密钥授权域组装各端点载荷为一个 JSON
    /// 文件（逐文件 sha + 整包 HMAC 签名）。组装走与在线协议完全
    /// 相同的导出函数，离线包内容 ≡ 在线拉取内容。
    pub async fn export_offline_package(&self, key_id: i32, origin: &str) -> Result<Value> {
        let key = self
            .repo()
            .find_key_by_id(key_id)
            .await?
            .ok_or_else(|| Error::biz(404, "搬家密钥不存在"))?;
        let mut files: Vec<CloneOfflineFile> = Vec::new();
        let mut add = |name: &str, value: Value| {
            let content = value.to_string();
            let sha256 = utils::sha256_hex(&content);
            files.push(CloneOfflineFile { name: name.to_string(), sha256, content });
        };

        let manifest = self.manifest(&key).await?;
        let source_label = manifest.site_name.clone();
        add("manifest", serde_json::to_value(&manifest).unwrap_or(Value::Null));
        if key.has_domain(DOMAIN_PRODUCTS) {
            add("catalog", serde_json::to_value(self.catalog(&key).await?).unwrap_or(Value::Null));
            // 商品全量：分页拉满（与导入端 200 页上限同界）。
            let mut all: Vec<Value> = Vec::new();
            let mut page = 1u64;
            loop {
                let p = self.products_page(&key, page, 100).await?;
                let total = p.total;
                let batch = serde_json::to_value(&p.list).unwrap_or(Value::Array(Vec::new()));
                let n = batch.as_array().map_or(0, Vec::len);
                if let Value::Array(items) = batch {
                    all.extend(items);
                }
                if n == 0 || (all.len() as u64) >= total || page >= 200 {
                    break;
                }
                page += 1;
            }
            add("products", json!({ "list": all, "total": all.len() }));
        }
        if key.has_domain(DOMAIN_SETTINGS) {
            add("settings", serde_json::to_value(self.settings_package().await?).unwrap_or(Value::Null));
        }
        if key.has_domain(DOMAIN_DECORATE) {
            add("decorate", serde_json::to_value(self.decorate_package(&key).await?).unwrap_or(Value::Null));
        }
        if key.has_domain(DOMAIN_ASSETS) {
            add("assets", serde_json::to_value(self.assets_package(&key).await?).unwrap_or(Value::Null));
        }
        if key.has_domain(DOMAIN_MARKETING) {
            add("marketing", serde_json::to_value(self.marketing_package(&key).await?).unwrap_or(Value::Null));
        }
        if key.has_domain(DOMAIN_CONTENT) {
            add("content", serde_json::to_value(self.content_package().await?).unwrap_or(Value::Null));
        }
        if key.has_domain(DOMAIN_MAIL) {
            add("mail", serde_json::to_value(self.mail_package().await?).unwrap_or(Value::Null));
        }
        if key.has_domain(DOMAIN_SHOPS) {
            add("shops", serde_json::to_value(self.shops_package().await?).unwrap_or(Value::Null));
        }

        let signature = offline_sign(&key.secret_hash, &offline_canonical(&files));
        let pkg = CloneOfflinePackage {
            format: OFFLINE_PACKAGE_FORMAT.to_string(),
            package_version: OFFLINE_PACKAGE_VERSION,
            key_code: key.key_code.clone(),
            source_label,
            source_origin: origin.trim().trim_end_matches('/').to_string(),
            source_shop_id: key.source_shop_id,
            exporter_version: EXPORTER_VERSION,
            created_at: utils::now_seconds() as i32,
            files,
            signature,
        };
        Ok(serde_json::to_value(&pkg).unwrap_or(Value::Null))
    }

    /// 离线包验签并构造离线数据源：格式/密钥归属 → 逐文件 sha →
    /// HMAC 验签（键由口令 secret 现算的哈希，与导出端落库哈希同值）。
    /// 任一不过直接报错、不建任务；通过后与在线导入共用同一编排。
    pub fn verify_offline_source(
        &self,
        dto: &CloneExecuteDTO,
        pkg_value: &Value,
    ) -> Result<CloneSource> {
        let pkg: CloneOfflinePackage = serde_json::from_value(pkg_value.clone())
            .map_err(|e| Error::biz(400, &format!("离线包格式不正确: {e}")))?;
        if pkg.format != OFFLINE_PACKAGE_FORMAT {
            return Err(Error::biz(400, "离线包格式不正确（format 标识不符）"));
        }
        let mut cred = resolve_source(&dto.source)?;
        if pkg.key_code != cred.key_code {
            return Err(Error::biz(400, "离线包与所填口令不属于同一把密钥"));
        }
        let mut files: HashMap<String, Value> = HashMap::new();
        for f in &pkg.files {
            if utils::sha256_hex(&f.content) != f.sha256 {
                return Err(Error::biz(400, &format!("离线包文件校验失败（{} 的 sha256 不符，包已损坏或被篡改）", f.name)));
            }
            let value: Value = serde_json::from_str(&f.content)
                .map_err(|e| Error::biz(400, &format!("离线包文件 {} 内容损坏: {e}", f.name)))?;
            files.insert(f.name.clone(), value);
        }
        let expect = offline_sign(&utils::sha256_hex(&cred.secret), &offline_canonical(&pkg.files));
        if !constant_time_eq(expect.as_bytes(), pkg.signature.as_bytes()) {
            return Err(Error::biz(400, "离线包验签失败（签名不符，包不是该密钥签发的或已被篡改）"));
        }
        if !pkg.source_origin.is_empty() {
            cred.origin = pkg.source_origin.trim_end_matches('/').to_string();
        }
        Ok(CloneSource::offline(cred, files))
    }
}

// ———————————————————— P3 源站拉取 ————————————————————

async fn fetch_marketing(src: &CloneSource) -> Result<CloneMarketingPackage> {
    let data = fetch_json(src, "marketing", "/api/clone/v1/marketing").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站营销包解析失败: {e}")))
}

async fn fetch_content(src: &CloneSource) -> Result<CloneContentPackage> {
    let data = fetch_json(src, "content", "/api/clone/v1/content").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站文章包解析失败: {e}")))
}

async fn fetch_mail(src: &CloneSource) -> Result<CloneMailPackage> {
    let data = fetch_json(src, "mail", "/api/clone/v1/mail").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站邮件包解析失败: {e}")))
}

async fn fetch_shops(src: &CloneSource) -> Result<CloneShopsPackage> {
    let data = fetch_json(src, "shops", "/api/clone/v1/shops").await?;
    serde_json::from_value(data)
        .map_err(|e| Error::Validation(format!("来源站店铺壳包解析失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_list_matches_java_tostring() {
        assert_eq!(java_list(&[1, 2, 3]), "[1, 2, 3]");
        assert_eq!(java_list(&[]), "[]");
    }

    #[test]
    fn offline_sign_roundtrip_and_tamper_detection() {
        let secret_hash = utils::sha256_hex("test-secret");
        let content = "{\"a\":1}".to_string();
        let files = vec![CloneOfflineFile {
            name: "manifest".into(),
            sha256: utils::sha256_hex(&content),
            content,
        }];
        let sig = offline_sign(&secret_hash, &offline_canonical(&files));
        // 同密钥验签通过、异密钥失败、文件改一字 sha 即不符。
        assert_eq!(sig, offline_sign(&secret_hash, &offline_canonical(&files)));
        assert_ne!(sig, offline_sign(&utils::sha256_hex("other"), &offline_canonical(&files)));
        let tampered = "{\"a\":2}";
        assert_ne!(utils::sha256_hex(tampered), files[0].sha256);
    }

    #[test]
    fn ranged_ids_remap_by_kind() {
        let mut refs = RefMaps::default();
        refs.categories.insert(7, 70);
        refs.products.insert(9, 90);
        let mut unmapped = UnmappedRefs::default();
        // 范围 1 = 分类：命中重写、未命中剔除。
        assert_eq!(remap_ranged_ids(1, "[7, 8]", &refs, &mut unmapped), "[70]");
        // 范围 3 = 商品。
        assert_eq!(remap_ranged_ids(3, "[9]", &refs, &mut unmapped), "[90]");
        // 范围 0 = 全场：原文不动、不记未对齐。
        assert_eq!(remap_ranged_ids(0, "[1, 2]", &refs, &mut unmapped), "[1, 2]");
        assert_eq!(unmapped.items.len(), 1);
    }
}
