# 权限加固实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 统一前后端权限命名，并为所有 API 路由添加 `require_permission` 中间件保护

**架构:** 以后端 `resource:action` 格式为权威命名规范，前端 Permission 常量全部对齐到该格式。后端扩展权限列表覆盖 POS 业务操作（void、discount 等），每个 API 路由模块添加适当的权限中间件层。

**Tech Stack:** Rust (Axum middleware), TypeScript (React)

---

## 设计决策

### 权限命名规范

统一采用 `resource:action` 格式，所有权限字符串前后端一致：

| 前端旧值 | 统一新值 | 说明 |
|----------|---------|------|
| `manage_users` | `users:manage` | 用户管理 |
| `void_order` | `orders:void` | 作废订单 |
| `manage_products` | `products:manage` | 产品管理(通用) |
| `create_product` | `products:write` | 创建产品 |
| `update_product` | `products:write` | 更新产品 |
| `delete_product` | `products:delete` | 删除产品 |
| `manage_categories` | `categories:manage` | 分类管理 |
| `manage_zones` | `zones:manage` | 区域管理 |
| `manage_tables` | `tables:manage` | 桌台管理 |
| `modify_price` | `pricing:write` | 修改价格 |
| `apply_discount` | `orders:discount` | 应用折扣 |
| `view_statistics` | `statistics:read` | 查看统计 |
| `manage_printers` | `printers:manage` | 打印机管理 |
| `manage_attributes` | `attributes:manage` | 属性管理 |
| `manage_settings` | `settings:manage` | 设置管理 |
| `system_settings` | `system:write` | 系统设置 |
| `print_receipts` | `receipts:print` | 打印小票 |
| `reprint_receipt` | `receipts:reprint` | 重打小票 |
| `refund` | `orders:refund` | 退款 |
| `discount` | `orders:discount` | 折扣(同 apply_discount) |
| `cancel_item` | `orders:cancel_item` | 取消单品 |
| `open_cash_drawer` | `pos:cash_drawer` | 开钱箱 |
| `merge_bill` | `tables:merge_bill` | 并单 |
| `transfer_table` | `tables:transfer` | 转台 |

### 路由权限映射

| API 模块 | 读取权限 | 写入权限 | 删除权限 | 特殊权限 |
|----------|---------|---------|---------|---------|
| auth | 公开(login), 已认证(me/logout) | - | - | - |
| health | 公开 | - | - | - |
| role | `roles:read` | `roles:write` | `roles:write` | - |
| upload | - | `products:write` | - | - |
| tags | `products:read` | `products:write` | `products:delete` | - |
| categories | `categories:read` | `categories:manage` | `categories:manage` | - |
| products | `products:read` | `products:write` | `products:delete` | - |
| attributes | `attributes:read` | `attributes:manage` | `attributes:manage` | - |
| has_attribute | `attributes:read` | `attributes:manage` | `attributes:manage` | - |
| zones | `zones:read` | `zones:manage` | `zones:manage` | - |
| tables | `tables:read` | `tables:manage` | `tables:manage` | - |
| price_rules | `pricing:read` | `pricing:write` | `pricing:write` | - |
| print_destinations | `printers:read` | `printers:manage` | `printers:manage` | - |
| print_config | `printers:read` | `printers:manage` | - | - |
| employees | `users:read` | `users:manage` | `users:manage` | - |
| orders | `orders:read` | - | - | - |
| system_state | `system:read` | `system:write` | - | genesis = `system:write` |
| store_info | `system:read` | `system:write` | - | - |
| label_template | `system:read` | `system:write` | `system:write` | - |
| shifts | `system:read` | `system:write` | - | force-close = `system:write` |
| daily_reports | `statistics:read` | `statistics:read` | `system:write` | generate = `statistics:read` |
| statistics | `statistics:read` | - | - | - |
| sync | `system:read` | - | - | - |

---

## Task 1: 扩展后端权限列表

**Files:**
- Modify: `edge-server/src/auth/permissions.rs`

**Step 1: 更新 ALL_PERMISSIONS 常量**

将 `ALL_PERMISSIONS` 扩展为覆盖所有 POS 业务操作的完整权限列表：

