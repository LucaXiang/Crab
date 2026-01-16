use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::credential::{Credential, Subscription};
use crate::utils::AppError;

/// 激活服务 - 管理边缘节点激活状态
///
/// # 激活流程
///
/// ```text
/// 1. 服务器启动，credential.json 可能存在或不存在
/// 2. wait_for_activation() 检查激活状态
///    - 已激活：返回，继续启动服务
///    - 未激活：等待 notify.notified()
/// 3. 外部通过 ProvisioningService 完成激活
/// 4. 激活成功后调用 notify.notify_waiters()
/// 5. wait_for_activation() 返回，继续启动服务
/// ```
///
/// # 状态存储
///
/// 激活凭证存储在 `work_dir/auth_storage/credential.json`
#[derive(Clone, Debug)]
pub struct ActivationService {
    /// 激活通知器 (用于等待/通知)
    notify: Arc<tokio::sync::Notify>,
    /// 认证服务器 URL
    auth_server_url: String,
    /// 证书目录
    cert_dir: PathBuf,
    /// 凭证缓存 (内存)
    pub credential_cache: Arc<RwLock<Option<Credential>>>,
}

/// 激活状态 (用于 API 查询)
#[derive(Debug, Default, Clone)]
pub struct ActivationStatus {
    /// 是否已激活
    pub is_activated: bool,
    /// 租户 ID
    pub tenant_id: Option<String>,
    /// 边缘节点 ID
    pub edge_id: Option<String>,
    /// 证书指纹
    pub cert_fingerprint: Option<String>,
    /// 证书过期时间
    pub cert_expires_at: Option<DateTime<Utc>>,
}

impl ActivationService {
    /// 创建激活服务
    ///
    /// 启动时从磁盘加载凭证缓存
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

    /// 获取认证服务器 URL
    pub fn auth_server_url(&self) -> &str {
        &self.auth_server_url
    }

    /// 等待激活信号
    ///
    /// # 行为
    ///
    /// - 已激活：立即返回
    /// - 未激活：阻塞等待 notify.notified()
    pub async fn wait_for_activation(&self, cert_service: &crate::services::cert::CertService) {
        // 1. Check activation status
        if !self.is_activated().await {
            tracing::info!("Waiting for activation signal...");
            self.notify.notified().await;
            tracing::info!("Activation signal received!");
        }

        // 2. Perform boot self-check
        tracing::info!("Performing boot self-check...");
        if let Err(e) = cert_service.self_check().await {
            tracing::error!(
                "Boot self-check failed: {}. Cleaning up certificates and waiting for reactivation.",
                e
            );

            // 清理旧的证书文件
            if let Err(cleanup_error) = cert_service.cleanup_certificates().await {
                tracing::error!("Failed to cleanup certificates: {}", cleanup_error);
            }

            // 清空缓存，强制重新激活
            {
                let mut cache = self.credential_cache.write().await;
                *cache = None;
            }

            tracing::error!(
                "Certificate validation failed. Server is now waiting for reactivation."
            );
            tracing::error!("Please check certificate validity and hardware binding.");

            // 等待重新激活
            self.notify.notified().await;
            tracing::info!("Reactivation signal received!");

            // 重新执行自检
            tracing::info!("Performing reactivation self-check...");
            if let Err(recheck_error) = cert_service.self_check().await {
                tracing::error!(
                    "Reactivation self-check failed: {}. Will wait again.",
                    recheck_error
                );
                // 再次清空缓存并等待
                {
                    let mut cache = self.credential_cache.write().await;
                    *cache = None;
                }
                self.notify.notified().await;
            } else {
                tracing::info!("Reactivation self-check passed!");
            }
        } else {
            tracing::info!("Boot self-check passed!");
        }

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
