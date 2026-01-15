use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::common::AppError;
use crate::server::credential::{Credential, Subscription};

#[derive(Clone, Debug)]
pub struct ActivationService {
    notify: Arc<tokio::sync::Notify>,
    auth_server_url: String,
    cert_dir: PathBuf,
    pub credential_cache: Arc<RwLock<Option<Credential>>>,
}

#[derive(Debug, Default, Clone)]
pub struct ActivationStatus {
    pub is_activated: bool,
    pub tenant_id: Option<String>,
    pub edge_id: Option<String>,
    pub cert_fingerprint: Option<String>,
    pub cert_expires_at: Option<DateTime<Utc>>,
}

impl ActivationService {
    pub fn new(auth_server_url: String, cert_dir: PathBuf) -> Self {
        // Load credential from disk to memory cache on startup
        let credential_cache = match Credential::load(&cert_dir) {
            Ok(cred) => {
                if let Some(c) = &cred {
                    tracing::info!(
                        "Loaded cached credential for tenant={}, edge={}",
                        c.tenant_id,
                        c.server_id
                    );
                }
                Arc::new(RwLock::new(cred))
            }
            Err(e) => {
                tracing::error!("Failed to load credential during startup: {}", e);
                Arc::new(RwLock::new(None))
            }
        };

        Self {
            notify: Arc::new(tokio::sync::Notify::new()),
            auth_server_url,
            cert_dir,
            credential_cache,
        }
    }

    pub async fn wait_for_activation(
        &self,
        cert_service: &crate::server::services::cert::CertService,
    ) {
        // 1. Check activation status
        if !self.is_activated().await {
            tracing::info!("Waiting for activation signal...");
            self.notify.notified().await;
            tracing::info!("Activation signal received!");
        }

        // 2. Perform boot self-check
        tracing::info!("Performing boot self-check...");
        if let Err(e) = cert_service.self_check().await {
            tracing::error!("Boot self-check failed: {}", e);
            panic!("Boot self-check failed: {}", e);
        }
        tracing::info!("✅ Boot self-check passed!");

        // 3. Initial Subscription Sync
        // Integrated from perform_initial_subscription_check as per user request
        self.sync_subscription().await;
    }

    pub async fn is_activated(&self) -> bool {
        self.credential_cache.read().await.is_some()
    }

    pub async fn get_credential(&self) -> Result<Option<Credential>, AppError> {
        let cache = self.credential_cache.read().await;
        Ok(cache.clone())
    }

    pub async fn get_status(&self) -> Result<ActivationStatus, AppError> {
        let credential = self.get_credential().await?;
        match credential {
            Some(cred) => Ok(ActivationStatus {
                is_activated: true,
                tenant_id: Some(cred.tenant_id),
                edge_id: Some(cred.server_id),
                cert_fingerprint: Some(cred.fingerprint),
                cert_expires_at: None,
            }),
            None => Ok(ActivationStatus::default()),
        }
    }

    pub async fn activate(&self, credential: Credential) -> Result<(), AppError> {
        tracing::info!(
            "Attempting to activate edge server: tenant={}, edge={}, device={:?}",
            credential.tenant_id,
            credential.server_id,
            credential.device_id
        );

        // 1. Save to disk
        credential
            .save(&self.cert_dir)
            .map_err(|e| AppError::internal(format!("Failed to save credential: {}", e)))?;

        // 2. Update memory cache
        {
            let mut cache = self.credential_cache.write().await;
            *cache = Some(credential.clone());
        }

        tracing::info!("🚀 Server activated!");
        self.notify.notify_waiters();
        Ok(())
    }

    pub async fn deactivate(&self) -> Result<(), AppError> {
        tracing::warn!("⚠️ Deactivating server and resetting state");

        // 1. Delete from disk
        Credential::delete(&self.cert_dir)
            .map_err(|e| AppError::internal(format!("Failed to delete credential: {}", e)))?;

        // 2. Clear memory cache
        {
            let mut cache = self.credential_cache.write().await;
            *cache = None;
        }

        Ok(())
    }

    pub async fn deactivate_and_reset(&self) -> Result<(), AppError> {
        self.deactivate().await
    }

    /// Sync subscription status (Local Cache -> Remote Fetch -> Update Cache)
    /// integrated into wait_for_activation flow.
    pub async fn sync_subscription(&self) {
        tracing::info!("Running subscription synchronization...");

        // Use cache to get current credential
        let mut credential = match self.get_credential().await {
            Ok(Some(c)) => c,
            _ => {
                tracing::debug!("Server not activated, skipping subscription sync");
                return;
            }
        };

        // Fetch subscription from remote
        if let Some(sub) = self
            .fetch_subscription_from_auth_server(&credential.tenant_id)
            .await
        {
            tracing::info!(
                "Subscription sync successful for tenant {}: {:?}",
                credential.tenant_id,
                sub.status
            );

            // Update credential with new subscription
            credential.subscription = Some(sub);

            // 1. Persist to disk
            if let Err(e) = credential.save(&self.cert_dir) {
                tracing::error!(
                    "Failed to save updated subscription to credential file: {}",
                    e
                );
            }

            // 2. Update memory cache
            {
                let mut cache = self.credential_cache.write().await;
                *cache = Some(credential);
            }
        } else {
            tracing::warn!(
                "Subscription sync failed (network/auth error). Using offline/cached trust."
            );
        }
    }

    pub async fn fetch_subscription_from_auth_server(
        &self,
        tenant_id: &str,
    ) -> Option<Subscription> {
        let client = reqwest::Client::new();
        match client
            .post(format!("{}/api/tenant/subscription", self.auth_server_url))
            .json(&serde_json::json!({ "tenant_id": tenant_id }))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    #[derive(Deserialize)]
                    struct SubResponse {
                        subscription: Subscription,
                    }
                    match resp.json::<SubResponse>().await {
                        Ok(data) => Some(data.subscription),
                        Err(e) => {
                            tracing::error!("Failed to parse subscription response: {}", e);
                            None
                        }
                    }
                } else {
                    tracing::warn!("Auth Server error: {}", resp.status());
                    None
                }
            }
            Err(e) => {
                tracing::error!("Failed to contact Auth Server: {}", e);
                None
            }
        }
    }
}
