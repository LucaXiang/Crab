//! Transport 传输层抽象
//!
//! 提供可插拔的传输层架构：
//! ```text
//!         ┌────────────────────┐
//!         │   Transport Trait  │  ◄── 可插拔接口
//!         └────────┬───────────┘
//!                  │
//!     ┌────────────┼────────────┐
//!     ▼            ▼            ▼
//! TcpTransport  TlsTransport  MemoryTransport
//! (TCP 协议)    (TLS 加密)    (同进程通信)
//! ```

mod memory;
mod tcp;
mod tls;

pub use memory::MemoryTransport;
pub use tcp::TcpTransport;
pub use tls::TlsTransport;

use async_trait::async_trait;
use shared::message::BusMessage;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::utils::AppError;

/// Transport 传输层特征
///
/// 所有传输实现必须实现此特征，支持消息的读写和连接管理。
#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    /// 从传输层读取一条消息
    async fn read_message(&self) -> Result<BusMessage, AppError>;

    /// 向传输层写入一条消息
    async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError>;

    /// 关闭传输连接
    async fn close(&self) -> Result<(), AppError>;

    /// 获取对端身份标识 (mTLS 场景下从证书提取)
    fn peer_identity(&self) -> Option<String> {
        None
    }

    /// 获取对端地址
    fn peer_addr(&self) -> Option<String> {
        None
    }
}

// ========== 辅助函数 ==========

/// 从异步流中读取 BusMessage
pub(crate) async fn read_from_stream<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<BusMessage, AppError> {
    use shared::message::EventType;

    // 读取事件类型 (1 字节)
    let mut type_buf = [0u8; 1];
    match reader.read_exact(&mut type_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(AppError::ClientDisconnected);
        }
        Err(e) => {
            // Handle rustls "peer closed connection without sending TLS close_notify"
            if e.to_string().contains("close_notify") {
                return Err(AppError::ClientDisconnected);
            }
            return Err(AppError::internal(format!("Read type failed: {}", e)));
        }
    }

    let event_type =
        EventType::try_from(type_buf[0]).map_err(|_| AppError::invalid("Invalid event type"))?;

    // 读取 Request ID (16 字节)
    let mut uuid_buf = [0u8; 16];
    reader
        .read_exact(&mut uuid_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read UUID failed: {}", e)))?;
    let request_id = Uuid::from_bytes(uuid_buf);

    // 读取 Correlation ID (16 字节)
    let mut correlation_buf = [0u8; 16];
    reader
        .read_exact(&mut correlation_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read Correlation UUID failed: {}", e)))?;
    let correlation_id_raw = Uuid::from_bytes(correlation_buf);
    let correlation_id = if correlation_id_raw.is_nil() {
        None
    } else {
        Some(correlation_id_raw)
    };

    // 读取载荷长度 (4 字节)
    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| AppError::internal(format!("Read len failed: {}", e)))?;

    let len = u32::from_le_bytes(len_buf) as usize;

    // 读取载荷内容
    let mut payload = vec![0u8; len];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|e| AppError::internal(format!("Read payload failed: {}", e)))?;

    Ok(BusMessage {
        request_id,
        event_type,
        source: None,
        correlation_id,
        target: None,
        payload,
    })
}

/// 向异步流写入 BusMessage
pub(crate) async fn write_to_stream<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &BusMessage,
) -> Result<(), AppError> {
    let mut data = Vec::new();
    data.push(msg.event_type as u8);
    data.extend_from_slice(msg.request_id.as_bytes());

    // Write correlation_id (16 bytes) - using nil UUID if None
    let correlation_bytes = msg.correlation_id.unwrap_or(Uuid::nil()).into_bytes();
    data.extend_from_slice(&correlation_bytes);

    data.extend_from_slice(&(msg.payload.len() as u32).to_le_bytes());
    data.extend_from_slice(&msg.payload);

    writer
        .write_all(&data)
        .await
        .map_err(|e| AppError::internal(format!("Write failed: {}", e)))?;
    Ok(())
}
