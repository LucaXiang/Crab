# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**现阶段项目是开发阶段, 不要适配层,不要兼容性,不要留下技术债,不要留下历史包裹**

## Crab - 分布式餐饮管理系统

Rust workspace 架构，专注离线优先、边缘计算、mTLS 安全通信。

## Workspace 成员

| Crate | 用途 | 详细文档 |
|-------|------|----------|
| `shared` | 共享类型、协议、消息定义 | [`shared/CLAUDE.md`](shared/CLAUDE.md) |
| `edge-server` | 边缘服务器 (SurrealDB + Axum + MessageBus) | [`edge-server/CLAUDE.md`](edge-server/CLAUDE.md) |
| `crab-client` | 统一客户端库 (Local/Remote + Typestate) | [`crab-client/CLAUDE.md`](crab-client/CLAUDE.md) |
| `crab-cert` | PKI/证书管理 (Root CA → Tenant CA → Entity) | [`crab-cert/CLAUDE.md`](crab-cert/CLAUDE.md) |
| `crab-auth` | 认证服务器 (Port 3001) | [`crab-auth/CLAUDE.md`](crab-auth/CLAUDE.md) |
| `red_coral` | **Tauri POS 前端** | [`red_coral/CLAUDE.md`](red_coral/CLAUDE.md) |

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
cd red_coral && npm run tauri:dev   # 开发
cd red_coral && npx tsc --noEmit    # TS 检查
```

## 核心架构

```
┌─────────────────┐     ┌─────────────────┐
│   red_coral     │     │   crab-auth     │
│  (Tauri POS)    │     │ (认证服务器)     │
└────────┬────────┘     └────────┬────────┘
         │ In-Process / mTLS     │ 激活/证书
         ▼                       ▼
┌─────────────────────────────────────────┐
│            edge-server                   │
│  ┌─────────┬──────────┬──────────────┐  │
│  │ Axum API│ MessageBus│ SurrealDB   │  │
│  │ (HTTP)  │ (TCP/TLS) │ (Embedded)  │  │
│  └─────────┴──────────┴──────────────┘  │
└─────────────────────────────────────────┘
```

**ClientBridge 双模式**:
- Server 模式: 内嵌 edge-server，进程内通信 (LocalClient)
- Client 模式: mTLS 连接远程 edge-server (RemoteClient)

## 数据库规则 (SurrealDB)

### ID 格式规范

**核心原则**: 全栈统一使用 `surrealdb::RecordId`，格式 `"table:id"`。

| 层级 | 类型 | 示例 |
|------|------|------|
| **前端 (TypeScript)** | `string` | `"product:abc123"` |
| **API 传输 (JSON)** | `string` | `"product:abc123"` |
| **后端 (Rust)** | `RecordId` | `surrealdb::RecordId` |
| **数据库** | `record` | SurrealDB 原生 record |

**RecordId 使用**:
```rust
use surrealdb::RecordId;

// 解析
let id: RecordId = "product:abc".parse()?;

// 创建
let id = RecordId::from_table_key("product", "abc");

// 获取组件
id.table()        // "product"
id.key()          // Key 类型
id.to_string()    // "product:abc"

// SDK 操作
db.select(id.clone()).await?;
db.delete(id).await?;
```

**禁止**:
- ❌ `surrealdb::sql::Thing` - 不要用
- ❌ `serde_thing` 模块 - 已删除
- ❌ 任何 ID 适配层/转换层

**RELATE 边关系**:
- `attribute_binding`: product/category → attribute
- `has_event`: order → order_event

**删除规则**:
- Order/OrderEvent **禁止删除** (使用状态管理)
- 删除 Product/Category 时**必须清理** `attribute_binding` 边
- 删除 Attribute 时自动清理关联边

## 安全

- mTLS 双向认证 (TLS 1.3 + aws-lc-rs FIPS)
- 三层 CA 层级: Root CA → Tenant CA → Entity Cert
- 硬件绑定: 证书包含 device_id 防克隆

## 类型对齐

TypeScript (前端) ↔ Rust (后端) 类型必须完全匹配：
1. 先改 Rust 类型 (`shared/`, `edge-server/src/db/models/`)
2. 再改 TypeScript (`red_coral/src/core/domain/types/`)
3. 验证: `cargo check && npx tsc --noEmit`

## 约束与准则

### 代码规范

| 规则 | 说明 |
|------|------|
| **类型优先** | 修改数据结构时：Rust 先行 → TypeScript 跟进 → 双端验证 |
| **金额计算** | 前端必须使用 `Currency` 工具类，禁止直接浮点运算 |
| **错误处理** | 使用 `shared::error::ErrorCode` 统一错误码 |
| **异步运行时** | 统一使用 `tokio`，trait object (`dyn Trait`) 场景使用 `#[async_trait]` |
| **共享状态** | 使用 `Arc` 包装，`ServerState` 设计为 clone-cheap |

### 数据库约束

| 约束 | 说明 |
|------|------|
| **Order 不可删除** | 订单使用状态管理（VOID），禁止物理删除 |
| **OrderEvent 不可删除** | 事件溯源，只追加不删除 |
| **RELATE 边清理** | 删除实体时必须清理关联的图边 |
| **ID 格式** | 全栈统一 `"table:id"` 格式，使用 `serde_thing` 自动转换 |

### 安全约束

| 约束 | 说明 |
|------|------|
| **mTLS 必须** | 生产环境必须启用双向 TLS 认证 |
| **硬件绑定** | 证书包含 device_id，防止凭据克隆 |
| **密码存储** | 使用 Argon2 哈希，禁止明文存储 |
| **JWT 过期** | Access Token 短期，Refresh Token 长期 |

### 架构约定

| 约定 | 说明 |
|------|------|
| **离线优先** | 边缘节点必须支持完全离线运行 |
| **事件溯源** | 订单系统使用 Event Sourcing 模式 |
| **类型状态** | 客户端使用 Typestate 模式确保状态转换安全 |
| **依赖集中** | 所有依赖在 workspace Cargo.toml 统一管理 |

### 禁止事项

- ❌ 直接删除 Order/OrderEvent 记录
- ❌ 前端直接进行金额浮点运算
- ❌ 跳过类型对齐直接部署
- ❌ 在非 mTLS 环境传输敏感数据
- ❌ 子 crate 单独声明依赖版本

## 响应语言

使用中文回答。
