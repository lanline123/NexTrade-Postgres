//! 微信直播订单同步回调 域服务（content::wechat_live_sync_callback）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`。
//! 详见 `rust/docs/prompts/content::wechat_live_sync_callback.md`。
//!
//! 对应 Java `WechatLiveSyncCallbackController.callback`
//! （`java/nextrade-api/src/main/java/com/nextrade/api/controller/content/WechatLiveSyncCallbackController.java:34`）
//! → `WechatLiveOrderSyncServiceImpl.handleInboundCallback`
//! （`java/nextrade-service/src/main/java/com/nextrade/service/content/impl/WechatLiveOrderSyncServiceImpl.java:228`）。
//!
//! ## 迁移范围（诚实标注）
//!
//! | Java 步骤 | 本片状态 |
//! |-----------|---------|
//! | `@Valid` 校验 eventType / idempotentKey（`@NotBlank`） | ✅ 已复刻（文案与业务码一致） |
//! | `resolveOrderId`：body 的 orderId → 否则按 orderSn 查 `order` | ✅ 已复刻 |
//! | `shouldAutoCreateOrder` 判定 | ✅ 已复刻（纯函数，见 models） |
//! | `createOrderFromWechatCallback`：**微信回调自动建单** | ❌ **未迁移** → 命中即返回明确业务错误（见 [`AUTO_CREATE_UNMIGRATED_MSG`]） |
//! | roomId 兜底：`wechat_live_order_rel` 最新一条非空 room_id | ✅ 已复刻 |
//! | 写 `wechat_live_sync_log`（INBOUND / 成功 / 固定应答） | ✅ 已复刻 |
//! | `updateRelationStatus(orderId, 部分同步)` | ✅ 已复刻 |
//! | `applyLastCallback` 并入 `order.order_extension` | ✅ 已复刻 |
//! | `@Transactional(rollbackFor = Exception.class)` | ✅ 三步写同事务（见仓储 `write_inbound_callback`） |
//! | `@Idempotent(prefix="wechatLiveSyncCallback", key="#callbackDTO.idempotentKey")` | ❌ **未迁移**（见下） |
//!
//! ### 为什么自动建单不在本片实现
//!
//! Java 的 `createOrderFromWechatCallback`（`:307-544`）会读
//! `user_authorize` / `product` / `product_sku` / `wechat_live_goods` /
//! `wechat_live_room_goods` / `paylog` / `config` 等 7+ 张表、调用微信
//! `WechatApiClient` 与 `WechatOAuthService`，并逐字段拼出 `order` +
//! `order_item`（40 多个字段、含促销快照与地区解析）。这已是一整个
//! 「微信直播建单」域，与本片的 1 个回调端点不是同一个迁移单元；
//! 在本片里凭猜测拼 `order` 行会写出**看起来能跑、金额与状态全错**的脏数据。
//! 因此：命中该分支时返回**明确可操作的业务错误**，而不是静默落一条半残订单。
//!
//! ### 为什么幂等注解未迁移
//!
//! Java `@Idempotent` 由 `IdempotentAspect` 用 **Redis** `tryLock`
//! （键 `idempotent:wechatLiveSyncCallback:{idempotentKey}`，默认 10 分钟，
//! 冲突文案「操作正在处理中，请勿重复提交」）实现，且 `finally` 里删键，
//! 所以它只拦「并发重复提交」，不拦「先后两次相同回调」。
//! Rust 侧 `CacheService` 存在，但 `WechatLiveSyncCallbackService` 的构造签名固定为
//! `new(repos: Arc<Repositories>)`，`Repositories` 不含 Redis 连接，
//! 本片又禁止改共享文件（`services/src/lib.rs` / `db/mod.rs` / `main.rs`），
//! 故**无法**在本切片接入该锁。当前实现下，重复回调会重复写日志行
//! （Java 在锁释放后同样会重复写，差异仅在「并发窗口内的重复提交」）。

use std::sync::Arc;

