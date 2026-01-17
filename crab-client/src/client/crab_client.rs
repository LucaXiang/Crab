// crab-client/src/client/crab_client.rs
// 统一的 CrabClient - 支持远程和本地模式

use std::marker::PhantomData;
use crate::{ClientResult, LoginResponse, CurrentUserResponse, ClientError, HttpClient};

/// 远程模式标记
pub struct RemoteMode;

/// 本地模式标记
pub struct LocalMode;

/// 统一的 CrabClient - 集成证书管理和消息连接
#[derive(Debug, Clone)]
pub struct CrabClient<M> {
    _mode: PhantomData<M>,
    http: Option<crate::client::NetworkHttpClient>,
    message: Option<crate::client::NetworkMessageClient>,
    http_token: Option<String>,  // 员工 HTTP 请求的 token
    auth_token: Option<String>,  // Auth Server 租户认证的 token
    cert_manager: Option<crate::CertManager>,
}

impl<M> CrabClient<M> {
    /// 获取员工 token
    pub fn token(&self) -> Option<&str> {
        self.http_token.as_deref()
    }

    /// 检查是否已连接消息服务器
    pub fn is_connected(&self) -> bool {
        self.message.as_ref().map(|m| m.is_connected()).unwrap_or(false)
    }
}

impl CrabClient<RemoteMode> {
    /// 创建远程客户端
    pub fn new(auth_url: &str, cert_path: &str, client_name: &str) -> Self {
        let cert_manager = crate::CertManager::new(cert_path, client_name);
        let http = crate::client::NetworkHttpClient::new(auth_url)
            .expect("Failed to create HTTP client");
        Self {
            _mode: PhantomData,
            http: Some(http),
            message: None,
            http_token: None,
            auth_token: None,
            cert_manager: Some(cert_manager),
        }
    }

    /// 租户登录 - 获取证书并连接消息服务器
    ///
    /// 如果已有缓存的凭据和证书，直接连接无需重新登录
    /// 否则需要先调用 setup() 进行首次设置
    pub async fn connect(&mut self, message_addr: &str) -> Result<(), crate::MessageError> {
        let cert_manager = self.cert_manager.as_mut()
            .expect("CertManager not configured");

        // 检查是否有缓存的证书
        if cert_manager.has_local_certificates() {
            tracing::info!("Using cached certificates");
        } else {
            return Err(crate::MessageError::Connection(
                "No cached certificates. Please call setup() first.".to_string()
            ));
        }

        // 加载证书
        let (cert_pem, key_pem, ca_cert_pem) = cert_manager
            .load_local_certificates()
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;

        // 连接消息服务器
        let client = crate::client::NetworkMessageClient::connect_mtls(
            message_addr,
            ca_cert_pem.as_bytes(),
            cert_pem.as_bytes(),
            key_pem.as_bytes(),
            cert_manager.client_name(),
        ).await?;
        self.message = Some(client);

        tracing::info!("Connected to message server: {}", message_addr);
        Ok(())
    }

    /// 首次设置 - 登录租户并下载证书
    ///
    /// 使用租户凭据从 Auth Server 获取 token 和证书
    /// 之后可使用 connect() 直接连接
    pub async fn setup(
        &mut self,
        tenant_username: &str,
        tenant_password: &str,
        message_addr: &str,
    ) -> Result<(), crate::MessageError> {
        let cert_manager = self.cert_manager.as_mut()
            .expect("CertManager not configured");

        // 1. 租户登录获取 Auth Server token
        let credential = cert_manager.login(
            self.http.as_ref().unwrap().base_url(),
            tenant_username,
            tenant_password,
        ).await
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;

        // 保存 Auth Server token（用于后续重新下载证书）
        self.auth_token = Some(credential.token.clone());

        // 2. 使用 token 下载证书
        let (cert_pem, key_pem, ca_cert_pem) = cert_manager
            .request_certificates(
                self.http.as_ref().unwrap().base_url(),
                &credential.token,
                &credential.tenant_id,
            )
            .await
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;

        // 3. 保存证书
        cert_manager.save_certificates(&cert_pem, &key_pem, &ca_cert_pem)
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;

        // 4. 连接消息服务器
        let client = crate::client::NetworkMessageClient::connect_mtls(
            message_addr,
            ca_cert_pem.as_bytes(),
            cert_pem.as_bytes(),
            key_pem.as_bytes(),
            cert_manager.client_name(),
        ).await?;
        self.message = Some(client);

        tracing::info!("Setup complete. Certificates cached.");
        Ok(())
    }

