//! 外卖聚合（takeout）DTO / 归一模型 / 纯逻辑（我方新增）。
//!
//! 论坛 C 类第 ③ 项（帖 697900 / 127866 / 417810），设计见
//! `~/workspace/takeout-aggregation/DESIGN.md`。一期：A 向「订单汇入」
//! 聚合框架 + 美团闪购直连；二期（2026-11-21）：淘宝闪购（饿了么）与
//! 抖音生活服务直连补齐，三渠道同深度；B 向（麦芽田配送聚合）只在
//! 服务层留接口位。
//!
//! # 口径（设计 §1.4 共性结论）
//! - 三家渠道都是「ISV 应用 + 商家逐店授权」，凭据走 `config` 表
//!   `biz_code = "takeoutSettings"` 整块 JSON，未配置 = 诚实未启用；
//! - 外部订单资金不过我方：落内部单时 `order_type = 13` + `external_channel`
//!   标记，排除在平台结算 / 分销佣金 / 礼品卡 / 主动退款之外；
//! - 退款主导权在渠道侧：我方只同步状态，不走财务审核出款链；
//! - 金额一律以渠道推送为准落库，不按我方口径重算；美团/抖音金额单位
//!   为**分**（[`fen_to_yuan`]），饿了么 API 金额为**元（浮点）**
//!   （[`yuan_amount`]），在本层统一换算为元。

use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// 常量
// ---------------------------------------------------------------------------

/// 渠道：美团闪购（一期已接入）。
pub const CHANNEL_MEITUAN: &str = "meituan";
/// 渠道：淘宝闪购（饿了么）——二期已接入（NOP 协议 + 轻推送回查补全）。
pub const CHANNEL_ELEME: &str = "eleme";
/// 渠道：抖音生活服务——二期已接入（团购券核销模型，与配送订单不同构：
/// paid = 待核销 / completed = 已核销，见服务层 douyin 适配器）。
pub const CHANNEL_DOUYIN: &str = "douyin";
/// 全部已登记渠道（注册表顺序即后台展示顺序）。
pub const CHANNELS: [&str; 3] = [CHANNEL_MEITUAN, CHANNEL_ELEME, CHANNEL_DOUYIN];

/// `config` 表业务编码（整块 JSON 型配置，与 `buyStyle` / `adSettings` 同型）。
pub const TAKEOUT_SETTINGS_CODE: &str = "takeoutSettings";

/// 内部订单类型：外部渠道单（1 普通 / 2 拼团 / 9 预约到店 / 10 预约上门 /
/// 11 拍卖 / 12 礼品卡，13 为本功能新增，见迁移 030）。
pub const ORDER_TYPE_EXTERNAL: i32 = 13;

/// 渠道展示名。
#[must_use]
pub fn channel_name(channel: &str) -> &'static str {
    match channel {
        CHANNEL_MEITUAN => "美团闪购",
        CHANNEL_ELEME => "淘宝闪购",
        CHANNEL_DOUYIN => "抖音生活服务",
        _ => "未知渠道",
    }
}

/// 该渠道是否有真实适配器（二期起三渠道均已接入；未知渠道为 false）。
#[must_use]
pub fn channel_adapter_ready(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

// ---------------------------------------------------------------------------
// 渠道配置（config.biz_code = "takeoutSettings"）
// ---------------------------------------------------------------------------

/// 单渠道配置（凭据位）。`app_secret` 只进不出：后台读取接口必须脱敏
/// （见服务层 `config_view`），保存时空串表示「不修改」由服务层处理。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConfig {
    /// 渠道总开关（关 = 回调直接拒绝、后台显示未启用）。
    pub enabled: bool,
    /// 开放平台应用 id。
    pub app_id: String,
    /// 开放平台应用密钥。
    pub app_secret: String,
    /// API 网关根地址（可配，沙箱/正式切换；空 = 用适配器内置默认）。
    pub gateway_base: String,
}

impl ChannelConfig {
    /// 凭据是否齐备（app_id + app_secret 都非空）。
    #[must_use]
    pub fn configured(&self) -> bool {
        !self.app_id.trim().is_empty() && !self.app_secret.trim().is_empty()
    }

