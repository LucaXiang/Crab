use sqlx::PgPool;

/// P12 证书记录（数据和密码直接存 PG）
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct P12Certificate {
    pub tenant_id: String,
    pub p12_data: Option<String>,
    pub p12_password: Option<String>,
    pub fingerprint: Option<String>,
    pub common_name: Option<String>,
    pub serial_number: Option<String>,
    pub organization_id: Option<String>,
    pub organization: Option<String>,
    pub issuer: Option<String>,
    pub country: Option<String>,
    pub expires_at: Option<i64>,
    pub not_before: Option<i64>,
    pub uploaded_at: i64,
    pub updated_at: i64,
}

/// 查询租户的 P12 证书记录
#[allow(dead_code)]
pub async fn find_by_tenant(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<P12Certificate>, sqlx::Error> {
    sqlx::query_as::<_, P12Certificate>(
        "SELECT tenant_id, p12_data, p12_password, fingerprint, common_name, serial_number,
            organization_id, organization, issuer, country,
            expires_at, not_before, uploaded_at, updated_at
            FROM p12_certificates
            WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
}

/// 获取租户的 P12 证书状态 (供 SubscriptionInfo 使用)
pub async fn get_p12_info(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<shared::activation::P12Info, sqlx::Error> {
    match find_by_tenant(pool, tenant_id).await? {
        Some(cert) => Ok(shared::activation::P12Info {
            has_p12: true,
            fingerprint: cert.fingerprint,
            subject: cert.common_name,
            expires_at: cert.expires_at,
        }),
        None => Ok(shared::activation::P12Info {
            has_p12: false,
            fingerprint: None,
            subject: None,
            expires_at: None,
        }),
    }
}

/// 插入或更新 P12 证书（数据和密码直接存 PG）
pub async fn upsert(
    pool: &PgPool,
    tenant_id: &str,
    p12_data: &str,
    p12_password: &str,
    info: &crab_cert::P12CertInfo,
) -> Result<(), sqlx::Error> {
    let now = shared::util::now_millis();
    sqlx::query(
        "INSERT INTO p12_certificates
            (tenant_id, p12_data, p12_password, fingerprint, common_name, serial_number,
             organization_id, organization, issuer, country,
             expires_at, not_before, uploaded_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
            ON CONFLICT (tenant_id)
            DO UPDATE SET p12_data = $2, p12_password = $3, fingerprint = $4, common_name = $5,
                          serial_number = $6, organization_id = $7, organization = $8,
                          issuer = $9, country = $10, expires_at = $11, not_before = $12,
                          updated_at = $13",
    )
    .bind(tenant_id)
    .bind(p12_data)
    .bind(p12_password)
    .bind(&info.fingerprint)
    .bind(&info.common_name)
    .bind(&info.serial_number)
    .bind(&info.organization_id)
    .bind(&info.organization)
    .bind(&info.issuer)
    .bind(&info.country)
    .bind(info.expires_at)
    .bind(info.not_before)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
