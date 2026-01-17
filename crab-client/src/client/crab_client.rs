// crab-client/src/client/crab_client.rs
// 统一的 CrabClient - 支持远程和本地模式

use std::marker::PhantomData;
use crate::{ClientResult, LoginResponse, CurrentUserResponse, HttpClient};
use crate::client::NetworkHttpClient;

/// 远程模式标记
pub struct RemoteMode;

/// 本地模式标记
pub struct LocalMode;

/// 统一的 CrabClient
#[derive(Debug, Clone)]
pub struct CrabClient<M> {
    _mode: PhantomData<M>,
    http: NetworkHttpClient,
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

    /// 检查是否已登录
    pub fn is_logged_in(&self) -> bool {
        self.token.is_some()
    }
}

impl CrabClient<RemoteMode> {
    /// 创建远程客户端
    pub fn new(base_url: &str) -> Self {
        let http = NetworkHttpClient::new(base_url).expect("Failed to create HTTP client");
        Self {
            _mode: PhantomData,
            http,
            token: None,
        }
    }

    /// 登录
    pub async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        let resp = self.http.login(username, password).await?;
        self.token = Some(resp.token.clone());
        Ok(resp)
    }

    /// 获取当前用户
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        self.http.me().await
    }

    /// 登出
    pub async fn logout(&mut self) -> ClientResult<()> {
        self.http.logout().await?;
        self.token = None;
        Ok(())
    }
}

impl CrabClient<LocalMode> {
    /// 创建本地客户端
    pub fn new(base_url: &str) -> Self {
        let http = NetworkHttpClient::new(base_url).expect("Failed to create HTTP client");
        Self {
            _mode: PhantomData,
            http,
            token: None,
        }
    }

    /// 登录
    pub async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        let resp = self.http.login(username, password).await?;
        self.token = Some(resp.token.clone());
        Ok(resp)
    }

    /// 获取当前用户
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        self.http.me().await
    }

    /// 登出
    pub async fn logout(&mut self) -> ClientResult<()> {
        self.http.logout().await?;
        self.token = None;
        Ok(())
    }
}
