# CrabClient 统一客户端架构设计

## 1. 概述

### 1.1 目标

设计一个统一的 `CrabClient` 客户端库，支持两种运行模式：
- **远程模式 (RemoteMode)**: 跨进程通信，通过 HTTPS + TLS/TCP 连接 Edge Server
- **本地模式 (LocalMode)**: 同进程通信，通过 tower oneshot + MemoryTransport

用户使用统一的 API，底层传输方式对用户透明。

### 1.2 核心原则

- **项目隔离**: `crab-client` 不依赖 `edge-server` 内部类型
- **最小接口**: 只通过 `Router` 和 `broadcast::Sender` 暴露必要依赖
- **证书管理**: 自动维护 mTLS 证书，按 client_name 存储
- **明确子客户端**: HTTP 和 Message 子客户端分开，通过 `http()` 和 `message()` 获取

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         用户代码                                  │
│                                                                 │
│  let config = RemoteClientConfig::new()                         │
│      .with_auth_url("http://localhost:3001")                    │
│      .with_edge_url("http://localhost:8080")                    │
│      .with_tcp_addr("localhost:8081")                           │
│      .with_cert_path("/path/to/certs")                          │
│      .with_client_name("my-client");                            │
│                                                                 │
│  let mut client = CrabClient::new(config).await?;               │
│  client.login("employee_id", "password").await?;                │
│                                                                 │
│  // HTTP 操作                                                    │
│  client.http().get_products().await?;                           │
│  client.http().update_employee(&emp).await?;                    │
│                                                                 │
│  // 消息总线操作                                                  │
│  let mut msg = client.message();                                │
│  msg.send(&notification).await?;                                │
│  let event = msg.recv().await?;                                 │
│                                                                 │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CrabClient<M>                                │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ M = RemoteMode                                          │   │
│  │  - http: HttpClient (reqwest)                           │   │
│  │  - message: MessageClient (TLS/TCP)                     │   │
│  │  - cert_manager: CertificateManager                     │   │
│  │  - token: Option<String>                                │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ M = LocalMode                                           │   │
│  │  - http: OneshotClient (tower oneshot + Router)         │   │
│  │  - message: MemoryMessageClient (MemoryTransport)       │   │
│  │  - token: Option<String>                                │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 传输方式分工

| 传输方式 | 用途 | 场景 |
|---------|------|------|
| **HTTPS** | REST API，更新静态数据 | 产品信息、员工信息、配置更新 |
| **TLS/TCP** | 双向实时通信，消息推送，数据同步 | 通知、实时同步、事件订阅 |

---

## 3. 模块设计

### 3.1 模块结构

```
crab-client/src/
├── lib.rs                 # 主入口，统一 CrabClient
├── client/
│   ├── mod.rs            # 客户端模块导出
│   ├── crab_client.rs    # CrabClient 主结构 + 泛型
│   ├── http.rs           # HTTP 客户端 (HttpClient + OneshotClient)
│   └── message.rs        # 消息客户端 (MessageClient + MemoryMessageClient)
├── config/
│   ├── mod.rs            # 配置模块导出
│   ├── remote.rs         # RemoteClientConfig
│   └── local.rs          # LocalClientConfig
├── cert/
│   ├── mod.rs            # 证书模块导出
│   ├── manager.rs        # CertificateManager
│   └── storage.rs        # 证书存储 (按 client_name 目录结构)
├── error.rs              # 错误类型
└── auth.rs               # 认证相关 (LoginResponse, CurrentUserResponse)
```

### 3.2 核心类型定义

