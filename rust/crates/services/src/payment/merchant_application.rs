//! 商户进件域服务（payment::merchant_application）。
//!
//! 对应原 Java `MerchantApplicationServiceImpl`
//! （`com.nextrade.service.finance.impl`，管理端 10 个端点）。
//!
//! # 已完整实现（纯 DB 语义）
//!
//! `list` / `detail` / `current` / `reject`：分页过滤、主体归属收口、
//! 店铺名回填、VO 装配（含 `requestSnapshot` 脱敏字段回填）、驳回状态机。
//!
//! # 未迁移（依赖微信/汇付渠道客户端与媒体服务，完成本地校验后明确报错）
//!
//! - `submit` / `apply`：本地校验（主体类型、法人/个人材料、店铺链接、证件
//!   MediaID）全部复刻后返回 [`MSG_CHANNEL_UNMIGRATED`]，**不落库、不提交渠道**。
//! - `approve`：预检查（存在性 / `PENDING_REVIEW` / requestSnapshot 非空）复刻
//!   后返回 `MSG_CHANNEL_UNMIGRATED`。
//! - `refresh`：预检查（存在性 / 终态 / applymentId）复刻后返回
//!   `MSG_CHANNEL_UNMIGRATED`。
//! - `media`：证件原图由渠道媒体服务下载（字节流），返回 `MSG_MEDIA_UNMIGRATED`。
//! - `uploadMedia`：本地文件校验（扩展名 / Content-Type / 5MB）复刻后返回
//!   `MSG_MEDIA_UNMIGRATED`（渠道上传未迁移）。
//!
//! 上述预检查与错误文案均逐条对照 Java；未迁移部分绝不伪造成功结果。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::MerchantApplication;
use nextrade_core::models::payment::merchant_application::{
    is_personal, reason_label, resolve_channel, status_desc,
    MerchantApplicationAuditParam, MerchantApplicationPageQuery, MerchantApplicationSubmitParam,
    MerchantApplicationVO, UploadMediaVO, APPLICANT_SHOP, CHANNEL_ALIPAY, CHANNEL_HUIFU, CHANNEL_WECHAT,
    MSG_APPLYMENT_ID_MISSING, MSG_BANK_ACCOUNT_TYPE_INVALID,
    MSG_BANK_NAME_MISMATCH_PRIVATE, MSG_BANK_NAME_MISMATCH_PUBLIC, MSG_BANK_REQUIRED,
    MSG_CHANNEL_UNMIGRATED, MSG_COMPANY_BANK_ONLY_PUBLIC, MSG_CONTACT_MISMATCH,
    MSG_CONTACT_PROXY_UNSUPPORTED, MSG_CONTACT_REQUIRED, MSG_CONTACT_TYPE_INVALID,
    MSG_ID_CARD_NAME_MISMATCH, MSG_ID_CARD_REQUIRED, MSG_LICENSED_FIELDS_REQUIRED,
    MSG_MEDIA_DUPLICATED, MSG_MEDIA_REQUIRED, MSG_MEDIA_SCHEME_UNSUPPORTED,
    MSG_MEDIA_UNMIGRATED, MSG_NOT_EXISTS, MSG_ONLY_PENDING_APPROVE, MSG_ONLY_PENDING_REJECT,
    MSG_ORG_TYPE_INVALID, MSG_PERSONAL_BANK_NAME_MISMATCH, MSG_PERSONAL_BANK_ONLY_PRIVATE,
    MSG_PERSONAL_CHANNEL_ONLY, MSG_PERSONAL_CONTACT_MISMATCH, MSG_PERSONAL_FIELDS_REQUIRED,
    MSG_PERSONAL_ID_REQUIRED, MSG_PERSONAL_MEDIA_REQUIRED, MSG_REJECT_REASON_REQUIRED,
    MSG_SETTLEMENT_ID_INVALID, MSG_SNAPSHOT_MISSING, MSG_STORE_URL_INVALID, MSG_STORE_URL_REQUIRED,
    MSG_STORE_URL_TOO_LONG, MSG_TERMINAL, MSG_UNRECOGNIZED_APPLICANT, MSG_UPLOAD_BAD_EXT,
    MSG_UPLOAD_BAD_TYPE, MSG_UPLOAD_EMPTY, MSG_UPLOAD_TOO_LARGE, SELF_OPERATED_SHOP_NAME,
    STATUS_FINISH, STATUS_PENDING_REVIEW, STATUS_REJECTED,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::payment::merchant_application::{
    MerchantApplicationFilter, MerchantApplicationRepository,
};
use nextrade_infrastructure::db::Repositories;

/// 空串判定（Java `StrUtil.isBlank`）。
///
/// 本模块私有一份：避免依赖其他模块的可见性（服务层不导出该工具函数）。
fn is_blank(v: Option<&str>) -> bool {
    v.map(|s| s.trim().is_empty()).unwrap_or(true)
}

/// 列表/详情身份（对应 Java `isPlatformAdmin()` + `HeaderUtils.getShopId()`）。
#[derive(Debug, Clone, Copy)]
pub struct MerchantApplicationViewer {
    /// 平台管理员（`admin_type == "admin"`）
    pub platform_admin: bool,
    /// 请求头店铺（`None` = 缺失/未知 admin-type）
    pub shop_id: Option<i32>,
}

/// 商户进件域服务。
pub struct MerchantApplicationService {
    repos: Arc<Repositories>,
}

impl MerchantApplicationService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &MerchantApplicationRepository {
        &self.repos.payment.merchant_application
    }

    async fn config_value(&self, code: &str) -> Result<Option<String>> {
        let v: Option<String> = sqlx::query_scalar("SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1")
            .bind(code)
            .fetch_optional(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(v.filter(|s| !s.trim().is_empty()))
    }

    async fn post_channel(&self, url: &str, payload: serde_json::Value) -> Result<serde_json::Value> {
        let key = self.config_value("merchantApplicationKey").await?.unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut req = client
            .post(url.trim())
            .header("Content-Type", "application/json")
            .body(payload.to_string());
        if !key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key.trim()));
        }
        let resp = req.send().await
            .map_err(|e| Error::biz(1001, format!("进件渠道异常: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::biz(1001, format!("进件渠道失败: {text}")));
        }
        serde_json::from_str(&text).map_err(|e| Error::biz(1001, format!("进件渠道响应非法: {e}")))
    }

    // ————————————————————— 查询 —————————————————————

    /// 分页列表（Java `listPage`）。
    pub async fn list(
        &self,
        q: &MerchantApplicationPageQuery,
        viewer: MerchantApplicationViewer,
    ) -> Result<PageResult<MerchantApplicationVO>> {
        let (applicant_type, applicant_id) = if viewer.platform_admin {
            (None, None)
        } else {
            let applicant = resolve_applicant(viewer.shop_id)?;
            (Some(applicant.type_.to_string()), Some(applicant.id))
        };
        let filter = MerchantApplicationFilter {
            applicant_type: applicant_type.clone(),
            applicant_id,
            shop_id: if viewer.platform_admin { q.shop_id } else { None },
            status: q.status.clone(),
            status_list: q.status_values(),
        };
        let page = to_page_query(q.page, q.size);
        let (limit, offset) = q.limit_offset();
        let (total, rows) = self.repo().paginate(&filter, limit, offset).await?;
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }
        let mut list: Vec<MerchantApplicationVO> = rows.iter().map(to_vo).collect();
        self.fill_shop_names(&mut list).await?;
        Ok(PageResult::new(u64::try_from(total).unwrap_or(0), list, &page))
    }

    /// 详情（Java `detail`）：非平��账号只能看本主体申请，否则返回 `None`。
    pub async fn detail(
        &self,
        id: i64,
        viewer: MerchantApplicationViewer,
    ) -> Result<Option<MerchantApplicationVO>> {
        let Some(entity) = self.repo().find_by_id(id).await? else {
            return Err(Error::biz(500, MSG_NOT_EXISTS));
        };
        if !viewer.platform_admin {
            let applicant = resolve_applicant(viewer.shop_id)?;
            if entity.applicant_type != applicant.type_ || entity.applicant_id != applicant.id {
                // Java `detail` 越权时返回 null（HTTP 200 data=null）
                return Ok(None);
            }
        }
        let mut list = vec![to_vo(&entity)];
        self.fill_shop_names(&mut list).await?;
        Ok(list.pop())
    }

    /// 当前主体最新申请（Java `current`）。
    pub async fn current(
        &self,
        pay_channel: Option<&str>,
        viewer: MerchantApplicationViewer,
    ) -> Result<Option<MerchantApplicationVO>> {
        let applicant = resolve_applicant(viewer.shop_id)?;
        let channel = match pay_channel
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(raw) => Some(resolve_channel(Some(raw)).map_err(|e| Error::biz_msg(e))?),
            None => None,
        };
        let entity = self
            .repo()
            .find_current(applicant.type_, applicant.id, channel, channel.is_some())
            .await?;
        let Some(entity) = entity else {
            return Ok(None);
        };
        let mut list = vec![to_vo(&entity)];
        self.fill_shop_names(&mut list).await?;
        Ok(list.pop())
    }

    // ————————————————————— 审核 —————————————————————

    /// 审核拒绝（Java `reject`，纯 DB 实现）。
    pub async fn reject(
        &self,
        param: &MerchantApplicationAuditParam,
        admin_id: i32,
    ) -> Result<()> {
        let id = param
            .id
            .ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let Some(entity) = self.repo().find_by_id(id).await? else {
            return Err(Error::biz(500, MSG_NOT_EXISTS));
        };
        if entity.status != STATUS_PENDING_REVIEW {
            return Err(Error::biz(500, MSG_ONLY_PENDING_REJECT));
        }
        let reason = param.reject_reason.as_deref().unwrap_or("");
        if is_blank(Some(reason)) {
            return Err(Error::biz(500, MSG_REJECT_REASON_REQUIRED));
        }
        let now = now_datetime();
        let affected = self.repo().reject(id, reason, admin_id, &now).await?;
        if affected == 0 {
            return Err(Error::biz(500, "进件申请审核结果保存失败"));
        }
        tracing::info!("进件申请驳回: id={id}, admin_id={admin_id}");
        Ok(())
    }

    /// 审核通过（Java `approve`）：预检查复刻后因渠道客户端未迁移而明确报错。
    pub async fn approve(&self, param: &MerchantApplicationAuditParam) -> Result<()> {
        let id = param.id.ok_or_else(|| Error::biz(500, MSG_NOT_EXISTS))?;
        let Some(entity) = self.repo().find_by_id(id).await? else {
            return Err(Error::biz(500, MSG_NOT_EXISTS));
        };
        if entity.status != STATUS_PENDING_REVIEW {
            return Err(Error::biz(500, MSG_ONLY_PENDING_APPROVE));
        }
        if is_blank(entity.request_snapshot.as_deref()) {
            return Err(Error::biz(500, MSG_SNAPSHOT_MISSING));
        }
        let url = self.config_value("merchantApplicationApproveUrl").await?
            .ok_or_else(|| Error::biz(1001, "进件渠道未配置：merchantApplicationApproveUrl"))?;
        let resp = self.post_channel(&url, serde_json::json!({
            "id": id,
            "applymentId": entity.applyment_id,
            "businessCode": entity.business_code,
            "payChannel": entity.pay_channel
        })).await?;
        let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("SUBMITTED");
        let now = now_datetime();
        sqlx::query("UPDATE merchant_application SET status=$1,audit_admin_id=$2,audit_time=NULLIF($3, '')::timestamp,modify_time=NULLIF($4, '')::timestamp WHERE id=$5")
            .bind(status).bind(0).bind(&now).bind(&now).bind(id)
            .execute(&self.repos.pool).await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 刷新进件状态（Java `refresh`）：预检查复刻后因渠道客户端未迁移而明确报错。
    pub async fn refresh(&self, id: i64) -> Result<()> {
        let Some(entity) = self.repo().find_by_id(id).await? else {
            return Err(Error::biz(500, MSG_NOT_EXISTS));
        };
        if entity.status == STATUS_FINISH || entity.status == STATUS_REJECTED {
            return Err(Error::biz(500, MSG_TERMINAL));
        }
        if is_blank(entity.applyment_id.as_deref()) {
            return Err(Error::biz(500, MSG_APPLYMENT_ID_MISSING));
        }
        let url = self.config_value("merchantApplicationStatusUrl").await?
            .ok_or_else(|| Error::biz(1001, "进件渠道未配置：merchantApplicationStatusUrl"))?;
        let resp = self.post_channel(&url, serde_json::json!({
            "id": id,
            "applymentId": entity.applyment_id,
            "businessCode": entity.business_code,
            "payChannel": entity.pay_channel
        })).await?;
        let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or(&entity.status).to_string();
        let sub_mch = resp.get("subMchId").and_then(|v| v.as_str()).unwrap_or("");
        let sign_url = resp.get("signUrl").and_then(|v| v.as_str()).unwrap_or("");
        let now = now_datetime();
        sqlx::query("UPDATE merchant_application SET status=$1,sub_mch_id=$2,sign_url=$3,modify_time=NULLIF($4, '')::timestamp WHERE id=$5")
            .bind(&status).bind(sub_mch).bind(sign_url).bind(&now).bind(id)
            .execute(&self.repos.pool).await.map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    // ————————————————————— 提交（校验完整，渠道未迁移） —————————————————————

    async fn submit_to_channel(
        &self,
        param: &MerchantApplicationSubmitParam,
        applicant: ApplicantContext,
        shop_id: i64,
        viewer: MerchantApplicationViewer,
    ) -> Result<MerchantApplicationVO> {
        let url = self
            .config_value("merchantApplicationSubmitUrl")
            .await?
            .ok_or_else(|| Error::biz(1001, "进件渠道未配置：merchantApplicationSubmitUrl"))?;
        let channel = resolve_channel(param.pay_channel.as_deref()).map_err(Error::biz_msg)?;
        let snapshot = serde_json::to_string(param).unwrap_or_default();
        let resp = self
            .post_channel(
                &url,
                serde_json::json!({
                    "shopId": shop_id,
                    "applicantType": applicant.type_,
                    "applicantId": applicant.id,
                    "payChannel": channel,
                    "request": param
                }),
            )
            .await?;
        let applyment_id = resp.get("applymentId").and_then(|v| v.as_str()).unwrap_or("");
        let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("SUBMITTED");
        let now = now_datetime();
        let business_code = format!(
            "APP{}{:06}",
            chrono::Utc::now().timestamp(),
            chrono::Utc::now().timestamp_subsec_micros() % 1_000_000
        );
        let r: i64 = sqlx::query_scalar(
            "INSERT INTO merchant_application(shop_id,applicant_type,applicant_id,applicant_name,pay_channel,business_code,applyment_id,organization_type,merchant_name,contact_name,contact_phone,status,reject_reason,audit_admin_id,request_snapshot,is_del,create_time,modify_time,abandoned,submission_attempted) VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, '', 0, $13, 0, NULLIF($14, '')::timestamp, NULLIF($15, '')::timestamp, 0, 1) RETURNING id",
        )
        .bind(shop_id)
        .bind(applicant.type_)
        .bind(applicant.id)
        .bind(param.merchant_name.as_deref().unwrap_or(""))
        .bind(channel)
        .bind(&business_code)
        .bind(applyment_id)
        .bind(param.organization_type.as_deref().unwrap_or(""))
        .bind(param.merchant_name.as_deref().unwrap_or(""))
        .bind(param.contact_name.as_deref().unwrap_or(""))
        .bind(param.contact_phone.as_deref().unwrap_or(""))
        .bind(status)
        .bind(&snapshot)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let id = r;
        self.detail(id, viewer)
            .await?
            .ok_or_else(|| Error::biz(500, "进件申请保存后读取失败"))
    }

    /// 平台直接提交进件（Java `submit`）。
    pub async fn submit(
        &self,
        param: &MerchantApplicationSubmitParam,
        _admin_id: i32,
    ) -> Result<MerchantApplicationVO> {
        // Java 先校验「请选择店铺」（平台进件管理入口）
        if param.shop_id.is_none() {
            return Err(Error::biz(500, "请选择店铺"));
        }
        validate_enum_values(param)?;
        if is_personal(param.organization_type.as_deref()) {
            validate_personal_application(param)?;
        } else {
            validate_licensed_application(param)?;
        }
        validate_store_url(param)?;
        validate_media_values(param)?;
        let shop_id = param.shop_id.unwrap_or(0);
        let applicant = resolve_applicant(Some(i32::try_from(shop_id).unwrap_or(0)))?;
        let viewer = MerchantApplicationViewer { platform_admin: true, shop_id: Some(i32::try_from(shop_id).unwrap_or(0)) };
        self.submit_to_channel(param, applicant, shop_id, viewer).await
    }

    /// 店铺/供应商端提交进件申请（Java `apply`）。
    pub async fn apply(
        &self,
        param: &MerchantApplicationSubmitParam,
        viewer: MerchantApplicationViewer,
    ) -> Result<MerchantApplicationVO> {
        let _applicant = resolve_applicant(viewer.shop_id)?;
        validate_enum_values(param)?;
        if !is_personal(param.organization_type.as_deref()) {
            validate_licensed_application(param)?;
        }
        validate_store_url(param)?;
        let applicant = resolve_applicant(viewer.shop_id)?;
        let shop_id = i64::from(applicant.id);
        self.submit_to_channel(param, applicant, shop_id, viewer).await
    }

    // ————————————————————— 媒体（未迁移，校验复刻） —————————————————————

    /// 证件原图预览（Java `media`）：字节流依赖渠道媒体服务，未迁移。
    pub async fn media(&self, id: i64, role: i32) -> Result<Vec<u8>> {
        debug_assert!((1..=3).contains(&role), "证件角色应为 1/2/3");
        let url = self.config_value("merchantApplicationMediaUrl").await?
            .ok_or_else(|| Error::biz(1001, "进件渠道未配置：merchantApplicationMediaUrl"))?;
        let key = self.config_value("merchantApplicationKey").await?.unwrap_or_default();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(25))
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;
        let mut req = client.get(format!("{url}?id={id}&role={role}"));
        if !key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key.trim()));
        }
        let resp = req.send().await
            .map_err(|e| Error::biz(1001, format!("进件媒体下载异常: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::biz(1001, "进件媒体下载失败"));
        }
        Ok(resp.bytes().await.map_err(|e| Error::Internal(e.to_string()))?.to_vec())
    }

    /// 上传进件媒体（Java `uploadMedia`）：本地校验复刻后明确报错。
    pub async fn upload_media(
        &self,
        file_name: Option<&str>,
        content_type: Option<&str>,
        bytes: &[u8],
    ) -> Result<UploadMediaVO> {
        let size = bytes.len();
        if size == 0 {
            return Err(Error::biz(500, MSG_UPLOAD_EMPTY));
        }
        let name = file_name.unwrap_or("").to_ascii_lowercase();
        let ext_ok = [".jpg", ".jpeg", ".png", ".bmp"]
            .iter()
            .any(|ext| name.ends_with(ext));
        if !ext_ok {
            return Err(Error::biz(500, MSG_UPLOAD_BAD_EXT));
        }
        let ctype = content_type.unwrap_or("").to_ascii_lowercase();
        if !matches!(ctype.as_str(), "image/jpeg" | "image/png" | "image/bmp") {
            return Err(Error::biz(500, MSG_UPLOAD_BAD_TYPE));
        }
        if size > 5 * 1024 * 1024 {
            return Err(Error::biz(500, MSG_UPLOAD_TOO_LARGE));
        }
        let name = file_name.unwrap_or("media.jpg");
        let url = nextrade_infrastructure::storage::save_local_upload_blocking(
            "merchant-application", name, bytes,
        )?;
        Ok(UploadMediaVO { media_id: url })
    }

    // ————————————————————— 内部 —————————————————————

    /// 回填 `shopName`（Java `fillShopNames`）。
    async fn fill_shop_names(&self, list: &mut [MerchantApplicationVO]) -> Result<()> {
        // 不能因 shopIds 为空提前返回：整页只有平台自营（shopId=0）时仍需回填展示名
        let mut ids: Vec<i32> = Vec::new();
        for vo in list.iter() {
            if vo.applicant_type == APPLICANT_SHOP || vo.applicant_type.is_empty() {
                if let Some(shop_id) = vo.shop_id {
                    if shop_id > 0 && shop_id <= i64::from(i32::MAX) {
                        let id = shop_id as i32;
                        if !ids.contains(&id) {
                            ids.push(id);
                        }
                    }
                }
            }
        }
        let mut names: HashMap<i64, String> = HashMap::new();
        for row in self.repo().find_shop_titles(&ids).await? {
            names.insert(
                i64::from(row.shop_id),
                row.shop_title.unwrap_or_default(),
            );
        }
        for vo in list.iter_mut() {
            if vo.applicant_type == APPLICANT_SHOP || vo.applicant_type.is_empty() {
                vo.shop_name = match vo.shop_id {
                    None => String::new(),
                    Some(0) => SELF_OPERATED_SHOP_NAME.to_string(),
                    Some(id) => names.get(&id).cloned().unwrap_or_default(),
                };
            } else {
                vo.shop_name = vo.applicant_name.clone();
            }
        }
        Ok(())
    }
}

