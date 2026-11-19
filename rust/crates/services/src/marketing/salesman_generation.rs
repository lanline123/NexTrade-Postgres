//! 分销佣金「生成链」（原版有）：注册绑定客户、下单归属消费、消费自动开通、
//! 支付成功生成佣金行。
//!
//! 此前我方分销只有读 / 结算 / 提现 / 统计四段，写链未迁（核查报告：
//! `~/workspace/salesman-generation-chain/REPORT.md` §1 代码审计 + 线上真实订单
//! E2E 双证），本模块按原版契约补齐。结算（status 0→1）与退款失效（status→2）
//! 已由 `trade::settlement` 实现，本模块只负责生成 status=0 的行。
//!
//! 原版证据（快照仓 `lanline123/java-b2b2c-overseas-ref` v5.8.61）：
//! - C1 注册绑定：`UserServiceImpl#registerByMobile/#registerByEmail`
//!   （注册参数带 salesmanId → INSERT salesman_customer；registerToSalesman=1
//!   时一并给新用户建分销员行）
//! - C2 消费自动开通：`OrderCheckServiceImpl:1645` →
//!   `WechatBizRecruitServiceImpl:50`（微商模式 + recruitMethods∋1，提交订单时，
//!   取 cart 归因中第一个有效推广人建行；已是分销员跳过）
//! - C4 微商佣金生成：`OrderServiceImpl:2324/2334`（支付成功）→ `:3720` →
//!   `SalesmanOrderServiceImpl:521 addSalesmanOrderForWechatBiz`
//!   （order 级幂等：该单已有任意行整单跳过；受益人为买家的直接上级 +
//!   saleType≠1 时的上上级；**二级费率按直接上级的等级查表**——原版明示口径）
//! - C5 推广员佣金生成：`OrderCheckServiceImpl:904/1648 addSalesmanOrder`
//!   （提交订单时，按 cart 行 `productId_skuId → salesman_id` 归因；卖方行 +
//!   父级行，费率均按卖方等级查表）
//! - C6 计价：`SalesmanOrderServiceImpl#addNewOrderItem/#addWechatBizOrderItem`
//!   （type1=等级配置比例；type2=商品自定义比例；type3=固定金额/行；
//!   比例类 amount = price × qty × rate / 100，HALF_UP 2 位；amount < 0.01 不落行）
//!
//! 分销员禁用挂点：所有受益人 / 推广人在落行或建行前都经过
//! `generation_eligible`——当前 salesman 表无 status 字段，恒为 true；
//! 禁用功能（salesman.status）落地时只改此一处，即可实现「禁用后不计新佣金」
//! （衔接 `~/workspace/distributor-disable-eval/REPORT.md` §3/§5）。

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use rust_decimal::{Decimal, RoundingStrategy};
use serde_json::{json, Value};

use nextrade_core::models::marketing::salesman_product::{
    parse_commission_data, CommissionLevelData,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

use crate::marketing::salesman_product::config_levels;

const CFG_SALESMAN_CONFIG: &str = "salesmanConfig";
const CFG_SALESMAN_SETTLEMENT: &str = "salesmanSettlement";

/// 佣金下限：低于 1 分不落行（原版 C6 两路一致）。
fn min_amount() -> Decimal {
    Decimal::new(1, 2)
}

fn db_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}

fn now_i32() -> i32 {
    nextrade_common::utils::now_seconds() as i32
}

/// 等级费率一行（配置等级表与商品 levelArr 同构）。
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRate {
    pub level: i32,
    pub rate: Option<Decimal>,
    pub down_rate: Option<Decimal>,
}

fn level_rate_from(l: &CommissionLevelData) -> Option<LevelRate> {
    Some(LevelRate {
        level: l.level?,
        rate: l.rate.as_deref().and_then(|s| Decimal::from_str(s.trim()).ok()),
        down_rate: l
            .down_salesman_rate
            .as_deref()
            .and_then(|s| Decimal::from_str(s.trim()).ok()),
    })
}

/// 链上分销员（计价 / 归因的最小视图）。
#[derive(Debug, Clone, PartialEq)]
pub struct ChainSalesman {
    pub salesman_id: i32,
    pub user_id: i32,
    pub level: i32,
    pub pid: i32,
}

/// 商品佣金配置（salesman_product 参与行 + 解析后的等级表）。
#[derive(Debug, Clone)]
pub struct ProductCommission {
    pub salesman_product_id: i32,
    pub product_id: i32,
    pub shop_id: i32,
    /// 1=默认比例 2=自定义比例 3=自定义金额。
    pub commission_type: i32,
    /// type 2/3 的商品等级表；type 1 为空（用全局配置等级表）。
    pub level_arr: Vec<LevelRate>,
    /// 快照用的 commissionData（type 1 已按原版口径回填为配置等级表）。
    pub snapshot_commission_data: Value,
}

