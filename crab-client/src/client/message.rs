// crab-client/src/client/message.rs
// RPC 消息客户端 - mTLS 和内存通信

use rustls_pki_types::{CertificateDer, ServerName};
use shared::message::{BusMessage, HandshakePayload, PROTOCOL_VERSION, RequestCommandPayload};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify, RwLock, broadcast, oneshot};
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use uuid::Uuid;

use crate::MessageClientConfig;

/// 连接参数 (用于重连)
#[derive(Clone)]
struct ConnectionParams {
    addr: String,
    ca_cert_pem: Vec<u8>,
    client_cert_pem: Vec<u8>,
    client_key_pem: Vec<u8>,
    client_name: String,
}

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// 已连接
    Connected,
    /// 断开连接
    Disconnected,
    /// 正在重连
    Reconnecting,
}

/// 重连事件 (通知订阅者)
#[derive(Debug, Clone)]
pub enum ReconnectEvent {
    /// 连接断开
    Disconnected,
    /// 重连成功
    Reconnected,
    /// 重连失败 (达到最大重试次数)
    ReconnectFailed { attempts: u32 },
}

/// 心跳状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeartbeatStatus {
    /// 是否健康 (最近一次心跳成功)
    pub healthy: bool,
    /// 服务器 epoch (用于检测服务器重启)
    pub server_epoch: Option<String>,
    /// 服务器时间
    pub server_time: Option<String>,
    /// 上次心跳时间 (Unix 毫秒)
    pub last_heartbeat_ms: u64,
}

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
/// - 心跳任务检测连接状态
/// - 自动重连并通知订阅者
#[derive(Clone)]
pub struct NetworkMessageClient {
    /// 写入流 (发送请求)
    write_stream: Arc<RwLock<Option<WriteHalf<TlsStream<TcpStream>>>>>,
    /// 连接状态
    state: Arc<AtomicU32>,
    /// 非响应消息广播通道 (通知、同步信号等)
    notification_tx: broadcast::Sender<BusMessage>,
    /// 等待响应的 RPC 请求 (correlation_id -> response sender)
    pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>>,
    /// 重连事件通道
    reconnect_tx: broadcast::Sender<ReconnectEvent>,
    /// 心跳状态通道
    heartbeat_tx: broadcast::Sender<HeartbeatStatus>,
    /// 连接参数 (用于重连)
    conn_params: Arc<RwLock<Option<ConnectionParams>>>,
    /// 配置
    config: Arc<MessageClientConfig>,
    /// 停止信号
    stop_notify: Arc<Notify>,
    /// 是否已停止
    stopped: Arc<AtomicBool>,
}

impl std::fmt::Debug for NetworkMessageClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkMessageClient")
            .field("state", &self.get_state())
            .field("stopped", &self.stopped.load(Ordering::SeqCst))
            .finish()
    }
}

impl NetworkMessageClient {
    /// 获取连接状态
    fn get_state(&self) -> ConnectionState {
        match self.state.load(Ordering::SeqCst) {
            0 => ConnectionState::Connected,
            1 => ConnectionState::Disconnected,
            _ => ConnectionState::Reconnecting,
        }
    }

