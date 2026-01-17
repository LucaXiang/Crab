//! Remote Message Client Example - 使用 CrabClient 进行 RPC 调用
//!
//! Token 说明：
//! - Auth Server Token: 租户认证，用于下载证书（setup 时获取）
//! - Employee Token: 员工认证，用于 HTTP API（login 时获取）
//!
//! 使用流程：
//! 1. 首次运行: client.setup(username, password, addr) - 租户登录，下载证书
//! 2. 后续运行: client.connect(addr) - 使用缓存证书直接连接
//! 3. 员工操作: client.login(emp_user, emp_pass) - 获取员工 token
//!
//! 运行前请确保：
//! 1. 启动 Auth Server: cargo run -p crab-auth
//! 2. 启动 Edge Server: cargo run -p edge-server
//!
//! 运行: cargo run -p crab-client --example remote_message

use crab_client::{CrabClient, RemoteMode, BusMessage};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // === 1. 创建客户端 ===
    // 参数: Auth URL, 证书存储路径, 客户端名称
    let mut client = CrabClient::<RemoteMode>::new(
        "http://127.0.0.1:3001",  // Auth Server HTTPS
        "./certs",                 // 证书存储路径
        "remote-client",           // 客户端名称
    );

    // === 2. 连接消息服务器 ===
    // 如果是首次运行，需要先用 setup() 设置一次
    // 后续运行可直接使用 connect()
    if !client.is_connected() {
        println!("🔐 首次连接，设置中...");

        // 首次运行时调用 setup()，之后只需 connect()
        client.setup(
            "admin",                  // 租户用户名
            "password",               // 租户密码
            "127.0.0.1:8081",         // Edge Server TCP/mTLS 地址
        ).await?;

        println!("✅ 首次设置完成！凭据和证书已缓存。");
        println!("   下次运行可直接连接，无需重新登录。");
    } else {
        // 直接使用缓存的证书连接（无需密码）
        client.connect("127.0.0.1:8081").await?;
        println!("✅ 已使用缓存的证书连接消息服务器！");
    }

    println!("   连接状态: {}", if client.is_connected() { "已连接" } else { "断开" });

    // === 3. 员工登录 (可选，用于 HTTP API) ===
    println!("\n👤 员工登录...");
    let _login = client.login("employee", "emp_password").await?;
    println!("   Token: {}...", client.token().unwrap_or("").chars().take(20).collect::<String>());

    // === 4. RPC 调用 ===
    println!("\n📤 发送 ping 请求...");
    let response = send_ping(&client).await?;
    println!("   响应: {}", response.message);

    println!("\n📤 发送 status 请求...");
    let response = send_status(&client).await?;
    println!("   响应: {}", response.message);

    // === 5. 登出 ===
    // 只清理员工 token，证书和凭据保留缓存
    client.logout().await;
    println!("\n👋 已登出 (证书已缓存，下次可直接连接)");

    Ok(())
}

async fn send_ping(client: &CrabClient<RemoteMode>) -> Result<shared::message::ResponsePayload, crab_client::MessageError> {
    let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
        action: "ping".to_string(),
        params: None,
    });
    let response = client.request(&request).await?;
    Ok(response.parse_payload()?)
}

async fn send_status(client: &CrabClient<RemoteMode>) -> Result<shared::message::ResponsePayload, crab_client::MessageError> {
    let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
        action: "status".to_string(),
        params: None,
    });
    let response = client.request_with_timeout(&request, Duration::from_secs(3)).await?;
    Ok(response.parse_payload()?)
}
