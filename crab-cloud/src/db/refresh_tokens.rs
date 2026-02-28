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
    user_agent: &str,
    ip_address: &str,
) -> Result<String, sqlx::Error> {
    let token_id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let expires_at = now + REFRESH_TOKEN_TTL_MS;

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
        "INSERT INTO refresh_tokens (id, tenant_id, device_id, expires_at, user_agent, ip_address, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&token_id)
    .bind(tenant_id)
    .bind(device_id)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_address)
    .bind(now)
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
    user_agent: &str,
    ip_address: &str,
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
    let now = now_millis();
    let expires_at = now + REFRESH_TOKEN_TTL_MS;

    sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE tenant_id = $1 AND device_id = $2 AND NOT revoked",
    )
    .bind(&row.tenant_id)
    .bind(&row.device_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO refresh_tokens (id, tenant_id, device_id, expires_at, user_agent, ip_address, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&token_id)
    .bind(&row.tenant_id)
    .bind(&row.device_id)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_address)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some((row.tenant_id, row.device_id, token_id)))
}

/// List active (non-revoked, non-expired) sessions for a tenant.
pub async fn list_active(pool: &PgPool, tenant_id: &str) -> Result<Vec<SessionRow>, sqlx::Error> {
    let now = now_millis();
    sqlx::query_as(
        "SELECT id, device_id, user_agent, ip_address, created_at \
         FROM refresh_tokens \
         WHERE tenant_id = $1 AND NOT revoked AND expires_at > $2 \
         ORDER BY created_at DESC",
    )
    .bind(tenant_id)
    .bind(now)
    .fetch_all(pool)
    .await
}

/// Revoke a specific session by token id (must belong to tenant).
pub async fn revoke_session(
    pool: &PgPool,
    tenant_id: &str,
    token_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE id = $1 AND tenant_id = $2 AND NOT revoked",
    )
    .bind(token_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    tenant_id: String,
    device_id: String,
    expires_at: i64,
    revoked: bool,
}

#[derive(sqlx::FromRow)]
pub struct SessionRow {
    pub id: String,
    pub device_id: String,
    pub user_agent: String,
    pub ip_address: String,
    pub created_at: i64,
}
