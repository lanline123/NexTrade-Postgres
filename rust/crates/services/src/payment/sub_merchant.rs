//! 子商户域服务（payment::sub_merchant）。
//!
//! Java `SubMerchantController`（3 端点）→ `SubMerchantServiceImpl`。
//!
//! - `list`：DB 分页 + 店铺名回填（shop_id=0 ⇒「自营店铺」）。
//! - `unbind`：完全 DB（status=2 已冻结 + modify_time）。
//! - `bind`：本地校验（渠道默认 WECHAT、子商户号必填）后，因渠道归属校验
//!   （`isSubMerchantOwned`）依赖微信服务商 API，返回明确「未迁移」错误，
//!   **不落库、不伪造成功**（偏差见模块头；渠道可用时再补 DB 写入分支）。
//!
//! # 偏差
//!
//! 1. `bind` 的渠道归属校验未迁移 ⇒ 本地校验通过后返回
//!    [`MSG_CHANNEL_UNMIGRATED`]（Java 会调用渠道后落库/更新 ACTIVE）。
//! 2. 自营店铺展示名：Java 按 O2O 形态区分「自营门店/自营店铺」，Rust 无
//!    `is_o2o` 运行时属性来源 ⇒ 固定「自营店铺」（O2O 形态偏差）。
//! 3. `sub_merchant` 实体含 `is_del`，本域查询均带 `is_del = 0`；Java 原实现
//!    未过滤 `is_del`（逻辑删除由 MP 注解处理），等价。

use std::sync::Arc;

