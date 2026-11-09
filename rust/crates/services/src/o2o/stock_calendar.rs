//! 库存日历域服务（o2o::stock_calendar）。
//!
//! 对应原 Java（overseas）：
//! - `com.nextrade.bean.vo.o2o.StockCalendarListVO` / `StockCalendarVO`
//! - `com.nextrade.service.product.util.StockCalendarUtil`（日期范围 / 排除日）
//! - overseas 工程无 `/adminapi/o2o/stockCalendar` 控制器；端点契约以线上前端
//!   `java/view/admin/src/api/product/inventoryCalendar.ts` 与
//!   `views/product/inventoryCalendar/*.vue` 为准。
//!
//! ## 语义口径（以表结构 + 前端为准的最小实现）
//! - `available = total - used`（已用库存恒不覆盖）；
//! - `updateDailyStock`：`stockChange` 为**可预约数增量**，新可预约
//!   `= max(0, 旧可预约 + change)`，`total = used + 新可预约`；
//! - `setUnifiedStock` / `setTimeSlotStock`：把目标日期（×时间段）的
//!   **可预约数**设为 `quantity`，即 `total = used + quantity`（无行按 `used=0` 插入）；
//! - 日期区间闭区间、最多 31 天窗口（`StockCalendarUtil` 月历口径）。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use nextrade_core::models::o2o::stock_calendar::{
    filter_dates, DailyStockQuery, DailyStockVO, DateStockSummary, ProductTimeListVO,
    SetTimeSlotStockDTO, SetUnifiedStockDTO, StockCalendarListQuery, StockCalendarListVO,
    StockCalendarProductInfo, StockCalendarShopInfo, TimeSlotStockChange, TimeSlotStockVO,
    UpdateDailyStockDTO,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::o2o::stock_calendar::StockGroupKey;
use nextrade_infrastructure::db::Repositories;

/// 库存日历域服务（挂载于 `AdminState.o2o.stock_calendar`）。
pub struct StockCalendarService {
    repos: Arc<Repositories>,
}

