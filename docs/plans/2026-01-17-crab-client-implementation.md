# CrabClient 统一客户端实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现统一的 `CrabClient<M>` 客户端架构，支持 RemoteMode (HTTPS + TLS/TCP) 和 LocalMode (oneshot + MemoryTransport)

**Architecture:** 使用泛型 `CrabClient<M>` 统一入口，通过 `client.http()` 和 `client.message()` 获取子客户端。远程模式使用 HTTPS + TLS/TCP，本地模式使用 oneshot + MemoryTransport。证书按 client_name 目录存储。

**Tech Stack:** Rust, reqwest, tokio, tower, axum, rustls, crab-cert

---

## 前置条件

在开始实现前，需要完成以下准备工作：

1. 创建 worktree：`git worktree add .worktrees/crab-client -b feature/crab-client`
2. 确认 `crab-client/Cargo.toml` 依赖正确

---

## Phase 1: 基础结构

### Task 1: 重构目录结构

**Files:**
- Create: `crab-client/src/client/mod.rs`
- Create: `crab-client/src/client/http.rs`
- Create: `crab-client/src/client/message.rs`
- Modify: `crab-client/src/lib.rs:21-29`

**Step 1: 创建目录和文件**

```bash
mkdir -p crab-client/src/client
touch crab-client/src/client/mod.rs
touch crab-client/src/client/http.rs
touch crab-client/src/client/message.rs
```

**Step 2: 编写 client/mod.rs**

```rust
// crab-client/src/client/mod.rs

pub mod http;
pub mod message;

pub use http::{HttpClient, HttpClientImpl};
pub use message::{MessageClient, MessageClientImpl};
```

**Step 3: 移动 http.rs 内容到 client/http.rs**

```rust
// crab-client/src/client/http.rs

use crate::{ApiResponse, ClientConfig, ClientError, ClientResult, CurrentUserResponse, LoginResponse};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

/// HTTP client trait for network-based API calls
#[async_trait::async_trait]
pub trait HttpClient: Send + Sync {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<T>;
    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse>;
    async fn me(&self) -> ClientResult<CurrentUserResponse>;
    async fn logout(&mut self) -> ClientResult<()>;
    fn token(&self) -> Option<&str>;
}

/// Network HTTP client implementation
#[derive(Debug, Clone)]
pub struct HttpClientImpl {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl HttpClientImpl {
    pub fn new(config: &ClientConfig) -> Self { /* 现有实现 */ }
}

#[async_trait::async_trait]
impl HttpClient for HttpClientImpl {
    // 实现所有方法
}
```

**Step 4: 创建空的 message/client.rs**

```rust
// crab-client/src/client/message.rs

use async_trait::async_trait;
use crate::{ClientError, ClientResult};

#[async_trait]
pub trait MessageClient: Send + Sync {
    async fn send(&self, msg: &shared::message::BusMessage) -> Result<(), crate::message::MessageError>;
    async fn request(&self, msg: &shared::message::BusMessage) -> Result<shared::message::BusMessage, crate::message::MessageError>;
    async fn recv(&self) -> Result<shared::message::BusMessage, crate::message::MessageError>;
}
```

**Step 5: 更新 lib.rs**

```rust
// crab-client/src/lib.rs

pub mod client;
```

**Step 6: 运行检查**

```bash
cd /Users/xzy/workspace/crab
cargo check -p crab-client
```

**Step 7: 提交**

```bash
git add crab-client/src/client/ crab-client/src/lib.rs
git commit -m "refactor: restructure client module"
```

---

### Task 2: 创建配置模块

**Files:**
- Create: `crab-client/src/config/mod.rs`
- Create: `crab-client/src/config/remote.rs`
- Create: `crab-client/src/config/local.rs`
- Modify: `crab-client/src/lib.rs`

**Step 1: 创建 config 目录和文件**

```bash
mkdir -p crab-client/src/config
touch crab-client/src/config/mod.rs
touch crab-client/src/config/remote.rs
touch crab-client/src/config/local.rs
```

**Step 2: 编写 remote.rs**

