//! Application-layer rate limiting for login and registration routes

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

struct IpEntry {
    count: u32,
    window_start: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    /// route name -> (IP -> entry)
    inner: Arc<Mutex<HashMap<&'static str, HashMap<String, IpEntry>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    async fn check(
        &self,
        route: &'static str,
        ip: &str,
        max_requests: u32,
        window_secs: u64,
    ) -> bool {
        let mut map = self.inner.lock().await;
        let route_map = map.entry(route).or_default();
        let now = Instant::now();

        let entry = route_map.entry(ip.to_owned()).or_insert_with(|| IpEntry {
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
    pub async fn cleanup(&self) {
        let mut map = self.inner.lock().await;
        let cutoff = std::time::Duration::from_secs(300);
        let now = Instant::now();

        for route_map in map.values_mut() {
            route_map.retain(|_, entry| now.duration_since(entry.window_start) < cutoff);
        }

        // Remove empty route maps
        map.retain(|_, route_map| !route_map.is_empty());
    }
}

/// Extract client IP: X-Forwarded-For header first (ALB/CloudFront), then peer address.
fn extract_ip(request: &Request) -> String {
    if let Some(forwarded) = request.headers().get("x-forwarded-for")
        && let Ok(val) = forwarded.to_str()
    {
        // X-Forwarded-For can be comma-separated; first entry is the original client
        if let Some(first) = val.split(',').next() {
            let ip = first.trim();
            if !ip.is_empty() {
                return ip.to_owned();
            }
        }
    }

    // Fallback: peer address from extensions (ConnectInfo)
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn too_many_requests() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        axum::Json(serde_json::json!({"error": "Too many requests, try again later"})),
    )
        .into_response()
}

/// Rate limit middleware for login: 5 requests/minute per IP
pub async fn login_rate_limit(
    State(state): State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let ip = extract_ip(&request);
    if !state.rate_limiter.check("login", &ip, 5, 60).await {
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
    if !state.rate_limiter.check("register", &ip, 3, 60).await {
        return Err(too_many_requests());
    }
    Ok(next.run(request).await)
}
