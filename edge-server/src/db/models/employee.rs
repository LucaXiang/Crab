//! Employee Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use super::RoleId;

/// Employee ID type
pub type EmployeeId = Thing;

/// Employee model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: Option<EmployeeId>,
    pub username: String,
    pub hash_pass: String,
    pub role: RoleId,
    pub is_system: bool,
    pub is_active: bool,
}

/// Employee response (without password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeResponse {
    pub id: String,
    pub username: String,
    pub role: RoleId,
    pub is_active: bool,
}

impl From<Employee> for EmployeeResponse {
    fn from(emp: Employee) -> Self {
        Self {
            id: emp.id.map(|t| t.to_string()).unwrap_or_default(),
            username: emp.username,
            role: emp.role,
            is_active: emp.is_active,
        }
    }
}

impl Employee {
    /// Verify password using argon2
    pub fn verify_password(&self, password: &str) -> Result<bool, argon2::password_hash::Error> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };

        let parsed_hash = PasswordHash::new(&self.hash_pass)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Hash password using argon2
    pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(password_hash.to_string())
    }
}
