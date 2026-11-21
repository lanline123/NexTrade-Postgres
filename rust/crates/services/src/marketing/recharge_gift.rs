//! 充值赠品领取记录服务（marketing::recharge_gift）。
//!
//! 【我方新增 2026-11-21，论坛帖 717721「余额充值送实物礼品」；原版无此功能】
//!
//! 生命周期：
//! - 发放：充值单服务（payment::user_recharge_order）在充值单置「已支付」
//! 的入账事务内调用 [`RechargeGiftService::grant_in_tx`]，按充值单快照的
//! 档位 id 取档，档位配了实物赠品（gift_id > 0）才生成待领取记录；
//! `recharge_order_id` 唯一约束 + ON CONFLICT DO NOTHING 保证重复置
//! 已支付不重复发放。金额赠送（discount_money）与实物赠品相互独立、
//! 可叠加，发放只处理实物部分。
//! - 领取：会员选收货地址后领取。库存口径 = **领取时扣减**活动赠品池
//! `product_gift.gift_stock`（与商城下单链对赠品行的既定口径一致：
//! 赠品只扣赠品池、不扣商品库存）；不做充值时预留，避免长期占用库存
//! 且与赠品池既有扣减原语同源。扣减成功 → 同事务生成 ¥0 履约订单
//! （order_type = 12、order_source = "rechargeGift"、包邮）走商家正常
//! 发货链；库存不足 → 记录置「异常」并提交（后台记录页可见可处置），
//! 向会员返回库存不足提示。
//! - 撤销联动：充值单从「已支付」改回待确认/无效时，充值单服务在同一
//! 事务内调用 [`RechargeGiftService::void_open_in_tx`]，未领取/异常
//! 记录自动作废；**已领取记录不自动追回**——履约单已进入商家发货链，
//! 且现行充值撤销口径本身也不自动扣回已入账余额，赠品侧保持一致，
//! 由后台在记录页结合履约单人工处置（记录保留充值单/履约单关联）。

use std::sync::Arc;

use rust_decimal::Decimal;
use serde_json::json;
use sqlx::Transaction;

