//! 平台与门店相册互通（content::gallery_share）DTO / VO / 纯逻辑（我方新增）。
//!
//! 论坛 B 类缓做项 #35（帖 687807 余子项）：平台素材中心已有、门店相册与
//! 平台素材的互通/下发此前未实现。本模块承载其中「门店素材上报平台审核」
//! 一侧的契约；「平台下发」一侧走 `gallery.is_shared` / `gallery_video.is_shared`
//! 标记（见 [`crate::models::content::gallery`] 的 `shared_subtree_ids`）。
//!
//! # 口径（与 REPORT 第二节一致）
//! - 上报：门店把 **本店** 素材（图片 `gallery_pic` / 视频 `gallery_video_info`）
//!   提交平台审核；同一素材同时只允许一条待审核记录（DB 部分唯一索引兜底）。
//! - 审核：仅平台账号可审；通过时把素材 **复制** 入平台库（新行 `shop_id = 0`，
//!   共用同一存储文件、不重新上传），原门店素材不动；驳回须可填备注。
//! - 状态：0 待审核 / 1 已通过 / 2 已驳回；已审记录不可重复审核。

use serde::{Deserialize, Serialize};

/// 媒体类型：图片。
pub const MEDIA_IMAGE: &str = "image";
/// 媒体类型：视频。
pub const MEDIA_VIDEO: &str = "video";

/// 审核状态：待审核。
pub const STATUS_PENDING: i32 = 0;
/// 审核状态：已通过。
pub const STATUS_APPROVED: i32 = 1;
/// 审核状态：已驳回。
pub const STATUS_REJECTED: i32 = 2;

/// 审核动作：通过。
pub const ACTION_APPROVE: &str = "approve";
/// 审核动作：驳回。
pub const ACTION_REJECT: &str = "reject";

/// 上报时单次提交的素材数量上限（防滥用，与批量操作同量级）。
pub const SUBMIT_MAX_ITEMS: usize = 100;

/// 文案：媒体类型非法。
pub const ERR_MEDIA_TYPE_INVALID: &str = "素材类型有误";
/// 文案：未选择素材。
pub const ERR_IDS_REQUIRED: &str = "未选择素材";
/// 文案：提交数量超限。
pub const ERR_TOO_MANY_ITEMS: &str = "单次上报素材数量超限";
/// 文案：平台账号不能上报（平台素材直接归平台库）。
pub const ERR_PLATFORM_CANNOT_SUBMIT: &str = "平台账号无需上报素材";
/// 文案：非门店账号（供应商/未判定）不能上报。
pub const ERR_SUBMIT_SHOP_ONLY: &str = "仅门店账号可上报素材";
/// 文案：素材不存在。
pub const ERR_ITEM_NOT_FOUND: &str = "素材不存在";
/// 文案：只能上报本店素材。
pub const ERR_NOT_OWN_ITEM: &str = "只能上报本店素材";
/// 文案：该素材已在审核中。
pub const ERR_ALREADY_PENDING: &str = "该素材已在审核中，请勿重复上报";
/// 文案：上报记录不存在。
pub const ERR_SUBMIT_NOT_FOUND: &str = "上报记录不存在";
/// 文案：记录已审核。
pub const ERR_ALREADY_REVIEWED: &str = "该记录已审核，不能重复审核";
/// 文案：仅平台账号可审核。
pub const ERR_REVIEW_PLATFORM_ONLY: &str = "仅平台账号可审核上报素材";
/// 文案：审核动作非法。
pub const ERR_ACTION_INVALID: &str = "审核动作有误";
/// 文案：目标分组必须是平台分组。
pub const ERR_TARGET_NOT_PLATFORM: &str = "目标分组必须是平台素材分组";
/// 文案：记录 id 缺失。
pub const ERR_ID_REQUIRED: &str = "id不能为空";

/// 上报入参（门店账号）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryShareSubmitDTO {
    /// 媒体类型：`image` / `video`。
    pub r#type: Option<String>,
    /// 素材 id 列表（图片 = `gallery_pic.pic_id`；视频 = `gallery_video_info.id`）。
    pub ids: Vec<i32>,
}

