//! 入驻申请域服务（merchant::merchant_apply）。
//!
//! 构造签名固定 `pub fn new(repos: Arc<Repositories>) -> Self`；
//! 仓储访问为**嵌套**命名：`self.repos.merchant.merchant_apply.<method>()`。
//!
//! 对应原 Java `com.nextrade.service.merchant.impl.MerchantApplyServiceImpl`。
//!
//! # 已实现
//!
//! - `list`：分页 + 状态过滤 + 关键字（companyName/contactName LIKE）+ 默认主键降序；
//!   批量补会员展示信息与审核管理员名称、状态文案、时间格式化（与 Java 一致）。
//! - `detail`：`getById` → 不存在「商户申请信息不存在」；JSON 申请资料反序列化返回。
//! - `update`：Java `MerchantApplyUpdateDTO` 是空类 → 本实现同样无操作成功。
//! - `del` / `batch`（`type=del`）/ `updateField`（白名单 `GENERAL_FIELDS`）：
//!   `BaseServiceImpl` 语义，字段先映射列名再比对白名单。
//! - `audit`：仅允许「待审核(1)」→ 通过(10)/不通过(20)；写状态、审核备注、审核时间、
//!   审核管理员；**通过时创建 `merchant` 行**（Java `merchantService.save`）。
//! - `config`：状态参数列表（待审核/审核通过/审核未通过）。
//!
//! # 未迁移（审核通过后的完整开店链路，见 Java `MerchantApplyServiceImpl#audit`）
//!
//! Java 审核通过后还会：创建后台管理员（`adminUserService.providerCreate`，初始密码
//! `PASSWORD`、角色 `ROLE_ID`）、创建 `merchant_user` 关联、创建店铺 `shop` 与店铺
//! 管理员/员工关联、初始化保证金账户与规则、发送站内信/短信通知。
//! 这些跨 `identity` / `merchant` / `deposit` 多域，本实现**未迁移**，仅落地
//! `merchant_apply` 状态与 `merchant` 主记录；调用方不应把本接口视为「开店完成」。
//! 审核不通过的通知（`messageService.sendMerchantApplyResultMessage`）同样未迁移。

use std::sync::Arc;

