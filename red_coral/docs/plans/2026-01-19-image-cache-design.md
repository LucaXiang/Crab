# 图片缓存系统设计

## 概述

为 RedCoral POS 设计图片缓存系统，解决 Client 模式下无法直接访问 Edge Server 图片的问题（mTLS 限制）。

### 背景

| 模式 | 图片位置 | 访问方式 | 问题 |
|------|---------|---------|------|
| Server 模式 | 本地 `uploads/images/` | `convertFileSrc()` 直接访问 | ✅ 无问题 |
| Client 模式 | 远程 Edge Server | mTLS HTTPS 请求 | ❌ 浏览器无法直接访问 mTLS |

### 设计目标

1. **显示优化** - Client 模式能正常显示图片（绕过 mTLS 限制）
2. **缓存加速** - 已下载的图片直接使用缓存，无需重复下载

### 架构约束

- **Server 模式**: edge_server 与 redcoral 同进程运行，直接访问文件系统
- **Client 模式**: 必须在线才能操作，断网时所有操作无效

### 设计原则

- **内容寻址** - 用图片内容 hash 作为唯一标识（key）
- **按需加载** - 图片在需要显示时才下载
- **持久缓存** - 下载后保存到磁盘，相同 hash 不重复下载
- **后端下载** - 由 Rust 层处理 mTLS 请求，保证图片存在后返回路径

---

## 核心设计：Hash 作为 Key

```
┌─────────────────────────────────────────────────────────┐
│                      数据库                              │
│           Product.image = "a1b2c3d4" (hash)             │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              get_image_path(hash) -> path               │
│                    (Tauri Command)                       │
├─────────────────────────────────────────────────────────┤
│ Server: a1b2c3d4 → {work_dir}/uploads/images/a1b2c3d4.jpg│
│ Client: a1b2c3d4 → {tenant}/image_cache/images/a1b2c3d4.jpg│
│         (如果不存在，自动从远程下载)                      │
└─────────────────────────────────────────────────────────┘
```

**优点：**
1. ✅ 相同图片 = 相同 hash（内容寻址，天然去重）
2. ✅ 数据库与存储路径解耦
3. ✅ Client 离线上传的 hash 与 Server 一致
4. ✅ 路径结构变更不影响数据库

---

## 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (React)                        │
│  ┌─────────────────────────────────────────────────────┐    │
│  │    IndexedDB (base64 缓存，快速访问)                  │    │
│  └─────────────────────────────────────────────────────┘    │
│  const url = await getImageUrl(hash)                        │
│  <img src={url} />                                          │
└──────────────────────────┬──────────────────────────────────┘
                           │ cache miss
┌──────────────────────────▼──────────────────────────────────┐
│                 Tauri Commands (image.rs)                    │
│  • get_image_path(hash) -> local_path                       │
│  • prefetch_images(hashes) -> PrefetchResult                │
│  • cleanup_image_cache(active_hashes) -> CacheCleanupResult │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   ImageCacheService                          │
│  • hash -> local_path 映射                                  │
│  • 自动下载缺失图片 (Client 模式)                            │
│  • 缓存清理 (删除不再引用的图片)                             │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                     ClientBridge                             │
│  • get_bytes(path) -> Vec<u8>                               │
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
# Server 模式 (EdgeServer 存储，直接使用)
{work_dir}/
└── uploads/images/
    ├── {hash1}.jpg
    ├── {hash2}.jpg
    └── ...

# Client 模式 (本地缓存)
~/Library/Application Support/com.xzy.pos/redcoral/
└── tenants/{tenant_id}/
    └── image_cache/
        └── images/               # 缓存的图片文件
            ├── {hash1}.jpg
            └── ...
```

**注意：**
- Server 模式直接使用 EdgeServer 的图片，不重复存储
- Client 模式才需要 `image_cache/` 目录

---

## 数据结构

### 数据库存储

```typescript
// Product.image 存储图片的 hash（不带扩展名）
interface Product {
  id: string;
  name: string;
  image: string;  // "a1b2c3d4e5f6..." (SHA256 hash)
  // ...
}
```

### 返回类型

```rust
/// 预加载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchResult {
    pub success_count: u32,
    pub failed_count: u32,
    pub already_cached: u32,
}

