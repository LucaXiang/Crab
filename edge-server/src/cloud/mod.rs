//! Cloud — duplex communication with crab-cloud (WebSocket + HTTP fallback)
//!
//! ```text
//! CloudWorker
//!   ├── Connect: WebSocket → receive Welcome{cursors}
//!   ├── Initial sync: compare cursors with local versions → skip unchanged resources
//!   ├── Startup: archived_order catch-up sync (cursor-based)
//!   ├── Listen: MessageBus server broadcast (Sync events) → debounced push via WS
//!   ├── Listen: WS incoming → RPC execution + SyncAck handling
//!   └── Reconnect: exponential backoff on WS disconnect
//! ```

pub mod ops;
pub mod rpc_executor;
mod service;
mod worker;

pub use service::CloudService;
pub use worker::CloudWorker;
