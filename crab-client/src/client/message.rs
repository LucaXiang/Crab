// crab-client/src/client/message.rs
// 消息客户端 - mTLS TCP 和内存通信

use async_trait::async_trait;
use rustls_pki_types::{CertificateDer, ServerName};
use shared::message::{BusMessage, HandshakePayload, PROTOCOL_VERSION};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio_rustls::client::TlsStream;
use uuid::Uuid;

/// 消息客户端 trait
#[async_trait]
pub trait MessageClient: Send + Sync {
    async fn send(&self, msg: &BusMessage) -> Result<(), crate::MessageError>;
    async fn recv(&self) -> Result<BusMessage, crate::MessageError>;
}

/// mTLS TCP 消息客户端
///
/// 通过 mTLS 连接到 Edge Server 的消息总线：
/// 1. TCP 连接到服务器
/// 2. TLS 握手 (使用客户端证书)
/// 3. 协议握手 (Handshake)
/// 4. 消息收发
#[derive(Debug, Clone)]
pub struct NetworkMessageClient {
    stream: Arc<Mutex<TlsStream<TcpStream>>>,
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
        use tokio_rustls::TlsConnector;

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
                crate::MessageError::Connection(format!("Failed to build TLS config: {}", e))
            })?;

        // 5. 建立 TCP 连接
        let tcp_stream = TcpStream::connect(addr)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("TCP connect failed: {}", e)))?;

        // 6. TLS 握手 (使用任意 server_name，因为 SkipHostnameVerifier 会从证书中提取名称)
        let connector = TlsConnector::from(Arc::new(tls_config));
        // 任意 server_name 都可以，SkipHostnameVerifier 会从证书 SANs 中提取实际名称
        let domain = ServerName::try_from("edge-server")
            .map_err(|e| crate::MessageError::Connection(format!("Invalid domain: {}", e)))?;

        let tls_stream = connector
            .connect(domain, tcp_stream)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("TLS handshake failed: {}", e)))?;

        let client = Self {
            stream: Arc::new(Mutex::new(tls_stream)),
        };

        // 7. 协议握手
        tracing::info!("Starting protocol handshake...");
        client.perform_handshake(client_name).await?;
        tracing::info!("Protocol handshake completed!");

        Ok(client)
    }

    /// 执行协议握手
    async fn perform_handshake(&self, client_name: &str) -> Result<(), crate::MessageError> {
        let handshake = BusMessage::handshake(&HandshakePayload {
            version: PROTOCOL_VERSION,
            client_name: Some(client_name.to_string()),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            client_id: Some(Uuid::new_v4().to_string()),
        });

        tracing::debug!("Sending handshake message...");
        self.write_message(&handshake).await?;
        tracing::debug!("Handshake message sent, waiting for response...");

        // 等待服务器响应 (RPC 响应)
        let response = self.read_message().await?;
        tracing::debug!("Received response: event_type={}", response.event_type);

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
        let mut guard = self.stream.lock().await;
        let stream = &mut *guard;

        // 序列化消息
        let mut data = Vec::new();
        data.push(msg.event_type as u8);
        data.extend_from_slice(msg.request_id.as_bytes());

        // correlation_id (16 bytes) - nil UUID if None
        let correlation_bytes = msg.correlation_id.unwrap_or(Uuid::nil()).into_bytes();
        data.extend_from_slice(&correlation_bytes);

        data.extend_from_slice(&(msg.payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&msg.payload);

        stream
            .write_all(&data)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Write failed: {}", e)))?;

        Ok(())
    }

    /// 读取消息
    async fn read_message(&self) -> Result<BusMessage, crate::MessageError> {
        use shared::message::EventType;
        tracing::debug!("read_message: acquiring stream lock...");
        let mut guard = self.stream.lock().await;
        let stream = &mut *guard;
        tracing::debug!("read_message: stream lock acquired");

        // 读取事件类型 (1 字节)
        tracing::debug!("read_message: reading event type...");
        let type_buf = &mut [0u8; 1];
        stream
            .read_exact(type_buf)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("Read type failed: {}", e)))?;

        let event_type = EventType::try_from(type_buf[0])
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

    /// 关闭连接
    pub async fn close(&self) -> Result<(), crate::MessageError> {
        // TlsStream 在 drop 时自动关闭
        Ok(())
    }
}