```rust
pub const ALL_PERMISSIONS: &[&str] = &[
    // Product permissions
    "products:read",
    "products:write",
    "products:delete",
    "products:manage",
    // Category permissions
    "categories:read",
    "categories:manage",
    // Attribute permissions
    "attributes:read",
    "attributes:manage",
    // Order permissions
    "orders:read",
    "orders:write",
    "orders:void",
    "orders:discount",
    "orders:refund",
    "orders:cancel_item",
    // User management permissions
    "users:read",
    "users:manage",
    // Role management permissions
    "roles:read",
    "roles:write",
    // Zone & Table permissions
    "zones:read",
    "zones:manage",
    "tables:read",
    "tables:manage",
    "tables:merge_bill",
    "tables:transfer",
    // Pricing permissions
    "pricing:read",
    "pricing:write",
    // Statistics permissions
    "statistics:read",
    // Printer permissions
    "printers:read",
    "printers:manage",
    // Receipt permissions
    "receipts:print",
    "receipts:reprint",
    // Settings & System permissions
    "settings:manage",
    "system:read",
    "system:write",
    // POS operations
    "pos:cash_drawer",
    // Admin permission (grants all access)
    "all",
];
```

**Step 2: 更新默认角色权限**

```rust
pub const DEFAULT_ADMIN_PERMISSIONS: &[&str] = &["all"];

pub const DEFAULT_USER_PERMISSIONS: &[&str] = &[
    "products:read",
    "categories:read",
    "attributes:read",
    "orders:read",
    "orders:write",
    "zones:read",
    "tables:read",
    "pricing:read",
    "statistics:read",
    "printers:read",
    "receipts:print",
];

pub const DEFAULT_MANAGER_PERMISSIONS: &[&str] = &[
    "products:read",
    "products:write",
    "products:manage",
    "categories:read",
    "categories:manage",
    "attributes:read",
    "attributes:manage",
    "orders:read",
    "orders:write",
    "orders:void",
    "orders:discount",
    "orders:refund",
    "orders:cancel_item",
    "users:read",
    "zones:read",
    "zones:manage",
    "tables:read",
    "tables:manage",
    "tables:merge_bill",
    "tables:transfer",
    "pricing:read",
    "pricing:write",
    "statistics:read",
    "printers:read",
    "printers:manage",
    "receipts:print",
    "receipts:reprint",
    "settings:manage",
    "pos:cash_drawer",
];
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 4: 提交**

```bash
git add edge-server/src/auth/permissions.rs
git commit -m "feat(auth): expand permission list to cover all POS operations"
```

---

## Task 2: 修复 require_permission 中间件签名

**Files:**
- Modify: `edge-server/src/auth/middleware.rs`

当前 `require_permission` 是 `async fn` 返回闭包，Axum 的 `from_fn` 不支持这种签名。需要改为返回可直接用于 `.layer()` 的中间件。

**Step 1: 重构 require_permission 为工厂函数**

```rust
use axum::middleware::from_fn;
use tower_layer::Layer;

/// 创建权限检查中间件层
///
/// 用法: `.layer(permission_layer("products:write"))`
pub fn permission_layer(
    permission: &'static str,
) -> axum::middleware::FromFnLayer<
    impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> + Clone,
    (),
    Request,