    /// 设置连接状态
    fn set_state(&self, state: ConnectionState) {
        let val = match state {
            ConnectionState::Connected => 0,
            ConnectionState::Disconnected => 1,
            ConnectionState::Reconnecting => 2,
        };
        self.state.store(val, Ordering::SeqCst);
    }

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
        Self::connect_mtls_with_config(
            addr,
            ca_cert_pem,
            client_cert_pem,
            client_key_pem,
            client_name,
            MessageClientConfig::default(),
        )
        .await
    }

    /// 使用 mTLS 连接到 Edge Server (带配置)
    pub async fn connect_mtls_with_config(
        addr: &str,
        ca_cert_pem: &[u8],
        client_cert_pem: &[u8],
        client_key_pem: &[u8],
        client_name: &str,
        config: MessageClientConfig,
    ) -> Result<Self, crate::MessageError> {
        use tokio::io::split;

        // 保存连接参数
        let conn_params = ConnectionParams {
            addr: addr.to_string(),
            ca_cert_pem: ca_cert_pem.to_vec(),
            client_cert_pem: client_cert_pem.to_vec(),
            client_key_pem: client_key_pem.to_vec(),
            client_name: client_name.to_string(),
        };

        // 建立 TLS 连接
        let tls_stream = Self::establish_tls_connection(&conn_params).await?;

        // 分离读写流
        let (read_half, write_half) = split(tls_stream);

        // 创建通道
        let (notification_tx, _) = broadcast::channel(64);
        let (reconnect_tx, _) = broadcast::channel(16);
        let (heartbeat_tx, _) = broadcast::channel(16);
        let pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let stop_notify = Arc::new(Notify::new());

        let client = Self {
            write_stream: Arc::new(RwLock::new(Some(write_half))),
            state: Arc::new(AtomicU32::new(0)), // Connected
            notification_tx,
            pending_requests,
            reconnect_tx,
            heartbeat_tx,
            conn_params: Arc::new(RwLock::new(Some(conn_params))),
            config: Arc::new(config),
            stop_notify,
            stopped: Arc::new(AtomicBool::new(false)),
        };

        // 启动后台读取任务
        client.spawn_reader_task(read_half);

        // 协议握手
        tracing::info!("Starting protocol handshake...");
        client.perform_handshake(client_name).await?;
        tracing::info!("Protocol handshake completed!");

        // 启动心跳任务 (如果配置了)
        if client.config.heartbeat_interval > Duration::ZERO {
            client.spawn_heartbeat_task();
        }

        Ok(client)
    }

    /// 建立 TLS 连接 (内部方法，用于初始连接和重连)
    async fn establish_tls_connection(
        params: &ConnectionParams,
    ) -> Result<TlsStream<TcpStream>, crate::MessageError> {
        // 安装 aws-lc-rs CryptoProvider
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        // 解析 CA 证书链
        let mut ca_reader = std::io::Cursor::new(&params.ca_cert_pem);
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

        // 解析客户端证书
        let mut cert_reader = std::io::Cursor::new(&params.client_cert_pem);
        let client_certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<_, _>>()
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to parse client cert: {}", e))
            })?;
        let client_cert = client_certs.into_iter().next().ok_or_else(|| {
            crate::MessageError::Connection("No client certificate found".to_string())
        })?;

        // 解析私钥
        let mut key_reader = std::io::Cursor::new(&params.client_key_pem);
        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to parse private key: {}", e))
            })?
            .ok_or_else(|| crate::MessageError::Connection("No private key found".to_string()))?;

        // 构建 TLS 配置
        let mut root_store = rustls::RootCertStore::empty();
        for ca_cert in ca_certs {
            root_store.add(ca_cert).map_err(|e| {
                crate::MessageError::Connection(format!("Failed to add CA cert: {}", e))
            })?;
        }

        let verifier = crab_cert::SkipHostnameVerifier::new(root_store);
        let client_cert_chain = vec![client_cert];
        let tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_client_auth_cert(client_cert_chain, private_key)
            .map_err(|e| {
                crate::MessageError::Connection(format!("Failed to configure TLS: {}", e))
            })?;

        // TCP 连接
        let tcp_stream = TcpStream::connect(&params.addr)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("TCP connect failed: {}", e)))?;

        // TLS 握手
        let connector = TlsConnector::from(Arc::new(tls_config));
        let domain = ServerName::try_from("edge-server")
            .map_err(|e| crate::MessageError::Connection(format!("Invalid domain: {}", e)))?;

        connector
            .connect(domain, tcp_stream)
            .await
            .map_err(|e| crate::MessageError::Connection(format!("TLS handshake failed: {}", e)))
    }

    /// 启动后台读取任务
    fn spawn_reader_task(&self, read_half: ReadHalf<TlsStream<TcpStream>>) {
        let pending = self.pending_requests.clone();
        let notify_tx = self.notification_tx.clone();
        let state = self.state.clone();
        let reconnect_tx = self.reconnect_tx.clone();
        let stop_notify = self.stop_notify.clone();
        let stopped = self.stopped.clone();

        tokio::spawn(async move {
            Self::reader_task_loop(
                read_half,
                pending,
                notify_tx,
                state,
                reconnect_tx,
                stop_notify,
                stopped,
            )
            .await;
        });
    }

    /// 启动心跳任务
    fn spawn_heartbeat_task(&self) {
        let client = self.clone();
        tokio::spawn(async move {
            client.heartbeat_task_loop().await;
        });
    }

    /// 心跳任务循环
    async fn heartbeat_task_loop(&self) {
        let interval = self.config.heartbeat_interval;
        let timeout = self.config.heartbeat_timeout;

        loop {
            // 检查是否已停止
            if self.stopped.load(Ordering::SeqCst) {
                tracing::debug!("Heartbeat task: stopped");
                break;
            }

            // 等待心跳间隔
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = self.stop_notify.notified() => {
                    tracing::debug!("Heartbeat task: received stop signal");
                    break;
                }
            }

            // 只在连接状态下发送心跳
            if self.get_state() != ConnectionState::Connected {
                continue;
            }

            // 发送 ping
            let ping_msg = BusMessage::request_command(&RequestCommandPayload {
                action: "ping".to_string(),
                params: None,
            });

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            match self.request(&ping_msg, timeout).await {
                Ok(response) => {
                    tracing::trace!("Heartbeat: pong received");

                    // 解析 pong 响应中的 epoch 和 server_time
                    let (server_epoch, server_time) = if let Ok(payload) =
                        response.parse_payload::<shared::message::ResponsePayload>()
                    {
                        if let Some(data) = payload.data {
                            let epoch =
                                data.get("epoch").and_then(|v| v.as_str()).map(String::from);
                            let time = data
                                .get("server_time")
                                .and_then(|v| v.as_str())
                                .map(String::from);
                            (epoch, time)
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                    // 广播心跳状态
                    let _ = self.heartbeat_tx.send(HeartbeatStatus {
                        healthy: true,
                        server_epoch,
                        server_time,
                        last_heartbeat_ms: now_ms,
                    });
                }
                Err(e) => {
                    tracing::warn!("Heartbeat failed: {}", e);

                    // 广播心跳失败状态
                    let _ = self.heartbeat_tx.send(HeartbeatStatus {
                        healthy: false,
                        server_epoch: None,
                        server_time: None,
                        last_heartbeat_ms: now_ms,
                    });

                    // 心跳失败，触发断连处理
                    self.handle_disconnection().await;
                }
            }
        }
    }

    /// 处理断连
    async fn handle_disconnection(&self) {
        // 避免重复处理
        if self.get_state() != ConnectionState::Connected {
            return;
        }

        tracing::info!("Connection lost, starting reconnection...");
        self.set_state(ConnectionState::Disconnected);

        // 通知订阅者
        let _ = self.reconnect_tx.send(ReconnectEvent::Disconnected);

        // 如果启用了自动重连
        if self.config.auto_reconnect {
            self.spawn_reconnect_task();
        }
    }

    /// 启动重连任务
    fn spawn_reconnect_task(&self) {
        let client = self.clone();
        tokio::spawn(async move {
            client.reconnect_loop().await;
        });
    }

    /// 重连循环
    async fn reconnect_loop(&self) {
        self.set_state(ConnectionState::Reconnecting);

        let mut delay = self.config.reconnect_delay;
        let max_delay = self.config.max_reconnect_delay;
        let mut attempts = 0u32;

        // 获取连接参数
        let params = {
            let guard = self.conn_params.read().await;
            guard.clone()
        };

        let Some(params) = params else {
            tracing::error!("No connection parameters for reconnection");
            self.set_state(ConnectionState::Disconnected);
            let _ = self
                .reconnect_tx
                .send(ReconnectEvent::ReconnectFailed { attempts: 0 });
            return;
        };

        loop {
            // 检查是否已停止
            if self.stopped.load(Ordering::SeqCst) {
                tracing::debug!("Reconnect task: stopped");
                break;
            }

            attempts += 1;
            tracing::info!("Reconnection attempt #{}", attempts);

            // 尝试重连
            match Self::establish_tls_connection(&params).await {
                Ok(tls_stream) => {
                    use tokio::io::split;
                    let (read_half, write_half) = split(tls_stream);

                    // 更新写入流
                    {
                        let mut guard = self.write_stream.write().await;
                        *guard = Some(write_half);
                    }

                    // 协议握手
                    if let Err(e) = self.perform_handshake(&params.client_name).await {
                        tracing::warn!("Handshake failed after reconnect: {}", e);
                        // 继续重试
                    } else {
                        tracing::info!("Reconnected successfully after {} attempts", attempts);
                        self.set_state(ConnectionState::Connected);

                        // 启动新的读取任务
                        self.spawn_reader_task(read_half);

                        // 通知订阅者
                        let _ = self.reconnect_tx.send(ReconnectEvent::Reconnected);
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("Reconnection attempt #{} failed: {}", attempts, e);
                }
            }

            // 智能等待：在退避期间定期探测网络
            // 网络恢复立即重连
            let probe_interval = self.config.reconnect_probe_interval;
            let mut remaining = delay;

            while remaining > Duration::ZERO {
                let wait_time = std::cmp::min(remaining, probe_interval);

                tokio::select! {
                    _ = tokio::time::sleep(wait_time) => {}
                    _ = self.stop_notify.notified() => {
                        tracing::debug!("Reconnect task: received stop signal");
                        return;
                    }
                }

                remaining = remaining.saturating_sub(wait_time);

                // 如果还有剩余等待时间，做快速网络探测
                if remaining > Duration::ZERO && Self::quick_tcp_probe(&params.addr).await {
                    tracing::info!("Network recovered, attempting immediate reconnection");
                    break; // 跳出等待循环，立即重连
                }
            }

            // 增加延迟 (指数退避，上限为 max_delay)
            delay = std::cmp::min(delay * 2, max_delay);
        }
    }

    /// 快速 TCP 探测 - 检测网络是否可达
    async fn quick_tcp_probe(addr: &str) -> bool {
        matches!(tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(addr)).await, Ok(Ok(_)))
    }

    /// 后台读取任务循环
    async fn reader_task_loop(
        mut read_half: ReadHalf<TlsStream<TcpStream>>,
        pending_requests: Arc<Mutex<HashMap<Uuid, oneshot::Sender<BusMessage>>>>,
        notification_tx: broadcast::Sender<BusMessage>,
        state: Arc<AtomicU32>,
        reconnect_tx: broadcast::Sender<ReconnectEvent>,
        stop_notify: Arc<Notify>,
        stopped: Arc<AtomicBool>,
    ) {
        loop {
            // 检查是否已停止
            if stopped.load(Ordering::SeqCst) {
                tracing::debug!("Reader task: stopped");
                break;
            }

            // 检查连接状态
            let current_state = match state.load(Ordering::SeqCst) {
                0 => ConnectionState::Connected,
                1 => ConnectionState::Disconnected,
                _ => ConnectionState::Reconnecting,
            };

            if current_state != ConnectionState::Connected {
                tracing::debug!("Reader task: not connected, exiting");
                break;
            }

            // 使用 select 同时监听停止信号和消息
            tokio::select! {
                result = Self::read_message_from(&mut read_half) => {
                    match result {
                        Ok(msg) => {
                            // 检查是否是 RPC 响应
                            if let Some(correlation_id) = msg.correlation_id {
                                let mut pending = pending_requests.lock().await;
                                if let Some(tx) = pending.remove(&correlation_id) {
                                    let _ = tx.send(msg);
                                    continue;
                                }
                            }
                            // 非响应消息，广播给订阅者
                            let _ = notification_tx.send(msg);
                        }
                        Err(e) => {
                            tracing::debug!("Reader task error: {}", e);
                            // 标记断连
                            state.store(1, Ordering::SeqCst); // Disconnected
                            let _ = reconnect_tx.send(ReconnectEvent::Disconnected);
                            break;
                        }
                    }
                }
                _ = stop_notify.notified() => {
                    tracing::debug!("Reader task: received stop signal");
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
        if response.event_type == shared::message::EventType::Response
            && let Ok(payload) = response.parse_payload::<shared::message::ResponsePayload>()
        {
            if !payload.success {
                return Err(crate::MessageError::Connection(format!(
                    "Handshake failed: {}",
                    payload.message
                )));
            }
            tracing::debug!("Handshake successful: {}", payload.message);
        }

        Ok(())
    }

    /// 写入消息
    async fn write_message(&self, msg: &BusMessage) -> Result<(), crate::MessageError> {
        // 检查是否有活跃连接
        {
            let guard = self.write_stream.read().await;
            if guard.is_none() {
                return Err(crate::MessageError::Connection(
                    "No active connection".to_string(),
                ));
            }
        }

        // 获取写锁来实际写入
        let mut guard = self.write_stream.write().await;
        let stream = guard
            .as_mut()
            .ok_or_else(|| crate::MessageError::Connection("No active connection".to_string()))?;

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
        self.stopped.store(true, Ordering::SeqCst);
        self.set_state(ConnectionState::Disconnected);
        self.stop_notify.notify_waiters();

        // 清理写入流
        let mut guard = self.write_stream.write().await;
        *guard = None;

        Ok(())
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.get_state() == ConnectionState::Connected
    }

    /// 获取当前连接状态
    pub fn connection_state(&self) -> ConnectionState {
        self.get_state()
    }

    /// 订阅重连事件
    ///
    /// 当连接断开或重连成功时会收到通知。
    /// 订阅者应在收到 `Reconnected` 事件后刷新数据。
    pub fn subscribe_reconnect(&self) -> broadcast::Receiver<ReconnectEvent> {
        self.reconnect_tx.subscribe()
    }

    /// 订阅心跳状态
    ///
    /// 每次心跳成功或失败时会收到通知。
    /// 可用于前端显示连接状态和服务器信息。
    pub fn subscribe_heartbeat(&self) -> broadcast::Receiver<HeartbeatStatus> {
        self.heartbeat_tx.subscribe()
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

            // 写入失败可能意味着连接已断开
            self.handle_disconnection().await;
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
    pub async fn request_default(
        &self,
        msg: &BusMessage,
    ) -> Result<BusMessage, crate::MessageError> {
        self.request(msg, self.config.request_timeout).await
    }

    /// 手动触发重连
    pub async fn reconnect(&self) -> Result<(), crate::MessageError> {
        if self.get_state() == ConnectionState::Connected {
            return Ok(());
        }

        // 获取连接参数
        let params = {
            let guard = self.conn_params.read().await;
            guard.clone()
        };

        let Some(params) = params else {
            return Err(crate::MessageError::Connection(
                "No connection parameters for reconnection".to_string(),
            ));
        };

        // 尝试重连
        use tokio::io::split;
        let tls_stream = Self::establish_tls_connection(&params).await?;
        let (read_half, write_half) = split(tls_stream);

        // 更新写入流
        {
            let mut guard = self.write_stream.write().await;
            *guard = Some(write_half);
        }

        // 协议握手
        self.perform_handshake(&params.client_name).await?;

        self.set_state(ConnectionState::Connected);

        // 启动新的读取任务
        self.spawn_reader_task(read_half);

        // 通知订阅者
        let _ = self.reconnect_tx.send(ReconnectEvent::Reconnected);

        Ok(())
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
        Self {
            client_tx,
            server_tx,
        }
    }

    /// 创建内存消息客户端 (只需服务器 → 客户端通道)
    ///
    /// 适用于只需要接收广播的场景
    #[allow(dead_code)]
    pub fn new_receiver(server_tx: broadcast::Sender<BusMessage>) -> Self {
        let (client_tx, _) = broadcast::channel(16);
        Self {
            client_tx,
            server_tx,
        }
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
    pub async fn request_default(
        &self,
        msg: &BusMessage,
    ) -> Result<BusMessage, crate::MessageError> {
        let timeout = crate::MessageClientConfig::default().request_timeout;
        self.request(msg, timeout).await
    }

    /// 订阅服务器消息
    ///
    /// 返回一个 broadcast receiver，调用者可以在后台任务中循环接收消息。
    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.server_tx.subscribe()
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