/// 缓存清理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCleanupResult {
    pub removed_count: u32,
    pub freed_bytes: u64,
}
```

---

## 核心流程

### 获取图片路径 (get_image_path)

```
前端调用: invoke('get_image_path', { hash: "a1b2c3d4" })
       │
       ▼
ImageCacheService.get_image_path(hash)
       │
       ▼
检查模式
       │
       ├── Server 模式:
       │     直接返回 {work_dir}/uploads/images/{hash}.jpg
       │     (不检查文件是否存在，EdgeServer 保证存在)
       │
       └── Client 模式:
             │
             ▼
       检查本地缓存 {tenant}/image_cache/images/{hash}.jpg
             │
             ├── 存在 → 返回本地路径
             │
             └── 不存在 → 从远程下载
                   │
                   ▼
             bridge.get_bytes("/api/image/{hash}.jpg")
                   │
                   ▼
             保存到本地缓存
                   │
                   ▼
             返回本地路径
```

### 批量预加载 (prefetch_images)

```
前端调用: invoke('prefetch_images', { hashes: ["a1b2", "c3d4", ...] })
       │
       ▼
对每个 hash 并发执行 get_image_path()
       │
       ▼
返回 PrefetchResult { success, failed, already_cached }
```

### 缓存清理 (cleanup_image_cache)

**场景：** 产品 A 图片从 "abc123" 更新为 "def456"，旧缓存 "abc123.jpg" 变成孤儿文件。

```
触发时机: 产品数据加载完成后（或定时触发）
       │
       ▼
前端调用: invoke('cleanup_image_cache', { activeHashes: [...当前所有产品的 image hash] })
       │
       ▼
遍历 image_cache/images/ 目录
       │
       ▼
对每个 {hash}.jpg:
       │
       ├── hash 在 activeHashes 中 → 保留
       │
       └── hash 不在 activeHashes 中 → 删除
       │
       ▼
返回 CacheCleanupResult { removed_count, freed_bytes }
```

**注意：** Server 模式不需要清理（EdgeServer 自行管理）。

---

## 实现细节

### EdgeServer 修改：用 hash 作为文件名

```rust
// edge-server/src/api/upload/handler.rs

pub async fn upload(...) -> Result<...> {
    // 1. 处理图片 (压缩等)
    let compressed_data = process_and_compress_image(data)?;

    // 2. 计算 hash 作为文件名
    let hash = calculate_hash(&compressed_data);
    let filename = format!("{}.jpg", hash);
    let file_path = images_dir.join(&filename);

    // 3. 检查是否已存在 (天然去重)
    if file_path.exists() {
        return Ok(UploadResponse {
            hash: hash.clone(),
            filename,
            url: format!("/api/image/{}.jpg", hash),
            ...
        });
    }

    // 4. 保存新文件
    fs::write(&file_path, &compressed_data)?;

    Ok(UploadResponse {
        hash,
        filename,
        url: format!("/api/image/{}.jpg", hash),
        ...
    })
}
```

### ImageCacheService

```rust
// src-tauri/src/core/image_cache.rs

pub struct ImageCacheService {
    images_dir: PathBuf,  // Client 模式: {tenant}/image_cache/images/
}

impl ImageCacheService {
    pub fn new(tenant_path: &Path) -> Self {
        let images_dir = tenant_path.join("image_cache/images");
        std::fs::create_dir_all(&images_dir).ok();
        Self { images_dir }
    }

