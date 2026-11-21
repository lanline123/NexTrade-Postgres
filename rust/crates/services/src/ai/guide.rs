//! 买家侧 AI 导购引擎（受限 MVP，我方新增；评估 `ai-shopping-guide-eval` 定版）。
//!
//! 与运营助手的区别（评估 §3）：主体是买家（user 维度）、平台维度会话、
//! 工具全只读、无写确认状态机。红线（评估 §5.1/§5.5）：
//! - 工具一律走公开口径并携带当前买家身份：商品走 `ProductService` 公开
//!   列表/详情 + `price_gate` 按该买家身份脱敏后再喂模型；订单走
//!   `OrderService` 的本人作用域（`Some(user_id)`），结构上查不到他人订单；
//! - 模型只能逐字引用工具返回的价格/商品，系统提示词明文禁止自编；
//! - 平台自担成本，三级预算熔断（单会话 / 单用户日 / 全局日，阈值存
//!   `config.aiGuideSettings` JSON，可配）。
//!
//! 会话/消息用独立表 `ai_guide_conversation / ai_guide_message`（迁移
//! `rust/deploy/pg/2026-11-21_ai_guide.sql`），不混 admin 语义；用量记
//! `ai_usage` 的 scene=`guide` 行。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use futures::SinkExt;
use nextrade_core::models::catalog::product::ProductListQuery;
use nextrade_core::models::system::recommend::CohortBuysQuery;
use nextrade_core::types::PageQuery;
use nextrade_infrastructure::db::Repositories;
use serde_json::{json, Value};

use super::{channel, upstream, usage};
use crate::catalog::product::ProductService;
use crate::system::config_client::ConfigClientService;
use crate::system::price_gate;
use crate::system::recommend::RecommendService;
use crate::trade::order::OrderService;

/// 导购配置在 `config` 表的 biz_code（JSON）。
pub const GUIDE_CONFIG_CODE: &str = "aiGuideSettings";

const MAX_ROUNDS: usize = 6;
const MAX_INFLIGHT_PER_USER: usize = 2;
/// 工具结果喂回模型的字符上限（与运营助手同口径）。
const TOOL_RESULT_LIMIT: usize = 6000;

/// 导购系统提示词（价格/事实红线明文写死，探针用例集据此验收）。
const GUIDE_SYSTEM_PROMPT: &str = "你是 NexTrade 商城的 AI 导购助手，帮助买家找商品、了解商品详情、按其购买与浏览偏好推荐商品、查询他自己的订单状态。\n\n必须遵守的规则：\n1. 商品的价格、库存、销量、规格参数只能来自工具返回，并且只能逐字引用工具返回的数值；工具没有返回的价格绝不猜测、绝不估算、绝不引用其它商品的价格。\n2. 工具返回中某商品 priceVisible=false 时，该商品价格不可见：按其 priceGate 口径向用户说明（login=需要登录查看、certification=需要完成实名认证查看），不要给出任何数字价格。\n3. 只能介绍工具返回过的商品，不要虚构商品名称、型号、参数或活动。用用户问法中的品牌、型号、品类等关键词搜索后没有结果时，必须如实说明没有找到该商品并建议用户更换关键词；不得自行改用更宽泛的关键词再次搜索来凑结果，无命中时不得展示任何商品、不得引用任何商品的价格（换关键词重新搜索只能由用户提出）。凡在回答中展示了商品，正文必须逐项点名所展示的商品并说明它与用户问法的关系，不得出现正文未提及的商品卡。\n4. 订单只能通过工具查询当前用户自己的订单，不要尝试查询或推测他人的订单信息；回答订单问题时只说用户问到的那笔订单，不要主动罗列其未问及的其他订单信息。\n5. 兼容性、适用性问题只引用商品详情返回的事实字段，并提醒以商品详情页为准。\n6. 回答用简体中文，简洁友好；推荐时简要说明理由，理由只能基于工具返回的事实（如销量、类目、名称），不要编造用户评价或口碑。\n7. 涉及账号归属时要说清是『你的账号/你的订单』，不要把用户的账号说成自己的账号（不要说『我账号』）。";

/// 导购设置（`config.aiGuideSettings` JSON；缺省值见 `Default`）。
#[derive(Debug, Clone)]
pub struct GuideSettings {
    /// 总开关（false = 导购停用，新回合直接给出停用提示）。
    pub enabled: bool,
    /// 单会话累计 token 预算（prompt+completion）。
    pub session_token_budget: i64,
    /// 单用户日 token 预算（北京时间自然日）。
    pub user_daily_token_budget: i64,
    /// 全局日 token 预算（北京时间自然日）。
    pub global_daily_token_budget: i64,
}

impl Default for GuideSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            session_token_budget: 60_000,
            user_daily_token_budget: 200_000,
            global_daily_token_budget: 5_000_000,
        }
    }
}

/// 导购引擎事件（路由层翻译为 SSE；事件名与运营助手对齐）。
#[derive(Debug, Clone)]
pub enum GuideEvent {
    /// 新一轮 assistant 消息已建（msgId）。
    Assistant { msg_id: i64 },
    /// 文本增量。
    Delta(String),
    /// 工具开始执行。
    ToolStart { name: String },
    /// 工具执行完毕（cards = 本轮工具产出的商品卡，路由层透传前端渲染）。
    ToolDone { name: String, ok: bool, cards: Vec<Value> },
    /// 本轮用量。
    Usage { prompt: i32, completion: i32 },
    /// 致命错误（回合结束）。
    Error(String),
    /// 回合结束。
    Done { stopped: bool },
}

/// 无命中门控触发时替换工具结果的说明文本（喂回模型 + 随 tool 行落库）。
/// 不得以 `{"error"` 开头：回合循环按此前缀判工具成败，抑制不是失败。
const NO_HIT_SUPPRESSED_TEXT: &str = "{\"suppressed\":true,\"reason\":\"本回合先前的精确搜索无命中，按『无命中不出卡』口径，本次放宽搜索/推荐的结果不予展示。请如实告知用户没有找到其所问的商品并建议更换关键词，不要提及任何商品名称或价格。\"}";

