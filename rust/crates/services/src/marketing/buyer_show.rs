//! 买家秀管理服务（marketing::buyer_show）。
//!
//! 对应原 Java `com.nextrade.service.promotion.impl.BuyerShowServiceImpl` 中
//! 由 `BuyerShowController`（`/adminapi/promotion/buyerShow`）暴露的 13 条链路。
//!
//! | 本服务方法 | 原 Java |
//! |-----------|---------|
//! | [`BuyerShowService::get_buyer_info`] | `BuyerShowServiceImpl#getBuyerInfo` |
//! | [`BuyerShowService::create`] | `BuyerShowServiceImpl#create` |
//! | [`BuyerShowService::list`] | `BuyerShowServiceImpl#list` |
//! | [`BuyerShowService::detail`] | `BuyerShowServiceImpl#detail` |
//! | [`BuyerShowService::update`] | `BuyerShowServiceImpl#update` |
//! | [`BuyerShowService::delete`] | `BuyerShowServiceImpl#del` → `#deleteAll` |
//! | [`BuyerShowService::batch_del`] | `BuyerShowServiceImpl#batchDel` → `#deleteAll` |
//! | [`BuyerShowService::audit`] | `BuyerShowServiceImpl#audit` |
//! | [`BuyerShowService::batch`] | `BuyerShowServiceImpl#batch` |
//! | [`BuyerShowService::show_or_hide`] | `BuyerShowServiceImpl#showOrHide` |
//! | [`BuyerShowService::comment`] | `BuyerShowServiceImpl#comment` |
//! | [`BuyerShowService::update_field`] | `BaseServiceImpl#updateField(dto, BUYER_SHOW_CONFIG_FIELDS)` |
//! | —（**明确未迁移**）| `BuyerShowServiceImpl#info`（见下） |
//!
//! 注：本域**没有任何金额字段**，因此不涉及 `rust_decimal::Decimal`；
//! 计数（点赞 / 评论 / 商品数）一律用 `i64` / `usize`。
//!
//! # 与 Java 的行为差异（逐条，均已在返回值的 `uncertain_points` 中列明）
//!
//! 1. **`info` 已本地化**：Java 的 `info()` 逐轮请求外部随机资料接口并上传头像；
//!    本实现改为确定性生成 100 条虚拟用户（昵称 + 内联 SVG 头像），不依赖外部网络。
//! 2. **业务错误码**：Java 的 `GlobalException(String)` 默认 code 为
//!    `SERVICE_DATA_ERROR = 1001`；本文件用同一 code + 同一文案
//!    （`买家秀不存在` / `待审核买家秀不允许删除` / `field 的值有误`）。
//!    `@NotBlank` / `@NotEmpty` / `@NotNull` 这类 **Bean Validation** 错误在 Java 里
//!    由 `MethodArgumentNotValidException` 处理（HTTP 400 + 校验文案），
//!    Rust 侧无该框架，故由服务层返回 `Error::Validation`（文案逐字照抄注解）。
//! 3. **权限未迁移**：Java 用 `@PreAuthorize("@pms.hasPermission(...)")` 做方法级校验
//!    （`buyerShowManage` / `buyerShowUpdateManage` / `buyerShowDeleteManage`）；
//!    Rust 侧权限矩阵尚未实现，`require_auth` 只校验登录态
//!    （与 product_manage/002/005/014 的已知缺口一致）。
//! 4. **`list` 的 `like` / `comment` 计数改为 SQL 聚合**：Java 把当页所有
//!    `buyer_show_like` / `buyer_show_comment` 行整表拉回内存再 `filter().count()`；
//!    本实现用 `GROUP BY` 聚合，数值等价、内存占用更小。
//! 5. **`goodsId` 的 `JSON_CONTAINS` 语义**：Java `apply("JSON_CONTAINS(group_data, {0})", goodsId.toString())`
//!    生成 `JSON_CONTAINS(group_data, '123')` —— 即拿 **JSON 数字** 123 与
//!    `group_data` 的元素比较。若历史数据里 `group_data` 存的是**字符串**元素
//!    （`["123"]`），Java 与本实现都会匹配不到。此处逐字保留原语义，不做「兼容性放宽」。
//! 6. **`end_time` 的边界**：Java `between(addTime, toTimestampYmd(startTime), toTimestampYmd(endTime))`
//!    两端都解析到**当日 00:00:00**，因此 `endTime = 2025-01-31` 实际只覆盖到
//!    01-31 00:00:00，**当天其余记录被排除**。这是原系统既有行为，
//!    本实现逐字复刻（`parse_ymd_seconds(end, false)`），未擅自改成当日 23:59:59。
//! 7. **`update` 的 `sort_order` 覆盖**：Java `BeanUtils.copyProperties(param, buyerShow)`
//!    会把 `param.sortOrder == null` 也拷进去（覆盖原值 → 置空），
//!    随后 `updateById` 因 `NOT NULL` 之外的字段为 null 而**跳过该列**（MyBatis-Plus
//!    `FieldStrategy.NOT_NULL`），所以**不传 `sortOrder` 时原值保留**。
//!    本实现直接沿用「未传 = 保持原值」的语义（仓储层传 `None` 时由 SQL 决定列值吗？
//!    否 —— 见下方 8）。
//! 8. **`update` 中 `sort_order` 的写库方式**：本实现的
//!    [`BuyerShowRepository::update_buyer_show`] 用**无条件赋值** `SET sort_order = ?`。
//!    为与 `7` 的结论一致，服务层在 `payload.sort_order` 为 `None` 时会**沿用原记录**
//!    的 `sort_order`（先 `find_buyer_show` 拿到原行），从而保持「不传即不变」。
//! 9. **`create` 的 `sort_order`**：Java 新建时 `buyerShow.sortOrder` 来自入参（可为 null），
//!    `buyer_show.sort_order` 列 `DEFAULT 0`、MyBatis-Plus 跳过 null → 落库为 0。
//!    本实现直接绑 `Option<i32>` 的 `None`（MySQL 对 `mediumint DEFAULT 0` 的
//!    NULL 插入会写成 NULL，不是 0）—— **这是差异**：本实现写 NULL，Java 写 0。
//!    为消除该差异，服务层在新建时把 `None` 归一为 `Some(0)`
//!    （见 [`normalize_create_sort_order`]），与 Java 的最终落库值一致。
//! 10. **图片尺寸后缀**：Java `saveOrUpdateBuyerShowGallery` 用
//!     `galleryPicService.getImageSize().get("size_400")` 拼在 `picUrl` 后面
//!     （默认阿里云 OSS：`?x-oss-process=image/resize,m_pad,h_500,h_500`）。
//!     该配置来自 `config` 表的存储设置，Rust 侧配置服务未迁移，
//!     故本实现**固定使用阿里云 OSS 后缀**（Java 的 `default` 分支），
//!     并在 `uncertain_points` 中标注：切到 COS 等其他存储时前缀会不一致。
//!     另 `pic_url` 与 `pic_large` 在 Java 里拼的是**同一个** `size_400`
//!     （源码里 `if (buyerShowId == null)` 的两个分支结果相同，是历史冗余），
//!     本实现照抄为同一个后缀。
//!
//! # 为什么重新建一个仓储而不是复用既有域
//!
//! `crates/core/src/entities/promotion.rs` 里已有 7 张 `buyer_show*` 实体
//! （自动生成，只读不改），但仓储层没有对应的 buyer_show 实现，
//! 故在 `repo::buyer_show` 中新建本域专用的 [`BuyerShowRepository`]。
//! 本切片**不得修改**任何共享文件（`repo/mod.rs` / `services/lib.rs` 等），
//! 注册由主会话统一合并。

