//! 相册图片域（content::gallery_pic）DTO / VO / 纯函数。
//!
//! 对应原 Java：
//! - `com.nextrade.adminapi.controller.setting.GalleryPicController`
//!   （`@RequestMapping("/adminapi/setting/galleryPic")`，10 个端点）
//! - `com.nextrade.service.setting.impl.GalleryPicServiceImpl`
//! - 入参 `GalleryPicListDTO` / `GalleryPicSortDTO` / `GalleryPicTransferDTO` /
//!   `GalleryPicBatchDTO` / `GalleryPicExternalDTO` / `UpdateFieldDTO` / `OperateDTO`
//! - 出参 `GalleryListResVO` / `GalleryPicUploadVO` / `GalleryPicDTO` / `GalleryPicResDTO`
//!
//! # 口径说明
//!
//! - 列表页尺寸取 Java `BasePage`：缺省 15、上限 100（**不是** gallery 域的固定 99）。
//! - 多租户 scope 与 gallery 域一致（`x-admin-type` / `x-shop-id` / `x-vendor-id`）。
//! - `GalleryPicDTO.shopId` 在 Java 里是 `String`，而实体是 `Integer`，
//!   `BeanUtils.copyProperties` 因类型不兼容**不会复制**，故该字段恒为 `null`；
//!   本实现保留 `Option<String>` 并同样置空（见 `uncertain_points`）。
//! - `uploadImg` / `batchDownload` 依赖对象存储 / zip 打包（未迁移）：
//!   本域完成本地校验后返回明确的「未迁移」业务错误，不伪造成功。

use serde::{Deserialize, Serialize};

use crate::models::content::gallery::GalleryDTO;
use crate::types::PageQuery;

// ——————————————————————————————————————————————
// 常量（文案逐字对齐 Java）
// ——————————————————————————————————————————————

/// Java `GalleryPicConstants.FILE_NOT_EXIST`。
pub const FILE_NOT_EXIST: &str = "文件不存在";
/// Java `GalleryConstants.GALLERY_ID_IS_NULL`。
pub const GALLERY_ID_IS_NULL: &str = "相册不存在";
/// `id不能为空`
pub const ERR_ID_REQUIRED: &str = "id不能为空";
/// `未选择项目`
pub const ERR_UNSELECTED_ITEM: &str = "未选择项目";
/// `分组ID不能为空`（排序）
pub const ERR_GALLERY_ID_REQUIRED: &str = "分组ID不能为空";
/// `排序ID列表不能为空`
pub const ERR_SORT_IDS_REQUIRED: &str = "排序ID列表不能为空";
/// `素材ID列表不能为空`（移动 / 批量删除 / 批量下载）
pub const ERR_IDS_REQUIRED: &str = "素材ID列表不能为空";
/// `目标分组ID不能为空`
pub const ERR_TARGET_ID_REQUIRED: &str = "目标分组ID不能为空";
/// `类型不能为空`
pub const ERR_TYPE_REQUIRED: &str = "类型不能为空";
/// `图片地址不能为空`
pub const ERR_PIC_URL_REQUIRED: &str = "图片地址不能为空";
/// `来源不能为空`
pub const ERR_SOURCE_REQUIRED: &str = "来源不能为空";
/// `URL不能为空`
pub const ERR_URL_EMPTY: &str = "URL不能为空";
/// `URL必须以http://或https://开头`
pub const ERR_URL_SCHEME: &str = "URL必须以http://或https://开头";
/// `不支持网页搜索结果地址，请提供图片/视频直链`
pub const ERR_URL_BLOCKED: &str = "不支持网页搜索结果地址，请提供图片/视频直链";
/// `文件不能为空`
pub const ERR_UPLOAD_FILE_EMPTY: &str = "文件不能为空";
/// `不支持的文件类型`
pub const ERR_UPLOAD_FILE_TYPE: &str = "不支持的文件类型";
/// SVG 不合法或含不安全内容（本平台扩展：SVG 净化失败统一文案）。
pub const ERR_SVG_INVALID: &str = "SVG 文件不合法或包含不安全内容，已拒绝上传";
/// `没有可下载的素材`
pub const ERR_NO_DOWNLOADABLE: &str = "没有可下载的素材";
/// `field 的值有误`
pub const ERR_FIELD_INVALID: &str = "field 的值有误";
/// 图片上传「未迁移」业务错误（不伪造成功）。
pub const ERR_UPLOAD_NOT_MIGRATED: &str =
    "图片上传依赖对象存储服务（OSS/COS/七牛/S3/本地存储）与图片尺寸解析，当前 Rust 侧尚未迁移，本接口暂不可用";
/// 批量下载「未迁移」业务错误（不伪造成功）。
pub const ERR_DOWNLOAD_NOT_MIGRATED: &str =
    "批量下载依赖 zip 打包与对象存储下载，当前 Rust 侧尚未迁移，本接口暂不可用";

