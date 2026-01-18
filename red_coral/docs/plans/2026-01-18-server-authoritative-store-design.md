# 服务器权威 Store 架构设计

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 重构前端 Store 架构，实现服务器权威的数据同步模型。

**Architecture:** 工厂函数生成统一结构的 Store，收到 Sync 信号直接全量刷新，启动时预加载核心数据。

**Tech Stack:** TypeScript, React, Zustand, Tauri Events

---

## 1. 核心原则

- **服务器是唯一真相来源** - 客户端只是"渲染"服务器数据
- **收到 Sync → 直接全量刷新** - 不做增量更新，不比对版本
- **重连 → 全量刷新所有已加载的 Store**
- **组件必须直接订阅 Store** - 禁止 useState + useEffect 手动 fetch

---

## 2. 架构图

```
┌─────────────────────────────────────────────┐
│              createResourceStore            │
│         (工厂函数，生成统一结构)              │
└─────────────────────────────────────────────┘
                      │
    ┌─────────────────┼─────────────────┐
    ▼                 ▼                 ▼
┌─────────┐    ┌───────────┐    ┌───────────┐
│ product │    │ category  │    │  table    │  ... (13 种)
│  Store  │    │   Store   │    │   Store   │
└─────────┘    └───────────┘    └───────────┘
                      │
                      ▼
┌─────────────────────────────────────────────┐
│             useSyncListener                 │
│   (统一监听，根据 resource 分发到对应 Store)  │
└─────────────────────────────────────────────┘
```

---

## 3. 同步资源列表（13 种）

| 类别 | 资源 |
|------|------|
| 菜单相关 | product, category, tag, attribute, spec |
| 位置相关 | zone, table |
| 人员相关 | employee, role |
| 其他 | price_rule, kitchen_printer, order |

---

## 4. createResourceStore 工厂函数

```typescript
// src/core/stores/factory/createResourceStore.ts
import { create } from 'zustand';

export interface ResourceStore<T extends { id: string }> {
  // 状态
  items: T[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // 方法
  fetchAll: () => Promise<void>;
  applySync: () => void;
  getById: (id: string) => T | undefined;
  clear: () => void;
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

    fetchAll: async () => {
      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        set({ items, isLoading: false, isLoaded: true });
      } catch (e: any) {
        set({ error: e.message || 'Failed to fetch', isLoading: false });
      }
    },

    // 服务器权威：收到 Sync 直接全量刷新
    applySync: () => {
      if (get().isLoaded) {
        get().fetchAll();
      }
    },

    getById: (id) => get().items.find(item => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null }),
  }));
}
```

---

## 5. 资源 Store 定义（示例）

```typescript
// src/core/stores/resources/useProductStore.ts
import { createResourceStore } from '../factory/createResourceStore';
import { Product } from '@/types';
import { listProducts } from '@/api/product';

export const useProductStore = createResourceStore<Product>(
  'product',
  listProducts
);

// 便捷 hooks
export const useProducts = () => useProductStore(state => state.items);
export const useProductsLoading = () => useProductStore(state => state.isLoading);
export const useProductById = (id: string) =>
  useProductStore(state => state.items.find(p => p.id === id));
```

---

## 6. Store 注册表

```typescript
// src/core/stores/resources/registry.ts
import { useProductStore } from './useProductStore';
import { useCategoryStore } from './useCategoryStore';
import { useTagStore } from './useTagStore';
import { useAttributeStore } from './useAttributeStore';
import { useSpecStore } from './useSpecStore';
import { useZoneStore } from './useZoneStore';
import { useTableStore } from './useTableStore';
import { useEmployeeStore } from './useEmployeeStore';
import { useRoleStore } from './useRoleStore';
import { usePriceRuleStore } from './usePriceRuleStore';
import { useKitchenPrinterStore } from './useKitchenPrinterStore';
import { useOrderStore } from './useOrderStore';

export const storeRegistry: Record<string, any> = {
  product: useProductStore,
  category: useCategoryStore,
  tag: useTagStore,
  attribute: useAttributeStore,
  spec: useSpecStore,
  zone: useZoneStore,
  table: useTableStore,
  employee: useEmployeeStore,
  role: useRoleStore,
  price_rule: usePriceRuleStore,
  kitchen_printer: useKitchenPrinterStore,
  order: useOrderStore,
};
```

---

## 7. useSyncListener

```typescript
// src/core/hooks/useSyncListener.ts
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { storeRegistry } from '@/core/stores/resources/registry';

interface ServerMessageEvent {
  event_type: string;
  payload: {
    resource: string;
    action: string;
    id: string;
    data: any;
  };
}

export function useSyncListener() {
  useEffect(() => {
    const unlisten = listen<ServerMessageEvent>('server-message', (event) => {
      const message = event.payload;

      if (message.event_type !== 'Sync') return;

      const { resource } = message.payload;
      const store = storeRegistry[resource];

      if (store) {
        console.log(`[Sync] Received: ${resource}, triggering refresh`);
        store.getState().applySync();
      } else {
        console.log(`[Sync] Unknown resource: ${resource}`);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
```

