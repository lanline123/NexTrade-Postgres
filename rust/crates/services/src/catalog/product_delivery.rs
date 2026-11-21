//! 商品文件型交付服务（我方新增）：商家附件管理 + 买家令牌下载。
//!
//! 访问控制全部在接口层强校验，不依赖前端隐藏：
//! - 商家侧：商品归属作用域校验（与商品详情同口径：商家只能管本店
//!   商品，平台商品 shop_id=0 仅平台可管）；
//! - 买家侧：申领令牌时校验订单归属 + 支付/发货门（`delivery_access`），
//!   令牌绑订单 + 用户 + 文件落库，下载端点原子核销次数与有效期后
//!   才放行到存储层（OSS 限时签名 URL 跳转 / 本地直读）。

use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::entities::Product;
use nextrade_core::models::catalog::product_delivery::{
    delivery_access, display_file_name, token_state, validate_delivery_file, DeliveryFileVO,
    DownloadGrantVO, RequestDownloadDTO, TokenState, DELIVERY_PRODUCT_TYPES,
    MAX_FILES_PER_PRODUCT, SIGNED_URL_TTL_SECONDS, TOKEN_MAX_DOWNLOADS, TOKEN_TTL_SECONDS,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::catalog::product_delivery::DeliveryFileRow;
use nextrade_infrastructure::db::Repositories;
use nextrade_infrastructure::storage;

pub struct ProductDeliveryService {
    repos: Arc<Repositories>,
}

/// 下载放行结果（路由层据此组装响应）。
pub enum DeliveryDownload {
    /// 存储层限时签名 URL（302 跳转）。
    Redirect(String),
    /// 服务端直读字节（本地存储后端）。
    Bytes {
        bytes: Vec<u8>,
        file_name: String,
        content_type: &'static str,
    },
}

impl ProductDeliveryService {
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::catalog::product_delivery::ProductDeliveryRepository {
        &self.repos.catalog.product_delivery
    }

    /// 商家作用域校验（与管理端商品详情同口径）：`scope = None` 为平台
    /// 不限；商家只能操作本店商品，平台商品（shop_id = 0）仅平台可操作。
    fn ensure_scope(product: &Product, scope: Option<i32>) -> Result<()> {
        if let Some(s) = scope {
            if product.shop_id != s {
                return Err(Error::Forbidden(
                    if product.shop_id == 0 {
                        "无权操作平台商品"
                    } else {
                        "无权操作该商品"
                    }
                    .into(),
                ));
            }
        }
        Ok(())
    }

    async fn load_product(&self, product_id: i32) -> Result<Product> {
        self.repos
            .catalog
            .product_manage
            .find_product(product_id)
            .await?
            .ok_or_else(|| Error::NotFound("商品不存在".into()))
    }

    fn ensure_deliverable_type(product: &Product) -> Result<i32> {
        let product_type = product.product_type.unwrap_or(0);
        if !DELIVERY_PRODUCT_TYPES.contains(&product_type) {
            return Err(Error::Validation(
                "仅虚拟商品与付费内容商品可添加交付附件".into(),
            ));
        }
        Ok(product_type)
    }

    // ———————————————————— 商家侧 ————————————————————

    /// 商家附件列表。
    pub async fn admin_list(
        &self,
        product_id: i32,
        scope: Option<i32>,
    ) -> Result<Vec<DeliveryFileVO>> {
        let product = self.load_product(product_id).await?;
        Self::ensure_scope(&product, scope)?;
        let rows = self.repo().list_by_product(product_id).await?;
        Ok(rows.iter().map(row_to_vo).collect())
    }

    /// 商家上传附件：类型门 + 扩展名/大小校验 + 每商品上限，对象落
    /// 存储后端（key 不可枚举），元数据落库。
    pub async fn admin_upload(
        &self,
        product_id: i32,
        scope: Option<i32>,
        file_name: &str,
        bytes: Vec<u8>,
    ) -> Result<DeliveryFileVO> {
        let product = self.load_product(product_id).await?;
        Self::ensure_scope(&product, scope)?;
        Self::ensure_deliverable_type(&product)?;
        let ext = validate_delivery_file(file_name, bytes.len() as i64)?;
        let count = self.repo().count_by_product(product_id).await?;
        if count >= MAX_FILES_PER_PRODUCT {
            return Err(Error::Validation(format!(
                "每件商品最多添加 {MAX_FILES_PER_PRODUCT} 个附件"
            )));
        }
        let key = format!(
            "delivery/{product_id}/{}.{}",
            random_hex(16),
            ext
        );
        storage::put_delivery_object(&key, &bytes, storage::content_type_for(&ext)).await?;
        let now = utils::now_seconds() as i32;
        let display_name = display_file_name(file_name);
        let id = self
            .repo()
            .insert_file(
                product_id,
                product.shop_id,
                &display_name,
                &key,
                &ext,
                bytes.len() as i64,
                count as i32,
                now,
            )
            .await?;
        Ok(DeliveryFileVO {
            file_id: id,
            product_id,
            file_name: display_name,
            file_ext: ext,
            file_size: bytes.len() as i64,
            add_time: now,
        })
    }

    /// 商家删除附件（软删 + 尽力清理存储对象）。
    pub async fn admin_delete(&self, file_id: i32, scope: Option<i32>) -> Result<()> {
        let row = self
            .repo()
            .find_file(file_id)
            .await?
            .ok_or_else(|| Error::NotFound("附件不存在".into()))?;
        let product = self.load_product(row.1).await?;
        Self::ensure_scope(&product, scope)?;
        let now = utils::now_seconds() as i32;
        let hit = self.repo().soft_delete_file(file_id, row.1, now).await?;
        if !hit {
            return Err(Error::NotFound("附件不存在".into()));
        }
        storage::delete_delivery_object(&row.4).await;
        Ok(())
    }

    // ———————————————————— 买家侧 ————————————————————

    /// 申领下载令牌：强校验订单归属与交付门后签发（绑订单 + 用户 + 文件）。
    pub async fn request_download(
        &self,
        user_id: i32,
        dto: &RequestDownloadDTO,
    ) -> Result<DownloadGrantVO> {
        if dto.file_id <= 0 {
            return Err(Error::Validation("附件参数缺失".into()));
        }
        let row = self
            .repo()
            .find_file(dto.file_id)
            .await?
            .ok_or_else(|| Error::NotFound("附件不存在或已删除".into()))?;
        let product = self.load_product(row.1).await?;
        let product_type = Self::ensure_deliverable_type(&product)?;

        let order_id = if let Some(oid) = dto.order_id.filter(|v| *v > 0) {
            let ctx = self
                .repo()
                .find_order_context(oid, row.1)
                .await?
                .ok_or_else(|| Error::Forbidden("订单不存在或不包含该商品".into()))?;
            if ctx.1 != user_id {
                return Err(Error::Forbidden("无权下载该订单的交付文件".into()));
            }
            if !delivery_access(product_type, ctx.2, ctx.3) {
                return Err(Error::Forbidden(download_block_reason(product_type, ctx.2, ctx.3).into()));
            }
            oid
        } else {
            // 商品页申领：取本人最近一张符合交付门的已付订单绑单
            let ctx = self
                .repo()
                .find_latest_paid_order(user_id, row.1, product_type == 2)
                .await?
                .ok_or_else(|| {
                    Error::Forbidden(
                        if product_type == 2 {
                            "未找到已发货的购买订单，暂不可下载"
                        } else {
                            "未找到已支付的购买订单，暂不可下载"
                        }
                        .into(),
                    )
                })?;
            ctx.0
        };

        let now = utils::now_seconds() as i32;
        let expires_at = now + TOKEN_TTL_SECONDS;
        let token = random_hex(32);
        self.repo()
            .insert_token(
                &token,
                order_id,
                user_id,
                row.0,
                row.1,
                TOKEN_MAX_DOWNLOADS,
                expires_at,
                now,
            )
            .await?;
        Ok(DownloadGrantVO {
            download_url: format!("/api/product/delivery/download?token={token}"),
            expires_at,
            file_name: row.3.clone(),
            file_size: row.6,
        })
    }

    /// 凭令牌下载：原子核销（有效期 + 次数）后放行到存储层。
    pub async fn download(&self, token: &str) -> Result<DeliveryDownload> {
        let token = token.trim();
        if token.is_empty() {
            return Err(Error::Validation("下载令牌缺失".into()));
        }
        let now = utils::now_seconds() as i32;
        let existing = self
            .repo()
            .find_token(token)
            .await?
            .ok_or_else(|| Error::NotFound("下载链接无效".into()))?;
        match token_state(existing.8, existing.7, existing.6, now) {
            TokenState::Expired => {
                return Err(Error::biz(410, "下载链接已过期，请重新获取"));
            }
            TokenState::Exhausted => {
                return Err(Error::biz(410, "下载次数已用完，请重新获取"));
            }
            TokenState::Usable => {}
        }
        // 原子核销：并发下只有计数未满的请求能通过
        let consumed = self.repo().consume_token(token, now).await?;
        if consumed.is_none() {
            return Err(Error::biz(410, "下载链接已失效，请重新获取"));
        }
        let file = self
            .repo()
            .find_file(existing.4)
            .await?
            .ok_or_else(|| Error::NotFound("附件已被删除".into()))?;
        if let Some(url) =
            storage::delivery_signed_url(&file.4, i64::from(now) + SIGNED_URL_TTL_SECONDS)
        {
            return Ok(DeliveryDownload::Redirect(url));
        }
        let bytes = storage::read_delivery_object(&file.4)
            .await?
            .ok_or_else(|| Error::NotFound("附件文件不存在".into()))?;
        Ok(DeliveryDownload::Bytes {
            bytes,
            file_name: file.3.clone(),
            content_type: storage::content_type_for(&file.5),
        })
    }
}

/// 未达交付门时的拦截文案（按未付 / 未发货分流）。
fn download_block_reason(product_type: i32, pay_status: i32, shipping_status: i32) -> &'static str {
    if pay_status != 2 {
        "订单未支付，暂不可下载"
    } else if product_type == 2 && shipping_status == 0 {
        "商品尚未发货，暂不可下载"
    } else {
        "当前订单暂不可下载该附件"
    }
}