/// 允许的图片 MIME（Java `Constants.ALLOWED_IMAGE_TYPES`；本平台扩展：增加
/// `image/svg+xml`，使本地上传的 SVG 图标可通过类型校验——见购买按钮自定义图标需求）。
pub const ALLOWED_IMAGE_TYPES: [&str; 6] = [
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/tiff",
    "image/svg+xml",
];
/// 外部直链允许的扩展名（Java `ALLOWED_IMAGE_EXTENSIONS`；本平台扩展：增加
/// `webp`、`svg`——Tigshop v5.8.65 素材中心 WebP/SVG 支持，帖 517949006945033228）。
pub const ALLOWED_IMAGE_EXTENSIONS: [&str; 7] = ["bmp", "jpg", "jpeg", "png", "gif", "webp", "svg"];
/// 禁止的网页搜索结果地址片段（Java `BLOCKED_URL_PATTERNS`）。
pub const BLOCKED_URL_PATTERNS: [&str; 5] = [
    "baidu.com/s?",
    "image.baidu.com/search",
    "google.com/search",
    "bing.com/images/search",
    "sogou.com/pics",
];
/// `updateField` 白名单（Java `CheckFieldConstants.GALLERY_FIELDS`）。
pub const GALLERY_FIELDS: [&str; 1] = ["pic_name"];
/// 配置键：图片上传最大尺寸 MB（Java `SettingsEnum.IMAGE_UPLOAD_MAX_SIZE`）。
pub const CONFIG_IMAGE_UPLOAD_MAX_SIZE: &str = "imageUploadMaxSize";
/// 图片上传最大尺寸默认值（Java 枚举 `defaultValue = "20"`）。
pub const DEFAULT_IMAGE_UPLOAD_MAX_SIZE: &str = "20";
/// 外部图片默认名称（Java `"外部图片"`）。
pub const DEFAULT_EXTERNAL_PIC_NAME: &str = "外部图片";
/// 相册图片列表固定返回的子相册图片预览数（Java `new Page<>(1, 4)`）。
pub const CHILD_PREVIEW_PIC_LIMIT: u64 = 4;

// ——————————————————————————————————————————————
// 纯函数
// ——————————————————————————————————————————————

/// Java `BasePage#getPage/getSize`：缺省 1 / 15，上限 100。
#[must_use]
pub fn page_params(page: Option<u64>, size: Option<u64>) -> (u64, u64) {
    (page.unwrap_or(1).max(1), size.unwrap_or(15).clamp(1, 100))
}

/// 由 page/size 构造 [`PageQuery`]。
#[must_use]
pub fn page_query(page: Option<u64>, size: Option<u64>) -> PageQuery {
    let (page, size) = page_params(page, size);
    PageQuery {
        page,
        page_size: size,
    }
}

/// 来源筛选归一化（Java `buildGalleryPicQueryWrapper`）：
/// `external` → `external`；`local`/`upload` → `upload`；其它/空 → 不过滤。
#[must_use]
pub fn normalize_source_filter(raw: Option<&str>) -> Option<&'static str> {
    let source = raw?.trim();
    if source.is_empty() {
        return None;
    }
    if source.eq_ignore_ascii_case("external") {
        Some("external")
    } else if source.eq_ignore_ascii_case("local") || source.eq_ignore_ascii_case("upload") {
        Some("upload")
    } else {
        None
    }
}

/// 列表排序（Java `GalleryPicServiceImpl#list`）：
/// - `sortOrder == "manual"` ⇒ `pic_sort ASC`；
/// - 否则 `sortField` 缺省 `addTime`，`sortOrder` 为 `desc` 时降序、其余升序。
///
/// 返回 `(列名, 是否降序)`；`sortOrder == "manual"` 时降序恒 false。
#[must_use]
pub fn resolve_sort(sort_field: Option<&str>, sort_order: Option<&str>) -> (&'static str, bool) {
    let order = sort_order.map(str::trim);
    if order == Some("manual") {
        return ("pic_sort", false);
    }
    let field = sort_field.map(str::trim).filter(|f| !f.is_empty());
    let column = match field {
        Some("addTime") | Some("add_time") => "add_time",
        Some("picName") | Some("pic_name") => "pic_name",
        Some("picSize") | Some("pic_size") => "pic_size",
        Some("picWidth") | Some("pic_width") => "pic_width",
        Some("picHeight") | Some("pic_height") => "pic_height",
        Some("picSort") | Some("pic_sort") => "pic_sort",
        Some("picId") | Some("pic_id") => "pic_id",
        Some("galleryId") | Some("gallery_id") => "gallery_id",
        Some("fileType") | Some("file_type") => "file_type",
        _ => "add_time",
    };
    (column, order == Some("desc"))
}

/// `updateField` 属性名/列名 → 列名（Java `getColumnByPropertyName` + 白名单）。
#[must_use]
pub fn resolve_gallery_pic_field(field: &str) -> Option<&'static str> {
    match field.trim() {
        "picName" | "pic_name" => Some("pic_name"),
        _ => None,
    }
}

/// 提取 URL 扩展名（Java `extractExtension`：去 query/hash，取最后一个 `.` 后小写）。
#[must_use]
pub fn extract_extension(url: &str) -> String {
    if url.trim().is_empty() {
        return String::new();
    }
    let mut path = url;
    if let Some(index) = url.find('?') {
        if index > 0 {
            path = &url[..index];
        }
    }
    if let Some(index) = path.find('#') {
        if index > 0 {
            path = &path[..index];
        }
    }
    match path.rfind('.') {
        Some(index) if index > 0 && index < path.len() - 1 => path[index + 1..].to_lowercase(),
        _ => String::new(),
    }
}

/// 外部直链校验（Java `validateExternalUrl`，文案逐字）。
///
/// 返回小写扩展名。
///
/// # Errors
/// 返回错误文案（与 Java `GlobalException` 一致）。
pub fn validate_external_url(url: &str, allowed_extensions: &[&str]) -> Result<String, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err(ERR_URL_EMPTY.to_string());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ERR_URL_SCHEME.to_string());
    }
    for pattern in BLOCKED_URL_PATTERNS {
        if url.contains(pattern) {
            return Err(ERR_URL_BLOCKED.to_string());
        }
    }
    let ext = extract_extension(url);
    if ext.is_empty() || !allowed_extensions.iter().any(|a| *a == ext) {
        return Err(format!(
            "不支持的文件格式，允许的格式: {}",
            allowed_extensions.join(", ")
        ));
    }
    Ok(ext)
}

