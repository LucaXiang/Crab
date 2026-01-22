# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Crab Client

统一客户端库 - 支持 Remote (mTLS) 和 Local (In-Process) 双模式。

## 命令

```bash
cargo check -p crab-client
cargo test -p crab-client --lib
cargo run -p crab-client --example message_client
```

## 模块结构

```
src/
├── client/     # 客户端实现
│   ├── http/       # HTTP 客户端 (Network/Oneshot)
│   └── message/    # 消息客户端 (Network/InMemory)
├── cert/       # 证书管理
├── message/    # 消息协议
├── types.rs    # Typestate 类型标记
└── error.rs    # 错误类型
```

## Typestate 模式

```rust
CrabClient<Remote, Disconnected>  // 初始状态
    .setup()  →  CrabClient<Remote, Connected>
    .login()  →  CrabClient<Remote, Authenticated>

CrabClient<Local, Disconnected>   // 本地模式
    .connect()  →  CrabClient<Local, Connected>
    .login()    →  CrabClient<Local, Authenticated>
```

### 双模式

- **Remote**: mTLS 连接远程 edge-server
- **Local** (`in-process` feature): Tower oneshot + broadcast channel

## 使用示例

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
    .with_message_sender(sender)
    .build()?
    .connect().await?
    .login("waiter", "1234").await?;
```

## 响应语言

使用中文回答。
