//! 外卖聚合服务组（takeout 上下文）。
//!
//! 来源标注：**我方新增**（设计 `~/workspace/takeout-aggregation/DESIGN.md`，
//! 用户 2026-11-21 05:57 对论坛 C 类第 ③ 项拍板按分期办）。一期：
//! A 向订单汇入聚合框架 + 美团闪购直连；二期（用户 2026-11-21 22:50
//! 拍板）：淘宝闪购（饿了么）+ 抖音生活服务直连补齐。B 向配送聚合
//! 只留接口位（见 [`channel::MaiyatianAggregator`]，商务未到位不写死）。

pub mod admin;
pub mod channel;
pub mod douyin;
pub mod eleme;
pub mod ingest;
pub mod meituan;

use std::sync::Arc;

/// 外卖聚合服务聚合（挂载于 `Services.takeout` / 两端 State）。
#[derive(Clone)]
pub struct Services {
    /// 订单接入（回调处理链）。
    pub ingest: Arc<ingest::TakeoutIngestService>,
    /// 后台管理（配置/绑定/映射/外部单）。
    pub admin: Arc<admin::TakeoutAdminService>,
}
