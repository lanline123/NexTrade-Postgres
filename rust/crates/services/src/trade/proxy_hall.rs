//! 代客下单接单大厅服务（trade::proxy_hall，我方新增，二期）。
//!
//! 论坛帖 547750 的另一设想：一期（跨店履约 + 三方分成）在下单时
//! 按地址选定履约门店；二期「接单大厅」把履约店的确定推迟到下单
//! 之后——代客单以大厅方式提交后进待接单池，同体系营业门店抢单，
//! 或由下单店/平台派单；接单后履约店落 `proxy_order_ext` 的
//! `fulfill_shop_id`，结算三方分成与退款冲正沿一期口径逐字不变。
//!
//! 口径（与一期的衔接与差异，逐条）：
//! - **入池**：提交时 ext 行 hall_status=1、fulfill_shop_id=0、比例
//!   按当时配置快照（同一期）、接单截止 = now + 超时分钟配置。
//! - **计价差异**：大厅单履约店未定，无法按履约店价计价——按
//!   下单店口径计价（无 line_price_overrides、活动/满减按代客单
//!   现行规则、改价不禁；一期的禁改价只针对直选跨店单）。
//! - **可见性（接单资格/区域）**：沿一期候选集口径——查看者店铺
//!   与下单店同体系（同体系根）且营业中（status=1）才可见池；
//!   拒过该单的店铺不再看到该单（拒单记录排除）。
//! - **抢单**：仓储条件 UPDATE 原子认领，并发只有一个赢家。
//! - **派单**：下单店账号（或平台账号）把池中单指派给同体系营业
//!   门店；目标合法性按一期候选判定强校验。
//! - **拒单**：已接单门店可退单回池（改派计数 +1、截止按新一轮
//!   窗口刷新、落拒单记录），下单店收到消息提醒。
//! - **备货**：履约店接单后可标「已备货」（hall_status 3），仅作
//!   交接标记，不改变订单/结算状态机；发货仍由下单店在订单管理
//!   完成（大厅只决定「谁履约」，不接管订单操作作用域）。
//! - **超时兜底**（定时任务 `run_timeout_scan`）：截止已过仍无人
//!   接单时，按 `proxyHallFallback` 处置：1 = 自动改派下单店自
//!   履约（assign_type=3、fulfill=下单店，结算退化同店口径并发
//!   消息提醒下单店）；0 = 清除截止、继续留池（只处置一次）。
//!   订单被取消的大厅单一并联动关闭（hall_status=4）。
//! - **结算安全**：大厅单在接单前即完成结算时，结算链对
//!   fulfill<=0 / fulfill=下单店 的 ext 行退化同店口径（见
//!   settlement.rs 守卫），不会给 0 号店入账。
//! - **通知**：入池时给每个候选门店落 admin_msg（类型 60「代客
//!   接单」）；派单落目标店、拒单/兜底落下单店。通知失败不阻断
//!   主流程（best-effort，与消息中心既有口径一致）。

