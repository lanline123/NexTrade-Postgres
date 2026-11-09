//! 前台充值服务（payment::recharge_order，7 个 `optional` 端点）。
//!
//! 对应原 Java `com.nextrade.api.controller.user.RechargeOrderController`
//! → `UserRechargeOrderServiceImpl` client 部分 + `RechargeSettingServiceImpl#settingPage`
//! + `PaymentServiceImpl#getAvailablePayment("recharge")`。
//!
//! # 已实现（与 Java 逐条对齐）
//!
//! - `list`：`user_withdraw_apply(wallet_type=0)` + `user_recharge_order` 合并，
//!   按 `addTimeLong` 倒序**内存分页**（Java 同款），`status <> -1`；
//!   类型文案「充值 / 提现」+ 状态文案。
//! - `setting`：`recharge_setting.is_show = 1` 分页（请求参数 isShow 未使用，Java 同款）。
//! - `paymentList`：`available_payments(configs, clientType, "recharge")`（无 offline）。
//! - `update`：金额精确命中 `money` → 取 discount；否则 `recharge_id + is_show=1`；
//!   缺失 →「充值金额错误」；落库 postscript「会员充值」、add_time=now，返回 orderId。
//! - `pay`：orderId/userId 校验 →「参数缺失」；订单不存在「充值申请不存在」；
//!   status=1「充值申请已支付」；用户不存在「用户不存在」。
//! - `checkStatus`：同上归属校验；status>0 直接返回；否则查 paylog（orderType=1）。
//! - `create`：type 空「未选择支付方式」；cash「请重新选择充值支付方式」；
//!   其余本地校验后返回渠道未迁移错误。
//!
//! # 偏差
//! - 微信付款码轮询分支（`checkStatus` 内 `wechatPayService.payPlatformStatus`）未迁移：
//!   查到非 wechat 或非付款码场景直接返回本地 status（Java 亦是该回退）。
//! - `create` 各渠道（微信/支付宝/云闪付……）支付单创建依赖渠道 SDK，统一返回
//!   `MSG_CHANNEL_UNMIGRATED`。

use std::sync::Arc;

