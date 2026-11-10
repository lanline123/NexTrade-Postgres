//! POS 收银域服务（pos::cashier）。
//!
//! ⚠️ 无 Java 参考（POS 仅存在于 o2o.recy.cc 编译前端，两个 Java 工程均无
//! 控制器）。字段契约来源：前端 bundle + 真实表结构；挂单 / 桌台状态机
//! 的响应语义为推断实现，**需 POS 页面联调验证**。
//!
//! 购物车模型：工作草稿 = `pos_pending_order`（kind=0, status=0）最新一条 +
//! `pos_pending_order_item` 明细；`pendingOrder` 把草稿转成已保存挂单（kind=1），
//! `takePendingOrder` 把挂单明细并回新草稿并置 status=1。

use std::sync::Arc;

use rust_decimal::Decimal;
use nextrade_core::models::pos::cashier::*;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 整单改价：按当前行金额比例摊分到每一行（type 1=一口价总额，2=折扣百分比）。
fn total_change_prices(
    items: &[(i64, Decimal, i32)],
    change_type: i32,
    value: Decimal,
) -> Result<Vec<(i64, Decimal)>> {
    if value < Decimal::ZERO {
        return Err(Error::Validation("改价金额不能为负".into()));
    }
    let sum: Decimal = items
        .iter()
        .map(|(_, price, qty)| *price * Decimal::from((*qty).max(1)))
        .sum();
    if sum <= Decimal::ZERO {
        return Err(Error::Validation("购物车为空".into()));
    }
    let target = match change_type {
        1 => value,
        2 => (sum * value / Decimal::from(100)).round_dp(2),
        _ => return Err(Error::Validation("不支持的改价方式".into())),
    };
    if target > sum {
        return Err(Error::Validation("改价金额不能大于原金额".into()));
    }
    let mut out = Vec::with_capacity(items.len());
    let mut allocated = Decimal::ZERO;
    for (idx, (item_id, price, qty)) in items.iter().enumerate() {
        let quantity = (*qty).max(1);
        let new_price = if idx + 1 == items.len() {
            ((target - allocated) / Decimal::from(quantity)).round_dp(2)
        } else {
            (*price * target / sum).round_dp(2)
        };
        allocated += new_price * Decimal::from(quantity);
        out.push((*item_id, new_price));
    }
    Ok(out)
}

/// 兼容 MySQL 口径（字符串）与 PostgreSQL `numeric`（JSON 数字）两种价格形态。
fn json_decimal(value: Option<&serde_json::Value>) -> Decimal {
    match value {
        Some(serde_json::Value::String(text)) => text.parse().unwrap_or_default(),
        Some(serde_json::Value::Number(number)) => number.to_string().parse().unwrap_or_default(),
        _ => Decimal::default(),
    }
}

/// POS 收银域服务（挂载于 `AdminState.pos.cashier`）。
pub struct PosCashierService {
    repos: Arc<Repositories>,
}