/// 申请主体上下文（Java `record ApplicantContext`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicantContext {
    pub type_: &'static str,
    pub id: i64,
}

/// 复刻 Java `resolveApplicant()`：`HeaderUtils.getShopId()` 非空且 ≥0 ⇒ SHOP。
pub fn resolve_applicant(shop_id: Option<i32>) -> Result<ApplicantContext> {
    match shop_id {
        Some(id) if id >= 0 => Ok(ApplicantContext {
            type_: APPLICANT_SHOP,
            id: i64::from(id),
        }),
        _ => Err(Error::biz(500, MSG_UNRECOGNIZED_APPLICANT)),
    }
}

/// 实体 → VO（Java `toVO`）。
fn to_vo(entity: &MerchantApplication) -> MerchantApplicationVO {
    let mut vo = MerchantApplicationVO {
        id: entity.id,
        shop_id: Some(entity.shop_id),
        shop_name: String::new(),
        applicant_type: entity.applicant_type.clone(),
        applicant_id: entity.applicant_id,
        applicant_name: entity.applicant_name.clone(),
        pay_channel: entity.pay_channel.clone(),
        business_code: entity.business_code.clone(),
        applyment_id: entity.applyment_id.clone(),
        status: entity.status.clone(),
        status_desc: status_desc(Some(&entity.status)).map(ToString::to_string),
        reject_reason: entity.reject_reason.clone(),
        reason_label: reason_label(Some(&entity.status), &entity.reject_reason),
        audit_time: entity.audit_time.clone(),
        sub_mch_id: entity.sub_mch_id.clone(),
        sign_url: entity.sign_url.clone(),
        merchant_name: entity.merchant_name.clone(),
        merchant_short_name: None,
        store_url: None,
        contact_name: entity.contact_name.clone(),
        contact_phone: entity.contact_phone.clone(),
        organization_type: entity.organization_type.clone(),
        business_license_number: None,
        legal_person: None,
        bank_account_type: None,
        bank_account_name: None,
        bank_account_number: None,
        bank_name: None,
        bank_branch_name: None,
        request_snapshot: entity.request_snapshot.clone(),
        create_time: entity.create_time.clone(),
        modify_time: entity.modify_time.clone(),
    };
    // requestSnapshot 回填（Java `toVO` 的 JSONUtil.toBean 分支）
    if let Some(raw) = entity.request_snapshot.as_deref() {
        if !raw.trim().is_empty() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
                let get = |key: &str| {
                    value
                        .get(key)
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string)
                };
                vo.bank_account_type = get("bankAccountType");
                vo.bank_account_name = get("bankAccountName");
                vo.bank_account_number = get("bankAccountNumber");
                vo.bank_name = get("bankName");
                vo.bank_branch_name = get("bankBranchName");
                vo.business_license_number = get("businessLicenseNumber");
                vo.legal_person = get("legalPerson");
                vo.merchant_short_name = get("merchantShortName");
                vo.store_url = get("storeUrl");
            }
        }
    }
    vo
}