use nextrade_core::models::payment::pay::{
    available_payments, CFG_USE_ALIPAY, CFG_USE_APPLE_PAY, CFG_USE_HUIFU, CFG_USE_LAKALA,
    CFG_USE_OFFLINE, CFG_USE_PAYPAL, CFG_USE_WECHAT, CFG_USE_YABANPAY, CFG_USE_YABANPAY_ALIPAY,
    CFG_USE_YABANPAY_WECHAT, CFG_USE_YEEPAY, CFG_USE_YUNPAY,
};
use nextrade_core::models::payment::recharge_order::{
    format_time, recharge_status_text, withdraw_status_text, ClientRechargeListQuery,
    ClientRechargePayQuery, ClientUserRechargeOrderVO, RechargeCreatePayDTO, RechargePayUserVO,
    RechargePayVO, RechargeSettingPageQuery, RechargeSettingVO, RechargeUpdateOrderVO,
    UserRechargeOrderPayVO, MSG_AMOUNT_INVALID, MSG_CASH_NOT_ALLOWED, MSG_CHANNEL_UNMIGRATED,
    MSG_PARAM_MISSING, MSG_PAY_METHOD_NOT_SELECTED, MSG_RECHARGE_NOT_EXIST, MSG_RECHARGE_PAID,
    MSG_USER_NOT_EXIST,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

pub struct RechargeOrderService {
    repos: Arc<Repositories>,
}

impl RechargeOrderService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /api/user/rechargeOrder/list`：充值/提现记录合并分页。
    pub async fn list(
        &self,
        q: &ClientRechargeListQuery,
    ) -> Result<PageResult<ClientUserRechargeOrderVO>> {
        let user_id = authenticated_user(q.user_id)?;
        let withdraws = self
            .repos
            .payment
            .recharge_order
            .list_withdraw(user_id, q.status)
            .await?;
        let recharges = self
            .repos
            .payment
            .recharge_order
            .list_recharge(user_id, q.status)
            .await?;
        let mut merged: Vec<ClientUserRechargeOrderVO> = Vec::new();
        for w in &withdraws {
            merged.push(ClientUserRechargeOrderVO {
                status_type: withdraw_status_text(w.status).to_string(),
                amount: w.amount,
                add_time: format_time(w.add_time),
                add_time_long: i64::from(w.add_time),
                postscript: w.postscript.clone(),
                status: w.status,
                r#type: "提现".to_string(),
            });
        }
        for r in &recharges {
            merged.push(ClientUserRechargeOrderVO {
                status_type: recharge_status_text(r.status).to_string(),
                amount: r.amount,
                add_time: format_time(r.add_time),
                add_time_long: i64::from(r.add_time),
                postscript: r.postscript.clone(),
                status: r.status,
                r#type: "充值".to_string(),
            });
        }
        merged.sort_by(|a, b| b.add_time_long.cmp(&a.add_time_long));
        let total = merged.len() as u64;
        let (size, skip) = (q.size.unwrap_or(15).max(1), ((q.page.unwrap_or(1).max(1)) - 1) * q.size.unwrap_or(15).max(1));
        let list = merged
            .into_iter()
            .skip(usize::try_from(skip).unwrap_or(0))
            .take(usize::try_from(size).unwrap_or(15))
            .collect();
        Ok(PageResult {
            total,
            total_page: total.div_ceil(size as u64),
            list,
            page_size: size as u64,
            page: q.page.unwrap_or(1).max(1) as u64,
        })
    }

    /// `GET /api/user/rechargeOrder/setting`：充值配置分页。
    pub async fn setting(
        &self,
        q: &RechargeSettingPageQuery,
    ) -> Result<PageResult<RechargeSettingVO>> {
        let page = q.page.unwrap_or(1);
        let size = q.size.unwrap_or(15);
        let (total, rows) = self
            .repos
            .payment
            .recharge_order
            .paginate_settings(page, size)
            .await?;
        let list = rows
            .into_iter()
            .map(|r| RechargeSettingVO {
                recharge_id: r.recharge_id,
                money: r.money,
                discount_money: r.discount_money,
                sort_order: r.sort_order,
                is_show: r.is_show,
            })
            .collect();
        let size = size.max(1) as u64;
        Ok(PageResult {
            total: total.max(0) as u64,
            total_page: (total.max(0) as u64).div_ceil(size),
            list,
            page_size: size,
            page: page.max(1) as u64,
        })
    }

    /// `GET /api/user/rechargeOrder/paymentList`：可用支付方式（无 offline）。
    pub async fn payment_list(&self, client_type: &str) -> Result<Vec<String>> {
        let configs = self.load_payment_configs().await?;
        Ok(available_payments(&configs, client_type, "recharge"))
    }

    /// `POST /api/user/rechargeOrder/update`：创建/获取充值单。
    pub async fn update(&self, dto: &nextrade_core::models::payment::recharge_order::ClientRechargeUpdateDTO) -> Result<RechargeUpdateOrderVO> {
        let user_id = authenticated_user(dto.user_id)?;
        let (amount, discount_money) = if let Some(amount) = dto.amount {
            let discount = self
                .repos
                .payment
                .recharge_order
                .find_setting_by_money(amount)
                .await?
                .and_then(|s| s.discount_money)
                .unwrap_or_default();
            (amount, discount)
        } else {
            let id = dto.id.ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))?;
            let setting = self
                .repos
                .payment
                .recharge_order
                .find_setting_by_id_show(id)
                .await?
                .ok_or_else(|| Error::biz_msg(MSG_AMOUNT_INVALID))?;
            (
                setting.money.unwrap_or_default(),
                setting.discount_money.unwrap_or_default(),
            )
        };
        let now = nextrade_common::utils::now_seconds() as i32;
        let order_id = self
            .repos
            .payment
            .recharge_order
            .insert_recharge(user_id, amount, discount_money, now, "会员充值")
            .await?;
        Ok(RechargeUpdateOrderVO { order_id })
    }

    /// `POST /api/user/rechargeOrder/pay`：充值支付信息。
    pub async fn pay(&self, q: &ClientRechargePayQuery) -> Result<RechargePayVO> {
        let user_id = authenticated_user(q.user_id)?;
        let order_id = q.order_id.ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))?;
        let order = self
            .repos
            .payment
            .recharge_order
            .find_recharge(order_id, user_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_RECHARGE_NOT_EXIST))?;
        if order.status == 1 {
            return Err(Error::biz_msg(MSG_RECHARGE_PAID));
        }
        let user = self
            .repos
            .payment
            .recharge_order
            .find_user(order.user_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_USER_NOT_EXIST))?;
        let payment_list = self.payment_list("h5").await?;
        Ok(RechargePayVO {
            order: UserRechargeOrderPayVO {
                status_type: recharge_status_text(order.status).to_string(),
                order_id: order.order_id,
                user_id: order.user_id,
                amount: order.amount,
                discount_money: order.discount_money,
                pay_type: None,
                cash_amount: None,
                cash_change_amount: None,
                add_time: format_time(order.add_time),
                paid_time: format_time(order.paid_time),
                postscript: order.postscript.clone(),
                status: order.status,
                user: Some(RechargePayUserVO {
                    username: user.username,
                    nickname: user.nickname,
                    user_id: user.user_id,
                    mobile: user.mobile,
                }),
            },
            payment_list,
        })
    }

    /// `GET /api/user/rechargeOrder/checkStatus`：充值单支付状态。
    pub async fn check_status(&self, q: &ClientRechargePayQuery) -> Result<i32> {
        let user_id = authenticated_user(q.user_id)?;
        let order_id = q.order_id.ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))?;
        let order = self
            .repos
            .payment
            .recharge_order
            .find_recharge(order_id, user_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_RECHARGE_NOT_EXIST))?;
        if order.status > 0 {
            return Ok(order.status);
        }
        // orderType=1（充值单）最近一次 paylog；微信付款码轮询分支未迁移（见模块头偏差）
        let _paylog = self
            .repos
            .payment
            .paylog
            .find_last_by_order(order_id, 1)
            .await?;
        Ok(order.status)
    }

    /// `POST /api/user/rechargeOrder/create`：创建渠道支付单（渠道 SDK 未迁移）。
    pub async fn create(&self, dto: &RechargeCreatePayDTO) -> Result<nextrade_core::models::payment::pay::PayCreateVO> {
        let user_id = authenticated_user(dto.user_id)?;
        let order_id = dto.id.ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))?;
        let order = self
            .repos
            .payment
            .recharge_order
            .find_recharge(order_id, user_id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_RECHARGE_NOT_EXIST))?;
        if order.status == 1 {
            return Err(Error::biz_msg(MSG_RECHARGE_PAID));
        }
        let pay_type = dto
            .r#type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::biz_msg(MSG_PAY_METHOD_NOT_SELECTED))?;
        if pay_type == "cash" {
            return Err(Error::biz_msg(MSG_CASH_NOT_ALLOWED));
        }
        let configs = self.load_payment_configs().await?;
        if !available_payments(&configs, "h5", "recharge")
            .iter()
            .any(|m| m == pay_type)
        {
            return Err(Error::biz_msg(MSG_CASH_NOT_ALLOWED));
        }
        let client_type = dto.code.as_deref().unwrap_or("h5");
        crate::payment::pay::PayService::new(self.repos.clone())
            .create_recharge_payment(order_id, user_id, order.amount, pay_type, client_type)
            .await
    }

    async fn load_payment_configs(&self) -> Result<std::collections::HashMap<String, String>> {
        // 复用 payment::pay 仓储的配置读取（含微信/支付宝等开关）
        self.repos
            .payment
            .pay
            .config_values(&[
                CFG_USE_WECHAT,
                CFG_USE_ALIPAY,
                CFG_USE_PAYPAL,
                CFG_USE_APPLE_PAY,
                CFG_USE_LAKALA,
                CFG_USE_YABANPAY,
                CFG_USE_YABANPAY_WECHAT,
                CFG_USE_YABANPAY_ALIPAY,
                CFG_USE_YUNPAY,
                CFG_USE_HUIFU,
                CFG_USE_YEEPAY,
                CFG_USE_OFFLINE,
            ])
            .await
    }
}

/// Java `pay`/`checkStatus` 的 userId 来源：登录态注入；游客/缺失 →「参数缺失」。
fn authenticated_user(user_id: Option<i32>) -> Result<i32> {
    user_id
        .filter(|id| *id > 0)
        .ok_or_else(|| Error::biz_msg(MSG_PARAM_MISSING))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticated_user_requires_positive_id() {
        assert_eq!(authenticated_user(Some(3)).unwrap(), 3);
        assert!(authenticated_user(Some(0)).is_err());
        assert!(authenticated_user(None).is_err());
        assert!(format!("{}", authenticated_user(None).unwrap_err()).contains(MSG_PARAM_MISSING));
    }

    #[test]
    fn list_merge_sorts_and_pages_in_memory() {
        // 复用模型层 merge_and_page 覆盖 Java 的内存分页语义
        use nextrade_core::models::payment::recharge_order::merge_and_page;
        let mk = |t: i64| ClientUserRechargeOrderVO {
            add_time_long: t,
            ..Default::default()
        };
        let page = merge_and_page(vec![mk(1), mk(3), mk(2)], 2, 2);
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].add_time_long, 1);
    }
}
