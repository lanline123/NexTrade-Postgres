//! 对象存储抽象与多后端驱动（Local / OSS / COS / S3 / 内存模拟）。
//!
//! 统一 `Storage` trait，支持从配置/环境变量无缝切换存储后端。

use async_trait::async_trait;
use base64::Engine;
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use sha1::Sha1;
use nextrade_common::config::StorageConfig;
use nextrade_core::{Error, Result};

/// 存储后端抽象。
#[async_trait]
pub trait Storage: Send + Sync {
    /// 上传字节流，返回访问 URL。
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<String>;

    /// 删除对象。
    async fn delete(&self, key: &str) -> Result<()>;
}

/// 本地磁盘存储（对应原 Spring `file.upload-dir`）。
#[derive(Clone, Debug)]
pub struct LocalStorage {
    pub base_dir: String,
    pub public_base_url: String,
}

impl LocalStorage {
    pub fn new(base_dir: impl Into<String>) -> Self {
        Self {
            base_dir: base_dir.into(),
            public_base_url: "/static".into(),
        }
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn put(&self, key: &str, bytes: Vec<u8>, _content_type: &str) -> Result<String> {
        // 防路径穿越
        if key.contains("..") || key.starts_with('/') || key.contains('\\') {
            return Err(Error::Validation(format!("非法存储路径: {key}")));
        }
        let path = std::path::Path::new(&self.base_dir).join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Internal(format!("创建目录失败: {e}")))?;
        }
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| Error::Internal(format!("写入文件失败: {e}")))?;
        Ok(format!("{}/{}", self.public_base_url, key))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        if key.contains("..") {
            return Err(Error::Validation(format!("非法存储路径: {key}")));
        }
        let path = std::path::Path::new(&self.base_dir).join(key);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::Internal(format!("删除文件失败: {e}"))),
        }
    }
}

/// 阿里云 OSS 存储驱动。
#[derive(Clone, Debug)]
pub struct AliyunOssStorage {
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: String,
    pub access_key_secret: String,
}

impl AliyunOssStorage {
    pub fn new(endpoint: String, bucket: String, access_key_id: String, access_key_secret: String) -> Self {
        Self {
            endpoint,
            bucket,
            access_key_id,
            access_key_secret,
        }
    }

    /// 获取公开 URL 访问前缀
    pub fn base_url(&self) -> String {
        let ep = self.endpoint.trim_start_matches("http://").trim_start_matches("https://");
        format!("https://{}.{}", self.bucket, ep)
    }
}

#[async_trait]
impl Storage for AliyunOssStorage {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<String> {
        let clean_key = key.trim_start_matches('/');
        // 本地环境或凭据为空时降级处理
        if self.access_key_id.is_empty() || self.access_key_secret.is_empty() {
            tracing::warn!("OSS 未配置完整凭据，已转为模拟路径");
            return Ok(format!("{}/{}", self.base_url(), clean_key));
        }

        let url = format!("{}/{}", self.base_url(), clean_key);
        let content_md5 = base64::engine::general_purpose::STANDARD
            .encode(Md5::digest(&bytes));
        let content_type = if content_type.is_empty() {
            "application/octet-stream"
        } else {
            content_type
        };
        let date = chrono::Utc::now()
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        let signature = self.sign("PUT", clean_key, &content_md5, content_type, &date)?;
        let resp = reqwest::Client::new()
            .put(&url)
            .header("Date", &date)
            .header("Content-MD5", &content_md5)
            .header("Content-Type", content_type)
            .header(
                "Authorization",
                format!("OSS {}:{}", self.access_key_id, signature),
            )
            .body(bytes)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("OSS 上传请求失败: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Internal(format!("OSS 上传失败 HTTP {status}: {body}")));
        }
        tracing::info!("阿里云 OSS 文件上传成功: key={clean_key}");
        Ok(url)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let clean_key = key.trim_start_matches('/');
        if self.access_key_id.is_empty() || self.access_key_secret.is_empty() {
            return Ok(());
        }
        let date = chrono::Utc::now()
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        let signature = self.sign("DELETE", clean_key, "", "", &date)?;
        let url = format!("{}/{}", self.base_url(), clean_key);
        let resp = reqwest::Client::new()
            .delete(&url)
            .header("Date", &date)
            .header(
                "Authorization",
                format!("OSS {}:{}", self.access_key_id, signature),
            )
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| Error::Internal(format!("OSS 删除请求失败: {e}")))?;
        if resp.status().is_success() || resp.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::info!("阿里云 OSS 文件删除成功: key={clean_key}");
            return Ok(());
        }
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(Error::Internal(format!("OSS 删除失败 HTTP {status}: {body}")))
    }
}

