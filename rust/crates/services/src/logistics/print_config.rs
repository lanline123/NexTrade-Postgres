//! 打印机配置管理域服务（logistics::print_config）。
//!
//! 对应原 Java `com.nextrade.service.print.impl.PrintConfigServiceImpl`
//! （156 行）+ `PrintRoutingServiceImpl#resolve` + `PrintRoutingPolicy#select`
//! + `com.nextrade.service.print.impl.PrintLogServiceImpl#createPending`。
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /print/printConfig/getConfigsByPrintId` | `PrintConfigController.getConfigsByPrintId` | [`PrintConfigService::get_configs_by_print_id`] |
//! | `POST /print/printConfig/update` | `PrintConfigController.update` | [`PrintConfigService::update_config`] |
//! | `POST /print/printConfig/print` | `PrintConfigController.print` | [`PrintConfigService::print_orders`] |
//!
//! # 迁移要点（逐条对照 Java 源）
//!
//! ## `getConfigsByPrintId`
//!
//! ```text
//! List<PrintConfig> configs = lambdaQuery().eq(printId).list();
//! return configs.stream().map(PrintConfigVO::new).findFirst().orElse(null);
//! ```
//!
//! 即「按 `print_id` 取全部配置（**无排序**）→ 逐条转 VO → 取第一条」，
//! 一条都没有时返回 `null`（不是 404）。
//!
//! ## `updateConfig`
//!
//! 校验（`@Valid`）→ `getById(id)` → 不存在抛 `打印配置不存在`（**在 try 之外**，
//! 故不套 catch 文案）→ `new ObjectMapper().writeValueAsString(param.getTemplate())`
//! 把模板序列化成 JSON 字符串 → 写 `template` → `paperSize != null` 才覆盖
//! （`null` 保留原值，兼容旧前端）→ 刷新 `update_time` → `updateById`。
//! 其余异常统一包装成 `更新打印配置失败：<msg>`。
//!
//! ## `print`
//!
//! ```text
//! String deviceCode = isBlank(param.getDeviceCode()) ? null : trim();
//! List<Print> printers = routingService.resolve(getShopId(), deviceCode);
//! if (printers.isEmpty()) throw new GlobalException(deviceCode == null
//!         ? "当前店铺没有启用的打印机" : "所选设备没有绑定启用的打印机");
//! for (Integer id : param.getIds())
//!     for (Print print : printers)
//!         rabbitTemplate.convertAndSend(DIRECT_EXCHANGE, PRINT_ORDER_ROUTING_KEY,
//!             PrintOrderDTO(print, id, deviceCode, MANUAL));
//! return true;
//! ```
//!
//! 打印机选择策略（[`select_printers`]，逐条复刻 `PrintRoutingPolicy#select`）：
//! 1. `enabled` 为空 → 空列表
//! 2. 明确设备码 → **只**取 `print_pos_device.device_code == 设备码` 的启用打印机
//! 3. 无设备码且有默认打印机 → 取默认那台
//! 4. 无设备码且无默认 → 取 `print_id` 最小的第一台
//!
//! ⚠️ **消息投递的替代实现**：原 Java 把 `PrintOrderDTO` 发到 RabbitMQ
//! （`PRINT_ORDER_QUEUE`），由 `RabbitMqConsumer#receivePrintOrderMessage`
//! 消费，消费的第一步就是 `printLogService.createPending(...)` 预写一条
//! `status = 0`（待推送）的 `print_log`。本仓 `crates/infrastructure/src/queue`
//! 目前**只有 `health_check`，没有 publish**（RabbitMQ 客户端未迁移），
//! 且 `print` 的渲染器 / 平台适配器（飞鹅云、商米云）也尚未迁移。
//!
//! 因此本实现把「消费端第一步」的**可观测结果**直接落库：
//! 为每个「订单 × 打印机」写一条 `trigger_type = 2`（手动补打）、
//! `status = 0`（待推送）的 `print_log`，使打印任务**不丢失**、可被
//! 后续消费者（或运维）接管；真正的平台推送仍未实现。
//! 这一点已列入 `uncertain_points` / `notes`。

