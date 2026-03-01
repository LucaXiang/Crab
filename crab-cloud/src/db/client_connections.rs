use sqlx::PgPool;

/// Client 连接记录
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct ClientConnection {
    pub entity_id: String,
    pub tenant_id: i64,
    pub device_id: String,
    pub fingerprint: String,
    pub status: String,
    pub activated_at: i64,
    pub last_refreshed_at: Option<i64>,
}

/// 获取租户 client 激活 advisory lock (防止并发激活超配额)
pub async fn acquire_activation_lock(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: i64,
) -> Result<(), sqlx::Error> {
    // Offset by 1_000_000_000 to avoid collision with server activation lock
    sqlx::query("SELECT pg_advisory_xact_lock($1 + 1000000000)")
        .bind(tenant_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// 按 tenant_id + device_id 查找
pub async fn find_by_device(
    pool: &PgPool,
    tenant_id: i64,
    device_id: &str,
) -> Result<Option<ClientConnection>, sqlx::Error> {
    sqlx::query_as::<_, ClientConnection>(
        "SELECT entity_id, tenant_id, device_id, fingerprint, status,
            activated_at, last_refreshed_at
            FROM client_connections
            WHERE tenant_id = $1 AND device_id = $2",
    )
    .bind(tenant_id)
    .bind(device_id)
    .fetch_optional(pool)
    .await
}

/// 按 entity_id 查找
pub async fn find_by_entity(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Option<ClientConnection>, sqlx::Error> {
    sqlx::query_as::<_, ClientConnection>(
        "SELECT entity_id, tenant_id, device_id, fingerprint, status,
            activated_at, last_refreshed_at
            FROM client_connections
            WHERE entity_id = $1",
    )
    .bind(entity_id)
    .fetch_optional(pool)
    .await
}

/// 在事务内插入 Client 连接记录 (配合 advisory lock 使用)
pub async fn insert_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity_id: &str,
    tenant_id: i64,
    device_id: &str,
    fingerprint: &str,
) -> Result<(), sqlx::Error> {
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO client_connections (entity_id, tenant_id, device_id, fingerprint, status, activated_at)
            VALUES ($1, $2, $3, $4, 'active', $5)
            ON CONFLICT (tenant_id, device_id)
            DO UPDATE SET entity_id = $1, fingerprint = $4, status = 'active',
                          activated_at = $5, deactivated_at = NULL, replaced_by = NULL",
    )
    .bind(entity_id)
    .bind(tenant_id)
    .bind(device_id)
    .bind(fingerprint)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 注销客户端连接
pub async fn deactivate(pool: &PgPool, entity_id: &str) -> Result<bool, sqlx::Error> {
    let now = shared::util::now_millis();
    let result = sqlx::query(
        "UPDATE client_connections SET status = 'deactivated', deactivated_at = $1 WHERE entity_id = $2 AND status = 'active'",
    )
    .bind(now)
    .bind(entity_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