/// 订单行计价输入。
#[derive(Debug, Clone, PartialEq)]
pub struct ItemInput {
    pub item_id: i32,
    pub product_id: i32,
    pub sku_id: i32,
    pub price: Decimal,
    pub quantity: i32,
}

/// 待落库的佣金行（纯计划函数的产物）。
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedRow {
    pub item_id: i32,
    pub product_id: i32,
    /// 受益分销员。
    pub salesman_id: i32,
    /// price × qty。
    pub order_amount: Decimal,
    /// 佣金。
    pub amount: Decimal,
    /// 1=直接受益行；2=二级行（快照带 orderSaleType=2）。
    pub order_sale_type: i32,
    /// 快照 salesman 的主体（原版口径：二级行快照的也是查表主体分销员）。
    pub snapshot_salesman_id: i32,
}

/// 分销员禁用挂点：生成侧资格过滤。
///
/// 当前 salesman 表没有 status 字段，恒为 true。禁用功能落地后在此读状态：
/// 被禁用的分销员不计新佣金、不作为推广人建行，其余生成逻辑无需改动。
pub fn generation_eligible(_salesman: &ChainSalesman) -> bool {
    true
}

/// 单 item 佣金计价（C6）。
///
/// `lookup_level` 为查表等级：微商模式两行都按**直接上级**等级查表（原版口径，
/// 二级行只是受益人换成上上级、费率列换成 downSalesmanRate）；推广员模式按卖方
/// 等级查表。`down` 表示取 downSalesmanRate 列。
/// 返回 `None` = 不落行（无费率 / 低于 0.01 下限）。
pub fn commission_amount(
    product: &ProductCommission,
    config_levels: &[LevelRate],
    lookup_level: i32,
    down: bool,
    price: Decimal,
    quantity: i32,
) -> Option<Decimal> {
    let table: &[LevelRate] = match product.commission_type {
        1 => config_levels,
        2 | 3 => &product.level_arr,
        _ => return None,
    };
    let row = table.iter().find(|r| r.level == lookup_level)?;
    let rate = if down { row.down_rate } else { row.rate }?;
    let amount = if product.commission_type == 3 {
        // 固定金额：每 item 行一次，不乘数量、不除 100。
        rate
    } else {
        (price * Decimal::from(quantity) * rate / Decimal::from(100))
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
    };
    if amount < min_amount() {
        None
    } else {
        Some(amount)
    }
}

/// 微商模式佣金计划（C4）：受益人 = 买家直接上级（+ saleType≠1 时的上上级）。
pub fn plan_wechatbiz_rows(
    items: &[ItemInput],
    products: &HashMap<i32, ProductCommission>,
    config_levels: &[LevelRate],
    parent: &ChainSalesman,
    grandparent: Option<&ChainSalesman>,
    sale_type: i32,
) -> Vec<PlannedRow> {
    let mut rows = Vec::new();
    for item in items {
        let Some(product) = products.get(&item.product_id) else {
            continue;
        };
        let order_amount = item.price * Decimal::from(item.quantity);
        if let Some(amount) = commission_amount(
            product,
            config_levels,
            parent.level,
            false,
            item.price,
            item.quantity,
        ) {
            rows.push(PlannedRow {
                item_id: item.item_id,
                product_id: item.product_id,
                salesman_id: parent.salesman_id,
                order_amount,
                amount,
                order_sale_type: 1,
                snapshot_salesman_id: parent.salesman_id,
            });
        }
        if sale_type != 1 {
            if let Some(gp) = grandparent {
                if let Some(amount) = commission_amount(
                    product,
                    config_levels,
                    parent.level,
                    true,
                    item.price,
                    item.quantity,
                ) {
                    rows.push(PlannedRow {
                        item_id: item.item_id,
                        product_id: item.product_id,
                        salesman_id: gp.salesman_id,
                        order_amount,
                        amount,
                        order_sale_type: 2,
                        snapshot_salesman_id: parent.salesman_id,
                    });
                }
            }
        }
    }
    rows
}