```rust
// crab-client/src/config/remote.rs

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RemoteClientConfig {
    auth_url: String,
    edge_url: String,
    tcp_addr: String,
    cert_path: PathBuf,
    client_name: String,
    timeout: Duration,
}

impl RemoteClientConfig {
    pub fn new() -> Self {
        Self {
            auth_url: "http://localhost:3001".to_string(),
            edge_url: "http://localhost:8080".to_string(),
            tcp_addr: "localhost:8081".to_string(),
            cert_path: PathBuf::from("./certs"),
            client_name: "crab-client".to_string(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_auth_url(mut self, url: &str) -> Self {
        self.auth_url = url.to_string();
        self
    }

    pub fn with_edge_url(mut self, url: &str) -> Self {
        self.edge_url = url.to_string();
        self
    }

    pub fn with_tcp_addr(mut self, addr: &str) -> Self {
        self.tcp_addr = addr.to_string();
        self
    }

    pub fn with_cert_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cert_path = path.into();
        self
    }

    pub fn with_client_name(mut self, name: &str) -> Self {
        self.client_name = name.to_string();
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    // Getters
    pub fn auth_url(&self) -> &str { &self.auth_url }
    pub fn edge_url(&self) -> &str { &self.edge_url }
    pub fn tcp_addr(&self) -> &str { &self.tcp_addr }
    pub fn cert_path(&self) -> &PathBuf { &self.cert_path }
    pub fn client_name(&self) -> &str { &self.client_name }
    pub fn timeout(&self) -> Duration { self.timeout }
}
```

**Step 3: 编写 local.rs**

```rust
// crab-client/src/config/local.rs

use axum::Router;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

#[derive(Debug)]
pub struct LocalClientConfig {
    router: Router,
    bus_tx: broadcast::Sender<shared::message::BusMessage>,
    bus_rx: Arc<Mutex<broadcast::Receiver<shared::message::BusMessage>>>,
}

impl LocalClientConfig {
    pub fn new(
        router: Router,
        bus_tx: broadcast::Sender<shared::message::BusMessage>,
        bus_rx: Arc<Mutex<broadcast::Receiver<shared::message::BusMessage>>>,
    ) -> Self {
        Self { router, bus_tx, bus_rx }
    }

    pub fn router(&self) -> &Router { &self.router }
    pub fn bus_tx(&self) -> &broadcast::Sender<shared::message::BusMessage> { &self.bus_tx }
    pub fn bus_rx(&self) -> &Arc<Mutex<broadcast::Receiver<shared::message::BusMessage>>> { &self.bus_rx }
}
```

**Step 4: 编写 config/mod.rs**

```rust
// crab-client/src/config/mod.rs

pub mod remote;
pub mod local;

pub use remote::RemoteClientConfig;
pub use local::LocalClientConfig;
```

**Step 5: 更新 lib.rs**

```rust
// crab-client/src/lib.rs

pub mod config;
pub use config::{RemoteClientConfig, LocalClientConfig};
```

**Step 6: 运行检查**

```bash
cargo check -p crab-client
```

**Step 7: 提交**

```bash
git add crab-client/src/config/
git commit -m "feat: add config module with RemoteClientConfig and LocalClientConfig"
```

---

## Phase 2: 证书管理

### Task 3: 实现 CertificateManager

**Files:**
- Create: `crab-client/src/cert/mod.rs`
- Create: `crab-client/src/cert/manager.rs`
- Create: `crab-client/src/cert/storage.rs`
- Modify: `crab-client/src/lib.rs`
- Modify: `crab-client/Cargo.toml`

**Step 1: 添加依赖到 Cargo.toml**

```toml
# crab-client/Cargo.toml 添加
fs2 = "0.4"  # 文件锁
```

**Step 2: 编写 storage.rs**

