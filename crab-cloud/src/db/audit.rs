//! Audit log operations

use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Write an audit log entry
pub async fn log(
    pool: &PgPool,
    tenant_id: &str,
    action: &str,
    detail: Option<&serde_json::Value>,
    ip_address: Option<&str>,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        "INSERT INTO audit_logs (tenant_id, action, detail, ip_address, created_at) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(tenant_id)
    .bind(action)
    .bind(detail)
    .bind(ip_address)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Query audit log entries for a tenant (paginated)
#[derive(sqlx::FromRow, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub action: String,
    pub detail: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: i64,
}

pub async fn query(
    pool: &PgPool,
    tenant_id: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<AuditEntry>, BoxError> {
    let rows: Vec<AuditEntry> = sqlx::query_as(
        "SELECT id, action, detail, ip_address, created_at FROM audit_logs WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
