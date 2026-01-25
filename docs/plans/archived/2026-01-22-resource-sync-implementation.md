# Resource Sync Improvement Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement version-based incremental sync with epoch detection for static resources.

**Architecture:** Add `epoch` to ServerState, create `/api/sync/status` endpoint, refactor frontend stores to use version-based incremental updates instead of full refresh, add reconnection sync via `useSyncConnection` hook.

**Tech Stack:** Rust (axum, serde), TypeScript (Zustand, Tauri API)

---

## Task 1: Add SyncStatus Model (shared)

**Files:**
- Create: `shared/src/models/sync.rs`
- Modify: `shared/src/models/mod.rs`

**Step 1: Create sync.rs with SyncStatus struct**

```rust
// shared/src/models/sync.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 同步状态响应
///
/// 用于客户端重连时检查资源版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// 服务器实例 epoch (启动时生成的 UUID)
    /// 用于检测服务器重启
    pub epoch: String,
    /// 各资源类型的当前版本
    pub versions: HashMap<String, u64>,
}
```

**Step 2: Export sync module in mod.rs**

Add to `shared/src/models/mod.rs`:
```rust
pub mod sync;
pub use sync::*;
```

**Step 3: Verify compilation**

Run: `cargo check -p shared`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add shared/src/models/sync.rs shared/src/models/mod.rs
git commit -m "feat(shared): add SyncStatus model for version-based sync"
```

---

## Task 2: Add epoch to ServerState (edge-server)

**Files:**
- Modify: `edge-server/src/core/state.rs`

**Step 1: Add epoch field to ServerState struct**

Find the `ServerState` struct and add the `epoch` field:

```rust
pub struct ServerState {
    // ... existing fields ...
    /// 服务器实例 epoch (启动时生成的 UUID)
    /// 用于客户端检测服务器重启
    pub epoch: String,
}
```

**Step 2: Update ServerState::new() to accept epoch**

Add `epoch: String` parameter to the `new()` function and assign it.

**Step 3: Generate epoch in ServerState::initialize()**

In the `initialize()` function, generate epoch before creating ServerState:
```rust
let epoch = uuid::Uuid::new_v4().to_string();
```

Pass it to `Self::new(...)`.

**Step 4: Verify compilation**

Run: `cargo check -p edge-server`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add edge-server/src/core/state.rs
git commit -m "feat(edge-server): add epoch to ServerState for restart detection"
```

---

## Task 3: Create Sync API Module (edge-server)

**Files:**
- Create: `edge-server/src/api/sync/mod.rs`
- Create: `edge-server/src/api/sync/handler.rs`
- Modify: `edge-server/src/api/mod.rs`

**Step 1: Create sync module directory and mod.rs**

```rust
// edge-server/src/api/sync/mod.rs
pub mod handler;

pub use handler::*;
```

**Step 2: Create handler.rs with get_sync_status**

```rust
// edge-server/src/api/sync/handler.rs
//! Sync API Handlers

use axum::{Json, extract::State};
use shared::models::SyncStatus;
use std::collections::HashMap;

use crate::core::ServerState;

/// 资源类型列表 (必须与前端 registry 保持一致)
const RESOURCE_TYPES: &[&str] = &[
    "product",
    "category",
    "tag",
    "attribute",
    "zone",
    "dining_table",
    "employee",
    "role",
    "price_rule",
    "print_destination",
];

/// GET /api/sync/status - 获取同步状态
///
/// 返回服务器 epoch 和各资源类型的当前版本号
/// 客户端重连时调用此接口检查是否需要刷新
pub async fn get_sync_status(
    State(state): State<ServerState>,
) -> Json<SyncStatus> {
    let mut versions = HashMap::new();

    for resource in RESOURCE_TYPES {
        versions.insert(resource.to_string(), state.resource_versions.get(resource));
    }

    Json(SyncStatus {
        epoch: state.epoch.clone(),
        versions,
    })
}
```

**Step 3: Export sync module in api/mod.rs**

Add to `edge-server/src/api/mod.rs`:
```rust
pub mod sync;
```

**Step 4: Verify compilation**