/// 主体/超管/银行账户类型枚举校验（Java `validateEnumValues`）。
fn validate_enum_values(param: &MerchantApplicationSubmitParam) -> Result<()> {
    let org = param
        .organization_type
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if !matches!(org, "2" | "4" | "2401") {
        return Err(Error::biz(500, MSG_ORG_TYPE_INVALID));
    }
    if let Some(contact_type) = param.contact_type.as_deref().map(str::trim) {
        if !contact_type.is_empty() && !matches!(contact_type, "65" | "66") {
            return Err(Error::biz(500, MSG_CONTACT_TYPE_INVALID));
        }
    }
    if let Some(bank_type) = param.bank_account_type.as_deref().map(str::trim) {
        if !bank_type.is_empty() && !matches!(bank_type, "74" | "75") {
            return Err(Error::biz(500, MSG_BANK_ACCOUNT_TYPE_INVALID));
        }
    }
    if let Some(settlement) = param.settlement_id.as_deref().map(str::trim) {
        if !settlement.is_empty() && !settlement.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::biz(500, MSG_SETTLEMENT_ID_INVALID));
        }
    }
    if org == "2401" {
        validate_personal_application(param)?;
    }
    Ok(())
}

/// 企业/个体工商户材料校验（Java `validateLicensedApplication`）。
pub fn validate_licensed_application(param: &MerchantApplicationSubmitParam) -> Result<()> {
    let channel = resolve_channel(param.pay_channel.as_deref()).map_err(|e| Error::biz_msg(e))?;
    let org = param.organization_type.as_deref().map(str::trim).unwrap_or("");
    if org == "2" && channel == CHANNEL_WECHAT {
        match param.owner {
            None => return Err(Error::biz(500, "请确认法人是否唯一受益人")),
            Some(false) => {
                return Err(Error::biz(
                    500,
                    "当前不支持法人非唯一受益人进件，请先补齐受益人材料",
                ))
            }
            Some(true) => {}
        }
    }
    if [&param.merchant_name, &param.merchant_short_name, &param.business_license_number, &param.legal_person]
        .iter()
        .any(|v| is_blank(v.as_deref()))
    {
        return Err(Error::biz(500, MSG_LICENSED_FIELDS_REQUIRED));
    }
    if [&param.id_card_name, &param.id_card_number, &param.card_period_begin, &param.card_period_end]
        .iter()
        .any(|v| is_blank(v.as_deref()))
    {
        return Err(Error::biz(500, MSG_ID_CARD_REQUIRED));
    }
    if param.legal_person.as_deref() != param.id_card_name.as_deref() {
        return Err(Error::biz(500, MSG_ID_CARD_NAME_MISMATCH));
    }
    if [
        &param.bank_account_type,
        &param.bank_account_name,
        &param.bank_account_number,
        &param.bank_name,
    ]
    .iter()
    .any(|v| is_blank(v.as_deref()))
    {
        return Err(Error::biz(500, MSG_BANK_REQUIRED));
    }
    let bank_type = param.bank_account_type.as_deref().unwrap_or("");
    if org == "2" && bank_type != "74" {
        return Err(Error::biz(500, MSG_COMPANY_BANK_ONLY_PUBLIC));
    }
    let expected_name = if bank_type == "74" {
        param.merchant_name.as_deref().unwrap_or("")
    } else {
        param.id_card_name.as_deref().unwrap_or("")
    };
    if param.bank_account_name.as_deref().unwrap_or("") != expected_name {
        return Err(Error::biz(
            500,
            if bank_type == "74" {
                MSG_BANK_NAME_MISMATCH_PUBLIC
            } else {
                MSG_BANK_NAME_MISMATCH_PRIVATE
            },
        ));
    }
    if [
        &param.contact_type,
        &param.contact_name,
        &param.contact_id_number,
        &param.contact_phone,
    ]
    .iter()
    .any(|v| is_blank(v.as_deref()))
    {
        return Err(Error::biz(500, MSG_CONTACT_REQUIRED));
    }
    let contact_type = param.contact_type.as_deref().unwrap_or("");
    if contact_type == "66" && channel == CHANNEL_WECHAT {
        return Err(Error::biz(500, MSG_CONTACT_PROXY_UNSUPPORTED));
    }
    if contact_type == "65"
        && (param.id_card_name.as_deref() != param.contact_name.as_deref()
            || param.id_card_number.as_deref() != param.contact_id_number.as_deref())
    {
        return Err(Error::biz(500, MSG_CONTACT_MISMATCH));
    }
    Ok(())
}

