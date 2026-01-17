// crab-client/src/client/message.rs
// RPC 消息客户端 - mTLS 和内存通信

use rustls_pki_types::{CertificateDer, ServerName};
use shared::message::{BusMessage, HandshakePayload, PROTOCOL_VERSION};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_rustls::client::TlsStream;
use uuid::Uuid;

/// mTLS TCP 消息客户端
///
/// 通过 mTLS 连接到 Edge Server 的消息总线：
/// 1. TCP 连接到服务器
/// 2. TLS 握手 (使用客户端证书)
/// 3. 协议握手 (Handshake)
/// 4. 消息收发
///
/// 架构:
/// - 后台任务持续读取消息
/// - RPC 响应通过 pending_requests 路由给等待者
/// - 非响应消息广播给所有订阅者
#[derive(Debug, Clone)]
pub struct NetworkMessageClient {
    /// 写入流 (发送请求)
    write_stream: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>,
    connected: Arc<AtomicBool>,
    /// 非响应消息广播通道 (通知、同步信号等)
    notification_tx: broadcast::Sender<BusMessage>,
    /// 等待响应的 RPC 请求 (correlation_id -> response sender)
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>>,
}

impl NetworkMessageClient {
    /// 使用 mTLS 连接到 Edge Server
    ///
    /// # Arguments
    /// * `addr` - 服务器地址，如 "127.0.0.1:8082"
    /// * `ca_cert_pem` - CA 证书 (PEM 格式，用于验证服务器)
    /// * `client_cert_pem` - 客户端证书 (PEM 格式)
    /// * `client_key_pem` - 客户端私钥 (PEM 格式，支持 PKCS8/PKCS1/SEC1)
    /// * `client_name` - 客户端名称 (用于身份验证)
    ///
    /// # Returns
    /// 连接的客户端
    pub async fn connect_mtls(
        addr: &str,
        ca_cert_pem: &[u8],
        client_cert_pem: &[u8],
        client_key_pem: &[u8],
        client_name: &str,
    ) -> Result<Self, crate::MessageError> {
        use tokio::io::split;
        use tokio_rustls::TlsConnector;

        // 0. 安装 aws-lc-rs CryptoProvider (FIPS 140-3)
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        // 1. 解析 CA 证书链 (支持多个证书)
        let mut ca_reader = std::io::Cursor::new(ca_cert_pem);
        let ca_certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut ca_reader)
            .collect::<Result<_, _>>()
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to parse CA certs: {}", e))
            })?;
        if ca_certs.is_empty() {
            return Err(crate::MessageError::Connection(
                "No CA certificates found".to_string(),
            ));
        }

        // 2. 解析客户端证书
        let mut cert_reader = std::io::Cursor::new(client_cert_pem);
        let client_certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<_, _>>()
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to parse client cert: {}", e))
            })?;
        let client_cert = client_certs.into_iter().next().ok_or_else(|| {
            crate::MessageError::Connection("No client certificate found".to_string())
        })?;

        // 3. 解析私钥 (支持 PKCS8, PKCS1, SEC1 等格式)
        let mut key_reader = std::io::Cursor::new(client_key_pem);
        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to parse private key: {}", e))
            })?
            .ok_or_else(|| crate::MessageError::Connection("No private key found".to_string()))?;

        // 4. 构建 TLS 客户端配置，使用 crab_cert 的 SkipHostnameVerifier
        let mut root_store = rustls::RootCertStore::empty();
        for ca_cert in ca_certs {
            root_store.add(ca_cert).map_err(|e| {
                crate::MessageError::Connection(format!("Failed to add CA cert: {}", e))
            })?;
        }

        // 使用 crab_cert 的 SkipHostnameVerifier (跳过 hostname 验证)
        let verifier = crab_cert::SkipHostnameVerifier::new(root_store);

        let client_cert_chain = vec![client_cert];
        let tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_client_auth_cert(client_cert_chain, private_key)
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to configure TLS: {}", e))
            })?;

        // 5. 建立 TCP 连接
        let tcp_stream = TcpStream::connect(addr)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("TCP connect failed: {}", e)))?;

        // 6. TLS 握手
        let connector = TlsConnector::from(Arc::new(tls_config));
        let domain = ServerName::try_from("edge-server")
            .map_err(|e| crate::MessageError::Connection(format!("Invalid domain: {}", e)))?;

        let tls_stream = connector
            .connect(domain, tcp_stream)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("TLS handshake failed: {}", e)))?;

        // 7. 分离读写流
        let (read_half, write_half) = split(tls_stream);

        // 8. 创建通道
        let (notification_tx, _) = broadcast::channel(64);
        let pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(AtomicBool::new(true));

        // 9. 启动后台读取任务
        let reader_pending = pending_requests.clone();
        let reader_notify_tx = notification_tx.clone();
        let reader_connected = connected.clone();
        tokio::spawn(async move {
            Self::reader_task(read_half, reader_pending, reader_notify_tx, reader_connected).await;
        });

        let client = Self {
            write_stream: Arc::new(Mutex::new(write_half)),
            connected,
            notification_tx,
            pending_requests,
        };

        // 10. 协议握手
        tracing::info!("Starting protocol handshake...");
        client.perform_handshake(client_name).await?;
        tracing::info!("Protocol handshake completed!");

        Ok(client)
    }

    /// 后台读取任务
    async fn reader_task(
        mut read_half: ReadHalf<TlsStream<TcpStream>>,
        pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>>,
        notification_tx: broadcast::Sender<BusMessage>,
        connected: Arc<AtomicBool>,
    ) {
        loop {
            if !connected.load(Ordering::SeqCst) {
                tracing::debug!("Reader task: connection closed");
                break;
            }

            match Self::read_message_from(&mut read_half).await {
                Ok(msg) => {
                    // 检查是否是 RPC 响应
                    if let Some(correlation_id) = msg.correlation_id {
                        let mut pending = pending_requests.lock().await;
                        if let Some(tx) = pending.remove(&correlation_id) {
                            // 发送给等待的 RPC 调用者
                            let _ = tx.send(msg);
                            continue;
                        }
                    }
                    // 非响应消息，广播给订阅者
                    let _ = notification_tx.send(msg);
                }
                Err(e) => {
                    tracing::debug!("Reader task error: {}", e);
                    connected.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
        tracing::debug!("Reader task exited");
    }

    /// 从读取流读取一条消息
    async fn read_message_from(
        stream: &mut ReadHalf<TlsStream<TcpStream>>,
    ) -> Result<BusMessage, crate::MessageError> {
        // 读取事件类型 (1 字节)
        let type_buf = &mut [0u8; 1];
        stream
            .read_exact(type_buf)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Read type failed: {}", e)))?;

        let event_type = shared::EventType::try_from(type_buf[0])
            .map_err(|_| crate::MessageError::InvalidMessage("Invalid event type".to_string()))?;

        // 读取 Request ID (16 字节)
        let uuid_buf = &mut [0u8; 16];
        stream
            .read_exact(uuid_buf)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Read UUID failed: {}", e)))?;
        let request_id = Uuid::from_bytes(*uuid_buf);

        // 读取 Correlation ID (16 字节)
        let correlation_buf = &mut [0u8; 16];
        stream.read_exact(correlation_buf).await.map_err(|e| {
            crate::MessageError::Connection(format!("Read Correlation UUID failed: {}", e))
        })?;
        let correlation_id_raw = Uuid::from_bytes(*correlation_buf);
        let correlation_id = if correlation_id_raw.is_nil() {
            None
        } else {
            Some(correlation_id_raw)
        };

        // 读取载荷长度 (4 字节)
        let len_buf = &mut [0u8; 4];
        stream
            .read_exact(len_buf)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Read len failed: {}", e)))?;

        let len = u32::from_le_bytes(*len_buf) as usize;

        // 读取载荷内容
        let mut payload = vec![0u8; len];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Read payload failed: {}", e)))?;

        Ok(BusMessage {
            request_id,
            event_type,
            source: None,
            correlation_id,
            target: None,
            payload,
        })
    }

    /// 执行协议握手
    async fn perform_handshake(&self, client_name: &str) -> Result<(), crate::MessageError> {
        let handshake = BusMessage::handshake(&HandshakePayload {
            version: PROTOCOL_VERSION,
            client_name: Some(client_name.to_string()),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            client_id: Some(Uuid::new_v4().to_string()),
        });

        // 发送握手消息
        tracing::debug!("Sending handshake message...");
        self.write_message(&handshake).await?;
        tracing::debug!("Handshake message sent, waiting for response...");

        // 等待服务器响应 (使用 RPC 机制)
        let response = self
            .request_internal(&handshake, Duration::from_secs(10))
            .await?;

        // 检查是否是响应消息
        if response.event_type == shared::message::EventType::Response {
            if let Ok(payload) = response.parse_payload::<shared::message::ResponsePayload>() {
                if !payload.success {
                    return Err(crate::MessageError::Connection(format!(
                        "Handshake failed: {}",
                        payload.message
                    )));
                }
                tracing::debug!("Handshake successful: {}", payload.message);
            }
        }

        Ok(())
    }

    /// 写入消息
    async fn write_message(&self, msg: &BusMessage) -> Result<(), crate::MessageError> {
        let mut stream = self.write_stream.lock().await;

        // 序列化消息
        let mut data = Vec::new();
        data.push(msg.event_type as u8);
        data.extend_from_slice(msg.request_id.as_bytes());

        // Correlation ID (16 bytes, nil if None)
        let correlation_bytes = msg
            .correlation_id
            .unwrap_or(Uuid::nil())
            .as_bytes()
            .to_vec();
        data.extend_from_slice(&correlation_bytes);

        // Payload length (4 bytes) + payload
        let payload_len = msg.payload.len() as u32;
        data.extend_from_slice(&payload_len.to_le_bytes());
        data.extend_from_slice(&msg.payload);

        stream
            .write_all(&data)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Write failed: {}", e)))?;
        stream
            .flush()
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// 关闭连接
    pub async fn close(&self) -> Result<(), crate::MessageError> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// 订阅非响应消息 (通知、同步信号等)
    ///
    /// 返回一个 broadcast receiver，调用者可以在后台任务中循环接收消息。
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut rx = client.subscribe();
    /// tokio::spawn(async move {
    ///     while let Ok(msg) = rx.recv().await {
    ///         match msg.event_type {
    ///             EventType::Notification => { /* handle */ }
    ///             EventType::Sync => { /* handle */ }
    ///             _ => {}
    ///         }
    ///     }
    /// });
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.notification_tx.subscribe()
    }

    /// 内部 RPC 请求 (用于握手等内部操作)
    async fn request_internal(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<BusMessage, crate::MessageError> {
        let correlation_id = msg.request_id;

        // 创建响应通道
        let (tx, rx) = oneshot::channel();

        // 注册到 pending_requests
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(correlation_id, tx);
        }

        // 等待响应 (消息已由调用者发送)
        let result = tokio::time::timeout(timeout, rx).await;

        // 清理
        {
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&correlation_id);
        }

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(crate::MessageError::Connection(
                "Response channel closed".to_string(),
            )),
            Err(_) => Err(crate::MessageError::Timeout(format!(
                "Request timed out after {:?}",
                timeout
            ))),
        }
    }

    /// 发送请求并等待响应（带超时）
    pub async fn request(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<BusMessage, crate::MessageError> {
        if !self.is_connected() {
            return Err(crate::MessageError::Connection("Not connected".to_string()));
        }

        let correlation_id = msg.request_id;

        // 创建响应通道
        let (tx, rx) = oneshot::channel();

        // 注册到 pending_requests
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(correlation_id, tx);
        }

        // 发送请求
        if let Err(e) = self.write_message(msg).await {
            // 清理
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&correlation_id);
            return Err(e);
        }

        // 等待响应
        let result = tokio::time::timeout(timeout, rx).await;

        // 清理
        {
            let mut pending = self.pending_requests.lock().await;
            pending.remove(&correlation_id);
        }

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(crate::MessageError::Connection(
                "Response channel closed".to_string(),
            )),
            Err(_) => Err(crate::MessageError::Timeout(format!(
                "Request timed out after {:?}",
                timeout
            ))),
        }
    }

    /// 发送请求并等待响应（使用默认超时）
    pub async fn request_default(&self, msg: &BusMessage) -> Result<BusMessage, crate::MessageError> {
        let timeout = crate::MessageClientConfig::default().request_timeout;
        self.request(msg, timeout).await
    }
}

