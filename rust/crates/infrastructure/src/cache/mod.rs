//! 缓存服务：Redis 优先，连接失败时自动降级为进程内内存缓存。
//!
//! 对应原 Java Redisson / Spring Data Redis。内存降级仅适用于单实例开发环境
//! （验证码、登录态缓存等场景），生产多实例必须使用 Redis。
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use nextrade_core::{Error, Result};

type MemoryStore = Arc<Mutex<HashMap<String, (String, Option<Instant>)>>>;

#[derive(Clone)]
enum Backend {
    Redis(ConnectionManager),
    Memory(MemoryStore),
}

static GLOBAL_CACHE: std::sync::OnceLock<CacheService> = std::sync::OnceLock::new();

/// 获取全局缓存实例（未显式初始化时自动使用内存降级缓存）。
pub fn get_cache() -> &'static CacheService {
    GLOBAL_CACHE.get_or_init(|| CacheService {
        backend: Backend::Memory(Arc::new(Mutex::new(HashMap::new()))),
    })
}

/// 异步缓存客户端（Redis 或内存降级）。
#[derive(Clone)]
pub struct CacheService {
    backend: Backend,
}

impl CacheService {
    /// 初始化：优先 Redis；连接失败降级内存缓存（进程内，单实例）。
    ///
    /// # Errors
    /// 仅当构建 Redis 客户端 URL 非法且内存分配失败等不可恢复场景；连接失败不报错。
    pub async fn init(url: &str) -> Result<Self> {
        let instance = match Self::try_redis(url).await {
            Ok(service) => {
                tracing::info!("Redis 缓存初始化完成: {url}");
                service
            }
            Err(e) => {
                tracing::warn!("Redis 不可用（{e}），降级为进程内内存缓存（单实例开发模式）");
                Self {
                    backend: Backend::Memory(Arc::new(Mutex::new(HashMap::new()))),
                }
            }
        };
        let _ = GLOBAL_CACHE.set(instance.clone());
        Ok(instance)
    }

