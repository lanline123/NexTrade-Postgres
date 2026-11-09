//! 商品库存管理域（catalog::product_stock）服务。
//!
//! 逐条对照原 Java 实现：
//! - `ProductStockRecordServiceImpl`（出入库、记录、批量导入）
//! - `ProductStockAdjustRecordServiceImpl`（库存调整）
//! - `ProductStockAllocateRecordServiceImpl`（调拨全链路）
//! - `ProductInventoryLogServiceImpl#getDetail`（出入库明细）
//! - `ProductSkuServiceImpl#getProductStockPage`（库存分页）
//! - `ProductStockController` 各端点
//!
//! ## 与原 Java 的语义对齐要点
//! - **组织（shopId）**：Java 用 `HeaderUtils.getShopId()` 从请求头
//!   `x-admin-type` + `x-shop-id` 解析；Rust 的 JWT 未签发 shop_id，
//!   故路由层按同一规则解析请求头后以 [`StockOperator`] 传入（见路由文件注释）。
//! - **出入库**：`productStockIn` / `productStockOut` 的库存落点由
//!   `resolveAssignedStoreProduct` 决定 ——
//!   商品属于当前组织 → 直接改 `product` / `product_sku`；
//!   平台商品（`shop_id = 0`）且当前组织为门店 → 改
//!   `store_product` / `store_product_sku`；其余报错。
//!   出库一律 `UPDATE ... WHERE 库存 >= ?` 原子扣减。
//! - **库存调整**：`adjustType = 1` 增量（按符号增减），`adjustType = 0` 覆盖
//!   （目标值 = `quantity`）。SKU 场景会同步把商品级库存按「旧 SKU 库存 - 新目标值」
//!   调整（Java 原逻辑，含 `productSku.getSkuStock() - quantity` 的溢出判定）。
//! - **调拨状态机**：1 待审核 → 2 待调出 → 3 待调入 → 4 已完成；
//!   5 已驳回、6 已作废。审核人必须是入库方；调出方必须是出库方。
//! - **未迁移**：Redis 分布式锁（`addOrDescStockLock`）、库存预警
//!   （`stockWarningService.evaluate`）、多语言翻译（`TranslatePackage`）、
//!   配置项（`SHOP_NAME` / `nextradeProperties.isO2o`）、Excel 导入导出。
//!   原锁只用于串行化「读旧值 + 写新值 + 记日志」，Rust 侧改为
//!   **原子条件 `UPDATE`**（`WHERE 库存 >= ?`）并由数据库事务保证一致性，
//!   防超卖语义不降级。

