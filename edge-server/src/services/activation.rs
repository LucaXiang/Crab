use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::services::tenant_binding::{PlanType, Subscription, SubscriptionStatus, TenantBinding};
use crate::utils::AppError;
use shared::app_state::SubscriptionBlockedInfo;

/// 激活服务 - 管理边缘节点激活状态
///
/// # 激活流程
///
/// ```text
/// 1. 服务器启动，credential.json 可能存在或不存在
/// 2. check_activation() 检查激活状态
///    - 已激活且自检通过：返回 Ok(())
///    - 未激活：返回 Err(NotActivated)
///    - 自检失败：清理损坏数据，返回 Err(具体错误)
/// 3. 调用方 (red_coral) 决定如何处理未激活状态
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
    pub credential_cache: Arc<RwLock<Option<TenantBinding>>>,
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
    pub cert_expires_at: Option<i64>,
}

impl ActivationService {
    /// 创建激活服务
    ///
    /// 启动时从磁盘加载凭证缓存
    pub fn new(auth_server_url: String, cert_dir: PathBuf) -> Self {
        // Load credential from disk to memory cache on startup
        let credential_cache = match TenantBinding::load(&cert_dir) {
            Ok(cred) => {
                if let Some(c) = &cred {
                    tracing::info!(
                        "Loaded cached credential for tenant={}, edge={}",
                        c.binding.tenant_id,
                        c.binding.entity_id
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

    /// 检查订阅是否被阻止
    ///
    /// 阻止条件 (任一满足):
    /// 1. status 为 Inactive/Expired/Canceled/Unpaid
    /// 2. 签名已陈旧 (过期 + 3 天宽限期也已过)
    pub async fn is_subscription_blocked(&self) -> bool {
        let cache = self.credential_cache.read().await;
        let sub = match cache.as_ref().and_then(|c| c.subscription.as_ref()) {
            Some(s) => s,
            None => return false, // 无订阅数据 = 首次激活，不阻止
        };

        // 1. 状态阻止
        if sub.status.is_blocked() {
            return true;
        }

        // 2. 签名陈旧检查 (签名过期 + 宽限期也已过)
        if sub.is_signature_stale() {
            tracing::warn!(
                "Subscription signature stale (expired + grace period exceeded). Blocking."
            );
            return true;
        }

        false
    }

    /// 获取订阅阻止信息 (供 Bridge 使用)
    ///
    /// 返回 `None` 表示未阻止，可正常使用。
    /// 将阻止判断和 info 构建统一到 edge-server，避免 Bridge 重复实现。
    pub async fn get_subscription_blocked_info(&self) -> Option<SubscriptionBlockedInfo> {
        let cache = self.credential_cache.read().await;
        let sub = cache.as_ref()?.subscription.as_ref()?;

        let status_blocked = sub.status.is_blocked();
        let signature_stale = sub.is_signature_stale();

        if !status_blocked && !signature_stale {
            return None;
        }

        let status = sub.status.to_shared();
        let plan = sub.plan.to_shared();

        let (user_message, expired_at) = if signature_stale && !status_blocked {
            // 签名陈旧但状态本身是 Active/PastDue → 需要联网刷新
            ("subscription_signature_stale".to_string(), None)
        } else {
            let msg = match sub.status {
                SubscriptionStatus::Inactive => "subscription_inactive",
                SubscriptionStatus::Expired => "subscription_expired",
                SubscriptionStatus::Canceled => "subscription_canceled",
                SubscriptionStatus::Unpaid => "subscription_unpaid",
                _ => "subscription_blocked",
            };
            // Inactive/Unpaid 未激活状态不应有过期时间
            let expired_at = match sub.status {
                SubscriptionStatus::Inactive | SubscriptionStatus::Unpaid => None,
                _ => sub.expires_at,
            };
            (msg.to_string(), expired_at)
        };

        Some(SubscriptionBlockedInfo {
            status,
            plan,
            max_stores: sub.max_stores,
            expired_at,
            grace_period_days: None,
            grace_period_ends_at: None,
            in_grace_period: false,
            support_url: Some("https://support.example.com".to_string()),
            renewal_url: Some("https://billing.example.com/renew".to_string()),
            user_message,
        })
    }

    /// 检查激活状态并执行自检
    ///
    /// # 行为
    ///
    /// - 已激活且自检通过：返回 Ok(())
    /// - 未激活：返回 Err(NotActivated)
    /// - 自检失败：清理损坏数据，返回 Err(具体错误)
    ///
    /// # 与旧版 wait_for_activation 的区别
    ///
    /// 旧版会阻塞等待激活，新版立即返回结果。
    /// 调用方（red_coral）负责决定如何处理未激活状态。
    pub async fn check_activation(
        &self,
        cert_service: &crate::services::cert::CertService,
    ) -> Result<(), AppError> {
        // 1. Check activation status
        if !self.is_activated().await {
            tracing::info!("⏳ Server not activated");
            return Err(AppError::not_activated("Server not activated"));
        }

        // 2. Perform self-check (cert chain + hardware binding + credential signature + clock)
        tracing::info!("🔍 Performing self-check...");
        let cached_binding = self.credential_cache.read().await.clone();
        
        if let Err(e) = cert_service
            .self_check_with_binding(cached_binding.as_ref())
            .await
        {
            tracing::error!("❌ Self-check failed: {}", e);
            
            // 进入未绑定状态
            self.enter_unbound_state(cert_service).await;
            
            return Err(e);
        }

        tracing::info!("✅ Self-check passed!");

        // 3. Update last_verified_at timestamp (防止时钟篡改)
        self.update_last_verified_at().await;

        // 4. Subscription Sync (only after successful self-check)
        self.sync_subscription().await;

        Ok(())
    }

    /// 等待激活并执行自检（阻塞，可取消）
    ///
    /// # 行为
    ///
    /// - 已激活且自检通过：立即返回 `Ok(())`
    /// - 未激活：阻塞等待 `notify.notified()`
    /// - 自检失败：清理后继续等待
    /// - shutdown_token 取消：返回 `Err(())`
    ///
    /// # 用途
    ///
    /// 供 `Server::run()` 使用，确保 HTTPS 只在激活成功后启动。
    /// shutdown_token 使得 graceful shutdown 时不再卡在激活等待。
    pub async fn wait_for_activation(
        &self,
        cert_service: &crate::services::cert::CertService,
        shutdown_token: &CancellationToken,
    ) -> Result<(), ()> {
        loop {
            // 1. Check activation status
            if !self.is_activated().await {
                tracing::info!("⏳ Server not activated. Waiting for activation signal...");
                tokio::select! {
                    _ = shutdown_token.cancelled() => {
                        tracing::info!("Shutdown requested during activation wait");
                        return Err(());
                    }
                    _ = self.notify.notified() => {
                        tracing::info!("📡 Activation signal received!");
                    }
                }
            }

            // 2. Perform self-check (cert chain + hardware binding + credential signature + clock)
            tracing::info!("🔍 Performing self-check...");
            let cached_binding = self.credential_cache.read().await.clone();

            match cert_service
                .self_check_with_binding(cached_binding.as_ref())
                .await
            {
                Ok(()) => {
                    tracing::info!("✅ Self-check passed!");

                    // 3. Update last_verified_at timestamp (防止时钟篡改)
                    self.update_last_verified_at().await;

                    break; // Exit loop, continue to start server
                }
                Err(e) => {
                    tracing::error!("❌ Self-check failed: {}", e);

                    // 进入未绑定状态
                    self.enter_unbound_state(cert_service).await;

                    tracing::warn!("🔄 Server entered unbound state. Waiting for reactivation...");
                    // Loop continues, will wait for activation again
                }
            }
        }

        // 4. Subscription Sync (only after successful self-check)
        self.sync_subscription().await;
        Ok(())
    }

    /// 进入未绑定状态 (公开接口)
    ///
    /// 供 ServerState 在 TLS 加载失败时调用
    pub async fn enter_unbound_state_public(
        &self,
        cert_service: &crate::services::cert::CertService,
    ) {
        self.enter_unbound_state(cert_service).await;
    }

    /// 进入未绑定状态
    ///
    /// 清理所有可能损坏的数据，准备重新激活
    async fn enter_unbound_state(&self, cert_service: &crate::services::cert::CertService) {
        // 1. 清理损坏的证书文件
        if let Err(e) = cert_service.cleanup_certificates().await {
            tracing::warn!("Failed to cleanup certificates: {}", e);
        }

        // 2. 删除可能损坏的 Credential.json
        if let Err(e) = TenantBinding::delete(&self.cert_dir) {
            tracing::warn!("Failed to delete credential file: {}", e);
        }

        // 3. 清空内存缓存
        {
            let mut cache = self.credential_cache.write().await;
            *cache = None;
        }

        tracing::info!("🧹 Cleanup completed. Ready for reactivation.");
    }

    pub async fn is_activated(&self) -> bool {
        self.credential_cache.read().await.is_some()
    }

    pub async fn get_credential(&self) -> Result<Option<TenantBinding>, AppError> {
        let cache = self.credential_cache.read().await;
        Ok(cache.clone())
    }

    pub async fn get_status(&self) -> Result<ActivationStatus, AppError> {
        let credential = self.get_credential().await?;
        match credential {
            Some(cred) => Ok(ActivationStatus {
                is_activated: true,
                tenant_id: Some(cred.binding.tenant_id.clone()),
                edge_id: Some(cred.binding.entity_id.clone()),
                cert_fingerprint: Some(cred.binding.fingerprint.clone()),
                cert_expires_at: None,
            }),
            None => Ok(ActivationStatus::default()),
        }
    }

    pub async fn activate(&self, credential: TenantBinding) -> Result<(), AppError> {
        tracing::info!(
            "Attempting to activate edge server: tenant={}, edge={}, device={}",
            credential.binding.tenant_id,
            credential.binding.entity_id,
            credential.binding.device_id
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
        TenantBinding::delete(&self.cert_dir)
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

    /// 刷新 binding 时间戳
    ///
    /// 在自检成功后调用，向 Auth Server 请求刷新 binding。
    /// 新的 binding 包含更新的 last_verified_at 和新签名。
    async fn update_last_verified_at(&self) {
        let mut cache = self.credential_cache.write().await;
        let credential = match cache.as_ref() {
            Some(c) => c,
            None => return,
        };

        // 调用 Auth Server 刷新 binding
        let client = reqwest::Client::new();
        let resp = match client
            .post(format!("{}/api/binding/refresh", self.auth_server_url))
            .json(&serde_json::json!({ "binding": credential.binding }))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to refresh binding (offline?): {}", e);
                return;
            }
        };

        if !resp.status().is_success() {
            tracing::warn!(
                "Auth Server returned error for binding refresh: {}",
                resp.status()
            );
            return;
        }

        // 解析响应
        #[derive(serde::Deserialize)]
        struct RefreshResponse {
            success: bool,
            binding: Option<shared::activation::SignedBinding>,
            error: Option<String>,
        }

        let data: RefreshResponse = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to parse refresh response: {}", e);
                return;
            }
        };

        if !data.success {
            tracing::warn!("Binding refresh failed: {}", data.error.unwrap_or_default());
            return;
        }

        let new_binding = match data.binding {
            Some(b) => b,
            None => {
                tracing::error!("Refresh response missing binding");
                return;
            }
        };

        // 更新内存缓存
        if let Some(ref mut cred) = *cache {
            cred.update_binding(new_binding);

            // 保存到磁盘
            if let Err(e) = cred.save(&self.cert_dir) {
                tracing::error!("Failed to save refreshed binding: {}", e);
            } else {
                tracing::debug!("Binding refreshed successfully (last_verified_at updated)");
            }
        }
    }

    /// Sync subscription status (Local Cache -> Remote Fetch -> Update Cache)
    /// integrated into check_activation flow.
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
            .fetch_subscription_from_auth_server(&credential.binding.tenant_id)
            .await
        {
            tracing::info!(
                "Subscription sync successful for tenant {}: {:?}",
                credential.binding.tenant_id,
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
            // 网络失败 → 检查签名是否过期
            if let Some(sub) = &credential.subscription {
                if sub.is_signature_expired() {
                    tracing::warn!(
                        "Subscription sync failed AND signature expired! \
                         Offline grace period applies ({}d remaining).",
                        sub.signature_valid_until
                            .map(|v| {
                                let remaining_ms = (v + Subscription::SIGNATURE_GRACE_PERIOD_MS)
                                    - shared::util::now_millis();
                                remaining_ms / 86_400_000
                            })
                            .unwrap_or(0)
                    );
                } else {
                    tracing::info!(
                        "Subscription sync failed but signature still valid \
                         (expires in {}h). Using cached data.",
                        sub.signature_valid_until
                            .map(|v| (v - shared::util::now_millis()) / 3_600_000)
                            .unwrap_or(0)
                    );
                }
            } else {
                tracing::warn!(
                    "Subscription sync failed (network/auth error). No cached subscription."
                );
            }
        }
    }

    pub async fn fetch_subscription_from_auth_server(
        &self,
        tenant_id: &str,
    ) -> Option<Subscription> {
        use shared::activation::SubscriptionInfo;

        let client = reqwest::Client::new();
        let resp = match client
            .post(format!("{}/api/tenant/subscription", self.auth_server_url))
            .json(&serde_json::json!({ "tenant_id": tenant_id }))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to contact Auth Server: {}", e);
                return None;
            }
        };

        if !resp.status().is_success() {
            tracing::warn!("Auth Server error: {}", resp.status());
            return None;
        }

        // Parse response
        #[derive(Deserialize)]
        struct SubResponse {
            success: bool,
            error: Option<String>,
            subscription: Option<SubscriptionInfo>,
        }

        let data: SubResponse = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to parse subscription response: {}", e);
                return None;
            }
        };