```rust
// crab-client/src/cert/storage.rs

use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Certificate not found: {0}")]
    NotFound(String),
    #[error("Invalid certificate: {0}")]
    Invalid(String),
}

pub struct CertStorage {
    base_path: PathBuf,
}

impl CertStorage {
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Self {
        let path = base_path.into().join(client_name);
        Self { base_path: path }
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.base_path)
    }

    pub fn client_cert_path(&self) -> PathBuf { self.base_path.join("client_cert.pem") }
    pub fn client_key_path(&self) -> PathBuf { self.base_path.join("client_key.pem") }
    pub fn tenant_ca_path(&self) -> PathBuf { self.base_path.join("tenant_ca_cert.pem") }

    pub fn load_client_cert(&self) -> Result<Vec<u8>, StorageError> {
        let path = self.client_cert_path();
        if !path.exists() {
            return Err(StorageError::NotFound(path.to_string_lossy().to_string()));
        }
        Ok(fs::read(&path)?)
    }

    pub fn load_client_key(&self) -> Result<Vec<u8>, StorageError> {
        let path = self.client_key_path();
        if !path.exists() {
            return Err(StorageError::NotFound(path.to_string_lossy().to_string()));
        }
        Ok(fs::read(&path)?)
    }

    pub fn load_tenant_ca(&self) -> Result<Vec<u8>, StorageError> {
        let path = self.tenant_ca_path();
        if !path.exists() {
            return Err(StorageError::NotFound(path.to_string_lossy().to_string()));
        }
        Ok(fs::read(&path)?)
    }

    pub fn save_client_cert(&self, data: &[u8]) -> Result<(), StorageError> {
        fs::write(self.client_cert_path(), data)?;
        Ok(())
    }

    pub fn save_client_key(&self, data: &[u8]) -> Result<(), StorageError> {
        fs::write(self.client_key_path(), data)?;
        Ok(())
    }

    pub fn save_tenant_ca(&self, data: &[u8]) -> Result<(), StorageError> {
        fs::write(self.tenant_ca_path(), data)?;
        Ok(())
    }

    pub fn exists(&self) -> bool {
        self.client_cert_path().exists() && self.client_key_path().exists()
    }
}
```

**Step 3: 编写 manager.rs**

```rust
// crab-client/src/cert/manager.rs

use crate::cert::storage::CertStorage;
use crate::cert::storage::StorageError;
use rustls::ClientConfig;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CertError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("TLS config error: {0}")]
    Tls(String),
    #[error("Certificate expired, please login again")]
    Expired,
    #[error("Failed to request certificate: {0}")]
    RequestFailed(String),
}

pub struct CertificateManager {
    storage: CertStorage,
    client_name: String,
}

impl CertificateManager {
    pub fn new(cert_path: impl Into<PathBuf>, client_name: &str) -> Self {
        let storage = CertStorage::new(cert_path, client_name);
        Self {
            storage,
            client_name: client_name.to_string(),
        }
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        self.storage.ensure_dir()
    }

    pub fn exists(&self) -> bool {
        self.storage.exists()
    }

    pub fn load_or_request(&self, auth_url: &str) -> Result<(), CertError> {
        if self.exists() {
            tracing::info!("Certificates found, checking validity...");
            // TODO: 检查证书是否过期
            return Ok(());
        }

        tracing::info!("No certificates found, requesting from auth server...");
        self.request_cert(auth_url).await?;
        Ok(())
    }

    async fn request_cert(&self, auth_url: &str) -> Result<(), CertError> {
        // 1. 从 Auth Server 获取证书
        let client = reqwest::Client::new();
        let device_id = crab_cert::generate_hardware_id();

        let response = client
            .post(format!("{}/api/cert/issue", auth_url))
            .json(&serde_json::json!({
                "client_name": self.client_name,
                "device_id": device_id
            }))
            .send()
            .await
            .map_err(|e| CertError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CertError::RequestFailed(
                response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
            ));
        }

        let cert_data: serde_json::Value = response.json().await.map_err(|e| CertError::RequestFailed(e.to_string()))?;

        let cert_pem = cert_data["cert"].as_str()
            .ok_or_else(|| CertError::RequestFailed("No cert in response".to_string()))?;
        let key_pem = cert_data["key"].as_str()
            .ok_or_else(|| CertError::RequestFailed("No key in response".to_string()))?;
        let tenant_ca_pem = cert_data["tenant_ca_cert"].as_str()
            .ok_or_else(|| CertError::RequestFailed("No tenant CA in response".to_string()))?;

        // 2. 保存到存储
        self.storage.save_client_cert(cert_pem.as_bytes())?;
        self.storage.save_client_key(key_pem.as_bytes())?;
        self.storage.save_tenant_ca(tenant_ca_pem.as_bytes())?;

        tracing::info!("Certificates saved to {}", self.storage.base_path.display());
        Ok(())
    }

    pub fn build_tls_config(&self) -> Result<ClientConfig, CertError> {
        // 读取证书和密钥
        let client_cert = self.storage.load_client_cert()?;
        let client_key = self.storage.load_client_key()?;
        let tenant_ca = self.storage.load_tenant_ca()?;

        // 构建 TLS 配置 (与现有 http.rs 逻辑相同)
        // ... 省略实现细节
        Ok(config)
    }
}
```

