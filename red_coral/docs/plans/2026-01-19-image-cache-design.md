# 图片缓存系统设计

## 概述

为 RedCoral POS 设计图片缓存系统，解决 Client 模式下无法直接访问 Edge Server 图片的问题（mTLS 限制）。

### 背景

| 模式 | 图片位置 | 访问方式 | 问题 |
|------|---------|---------|------|
| Server 模式 | 本地 `uploads/images/` | `convertFileSrc()` 直接访问 | ✅ 无问题 |
| Client 模式 | 远程 Edge Server | mTLS HTTPS 请求 | ❌ 浏览器无法直接访问 mTLS |

### 设计目标

1. **显示优化** - Client 模式能正常显示图片
2. **离线支持** - 网络断开时显示已缓存图片
3. **上传同步** - Client 模式离线上传时，先缓存再同步

### 设计原则

- **按需加载** - 图片在需要显示时才下载，不预加载
- **永久缓存** - 下载后持久化到硬盘，不重复下载（图片基本不会改动）
- **App 层实现** - 缓存逻辑在 Tauri App 层，CrabClient 只负责网络传输

---

## 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (React)                        │
│  const path = await invoke('get_image_path', { url })       │
│  <img src={convertFileSrc(path)} />                         │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                 Tauri Commands (image.rs)                    │
│  • get_image_path(url) -> local_path                        │
│  • upload_image(file_path) -> ImageUploadResult             │
│  • sync_image_uploads() -> SyncResult                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   ImageCacheService                          │
│  • 缓存索引管理 (cache_index.json)                           │
│  • 上传队列管理 (upload_queue.json)                          │
│  • 文件存储 (image_cache/images/)                           │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                     ClientBridge                             │
│  • get_bytes(path) -> Vec<u8>    (新增)                     │
│  • upload_file(path, file)       (新增)                     │
└──────────────────────────┬──────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
       Server 模式                 Client 模式
    (直接文件系统)              (mTLS HTTP 请求)
```

---

## 文件结构

```
~/Library/Application Support/com.xzy.pos/redcoral/
└── tenants/{tenant_id}/
    ├── session_cache.json        # 已有
    ├── current_session.json      # 已有
    └── image_cache/
        ├── images/               # 图片文件 (持久化)
        │   ├── {uuid}.jpg
        │   └── ...
        ├── cache_index.json      # 缓存索引 (持久化)
        └── upload_queue.json     # 待上传队列 (持久化)
```

---

## 数据结构

### 缓存索引

```rust
// src-tauri/src/core/image_cache.rs

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub local_filename: String,   // "abc.jpg"
    pub size_bytes: u64,
    pub cached_at: u64,           // Unix timestamp (仅用于统计)
}

/// 缓存索引
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheIndex {
    /// remote_url -> entry (e.g., "/api/image/abc.jpg" -> CacheEntry)
    pub entries: HashMap<String, CacheEntry>,
}
```

### 上传队列

```rust
/// 待上传项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingUpload {
    pub id: String,               // UUID
    pub local_path: PathBuf,      // 本地缓存路径
    pub original_filename: String,
    pub created_at: u64,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub status: UploadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UploadStatus {
    Pending,
    Uploading,
    Failed,
}

/// 上传队列
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UploadQueue {
    pub pending: Vec<PendingUpload>,
}
```

### 返回类型

```rust
/// 上传结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUploadResult {
    pub local_path: String,         // 本地缓存路径 (立即可用)
    pub remote_url: Option<String>, // Server模式立即返回，Client离线时为 None
    pub is_pending: bool,           // Client离线时为 true
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success_count: u32,
    pub failed_count: u32,
    pub pending_count: u32,
}
```

---

## 核心流程

### 图片下载（按需加载）

```
前端渲染需要图片
       │
       ▼
invoke('get_image_path', { url: "/api/image/abc.jpg" })
       │
       ▼
ImageCacheService.get_image_path()
       │
       ▼
检查 cache_index ─── 有记录且文件存在 ──→ 返回本地路径
       │
       无记录或文件被删除
       ▼
bridge.get_bytes("/api/image/abc.jpg")
       │
       ├── Server 模式: 读取 {work_dir}/uploads/images/abc.jpg
       └── Client 模式: mTLS GET https://edge/api/image/abc.jpg
       │
       ▼
保存到 image_cache/images/abc.jpg
       │
       ▼
更新 cache_index.json
       │
       ▼
