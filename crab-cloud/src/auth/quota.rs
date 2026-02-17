//! Quota validation middleware for edge-server sync
//!
//! Checks tenant status (must be active) and subscription edge-server limits.
//! Results are cached for 5 minutes to avoid DB queries on every sync request.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::auth::EdgeIdentity;
use crate::state::AppState;

/// Cache entry for quota validation
struct QuotaCacheEntry {
    allowed: bool,
    reason: Option<String>,
    expires_at: Instant,
}

/// Quota cache shared across requests
#[derive(Clone)]
pub struct QuotaCache {
    entries: Arc<RwLock<HashMap<String, QuotaCacheEntry>>>,
}

impl QuotaCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

const CACHE_TTL_SECS: u64 = 300; // 5 minutes

/// Middleware that validates tenant subscription quota
///
/// Must run AFTER edge_auth_middleware (requires EdgeIdentity in extensions).
pub async fn quota_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let identity = request
        .extensions()
        .get::<EdgeIdentity>()
        .cloned()
        .ok_or_else(|| error_response(500, "Missing EdgeIdentity"))?;

    let cache_key = identity.tenant_id.clone();

    // Check cache
    {
        let entries = state.quota_cache.entries.read().await;
        if let Some(entry) = entries.get(&cache_key)
            && entry.expires_at > Instant::now()
        {
            if !entry.allowed {
                let reason = entry.reason.as_deref().unwrap_or("Quota exceeded");
                return Err(error_response(403, reason));
            }
            return Ok(next.run(request).await);
        }
    }

    // Query DB
    let (allowed, reason) = check_quota(&state.pool, &identity).await;

    // Update cache
    {
        let mut entries = state.quota_cache.entries.write().await;
        entries.insert(
            cache_key,
            QuotaCacheEntry {
                allowed,
                reason: reason.clone(),
                expires_at: Instant::now() + std::time::Duration::from_secs(CACHE_TTL_SECS),
            },
        );
    }

    if !allowed {
        let reason = reason.as_deref().unwrap_or("Quota exceeded");
        return Err(error_response(403, reason));
    }

    Ok(next.run(request).await)
}

async fn check_quota(pool: &PgPool, identity: &EdgeIdentity) -> (bool, Option<String>) {
    // Check tenant status
    let tenant_status: Option<(String,)> =
        match sqlx::query_as("SELECT status FROM tenants WHERE id = $1")
            .bind(&identity.tenant_id)
            .fetch_optional(pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Quota check DB error: {e}");
                return (false, Some("Internal error".to_string()));
            }
        };

    let Some((status,)) = tenant_status else {
        return (false, Some("Tenant not found".to_string()));
    };

    if status != "active" {
        return (
            false,
            Some(format!("Tenant status is '{status}', must be 'active'")),
        );
    }

    // Check subscription quota
    let sub: Option<(i32,)> = match sqlx::query_as(
        "SELECT max_edge_servers FROM subscriptions WHERE tenant_id = $1 AND status = 'active'",
    )
    .bind(&identity.tenant_id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Subscription query error: {e}");
            return (false, Some("Internal error".to_string()));
        }
    };

    let Some((max_edge_servers,)) = sub else {
        return (false, Some("No active subscription".to_string()));
    };

    // Count current edge servers
    let (current_count,): (i64,) =
        match sqlx::query_as("SELECT COUNT(*) FROM cloud_edge_servers WHERE tenant_id = $1")
            .bind(&identity.tenant_id)
            .fetch_one(pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Edge server count error: {e}");
                return (false, Some("Internal error".to_string()));
            }
        };

    // Check if this edge-server is already registered
    let already_registered: bool = match sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM cloud_edge_servers WHERE entity_id = $1 AND tenant_id = $2",
    )
    .bind(&identity.entity_id)
    .bind(&identity.tenant_id)
    .fetch_one(pool)
    .await
    {
        Ok((count,)) => count > 0,
        Err(_) => false,
    };

    if !already_registered && current_count >= max_edge_servers as i64 {
        return (
            false,
            Some(format!(
                "Edge server quota exceeded: {current_count}/{max_edge_servers}"
            )),
        );
    }

    (true, None)
}

fn error_response(status: u16, message: &str) -> Response {
    let body = serde_json::json!({ "error": message });
    let status =
        http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(body)).into_response()
}
