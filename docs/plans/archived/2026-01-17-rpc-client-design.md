# Crab Client 可靠 RPC 设计

**创建日期**: 2026-01-17
**作者**: Claude

## 概述

为 crab-client 实现真正的可靠 RPC 通信，支持请求-响应关联、超时控制、自动重试、请求队列、自动重连和心跳保活。

## 核心目标

| 特性 | 描述 |
|------|------|
| 请求-响应关联 | 使用 correlation_id 关联请求和响应 |
| 超时控制 | 可配置请求超时时间 |
| 自动重试 | 指数退避重试策略 |
| 请求队列 | 串行请求处理，避免连接过载 |
| 自动重连 | 连接断开后自动重建 |
| 心跳保活 | 定期心跳检测连接状态 |

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                      RpcClient (同步 API)                    │
│  request() → correlation_id → 超时包装 → 放入队列 → await   │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                       RpcChannel                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  RequestQueue (mpsc channel)                        │   │
│  │  - pending_requests (HashMap<correlation_id, Rx>)   │   │
│  │  - retry_policy (指数退避)                           │   │
│  │  - heartbeat_task (ping/pong 检测)                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│  ┌────────────────────────▼────────────────────────────────┐│
│  │              SingleConnectionManager                    ││
│  │  - TlsStream<TcpStream>                                ││
│  │  - state: Disconnected | Connecting | Connected        ││
│  │  - auto-reconnect with exponential backoff             ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## 核心类型定义

### RpcClient（同步阻塞 API）

```rust
/// RPC 客户端（同步阻塞 API）
pub struct RpcClient {
    channel: Arc<RpcChannel>,
}

impl RpcClient {
    /// 使用 mTLS 创建客户端
    pub async fn connect_mtls(
        addr: &str,
        client_name: &str,
        config: RpcConfig,
        cert_config: TlsCertConfig,
    ) -> Result<Self, RpcError>;

    /// 同步 RPC 调用（阻塞等待结果，带超时）
    pub async fn request<T: DeserializeOwned>(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<T, RpcError>;

    /// 带重试的 RPC 调用
    pub async fn request_with_retry<T: DeserializeOwned>(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<T, RpcError>;

    /// 检查连接状态
    pub fn is_connected(&self) -> bool;

    /// 关闭连接
    pub async fn close(&self);
}
```

### RpcChannel（单连接管理）

```rust
/// RPC 通道（单连接管理）
pub struct RpcChannel {
    config: RpcConfig,
    state: Arc<Mutex<ConnectionState>>,
    pending: Arc<Mutex<PendingRequests>>,
    request_tx: mpsc::Sender<OutgoingRequest>,
    response_rx: mpsc::Receiver<Result<BusMessage, RpcError>>,
    heartbeat_handle: JoinHandle<()>,
    worker_handle: JoinHandle<()>,
}

impl RpcChannel {
    /// 内部请求方法
    async fn send_request(
        &self,
        msg: &BusMessage,
        timeout: Duration,
        retry: bool,
    ) -> Result<BusMessage, RpcError>;

    /// 处理重连
    async fn handle_reconnect(&self) -> Result<(), RpcError>;
}
```

### 配置类型

```rust
/// RPC 配置
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// 默认请求超时（默认 30 秒）
    pub request_timeout: Duration,
    /// 心跳间隔（默认 30 秒）
    pub heartbeat_interval: Duration,
    /// 初始重连延迟（默认 1 秒）
    pub reconnect_delay: Duration,
    /// 最大重连延迟（默认 60 秒）
    pub max_reconnect_delay: Duration,
    /// 最大重试次数（默认 3 次）
    pub max_retries: u32,
    /// 重试退乘数（默认 2.0）
    pub backoff_multiplier: f64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(30),
            reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(60),
            max_retries: 3,
            backoff_multiplier: 2.0,
        }
    }
}

/// TLS 证书配置
pub struct TlsCertConfig {
    pub ca_cert_pem: Vec<u8>,
    pub client_cert_pem: Vec<u8>,
    pub client_key_pem: Vec<u8>,
    pub client_name: String,
}
```

### 错误类型