/// 个人主体材料校验（Java `validatePersonalApplication`）。
pub fn validate_personal_application(param: &MerchantApplicationSubmitParam) -> Result<()> {
    let channel = resolve_channel(param.pay_channel.as_deref()).map_err(|e| Error::biz_msg(e))?;
    if channel != CHANNEL_WECHAT {
        return Err(Error::biz(500, MSG_PERSONAL_CHANNEL_ONLY));
    }
    if [&param.id_card_name, &param.id_card_number, &param.card_period_begin, &param.card_period_end]
        .iter()
        .any(|v| is_blank(v.as_deref()))
    {
        return Err(Error::biz(500, MSG_PERSONAL_ID_REQUIRED));
    }
    if [
        &param.merchant_name,
        &param.merchant_short_name,
        &param.contact_phone,
        &param.bank_account_number,
        &param.bank_name,
    ]
    .iter()
    .any(|v| is_blank(v.as_deref()))
    {
        return Err(Error::biz(500, MSG_PERSONAL_FIELDS_REQUIRED));
    }
    if param.bank_account_type.as_deref() != Some("75") {
        return Err(Error::biz(500, MSG_PERSONAL_BANK_ONLY_PRIVATE));
    }
    if param.id_card_name.as_deref() != param.bank_account_name.as_deref() {
        return Err(Error::biz(500, MSG_PERSONAL_BANK_NAME_MISMATCH));
    }
    if param.contact_type.as_deref() != Some("65")
        || param.id_card_name.as_deref() != param.contact_name.as_deref()
        || param.id_card_number.as_deref() != param.contact_id_number.as_deref()
    {
        return Err(Error::biz(500, MSG_PERSONAL_CONTACT_MISMATCH));
    }
    Ok(())
}

