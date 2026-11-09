//! 微信直播（`content/wechat_live`）DTO / VO / 查询参数。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.content.WechatLiveController`（14 条）
//! - `com.nextrade.bean.dto.content.WechatLiveListDTO` / `WechatLiveUpdateDTO` /
//!   `WechatLiveRoomCreateDTO` / `WechatLiveGoodsAddDTO`
//! - `com.nextrade.bean.vo.content.WechatLiveVO` / `WechatLiveGoodsVO` /
//!   `WechatLivePushUrlVO` / `WechatLivePromoteVO`
//! - 枚举 `WechatLiveStatus` / `WechatLiveAuditStatus`
//!
//! 表结构见 `entities::content::{WechatLive, WechatLiveGoods, WechatLiveRoomGoods,
//! WechatLiveOrderRel, WechatLiveSyncLog}`。
//!
//! **外部依赖（未迁移）**：Java 的 `WechatApiClient` / `WechatOAuthService`
//! （小程序 access_token、直播间创建/编辑/删除、推流地址、推广二维码、
//! 视频号直播记录）依赖微信开放平台 HTTP 接口与 `appid/secret` 配置；
//! Rust 侧未迁移该客户端，对应接口返回明确业务错误（见服务层）。

use serde::{Deserialize, Serialize};

use crate::types::PageQuery;

/// 直播状态文案（Java `WechatLiveStatus.getStatusName`）。
#[must_use]
pub fn live_status_text(status: Option<i32>) -> String {
    match status {
        Some(101) => "直播中",
        Some(102) => "未开始",
        Some(103) => "已结束",
        Some(104) => "禁播",
        Some(105) => "暂停",
        Some(106) => "异常",
        Some(107) => "已过期",
        _ => "",
    }
    .to_string()
}

/// 审核状态文案（Java `WechatLiveAuditStatus.getStatusName`）。
#[must_use]
pub fn audit_status_text(status: Option<i32>) -> String {
    match status {
        Some(0) => "未审核",
        Some(1) => "审核中",
        Some(2) => "审核通过",
        Some(3) => "审核驳回",
        _ => "",
    }
    .to_string()
}

/// `str2IntList`（Java `StringUtils.str2IntList`）：JSON 数组或逗号串 → `Vec<i32>`。
#[must_use]
pub fn parse_int_list(raw: &str) -> Vec<i32> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    if let Ok(v) = serde_json::from_str::<Vec<i32>>(raw) {
        return v;
    }
    raw.trim_matches(['[', ']'])
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect()
}

/// 秒级时间戳 → `yyyy-MM-dd HH:mm:ss`（Java `formatTimestamp`：0/负值返回空串）。
#[must_use]
pub fn format_timestamp(ts: Option<i32>) -> String {
    let Some(ts) = ts.filter(|v| *v > 0) else {
        return String::new();
    };
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(i64::from(ts), 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => String::new(),
    }
}

/// 日期时间串 → 秒级时间戳（Java `parseToTimestamp`：空白返回 0）。
#[must_use]
pub fn parse_timestamp(raw: Option<&str>) -> i32 {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return 0;
    };
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%d"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, fmt) {
            use chrono::TimeZone;
            if let Some(local) = chrono::Local.from_local_datetime(&dt).earliest() {
                return local.timestamp() as i32;
            }
        }
        if fmt == "%Y-%m-%d" {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(raw, fmt) {
                if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                    use chrono::TimeZone;
                    if let Some(local) = chrono::Local.from_local_datetime(&dt).earliest() {
                        return local.timestamp() as i32;
                    }
                }
            }
        }
    }
    0
}

/// 当前时间的 `yyyy-MM-dd HH:mm:ss`（Java `StringUtils.getCurrentTime`）。
#[must_use]
pub fn current_time_string() -> String {
    format_timestamp(Some(chrono::Local::now().timestamp() as i32))
}

/// 直播间列表查询（对应 Java `WechatLiveListDTO extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveListQuery {
    /// `#[serde(alias)]` 支持 H5 的 `pageNum` 形态（camelCase rename 后仍生效）。
    #[serde(alias = "pageNum")]
    pub page: Option<u64>,
    #[serde(rename = "pageSize", alias = "size")]
    pub page_size: Option<u64>,
    pub sort_field: Option<String>,
    pub sort_order: Option<String>,
    /// 通用关键词（匹配直播标题）
    pub keyword: Option<String>,
    /// 直播标题（匹配直播标题）
    pub wechat_live_title: Option<String>,
    /// 直播状态（101~107 才生效）
    pub live_status: Option<i32>,
    /// 创建日期起 `yyyy-MM-dd`
    pub start_time: Option<String>,
    /// 创建日期止 `yyyy-MM-dd`
    pub end_time: Option<String>,
}

