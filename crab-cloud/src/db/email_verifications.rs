use sqlx::PgPool;

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct EmailVerification {
    pub email: String,
    pub code: String,
    pub attempts: i32,
    pub expires_at: i64,
    pub created_at: i64,
    pub purpose: String,
    pub metadata: Option<String>,
}

pub async fn upsert(
    pool: &PgPool,
    email: &str,
    code_hash: &str,
    expires_at: i64,
    now: i64,
    purpose: &str,
    metadata: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO email_verifications (email, code, attempts, expires_at, created_at, purpose, metadata)
         VALUES ($1, $2, 0, $3, $4, $5, $6)
         ON CONFLICT (email, purpose) DO UPDATE SET
            code = $2, attempts = 0, expires_at = $3, created_at = $4, metadata = $6",
    )
    .bind(email)
    .bind(code_hash)
    .bind(expires_at)
    .bind(now)
    .bind(purpose)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find(
    pool: &PgPool,
    email: &str,
    purpose: &str,
) -> Result<Option<EmailVerification>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM email_verifications WHERE email = $1 AND purpose = $2")
        .bind(email)
        .bind(purpose)
        .fetch_optional(pool)
        .await
}

pub async fn increment_attempts(
    pool: &PgPool,
    email: &str,
    purpose: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE email_verifications SET attempts = attempts + 1 WHERE email = $1 AND purpose = $2",
    )
    .bind(email)
    .bind(purpose)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, email: &str, purpose: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_verifications WHERE email = $1 AND purpose = $2")
        .bind(email)
        .bind(purpose)
        .execute(pool)
        .await?;
    Ok(())
}
