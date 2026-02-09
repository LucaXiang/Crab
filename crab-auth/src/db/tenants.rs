use sqlx::PgPool;

/// 租户记录（只读，由 SaaS 管理平台写入）
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub hashed_password: String,
    pub status: String,
}

/// 通过用户名查找租户并验证密码
pub async fn authenticate(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant: Option<Tenant> = sqlx::query_as(
        "SELECT id, name, hashed_password, status FROM tenants WHERE id = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    let Some(tenant) = tenant else {
        return Ok(None);
    };

    if tenant.status != "active" {
        return Ok(None);
    }

    // 验证 argon2 密码哈希
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
