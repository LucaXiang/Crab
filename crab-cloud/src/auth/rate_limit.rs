//! Application-layer rate limiting for login and registration routes

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use shared::error::{AppError, ErrorCode};
use std::sync::Arc;
use tokio::time::Instant;

struct IpEntry {
    count: u32,
    window_start: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    /// route name -> (IP -> entry)
    inner: Arc<DashMap<&'static str, DashMap<String, IpEntry>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    fn check(&self, route: &'static str, ip: &str, max_requests: u32, window_secs: u64) -> bool {
        let route_map = self.inner.entry(route).or_default();
        let now = Instant::now();

        let mut entry = route_map.entry(ip.to_owned()).or_insert_with(|| IpEntry {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(entry.window_start).as_secs() >= window_secs {
            entry.count = 0;
            entry.window_start = now;
        }

        entry.count += 1;
        entry.count <= max_requests
    }

    /// Remove entries older than 5 minutes
    pub fn cleanup(&self) {
        let cutoff = std::time::Duration::from_secs(300);
        let now = Instant::now();

        for route_map in self.inner.iter() {
            route_map.retain(|_, entry| now.duration_since(entry.window_start) < cutoff);
        }

        // Remove empty route maps
        self.inner.retain(|_, route_map| !route_map.is_empty());
    }
}

/// Extract client IP: X-Real-IP (Caddy sets from remote_host), then X-Forwarded-For last entry, then peer address.
fn extract_ip(request: &Request) -> String {
    // Caddy sets X-Real-IP to the direct client IP (not spoofable by client)
    if let Some(real_ip) = request.headers().get("x-real-ip")
        && let Ok(ip) = real_ip.to_str()
    {
        let ip = ip.trim();
        if !ip.is_empty() {
            return ip.to_owned();
        }
    }

    // Fallback: last entry in X-Forwarded-For
    if let Some(forwarded) = request.headers().get("x-forwarded-for")
        && let Ok(val) = forwarded.to_str()
        && let Some(last) = val.rsplit(',').next()
    {
        let ip = last.trim();
        if !ip.is_empty() {
            return ip.to_owned();
        }
    }

    // Fallback: peer address from ConnectInfo
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn too_many_requests() -> Response {
    AppError::new(ErrorCode::TooManyAttempts).into_response()
}

/// Rate limit middleware for login: 10 requests/minute per IP
pub async fn login_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    if !state.rate_limiter.check("login", &ip, 10, 60) {
        return Err(too_many_requests());
    }
    Ok(next.run(request).await)
}

/// Rate limit middleware for registration: 3 requests/minute per IP
pub async fn register_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    if !state.rate_limiter.check("register", &ip, 3, 60) {
        return Err(too_many_requests());
    }
    Ok(next.run(request).await)
}

/// Rate limit middleware for password reset: 3 requests/minute per IP
pub async fn password_reset_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    if !state.rate_limiter.check("password_reset", &ip, 3, 60) {
        return Err(too_many_requests());
    }
    Ok(next.run(request).await)
}

/// Rate limit middleware for P12 upload: 5 requests/minute per IP
pub async fn p12_upload_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    if !state.rate_limiter.check("p12_upload", &ip, 5, 60) {
        return Err(too_many_requests());
    }
    Ok(next.run(request).await)
}

/// Global rate limit middleware: 200 requests/minute per IP
///
/// Authenticated tenant API can burst many calls per page load (stores, orders,
/// stats, overview, products…), so the global cap must be generous.
/// Sensitive endpoints (login, register, password_reset) have their own stricter limits.
pub async fn global_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    if !state.rate_limiter.check("global", &ip, 200, 60) {
        return Err(too_many_requests());
    }
    Ok(next.run(request).await)
}
