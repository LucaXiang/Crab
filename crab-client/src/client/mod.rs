// crab-client/src/client/mod.rs
// 统一客户端模块

pub use self::crab_client::{CrabClient, RemoteMode, LocalMode};
pub use self::http::{HttpClient, NetworkHttpClient};
pub use self::message::{MessageClient, NetworkMessageClient};

mod crab_client;
mod http;
mod message;
