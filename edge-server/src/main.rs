use edge_server::{Config, Server, ServerState, print_banner, setup_environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 设置环境 (dotenv, 工作目录, 日志)
    setup_environment()?;

    // 打印横幅
    print_banner();

    tracing::info!("🦀 Crab Edge Server starting...");

    // 2. 加载配置
    let config = Config::from_env();

    // 3. 初始化服务器状态
    let state = ServerState::initialize(&config).await;

    // 4. 启动 HTTP 服务器 (Server::run 会自动启动后台任务)
    let server = Server::with_state(config, state);

    if let Err(e) = server.run().await {
        tracing::error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
