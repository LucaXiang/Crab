# 资源同步机制改进设计

> 日期: 2026-01-22
> 状态: 待实现
> 范围: 静态资源 (products, categories, zones, tables 等)

## 背景

当前静态资源同步存在以下技术债：

1. **全量刷新策略** - `applySync` 忽略 `data` 字段，每次都 `fetchAll()`
2. **Hacky 去重机制** - 使用 `pendingIds` + 1秒时间窗口
3. **无 Gap 检测** - 无法发现消息丢失
4. **无断线重连处理** - 断线期间错过的 Sync 消息无法恢复

## 设计目标

- 引入 Version 机制，支持增量更新和 Gap 检测
- 引入 Epoch 机制，检测服务器重启
- 实现断线重连后的状态恢复
- 移除 `pendingIds` hacky 方案

## 架构设计

### 数据流

```
后端 CRUD 操作
     ↓
broadcast_sync(resource, action, id, data, version)
     ↓
MessageBus → server-message 事件
     ↓
useSyncListener 解析 payload
     ↓
store.applySync({ id, version, action, data })
     ↓
┌─────────────────────────────────────┐
│  version === lastVersion + 1?       │
│    ├─ Yes → 增量更新                │
│    └─ No  → 全量刷新 fetchAll()     │
└─────────────────────────────────────┘
```

### 断线重连流程

```
客户端重连成功
     ↓
调用 GET /api/sync/status
     ↓
返回 { epoch, versions: { product: 10, category: 5, ... } }
     ↓
┌─────────────────────────────────────────┐
│  epoch !== 本地 cachedEpoch?            │
│    ├─ Yes → 全量刷新所有已加载的 Store   │
│    └─ No  → 逐个比对 version            │
│              └─ version > local → 刷新   │
└─────────────────────────────────────────┘
```

## API 设计

### 新增接口: `GET /api/sync/status`

**响应结构:**

```rust
// shared/src/models/sync.rs
#[derive(Serialize, Deserialize)]
pub struct SyncStatus {
    /// 服务器实例 epoch (启动时生成的 UUID)
    pub epoch: String,
    /// 各资源类型的当前版本
    pub versions: HashMap<String, u64>,
}
```

**响应示例:**

```json
{
  "epoch": "550e8400-e29b-41d4-a716-446655440000",
  "versions": {
    "product": 42,
    "category": 15,
    "zone": 3,
    "dining_table": 8,
    "employee": 5,
    "attribute": 12,
    "tag": 7,
    "price_rule": 2,
    "print_destination": 4
  }
}
```

## 前端实现

### 1. 改造 `createResourceStore`

```typescript
// stores/factory/createResourceStore.ts

export interface ResourceStore<T extends { id: string }> {
  items: T[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;
  lastVersion: number;  // 新增

  fetchAll: (force?: boolean) => Promise<void>;
  applySync: (payload: SyncPayload<T>) => void;  // 改签名
  checkVersion: (serverVersion: number) => boolean;  // 新增
  getById: (id: string) => T | undefined;
  clear: () => void;
}

interface SyncPayload<T> {
  id: string;
  version: number;
  action: 'created' | 'updated' | 'deleted';
  data: T | null;
}

export function createResourceStore<T extends { id: string }>(
  resourceName: string,
  fetchFn: () => Promise<T[]>
) {
  return create<ResourceStore<T>>((set, get) => ({
    items: [],
    isLoading: false,
    isLoaded: false,
    error: null,
    lastVersion: 0,

    fetchAll: async (force = false) => {
      const state = get();
      if (state.isLoading) return;
      if (state.isLoaded && !force) return;

      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        set({ items, isLoading: false, isLoaded: true });
      } catch (e: any) {
        set({ error: e.message || 'Failed to fetch', isLoading: false });
      }
    },

    applySync: (payload: SyncPayload<T>) => {
      const { id, version, action, data } = payload;
      const state = get();

      // 1. 重复消息检测
      if (version <= state.lastVersion) {
        console.debug(`[${resourceName}] Skip duplicate sync v${version}`);
        return;
      }

      // 2. Gap 检测
      if (version > state.lastVersion + 1) {
        console.warn(`[${resourceName}] Version gap: ${state.lastVersion} → ${version}`);
        get().fetchAll(true);
        return;
      }

      // 3. 增量更新
      let newItems = state.items;

      if (action === 'created' && data) {
        newItems = [...state.items, data];
      } else if (action === 'updated' && data) {
        newItems = state.items.map(item => item.id === id ? data : item);
      } else if (action === 'deleted') {
        newItems = state.items.filter(item => item.id !== id);
      }

      set({ items: newItems, lastVersion: version });
    },

    checkVersion: (serverVersion: number) => {
      return serverVersion > get().lastVersion;
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null, lastVersion: 0 }),
  }));
}
```

### 2. 改造 `useSyncListener`

```typescript
// hooks/useSyncListener.ts

interface SyncPayload {
  resource: string;
  action: 'created' | 'updated' | 'deleted';
  id: string;
  version: number;
  data: unknown | null;
}

export function useSyncListener() {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ServerMessageEvent>('server-message', async (event) => {
      const message = event.payload;
      if (message.event_type.toLowerCase() !== 'sync') return;

      // Lagged 处理保持不变
      if (isLaggedPayload(message.payload)) {
        // ... 现有逻辑
        return;
      }

      const payload = message.payload as SyncPayload;
      const { resource, id, version, action, data } = payload;

      // Order sync 特殊处理保持不变
      if (resource === 'order_sync') {
        // ... 现有逻辑
        return;
      }

      // 静态资源：传递完整 payload
      const store = storeRegistry[resource];
      if (store) {
        store.getState().applySync({ id, version, action, data });
      }
    }).then(fn => { unlisten = fn; });

    return () => unlisten?.();
  }, []);
}
```