**Step 4: 编写 cert/mod.rs**

```rust
// crab-client/src/cert/mod.rs

pub mod storage;
pub mod manager;

pub use storage::{CertStorage, StorageError};
pub use manager::{CertificateManager, CertError};
```

**Step 5: 更新 lib.rs**

```rust
// crab-client/src/lib.rs

pub mod cert;
pub use cert::{CertificateManager, CertError, CertStorage};
```

**Step 6: 运行检查**

```bash
cargo check -p crab-client
```

**Step 7: 提交**

```bash
git add crab-client/src/cert/
git commit -m "feat: add certificate manager with storage"
```

---

## Phase 3: HTTP 客户端

### Task 4: 实现 HttpClient

**Files:**
- Modify: `crab-client/src/client/http.rs`

**Step 1: 重写 http.rs 使用新配置**

```rust
// crab-client/src/client/http.rs

use async_trait::async_trait;
use crate::{ApiResponse, ClientError, ClientResult, CurrentUserResponse, LoginResponse};
use crate::config::RemoteClientConfig;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use rustls::ClientConfig as RustlsClientConfig;

/// HTTP client trait
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<T>;
    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse>;
    async fn me(&self) -> ClientResult<CurrentUserResponse>;
    async fn logout(&mut self) -> ClientResult<()>;
    fn token(&self) -> Option<&str>;
}

/// Network HTTP client implementation
#[derive(Debug, Clone)]
pub struct NetworkHttpClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl NetworkHttpClient {
    pub fn new(config: &RemoteClientConfig, tls_config: Option<Arc<RustlsClientConfig>>) -> Result<Self, ClientError> {
        let mut builder = Client::builder()
            .timeout(config.timeout())
            .user_agent(concat!("crab-client/", env!("CARGO_PKG_VERSION")));

        if let Some(tls_config) = tls_config {
            builder = builder.use_preconfigured_tls(tls_config);
        }

        let client = builder.build()?;
        Ok(Self {
            client,
            base_url: config.edge_url().to_string(),
            token: None,
        })
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> ClientResult<T> {
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await?;
            return match status {
                StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized),
                StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
                StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
                StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
                _ => Err(ClientError::Internal(text)),
            };
        }
        Ok(response.json().await?)
    }
}

#[async_trait]
impl HttpClient for NetworkHttpClient {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.post(&url).json(body);
        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.post(&url);
        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        #[derive(serde::Serialize)]
        struct LoginRequest { username: String, password: String }
        let req = LoginRequest { username: username.to_string(), password: password.to_string() };
        let resp: ApiResponse<LoginResponse> = self.post("/api/auth/login", &req).await?;
        resp.data.ok_or_else(|| ClientError::InvalidResponse("Missing login data".into()))
    }

    async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let resp: ApiResponse<CurrentUserResponse> = self.get("/api/auth/me").await?;
        resp.data.ok_or_else(|| ClientError::InvalidResponse("Missing user data".into()))
    }

    async fn logout(&mut self) -> ClientResult<()> {
        self.post_empty::<ApiResponse<()>>("/api/auth/logout").await?;
        self.token = None;
        Ok(())
    }

    fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}
```

**Step 2: 运行检查**

```bash
cargo check -p crab-client
```

**Step 3: 提交**

```bash
git commit -a -m "feat: implement NetworkHttpClient with RemoteClientConfig"
```

---

### Task 5: 实现 OneshotHttpClient

**Files:**
- Modify: `crab-client/src/client/http.rs`

**Step 1: 添加 OneshotHttpClient**