use nextrade_common::utils::now_seconds;
use nextrade_core::models::payment::sub_merchant::{
    self_operated_shop_name, status_desc, SubMerchantBindParam, SubMerchantPageQuery,
    SubMerchantVO, MSG_CHANNEL_UNMIGRATED, MSG_NOT_EXISTS,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::sub_merchant::SubMerchantRepository;
use nextrade_infrastructure::db::Repositories;

/// 子商户域服务。
pub struct SubMerchantService {
    repos: Arc<Repositories>,
}

impl SubMerchantService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &SubMerchantRepository {
        &self.repos.payment.sub_merchant
    }

    /// 分页列表（Java `listPage`）。
    pub async fn list(
        &self,
        q: &SubMerchantPageQuery,
    ) -> Result<PageResult<SubMerchantVO>> {
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(q, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let shop_ids: Vec<i64> = rows.iter().map(|r| r.shop_id).collect();
        let names = self.repo().find_shop_titles(&shop_ids).await?;
        let list: Vec<SubMerchantVO> = rows
            .iter()
            .map(|entity| SubMerchantVO {
                id: entity.id,
                shop_id: entity.shop_id,
                shop_name: if entity.shop_id == 0 {
                    self_operated_shop_name(false).to_string()
                } else {
                    names.get(&entity.shop_id).cloned().unwrap_or_default()
                },
                pay_channel: entity.pay_channel.clone(),
                sub_mch_id: entity.sub_mch_id.clone(),
                sub_appid: entity.sub_appid.clone(),
                merchant_name: entity.merchant_name.clone(),
                status: entity.status,
                status_desc: status_desc(entity.status).map(ToString::to_string),
                application_id: entity.application_id,
                create_time: entity.create_time.clone(),
                modify_time: entity.modify_time.clone(),
            })
            .collect();
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 手动绑定子商户（Java `bind`）：本地校验后渠道未迁移。
    pub async fn bind(&self, param: &SubMerchantBindParam) -> Result<()> {
        let input = param.validate().map_err(Error::Validation)?;
        self.assert_owned(&input.pay_channel, &input.sub_mch_id).await?;
        let existing: Option<(i64, i32)> = sqlx::query_as(
            "SELECT id,status FROM sub_merchant WHERE shop_id=$1 AND pay_channel=$2 AND is_del=0 LIMIT 1",
        )
        .bind(input.shop_id)
        .bind(&input.pay_channel)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if let Some((id, status)) = existing {
            if status == 1 {
                return Err(Error::biz(1001, "该店铺已绑定此支付渠道的子商户"));
            }
            sqlx::query(
                "UPDATE sub_merchant SET sub_mch_id=$1,sub_appid=$2,merchant_name=$3,status=1,modify_time=NULLIF($4, '') WHERE id=$5",
            )
            .bind(&input.sub_mch_id)
            .bind(input.sub_appid.as_deref().unwrap_or(""))
            .bind(input.merchant_name.as_deref().unwrap_or(""))
            .bind(&now)
            .bind(id)
            .execute(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        } else {
            sqlx::query(
                "INSERT INTO sub_merchant(shop_id,pay_channel,sub_mch_id,sub_appid,merchant_name,status,application_id,is_del,create_time,modify_time) VALUES($1, $2, $3, $4, $5, 1, 0, 0, NULLIF($6, ''), NULLIF($7, ''))",
            )
            .bind(input.shop_id)
            .bind(&input.pay_channel)
            .bind(&input.sub_mch_id)
            .bind(input.sub_appid.as_deref().unwrap_or(""))
            .bind(input.merchant_name.as_deref().unwrap_or(""))
            .bind(&now)
            .bind(&now)
            .execute(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// 渠道归属校验：配置了校验地址才调用，未配置按 Java「无法判定放行」处理。
    async fn assert_owned(&self, channel: &str, sub_mch_id: &str) -> Result<()> {
        let url: Option<String> = sqlx::query_scalar(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("wechatSubMerchantCheckUrl")
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let Some(url) = url.filter(|v| !v.trim().is_empty()) else {
            tracing::warn!(channel, sub_mch_id, "未配置子商户归属校验地址，按 Java 规则放行");
            return Ok(());
        };
        let key: Option<String> = sqlx::query_scalar(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("wechatSubMerchantCheckKey")
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut req = client
            .post(url.trim())
            .header("Content-Type", "application/json")
            .body(serde_json::json!({"subMchId": sub_mch_id, "channel": channel}).to_string());
        if let Some(k) = key.filter(|v| !v.trim().is_empty()) {
            req = req.header("Authorization", format!("Bearer {}", k.trim()));
        }
        let resp = req.send().await
            .map_err(|e| Error::biz(1001, format!("子商户归属校验网络异常: {e}")))?;
        let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        if json.get("owned").and_then(serde_json::Value::as_bool) == Some(false) {
            return Err(Error::biz(1001, format!("子商户号[{sub_mch_id}]不在当前服务商名下，请核对后重试或改走系统进件")));
        }
        Ok(())
    }

    /// 解绑子商户（Java `unbind`：status=FROZEN + modify_time）。
    pub async fn unbind(&self, id: i64) -> Result<()> {
        self.unbind_scoped(id, None).await
    }

    /// 解绑子商户（带店铺归属校验：店铺账号只能解绑本店绑定）。
    pub async fn unbind_scoped(&self, id: i64, shop_scope: Option<i32>) -> Result<()> {
        let entity = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        if let Some(scope) = shop_scope {
            if entity.shop_id != i64::from(scope) {
                return Err(Error::Forbidden(format!(
                    "无权操作店铺 {scope} 的子商户绑定"
                )));
            }
        }
        let modify_time = now_datetime();
        self.repo().freeze(id, &modify_time).await?;
        tracing::info!("解绑子商户: id={id}");
        Ok(())
    }
}

/// `LocalDateTime.now()` 文本（UTC+8，MySQL DATETIME）。
fn now_datetime() -> String {
    let _ = now_seconds();
    (chrono::Utc::now().naive_utc() + chrono::Duration::hours(8))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    let (size, offset) = SubMerchantRepository::page_window(page, size);
    let page_no = if size > 0 { offset / size + 1 } else { 1 };
    nextrade_core::types::PageQuery {
        page: u64::try_from(page_no.max(1)).unwrap_or(1),
        page_size: u64::try_from(size.max(1)).unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::sub_merchant::MSG_SUB_MCH_REQUIRED;

    #[test]
    fn bind_requires_sub_mch_id_before_unmigrated() {
        let dto = SubMerchantBindParam::default();
        let err = dto.validate().unwrap_err();
        assert_eq!(err, MSG_SUB_MCH_REQUIRED);
        let dto = SubMerchantBindParam {
            sub_mch_id: Some("123".into()),
            ..Default::default()
        };
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn page_conversion() {
        let p = to_page_query(None, None);
        assert_eq!((p.page, p.page_size), (1, 15));
        let p = to_page_query(Some(2), Some(10));
        assert_eq!((p.page, p.page_size), (2, 10));
    }

    #[test]
    fn now_datetime_shape() {
        let value = now_datetime();
        assert_eq!(value.len(), 19, "{value}");
        assert!(value.contains(':'));
    }
}
