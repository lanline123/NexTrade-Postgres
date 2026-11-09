//! SEO 域服务（content::seo）：站点地图出参、分类 SEO 链接反查、SEO 标识批量生成。
//!
//! 对应原 Java：
//! - `nextrade-adminapi` → `com.nextrade.adminapi.controller.setting.SeoController`
//!   （`@RequestMapping("/adminapi/setting/seo")`，3 个端点）
//! - `nextrade-api` → `com.nextrade.api.controller.sys.SeoController`
//!   （`@RequestMapping("/api/sys/seo")`，5 个端点）
//! - `com.nextrade.service.seo.impl.SeoServiceImpl`（全部 7 个方法）
//! - `com.nextrade.common.utils.SeoIdentificationUtil`（链接解析 / 英文格式化，
//!   已移植到 `nextrade_core::models::content::seo`）
//!
//! # 可完整迁移的链路（DB-only，行为逐条对齐 Java）
//!
//! | 服务方法 | Java | 说明 |
//! |----------|------|------|
//! | [`SeoService::get_category_id_by_seo_link`] | `getCategoryIdBySeoLink` | 解析链接 → 按 `seo_identification` 查分类 |
//! | [`SeoService::sitemap_products`] | `getSitemapProducts` | `product_status = 1 AND is_delete = 0` |
//! | [`SeoService::sitemap_categories`] | `getSitemapCategories` | `is_show = 1` |
//! | [`SeoService::sitemap_articles`] | `getSitemapArticles` | `is_show = 1` + 秒→毫秒换算 |
//! | [`SeoService::sitemap_brands`] | `getSitemapBrands` | 先取有上架商品的品牌，再取 `is_show = 1` |
//!
//! # 未迁移依赖（**如实拒绝，不伪造实现**）
//!
//! 1. **翻译服务（火山引擎机器翻译）**：Java 的
//!    `TranslationsService#getTranslations` / `#getTranslationsResponse` 依赖
//!    `com.volcengine` SDK + 配置项 `langVolcengineAccessKey` /
//!    `langVolcengineSecret`（`SettingsEnum`）+ 配置服务（`ConfigService`），
//!    本构建均未迁移。因此以下 3 条链路的**成功路径无法实现**，统一返回业务码
//!    1001 与明确文案：
//!    - `GET  /adminapi/setting/seo/translateToSeoIdentification`（中文→英文并格式化）
//!    - `POST /adminapi/setting/seo/batchUpdateCategoryLink`
//!    - `POST /adminapi/setting/seo/batchUpdateProductLink`
//!
//!    > ⚠️ 与原 Java 的行为差异（**有意为之**）：Java 的
//!    > `generateAndUpdateCategorySeoIdentification` /
//!    > `generateAndUpdateProductSeoIdentification` 标注了 `@Async` + `void`，
//!    > 接口**总是立即返回成功**，翻译失败只在日志里 `log.error`（静默不生效）；
//!    > 且方法体内部用 `try { ... } catch (Exception e)` 吞掉全部异常。
//!    > Rust 侧没有等价的后台任务基础设施，若照抄「返回成功」会变成
//!    > **静默空操作**（更难排查），故按项目约定返回显式业务错误。
//!
//! 2. **Redis 兜底查询**：Java `getCategoryIdBySeoLink` 在库内查不到分类时，会再查
//!    `redisCache.getCacheObject(SEO_LINK_CATEGORY + seoIdentification)`
//!    （即 `seo_link_category::<seoIdentification>`）并用其值回表
//!    `categoryMapper.selectById(...)`。本构建服务层未注入 Redis
//!    （`SeoService::new` 只接 `Arc<Repositories>`），故省略该兜底。
//!    **影响评估**：全仓库 grep 显示 `seo_link_category::*` 只有**删除**
//!    （`CategoryServiceImpl`：`seo_link_category::<categoryId>`、
//!    `seo_link_category::getCatFloor`），**没有任何写入点**，该兜底实际永不命中，
//!    省略与 Java 行为等价（详见 `uncertain_points`）。
//!
//! 3. **`SeoResponseAdvice`**：Java 侧曾用于给响应体回填 `seoLink`，当前实现是
//!    直通 no-op（`beforeBodyWrite` 直接 `return body`），故本切片不做响应改写。

