//! 微信视频号直播记录域（content::api_wechat_live）DTO / 纯函数。
//!
//! 对应原 Java `ApiWechatLiveController#getFinderLiveRecord`
//! （`GET /api/content/wechatVideoAccount/finderLiveRecord?id=`，契约
//! auth=optional）与管理端 `WechatLiveController#getFinderLiveRecord`
//! （`GET /adminapi/content/wechatLive/finderLiveRecord?id=`）。
//!
//! 数据来自微信「视频号助手」开放接口
//! `POST /channels/ec/finderlive/getfinderliverecordlist`（微信官方文档
//! 口径：无请求体，返回 `live_list[{export_id, description, cover_url,
//! nickname, head_url}]`），经**直播小程序**凭据
//! （`wechatLiveMiniAppId`/`wechatLiveMiniAppSecret`，与 Java
//! `WechatOAuthServiceImpl#getLiveMiniAccessToken` 同口径）的
//! access_token 调用（凭据与 token 底座见 services 层 `wechat_finder`）。
//! 拉取结果落库 `finder_live_record`（迁移 028，我方新增——原版不落库）。
//!
//! 选取口径（**原版有·我方改口径**）：Java 在 live_list 中按
//! `export_id == id` 精确匹配、未命中返回 null；但前台装修模块传入的
//! id 实为视频号 ID（finderId，编辑器选项值），与 export_id 恒不相等，
//! Java 口径下直播描述恒为空。本实现保留精确匹配优先、未命中时回退
//! 当前列表首条；列表为空（未在直播）返回 None（与 Java 返回 null 同义）。

use serde::{Deserialize, Serialize};

/// 直播记录查询参数（Java `@RequestParam("id")`；前台传视频号 ID，
/// 管理端同名参数，均为字符串）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FinderLiveRecordQuery {
    /// 直播记录 ID（export_id）或视频号 ID（finderId，见模块头选取口径）。
    pub id: Option<String>,
}

/// 落库记录分页查询（管理端落库查询，我方新增，无 Java 对应）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FinderLiveRecordListQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    /// 仅当前直播（最近一次拉取仍在当前列表中）。
    pub only_current: Option<bool>,
}

/// 直播记录 VO（微信官方 `live_list` 单条 / Java `LiveRecord` 字段逐字对齐）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FinderLiveRecordVO {
    /// 直播 id（export_id）。
    pub export_id: Option<String>,
    /// 直播描述。
    pub description: Option<String>,
    /// 直播封面（cover_url）。
    pub cover_url: Option<String>,
    /// 开播视频号昵称。
    pub nickname: Option<String>,
    /// 开播视频号头像（head_url）。
    pub head_url: Option<String>,
}

/// 分页归一（Java `BasePage`：缺省 1/15，上限 100）。
#[must_use]
pub fn page_params(page: Option<u64>, size: Option<u64>) -> (u64, u64) {
    (page.unwrap_or(1).max(1), size.unwrap_or(15).clamp(1, 100))
}

/// 从当前直播列表中选取一条（纯函数）：
/// 1. `id` 与某条 `export_id` 精确相等 → 该条（Java 原口径）；
/// 2. 否则回退首条（我方改口径，见模块头）；
/// 3. 列表为空 → None。
#[must_use]
pub fn select_record<'a>(
    records: &'a [FinderLiveRecordVO],
    id: Option<&str>,
) -> Option<&'a FinderLiveRecordVO> {
    if let Some(want) = id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(hit) = records
            .iter()
            .find(|r| r.export_id.as_deref() == Some(want))
        {
            return Some(hit);
        }
    }
    records.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(export_id: &str, nickname: &str) -> FinderLiveRecordVO {
        FinderLiveRecordVO {
            export_id: Some(export_id.into()),
            nickname: Some(nickname.into()),
            ..Default::default()
        }
    }

    #[test]
    fn page_params_match_base_page() {
        assert_eq!(page_params(None, None), (1, 15));
        assert_eq!(page_params(Some(3), Some(500)), (3, 100));
    }

    #[test]
    fn select_prefers_exact_export_id_match() {
        let records = vec![rec("exp-1", "甲"), rec("exp-2", "乙")];
        let hit = select_record(&records, Some("exp-2")).unwrap();
        assert_eq!(hit.nickname.as_deref(), Some("乙"));
    }

    #[test]
    fn select_falls_back_to_first_when_id_is_finder_id() {
        // 前台传 finderId（与 export_id 恒不相等）→ 回退首条（我方改口径）
        let records = vec![rec("exp-1", "甲"), rec("exp-2", "乙")];
        let hit = select_record(&records, Some("sph_finder_123")).unwrap();
        assert_eq!(hit.nickname.as_deref(), Some("甲"));
        // id 缺省 / 空白同样回退首条
        assert_eq!(
            select_record(&records, None).unwrap().nickname.as_deref(),
            Some("甲")
        );
        assert_eq!(
            select_record(&records, Some("  "))
                .unwrap()
                .nickname
                .as_deref(),
            Some("甲")
        );
    }

    #[test]
    fn select_empty_list_is_none() {
        assert!(select_record(&[], Some("exp-1")).is_none());
        assert!(select_record(&[], None).is_none());
    }

    #[test]
    fn query_accepts_java_id_param() {
        let q: FinderLiveRecordQuery = serde_json::from_str(r#"{"id":"sph123"}"#).unwrap();
        assert_eq!(q.id.as_deref(), Some("sph123"));
        let empty: FinderLiveRecordQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert!(empty.id.is_none());
    }

    #[test]
    fn vo_serializes_camel_case_like_java_live_record() {
        let vo = FinderLiveRecordVO {
            export_id: Some("exp-1".into()),
            description: Some("描述".into()),
            cover_url: Some("https://cover".into()),
            nickname: Some("昵称".into()),
            head_url: Some("https://head".into()),
        };
        let v = serde_json::to_value(&vo).unwrap();
        assert_eq!(v["exportId"], "exp-1");
        assert_eq!(v["description"], "描述");
        assert_eq!(v["coverUrl"], "https://cover");
        assert_eq!(v["nickname"], "昵称");
        assert_eq!(v["headUrl"], "https://head");
    }
}
