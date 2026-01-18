# Sync 信号系统设计

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现多端数据同步，当任一客户端修改数据时，其他客户端自动更新。

**Architecture:** 基于现有 Message Bus 广播机制，Sync 信号携带资源类型、版本号、操作类型和完整数据。前端根据版本差距决定增量更新或全量拉取。

**Tech Stack:** Rust (后端), TypeScript/React/Zustand (前端), Tauri Events

---

## 1. 概述

### 核心机制

```
数据变更 → broadcast_sync(resource, version, action, id, data)
         → 所有客户端收到 Sync 信号
         → 前端比较版本号：
            - 差距 ≤ 5：使用携带的 data 直接更新本地缓存
            - 差距 > 5：全量拉取该资源列表
```

### Sync 信号结构

```typescript
interface SyncPayload {
  resource: string;    // "product" | "category" | "tag" | ...
  version: number;     // 递增版本号
  action: string;      // "created" | "updated" | "deleted"
  id: string;          // 实体 ID
  data: T | null;      // 完整实体数据（deleted 时为 null）
}
```

### 同步范围

所有可修改的业务实体：
- 菜单相关：product, category, tag, attribute, spec
- 位置相关：zone, table
- 人员相关：employee, role
- 其他：price_rule, kitchen_printer

---

## 2. 后端实现

### 版本号管理

在 ServerState 中新增 ResourceVersions：

```rust
// edge-server/src/core/state.rs
use dashmap::DashMap;

pub struct ResourceVersions {
    versions: DashMap<String, u64>,  // resource_type -> version
}

impl ResourceVersions {
    pub fn new() -> Self {
        Self {
            versions: DashMap::new(),
        }
    }

    pub fn increment(&self, resource: &str) -> u64 {
        let mut entry = self.versions.entry(resource.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    pub fn get(&self, resource: &str) -> u64 {
        self.versions.get(resource).map(|v| *v).unwrap_or(0)
    }
}
```

### 改造 broadcast_sync

```rust
// 现有签名
pub async fn broadcast_sync<T: Serialize>(
    &self,
    resource: &str,
    id: Option<&str>,
    action: &str,
    data: Option<&T>
)

// 改为自动递增版本号
pub async fn broadcast_sync<T: Serialize>(
    &self,
    resource: &str,
    id: &str,          // 改为必填
    action: &str,
    data: Option<&T>
) {
    let version = self.resource_versions.increment(resource);
    let payload = SyncPayload {
        resource: resource.to_string(),
        version,
        action: action.to_string(),
        id: id.to_string(),
        data: data.map(|d| serde_json::to_value(d).ok()).flatten(),
    };
    self.message_bus().publish(Message::Sync(payload));
}
```

### 修改 SyncPayload

```rust
// shared/src/message/payload.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub resource: String,
    pub version: u64,      // 新增
    pub action: String,
    pub id: String,        // 改为必填
    pub data: Option<serde_json::Value>,
}
```

---

## 3. 前端实现

### 本地版本号存储

每个资源 store 新增 version 字段：

```typescript
// 示例：useProductStore
interface ProductStore {
  products: Product[];
  version: number;      // 本地版本号
  isLoaded: boolean;    // 是否已加载过数据

  fetchAll: () => Promise<void>;
  applySync: (action: string, id: string, data: Product | null) => void;
}
```

### 全局 Sync 监听 Hook

```typescript
// src/core/hooks/useSyncListener.ts
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

const SYNC_THRESHOLD = 5;

// 资源类型到 store 的映射
const storeMap: Record<string, () => any> = {
  product: () => useProductStore.getState(),
  category: () => useCategoryStore.getState(),
  tag: () => useTagStore.getState(),
  attribute: () => useAttributeStore.getState(),
  spec: () => useSpecStore.getState(),
  zone: () => useZoneStore.getState(),
  table: () => useTableStore.getState(),
  employee: () => useEmployeeStore.getState(),
  role: () => useRoleStore.getState(),
  price_rule: () => usePriceRuleStore.getState(),
  kitchen_printer: () => useKitchenPrinterStore.getState(),
};

interface SyncPayload {
  resource: string;
  version: number;
  action: string;
  id: string;
  data: any | null;
}

export function useSyncListener() {
  useEffect(() => {
    const unlisten = listen<{ type: string; payload: SyncPayload }>('server-message', (event) => {
      const message = event.payload;
      if (message.type !== 'Sync') return;

      const { resource, version, action, id, data } = message.payload;

      const getStore = storeMap[resource];
      if (!getStore) return;  // 未知资源类型，忽略

      const store = getStore();
      if (!store.isLoaded) return;  // 该资源未加载，忽略

      const localVersion = store.version || 0;
      const gap = version - localVersion;

      if (gap <= 0) return;  // 已处理过或旧消息，忽略

      if (gap <= SYNC_THRESHOLD) {
        // 增量更新
        store.applySync(action, id, data);
        store.setVersion(version);
      } else {
        // 版本差距太大，全量拉取
        console.log(`[Sync] Version gap ${gap} > ${SYNC_THRESHOLD}, full refresh for ${resource}`);
        store.fetchAll();
      }
    });

    return () => { unlisten.then(fn => fn()); };
  }, []);
}
```

