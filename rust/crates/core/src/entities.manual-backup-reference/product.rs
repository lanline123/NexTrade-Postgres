//! 商品实体 `product` / SKU 实体 `product_sku`。
//! 对应 `com.nextrade.bean.model.product.Product` / `ProductSku`。
//!
//! 原 Java 中 `@TableField(exist = false)` 的临时字段（store_* 等）
//! 不属于表结构，放入 `crate::models::catalog::product` 的 VO 中。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Product {
    /// 商品ID
    pub product_id: Option<i32>,
    /// 商品名称
    pub product_name: Option<String>,
    /// SEO标识
    pub seo_identification: Option<String>,
    /// 商品编号
    pub product_sn: Option<String>,
    /// 商品临时编号
    pub product_tsn: Option<String>,
    /// 商品库存
    pub product_stock: Option<i32>,
    /// 商品价格
    pub product_price: Option<Decimal>,
    /// 市场价
    pub market_price: Option<Decimal>,
    /// 运费模板ID
    pub shipping_tpl_id: Option<i64>,
    /// 商品状态
    pub product_status: Option<i32>,
    /// 商品类型
    pub product_type: Option<i32>,
    /// 交付模式：1-单次交付（默认），2-多次交付（周期购）
    pub delivery_mode: Option<i32>,
    /// 销售模式：0-现货销售，1-全款预售，2-定金预售
    pub sale_mode: Option<i32>,
    /// 分类ID
    pub category_id: Option<i32>,
    /// 品牌ID
    pub brand_id: Option<i32>,
    /// 店铺ID
    pub shop_id: Option<i32>,
    /// 关键词
    pub keywords: Option<String>,
    /// 店铺分类ID
    pub shop_category_id: Option<i32>,
    /// 审核状态
    pub check_status: Option<i32>,
    /// 审核原因
    pub check_reason: Option<String>,
    /// 点击次数
    pub click_count: Option<i32>,
    /// 商品重量
    pub product_weight: Option<Decimal>,
    /// 是否促销
    pub is_promote: Option<i32>,
    /// 是否促销活动
    pub is_promote_activity: Option<i32>,
    /// 促销价格
    pub promote_price: Option<Decimal>,
    /// 促销开始日期
    pub promote_start_date: Option<i64>,
    /// 促销结束日期
    pub promote_end_date: Option<i64>,
    /// 秒杀最大数量
    pub seckill_max_num: Option<i32>,
    /// 商品简介
    pub product_brief: Option<String>,
    /// 商品描述
    pub product_desc: Option<String>,
    /// 图片URL
    pub pic_url: Option<String>,
    /// 缩略图URL
    pub pic_thumb: Option<String>,
    /// 原图URL
    pub pic_original: Option<String>,
    /// 大图
    pub pic_large: Option<String>,
    /// 评论标签
    pub comment_tag: Option<String>,
    /// 是否包邮
    pub free_shipping: Option<i32>,
    /// 积分
    pub integral: Option<i32>,
    /// 添加时间
    pub add_time: Option<i64>,
    /// 排序
    pub sort_order: Option<i32>,
    /// 店铺排序
    pub store_sort_order: Option<i32>,
    /// 是否删除
    pub is_delete: Option<i32>,
    /// 是否精品
    pub is_best: Option<i32>,
    /// 是否新品
    pub is_new: Option<i32>,
    /// 是否热销
    pub is_hot: Option<i32>,
    /// 最后更新时间
    pub last_update: Option<i64>,
    /// 备注
    pub remark: Option<String>,
    /// 赠送积分
    pub give_integral: Option<i32>,
    /// 等级积分
    pub rank_integral: Option<i32>,
    /// 供应商ID
    pub suppliers_id: Option<i32>,
    /// 虚拟销量
    pub virtual_sales: Option<i32>,
    /// 限购数量
    pub limit_number: Option<i32>,
    /// 商品保养
    pub product_care: Option<String>,
    /// 相关商品
    pub product_related: Option<String>,
    /// 商品服务ID
    pub product_service_ids: Option<String>,
    /// 是否支持退货
    pub is_support_return: Option<i32>,
    /// 是否支持货到付款
    pub is_support_cod: Option<i32>,
    /// 商品视频
    pub product_video: Option<String>,
    /// 预付价格
    pub prepay_price: Option<Decimal>,
    /// 卡组ID
    pub card_group_id: Option<i32>,
    /// 虚拟样品
    pub virtual_sample: Option<String>,
    /// 付费内容
    pub paid_content: Option<String>,
    /// 是否无需配送：0-否，1-是
    pub no_shipping: Option<i32>,
    /// 运费类型
    pub fixed_shipping_type: Option<i32>,
    /// 固定运费金额
    pub fixed_shipping_fee: Option<Decimal>,
    /// 是否到店自提：0-否，1-是
    pub is_shop_pickup: Option<i32>,
    /// 到店自提模板ID
    pub shop_pickup_tpl_id: Option<i64>,
    /// 供应商产品ID
    pub vendor_product_id: Option<i32>,
    /// 供应商ID
    pub vendor_id: Option<i32>,
    /// 是否物流配送：0-否，1-是
    pub is_logistics: Option<i32>,
    /// 是否门店配送：0-否，1-是
    pub is_shop_delivery: Option<i32>,
    /// 预约模板Id
    pub reservation_tpl_id: Option<i32>,
    /// 礼品卡组id
    pub gift_card_groups_id: Option<i32>,
    /// 商品单位
    pub product_unit: Option<String>,
    /// 成本价
    pub cost_price: Option<Decimal>,
    /// 是否需要重新导入供应商商品
    pub is_need_re_import_vendor_product: Option<i32>,
    /// 属性模板ID
    pub attributes_tpl_id: Option<i32>,
    /// 分享信息（JSON）
    pub share_info: Option<String>,
    /// 配送员服务费类型：0=固定金额 1=按比例
    pub delivery_service_fee_type: Option<i32>,
    /// 配送员服务费值
    pub delivery_service_fee_value: Option<Decimal>,
    /// 服务员服务费类型：0=固定金额 1=按比例
    pub server_service_fee_type: Option<i32>,
    /// 服务员服务费值
    pub server_service_fee_value: Option<Decimal>,
}

