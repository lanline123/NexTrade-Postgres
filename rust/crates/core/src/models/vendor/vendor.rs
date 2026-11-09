//! 供应商管理（`/adminapi/vendor/vendor`）DTO。
//!
//! Java 来源（overseas）：
//! - `com.nextrade.bean.query.vendor.VendorListPageParam`（list 查询）
//! - `com.nextrade.bean.vo.vendor.VendorVO`（list 行）
//! - `com.nextrade.bean.param.vendor.VendorCreateParam`（create 入参）
//! - `com.nextrade.bean.vo.vendor.VendorDetailVO` / `VendorDetailConfigVO`（detail / currentDetail）
//! - `com.nextrade.bean.param.vendor.VendorConfigParam`（setting 入参）

use serde::{Deserialize, Serialize};

/// `GET /adminapi/vendor/vendor/list` 查询参数（Java `VendorListPageParam`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorListQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    /// 兼容 `size` / `pageSize` 两种参数名（原前端两种都在用）。
    #[serde(default = "default_size", alias = "pageSize")]
    pub size: u64,
    /// 关键字：供应商名称 / 联系人手机 模糊。
    pub keyword: Option<String>,
    /// 状态 1开启 2关闭（<=0 忽略）。
    pub status: Option<i32>,
    /// 主账号（登录名精确匹配 admin_user.username）。
    pub main_account: Option<String>,
    /// 子账号（登录名精确匹配 admin_user.username）。
    pub account: Option<String>,
    /// 逗号分隔的供应商 ID 集合（前端 `vendorIds` 传字符串）。
    pub vendor_ids: Option<String>,
}

fn default_page() -> u64 {
    1
}

fn default_size() -> u64 {
    20
}

impl VendorListQuery {
    /// 解析 `vendorIds`（逗号分隔字符串 → 去重非零 i32 列表）。
    #[must_use]
    pub fn vendor_id_list(&self) -> Vec<i32> {
        self.vendor_ids
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|x| x.trim().parse::<i32>().ok())
                    .filter(|x| *x > 0)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 分页 LIMIT/OFFSET。
    #[must_use]
    pub fn limit_offset(&self) -> (i64, i64) {
        let size = self.size.clamp(1, 200) as i64;
        let page = self.page.max(1) as i64;
        (size, (page - 1) * size)
    }
}

/// list 行 VO（Java `VendorVO`）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorListVO {
    pub vendor_id: i32,
    pub vendor_name: Option<String>,
    pub vendor_logo: Option<String>,
    /// 联系人姓名（来自 `vendor_data` JSON 的 `contactName`）。
    pub contact_name: Option<String>,
    pub contact_mobile: Option<String>,
    /// 主账号登录名（admin_user_vendor(is_admin=1) → admin_user.username）。
    pub login_account: Option<String>,
    /// 所属性质 1个人 2企业。
    pub r#type: Option<i32>,
    /// 供应商状态 1开启 2关闭。
    pub status: Option<i32>,
    /// 添加时间（格式 `yyyy-MM-dd HH:mm:ss`，对齐 Java `TimestampToDateSerializer`）。
    pub add_time: Option<String>,
}

/// create 入参（Java `VendorCreateParam`）。
///
/// `vendorData` / `personData` 在 Java 里是强类型嵌套对象后整体转 JSON 存储；
/// 这里按 `serde_json::Value` 透传（存库时序列化为字符串），仅抽取出
/// `contactPhone` / `customerServicePhone` 两个落列字段。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorCreateParam {
    pub vendor_logo: Option<String>,
    #[serde(default)]
    pub vendor_data: serde_json::Value,
    #[serde(default)]
    pub person_data: serde_json::Value,
    /// 类型 1个人 2企业。
    pub r#type: Option<i32>,
    pub vendor_name: Option<String>,
    /// 主账号绑定信息（Java `VendorCreateParam.AdminData`）。
    #[serde(default)]
    pub admin_data: Option<VendorAdminInfo>,
}

/// 主账号绑定信息（Java `VendorCreateParam.AdminData`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorAdminInfo {
    /// 1关联会员 2关联管理员（Java `AdminInfoEnum`）。
    pub r#type: Option<i32>,
    pub admin_id: Option<i32>,
    pub user_id: Option<i32>,
    pub username: Option<String>,
}

/// `vendor_data` 中本实现需要的两个键（对齐 Java `VendorCreateParam.VendorData`）。
pub const VENDOR_DATA_CONTACT_PHONE: &str = "contactPhone";
pub const VENDOR_DATA_CUSTOMER_SERVICE_PHONE: &str = "customerServicePhone";
/// `vendor_data` 中联系人姓名（list 的 `contactName`）。
pub const VENDOR_DATA_CONTACT_NAME: &str = "contactName";

/// detail / currentDetail VO（Java `VendorDetailVO` + `VendorDetailConfigVO` 合并形态）。
///
/// `vendorData` / `personData` / `shipInfo` / `receiveInfo` 按 JSON 原样透传
/// （Java 是转 bean 后返回，字段超集，透传不丢信息）。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorDetailVO {
    pub vendor_id: i32,
    /// 类型 1个人 2企业。
    pub r#type: Option<i32>,
    pub vendor_logo: Option<String>,
    pub vendor_name: Option<String>,
    /// 状态 1开启 2关闭。
    pub status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_data: Option<serde_json::Value>,
    /// 主账号（Java `VendorDetailVO.AdminInfo`：type/adminId/userId/accountName）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_data: Option<serde_json::Value>,
    // ———— 以下仅 currentDetail（Java `VendorDetailConfigVO`）返回 ————
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kefu_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ship_info: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_info: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 添加时间（`yyyy-MM-dd HH:mm:ss`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_time: Option<String>,
}

/// setting 入参（Java `VendorConfigParam`）。
///
/// `shipInfo` / `receiveInfo` 整体 JSON 存储（Java `DeliveryInfo` bean）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorConfigParam {
    /// 状态 1开启 2关闭（必填，非法值报「参数错误」）。
    pub status: Option<i32>,
    pub kefu_phone: Option<String>,
    #[serde(default)]
    pub ship_info: Option<serde_json::Value>,
    #[serde(default)]
    pub receive_info: Option<serde_json::Value>,
}

/// 状态枚举值（Java `VendorStatusEnum`）。
pub const VENDOR_STATUS_OPEN: i32 = 1;
pub const VENDOR_STATUS_CLOSE: i32 = 2;

/// Unix 秒 → `yyyy-MM-dd HH:mm:ss`（对齐 Java `TimestampToDateSerializer` 本地时区口径）。
#[must_use]
pub fn format_unix_time(secs: i64) -> String {
    use chrono::TimeZone;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_default();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_ids_parses_comma_list() {
        let q: VendorListQuery =
            serde_json::from_str(r#"{"page":1,"size":10,"vendorIds":"1, 2,abc,0,3"}"#).unwrap();
        assert_eq!(q.vendor_id_list(), vec![1, 2, 3]);
    }

    #[test]
    fn create_param_accepts_camel_case() {
        let p: VendorCreateParam = serde_json::from_str(
            r#"{"vendorName":"n","type":2,"vendorData":{"contactPhone":"138"},"adminData":{"type":2,"adminId":5}}"#,
        )
        .unwrap();
        assert_eq!(p.r#type, Some(2));
        assert_eq!(p.admin_data.as_ref().unwrap().admin_id, Some(5));
    }

    #[test]
    fn format_unix_time_shape() {
        assert_eq!(format_unix_time(0).len(), 19);
    }
}