use std::sync::Arc;

use nextrade_core::models::content::seo::{
    article_last_update_millis, extract_seo_identification_from_link, format_english_text,
    non_blank, SeoArticleVO, SeoBrandVO, SeoCategoryVO, SeoProductVO,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::seo::SeoRepository;
use nextrade_infrastructure::db::Repositories;

/// 通用业务失败码（Java `SERVICE_DATA_ERROR`，也是 `GlobalException` 的默认码）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `SeoServiceImpl.MAX_BATCH_TRANSLATE_SIZE`：批量翻译单批最大条数。
///
/// 保留在 Rust 侧作为对照常量（批量链路因翻译服务未迁移而不可用）。
pub const MAX_BATCH_TRANSLATE_SIZE: usize = 50;

/// 翻译服务未迁移时的统一错误文案（`translateToSeoIdentification`）。
///
/// Java 在密钥缺失时抛的文案是「请检查语言翻译服务密钥配置」；本构建连
/// 火山引擎 SDK 与配置服务都未迁移，故给出更前置、可操作的说明。
pub const TRANSLATION_UNMIGRATED_MSG: &str = "翻译服务尚未迁移（依赖火山引擎机器翻译 SDK 与配置项 langVolcengineAccessKey/langVolcengineSecret），无法完成中文转英文，请在 Java 侧执行或先迁移翻译服务";

/// SEO 标识批量生成未迁移时的统一错误文案。
pub const BATCH_SEO_UNMIGRATED_MSG: &str = "批量生成 SEO 标识依赖尚未迁移的翻译服务（火山引擎机器翻译 SDK）与 Redis 缓存清理，Rust 侧无法执行，请在 Java 侧执行或先迁移翻译服务";

/// Java 里分类 SEO 链接缓存的键前缀（`Constants.SEO_LINK_CATEGORY`）。
///
/// 保留常量用于对照；本构建未注入 Redis（见模块头「未迁移依赖」第 2 条）。
#[allow(dead_code)]
const SEO_LINK_CATEGORY_CACHE_PREFIX: &str = "seo_link_category::";

/// `translateToSeoIdentification` 的入参校验（对应 Java 的必填
/// `@RequestParam(value = "chineseText")`）。
///
/// Java 缺参由 `MissingServletRequestParameterException` 兜底为 1001；
/// 本实现同样给出 1001，并额外拒绝空白串（Java 能进方法体，但翻译空白无意义）。
///
/// # Errors
/// 业务码 1001「chineseText 不能为空」。
pub fn validate_chinese_text(chinese_text: Option<&str>) -> Result<&str> {
    chinese_text
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "chineseText 不能为空"))
}

/// SEO 域服务。
pub struct SeoService {
    repos: Arc<Repositories>,
}

