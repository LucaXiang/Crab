//! Label Template Model
//!
//! Types for label printing templates

use serde::{Deserialize, Serialize};

/// Label field type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
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
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: LabelFieldType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_font_size")]
    pub font_size: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<LabelFieldAlignment>,
    pub data_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintain_aspect_ratio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_style: Option<String>,
}

fn default_font_size() -> i32 {
    10
}

fn default_true() -> bool {
    true
}

/// Label template entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelTemplate {
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_padding")]
    pub padding: f32,
    #[serde(default)]
    pub fields: Vec<LabelField>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_dpi: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_data: Option<String>,
}

fn default_padding() -> f32 {
    2.0
}

/// Create label template payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelTemplateCreate {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_padding")]
    pub padding: f32,
    #[serde(default)]
    pub fields: Vec<LabelField>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f32>,
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
    pub padding: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<LabelField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm_y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_dpi: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_data: Option<String>,
}
