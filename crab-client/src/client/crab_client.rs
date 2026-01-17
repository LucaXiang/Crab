// crab-client/src/client/crab_client.rs
// 统一的 CrabClient - 支持远程和本地模式

use std::marker::PhantomData;
use crate::ClientError;

/// 远程模式标记
pub struct RemoteMode;

/// 本地模式标记
pub struct LocalMode;

/// 统一的 CrabClient
#[derive(Debug, Clone)]
pub struct CrabClient<M> {
    _mode: PhantomData<M>,
    base_url: String,
    token: Option<String>,
}

impl<M> CrabClient<M> {
    /// 获取 token
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// 设置 token
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }
}

impl CrabClient<RemoteMode> {
    /// 创建远程客户端
    pub fn new(base_url: &str) -> Self {
        Self {
            _mode: PhantomData,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        }
    }
}

impl CrabClient<LocalMode> {
    /// 创建本地客户端
    pub fn new() -> Self {
        Self {
            _mode: PhantomData,
            base_url: String::new(),
            token: None,
        }
    }
}
