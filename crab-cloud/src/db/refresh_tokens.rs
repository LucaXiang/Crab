//! Refresh token storage

use shared::util::now_millis;
use sqlx::PgPool;

const REFRESH_TOKEN_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000; // 30 days

/// Create a new refresh token, revoking any existing tokens for this tenant+device
pub async fn create(
    pool: &PgPool,
    tenant_id: &str,
    device_id: &str,
) -> Result<String, sqlx::Error> {
    let token_id = uuid::Uuid::new_v4().to_string();
    let expires_at = now_millis() + REFRESH_TOKEN_TTL_MS;

    // Revoke existing tokens for this tenant+device
    sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE tenant_id = $1 AND device_id = $2 AND NOT revoked",
    )
    .bind(tenant_id)
    .bind(device_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO refresh_tokens (id, tenant_id, device_id, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token_id)
    .bind(tenant_id)
    .bind(device_id)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(token_id)
}

/// Validate and rotate a refresh token. Returns (tenant_id, device_id, new_refresh_token).
pub async fn rotate(
    pool: &PgPool,
    refresh_token: &str,
) -> Result<Option<(String, String, String)>, sqlx::Error> {
    // Find valid token
    let row: Option<RefreshTokenRow> = sqlx::query_as(
        "SELECT tenant_id, device_id, expires_at, revoked FROM refresh_tokens WHERE id = $1",
    )
    .bind(refresh_token)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    if row.revoked || row.expires_at < now_millis() {
        return Ok(None);
    }

    // Revoke the used token
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE id = $1")
        .bind(refresh_token)
        .execute(pool)
        .await?;

    // Issue new token
    let new_token = create(pool, &row.tenant_id, &row.device_id).await?;

    Ok(Some((row.tenant_id, row.device_id, new_token)))
}

/// Revoke all refresh tokens for a tenant
#[allow(dead_code)]
pub async fn revoke_all(pool: &PgPool, tenant_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE tenant_id = $1 AND NOT revoked")
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    tenant_id: String,
    device_id: String,
    expires_at: i64,
    revoked: bool,
}
