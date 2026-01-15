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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EdgeActivation {
    pub id: Option<ActivationId>,
    pub is_activated: bool,
    pub activated_at: Option<DateTime<Utc>>,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub edge_id: Option<String>,
    pub edge_name: Option<String>,
    pub device_id: Option<String>,
    pub cert_fingerprint: Option<String>,
    pub cert_expires_at: Option<DateTime<Utc>>,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

/// Activation service for database operations
#[derive(Clone)]
pub struct ActivationService {
    db: Surreal<Db>,
}

/// Activation parameters
pub struct ActivationParams<'a> {
    pub tenant_id: &'a str,
    pub tenant_name: &'a str,
    pub edge_id: &'a str,
    pub edge_name: &'a str,
    pub device_id: &'a str,
    pub cert_fingerprint: &'a str,
    pub cert_expires_at: Option<DateTime<Utc>>,
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
    pub async fn activate(&self, params: ActivationParams<'_>) -> Result<EdgeActivation, AppError> {
        tracing::info!(
            "Attempting to activate edge server: tenant={}, edge={}, device={}",
            params.tenant_id,
            params.edge_id,
            params.device_id
        );

        // Robust activation using INSERT ... ON DUPLICATE KEY UPDATE (Upsert)
        // This is a single statement, so result.take(0) works correctly.
        let mut result = self
            .db
            .query(
                r#"
                INSERT INTO edge_activation (
                    id,
                    is_activated,
                    activated_at,
                    tenant_id,
                    tenant_name,
                    edge_id,
                    edge_name,
                    device_id,
                    cert_fingerprint,
                    cert_expires_at,
                    last_heartbeat,
                    updated_at
                ) VALUES (
                    'default',
                    true,
                    time::now(),
                    $tenant_id,
                    $tenant_name,
                    $edge_id,
                    $edge_name,
                    $device_id,
                    $cert_fingerprint,
                    $cert_expires_at,
                    time::now(),
                    time::now()
                ) ON DUPLICATE KEY UPDATE
                    is_activated = true,
                    activated_at = time::now(),
                    tenant_id = $tenant_id,
                    tenant_name = $tenant_name,
                    edge_id = $edge_id,
                    edge_name = $edge_name,
                    device_id = $device_id,
                    cert_fingerprint = $cert_fingerprint,
                    cert_expires_at = $cert_expires_at,
                    last_heartbeat = time::now(),
                    updated_at = time::now()
                ;
                "#,
            )
            .bind(("tenant_id", params.tenant_id.to_string()))
            .bind(("tenant_name", params.tenant_name.to_string()))
            .bind(("edge_id", params.edge_id.to_string()))
            .bind(("edge_name", params.edge_name.to_string()))
            .bind(("device_id", params.device_id.to_string()))
            .bind(("cert_fingerprint", params.cert_fingerprint.to_string()))
            .bind(("cert_expires_at", params.cert_expires_at))
            .await
            .map_err(|e| {
                tracing::error!("Activation query failed: {}", e);
                AppError::database(e.to_string())
            })?;

        // INSERT/UPDATE returns a list of records
        let mut activations: Vec<EdgeActivation> = result.take(0).map_err(|e| {
            tracing::error!("Failed to take result: {}", e);
            AppError::database(e.to_string())
        })?;

        let activation = activations.pop();

        if activation.is_none() {
            tracing::error!("Activation query returned no result!");
        }

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