#[async_trait]
impl MessageClient for NetworkMessageClient {
    async fn send(&self, msg: &BusMessage) -> Result<(), crate::MessageError> {
        self.write_message(msg).await
    }

    async fn recv(&self) -> Result<BusMessage, crate::MessageError> {
        self.read_message().await
    }
}

/// 内存消息客户端 (同进程通信)
///
/// 使用 broadcast 通道实现：
/// - send() -> broadcast::Sender::send()
/// - recv() -> broadcast::Receiver::recv()
///
/// # Example
/// ```rust
/// use tokio::sync::broadcast;
/// use crab_client::{MessageClient, InMemoryMessageClient};
///
/// #[tokio::main]
/// async fn main() {
///     let (tx, _rx) = broadcast::channel(16);
///     let client = InMemoryMessageClient::new(tx);
///
///     // 发送消息
///     let msg = shared::message::BusMessage::notification(
///         &shared::message::NotificationPayload::info("test", "hello"),
///     );
///     client.send(&msg).await.unwrap();
///
///     // 接收消息
///     let received = client.recv().await.unwrap();
/// }
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InMemoryMessageClient {
    tx: Arc<broadcast::Sender<BusMessage>>,
}

#[allow(dead_code)]
impl InMemoryMessageClient {
    /// 创建连接到 MessageBus 的内存客户端
    ///
    /// # Arguments
    /// * `sender` - MessageBus.server_tx 的克隆
    pub fn new(sender: broadcast::Sender<BusMessage>) -> Self {
        Self {
            tx: Arc::new(sender),
        }
    }

    /// 创建仅发送的客户端 (不接收消息)
    pub fn sender_only(sender: broadcast::Sender<BusMessage>) -> Self {
        Self {
            tx: Arc::new(sender),
        }
    }

    /// 获取发送端供其他客户端使用
    pub fn sender(&self) -> broadcast::Sender<BusMessage> {
        (*self.tx).clone()
    }
}

#[async_trait]
impl MessageClient for InMemoryMessageClient {
    async fn send(&self, msg: &BusMessage) -> Result<(), crate::MessageError> {
        self.tx
            .send(msg.clone())
            .map_err(|e| crate::MessageError::Connection(e.to_string()))?;
        Ok(())
    }

    async fn recv(&self) -> Result<BusMessage, crate::MessageError> {
        // 每次接收时创建新的 receiver，避免存储问题
        let mut rx = self.tx.subscribe();
        rx.recv().await.map_err(|e: broadcast::error::RecvError| {
            crate::MessageError::Connection(e.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_in_memory_client_send_recv() {
        let (tx, _rx) = broadcast::channel(16);
        let client = InMemoryMessageClient::new(tx);

        // 使用 spawn 先启动接收任务（确保 receiver 在发送前已创建）
        let client_clone = client.clone();
        let recv_task = tokio::spawn(async move { client_clone.recv().await.unwrap() });

        // 等待接收任务开始运行
        tokio::time::sleep(Duration::from_millis(10)).await;

        // 发送消息
        let msg =
            BusMessage::notification(&shared::message::NotificationPayload::info("Test", "Hello"));
        client.send(&msg).await.unwrap();

        // 验证收到消息
        let received = timeout(Duration::from_secs(1), recv_task)
            .await
            .expect("recv() should receive message within 1 second")
            .expect("recv() should succeed");

        assert_eq!(
            received.event_type,
            shared::message::EventType::Notification
        );
    }

    #[tokio::test]
    async fn test_in_memory_client_sender_only() {
        let (tx, mut rx) = broadcast::channel(16);
        let client = InMemoryMessageClient::sender_only(tx);

        let msg = BusMessage::server_command(&shared::message::ServerCommandPayload {
            command: shared::message::ServerCommand::Ping,
        });

        // Client can send
        client.send(&msg).await.unwrap();

        // External receiver should get the message
        let received = rx.recv().await.unwrap();
        assert_eq!(
            received.event_type,
            shared::message::EventType::ServerCommand
        );
    }
}