/// 审核入参（平台账号）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryShareReviewDTO {
    /// 上报记录 id。
    pub id: Option<i32>,
    /// 动作：`approve` / `reject`。
    pub action: Option<String>,
    /// 通过时落入的平台分组 id（缺省 0 = 未分组）。
    pub target_gallery_id: Option<i32>,
    /// 审核备注（驳回原因等）。
    pub remark: Option<String>,
}

/// 上报列表查询（门店看自己 / 平台看全部，均可按状态过滤）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryShareListQuery {
    /// 页码（缺省 1）。
    pub page: Option<u64>,
    /// 页大小（缺省 20，上限 100）。
    pub size: Option<u64>,
    /// 状态过滤：0/1/2；缺省全部。
    pub status: Option<i32>,
}

impl GalleryShareListQuery {
    /// 归一化分页 `(page, page_size)`。
    #[must_use]
    pub fn page_params(&self) -> (u64, u64) {
        let page = self.page.unwrap_or(1).max(1);
        let size = self.size.unwrap_or(20).clamp(1, 100);
        (page, size)
    }
}

/// 上报记录 VO（出参）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryShareSubmitVO {
    /// 记录 id。
    pub id: i32,
    /// 上报门店 id。
    pub shop_id: i32,
    /// 媒体类型。
    pub media_type: String,
    /// 来源分组 id。
    pub source_gallery_id: i32,
    /// 来源素材 id。
    pub source_item_id: i32,
    /// 素材名（快照）。
    pub item_name: String,
    /// 素材地址（快照）。
    pub item_url: String,
    /// 缩略图/封面（快照）。
    pub item_thumb: String,
    /// 状态：0 待审核 / 1 已通过 / 2 已驳回。
    pub status: i32,
    /// 状态文案。
    pub status_name: String,
    /// 提交时间（秒级时间戳）。
    pub submit_time: i32,
    /// 审核时间（秒级时间戳，未审为 0）。
    pub review_time: i32,
    /// 审核备注。
    pub review_remark: String,
    /// 通过时落入的平台分组 id。
    pub target_gallery_id: i32,
    /// 通过后生成的平台素材 id。
    pub result_item_id: i32,
}

/// 提交结果 VO（逐条校验后汇总）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryShareSubmitResultVO {
    /// 成功提交条数。
    pub submitted: i32,
    /// 跳过条数（已在审核中）。
    pub skipped: i32,
}

/// 状态文案。
#[must_use]
pub fn status_name(status: i32) -> &'static str {
    match status {
        STATUS_PENDING => "待审核",
        STATUS_APPROVED => "已通过",
        STATUS_REJECTED => "已驳回",
        _ => "未知",
    }
}

/// 媒体类型是否合法。
#[must_use]
pub fn is_valid_media_type(media_type: Option<&str>) -> bool {
    matches!(media_type, Some(MEDIA_IMAGE) | Some(MEDIA_VIDEO))
}

/// 校验上报入参，返回归一后的媒体类型。
///
/// # Errors
/// 类型非法 → [`ERR_MEDIA_TYPE_INVALID`]；未选素材 → [`ERR_IDS_REQUIRED`]；
/// 超限 → [`ERR_TOO_MANY_ITEMS`]。
pub fn validate_submit(dto: &GalleryShareSubmitDTO) -> Result<&'static str, String> {
    let media_type = match dto.r#type.as_deref().map(str::trim) {
        Some(MEDIA_IMAGE) => MEDIA_IMAGE,
        Some(MEDIA_VIDEO) => MEDIA_VIDEO,
        _ => return Err(ERR_MEDIA_TYPE_INVALID.to_string()),
    };
    if dto.ids.is_empty() {
        return Err(ERR_IDS_REQUIRED.to_string());
    }
    if dto.ids.len() > SUBMIT_MAX_ITEMS {
        return Err(ERR_TOO_MANY_ITEMS.to_string());
    }
    Ok(media_type)
}

