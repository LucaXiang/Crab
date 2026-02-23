//! Printer adapters for sending ESC/POS data
//!
//! Supports:
//! - Network printers (TCP port 9100)
//! - Windows driver printers (via Win32 API)

use crate::error::{PrintError, PrintResult};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{info, instrument, warn};

/// Trait for printer adapters
#[allow(async_fn_in_trait)]
pub trait Printer {
    /// Send raw ESC/POS data to the printer
    async fn print(&self, data: &[u8]) -> PrintResult<()>;

    /// Check if the printer is online/reachable
    async fn is_online(&self) -> bool;
}

/// Network printer (TCP port 9100)
///
/// Most thermal printers support raw TCP printing on port 9100.
#[derive(Debug, Clone)]
pub struct NetworkPrinter {
    addr: SocketAddr,
    timeout: Duration,
}

impl NetworkPrinter {
    /// Create a new network printer
    ///
    /// Default port is 9100 if not specified in address.
    pub fn new(host: &str, port: u16) -> PrintResult<Self> {
        let addr_str = format!("{}:{}", host, port);
        let addr = addr_str
            .parse()
            .map_err(|_| PrintError::InvalidConfig(format!("Invalid address: {}", addr_str)))?;

        Ok(Self {
            addr,
            timeout: Duration::from_secs(5),
        })
    }

    /// Create from a socket address string (e.g., "192.168.1.100:9100")
    pub fn from_addr(addr: &str) -> PrintResult<Self> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|_| PrintError::InvalidConfig(format!("Invalid address: {}", addr)))?;

        Ok(Self {
            addr,
            timeout: Duration::from_secs(5),
        })
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get the printer address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Printer for NetworkPrinter {
    #[instrument(skip(data), fields(addr = %self.addr, data_len = data.len()))]
    async fn print(&self, data: &[u8]) -> PrintResult<()> {
        info!("Connecting to printer");

        let stream = tokio::time::timeout(self.timeout, TcpStream::connect(self.addr))
            .await
            .map_err(|_| PrintError::Timeout(format!("Connection timeout: {}", self.addr)))?
            .map_err(|e| PrintError::Connection(format!("{}: {}", self.addr, e)))?;

        info!("Connected, sending {} bytes", data.len());

        let mut stream = stream;
        stream.write_all(data).await.map_err(|e| {
            PrintError::Io(std::io::Error::new(
                e.kind(),
                format!("Write failed: {}", e),
            ))
        })?;

        stream.flush().await?;

        info!("Print job sent successfully");
        Ok(())
    }

    #[instrument(fields(addr = %self.addr))]
    async fn is_online(&self) -> bool {
        let check_timeout = Duration::from_millis(500);

        match tokio::time::timeout(check_timeout, TcpStream::connect(self.addr)).await {
            Ok(Ok(_)) => {
                info!("Printer online");
                true
            }
            Ok(Err(e)) => {
                warn!(error = %e, "Printer offline");
                false
            }
            Err(_) => {
                warn!("Printer check timeout");
                false
            }
        }
    }
}

/// Windows driver printer
///
/// Uses Win32 API to print through installed printer drivers.
#[cfg(windows)]
pub struct WindowsPrinter {
    name: String,
}

#[cfg(windows)]
impl WindowsPrinter {
    /// Create a printer with a specific name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Get the printer name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// List available printers (filters out virtual printers)
    pub fn list() -> PrintResult<Vec<String>> {
        use windows::Win32::Graphics::Printing::{
            EnumPrintersW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_5W,
        };
        use windows::core::PWSTR;

        unsafe {
            let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
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
            .map_err(|_| PrintError::WindowsPrinter("EnumPrintersW failed".to_string()))?;

            let ptr = buf.as_ptr() as *const PRINTER_INFO_5W;
            let slice = std::slice::from_raw_parts(ptr, returned as usize);

            let mut result: Vec<String> = Vec::new();
            for info in slice.iter() {
                if info.pPrinterName.is_null() {
                    continue;
                }
                let name = PWSTR(info.pPrinterName.0).to_string().unwrap_or_default();

                // Filter out virtual printers by port name
                let port = if info.pPortName.is_null() {
                    String::new()
                } else {
                    PWSTR(info.pPortName.0).to_string().unwrap_or_default()
                };

                if !Self::is_virtual_port(&port) {
                    result.push(name);
                }
            }

            Ok(result)
        }
    }