/// 内存消息客户端 (同进程通信)
///
/// 使用双向 broadcast 通道实现，适用于同进程的服务器-客户端通信。
///
/// 通道说明:
/// - `client_tx`: 客户端 → 服务器 (发送请求)
/// - `server_tx`: 服务器 → 客户端 (接收广播/响应)
///
/// # Example
///
/// ```ignore
/// use tokio::sync::broadcast;
/// use crab_client::InMemoryMessageClient;
///
/// // 创建双向通道
/// let (client_tx, _) = broadcast::channel(16);
/// let (server_tx, _) = broadcast::channel(16);
///
/// // 创建客户端
/// let client = InMemoryMessageClient::new(client_tx.clone(), server_tx.clone());
///
/// // 服务器端接收请求
/// let mut server_rx = client_tx.subscribe();
/// // 客户端接收响应
/// let mut client_rx = server_tx.subscribe();
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryMessageClient {
    /// 客户端 → 服务器
    client_tx: broadcast::Sender<BusMessage>,
    /// 服务器 → 客户端
    server_tx: broadcast::Sender<BusMessage>,
}

impl InMemoryMessageClient {
    /// 创建内存消息客户端
    pub fn new(
        client_tx: broadcast::Sender<BusMessage>,
        server_tx: broadcast::Sender<BusMessage>,
    ) -> Self {
        Self { client_tx, server_tx }
    }

