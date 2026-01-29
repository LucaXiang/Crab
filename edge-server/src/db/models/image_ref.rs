//! Image Reference Model
//!
//! 图片引用计数，用于跟踪图片被哪些实体引用

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// 实体类型枚举
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
pub struct ImageRef {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    /// 图片哈希 (SHA256)
    pub hash: String,
    /// 引用实体类型
    pub entity_type: String,
    /// 引用实体 ID
    pub entity_id: String,
    /// 创建时间
    pub created_at: Option<i64>,
}

impl ImageRef {
    pub fn new(hash: String, entity_type: ImageRefEntityType, entity_id: String) -> Self {
        Self {
            id: None,
            hash,
            entity_type: entity_type.to_string(),
            entity_id,
            created_at: None,
        }
    }
}

/// Create image reference payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRefCreate {
    pub hash: String,
    pub entity_type: String,
    pub entity_id: String,
}
