//! Common types for the shared crate
//!
//! Utility types used across the framework

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Timestamp type (Unix milliseconds)
pub type Timestamp = i64;

/// Entity ID type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityId(pub String);

impl EntityId {
    /// Create a new entity ID from a string
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the inner value
    pub fn inner(&self) -> &str {
        &self.0
    }

    /// Parse into a specific format (e.g., "employee:admin")
    pub fn parse(&self, prefix: &str) -> Option<&str> {
        self.0.strip_prefix(&format!("{}:", prefix))
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for EntityId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EntityId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: String,
    pub action: String,
    pub resource: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: Timestamp,
}

/// User role enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Manager,
    User,
}

impl UserRole {
    /// Get default permissions for this role
    pub fn default_permissions(&self) -> Vec<String> {
        match self {
            Self::Admin => vec!["*".to_string()],
            Self::Manager => vec!["read:*".to_string(), "write:own".to_string()],
            Self::User => vec!["read:own".to_string()],
        }
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Manager => write!(f, "manager"),
            Self::User => write!(f, "user"),
        }
    }
}

/// Permission type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission(pub String);

impl Permission {
    /// Check if this permission grants access to the given resource action
    pub fn grants(&self, action: &str) -> bool {
        if self.0 == "*" {
            return true;
        }
        if self.0.ends_with(":*") {
            let prefix = &self.0[..self.0.len() - 2];
            return action.starts_with(prefix);
        }
        self.0 == action
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthCheckStatus,
    pub version: String,
    pub timestamp: Timestamp,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    /// Create a new healthy status
    pub fn healthy(version: String, uptime_seconds: u64) -> Self {
        Self {
            status: HealthCheckStatus::Healthy,
            version,
            timestamp: crate::util::now_millis(),
            uptime_seconds,
        }
    }
}
