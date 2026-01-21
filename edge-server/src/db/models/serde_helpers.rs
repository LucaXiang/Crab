//! Common serde helpers for handling null values from SurrealDB

use serde::{Deserialize, Deserializer};

/// Deserialize bool that treats null as true
pub fn bool_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|opt| opt.unwrap_or(true))
}

/// Deserialize bool that treats null as false
pub fn bool_false<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|opt| opt.unwrap_or(false))
}
