//! API routes for crab-cloud
//!
//! Split into two routers:
//! - `public_router`: HTTP port (health, register, webhook, update, tenant API)
//! - `edge_router`: mTLS port (edge sync with SignedBinding + quota validation)

pub mod health;
pub mod pki;
pub mod register;
pub mod stripe_webhook;
pub mod sync;
pub mod tenant;
pub mod update;
pub mod ws;

use crate::auth::edge_auth::edge_auth_middleware;
use crate::auth::quota::quota_middleware;
use crate::auth::rate_limit::{
    global_rate_limit, login_rate_limit, p12_upload_rate_limit, password_reset_rate_limit,
    register_rate_limit,
};
use crate::auth::tenant_auth::tenant_auth_middleware;
use crate::state::AppState;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use axum::{Router, middleware};
use http::HeaderName;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Public router — served on HTTP port (no mTLS)
///
/// Includes: health, registration, Stripe webhook, app update, tenant management API
pub fn public_router(state: AppState) -> Router {
    // Public registration (rate-limited)
    let registration = Router::new()
        .route("/api/register", post(register::register))
        .route("/api/verify-email", post(register::verify_email))
        .route("/api/resend-code", post(register::resend_code))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            register_rate_limit,
        ));

    // Stripe webhook (signature-verified, raw body)
    let webhook = Router::new().route("/stripe/webhook", post(stripe_webhook::handle_webhook));

    // App update check (public, no auth)
    let app_update = Router::new()
        .route(
            "/api/update/{target}/{arch}/{current_version}",
            get(update::check_update),
        )
        .route("/api/download/latest", get(update::download_latest));

    // Tenant management API (JWT authenticated)
    let tenant_api = Router::new()
        .route(
            "/api/tenant/profile",
            get(tenant::get_profile).put(tenant::update_profile),
        )
        .route("/api/tenant/change-email", post(tenant::change_email))
        .route(
            "/api/tenant/confirm-email-change",
            post(tenant::confirm_email_change),
        )
        .route("/api/tenant/change-password", post(tenant::change_password))
        .route("/api/tenant/overview", get(tenant::get_tenant_overview))
        .route("/api/tenant/stores", get(tenant::list_stores))
        .route("/api/tenant/stores/{id}", patch(tenant::update_store))
        .route("/api/tenant/stores/{id}/orders", get(tenant::list_orders))
        .route(
            "/api/tenant/stores/{id}/orders/{order_key}/detail",
            get(tenant::get_order_detail),
        )
        .route("/api/tenant/stores/{id}/stats", get(tenant::get_stats))
        .route(
            "/api/tenant/stores/{id}/overview",
            get(tenant::get_store_overview),
        )
        .route(
            "/api/tenant/stores/{id}/products",
            get(tenant::list_products),
        )
        .route("/api/tenant/billing-portal", post(tenant::billing_portal))
        .route("/api/tenant/create-checkout", post(tenant::create_checkout))
        .route("/api/tenant/audit-log", get(tenant::audit_log))
        .route(
            "/api/tenant/stores/{id}/commands",
            post(tenant::create_command).get(tenant::list_commands),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            tenant_auth_middleware,
        ));

    // Tenant login (rate-limited)
    let tenant_login = Router::new()
        .route("/api/tenant/login", post(tenant::login))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            login_rate_limit,
        ));

    // Password reset (rate-limited)
    let password_reset = Router::new()
        .route("/api/tenant/forgot-password", post(tenant::forgot_password))
        .route("/api/tenant/reset-password", post(tenant::reset_password))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            password_reset_rate_limit,
        ));

    // PKI routes (merged from crab-auth)
    let pki_routes = pki::pki_router();

    // P12 upload (rate-limited: 3 req/min per IP)
    let p12_upload =
        Router::new()
            .merge(pki::p12_upload_router())
            .layer(middleware::from_fn_with_state(
                state.clone(),
                p12_upload_rate_limit,
            ));

    // PKI auth routes (accept password → login rate limit: 5 req/min per IP)
    let pki_auth_routes = pki::pki_auth_router();
    let pki_auth_limited =
        Router::new()
            .merge(pki_auth_routes)
            .layer(middleware::from_fn_with_state(
                state.clone(),
                login_rate_limit,
            ));

    // CORS — allow portal frontend to call API
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "https://redcoral.app".parse().unwrap(),
            "https://www.redcoral.app".parse().unwrap(),
            "https://console.redcoral.app".parse().unwrap(),
            "http://localhost:5173".parse().unwrap(), // dev
            "http://localhost:5174".parse().unwrap(), // dev console
        ]))
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::DELETE,
            http::Method::OPTIONS,
        ])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            HeaderName::from_static("x-signed-binding"),
        ])
        .max_age(std::time::Duration::from_secs(3600));

    Router::new()
        .route("/health", get(health::health_check))
        .merge(registration)
        .merge(webhook)
        .merge(app_update)
        .merge(tenant_api)
        .merge(tenant_login)
        .merge(password_reset)
        .merge(pki_routes)
        .merge(p12_upload)
        .merge(pki_auth_limited)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            global_rate_limit,
        ))
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB
        .layer(cors)
        .with_state(state)
}

/// Edge router — served on mTLS port (requires client certificate + SignedBinding + quota)
pub fn edge_router(state: AppState) -> Router {
    Router::new()
        .route("/api/edge/sync", post(sync::handle_sync))
        .route("/api/edge/ws", get(ws::handle_edge_ws))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            edge_auth_middleware,
        ))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB for sync batches
        .with_state(state)
}
