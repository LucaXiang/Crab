//! TLS 传输层实现 (mTLS 支持)

use std::sync::Arc;

use async_trait::async_trait;
use crab_cert::CertMetadata;
use shared::message::BusMessage;
use tokio::io::{ReadHalf, WriteHalf, split};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_rustls::server::TlsStream;

use super::{Transport, read_from_stream, write_to_stream};
use crate::utils::AppError;

/// TLS 传输实现 (支持 mTLS)
#[derive(Debug, Clone)]
pub struct TlsTransport {
    reader: Arc<Mutex<ReadHalf<TlsStream<TcpStream>>>>,
    writer: Arc<Mutex<WriteHalf<TlsStream<TcpStream>>>>,
    peer_identity: Option<String>,
    addr: Option<String>,
}

impl TlsTransport {
    /// 从 TLS 流创建传输层
    ///
    /// 自动从客户端证书中提取身份标识
    pub fn new(stream: TlsStream<TcpStream>) -> Self {
        // Extract identity from TLS session
        let (io, connection) = stream.get_ref();
        let peer_addr = io.peer_addr().ok().map(|a| a.to_string());

        let peer_identity = if let Some(certs) = connection.peer_certificates() {
            if let Some(cert) = certs.first() {
                CertMetadata::from_der(cert.as_ref())
                    .ok()
                    .and_then(|m| m.client_name.or(m.common_name))
            } else {
                None
            }
        } else {
            None
        };

        let (reader, writer) = split(stream);
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            peer_identity,
            addr: peer_addr,
        }
    }
}

#[async_trait]
impl Transport for TlsTransport {
    async fn read_message(&self) -> Result<BusMessage, AppError> {
        let mut reader = self.reader.lock().await;
        read_from_stream(&mut *reader).await
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        write_to_stream(&mut *writer, msg).await
    }

    async fn close(&self) -> Result<(), AppError> {
        use tokio::io::AsyncWriteExt;
        let mut writer = self.writer.lock().await;
        writer
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("TLS close failed: {}", e)))?;
        Ok(())
    }

    fn peer_identity(&self) -> Option<String> {
        self.peer_identity.clone()
    }

    fn peer_addr(&self) -> Option<String> {
        self.addr.clone()
    }
}