/// 回合级商品卡透出门控（用户 2026-11-21 裁决：无命中不出卡，维持验收
/// 定版标准「虚构/无命中问法 0 卡 0 价格」）。
///
/// 背景（复核 `ai-guide-probe-recheck` 定案）：无命中问法下，模型先按问法
/// 原词精确搜索（total=0），随后自行放宽到「电池/电芯」等宽词再搜，引擎
/// 把工具卡原样挂出——正文虽如实声明无此商品，界面仍出卡带价。提示词
/// 第 3 条已明文禁止放宽出卡；本门控在引擎层兜底：本回合一旦出现过
/// search_products 无命中，后续轮次的 search_products / recommend_products
/// 结果不再透出（文本替换为抑制说明、卡清空），模型无从引用其名称与价格。
///
/// 轮次内并行的一批调用视为同一次尝试：门控按批开始前的状态判定、
/// 批末统一登记无命中——避免同一批里的关键词变体互相误伤（先窄后宽的
/// 变体若命中仍可正常出卡，正常购物路径零影响）。get_product_detail 与
/// my_order_status 不是商品发现入口，不受门控。
#[derive(Debug, Default)]
struct TurnCardGate {
    /// 本回合是否已有 search_products 返回 total=0（无命中）。
    search_missed: bool,
}

impl TurnCardGate {
    /// 本批调用执行前判定：已无命中时，商品发现类工具的结果不再透出。
    fn suppresses(&self, tool: &str) -> bool {
        self.search_missed && matches!(tool, "search_products" | "recommend_products")
    }

    /// 一批调用执行完后登记：本批出现过无命中搜索则武装门控。
    fn note_search_miss(&mut self) {
        self.search_missed = true;
    }
}

/// 导购服务（public 二进制按请求从各域 Services 组装，均为 Arc 克隆）。
pub struct GuideService {
    repos: Arc<Repositories>,
    product: Arc<ProductService>,
    order: Arc<OrderService>,
    recommend: Arc<RecommendService>,
    config_client: Arc<ConfigClientService>,
}

impl GuideService {
    #[must_use]
    pub fn new(
        repos: Arc<Repositories>,
        product: Arc<ProductService>,
        order: Arc<OrderService>,
        recommend: Arc<RecommendService>,
        config_client: Arc<ConfigClientService>,
    ) -> Self {
        Self { repos, product, order, recommend, config_client }
    }

    fn pool(&self) -> &sqlx::PgPool {
        &self.repos.pool
    }

    // —— 设置与预算 ——

