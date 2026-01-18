use tracing::instrument;

pub trait PrinterAdapter {
    fn list_printers(&self) -> Result<Vec<String>, String>;
    fn resolve_printer(&self, printer_name: Option<String>) -> Result<String, String>;
    fn print_raw_bytes(&self, printer_name: Option<String>, bytes: Vec<u8>) -> Result<(), String>;
    fn open_cash_drawer(&self, printer_name: Option<String>) -> Result<(), String>;
    fn print_receipt(
        &self,
        printer_name: Option<String>,
        receipt: crate::api::printers::ReceiptData,
    ) -> Result<(), String>;
}

#[cfg(target_os = "windows")]
mod windows_adapter {
    use super::PrinterAdapter;
    use crate::utils::escpos_text::{convert_mixed_utf8_to_gbk, process_logo};
    use crate::utils::receipt_renderer::ReceiptRenderer;
    use core::ffi::c_void;
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;
    use tracing::{error, info, warn};
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::Graphics::Printing::{
            ClosePrinter, EndDocPrinter, EndPagePrinter, EnumPrintersW, GetDefaultPrinterW,
            GetPrinterW, OpenPrinterW, StartDocPrinterW, StartPagePrinter, WritePrinter,
            DOC_INFO_1W, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_HANDLE,
            PRINTER_INFO_5W, PRINTER_INFO_6, PRINTER_STATUS_OFFLINE,
        },
    };

    pub struct WindowsPrinterAdapter;

    impl WindowsPrinterAdapter {
        fn to_wide_null(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        fn is_printer_online(name: &str) -> Result<bool, String> {
            unsafe {
                let mut handle: PRINTER_HANDLE = PRINTER_HANDLE::default();
                let name_w = Self::to_wide_null(name);
                OpenPrinterW(PCWSTR::from_raw(name_w.as_ptr()), &mut handle, None)
                    .map_err(|_| "OpenPrinterW failed".to_string())?;
                let mut needed6: u32 = 0;
                let _ = GetPrinterW(handle, 6, None, &mut needed6);
                if needed6 == 0 {
                    let _ = ClosePrinter(handle);
                    return Ok(true);
                }
                let mut buf6: Vec<u8> = vec![0; needed6 as usize];
                GetPrinterW(handle, 6, Some(buf6.as_mut_slice()), &mut needed6)
                    .map_err(|_| "GetPrinterW failed".to_string())?;
                let info6 = *(buf6.as_ptr() as *const PRINTER_INFO_6);
                let status = info6.dwStatus;

                let mut needed5: u32 = 0;
                let _ = GetPrinterW(handle, 5, None, &mut needed5);
                if needed5 == 0 {
                    let _ = ClosePrinter(handle);
                    return Ok(true);
                }
                let mut buf5: Vec<u8> = vec![0; needed5 as usize];
                GetPrinterW(handle, 5, Some(buf5.as_mut_slice()), &mut needed5)
                    .map_err(|_| "GetPrinterW level5 failed".to_string())?;
                let info5 = *(buf5.as_ptr() as *const PRINTER_INFO_5W);
                let _ = ClosePrinter(handle);

                if (status & PRINTER_STATUS_OFFLINE) != 0 {
                    return Ok(false);
                }

                let port = if info5.pPortName.is_null() {
                    String::new()
                } else {
                    PWSTR(info5.pPortName.0).to_string().unwrap_or_default()
                };
                let lower = port.to_lowercase();
                if lower.starts_with("ip_") {
                    let host = lower.trim_start_matches("ip_");
                    let host = host.split(',').next().unwrap_or(host);
                    let timeout = Duration::from_millis(400);
                    if let Ok(mut iter) = format!("{}:{}", host, 9100).to_socket_addrs() {
                        if let Some(addr) = iter.next() {
                            match TcpStream::connect_timeout(&addr, timeout) {
                                Ok(_) => Ok(true),
                                Err(_) => Ok(false),
                            }
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(true)
                }
            }
        }

        fn write_raw(&self, printer_name: &str, data: &[u8]) -> Result<(), String> {
            unsafe {
                match Self::is_printer_online(printer_name) {
                    Ok(true) => {}
                    Ok(false) => return Err("PRINTER_OFFLINE".to_string()),
                    Err(e) => warn!(error = e, "status check failed, continue to write"),
                }

                let mut printer_handle: PRINTER_HANDLE = PRINTER_HANDLE::default();
                let name_w = Self::to_wide_null(printer_name);
                OpenPrinterW(PCWSTR::from_raw(name_w.as_ptr()), &mut printer_handle, None)
                    .map_err(|_| "Failed to open printer".to_string())?;

                let doc_name_w = Self::to_wide_null("Raw Document");
                let datatype_w = Self::to_wide_null("RAW");
                let document_info = DOC_INFO_1W {
                    pDocName: PWSTR(doc_name_w.as_ptr() as *mut _),
                    pOutputFile: PWSTR::null(),
                    pDatatype: PWSTR(datatype_w.as_ptr() as *mut _),
                };

                if StartDocPrinterW(printer_handle, 1, &document_info as *const DOC_INFO_1W) == 0 {
                    let _ = ClosePrinter(printer_handle);
                    return Err("Failed to start doc".to_string());
                }

                if StartPagePrinter(printer_handle) == false {
                    let _ = EndDocPrinter(printer_handle);
                    let _ = ClosePrinter(printer_handle);
                    return Err("Failed to start page".to_string());
                }

                let mut written: u32 = 0;
                let ok = WritePrinter(
                    printer_handle,
                    data.as_ptr() as *const c_void,
                    data.len() as u32,
                    &mut written,
                );

                let _ = EndPagePrinter(printer_handle);
                let _ = EndDocPrinter(printer_handle);
                let _ = ClosePrinter(printer_handle);

                if ok == false {
                    return Err("Failed to write to printer".to_string());
                }
                if written != data.len() as u32 {
                    return Err("Failed to write all bytes".to_string());
                }
                Ok(())
            }
        }
    }

    impl PrinterAdapter for WindowsPrinterAdapter {
        fn list_printers(&self) -> Result<Vec<String>, String> {
            unsafe {
                let flags: u32 = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
                let mut needed: u32 = 0;
                let mut returned: u32 = 0;
                let _ = EnumPrintersW(flags, None, 5, None, &mut needed, &mut returned);
                if needed == 0 {
                    return Ok(Vec::new());
                }
                let mut buf: Vec<u8> = vec![0; needed as usize];
                EnumPrintersW(
                    flags,
                    None,
                    5,
                    Some(buf.as_mut_slice()),
                    &mut needed,
                    &mut returned,
                )
                .map_err(|_| "EnumPrintersW failed".to_string())?;
                let ptr = buf.as_ptr() as *const PRINTER_INFO_5W;
                let slice = std::slice::from_raw_parts(ptr, returned as usize);
                fn is_virtual_port(port: &str) -> bool {
                    let p = port.to_lowercase();
                    p == "file:"
                        || p == "portprompt:"
                        || p == "xpsport:"
                        || p.starts_with("onenote")
                        || p == "nul:"
                        || p.starts_with("wfsport:")
                }
                let mut result: Vec<String> = Vec::new();
                for info in slice.iter() {
                    if info.pPrinterName.is_null() {
                        continue;
                    }
                    let name = PWSTR(info.pPrinterName.0).to_string().unwrap_or_default();
                    let port = if info.pPortName.is_null() {
                        String::new()
                    } else {
                        PWSTR(info.pPortName.0).to_string().unwrap_or_default()
                    };
                    if !is_virtual_port(&port) {
                        result.push(name);
                    }
                }
                info!(count = result.len(), "printers listed (port filtered)");
                Ok(result)
            }
        }

        fn resolve_printer(&self, printer_name: Option<String>) -> Result<String, String> {
            if let Some(name) = printer_name {
                let printers = self.list_printers()?;
                if printers.iter().any(|p| p == &name) {
                    Ok(name)
                } else {
                    error!(requested = name, "printer not found");
                    Err("printer not found".to_string())
                }
            } else {
                unsafe {
                    let mut needed: u32 = 0;
                    let _ = GetDefaultPrinterW(None, &mut needed);

                    if needed > 0 {
                        let mut buf: Vec<u16> = vec![0; needed as usize];
                        let ok = GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut needed);
                        if ok != false {
                            let name = PWSTR(buf.as_mut_ptr())
                                .to_string()
                                .map_err(|e| format!("{}", e))?;
                            return Ok(name);
                        }
                    }
                }

                let printers = self.list_printers()?;
                let first = printers
                    .first()
                    .ok_or_else(|| "no printers found".to_string())?;
                Ok(first.clone())
            }
        }

        fn print_raw_bytes(
            &self,
            printer_name: Option<String>,
            bytes: Vec<u8>,
        ) -> Result<(), String> {
            let name = self.resolve_printer(printer_name)?;
            info!(printer = name, "printing raw bytes");
            let final_bytes = convert_mixed_utf8_to_gbk(&bytes);
            self.write_raw(&name, &final_bytes)
        }

        fn open_cash_drawer(&self, printer_name: Option<String>) -> Result<(), String> {
            let name = self.resolve_printer(printer_name.clone())?;
            info!(printer = name, "open cash drawer");
            let cmd_pin2: [u8; 5] = [0x1B, b'p', 0, 25, 250];
            if let Err(_e) = self.write_raw(&name, &cmd_pin2) {
                warn!("kick pin2 failed, trying pin5");
                let name2 = self.resolve_printer(printer_name)?;
                let cmd_pin5: [u8; 5] = [0x1B, b'p', 1, 25, 250];
                self.write_raw(&name2, &cmd_pin5).map_err(|e| {
                    error!(error = format!("{}", e), "cash drawer pin5 write failed");
                    format!("{}", e)
                })?;
            }
            Ok(())
        }

        fn print_receipt(
            &self,
            printer_name: Option<String>,
            receipt: crate::api::printers::ReceiptData,
        ) -> Result<(), String> {
            let name = self.resolve_printer(printer_name.clone())?;
            info!(printer = name, "printing receipt");

            let mut data: Vec<u8> = vec![0x1B, 0x40];

            info!("checking store info/logo");
            if let Some(info) = &receipt.store_info {
                if let Some(logo_path) = &info.logo_url {
                    info!(logo = logo_path, "logo url found");
                    if !logo_path.is_empty() {
                        if let Some(logo_bytes) = process_logo(logo_path) {
                            info!(bytes = logo_bytes.len(), "sending logo data");
                            data.extend_from_slice(&logo_bytes);
                            info!("logo prepared");
                        } else {
                            warn!("process_logo returned None");
                        }
                    } else {
                        warn!("logo path empty");
                    }
                } else {
                    warn!("no logo_url in store_info");
                }
            } else {
                warn!("no store_info provided");
            }

            let output = ReceiptRenderer::new(&receipt, 48).render();
            let text_bytes = convert_mixed_utf8_to_gbk(output.as_bytes());
            data.extend_from_slice(&text_bytes);
            self.write_raw(&name, &data)
        }
    }

    pub fn current_adapter() -> WindowsPrinterAdapter {
        WindowsPrinterAdapter
    }
}

#[cfg(not(target_os = "windows"))]
mod fallback_adapter {
    use super::PrinterAdapter;

    pub struct UnsupportedPrinterAdapter;

    impl PrinterAdapter for UnsupportedPrinterAdapter {
        fn list_printers(&self) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }

        fn resolve_printer(&self, printer_name: Option<String>) -> Result<String, String> {
            if let Some(name) = printer_name {
                Ok(name)
            } else {
                Err("PRINTING_NOT_SUPPORTED".to_string())
            }
        }

        fn print_raw_bytes(
            &self,
            _printer_name: Option<String>,
            _bytes: Vec<u8>,
        ) -> Result<(), String> {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }

        fn open_cash_drawer(&self, _printer_name: Option<String>) -> Result<(), String> {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }

        fn print_receipt(
            &self,
            _printer_name: Option<String>,
            _receipt: crate::api::printers::ReceiptData,
        ) -> Result<(), String> {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }
    }

    pub fn current_adapter() -> UnsupportedPrinterAdapter {
        UnsupportedPrinterAdapter
    }
}

#[cfg(not(target_os = "windows"))]
use fallback_adapter::current_adapter;
#[cfg(target_os = "windows")]
use windows_adapter::current_adapter;

#[instrument]
pub fn list_printers() -> Result<Vec<String>, String> {
    current_adapter().list_printers()
}

#[instrument(skip(printer_name))]
pub fn resolve_printer(printer_name: Option<String>) -> Result<String, String> {
    current_adapter().resolve_printer(printer_name)
}

#[instrument(skip(bytes))]
pub fn print_raw_bytes(printer_name: Option<String>, bytes: Vec<u8>) -> Result<(), String> {
    current_adapter().print_raw_bytes(printer_name, bytes)
}

#[instrument(skip(printer_name))]
pub fn open_cash_drawer(printer_name: Option<String>) -> Result<(), String> {
    current_adapter().open_cash_drawer(printer_name)
}

#[instrument(skip(receipt))]
pub fn print_receipt(
    printer_name: Option<String>,
    receipt: crate::api::printers::ReceiptData,
) -> Result<(), String> {
    current_adapter().print_receipt(printer_name, receipt)
}
