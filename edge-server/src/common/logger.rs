//! Logging Infrastructure
//!
//! Structured logging setup with support for both development and production environments
//! Features:
//! - Daily rotating application logs (deleted after 14 days)
//! - Permanent audit logs (never deleted)
//! - Permanent security logs (never deleted)

use std::fs;
use std::path::{Path, PathBuf};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, prelude::*};

/// Clean up old application log files (older than 14 days)
///
/// Call this periodically (e.g., daily) to maintain log size
pub fn cleanup_old_logs(log_dir: &Path) -> anyhow::Result<()> {
    use chrono::{Local, TimeZone};

    // Use local time (Europe/Madrid timezone)
    let cutoff = Local::now() - chrono::Duration::days(14);

    // Application logs subdirectory
    let app_log_dir = log_dir.join("app");
    if app_log_dir.exists() {
        // Read directory and filter old app-YYYY-MM-DD.log files
        for entry in fs::read_dir(app_log_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Match app-YYYY-MM-DD.log pattern
                if name.starts_with("app-") && name.ends_with(".log") {
                    // Extract date from filename
                    if let Some(date_part) = name
                        .strip_prefix("app-")
                        .and_then(|d| d.strip_suffix(".log"))
                        && let Ok(naive_date) =
                            chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d")
                    {
                        // Parse as local date at midnight
                        if let Some(local_datetime) = Local
                            .from_local_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap())
                            .single()
                            && local_datetime < cutoff
                        {
                            // Delete old log file
                            fs::remove_file(&path)?;
                            tracing::info!(file = %name, "Deleted old log file");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Initialize the logging system with daily rotating logs
///
/// # Arguments
/// * `level` - Log level (e.g., "info", "debug", "warn")
/// * `json_format` - Whether to use JSON format (true for production, false for development)
/// * `log_dir` - Optional directory for file logging (e.g., Some("./work_dir/logs"))
///
/// # Examples
/// ```no_run
/// // Development setup (console only)
/// init_logger_with_file("debug", false, None)?;
///
/// // Production setup (console + file)
/// init_logger_with_file("info", true, Some("./work_dir/logs"))?;
/// ```
pub fn init_logger_with_file(
    level: &str,
    json_format: bool,
    log_dir: Option<&str>,
) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    // Console layer
    if json_format {
        // JSON format for production
        let console_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_current_span(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_filter(EnvFilter::new(level));

        if let Some(dir) = log_dir {
            // Create main log directory
            let log_dir = Path::new(dir);
            fs::create_dir_all(log_dir)?;

            // Create subdirectories for each log type
            let app_log_dir = log_dir.join("app");
            let audit_log_dir = log_dir.join("audit");
            let security_log_dir = log_dir.join("security");

            fs::create_dir_all(&app_log_dir)?;
            fs::create_dir_all(&audit_log_dir)?;
            fs::create_dir_all(&security_log_dir)?;

            // Daily rotating appender for application logs
            let app_log = RollingFileAppender::new(Rotation::DAILY, app_log_dir, "app");

            // Standard application logs (rotated daily, subject to 14-day cleanup)
            // Only log to app file if target is NOT "audit" or "security"
            let app_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::sync::Mutex::new(app_log))
                .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
                    meta.target() != "audit" && meta.target() != "security"
                }));

            // Permanent audit logs (never deleted)
            // Only log to audit file if target is "audit"
            let audit_log = RollingFileAppender::new(Rotation::DAILY, audit_log_dir, "audit");
            let audit_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::sync::Mutex::new(audit_log))
                .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
                    meta.target() == "audit"
                }));

            // Permanent security logs (never deleted)
            // Only log to security file if target is "security"
            let security_log =
                RollingFileAppender::new(Rotation::DAILY, security_log_dir, "security");
            let security_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(std::sync::Mutex::new(security_log))
                .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
                    meta.target() == "security"
                }));

            // Start cleanup task
            tokio::spawn(periodic_cleanup(log_dir.to_path_buf()));

            subscriber
                .with(console_layer)
                .with(app_layer)
                .with(audit_layer)
                .with(security_layer)
                .init();
        } else {
            subscriber.with(console_layer).init();
        }
    } else {
        // Pretty format for development
        let console_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .with_filter(EnvFilter::new(level));

        if let Some(dir) = log_dir {
            // Create main log directory
            let log_dir = Path::new(dir);
            fs::create_dir_all(log_dir)?;

            // Create subdirectories for each log type
            let app_log_dir = log_dir.join("app");
            let audit_log_dir = log_dir.join("audit");
            let security_log_dir = log_dir.join("security");

            fs::create_dir_all(&app_log_dir)?;
            fs::create_dir_all(&audit_log_dir)?;
            fs::create_dir_all(&security_log_dir)?;

            // Daily rotating appender for application logs
            let app_log = RollingFileAppender::new(Rotation::DAILY, app_log_dir, "app");

            // Standard application logs (rotated daily, subject to 14-day cleanup)
            // Only log to app file if target is NOT "audit" or "security"
            let app_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(app_log))
                .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
                    meta.target() != "audit" && meta.target() != "security"
                }));

            // Permanent audit logs (never deleted)
            // Only log to audit file if target is "audit"
            let audit_log = RollingFileAppender::new(Rotation::DAILY, audit_log_dir, "audit");
            let audit_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(audit_log))
                .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
                    meta.target() == "audit"
                }));

            // Permanent security logs (never deleted)
            // Only log to security file if target is "security"
            let security_log =
                RollingFileAppender::new(Rotation::DAILY, security_log_dir, "security");
            let security_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(security_log))
                .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
                    meta.target() == "security"
                }));

            // Start cleanup task
            tokio::spawn(periodic_cleanup(log_dir.to_path_buf()));

            subscriber
                .with(console_layer)
                .with(app_layer)
                .with(audit_layer)
                .with(security_layer)
                .init();
        } else {
            subscriber.with(console_layer).init();
        }
    }

    Ok(())
}

