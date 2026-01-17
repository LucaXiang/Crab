// crab-client/src/client/crab_client.rs
// 统一的 CrabClient - 支持远程和本地模式

use std::marker::PhantomData;
use crate::{ClientResult, LoginResponse, CurrentUserResponse, ClientError, HttpClient};

/// 远程模式标记
pub struct RemoteMode;

/// 本地模式标记
pub struct LocalMode;

/// 统一的 CrabClient - 泛型存储不同类型的 HTTP 客户端
#[derive(Debug, Clone)]
pub struct CrabClient<M> {
    _mode: PhantomData<M>,
    http: Option<crate::client::NetworkHttpClient>,
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
        let http = crate::client::NetworkHttpClient::new(base_url).expect("Failed to create HTTP client");
        Self {
            _mode: PhantomData,
            http: Some(http),
            token: None,
        }
    }

    /// 登录
    pub async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        let http = self.http.as_mut().expect("HTTP client not initialized");
        let resp = http.login(username, password).await?;
        self.token = Some(resp.token.clone());
        Ok(resp)
    }

    /// 获取当前用户
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let http = self.http.as_ref().expect("HTTP client not initialized");
        http.me().await
    }

    /// 登出
    pub async fn logout(&mut self) -> ClientResult<()> {
        let http = self.http.as_mut().expect("HTTP client not initialized");
        http.logout().await?;
        self.token = None;
        Ok(())
    }
}

impl Default for CrabClient<LocalMode> {
    fn default() -> Self {
        Self::new()
    }
}

impl CrabClient<LocalMode> {
    /// 创建本地客户端 (不需要 base_url)
    pub fn new() -> Self {
        Self {
            _mode: PhantomData,
            http: None,
            token: None,
        }
    }

    /// LocalMode 不支持远程登录
    /// 登录应通过 CertManager 进行证书认证
    pub async fn login(&mut self, _username: &str, _password: &str) -> ClientResult<LoginResponse> {
        Err(ClientError::NotFound("LocalMode does not support login. Use CertManager for authentication.".into()))
    }

    /// LocalMode 不支持远程用户查询
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        Err(ClientError::NotFound("LocalMode does not support /me endpoint".into()))
    }

    /// 登出
    pub async fn logout(&mut self) -> ClientResult<()> {
        self.token = None;
        Ok(())
    }
}