    /// 读导购设置（缺失/解析失败走默认）。
    pub async fn settings(&self) -> GuideSettings {
        let mut out = GuideSettings::default();
        if let Ok(rows) = self
            .repos
            .system
            .config_client
            .find_config_values(&[GUIDE_CONFIG_CODE])
            .await
        {
            if let Some(raw) = rows.first().and_then(|c| c.biz_val.as_deref()) {
                if let Ok(v) = serde_json::from_str::<Value>(raw) {
                    if let Some(b) = v.get("enabled").and_then(Value::as_bool) {
                        out.enabled = b;
                    }
                    for (key, slot) in [
                        ("sessionTokens", &mut out.session_token_budget),
                        ("userDailyTokens", &mut out.user_daily_token_budget),
                        ("globalDailyTokens", &mut out.global_daily_token_budget),
                    ] {
                        if let Some(n) = v.get(key).and_then(Value::as_i64) {
                            if n > 0 {
                                *slot = n;
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// 三级预算检查；返回命中的熔断提示（None = 放行）。
    async fn budget_block(&self, s: &GuideSettings, user_id: i32, conv_id: i32) -> Option<String> {
        // 单会话：本会话 assistant 消息 token 累计。
        let session_used: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0)::bigint
             FROM ai_guide_message WHERE conv_id = $1",
        )
        .bind(conv_id)
        .fetch_one(self.pool())
        .await
        .unwrap_or(0);
        if session_used >= s.session_token_budget {
            return Some(
                "本次导购会话的用量已达上限，请开启新会话继续咨询。".to_string(),
            );
        }
        let (day_start, day_end) = beijing_day_window(super::now_secs());
        let user_used = usage::sum_tokens(
            self.pool(),
            usage::SCENE_GUIDE,
            Some(user_id),
            day_start,
            day_end,
        )
        .await;
        if user_used >= s.user_daily_token_budget {
            return Some("今日导购用量已达上限，请明天再来咨询。".to_string());
        }
        let global_used =
            usage::sum_tokens(self.pool(), usage::SCENE_GUIDE, None, day_start, day_end).await;
        if global_used >= s.global_daily_token_budget {
            return Some("导购服务今日用量已达上限，请明天再来咨询。".to_string());
        }
        None
    }

    // —— 会话/消息持久化 ——

    /// 新建会话（渠道/模型在首回合解析后回填）。
    pub async fn create_conversation(&self, user_id: i32) -> Result<i32, String> {
        let now = super::now_secs();
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO ai_guide_conversation (user_id, title, channel_id, model_code, create_time, update_time)
             VALUES ($1, '', 0, '', $2, $2) RETURNING conv_id",
        )
        .bind(user_id)
        .bind(now)
        .fetch_one(self.pool())
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.0)
    }

    /// 我的会话列表（JSON 直出，字段与 admin 会话列表同形）。
    pub async fn list_conversations(&self, user_id: i32) -> Vec<Value> {
        let rows: Vec<ConvRow> = sqlx::query_as(
            "SELECT conv_id, user_id, title, channel_id, model_code, create_time, update_time
             FROM ai_guide_conversation WHERE user_id = $1 AND is_del = 0
             ORDER BY update_time DESC, conv_id DESC LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .unwrap_or_default();
        rows.iter()
            .map(|r| {
                json!({
                    "convId": r.conv_id,
                    "title": r.title,
                    "channelId": r.channel_id,
                    "modelCode": r.model_code,
                    "createTime": r.create_time,
                    "updateTime": r.update_time,
                })
            })
            .collect()
    }

    /// 取会话（强制本人作用域）。
    pub async fn get_conversation(&self, user_id: i32, conv_id: i32) -> Option<ConvRow> {
        sqlx::query_as(
            "SELECT conv_id, user_id, title, channel_id, model_code, create_time, update_time
             FROM ai_guide_conversation WHERE conv_id = $1 AND user_id = $2 AND is_del = 0",
        )
        .bind(conv_id)
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
        .ok()
        .flatten()
    }

    /// 逻辑删除会话（本人作用域）。
    pub async fn delete_conversation(&self, user_id: i32, conv_id: i32) -> bool {
        sqlx::query(
            "UPDATE ai_guide_conversation SET is_del = 1, update_time = $1
             WHERE conv_id = $2 AND user_id = $3 AND is_del = 0",
        )
        .bind(super::now_secs())
        .bind(conv_id)
        .bind(user_id)
        .execute(self.pool())
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
    }

    /// 会话消息列表（前端回显；tool 行带 cards 以便刷新后仍渲染商品卡）。
    pub async fn list_messages(&self, user_id: i32, conv_id: i32) -> Option<Vec<Value>> {
        self.get_conversation(user_id, conv_id).await?;
        let rows: Vec<MsgRow> = sqlx::query_as(
            "SELECT msg_id, conv_id, role, content, tool_calls, tool_name, tool_status,
                    prompt_tokens, completion_tokens, create_time
             FROM ai_guide_message WHERE conv_id = $1 ORDER BY msg_id",
        )
        .bind(conv_id)
        .fetch_all(self.pool())
        .await
        .unwrap_or_default();
        Some(
            rows.iter()
                .map(|m| {
                    json!({
                        "msgId": m.msg_id,
                        "role": m.role,
                        "content": m.content,
                        "toolCalls": m.tool_calls,
                        "toolName": m.tool_name,
                        "toolStatus": m.tool_status,
                        "createTime": m.create_time,
                    })
                })
                .collect(),
        )
    }

    async fn insert_message(
        &self,
        conv_id: i32,
        role: i32,
        content: &str,
        tool_calls: Option<Value>,
        tool_name: Option<&str>,
        tool_status: i32,
    ) -> Result<i64, String> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO ai_guide_message (conv_id, role, content, tool_calls, tool_name, tool_status, create_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING msg_id",
        )
        .bind(conv_id)
        .bind(role)
        .bind(content)
        .bind(tool_calls)
        .bind(tool_name)
        .bind(tool_status)
        .bind(super::now_secs())
        .fetch_one(self.pool())
        .await
        .map_err(|e| e.to_string())?;
        let _ = sqlx::query(
            "UPDATE ai_guide_conversation SET update_time = $1 WHERE conv_id = $2",
        )
        .bind(super::now_secs())
        .bind(conv_id)
        .execute(self.pool())
        .await;
        Ok(row.0)
    }

    async fn load_messages(&self, conv_id: i32) -> Vec<MsgRow> {
        sqlx::query_as(
            "SELECT msg_id, conv_id, role, content, tool_calls, tool_name, tool_status,
                    prompt_tokens, completion_tokens, create_time
             FROM ai_guide_message WHERE conv_id = $1 ORDER BY msg_id",
        )
        .bind(conv_id)
        .fetch_all(self.pool())
        .await
        .unwrap_or_default()
    }

    // —— 工具 ——

    /// 工具定义（OpenAI function 格式；全只读）。
    fn tool_defs() -> Vec<Value> {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "search_products",
                    "description": "按关键词搜索商城在售商品。找商品、比选、问价格都先用它。返回商品的名称、价格（按当前用户看价权限脱敏）、库存、销量与商品卡。",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "keyword": { "type": "string", "description": "搜索关键词，如商品名、品类、型号" },
                            "categoryId": { "type": "integer", "description": "可选，分类 ID" }
                        },
                        "required": ["keyword"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "get_product_detail",
                    "description": "查看某个商品的详情（价格、库存、规格事实字段）。用户追问某个商品的细节、参数、是否现货时用它。",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "productId": { "type": "integer", "description": "商品 ID（来自搜索/推荐结果）" }
                        },
                        "required": ["productId"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "recommend_products",
                    "description": "基于当前用户的购买与浏览偏好给出个性化推荐（无偏好数据时回退热销）。用户说「推荐一下」「有什么值得买」时用它。",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "limit": { "type": "integer", "description": "返回条数，默认 6，最多 10" }
                        }
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "my_order_status",
                    "description": "查询当前用户自己的订单状态。给 orderId 查单笔详情；不给则按关键词/状态列最近订单。只能查本人订单。",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "orderId": { "type": "integer", "description": "可选，订单 ID" },
                            "keyword": { "type": "string", "description": "可选，订单号/商品名关键词" },
                            "status": { "type": "integer", "description": "可选，订单状态码" }
                        }
                    }
                }
            }),
        ]
    }

    /// 执行一个只读工具。返回（喂回模型的文本, 商品卡列表, 搜索命中总数）。
    /// 搜索命中总数仅 search_products 成功时为 Some（供回合级无命中门控
    /// 判定），其余工具恒为 None。
    /// 价格红线在本层落地：商品数据先经 `price_gate` 按当前买家身份脱敏，
    /// 卡与文本同源于脱敏后的载荷，模型无从接触未脱敏价格。
    async fn execute_tool(
        &self,
        user_id: i32,
        name: &str,
        args: &Value,
    ) -> (String, Vec<Value>, Option<i64>) {
        let access = match self.config_client.price_access(Some(user_id)).await {
            Ok(a) => a,
            Err(e) => return (format!("{{\"error\":\"价格权限判定失败：{e}\"}}"), Vec::new(), None),
        };
        match name {
            "search_products" => self.tool_search(args, access).await,
            "get_product_detail" => {
                let (text, cards) = self.tool_detail(args, access).await;
                (text, cards, None)
            }
            "recommend_products" => {
                let (text, cards) = self.tool_recommend(user_id, args, access).await;
                (text, cards, None)
            }
            "my_order_status" => {
                let (text, cards) = self.tool_orders(user_id, args).await;
                (text, cards, None)
            }
            _ => (format!("{{\"error\":\"未知工具 {name}\"}}"), Vec::new(), None),
        }
    }

    async fn tool_search(
        &self,
        args: &Value,
        access: price_gate::PriceAccess,
    ) -> (String, Vec<Value>, Option<i64>) {
        let keyword = args.get("keyword").and_then(Value::as_str).unwrap_or("").trim().to_string();
        let mut qv = json!({
            "keyword": keyword,
            "productStatus": 1,
            "page": 1,
            "pageSize": 8,
        });
        if let Some(c) = args.get("categoryId").and_then(Value::as_i64) {
            qv["categoryId"] = json!(c);
        }
        let query: ProductListQuery = match serde_json::from_value(qv) {
            Ok(q) => q,
            Err(e) => return (format!("{{\"error\":\"参数错误：{e}\"}}"), Vec::new(), None),
        };
        match self.product.list(&query).await {
            Ok(page) => {
                let mut v = serde_json::to_value(&page).unwrap_or(Value::Null);
                price_gate::mask_page_items(&mut v, access);
                let total = v.get("total").and_then(Value::as_i64);
                let cards = page_cards(&v);
                let text = json!({
                    "total": v.get("total").cloned().unwrap_or(json!(0)),
                    "products": cards,
                })
                .to_string();
                (truncate_chars(&text, TOOL_RESULT_LIMIT), cards, total)
            }
            Err(e) => (format!("{{\"error\":\"搜索失败：{e}\"}}"), Vec::new(), None),
        }
    }

    async fn tool_detail(
        &self,
        args: &Value,
        access: price_gate::PriceAccess,
    ) -> (String, Vec<Value>) {
        let Some(pid) = args.get("productId").and_then(Value::as_i64) else {
            return ("{\"error\":\"缺少 productId\"}".to_string(), Vec::new());
        };
        match self.product.detail(pid as i32).await {
            Ok(vo) => {
                let mut v = serde_json::to_value(&vo).unwrap_or(Value::Null);
                if !access.can_view() {
                    price_gate::mask_price_keys(&mut v);
                }
                price_gate::stamp_price_gate(&mut v, access);
                let card = product_card(&v);
                let text = detail_text(&v);
                (truncate_chars(&text, TOOL_RESULT_LIMIT), vec![card])
            }
            Err(e) => (format!("{{\"error\":\"商品不存在或已下架：{e}\"}}"), Vec::new()),
        }
    }

    async fn tool_recommend(
        &self,
        user_id: i32,
        args: &Value,
        access: price_gate::PriceAccess,
    ) -> (String, Vec<Value>) {
        let limit = args
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(6)
            .clamp(1, 10) as u32;
        let q = CohortBuysQuery { page: Some(1), size: Some(limit), shop_id: None, days: None };
        match self.recommend.cohort_buys(user_id, &q).await {
            Ok(list) => {
                let mut cards: Vec<Value> = Vec::new();
                for vo in &list {
                    let mut v = serde_json::to_value(vo).unwrap_or(Value::Null);
                    if !access.can_view() {
                        price_gate::mask_price_keys(&mut v);
                    }
                    price_gate::stamp_price_gate(&mut v, access);
                    cards.push(recommend_card(&v));
                }
                let text = json!({ "products": cards }).to_string();
                (truncate_chars(&text, TOOL_RESULT_LIMIT), cards)
            }
            Err(e) => (format!("{{\"error\":\"推荐失败：{e}\"}}"), Vec::new()),
        }
    }

    async fn tool_orders(&self, user_id: i32, args: &Value) -> (String, Vec<Value>) {
        // 订单只查本人：OrderService 的 user 作用域在服务层强制（Some(user_id)）。
        if let Some(oid) = args.get("orderId").and_then(Value::as_i64) {
            return match self.order.detail(oid as i32, Some(user_id)).await {
                Ok(vo) => {
                    let v = serde_json::to_value(&vo).unwrap_or(Value::Null);
                    (truncate_chars(&order_projection(&v).to_string(), TOOL_RESULT_LIMIT), Vec::new())
                }
                Err(e) => (format!("{{\"error\":\"订单不存在或不属于当前用户：{e}\"}}"), Vec::new()),
            };
        }
        let status = args.get("status").and_then(Value::as_i64).map(|s| s as i32);
        let keyword = args.get("keyword").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
        let page = PageQuery { page: 1, page_size: 5 };
        match self.order.list(Some(user_id), status, None, None, keyword, &page).await {
            Ok(p) => {
                let v = serde_json::to_value(&p).unwrap_or(Value::Null);
                let items: Vec<Value> = v
                    .get("list")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().map(order_projection).collect())
                    .unwrap_or_default();
                let text = json!({ "total": v.get("total").cloned().unwrap_or(json!(0)), "orders": items }).to_string();
                (truncate_chars(&text, TOOL_RESULT_LIMIT), Vec::new())
            }
            Err(e) => (format!("{{\"error\":\"订单查询失败：{e}\"}}"), Vec::new()),
        }
    }

    // —— 引擎 ——

    /// 跑一个回合：落用户消息 → 多轮（上游流式 + 只读工具即时执行）→ 收口。
    /// 调用方（路由层）负责登录校验与 SSE 翻译；本函数据事件流回报进度。
    pub async fn run_turn(
        &self,
        user_id: i32,
        conv_id: i32,
        content: &str,
        tx: &mut futures::channel::mpsc::Sender<GuideEvent>,
    ) {
        let Some(_guard) = InflightGuard::acquire(user_id) else {
            let _ = tx
                .send(GuideEvent::Error("你有导购回复正在生成中，请等它完成后再发。".to_string()))
                .await;
            let _ = tx.send(GuideEvent::Done { stopped: false }).await;
            return;
        };
        let stop_flag = stop_flag_for(conv_id);
        stop_flag.store(false, Ordering::Relaxed);

        let Some(conv) = self.get_conversation(user_id, conv_id).await else {
            let _ = tx.send(GuideEvent::Error("会话不存在".to_string())).await;
            let _ = tx.send(GuideEvent::Done { stopped: false }).await;
            return;
        };

        // 用户消息先落库：即使随后被停用/熔断拦下，用户的话也进会话记录，
        // 提示以普通 assistant 回复形态跟在后面（回显完整）。
        if self
            .insert_message(conv_id, 1, content, None, None, 0)
            .await
            .is_err()
        {
            let _ = tx.send(GuideEvent::Error("消息保存失败".to_string())).await;
            let _ = tx.send(GuideEvent::Done { stopped: false }).await;
            return;
        }
        if conv.title.is_empty() {
            let title: String = content.chars().take(40).collect();
            let _ = sqlx::query("UPDATE ai_guide_conversation SET title = $1 WHERE conv_id = $2")
                .bind(title)
                .bind(conv_id)
                .execute(self.pool())
                .await;
        }

        let settings = self.settings().await;
        if !settings.enabled {
            self.notice(tx, conv_id, "导购功能暂未开放，敬请期待。").await;
            return;
        }
        if let Some(block) = self.budget_block(&settings, user_id, conv_id).await {
            self.notice(tx, conv_id, &block).await;
            return;
        }

        // 渠道解析：平台维度（owner=0），会话首回合解析后回填绑定。
        let (channel_id, model_code) = if conv.channel_id > 0 {
            (conv.channel_id, conv.model_code.clone())
        } else {
            match channel::resolve_channel_model(self.pool(), 0, None, None).await {
                Ok((cid, m)) => {
                    let _ = sqlx::query(
                        "UPDATE ai_guide_conversation SET channel_id = $1, model_code = $2 WHERE conv_id = $3",
                    )
                    .bind(cid)
                    .bind(&m)
                    .bind(conv_id)
                    .execute(self.pool())
                    .await;
                    (cid, m)
                }
                Err(msg) => {
                    let _ = tx.send(GuideEvent::Error(msg)).await;
                    let _ = tx.send(GuideEvent::Done { stopped: false }).await;
                    return;
                }
            }
        };

        let rows = self.load_messages(conv_id).await;
        let mut messages = rebuild_history(&rows);

        let tools = Self::tool_defs();
        // 无命中门控是回合级状态：跨轮次保留、下一回合（新用户消息）重建。
        let mut card_gate = TurnCardGate::default();
        for round in 0..MAX_ROUNDS {
            if stop_flag.load(Ordering::Relaxed) {
                let _ = tx.send(GuideEvent::Done { stopped: true }).await;
                return;
            }
            // 最后一轮不给工具，强制模型用已有事实收口作答。
            let round_tools: &[Value] = if round + 1 == MAX_ROUNDS { &[] } else { &tools };
            let assistant_id = match self.insert_message(conv_id, 2, "", None, None, 0).await {
                Ok(id) => id,
                Err(msg) => {
                    let _ = tx.send(GuideEvent::Error(msg)).await;
                    let _ = tx.send(GuideEvent::Done { stopped: false }).await;
                    return;
                }
            };
            let _ = tx.send(GuideEvent::Assistant { msg_id: assistant_id }).await;

            let Some((base_url, kind, is_enabled)) =
                channel::load_channel_endpoint(self.pool(), channel_id, 0).await
            else {
                self.fail_round(tx, assistant_id, "会话绑定的渠道已不存在").await;
                return;
            };
            if is_enabled != 1 || kind != 2 {
                self.fail_round(tx, assistant_id, "会话绑定的渠道不可用").await;
                return;
            }
            let (api_key, _) = match channel::load_channel_secret(self.pool(), channel_id).await {
                Ok(pair) => pair,
                Err(msg) => {
                    self.fail_round(tx, assistant_id, &msg).await;
                    return;
                }
            };

            // 上游增量 → 引擎事件的转发任务（delta 通道解耦）。
            let (mut dtx, mut drx) = futures::channel::mpsc::channel::<String>(64);
            let mut fwd_tx = tx.clone();
            let forwarder = tokio::spawn(async move {
                while let Some(d) = futures::StreamExt::next(&mut drx).await {
                    let _ = fwd_tx.send(GuideEvent::Delta(d)).await;
                }
            });
            let outcome = upstream::stream_completion(
                &base_url,
                &api_key,
                &model_code,
                &messages,
                round_tools,
                &stop_flag,
                assistant_id,
                &mut dtx,
            )
            .await;
            drop(dtx);
            let _ = forwarder.await;

            let outcome = match outcome {
                Ok(o) => o,
                Err(msg) => {
                    usage::record_usage(
                        self.pool(),
                        &usage::UsageRecord::guide_chat(
                            user_id, conv_id, channel_id, &model_code, 0, 0, 2, &msg,
                        ),
                    )
                    .await;
                    let _ = sqlx::query(
                        "DELETE FROM ai_guide_message WHERE msg_id = $1 AND content = ''",
                    )
                    .bind(assistant_id)
                    .execute(self.pool())
                    .await;
                    let _ = tx.send(GuideEvent::Error(msg)).await;
                    let _ = tx.send(GuideEvent::Done { stopped: false }).await;
                    return;
                }
            };

            let tool_calls_json = if outcome.calls.is_empty() {
                None
            } else {
                Some(Value::Array(
                    outcome
                        .calls
                        .iter()
                        .map(|c| json!({ "id": c.id, "name": c.name, "args": c.args }))
                        .collect(),
                ))
            };
            let _ = sqlx::query(
                "UPDATE ai_guide_message SET content = $1, tool_calls = $2, prompt_tokens = $3, completion_tokens = $4
                 WHERE msg_id = $5",
            )
            .bind(&outcome.text)
            .bind(tool_calls_json)
            .bind(outcome.prompt_tokens)
            .bind(outcome.completion_tokens)
            .bind(assistant_id)
            .execute(self.pool())
            .await;
            let status = if outcome.stopped || outcome.stream_broken { 3 } else { 1 };
            usage::record_usage(
                self.pool(),
                &usage::UsageRecord::guide_chat(
                    user_id,
                    conv_id,
                    channel_id,
                    &model_code,
                    outcome.prompt_tokens,
                    outcome.completion_tokens,
                    status,
                    if outcome.stream_broken { "上游流中断" } else { "" },
                ),
            )
            .await;
            let _ = tx
                .send(GuideEvent::Usage {
                    prompt: outcome.prompt_tokens,
                    completion: outcome.completion_tokens,
                })
                .await;

            if outcome.calls.is_empty() || outcome.stopped {
                let _ = tx.send(GuideEvent::Done { stopped: outcome.stopped }).await;
                return;
            }

            // 把本轮 assistant（含 tool_calls）补进上下文，逐个执行只读工具。
            let mut assistant_msg = json!({ "role": "assistant", "content": outcome.text });
            assistant_msg["tool_calls"] = Value::Array(
                outcome
                    .calls
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "type": "function",
                            "function": {
                                "name": c.name,
                                "arguments": c.args.to_string(),
                            }
                        })
                    })
                    .collect(),
            );
            messages.push(assistant_msg);
            let mut batch_missed = false;
            for call in &outcome.calls {
                if stop_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(GuideEvent::Done { stopped: true }).await;
                    return;
                }
                let _ = tx.send(GuideEvent::ToolStart { name: call.name.clone() }).await;
                // 门控按本批开始前的回合状态判定；本批的无命中在批末统一
                // 登记，同批并行的关键词变体互不误伤。
                let suppressed = card_gate.suppresses(&call.name);
                let (text, cards, search_total) =
                    self.execute_tool(user_id, &call.name, &call.args).await;
                if search_total == Some(0) {
                    batch_missed = true;
                }
                // 已无命中后的放宽搜索/推荐：工具照常执行（只读、无副作用），
                // 但结果不透出——文本替换为抑制说明、卡清空，落库的 tool 行
                // 即抑制标记本身，刷新回显与后续回合上下文口径一致。
                let (text, cards) = if suppressed {
                    (NO_HIT_SUPPRESSED_TEXT.to_string(), Vec::new())
                } else {
                    (text, cards)
                };
                let ok = !text.starts_with("{\"error\"");
                // cards 随 tool 行落库（tool_calls 列借位存 {callId, cards}），刷新回显不丢卡。
                let _ = self
                    .insert_message(
                        conv_id,
                        3,
                        &text,
                        Some(json!({ "callId": call.id, "cards": cards })),
                        Some(&call.name),
                        if ok { 2 } else { 4 },
                    )
                    .await;
                let _ = tx
                    .send(GuideEvent::ToolDone { name: call.name.clone(), ok, cards })
                    .await;
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": truncate_chars(&text, TOOL_RESULT_LIMIT),
                }));
            }
            if batch_missed {
                card_gate.note_search_miss();
            }
            // 回合间预算复查（单会话熔断优先在下一轮开头拦）。
            if let Some(block) = self.budget_block(&settings, user_id, conv_id).await {
                self.notice(tx, conv_id, &block).await;
                return;
            }
        }
        let _ = tx.send(GuideEvent::Done { stopped: false }).await;
    }

    /// 熔断/停用提示：落一条 assistant 消息并以普通回复形态下发。
    async fn notice(
        &self,
        tx: &mut futures::channel::mpsc::Sender<GuideEvent>,
        conv_id: i32,
        text: &str,
    ) {
        if let Ok(id) = self.insert_message(conv_id, 2, text, None, None, 0).await {
            let _ = tx.send(GuideEvent::Assistant { msg_id: id }).await;
            let _ = tx.send(GuideEvent::Delta(text.to_string())).await;
        }
        let _ = tx.send(GuideEvent::Done { stopped: false }).await;
    }

    /// 轮次前置失败（渠道不可用等）：删空行 + 错误事件收口。
    async fn fail_round(
        &self,
        tx: &mut futures::channel::mpsc::Sender<GuideEvent>,
        assistant_id: i64,
        msg: &str,
    ) {
        let _ = sqlx::query("DELETE FROM ai_guide_message WHERE msg_id = $1 AND content = ''")
            .bind(assistant_id)
            .execute(self.pool())
            .await;
        let _ = tx.send(GuideEvent::Error(msg.to_string())).await;
        let _ = tx.send(GuideEvent::Done { stopped: false }).await;
    }
}

