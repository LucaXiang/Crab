//! Order Event Sourcing Module for Edge Server
//!
//! This module implements the order management system using event sourcing:
//!
//! - **manager**: Core OrdersManager for command processing and event generation
//! - **storage**: redb-based persistence layer for events, snapshots, and indices
//! - **reducer**: Event replay and snapshot computation
//! - **sync**: Reconnection synchronization API
//!
//! # Architecture
//!
//! ```text
//! Command → OrdersManager → Event → Storage (redb)
//!                 ↓                      ↓
//!              Broadcast          Snapshot Update
//!                 ↓
//!           All Subscribers
//! ```
//!
//! # Data Flow
//!
//! 1. Client sends OrderCommand via MessageBus
//! 2. OrdersManager validates and processes command
//! 3. OrderEvent is generated with global sequence
//! 4. Event is persisted to redb (transactional)
//! 5. Snapshot is updated
//! 6. Event is broadcast to all subscribers
//! 7. CommandResponse is returned to client

pub mod actions;
pub mod appliers;
pub mod archive;
pub mod archive_worker;
pub mod manager;
pub mod money;
pub mod reducer;
pub mod storage;
pub mod sync;
pub mod traits;
pub mod verify_scheduler;

// Re-exports
pub use archive::{ArchiveError, ArchiveResult, OrderArchiveService};
pub use archive_worker::ArchiveWorker;
pub use manager::OrdersManager;
pub use reducer::{generate_instance_id, input_to_snapshot};
pub use storage::OrderStorage;
pub use sync::{SyncRequest, SyncResponse};
pub use verify_scheduler::VerifyScheduler;

// Re-export shared types for convenience
pub use shared::order::{
    CommandError, CommandErrorCode, CommandResponse, EventPayload, OrderCommand,
    OrderCommandPayload, OrderEvent, OrderEventType, OrderSnapshot, OrderStatus,
};
