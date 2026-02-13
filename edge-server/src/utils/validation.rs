//! Input validation helpers
//!
//! Centralized text length constants and validation functions.
//! Limits are chosen based on:
//! - ESC/POS 80mm printer line width: 48 chars
//! - Reasonable UX limits for names, notes, descriptions
//! - SQLite TEXT has no built-in length enforcement

use crate::utils::AppError;

// ── Text length limits ──────────────────────────────────────────────

/// Entity names: product, category, attribute, zone, table, tag, role, etc.
pub const MAX_NAME_LEN: usize = 200;

/// Receipt / kitchen print names (80mm = 48 chars, but allow some overflow for wrapping)
pub const MAX_RECEIPT_NAME_LEN: usize = 64;

/// Notes, descriptions, reasons (void note, comp reason, order note, etc.)
pub const MAX_NOTE_LEN: usize = 500;

/// Short identifiers: phone, card_number, NIF, color codes, etc.
pub const MAX_SHORT_TEXT_LEN: usize = 100;

/// Email addresses (RFC 5321)
pub const MAX_EMAIL_LEN: usize = 254;

/// Passwords (before hashing)
pub const MAX_PASSWORD_LEN: usize = 128;

/// URLs / image paths
pub const MAX_URL_LEN: usize = 2048;

/// Addresses
pub const MAX_ADDRESS_LEN: usize = 500;

// ── Validation helpers (CRUD handlers) ──────────────────────────────

/// Validate that a required string is non-empty and within the length limit.
pub fn validate_required_text(value: &str, field: &str, max_len: usize) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::validation(format!("{field} must not be empty")));
    }
    if value.len() > max_len {
        return Err(AppError::validation(format!(
            "{field} is too long ({} chars, max {max_len})",
            value.len()
        )));
    }
    Ok(())
}

/// Validate that an optional string, if present, is within the length limit.
pub fn validate_optional_text(
    value: &Option<String>,
    field: &str,
    max_len: usize,
) -> Result<(), AppError> {
    if let Some(v) = value
        && v.len() > max_len
    {
        return Err(AppError::validation(format!(
            "{field} is too long ({} chars, max {max_len})",
            v.len()
        )));
    }
    Ok(())
}

// ── Validation helpers (Order actions) ──────────────────────────────

use shared::order::types::CommandErrorCode;

use crate::orders::traits::OrderError;

/// Validate a required string for order actions (non-empty + max length).
pub fn validate_order_text(
    value: &str,
    field: &str,
    max_len: usize,
) -> Result<(), OrderError> {
    if value.len() > max_len {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidOperation,
            format!("{field} is too long ({} chars, max {max_len})", value.len()),
        ));
    }
    Ok(())
}

/// Validate an optional string for order actions (max length).
pub fn validate_order_optional_text(
    value: &Option<String>,
    field: &str,
    max_len: usize,
) -> Result<(), OrderError> {
    if let Some(v) = value
        && v.len() > max_len
    {
        return Err(OrderError::InvalidOperation(
            CommandErrorCode::InvalidOperation,
            format!("{field} is too long ({} chars, max {max_len})", v.len()),
        ));
    }
    Ok(())
}