/// 上传文件本地校验（Java `filterImageType` + `filterImageSize`）：
/// 类型必须在 [`ALLOWED_IMAGE_TYPES`]，大小不超过配置上限（MB）。
///
/// # Errors
/// 返回 `(业务码, 文案)`：类型/空文件为 `1001`，尺寸超限为 `1001`（Java `SERVICE_DATA_ERROR`）。
pub fn validate_upload_file(
    content_type: Option<&str>,
    size: u64,
    max_size_mb: &str,
) -> Result<(), (i32, String)> {
    if size == 0 {
        return Err((1001, ERR_UPLOAD_FILE_EMPTY.to_string()));
    }
    let allowed = content_type.is_some_and(|ct| ALLOWED_IMAGE_TYPES.contains(&ct));
    if !allowed {
        return Err((1001, ERR_UPLOAD_FILE_TYPE.to_string()));
    }
    let max_mb = parse_max_size_mb(max_size_mb);
    let max_bytes = max_mb * 1024.0 * 1024.0;
    if size as f64 > max_bytes {
        return Err((
            1001,
            format!("图片大小不能超过{}MB", format_max_size(max_mb)),
        ));
    }
    Ok(())
}

/// 解析配置的最大图片尺寸（MB）；缺失/非法/非正 → 默认 20。
#[must_use]
pub fn parse_max_size_mb(raw: &str) -> f64 {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| *value > 0.0)
        .unwrap_or(20.0)
}

/// 尺寸文案（Java `BigDecimal.stripTrailingZeros().toPlainString()`）：`20.0` → `"20"`。
#[must_use]
pub fn format_max_size(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        let text = format!("{value}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// 素材类型是否为视频（Java `"video".equals(type)`）。
#[must_use]
pub fn is_video_type(type_: Option<&str>) -> bool {
    type_ == Some("video")
}

/// 判断一次上传是否为 SVG（MIME 或文件名扩展名任一命中即按 SVG 处理）。
#[must_use]
pub fn is_svg_upload(content_type: Option<&str>, file_name: Option<&str>) -> bool {
    if content_type.is_some_and(|ct| ct.trim().eq_ignore_ascii_case("image/svg+xml")) {
        return true;
    }
    file_name.is_some_and(|name| extract_extension(name) == "svg")
}

// ——————————————————————————————————————————————
// SVG 净化（本平台扩展：Tigshop v5.8.65 素材 SVG 支持的安全配套，原版无此处理）
//
// 策略（轻量线性扫描，不引 XML 依赖）：
// - 必须是合法 UTF-8、根元素为 <svg>、标签配平，否则整体拒绝；
// - 带内部子集 / <!ENTITY 的 DOCTYPE 拒绝（防 XXE / 实体膨胀），其余
//   DOCTYPE 与非 xml 的处理指令（xml-stylesheet 等）直接丢弃；
// - 危险元素整体移除：script / foreignObject / iframe / embed / object /
//   link / meta / base / handler / listener / audio / video；
// - SMIL 元素（animate / set / animateMotion / animateTransform）若
//   attributeName 指向 href/xlink:href 则整体移除，其 to/from/by/values
//   含 javascript:/vbscript:/data:text 时丢弃该属性；
// - on* 事件属性一律丢弃；href / xlink:href 做实体解码 + 去空白后校验，
//   仅允许 #fragment、http(s):、data:image/ 与相对引用，其余丢弃属性；
// - style 属性含 javascript:/expression(/-moz-binding/behavior: 时丢弃；
//   <style> 元素内容含上述危险串时整元素丢弃，@import 语句逐条剥离；
// - CDATA 段展开为转义文本（保持字符数据语义，杜绝借 CDATA 注入标记）。
// ——————————————————————————————————————————————

/// 危险元素名单（小写）。
const SVG_BANNED_ELEMENTS: [&str; 12] = [
    "script",
    "foreignobject",
    "iframe",
    "embed",
    "object",
    "link",
    "meta",
    "base",
    "handler",
    "listener",
    "audio",
    "video",
];

/// SMIL 动画元素名单（小写）。
const SVG_SMIL_ELEMENTS: [&str; 4] = ["animate", "animatemotion", "animatetransform", "set"];

/// 净化 SVG 字节：成功返回净化后字节，失败（非法 XML / 非 SVG /
/// DTD 实体）返回 [`ERR_SVG_INVALID`]。
///
/// # Errors
/// 输入不是合法 SVG 或含不可净化的危险结构时返回错误文案。
pub fn sanitize_svg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| ERR_SVG_INVALID.to_string())?;
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("<!entity")
        || (lowered.contains("<!doctype") && text.contains('['))
    {
        return Err(ERR_SVG_INVALID.to_string());
    }
    let mut cleaner = SvgCleaner {
        src: text,
        pos: 0,
        out: String::with_capacity(text.len()),
        stack: Vec::new(),
        root_seen: false,
    };
    cleaner.run()?;
    if !cleaner.root_seen || !cleaner.stack.is_empty() {
        return Err(ERR_SVG_INVALID.to_string());
    }
    Ok(cleaner.out.into_bytes())
}

/// SVG 线性扫描净化器（字节下标只落在 ASCII 定界符上，UTF-8 安全）。
struct SvgCleaner<'a> {
    src: &'a str,
    pos: usize,
    out: String,
    /// 已打开元素名（小写）栈，用于配平校验。
    stack: Vec<String>,
    root_seen: bool,
}

