//! 数据库实体层。
//!
//! 对应原 Java `nextrade-bean/src/main/java/com/nextrade/bean/model/**`，
//! 字段名与数据库列名通过 `#[sqlx(rename_all = "snake_case")]` 严格对齐，
//! 保证与既有 MySQL 数据库零迁移兼容。

pub mod admin_user;
pub mod cart;
pub mod catalog;
pub mod content;
pub mod coupon;
pub mod logistics;
pub mod member;
pub mod order;
pub mod product;
pub mod user;

pub use admin_user::AdminUser;
pub use cart::Cart;
pub use catalog::{Brand, Category};
pub use content::{Article, ArticleCategory};
pub use coupon::{
    parse_json_i32_array, Coupon, CouponStatus, UserCoupon, UserCouponStatus, COUPON_TYPE_DISCOUNT,
    COUPON_TYPE_MONEY, NEW_USER_MEMBER, NEW_USER_NONE, NEW_USER_ONLY, SEND_TYPE_BY_CLAIM,
    SEND_TYPE_FIXED,
};
pub use logistics::{LogisticsCompany, ShippingTpl, ShippingTplInfo, ShippingType};
pub use member::{MemberTag, MemberTagGroup, UserPointsLog, UserRank};
pub use order::{Order, OrderItem};
pub use product::{Product, ProductSku};
pub use user::User;