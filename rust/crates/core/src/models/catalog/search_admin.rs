//! 商品搜索（`catalog/search_admin`）DTO / VO。
//!
//! 对应原 Java：
//! - `com.nextrade.bean.dto.product.ProductSearchClientDTO`（前台搜索参数）
//! - `com.nextrade.bean.vo.search.SearchFilterVO`（筛选）
//! - `com.nextrade.bean.vo.product.ProductSearchFilterListVO`（筛选项）
//! - `com.nextrade.bean.vo.product.ProductSearchFilterSelectedVO`（已选筛选项）
//!
//! 本文件包含 `catalog/search_admin` 的搜索参数/筛选结构；关键词联想
//! （`catalog/product_search`）的结构在 `models/catalog/product_search.rs`。
//!
//! **未迁移/偏差**：
//! - Java 前台搜索在 `SELECT_SEARCH_TYPE=es` 时走 Elasticsearch，本工作区无 ES
//!   依赖，恒走 MySQL 分支（与 Java 的 ES 失败回退分支等价）。
//! - `shopCategory` 筛选项依赖店铺域 `shop_product_category`，本切片不跨域
//!   装配，字段保留为 `None`（见服务层不确定点）。
//! - 分类/品牌选项使用精简 VO（Java 直接序列化整行实体）。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 商品搜索参数（对应 Java `ProductSearchClientDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductSearchClientQuery {
    /// 分类 ID
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub cat: Option<i32>,
    /// 品牌，可多个，逗号分隔
    pub brand: Option<String>,
    /// 价格区间下限
    pub min: Option<Decimal>,
    /// 价格区间上限
    pub max: Option<Decimal>,
    /// 关键字
    pub keyword: Option<String>,
    /// 排序：`sale` / `price` / `time` / `default`
    pub sort: Option<String>,
    /// 排序方向（`price` 时生效）
    pub order: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
    /// 用户位置经度（与纬度同时传入时走 O2O 最近门店语义）
    pub longitude: Option<Decimal>,
    /// 用户位置纬度
    pub latitude: Option<Decimal>,
    /// 活动类型：`new` / `hot` / `best`
    pub intro: Option<String>,
    /// 优惠券 ID（ES 分支不支持，MySQL 支持）
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub coupon_id: Option<i32>,
    /// 店铺分类 ID
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub shop_category_id: Option<i32>,
    /// 店铺 ID
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub shop_id: Option<i32>,
    /// 商品状态（缺省 1 上架）
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub product_status: Option<i32>,
    /// 属性查询
    pub attrs: Option<Vec<SearchAttrObj>>,
    /// 收银台商品来源过滤（Java 只做参数透传，本实现不参与过滤）
    pub pos_product_source: Option<Vec<i32>>,
    /// 是否排除周期购商品
    pub exclude_cycle_product: Option<bool>,
    /// 是否仅排除按计划配送的周期购
    pub exclude_plan_delivery_cycle_only: Option<bool>,
    /// 是否排除礼品卡商品
    pub exclude_gift_card_product: Option<bool>,
    /// 是否过滤预售商品
    #[serde(default, deserialize_with = "de_opt_i32_lenient")]
    pub is_filter_presale_product: Option<i32>,
}

impl ProductSearchClientQuery {
    /// 分页参数（Java：`page` 缺省 1；`size` 缺省/超 50 时取 25）。
    #[must_use]
    pub fn page_query(&self) -> crate::types::PageQuery {
        crate::types::PageQuery {
            page: self.page.unwrap_or(1).max(1),
            page_size: match self.size {
                Some(s) if s <= 50 && s > 0 => s,
                _ => 25,
            },
        }
    }

    /// 品牌 ID 列表（Java：`brand` 逗号串，`Integer::parseInt`）。
    #[must_use]
    pub fn parsed_brand_ids(&self) -> Vec<i32> {
        parse_id_csv(self.brand.as_deref())
    }

    /// 排序映射（Java `ProductSearchClientDTO#sort` → `ProductListDTO.sortField`）：
    /// - `sale` → `virtual_sales desc`
    /// - `price` → `product_price {order}`
    /// - `time` → `product_id desc`
    /// - 其它/`default` → 不排序（Java 只有非空且非 `default` 才进入 switch）
    #[must_use]
    pub fn sort_mapping(&self) -> Option<(String, String)> {
        let sort = self.sort.as_deref()?.trim();
        if sort.is_empty() || sort.eq_ignore_ascii_case("default") {
            return None;
        }
        let order = self
            .order
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("desc")
            .to_string();
        let field = match sort {
            "sale" => "virtual_sales",
            "price" => "product_price",
            "time" => "product_id",
            _ => "sort_order",
        };
        let dir = if sort == "price" {
            order
        } else {
            "desc".to_string()
        };
        Some((field.to_string(), dir))
    }
}

/// 属性查询项（对应 Java `ProductSearchClientDTO.AttrObj`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchAttrObj {
    pub attr_name: Option<String>,
    #[serde(default, deserialize_with = "de_opt_i32_or_str_list")]
    pub attr_value: Option<Vec<String>>,
}

/// 分类筛选项（Java 直接返回 `Category` 实体；这里取前端实际用到的字段）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchCategoryVO {
    pub category_id: i32,
    pub category_name: String,
    pub parent_id: i32,
    pub sort_order: i32,
}

