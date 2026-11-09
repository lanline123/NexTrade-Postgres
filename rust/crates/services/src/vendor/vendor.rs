//! 供应商管理域服务（vendor::vendor）。
//!
//! 对应原 Java（overseas）`com.nextrade.service.vendor.impl.VendorServiceImpl`
//! + `VendorController`（`@RequestMapping("/adminapi/vendor/vendor")`）：
//!
//! | 端点（挂载于 `/adminapi`） | 原 Java |
//! |---------------------------|---------|
//! | `GET  /vendor/vendor/list` | `VendorController.list` → `VendorServiceImpl#list` |
//! | `POST /vendor/vendor/create` | `VendorController.create` → `#create` |
//! | `GET  /vendor/vendor/detail` | `VendorController.detail` → `#detail` |
//! | `GET  /vendor/vendor/currentDetail` | `VendorController.currentDetail` → `#currentDetail` |
//! | `POST /vendor/vendor/setting` | `VendorController.setting` → `#setting` |
//!
//! ## 与 Java 的已知差异（最小实现取舍，均列入模块尾注释）
//!
//! 1. `list` 的 `mainAccount` / `account` 过滤只实现「按登录名精确匹配
//!    admin_user → admin_user_vendor → vendor_ids」主链路，不做 Java 里
//!    「主账号/子账号区分」（Java `getBindMainVendorIds(account, isMain)` 的
//!    is_main 分支只影响 `is_admin` 过滤，最小实现合并为同一集合）。
//! 2. `create` 的 `adminData.type = 1`（关联会员）**不自动创建 admin_user**
//!    （Java 会用默认密码建号）；仅当 adminData 指向已存在管理员（type=2）
//!    或缺省绑定当前操作人。前端「新建供应商」默认走 type=2。
//! 3. `create` 不初始化供应商角色（Java `adminRoleService.initVendorRole`），
//!    `auth_list` 写空数组、`role_id = 0`。
//! 4. `setting` 关闭供应商时 Java 发 RabbitMQ 下架全部商品；本实现**不级联下架**
//!    （无 MQ 通道），仅更新状态。

use std::sync::Arc;

use nextrade_core::entities::vendor::AdminUserVendor;
use nextrade_core::models::vendor::vendor::{
    format_unix_time, VendorConfigParam, VendorCreateParam, VendorDetailVO, VendorListQuery,
    VendorListVO, VENDOR_DATA_CONTACT_NAME, VENDOR_DATA_CONTACT_PHONE,
    VENDOR_DATA_CUSTOMER_SERVICE_PHONE, VENDOR_STATUS_CLOSE, VENDOR_STATUS_OPEN,
};
use nextrade_core::types::{PageQuery, PageResult};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 供应商管理域服务（挂载于 `AdminState.vendor.vendor`）。
pub struct VendorService {
    repos: Arc<Repositories>,
}

