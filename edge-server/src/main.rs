//! Crab Edge Server 二进制入口
//!
//! 此文件负责:
//! - 加载 .env 配置文件
//! - 设置工作目录
//! - 初始化日志系统
//! - 启动服务器

use edge_server::{
    Config, Server, ServerState, cleanup_old_logs, init_logger_with_file, print_banner,
};
use std::path::PathBuf;

/// 设置运行环境 (仅 bin 使用)
///
/// - 加载 .env 文件
/// - 创建必要的目录结构
/// - 初始化日志系统
fn setup_environment() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 加载 .env 文件 (仅 bin 层面支持)
    dotenv::dotenv().ok();

    // 获取工作目录
    let work_dir = std::env::var("WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    // 创建工作目录
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir)?;
        println!("Created work directory: {}", work_dir.display());
    }

    // 切换到工作目录
    std::env::set_current_dir(&work_dir)?;

    // 创建日志目录
    let log_dir = work_dir.join("logs");
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)?;
    }

    // 创建证书目录
    let certs_dir = work_dir.join("certs");
    if !certs_dir.exists() {
        std::fs::create_dir_all(&certs_dir)?;
        println!("Created certs directory: {}", certs_dir.display());
    }

    // 初始化日志
    let json_format = std::env::var("LOG_JSON")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    init_logger_with_file(
        Some(&log_level),
        Some(json_format),
        Some(log_dir.to_str().unwrap_or("logs")),
    );

    // 清理旧日志 (忽略错误)
    let _ = cleanup_old_logs(log_dir.to_str().unwrap_or("logs"), 7);

    tracing::info!(
        "Environment initialized. WorkDir: {}, LogLevel: {}",
        work_dir.display(),
        log_level
    );

    Ok(work_dir)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 设置环境 (dotenv, 工作目录, 日志)
    let work_dir = setup_environment()?;

    // 打印横幅
    print_banner();

    tracing::info!("🦀 Crab Edge Server starting...");
    tracing::debug!("Work directory: {}", work_dir.display());

    // 2. 加载配置 (从环境变量)
    let config = Config::from_env();

    // 3. 初始化服务器状态
    let state = ServerState::initialize(&config).await;

    // 4. 启动 HTTP 服务器 (Server::run 会自动启动后台任务)
    let server = Server::with_state(config, state);
    let token = server.shutdown_token();

    // ctrl_c 和 server.run() 并行，任一结束则退出
    let result = tokio::select! {
        r = server.run() => r.map_err(|e| e.into()),
        _ = tokio::signal::ctrl_c() => {
            token.cancel();
            Ok(())
        }
    };

    if let Err(e) = &result {
        tracing::error!("Server error: {}", e);
    }

    result
}
