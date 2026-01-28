//! 服务端消息处理器
//!
//! MessageHandler 订阅消息总线并处理业务逻辑相关的消息。
//! 针对 1-3 客户端场景简化设计，移除重试和死信队列。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::message::processor::{MessageProcessor, ProcessResult};
use crate::message::{BusMessage, EventType};
use crate::utils::AppError;

use crate::core::ServerState;

/// 服务端消息处理器
///
/// 该处理器在后台运行，处理发布到总线的所有消息，执行服务端业务逻辑。
/// 针对 1-3 客户端场景简化，失败时仅记录日志。
pub struct MessageHandler {
    receiver: broadcast::Receiver<BusMessage>,
    broadcast_tx: Option<broadcast::Sender<BusMessage>>,
    shutdown_token: CancellationToken,
    processors: HashMap<EventType, Arc<dyn MessageProcessor>>,
}

impl MessageHandler {
    /// 创建新的消息处理器
    pub fn new(
        receiver: broadcast::Receiver<BusMessage>,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            receiver,
            broadcast_tx: None,
            shutdown_token,
            processors: HashMap::new(),
        }
    }

    /// 设置广播发送端 (用于处理后发送消息)
    pub fn with_broadcast_tx(mut self, tx: broadcast::Sender<BusMessage>) -> Self {
        self.broadcast_tx = Some(tx);
        self
    }

    /// 为特定事件类型注册处理器
    pub fn register_processor(mut self, processor: Arc<dyn MessageProcessor>) -> Self {
        let event_type = processor.event_type();
        self.processors.insert(event_type, processor);
        self
    }

    /// 创建带有默认处理器的处理器实例
    pub fn with_default_processors(
        receiver: broadcast::Receiver<BusMessage>,
        shutdown_token: CancellationToken,
        state: Arc<ServerState>,
    ) -> Self {
        use crate::message::processor::*;

        Self::new(receiver, shutdown_token)
            .register_processor(Arc::new(NotificationProcessor))
            .register_processor(Arc::new(ServerCommandProcessor::new(state.clone())))
            .register_processor(Arc::new(RequestCommandProcessor::new(state)))
    }

    /// 启动消息处理
    ///
    /// 这是一个长运行任务，应该在后台生成 (spawn)。
    pub async fn run(mut self) {
        tracing::info!("🎯 Message handler started");

        loop {
            tokio::select! {
                // 监听关闭信号
                _ = self.shutdown_token.cancelled() => {
                    tracing::info!("Message handler shutting down");
                    break;
                }

                // 从总线接收消息
                msg_result = self.receiver.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            if let Err(e) = self.handle_message(&msg).await {
                                tracing::error!("Failed to handle message: {}", e);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!("Message handler lagged, skipped {} messages", skipped);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Message channel closed");
                            break;
                        }
                    }
                }
            }
        }

        tracing::info!("Message handler stopped");
    }

    /// 消息处理 (简化版，无重试)
    async fn handle_message(&mut self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
        let event_type = msg.event_type;

        // 检查是否有注册该事件类型的处理器
        if let Some(processor) = self.processors.get(&event_type) {
            self.process_message(msg, processor.clone()).await?;
        } else {
            self.handle_unregistered(msg).await?;
        }

        Ok(())
    }

    /// 简化的消息处理 (无重试，失败仅记录日志)
    async fn process_message(
        &self,
        msg: &BusMessage,
        processor: Arc<dyn MessageProcessor>,
    ) -> Result<(), AppError> {
        match processor.process(msg).await {
            Ok(result) => match result {
                ProcessResult::Success {
                    message: success_msg,
                    payload,
                } => {
                    tracing::debug!(
                        event_type = ?msg.event_type,
                        result = %success_msg,
                        "Message processed"
                    );

                    // 发送响应给客户端
                    if let (Some(source), Some(broadcast_tx)) = (&msg.source, &self.broadcast_tx) {
                        let response_payload =
                            shared::message::ResponsePayload::success(success_msg, payload);

                        let mut ack_msg = BusMessage::response(&response_payload);
                        ack_msg.correlation_id = Some(msg.request_id);
                        ack_msg.target = Some(source.clone());

                        if let Err(e) = broadcast_tx.send(ack_msg) {
                            tracing::warn!("Failed to send response: {}", e);
                        }
                    }
                    Ok(())
                }
                ProcessResult::Skipped { reason } => {
                    tracing::debug!(event_type = ?msg.event_type, reason = %reason, "Skipped");
                    Ok(())
                }
                ProcessResult::Failed { reason } => {
                    tracing::error!(
                        event_type = ?msg.event_type,
                        reason = %reason,
                        "Processing failed"
                    );

                    // 发送错误响应
                    if let (Some(source), Some(broadcast_tx)) = (&msg.source, &self.broadcast_tx) {
                        let response_payload =
                            shared::message::ResponsePayload::error(reason.clone(), None);

                        let mut ack_msg = BusMessage::response(&response_payload);
                        ack_msg.correlation_id = Some(msg.request_id);
                        ack_msg.target = Some(source.clone());

                        let _ = broadcast_tx.send(ack_msg);
                    }
                    Err(AppError::internal(format!("Processing failed: {}", reason)))
                }
                ProcessResult::Retry { reason, .. } => {
                    // 简化版不重试，直接记录为失败
                    tracing::warn!(
                        event_type = ?msg.event_type,
                        reason = %reason,
                        "Retry requested but disabled, logging as warning"
                    );
                    Ok(())
                }
            },
            Err(e) => {
                tracing::error!(event_type = ?msg.event_type, error = %e, "Processing error");
                Err(e)
            }
        }
    }

    /// 处理未注册消息类型
    async fn handle_unregistered(&self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
        tracing::warn!(
            event_type = ?msg.event_type,
            "No processor registered for event type"
        );
        Ok(())
    }
}