impl SvgCleaner<'_> {
    fn bytes(&self) -> &[u8] {
        self.src.as_bytes()
    }

    fn run(&mut self) -> Result<(), String> {
        let len = self.src.len();
        while self.pos < len {
            if self.bytes()[self.pos] == b'<' {
                self.markup()?;
            } else {
                let start = self.pos;
                while self.pos < len && self.bytes()[self.pos] != b'<' {
                    self.pos += 1;
                }
                let text = &self.src[start..self.pos];
                if !self.root_seen && !text.trim().is_empty() {
                    return Err(ERR_SVG_INVALID.to_string());
                }
                if self.root_seen && self.stack.is_empty() && !text.trim().is_empty() {
                    return Err(ERR_SVG_INVALID.to_string());
                }
                self.out.push_str(text);
            }
        }
        Ok(())
    }

    /// 处理 `<` 开头的一个标记（注释 / CDATA / PI / DOCTYPE / 开闭标签）。
    fn markup(&mut self) -> Result<(), String> {
        let rest = &self.src[self.pos..];
        if rest.starts_with("<!--") {
            let end = rest[4..]
                .find("-->")
                .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
            self.out.push_str(&rest[..4 + end + 3]);
            self.pos += 4 + end + 3;
            return Ok(());
        }
        if rest.starts_with("<![CDATA[") {
            let end = rest[9..]
                .find("]]>")
                .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
            // CDATA 是字符数据：转义后输出，杜绝借其注入标记。
            for ch in rest[9..9 + end].chars() {
                match ch {
                    '&' => self.out.push_str("&amp;"),
                    '<' => self.out.push_str("&lt;"),
                    '>' => self.out.push_str("&gt;"),
                    _ => self.out.push(ch),
                }
            }
            self.pos += 9 + end + 3;
            return Ok(());
        }
        if rest.starts_with("<?") {
            let end = rest[2..]
                .find("?>")
                .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
            // 仅保留 <?xml 声明，其余 PI（xml-stylesheet 等）丢弃。
            if rest[2..].starts_with("xml") && matches!(rest.as_bytes().get(5), Some(b' ' | b'\t' | b'\r' | b'\n' | b'?')) {
                self.out.push_str(&rest[..2 + end + 2]);
            }
            self.pos += 2 + end + 2;
            return Ok(());
        }
        if rest.starts_with("<!") {
            // DOCTYPE 等声明：内部子集已在入口拒绝，这里整段丢弃。
            let end = rest
                .find('>')
                .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
            self.pos += end + 1;
            return Ok(());
        }
        if rest.starts_with("</") {
            return self.close_tag();
        }
        self.open_tag()
    }

    fn close_tag(&mut self) -> Result<(), String> {
        let rest = &self.src[self.pos..];
        let end = rest
            .find('>')
            .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
        let raw_name = rest[2..end].trim();
        if raw_name.is_empty() || !raw_name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':')) {
            return Err(ERR_SVG_INVALID.to_string());
        }
        let lname = raw_name.to_ascii_lowercase();
        if self.stack.pop().as_deref() != Some(lname.as_str()) {
            return Err(ERR_SVG_INVALID.to_string());
        }
        self.out.push_str("</");
        self.out.push_str(raw_name);
        self.out.push('>');
        self.pos += end + 1;
        Ok(())
    }

    fn open_tag(&mut self) -> Result<(), String> {
        let bytes = self.bytes();
        let len = bytes.len();
        let mut i = self.pos + 1;
        let name_start = i;
        while i < len && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'_' | b'-' | b':')) {
            i += 1;
        }
        if i == name_start {
            return Err(ERR_SVG_INVALID.to_string());
        }
        // 取_owned：后续要以 &mut self 调用处理方法，不能再借用 self.src。
        let raw_name = self.src[name_start..i].to_string();
        let lname = raw_name.to_ascii_lowercase();
        // 解析属性表
        let mut attrs: Vec<(String, Option<String>)> = Vec::new();
        let mut self_closing = false;
        loop {
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= len {
                return Err(ERR_SVG_INVALID.to_string());
            }
            if bytes[i] == b'>' {
                i += 1;
                break;
            }
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>') {
                self_closing = true;
                i += 2;
                break;
            }
            let attr_start = i;
            while i < len
                && !bytes[i].is_ascii_whitespace()
                && !matches!(bytes[i], b'=' | b'>' | b'/')
            {
                i += 1;
            }
            if i == attr_start {
                return Err(ERR_SVG_INVALID.to_string());
            }
            let attr_name = self.src[attr_start..i].to_string();
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let mut attr_value: Option<String> = None;
            if i < len && bytes[i] == b'=' {
                i += 1;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i >= len {
                    return Err(ERR_SVG_INVALID.to_string());
                }
                if bytes[i] == b'"' || bytes[i] == b'\'' {
                    let quote = bytes[i];
                    i += 1;
                    let value_start = i;
                    while i < len && bytes[i] != quote {
                        i += 1;
                    }
                    if i >= len {
                        return Err(ERR_SVG_INVALID.to_string());
                    }
                    attr_value = Some(self.src[value_start..i].to_string());
                    i += 1;
                } else {
                    let value_start = i;
                    while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                        i += 1;
                    }
                    attr_value = Some(self.src[value_start..i].to_string());
                }
            }
            attrs.push((attr_name, attr_value));
        }
        self.pos = i;

        // 根元素约束：首个元素必须是 svg，且只能有一个根。
        if self.stack.is_empty() {
            if self.root_seen || lname != "svg" {
                return Err(ERR_SVG_INVALID.to_string());
            }
            self.root_seen = true;
        }

        let banned = SVG_BANNED_ELEMENTS.contains(&lname.as_str())
            || (SVG_SMIL_ELEMENTS.contains(&lname.as_str())
                && attrs.iter().any(|(name, value)| {
                    name.eq_ignore_ascii_case("attributeName")
                        && value.as_deref().is_some_and(|v| {
                            let v = v.trim().to_ascii_lowercase();
                            v == "href" || v == "xlink:href"
                        })
                }));
        if banned {
            if !self_closing {
                self.skip_element(&lname)?;
            }
            return Ok(());
        }

        if lname == "style" && !self_closing {
            return self.style_element(&raw_name, attrs);
        }

        let kept = filter_svg_attrs(&lname, attrs);
        push_open_tag(&mut self.out, &raw_name, &kept, self_closing);
        if !self_closing {
            self.stack.push(lname);
        }
        Ok(())
    }

    /// `<style>` 元素：内容为 CSS 原文，单独净化后整段输出（含闭合标签）。
    fn style_element(
        &mut self,
        raw_name: &str,
        attrs: Vec<(String, Option<String>)>,
    ) -> Result<(), String> {
        let rest = &self.src[self.pos..];
        let lower = rest.to_ascii_lowercase();
        let close_at = lower
            .find("</style")
            .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
        let content = &rest[..close_at];
        let content_lower = content.to_ascii_lowercase();
        if !css_text_is_safe(&content_lower) {
            // 危险样式：整元素连同闭合标签跳过。
            let after = &rest[close_at..];
            let end = after
                .find('>')
                .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
            self.pos += close_at + end + 1;
            return Ok(());
        }
        let cleaned = strip_css_imports(content);
        let kept = filter_svg_attrs("style", attrs);
        push_open_tag(&mut self.out, raw_name, &kept, false);
        self.out.push_str(&cleaned);
        self.out.push_str("</style>");
        let after = &rest[close_at..];
        let end = after
            .find('>')
            .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
        self.pos += close_at + end + 1;
        Ok(())
    }

    /// 跳过一个危险元素的全部内容（同名嵌套计数），消费其闭合标签。
    fn skip_element(&mut self, lname: &str) -> Result<(), String> {
        let mut depth = 1usize;
        let len = self.src.len();
        while self.pos < len {
            let Some(rel) = self.src[self.pos..].find('<') else {
                break;
            };
            let at = self.pos + rel;
            let rest = &self.src[at..];
            let after_name = |tag: &str| -> Option<usize> {
                let rest_bytes = rest.as_bytes();
                let tag_bytes = tag.as_bytes();
                if rest_bytes.len() > tag_bytes.len()
                    && rest_bytes[..tag_bytes.len()].eq_ignore_ascii_case(tag_bytes)
                    && matches!(
                        rest_bytes[tag_bytes.len()],
                        b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/'
                    )
                {
                    Some(at + tag_bytes.len())
                } else {
                    None
                }
            };
            if let Some(name_end) = after_name(&format!("</{lname}")) {
                let end = self.src[name_end..]
                    .find('>')
                    .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
                self.pos = name_end + end + 1;
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            } else if let Some(name_end) = after_name(&format!("<{lname}")) {
                let end = self.src[name_end..]
                    .find('>')
                    .ok_or_else(|| ERR_SVG_INVALID.to_string())?;
                if !self.src[name_end..name_end + end].ends_with('/') {
                    depth += 1;
                }
                self.pos = name_end + end + 1;
            } else {
                self.pos = at + 1;
            }
        }
        Err(ERR_SVG_INVALID.to_string())
    }
}

