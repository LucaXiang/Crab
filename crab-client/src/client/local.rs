//! Local mode implementation for CrabClient.
//!
//! This module implements the Local mode functionality, which uses
//! Tower oneshot for HTTP calls and broadcast channels for message bus.
//! Zero network overhead - all communication happens in-process.
//!
//! Requires the "in-process" feature.

#![cfg(feature = "in-process")]

use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};
use shared::message::BusMessage;
use tokio::sync::broadcast;

use crate::error::{ClientError, ClientResult};
use crate::types::{Authenticated, ClientState, Connected, Disconnected, Local};

use super::common::CrabClient;
use super::http::HttpClient;

// ============================================================================
// Common Methods for All States
// ============================================================================

impl<S: ClientState> CrabClient<Local, S> {
    /// 订阅服务器广播消息
    ///
    /// 返回一个 Receiver，可以接收服务器发送的所有消息：
    /// - Notification (通知)
    /// - Response (响应)
    /// - Sync (同步信号)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut rx = client.subscribe()?;
    /// tokio::spawn(async move {
    ///     while let Ok(msg) = rx.recv().await {
    ///         match msg.event_type {
    ///             EventType::Notification => println!("Got notification"),
    ///             EventType::Sync => println!("Sync signal"),
    ///             _ => {}
    ///         }
    ///     }
    /// });
    /// ```
    pub fn subscribe(&self) -> ClientResult<broadcast::Receiver<BusMessage>> {
        let message_client = self
            .memory_message
            .as_ref()
            .ok_or_else(|| ClientError::Config("Message client not configured".into()))?;

        Ok(message_client.subscribe())
    }
}

// ============================================================================
// Disconnected State
// ============================================================================

impl CrabClient<Local, Disconnected> {
    /// Connects to the local server (in-process).
    ///
    /// For Local mode, this simply transitions to Connected state since
    /// the router and message sender are already configured.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use crab_client::CrabClient;
    /// # use axum::Router;
    /// # use tokio::sync::broadcast;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// let router: Router = build_app().with_state(state);
    /// let (sender, _) = broadcast::channel(1024);
    ///
    /// let client = CrabClient::local()
    ///     .with_router(router)
    ///     .with_message_sender(sender)
    ///     .build()?;
    ///
    /// let client = client.connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(self) -> Result<CrabClient<Local, Connected>, ClientError> {
        // Verify that oneshot_http and memory_message are configured
        if self.oneshot_http.is_none() {
            return Err(ClientError::Config("Router not configured".into()));
        }
        if self.memory_message.is_none() {
            return Err(ClientError::Config("Message sender not configured".into()));
        }

        tracing::info!("Connected to local server (in-process)");
        Ok(self.transition())
    }
}

// ============================================================================
// Connected State
// ============================================================================

impl CrabClient<Local, Connected> {
    /// Logs in with employee credentials.
    ///
    /// This authenticates the employee with the local Edge Server and obtains
    /// an access token for HTTP API requests.
    ///
    /// # Arguments
    ///
    /// * `username` - Employee username
    /// * `password` - Employee password
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use crab_client::CrabClient;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client: CrabClient<crab_client::Local, crab_client::Connected> = todo!();
    /// let client = client.login("waiter", "1234").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn login(
        mut self,
        username: &str,
        password: &str,
    ) -> Result<CrabClient<Local, Authenticated>, ClientError> {
        let http = self
            .oneshot_http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        tracing::info!("Employee login (local): {}", username);
        let response = http.login(username, password).await?;

        // Store session data
        self.session.set_login(response.token.clone(), response.user);

        // Set token in oneshot client for subsequent requests
        if let Some(ref http) = self.oneshot_http {
            http.set_token(Some(response.token)).await;
        }

        tracing::info!("Employee logged in successfully (local).");
        Ok(self.transition())
    }

    /// Disconnects from the server.
    pub fn disconnect(mut self) -> CrabClient<Local, Disconnected> {
        self.session.clear();
        tracing::info!("Disconnected from local server.");
        self.transition()
    }

    /// Restores an authenticated session from cached token.
    ///
    /// This allows restoring a previous login session without re-authenticating
    /// with the server. Use this when the app restarts and has a cached token.
    ///
    /// # Arguments
    ///
    /// * `token` - The cached JWT token
    /// * `user` - The cached user information
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use crab_client::CrabClient;
    /// # use shared::client::UserInfo;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client: CrabClient<crab_client::Local, crab_client::Connected> = todo!();
    /// let token = "cached_jwt_token".to_string();
    /// let user = UserInfo { /* ... */ };
    /// let client = client.restore_session(token, user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn restore_session(
        mut self,
        token: String,
        user: shared::client::UserInfo,
    ) -> Result<CrabClient<Local, Authenticated>, ClientError> {
        // Set token in oneshot client for subsequent requests
        if let Some(ref http) = self.oneshot_http {
            http.set_token(Some(token.clone())).await;
        }

        // Store session data
        self.session.set_login(token, user.clone());

        tracing::info!(username = %user.username, "Session restored from cache (local)");
        Ok(self.transition())
    }
}