/// Periodic cleanup task - runs every hour to clean old logs
async fn periodic_cleanup(log_dir: PathBuf) {
    use tokio::time::{Duration, sleep};

    loop {
        sleep(Duration::from_secs(3600)).await; // Run every hour

        if let Err(e) = cleanup_old_logs(&log_dir) {
            tracing::error!(error = %e, "Failed to cleanup old logs");
        }
    }
}

/// Initialize the logging system (console only)
///
/// Convenience function for console-only logging
pub fn init_logger(level: &str, json_format: bool) -> anyhow::Result<()> {
    init_logger_with_file(level, json_format, None)
}

/// Audit log helper - records critical business operations
///
/// Audit logs are permanently stored in `audit-YYYY-MM-DD.log` files
/// They are NEVER deleted, even after 14 days.
/// Uses local time (Europe/Madrid timezone).
///
/// # Examples
/// ```no_run
/// // Login event
/// audit_log!("user123", "login", "employee:admin");
///
/// // Product creation
/// audit_log!("user456", "create", "product:789", "Added new product 'Widget'");
///
/// // Order cancellation
/// audit_log!("user789", "cancel", "order:12345", "Cancelled due to inventory shortage");
/// ```
#[macro_export]
macro_rules! audit_log {
    ($user_id:expr, $action:expr, $resource:expr) => {
        tracing::info!(
            target: "audit",
            user_id = $user_id,
            action = $action,
            resource = $resource,
            timestamp = chrono::Local::now().to_rfc3339(),
            "AUDIT"
        );
    };
    ($user_id:expr, $action:expr, $resource:expr, $details:expr) => {
        tracing::info!(
            target: "audit",
            user_id = $user_id,
            action = $action,
            resource = $resource,
            details = $details,
            timestamp = chrono::Local::now().to_rfc3339(),
            "AUDIT"
        );
    };
}

/// Security log helper - records security-related events
///
/// Security logs are permanently stored in `security-YYYY-MM-DD.log` files
/// They are NEVER deleted, even after 14 days.
/// Uses local time (Europe/Madrid timezone).
///
/// # Examples
/// ```no_run
/// // Failed authentication
/// security_log!(WARN, "auth_failed", username = "admin", ip = "192.168.1.1", reason = "invalid_password");
///
/// // Brute force attack
/// security_log!(ERROR, "brute_force", ip = "10.0.0.1", attempts = 50, blocked = true);
///
/// // Permission denied
/// security_log!(WARN, "permission_denied", user_id = "user123", action = "delete", resource = "product:456");
/// ```
#[macro_export]
macro_rules! security_log {
    (WARN, $event:expr, $($arg:tt)*) => {
        tracing::warn!(
            target: "security",
            event = $event,
            timestamp = chrono::Local::now().to_rfc3339(),
            level = "WARN",
            $($arg)*
        );
    };
    (ERROR, $event:expr, $($arg:tt)*) => {
        tracing::error!(
            target: "security",
            event = $event,
            timestamp = chrono::Local::now().to_rfc3339(),
            level = "ERROR",
            $($arg)*
        );
    };
    (INFO, $event:expr, $($arg:tt)*) => {
        tracing::info!(
            target: "security",
            event = $event,
            timestamp = chrono::Local::now().to_rfc3339(),
            level = "INFO",
            $($arg)*
        );
    };
}