Run: `cargo check -p edge-server`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add edge-server/src/api/sync/
git add edge-server/src/api/mod.rs
git commit -m "feat(edge-server): add GET /api/sync/status endpoint"
```

---

## Task 4: Register Sync Route (edge-server)

**Files:**
- Modify: `edge-server/src/lib.rs` or wherever routes are registered

**Step 1: Find route registration**

Search for where `Router::new()` or routes are defined.

**Step 2: Add sync status route**

```rust
.route("/api/sync/status", get(api::sync::get_sync_status))
```

**Step 3: Verify compilation**

Run: `cargo check -p edge-server`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add edge-server/src/
git commit -m "feat(edge-server): register /api/sync/status route"
```

---

## Task 5: Add Tauri Command for Sync Status (red_coral)

**Files:**
- Create: `red_coral/src-tauri/src/commands/sync.rs`
- Modify: `red_coral/src-tauri/src/commands/mod.rs`
- Modify: `red_coral/src-tauri/src/lib.rs`

**Step 1: Create sync.rs command**

```rust
// red_coral/src-tauri/src/commands/sync.rs
//! Sync Commands

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::response::{ApiResponse, ErrorCode};
use crate::core::ClientBridge;

/// 同步状态响应
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncStatusResponse {
    pub epoch: String,
    pub versions: std::collections::HashMap<String, u64>,
}

/// 获取同步状态
#[tauri::command(rename_all = "snake_case")]
pub async fn get_sync_status(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<SyncStatusResponse>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<SyncStatusResponse>("/api/sync/status").await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NetworkError,
            e.to_string(),
        )),
    }
}
```

**Step 2: Export in commands/mod.rs**

Add to `red_coral/src-tauri/src/commands/mod.rs`:
```rust
pub mod sync;
pub use sync::*;
```

**Step 3: Register command in lib.rs**

Find the `invoke_handler` and add `get_sync_status`.

**Step 4: Verify compilation**

Run: `cd red_coral && cargo check -p red_coral`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/commands/sync.rs
git add red_coral/src-tauri/src/commands/mod.rs
git add red_coral/src-tauri/src/lib.rs
git commit -m "feat(red_coral): add get_sync_status tauri command"
```

---

## Task 6: Refactor createResourceStore - Add Version Support

**Files:**
- Modify: `red_coral/src/core/stores/factory/createResourceStore.ts`

**Step 1: Add SyncPayload type**

```typescript
// Add at top of file
export interface SyncPayload<T = unknown> {
  id: string;
  version: number;
  action: 'created' | 'updated' | 'deleted';
  data: T | null;
}
```

**Step 2: Update ResourceStore interface**

```typescript
export interface ResourceStore<T extends { id: string }> {
  items: T[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;
  lastVersion: number;  // ADD THIS

  fetchAll: (force?: boolean) => Promise<void>;
  applySync: (payload: SyncPayload<T>) => void;  // CHANGE SIGNATURE
  checkVersion: (serverVersion: number) => boolean;  // ADD THIS
  getById: (id: string) => T | undefined;
  clear: () => void;
}
```

**Step 3: Implement new createResourceStore**

Replace the implementation with version-based logic:
- Add `lastVersion: 0` to initial state
- Implement `applySync` with duplicate detection, gap detection, and incremental update
- Add `checkVersion` method
- Update `clear` to reset `lastVersion`

**Step 4: Remove pendingIds from createCrudResourceStore**

Delete:
```typescript
const PENDING_EXPIRY_MS = 1000;
const pendingIds = new Set<string>();
const addPendingId = (id: string) => { ... };
```

Update CRUD operations to NOT call `addPendingId`.

**Step 5: Verify TypeScript**

Run: `cd red_coral && npx tsc --noEmit`
Expected: No type errors

**Step 6: Commit**

```bash
git add red_coral/src/core/stores/factory/createResourceStore.ts
git commit -m "refactor(stores): implement version-based incremental sync"
```

---

## Task 7: Update useSyncListener

**Files:**
- Modify: `red_coral/src/core/hooks/useSyncListener.ts`

**Step 1: Update SyncPayload interface**

```typescript
interface SyncPayload {
  resource: string;
  action: 'created' | 'updated' | 'deleted';
  id: string;
  version: number;  // ADD THIS
  data: unknown | null;
}
```

**Step 2: Pass full payload to applySync**

Change:
```typescript
store.getState().applySync(id);
```
To:
```typescript
store.getState().applySync({ id, version, action, data });
```

**Step 3: Verify TypeScript**

Run: `cd red_coral && npx tsc --noEmit`
Expected: No type errors

**Step 4: Commit**

```bash
git add red_coral/src/core/hooks/useSyncListener.ts
git commit -m "refactor(hooks): pass full sync payload with version"
```

---

## Task 8: Update Registry Interface

**Files:**
- Modify: `red_coral/src/core/stores/resources/registry.ts`

**Step 1: Update RegistryStore interface**

```typescript
interface RegistryStore {
  getState: () => {
    isLoaded: boolean;
    lastVersion: number;  // ADD
    fetchAll: (force?: boolean) => Promise<void>;
    applySync: (payload: { id: string; version: number; action: string; data: unknown }) => void;  // CHANGE
    checkVersion: (serverVersion: number) => boolean;  // ADD
    clear: () => void;
  };
}
```

**Step 2: Verify TypeScript**

Run: `cd red_coral && npx tsc --noEmit`
Expected: No type errors

**Step 3: Commit**

```bash
git add red_coral/src/core/stores/resources/registry.ts
git commit -m "refactor(registry): update interface for version-based sync"
```

---

## Task 9: Create useSyncConnection Hook

**Files:**
- Create: `red_coral/src/core/hooks/useSyncConnection.ts`
- Modify: `red_coral/src/core/hooks/index.ts` (if exists)

**Step 1: Create useSyncConnection.ts**

```typescript
// red_coral/src/core/hooks/useSyncConnection.ts
/**
 * Sync Connection Hook - 断线重连管理
 *
 * 监听连接状态变化，重连时检查 epoch 和 version，
 * 按需刷新过期的 Store。
 */

import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invokeApi } from '@/infrastructure/api';
import { storeRegistry, getLoadedStores, refreshAllLoadedStores } from '@/core/stores/resources';

