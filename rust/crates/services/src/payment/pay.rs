//! 订单支付域服务（payment::pay）。
//!
//! 对应原 Java `com.nextrade.api.controller.order.PayController` 的 6 个
//! `optional` 端点 + `OrderPayServiceImpl` 的 DB/config 部分。
//!
//! # 已实现（纯 DB / config）
//!
//! - `paymentList`：`getAvailablePayment("checkout")`（支付开关 + 客户端过滤 + 排序）。
//! - `index`：订单归属校验 + 订单摘要 + 支付方式（按 `pay_type_id` 过滤式）+
//!   线下支付说明；无 id 时等同下单前收银台（不含 offline 展示项）。
//! - `getPayLog`：拆单子单兜底、最后一笔成功 paylog、实付金额/支付名称、
//!   预售定金/尾款口径、预约信息。
//! - `checkStatus`：订单 `pay_status` 优先 + 最后一笔 paylog 口径。
//!
//! # 未迁移（渠道 SDK）
//!
//! - `create` / `createDepositPayment`：完成用户/订单归属/已支付校验后返回
//!   [`MSG_CHANNEL_UNMIGRATED`]，**不创建 paylog、不伪造支付参数**。
//! - `checkStatus` 的微信付款码轮询分支（`PayConstants.CODE_PAY_CODE`）依赖微信
//!   支付查询 SDK：未迁移，直接返回本地口径。
//!
//! # 偏差
//!
//! 1. `index.order` 为订单摘要（见 [`PayOrderVO`] 注释），非 Java `OrderVO` 全量。
//! 2. `index` 的「已支付」判定用 `pay_status == 2` 近似 Java 的
//!    `availableActions.toPay == false`。
//! 3. i18n 文案未迁移，使用固定中文（与 Java 默认文案一致）。
//! 4. `checkStatus.paylogId` 在 Java 仅当 id 为空时使用（控制器 id 必填，实际不可达）。

use std::sync::Arc;

