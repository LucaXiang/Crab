use edge_server::server::{auth::JwtConfig, ServerState};
use edge_server::{Config, Server};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;
use tracing::{error, info};

struct AppState {
    server_state: RwLock<Option<Arc<ServerState>>>,
}

#[tauri::command]
async fn check_health(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let guard = state.server_state.read().await;
    let server_state = guard
        .as_ref()
        .ok_or_else(|| "Server is still initializing...".to_string())?;

    let req = http::Request::builder()
        .uri("/health")
        .body(axum::body::Body::empty())
        .map_err(|e| e.to_string())?;

    let res = server_state.oneshot(req).await.map_err(|e| e.to_string())?;

    Ok(format!("Status: {}", res.status()))
}

#[tauri::command]
fn exit_app() {
    std::process::exit(0);
}

#[tauri::command]
fn get_local_ip() -> Result<String, String> {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ActivationParams {
    username: String,
    password: String,
    auth_url: String,
    tenant_id: String,
    common_name: String,
    role: Option<String>,
}

#[tauri::command]
async fn activate_server(
    state: tauri::State<'_, AppState>,
    _params: ActivationParams,
) -> Result<String, String> {
    let guard = state.server_state.read().await;
    let server_state = guard
        .as_ref()
        .ok_or_else(|| "Server is still initializing...".to_string())?;

    let _bus = server_state.get_message_bus();

    // Construct activation command
    // TODO: Update ServerCommand to support activation request with credentials
    // The current ServerCommand::Activate expects certificates, not credentials.
    /*
    let payload = shared::message::ServerCommandPayload {
        command: "activate_server".to_string(),
        data: serde_json::json!({
            "username": params.username,
            "password": params.password,
            "auth_url": params.auth_url,
            "tenant_id": params.tenant_id,
            "common_name": params.common_name,
            "role": params.role.unwrap_or("server".to_string())
        }),
    };

    let message = edge_server::message::BusMessage::server_command(&payload);

    // Publish to bus (Send to server for processing)
    bus.send_to_server(message)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Activation triggered".to_string())
    */
    Err("Activation not implemented in current protocol version".to_string())
}

#[tauri::command]
async fn send_test_message(state: tauri::State<'_, AppState>, msg: String) -> Result<(), String> {
    let guard = state.server_state.read().await;
    let server_state = guard
        .as_ref()
        .ok_or_else(|| "Server is still initializing...".to_string())?;

    let bus = server_state.get_message_bus();

    // Use OrderIntent instead of Notification to comply with client-side restrictions
    // Using a dummy dish item to carry the message in notes
    let payload = shared::message::OrderIntentPayload::add_dish(
        shared::message::TableId::new_unchecked("T_TEST"),
        vec![shared::message::DishItem::with_notes("TEST_MSG", 1, msg)],
        Some(shared::message::OperatorId::new("tauri_user")),
    );

    let message = edge_server::message::BusMessage::order_intent(&payload);

    // Publish to bus
    bus.publish(message).await.map_err(|e| e.to_string())?;
    Ok(())
}

use std::io::Write;

#[tauri::command]
async fn export_logs(app: tauri::AppHandle) -> Result<Vec<u8>, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_dir = app_data_dir.join("edge-server").join("logs");

    if !log_dir.exists() {
        return Err("Log directory not found".to_string());
    }

    let mut buffer = Vec::new();
    // Scope for ZipWriter to finish borrowing buffer
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));

        // zip 0.6 uses FileOptions, zip 2.x uses SimpleFileOptions.
        // We will try SimpleFileOptions first, assuming a modern version.
        // Note: We need to import compression method.

        let entries = std::fs::read_dir(&log_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_string_lossy();

                // 尝试兼容性写法
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .unix_permissions(0o644);

                zip.start_file(name, options).map_err(|e| e.to_string())?;
                let content = std::fs::read(&path).map_err(|e| e.to_string())?;
                zip.write_all(&content).map_err(|e| e.to_string())?;
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }

    Ok(buffer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install default crypto provider for Rustls if not already installed
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 初始化 AppState 并立即交给 Tauri 管理，避免 "managed state not found" 错误
            app.manage(AppState {
                server_state: RwLock::new(None),
            });

            let handle = app.handle().clone();

            // 获取应用数据目录作为工作目录，确保在 Android 上有写权限
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let work_dir = app_data_dir.join("edge-server");

            // 初始化文件日志系统 (使用 edge-server 的 logger)
            let log_dir = work_dir.join("logs");
            // 确保日志目录存在
            if !log_dir.exists() {
                std::fs::create_dir_all(&log_dir).expect("failed to create log dir");
            }

            // 初始化日志：INFO 级别，非 JSON 格式（方便阅读），写入 log_dir
            // 注意：这会初始化全局 tracing subscriber
            // 使用 block_on 确保 init_logger_with_file 内部的 tokio::spawn 能找到运行时
            let log_dir_str = log_dir.to_str().unwrap().to_string();
            tauri::async_runtime::block_on(async {
                if let Err(e) =
                    edge_server::common::init_logger_with_file("info", false, Some(&log_dir_str))
                {
                    eprintln!("Failed to initialize logger: {}", e);
                }
            });
            info!("Logger initialized! Logs will be written to: {:?}", log_dir);

            // 在异步运行时中初始化 Edge Server
            tauri::async_runtime::spawn(async move {
                // 确保工作目录存在
                if !work_dir.exists() {
                    match std::fs::create_dir_all(&work_dir) {
                        Ok(_) => info!("Created work dir: {:?}", work_dir),
                        Err(e) => {
                            error!("Failed to create work dir {:?}: {}", work_dir, e);
                            return;
                        }
                    }
                }

                println!("Initializing Edge Server in: {:?}", work_dir);

                let mut config =
                    Config::with_overrides(work_dir.to_string_lossy().to_string(), 3002, 8082);
                config.jwt = JwtConfig::default();
                config.environment = "development".to_string();

                let server_state = ServerState::initialize(&config).await;
                let server_state = Arc::new(server_state);

                // 更新全局状态
                {
                    let state = handle.state::<AppState>();
                    let mut w = state.server_state.write().await;
                    *w = Some(server_state.clone());
                }

                // 启动 HTTP Server (TCP Server 已经在 initialize 中启动)
                let s_state = server_state.clone();
                let s_config = config.clone();
                tokio::spawn(async move {
                    let server = Server::with_state(s_config, (*s_state).clone());
                    if let Err(e) = server.run().await {
                        eprintln!("HTTP Server error: {}", e);
                    }
                });

                // 订阅消息并转发给前端
                let bus = server_state.get_message_bus();
                let mut rx = bus.subscribe();

                let emit_handle = handle.clone();
                tokio::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        // 尝试解析 payload 为 JSON 以便打印和发送给前端
                        let payload_value: serde_json::Value = serde_json::from_slice(&msg.payload)
                            .unwrap_or_else(|_| serde_json::json!({"raw": msg.payload}));

                        println!(
                            "Received message from bus: [{}] {:?}",
                            msg.event_type, payload_value
                        );

                        // 构造一个对前端友好的消息结构
                        let frontend_msg = serde_json::json!({
                            "event_type": msg.event_type,
                            "payload": payload_value
                        });

                        // 将消息转发给前端
                        if let Err(e) = emit_handle.emit("server-message", frontend_msg) {
                            eprintln!("Failed to emit message: {}", e);
                        }
                    }
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_health,
            send_test_message,
            activate_server,
            get_local_ip,
            exit_app,
            export_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
