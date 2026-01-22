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
pub mod events;

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
    // Install default crypto provider for rustls
    // This is required to prevent panic: "Could not automatically determine the process-level CryptoProvider"
    let _ = rustls::crypto::ring::default_provider().install_default();

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
                EnvFilter::new("info,tao=error,http_access=warn,surrealdb=warn,red_coral=debug")
            } else {
                EnvFilter::new("warn,tao=error,http_access=warn,surrealdb=warn")
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

            // 3. Initialize ClientBridge with AppHandle for event emission
            let client_name = format!("redcoral-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));
            let bridge = ClientBridge::with_app_handle(&work_dir, &client_name, Some(app.handle().clone()))
                .map_err(|e| format!("Failed to initialize ClientBridge: {}", e))?;

            let bridge = Arc::new(RwLock::new(bridge));

            // Auto-restore session in background
            let bridge_for_task = bridge.clone();
            tauri::async_runtime::spawn(async move {
                let bridge = bridge_for_task.read().await;
                if let Err(e) = bridge.restore_last_session().await {
                    tracing::error!("Failed to restore session: {}", e);
                }
            });

            app.manage(bridge.clone());

            tracing::info!("ClientBridge initialized, restoring session...");

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            // Generic API commands
            commands::api_get,
            commands::api_post,
            commands::api_put,
            commands::api_delete,
            // Mode commands
            commands::check_first_run,
            commands::get_app_state,
            commands::get_mode_info,
            commands::get_current_mode_type,
            commands::start_server_mode,
            commands::start_client_mode,
            commands::stop_mode,
            commands::reconnect,
            commands::get_app_config,
            // Tenant commands
            commands::list_tenants,
            commands::activate_tenant,
            commands::switch_tenant,
            commands::remove_tenant,
            commands::get_current_tenant,
            // Auth commands (ClientBridge-based - unified)
            commands::login_employee,
            commands::logout_employee,
            // Data commands
            commands::list_tags,
            commands::get_tag,
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::list_categories,
            commands::get_category,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::list_products,
            commands::get_product,
            commands::get_product_full,
            commands::create_product,
            commands::update_product,
            commands::delete_product,
            commands::list_attributes,
            commands::get_attribute,
            commands::create_attribute,
            commands::update_attribute,
            commands::delete_attribute,
            commands::add_attribute_option,
            commands::update_attribute_option,
            commands::delete_attribute_option,
            // Product-Attribute binding commands
            commands::list_product_attributes,
            commands::bind_product_attribute,
            commands::unbind_product_attribute,
            commands::update_product_attribute_binding,
            // Category-Attribute binding commands
            commands::list_category_attributes,
            commands::bind_category_attribute,
            commands::unbind_category_attribute,
            commands::batch_update_category_sort_order,
            // Print Destination commands
            commands::list_print_destinations,
            commands::get_print_destination,
            commands::create_print_destination,
            commands::update_print_destination,
            commands::delete_print_destination,
            // Location commands
            commands::list_zones,
            commands::get_zone,
            commands::create_zone,
            commands::update_zone,
            commands::delete_zone,
            commands::list_tables,
            commands::list_tables_by_zone,
            commands::get_table,
            commands::create_table,
            commands::update_table,
            commands::delete_table,
            // Order commands (Query)
            commands::fetch_order_list,
            commands::list_orders,
            commands::list_open_orders,
            commands::get_order,
            commands::get_order_by_receipt,
            commands::get_last_order,
            commands::verify_order_chain,
            // Order Event Sourcing commands
            commands::order_execute_command,
            commands::order_execute,
            commands::order_get_active_orders,
            commands::order_get_snapshot,
            commands::order_sync_since,
            commands::order_get_events_since,
            commands::order_get_events_for_order,
            // System commands
            commands::get_system_state,
            commands::update_system_state,
            commands::init_genesis,
            commands::update_last_order,
            commands::update_sync_state,
            commands::get_pending_sync_orders,
            commands::list_employees,
            commands::list_all_employees,
            commands::get_employee,
            commands::create_employee,
            commands::update_employee,
            commands::delete_employee,
            commands::list_price_rules,
            commands::list_active_price_rules,
            commands::get_price_rule,
            commands::create_price_rule,
            commands::update_price_rule,
            commands::delete_price_rule,
            // Roles commands
            commands::list_roles,
            commands::get_role,
            commands::create_role,
            commands::update_role,
            commands::delete_role,
            commands::get_role_permissions,
            commands::get_all_permissions,
            commands::update_role_permissions,
            // Backup commands
            commands::export_data,
            commands::import_data,
            // Image cache commands
            commands::get_image_path,
            commands::resolve_image_paths,
            commands::prefetch_images,
            commands::cleanup_image_cache,
            commands::save_image,
            // Sync commands
            commands::get_sync_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
