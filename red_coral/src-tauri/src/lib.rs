//! RedCoral POS - Tauri Application
//!
//! 支持两种运行模式：
//! - Server 模式: 本地运行 edge-server，使用 In-Process 通信
//! - Client 模式: 连接远程 edge-server，使用 mTLS 通信

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use tracing_appender::rolling;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Re-export edge-server and crab-client for future use
pub use crab_client;
pub use edge_server;
pub use shared;

pub mod commands;
pub mod core;

use core::ClientBridge;

struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(
            w,
            "{}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f")
        )
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 1. Initialize logging system
            let log_dir = app.path().app_data_dir()?.join("logs");
            std::fs::create_dir_all(&log_dir)
                .map_err(|e| format!("Failed to create logs directory: {}", e))?;

            let file_appender = rolling::daily(&log_dir, "redcoral-pos.log");
            let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

            let env_filter = if let Ok(from_env) = EnvFilter::try_from_default_env() {
                from_env
            } else if cfg!(debug_assertions) {
                EnvFilter::new("info,tao=error,sqlx=warn,red_coral=debug")
            } else {
                EnvFilter::new("warn,tao=error,sqlx=error")
            };

            let file_layer = fmt::layer()
                .with_timer(LocalTimer)
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_writer(non_blocking_file);

            let stdout_layer = fmt::layer()
                .with_timer(LocalTimer)
                .with_ansi(true)
                .with_target(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::io::stdout);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(file_layer)
                .with(stdout_layer)
                .init();

            std::panic::set_hook(Box::new(|info| {
                let backtrace = std::backtrace::Backtrace::capture();
                let msg = info.to_string();
                eprintln!("!!! APPLICATION PANIC !!!\nMessage: {}\nBacktrace:\n{}", msg, backtrace);
                tracing::error!(target: "panic", message = %msg, backtrace = %backtrace, "panic occurred");
            }));

            tracing::info!(path = log_dir.display().to_string(), "Tracing initialized successfully");

            // 2. Setup data directory
            let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            let work_dir = app_data_dir.join("redcoral");
            std::fs::create_dir_all(&work_dir).ok();

            tracing::info!(work_dir = %work_dir.display(), "RedCoral POS starting...");

            // 3. Initialize ClientBridge
            let client_name = format!("redcoral-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));
            let bridge = ClientBridge::new(&work_dir, &client_name)
                .map_err(|e| format!("Failed to initialize ClientBridge: {}", e))?;

            let bridge = Arc::new(RwLock::new(bridge));
            app.manage(bridge);

            tracing::info!("ClientBridge initialized, mode: Disconnected");

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // Mode commands
            commands::check_first_run,
            commands::get_mode_info,
            commands::get_current_mode_type,
            commands::start_server_mode,
            commands::start_client_mode,
            commands::stop_mode,
            commands::get_app_config,
            // Tenant commands
            commands::list_tenants,
            commands::activate_tenant,
            commands::switch_tenant,
            commands::remove_tenant,
            commands::get_current_tenant,
            // Auth commands (TenantManager-based)
            commands::login_online,
            commands::login_offline,
            commands::login_auto,
            commands::logout,
            commands::get_current_session,
            commands::has_offline_cache,
            commands::list_cached_employees,
            // Auth commands (ClientBridge-based - unified)
            commands::login_employee,
            commands::logout_employee,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