impl SeoService {
    /// 构造服务。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    fn repo(&self) -> &SeoRepository {
        &self.repos.content.seo
    }

    // ——————————————————————————————————————————————
    // GET /adminapi/setting/seo/translateToSeoIdentification
    // ——————————————————————————————————————————————

    /// 中文转英文并格式化为 SEO 标识。
    ///
    /// 对应 Java `SeoController#translateToEnglish` →
    /// `TranslationsService#getTranslations("en", chineseText)` +
    /// `SeoIdentificationUtil#formatEnglishText`。
    ///
    /// **未迁移**：翻译服务不可用（见模块头），因此这里在完成入参校验后
    /// 直接返回业务码 1001 的明确错误，**不会**退化成「对中文直接跑格式化」
    /// （那样会把中文全部删空，得到空串并静默写坏数据）。
    ///
    /// # Errors
    /// - 业务码 1001：`chineseText` 缺失/空白；
    /// - 业务码 1001：[`TRANSLATION_UNMIGRATED_MSG`]（翻译服务未迁移）。
    pub async fn translate_to_seo_identification(
        &self,
        chinese_text: Option<&str>,
    ) -> Result<String> {
        let text = validate_chinese_text(chinese_text)?;
        let client = crate::content::volcengine_translate::VolcengineTranslateClient::from_repos(
            &self.repos,
        )
        .await?;
        let english = client.translate_text("en", text).await?;
        let seo = format_english_text(&english);
        if seo.is_empty() {
            return Err(Error::biz(
                SERVICE_DATA_ERROR,
                "翻译结果格式化为空，无法生成 SEO 标识",
            ));
        }
        Ok(seo)
    }

    // ——————————————————————————————————————————————
    // POST /adminapi/setting/seo/batchUpdateCategoryLink
    // ——————————————————————————————————————————————

    /// 商品类目链接批量更新。
    ///
    /// 对应 Java `SeoController#batchUpdateCategoryLink` →
    /// `SeoServiceImpl#generateAndUpdateCategorySeoIdentification`
    /// （`@Async` + `@Transactional`）：
    /// 1. 查 `seo_identification` 为 NULL 或 `''` 的分类；
    /// 2. 过滤掉 `category_name` 为空的分类；
    /// 3. 按 [`MAX_BATCH_TRANSLATE_SIZE`] 分批调用翻译服务（`"en"`）；
    /// 4. 逐条 `UPDATE category SET seo_identification = ?`；
    /// 5. 有过更新时清理 `seo_link_category::getCatFloor*` Redis 缓存。
    ///
    /// **未迁移**：第 3 步（翻译服务）与第 5 步（Redis 缓存）均不可用，
    /// 前两步单独执行不产生任何效果，故整条链路显式拒绝（业务码 1001）。
    /// 详见模块头「未迁移依赖」第 1 条对 `@Async` 语义差异的说明。
    ///
    /// # Errors
    /// 恒定返回 [`BATCH_SEO_UNMIGRATED_MSG`]（业务码 1001）。
    pub async fn batch_update_category_link(&self) -> Result<()> {
        let client = crate::content::volcengine_translate::VolcengineTranslateClient::from_repos(
            &self.repos,
        )
        .await?;
        let rows = self.repo().find_categories_missing_seo().await?;
        let mut updated = 0u32;
        for batch in rows.chunks(MAX_BATCH_TRANSLATE_SIZE) {
            for (category_id, category_name) in batch {
                let english = client.translate_text("en", category_name).await?;
                let seo = format_english_text(&english);
                if seo.is_empty() {
                    continue;
                }
                if self.repo().update_category_seo(*category_id, &seo).await? > 0 {
                    updated += 1;
                }
            }
        }
        tracing::info!("批量生成分类SEO标识完成，更新数量: {updated}");
        Ok(())
    }

    // ——————————————————————————————————————————————
    // POST /adminapi/setting/seo/batchUpdateProductLink
    // ——————————————————————————————————————————————

    /// 商品链接批量更新。
    ///
    /// 对应 Java `SeoController#batchUpdateProductLink` →
    /// `SeoServiceImpl#generateAndUpdateProductSeoIdentification`
    /// （`@Async` + `@Transactional`）：
    /// 1. 查 `seo_identification` 为 NULL 或 `''` 的商品；
    /// 2. 过滤掉 `product_name` 为空的商品；
    /// 3. 分批翻译（`"en"`）；
    /// 4. 逐条 `UPDATE product SET seo_identification = ?`。
    ///
    /// 与分类批量更新不同，Java 的**商品**批量更新**没有** Redis 清理步骤。
    ///
    /// **未迁移**：同 [`SeoService::batch_update_category_link`]。
    ///
    /// # Errors
    /// 恒定返回 [`BATCH_SEO_UNMIGRATED_MSG`]（业务码 1001）。
    pub async fn batch_update_product_link(&self) -> Result<()> {
        let client = crate::content::volcengine_translate::VolcengineTranslateClient::from_repos(
            &self.repos,
        )
        .await?;
        let rows = self.repo().find_products_missing_seo().await?;
        let mut updated = 0u32;
        for batch in rows.chunks(MAX_BATCH_TRANSLATE_SIZE) {
            for (product_id, product_name) in batch {
                let english = client.translate_text("en", product_name).await?;
                let seo = format_english_text(&english);
                if seo.is_empty() {
                    continue;
                }
                if self.repo().update_product_seo(*product_id, &seo).await? > 0 {
                    updated += 1;
                }
            }
        }
        tracing::info!("批量生成商品SEO标识完成，更新数量: {updated}");
        Ok(())
    }

    // ——————————————————————————————————————————————
    // GET /api/sys/seo/getCategoryIdBySeoLink
    // ——————————————————————————————————————————————

    /// 商品分类 `seoLink` 转商品分类 ID。
    ///
    /// 对应 Java `SeoServiceImpl#getCategoryIdBySeoLink`：
    /// 1. `SeoIdentificationUtil#extractSeoIdentificationFromLink` 解析链接
    ///    （空 → `GlobalException("SEO链接不能为空")`）；
    /// 2. 按 `seo_identification` 查分类（`LIMIT 1`）；
    /// 3. 查不到 → Java 再查 Redis 兜底（本构建未迁移，见模块头），
    ///    最终抛 `GlobalException("未找到对应的商品分类")`。
    ///
    /// # Errors
    /// - 业务码 1001「SEO链接不能为空」；
    /// - 业务码 1001「未找到对应的商品分类」（含 Redis 兜底未迁移的情形）。
    pub async fn get_category_id_by_seo_link(&self, seo_link: Option<&str>) -> Result<i32> {
        let seo_identification = extract_seo_identification_from_link(seo_link)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "SEO链接不能为空"))?;

        if let Some(category_id) = self
            .repo()
            .find_category_id_by_seo_identification(&seo_identification)
            .await?
        {
            return Ok(category_id);
        }

        // 与 Java 的差异：Java 在此处还会查 `seo_link_category::<seoIdentification>`
        // 缓存并用其值回表。全仓库 grep 确认该键从未被写入（只有删除），
        // 故本构建省略该兜底（见模块头「未迁移依赖」第 2 条），直接给出业务错误。
        Err(Error::biz(SERVICE_DATA_ERROR, "未找到对应的商品分类"))
    }

    // ——————————————————————————————————————————————
    // GET /api/sys/seo/products
    // ——————————————————————————————————————————————

    /// 站点地图-商品列表（已上架且未删除）。
    ///
    /// 对应 Java `SeoServiceImpl#getSitemapProducts`：
    /// `product_status = 1 AND is_delete = 0`，投影
    /// `productSn / productName / seoIdentification / lastUpdate / picUrl`；
    /// `seoLink` 用 `StrUtil.isNotBlank(seoIdentification) ? ... : null`。
    ///
    /// ⚠️ Java 对商品**原样回传** `last_update`（秒级时间戳，未做秒→毫秒换算，
    /// 与文章列表不同）；本实现逐字对齐，不「顺手修正」。
    pub async fn sitemap_products(&self) -> Result<Vec<SeoProductVO>> {
        let rows = self.repo().find_sitemap_products().await?;
        Ok(rows
            .into_iter()
            .map(|r| SeoProductVO {
                product_sn: r.product_sn,
                product_name: r.product_name,
                seo_link: non_blank(r.seo_identification.as_deref()),
                last_update: Some(i64::from(r.last_update)),
                pic_url: r.pic_url,
            })
            .collect())
    }

    // ——————————————————————————————————————————————
    // GET /api/sys/seo/categories
    // ——————————————————————————————————————————————

    /// 站点地图-分类列表（`is_show = 1`，扁平化）。
    ///
    /// 对应 Java `SeoServiceImpl#getSitemapCategories`：
    /// `is_show = 1`，投影 `categoryId / categoryName / seoIdentification`；
    /// `id` 为**字符串**（`String.valueOf(categoryId)`），`lastUpdate` 固定 `null`。
    pub async fn sitemap_categories(&self) -> Result<Vec<SeoCategoryVO>> {
        let rows = self.repo().find_sitemap_categories().await?;
        Ok(rows
            .into_iter()
            .map(|r| SeoCategoryVO {
                id: r.category_id.to_string(),
                category_name: r.category_name,
                seo_link: non_blank(r.seo_identification.as_deref()),
                last_update: None,
            })
            .collect())
    }

    // ——————————————————————————————————————————————
    // GET /api/sys/seo/articles
    // ——————————————————————————————————————————————

    /// 站点地图-文章列表（`is_show = 1`）。
    ///
    /// 对应 Java `SeoServiceImpl#getSitemapArticles`：
    /// `is_show = 1`，投影 `articleId / articleTitle / addTime`；
    /// `id` 为**字符串**，`lastUpdate` 按
    /// [`article_last_update_millis`] 从秒换算为毫秒（`addTime <= 0` → `null`）。
    pub async fn sitemap_articles(&self) -> Result<Vec<SeoArticleVO>> {
        let rows = self.repo().find_sitemap_articles().await?;
        Ok(rows
            .into_iter()
            .map(|r| SeoArticleVO {
                id: r.article_id.to_string(),
                article_title: r.article_title,
                last_update: article_last_update_millis(r.add_time),
            })
            .collect())
    }

    // ——————————————————————————————————————————————
    // GET /api/sys/seo/brands
    // ——————————————————————————————————————————————

    /// 站点地图-品牌列表（有上架商品关联的品牌）。
    ///
    /// 对应 Java `SeoServiceImpl#getSitemapBrands`：
    /// 1. 取「`product_status = 1 AND is_delete = 0` 且 `brand_id` 非 NULL 且 `<> 0`」
    ///    的商品品牌 ID（去重）；
    /// 2. 为空 → **直接返回空列表**（Java `Collections.emptyList()`，不发第二条 SQL）；
    /// 3. 否则按 `brand_id IN (...) AND is_show = 1` 取品牌；
    /// 4. `id` 为**字符串**，`seoLink` 与 `lastUpdate` 固定 `null`
    ///    （`brand` 表没有 `seo_identification` 列）。
    pub async fn sitemap_brands(&self) -> Result<Vec<SeoBrandVO>> {
        let brand_ids = self.repo().find_active_brand_ids().await?;
        if brand_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self.repo().find_shown_brands_by_ids(&brand_ids).await?;
        Ok(rows
            .into_iter()
            .map(|r| SeoBrandVO {
                id: r.brand_id.to_string(),
                brand_name: r.brand_name,
                seo_link: None,
                last_update: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_code_matches_java_global_exception_default() {
        // Java `GlobalException(String)` → `SERVICE_DATA_ERROR`
        assert_eq!(SERVICE_DATA_ERROR, 1001);
        assert_eq!(MAX_BATCH_TRANSLATE_SIZE, 50);
        assert_eq!(SEO_LINK_CATEGORY_CACHE_PREFIX, "seo_link_category::");
    }

    #[test]
    fn unmigrated_messages_are_explicit_and_actionable() {
        // 「未迁移」必须给出明确、可操作的文案，绝不能静默成功
        assert!(TRANSLATION_UNMIGRATED_MSG.contains("尚未迁移"));
        assert!(TRANSLATION_UNMIGRATED_MSG.contains("langVolcengineAccessKey"));
        assert!(BATCH_SEO_UNMIGRATED_MSG.contains("尚未迁移"));
        assert!(!BATCH_SEO_UNMIGRATED_MSG.is_empty());
        // 错误码必须是 Java 的默认业务码
        let err = Error::biz(SERVICE_DATA_ERROR, TRANSLATION_UNMIGRATED_MSG);
        assert_eq!(err.business_code(), 1001);
    }

    #[test]
    fn chinese_text_validation_rejects_missing_and_blank() {
        assert_eq!(validate_chinese_text(Some("手机")).unwrap(), "手机");
        // 前后空白被裁剪（Java 不裁剪，但空白文本无法翻译，差异见模块头）
        assert_eq!(validate_chinese_text(Some("  手机 ")).unwrap(), "手机");
        for empty in [None, Some(""), Some(" "), Some("\t\n")] {
            let err = validate_chinese_text(empty).unwrap_err();
            assert_eq!(err.business_code(), SERVICE_DATA_ERROR);
            assert!(err.to_string().contains("chineseText"));
        }
    }
}