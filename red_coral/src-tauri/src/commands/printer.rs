//! 打印机相关命令
//!
//! 提供打印机列表、钱箱控制、收据打印等功能

use crate::api::printers::ReceiptData;
use crate::core::response::{ApiResponse, ErrorCode};
use crate::utils::printing;
use serde::Deserialize;

/// 获取本地驱动打印机列表
#[tauri::command]
pub fn list_printers() -> Result<ApiResponse<Vec<String>>, String> {
    match printing::list_printers() {
        Ok(printers) => Ok(ApiResponse::success(printers)),
        Err(e) => {
            if e == "PRINTING_NOT_SUPPORTED" {
                // 非 Windows 平台返回空列表
                Ok(ApiResponse::success(Vec::new()))
            } else {
                Ok(ApiResponse::error_with_code(
                    ErrorCode::PrinterNotAvailable,
                    e,
                ))
            }
        }
    }
}

/// 打开钱箱
#[tauri::command]
pub fn open_cash_drawer(printer_name: Option<String>) -> Result<ApiResponse<()>, String> {
    match printing::open_cash_drawer(printer_name) {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => {
            if e == "PRINTING_NOT_SUPPORTED" {
                Ok(ApiResponse::error_with_code(
                    ErrorCode::PrinterNotAvailable,
                    ErrorCode::PrinterNotAvailable.message().to_string(),
                ))
            } else {
                Ok(ApiResponse::error_with_code(ErrorCode::PrintFailed, e))
            }
        }
    }
}

/// 打印收据
#[tauri::command]
pub fn print_receipt(
    printer_name: Option<String>,
    receipt: ReceiptData,
) -> Result<ApiResponse<()>, String> {
    tracing::debug!(
        printer = ?printer_name,
        order_id = %receipt.order_id,
        items = receipt.items.len(),
        "print_receipt: entry"
    );
    match printing::print_receipt(printer_name, receipt) {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => {
            if e == "PRINTING_NOT_SUPPORTED" {
                Ok(ApiResponse::error_with_code(
                    ErrorCode::PrinterNotAvailable,
                    ErrorCode::PrinterNotAvailable.message().to_string(),
                ))
            } else {
                Ok(ApiResponse::error_with_code(ErrorCode::PrintFailed, e))
            }
        }
    }
}

/// 标签打印请求参数
#[derive(Debug, Deserialize)]
pub struct LabelPrintRequest {
    pub printer_name: Option<String>,
    pub data: serde_json::Value,
    pub template: Option<serde_json::Value>,
    pub label_width_mm: Option<f32>,
    pub label_height_mm: Option<f32>,
    pub override_dpi: Option<f32>,
}

/// 打印标签
#[tauri::command]
pub fn print_label(request: LabelPrintRequest) -> Result<ApiResponse<()>, String> {
    tracing::debug!(
        printer = ?request.printer_name,
        has_template = request.template.is_some(),
        label_w = ?request.label_width_mm,
        label_h = ?request.label_height_mm,
        "print_label: entry"
    );
    match printing::print_label(request) {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => {
            if e == "PRINTING_NOT_SUPPORTED" {
                Ok(ApiResponse::error_with_code(
                    ErrorCode::PrinterNotAvailable,
                    ErrorCode::PrinterNotAvailable.message().to_string(),
                ))
            } else {
                Ok(ApiResponse::error_with_code(ErrorCode::PrintFailed, e))
            }
        }
    }
}
