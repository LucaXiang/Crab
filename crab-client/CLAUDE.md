# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Crab Client

统一客户端库 — 支持 Remote (mTLS) 和 Local (In-Process) 双模式，Typestate 状态安全。

## 命令

```bash
cargo check -p crab-client
cargo test -p crab-client --lib
cargo run -p crab-client --example message_client
```

## 模块结构

```
src/
├── client/         # 客户端实现
│   ├── http/           # HTTP 客户端
│   │   ├── network.rs  # 网络 HTTP (mTLS)
│   │   └── oneshot.rs  # 进程内 HTTP (Tower oneshot)
│   └── message/        # 消息客户端
│       ├── network.rs  # 网络消息 (TCP/TLS + 心跳 + 自动重连)
│       └── in_memory.rs # 进程内消息 (broadcast channel)
├── cert/           # 证书管理 (CertManager)
├── message/        # 消息协议
├── types.rs        # Typestate 类型标记 (Remote/Local, Connected/Authenticated)
└── error.rs        # 错误类型
```

## Typestate 模式

```rust
CrabClient<Remote, Disconnected>  // 初始状态
    .setup()  →  CrabClient<Remote, Connected>     // mTLS 连接
    .login()  →  CrabClient<Remote, Authenticated> // 员工认证

CrabClient<Local, Disconnected>   // 本地模式
    .connect()  →  CrabClient<Local, Connected>    // 进程内连接
    .login()    →  CrabClient<Local, Authenticated>
```

## 双模式

- **Remote**: mTLS 连接远程 edge-server，TCP/TLS 消息通道
- **Local** (`in-process` feature): Tower oneshot HTTP + broadcast channel 消息

## 核心功能

### 心跳 & 自动重连

- 网络消息客户端内置心跳机制
- 连接断开时自动重连
- 重连后自动恢复订阅

### 凭据管理

- CertManager: 证书 + 私钥的本地缓存和加载
- 双重 Token: Access Token (短期) + Refresh Token (长期)
- 会话缓存: 支持离线场景下的凭据恢复

### 使用示例

```rust
// Remote 模式
let client = CrabClient::remote()
    .auth_server("https://auth.example.com")
    .cert_path("./certs")
    .client_name("pos-01")
    .build()?
    .setup("tenant", "pass", "edge:8081").await?
    .login("cashier", "1234").await?;

// Local 模式
let client = CrabClient::local()
    .with_router(router)
    .with_message_channels(client_tx, server_tx)
    .build()?
    .connect().await?
    .login("waiter", "1234").await?;
```

## 响应语言

使用中文回答。