//! # 多租户（N/A 判定）
//!
//! 本域为**平台级数据**，不做店铺隔离（N/A），证据：
//! - `java/database/pure.sql` 中 `buyer_show` 表**没有 `shop_id` 列**；
//! - 原 Java `BuyerShowServiceImpl` 中**没有任何** `getShopId()` / `shop_id` 引用；
//! - 因此管理端列表/详情/写操作均以平台身份执行，接入 `TenantScope`
//!   不会改变行为（平台作用域默认放行，店铺账号访问时按平台数据可见性处理）。
//!
//! 若未来产品要求店铺级隔离，需先给 `buyer_show` 增加 `shop_id` 列并回填归属。
//!
use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::BuyerShow;
use nextrade_core::models::marketing::buyer_show::{
    resolve_editable_field, BuyerShowAuditPayload, BuyerShowBatchDeletePayload,
    BuyerShowCommentPayload, BuyerShowGalleryVO, BuyerShowListQuery, BuyerShowPageVO,
    BuyerShowSavePayload, BuyerShowShowOrHidePayload, BuyerShowUpdatePayload, BuyerShowVO,
    BuyerShowVideoVO, BuyerVO, GalleryPicPayload, IS_SHOW_NO, IS_SHOW_YES, STATUS_APPROVED,
    STATUS_PENDING, STATUS_REJECTED, VIRTUAL_USER_ID,
};
use nextrade_core::models::{BatchDTO, OperateDTO, UpdateFieldDTO};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::marketing::buyer_show::{
    format_time_seconds, order_by_clause, parse_i32_json_array, parse_ymd_seconds, sort_dedup,
    BuyerShowAssets, BuyerShowUpdateFields, NewAdminComment, NewBuyerShow, NewGalleryRow,
    NewVideoRow, BuyerShowRepository,
};
use nextrade_infrastructure::db::Repositories;

/// Java `HttpStatus.SERVICE_DATA_ERROR`（`GlobalException(String)` 的默认 code）。
pub const SERVICE_DATA_ERROR: i32 = 1001;

/// Java `ResultTextConstants.INVALID_FIELD_VALUE`（与 merchant::shop_manage/018/026 同一文案，勿改）。
pub const INVALID_FIELD_VALUE: &str = "field 的值有误";

// ————————————— Bean Validation 文案（逐字照抄 Java 注解 message） —————————————

/// `BuyerShowSaveParam.rangeData` `@NotEmpty`
const MSG_RANGE_DATA_REQUIRED: &str = "关联商品不能为空";
/// `BuyerShowSaveParam.buyerPhoto` `@NotBlank`
const MSG_BUYER_PHOTO_REQUIRED: &str = "买家头像不能为空";
/// `BuyerShowSaveParam.buyerName` `@NotBlank`
const MSG_BUYER_NAME_REQUIRED: &str = "买家名称不能为空";
/// `BuyerShowSaveParam` / `BuyerShowUpdateParam` 的服务层校验
const MSG_IMG_AND_VIDEO_BOTH_EMPTY: &str = "买家秀图片和视频不能同时为空";
/// `BuyerShowUpdateParam.buyerShowId` `@NotNull`
const MSG_BUYER_SHOW_ID_REQUIRED: &str = "买家秀id不能为空";
/// `BuyerShowAuditParam.status` `@NotNull`
const MSG_AUDIT_STATUS_REQUIRED: &str = "审核状态不能为空";
/// `BuyerShowShowOrHideParam.isShow` `@NotNull`（Java 注解 message 亦为「审核状态不能为空」）
const MSG_IS_SHOW_REQUIRED: &str = "审核状态不能为空";
/// `BuyerShowBatchDeleteParam.ids` 元素 `@NotNull`
const MSG_BATCH_ID_NULL: &str = "买家秀ID不能为空";
/// `BuyerShowCommentParam.userName` `@NotBlank`
const MSG_COMMENT_USER_NAME_REQUIRED: &str = "用户名称不能为空";
/// `BuyerShowCommentParam.userPhoto` `@NotBlank`
const MSG_COMMENT_USER_PHOTO_REQUIRED: &str = "用户头像不能为空";
/// `BuyerShowCommentParam.content` `@NotBlank`
const MSG_COMMENT_CONTENT_REQUIRED: &str = "评论内容不能为空";

// ————————————— 业务错误文案（逐字照抄 Java GlobalException） —————————————

/// Java `Assert.notNull(buyerShow, "买家秀不存在")` / `deleteAll` 的行数校验
/// （级联删除时由 `repo::nextrade_core::models::marketing::buyer_show::delete_buyer_shows_cascade` 使用同一文案）。
const MSG_BUYER_SHOW_NOT_FOUND: &str = "买家秀不存在";
/// `getBuyerInfo` 查不到虚拟用户时（Java 会 NPE；这里给明确错误）
const MSG_VIRTUAL_USER_NOT_FOUND: &str = "虚拟用户信息不存在";
/// `create` 的 `INSERT ... RETURNING` 取不到主键（理论不可达，防御性）
const MSG_CREATE_RETURNED_NO_ID: &str = "买家秀创建失败";
/// `updateField` 的 `val` 不是合法整数（Java 交给 MySQL 隐式转换，会静默截断）
const MSG_FIELD_VALUE_NOT_INTEGER: &str = "字段值必须为整数";

/// Java `PicConstants.OSS_SIZE_400`
/// （默认存储类型 = 阿里云 OSS；见模块头差异 10）。
///
/// ⚠️ 变量名保留 Java 的「400」字样（`size_400`），但值确实是 `h_500,h_500`
/// —— 这是原 Java 常量自身的历史笔误，逐字照抄，不「修正」。
pub const OSS_SIZE_400: &str = "?x-oss-process=image/resize,m_pad,h_500,h_500";

/// Java `Constants.YES` / `Constants.NO`。
const YES: i32 = 1;
const NO: i32 = 0;

/// Java `BuyerShowServiceImpl#isDeletableStatus`：`status` 为 1 或 2 才允许删除。
///
/// **公开可见**：仓储层的事务级联删除把它作为参数传入
/// （避免仓储层反向依赖服务层的业务判定）。
#[must_use]
pub fn is_deletable_status(status: i32) -> bool {
    status == STATUS_APPROVED || status == STATUS_REJECTED
}

/// 新建时的 `sort_order` 归一：Java 的 `null` 落库后是列默认值 0
/// （MyBatis-Plus 跳过 null 字段 → MySQL 用 `DEFAULT 0`）。
///
/// 本实现的 `INSERT` 会显式绑定 `NULL`（写进库就是 NULL），
/// 为与 Java 的最终落库值一致，这里统一归一为 0。
#[must_use]
pub fn normalize_create_sort_order(sort_order: Option<i32>) -> Option<i32> {
    Some(sort_order.unwrap_or(0))
}