### 3. 新增 `useSyncConnection`

```typescript
// hooks/useSyncConnection.ts

import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invokeApi } from '@/infrastructure/api';
import { storeRegistry, getLoadedStores, refreshAllLoadedStores } from '@/core/stores/resources';

interface SyncStatus {
  epoch: string;
  versions: Record<string, number>;
}

let cachedEpoch: string | null = null;

export function useSyncConnection() {
  const isReconnecting = useRef(false);

  useEffect(() => {
    const handleConnectionChange = async (connected: boolean) => {
      if (!connected || isReconnecting.current) return;

      isReconnecting.current = true;
      console.log('[SyncConnection] Reconnected, checking sync status...');

      try {
        const status = await invokeApi<SyncStatus>('get_sync_status');

        // Epoch 检查
        if (cachedEpoch && cachedEpoch !== status.epoch) {
          console.warn('[SyncConnection] Epoch changed, full refresh');
          cachedEpoch = status.epoch;
          await refreshAllLoadedStores();
          return;
        }

        cachedEpoch = status.epoch;

        // Version 比对
        const loadedStores = getLoadedStores();
        const staleStores: string[] = [];

        for (const [name, store] of loadedStores) {
          const serverVersion = status.versions[name] || 0;
          if (store.getState().checkVersion(serverVersion)) {
            staleStores.push(name);
          }
        }

        if (staleStores.length > 0) {
          console.log(`[SyncConnection] Refreshing: ${staleStores.join(', ')}`);
          await Promise.all(
            staleStores.map(name => storeRegistry[name].getState().fetchAll(true))
          );
        }
      } catch (err) {
        console.error('[SyncConnection] Check failed, fallback to full refresh');
        await refreshAllLoadedStores();
      } finally {
        isReconnecting.current = false;
      }
    };

    const unlisten = listen<boolean>('connection-state-changed', (event) => {
      handleConnectionChange(event.payload);
    });

    return () => { unlisten.then(fn => fn()); };
  }, []);
}
```

### 4. Registry 接口更新

```typescript
// stores/resources/registry.ts

interface RegistryStore {
  getState: () => {
    isLoaded: boolean;
    lastVersion: number;
    fetchAll: (force?: boolean) => Promise<void>;
    applySync: (payload: SyncPayload) => void;
    checkVersion: (serverVersion: number) => boolean;
    clear: () => void;
  };
}
```

## 后端实现

### 1. ServerState 添加 epoch

```rust
// edge-server/src/core/state.rs

pub struct ServerState {
    // ... 现有字段
    pub epoch: String,  // 新增
}

impl ServerState {
    pub async fn initialize(config: &Config) -> Self {
        // ...
        let epoch = uuid::Uuid::new_v4().to_string();
        // ...
    }
}
```

### 2. 新增 Sync API

```rust
// edge-server/src/api/sync/handler.rs

use crate::core::ServerState;
use axum::{Json, extract::State};
use shared::models::SyncStatus;
use std::collections::HashMap;

pub async fn get_sync_status(
    State(state): State<ServerState>,
) -> Json<SyncStatus> {
    let mut versions = HashMap::new();

    // 从 ResourceVersions 获取各资源当前版本
    for resource in &[
        "product", "category", "tag", "attribute",
        "zone", "dining_table", "employee", "role",
        "price_rule", "print_destination"
    ] {
        versions.insert(resource.to_string(), state.resource_versions.get(resource));
    }

    Json(SyncStatus {
        epoch: state.epoch.clone(),
        versions,
    })
}
```

### 3. 路由注册

```rust
// edge-server/src/api/mod.rs

Router::new()
    // ... 现有路由
    .route("/api/sync/status", get(sync::handler::get_sync_status))
```

## 改动清单

| 文件 | 类型 | 描述 |
|-----|------|-----|
| `shared/src/models/mod.rs` | 修改 | 导出 `sync` 模块 |
| `shared/src/models/sync.rs` | 新增 | `SyncStatus` 结构体 |
| `edge-server/src/core/state.rs` | 修改 | 添加 `epoch` 字段 |
| `edge-server/src/api/sync/mod.rs` | 新增 | 模块定义 |
| `edge-server/src/api/sync/handler.rs` | 新增 | `get_sync_status` |
| `edge-server/src/api/mod.rs` | 修改 | 注册路由 |
| `red_coral/src-tauri/src/commands/` | 修改 | 添加 `get_sync_status` command |
| `createResourceStore.ts` | 修改 | 增量更新 + version 管理 |
| `useSyncListener.ts` | 修改 | 传递完整 payload |
| `useSyncConnection.ts` | 新增 | 断线重连管理 |
| `registry.ts` | 修改 | 接口更新 |

## 删除的代码

```typescript
// createResourceStore.ts 中删除:
const PENDING_EXPIRY_MS = 1000;
const pendingIds = new Set<string>();
const addPendingId = (id: string) => { ... };
```

## 测试要点

1. **增量更新**: 创建/更新/删除资源，验证 UI 即时反映变化
2. **Gap 检测**: 模拟丢失 Sync 消息，验证触发全量刷新
3. **Epoch 检测**: 重启服务器，验证客户端全量刷新
4. **断线重连**: 断开网络再恢复，验证状态同步正确

## 后续优化（可选）

- [ ] 添加 `lastSyncTime` 用于调试
- [ ] 实现 Sync 消息压缩（批量更新合并）
- [ ] 添加 version 持久化（localStorage）避免刷新页面丢失
