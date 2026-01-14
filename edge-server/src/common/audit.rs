//! Audit Logging System
//!
//! Provides comprehensive audit logging for edge server

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// Audit log entry for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Type of action performed
    pub action: AuditAction,
    /// Resource that was accessed or modified
    pub resource: String,
    /// Whether the action was successful
    pub success: bool,
    /// ID of the user who performed the action (if authenticated)
    pub user_id: Option<String>,
    /// Username (for display)
    pub username: Option<String>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Details about the action
    pub details: Option<String>,
    /// Error message if action failed
    pub error: Option<String>,
}

/// Types of auditable actions (matches database schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication actions
    Login,
    Logout,
    LoginFailed,
    TokenRefresh,

    // Data access
    Read,
    Create,
    Update,
    Delete,

    // Administrative actions
    ConfigChange,
    UserCreate,
    UserUpdate,
    UserDelete,
    RoleChange,
    PermissionChange,

    // System actions
    ServerStart,
    ServerStop,
    BackupCreate,
    BackupRestore,

    // Security events
    UnauthorizedAccess,
    InvalidToken,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(action: AuditAction, resource: impl Into<String>) -> Self {
        Self {
            action,
            resource: resource.into(),
            success: true,
            user_id: None,
            username: None,
            ip_address: None,
            user_agent: None,
            details: None,
            error: None,
        }
    }

    /// Set user information
    pub fn with_user(mut self, user_id: impl Into<String>, username: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self.username = Some(username.into());
        self
    }

    /// Set IP address
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set details
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Mark as failed with error
    pub fn failed(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error = Some(error.into());
        self
    }
}

/// Database-backed audit logger
#[derive(Clone)]
pub struct AuditLogger {
    db: Surreal<Db>,
}

impl AuditLogger {
    /// Create a new audit logger with database connection
    pub fn new(db: Surreal<Db>) -> Self {
        Self { db }
    }

    /// Log an audit entry to the database
    pub async fn log(&self, entry: AuditEntry) {
        let result: Result<Option<AuditEntry>, _> = self
            .db
            .create("audit_log")
            .content(entry.clone())
            .await;

        if let Err(e) = result {
            // Fall back to tracing if database write fails
            tracing::error!("Failed to write audit log to database: {}", e);
            Self::log_to_console(&entry);
        }
    }

    /// Query recent audit logs
    pub async fn query_recent(&self, limit: usize) -> Vec<AuditEntry> {
        let result = self
            .db
            .query("SELECT * FROM audit_log ORDER BY created_at DESC LIMIT $limit")
            .bind(("limit", limit))
            .await;

        match result {
            Ok(mut response) => response.take(0).unwrap_or_default(),
            Err(e) => {
                tracing::error!("Failed to query audit logs: {}", e);
                vec![]
            }
        }
    }

    /// Query audit logs by user
    pub async fn query_by_user(&self, user_id: &str, limit: usize) -> Vec<AuditEntry> {
        let result = self
            .db
            .query("SELECT * FROM audit_log WHERE user_id = $user_id ORDER BY created_at DESC LIMIT $limit")
            .bind(("user_id", user_id.to_string()))
            .bind(("limit", limit))
            .await;

        match result {
            Ok(mut response) => response.take(0).unwrap_or_default(),
            Err(e) => {
                tracing::error!("Failed to query audit logs: {}", e);
                vec![]
            }
        }
    }

    /// Fallback console logging
    fn log_to_console(entry: &AuditEntry) {
        let status = if entry.success { "✓" } else { "✗" };
        tracing::info!(
            target: "audit",
            "[{}] {:?} on {} by {:?}",
            status,
            entry.action,
            entry.resource,
            entry.username.as_deref().unwrap_or("anonymous")
        );
    }
}
