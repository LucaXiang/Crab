# 全栈约定与架构原则

本文件是所有跨前后端约定的单一真相源。修改类型、添加约定时参考此文件。

## 类型对齐

TypeScript (前端) <-> Rust (后端) 类型必须完全匹配：
1. 先改 Rust 类型 (`shared/`, `edge-server/src/db/models/`)
2. 再改 TypeScript (`red_coral/src/core/domain/types/`)
3. 验证: `cargo check && npx tsc --noEmit`

## 全栈统一约定

| 约定 | 说明 |
|------|------|
| **ID 格式** | 全栈统一 i64 整数 (SQLite INTEGER PRIMARY KEY) |
| **时间戳** | `i64` Unix 毫秒 (Rust `i64` / TS `number` / SQLite INTEGER) |
| **金额计算** | 后端 `rust_decimal`，前端 `Currency` (decimal.js)，禁止原生浮点 |
| **货币** | 欧元 (€)，前端用 `formatCurrency()` 格式化 |
| **支付方式** | 统一大写: `CASH`, `CARD` |
| **API 错误码** | `shared::error::ErrorCode` (u16，按领域分区 0xxx-9xxx) |
| **命令错误码** | `shared::order::types::CommandErrorCode` — 订单命令失败的结构化错误码，详见 `shared/CLAUDE.md`；前端 `commandErrorMessage(code)` 自动翻译 |
| **异步运行时** | `tokio`，trait object 场景用 `#[async_trait]` |
| **共享状态** | `Arc` 包装，`ServerState` 设计为 clone-cheap |
| **依赖管理** | 所有依赖在 workspace `Cargo.toml` 统一声明 |
| **Tauri 命令参数** | Tauri 2 **仅自动映射顶层命令参数名** (camelCase<->snake_case)，不要在 Rust 命令上加 `rename_all`；**嵌套 struct 字段由 serde 反序列化，前端发送时必须手动转为 snake_case**，接收时手动转为 camelCase |

## 架构原则

| 原则 | 说明 |
|------|------|
| **服务端权威** | 所有金额计算、状态变更由服务端完成，前端不做乐观更新 |
| **事件溯源** | 订单用 Event Sourcing + CQRS，详见 `edge-server/CLAUDE.md` |
| **离线优先** | 边缘节点必须支持完全离线运行 |
| **RBAC 双层防御** | 前端 PermissionGate + 后端 `require_permission()` 中间件 |
| **mTLS 安全** | 生产环境必须启用双向 TLS (TLS 1.3 + aws-lc-rs) |
