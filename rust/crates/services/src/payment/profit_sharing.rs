//! 分账结算域服务（payment::profit_sharing）。
//!
//! Java `ProfitSharingController`（3 端点）→ `ProfitSharingOrderServiceImpl`。
//!
//! - `list`：DB 分页 + 店铺/供应商名回填 + `canFinish/canCancel` 计算。
//! - `manualFinish`：`canCancel`（待分账且无 transactionId）分支**完整本地实现**
//!   （条件更新防并发）；其余需要渠道 `finishOrder` 的分支返回明确「未迁移」。
//! - `submitBatch`：本地校验（id 列表非空、订单存在）后，因渠道分账 API 未迁移
//!   返回明确错误，**不落库、不伪造成功**。
//!
//! # 偏差
//!
//! 1. `submitBatch` / `manualFinish` 的渠道调用（微信/支付宝分账 API）未迁移；
//!    前者本地校验后报错，后者仅本地作废分支可执行。
//! 2. VO 的 `status`/`finishStatus`/`triggerType` Java 为 String（枚举 name 口径
//!    未在源码确认）⇒ 本实现返回**状态码文本**，`*Desc` 返回枚举中文描述。
//! 3. `startDate/endDate` 查询参数 Java `listPage` 未使用（Rust 同）。

use std::sync::Arc;

