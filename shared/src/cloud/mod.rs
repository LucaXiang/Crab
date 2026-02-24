//! Cloud sync types for edge-server → crab-cloud data synchronization

pub mod store_op;
pub mod sync;
pub mod ws;

pub use sync::*;
pub use ws::*;

use serde::{Deserialize, Serialize};

/// Tenant registration/lifecycle status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantStatus {
    /// Registered, awaiting email verification
    Pending,
    /// Email verified, awaiting plan selection & payment
    Verified,
    /// Payment completed, fully active
    Active,
    /// Payment failed or past due
    Suspended,
    /// Subscription canceled
    Canceled,
}

impl TenantStatus {
    /// Parse from database string value (lowercase)
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "verified" => Some(Self::Verified),
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }

    /// Database string representation (lowercase)
    pub fn as_db(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Verified => "verified",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Canceled => "canceled",
        }
    }

    /// Can this tenant log in?
    /// All states except Pending can login — canceled/suspended tenants
    /// can still access the console (view history, resubscribe).
    pub fn can_login(&self) -> bool {
        !matches!(self, Self::Pending)
    }
}
