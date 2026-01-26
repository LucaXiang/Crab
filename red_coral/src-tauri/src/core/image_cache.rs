//! ImageCacheService - 图片缓存服务
//!
//! 提供跨模式的图片访问:
//! - Server 模式: 直接使用 EdgeServer 的图片路径
//! - Client 模式: 从 EdgeServer 下载并缓存到本地
//!
//! 使用 content hash 作为文件名，天然去重。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Semaphore;

use super::bridge::BridgeError;

#[derive(Debug, Error)]
pub enum ImageCacheError {
    #[error("Bridge error: {0}")]
    Bridge(#[from] BridgeError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image not found: {0}")]
    NotFound(String),

    #[error("Invalid hash: {0}")]
    InvalidHash(String),

    #[error("Not initialized")]
    NotInitialized,

    #[error("HTTP error: {0}")]
    Http(String),
}

/// 预加载结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrefetchResult {
    pub success_count: u32,
    pub failed_count: u32,
    pub already_cached: u32,
}

/// 缓存清理结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheCleanupResult {
    pub removed_count: u32,
    pub freed_bytes: u64,
}

/// 批量解析结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolveResult {
    /// hash -> local path 映射
    pub paths: std::collections::HashMap<String, String>,
    /// 失败的 hash 列表
    pub failed: Vec<String>,
}

/// 图片下载上下文
#[derive(Clone)]
pub struct ImageDownloadContext {
    pub edge_url: String,
    pub http_client: reqwest::Client,
}

/// 图片缓存服务
pub struct ImageCacheService {
    /// Client 模式的缓存目录: {tenant}/cache/images/
    cache_images_dir: PathBuf,
}

impl ImageCacheService {
    /// 创建新的 ImageCacheService
    ///
    /// `tenant_path` 是当前租户的数据目录
    pub fn new(tenant_path: &Path) -> Self {
        // 新路径: {tenant}/cache/images/
        let cache_images_dir = tenant_path.join("cache/images");
        // 创建目录（如果不存在）
        if let Err(e) = std::fs::create_dir_all(&cache_images_dir) {
            tracing::warn!("Failed to create image cache directory: {}", e);
        }
        Self { cache_images_dir }
    }

