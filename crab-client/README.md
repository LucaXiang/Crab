# Crab Client

A unified client library for the Crab Edge Server with support for both HTTP and Message Bus communication.

## Features

- **HTTP Client**: Network-based HTTP calls to the Edge Server API
- **Message Client**: Subscribe and publish messages via:
  - **TCP Transport**: Network-based message communication
  - **Memory Transport**: Zero-copy in-process communication

## Quick Start

### HTTP Client

```rust
use crab_client::{HttpClient, ClientConfig, ApiResponse};

let config = ClientConfig {
    base_url: "http://localhost:3000".to_string(),
    timeout: std::time::Duration::from_secs(5),
};

let client = HttpClient::new(config);
let response = client.get::<ApiResponse<Health>>("/health").await?;
```

### Message Client - TCP Mode

```rust
use crab_client::{MessageClient, BusMessage, tcp};

let addr = "127.0.0.1:8081";
let client = tcp(addr, "").await?;

// Send a message
let msg = BusMessage::notification("Test", "Hello!");
client.send(&msg).await?;

// Receive messages
loop {
    let msg = client.recv().await?;
    println!("Received: {:?}", msg);
}
```

### Message Client - Memory Mode

```rust
use crab_client::{MessageClient, BusMessage, oneshot};
use tokio::sync::broadcast;

// Get receiver from ServerState (in edge-server)
let receiver = state.message_bus().subscribe();
let client = oneshot(receiver);

// Receive messages
loop {
    let msg = client.recv().await?;
    println!("Received: {:?}", msg);
}

// Send messages (via HTTP API)
let request = Request::builder()
    .uri("/api/message/emit?type=notification&title=Test&body=Hello")
    .method("GET")
    .body(String::new().into())?;
state.oneshot(request).await?;
```

## Examples

### Basic Message Client
```bash
# TCP mode
cargo run --example message_client tcp 127.0.0.1:8081

# Memory mode
cargo run --example message_client mem
```

### Full Message Example
```bash
# TCP mode with send/receive
cargo run --example full_message_example tcp 127.0.0.1:8081

# Memory mode with broadcast channel demo
cargo run --example full_message_example memory
```

### Edge Server Examples

#### TCP Subscriber
```bash
# Start edge server
cargo run --bin edge-server

# In another terminal - subscribe via TCP
cargo run --example message_subscriber
```

#### Memory Subscriber
```bash
# Subscribe in-process (no network)
cargo run --example oneshot_subscriber
```

## Architecture

```
┌─────────────────────────────────────┐
│        crab-client                  │
│  ┌─────────────────────────────┐  │
│  │  MessageClient (trait)       │  │
│  └────────────┬──────────────────┘  │
│               │                      │
│   ┌───────────┴───────────┐         │
│   │                       │         │
│   ▼                       ▼         │
│ ┌─────────┐         ┌──────────┐   │
│ │   TCP   │         │ Oneshot  │   │
│ │Client   │         │Client    │   │
│ └─────────┘         └──────────┘   │
│                                   │
│  - 网络通信                       │
│  - 跨进程通信                     │
│                                   │
└───────────────────────────────────┘
```

## Message Types

The library supports 5 event types for restaurant POS operations:

1. **TableIntent** - Client → Server (点菜、付款、结账请求)
2. **TableSync** - Server → All Clients (桌台状态广播)
3. **DataSync** - Server → All Clients (菜品数据更新)
4. **Notification** - Server → All Clients (系统通知)
5. **ServerCommand** - Central Server → Edge Server (服务器指令)

## Usage Patterns

### Same-Process (Edge Server + Client)

```rust
// Create server state
let state = ServerState::new(&config).await;

// Client receives via OneshotMessageClient
let receiver = state.message_bus().subscribe();
let client = oneshot(receiver);

// Client publishes via HTTP API
let request = Request::builder()
    .uri("/api/message/emit?type=notification&title=Test&body=Hello")
    .method("GET")
    .body(String::new().into())?;
state.oneshot(request).await?;
```

### Cross-Process (Separate Client + Server)

```rust
// Client connects via TCP
let client = tcp("127.0.0.1:8081", "").await?;

// Send message via TCP
let msg = BusMessage::table_intent("add_dish", serde_json::json!({
    "table_id": "T01",
    "dish": "宫保鸡丁"
}));
client.send(&msg).await?;

// Receive message via TCP
let msg = client.recv().await?;
```

## Configuration

### Environment Variables

```bash
# Edge Server
WORK_DIR=./data
HTTP_PORT=3000
MESSAGE_TCP_PORT=8081
JWT_SECRET=your-secret
```

## License

MIT
