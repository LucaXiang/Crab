//! RedCoral POS - Tauri Application
//!
//! 支持两种运行模式：
//! - Server 模式: 本地运行 edge-server，使用 In-Process 通信
//! - Client 模式: 连接远程 edge-server，使用 mTLS 通信

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing_appender::rolling;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Re-export edge-server and crab-client for future use
pub use crab_client;
pub use edge_server;
pub use shared;

pub mod api;
pub mod commands;
pub mod core;
pub mod events;
pub mod utils;

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
    // Err(()) means provider was already installed — safe to ignore
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        // Already installed by another component — this is expected and harmless
    }

    tauri::Builder::default()
        .setup(|app| {
            // 1. Initialize logging system
            let log_dir = app.path().app_data_dir()?.join("logs");
            std::fs::create_dir_all(&log_dir)
                .map_err(|e| format!("Failed to create logs directory: {}", e))?;

            let file_appender = rolling::daily(&log_dir, "redcoral-pos.log");
            let (non_blocking_file, log_guard) = tracing_appender::non_blocking(file_appender);

            let env_filter = if let Ok(from_env) = EnvFilter::try_from_default_env() {
                from_env
            } else if cfg!(debug_assertions) {
                EnvFilter::new("info,tao=error,http_access=warn,red_coral=debug,edge_server::orders=debug,edge_server::pricing=debug")
            } else {
                EnvFilter::new("info,tao=error,http_access=warn")
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
                tracing::error!(target: "panic", message = %msg, backtrace = %backtrace, "APPLICATION PANIC");
            }));

            // Keep the log guard alive for the app's lifetime.
            // Without this, the non-blocking writer thread stops when setup() returns,
            // silently dropping ALL subsequent log output.
            app.manage(std::sync::Mutex::new(log_guard));

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

            let bridge = Arc::new(bridge);

            // Auto-restore session in background, notify frontend when done
            let bridge_for_task = bridge.clone();
            let handle_for_task = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let error = match bridge_for_task.restore_last_session().await {
                    Ok(()) => None,
                    Err(e) => {
                        tracing::error!("Failed to restore session: {}", e);
                        Some(e.to_string())
                    }
                };
                // 存储结果（可查询），同时发送事件（通知等待中的前端）
                bridge_for_task.mark_initialized(error.clone());
                let _ = handle_for_task.emit("backend-ready", error);
            });

            app.manage(bridge.clone());

            tracing::info!("ClientBridge initialized, restoring session...");

            Ok(())
        })
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // When a second instance is launched, focus the main window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
            tracing::info!("Second instance detected, focusing existing window");
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            // Generic API commands
            commands::api_get,
            commands::api_post,
            commands::api_put,
            commands::api_delete,
            // Mode commands
            commands::check_first_run,
            commands::get_init_status,
            commands::retry_init,
            commands::get_app_state,
            commands::get_mode_info,
            commands::get_current_mode_type,
            commands::start_server_mode,
            commands::start_client_mode,
            commands::stop_mode,
            commands::reconnect,
            commands::get_app_config,
            commands::update_server_config,
            commands::update_client_config,
            // Tenant commands
            commands::verify_tenant,
            commands::activate_server_tenant,
            commands::activate_client_tenant,
            commands::deactivate_current_mode,
            commands::exit_tenant,
            commands::upload_p12,
            commands::get_current_tenant,
            commands::get_tenant_details,
            commands::check_subscription,
            // Auth commands (ClientBridge-based - unified)
            commands::login_employee,
            commands::logout_employee,
            commands::get_current_session,
            commands::escalate_permission,
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
            commands::batch_update_product_sort_order,
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
            commands::fetch_member_order_history,
            commands::fetch_order_detail,
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
            commands::get_store_info,
            commands::update_store_info,
            commands::list_label_templates,
            commands::get_label_template,
            commands::create_label_template,
            commands::update_label_template,
            commands::delete_label_template,
            commands::list_employees,
            commands::list_all_employees,
            commands::get_employee,
            commands::create_employee,
            commands::update_employee,
            commands::delete_employee,
            commands::list_price_rules,
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
            // Printer commands
            commands::list_printers,
            commands::open_cash_drawer,
            commands::print_receipt,
            // Health commands
            commands::get_health_status,
            // Shift commands (班次管理)
            commands::list_shifts,
            commands::get_shift,
            commands::get_current_shift,
            commands::open_shift,
            commands::update_shift,
            commands::close_shift,
            commands::force_close_shift,
            commands::heartbeat_shift,
            commands::recover_stale_shifts,
            // Daily Report commands (日结报告)
            commands::list_daily_reports,
            commands::get_daily_report,
            commands::get_daily_report_by_date,
            commands::generate_daily_report,
            // Statistics commands (数据统计)
            commands::get_statistics,
            commands::get_sales_report,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Graceful shutdown: stop the bridge so Server::cleanup() runs
                // and audit.lock is removed (preventing false "abnormal shutdown" on next start)
                if let Some(bridge) = app.try_state::<Arc<ClientBridge>>() {
                    let bridge = Arc::clone(&*bridge);
                    // block_on is safe here — Tauri calls Exit after the event loop ends
                    tauri::async_runtime::block_on(async move {
                        if let Err(e) = bridge.stop().await {
                            tracing::error!("Failed to stop bridge on exit: {}", e);
                        } else {
                            tracing::info!("Bridge stopped gracefully on app exit");
                        }
                    });
                }
            }
        });
}
