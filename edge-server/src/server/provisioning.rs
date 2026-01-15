//! Provisioning Service
//!
//! Handles interaction with the Auth Server for provisioning and activation.

use crate::common::AppError;
use crate::server::ServerState;
use crate::server::credential::verify_cert_pair;
use serde_json::json;

pub struct ProvisioningService {
    state: ServerState,
    auth_url: String,
    client: reqwest::Client,
}

impl ProvisioningService {
    pub fn new(state: ServerState, auth_url: String) -> Self {
        Self {
            state,
            auth_url,
            client: reqwest::Client::new(),
        }
    }

    /// Perform full activation flow: Login -> Issue Cert -> Save -> Activate
    pub async fn activate(
        &self,
        username: &str,
        password: &str,
        tenant_id: &str,
        edge_id: &str,
    ) -> Result<(), AppError> {
        // 1. Login
        let token = self.login(username, password).await?;

        // 2. Generate Device ID
        let device_id = crab_cert::generate_hardware_id();
        tracing::info!("Generated Device ID: {}", device_id);

        // 3. Issue Certificate
        let (cert_pem, key_pem, tenant_ca_pem) = self
            .issue_cert(&token, tenant_id, edge_id, &device_id)
            .await?;

        // 4. Verify Certificate Chain
        let credential = verify_cert_pair(&cert_pem, &tenant_ca_pem)
            .map_err(|e| AppError::validation(format!("Certificate verification failed: {}", e)))?;

        // Validate IDs match input
        if credential.tenant_id != tenant_id {
            return Err(AppError::validation(format!(
                "Tenant ID mismatch: expected {}, got {}",
                tenant_id, credential.tenant_id
            )));
        }
        if credential.server_id != edge_id {
            return Err(AppError::validation(format!(
                "Edge ID mismatch: expected {}, got {}",
                edge_id, credential.server_id
            )));
        }

        // 5. Save Certificates
        self.state
            .save_certificates(&tenant_ca_pem, &cert_pem, &key_pem)
            .await
            .map_err(|e| AppError::internal(format!("Failed to save certificates: {}", e)))?;

        // 6. Activate in DB
        self.state
            .activate_with_metadata(
                tenant_id,
                &format!("Tenant {}", tenant_id),
                edge_id,
                edge_id,
                &device_id,
                &credential.fingerprint,
            )
            .await?;

        Ok(())
    }

    async fn login(&self, username: &str, password: &str) -> Result<String, AppError> {
        let resp = self
            .client
            .post(format!("{}/api/auth/login", self.auth_url))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Auth server connection failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::internal(format!(
                "Login failed: status {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Invalid login response: {}", e)))?;

        if !body["success"].as_bool().unwrap_or(false) {
            let msg = body["message"].as_str().unwrap_or("Unknown error");
            return Err(AppError::validation(format!("Login failed: {}", msg)));
        }

        body["token"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| AppError::internal("Token missing in response".to_string()))
    }

    async fn issue_cert(
        &self,
        token: &str,
        tenant_id: &str,
        edge_id: &str,
        device_id: &str,
    ) -> Result<(String, String, String), AppError> {
        let resp = self
            .client
            .post(format!("{}/api/cert/issue", self.auth_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&json!({
                "tenant_id": tenant_id,
                "common_name": edge_id,
                "is_server": true,
                "device_id": device_id
            }))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Auth server connection failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::internal(format!(
                "Issue cert failed: {} - {}",
                status, text
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Invalid cert response: {}", e)))?;

        if !body["success"].as_bool().unwrap_or(false) {
            let msg = body["error"].as_str().unwrap_or("Unknown error");
            return Err(AppError::validation(format!("Issue cert failed: {}", msg)));
        }

        let cert = body["cert"]
            .as_str()
            .ok_or_else(|| AppError::validation("Cert missing".to_string()))?;
        let key = body["key"]
            .as_str()
            .ok_or_else(|| AppError::validation("Key missing".to_string()))?;
        let tenant_ca = body["tenant_ca_cert"]
            .as_str()
            .ok_or_else(|| AppError::validation("Tenant CA missing".to_string()))?;

        Ok((cert.to_string(), key.to_string(), tenant_ca.to_string()))
    }
}
