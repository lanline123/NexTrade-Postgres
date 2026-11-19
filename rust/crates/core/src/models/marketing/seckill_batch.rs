//! 秒杀批处理 / 装修预览域（marketing::seckill_batch）DTO / VO / 纯逻辑。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.promotion.SeckillController` 的
//! 3 个端点（`@RequestMapping("/adminapi/promotion/seckill")`）：
//! - `POST /batch` → `SeckillServiceImpl#batch(BatchDTO)`：`type=del` 级联删除
//!   `seckill` + `seckill_item` + `promotion(type=1)`
//! - `GET /listForDecorate` → `SeckillServiceImpl#listForDecorate`：
//!   装修用「进行中」秒杀商品列表（不分页，返回 `List<SeckillPromotionVO>`）
//! - `GET /listForHomePreview` → 复用 C 端首页 `getSeckillProductList(size,
//!   page, unStarted=0)`，返回 `Page<SeckillListVO>`
//!
//! 表复用既有 `seckill` / `seckill_item` / `promotion`（schema 已生成）。
//!
//! # 与 Java 的差异（uncertain_points）
//! 1. Java `listForDecorate` 先取「进行中」`seckill` 活动分页（默认 15 个
//!    活动）、再按活动展开商品并聚合完整信息（`SeckillPromotionVO` 30+
//!    字段）；Rust 以 `seckill_item` 自带起止时间作等价进行中窗口，不做
//!    活动级分页，改以 item 上限 500（`seckill_price` 升序）兜底。其余口径
//!    已对齐 Java：无库存过滤、不查商品上下架/删除状态、SKU 仅过滤已禁用
//!    项，并按（活动, 商品）聚合成一商品一行（销量/库存取同组全部 item
//!    合计——Java 此处有把库存合计误写进 `seckillSales` 的 bug，未复刻）。
//!    VO 仍为精简版 `seckillId/productId/productName/skuId/seckillPrice/
//!    seckillStock/seckillSales/seckillStartTime/seckillEndTime/
//!    seckillLimitNum` + 预览扩展字段（picUrl/picThumb/productPrice/
//!    orgProductPrice/marketPrice/seckkillData），商品名从
//!    `product.product_name` 回填。
//! 2. Java `listForHomePreview` 调 C 端 `getSeckillProductList`（含上架过滤、
//!    排序、`unStarted=0`）；Rust 用单独的首页预览查询（进行中窗口 +
//!    `seckill_stock > 0` + `rec_id DESC`，与装修口径分开），分页默认
//!    `page=1,size=15`（与 Java 控制器默认值一致）。
//! 3. Java `batch` 仅在 `isMove=true` 时执行后续两步删除；Rust 在
//!    活动不存在（`rows_affected=0`）时跳过级联（等价保护）。
//!
//! # 金额：`seckill_price` 为 `Decimal`（JSON 字符串）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// `promotion.type`：秒杀（Java `PromotionType.TYPE_SECKILL` = 1）。
pub const PROMOTION_TYPE_SECKILL: i32 = 1;

/// 注解文案（Java `BatchDTO` `@NotEmpty` / `@NotNull`，逐字）。
pub const MSG_UNSELECTED_ITEM: &str = "未选择项目";
pub const MSG_OPERATION_TYPE_REQUIRED: &str = "操作类型不能为空";

/// 批量操作入参（Java `BatchDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchDTO {
    pub ids: Option<Vec<i32>>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub value: Option<i32>,
}

/// 校验批量入参（Java 控制器 `@Valid` 注解文案）。
///
/// # Errors
/// 空 ids ⇒「未选择项目」；缺 type ⇒「操作类型不能为空」。
pub fn validate_batch(dto: &BatchDTO) -> Result<(Vec<i32>, String), String> {
    let ids = dto.ids.clone().unwrap_or_default();
    if ids.is_empty() {
        return Err(MSG_UNSELECTED_ITEM.to_string());
    }
    let op_type = dto
        .r#type
        .clone()
        .ok_or_else(|| MSG_OPERATION_TYPE_REQUIRED.to_string())?;
    Ok((ids, op_type))
}

/// 是否删除操作（Java `"del".equals(batchDTO.getType())`）。
#[must_use]
pub fn is_delete(dto: &BatchDTO) -> bool {
    dto.r#type.as_deref() == Some("del")
}

