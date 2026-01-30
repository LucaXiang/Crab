//! 打印机相关命令
//!
//! 提供打印机列表、钱箱控制、收据打印等功能

use crate::api::printers::{LabelData, ReceiptData};
use crate::core::response::{ApiResponse, ErrorCode};
use crate::utils::printing;

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

/// 打印标签
#[tauri::command]
pub fn print_label(
    printer_name: Option<String>,
    label: LabelData,
    template_id: Option<String>,
) -> Result<ApiResponse<()>, String> {
    // TODO: 实现标签打印逻辑
    // 目前返回未实现错误
    let _ = (printer_name, label, template_id);
    Ok(ApiResponse::error_with_code(
        ErrorCode::LabelPrintingNotImplemented,
        ErrorCode::LabelPrintingNotImplemented.message().to_string(),
    ))
}