impl WechatLiveListQuery {
    /// 分页（Java `BasePage`：缺省 15、上限 100）。
    #[must_use]
    pub fn page_query(&self) -> PageQuery {
        PageQuery {
            page: self.page.unwrap_or(1).max(1),
            page_size: self.page_size.unwrap_or(15).clamp(1, 100),
        }
    }

    /// 直播状态是否合法（Java `WechatLiveStatus.isValidCode`，101~107）。
    #[must_use]
    pub fn valid_live_status(&self) -> Option<i32> {
        self.live_status.filter(|s| (101..=107).contains(s))
    }
}

/// 直播间列表/详情 VO（对应 Java `WechatLiveVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveVO {
    pub wechat_live_id: Option<i32>,
    pub wechat_live_title: Option<String>,
    pub wechat_live_data: Option<String>,
    pub act_range: Option<i32>,
    pub act_range_ext: Option<Vec<i32>>,
    pub room_id: Option<i32>,
    pub share_img: Option<String>,
    pub live_type: Option<i32>,
    pub close_like: Option<i32>,
    pub close_goods: Option<i32>,
    pub close_comment: Option<i32>,
    pub close_kf: Option<i32>,
    pub close_replay: Option<i32>,
    pub is_feeds_public: Option<i32>,
    pub feeds_img: Option<String>,
    pub cover_img: Option<String>,
    pub anchor_id: Option<String>,
    pub anchor_openid: Option<String>,
    pub sub_anchor_wechat: Option<String>,
    pub anchor_name: Option<String>,
    pub anchor_img: Option<String>,
    pub creator_openid: Option<String>,
    pub live_status: Option<i32>,
    pub live_status_text: Option<String>,
    pub audit_status: Option<i32>,
    pub audit_status_text: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub last_update_time: Option<String>,
    pub product_data: Option<Vec<i32>>,
    pub thumb_img: Option<String>,
    pub live_sn: Option<String>,
    pub shop_id: Option<i32>,
    /// 关联订单数（`wechat_live_order_rel` 按 room_id 统计）
    pub order_count: Option<i64>,
    /// 同步失败数（`wechat_live_sync_log` 按 room_id + sync_status=2 统计）
    pub sync_failed_count: Option<i64>,
}

/// 更新直播间参数（对应 Java `WechatLiveUpdateDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveUpdateDTO {
    pub wechat_live_id: Option<i32>,
    pub wechat_live_title: Option<String>,
    pub act_range: Option<i32>,
    pub act_range_ext: Option<Vec<i32>>,
    pub is_feeds_public: Option<i32>,
    /// Java 兼容字段：非空时覆盖标题
    pub name: Option<String>,
    pub cover_img: Option<String>,
    pub share_img: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub anchor_name: Option<String>,
    pub anchor_img: Option<String>,
    pub live_type: Option<i32>,
    pub close_like: Option<i32>,
    pub close_goods: Option<i32>,
    pub close_comment: Option<i32>,
    pub close_kf: Option<i32>,
    pub close_replay: Option<i32>,
    pub feeds_img: Option<String>,
    pub anchor_id: Option<String>,
    pub anchor_openid: Option<String>,
    pub sub_anchor_wechat: Option<String>,
}

/// 创建直播间参数（对应 Java `WechatLiveRoomCreateDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveRoomCreateDTO {
    pub name: Option<String>,
    pub cover_img: Option<String>,
    pub share_img: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub anchor_name: Option<String>,
    pub anchor_img: Option<String>,
    pub live_type: Option<i32>,
    pub close_like: Option<i32>,
    pub close_goods: Option<i32>,
    pub close_comment: Option<i32>,
    pub close_kf: Option<i32>,
    pub close_replay: Option<i32>,
    pub is_feeds_public: Option<i32>,
    pub feeds_img: Option<String>,
    pub anchor_id: Option<String>,
    pub anchor_openid: Option<String>,
    pub sub_anchor_wechat: Option<String>,
}