返回本地路径
```

### 图片上传

```
用户选择图片上传
       │
       ▼
invoke('upload_image', { filePath: "/path/to/image.jpg" })
       │
       ▼
ImageCacheService.upload_image()
       │
       ├── Server 模式:
       │     bridge.upload_file("/api/image/upload", file)
       │     └── 返回 { local_path, remote_url: Some(...), is_pending: false }
       │
       └── Client 模式:
             ├── 在线: 同 Server 模式
             └── 离线:
                   1. 复制到 image_cache/images/{uuid}.jpg
                   2. 加入 upload_queue.json
                   3. 返回 { local_path, remote_url: None, is_pending: true }
```

### 上传队列同步

```
触发时机: App启动 / 网络恢复 / 手动触发
       │
       ▼
ImageCacheService.sync_pending_uploads()
       │
       ▼
遍历 upload_queue.pending
       │
       ▼
对每个 PendingUpload:
  bridge.upload_file("/api/image/upload", local_path)
       │
       ├── 成功:
       │     1. 更新 cache_index (local_path -> remote_url)
       │     2. 从 upload_queue 移除
       │     3. (可选) 通知前端刷新
       │
       └── 失败:
             retry_count++
             if retry_count > 3: status = Failed
```

---

## 实现细节

### ImageCacheService

```rust
// src-tauri/src/core/image_cache.rs

pub struct ImageCacheService {
    cache_dir: PathBuf,           // .../image_cache/
    images_dir: PathBuf,          // .../image_cache/images/
    index_path: PathBuf,          // .../image_cache/cache_index.json
    queue_path: PathBuf,          // .../image_cache/upload_queue.json
    index: RwLock<CacheIndex>,
    queue: RwLock<UploadQueue>,
}

impl ImageCacheService {
    /// 从租户目录初始化
    pub fn new(tenant_path: &Path) -> Result<Self, ImageCacheError> {
        let cache_dir = tenant_path.join("image_cache");
        let images_dir = cache_dir.join("images");
        std::fs::create_dir_all(&images_dir)?;

        let index_path = cache_dir.join("cache_index.json");
        let queue_path = cache_dir.join("upload_queue.json");

        let index = Self::load_index(&index_path)?;
        let queue = Self::load_queue(&queue_path)?;

        Ok(Self {
            cache_dir,
            images_dir,
            index_path,
            queue_path,
            index: RwLock::new(index),
            queue: RwLock::new(queue),
        })
    }

    /// 获取图片本地路径（核心方法）
    pub async fn get_image_path(
        &self,
        remote_url: &str,
        bridge: &ClientBridge,
    ) -> Result<PathBuf, ImageCacheError> {
        // 1. 检查缓存
        {
            let index = self.index.read().await;
            if let Some(entry) = index.entries.get(remote_url) {
                let local_path = self.images_dir.join(&entry.local_filename);
                if local_path.exists() {
                    return Ok(local_path);
                }
            }
        }

        // 2. 下载
        let bytes = bridge.get_bytes(remote_url).await
            .map_err(|e| ImageCacheError::Download(e.to_string()))?;

        // 3. 保存到本地
        let filename = Self::extract_filename(remote_url);
        let local_path = self.images_dir.join(&filename);
        tokio::fs::write(&local_path, &bytes).await?;

        // 4. 更新索引
        {
            let mut index = self.index.write().await;
            index.entries.insert(remote_url.to_string(), CacheEntry {
                local_filename: filename,
                size_bytes: bytes.len() as u64,
                cached_at: Self::now(),
            });
        }
        self.save_index().await?;

        Ok(local_path)
    }

