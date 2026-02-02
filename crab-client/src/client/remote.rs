//! Remote mode implementation for CrabClient.
//!
//! This module implements the Remote mode functionality, which uses
//! mTLS certificates to connect to Edge Servers.

use crate::error::{handle_reqwest_response, ClientError, ClientResult};
use crate::types::{Authenticated, Connected, Disconnected, Remote};
use serde::de::DeserializeOwned;
use shared::message::BusMessage;

use super::http::HttpClient;
use std::time::Duration;

use super::common::CrabClient;


// ============================================================================
// Disconnected State
// ============================================================================

impl CrabClient<Remote, Disconnected> {
    /// Performs first-time setup: authenticates with Auth Server and downloads certificates.
    ///
    /// This method:
    /// 1. Logs in to the Auth Server with tenant credentials
    /// 2. Downloads mTLS certificates
    /// 3. Saves certificates locally for future use
    /// 4. Connects to the Edge Server message bus
    ///
    /// After calling this method, you can use `connect_with_credentials()` for subsequent connections.
    ///
    /// # Arguments
    ///
    /// * `tenant_username` - Tenant username for Auth Server
    /// * `tenant_password` - Tenant password for Auth Server
    /// * `message_addr` - Edge Server message bus address (e.g., "192.168.1.100:8081")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crab_client::CrabClient;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// let client = CrabClient::remote()
    ///     .auth_server("https://auth.example.com")
    ///     .cert_path("./certs")
    ///     .client_name("pos-01")
    ///     .build()?;
    ///
    /// let client = client.setup("admin", "password", "edge.local:8081").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn setup(
        mut self,
        tenant_username: &str,
        tenant_password: &str,
        message_addr: &str,
    ) -> Result<CrabClient<Remote, Connected>, ClientError> {
        let cert_manager = self
            .cert_manager
            .as_mut()
            .ok_or_else(|| ClientError::Config("CertManager not configured".into()))?;

        let http = self
            .http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        // 1. Login to Auth Server to get tenant token
        tracing::info!("Logging in to Auth Server...");
        let credential = cert_manager
            .login(http.base_url(), tenant_username, tenant_password)
            .await
            .map_err(|e| ClientError::Auth(e.to_string()))?;

        // 2. Download certificates using tenant token
        tracing::info!("Downloading certificates...");
        let (cert_pem, key_pem, ca_cert_pem) = cert_manager
            .request_certificates(http.base_url(), credential.token(), &credential.tenant_id)
            .await
            .map_err(|e| ClientError::Certificate(e.to_string()))?;

        // 3. Save certificates locally
        cert_manager
            .save_certificates(&cert_pem, &key_pem, &ca_cert_pem)
            .map_err(|e| ClientError::Certificate(e.to_string()))?;

        // Extract client name from certificate (for handshake verification)
        let cert_metadata = crab_cert::CertMetadata::from_pem(&cert_pem)
            .map_err(|e| ClientError::Certificate(format!("Failed to parse certificate: {}", e)))?;
        let handshake_name = cert_metadata
            .client_name
            .or(cert_metadata.common_name)
            .unwrap_or_default();

        // 4. Connect to message server
        tracing::info!("Connecting to message server: {}", message_addr);
        let message_client = crate::client::message::NetworkMessageClient::connect_mtls(
            message_addr,
            ca_cert_pem.as_bytes(),
            cert_pem.as_bytes(),
            key_pem.as_bytes(),
            &handshake_name,
        )
        .await
        .map_err(|e| ClientError::Connection(e.to_string()))?;

        self.message = Some(message_client);

        // 5. Create mTLS HTTP client for Edge Server HTTPS API
        self.edge_http = cert_manager
            .build_mtls_http_client()
            .map_err(|e| {
                tracing::warn!("Failed to build edge HTTP client: {}", e);
                e
            })
            .ok();
        if self.edge_http.is_some() {
            tracing::info!("🔐 mTLS HTTP client created for Edge Server");
        }

        tracing::info!("Setup complete. Certificates cached for future use.");
        Ok(self.transition())
    }

    /// 使用本地缓存的证书和密钥连接到 Edge Server。
    ///
    /// 要求之前已调用 `setup()` 下载并缓存了证书。
    /// 可用 `has_cached_credentials()` 检查是否有缓存凭据。
    ///
    /// 连接前会自动执行自检（证书链验证、硬件绑定、时钟篡改检测），
    /// 并尝试从 Auth Server 刷新时间戳（离线时不阻止连接）。
    ///
    /// # Arguments
    ///
    /// * `message_addr` - Edge Server message bus address (e.g., "192.168.1.100:8081")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crab_client::CrabClient;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// let client = CrabClient::remote()
    ///     .auth_server("https://auth.example.com")
    ///     .cert_path("./certs")
    ///     .client_name("pos-01")
    ///     .build()?;
    ///
    /// if client.has_cached_credentials() {
    ///     let client = client.connect_with_credentials("edge.local:8081").await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_with_credentials(
        mut self,
        message_addr: &str,
    ) -> Result<CrabClient<Remote, Connected>, ClientError> {
        let cert_manager = self
            .cert_manager
            .as_ref()
            .ok_or_else(|| ClientError::Config("CertManager not configured".into()))?;

        // Load cached certificates
        if !cert_manager.has_local_certificates() {
            return Err(ClientError::NoCertificates);
        }

        // 1. 执行完整自检 (证书链、硬件绑定、时钟篡改检测)
        tracing::info!("Running self-check...");
        cert_manager
            .self_check()
            .map_err(|e| ClientError::Certificate(e.to_string()))?;

        // 2. 尝试刷新时间戳 (离线时不阻止连接)
        if let Some(http) = self.http.as_ref()
            && let Err(e) = cert_manager
                .refresh_credential_timestamp(http.base_url())
                .await
        {
            tracing::warn!("Failed to refresh timestamp (offline?): {}", e);
        }

        let (cert_pem, key_pem, ca_cert_pem) = cert_manager
            .load_local_certificates()
            .map_err(|e| ClientError::Certificate(e.to_string()))?;

        // Extract client name from certificate (for handshake verification)
        let cert_metadata = crab_cert::CertMetadata::from_pem(&cert_pem)
            .map_err(|e| ClientError::Certificate(format!("Failed to parse certificate: {}", e)))?;
        let handshake_name = cert_metadata
            .client_name
            .or(cert_metadata.common_name)
            .unwrap_or_default();

        // 3. Connect to message server
        tracing::info!("Connecting to message server with cached credentials: {}", message_addr);
        let message_client = crate::client::message::NetworkMessageClient::connect_mtls(
            message_addr,
            ca_cert_pem.as_bytes(),
            cert_pem.as_bytes(),
            key_pem.as_bytes(),
            &handshake_name,
        )
        .await
        .map_err(|e| ClientError::Connection(e.to_string()))?;

        self.message = Some(message_client);

        // 4. Create mTLS HTTP client for Edge Server HTTPS API
        self.edge_http = cert_manager
            .build_mtls_http_client()
            .map_err(|e| {
                tracing::warn!("Failed to build edge HTTP client: {}", e);
                e
            })
            .ok();
        if self.edge_http.is_some() {
            tracing::info!("🔐 mTLS HTTP client created for Edge Server");
        }

        tracing::info!("Connected using cached certificates.");
        Ok(self.transition())
    }
}