use std::collections::HashMap;
use std::sync::Arc;

use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use sha2::Sha256;

use nextrade_core::entities::Print;
use nextrade_core::models::logistics::print_config::{
    build_vo_template, default_receipt_template, format_time, normalize_device_code,
    paper_size_code, paper_size_desc, type_desc, validate_print, validate_update,
    PrintConfigUpdateDTO, PrintConfigVO, PrintOrdersDTO, PrintTemplateItem, DEFAULT_SHOP_ID,
    MSG_CONFIG_NOT_FOUND, MSG_DEVICE_NO_PRINTER, MSG_NO_ENABLED_PRINTER,
    MSG_UPDATE_FAILED_PREFIX, TRIGGER_TYPE_MANUAL,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::logistics::print_config::{PrintConfigRow, PrintConfigRepository};
use nextrade_infrastructure::db::Repositories;

use crate::logistics::receipt::{render_feieyun, render_sunmi, ReceiptContext};

/// 打印机配置管理域服务（挂载于 `AdminState.print_config`）。
pub struct PrintConfigService {
    repos: Arc<Repositories>,
}

impl PrintConfigService {
    /// 构造（与 seckill / member_tag / vendor_deposit_shop 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::logistics::print_config::PrintConfigRepository {
        &self.repos.logistics.print_config
    }

    /// 路由层默认使用的店铺 ID（平台 0）。
    ///
    /// Java `BaseServiceImpl#getShopId()` → `HeaderUtils.getShopId()`：
    /// `X-Admin-Type: admin` → `0`。Rust 侧鉴权中间件尚未注入店铺上下文，
    /// 故默认按平台账号处理（见模块头与 `uncertain_points`）。
    #[must_use]
    pub const fn default_shop_id() -> i32 {
        DEFAULT_SHOP_ID
    }

    // ————————————————————— 读取配置 —————————————————————

    /// 获取某打印机的配置（对应 Java
    /// `PrintConfigServiceImpl#getConfigsByPrintId`）。
    ///
    /// 流程与 Java 逐条对齐：
    /// 1. `WHERE print_id = ?`（**无 ORDER BY**，与 Java `selectList` 一致）
    /// 2. 逐条转 [`PrintConfigVO`]（模板解析失败回退默认模板）
    /// 3. `findFirst()` —— 无配置时返回 `Ok(None)`（Java 返回 `null`）
    ///
    /// # Errors
    /// 数据库执行失败时返回 `Error::Database`。
    pub async fn get_configs_by_print_id(&self, print_id: i32) -> Result<Option<PrintConfigVO>> {
        let rows = self
            .repos.logistics.print_config
            .find_configs_by_print_id(print_id)
            .await?;
        Ok(rows.first().map(to_vo))
    }

    // ————————————————————— 更新配置 —————————————————————

    /// 更新打印配置（对应 Java `PrintConfigServiceImpl#updateConfig`）。
    ///
    /// 流程与 Java 逐条对齐（详见模块头）：
    /// 1. `@Valid PrintConfigUpdateParam`（`id` / `template` 必填）
    /// 2. `getById(id)` → 不存在抛 `打印配置不存在`
    /// 3. 模板序列化为 JSON 字符串（等价 Jackson `writeValueAsString`）
    /// 4. `paperSize` 传值才覆盖；不传则保留库里的原值
    /// 5. 刷新 `update_time` → `UPDATE ... WHERE id = ?`
    ///
    /// 返回是否命中记录（Java 返回 `this.updateById(config)` 的布尔值）。
    ///
    /// # Errors
    /// - 入参缺失 → `Error::Validation`（文案逐字符照抄 Java）
    /// - 配置不存在 → `Error::Validation(打印配置不存在)`
    /// - 模板序列化失败 → `Error::Validation(更新打印配置失败：<msg>)`
    /// - 数据库失败 → `Error::Database`
    pub async fn update_config(&self, dto: &PrintConfigUpdateDTO) -> Result<bool> {
        // 1. `@Valid`（`@NotNull id` / `@NotNull template`）
        let (id, template) = validate_update(dto).map_err(Error::Validation)?;

        // 2. `getById`：不存在 → 「打印配置不存在」
        //    （Java 该异常在 try 之外抛出，不套「更新打印配置失败：」前缀）
        let row = self
            .repos.logistics.print_config
            .find_config_by_id(id)
            .await?
            .ok_or_else(|| Error::Validation(MSG_CONFIG_NOT_FOUND.into()))?;

        // 3. 模板 → JSON 字符串（Jackson `writeValueAsString(param.getTemplate())`）
        let template_json = serde_json::to_string(&template)
            .map_err(|e| Error::Validation(format!("{MSG_UPDATE_FAILED_PREFIX}{e}")))?;

        // 4. 纸张尺寸：前端传值才覆盖（走 `PrintPaperSizeEnum#of` 兜底），
        //    否则原样回写库里已有的值（等价于 Java 保留原枚举值）
        let paper_size = dto
            .paper_size
            .map(paper_size_code)
            .unwrap_or(row.paper_size);

        // 5. 刷新 update_time 并更新
        let affected = self
            .repos.logistics.print_config
            .touch_config(id, &template_json, paper_size, now_secs_i32())
            .await?;
        tracing::info!(
            "更新打印配置: id={id}, printId={}, paperSize={paper_size}, 影响 {affected} 行",
            row.print_id
        );
        Ok(affected > 0)
    }

    // ————————————————————— 打印订单 —————————————————————

    /// 打印订单（对应 Java `PrintConfigServiceImpl#print`）。
    ///
    /// 流程与 Java 逐条对齐（详见模块头）：
    /// 1. `@Valid PrintOrdersParam`（`ids` 必填，但**空数组合法**）
    /// 2. 设备码归一：空白 → `None`
    /// 3. 解析该店铺启用打印机 + 收银台设备绑定 + 默认打印机
    /// 4. 按 [`select_printers`] 选出目标打印机；为空则抛
    ///    `当前店铺没有启用的打印机` / `所选设备没有绑定启用的打印机`
    /// 5. 每个「订单 × 打印机」落一条待推送 `print_log`
    ///    （原 Java 为投递 RabbitMQ，见模块头的替代说明）
    ///
    /// Java 返回 `true`（恒真）。
    ///
    /// # Errors
    /// - `ids` 为 `null` → `Error::Validation(订单id不能为空)`
    /// - 无可用打印机 → `Error::Validation(当前店铺没有启用的打印机
    ///   / 所选设备没有绑定启用的打印机)`
    /// - 数据库失败 → `Error::Database`
    pub async fn print_orders(&self, dto: &PrintOrdersDTO, shop_id: i32) -> Result<bool> {
        // 1 + 2. 校验与设备码归一
        let ids = validate_print(dto).map_err(Error::Validation)?;
        let device_code = normalize_device_code(dto.device_code.as_deref());

        // 3. 解析启用打印机 + 设备绑定 + 默认打印机
        let enabled = self.repos.logistics.print_config.find_enabled_prints(shop_id).await?;
        let print_ids: Vec<i32> = enabled.iter().map(|p| p.print_id).collect();
        let bindings = self.bindings_map(shop_id, &print_ids).await?;
        let default_id = enabled
            .iter()
            .find(|p| p.is_default == 1)
            .map(|p| p.print_id);

        // 4. 选择目标打印机
        let printers = select_printers(device_code.as_deref(), &enabled, &bindings, default_id);
        if printers.is_empty() {
            return Err(Error::Validation(
                no_printer_message(device_code.as_deref()).into(),
            ));
        }

        // 5. 每个「订单 × 打印机」预写一条待推送日志并执行投递
        let now = now_secs_i32();
        let mut written = 0usize;
        for order_id in &ids {
            for p in &printers {
                let log_id = self.repos.logistics.print_config
                    .insert_pending_print_log(
                        shop_id,
                        p.print_id,
                        p.platform,
                        *order_id,
                        p.print_sn.as_deref(),
                        TRIGGER_TYPE_MANUAL,
                        now,
                    )
                    .await?;
                written += 1;

                if let Err(e) = self.dispatch_print_task(log_id, p, *order_id).await {
                    tracing::warn!("投递打印任务失败: log_id={log_id}, error={e}");
                }
            }
        }
        tracing::info!(
            "打印订单任务已处理: shopId={shop_id}, 订单 {} 个, 打印机 {} 台, 任务 {written} 条",
            ids.len(),
            printers.len()
        );
        Ok(true)
    }

    /// 渲染小票内容（按 print_config.template / paper_size，平台决定输出协议）。
    ///
    /// 对应 Java `RabbitMqConsumer#receivePrintOrderMessage` 的
    /// 「取配置 → rendererFactory.of(platform).render(orderId, template, paperSize)」。
    async fn build_receipt(&self, printer: &Print, order_id: i32) -> Result<(String, i32)> {
        let order = self
            .repos
            .trade
            .order
            .find_by_id(order_id)
            .await?
            .ok_or_else(|| Error::NotFound("订单不存在".into()))?;
        let items = self.repos.trade.order.find_items(order_id).await?;
        let configs = self.repo().find_configs_by_print_id(printer.print_id).await?;
        let config = configs
            .first()
            .ok_or_else(|| Error::Validation("未找到打印机配置".into()))?;
        let template: Vec<PrintTemplateItem> = config
            .template
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(default_receipt_template);
        let paper = config.paper_size;
        let currency = self
            .repos
            .system
            .config_basic
            .find_by_code("dollarSign")
            .await?
            .unwrap_or_else(|| "￥".into())
            .replace('¥', "￥");
        let default_shop_name = self
            .repos
            .system
            .config_basic
            .find_by_code("shopName")
            .await?
            .unwrap_or_default();
        let shop = self
            .repos
            .merchant
            .shop
            .find_by_id(order.shop_id)
            .await
            .ok()
            .flatten();
        let user = self
            .repos
            .member
            .user
            .find_by_id(order.user_id)
            .await
            .ok()
            .flatten();
        let paylog = self
            .repos
            .payment
            .paylog
            .find_last_paid_by_order(order_id, 0)
            .await
            .ok()
            .flatten();
        let reservation = self
            .repos
            .payment
            .pay
            .find_reservation_by_order_id(order_id)
            .await
            .ok()
            .flatten();

        let ctx = ReceiptContext {
            order: &order,
            items: &items,
            template: &template,
            paper_size: paper,
            currency: &currency,
            default_shop_name: &default_shop_name,
            shop_title: shop.as_ref().and_then(|s| s.shop_title.as_deref()),
            shop_address: shop.as_ref().and_then(|s| s.shop_detailed_address.as_deref()),
            user_name: user.as_ref().map(|u| u.username.as_str()),
            user_mobile: user.as_ref().map(|u| u.mobile.as_str()),
            paylog: paylog.as_ref(),
            reservation: reservation.as_ref(),
        };
        let content = if printer.platform == Some(2) {
            render_sunmi(&ctx)
        } else {
            render_feieyun(&ctx)
        };
        Ok((content, printer.print_number.unwrap_or(1).max(1)))
    }

    /// 投递打印任务给打印平台（商米云 / 飞鹅云），并回写状态到 print_log
    async fn dispatch_print_task(
        &self,
        log_id: i64,
        printer: &Print,
        order_id: i32,
    ) -> Result<()> {
        let now = now_secs_i32();
        let (receipt, copies) = match self.build_receipt(printer, order_id).await {
            Ok(v) => v,
            Err(e) => {
                let msg = e.to_string();
                self.repos
                    .logistics
                    .print_config
                    .update_print_log_status(
                        log_id,
                        2,
                        None,
                        Some("RENDER_ERR"),
                        Some(&msg),
                        None,
                        None,
                    )
                    .await?;
                return Ok(());
            }
        };
        if printer.platform == Some(2) {
            // 商米云
            let appid = printer.third_account.as_deref().unwrap_or_default().trim();
            let appkey = printer.third_key.as_deref().unwrap_or_default().trim();
            let sn = printer.print_sn.as_deref().unwrap_or_default().trim();
            if appid.is_empty() || appkey.is_empty() || sn.is_empty() {
                self.repos.logistics.print_config.update_print_log_status(
                    log_id, 2, None, Some("PARAM_ERR"), Some("商米云账号或SN未配置"), None, None
                ).await?;
                return Ok(());
            }

            let trade_no = order_id.to_string();
            let payload = serde_json::json!({
                "sn": sn,
                "trade_no": &trade_no,
                "order_type": 1,
                "content": receipt,
                "count": copies,
            });
            let body = payload.to_string();
            let timestamp = nextrade_common::utils::now_seconds().to_string();
            let nonce = format!("{:06}", rand::random::<u32>() % 900_000 + 100_000);

            let sign_str = format!("{body}{appid}{timestamp}{nonce}");
            let mut mac = Hmac::<Sha256>::new_from_slice(appkey.as_bytes())
                .map_err(|e| Error::biz_msg(format!("初始化 HMAC 失败: {e}")))?;
            mac.update(sign_str.as_bytes());
            let sign = hex::encode(mac.finalize().into_bytes());

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| Error::biz_msg(format!("创建 HTTP 客户端失败: {e}")))?;

            let res = client
                .post("https://openapi.sunmi.com/v2/printer/open/open/device/pushContent")
                .header("Source", "openapi")
                .header("Sunmi-Appid", appid)
                .header("Sunmi-Timestamp", &timestamp)
                .header("Sunmi-Nonce", &nonce)
                .header("Sunmi-Sign", &sign)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;

            match res {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let code = v.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                        if code == 1 || code == 10000 {
                            self.repos.logistics.print_config.update_print_log_status(
                                log_id, 1, Some(&trade_no), None, None, Some(&text), Some(now)
                            ).await?;
                        } else {
                            let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("推送失败");
                            let code_str = code.to_string();
                            self.repos.logistics.print_config.update_print_log_status(
                                log_id, 2, Some(&trade_no), Some(&code_str), Some(msg), Some(&text), None
                            ).await?;
                        }
                    } else {
                        self.repos.logistics.print_config.update_print_log_status(
                            log_id, 2, Some(&trade_no), Some("PARSE_ERR"), Some("响应解析失败"), Some(&text), None
                        ).await?;
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    self.repos.logistics.print_config.update_print_log_status(
                        log_id, 2, Some(&trade_no), Some("NET_ERR"), Some(&err_str), None, None
                    ).await?;
                }
            }
        } else if printer.platform == Some(1) {
            // 飞鹅云
            let user = printer.third_account.as_deref().unwrap_or_default().trim();
            let ukey = printer.third_key.as_deref().unwrap_or_default().trim();
            let sn = printer.print_sn.as_deref().unwrap_or_default().trim();
            if user.is_empty() || ukey.is_empty() || sn.is_empty() {
                self.repos.logistics.print_config.update_print_log_status(
                    log_id, 2, None, Some("PARAM_ERR"), Some("飞鹅云账号或UKEY未配置"), None, None
                ).await?;
                return Ok(());
            }

            let stime = nextrade_common::utils::now_seconds().to_string();
            let sig_str = format!("{user}{ukey}{stime}");
            let sig = format!("{:x}", <Md5 as Digest>::digest(sig_str.as_bytes()));
            let content = receipt.clone();

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| Error::biz_msg(format!("创建 HTTP 客户端失败: {e}")))?;

            let params = [
                ("user", user),
                ("stime", &stime),
                ("sig", &sig),
                ("apiname", "Open_printMsg"),
                ("sn", sn),
                ("content", &content),
                ("times", &copies.to_string()),
            ];

            let res = client
                .post("http://api.feieyun.cn/Api/Open/")
                .form(&params)
                .send()
                .await;

            match res {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let ret = v.get("ret").and_then(|r| r.as_i64()).unwrap_or(-1);
                        if ret == 0 {
                            let order_index = v.get("data").and_then(|d| d.as_str()).unwrap_or("");
                            self.repos.logistics.print_config.update_print_log_status(
                                log_id, 1, Some(order_index), None, None, Some(&text), Some(now)
                            ).await?;
                        } else {
                            let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("推送失败");
                            let ret_str = ret.to_string();
                            self.repos.logistics.print_config.update_print_log_status(
                                log_id, 2, None, Some(&ret_str), Some(msg), Some(&text), None
                            ).await?;
                        }
                    } else {
                        self.repos.logistics.print_config.update_print_log_status(
                            log_id, 2, None, Some("PARSE_ERR"), Some("响应解析失败"), Some(&text), None
                        ).await?;
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    self.repos.logistics.print_config.update_print_log_status(
                        log_id, 2, None, Some("NET_ERR"), Some(&err_str), None, None
                    ).await?;
                }
            }
        }
        Ok(())
    }

    /// 设备码 → 打印机 ID 绑定映射
    /// （Java `PrintPosDeviceServiceImpl#bindingsByPrintIds` 的
    /// `rows.forEach(row -> result.putIfAbsent(printId, deviceCode))` 语义：
    /// **同一打印机有多条绑定时取先出现的那条**）。
    async fn bindings_map(
        &self,
        shop_id: i32,
        print_ids: &[i32],
    ) -> Result<HashMap<i32, String>> {
        let rows = self.repos.logistics.print_config.find_bindings(shop_id, print_ids).await?;
        let mut bindings: HashMap<i32, String> = HashMap::with_capacity(rows.len());
        for row in rows {
            bindings.entry(row.print_id).or_insert(row.device_code);
        }
        Ok(bindings)
    }
}

