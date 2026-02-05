//! Label Template Model
//!
//! 标签打印模板，支持自定义字段布局

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Label field type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LabelFieldType {
    #[default]
    Text,
    Barcode,
    Qrcode,
    Image,
    Separator,
    Datetime,
    Price,
    Counter,
}


/// Label field alignment
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LabelFieldAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Label template field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelField {
    /// Field ID (unique within template)
    pub id: String,
    /// Field display name
    pub name: String,
    /// Field type
    #[serde(rename = "type")]
    pub field_type: LabelFieldType,
    /// X position in mm
    pub x: f32,
    /// Y position in mm
    pub y: f32,
    /// Width in mm
    pub width: f32,
    /// Height in mm
    pub height: f32,
    /// Font size
    pub font_size: i32,
    /// Font weight (e.g., "normal", "bold")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<String>,
    /// Font family
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// Text color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Rotation in degrees
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<i32>,
    /// Text alignment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<LabelFieldAlignment>,
    /// Data source path (e.g., "product.name")
    pub data_source: String,
    /// Format pattern (e.g., for date/time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Whether the field is visible
    pub visible: bool,
    /// UI-specific: label text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// UI-specific: template string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// UI-specific: data key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_key: Option<String>,
    /// UI-specific: source type for image fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// UI-specific: maintain aspect ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintain_aspect_ratio: Option<bool>,
    /// UI-specific: style string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// UI-specific: horizontal align
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// UI-specific: vertical align
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    /// UI-specific: line style for separator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_style: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Label template entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelTemplate {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    /// Template name
    pub name: String,
    /// Template description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Label width in mm
    pub width: f32,
    /// Label height in mm
    pub height: f32,
    /// Template fields
    pub fields: Vec<LabelField>,
    /// Is default template
    pub is_default: bool,
    /// Is active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: Option<i64>,
    /// Updated timestamp
    pub updated_at: Option<i64>,
    /// Horizontal padding in mm
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_x: Option<f32>,
    /// Vertical padding in mm
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_y: Option<f32>,
    /// Render DPI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_dpi: Option<i32>,
    /// Test data JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_data: Option<String>,
}

/// Create label template payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelTemplateCreate {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub fields: Vec<LabelField>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_dpi: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_data: Option<String>,
}

/// Update label template payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabelTemplateUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<LabelField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_dpi: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_data: Option<String>,
}