// ============================================================================
// Connected State
// ============================================================================

impl CrabClient<Remote, Connected> {
    /// Sends an RPC request with the default timeout (5 seconds).
    ///
    /// RPC requests only require mTLS connection (no employee login needed).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crab_client::CrabClient;
    /// # use shared::message::BusMessage;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client = CrabClient::remote()
    /// #     .auth_server("https://auth.example.com")
    /// #     .cert_path("./certs")
    /// #     .client_name("pos-01")
    /// #     .build()?
    /// #     .connect_with_credentials("edge:8081").await?;
    /// let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
    ///     action: "ping".to_string(),
    ///     params: None,
    /// });
    /// let response = client.request(&request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request(&self, msg: &BusMessage) -> Result<BusMessage, ClientError> {
        let client = self
            .message
            .as_ref()
            .ok_or_else(|| ClientError::Connection("Not connected".into()))?;

        client.request_default(msg).await
    }

    /// Sends an RPC request with a custom timeout.
    ///
    /// RPC requests only require mTLS connection (no employee login needed).
    pub async fn request_with_timeout(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<BusMessage, ClientError> {
        let client = self
            .message
            .as_ref()
            .ok_or_else(|| ClientError::Connection("Not connected".into()))?;

        client.request(msg, timeout).await
    }

    /// Logs in with employee credentials.
    ///
    /// This authenticates the employee with the Edge Server and obtains
    /// an access token for HTTP API requests that require authentication.
    ///
    /// Note: RPC requests via message bus don't require employee login,
    /// only mTLS connection is needed.
    ///
    /// # Arguments
    ///
    /// * `username` - Employee username
    /// * `password` - Employee password
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crab_client::CrabClient;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client = CrabClient::remote()
    /// #     .auth_server("https://auth.example.com")
    /// #     .cert_path("./certs")
    /// #     .client_name("pos-01")
    /// #     .build()?
    /// #     .setup("tenant", "pass", "edge:8081").await?;
    /// let client = client.login("cashier", "1234").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Returns
    /// - `Ok(Authenticated)` on success
    /// - `Err((error, Connected))` on failure, returning the original client for retry
    pub async fn login(
        mut self,
        username: &str,
        password: &str,
    ) -> Result<CrabClient<Remote, Authenticated>, (ClientError, Self)> {
        // Use mTLS HTTP client to login to Edge Server (port 3000)
        let edge_http = match self.edge_http.take() {
            Some(h) => h,
            None => {
                return Err((
                    ClientError::Connection("mTLS HTTP client not available. Are you connected?".into()),
                    self,
                ))
            }
        };

        tracing::info!("Employee login: {}", username);

        // Build login request
        let login_req = shared::client::LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        // Get Edge Server URL from config
        let edge_url = match self.config.edge_url.clone() {
            Some(url) => url,
            None => {
                self.edge_http = Some(edge_http);
                return Err((ClientError::Config("Edge Server URL not configured".into()), self));
            }
        };

        // Send login request via mTLS
        let response = match edge_http
            .post(format!("{}/api/auth/login", edge_url))
            .json(&login_req)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                self.edge_http = Some(edge_http);
                return Err((ClientError::Connection(format!("Login request failed: {}", e)), self));
            }
        };

        // Parse response
        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            self.edge_http = Some(edge_http);
            return Err((ClientError::Auth(text), self));
        }

        let login_data: shared::client::LoginResponse = match response.json().await {
            Ok(r) => r,
            Err(e) => {
                self.edge_http = Some(edge_http);
                return Err((
                    ClientError::InvalidResponse(format!("Failed to parse login response: {}", e)),
                    self,
                ));
            }
        };

        // Store session data
        self.session
            .set_login(login_data.token.clone(), login_data.user);

        // Restore edge_http for future requests
        self.edge_http = Some(edge_http);

        tracing::info!("Employee logged in successfully.");
        Ok(self.transition())
    }

    /// Disconnects from the server.
    ///
    /// This closes the message connection but preserves cached certificates.
    pub async fn disconnect(mut self) -> CrabClient<Remote, Disconnected> {
        if let Some(client) = &self.message {
            let _ = client.close().await;
        }
        self.message = None;
        self.session.clear();

        tracing::info!("Disconnected from server.");
        self.transition()
    }
}

