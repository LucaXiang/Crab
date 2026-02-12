//! Member Model

use serde::{Deserialize, Serialize};

/// Member entity (会员)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Member {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: i64,
    pub birthday: Option<String>,
    pub email: Option<String>,
    pub points_balance: i64,
    pub total_spent: f64,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Create member payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberCreate {
    pub name: String,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: i64,
    pub birthday: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
}

/// Update member payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberUpdate {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: Option<i64>,
    pub birthday: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub is_active: Option<bool>,
}

/// Member with marketing group info (for list/detail views)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MemberWithGroup {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: i64,
    pub marketing_group_name: String,
    pub birthday: Option<String>,
    pub email: Option<String>,
    pub points_balance: i64,
    pub total_spent: f64,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