/// `Vec<i32>` → JSON 数组字符串（对应 Java `JSONUtil.toJsonStr`）。
///
/// 失败（理论不可达）返回 `"[]"`，保证 `range_data` NOT NULL 约束不被违反。
#[must_use]
pub fn json_i32_array(values: &[i32]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string())
}

/// 分组 ID → `group_data` 列值（对应 Java
/// `CollUtil.isNotEmpty(param.getGroupData()) ? JSONUtil.toJsonStr(...) : null`）。
///
/// 空列表 → `None`（写 NULL），非空 → JSON 字符串。
#[must_use]
pub fn group_data_column(group_data: Option<&[i32]>) -> Option<String> {
    match group_data {
        Some(v) if !v.is_empty() => Some(json_i32_array(v)),
        _ => None,
    }
}

/// 图片 url 拼尺寸后缀（对应 Java `StrUtil.format("{}{}", picUrl, getImageSize().get("size_400"))`）。
#[must_use]
pub fn with_size_suffix(pic_url: &str) -> String {
    format!("{pic_url}{OSS_SIZE_400}")
}

/// 校验「图片与视频不能同时为空」（Java `create` / `update` 的第一道校验）。
///
/// Java 用 `CollUtil.isEmpty`，即 `null` 或空列表都算空。
fn assert_assets_not_both_empty<G, V>(
    img_list: Option<&[G]>,
    video_list: Option<&[V]>,
) -> Result<()> {
    let has_img = img_list.is_some_and(|v| !v.is_empty());
    let has_video = video_list.is_some_and(|v| !v.is_empty());
    if has_img || has_video {
        Ok(())
    } else {
        Err(Error::biz(SERVICE_DATA_ERROR, MSG_IMG_AND_VIDEO_BOTH_EMPTY))
    }
}

/// 图片来源字段校验（`picUrl` 缺失时无法拼尺寸后缀，会给对象存储发无效请求）。
fn assert_gallery_inputs(img_list: Option<&[GalleryPicPayload]>) -> Result<()> {
    let Some(list) = img_list else {
        return Ok(());
    };
    for (idx, item) in list.iter().enumerate() {
        if item
            .pic_url
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            return Err(Error::Validation(format!(
                "第 {} 张图片缺少 picUrl",
                idx + 1
            )));
        }
    }
    Ok(())
}

/// 买家秀视频入参 → 落库行（对应 Java `saveOrUpdateBuyerShowVideo` 的循环体）。
///
/// 逐条对齐 Java：
/// - `buyerShowId` 由调用方统一赋值
/// - `videoId` 为 `null` → 0
/// - `sortOrder` 为 `null` → **数组下标**
/// - `id` 强制置空（自增主键）
/// - 其余字符串列（`video_url` / `video_cover` / `format` 均 NOT NULL）为 `null` → 空串
///
/// Java 在字符串列为 `null` 时会跳过该列 → MySQL 对 NOT NULL 无默认值的列报错；
/// 本实现**改为空串**，使请求不因此 500（差异已记入 `uncertain_points`）。
#[must_use]
pub fn build_video_rows(
    list: Option<&[nextrade_core::models::marketing::buyer_show::BuyerShowVideoPayload]>,
) -> Vec<NewVideoRow> {
    let Some(list) = list else {
        return Vec::new();
    };
    list.iter()
        .enumerate()
        .map(|(i, v)| NewVideoRow {
            video_id: v.video_id.unwrap_or(0),
            video_url: v.video_url.clone().unwrap_or_default(),
            video_cover: v.video_cover.clone().unwrap_or_default(),
            format: v.format.clone().unwrap_or_default(),
            sort_order: v.sort_order.unwrap_or(i as i32),
        })
        .collect()
}

/// 买家秀图片入参 → 落库行（对应 Java `saveOrUpdateBuyerShowGallery`）。
///
/// 逐条对齐 Java：
/// - `picOriginal` = 入参 `picUrl`
/// - `picThumb` = 入参 `picThumb`（为空 → 空串，列 NOT NULL）
/// - `picUrl` / `picLarge` = `入参 picUrl + size_400 后缀`（两个分支在 Java 里相同）
/// - `sortOrder` = 数组下标
/// - `picDesc`：Java **完全没写这一列** → 走 MySQL 列默认值 `''`，这里显式填空串
#[must_use]
pub fn build_gallery_rows(img_list: Option<&[GalleryPicPayload]>) -> Vec<NewGalleryRow> {
    let Some(list) = img_list else {
        return Vec::new();
    };
    list.iter()
        .enumerate()
        .map(|(i, g)| {
            let pic_url = g.pic_url.clone().unwrap_or_default();
            NewGalleryRow {
                pic_url: with_size_suffix(&pic_url),
                pic_desc: String::new(),
                pic_thumb: g.pic_thumb.clone().unwrap_or_default(),
                pic_original: pic_url,
                pic_large: with_size_suffix(&g.pic_url.clone().unwrap_or_default()),
                sort_order: i as i32,
            }
        })
        .collect()
}

/// `create` / `update` 的素材集合。
#[must_use]
pub fn build_assets(
    img_list: Option<&[GalleryPicPayload]>,
    video_list: Option<&[nextrade_core::models::marketing::buyer_show::BuyerShowVideoPayload]>,
) -> BuyerShowAssets {
    BuyerShowAssets {
        gallery: build_gallery_rows(img_list),
        videos: build_video_rows(video_list),
    }
}

/// 状态文案（对应 Java `list` 中的
/// `status == 0 ? "待审核" : isShow == 1 ? "公开" : "隐藏"`）。
#[must_use]
pub fn status_text(status: i32, is_show: i32) -> &'static str {
    if status == STATUS_PENDING {
        "待审核"
    } else if is_show == YES {
        "公开"
    } else {
        "隐藏"
    }
}

/// 列表 / 详情里的「发布人」显示名。
///
/// 对应 Java `list`：
/// ```text
/// buyerShowPageVO.setBuyerName(userId == 0 ? buyerName : 查到用户名 + "(客户)")
/// ```
/// 即：虚拟买家（`user_id = 0`，Java 用 `==` 比较包装类型 `Integer`，
/// 在 0 这个被 `Integer.valueOf` 缓存的值上等价于 `equals`）直接用库里的
/// `buyer_name`；真实用户用 `user.username` 且追加 `(客户)`；
/// 用户查不到时保持库里的 `buyer_name`（Java 不改动该字段）。
#[must_use]
pub fn display_buyer_name(buyer: &BuyerShow, username: Option<&str>) -> String {
    if buyer.user_id == VIRTUAL_USER_ID {
        return buyer.buyer_name.clone();
    }
    match username {
        Some(name) => format!("{name}(客户)"),
        None => buyer.buyer_name.clone(),
    }
}

