//! Refresh token storage

use shared::util::now_millis;
use sqlx::PgPool;

const REFRESH_TOKEN_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000; // 30 days

/// Create a new refresh token, revoking any existing tokens for this tenant+device.
/// Both operations run in a single transaction.
pub async fn create(
    pool: &PgPool,
    tenant_id: &str,
    device_id: &str,
) -> Result<String, sqlx::Error> {
    let token_id = uuid::Uuid::new_v4().to_string();
    let expires_at = now_millis() + REFRESH_TOKEN_TTL_MS;

    let mut tx = pool.begin().await?;

    // Revoke existing tokens for this tenant+device
    sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE tenant_id = $1 AND device_id = $2 AND NOT revoked",
    )
    .bind(tenant_id)
    .bind(device_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO refresh_tokens (id, tenant_id, device_id, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token_id)
    .bind(tenant_id)
    .bind(device_id)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(token_id)
}

/// Validate and rotate a refresh token. Returns (tenant_id, device_id, new_refresh_token).
/// The entire SELECT + revoke + create sequence runs in a single transaction.
pub async fn rotate(
    pool: &PgPool,
    refresh_token: &str,
) -> Result<Option<(String, String, String)>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Find valid token (SELECT ... FOR UPDATE to lock the row)
    let row: Option<RefreshTokenRow> = sqlx::query_as(
        "SELECT tenant_id, device_id, expires_at, revoked FROM refresh_tokens WHERE id = $1 FOR UPDATE",
    )
    .bind(refresh_token)
    .fetch_optional(&mut *tx)
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
        .execute(&mut *tx)
        .await?;

    // Revoke any other active tokens for this tenant+device and insert new token
    let token_id = uuid::Uuid::new_v4().to_string();
    let expires_at = now_millis() + REFRESH_TOKEN_TTL_MS;

    sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE tenant_id = $1 AND device_id = $2 AND NOT revoked",
    )
    .bind(&row.tenant_id)
    .bind(&row.device_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO refresh_tokens (id, tenant_id, device_id, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token_id)
    .bind(&row.tenant_id)
    .bind(&row.device_id)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some((row.tenant_id, row.device_id, token_id)))
}

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    tenant_id: String,
    device_id: String,
    expires_at: i64,
    revoked: bool,
}