/// 装修预览 VO（Java `SeckillPromotionVO` 精简版）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeckillPromotionVO {
    pub seckill_id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub sku_id: i32,
    pub seckill_price: Option<Decimal>,
    pub seckill_stock: Option<i32>,
    pub seckill_sales: Option<i32>,
    pub seckill_start_time: Option<i32>,
    pub seckill_end_time: Option<i32>,
    pub seckill_limit_num: Option<i32>,
    // —— 装修预览扩展字段（对齐 Java `SeckillListVO`：编辑器预览需要商品图 /
    //    原价 / 划线价 / 活动起止时间；`listForDecorate` / `listForHomePreview`
    //    由服务层回填，缺数据时保持 None，JSON 里省略）——
    /// 商品主图（product.pic_url，全量 URL）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pic_url: Option<String>,
    /// 商品缩略图（product.pic_thumb，全量 URL）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pic_thumb: Option<String>,
    /// 商品价格（Java 口径：与 marketPrice 交叉赋值，编辑器取它做划线价/折扣基数）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_price: Option<Decimal>,
    /// 商品售价的字符串形式（Java `orgProductPrice` = `Product.productPrice`
    /// 的字符串；秒杀2 预览算折扣/划线原价用，服务层与 productPrice 同源回填）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_product_price: Option<String>,
    /// 市场价（划线价）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_price: Option<Decimal>,
    /// 秒杀活动概要（字段名保留 Java 双 k 拼写 `seckkillData`）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seckkill_data: Option<SeckkillData>,
}

/// 秒杀活动概要（Java `SeckillListVO.SeckillData`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeckkillData {
    pub seckill_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seckill_name: Option<String>,
    /// 格式化开始时间（`YYYY-MM-DD HH:mm:ss`，与 Java 返回一致）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seckill_start_time: Option<String>,
    /// 格式化结束时间（`YYYY-MM-DD HH:mm:ss`，装修预览倒计时取它）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seckill_end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seckill_limit_num: Option<i32>,
    /// 距开始/结束的剩余秒数（进行中为距结束）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remain_seconds: Option<i64>,
    /// 活动是否已开始
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<bool>,
}

/// 首页预览 VO（Java `SeckillListVO` 精简版，字段同装修预览）。
pub type SeckillListVO = SeckillPromotionVO;

/// 首页预览查询（Java 控制器 `size` / `page` 参数，默认 15 / 1）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HomePreviewQuery {
    pub page: Option<u64>,
    #[serde(rename = "size", alias = "pageSize")]
    pub size: Option<u64>,
}

/// 百分号 ILIKE 包装（关键词过滤用）。
#[must_use]
pub fn like_pattern(keyword: &str) -> String {
    format!("%{}%", keyword.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_validation_texts_are_verbatim() {
        let empty = BatchDTO::default();
        assert_eq!(validate_batch(&empty).unwrap_err(), "未选择项目");
        let no_type = BatchDTO {
            ids: Some(vec![1]),
            ..Default::default()
        };
        assert_eq!(validate_batch(&no_type).unwrap_err(), "操作类型不能为空");
        let ok = BatchDTO {
            ids: Some(vec![1, 2]),
            r#type: Some("del".into()),
            ..Default::default()
        };
        assert_eq!(
            validate_batch(&ok).unwrap(),
            (vec![1, 2], "del".to_string())
        );
    }

    #[test]
    fn delete_gate_and_promotion_type() {
        assert!(is_delete(&BatchDTO {
            r#type: Some("del".into()),
            ..Default::default()
        }));
        assert!(!is_delete(&BatchDTO {
            r#type: Some("Del".into()),
            ..Default::default()
        }));
        assert_eq!(PROMOTION_TYPE_SECKILL, 1);
    }

    #[test]
    fn home_preview_defaults_and_like() {
        let q = HomePreviewQuery::default();
        assert_eq!(q.page.unwrap_or(1), 1);
        assert_eq!(q.size.unwrap_or(15), 15);
        assert_eq!(like_pattern(" 秒杀 "), "%秒杀%");
    }

    #[test]
    fn vo_serializes_camel_case_money_as_string() {
        let vo = SeckillPromotionVO {
            seckill_id: 3,
            product_id: 9,
            sku_id: 5,
            seckill_price: Some("9.90".parse().unwrap()),
            ..Default::default()
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["seckillId"], 3);
        assert_eq!(json["seckillPrice"], "9.90");
        assert_eq!(json["productId"], 9);
        // 未回填时 orgProductPrice 省略，不输出 null
        assert!(json.get("orgProductPrice").is_none());
    }

    #[test]
    fn org_product_price_serializes_as_string() {
        let vo = SeckillPromotionVO {
            product_price: Some("248.00".parse().unwrap()),
            org_product_price: Some("248.00".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["orgProductPrice"], "248.00");
        assert_eq!(json["productPrice"], "248.00");
    }
}