/// 推广员模式佣金计划（C5）：逐 item 按归因卖方生成卖方行 + 父级行，
/// 两行费率都按**卖方**等级查表（父级行取 downSalesmanRate 列）。
///
/// `sellers` 须同时含卖方与其父级（服务层预加载）；缺失任一节点则该行跳过。
pub fn plan_promoter_rows(
    items: &[ItemInput],
    attributions: &HashMap<(i32, i32), i32>,
    sellers: &HashMap<i32, ChainSalesman>,
    products: &HashMap<i32, ProductCommission>,
    config_levels: &[LevelRate],
    sale_type: i32,
) -> Vec<PlannedRow> {
    let mut rows = Vec::new();
    for item in items {
        let Some(&seller_id) = attributions.get(&(item.product_id, item.sku_id)) else {
            continue;
        };
        if seller_id <= 0 {
            continue;
        }
        let Some(seller) = sellers.get(&seller_id) else {
            continue;
        };
        let Some(product) = products.get(&item.product_id) else {
            continue;
        };
        let order_amount = item.price * Decimal::from(item.quantity);
        if let Some(amount) = commission_amount(
            product,
            config_levels,
            seller.level,
            false,
            item.price,
            item.quantity,
        ) {
            rows.push(PlannedRow {
                item_id: item.item_id,
                product_id: item.product_id,
                salesman_id: seller.salesman_id,
                order_amount,
                amount,
                order_sale_type: 1,
                snapshot_salesman_id: seller.salesman_id,
            });
        }
        if sale_type != 1 && seller.pid > 0 {
            if let Some(parent) = sellers.get(&seller.pid) {
                if let Some(amount) = commission_amount(
                    product,
                    config_levels,
                    seller.level,
                    true,
                    item.price,
                    item.quantity,
                ) {
                    rows.push(PlannedRow {
                        item_id: item.item_id,
                        product_id: item.product_id,
                        salesman_id: parent.salesman_id,
                        order_amount,
                        amount,
                        order_sale_type: 2,
                        snapshot_salesman_id: seller.salesman_id,
                    });
                }
            }
        }
    }
    rows
}

/// 消费自动开通的推广人选择（C2）：按 cart 归因顺序取第一个有效推广人
/// （调用方已过滤不存在 / 不合资格者；此处再排除推广人本人即买家）。
pub fn pick_promoter(candidates: &[ChainSalesman], buyer_user_id: i32) -> Option<i32> {
    candidates
        .iter()
        .find(|s| s.user_id != buyer_user_id)
        .map(|s| s.salesman_id)
}

/// 分销配置（salesmanConfig）的生成链视图。
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationConfig {
    /// 1=推广员模式 2=微商模式（缺省按原版枚举缺省 1）。
    pub distribution_mode: i32,
    /// 1=一级分销；≠1 时生成二级行（缺省 1）。
    pub sale_type: i32,
    /// recruitMethods 是否含 1（消费自动开通）。
    pub recruit_consume: bool,
    /// registerToSalesman=1 时注册绑定一并建分销员行。
    pub register_to_salesman: bool,
    pub levels: Vec<LevelRate>,
}

/// 解析 salesmanConfig JSON（字段口径与现行读侧一致：等级行键为 `id`）。
pub fn parse_generation_config(cfg: &Value, config_data_raw: &str) -> GenerationConfig {
    let get_i32 = |key: &str| cfg.get(key).and_then(Value::as_i64).map(|v| v as i32);
    let recruit_consume = cfg
        .get("recruitMethods")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().any(|v| v.as_i64() == Some(1)))
        .unwrap_or(false);
    GenerationConfig {
        distribution_mode: get_i32("distributionMode").unwrap_or(1),
        sale_type: get_i32("saleType").unwrap_or(1),
        recruit_consume,
        register_to_salesman: get_i32("registerToSalesman").unwrap_or(0) == 1,
        levels: config_levels(config_data_raw)
            .iter()
            .filter_map(level_rate_from)
            .collect(),
    }
}

