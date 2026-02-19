use sqlx::PgPool;

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: String,
    pub email: String,
    pub hashed_password: String,
    pub name: Option<String>,
    pub status: String,
    pub stripe_customer_id: Option<String>,
    pub created_at: i64,
    pub verified_at: Option<i64>,
}

#[allow(dead_code)]
pub async fn create(
    pool: &PgPool,
    id: &str,
    email: &str,
    hashed_password: &str,
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tenants (id, email, hashed_password, status, created_at)
         VALUES ($1, $2, $3, 'pending', $4)",
    )
    .bind(id)
    .bind(email)
    .bind(hashed_password)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Tenant>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tenants WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_stripe_customer(
    pool: &PgPool,
    customer_id: &str,
) -> Result<Option<Tenant>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tenants WHERE stripe_customer_id = $1")
        .bind(customer_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_status(
    pool: &PgPool,
    tenant_id: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET status = $1 WHERE id = $2")
        .bind(status)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_verified(pool: &PgPool, tenant_id: &str, now: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET status = 'verified', verified_at = $1 WHERE id = $2")
        .bind(now)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_stripe_customer(
    pool: &PgPool,
    tenant_id: &str,
    stripe_customer_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET stripe_customer_id = $1 WHERE id = $2")
        .bind(stripe_customer_id)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_password(
    pool: &PgPool,
    tenant_id: &str,
    hashed_password: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET hashed_password = $1 WHERE id = $2")
        .bind(hashed_password)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_email(
    pool: &PgPool,
    tenant_id: &str,
    new_email: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tenants SET email = $1 WHERE id = $2")
        .bind(new_email)
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_by_id(pool: &PgPool, id: &str) -> Result<Option<Tenant>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tenants WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// 通过用户名(id)查找租户并验证密码 (从 crab-auth 合并)
pub async fn authenticate(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<Option<Tenant>, sqlx::Error> {
    // Support login by tenant_id (UUID) or email
    let tenant: Option<Tenant> =
        sqlx::query_as("SELECT * FROM tenants WHERE id = $1 OR email = $1")
            .bind(username)
            .fetch_optional(pool)
            .await?;

    let Some(tenant) = tenant else {
        return Ok(None);
    };

    if tenant.status != "active" {
        return Ok(None);
    }

    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let hash = match PasswordHash::new(&tenant.hashed_password) {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };

    if Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
    {
        Ok(Some(tenant))
    } else {
        Ok(None)
    }
}
