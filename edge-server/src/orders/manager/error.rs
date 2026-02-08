use super::super::storage::StorageError;
use super::super::traits::OrderError;
use shared::order::{CommandError, CommandErrorCode};
use thiserror::Error;

/// Manager errors
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Order already completed: {0}")]
    OrderAlreadyCompleted(String),

    #[error("Order already voided: {0}")]
    OrderAlreadyVoided(String),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error("Payment not found: {0}")]
    PaymentNotFound(String),

    #[error("Insufficient quantity")]
    InsufficientQuantity,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Table is already occupied: {0}")]
    TableOccupied(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// 将存储错误转换为错误码（前端负责本地化）
fn classify_storage_error(e: &StorageError) -> CommandErrorCode {
    // 先按枚举变体精确匹配
    match e {
        StorageError::Serialization(_) => return CommandErrorCode::InternalError,
        StorageError::OrderNotFound(_) => return CommandErrorCode::OrderNotFound,
        StorageError::EventNotFound(_, _) => return CommandErrorCode::InternalError,
        _ => {}
    }

    // redb 错误通过字符串匹配分类
    let err_str = e.to_string().to_lowercase();

    // 磁盘空间不足
    if err_str.contains("no space") || err_str.contains("disk full") || err_str.contains("enospc")
    {
        return CommandErrorCode::StorageFull;
    }

    // 内存不足
    if err_str.contains("out of memory") || err_str.contains("cannot allocate") {
        return CommandErrorCode::OutOfMemory;
    }

    // 数据损坏
    if err_str.contains("corrupt") || err_str.contains("invalid database") {
        return CommandErrorCode::StorageCorrupted;
    }

    // 默认：系统繁忙（redb 的 Database/Transaction/Table/Storage/Commit 错误）
    CommandErrorCode::SystemBusy
}

impl From<ManagerError> for CommandError {
    fn from(err: ManagerError) -> Self {
        let (code, message) = match err {
            ManagerError::Storage(e) => {
                let code = classify_storage_error(&e);
                let message = e.to_string(); // 保留技术细节用于日志/调试
                tracing::error!(error = %e, error_code = ?code, "Storage error occurred");
                (code, message)
            }
            ManagerError::OrderNotFound(id) => (
                CommandErrorCode::OrderNotFound,
                format!("Order not found: {}", id),
            ),
            ManagerError::OrderAlreadyCompleted(id) => (
                CommandErrorCode::OrderAlreadyCompleted,
                format!("Order already completed: {}", id),
            ),
            ManagerError::OrderAlreadyVoided(id) => (
                CommandErrorCode::OrderAlreadyVoided,
                format!("Order already voided: {}", id),
            ),
            ManagerError::ItemNotFound(id) => (
                CommandErrorCode::ItemNotFound,
                format!("Item not found: {}", id),
            ),
            ManagerError::PaymentNotFound(id) => (
                CommandErrorCode::PaymentNotFound,
                format!("Payment not found: {}", id),
            ),
            ManagerError::InsufficientQuantity => (
                CommandErrorCode::InsufficientQuantity,
                "Insufficient quantity".to_string(),
            ),
            ManagerError::InvalidAmount => (
                CommandErrorCode::InvalidAmount,
                "Invalid amount".to_string(),
            ),
            ManagerError::InvalidOperation(msg) => (CommandErrorCode::InvalidOperation, msg),
            ManagerError::TableOccupied(msg) => (CommandErrorCode::TableOccupied, msg),
            ManagerError::Internal(msg) => (CommandErrorCode::InternalError, msg),
        };
        CommandError::new(code, message)
    }
}

impl From<OrderError> for ManagerError {
    fn from(err: OrderError) -> Self {
        match err {
            OrderError::OrderNotFound(id) => ManagerError::OrderNotFound(id),
            OrderError::OrderAlreadyCompleted(id) => ManagerError::OrderAlreadyCompleted(id),
            OrderError::OrderAlreadyVoided(id) => ManagerError::OrderAlreadyVoided(id),
            OrderError::ItemNotFound(id) => ManagerError::ItemNotFound(id),
            OrderError::PaymentNotFound(id) => ManagerError::PaymentNotFound(id),
            OrderError::InsufficientQuantity => ManagerError::InsufficientQuantity,
            OrderError::InvalidAmount => ManagerError::InvalidAmount,
            OrderError::InvalidOperation(msg) => ManagerError::InvalidOperation(msg),
            OrderError::TableOccupied(msg) => ManagerError::TableOccupied(msg),
            OrderError::Storage(msg) => ManagerError::Internal(msg),
        }
    }
}

pub type ManagerResult<T> = Result<T, ManagerError>;
