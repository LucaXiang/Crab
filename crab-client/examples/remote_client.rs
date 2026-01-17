// crab-client/examples/remote_client.rs
// 远程客户端示例

use crab_client::{CrabClient, RemoteMode, CertManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: {} <auth_url> <username> <password>", args[0]);
        println!("  Example: {} http://localhost:3001 employee1 password123", args[0]);
        return Ok(());
    }

    let auth_url = &args[1];
    let username = &args[2];
    let password = &args[3];

    // 创建证书管理器
    let cert_path = std::env::var("CRAB_CERT_PATH")
        .unwrap_or_else(|_| "./certs".to_string());
    let client_name = std::env::var("CRAB_CLIENT_NAME")
        .unwrap_or_else(|_| "remote-client".to_string());

    let cert_manager = CertManager::new(&cert_path, &client_name);

    // 尝试加载已有凭证，否则登录
    let credential = match cert_manager.load_or_login(auth_url, username, password).await {
        Ok(cred) => cred,
        Err(e) => {
            tracing::error!("Failed to login: {}", e);
            return Err(e.into());
        }
    };

    tracing::info!("Logged in as: {}", credential.client_name);
    tracing::info!("Token: {}...", &credential.token[..std::cmp::min(20, credential.token.len())]);

    // 创建客户端
    let edge_url = std::env::var("CRAB_EDGE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut client = CrabClient::<RemoteMode>::new(&edge_url);
    client.set_token(credential.token);

    // 获取当前用户
    match client.me().await {
        Ok(user) => tracing::info!("Current user: {:?}", user),
        Err(e) => tracing::error!("Failed to get user: {}", e),
    }

    Ok(())
}
