//! Edge Activation Model
//!
//! Manages edge server activation state in database

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;

use crate::common::AppError;

/// Activation record ID
pub type ActivationId = Thing;

/// Edge activation state from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeActivation {
    pub id: Option<ActivationId>,
    pub is_activated: bool,
    pub activated_at: Option<DateTime<Utc>>,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub edge_id: Option<String>,
    pub edge_name: Option<String>,
    pub cert_fingerprint: Option<String>,
    pub cert_expires_at: Option<DateTime<Utc>>,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

impl Default for EdgeActivation {
    fn default() -> Self {
        Self {
            id: None,
            is_activated: false,
            activated_at: None,
            tenant_id: None,
            tenant_name: None,
            edge_id: None,
            edge_name: None,
            cert_fingerprint: None,
            cert_expires_at: None,
            last_heartbeat: None,
        }
    }
}

/// Activation service for database operations
#[derive(Clone)]
pub struct ActivationService {
    db: Surreal<Db>,
}

impl ActivationService {
    pub fn new(db: Surreal<Db>) -> Self {
        Self { db }
    }

    /// Get current activation state
    pub async fn get_status(&self) -> Result<EdgeActivation, AppError> {
        let result: Option<EdgeActivation> = self
            .db
            .select(("edge_activation", "default"))
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        Ok(result.unwrap_or_default())
    }

    /// Check if server is activated
    pub async fn is_activated(&self) -> bool {
        self.get_status()
            .await
            .map(|s| s.is_activated)
            .unwrap_or(false)
    }

    /// Activate the server
    pub async fn activate(
        &self,
        tenant_id: &str,
        tenant_name: &str,
        edge_id: &str,
        edge_name: &str,
        cert_fingerprint: &str,
        cert_expires_at: Option<DateTime<Utc>>,
    ) -> Result<EdgeActivation, AppError> {
        let mut result = self
            .db
            .query(
                r#"
                UPDATE edge_activation:default SET
                    is_activated = true,
                    activated_at = time::now(),
                    tenant_id = $tenant_id,
                    tenant_name = $tenant_name,
                    edge_id = $edge_id,
                    edge_name = $edge_name,
                    cert_fingerprint = $cert_fingerprint,
                    cert_expires_at = $cert_expires_at,
                    last_heartbeat = time::now(),
                    updated_at = time::now()
                "#,
            )
            .bind(("tenant_id", tenant_id.to_string()))
            .bind(("tenant_name", tenant_name.to_string()))
            .bind(("edge_id", edge_id.to_string()))
            .bind(("edge_name", edge_name.to_string()))
            .bind(("cert_fingerprint", cert_fingerprint.to_string()))
            .bind(("cert_expires_at", cert_expires_at))
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        let activation: Option<EdgeActivation> = result
            .take(0)
            .map_err(|e| AppError::database(e.to_string()))?;

        activation.ok_or_else(|| AppError::database("Failed to update activation".to_string()))
    }

    /// Update heartbeat timestamp
    pub async fn update_heartbeat(&self) -> Result<(), AppError> {
        self.db
            .query("UPDATE edge_activation:default SET last_heartbeat = time::now(), updated_at = time::now()")
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        Ok(())
    }

    /// Deactivate the server (for testing/reset)
    pub async fn deactivate(&self) -> Result<(), AppError> {
        self.db
            .query(
                "UPDATE edge_activation:default SET is_activated = false, updated_at = time::now()",
            )
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        Ok(())
    }
}
