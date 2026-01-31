# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**现阶段项目是开发阶段, 不要适配层,不要兼容性,不要留下技术债,不要留下历史包裹**

## Crab - 分布式餐饮管理系统

Rust workspace 架构，专注离线优先、边缘计算、mTLS 安全通信的 POS 系统。

## Workspace 成员

| Crate | 用途 | 详细文档 |
|-------|------|----------|
| `shared` | 共享类型、协议、错误系统、事件溯源定义 | [`shared/CLAUDE.md`](shared/CLAUDE.md) |
| `edge-server` | 边缘服务器 (SurrealDB + Axum + MessageBus + 事件溯源) | [`edge-server/CLAUDE.md`](edge-server/CLAUDE.md) |
| `crab-client` | 统一客户端库 (Local/Remote + Typestate + 心跳重连) | [`crab-client/CLAUDE.md`](crab-client/CLAUDE.md) |
| `crab-cert` | PKI/证书管理 (Root CA → Tenant CA → Entity) | [`crab-cert/CLAUDE.md`](crab-cert/CLAUDE.md) |
| `crab-auth` | 认证服务器 (激活 + 订阅校验) | [`crab-auth/CLAUDE.md`](crab-auth/CLAUDE.md) |
| `crab-printer` | ESC/POS 热敏打印底层库 (GBK 编码) | [`crab-printer/CLAUDE.md`](crab-printer/CLAUDE.md) |
| `red_coral` | **Tauri POS 前端** (React 19 + Zustand + Tailwind) | [`red_coral/CLAUDE.md`](red_coral/CLAUDE.md) |

## 技术文档

| 主题 | 文档 |
|------|------|
| SurrealDB & SurrealQL | [`docs/SURREALDB.md`](docs/SURREALDB.md) |

## 命令

```bash
# Rust workspace
cargo check --workspace        # 类型检查
cargo build --workspace        # 编译
cargo test --workspace --lib   # 测试
cargo clippy --workspace       # Lint

# POS 前端 (red_coral/)
cd red_coral && npm run tauri:dev   # Tauri 开发
cd red_coral && npx tsc --noEmit    # TS 类型检查
```

## 核心架构

```
┌─────────────────┐     ┌─────────────────┐
│   red_coral     │     │   crab-auth     │
│  (Tauri POS)    │     │ (认证 + 订阅)   │
└────────┬────────┘     └────────┬────────┘
         │ In-Process / mTLS     │ 激活/证书/订阅
         ▼                       ▼
┌─────────────────────────────────────────┐
│            edge-server                   │
│  ┌─────────┬──────────┬──────────────┐  │
│  │ Axum API│ MessageBus│ SurrealDB   │  │
│  │ (HTTP)  │ (TCP/TLS) │ (Embedded)  │  │
│  └─────────┴──────────┴──────────────┘  │
│  ┌─────────┬──────────┬──────────────┐  │
│  │ Orders  │ Pricing  │ Printing     │  │
│  │(Event   │ (规则    │ (厨房/标签)  │  │
│  │ Sourcing)│ 引擎)   │              │  │
│  └─────────┴──────────┴──────────────┘  │
│  ┌─────────────────────────────────────┐ │
│  │ redb (事件存储) + ArchiveWorker     │ │
│  └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**ClientBridge 双模式** (详见 [`red_coral/CLAUDE.md`](red_coral/CLAUDE.md)):
- **Server 模式**: 内嵌 edge-server，进程内通信 (LocalClient)
- **Client 模式**: mTLS 连接远程 edge-server (RemoteClient)

## 跨项目规则

### 类型对齐

TypeScript (前端) ↔ Rust (后端) 类型必须完全匹配：
1. 先改 Rust 类型 (`shared/`, `edge-server/src/db/models/`)
2. 再改 TypeScript (`red_coral/src/core/domain/types/`)
3. 验证: `cargo check && npx tsc --noEmit`

### 全栈统一约定

| 约定 | 说明 |
|------|------|
| **ID 格式** | 全栈统一 `"table:id"` 字符串，后端用 `RecordId`，详见 [`edge-server/CLAUDE.md`](edge-server/CLAUDE.md) |
| **时间戳** | `i64` Unix 毫秒 (Rust `i64` / TS `number` / SurrealDB `int`) |
| **金额计算** | 后端 `rust_decimal`，前端 `Currency` (decimal.js)，禁止原生浮点 |
| **货币** | 欧元 (€)，前端用 `formatCurrency()` 格式化 |
| **支付方式** | 统一大写: `CASH`, `CARD` |
| **错误码** | `shared::error::ErrorCode` (u16，按领域分区 0xxx-9xxx) |
| **异步运行时** | `tokio`，trait object 场景用 `#[async_trait]` |
| **共享状态** | `Arc` 包装，`ServerState` 设计为 clone-cheap |
| **依赖管理** | 所有依赖在 workspace `Cargo.toml` 统一声明 |
| **Tauri 命令参数** | Tauri 2 **仅自动映射顶层命令参数名** (camelCase↔snake_case)，不要在 Rust 命令上加 `rename_all`；**嵌套 struct 字段由 serde 反序列化，前端发送时必须手动转为 snake_case**，接收时手动转为 camelCase |

### 架构原则

| 原则 | 说明 |
|------|------|
| **服务端权威** | 所有金额计算、状态变更由服务端完成，前端不做乐观更新 |
| **事件溯源** | 订单用 Event Sourcing + CQRS，详见 [`edge-server/CLAUDE.md`](edge-server/CLAUDE.md) |
| **离线优先** | 边缘节点必须支持完全离线运行 |
| **RBAC 双层防御** | 前端 PermissionGate + 后端 `require_permission()` 中间件 |
| **mTLS 安全** | 生产环境必须启用双向 TLS (TLS 1.3 + aws-lc-rs) |

### 禁止事项

- ❌ 直接删除 Order/OrderEvent 记录 (用 VOID 状态管理)
- ❌ 前端直接进行金额浮点运算 (用 `Currency` 类)
- ❌ 跳过类型对齐直接部署
- ❌ 在非 mTLS 环境传输敏感数据
- ❌ 子 crate 单独声明依赖版本
- ❌ 使用 `surrealdb::sql::Thing` (用 `RecordId`)
- ❌ 使用 `string` 格式的时间戳 (用 `i64` Unix 毫秒)
- ❌ EventApplier 中执行 I/O 或副作用
- ❌ 使用 `f64` 进行金额计算 (用 `rust_decimal`)

## 响应语言

使用中文回答。