    /// 获取图片本地路径（核心方法）
    pub async fn get_image_path(
        &self,
        hash: &str,
        bridge: &ClientBridge,
    ) -> Result<PathBuf, ImageCacheError> {
        let mode = bridge.current_mode().await;

        // Server 模式: 直接返回 EdgeServer 的图片路径
        if mode == ModeType::Server {
            let work_dir = bridge.get_server_work_dir().await?;
            return Ok(work_dir.join("uploads/images").join(format!("{}.jpg", hash)));
        }

        // Client 模式: 检查本地缓存
        let local_path = self.images_dir.join(format!("{}.jpg", hash));
        if local_path.exists() {
            return Ok(local_path);
        }

        // Client 模式: 下载并缓存
        let url = format!("/api/image/{}.jpg", hash);
        let bytes = bridge.get_bytes(&url).await?;
        tokio::fs::write(&local_path, &bytes).await?;

        Ok(local_path)
    }

    /// 批量预加载图片
    pub async fn prefetch_images(
        &self,
        hashes: &[String],
        bridge: &ClientBridge,
    ) -> Result<PrefetchResult, ImageCacheError> {
        // Server 模式不需要预加载
        if bridge.current_mode().await == ModeType::Server {
            return Ok(PrefetchResult {
                success_count: 0,
                failed_count: 0,
                already_cached: hashes.len() as u32,
            });
        }

        let mut success = 0u32;
        let mut failed = 0u32;
        let mut already_cached = 0u32;

        // 并发下载 (限制并发数)
        let semaphore = Arc::new(tokio::sync::Semaphore::new(4));
        let mut handles = Vec::new();

        for hash in hashes {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let hash = hash.clone();
            let images_dir = self.images_dir.clone();
            let bridge = bridge.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let local_path = images_dir.join(format!("{}.jpg", hash));

                if local_path.exists() {
                    return (hash, Ok(true)); // already cached
                }

                let url = format!("/api/image/{}.jpg", hash);
                match bridge.get_bytes(&url).await {
                    Ok(bytes) => {
                        match tokio::fs::write(&local_path, &bytes).await {
                            Ok(_) => (hash, Ok(false)), // success
                            Err(e) => (hash, Err(e.to_string())),
                        }
                    }
                    Err(e) => (hash, Err(e.to_string())),
                }
            }));
        }

        for handle in handles {
            match handle.await {
                Ok((_, Ok(true))) => already_cached += 1,
                Ok((_, Ok(false))) => success += 1,
                Ok((_, Err(_))) => failed += 1,
                Err(_) => failed += 1,
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
        bridge: &ClientBridge,
    ) -> Result<CacheCleanupResult, ImageCacheError> {
        // Server 模式不需要清理
        if bridge.current_mode().await == ModeType::Server {
            return Ok(CacheCleanupResult {
                removed_count: 0,
                freed_bytes: 0,
            });
        }

        let active_set: std::collections::HashSet<_> = active_hashes.iter().collect();
        let mut removed_count = 0u32;
        let mut freed_bytes = 0u64;

        let mut entries = tokio::fs::read_dir(&self.images_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let filename = entry.file_name().to_string_lossy().to_string();
            if let Some(hash) = filename.strip_suffix(".jpg") {
                if !active_set.contains(&hash.to_string()) {
                    let metadata = entry.metadata().await?;
                    freed_bytes += metadata.len();
                    tokio::fs::remove_file(entry.path()).await?;
                    removed_count += 1;
                }
            }
        }

        Ok(CacheCleanupResult {
            removed_count,
            freed_bytes,
        })
    }
}
```

### ClientBridge 新增方法

```rust
impl ClientBridge {
    /// 获取原始字节数据 (图片下载)
    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // Server 模式: 直接读取本地文件
                let filename = path.trim_start_matches("/api/image/");
                let file_path = server_state
                    .work_dir()
                    .join("uploads/images")
                    .join(filename);