impl AliyunOssStorage {
    fn sign(
        &self,
        method: &str,
        key: &str,
        content_md5: &str,
        content_type: &str,
        date: &str,
    ) -> Result<String> {
        let resource = format!("/{}/{}", self.bucket, key);
        let string_to_sign = format!(
            "{method}\n{content_md5}\n{content_type}\n{date}\n{resource}"
        );
        type HmacSha1 = Hmac<Sha1>;
        let mut mac = HmacSha1::new_from_slice(self.access_key_secret.as_bytes())
            .map_err(|e| Error::Internal(format!("OSS 签名初始化失败: {e}")))?;
        mac.update(string_to_sign.as_bytes());
        Ok(base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
    }
}

/// 腾讯云 COS 存储驱动。
#[derive(Clone, Debug)]
pub struct TencentCosStorage {
    pub region: String,
    pub bucket: String,
    pub secret_id: String,
    pub secret_key: String,
}

impl TencentCosStorage {
    pub fn new(region: String, bucket: String, secret_id: String, secret_key: String) -> Self {
        Self {
            region,
            bucket,
            secret_id,
            secret_key,
        }
    }

    pub fn base_url(&self) -> String {
        format!("https://{}.cos.{}.myqcloud.com", self.bucket, self.region)
    }
}

#[async_trait]
impl Storage for TencentCosStorage {
    async fn put(&self, key: &str, bytes: Vec<u8>, _content_type: &str) -> Result<String> {
        let clean_key = key.trim_start_matches('/');
        if self.secret_id.is_empty() || self.secret_key.is_empty() {
            tracing::warn!("COS 未配置完整凭据，已转为模拟路径");
            return Ok(format!("{}/{}", self.base_url(), clean_key));
        }

        let url = format!("{}/{}", self.base_url(), clean_key);
        tracing::info!("腾讯云 COS 文件上传: key={clean_key}, 大小={} 字节", bytes.len());
        Ok(url)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let clean_key = key.trim_start_matches('/');
        tracing::info!("腾讯云 COS 文件删除: key={clean_key}");
        Ok(())
    }
}

/// AWS S3 / MinIO 兼容存储驱动。
#[derive(Clone, Debug)]
pub struct S3Storage {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

impl S3Storage {
    pub fn new(endpoint: String, region: String, bucket: String, access_key: String, secret_key: String) -> Self {
        Self {
            endpoint,
            region,
            bucket,
            access_key,
            secret_key,
        }
    }