// ============================================================================
// Authenticated State
// ============================================================================

impl CrabClient<Remote, Authenticated> {
    // ============ Edge Server HTTP API ============

    /// 获取 Edge Server 请求上下文 (http_client, base_url, token)
    fn edge_context(&self) -> ClientResult<(&reqwest::Client, &str, &str)> {
        let http = self
            .edge_http
            .as_ref()
            .ok_or_else(|| ClientError::Connection("mTLS HTTP client not available".into()))?;
        let edge_url = self
            .config
            .edge_url
            .as_deref()
            .ok_or_else(|| ClientError::Config("Edge Server URL not configured".into()))?;
        let token = self
            .session
            .token()
            .ok_or_else(|| ClientError::InvalidState("Not authenticated".into()))?;
        Ok((http, edge_url, token))
    }

    /// GET 请求到 Edge Server
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let (http, edge_url, token) = self.edge_context()?;
        let url = format!("{}{}", edge_url, path);
        let resp = http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        handle_reqwest_response(resp).await
    }

    /// POST 请求到 Edge Server
    pub async fn post<T: DeserializeOwned, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let (http, edge_url, token) = self.edge_context()?;
        let url = format!("{}{}", edge_url, path);
        let resp = http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await?;
        handle_reqwest_response(resp).await
    }

    /// PUT 请求到 Edge Server
    pub async fn put<T: DeserializeOwned, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let (http, edge_url, token) = self.edge_context()?;
        let url = format!("{}{}", edge_url, path);
        let resp = http
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await?;
        handle_reqwest_response(resp).await
    }

    /// DELETE 请求到 Edge Server
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let (http, edge_url, token) = self.edge_context()?;
        let url = format!("{}{}", edge_url, path);
        let resp = http
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        handle_reqwest_response(resp).await
    }

    /// DELETE 请求到 Edge Server (带 body)
    pub async fn delete_with_body<T: DeserializeOwned, B: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let (http, edge_url, token) = self.edge_context()?;
        let url = format!("{}{}", edge_url, path);
        let resp = http
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await?;
        handle_reqwest_response(resp).await
    }

    // ============ Message Bus RPC ============

    /// Sends an RPC request with the default timeout (5 seconds).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crab_client::CrabClient;
    /// # use shared::message::BusMessage;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client: CrabClient<crab_client::Remote, crab_client::Authenticated> = todo!();
    /// let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
    ///     action: "ping".to_string(),
    ///     params: None,
    /// });
    /// let response = client.request(&request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request(&self, msg: &BusMessage) -> Result<BusMessage, ClientError> {
        let client = self
            .message
            .as_ref()
            .ok_or_else(|| ClientError::Connection("Not connected".into()))?;

        client.request_default(msg).await
    }

    /// Sends an RPC request with a custom timeout.
    pub async fn request_with_timeout(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<BusMessage, ClientError> {
        let client = self
            .message
            .as_ref()
            .ok_or_else(|| ClientError::Connection("Not connected".into()))?;

        client.request(msg, timeout).await
    }

    /// Logs out the employee.
    ///
    /// This clears the session token but keeps the connection open.
    /// Use `disconnect()` to fully close the connection.
    pub async fn logout(mut self) -> CrabClient<Remote, Connected> {
        // Clear session
        self.session.clear();

        // Logout from HTTP client
        if let Some(http) = &mut self.http {
            let _ = http.logout().await;
        }

        tracing::info!("Employee logged out.");
        self.transition()
    }

    /// Disconnects from the server.
    ///
    /// This closes the message connection and clears the session.
    pub async fn disconnect(mut self) -> CrabClient<Remote, Disconnected> {
        // Close message connection
        if let Some(client) = &self.message {
            let _ = client.close().await;
        }
        self.message = None;
        self.session.clear();

        // Logout from HTTP client
        if let Some(http) = &mut self.http {
            let _ = http.logout().await;
        }

        tracing::info!("Disconnected from server.");
        self.transition()
    }
}