    async fn try_redis(url: &str) -> Result<Self> {
        let client = Client::open(url).map_err(|e| Error::Cache(format!("Redis URL 无效: {e}")))?;
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| Error::Cache(format!("Redis 连接失败: {e}")))?;
        Ok(Self {
            backend: Backend::Redis(conn),
        })
    }

    /// 是否处于内存降级模式。
    #[must_use]
    pub fn is_memory(&self) -> bool {
        matches!(self.backend, Backend::Memory(_))
    }

    /// GET（JSON 反序列化）。
    ///
    /// # Errors
    /// 缓存读取/反序列化失败。
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let Some(raw) = self.get_str(key).await? else {
            return Ok(None);
        };
        serde_json::from_str(&raw)
            .map(Some)
            .map_err(|e| Error::Cache(format!("缓存值反序列化失败[{key}]: {e}")))
    }

    /// SET（JSON 序列化 + TTL 秒）。
    ///
    /// # Errors
    /// 序列化/写入失败。
    pub async fn set_json<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: u64,
    ) -> Result<()> {
        let payload = serde_json::to_string(value)
            .map_err(|e| Error::Cache(format!("缓存序列化失败: {e}")))?;
        self.set_str(key, &payload, ttl_secs).await
    }

    /// 原始字符串 GET。
    pub async fn get_str(&self, key: &str) -> Result<Option<String>> {
        match &self.backend {
            Backend::Redis(conn) => {
                let mut conn = conn.clone();
                conn.get(key).await.map_err(|e| Error::Cache(e.to_string()))
            }
            Backend::Memory(store) => {
                let mut guard = store
                    .lock()
                    .map_err(|_| Error::Cache("内存缓存锁失败".into()))?;
                match guard.get(key) {
                    Some((value, expire_at)) => {
                        if let Some(at) = expire_at {
                            if Instant::now() >= *at {
                                guard.remove(key);
                                return Ok(None);
                            }
                        }
                        Ok(Some(value.clone()))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    /// 原始字符串 SET（`ttl_secs=0` 表示不过期）。
    pub async fn set_str(&self, key: &str, value: &str, ttl_secs: u64) -> Result<()> {
        match &self.backend {
            Backend::Redis(conn) => {
                let mut conn = conn.clone();
                if ttl_secs == 0 {
                    conn.set::<_, _, ()>(key, value)
                        .await
                        .map_err(|e| Error::Cache(e.to_string()))
                } else {
                    conn.set_ex::<_, _, ()>(key, value, ttl_secs)
                        .await
                        .map_err(|e| Error::Cache(e.to_string()))
                }
            }
            Backend::Memory(store) => {
                let mut guard = store
                    .lock()
                    .map_err(|_| Error::Cache("内存缓存锁失败".into()))?;
                let expire_at = if ttl_secs == 0 {
                    None
                } else {
                    Some(Instant::now() + Duration::from_secs(ttl_secs))
                };
                guard.insert(key.to_string(), (value.to_string(), expire_at));
                Ok(())
            }
        }
    }

    /// 删除 key。
    pub async fn del(&self, key: &str) -> Result<()> {
        match &self.backend {
            Backend::Redis(conn) => {
                let mut conn = conn.clone();
                conn.del::<_, ()>(key)
                    .await
                    .map_err(|e| Error::Cache(e.to_string()))
            }
            Backend::Memory(store) => {
                let mut guard = store
                    .lock()
                    .map_err(|_| Error::Cache("内存缓存锁失败".into()))?;
                guard.remove(key);
                Ok(())
            }
        }
    }

    /// 「读缓存 → 未命中回源 → 写缓存」。
    ///
    /// # Errors
    /// 回源失败。
    pub async fn get_or_load<T, F, Fut>(&self, key: &str, ttl_secs: u64, loader: F) -> Result<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        if let Some(hit) = self.get_json::<T>(key).await? {
            return Ok(hit);
        }
        let value = loader().await?;
        if let Err(e) = self.set_json(key, &value, ttl_secs).await {
            tracing::warn!("写入缓存失败[{key}]: {e}");
        }
        Ok(value)
    }

    /// 原子自增（验证码/限购计数等）。
    ///
    /// # Errors
    /// 缓存读写失败。
    pub async fn incr(&self, key: &str, ttl_secs: u64) -> Result<i64> {
        match &self.backend {
            Backend::Redis(conn) => {
                let mut conn = conn.clone();
                let n: i64 = conn
                    .incr(key, 1i64)
                    .await
                    .map_err(|e| Error::Cache(e.to_string()))?;
                if n == 1 && ttl_secs > 0 {
                    let _: bool = conn
                        .expire(key, ttl_secs as i64)
                        .await
                        .map_err(|e| Error::Cache(e.to_string()))?;
                }
                Ok(n)
            }
            Backend::Memory(store) => {
                let mut guard = store
                    .lock()
                    .map_err(|_| Error::Cache("内存缓存锁失败".into()))?;
                let current = guard
                    .get(key)
                    .and_then(|(v, _)| v.parse::<i64>().ok())
                    .unwrap_or(0);
                let next = current + 1;
                let expire_at = guard.get(key).and_then(|(_, at)| *at).or_else(|| {
                    if ttl_secs == 0 {
                        None
                    } else {
                        Some(Instant::now() + Duration::from_secs(ttl_secs))
                    }
                });
                guard.insert(key.to_string(), (next.to_string(), expire_at));
                Ok(next)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_cache() -> CacheService {
        CacheService {
            backend: Backend::Memory(Arc::new(Mutex::new(HashMap::new()))),
        }
    }

    #[tokio::test]
    async fn memory_roundtrip_and_del() {
        let cache = memory_cache();
        assert!(cache.is_memory());
        cache.set_str("k1", "v1", 0).await.unwrap();
        assert_eq!(cache.get_str("k1").await.unwrap().as_deref(), Some("v1"));
        cache.del("k1").await.unwrap();
        assert!(cache.get_str("k1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn memory_ttl_expires() {
        let cache = memory_cache();
        cache.set_str("k", "v", 1).await.unwrap();
        assert_eq!(cache.get_str("k").await.unwrap().as_deref(), Some("v"));
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(cache.get_str("k").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn memory_json_and_incr() {
        let cache = memory_cache();
        cache.set_json("obj", &vec![1, 2, 3], 60).await.unwrap();
        let got: Option<Vec<i32>> = cache.get_json("obj").await.unwrap();
        assert_eq!(got, Some(vec![1, 2, 3]));
        assert_eq!(cache.incr("counter", 60).await.unwrap(), 1);
        assert_eq!(cache.incr("counter", 60).await.unwrap(), 2);
    }
}