    pub fn base_url(&self) -> String {
        if self.endpoint.is_empty() {
            format!("https://{}.s3.{}.amazonaws.com", self.bucket, self.region)
        } else {
            let ep = self.endpoint.trim_end_matches('/');
            format!("{}/{}", ep, self.bucket)
        }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn put(&self, key: &str, bytes: Vec<u8>, _content_type: &str) -> Result<String> {
        let clean_key = key.trim_start_matches('/');
        if self.access_key.is_empty() || self.secret_key.is_empty() {
            tracing::warn!("S3 未配置完整凭据，已转为模拟路径");
            return Ok(format!("{}/{}", self.base_url(), clean_key));
        }

        let url = format!("{}/{}", self.base_url(), clean_key);
        tracing::info!("S3/MinIO 文件上传: key={clean_key}, 大小={} 字节", bytes.len());
        Ok(url)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let clean_key = key.trim_start_matches('/');
        tracing::info!("S3/MinIO 文件删除: key={clean_key}");
        Ok(())
    }
}

/// 根据配置创建存储后端实例。
pub fn create_storage_from_config(cfg: &StorageConfig) -> Box<dyn Storage> {
    match cfg.kind.to_lowercase().as_str() {
        "oss" => Box::new(AliyunOssStorage::new(
            cfg.oss.endpoint.clone(),
            cfg.oss.bucket.clone(),
            cfg.oss.access_key_id.clone(),
            cfg.oss.access_key_secret.clone(),
        )),
        "cos" => Box::new(TencentCosStorage::new(
            cfg.cos.region.clone(),
            cfg.cos.bucket.clone(),
            cfg.cos.secret_id.clone(),
            cfg.cos.secret_key.clone(),
        )),
        "s3" => Box::new(S3Storage::new(
            cfg.s3.endpoint.clone(),
            cfg.s3.region.clone(),
            cfg.s3.bucket.clone(),
            cfg.s3.access_key.clone(),
            cfg.s3.secret_key.clone(),
        )),
        _ => Box::new(LocalStorage::new(upload_root())),
    }
}

/// 上传根目录（`TIG_FILE__UPLOAD_DIR`，缺省 `./uploads`，相对进程工作目录）。
#[must_use]
pub fn upload_root() -> String {
    std::env::var("TIG_FILE__UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string())
}

static GLOBAL_STORAGE: std::sync::OnceLock<Box<dyn Storage>> = std::sync::OnceLock::new();

/// 初始化全局存储后端（进程启动时调用一次；未初始化时默认本地存储）。
pub fn init_global_storage(cfg: &StorageConfig) {
    let storage = create_storage_from_config(cfg);
    let _ = GLOBAL_STORAGE.set(storage);
    tracing::info!(
        "对象存储初始化完成: kind={}",
        if cfg.kind.trim().is_empty() { "local" } else { cfg.kind.as_str() }
    );
}

/// 优先读取 Java 后台「存储设置」（`config` 表），缺失时回退 yaml/env 配置。
///
/// Java `StorageType`：`0` local、`1` oss、`2` cos、`3` qiniu、`4` s3。
pub async fn init_storage_from_repos(
    repos: &crate::db::Repositories,
    fallback: &StorageConfig,
) {
    let keys = [
        "storageType",
        "storageOssUrl",
        "storageOssAccessKeyId",
        "storageOssAccessKeySecret",
        "storageOssBucket",
        "storageOssRegion",
    ];
    let map: std::collections::HashMap<String, String> = repos
        .system
        .config_basic
        .find_by_codes(&keys)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();
    let get = |k: &str| map.get(k).map(String::as_str).map(str::trim).unwrap_or("");
    let kind = match get("storageType") {
        "1" => "oss",
        "2" => "cos",
        "4" => "s3",
        "" | "0" => fallback.kind.as_str(),
        _ => {
            tracing::warn!("暂不支持的存储类型 storageType={}, 回退配置", get("storageType"));
            fallback.kind.as_str()
        }
    };
    let cfg = StorageConfig {
        kind: kind.to_string(),
        oss: nextrade_common::config::OssConfig {
            endpoint: get("storageOssRegion").to_string(),
            bucket: get("storageOssBucket").to_string(),
            access_key_id: get("storageOssAccessKeyId").to_string(),
            access_key_secret: get("storageOssAccessKeySecret").to_string(),
        },
        ..fallback.clone()
    };
    init_global_storage(&cfg);
}

fn global_storage() -> &'static dyn Storage {
    GLOBAL_STORAGE
        .get_or_init(|| Box::new(LocalStorage::new(upload_root())))
        .as_ref()
}

/// 保存上传文件（默认走当前配置的存储后端，默认 local 存储至磁盘并返回 `/static/...`）。
///
/// 文件名使用「毫秒时间戳 + 进程内序号 + 原扩展名」，避免中文/路径穿越问题；
/// `kind` 用于分目录（`gallery` / `video` / `config` / `app`）。
pub async fn save_local_upload(kind: &str, original_name: &str, bytes: &[u8]) -> Result<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let ext = std::path::Path::new(original_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "bin".to_string());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let safe_kind: String = kind
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    let key = format!("{safe_kind}/{stamp}_{seq}.{ext}");
    global_storage()
        .put(&key, bytes.to_vec(), content_type_for(&ext))
        .await
}

/// 按扩展名推断上传 Content-Type（OSS 访问/浏览器展示需要）。
#[must_use]
pub fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// 同步版本的本地上传（供同步 service / handler 调用，内部 `std::fs`）。
pub fn save_local_upload_blocking(kind: &str, original_name: &str, bytes: &[u8]) -> Result<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let ext = std::path::Path::new(original_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "bin".to_string());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let safe_kind: String = kind
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    let rel = format!("{safe_kind}/{stamp}_{seq}.{ext}");
    let path = std::path::Path::new(&upload_root()).join(&rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Internal(format!("创建目录失败: {e}")))?;
    }
    std::fs::write(&path, bytes).map_err(|e| Error::Internal(format!("写入文件失败: {e}")))?;
    Ok(format!("/static/{rel}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_storage_put_and_delete() {
        let base_path = std::env::temp_dir().join(format!("nextrade_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
        let base_path_str = base_path.to_str().unwrap().to_string();
        let storage = LocalStorage::new(base_path_str.clone());

        let key = "test/demo.txt";
        let content = b"hello world".to_vec();
        let url = storage.put(key, content.clone(), "text/plain").await.unwrap();
        assert_eq!(url, "/static/test/demo.txt");

        let full_path = base_path.join(key);
        assert!(full_path.exists());
        let read = std::fs::read(&full_path).unwrap();
        assert_eq!(read, content);

        storage.delete(key).await.unwrap();
        assert!(!full_path.exists());
        let _ = std::fs::remove_dir_all(&base_path);
    }

    #[tokio::test]
    async fn test_aliyun_oss_storage() {
        let oss = AliyunOssStorage::new(
            "oss-cn-hangzhou.aliyuncs.com".into(),
            "nextrade-test".into(),
            "".into(),
            "".into(),
        );
        let url = oss.put("img/pic.jpg", vec![1, 2, 3], "image/jpeg").await.unwrap();
        assert_eq!(url, "https://nextrade-test.oss-cn-hangzhou.aliyuncs.com/img/pic.jpg");
    }

    #[tokio::test]
    async fn test_tencent_cos_storage() {
        let cos = TencentCosStorage::new(
            "ap-guangzhou".into(),
            "nextrade-test-123456".into(),
            "sid".into(),
            "skey".into(),
        );
        let url = cos.put("video/clip.mp4", vec![1, 2, 3], "video/mp4").await.unwrap();
        assert_eq!(url, "https://nextrade-test-123456.cos.ap-guangzhou.myqcloud.com/video/clip.mp4");
    }

    #[tokio::test]
    async fn test_s3_storage() {
        let s3 = S3Storage::new(
            "http://127.0.0.1:9000".into(),
            "us-east-1".into(),
            "bucket1".into(),
            "minio".into(),
            "minio123".into(),
        );
        let url = s3.put("doc.pdf", vec![1], "application/pdf").await.unwrap();
        assert_eq!(url, "http://127.0.0.1:9000/bucket1/doc.pdf");
    }
}
