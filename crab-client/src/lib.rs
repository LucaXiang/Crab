//! Crab Client - 统一客户端接口
//!
//! 支持两种运行模式：
//! - **Network**: 通过 HTTP 连接远程 Edge Server
//! - **InProcess**: 同进程直接调用，零网络开销
//!
//! # 使用示例
//!
//! ```ignore
//! // 网络模式
//! let mut client = CrabClient::network("https://edge:3000");
//!
//! // 同进程模式 (需要 edge-server 依赖)
//! let mut client = CrabClient::in_process(server_state);
//!
//! // 统一 API
//! client.login("user", "pass").await?;
//! let me = client.me().await?;
//! ```

mod client;
pub mod error;

pub use client::{Client, CrabClient, InProcessClient, NetworkClient};
pub use error::{ClientError, ClientResult};

// Re-export shared types
pub use shared::client::{ApiResponse, CurrentUserResponse, LoginResponse, UserInfo};