/// 校验审核入参，返回 `(记录 id, 是否通过, 目标分组, 备注)`。
///
/// # Errors
/// id 缺失 → [`ERR_ID_REQUIRED`]；动作非法 → [`ERR_ACTION_INVALID`]。
pub fn validate_review(dto: &GalleryShareReviewDTO) -> Result<(i32, bool, i32, String), String> {
    let id = dto.id.ok_or_else(|| ERR_ID_REQUIRED.to_string())?;
    let approve = match dto.action.as_deref().map(str::trim) {
        Some(ACTION_APPROVE) => true,
        Some(ACTION_REJECT) => false,
        _ => return Err(ERR_ACTION_INVALID.to_string()),
    };
    let remark = dto
        .remark
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .chars()
        .take(500)
        .collect();
    Ok((id, approve, dto.target_gallery_id.unwrap_or(0), remark))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_validation() {
        let dto = GalleryShareSubmitDTO { r#type: Some("image".into()), ids: vec![1] };
        assert_eq!(validate_submit(&dto).unwrap(), MEDIA_IMAGE);
        let dto = GalleryShareSubmitDTO { r#type: Some("video".into()), ids: vec![1, 2] };
        assert_eq!(validate_submit(&dto).unwrap(), MEDIA_VIDEO);
        let dto = GalleryShareSubmitDTO { r#type: Some("audio".into()), ids: vec![1] };
        assert_eq!(validate_submit(&dto).unwrap_err(), ERR_MEDIA_TYPE_INVALID);
        let dto = GalleryShareSubmitDTO { r#type: None, ids: vec![1] };
        assert_eq!(validate_submit(&dto).unwrap_err(), ERR_MEDIA_TYPE_INVALID);
        let dto = GalleryShareSubmitDTO { r#type: Some("image".into()), ids: vec![] };
        assert_eq!(validate_submit(&dto).unwrap_err(), ERR_IDS_REQUIRED);
        let dto = GalleryShareSubmitDTO {
            r#type: Some("image".into()),
            ids: (0..(SUBMIT_MAX_ITEMS as i32 + 1)).collect(),
        };
        assert_eq!(validate_submit(&dto).unwrap_err(), ERR_TOO_MANY_ITEMS);
    }

    #[test]
    fn review_validation() {
        let dto = GalleryShareReviewDTO {
            id: Some(9),
            action: Some("approve".into()),
            target_gallery_id: Some(4),
            remark: Some("  好  ".into()),
        };
        assert_eq!(validate_review(&dto).unwrap(), (9, true, 4, "好".to_string()));
        let dto = GalleryShareReviewDTO { id: Some(9), action: Some("reject".into()), ..Default::default() };
        assert_eq!(validate_review(&dto).unwrap(), (9, false, 0, String::new()));
        let dto = GalleryShareReviewDTO { id: None, action: Some("approve".into()), ..Default::default() };
        assert_eq!(validate_review(&dto).unwrap_err(), ERR_ID_REQUIRED);
        let dto = GalleryShareReviewDTO { id: Some(1), action: Some("pass".into()), ..Default::default() };
        assert_eq!(validate_review(&dto).unwrap_err(), ERR_ACTION_INVALID);
        // 备注截断 500 字
        let dto = GalleryShareReviewDTO {
            id: Some(1),
            action: Some("reject".into()),
            remark: Some("字".repeat(600)),
            ..Default::default()
        };
        assert_eq!(validate_review(&dto).unwrap().3.chars().count(), 500);
    }

    #[test]
    fn status_names_and_page_params() {
        assert_eq!(status_name(0), "待审核");
        assert_eq!(status_name(1), "已通过");
        assert_eq!(status_name(2), "已驳回");
        assert_eq!(status_name(9), "未知");
        let q = GalleryShareListQuery::default();
        assert_eq!(q.page_params(), (1, 20));
        let q = GalleryShareListQuery { page: Some(0), size: Some(500), status: None };
        assert_eq!(q.page_params(), (1, 100));
        assert!(is_valid_media_type(Some("image")));
        assert!(!is_valid_media_type(Some("doc")));
        assert!(!is_valid_media_type(None));
    }

    #[test]
    fn vo_serializes_camel_case() {
        let vo = GalleryShareSubmitVO {
            id: 1,
            shop_id: 8,
            media_type: "image".into(),
            source_item_id: 3,
            status: STATUS_PENDING,
            status_name: status_name(STATUS_PENDING).into(),
            ..Default::default()
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["mediaType"], "image");
        assert_eq!(json["sourceItemId"], 3);
        assert_eq!(json["statusName"], "待审核");
        assert_eq!(json["resultItemId"], 0);
    }
}