/// 把一个开标签按保留属性重组输出（属性值统一双引号并转义 `"` 与 `<`）。
fn push_open_tag(
    out: &mut String,
    raw_name: &str,
    attrs: &[(String, Option<String>)],
    self_closing: bool,
) {
    out.push('<');
    out.push_str(raw_name);
    for (name, value) in attrs {
        out.push(' ');
        out.push_str(name);
        if let Some(value) = value {
            out.push_str("=\"");
            for ch in value.chars() {
                match ch {
                    '"' => out.push_str("&quot;"),
                    '<' => out.push_str("&lt;"),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
    }
    if self_closing {
        out.push_str("/>");
    } else {
        out.push('>');
    }
}

/// 属性过滤：丢弃 on* 事件属性、危险 URL 的 href、危险 style 与 SMIL 目标值。
fn filter_svg_attrs(
    lname: &str,
    attrs: Vec<(String, Option<String>)>,
) -> Vec<(String, Option<String>)> {
    let is_smil = SVG_SMIL_ELEMENTS.contains(&lname);
    attrs
        .into_iter()
        .filter(|(name, value)| {
            let aname = name.to_ascii_lowercase();
            if aname.starts_with("on") {
                return false;
            }
            if aname == "href" || aname.ends_with(":href") {
                return value
                    .as_deref()
                    .is_none_or(|v| svg_url_allowed(v));
            }
            if aname == "style" {
                return value
                    .as_deref()
                    .is_none_or(|v| css_text_is_safe(&v.to_ascii_lowercase()));
            }
            if is_smil && matches!(aname.as_str(), "to" | "from" | "by" | "values") {
                return value.as_deref().is_none_or(|v| {
                    !v.split(';').any(|seg| {
                        let s = decode_xml_entities(seg);
                        let s: String = s
                            .chars()
                            .filter(|c| !c.is_ascii_whitespace() && !c.is_control())
                            .collect::<String>()
                            .to_ascii_lowercase();
                        s.starts_with("javascript:")
                            || s.starts_with("vbscript:")
                            || s.starts_with("data:text")
                    })
                });
            }
            true
        })
        .collect()
}

/// CSS 文本安全性（入参须已小写）：不含脚本型构造。
fn css_text_is_safe(lowered: &str) -> bool {
    !lowered.contains("javascript:")
        && !lowered.contains("expression(")
        && !lowered.contains("-moz-binding")
        && !lowered.contains("behavior:")
}

/// 剥离 CSS 中的 @import 语句（到最近的 `;` 为止，无 `;` 则剥到结尾）。
fn strip_css_imports(css: &str) -> String {
    let lower = css.to_ascii_lowercase();
    let mut out = String::with_capacity(css.len());
    let mut cursor = 0usize;
    while let Some(rel) = lower[cursor..].find("@import") {
        let at = cursor + rel;
        out.push_str(&css[cursor..at]);
        match css[at..].find(';') {
            Some(off) => cursor = at + off + 1,
            None => {
                cursor = css.len();
                break;
            }
        }
    }
    out.push_str(&css[cursor..]);
    out
}

/// href 类属性的 URL 白名单校验：先解 XML 实体、去空白/控制字符，
/// 仅允许 #fragment、http(s):、data:image/ 与相对引用。
fn svg_url_allowed(raw: &str) -> bool {
    let decoded = decode_xml_entities(raw);
    let cleaned: String = decoded
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && !c.is_control())
        .collect();
    let lower = cleaned.to_ascii_lowercase();
    if lower.starts_with('#')
        || lower.starts_with("http:")
        || lower.starts_with("https:")
        || lower.starts_with("data:image/")
    {
        return true;
    }
    // 相对引用：首个定界符不是 ':'（即没有 scheme）。
    match lower.find([':', '/', '#', '?']) {
        Some(idx) => lower.as_bytes()[idx] != b':',
        None => true,
    }
}

/// 解码 XML 实体（命名实体 + 十进制/十六进制字符引用），供 URL 校验用。
fn decode_xml_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let Some(semi) = rest[at..].find(';') else {
            out.push_str(&rest[at..]);
            return out;
        };
        let entity = &rest[at + 1..at + semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| {
                    entity
                        .strip_prefix('#')
                        .and_then(|dec| dec.parse::<u32>().ok())
                })
                .and_then(char::from_u32),
        };
        match decoded {
            Some(ch) => {
                out.push(ch);
                rest = &rest[at + semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

// ——————————————————————————————————————————————
// 入参
// ——————————————————————————————————————————————

/// 相册图片列表查询（Java `GalleryPicListDTO extends BasePage`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicListQuery {
    /// 相册 id；不传/为 0 表示全部（含未归组）
    pub gallery_id: Option<i32>,
    /// 格式筛选（jpg/png/webp…）
    pub format: Option<String>,
    /// 来源筛选：local/upload/external
    pub source: Option<String>,
    /// 关键词（`pic_name ILIKE`）
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub sort_field: Option<String>,
    /// `manual` 表示按 `pic_sort` 升序
    pub sort_order: Option<String>,
}

impl GalleryPicListQuery {
    /// 分页换算（Java `BasePage`）。
    #[must_use]
    pub fn page_query(&self) -> PageQuery {
        page_query(self.page, self.size)
    }

    /// 生效的相册过滤：`galleryId > 0` 才按「自身及子孙」过滤。
    #[must_use]
    pub fn gallery_filter(&self) -> Option<i32> {
        self.gallery_id.filter(|id| *id > 0)
    }

    /// 有效关键词（Java `StrUtil.isNotBlank`，ILIKE 用原始值不 trim）。
    #[must_use]
    pub fn effective_keyword(&self) -> Option<&str> {
        self.keyword.as_deref().filter(|kw| !kw.trim().is_empty())
    }

    /// 有效格式（Java `isNotBlank(format)`）。
    #[must_use]
    pub fn effective_format(&self) -> Option<&str> {
        self.format.as_deref().filter(|f| !f.trim().is_empty())
    }
}

/// 图片手动排序（Java `GalleryPicSortDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicSortDTO {
    /// 当前分组 id（@NotNull 分组ID不能为空）
    pub gallery_id: Option<i32>,
    /// 排序后的 picId 列表（@NotEmpty）
    pub ids: Option<Vec<i32>>,
}

/// 批量移动素材（Java `GalleryPicTransferDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicTransferDTO {
    pub ids: Option<Vec<i32>>,
    pub target_gallery_id: Option<i32>,
    /// image / video
    pub r#type: Option<String>,
}

/// 批量删除/下载素材（Java `GalleryPicBatchDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicBatchDTO {
    pub ids: Option<Vec<i32>>,
    /// image / video
    pub r#type: Option<String>,
}