```rust
// 在 http.rs 中添加

use tower::ServiceExt;
use axum::body::Body;

/// Oneshot HTTP client for in-process communication
#[derive(Debug, Clone)]
pub struct OneshotHttpClient {
    router: Router,
    token: Option<String>,
}

impl OneshotHttpClient {
    pub fn new(router: Router) -> Self {
        Self { router, token: None }
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: http::Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> ClientResult<T> {
        let mut builder = http::Request::builder().method(method).uri(path);
        if let Some(token) = &self.token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }
        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }
        let req = builder.body(Body::from(body.unwrap_or_default()))?;
        let resp = self.router.clone().oneshot(req).await?;
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes).to_string();
            return match status {
                http::StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized),
                http::StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
                http::StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
                http::StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
                _ => Err(ClientError::Internal(text)),
            };
        }
        serde_json::from_slice(&bytes).map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }
}

#[async_trait]
impl HttpClient for OneshotHttpClient {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        self.request(http::Method::GET, path, None).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<T> {
        let body = serde_json::to_vec(body)?;
        self.request(http::Method::POST, path, Some(body)).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        self.request(http::Method::POST, path, None).await
    }

    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        #[derive(serde::Serialize)]
        struct LoginRequest { username: String, password: String }
        let req = LoginRequest { username: username.to_string(), password: password.to_string() };
        let resp: ApiResponse<LoginResponse> = self.post("/api/auth/login", &req).await?;
        resp.data.ok_or_else(|| ClientError::InvalidResponse("Missing login data".into()))
    }

    async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let resp: ApiResponse<CurrentUserResponse> = self.get("/api/auth/me").await?;
        resp.data.ok_or_else(|| ClientError::InvalidResponse("Missing user data".into()))
    }

    async fn logout(&mut self) -> ClientResult<()> {
        self.post_empty::<ApiResponse<()>>("/api/auth/logout").await?;
        self.token = None;
        Ok(())
    }

    fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}
```

**Step 2: 运行检查**

```bash
cargo check -p crab-client
```

**Step 3: 提交**

```bash
git commit -a -m "feat: implement OneshotHttpClient for in-process communication"
```

---

## Phase 4: 消息客户端

### Task 6: 实现 Transport trait

**Files:**
- Modify: `crab-client/src/message/transport.rs`

**Step 1: 重构 Transport trait**

```rust
// crab-client/src/message/transport.rs

use async_trait::async_trait;
use crate::message::MessageError;
use shared::message::BusMessage;

/// Transport abstraction for message bus communication
#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    async fn read_message(&self) -> Result<BusMessage, MessageError>;
    async fn write_message(&self, msg: &BusMessage) -> Result<(), MessageError>;
    async fn close(&self) -> Result<(), MessageError>;
}

// TcpTransport, TlsTransport, MemoryTransport 保持现有实现
```

**Step 2: 运行检查**

```bash
cargo check -p crab-client
```

**Step 3: 提交**

```bash
git commit -a -m "refactor: update Transport trait for unified message client"
```

---

### Task 7: 实现 MessageClient

**Files:**
- Modify: `crab-client/src/client/message.rs`

**Step 1: 编写 NetworkMessageClient**