/// 删除直播间参数（对应 Java `OperateDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveDeleteParam {
    pub id: Option<i32>,
}

/// 添加直播商品到直播间（对应 Java `WechatLiveGoodsAddDTO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveGoodsAddDTO {
    pub room_id: Option<i32>,
    pub goods_ids: Option<Vec<i64>>,
}

/// 移除直播间商品查询参数（`?goodsId=&roomId=`，`goodsId` 为 i64）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveRemoveGoodsQuery {
    pub goods_id: Option<i64>,
    pub room_id: Option<i32>,
}

/// 推流地址（对应 Java `WechatLivePushUrlVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLivePushUrlVO {
    pub room_id: Option<i32>,
    pub push_url: Option<String>,
    pub live_code: Option<String>,
    pub room_name: Option<String>,
}

/// 直播间推广信息（对应 Java `WechatLivePromoteVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLivePromoteVO {
    pub room_id: Option<i32>,
    pub room_name: Option<String>,
    pub cover_img: Option<String>,
    pub page_path: Option<String>,
    pub qrcode_url: Option<String>,
    pub poster_url: Option<String>,
    pub wechat_short_link: Option<String>,
}

/// 直播间商品（对应 Java `WechatLiveGoodsVO`）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WechatLiveGoodsVO {
    pub id: Option<i64>,
    pub room_goods_id: Option<i64>,
    pub room_id: Option<i32>,
    pub wechat_goods_id: Option<i32>,
    pub product_id: Option<i64>,
    pub product_sku_id: Option<i64>,
    pub sku_data: Option<String>,
    pub goods_name: Option<String>,
    pub cover_img: Option<String>,
    /// 商品价格（分）
    pub price: Option<i64>,
    pub stock: Option<i32>,
    pub price_type: Option<i32>,
    pub url: Option<String>,
    pub audit_status: Option<i32>,
    pub sort: Option<i32>,
    pub is_show: Option<i32>,
    /// 是否已加入直播间（列表查询用，本切片恒 1）
    pub is_joined_room: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_texts_match_java_enums() {
        assert_eq!(live_status_text(Some(101)), "直播中");
        assert_eq!(live_status_text(Some(102)), "未开始");
        assert_eq!(live_status_text(Some(107)), "已过期");
        assert_eq!(live_status_text(None), "");
        assert_eq!(live_status_text(Some(999)), "");
        assert_eq!(audit_status_text(Some(0)), "未审核");
        assert_eq!(audit_status_text(Some(2)), "审核通过");
        assert_eq!(audit_status_text(Some(3)), "审核驳回");
        assert_eq!(audit_status_text(None), "");
    }

    #[test]
    fn int_list_parsing_supports_json_and_csv() {
        assert_eq!(parse_int_list(""), Vec::<i32>::new());
        assert_eq!(parse_int_list("[1,2,3]"), vec![1, 2, 3]);
        assert_eq!(parse_int_list("1, 2 ,3"), vec![1, 2, 3]);
        // 非法项忽略（Java 抛异常 → 空列表；这里更宽容）
        assert_eq!(parse_int_list("[1,x,2]"), vec![1, 2]);
    }

    #[test]
    fn timestamp_parse_and_format_roundtrip() {
        assert_eq!(format_timestamp(None), "");
        assert_eq!(format_timestamp(Some(0)), "");
        let ts = parse_timestamp(Some("2023-11-15 06:13:20"));
        assert!(ts > 0);
        assert_eq!(format_timestamp(Some(ts)), "2023-11-15 06:13:20");
        assert_eq!(parse_timestamp(None), 0);
        assert_eq!(parse_timestamp(Some("  ")), 0);
    }

    #[test]
    fn list_query_validation_and_paging() {
        let q = WechatLiveListQuery::default();
        let page = q.page_query();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 15);
        // 非法状态（101~107 之外）不过滤
        assert_eq!(
            WechatLiveListQuery {
                live_status: Some(100),
                ..Default::default()
            }
            .valid_live_status(),
            None
        );
        assert_eq!(
            WechatLiveListQuery {
                live_status: Some(101),
                ..Default::default()
            }
            .valid_live_status(),
            Some(101)
        );
        let q = WechatLiveListQuery {
            page: Some(0),
            page_size: Some(999),
            ..Default::default()
        };
        let page = q.page_query();
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 100);
    }
}