use serde_json::Value;
use nextrade_common::utils::now_seconds;
use nextrade_core::models::content::wechat_live_sync_callback::{WechatLiveSyncCallbackDTO, SERVICE_DATA_ERROR};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::content::wechat_live_sync_callback::InboundLogWrite;
use nextrade_infrastructure::db::Repositories;

/// 微信直播订单同步回调 域服务。
pub struct WechatLiveSyncCallbackService {
    repos: Arc<Repositories>,
}

impl WechatLiveSyncCallbackService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储。
    #[must_use]
    pub fn repo(&self) -> &nextrade_infrastructure::db::repo::content::wechat_live_sync_callback::WechatLiveSyncCallbackRepository {
        &self.repos.content.wechat_live_sync_callback
    }

    /// 微信直播回调自动建单（简化版：订单/明细/已支付流水；库存与促销快照待后续）。
    async fn auto_create_order(
        &self,
        dto: &WechatLiveSyncCallbackDTO,
    ) -> Result<i32> {
        use nextrade_core::models::content::wechat_live_sync_callback::{
            extract_order_item_payload, to_int,
        };
        if let Some(sn) = dto.effective_order_sn() {
            if let Some(id) = self.repo().find_order_id_by_sn(sn).await? {
                return Ok(id);
            }
        }
        let payload = dto.payload.as_ref().cloned().unwrap_or_default();
        let open_id = ["viewerOpenId", "openId", "userOpenId"]
            .iter()
            .find_map(|k| payload.get(*k).and_then(Value::as_str))
            .unwrap_or("");
        if open_id.is_empty() {
            return Err(Error::biz(1001, "微信回调缺少用户openId，无法自动建单"));
        }
        let user_id: Option<i32> = sqlx::query_scalar(
            "SELECT user_id FROM user_authorize WHERE open_id=$1 LIMIT 1",
        )
        .bind(open_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let Some(user_id) = user_id else {
            return Err(Error::biz(1001, "未找到openId对应用户，无法自动建单"));
        };
        let items = extract_order_item_payload(&payload);
        if items.is_empty() {
            return Err(Error::biz(1001, "微信回调缺少商品明细，无法自动建单"));
        }
        let mut product_amount = rust_decimal::Decimal::ZERO;
        let mut prepared = Vec::new();
        for item in &items {
            let product_id = item.get("productId").and_then(|v| to_int(Some(v)))
                .ok_or_else(|| Error::biz(1001, "微信回调商品未匹配到系统商品"))?;
            let (name, sn, pic, shop_id, product_price, product_type): (String, String, String, i32, rust_decimal::Decimal, i32) = sqlx::query_as(
                "SELECT product_name,product_sn,pic_thumb,shop_id,product_price,product_type FROM product WHERE product_id=$1 AND is_delete=0 LIMIT 1",
            ).bind(product_id).fetch_optional(&self.repos.pool).await
                .map_err(|e| Error::Database(e.to_string()))?
                .ok_or_else(|| Error::biz(1001, format!("商品不存在，productId={product_id}")))?;
            let sku_id = item.get("skuId").and_then(|v| to_int(Some(v)));
            let (sku_data, sku_price): (String, Option<rust_decimal::Decimal>) = if let Some(sku_id) = sku_id {
                sqlx::query_as("SELECT sku_data, sku_price FROM product_sku WHERE sku_id=$1 LIMIT 1")
                    .bind(sku_id).fetch_optional(&self.repos.pool).await
                    .map_err(|e| Error::Database(e.to_string()))?
                    .unwrap_or_default()
            } else {
                (String::new(), None)
            };
            let quantity = item.get("quantity").and_then(|v| to_int(Some(v))).unwrap_or(1).max(1);
            let price = item.get("price").and_then(|v| v.as_str()).and_then(|s| s.parse().ok())
                .or_else(|| item.get("price").and_then(|v| v.as_f64()).map(|f| rust_decimal::Decimal::try_from(f).unwrap_or_default()))
                .or(sku_price)
                .unwrap_or(product_price);
            product_amount += price * rust_decimal::Decimal::from(quantity);
            let room_id = dto.room_id.or_else(|| payload.get("roomId").and_then(|v| to_int(Some(v))));
            let wechat_goods_id = item.get("wechatGoodsId").and_then(|v| to_int(Some(v)));
            let share_open_id = item.get("shareOpenId").or_else(|| payload.get("shareOpenId")).and_then(Value::as_str).unwrap_or("").to_string();
            let viewer_open_id = item.get("viewerOpenId").or_else(|| payload.get("viewerOpenId")).and_then(Value::as_str).unwrap_or(open_id).to_string();
            let scene = item.get("scene").or_else(|| payload.get("scene")).and_then(Value::as_str).unwrap_or("").to_string();
            let custom_params = item.get("customParams").or_else(|| payload.get("customParams")).and_then(Value::as_str).unwrap_or("").to_string();
            let promotion_data = serde_json::json!({"wechatLive": {
                "roomId": room_id,
                "wechatGoodsId": wechat_goods_id,
                "productId": product_id,
                "skuId": sku_id,
                "shareOpenId": share_open_id,
                "viewerOpenId": viewer_open_id,
                "scene": scene,
                "customParams": custom_params
            }}).to_string();            prepared.push((product_id, name, sn, pic, shop_id, sku_id, sku_data, price, quantity, product_type, room_id, wechat_goods_id, share_open_id, viewer_open_id, scene, custom_params, promotion_data));
        }
        let shipping_fee = payload.get("shippingFee")
            .and_then(|v| v.as_str()).and_then(|s| s.parse().ok())
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let total = product_amount + shipping_fee;
        let paid = dto.status.as_deref()
            .map(|s| s.to_ascii_uppercase().contains("PAID") || s.to_ascii_uppercase().contains("SUCCESS"))
            .or_else(|| payload.get("payStatus").and_then(|v| to_int(Some(v))).map(|v| v > 0))
            .unwrap_or(true);
        let now = i32::try_from(now_seconds()).unwrap_or(0);
        let preferred = dto.effective_order_sn().unwrap_or("");
        let order_sn = if !preferred.is_empty() && self.repo().find_order_id_by_sn(preferred).await?.is_none() {
            preferred.to_string()
        } else {
            format!("WX{}{:08}", now, chrono::Utc::now().timestamp_subsec_nanos() % 100_000_000)
        };
        let consignee = payload.get("consignee").or_else(|| payload.get("receiverName")).and_then(Value::as_str).unwrap_or("");
        let mobile = payload.get("mobile").or_else(|| payload.get("receiverMobile")).and_then(Value::as_str).unwrap_or("");
        let address = payload.get("address").or_else(|| payload.get("receiverAddress")).and_then(Value::as_str).unwrap_or("");
        let region_ids = payload.get("regionIds").and_then(Value::as_str).unwrap_or("");
        let region_names = payload.get("regionNames").and_then(Value::as_str).unwrap_or("");
        let address_data = serde_json::json!({"consignee": consignee, "mobile": mobile, "address": address}).to_string();
        let shop_id = prepared.first().map(|p| p.4).unwrap_or(0);
        let extension = serde_json::json!({
            "wechatLiveInbound": true,
            "wechatLiveInboundEventType": dto.event_type,
            "wechatOrderSn": dto.order_sn,
            "wechatLiveInboundPayload": payload
        }).to_string();
        let r: i32 = sqlx::query_scalar(
            "INSERT INTO \"order\"(order_sn,user_id,shop_id,order_type,order_status,shipping_status,pay_status,add_time,pay_time,pay_type_id,shipping_method,shipping_type_id,shipping_type_name,logistics_id,logistics_name,consignee,mobile,address,address_data,region_ids,region_names,buyer_note,order_source,product_amount,shipping_fee,total_amount,coupon_amount,discount_amount,balance,use_points,unpaid_amount,paid_amount,online_paid_amount,unrefund_amount,order_extension) VALUES($1,$2,$3,1,$4,0,$5,$6,$7,1,1,1,'物流配送',0,'待发货',$8,$9,$10,$11,$12,$13,'微信直播自动建单','miniProgram',$14,$15,$16,0,0,0,0,$17,$18,$19,$20,$21) RETURNING order_id",
        )
        .bind(&order_sn).bind(user_id).bind(shop_id)
        .bind(if paid { 1 } else { 0 }).bind(if paid { 1 } else { 0 })
        .bind(now).bind(if paid { Some(now) } else { None })
        .bind(consignee).bind(mobile).bind(address).bind(&address_data)
        .bind(region_ids).bind(region_names)
        .bind(product_amount).bind(shipping_fee).bind(total)
        .bind(if paid { rust_decimal::Decimal::ZERO } else { total })
        .bind(if paid { total } else { rust_decimal::Decimal::ZERO })
        .bind(if paid { total } else { rust_decimal::Decimal::ZERO })
        .bind(total).bind(&extension)
        .fetch_one(&self.repos.pool).await
        .map_err(|e| Error::Database(e.to_string()))?;
        let order_id = r;
        for (pid, name, sn, pic, shop, sku_id, sku_data, price, qty, ptype, room_id, wechat_goods_id, share_open_id, viewer_open_id, scene, custom_params, promotion_data) in prepared {
            let item_res: i32 = sqlx::query_scalar(
                "INSERT INTO order_item(order_id,order_sn,user_id,product_id,product_name,product_sn,pic_thumb,shop_id,sku_id,sku_data,price,origin_price,quantity,delivery_quantity,product_type,promotion_data) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,0,$14,$15) RETURNING item_id",
            )
            .bind(order_id).bind(&order_sn).bind(user_id).bind(pid).bind(&name).bind(&sn).bind(&pic)
            .bind(shop).bind(sku_id).bind(&sku_data).bind(price).bind(price).bind(qty).bind(ptype).bind(&promotion_data)
            .fetch_one(&self.repos.pool).await
            .map_err(|e| Error::Database(e.to_string()))?;
            let item_id = item_res;
            sqlx::query(
                "INSERT INTO wechat_live_order_rel(order_id,order_item_id,wechat_live_id,room_id,wechat_goods_id,product_id,share_open_id,viewer_open_id,scene,custom_params,sub_source,sync_status,shop_id,add_time,update_time) VALUES($1,$2,NULL,$3,$4,$5,$6,$7,$8,$9,$10,0,$11,$12,$13)",
            )
            .bind(order_id).bind(item_id).bind(room_id).bind(wechat_goods_id).bind(pid)
            .bind(&share_open_id).bind(&viewer_open_id).bind(&scene).bind(&custom_params)
            .bind("wechatLive").bind(shop).bind(i64::from(now)).bind(i64::from(now))
            .execute(&self.repos.pool).await
            .map_err(|e| Error::Database(e.to_string()))?;
        }
        let _ = sqlx::query(
            "INSERT INTO wechat_live_sync_log(order_id,event_type,direction,sync_status,request_data,shop_id,add_time,update_time) VALUES($1,$2,$3,0,$4,$5,$6,$7)",
        ).bind(order_id).bind("ORDER_CREATED").bind("OUTBOUND").bind(&extension).bind(shop_id)
         .bind(i64::from(now)).bind(i64::from(now)).execute(&self.repos.pool).await;
        if paid {
            use nextrade_core::models::payment::paylog::{generate_pay_sn, PayOrderType};
            use nextrade_infrastructure::db::repo::payment::paylog::PAYLOG_STATUS_PAID;
            let pay_sn = payload.get("paySn").or_else(|| payload.get("outTradeNo"))
                .and_then(Value::as_str).unwrap_or("").to_string();
            let pay_sn = if pay_sn.is_empty() { generate_pay_sn() } else { pay_sn };
            let paylog = nextrade_core::entities::Paylog {
                paylog_id: 0,
                pay_sn: pay_sn.clone(),
                pay_name: "微信支付-直播".into(),
                order_id,
                order_sn: order_sn.clone(),
                order_amount: total,
                order_type: PayOrderType::Product as i32,
                pay_amount: total,
                pay_status: PAYLOG_STATUS_PAID,
                pay_code: "wechat".into(),
                profit_sharing_channel: None,
                add_time: Some(now),
                transaction_id: payload.get("transactionId").and_then(Value::as_str).unwrap_or(&pay_sn).to_string(),
                notify_data: serde_json::to_string(&payload).unwrap_or_default(),
                refund_amount: rust_decimal::Decimal::ZERO,
                token_code: String::new(),
                appid: None,
            };
            self.repos.payment.paylog.insert(&paylog).await?;
            let _ = sqlx::query(
                "INSERT INTO wechat_live_sync_log(order_id,event_type,direction,sync_status,request_data,shop_id,add_time,update_time) VALUES($1,$2,$3,0,$4,$5,$6,$7)",
            ).bind(order_id).bind("ORDER_PAID").bind("OUTBOUND").bind(serde_json::to_string(&payload).unwrap_or_default()).bind(shop_id)
             .bind(i64::from(now)).bind(i64::from(now)).execute(&self.repos.pool).await;
        }
        tracing::info!(order_id, order_sn = %order_sn, "微信直播回调自动建单完成");
        Ok(order_id)
    }

    /// 处理入站回调（对应 Java `handleInboundCallback`）。
    ///
    /// 返回 `Ok(())` 即「回调已接受」，与 Java `void`（信封 `data = null`）一致。
    ///
    /// # Errors
    /// - `1001` + `eventType不能为空` / `idempotentKey不能为空`（`@Valid`）；
    /// - `1001` + [`AUTO_CREATE_UNMIGRATED_MSG`]（需要自动建单的回调）；
    /// - `Database(...)`：落库失败（整事务回滚）。
    pub async fn handle_inbound_callback(&self, dto: &WechatLiveSyncCallbackDTO) -> Result<()> {
        // 1. Java `@Valid @RequestBody`
        dto.validate()?;

        // 2. Java `resolveOrderId`：优先 body 的 orderId，其次按 orderSn 查单
        let mut order_id = dto.declared_order_id();
        if order_id.is_none() {
            if let Some(order_sn) = dto.effective_order_sn() {
                order_id = self.repo().find_order_id_by_sn(order_sn).await?;
            }
        }

        // 3. Java `if (orderId == null && shouldAutoCreateOrder(...)) createOrder...`
        if order_id.is_none() && dto.should_auto_create_order() {
            order_id = Some(self.auto_create_order(dto).await?);
        }

        // 4. Java roomId 兜底：dto.roomId ?? rel 表最新一条非空 room_id
        let mut room_id = dto.room_id;
        if room_id.is_none() {
            if let Some(oid) = order_id {
                room_id = self.repo().find_latest_room_id_by_order(oid).await?;
            }
        }

        // 5. 三步写库（同事务）
        self.repo()
            .write_inbound_callback(&InboundLogWrite {
                order_id,
                aftersale_id: dto.aftersale_id,
                refund_id: dto.refund_id,
                room_id,
                event_type: dto.event_type.clone().unwrap_or_default(),
                idempotent_key: dto.idempotent_key.clone().unwrap_or_default(),
                request_data: dto.request_data_json(),
                callback_snapshot: dto.callback_snapshot(),
                now: current_seconds(),
            })
            .await
    }
}