/// 外部直链创建图片（Java `GalleryPicExternalDTO`，字段均必填）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicExternalDTO {
    pub pic_url: Option<String>,
    pub pic_thumb: Option<String>,
    pub pic_name: Option<String>,
    pub gallery_id: Option<i32>,
    pub source: Option<String>,
}

/// 通用操作（Java `OperateDTO`，`del` 只用 `id`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OperateDTO {
    pub id: Option<i32>,
}

/// 更新字段（Java `UpdateFieldDTO`）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateFieldDTO {
    pub id: i32,
    pub field: String,
    #[serde(deserialize_with = "crate::models::serde_helpers::deserialize_stringish")]
    pub val: String,
}

// ——————————————————————————————————————————————
// 出参
// ——————————————————————————————————————————————

/// 相册图片 DTO（Java `GalleryPicDTO`；`shopId` 恒 null，见模块注释）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicDTO {
    pub pic_id: i32,
    /// Java DTO 为 String 而实体 Integer → BeanUtils 不复制 → 恒 null
    pub shop_id: Option<String>,
    pub gallery_id: i32,
    pub pic_ower_id: i32,
    pub pic_url: String,
    pub pic_name: String,
    pub pic_thumb: String,
    pub add_time: String,
    pub file_type: Option<String>,
    pub pic_width: Option<i32>,
    pub pic_height: Option<i32>,
    pub pic_size: Option<i64>,
}

/// 子相册预览（Java `GalleryPicResDTO extends GalleryDTO` + `galleryPics`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryPicResVO {
    /// 嵌套相册字段（flatten 后与原 GalleryDTO JSON 同级）
    #[serde(flatten)]
    pub gallery: GalleryDTO,
    /// 该相册前 4 张图片
    pub gallery_pics: Vec<GalleryPicDTO>,
}

/// 相册图片列表返回（Java `GalleryListResVO`）。
///
/// `PageResult` 没有实现 `Default`，故这里手写 `Default` 而不是 derive。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryListResVO {
    pub child_gallery_list: Vec<GalleryPicResVO>,
    pub gallery_pic_page: crate::types::PageResult<GalleryPicDTO>,
    pub gallery_info: Option<GalleryDTO>,
}

