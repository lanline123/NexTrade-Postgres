//! o2o 域（本地生活/到店）DTO / 查询参数 / VO。
//!
//! Java 来源（overseas）：`nextrade-bean/src/main/java/com/nextrade/bean/{model,dto,vo}/o2o/`。
//! overseas 工程未提供 `/adminapi/o2o/**` 控制器；路由路径以线上管理端前端
//! 契约为准（`java/view/admin/src/api/setting/*.ts`、`api/product/inventoryCalendar.ts`），
//! 字段命名对齐 Java 实体 / 前端 TS 类型（camelCase）。

pub mod config;
pub mod reservation_tpl;
pub mod stock_calendar;
pub mod store_product;