/// 请求停止某会话的进行中回合（路由层 stop 接口调用）。
pub fn request_stop(conv_id: i32) {
    stop_flag_for(conv_id).store(true, Ordering::Relaxed);
}

// —— 行结构 ——

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConvRow {
    pub conv_id: i32,
    pub user_id: i32,
    pub title: String,
    pub channel_id: i32,
    pub model_code: String,
    pub create_time: i64,
    pub update_time: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct MsgRow {
    msg_id: i64,
    #[allow(dead_code)]
    conv_id: i32,
    role: i32,
    content: String,
    tool_calls: Option<Value>,
    tool_name: Option<String>,
    tool_status: i32,
    #[allow(dead_code)]
    prompt_tokens: i32,
    #[allow(dead_code)]
    completion_tokens: i32,
    #[allow(dead_code)]
    create_time: i64,
}

// —— 并发与停止（进程内；口径同运营助手） ——

fn stop_flags() -> &'static Mutex<HashMap<i32, Arc<AtomicBool>>> {
    static MAP: OnceLock<Mutex<HashMap<i32, Arc<AtomicBool>>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn stop_flag_for(conv_id: i32) -> Arc<AtomicBool> {
    stop_flags()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .entry(conv_id)
        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

fn inflight_counts() -> &'static Mutex<HashMap<i32, usize>> {
    static MAP: OnceLock<Mutex<HashMap<i32, usize>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

struct InflightGuard {
    user_id: i32,
}

impl InflightGuard {
    fn acquire(user_id: i32) -> Option<Self> {
        let mut map = inflight_counts().lock().unwrap_or_else(|p| p.into_inner());
        let n = map.entry(user_id).or_insert(0);
        if *n >= MAX_INFLIGHT_PER_USER {
            return None;
        }
        *n += 1;
        Some(Self { user_id })
    }
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        let mut map = inflight_counts().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(n) = map.get_mut(&self.user_id) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                map.remove(&self.user_id);
            }
        }
    }
}