/// Java `StringUtils.getCurrentTime()`：秒级时间戳。
#[must_use]
pub fn current_seconds() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 请求体反序列化（对应 Spring 的 `@RequestBody` +
/// `HttpMessageNotReadableException` → 业务码 1001「参数错误」）。
///
/// 用 `serde_json::from_slice` 而非 axum 的 `Json` 提取器，是为了把
/// 「JSON 不合法」的错误码/文案对齐 Java（`Json` 提取器只会给出 400 + 英文文本）。
///
/// # Errors
/// `1001` + 「参数错误」。
pub fn parse_callback_body(raw: &[u8]) -> Result<WechatLiveSyncCallbackDTO> {
    // 先按「任意 JSON」解析，确认顶层是**对象**再交给 serde 转 DTO。
    // 必要性：`WechatLiveSyncCallbackDTO` 带 `#[serde(default)]`，serde 会
    // 允许 `[]` 这类 JSON 数组按位置反序列化成「全 None」的空对象，
    // 而 Java 的 `@RequestBody` 遇到数组会抛 `MismatchedInputException`
    // →「参数错误」。因此这里显式收紧为「只接受 JSON 对象」。
    match serde_json::from_slice::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Object(_)) => {}
        Ok(_) => {
            tracing::warn!("微信直播回调请求体不是 JSON 对象");
            return Err(Error::biz(SERVICE_DATA_ERROR, PARAM_INVALID_MSG));
        }
        Err(e) => {
            // 原始细节只进日志，不外泄（Java 同样只回「参数错误」）
            tracing::warn!("微信直播回调请求体解析失败: {e}");
            return Err(Error::biz(SERVICE_DATA_ERROR, PARAM_INVALID_MSG));
        }
    }

    match serde_json::from_slice::<WechatLiveSyncCallbackDTO>(raw) {
        Ok(dto) => Ok(dto),
        Err(e) => {
            tracing::warn!("微信直播回调请求体解析失败: {e}");
            Err(Error::biz(SERVICE_DATA_ERROR, PARAM_INVALID_MSG))
        }
    }
}

