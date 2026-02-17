//! Cloud sync — push local data to crab-cloud
//!
//! ```text
//! CloudSyncWorker
//!   ├── Startup: full sync (all resources)
//!   ├── Listen: MessageBus server broadcast (Sync events) → debounced push
//!   ├── Periodic: full sync every hour
//!   └── Execute: cloud commands received via sync response
//! ```

pub mod command_executor;
mod service;
mod worker;

pub use service::CloudSyncService;
pub use worker::CloudSyncWorker;