use nextrade_common::utils::{self, now_seconds};
use nextrade_core::entities::{
    Order, OrderItem, RechargeGiftRecord, UserAddress, UserRechargeOrder,
};
use nextrade_core::models::marketing::recharge_gift::{
    limit_offset, validate_claim, validate_restore, RechargeGiftAdminActionDTO,
    RechargeGiftClaimDTO, RechargeGiftListQuery, RechargeGiftRecordVO, GIFT_ORDER_SOURCE,
    GIFT_ORDER_TYPE, MSG_ADDRESS_NOT_EXISTS, MSG_ADDRESS_REQUIRED, MSG_RECHARGE_NOT_PAID,
    MSG_RECORD_NOT_EXISTS, MSG_STATUS_INVALID, MSG_STOCK_NOT_ENOUGH, STATUS_ABNORMAL,
    STATUS_CLAIMED, STATUS_PENDING, STATUS_VOID,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::recharge_gift::RechargeGiftRepository;
use nextrade_infrastructure::db::Repositories;

/// 充值赠品领取记录服务。
pub struct RechargeGiftService {
    repos: Arc<Repositories>,
}

impl RechargeGiftService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &RechargeGiftRepository {
        &self.repos.marketing.recharge_gift
    }

    fn now() -> i32 {
        i32::try_from(now_seconds()).unwrap_or_default()
    }

    /// 发放（充值入账事务内调用）：按充值单快照档位生成待领取记录。
    /// 无档位快照 / 档位无实物赠品 → 不生成（返回 false），金额赠送不受影响。
    /// 赠品池条目已被删除时仍生成记录但直接置「异常」，让后台可见可处置，
    /// 不让已付款会员的赠品静默消失。
    pub async fn grant_in_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        order: &UserRechargeOrder,
    ) -> Result<bool> {
        if order.recharge_id <= 0 {
            return Ok(false);
        }
        let Some(setting) = self
            .repos
            .marketing
            .recharge_setting
            .find_by_id(order.recharge_id)
            .await?
        else {
            return Ok(false);
        };
        if setting.gift_id <= 0 || setting.gift_quantity < 1 {
            return Ok(false);
        }
        let now = Self::now();
        let mut record = RechargeGiftRecord {
            recharge_order_id: order.order_id,
            recharge_id: order.recharge_id,
            user_id: order.user_id,
            gift_id: setting.gift_id,
            quantity: setting.gift_quantity,
            status: STATUS_PENDING,
            add_time: now,
            update_time: now,
            ..Default::default()
        };
        match self
            .repos
            .marketing
            .product_gift
            .find_by_id(setting.gift_id)
            .await?
        {
            Some(gift) => {
                record.gift_name = gift.gift_name;
                record.product_id = gift.product_id;
                record.sku_id = gift.sku_id;
                record.shop_id = gift.shop_id;
            }
            None => {
                record.status = STATUS_ABNORMAL;
                record.admin_note = "发放时赠品已不存在，请后台核查档位配置".to_string();
            }
        }
        self.repo().insert_in_tx(tx, &record).await
    }

    /// 撤销联动（充值单离开「已支付」的同一事务内调用）：未领取/异常记录
    /// 自动作废；已领取记录保持不变（口径见模块注释）。
    pub async fn void_open_in_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        recharge_order_id: i32,
    ) -> Result<u64> {
        self.repo()
            .void_open_by_recharge_order_in_tx(
                tx,
                recharge_order_id,
                "充值单已撤销/置无效，赠品自动作废",
                Self::now(),
            )
            .await
    }

    /// 会员端列表（强制本人）。
    pub async fn my_list(
        &self,
        user_id: i32,
        query: &RechargeGiftListQuery,
    ) -> Result<PageResult<RechargeGiftRecordVO>> {
        if user_id <= 0 {
            return Err(Error::biz(401, "请先登录"));
        }
        let mut q = query.clone();
        q.user_id = Some(user_id);
        self.list_inner(&q).await
    }

    /// 后台列表（可按会员/状态筛选，异常记录在此可见）。
    pub async fn admin_list(
        &self,
        query: &RechargeGiftListQuery,
    ) -> Result<PageResult<RechargeGiftRecordVO>> {
        self.list_inner(query).await
    }

    async fn list_inner(
        &self,
        query: &RechargeGiftListQuery,
    ) -> Result<PageResult<RechargeGiftRecordVO>> {
        let (total, rows) = self.repo().paginate(query).await?;
        let product_ids: Vec<i32> = rows.iter().map(|r| r.product_id).collect();
        let pics: std::collections::HashMap<i32, String> = self
            .repo()
            .product_pics(&product_ids)
            .await?
            .into_iter()
            .collect();
        let list = rows
            .iter()
            .map(|r| {
                let mut vo = RechargeGiftRecordVO::from(r);
                vo.gift_pic = pics.get(&r.product_id).cloned().filter(|p| !p.is_empty());
                vo
            })
            .collect();
        let (size, offset) = limit_offset(query.page, query.size);
        let page = nextrade_core::types::PageQuery {
            page: u64::try_from(if size > 0 { offset / size + 1 } else { 1 }).unwrap_or(1),
            page_size: u64::try_from(size.max(1)).unwrap_or(15),
        };
        Ok(PageResult::new(
            u64::try_from(total).unwrap_or(0),
            list,
            &page,
        ))
    }

    /// 会员领取：扣赠品池库存 → 生成 ¥0 履约订单 → 记录置已领取（同事务）。
    /// 库存不足时记录置异常并提交事务，再返回业务错误提示。
    pub async fn claim(
        &self,
        user_id: i32,
        dto: &RechargeGiftClaimDTO,
    ) -> Result<RechargeGiftRecordVO> {
        if user_id <= 0 {
            return Err(Error::biz(401, "请先登录"));
        }
        let record_id = dto
            .record_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        let address_id = dto
            .address_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(500, MSG_ADDRESS_REQUIRED))?;

        let mut tx = self.repo().begin().await?;
        let mut record = self
            .repo()
            .find_by_id_in_tx(&mut tx, record_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        validate_claim(&record, user_id).map_err(Error::Validation)?;

        let address: Option<UserAddress> = sqlx::query_as(
            "SELECT * FROM user_address WHERE address_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(address_id)
        .bind(user_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let address = address.ok_or_else(|| Error::biz(500, MSG_ADDRESS_NOT_EXISTS))?;

        let now = Self::now();
        let deducted = self
            .repos
            .marketing
            .product_gift
            .dec_gift_stock_in_tx(&mut tx, record.gift_id, record.quantity)
            .await?;
        if !deducted {
            // 库存不足：记录置异常并提交（后台可见），会员侧收到明确提示。
            record.status = STATUS_ABNORMAL;
            record.admin_note = "领取时赠品库存不足".to_string();
            record.update_time = now;
            self.repo().update_in_tx(&mut tx, &record).await?;
            tx.commit()
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            return Err(Error::biz(500, MSG_STOCK_NOT_ENOUGH));
        }

        // 履约单商品行快照（商品已删时回退到记录里的赠品名快照）。
        let product = self
            .repos
            .marketing
            .product_gift
            .find_product(record.product_id)
            .await?;
        let (product_name, product_sn, pic_thumb) = match &product {
            Some(p) => (
                p.product_name.clone(),
                p.product_sn.clone(),
                p.pic_thumb.clone(),
            ),
            None => (record.gift_name.clone(), String::new(), String::new()),
        };

        let order_sn = utils::generate_order_sn();
        let order = Order {
            order_sn: order_sn.clone(),
            user_id,
            shop_id: record.shop_id,
            // 已支付待发货：商家在订单链直接发货；金额全 0、包邮。
            order_status: 1,
            pay_status: 2,
            pay_time: now,
            total_amount: Decimal::ZERO,
            product_amount: Decimal::ZERO,
            paid_amount: Decimal::ZERO,
            unpaid_amount: Decimal::ZERO,
            balance: Decimal::ZERO,
            consignee: address.consignee.clone(),
            address: address.address.clone(),
            region_ids: address.region_ids.clone(),
            region_names: address.region_names.clone(),
            mobile: address.mobile.clone(),
            add_time: now,
            order_type: Some(GIFT_ORDER_TYPE),
            order_source: GIFT_ORDER_SOURCE.to_string(),
            order_extension: json!({
            "rechargeGiftRecordId": record.record_id,
            "rechargeOrderId": record.recharge_order_id,
            })
            .to_string(),
            ..Default::default()
        };
        let order_item = OrderItem {
            order_sn,
            user_id,
            price: Decimal::ZERO,
            quantity: record.quantity,
            product_id: record.product_id,
            product_name,
            product_sn,
            pic_thumb,
            sku_id: record.sku_id,
            product_type: 1,
            is_gift: 1,
            shop_id: record.shop_id,
            origin_price: Some(Decimal::ZERO),
            promotion_data: Some(
                json!({ "giftId": record.gift_id, "rechargeGiftRecordId": record.record_id})
                    .to_string(),
            ),
            ..Default::default()
        };
        let order_id = self
            .repos
            .trade
            .order
            .insert_order_in_tx(&mut tx, &order, &[order_item])
            .await?;

        record.status = STATUS_CLAIMED;
        record.order_id = order_id;
        record.address_id = address_id;
        record.consignee = address.consignee.clone();
        record.mobile = address.mobile.clone();
        record.address_text = format!("{} {}", address.region_names, address.address)
            .trim()
            .to_string();
        record.claim_time = now;
        record.update_time = now;
        self.repo().update_in_tx(&mut tx, &record).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let mut vo = RechargeGiftRecordVO::from(&record);
        vo.gift_pic = product.map(|p| p.pic_thumb).filter(|p| !p.is_empty());
        Ok(vo)
    }

    /// 后台作废（待领取/异常 → 已作废）。
    pub async fn admin_void(&self, dto: &RechargeGiftAdminActionDTO) -> Result<()> {
        let record_id = dto
            .record_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        let mut tx = self.repo().begin().await?;
        let mut record = self
            .repo()
            .find_by_id_in_tx(&mut tx, record_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        if record.status != STATUS_PENDING && record.status != STATUS_ABNORMAL {
            return Err(Error::biz(500, MSG_STATUS_INVALID));
        }
        record.status = STATUS_VOID;
        record.admin_note = dto
            .note
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "后台作废".to_string());
        record.update_time = Self::now();
        self.repo().update_in_tx(&mut tx, &record).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 后台恢复（异常/作废 → 待领取）：关联充值单必须仍为已支付，
    /// 防止已撤销充值的赠品被恢复发放。
    pub async fn admin_restore(&self, dto: &RechargeGiftAdminActionDTO) -> Result<()> {
        let record_id = dto
            .record_id
            .filter(|id| *id > 0)
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        let mut tx = self.repo().begin().await?;
        let mut record = self
            .repo()
            .find_by_id_in_tx(&mut tx, record_id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_RECORD_NOT_EXISTS))?;
        validate_restore(&record).map_err(Error::Validation)?;
        let recharge = self
            .repos
            .payment
            .user_recharge_order
            .find_by_id(record.recharge_order_id)
            .await?;
        if recharge.map(|r| r.status) != Some(1) {
            return Err(Error::biz(500, MSG_RECHARGE_NOT_PAID));
        }
        record.status = STATUS_PENDING;
        record.admin_note = dto.note.clone().unwrap_or_default();
        record.update_time = Self::now();
        self.repo().update_in_tx(&mut tx, &record).await?;
        tx.commit()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::recharge_gift::MSG_NOT_PENDING;

    #[test]
    fn claim_state_machine_guards() {
        // 领取校验与模型层同源：非待领取一律拒绝，文案一致。
        let record = RechargeGiftRecord {
            record_id: 1,
            user_id: 7,
            status: STATUS_CLAIMED,
            ..Default::default()
        };
        assert_eq!(
            validate_claim(&record, 7).unwrap_err(),
            MSG_NOT_PENDING.to_string()
        );
    }
}
