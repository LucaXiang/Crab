# Tauri API Migration - 前端 API 全面迁移到 Tauri Commands

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将前端所有 HTTP API 调用迁移到 Tauri Commands，通过 CrabClient 处理 mTLS 认证

**Architecture:**
- 前端不再直接调用 HTTP API（因为 mTLS 自签名证书无法在 WebView 中使用）
- 所有 API 请求通过 `invoke()` → Tauri Command → ClientBridge → CrabClient → EdgeServer
- CrabClient 统一处理 Server 模式（In-Process）和 Client 模式（Remote mTLS）

**Tech Stack:** Tauri 2.x, TypeScript, Rust, CrabClient

---

## Phase 1: 补全缺失的 Tauri Commands

### Task 1: 添加 Roles 相关 Commands

**Files:**
- Modify: `src-tauri/src/commands/system.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 在 system.rs 中添加 Roles commands**

```rust
// 在 system.rs 文件末尾添加

// ============ Roles ============

#[tauri::command]
pub async fn list_roles(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/roles").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/roles/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/roles", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/roles/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/roles/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_role_permissions(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    role_id: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/roles/{}/permissions", role_id)).await.map_err(|e| e.to_string())
}
```

**Step 2: 在 lib.rs 中注册新 commands**

在 `invoke_handler` 中添加:
```rust
commands::list_roles,
commands::get_role,
commands::create_role,
commands::update_role,
commands::delete_role,
commands::get_role_permissions,
```

**Step 3: 编译验证**

Run: `cargo build -p red_coral`
Expected: 编译成功

**Step 4: Commit**

```bash
git add src-tauri/src/commands/system.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add roles API commands"
```

---

### Task 2: 添加 Product Attributes 绑定 Commands

**Files:**
- Modify: `src-tauri/src/commands/data.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 在 data.rs 中添加 Product-Attribute 绑定 commands**

```rust
// ============ Product Attributes (Bindings) ============

#[tauri::command]
pub async fn list_product_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    product_id: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/products/{}/attributes", product_id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/has-attribute", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unbind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/has-attribute/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product_attribute_binding(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/has-attribute/{}", id), &data).await.map_err(|e| e.to_string())
}
```

**Step 2: 在 lib.rs 中注册**

```rust
commands::list_product_attributes,
commands::bind_product_attribute,
commands::unbind_product_attribute,
commands::update_product_attribute_binding,
```

**Step 3: 编译验证**

Run: `cargo build -p red_coral`

**Step 4: Commit**

```bash
git add src-tauri/src/commands/data.rs src-tauri/src/lib.rs
git commit -m "feat(tauri): add product-attribute binding commands"
```

---

### Task 3: 添加通用 HTTP 请求 Command (备用)

**Files:**
- Create: `src-tauri/src/commands/api.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 api.rs**

```rust
//! 通用 API Commands
//!
//! 提供通用的 HTTP 方法，用于前端调用尚未封装的 API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::ClientBridge;

/// 通用 GET 请求
#[tauri::command]
pub async fn api_get(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&path).await.map_err(|e| e.to_string())
}

/// 通用 POST 请求
#[tauri::command]
pub async fn api_post(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.post(&path, &body).await.map_err(|e| e.to_string())
}

/// 通用 PUT 请求
#[tauri::command]
pub async fn api_put(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.put(&path, &body).await.map_err(|e| e.to_string())
}

/// 通用 DELETE 请求
#[tauri::command]
pub async fn api_delete(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.delete(&path).await.map_err(|e| e.to_string())
}
```

**Step 2: 更新 mod.rs**

```rust
pub mod api;
// ... existing modules

pub use api::*;
```

**Step 3: 更新 lib.rs**

```rust
commands::api_get,
commands::api_post,
commands::api_put,
commands::api_delete,
```

**Step 4: 编译验证**

Run: `cargo build -p red_coral`

**Step 5: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat(tauri): add generic API commands for flexibility"
```

---

## Phase 2: 前端 API 适配器

### Task 4: 创建 Tauri API 适配器

**Files:**
- Create: `src/infrastructure/api/tauri-client.ts`

**Step 1: 创建 tauri-client.ts**

