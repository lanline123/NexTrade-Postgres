//! 商品询价域服务：列表 / 详情 / 商家回复 / 删除 / 批量操作。
//! 对应原 Java `PriceInquiryServiceImpl`
//! （`com.nextrade.service.product.impl`，仅管理端暴露的 5 个方法）。
//!
//! 迁移要点（逐条对照 Java 源）：
//! - `list`：先按 `keyword` 在 `product` 表匹配 商品名/编号/TSN 得到商品 ID，
//!   再按 `product_id IN (...)` + 手机号模糊/状态/用户/用户侧删除/店铺过滤分页；
//!   关键词匹配不到商品时**直接返回空页**（Java 原样）。
//!   列表项补充商品摘要（取 `product.pic_url` 作为 `picThumb`）。
//! - `detail`：查不到抛「询价信息不存在」；补充商品摘要 + 回复记录
//!   （回复记录为空但 `remark` 非空白时生成一条「商家」兜底记录，见
//!   [`nextrade_core::models::catalog::price_inquiry::build_reply_records`]）。
//! - `reply`（`replyByAdmin`，Java `@Transactional`）：
//!   1. 校验 `id` / `remark`（`@NotNull` / `@NotBlank` / `@Size(max=100)`）
//!   2. 查询价，不存在抛「询价信息不存在」
//!   3. `UPDATE status = 1, remark = ?`
//!   4. 插入回复记录：`reply_type = 1`（商家）、`reply_name` 取当前管理员名
//!      （空白回退「商家」）、`admin_id` 取当前管理员 ID、`user_id = 0`、
//!      `create_time = now`
//!   5. 两步写入在同一事务内（等价 Java `@Transactional`）
//! - `del`：`removeById`，Java 控制器**不检查返回值**（删不存在的 ID 也成功）。
//! - `batch`：对应 `BaseServiceImpl#batch` —— 仅 `type == "del"` 时
//!   `removeByIds`；其它类型返回 `false`（Java 同样不报错）。
//!
//! # 未迁移 / 不确定点
//!
//! - Java `replyByAdmin` 第 5 步 `sendInquiryReplyMessage` 会写站内信
//!   （`user_message` 表，属 member 域，尚无对应仓储）——**本实现跳过**，
//!   见 `uncertain_points`。
//! - Java 用 `SecurityUtils.getCurrentUsername()` / `getCurrentAdminId()`
//!   取登录态；Rust 侧由路由层从 `AuthUser` 显式传入 [`AdminOperator`]。
//! - `@Sensitive(PHONE/EMAIL)` 脱敏仅在 `nextrade.is-demo=1` 时生效
//!   （`SensitiveJsonSerializer`），Rust 侧未迁移 demo 开关，故不脱敏。
//! - `page / size` 之外的 `sortItems` 多字段排序 Java 本域未使用，同样忽略。
//! - `Page<PriceInquiryVO>` 的分页外壳用项目统一的 `PageResult`
//!   （`list/total/totalPage/pageSize/page`），与 brand / product 等既有域一致；
//!   Java 的 MyBatis `Page` 是 `records/total/size/current/pages`。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::entities::PriceInquiryReplyRecord;
use nextrade_core::models::catalog::price_inquiry::{
    build_reply_records, inquiry_to_vo, BatchDTO, PriceInquiryProductVO, PriceInquiryQuery,
    PriceInquiryReplyDTO, PriceInquiryVO, ID_REQUIRED, INQUIRY_NOT_FOUND, OPERATION_TYPE_REQUIRED,
    REPLY_CONTENT_MAX_LEN, REPLY_CONTENT_REQUIRED, REPLY_CONTENT_TOO_LONG, UNSELECTED_ITEM,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;
use nextrade_infrastructure::db::repo::catalog::price_inquiry::ProductBrief;

/// 回复类型：商家回复（Java `saveReplyRecord(..., 1, ...)`）。
pub const REPLY_TYPE_ADMIN: i32 = 1;

/// 回复人名称兜底（Java `StrUtil.blankToDefault(currentUsername, "商家")`）。
pub const REPLY_NAME_FALLBACK: &str = "商家";

/// 管理端操作人（替代 Java `SecurityUtils`）。
#[derive(Debug, Clone, Default)]
pub struct AdminOperator {
    /// 当前管理员 ID（写入 `price_inquiry_reply_record.admin_id`）
    pub admin_id: i32,
    /// 当前管理员用户名（写入 `reply_name`，空白回退「商家」）
    pub username: String,
}

impl AdminOperator {
    #[must_use]
    pub fn new(admin_id: i32, username: impl Into<String>) -> Self {
        Self {
            admin_id,
            username: username.into(),
        }
    }

    /// 回复记录里的回复人名称（Java `StrUtil.blankToDefault(username, "商家")`）。
    ///
    /// 注意 Java 只在**空白**时替换，非空白值原样写入（不 trim）。
    #[must_use]
    pub fn reply_name(&self) -> String {
        if self.username.trim().is_empty() {
            REPLY_NAME_FALLBACK.to_string()
        } else {
            self.username.clone()
        }
    }
}

/// 商品询价域服务。
pub struct PriceInquiryService {
    repos: Arc<Repositories>,
}

impl PriceInquiryService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 查询 —————————————————————

    /// 询价分页列表（对应 Java `PriceInquiryServiceImpl#list`）。
    ///
    /// `q.shop_id` 已由路由层按 Java `HeaderUtils.getShopId()` 语义覆盖
    /// （平台账号 = 0，店铺/门店 = `x-shop-id`，其它 = 不过滤）。
    pub async fn list(&self, q: &PriceInquiryQuery) -> Result<PageResult<PriceInquiryVO>> {
        let page = q.page_query();

        // 1. 关键词 → 商品 ID（Java 先查 product 再 `IN`；无匹配直接空页）
        let product_ids = match q.effective_keyword() {
            Some(keyword) => Some(
                self.repos.catalog.price_inquiry
                    .find_product_ids_by_keyword(keyword)
                    .await?,
            ),
            None => None,
        };
        if let Some(ids) = &product_ids {
            if ids.is_empty() {
                return Ok(PageResult::empty(&page));
            }
        }

        // 2. 分页查询价
        let (total, rows) = self
            .repos.catalog.price_inquiry
            .paginate(q, product_ids.as_deref(), &page)
            .await?;

        // 3. 批量补商品摘要（Java `productMap`）
        let product_ids: Vec<i32> = rows.iter().map(|r| r.product_id).collect();
        let mut products: HashMap<i32, ProductBrief> = HashMap::new();
        if !product_ids.is_empty() {
            for p in self
                .repos.catalog.price_inquiry
                .find_products_by_ids(&product_ids)
                .await?
            {
                products.insert(p.product_id, p);
            }
        }

        let list: Vec<PriceInquiryVO> = rows
            .iter()
            .map(|r| {
                let mut vo = inquiry_to_vo(r);
                if let Some(p) = products.get(&r.product_id) {
                    vo.product = Some(to_product_vo(p));
                }
                vo
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// 询价详情（对应 Java `PriceInquiryServiceImpl#detail(Integer)`）。
    ///
    /// Java 查不到抛 `GlobalException("询价信息不存在")`（业务 500）。
    pub async fn detail(&self, id: i32) -> Result<PriceInquiryVO> {
        let entity = self
            .repos.catalog.price_inquiry
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, INQUIRY_NOT_FOUND))?;

        let product = self
            .repos.catalog.price_inquiry
            .find_product_by_id(entity.product_id)
            .await?;
        let records = self
            .repos.catalog.price_inquiry
            .find_reply_records(entity.id)
            .await?;

        let mut vo = inquiry_to_vo(&entity);
        vo.product = product.as_ref().map(to_product_vo);
        vo.reply_records = Some(build_reply_records(&records, &entity.remark));
        Ok(vo)
    }

    // ————————————————————— 写操作 —————————————————————

    /// 商家回复（对应 Java `PriceInquiryServiceImpl#replyByAdmin`，`@Transactional`）。
    ///
    /// 事务内完成：查存在性 → 更新 `status/remark` → 插入回复记录。
    pub async fn reply(&self, dto: &PriceInquiryReplyDTO, operator: &AdminOperator) -> Result<()> {
        let (id, remark) = validate_reply(dto)?;

        let mut tx = self.repos.catalog.price_inquiry.begin().await?;
        let inquiry = self
            .repos.catalog.price_inquiry
            .find_by_id_in_tx(&mut tx, id)
            .await?
            .ok_or_else(|| Error::biz(500, INQUIRY_NOT_FOUND))?;

        // Java `lambdaUpdate().set(status,1).set(remark, dto.getRemark())`
        self.repos.catalog.price_inquiry
            .update_reply_in_tx(&mut tx, id, remark)
            .await?;

        // Java `saveReplyRecord(inquiryId, 1, blankToDefault(username,"商家"),
        //                        dto.getRemark(), currentAdminId, 0)`
        let record = PriceInquiryReplyRecord {
            id: 0, // 自增
            inquiry_id: inquiry.id,
            reply_type: REPLY_TYPE_ADMIN,
            reply_name: operator.reply_name(),
            content: remark.to_string(),
            admin_id: operator.admin_id,
            user_id: 0,
            create_time: now_seconds() as i32,
        };
        self.repos.catalog.price_inquiry
            .insert_reply_record_in_tx(&mut tx, &record)
            .await?;

        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Java 还会 `sendInquiryReplyMessage`（站内信，member 域未迁移，跳过）
        tracing::info!(
            "商家回复询价成功: id={id}, admin_id={}",
            operator.admin_id
        );
        Ok(())
    }

    /// 删除询价（对应 Java `PriceInquiryController#del` → `removeById`）。
    ///
    /// Java 控制器**忽略返回值**，因此不存在的 ID 也返回成功。
    pub async fn delete(&self, id: i32) -> Result<()> {
        let deleted = self
            .repos.catalog.price_inquiry
            .delete(id)
            .await?;
        tracing::info!("删除询价: id={id}, deleted={deleted}");
        Ok(())
    }

    /// 批量操作（对应 Java `BaseServiceImpl#batch`）。
    ///
    /// - `ids` 为空 ⇒ 「未选择项目」（Java `BatchDTO.ids @NotEmpty`）
    /// - `type` 缺失 ⇒ 「操作类型不能为空」（Java `BatchDTO.type @NotNull`）
    /// - `type == "del"` ⇒ 批量删除；其它类型 ⇒ `false`（Java 静默返回 false）
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        let ids = dto.ids.as_deref().unwrap_or_default();
        if ids.is_empty() {
            return Err(Error::Validation(UNSELECTED_ITEM.into()));
        }
        let op_type = dto
            .r#type
            .as_deref()
            .ok_or_else(|| Error::Validation(OPERATION_TYPE_REQUIRED.into()))?;
        if op_type == "del" {
            let n = self
                .repos.catalog.price_inquiry
                .delete_batch(ids)
                .await?;
            tracing::info!("批量删除询价: {n} 条");
            return Ok(n > 0);
        }
        Ok(false)
    }
}