    /// 渠道生效状态：开关开且凭据齐备。
    #[must_use]
    pub fn active(&self) -> bool {
        self.enabled && self.configured()
    }
}

/// 全部渠道配置。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TakeoutSettings {
    /// 美团闪购。
    pub meituan: ChannelConfig,
    /// 淘宝闪购（app_id 位填饿了么开放平台 app_key）。
    pub eleme: ChannelConfig,
    /// 抖音生活服务（app_id 位填抖音开放平台 client_key）。
    pub douyin: ChannelConfig,
}

impl TakeoutSettings {
    /// 按渠道 id 取配置（未知渠道返回 None）。
    #[must_use]
    pub fn channel(&self, channel: &str) -> Option<&ChannelConfig> {
        match channel {
            CHANNEL_MEITUAN => Some(&self.meituan),
            CHANNEL_ELEME => Some(&self.eleme),
            CHANNEL_DOUYIN => Some(&self.douyin),
            _ => None,
        }
    }

    /// 按渠道 id 取可变配置。
    pub fn channel_mut(&mut self, channel: &str) -> Option<&mut ChannelConfig> {
        match channel {
            CHANNEL_MEITUAN => Some(&mut self.meituan),
            CHANNEL_ELEME => Some(&mut self.eleme),
            CHANNEL_DOUYIN => Some(&mut self.douyin),
            _ => None,
        }
    }
}

fn clean_text(v: Option<&Value>, max_chars: usize) -> String {
    v.and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .chars()
        .take(max_chars)
        .collect()
}

fn sanitize_channel(input: Option<&Value>) -> ChannelConfig {
    let Some(obj) = input.and_then(Value::as_object) else {
        return ChannelConfig::default();
    };
    ChannelConfig {
        enabled: obj.get("enabled").and_then(Value::as_bool).unwrap_or(false),
        app_id: clean_text(obj.get("appId"), 64),
        app_secret: clean_text(obj.get("appSecret"), 128),
        gateway_base: clean_text(obj.get("gatewayBase"), 255),
    }
}

/// 清洗一份外卖聚合配置（保存与读取共用）：只保留三渠道白名单字段，
/// 文本截断，输入非对象回落全默认（全未启用）。
#[must_use]
pub fn sanitize_takeout_settings(input: &Value) -> TakeoutSettings {
    let Some(obj) = input.as_object() else {
        return TakeoutSettings::default();
    };
    TakeoutSettings {
        meituan: sanitize_channel(obj.get("meituan")),
        eleme: sanitize_channel(obj.get("eleme")),
        douyin: sanitize_channel(obj.get("douyin")),
    }
}

/// 由存储的原始 JSON 文本得到生效配置：解析失败/非对象 ⇒ 默认（全未启用）。
#[must_use]
pub fn effective_takeout_settings(stored: Option<&str>) -> TakeoutSettings {
    let Some(raw) = stored else {
        return TakeoutSettings::default();
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(v) if v.is_object() => sanitize_takeout_settings(&v),
        _ => TakeoutSettings::default(),
    }
}

/// 配置转存储 JSON（`appSecret` 原样落库；脱敏只发生在读接口）。
#[must_use]
pub fn settings_to_value(settings: &TakeoutSettings) -> Value {
    fn ch(c: &ChannelConfig) -> Value {
        json!({
            "enabled": c.enabled,
            "appId": c.app_id,
            "appSecret": c.app_secret,
            "gatewayBase": c.gateway_base,
        })
    }
    json!({
        "meituan": ch(&settings.meituan),
        "eleme": ch(&settings.eleme),
        "douyin": ch(&settings.douyin),
    })
}

/// 配置的后台展示形态：密钥只回「是否已配置」，永不回明文。
#[must_use]
pub fn settings_view(settings: &TakeoutSettings) -> Value {
    fn ch(channel: &str, c: &ChannelConfig) -> Value {
        json!({
            "channel": channel,
            "channelName": channel_name(channel),
            "adapterReady": channel_adapter_ready(channel),
            "enabled": c.enabled,
            "appId": c.app_id,
            "appSecretConfigured": !c.app_secret.trim().is_empty(),
            "gatewayBase": c.gateway_base,
            "configured": c.configured(),
            "active": c.active(),
        })
    }
    json!({
        "channels": [
            ch(CHANNEL_MEITUAN, &settings.meituan),
            ch(CHANNEL_ELEME, &settings.eleme),
            ch(CHANNEL_DOUYIN, &settings.douyin),
        ]
    })
}

