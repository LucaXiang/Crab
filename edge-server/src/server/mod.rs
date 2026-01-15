pub mod auth;
mod config;
mod error;
pub mod middleware;
pub mod services;
mod state;

pub use auth::{CurrentUser, JwtConfig, JwtService};
pub use config::Config;
pub use error::Result;
pub use services::ProvisioningService;
pub use services::credential;
pub use services::provisioning;
pub use state::ServerState;

pub struct Server {
    config: Config,
    state: Option<ServerState>,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: None,
        }
    }

    /// 使用现有状态创建服务器 (用于与 oneshot 共享)
    pub fn with_state(config: Config, state: ServerState) -> Self {
        Self {
            config,
            state: Some(state),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // 如果未提供，则创建应用状态
        let state = match &self.state {
            Some(s) => s.clone(),
            None => ServerState::initialize(&self.config).await,
        };

        // 启动后台任务 (消息总线等)
        // 统一在此处启动，确保无论 state 是外部传入还是内部创建，后台任务都会运行
        state.start_background_tasks().await;

        // Note: https_service().initialize() is already called in ServerState::initialize()
        // so we don't need to call it again here.

        // 处理激活和潜在重新激活的循环
        loop {
            // 1. 等待激活
            state.wait_for_activation().await;

            // 2. 加载 mTLS 证书
            let tls_config = match state.load_tls_config() {
                Ok(Some(config)) => config,
                Ok(None) => {
                    tracing::error!(
                        "❌ Server activated but certificates missing! Resetting activation state..."
                    );
                    if let Err(e) = state.deactivate_and_reset().await {
                        tracing::error!("Failed to reset state: {}", e);
                    }
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        "❌ Certificate chain broken: {}. Resetting activation state...",
                        e
                    );
                    if let Err(e) = state.deactivate_and_reset().await {
                        tracing::error!("Failed to reset state: {}", e);
                    }
                    continue;
                }
            };

            // 3. 启动服务
            self.print_activation_banner(&state).await;
            tracing::info!("🔒 mTLS certificates loaded. Starting services...");

            // --- 统一启动服务 ---

            // A. 启动 TCP 消息总线服务器
            let tcp_tls_config = tls_config.clone();
            let bus = state.message_bus().clone();
            tokio::spawn(async move {
                tracing::info!("Starting Message Bus TCP server...");
                if let Err(e) = bus.start_tcp_server(Some(tcp_tls_config)).await {
                    tracing::error!("Message bus TCP server error: {}", e);
                }
            });

            // B. 启动 HTTPS 服务器
            let rustls_config = axum_server::tls_rustls::RustlsConfig::from_config(tls_config);

            state.print_activated_banner_content().await;

            state
                .https_service()
                .start_server(rustls_config, shutdown_signal())
                .await
                .map_err(|e| {
                    crate::server::error::ServerError::Internal(anyhow::anyhow!(
                        "Server error: {}",
                        e
                    ))
                })?;

            tracing::info!("✅ Server shutdown complete");
            return Ok(());
        }
    }

    async fn print_activation_banner(&self, state: &ServerState) {
        let activation = state
            .activation_service()
            .get_status()
            .await
            .unwrap_or_default();
        let tenant_id = activation
            .tenant_id
            .unwrap_or_else(|| "Unknown".to_string());
        let edge_id = activation.edge_id.unwrap_or_else(|| "Unknown".to_string());

        // Truncate long IDs for better display
        let display_edge_id = if edge_id.len() > 20 {
            format!("{}...", &edge_id[..17])
        } else {
            edge_id
        };

        let cert_fingerprint = activation
            .cert_fingerprint
            .unwrap_or_else(|| "Unknown".to_string());
        let display_fingerprint = if cert_fingerprint.len() > 20 {
            format!("{}...", &cert_fingerprint[..17])
        } else {
            cert_fingerprint
        };

        let sub_info = {
            let cache = state.activation_service().credential_cache.read().await;
            if let Some(cred) = &*cache {
                if let Some(sub) = &cred.subscription {
                    format!("{:?} ({:?})", sub.plan, sub.status)
                } else {
                    "No Subscription".to_string()
                }
            } else {
                "No Subscription".to_string()
            }
        };

        tracing::info!("");
        tracing::info!(
            "╔════════════════════════════════════════════════════════════════════════════╗"
        );
        tracing::info!(
            "║                   🦀 Crab Edge Server - Activated 🚀                       ║"
        );
        tracing::info!(
            "╠════════════════════════════════════════════════════════════════════════════╣"
        );
        tracing::info!("║ 🏢 Tenant ID       : {:<46} ║", tenant_id);
        tracing::info!("║ 🆔 Edge ID         : {:<46} ║", display_edge_id);
        tracing::info!("║ 📜 Cert Fingerprint: {:<46} ║", display_fingerprint);
        tracing::info!("║ 📦 Subscription    : {:<46} ║", sub_info);
        tracing::info!(
            "╠════════════════════════════════════════════════════════════════════════════╣"
        );
        tracing::info!(
            "║ 🌐 HTTPS Listener  : https://0.0.0.0:{:<30} ║",
            self.config.http_port
        );
        tracing::info!(
            "║ 📨 TCP Listener    : 0.0.0.0:{:<37} ║",
            self.config.message_tcp_port
        );
        tracing::info!(
            "╚════════════════════════════════════════════════════════════════════════════╝"
        );
        tracing::info!("");
    }
}

/// 优雅停机处理器
///
/// 监听 SIGTERM (Kubernetes) 和 Ctrl+C 信号
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM signal, shutting down gracefully...");
        },
    }
}
