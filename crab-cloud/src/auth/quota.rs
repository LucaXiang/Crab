//! Quota validation middleware for edge-server sync
//!
//! Checks tenant status (must be active) and subscription edge-server limits.
//! Results are cached for 5 minutes to avoid DB queries on every sync request.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use shared::error::{AppError, ErrorCode};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::auth::EdgeIdentity;
use crate::state::AppState;

/// Cache entry for quota validation
struct QuotaCacheEntry {
    error: Option<ErrorCode>,
    expires_at: Instant,
}

/// Quota cache shared across requests
#[derive(Clone)]
pub struct QuotaCache {
    entries: Arc<RwLock<HashMap<i64, QuotaCacheEntry>>>,
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
        .ok_or_else(|| AppError::new(ErrorCode::InternalError).into_response())?;

    let cache_key = identity.tenant_id;

    // Check cache (extract result before dropping read lock)
    let cached_result = {
        let entries = state.quota_cache.entries.read().await;
        entries
            .get(&cache_key)
            .filter(|e| e.expires_at > Instant::now())
            .map(|e| e.error)
    };

    if let Some(maybe_err) = cached_result {
        if let Some(code) = maybe_err {
            return Err(AppError::new(code).into_response());
        }
        return Ok(next.run(request).await);
    }

    // Query DB
    let error = check_quota(&state.pool, &identity).await;

    // Update cache
    {
        let mut entries = state.quota_cache.entries.write().await;
        entries.insert(
            cache_key,
            QuotaCacheEntry {
                error,
                expires_at: Instant::now() + std::time::Duration::from_secs(CACHE_TTL_SECS),
            },
        );
    }

    if let Some(code) = error {
        return Err(AppError::new(code).into_response());
    }

    Ok(next.run(request).await)
}

/// Returns `None` if quota check passes, or `Some(ErrorCode)` on failure.
async fn check_quota(pool: &PgPool, identity: &EdgeIdentity) -> Option<ErrorCode> {
    // Check tenant status
    let tenant_status: Option<(String,)> =
        match sqlx::query_as("SELECT status FROM tenants WHERE id = $1")
            .bind(identity.tenant_id)
            .fetch_optional(pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Quota check DB error: {e}");
                return Some(ErrorCode::InternalError);
            }
        };

    let Some((status,)) = tenant_status else {
        return Some(ErrorCode::TenantNotFound);
    };

    if status != shared::cloud::TenantStatus::Active.as_db() {
        tracing::warn!(tenant_id = %identity.tenant_id, status = %status, "Tenant not active");
        return Some(ErrorCode::SubscriptionBlocked);
    }

    // Check subscription quota
    let sub: Option<(i32,)> = match sqlx::query_as(
        "SELECT max_stores FROM subscriptions WHERE tenant_id = $1 AND status = 'active'",
    )
    .bind(identity.tenant_id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Subscription query error: {e}");
            return Some(ErrorCode::InternalError);
        }
    };

    let Some((max_stores,)) = sub else {
        return Some(ErrorCode::TenantNoSubscription);
    };

    // Count current edge servers
    let (current_count,): (i64,) =
        match sqlx::query_as("SELECT COUNT(*) FROM stores WHERE tenant_id = $1")
            .bind(identity.tenant_id)
            .fetch_one(pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Edge server count error: {e}");
                return Some(ErrorCode::InternalError);
            }
        };

    // Check if this edge-server is already registered
    let already_registered: bool = match sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM stores WHERE entity_id = $1 AND tenant_id = $2",
    )
    .bind(&identity.entity_id)
    .bind(identity.tenant_id)
    .fetch_one(pool)
    .await
    {
        Ok((count,)) => count > 0,
        Err(e) => {
            tracing::warn!(entity_id = %identity.entity_id, error = %e, "DB error checking registration status, assuming not registered");
            false
        }
    };

    if !already_registered && current_count >= max_stores as i64 {
        tracing::warn!(
            tenant_id = %identity.tenant_id,
            current = current_count,
            max = max_stores,
            "Store quota exceeded"
        );
        return Some(ErrorCode::StoreLimitReached);
    }

    None
}
