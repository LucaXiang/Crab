//! CloudSyncService — HTTP client for pushing data to crab-cloud

use reqwest::Client;
use shared::activation::SignedBinding;
use shared::cloud::{CloudSyncBatch, CloudSyncResponse};
use std::path::Path;

use crate::utils::AppError;

/// HTTP client for crab-cloud sync API
pub struct CloudSyncService {
    client: Client,
    cloud_url: String,
    /// Entity ID for this edge-server
    edge_id: String,
}

impl CloudSyncService {
    /// Create a new CloudSyncService with mTLS configuration
    ///
    /// Loads entity cert + key from `certs_dir` for mTLS client auth.
    /// The `cloud_url` should be the base URL of crab-cloud (e.g., "https://cloud.example.com").
    pub fn new(cloud_url: String, edge_id: String, certs_dir: &Path) -> Result<Self, AppError> {
        let edge_cert_pem = std::fs::read(certs_dir.join("edge_cert.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read edge cert: {e}")))?;
        let edge_key_pem = std::fs::read(certs_dir.join("edge_key.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read edge key: {e}")))?;
        let tenant_ca_pem = std::fs::read(certs_dir.join("tenant_ca.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read tenant CA: {e}")))?;
        let root_ca_pem = std::fs::read(certs_dir.join("root_ca.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read root CA: {e}")))?;

        // Build identity: edge_cert + tenant_ca (full chain) + key
        let identity_pem = [
            edge_cert_pem,
            b"\n".to_vec(),
            tenant_ca_pem,
            b"\n".to_vec(),
            edge_key_pem,
        ]
        .concat();
        let identity = reqwest::Identity::from_pem(&identity_pem)
            .map_err(|e| AppError::internal(format!("Failed to create identity: {e}")))?;

        // Root CA for verifying crab-cloud's server certificate
        let root_cert = reqwest::Certificate::from_pem(&root_ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to parse root CA: {e}")))?;

        let client = Client::builder()
            .identity(identity)
            .add_root_certificate(root_cert)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::internal(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            cloud_url,
            edge_id,
        })
    }

    /// Push a sync batch to crab-cloud
    pub async fn push_batch(
        &self,
        batch: CloudSyncBatch,
        binding: &SignedBinding,
    ) -> Result<CloudSyncResponse, AppError> {
        let binding_json = serde_json::to_string(binding)
            .map_err(|e| AppError::internal(format!("Failed to serialize binding: {e}")))?;

        let url = format!("{}/api/edge/sync", self.cloud_url);

        let response = self
            .client
            .post(&url)
            .header("X-Signed-Binding", &binding_json)
            .json(&batch)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Cloud sync request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::internal(format!(
                "Cloud sync failed with status {status}: {body}"
            )));
        }

        let sync_response: CloudSyncResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse sync response: {e}")))?;

        Ok(sync_response)
    }

    pub fn edge_id(&self) -> &str {
        &self.edge_id
    }
}