/// 品牌筛选项（Java 直接返回 `Brand` 实体；这里取前端实际用到的字段）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchBrandVO {
    pub brand_id: i32,
    pub brand_name: String,
}

/// 筛选项（对应 Java `ProductSearchFilterListVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductSearchFilterListVO {
    pub category: Option<Vec<SearchCategoryVO>>,
    pub brand: Option<Vec<SearchBrandVO>>,
    /// 店铺分类（依赖店铺域，未迁移；恒 `None`）
    pub shop_category: Option<Vec<serde_json::Value>>,
    pub max_price: Option<Decimal>,
}

/// 已选筛选项（对应 Java `ProductSearchFilterSelectedVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductSearchFilterSelectedVO {
    pub intro: Option<String>,
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub brand: Option<String>,
    pub shop_category: Option<String>,
    pub attrs: Option<Vec<SearchAttrObj>>,
}

/// 搜索筛选返回（对应 Java `SearchFilterVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchFilterVO {
    pub filter: ProductSearchFilterListVO,
    pub filter_selected: ProductSearchFilterSelectedVO,
}

/// 属性值计数（Java `getAttributeSuggestions` 的 `{value, count}`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AttributeValueCountVO {
    pub value: String,
    pub count: i64,
}

/// 属性搜索建议（Java `{name, values:[{value,count}]}`，最多 6 个属性 × 20 个值）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AttributeSuggestionVO {
    pub name: String,
    pub values: Vec<AttributeValueCountVO>,
}

/// 解析逗号分隔 ID（空白项忽略）。
#[must_use]
pub fn parse_id_csv(raw: Option<&str>) -> Vec<i32> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.split(',')
                .filter_map(|v| v.trim().parse::<i32>().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// 兼容原版前端把数字 ID 放在 URL query 后按字符串提交（`"17"`）。
fn de_opt_i32_lenient<'de, D>(de: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(de)?;
    Ok(match value {
        Some(serde_json::Value::Number(n)) => n.as_i64().and_then(|v| i32::try_from(v).ok()),
        Some(serde_json::Value::String(s)) => s.trim().parse::<i32>().ok(),
        _ => None,
    })
}

/// 兼容前端把 `attrValue` 传成字符串数组 / 单字符串。
fn de_opt_i32_or_str_list<'de, D>(de: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(de)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(vec![s]),
        Some(serde_json::Value::Array(items)) => Some(
            items
                .into_iter()
                .filter_map(|v| match v {
                    serde_json::Value::String(s) => Some(s),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect(),
        ),
        Some(_) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_search_accepts_original_string_id() {
        let q: ProductSearchClientQuery =
            serde_json::from_str(r#"{"cat":"17","size":30,"page":1,"attrs":[]}"#)
                .expect("原版分类页把 cat 作为字符串提交");
        assert_eq!(q.cat, Some(17));
        assert_eq!(q.page_query().page_size, 30);
    }

    #[test]
    fn sort_mapping_matches_java_build_product_list_dto() {
        let q = ProductSearchClientQuery {
            sort: Some("sale".into()),
            ..Default::default()
        };
        assert_eq!(
            q.sort_mapping(),
            Some(("virtual_sales".to_string(), "desc".to_string()))
        );
        let q = ProductSearchClientQuery {
            sort: Some("price".into()),
            order: Some("asc".into()),
            ..Default::default()
        };
        assert_eq!(
            q.sort_mapping(),
            Some(("product_price".to_string(), "asc".to_string()))
        );
        let q = ProductSearchClientQuery {
            sort: Some("time".into()),
            ..Default::default()
        };
        assert_eq!(
            q.sort_mapping(),
            Some(("product_id".to_string(), "desc".to_string()))
        );
        // default / 空 / 未传 → None（Java 不进 switch）
        assert_eq!(ProductSearchClientQuery::default().sort_mapping(), None);
        let q = ProductSearchClientQuery {
            sort: Some("default".into()),
            ..Default::default()
        };
        assert_eq!(q.sort_mapping(), None);
    }

    #[test]
    fn page_query_uses_java_defaults() {
        let q = ProductSearchClientQuery::default();
        let page = q.page_query();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 25, "Java: size 缺省/超 50 → 25");
        let q = ProductSearchClientQuery {
            page: Some(0),
            size: Some(10),
            ..Default::default()
        };
        let page = q.page_query();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 10);
        let q = ProductSearchClientQuery {
            size: Some(500),
            ..Default::default()
        };
        assert_eq!(q.page_query().page_size, 25);
    }

    #[test]
    fn brand_csv_and_attr_value_parsing() {
        assert_eq!(parse_id_csv(Some("1, 2,x,3")), vec![1, 2, 3]);
        assert!(parse_id_csv(None).is_empty());
        let attr: SearchAttrObj =
            serde_json::from_str(r#"{"attrName":"颜色","attrValue":"红"}"#).unwrap();
        assert_eq!(attr.attr_name.as_deref(), Some("颜色"));
        assert_eq!(attr.attr_value, Some(vec!["红".to_string()]));
        let attr: SearchAttrObj =
            serde_json::from_str(r#"{"attrName":"颜色","attrValue":["红","蓝"]}"#).unwrap();
        assert_eq!(
            attr.attr_value,
            Some(vec!["红".to_string(), "蓝".to_string()])
        );
    }
}
