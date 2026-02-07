//! Label Template Model

use serde::{Deserialize, Serialize};

/// Label field type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "lowercase"))]
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
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "lowercase"))]
pub enum LabelFieldAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Label template field (independent table)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct LabelField {
    pub id: i64,
    pub template_id: i64,
    /// Client-generated UUID
    pub field_id: String,
    pub name: String,
    pub field_type: LabelFieldType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_font_size")]
    pub font_size: i32,
    pub font_weight: Option<String>,
    pub font_family: Option<String>,
    pub color: Option<String>,
    pub rotate: Option<i32>,
    pub alignment: Option<LabelFieldAlignment>,
    pub data_source: String,
    pub format: Option<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    pub label: Option<String>,
    pub template: Option<String>,
    pub data_key: Option<String>,
    pub source_type: Option<String>,
    pub maintain_aspect_ratio: Option<bool>,
    pub style: Option<String>,
    pub align: Option<String>,
    pub vertical_align: Option<String>,
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
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct LabelTemplate {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_padding")]
    pub padding: f32,
    pub is_default: bool,
    pub is_active: bool,
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
    pub padding_mm_x: Option<f32>,
    pub padding_mm_y: Option<f32>,
    pub render_dpi: Option<i32>,
    pub test_data: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,

    // -- Relations (populated by application code, skipped by FromRow) --

    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub fields: Vec<LabelField>,
}

fn default_padding() -> f32 {
    2.0
}

/// Label field input (for create/update, without id/template_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelFieldInput {
    /// Client-generated UUID
    pub field_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: LabelFieldType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_font_size")]
    pub font_size: i32,
    pub font_weight: Option<String>,
    pub font_family: Option<String>,
    pub color: Option<String>,
    pub rotate: Option<i32>,
    pub alignment: Option<LabelFieldAlignment>,
    pub data_source: String,
    pub format: Option<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    pub label: Option<String>,
    pub template: Option<String>,
    pub data_key: Option<String>,
    pub source_type: Option<String>,
    pub maintain_aspect_ratio: Option<bool>,
    pub style: Option<String>,
    pub align: Option<String>,
    pub vertical_align: Option<String>,
    pub line_style: Option<String>,
}

/// Create label template payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelTemplateCreate {
    pub name: String,
    pub description: Option<String>,
    pub width: f32,
    pub height: f32,
    #[serde(default = "default_padding")]
    pub padding: f32,
    #[serde(default)]
    pub fields: Vec<LabelFieldInput>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
    pub padding_mm_x: Option<f32>,
    pub padding_mm_y: Option<f32>,
    pub render_dpi: Option<i32>,
    pub test_data: Option<String>,
}

/// Update label template payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabelTemplateUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub padding: Option<f32>,
    pub fields: Option<Vec<LabelFieldInput>>,
    pub is_default: Option<bool>,
    pub is_active: Option<bool>,
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
    pub padding_mm_x: Option<f32>,
    pub padding_mm_y: Option<f32>,
    pub render_dpi: Option<i32>,
    pub test_data: Option<String>,
}