```rust
/// RPC 错误
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("连接错误: {0}")]
    Connection(String),

    #[error("请求超时")]
    Timeout,

    #[error("重试次数耗尽: {0}")]
    RetriesExhausted(String),

    #[error("连接断开")]
    Disconnected,

    #[error("协议错误: {0}")]
    Protocol(String),

    #[error("序列化错误: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}
```

## 工作流程

### 请求处理流程

```
1. request() 被调用
   │
   ├─► 生成 request_id + correlation_id
   │
   ├─► 创建 oneshot::channel 用于接收响应
   │
   ├─► 放入 pending_requests HashMap
   │
   ├─► 发送到 request_tx channel
   │
   └─► 阻塞等待 oneshot Receiver（带超时）

2. Worker 循环
   │
   ├─► 从 request_tx channel 读取请求
   │
   ├─► 写入 TlsStream（发送）
   │
   ├─► 读取 TlsStream（接收）
   │
   ├─► 收到响应后查找 correlation_id
   │
   ├─► 通过 oneshot 发送回调用方
   │
   └─► 处理超时/断开连接
```

### 心跳/断线检测

```
心跳任务：
├─► 定期（heartbeat_interval）发送 Ping
├─► 期望收到 Pong 或 Response
└─► 超时未响应 → 标记为 Disconnected

重连流程：
├─► 检测到断开
├─► 进入 Reconnecting 状态
├─► 指数退避等待
├─► 尝试重建 TLS 连接
├─► 成功 → 发送 Re-register 请求
└─► 失败 → 继续重连
```

### 重试策略（指数退避）

```
首次重试：delay = base_delay (1s)
第二次重试：delay = base_delay * 2 (2s)
第三次重试：delay = base_delay * 4 (4s)
...
最大延迟：max_delay (60s)

每次请求前检查是否已重试 max_retries 次
```

## 模块结构

```
crab-client/src/
├── lib.rs
├── client/
│   ├── mod.rs
│   ├── crab_client.rs       # 现有（保留）
│   ├── http.rs              # 现有（保留）
│   ├── message.rs           # 现有（保留）
│   └── rpc/                 # 新增
│       ├── mod.rs
│       ├── client.rs        # RpcClient
│       ├── channel.rs       # RpcChannel
│       ├── config.rs        # RpcConfig, TlsCertConfig
│       ├── error.rs         # RpcError
│       ├── state.rs         # ConnectionState
│       └── worker.rs        # 连接 worker
└── message/                 # 现有（保留）
```

## 实现优先级

1. **Phase 1**: 基础框架
   - `RpcConfig`, `RpcError`, `ConnectionState`
   - `RpcChannel` 结构体和 worker 循环
   - 单连接消息发送/接收

2. **Phase 2**: 请求-响应关联
   - `pending_requests` HashMap
   - oneshot channel 用于响应传递
   - correlation_id 匹配

3. **Phase 3**: 超时和重试
   - `tokio::time::timeout` 包装
   - 指数退避重试逻辑
   - 重试次数统计

4. **Phase 4**: 心跳和自动重连
   - `heartbeat_task` 定期 Ping
   - `handle_reconnect` 逻辑
   - 状态转换

5. **Phase 5**: 集成测试
   - 与 edge-server 集成测试
   - 错误场景测试
   - 性能测试

## 向后兼容性

- 保留现有 `MessageClient` trait 和实现
- `RpcClient` 作为新增的可靠 RPC 客户端
- 用户可选择使用 `MessageClient`（简单场景）或 `RpcClient`（可靠场景）

## 使用示例

```rust
use crab_client::{RpcClient, RpcConfig, TlsCertConfig};

// 配置
let config = RpcConfig::default();
let cert_config = TlsCertConfig {
    ca_cert_pem: std::fs::read("certs/ca.pem")?,
    client_cert_pem: std::fs::read("certs/client.pem")?,
    client_key_pem: std::fs::read("certs/client.key")?,
    client_name: "client-001".to_string(),
};

// 连接
let client = RpcClient::connect_mtls(
    "127.0.0.1:8082",
    "client-001",
    config,
    cert_config,
).await?;

// 发送请求
let request = BusMessage::request_command(&RequestCommandPayload {
    command: RequestCommand::Ping,
});

let response: ResponsePayload = client.request(&request, Duration::from_secs(5)).await?;
println!("Response: {:?}", response);

// 关闭
client.close().await;
```
