use sqlx::PgPool;

/// P12 证书元数据记录
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct P12Certificate {
    pub tenant_id: String,
    pub s3_key: String,
    pub p12_password: String,
    pub fingerprint: Option<String>,
    pub subject: Option<String>,
    pub expires_at: Option<i64>,
    pub uploaded_at: i64,
    pub updated_at: i64,
}

/// 查询租户的 P12 证书记录（签名服务使用）
#[allow(dead_code)]
pub async fn find_by_tenant(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<P12Certificate>, sqlx::Error> {
    sqlx::query_as::<_, P12Certificate>(
        "SELECT tenant_id, s3_key, p12_password, fingerprint, subject,
            expires_at, uploaded_at, updated_at
            FROM p12_certificates
            WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}

/// 插入或更新 P12 证书记录
pub async fn upsert(
    pool: &PgPool,
    tenant_id: &str,
    s3_key: &str,
    p12_password: &str,
    fingerprint: Option<&str>,
    subject: Option<&str>,
    expires_at: Option<i64>,
) -> Result<(), sqlx::Error> {
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO p12_certificates (tenant_id, s3_key, p12_password, fingerprint, subject, expires_at, uploaded_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
            ON CONFLICT (tenant_id)
            DO UPDATE SET s3_key = $2, p12_password = $3, fingerprint = $4, subject = $5,
                          expires_at = $6, updated_at = $7",
    )
    .bind(tenant_id)
    .bind(s3_key)
    .bind(p12_password)
    .bind(fingerprint)
    .bind(subject)
    .bind(expires_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