> {
    from_fn(move |req: Request, next: Next| {
        Box::pin(async move {
            let user = req
                .extensions()
                .get::<CurrentUser>()
                .ok_or(AppError::unauthorized())?;

            if !user.has_permission(permission) {
                security_log!(
                    "WARN",
                    "permission_denied",
                    user_id = user.id.clone(),
                    username = user.username.clone(),
                    required_permission = permission
                );
                return Err(AppError::forbidden(format!(
                    "Permission denied: {}",
                    permission
                )));
            }

            Ok(next.run(req).await)
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
    })
}
```

**Step 2: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 3: 提交**

```bash
git add edge-server/src/auth/middleware.rs
git commit -m "refactor(auth): add permission_layer factory for route-level permission checks"
```

---

## Task 3: 为所有 API 路由添加权限中间件

**Files:**
- Modify: 所有 `edge-server/src/api/*/mod.rs` 路由模块 (20个文件)

对每个路由模块，将路由分为读取组和写入组，分别应用不同权限层。

**原则:**
- `auth` 和 `health` 模块不需要权限（已有认证跳过逻辑或公开）
- 读取操作 (GET) 使用 `resource:read` 权限
- 写入操作 (POST/PUT) 使用 `resource:write` 或 `resource:manage` 权限
- 删除操作 (DELETE) 使用 `resource:delete` 或 `resource:manage` 权限
- `sync` 模块使用 `system:read`

**Step 1: 在 auth/mod.rs 导出 permission_layer**

确保 `permission_layer` 从 `crate::auth` 模块可访问。

**Step 2: 逐个模块添加权限层**

每个模块的 `router()` 函数中，将路由分组并添加 `.layer(permission_layer("xxx"))`:

示例 - `products/mod.rs`:
```rust
use crate::auth::permission_layer;

pub fn router() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/api/products", get(handler::list))
        .route("/api/products/{id}", get(handler::get_by_id))
        .route("/api/products/{id}/full", get(handler::get_full))
        .route("/api/products/{id}/attributes", get(handler::list_product_attributes))
        .route("/api/products/by-category/{category_id}", get(handler::list_by_category))
        .layer(permission_layer("products:read"));

    let write_routes = Router::new()
        .route("/api/products", post(handler::create))
        .route("/api/products/sort-order", put(handler::batch_update_sort_order))
        .route("/api/products/{id}", put(handler::update))
        .route("/api/products/{id}/tags/{tag_id}", post(handler::add_product_tag))
        .layer(permission_layer("products:write"));

    let delete_routes = Router::new()
        .route("/api/products/{id}", delete(handler::delete))
        .route("/api/products/{id}/tags/{tag_id}", delete(handler::remove_product_tag))
        .layer(permission_layer("products:delete"));

    read_routes.merge(write_routes).merge(delete_routes)
}
```

**需要修改的所有模块及其权限映射:**

1. **role/mod.rs** - 移除 `require_admin`，改用 `roles:read` / `roles:write`
2. **upload/mod.rs** - `products:write`
3. **tags/mod.rs** - `products:read` / `products:write` / `products:delete`
4. **categories/mod.rs** - `categories:read` / `categories:manage`
5. **products/mod.rs** - `products:read` / `products:write` / `products:delete`
6. **attributes/mod.rs** - `attributes:read` / `attributes:manage`
7. **has_attribute/mod.rs** - `attributes:read` / `attributes:manage`
8. **zones/mod.rs** - `zones:read` / `zones:manage`
9. **tables/mod.rs** - `tables:read` / `tables:manage`
10. **price_rules/mod.rs** - `pricing:read` / `pricing:write`
11. **print_destinations/mod.rs** - `printers:read` / `printers:manage`
12. **print_config/mod.rs** - `printers:read` / `printers:manage`
13. **employees/mod.rs** - `users:read` / `users:manage`
14. **orders/mod.rs** - `orders:read`
15. **system_state/mod.rs** - `system:read` / `system:write`
16. **store_info/mod.rs** - `system:read` / `system:write`
17. **label_template/mod.rs** - `system:read` / `system:write`
18. **shifts/mod.rs** - `system:read` / `system:write`
19. **daily_reports/mod.rs** - `statistics:read` / `system:write` (删除)
20. **statistics/mod.rs** - `statistics:read`
21. **sync/mod.rs** - `system:read`

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译通过

**Step 4: 提交**

```bash
git add edge-server/src/api/
git commit -m "feat(auth): add permission middleware to all API routes"
```

---

## Task 4: 统一前端 Permission 常量命名

**Files:**
- Modify: `red_coral/src/core/domain/types/index.ts`

**Step 1: 更新 Permission 常量为 resource:action 格式**

```typescript
export const Permission = {
  // User management
  USERS_READ: 'users:read' as Permission,
  USERS_MANAGE: 'users:manage' as Permission,
  // Product permissions
  PRODUCTS_READ: 'products:read' as Permission,
  PRODUCTS_WRITE: 'products:write' as Permission,
  PRODUCTS_DELETE: 'products:delete' as Permission,
  PRODUCTS_MANAGE: 'products:manage' as Permission,
  // Category permissions
  CATEGORIES_READ: 'categories:read' as Permission,
  CATEGORIES_MANAGE: 'categories:manage' as Permission,
  // Attribute permissions
  ATTRIBUTES_READ: 'attributes:read' as Permission,
  ATTRIBUTES_MANAGE: 'attributes:manage' as Permission,
  // Order permissions
  ORDERS_READ: 'orders:read' as Permission,
  ORDERS_WRITE: 'orders:write' as Permission,
  ORDERS_VOID: 'orders:void' as Permission,
  ORDERS_DISCOUNT: 'orders:discount' as Permission,
  ORDERS_REFUND: 'orders:refund' as Permission,
  ORDERS_CANCEL_ITEM: 'orders:cancel_item' as Permission,
  // Zone & Table permissions
  ZONES_READ: 'zones:read' as Permission,
  ZONES_MANAGE: 'zones:manage' as Permission,
  TABLES_READ: 'tables:read' as Permission,
  TABLES_MANAGE: 'tables:manage' as Permission,
  TABLES_MERGE_BILL: 'tables:merge_bill' as Permission,
  TABLES_TRANSFER: 'tables:transfer' as Permission,
  // Pricing permissions
  PRICING_READ: 'pricing:read' as Permission,
  PRICING_WRITE: 'pricing:write' as Permission,
  // Statistics
  STATISTICS_READ: 'statistics:read' as Permission,
  // Printer permissions
  PRINTERS_READ: 'printers:read' as Permission,
  PRINTERS_MANAGE: 'printers:manage' as Permission,
  // Receipt permissions
  RECEIPTS_PRINT: 'receipts:print' as Permission,
  RECEIPTS_REPRINT: 'receipts:reprint' as Permission,
  // Settings & System
  SETTINGS_MANAGE: 'settings:manage' as Permission,
  SYSTEM_READ: 'system:read' as Permission,
  SYSTEM_WRITE: 'system:write' as Permission,
  // Role management
  ROLES_READ: 'roles:read' as Permission,
  ROLES_WRITE: 'roles:write' as Permission,
  // POS operations
  POS_CASH_DRAWER: 'pos:cash_drawer' as Permission,
} as const;
```

**Step 2: 验证 TypeScript**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 大量错误（旧常量名不存在），这是预期的

**Step 3: 提交**

```bash
git add red_coral/src/core/domain/types/index.ts
git commit -m "refactor(frontend): rename Permission constants to resource:action format"
```

---

## Task 5: 更新前端所有 Permission 引用

**Files:** (所有引用旧 Permission 常量的文件)
- Modify: `red_coral/src/hooks/usePermission.ts`
- Modify: `red_coral/src/presentation/components/ui/ItemActionPanel/index.tsx`
- Modify: `red_coral/src/features/tag/TagManagement.tsx`
- Modify: `red_coral/src/screens/History/HistoryDetail.tsx`
- Modify: `red_coral/src/screens/POS/components/ActionBar.tsx`
- Modify: `red_coral/src/screens/Settings/index.tsx`
- Modify: `red_coral/src/screens/Settings/SettingsSidebar.tsx`
- Modify: `red_coral/src/screens/TableSelection/components/TableManagementModal.tsx`
- Modify: `red_coral/src/features/product/ProductManagement.tsx`
- Modify: `red_coral/src/features/price-rule/PriceRuleManagement.tsx`
- Modify: `red_coral/src/screens/Checkout/index.tsx`
- Modify: `red_coral/src/features/table/TableManagement.tsx`
- Modify: `red_coral/src/screens/Checkout/payment/PaymentFlow.tsx`
- Modify: `red_coral/src/features/user/UserManagement.tsx`
- Modify: `red_coral/src/features/category/CategoryManagement.tsx`
- Modify: `red_coral/src/features/attribute/AttributeManagement.tsx`

**替换映射:**

| 旧引用 | 新引用 |
|--------|--------|
| `Permission.MANAGE_USERS` | `Permission.USERS_MANAGE` |
| `Permission.VOID_ORDER` | `Permission.ORDERS_VOID` |
| `Permission.MANAGE_PRODUCTS` | `Permission.PRODUCTS_MANAGE` |
| `Permission.CREATE_PRODUCT` | `Permission.PRODUCTS_WRITE` |
| `Permission.UPDATE_PRODUCT` | `Permission.PRODUCTS_WRITE` |
| `Permission.DELETE_PRODUCT` | `Permission.PRODUCTS_DELETE` |
| `Permission.MANAGE_CATEGORIES` | `Permission.CATEGORIES_MANAGE` |
| `Permission.MANAGE_ZONES` | `Permission.ZONES_MANAGE` |
| `Permission.MANAGE_TABLES` | `Permission.TABLES_MANAGE` |
| `Permission.MODIFY_PRICE` | `Permission.PRICING_WRITE` |
| `Permission.APPLY_DISCOUNT` | `Permission.ORDERS_DISCOUNT` |
| `Permission.VIEW_STATISTICS` | `Permission.STATISTICS_READ` |
| `Permission.MANAGE_PRINTERS` | `Permission.PRINTERS_MANAGE` |
| `Permission.MANAGE_ATTRIBUTES` | `Permission.ATTRIBUTES_MANAGE` |
| `Permission.MANAGE_SETTINGS` | `Permission.SETTINGS_MANAGE` |
| `Permission.SYSTEM_SETTINGS` | `Permission.SYSTEM_WRITE` |
| `Permission.PRINT_RECEIPTS` | `Permission.RECEIPTS_PRINT` |
| `Permission.REPRINT_RECEIPT` | `Permission.RECEIPTS_REPRINT` |
| `Permission.REFUND` | `Permission.ORDERS_REFUND` |
| `Permission.DISCOUNT` | `Permission.ORDERS_DISCOUNT` |
| `Permission.CANCEL_ITEM` | `Permission.ORDERS_CANCEL_ITEM` |
| `Permission.OPEN_CASH_DRAWER` | `Permission.POS_CASH_DRAWER` |
| `Permission.MERGE_BILL` | `Permission.TABLES_MERGE_BILL` |
| `Permission.TRANSFER_TABLE` | `Permission.TABLES_TRANSFER` |

同时更新 `usePermission.ts` 中的 hook 函数内的 `PermissionValues.XXX` 引用。

**Step 1: 批量替换所有文件中的旧引用**

逐个替换上述映射中的所有引用。

**Step 2: 验证 TypeScript 编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译通过，无错误

**Step 3: 提交**

```bash
git add red_coral/src/
git commit -m "refactor(frontend): update all Permission references to resource:action format"
```

---

## Task 6: 更新后端 GET /api/permissions 返回值

**Files:**
- Modify: `edge-server/src/api/role/handler.rs` (get_all_permissions handler)

确保 `GET /api/permissions` 返回的权限列表与新的 `ALL_PERMISSIONS` 一致，前端角色权限编辑器能正确显示所有可用权限。

**Step 1: 检查并更新 handler**

确认 `get_all_permissions` handler 是否直接引用 `ALL_PERMISSIONS`，如果是则无需修改（Task 1 已更新）。如果有额外逻辑，需要同步更新。

**Step 2: 检查前端 RolePermissionsEditor**

确认 `red_coral/src/features/role/RolePermissionsEditor.tsx` 是否正确展示新的权限格式。如果它有权限分组显示逻辑，需要更新分组策略（按 `resource:` 前缀分组）。

**Step 3: 验证**

Run: `cargo check -p edge-server && cd red_coral && npx tsc --noEmit`
Expected: 双端编译通过

**Step 4: 提交**

```bash
git add edge-server/src/api/role/ red_coral/src/features/role/
git commit -m "feat(auth): update permissions API and role editor for new permission format"
```

---

## Task 7: 全栈验证

**Step 1: 后端编译验证**

Run: `cargo check --workspace`
Expected: 编译通过

**Step 2: 后端测试**

Run: `cargo test --workspace --lib`
Expected: 所有测试通过

**Step 3: 前端类型检查**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译通过

**Step 4: 最终提交**

```bash
git add -A
git commit -m "feat(auth): complete permission hardening - unified naming and route protection"
```