                tokio::fs::read(&file_path).await.map_err(BridgeError::Io)
            }

            ClientMode::Client { client, edge_url, .. } => {
                // Client 模式: 通过 mTLS HTTP 获取
                let http = match client {
                    Some(RemoteClientState::Connected(c)) => c.edge_http_client(),
                    Some(RemoteClientState::Authenticated(c)) => c.edge_http_client(),
                    None => return Err(BridgeError::NotInitialized),
                }.ok_or(BridgeError::NotInitialized)?;

                let url = format!("{}{}", edge_url, path);
                let resp = http.get(&url).send().await?;

                if !resp.status().is_success() {
                    return Err(BridgeError::Server(format!("HTTP {}", resp.status())));
                }

                Ok(resp.bytes().await?.to_vec())
            }

            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 获取 Server 模式的工作目录
    pub async fn get_server_work_dir(&self) -> Result<PathBuf, BridgeError> {
        let mode_guard = self.mode.read().await;
        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                Ok(server_state.work_dir().clone())
            }
            _ => Err(BridgeError::NotInitialized),
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

    /// 检查网络是否可用
    pub async fn is_network_available(&self) -> bool {
        // ... 健康检查实现
    }
}
```

### Tauri Commands

```rust
// src-tauri/src/commands/image.rs

/// 获取图片本地路径
#[tauri::command]
pub async fn get_image_path(
    hash: String,
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    image_cache: State<'_, Arc<ImageCacheService>>,
) -> Result<String, String> {
    let bridge = bridge.read().await;
    image_cache
        .get_image_path(&hash, &bridge)
        .await
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

/// 批量预加载图片
#[tauri::command]
pub async fn prefetch_images(
    hashes: Vec<String>,
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    image_cache: State<'_, Arc<ImageCacheService>>,
) -> Result<PrefetchResult, String> {
    let bridge = bridge.read().await;
    image_cache
        .prefetch_images(&hashes, &bridge)
        .await
        .map_err(|e| e.to_string())
}

/// 清理不再引用的缓存图片
#[tauri::command]
pub async fn cleanup_image_cache(
    active_hashes: Vec<String>,
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    image_cache: State<'_, Arc<ImageCacheService>>,
) -> Result<CacheCleanupResult, String> {
    let bridge = bridge.read().await;
    image_cache
        .cleanup_cache(&active_hashes, &bridge)
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
 * 获取图片 URL
 * @param hash 图片 hash (从数据库读取)
 */
export async function getImageUrl(hash: string | null | undefined): Promise<string> {
  if (!hash) {
    return DefaultImage;
  }

  try {
    const localPath = await invoke<string>('get_image_path', { hash });
    return convertFileSrc(localPath);
  } catch (error) {
    console.error('Failed to get image:', hash, error);
    return DefaultImage;
  }
}

/**
 * 批量预加载图片
 */
export async function prefetchImages(hashes: string[]): Promise<PrefetchResult> {
  const validHashes = hashes.filter(Boolean);
  if (validHashes.length === 0) {
    return { success_count: 0, failed_count: 0, already_cached: 0 };
  }
  return invoke<PrefetchResult>('prefetch_images', { hashes: validHashes });
}

/**
 * 上传图片
 */
export async function uploadImage(filePath: string): Promise<ImageUploadResult> {
  return invoke<ImageUploadResult>('upload_image', { filePath });
}
```

### React Hook

```typescript
// src/hooks/useImageUrl.ts

import { useState, useEffect } from 'react';
import { getImageUrl } from '@/utils/image';
import DefaultImage from '@/assets/reshot.svg';

export function useImageUrl(hash: string | null | undefined) {
  const [src, setSrc] = useState(DefaultImage);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!hash) {
      setSrc(DefaultImage);
      setLoading(false);
      return;
    }

    setLoading(true);
    getImageUrl(hash)
      .then(setSrc)
      .finally(() => setLoading(false));
  }, [hash]);

  return { src, loading };
}
```

### 组件使用

```tsx
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

### 预加载优化

```tsx
// 获取产品列表后预加载图片
function ProductList() {
  const { products } = useProducts();

  useEffect(() => {
    if (products.length > 0) {
      const hashes = products.map(p => p.image).filter(Boolean);
      prefetchImages(hashes);
    }
  }, [products]);

  return (
    <div className="product-grid">
      {products.map(p => <ProductCard key={p.id} product={p} />)}
    </div>
  );
}
```

