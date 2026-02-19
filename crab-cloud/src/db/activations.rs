use sqlx::PgPool;

/// 激活记录
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Activation {
    pub entity_id: String,
    pub tenant_id: String,
    pub device_id: String,
    pub fingerprint: String,
    pub status: String,
    pub activated_at: i64,
    pub last_refreshed_at: Option<i64>,
}

/// 统计租户活跃设备数
pub async fn count_active(pool: &PgPool, tenant_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM activations WHERE tenant_id = $1 AND status = 'active'",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// 获取租户所有活跃设备
pub async fn list_active(pool: &PgPool, tenant_id: &str) -> Result<Vec<Activation>, sqlx::Error> {
    sqlx::query_as::<_, Activation>(
        "SELECT entity_id, tenant_id, device_id, fingerprint, status,
            activated_at, last_refreshed_at
            FROM activations
            WHERE tenant_id = $1 AND status = 'active'
            ORDER BY activated_at",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}

/// 按 tenant_id + device_id 查找激活记录
pub async fn find_by_device(
    pool: &PgPool,
    tenant_id: &str,
    device_id: &str,
) -> Result<Option<Activation>, sqlx::Error> {
    sqlx::query_as::<_, Activation>(
        "SELECT entity_id, tenant_id, device_id, fingerprint, status,
            activated_at, last_refreshed_at
            FROM activations
            WHERE tenant_id = $1 AND device_id = $2",
    )
    .bind(tenant_id)
    .bind(device_id)
    .fetch_optional(pool)
    .await
}

/// 按 entity_id 查找激活记录
pub async fn find_by_entity(
    pool: &PgPool,
    entity_id: &str,
) -> Result<Option<Activation>, sqlx::Error> {
    sqlx::query_as::<_, Activation>(
        "SELECT entity_id, tenant_id, device_id, fingerprint, status,
            activated_at, last_refreshed_at
            FROM activations
            WHERE entity_id = $1",
    )
    .bind(entity_id)
    .fetch_optional(pool)
    .await
}

/// 插入新的激活记录
pub async fn insert(
    pool: &PgPool,
    entity_id: &str,
    tenant_id: &str,
    device_id: &str,
    fingerprint: &str,
) -> Result<(), sqlx::Error> {
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO activations (entity_id, tenant_id, device_id, fingerprint, status, activated_at)
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

/// 将旧设备标记为 replaced
pub async fn mark_replaced(
    pool: &PgPool,
    old_entity_id: &str,
    new_entity_id: &str,
) -> Result<bool, sqlx::Error> {
    let now = shared::util::now_millis();
    let result = sqlx::query(
        "UPDATE activations
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

/// 注销激活记录 (释放配额)
pub async fn deactivate(pool: &PgPool, entity_id: &str) -> Result<bool, sqlx::Error> {
    let now = shared::util::now_millis();
    let result = sqlx::query(
        "UPDATE activations SET status = 'deactivated', deactivated_at = $1 WHERE entity_id = $2 AND status = 'active'",
    )
    .bind(now)
    .bind(entity_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// 更新 last_refreshed_at
pub async fn update_last_refreshed(pool: &PgPool, entity_id: &str) -> Result<(), sqlx::Error> {
    let now = shared::util::now_millis();
    sqlx::query("UPDATE activations SET last_refreshed_at = $1 WHERE entity_id = $2")
        .bind(now)
        .bind(entity_id)
        .execute(pool)
        .await?;
    Ok(())
}
