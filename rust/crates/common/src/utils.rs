//! 通用工具函数，替代原 Java `com.nextrade.common.utils.*` 与 Hutool 工具类。

use sha2::{Digest, Sha256};

/// 生成订单号：`yyyyMMddHHmmss` + 6位随机数（与原 Java 订单号格式一致）。
pub fn generate_order_sn() -> String {
    use rand::Rng;
    let now = chrono::Local::now();
    let rand: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{}{:06}", now.format("%Y%m%d%H%M%S"), rand)
}

/// 生成指定长度的随机十六进制串（短信验证码/token 等）。
pub fn random_hex(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect()
}

/// SHA-256 十六进制摘要（原 Java DigestUtil.sha256Hex 等价）。
pub fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_ref());
    hex::encode(hasher.finalize())
}

/// 生成带前缀的业务流水号，如 `UC20260101120000ab12cd`。
///
/// 用于优惠券 SN（`user_coupon.coupon_sn`）等需要唯一短号的场景。
pub fn gen_sn(prefix: &str) -> String {
    let now = chrono::Local::now();
    format!("{prefix}{}{}", now.format("%Y%m%d%H%M%S"), random_hex(6))
}

/// 毫秒时间戳。
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 秒时间戳。
pub fn now_seconds() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 手机号脱敏：`138****8000`。
pub fn mask_mobile(mobile: &str) -> String {
    let chars: Vec<char> = mobile.chars().collect();
    if chars.len() >= 11 {
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[7..11].iter().collect();
        format!("{head}****{tail}")
    } else if mobile.is_empty() {
        String::new()
    } else {
        "*".repeat(mobile.len())
    }
}

/// 邮箱脱敏：`ab****@example.com`。
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((user, domain)) if user.len() > 2 => {
            format!("{}****@{}", &user[..2], domain)
        }
        _ => "***".to_string(),
    }
}

/// Java `PicConstants.OSS_SIZE_200`：默认存储（阿里云 OSS）的 200px 缩略后缀。
///
/// ⚠️ 拼写口径：原版常量字面为 `?x-oss-process=image/resize,m_pad,h_200,h_200`
/// （快照仓库 `tigshop-common/.../pic/PicConstants.java:19`，`h` 重复是原版自身
/// 的历史笔误，`OSS_SIZE_400`/`800` 同型；线上 Java 载荷实测亦为该串）。
/// 本项目存量已归一行（2026-11-19 数据治理：product 35 条）采用 `h_200,w_200`
/// 拼写；两串在 OSS `resize,m_pad` 下同为 200×200 等比填充、渲染等价，
/// 为全库单一口径，本常量跟随存量取 `w_200` 拼写。
pub const OSS_SIZE_200: &str = "?x-oss-process=image/resize,m_pad,h_200,w_200";

/// Java `PicConstants.OSS_SIZE_400`：默认存储（阿里云 OSS）的「400 档」后缀。
///
/// ⚠️ 名实不符是原版自身口径：常量名与 `getImageSize` 的键为 `size_400`，
/// 字面值却是 `h_500,h_500`（快照仓库 `PicConstants.java:24`；种子数据
/// product.pic_url 存量亦为该串）。本常量逐字取原版字面，不做「纠正」。
pub const OSS_SIZE_400: &str = "?x-oss-process=image/resize,m_pad,h_500,h_500";

/// Java `PicConstants.OSS_SIZE_800`：默认存储（阿里云 OSS）的 800px 大图后缀
/// （快照仓库 `PicConstants.java:27`，字面 `h_800,h_800` 与存量
/// product.pic_large 已派生行一致）。
pub const OSS_SIZE_800: &str = "?x-oss-process=image/resize,m_pad,h_800,h_800";

/// 给原图 URL 拼 OSS 尺寸处理后缀的共用核（200/400/800 三档共用）。
///
/// 与原版的差异（我方健壮性增强，原版为无条件直拼）：本实现幂等——
/// URL 已带 `x-oss-process=` 时原样返回，绝不重复拼参；已带其它 query 时
/// 以 `&` 续接，保留既有参数（如签名）。空串原样返回。
#[must_use]
fn with_oss_size(pic_url: &str, suffix: &str) -> String {
    let url = pic_url.trim();
    if url.is_empty() || url.contains("x-oss-process=") {
        return pic_url.to_string();
    }
    if url.contains('?') {
        format!("{url}&{}", &suffix[1..])
    } else {
        format!("{url}{suffix}")
    }
}

/// 由原图 URL 生成 200px 缩略图 URL（对应 Java 图库上传落库点
/// `GalleryPicServiceImpl#createPic`：`picThumb = picUrl + getImageSize().get("size_200")`，
/// 商品侧 `setImgList`/`saveProductGallery` 只透传该值——缩略图在图库写入时生成）。
#[must_use]
pub fn pic_thumb_200(pic_url: &str) -> String {
    with_oss_size(pic_url, OSS_SIZE_200)
}

/// 由原图 URL 生成「400 档」（实为 500px）展示图 URL（对应 Java
/// `ProductServiceImpl#setImgList` 新增分支：
/// `picUrl = picUrl + getImageSize().get("size_400")`，仅商品**新增**时
/// 对 product.pic_url 派生；编辑分支 picUrl 存裸值，见调用方口径）。
#[must_use]
pub fn pic_sized_400(pic_url: &str) -> String {
    with_oss_size(pic_url, OSS_SIZE_400)
}