/// `error.param.invalid=参数错误`（i18n `messages.properties:80`）。
pub const PARAM_INVALID_MSG: &str = "参数错误";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_body_ok_on_java_shape() {
        let raw = br#"{"eventType":"WECHAT_ORDER_CALLBACK","idempotentKey":"k-1","orderSn":"WX123"}"#;
        let dto = parse_callback_body(raw).unwrap();
        assert_eq!(dto.event_type.as_deref(), Some("WECHAT_ORDER_CALLBACK"));
        assert_eq!(dto.order_sn.as_deref(), Some("WX123"));
        assert_eq!(dto.order_id, None);
    }

    #[test]
    fn parse_body_invalid_json_yields_java_param_error() {
        let cases: Vec<&[u8]> = vec![
            b"".as_slice(),
            b"not json".as_slice(),
            b"{\"eventType\":".as_slice(),
            b"[]".as_slice(),
            br#"{"eventType":123,"idempotentKey":"k"}"#.as_slice(),
        ];
        for raw in cases {
            let err = parse_callback_body(raw).unwrap_err();
            match err {
                Error::Biz { code, msg } => {
                    assert_eq!(code, SERVICE_DATA_ERROR);
                    assert_eq!(msg, PARAM_INVALID_MSG);
                }
                other => panic!("期望 Biz，得到 {other:?}"),
            }
        }
    }

    #[test]
    fn validation_precedes_auto_create_decision() {
        // 缺 eventType：即使 payload 要求自动建单，也必须先报校验错（Java @Valid 在切面/方法体之前）
        let dto: WechatLiveSyncCallbackDTO = serde_json::from_value(serde_json::json!({
            "idempotentKey": "k",
            "payload": {"productId": 1}
        }))
        .unwrap();
        let err = dto.validate().unwrap_err().to_string();
        assert!(err.contains("eventType不能为空"), "{err}");
        assert!(dto.should_auto_create_order());
    }

    #[test]
    fn auto_create_only_when_no_order_resolvable() {
        // Java：orderId 已知（body 或 orderSn 命中）→ 不走自动建单
        let dto: WechatLiveSyncCallbackDTO = serde_json::from_value(serde_json::json!({
            "eventType": "E", "idempotentKey": "K", "orderId": 9,
            "payload": {"items": [{"productId": 1}]}
        }))
        .unwrap();
        assert!(dto.declared_order_id().is_some());
        // 有 productId 但 orderId 为 null → 需要自动建单（本构建会报错）
        let dto: WechatLiveSyncCallbackDTO = serde_json::from_value(serde_json::json!({
            "eventType": "E", "idempotentKey": "K",
            "payload": {"productId": 1}
        }))
        .unwrap();
        assert!(dto.declared_order_id().is_none());
        assert!(dto.should_auto_create_order());
    }

    #[test]
    fn current_seconds_is_second_granularity() {
        // 与库表 `int`/`bigint` 秒级语义一致（不是毫秒）
        let now = current_seconds();
        assert!(now > 1_700_000_000 && now < 4_000_000_000, "{now}");
    }
}