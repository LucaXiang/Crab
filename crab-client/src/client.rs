//! 统一客户端实现

use crate::{ApiResponse, ClientError, ClientResult, CurrentUserResponse, LoginResponse};
use async_trait::async_trait;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

// ============================================================================
// Client Trait
// ============================================================================

/// 统一客户端接口
#[async_trait]
pub trait Client: Send + Sync {
    /// 登录
    async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse>;

    /// 获取当前用户信息
    async fn me(&self) -> ClientResult<CurrentUserResponse>;

    /// 登出
    async fn logout(&mut self) -> ClientResult<()>;

    /// 获取当前 token
    fn token(&self) -> Option<&str>;
}

// ============================================================================
// CrabClient Factory
// ============================================================================

/// 客户端工厂
pub struct CrabClient;

impl CrabClient {
    /// 创建网络客户端
    pub fn network(base_url: &str) -> NetworkClient {
        NetworkClient::new(base_url)
    }

    /// 创建网络客户端 (带 mTLS)
    pub fn network_with_tls(
        base_url: &str,
        ca_cert: &str,
        client_cert: &str,
        client_key: &str,
    ) -> NetworkClient {
        NetworkClient::with_tls(base_url, ca_cert, client_cert, client_key)
    }

    /// 创建同进程客户端 (需要传入 Router)
    #[cfg(feature = "in-process")]
    pub fn in_process(router: axum::Router) -> InProcessClient {
        InProcessClient::new(router)
    }
}

// ============================================================================
// NetworkClient - HTTP 网络客户端
// ============================================================================

/// 网络客户端 (HTTP)
#[derive(Debug, Clone)]
pub struct NetworkClient {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl NetworkClient {
    /// 创建新的网络客户端
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        }
    }

    /// 创建带 mTLS 的网络客户端
    pub fn with_tls(base_url: &str, ca_cert: &str, client_cert: &str, client_key: &str) -> Self {
        // 解析 CA 证书
        let mut ca_reader = std::io::Cursor::new(ca_cert);
        let ca_certs: Vec<rustls::pki_types::CertificateDer> =
            rustls_pemfile::certs(&mut ca_reader)
                .filter_map(|r| r.ok())
                .collect();

        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            let _ = root_store.add(cert);
        }

        // 创建跳过主机名验证的 verifier
        let verifier = std::sync::Arc::new(crab_cert::SkipHostnameVerifier::new(root_store));

        // 解析客户端证书和密钥
        let mut cert_reader = std::io::Cursor::new(client_cert);
        let certs: Vec<rustls::pki_types::CertificateDer> =
            rustls_pemfile::certs(&mut cert_reader)
                .filter_map(|r| r.ok())
                .collect();

        let mut key_reader = std::io::Cursor::new(client_key);
        let key = rustls_pemfile::private_key(&mut key_reader)
            .ok()
            .flatten()
            .expect("Failed to parse client key");

        // 构建 TLS 配置
        let tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(certs, key)
            .expect("Failed to set client auth");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .use_preconfigured_tls(tls_config)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        }
    }

    /// 设置 token
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);

        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }

        let resp = req.send().await?;
        Self::handle_response(resp).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);

        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }

        let resp = req.send().await?;
        Self::handle_response(resp).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url);

        if let Some(auth) = self.auth_header() {
            req = req.header(reqwest::header::AUTHORIZATION, auth);
        }

        let resp = req.send().await?;
        Self::handle_response(resp).await
    }

    async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> ClientResult<T> {
        let status = resp.status();

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return match status {
                StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized),
                StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
                StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
                StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
                _ => Err(ClientError::Internal(text)),
            };
        }

        resp.json().await.map_err(Into::into)
    }
}

#[async_trait]
impl Client for NetworkClient {
    async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        #[derive(serde::Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let resp: ApiResponse<LoginResponse> = self.post("/api/auth/login", &req).await?;
        let login = resp
            .data
            .ok_or_else(|| ClientError::InvalidResponse("Missing login data".into()))?;

        self.token = Some(login.token.clone());
        Ok(login)
    }

    async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let resp: ApiResponse<CurrentUserResponse> = self.get("/api/auth/me").await?;
        resp.data
            .ok_or_else(|| ClientError::InvalidResponse("Missing user data".into()))
    }

    async fn logout(&mut self) -> ClientResult<()> {
        let _: ApiResponse<()> = self.post_empty("/api/auth/logout").await?;
        self.token = None;
        Ok(())
    }

    fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

// ============================================================================
// InProcessClient - 同进程客户端 (tower oneshot)
// ============================================================================

/// 同进程客户端 (直接调用 Router，零网络开销)
#[cfg(feature = "in-process")]
#[derive(Clone)]
pub struct InProcessClient {
    router: axum::Router,
    token: Option<String>,
}

#[cfg(feature = "in-process")]
impl InProcessClient {
    /// 创建同进程客户端
    pub fn new(router: axum::Router) -> Self {
        Self {
            router,
            token: None,
        }
    }

    /// 设置 token
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: http::Method,
        path: &str,
        body: Option<Vec<u8>>,
    ) -> ClientResult<T> {
        use axum::body::Body;
        use tower::ServiceExt;

        let mut builder = http::Request::builder().method(method).uri(path);

        if let Some(token) = &self.token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }

        let req = builder
            .body(Body::from(body.unwrap_or_default()))
            .map_err(|e| ClientError::Internal(e.to_string()))?;

        let resp = self
            .router
            .clone()
            .oneshot(req)
            .await
            .map_err(|e| ClientError::Internal(e.to_string()))?;

        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .map_err(|e| ClientError::Internal(e.to_string()))?;

        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes).to_string();
            return match status {
                http::StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized),
                http::StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
                http::StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
                http::StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
                _ => Err(ClientError::Internal(text)),
            };
        }

        serde_json::from_slice(&bytes).map_err(|e| ClientError::InvalidResponse(e.to_string()))
    }
}

#[cfg(feature = "in-process")]
#[async_trait]
impl Client for InProcessClient {
    async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        #[derive(serde::Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let body = serde_json::to_vec(&req).unwrap();
        let resp: ApiResponse<LoginResponse> = self
            .request(http::Method::POST, "/api/auth/login", Some(body))
            .await?;

        let login = resp
            .data
            .ok_or_else(|| ClientError::InvalidResponse("Missing login data".into()))?;

        self.token = Some(login.token.clone());
        Ok(login)
    }

    async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let resp: ApiResponse<CurrentUserResponse> =
            self.request(http::Method::GET, "/api/auth/me", None).await?;
        resp.data
            .ok_or_else(|| ClientError::InvalidResponse("Missing user data".into()))
    }

    async fn logout(&mut self) -> ClientResult<()> {
        let _: ApiResponse<()> = self
            .request(http::Method::POST, "/api/auth/logout", None)
            .await?;
        self.token = None;
        Ok(())
    }

    fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

// Placeholder for non-feature builds
#[cfg(not(feature = "in-process"))]
pub struct InProcessClient {
    _private: (),
}