// ============================================================================
// Authenticated State
// ============================================================================

impl CrabClient<Local, Authenticated> {
    /// Sends a GET request to the specified path.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use crab_client::CrabClient;
    /// # use serde::Deserialize;
    /// # #[derive(Deserialize)]
    /// # struct Order { id: String }
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client: CrabClient<crab_client::Local, crab_client::Authenticated> = todo!();
    /// let orders: Vec<Order> = client.get("/api/orders").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let http = self
            .oneshot_http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        http.get(path).await
    }

    /// Sends a POST request to the specified path with a JSON body.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use crab_client::CrabClient;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Serialize)]
    /// # struct CreateOrder { items: Vec<String> }
    /// # #[derive(Deserialize)]
    /// # struct Order { id: String }
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client: CrabClient<crab_client::Local, crab_client::Authenticated> = todo!();
    /// let order: Order = client.post("/api/orders", &CreateOrder { items: vec![] }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn post<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let http = self
            .oneshot_http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        http.post(path, body).await
    }

    /// Sends a PUT request to the specified path with a JSON body.
    pub async fn put<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let http = self
            .oneshot_http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        http.put(path, body).await
    }

    /// Sends a DELETE request to the specified path.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let http = self
            .oneshot_http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        http.delete(path).await
    }

    /// Sends a DELETE request with a JSON body.
    pub async fn delete_with_body<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let http = self
            .oneshot_http
            .as_ref()
            .ok_or_else(|| ClientError::Config("HTTP client not configured".into()))?;

        http.delete_with_body(path, body).await
    }

    /// Sends an RPC request via the message bus and waits for a response.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use crab_client::CrabClient;
    /// # use shared::message::BusMessage;
    /// # async fn example() -> Result<(), crab_client::ClientError> {
    /// # let client: CrabClient<crab_client::Local, crab_client::Authenticated> = todo!();
    /// let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
    ///     action: "ping".to_string(),
    ///     params: None,
    /// });
    /// let response = client.request(&request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request(&self, msg: &BusMessage) -> ClientResult<BusMessage> {
        let message_client = self
            .memory_message
            .as_ref()
            .ok_or_else(|| ClientError::Config("Message client not configured".into()))?;

        message_client
            .request_default(msg)
            .await
            .map_err(ClientError::Message)
    }

    /// Sends an RPC request with a custom timeout.
    pub async fn request_with_timeout(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> ClientResult<BusMessage> {
        let message_client = self
            .memory_message
            .as_ref()
            .ok_or_else(|| ClientError::Config("Message client not configured".into()))?;

        message_client
            .request(msg, timeout)
            .await
            .map_err(ClientError::Message)
    }

    /// Logs out the employee.
    ///
    /// This clears the session token.
    pub async fn logout(mut self) -> CrabClient<Local, Connected> {
        // Clear token in oneshot client
        if let Some(ref http) = self.oneshot_http {
            http.set_token(None).await;
        }
        self.session.clear();
        tracing::info!("Employee logged out (local).");
        self.transition()
    }

    /// Disconnects from the server.
    pub async fn disconnect(mut self) -> CrabClient<Local, Disconnected> {
        // Clear token in oneshot client
        if let Some(ref http) = self.oneshot_http {
            http.set_token(None).await;
        }
        self.session.clear();
        tracing::info!("Disconnected from local server.");
        self.transition()
    }
}
