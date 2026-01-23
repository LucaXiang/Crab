// crab-client/src/client/http.rs
// HTTP 客户端 - 网络通信

use crate::{ClientError, ClientResult, CurrentUserResponse, LoginResponse};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

/// Internal response wrapper for Edge Server API (which uses success/data/error format)
#[derive(serde::Deserialize)]
struct EdgeResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// 服务端返回的错误响应格式
#[derive(serde::Deserialize)]
struct ApiErrorResponse {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

/// HTTP 客户端 trait
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn post<T: DeserializeOwned, B: serde::Serialize + std::marker::Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T>;
    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn put<T: DeserializeOwned, B: serde::Serialize + std::marker::Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T>;
    async fn delete<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T>;
    async fn delete_with_body<T: DeserializeOwned, B: serde::Serialize + std::marker::Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T>;
    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse>;
    async fn me(&self) -> ClientResult<CurrentUserResponse>;
    async fn logout(&mut self) -> Result<(), ClientError>;
    fn token(&self) -> Option<&str>;
}

/// 网络 HTTP 客户端
#[derive(Debug, Clone)]
pub struct NetworkHttpClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl NetworkHttpClient {
    pub fn new(base_url: &str) -> Result<Self, ClientError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        })
    }

    /// 获取基础 URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> ClientResult<T> {
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await?;
            // 尝试解析为 API 错误响应
            if let Ok(api_err) = serde_json::from_str::<ApiErrorResponse>(&text) {
                return Err(ClientError::Api {
                    code: api_err.code,
                    message: api_err.message,
                    details: api_err.details,
                });
            }
            // 降级到原来的处理方式
            return match status {
                StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized("Unauthorized".into())),
                StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
                StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
                StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
                _ => Err(ClientError::Internal(text)),
            };
        }
        Ok(response.json().await?)
    }
}

#[async_trait]
impl HttpClient for NetworkHttpClient {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.get(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = req.send().await?;
        self.handle_response(response).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize + std::marker::Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);
        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = req.send().await?;
        self.handle_response(response).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.post(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = req.send().await?;
        self.handle_response(response).await
    }

    async fn put<T: DeserializeOwned, B: serde::Serialize + std::marker::Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.put(&url).json(body);
        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = req.send().await?;
        self.handle_response(response).await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.delete(&url);
        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = req.send().await?;
        self.handle_response(response).await
    }

    async fn delete_with_body<T: DeserializeOwned, B: serde::Serialize + std::marker::Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url, path);
        let mut req = self.client.delete(&url).json(body);
        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = req.send().await?;
        self.handle_response(response).await
    }

    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        use shared::client::LoginRequest;
        let req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };
        let resp: EdgeResponse<LoginResponse> = self.post("/api/auth/login", &req).await?;
        if !resp.success {
            return Err(ClientError::Auth(
                resp.error.unwrap_or_else(|| "Unknown error".into()),
            ));
        }
        resp.data
            .ok_or_else(|| ClientError::InvalidResponse("Missing login data".into()))
    }

    async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let resp: EdgeResponse<CurrentUserResponse> = self.get("/api/auth/me").await?;
        if !resp.success {
            return Err(ClientError::Auth(
                resp.error.unwrap_or_else(|| "Unknown error".into()),
            ));
        }
        resp.data
            .ok_or_else(|| ClientError::InvalidResponse("Missing user data".into()))
    }

    async fn logout(&mut self) -> Result<(), ClientError> {
        let _resp: EdgeResponse<()> = self.post_empty("/api/auth/logout").await?;
        self.token = None;
        Ok(())
    }

    fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}
