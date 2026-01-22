# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Edge Server

分布式餐厅管理系统边缘节点 - 嵌入式数据库 + RESTful API + 实时消息总线。

## 命令

```bash
cargo check -p edge-server
cargo test -p edge-server --lib
cargo run -p edge-server --example interactive_demo
```

## 模块结构

```
src/
├── core/       # Config, ServerState, Server
├── api/        # HTTP 路由和处理器 (Axum)
├── auth/       # JWT + Argon2 认证
├── db/         # SurrealDB 数据访问层
│   ├── models/     # 数据模型
│   └── repository/ # CRUD 操作
├── message/    # 消息总线 (TCP/TLS/Memory)
├── orders/     # 订单事件溯源
├── pricing/    # 价格规则引擎
├── services/   # 业务服务 (Cert, Activation, MessageBus)
└── utils/      # AppError, Logger
```

## 核心概念

### ServerState

```rust
pub struct ServerState {
    pub config: Config,
    pub db: Surreal<Db>,           // SurrealDB
    pub message_bus: MessageBusService,
    pub cert_service: CertService,
    pub jwt_service: Arc<JwtService>,
}
// Clone 成本极低 - 所有字段都是 Arc 包装
```

### 数据库 RELATE 边

- `has_attribute`: product/category → attribute
- `has_event`: order → order_event

**删除规则**:
- Order/OrderEvent **禁止删除**
- 删除 Product/Category 时**必须清理** `has_attribute` 边

### 添加 API

1. `api/<resource>/` 创建 `mod.rs` + `handler.rs`
2. `api/mod.rs` 添加路由
3. 使用 `ok!()` 宏返回响应

## 响应语言

使用中文回答。
