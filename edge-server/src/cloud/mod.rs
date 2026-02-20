//! Cloud — duplex communication with crab-cloud (WebSocket + HTTP fallback)
//!
//! ```text
//! CloudWorker
//!   ├── Startup: WebSocket connect → full sync (products + categories)
//!   ├── Startup: archived_order catch-up sync (cursor-based)
//!   ├── Listen: MessageBus server broadcast (Sync events) → debounced push via WS
//!   ├── Listen: WS incoming → Command execution + SyncAck handling
//!   ├── Periodic: full sync every hour
//!   └── Reconnect: exponential backoff on WS disconnect
//! ```

pub mod command_executor;
mod service;
mod worker;

pub use service::CloudService;
pub use worker::CloudWorker;