```rust
// ========== 模式标记 ==========

/// 远程模式标记
struct RemoteMode;

/// 本地模式标记
struct LocalMode;

// ========== CrabClient ==========

/// 统一的 CrabClient 客户端
#[derive(Clone)]
struct CrabClient<M> {
    mode: PhantomData<M>,
    http: HttpClient,
    message: MessageClient,
    token: Option<String>,
}

impl<M> CrabClient<M> {
    /// 获取 HTTP 子客户端
    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    /// 获取消息子客户端 (可变)
    pub fn message(&mut self) -> &mut MessageClient {
        &mut self.message
    }

    /// 获取当前 token
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

impl CrabClient<RemoteMode> {
    /// 创建远程客户端
    pub async fn new(config: RemoteClientConfig) -> Result<Self, ClientError> {
        // 1. 加载或申请证书
        let cert_manager = CertificateManager::new(config.cert_path(), config.client_name());
        cert_manager.load_or_request().await?;

        // 2. 创建 HTTP 客户端 (带 mTLS)
        let http = HttpClient::new(&config, &cert_manager)?;

        // 3. 创建 Message 客户端 (TCP/TLS)
        let message = MessageClient::connect_tls(
            config.tcp_addr(),
            "localhost",
            cert_manager.tls_config(),
            config.client_name(),
        ).await?;

        Ok(Self {
            mode: PhantomData,
            http,
            message,
            token: None,
        })
    }
}

impl CrabClient<LocalMode> {
    /// 创建本地客户端
    pub fn new(config: LocalClientConfig) -> Self {
        let http = OneshotClient::new(config.router());
        let message = MemoryMessageClient::new(config.bus_tx(), config.bus_rx());

        Self {
            mode: PhantomData,
            http,
            message,
            token: None,
        }
    }
}

// ========== 子客户端 ==========

/// HTTP 客户端
trait HttpClient: Send + Sync {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError>;
    async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T, ClientError>;
    // ... 其他方法
}

/// 消息客户端
trait MessageClient: Send + Sync {
    async fn send(&self, msg: &BusMessage) -> Result<(), MessageError>;
    async fn request(&self, msg: &BusMessage) -> Result<BusMessage, MessageError>;
    async fn recv(&self) -> Result<BusMessage, MessageError>;
}
```

### 3.3 配置结构

```rust
// ========== 远程配置 ==========

pub struct RemoteClientConfig {
    auth_url: String,           // Auth Server 地址
    edge_url: String,           // Edge Server HTTP 地址
    tcp_addr: String,           // Edge Server TCP 地址
    cert_path: PathBuf,         // 证书存储路径
    client_name: String,        // 客户端名称 (也是目录名)
    timeout: Duration,
}

impl RemoteClientConfig {
    pub fn new() -> Self { ... }
    pub fn with_auth_url(mut self, url: &str) -> Self { ... }
    pub fn with_edge_url(mut self, url: &str) -> Self { ... }
    pub fn with_tcp_addr(mut self, addr: &str) -> Self { ... }
    pub fn with_cert_path(mut self, path: impl Into<PathBuf>) -> Self { ... }
    pub fn with_client_name(mut self, name: &str) -> Self { ... }
    pub fn with_timeout(mut self, secs: u64) -> Self { ... }
}

// ========== 本地配置 ==========

pub struct LocalClientConfig {
    router: Router,
    bus_tx: broadcast::Sender<BusMessage>,
    bus_rx: Arc<Mutex<broadcast::Receiver<BusMessage>>>,
}

impl LocalClientConfig {
    pub fn new(
        router: Router,
        bus_tx: broadcast::Sender<BusMessage>,
        bus_rx: Arc<Mutex<broadcast::Receiver<BusMessage>>>,
    ) -> Self {
        Self { router, bus_tx, bus_rx }
    }
}
```

### 3.4 证书管理

```rust
/// 证书存储结构
auth_storage/
└── {client_name}/
    ├── client_cert.pem      # 客户端证书
    ├── client_key.pem       # 客户端私钥
    └── tenant_ca_cert.pem   # 租户 CA 证书

/// CertificateManager
struct CertificateManager {
    cert_path: PathBuf,
    client_name: String,
}

impl CertificateManager {
    /// 加载或申请证书
    async fn load_or_request(&self, auth_url: &str) -> Result<(), CertError> {
        // 1. 检查本地证书是否存在
        if self.cert_exists() {
            // 2. 检查是否过期
            if self.is_expired() {
                // 证书过期，提示用户重新登录
                return Err(CertError::Expired("证书已过期，请重新登录".into()));
            }
            return Ok(());
        }

        // 3. 证书不存在，申请新证书
        self.request_cert(auth_url).await?;
        Ok(())
    }

    /// 获取 TLS 配置 (用于 mTLS)
    pub fn tls_config(&self) -> ClientConfig { ... }
}
```

---

## 4. 认证流程

### 4.1 远程模式登录流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     CrabClient::login()                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 验证/获取 mTLS 证书                                          │
│     ├─ 检查本地证书是否存在                                       │
│     ├─ 检查是否过期                                              │
│     └─ 如需新证书，通过 HTTP 请求 Auth Server                     │
│                                                                 │
│  2. HTTP 登录到 Edge Server                                      │
│     POST /api/auth/login                                        │
│     Body: { username, password }                                │
│     Response: { token, user_info }                              │
│                                                                 │
│  3. 保存 token (用于后续请求)                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 本地模式登录流程