        if !data.success {
            tracing::warn!(
                "Auth Server returned error: {}",
                data.error.unwrap_or_default()
            );
            return None;
        }

        let sub_info = match data.subscription {
            Some(s) => s,
            None => {
                tracing::warn!("Auth Server returned no subscription");
                return None;
            }
        };

        // Verify subscription signature using local tenant_ca.pem
        // Note: cert_dir is {work_dir} = {tenant}/server/, tenant_ca is in {tenant}/server/certs/
        let tenant_ca_path = self.cert_dir.join("certs").join("tenant_ca.pem");
        let tenant_ca_pem = match std::fs::read_to_string(&tenant_ca_path) {
            Ok(pem) => pem,
            Err(e) => {
                tracing::error!(
                    "Failed to read tenant CA for subscription verification: {}",
                    e
                );
                return None;
            }
        };

        if let Err(e) = sub_info.validate(&tenant_ca_pem) {
            tracing::error!("Subscription signature validation failed: {}", e);
            return None;
        }

        tracing::debug!("Subscription signature verified successfully");

        // Convert SubscriptionInfo to local Subscription
        let status = match sub_info.status {
            shared::activation::SubscriptionStatus::Inactive => SubscriptionStatus::Inactive,
            shared::activation::SubscriptionStatus::Active => SubscriptionStatus::Active,
            shared::activation::SubscriptionStatus::PastDue => SubscriptionStatus::PastDue,
            shared::activation::SubscriptionStatus::Expired => SubscriptionStatus::Expired,
            shared::activation::SubscriptionStatus::Canceled => SubscriptionStatus::Canceled,
            shared::activation::SubscriptionStatus::Unpaid => SubscriptionStatus::Unpaid,
        };

        let plan = match sub_info.plan {
            shared::activation::PlanType::Basic => PlanType::Basic,
            shared::activation::PlanType::Pro => PlanType::Pro,
            shared::activation::PlanType::Enterprise => PlanType::Enterprise,
        };

        let starts_at = sub_info.starts_at;
        let expires_at = sub_info.expires_at;
        let signature_valid_until = Some(sub_info.signature_valid_until);

        Some(Subscription {
            id: sub_info.id,
            tenant_id: sub_info.tenant_id,
            status,
            plan,
            starts_at,
            expires_at,
            features: sub_info.features,
            max_stores: sub_info.max_stores,
            last_checked_at: shared::util::now_millis(),
            signature_valid_until,
            signature: Some(sub_info.signature),
        })
    }
}