```typescript
/**
 * Tauri API Client - 通过 Tauri Commands 调用 API
 *
 * 替代直接 HTTP 调用，所有请求通过:
 * invoke() → Tauri Command → ClientBridge → CrabClient → EdgeServer
 *
 * 这样可以正确处理 mTLS 认证（自签名证书）
 */

import { invoke } from '@tauri-apps/api/core';

// API Error class (与原 client.ts 保持一致)
export class ApiError extends Error {
  code: string;
  httpStatus: number;

  constructor(code: string, message: string, httpStatus: number = 500) {
    super(message);
    this.code = code;
    this.httpStatus = httpStatus;
    this.name = 'ApiError';
  }
}

/**
 * 包装 Tauri invoke 调用，统一错误处理
 */
async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new ApiError('INVOKE_ERROR', message, 500);
  }
}

/**
 * Tauri API Client
 *
 * 与原 ApiClient 接口保持一致，但使用 Tauri commands
 */
export class TauriApiClient {
  // ============ Health ============

  async getHealth() {
    return invokeCommand<{ status: string }>('api_get', { path: '/health' });
  }

  async isAvailable(): Promise<boolean> {
    try {
      await this.getHealth();
      return true;
    } catch {
      return false;
    }
  }

  // ============ Auth ============

  async login(data: { username: string; password: string }) {
    // 使用专用的 login_employee command
    return invokeCommand('login_employee', {
      username: data.username,
      password: data.password
    });
  }

  async logout() {
    return invokeCommand('logout_employee');
  }

  async getCurrentUser() {
    return invokeCommand('get_current_session');
  }

  // ============ Tags ============

  async listTags() {
    return invokeCommand('list_tags');
  }

  async getTag(id: string) {
    return invokeCommand('get_tag', { id });
  }

  async createTag(data: { name: string; color?: string; display_order?: number }) {
    return invokeCommand('create_tag', { data });
  }

  async updateTag(id: string, data: { name?: string; color?: string; display_order?: number; is_active?: boolean }) {
    return invokeCommand('update_tag', { id, data });
  }

  async deleteTag(id: string) {
    return invokeCommand('delete_tag', { id });
  }

  // ============ Categories ============

  async listCategories() {
    return invokeCommand('list_categories');
  }

  async getCategory(id: string) {
    return invokeCommand('get_category', { id });
  }

  async createCategory(data: { name: string; sort_order?: number }) {
    return invokeCommand('create_category', { data });
  }

  async updateCategory(id: string, data: { name?: string; sort_order?: number; is_active?: boolean }) {
    return invokeCommand('update_category', { id, data });
  }

  async deleteCategory(id: string) {
    return invokeCommand('delete_category', { id });
  }

  // ============ Products ============

  async listProducts() {
    return invokeCommand('list_products');
  }

  async getProduct(id: string) {
    return invokeCommand('get_product', { id });
  }

  async createProduct(data: Record<string, unknown>) {
    return invokeCommand('create_product', { data });
  }

  async updateProduct(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_product', { id, data });
  }

  async deleteProduct(id: string) {
    return invokeCommand('delete_product', { id });
  }

  // ============ Product Specifications ============

  async listProductSpecs(productId: string) {
    return invokeCommand('list_specs', { product_id: productId });
  }

  async createProductSpec(data: Record<string, unknown>) {
    return invokeCommand('create_spec', { data });
  }

  async updateProductSpec(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_spec', { id, data });
  }

  async deleteProductSpec(id: string) {
    return invokeCommand('delete_spec', { id });
  }

  // ============ Product Attributes ============

  async fetchProductAttributes(productId: string) {
    return invokeCommand('list_product_attributes', { product_id: productId });
  }

  async bindProductAttribute(data: Record<string, unknown>) {
    return invokeCommand('bind_product_attribute', { data });
  }

  async unbindProductAttribute(id: string) {
    return invokeCommand('unbind_product_attribute', { id });
  }

  // ============ Attributes ============

  async listAttributeTemplates() {
    return invokeCommand('list_attributes');
  }

  async getAttributeTemplate(id: string) {
    return invokeCommand('get_attribute', { id });
  }

  async createAttributeTemplate(data: Record<string, unknown>) {
    return invokeCommand('create_attribute', { data });
  }

  async updateAttributeTemplate(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_attribute', { id, data });
  }

  async deleteAttributeTemplate(id: string) {
    return invokeCommand('delete_attribute', { id });
  }

  // ============ Zones ============

  async listZones() {
    return invokeCommand('list_zones');
  }

  async getZone(id: string) {
    return invokeCommand('get_zone', { id });
  }

  async createZone(data: { name: string; description?: string }) {
    return invokeCommand('create_zone', { data });
  }

  async updateZone(id: string, data: { name?: string; description?: string; is_active?: boolean }) {
    return invokeCommand('update_zone', { id, data });
  }

  async deleteZone(id: string) {
    return invokeCommand('delete_zone', { id });
  }

  // ============ Tables ============

  async listTables() {
    return invokeCommand('list_tables');
  }

  async getTablesByZone(zoneId: string) {
    return invokeCommand('list_tables_by_zone', { zone_id: zoneId });
  }

  async getTable(id: string) {
    return invokeCommand('get_table', { id });
  }

  async createTable(data: { name: string; zone: string; capacity?: number }) {
    return invokeCommand('create_table', { data });
  }

  async updateTable(id: string, data: { name?: string; zone?: string; capacity?: number; is_active?: boolean }) {
    return invokeCommand('update_table', { id, data });
  }

  async deleteTable(id: string) {
    return invokeCommand('delete_table', { id });
  }

  // ============ Kitchen Printers ============

  async listPrinters() {
    return invokeCommand('list_kitchen_printers');
  }

  async getPrinter(id: string) {
    return invokeCommand('get_kitchen_printer', { id });
  }

  async createPrinter(data: { name: string; printer_name?: string; description?: string }) {
    return invokeCommand('create_kitchen_printer', { data });
  }

  async updatePrinter(id: string, data: { name?: string; printer_name?: string; description?: string; is_active?: boolean }) {
    return invokeCommand('update_kitchen_printer', { id, data });
  }

  async deletePrinter(id: string) {
    return invokeCommand('delete_kitchen_printer', { id });
  }

  // ============ Employees ============

  async listEmployees() {
    return invokeCommand('list_employees');
  }

  async getEmployee(id: string) {
    return invokeCommand('get_employee', { id });
  }

  async createEmployee(data: { username: string; password: string; role: string }) {
    return invokeCommand('create_employee', { data });
  }

  async updateEmployee(id: string, data: { username?: string; password?: string; role?: string; is_active?: boolean }) {
    return invokeCommand('update_employee', { id, data });
  }

  async deleteEmployee(id: string) {
    return invokeCommand('delete_employee', { id });
  }

  // ============ Price Rules ============

  async listPriceAdjustments() {
    return invokeCommand('list_price_rules');
  }

  async listActivePriceAdjustments() {
    return invokeCommand('list_active_price_rules');
  }

  async getPriceAdjustment(id: string) {
    return invokeCommand('get_price_rule', { id });
  }

  async createPriceAdjustment(data: Record<string, unknown>) {
    return invokeCommand('create_price_rule', { data });
  }

  async updatePriceAdjustment(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_price_rule', { id, data });
  }

  async deletePriceAdjustment(id: string) {
    return invokeCommand('delete_price_rule', { id });
  }

  // ============ Roles ============

  async listRoles() {
    return invokeCommand('list_roles');
  }

  async getRole(id: string) {
    return invokeCommand('get_role', { id });
  }

  async createRole(data: { name: string }) {
    return invokeCommand('create_role', { data });
  }

  async updateRole(id: string, data: { name?: string }) {
    return invokeCommand('update_role', { id, data });
  }

  async deleteRole(id: string) {
    return invokeCommand('delete_role', { id });
  }

  async getRolePermissions(roleId: string) {
    return invokeCommand('get_role_permissions', { role_id: roleId });
  }

  // ============ Orders ============

  async listOrders() {
    return invokeCommand('list_orders');
  }

  async listOpenOrders() {
    return invokeCommand('list_open_orders');
  }

  async getOrder(id: string) {
    return invokeCommand('get_order', { id });
  }

  async getOrderByReceipt(receiptNumber: string) {
    return invokeCommand('get_order_by_receipt', { receipt_number: receiptNumber });
  }

  async createOrder(data: Record<string, unknown>) {
    return invokeCommand('create_order', { data });
  }

  async addOrderItem(orderId: string, item: Record<string, unknown>) {
    return invokeCommand('add_order_item', { order_id: orderId, item });
  }

  async addOrderPayment(orderId: string, payment: Record<string, unknown>) {
    return invokeCommand('add_order_payment', { order_id: orderId, payment });
  }

  // ============ System ============

  async getSystemState() {
    return invokeCommand('get_system_state');
  }

  // ============ Generic API (fallback) ============

  async apiGet<T>(path: string): Promise<T> {
    return invokeCommand('api_get', { path });
  }

  async apiPost<T>(path: string, body: unknown): Promise<T> {
    return invokeCommand('api_post', { path, body });
  }

  async apiPut<T>(path: string, body: unknown): Promise<T> {
    return invokeCommand('api_put', { path, body });
  }

  async apiDelete<T>(path: string): Promise<T> {
    return invokeCommand('api_delete', { path });
  }
}

// 创建单例
let clientInstance: TauriApiClient | null = null;

export function createTauriClient(): TauriApiClient {
  if (!clientInstance) {
    clientInstance = new TauriApiClient();
  }
  return clientInstance;
}

// 默认导出
export default TauriApiClient;
```