// ---------------------------------------------------------------------------
// 归一订单模型（渠道推送 → 本层归一 → 服务层落库）
// ---------------------------------------------------------------------------

/// 归一订单状态（存 `takeout_external_order.status`）。
pub const EXT_STATUS_PAID: &str = "paid";
/// 已确认（商家接单）。
pub const EXT_STATUS_CONFIRMED: &str = "confirmed";
/// 已完成。
pub const EXT_STATUS_COMPLETED: &str = "completed";
/// 已取消。
pub const EXT_STATUS_CANCELLED: &str = "cancelled";
/// 退款中（渠道侧发起，我方只同步）。
pub const EXT_STATUS_REFUNDING: &str = "refunding";
/// 已退款（渠道侧执行完毕）。
pub const EXT_STATUS_REFUNDED: &str = "refunded";
/// 门店未绑定（异常兜底态：已落外部单、未生成内部单，待补绑后重处理）。
pub const EXT_STATUS_PENDING_BINDING: &str = "pending_binding";

/// 美团订单状态码 → 归一状态（官方口径：1 已提交 / 2 已推送商家 /
/// 4 已确认 / 8 已完成 / 9 已取消；已支付订单推送时通常为 2）。
#[must_use]
pub fn meituan_status_normalize(raw: i32) -> Option<&'static str> {
    match raw {
        1 | 2 => Some(EXT_STATUS_PAID),
        4 => Some(EXT_STATUS_CONFIRMED),
        8 => Some(EXT_STATUS_COMPLETED),
        9 => Some(EXT_STATUS_CANCELLED),
        _ => None,
    }
}

/// 归一状态 → 我方订单状态（`order.order_status`：
/// 0 待确定 / 1 已确定待发货 / 2 处理中 / 3 已取消 / 5 已完成）。
#[must_use]
pub fn internal_order_status(ext: &str) -> i32 {
    match ext {
        EXT_STATUS_CONFIRMED => 2,
        EXT_STATUS_COMPLETED => 5,
        EXT_STATUS_CANCELLED | EXT_STATUS_REFUNDED => 3,
        _ => 1,
    }
}

/// 归一状态 → 我方支付状态（`order.pay_status`：
/// 0 未支付 / 2 已支付 / 3 退款中 / 4 已退款）。外部单恒为渠道侧已结算，
/// 落库只表达状态镜像，资金不经我方（设计 §1.4.2）。
#[must_use]
pub fn internal_pay_status(ext: &str) -> i32 {
    match ext {
        EXT_STATUS_REFUNDING => 3,
        EXT_STATUS_REFUNDED => 4,
        EXT_STATUS_CANCELLED => 4,
        _ => 2,
    }
}

/// 渠道金额（分）→ 元。美团推送/接口、抖音生活服务金额单位为分，
/// 落库统一为元。
#[must_use]
pub fn fen_to_yuan(fen: i64) -> Decimal {
    Decimal::new(fen, 2)
}

/// 淘宝闪购（饿了么）订单状态字符串 → 归一状态。依据饿了么开放
/// 平台官方 SDK 订单实体（OOrder）的状态枚举：`status` ∈
/// unpaid 未支付 / unprocessed 待接单 / valid 已生效（已确认）/
/// invalid 已取消 / settled 已完成；`refundStatus` ∈ noRefund
/// 无退款 / applied 已申请 / arbitrating 仲裁中 / rejected 已驳回 /
/// failed 退款失败 / successful 退款成功。退款进程优先于订单状态
/// 表达（与美团退款单独推送的口径等价）；rejected / failed /
/// noRefund 不改变订单状态。unpaid 与未知值 → None（适配器侧
/// 保守处理：未支付单不落履约，未知状态不冒认）。
#[must_use]
pub fn eleme_status_normalize(status: &str, refund_status: &str) -> Option<&'static str> {
    match refund_status.trim().to_ascii_lowercase().as_str() {
        "applied" | "arbitrating" => return Some(EXT_STATUS_REFUNDING),
        "successful" => return Some(EXT_STATUS_REFUNDED),
        _ => {}
    }
    match status.trim().to_ascii_lowercase().as_str() {
        "unprocessed" => Some(EXT_STATUS_PAID),
        "valid" => Some(EXT_STATUS_CONFIRMED),
        "settled" => Some(EXT_STATUS_COMPLETED),
        "invalid" => Some(EXT_STATUS_CANCELLED),
        _ => None,
    }
}