---

## 存储位置对比

| 模式 | 图片位置 | 份数 | 说明 |
|------|---------|------|------|
| Server | `{work_dir}/uploads/images/{hash}.jpg` | 1 份 | EdgeServer 原有存储 |
| Client | `{tenant}/image_cache/images/{hash}.jpg` | 1 份 | 本地缓存 |

**Server 模式不重复存储** - 直接返回 EdgeServer 的图片路径。

---

## 实现步骤

1. **修改 EdgeServer** - 上传时用 hash 作为文件名（替代 UUID）
2. **创建 `image_cache.rs`** - 实现 `ImageCacheService`
3. **修改 `client_bridge.rs`** - 添加 `get_bytes`、`get_server_work_dir`、`current_mode`
4. **创建 `commands/image.rs`** - 实现 Tauri commands (`get_image_path`, `prefetch_images`, `cleanup_image_cache`)
5. **修改 `lib.rs`** - 注册 commands，初始化 `ImageCacheService`
6. **创建前端工具**:
   - `src/utils/image/indexedDBCache.ts` - IndexedDB 缓存操作
   - `src/utils/image/index.ts` - `getImageUrl()` 函数
   - `src/hooks/useImageUrl.ts` - React hook
7. **创建连接恢复 hook** - `src/core/hooks/useConnectionRecovery.ts`
8. **更新现有组件** - 使用新的图片加载方式
9. **更新数据库** - Product.image 存储 hash（如需迁移）

---

## 边界情况

| 情况 | 处理方式 |
|------|---------|
| Client 模式图片未缓存 | `get_image_path` 自动下载 |
| 网络不可用 + 无缓存 | 返回错误，前端显示默认图片 |
| 离线上传后网络恢复 | `sync_image_uploads` 同步到服务器 |
| 相同图片重复上传 | hash 相同，自动去重 |

---

## 多端同步问题分析

### 场景 1：Sync 事件丢失

```
时间线:
  T1: Client 在线，Product.image = "abc123"
  T2: Client 离线
  T3: Server 更新 Product.image = "def456" (Sync 事件发出)
  T4: Client 重新上线 (错过了 Sync 事件)
```

**问题：** Client 仍然显示旧图片 "abc123"

**解决方案：** 重连时强制刷新产品数据

```typescript
// src/core/hooks/useConnectionRecovery.ts
export function useConnectionRecovery() {
  const { connectionState, prevState } = useBridgeStore();

  useEffect(() => {
    // 检测从离线恢复到在线
    if (connectionState === 'connected' && prevState === 'disconnected') {
      // 强制刷新所有资源
      useProductStore.getState().fetchAll();
      // ... 其他 stores
    }
  }, [connectionState, prevState]);
}
```

### 场景 2：IndexedDB 缓存与服务器数据不一致

```
时间线:
  T1: Client 加载 Product.image = "abc123"，缓存到 IndexedDB
  T2: Server 更新 Product.image = "def456"
  T3: Client 收到 Sync，产品数据更新为 image = "def456"
  T4: 前端用 "def456" 调用 getImageUrl()
```

**这不是问题！**

- 因为 hash 是内容寻址的
- "abc123" 在 IndexedDB 里仍然是正确的（那就是 abc123 的内容）
- "def456" 是新的 hash，IndexedDB 没有 → 会下载新图片
- 两个缓存各自正确，互不影响

### 场景 3：缓存清理导致 IndexedDB 与磁盘不一致

```
清理时序:
  1. cleanup_image_cache() 删除磁盘上的 "abc123.jpg"
  2. IndexedDB 还有 "abc123" 的 base64
  3. 下次 getImageUrl("abc123") 命中 IndexedDB，返回正确图片 ✓
```

**这不是问题！** IndexedDB 有缓存就用，没有才走磁盘/网络。

### 场景 4：多 Client 上传相同图片

```
  Client A 上传 photo.jpg → hash = "xyz789"
  Client B 上传 photo.jpg → hash = "xyz789" (相同内容)
```