/// 打印机选择策略（逐条复刻 Java `PrintRoutingPolicy#select`）。
///
/// ```text
/// if (enabledPrinters == null || isEmpty) return List.of();
/// if (deviceCode != null && !isBlank) return enabled.filter(bindings.get(id) == deviceCode);
/// if (defaultPrintId != null) { defaults = enabled.filter(id == defaultPrintId);
///                               if (!defaults.isEmpty()) return defaults; }
/// return List.of(enabledPrinters.getFirst());
/// ```
///
/// ⚠️ 入参 `device_code` 应已归一（空白视为 `None`）；这里仍按
/// Java 再判一次 `!is_blank`，保证单独调用本函数时语义一致。
#[must_use]
pub fn select_printers(
    device_code: Option<&str>,
    enabled: &[Print],
    bindings: &HashMap<i32, String>,
    default_id: Option<i32>,
) -> Vec<Print> {
    if enabled.is_empty() {
        return Vec::new();
    }
    if let Some(code) = device_code {
        if !code.is_empty() {
            return enabled
                .iter()
                .filter(|p| bindings.get(&p.print_id).map(String::as_str) == Some(code))
                .cloned()
                .collect();
        }
    }
    if let Some(default_id) = default_id {
        let defaults: Vec<Print> = enabled
            .iter()
            .filter(|p| p.print_id == default_id)
            .cloned()
            .collect();
        if !defaults.is_empty() {
            return defaults;
        }
    }
    vec![enabled[0].clone()]
}