// —— 上下文重建 ——

/// 从消息行重建上游上下文（形态与运营助手 rebuild_history 同构；
/// 导购无待确认调用，故不返回 pending）。
fn rebuild_history(rows: &[MsgRow]) -> Vec<Value> {
    let mut tool_rows: HashMap<String, &MsgRow> = HashMap::new();
    for r in rows.iter().filter(|r| r.role == 3) {
        if let Some(call_id) = r
            .tool_calls
            .as_ref()
            .and_then(|tc| tc.get("callId"))
            .and_then(Value::as_str)
        {
            tool_rows.insert(call_id.to_string(), r);
        }
    }
    let mut messages: Vec<Value> = vec![json!({
        "role": "system",
        "content": GUIDE_SYSTEM_PROMPT,
    })];
    let answered: HashSet<String> = tool_rows.keys().cloned().collect();
    for r in rows {
        match r.role {
            1 => messages.push(json!({ "role": "user", "content": r.content })),
            2 => {
                let mut msg = json!({ "role": "assistant", "content": r.content });
                if let Some(calls) = r.tool_calls.as_ref().and_then(Value::as_array) {
                    // 只回放已有结果行的调用，未答调用（中断回合）不回放防上游报错。
                    let done_calls: Vec<&Value> = calls
                        .iter()
                        .filter(|c| {
                            c.get("id")
                                .and_then(Value::as_str)
                                .is_some_and(|id| answered.contains(id))
                        })
                        .collect();
                    if !done_calls.is_empty() {
                        msg["tool_calls"] = Value::Array(
                            done_calls
                                .iter()
                                .map(|c| {
                                    json!({
                                        "id": c.get("id").cloned().unwrap_or(Value::Null),
                                        "type": "function",
                                        "function": {
                                            "name": c.get("name").cloned().unwrap_or(Value::Null),
                                            "arguments": c.get("args").map(|a| a.to_string()).unwrap_or_else(|| "{}".to_string()),
                                        }
                                    })
                                })
                                .collect(),
                        );
                    }
                }
                messages.push(msg);
            }
            3 => {
                let call_id = r
                    .tool_calls
                    .as_ref()
                    .and_then(|tc| tc.get("callId"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let content = match r.tool_status {
                    2 => truncate_chars(&r.content, TOOL_RESULT_LIMIT),
                    4 => format!("执行失败：{}", truncate_chars(&r.content, 500)),
                    _ => "该操作未执行。".to_string(),
                };
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": content,
                }));
            }
            _ => {}
        }
    }
    // 上下文预算：粗略按字符数保留 system + 最近消息（口径同运营助手）。
    let budget = 120_000usize;
    let total: usize = messages.iter().map(|m| m.to_string().len()).sum();
    if total > budget {
        let mut kept: Vec<Value> = vec![messages[0].clone()];
        let mut acc = kept[0].to_string().len();
        let mut tail: Vec<Value> = Vec::new();
        for m in messages.iter().skip(1).rev() {
            let len = m.to_string().len();
            if acc + len > budget && !tail.is_empty() {
                break;
            }
            acc += len;
            tail.push(m.clone());
        }
        tail.reverse();
        kept.extend(tail);
        return kept;
    }
    messages
}

