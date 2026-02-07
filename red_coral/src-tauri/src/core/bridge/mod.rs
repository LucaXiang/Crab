//! ClientBridge - 统一的客户端桥接层
//!
//! 提供 Server/Client 模式的统一接口，屏蔽底层差异。
//! - Server 模式: 本地运行 edge-server，使用 In-Process 通信
//! - Client 模式: 连接远程 edge-server，使用 mTLS 通信

mod config;
mod error;
mod types;

// Re-export public types
pub use config::{AppConfig, ClientModeConfig, ServerModeConfig};
pub use error::BridgeError;
pub use types::{AppState, ModeInfo, ModeType};

// Internal types (pub(crate) for use within this crate)
pub(crate) use types::{ClientMode, LocalClientState, RemoteClientState};

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

use crab_client::CrabClient;
use edge_server::services::tenant_binding::SubscriptionStatus;
use shared::app_state::{ActivationRequiredReason, ClockDirection};
use shared::order::{CommandResponse, OrderCommand, OrderCommandPayload, OrderEvent, OrderSnapshot, SyncResponse};
use super::tenant_manager::{TenantError, TenantManager};

/// 客户端桥接层
pub struct ClientBridge {
    /// 多租户管理器
    tenant_manager: Arc<RwLock<TenantManager>>,
    /// 当前模式
    mode: RwLock<ClientMode>,
    /// 应用配置
    config: RwLock<AppConfig>,
    /// 配置文件路径
    config_path: PathBuf,
    /// Tauri AppHandle for emitting events (optional for testing)
    app_handle: Option<tauri::AppHandle>,
}

impl ClientBridge {
    /// 创建新的 ClientBridge (convenience wrapper without AppHandle)
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Result<Self, BridgeError> {
        Self::with_app_handle(base_path, client_name, None)
    }