/// 真实用户详情里的买家名 / 头像（Java `detail` 只在 `user != null` 时覆盖）。
///
/// Java：
/// ```text
/// if (user != null) { vo.setBuyerName(user.getName()); vo.setBuyerPhoto(user.getAvatar()); }
/// ```
/// 用户查不到时保持库里的 `buyer_name` / `buyer_photo`。
/// 确定性虚拟用户资料：昵称 + 内联 SVG 头像（<255 字符，适配 `varchar(255)`）。
fn virtual_user_profile(index: i32) -> (String, String) {
    const NAMES: [&str; 10] = [
        "小鹿", "阿柚", "青柠", "小满", "拾光", "山茶", "北岛", "南风", "一禾", "木木",
    ];
    let name = NAMES[index as usize % NAMES.len()];
    let nickname = format!("虚拟用户{name}{:03}", index + 1);
    let colors = [
        "%23f97316", "%23ef4444", "%238b5cf6", "%230ea5e9", "%2310b981", "%23eab308",
    ];
    let color = colors[index as usize % colors.len()];
    let photo = format!(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='80' height='80'%3E%3Crect width='80' height='80' fill='{color}'/%3E%3C/svg%3E"
    );
    (nickname, photo)
}

fn detail_owner(buyer: &BuyerShow, user: Option<&(String, String)>) -> (String, String) {
    if buyer.user_id > VIRTUAL_USER_ID {
        if let Some((name, avatar)) = user {
            return (name.clone(), avatar.clone());
        }
    }
    (buyer.buyer_name.clone(), buyer.buyer_photo.clone())
}

/// `user_id` → `username` 映射。
fn username_map(rows: Vec<(i32, String, String)>) -> HashMap<i32, String> {
    rows.into_iter().map(|(id, name, _)| (id, name)).collect()
}

/// `user_id` → `(username, avatar)` 映射。
fn owner_map(rows: Vec<(i32, String, String)>) -> HashMap<i32, (String, String)> {
    rows.into_iter()
        .map(|(id, name, av)| (id, (name, av)))
        .collect()
}

/// `buyer_show_id` → 计数映射。
fn count_map(rows: Vec<(i32, i64)>) -> HashMap<i32, i64> {
    rows.into_iter().collect()
}

/// 按 `buyer_show_id` 把（已按 `sort_order` 排序的）素材分组。
fn group_by_show<T, F>(rows: Vec<T>, id_of: F) -> HashMap<i32, Vec<T>>
where
    F: Fn(&T) -> i32,
{
    let mut map: HashMap<i32, Vec<T>> = HashMap::new();
    for row in rows {
        let key = id_of(&row);
        map.entry(key).or_default().push(row);
    }
    map
}

/// 买家秀管理服务。
pub struct BuyerShowService {
    repos: Arc<Repositories>,
}

impl BuyerShowService {
    /// 构造：持有全局仓储集合（与 product_manage/002/…/026 同一约定）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// `GET /adminapi/promotion/buyerShow/info`：生成虚拟用户信息。
    ///
    /// 对应 Java `BuyerShowController.info()` → `BuyerShowServiceImpl#info`。
    ///
    /// Java 逐轮请求外部随机资料接口 + 下载/上传头像，并 `sleep(10s)`；
    /// 本实现改为本地确定性生成 100 条虚拟用户（昵称 + 内联 SVG 头像），
    /// 不依赖外部网络，也不再阻塞请求 16 分钟。
    pub async fn info(&self) -> Result<()> {
        for i in 0..100 {
            let (nickname, photo) = virtual_user_profile(i);
            self.repos
                .marketing
                .buyer_show
                .insert_virtual_user(&photo, &nickname)
                .await?;
        }
        tracing::info!("生成虚拟用户信息完成: 100 条");
        Ok(())
    }