// —— 载荷投影（卡与模型文本同源） ——

/// 分页载荷 → 商品卡列表（载荷须已脱敏；list/records 双键兼容同 price_gate）。
fn page_cards(page: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for key in ["list", "records"] {
        if let Some(arr) = page.get(key).and_then(Value::as_array) {
            for item in arr {
                out.push(product_card(item));
            }
            if !out.is_empty() {
                break;
            }
        }
    }
    out
}

/// 商品卡（前端渲染 + 模型引用同源；价格字段已按门控口径处理）。
fn product_card(v: &Value) -> Value {
    json!({
        "productId": v.get("productId").cloned().unwrap_or(Value::Null),
        "productName": v.get("productName").cloned().unwrap_or(Value::Null),
        "picUrl": v.get("picUrl").cloned().unwrap_or(Value::Null),
        "picThumb": v.get("picThumb").cloned().unwrap_or(Value::Null),
        "shopId": v.get("shopId").cloned().unwrap_or(Value::Null),
        "productPrice": v.get("productPrice").cloned().unwrap_or(Value::Null),
        "marketPrice": v.get("marketPrice").cloned().unwrap_or(Value::Null),
        "priceVisible": v.get("priceVisible").cloned().unwrap_or(json!(true)),
        "priceGate": v.get("priceGate").cloned().unwrap_or(json!("none")),
        "productStock": v.get("productStock").cloned().unwrap_or(Value::Null),
        "sales": v.get("sales").cloned().unwrap_or(Value::Null),
    })
}

