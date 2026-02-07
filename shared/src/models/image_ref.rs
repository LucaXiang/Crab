//! Image Reference Model

use serde::{Deserialize, Serialize};

/// Entity type for image references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageRefEntityType {
    Product,
    LabelTemplate,
}

impl ImageRefEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageRefEntityType::Product => "product",
            ImageRefEntityType::LabelTemplate => "label_template",
        }
    }
}

impl std::fmt::Display for ImageRefEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Image reference record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct ImageRef {
    pub id: i64,
    pub hash: String,
    pub entity_type: String,
    pub entity_id: String,
    pub created_at: i64,
}