/// 抖音生活服务订单通知 `action` → 归一状态。依据官方 WebHooks
/// 文档示例（`action = "pay_success"`，content.order 载荷）与交易
/// 系统动作命名约定做包含匹配：含 pay → 已支付（团购口径 = 待
/// 核销）；含 verify / fulfil / consume → 已完成（已核销）；含
/// refund → 退款中，带 success / finish / complete 后缀 → 已退款；
/// 含 cancel / close / expire → 已取消（过期未核销后平台自动退款，
/// 后续 refund 动作再推进到 refunded）。未知动作 → None。
#[must_use]
pub fn douyin_action_normalize(action: &str) -> Option<&'static str> {
    let a = action.trim().to_ascii_lowercase();
    if a.is_empty() {
        return None;
    }
    if a.contains("refund") {
        if a.contains("success") || a.contains("finish") || a.contains("complete") {
            return Some(EXT_STATUS_REFUNDED);
        }
        return Some(EXT_STATUS_REFUNDING);
    }
    if a.contains("pay") {
        return Some(EXT_STATUS_PAID);
    }
    if a.contains("verif") || a.contains("fulfil") || a.contains("consume") {
        return Some(EXT_STATUS_COMPLETED);
    }
    if a.contains("cancel") || a.contains("close") || a.contains("expire") {
        return Some(EXT_STATUS_CANCELLED);
    }
    None
}

/// 渠道金额（元，浮点）→ 元 Decimal。饿了么 API 金额为元的浮点数
/// （如 12.5），经两位小数格式化再解析，避免二进制浮点尾差直接
/// 落库；非有限值回落 0。
#[must_use]
pub fn yuan_amount(value: f64) -> Decimal {
    if !value.is_finite() {
        return Decimal::new(0, 0);
    }
    Decimal::from_str(&format!("{value:.2}")).unwrap_or_else(|_| Decimal::new(0, 0))
}

/// 归一订单行（渠道商品行 + 映射结果位）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrderItem {
    /// 渠道商品 id（spu 级）。
    pub external_spu_id: String,
    /// 渠道 SKU id。
    pub external_sku_id: String,
    /// 渠道商品名（原样，未映射时以此落单）。
    pub name: String,
    /// 数量。
    pub quantity: i32,
    /// 单价（元，渠道口径）。
    pub price: Decimal,
    /// 映射到的我方商品（0 = 未映射）。
    pub product_id: i32,
    /// 映射到的我方 SKU（0 = 未映射/无规格）。
    pub sku_id: i32,
}

/// 归一外部订单（渠道无关形态）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrder {
    /// 渠道订单号。
    pub external_order_id: String,
    /// 我方侧门店编码（绑定键）。
    pub app_poi_code: String,
    /// 渠道原始状态码。
    pub external_status: i32,
    /// 归一状态（[`meituan_status_normalize`] 等渠道映射的结果）。
    pub status: String,
    /// 商品行。
    pub items: Vec<ExternalOrderItem>,
    /// 总额（元，渠道口径原样）。
    pub total_amount: Decimal,
    /// 商品总额（元）。
    pub product_amount: Decimal,
    /// 配送费（元）。
    pub shipping_fee: Decimal,
    /// 收货人（渠道侧，可能为隐私保护名）。
    pub consignee: String,
    /// 手机号（渠道隐私号原样）。
    pub mobile: String,
    /// 收货地址全文。
    pub address: String,
    /// 买家备注。
    pub remark: String,
}