    /// 上传图片
    pub async fn upload_image(
        &self,
        file_path: &Path,
        bridge: &ClientBridge,
    ) -> Result<ImageUploadResult, ImageCacheError> {
        let mode = bridge.current_mode().await;
        let is_online = bridge.is_network_available().await;

        // Server 模式或 Client 在线: 直接上传
        if mode == ModeType::Server || is_online {
            let response: serde_json::Value = bridge
                .upload_file("/api/image/upload", file_path)
                .await
                .map_err(|e| ImageCacheError::Upload(e.to_string()))?;

            let remote_url = response["data"]["url"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            // 如果是 Client 模式，将上传的图片也缓存到本地
            if mode == ModeType::Client {
                let filename = Self::extract_filename(&remote_url);
                let local_path = self.images_dir.join(&filename);
                tokio::fs::copy(file_path, &local_path).await?;

                let mut index = self.index.write().await;
                let metadata = tokio::fs::metadata(&local_path).await?;
                index.entries.insert(remote_url.clone(), CacheEntry {
                    local_filename: filename.clone(),
                    size_bytes: metadata.len(),
                    cached_at: Self::now(),
                });
                drop(index);
                self.save_index().await?;

                return Ok(ImageUploadResult {
                    local_path: local_path.to_string_lossy().to_string(),
                    remote_url: Some(remote_url),
                    is_pending: false,
                });
            }

            Ok(ImageUploadResult {
                local_path: file_path.to_string_lossy().to_string(),
                remote_url: Some(remote_url),
                is_pending: false,
            })
        } else {
            // Client 离线: 缓存到本地，加入上传队列
            let id = uuid::Uuid::new_v4().to_string();
            let filename = format!("{}.jpg", id);
            let local_path = self.images_dir.join(&filename);

            tokio::fs::copy(file_path, &local_path).await?;

            let pending = PendingUpload {
                id: id.clone(),
                local_path: local_path.clone(),
                original_filename: file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image.jpg")
                    .to_string(),
                created_at: Self::now(),
                retry_count: 0,
                last_error: None,
                status: UploadStatus::Pending,
            };

            {
                let mut queue = self.queue.write().await;
                queue.pending.push(pending);
            }
            self.save_queue().await?;

            Ok(ImageUploadResult {
                local_path: local_path.to_string_lossy().to_string(),
                remote_url: None,
                is_pending: true,
            })
        }
    }

    /// 同步待上传队列
    pub async fn sync_pending_uploads(
        &self,
        bridge: &ClientBridge,
    ) -> Result<SyncResult, ImageCacheError> {
        let mut success_count = 0u32;
        let mut failed_count = 0u32;

        loop {
            // 获取下一个待上传项
            let pending = {
                let mut queue = self.queue.write().await;
                queue.pending.iter_mut()
                    .find(|p| matches!(p.status, UploadStatus::Pending))
                    .map(|p| {
                        p.status = UploadStatus::Uploading;
                        p.clone()
                    })
            };

            let Some(mut item) = pending else {
                break;
            };

            // 尝试上传
            match bridge.upload_file("/api/image/upload", &item.local_path).await {
                Ok(response) => {
                    let remote_url = response["data"]["url"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();

                    // 更新缓存索引
                    {
                        let mut index = self.index.write().await;
                        let metadata = tokio::fs::metadata(&item.local_path).await?;
                        index.entries.insert(remote_url.clone(), CacheEntry {
                            local_filename: item.local_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or_default()
                                .to_string(),
                            size_bytes: metadata.len(),
                            cached_at: Self::now(),
                        });
                    }

                    // 从队列移除
                    {
                        let mut queue = self.queue.write().await;
                        queue.pending.retain(|p| p.id != item.id);
                    }

                    success_count += 1;
                }
                Err(e) => {
                    item.retry_count += 1;
                    item.last_error = Some(e.to_string());
                    item.status = if item.retry_count >= 3 {
                        UploadStatus::Failed
                    } else {
                        UploadStatus::Pending
                    };

                    // 更新队列
                    {
                        let mut queue = self.queue.write().await;
                        if let Some(p) = queue.pending.iter_mut().find(|p| p.id == item.id) {
                            *p = item;
                        }
                    }

                    if matches!(item.status, UploadStatus::Failed) {
                        failed_count += 1;
                    }
                }
            }

            self.save_queue().await?;
        }

        let pending_count = {
            let queue = self.queue.read().await;
            queue.pending.iter()
                .filter(|p| matches!(p.status, UploadStatus::Pending))
                .count() as u32
        };

        Ok(SyncResult {
            success_count,
            failed_count,
            pending_count,
        })
    }

    // ============ 辅助方法 ============

    fn extract_filename(url: &str) -> String {
        url.trim_start_matches("/api/image/")
            .to_string()
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    async fn save_index(&self) -> Result<(), ImageCacheError> {
        let index = self.index.read().await;
        let content = serde_json::to_string_pretty(&*index)?;
        tokio::fs::write(&self.index_path, content).await?;
        Ok(())
    }

    async fn save_queue(&self) -> Result<(), ImageCacheError> {
        let queue = self.queue.read().await;
        let content = serde_json::to_string_pretty(&*queue)?;
        tokio::fs::write(&self.queue_path, content).await?;
        Ok(())
    }

    fn load_index(path: &Path) -> Result<CacheIndex, ImageCacheError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(CacheIndex::default())
        }
    }

    fn load_queue(path: &Path) -> Result<UploadQueue, ImageCacheError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(UploadQueue::default())
        }
    }
}
```

### ClientBridge 新增方法

```rust
// 在 src-tauri/src/core/client_bridge.rs 中新增

