//! 图片缓存 Commands
//!
//! 提供图片路径解析、缓存管理和图片保存功能。
//! 支持批量操作以减少 IPC 开销。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::bridge::{ClientBridge, ModeType};
use crate::core::image_cache::{
    CacheCleanupResult, ImageCacheService, ImageDownloadContext, PrefetchResult, ResolveResult,
};

/// 支持的图片格式
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// 最大文件大小 (5MB)
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

/// JPEG 压缩质量 (85% - 保持颜色质量同时控制文件大小)
const JPEG_QUALITY: u8 = 85;

/// 内部辅助: 获取当前模式的图片访问上下文
async fn get_image_context(bridge: &ClientBridge) -> Result<ImageContext, String> {
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
            // 获取 mTLS HTTP client
            let (edge_url, http_client, _token) = bridge
                .get_edge_http_context()
                .await
                .ok_or("Not authenticated or mTLS client not available")?;

            Ok(ImageContext::Client {
                tenant_path,
                download_ctx: ImageDownloadContext {
                    edge_url,
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
        ImageContext::Server {
            tenant_path,
            work_dir,
        } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            image_cache
                .get_server_image_path(&hash, &work_dir)
                .map(|p| p.to_string_lossy().to_string())
                .map_err(|e| e.to_string())
        }
        ImageContext::Client {
            tenant_path,
            download_ctx,
        } => {
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
        ImageContext::Server {
            tenant_path,
            work_dir,
        } => {
            let image_cache = ImageCacheService::new(&tenant_path);
            Ok(image_cache.resolve_server_image_paths(&hashes, &work_dir))
        }
        ImageContext::Client {
            tenant_path,
            download_ctx,
        } => {
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
        ImageContext::Client {
            tenant_path,
            download_ctx,
        } => {
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

/// 保存图片
///
/// 从本地路径读取图片，处理后保存并返回 hash。
///
/// - Server 模式：直接保存到本地 `uploads/images/` 目录
/// - Client 模式：上传到 EdgeServer
///
/// 返回图片的 content hash (SHA256)，用于后续引用。
#[tauri::command]
pub async fn save_image(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    source_path: String,
) -> Result<String, String> {
    let bridge = bridge.read().await;
    let mode_info = bridge.get_mode_info().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    let tenant_path = tenant_manager
        .current_tenant_path()
        .ok_or("No tenant selected")?;

    // 1. 验证和读取源文件
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("Source file not found: {}", source_path));
    }

    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .ok_or("Invalid file extension")?;

    if !SUPPORTED_FORMATS.contains(&ext.as_str()) {
        return Err(format!(
            "Unsupported format '{}'. Supported: {}",
            ext,
            SUPPORTED_FORMATS.join(", ")
        ));
    }

    let data = tokio::fs::read(&source)
        .await
        .map_err(|e| format!("Failed to read source file: {}", e))?;

    if data.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File too large. Maximum size is {}MB",
            MAX_FILE_SIZE / 1024 / 1024
        ));
    }

    match mode_info.mode {
        ModeType::Server => {
            // Server 模式：直接处理并保存
            save_image_server(&data, &tenant_path).await
        }
        ModeType::Client => {
            // Client 模式：上传到 EdgeServer (使用 mTLS)
            save_image_client(&data, &source_path, &bridge).await
        }
        ModeType::Disconnected => Err("Not connected".to_string()),
    }
}

/// Server 模式：本地处理并保存图片
async fn save_image_server(data: &[u8], work_dir: &Path) -> Result<String, String> {
    // 1. 加载并验证图片
    let img = image::load_from_memory(data).map_err(|e| format!("Invalid image: {}", e))?;

    // 2. 压缩为 JPEG
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        let rgb_img = img.to_rgb8();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, JPEG_QUALITY);
        rgb_img
            .write_with_encoder(encoder)
            .map_err(|e| format!("Failed to compress image: {}", e))?;
    }

    // 3. 计算 hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let hash = hex::encode(hasher.finalize());

    // 4. 保存到 uploads/images/
    let images_dir = work_dir.join("uploads/images");
    tokio::fs::create_dir_all(&images_dir)
        .await
        .map_err(|e| format!("Failed to create images directory: {}", e))?;

    let filename = format!("{}.jpg", hash);
    let file_path = images_dir.join(&filename);

    // 检查是否已存在（去重）
    if file_path.exists() {
        tracing::debug!(hash = %hash, "Image already exists, skipping save");
        return Ok(hash);
    }

    tokio::fs::write(&file_path, &buffer)
        .await
        .map_err(|e| format!("Failed to save image: {}", e))?;

    tracing::info!(hash = %hash, size = %buffer.len(), "Image saved successfully");
    Ok(hash)
}

/// Client 模式：上传图片到 EdgeServer (使用 mTLS)
async fn save_image_client(
    data: &[u8],
    source_path: &str,
    bridge: &ClientBridge,
) -> Result<String, String> {
    // 获取 mTLS HTTP client 和认证信息
    let (edge_url, http_client, token) = bridge
        .get_edge_http_context()
        .await
        .ok_or("Not authenticated or mTLS client not available")?;

    // 从路径提取文件名
    let filename = PathBuf::from(source_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.jpg")
        .to_string();

    // 构建 multipart 请求
    let part = reqwest::multipart::Part::bytes(data.to_vec())
        .file_name(filename)
        .mime_str("application/octet-stream")
        .map_err(|e| format!("Failed to create multipart: {}", e))?;

    let form = reqwest::multipart::Form::new().part("file", part);

    let url = format!("{}/api/image/upload", edge_url);

    let resp = http_client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Upload failed ({}): {}", status, text));
    }

    // 解析响应
    #[derive(serde::Deserialize)]
    struct UploadResponse {
        data: UploadData,
    }

    #[derive(serde::Deserialize)]
    struct UploadData {
        hash: String,
    }

    let response: UploadResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    tracing::info!(hash = %response.data.hash, "Image uploaded to EdgeServer");
    Ok(response.data.hash)
}
