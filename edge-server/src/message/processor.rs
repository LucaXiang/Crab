//! Message Processor Trait
//!
//! Provides a pluggable architecture for message processing with ACID guarantees.

use async_trait::async_trait;
use std::sync::Arc;

use crate::common::AppError;
use crate::message::{BusMessage, EventType};

/// Result of message processing
#[derive(Debug, Clone)]
pub enum ProcessResult {
    /// Message processed successfully
    Success { message: String },
    /// Message processing failed, should retry
    Retry { reason: String, retry_count: u32 },
    /// Message processing failed permanently, do not retry
    Failed { reason: String },
    /// Message skipped (e.g., duplicate)
    Skipped { reason: String },
}

impl ProcessResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ProcessResult::Success { .. })
    }

    pub fn should_retry(&self) -> bool {
        matches!(self, ProcessResult::Retry { .. })
    }
}

/// Message Processor trait
///
/// Implement this trait to create custom message processors with:
/// - ACID transaction support
/// - Idempotency checks
/// - Retry logic
/// - Error recovery
#[async_trait]
pub trait MessageProcessor: Send + Sync {
    /// Get the event type this processor handles
    fn event_type(&self) -> EventType;

    /// Process a message with ACID guarantees
    ///
    /// # Implementation Guidelines
    ///
    /// 1. **Idempotency**: Check if message was already processed
    /// 2. **Atomicity**: Use database transactions
    /// 3. **Consistency**: Validate data before processing
    /// 4. **Isolation**: Use appropriate transaction isolation levels
    /// 5. **Durability**: Commit to database before returning Success
    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError>;

    /// Check if a message was already processed (idempotency)
    ///
    /// Default implementation returns false (always process).
    /// Override for idempotency support.
    async fn is_duplicate(&self, _msg: &BusMessage) -> Result<bool, AppError> {
        Ok(false)
    }

    /// Maximum retry attempts for this processor
    fn max_retries(&self) -> u32 {
        3
    }

    /// Delay between retries (in milliseconds)
    fn retry_delay_ms(&self) -> u64 {
        1000
    }
}

/// Notification processor - handles system notifications
pub struct NotificationProcessor;

#[async_trait]
impl MessageProcessor for NotificationProcessor {
    fn event_type(&self) -> EventType {
        EventType::Notification
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: shared::message::NotificationPayload = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        tracing::info!(
            event = "notification",
            title = %payload.title,
            message = %payload.message,
            level = %payload.level,
            "Processing notification"
        );

        // TODO: Notification logic
        // - Log to database
        // - Push notification
        // - Email/SMS

        Ok(ProcessResult::Success {
            message: format!("Notification processed: {}", payload.title),
        })
    }
}

/// Server command processor - handles commands from upstream/central server
///
/// 处理上层服务器发来的指令（配置更新、数据同步指令、远程控制等）
pub struct ServerCommandProcessor {
    state: Arc<crate::server::ServerState>,
}

impl ServerCommandProcessor {
    pub fn new(state: Arc<crate::server::ServerState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MessageProcessor for ServerCommandProcessor {
    fn event_type(&self) -> EventType {
        EventType::ServerCommand
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: shared::message::ServerCommandPayload = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        match payload.command {
            shared::message::ServerCommand::Activate {
                tenant_id,
                tenant_name,
                edge_id,
                edge_name,
                tenant_ca_pem,
                edge_cert_pem,
                edge_key_pem,
            } => {
                tracing::info!(
                    "🚀 Received Activate command: tenant={}, edge={}",
                    tenant_name,
                    edge_name
                );

                // Save certificates to filesystem
                if let Err(e) = self
                    .state
                    .save_certificates(&tenant_ca_pem, &edge_cert_pem, &edge_key_pem)
                    .await
                {
                    tracing::error!("Failed to save certificates: {}", e);
                    return Ok(ProcessResult::Failed {
                        reason: format!("Failed to save certificates: {}", e),
                    });
                }

                // Calculate certificate fingerprint and extract device_id using crab-cert
                let (cert_fingerprint, cert_device_id) =
                    match crab_cert::CertMetadata::from_pem(&edge_cert_pem) {
                        Ok(meta) => (meta.fingerprint_sha256, meta.device_id),
                        Err(e) => {
                            tracing::error!("Failed to parse certificate: {}", e);
                            return Ok(ProcessResult::Failed {
                                reason: format!("Failed to parse certificate: {}", e),
                            });
                        }
                    };

                // Generate local hardware ID
                let local_device_id = crab_cert::generate_hardware_id();

                // Verify device ID if present in cert
                if let Some(ref cert_id) = cert_device_id {
                    if cert_id != &local_device_id {
                        tracing::error!(
                            "Device ID mismatch! Cert: {}, Local: {}",
                            cert_id,
                            local_device_id
                        );
                        return Ok(ProcessResult::Failed {
                            reason: format!(
                                "Device ID mismatch: Cert={}, Local={}",
                                cert_id, local_device_id
                            ),
                        });
                    }
                } else {
                    tracing::warn!(
                        "Certificate missing Device ID. Proceeding with local ID: {}",
                        local_device_id
                    );
                }

                let device_id = cert_device_id.unwrap_or(local_device_id);

                // Update activation state in database
                match self
                    .state
                    .activate_with_metadata(
                        &tenant_id,
                        &tenant_name,
                        &edge_id,
                        &edge_name,
                        &device_id,
                        &cert_fingerprint,
                    )
                    .await
                {
                    Ok(_) => Ok(ProcessResult::Success {
                        message: format!(
                            "Server activated: tenant={}, edge={}",
                            tenant_name, edge_name
                        ),
                    }),
                    Err(e) => {
                        tracing::error!("Activation failed: {}", e);
                        Ok(ProcessResult::Failed {
                            reason: format!("Activation failed: {}", e),
                        })
                    }
                }
            }
            shared::message::ServerCommand::Ping => Ok(ProcessResult::Success {
                message: "Pong".to_string(),
            }),
            shared::message::ServerCommand::ConfigUpdate { key, value } => {
                tracing::info!("Received ConfigUpdate: {} = {:?}", key, value);
                // TODO: Implement config update logic
                Ok(ProcessResult::Skipped {
                    reason: "Config update not implemented yet".to_string(),
                })
            }
            shared::message::ServerCommand::Restart {
                delay_seconds,
                reason,
            } => {
                tracing::warn!(
                    "Received Restart command: delay={}s, reason={:?}",
                    delay_seconds,
                    reason
                );
                // TODO: Implement graceful restart
                Ok(ProcessResult::Skipped {
                    reason: "Restart not implemented yet".to_string(),
                })
            }
        }
    }
}