impl Product {
    pub const TABLE: &'static str = "product";

    /// 是否上架可售（状态=1 且未删除）。
    pub fn is_on_sale(&self) -> bool {
        self.product_status.unwrap_or(0) == 1 && self.is_delete.unwrap_or(1) == 0
    }
}

/// SKU 实体 `product_sku`。
/// 对应 `com.nextrade.bean.model.product.ProductSku`。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductSku {
    pub sku_id: Option<i32>,
    pub product_id: Option<i32>,
    /// 规格值，如 `红色,L`
    pub sku_value: Option<String>,
    /// 规格数据 JSON `{"颜色":"红","尺寸":"L"}`
    pub sku_data: Option<String>,
    pub sku_sn: Option<String>,
    pub sku_stock: Option<i32>,
    pub sku_tsn: Option<String>,
    pub sku_price: Option<Decimal>,
    /// 是否启用：1 是 0 否
    pub is_enabled: Option<i32>,
    pub market_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    pub delivery_service_fee_type: Option<i32>,
    pub delivery_service_fee_value: Option<Decimal>,
    pub server_service_fee_type: Option<i32>,
    pub server_service_fee_value: Option<Decimal>,
    pub vendor_product_sku_id: Option<i32>,
}

impl ProductSku {
    pub const TABLE: &'static str = "product_sku";

    /// 可售库存（启用状态下）。
    pub fn available_stock(&self) -> i64 {
        if self.is_enabled.unwrap_or(1) == 1 {
            i64::from(self.sku_stock.unwrap_or(0)).max(0)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn on_sale_rule() {
        let p = Product {
            product_status: Some(1),
            is_delete: Some(0),
            ..Default::default()
        };
        assert!(p.is_on_sale());
    }

    #[test]
    fn sku_stock_non_negative() {
        let s = ProductSku {
            sku_stock: Some(-5),
            is_enabled: Some(1),
            ..Default::default()
        };
        assert_eq!(s.available_stock(), 0);
        assert_eq!(
            Decimal::from_str("19.90").unwrap(),
            Decimal::from_str("19.90").unwrap()
        );
    }
}