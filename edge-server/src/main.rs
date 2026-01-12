use edge_server::server::ServerState;
use edge_server::{Config, Server, common::init_logger_with_file};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get work directory from env or use current directory
    let work_dir = std::env::var("WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    // Create work directory if it doesn't exist
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir).expect("Failed to create work directory");
        tracing::info!("Created work directory: {}", work_dir.display());
    }

    // Change to work directory so relative paths work (uploads, database, etc.)
    std::env::set_current_dir(&work_dir).expect("Failed to change to work directory");

    // Create logs directory path
    let log_dir = work_dir.join("logs");

    // Initialize logging system with file output
    let json_format = std::env::var("LOG_JSON")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    init_logger_with_file(&log_level, json_format, Some(log_dir.to_str().unwrap()))
        .expect("Failed to initialize logger");

    tracing::info!("🦀 Crab Edge Server starting...");
    tracing::info!(
        "Log level: {}, Format: {}, File: {}",
        log_level,
        if json_format { "JSON" } else { "Pretty" },
        log_dir.display()
    );

    // Load configuration
    let config = Config::from_env();

    // 创建 ServerState（可以在 HTTP 服务器和 oneshot 之间共享）
    let state = ServerState::initialize(&config).await;
    let state_arc: Arc<ServerState> = Arc::new(state);

    // 使用 with_state 让 HTTP 服务器复用同一个状态
    let config_for_http = config.clone();
    let state_for_http = (*state_arc).clone();

    // 在后台任务中启动 HTTP 服务器
    let http_server_handle = tokio::spawn(async move {
        let server = Server::with_state(config_for_http, state_for_http);
        if let Err(e) = server.run().await {
            tracing::error!("HTTP server error: {}", e);
        }
    });

    tracing::info!("🚀 HTTP 服务器已在后台启动");
    tracing::info!("💡 现在可以同时使用 HTTP 接口和 oneshot 调用");

    // 主线程可以继续执行其他任务...
    // 示例：演示 oneshot 调用（可选）
    // use http::Request;
    // let request = Request::builder()
    //     .uri("/health")
    //     .method("GET")
    //     .body(String::new().into())
    //     .unwrap();
    // let response = state_arc.oneshot(request).await.unwrap();
    // tracing::info!("Oneshot 健康检查响应: {}", response.status());

    // 等待 HTTP 服务器完成（或处理其他任务）
    tokio::select! {
        _ = http_server_handle => {
            tracing::info!("HTTP 服务器已停止");
        },
        // 可以添加其他任务...
    }
}
