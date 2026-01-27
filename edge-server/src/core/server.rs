//! Server Implementation
//!
//! HTTP 服务器启动和管理
//!
//! # 启动流程
//!
//! ```text
//! 1. ServerState::initialize()      - 初始化服务和数据库
//! 2. start_background_tasks()       - 启动无需 TLS 的后台任务
//! 3. wait_for_activation()          - 等待设备激活
//! 4. load_tls_config()              - 加载 mTLS 证书
//! 5. start_tls_tasks()              - 启动需要 TLS 的任务
//! 6. https.start_server()           - 启动 HTTPS 服务
//! 7. shutdown()                     - Graceful shutdown
//! ```

use crate::core::{Config, Result, ServerState};
use axum_server::tls_rustls::RustlsConfig;

/// HTTP Server
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

    /// Create server with existing state (for sharing with oneshot)
    pub fn with_state(config: Config, state: ServerState) -> Self {
        Self {
            config,
            state: Some(state),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // ═══════════════════════════════════════════════════════════════════
        // Phase 1: Initialize
        // ═══════════════════════════════════════════════════════════════════
        let state = match &self.state {
            Some(s) => s.clone(),
            None => ServerState::initialize(&self.config).await,
        };

        // ═══════════════════════════════════════════════════════════════════
        // Phase 2: Start background tasks (no TLS required)
        // ═══════════════════════════════════════════════════════════════════
        let mut background_tasks = state.start_background_tasks().await;

        // ═══════════════════════════════════════════════════════════════════
        // Phase 3: Wait for activation and load TLS
        // ═══════════════════════════════════════════════════════════════════
        let tls_config = self.wait_for_tls(&state).await;
        let rustls_config = RustlsConfig::from_config(tls_config.clone());

        // ═══════════════════════════════════════════════════════════════════
        // Phase 4: Start TLS-dependent tasks
        // ═══════════════════════════════════════════════════════════════════
        state.start_tls_tasks(&mut background_tasks, tls_config);
        state.print_activated_banner_content().await;

        // ═══════════════════════════════════════════════════════════════════
        // Phase 5: Start HTTPS server (blocks until shutdown)
        // ═══════════════════════════════════════════════════════════════════
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
        tracing::info!("🦀 Crab Edge Server starting on {}", addr);

        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("Shutting down...");
        };

        state
            .https
            .start_server(rustls_config, shutdown)
            .await
            .map_err(|e| crate::core::ServerError::Internal(e.into()))?;

        // ═══════════════════════════════════════════════════════════════════
        // Phase 6: Graceful shutdown
        // ═══════════════════════════════════════════════════════════════════
        background_tasks.shutdown().await;

        Ok(())
    }

    /// Wait for activation and load TLS config
    ///
    /// Blocks until device is activated and TLS certificates are loaded.
    /// Retries on failure by re-entering unbound state.
    async fn wait_for_tls(&self, state: &ServerState) -> std::sync::Arc<rustls::ServerConfig> {
        loop {
            state.wait_for_activation().await;

            match state.load_tls_config() {
                Ok(Some(cfg)) => return cfg,
                Ok(None) => {
                    tracing::error!("❌ TLS certificates not found after activation!");
                    state.enter_unbound_state().await;
                }
                Err(e) => {
                    tracing::error!("❌ Failed to load TLS config: {}. Entering unbound state.", e);
                    state.enter_unbound_state().await;
                }
            }
        }
    }
}