use std::sync::Arc;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use nextrade_core::models::trade::proxy_hall::{
    HallActionVO, HallAssignParam, HallOrderActionParam, HallOrderVO, HallRejectParam,
    ASSIGN_TYPE_ASSIGN, ASSIGN_TYPE_FALLBACK, ASSIGN_TYPE_GRAB, HALL_MSG_TYPE,
    HALL_STATUS_ACCEPTED, HALL_STATUS_PENDING, HALL_STATUS_READY,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::trade::proxy_hall::HallOrderRow;
use nextrade_infrastructure::db::Repositories;

use super::proxy_shopping_order_check::{
    family_root_of, is_family_candidate, RequestContext, SERVICE_DATA_ERROR,
};

/// 接单大厅总开关（订单设置 `proxyHallEnabled`，默认关）。
pub const CFG_PROXY_HALL_ENABLED: &str = "proxyHallEnabled";
/// 接单超时分钟（订单设置 `proxyHallTimeoutMinutes`，默认 30）。
pub const CFG_PROXY_HALL_TIMEOUT_MINUTES: &str = "proxyHallTimeoutMinutes";
/// 超时兜底策略（订单设置 `proxyHallFallback`，默认 1 = 改派下单店）。
pub const CFG_PROXY_HALL_FALLBACK: &str = "proxyHallFallback";

/// 大厅未开启文案。
pub const MSG_HALL_DISABLED: &str = "接单大厅未开启";
/// 需要店铺账号文案。
pub const MSG_HALL_NEED_SHOP_SCOPE: &str = "接单大厅需以店铺账号操作";
/// 订单不在池中/已被接单文案。
pub const MSG_HALL_NOT_IN_POOL: &str = "订单不在接单大厅中或已被接单";
/// 无权操作文案。
pub const MSG_HALL_FORBIDDEN: &str = "无权操作该大厅订单";
/// 派单目标非法文案（与一期履约店非法同义）。
pub const MSG_HALL_TARGET_INVALID: &str = "派单门店不属于下单店体系或已停业";

/// 大厅三项配置。
#[derive(Debug, Clone, Copy)]
pub struct HallConfig {
    pub enabled: bool,
    pub timeout_minutes: i64,
    pub fallback_to_proxy: bool,
}

/// 代客下单接单大厅服务。
pub struct ProxyHallService {
    repos: Arc<Repositories>,
}

impl ProxyHallService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn hall_repo(&self) -> &nextrade_infrastructure::db::repo::trade::proxy_hall::ProxyHallRepository {
        &self.repos.trade.proxy_hall
    }

    fn check_repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::trade::proxy_shopping_order_check::ProxyShoppingOrderCheckRepository
    {
        &self.repos.trade.proxy_shopping_order_check
    }

    /// 读大厅配置（缺失/脏数据回 FieldSpec 默认：关 / 30 分钟 / 兜底改派）。
    pub async fn hall_config(&self) -> Result<HallConfig> {
        let keys = [
            CFG_PROXY_HALL_ENABLED,
            CFG_PROXY_HALL_TIMEOUT_MINUTES,
            CFG_PROXY_HALL_FALLBACK,
        ];
        let cfg = self.check_repo().find_config_vals(&keys).await?;
        let val = |k: &str| cfg.get(k).map(String::as_str).unwrap_or("");
        let timeout = val(CFG_PROXY_HALL_TIMEOUT_MINUTES)
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|v| *v > 0)
            .unwrap_or(30);
        Ok(HallConfig {
            enabled: val(CFG_PROXY_HALL_ENABLED).trim() == "1",
            timeout_minutes: timeout,
            fallback_to_proxy: val(CFG_PROXY_HALL_FALLBACK).trim() != "0",
        })
    }

    /// 当前账号的体系根与营业状态：平台账号（shop_id=0）返回 None。
    async fn viewer_family_root(&self, shop_id: i32) -> Result<Option<i32>> {
        if shop_id <= 0 {
            return Ok(None);
        }
        let Some(row) = self.check_repo().find_shop_family_row(shop_id).await? else {
            return Ok(None);
        };
        if row.status != Some(1) {
            return Ok(None);
        }
        Ok(Some(family_root_of(row.shop_type, row.store_parent_id, shop_id)))
    }

    /// 体系店铺 id 集（根 + 直属门店，含停业——可见性再按营业过滤在
    /// 池查询的下单店维度不适用：池按下单店归属过滤，查看者营业
    /// 校验已在 viewer_family_root 完成）。
    async fn family_shop_ids(&self, root: i32) -> Result<Vec<i32>> {
        let rows = self.check_repo().list_family_shops(root).await?;
        Ok(rows.into_iter().map(|r| r.shop_id).collect())
    }

    /// 查看门店坐标（体系行 shop_longitude/latitude，双全才算有）：
    /// 池距离展示/排序与「我的接单」距离的基准点。门店未维护坐标
    /// 时距离恒空（前端显示「-」，不造数）。
    async fn viewer_coords(&self, shop_id: i32) -> Result<Option<(Decimal, Decimal)>> {
        if shop_id <= 0 {
            return Ok(None);
        }
        let Some(row) = self.check_repo().find_shop_family_row(shop_id).await? else {
            return Ok(None);
        };
        Ok(match (row.shop_longitude, row.shop_latitude) {
            (Some(lng), Some(lat)) => Some((lng, lat)),
            _ => None,
        })
    }

    fn row_to_vo(row: HallOrderRow, viewer_coords: Option<(Decimal, Decimal)>) -> HallOrderVO {
        // 池距离（米）：查看门店坐标与收货快照（迁移 041）双全时按
        // 球面距离计算并取整；任一缺失恒 None——存量单无快照、门店
        // 无坐标时距离为空属正常口径，不回填造数。
        let distance_meters = match (
            viewer_coords,
            row.receiver_longitude,
            row.receiver_latitude,
        ) {
            (Some((lng1, lat1)), Some(lng2), Some(lat2)) => {
                crate::trade::proxy_shopping_product::haversine_meters(Some(lng1), Some(lat1), Some(lng2), Some(lat2))
                    .round_dp(0)
                    .to_i64()
            }
            _ => None,
        };
        HallOrderVO {
            order_id: row.order_id,
            order_sn: row.order_sn,
            order_status: row.order_status,
            pay_status: row.pay_status,
            proxy_shop_id: row.proxy_shop_id,
            proxy_shop_title: row.proxy_shop_title,
            fulfill_shop_id: row.fulfill_shop_id,
            fulfill_shop_title: row.fulfill_shop_title,
            consignee: row.consignee,
            address: row.address,
            mobile: row.mobile,
            total_amount: row.total_amount,
            item_count: row.item_count.clamp(0, i64::from(i32::MAX)) as i32,
            first_product_name: row.first_product_name,
            distance_meters,
            hall_status: row.hall_status,
            hall_deadline: row.hall_deadline,
            accept_time: row.accept_time,
            assign_type: row.assign_type,
            reassign_count: row.reassign_count,
            add_time: row.add_time,
        }
    }

    /// 待接单池（店铺账号）：同体系下单店的在池单。
    /// 平台账号无店铺作用域，返回空（大厅是门店间机制）。
    /// `sort_by_distance` 时按与本店的距离升序（无快照单置后），
    /// 缺省保持截止优先的现行口径。
    pub async fn pool(
        &self,
        query: &PageQuery,
        ctx: &RequestContext,
        sort_by_distance: bool,
    ) -> Result<PageResult<HallOrderVO>> {
        let shop_id = ctx.shop_id_or_zero();
        let (limit, offset) = query.limit_offset();
        let Some(root) = self.viewer_family_root(shop_id).await? else {
            return Ok(PageResult::new(0, Vec::new(), query));
        };
        let ids = self.family_shop_ids(root).await?;
        let coords = self.viewer_coords(shop_id).await?;
        let (rows, total) = self
            .hall_repo()
            .pool_list(&ids, shop_id, coords, sort_by_distance, limit, offset)
            .await?;
        Ok(PageResult::new(
            total.max(0) as u64,
            rows.into_iter().map(|r| Self::row_to_vo(r, coords)).collect(),
            query,
        ))
    }

    /// 我的接单（店铺账号）：本店为履约店的已接单/已备货单。
    pub async fn mine(
        &self,
        query: &PageQuery,
        ctx: &RequestContext,
    ) -> Result<PageResult<HallOrderVO>> {
        let shop_id = ctx.shop_id_or_zero();
        let (limit, offset) = query.limit_offset();
        if shop_id <= 0 {
            return Ok(PageResult::new(0, Vec::new(), query));
        }
        let (rows, total) = self.hall_repo().mine_list(shop_id, limit, offset).await?;
        let coords = self.viewer_coords(shop_id).await?;
        Ok(PageResult::new(
            total.max(0) as u64,
            rows.into_iter().map(|r| Self::row_to_vo(r, coords)).collect(),
            query,
        ))
    }

    /// 抢单：查看者须与下单店同体系且营业；原子认领。
    pub async fn accept(
        &self,
        param: &HallOrderActionParam,
        ctx: &RequestContext,
        admin_id: i32,
    ) -> Result<HallActionVO> {
        let order_id = require_order_id(param.order_id)?;
        let shop_id = ctx.shop_id_or_zero();
        if shop_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NEED_SHOP_SCOPE));
        }
        let ext = self
            .hall_repo()
            .find_hall_ext(order_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL))?;
        if ext.hall_status != HALL_STATUS_PENDING {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL));
        }
        // 接单资格：与下单店同体系（查看者根 == 下单店根）。
        let viewer_root = self.viewer_family_root(shop_id).await?;
        let proxy_row = self.check_repo().find_shop_family_row(ext.proxy_shop_id).await?;
        let proxy_root = proxy_row.map(|r| family_root_of(r.shop_type, r.store_parent_id, r.shop_id));
        if viewer_root.is_none() || viewer_root != proxy_root {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_FORBIDDEN));
        }
        let now = nextrade_common::utils::now_seconds() as i32;
        let claimed = self
            .hall_repo()
            .claim(order_id, shop_id, admin_id, ASSIGN_TYPE_GRAB, now)
            .await?;
        if !claimed {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL));
        }
        Ok(HallActionVO {
            order_id,
            hall_status: HALL_STATUS_ACCEPTED,
            fulfill_shop_id: shop_id,
        })
    }

    /// 派单：下单店账号或平台账号把池中单指派给同体系营业门店。
    pub async fn assign(
        &self,
        param: &HallAssignParam,
        ctx: &RequestContext,
        admin_id: i32,
    ) -> Result<HallActionVO> {
        let order_id = require_order_id(param.order_id)?;
        let target = param
            .shop_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_HALL_TARGET_INVALID))?;
        let ext = self
            .hall_repo()
            .find_hall_ext(order_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL))?;
        if ext.hall_status != HALL_STATUS_PENDING {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL));
        }
        // 派单权限：平台账号（0）或下单店本人。
        let actor = ctx.shop_id_or_zero();
        if actor != 0 && actor != ext.proxy_shop_id {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_FORBIDDEN));
        }
        // 目标合法性：一期候选判定（同体系 + 营业）。
        let proxy_row = self
            .check_repo()
            .find_shop_family_row(ext.proxy_shop_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_HALL_TARGET_INVALID))?;
        let root = family_root_of(proxy_row.shop_type, proxy_row.store_parent_id, ext.proxy_shop_id);
        let target_row = self
            .check_repo()
            .find_shop_family_row(target)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_HALL_TARGET_INVALID))?;
        let target_root = family_root_of(target_row.shop_type, target_row.store_parent_id, target);
        if !is_family_candidate(target_root, root, target_row.status) {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_TARGET_INVALID));
        }
        let now = nextrade_common::utils::now_seconds() as i32;
        let claimed = self
            .hall_repo()
            .claim(order_id, target, admin_id, ASSIGN_TYPE_ASSIGN, now)
            .await?;
        if !claimed {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL));
        }
        self.notify(
            target,
            order_id,
            "代客订单派单",
            "有一笔代客订单已派给贵店履约，请在大厅「我的接单」中查看并备货",
        )
        .await;
        Ok(HallActionVO {
            order_id,
            hall_status: HALL_STATUS_ACCEPTED,
            fulfill_shop_id: target,
        })
    }

    /// 拒单回池：仅当前履约店可退（已接单状态）；回池后该店不再
    /// 在池中看到此单，截止按新一轮窗口刷新，下单店收到提醒。
    pub async fn reject(
        &self,
        param: &HallRejectParam,
        ctx: &RequestContext,
        admin_id: i32,
    ) -> Result<HallActionVO> {
        let order_id = require_order_id(param.order_id)?;
        let shop_id = ctx.shop_id_or_zero();
        if shop_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NEED_SHOP_SCOPE));
        }
        let ext = self
            .hall_repo()
            .find_hall_ext(order_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NOT_IN_POOL))?;
        if ext.hall_status != HALL_STATUS_ACCEPTED || ext.fulfill_shop_id != shop_id {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_FORBIDDEN));
        }
        let cfg = self.hall_config().await?;
        let now = nextrade_common::utils::now_seconds() as i32;
        let new_deadline = now + (cfg.timeout_minutes * 60) as i32;
        let reason: String = param
            .reason
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(255)
            .collect();
        let released = self
            .hall_repo()
            .release_to_pool(order_id, shop_id, admin_id, &reason, new_deadline, now)
            .await?;
        if !released {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_FORBIDDEN));
        }
        self.notify(
            ext.proxy_shop_id,
            order_id,
            "代客订单被拒单",
            "履约门店已拒单，订单已退回接单大厅重新等待接单",
        )
        .await;
        Ok(HallActionVO {
            order_id,
            hall_status: HALL_STATUS_PENDING,
            fulfill_shop_id: 0,
        })
    }

    /// 备货完成：仅当前履约店可标（已接单 → 已备货）。
    pub async fn ready(
        &self,
        param: &HallOrderActionParam,
        ctx: &RequestContext,
    ) -> Result<HallActionVO> {
        let order_id = require_order_id(param.order_id)?;
        let shop_id = ctx.shop_id_or_zero();
        if shop_id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_NEED_SHOP_SCOPE));
        }
        let now = nextrade_common::utils::now_seconds() as i32;
        let marked = self.hall_repo().mark_ready(order_id, shop_id, now).await?;
        if !marked {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_HALL_FORBIDDEN));
        }
        Ok(HallActionVO {
            order_id,
            hall_status: HALL_STATUS_READY,
            fulfill_shop_id: shop_id,
        })
    }

    /// 入池通知（提交链调用）：给下单店体系内全部营业候选门店落
    /// 消息（含下单店本身——其也可自接）。best-effort，失败只记日志。
    pub async fn notify_new_pool_order(&self, order_id: i32, proxy_shop_id: i32) {
        let Some(proxy_row) = self
            .check_repo()
            .find_shop_family_row(proxy_shop_id)
            .await
            .ok()
            .flatten()
        else {
            return;
        };
        let root = family_root_of(proxy_row.shop_type, proxy_row.store_parent_id, proxy_shop_id);
        let Ok(family) = self.check_repo().list_family_shops(root).await else {
            return;
        };
        for shop in family {
            if shop.status == Some(1) {
                self.notify(
                    shop.shop_id,
                    order_id,
                    "代客订单待接单",
                    "有一笔新的代客订单进入接单大厅，请及时抢单履约",
                )
                .await;
            }
        }
    }

    /// 超时与取消兜底扫描（定时任务调用）：返回处置单数。
    /// ① 截止已过仍在池中：按兜底策略改派下单店或清除截止留池；
    /// ② 订单已取消的大厅单联动关闭。
    pub async fn run_timeout_scan(&self, now: i32) -> Result<u64> {
        let cfg = self.hall_config().await?;
        let mut handled: u64 = 0;
        if cfg.enabled {
            let expired = self.hall_repo().list_expired_pending(now, 100).await?;
            for row in expired {
                if cfg.fallback_to_proxy {
                    let claimed = self
                        .hall_repo()
                        .claim(
                            row.order_id,
                            row.proxy_shop_id,
                            0,
                            ASSIGN_TYPE_FALLBACK,
                            now,
                        )
                        .await?;
                    if claimed {
                        handled += 1;
                        self.notify(
                            row.proxy_shop_id,
                            row.order_id,
                            "代客订单超时改派",
                            "接单大厅订单超时无人接单，已自动改派贵店自履约",
                        )
                        .await;
                    }
                } else {
                    self.hall_repo().clear_deadline(row.order_id).await?;
                    handled += 1;
                }
            }
        }
        let closed = self.hall_repo().close_cancelled().await?;
        handled += closed;
        Ok(handled)
    }

    /// 消息中心落行（类型 60「代客接单」），失败不阻断主流程。
    async fn notify(&self, shop_id: i32, order_id: i32, title: &str, content: &str) {
        if shop_id <= 0 {
            return;
        }
        let related = serde_json::json!({ "orderId": order_id }).to_string();
        if let Err(e) = self
            .repos
            .system
            .admin_msg
            .create_message(HALL_MSG_TYPE, shop_id, order_id, title, content, &related)
            .await
        {
            tracing::warn!("接单大厅消息落行失败 order={order_id} shop={shop_id}: {e}");
        }
    }
}

