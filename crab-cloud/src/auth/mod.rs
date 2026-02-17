//! Authentication middleware for edge-server and tenant connections

pub mod edge_auth;
pub mod quota;
pub mod rate_limit;
pub mod tenant_auth;

pub use edge_auth::EdgeIdentity;
pub use quota::QuotaCache;