    /// 员工登录 - 用于 HTTP API 请求
    ///
    /// 如果有缓存的员工 token，直接使用；否则登录获取并缓存
    pub async fn login(&mut self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        // 检查是否有缓存的员工 token
        if let Some(cm) = &self.cert_manager {
            if let Ok(cred) = cm.load_credential() {
                if let Some(token) = cred.employee_token() {
                    tracing::info!("Using cached employee token");
                    self.http_token = Some(token.to_string());
                    return Ok(LoginResponse {
                        token: token.to_string(),
                        user: shared::client::UserInfo {
                            id: "cached".to_string(),
                            username: "cached".to_string(),
                            role: "cached".to_string(),
                            permissions: vec![],
                        },
                    });
                }
            }
        }

        // 登录获取员工 token
        let http = self.http.as_mut().expect("HTTP client not initialized");
        let resp = http.login(username, password).await?;

        // 缓存员工 token
        self.http_token = Some(resp.token.clone());
        if let Some(cm) = &self.cert_manager {
            if let Ok(mut cred) = cm.load_credential() {
                cred.set_employee_token(resp.token.clone());
                let _ = cm.save_credential(&cred);
            }
        }

        Ok(resp)
    }

    /// 获取当前用户
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        let http = self.http.as_ref().expect("HTTP client not initialized");
        http.me().await
    }

    /// 登出 - 只清理员工 token，保留证书缓存
    pub async fn logout(&mut self) {
        // 关闭消息连接
        if let Some(client) = &self.message {
            let _ = client.close().await;
        }
        self.message.take();

        // 清理 HTTP token
        if let Some(http) = &mut self.http {
            let _ = http.logout().await;
        }
        self.http_token.take();

        // 清除缓存的员工 token（保留租户凭据和证书）
        if let Some(cm) = &self.cert_manager {
            if let Ok(mut cred) = cm.load_credential() {
                cred.clear_employee_token();
                let _ = cm.save_credential(&cred);
            }
        }
        tracing::info!("Logged out (credentials cached for reuse)");
    }

    /// 发送 RPC 请求 (使用默认超时 5 秒)
    pub async fn request(&self, msg: &crate::BusMessage) -> crate::MessageResult<crate::BusMessage> {
        let client = self.message.as_ref().expect("Message client not connected");
        client.request_default(msg).await
    }

    /// 发送 RPC 请求 (自定义超时)
    pub async fn request_with_timeout(
        &self,
        msg: &crate::BusMessage,
        timeout: std::time::Duration,
    ) -> crate::MessageResult<crate::BusMessage> {
        let client = self.message.as_ref().expect("Message client not connected");
        client.request(msg, timeout).await
    }
}

impl Default for CrabClient<LocalMode> {
    fn default() -> Self {
        Self::new()
    }
}

impl CrabClient<LocalMode> {
    /// 创建本地客户端
    pub fn new() -> Self {
        Self {
            _mode: PhantomData,
            http: None,
            message: None,
            http_token: None,
            auth_token: None,
            cert_manager: None,
        }
    }

    /// LocalMode 不支持远程登录
    pub async fn login(&mut self, _username: &str, _password: &str) -> ClientResult<LoginResponse> {
        Err(ClientError::NotFound("LocalMode does not support login".into()))
    }

    /// LocalMode 不支持远程用户查询
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        Err(ClientError::NotFound("LocalMode does not support /me endpoint".into()))
    }

    /// LocalMode 不支持消息连接
    pub async fn connect(&mut self, _addr: &str) -> Result<(), crate::MessageError> {
        Err(crate::MessageError::Connection("LocalMode does not support message connection".into()))
    }

    /// LocalMode 不支持 setup
    pub async fn setup(
        &mut self,
        _username: &str,
        _password: &str,
        _addr: &str,
    ) -> Result<(), crate::MessageError> {
        Err(crate::MessageError::Connection("LocalMode does not support setup".into()))
    }

    /// 登出 - 只清理员工 token，保留证书缓存
    pub async fn logout(&mut self) {
        // 清理 HTTP token
        if let Some(http) = &mut self.http {
            let _ = http.logout().await;
        }
        self.http_token.take();
        tracing::info!("Logged out (LocalMode)");
    }
}
