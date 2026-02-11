//! Order Event Sourcing Module
//!
//! This module provides types for the order event sourcing system:
//! - Commands: Requests from clients to modify orders
//! - Events: Immutable facts recorded after command processing
//! - Snapshots: Computed order state from event stream

pub mod applied_mg_rule;
pub mod applied_rule;
pub mod command;
pub mod event;
pub mod snapshot;
pub mod types;

// Re-exports
pub use applied_mg_rule::AppliedMgRule;
pub use applied_rule::AppliedRule;
pub use command::{OrderCommand, OrderCommandPayload};
pub use event::{EventPayload, OrderEvent, OrderEventType};
pub use snapshot::{OrderSnapshot, OrderStatus};
pub use types::*;
