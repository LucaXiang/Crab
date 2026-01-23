//! Employee Model

use super::RoleId;
use super::serde_helpers;
use super::serde_thing;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Employee ID type
pub type EmployeeId = Thing;

/// Employee model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    #[serde(default, with = "serde_thing::option")]
    pub id: Option<EmployeeId>,
    pub username: String,
    #[serde(rename = "employee_name")]
    pub display_name: String,
    #[serde(skip_serializing)]
    pub hash_pass: String,
    #[serde(with = "serde_thing")]
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
    #[serde(with = "serde_thing")]
    pub role: RoleId,
}

/// Update employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, with = "serde_thing::option", skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleId>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