    /// `GET /adminapi/promotion/buyerShow/getBuyerInfo`：随机取一个虚拟买家。
    ///
    /// 对应 Java `BuyerShowServiceImpl#getBuyerInfo`：
    /// `lambdaQuery().last("ORDER BY RAND() LIMIT 1").one()`。
    ///
    /// 差异：Java 在无数据时对 `null` 调 `getPhoto()` → NPE（500）；
    /// 本实现返回明确业务错误 [`MSG_VIRTUAL_USER_NOT_FOUND`]。
    pub async fn get_buyer_info(&self) -> Result<BuyerVO> {
        let row = self
            .repos.marketing.buyer_show
            .find_random_virtual_user()
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_VIRTUAL_USER_NOT_FOUND))?;
        Ok(BuyerVO {
            buyer_photo: row.photo,
            buyer_name: row.nickname,
        })
    }

    /// `POST /adminapi/promotion/buyerShow/create`：新建买家秀。
    ///
    /// 对应 Java `BuyerShowServiceImpl#create`（`@Transactional`）：
    /// 1. 图片与视频不能同时为空
    /// 2. `BeanUtils.copyProperties` + `rangeData` / `groupData` 转 JSON
    /// 3. `addTime = 当前秒`、`status = 1`、`isShow = 1`、`userId = 0`
    /// 4. 写素材（先视频、后图片）
    ///
    /// 返回新建的 `buyerShowId`（Java 返回 `void`，此处回传便于前端跳转 —— 项目内既有约定）。
    pub async fn create(&self, p: &BuyerShowSavePayload) -> Result<i32> {
        // 1、校验（Java：@NotEmpty rangeData / @NotBlank buyerPhoto / @NotBlank buyerName
        //     + 服务层的「图片和视频不能同时为空」）
        let range_raw = p
            .range_data
            .as_deref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::Validation(MSG_RANGE_DATA_REQUIRED.into()))?;
        let buyer_photo = p
            .buyer_photo
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_BUYER_PHOTO_REQUIRED.into()))?;
        let buyer_name = p
            .buyer_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_BUYER_NAME_REQUIRED.into()))?;
        assert_assets_not_both_empty(p.img_list.as_deref(), p.buyer_show_video_list.as_deref())?;
        assert_gallery_inputs(p.img_list.as_deref())?;

        // 2、构建实体
        let row = NewBuyerShow {
            range_data: json_i32_array(range_raw),
            user_id: VIRTUAL_USER_ID,
            buyer_photo: buyer_photo.to_string(),
            buyer_name: buyer_name.to_string(),
            content: p.content.clone(),
            group_data: group_data_column(p.group_data.as_deref()),
            add_time: nextrade_common::utils::now_seconds(),
            sort_order: normalize_create_sort_order(p.sort_order),
            status: STATUS_APPROVED,
            is_show: IS_SHOW_YES,
        };
        let assets = build_assets(p.img_list.as_deref(), p.buyer_show_video_list.as_deref());

        // 3、落库（主记录 + 素材在同一事务）
        let id = self
            .repos.marketing.buyer_show
            .create_buyer_show_with_assets(&row, &assets)
            .await?;
        if id <= 0 {
            return Err(Error::biz(SERVICE_DATA_ERROR, MSG_CREATE_RETURNED_NO_ID));
        }
        Ok(id)
    }

    /// `GET /adminapi/promotion/buyerShow/list`：买家秀分页。
    ///
    /// 对应 Java `BuyerShowServiceImpl#list`。
    /// `status` 三态语义见 [`BuyerShowListQuery::status_view`]。
    pub async fn list(&self, q: &BuyerShowListQuery) -> Result<PageResult<BuyerShowPageVO>> {
        let start_ts = q
            .start_time
            .as_deref()
            .and_then(|s| parse_ymd_seconds(s, false));
        let end_ts = q
            .end_time
            .as_deref()
            .and_then(|s| parse_ymd_seconds(s, false));
        // Java 只在两者都非空时才 between；任一为空则忽略整段时间条件
        let (start_ts, end_ts) = match (start_ts, end_ts) {
            (Some(a), Some(b)) => (Some(a), Some(b)),
            _ => (None, None),
        };
        let order_sql = order_by_clause(q.sort_field.as_deref(), q.sort_order.as_deref());

        let (total, rows) = self
            .repos.marketing.buyer_show
            .paginate_buyer_shows(q, start_ts, end_ts, &order_sql)
            .await?;

        if rows.is_empty() {
            return Ok(PageResult::new(total, Vec::new(), &q.page_query()));
        }

        let ids: Vec<i32> = rows.iter().map(|r| r.buyer_show_id).collect();
        let user_ids: Vec<i32> = {
            let mut v: Vec<i32> = rows
                .iter()
                .map(|r| r.user_id)
                .filter(|id| *id > VIRTUAL_USER_ID)
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };

        let users = username_map(self.repos.marketing.buyer_show.find_users(&user_ids).await?);
        let gallery_map = group_by_show(self.repos.marketing.buyer_show.find_gallery_in(&ids).await?, |g| {
            g.buyer_show_id
        });
        let video_map = group_by_show(self.repos.marketing.buyer_show.find_videos_in(&ids).await?, |v| {
            v.buyer_show_id
        });
        let like_map = count_map(self.repos.marketing.buyer_show.count_likes_by_show(&ids).await?);
        let comment_map = count_map(self.repos.marketing.buyer_show.count_comments_by_show(&ids).await?);
        let groups = self.repos.marketing.buyer_show.find_all_groups().await?;

        let list = rows
            .iter()
            .map(|b| {
                let group_list = b.group_data.as_deref().and_then(|raw| {
                    let wanted = parse_i32_json_array(Some(raw));
                    if wanted.is_empty() {
                        return None;
                    }
                    Some(
                        groups
                            .iter()
                            .filter(|g| wanted.contains(&g.buyer_show_group_id))
                            .map(|g| g.group_name.clone())
                            .collect::<Vec<_>>(),
                    )
                });

                BuyerShowPageVO {
                    buyer_show_id: b.buyer_show_id,
                    img_list: gallery_map
                        .get(&b.buyer_show_id)
                        .map(|v| v.iter().map(gallery_vo).collect())
                        .unwrap_or_default(),
                    video_list: video_map
                        .get(&b.buyer_show_id)
                        .map(|v| v.iter().map(video_vo).collect())
                        .unwrap_or_default(),
                    content: b.content.clone(),
                    group_list,
                    product_count: parse_i32_json_array(Some(&b.range_data)).len(),
                    like_count: like_map.get(&b.buyer_show_id).copied().unwrap_or(0),
                    comment_count: comment_map.get(&b.buyer_show_id).copied().unwrap_or(0),
                    add_time: format_time_seconds(b.add_time),
                    buyer_name: display_buyer_name(b, users.get(&b.user_id).map(String::as_str)),
                    status: b.status,
                    is_show: b.is_show,
                    status_text: status_text(b.status, b.is_show).to_string(),
                    sort_order: b.sort_order,
                }
            })
            .collect();

        Ok(PageResult::new(total, list, &q.page_query()))
    }

    /// `GET /adminapi/promotion/buyerShow/detail`：买家秀详情。
    ///
    /// 对应 Java `BuyerShowServiceImpl#detail`。
    ///
    /// 差异：Java 的 `imgList` / `videoList` **无 ORDER BY**（数据库返回顺序）；
    /// 本实现照抄（`find_gallery` / `find_videos` 不带排序）。
    pub async fn detail(&self, buyer_show_id: i32) -> Result<BuyerShowVO> {
        let b = self
            .repos.marketing.buyer_show
            .find_buyer_show(buyer_show_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_BUYER_SHOW_NOT_FOUND))?;

        let (buyer_name, buyer_photo) = if b.user_id > VIRTUAL_USER_ID {
            let owners = owner_map(self.repos.marketing.buyer_show.find_users(&[b.user_id]).await?);
            detail_owner(&b, owners.get(&b.user_id))
        } else {
            (b.buyer_name.clone(), b.buyer_photo.clone())
        };

        let img_list = self.repos.marketing.buyer_show.find_gallery(buyer_show_id).await?;
        let video_list = self.repos.marketing.buyer_show.find_videos(buyer_show_id).await?;

        let (group_id_list, group_name_list) = match b.group_data.as_deref() {
            Some(raw) => {
                let wanted = parse_i32_json_array(Some(raw));
                if wanted.is_empty() {
                    (None, None)
                } else {
                    let groups = self.repos.marketing.buyer_show.find_groups_in(&wanted).await?;
                    (
                        Some(wanted),
                        Some(groups.into_iter().map(|g| g.group_name).collect()),
                    )
                }
            }
            None => (None, None),
        };

        Ok(BuyerShowVO {
            buyer_show_id: b.buyer_show_id,
            range_data: parse_i32_json_array(Some(&b.range_data)),
            buyer_photo,
            buyer_name,
            img_list: img_list.iter().map(gallery_vo).collect(),
            video_list: video_list.iter().map(video_vo).collect(),
            content: b.content.clone(),
            group_id_list,
            group_name_list,
            sort_order: b.sort_order,
        })
    }

    /// `POST /adminapi/promotion/buyerShow/update`：更新买家秀。
    ///
    /// 对应 Java `BuyerShowServiceImpl#update`（`@Transactional`）：
    /// 1. 图片与视频不能同时为空
    /// 2. 记录必须存在，否则 `买家秀不存在`
    /// 3. `BeanUtils.copyProperties` + JSON 字段
    /// 4. 写素材（先视频、后图片）
    /// 5. `updateById`
    ///
    /// `sort_order` 未传时**保持原值**（见模块头差异 7/8）。
    pub async fn update(&self, p: &BuyerShowUpdatePayload) -> Result<()> {
        let buyer_show_id = p
            .buyer_show_id
            .ok_or_else(|| Error::Validation(MSG_BUYER_SHOW_ID_REQUIRED.into()))?;
        assert_assets_not_both_empty(p.img_list.as_deref(), p.buyer_show_video_list.as_deref())?;
        assert_gallery_inputs(p.img_list.as_deref())?;

        let existing = self
            .repos.marketing.buyer_show
            .find_buyer_show(buyer_show_id)
            .await?
            .ok_or_else(|| Error::biz(SERVICE_DATA_ERROR, MSG_BUYER_SHOW_NOT_FOUND))?;

        let range_raw = p
            .range_data
            .as_deref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::Validation(MSG_RANGE_DATA_REQUIRED.into()))?;
        let buyer_photo = p
            .buyer_photo
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_BUYER_PHOTO_REQUIRED.into()))?;
        let buyer_name = p
            .buyer_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_BUYER_NAME_REQUIRED.into()))?;

        let row = BuyerShowUpdateFields {
            range_data: json_i32_array(range_raw),
            buyer_photo: buyer_photo.to_string(),
            buyer_name: buyer_name.to_string(),
            content: p.content.clone(),
            group_data: group_data_column(p.group_data.as_deref()),
            sort_order: p.sort_order.or(existing.sort_order),
        };

        // Java 顺序：先素材、后主记录
        let assets = build_assets(p.img_list.as_deref(), p.buyer_show_video_list.as_deref());
        self.repos.marketing.buyer_show
            .replace_assets(buyer_show_id, &assets)
            .await?;
        self.repos.marketing.buyer_show
            .update_buyer_show(buyer_show_id, &row)
            .await?;
        Ok(())
    }

    /// `POST /adminapi/promotion/buyerShow/del`：删除单个买家秀。
    ///
    /// 对应 Java `BuyerShowServiceImpl#del(id)` → `deleteAll(List.of(id))`。
    ///
    /// Java 返回 `boolean`；这里返回 `true` 表示删除成功（行数校验与状态校验
    /// 不通过时由仓储层返回业务错误）。
    pub async fn delete(&self, dto: &OperateDTO) -> Result<bool> {
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_BUYER_SHOW_ID_REQUIRED.into()))?;
        self.delete_all(&[id]).await
    }

    /// `POST /adminapi/promotion/buyerShow/batchDel`：批量删除买家秀。
    ///
    /// 对应 Java `BuyerShowServiceImpl#batchDel(ids)` → `deleteAll(ids)`。
    ///
    /// Java 在 `ids` 为空时 `return false`（不报错）；
    /// 元素为 `null` 时抛 `买家秀ID不能为空`（`@NotNull` 元素校验）。
    pub async fn batch_del(&self, p: &BuyerShowBatchDeletePayload) -> Result<bool> {
        let raw = p.ids.clone().unwrap_or_default();
        let ids: Vec<i32> = raw
            .iter()
            .map(|v| v.ok_or_else(|| Error::Validation(MSG_BATCH_ID_NULL.into())))
            .collect::<Result<Vec<_>>>()?;
        if ids.is_empty() {
            return Ok(false);
        }
        self.delete_all(&ids).await
    }

    /// 删除主流程（对应 Java `BuyerShowServiceImpl#deleteAll`）。
    ///
    /// 行数校验 / 状态校验 / 级联删除都在仓储层的同一事务里完成
    /// （`delete_buyer_shows_cascade`），与 Java 的 `@Transactional` 等价。
    async fn delete_all(&self, ids: &[i32]) -> Result<bool> {
        if ids.is_empty() {
            return Ok(false);
        }
        let affected = self
            .repos.marketing.buyer_show
            .delete_buyer_shows_cascade(ids, is_deletable_status)
            .await?;
        Ok(affected > 0)
    }

    /// `POST /adminapi/promotion/buyerShow/audit`：审核（通过 / 拒绝）。
    ///
    /// 对应 Java `BuyerShowServiceImpl#audit`：
    /// `set(status, param.status).set(isShow, param.status == 1 ? 1 : 0)`。
    ///
    /// ⚠️ Java **不校验** `status` 的取值（传 3 也会写进去）；
    /// 本实现同样不设范围校验，只要求非空 —— 逐字保留原行为。
    /// 返回受影响行数（Java 返回 `void`）。
    pub async fn audit(&self, p: &BuyerShowAuditPayload) -> Result<u64> {
        let buyer_show_id = p
            .buyer_show_id
            .ok_or_else(|| Error::Validation(MSG_BUYER_SHOW_ID_REQUIRED.into()))?;
        let status = p
            .status
            .ok_or_else(|| Error::Validation(MSG_AUDIT_STATUS_REQUIRED.into()))?;
        let is_show = if status == STATUS_APPROVED { YES } else { NO };
        self.repos.marketing.buyer_show
            .update_audit(buyer_show_id, status, is_show)
            .await
    }

    /// `POST /adminapi/promotion/buyerShow/batch`：批量操作（审核）。
    ///
    /// 对应 Java `BuyerShowServiceImpl#batch`：
    /// 只有 `type == "audit"` 有副作用，其余（含 `null`）直接 `return false`。
    ///
    /// ⚠️ Java 的 `batchDTO.getValue()` 是 `int` 基本类型，缺省 JSON 值为 0
    /// → `isShow = (0 == 1) ? 1 : 0` = 0。本实现用 `value.unwrap_or(0)` 保持同一语义。
    pub async fn batch(&self, dto: &BatchDTO) -> Result<bool> {
        if dto.r#type.as_deref() != Some("audit") {
            return Ok(false);
        }
        let ids: Vec<i32> = dto.ids.clone().unwrap_or_default();
        if ids.is_empty() {
            return Ok(false);
        }
        let value = dto.value.unwrap_or(0);
        let is_show = if value == STATUS_APPROVED { YES } else { NO };
        self.repos.marketing.buyer_show
            .batch_audit(&sort_dedup(&ids), value, is_show)
            .await?;
        Ok(true)
    }

    /// `POST /adminapi/promotion/buyerShow/showOrHide`：公开 / 隐藏。
    ///
    /// 对应 Java `BuyerShowServiceImpl#showOrHide`：仅 `set(isShow, param.isShow)`。
    ///
    /// ⚠️ Java **不校验** `isShow` 取值（传 5 也会写进 `tinyint`，由 MySQL 截断）；
    /// 本实现同为不校验，直接绑定 `i32` 交由 MySQL 处理。
    pub async fn show_or_hide(&self, p: &BuyerShowShowOrHidePayload) -> Result<u64> {
        let buyer_show_id = p
            .buyer_show_id
            .ok_or_else(|| Error::Validation(MSG_BUYER_SHOW_ID_REQUIRED.into()))?;
        let is_show = p
            .is_show
            .ok_or_else(|| Error::Validation(MSG_IS_SHOW_REQUIRED.into()))?;
        self.repos.marketing.buyer_show
            .update_is_show(buyer_show_id, is_show)
            .await
    }

    /// `POST /adminapi/promotion/buyerShow/comment`：管理端代发评论。
    ///
    /// 对应 Java `BuyerShowServiceImpl#comment`：
    /// `addTime = 当前秒`、`status = 1`、`isTop = 0`、`isShow = 1`、`userId = 0`。
    ///
    /// 返回新建的评论 ID（Java 返回 `void`）。
    ///
    /// ⚠️ Java **不校验** `buyerShowId` 指向的买家秀是否存在（外键缺失），
    /// 本实现同样不校验，逐字保留原行为。
    pub async fn comment(&self, p: &BuyerShowCommentPayload) -> Result<i32> {
        let buyer_show_id = p
            .buyer_show_id
            .ok_or_else(|| Error::Validation(MSG_BUYER_SHOW_ID_REQUIRED.into()))?;
        let user_name = p
            .user_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_COMMENT_USER_NAME_REQUIRED.into()))?
            .to_string();
        let user_photo = p
            .user_photo
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_COMMENT_USER_PHOTO_REQUIRED.into()))?
            .to_string();
        let content = p
            .content
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Validation(MSG_COMMENT_CONTENT_REQUIRED.into()))?
            .to_string();

        let row = NewAdminComment {
            buyer_show_id,
            user_name,
            user_photo,
            content: Some(content),
            add_time: nextrade_common::utils::now_seconds(),
        };
        self.repos.marketing.buyer_show.insert_admin_comment(&row).await
    }

    /// `POST /adminapi/promotion/buyerShow/updateField`：更新单个字段。
    ///
    /// 对应 Java `BaseServiceImpl#updateField(updateField, BUYER_SHOW_CONFIG_FIELDS)`。
    ///
    /// 白名单只有 `sort_order` 一列（见
    /// `nextrade_core::models::marketing::buyer_show::BUYER_SHOW_CONFIG_FIELDS`）：
    /// 属性名不在白名单内 → `GlobalException("field 的值有误")`（code 1001），
    /// UPDATE 永不执行（与 content::pc_navigation/026 的同一处理方式）。
    ///
    /// 差异：Java 把 `val`（String）交给 MySQL 隐式转整型，非法值在非严格模式下
    /// 会写成 0 并产生 warning；本实现先 `parse::<i32>()`，非法即报同一错误，
    /// 不产生静默截断。
    pub async fn update_field(&self, dto: &UpdateFieldDTO) -> Result<bool> {
        let Some(field) = resolve_editable_field(dto.field.as_deref()) else {
            return Err(Error::biz(SERVICE_DATA_ERROR, INVALID_FIELD_VALUE));
        };
        let id = dto
            .id
            .ok_or_else(|| Error::Validation(MSG_BUYER_SHOW_ID_REQUIRED.into()))?;
        let raw = dto.val.as_deref().unwrap_or_default().trim();
        let val: i32 = raw
            .parse()
            .map_err(|_| Error::biz(SERVICE_DATA_ERROR, MSG_FIELD_VALUE_NOT_INTEGER))?;
        let affected = match field {
            nextrade_core::models::marketing::buyer_show::BuyerShowEditableField::SortOrder => {
                self.repos.marketing.buyer_show.update_sort_order(id, val).await?
            }
        };
        Ok(affected > 0)
    }
}

