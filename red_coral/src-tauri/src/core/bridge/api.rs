//! HTTP proxy and configuration access

use super::*;

impl ClientBridge {
    /// 获取租户管理器的只读引用
    pub fn tenant_manager(&self) -> &Arc<RwLock<TenantManager>> {
        &self.tenant_manager
    }

    /// 获取 Auth Server URL
    pub async fn get_auth_url(&self) -> String {
        self.config.read().await.auth_url.clone()
    }

    /// 使用 refresh_token 换取新的 access token
    ///
    /// 自动更新 config 中的 refresh_token（轮转机制）
    pub async fn get_fresh_token(&self) -> Result<String, BridgeError> {
        let (auth_url, refresh_token) = {
            let config = self.config.read().await;
            let rt = config.refresh_token.clone().ok_or_else(|| {
                BridgeError::Config("No refresh token stored — re-login required".to_string())
            })?;
            (config.auth_url.clone(), rt)
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| BridgeError::Config(e.to_string()))?;

        let resp = client
            .post(format!("{auth_url}/api/tenant/refresh"))
            .json(&shared::activation::TokenRefreshRequest { refresh_token })
            .send()
            .await
            .map_err(|e| BridgeError::Config(format!("Token refresh network error: {e}")))?;

        let data: shared::activation::TokenRefreshResponse = resp
            .json()
            .await
            .map_err(|e| BridgeError::Config(format!("Token refresh parse error: {e}")))?;

        if !data.success {
            return Err(BridgeError::Config(
                data.error
                    .unwrap_or_else(|| "Token refresh failed".to_string()),
            ));
        }

        let token = data
            .token
            .ok_or_else(|| BridgeError::Config("No token in refresh response".to_string()))?;

        // 保存新的 refresh_token（轮转后旧的失效）
        if let Some(new_rt) = data.refresh_token {
            let mut config = self.config.write().await;
            config.refresh_token = Some(new_rt);
            config.save(&self.config_path)?;
        }

        Ok(token)
    }

    /// 获取服务器模式配置
    pub async fn get_server_config(&self) -> ServerModeConfig {
        self.config.read().await.server_config.clone()
    }

    /// 获取客户端模式配置
    pub async fn get_client_config(&self) -> Option<ClientModeConfig> {
        self.config.read().await.client_config.clone()
    }

    /// 更新 Server 模式配置 (端口配置)
    ///
    /// 仅更新配置并保存，不启动模式
    pub async fn update_server_config(
        &self,
        http_port: u16,
        message_port: u16,
    ) -> Result<(), BridgeError> {
        {
            let mut config = self.config.write().await;
            config.server_config.http_port = http_port;
            config.server_config.message_port = message_port;
            config.save(&self.config_path)?;
        }
        tracing::info!(http_port = %http_port, message_port = %message_port, "Server config updated");
        Ok(())
    }

    /// 更新 Client 模式配置 (连接地址)
    ///
    /// 仅更新配置并保存，不启动模式
    pub async fn update_client_config(
        &self,
        edge_url: &str,
        message_addr: &str,
    ) -> Result<(), BridgeError> {
        {
            let mut config = self.config.write().await;
            config.client_config = Some(ClientModeConfig {
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
            });
            config.save(&self.config_path)?;
        }
        tracing::info!(edge_url = %edge_url, message_addr = %message_addr, "Client config updated");
        Ok(())
    }

    /// 获取 Client 模式的 mTLS HTTP client 和相关信息
    ///
    /// 返回 (edge_url, http_client, token) 用于需要直接访问 EdgeServer 的场景 (如图片上传)
    /// Server 模式或未认证时返回 None
    pub async fn get_edge_http_context(&self) -> Option<(String, reqwest::Client, String)> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Client {
                client: Some(RemoteClientState::Authenticated(auth)),
                edge_url,
                ..
            } => {
                let http = auth.edge_http_client()?.clone();
                let token = auth.token()?.to_string();
                Some((edge_url.clone(), http, token))
            }
            _ => None,
        }
    }

    /// 通用 GET 请求 (使用 CrabClient)
    pub async fn get<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.get(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.get(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 POST 请求 (使用 CrabClient)
    pub async fn post<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.post(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.post(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 PUT 请求 (使用 CrabClient)
    pub async fn put<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.put(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.put(path, body).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 DELETE 请求 (使用 CrabClient)
    pub async fn delete<T>(&self, path: &str) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => {
                    auth.delete(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    auth.delete(path).await.map_err(BridgeError::Client)
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// 通用 DELETE 请求 (带 body)
    pub async fn delete_with_body<T, B>(&self, path: &str, body: &B) -> Result<T, BridgeError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + Sync,
    {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { client, .. } => match client {
                Some(LocalClientState::Authenticated(auth)) => auth
                    .delete_with_body(path, body)
                    .await
                    .map_err(BridgeError::Client),
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => auth
                    .delete_with_body(path, body)
                    .await
                    .map_err(BridgeError::Client),
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }
}