use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_common::utils;
use nextrade_core::entities::{
    Product, ProductInventoryLog, ProductSku, ProductStockAdjustRecord,
    ProductStockAllocateRecord, ProductStockRecord, ProductStockRecordDetail, StoreProduct,
};
use nextrade_core::models::catalog::product_stock::{
    allocate_operation, allocate_status, product_type, record_type, AdjustDetailQuery,
    AllocateOrgVO, AllocatePageQuery, AllocateStockOutDTO, AuditAllocateQuery, CreateAllocateDTO,
    ProductStockAdjustRecordDetailVO, ProductStockAllocateOperationRecordVO,
    ProductStockAllocateRecordDetailVO, ProductStockAllocateRecordPageVO, ProductStockDetailVO,
    ProductStockPageVO, ProductStockRecordDetailVO, ProductStockRecordVO, StockAdjustDTO,
    StockDetailQuery, StockInDTO, StockPageQuery, StockRecordDetailItem, StockRecordPageQuery,
    EXCEL_UNMIGRATED_MSG,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 请求上下文中的「组织 + 操作人」（替代 Java 的
/// `HeaderUtils.getShopId()` + `SecurityUtils.getCurrentUsername()`）。
#[derive(Debug, Clone, Default)]
pub struct StockOperator {
    /// 当前组织 ID（0 = 平台），对应 Java `HeaderUtils.getShopId()`。
    pub shop_id: i32,
    /// 当前登录用户名，对应 Java `SecurityUtils.getCurrentUsername()`。
    pub username: Option<String>,
}

impl StockOperator {
    /// 构造。
    #[must_use]
    pub fn new(shop_id: i32, username: Option<String>) -> Self {
        Self { shop_id, username }
    }

    /// 操作人名字切片。
    #[must_use]
    pub fn username_str(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

pub struct ProductStockService {
    repos: Arc<Repositories>,
}

impl ProductStockService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::catalog::product_stock::ProductStockRepository {
        &self.repos.catalog.product_stock
    }

    // ————————————————————— 库存分页 / 出入库明细 —————————————————————

    /// `GET /page`：库存分页（Java `ProductStockController.page`
    /// → `ProductSkuService.getProductStockPage`）。
    ///
    /// 派生字段规则与 Java 完全一致：
    /// - 卡密商品（`product_type = 3`）：当前库存 = 关联卡券分组中未使用卡券数
    ///   （分组不存在或未启用则为 0）；
    /// - 预约类商品（`product_type = 5/6`）：当前库存为 `null`；
    /// - 有 SKU 时用 SKU 库存 / SKU 成本价覆盖，且成本价为 0 时置 `null`。
    pub async fn stock_page(
        &self,
        q: &StockPageQuery,
        op: &StockOperator,
    ) -> Result<PageResult<ProductStockPageVO>> {
        // Java：DTO.shopId 为空时用请求头兜底
        let shop_id = q.shop_id.unwrap_or(op.shop_id);
        let page = q.page_query();
        let (total, rows) = self.repo().paginate_stock_page(q, shop_id).await?;

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let mut vo = ProductStockPageVO {
                product_id: row.product_id,
                product_name: row.product_name.clone(),
                product_sku_name: row.product_sku_name.clone(),
                product_sku_id: None,
                product_unit: row.product_unit.clone(),
                current_product_stock: Some(row.product_stock),
                product_cost: row.product_cost,
                product_pic_thumb: row.product_pic_thumb.clone(),
                product_sn: row.product_sn.clone(),
                product_tsn: row.product_tsn.clone(),
            };

            if row.product_type == Some(product_type::CARD) {
                vo.current_product_stock = Some(self.card_stock(row.card_group_id).await?);
            } else if product_type::is_reservation(row.product_type) {
                vo.current_product_stock = None;
            }

            if let Some(sku_id) = row.sku_id {
                vo.product_sku_id = Some(sku_id);
                vo.current_product_stock = row.product_sku_stock;
                vo.product_sku_name = row.product_sku_name.clone();
                vo.product_cost = row.product_sku_cost;
                if row.product_sku_cost == Some(Decimal::ZERO) {
                    vo.product_cost = None;
                }
            }
            list.push(vo);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 卡密商品的可售库存（Java：分组有效时取未使用卡券数，否则 0）。
    async fn card_stock(&self, card_group_id: i32) -> Result<i32> {
        if self.repo().find_card_group(card_group_id).await?.is_none() {
            return Ok(0);
        }
        let count = self.repo().count_unused_cards(card_group_id).await?;
        Ok(i32::try_from(count).unwrap_or(i32::MAX))
    }

    /// `GET /stockDetail`：出入库明细分页
    /// （Java `ProductInventoryLogServiceImpl#getDetail`）。
    ///
    /// Java 在无数据时返回 `null`；Rust 统一返回空分页结果。
    pub async fn stock_detail(
        &self,
        q: &StockDetailQuery,
        op: &StockOperator,
    ) -> Result<PageResult<ProductStockDetailVO>> {
        let page = q.page_query();
        let product_id = q
            .product_id
            .ok_or_else(|| Error::Validation("商品ID不能为空".into()))?;
        let (total, logs) = self
            .repo()
            .paginate_inventory_log(
                product_id,
                q.sku_id,
                op.shop_id,
                page.page_size,
                (page.page - 1) * page.page_size,
            )
            .await?;

        // Java：skuId 不为空时，每条记录都补该 SKU 的规格名
        let sku_name = match q.sku_id {
            None => None,
            Some(sku_id) => self.repo().find_sku(sku_id).await?.map(|s| s.sku_value),
        };

        let product_ids: Vec<i32> = logs.iter().filter_map(|l| l.product_id).collect();
        let briefs = self.repo().find_product_briefs(&product_ids).await?;

        let list = logs
            .into_iter()
            .map(|log| {
                let brief = log.product_id.and_then(|id| briefs.get(&id));
                ProductStockDetailVO {
                    desc: log.desc.clone(),
                    product_name: brief.map(|b| b.0.clone()).unwrap_or_default(),
                    product_sku_name: sku_name.clone(),
                    quantity: log.change_number,
                    product_pic_thumb: brief.map(|b| b.2.clone()).unwrap_or_default(),
                    in_or_out: log.r#type,
                }
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 出入库记录 —————————————————————

    /// `GET /record/page`：出入库记录分页
    /// （Java `ProductStockRecordServiceImpl#productStockRecordPage`）。
    ///
    /// Java 在服务层用请求头 `x-shop-id` 覆盖 DTO 的 `shopId`，
    /// `beginTime` / `endTime`（`yyyy-MM-dd`）解析为当日 00:00:00 / 23:59:59 秒。
    pub async fn record_page(
        &self,
        q: &StockRecordPageQuery,
        op: &StockOperator,
    ) -> Result<PageResult<ProductStockRecordVO>> {
        let begin = q
            .begin_time
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(day_start_seconds)
            .transpose()?;
        let end = q
            .end_time
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(day_end_seconds)
            .transpose()?;

        let page = q.page_query();
        let (total, rows) = self
            .repo()
            .paginate_stock_records(q, op.shop_id, begin, end)
            .await?;

        let list = rows
            .into_iter()
            .map(|r| ProductStockRecordVO {
                product_stock_id: r.product_stock_id,
                stock_code: r.stock_code,
                r#type: r.r#type,
                add_by: r.add_by,
                comment: r.comment,
                add_time: fmt_time_seconds(r.add_time.map(i64::from)),
                product_list: None,
            })
            .collect();
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /getRecordDetail`：出入库记录详情
    /// （Java `ProductStockRecordServiceImpl#getRecordDetail`）。
    ///
    /// Java 在记录不存在或明细为空时返回 `null`；Rust 返回 404 业务错误，
    /// 语义等价（前端拿不到数据）。
    pub async fn record_detail(&self, id: i32) -> Result<ProductStockRecordVO> {
        let Some(record) = self.repo().find_stock_record(id).await? else {
            return Err(Error::biz_msg("出入库记录不存在"));
        };
        let details = self.repo().find_stock_record_details(id).await?;
        if details.is_empty() {
            return Err(Error::biz_msg("出入库记录明细不存在"));
        }

        let product_ids: Vec<i32> = details.iter().filter_map(|d| d.product_id).collect();
        let sku_ids: Vec<i32> = details.iter().filter_map(|d| d.sku_id).collect();
        let briefs = self.repo().find_product_briefs(&product_ids).await?;
        let sku_values = self.repo().find_sku_values(&sku_ids).await?;

        let product_list = details
            .iter()
            .map(|d| ProductStockRecordDetailVO {
                product_name: d.product_id.and_then(|id| briefs.get(&id)).map(|b| b.0.clone()),
                product_sku_name: d.sku_id.and_then(|id| sku_values.get(&id).cloned()),
                product_unit: d
                    .product_id
                    .and_then(|id| briefs.get(&id))
                    .and_then(|b| b.1.clone()),
                quantity: d.quantity,
                product_pic_thumb: d
                    .product_id
                    .and_then(|id| briefs.get(&id))
                    .map(|b| b.2.clone()),
            })
            .collect();

        Ok(ProductStockRecordVO {
            product_stock_id: record.product_stock_id,
            stock_code: record.stock_code,
            r#type: record.r#type,
            add_by: record.add_by,
            comment: record.comment,
            add_time: fmt_time_seconds(record.add_time.map(i64::from)),
            product_list: Some(product_list),
        })
    }

    // ————————————————————— 入库 / 出库 —————————————————————

    /// `POST /stockIn`：商品入库
    /// （Java `ProductStockRecordServiceImpl#productStockIn`）。
    ///
    /// 事务语义：逐行增减库存 → 写 `product_stock_record`（`in_or_out = 1`）
    /// → 回填并写明细。任一步失败即整体报错（Java 为 `@Transactional`）。
    pub async fn stock_in(&self, dto: &StockInDTO, op: &StockOperator) -> Result<i32> {
        self.stock_change(dto, op, record_type::IN, "入库增加库存")
            .await
    }

    /// `POST /stockOut`：商品出库
    /// （Java `ProductStockRecordServiceImpl#productStockOut`）。
    pub async fn stock_out(&self, dto: &StockInDTO, op: &StockOperator) -> Result<i32> {
        self.stock_change(dto, op, record_type::OUT, "出库扣减库存")
            .await
    }

    /// 出入库公共实现（Java `productStockInForShop` / `productStockOutForShop`）。
    async fn stock_change(
        &self,
        dto: &StockInDTO,
        op: &StockOperator,
        in_or_out: i32,
        desc: &str,
    ) -> Result<i32> {
        if dto.product_stock_record_detail_list.is_empty() {
            return Err(Error::biz_msg("请选择商品"));
        }
        for detail in &dto.product_stock_record_detail_list {
            let product_id = detail
                .product_id
                .ok_or_else(|| Error::Validation("商品ID不能为空".into()))?;
            let quantity = detail.quantity.unwrap_or(0);
            if quantity <= 0 {
                return Err(Error::biz_msg("库存变动数量必须大于0"));
            }
            let product = self
                .repo()
                .find_product(product_id)
                .await?
                .ok_or_else(|| Error::biz_msg("商品不存在或不属于目标组织"))?;
            if in_or_out == record_type::IN {
                self.increase_stock_for_shop(&product, op.shop_id, detail, desc)
                    .await?;
            } else {
                self.decrease_stock_for_shop(&product, op.shop_id, detail, desc)
                    .await?;
            }
        }

        let now = utils::now_seconds() as i32;
        let stock_code = stock_code(in_or_out, op.shop_id);
        let record = ProductStockRecord {
            product_stock_id: 0,
            stock_code: Some(stock_code),
            in_or_out,
            r#type: dto.r#type.or(Some(if in_or_out == record_type::IN {
                record_type::OTHER_IN
            } else {
                record_type::OTHER_OUT
            })),
            add_time: Some(now),
            add_by: op.username.clone(),
            comment: dto.comment.clone(),
            shop_id: Some(op.shop_id),
        };
        let record_id = self.repo().insert_stock_record(&record).await?;
        if record_id <= 0 {
            return Err(Error::biz_msg("库存记录创建失败"));
        }

        let details: Vec<ProductStockRecordDetail> = dto
            .product_stock_record_detail_list
            .iter()
            .map(|d| ProductStockRecordDetail {
                record_detail_id: 0,
                stock_record_id: Some(record_id),
                product_id: d.product_id,
                sku_id: d.sku_id,
                quantity: d.quantity,
            })
            .collect();
        if !self.repo().insert_stock_record_details(&details).await? {
            return Err(Error::biz_msg("库存记录明细创建失败"));
        }
        // Java 随后调用 `stockWarningService.evaluate(...)` 重算库存预警：
        // 该服务未迁移（见 uncertain_points），此处不伪造实现。
        tracing::info!(
            "库存{}成功: shop_id={}, record_id={record_id}, 明细 {} 行",
            if in_or_out == record_type::IN { "入库" } else { "出库" },
            op.shop_id,
            details.len()
        );
        Ok(record_id)
    }

    /// 入库落点（Java `increaseStockForShop`）。
    async fn increase_stock_for_shop(
        &self,
        product: &Product,
        shop_id: i32,
        detail: &StockRecordDetailItem,
        desc: &str,
    ) -> Result<()> {
        let quantity = detail.quantity.unwrap_or(0);
        let sku = self.require_product_sku(product, detail.sku_id).await?;
        let store_product = self.resolve_assigned_store_product(product, shop_id).await?;

        let Some(sp) = store_product else {
            if let Some(sku) = &sku {
                self.repo().inc_sku_stock(sku.sku_id, quantity).await?;
                self.write_inventory_log(
                    product.product_id,
                    Some(sku.sku_id),
                    sku.sku_stock.unwrap_or(0),
                    quantity,
                    true,
                    desc,
                    product.shop_id,
                )
                .await?;
            }
            self.repo()
                .inc_product_stock(product.product_id, quantity)
                .await?;
            self.write_inventory_log(
                product.product_id,
                None,
                product.product_stock,
                quantity,
                true,
                desc,
                product.shop_id,
            )
            .await?;
            return Ok(());
        };

        // 平台商品已分配给门店：改门店库存
        let store_sku = match &sku {
            None => None,
            Some(sku) => Some(
                self.repo()
                    .find_store_product_sku(sp.store_product_id, sku.sku_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("平台分配商品规格不存在或已失效"))?,
            ),
        };
        if let Some(store_sku) = &store_sku {
            let affected = self
                .repo()
                .inc_store_sku_stock(store_sku.store_product_sku_id, quantity)
                .await?;
            if affected == 0 {
                return Err(Error::biz_msg("门店商品规格已失效"));
            }
        }
        let affected = self
            .repo()
            .inc_store_product_stock(sp.store_product_id, quantity)
            .await?;
        if affected == 0 {
            return Err(Error::biz_msg("平台分配商品已失效"));
        }
        let old_stock = store_sku
            .as_ref()
            .and_then(|s| s.sku_stock)
            .or(sp.product_stock)
            .unwrap_or(0);
        self.write_inventory_log(
            product.product_id,
            store_sku.as_ref().map(|s| s.product_sku_id as i32),
            old_stock,
            quantity,
            true,
            desc,
            shop_id,
        )
        .await
    }

    /// 出库落点（Java `decreaseStockForShop`）。
    async fn decrease_stock_for_shop(
        &self,
        product: &Product,
        shop_id: i32,
        detail: &StockRecordDetailItem,
        desc: &str,
    ) -> Result<()> {
        let quantity = detail.quantity.unwrap_or(0);
        let sku = self.require_product_sku(product, detail.sku_id).await?;
        let store_product = self.resolve_assigned_store_product(product, shop_id).await?;

        let Some(sp) = store_product else {
            if let Some(sku) = &sku {
                let affected = self.repo().dec_sku_stock(sku.sku_id, quantity).await?;
                if affected == 0 {
                    return Err(Error::biz_msg(format!(
                        "{}库存不足",
                        product.product_name
                    )));
                }
                self.write_inventory_log(
                    product.product_id,
                    Some(sku.sku_id),
                    sku.sku_stock.unwrap_or(0),
                    quantity,
                    false,
                    desc,
                    product.shop_id,
                )
                .await?;
            }
            let affected = self
                .repo()
                .dec_product_stock(product.product_id, quantity)
                .await?;
            if affected == 0 {
                return Err(Error::biz_msg(format!(
                    "{}库存不足",
                    product.product_name
                )));
            }
            self.write_inventory_log(
                product.product_id,
                None,
                product.product_stock,
                quantity,
                false,
                desc,
                product.shop_id,
            )
            .await?;
            return Ok(());
        };

        let store_sku = match &sku {
            None => None,
            Some(sku) => Some(
                self.repo()
                    .find_store_product_sku(sp.store_product_id, sku.sku_id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("平台分配商品规格不存在或已失效"))?,
            ),
        };
        if let Some(store_sku) = &store_sku {
            let affected = self
                .repo()
                .dec_store_sku_stock(store_sku.store_product_sku_id, quantity)
                .await?;
            if affected == 0 {
                return Err(Error::biz_msg("门店商品规格库存不足"));
            }
        }
        let affected = self
            .repo()
            .dec_store_product_stock(sp.store_product_id, quantity)
            .await?;
        if affected == 0 {
            return Err(Error::biz_msg("门店商品库存不足或分配已失效"));
        }
        let old_stock = store_sku
            .as_ref()
            .and_then(|s| s.sku_stock)
            .or(sp.product_stock)
            .unwrap_or(0);
        self.write_inventory_log(
            product.product_id,
            store_sku.as_ref().map(|s| s.product_sku_id as i32),
            old_stock,
            quantity,
            false,
            desc,
            shop_id,
        )
        .await
    }

    /// 校验并取 SKU（Java `requireProductSku`：不存在或不属于该商品即报错）。
    async fn require_product_sku(
        &self,
        product: &Product,
        sku_id: Option<i32>,
    ) -> Result<Option<ProductSku>> {
        let Some(sku_id) = sku_id.filter(|v| *v > 0) else {
            return Ok(None);
        };
        let sku = self
            .repo()
            .find_sku(sku_id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品规格不存在或不属于目标商品"))?;
        if sku.product_id != product.product_id {
            return Err(Error::biz_msg("商品规格不存在或不属于目标商品"));
        }
        Ok(Some(sku))
    }

    /// 判定库存落点（Java `resolveAssignedStoreProduct`）。
    ///
    /// - 商品属于当前组织 → `Ok(None)`（直接改 `product` / `product_sku`）；
    /// - 平台商品（`shop_id = 0`）且当前组织 > 0 → 取门店分配记录，
    ///   缺失即「平台商品未分配给目标组织或分配已失效」；
    /// - 其余（门店商品被其他组织操作等）→「商品不存在或不属于目标组织」。
    async fn resolve_assigned_store_product(
        &self,
        product: &Product,
        shop_id: i32,
    ) -> Result<Option<StoreProduct>> {
        if product.shop_id == shop_id {
            return Ok(None);
        }
        if product.shop_id != 0 || shop_id <= 0 {
            return Err(Error::biz_msg("商品不存在或不属于目标组织"));
        }
        self.repo()
            .find_store_product(product.product_id, shop_id)
            .await?
            .map(Some)
            .ok_or_else(|| Error::biz_msg("平台商品未分配给目标组织或分配已失效"))
    }

    /// 写入库存日志（Java `saveStoreInventoryLog` / `ProductInventoryLog.builder()`）。
    #[allow(clippy::too_many_arguments)]
    async fn write_inventory_log(
        &self,
        product_id: i32,
        spec_id: Option<i32>,
        old_number: i32,
        quantity: i32,
        increase: bool,
        desc: &str,
        shop_id: i32,
    ) -> Result<()> {
        let log = ProductInventoryLog {
            log_id: 0,
            product_id: Some(product_id),
            spec_id: Some(spec_id.unwrap_or(0)),
            number: Some(if increase {
                old_number.saturating_add(quantity)
            } else {
                old_number.saturating_sub(quantity)
            }),
            add_time: Some(utils::now_seconds() as i32),
            old_number: Some(old_number),
            r#type: Some(if increase { 1 } else { 2 }),
            change_number: Some(quantity),
            desc: Some(desc.to_string()),
            shop_id,
        };
        self.repo().insert_inventory_log(&log).await
    }

    /// 解析调拨组织名称（平台名称取 `config.shopName`，否则按门店标题精确匹配）。
    async fn resolve_allocate_org(&self, name: &str) -> Result<Option<i32>> {
        let platform_name = self
            .repos
            .catalog
            .product_manage
            .config_val("shopName")
            .await?
            .unwrap_or_else(|| "商城自营".to_string());
        if name == platform_name {
            return Ok(Some(0));
        }
        let shops = self
            .repos
            .merchant
            .shop_manage
            .find_shops_for_picker(
                nextrade_core::models::merchant::shop_manage::shop_type::STORE,
                Some(name),
                None,
            )
            .await?;
        Ok(shops
            .into_iter()
            .find(|s| s.shop_title.as_deref() == Some(name) && s.status == Some(1))
            .map(|s| s.shop_id))
    }

    /// 调拨单导入校验（Java `checkImportData`）：返回 (出库, 入库, 备注, 明细, 错误)。
    #[allow(clippy::type_complexity)]
    pub async fn allocate_import_validate(
        &self,
        rows: &[nextrade_core::models::catalog::product_stock::AllocateBatchImportRow],
    ) -> Result<(
        Option<(i32, i32, String)>,
        Vec<nextrade_core::models::catalog::product_stock::AllocateDetailItem>,
        Vec<nextrade_core::models::catalog::product_stock::AllocateBatchImportErrorVO>,
    )> {
        use nextrade_core::models::catalog::product_stock::{
            AllocateBatchImportErrorVO, AllocateDetailItem,
        };
        let mut errors = Vec::new();
        let mut details = Vec::new();
        let mut header: Option<(i32, i32, String)> = None;
        for row in rows {
            let out_name = row.stock_out_org.as_deref().map(str::trim).unwrap_or("");
            let in_name = row.stock_in_org.as_deref().map(str::trim).unwrap_or("");
            let mut push_err = |msg: &str| {
                errors.push(AllocateBatchImportErrorVO {
                    stock_out_org: row.stock_out_org.clone(),
                    stock_in_org: row.stock_in_org.clone(),
                    comment: row.comment.clone(),
                    product_name: row.product_name.clone(),
                    product_sn: row.product_sn.clone(),
                    sku_name: row.sku_name.clone(),
                    quantity: row.quantity.clone(),
                    error_msg: msg.to_string(),
                });
            };
            let Some(out_id) = self.resolve_allocate_org(out_name).await? else {
                push_err(&format!("{out_name}门店不存在"));
                continue;
            };
            let Some(in_id) = self.resolve_allocate_org(in_name).await? else {
                push_err(&format!("{in_name}门店不存在"));
                continue;
            };
            if out_id == in_id {
                push_err("出库组织和入库组织不能相同");
                continue;
            }
            match &header {
                Some((o, i, _)) if *o != out_id || *i != in_id => {
                    push_err("出库/入库组织前后不一致");
                    continue;
                }
                Some(_) => {}
                None => header = Some((out_id, in_id, row.comment.clone().unwrap_or_default())),
            }
            let sn = row.product_sn.as_deref().map(str::trim).unwrap_or("");
            if sn.is_empty() {
                push_err("商品编码没有填写");
                continue;
            }
            let Some(product) = self.repo().find_product_by_sn(sn, None).await? else {
                push_err("商品不存在");
                continue;
            };
            if product.product_type != Some(1) {
                push_err("该商品不是实物商品不能调拨出库");
                continue;
            }
            let sku_name = row.sku_name.as_deref().map(str::trim).unwrap_or("");
            let mut sku_id = None;
            if sku_name.is_empty() {
                if self.repo().count_skus(product.product_id).await? > 0 {
                    push_err("该商品下有规格商品，请补齐商品规格");
                    continue;
                }
            } else {
                let Some(sku) = self.repo().find_sku_by_value(product.product_id, sku_name).await?
                else {
                    push_err("商品规格不存在");
                    continue;
                };
                sku_id = Some(sku.sku_id);
            }
            let quantity = row
                .quantity
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .parse::<i32>()
                .unwrap_or(0);
            details.push(AllocateDetailItem {
                allocate_detail_id: None,
                allocate_id: None,
                product_id: Some(product.product_id),
                sku_id,
                quantity: Some(quantity),
                stock_in_quantity: None,
                stock_out_quantity: None,
            });
        }
        Ok((header, details, errors))
    }

    /// 调拨单批量导入：校验通过后创建调拨单（Java `batchImportAllocate`）。
    pub async fn batch_import_allocate(
        &self,
        rows: &[nextrade_core::models::catalog::product_stock::AllocateBatchImportRow],
        op: &StockOperator,
    ) -> Result<bool> {
        use nextrade_core::models::catalog::product_stock::CreateAllocateDTO;
        let (header, details, errors) = self.allocate_import_validate(rows).await?;
        if !errors.is_empty() {
            return Err(Error::biz_msg(format!(
                "导入失败:共导入{}个,失败{}个",
                rows.len(),
                errors.len()
            )));
        }
        let Some((out_id, in_id, comment)) = header else {
            return Err(Error::biz_msg("表格数据为空"));
        };
        let dto = CreateAllocateDTO {
            outbound_organization_id: Some(out_id),
            warehousing_organization_id: Some(in_id),
            comment: Some(comment),
            product_stock_allocate_record_detail_list: details,
        };
        self.create_allocate(&dto, op).await
    }

    /// 调拨商品导入校验/预览（Java `checkAllocateImportData` + `batchImportAllocateProduct`）。
    pub async fn allocate_product_preview(
        &self,
        rows: &[nextrade_core::models::catalog::product_stock::AllocateProductImportRow],
        shop_id: i32,
        target_shop_id: i32,
    ) -> Result<(
        Vec<nextrade_core::models::catalog::product_stock::ProductImportSuccessVO>,
        Vec<nextrade_core::models::catalog::product_stock::AllocateProductImportErrorVO>,
    )> {
        use nextrade_core::models::catalog::product_stock::{
            AllocateProductImportErrorVO, ProductImportSuccessVO,
        };
        let mut success = Vec::new();
        let mut errors = Vec::new();
        for row in rows {
            let sn = row.product_sn.as_deref().map(str::trim).unwrap_or("");
            let sku_name = row.product_sku_name.as_deref().map(str::trim).unwrap_or("");
            let mut push_err = |msg: &str| {
                errors.push(AllocateProductImportErrorVO {
                    product_name: row.product_name.clone(),
                    product_sn: row.product_sn.clone(),
                    product_sku_name: row.product_sku_name.clone(),
                    quantity: row.quantity.clone(),
                    error_msg: msg.to_string(),
                });
            };
            if sn.is_empty() {
                push_err("商品编码不能为空");
                continue;
            }
            let Some(product) = self.repo().find_product_by_sn(sn, None).await? else {
                push_err("商品不存在");
                continue;
            };
            if product.product_type != Some(1) {
                push_err("只能调拨普通商品（实物商品）");
                continue;
            }
            let mut source_store = None;
            let mut target_store = None;
            if shop_id == 0 && target_shop_id > 0 {
                if product.shop_id != 0 {
                    push_err("该商品不是平台商品，无法从平台调拨");
                    continue;
                }
                target_store = self
                    .repo()
                    .find_store_product(product.product_id, target_shop_id)
                    .await?;
                if target_store.is_none() {
                    push_err("该商品未分配给目标门店");
                    continue;
                }
            } else if shop_id > 0 && target_shop_id == 0 {
                source_store = self
                    .repo()
                    .find_store_product(product.product_id, shop_id)
                    .await?;
                if source_store.is_none() {
                    push_err("出库门店不存在该商品");
                    continue;
                }
            } else if shop_id > 0 && target_shop_id > 0 {
                if shop_id == target_shop_id {
                    push_err("出库门店与入库门店不能相同");
                    continue;
                }
                source_store = self
                    .repo()
                    .find_store_product(product.product_id, shop_id)
                    .await?;
                if source_store.is_none() {
                    push_err("出库门店不存在该商品");
                    continue;
                }
                target_store = self
                    .repo()
                    .find_store_product(product.product_id, target_shop_id)
                    .await?;
                if target_store.is_none() {
                    push_err("入库门店不存在该商品");
                    continue;
                }
            }
            let mut sku = None;
            if sku_name.is_empty() {
                if self.repo().count_skus(product.product_id).await? > 0 {
                    push_err("该商品下有规格商品，请补齐商品规格");
                    continue;
                }
                if let Some(store) = &source_store {
                    if self.repo().count_store_skus(store.store_product_id).await? > 0 {
                        push_err("该门店商品下有规格商品，请补齐商品规格");
                        continue;
                    }
                }
            } else {
                let Some(found) = self
                    .repo()
                    .find_sku_by_value(product.product_id, sku_name)
                    .await?
                else {
                    push_err("商品规格不存在");
                    continue;
                };
                if let Some(store) = &source_store {
                    if self
                        .repo()
                        .find_store_product_sku(store.store_product_id, found.sku_id)
                        .await?
                        .is_none()
                    {
                        push_err("出库门店商品规格不存在");
                        continue;
                    }
                }
                sku = Some(found);
            }
            let quantity = row
                .quantity
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .parse::<i32>()
                .unwrap_or(0);
            let current_stock = if let Some(store) = &source_store {
                if let Some(sku) = &sku {
                    self.repo()
                        .find_store_product_sku(store.store_product_id, sku.sku_id)
                        .await?
                        .and_then(|s| s.sku_stock)
                        .unwrap_or_else(|| store.product_stock.unwrap_or(0))
                } else {
                    store.product_stock.unwrap_or(0)
                }
            } else {
                sku.as_ref()
                    .and_then(|s| s.sku_stock)
                    .unwrap_or(product.product_stock)
            };
            success.push(ProductImportSuccessVO {
                product_id: product.product_id,
                product_name: product.product_name.clone(),
                product_unit: product.product_unit.clone(),
                product_pic_thumb: Some(product.pic_thumb.clone()),
                product_sku_id: sku.as_ref().map(|s| s.sku_id),
                product_sku_name: sku.as_ref().map(|s| s.sku_value.clone()),
                quantity,
                current_product_stock: current_stock,
            });
        }
        Ok((success, errors))
    }

    /// 商品出入库导入预览（Java `ProductStockRecordServiceImpl#batchImport`）。
    ///
    /// 只做校验与预览，不直接改库存；有错误时由路由层返回「导入失败:...」。
    pub async fn import_preview(
        &self,
        rows: &[nextrade_core::models::catalog::product_stock::ProductStockImportRow],
        shop_id: i32,
    ) -> Result<(
        Vec<nextrade_core::models::catalog::product_stock::ProductImportSuccessVO>,
        Vec<nextrade_core::models::catalog::product_stock::ProductImportErrorVO>,
    )> {
        use nextrade_core::models::catalog::product_stock::{
            ProductImportErrorVO, ProductImportSuccessVO,
        };
        let mut success = Vec::new();
        let mut errors = Vec::new();
        for row in rows {
            let sn = row.product_sn.as_deref().map(str::trim).unwrap_or("");
            let sku_name = row.product_sku_name.as_deref().map(str::trim).unwrap_or("");
            let mut push_err = |msg: &str| {
                errors.push(ProductImportErrorVO {
                    product_name: row.product_name.clone(),
                    product_sn: row.product_sn.clone(),
                    product_sku_name: row.product_sku_name.clone(),
                    quantity: row.quantity.clone(),
                    comment: row.comment.clone(),
                    error_msg: msg.to_string(),
                });
            };
            if sn.is_empty() {
                push_err("商品编码没有填写");
                continue;
            }
            let Some(product) = self
                .repo()
                .find_product_by_sn(sn, Some(shop_id))
                .await?
            else {
                push_err("商品不存在");
                continue;
            };
            if product.product_type != Some(1) {
                push_err("该商品不是实物商品不能出入库");
                continue;
            }
            let mut sku_id = None;
            let mut sku_stock = None;
            let mut sku_value = None;
            if sku_name.is_empty() {
                if self.repo().count_skus(product.product_id).await? > 0 {
                    push_err("该商品下面有规格商品，请补齐商品规格");
                    continue;
                }
            } else {
                let Some(sku) = self.repo().find_sku_by_value(product.product_id, sku_name).await?
                else {
                    push_err("商品规格不存在");
                    continue;
                };
                sku_id = Some(sku.sku_id);
                sku_stock = sku.sku_stock;
                sku_value = Some(sku.sku_value);
            }
            let quantity = row
                .quantity
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .parse::<i32>()
                .unwrap_or(0);
            let current_stock = if shop_id > 0 {
                if let Some(store) = self
                    .repo()
                    .find_store_product(product.product_id, shop_id)
                    .await?
                {
                    if let Some(sid) = sku_id {
                        self.repo()
                            .find_store_product_sku(store.store_product_id, sid)
                            .await?
                            .and_then(|s| s.sku_stock)
                            .unwrap_or_else(|| store.product_stock.unwrap_or(0))
                    } else {
                        store.product_stock.unwrap_or(0)
                    }
                } else {
                    sku_stock.unwrap_or(product.product_stock)
                }
            } else {
                sku_stock.unwrap_or(product.product_stock)
            };
            success.push(ProductImportSuccessVO {
                product_id: product.product_id,
                product_name: product.product_name.clone(),
                product_unit: product.product_unit.clone(),
                product_pic_thumb: Some(product.pic_thumb.clone()),
                product_sku_id: sku_id,
                product_sku_name: sku_value,
                quantity,
                current_product_stock: current_stock,
            });
        }
        Ok((success, errors))
    }

    // ————————————————————— 库存调整 —————————————————————

    /// `POST /stockAdjust`：库存调整
    /// （Java `ProductStockAdjustRecordServiceImpl#stockAdjust`）。
    pub async fn stock_adjust(&self, dto: &StockAdjustDTO, op: &StockOperator) -> Result<bool> {
        let adjust_type = dto.adjust_type.unwrap_or(1);
        let quantity = dto.quantity.unwrap_or(0);
        if adjust_type != 0 && adjust_type != 1 {
            return Err(Error::Validation("调整类型错误".into()));
        }
        if adjust_type == 0 && quantity < 0 {
            return Err(Error::biz_msg("覆盖调整调整数量不能小于0"));
        }
        let product_id = dto
            .product_id
            .ok_or_else(|| Error::Validation("商品ID不能为空".into()))?;
        let product = self
            .repo()
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::biz_msg("商品不存在"))?;

        let sku_id = dto.sku_id.filter(|v| *v > 0);
        let sku = match sku_id {
            None => None,
            Some(id) => Some(
                self.repo()
                    .find_sku(id)
                    .await?
                    .ok_or_else(|| Error::biz_msg("商品规格不存在"))?,
            ),
        };

        // 调整前库存（Java：有 sku 取 sku_stock，否则取 product_stock）
        let change_before = sku
            .as_ref()
            .map_or(product.product_stock, |s| s.sku_stock.unwrap_or(0));

        let effect = resolve_adjust_effect(
            adjust_type,
            quantity,
            change_before,
            product.product_stock,
            sku.as_ref().map(|s| s.sku_stock.unwrap_or(0)),
        );

        // 按 SKU 落点调整
        if let (Some(sku), Some(delta)) = (&sku, effect.sku_delta) {
            if delta >= 0 {
                self.repo().inc_sku_stock(sku.sku_id, delta).await?;
                self.write_inventory_log(
                    product_id,
                    Some(sku.sku_id),
                    change_before,
                    delta,
                    true,
                    "调整增加库存",
                    product.shop_id,
                )
                .await?;
            } else {
                let amount = delta.saturating_abs();
                let affected = self.repo().dec_sku_stock(sku.sku_id, amount).await?;
                if affected == 0 {
                    return Err(Error::biz_msg(format!(
                        "{}库存不足",
                        product.product_name
                    )));
                }
                self.write_inventory_log(
                    product_id,
                    Some(sku.sku_id),
                    change_before,
                    amount,
                    false,
                    "调整减少库存",
                    product.shop_id,
                )
                .await?;
            }
        }

        // 按商品落点调整
        if let Some(delta) = effect.product_delta {
            if delta >= 0 {
                self.repo().inc_product_stock(product_id, delta).await?;
                self.write_inventory_log(
                    product_id,
                    None,
                    product.product_stock,
                    delta,
                    true,
                    "调整增加库存",
                    product.shop_id,
                )
                .await?;
            } else {
                let amount = delta.saturating_abs();
                let affected = self.repo().dec_product_stock(product_id, amount).await?;
                if affected == 0 {
                    return Err(Error::biz_msg(format!(
                        "{}库存不足",
                        product.product_name
                    )));
                }
                self.write_inventory_log(
                    product_id,
                    None,
                    product.product_stock,
                    amount,
                    false,
                    "调整减少库存",
                    product.shop_id,
                )
                .await?;
            }
        }

        // 调整后库存（Java 重新查库）
        let after_product = self
            .repo()
            .find_product(product_id)
            .await?
            .map_or(product.product_stock, |p| p.product_stock);
        let change_after = match sku_id {
            None => after_product,
            Some(id) => self
                .repo()
                .find_sku(id)
                .await?
                .and_then(|s| s.sku_stock)
                .unwrap_or(0),
        };

        let record = ProductStockAdjustRecord {
            adjust_record_id: 0,
            sku_id,
            adjust_type: Some(adjust_type),
            quantity: Some(quantity),
            add_by: op.username.clone(),
            add_time: Some(utils::now_seconds()),
            product_id: Some(product_id),
            // Java：`HeaderUtils.getShopId() == null ? 0 : getShopId()`
            shop_id: Some(op.shop_id),
            change_before_quantity: Some(change_before),
            change_after_quantity: Some(change_after),
        };
        self.repo().insert_adjust_record(&record).await?;
        // Java 随后调用 `stockWarningService.evaluate(...)`（未迁移）
        Ok(true)
    }

    /// `GET /adjustDetail`：商品调整记录
    /// （Java `ProductStockAdjustRecordServiceImpl#adjustDetail`）。
    pub async fn adjust_detail(
        &self,
        q: &AdjustDetailQuery,
        op: &StockOperator,
    ) -> Result<Vec<ProductStockAdjustRecordDetailVO>> {
        let rows = self
            .repo()
            .find_adjust_records(q.product_id, q.sku_id, op.shop_id)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| ProductStockAdjustRecordDetailVO {
                adjust_type: r.adjust_type,
                quantity: r.quantity,
                add_by: r.add_by,
                add_time: fmt_time_seconds(r.add_time),
                change_before_quantity: r.change_before_quantity,
                change_after_quantity: r.change_after_quantity,
            })
            .collect())
    }

    // ————————————————————— 调拨选品 / 调拨单 —————————————————————

    /// `GET /getAllocateProductStockPage`：可调拨商品列表
    /// （Java `ProductStockController.getAllocateProductStockPage`）。
    ///
    /// 三个场景：平台→门店、门店→平台、门店→门店；两组织均必填。
    pub async fn allocate_product_stock_page(
        &self,
        q: &StockPageQuery,
        op: &StockOperator,
    ) -> Result<PageResult<ProductStockPageVO>> {
        let shop_id = q
            .shop_id
            .or(Some(op.shop_id))
            .filter(|v| *v >= 0)
            .unwrap_or(op.shop_id);
        let target = q
            .target_shop_id
            .ok_or_else(|| Error::biz_msg("请选择入库组织"))?;

        // Java：`Assert.isTrue(shopId != null, "请选择出库组织")` —— 请求头已提供组织，
        // 显式传 null 的场景在 Rust 中不存在；此处仍保留入库组织校验。
        let page = q.page_query();
        let keyword = q.keyword.as_deref();
        let (limit, offset) = (page.page_size, (page.page - 1) * page.page_size);

        let repo = self.repo();
        let (total, rows) = match (shop_id, target) {
            (0, t) if t > 0 => {
                repo.paginate_allocate_platform_to_store(t, keyword, limit, offset)
                    .await?
            }
            (s, 0) if s > 0 => {
                repo.paginate_allocate_store_to_platform(s, keyword, limit, offset)
                    .await?
            }
            (s, t) if s > 0 && t > 0 => {
                repo.paginate_allocate_store_to_store(s, t, keyword, limit, offset)
                    .await?
            }
            // Java 默认返回空页
            _ => (0, Vec::new()),
        };

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            list.push(self.allocate_page_vo(row).await?);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 调拨选品行 → `ProductStockPageVO`（Java `translateAllocateStockPage` + Mapper 字段）。
    async fn allocate_page_vo(&self, row: nextrade_core::models::catalog::product_stock::StockPageRow) -> Result<ProductStockPageVO> {
        let mut vo = ProductStockPageVO {
            product_id: row.product_id,
            product_name: row.product_name.clone(),
            product_sku_name: row.product_sku_name.clone(),
            product_sku_id: row.sku_id,
            product_unit: row.product_unit.clone(),
            // 场景1/2：SKU 存在取 SKU 库存，否则商品库存；场景3 由 SQL 直接算出
            current_product_stock: Some(if row.sku_id.is_some() {
                row.product_sku_stock.unwrap_or(0)
            } else {
                row.product_stock
            }),
            product_cost: row.product_cost,
            product_pic_thumb: row.product_pic_thumb.clone(),
            product_sn: row.product_sn.clone(),
            product_tsn: row.product_tsn.clone(),
        };
        if row.product_type == Some(product_type::CARD) {
            vo.current_product_stock = Some(self.card_stock(row.card_group_id).await?);
        } else if product_type::is_reservation(row.product_type) {
            vo.current_product_stock = None;
        }
        Ok(vo)
    }

    /// `GET /productStockAllocateRecordPage`：库存调拨记录
    /// （Java `ProductStockAllocateRecordServiceImpl#getProductStockAllocateRecordPage`）。
    ///
    /// Java 仅在 `nextradeProperties.isO2o != 0` 时返回数据；
    /// Rust 尚无该配置项，按「O2O 开启」处理（见 uncertain_points）。
    pub async fn allocate_record_page(
        &self,
        q: &AllocatePageQuery,
        op: &StockOperator,
    ) -> Result<PageResult<ProductStockAllocateRecordPageVO>> {
        let page = q.page_query();
        let (total, rows) = self.repo().paginate_allocate_records(q, op.shop_id).await?;

        // 批量取组织名（避免 N+1）
        let org_ids: Vec<i32> = rows
            .iter()
            .flat_map(|r| {
                [
                    r.outbound_organization_id.unwrap_or(0),
                    r.warehousing_organization_id.unwrap_or(0),
                ]
            })
            .filter(|v| *v != 0)
            .collect();
        let titles = self.repo().find_shop_titles(&org_ids).await?;

        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            let outbound = r.outbound_organization_id.unwrap_or(0);
            let warehousing = r.warehousing_organization_id.unwrap_or(0);
            // Java：当前组织是入库方，且状态为待审核 → 可审核
            let can_audit =
                warehousing == op.shop_id && r.status == Some(allocate_status::AUDIT_WAIT);
            list.push(ProductStockAllocateRecordPageVO {
                allocate_id: r.allocate_id,
                allocate_code: r.allocate_code,
                outbound_organization_id: r.outbound_organization_id,
                warehousing_organization_id: r.warehousing_organization_id,
                status: r.status,
                add_by: r.add_by,
                add_time: fmt_time_seconds(r.add_time),
                comment: r.stock_allocate_comment,
                stock_in_comment: r.stock_in_comment,
                stock_out_comment: r.stock_out_comment,
                outbound_organization_name: org_name(outbound, &titles),
                warehousing_organization_name: org_name(warehousing, &titles),
                is_can_audit: can_audit,
            });
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// `GET /productStockAllocateRecordDetail`：调拨记录详情
    /// （Java `productStockAllocateRecordDetail`）。
    pub async fn allocate_record_detail(
        &self,
        id: i32,
    ) -> Result<ProductStockAllocateRecordDetailVO> {
        let details = self.repo().find_allocate_details_with_product(id).await?;
        if details.is_empty() {
            return Err(Error::biz_msg("调拨单明细不存在"));
        }
        Ok(ProductStockAllocateRecordDetailVO {
            product_allocate_detail_vo_list: details,
        })
    }

    /// `POST /createAllocate`：新建调拨单
    /// （Java `ProductStockAllocateRecordServiceImpl#createAllocate`）。
    ///
    /// 状态：平台建单（`shop_id` 为空或 0）→ 直接「待调出」；门店建单 → 「待审核」。
    /// 仅「待调出」时写「提交审核」操作记录（Java 原逻辑如此）。
    pub async fn create_allocate(
        &self,
        dto: &CreateAllocateDTO,
        op: &StockOperator,
    ) -> Result<bool> {
        let outbound = dto
            .outbound_organization_id
            .ok_or_else(|| Error::biz_msg("请选择出库组织"))?;
        let warehousing = dto
            .warehousing_organization_id
            .ok_or_else(|| Error::biz_msg("请选择入库组织"))?;
        if outbound == warehousing {
            return Err(Error::biz_msg("出库组织和入库组织不能相同"));
        }
        if dto.product_stock_allocate_record_detail_list.is_empty() {
            return Err(Error::biz_msg("请选择商品"));
        }

        let status = if op.shop_id == 0 {
            allocate_status::STOCK_OUT_WAIT
        } else {
            allocate_status::AUDIT_WAIT
        };
        let record = ProductStockAllocateRecord {
            allocate_id: 0,
            allocate_code: Some(utils::now_millis().to_string()),
            outbound_organization_id: Some(outbound),
            warehousing_organization_id: Some(warehousing),
            status: Some(status),
            add_by: op.username.clone(),
            add_time: Some(utils::now_seconds()),
            stock_in_comment: None,
            stock_out_comment: None,
            stock_allocate_comment: dto.comment.clone(),
        };
        let allocate_id = self.repo().insert_allocate_record(&record).await?;

        if status == allocate_status::STOCK_OUT_WAIT {
            self.repo()
                .insert_allocate_operation(
                    allocate_id,
                    allocate_operation::ALLOCATE_SUBMIT,
                    None,
                    op.username_str(),
                )
                .await?;
        }

        let saved = self
            .repo()
            .insert_allocate_details(allocate_id, &dto.product_stock_allocate_record_detail_list)
            .await?;
        Ok(saved)
    }

    /// `GET /getOrgList`：组织集合（Java `getOrgList`）。
    ///
    /// 首项为平台（`shop_id = 0`）；Java 取配置项 `SHOP_NAME` 作为平台名，
    /// 配置服务未迁移，此处平台名返回空串。
    pub async fn get_org_list(&self) -> Result<Vec<AllocateOrgVO>> {
        let mut out = vec![AllocateOrgVO {
            shop_id: 0,
            shop_name: String::new(),
        }];
        for (shop_id, title) in self.repo().find_allocate_orgs().await? {
            out.push(AllocateOrgVO {
                shop_id,
                shop_name: title.unwrap_or_default(),
            });
        }
        Ok(out)
    }

    /// `GET /getAddByList`：经办人集合（Java `getAddBy`）。
    pub async fn get_add_by_list(&self) -> Result<Vec<String>> {
        self.repo().find_distinct_add_by().await
    }

    /// `POST /auditAllocate`：调拨审核（Java `auditAllocate`）。
    ///
    /// 前置校验：调拨单存在、当前组织 = 入库组织、状态 = 待审核、
    /// `auditType` ∈ {2 待调出, 5 已驳回}。
    pub async fn audit_allocate(
        &self,
        q: &AuditAllocateQuery,
        op: &StockOperator,
    ) -> Result<bool> {
        let record = self
            .repo()
            .find_allocate_record(q.id)
            .await?
            .ok_or_else(|| Error::biz_msg("调拨单不存在"))?;
        let is_warehouse = record.warehousing_organization_id == Some(op.shop_id);
        if !is_warehouse || record.status != Some(allocate_status::AUDIT_WAIT) {
            return Err(Error::biz_msg("无权限审核此调拨单"));
        }
        if q.audit_type != allocate_status::STOCK_OUT_WAIT
            && q.audit_type != allocate_status::AUDIT_REJECT
        {
            return Err(Error::biz_msg("审核类型错误"));
        }

        let (operation_type, reason) = if q.audit_type == allocate_status::AUDIT_REJECT {
            (
                allocate_operation::AUDIT_REJECT,
                q.remark.as_deref().filter(|s| !s.is_empty()),
            )
        } else {
            (allocate_operation::AUDIT_PASS, None)
        };
        // Java 先写操作记录再 updateById
        self.repo()
            .insert_allocate_operation(q.id, operation_type, reason, op.username_str())
            .await?;
        let affected = self
            .repo()
            .update_allocate_status(q.id, q.audit_type, None, None)
            .await?;
        Ok(affected > 0)
    }

    /// `POST /cancelAllocate`：调拨作废（Java `cancelAllocate`）。
    ///
    /// 可作废条件（Java）：`status <= 2（待调出）` 或 `status == 5（已驳回）`。
    pub async fn cancel_allocate(
        &self,
        allocate_ids: &[i32],
        op: &StockOperator,
    ) -> Result<bool> {
        if allocate_ids.is_empty() {
            return Err(Error::biz_msg("请选择要取消的调拨单"));
        }
        for id in allocate_ids {
            let record = self
                .repo()
                .find_allocate_record(*id)
                .await?
                .ok_or_else(|| Error::biz_msg("调拨单不存在"))?;
            let status = record.status.unwrap_or(0);
            if !allocate_status::can_cancel(status) {
                return Err(Error::biz_msg("该调拨单不能作废"));
            }
            self.repo()
                .insert_allocate_operation(
                    *id,
                    allocate_operation::ALLOCATE_CANCEL,
                    None,
                    op.username_str(),
                )
                .await?;
            self.repo()
                .update_allocate_status(*id, allocate_status::CANCEL, None, None)
                .await?;
        }
        Ok(true)
    }

    /// `POST /stockOutAllocate`：调拨出库
    /// （Java `ProductStockAllocateRecordServiceImpl#stockOutAllocate`）。
    ///
    /// 状态 2 → 3，写「调拨出库」操作记录；仅平台（`shop_id = 0`）扣减平台库存；
    /// 随后生成一条出库记录（Java 的 `stock_code` 前缀用的是「入库」码，属原实现
    /// 既有行为，此处保持一致，见 uncertain_points）。
    pub async fn stock_out_allocate(
        &self,
        dto: &AllocateStockOutDTO,
        op: &StockOperator,
    ) -> Result<bool> {
        let allocate_id = dto
            .allocate_id
            .ok_or_else(|| Error::Validation("调拨单ID不能为空".into()))?;
        let record = self
            .repo()
            .find_allocate_record(allocate_id)
            .await?
            .ok_or_else(|| Error::biz_msg("调拨单不存在"))?;
        if record.outbound_organization_id != Some(op.shop_id) {
            return Err(Error::biz_msg("无权限操作此调拨单"));
        }
        if record.status != Some(allocate_status::STOCK_OUT_WAIT) {
            return Err(Error::biz_msg("调拨单不是待调出状态"));
        }

        self.repo()
            .update_allocate_status(
                allocate_id,
                allocate_status::STOCK_IN_WAIT,
                dto.comment.as_deref(),
                None,
            )
            .await?;
        self.repo()
            .insert_allocate_operation(
                allocate_id,
                allocate_operation::OUT_SUBMIT,
                None,
                op.username_str(),
            )
            .await?;

        for detail in &dto.stock_allocate_record_details {
            self.repo().update_allocate_detail_quantities(detail).await?;

            // 平台才可以调拨所有自己的商品（Java `if (shopId == 0)`）
            if op.shop_id == 0 {
                if let Some(product_id) = detail.product_id {
                    let quantity = detail.stock_out_quantity.unwrap_or(0);
                    let has_sku = detail.sku_id.is_some_and(|v| v > 0);
                    if quantity > 0 {
                        let affected = self.repo().dec_product_stock(product_id, quantity).await?;
                        if affected == 0 {
                            return Err(Error::biz_msg("库存不足"));
                        }
                        if has_sku {
                            if let Some(sku_id) = detail.sku_id {
                                let affected = self.repo().dec_sku_stock(sku_id, quantity).await?;
                                if affected == 0 {
                                    return Err(Error::biz_msg("库存不足"));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 出库记录（`in_or_out = 2`，`type = 24 调拨出库`）
        let record_id = self
            .repo()
            .insert_stock_record(&ProductStockRecord {
                product_stock_id: 0,
                // Java：`ProductStockRecordTypeEnum.IN.getCode() + ...`（原实现既有行为）
                stock_code: Some(stock_code(record_type::IN, op.shop_id)),
                in_or_out: record_type::OUT,
                r#type: Some(record_type::ALLOCATE_OUT),
                add_time: Some(utils::now_seconds() as i32),
                add_by: op.username.clone(),
                comment: dto.comment.clone(),
                shop_id: Some(op.shop_id),
            })
            .await?;

        let details: Vec<ProductStockRecordDetail> = dto
            .stock_allocate_record_details
            .iter()
            .map(|d| ProductStockRecordDetail {
                record_detail_id: 0,
                stock_record_id: Some(record_id),
                product_id: d.product_id,
                sku_id: d.sku_id,
                quantity: d.stock_out_quantity,
            })
            .collect();
        self.repo().insert_stock_record_details(&details).await
    }

    /// `POST /stockInAllocate`：调拨入库
    /// （Java `ProductStockAllocateRecordServiceImpl#stockInAllocate`）。
    ///
    /// 状态 3 → 4（已完成），写「调拨入库」操作记录；仅平台增加平台库存。
    pub async fn stock_in_allocate(
        &self,
        dto: &AllocateStockOutDTO,
        op: &StockOperator,
    ) -> Result<bool> {
        let allocate_id = dto
            .allocate_id
            .ok_or_else(|| Error::Validation("调拨单ID不能为空".into()))?;
        let record = self
            .repo()
            .find_allocate_record(allocate_id)
            .await?
            .ok_or_else(|| Error::biz_msg("调拨单不存在"))?;
        if record.warehousing_organization_id != Some(op.shop_id) {
            return Err(Error::biz_msg("无权限操作此调拨单"));
        }
        if record.status != Some(allocate_status::STOCK_IN_WAIT) {
            return Err(Error::biz_msg("调拨单不是待调入状态"));
        }

        self.repo()
            .update_allocate_status(
                allocate_id,
                allocate_status::COMPLETE,
                None,
                dto.comment.as_deref(),
            )
            .await?;
        self.repo()
            .insert_allocate_operation(
                allocate_id,
                allocate_operation::IN_SUBMIT,
                None,
                op.username_str(),
            )
            .await?;

        for detail in &dto.stock_allocate_record_details {
            self.repo().update_allocate_detail_quantities(detail).await?;
            if op.shop_id == 0 {
                if let Some(product_id) = detail.product_id {
                    let quantity = detail.stock_in_quantity.unwrap_or(0);
                    let has_sku = detail.sku_id.is_some_and(|v| v > 0);
                    if quantity > 0 {
                        self.repo().inc_product_stock(product_id, quantity).await?;
                        if has_sku {
                            if let Some(sku_id) = detail.sku_id {
                                self.repo().inc_sku_stock(sku_id, quantity).await?;
                            }
                        }
                    }
                }
            }
        }

        let record_id = self
            .repo()
            .insert_stock_record(&ProductStockRecord {
                product_stock_id: 0,
                // Java：`ProductStockRecordTypeEnum.OUT.getCode() + ...`（原实现既有行为）
                stock_code: Some(stock_code(record_type::OUT, op.shop_id)),
                in_or_out: record_type::IN,
                r#type: Some(record_type::ALLOCATE_IN),
                add_time: Some(utils::now_seconds() as i32),
                add_by: op.username.clone(),
                comment: dto.comment.clone(),
                shop_id: Some(op.shop_id),
            })
            .await?;

        let details: Vec<ProductStockRecordDetail> = dto
            .stock_allocate_record_details
            .iter()
            .map(|d| ProductStockRecordDetail {
                record_detail_id: 0,
                stock_record_id: Some(record_id),
                product_id: d.product_id,
                sku_id: d.sku_id,
                quantity: d.stock_in_quantity,
            })
            .collect();
        self.repo().insert_stock_record_details(&details).await
    }

    /// `GET /productStockAllocateOperationDetail`：调拨操作记录
    /// （Java `productStockAllocateOperationRecordService.productStockAllocateOperationDetail`）。
    pub async fn allocate_operation_detail(
        &self,
        allocate_id: i32,
    ) -> Result<Vec<ProductStockAllocateOperationRecordVO>> {
        let rows = self.repo().find_allocate_operations(allocate_id).await?;
        Ok(rows
            .into_iter()
            .map(|r| ProductStockAllocateOperationRecordVO {
                operation_type: r.operation_type,
                add_time: fmt_time_seconds(r.add_time),
                add_by: r.add_by,
                review_rejected_reason: r.review_rejected_reason,
            })
            .collect())
    }

    /// `POST /exportAllocate`：调拨单导出
    /// （Java `ProductStockAllocateRecordServiceImpl#exportAllocate`）。
    ///
    /// Java 走 `@ResponseExcel` 直接写二进制流；Rust 侧 Excel 未迁移，
    /// 返回明确业务错误（见 [`EXCEL_UNMIGRATED_MSG`]），不伪造实现。
    pub async fn export_allocate(&self, dto: &nextrade_core::models::catalog::product_stock::AllocateExcelExportDTO) -> Result<()> {
        let _ = dto;
        Err(Error::biz_msg(EXCEL_UNMIGRATED_MSG))
    }

    /// 调拨单导出为 xlsx（Java `exportAllocate`）。
    pub async fn export_allocate_xlsx(
        &self,
        dto: &nextrade_core::models::catalog::product_stock::AllocateExcelExportDTO,
        shop_id: i32,
    ) -> Result<(String, Vec<u8>)> {
        use nextrade_core::models::catalog::product_stock::allocate_status;
        let rows = self
            .repo()
            .find_allocate_records_for_export(shop_id, dto.allocate_ids.as_deref())
            .await?;
        if rows.is_empty() {
            return Ok(("调拨单.xlsx".to_string(), Vec::new()));
        }
        let mut org_ids = Vec::new();
        for r in &rows {
            for id in [r.outbound_organization_id, r.warehousing_organization_id]
                .into_iter()
                .flatten()
                .filter(|v| *v > 0)
            {
                org_ids.push(id);
            }
        }
        let mut titles = self.repo().find_shop_titles(&org_ids).await?;
        let platform_name = self
            .repos
            .catalog
            .product_manage
            .config_val("shopName")
            .await?
            .unwrap_or_else(|| "商城自营".to_string());
        titles.insert(0, Some(platform_name));
        let headers = ["调拨单号", "出库组织", "入库组织", "状态", "经办人", "创建时间"];
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|r| {
                vec![
                    r.allocate_code.clone().unwrap_or_default(),
                    titles
                        .get(&r.outbound_organization_id.unwrap_or(0))
                        .cloned()
                        .flatten()
                        .unwrap_or_default(),
                    titles
                        .get(&r.warehousing_organization_id.unwrap_or(0))
                        .cloned()
                        .flatten()
                        .unwrap_or_default(),
                    allocate_status::name_of(r.status.unwrap_or(0))
                        .unwrap_or("")
                        .to_string(),
                    r.add_by.clone().unwrap_or_default(),
                    format_epoch(r.add_time.unwrap_or(0)),
                ]
            })
            .collect();
        let bytes = crate::system::excel::build_xlsx(&headers, &data, "调拨单")
            .map_err(|e| Error::Internal(format!("调拨单 Excel 生成失败: {e}")))?;
        Ok(("调拨单.xlsx".to_string(), bytes))
    }
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（供导出用）。
fn format_epoch(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    use chrono::TimeZone as _;
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// 库存调整的纯计算部分（Java `stockAdjust` 的算术核心）。
///
/// 返回商品级 / SKU 级库存的**变化量**（正增负减），`None` 表示该维度不调整。
/// 语义逐条对齐 Java：
/// - `adjustType = 1`（增量）：SKU 与商品均按 `quantity` 的符号增减
///   （Java 对负数取绝对值走 `decStock`，对正数走 `incStock`）；
/// - `adjustType = 0`（覆盖，目标值 = `quantity`）：
///   - 有 SKU：SKU 变化量 = `quantity - 当前 SKU 库存`；
///     商品级同样按该差值反向调整（Java
///     `productSku.getSkuStock() - quantity` 的加减分支）；
///   - 无 SKU：商品级变化量 = `quantity - 当前商品库存`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustEffect {
    /// 商品级库存变化量（正增负减）
    pub product_delta: Option<i32>,
    /// SKU 级库存变化量（正增负减）
    pub sku_delta: Option<i32>,
}

/// 计算库存调整的增减量。
#[must_use]
pub fn resolve_adjust_effect(
    adjust_type: i32,
    quantity: i32,
    change_before: i32,
    product_stock: i32,
    sku_stock: Option<i32>,
) -> AdjustEffect {
    if adjust_type == 1 {
        // 增量：SKU 与商品都按 quantity 增减
        return AdjustEffect {
            product_delta: Some(quantity),
            sku_delta: sku_stock.map(|_| quantity),
        };
    }

    // 覆盖调整
    let _ = change_before;
    match sku_stock {
        None => AdjustEffect {
            product_delta: Some(quantity.saturating_sub(product_stock)),
            sku_delta: None,
        },
        Some(current_sku_stock) => {
            let delta = quantity.saturating_sub(current_sku_stock);
            AdjustEffect {
                product_delta: Some(delta),
                sku_delta: Some(delta),
            }
        }
    }
}

/// 出入库唯一编码：出入库类型 1 位 + 店铺 ID 3 位 + 13 位时间戳 + 2 位随机数
/// （Java `String.format("%03d", shopId) + System.currentTimeMillis() + randomNumStr`）。
///
/// Java 的 2 位随机数由 `new Random().nextInt(90) + 10` 生成；Rust 侧未引入
/// 随机依赖，改用「毫秒时间戳低 2 位」派生，保证同一毫秒内不同组织/类型不冲突
/// （同毫秒同组织的并发由数据库主键与业务幂等约束兜底）。
#[must_use]
pub fn stock_code(in_or_out: i32, shop_id: i32) -> String {
    let millis = utils::now_millis();
    let suffix = millis.rem_euclid(90) + 10;
    format!("{in_or_out}{:03}{millis}{suffix:02}", shop_id.abs())
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `TigUtils.handelTime`：0/负值返回空串）。
#[must_use]
pub fn fmt_time_seconds(secs: Option<i64>) -> String {
    let Some(secs) = secs.filter(|v| *v > 0) else {
        return String::new();
    };
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 毫秒时间戳 → `yyyy-MM-dd HH:mm:ss`。
///
/// 历史数据中 `product_stock_adjust_record.add_time` 既有 Java
/// `System.currentTimeMillis() / 1000`（秒）写入，也存在毫秒落库的情形，
/// 故按数值量级判定单位（> 99999999999 视为毫秒）。
#[must_use]
pub fn fmt_time_millis(millis: i64) -> String {
    if millis <= 0 {
        return String::new();
    }
    let (secs, sub) = if millis > 99_999_999_999 {
        (millis / 1000, (millis % 1000) as u32)
    } else {
        (millis, 0)
    };
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(secs, sub * 1_000_000) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// `yyyy-MM-dd` → 当日 00:00:00 秒级时间戳（Java `TigUtils.toTimestampYmd`）。
fn day_start_seconds(date: &str) -> Result<i64> {
    let (y, m, d) = parse_ymd(date)?;
    let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?;
    Ok(local_seconds(naive))
}

/// `yyyy-MM-dd` → 当日 23:59:59 秒级时间戳（Java
/// `LocalDate.atTime(LocalTime.MAX)` 的秒级等价，与 order_admin 的日期语义一致）。
fn day_end_seconds(date: &str) -> Result<i64> {
    let (y, m, d) = parse_ymd(date)?;
    let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| Error::Validation(format!("日期不合法: {date}")))?;
    Ok(local_seconds(naive))
}

fn parse_ymd(date: &str) -> Result<(i32, u32, u32)> {
    let mut it = date.trim().split('-');
    let y = it.next().and_then(|v| v.trim().parse::<i32>().ok());
    let m = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    let d = it.next().and_then(|v| v.trim().parse::<u32>().ok());
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) => Ok((y, m, d)),
        _ => Err(Error::Validation(format!("日期格式应为 yyyy-MM-dd: {date}"))),
    }
}

fn local_seconds(naive: chrono::NaiveDateTime) -> i64 {
    use chrono::TimeZone;
    match chrono::Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.timestamp(),
        chrono::LocalResult::Ambiguous(dt, _) => dt.timestamp(),
        chrono::LocalResult::None => naive.and_utc().timestamp(),
    }
}

/// 组织名（Java `resolveOrganizationName`：0 表示平台自营）。
///
/// Java 对 `organizationId == 0` 返回配置项 `SHOP_NAME`；配置服务未迁移，
/// 此处返回 `None`（前端展示为空，而不是错误的门店名）。
/// 门店可能已被删除，此时同样返回 `None` 而不阻断列表。
#[must_use]
pub fn org_name(org_id: i32, titles: &HashMap<i32, Option<String>>) -> Option<String> {
    if org_id == 0 {
        return None;
    }
    titles.get(&org_id).cloned().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ————————————————— 库存调整算术 —————————————————

    #[test]
    fn adjust_increment_uses_signed_quantity_for_both_levels() {
        // 增量：quantity 直接作为变化量
        let e = resolve_adjust_effect(1, 5, 10, 10, Some(10));
        assert_eq!(e.sku_delta, Some(5));
        assert_eq!(e.product_delta, Some(5));

        let e = resolve_adjust_effect(1, -3, 10, 10, Some(10));
        assert_eq!(e.sku_delta, Some(-3));
        assert_eq!(e.product_delta, Some(-3));

        // 无 SKU：只调整商品
        let e = resolve_adjust_effect(1, 4, 0, 10, None);
        assert_eq!(e.sku_delta, None);
        assert_eq!(e.product_delta, Some(4));
    }

    #[test]
    fn adjust_override_without_sku_targets_product_stock() {
        // 目标值 3，当前 10 → 商品减少 7
        let e = resolve_adjust_effect(0, 3, 10, 10, None);
        assert_eq!(e.product_delta, Some(-7));
        assert_eq!(e.sku_delta, None);

        // 目标值 15，当前 10 → 商品增加 5
        let e = resolve_adjust_effect(0, 15, 10, 10, None);
        assert_eq!(e.product_delta, Some(5));

        // 目标值 == 当前 → 0
        let e = resolve_adjust_effect(0, 10, 10, 10, None);
        assert_eq!(e.product_delta, Some(0));
    }

    #[test]
    fn adjust_override_with_sku_syncs_product_by_sku_delta() {
        // SKU 目标 3，当前 10 → SKU -7；商品级同样 -7（Java 语义）
        let e = resolve_adjust_effect(0, 3, 10, 100, Some(10));
        assert_eq!(e.sku_delta, Some(-7));
        assert_eq!(e.product_delta, Some(-7));

        // SKU 目标 15，当前 10 → SKU +5；商品级 +5
        let e = resolve_adjust_effect(0, 15, 10, 100, Some(10));
        assert_eq!(e.sku_delta, Some(5));
        assert_eq!(e.product_delta, Some(5));
    }

    // ————————————————— 时间格式化 —————————————————

    #[test]
    fn fmt_time_seconds_handles_zero_and_negative() {
        assert_eq!(fmt_time_seconds(None), "");
        assert_eq!(fmt_time_seconds(Some(0)), "");
        assert_eq!(fmt_time_seconds(Some(-1)), "");
        assert_eq!(fmt_time_seconds(Some(1_700_000_000)).len(), 19);
    }

    #[test]
    fn fmt_time_millis_accepts_both_units() {
        // 13 位毫秒时间戳
        assert_eq!(fmt_time_millis(1_700_000_000_000).len(), 19);
        // 10 位秒级时间戳
        assert_eq!(fmt_time_millis(1_700_000_000).len(), 19);
        assert_eq!(fmt_time_millis(0), "");
        // 同一时刻的两种表示应得到同一文案
        assert_eq!(
            fmt_time_millis(1_700_000_000_000),
            fmt_time_millis(1_700_000_000)
        );
    }

    // ————————————————— 日期解析 —————————————————

    #[test]
    fn day_bounds_match_java() {
        let s = day_start_seconds("2026-01-01").unwrap();
        let e = day_end_seconds("2026-01-01").unwrap();
        assert_eq!(e - s, 86_399);
        assert!(day_start_seconds("2026-13-01").is_err());
        assert!(day_start_seconds("bad").is_err());
        // 前后空白容忍
        assert!(day_start_seconds(" 2026-01-01 ").is_ok());
    }

    // ————————————————— 编码生成 —————————————————

    #[test]
    fn stock_code_layout_matches_java() {
        let code = stock_code(record_type::IN, 7);
        // 1 位类型 + 3 位店铺 + 13 位毫秒 + 2 位随机 = 19 位
        assert_eq!(code.len(), 19, "code = {code}");
        assert!(code.starts_with('1'));
        assert!(code[1..4].eq("007"));
        assert_eq!(stock_code(record_type::OUT, 0).len(), 19);
    }

    // ————————————————— 组织解析 —————————————————

    #[test]
    fn org_name_returns_none_for_platform_and_missing_shop() {
        let mut titles: HashMap<i32, Option<String>> = HashMap::new();
        titles.insert(5, Some("门店A".into()));
        // 0 = 平台（Java 走配置项 SHOP_NAME，未迁移 → None）
        assert_eq!(org_name(0, &titles), None);
        assert_eq!(org_name(5, &titles).as_deref(), Some("门店A"));
        // 门店已删除 / 不存在 → None
        assert_eq!(org_name(9, &titles), None);
    }
}