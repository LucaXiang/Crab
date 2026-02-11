use sqlx::PgPool;

/// Client 连接记录
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct ClientConnection {
    pub entity_id: String,
    pub tenant_id: String,
    pub device_id: String,
    pub fingerprint: String,
    pub status: String,
    pub activated_at: i64,
    pub last_refreshed_at: Option<i64>,
}

/// 统计租户活跃 Client 数
pub async fn count_active(pool: &PgPool, tenant_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM client_connections WHERE tenant_id = $1 AND status = 'active'",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// 获取租户所有活跃 Client
pub async fn list_active(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Vec<ClientConnection>, sqlx::Error> {
    sqlx::query_as::<_, ClientConnection>(
        "SELECT entity_id, tenant_id, device_id, fingerprint, status,
            activated_at, last_refreshed_at
            FROM client_connections
            WHERE tenant_id = $1 AND status = 'active'
            ORDER BY activated_at",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}

/// 按 tenant_id + device_id 查找
pub async fn find_by_device(
    pool: &PgPool,
    tenant_id: &str,
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

/// 插入新的 Client 连接记录
pub async fn insert(
    pool: &PgPool,
    entity_id: &str,
    tenant_id: &str,
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
    .execute(pool)
    .await?;
    Ok(())
}

/// 标记 Client 为 replaced
pub async fn mark_replaced(
    pool: &PgPool,
    old_entity_id: &str,
    new_entity_id: &str,
) -> Result<bool, sqlx::Error> {
    let now = shared::util::now_millis();
    let result = sqlx::query(
        "UPDATE client_connections
            SET status = 'replaced', deactivated_at = $1, replaced_by = $2
            WHERE entity_id = $3 AND status = 'active'",
    )
    .bind(now)
    .bind(new_entity_id)
    .bind(old_entity_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// 注销客户端连接 (释放配额)
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
