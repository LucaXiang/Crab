# Store Management Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现门店管理功能 — Console 可查看设备/删除门店，Setup 始终可选新建或替换门店，移除 Client 配额限制。

**Architecture:** Cloud DB 加 `stores.status/deleted_at`，Cloud API 新增 DELETE 和设备列表端点，改造 activate API 支持 `store_id` 参数替代 `replace_entity_id`，Console 门店设置页新增设备区域和删除功能，RedCoral Setup 新增"选择门店"步骤。

**Tech Stack:** Rust (axum, sqlx, PostgreSQL), TypeScript (React 19, Zustand, Tailwind CSS, Tauri 2)

---

## Task 1: Cloud DB Migration — stores 表加 status/deleted_at

**Files:**
- Create: `crab-cloud/migrations/0002_store_management.up.sql`
- Create: `crab-cloud/migrations/0002_store_management.down.sql`

**Step 1: 创建迁移文件**

`0002_store_management.up.sql`:
```sql
-- Add status and deleted_at to stores
ALTER TABLE stores ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE stores ADD COLUMN deleted_at BIGINT;

-- Rename max_edge_servers to max_stores in subscriptions
ALTER TABLE subscriptions RENAME COLUMN max_edge_servers TO max_stores;

-- Drop max_clients from subscriptions
ALTER TABLE subscriptions DROP COLUMN max_clients;
```

`0002_store_management.down.sql`:
```sql
ALTER TABLE subscriptions ADD COLUMN max_clients INT NOT NULL DEFAULT 5;
ALTER TABLE subscriptions RENAME COLUMN max_stores TO max_edge_servers;
ALTER TABLE stores DROP COLUMN deleted_at;
ALTER TABLE stores DROP COLUMN status;
```

**Step 2: 更新 0001_initial.up.sql 保持一致**

在 `stores` 表定义中加入 `status` 和 `deleted_at` 列。在 `subscriptions` 表中把 `max_edge_servers` 改为 `max_stores`，删除 `max_clients`。这样新建数据库时 schema 是最终状态。

**Step 3: 验证**

Run: `cargo check -p crab-cloud` — 预期会因字段名变化而编译失败，这是正确的，后续 Task 会修复。

**Step 4: Commit**

```bash
git add crab-cloud/migrations/
git commit -m "feat(cloud): add store status/deleted_at, rename max_stores, drop max_clients"
```

---

## Task 2: Cloud Rust — 更新 Subscription 模型 (max_stores, 移除 max_clients)

**Files:**
- Modify: `crab-cloud/src/db/subscriptions.rs` — `Subscription` struct + `CreateSubscription` + SQL
- Modify: `crab-cloud/src/db/tenant_queries.rs` — `SubscriptionSummary` struct + SQL
- Modify: `crab-cloud/src/stripe/mod.rs` — `PlanQuota` struct
- Modify: `crab-cloud/src/api/stripe_webhook.rs` — webhook handler
- Modify: `crab-cloud/src/auth/quota.rs` — quota check function

**Step 1: 更新 `crab-cloud/src/db/subscriptions.rs`**

- `Subscription` struct: `max_edge_servers: i32` → `max_stores: i32`，删除 `max_clients: i32`
- `CreateSubscription`: 同上
- 所有 SQL 查询中 `max_edge_servers` → `max_stores`，删除 `max_clients` 相关列
- `create()` 的 INSERT: 删除 `max_clients` 列和 bind
- `get_latest_subscription()` 的 SELECT: 删除 `max_clients`

**Step 2: 更新 `crab-cloud/src/db/tenant_queries.rs`**

- `SubscriptionSummary` struct: `max_edge_servers` → `max_stores`，删除 `max_clients`
- SQL 查询同步

**Step 3: 更新 `crab-cloud/src/stripe/mod.rs`**

- `PlanQuota` struct: `max_edge_servers` → `max_stores`，删除 `max_clients`
- 各 plan 定义删除 `max_clients` 字段