impl Default for GalleryListResVO {
    fn default() -> Self {
        Self {
            child_gallery_list: Vec::new(),
            gallery_pic_page: crate::types::PageResult {
                total: 0,
                list: Vec::new(),
                total_page: 0,
                page_size: 0,
                page: 0,
            },
            gallery_info: None,
        }
    }
}

/// 上传/外部创建返回（Java `GalleryPicUploadVO`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryPicUploadVO {
    pub pic_thumb: String,
    pub pic_url: String,
    pub pic_name: String,
    pub pic_id: i32,
    pub pic_width: Option<i32>,
    pub pic_height: Option<i32>,
    pub pic_size: Option<i64>,
    pub gallery_id: i32,
    pub file_type: Option<String>,
}

impl From<&crate::entities::GalleryPic> for GalleryPicDTO {
    fn from(pic: &crate::entities::GalleryPic) -> Self {
        Self {
            pic_id: pic.pic_id,
            // Java String/Integer 不兼容 → BeanUtils 不复制
            shop_id: None,
            gallery_id: pic.gallery_id,
            pic_ower_id: pic.pic_ower_id,
            pic_url: pic.pic_url.clone(),
            pic_name: pic.pic_name.clone(),
            pic_thumb: pic.pic_thumb.clone(),
            add_time: pic.add_time.clone(),
            file_type: pic.file_type.clone(),
            pic_width: pic.pic_width,
            pic_height: pic.pic_height,
            pic_size: pic.pic_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_url_validation_matches_java() {
        assert_eq!(
            validate_external_url("", &ALLOWED_IMAGE_EXTENSIONS).unwrap_err(),
            ERR_URL_EMPTY
        );
        assert_eq!(
            validate_external_url("ftp://a/b.jpg", &ALLOWED_IMAGE_EXTENSIONS).unwrap_err(),
            ERR_URL_SCHEME
        );
        assert_eq!(
            validate_external_url(
                "https://image.baidu.com/search/index?tn=baiduimage",
                &ALLOWED_IMAGE_EXTENSIONS
            )
            .unwrap_err(),
            ERR_URL_BLOCKED
        );
        assert!(validate_external_url(
            "https://image.baidu.com/search/photo.jpg",
            &ALLOWED_IMAGE_EXTENSIONS
        )
        .is_err());
        // 本平台扩展（v5.8.65）：webp 外链放行
        assert_eq!(
            validate_external_url("https://cdn.x.com/a.webp?x=1", &ALLOWED_IMAGE_EXTENSIONS)
                .unwrap(),
            "webp"
        );
        assert_eq!(
            validate_external_url("https://cdn.x.com/a.tiff?x=1", &ALLOWED_IMAGE_EXTENSIONS)
                .unwrap_err(),
            "不支持的文件格式，允许的格式: bmp, jpg, jpeg, png, gif, webp, svg"
        );
        assert_eq!(
            validate_external_url("https://cdn.x.com/icon.svg", &ALLOWED_IMAGE_EXTENSIONS)
                .unwrap(),
            "svg"
        );
        assert_eq!(
            validate_external_url("https://cdn.x.com/a.PNG?x=1#f", &ALLOWED_IMAGE_EXTENSIONS)
                .unwrap(),
            "png"
        );
    }

    #[test]
    fn extension_and_source_and_sort_helpers() {
        assert_eq!(extract_extension("https://a/b/c.tar.gz?q=1"), "gz");
        assert_eq!(extract_extension("noext"), "");
        assert_eq!(extract_extension(""), "");
        assert_eq!(normalize_source_filter(Some("external")), Some("external"));
        assert_eq!(normalize_source_filter(Some(" LOCAL ")), Some("upload"));
        assert_eq!(normalize_source_filter(Some("other")), None);
        assert_eq!(normalize_source_filter(None), None);
        assert_eq!(resolve_sort(None, Some("manual")), ("pic_sort", false));
        assert_eq!(resolve_sort(None, Some("desc")), ("add_time", true));
        assert_eq!(resolve_sort(Some("picSize"), None), ("pic_size", false));
        assert_eq!(resolve_sort(Some("bad;drop"), None), ("add_time", false));
        assert_eq!(resolve_gallery_pic_field("picName"), Some("pic_name"));
        assert_eq!(resolve_gallery_pic_field("source"), None);
    }

    #[test]
    fn upload_validation_matches_java() {
        assert_eq!(
            validate_upload_file(Some("image/png"), 0, "20")
                .unwrap_err()
                .1,
            ERR_UPLOAD_FILE_EMPTY
        );
        assert_eq!(
            validate_upload_file(Some("text/plain"), 100, "20")
                .unwrap_err()
                .1,
            ERR_UPLOAD_FILE_TYPE
        );
        assert!(validate_upload_file(Some("image/png"), 1024, "20").is_ok());
        // 本平台扩展：WebP / SVG 上传（MIME）放行
        assert!(validate_upload_file(Some("image/webp"), 1024, "20").is_ok());
        assert!(validate_upload_file(Some("image/svg+xml"), 1024, "20").is_ok());
        assert!(is_svg_upload(Some("image/svg+xml"), Some("a.png")));
        assert!(is_svg_upload(Some("image/png"), Some("icon.SVG")));
        assert!(!is_svg_upload(Some("image/png"), Some("a.png")));
        assert!(!is_svg_upload(None, None));
        // 超限：20MB 配置
        assert_eq!(
            validate_upload_file(Some("image/jpeg"), 21 * 1024 * 1024, "20")
                .unwrap_err()
                .1,
            "图片大小不能超过20MB"
        );
        assert_eq!(parse_max_size_mb("bad"), 20.0);
        assert_eq!(parse_max_size_mb("2.5"), 2.5);
        assert_eq!(format_max_size(20.0), "20");
        assert_eq!(format_max_size(2.5), "2.5");
    }

    #[test]
    fn sanitize_svg_keeps_clean_document() {
        let clean = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#fff"/><a href="https://example.com/x"><text>ok</text></a><use href="#icon"/><image href="data:image/png;base64,AAAA"/></svg>"##;
        let out = sanitize_svg(clean).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("<rect"));
        assert!(text.contains("https://example.com/x"));
        assert!(text.contains("href=\"#icon\""));
        assert!(text.contains("data:image/png"));
    }

    #[test]
    fn sanitize_svg_strips_scripts_and_event_handlers() {
        let dirty = br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><foreignObject><div>hi</div></foreignObject><rect width="5" onclick="alert(2)" onload="alert(3)"/><circle r="4"/></svg>"#;
        let out = sanitize_svg(dirty).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains("script"), "{text}");
        assert!(!text.contains("alert"), "{text}");
        assert!(!text.contains("foreignObject"), "{text}");
        assert!(!text.contains("onclick"), "{text}");
        assert!(!text.contains("onload"), "{text}");
        assert!(text.contains("width=\"5\""), "{text}");
        assert!(text.contains("<circle"), "{text}");
    }