    /// 创建内存消息客户端 (只需服务器 → 客户端通道)
    ///
    /// 适用于只需要接收广播的场景
    #[allow(dead_code)]
    pub fn new_receiver(server_tx: broadcast::Sender<BusMessage>) -> Self {
        let (client_tx, _) = broadcast::channel(16);
        Self { client_tx, server_tx }
    }

    /// 检查是否已连接 (内存客户端始终连接)
    pub fn is_connected(&self) -> bool {
        true
    }

    /// 发送请求并等待响应
    pub async fn request(
        &self,
        msg: &BusMessage,
        timeout: Duration,
    ) -> Result<BusMessage, crate::MessageError> {
        let correlation_id = msg.request_id;

        // 订阅响应通道
        let mut rx = self.server_tx.subscribe();

        // 发送请求
        self.client_tx
            .send(msg.clone())
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;

        // 等待响应
        let response = tokio::time::timeout(timeout, async {
            loop {
                let received = rx.recv().await.map_err(|e: broadcast::error::RecvError| {
                    crate::MessageError::Connection(e.to_string())
                })?;
                if received.correlation_id == Some(correlation_id) {
                    return Ok::<BusMessage, crate::MessageError>(received);
                }
            }
        })
        .await
        .map_err(|_| {
            crate::MessageError::Timeout(format!("Request timed out after {:?}", timeout))
        })??;

        Ok(response)
    }