pub(crate) fn row_to_vo(r: &DeliveryFileRow) -> DeliveryFileVO {
    DeliveryFileVO {
        file_id: r.0,
        product_id: r.1,
        file_name: r.3.clone(),
        file_ext: r.5.clone(),
        file_size: r.6,
        add_time: r.8,
    }
}

/// 生成不可猜的十六进制随机串（令牌 / 对象 key 摘要共用）。
/// 优先 /dev/urandom；不可用时以时间 + 进程 + 序号哈希展开兜底。
fn random_hex(bytes_len: usize) -> String {
    use std::io::Read;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let mut buf = vec![0u8; bytes_len];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok();
    if !filled {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for chunk in buf.chunks_mut(8) {
            let mut hasher = DefaultHasher::new();
            (nanos, std::process::id(), seq, chunk.len()).hash(&mut hasher);
            let v = hasher.finish().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_hex_shape() {
        let t = random_hex(32);
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(random_hex(32), random_hex(32));
    }

    #[test]
    fn block_reason_priority() {
        assert_eq!(download_block_reason(4, 1, 0), "订单未支付，暂不可下载");
        assert_eq!(download_block_reason(2, 2, 0), "商品尚未发货，暂不可下载");
        assert_eq!(download_block_reason(2, 2, 1), "当前订单暂不可下载该附件");
    }
}