/// 推荐卡（RecommendProductVO 字段名与商品 VO 有别，单独投影）。
fn recommend_card(v: &Value) -> Value {
    json!({
        "productId": v.get("productId").cloned().unwrap_or(Value::Null),
        "productName": v.get("productName").cloned().unwrap_or(Value::Null),
        "picUrl": v.get("picUrl").cloned().unwrap_or(Value::Null),
        "picThumb": v.get("picThumb").cloned().unwrap_or(Value::Null),
        "shopId": v.get("shopId").cloned().unwrap_or(Value::Null),
        "productPrice": v.get("productPrice").cloned().unwrap_or(Value::Null),
        "priceVisible": v.get("priceVisible").cloned().unwrap_or(json!(true)),
        "priceGate": v.get("priceGate").cloned().unwrap_or(json!("none")),
        "productStock": v.get("productStock").cloned().unwrap_or(Value::Null),
        "sales": v.get("virtualSales").cloned().unwrap_or(Value::Null),
    })
}

/// 详情工具文本：卡字段 + 事实字段（下架/询价态等），不带任何未脱敏价格。
fn detail_text(v: &Value) -> String {
    let mut out = product_card(v).as_object().cloned().unwrap_or_default();
    for key in ["productSn", "categoryId", "brandId", "isInquiry", "inquiryState", "productStatus"] {
        if let Some(x) = v.get(key) {
            out.insert(key.to_string(), x.clone());
        }
    }
    Value::Object(out).to_string()
}

