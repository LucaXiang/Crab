// crab-client/src/client/http_oneshot.rs
// Oneshot HTTP 客户端 - 内存通信 (Local Mode)
//
// 需要启用 "in-process" feature

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::ApiErrorResponse;
use tower::ServiceExt;

use crate::{ClientError, ClientResult, CurrentUserResponse, LoginResponse};

use super::http::HttpClient;

/// Oneshot HTTP 客户端 (内存调用)
///
/// 使用 Tower Service 的 oneshot 模式直接调用 Router，
/// 适用于同进程的服务器-客户端通信，零网络开销。
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use crab_client::OneshotHttpClient;
///
/// let router: Router = build_app().with_state(state);
/// let client = OneshotHttpClient::new(router);
///
/// // 直接内存调用，无网络开销
/// let response: MyData = client.get("/api/data").await?;
/// ```
#[derive(Debug, Clone)]
pub struct OneshotHttpClient {
    router: Arc<RwLock<Router>>,
    token: Arc<RwLock<Option<String>>>,
}

impl OneshotHttpClient {
    /// 创建新的 Oneshot HTTP 客户端
    ///
    /// # Arguments
    /// * `router` - 已初始化的 Axum Router (with_state 已调用)
    pub fn new(router: Router) -> Self {
        Self {
            router: Arc::new(RwLock::new(router)),
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置认证 token
    pub async fn set_token(&self, token: Option<String>) {
        let mut guard = self.token.write().await;
        *guard = token;
    }

    /// 获取当前 token
    pub async fn get_token(&self) -> Option<String> {
        self.token.read().await.clone()
    }

    /// 构建带认证头的请求
    async fn build_request(&self, method: http::Method, path: &str) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(path);

        if let Some(token) = self.get_token().await {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        builder
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .expect("Failed to build request")
    }

    /// 构建带 body 的请求
    async fn build_request_with_body<B: serde::Serialize>(
        &self,
        method: http::Method,
        path: &str,
        body: &B,
    ) -> Result<Request<Body>, ClientError> {
        let body_bytes = serde_json::to_vec(body)?;

        let mut builder = Request::builder().method(method).uri(path);

        if let Some(token) = self.get_token().await {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        Ok(builder
            .header("Content-Type", "application/json")
            .body(Body::from(body_bytes))
            .expect("Failed to build request"))
    }

    /// 执行请求并处理响应
    async fn execute<T: DeserializeOwned>(&self, request: Request<Body>) -> ClientResult<T> {
        let router = self.router.read().await.clone();

        let response = router
            .oneshot(request)
            .await
            .map_err(|e| ClientError::Internal(format!("Oneshot call failed: {}", e)))?;

        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .map_err(|e| ClientError::Internal(format!("Failed to read body: {}", e)))?;

        if !status.is_success() {
            let text = String::from_utf8_lossy(&body_bytes).to_string();
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

        serde_json::from_slice(&body_bytes)
            .map_err(|e| ClientError::InvalidResponse(format!("JSON parse error: {}", e)))
    }
}

#[async_trait]
impl HttpClient for OneshotHttpClient {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let request = self.build_request(http::Method::GET, path).await;
        self.execute(request).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let request = self
            .build_request_with_body(http::Method::POST, path, body)
            .await?;
        self.execute(request).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let request = self.build_request(http::Method::POST, path).await;
        self.execute(request).await
    }

    async fn put<T: DeserializeOwned, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let request = self
            .build_request_with_body(http::Method::PUT, path, body)
            .await?;
        self.execute(request).await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let request = self.build_request(http::Method::DELETE, path).await;
        self.execute(request).await
    }

    async fn delete_with_body<T: DeserializeOwned, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let request = self
            .build_request_with_body(http::Method::DELETE, path, body)
            .await?;
        self.execute(request).await
    }

    async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        use shared::client::LoginRequest;

        let req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        self.post("/api/auth/login", &req).await
    }

    async fn me(&self) -> ClientResult<CurrentUserResponse> {
        self.get("/api/auth/me").await
    }

    async fn logout(&mut self) -> Result<(), ClientError> {
        let _resp: () = self.post_empty("/api/auth/logout").await?;
        self.set_token(None).await;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oneshot_client_creation() {
        // 创建一个简单的空 router 用于测试
        let router: Router = Router::new();
        let _client = OneshotHttpClient::new(router);
    }
}
