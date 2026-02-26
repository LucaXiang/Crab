//! Stamp Activity & Progress Models

use serde::{Deserialize, Serialize};

/// Reward strategy for stamp activities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum RewardStrategy {
    /// Comp cheapest qualifying item in order (default)
    Economizador,
    /// Comp most expensive qualifying item in order
    Generoso,
    /// Comp a designated fixed product
    Designated,
}

/// Stamp target type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum StampTargetType {
    Category,
    Product,
}

/// Stamp Activity entity (集章活动)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StampActivity {
    pub id: i64,
    pub marketing_group_id: i64,
    pub name: String,
    pub stamps_required: i32,
    pub reward_quantity: i32,
    pub reward_strategy: RewardStrategy,
    pub designated_product_id: Option<i64>,
    pub is_cyclic: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Create stamp activity payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampActivityCreate {
    pub name: String,
    pub stamps_required: i32,
    pub reward_quantity: Option<i32>,
    pub reward_strategy: Option<RewardStrategy>,
    pub designated_product_id: Option<i64>,
    pub is_cyclic: Option<bool>,
    pub stamp_targets: Vec<StampTargetInput>,
    pub reward_targets: Vec<StampTargetInput>,
}

/// Update stamp activity payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampActivityUpdate {
    pub name: Option<String>,
    pub stamps_required: Option<i32>,
    pub reward_quantity: Option<i32>,
    pub reward_strategy: Option<RewardStrategy>,
    pub designated_product_id: Option<i64>,
    pub is_cyclic: Option<bool>,
    pub is_active: Option<bool>,
    pub stamp_targets: Option<Vec<StampTargetInput>>,
    pub reward_targets: Option<Vec<StampTargetInput>>,
}

/// Input for stamp/reward target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampTargetInput {
    pub target_type: StampTargetType,
    pub target_id: i64,
}

/// Stamp target record (from DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StampTarget {
    pub id: i64,
    pub stamp_activity_id: i64,
    pub target_type: StampTargetType,
    pub target_id: i64,
}

/// Stamp reward target record (from DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StampRewardTarget {
    pub id: i64,
    pub stamp_activity_id: i64,
    pub target_type: StampTargetType,
    pub target_id: i64,
}

/// Member stamp progress
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MemberStampProgress {
    pub id: i64,
    pub member_id: i64,
    pub stamp_activity_id: i64,
    pub current_stamps: i32,
    pub completed_cycles: i32,
    pub last_stamp_at: Option<i64>,
    pub updated_at: i64,
}

/// Stamp activity with targets (for detail/config views)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampActivityDetail {
    #[serde(flatten)]
    pub activity: StampActivity,
    pub stamp_targets: Vec<StampTarget>,
    pub reward_targets: Vec<StampRewardTarget>,
}

/// Member stamp progress with activity info (for member detail view)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MemberStampProgressDetail {
    pub stamp_activity_id: i64,
    pub stamp_activity_name: String,
    pub stamps_required: i32,
    pub current_stamps: i32,
    pub completed_cycles: i32,
    pub is_redeemable: bool,
    pub is_cyclic: bool,
    pub reward_strategy: RewardStrategy,
    pub reward_quantity: i32,
    pub designated_product_id: Option<i64>,
}