use nextrade_core::entities::Paylog;
use nextrade_core::models::payment::pay::{
    available_payments, is_reservation_order, offline_payment_list, pay_method_name, CheckStatusQuery,
    OrderReservationVO, PayCreateQuery, PayCreateVO, PayIndexQuery, PayIndexVO, PayLogQuery, PayOrderVO,
    PaylogVO, MSG_CHANNEL_UNMIGRATED, MSG_NOT_LOGGED_IN, MSG_ORDER_ALREADY_PAID,
    MSG_ORDER_NOT_FOUND, MSG_PARAM_MISSING, PAY_STATUS_PAID,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::pay::{PayOrderRow, PayRepository};
use nextrade_infrastructure::db::Repositories;

/// 本域涉及的全部配置键（一次查询）。
const CONFIG_KEYS: [&str; 32] = [
    nextrade_core::models::payment::pay::CFG_USE_WECHAT,
    nextrade_core::models::payment::pay::CFG_USE_ALIPAY,
    nextrade_core::models::payment::pay::CFG_USE_PAYPAL,
    nextrade_core::models::payment::pay::CFG_USE_APPLE_PAY,
    nextrade_core::models::payment::pay::CFG_USE_LAKALA,
    nextrade_core::models::payment::pay::CFG_USE_YABANPAY,
    nextrade_core::models::payment::pay::CFG_USE_YABANPAY_WECHAT,
    nextrade_core::models::payment::pay::CFG_USE_YABANPAY_ALIPAY,
    nextrade_core::models::payment::pay::CFG_USE_YUNPAY,
    nextrade_core::models::payment::pay::CFG_USE_HUIFU,
    nextrade_core::models::payment::pay::CFG_USE_YEEPAY,
    nextrade_core::models::payment::pay::CFG_USE_OFFLINE,
    nextrade_core::models::payment::pay::CFG_PAYMENT_ORDER,
    nextrade_core::models::payment::pay::CFG_OFFLINE_PAY_BANK_ENABLE,
    nextrade_core::models::payment::pay::CFG_OFFLINE_PAY_BANK,
    nextrade_core::models::payment::pay::CFG_OFFLINE_PAY_COMPANY_ENABLE,
    nextrade_core::models::payment::pay::CFG_OFFLINE_PAY_COMPANY,
    nextrade_core::models::payment::pay::CFG_THIRD_PARTY_TRANSFER_ENABLE,
    nextrade_core::models::payment::pay::CFG_THIRD_PARTY_TRANSFER,
    nextrade_core::models::payment::pay::CFG_OTHER_REMITTANCE_ENABLE,
    nextrade_core::models::payment::pay::CFG_OTHER_REMITTANCE,
    nextrade_core::models::payment::pay::CFG_APPLE_PROVIDER,
    nextrade_core::models::payment::pay::CFG_APPLE_PROCESSING_KEY,
    nextrade_core::models::payment::pay::CFG_APPLE_MERCHANT_ID,
    nextrade_core::models::payment::pay::CFG_APPLE_DISPLAY_NAME,
    nextrade_core::models::payment::pay::CFG_APPLE_MERCHANT_CERT,
    nextrade_core::models::payment::pay::CFG_APPLE_MERCHANT_KEY,
    nextrade_core::models::payment::pay::CFG_APPLE_GATEWAY,
    nextrade_core::models::payment::pay::CFG_APPLE_MERCHANT_NO,
    nextrade_core::models::payment::pay::CFG_APPLE_ACQUIRER_KEY,
    nextrade_core::models::payment::pay::CFG_USE_OFFLINE,
    nextrade_core::models::payment::pay::CFG_PAYMENT_ORDER,
];

/// 订单支付域服务。
pub struct PayService {
    repos: Arc<Repositories>,
}

impl PayService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &PayRepository {
        &self.repos.payment.pay
    }

    async fn load_configs(&self) -> Result<std::collections::HashMap<String, String>> {
        self.repo().config_values(&CONFIG_KEYS).await
    }

    // ————————————————————— paymentList —————————————————————

    /// 支付方式列表（Java `getPaymentSelection`）。
    pub async fn payment_list(
        &self,
        user_id: Option<i32>,
        client_type: &str,
    ) -> Result<PayIndexVO> {
        require_user(user_id)?;
        let configs = self.load_configs().await?;
        Ok(PayIndexVO {
            order: None,
            payment_list: available_payments(&configs, client_type, "checkout"),
            offline_payment_list: std::collections::BTreeMap::new(),
        })
    }

    // ————————————————————— index —————————————————————

    /// 订单支付页（Java `getPayOrderInfo`）。
    pub async fn index(
        &self,
        q: &PayIndexQuery,
        user_id: Option<i32>,
        client_type: &str,
    ) -> Result<PayIndexVO> {
        let user_id = require_user(user_id)?;
        let configs = self.load_configs().await?;
        let mut payment_list = available_payments(
            &configs,
            client_type,
            if q.id.is_none() { "checkout" } else { "order" },
        );
        let mut order_vo: Option<PayOrderVO> = None;
        let mut order_sn = String::new();
        match q.id {
            None => {}
            Some(id) => {
                let order = self
                    .repo()
                    .find_order_by_id(id)
                    .await?
                    .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                if order.user_id != user_id {
                    return Err(Error::biz(500, MSG_ORDER_NOT_FOUND));
                }
                if order.pay_status == PAY_STATUS_PAID {
                    return Err(Error::biz(500, MSG_ORDER_ALREADY_PAID));
                }
                if order.pay_type_id == 1 {
                    payment_list.retain(|m| m != "offline");
                } else if order.pay_type_id == 3 {
                    payment_list.retain(|m| m == "offline");
                }
                order_sn = order.order_sn.clone();
                let mut vo = to_order_vo(&order);
                if let Some(user) = self.repos.member.user.find_by_id(user_id).await? {
                    vo.balance_amount = user.available_balance();
                }
                order_vo = Some(vo);
            }
        }
        let offline = offline_payment_list(&configs, &payment_list, &order_sn);
        Ok(PayIndexVO {
            order: order_vo,
            payment_list,
            offline_payment_list: offline,
        })
    }

    // ————————————————————— getPayLog —————————————————————

    /// 检测订单支付状态（Java `getPayLog` → `PaylogVO`）。
    pub async fn paylog(
        &self,
        q: &PayLogQuery,
        user_id: Option<i32>,
    ) -> Result<PaylogVO> {
        let user_id = require_user(user_id)?;
        let order_id = q
            .id
            .filter(|id| *id != 0)
            .ok_or_else(|| Error::biz(500, MSG_PARAM_MISSING))?;
        // 兼容拆单后父订单被物理删除：按 parent_order_id 子订单兜底
        let (order, split_children): (PayOrderRow, Option<Vec<PayOrderRow>>) =
            match self.repo().find_order_by_id(order_id).await? {
                Some(order) if order.user_id == user_id => (order, None),
                Some(_) => return Err(Error::biz(500, MSG_ORDER_NOT_FOUND)),
                None => {
                    let list = self
                        .repo()
                        .find_children_by_parent(order_id, user_id)
                        .await?;
                    if list.is_empty() {
                        return Err(Error::biz(500, MSG_ORDER_NOT_FOUND));
                    }
                    let first = list[0].clone();
                    (first, Some(list))
                }
            };
        let effective_paid_amount = match &split_children {
            None => order.paid_amount,
            Some(list) => list.iter().map(|o| o.paid_amount).sum(),
        };
        let last = self
            .repo()
            .find_last_success_paylog(order_id, 0)
            .await?;
        let mut target = PaylogVO::default();
        // 与 Java 一致：`balance == 1` 视为「全部在线/线下支付」
        if order.balance != rust_decimal::Decimal::ONE {
            if order.is_presale == 1 && order.presale_type == Some(2) {
                if let Some(presale) = self.repo().find_presale_by_order_id(order.order_id).await? {
                    if presale.presale_status == 1 {
                        target.pay_amount = Some(effective_paid_amount);
                    } else if presale.presale_status == 2 {
                        copy_paylog(&mut target, last.as_ref());
                    }
                }
            } else {
                target.pay_amount = Some(effective_paid_amount);
            }
        } else {
            copy_paylog(&mut target, last.as_ref());
        }
        if target
            .pay_name
            .as_deref()
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            target.pay_name = Some(pay_method_name(target.pay_code.as_deref().unwrap_or("")));
        }
        target.order_type = order.order_type;
        if is_reservation_order(order.order_type) {
            if let Some(reservation) = self
                .repo()
                .find_reservation_by_order_id(order.order_id)
                .await?
            {
                target.order_reservation = Some(OrderReservationVO {
                    order_reservations_id: reservation.order_reservations_id,
                    order_id: reservation.order_id,
                    product_reservation_time_id: reservation.product_reservation_time_id,
                    sku_id: reservation.sku_id,
                    reservation_date: reservation.reservation_date,
                    start_time: reservation.start_time,
                    end_time: reservation.end_time,
                    time_slot_description: reservation.time_slot_description,
                    add_time: reservation.add_time.to_string(),
                });
            }
        }
        Ok(target)
    }

    // ————————————————————— checkStatus —————————————————————

    /// 检测订单支付状态（Java `getCheckStatus`）：1 已支付 / 0 待支付。
    pub async fn check_status(
        &self,
        q: &CheckStatusQuery,
        user_id: Option<i32>,
    ) -> Result<i32> {
        let user_id = require_user(user_id)?;
        let order_id = q
            .id
            .ok_or_else(|| Error::biz(500, MSG_PARAM_MISSING))?;
        let order = self
            .repo()
            .find_order_by_id(order_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
        if order.user_id != user_id {
            return Err(Error::biz(500, MSG_ORDER_NOT_FOUND));
        }
        let pay_status = order.pay_status;
        if pay_status == PAY_STATUS_PAID {
            return Ok(1);
        }
        let last = self
            .repo()
            .find_last_success_paylog(order_id, 0)
            .await?;
        let Some(paylog) = last else {
            return Ok(pay_status);
        };
        if paylog.pay_code != "wechat" {
            return Ok(pay_status);
        }
        // 微信付款码轮询分支（渠道查询 SDK）未迁移：返回本地口径
        Ok(if pay_status > 0 { 1 } else { 0 })
    }

    // ————————————————————— create / createDepositPayment —————————————————————

    /// 支付订单创建（Java `create`）：本地校验后返回渠道未迁移错误。
    pub async fn create(
        &self,
        q: &PayCreateQuery,
        user_id: Option<i32>,
    ) -> Result<PayCreateVO> {
        self.validate_create_target(q, user_id).await?;
        let user_id = require_user(user_id)?;
        let pay_type = q.r#type.as_deref().unwrap_or_default();

        // —— 线下汇款：创建待支付 paylog，订单保持待付款，后台确认收款后置已付 ——
        if pay_type == "offline" {
            let order_id = q.id.unwrap_or(0);
            let order = self
                .repo()
                .find_order_by_id(order_id)
                .await?
                .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
            let pay_sn = self.create_pending_offline_paylog(&order).await?;
            return Ok(PayCreateVO {
                order_id: Some(order.order_id),
                order_sn: Some(order.order_sn.clone()),
                order_amount: Some(order.unpaid_amount),
                pay_info: Some(serde_json::json!({ "paySn": pay_sn, "payType": "offline" })),
                code: None,
            });
        }

        // —— 余额支付：直接扣减并标记订单（与 /api/balance/pay 同一实现）——
        if pay_type == "balance" {
            let balance_pay =
                crate::payment::balance_pay::BalancePayService::new(self.repos.clone());
            let result = balance_pay
                .pay_order(user_id, q.id.unwrap_or(0), None)
                .await?;
            return Ok(PayCreateVO {
                order_id: q.id,
                order_sn: None,
                order_amount: Some(result.deducted),
                pay_info: Some(serde_json::json!({
                    "paySn": result.pay_sn,
                    "payType": "balance",
                    "deducted": result.deducted.to_string(),
                    "orderFullyPaid": result.order_fully_paid,
                })),
                code: None,
            });
        }

        // —— 在线渠道：协议骨架已就绪，配置缺失时给出可操作错误 ——
        let configs = crate::payment::channels::load_channel_configs(&self.repos).await?;
        match pay_type {
            "wechat" | "wechatH5" | "wechatApp" | "wechatMini" => {
                if !configs.wechat.is_configured() {
                    return Err(Error::biz(
                        1001,
                        format!(
                            "微信支付未配置：缺少 {}（设置-支付设置）",
                            configs.wechat.missing().join("/")
                        ),
                    ));
                }
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?
                    .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(
                    &order,
                    "wechat",
                    &format!("微信支付-订单{}", order.order_sn),
                ).await?;
                let pay_info = self.wechat_create(&order, &configs.wechat, client_type, user_id, &pay_sn).await?;
                return Ok(PayCreateVO {
                    order_id: Some(order.order_id),
                    order_sn: Some(order.order_sn.clone()),
                    order_amount: Some(order.unpaid_amount),
                    pay_info: Some(pay_info),
                    code: None,
                });
            }
            "alipay" | "ali" => {
                if !configs.alipay.is_configured() {
                    return Err(Error::biz(
                        1001,
                        format!(
                            "支付宝未配置：缺少 {}（设置-支付设置）",
                            configs.alipay.missing().join("/")
                        ),
                    ));
                }
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?
                    .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(
                    &order,
                    "alipay",
                    &format!("支付宝-订单{}", order.order_sn),
                ).await?;
                let pay_info = self.alipay_create(&order, &configs.alipay, client_type, &pay_sn).await?;
                return Ok(PayCreateVO {
                    order_id: Some(order.order_id),
                    order_sn: Some(order.order_sn.clone()),
                    order_amount: Some(order.unpaid_amount),
                    pay_info: Some(pay_info),
                    code: None,
                });
            }
            "paypal" => {
                if !configs.paypal.is_configured() {
                    return Err(Error::biz(
                        1001,
                        format!(
                            "PayPal 未配置：缺少 {}（设置-支付设置）",
                            configs.paypal.missing().join("/")
                        ),
                    ));
                }
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?
                    .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(
                    &order,
                    "paypal",
                    &format!("PayPal-订单{}", order.order_sn),
                ).await?;
                let pay_info = self.paypal_create(&order, &configs.paypal, client_type, &pay_sn).await?;
                return Ok(PayCreateVO {
                    order_id: Some(order.order_id),
                    order_sn: Some(order.order_sn.clone()),
                    order_amount: Some(order.unpaid_amount),
                    pay_info: Some(pay_info),
                    code: None,
                });
            }
            "applepay" | "applePay" => {
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?.ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let pay_sn = self.create_pending_paylog(&order, "applepay", &format!("ApplePay-订单{}", order.order_sn)).await?;
                let display = self.config_value("applePayDisplayName").await?.unwrap_or_default();
                let country = self.config_value("applePayCountryCode").await?.unwrap_or_else(|| "CN".into());
                let currency = self.config_value("applePayCurrency").await?.unwrap_or_else(|| "CNY".into());
                let networks: Vec<String> = self.config_value("applePaySupportedNetworks").await?
                    .unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(ToString::to_string).collect();
                let caps: Vec<String> = if networks.iter().any(|n| matches!(n.as_str(), "visa" | "masterCard" | "amex" | "discover")) {
                    vec!["supports3DS".into(), "supportsEMV".into(), "supportsCredit".into(), "supportsDebit".into()]
                } else {
                    vec!["supports3DS".into()]
                };
                return Ok(PayCreateVO {
                    order_id: Some(order.order_id),
                    order_sn: Some(order.order_sn.clone()),
                    order_amount: Some(order.unpaid_amount),
                    pay_info: Some(serde_json::json!({"applePay": {
                        "paySn": pay_sn,
                        "countryCode": country,
                        "currencyCode": currency,
                        "merchantDisplayName": display,
                        "supportedNetworks": networks,
                        "merchantCapabilities": caps,
                        "total": {"label": display, "amount": order.unpaid_amount.to_string()}
                    }})),
                    code: None,
                });
            }
            "stripe" => {
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?.ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(&order, "stripe", &format!("Stripe-订单{}", order.order_sn)).await?;
                let pay_info = self.stripe_create(&order, client_type, &pay_sn).await?;
                return Ok(PayCreateVO { order_id: Some(order.order_id), order_sn: Some(order.order_sn.clone()), order_amount: Some(order.unpaid_amount), pay_info: Some(pay_info), code: None });
            }
            "xendit" => {
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?.ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(&order, "xendit", &format!("Xendit-订单{}", order.order_sn)).await?;
                let pay_info = self.xendit_create(&order, client_type, &pay_sn).await?;
                return Ok(PayCreateVO { order_id: Some(order.order_id), order_sn: Some(order.order_sn.clone()), order_amount: Some(order.unpaid_amount), pay_info: Some(pay_info), code: None });
            }
            "antom" => {
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?.ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(&order, "antom", &format!("Antom-订单{}", order.order_sn)).await?;
                let pay_info = self.antom_create(&order, user_id, client_type, &pay_sn).await?;
                return Ok(PayCreateVO { order_id: Some(order.order_id), order_sn: Some(order.order_sn.clone()), order_amount: Some(order.unpaid_amount), pay_info: Some(pay_info), code: None });
            }
            other if matches!(other, "huifu" | "lakala" | "yeepay" | "yabandpay" | "yunpay") => {
                let order_id = q.id.unwrap_or(0);
                let order = self.repo().find_order_by_id(order_id).await?.ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
                let client_type = q.code.as_deref().unwrap_or("pc");
                let pay_sn = self.create_pending_paylog(&order, other, &format!("{other}-订单{}", order.order_sn)).await?;
                let pay_info = self.generic_channel_create(&order, other, client_type, &pay_sn).await?;
                return Ok(PayCreateVO { order_id: Some(order.order_id), order_sn: Some(order.order_sn.clone()), order_amount: Some(order.unpaid_amount), pay_info: Some(pay_info), code: None });
            }
            _ => Err(Error::biz_msg(MSG_CHANNEL_UNMIGRATED)),
        }
    }

    /// 读取微信/支付宝/PayPal 渠道配置（供回调验签与下单使用）。
    ///
    /// # Errors
    /// 数据库读取失败。
    pub async fn channel_configs(
        &self,
    ) -> Result<crate::payment::channels::ChannelConfigs> {
        crate::payment::channels::load_channel_configs(&self.repos).await
    }

    /// 读取 `config` 表单项（Stripe/Xendit/Antom 等新增渠道使用）。
    pub async fn config_value(&self, code: &str) -> Result<Option<String>> {
        self.repos.system.config_basic.find_by_code(code).await
    }

    /// 创建线下汇款待支付流水（Java `PaylogService#creatPayLog` 离线分支）。
    async fn create_pending_offline_paylog(&self, order: &PayOrderRow) -> Result<String> {
        use nextrade_core::entities::Paylog;
        use nextrade_core::models::payment::paylog::{generate_pay_sn, PayOrderType};
        use nextrade_infrastructure::db::repo::payment::paylog::PAYLOG_STATUS_UNPAID;
        let pay_sn = generate_pay_sn();
        let paylog = Paylog {
            paylog_id: 0,
            pay_sn: pay_sn.clone(),
            pay_name: format!("线下汇款-订单{}", order.order_sn),
            order_id: order.order_id,
            order_sn: order.order_sn.clone(),
            order_amount: order.total_amount,
            order_type: PayOrderType::Product as i32,
            pay_amount: order.unpaid_amount,
            pay_status: PAYLOG_STATUS_UNPAID,
            pay_code: "offline".to_string(),
            profit_sharing_channel: None,
            add_time: Some(nextrade_common::utils::now_seconds() as i32),
            transaction_id: String::new(),
            notify_data: String::new(),
            refund_amount: rust_decimal::Decimal::ZERO,
            token_code: String::new(),
            appid: None,
        };
        self.repos.payment.paylog.insert(&paylog).await?;
        Ok(pay_sn)
    }

    /// 微信支付 v3 直连下单（native/h5/jsapi/app）。
    async fn wechat_create(
        &self,
        order: &PayOrderRow,
        cfg: &crate::payment::channels::WechatPayV3Config,
        client_type: &str,
        user_id: i32,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        use crate::payment::channels::{wechat_authorization, wechat_sign, wechat_sign_message};
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        if pc_domain.trim().is_empty() {
            return Err(Error::biz(1001, "pcDomain 未配置，无法生成微信支付回调地址"));
        }
        let notify_url = format!("{}/api/order/pay/notify/wechat", pc_domain.trim_end_matches('/'));
        let amount = (order.unpaid_amount * rust_decimal::Decimal::from(100)).round().to_string();
        let (path, body) = match client_type {
            "h5" => ("/v3/pay/transactions/h5", serde_json::json!({
                "appid": cfg.app_id,
                "mchid": cfg.mchid,
                "description": "订单支付",
                "out_trade_no": pay_sn,
                "notify_url": notify_url,
                "amount": {"total": amount, "currency": "CNY"},
                "scene_info": {"payer_client_ip": "127.0.0.1", "h5_info": {"type": "Wap"}}
            })),
            "wechat" | "wechatMini" | "miniProgram" => {
                use sqlx::Row;
                let auth_type = if matches!(client_type, "wechatMini" | "miniProgram") { 2 } else { 1 };
                let row = sqlx::query("SELECT open_id FROM user_authorize WHERE user_id=$1 AND authorize_type=$2 ORDER BY authorize_id DESC LIMIT 1")
                    .bind(user_id).bind(auth_type)
                    .fetch_optional(&self.repos.pool).await.map_err(|e| Error::Database(e.to_string()))?;
                let openid = row.and_then(|r| r.try_get::<String, _>("open_id").ok()).unwrap_or_default();
                if openid.is_empty() {
                    return Err(Error::biz(1001, "微信支付需要用户 openid，请先授权登录"));
                }
                ("/v3/pay/transactions/jsapi", serde_json::json!({
                    "appid": cfg.app_id,
                    "mchid": cfg.mchid,
                    "description": "订单支付",
                    "out_trade_no": pay_sn,
                    "notify_url": notify_url,
                    "amount": {"total": amount, "currency": "CNY"},
                    "payer": {"openid": openid}
                }))
            }
            "app" | "wechatApp" => ("/v3/pay/transactions/app", serde_json::json!({
                "appid": cfg.app_id,
                "mchid": cfg.mchid,
                "description": "订单支付",
                "out_trade_no": pay_sn,
                "notify_url": notify_url,
                "amount": {"total": amount, "currency": "CNY"}
            })),
            _ => ("/v3/pay/transactions/native", serde_json::json!({
                "appid": cfg.app_id,
                "mchid": cfg.mchid,
                "description": "订单支付",
                "out_trade_no": pay_sn,
                "notify_url": notify_url,
                "amount": {"total": amount, "currency": "CNY"}
            })),
        };
        let body_text = body.to_string();
        let timestamp = nextrade_common::utils::now_seconds().to_string();
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let message = wechat_sign_message("POST", path, &timestamp, &nonce, &body_text);
        let private_key = cfg.private_key_pem.as_deref().unwrap_or_default();
        let signature = wechat_sign(private_key, &message)?;
        let auth = wechat_authorization(&cfg.mchid, &cfg.serial_no, &nonce, &timestamp, &signature);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post(format!("https://api.mch.weixin.qq.com{path}"))
            .header("Authorization", auth)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(body_text)
            .send().await
            .map_err(|e| Error::biz(1001, format!("微信支付下单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        if !status.is_success() {
            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("微信支付下单失败");
            return Err(Error::biz(1001, format!("微信支付下单失败: {msg}")));
        }
        match client_type {
            "h5" => {
                let url = v.get("h5_url").and_then(|x| x.as_str()).unwrap_or_default();
                if url.is_empty() {
                    return Err(Error::biz(1001, "微信支付响应缺少 h5_url"));
                }
                Ok(serde_json::json!({"paySn": pay_sn, "payType": "wechat", "h5Url": url}))
            }
            "wechat" | "wechatMini" | "miniProgram" => {
                let prepay = v.get("prepay_id").and_then(|x| x.as_str()).unwrap_or_default();
                if prepay.is_empty() {
                    return Err(Error::biz(1001, "微信支付响应缺少 prepay_id"));
                }
                let ts = nextrade_common::utils::now_seconds().to_string();
                let n = uuid::Uuid::new_v4().simple().to_string();
                let package = format!("prepay_id={prepay}");
                let sign_msg = format!("{}\n{}\n{}\n{}\n", cfg.app_id, ts, n, package);
                let pay_sign = wechat_sign(private_key, &sign_msg)?;
                Ok(serde_json::json!({
                    "paySn": pay_sn, "payType": "wechat", "appId": cfg.app_id,
                    "timeStamp": ts, "nonceStr": n, "package": package,
                    "signType": "RSA", "paySign": pay_sign
                }))
            }
            "app" | "wechatApp" => {
                let prepay = v.get("prepay_id").and_then(|x| x.as_str()).unwrap_or_default();
                if prepay.is_empty() {
                    return Err(Error::biz(1001, "微信支付响应缺少 prepay_id"));
                }
                let ts = nextrade_common::utils::now_seconds().to_string();
                let n = uuid::Uuid::new_v4().simple().to_string();
                let sign_msg = format!("{}\n{}\n{}\n{}\n", cfg.app_id, ts, n, prepay);
                let pay_sign = wechat_sign(private_key, &sign_msg)?;
                Ok(serde_json::json!({
                    "paySn": pay_sn, "payType": "wechat", "appId": cfg.app_id,
                    "partnerId": cfg.mchid, "prepayId": prepay, "packageValue": "Sign=WXPay",
                    "nonceStr": n, "timeStamp": ts, "sign": pay_sign
                }))
            }
            _ => {
                let url = v.get("code_url").and_then(|x| x.as_str()).unwrap_or_default();
                if url.is_empty() {
                    return Err(Error::biz(1001, "微信支付响应缺少 code_url"));
                }
                Ok(serde_json::json!({"paySn": pay_sn, "payType": "wechat", "codeUrl": url}))
            }
        }
    }

    /// 支付宝下单（page/wap/app）。
    async fn alipay_create(
        &self,
        order: &PayOrderRow,
        cfg: &crate::payment::channels::AlipayConfig,
        client_type: &str,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        use crate::payment::channels::{alipay_rsa2_sign, alipay_sign_content};
        use std::collections::BTreeMap;
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        if pc_domain.trim().is_empty() {
            return Err(Error::biz(1001, "pcDomain 未配置，无法生成支付宝回调地址"));
        }
        let pc = pc_domain.trim_end_matches('/');
        let (method, product_code) = match client_type {
            "h5" => ("alipay.trade.wap.pay", "QUICK_WAP_WAY"),
            "app" | "android" | "ios" => ("alipay.trade.app.pay", "QUICK_MSECURITY_PAY"),
            _ => ("alipay.trade.page.pay", "FAST_INSTANT_TRADE_PAY"),
        };
        let biz = serde_json::json!({
            "out_trade_no": pay_sn,
            "total_amount": order.unpaid_amount.to_string(),
            "subject": format!("订单{}", order.order_sn),
            "product_code": product_code
        });
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("app_id".into(), cfg.app_id.clone());
        params.insert("method".into(), method.into());
        params.insert("format".into(), "JSON".into());
        params.insert("charset".into(), "utf-8".into());
        params.insert("sign_type".into(), "RSA2".into());
        params.insert("timestamp".into(), timestamp);
        params.insert("version".into(), "1.0".into());
        params.insert("notify_url".into(), format!("{pc}/api/order/pay/notify/ali"));
        params.insert("biz_content".into(), biz.to_string());
        if client_type == "h5" {
            params.insert("return_url".into(), format!("{pc}/member/order/info?id={}", order.order_id));
        }
        let content = alipay_sign_content(&params);
        let private_key = cfg.private_key_pem.as_deref().unwrap_or_default();
        let sign = alipay_rsa2_sign(private_key, &content)?;
        let mut pairs: Vec<String> = params
            .iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect();
        pairs.push(format!("sign={}", url_encode(&sign)));
        let query = pairs.join("&");
        if matches!(client_type, "app" | "android" | "ios") {
            return Ok(serde_json::json!({
                "provider": "alipay",
                "codeUrl": "",
                "orderInfo": query
            }));
        }
        let action = format!("https://openapi.alipay.com/gateway.do?{query}");
        let html = format!(
            "<form name=\"punchout_form\" method=\"get\" action=\"{action}\"><input type=\"submit\" value=\"立即支付\" style=\"display:none\"></form><script>document.forms[0].submit();</script>"
        );
        Ok(serde_json::json!({
            "provider": "alipay",
            "codeUrl": "",
            "html": html
        }))
    }

    /// PayPal 下单（OAuth2 + Orders v2）。
    async fn paypal_create(
        &self,
        order: &PayOrderRow,
        cfg: &crate::payment::channels::PaypalConfig,
        client_type: &str,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        if pc_domain.trim().is_empty() {
            return Err(Error::biz(1001, "pcDomain 未配置，无法生成 PayPal 返回地址"));
        }
        let pc = pc_domain.trim_end_matches('/');
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let token_resp = client
            .post(cfg.oauth_token_url())
            .basic_auth(&cfg.client_id, Some(&cfg.client_secret))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("PayPal 获取 token 网络异常: {e}")))?;
        let token_status = token_resp.status();
        let token_text = token_resp.text().await.unwrap_or_default();
        if !token_status.is_success() {
            return Err(Error::biz(1001, format!("PayPal 获取 token 失败: {token_text}")));
        }
        let token_json: serde_json::Value = serde_json::from_str(&token_text)
            .map_err(|e| Error::Internal(format!("PayPal token 响应解析失败: {e}")))?;
        let access_token = token_json.get("access_token").and_then(|v| v.as_str()).unwrap_or_default();
        if access_token.is_empty() {
            return Err(Error::biz(1001, "PayPal token 响应缺少 access_token"));
        }
        let currency = if cfg.currency.trim().is_empty() { "USD" } else { cfg.currency.trim() };
        let return_url = if client_type == "h5" {
            format!("{pc}/pages/user/order/info?id={}", order.order_id)
        } else {
            format!("{pc}/member/order/info?id={}", order.order_id)
        };
        let body = serde_json::json!({
            "intent": "CAPTURE",
            "purchase_units": [{
                "amount": {"currency_code": currency, "value": order.unpaid_amount.to_string()},
                "invoice_id": pay_sn
            }],
            "application_context": {
                "return_url": return_url,
                "cancel_url": format!("{pc}/member/order/info?id={}", order.order_id)
            }
        });
        let resp = client
            .post(format!("{}/v2/checkout/orders", cfg.base_url()))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("PayPal 创建订单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("PayPal 创建订单失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::Internal(format!("PayPal 订单响应解析失败: {e}")))?;
        if let Some(order_id) = v.get("id").and_then(|v| v.as_str()) {
            let _ = sqlx::query("UPDATE paylog SET transaction_id=$1 WHERE pay_sn=$2")
                .bind(order_id).bind(pay_sn)
                .execute(&self.repos.pool).await;
        }
        let approve = v
            .get("links")
            .and_then(|l| l.as_array())
            .and_then(|links| {
                links.iter().find_map(|l| {
                    if l.get("rel").and_then(|r| r.as_str()).is_some_and(|r| r.eq_ignore_ascii_case("approve")) {
                        l.get("href").and_then(|h| h.as_str()).map(ToString::to_string)
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| Error::biz(1001, "PayPal 订单响应缺少 approve 链接"))?;
        Ok(serde_json::json!({"paySn": pay_sn, "payType": "paypal", "url": approve}))
    }

    /// Stripe Checkout Session 创建。
    async fn stripe_create(
        &self,
        order: &PayOrderRow,
        client_type: &str,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        let secret = self.config_value("stripeSecretKey").await?.unwrap_or_default();
        if secret.trim().is_empty() {
            return Err(Error::biz(1001, "Stripe 未配置：缺少 stripeSecretKey"));
        }
        let currency = self.config_value("stripeCurrency").await?.unwrap_or_else(|| "usd".into());
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        if pc_domain.trim().is_empty() {
            return Err(Error::biz(1001, "pcDomain 未配置，无法生成 Stripe 返回地址"));
        }
        let pc = pc_domain.trim_end_matches('/');
        let (success, cancel) = if client_type == "h5" {
            (
                format!("{pc}/pages/user/order/info?id={}", order.order_id),
                format!("{pc}/pages/user/order/info?id={}", order.order_id),
            )
        } else {
            (
                format!("{pc}/member/order/info?id={}", order.order_id),
                format!("{pc}/member/order/info?id={}", order.order_id),
            )
        };
        let amount = (order.unpaid_amount * rust_decimal::Decimal::from(100)).round().to_string();
        let mut form = vec![
            ("mode".to_string(), "payment".to_string()),
            ("success_url".to_string(), format!("{success}&session_id={{CHECKOUT_SESSION_ID}}")),
            ("cancel_url".to_string(), cancel),
            ("line_items[0][quantity]".to_string(), "1".to_string()),
            ("line_items[0][price_data][currency]".to_string(), currency),
            ("line_items[0][price_data][unit_amount]".to_string(), amount),
            ("line_items[0][price_data][product_data][name]".to_string(), format!("订单: {}", order.order_sn)),
            ("metadata[pay_sn]".to_string(), pay_sn.to_string()),
            ("metadata[order_id]".to_string(), order.order_id.to_string()),
            ("payment_intent_data[metadata][pay_sn]".to_string(), pay_sn.to_string()),
            ("payment_intent_data[metadata][order_id]".to_string(), order.order_id.to_string()),
        ];
        form.sort_by(|a, b| a.0.cmp(&b.0));
        let body = form.iter().map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v))).collect::<Vec<_>>().join("&");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post("https://api.stripe.com/v1/checkout/sessions")
            .header("Authorization", format!("Bearer {}", secret.trim()))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send().await
            .map_err(|e| Error::biz(1001, format!("Stripe 下单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Stripe 下单失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::Internal(format!("Stripe 响应解析失败: {e}")))?;
        let url = v.get("url").and_then(|u| u.as_str()).unwrap_or_default();
        if url.is_empty() {
            return Err(Error::biz(1001, "Stripe 响应缺少 url"));
        }
        Ok(serde_json::json!({"paySn": pay_sn, "payType": "stripe", "url": url}))
    }

    /// Xendit Invoice 创建。
    async fn xendit_create(
        &self,
        order: &PayOrderRow,
        client_type: &str,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        let secret = self.config_value("xenditSecretKey").await?.unwrap_or_default();
        if secret.trim().is_empty() {
            return Err(Error::biz(1001, "Xendit 未配置：缺少 xenditSecretKey"));
        }
        let base = self.config_value("xenditBaseUrl").await?
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "https://api.xendit.co".into());
        let currency = self.config_value("xenditCurrency").await?.unwrap_or_else(|| "idr".into());
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        if pc_domain.trim().is_empty() {
            return Err(Error::biz(1001, "pcDomain 未配置，无法生成 Xendit 返回地址"));
        }
        let pc = pc_domain.trim_end_matches('/');
        let (success, failure) = if client_type == "h5" {
            (
                format!("{pc}/pages/user/order/info?id={}", order.order_id),
                format!("{pc}/pages/user/order/info?id={}", order.order_id),
            )
        } else {
            (
                format!("{pc}/member/order/info?id={}", order.order_id),
                format!("{pc}/member/order/info?id={}", order.order_id),
            )
        };
        let amount_value: f64 = order.unpaid_amount.to_string().parse().unwrap_or(0.0);
        let duration: i64 = self.config_value("xenditInvoiceDuration").await?
            .and_then(|v| v.trim().parse().ok()).unwrap_or(86400);
        let mut body = serde_json::json!({
            "external_id": pay_sn,
            "amount": amount_value,
            "currency": currency,
            "description": format!("NexTrade Order {}", order.order_sn),
            "success_redirect_url": success,
            "failure_redirect_url": failure,
            "invoice_duration": duration,
            "should_send_email": false,
            "items": [{"name": format!("Order {}", order.order_sn), "quantity": 1, "price": amount_value, "reference_id": order.order_id.to_string()}]
        });
        if let Some(methods) = self.config_value("xenditPaymentMethods").await?.filter(|v| !v.trim().is_empty()) {
            body["payment_methods"] = serde_json::Value::Array(
                methods.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|s| serde_json::Value::String(s.to_string())).collect(),
            );
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post(format!("{}/v2/invoices", base.trim_end_matches('/')))
            .basic_auth(secret.trim(), Option::<&str>::None)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send().await
            .map_err(|e| Error::biz(1001, format!("Xendit 下单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Xendit 下单失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::Internal(format!("Xendit 响应解析失败: {e}")))?;
        let url = v.get("invoice_url").and_then(|u| u.as_str()).unwrap_or_default();
        if url.is_empty() {
            return Err(Error::biz(1001, "Xendit 响应缺少 invoice_url"));
        }
        if let Some(id) = v.get("id").and_then(|u| u.as_str()) {
            let _ = sqlx::query("UPDATE paylog SET transaction_id=$1 WHERE pay_sn=$2")
                .bind(id).bind(pay_sn)
                .execute(&self.repos.pool).await;
        }
        Ok(serde_json::json!({"paySn": pay_sn, "payType": "xendit", "url": url}))
    }

    /// Antom Hosted Checkout 创建支付会话。
    async fn antom_create(
        &self,
        order: &PayOrderRow,
        user_id: i32,
        client_type: &str,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        use base64::Engine as _;
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::signature::Signer;
        use rsa::pkcs1v15::SigningKey;
        use sha2::Sha256;
        let client_id = self.config_value("antomClientId").await?.unwrap_or_default();
        let private_key = self.config_value("antomMerchantPrivateKey").await?.unwrap_or_default();
        if client_id.trim().is_empty() || private_key.trim().is_empty() {
            return Err(Error::biz(1001, "Antom 未配置：缺少 antomClientId / antomMerchantPrivateKey"));
        }
        let gateway = self.config_value("antomGatewayUrl").await?
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "https://open-sea-global.alipay.com".into());
        let sandbox = self.config_value("antomSandbox").await?.as_deref() == Some("1");
        let base = gateway.trim_end_matches('/');
        let base = if sandbox && !base.contains("/ams/sandbox") { format!("{base}/ams/sandbox") } else { base.to_string() };
        let currency = self.config_value("antomCurrency").await?.unwrap_or_else(|| "usd".into());
        let currency = currency.to_uppercase();
        let zero_decimal = ["BIF","CLP","DJF","GNF","JPY","KMF","KRW","MGA","PYG","RWF","UGX","VND","VUV","XAF","XOF","XPF"];
        let multiplier = if zero_decimal.contains(&currency.as_str()) { 1 } else { 100 };
        let amount = (order.unpaid_amount * rust_decimal::Decimal::from(multiplier)).round().to_string();
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        if pc_domain.trim().is_empty() {
            return Err(Error::biz(1001, "pcDomain 未配置，无法生成 Antom 返回地址"));
        }
        let pc = pc_domain.trim_end_matches('/');
        let redirect = if client_type == "h5" {
            format!("{pc}/pages/user/order/info?id={}", order.order_id)
        } else {
            format!("{pc}/member/order/info?id={}", order.order_id)
        };
        let notify = format!("{pc}/api/order/pay/notify/antom");
        let request_id = format!("{}-{}", pay_sn, &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let body = serde_json::json!({
            "productCode": "CASHIER_PAYMENT",
            "productScene": "CHECKOUT_PAYMENT",
            "paymentRequestId": request_id,
            "paymentAmount": {"currency": currency, "value": amount},
            "order": {
                "referenceOrderId": order.order_sn,
                "orderDescription": format!("NexTrade Order {}", order.order_sn),
                "orderAmount": {"currency": currency, "value": amount},
                "buyer": {"referenceBuyerId": user_id.to_string()},
                "goods": [{
                    "referenceGoodsId": order.order_id.to_string(),
                    "goodsName": format!("Order {}", order.order_sn),
                    "goodsQuantity": "1",
                    "goodsUnitAmount": {"currency": currency, "value": amount}
                }]
            },
            "paymentNotifyUrl": notify,
            "paymentRedirectUrl": redirect
        });
        let body_text = body.to_string();
        let path = "/ams/api/v1/payments/createPaymentSession";
        let request_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let sign_content = format!("POST {path}\n{}.{}.{}", client_id, request_time, body_text);
        let key = if private_key.contains("BEGIN") {
            rsa::RsaPrivateKey::from_pkcs8_pem(private_key.trim())
                .map_err(|e| Error::Validation(format!("Antom 商户私钥解析失败: {e}")))?
        } else {
            let der = base64::engine::general_purpose::STANDARD.decode(private_key.trim())
                .map_err(|e| Error::Validation(format!("Antom 商户私钥解码失败: {e}")))?;
            rsa::RsaPrivateKey::from_pkcs8_der(&der)
                .map_err(|e| Error::Validation(format!("Antom 商户私钥解析失败: {e}")))?
        };
        let signing_key = SigningKey::<Sha256>::new(key);
        let signature: rsa::pkcs1v15::Signature = signing_key.sign(sign_content.as_bytes());
        use rsa::signature::SignatureEncoding;
        let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let signature_header = format!("algorithm=RSA256,keyVersion=1,signature={signature_b64}");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post(format!("{base}{path}"))
            .header("Content-Type", "application/json")
            .header("client-id", client_id.trim())
            .header("request-time", request_time)
            .header("signature", signature_header)
            .body(body_text)
            .send().await
            .map_err(|e| Error::biz(1001, format!("Antom 下单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Antom 下单失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| Error::Internal(format!("Antom 响应解析失败: {e}")))?;
        let result_status = v.get("result").and_then(|r| r.get("resultStatus")).and_then(|s| s.as_str()).unwrap_or("");
        if result_status != "S" {
            let msg = v.get("result").and_then(|r| r.get("resultMessage")).and_then(|s| s.as_str()).unwrap_or("Antom 创建支付会话失败");
            return Err(Error::biz(1001, msg.to_string()));
        }
        let url = v.get("normalUrl").and_then(|u| u.as_str()).unwrap_or_default();
        if url.is_empty() {
            return Err(Error::biz(1001, "Antom 创建支付会话失败：normalUrl 为空"));
        }
        let _ = sqlx::query("UPDATE paylog SET transaction_id=$1 WHERE pay_sn=$2")
            .bind(&request_id).bind(pay_sn)
            .execute(&self.repos.pool).await;
        Ok(serde_json::json!({"paySn": pay_sn, "payType": "antom", "url": url}))
    }

    /// 渠道退款入口（按 pay_code 分发）。
    pub async fn refund_via_channel(
        &self,
        pay_code: &str,
        pay_sn: &str,
        refund_sn: &str,
        refund_amount: rust_decimal::Decimal,
    ) -> Result<String> {
        let paylog = self
            .repos
            .payment
            .paylog
            .find_latest_by_pay_sn(pay_sn)
            .await?
            .ok_or_else(|| Error::biz(500, "找不到原支付记录"))?;
        if paylog.transaction_id.trim().is_empty() {
            return Err(Error::biz(500, "原支付交易号缺失，无法退款"));
        }
        match pay_code {
            "wechat" => self.wechat_refund(pay_sn, refund_sn, refund_amount, paylog.pay_amount).await,
            "alipay" => self.alipay_refund(pay_sn, refund_sn, refund_amount).await,
            "antom" => self.antom_refund(&paylog.transaction_id, refund_sn, refund_amount).await,
            "stripe" => self.stripe_refund(&paylog.transaction_id, refund_sn, refund_amount).await,
            "paypal" => self.paypal_refund(&paylog.transaction_id, refund_sn, refund_amount).await,
            "xendit" => self.xendit_refund(&paylog.transaction_id, refund_sn, refund_amount).await,
            other if matches!(other, "huifu" | "lakala" | "yeepay" | "yabandpay" | "yunpay") => {
                self.generic_channel_refund(other, pay_sn, refund_sn, refund_amount).await
            }
            other => Err(Error::biz(500, format!("该支付方式暂不支持退款：{other}"))),
        }
    }

    async fn stripe_refund(
        &self,
        payment_intent: &str,
        refund_sn: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<String> {
        let secret = self.config_value("stripeSecretKey").await?.unwrap_or_default();
        if secret.trim().is_empty() {
            return Err(Error::biz(1001, "Stripe 未配置：缺少 stripeSecretKey"));
        }
        let minor = (amount * rust_decimal::Decimal::from(100)).round().to_string();
        let body = format!(
            "payment_intent={}&amount={}&metadata[refund_sn]={}",
            url_encode(payment_intent), minor, url_encode(refund_sn)
        );
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post("https://api.stripe.com/v1/refunds")
            .header("Authorization", format!("Bearer {}", secret.trim()))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body).send().await
            .map_err(|e| Error::biz(1001, format!("Stripe 退款网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Stripe 退款失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        Ok(v.get("status").and_then(|s| s.as_str()).unwrap_or("pending").to_string())
    }

    async fn paypal_refund(
        &self,
        capture_id: &str,
        refund_sn: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<String> {
        use crate::payment::channels::load_channel_configs;
        let configs = load_channel_configs(&self.repos).await?;
        let cfg = &configs.paypal;
        if !cfg.is_configured() {
            return Err(Error::biz(1001, "PayPal 未配置"));
        }
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let token_resp = client.post(cfg.oauth_token_url())
            .basic_auth(&cfg.client_id, Some(&cfg.client_secret))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials").send().await
            .map_err(|e| Error::biz(1001, format!("PayPal 获取 token 异常: {e}")))?;
        let token_text = token_resp.text().await.unwrap_or_default();
        let token_json: serde_json::Value = serde_json::from_str(&token_text).unwrap_or(serde_json::Value::Null);
        let access = token_json.get("access_token").and_then(|v| v.as_str()).unwrap_or_default();
        if access.is_empty() {
            return Err(Error::biz(1001, "PayPal 获取 token 失败"));
        }
        let currency = if cfg.currency.trim().is_empty() { "USD" } else { cfg.currency.trim() };
        let body = serde_json::json!({
            "amount": {"value": amount.to_string(), "currency_code": currency},
            "invoice_id": refund_sn
        });
        let resp = client.post(format!("{}/v2/payments/captures/{capture_id}/refund", cfg.base_url()))
            .bearer_auth(access)
            .header("Content-Type", "application/json")
            .body(body.to_string()).send().await
            .map_err(|e| Error::biz(1001, format!("PayPal 退款网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("PayPal 退款失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        Ok(v.get("status").and_then(|s| s.as_str()).unwrap_or("PENDING").to_string())
    }

    async fn xendit_refund(
        &self,
        invoice_id: &str,
        refund_sn: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<String> {
        let secret = self.config_value("xenditSecretKey").await?.unwrap_or_default();
        if secret.trim().is_empty() {
            return Err(Error::biz(1001, "Xendit 未配置：缺少 xenditSecretKey"));
        }
        let base = self.config_value("xenditBaseUrl").await?
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "https://api.xendit.co".into());
        let currency = self.config_value("xenditCurrency").await?.unwrap_or_else(|| "idr".into());
        let amount_value: f64 = amount.to_string().parse().unwrap_or(0.0);
        let body = serde_json::json!({
            "invoice_id": invoice_id,
            "amount": amount_value,
            "currency": currency,
            "reason": "REQUESTED_BY_CUSTOMER",
            "reference_id": refund_sn
        });
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post(format!("{}/refunds", base.trim_end_matches('/')))
            .basic_auth(secret.trim(), Option::<&str>::None)
            .header("Content-Type", "application/json")
            .header("X-IDEMPOTENCY-KEY", refund_sn)
            .body(body.to_string()).send().await
            .map_err(|e| Error::biz(1001, format!("Xendit 退款网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Xendit 退款失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        Ok(v.get("status").and_then(|s| s.as_str()).unwrap_or("PENDING").to_string())
    }

    /// 微信支付 v3 退款。
    async fn wechat_refund(
        &self,
        pay_sn: &str,
        refund_sn: &str,
        refund_amount: rust_decimal::Decimal,
        total_amount: rust_decimal::Decimal,
    ) -> Result<String> {
        use crate::payment::channels::{load_channel_configs, wechat_authorization, wechat_sign, wechat_sign_message};
        let configs = load_channel_configs(&self.repos).await?;
        let cfg = &configs.wechat;
        if !cfg.is_configured() {
            return Err(Error::biz(1001, "微信支付未配置"));
        }
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        let notify_url = if pc_domain.trim().is_empty() {
            String::new()
        } else {
            format!("{}/api/order/pay/refundNotify/wechat", pc_domain.trim_end_matches('/'))
        };
        let refund_fen = (refund_amount * rust_decimal::Decimal::from(100)).round().to_string();
        let total_fen = (total_amount * rust_decimal::Decimal::from(100)).round().to_string();
        let mut body = serde_json::json!({
            "out_trade_no": pay_sn,
            "out_refund_no": refund_sn,
            "reason": "退款",
            "amount": {"refund": refund_fen, "total": total_fen, "currency": "CNY"}
        });
        if !notify_url.is_empty() {
            body["notify_url"] = serde_json::Value::String(notify_url);
        }
        let path = "/v3/refund/domestic/refunds";
        let body_text = body.to_string();
        let timestamp = nextrade_common::utils::now_seconds().to_string();
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let message = wechat_sign_message("POST", path, &timestamp, &nonce, &body_text);
        let private_key = cfg.private_key_pem.as_deref().unwrap_or_default();
        let signature = wechat_sign(private_key, &message)?;
        let auth = wechat_authorization(&cfg.mchid, &cfg.serial_no, &nonce, &timestamp, &signature);
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post(format!("https://api.mch.weixin.qq.com{path}"))
            .header("Authorization", auth)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(body_text).send().await
            .map_err(|e| Error::biz(1001, format!("微信退款网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("微信退款失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        Ok(v.get("status").and_then(|s| s.as_str()).unwrap_or("PROCESSING").to_string())
    }

    /// 支付宝退款。
    async fn alipay_refund(
        &self,
        pay_sn: &str,
        refund_sn: &str,
        refund_amount: rust_decimal::Decimal,
    ) -> Result<String> {
        use crate::payment::channels::{alipay_rsa2_sign, alipay_sign_content, load_channel_configs};
        use std::collections::BTreeMap;
        let configs = load_channel_configs(&self.repos).await?;
        let cfg = &configs.alipay;
        if !cfg.is_configured() {
            return Err(Error::biz(1001, "支付宝未配置"));
        }
        let biz = serde_json::json!({
            "out_trade_no": pay_sn,
            "refund_amount": refund_amount.to_string(),
            "out_request_no": refund_sn,
            "refund_reason": "退款"
        });
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("app_id".into(), cfg.app_id.clone());
        params.insert("method".into(), "alipay.trade.refund".into());
        params.insert("format".into(), "JSON".into());
        params.insert("charset".into(), "utf-8".into());
        params.insert("sign_type".into(), "RSA2".into());
        params.insert("timestamp".into(), chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        params.insert("version".into(), "1.0".into());
        params.insert("biz_content".into(), biz.to_string());
        let content = alipay_sign_content(&params);
        let private_key = cfg.private_key_pem.as_deref().unwrap_or_default();
        let sign = alipay_rsa2_sign(private_key, &content)?;
        let mut pairs: Vec<String> = params.iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect();
        pairs.push(format!("sign={}", url_encode(&sign)));
        let body = pairs.join("&");
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post("https://openapi.alipay.com/gateway.do")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body).send().await
            .map_err(|e| Error::biz(1001, format!("支付宝退款网络异常: {e}")))?;
        let text = resp.text().await.unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| Error::Internal("支付宝退款响应解析失败".into()))?;
        let result = v.get("alipay_trade_refund_response").cloned().unwrap_or(serde_json::Value::Null);
        let code = result.get("code").and_then(|c| c.as_str()).unwrap_or("");
        if code != "10000" {
            let msg = result.get("sub_msg").or_else(|| result.get("msg")).and_then(|m| m.as_str()).unwrap_or("支付宝退款失败");
            return Err(Error::biz(1001, msg.to_string()));
        }
        Ok("SUCCESS".to_string())
    }

    /// Antom 退款。
    async fn antom_refund(
        &self,
        payment_id: &str,
        refund_sn: &str,
        refund_amount: rust_decimal::Decimal,
    ) -> Result<String> {
        use base64::Engine as _;
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::pkcs1v15::SigningKey;
        use rsa::signature::{SignatureEncoding, Signer};
        use sha2::Sha256;
        let client_id = self.config_value("antomClientId").await?.unwrap_or_default();
        let private_key = self.config_value("antomMerchantPrivateKey").await?.unwrap_or_default();
        if client_id.trim().is_empty() || private_key.trim().is_empty() {
            return Err(Error::biz(1001, "Antom 未配置：缺少 antomClientId / antomMerchantPrivateKey"));
        }
        let gateway = self.config_value("antomGatewayUrl").await?
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "https://open-sea-global.alipay.com".into());
        let base = gateway.trim_end_matches('/');
        let base = if self.config_value("antomSandbox").await?.as_deref() == Some("1") && !base.contains("/ams/sandbox") {
            format!("{base}/ams/sandbox")
        } else {
            base.to_string()
        };
        let currency = self.config_value("antomCurrency").await?.unwrap_or_else(|| "usd".into()).to_uppercase();
        let zero_decimal = ["BIF","CLP","DJF","GNF","JPY","KMF","KRW","MGA","PYG","RWF","UGX","VND","VUV","XAF","XOF","XPF"];
        let multiplier = if zero_decimal.contains(&currency.as_str()) { 1 } else { 100 };
        let amount = (refund_amount * rust_decimal::Decimal::from(multiplier)).round().to_string();
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        let notify = if pc_domain.trim().is_empty() {
            String::new()
        } else {
            format!("{}/api/order/pay/refundNotify/antom", pc_domain.trim_end_matches('/'))
        };
        let body = serde_json::json!({
            "refundRequestId": refund_sn,
            "paymentId": payment_id,
            "refundReason": format!("NexTrade refund {refund_sn}"),
            "refundAmount": {"currency": currency, "value": amount},
            "refundNotifyUrl": notify
        });
        let body_text = body.to_string();
        let path = "/ams/api/v1/payments/refund";
        let request_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let sign_content = format!("POST {path}\n{}.{}.{}", client_id, request_time, body_text);
        let key = if private_key.contains("BEGIN") {
            rsa::RsaPrivateKey::from_pkcs8_pem(private_key.trim())
                .map_err(|e| Error::Validation(format!("Antom 商户私钥解析失败: {e}")))?
        } else {
            let der = base64::engine::general_purpose::STANDARD.decode(private_key.trim())
                .map_err(|e| Error::Validation(format!("Antom 商户私钥解码失败: {e}")))?;
            rsa::RsaPrivateKey::from_pkcs8_der(&der)
                .map_err(|e| Error::Validation(format!("Antom 商户私钥解析失败: {e}")))?
        };
        let signing_key = SigningKey::<Sha256>::new(key);
        let signature: rsa::pkcs1v15::Signature = signing_key.sign(sign_content.as_bytes());
        let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(25)).build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let resp = client.post(format!("{base}{path}"))
            .header("Content-Type", "application/json")
            .header("client-id", client_id.trim())
            .header("request-time", request_time)
            .header("signature", format!("algorithm=RSA256,keyVersion=1,signature={signature_b64}"))
            .body(body_text).send().await
            .map_err(|e| Error::biz(1001, format!("Antom 退款网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("Antom 退款失败: {text}")));
        }
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let result_status = v.get("result").and_then(|r| r.get("resultStatus")).and_then(|s| s.as_str()).unwrap_or("");
        if result_status != "S" {
            let msg = v.get("result").and_then(|r| r.get("resultMessage")).and_then(|s| s.as_str()).unwrap_or("Antom 退款失败");
            return Err(Error::biz(1001, msg.to_string()));
        }
        Ok("SUCCESS".to_string())
    }

    /// 充值支付创建（复用渠道下单，paylog 使用充值订单类型）。
    pub async fn create_recharge_payment(
        &self,
        recharge_id: i32,
        user_id: i32,
        amount: rust_decimal::Decimal,
        pay_type: &str,
        client_type: &str,
    ) -> Result<PayCreateVO> {
        use nextrade_core::models::payment::paylog::{generate_pay_sn, PayOrderType};
        use nextrade_infrastructure::db::repo::payment::paylog::PAYLOG_STATUS_UNPAID;
        let pay_sn = generate_pay_sn();
        let order_sn = format!("RC{recharge_id}");
        let paylog = Paylog {
            paylog_id: 0,
            pay_sn: pay_sn.clone(),
            pay_name: format!("充值-{recharge_id}"),
            order_id: recharge_id,
            order_sn: order_sn.clone(),
            order_amount: amount,
            order_type: PayOrderType::Recharge as i32,
            pay_amount: amount,
            pay_status: PAYLOG_STATUS_UNPAID,
            pay_code: pay_type.to_string(),
            profit_sharing_channel: None,
            add_time: Some(nextrade_common::utils::now_seconds() as i32),
            transaction_id: String::new(),
            notify_data: String::new(),
            refund_amount: rust_decimal::Decimal::ZERO,
            token_code: String::new(),
            appid: None,
        };
        self.repos.payment.paylog.insert(&paylog).await?;
        let order = PayOrderRow {
            order_id: recharge_id,
            order_sn,
            user_id,
            total_amount: amount,
            paid_amount: rust_decimal::Decimal::ZERO,
            unpaid_amount: amount,
            balance: rust_decimal::Decimal::ZERO,
            pay_status: 0,
            order_status: 0,
            pay_type_id: 1,
            is_presale: 0,
            presale_type: None,
            order_type: Some(PayOrderType::Recharge as i32),
            parent_order_id: None,
            add_time: nextrade_common::utils::now_seconds() as i32,
            shipping_type_name: String::new(),
        };
        let configs = crate::payment::channels::load_channel_configs(&self.repos).await?;
        let pay_info = match pay_type {
            "wechat" => {
                if !configs.wechat.is_configured() {
                    return Err(Error::biz(1001, "微信支付未配置"));
                }
                self.wechat_create(&order, &configs.wechat, client_type, user_id, &pay_sn).await?
            }
            "alipay" | "ali" => {
                if !configs.alipay.is_configured() {
                    return Err(Error::biz(1001, "支付宝未配置"));
                }
                self.alipay_create(&order, &configs.alipay, client_type, &pay_sn).await?
            }
            "paypal" => {
                if !configs.paypal.is_configured() {
                    return Err(Error::biz(1001, "PayPal 未配置"));
                }
                self.paypal_create(&order, &configs.paypal, client_type, &pay_sn).await?
            }
            "stripe" => self.stripe_create(&order, client_type, &pay_sn).await?,
            "xendit" => self.xendit_create(&order, client_type, &pay_sn).await?,
            "antom" => self.antom_create(&order, user_id, client_type, &pay_sn).await?,
            other if matches!(other, "huifu" | "lakala" | "yeepay" | "yabandpay" | "yunpay") => {
                self.generic_channel_create(&order, other, client_type, &pay_sn).await?
            }
            _ => return Err(Error::biz_msg(MSG_CHANNEL_UNMIGRATED)),
        };
        Ok(PayCreateVO {
            order_id: Some(recharge_id),
            order_sn: Some(order.order_sn.clone()),
            order_amount: Some(amount),
            pay_info: Some(pay_info),
            code: None,
        })
    }

    /// 额外支付渠道的通用网关下单适配器（汇付/拉卡拉/易宝等）。
    async fn generic_channel_create(
        &self,
        order: &PayOrderRow,
        pay_code: &str,
        client_type: &str,
        pay_sn: &str,
    ) -> Result<serde_json::Value> {
        let url = match self.config_value(&format!("{pay_code}GatewayUrl")).await? {
            Some(v) => v,
            None => self
                .config_value("payChannelGatewayUrl")
                .await?
                .ok_or_else(|| Error::biz(1001, format!("支付渠道 {pay_code} 未配置网关地址")))?,
        };
        let key = self
            .config_value(&format!("{pay_code}GatewayKey"))
            .await?
            .or(self.config_value("payChannelGatewayKey").await?)
            .unwrap_or_default();
        let currency = self
            .config_value("payChannelCurrency")
            .await?
            .unwrap_or_else(|| "CNY".into());
        let pc_domain = self.config_value("pcDomain").await?.unwrap_or_default();
        let notify_url = if pc_domain.trim().is_empty() {
            String::new()
        } else {
            format!("{}/api/order/pay/notify/{pay_code}", pc_domain.trim_end_matches('/'))
        };
        let req = serde_json::json!({
            "paySn": pay_sn,
            "channel": pay_code,
            "amount": order.unpaid_amount.to_string(),
            "currency": currency,
            "subject": format!("NexTrade Order {}", order.order_sn),
            "clientType": client_type,
            "notifyUrl": notify_url
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut builder = client.post(url.trim()).header("Content-Type", "application/json");
        if !key.trim().is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", key.trim()));
        }
        let resp = builder
            .body(req.to_string())
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("{pay_code} 下单网络异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        if !status.is_success() {
            return Err(Error::biz(1001, format!("{pay_code} 下单失败: {text}")));
        }
        let pay_url = json
            .get("payUrl")
            .or_else(|| json.get("url"))
            .or_else(|| json.get("counterUrl"))
            .or_else(|| json.get("counter_url"))
            .or_else(|| json.get("data").and_then(|d| d.get("payUrl")))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if pay_url.is_empty() {
            return Err(Error::biz(1001, format!("{pay_code} 下单响应缺少支付地址")));
        }
        if let Some(tx) = json
            .get("transactionId")
            .or_else(|| json.get("orderId"))
            .or_else(|| json.get("tradeNo"))
            .and_then(serde_json::Value::as_str)
            .filter(|v| !v.is_empty())
        {
            let _ = sqlx::query("UPDATE paylog SET transaction_id=$1 WHERE pay_sn=$2")
                .bind(tx)
                .bind(pay_sn)
                .execute(&self.repos.pool)
                .await;
        }
        Ok(serde_json::json!({"url": pay_url, "payType": pay_code}))
    }

    /// 额外支付渠道的通用网关退款适配器。
    async fn generic_channel_refund(
        &self,
        pay_code: &str,
        pay_sn: &str,
        refund_sn: &str,
        amount: rust_decimal::Decimal,
    ) -> Result<String> {
        let url = match self.config_value(&format!("{pay_code}RefundUrl")).await? {
            Some(v) => v,
            None => match self.config_value(&format!("{pay_code}GatewayUrl")).await? {
                Some(v) => format!("{}/refund", v.trim_end_matches('/')),
                None => return Err(Error::biz(1001, format!("支付渠道 {pay_code} 未配置退款地址"))),
            },
        };
        let key = self
            .config_value(&format!("{pay_code}GatewayKey"))
            .await?
            .or(self.config_value("payChannelGatewayKey").await?)
            .unwrap_or_default();
        let req = serde_json::json!({
            "paySn": pay_sn,
            "refundSn": refund_sn,
            "channel": pay_code,
            "amount": amount.to_string()
        });
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut builder = client.post(url.trim()).header("Content-Type", "application/json");
        if !key.trim().is_empty() {
            builder = builder.header("Authorization", format!("Bearer {}", key.trim()));
        }
        let resp = builder
            .body(req.to_string())
            .send()
            .await
            .map_err(|e| Error::biz(1001, format!("{pay_code} 退款网络异常: {e}")))?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::biz(1001, format!("{pay_code} 退款失败: {text}")));
        }
        let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        Ok(json
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("SUCCESS")
            .to_string())
    }

    /// 创建通用待支付 paylog（渠道下单前落库）。
    async fn create_pending_paylog(
        &self,
        order: &PayOrderRow,
        pay_code: &str,
        pay_name: &str,
    ) -> Result<String> {
        use nextrade_core::models::payment::paylog::{generate_pay_sn, PayOrderType};
        use nextrade_infrastructure::db::repo::payment::paylog::PAYLOG_STATUS_UNPAID;
        let pay_sn = generate_pay_sn();
        let paylog = Paylog {
            paylog_id: 0,
            pay_sn: pay_sn.clone(),
            pay_name: pay_name.to_string(),
            order_id: order.order_id,
            order_sn: order.order_sn.clone(),
            order_amount: order.total_amount,
            order_type: PayOrderType::Product as i32,
            pay_amount: order.unpaid_amount,
            pay_status: PAYLOG_STATUS_UNPAID,
            pay_code: pay_code.to_string(),
            profit_sharing_channel: None,
            add_time: Some(nextrade_common::utils::now_seconds() as i32),
            transaction_id: String::new(),
            notify_data: String::new(),
            refund_amount: rust_decimal::Decimal::ZERO,
            token_code: String::new(),
            appid: None,
        };
        self.repos.payment.paylog.insert(&paylog).await?;
        Ok(pay_sn)
    }

    /// 预售定金支付创建（Java `createDepositPayment`）。
    pub async fn create_deposit_payment(
        &self,
        q: &PayCreateQuery,
        user_id: Option<i32>,
    ) -> Result<PayCreateVO> {
        self.validate_create_target(q, user_id).await?;
        let order_id = q.id.unwrap_or(0);
        let order = self.repo().find_order_by_id(order_id).await?
            .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
        if order.is_presale != 1 || order.presale_type != Some(2) {
            return Err(Error::biz(1001, "订单不是定金预售订单"));
        }
        let presale: Option<(Option<rust_decimal::Decimal>, i32)> = sqlx::query_as(
            "SELECT deposit_amount,presale_status FROM order_presale WHERE order_id=$1 LIMIT 1",
        ).bind(order_id).fetch_optional(&self.repos.pool).await
            .map_err(|e| Error::Database(e.to_string()))?;
        let (deposit, status) = presale.ok_or_else(|| Error::biz(1001, "预售订单信息不存在"))?;
        if status != 0 { return Err(Error::biz(1001, "定金已支付或预售已失效")); }
        let deposit = deposit.filter(|d| *d > rust_decimal::Decimal::ZERO)
            .ok_or_else(|| Error::biz(1001, "定金金额配置错误"))?;
        let pay_type = q.r#type.as_deref().unwrap_or("");
        if pay_type.is_empty() || pay_type == "cash" {
            return Err(Error::biz(1001, "该支付方式不支持定金支付"));
        }
        let client_type = q.code.as_deref().unwrap_or("pc");
        let mut pseudo = order.clone();
        pseudo.unpaid_amount = deposit;
        pseudo.total_amount = deposit;
        let pay_sn = self.create_pending_paylog(&pseudo, pay_type, &format!("预售定金-订单{}", order.order_sn)).await?;
        let configs = crate::payment::channels::load_channel_configs(&self.repos).await?;
        let uid = user_id.unwrap_or(order.user_id);
        let pay_info = match pay_type {
            "wechat" => { if !configs.wechat.is_configured() { return Err(Error::biz(1001, "微信支付未配置")); } self.wechat_create(&pseudo, &configs.wechat, client_type, uid, &pay_sn).await? }
            "alipay" | "ali" => { if !configs.alipay.is_configured() { return Err(Error::biz(1001, "支付宝未配置")); } self.alipay_create(&pseudo, &configs.alipay, client_type, &pay_sn).await? }
            "paypal" => { if !configs.paypal.is_configured() { return Err(Error::biz(1001, "PayPal 未配置")); } self.paypal_create(&pseudo, &configs.paypal, client_type, &pay_sn).await? }
            "stripe" => self.stripe_create(&pseudo, client_type, &pay_sn).await?,
            "xendit" => self.xendit_create(&pseudo, client_type, &pay_sn).await?,
            "antom" => self.antom_create(&pseudo, uid, client_type, &pay_sn).await?,
            other if matches!(other, "huifu" | "lakala" | "yeepay" | "yabandpay" | "yunpay") => self.generic_channel_create(&pseudo, other, client_type, &pay_sn).await?,
            _ => return Err(Error::biz(1001, "该支付方式暂不支持定金支付")),
        };
        Ok(PayCreateVO { order_id: Some(order.order_id), order_sn: Some(order.order_sn.clone()), order_amount: Some(deposit), pay_info: Some(pay_info), code: None })
    }

    /// 两条创建端点的共同本地校验：登录 + id + 订单归属 + 未支付。
    async fn validate_create_target(
        &self,
        q: &PayCreateQuery,
        user_id: Option<i32>,
    ) -> Result<()> {
        let user_id = require_user(user_id)?;
        let order_id = q
            .id
            .ok_or_else(|| Error::biz(500, MSG_PARAM_MISSING))?;
        let order = self
            .repo()
            .find_order_by_id(order_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_ORDER_NOT_FOUND))?;
        if order.user_id != user_id {
            return Err(Error::biz(500, MSG_ORDER_NOT_FOUND));
        }
        if order.pay_status == PAY_STATUS_PAID {
            return Err(Error::biz(500, MSG_ORDER_ALREADY_PAID));
        }
        let _ = (&q.r#type, &q.code);
        Ok(())
    }
}

/// URL 编码（支付宝网关 query 参数）。
fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 登录态校验（Java `requireUserId`）。
fn require_user(user_id: Option<i32>) -> Result<i32> {
    match user_id {
        Some(id) if id != 0 => Ok(id),
        _ => Err(Error::biz(500, MSG_NOT_LOGGED_IN)),
    }
}

/// 订单行 → 订单摘要 VO。
fn to_order_vo(order: &PayOrderRow) -> PayOrderVO {
    PayOrderVO {
        order_id: order.order_id,
        order_sn: order.order_sn.clone(),
        user_id: order.user_id,
        total_amount: order.total_amount,
        paid_amount: order.paid_amount,
        unpaid_amount: order.unpaid_amount,
        balance: order.balance,
        balance_amount: rust_decimal::Decimal::ZERO,
        pay_status: order.pay_status,
        order_status: order.order_status,
        is_presale: order.is_presale,
        presale_type: order.presale_type,
        pay_type_id: order.pay_type_id,
        add_time: format_cst(order.add_time),
        shipping_type_name: Some(order.shipping_type_name.clone()).filter(|s| !s.is_empty()),
    }
}

/// 秒级时间戳 → `YYYY-MM-DD HH:mm:ss`（UTC+8，Java `TigUtils.handelTime` 同口径）。
fn format_cst(sec: i32) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(i64::from(sec), 0)
        .map(|dt| (dt.naive_utc() + chrono::Duration::hours(8)).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// 复制 paylog 到 VO（Java `BeanUtils.copyProperties(one, target)`，`None` 时 no-op）。
fn copy_paylog(target: &mut PaylogVO, paylog: Option<&Paylog>) {
    let Some(source) = paylog else {
        return;
    };
    target.paylog_id = Some(source.paylog_id);
    target.pay_sn = Some(source.pay_sn.clone());
    target.pay_name = Some(source.pay_name.clone());
    target.order_id = Some(source.order_id);
    target.order_sn = Some(source.order_sn.clone());
    target.order_amount = Some(source.order_amount);
    target.order_type = Some(source.order_type);
    target.pay_amount = Some(source.pay_amount);
    target.pay_status = Some(source.pay_status);
    target.pay_code = Some(source.pay_code.clone());
    target.add_time = source.add_time.map(i64::from);
    target.transaction_id = Some(source.transaction_id.clone());
    target.notify_data = Some(source.notify_data.clone());
    target.refund_amount = Some(source.refund_amount);
    target.token_code = Some(source.token_code.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_cst_matches_java_handel_time() {
        // 2026-10-25 10:23:26 +08:00 == 1792899806
        assert_eq!(format_cst(1792899806), "2026-10-25 11:43:26");
        assert_eq!(format_cst(0), "1970-01-01 08:00:00");
    }


    #[test]
    fn require_user_rejects_guest() {
        assert_eq!(require_user(Some(7)).unwrap(), 7);
        assert_eq!(
            require_user(None).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_NOT_LOGGED_IN}")
        );
        assert_eq!(
            require_user(Some(0)).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_NOT_LOGGED_IN}")
        );
    }

    #[test]
    fn order_vo_maps_all_fields() {
        let row = PayOrderRow {
            order_id: 3,
            order_sn: "SN3".into(),
            user_id: 7,
            total_amount: rust_decimal::Decimal::new(10000, 2),
            paid_amount: rust_decimal::Decimal::ZERO,
            unpaid_amount: rust_decimal::Decimal::new(10000, 2),
            balance: rust_decimal::Decimal::ZERO,
            pay_status: 0,
            order_status: 1,
            pay_type_id: 1,
            is_presale: 0,
            presale_type: None,
            order_type: Some(1),
            parent_order_id: None,
            add_time: 1792899806,
            shipping_type_name: String::new(),
        };
        let vo = to_order_vo(&row);
        assert_eq!(vo.add_time, "2026-10-25 11:43:26");
        assert_eq!(vo.shipping_type_name, None);
        assert_eq!(vo.order_id, 3);
        assert_eq!(vo.order_sn, "SN3");
        assert_eq!(vo.total_amount, rust_decimal::Decimal::new(10000, 2));
        assert_eq!(vo.unpaid_amount, rust_decimal::Decimal::new(10000, 2));
    }

    #[test]
    fn copy_paylog_none_is_noop_and_some_fills() {
        let mut target = PaylogVO::default();
        copy_paylog(&mut target, None);
        assert_eq!(target.paylog_id, None);
        let source = Paylog {
            paylog_id: 9,
            pay_sn: "P9".into(),
            pay_name: "".into(),
            order_id: 3,
            order_sn: "SN3".into(),
            order_amount: rust_decimal::Decimal::new(10000, 2),
            order_type: 0,
            pay_amount: rust_decimal::Decimal::new(10000, 2),
            pay_status: 1,
            pay_code: "wechat".into(),
            profit_sharing_channel: None,
            add_time: Some(1_700_000_000),
            transaction_id: "T".into(),
            notify_data: "{}".into(),
            refund_amount: rust_decimal::Decimal::ZERO,
            token_code: "tk".into(),
            appid: None,
        };
        copy_paylog(&mut target, Some(&source));
        assert_eq!(target.paylog_id, Some(9));
        assert_eq!(target.add_time, Some(1_700_000_000));
        assert_eq!(target.pay_code.as_deref(), Some("wechat"));
    }
}