**Step 2: 验证 TypeScript 编译**

Run: `npx tsc --noEmit src/infrastructure/api/tauri-client.ts`

**Step 3: Commit**

```bash
git add src/infrastructure/api/tauri-client.ts
git commit -m "feat(frontend): add Tauri API client adapter"
```

---

### Task 5: 更新 API 入口文件

**Files:**
- Modify: `src/infrastructure/api/index.ts`

**Step 1: 更新 index.ts 导出 Tauri client**

```typescript
// 导出 Tauri API Client (用于 Tauri 环境)
export { TauriApiClient, createTauriClient, ApiError } from './tauri-client';

// 导出原 HTTP Client (用于开发/测试)
export { ApiClient, createClient } from './client';

// 类型导出
export * from './types';

// 环境检测
export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

// 智能工厂函数
export function createApiClient() {
  if (isTauriEnvironment()) {
    return createTauriClient();
  }
  return createClient();
}
```

**Step 2: Commit**

```bash
git add src/infrastructure/api/index.ts
git commit -m "feat(frontend): add smart API client factory"
```

---

## Phase 3: 前端组件迁移

### Task 6: 更新组件使用 Tauri Client

**Files:**
- 需要更新的文件列表（按优先级）:
  1. `src/screens/Login/index.tsx` - 登录
  2. `src/screens/POS/index.tsx` - 主 POS
  3. `src/screens/Settings/*.tsx` - 设置页面