**自然去重！** 相同内容 = 相同 hash，EdgeServer 检测到已存在，直接返回。

### 结论

使用 **content hash** 作为 key 的设计天然解决了大部分同步问题：

| 问题 | 是否存在 | 原因 |
|------|---------|------|
| 缓存显示错误图片 | ❌ 不存在 | 同一 hash 内容永远相同 |
| 数据与图片不一致 | ❌ 不存在 | 数据变化 = hash 变化 = 重新加载 |
| Sync 丢失 | ✅ 存在 | 需要重连时强制刷新 |
| 多端上传冲突 | ❌ 不存在 | 相同内容 = 相同 hash |

**唯一需要处理的是：Sync 事件丢失后的数据恢复。**

---

## 前端 IndexedDB 缓存层

### 为什么需要双层缓存？

```
┌─────────────────────────────────────────────────────────────┐
│                    前端 (React)                              │
│    ┌─────────────────────────────────────────────────┐      │
│    │              IndexedDB 缓存                      │      │
│    │    hash -> base64 (内存快速访问)                 │      │
│    └──────────────────────┬──────────────────────────┘      │
│                           │ cache miss                       │
│                           ▼                                  │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    后端 (Tauri/Rust)                         │
│    ┌─────────────────────────────────────────────────┐      │
│    │              文件系统缓存                         │      │
│    │    hash.jpg (磁盘持久化)                         │      │
│    └─────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

**优点：**
1. ✅ IndexedDB 访问比 `invoke()` 调用快得多（无 IPC 开销）
2. ✅ 减少文件系统 I/O
3. ✅ 适合频繁显示的图片（如产品列表）

### IndexedDB 存储结构

```typescript
// 数据库名: red_coral_image_cache
// Store名: images

interface CachedImage {
  hash: string;        // 主键
  data: string;        // base64 数据
  cachedAt: number;    // 缓存时间戳
}
```

### 图片加载流程（含 IndexedDB）

```
getImageUrl(hash)
       │
       ▼
检查 IndexedDB
       │
       ├── 命中 → 返回 base64 data URL
       │
       └── 未命中 → 调用 invoke('get_image_path')
             │
             ▼
       后端返回本地路径
             │
             ▼
       convertFileSrc() → 显示图片
             │
             ▼
       读取文件为 base64 → 存入 IndexedDB
```

### IndexedDB 实现

```typescript
// src/utils/image/indexedDBCache.ts

const DB_NAME = 'red_coral_image_cache';
const DB_VERSION = 1;
const STORE_NAME = 'images';

let dbPromise: Promise<IDBDatabase> | null = null;

function openDB(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise;

  dbPromise = new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'hash' });
      }
    };

    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });

  return dbPromise;
}

export async function getCachedImage(hash: string): Promise<string | null> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readonly');
    const store = tx.objectStore(STORE_NAME);
    const request = store.get(hash);

    request.onsuccess = () => {
      const result = request.result as CachedImage | undefined;
      resolve(result?.data ?? null);
    };
    request.onerror = () => reject(request.error);
  });
}

export async function setCachedImage(hash: string, data: string): Promise<void> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const request = store.put({
      hash,
      data,
      cachedAt: Date.now(),
    });

    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}

export async function deleteCachedImage(hash: string): Promise<void> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const request = store.delete(hash);

    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}

export async function clearAllCachedImages(): Promise<void> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const request = store.clear();

    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}
```

### 更新后的 getImageUrl

```typescript
// src/utils/image/index.ts

import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { getCachedImage, setCachedImage } from './indexedDBCache';
import DefaultImage from '@/assets/reshot.svg';

export async function getImageUrl(hash: string | null | undefined): Promise<string> {
  if (!hash) {
    return DefaultImage;
  }

  try {
    // 1. 先查 IndexedDB
    const cached = await getCachedImage(hash);
    if (cached) {
      return cached; // data:image/jpeg;base64,...
    }

    // 2. 调用后端获取本地路径
    const localPath = await invoke<string>('get_image_path', { hash });
    const fileUrl = convertFileSrc(localPath);

    // 3. 后台加载并缓存到 IndexedDB（不阻塞返回）
    cacheImageToIndexedDB(hash, fileUrl);

    return fileUrl;
  } catch (error) {
    console.error('Failed to get image:', hash, error);
    return DefaultImage;
  }
}

