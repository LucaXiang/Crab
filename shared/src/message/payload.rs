use serde::{Deserialize, Serialize};
use std::fmt;

// ==================== Notification Level ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for NotificationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    System,
    Printer,
    Network,
    Business, // Renamed from Order/Payment to generic Business
}

// ==================== Server Commands ====================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum ServerCommand {
    /// Activate the edge server (receive certificates and metadata)
    Activate {
        tenant_id: String,
        tenant_name: String,
        edge_id: String,
        edge_name: String,
        tenant_ca_pem: String,
        edge_cert_pem: String,
        edge_key_pem: String,
    },

    /// Update server configuration
    ConfigUpdate {
        key: String,
        value: serde_json::Value,
    },

    /// Remote restart
    Restart {
        delay_seconds: u32,
        reason: Option<String>,
    },

    /// Health check ping
    Ping,
}

// ==================== Payloads ====================

/// Payload for Notification (Server -> Client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub category: NotificationCategory,
    pub data: Option<serde_json::Value>,
}

/// Payload for ServerCommand (Upstream -> Server)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerCommandPayload {
    pub command: ServerCommand,
}

// ==================== Convenience Constructors ====================

impl NotificationPayload {
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Info,
            category: NotificationCategory::System,
            data: None,
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Warning,
            category: NotificationCategory::System,
            data: None,
        }
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Error,
            category: NotificationCategory::System,
            data: None,
        }
    }
}