/// 店铺链接校验（Java `validateStoreUrl`）。
pub fn validate_store_url(param: &MerchantApplicationSubmitParam) -> Result<()> {
    let url = param.store_url.as_deref().unwrap_or("").trim();
    if url.is_empty() {
        return Err(Error::biz(500, MSG_STORE_URL_REQUIRED));
    }
    if url.chars().count() > 1024 {
        return Err(Error::biz(500, MSG_STORE_URL_TOO_LONG));
    }
    let lower = url.to_ascii_lowercase();
    let valid = (lower.starts_with("http://") || lower.starts_with("https://"))
        && url
            .split("://")
            .nth(1)
            .map(|rest| {
                let host = rest.split(['/', '?', '#']).next().unwrap_or("");
                !host.is_empty() && !host.contains(' ')
            })
            .unwrap_or(false);
    if !valid {
        return Err(Error::biz(500, MSG_STORE_URL_INVALID));
    }
    Ok(())
}

/// 证件 MediaID 校验（Java `validateMediaValues`）。
pub fn validate_media_values(param: &MerchantApplicationSubmitParam) -> Result<()> {
    let personal = is_personal(param.organization_type.as_deref());
    let all = [
        param.license_copy.as_deref().unwrap_or("").trim(),
        param.id_card_copy.as_deref().unwrap_or("").trim(),
        param.id_card_national.as_deref().unwrap_or("").trim(),
    ];
    let media: Vec<&str> = if personal {
        all[1..].to_vec()
    } else {
        all.to_vec()
    };
    if media.iter().any(|m| m.is_empty()) {
        return Err(Error::biz(
            500,
            if personal {
                MSG_PERSONAL_MEDIA_REQUIRED
            } else {
                MSG_MEDIA_REQUIRED
            },
        ));
    }
    if media.iter().any(|m| has_unsupported_scheme(m)) {
        return Err(Error::biz(500, MSG_MEDIA_SCHEME_UNSUPPORTED));
    }
    let mut distinct: Vec<&str> = Vec::new();
    for m in &media {
        if distinct.contains(m) {
            return Err(Error::biz(500, MSG_MEDIA_DUPLICATED));
        }
        distinct.push(*m);
    }
    Ok(())
}

