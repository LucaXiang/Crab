//! Server Implementation
//!
//! HTTP 服务器启动和管理
//!
//! # 启动流程
//!
//! ```text
//! 1. ServerState::initialize()      - 初始化服务和数据库
//! 2. start_background_tasks()       - 启动无需 TLS 的后台任务
//! 3. wait_for_activation()          - 等待设备激活 + 加载 mTLS 证书
//! 4. subscription_check()           - 订阅阻止检查 (blocked → 60s 重试循环)
//! 5. start_tls_tasks()              - 启动需要 TLS 的任务
//! 6. https.start_server()           - 启动 HTTPS 服务
//! 7. shutdown()                     - Graceful shutdown
//! ```

use crate::core::{Config, ServerState};
use crate::utils::AppError;
use axum_server::tls_rustls::RustlsConfig;
use tokio_util::sync::CancellationToken;

/// HTTP Server
pub struct Server {
    config: Config,
    state: Option<ServerState>,
    shutdown_token: CancellationToken,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: None,
            shutdown_token: CancellationToken::new(),
        }
    }

    /// Create server with existing state (for sharing with oneshot)
    pub fn with_state(config: Config, state: ServerState) -> Self {
        Self {
            config,
            state: Some(state),
            shutdown_token: CancellationToken::new(),
        }
    }

    /// Get the shutdown token for external shutdown control
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    pub async fn run(&self) -> Result<(), AppError> {
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
        // Phase 3: Wait for activation and load TLS (可取消)
        // ═══════════════════════════════════════════════════════════════════
        let tls_config = match self.wait_for_tls(&state).await {
            Some(cfg) => cfg,
            None => {
                tracing::info!("Shutdown requested during activation wait");
                background_tasks.shutdown().await;
                return Ok(());
            }
        };
        let rustls_config = RustlsConfig::from_config(tls_config.clone());

        // ═══════════════════════════════════════════════════════════════════
        // Phase 4: Subscription check — 指数退避重试
        // ═══════════════════════════════════════════════════════════════════
        let mut retry_delay = std::time::Duration::from_secs(10);
        const MAX_DELAY: std::time::Duration = std::time::Duration::from_secs(300);

        while state.is_subscription_blocked().await {
            state.print_subscription_blocked_banner().await;

            tokio::select! {
                _ = self.shutdown_token.cancelled() => {
                    tracing::info!("Shutdown requested during subscription check");
                    background_tasks.shutdown().await;
                    return Ok(());
                }
                _ = tokio::time::sleep(retry_delay) => {}
            }

            // Re-sync subscription from auth-server
            state.sync_subscription().await;

            // 指数退避: 10s → 20s → 40s → 80s → 160s → 300s
            retry_delay = (retry_delay * 2).min(MAX_DELAY);
            tracing::info!("🔄 Re-checked subscription (next retry in {:?})", retry_delay);
        }
        tracing::info!("✅ Subscription OK, proceeding to start services");

        // ═══════════════════════════════════════════════════════════════════
        // Phase 5: Start TLS-dependent tasks
        // ═══════════════════════════════════════════════════════════════════
        state.start_tls_tasks(&mut background_tasks, tls_config);
        state.print_activated_banner_content().await;

        // ═══════════════════════════════════════════════════════════════════
        // Phase 6: Start HTTPS server (blocks until shutdown)
        // ═══════════════════════════════════════════════════════════════════
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
        tracing::info!("🦀 Crab Edge Server starting on {}", addr);

        let token = self.shutdown_token.clone();
        let shutdown = async move {
            token.cancelled().await;
            tracing::info!("Shutting down...");
        };

        state
            .https
            .start_server(rustls_config, shutdown)
            .await
            .map_err(|e| AppError::internal(format!("HTTPS server error: {e}")))?;

        // ═══════════════════════════════════════════════════════════════════
        // Phase 7: Graceful shutdown
        // ═══════════════════════════════════════════════════════════════════
        background_tasks.shutdown().await;

        Ok(())
    }

    /// Wait for activation and load TLS config (可取消)
    ///
    /// Blocks until device is activated and TLS certificates are loaded.
    /// Returns `None` if shutdown was requested during activation wait.
    /// Retries on failure by re-entering unbound state.
    async fn wait_for_tls(&self, state: &ServerState) -> Option<std::sync::Arc<rustls::ServerConfig>> {
        loop {
            if state.wait_for_activation(&self.shutdown_token).await.is_err() {
                return None; // shutdown requested
            }

            match state.load_tls_config() {
                Ok(Some(cfg)) => return Some(cfg),
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