---

## 8. useConnectionRecovery

```typescript
// src/core/hooks/useConnectionRecovery.ts
import { useEffect, useRef } from 'react';
import { useBridgeConnectionStatus } from '@/core/stores/bridge';
import { storeRegistry } from '@/core/stores/resources/registry';

export function useConnectionRecovery() {
  const connectionStatus = useBridgeConnectionStatus();
  const prevConnected = useRef(connectionStatus.connected);

  useEffect(() => {
    if (!prevConnected.current && connectionStatus.connected) {
      console.log('[Sync] Connection recovered, refreshing all stores');
      refreshAllLoadedStores();
    }
    prevConnected.current = connectionStatus.connected;
  }, [connectionStatus.connected]);
}

function refreshAllLoadedStores() {
  Object.entries(storeRegistry).forEach(([name, store]) => {
    if (store.getState().isLoaded) {
      console.log(`[Sync] Refreshing ${name} store`);
      store.getState().fetchAll();
    }
  });
}
```

---

## 9. usePreloadCoreData

```typescript
// src/core/hooks/usePreloadCoreData.ts
import { useState, useEffect } from 'react';
import { useZoneStore } from '@/core/stores/resources/useZoneStore';
import { useTableStore } from '@/core/stores/resources/useTableStore';
import { useCategoryStore } from '@/core/stores/resources/useCategoryStore';
import { useProductStore } from '@/core/stores/resources/useProductStore';

// 核心资源：启动时预加载
const CORE_STORES = [
  useZoneStore,
  useTableStore,
  useCategoryStore,
  useProductStore,
];

export function usePreloadCoreData() {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const preload = async () => {
      console.log('[Preload] Loading core data...');
      await Promise.all(
        CORE_STORES.map(store => store.getState().fetchAll())
      );
      console.log('[Preload] Core data ready');
      setReady(true);
    };
    preload();
  }, []);

  return ready;
}
```

---

## 10. App.tsx 集成

```typescript
function App() {
  const coreDataReady = usePreloadCoreData();

  useSyncListener();
  useConnectionRecovery();

  if (!coreDataReady) {
    return <LoadingScreen />;
  }

  return <Routes>...</Routes>;
}
```

---

## 11. 组件使用规范

**正确用法**：
```typescript
function ProductList() {
  const products = useProducts();  // 直接订阅
  const isLoading = useProductsLoading();

  if (isLoading) return <Loading />;

  return (
    <div>
      {products.map(p => <ProductCard key={p.id} product={p} />)}
    </div>
  );
}
```

**禁止用法**：
```typescript
// ❌ 禁止：本地 state + 手动 fetch
function ProductList() {
  const [products, setProducts] = useState([]);

  useEffect(() => {
    fetchProducts().then(setProducts);
  }, []);
}
```

---

## 12. 文件结构

```
src/core/stores/
├── factory/
│   └── createResourceStore.ts
├── resources/
│   ├── index.ts
│   ├── registry.ts
│   ├── useProductStore.ts
│   ├── useCategoryStore.ts
│   ├── useTagStore.ts
│   ├── useAttributeStore.ts
│   ├── useSpecStore.ts
│   ├── useZoneStore.ts
│   ├── useTableStore.ts
│   ├── useEmployeeStore.ts
│   ├── useRoleStore.ts
│   ├── usePriceRuleStore.ts
│   ├── useKitchenPrinterStore.ts
│   └── useOrderStore.ts
├── bridge/
│   └── useBridgeStore.ts (保留)
└── index.ts

src/core/hooks/
├── useSyncListener.ts (重写)
├── useConnectionRecovery.ts (重写)
├── usePreloadCoreData.ts (新增)
└── index.ts
```

---

## 13. 实现任务清单

### 阶段 1：基础设施
1. 创建 `createResourceStore.ts` 工厂函数
2. 创建 `registry.ts` 注册表

### 阶段 2：资源 Store（13 个）
3. useProductStore
4. useCategoryStore
5. useTagStore
6. useAttributeStore
7. useSpecStore
8. useZoneStore
9. useTableStore
10. useEmployeeStore
11. useRoleStore
12. usePriceRuleStore
13. useKitchenPrinterStore
14. useOrderStore

### 阶段 3：Hooks
15. 重写 useSyncListener
16. 重写 useConnectionRecovery
17. 新增 usePreloadCoreData

### 阶段 4：集成
18. 更新 App.tsx
19. 改造现有组件

### 阶段 5：清理
20. 删除旧 Store 文件
21. 删除旧的 sync 相关代码

---

## 14. 总结

| 项目 | 内容 |
|------|------|
| 架构 | 工厂函数 + 13 个统一结构的 Store |
| 同步策略 | 服务器权威，收到 Sync 直接全量刷新 |
| 预加载 | 核心 4 种（product, category, table, zone） |
| 组件规范 | 必须直接订阅 Store |
| 优点 | 简单、可靠、易维护 |
