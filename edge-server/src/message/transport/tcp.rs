//! TCP 传输层实现

use std::sync::Arc;

use async_trait::async_trait;
use shared::message::BusMessage;
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use super::{Transport, read_from_stream, write_to_stream};
use crate::utils::AppError;

/// TCP 传输实现
#[derive(Debug, Clone)]
pub struct TcpTransport {
    reader: Arc<Mutex<OwnedReadHalf>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
    addr: Option<String>,
}

impl TcpTransport {
    /// 连接到指定地址
    pub async fn connect(addr: &str) -> Result<Self, AppError> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| AppError::internal(format!("TCP connect failed: {}", e)))?;
        let peer_addr = stream.peer_addr().ok().map(|a| a.to_string());
        let (reader, writer) = stream.into_split();
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            addr: peer_addr,
        })
    }

    /// 从已有的 TcpStream 创建
    pub fn from_stream(stream: TcpStream) -> Self {
        let peer_addr = stream.peer_addr().ok().map(|a| a.to_string());
        let (reader, writer) = stream.into_split();
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            addr: peer_addr,
        }
    }

    pub async fn read_message(&self) -> Result<BusMessage, AppError> {
        let mut reader = self.reader.lock().await;
        read_from_stream(&mut *reader).await
    }

    pub async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        let mut writer = self.writer.lock().await;
        write_to_stream(&mut *writer, msg).await
    }

    pub async fn close(&self) -> Result<(), AppError> {
        use tokio::io::AsyncWriteExt;
        let mut writer = self.writer.lock().await;
        writer
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("TCP close failed: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn read_message(&self) -> Result<BusMessage, AppError> {
        self.read_message().await
    }

    async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        self.write_message(msg).await
    }

    async fn close(&self) -> Result<(), AppError> {
        use tokio::io::AsyncWriteExt;
        let mut writer = self.writer.lock().await;
        writer
            .shutdown()
            .await
            .map_err(|e| AppError::internal(format!("TCP close failed: {}", e)))?;
        Ok(())
    }

    fn peer_addr(&self) -> Option<String> {
        self.addr.clone()
    }
}