use serde_json::Value;
use nextrade_common::utils::now_seconds;
use nextrade_core::models::merchant::merchant_apply::{
    apply_field_to_column, format_time, is_general_field, status_options, status_text,
    valid_audit_status, MerchantApplyAuditParam, MerchantApplyListQuery, MerchantApplyUpdateDTO,
    MerchantApplyVO, AUDIT_STATUS_INVALID, MERCHANT_APPLY_NOT_EXIST, MOBILE_NOT_BOUND,
    RESOURCE_NOT_FOUND, SERVICE_DATA_ERROR, STATUS_PASS, STATUS_PENDING,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::merchant::merchant_apply::MerchantApplyRepository;
use nextrade_infrastructure::db::Repositories;

/// 入驻申请域服务。
pub struct MerchantApplyService {
    repos: Arc<Repositories>,
}

impl MerchantApplyService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本特性仓储（嵌套访问）。
    #[must_use]
    pub fn repo(&self) -> &MerchantApplyRepository {
        &self.repos.merchant.merchant_apply
    }

    /// 申请分页（Java `MerchantApplyServiceImpl#list`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn list(&self, q: &MerchantApplyListQuery) -> Result<PageResult<MerchantApplyVO>> {
        let (total, rows) = self.repo().paginate(q).await?;
        let page = nextrade_core::types::PageQuery {
            page: q.page_params().0,
            page_size: q.page_params().1,
        };
        if rows.is_empty() {
            return Ok(PageResult::empty(&page));
        }

        let user_ids: Vec<i32> = rows.iter().filter_map(|r| r.user_id).collect();
        let users = self.repo().find_users(&user_ids).await?;
        let user_map: std::collections::HashMap<i32, (String, String, String)> = users
            .into_iter()
            .map(|(id, username, nickname, mobile, avatar)| {
                let user_json = serde_json::json!({
                    "userId": id,
                    "username": username,
                    "nickname": nickname,
                    "mobile": mobile,
                    "avatar": avatar,
                })
                .to_string();
                (
                    id,
                    (
                        username,
                        nickname.unwrap_or_default(),
                        user_json,
                    ),
                )
            })
            .collect();

        let admin_ids: Vec<i32> = rows.iter().filter_map(|r| r.audit_admin_id).collect();
        let admin_map: std::collections::HashMap<i32, String> = self
            .repo()
            .admin_usernames(&admin_ids)
            .await?
            .into_iter()
            .collect();

        let mut list = Vec::with_capacity(rows.len());
        for (idx, row) in rows.iter().enumerate() {
            let user = user_map.get(&row.user_id.unwrap_or(0)).and_then(|u| {
                serde_json::from_str::<Value>(&u.2).ok()
            });
            let audit_admin_id = row.audit_admin_id.unwrap_or(0);
            let mut vo = to_vo(row);
            vo.user = user;
            vo.audit_admin_name = admin_map
                .get(&audit_admin_id)
                .cloned()
                .unwrap_or_default();
            // 防御：保留索引便于调试
            let _ = idx;
            list.push(vo);
        }
        Ok(PageResult::new(total, list, &page))
    }

    /// 申请详情（Java `#detail`）。
    ///
    /// # Errors
    /// 不存在 → 「商户申请信息不存在」；数据库失败 → `Error::Database`。
    pub async fn detail(&self, id: i32) -> Result<MerchantApplyVO> {
        let row = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MERCHANT_APPLY_NOT_EXIST))?;
        let mut vo = to_vo(&row);
        if let Some(user_id) = row.user_id {
            if let Some((uid, username, nickname, mobile, avatar)) =
                self.repo().find_users(&[user_id]).await?.into_iter().next()
            {
                vo.user = Some(serde_json::json!({
                    "userId": uid,
                    "username": username,
                    "nickname": nickname,
                    "mobile": mobile,
                    "avatar": avatar,
                }));
            }
        }
        vo.audit_admin_name = if row.audit_admin_id.unwrap_or(0) > 0 {
            self.repo()
                .admin_usernames(&[row.audit_admin_id.unwrap_or(0)])
                .await?
                .into_iter()
                .next()
                .map(|(_, name)| name)
                .unwrap_or_default()
        } else {
            String::new()
        };
        Ok(vo)
    }

    /// 更新（Java `#update`；DTO 为空类 → 无操作成功）。
    ///
    /// # Errors
    /// 恒成功（保留 Java 行为）。
    pub fn update(&self, _dto: &MerchantApplyUpdateDTO) -> Result<()> {
        Ok(())
    }

    /// 删除（Java `#del` / `BaseServiceImpl#del`）。
    ///
    /// # Errors
    /// id 缺失或不存在 → 「商户申请信息不存在」；数据库失败 → `Error::Database`。
    pub async fn del(&self, id: Option<i32>) -> Result<()> {
        let id = id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MERCHANT_APPLY_NOT_EXIST))?;
        if !self.repo().delete(id).await? {
            return Err(Error::biz(SERVICE_DATA_ERROR, MERCHANT_APPLY_NOT_EXIST));
        }
        Ok(())
    }

    /// 字段更新（Java `#updateField(updateField, GENERAL_FIELDS)`）。
    ///
    /// 先按 `getColumnByPropertyName` 映射列名，再与 `GENERAL_FIELDS` 比对。
    ///
    /// # Errors
    /// id/字段/值缺失 → 对应文案；字段不在白名单 → 「field 的值有误」。
    pub async fn update_field(
        &self,
        id: Option<i32>,
        field: Option<&str>,
        val: Option<&str>,
    ) -> Result<bool> {
        let id = id.ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "id不能为空"))?;
        let column = apply_field_to_column(field);
        if !is_general_field(column) {
            return Err(Error::biz(SERVICE_DATA_ERROR, "field 的值有误"));
        }
        self.repo().update_field(id, column, val).await
    }

    /// 批量操作（Java `BaseServiceImpl#batch`：仅 `del` 生效）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn batch(&self, ids: Option<&[i32]>, op_type: Option<&str>) -> Result<bool> {
        if op_type != Some("del") {
            return Ok(false);
        }
        let Some(ids) = ids.filter(|i| !i.is_empty()) else {
            return Ok(false);
        };
        self.repo().delete_many(ids).await?;
        Ok(true)
    }

    /// 审核（Java `#audit`）。
    ///
    /// 1. 申请必须存在（否则「资源不存在」）且状态为待审核（否则「审核状态不正确」）；
    /// 2. 目标状态只能是 10/20；
    /// 3. 通过时校验会员已绑定手机号（「请先绑定手机号」），并创建 `merchant` 行；
    /// 4. 写回状态/备注/审核时间/审核人。
    ///
    /// ⚠️ 审核通过后的开店链路（管理员/店铺/保证金/通知）未迁移，见模块文档。
    ///
    /// # Errors
    /// 上述业务错误或数据库失败。
    pub async fn audit(&self, param: &MerchantApplyAuditParam, audit_admin_id: i32) -> Result<()> {
        let id = param
            .merchant_apply_id
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, "id不能为空"))?;
        let status = param
            .status
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, AUDIT_STATUS_INVALID))?;
        if !valid_audit_status(status) {
            return Err(Error::biz(SERVICE_DATA_ERROR, AUDIT_STATUS_INVALID));
        }
        let apply = self
            .repo()
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, RESOURCE_NOT_FOUND))?;
        if apply.status != Some(STATUS_PENDING) {
            return Err(Error::biz(SERVICE_DATA_ERROR, AUDIT_STATUS_INVALID));
        }

        let now = i32::try_from(now_seconds()).unwrap_or(0);
        self.repo()
            .update_audit(
                id,
                status,
                param.audit_remark.as_deref(),
                now,
                audit_admin_id,
            )
            .await?;

        if status == STATUS_PASS {
            self.open_shop_for_passed_apply(&apply, now).await?;
            self.notify_audit_result(&apply, true, param.audit_remark.as_deref()).await;
        } else {
            self.notify_audit_result(&apply, false, param.audit_remark.as_deref()).await;
        }
        Ok(())
    }

    /// 审核结果通知适配器：配置了 webhook 才发送，失败不阻塞审核主流程。
    async fn notify_audit_result(
        &self,
        apply: &nextrade_core::entities::MerchantApply,
        passed: bool,
        remark: Option<&str>,
    ) {
        let url: Option<String> = match sqlx::query_scalar::<_, String>(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("merchantApplyNotifyUrl")
        .fetch_optional(&self.repos.pool)
        .await
        {
            Ok(v) => v.filter(|s| !s.trim().is_empty()),
            Err(e) => {
                tracing::warn!("读取商户审核通知地址失败: {e}");
                return;
            }
        };
        let Some(url) = url else {
            tracing::info!(apply_id = apply.merchant_apply_id, passed, "未配置商户审核通知 webhook");
            return;
        };
        let key: Option<String> = sqlx::query_scalar(
            "SELECT biz_val FROM config WHERE biz_code=$1 LIMIT 1",
        )
        .bind("merchantApplyNotifyKey")
        .fetch_optional(&self.repos.pool)
        .await
        .ok()
        .flatten();
        let merchant_json: Value = apply
            .merchant_data
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(Value::Null);
        let phone = merchant_json
            .get("contactPhone")
            .and_then(Value::as_str)
            .or(apply.contact_mobile.as_deref())
            .unwrap_or("");
        let email = merchant_json
            .get("contactEmail")
            .and_then(Value::as_str)
            .unwrap_or("");
        let payload = serde_json::json!({
            "event": if passed { "MERCHANT_SETTLED_IN_SUCCESS" } else { "MERCHANT_SETTLED_IN_FAIL" },
            "merchantApplyId": apply.merchant_apply_id,
            "shopTitle": apply.shop_title,
            "companyName": apply.company_name,
            "contactPhone": phone,
            "contactEmail": email,
            "remark": remark,
            "username": phone,
            "initialPassword": if passed { Some("admin123321") } else { None }
        });
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("创建商户审核通知客户端失败: {e}");
                return;
            }
        };
        let mut req = client
            .post(url.trim())
            .header("Content-Type", "application/json")
            .body(payload.to_string());
        if let Some(k) = key.filter(|v| !v.trim().is_empty()) {
            req = req.header("Authorization", format!("Bearer {}", k.trim()));
        }
        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(apply_id = apply.merchant_apply_id, passed, "商户审核通知已发送");
            }
            Ok(resp) => {
                tracing::warn!("商户审核通知返回非 2xx: {}", resp.status());
            }
            Err(e) => {
                tracing::warn!("商户审核通知发送失败: {e}");
            }
        }
    }

    /// 免审核/人工审核共用的开店入口：建 merchant + 管理员/店铺/角色/账户/模板。
    pub async fn open_shop_for_passed_apply(
        &self,
        apply: &nextrade_core::entities::MerchantApply,
        now: i32,
    ) -> Result<i32> {
        let user_id = apply.user_id.unwrap_or(0);
        let users = self.repo().find_users(&[user_id]).await?;
        let mobile = users.first().map(|u| u.3.clone()).unwrap_or_default();
        if mobile.trim().is_empty() {
            return Err(Error::biz(SERVICE_DATA_ERROR, MOBILE_NOT_BOUND));
        }
        let merchant_id = self
            .repo()
            .insert_merchant_from_apply(
                apply.merchant_apply_id,
                user_id,
                now,
                apply.base_data.as_deref(),
                apply.shop_data.as_deref(),
                apply.merchant_data.as_deref(),
                apply.r#type.unwrap_or(1),
                apply.company_name.as_deref(),
                apply.corporate_name.as_deref(),
            )
            .await?;
        self.open_shop_after_audit(apply, merchant_id, now).await?;
        Ok(merchant_id)
    }

    /// 审核通过后的开店：管理员、店铺、商户用户、店铺管理员关联。
    async fn open_shop_after_audit(
        &self,
        apply: &nextrade_core::entities::MerchantApply,
        merchant_id: i32,
        now: i32,
    ) -> Result<()> {
        let merchant_json: Value = apply
            .merchant_data
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(Value::Null);
        let phone = merchant_json
            .get("contactPhone")
            .and_then(Value::as_str)
            .or(apply.contact_mobile.as_deref())
            .unwrap_or("")
            .trim()
            .to_string();
        if phone.is_empty() {
            return Err(Error::biz(SERVICE_DATA_ERROR, "联系人电话不能为空"));
        }
        let email = merchant_json
            .get("contactEmail")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let shop_json: Value = apply
            .shop_data
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(Value::Null);
        let shop_logo = shop_json
            .get("shopLogo")
            .or_else(|| shop_json.get("shop_logo"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let shop_title = apply
            .shop_title
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("店铺{merchant_id}"));
        let password = nextrade_security::hash_password("admin123321")?;
        let admin_id = match self
            .repos
            .identity
            .admin_user
            .find_by_mobile(&phone, Some(false), None)
            .await?
        {
            Some(admin) => admin.admin_id,
            None => {
                let r: i32 = sqlx::query_scalar(
                    "INSERT INTO admin_user(username,admin_type,mobile,avatar,password,email,add_time,auth_list,user_id,suppliers_id,role_id,merchant_id,parent_id,shop_id,is_using,initial_password,is_deleted) \
                     VALUES(?, 'shop', ?, '', ?, ?, ?, '[]', ?, 0, 1, ?, 0, 0, 1, ?, 0) RETURNING admin_id",
                )
                .bind(&phone)
                .bind(&phone)
                .bind(&password)
                .bind(&email)
                .bind(now)
                .bind(apply.user_id.unwrap_or(0))
                .bind(merchant_id)
                .bind(&password)
                .fetch_one(&self.repos.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
                r
            }
        };
        if admin_id <= 0 {
            return Err(Error::Internal("创建店铺管理员失败".into()));
        }
        let shop_id: i32 = sqlx::query_scalar("SELECT COALESCE(MAX(shop_id),0)+1 FROM shop")
            .fetch_one(&self.repos.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        sqlx::query(
            "INSERT INTO shop(shop_id,shop_title,add_time,shop_logo,click_count,status,merchant_id,shop_money,frozen_money,contact_mobile,description,is_contact_kefu,last_login_time,auto_erase_zero,auto_deduction_points,default_collection_method,auto_add_cart_after_scan,auto_print_online_order,auto_print_offline_order,pos_industry_type,is_delete) \
             VALUES(?,?,?,?,0,1,?,0,0,?,'',0,0,0,0,0,0,0,0,0,0)",
        )
        .bind(shop_id)
        .bind(&shop_title)
        .bind(now)
        .bind(&shop_logo)
        .bind(merchant_id)
        .bind(&phone)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let store_role_id: i32 = sqlx::query_scalar(
            "INSERT INTO admin_role(role_name,role_code,role_desc,authority_list,admin_type,merchant_id,shop_id,vendor_id,is_system) VALUES(?,?,?,?,?,?,?,0,0) RETURNING role_id",
        )
        .bind("店铺管理员")
        .bind("store_admin")
        .bind("店铺管理员")
        .bind("[\"all\"]")
        .bind("shop")
        .bind(merchant_id)
        .bind(shop_id)
        .fetch_one(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        let _ = sqlx::query(
            "INSERT INTO admin_role(role_name,role_code,role_desc,authority_list,admin_type,merchant_id,shop_id,vendor_id,is_system) VALUES($1,$2,$3,$4,$5,$6,$7,0,0)",
        )
        .bind("店铺员工")
        .bind("store_employee")
        .bind("店铺员工")
        .bind("[]")
        .bind("shop")
        .bind(merchant_id)
        .bind(shop_id)
        .execute(&self.repos.pool)
        .await;
        let _ = sqlx::query("UPDATE admin_user SET role_id=$1 WHERE admin_id=$2")
            .bind(store_role_id)
            .bind(admin_id)
            .execute(&self.repos.pool)
            .await;
        // 默认运费模板（Java ShippingTplServiceImpl#initShippingTpl）
        let tpl_id: i32 = sqlx::query_scalar(
            "INSERT INTO shipping_tpl(shipping_time,shipping_tpl_name,is_free,pricing_type,is_default,shop_id) VALUES($1,$2,0,1,1,$3) RETURNING shipping_tpl_id",
        )
        .bind("1天内").bind("默认模板").bind(shop_id)
        .fetch_one(&self.repos.pool).await.map_err(|e| Error::Database(e.to_string()))?;
        let region_data = "{\"areaRegions\":[],\"areaRegionNames\":[]}";
        let _ = sqlx::query(
            "INSERT INTO shipping_tpl_info(shipping_type_id,shipping_tpl_id,is_free,is_default,region_data,start_number,start_price,add_number,add_price,free_price,pricing_type) VALUES(1,$1,0,1,$2,1,1,1,1,0,1)",
        )
        .bind(tpl_id).bind(region_data)
        .execute(&self.repos.pool).await;
        // 默认装修页（Java DecorateServiceImpl#initDecorate）
        let _ = sqlx::query(
            "INSERT INTO decorate(decorate_title,data,decorate_type,page_type,is_home,shop_id,status,update_time,locale_id,parent_id,sort_order,binded_h5_id) VALUES($1 ,$2,1,1,1,$3,1,$4,0,0,0,0)",
        )
        .bind("默认模板")
        .bind(crate::merchant::default_decorate::DEFAULT_DECORATE_DATA)
        .bind(shop_id)
        .bind(now)
        .execute(&self.repos.pool).await;
        let bank_card = merchant_json.get("bank_card").and_then(Value::as_str).unwrap_or("").trim();
        if !(apply.r#type == Some(3) && bank_card.is_empty()) {
            let account_name = if apply.r#type == Some(1) {
                apply.corporate_name.clone().unwrap_or_default()
            } else {
                apply.company_name.clone().unwrap_or_default()
            };
            let bank_name = merchant_json.get("bank_deposit").and_then(Value::as_str).unwrap_or("");
            let bank_branch = merchant_json.get("bank_branch").and_then(Value::as_str).unwrap_or("");
            let _ = sqlx::query(
                "INSERT INTO merchant_account(merchant_id,account_type,account_name,account_no,bank_name,add_time,bank_branch,shop_id) VALUES($1,1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(merchant_id)
            .bind(&account_name)
            .bind(bank_card)
            .bind(bank_name)
            .bind(now)
            .bind(bank_branch)
            .bind(shop_id)
            .execute(&self.repos.pool)
            .await;
        }
        let _ = sqlx::query(
            "INSERT INTO merchant_user(merchant_user_id,user_id,admin_user_id,merchant_id,is_admin) \
             SELECT COALESCE(MAX(merchant_user_id),0)+1, ?, ?, ?, 1 FROM merchant_user",
        )
        .bind(apply.user_id.unwrap_or(0))
        .bind(admin_id)
        .bind(merchant_id)
        .execute(&self.repos.pool)
        .await;
        sqlx::query(
            "INSERT INTO admin_user_shop(admin_id,user_id,shop_id,username,email,avatar,auth_list,is_using,is_admin,add_time,role_id,work_on_time,default_pos_login,last_login_system,device_code) \
             VALUES(?,?,?,?,?,?,'[\"all\"]',1,1,?,?,0,0,'','')",
        )
        .bind(admin_id)
        .bind(apply.user_id.unwrap_or(0))
        .bind(shop_id)
        .bind(&phone)
        .bind(&email)
        .bind("")
        .bind(now)
        .bind(store_role_id)
        .execute(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        tracing::info!(merchant_id, shop_id, admin_id, "商户审核通过，开店链路已完成");
        Ok(())
    }

    /// 状态参数（Java `#config`）。
    #[must_use]
    pub fn config(&self) -> Value {
        status_options()
    }
}

/// 实体 → VO（Java `BeanUtils.copyProperties` + 文案/时间处理）。
fn to_vo(row: &nextrade_core::entities::MerchantApply) -> MerchantApplyVO {
    MerchantApplyVO {
        merchant_apply_id: row.merchant_apply_id,
        user_id: row.user_id.unwrap_or(0),
        shop_title: row.shop_title.clone(),
        status: row.status.unwrap_or(0),
        r#type: row.r#type.unwrap_or(1),
        contact_name: row.contact_name.clone(),
        contact_mobile: row.contact_mobile.clone(),
        shop_data: row.shop_data.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        base_data: row.base_data.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        merchant_data: row.merchant_data.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        company_name: row.company_name.clone(),
        corporate_name: row.corporate_name.clone(),
        audit_remark: row.audit_remark.clone(),
        user: None,
        status_text: status_text(row.status.unwrap_or(0)).to_string(),
        add_time: format_time(row.add_time),
        audit_time: format_time(row.audit_time),
        audit_admin_id: row.audit_admin_id.unwrap_or(0),
        audit_admin_name: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::merchant::merchant_apply::{
        apply_field_to_column, is_general_field, status_text, valid_audit_status, STATUS_REJECT,
    };

    #[test]
    fn audit_status_guard_matches_java_enum() {
        assert!(valid_audit_status(STATUS_PASS));
        assert!(valid_audit_status(STATUS_REJECT));
        assert!(!valid_audit_status(STATUS_PENDING));
        assert_eq!(status_text(STATUS_REJECT), "审核未通过");
    }

    #[test]
    fn update_field_whitelist_is_general_fields() {
        // GENERAL_FIELDS = is_show / sort_order / status；merchant_apply 只有 status 列
        assert!(is_general_field(&apply_field_to_column(Some("status"))));
        assert!(!is_general_field(&apply_field_to_column(Some("shopTitle"))));
        assert!(!is_general_field(&apply_field_to_column(Some("unknown"))));
    }

    #[test]
    fn update_dto_is_empty_like_java() {
        // Java MerchantApplyUpdateDTO 无字段：反序列化任意 JSON 都是空对象
        let dto: MerchantApplyUpdateDTO = serde_json::from_str(r#"{"anything":1}"#).unwrap();
        let _ = dto;
    }
}
