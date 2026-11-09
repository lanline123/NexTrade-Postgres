//! 静态资源文件域服务（system::files）。
//!
//! 对应原 Java `com.nextrade.adminapi.controller.setting.FilesController`
//! （三个方法都转发到 `GalleryPicServiceImpl#selectFile(timestamp, filename, fileType)`）。
//!
//! | 本服务 | 原 Java |
//! |--------|---------|
//! | [`FilesService::open`] | `GalleryPicServiceImpl#selectFile` |
//!
//! # 路径口径（照抄 Java）
//!
//! ```java
//! File pemDir = FileUtil.mkdir(jarDir + File.separator + uploadDir);   // file.upload-dir
//! uploadPath = "{absolutePath}/{fileType}/gallery/{timestamp}"          // img / video
//! uploadPath = "{absolutePath}/{fileType}/update/{timestamp}"           // app
//! filePath   = Paths.get(uploadPath, filename)
//! ```
//!
//! # 与 Java 的差异（逐条）
//!
//! 1. **根目录来源**：Java 取「JAR 所在目录」拼接 `${file.upload-dir}`；Rust 侧
//!    没有 JAR 概念，改为环境变量 `TIG_FILE_UPLOAD_DIR`（未设置时用
//!    `nextrade_common::config` 的同款默认值 `./uploads`，即进程工作目录下的
//!    `uploads/`）。与 `shop_manage` 读 `TIG_IS_O2O` 的做法一致（配置服务未迁移）。
//! 2. **目录穿越防护（安全加固，与原系统行为不同但必要）**：Java 直接把
//!    路径变量拼进 `Paths.get`，未做任何校验；Rust 侧对 `timestamp` / `filename`
//!    两段做白名单校验（不得含 `/`、`\`、`..`、`\0`，且 `filename` 不得为绝对路径），
//!    非法入参统一按「文件不存在」返回，避免通过错误信息探测目录结构。
//! 3. **不存在时的语义**：Java `urlResource.exists()==false` →
//!    `GlobalException(FILE_NOT_EXIST)`（`文件不存在`）；本实现返回同一文案的
//!    业务错误。Java 读流失败（`MalformedURLException`）→ 空 404；Rust 侧
//!    路径由 `Paths` 构造不可能畸形，故不复刻该分支。
//! 4. **不实现 `mkdir`**：Java 每次请求都会先 `FileUtil.mkdir(uploadDir)`；
//!    Rust 侧只读不建目录（读不存在的目录本就返回「文件不存在」）。

use std::path::PathBuf;

use nextrade_core::{Error, Result};

/// Java `selectFile` 的 `fileType` 参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryFileKind {
    /// `/img/gallery/{timestamp}/{filename}`
    Img,
    /// `/video/gallery/{timestamp}/{filename}`
    Video,
    /// `/app/update/{timestamp}/{filename}`
    App,
}

impl GalleryFileKind {
    /// Java 里拼接的第一级目录名。
    #[must_use]
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Img => "img",
            Self::Video => "video",
            Self::App => "app",
        }
    }

    /// Java 里拼接的第二级目录名（`app` 用 `update`，其余用 `gallery`）。
    #[must_use]
    pub fn sub_dir(self) -> &'static str {
        match self {
            Self::App => "update",
            _ => "gallery",
        }
    }
}

/// 打开成功后的文件句柄与元信息。
pub struct OpenedFile {
    pub file: tokio::fs::File,
    /// 原始文件名（用于 `Content-Disposition`）。
    pub filename: String,
    pub size: u64,
}

/// Java `FILE_NOT_EXIST`（`GalleryPicConstants`）。
const FILE_NOT_EXIST: &str = "文件不存在";

/// 文件根目录（差异见模块头第 1 条）。
fn upload_root() -> PathBuf {
    PathBuf::from(
        std::env::var("TIG_FILE_UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
    )
}

/// 路径片段安全校验（差异见模块头第 2 条）。
///
/// 允许普通文件名（含点、中文），拒绝一切可能改变目录层级的输入。
#[must_use]
pub fn is_safe_segment(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('/')
        && !s.contains('\\')
        && !s.contains('\0')
        && s != "."
        && s != ".."
        && !s.starts_with('.')
}

/// 静态资源文件服务。
pub struct FilesService {
    _repos: std::sync::Arc<nextrade_infrastructure::db::Repositories>,
}

impl FilesService {
    #[must_use]
    pub fn new(repos: std::sync::Arc<nextrade_infrastructure::db::Repositories>) -> Self {
        Self { _repos: repos }
    }

    /// Java `selectFile(timestamp, filename, fileType)`：定位并打开文件。
    pub async fn open(
        &self,
        kind: GalleryFileKind,
        timestamp: &str,
        filename: &str,
    ) -> Result<OpenedFile> {
        if !is_safe_segment(timestamp) || !is_safe_segment(filename) {
            return Err(Error::biz_msg(FILE_NOT_EXIST));
        }

        let path = upload_root()
            .join(kind.dir_name())
            .join(kind.sub_dir())
            .join(timestamp)
            .join(filename);

        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|_| Error::biz_msg(FILE_NOT_EXIST))?;
        let meta = file.metadata().await.map_err(|_| Error::biz_msg(FILE_NOT_EXIST))?;
        if !meta.is_file() {
            return Err(Error::biz_msg(FILE_NOT_EXIST));
        }

        Ok(OpenedFile {
            file,
            filename: filename.to_string(),
            size: meta.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_dirs_match_java() {
        assert_eq!(GalleryFileKind::Img.dir_name(), "img");
        assert_eq!(GalleryFileKind::Img.sub_dir(), "gallery");
        assert_eq!(GalleryFileKind::Video.sub_dir(), "gallery");
        assert_eq!(GalleryFileKind::App.dir_name(), "app");
        assert_eq!(GalleryFileKind::App.sub_dir(), "update");
    }

    #[test]
    fn traversal_inputs_are_rejected() {
        assert!(!is_safe_segment(".."));
        assert!(!is_safe_segment("../etc/passwd"));
        assert!(!is_safe_segment("a\\b"));
        assert!(!is_safe_segment("a/b"));
        assert!(!is_safe_segment(""));
        assert!(!is_safe_segment(".hidden"));
    }

    #[test]
    fn normal_inputs_are_accepted() {
        assert!(is_safe_segment("1701234567"));
        assert!(is_safe_segment("a.jpg"));
        assert!(is_safe_segment("商品主图.png"));
        assert!(is_safe_segment("app-v1.2.3.apk"));
    }
}