```rust
// crab-client/src/client/message.rs

use async_trait::async_trait;
use crate::message::{MessageClient as MessageClientTrait, MessageError, Transport};
use shared::message::BusMessage;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, oneshot};
use std::collections::HashMap;
use uuid::Uuid;

/// Network message client (TCP/TLS)
#[derive(Debug)]
pub struct NetworkMessageClient {
    transport: Arc<dyn Transport>,
    event_tx: broadcast::Sender<BusMessage>,
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>>,
}

impl NetworkMessageClient {
    pub async fn connect_tls(
        addr: &str,
        domain: &str,
        tls_config: rustls::ClientConfig,
        client_name: &str,
    ) -> Result<Self, MessageError> {
        let transport = crate::message::TlsTransport::connect(addr, domain, tls_config).await?;
        Self::new(Arc::new(transport)).await
    }

    pub async fn connect(addr: &str, client_name: &str) -> Result<Self, MessageError> {
        let transport = crate::message::TcpTransport::connect(addr).await?;
        Self::new(Arc::new(transport)).await
    }

    async fn new(transport: Arc<dyn Transport>) -> Result<Self, MessageError> {
        let (event_tx, _) = broadcast::channel(1024);
        let pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>> = Arc::new(Mutex::new(HashMap::new()));

        let client = Self {
            transport: transport.clone(),
            event_tx: event_tx.clone(),
            pending_requests: pending_requests.clone(),
        };

        // Spawn background task
        let transport_clone = transport;
        tokio::spawn(async move {
            loop {
                match transport_clone.read_message().await {
                    Ok(msg) => {
                        if let Some(correlation_id) = msg.correlation_id {
                            let mut pending = pending_requests.lock().unwrap();
                            if let Some(tx) = pending.remove(&correlation_id) {
                                let _ = tx.send(msg.clone());
                            }
                        }
                        let _ = event_tx.send(msg);
                    }
                    Err(e) => {
                        tracing::error!("Transport read error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(client)
    }
}

#[async_trait]
impl MessageClientTrait for NetworkMessageClient {
    async fn send(&self, msg: &BusMessage) -> Result<(), MessageError> {
        self.transport.write_message(msg).await
    }

    async fn request(&self, msg: &BusMessage) -> Result<BusMessage, MessageError> {
        let request_id = msg.request_id;
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id, tx);
        }

        if let Err(e) = self.send(msg).await {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.remove(&request_id);
            return Err(e);
        }

        match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(MessageError::Connection("Response channel closed".to_string())),
            Err(_) => {
                let mut pending = self.pending_requests.lock().unwrap();
                pending.remove(&request_id);
                Err(MessageError::Timeout("Request timed out".to_string()))
            }
        }
    }

    async fn recv(&self) -> Result<BusMessage, MessageError> {
        let mut rx = self.event_tx.subscribe();
        rx.recv().await.map_err(|e| MessageError::Connection(format!("Event bus error: {}", e)))
    }
}
```

**Step 2: 编写 MemoryMessageClient**

```rust
// 在 message.rs 中添加

/// Memory message client for in-process communication
#[derive(Debug, Clone)]
pub struct MemoryMessageClient {
    tx: broadcast::Sender<BusMessage>,
    rx: Arc<Mutex<broadcast::Receiver<BusMessage>>>,
}

impl MemoryMessageClient {
    pub fn new(
        tx: &broadcast::Sender<BusMessage>,
        rx: &Arc<Mutex<broadcast::Receiver<BusMessage>>>,
    ) -> Self {
        Self {
            tx: tx.clone(),
            rx: rx.clone(),
        }
    }
}

#[async_trait]
impl MessageClientTrait for MemoryMessageClient {
    async fn send(&self, msg: &BusMessage) -> Result<(), MessageError> {
        self.tx.send(msg.clone())
            .map_err(|e| MessageError::Connection(format!("Failed to send: {}", e)))?;
        Ok(())
    }

    async fn request(&self, msg: &BusMessage) -> Result<BusMessage, MessageError> {
        // 对于内存传输，使用直接调用方式
        self.send(msg).await?;
        // 在内存模式下，request 和 send 行为相同
        Ok(msg.clone())
    }

    async fn recv(&self) -> Result<BusMessage, MessageError> {
        let mut rx = self.rx.lock().await;
        rx.recv().await.map_err(|e| MessageError::Connection(format!("Event bus error: {}", e)))
    }
}
```

**Step 3: 更新 mod.rs 导出**

```rust
// crab-client/src/client/message.rs 顶部添加导出

pub use self::network::{NetworkMessageClient};
pub use self::memory::{MemoryMessageClient};

mod network;
mod memory;
```

**Step 4: 运行检查**

```bash
cargo check -p crab-client
```

**Step 5: 提交**

```bash
git commit -a -m "feat: implement NetworkMessageClient and MemoryMessageClient"
```

---

## Phase 5: CrabClient 主结构

### Task 8: 实现 CrabClient<M>

**Files:**
- Create: `crab-client/src/client/crab_client.rs`
- Modify: `crab-client/src/lib.rs`

**Step 1: 创建 crab_client.rs**

