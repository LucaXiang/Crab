//! Local Client Example - 使用 CrabClient 在同进程内通信
//!
//! 这个示例展示如何在 Edge Server 内部使用 CrabClient 进行:
//! 1. HTTP API 调用 (通过 Tower oneshot，零网络开销)
//! 2. Message Bus RPC (通过 broadcast channel，零网络开销)
//! 3. 订阅服务器广播消息 (Notifications, Sync 等)
//!
//! 适用场景:
//! - 服务器内部测试
//! - 同进程的客户端逻辑
//! - 集成测试
//!
//! 运行: cargo run -p edge-server --example local_client

use crab_client::{Authenticated, BusMessage, CrabClient, Local};
use edge_server::core::{Config, ServerState};
use shared::message::EventType;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Local Client Example ===\n");

    // === 1. 初始化 ServerState ===
    println!("1. Initializing ServerState...");

    // 使用临时目录避免权限问题
    let temp_dir = std::env::temp_dir().join("crab-local-example");
    std::fs::create_dir_all(&temp_dir)?;

    let config = Config::with_overrides(temp_dir.to_string_lossy(), 0, 0);
    let state = ServerState::initialize(&config).await?;

    // 启动后台任务 (MessageHandler 等)
    state.start_background_tasks().await;
    println!("   ServerState initialized.\n");

    // === 2. 获取 Router 和 Message Channels ===
    println!("2. Getting Router and Message Channels...");

    // 从 HttpsService 获取已初始化的 Router
    let router = state
        .https_service()
        .router()
        .expect("HttpsService not initialized");

    // 从 MessageBus 获取双向通道
    let client_tx = state.message_bus().sender_to_server().clone(); // 客户端→服务器
    let server_tx = state.message_bus().sender().clone(); // 服务器→客户端

    println!("   Router and Channels ready.\n");

    // === 3. 创建 Local Client ===
    println!("3. Creating Local Client...");
    let client = CrabClient::local()
        .with_router(router)
        .with_message_channels(client_tx, server_tx)
        .build()?;

    println!("   Client built. is_connected={}", client.is_connected());

    // === 4. 订阅服务器广播 (在连接之前也可以订阅) ===
    println!("\n4. Subscribing to server broadcasts...");
    let mut broadcast_rx = client.subscribe()?;

    // 启动后台任务处理广播消息
    tokio::spawn(async move {
        println!("   [Subscriber] Listening for broadcasts...");
        while let Ok(msg) = broadcast_rx.recv().await {
            match msg.event_type {
                EventType::Notification => {
                    println!("   [Subscriber] Got Notification: {:?}", msg.request_id);
                }
                EventType::Sync => {
                    println!("   [Subscriber] Got Sync signal");
                }
                EventType::Response => {
                    println!("   [Subscriber] Got Response for: {:?}", msg.correlation_id);
                }
                _ => {}
            }
        }
        println!("   [Subscriber] Channel closed");
    });

    // === 5. 连接 (typestate 转换) ===
    println!("\n5. Connecting...");
    let client = client.connect().await?;
    println!("   Connected. is_connected={}", client.is_connected());

    // === 6. 演示 HTTP 调用 (健康检查不需要认证) ===
    println!("\n6. Making HTTP request (health check)...");

    // 直接使用 HttpsService oneshot 调用
    let http = state.https_service();
    let request = http::Request::builder()
        .method(http::Method::GET)
        .uri("/health")
        .body(axum::body::Body::empty())?;

    match http.oneshot(request).await {
        Ok(response) => println!("   Health check: {:?}", response.status()),
        Err(e) => println!("   Health check failed: {:?}", e),
    }

    // === 7. 演示 Message Bus (服务器广播) ===
    println!("\n7. Testing server broadcast...");

    // 服务器发送一条通知 (模拟)
    let notification = BusMessage::notification(&shared::message::NotificationPayload {
        level: shared::message::NotificationLevel::Info,
        category: shared::message::NotificationCategory::System,
        title: "Test".to_string(),
        message: "Hello from server!".to_string(),
        data: None,
    });

    // 通过 server_tx 发送广播
    state.message_bus().sender().send(notification)?;

    // 等待订阅者处理
    tokio::time::sleep(Duration::from_millis(100)).await;

    // === 8. 清理 ===
    println!("\n8. Cleanup...");
    let _client = client.disconnect();
    println!("   Disconnected.\n");

    println!("=== Example Complete ===");
    println!("\nKey points:");
    println!("  - Local mode uses Tower oneshot for HTTP (zero network)");
    println!("  - Local mode uses broadcast channel for RPC (zero network)");
    println!("  - Client can subscribe to server broadcasts");
    println!("  - Perfect for in-process testing and embedded scenarios");

    Ok(())
}

/// 完整的认证流程示例 (需要数据库中有用户)
#[allow(dead_code)]
async fn authenticated_example(
    client: CrabClient<Local, Authenticated>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 订阅服务器广播
    let mut rx = client.subscribe()?;
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            println!("Broadcast: {:?}", msg.event_type);
        }
    });

    // HTTP GET 请求
    println!("Making authenticated GET request...");
    // let orders: Vec<serde_json::Value> = client.get("/api/orders").await?;

    // HTTP POST 请求
    println!("Making authenticated POST request...");
    // let new_order: serde_json::Value = client.post("/api/orders", &body).await?;

    // Message Bus RPC 请求
    println!("Making RPC request...");
    let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
        action: "ping".to_string(),
        params: None,
    });

    let response = client.request(&request).await?;
    println!("RPC response: {:?}", response.event_type);

    // 带超时的 RPC 请求
    let response = client
        .request_with_timeout(&request, Duration::from_secs(5))
        .await?;
    println!("RPC response (with timeout): {:?}", response.event_type);

    // 登出
    let client = client.logout().await;
    println!("Logged out. is_authenticated={}", client.is_authenticated());

    // 断开连接
    let _client = client.disconnect();
    println!("Disconnected.");

    Ok(())
}