impl PosCashierService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::pos::cashier::PosCashierRepository {
        &self.repos.pos.cashier
    }

    /// 当前工作草稿（kind=0, status=0）。
    async fn working_draft(
        &self,
        admin_id: i32,
    ) -> Result<Option<nextrade_core::entities::o2o::PosPendingOrder>> {
        self.repo().working_draft(admin_id).await
    }

    // ————————————————————— 收银员 —————————————————————

    pub async fn cashier_list(
        &self,
        q: &PosAdminUserQuery,
    ) -> Result<nextrade_core::types::PageResult<PosAdminUserVO>> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.size.unwrap_or(10).clamp(1, 100);
        let (total, rows) = self
            .repo()
            .cashier_paginate(q.keyword.as_deref(), q.status, size, (page - 1) * size)
            .await?;
        let total_u = total.max(0) as u64;
        let total_page = if size > 0 { (total_u + size - 1) / size } else { 0 };
        Ok(nextrade_core::types::PageResult {
            total: total_u,
            total_page,
            list: rows
                .into_iter()
                .map(|r| PosAdminUserVO {
                    id: r.id,
                    admin_id: r.admin_id,
                    shop_id: r.shop_id,
                    username: r.username,
                    email: r.email,
                    avatar: r.avatar,
                    is_using: r.is_using,
                    add_time: r.add_time,
                    role_id: r.role_id,
                })
                .collect(),
            page,
            page_size: size,
        })
    }

    pub async fn cashier_detail(&self, id: i32) -> Result<Option<PosAdminUserVO>> {
        Ok(self
            .repo()
            .cashier_find(id)
            .await?
            .map(|r| PosAdminUserVO {
                id: r.id,
                admin_id: r.admin_id,
                shop_id: r.shop_id,
                username: r.username,
                email: r.email,
                avatar: r.avatar,
                is_using: r.is_using,
                add_time: r.add_time,
                role_id: r.role_id,
            }))
    }

    pub async fn cashier_save(&self, dto: &PosAdminUserWriteDTO) -> Result<i32> {
        let e = nextrade_core::entities::o2o::AdminUserPos {
            id: dto.id.unwrap_or(0),
            admin_id: dto.admin_id.unwrap_or(0),
            shop_id: dto.shop_id.unwrap_or(0),
            username: dto.username.clone().unwrap_or_default(),
            email: dto.email.clone().unwrap_or_default(),
            is_using: dto.is_using.unwrap_or(0),
            role_id: dto.role_id.unwrap_or(0),
            ..Default::default()
        };
        if e.username.trim().is_empty() {
            return Err(Error::Validation("收银员名称不能为空".into()));
        }
        if e.id > 0 {
            self.repo()
                .cashier_update_field(e.id, "username", &e.username)
                .await?;
            self.repo()
                .cashier_update_field(e.id, "email", &e.email)
                .await?;
            return Ok(e.id);
        }
        self.repo().cashier_insert(&e).await
    }

    pub async fn cashier_update_field(&self, dto: &PosAdminUserUpdateFieldDTO) -> Result<()> {
        let field = dto.field.as_deref().unwrap_or_default().to_string();
        if !POS_ADMIN_USER_FIELDS.contains(&field.as_str()) {
            return Err(Error::Validation("不支持的字段".into()));
        }
        let val = dto
            .value
            .as_ref()
            .map(|v| match v {
                serde_json::Value::String(sv) => sv.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        self.repo().cashier_update_field(dto.id, &field, &val).await
    }

    pub async fn cashier_delete(&self, id: i32) -> Result<()> {
        self.repo().cashier_delete(id).await
    }

    // ————————————————————— 上下班 / 统计 —————————————————————

    pub async fn work_status(&self, admin_id: i32) -> Result<PosWorkStatusVO> {
        let flag = self.repo().work_flag(admin_id).await?;
        let mut vo = PosWorkStatusVO {
            working: false,
            work_time: None,
            off_time: None,
        };
        if let Some(raw) = flag {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                vo.working = v
                    .get("working")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let t = v.get("time").and_then(serde_json::Value::as_i64);
                vo.work_time = if vo.working { t } else { None };
                vo.off_time = if !vo.working { t } else { None };
            }
        }
        Ok(vo)
    }

    pub async fn set_work(&self, admin_id: i32, working: bool) -> Result<()> {
        self.repo().set_work_flag(admin_id, working).await
    }

    pub async fn statistics(&self, admin_id: i32) -> Result<PosStatisticsVO> {
        let day_start =
            (chrono::Utc::now().timestamp() / 86400 * 86400) as i32;
        let (order_count, total_amount) =
            self.repo().shift_statistics(admin_id, day_start).await?;
        Ok(PosStatisticsVO {
            order_count,
            total_amount,
            today: true,
        })
    }

    // ————————————————————— 会员 / 游客 —————————————————————

    pub async fn create_member(&self, dto: &PosCreateMemberDTO) -> Result<i32> {
        let mobile = dto.mobile.as_deref().unwrap_or_default().trim().to_string();
        if mobile.is_empty() {
            return Err(Error::Validation("手机号不能为空".into()));
        }
        let nickname = dto
            .username
            .clone()
            .unwrap_or_else(|| format!("会员{}", &mobile[mobile.len().saturating_sub(4)..]));
        let password = dto
            .password
            .clone()
            .unwrap_or_else(|| "123456".to_string());
        let hash = nextrade_security::password::hash_password(&password)
            .map_err(|e| Error::Internal(format!("密码哈希失败: {e}")))?;
        self.repo().user_insert_pos(&mobile, &nickname, &hash).await
    }

    pub async fn create_tourists(&self) -> Result<i32> {
        let stamp = chrono::Utc::now().timestamp_millis() as i64;
        let mobile = format!("T{stamp}");
        let nickname = format!("游客{}", &stamp.to_string()[stamp.to_string().len() - 6..]);
        let hash = nextrade_security::password::hash_password("123456")
            .map_err(|e| Error::Internal(format!("密码哈希失败: {e}")))?;
        self.repo().user_insert_pos(&mobile, &nickname, &hash).await
    }

    pub async fn users(&self, q: &PosUserQuery) -> Result<serde_json::Value> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.size.unwrap_or(10).clamp(1, 50);
        let (total, rows) = self
            .repo()
            .users(q.keyword.as_deref(), q.user_id, size, (page - 1) * size)
            .await?;
        Ok(serde_json::json!({ "records": rows, "total": total, "page": page, "size": size }))
    }

    pub async fn user_addresses(&self, user_id: i32) -> Result<serde_json::Value> {
        let rows = self.repo().user_addresses(user_id).await?;
        Ok(serde_json::json!({ "records": rows }))
    }

    // ————————————————————— 优惠券 / 充值 —————————————————————

    /// 可见优惠券：复用代购侧可见性查询（同一 `coupon` 表口径，时间过滤）。
    pub async fn coupon_visible(&self, _user_id: i32) -> Result<serde_json::Value> {
        let now = chrono::Utc::now().timestamp();
        let list = self
            .repos
            .trade
            .proxy_shopping_user_coupon
            .find_visible_coupons(now)
            .await?;
        Ok(serde_json::to_value(list).unwrap_or_else(|_| serde_json::json!([])))
    }

    pub async fn recharge_orders(&self, user_id: i32) -> Result<serde_json::Value> {
        let rows = self.repo().recharge_orders(user_id).await?;
        Ok(serde_json::json!({ "records": rows }))
    }

    /// 线下充值单落库（POS 收银）。
    pub async fn recharge_insert(
        &self,
        e: nextrade_core::entities::finance::UserRechargeOrder,
    ) -> Result<i32> {
        self.repos.payment.user_recharge_order.insert(&e).await
    }

    /// POS 线下现金充值：建已支付充值单并给会员余额入账 + 流水。
    pub async fn recharge(&self, dto: &PosRechargeCreateDTO) -> Result<i32> {
        if dto.user_id <= 0 {
            return Err(Error::Validation("请先选择会员".into()));
        }
        if dto.money <= Decimal::ZERO {
            return Err(Error::Validation("充值金额必须大于 0".into()));
        }
        let user = self
            .repos
            .member
            .user_manage
            .find_user(dto.user_id)
            .await?
            .ok_or_else(|| Error::NotFound("会员不存在".into()))?;
        let before = user.balance;
        let updated = self
            .repos
            .member
            .user_manage
            .apply_balance_delta(
                dto.user_id,
                nextrade_core::models::member::user_manage::CHANGE_TYPE_INC,
                dto.money,
            )
            .await?;
        if updated == 0 {
            return Err(Error::Conflict("余额入账失败，请重试".into()));
        }
        let now = chrono::Utc::now().timestamp() as i32;
        let _ = self
            .repos
            .member
            .user_manage
            .insert_balance_log(&nextrade_core::entities::UserBalanceLog {
                log_id: 0,
                user_id: dto.user_id,
                before_balance: before,
                balance: dto.money,
                after_balance: before + dto.money,
                frozen_balance: Decimal::ZERO,
                before_frozen_balance: Decimal::ZERO,
                after_frozen_balance: Decimal::ZERO,
                change_time: now,
                change_desc: "POS 收银充值".into(),
                change_type: nextrade_core::models::member::user_manage::CHANGE_TYPE_INC,
                wallet_type: 0,
                new_balance: before + dto.money,
                new_frozen_balance: Decimal::ZERO,
            })
            .await;
        let order = nextrade_core::entities::finance::UserRechargeOrder {
            user_id: dto.user_id,
            amount: dto.money,
            add_time: now,
            paid_time: now,
            postscript: "POS 收银充值".into(),
            status: 1,
            ..Default::default()
        };
        self.repos.payment.user_recharge_order.insert(&order).await
    }

    /// 最近订单备注（POS 收银台展示）。
    pub async fn order_remark(&self, user_id: i32) -> Result<Option<String>> {
        self.repos.pos.cashier.order_remark(user_id).await
    }

    /// 该管理员的 API Token 列表。
    pub async fn api_tokens(
        &self,
        admin_id: i32,
    ) -> Result<Vec<nextrade_core::entities::AdminApiToken>> {
        self.repos.identity.admin_user.find_api_tokens(admin_id).await
    }

    /// 吊销 Token。
    pub async fn api_token_revoke(&self, id: i32) -> Result<()> {
        self.repos.identity.admin_user.revoke_api_token(id).await?;
        Ok(())
    }

    pub async fn recharge_setting(&self) -> Result<serde_json::Value> {
        let raw = self
            .repos
            .system
            .config_basic
            .find_by_code("rechargeSetting")
            .await?;
        Ok(raw
            .and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
            .unwrap_or_else(|| serde_json::json!({})))
    }

    // ————————————————————— 购物车 / 挂单 —————————————————————

    fn to_item_vo(&self, i: nextrade_core::entities::o2o::PosPendingOrderItem) -> PosCartItemVO {
        let subtotal = i.price * Decimal::from(i.quantity);
        PosCartItemVO {
            cart_id: i.item_id,
            product_id: i.product_id,
            sku_id: i.sku_id,
            product_name: i.product_name.unwrap_or_default(),
            pic_thumb: i.pic_thumb.unwrap_or_default(),
            price: i.price,
            origin_price: i.origin_price,
            quantity: i.quantity,
            subtotal,
            is_checked: i.is_checked == 1,
            sku_data: i
                .sku_data
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }

    pub async fn cart(&self, admin_id: i32, user_id: i32, table_id: i32) -> Result<PosCartVO> {
        let draft = self
            .repo()
            .ensure_draft(admin_id, user_id, 0, table_id)
            .await?;
        if user_id > 0 || table_id > 0 {
            self.repo().draft_touch(draft.pending_id, user_id, table_id).await?;
        }
        let items = self.repo().items(draft.pending_id).await?;
        let carts: Vec<PosCartItemVO> = items.into_iter().map(|i| self.to_item_vo(i)).collect();
        let total_count = carts.iter().map(|c| c.quantity as i64).sum::<i64>();
        let checked_count = carts
            .iter()
            .filter(|c| c.is_checked)
            .map(|c| c.quantity as i64)
            .sum::<i64>();
        let product_amount = carts
            .iter()
            .filter(|c| c.is_checked)
            .map(|c| c.subtotal)
            .sum::<Decimal>();
        Ok(PosCartVO {
            pending_id: draft.pending_id,
            cart_list: vec![PosCartStoreVO {
                shop_id: 0,
                shop_title: String::new(),
                is_checked: !carts.is_empty() && carts.iter().all(|c| c.is_checked),
                carts,
            }],
            total: PosCartTotalVO {
                total_count,
                checked_count,
                product_amount,
                discount_after: product_amount,
            },
        })
    }

    pub async fn add_to_cart(&self, admin_id: i32, dto: &PosCartAddDTO) -> Result<i64> {
        if dto.product_id <= 0 {
            return Err(Error::Validation("缺少商品".into()));
        }
        let draft = self
            .repo()
            .ensure_draft(
                admin_id,
                dto.user_id.unwrap_or(0),
                dto.shop_id.unwrap_or(0),
                dto.table_id.unwrap_or(0),
            )
            .await?;
        let product = self
            .repo()
            .product_one(dto.product_id)
            .await?
            .ok_or_else(|| Error::NotFound("商品不存在".into()))?;
        let name = product
            .get("productName")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let pic = product
            .get("picThumb")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let price = json_decimal(product.get("productPrice"));
        let sku_id = dto.sku_id.unwrap_or(0);
        if let Some(existing) = self
            .repo()
            .item_merge_same(draft.pending_id, dto.product_id, sku_id, dto.quantity.unwrap_or(1))
            .await?
        {
            return Ok(existing);
        }
        self.repo()
            .item_insert(
                draft.pending_id,
                dto.product_id,
                sku_id,
                &name,
                &pic,
                price,
                price,
                dto.quantity.unwrap_or(1),
                None,
            )
            .await
    }

    pub async fn update_item(&self, _admin_id: i32, dto: &PosCartUpdateDTO) -> Result<()> {
        let item_id = dto
            .item_id
            .or(dto.cart_id)
            .ok_or_else(|| Error::Validation("缺少明细ID".into()))?;
        if let Some(qty) = dto.quantity {
            self.repo().item_update_quantity(item_id, qty).await?;
        }
        if let Some(checked) = dto.is_checked {
            self.repo().item_update_checked(item_id, checked).await?;
        }
        self.repo().draft_touch(draft.pending_id, 0, 0).await?;
        Ok(())
    }

    pub async fn remove_items(&self, _admin_id: i32, dto: &PosCartRemoveDTO) -> Result<()> {
        let ids = dto
            .item_ids
            .clone()
            .or_else(|| dto.cart_ids.clone())
            .unwrap_or_default();
        self.repo().item_delete_ids(&ids).await
    }

    pub async fn clear_cart(&self, admin_id: i32) -> Result<()> {
        self.clear_cart_for(admin_id, 0).await
    }

    pub async fn clear_cart_for(&self, admin_id: i32, table_id: i32) -> Result<()> {
        if let Some(draft) = self.working_draft_for(admin_id, table_id).await? {
            self.repo().item_clear(draft.pending_id).await?;
        }
        Ok(())
    }

    pub async fn change_price(&self, admin_id: i32, dto: &PosCartChangePriceDTO) -> Result<()> {
        if let (Some(change_type), Some(value)) =
            (dto.total_change_price_type, dto.change_price_value)
        {
            let Some(draft) = self
                .working_draft_for(admin_id, dto.table_id.unwrap_or(0))
                .await?
            else {
                return Ok(());
            };
            let rows: Vec<(i64, Decimal, i32)> = self
                .repo()
                .items(draft.pending_id)
                .await?
                .into_iter()
                .filter(|item| item.is_checked != 0)
                .map(|item| (item.item_id, item.price, item.quantity))
                .collect();
            for (item_id, new_price) in total_change_prices(&rows, change_type, value)? {
                self.repo().item_update_price(item_id, new_price).await?;
            }
            return Ok(());
        }
        let Some(price) = dto.price else {
            return Ok(());
        };
        let _ = dto.show_only; // 展示价与结算价未区分（单一 price 列）
        let item_id = dto
            .item_id
            .or(dto.cart_id)
            .ok_or_else(|| Error::Validation("缺少明细ID".into()))?;
        let _ = self.working_draft(admin_id).await?;
        self.repo().item_update_price(item_id, price).await
    }

    /// 整单改价预览：不落库，返回调整后的购物车。
    pub async fn change_price_preview(
        &self,
        admin_id: i32,
        dto: &PosCartChangePriceDTO,
    ) -> Result<serde_json::Value> {
        let cart = self
            .cart(
                admin_id,
                dto.user_id.unwrap_or(0),
                dto.table_id.unwrap_or(0),
            )
            .await?;
        let mut value = serde_json::to_value(cart).unwrap_or(serde_json::Value::Null);
        let (Some(change_type), Some(change_value)) =
            (dto.total_change_price_type, dto.change_price_value)
        else {
            return Ok(value);
        };
        let Some(draft) = self.working_draft(admin_id).await? else {
            return Ok(value);
        };
        let rows: Vec<(i64, Decimal, i32)> = self
            .repo()
            .items(draft.pending_id)
            .await?
            .into_iter()
            .filter(|item| item.is_checked != 0)
            .map(|item| (item.item_id, item.price, item.quantity))
            .collect();
        let prices = total_change_prices(&rows, change_type, change_value)?;
        let mut map = std::collections::HashMap::new();
        for (item_id, price) in &prices {
            map.insert(*item_id, *price);
        }
        let mut total = Decimal::ZERO;
        if let Some(stores) = value.get_mut("cartList").and_then(|v| v.as_array_mut()) {
            for store in stores {
                if let Some(carts) = store.get_mut("carts").and_then(|v| v.as_array_mut()) {
                    for item in carts {
                        let Some(id) = item.get("cartId").and_then(serde_json::Value::as_i64)
                        else {
                            continue;
                        };
                        let Some(price) = map.get(&id) else {
                            continue;
                        };
                        let qty = item
                            .get("quantity")
                            .and_then(serde_json::Value::as_i64)
                            .unwrap_or(1);
                        item["price"] = serde_json::json!(price);
                        item["subtotal"] = serde_json::json!(*price * Decimal::from(qty));
                        if item
                            .get("isChecked")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                        {
                            total += *price * Decimal::from(qty);
                        }
                    }
                }
            }
        }
        if let Some(t) = value.get_mut("total") {
            t["productAmount"] = serde_json::json!(total);
            t["discountAfter"] = serde_json::json!(total);
        }
        Ok(value)
    }

    pub async fn pending_order(&self, admin_id: i32, dto: &PosPendingOrderDTO) -> Result<()> {
        let Some(draft) = self
            .working_draft_for(admin_id, dto.table_id.unwrap_or(0))
            .await?
        else {
            return Ok(());
        };
        self.repo()
            .save_pending(draft.pending_id, dto.remark.as_deref().unwrap_or(""))
            .await
    }

    pub async fn pending_list(
        &self,
        admin_id: i32,
        shop_id: Option<i32>,
    ) -> Result<serde_json::Value> {
        let rows = self.repo().pending_list(admin_id, shop_id).await?;
        let mut out = Vec::with_capacity(rows.len());
        for p in rows {
            let items = self.repo().items(p.pending_id).await?;
            out.push(serde_json::json!({
                "pendingId": p.pending_id,
                "remark": p.remark,
                "userId": p.user_id,
                "tableId": p.table_id,
                "addTime": p.add_time,
                "count": items.iter().map(|i| i.quantity).sum::<i32>(),
                "amount": items
                    .iter()
                    .map(|i| i.price * Decimal::from(i.quantity))
                    .sum::<Decimal>(),
            }));
        }
        Ok(serde_json::json!({ "records": out }))
    }

    pub async fn take_pending(&self, admin_id: i32, pending_id: i32) -> Result<()> {
        self.repo().take_pending(admin_id, pending_id).await
    }

    pub async fn cancel_pending(&self, pending_id: i32) -> Result<()> {
        self.repo().cancel_pending(pending_id).await
    }

    // ————————————————————— 桌台 —————————————————————

    pub async fn areas(&self, shop_id: i32) -> Result<Vec<nextrade_core::entities::o2o::PosTableArea>> {
        self.repo().areas(shop_id).await
    }

    pub async fn area_save(&self, dto: &PosTableAreaWriteDTO) -> Result<i32> {
        let name = dto.area_name.clone().unwrap_or_default();
        if name.trim().is_empty() {
            return Err(Error::Validation("区域名称不能为空".into()));
        }
        match dto.area_id {
            Some(id) if id > 0 => {
                self.repo().area_update(id, &name, dto.sort.unwrap_or(0)).await?;
                Ok(id)
            }
            _ => self
                .repo()
                .area_insert(dto.shop_id.unwrap_or(0), &name, dto.sort.unwrap_or(0))
                .await,
        }
    }

    pub async fn area_delete(&self, id: i32) -> Result<()> {
        self.repo().area_delete(id).await
    }

    pub async fn tables(
        &self,
        shop_id: i32,
        q: &PosTableQuery,
    ) -> Result<Vec<nextrade_core::entities::o2o::PosTable>> {
        self.repo().tables(shop_id, q.area_id, q.status).await
    }

    pub async fn table_detail(&self, id: i32) -> Result<Option<nextrade_core::entities::o2o::PosTable>> {
        self.repo().table_find(id).await
    }

    pub async fn table_save(&self, dto: &PosTableWriteDTO) -> Result<i32> {
        let name = dto
            .table_no
            .clone()
            .or_else(|| dto.table_name.clone())
            .unwrap_or_default();
        if name.trim().is_empty() {
            return Err(Error::Validation("桌台名称不能为空".into()));
        }
        let capacity = dto.seat_count.or(dto.capacity).unwrap_or(4);
        let sort_order = dto.sort_order.or(dto.sort).unwrap_or(50);
        match dto.table_id {
            Some(id) if id > 0 => {
                self.repo()
                    .table_update(id, &name, dto.area_id.unwrap_or(0), capacity, sort_order)
                    .await?;
                Ok(id)
            }
            _ => self
                .repo()
                .table_insert(
                    dto.shop_id.unwrap_or(0),
                    dto.area_id.unwrap_or(0),
                    &name,
                    capacity,
                    sort_order,
                )
                .await,
        }
    }

    pub async fn table_update_field(&self, id: i32, field: &str, value: &str) -> Result<()> {
        self.repo().table_update_field(id, field, value).await
    }

    pub async fn table_delete(&self, id: i32) -> Result<()> {
        self.repo().table_delete(id).await
    }

    pub async fn table_operate(&self, dto: &PosTableOperateDTO) -> Result<()> {
        match dto.table_id {
            _ if dto.target_table_id.is_some() => {
                self.repo()
                    .table_transfer(dto.table_id, dto.target_table_id.unwrap_or(0))
                    .await
            }
            _ => {
                // open / bindUser / cancel / clear 共用：按是否带 userId 区分占用
                let occupied = dto.user_id.map(|v| v > 0).unwrap_or(false);
                self.repo()
                    .table_set_occupied(dto.table_id, occupied, dto.user_id.unwrap_or(0), 0)
                    .await
            }
        }
    }

    /// 开台：仅未开台的桌台可开，写入人数并置「待下单」。
    pub async fn table_open(&self, dto: &PosTableOperateDTO) -> Result<()> {
        let table = self
            .repo()
            .table_find(dto.table_id)
            .await?
            .ok_or_else(|| Error::NotFound("桌台不存在".into()))?;
        if table.table_status != 0 || table.status != 0 {
            return Err(Error::Conflict("桌台已开台".into()));
        }
        self.repo()
            .table_open_state(
                dto.table_id,
                dto.user_id.unwrap_or(0),
                dto.dining_count.unwrap_or(0).max(0),
            )
            .await
    }

    /// 清台：复位桌台状态。
    pub async fn table_clear_table(&self, table_id: i32) -> Result<()> {
        self.repo().table_clear_state(table_id).await
    }

    /// 转台：源桌台已开台且目标桌台空闲。
    pub async fn table_transfer_to(&self, from: i32, to: i32) -> Result<()> {
        if from <= 0 || to <= 0 || from == to {
            return Err(Error::Validation("请选择要转出的桌台和目标桌台".into()));
        }
        let source = self
            .repo()
            .table_find(from)
            .await?
            .ok_or_else(|| Error::NotFound("源桌台不存在".into()))?;
        let target = self
            .repo()
            .table_find(to)
            .await?
            .ok_or_else(|| Error::NotFound("目标桌台不存在".into()))?;
        if source.table_status == 0 {
            return Err(Error::Validation("请选择已开台的桌台".into()));
        }
        if target.table_status != 0 {
            return Err(Error::Conflict("目标桌台已被占用".into()));
        }
        self.repo().table_transfer(from, to).await
    }

    /// POS 收银下单：把当前草稿明细写入会员购物车，走真实下单管道，现金默认记已支付。
    pub async fn checkout(
        &self,
        admin_id: i32,
        dto: &PosCheckoutDTO,
    ) -> Result<serde_json::Value> {
        let checkout_table_id = dto.table_id.unwrap_or(0);
        let draft = self
            .working_draft_for(admin_id, checkout_table_id)
            .await?
            .ok_or_else(|| Error::Validation("购物车为空".into()))?;
        let items: Vec<_> = self
            .repo()
            .items(draft.pending_id)
            .await?
            .into_iter()
            .filter(|item| item.is_checked != 0)
            .collect();
        if items.is_empty() {
            return Err(Error::Validation("购物车为空".into()));
        }

        // 未绑定会员时自动落一个游客账号，保证订单有真实 user_id。
        let user_id = if draft.user_id > 0 {
            draft.user_id
        } else {
            self.create_tourists().await?
        };
        if draft.user_id != user_id || dto.table_id.unwrap_or(0) != draft.table_id {
            self.repo()
                .draft_touch(draft.pending_id, user_id, dto.table_id.unwrap_or(draft.table_id))
                .await?;
        }

        let shop_id = draft.shop_id.unwrap_or(0);
        let now = chrono::Utc::now().timestamp() as i32;
        let mut cart_ids = Vec::with_capacity(items.len());
        for item in &items {
            let product = self.repo().product_one(item.product_id).await?;
            let product_sn = product
                .as_ref()
                .and_then(|p| p.get("productSn"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let cart = nextrade_core::entities::Cart {
                cart_id: 0,
                user_id,
                product_id: item.product_id as i32,
                product_sn,
                pic_thumb: item.pic_thumb.clone(),
                market_price: None,
                original_price: item.price,
                quantity: item.quantity.max(1),
                sku_id: (item.sku_id > 0).then_some(item.sku_id as i32),
                sku_data: item.sku_data.clone(),
                product_type: 1,
                is_checked: 1,
                shop_id,
                r#type: 0,
                update_time: now,
                salesman_id: 0,
                extra_sku_data: None,
                extra_data: None,
                change_price_data: None,
                cycle_spec_id: None,
                redeem_card_id: None,
                is_wholesale_line: 0,
                wholesale_grade_id: 0,
            };
            cart_ids.push(self.repos.trade.cart.insert(&cart).await?);
        }

        let order_dto = nextrade_core::models::OrderCreateDTO {
            cart_ids: cart_ids.clone(),
            address_id: None,
            shipping_method: Some(2),
            delivery_type: Some(2),
            pickup_id: None,
            pickup_contact_name: None,
            pickup_contact_mobile: None,
            coupon_id: None,
            use_points: dto.use_points,
            buyer_note: dto.remark.clone(),
            pay_type_id: dto.pay_type_id,
            use_balance: dto.use_balance,
            use_coupon_ids: dto.use_coupon_ids.clone(),
            invoice_data: None,
            order_source: Some("pos".into()),
            pos_user_id: Some(admin_id),
        };

        let order_service = crate::trade::order::OrderService::from_repos(self.repos.clone());
        let created = match order_service.create(user_id, &order_dto).await {
            Ok(value) => value,
            Err(error) => {
                let _ = self.repos.trade.cart.delete(&cart_ids, user_id).await;
                return Err(error);
            }
        };

        let mark_paid = dto.mark_paid.unwrap_or(true);
        if mark_paid {
            order_service
                .on_payment_success(created.order_id, created.total_amount, None)
                .await?;
        }
        if let Some(table_id) = dto.table_id.filter(|id| *id > 0) {
            let _ = self
                .repo()
                .table_set_occupied(table_id, false, 0, created.order_id)
                .await;
        }

        let _ = self.repos.trade.cart.delete(&cart_ids, user_id).await;
        self.clear_cart_for(admin_id, checkout_table_id).await?;

        Ok(serde_json::json!({
            "orderId": created.order_id,
            "orderSn": created.order_sn,
            "totalAmount": created.total_amount,
            "onlineAmount": created.online_amount,
            "markPaid": mark_paid,
        }))
    }

    pub async fn place_order(&self, dto: &PosTableOperateDTO) -> Result<serde_json::Value> {
        let table = self
            .repo()
            .table_find(dto.table_id)
            .await?
            .ok_or_else(|| Error::NotFound("桌台不存在".into()))?;
        // 简化：生成挂单快照并占用桌台（正式订单创建需联调后接入下单管道）
        let draft = self
            .repo()
            .ensure_draft(0, dto.user_id.unwrap_or(0), table.shop_id, table.table_id)
            .await?;
        self.repo().save_pending(draft.pending_id, "桌台下单").await?;
        self.repo()
            .table_set_occupied(table.table_id, true, dto.user_id.unwrap_or(0), 0)
            .await?;
        Ok(serde_json::json!({ "pendingId": draft.pending_id }))
    }

    pub async fn receipt_list(&self, admin_id: i32) -> Result<serde_json::Value> {
        let rows = self.repo().pending_list(admin_id, None).await?;
        Ok(serde_json::to_value(rows).unwrap_or_else(|_| serde_json::json!([])))
    }

    // ————————————————————— 商品 / 订单辅助 —————————————————————

    pub async fn products(&self, q: &PosProductQuery) -> Result<serde_json::Value> {
        let page = q.page.unwrap_or(1).max(1);
        let size = q.size.unwrap_or(20).clamp(1, 100);
        let (total, rows) = self
            .repo()
            .products(q.keyword.as_deref(), q.category_id, size, (page - 1) * size)
            .await?;
        Ok(serde_json::json!({ "records": rows, "total": total, "page": page, "size": size }))
    }

    pub async fn product_detail(&self, product_id: i64) -> Result<serde_json::Value> {
        self.repo()
            .product_one(product_id)
            .await?
            .ok_or_else(|| Error::NotFound("商品不存在".into()))
    }

    pub async fn categories(&self) -> Result<serde_json::Value> {
        let rows = self.repo().categories().await?;
        Ok(serde_json::json!({ "records": rows }))
    }
}