impl StockCalendarService {
    /// 构造。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::o2o::stock_calendar::StockCalendarRepository {
        &self.repos.o2o.stock_calendar
    }

    // ————————————————————— pageList —————————————————————

    /// `GET /o2o/stockCalendar/pageList`：按「门店×商品×SKU」分组 + 日期汇总。
    pub async fn page_list(
        &self,
        q: &StockCalendarListQuery,
        shop_scope: Option<i32>,
    ) -> Result<PageResult<StockCalendarListVO>> {
        let (start, end) = q.date_range();
        let (start_s, end_s) = (start.to_string(), end.to_string());
        let (limit, offset) = q.limit_offset();
        let (total, groups) = self
            .repo()
            .paginate_groups(
                &start_s,
                &end_s,
                q.product_name.as_deref(),
                q.spec_value.as_deref(),
                &q.shop_id_list(),
                shop_scope,
                limit,
                offset,
            )
            .await?;

        let stock_rows = self
            .repo()
            .find_stocks_for_groups(&start_s, &end_s, &groups_raw(&groups))
            .await?;
        // 分组聚合：key → date → (total, used)
        let mut agg: HashMap<(i32, i32, i32), HashMap<String, (i32, i32)>> = HashMap::new();
        for r in &stock_rows {
            let e = agg
                .entry((r.shop_id, r.product_id, r.sku_id))
                .or_default();
            let slot = e.entry(r.stock_date.clone()).or_insert((0, 0));
            slot.0 += r.total_stock;
            slot.1 += r.used_stock;
        }

        // 门店名 / 商品图
        let shop_ids: Vec<i32> = groups.iter().map(|(g, _)| g.shop_id).collect();
        let shop_names = self.repo().find_shop_names(&shop_ids).await?;
        let shop_map: HashMap<i32, String> = shop_names.into_iter().collect();
        let product_ids: Vec<i32> = groups.iter().map(|(g, _)| g.product_id).collect();
        let products = self.repo().find_product_infos(&product_ids).await?;
        let product_map: HashMap<i32, (String, String)> = products
            .into_iter()
            .map(|(id, name, _url, thumb, _orig)| (id, (name, thumb)))
            .collect();

        let all_dates = date_list(&start, &end);
        let mut vos = Vec::with_capacity(groups.len());
        for (g, fallback_name) in &groups {
            let empty_dates = HashMap::new();
            let dates = agg
                .get(&(g.shop_id, g.product_id, g.sku_id))
                .unwrap_or(&empty_dates);
            let date_stock_list = all_dates
                .iter()
                .map(|d| {
                    let key = d.to_string();
                    let (total, used) = dates.get(&key).copied().unwrap_or((0, 0));
                    DateStockSummary {
                        stock_date: key,
                        total_stock: total,
                        used_stock: used,
                        available_stock: (total - used).max(0),
                    }
                })
                .collect();
            let product_info = product_map.get(&g.product_id);
            vos.push(StockCalendarListVO {
                product_info: StockCalendarProductInfo {
                    product_id: g.product_id,
                    product_name: Some(
                        product_info
                            .map(|(n, _)| n.clone())
                            .unwrap_or_else(|| fallback_name.clone()),
                    ),
                    product_image: product_info.map(|(_, t)| t.clone()),
                    sku_id: g.sku_id,
                    spec_value: if g.sku_id > 0 {
                        self.repo().find_sku_value(g.sku_id).await?
                    } else {
                        None
                    },
                },
                shop_info: StockCalendarShopInfo {
                    shop_id: g.shop_id,
                    shop_name: shop_map.get(&g.shop_id).cloned(),
                },
                date_stock_list,
            });
        }
        let query = PageQuery {
            page: q.page,
            page_size: q.size,
        };
        Ok(PageResult::new(total, vos, &query))
    }

    // ————————————————————— getDailyStock —————————————————————

    /// `GET /o2o/stockCalendar/getDailyStock`：某日各时间段库存。
    pub async fn daily_stock(&self, q: &DailyStockQuery) -> Result<DailyStockVO> {
        if q.product_id <= 0 || q.shop_id <= 0 {
            return Err(Error::Validation("productId / shopId 不能为空".into()));
        }
        let sku_id = q.sku_id.unwrap_or(0);
        let times = self.repo().find_times(q.product_id).await?;
        let stocks = self
            .repo()
            .find_daily_stocks(q.product_id, q.shop_id, sku_id, &q.stock_date)
            .await?;
        let by_time: HashMap<i32, &nextrade_core::entities::o2o::ProductReservationStock> =
            stocks.iter().map(|s| (s.product_reservation_time_id, s)).collect();
        let list = times
            .iter()
            .map(|t| {
                let row = by_time.get(&t.product_reservation_time_id).copied();
                let (total, used) = row
                    .map(|s| (s.total_stock, s.used_stock))
                    .unwrap_or((0, 0));
                TimeSlotStockVO {
                    product_reservation_time_id: t.product_reservation_time_id,
                    start_time: Some(t.start_time.clone()),
                    end_time: Some(t.end_time.clone()),
                    total_stock: total,
                    used_stock: used,
                    available_stock: (total - used).max(0),
                    stock_change: 0,
                }
            })
            .collect();
        Ok(DailyStockVO {
            product_id: q.product_id,
            shop_id: q.shop_id,
            sku_id,
            stock_date: q.stock_date.clone(),
            time_slot_stock_list: list,
        })
    }

    // ————————————————————— updateDailyStock —————————————————————

    /// `POST /o2o/stockCalendar/updateDailyStock`：按增量调整某日库存。
    pub async fn update_daily_stock(&self, dto: &UpdateDailyStockDTO) -> Result<()> {
        if dto.product_id <= 0 || dto.shop_id <= 0 {
            return Err(Error::Validation("productId / shopId 不能为空".into()));
        }
        if dto.stock_date.trim().is_empty() {
            return Err(Error::Validation("stockDate 不能为空".into()));
        }
        if dto.time_slot_stock_changes.is_empty() {
            return Err(Error::Validation("timeSlotStockChanges 不能为空".into()));
        }
        let sku_id = dto.sku_id.unwrap_or(0);
        let now = nextrade_common::utils::now_millis() as i32;
        for change in &dto.time_slot_stock_changes {
            self.apply_change(dto.product_id, dto.shop_id, sku_id, &dto.stock_date, change, now)
                .await?;
        }
        Ok(())
    }

    /// 应用单条增量。
    async fn apply_change(
        &self,
        product_id: i32,
        shop_id: i32,
        sku_id: i32,
        stock_date: &str,
        change: &TimeSlotStockChange,
        now: i32,
    ) -> Result<()> {
        let time_id = change.product_reservation_time_id;
        if time_id <= 0 {
            return Err(Error::Validation("productReservationTimeId 不能为空".into()));
        }
        let row = self
            .repo()
            .find_stock_row(product_id, shop_id, sku_id, stock_date, time_id)
            .await?;
        let (used, available) = row
            .map(|r| (r.used_stock, (r.total_stock - r.used_stock).max(0)))
            .unwrap_or((0, 0));
        let new_available = (available + change.stock_change.unwrap_or(0)).max(0);
        self.repo()
            .upsert_stock(
                product_id,
                shop_id,
                sku_id,
                stock_date,
                time_id,
                used + new_available,
                used,
                now,
            )
            .await
    }

    // ————————————————————— setUnifiedStock / setTimeSlotStock —————————————————————

    /// `POST /o2o/stockCalendar/setUnifiedStock`：区间统一设置可预约库存。
    pub async fn set_unified_stock(&self, dto: &SetUnifiedStockDTO) -> Result<u64> {
        let shops = self.resolve_shops(dto.shop_selection.as_ref()).await?;
        if shops.is_empty() {
            return Err(Error::Validation("请选择门店".into()));
        }
        let start = parse_date(&dto.start_date)?;
        let end = parse_date(&dto.end_date)?;
        let dates = filter_dates(&date_list(&start, &end), dto.date_exclusion.as_ref());
        if dates.is_empty() {
            return Ok(0);
        }
        let times = self.repo().find_times(dto.product_id).await?;
        if times.is_empty() {
            return Err(Error::Biz {
                code: 500,
                msg: "该商品未配置预约时间段".into(),
            });
        }
        let sku_id = dto.sku_id.unwrap_or(0);
        let now = nextrade_common::utils::now_millis() as i32;
        let mut affected: u64 = 0;
        for shop_id in &shops {
            for date in &dates {
                for t in &times {
                    affected += self
                        .set_available(
                            dto.product_id,
                            *shop_id,
                            sku_id,
                            &date.to_string(),
                            t.product_reservation_time_id,
                            dto.quantity,
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(affected)
    }

    /// `POST /o2o/stockCalendar/setTimeSlotStock`：按时间段设置可预约库存。
    pub async fn set_time_slot_stock(&self, dto: &SetTimeSlotStockDTO) -> Result<u64> {
        if dto.time_slot_stock_items.is_empty() {
            return Err(Error::Validation("timeSlotStockItems 不能为空".into()));
        }
        let shops = self.resolve_shops(dto.shop_selection.as_ref()).await?;
        if shops.is_empty() {
            return Err(Error::Validation("请选择门店".into()));
        }
        let start = parse_date(&dto.start_date)?;
        let end = parse_date(&dto.end_date)?;
        let dates = filter_dates(&date_list(&start, &end), dto.date_exclusion.as_ref());
        if dates.is_empty() {
            return Ok(0);
        }
        let sku_id = dto.sku_id.unwrap_or(0);
        let now = nextrade_common::utils::now_millis() as i32;
        let mut affected: u64 = 0;
        for shop_id in &shops {
            for date in &dates {
                for item in &dto.time_slot_stock_items {
                    affected += self
                        .set_available(
                            dto.product_id,
                            *shop_id,
                            sku_id,
                            &date.to_string(),
                            item.product_reservation_time_id,
                            item.quantity,
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(affected)
    }

    /// 把某行可预约数设为 `quantity`（`total = used + quantity`）。
    /// 返回写入行数（0/1）。
    async fn set_available(
        &self,
        product_id: i32,
        shop_id: i32,
        sku_id: i32,
        stock_date: &str,
        time_id: i32,
        quantity: i32,
        now: i32,
    ) -> Result<u64> {
        let row = self
            .repo()
            .find_stock_row(product_id, shop_id, sku_id, stock_date, time_id)
            .await?;
        let used = row.map(|r| r.used_stock).unwrap_or(0);
        self.repo()
            .upsert_stock(
                product_id,
                shop_id,
                sku_id,
                stock_date,
                time_id,
                used + quantity.max(0),
                used,
                now,
            )
            .await?;
        Ok(1)
    }

    /// 解析门店选择：1 → 全部启用门店；2/3 → 指定 `shopIds`（店铺账号再叠加自己的范围）。
    async fn resolve_shops(&self, selection: Option<&nextrade_core::models::o2o::stock_calendar::ShopSelectionDTO>) -> Result<Vec<i32>> {
        let sel = selection.ok_or_else(|| Error::Validation("shopSelection 不能为空".into()))?;
        let mut shops: Vec<i32> = match sel.selection_type {
            1 => self
                .repos
                .o2o
                .store_product
                .find_all_shop_ids()
                .await?
                .into_iter()
                .map(|id| id as i32)
                .collect(),
            _ => sel.shop_ids.clone().unwrap_or_default(),
        };
        shops.retain(|x| *x > 0);
        shops.sort_unstable();
        shops.dedup();
        Ok(shops)
    }

    // ————————————————————— getProductTimeList —————————————————————

    /// `GET /o2o/stockCalendar/getProductTimeList`：商品的时间段配置（含商品信息）。
    pub async fn product_time_list(&self, product_id: i32) -> Result<ProductTimeListVO> {
        if product_id <= 0 {
            return Err(Error::Validation("productId 不能为空".into()));
        }
        let times = self.repo().find_times(product_id).await?;
        let products = self.repo().find_product_infos(&[product_id]).await?;
        let (name, thumb) = products
            .first()
            .map(|(_, n, _u, t, _o)| (n.clone(), t.clone()))
            .unwrap_or_default();
        Ok(ProductTimeListVO {
            product_id,
            product_name: Some(name),
            pic_url: None,
            pic_thumb: Some(thumb),
            pic_original: None,
            time_slot_stock_list: times
                .into_iter()
                .map(|t| TimeSlotStockVO {
                    product_reservation_time_id: t.product_reservation_time_id,
                    start_time: Some(t.start_time),
                    end_time: Some(t.end_time),
                    total_stock: 0,
                    used_stock: 0,
                    available_stock: 0,
                    stock_change: 0,
                })
                .collect(),
        })
    }
}

/// 取分组键列表。
fn groups_raw(groups: &[(StockGroupKey, String)]) -> Vec<StockGroupKey> {
    groups.iter().map(|(g, _)| g.clone()).collect()
}

/// 生成闭区间日期列表（`NaiveDate`）。
fn date_list(start: &NaiveDate, end: &NaiveDate) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut cur = *start;
    while cur <= *end {
        out.push(cur);
        cur += chrono::Duration::days(1);
    }
    out
}

/// 解析 `yyyy-MM-dd`。
fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| Error::Validation(format!("日期格式错误: {s}（应为 yyyy-MM-dd）")))
}