/// 无可用打印机时的业务文案（Java 三元表达式）：
/// `deviceCode == null ? 当前店铺没有启用的打印机 : 所选设备没有绑定启用的打印机`。
#[must_use]
pub fn no_printer_message(device_code: Option<&str>) -> &'static str {
    if device_code.is_none() {
        MSG_NO_ENABLED_PRINTER
    } else {
        MSG_DEVICE_NO_PRINTER
    }
}

/// 一行 `print_config` → VO（对应 Java `new PrintConfigVO(PrintConfig)`）。
fn to_vo(row: &PrintConfigRow) -> PrintConfigVO {
    PrintConfigVO {
        id: row.id,
        print_id: row.print_id,
        template: build_vo_template(row.template.as_deref()),
        config_type: row.config_type,
        type_desc: type_desc(row.config_type).to_string(),
        paper_size: paper_size_code(row.paper_size),
        paper_size_desc: paper_size_desc(row.paper_size).to_string(),
        add_time: format_time(row.add_time),
        update_time: format_time(row.update_time),
    }
}

/// 当前 Unix 秒（等价 Java `System.currentTimeMillis() / 1000` 存入 `int` 列）。
fn now_secs_i32() -> i32 {
    i32::try_from(nextrade_common::utils::now_seconds()).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::logistics::print_config::{
        build_vo_template, default_receipt_template, MSG_ORDER_IDS_REQUIRED,
    };

    fn printer(id: i32, is_default: i32) -> Print {
        Print {
            print_id: id,
            is_default,
            shop_id: Some(1),
            status: Some(1),
            ..Default::default()
        }
    }

    // ————————————————————— default_shop_id —————————————————————

    /// 默认店铺 ID 是平台 0（Java `X-Admin-Type: admin`）。
    #[test]
    fn default_shop_id_is_platform() {
        assert_eq!(PrintConfigService::default_shop_id(), 0);
    }

    // ————————————————————— select_printers —————————————————————

    /// 明确设备码 → 只取该设备绑定的打印机（严格过滤）。
    #[test]
    fn select_printers_by_device_code_is_strict() {
        let enabled = vec![printer(1, 1), printer(2, 0)];
        let mut bindings = HashMap::new();
        bindings.insert(1, "POS-A".to_string());
        bindings.insert(2, "POS-B".to_string());

        let hit = select_printers(Some("POS-B"), &enabled, &bindings, Some(1));
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].print_id, 2);

        // 选了默认机也不回退：设备码优先
        let hit = select_printers(Some("POS-A"), &enabled, &bindings, Some(1));
        assert_eq!(hit[0].print_id, 1);

        // 设备码没有绑定任何启用打印机 → 空（Java 抛「所选设备没有绑定启用的打印机」）
        let none = select_printers(Some("POS-X"), &enabled, &bindings, Some(1));
        assert!(none.is_empty());
        // 未绑定的打印机同样不会被选中
        let mut partial = HashMap::new();
        partial.insert(1, "POS-A".to_string());
        assert!(select_printers(Some("POS-B"), &enabled, &partial, None).is_empty());
    }

    /// 无设备码 → 默认打印机优先；无默认 → 首台启用打印机。
    #[test]
    fn select_printers_without_device_code_falls_back() {
        let enabled = vec![printer(5, 1), printer(9, 0)];
        let empty = HashMap::new();

        let hit = select_printers(None, &enabled, &empty, Some(9));
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].print_id, 9);

        // 无默认打印机 → 取 enabled[0]
        let hit = select_printers(None, &enabled, &empty, None);
        assert_eq!(hit[0].print_id, 5);

        // 默认 ID 不在 enabled 里（理论不可达）→ 同样回退首台
        let hit = select_printers(None, &enabled, &empty, Some(404));
        assert_eq!(hit[0].print_id, 5);
    }

    /// enabled 为空 → 任何入参都返回空。
    #[test]
    fn select_printers_empty_enabled_returns_empty() {
        let mut bindings = HashMap::new();
        bindings.insert(1, "POS-A".to_string());
        assert!(select_printers(None, &[], &bindings, None).is_empty());
        assert!(select_printers(Some("POS-A"), &[], &bindings, Some(1)).is_empty());
    }

    /// 空字符串设备码等价于 `None`（Java 已归一，策略再判一次）。
    #[test]
    fn select_printers_blank_device_code_is_ignored() {
        let enabled = vec![printer(1, 0)];
        let empty = HashMap::new();
        assert_eq!(select_printers(Some(""), &enabled, &empty, None)[0].print_id, 1);
    }

    // ————————————————————— no_printer_message —————————————————————

    /// 文案分叉：无设备码 / 有设备码。
    #[test]
    fn no_printer_message_switches_on_device_code() {
        assert_eq!(no_printer_message(None), "当前店铺没有启用的打印机");
        assert_eq!(no_printer_message(Some("POS-A")), "所选设备没有绑定启用的打印机");
    }

    // ————————————————————— to_vo —————————————————————

    /// VO 组装：`type` / 纸张 / 时间 / 模板逐字段对照 Java。
    #[test]
    fn to_vo_maps_row_like_java() {
        let row = PrintConfigRow {
            id: 3,
            print_id: 7,
            template: Some(r#"[{"title":"其他订单信息","options":[]}]"#.to_string()),
            config_type: Some(1),
            paper_size: 80,
            add_time: Some(1_700_000_000),
            update_time: Some(0),
        };
        let vo = to_vo(&row);
        assert_eq!(vo.id, 3);
        assert_eq!(vo.print_id, 7);
        assert_eq!(vo.config_type, Some(1));
        assert_eq!(vo.type_desc, "购物小票");
        assert_eq!(vo.paper_size, 80);
        assert_eq!(vo.paper_size_desc, "80mm");
        assert_eq!(vo.add_time, "2023-11-15 06:13:20");
        assert_eq!(vo.update_time, ""); // 0 → 空串
        // 模板补齐二维码：原 1 项 → 2 项
        let t = vo.template.unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[1].title.as_deref(), Some("二维码"));
    }

    /// `template` 列为 `NULL` → VO `template = null`（不落默认模板）。
    #[test]
    fn to_vo_keeps_null_template_null() {
        let row = PrintConfigRow {
            template: None,
            paper_size: 0, // 历史数据兜底 58
            ..Default::default()
        };
        let vo = to_vo(&row);
        assert!(vo.template.is_none());
        assert_eq!(vo.paper_size, 58);
        assert_eq!(vo.paper_size_desc, "58mm");
        assert_eq!(vo.type_desc, "未知类型"); // type = None
        assert_eq!(vo.add_time, "");
    }

    /// 模板非法 → VO 回退默认模板（Java catch 分支）。
    #[test]
    fn to_vo_falls_back_to_default_template_on_garbage() {
        let row = PrintConfigRow {
            template: Some("{not json".to_string()),
            ..Default::default()
        };
        let vo = to_vo(&row);
        assert_eq!(vo.template, Some(default_receipt_template()));
        // build_vo_template 与 to_vo 取同一分支
        assert_eq!(
            build_vo_template(Some("{not json")),
            Some(default_receipt_template())
        );
    }

    // ————————————————————— 时间戳 —————————————————————

    /// `now_secs_i32()` 必须是合理的「当前」秒级时间戳。
    #[test]
    fn now_secs_is_current_epoch_seconds() {
        let t = now_secs_i32();
        assert!(t > 1_577_836_800, "时间戳过小: {t}"); // > 2020-01-01
        assert!(t < 2_000_000_000, "时间戳过大: {t}"); // < 2033（int 列安全区）
    }

    // ————————————————————— 校验文案透传 —————————————————————

    /// 服务层把校验文案原样透传（`Error::Validation`）。
    #[test]
    fn validation_message_is_passed_through_verbatim() {
        let dto = PrintOrdersDTO::default();
        let err = validate_print(&dto).unwrap_err();
        assert_eq!(err, MSG_ORDER_IDS_REQUIRED);
        assert_eq!(err, "订单id不能为空");
    }
}