// ---------------------------------------------------------------------------
// 表单解析（渠道回调为 application/x-www-form-urlencoded）
// ---------------------------------------------------------------------------

/// 单次 percent-decode（`%XX` → 字节，`+` → 空格）；非法序列原样保留。
/// 美团 POST 推送验签要求参数值解码 2 次（官方《签名规则》注意事项 6），
/// 本函数供调用方叠加调用，GET 推送只解 1 次。
#[must_use]
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = |b: u8| -> Option<u8> {
                    match b {
                        b'0'..=b'9' => Some(b - b'0'),
                        b'a'..=b'f' => Some(b - b'a' + 10),
                        b'A'..=b'F' => Some(b - b'A' + 10),
                        _ => None,
                    }
                };
                if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    out.push(h * 16 + l);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 解析 form-urlencoded 原文为键值对（保持出现顺序；键与值各解码
/// `decode_rounds` 次）。同名键保留全部出现（验签按官方口径以原始
/// 参数集为准，取值时用 [`form_get`] 取第一个）。
#[must_use]
pub fn parse_form_pairs(raw: &str, decode_rounds: usize) -> Vec<(String, String)> {
    let decode = |s: &str| -> String {
        let mut cur = s.to_string();
        for _ in 0..decode_rounds.max(1) {
            cur = percent_decode(&cur);
        }
        cur
    };
    raw.split('&')
        .filter(|seg| !seg.is_empty())
        .map(|seg| {
            let (k, v) = seg.split_once('=').unwrap_or((seg, ""));
            (decode(k), decode(v))
        })
        .collect()
}

/// 取表单中某键的第一个值。
#[must_use]
pub fn form_get<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

// ---------------------------------------------------------------------------
// 后台 DTO / VO
// ---------------------------------------------------------------------------

/// 渠道配置保存入参（`appSecret` 空串 = 保持原值不变，服务层处理）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConfigSaveDTO {
    /// 渠道 id。
    pub channel: String,
    /// 开关。
    pub enabled: bool,
    /// 应用 id。
    pub app_id: String,
    /// 应用密钥（空 = 不修改）。
    pub app_secret: String,
    /// 网关根地址。
    pub gateway_base: String,
}

/// 门店绑定保存入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingSaveDTO {
    /// 绑定 id（0 = 新增）。
    pub id: i32,
    /// 渠道 id。
    pub channel: String,
    /// 我方门店 id（店铺账号强制本店，服务层校验）。
    pub shop_id: i32,
    /// 我方侧门店编码（绑定键）。
    pub app_poi_code: String,
    /// 渠道侧门店 id。
    pub external_poi_id: String,
    /// 渠道侧门店名。
    pub external_poi_name: String,
    /// 授权状态：0 未授权 / 1 已授权 / 2 已失效。
    pub auth_status: i32,
    /// 商家授权令牌（空 = 不修改，服务层处理）。
    pub access_token: String,
    /// 新单自动确认。
    pub auto_accept: bool,
    /// 启用。
    pub enabled: bool,
}

/// SKU 映射保存入参。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingSaveDTO {
    /// 映射 id（0 = 新增）。
    pub id: i32,
    /// 渠道 id。
    pub channel: String,
    /// 我方门店 id。
    pub shop_id: i32,
    /// 渠道商品 id。
    pub external_spu_id: String,
    /// 渠道 SKU id。
    pub external_sku_id: String,
    /// 渠道商品名快照。
    pub external_name: String,
    /// 我方商品 id。
    pub product_id: i32,
    /// 我方 SKU id。
    pub sku_id: i32,
}

/// 删除入参（绑定 / 映射共用）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdDTO {
    /// 记录 id。
    pub id: i32,
}

/// 外部订单列表查询。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrderListQuery {
    /// 渠道过滤（空 = 全部）。
    pub channel: Option<String>,
    /// 归一状态过滤（空 = 全部）。
    pub status: Option<String>,
    /// 关键词（渠道订单号 / 手机号）。
    pub keyword: Option<String>,
    /// 页码（从 1 起）。
    pub page: Option<i64>,
    /// 每页条数。
    pub page_size: Option<i64>,
}

