# CLAUDE.md - Crab Project Guide

## Project Overview
Crab is a distributed restaurant management system written in Rust, featuring an Edge Server and Client architecture. It focuses on reliability, offline capabilities, and type-safe communication.

## Architecture
- **Workspace**:
  - `shared`: Common types, protocols, and message definitions (`Notification`, `ServerCommand`).
  - `edge-server`: The core edge node. Handles HTTP/TCP requests, database (SurrealDB), and message broadcasting. Supports mTLS.
  - `crab-client`: Unified client library supporting both Network (HTTP/TCP) and In-Process (Oneshot/Memory) communication.
  - `crab-cert`: Certificate authority and PKI management library. Handles Root CA, Tenant CA, and entity certificates.
  - `crab-auth`: Authentication server (Port 3001). Manages centralized identity and CA hierarchy.

### Edge Server 详细架构

```
┌─────────────────────────────────────────────────┐
│              HTTP API Layer (Axum)               │
│   /api/auth, /api/role, /api/upload, /health    │
└────────────────────┬────────────────────────────┘
                     │
    ┌────────────────┼────────────────┐
    ▼                ▼                ▼
┌─────────┐   ┌──────────┐   ┌────────────┐
│ Oneshot │   │   Http   │   │  Message   │
│ Client  │   │  Client  │   │  Client    │
│(同进程) │   │ (HTTP)   │   │ (TCP/TLS)  │
└─────────┘   └──────────┘   └────────────┘
                     │
         ┌───────────────────────┐
         │  ServerState (Arc共享) │
         └───────────┬───────────┘
                     │
    ┌────────────────┼────────────────┐
    ▼                ▼                ▼
┌──────────┐  ┌───────────┐  ┌────────────┐
│ Message  │  │ Embedded  │  │  Services  │
│   Bus    │  │    DB     │  │(Cert,Auth) │
│(broadcast)│  │(SurrealDB)│  │            │
└──────────┘  └───────────┘  └────────────┘
```

#### 模块结构 (`edge-server/src/`)
| 目录 | 职责 |
|------|------|
| `core/` | 核心状态 (`ServerState`, `Config`, `Server`) |
| `api/` | HTTP API 路由和处理器 |
| `auth/` | JWT 认证、权限、中间件 |
| `message/` | 消息总线、Transport、Handler、Processor |
| `client/` | 客户端实现 (Http, Oneshot, Message) |
| `services/` | 业务服务 (Https, MessageBus, Cert, Activation) |
| `db/` | SurrealDB 数据访问层 |
| `utils/` | 工具函数 (AppError, Logger 等) |

#### Message 模块结构 (`edge-server/src/message/`)
```
message/
├── mod.rs              # 模块导出 + 测试
├── bus.rs              # MessageBus 核心逻辑
├── tcp_server.rs       # TCP 服务器 (连接管理、握手、消息转发)
├── handler.rs          # 消息处理器 (重试、死信队列)
├── processor.rs        # 具体处理逻辑 (Notification, ServerCommand, RequestCommand)
└── transport/
    ├── mod.rs          # Transport trait + 辅助函数
    ├── tcp.rs          # TcpTransport
    ├── tls.rs          # TlsTransport (mTLS)
    └── memory.rs       # MemoryTransport (同进程)
```

#### 多客户端连接管理
- **存储结构**: `Arc<DashMap<String, Arc<dyn Transport>>>` (无锁并发安全)
- **连接流程**: TCP Connect → TLS Handshake (mTLS) → Handshake Message → Register to DashMap
- **断线处理**: 自动检测并从 DashMap 移除，资源自动释放

#### 消息类型
| 类型 | 方向 | 用途 |
|------|------|------|
| `Handshake` | C→S | 握手验证 (Protocol version, 身份) |
| `RequestCommand` | C→S | 客户端 RPC 请求 (ping, echo, status) |
| `Response` | S→C | 请求响应 (带 correlation_id) |
| `Notification` | S→C | 系统通知/日志 |
| `ServerCommand` | Upstream→S | 上层服务器指令 |
| `Sync` | S→C | 数据同步信号 |

#### ServerState 核心字段
```rust
pub struct ServerState {
    pub config: Config,              // 不可变配置
    pub db: Surreal<Db>,             // SurrealDB (Arc 包装)
    pub message_bus: MessageBusService, // 消息总线
    pub cert_service: CertService,   // 证书管理
    pub activation: ActivationService, // 激活状态
    pub jwt_service: Arc<JwtService>, // JWT 认证
}
// Clone 成本极低 - 所有字段都是 Arc 包装
```

#### 性能特性 (3-4 客户端场景)
- 每客户端内存: ~400 bytes
- 消息队列容量: 1024 条 (broadcast channel)
- 请求延迟: 5-15 ms (简单命令)
- 支持广播 (1→N) 和单播 (1→1) 两种模式

## TLS & Security