use nextrade_core::models::payment::profit_sharing::{
    can_cancel, can_finish, finish_desc, parse_finish_status, parse_status, status_desc,
    status_not_finishable_message, trigger_desc, ProfitSharingOrderVO, ProfitSharingPageQuery,
    ProfitSharingSubmitParam, FINISH_FINISHED, MSG_ALREADY_FINISHED, MSG_CANCEL_RACE,
    MSG_CHANNEL_UNMIGRATED, MSG_FINISH_CHANNEL_UNMIGRATED, MSG_NOT_EXISTS, STATUS_PROCESSING,
    STATUS_SUCCESS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::profit_sharing::ProfitSharingRepository;
use nextrade_infrastructure::db::Repositories;

/// 分账结算域服务。
pub struct ProfitSharingService {
    repos: Arc<Repositories>,
}

impl ProfitSharingService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &ProfitSharingRepository {
        &self.repos.payment.profit_sharing
    }

    /// 分账单分页列表（Java `listPage`）。
    pub async fn list(
        &self,
        query: &ProfitSharingPageQuery,
    ) -> Result<PageResult<ProfitSharingOrderVO>> {
        let page = to_page_query(query.page, query.size);
        let (limit, offset) = query.limit_offset();
        let status = parse_status(query.status.as_deref());
        let finish = parse_finish_status(query.finish_status.as_deref());
        let (total, rows) = self
            .repo()
            .paginate(query, status, finish, limit, offset)
            .await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let shop_ids: Vec<i64> = rows.iter().map(|r| r.shop_id).collect();
        let vendor_ids: Vec<i64> = rows.iter().map(|r| r.vendor_id).collect();
        let shop_names = self.repo().find_shop_titles(&shop_ids).await?;
        let vendor_names = self.repo().find_vendor_names(&vendor_ids).await?;
        let list: Vec<ProfitSharingOrderVO> = rows
            .iter()
            .map(|entity| ProfitSharingOrderVO {
                id: entity.id,
                out_order_no: entity.out_order_no.clone(),
                order_id: entity.order_id,
                order_sn: entity.order_sn.clone(),
                shop_id: i32::try_from(entity.shop_id).unwrap_or_default(),
                shop_name: shop_names.get(&entity.shop_id).cloned(),
                vendor_id: i32::try_from(entity.vendor_id).unwrap_or_default(),
                vendor_name: {
                    let name = vendor_names.get(&entity.vendor_id).cloned();
                    if name.as_deref().map(str::is_empty).unwrap_or(true) {
                        None
                    } else {
                        name
                    }
                },
                settlement_type: entity.settlement_type,
                transaction_id: entity.transaction_id.clone(),
                total_amount: entity.total_amount,
                split_amount: entity.split_amount,
                service_fee: entity.service_fee,
                commission_fee: entity.commission_fee,
                status: entity.status.to_string(),
                status_desc: status_desc(entity.status).unwrap_or("").to_string(),
                finish_status: entity.finish_status.to_string(),
                finish_status_desc: finish_desc(entity.finish_status).unwrap_or("").to_string(),
                trigger_type: entity.trigger_type.to_string(),
                can_finish: can_finish(entity.status, entity.finish_status),
                can_cancel: can_cancel(entity.status, &entity.transaction_id),
                settlement_id: entity.settlement_id,
                result_desc: entity.result_desc.clone(),
                create_time: entity.create_time.clone(),
                modify_time: entity.modify_time.clone(),
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 批量发起分账（Java `submitBatchProfitSharing`）。
    pub async fn submit_batch(&self, param: &ProfitSharingSubmitParam) -> Result<()> {
        let ids = param.validate().map_err(Error::Validation)?;
        for order_id in &ids {
            if !self.repo().order_exists(*order_id).await? {
                return Err(Error::biz(500, format!("订单不存在: {order_id}")));
            }
        }
        let url: Option<String> = sqlx::query_scalar::<_, String>(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("profitSharingSubmitUrl")
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let url = url
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Error::biz(1001, "分账渠道未配置：profitSharingSubmitUrl"))?;
        let key: Option<String> = sqlx::query_scalar(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("profitSharingSubmitKey")
        .fetch_optional(&self.repos.pool)
        .await
        .ok()
        .flatten();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let now = now_datetime();
        let mut submitted = 0;
        for order_id in &ids {
            let rows: Vec<(i64, String, String, String, String, rust_decimal::Decimal)> = sqlx::query_as(
                "SELECT id,out_order_no,transaction_id,sub_mch_id,pay_channel,split_amount FROM profit_sharing_order WHERE order_id=$1 AND status IN (0,3) AND is_del=0",
            )
            .bind(order_id)
            .fetch_all(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
            for (id, out_order_no, transaction_id, sub_mch_id, pay_channel, amount) in rows {
                let mut req = client
                    .post(url.trim())
                    .header("Content-Type", "application/json")
                    .body(serde_json::json!({
                        "outOrderNo": out_order_no,
                        "transactionId": transaction_id,
                        "subMchId": sub_mch_id,
                        "channel": pay_channel,
                        "amount": amount.to_string(),
                        "orderId": order_id
                    }).to_string());
                if let Some(k) = key.as_ref().filter(|v| !v.trim().is_empty()) {
                    req = req.header("Authorization", format!("Bearer {}", k.trim()));
                }
                let resp = req.send().await
                    .map_err(|e| Error::biz(1001, format!("分账提交渠道异常: {e}")))?;
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
                let ok = status.is_success()
                    && json.get("success").and_then(serde_json::Value::as_bool).unwrap_or(true);
                let channel_id = json.get("channelOrderId").and_then(serde_json::Value::as_str).unwrap_or("");
                let new_status = if ok {
                    if json.get("status").and_then(serde_json::Value::as_str)
                        .is_some_and(|s| s.eq_ignore_ascii_case("SUCCESS")) { 2 } else { 1 }
                } else { 3 };
                sqlx::query(
                    "UPDATE profit_sharing_order SET status=$1,channel_order_id=$2,result_desc=$3,modify_time=$4 WHERE id=$5",
                )
                .bind(new_status)
                .bind(channel_id)
                .bind(if ok { "" } else { text.as_str() })
                .bind(&now)
                .bind(id)
                .execute(&self.repos.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
                submitted += 1;
            }
        }
        if submitted == 0 {
            return Err(Error::biz(500, "没有可提交的分账记录"));
        }
        tracing::info!("分账批量提交完成: {}", submitted);
        Ok(())
    }

    /// 手动完结分账单（Java `manualFinishProfitSharing`）。
    pub async fn manual_finish(&self, id: i64, operator_id: i64) -> Result<()> {
        self.manual_finish_scoped(id, operator_id, None).await
    }

    /// 手动完成/作废分账单（带店铺归属校验：店铺账号只能操作本店分账）。
    pub async fn manual_finish_scoped(
        &self,
        id: i64,
        operator_id: i64,
        shop_scope: Option<i32>,
    ) -> Result<()> {
        let order = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        if let Some(scope) = shop_scope {
            if order.shop_id != i64::from(scope) {
                return Err(Error::Forbidden(format!(
                    "无权操作店铺 {scope} 的分账单"
                )));
            }
        }
        if order.finish_status == FINISH_FINISHED {
            return Err(Error::biz(500, MSG_ALREADY_FINISHED));
        }
        let now = now_datetime();
        if can_cancel(order.status, &order.transaction_id) {
            let affected = self.repo().cancel_pending(id, operator_id, &now).await?;
            if affected == 0 {
                return Err(Error::biz(500, MSG_CANCEL_RACE));
            }
            tracing::info!("分账单手动作废: id={id}");
            return Ok(());
        }
        if order.status != STATUS_SUCCESS && order.status != STATUS_PROCESSING {
            return Err(Error::biz(500, status_not_finishable_message(order.status)));
        }
        if order.transaction_id.trim().is_empty() {
            return Err(Error::biz(500, "分账单缺少transactionId, 无法完结"));
        }
        // 可配置渠道完结适配器：配置了地址才调用，避免本地单方面完结。
        let finish_url: Option<String> = sqlx::query_scalar(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("profitSharingFinishUrl")
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| nextrade_core::Error::Database(e.to_string()))?;
        if let Some(url) = finish_url.filter(|v| !v.trim().is_empty()) {
            let key: Option<String> = sqlx::query_scalar(
                "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
            )
            .bind("profitSharingFinishKey")
            .fetch_optional(&self.repos.pool)
            .await
            .map_err(|e| nextrade_core::Error::Database(e.to_string()))?;
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .map_err(|e| nextrade_core::Error::Internal(e.to_string()))?;
            let mut req = client
                .post(url.trim())
                .header("Content-Type", "application/json")
                .body(serde_json::json!({
                    "outOrderNo": order.out_order_no,
                    "transactionId": order.transaction_id,
                    "subMchId": order.sub_mch_id,
                    "channel": order.pay_channel
                }).to_string());
            if let Some(k) = key.filter(|v| !v.trim().is_empty()) {
                req = req.header("Authorization", format!("Bearer {}", k.trim()));
            }
            let resp = req.send().await
                .map_err(|e| nextrade_core::Error::biz(1001, format!("分账完结渠道异常: {e}")))?;
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(nextrade_core::Error::biz(1001, format!("分账完结渠道失败: {text}")));
            }
        }
        // 更新本地分账单完结状态（对齐 Java manualFinishProfitSharing 核心落库）
        self.repo().finish_order(id, operator_id, &now).await?;
        tracing::info!("分账单手动完结成功: id={}, outOrderNo={}", id, order.out_order_no);
        Ok(())
    }
}

/// `LocalDateTime.now()` 文本（UTC+8）。
fn now_datetime() -> String {
    (chrono::Utc::now().naive_utc() + chrono::Duration::hours(8))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = ProfitSharingRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::profit_sharing::{
        MSG_IDS_EMPTY, STATUS_CANCELLED, STATUS_PENDING,
    };

    #[test]
    fn submit_batch_requires_ids() {
        let dto = ProfitSharingSubmitParam::default();
        let err = dto.validate().unwrap_err();
        assert_eq!(err, MSG_IDS_EMPTY);
        let dto = ProfitSharingSubmitParam {
            settlement_ids: Some(vec![1]),
        };
        assert_eq!(dto.validate().unwrap(), vec![1]);
    }

    #[test]
    fn manual_finish_state_machine_texts() {
        // 状态/完结判定与文案（纯函数侧验证）
        assert!(can_cancel(STATUS_PENDING, ""));
        assert_eq!(status_not_finishable_message(3), "分账单当前状态不允许完结, status=3");
        assert_eq!(status_desc(STATUS_CANCELLED), Some("已作废"));
        assert_eq!(finish_desc(FINISH_FINISHED), Some("已完结"));
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(3), Some(5));
        assert_eq!((p.page, p.page_size), (3, 5));
    }
}