/// 外部订单行 VO（后台列表/详情；raw_payload 只在详情回）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOrderVO {
    /// 外部单 id。
    pub id: i32,
    /// 渠道。
    pub channel: String,
    /// 渠道名。
    pub channel_name: String,
    /// 渠道订单号。
    pub external_order_id: String,
    /// 门店 id。
    pub shop_id: i32,
    /// 内部订单 id（0 = 未生成）。
    pub internal_order_id: i32,
    /// 渠道原始状态码。
    pub external_status: i32,
    /// 归一状态。
    pub status: String,
    /// 总额（元）。
    pub total_amount: Decimal,
    /// 收货人。
    pub consignee: String,
    /// 手机号。
    pub mobile: String,
    /// 地址。
    pub address: String,
    /// 最近事件。
    pub last_event: String,
    /// 最近错误（兜底信息，空 = 无）。
    pub last_error: String,
    /// 创建时间（秒）。
    pub add_time: i32,
    /// 更新时间（秒）。
    pub update_time: i32,
    /// 归一载荷 JSON（仅详情）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_payload: Option<String>,
}

/// 校验绑定保存入参（纯函数，返回错误文案）。
#[must_use]
pub fn validate_binding(dto: &BindingSaveDTO) -> Option<&'static str> {
    if !CHANNELS.contains(&dto.channel.as_str()) {
        return Some("渠道有误");
    }
    if dto.app_poi_code.trim().is_empty() {
        return Some("门店编码（app_poi_code）不能为空");
    }
    if !(0..=2).contains(&dto.auth_status) {
        return Some("授权状态有误");
    }
    None
}