### TLS 配置
- **加密后端**: `aws-lc-rs` (FIPS 140-3 合规)
- **TLS 版本**: 仅 TLS 1.3
- **协议**: mTLS (双向认证)

```toml
# Cargo.toml (workspace)
aws-lc-rs = { version = "1", features = ["fips"] }
rustls = { version = "0.23", default-features = false, features = ["aws_lc_rs", "fips"] }
```

### 三层证书验证
```
Layer 1: TLS 握手 (WebPkiClientVerifier)
   → 验证证书链是否由受信 CA 签发

Layer 2: 身份验证 (tcp_server.rs)
   → peer_identity (证书) == client_name (握手消息)

Layer 3: 硬件绑定 (credential.rs)
   → device_id (证书) == generate_hardware_id() (当前机器)
```

### 自定义 X.509 扩展
| OID | 字段 | 用途 |
|-----|------|------|
| `1.3.6.1.4.1.99999.1` | `tenant_id` | 租户标识 |
| `1.3.6.1.4.1.99999.2` | `device_id` | 硬件绑定 |
| `1.3.6.1.4.1.99999.5` | `client_name` | 客户端名称 |

## Build & Test Commands
- **Build**: `cargo build --workspace`
- **Check**: `cargo check --workspace`
- **Test**: `cargo test --workspace --lib`
- **Lint**: `cargo clippy --workspace -- -D warnings`
- **Format**: `cargo fmt`
- **Release Build**: `cargo build --workspace --release`

### Release 编译优化
```toml
[profile.release]
lto = true           # 链接时优化
codegen-units = 1    # 单代码生成单元
opt-level = 3        # 最大优化级别
strip = true         # 移除符号表
```

## Run Examples
- **Interactive Server Demo**:
  ```bash
  cargo run -p edge-server --example interactive_demo
  ```
- **Message Client Demo**:
  ```bash
  cargo run -p crab-client --example message_client
  ```
- **mTLS Certificate Demo**:
  ```bash
  cargo run -p crab-cert --example mtls_demo
  ```
- **Auth Server**:
  ```bash
  cargo run -p crab-auth
  ```

## Key Protocols & Patterns
- **Message Bus**:
  - Uses `Notification` (Server -> Client) and `ServerCommand` (Upstream -> Server) for system communication.
  - Payloads are defined in `shared::message`.
  - Supports both TCP (network) and Memory (in-process) transports.
- **Security & Identity**:
  - **mTLS**: Uses a 3-tier CA hierarchy (Root CA -> Tenant CA -> Entity Certs) for device trust.
  - **Hardware Binding**: Certificates are bound to hardware IDs to prevent cloning.
  - **Storage**: Certificates are stored in `auth_storage/` (gitignored).
- **Server State**:
  - `ServerState` is initialized via `ServerState::initialize(&config).await`.
  - Background tasks must be started explicitly via `state.start_background_tasks().await` if not using `Server::run`.
  - `ServerState` is designed to be clone-cheap (uses `Arc`).
- **Client**:
  - `CrabClient` trait unifies `Http` and `Oneshot` backends.
  - `MessageClient` handles real-time bidirectional communication.

## Dependency Management
所有依赖统一在 workspace `Cargo.toml` 中管理，子 crate 使用 `xxx.workspace = true` 引用。

主要分类：
- **TLS**: `aws-lc-rs`, `rustls`, `tokio-rustls`, `rustls-pemfile`
- **Web**: `axum`, `tower`, `tower-http`, `hyper`
- **Database**: `surrealdb`, `surrealdb-migrations`
- **Cryptography**: `sha2`, `ring`, `rsa`, `argon2`
- **Certificate**: `rcgen`, `x509-parser`, `pem`

## Coding Standards
- **Error Handling**:
  - **Current Phase (PoC/Alpha)**: `unwrap()`/`expect()` are permitted for rapid prototyping and asserting invariants in controlled environments.
  - **Production Goal**: Move towards strict, typed error handling (`AppError`, `Result<T, E>`). Eliminate panics in runtime paths.
- **Async**: Prefer `tokio`. Use `#[async_trait]` for traits with async methods.
- **Ownership**: Prefer borrowing over cloning. Use `Arc` for shared state.

## Project Status & Philosophy
- **Phase**: **Feasibility Testing / Prototype**
- **Edge Server Focus**:
  - Designed as an **Edge Node**: Self-contained, offline-capable, and maintenance-free.
  - **Embedded DB**: Uses embedded SurrealDB to avoid external dependencies.
  - **Future Roadmap**: Transition to strong typing enforcement and robust error handling as the project matures from prototype to production.

## User Preferences
- **Language**: Rust Idiomatic.
- **Concurrency**: Safe patterns (`Arc<Mutex<T>>`, channels).
- **Type System**: Leverage newtypes and traits to enforce invariants.
- **Response Language**: Chinese (Answer in Chinese).