/// 由原图 URL 生成 800px 大图 URL（对应 Java `ProductServiceImpl#setImgList`
/// 两分支共同的 `picLarge = picUrl + getImageSize().get("size_800")`：
/// 新增与编辑都对 product.pic_large 派生）。
#[must_use]
pub fn pic_sized_800(pic_url: &str) -> String {
    with_oss_size(pic_url, OSS_SIZE_800)
}

/// 归一既有 `pic_thumb` 取值（商品 / 商品相册 / 图库写入侧统一口径）：
/// - 为空、或与 `pic_url` 逐字相同（裸 thumb）→ 由 `pic_url` 按
///   [`pic_thumb_200`] 再生；
/// - 其余（已带处理参数、或与 `pic_url` 不同图的显式缩略）→ 原样保留，
///   不臆造参数、不覆盖调用方显式取值。
#[must_use]
pub fn normalize_pic_thumb(pic_url: &str, pic_thumb: &str) -> String {
    let thumb = pic_thumb.trim();
    if thumb.is_empty() || thumb == pic_url.trim() {
        pic_thumb_200(pic_url)
    } else {
        pic_thumb.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_sn_format() {
        let sn = generate_order_sn();
        assert_eq!(sn.len(), 20); // 14 时间 + 6 随机
        assert!(sn.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn gen_sn_has_prefix_and_is_unique() {
        let a = gen_sn("UC");
        let b = gen_sn("UC");
        assert!(a.starts_with("UC"));
        assert_eq!(a.len(), 2 + 14 + 6);
        assert_ne!(a, b);
    }

    #[test]
    fn mask() {
        assert_eq!(mask_mobile("13812348000"), "138****8000");
        assert_eq!(mask_email("zhangsan@example.com"), "zh****@example.com");
    }

    #[test]
    fn pic_thumb_200_appends_and_is_idempotent() {
        assert_eq!(
            pic_thumb_200("https://oss.example.com/a/b.jpg"),
            "https://oss.example.com/a/b.jpg?x-oss-process=image/resize,m_pad,h_200,w_200"
        );
        // 相对路径同口径
        let once = pic_thumb_200("img/gallery/202602/a.jpeg");
        assert_eq!(
            once,
            "img/gallery/202602/a.jpeg?x-oss-process=image/resize,m_pad,h_200,w_200"
        );
        // 幂等：已带参不重复拼
        assert_eq!(pic_thumb_200(&once), once);
        // 已带其它 query：以 & 续接，保留既有参数
        assert_eq!(
            pic_thumb_200("https://oss.example.com/a.jpg?token=abc"),
            "https://oss.example.com/a.jpg?token=abc&x-oss-process=image/resize,m_pad,h_200,w_200"
        );
        // 空串原样
        assert_eq!(pic_thumb_200(""), "");
    }

    #[test]
    fn normalize_pic_thumb_cases() {
        let url = "https://oss.example.com/a.jpg";
        let sized = "https://oss.example.com/a.jpg?x-oss-process=image/resize,m_pad,h_200,w_200";
        // 缺失 → 再生
        assert_eq!(normalize_pic_thumb(url, ""), sized);
        // 裸 thumb（与原图逐字相同）→ 再生
        assert_eq!(normalize_pic_thumb(url, url), sized);
        // 已带其它尺寸参数 → 保留
        assert_eq!(
            normalize_pic_thumb(url, "https://oss.example.com/a.jpg?x-oss-process=image/resize,w_800"),
            "https://oss.example.com/a.jpg?x-oss-process=image/resize,w_800"
        );
        // 异图裸缩略 → 保留原值，不臆造
        assert_eq!(
            normalize_pic_thumb(url, "https://oss.example.com/thumb.jpg"),
            "https://oss.example.com/thumb.jpg"
        );
    }

    #[test]
    fn pic_sized_400_800_append_and_are_idempotent() {
        let url = "https://oss.example.com/a.jpg";
        // 400 档：原版字面 h_500,h_500（名实不符为原版口径，逐字保留）
        assert_eq!(
            pic_sized_400(url),
            "https://oss.example.com/a.jpg?x-oss-process=image/resize,m_pad,h_500,h_500"
        );
        // 800 档
        assert_eq!(
            pic_sized_800(url),
            "https://oss.example.com/a.jpg?x-oss-process=image/resize,m_pad,h_800,h_800"
        );
        // 幂等：已带任意 x-oss-process 时原样返回，不重复拼参
        let sized = pic_sized_800(url);
        assert_eq!(pic_sized_800(&sized), sized);
        assert_eq!(pic_sized_400(&sized), sized);
        // 已带其它 query 时以 & 续接
        assert_eq!(
            pic_sized_800("https://oss.example.com/a.jpg?token=abc"),
            "https://oss.example.com/a.jpg?token=abc&x-oss-process=image/resize,m_pad,h_800,h_800"
        );
        // 空串原样
        assert_eq!(pic_sized_400(""), "");
        assert_eq!(pic_sized_800(""), "");
    }

    #[test]
    fn sha256_known_value() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}