### Store 的 applySync 方法

```typescript
// 示例实现（各 store 类似）
applySync: (action: string, id: string, data: Product | null) => {
  set(state => {
    switch (action) {
      case 'created':
        if (!data) return state;
        return { products: [...state.products, data] };

      case 'updated':
        if (!data) return state;
        return {
          products: state.products.map(p => p.id === id ? data : p)
        };

      case 'deleted':
        return {
          products: state.products.filter(p => p.id !== id)
        };

      default:
        return state;
    }
  });
},

setVersion: (version: number) => {
  set({ version });
},
```

---

## 4. 连接恢复处理

### 场景

客户端断连后重新连接，服务端版本号可能已重置（服务重启）或跳跃很大。

### 处理策略

```typescript
// src/core/hooks/useConnectionRecovery.ts
import { useEffect, useRef } from 'react';
import { useBridgeConnectionStatus } from '@/core/stores/bridge';

export function useConnectionRecovery() {
  const connectionStatus = useBridgeConnectionStatus();
  const prevConnected = useRef(connectionStatus.connected);

  useEffect(() => {
    // 检测从断开到连接的转换
    if (!prevConnected.current && connectionStatus.connected) {
      console.log('[Sync] Connection recovered, refreshing all loaded stores');
      refreshAllLoadedStores();
    }
    prevConnected.current = connectionStatus.connected;
  }, [connectionStatus.connected]);
}

function refreshAllLoadedStores() {
  const stores = [
    useProductStore,
    useCategoryStore,
    useTagStore,
    useAttributeStore,
    useSpecStore,
    useZoneStore,
    useTableStore,
    useEmployeeStore,
    useRoleStore,
    usePriceRuleStore,
    useKitchenPrinterStore,
  ];

  stores.forEach(useStore => {
    const store = useStore.getState();
    if (store.isLoaded) {
      store.fetchAll();
    }
  });
}
```

### 为什么重连后全量刷新

- 服务可能重启过，版本号从 0 开始
- 断连期间可能错过很多 Sync 信号
- 一次性全量刷新比逐个比对版本号更简单可靠

---

## 5. 实现任务清单

### 后端改动

1. **shared/src/message/payload.rs** - SyncPayload 新增 `version` 字段，`id` 改为必填
2. **edge-server/src/core/state.rs** - 新增 ResourceVersions 结构，改造 broadcast_sync 方法
3. **检查所有 CRUD handler** - 确保调用 broadcast_sync 时传入正确参数

### 前端改动

1. **各资源 store** - 新增 `version`、`isLoaded` 字段和 `applySync`、`setVersion` 方法
2. **src/core/hooks/useSyncListener.ts** - 新增全局 Sync 监听 hook
3. **src/core/hooks/useConnectionRecovery.ts** - 新增连接恢复处理 hook
4. **App.tsx** - 挂载 useSyncListener 和 useConnectionRecovery

### 涉及的 Store

- useProductStore, useCategoryStore, useTagStore
- useAttributeStore, useSpecStore
- useZoneStore, useTableStore
- useEmployeeStore, useRoleStore
- usePriceRuleStore, useKitchenPrinterStore

### 测试场景

1. 单端修改菜品 → 其他端自动更新（增量）
2. 批量修改（版本差距 > 5）→ 触发全量拉取
3. 客户端断连重连 → 自动全量刷新
4. 服务重启 → 客户端检测到版本异常，全量刷新
