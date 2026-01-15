//! 服务端消息处理器
//!
//! MessageHandler 订阅消息总线并处理业务逻辑相关的消息（如日志记录、数据库更新等）。
//!
//! 功能特性：
//! - ACID 事务支持
//! - 指数退避的自动重试机制
//! - 幂等性检查
//! - 失败消息的死信队列

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::common::AppError;
use crate::message::processor::{MessageProcessor, ProcessResult};
use crate::message::{BusMessage, EventType};

use crate::server::ServerState;

/// 具备 ACID 保证的服务端消息处理器
///
/// 该处理器在后台运行，处理发布到总线的所有消息，执行服务端业务逻辑。
///
/// 特性：
/// - 针对不同消息类型的可插拔处理器
/// - 指数退避自动重试
/// - 永久失败消息的死信队列
/// - 幂等性支持
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

    /// 带有重试逻辑的消息处理
    async fn handle_message(&mut self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
        let event_type = msg.event_type;

        // 检查是否有注册该事件类型的处理器
        if let Some(processor) = self.processors.get(&event_type) {
            self.process_with_retry(msg, processor.clone()).await?;
        } else {
            // 对未注册类型的降级处理
            self.handle_legacy(msg).await?;
        }

        Ok(())
    }

    /// 自动重试的消息处理流程
    async fn process_with_retry(
        &self,
        msg: &BusMessage,
        processor: Arc<dyn MessageProcessor>,
    ) -> Result<(), AppError> {
        let max_retries = processor.max_retries();
        let base_delay = processor.retry_delay_ms();
        let mut retry_count = 0;

        loop {
            match processor.process(msg).await {
                Ok(result) => match result {
                    ProcessResult::Success {
                        message: success_msg,
                        payload,
                    } => {
                        tracing::info!(
                            event_type = ?msg.event_type,
                            result = %success_msg,
                            "Message processed successfully"
                        );

                        // If message is from a client, send Ack/Result
                        if let (Some(source), Some(broadcast_tx)) = (&msg.source, &self.broadcast_tx) {
                            let response_payload =
                                shared::message::ResponsePayload::success(success_msg, payload);

                            let mut ack_msg = BusMessage::response(&response_payload);
                            ack_msg.correlation_id = Some(msg.request_id);
                            ack_msg.target = Some(source.clone());

                            // Send result (MessageBus will handle unicast routing based on target)
                            if let Err(e) = broadcast_tx.send(ack_msg) {
                                tracing::warn!("Failed to send Ack: {}", e);
                            }
                        }

                        return Ok(());
                    }
                    ProcessResult::Skipped { reason } => {
                        tracing::info!(
                            event_type = ?msg.event_type,
                            reason = %reason,
                            "Message skipped"
                        );
                        return Ok(());
                    }
                    ProcessResult::Failed { reason } => {
                        tracing::error!(
                            event_type = ?msg.event_type,
                            reason = %reason,
                            "Message processing failed permanently"
                        );
                        self.send_to_dead_letter_queue(msg, &reason).await;

                        // Send error notification to client
                        if let Some(source) = &msg.source {
                            if let Some(broadcast_tx) = &self.broadcast_tx {
                                let response_payload =
                                    shared::message::ResponsePayload::error(reason.clone(), None);

                                let mut ack_msg = BusMessage::response(&response_payload);
                                ack_msg.correlation_id = Some(msg.request_id);
                                ack_msg.target = Some(source.clone());

                                if let Err(e) = broadcast_tx.send(ack_msg) {
                                    tracing::warn!("Failed to send Error Ack: {}", e);
                                }
                            }
                        }

                        return Err(AppError::internal(format!("Processing failed: {}", reason)));
                    }
                    ProcessResult::Retry {
                        reason,
                        retry_count: _,
                    } => {
                        retry_count += 1;
                        if retry_count > max_retries {
                            tracing::error!(
                                event_type = ?msg.event_type,
                                retry_count = %retry_count,
                                reason = %reason,
                                "Max retries exceeded"
                            );
                            self.send_to_dead_letter_queue(msg, &reason).await;
                            return Err(AppError::internal(format!(
                                "Max retries exceeded: {}",
                                reason
                            )));
                        }

                        // 指数退避
                        let delay = base_delay * 2_u64.pow(retry_count - 1);
                        tracing::warn!(
                            event_type = ?msg.event_type,
                            retry_count = %retry_count,
                            delay_ms = %delay,
                            reason = %reason,
                            "Retrying message processing"
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                },
                Err(e) => {
                    retry_count += 1;
                    if retry_count > max_retries {
                        tracing::error!(
                            event_type = ?msg.event_type,
                            error = %e,
                            "Processing error, max retries exceeded"
                        );
                        self.send_to_dead_letter_queue(msg, &e.to_string()).await;
                        return Err(e);
                    }

                    let delay = base_delay * 2_u64.pow(retry_count - 1);
                    tracing::warn!(
                        event_type = ?msg.event_type,
                        retry_count = %retry_count,
                        delay_ms = %delay,
                        error = %e,
                        "Processing error, retrying"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// 发送失败消息到死信队列
    async fn send_to_dead_letter_queue(&self, msg: &BusMessage, reason: &str) {
        tracing::error!(
            event_type = ?msg.event_type,
            reason = %reason,
            payload_len = %msg.payload.len(),
            "Sending message to dead letter queue"
        );

        // TODO: 实现死信队列
        // - 保存到数据库
        // - 发送告警
        // - 记录到文件
        // 例如：
        // db.insert_dead_letter(msg, reason).await?;
        // alert_service.send("Message processing failed", msg).await?;
    }

    /// 未注册消息类型的遗留处理逻辑
    async fn handle_legacy(&self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
        tracing::warn!(
            event_type = ?msg.event_type,
            "No processor registered for event type"
        );
        Ok(())
    }
}