```
┌─────────────────────────────────────────────────────────────────┐
│                  OneshotClient::login()                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 直接调用 Router (oneshot)                                    │
│     POST /api/auth/login                                        │
│     Body: { username, password }                                │
│                                                                 │
│  2. 返回 LoginResponse (含 token)                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("认证失败: {0}")]
    AuthFailed(String),

    #[error("证书错误: {0}")]
    Certificate(#[from] CertError),

    #[error("HTTP 请求失败: {0}")]
    HttpRequest(String),

    #[error("消息错误: {0}")]
    Message(#[from] MessageError),

    #[error("未授权访问")]
    Unauthorized,

    #[error("请求超时: {0}")]
    Timeout(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CertError {
    #[error("证书存储失败: {0}")]
    Storage(String),

    #[error("证书申请失败: {0}")]
    RequestFailed(String),

    #[error("证书已过期，请重新登录")]
    Expired(String),
}
```

---

## 6. 使用示例

### 6.1 远程模式

```rust
use crab_client::{CrabClient, RemoteClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 配置
    let config = RemoteClientConfig::new()
        .with_auth_url("http://localhost:3001")
        .with_edge_url("http://localhost:8080")
        .with_tcp_addr("localhost:8081")
        .with_cert_path("./certs")
        .with_client_name("kitchen-client")
        .with_timeout(30);

    // 创建客户端
    let mut client = CrabClient::new(config).await?;

    // 登录
    client.login("emp001", "password123").await?;
    println!("登录成功! Token: {:?}", client.token());

    // HTTP 操作
    let products = client.http().get_products().await?;
    println!("产品列表: {:?}", products);

    // 消息操作
    let mut msg = client.message();
    msg.send(&notification).await?;
    let event = msg.recv().await?;

    Ok(())
}
```

### 6.2 本地模式 (同进程)

```rust
use crab_client::{CrabClient, LocalClientConfig};
use edge_server::MessageBus;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 假设已创建 Router 和 MessageBus
    let (router, message_bus) = create_server().await;

    // 配置
    let config = LocalClientConfig::new(
        router,
        message_bus.tx(),
        message_bus.rx(),
    );

    // 创建客户端 (同 API!)
    let mut client = CrabClient::new(config).await?;

    // 登录
    client.login("emp001", "password123").await?;

    // 后续操作完全相同
    let products = client.http().get_products().await?;

    Ok(())
}
```

---

## 7. 依赖关系

### 7.1 crab-client 依赖

```toml
[dependencies]
# Web
reqwest = { version = "0.11", features = ["json"] }
axum = { version = "0.7", optional = true }
tower = { version = "0.4", optional = true }

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# TLS
rustls = { version = "0.23" }
aws-lc-rs = { version = "1" }
tokio-rustls = "0.26"
rustls-pemfile = "2"

# Utils
thiserror = "1.0"
tracing = "0.1"
uuid = { version = "1" }

# Shared (workspace)
shared = { path = "../shared" }
```

### 7.2 项目隔离

- `crab-client` **不** 依赖 `edge-server` 内部类型
- 本地模式仅依赖 `Router` (axum) 和 `broadcast::Sender`
- `edge-server` 导出必要的类型供本地模式使用

---

## 8. 实现计划

### Phase 1: 基础结构
- [ ] 定义 `RemoteMode` / `LocalMode` 标记类型
- [ ] 创建 `RemoteClientConfig` / `LocalClientConfig`
- [ ] 实现 `CrabClient<M>` 主结构

### Phase 2: HTTP 客户端
- [ ] 实现 `HttpClient` trait (reqwest)
- [ ] 实现 `OneshotClient` (tower oneshot)
- [ ] 实现 `CrabClient::http()` 子客户端

### Phase 3: 消息客户端
- [ ] 实现 `Transport` trait 统一接口
- [ ] 实现 `MessageClient` (TCP/TLS)
- [ ] 实现 `MemoryMessageClient` (MemoryTransport)
- [ ] 实现 `CrabClient::message()` 子客户端

### Phase 4: 证书管理
- [ ] 实现 `CertificateManager`
- [ ] 实现证书存储 (按 client_name 目录)
- [ ] 实现证书申请流程

### Phase 5: 认证集成
- [ ] 实现 `login()` / `me()` / `logout()` 方法
- [ ] 集成 token 管理
- [ ] 错误处理完善

---

## 9. 后续优化

- [ ] 连接池管理
- [ ] 自动重连机制
- [ ] 性能优化 (连接复用)
- [ ] 更多 HTTP 方法支持
