//! Crab Client - 统一客户端接口
//!
//! 支持两种运行模式：
//! - **RemoteMode**: 通过 HTTPS + TCP/TLS 连接远程 Edge Server
//! - **LocalMode**: 同进程直接调用，零网络开销

mod cert;
mod client;
pub mod error;
pub mod message;

pub use cert::{Credential, CredentialStorage, CertManager, CertError};
pub use client::{
    CrabClient, RemoteMode, LocalMode, HttpClient, NetworkMessageClient,
    InMemoryMessageClient, MessageClientConfig,
};
pub use error::{ClientError, ClientResult};
pub use message::{MessageError, MessageResult, BusMessage, EventType};

// Re-export shared types
pub use shared::client::{ApiResponse, CurrentUserResponse, LoginResponse, UserInfo};