impl VendorService {
    /// 构造（与其它切片服务一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn vendor_repo(&self) -> &nextrade_infrastructure::db::repo::vendor::vendor::VendorRepository {
        &self.repos.vendor.vendor
    }

    // ————————————————————— list —————————————————————

    /// 供应商分页（Java `VendorServiceImpl#list`）。
    ///
    /// `vendor_scope`：供应商账号登录时只看自己；平台账号看全部。
    pub async fn list(
        &self,
        q: &VendorListQuery,
        vendor_scope: Option<i32>,
    ) -> Result<PageResult<VendorListVO>> {
        // mainAccount / account → admin_user → vendor_ids（Java 两段查询）
        let mut extra: Vec<i32> = q.vendor_id_list();
        for account in [q.main_account.as_deref(), q.account.as_deref()]
            .into_iter()
            .flatten()
        {
            if account.trim().is_empty() {
                continue;
            }
            let admin_ids = self
                .vendor_repo()
                .find_admin_ids_by_username(account.trim())
                .await?;
            let vendor_ids = self
                .vendor_repo()
                .find_vendor_ids_by_admins(&admin_ids)
                .await?;
            if vendor_ids.is_empty() {
                // Java：查不到绑定直接返回空页
                return Ok(PageResult::new(0, Vec::new(), &PageQuery::default()));
            }
            extra.extend(vendor_ids);
        }
        extra.sort_unstable();
        extra.dedup();

        let (total, rows) = self
            .vendor_repo()
            .paginate(q, &extra, vendor_scope)
            .await?;

        // VO 组装：contactName（vendor_data JSON）、loginAccount（主账号绑定）
        let mut vos = Vec::with_capacity(rows.len());
        for vendor in rows {
            let contact_name = vendor
                .vendor_data
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| {
                    v.get(VENDOR_DATA_CONTACT_NAME)
                        .and_then(|x| x.as_str().map(str::to_string))
                });
            let login_account = self.main_login_account(vendor.vendor_id).await?;
            vos.push(VendorListVO {
                vendor_id: vendor.vendor_id,
                vendor_name: vendor.vendor_name.clone(),
                vendor_logo: vendor.vendor_logo.clone(),
                contact_name,
                contact_mobile: vendor.contact_mobile.clone(),
                login_account,
                r#type: vendor.r#type,
                status: vendor.status,
                add_time: vendor.add_time.map(|t| format_unix_time(i64::from(t))),
            });
        }
        let query = PageQuery {
            page: q.page,
            page_size: q.size,
        };
        Ok(PageResult::new(total, vos, &query))
    }

    /// 供应商主账号登录名（`admin_user_vendor.is_admin = 1` 的第一行 → admin_user.username）。
    async fn main_login_account(&self, vendor_id: i32) -> Result<Option<String>> {
        let binds = self.vendor_repo().find_admin_user_vendors(vendor_id).await?;
        let admin = binds.iter().find(|b| b.is_admin == 1);
        let Some(admin) = admin else {
            return Ok(None);
        };
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT username FROM admin_user WHERE admin_id = $1 LIMIT 1",
        )
        .bind(admin.admin_id)
        .fetch_optional(&self.repos.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    // ————————————————————— create —————————————————————

    /// 创建供应商（Java `VendorServiceImpl#create` 最小化）。
    ///
    /// 落 `vendor` 行 + `admin_user_vendor` 绑定行：
    /// - `adminData.type = 2`（管理员）：绑定该 adminId；
    /// - 其它 / 缺省：绑定当前操作人。
    pub async fn create(
        &self,
        p: &VendorCreateParam,
        current_admin_id: i32,
        current_username: &str,
    ) -> Result<i32> {
        if p.vendor_name.as_deref().unwrap_or("").trim().is_empty() {
            return Err(Error::Validation("供应商名称不能为空".into()));
        }
        let contact_phone = json_str(&p.vendor_data, VENDOR_DATA_CONTACT_PHONE);
        let kefu_phone = json_str(&p.vendor_data, VENDOR_DATA_CUSTOMER_SERVICE_PHONE);
        let vendor_data = json_to_string(&p.vendor_data);
        let person_data = json_to_string(&p.person_data);
        let now = nextrade_common::utils::now_millis() as i32;

        let vendor_id = self
            .vendor_repo()
            .insert(
                p.vendor_logo.as_deref(),
                contact_phone.as_deref(),
                vendor_data.as_deref(),
                person_data.as_deref(),
                VENDOR_STATUS_OPEN,
                p.r#type,
                p.vendor_name.as_deref(),
                kefu_phone.as_deref(),
                now,
            )
            .await?;

        // 绑定主账号：adminData.type=2 用指定 adminId，否则绑定当前操作人
        let (bind_admin_id, bind_username) = match p
            .admin_data
            .as_ref()
            .filter(|a| a.r#type == Some(2))
            .and_then(|a| a.admin_id.filter(|id| *id > 0))
        {
            Some(admin_id) => {
                let username: Option<(String,)> = sqlx::query_as(
                    "SELECT username FROM admin_user WHERE admin_id = $1 LIMIT 1",
                )
                .bind(admin_id)
                .fetch_optional(&self.repos.pool)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
                match username {
                    Some((name,)) => (admin_id, name),
                    None => {
                        return Err(Error::Biz {
                            code: 500,
                            msg: "管理员不存在".into(),
                        })
                    }
                }
            }
            None => (current_admin_id, current_username.to_string()),
        };

        let bind = AdminUserVendor {
            id: 0,
            admin_id: bind_admin_id,
            user_id: p
                .admin_data
                .as_ref()
                .and_then(|a| a.user_id)
                .unwrap_or(0),
            vendor_id,
            username: bind_username,
            email: String::new(),
            avatar: None,
            auth_list: Some("[]".into()),
            is_using: 0,
            is_admin: 1,
            add_time: Some(now),
            role_id: 0,
        };
        self.vendor_repo().insert_admin_user_vendor(&bind).await?;
        Ok(vendor_id)
    }

    // ————————————————————— detail / currentDetail —————————————————————

    /// 供应商详情（Java `VendorServiceImpl#detail`）。
    pub async fn detail(&self, vendor_id: i32) -> Result<VendorDetailVO> {
        let vendor = self
            .vendor_repo()
            .find_by_id(vendor_id)
            .await?
            .ok_or_else(|| Error::Biz {
                code: 500,
                msg: "供应商不存在".into(),
            })?;

        // 主账号信息（Java AdminInfo：type=1 会员 / 2 管理员 + accountName）
        let binds = self.vendor_repo().find_admin_user_vendors(vendor_id).await?;
        let admin_data = binds.iter().find(|b| b.is_admin == 1).map(|b| {
            serde_json::json!({
                "type": if b.user_id == 0 { 2 } else { 1 },
                "adminId": b.admin_id,
                "userId": b.user_id,
            })
        });

        Ok(VendorDetailVO {
            vendor_id: vendor.vendor_id,
            r#type: vendor.r#type,
            vendor_logo: vendor.vendor_logo.clone(),
            vendor_name: vendor.vendor_name.clone(),
            status: vendor.status,
            vendor_data: parse_json(vendor.vendor_data.as_deref()),
            person_data: parse_json(vendor.person_data.as_deref()),
            admin_data,
            ..Default::default()
        })
    }

    /// 当前登录管理员绑定的供应商配置详情（Java `VendorServiceImpl#currentDetail`）。
    ///
    /// ⚠️ 任务口径：**当前管理员无绑定时返回 `Ok(None)`**（code:0 + data:null），
    /// 而非 Java 的 `GlobalException(VENDOR_NOT_EXIST)`。
    pub async fn current_detail(
        &self,
        admin_id: i32,
        vendor_scope: Option<i32>,
    ) -> Result<Option<VendorDetailVO>> {
        let vendor_id = match vendor_scope {
            Some(id) => id,
            None => match self.vendor_repo().find_vendor_id_by_admin(admin_id).await? {
                Some(id) => id,
                None => return Ok(None),
            },
        };
        let vendor = match self.vendor_repo().find_by_id(vendor_id).await? {
            Some(v) => v,
            None => return Ok(None),
        };
        let mut vo = self.detail(vendor_id).await?;
        vo.kefu_phone = vendor.kefu_phone.clone();
        vo.ship_info = parse_json(vendor.ship_info.as_deref());
        vo.receive_info = parse_json(vendor.receive_info.as_deref());
        vo.description = vendor.description.clone();
        vo.add_time = vendor.add_time.map(|t| format_unix_time(i64::from(t)));
        Ok(Some(vo))
    }

    // ————————————————————— setting —————————————————————

    /// 供应商设置保存（Java `VendorServiceImpl#setting` 最小化，不级联下架商品）。
    pub async fn setting(
        &self,
        vendor_id: i32,
        p: &VendorConfigParam,
    ) -> Result<()> {
        let status = p.status.ok_or_else(|| Error::Biz {
            code: 500,
            msg: "参数错误".into(),
        })?;
        if status != VENDOR_STATUS_OPEN && status != VENDOR_STATUS_CLOSE {
            return Err(Error::Biz {
                code: 500,
                msg: "参数错误".into(),
            });
        }
        let vendor = self
            .vendor_repo()
            .find_by_id(vendor_id)
            .await?
            .ok_or_else(|| Error::Biz {
                code: 500,
                msg: "供应商不存在".into(),
            })?;

        // Java：把 kefuPhone 同步写回 vendor_data.customerServicePhone
        let vendor_data = vendor.vendor_data.as_deref().map(|raw| {
            let mut val = serde_json::from_str::<serde_json::Value>(raw)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            if let Some(obj) = val.as_object_mut() {
                match &p.kefu_phone {
                    Some(v) => {
                        obj.insert(
                            VENDOR_DATA_CUSTOMER_SERVICE_PHONE.into(),
                            serde_json::Value::String(v.clone()),
                        );
                    }
                    None => {
                        obj.remove(VENDOR_DATA_CUSTOMER_SERVICE_PHONE);
                    }
                }
            }
            val.to_string()
        });

        self.vendor_repo()
            .update_setting(
                vendor_id,
                status,
                p.kefu_phone.as_deref(),
                p.ship_info.as_ref().map(serde_json::Value::to_string).as_deref(),
                p.receive_info.as_ref().map(serde_json::Value::to_string).as_deref(),
                vendor_data.as_deref(),
            )
            .await?;
        Ok(())
    }
}

/// 取 JSON 对象里的字符串字段。
fn json_str(val: &serde_json::Value, key: &str) -> Option<String> {
    val.get(key).and_then(|v| v.as_str().map(str::to_string))
}

/// 序列化 JSON（`null` → `None`，避免落库字符串 "null"）。
fn json_to_string(val: &serde_json::Value) -> Option<String> {
    if val.is_null() {
        None
    } else {
        Some(val.to_string())
    }
}

/// 解析 text 列 JSON（空 / 非法 → `None`）。
fn parse_json(raw: Option<&str>) -> Option<serde_json::Value> {
    let s = raw?.trim();
    if s.is_empty() {
        return None;
    }
    serde_json::from_str(s).ok()
}