// 后台缓存（不阻塞）
async function cacheImageToIndexedDB(hash: string, url: string) {
  try {
    const response = await fetch(url);
    const blob = await response.blob();
    const reader = new FileReader();

    reader.onloadend = async () => {
      const base64 = reader.result as string;
      await setCachedImage(hash, base64);
    };

    reader.readAsDataURL(blob);
  } catch (e) {
    console.warn('Failed to cache image to IndexedDB:', hash, e);
  }
}
```

---

## 图片同步与缓存失效

### 问题：如何知道图片已更新？

由于使用 **content hash 作为文件名**，同一个 hash 的内容永远不会变：

- **Product A 图片 hash = "abc123"**
- **更新 Product A 图片** → 新 hash = "def456"
- **数据库中 Product A 的 image 字段变为 "def456"**

所以：
1. ✅ 旧 hash "abc123" 的缓存仍然有效（内容未变）
2. ✅ 新 hash "def456" 会触发新的下载
3. ✅ **不需要主动失效缓存**，只需用新 hash 加载

### 处理 Sync 事件

```typescript
// src/core/hooks/useSyncListener.ts

import { listen } from '@tauri-apps/api/event';
import { useProductStore } from '@/core/stores/resources/useProductStore';

export function useSyncListener() {
  useEffect(() => {
    const unlisten = listen('sync', (event) => {
      const { table, data } = event.payload as SyncPayload;

      if (table === 'products') {
        // 重新获取产品列表
        // 新的 image hash 会自动触发新图片加载
        useProductStore.getState().fetchAll();
      }
    });

    return () => { unlisten.then(fn => fn()); };
  }, []);
}
```

### 处理 Sync 事件丢失

参见上方「多端同步问题分析 - 场景 1」的解决方案。

**核心思路：** 重连时触发全量数据刷新，新的 hash 自然会触发新图片加载。

---

## 缓存清理策略

### IndexedDB 缓存清理

由于 hash 是内容寻址的，旧图片可能不再被引用：

```typescript
// src/utils/image/cacheCleanup.ts

export async function cleanupUnusedImages(activeHashes: string[]) {
  const db = await openDB();
  const tx = db.transaction(STORE_NAME, 'readwrite');
  const store = tx.objectStore(STORE_NAME);

  // 获取所有缓存的 hash
  const allKeys = await new Promise<string[]>((resolve, reject) => {
    const request = store.getAllKeys();
    request.onsuccess = () => resolve(request.result as string[]);
    request.onerror = () => reject(request.error);
  });

  // 删除不在 activeHashes 中的缓存
  const activeSet = new Set(activeHashes);
  for (const key of allKeys) {
    if (!activeSet.has(key)) {
      store.delete(key);
    }
  }
}

// 使用示例：在产品列表加载后清理
async function onProductsLoaded(products: Product[]) {
  const activeHashes = products.map(p => p.image).filter(Boolean);
  await cleanupUnusedImages(activeHashes);
}
```

### 后端文件缓存清理

Client 模式的 `image_cache/images/` 目录可以定期清理：

```rust
// 保留最近 30 天使用的图片
impl ImageCacheService {
    pub async fn cleanup_old_cache(&self, max_age_days: u32) -> Result<u32, ImageCacheError> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            - (max_age_days as u64 * 24 * 60 * 60);

        let mut removed = 0;
        let mut entries = tokio::fs::read_dir(&self.images_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if let Ok(accessed) = metadata.accessed() {
                let accessed_ts = accessed.duration_since(UNIX_EPOCH)?.as_secs();
                if accessed_ts < cutoff {
                    tokio::fs::remove_file(entry.path()).await?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}
