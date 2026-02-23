//! Console WebSocket protocol types
//!
//! 独立于 CloudMessage 的 console 协议，用于 crab-cloud → crab-console 实时推送。
//! 设计为可扩展：未来可添加其他实时数据类型（不仅限于订单）。

pub mod ws;

pub use ws::*;
