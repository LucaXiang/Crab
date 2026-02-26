use crate::auth::require_auth;
use crate::core::{Config, ServerState};
use axum::{Router, middleware};
use axum_server::tls_rustls::RustlsConfig;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::Service;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;

pub type OneshotResult =
    Result<http::Response<axum::body::Body>, Box<dyn std::error::Error + Send + Sync>>;

/// HTTP 请求日志中间件
async fn log_request(
    request: http::Request<axum::body::Body>,
    next: middleware::Next,
) -> http::Response<axum::body::Body> {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    let status = response.status();

    tracing::info!(target: "http_access", "{} {} {}", method, uri, status);

    response
}

/// Build the Axum router (without state)
pub fn build_app() -> Router<ServerState> {
    Router::<ServerState>::new()
        // Core APIs
        .merge(crate::api::auth::router())
        .merge(crate::api::health::router())
        .merge(crate::api::role::router())
        .merge(crate::api::upload::router())
        // Data model APIs
        .merge(crate::api::tags::router())
        .merge(crate::api::categories::router())
        .merge(crate::api::products::router())
        .merge(crate::api::attributes::router())
        .merge(crate::api::has_attribute::router())
        .merge(crate::api::zones::router())
        .merge(crate::api::tables::router())
        .merge(crate::api::price_rules::router())
        .merge(crate::api::print_destinations::router())
        .merge(crate::api::print_config::router())
        .merge(crate::api::employees::router())
        .merge(crate::api::orders::router())
        .merge(crate::api::kitchen_orders::router())
        .merge(crate::api::system_state::router())
        .merge(crate::api::store_info::router())
        .merge(crate::api::label_template::router())
        // Membership & Marketing
        .merge(crate::api::members::router())
        .merge(crate::api::marketing_groups::router())
        // Operations (班次与日结)
        .merge(crate::api::shifts::router())
        .merge(crate::api::daily_reports::router())
        // Analytics (数据统计)
        .merge(crate::api::statistics::router())
        // Archive (归档验证)
        .merge(crate::api::archive_verify::router())
        // Audit (审计日志)
        .merge(crate::api::audit_log::router())
        // System Issues (系统问题)
        .merge(crate::api::system_issues::router())
        // Data Transfer (catalog export/import)
        .merge(crate::api::data_transfer::router())
        // Sync API
        .merge(crate::api::sync::router())
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
        let app = build_app()
            // JWT 认证中间件 - 在 Router 级别应用，require_auth 内部会跳过公共路由
            // 使用 from_fn_with_state 以便中间件可以访问 ServerState
            .layer(middleware::from_fn_with_state(state.clone(), require_auth))
            .with_state(state)
            // Tower HTTP 中间件
            .layer(CorsLayer::permissive())
            .layer(CompressionLayer::new())
            // HTTP 请求日志中间件
            .layer(middleware::from_fn(log_request));

        let mut router = self.router.write();
        *router = Some(app);
    }

    pub fn router(&self) -> Option<Router> {
        self.router.read().clone()
    }

    /// Break circular reference: Router → ServerState → HttpsService → Router
    ///
    /// Must be called during shutdown BEFORE dropping ServerState,
    /// otherwise Arc<Database> inside OrdersManager will leak due to
    /// the reference cycle and redb file lock will never be released.
    pub fn clear_router(&self) {
        *self.router.write() = None;
    }

    pub async fn oneshot(&self, request: http::Request<axum::body::Body>) -> OneshotResult {
        let router_opt = self.router.read().clone();

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
        tracing::info!("Starting HTTPS server on {}", addr);

        let handle = axum_server::Handle::new();

        // Handle shutdown signal
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            shutdown_signal.await;
            handle_clone.graceful_shutdown(Some(std::time::Duration::from_secs(2)));
        });

        axum_server::bind_rustls(addr, tls_config)
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .map_err(|e| crate::utils::AppError::internal(format!("Server error: {}", e)))?;

        Ok(())
    }
}
