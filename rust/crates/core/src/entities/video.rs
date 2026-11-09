//! 视频域实体
//!
//! **本文件由 `tools/schema_parser.py` 从 `java/database/pure.sql` +
//! 全部 `*update.sql` 增量补丁自动生成，请勿手工编辑。**
//!
//! 重新生成: `python tools/schema_parser.py --gen-entities`
//!
//! 对应数据库表: `gallery_video`, `gallery_video_info`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 视频相册文件夹
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GalleryVideo {
    /// 主键id [NOT NULL]
    pub id: i32,
    /// 商户id [NOT NULL]
    pub shop_id: i32,
    /// 父级id [NOT NULL]
    pub parent_id: i32,
    /// 名称 [NOT NULL]
    pub name: String,
    /// 排序 [NOT NULL]
    pub sort: i32,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 创建人id [NOT NULL]
    pub add_user_id: i32,
    /// 供应商id [NOT NULL]
    pub vendor_id: i32,
}

impl GalleryVideo {
    /// 数据库表名。
    pub const TABLE: &'static str = "gallery_video";
}


/// 视频相册
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
#[sqlx(rename_all = "snake_case")]
pub struct GalleryVideoInfo {
    /// 主键id [NOT NULL]
    pub id: i32,
    /// 商户id [NOT NULL]
    pub shop_id: i32,
    /// 相册id [NOT NULL]
    pub gallery_id: i32,
    /// 视频地址 [NOT NULL]
    pub video_url: String,
    /// 视频名称 [NOT NULL]
    pub video_name: String,
    /// 视频封面
    pub video_cover: Option<String>,
    /// 添加时间 [NOT NULL]
    pub add_time: i32,
    /// 创建人id [NOT NULL]
    pub add_user_id: i32,
    /// 视频格式后缀
    pub format: Option<String>,
    /// 视频封面（第一帧）
    pub video_first_frame: Option<String>,
    /// 时长
    pub duration: Option<String>,
    /// 大小
    pub size: Option<String>,
    /// 供应商id [NOT NULL]
    pub vendor_id: i32,
    /// 来源：upload=本地上传, external=外部直链 [NOT NULL]
    pub source: String,
    /// 视频简介
    pub description: Option<String>,
}

impl GalleryVideoInfo {
    /// 数据库表名。
    pub const TABLE: &'static str = "gallery_video_info";
}