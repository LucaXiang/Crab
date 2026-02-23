//! Cloud RPC operation implementations — grouped by domain
//!
//! Each submodule exposes `pub(crate)` functions called by `rpc_executor::execute()`.

pub mod attribute;
pub mod catalog;
pub mod provisioning;
pub mod resource;