/// 订单 id 必填校验。
fn require_order_id(order_id: Option<i32>) -> Result<i32> {
    order_id
        .filter(|id| *id > 0)
        .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "缺少订单ID"))
}

/// 大厅状态机纯函数校验（单测覆盖）：哪些迁移合法。
#[must_use]
pub fn hall_transition_allowed(from: i32, to: i32) -> bool {
    use nextrade_core::models::trade::proxy_hall::{
        HALL_STATUS_ACCEPTED as A, HALL_STATUS_CLOSED as C, HALL_STATUS_PENDING as P,
        HALL_STATUS_READY as R,
    };
    matches!(
        (from, to),
        (P, A) | (A, P) | (A, R) | (P, C) | (A, C)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hall_transitions() {
        assert!(hall_transition_allowed(1, 2)); // 待接单 → 已接单（抢/派/兜底）
        assert!(hall_transition_allowed(2, 1)); // 已接单 → 待接单（拒单回池）
        assert!(hall_transition_allowed(2, 3)); // 已接单 → 已备货
        assert!(hall_transition_allowed(1, 4)); // 待接单 → 关闭（取消）
        assert!(hall_transition_allowed(2, 4)); // 已接单 → 关闭（取消）
        assert!(!hall_transition_allowed(0, 2)); // 非大厅单不可接单
        assert!(!hall_transition_allowed(3, 1)); // 已备货不可回池
        assert!(!hall_transition_allowed(4, 2)); // 已关闭不可复活
        assert!(!hall_transition_allowed(1, 3)); // 未接单不可备货
    }
}
