# Audit Log Resource ID 标准化 — 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 统一 audit_log 的 resource_id 字段为纯 ID 值，消除混乱的 `type:id` 前缀。

**Architecture:** resource_id 只存纯标识值（数字 ID、UUID、username），资源类型由已有的 resource_type 字段承载。修改 10 个带冗余前缀的调用点，其余 40+ 个已正确的调用点无需改动。

**Tech Stack:** Rust (edge-server crate)

---

### Task 1: 修复 auth handler 的 resource_id 前缀

**Files:**
- Modify: `edge-server/src/api/auth/handler.rs`

**Step 1: 移除 5 处 `employee:` 前缀**

Line 51 — 登录失败 (密码错误):
```rust
// 之前
AuditAction::LoginFailed, "auth", format!("employee:{}", username),
// 之后
AuditAction::LoginFailed, "auth", &username,
```

Line 63 — 登录失败 (用户不存在):
```rust
// 之前
AuditAction::LoginFailed, "auth", format!("employee:{}", username),
// 之后
AuditAction::LoginFailed, "auth", &username,
```

Line 98 — 登录成功:
```rust
// 之前
AuditAction::LoginSuccess, "auth", format!("employee:{}", emp.id),
// 之后
AuditAction::LoginSuccess, "auth", emp.id.to_string(),
```

Line 161 — 登出:
```rust
// 之前
AuditAction::Logout, "auth", format!("employee:{}", user.id),
// 之后
AuditAction::Logout, "auth", user.id.to_string(),
```

Line 261 — 权限提升:
```rust
// 之前
format!("employee:{}", authorizer_id),
// 之后
authorizer_id.to_string(),
```

**Step 2: 编译检查**

Run: `cargo check -p edge-server`
Expected: PASS

---

### Task 2: 修复 upload、store_info、print_config handler

**Files:**
- Modify: `edge-server/src/api/upload/handler.rs`
- Modify: `edge-server/src/api/store_info/handler.rs`
- Modify: `edge-server/src/api/print_config/handler.rs`

**Step 1: upload handler — 移除 `image:` 前缀**

`upload/handler.rs` Line 195:
```rust
// 之前
format!("image:{}", hash),
// 之后
hash.clone(),
```

**Step 2: store_info handler — 移除 `store_info:` 前缀**

`store_info/handler.rs` Line 51:
```rust
// 之前
"store_info", "store_info:main",
// 之后
"store_info", "main",
```

**Step 3: print_config handler — 移除 `print_config:` 前缀**

`print_config/handler.rs` Line 52:
```rust
// 之前
"print_config", "print_config:default",
// 之后
"print_config", "default",
```

**Step 4: 编译检查**

Run: `cargo check -p edge-server`
Expected: PASS

---

### Task 3: 修复 system 和 order 的 resource_id

**Files:**
- Modify: `edge-server/src/core/state.rs`
- Modify: `edge-server/src/core/server.rs`
- Modify: `edge-server/src/orders/archive_worker.rs`

**Step 1: state.rs — 系统启动**

`core/state.rs` Line 294:
```rust
// 之前
"server:main",
// 之后
"main",
```

**Step 2: server.rs — 系统关闭**

`core/server.rs` Line 140:
```rust
// 之前
"server:main",
// 之后
"main",
```

**Step 3: archive_worker.rs — 订单归档审计**

`orders/archive_worker.rs` Line 389:
```rust
// 之前
let resource_id = format!("order:{}", snapshot.order_id);
// 之后
let resource_id = snapshot.order_id.to_string();
```

**Step 4: 编译检查**

Run: `cargo check -p edge-server`
Expected: PASS

---

### Task 4: 更新 audit_log! 宏文档

**Files:**
- Modify: `edge-server/src/lib.rs`

**Step 1: 修正 doc comment 示例**

`lib.rs` Line 62:
```rust
// 之前
///     "auth", "employee:emp1",
///     operator_id = Some("emp1".into()),
// 之后
///     "auth", "1",
///     operator_id = Some(1),
```

---

### Task 5: 最终验证 + 提交

**Step 1: Clippy 检查**

Run: `cargo clippy -p edge-server`
Expected: 零警告

**Step 2: 测试**

Run: `cargo test -p edge-server --lib`
Expected: 全部通过

**Step 3: 全量搜索确认无遗漏**

Run: `grep -rn 'employee:\|order:\|server:\|image:\|store_info:\|print_config:' edge-server/src/ --include='*.rs' | grep -v '//\|test\|doc\|mod.rs'`
Expected: 无残留的 `type:id` 格式（仅 doc/test/comment 中可能还有）

**Step 4: 提交**

```bash
git add edge-server/src/api/auth/handler.rs \
        edge-server/src/api/upload/handler.rs \
        edge-server/src/api/store_info/handler.rs \
        edge-server/src/api/print_config/handler.rs \
        edge-server/src/core/state.rs \
        edge-server/src/core/server.rs \
        edge-server/src/orders/archive_worker.rs \
        edge-server/src/lib.rs
git commit -m "fix(audit): standardize resource_id to pure ID values

Remove redundant type prefixes (employee:, order:, server:, image:,
store_info:, print_config:) from audit log resource_id field.
Resource type is already captured by the resource_type field."
```