**Step 4: 更新 `crab-cloud/src/api/stripe_webhook.rs`**

- `CreateSubscription` 构造: `max_edge_servers` → `max_stores`，删除 `max_clients`

**Step 5: 更新 `crab-cloud/src/auth/quota.rs`**

- SQL: `max_edge_servers` → `max_stores`
- 变量名: `max_edge_servers` → `max_stores`

**Step 6: 验证**

Run: `cargo check -p crab-cloud` — 预期仍有 activate.rs 等文件的编译错误，后续修复。

**Step 7: Commit**

```bash
git add crab-cloud/src/db/subscriptions.rs crab-cloud/src/db/tenant_queries.rs crab-cloud/src/stripe/mod.rs crab-cloud/src/api/stripe_webhook.rs crab-cloud/src/auth/quota.rs
git commit -m "refactor(cloud): rename max_stores, remove max_clients from subscription model"
```

---

## Task 3: Cloud Rust — 更新 Store DB 层 + 新增 delete/devices 查询

**Files:**
- Modify: `crab-cloud/src/db/tenant_queries.rs` — `StoreSummary` 加 status，`list_stores` 过滤 active
- Modify: `crab-cloud/src/db/sync_store.rs` — `ensure_store` 排除 deleted 门店的 store_number
- Create: `crab-cloud/src/db/store/devices.rs` — 查询门店关联设备
- Modify: `crab-cloud/src/db/store/mod.rs` — pub mod devices

**Step 1: 更新 `tenant_queries.rs`**

`StoreSummary` 新增 `status: String`。

`list_stores` SQL 加 `WHERE status = 'active'` 过滤（已有 `WHERE tenant_id = $1`，加 `AND status = 'active'`）。

新增函数:
```rust
/// 软删除门店 + 停用关联的所有设备
pub async fn soft_delete_store(pool: &PgPool, store_id: i64, tenant_id: &str, now: i64) -> Result<(), BoxError> {
    let mut tx = pool.begin().await?;

    // 获取 store 的 entity_id
    let store: (String,) = sqlx::query_as(
        "SELECT entity_id FROM stores WHERE id = $1 AND tenant_id = $2 AND status = 'active'"
    )
    .bind(store_id)
    .bind(tenant_id)
    .fetch_one(&mut *tx)
    .await?;

    // 软删除门店
    sqlx::query("UPDATE stores SET status = 'deleted', deleted_at = $1 WHERE id = $2")
        .bind(now)
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    // 停用关联的 server activation
    sqlx::query(
        "UPDATE activations SET status = 'deactivated', deactivated_at = $1 WHERE entity_id = $2 AND status = 'active'"
    )
    .bind(now)
    .bind(&store.0)
    .execute(&mut *tx)
    .await?;

    // 停用关联的所有 client connections (通过 tenant_id 关联 — client 没有直接的 store 关联，
    // 但 client 连接的是这个 entity_id 对应的 edge-server，暂时只停用 server activation)
    // 注意: client_connections 目前没有 store_id 字段，无法按门店停用 client。
    // 这里只停用 server activation。

    tx.commit().await?;
    Ok(())
}
```

**Step 2: 更新 `sync_store.rs`**

`ensure_store` 中 `store_number` 子查询加 `AND status = 'active'`，避免已删除门店影响编号。

同时需要处理 `store_id` 替换场景：新增函数:
```rust
/// 将已有门店绑定到新的 entity_id/device_id (设备替换)
pub async fn rebind_store(pool: &PgPool, store_id: i64, entity_id: &str, device_id: &str) -> Result<(), BoxError> {
    sqlx::query("UPDATE stores SET entity_id = $1, device_id = $2 WHERE id = $3 AND status = 'active'")
        .bind(entity_id)
        .bind(device_id)
        .bind(store_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 3: 创建 `crab-cloud/src/db/store/devices.rs`**

```rust
use sqlx::PgPool;
use serde::Serialize;