/// 解析商品 commission_data → 等级表（type 2/3 用）。
pub fn parse_product_level_arr(raw: Option<&str>) -> Vec<LevelRate> {
    parse_commission_data(raw)
        .map(|data| {
            data.iter()
                .flat_map(|d| d.level_arr.iter().flatten())
                .filter_map(level_rate_from)
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SalesmanRow {
    salesman_id: i32,
    user_id: Option<i32>,
    level: Option<i32>,
    pid: i32,
}

impl SalesmanRow {
    fn to_chain(&self) -> ChainSalesman {
        ChainSalesman {
            salesman_id: self.salesman_id,
            user_id: self.user_id.unwrap_or(0),
            level: self.level.unwrap_or(1),
            pid: self.pid,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SalesmanProductRow {
    salesman_product_id: i32,
    commission_type: Option<i32>,
    commission_data: Option<String>,
    shop_id: Option<i32>,
}

/// 分销佣金生成服务。构造方式与 `SettlementService` 一致：调用点以
/// `SalesmanGenerationService::new(repos.clone())` 现场构造，无全局装配。
pub struct SalesmanGenerationService {
    repos: Arc<Repositories>,
}

impl SalesmanGenerationService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.repos.pool
    }

    /// 读 salesman_config 指定 code 的 data（取最小 id 行，与后台读侧一致）。
    async fn config_raw(&self, code: &str) -> Result<Option<String>> {
        sqlx::query_scalar::<_, String>(
            "SELECT data FROM salesman_config WHERE code = $1 ORDER BY id ASC LIMIT 1",
        )
        .bind(code)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)
    }

    async fn generation_config(&self) -> Result<Option<(GenerationConfig, String)>> {
        let Some(raw) = self.config_raw(CFG_SALESMAN_CONFIG).await? else {
            return Ok(None);
        };
        let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        Ok(Some((parse_generation_config(&value, &raw), raw)))
    }

    async fn salesman_by_id(&self, salesman_id: i32) -> Result<Option<ChainSalesman>> {
        let row = sqlx::query_as::<_, SalesmanRow>(
            "SELECT salesman_id, user_id, level, pid FROM salesman WHERE salesman_id = $1",
        )
        .bind(salesman_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        Ok(row.map(|r| r.to_chain()))
    }

    async fn salesman_by_user_id(&self, user_id: i32) -> Result<Option<ChainSalesman>> {
        let row = sqlx::query_as::<_, SalesmanRow>(
            "SELECT salesman_id, user_id, level, pid FROM salesman \
             WHERE user_id = $1 ORDER BY salesman_id ASC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        Ok(row.map(|r| r.to_chain()))
    }

    /// 建分销员行（口径与名片邀请 accept_card_invite 一致：level=1、group=0、shop=0）。
    async fn create_salesman(&self, user_id: i32, pid: i32) -> Result<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO salesman (user_id, level, group_id, pid, add_time, shop_id, sale_amount) \
             VALUES ($1, 1, 0, $2, $3, 0, 0) RETURNING salesman_id",
        )
        .bind(user_id)
        .bind(pid)
        .bind(now_i32())
        .fetch_one(self.pool())
        .await
        .map_err(db_err)
    }

    /// 商品佣金配置：店铺行优先、平台行（shop_id=0）兜底（原版口径）。
    async fn product_commission(
        &self,
        product_id: i32,
        shop_id: i32,
        config_levels: &[LevelRate],
    ) -> Result<Option<ProductCommission>> {
        let row = sqlx::query_as::<_, SalesmanProductRow>(
            "SELECT salesman_product_id, commission_type, commission_data, shop_id \
             FROM salesman_product \
             WHERE product_id = $1 AND is_join = 1 AND shop_id IN ($2, 0) \
             ORDER BY CASE WHEN shop_id = $2 THEN 0 ELSE 1 END, salesman_product_id ASC \
             LIMIT 1",
        )
        .bind(product_id)
        .bind(shop_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let commission_type = row.commission_type.unwrap_or(1);
        let (level_arr, snapshot_commission_data) = if commission_type == 1 {
            // 原版口径：type 1 快照的 commissionData 回填为配置等级表。
            let levels_json: Vec<Value> = config_levels
                .iter()
                .map(|l| {
                    json!({
                        "level": l.level,
                        "rate": l.rate.map(|d| d.to_string()),
                        "downSalesmanRate": l.down_rate.map(|d| d.to_string()),
                    })
                })
                .collect();
            (Vec::new(), json!([{ "levelArr": levels_json }]))
        } else {
            let snapshot = row
                .commission_data
                .as_deref()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or(Value::Null);
            (parse_product_level_arr(row.commission_data.as_deref()), snapshot)
        };
        Ok(Some(ProductCommission {
            salesman_product_id: row.salesman_product_id,
            product_id,
            shop_id: row.shop_id.unwrap_or(0),
            commission_type,
            level_arr,
            snapshot_commission_data,
        }))
    }

    /// 组装 salesman_product_data 快照（字段命名对齐原版 SalesmanProductVO；
    /// 我方读侧按不透明 JSON 消费，结算快照另存 salesman_settlement_data）。
    fn product_snapshot(
        product: &ProductCommission,
        snapshot_salesman: &ChainSalesman,
        order_sale_type: i32,
    ) -> String {
        let mut v = json!({
            "salesmanProductId": product.salesman_product_id,
            "productId": product.product_id,
            "isJoin": 1,
            "commissionType": product.commission_type,
            "commissionData": product.snapshot_commission_data,
            "shopId": product.shop_id,
            "salesman": {
                "salesmanId": snapshot_salesman.salesman_id,
                "userId": snapshot_salesman.user_id,
                "level": snapshot_salesman.level,
                "pid": snapshot_salesman.pid,
            },
        });
        if order_sale_type == 2 {
            v["orderSaleType"] = json!(2);
        }
        v.to_string()
    }

    async fn insert_order_row(
        &self,
        order_id: i32,
        row: &PlannedRow,
        snapshot: &str,
        settlement_snapshot: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO salesman_order \
             (order_id, item_id, product_id, salesman_id, salesman_product_data, \
              salesman_settlement_data, order_amount, amount, add_time) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(order_id)
        .bind(row.item_id)
        .bind(row.product_id)
        .bind(row.salesman_id)
        .bind(snapshot)
        .bind(settlement_snapshot)
        .bind(row.order_amount)
        .bind(row.amount)
        .bind(now_i32())
        .execute(self.pool())
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn order_row_exists(&self, order_id: i32, item_id: i32, salesman_id: i32) -> Result<bool> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM salesman_order \
             WHERE order_id = $1 AND item_id = $2 AND salesman_id = $3",
        )
        .bind(order_id)
        .bind(item_id)
        .bind(salesman_id)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        Ok(n > 0)
    }

    async fn load_products(
        &self,
        items: &[ItemInput],
        item_shop: &HashMap<i32, i32>,
        config_levels: &[LevelRate],
    ) -> Result<HashMap<i32, ProductCommission>> {
        let mut map = HashMap::new();
        for item in items {
            if map.contains_key(&item.product_id) {
                continue;
            }
            let shop_id = item_shop.get(&item.item_id).copied().unwrap_or(0);
            if let Some(pc) = self
                .product_commission(item.product_id, shop_id, config_levels)
                .await?
            {
                map.insert(item.product_id, pc);
            }
        }
        Ok(map)
    }

    async fn settlement_snapshot(&self) -> String {
        self.config_raw(CFG_SALESMAN_SETTLEMENT)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "{}".to_string())
    }

    /// 提交订单成功后的分销处理（挂点：`OrderService::create` 尾部）。
    ///
    /// `attributions` 为本单 cart 行的 (product_id, sku_id, salesman_id) 三元组。
    /// - 微商模式（mode=2）：走 C2 消费自动开通（佣金在支付成功时生成）；
    /// - 推广员模式（mode=1）：走 C5 当场生成佣金行（带行级防重——原版提交为
    ///   单次动作，我方加 (order_id, item_id, salesman_id) 存在性检查防重复
    ///   提交双记，口径差异在此注明）。
    pub async fn on_order_submitted(
        &self,
        buyer_user_id: i32,
        order_id: i32,
        attributions: &[(i32, Option<i32>, i32)],
    ) -> Result<()> {
        let Some((cfg, _raw)) = self.generation_config().await? else {
            return Ok(());
        };
        if cfg.distribution_mode == 2 {
            self.auto_recruit(buyer_user_id, attributions, &cfg).await
        } else {
            self.generate_promoter(order_id, attributions, &cfg).await
        }
    }

    /// C2 消费自动开通：买家已是分销员则跳过（不改上级）；否则取 cart 归因中
    /// 第一个有效推广人建行。
    async fn auto_recruit(
        &self,
        buyer_user_id: i32,
        attributions: &[(i32, Option<i32>, i32)],
        cfg: &GenerationConfig,
    ) -> Result<()> {
        if !cfg.recruit_consume {
            return Ok(());
        }
        if self.salesman_by_user_id(buyer_user_id).await?.is_some() {
            return Ok(());
        }
        let mut seen: Vec<i32> = Vec::new();
        let mut candidates: Vec<ChainSalesman> = Vec::new();
        for (_, _, salesman_id) in attributions {
            if *salesman_id <= 0 || seen.contains(salesman_id) {
                continue;
            }
            seen.push(*salesman_id);
            if let Some(s) = self.salesman_by_id(*salesman_id).await? {
                if generation_eligible(&s) {
                    candidates.push(s);
                }
            }
        }
        if let Some(promoter_id) = pick_promoter(&candidates, buyer_user_id) {
            self.create_salesman(buyer_user_id, promoter_id).await?;
            tracing::info!("分销消费自动开通: user={buyer_user_id} pid={promoter_id}");
        }
        Ok(())
    }

    /// C5 推广员模式生成（提交订单时）。
    async fn generate_promoter(
        &self,
        order_id: i32,
        attributions: &[(i32, Option<i32>, i32)],
        cfg: &GenerationConfig,
    ) -> Result<()> {
        let mut attr_map: HashMap<(i32, i32), i32> = HashMap::new();
        for (product_id, sku_id, salesman_id) in attributions {
            attr_map
                .entry((*product_id, sku_id.unwrap_or(0)))
                .or_insert(*salesman_id);
        }
        if attr_map.is_empty() {
            return Ok(());
        }
        // 预加载卖方 + 其父级，并过禁用挂点过滤。
        let mut sellers: HashMap<i32, ChainSalesman> = HashMap::new();
        for seller_id in attr_map.values().copied().collect::<Vec<_>>() {
            if seller_id <= 0 || sellers.contains_key(&seller_id) {
                continue;
            }
            if let Some(seller) = self.salesman_by_id(seller_id).await? {
                if !generation_eligible(&seller) {
                    continue;
                }
                if seller.pid > 0 && !sellers.contains_key(&seller.pid) {
                    if let Some(parent) = self.salesman_by_id(seller.pid).await? {
                        if generation_eligible(&parent) {
                            sellers.insert(parent.salesman_id, parent);
                        }
                    }
                }
                sellers.insert(seller.salesman_id, seller);
            }
        }
        if sellers.is_empty() {
            return Ok(());
        }
        let order_items = self.repos.trade.order.find_items(order_id).await?;
        let items: Vec<ItemInput> = order_items
            .iter()
            .map(|it| ItemInput {
                item_id: it.item_id,
                product_id: it.product_id,
                sku_id: it.sku_id,
                price: it.price,
                quantity: it.quantity,
            })
            .collect();
        let item_shop: HashMap<i32, i32> = order_items
            .iter()
            .map(|it| (it.item_id, it.shop_id))
            .collect();
        let products = self.load_products(&items, &item_shop, &cfg.levels).await?;
        let rows = plan_promoter_rows(
            &items,
            &attr_map,
            &sellers,
            &products,
            &cfg.levels,
            cfg.sale_type,
        );
        if rows.is_empty() {
            return Ok(());
        }
        let settlement_snapshot = self.settlement_snapshot().await;
        for row in &rows {
            if self
                .order_row_exists(order_id, row.item_id, row.salesman_id)
                .await?
            {
                continue;
            }
            let Some(snapshot_subject) = sellers.get(&row.snapshot_salesman_id) else {
                continue;
            };
            let Some(product) = products.get(&row.product_id) else {
                continue;
            };
            let snapshot = Self::product_snapshot(product, snapshot_subject, row.order_sale_type);
            self.insert_order_row(order_id, row, &snapshot, &settlement_snapshot)
                .await?;
        }
        Ok(())
    }

    /// 支付成功后的分销处理（挂点：`OrderService::on_payment_success` 与余额
    /// 支付全额付清分支）。仅微商模式（C4）；推广员模式已在提交时生成。
    ///
    /// 幂等与原版一致：该 order_id 已有任意 salesman_order 行 → 整单跳过，
    /// 因此多支付通道 / 重复回调 / 两处挂点先后触发都不会双记。
    pub async fn on_order_paid(&self, order_id: i32) -> Result<()> {
        let Some((cfg, _raw)) = self.generation_config().await? else {
            return Ok(());
        };
        if cfg.distribution_mode != 2 {
            return Ok(());
        }
        let existing: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM salesman_order WHERE order_id = $1")
                .bind(order_id)
                .fetch_one(self.pool())
                .await
                .map_err(db_err)?;
        if existing > 0 {
            return Ok(());
        }
        let Some(order) = self.repos.trade.order.find_by_id(order_id).await? else {
            return Ok(());
        };
        let Some(buyer) = self.salesman_by_user_id(order.user_id).await? else {
            return Ok(());
        };
        if buyer.pid <= 0 {
            return Ok(());
        }
        let Some(parent) = self.salesman_by_id(buyer.pid).await? else {
            return Ok(());
        };
        if !generation_eligible(&parent) {
            return Ok(());
        }
        let grandparent = if cfg.sale_type != 1 && parent.pid > 0 {
            match self.salesman_by_id(parent.pid).await? {
                Some(gp) if generation_eligible(&gp) => Some(gp),
                _ => None,
            }
        } else {
            None
        };
        let order_items = self.repos.trade.order.find_items(order_id).await?;
        let items: Vec<ItemInput> = order_items
            .iter()
            .map(|it| ItemInput {
                item_id: it.item_id,
                product_id: it.product_id,
                sku_id: it.sku_id,
                price: it.price,
                quantity: it.quantity,
            })
            .collect();
        let item_shop: HashMap<i32, i32> = order_items
            .iter()
            .map(|it| (it.item_id, it.shop_id))
            .collect();
        let products = self.load_products(&items, &item_shop, &cfg.levels).await?;
        let rows = plan_wechatbiz_rows(
            &items,
            &products,
            &cfg.levels,
            &parent,
            grandparent.as_ref(),
            cfg.sale_type,
        );
        if rows.is_empty() {
            return Ok(());
        }
        let settlement_snapshot = self.settlement_snapshot().await;
        for row in &rows {
            let snapshot = Self::product_snapshot(
                &products[&row.product_id],
                &parent,
                row.order_sale_type,
            );
            self.insert_order_row(order_id, row, &snapshot, &settlement_snapshot)
                .await?;
        }
        tracing::info!("分销佣金生成(微商): order={order_id} rows={}", rows.len());
        Ok(())
    }

    /// 注册成功后的分销绑定（C1，挂点：`UserService::register_with_salesman`）。
    ///
    /// `salesman_ref` 为前端带来的 salesmanId：原版口径先按 salesman_id 解析、
    /// 再按 user_id 解析。绑定行靠 salesman_customer.user_id 唯一索引防重
    /// （ON CONFLICT DO NOTHING）。registerToSalesman=1 时一并建分销员行。
    /// 原版的邀请人升级重评走 MQ 异步触发；我方升级在结算链内同步重评，
    /// 绑定本身不改变邀请人等级入账口径，此处不另行触发。
    pub async fn on_user_registered(
        &self,
        new_user_id: i32,
        salesman_ref: i32,
        channel: &str,
    ) -> Result<()> {
        if salesman_ref <= 0 {
            return Ok(());
        }
        let referrer = match self.salesman_by_id(salesman_ref).await? {
            Some(s) => Some(s),
            None => self.salesman_by_user_id(salesman_ref).await?,
        };
        let Some(referrer) = referrer else {
            return Ok(());
        };
        if !generation_eligible(&referrer) {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO salesman_customer (salesman_id, user_id, add_time, channel, remark) \
             VALUES ($1, $2, $3, $4, '') ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(referrer.salesman_id)
        .bind(new_user_id)
        .bind(now_i32())
        .bind(channel)
        .execute(self.pool())
        .await
        .map_err(db_err)?;
        if let Some((cfg, _)) = self.generation_config().await? {
            if cfg.register_to_salesman && self.salesman_by_user_id(new_user_id).await?.is_none() {
                self.create_salesman(new_user_id, referrer.salesman_id).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn levels() -> Vec<LevelRate> {
        vec![
            LevelRate {
                level: 1,
                rate: Some(d("1.10")),
                down_rate: Some(d("2")),
            },
            LevelRate {
                level: 2,
                rate: Some(d("1.50")),
                down_rate: Some(d("3")),
            },
            LevelRate {
                level: 3,
                rate: Some(d("3")),
                down_rate: Some(d("5")),
            },
        ]
    }

    fn product_type1() -> ProductCommission {
        ProductCommission {
            salesman_product_id: 2,
            product_id: 1591,
            shop_id: 0,
            commission_type: 1,
            level_arr: vec![],
            snapshot_commission_data: Value::Null,
        }
    }

    fn chain(id: i32, user_id: i32, level: i32, pid: i32) -> ChainSalesman {
        ChainSalesman {
            salesman_id: id,
            user_id,
            level,
            pid,
        }
    }

    fn item(item_id: i32, product_id: i32, price: &str, qty: i32) -> ItemInput {
        ItemInput {
            item_id,
            product_id,
            sku_id: 0,
            price: d(price),
            quantity: qty,
        }
    }

    #[test]
    fn type1_rate_amount() {
        // E2E 口径：¥100 × L1 1.10% = ¥1.10；down 2% = ¥2.00。
        let p = product_type1();
        assert_eq!(
            commission_amount(&p, &levels(), 1, false, d("100"), 1),
            Some(d("1.10"))
        );
        assert_eq!(
            commission_amount(&p, &levels(), 1, true, d("100"), 1),
            Some(d("2.00"))
        );
        // 数量乘算：¥100 × 3 × L2 1.50% = ¥4.50。
        assert_eq!(
            commission_amount(&p, &levels(), 2, false, d("100"), 3),
            Some(d("4.50"))
        );
    }

    #[test]
    fn type1_rounding_half_up() {
        let p = product_type1();
        // 33.33 × 3 × 1.50% = 1.49985 → HALF_UP 1.50。
        assert_eq!(
            commission_amount(&p, &levels(), 2, false, d("33.33"), 3),
            Some(d("1.50"))
        );
    }

    #[test]
    fn floor_skips_tiny_amount() {
        let p = product_type1();
        // 1 × 0.10% = 0.001 → 舍入 0.00 < 0.01 → 不落行。
        let tiny = vec![LevelRate {
            level: 9,
            rate: Some(d("0.1")),
            down_rate: None,
        }];
        assert_eq!(commission_amount(&p, &tiny, 9, false, d("1"), 1), None);
    }

    #[test]
    fn type2_uses_product_level_arr() {
        let p = ProductCommission {
            commission_type: 2,
            level_arr: vec![LevelRate {
                level: 1,
                rate: Some(d("10")),
                down_rate: Some(d("4")),
            }],
            ..product_type1()
        };
        // 配置表里 L1 是 1.10%，type2 必须用商品表的 10%。
        assert_eq!(
            commission_amount(&p, &levels(), 1, false, d("100"), 1),
            Some(d("10.00"))
        );
        assert_eq!(
            commission_amount(&p, &levels(), 1, true, d("100"), 1),
            Some(d("4.00"))
        );
    }

    #[test]
    fn type3_fixed_amount_ignores_qty() {
        let p = ProductCommission {
            commission_type: 3,
            level_arr: vec![LevelRate {
                level: 1,
                rate: Some(d("5")),
                down_rate: Some(d("1.5")),
            }],
            ..product_type1()
        };
        assert_eq!(
            commission_amount(&p, &levels(), 1, false, d("100"), 7),
            Some(d("5"))
        );
        assert_eq!(
            commission_amount(&p, &levels(), 1, true, d("100"), 7),
            Some(d("1.5"))
        );
    }

    #[test]
    fn wechatbiz_plan_two_levels_lookup_by_parent_level() {
        // 线上 E2E 链：买家上级 #7(L1)、上上级 #1(L3)，saleType=2。
        // 期望：#7 得 ¥1.10（L1 rate），#1 得 ¥2.00（按 L1 的 down 2%，不是 L3 的 5%）。
        let parent = chain(7, 120, 1, 1);
        let grandparent = chain(1, 82, 3, 0);
        let mut products = HashMap::new();
        products.insert(1591, product_type1());
        let rows = plan_wechatbiz_rows(
            &[item(11, 1591, "100", 1)],
            &products,
            &levels(),
            &parent,
            Some(&grandparent),
            2,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].salesman_id, 7);
        assert_eq!(rows[0].amount, d("1.10"));
        assert_eq!(rows[0].order_sale_type, 1);
        assert_eq!(rows[1].salesman_id, 1);
        assert_eq!(rows[1].amount, d("2.00"));
        assert_eq!(rows[1].order_sale_type, 2);
        // 二级行快照主体仍是直接上级（原版口径）。
        assert_eq!(rows[1].snapshot_salesman_id, 7);
    }

    #[test]
    fn wechatbiz_plan_sale_type1_direct_only() {
        let parent = chain(7, 120, 1, 1);
        let grandparent = chain(1, 82, 3, 0);
        let mut products = HashMap::new();
        products.insert(1591, product_type1());
        let rows = plan_wechatbiz_rows(
            &[item(11, 1591, "100", 1)],
            &products,
            &levels(),
            &parent,
            Some(&grandparent),
            1,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].salesman_id, 7);
    }

    #[test]
    fn wechatbiz_plan_skips_non_join_product() {
        let parent = chain(7, 120, 1, 1);
        let rows = plan_wechatbiz_rows(
            &[item(11, 9999, "100", 1)],
            &HashMap::new(),
            &levels(),
            &parent,
            None,
            2,
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn promoter_plan_parent_row_uses_seller_level() {
        // 卖方 #8(L2, pid=7)：卖方行 ¥100×1.50%=1.50；父级行按卖方 L2 的 down 3% = 3.00。
        let seller = chain(8, 121, 2, 7);
        let parent = chain(7, 120, 1, 1);
        let mut sellers = HashMap::new();
        sellers.insert(8, seller);
        sellers.insert(7, parent);
        let mut attributions = HashMap::new();
        attributions.insert((1591, 0), 8);
        let mut products = HashMap::new();
        products.insert(1591, product_type1());
        let rows = plan_promoter_rows(
            &[item(11, 1591, "100", 1)],
            &attributions,
            &sellers,
            &products,
            &levels(),
            2,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].salesman_id, 8);
        assert_eq!(rows[0].amount, d("1.50"));
        assert_eq!(rows[1].salesman_id, 7);
        assert_eq!(rows[1].amount, d("3.00"));
        assert_eq!(rows[1].snapshot_salesman_id, 8);
    }

    #[test]
    fn promoter_plan_no_attribution_no_rows() {
        let rows = plan_promoter_rows(
            &[item(11, 1591, "100", 1)],
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &levels(),
            2,
        );
        assert!(rows.is_empty());
    }

    #[test]
    fn pick_promoter_skips_self_and_takes_first() {
        let self_first = chain(9, 555, 1, 0);
        let valid = chain(7, 120, 1, 1);
        assert_eq!(pick_promoter(&[self_first, valid], 555), Some(7));
        assert_eq!(pick_promoter(&[], 555), None);
    }

    #[test]
    fn parse_config_live_shape() {
        let raw = "{\"distributionMode\":2,\"recruitMethods\":[1,2],\"saleType\":2,\
            \"registerToSalesman\":0,\
            \"level\":[{\"id\":1,\"rate\":\"1.10\",\"downSalesmanRate\":\"2\"},\
            {\"id\":3,\"rate\":\"3\",\"downSalesmanRate\":\"5\"}]}";
        let v: Value = serde_json::from_str(raw).unwrap();
        let cfg = parse_generation_config(&v, raw);
        assert_eq!(cfg.distribution_mode, 2);
        assert_eq!(cfg.sale_type, 2);
        assert!(cfg.recruit_consume);
        assert!(!cfg.register_to_salesman);
        assert_eq!(cfg.levels.len(), 2);
        assert_eq!(cfg.levels[0].level, 1);
        assert_eq!(cfg.levels[0].rate, Some(d("1.10")));
        assert_eq!(cfg.levels[0].down_rate, Some(d("2")));
    }

    #[test]
    fn parse_product_level_arr_shapes() {
        let arr = parse_product_level_arr(Some(
            "[{\"levelArr\":[{\"level\":1,\"rate\":\"10\",\"downSalesmanRate\":\"4\"}]}]",
        ));
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0].rate, Some(d("10")));
        assert_eq!(arr[0].down_rate, Some(d("4")));
        assert!(parse_product_level_arr(None).is_empty());
        assert!(parse_product_level_arr(Some("not json")).is_empty());
    }
}