    /// 验证 hash 格式 (SHA256 = 64 hex chars)
    fn validate_hash(hash: &str) -> Result<(), ImageCacheError> {
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ImageCacheError::InvalidHash(hash.to_string()));
        }
        Ok(())
    }

    /// 获取图片本地路径 - Server 模式
    ///
    /// 直接返回 EdgeServer 的图片路径
    /// `server_work_dir` 是 `{tenant}/server/`
    pub fn get_server_image_path(
        &self,
        hash: &str,
        server_work_dir: &Path,
    ) -> Result<PathBuf, ImageCacheError> {
        Self::validate_hash(hash)?;

        // 新路径: {tenant}/server/images/{hash}.jpg
        let path = server_work_dir
            .join("images")
            .join(format!("{}.jpg", hash));

        if !path.exists() {
            tracing::warn!(hash = %hash, "Image not found in server images");
            return Err(ImageCacheError::NotFound(hash.to_string()));
        }

        Ok(path)
    }

    /// 获取图片本地路径 - Client 模式
    ///
    /// 检查本地缓存，不存在则下载
    pub async fn get_client_image_path(
        &self,
        hash: &str,
        ctx: &ImageDownloadContext,
    ) -> Result<PathBuf, ImageCacheError> {
        Self::validate_hash(hash)?;

        let local_path = self.cache_images_dir.join(format!("{}.jpg", hash));

        if local_path.exists() {
            return Ok(local_path);
        }

        // 下载并缓存
        self.download_and_cache(hash, &local_path, ctx).await?;
        Ok(local_path)
    }

    /// 批量解析图片路径 - Server 模式
    ///
    /// `server_work_dir` 是 `{tenant}/server/`
    pub fn resolve_server_image_paths(
        &self,
        hashes: &[String],
        server_work_dir: &Path,
    ) -> ResolveResult {
        let mut paths = std::collections::HashMap::new();
        let mut failed = Vec::new();

        // 新路径: {tenant}/server/images/
        let images_base = server_work_dir.join("images");

        for hash in hashes {
            if hash.is_empty() {
                continue;
            }
            if Self::validate_hash(hash).is_err() {
                failed.push(hash.clone());
                continue;
            }

            let path = images_base.join(format!("{}.jpg", hash));
            if path.exists() {
                paths.insert(hash.clone(), path.to_string_lossy().to_string());
            } else {
                failed.push(hash.clone());
            }
        }

        ResolveResult { paths, failed }
    }

    /// 批量解析图片路径 - Client 模式
    ///
    /// 一次调用返回所有图片的本地路径，内部懒下载。
    pub async fn resolve_client_image_paths(
        &self,
        hashes: &[String],
        ctx: &ImageDownloadContext,
    ) -> ResolveResult {
        let mut paths = std::collections::HashMap::new();
        let mut failed = Vec::new();
        let mut to_download = Vec::new();

        for hash in hashes {
            if hash.is_empty() {
                continue;
            }
            if Self::validate_hash(hash).is_err() {
                failed.push(hash.clone());
                continue;
            }

            let local_path = self.cache_images_dir.join(format!("{}.jpg", hash));
            if local_path.exists() {
                paths.insert(hash.clone(), local_path.to_string_lossy().to_string());
            } else {
                to_download.push(hash.clone());
            }
        }

        // 批量下载缺失的图片
        if !to_download.is_empty() {
            let download_results = self.batch_download(&to_download, ctx).await;

            for (hash, result) in download_results {
                match result {
                    Ok(path) => {
                        paths.insert(hash, path);
                    }
                    Err(e) => {
                        tracing::warn!(hash = %hash, error = %e, "Failed to download image");
                        failed.push(hash);
                    }
                }
            }
        }

        ResolveResult { paths, failed }
    }

    /// 批量下载图片（内部方法）
    async fn batch_download(
        &self,
        hashes: &[String],
        ctx: &ImageDownloadContext,
    ) -> Vec<(String, Result<String, String>)> {
        let semaphore = Arc::new(Semaphore::new(4));
        let mut handles = Vec::new();

        for hash in hashes {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let hash = hash.clone();
            let cache_images_dir = self.cache_images_dir.clone();
            let ctx = ctx.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let local_path = cache_images_dir.join(format!("{}.jpg", hash));

                let url = format!("{}/api/image/{}.jpg", ctx.edge_url, hash);
                match ctx.http_client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                        Ok(bytes) => match tokio::fs::write(&local_path, &bytes).await {
                            Ok(_) => (hash, Ok(local_path.to_string_lossy().to_string())),
                            Err(e) => (hash, Err(e.to_string())),
                        },
                        Err(e) => (hash, Err(e.to_string())),
                    },
                    Ok(resp) => (hash, Err(format!("HTTP {}", resp.status()))),
                    Err(e) => (hash, Err(e.to_string())),
                }
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!(error = %e, "Download task panicked");
                }
            }
        }

        results
    }

    /// 批量预加载图片 (Client 模式使用)
    pub async fn prefetch_images(
        &self,
        hashes: &[String],
        ctx: &ImageDownloadContext,
    ) -> Result<PrefetchResult, ImageCacheError> {
        let mut success = 0u32;
        let mut failed = 0u32;
        let mut already_cached = 0u32;

        let semaphore = Arc::new(Semaphore::new(4));
        let mut handles = Vec::new();

        for hash in hashes {
            if Self::validate_hash(hash).is_err() {
                failed += 1;
                continue;
            }

            let local_path = self.cache_images_dir.join(format!("{}.jpg", hash));

            if local_path.exists() {
                already_cached += 1;
                continue;
            }

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let hash = hash.clone();
            let cache_images_dir = self.cache_images_dir.clone();
            let ctx = ctx.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let local_path = cache_images_dir.join(format!("{}.jpg", hash));

                let url = format!("{}/api/image/{}.jpg", ctx.edge_url, hash);
                match ctx.http_client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                        Ok(bytes) => match tokio::fs::write(&local_path, &bytes).await {
                            Ok(_) => (hash, Ok(())),
                            Err(e) => (hash, Err(e.to_string())),
                        },
                        Err(e) => (hash, Err(e.to_string())),
                    },
                    Ok(resp) => (hash, Err(format!("HTTP {}", resp.status()))),
                    Err(e) => (hash, Err(e.to_string())),
                }
            }));
        }

        for handle in handles {
            match handle.await {
                Ok((_, Ok(()))) => success += 1,
                Ok((hash, Err(e))) => {
                    tracing::warn!(hash = %hash, error = %e, "Failed to prefetch image");
                    failed += 1;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Prefetch task panicked");
                    failed += 1;
                }
            }
        }

        Ok(PrefetchResult {
            success_count: success,
            failed_count: failed,
            already_cached,
        })
    }

    /// 清理不再引用的缓存图片
    pub async fn cleanup_cache(
        &self,
        active_hashes: &[String],
    ) -> Result<CacheCleanupResult, ImageCacheError> {
        let active_set: HashSet<_> = active_hashes.iter().collect();
        let mut removed_count = 0u32;
        let mut freed_bytes = 0u64;

        let mut entries = match tokio::fs::read_dir(&self.cache_images_dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read image cache directory: {}", e);
                return Ok(CacheCleanupResult {
                    removed_count: 0,
                    freed_bytes: 0,
                });
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let filename = entry.file_name().to_string_lossy().to_string();

            if let Some(hash) = filename.strip_suffix(".jpg") {
                if !active_set.contains(&hash.to_string()) {
                    if let Ok(metadata) = entry.metadata().await {
                        freed_bytes += metadata.len();
                    }
                    if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                        tracing::warn!(hash = %hash, error = %e, "Failed to remove cached image");
                    } else {
                        removed_count += 1;
                        tracing::debug!(hash = %hash, "Removed orphan cached image");
                    }
                }
            }
        }

        tracing::info!(
            removed = removed_count,
            freed_bytes = freed_bytes,
            "Image cache cleanup completed"
        );

        Ok(CacheCleanupResult {
            removed_count,
            freed_bytes,
        })
    }

    // ============ 内部方法 ============

    /// 下载图片并缓存到本地
    async fn download_and_cache(
        &self,
        hash: &str,
        local_path: &Path,
        ctx: &ImageDownloadContext,
    ) -> Result<(), ImageCacheError> {
        let url = format!("{}/api/image/{}.jpg", ctx.edge_url, hash);

        let resp = ctx
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ImageCacheError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ImageCacheError::NotFound(format!(
                "{} (HTTP {})",
                hash,
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ImageCacheError::Http(e.to_string()))?;

        // 确保目录存在
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(local_path, &bytes).await?;
        tracing::debug!(hash = %hash, "Image downloaded and cached");

        Ok(())
    }
}
