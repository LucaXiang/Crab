//! 图片缓存 Commands
//!
//! 提供图片路径解析和缓存管理功能。
//! 支持批量操作以减少 IPC 开销。

use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::bridge::{ClientBridge, ModeType};
use crate::core::image_cache::{
    CacheCleanupResult, ImageCacheService, ImageDownloadContext, PrefetchResult, ResolveResult,
};

/// 内部辅助: 获取当前模式的图片访问上下文
async fn get_image_context(
    bridge: &ClientBridge,
) -> Result<ImageContext, String> {
    let mode_info = bridge.get_mode_info().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    let tenant_path = tenant_manager
        .current_tenant_path()
        .ok_or("No tenant selected")?;

    match mode_info.mode {
        ModeType::Server => {
            // Server mode uses tenant path as work_dir
            let work_dir = tenant_path.clone();
            Ok(ImageContext::Server {
                tenant_path,
                work_dir,
            })
        }
        ModeType::Client => {
            // 获取 edge_url 和 http client
            let client_config = bridge
                .get_client_config()
                .await
                .ok_or("Client mode not configured")?;

            // 创建一个简单的 HTTP client 用于图片下载
            let http_client = reqwest::Client::new();

            Ok(ImageContext::Client {
                tenant_path,
                download_ctx: ImageDownloadContext {
                    edge_url: client_config.edge_url,
                    http_client,
                },
            })
        }
        ModeType::Disconnected => Err("Not connected".to_string()),
    }
}

enum ImageContext {
    Server {
        tenant_path: PathBuf,
        work_dir: PathBuf,
    },
    Client {
        tenant_path: PathBuf,
        download_ctx: ImageDownloadContext,
    },
}

/// 获取单个图片的本地路径
///
/// 如果是 Client 模式且图片未缓存，会自动下载。
#[tauri::command]
pub async fn get_image_path(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    hash: String,
) -> Result<String, String> {
    let bridge = bridge.read().await;
    let ctx = get_image_context(&bridge).await?;

    match ctx {
        ImageContext::Server { tenant_path, work_dir } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            image_cache
                .get_server_image_path(&hash, &work_dir)
                .map(|p| p.to_string_lossy().to_string())
                .map_err(|e| e.to_string())
        }
        ImageContext::Client { tenant_path, download_ctx } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            image_cache
                .get_client_image_path(&hash, &download_ctx)
                .await
                .map(|p| p.to_string_lossy().to_string())
                .map_err(|e| e.to_string())
        }
    }
}

/// 批量解析图片路径（推荐使用）
///
/// 一次调用返回所有图片的本地路径，内部懒下载。
/// 适用于产品列表等需要显示多张图片的场景。
///
/// 返回 `ResolveResult`:
/// - `paths`: hash -> 本地路径 的映射
/// - `failed`: 解析失败的 hash 列表
#[tauri::command]
pub async fn resolve_image_paths(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    hashes: Vec<String>,
) -> Result<ResolveResult, String> {
    let bridge = bridge.read().await;
    let ctx = get_image_context(&bridge).await?;

    match ctx {
        ImageContext::Server { tenant_path, work_dir } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            Ok(image_cache.resolve_server_image_paths(&hashes, &work_dir))
        }
        ImageContext::Client { tenant_path, download_ctx } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            Ok(image_cache
                .resolve_client_image_paths(&hashes, &download_ctx)
                .await)
        }
    }
}

/// 批量预加载图片
///
/// 在后台下载指定的图片到本地缓存。
/// 适用于提前加载可能需要的图片。
/// Server 模式下直接返回成功（无需预加载）。
#[tauri::command]
pub async fn prefetch_images(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    hashes: Vec<String>,
) -> Result<PrefetchResult, String> {
    let bridge = bridge.read().await;
    let ctx = get_image_context(&bridge).await?;

    match ctx {
        ImageContext::Server { .. } => {
            // Server 模式不需要预加载
            Ok(PrefetchResult {
                success_count: 0,
                failed_count: 0,
                already_cached: hashes.len() as u32,
            })
        }
        ImageContext::Client { tenant_path, download_ctx } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            image_cache
                .prefetch_images(&hashes, &download_ctx)
                .await
                .map_err(|e| e.to_string())
        }
    }
}

/// 清理不再引用的缓存图片
///
/// 传入当前活跃的图片 hash 列表，删除不在列表中的缓存。
/// 适用于产品删除或图片更换后清理旧缓存。
/// Server 模式下直接返回成功（EdgeServer 自行管理）。
#[tauri::command]
pub async fn cleanup_image_cache(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    active_hashes: Vec<String>,
) -> Result<CacheCleanupResult, String> {
    let bridge = bridge.read().await;
    let ctx = get_image_context(&bridge).await?;

    match ctx {
        ImageContext::Server { .. } => {
            // Server 模式不需要清理（EdgeServer 自行管理）
            Ok(CacheCleanupResult {
                removed_count: 0,
                freed_bytes: 0,
            })
        }
        ImageContext::Client { tenant_path, .. } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            image_cache
                .cleanup_cache(&active_hashes)
                .await
                .map_err(|e| e.to_string())
        }
    }
}
