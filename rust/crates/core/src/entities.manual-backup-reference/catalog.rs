//! `category` / `brand` 商品分类与品牌实体。
//! 对应 `com.nextrade.bean.model.product.Category` 与 `Brand`。
//!
//! 字段与 `java/database/pure.sql` 中 `CREATE TABLE category`（第 758 行）与
//! `CREATE TABLE brand`（第 552 行）的列一一对应：
//! - 列名通过 `#[sqlx(rename_all = "snake_case")]` 对齐，零迁移直连原库；
//! - 可空列为 `Option<T>`，非空列取对应 SQL 类型（`NOT NULL` 列为必填值）。

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 商品分类（表 `category`，对应 `com.nextrade.bean.model.product.Category`）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Category {
    /// 自增id号 `category_id`
    pub category_id: i32,
    /// 分类名称 `category_name`（NOT NULL，默认 ''）
    pub category_name: String,
    /// SEO标识 `seo_identification`（可空，仅小写字母/数字/连字符）
    pub seo_identification: Option<String>,
    /// 分类关键字 `keywords`（可能是为了搜索）
    pub keywords: String,
    /// 分类描述 `category_desc`
    pub category_desc: String,
    /// 父级id `parent_id`，取值于本表 `category_id`，0 表示顶级
    pub parent_id: i32,
    /// 页面显示顺序 `sort_order`，数字越大越靠后（默认 50）
    pub sort_order: i32,
    /// 计量单位 `measure_unit`
    pub measure_unit: String,
    /// 是否在前台页面显示 `is_show`：1 显示；0 不显示（默认 1）
    pub is_show: i32,
    /// SEO标题 `seo_title`
    pub seo_title: String,
    /// 分类短名 `short_name`
    pub short_name: String,
    /// 分类图片 `category_pic`
    pub category_pic: String,
    /// 分类图标 `category_ico`
    pub category_ico: String,
    /// 是否热门 `is_hot`：1 是；0 否（默认 0）
    pub is_hot: i32,
    /// 搜索关键字 `search_keywords`
    pub search_keywords: String,
}

impl Category {
    /// 表名。
    pub const TABLE: &'static str = "category";

    /// 顶级分类的 `parent_id` 取值（对应原库默认值 0）。
    pub const ROOT_PARENT: i32 = 0;

    /// 是否根（顶级）分类。
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_id == Self::ROOT_PARENT
    }

    /// 是否在前台显示（`is_show = 1`）。
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.is_show == 1
    }

    /// 是否热门分类（`is_hot = 1`）。
    #[must_use]
    pub fn is_hot_category(&self) -> bool {
        self.is_hot == 1
    }
}

/// 商品品牌（表 `brand`，对应 `com.nextrade.bean.model.product.Brand`）。
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Brand {
    /// 自增id号 `brand_id`
    pub brand_id: i32,
    /// 品牌名称 `brand_name`（NOT NULL，默认 ''）
    pub brand_name: String,
    /// 品牌英文名称 `brand_en_name`（可空，默认 ''）
    pub brand_en_name: Option<String>,
    /// 公司Logo图片 `brand_logo`（可空，默认 ''）
    pub brand_logo: Option<String>,
    /// 品牌描述 `brand_desc`（可空）
    pub brand_desc: Option<String>,
    /// 品牌网址 `site_url`（NOT NULL，默认 ''）
    pub site_url: String,
    /// 店铺id `shop_id`（NOT NULL，默认 0，0 表示平台）
    pub shop_id: i32,
    /// 审核状态 `status`：0 待审核 1 审核通过 2 已拒绝（默认 0）
    pub status: i32,
    /// 拒绝原因 `reject_remark`（可空）
    pub reject_remark: Option<String>,
    /// 前台显示顺序 `sort_order`，数字越大越靠后（默认 50）
    pub sort_order: i32,
    /// 是否显示 `is_show`：0 否 1 显示（默认 1）
    pub is_show: i32,
    /// 品牌分类 `brand_type`（可空，默认 ''）
    pub brand_type: Option<String>,
    /// 是否热销品牌 `brand_is_hot`：0 否 1 是（默认 0）
    pub brand_is_hot: i32,
    /// 品牌首字母 `first_word`（可空，默认 ''）
    pub first_word: Option<String>,
    /// （已废弃）`is_store_brand`（默认 0）
    pub is_store_brand: i32,
    /// 审核状态 `check_status`：0 待审核 1 审核通过 2 不通过（默认 0）
    pub check_status: i32,
}

impl Brand {
    /// 表名。
    pub const TABLE: &'static str = "brand";

    /// 待审核状态（对应 `BrandStatusEnum.PENDING` / `CheckStatus.PENDING`）。
    pub const STATUS_PENDING: i32 = 0;
    /// 审核通过状态（对应 `BrandStatusEnum.APPROVED`）。
    pub const STATUS_APPROVED: i32 = 1;
    /// 已拒绝状态（对应 `BrandStatusEnum.REJECTED`）。
    pub const STATUS_REJECTED: i32 = 2;

    /// 是否在前台显示（`is_show = 1`）。
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.is_show == 1
    }

    /// 是否已审核通过（`status = 1`）。
    #[must_use]
    pub fn is_approved(&self) -> bool {
        self.status == Self::STATUS_APPROVED
    }

    /// 是否平台自营（`shop_id = 0`，管理端直接建品牌默认审核通过）。
    #[must_use]
    pub fn is_platform(&self) -> bool {
        self.shop_id == 0
    }

    /// 审核状态文案，对应 `BrandStatusEnum.getStatusName`。
    #[must_use]
    pub fn status_text(&self) -> Option<&'static str> {
        match self.status {
            Self::STATUS_PENDING => Some("待审核"),
            Self::STATUS_APPROVED => Some("审核通过"),
            Self::STATUS_REJECTED => Some("已拒绝"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_category() {
        let c = Category {
            parent_id: 0,
            ..Default::default()
        };
        assert!(c.is_root());
        let child = Category {
            parent_id: 3,
            ..Default::default()
        };
        assert!(!child.is_root());
    }

    #[test]
    fn visibility_and_hot_flags() {
        let c = Category {
            is_show: 1,
            is_hot: 1,
            ..Default::default()
        };
        assert!(c.is_visible());
        assert!(c.is_hot_category());
        let hidden = Category {
            is_show: 0,
            ..Default::default()
        };
        assert!(!hidden.is_visible());
    }

    #[test]
    fn brand_status_text() {
        let mut b = Brand {
            status: Brand::STATUS_PENDING,
            ..Default::default()
        };
        assert_eq!(b.status_text(), Some("待审核"));
        b.status = Brand::STATUS_APPROVED;
        assert_eq!(b.status_text(), Some("审核通过"));
        b.status = Brand::STATUS_REJECTED;
        assert_eq!(b.status_text(), Some("已拒绝"));
        b.status = 9;
        assert_eq!(b.status_text(), None);
        assert!(Brand {
            shop_id: 0,
            ..Default::default()
        }
        .is_platform());
    }
}