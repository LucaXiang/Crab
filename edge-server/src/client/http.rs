//! HTTP 客户端 - CrabClient 的 HTTP 后端
//!
//! 向 Edge Server 发起基于 HTTP 的网络调用。
//!
//! # 与 MessageClient 的区别
//!
//! | 特性 | Http | MessageClient |
//! |------|------|---------------|
//! | 通信方式 | REST API | TCP 消息总线 |
//! | 实时性 | 请求-响应 | 实时推送 |
//! | 适用场景 | CRUD 操作 | 实时通知 |

use async_trait::async_trait;
use reqwest::{Client, StatusCode as ReqwestStatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;

use super::{ApiResponse, CrabClient, CurrentUserResponse, LoginResponse};
use crate::AppError;
use crate::message::BusMessage;

/// HTTP 客户端
///
/// 用于向 Edge Server 发起 REST API 调用
///
/// # 示例
///
/// ```ignore
/// let client = Http::new("https://edge-server:3000")
///     .with_token("jwt-token");
///
/// let users: Vec<User> = client.get("/api/users").await?;
/// ```
#[derive(Debug, Clone)]
pub struct Http {
    /// HTTP 客户端
    client: Client,
    /// 服务器地址
    base_url: String,
    /// JWT 令牌
    token: Option<String>,
    /// TCP 消息总线地址
    tcp_addr: Option<String>,
}

impl Http {
    /// 创建 HTTP 客户端
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.into(),
            token: None,
            tcp_addr: None,
        }
    }

    /// 设置认证令牌
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// 设置 TCP 消息总线地址
    pub fn with_tcp_addr(mut self, addr: impl Into<String>) -> Self {
        self.tcp_addr = Some(addr.into());
        self
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    async fn handle_response<T: DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, AppError> {
        let status = response.status();

        if !status.is_success() {
            let text = response
                .text()
                .await
                .map_err(|e: reqwest::Error| AppError::internal(e.to_string()))?;
            return match status {
                ReqwestStatusCode::UNAUTHORIZED => Err(AppError::Unauthorized),
                ReqwestStatusCode::FORBIDDEN => Err(AppError::forbidden(text)),
                ReqwestStatusCode::NOT_FOUND => Err(AppError::not_found(text)),
                ReqwestStatusCode::BAD_REQUEST => Err(AppError::validation(text)),
                _ => Err(AppError::internal(text)),
            };
        }

        response
            .json()
            .await
            .map_err(|e| AppError::internal(e.to_string()))
    }
}

#[async_trait]
impl CrabClient for Http {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.get(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Self::handle_response(response).await
    }

    async fn post<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.post(&url).json(body);

        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Self::handle_response(response).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.post(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        Self::handle_response(response).await
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<LoginResponse, AppError> {
        #[derive(Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .post::<ApiResponse<LoginResponse>, _>("/api/auth/login", &request)
            .await?
            .data
            .ok_or_else(|| AppError::internal("Missing login data".to_string()))?;

        // Store the token for subsequent requests
        self.token = Some(response.token.clone());

        Ok(response)
    }

    async fn me(&self) -> Result<CurrentUserResponse, AppError> {
        self.get::<ApiResponse<CurrentUserResponse>>("/api/auth/me")
            .await?
            .data
            .ok_or_else(|| AppError::internal("Missing user data".to_string()))
    }

    async fn logout(&mut self) -> Result<(), AppError> {
        self.post_empty::<ApiResponse<()>>("/api/auth/logout")
            .await?;
        // Clear the token after logout
        self.token = None;
        Ok(())
    }

    fn subscribe(&self) -> Result<broadcast::Receiver<BusMessage>, AppError> {
        // HTTP 模式不支持实时消息订阅
        // 如需实时消息，请使用 MessageClient 直连 TCP 消息总线
        Err(AppError::internal(
            "HTTP client does not support message subscription. Use MessageClient instead.",
        ))
    }
}
