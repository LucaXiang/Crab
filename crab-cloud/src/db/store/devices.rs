use serde::Serialize;
use sqlx::PgPool;

#[derive(sqlx::FromRow, Serialize)]
pub struct DeviceRecord {
    pub entity_id: String,
    pub device_id: String,
    pub device_type: String,
    pub status: String,
    pub activated_at: i64,
    pub deactivated_at: Option<i64>,
    pub replaced_by: Option<String>,
    pub last_refreshed_at: Option<i64>,
}

pub async fn list_devices_for_store(
    pool: &PgPool,
    entity_id: &str,
    tenant_id: i64,
) -> Result<Vec<DeviceRecord>, sqlx::Error> {
    sqlx::query_as::<_, DeviceRecord>(
        r#"
        SELECT entity_id, device_id, 'server' AS device_type, status, activated_at, deactivated_at, replaced_by, last_refreshed_at
        FROM activations WHERE tenant_id = $2 AND (entity_id = $1 OR replaced_by = $1)
        UNION ALL
        SELECT entity_id, device_id, 'client' AS device_type, status, activated_at, deactivated_at, replaced_by, last_refreshed_at
        FROM client_connections WHERE tenant_id = $2
        ORDER BY activated_at DESC
        "#,
    )
    .bind(entity_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}