    /// 创建新的 ClientBridge with optional AppHandle for emitting Tauri events
    pub fn with_app_handle(
        base_path: impl Into<PathBuf>,
        client_name: &str,
        app_handle: Option<tauri::AppHandle>,
    ) -> Result<Self, BridgeError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)?;

        let config_path = base_path.join("config.json");
        let config = AppConfig::load(&config_path)?;

        let tenants_path = base_path.join("tenants");
        let mut tenant_manager = TenantManager::new(&tenants_path, client_name);
        tenant_manager.load_existing_tenants()?;

        Ok(Self {
            tenant_manager: Arc::new(RwLock::new(tenant_manager)),
            mode: RwLock::new(ClientMode::Disconnected),
            config: RwLock::new(config),
            config_path,
            app_handle,
        })
    }

    /// Set the app handle after initialization
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 保存当前配置
    async fn save_config(&self) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        config.save(&self.config_path)
    }

    // ============ 租户管理辅助 ============

    /// 切换当前租户并保存配置
    pub async fn switch_tenant(&self, tenant_id: &str) -> Result<(), BridgeError> {
        // Check if we need to restart server
        let is_server_mode = {
            let mode = self.mode.read().await;
            matches!(*mode, ClientMode::Server { .. })
        };

        // If server mode, stop it first to release resources/locks
        if is_server_mode {
            tracing::info!("Stopping server to switch tenant...");
            self.stop().await?;
        }

        // 1. 切换内存状态
        {
            let mut tm = self.tenant_manager.write().await;
            tm.switch_tenant(tenant_id)?;
        }

        // 2. 更新并保存配置
        {
            let mut config = self.config.write().await;
            config.current_tenant = Some(tenant_id.to_string());
            config.save(&self.config_path)?;
        }

        // If it was server mode, restart it with new tenant data
        if is_server_mode {
            tracing::info!("Restarting server with new tenant...");
            self.start_server_mode().await?;
        }

        tracing::info!(tenant_id = %tenant_id, "Switched tenant and saved config");
        Ok(())
    }

    /// 激活设备并保存证书
    ///
    /// 仅保存证书到磁盘，不启动任何模式。
    /// 返回 (tenant_id, subscription_status)，前端据此决定下一步。
    pub async fn handle_activation(
        &self,
        auth_url: &str,
        username: &str,
        password: &str,
    ) -> Result<(String, Option<String>), BridgeError> {
        // 1. 调用 TenantManager 激活（保存证书和 credential 到磁盘）
        let tenant_id = {
            let mut tm = self.tenant_manager.write().await;
            tm.activate_device(auth_url, username, password).await?
        };

        // 2. 更新已知租户列表和当前租户
        {
            let mut config = self.config.write().await;
            if !config.known_tenants.contains(&tenant_id) {
                config.known_tenants.push(tenant_id.clone());
            }
            config.current_tenant = Some(tenant_id.clone());
            config.save(&self.config_path)?;
        }

        // 3. 读取订阅状态（从刚保存的 credential）
        let subscription_status = {
            let tm = self.tenant_manager.read().await;
            tm.get_subscription_status(&tenant_id)
        };

        tracing::info!(tenant_id = %tenant_id, ?subscription_status, "Device activated and config saved (mode not started)");
        Ok((tenant_id, subscription_status))
    }

    // ============ 模式管理 ============

    /// 恢复上次的会话状态 (启动时调用)
    pub async fn restore_last_session(self: &Arc<Self>) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        let mode = config.current_mode;
        let client_config = config.client_config.clone();
        let current_tenant = config.current_tenant.clone();
        drop(config);

        // 恢复租户选择
        if let Some(tenant_id) = current_tenant {
            tracing::info!("Restoring tenant selection: {}", tenant_id);
            let mut tm = self.tenant_manager.write().await;
            if let Err(e) = tm.switch_tenant(&tenant_id) {
                tracing::warn!("Failed to restore tenant {}: {}", tenant_id, e);
            }
        }

        let result = match mode {
            ModeType::Server => {
                tracing::info!("Restoring Server mode...");
                if let Err(e) = self.start_server_mode().await {
                    tracing::error!("Failed to restore Server mode: {}", e);
                    Err(e)
                } else {
                    Ok(())
                }
            }
            ModeType::Client => {
                if let Some(cfg) = client_config {
                    tracing::info!("Restoring Client mode...");
                    if let Err(e) = self
                        .start_client_mode(&cfg.edge_url, &cfg.message_addr)
                        .await
                    {
                        tracing::error!("Failed to restore Client mode: {}", e);
                        Err(e)
                    } else {
                        Ok(())
                    }
                } else {
                    tracing::warn!("Client mode configured but missing client_config");
                    Ok(())
                }
            }
            ModeType::Disconnected => {
                tracing::info!("Starting in Disconnected mode");
                Ok(())
            }
        };

        result
    }

    /// 获取当前模式信息
    pub async fn get_mode_info(&self) -> ModeInfo {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        let (mode, is_connected, is_authenticated, client_check_info) = match &*mode_guard {
            ClientMode::Disconnected => (ModeType::Disconnected, false, false, None),
            ClientMode::Server { client, .. } => {
                let is_auth = matches!(client, Some(LocalClientState::Authenticated(_)));
                (ModeType::Server, true, is_auth, None)
            }
            ClientMode::Client {
                client, edge_url, ..
            } => {
                let is_auth = matches!(client, Some(RemoteClientState::Authenticated(_)));
                let check_info = if let Some(state) = client {
                    let http = match state {
                        RemoteClientState::Connected(c) => c.edge_http_client().cloned(),
                        RemoteClientState::Authenticated(c) => c.edge_http_client().cloned(),
                    };
                    Some((edge_url.clone(), http))
                } else {
                    None
                };
                (ModeType::Client, false, is_auth, check_info)
            }
        };

        drop(mode_guard);

        // Perform real network health check for Client mode
        let final_is_connected = if mode == ModeType::Client {
            if let Some((url, Some(http))) = client_check_info {
                match http
                    .get(format!("{}/health", url))
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    Ok(resp) => resp.status().is_success(),
                    Err(e) => {
                        tracing::warn!("Health check failed: {}", e);
                        false
                    }
                }
            } else {
                false
            }
        } else {
            is_connected
        };

        ModeInfo {
            mode,
            is_connected: final_is_connected,
            is_authenticated,
            tenant_id: tenant_manager.current_tenant_id().map(|s| s.to_string()),
            username: tenant_manager.current_session().map(|s| s.username.clone()),
        }
    }

    /// 从 edge-server 检测需要激活的具体原因
    async fn detect_activation_reason_from_server(
        &self,
        server_state: &edge_server::ServerState,
        tenant_manager: &TenantManager,
    ) -> ActivationRequiredReason {
        // 尝试调用 edge-server 的自检获取具体错误
        let cert_service = server_state.cert_service();
        let credential = server_state.activation_service().get_credential().await.ok().flatten();
        
        match cert_service.self_check_with_binding(credential.as_ref()).await {
            Ok(()) => {
                // 自检通过但未激活，说明 Credential.json 不存在
                ActivationRequiredReason::FirstTimeSetup
            }
            Err(e) => {
                self.parse_activation_error(&e.to_string(), tenant_manager)
            }
        }
    }

    /// 解析激活错误消息
    fn parse_activation_error(&self, error_str: &str, tenant_manager: &TenantManager) -> ActivationRequiredReason {
        let error_lower = error_str.to_lowercase();
        
        if error_lower.contains("expired") {
            // 证书过期
            if let Some(paths) = tenant_manager.current_paths() {
                if let Ok(cert_pem) = std::fs::read_to_string(paths.edge_cert()) {
                    if let Ok(metadata) = crab_cert::CertMetadata::from_pem(&cert_pem) {
                        let now = time::OffsetDateTime::now_utc();
                        let duration = metadata.not_after - now;
                        let days_overdue = -duration.whole_days();
                        let expired_at_millis =
                            metadata.not_after.unix_timestamp() * 1000
                                + metadata.not_after.millisecond() as i64;
                        return ActivationRequiredReason::CertificateExpired {
                            expired_at: expired_at_millis,
                            days_overdue,
                        };
                    }
                }
            }
            ActivationRequiredReason::CertificateExpired {
                expired_at: 0,
                days_overdue: 0,
            }
        } else if error_lower.contains("hardware id mismatch") || error_lower.contains("device id mismatch") || error_lower.contains("device_id") {
            // 设备 ID 不匹配
            let (expected, actual) = self.extract_device_ids(error_str);
            ActivationRequiredReason::DeviceMismatch { expected, actual }
        } else if error_lower.contains("clock") || error_lower.contains("time") && error_lower.contains("tamper") {
            // 时钟篡改
            let direction = if error_lower.contains("backward") {
                ClockDirection::Backward
            } else {
                ClockDirection::Forward
            };
            let drift_seconds = error_str
                .split_whitespace()
                .find_map(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            ActivationRequiredReason::ClockTampering {
                direction,
                drift_seconds,
                last_verified_at: 0,
            }
        } else if error_lower.contains("signature") {
            // 签名无效
            ActivationRequiredReason::SignatureInvalid {
                component: "credential".to_string(),
                error: error_str.to_string(),
            }
        } else if error_lower.contains("chain") || error_lower.contains("certificate") && error_lower.contains("invalid") {
            // 证书链无效
            ActivationRequiredReason::CertificateInvalid {
                error: error_str.to_string(),
            }
        } else if error_lower.contains("not found") || error_lower.contains("missing") {
            // 文件缺失
            ActivationRequiredReason::FirstTimeSetup
        } else {
            // 未知错误，返回通用的绑定无效
            ActivationRequiredReason::BindingInvalid {
                error: error_str.to_string(),
            }
        }
    }

    /// 从错误消息中提取设备 ID
    fn extract_device_ids(&self, error_str: &str) -> (String, String) {
        // 尝试解析格式如 "expected xxx, got yyy" 或类似格式
        if let Some(idx) = error_str.find("expected ") {
            let rest = &error_str[idx + 9..];
            if let Some(comma_idx) = rest.find(", ") {
                let exp = rest[..comma_idx].trim().to_string();
                let act_start = rest.find("got ").map(|i| i + 4).unwrap_or(comma_idx + 2);
                let act_end = rest[act_start..].find(|c: char| !c.is_alphanumeric() && c != '-').unwrap_or(rest.len() - act_start);
                let act = rest[act_start..act_start + act_end].trim().to_string();
                return (exp, act);
            }
        }
        // 无法解析，返回掩码值
        ("***".to_string(), crab_cert::generate_hardware_id()[..8].to_string())
    }

    /// 检测需要激活的具体原因 (基于 TenantPaths)
    ///
    /// Server 模式: 检查 server/certs/ 下的证书
    /// Client 模式: 检查 certs/ 下的证书
    fn detect_activation_reason(&self, tenant_manager: &TenantManager, for_server: bool) -> ActivationRequiredReason {
        // 1. 检查是否有路径管理器
        let paths = match tenant_manager.current_paths() {
            Some(p) => p,
            None => return ActivationRequiredReason::FirstTimeSetup,
        };

        // 2. 检查证书是否存在
        let has_certs = if for_server {
            paths.has_server_certificates()
        } else {
            paths.has_client_certificates()
        };

        if !has_certs {
            return ActivationRequiredReason::FirstTimeSetup;
        }

        // 3. 读取证书检查有效性
        let cert_path = if for_server {
            paths.edge_cert()
        } else {
            paths.client_cert()
        };

        let cert_pem = match std::fs::read_to_string(&cert_path) {
            Ok(pem) => pem,
            Err(_) => return ActivationRequiredReason::CertificateInvalid {
                error: "Cannot read certificate file".to_string(),
            },
        };

        let metadata = match crab_cert::CertMetadata::from_pem(&cert_pem) {
            Ok(m) => m,
            Err(e) => return ActivationRequiredReason::CertificateInvalid {
                error: format!("Invalid certificate: {}", e),
            },
        };

        // 4. 检查证书过期
        let now = time::OffsetDateTime::now_utc();
        let duration = metadata.not_after - now;
        let days_remaining = duration.whole_days();
        let not_after_millis =
            metadata.not_after.unix_timestamp() * 1000
                + metadata.not_after.millisecond() as i64;

        if days_remaining < 0 {
            return ActivationRequiredReason::CertificateExpired {
                expired_at: not_after_millis,
                days_overdue: -days_remaining,
            };
        }

        if days_remaining <= 30 {
            return ActivationRequiredReason::CertificateExpiringSoon {
                expires_at: not_after_millis,
                days_remaining,
            };
        }

        // 5. 检查设备 ID 绑定
        let current_device_id = crab_cert::generate_hardware_id();
        if let Some(cert_device_id) = &metadata.device_id {
            if cert_device_id != &current_device_id {
                return ActivationRequiredReason::DeviceMismatch {
                    expected: cert_device_id[..8].to_string(),
                    actual: current_device_id[..8].to_string(),
                };
            }
        }

        // 证书有效，可能是其他原因或需要检查 credential
        ActivationRequiredReason::FirstTimeSetup
    }

    /// 获取应用状态 (用于前端路由守卫)
    pub async fn get_app_state(&self) -> AppState {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        match &*mode_guard {
            ClientMode::Disconnected => {
                if tenant_manager.current_tenant_id().is_none() {
                    AppState::ServerNoTenant
                } else {
                    let has_certs = tenant_manager
                        .current_paths()
                        .map(|p| p.is_server_activated())
                        .unwrap_or(false);

                    if has_certs {
                        AppState::Uninitialized
                    } else {
                        let reason = ActivationRequiredReason::FirstTimeSetup;
                        AppState::ServerNeedActivation {
                            can_auto_recover: reason.can_auto_recover(),
                            recovery_hint: reason.recovery_hint_code().to_string(),
                            reason,
                        }
                    }
                }
            }

            ClientMode::Server {
                server_state,
                client,
                ..
            } => {
                // 1. 首先检查 edge-server 激活状态 (权威)
                let is_activated = server_state.is_activated().await;

                if !is_activated {
                    // 调用 edge-server 自检获取具体错误
                    let reason = self.detect_activation_reason_from_server(server_state, &tenant_manager).await;
                    return AppState::ServerNeedActivation {
                        can_auto_recover: reason.can_auto_recover(),
                        recovery_hint: reason.recovery_hint_code().to_string(),
                        reason,
                    };
                }

                let credential = server_state
                    .activation_service()
                    .get_credential()
                    .await
                    .ok()
                    .flatten();

                if let Some(_cred) = credential {
                    // 订阅阻止检查 (统一使用 edge-server 判断，包含签名陈旧检查)
                    let blocked_info = server_state.get_subscription_blocked_info().await;
                    if let Some(info) = blocked_info {
                        AppState::ServerSubscriptionBlocked { info }
                    } else {
                        // 2. 检查员工登录状态
                        // 优先检查 CrabClient 状态（权威）
                        if matches!(client, Some(LocalClientState::Authenticated(_))) {
                            return AppState::ServerAuthenticated;
                        }
                        // 其次检查内存中的 session
                        if tenant_manager.current_session().is_some() {
                            return AppState::ServerAuthenticated;
                        }
                        // 未登录
                        AppState::ServerReady
                    }
                } else {
                    // 无 credential，需要激活
                    let reason = self.detect_activation_reason(&tenant_manager, true); // Server mode
                    AppState::ServerNeedActivation {
                        can_auto_recover: reason.can_auto_recover(),
                        recovery_hint: reason.recovery_hint_code().to_string(),
                        reason,
                    }
                }
            }

            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(_)) => AppState::ClientAuthenticated,
                Some(RemoteClientState::Connected(_)) => AppState::ClientConnected,
                None => {
                    let has_certs = tenant_manager
                        .current_paths()
                        .map(|p| p.has_client_certificates())
                        .unwrap_or(false);

                    if has_certs {
                        AppState::ClientDisconnected
                    } else {
                        AppState::ClientNeedSetup
                    }
                }
            },
        }
    }

    /// 获取当前活动会话 (用于启动时恢复登录状态)
    pub async fn get_current_session(&self) -> Option<super::session_cache::EmployeeSession> {
        let tenant_manager = self.tenant_manager.read().await;
        tenant_manager.current_session().cloned()
    }

    /// 重新检查订阅状态
    ///
    /// 在 Server 模式下，调用 edge-server 的 sync_subscription 从 auth-server 拉取最新订阅，
    /// 然后返回最新的 AppState。
    pub async fn check_subscription(&self) -> Result<AppState, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // 从 auth-server 同步最新订阅状态
                server_state.sync_subscription().await;
                tracing::info!("Subscription re-checked from auth-server");
            }
            _ => {
                tracing::warn!("check_subscription called in non-Server mode, skipping sync");
            }
        }

        // 释放 mode_guard 以避免死锁（get_app_state 也需要读锁）
        drop(mode_guard);

        // 返回最新的 AppState
        Ok(self.get_app_state().await)
    }

    /// 获取健康检查组件 (订阅、网络、数据库)
    pub async fn get_health_components(
        &self,
    ) -> (
        shared::app_state::SubscriptionHealth,
        shared::app_state::NetworkHealth,
        shared::app_state::DatabaseHealth,
    ) {
        use shared::app_state::{DatabaseHealth, HealthLevel, NetworkHealth, SubscriptionHealth};

        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // === 订阅健康状态 ===
                let subscription = match server_state.activation_service().get_credential().await {
                    Ok(Some(cred)) => {
                        if let Some(sub) = &cred.subscription {
                            let status = match sub.status {
                                SubscriptionStatus::Active => HealthLevel::Healthy,
                                SubscriptionStatus::PastDue => HealthLevel::Warning,
                                SubscriptionStatus::Expired | SubscriptionStatus::Canceled => {
                                    HealthLevel::Critical
                                }
                                SubscriptionStatus::Inactive | SubscriptionStatus::Unpaid => {
                                    HealthLevel::Critical
                                }
                            };
                            let needs_refresh = sub.is_signature_expired();
                            SubscriptionHealth {
                                status: if needs_refresh {
                                    HealthLevel::Warning
                                } else {
                                    status
                                },
                                plan: Some(format!("{:?}", sub.plan)),
                                subscription_status: Some(format!("{:?}", sub.status)),
                                signature_valid_until: sub.signature_valid_until,
                                needs_refresh,
                            }
                        } else {
                            SubscriptionHealth {
                                status: HealthLevel::Unknown,
                                plan: None,
                                subscription_status: None,
                                signature_valid_until: None,
                                needs_refresh: false,
                            }
                        }
                    }
                    _ => SubscriptionHealth {
                        status: HealthLevel::Unknown,
                        plan: None,
                        subscription_status: None,
                        signature_valid_until: None,
                        needs_refresh: false,
                    },
                };

                // === 网络健康状态 ===
                // 尝试连接 auth server 检查可达性
                let network = {
                    let auth_url = std::env::var("AUTH_SERVER_URL")
                        .unwrap_or_else(|_| "https://localhost:3001".to_string());
                    let client = reqwest::Client::builder()
                        .danger_accept_invalid_certs(true) // 开发环境
                        .timeout(std::time::Duration::from_secs(3))
                        .build();

                    let (reachable, last_connected) = match client {
                        Ok(c) => {
                            match c.get(format!("{}/health", auth_url)).send().await {
                                Ok(resp) if resp.status().is_success() => {
                                    (true, Some(shared::util::now_millis()))
                                }
                                _ => (false, None),
                            }
                        }
                        Err(_) => (false, None),
                    };

                    NetworkHealth {
                        status: if reachable {
                            HealthLevel::Healthy
                        } else {
                            HealthLevel::Warning
                        },
                        auth_server_reachable: reachable,
                        last_connected_at: last_connected,
                    }
                };

                // === 数据库健康状态 ===
                let database = {
                    // 尝试执行简单查询检查数据库是否正常
                    let db_ok: bool = server_state
                        .db
                        .query("SELECT count() FROM employee GROUP ALL")
                        .await
                        .is_ok();

                    DatabaseHealth {
                        status: if db_ok {
                            HealthLevel::Healthy
                        } else {
                            HealthLevel::Critical
                        },
                        size_bytes: None, // SurrealDB embedded 不易获取大小
                        last_write_at: None,
                    }
                };

                (subscription, network, database)
            }

            ClientMode::Client { client, edge_url, .. } => {
                // Client 模式: 检查与 edge server 的连接
                let (network_status, reachable) = if let Some(state) = client {
                    let http = match state {
                        RemoteClientState::Connected(c) => c.edge_http_client().cloned(),
                        RemoteClientState::Authenticated(c) => c.edge_http_client().cloned(),
                    };
                    if let Some(http) = http {
                        match http
                            .get(format!("{}/health", edge_url))
                            .timeout(std::time::Duration::from_secs(2))
                            .send()
                            .await
                        {
                            Ok(resp) if resp.status().is_success() => (HealthLevel::Healthy, true),
                            _ => (HealthLevel::Warning, false),
                        }
                    } else {
                        (HealthLevel::Unknown, false)
                    }
                } else {
                    (HealthLevel::Critical, false)
                };

                let subscription = SubscriptionHealth {
                    status: HealthLevel::Unknown, // Client 模式不直接访问订阅
                    plan: None,
                    subscription_status: None,
                    signature_valid_until: None,
                    needs_refresh: false,
                };

                let network = NetworkHealth {
                    status: network_status,
                    auth_server_reachable: reachable,
                    last_connected_at: if reachable {
                        Some(shared::util::now_millis())
                    } else {
                        None
                    },
                };

                let database = DatabaseHealth {
                    status: HealthLevel::Unknown, // Client 模式不直接访问数据库
                    size_bytes: None,
                    last_write_at: None,
                };

                (subscription, network, database)
            }

            ClientMode::Disconnected => {
                let subscription = SubscriptionHealth {
                    status: HealthLevel::Unknown,
                    plan: None,
                    subscription_status: None,
                    signature_valid_until: None,
                    needs_refresh: false,
                };

                let network = NetworkHealth {
                    status: HealthLevel::Critical,
                    auth_server_reachable: false,
                    last_connected_at: None,
                };

                let database = DatabaseHealth {
                    status: HealthLevel::Unknown,
                    size_bytes: None,
                    last_write_at: None,
                };

                (subscription, network, database)
            }
        }
    }

    /// 以 Server 模式启动
    ///
    /// 如果已经在 Server 模式，直接返回成功（幂等操作）
    pub async fn start_server_mode(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 如果已经在 Server 模式，直接返回成功
        if matches!(&*mode_guard, ClientMode::Server { .. }) {
            tracing::debug!("Already in Server mode, skipping start");
            return Ok(());
        }

        // 如果在 Client 模式，先停止再切换
        if matches!(&*mode_guard, ClientMode::Client { .. }) {
            tracing::debug!("Stopping Client mode to switch to Server mode");
            *mode_guard = ClientMode::Disconnected;
        }

        let config = self.config.read().await;
        let server_config = &config.server_config;

        let tenant_manager = self.tenant_manager.read().await;
        let work_dir = if let Some(path) = tenant_manager.current_tenant_path() {
            // Server work_dir is {tenant}/server/
            let server_dir = path.join("server");
            tracing::debug!(path = %server_dir.display(), "Using server directory");
            server_dir.to_string_lossy().to_string()
        } else {
            tracing::warn!(
                "No active tenant, falling back to default data dir: {:?}",
                server_config.data_dir
            );
            server_config.data_dir.to_string_lossy().to_string()
        };
        drop(tenant_manager);

        let edge_config = edge_server::Config::builder()
            .work_dir(work_dir)
            .http_port(server_config.http_port)
            .message_tcp_port(server_config.message_port)
            .build();

        let server_state = edge_server::ServerState::initialize(&edge_config).await;

        let server_instance =
            edge_server::Server::with_state(edge_config.clone(), server_state.clone());
        let shutdown_token = server_instance.shutdown_token();

        let server_task = tokio::spawn(async move {
            if let Err(e) = server_instance.run().await {
                tracing::error!("Server run error: {}", e);
            }
        });

        let state_arc = Arc::new(server_state);

        let router = state_arc
            .https_service()
            .router()
            .ok_or_else(|| {
                tracing::error!("Router is None after ServerState initialization");
                BridgeError::Server("Router not initialized".to_string())
            })?;

        let message_bus = state_arc.message_bus();
        let client_tx = message_bus.sender_to_server().clone();
        let server_tx = message_bus.sender().clone();

        // 启动消息广播订阅 (转发给前端)
        let listener_task = if let Some(handle) = &self.app_handle {
            let mut server_rx = message_bus.subscribe();
            let handle_clone = handle.clone();
            let listener_token = shutdown_token.clone();

            let handle = tokio::spawn(async move {
                tracing::debug!("Server message listener started");
                loop {
                    tokio::select! {
                        _ = listener_token.cancelled() => {
                            tracing::debug!("Server message listener shutdown");
                            break;
                        }
                        result = server_rx.recv() => {
                            match result {
                                Ok(msg) => {
                                    // Route messages to appropriate channels
                                    use crate::events::MessageRoute;
                                    match MessageRoute::from_bus_message(msg) {
                                        MessageRoute::OrderSync(order_sync) => {
                                            if let Err(e) = handle_clone.emit("order-sync", &*order_sync) {
                                                tracing::warn!("Failed to emit order sync: {}", e);
                                            }
                                        }
                                        MessageRoute::ServerMessage(event) => {
                                            tracing::debug!(event_type = %event.event_type, "Emitting server-message");
                                            if let Err(e) = handle_clone.emit("server-message", &event) {
                                                tracing::warn!("Failed to emit server message: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    tracing::warn!("Server message listener lagged {} messages", n);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    tracing::debug!("Server message channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }
            });
            Some(handle)
        } else {
            None
        };

        let client = CrabClient::local()
            .with_router(router)
            .with_message_channels(client_tx, server_tx)
            .build()?;

        let connected_client = client.connect().await?;

        // 尝试加载缓存的员工会话
        let tenant_manager_read = self.tenant_manager.read().await;
        let cached_session = tenant_manager_read.load_current_session().ok().flatten();
        drop(tenant_manager_read);

        let client_state = if let Some(session) = cached_session {
            tracing::debug!(username = %session.username, "Restoring cached session");
            match connected_client
                .restore_session(session.token.clone(), session.user_info.clone())
                .await
            {
                Ok(authenticated_client) => {
                    tracing::debug!(username = %session.username, "Session restored");
                    let mut tenant_manager = self.tenant_manager.write().await;
                    tenant_manager.set_current_session(session);
                    LocalClientState::Authenticated(authenticated_client)
                }
                Err(e) => {
                    tracing::warn!("Failed to restore session: {}", e);
                    let tenant_manager = self.tenant_manager.read().await;
                    let _ = tenant_manager.clear_current_session();
                    let client = CrabClient::local()
                        .with_router(
                            state_arc
                                .https_service()
                                .router()
                                .ok_or_else(|| BridgeError::Server("Router not initialized".to_string()))?,
                        )
                        .with_message_channels(
                            state_arc.message_bus().sender_to_server().clone(),
                            state_arc.message_bus().sender().clone(),
                        )
                        .build()?;
                    LocalClientState::Connected(client.connect().await?)
                }
            }
        } else {
            tracing::debug!("No cached employee session found");
            LocalClientState::Connected(connected_client)
        };

        *mode_guard = ClientMode::Server {
            server_state: state_arc,
            client: Some(client_state),
            server_task,
            listener_task,
            shutdown_token,
        };

        let http_port = server_config.http_port;
        drop(config);
        {
            let mut config = self.config.write().await;
            config.current_mode = ModeType::Server;
        }
        self.save_config().await?;

        tracing::info!(port = http_port, "Server mode started");
        Ok(())
    }

    /// 以 Client 模式连接
    pub async fn start_client_mode(
        self: &Arc<Self>,
        edge_url: &str,
        message_addr: &str,
    ) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 如果在其他模式，先停止
        if let ClientMode::Server { shutdown_token, .. } = &*mode_guard {
            tracing::info!("Stopping Server mode to switch to Client mode...");
            shutdown_token.cancel();
            let old_mode = std::mem::replace(&mut *mode_guard, ClientMode::Disconnected);
            drop(mode_guard);

            if let ClientMode::Server {
                server_task,
                listener_task,
                ..
            } = old_mode
            {
                let server_abort = server_task.abort_handle();
                let listener_abort = listener_task.as_ref().map(|lt| lt.abort_handle());

                match tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    let server_result = server_task.await;
                    if let Some(lt) = listener_task {
                        let _ = lt.await;
                    }
                    server_result
                })
                .await
                {
                    Ok(Ok(())) => tracing::info!("Server tasks completed gracefully"),
                    Ok(Err(e)) if e.is_cancelled() => tracing::info!("Server task cancelled"),
                    Ok(Err(e)) => tracing::error!("Server task panicked: {}", e),
                    Err(_) => {
                        tracing::warn!("Server shutdown timed out (10s), aborting remaining tasks");
                        server_abort.abort();
                        if let Some(la) = listener_abort {
                            la.abort();
                        }
                    }
                }
            }
        } else if let ClientMode::Client { shutdown_token, .. } = &*mode_guard {
            tracing::info!("Already in Client mode, stopping first...");
            shutdown_token.cancel();
            *mode_guard = ClientMode::Disconnected;
            drop(mode_guard);
        } else {
            drop(mode_guard);
        }

        let tenant_manager = self.tenant_manager.read().await;
        let paths = tenant_manager
            .current_paths()
            .ok_or(TenantError::NoTenantSelected)?;

        let config = self.config.read().await;
        let auth_url = config
            .client_config
            .as_ref()
            .map(|c| c.auth_url.clone())
            .unwrap_or_else(|| "https://auth.example.com".to_string());
        drop(config);

        if !paths.has_client_certificates() {
            return Err(BridgeError::Config(
                "No cached certificates. Please activate tenant first.".into(),
            ));
        }

        // CrabClient 使用 cert_path + client_name 构建 CertManager
        // 我们传 certs_dir 作为 cert_path，空字符串作为 client_name
        // 这样 CertManager 会在 {tenant}/certs/ 查找证书
        // 握手时 CrabClient 会自动从证书中读取正确的 name
        let client = CrabClient::remote()
            .auth_server(&auth_url)
            .edge_server(edge_url)
            .cert_path(paths.certs_dir())
            .client_name("") // 空字符串使 CertManager 直接使用 certs_dir
            .build()?;

        let connected_client = client.connect_with_credentials(message_addr).await?;

        tracing::info!(edge_url = %edge_url, message_addr = %message_addr, "Client mode connected");

        let client_shutdown_token = tokio_util::sync::CancellationToken::new();

        // 启动消息广播订阅 (转发给前端)
        if let Some(handle) = &self.app_handle {
            if let Some(mc) = connected_client.message_client() {
                // 消息监听
                let mut rx = mc.subscribe();
                let handle_clone = handle.clone();
                let token = client_shutdown_token.clone();

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                tracing::info!("Client message listener shutdown");
                                break;
                            }
                            result = rx.recv() => {
                                match result {
                                    Ok(msg) => {
                                        use crate::events::MessageRoute;
                                        match MessageRoute::from_bus_message(msg) {
                                            MessageRoute::OrderSync(order_sync) => {
                                                if let Err(e) =
                                                    handle_clone.emit("order-sync", &*order_sync)
                                                {
                                                    tracing::warn!("Failed to emit order sync: {}", e);
                                                }
                                            }
                                            MessageRoute::ServerMessage(event) => {
                                                if let Err(e) = handle_clone.emit("server-message", &event)
                                                {
                                                    tracing::warn!("Failed to emit server message: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("Client message listener lagged {} messages", n);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        tracing::debug!("Client message channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::info!("Client message listener started");

                // 重连事件监听 (心跳失败或网络断开时触发)
                let mut reconnect_rx = mc.subscribe_reconnect();
                let handle_reconnect = handle.clone();
                let token = client_shutdown_token.clone();

                // 获取 bridge 自身引用（用于 ReconnectFailed 时触发重建）
                let bridge_for_rebuild = Arc::clone(self);

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                tracing::info!("Client reconnect listener shutdown");
                                break;
                            }
                            result = reconnect_rx.recv() => {
                                match result {
                                    Ok(event) => {
                                        use crab_client::ReconnectEvent;
                                        match event {
                                            ReconnectEvent::Disconnected => {
                                                tracing::warn!("Client disconnected, waiting for reconnection...");
                                                if let Err(e) = handle_reconnect.emit("connection-state-changed", false) {
                                                    tracing::warn!("Failed to emit connection state: {}", e);
                                                }
                                            }
                                            ReconnectEvent::Reconnected => {
                                                tracing::info!("Client reconnected successfully");
                                                if let Err(e) = handle_reconnect.emit("connection-state-changed", true) {
                                                    tracing::warn!("Failed to emit connection state: {}", e);
                                                }
                                            }
                                            ReconnectEvent::ReconnectFailed { attempts } => {
                                                tracing::error!("Client reconnection failed after {} attempts, triggering bridge rebuild", attempts);
                                                if let Err(e) = handle_reconnect.emit("connection-state-changed", false) {
                                                    tracing::warn!("Failed to emit connection state: {}", e);
                                                }

                                                // 在独立 task 中执行重建
                                                let bridge_arc = Arc::clone(&bridge_for_rebuild);
                                                let rebuild_handle = handle_reconnect.clone();
                                                tauri::async_runtime::spawn(async move {
                                                    do_rebuild_connection(bridge_arc, rebuild_handle).await;
                                                });

                                                // 退出此监听器（start_client_mode 会创建新的监听器）
                                                break;
                                            }
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("Reconnect event listener lagged {} events", n);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        tracing::debug!("Reconnect event channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::info!("Client reconnection listener started");

                // 心跳状态监听 (每次心跳成功/失败都会触发)
                let mut heartbeat_rx = mc.subscribe_heartbeat();
                let handle_heartbeat = handle.clone();
                let token = client_shutdown_token.clone();

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                tracing::info!("Client heartbeat listener shutdown");
                                break;
                            }
                            result = heartbeat_rx.recv() => {
                                match result {
                                    Ok(status) => {
                                        if let Err(e) = handle_heartbeat.emit("heartbeat-status", &status) {
                                            tracing::warn!("Failed to emit heartbeat status: {}", e);
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("Heartbeat listener lagged {} events", n);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        tracing::debug!("Heartbeat channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::info!("Client heartbeat listener started");
            }
        }

        {
            let mut mode_guard = self.mode.write().await;
            if !matches!(&*mode_guard, ClientMode::Disconnected) {
                tracing::warn!("Mode changed during Client setup, aborting");
                client_shutdown_token.cancel();
                return Err(BridgeError::Server(
                    "Mode changed during client setup".to_string(),
                ));
            }
            *mode_guard = ClientMode::Client {
                client: Some(RemoteClientState::Connected(connected_client)),
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
                shutdown_token: client_shutdown_token,
            };
        }

        {
            let mut config = self.config.write().await;
            config.current_mode = ModeType::Client;
            config.client_config = Some(ClientModeConfig {
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
                auth_url,
            });
        }
        self.save_config().await?;

        Ok(())
    }

    /// 停止当前模式（优雅关闭）
    ///
    /// Server 模式: cancel shutdown_token → 等待 server_task + listener_task（10s 超时）
    /// Client 模式: cancel shutdown_token → 监听器自行退出
    pub async fn stop(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 1. 发送 graceful shutdown 信号
        match &*mode_guard {
            ClientMode::Server { shutdown_token, .. } => {
                shutdown_token.cancel();
                tracing::info!("Server shutdown signal sent, waiting for tasks to stop...");
            }
            ClientMode::Client { shutdown_token, .. } => {
                shutdown_token.cancel();
                tracing::info!("Client shutdown signal sent");
            }
            ClientMode::Disconnected => {}
        }

        // 2. 取出 mode（move ownership of server_task 才能 await）
        let old_mode = std::mem::replace(&mut *mode_guard, ClientMode::Disconnected);
        drop(mode_guard);

        // 3. 等待 server_task + listener_task 完成（10s 超时保底）
        if let ClientMode::Server {
            server_task,
            listener_task,
            ..
        } = old_mode
        {
            let server_abort = server_task.abort_handle();
            let listener_abort = listener_task.as_ref().map(|lt| lt.abort_handle());

            match tokio::time::timeout(std::time::Duration::from_secs(10), async {
                // 并行等待 server_task 和 listener_task
                let server_result = server_task.await;
                if let Some(lt) = listener_task {
                    let _ = lt.await;
                }
                server_result
            })
            .await
            {
                Ok(Ok(())) => tracing::info!("Server tasks completed gracefully"),
                Ok(Err(e)) if e.is_cancelled() => tracing::info!("Server task cancelled"),
                Ok(Err(e)) => tracing::error!("Server task panicked: {}", e),
                Err(_) => {
                    tracing::warn!("Server shutdown timed out (10s), aborting remaining tasks");
                    server_abort.abort();
                    if let Some(la) = listener_abort {
                        la.abort();
                    }
                }
            }
        }

        // 4. 更新配置
        {
            let mut config = self.config.write().await;
            config.current_mode = ModeType::Disconnected;
        }
        self.save_config().await?;

        tracing::info!("Mode stopped, now disconnected");

        Ok(())
    }

    // ============ Client 模式连接重建 ============

    /// 从当前 `ClientMode::Client` 读取连接参数，销毁旧 client 并重新连接。
    ///
    /// 仅在 Client 模式下有效，复用 `start_client_mode` 的逻辑。
    /// 返回 boxed future 显式标注 `Send`，
    /// 打破 start_client_mode → spawn(do_rebuild_connection) → rebuild_client_connection → start_client_mode
    /// 的递归 opaque type 循环。
    pub fn rebuild_client_connection(
        self: &Arc<Self>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), BridgeError>> + Send + '_>>
    {
        Box::pin(async move {
            let (edge_url, message_addr) = {
                let guard = self.mode.read().await;
                match &*guard {
                    ClientMode::Client {
                        edge_url,
                        message_addr,
                        ..
                    } => (edge_url.clone(), message_addr.clone()),
                    _ => return Err(BridgeError::NotInitialized),
                }
            };

            tracing::info!(
                edge_url = %edge_url,
                message_addr = %message_addr,
                "Rebuilding client connection..."
            );

            self.start_client_mode(&edge_url, &message_addr).await
        })
    }

    /// 退出当前租户：停止服务器 → 清除当前租户选择（保留文件）
    pub async fn exit_tenant(&self) -> Result<(), BridgeError> {
        let tenant_id = {
            let tm = self.tenant_manager.read().await;
            tm.current_tenant_id().map(|s| s.to_string())
        };

        let Some(tenant_id) = tenant_id else {
            return Err(BridgeError::Config("No current tenant".to_string()));
        };

        // 1. 停止服务器模式（切换到 Disconnected）
        self.stop().await?;

        // 2. 清除当前租户选择（不删除文件）
        {
            let mut tm = self.tenant_manager.write().await;
            tm.clear_current_tenant();
        }

        // 3. 清除配置中的当前租户
        {
            let mut config = self.config.write().await;
            config.current_tenant = None;
            config.save(&self.config_path)?;
        }

        tracing::info!(tenant_id = %tenant_id, "Exited tenant (files preserved)");
        Ok(())
    }

    // ============ 租户管理代理 ============

    /// 获取租户管理器的只读引用
    pub fn tenant_manager(&self) -> &Arc<RwLock<TenantManager>> {
        &self.tenant_manager
    }

    /// 获取服务器模式配置
    pub async fn get_server_config(&self) -> ServerModeConfig {
        self.config.read().await.server_config.clone()
    }

    /// 获取客户端模式配置
    pub async fn get_client_config(&self) -> Option<ClientModeConfig> {
        self.config.read().await.client_config.clone()
    }

    /// 更新 Server 模式配置 (端口配置)
    ///
    /// 仅更新配置并保存，不启动模式
    pub async fn update_server_config(
        &self,
        http_port: u16,
        message_port: u16,
    ) -> Result<(), BridgeError> {
        {
            let mut config = self.config.write().await;
            config.server_config.http_port = http_port;
            config.server_config.message_port = message_port;
            config.save(&self.config_path)?;
        }
        tracing::info!(http_port = %http_port, message_port = %message_port, "Server config updated");
        Ok(())
    }

    /// 更新 Client 模式配置 (连接地址)
    ///
    /// 仅更新配置并保存，不启动模式
    pub async fn update_client_config(
        &self,
        edge_url: &str,
        message_addr: &str,
        auth_url: &str,
    ) -> Result<(), BridgeError> {
        {
            let mut config = self.config.write().await;
            config.client_config = Some(ClientModeConfig {
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
                auth_url: auth_url.to_string(),
            });
            config.save(&self.config_path)?;
        }
        tracing::info!(edge_url = %edge_url, message_addr = %message_addr, auth_url = %auth_url, "Client config updated");
        Ok(())
    }

    /// 获取 Client 模式的 mTLS HTTP client 和相关信息
    ///
    /// 返回 (edge_url, http_client, token) 用于需要直接访问 EdgeServer 的场景 (如图片上传)
    /// Server 模式或未认证时返回 None
    pub async fn get_edge_http_context(&self) -> Option<(String, reqwest::Client, String)> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Client {
                client: Some(RemoteClientState::Authenticated(auth)),
                edge_url,
                ..
            } => {
                let http = auth.edge_http_client()?.clone();
                let token = auth.token()?.to_string();
                Some((edge_url.clone(), http, token))
            }
            _ => None,
        }
    }

    // ============ 员工认证 ============

    /// 员工登录 (使用 CrabClient)
    pub async fn login_employee(
        &self,
        username: &str,
        password: &str,
    ) -> Result<super::session_cache::EmployeeSession, BridgeError> {
        let mut mode_guard = self.mode.write().await;

        let result = match &mut *mode_guard {
            ClientMode::Server {
                server_state: _,
                client,
                ..
            } => {
                let current_client = client.take().ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    LocalClientState::Connected(connected) => {
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();
                                let expires_at =
                                    super::session_cache::EmployeeSession::parse_jwt_exp(&token);

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at,
                                    logged_in_at: shared::util::now_millis(),
                                };

                                *client = Some(LocalClientState::Authenticated(authenticated));

                                tracing::debug!(username = %username, "Employee logged in via CrabClient (local)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                // 登录失败，恢复 Connected 状态
                                *client = Some(LocalClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                    LocalClientState::Authenticated(auth) => {
                        let connected = auth.logout().await;
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();
                                let expires_at =
                                    super::session_cache::EmployeeSession::parse_jwt_exp(&token);

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at,
                                    logged_in_at: shared::util::now_millis(),
                                };

                                *client = Some(LocalClientState::Authenticated(authenticated));
                                tracing::debug!(username = %username, "Employee re-logged in via CrabClient (local)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                // 登录失败，恢复 Connected 状态
                                *client = Some(LocalClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                }
            }
            ClientMode::Client { client, .. } => {
                let current_client = client.take().ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    RemoteClientState::Connected(connected) => {
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();
                                let expires_at =
                                    super::session_cache::EmployeeSession::parse_jwt_exp(&token);

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at,
                                    logged_in_at: shared::util::now_millis(),
                                };

                                *client = Some(RemoteClientState::Authenticated(authenticated));
                                tracing::debug!(username = %username, "Employee logged in via CrabClient (remote)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                // 登录失败，恢复 Connected 状态
                                *client = Some(RemoteClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                    RemoteClientState::Authenticated(auth) => {
                        let connected = auth.logout().await;
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                let user_info = authenticated.me().cloned().ok_or_else(|| {
                                    BridgeError::Client(crab_client::ClientError::Auth(
                                        "No user info after login".into(),
                                    ))
                                })?;
                                let token = authenticated.token().unwrap_or_default().to_string();
                                let expires_at =
                                    super::session_cache::EmployeeSession::parse_jwt_exp(&token);

                                let session = super::session_cache::EmployeeSession {
                                    username: username.to_string(),
                                    token,
                                    user_info,
                                    login_mode: super::session_cache::LoginMode::Online,
                                    expires_at,
                                    logged_in_at: shared::util::now_millis(),
                                };

                                *client = Some(RemoteClientState::Authenticated(authenticated));
                                tracing::debug!(username = %username, "Employee re-logged in via CrabClient (remote)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                // 登录失败，恢复 Connected 状态
                                *client = Some(RemoteClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                }
            }
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        };

        drop(mode_guard);

        if let Ok(ref session) = result {
            // 1. 保存到磁盘
            {
                let tenant_manager = self.tenant_manager.read().await;
                if let Err(e) = tenant_manager.save_current_session(session) {
                    tracing::warn!("Failed to persist session: {}", e);
                }
            }
            // 2. 更新内存中的 current_session
            {
                let mut tenant_manager = self.tenant_manager.write().await;
                tenant_manager.set_current_session(session.clone());
            }
        }

        result
    }

    /// 员工登出
    pub async fn logout_employee(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        match &mut *mode_guard {
            ClientMode::Server {
                server_state: _,
                client,
                ..
            } => {
                if let Some(current_client) = client.take() {
                    match current_client {
                        LocalClientState::Authenticated(auth) => {
                            let connected = auth.logout().await;
                            *client = Some(LocalClientState::Connected(connected));
                            tracing::debug!("Employee logged out (local)");
                        }
                        LocalClientState::Connected(c) => {
                            *client = Some(LocalClientState::Connected(c));
                        }
                    }
                }
            }
            ClientMode::Client { client, .. } => {
                if let Some(current_client) = client.take() {
                    match current_client {
                        RemoteClientState::Authenticated(auth) => {
                            let connected = auth.logout().await;
                            *client = Some(RemoteClientState::Connected(connected));
                            tracing::debug!("Employee logged out (remote)");
                        }
                        RemoteClientState::Connected(c) => {
                            *client = Some(RemoteClientState::Connected(c));
                        }
                    }
                }
            }
            ClientMode::Disconnected => {}
        }

        drop(mode_guard);

        let tenant_manager = self.tenant_manager.read().await;
        if let Err(e) = tenant_manager.clear_current_session() {
            tracing::warn!("Failed to clear cached session: {}", e);
        }

        Ok(())
    }

    // ============ 统一业务 API ============

    /// 通用 GET 请求 (使用 CrabClient)
    pub async fn get<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.get(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.get(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 POST 请求 (使用 CrabClient)
    pub async fn post<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.post(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.post(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 PUT 请求 (使用 CrabClient)
    pub async fn put<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.put(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.put(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 DELETE 请求 (使用 CrabClient)
    pub async fn delete<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.delete(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.delete(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 DELETE 请求 (带 body)
    pub async fn delete_with_body<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => auth
                    .delete_with_body(path, body)
                    .await
                    .map_err(BridgeError::Client),
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => auth
                    .delete_with_body(path, body)
                    .await
                    .map_err(BridgeError::Client),
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    // ============ Order Event Sourcing API ============

    /// Execute an order command (event sourcing)
    pub async fn execute_order_command(
        &self,
        command: OrderCommand,
    ) -> Result<CommandResponse, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // 保存需要加载规则的命令信息
                let open_table_info = if let OrderCommandPayload::OpenTable {
                    zone_id, is_retail, ..
                } = &command.payload
                {
                    Some((zone_id.clone(), *is_retail))
                } else {
                    None
                };
                let move_order_info = if let OrderCommandPayload::MoveOrder {
                    order_id,
                    target_zone_id,
                    ..
                } = &command.payload
                {
                    Some((order_id.clone(), target_zone_id.clone()))
                } else {
                    None
                };

                let (response, events) = server_state
                    .orders_manager()
                    .execute_command_with_events(command);

                if response.success {
                    // OpenTable 成功后加载并缓存价格规则
                    if let Some((zone_id, is_retail)) = open_table_info {
                        if let Some(ref order_id) = response.order_id {
                            let rules = edge_server::orders::actions::open_table::load_matching_rules(
                                &server_state.get_db(),
                                zone_id.as_deref(),
                                is_retail,
                            )
                            .await;

                            if !rules.is_empty() {
                                tracing::debug!(
                                    order_id = %order_id,
                                    rule_count = rules.len(),
                                    "缓存订单价格规则 (Server 模式)"
                                );
                                server_state.orders_manager().cache_rules(order_id, rules);
                            }
                        }
                    }

                    // MoveOrder 成功后：用新区域重新加载规则
                    if let Some((ref order_id, ref target_zone_id)) = move_order_info {
                        if let Ok(Some(snapshot)) = server_state.orders_manager().get_snapshot(order_id) {
                            let rules = edge_server::orders::actions::open_table::load_matching_rules(
                                &server_state.get_db(),
                                target_zone_id.as_deref(),
                                snapshot.is_retail,
                            )
                            .await;
                            tracing::debug!(
                                order_id = %order_id,
                                target_zone_id = ?target_zone_id,
                                rule_count = rules.len(),
                                "移桌后重新加载区域规则 (Server 模式)"
                            );
                            server_state.orders_manager().cache_rules(order_id, rules);
                        }
                    }
                }

                if let Some(handle) = &self.app_handle {
                    for event in events {
                        if let Err(e) = handle.emit("order-event", &event) {
                            tracing::warn!("Failed to emit order event: {}", e);
                        }
                    }
                }

                Ok(response)
            }
            ClientMode::Client { client, .. } => {
                // Send command via MessageBus RequestCommand protocol
                match client {
                    Some(RemoteClientState::Authenticated(auth)) => {
                        // Map OrderCommand payload type to action string
                        let action = match &command.payload {
                            shared::order::OrderCommandPayload::OpenTable { .. } => {
                                "order.open_table"
                            }
                            shared::order::OrderCommandPayload::CompleteOrder { .. } => {
                                "order.complete"
                            }
                            shared::order::OrderCommandPayload::VoidOrder { .. } => "order.void",
                            shared::order::OrderCommandPayload::AddItems { .. } => {
                                "order.add_items"
                            }
                            shared::order::OrderCommandPayload::ModifyItem { .. } => {
                                "order.modify_item"
                            }
                            shared::order::OrderCommandPayload::RemoveItem { .. } => {
                                "order.remove_item"
                            }
                            shared::order::OrderCommandPayload::AddPayment { .. } => {
                                "order.add_payment"
                            }
                            shared::order::OrderCommandPayload::CancelPayment { .. } => {
                                "order.cancel_payment"
                            }
                            shared::order::OrderCommandPayload::SplitByItems { .. } => {
                                "order.split_by_items"
                            }
                            shared::order::OrderCommandPayload::SplitByAmount { .. } => {
                                "order.split_by_amount"
                            }
                            shared::order::OrderCommandPayload::StartAaSplit { .. } => {
                                "order.start_aa_split"
                            }
                            shared::order::OrderCommandPayload::PayAaSplit { .. } => {
                                "order.pay_aa_split"
                            }
                            shared::order::OrderCommandPayload::MoveOrder { .. } => "order.move",
                            shared::order::OrderCommandPayload::MergeOrders { .. } => "order.merge",
                            shared::order::OrderCommandPayload::UpdateOrderInfo { .. } => {
                                "order.update_info"
                            }
                            shared::order::OrderCommandPayload::ToggleRuleSkip { .. } => {
                                "order.toggle_rule_skip"
                            }
                            shared::order::OrderCommandPayload::ApplyOrderDiscount { .. } => {
                                "order.apply_order_discount"
                            }
                            shared::order::OrderCommandPayload::ApplyOrderSurcharge { .. } => {
                                "order.apply_order_surcharge"
                            }
                            shared::order::OrderCommandPayload::CompItem { .. } => {
                                "order.comp_item"
                            }
                            shared::order::OrderCommandPayload::UncompItem { .. } => {
                                "order.uncomp_item"
                            }
                            shared::order::OrderCommandPayload::AddOrderNote { .. } => {
                                "order.add_order_note"
                            }
                        };

                        // Build RequestCommand message with full command (preserves command_id, operator info)
                        let request_payload = shared::message::RequestCommandPayload {
                            action: action.to_string(),
                            params: serde_json::to_value(&command).ok(),
                        };
                        let request_msg =
                            shared::message::BusMessage::request_command(&request_payload);

                        // Send via MessageClient and wait for response
                        let response_msg = auth
                            .request(&request_msg)
                            .await
                            .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                        // Parse response
                        let response_payload: shared::message::ResponsePayload = response_msg
                            .parse_payload()
                            .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                        if response_payload.success {
                            // Extract CommandResponse from data if present
                            if let Some(data) = response_payload.data {
                                let cmd_response: CommandResponse = serde_json::from_value(data)
                                    .unwrap_or_else(|_| CommandResponse {
                                        command_id: command.command_id.clone(),
                                        success: true,
                                        order_id: None,
                                        error: None,
                                    });
                                Ok(cmd_response)
                            } else {
                                Ok(CommandResponse {
                                    command_id: command.command_id,
                                    success: true,
                                    order_id: None,
                                    error: None,
                                })
                            }
                        } else {
                            Ok(CommandResponse {
                                command_id: command.command_id,
                                success: false,
                                order_id: None,
                                error: Some(shared::order::CommandError::new(
                                    shared::order::CommandErrorCode::InternalError,
                                    response_payload.message,
                                )),
                            })
                        }
                    }
                    Some(RemoteClientState::Connected(_)) => Err(BridgeError::NotAuthenticated),
                    None => Err(BridgeError::NotInitialized),
                }
            }
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get all active order snapshots (event sourcing)
    pub async fn get_active_orders(&self) -> Result<Vec<OrderSnapshot>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .get_active_orders()
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.orders request to get active orders
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.orders".to_string(),
                        params: Some(serde_json::json!({ "since_sequence": 0 })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let sync_response: SyncResponse = serde_json::from_value(data)
                                .map_err(|e| {
                                    BridgeError::Server(format!("Invalid sync response: {}", e))
                                })?;
                            Ok(sync_response.active_orders)
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get a single order snapshot by ID
    pub async fn get_order_snapshot(
        &self,
        order_id: &str,
    ) -> Result<Option<OrderSnapshot>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .get_snapshot(order_id)
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.order_snapshot request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.order_snapshot".to_string(),
                        params: Some(serde_json::json!({ "order_id": order_id })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let snapshot: OrderSnapshot =
                                serde_json::from_value(data).map_err(|e| {
                                    BridgeError::Server(format!("Invalid snapshot: {}", e))
                                })?;
                            Ok(Some(snapshot))
                        } else {
                            Ok(None)
                        }
                    } else {
                        // Not found is not an error, just return None
                        if response_payload.error_code.as_deref() == Some("NOT_FOUND") {
                            Ok(None)
                        } else {
                            Err(BridgeError::Server(response_payload.message))
                        }
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Sync orders since a given sequence (for reconnection)
    pub async fn sync_orders_since(
        &self,
        since_sequence: u64,
    ) -> Result<SyncResponse, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                let orders_manager = server_state.orders_manager();

                let events = orders_manager
                    .get_events_since(since_sequence)
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                let active_orders = orders_manager
                    .get_active_orders()
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                let server_sequence = orders_manager
                    .get_current_sequence()
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                Ok(SyncResponse {
                    events,
                    active_orders,
                    server_sequence,
                    requires_full_sync: since_sequence == 0,
                })
            }
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.orders request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.orders".to_string(),
                        params: Some(serde_json::json!({ "since_sequence": since_sequence })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let sync_response: SyncResponse = serde_json::from_value(data)
                                .map_err(|e| {
                                    BridgeError::Server(format!("Invalid sync response: {}", e))
                                })?;
                            Ok(sync_response)
                        } else {
                            Ok(SyncResponse {
                                events: vec![],
                                active_orders: vec![],
                                server_sequence: 0,
                                requires_full_sync: true,
                            })
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get events for active orders since a given sequence
    pub async fn get_active_events_since(
        &self,
        since_sequence: u64,
    ) -> Result<Vec<OrderEvent>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .get_active_events_since(since_sequence)
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.active_events request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.active_events".to_string(),
                        params: Some(serde_json::json!({ "since_sequence": since_sequence })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let events: Vec<OrderEvent> =
                                serde_json::from_value(data).map_err(|e| {
                                    BridgeError::Server(format!("Invalid events: {}", e))
                                })?;
                            Ok(events)
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get all events for a specific order (event sourcing)
    ///
    /// Used to reconstruct full order history including timeline.
    pub async fn get_events_for_order(
        &self,
        order_id: &str,
    ) -> Result<Vec<OrderEvent>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .storage()
                .get_events_for_order(order_id)
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.order_events request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.order_events".to_string(),
                        params: Some(serde_json::json!({ "order_id": order_id })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let events: Vec<OrderEvent> =
                                serde_json::from_value(data).map_err(|e| {
                                    BridgeError::Server(format!("Invalid events: {}", e))
                                })?;
                            Ok(events)
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }
}

// ============================================================================
// 独立重建函数（在 tokio::spawn 中使用，避免 Send 问题）
// ============================================================================

/// Client 模式连接重建（限次 + 指数退避）。
///
/// 当 `NetworkMessageClient` 的 `reconnect_loop` 耗尽所有重连尝试后，
/// bridge 层会调用此函数进行更高层级的重建：销毁旧 client，重新执行
/// `start_client_mode` 建立全新连接。
///
/// - 最多 5 次重建，每次间隔指数退避 (5s → 10s → 20s → 40s → 80s)
/// - 每次重建内部 CrabClient 会持续网络重连
/// - 全部失败后切换到 `ClientMode::Disconnected`，通知前端
async fn do_rebuild_connection(bridge: Arc<ClientBridge>, app_handle: tauri::AppHandle) {
    const MAX_REBUILDS: u32 = 5;
    let base_delay = std::time::Duration::from_secs(5);
    let mut delay = base_delay;

    for attempt in 1..=MAX_REBUILDS {
        tracing::info!(
            attempt,
            MAX_REBUILDS,
            delay_secs = delay.as_secs(),
            "Bridge rebuild attempt"
        );

        tokio::time::sleep(delay).await;

        let result = bridge.rebuild_client_connection().await;

        match result {
            Ok(()) => {
                tracing::info!(attempt, "Bridge rebuild succeeded");
                return;
            }
            Err(e) => {
                tracing::warn!(attempt, MAX_REBUILDS, error = %e, "Bridge rebuild failed");
            }
        }

        delay *= 2;
    }

    // 全部失败：切换到 Disconnected
    tracing::error!(
        MAX_REBUILDS,
        "All bridge rebuild attempts exhausted, switching to Disconnected"
    );
    {
        let mut guard = bridge.mode.write().await;
        if let ClientMode::Client { shutdown_token, .. } = &*guard {
            shutdown_token.cancel();
        }
        *guard = ClientMode::Disconnected;
    }

    let _ = app_handle.emit("connection-state-changed", false);
    let _ = app_handle.emit("connection-permanently-lost", true);
}