    #[test]
    fn sanitize_svg_drops_javascript_hrefs() {
        let dirty = br#"<svg><a href="javascript:alert(1)"><text>x</text></a><a href="&#106;avascript:alert(1)"><text>y</text></a><a xlink:href=" java&#x09;script:alert(1)"><text>z</text></a></svg>"#;
        let out = sanitize_svg(dirty).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains("javascript:"), "{text}");
        assert!(!text.contains("href"), "{text}");
        assert!(text.contains("<text>x</text>"), "{text}");
    }

    #[test]
    fn sanitize_svg_handles_style_and_smil() {
        let dirty = br#"<svg><style>@import url("https://evil.example/x.css"); .a{fill:red}</style><style>.b{fill:url(javascript:alert(1))}</style><animate attributeName="href" to="javascript:alert(1)"/><rect class="a"/></svg>"#;
        let out = sanitize_svg(dirty).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains("@import"), "{text}");
        assert!(!text.contains("javascript:"), "{text}");
        assert!(!text.contains("animate"), "{text}");
        assert!(text.contains(".a{fill:red}"), "{text}");
        assert!(text.contains("<rect"), "{text}");
    }

    #[test]
    fn sanitize_svg_cdata_stays_text() {
        let dirty = br#"<svg><text><![CDATA[<script>alert(1)</script>]]></text></svg>"#;
        let out = sanitize_svg(dirty).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains("<script>"), "{text}");
        assert!(text.contains("&lt;script&gt;"), "{text}");
    }

    #[test]
    fn sanitize_svg_rejects_invalid_documents() {
        // DTD 实体（XXE / 实体膨胀）
        assert!(sanitize_svg(
            br#"<!DOCTYPE svg [ <!ENTITY xxe SYSTEM "file:///etc/passwd"> ]><svg/>"#
        )
        .is_err());
        // 非 SVG 根元素
        assert!(sanitize_svg(b"<html><body>x</body></html>").is_err());
        // 纯文本
        assert!(sanitize_svg(b"not an svg").is_err());
        // 标签不配平
        assert!(sanitize_svg(b"<svg><rect></svg>").is_err());
        assert!(sanitize_svg(b"<svg><rect>").is_err());
        // 非 UTF-8
        assert!(sanitize_svg(&[0xff, 0xfe, 0x3c]).is_err());
    }

    #[test]
    fn query_helpers_match_base_page() {
        let q = GalleryPicListQuery {
            page: Some(2),
            size: Some(500),
            gallery_id: Some(0),
            keyword: Some("  ".into()),
            ..Default::default()
        };
        let page = q.page_query();
        assert_eq!(page.page, 2);
        assert_eq!(page.page_size, 100);
        assert_eq!(q.gallery_filter(), None);
        assert_eq!(q.effective_keyword(), None);
        let q = GalleryPicListQuery {
            gallery_id: Some(3),
            keyword: Some("图".into()),
            ..Default::default()
        };
        assert_eq!(q.gallery_filter(), Some(3));
        assert_eq!(q.effective_keyword(), Some("图"));
        assert!(is_video_type(Some("video")));
        assert!(!is_video_type(Some("image")));
        assert!(!is_video_type(None));
    }

    #[test]
    fn dto_and_vo_serialize_camel_case() {
        let pic = crate::entities::GalleryPic {
            pic_id: 1,
            shop_id: 0,
            gallery_id: 2,
            pic_ower_id: 3,
            pic_url: "https://x/a.jpg".into(),
            pic_name: "a".into(),
            pic_thumb: "t".into(),
            add_time: "1700000000".into(),
            vendor_id: 0,
            hidden: None,
            pic_width: Some(1),
            pic_height: Some(2),
            pic_size: Some(3),
            file_type: Some("jpg".into()),
            pic_sort: None,
            source: "external".into(),
        };
        let dto = GalleryPicDTO::from(&pic);
        assert_eq!(dto.shop_id, None);
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["picId"], 1);
        assert_eq!(value["galleryId"], 2);
        assert_eq!(value["picOwerId"], 3);
        assert!(value["shopId"].is_null());

        let vo = GalleryPicUploadVO {
            pic_id: 5,
            pic_url: "u".into(),
            pic_name: "n".into(),
            gallery_id: 9,
            ..Default::default()
        };
        let value = serde_json::to_value(&vo).unwrap();
        assert_eq!(value["picId"], 5);
        assert_eq!(value["galleryId"], 9);
    }
}