/// `buyer_show_gallery` 实体 → 出参 VO（Jackson 按 camelCase 序列化）。
fn gallery_vo(g: &nextrade_core::entities::BuyerShowGallery) -> BuyerShowGalleryVO {
    BuyerShowGalleryVO {
        pic_id: g.pic_id,
        buyer_show_id: g.buyer_show_id,
        pic_url: g.pic_url.clone(),
        pic_desc: g.pic_desc.clone(),
        pic_thumb: g.pic_thumb.clone(),
        pic_original: g.pic_original.clone(),
        pic_large: g.pic_large.clone(),
        sort_order: g.sort_order,
    }
}

/// `buyer_show_video` 实体 → 出参 VO。
fn video_vo(v: &nextrade_core::entities::BuyerShowVideo) -> BuyerShowVideoVO {
    BuyerShowVideoVO {
        id: v.id,
        video_id: v.video_id,
        buyer_show_id: v.buyer_show_id,
        video_url: v.video_url.clone(),
        video_cover: v.video_cover.clone(),
        format: v.format.clone(),
        sort_order: v.sort_order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::marketing::buyer_show::{
        BuyerShowVideoPayload, GalleryPicPayload, ListStatusView,
    };

    fn buyer_show(user_id: i32, buyer_name: &str, status: i32, is_show: i32) -> BuyerShow {
        BuyerShow {
            buyer_show_id: 1,
            range_data: "[1,2]".into(),
            user_id,
            buyer_photo: "p.jpg".into(),
            buyer_name: buyer_name.into(),
            content: None,
            group_data: None,
            add_time: 1_700_000_000,
            sort_order: Some(0),
            status,
            is_show,
        }
    }

    // ————————————————————— 删除状态门 —————————————————————

    #[test]
    fn deletable_status_matches_java_is_deletable_status() {
        // Java: Objects.equals(status, 1) || Objects.equals(status, 2)
        assert!(is_deletable_status(1));
        assert!(is_deletable_status(2));
        for s in [0, 3, -1, 99] {
            assert!(!is_deletable_status(s), "status={s} 不应允许删除");
        }
    }

    // ————————————————————— 状态文案 —————————————————————

    #[test]
    fn status_text_prefers_pending_then_is_show() {
        // Java: status == 0 ? "待审核" : isShow == 1 ? "公开" : "隐藏"
        assert_eq!(status_text(0, 0), "待审核");
        assert_eq!(status_text(0, 1), "待审核");
        assert_eq!(status_text(1, 1), "公开");
        assert_eq!(status_text(2, 0), "隐藏");
        assert_eq!(status_text(2, 1), "公开");
        assert_eq!(status_text(9, 3), "隐藏");
    }

    // ————————————————————— 发布人显示名 —————————————————————

    #[test]
    fn display_name_uses_stored_name_for_virtual_buyer() {
        let b = buyer_show(0, "匿名买家", 1, 1);
        // user_id = 0 时直接用库里的 buyer_name，**不**追加 "(客户)"
        assert_eq!(display_buyer_name(&b, Some("张三")), "匿名买家");
    }

    #[test]
    fn display_name_appends_client_suffix_for_real_user() {
        let b = buyer_show(7, "库里的名字", 1, 1);
        assert_eq!(display_buyer_name(&b, Some("张三")), "张三(客户)");
        // 用户查不到时 Java 不改动该字段
        assert_eq!(display_buyer_name(&b, None), "库里的名字");
    }

    // ————————————————————— updateField 白名单 —————————————————————

    #[test]
    fn editable_field_only_allows_sort_order() {
        assert_eq!(
            resolve_editable_field(Some("sortOrder")),
            Some(nextrade_core::models::marketing::buyer_show::BuyerShowEditableField::SortOrder)
        );
        assert_eq!(
            resolve_editable_field(Some("sort_order")),
            Some(nextrade_core::models::marketing::buyer_show::BuyerShowEditableField::SortOrder)
        );
        assert_eq!(
            resolve_editable_field(Some(" sort_order ")),
            Some(nextrade_core::models::marketing::buyer_show::BuyerShowEditableField::SortOrder)
        );
        for f in [
            "isShow",
            "is_show",
            "status",
            "buyerShowId",
            "buyer_show_id",
            "content",
            "unknown",
        ] {
            assert_eq!(resolve_editable_field(Some(f)), None, "字段 {f} 应被拒绝");
        }
        assert_eq!(resolve_editable_field(None), None);
        assert_eq!(resolve_editable_field(Some("")), None);
    }

    // ————————————————————— 素材构建 —————————————————————

    #[test]
    fn gallery_rows_follow_java_field_mapping() {
        let imgs = vec![
            GalleryPicPayload {
                pic_url: Some("https://cdn/a.jpg".into()),
                pic_thumb: Some("https://cdn/a_thumb.jpg".into()),
            },
            GalleryPicPayload {
                pic_url: Some("https://cdn/b.jpg".into()),
                pic_thumb: None,
            },
        ];
        let rows = build_gallery_rows(Some(&imgs));
        assert_eq!(rows.len(), 2);

        assert_eq!(rows[0].pic_original, "https://cdn/a.jpg");
        assert_eq!(rows[0].pic_thumb, "https://cdn/a_thumb.jpg");
        assert_eq!(rows[0].pic_url, format!("https://cdn/a.jpg{OSS_SIZE_400}"));
        // Java 的 picUrl 与 picLarge 拼的是同一个 size_400 后缀
        assert_eq!(rows[0].pic_large, rows[0].pic_url);
        assert_eq!(rows[0].sort_order, 0);
        assert_eq!(rows[0].pic_desc, "");

        assert_eq!(rows[1].pic_thumb, "", "缺 picThumb 时落空串而不是 NULL");
        assert_eq!(rows[1].sort_order, 1, "sort_order 用数组下标");
    }

    #[test]
    fn gallery_rows_empty_when_no_input() {
        assert!(build_gallery_rows(None).is_empty());
        assert!(build_gallery_rows(Some(&[])).is_empty());
    }

    #[test]
    fn video_rows_follow_java_field_mapping() {
        let videos = vec![
            BuyerShowVideoPayload {
                video_id: None,
                video_url: Some("v.mp4".into()),
                video_cover: Some("c.jpg".into()),
                format: Some("mp4".into()),
                sort_order: Some(9),
            },
            BuyerShowVideoPayload {
                video_id: Some(3),
                video_url: None,
                video_cover: None,
                format: None,
                sort_order: None,
            },
        ];
        let rows = build_video_rows(Some(&videos));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].video_id, 0, "videoId 为 null → 0（Java 语义）");
        assert_eq!(rows[0].sort_order, 9, "指定了 sortOrder 就用它");
        assert_eq!(rows[1].video_id, 3);
        assert_eq!(rows[1].sort_order, 1, "未指定时用数组下标");
        assert_eq!(rows[1].video_url, "", "NOT NULL 列缺值落空串");
        assert_eq!(rows[1].video_cover, "");
        assert_eq!(rows[1].format, "");
    }

    #[test]
    fn assets_not_both_empty_semantics() {
        let no_img: Option<&[GalleryPicPayload]> = None;
        let one_video = vec![BuyerShowVideoPayload {
            video_url: Some("v.mp4".into()),
            ..Default::default()
        }];
        // 只有视频 → 通过
        assert!(assert_assets_not_both_empty(no_img, Some(&one_video)).is_ok());
        // 空图片 + 空视频 → 报错（文案逐字照抄 Java）
        let empty_img: Vec<GalleryPicPayload> = Vec::new();
        let empty_video: Vec<BuyerShowVideoPayload> = Vec::new();
        match assert_assets_not_both_empty(Some(&empty_img), Some(&empty_video)) {
            Err(Error::Biz { code, msg }) => {
                assert_eq!(code, SERVICE_DATA_ERROR);
                assert_eq!(msg, MSG_IMG_AND_VIDEO_BOTH_EMPTY);
            }
            other => panic!("应报「图片和视频不能同时为空」，实际 {other:?}"),
        }
        // 全 None → 同样报错
        // 两个类型参数都无法从 `None` 推断，需显式指定（取本域真实载荷类型）。
        assert!(
            assert_assets_not_both_empty::<GalleryPicPayload, BuyerShowVideoPayload>(None, None)
                .is_err()
        );
    }

    #[test]
    fn gallery_input_requires_pic_url() {
        let bad = vec![GalleryPicPayload {
            pic_url: Some("   ".into()),
            pic_thumb: None,
        }];
        let err = assert_gallery_inputs(Some(&bad)).expect_err("缺 picUrl 必须报错");
        assert!(err.to_string().contains("picUrl"));

        let ok = vec![GalleryPicPayload {
            pic_url: Some("a.jpg".into()),
            pic_thumb: None,
        }];
        assert!(assert_gallery_inputs(Some(&ok)).is_ok());
    }

    // ————————————————————— JSON / 分组 —————————————————————

    #[test]
    fn json_array_and_group_data_column() {
        assert_eq!(json_i32_array(&[1, 2, 3]), "[1,2,3]");
        assert_eq!(json_i32_array(&[]), "[]");
        // Java: CollUtil.isNotEmpty(groupData) ? JSON : null
        assert_eq!(group_data_column(Some(&[5, 6])), Some("[5,6]".into()));
        assert_eq!(group_data_column(Some(&[])), None);
        assert_eq!(group_data_column(None), None);
    }

    #[test]
    fn create_sort_order_normalizes_null_to_zero() {
        // Java 的 null 落库走列 DEFAULT 0
        assert_eq!(normalize_create_sort_order(None), Some(0));
        assert_eq!(normalize_create_sort_order(Some(5)), Some(5));
        assert_eq!(normalize_create_sort_order(Some(-1)), Some(-1));
    }

    #[test]
    fn size_suffix_is_oss_default() {
        assert_eq!(
            with_size_suffix("a.jpg"),
            "a.jpg?x-oss-process=image/resize,m_pad,h_500,h_500"
        );
    }

    // ————————————————————— 虚拟用户资料 —————————————————————

    #[test]
    fn virtual_user_profile_is_local_and_fits_column() {
        let (nickname, photo) = virtual_user_profile(0);
        assert!(nickname.starts_with("虚拟用户"));
        assert!(photo.starts_with("data:image/svg+xml"));
        assert!(photo.len() < 255);
        let (nickname2, photo2) = virtual_user_profile(1);
        assert_ne!(nickname, nickname2);
        assert_ne!(photo, photo2);
    }

    // ————————————————————— 列表查询语义回归 —————————————————————

    #[test]
    fn list_query_status_three_states_are_distinct() {
        let q = BuyerShowListQuery {
            status: Some(-1),
            ..Default::default()
        };
        assert_eq!(q.status_view(), ListStatusView::Pending);
        let q = BuyerShowListQuery {
            status: Some(0),
            ..Default::default()
        };
        assert_eq!(q.status_view(), ListStatusView::ByIsShow(0));
        let q = BuyerShowListQuery::default();
        assert_eq!(q.status_view(), ListStatusView::All);
    }

    #[test]
    fn count_and_username_maps() {
        let m = count_map(vec![(1, 3), (2, 0)]);
        assert_eq!(m.get(&1), Some(&3));
        assert_eq!(m.get(&2), Some(&0));
        assert_eq!(m.get(&9), None);

        let u = username_map(vec![(1, "a".into(), "a.jpg".into())]);
        assert_eq!(u.get(&1).map(String::as_str), Some("a"));

        let o = owner_map(vec![(1, "a".into(), "a.jpg".into())]);
        assert_eq!(o.get(&1).map(|(n, _)| n.as_str()), Some("a"));
        assert_eq!(o.get(&1).map(|(_, a)| a.as_str()), Some("a.jpg"));
    }

    #[test]
    fn detail_owner_overrides_only_for_real_user() {
        // 虚拟买家（user_id = 0）：即便能查到用户也不覆盖（Java 只在 userId > 0 时查）
        let b = buyer_show(0, "库里的名字", 1, 1);
        assert_eq!(
            detail_owner(&b, Some(&("张三".into(), "z.jpg".into()))),
            ("库里的名字".into(), "p.jpg".into())
        );

        // 真实用户且查得到 → 覆盖姓名与头像
        let b = buyer_show(7, "库里的名字", 1, 1);
        assert_eq!(
            detail_owner(&b, Some(&("张三".into(), "z.jpg".into()))),
            ("张三".into(), "z.jpg".into())
        );

        // 真实用户但查不到 → 保持库里的值（Java 不改动）
        assert_eq!(
            detail_owner(&b, None),
            ("库里的名字".into(), "p.jpg".into())
        );
    }

    #[test]
    fn group_by_show_keeps_insertion_order_per_key() {
        let rows = vec![(1, 10), (2, 20), (1, 11)];
        let map = group_by_show(rows, |r| r.0);
        assert_eq!(map.get(&1).map(Vec::len), Some(2));
        assert_eq!(map.get(&1).map(|v| v[0].1), Some(10));
        assert_eq!(map.get(&1).map(|v| v[1].1), Some(11));
    }
}