impl ClientBridge {
    /// 获取原始字节数据 (图片下载)
    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // Server 模式: 直接读取本地文件
                // path: "/api/image/abc.jpg" -> {work_dir}/uploads/images/abc.jpg
                let filename = path.trim_start_matches("/api/image/");
                let file_path = server_state
                    .work_dir()
                    .join("uploads/images")
                    .join(filename);

                tokio::fs::read(&file_path)
                    .await
                    .map_err(BridgeError::Io)
            }

            ClientMode::Client { client, edge_url, .. } => {
                // Client 模式: 通过 mTLS HTTP 获取
                let http = match client {
                    Some(RemoteClientState::Connected(c)) => c.edge_http_client(),
                    Some(RemoteClientState::Authenticated(c)) => c.edge_http_client(),
                    None => return Err(BridgeError::NotInitialized),
                }.ok_or(BridgeError::NotInitialized)?;

                let url = format!("{}{}", edge_url, path);
                let resp = http
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                if !resp.status().is_success() {
                    return Err(BridgeError::Server(format!("HTTP {}", resp.status())));
                }

                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                Ok(bytes.to_vec())
            }

            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 上传文件 (multipart/form-data)
    pub async fn upload_file(
        &self,
        path: &str,
        file_path: &Path,
    ) -> Result<serde_json::Value, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => {
                match client {
                    Some(LocalClientState::Authenticated(auth)) => {
                        auth.upload_file(path, file_path)
                            .await
                            .map_err(BridgeError::Client)
                    }
                    _ => Err(BridgeError::NotAuthenticated),
                }
            }

            ClientMode::Client { client, edge_url, .. } => {
                let (http, token) = match client {
                    Some(RemoteClientState::Authenticated(c)) => {
                        (c.edge_http_client(), c.token())
                    }
                    _ => return Err(BridgeError::NotAuthenticated),
                };

                let http = http.ok_or(BridgeError::NotInitialized)?;
                let token = token.ok_or(BridgeError::NotAuthenticated)?;

                let file_bytes = tokio::fs::read(file_path)
                    .await
                    .map_err(BridgeError::Io)?;

                let filename = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image.jpg")
                    .to_string();

                let part = reqwest::multipart::Part::bytes(file_bytes)
                    .file_name(filename);
                let form = reqwest::multipart::Form::new().part("file", part);

                let url = format!("{}{}", edge_url, path);
                let resp = http
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                if !resp.status().is_success() {
                    let text = resp.text().await.unwrap_or_default();
                    return Err(BridgeError::Server(text));
                }

                resp.json()
                    .await
                    .map_err(|e| BridgeError::Server(e.to_string()))
            }

            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 获取当前模式
    pub async fn current_mode(&self) -> ModeType {
        let mode_guard = self.mode.read().await;
        match &*mode_guard {
            ClientMode::Server { .. } => ModeType::Server,
            ClientMode::Client { .. } => ModeType::Client,
            ClientMode::Disconnected => ModeType::Disconnected,
        }
    }

    /// 检查网络是否可用 (Client 模式健康检查)
    pub async fn is_network_available(&self) -> bool {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { .. } => true,
            ClientMode::Client { client, edge_url, .. } => {
                let http = match client {
                    Some(RemoteClientState::Connected(c)) => c.edge_http_client(),
                    Some(RemoteClientState::Authenticated(c)) => c.edge_http_client(),
                    None => return false,
                };

                let Some(http) = http else { return false };

                match http
                    .get(format!("{}/health", edge_url))
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                }
            }
            ClientMode::Disconnected => false,
        }
    }
}
```

### Tauri Commands

```rust
// src-tauri/src/commands/image.rs

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::client_bridge::ClientBridge;
use crate::core::image_cache::{ImageCacheService, ImageUploadResult, SyncResult};