/// 订单投影（白名单字段；金额为买家本人已付/应付，属其自身数据）。
fn order_projection(v: &Value) -> Value {
    let items: Vec<Value> = v
        .get("items")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|it| {
                    json!({
                        "productId": it.get("productId").cloned().unwrap_or(Value::Null),
                        "productName": it.get("productName").cloned().unwrap_or(Value::Null),
                        "picUrl": it.get("picUrl").cloned().unwrap_or(Value::Null),
                        "qty": it.get("qty").cloned().unwrap_or(it.get("quantity").cloned().unwrap_or(Value::Null)),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "orderId": v.get("orderId").cloned().unwrap_or(Value::Null),
        "orderSn": v.get("orderSn").cloned().unwrap_or(Value::Null),
        "orderStatus": v.get("orderStatus").cloned().unwrap_or(Value::Null),
        "orderStatusName": v.get("orderStatusName").cloned().unwrap_or(Value::Null),
        "payStatus": v.get("payStatus").cloned().unwrap_or(Value::Null),
        "shippingStatus": v.get("shippingStatus").cloned().unwrap_or(Value::Null),
        "totalAmount": v.get("totalAmount").cloned().unwrap_or(Value::Null),
        "paidAmount": v.get("paidAmount").cloned().unwrap_or(Value::Null),
        "addTime": v.get("addTime").cloned().unwrap_or(Value::Null),
        "items": items,
    })
}

/// 北京时间自然日窗口 `[start, end)`（epoch 秒；预算「日」口径）。
fn beijing_day_window(now: i64) -> (i64, i64) {
    const OFFSET: i64 = 8 * 3600;
    let local = now + OFFSET;
    let start_local = (local / 86_400) * 86_400;
    (start_local - OFFSET, start_local - OFFSET + 86_400)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("…（已截断）");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beijing_day_window_is_aligned() {
        // 1763670600 = 2025-11-21 04:30 CST；当日窗口起点应为
        // 2025-11-21 00:00 CST = 2025-11-20 16:00 UTC = 1763654400。
        let (start, end) = beijing_day_window(1_763_670_600);
        assert_eq!(end - start, 86_400);
        assert_eq!(start, 1_763_654_400);
    }

    #[test]
    fn product_card_keeps_gate_markers() {
        let v = json!({
            "productId": 7, "productName": "x", "productPrice": null,
            "priceVisible": false, "priceGate": "certification",
        });
        let card = product_card(&v);
        assert_eq!(card["priceVisible"], json!(false));
        assert_eq!(card["priceGate"], json!("certification"));
        assert!(card["productPrice"].is_null());
    }

    #[test]
    fn system_prompt_states_price_red_lines() {
        assert!(GUIDE_SYSTEM_PROMPT.contains("逐字引用"));
        assert!(GUIDE_SYSTEM_PROMPT.contains("绝不猜测"));
        assert!(GUIDE_SYSTEM_PROMPT.contains("不要虚构商品"));
    }

    #[test]
    fn system_prompt_states_no_hit_no_card_rules() {
        // 无命中不出卡（用户 2026-11-21 裁决）：禁止自行放宽关键词凑结果、
        // 展示须正文逐项点名（治卡文脱节）。
        assert!(GUIDE_SYSTEM_PROMPT.contains("不得自行改用更宽泛的关键词"));
        assert!(GUIDE_SYSTEM_PROMPT.contains("不得出现正文未提及的商品卡"));
        // 话术收敛：订单只答所问、人称归属说清。
        assert!(GUIDE_SYSTEM_PROMPT.contains("不要主动罗列其未问及的其他订单信息"));
        assert!(GUIDE_SYSTEM_PROMPT.contains("不要把用户的账号说成自己的账号"));
    }

    #[test]
    fn card_gate_suppresses_discovery_tools_only_after_miss() {
        let mut gate = TurnCardGate::default();
        assert!(!gate.suppresses("search_products"));
        assert!(!gate.suppresses("recommend_products"));
        gate.note_search_miss();
        assert!(gate.suppresses("search_products"));
        assert!(gate.suppresses("recommend_products"));
        // 非商品发现入口不受门控：详情追问与订单查询照常。
        assert!(!gate.suppresses("get_product_detail"));
        assert!(!gate.suppresses("my_order_status"));
    }

    #[test]
    fn suppressed_text_is_not_an_error_payload() {
        // 回合循环按 {"error" 前缀判工具成败；抑制说明必须不触发失败分支，
        // 且自带 suppressed 标记供落库审计。
        assert!(!NO_HIT_SUPPRESSED_TEXT.starts_with("{\"error\""));
        assert!(NO_HIT_SUPPRESSED_TEXT.contains("\"suppressed\":true"));
    }
}
