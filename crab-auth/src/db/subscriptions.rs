use sqlx::PgPool;

/// 订阅记录（只读，由 Stripe webhook 处理服务写入）
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Subscription {
    pub id: String,
    pub tenant_id: String,
    pub status: String,
    pub plan: String,
    pub max_edge_servers: i32,
    pub features: Vec<String>,
    pub current_period_end: Option<i64>,
}

/// 获取租户当前有效订阅
pub async fn get_active_subscription(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT id, tenant_id, status, plan, max_edge_servers,
            features, current_period_end
            FROM subscriptions
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}