#[derive(sqlx::FromRow, Serialize)]
pub struct DeviceRecord {
    pub entity_id: String,
    pub device_id: String,
    pub device_type: String,   // "server" or "client"
    pub status: String,
    pub activated_at: i64,
    pub deactivated_at: Option<i64>,
    pub replaced_by: Option<String>,
    pub last_refreshed_at: Option<i64>,
}

/// 获取门店关联的所有设备 (server activations + client connections)
pub async fn list_devices_for_store(pool: &PgPool, entity_id: &str, tenant_id: &str) -> Result<Vec<DeviceRecord>, sqlx::Error> {
    sqlx::query_as::<_, DeviceRecord>(
        r#"
        SELECT entity_id, device_id, 'server' AS device_type, status, activated_at, deactivated_at, replaced_by, last_refreshed_at
        FROM activations WHERE entity_id = $1 AND tenant_id = $2
        UNION ALL
        SELECT entity_id, device_id, 'server' AS device_type, status, activated_at, deactivated_at, replaced_by, last_refreshed_at
        FROM activations WHERE tenant_id = $2 AND replaced_by = $1
        UNION ALL
        SELECT entity_id, device_id, 'client' AS device_type, status, activated_at, deactivated_at, replaced_by, last_refreshed_at
        FROM client_connections WHERE tenant_id = $2
        ORDER BY activated_at DESC
        "#,
    )
    .bind(entity_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}
```

注意: Client connections 目前没有 `store_id`/`entity_id` 关联到特定门店。暂时返回该租户下所有 client，后续可以加关联。

**Step 4: 在 `crab-cloud/src/db/store/mod.rs` 中 pub mod devices**

**Step 5: 验证**

Run: `cargo check -p crab-cloud`

**Step 6: Commit**

```bash
git add crab-cloud/src/db/
git commit -m "feat(cloud): add store soft-delete, rebind, and device listing DB functions"
```

---

## Task 4: Cloud Rust — 更新 Server Activate API (store_id 替代 replace_entity_id)

**Files:**
- Modify: `crab-cloud/src/api/pki/activate.rs`

**Step 1: 更新 `ActivateRequest`**

```rust
pub struct ActivateRequest {
    pub token: String,
    pub device_id: String,
    pub store_id: Option<i64>,              // 新增: 指定门店 = 替换设备
    pub replace_entity_id: Option<String>,  // 保留向后兼容，优先用 store_id
}
```

**Step 2: 更新配额检查逻辑**

当 `store_id` 存在时:
1. 查询 `stores WHERE id = store_id AND tenant_id = tenant.id AND status = 'active'` → 获取旧 `entity_id`
2. 如果找到，使用旧 `entity_id` 作为 `replace_entity_id` 的值
3. 跳过配额检查（替换不增加门店数量）
4. 在 `ensure_store` 后改为调用 `rebind_store(pool, store_id, entity_id, device_id)`

当 `store_id` 不存在时:
- 走现有逻辑（新建门店）

配额检查改为计算 `stores WHERE tenant_id = $1 AND status = 'active'` 的数量 vs `max_stores`（而不是 activations count）。

**Step 3: 更新 `max_edge_servers` → `max_stores` 变量名**

整个文件中 `max_edge_servers` → `max_stores`。

**Step 4: 验证**

Run: `cargo check -p crab-cloud`

**Step 5: Commit**

```bash
git add crab-cloud/src/api/pki/activate.rs
git commit -m "feat(cloud): support store_id in server activation for device replacement"
```

---

## Task 5: Cloud Rust — 更新 Client Activate API (移除配额检查)

**Files:**
- Modify: `crab-cloud/src/api/pki/activate_client.rs`

**Step 1: 移除配额检查逻辑**

删除:
- `let max_clients = sub.max_clients;`
- 整个 `if !is_reactivation && max_clients > 0 { ... }` 块
- `replace_entity_id` 相关逻辑（不再需要）

保留:
- reactivation 检查（同设备重新激活仍需要）
- advisory lock（防并发）
- `insert_in_tx`

**Step 2: 更新 `ActivateClientRequest`**

移除 `replace_entity_id` 字段。

**Step 3: 更新 subscription info 构造**

`max_clients` 相关字段删除或设为 0。

**Step 4: 验证**

Run: `cargo check -p crab-cloud`

**Step 5: Commit**

```bash
git add crab-cloud/src/api/pki/activate_client.rs
git commit -m "feat(cloud): remove client quota check from activation"
```

---

## Task 6: Cloud Rust — 新增 API 端点 (DELETE store, GET devices)

**Files:**
- Modify: `crab-cloud/src/api/tenant/store.rs` — 新增 `delete_store` 和 `list_devices` handler
- Modify: `crab-cloud/src/api/mod.rs` — 注册路由
- Modify: `crab-cloud/src/api/tenant/mod.rs` — 导出新 handler

**Step 1: 在 `store.rs` 中新增 `delete_store`**

```rust
/// DELETE /api/tenant/stores/:id
pub async fn delete_store(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<()> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let now = shared::util::now_millis();
    tenant_queries::soft_delete_store(&state.pool, store_id, &identity.tenant_id, now)
        .await
        .map_err(|e| {
            tracing::error!("Delete store error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(()))
}
```

**Step 2: 在 `store.rs` 中新增 `list_devices`**

```rust
/// GET /api/tenant/stores/:id/devices
pub async fn list_devices(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<store::devices::DeviceRecord>> {
    let store = verify_store(&state, store_id, &identity.tenant_id).await?;

    // 需要获取 entity_id — 从 verify_store 或单独查询
    let stores = tenant_queries::list_stores(&state.pool, &identity.tenant_id).await
        .map_err(|e| {
            tracing::error!("List stores error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;
    let entity_id = stores.iter().find(|s| s.id == store_id)
        .map(|s| s.entity_id.clone())
        .ok_or_else(|| AppError::not_found("Store not found"))?;

    let devices = store::devices::list_devices_for_store(&state.pool, &entity_id, &identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("List devices error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(devices))
}
```

注意: 上面 `list_stores` 过滤了 `status = 'active'`，删除门店后查不到。可能需要新增一个不过滤 status 的查询，或者改用 `verify_store_ownership` 直接查 entity_id。更优的方案是修改 `verify_store_ownership` 返回 entity_id:

```rust
pub async fn get_store_entity_id(pool: &PgPool, store_id: i64, tenant_id: &str) -> Result<Option<String>, BoxError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT entity_id FROM stores WHERE id = $1 AND tenant_id = $2")
        .bind(store_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}
```

**Step 3: 注册路由**

在 `crab-cloud/src/api/mod.rs` 中:
```rust
.route("/api/tenant/stores/{id}", patch(tenant::update_store).delete(tenant::delete_store))
.route("/api/tenant/stores/{id}/devices", get(tenant::list_devices))
```

**Step 4: 导出 handler**

在 `crab-cloud/src/api/tenant/mod.rs` 中 pub use 新增的 handler。

**Step 5: 验证**

Run: `cargo check -p crab-cloud`

**Step 6: Commit**

```bash
git add crab-cloud/src/api/
git commit -m "feat(cloud): add DELETE store and GET devices API endpoints"
```

---

## Task 7: Cloud Rust — 更新 Verify 端点 + 新增门店列表返回

**Files:**
- Modify: `crab-cloud/src/api/pki/verify.rs` — 移除 `client_slots_remaining`，新增 `stores` 列表
- Modify: `shared/src/activation.rs` — `TenantVerifyData` 类型变更

**Step 1: 更新 `shared/src/activation.rs`**

`TenantVerifyData` 修改:
```rust
pub struct TenantVerifyData {
    pub tenant_id: String,
    pub token: String,
    pub refresh_token: String,
    pub subscription_status: SubscriptionStatus,
    pub plan: PlanType,
    pub server_slots_remaining: i32,
    // 移除: pub client_slots_remaining: i32,
    pub stores: Vec<StoreSlot>,     // 新增: 已有门店列表
    pub has_active_server: bool,
    pub has_active_client: bool,
    pub has_p12: bool,
}

/// Setup 流程中展示的门店选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSlot {
    pub id: i64,
    pub alias: String,
    pub store_number: u32,
    pub is_online: bool,
}
```

**Step 2: 更新 `verify.rs`**

- 移除 `client_slots_remaining` 计算
- 新增查询 `stores WHERE tenant_id = $1 AND status = 'active'` → 构造 `Vec<StoreSlot>`
- `server_slots_remaining` 改为基于 `stores` 数量 vs `max_stores`（而不是 `activations` count）

**Step 3: 验证**

Run: `cargo check -p crab-cloud`

**Step 4: Commit**

```bash
git add shared/src/activation.rs crab-cloud/src/api/pki/verify.rs
git commit -m "feat(cloud): return store list in verify, remove client_slots_remaining"
```

---

## Task 8: Cloud Rust — 更新 Subscription 签名 + Deactivate + 剩余引用

**Files:**
- Modify: `shared/src/activation.rs` — `SubscriptionInfo` 的 `max_clients` 处理
- Modify: `shared/src/app_state.rs` — `max_stores` 字段确认
- Modify: `crab-cloud/src/api/pki/deactivate.rs` — 确认不需改动
- Modify: `crab-cloud/src/api/pki/deactivate_client.rs` — 确认不需改动
- Modify: `crab-cloud/src/api/pki/subscription.rs` — `max_clients` 处理

**Step 1: 检查 `SubscriptionInfo`**

`shared/src/activation.rs` 中 `SubscriptionInfo` 有 `max_stores` 和 `max_clients` 字段。保留 `max_clients` 字段设为 0（签名内容格式已固定，改动影响所有已签发的 binding）。或者直接从签名内容中删除 `max_clients`。

需要决策：如果已有签发的 binding 包含 `max_clients`，直接删除会破坏验签。**建议保留字段但值设为 0**，签名格式不变。

**Step 2: 更新各处 `max_clients` 引用**

- `activate.rs`: `max_clients: sub.max_clients as u32` → `max_clients: 0`（subscription 已无此字段）
- `activate_client.rs`: 同上
- `subscription.rs`: 同上

**Step 3: 全量编译验证**

Run: `cargo check --workspace`

**Step 4: Commit**

```bash
git add shared/src/ crab-cloud/src/
git commit -m "refactor(cloud): update subscription info max_clients to 0, full compile pass"
```

---

## Task 9: Shared 类型 — 更新 ErrorCode (DeviceLimitReached → StoreLimitReached)

**Files:**
- Modify: `shared/src/error/codes.rs` — rename `DeviceLimitReached` → `StoreLimitReached`
- Modify: `shared/src/error/http.rs` — 同步

**Step 1: 重命名 ErrorCode**

- `DeviceLimitReached = 3007` → `StoreLimitReached = 3007`（保留相同的数值）
- 保留 `ClientLimitReached` 但标记为未使用（或直接删除，因为不再需要）
- 更新 `message()`、`TryFrom<u16>`、variant count guard

**Step 2: 全局搜索替换**

搜索所有 `DeviceLimitReached` → `StoreLimitReached` 引用:
- `crab-cloud/src/api/pki/activate.rs`
- `red_coral/src-tauri/src/commands/tenant.rs`
- 前端 i18n 文件

同样搜索 `ClientLimitReached` 引用并删除。

**Step 3: 验证**

Run: `cargo check --workspace`

**Step 4: Commit**

```bash
git add shared/src/error/ crab-cloud/src/ red_coral/src-tauri/
git commit -m "refactor: rename DeviceLimitReached to StoreLimitReached, remove ClientLimitReached"
```

---

## Task 10: Console — 门店设置页新增设备区域和删除功能

**Files:**
- Modify: `crab-console/src/infrastructure/api/stores.ts` — 新增 `deleteStore`, `getStoreDevices`
- Modify: `crab-console/src/core/types/store.ts` — 新增 `DeviceRecord` 类型
- Modify: `crab-console/src/screens/Store/Settings/StoreSettingsScreen.tsx` — 新增设备列表和删除按钮

**Step 1: 新增 API 函数**

在 `stores.ts`:
```typescript
export async function deleteStore(token: string, storeId: number): Promise<void> {
  await fetchApi(`/api/tenant/stores/${storeId}`, { method: 'DELETE', token });
}

export async function getStoreDevices(token: string, storeId: number): Promise<DeviceRecord[]> {
  return fetchApi(`/api/tenant/stores/${storeId}/devices`, { token });
}
```

**Step 2: 新增类型**

在 `store.ts`:
```typescript
export interface DeviceRecord {
  entity_id: string;
  device_id: string;
  device_type: 'server' | 'client';
  status: string;
  activated_at: number;
  deactivated_at: number | null;
  replaced_by: string | null;
  last_refreshed_at: number | null;
}
```

**Step 3: 更新 StoreSettingsScreen**

新增「设备」section（在 device info strip 下方）:
- 调用 `getStoreDevices` 加载设备列表
- 显示每个设备: device_type badge、device_id (truncated)、status badge（active=绿、replaced=灰、deactivated=红）、activated_at
- 只读，用于审计

新增「危险区域」section（页面底部）:
- 红色边框卡片
- "删除门店" 按钮
- 点击弹出确认对话框: "确定要删除门店 {alias} 吗？门店数据将在 30 天后彻底删除。"
- 确认后调用 `deleteStore`，成功后导航回门店列表

**Step 4: 验证**

Run: `cd crab-console && npx tsc --noEmit`

**Step 5: Commit**

```bash
git add crab-console/src/
git commit -m "feat(console): add device list and delete store to store settings"
```

---

## Task 11: RedCoral — 更新 TypeScript 类型

**Files:**
- Modify: `red_coral/src/core/stores/bridge/useBridgeStore.ts` — 更新 `TenantVerifyData`，新增 `StoreSlot`

**Step 1: 更新类型**

```typescript
export interface StoreSlot {
  id: number;
  alias: string;
  store_number: number;
  is_online: boolean;
}

export interface TenantVerifyData {
  tenant_id: string;
  token: string;
  refresh_token: string;
  subscription_status: SubscriptionStatus;
  plan: PlanType;
  server_slots_remaining: number;
  // 移除: client_slots_remaining: number;
  stores: StoreSlot[];     // 新增
  has_active_server: boolean;
  has_active_client: boolean;
  has_p12: boolean;
}
```

**Step 2: 更新 `activateServerTenant` 调用签名**

bridge store 中 `activateServerTenant` 需要支持 `store_id` 参数（替代 `replaceEntityId`）:

```typescript
activateServerTenant: (storeId?: number) => Promise<ActivationResult>;
```

内部 invoke 改为传 `store_id` 而不是 `replace_entity_id`。

**Step 3: 更新 `activateClientTenant`**

移除 `replaceEntityId` 参数（不再需要配额替换）:

```typescript
activateClientTenant: () => Promise<ActivationResult>;
```

**Step 4: 验证**

Run: `cd red_coral && npx tsc --noEmit` — 预期 Setup 页面会报错（还没改）

**Step 5: Commit**

```bash
git add red_coral/src/core/
git commit -m "feat(redcoral): update TenantVerifyData with stores list, update activation signatures"
```

---

## Task 12: RedCoral Tauri — 更新 Rust 命令

**Files:**
- Modify: `red_coral/src-tauri/src/commands/tenant.rs` — `activate_server_tenant` 改用 `store_id`，`activate_client_tenant` 移除 `replace_entity_id`
- Modify: `red_coral/src-tauri/src/core/bridge/activation.rs` — 对应更新

**Step 1: 更新 `activate_server_tenant` 命令**

```rust
#[tauri::command]
pub async fn activate_server_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    store_id: Option<i64>,       // 替代 replace_entity_id
) -> Result<ApiResponse<ActivationResultData>, String>
```

**Step 2: 更新 `activate_client_tenant` 命令**

移除 `replace_entity_id` 参数。

**Step 3: 更新 bridge activation 实现**

`handle_activation_with_replace` 改为接受 `store_id: Option<i64>`，传给 cloud API。

**Step 4: 验证**

Run: `cargo check -p red_coral` (或 `cargo check --workspace`)

**Step 5: Commit**

```bash
git add red_coral/src-tauri/
git commit -m "feat(redcoral): update activation commands to use store_id"
```

---

## Task 13: RedCoral — 改造 Setup 流程 (选择门店步骤)

**Files:**
- Modify: `red_coral/src/screens/Setup/index.tsx`

**Step 1: 新增 SetupStep `'select_store'`**

```typescript
type SetupStep = 'credentials' | 'subscription_blocked' | 'p12_blocked' | 'mode' | 'select_store' | 'configure' | 'complete';
```

**Step 2: Mode 选择后进入 select_store**

Server 模式: mode → `select_store` → configure
Client 模式: mode → configure（跳过 select_store，无配额限制）

**Step 3: 实现 select_store 步骤 UI**

```
┌─────────────────────────────┐
│  选择门店                    │
│                             │
│  ○ 新建门店                  │
│  ○ Store01 (在线 🟢)         │
│  ○ Store02 (离线)            │
│                             │
│  [继续]                      │
└─────────────────────────────┘
```

- 数据来源: `tenantInfo.stores` (从 verify 返回)
- 新增状态: `selectedStoreId: number | null` (null = 新建)
- 选择后点「继续」→ `setStep('configure')`

**Step 4: 更新 handleConfigure**

Server 模式激活时:
```typescript
const result = await activateServerTenant(selectedStoreId ?? undefined);
```

**Step 5: 移除旧的 quota-full 替换 UI**

删除:
- `quotaInfo` 状态
- `DeviceLimitReached` / `ClientLimitReached` 错误处理中的替换 UI
- `handleReplace` 函数

`StoreLimitReached` 错误仍需处理，但改为简单的错误提示 "门店数量已达上限，请在管理后台删除不需要的门店"。

**Step 6: 移除 mode 页面的 `client_slots_remaining` 显示**

Client 模式不再显示剩余配额。

**Step 7: 验证**

Run: `cd red_coral && npx tsc --noEmit`

**Step 8: Commit**

```bash
git add red_coral/src/screens/Setup/
git commit -m "feat(redcoral): add store selection step to Setup, remove quota replacement UI"
```

---

## Task 14: 全栈验证 + SQLx Prepare

**Files:**
- Modify: `.sqlx/` 或 `sqlx-data.json` (auto-generated)

**Step 1: 全量编译**

Run: `cargo check --workspace`
Run: `cargo clippy --workspace`

**Step 2: 前端类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Run: `cd crab-console && npx tsc --noEmit`

**Step 3: SQLx prepare**

Run: `cargo sqlx prepare --workspace`

**Step 4: 运行测试**

Run: `cargo test --workspace --lib`

**Step 5: Commit**

```bash
git add .
git commit -m "chore: full-stack compile verification and sqlx prepare"
```

---

## Task 15: i18n — 更新错误码翻译

**Files:**
- Modify: `red_coral/src/i18n/locales/` — 更新 `StoreLimitReached` 翻译，移除 `ClientLimitReached`
- Modify: `crab-console/src/i18n/` (if exists) — 同步

**Step 1: 更新翻译**

中文: `"门店数量已达上限"` (替代 "设备数量已达上限")
西语: `"Límite de tiendas alcanzado"`
英语: `"Store limit reached"`

移除 `ClientLimitReached` 相关翻译。

**Step 2: Commit**

```bash
git add red_coral/src/i18n/ crab-console/src/i18n/
git commit -m "feat(i18n): update error translations for store management"
```