    /// 发送消息 (不等待响应)
    pub fn send(&self, msg: &BusMessage) -> Result<(), crate::MessageError> {
        self.client_tx
            .send(msg.clone())
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;
        Ok(())
    }

    /// 接收一条服务器消息
    pub async fn recv(&self) -> Result<BusMessage, crate::MessageError> {
        let mut rx = self.server_tx.subscribe();
        rx.recv().await.map_err(|e: broadcast::error::RecvError| {
            crate::MessageError::Connection(e.to_string())
        })
    }

    /// 发送请求并等待响应（使用默认超时）
    pub async fn request_default(&self, msg: &BusMessage) -> Result<BusMessage, crate::MessageError> {
        let timeout = crate::MessageClientConfig::default().request_timeout;
        self.request(msg, timeout).await
    }

    /// 订阅服务器消息
    ///
    /// 返回一个 broadcast receiver，调用者可以在后台任务中循环接收消息。
    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.server_tx.subscribe()
    }

    /// 创建内存消息客户端 (别名方法)
    ///
    /// 等同于 `new()`，保持向后兼容
    #[allow(dead_code)]
    pub fn with_channels(
        client_tx: broadcast::Sender<BusMessage>,
        server_tx: broadcast::Sender<BusMessage>,
    ) -> Self {
        Self::new(client_tx, server_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_in_memory_client_is_connected() {
        let (tx, _rx) = broadcast::channel(16);
        let client = InMemoryMessageClient::new(tx.clone(), tx);
        assert!(client.is_connected());
    }

    #[tokio::test]
    async fn test_in_memory_client_request_timeout() {
        let (client_tx, _) = broadcast::channel(16);
        let (server_tx, _) = broadcast::channel(16);
        let client = InMemoryMessageClient::new(client_tx, server_tx);

        let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
            action: "test".to_string(),
            params: None,
        });

        // 没有响应，应该超时
        let result = client.request(&request, Duration::from_millis(100)).await;
        assert!(result.is_err());
    }
}