/// 获取图片本地路径
#[tauri::command]
pub async fn get_image_path(
    url: String,
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    image_cache: State<'_, Arc<ImageCacheService>>,
) -> Result<String, String> {
    let bridge = bridge.read().await;

    image_cache
        .get_image_path(&url, &bridge)
        .await
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// 上传图片
#[tauri::command]
pub async fn upload_image(
    file_path: String,
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    image_cache: State<'_, Arc<ImageCacheService>>,
) -> Result<ImageUploadResult, String> {
    let bridge = bridge.read().await;
    let path = std::path::PathBuf::from(&file_path);

    image_cache
        .upload_image(&path, &bridge)
        .await
        .map_err(|e| e.to_string())
}

/// 同步待上传的图片
#[tauri::command]
pub async fn sync_image_uploads(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    image_cache: State<'_, Arc<ImageCacheService>>,
) -> Result<SyncResult, String> {
    let bridge = bridge.read().await;

    image_cache
        .sync_pending_uploads(&bridge)
        .await
        .map_err(|e| e.to_string())
}
```

---

## 前端使用

### 工具函数

```typescript
// src/utils/image.ts

import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import DefaultImage from '@/assets/reshot.svg';

/**
 * 获取图片 URL（统一入口）
 * 自动处理 Server/Client 模式和缓存
 */
export async function getImageUrl(remoteUrl: string | null | undefined): Promise<string> {
  if (!remoteUrl) {
    return DefaultImage;
  }

  // 外部图片直接返回
  if (/^https?:\/\//.test(remoteUrl)) {
    return remoteUrl;
  }

  try {
    const localPath = await invoke<string>('get_image_path', { url: remoteUrl });
    return convertFileSrc(localPath);
  } catch (error) {
    console.error('Failed to get image:', remoteUrl, error);
    return DefaultImage;
  }
}

/**
 * 上传图片
 */
export interface ImageUploadResult {
  local_path: string;
  remote_url: string | null;
  is_pending: boolean;
}

export async function uploadImage(filePath: string): Promise<ImageUploadResult> {
  return invoke<ImageUploadResult>('upload_image', { filePath });
}

/**
 * 同步待上传图片
 */
export interface SyncResult {
  success_count: number;
  failed_count: number;
  pending_count: number;
}

export async function syncImageUploads(): Promise<SyncResult> {
  return invoke<SyncResult>('sync_image_uploads');
}
```

### 组件使用

```tsx
// 使用 hook 封装
import { useState, useEffect } from 'react';
import { getImageUrl } from '@/utils/image';
import DefaultImage from '@/assets/reshot.svg';

export function useImageUrl(remoteUrl: string | null | undefined) {
  const [src, setSrc] = useState(DefaultImage);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    getImageUrl(remoteUrl)
      .then(setSrc)
      .finally(() => setLoading(false));
  }, [remoteUrl]);

  return { src, loading };
}

// 组件使用
function ProductCard({ product }: { product: Product }) {
  const { src, loading } = useImageUrl(product.image);

  return (
    <div className="product-card">
      {loading ? (
        <div className="skeleton" />
      ) : (
        <img src={src} alt={product.name} />
      )}
      <span>{product.name}</span>
    </div>
  );
}
```

---

## 同步策略

### 上传队列同步时机

| 触发点 | 实现方式 |
|--------|---------|
| App 启动 | `lib.rs` setup 时调用 `sync_image_uploads` |
| 网络恢复 | 监听网络状态变化事件 |
| 手动触发 | 设置页面提供同步按钮 |

### 重试策略

- 最大重试次数：3 次
- 超过重试次数：标记为 `Failed`，需手动处理
- 失败项不阻塞其他上传

---

## 边界情况

| 情况 | 处理方式 |
|------|---------|
| 索引有记录但文件被删除 | 重新下载并更新索引 |
| 网络不可用 + 无缓存 | 返回错误，前端显示默认图片 |
| 图片 URL 变更（极少） | 新 URL 视为新图片，下载新文件 |
| 磁盘空间不足 | 上传/下载时返回 IO 错误 |

---

## 实现步骤

1. **创建 `image_cache.rs`** - 实现 `ImageCacheService`
2. **修改 `client_bridge.rs`** - 添加 `get_bytes`、`upload_file`、`current_mode`、`is_network_available` 方法
3. **创建 `commands/image.rs`** - 实现 Tauri commands
4. **修改 `lib.rs`** - 注册 commands，初始化 `ImageCacheService`
5. **创建 `src/utils/image.ts`** - 前端工具函数
6. **创建 `useImageUrl` hook** - 封装图片加载逻辑
7. **更新现有组件** - 使用新的图片加载方式