```rust
// crab-client/src/client/crab_client.rs

use std::marker::PhantomData;
use crate::config::{RemoteClientConfig, LocalClientConfig};
use crate::client::{HttpClient, MessageClient};
use crate::client::http::{NetworkHttpClient, OneshotHttpClient};
use crate::client::message::{NetworkMessageClient, MemoryMessageClient};
use crate::CertError;

/// Remote mode marker
pub struct RemoteMode;

/// Local mode marker
pub struct LocalMode;

/// Unified CrabClient with mode support
#[derive(Debug, Clone)]
pub struct CrabClient<M> {
    _mode: PhantomData<M>,
    http: Box<dyn HttpClient>,
    message: Box<dyn MessageClient>,
    token: Option<String>,
}

impl<M> CrabClient<M> {
    /// Get HTTP sub-client
    pub fn http(&self) -> &dyn HttpClient {
        self.http.as_ref()
    }

    /// Get mutable HTTP sub-client
    pub fn http_mut(&mut self) -> &mut Box<dyn HttpClient> {
        &mut self.http
    }

    /// Get message sub-client
    pub fn message(&mut self) -> &mut dyn MessageClient {
        self.message.as_mut()
    }

    /// Get current token
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Set token
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }
}

impl CrabClient<RemoteMode> {
    /// Create remote client
    pub async fn new(config: RemoteClientConfig) -> Result<Self, CertError> {
        // 1. 初始化证书目录
        let cert_path = config.cert_path().clone();
        std::fs::create_dir_all(&cert_path).ok();

        // 2. 证书管理
        let cert_manager = crate::CertificateManager::new(&cert_path, config.client_name());
        cert_manager.load_or_request(config.auth_url()).await?;

        // 3. 构建 TLS 配置
        let tls_config = cert_manager.build_tls_config()?;

        // 4. 创建 HTTP 客户端
        let http = Box::new(NetworkHttpClient::new(&config, Some(tls_config.clone()))?);

        // 5. 创建消息客户端
        let message = Box::new(
            NetworkMessageClient::connect_tls(
                config.tcp_addr(),
                "localhost",
                tls_config,
                config.client_name(),
            ).await?
        );

        Ok(Self {
            _mode: PhantomData,
            http,
            message,
            token: None,
        })
    }
}

impl CrabClient<LocalMode> {
    /// Create local (in-process) client
    pub fn new(config: LocalClientConfig) -> Self {
        let http = Box::new(OneshotHttpClient::new(config.router().clone()));
        let message = Box::new(MemoryMessageClient::new(config.bus_tx(), config.bus_rx()));

        Self {
            _mode: PhantomData,
            http,
            message,
            token: None,
        }
    }
}
```

**Step 2: 更新 client/mod.rs**

```rust
// crab-client/src/client/mod.rs

pub mod http;
pub mod message;
pub mod crab_client;

pub use http::{HttpClient, NetworkHttpClient, OneshotHttpClient};
pub use message::{MessageClient, NetworkMessageClient, MemoryMessageClient};
pub use crab_client::{CrabClient, RemoteMode, LocalMode};
```

**Step 3: 更新 lib.rs**

```rust
// crab-client/src/lib.rs

pub use client::{CrabClient, RemoteMode, LocalMode};
```

**Step 4: 运行检查**

```bash
cargo check -p crab-client
```

**Step 5: 提交**

```bash
git commit -a -m "feat: implement generic CrabClient<M> with RemoteMode and LocalMode"
```

---

### Task 9: 实现登录方法

**Files:**
- Modify: `crab-client/src/client/crab_client.rs`

**Step 1: 添加登录方法**

```rust
// 在 CrabClient<M> impl 块中添加

impl<M> CrabClient<M> {
    /// Login with employee credentials
    pub async fn login(&mut self, employee_id: &str, password: &str) -> Result<(), crate::ClientError> {
        let resp = self.http.login(employee_id, password).await?;
        self.token = Some(resp.token);
        Ok(())
    }

    /// Get current user info
    pub async fn me(&self) -> Result<(), crate::ClientError> {
        self.http.me().await?;
        Ok(())
    }

    /// Logout
    pub async fn logout(&mut self) -> Result<(), crate::ClientError> {
        self.http.logout().await?;
        self.token = None;
        Ok(())
    }
}
```

**Step 2: 运行检查**