/// 商品摘要仓储行 → VO（Java `productVO.setPicThumb(product.getPicUrl())`）。
fn to_product_vo(p: &ProductBrief) -> PriceInquiryProductVO {
    PriceInquiryProductVO {
        product_id: p.product_id,
        product_name: p.product_name.clone(),
        product_sn: p.product_sn.clone(),
        pic_thumb: p.pic_url.clone(),
    }
}

/// 校验 `PriceInquiryReplyDTO` 并返回 `(id, remark)`（Java 注解语义）。
///
/// - `id`：`@NotNull(message = "id不能为空")`
/// - `remark`：`@NotBlank(message = "回复内容不能为空")` +
///   `@Size(min = 1, max = 100, message = "回复内容长度在1到255之间")`
///
/// 注意：长度按**字符数**、对**原始值**（不 trim）计算，与 Bean Validation 一致；
/// 落库也使用未 trim 的原始值（Java 直接把 `dto.getRemark()` 写库）。
fn validate_reply(dto: &PriceInquiryReplyDTO) -> Result<(i32, &str)> {
    let id = dto.id.ok_or_else(|| Error::Validation(ID_REQUIRED.into()))?;
    let remark = dto
        .remark
        .as_deref()
        .ok_or_else(|| Error::Validation(REPLY_CONTENT_REQUIRED.into()))?;
    if remark.trim().is_empty() {
        return Err(Error::Validation(REPLY_CONTENT_REQUIRED.into()));
    }
    if remark.chars().count() > REPLY_CONTENT_MAX_LEN {
        return Err(Error::Validation(REPLY_CONTENT_TOO_LONG.into()));
    }
    Ok((id, remark))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ———————————————— 回复人名称 ————————————————

    #[test]
    fn reply_name_falls_back_to_merchant_when_blank() {
        assert_eq!(AdminOperator::new(1, "").reply_name(), "商家");
        assert_eq!(AdminOperator::new(1, "   ").reply_name(), "商家");
        // 非空白原样保留（Java `blankToDefault` 不 trim）
        assert_eq!(AdminOperator::new(1, "admin").reply_name(), "admin");
        assert_eq!(AdminOperator::new(1, " 张三 ").reply_name(), " 张三 ");
        assert_eq!(AdminOperator::default().reply_name(), "商家");
    }

    // ———————————————— 回复校验（对应 Java 注解） ————————————————

    fn dto(id: Option<i32>, remark: Option<&str>) -> PriceInquiryReplyDTO {
        PriceInquiryReplyDTO {
            id,
            remark: remark.map(ToString::to_string),
        }
    }

    #[test]
    fn reply_requires_id() {
        let err = validate_reply(&dto(None, Some("内容"))).unwrap_err();
        assert!(err.to_string().contains("id不能为空"), "实际: {err}");
    }

    #[test]
    fn reply_requires_non_blank_content() {
        for raw in [None, Some(""), Some("   ")] {
            let err = validate_reply(&dto(Some(1), raw)).unwrap_err();
            assert!(
                err.to_string().contains("回复内容不能为空"),
                "remark={raw:?} 应触发 @NotBlank, 实际: {err}"
            );
        }
    }

    #[test]
    fn reply_content_max_len_is_100_chars_not_bytes() {
        // 100 个字符（含中文）合法；101 个字符触发 @Size
        let ok = "回".repeat(100);
        let input = dto(Some(3), Some(&ok));
        let (id, remark) = validate_reply(&input).unwrap();
        assert_eq!(id, 3);
        assert_eq!(remark, ok);

        let too_long = "回".repeat(101);
        let err = validate_reply(&dto(Some(3), Some(&too_long))).unwrap_err();
        // 文案原文写的是 255（Java 注解如此），实现上限是 100
        assert!(err.to_string().contains("回复内容长度在1到255之间"), "实际: {err}");
    }

    #[test]
    fn reply_passes_raw_content_through_unchanged() {
        let input = dto(Some(7), Some(" 已电话联系 "));
        let (_, remark) = validate_reply(&input).unwrap();
        assert_eq!(remark, " 已电话联系 ", "Java 直接写库，不 trim");
    }
}
