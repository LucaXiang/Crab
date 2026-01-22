//! Employee Model

use super::RoleId;
use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Employee ID type
pub type EmployeeId = Thing;

/// Employee model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: Option<EmployeeId>,
    pub username: String,
    #[serde(rename = "employee_name")]
    pub display_name: String,
    pub hash_pass: String,
    pub role: RoleId,
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_system: bool,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Create employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeCreate {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub role: RoleId,
}

/// Update employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeUpdate {
    pub username: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<RoleId>,
    pub is_active: Option<bool>,
}

impl Employee {
    /// Verify password using argon2
    pub fn verify_password(&self, password: &str) -> Result<bool, argon2::password_hash::Error> {
        use argon2::{
            Argon2,
            password_hash::{PasswordHash, PasswordVerifier},
        };

        let parsed_hash = PasswordHash::new(&self.hash_pass)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Hash password using argon2
    pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(password_hash.to_string())
    }
}