```bash
cargo check -p crab-client
```

**Step 3: 提交**

```bash
git commit -a -m "feat: add login/me/logout methods to CrabClient"
```

---

## Phase 6: 错误处理和测试

### Task 10: 统一错误类型

**Files:**
- Modify: `crab-client/src/error.rs`

**Step 1: 更新错误类型**

```rust
// crab-client/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Certificate error: {0}")]
    Certificate(#[from] crate::CertError),

    #[error("HTTP request failed: {0}")]
    HttpRequest(String),

    #[error("Message error: {0}")]
    Message(#[from] crate::message::MessageError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError::HttpRequest(e.to_string())
    }
}
```

**Step 2: 运行检查**

```bash
cargo check -p crab-client
```

**Step 3: 提交**

```bash
git commit -a -m "feat: unify error types in ClientError"
```

---

### Task 11: 集成测试

**Files:**
- Create: `crab-client/tests/client_integration.rs`

**Step 1: 编写集成测试**

```rust
// crab-client/tests/client_integration.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crab_client::{CrabClient, LocalClientConfig, RemoteClientConfig};
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_local_client_creation() {
        let (tx, _) = broadcast::channel(100);
        let rx = std::sync::Arc::new(tokio::sync::Mutex::new(tx.subscribe()));

        // 创建空 Router
        let router = axum::Router::new();

        let config = LocalClientConfig::new(router, tx, rx);
        let client = CrabClient::<LocalMode>::new(config);

        assert!(client.token().is_none());
    }

    #[tokio::test]
    async fn test_remote_config_builder() {
        let config = RemoteClientConfig::new()
            .with_auth_url("http://localhost:3001")
            .with_edge_url("http://localhost:8080")
            .with_tcp_addr("localhost:8081")
            .with_cert_path("/tmp/test-certs")
            .with_client_name("test-client")
            .with_timeout(30);

        assert_eq!(config.auth_url(), "http://localhost:3001");
        assert_eq!(config.edge_url(), "http://localhost:8080");
        assert_eq!(config.tcp_addr(), "localhost:8081");
        assert_eq!(config.client_name(), "test-client");
    }
}
```

**Step 2: 运行测试**

```bash
cargo test -p crab-client --test client_integration
```

**Step 3: 提交**

```bash
git commit -a -m "test: add integration tests for CrabClient"
```

---

## Phase 7: 示例和文档

### Task 12: 创建示例

**Files:**
- Create: `crab-client/examples/remote_client.rs`
- Create: `crab-client/examples/local_client.rs`

**Step 1: 远程客户端示例**

```rust
// crab-client/examples/remote_client.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RemoteClientConfig::new()
        .with_auth_url("http://localhost:3001")
        .with_edge_url("http://localhost:8080")
        .with_tcp_addr("localhost:8081")
        .with_cert_path("./certs")
        .with_client_name("my-client");

    let mut client = CrabClient::<RemoteMode>::new(config).await?;
    client.login("emp001", "password").await?;

    let me = client.http().me().await?;
    println!("Logged in as: {:?}", me);

    let mut msg = client.message();
    msg.send(&notification).await?;

    Ok(())
}
```

**Step 2: 本地客户端示例**

```rust
// crab-client/examples/local_client.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel(100);
    let rx = std::sync::Arc::new(tokio::sync::Mutex::new(tx.subscribe()));
    let router = axum::Router::new();

    let config = LocalClientConfig::new(router, tx, rx);
    let mut client = CrabClient::<LocalMode>::new(config);

    // 同进程调用...
    client.login("emp001", "password").await?;

    Ok(())
}
```

**Step 3: 提交**

```bash
git commit -a -m "docs remote and: add examples for local client usage"
```

---

## 总结

**实现顺序：**
1. Task 1-2: 基础结构 (模块和配置)
2. Task 3: 证书管理
3. Task 4-5: HTTP 客户端 (Network + Oneshot)
4. Task 6-7: 消息客户端 (Network + Memory)
5. Task 8-9: CrabClient 主结构
6. Task 10-11: 错误处理和测试
7. Task 12: 示例和文档

**预计文件变更：**
- 新增: 15+ 文件
- 修改: 5 文件
- 删除: 0 文件