**Step 1: 搜索所有使用 createClient 的地方**

Run: `grep -r "createClient\|ApiClient" src/ --include="*.ts" --include="*.tsx" | head -50`

**Step 2: 批量替换导入语句**

将:
```typescript
import { createClient } from '@/infrastructure/api';
const api = createClient();
```

替换为:
```typescript
import { createApiClient } from '@/infrastructure/api';
const api = createApiClient();
```

**Step 3: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`

**Step 4: Commit**

```bash
git add src/
git commit -m "refactor(frontend): migrate to Tauri API client"
```

---

## Phase 4: 测试验证

### Task 7: 端到端测试

**Step 1: 启动 Server 模式测试**

1. 运行 `npm run tauri:dev`
2. 进入 Setup 页面，激活租户
3. 启动 Server 模式
4. 登录
5. 验证各个功能：
   - 产品列表加载
   - 分类列表加载
   - 创建/编辑/删除操作

**Step 2: 验证 Client 模式 (可选)**

1. 先启动独立的 edge-server
2. 切换到 Client 模式
3. 验证 mTLS 连接正常

**Step 3: 最终提交**

```bash
git add -A
git commit -m "feat: complete Tauri API migration for mTLS support"
```

---

## 文件清单

### 新增文件
- `src-tauri/src/commands/api.rs` - 通用 API commands
- `src/infrastructure/api/tauri-client.ts` - Tauri API 适配器

### 修改文件
- `src-tauri/src/commands/mod.rs` - 导出新 commands
- `src-tauri/src/commands/system.rs` - 添加 Roles commands
- `src-tauri/src/commands/data.rs` - 添加 Product-Attribute 绑定
- `src-tauri/src/lib.rs` - 注册新 commands
- `src/infrastructure/api/index.ts` - 更新导出
- `src/screens/**/*.tsx` - 更新 API 调用

---

## 注意事项

1. **ID 类型**: Edge Server 使用 SurrealDB，ID 格式为 `table:id` (string)，前端需要适配

2. **响应格式**: Tauri commands 直接返回数据，不再有 `{ data: T }` 包装

3. **错误处理**: Tauri invoke 的错误是字符串，需要统一转换为 ApiError

4. **类型安全**: 建议逐步为每个 command 添加 TypeScript 返回类型定义
