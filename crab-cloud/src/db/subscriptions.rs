use sqlx::PgPool;

pub struct CreateSubscription<'a> {
    pub id: &'a str,
    pub tenant_id: &'a str,
    pub plan: &'a str,
    pub max_edge_servers: i32,
    pub max_clients: i32,
    pub current_period_end: Option<i64>,
    pub now: i64,
}

pub async fn create(pool: &PgPool, sub: &CreateSubscription<'_>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO subscriptions (id, tenant_id, status, plan, max_edge_servers, max_clients, current_period_end, created_at)
         VALUES ($1, $2, 'active', $3, $4, $5, $6, $7)
         ON CONFLICT (id) DO UPDATE SET
            status = 'active', plan = $3, max_edge_servers = $4,
            max_clients = $5, current_period_end = $6",
    )
    .bind(sub.id)
    .bind(sub.tenant_id)
    .bind(sub.plan)
    .bind(sub.max_edge_servers)
    .bind(sub.max_clients)
    .bind(sub.current_period_end)
    .bind(sub.now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_status(
    pool: &PgPool,
    subscription_id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE subscriptions SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(subscription_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_tenant_by_sub_id(
    pool: &PgPool,
    stripe_sub_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT tenant_id FROM subscriptions WHERE id = $1")
            .bind(stripe_sub_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

/// 订阅记录 (从 crab-auth 合并，用于 PKI 端点)
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Subscription {
    pub id: String,
    pub tenant_id: String,
    pub status: String,
    pub plan: String,
    pub max_edge_servers: i32,
    pub max_clients: i32,
    pub features: Vec<String>,
    pub current_period_end: Option<i64>,
}

/// 获取租户当前有效订阅 (从 crab-auth 合并)
pub async fn get_active_subscription(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT id, tenant_id, status, plan, max_edge_servers, max_clients,
            features, current_period_end
            FROM subscriptions
            WHERE tenant_id = $1 AND status = 'active'
            ORDER BY created_at DESC
            LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}