    /// Check if a port is a virtual printer port
    fn is_virtual_port(port: &str) -> bool {
        let p = port.to_lowercase();
        p == "file:"
            || p == "portprompt:"
            || p == "xpsport:"
            || p.starts_with("onenote")
            || p == "nul:"
            || p.starts_with("wfsport:")
    }

    /// Get the default printer name
    pub fn default_printer() -> PrintResult<Option<String>> {
        use windows::Win32::Graphics::Printing::GetDefaultPrinterW;
        use windows::core::PWSTR;

        unsafe {
            let mut needed: u32 = 0;
            let _ = GetDefaultPrinterW(None, &mut needed);

            if needed == 0 {
                return Ok(None);
            }

            let mut buf: Vec<u16> = vec![0; needed as usize];
            let ok = GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut needed);

            if !ok.as_bool() {
                return Ok(None);
            }

            let name = PWSTR(buf.as_mut_ptr())
                .to_string()
                .map_err(|e| PrintError::WindowsPrinter(format!("UTF-16 decode failed: {}", e)))?;

            Ok(Some(name))
        }
    }

    /// Resolve a printer name - returns the name if valid, or default/first available
    pub fn resolve(name: Option<&str>) -> PrintResult<String> {
        if let Some(name) = name {
            // Verify the printer exists
            let printers = Self::list()?;
            if printers.iter().any(|p| p == name) {
                return Ok(name.to_string());
            }
            return Err(PrintError::WindowsPrinter(format!(
                "Printer not found: {}",
                name
            )));
        }

        // Try default printer first
        if let Some(default) = Self::default_printer()? {
            return Ok(default);
        }

        // Fall back to first available
        let printers = Self::list()?;
        printers
            .first()
            .cloned()
            .ok_or_else(|| PrintError::WindowsPrinter("No printers available".to_string()))
    }

    /// Check if printer is online (includes network port detection for IP printers)
    pub fn check_online(name: &str) -> PrintResult<bool> {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;
        use windows::Win32::Graphics::Printing::{
            ClosePrinter, GetPrinterW, OpenPrinterW, PRINTER_HANDLE, PRINTER_INFO_5W,
            PRINTER_INFO_6, PRINTER_STATUS_OFFLINE,
        };
        use windows::core::{PCWSTR, PWSTR};

        fn to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        unsafe {
            let mut handle: PRINTER_HANDLE = PRINTER_HANDLE::default();
            let name_w = to_wide(name);

            OpenPrinterW(PCWSTR::from_raw(name_w.as_ptr()), &mut handle, None)
                .map_err(|_| PrintError::WindowsPrinter("OpenPrinterW failed".to_string()))?;

            // Get PRINTER_INFO_6 for status
            let mut needed6: u32 = 0;
            let _ = GetPrinterW(handle, 6, None, &mut needed6);

            if needed6 > 0 {
                let mut buf6: Vec<u8> = vec![0; needed6 as usize];
                if GetPrinterW(handle, 6, Some(buf6.as_mut_slice()), &mut needed6).is_ok() {
                    let info6 = *(buf6.as_ptr() as *const PRINTER_INFO_6);
                    if (info6.dwStatus & PRINTER_STATUS_OFFLINE) != 0 {
                        let _ = ClosePrinter(handle);
                        return Ok(false);
                    }
                }
            }

            // Get PRINTER_INFO_5 for port name
            let mut needed5: u32 = 0;
            let _ = GetPrinterW(handle, 5, None, &mut needed5);

            let port = if needed5 > 0 {
                let mut buf5: Vec<u8> = vec![0; needed5 as usize];
                if GetPrinterW(handle, 5, Some(buf5.as_mut_slice()), &mut needed5).is_ok() {
                    let info5 = *(buf5.as_ptr() as *const PRINTER_INFO_5W);
                    if !info5.pPortName.is_null() {
                        PWSTR(info5.pPortName.0).to_string().unwrap_or_default()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let _ = ClosePrinter(handle);

            // For IP-based printers (IP_xxx port), try TCP connection
            let lower = port.to_lowercase();
            if lower.starts_with("ip_") {
                let host = lower.trim_start_matches("ip_");
                let host = host.split(',').next().unwrap_or(host);

                let timeout = Duration::from_millis(400);
                if let Ok(mut iter) = format!("{}:9100", host).to_socket_addrs()
                    && let Some(addr) = iter.next()
                {
                    return Ok(TcpStream::connect_timeout(&addr, timeout).is_ok());
                }
                return Ok(false);
            }

            // For non-IP printers, assume online if not marked offline
            Ok(true)
        }
    }

    /// Send raw ESC/POS data synchronously (for use from sync/blocking contexts)
    pub fn print_sync(&self, data: &[u8]) -> PrintResult<()> {
        self.write_raw(data)
    }

    /// Send ESC/POS command to open cash drawer
    ///
    /// Tries pin 2 first, then pin 5 if that fails.
    pub fn open_cash_drawer(&self) -> PrintResult<()> {
        // ESC p m t1 t2 - Generate pulse at connector pin
        // Pin 2: m=0, Pin 5: m=1
        let cmd_pin2: [u8; 5] = [0x1B, b'p', 0, 25, 250];

        if self.write_raw(&cmd_pin2).is_ok() {
            return Ok(());
        }

        // Try pin 5 if pin 2 failed
        warn!("Cash drawer pin 2 failed, trying pin 5");
        let cmd_pin5: [u8; 5] = [0x1B, b'p', 1, 25, 250];
        self.write_raw(&cmd_pin5)
    }

    fn write_raw(&self, data: &[u8]) -> PrintResult<()> {
        use core::ffi::c_void;
        use windows::Win32::Graphics::Printing::{
            ClosePrinter, DOC_INFO_1W, EndDocPrinter, EndPagePrinter, OpenPrinterW, PRINTER_HANDLE,
            StartDocPrinterW, StartPagePrinter, WritePrinter,
        };
        use windows::core::{PCWSTR, PWSTR};

        fn to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        unsafe {
            // Check if printer is online first
            if !Self::check_online(&self.name).unwrap_or(true) {
                return Err(PrintError::Offline(self.name.clone()));
            }

            let mut handle: PRINTER_HANDLE = PRINTER_HANDLE::default();
            let name_w = to_wide(&self.name);

            OpenPrinterW(PCWSTR::from_raw(name_w.as_ptr()), &mut handle, None)
                .map_err(|_| PrintError::WindowsPrinter("OpenPrinterW failed".to_string()))?;

            let doc_name_w = to_wide("Raw Document");
            let datatype_w = to_wide("RAW");
            let doc_info = DOC_INFO_1W {
                pDocName: PWSTR(doc_name_w.as_ptr() as *mut _),
                pOutputFile: PWSTR::null(),
                pDatatype: PWSTR(datatype_w.as_ptr() as *mut _),
            };

            if StartDocPrinterW(handle, 1, &doc_info as *const DOC_INFO_1W) == 0 {
                let _ = ClosePrinter(handle);
                return Err(PrintError::WindowsPrinter(
                    "StartDocPrinter failed".to_string(),
                ));
            }

            if !StartPagePrinter(handle).as_bool() {
                let _ = EndDocPrinter(handle);
                let _ = ClosePrinter(handle);
                return Err(PrintError::WindowsPrinter(
                    "StartPagePrinter failed".to_string(),
                ));
            }

            let mut written: u32 = 0;
            let ok = WritePrinter(
                handle,
                data.as_ptr() as *const c_void,
                data.len() as u32,
                &mut written,
            );

            let _ = EndPagePrinter(handle);
            let _ = EndDocPrinter(handle);
            let _ = ClosePrinter(handle);

            if !ok.as_bool() {
                return Err(PrintError::WindowsPrinter(
                    "WritePrinter failed".to_string(),
                ));
            }

            if written != data.len() as u32 {
                return Err(PrintError::WindowsPrinter("Incomplete write".to_string()));
            }

            Ok(())
        }
    }
}

#[cfg(windows)]
impl Printer for WindowsPrinter {
    async fn print(&self, data: &[u8]) -> PrintResult<()> {
        // Windows printing is synchronous, run in blocking task
        let name = self.name.clone();
        let data = data.to_vec();

        tokio::task::spawn_blocking(move || {
            let printer = WindowsPrinter { name };
            printer.write_raw(&data)
        })
        .await
        .map_err(|e| PrintError::WindowsPrinter(format!("Task join failed: {}", e)))?
    }

    async fn is_online(&self) -> bool {
        Self::check_online(&self.name).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_printer_new() {
        let printer = NetworkPrinter::new("192.168.1.100", 9100).unwrap();
        assert_eq!(printer.addr().port(), 9100);
    }

    #[test]
    fn test_network_printer_from_addr() {
        let printer = NetworkPrinter::from_addr("192.168.1.100:9100").unwrap();
        assert_eq!(printer.addr().port(), 9100);
    }

    #[test]
    fn test_invalid_addr() {
        let result = NetworkPrinter::from_addr("invalid");
        assert!(result.is_err());
    }
}