interface SyncStatus {
  epoch: string;
  versions: Record<string, number>;
}

// 缓存的服务器 epoch
let cachedEpoch: string | null = null;

/**
 * 获取缓存的 epoch (用于测试)
 */
export function getCachedEpoch(): string | null {
  return cachedEpoch;
}

/**
 * 设置缓存的 epoch (用于初始化和测试)
 */
export function setCachedEpoch(epoch: string | null): void {
  cachedEpoch = epoch;
}

export function useSyncConnection() {
  const isReconnecting = useRef(false);

  useEffect(() => {
    const handleConnectionChange = async (connected: boolean) => {
      if (!connected || isReconnecting.current) return;

      isReconnecting.current = true;
      console.log('[SyncConnection] Reconnected, checking sync status...');

      try {
        const status = await invokeApi<SyncStatus>('get_sync_status');

        // Epoch 检查：epoch 变化说明服务器重启，需要全量刷新
        if (cachedEpoch && cachedEpoch !== status.epoch) {
          console.warn('[SyncConnection] Epoch changed, full refresh all stores');
          cachedEpoch = status.epoch;
          await refreshAllLoadedStores();
          return;
        }

        cachedEpoch = status.epoch;

        // Version 比对：只刷新落后的 Store
        const loadedStores = getLoadedStores();
        const staleStores: string[] = [];

        for (const [name, store] of loadedStores) {
          const serverVersion = status.versions[name] || 0;
          if (store.getState().checkVersion(serverVersion)) {
            staleStores.push(name);
          }
        }

        if (staleStores.length > 0) {
          console.log(`[SyncConnection] Refreshing stale stores: ${staleStores.join(', ')}`);
          await Promise.all(
            staleStores.map(name => storeRegistry[name].getState().fetchAll(true))
          );
        } else {
          console.log('[SyncConnection] All stores up to date');
        }
      } catch (err) {
        console.error('[SyncConnection] Sync status check failed, fallback to full refresh:', err);
        await refreshAllLoadedStores();
      } finally {
        isReconnecting.current = false;
      }
    };

    // 监听 Tauri 连接状态事件
    const unlisten = listen<boolean>('connection-state-changed', (event) => {
      handleConnectionChange(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);
}
```

**Step 2: Export in hooks/index.ts (if exists)**

Add:
```typescript
export { useSyncConnection, getCachedEpoch, setCachedEpoch } from './useSyncConnection';
```

**Step 3: Verify TypeScript**

Run: `cd red_coral && npx tsc --noEmit`
Expected: No type errors

**Step 4: Commit**

```bash
git add red_coral/src/core/hooks/useSyncConnection.ts
git add red_coral/src/core/hooks/index.ts 2>/dev/null || true
git commit -m "feat(hooks): add useSyncConnection for reconnection sync"
```

---

## Task 10: Integrate useSyncConnection in App

**Files:**
- Modify: `red_coral/src/App.tsx` (or appropriate root component)

**Step 1: Import and use useSyncConnection**

Add near other sync hooks:
```typescript
import { useSyncConnection } from '@/core/hooks/useSyncConnection';

// Inside App component
useSyncConnection();
```

**Step 2: Verify app runs**

Run: `cd red_coral && npm run dev`
Expected: App compiles and runs without errors

**Step 3: Commit**

```bash
git add red_coral/src/App.tsx
git commit -m "feat(app): integrate useSyncConnection hook"
```

---

## Task 11: Backend Integration Test

**Files:**
- Test manually or create test file

**Step 1: Start edge-server**

Run: `cargo run -p edge-server`

**Step 2: Test sync status endpoint**

Run:
```bash
curl http://localhost:3000/api/sync/status | jq
```

Expected output:
```json
{
  "epoch": "some-uuid",
  "versions": {
    "product": 0,
    "category": 0,
    ...
  }
}
```

**Step 3: Create a product and check version increments**

```bash
# Create product (adjust auth as needed)
curl -X POST http://localhost:3000/api/products ...

# Check versions again
curl http://localhost:3000/api/sync/status | jq '.versions.product'
```

Expected: product version is now 1

**Step 4: Commit any test fixtures if created**

---

## Task 12: Frontend Integration Test

**Step 1: Run the app**

Run: `cd red_coral && npm run tauri:dev`

**Step 2: Test incremental sync**

1. Open DevTools console
2. Create a product via UI
3. Observe console logs - should show `[product] applySync` with incremental update

**Step 3: Test reconnection sync**

1. Stop edge-server
2. Wait a moment
3. Restart edge-server
4. Observe console - should show `[SyncConnection] Epoch changed, full refresh`

**Step 4: Verify no regressions**

- Products/Categories/Tables CRUD all work
- UI updates immediately on changes

---

## Task 13: Final Cleanup and Documentation

**Step 1: Update design doc status**

Change `> 状态: 待实现` to `> 状态: 已实现` in `docs/plans/2026-01-22-resource-sync-improvement-design.md`

**Step 2: Final commit**

```bash
git add docs/plans/2026-01-22-resource-sync-improvement-design.md
git commit -m "docs: mark resource sync improvement as implemented"
```

**Step 3: Run full test suite**

```bash
cargo test --workspace
cd red_coral && npm run test
```

Expected: All tests pass

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add SyncStatus model | shared/src/models/sync.rs |
| 2 | Add epoch to ServerState | edge-server/src/core/state.rs |
| 3 | Create sync API module | edge-server/src/api/sync/ |
| 4 | Register sync route | edge-server routing |
| 5 | Add Tauri command | red_coral/src-tauri/src/commands/sync.rs |
| 6 | Refactor createResourceStore | createResourceStore.ts |
| 7 | Update useSyncListener | useSyncListener.ts |
| 8 | Update registry interface | registry.ts |
| 9 | Create useSyncConnection | useSyncConnection.ts |
| 10 | Integrate in App | App.tsx |
| 11 | Backend integration test | Manual |
| 12 | Frontend integration test | Manual |
| 13 | Cleanup and docs | docs/ |
