use crate::core::{Config, ServerState};
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower::Service;

pub type OneshotResult =
    Result<http::Response<axum::body::Body>, Box<dyn std::error::Error + Send + Sync>>;

/// Build the Axum router (without state)
pub fn build_app() -> Router<ServerState> {
    Router::new()
        .merge(crate::api::auth::router())
        .merge(crate::api::health::router())
        .merge(crate::api::role::router())
        .merge(crate::api::audit::router())
        .merge(crate::api::upload::router())
    // 认证中间件在 individual routes 中通过 .layer() 添加
}

#[derive(Clone, Debug)]
pub struct HttpsService {
    config: Config,
    router: Arc<RwLock<Option<Router>>>,
}

impl HttpsService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            router: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the router with the given server state.
    /// This should be called after ServerState is fully initialized.
    pub fn initialize(&self, state: ServerState) {
        // Build the app with state and cache it
        let app = build_app().with_state(state);
        let mut router = self.router.write().expect("Failed to lock router");
        *router = Some(app);
    }

    pub fn router(&self) -> Option<Router> {
        self.router.read().expect("Failed to lock router").clone()
    }

    pub async fn oneshot(&self, request: http::Request<axum::body::Body>) -> OneshotResult {
        let router_opt = self.router.read().expect("Failed to lock router").clone();

        match router_opt {
            Some(router) => {
                let mut service = router.clone();
                // We must use the router as a service.
                // Since it's already bound with state, it implements Service<Request>.
                match service.call(request).await {
                    Ok(response) => Ok(response),
                    Err(_) => Err(crate::utils::AppError::internal("Oneshot call failed").into()),
                }
            }
            None => Err(crate::utils::AppError::internal("HttpsService not initialized").into()),
        }
    }

    /// Explicitly start the HTTPS server
    pub async fn start_server<F>(
        &self,
        tls_config: RustlsConfig,
        shutdown_signal: F,
    ) -> Result<(), crate::utils::AppError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let app = self.router().ok_or_else(|| {
            crate::utils::AppError::internal("HttpsService not initialized with router")
        })?;

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http_port));
        tracing::info!("🚀 Starting HTTPS server on {}", addr);

        let handle = axum_server::Handle::new();

        // Handle shutdown signal
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            shutdown_signal.await;
            handle_clone.graceful_shutdown(Some(std::time::Duration::from_secs(10)));
        });

        axum_server::bind_rustls(addr, tls_config)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .map_err(|e| crate::utils::AppError::internal(format!("Server error: {}", e)))?;

        Ok(())
    }
}