/// 校验映射保存入参（纯函数，返回错误文案）。
#[must_use]
pub fn validate_mapping(dto: &MappingSaveDTO) -> Option<&'static str> {
    if !CHANNELS.contains(&dto.channel.as_str()) {
        return Some("渠道有误");
    }
    if dto.external_sku_id.trim().is_empty() {
        return Some("渠道 SKU 编号不能为空");
    }
    if dto.product_id <= 0 {
        return Some("请选择我方商品");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_all_inactive() {
        let s = effective_takeout_settings(None);
        assert!(!s.meituan.active());
        assert!(!s.meituan.configured());
    }

    #[test]
    fn settings_sanitize_keeps_only_whitelist() {
        let v = json!({
            "meituan": {"enabled": true, "appId": " 123 ", "appSecret": "s3cr3t", "gatewayBase": "https://example.com", "evil": "x"},
            "eleme": {"enabled": true}
        });
        let s = sanitize_takeout_settings(&v);
        assert!(s.meituan.active());
        assert_eq!(s.meituan.app_id, "123");
        assert!(!s.eleme.configured());
        let stored = settings_to_value(&s);
        assert!(stored["meituan"].get("evil").is_none());
    }

    #[test]
    fn settings_view_never_leaks_secret() {
        let mut s = TakeoutSettings::default();
        s.meituan.app_secret = "topsecret".into();
        let view = settings_view(&s);
        let text = view.to_string();
        assert!(!text.contains("topsecret"));
        assert_eq!(view["channels"][0]["appSecretConfigured"], json!(true));
    }

    #[test]
    fn meituan_status_map_matches_official_codes() {
        assert_eq!(meituan_status_normalize(1), Some(EXT_STATUS_PAID));
        assert_eq!(meituan_status_normalize(2), Some(EXT_STATUS_PAID));
        assert_eq!(meituan_status_normalize(4), Some(EXT_STATUS_CONFIRMED));
        assert_eq!(meituan_status_normalize(8), Some(EXT_STATUS_COMPLETED));
        assert_eq!(meituan_status_normalize(9), Some(EXT_STATUS_CANCELLED));
        assert_eq!(meituan_status_normalize(3), None);
    }

    #[test]
    fn fen_conversion() {
        assert_eq!(fen_to_yuan(12345), Decimal::new(12345, 2));
        assert_eq!(fen_to_yuan(0), Decimal::new(0, 0));
    }

    #[test]
    fn percent_decode_rounds() {
        // 一次解码：%E5%BA%97 → 店；双重编码需两次。
        assert_eq!(percent_decode("%E5%BA%97"), "店");
        let once = percent_decode("%25E5%25BA%2597");
        assert_eq!(once, "%E5%BA%97");
        assert_eq!(percent_decode(&once), "店");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn form_pairs_double_decode() {
        let raw = "app_poi_code=%25E5%25BA%2597&status=2&empty=";
        let pairs = parse_form_pairs(raw, 2);
        assert_eq!(form_get(&pairs, "app_poi_code"), Some("店"));
        assert_eq!(form_get(&pairs, "status"), Some("2"));
        assert_eq!(form_get(&pairs, "empty"), Some(""));
        assert_eq!(form_get(&pairs, "missing"), None);
    }

    #[test]
    fn adapter_ready_covers_all_channels() {
        assert!(channel_adapter_ready(CHANNEL_MEITUAN));
        assert!(channel_adapter_ready(CHANNEL_ELEME));
        assert!(channel_adapter_ready(CHANNEL_DOUYIN));
        assert!(!channel_adapter_ready("nope"));
    }

    #[test]
    fn eleme_status_map_matches_sdk_enums() {
        assert_eq!(eleme_status_normalize("unprocessed", ""), Some(EXT_STATUS_PAID));
        assert_eq!(eleme_status_normalize("valid", "noRefund"), Some(EXT_STATUS_CONFIRMED));
        assert_eq!(eleme_status_normalize("settled", ""), Some(EXT_STATUS_COMPLETED));
        assert_eq!(eleme_status_normalize("invalid", ""), Some(EXT_STATUS_CANCELLED));
        // 未支付不落履约；未知值不冒认。
        assert_eq!(eleme_status_normalize("unpaid", ""), None);
        assert_eq!(eleme_status_normalize("whatever", ""), None);
        // 退款进程优先于订单状态；驳回/失败不改变订单状态。
        assert_eq!(eleme_status_normalize("valid", "applied"), Some(EXT_STATUS_REFUNDING));
        assert_eq!(eleme_status_normalize("valid", "arbitrating"), Some(EXT_STATUS_REFUNDING));
        assert_eq!(eleme_status_normalize("valid", "successful"), Some(EXT_STATUS_REFUNDED));
        assert_eq!(eleme_status_normalize("valid", "rejected"), Some(EXT_STATUS_CONFIRMED));
        assert_eq!(eleme_status_normalize("Valid", "APPLIED"), Some(EXT_STATUS_REFUNDING));
    }

    #[test]
    fn douyin_action_map() {
        assert_eq!(douyin_action_normalize("pay_success"), Some(EXT_STATUS_PAID));
        assert_eq!(douyin_action_normalize("refund_apply"), Some(EXT_STATUS_REFUNDING));
        assert_eq!(douyin_action_normalize("refund_success"), Some(EXT_STATUS_REFUNDED));
        assert_eq!(douyin_action_normalize("order_cancel"), Some(EXT_STATUS_CANCELLED));
        assert_eq!(douyin_action_normalize("certificate_verify"), Some(EXT_STATUS_COMPLETED));
        assert_eq!(douyin_action_normalize(""), None);
        assert_eq!(douyin_action_normalize("something_new"), None);
    }

    #[test]
    fn yuan_amount_avoids_float_tail() {
        assert_eq!(yuan_amount(12.5), Decimal::new(1250, 2));
        assert_eq!(yuan_amount(0.1 + 0.2), Decimal::new(30, 2));
        assert_eq!(yuan_amount(25.9), Decimal::new(2590, 2));
        assert_eq!(yuan_amount(f64::NAN), Decimal::new(0, 0));
    }

    #[test]
    fn validate_dtos() {
        let mut b = BindingSaveDTO {
            channel: CHANNEL_MEITUAN.into(),
            app_poi_code: "S1".into(),
            ..Default::default()
        };
        assert_eq!(validate_binding(&b), None);
        b.channel = "nope".into();
        assert!(validate_binding(&b).is_some());
        let m = MappingSaveDTO {
            channel: CHANNEL_MEITUAN.into(),
            external_sku_id: "F1".into(),
            product_id: 9,
            ..Default::default()
        };
        assert_eq!(validate_mapping(&m), None);
    }
}