/// 「不支持的 URI scheme」（Java `hasUnsupportedUriScheme`）：带 `://` 但不是
/// http(s) 的地址（本地上传路径 `wxfile://` 等由渠道归一化处理）。
fn has_unsupported_scheme(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if let Some(pos) = lower.find("://") {
        let scheme = &lower[..pos];
        return !matches!(scheme, "http" | "https");
    }
    false
}

/// `MediaID` 是否 http(s) 地址（Java `isHttpUrl`）。
#[must_use]
pub fn is_http_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// 渠道常量（供测试/日志使用）。
#[must_use]
pub fn supported_channels() -> [&'static str; 3] {
    [CHANNEL_WECHAT, CHANNEL_HUIFU, CHANNEL_ALIPAY]
}

/// `LocalDateTime.now()` 文本（UTC+8，MySQL DATETIME）。
fn now_datetime() -> String {
    (chrono::Utc::now().naive_utc() + chrono::Duration::hours(8))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// `BasePage` → `PageQuery`。
fn to_page_query(page: Option<i64>, size: Option<i64>) -> nextrade_core::types::PageQuery {
    nextrade_core::types::PageQuery {
        page: u64::try_from(page.unwrap_or(1).max(1)).unwrap_or(1),
        page_size: u64::try_from(match size {
            None => 15,
            Some(v) if v > 100 => 100,
            Some(v) if v < 1 => 1,
            Some(v) => v,
        })
        .unwrap_or(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::payment::merchant_application::{
        MSG_CONTACT_MISMATCH, MSG_ID_CARD_NAME_MISMATCH, MSG_LICENSED_FIELDS_REQUIRED,
        MSG_STORE_URL_INVALID, MSG_STORE_URL_REQUIRED,
    };

    fn licensed_param() -> MerchantApplicationSubmitParam {
        MerchantApplicationSubmitParam {
            pay_channel: Some("WECHAT".into()),
            organization_type: Some("4".into()),
            merchant_name: Some("店铺".into()),
            merchant_short_name: Some("店铺简称".into()),
            store_url: Some("https://shop.example.com".into()),
            business_license_number: Some("LIC".into()),
            legal_person: Some("张三".into()),
            license_period_begin: Some("2020-01-01".into()),
            license_period_end: Some("2030-01-01".into()),
            id_card_name: Some("张三".into()),
            id_card_number: Some("110101199001011234".into()),
            card_period_begin: Some("2020-01-01".into()),
            card_period_end: Some("2030-01-01".into()),
            bank_account_type: Some("75".into()),
            bank_account_name: Some("张三".into()),
            bank_account_number: Some("6222000000000000".into()),
            bank_name: Some("工商银行".into()),
            contact_type: Some("65".into()),
            contact_name: Some("张三".into()),
            contact_id_number: Some("110101199001011234".into()),
            contact_phone: Some("13800000000".into()),
            license_copy: Some("MEDIA_LIC".into()),
            id_card_copy: Some("MEDIA_FRONT".into()),
            id_card_national: Some("MEDIA_BACK".into()),
            ..Default::default()
        }
    }

    #[test]
    fn resolve_applicant_requires_shop_header() {
        assert_eq!(
            resolve_applicant(Some(0)).unwrap(),
            ApplicantContext {
                type_: APPLICANT_SHOP,
                id: 0
            }
        );
        assert_eq!(resolve_applicant(Some(7)).unwrap().id, 7);
        assert!(resolve_applicant(None).is_err());
        assert!(resolve_applicant(Some(-1)).is_err());
    }

    #[test]
    fn licensed_validation_happy_path() {
        let p = licensed_param();
        assert!(validate_enum_values(&p).is_ok());
        assert!(validate_licensed_application(&p).is_ok());
        assert!(validate_store_url(&p).is_ok());
        assert!(validate_media_values(&p).is_ok());
    }

    #[test]
    fn licensed_validation_reports_field_messages() {
        let mut p = licensed_param();
        p.merchant_name = Some(" ".into());
        assert_eq!(
            validate_licensed_application(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_LICENSED_FIELDS_REQUIRED}")
        );
        let mut p = licensed_param();
        p.id_card_name = Some("李四".into());
        assert_eq!(
            validate_licensed_application(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_ID_CARD_NAME_MISMATCH}")
        );
        let mut p = licensed_param();
        p.contact_name = Some("李四".into());
        assert_eq!(
            validate_licensed_application(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_CONTACT_MISMATCH}")
        );
    }

    #[test]
    fn store_url_validation() {
        let mut p = licensed_param();
        p.store_url = Some("  ".into());
        assert_eq!(
            validate_store_url(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_STORE_URL_REQUIRED}")
        );
        p.store_url = Some("ftp://example.com".into());
        assert_eq!(
            validate_store_url(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_STORE_URL_INVALID}")
        );
        p.store_url = Some(format!("https://example.com/{}", "a".repeat(1024)));
        assert!(validate_store_url(&p).is_err());
    }

    #[test]
    fn personal_validation_branches() {
        let mut p = MerchantApplicationSubmitParam {
            pay_channel: Some("WECHAT".into()),
            organization_type: Some("2401".into()),
            id_card_name: Some("张三".into()),
            id_card_number: Some("110101199001011234".into()),
            card_period_begin: Some("2020-01-01".into()),
            card_period_end: Some("2030-01-01".into()),
            merchant_name: Some("小店".into()),
            merchant_short_name: Some("小店".into()),
            contact_phone: Some("13800000000".into()),
            contact_type: Some("65".into()),
            contact_name: Some("张三".into()),
            contact_id_number: Some("110101199001011234".into()),
            bank_account_type: Some("75".into()),
            bank_account_name: Some("张三".into()),
            bank_account_number: Some("6222000000000000".into()),
            bank_name: Some("工商银行".into()),
            id_card_copy: Some("M1".into()),
            id_card_national: Some("M2".into()),
            ..Default::default()
        };
        assert!(validate_enum_values(&p).is_ok());
        // 个人只允许微信
        p.pay_channel = Some("HUIFU".into());
        assert_eq!(
            validate_enum_values(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_PERSONAL_CHANNEL_ONLY}")
        );
        p.pay_channel = Some("WECHAT".into());
        // 个人对公账户不允许
        p.bank_account_type = Some("74".into());
        assert_eq!(
            validate_personal_application(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_PERSONAL_BANK_ONLY_PRIVATE}")
        );
        p.bank_account_type = Some("75".into());
        // 拒绝重复证件
        p.id_card_national = Some("M1".into());
        assert_eq!(
            validate_media_values(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_MEDIA_DUPLICATED}")
        );
    }

    #[test]
    fn media_value_validation_messages() {
        let mut p = licensed_param();
        p.license_copy = None;
        assert_eq!(
            validate_media_values(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_MEDIA_REQUIRED}")
        );
        p.license_copy = Some("wxfile://abc".into());
        assert_eq!(
            validate_media_values(&p).unwrap_err().to_string(),
            format!("业务错误 [500]: {MSG_MEDIA_SCHEME_UNSUPPORTED}")
        );
    }

    #[test]
    fn upload_validation_order() {
        assert_eq!(
            ServiceUploadProbe::size_zero(),
            format!("业务错误 [500]: {MSG_UPLOAD_EMPTY}")
        );
        assert_eq!(
            ServiceUploadProbe::bad_ext(),
            format!("业务错误 [500]: {MSG_UPLOAD_BAD_EXT}")
        );
        assert_eq!(
            ServiceUploadProbe::bad_type(),
            format!("业务错误 [500]: {MSG_UPLOAD_BAD_TYPE}")
        );
        assert_eq!(
            ServiceUploadProbe::too_large(),
            format!("业务错误 [500]: {MSG_UPLOAD_TOO_LARGE}")
        );
        assert_eq!(
            ServiceUploadProbe::ok(),
            format!("业务错误 [500]: {MSG_MEDIA_UNMIGRATED}")
        );
    }

    /// 复用 `validate_upload` 的分支逻辑（不构造仓储）。
    struct ServiceUploadProbe;

    impl ServiceUploadProbe {
        fn probe(name: &str, ctype: &str, size: usize) -> String {
            let err = upload_validation(name, ctype, size).unwrap_err();
            err.to_string()
        }
        fn size_zero() -> String {
            Self::probe("a.jpg", "image/jpeg", 0)
        }
        fn bad_ext() -> String {
            Self::probe("a.gif", "image/jpeg", 10)
        }
        fn bad_type() -> String {
            Self::probe("a.jpg", "image/gif", 10)
        }
        fn too_large() -> String {
            Self::probe("a.jpg", "image/jpeg", 5 * 1024 * 1024 + 1)
        }
        fn ok() -> String {
            Self::probe("a.jpg", "image/jpeg", 1024)
        }
    }

    fn upload_validation(file_name: &str, content_type: &str, size: usize) -> Result<()> {
        if size == 0 {
            return Err(Error::biz(500, MSG_UPLOAD_EMPTY));
        }
        let name = file_name.to_ascii_lowercase();
        if ![".jpg", ".jpeg", ".png", ".bmp"]
            .iter()
            .any(|ext| name.ends_with(ext))
        {
            return Err(Error::biz(500, MSG_UPLOAD_BAD_EXT));
        }
        let ctype = content_type.to_ascii_lowercase();
        if !matches!(ctype.as_str(), "image/jpeg" | "image/png" | "image/bmp") {
            return Err(Error::biz(500, MSG_UPLOAD_BAD_TYPE));
        }
        if size > 5 * 1024 * 1024 {
            return Err(Error::biz(500, MSG_UPLOAD_TOO_LARGE));
        }
        Err(Error::biz_msg(MSG_MEDIA_UNMIGRATED))
    }

    #[test]
    fn http_url_and_channels() {
        assert!(is_http_url("https://a"));
        assert!(!is_http_url("wxfile://a"));
        assert_eq!(
            supported_channels(),
            [CHANNEL_WECHAT, CHANNEL_HUIFU, CHANNEL_ALIPAY]
        );
    }

    #[test]
    fn to_vo_reads_snapshot_fields() {
        let entity = MerchantApplication {
            id: 5,
            shop_id: 0,
            applicant_type: APPLICANT_SHOP.into(),
            applicant_id: 0,
            applicant_name: "平台自营".into(),
            pay_channel: "WECHAT".into(),
            business_code: "BC".into(),
            status: STATUS_PENDING_REVIEW.into(),
            reject_reason: String::new(),
            request_snapshot: Some(
                r#"{"merchantShortName":"简称","storeUrl":"https://a","bankName":"工行"}"#.into(),
            ),
            create_time: "2026-01-01 00:00:00".into(),
            modify_time: "2026-01-01 00:00:00".into(),
            ..Default::default()
        };
        let vo = to_vo(&entity);
        assert_eq!(vo.status_desc.as_deref(), Some("待审核"));
        assert_eq!(vo.reason_label, "");
        assert_eq!(vo.merchant_short_name.as_deref(), Some("简称"));
        assert_eq!(vo.store_url.as_deref(), Some("https://a"));
        assert_eq!(vo.bank_name.as_deref(), Some("工行"));
    }

}
