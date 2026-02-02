//! Remote Message Client Example - 使用 CrabClient 进行 RPC 调用
//!
//! 认证说明：
//! - mTLS (租户证书): 用于 Message Bus RPC 通信，setup/connect_with_credentials 后即可使用
//! - Employee Token: 用于 HTTP API 请求，需要 login 获取
//!
//! 使用流程：
//! 1. 首次运行: client.setup(username, password, addr) - 租户登录，下载证书
//! 2. 后续运行: client.connect_with_credentials(addr) - 使用缓存证书直接连接
//!    - 自动执行自检 (证书链验证、硬件绑定、时钟篡改检测)
//!    - 尝试刷新时间戳 (调用 Auth Server，Tenant CA 签名)
//! 3. RPC 通信: 连接后即可发送 RPC (不需要登录!)
//! 4. HTTP API: client.login(emp_user, emp_pass) - 获取员工 token
//!
//! 安全特性：
//! - mTLS 双向认证
//! - 硬件 ID 绑定 (防止证书拷贝)
//! - 时钟篡改检测 (回拨 > 1h 或前进 > 30d 触发告警)
//! - 时间戳由 Auth Server 使用 Tenant CA 签名 (防止本地伪造)
//!
//! 运行前请确保：
//! 1. 启动 Auth Server: cargo run -p crab-auth
//! 2. 启动 Edge Server: cargo run -p edge-server
//!
//! 运行: cargo run -p crab-client --example remote_message

use crab_client::{BusMessage, CrabClient, NetworkMessageClient};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // === 1. 创建客户端 ===
    let client = CrabClient::remote()
        .auth_server("http://127.0.0.1:3001") // Auth Server URL
        .cert_path("./certs") // 证书存储路径
        .client_name("remote-client") // 客户端名称
        .build()?;

    // === 2. 连接消息服务器 ===
    let client = if client.has_cached_credentials() {
        // 使用缓存证书直接连接
        // connect_with_credentials() 会自动：
        // 1. 执行自检 (证书链验证、硬件绑定、时钟篡改检测)
        // 2. 刷新时间戳 (调用 Auth Server，Tenant CA 签名)
        println!("Using cached certificates (with self-check & timestamp refresh)...");
        client.connect_with_credentials("127.0.0.1:8081").await?
    } else {
        // 首次运行，需要 setup
        println!("First-time setup...");
        client
            .setup(
                "admin",          // 租户用户名
                "password",       // 租户密码
                "127.0.0.1:8081", // Edge Server TCP/mTLS 地址
            )
            .await?
    };

    println!(
        "Connected: {}",
        if client.is_connected() { "yes" } else { "no" }
    );

    // === 3. RPC 调用 (只需 mTLS 连接，不需要登录!) ===
    println!("\n=== RPC 通信 (不需要登录) ===");

    let mc = client.message_client().expect("Not connected");

    println!("Sending ping request...");
    let response = send_rpc(mc, "ping", None).await?;
    println!("Response: {}", response.message);

    println!("\nSending status request...");
    let response = send_rpc(mc, "status", None).await?;
    println!("Response: {}", response.message);

    // === 4. 员工登录 (用于 HTTP API) ===
    println!("\n=== HTTP API (需要登录) ===");
    println!("Employee login...");
    let client = client.login("employee", "emp_password").await.map_err(|(e, _)| e)?;
    println!(
        "Token: {}...",
        client
            .token()
            .unwrap_or("")
            .chars()
            .take(20)
            .collect::<String>()
    );

    // 登录后仍然可以发 RPC
    let mc = client.message_client().expect("Not connected");
    println!("\nSending echo request (still works after login)...");
    let response = send_rpc(mc, "echo", Some(serde_json::json!({"message": "Hello!"}))).await?;
    println!("Response: {}", response.message);

    // === 5. 登出 ===
    let client = client.logout().await;
    println!("\nLogged out (certificates cached for next time)");

    // Optionally disconnect completely
    let _client = client.disconnect().await;

    Ok(())
}

async fn send_rpc(
    mc: &NetworkMessageClient,
    action: &str,
    params: Option<serde_json::Value>,
) -> Result<shared::message::ResponsePayload, Box<dyn std::error::Error>> {
    let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
        action: action.to_string(),
        params,
    });
    let response = mc.request(&request, Duration::from_secs(5)).await?;
    Ok(response.parse_payload()?)
}
