//! GBK encoding utilities for Chinese thermal printers
//!
//! Most Chinese thermal printers use GBK encoding for text.
//! This module provides utilities for:
//! - Calculating GBK string widths
//! - Truncating/padding strings to GBK widths
//! - Converting UTF-8 to GBK while preserving ESC/POS commands

use tracing::instrument;

/// Get the GBK byte width of a string
///
/// Chinese characters are typically 2 bytes in GBK, ASCII is 1 byte.
pub fn gbk_width(s: &str) -> usize {
    let (cow, _, _) = encoding_rs::GBK.encode(s);
    cow.len()
}

/// Truncate a string to fit within a GBK byte width
pub fn truncate_gbk(s: &str, max_width: usize) -> String {
    let mut width = 0;
    let mut result = String::new();
    for c in s.chars() {
        let s_char = c.to_string();
        let (cow, _, _) = encoding_rs::GBK.encode(&s_char);
        let char_len = cow.len();

        if width + char_len > max_width {
            break;
        }
        result.push(c);
        width += char_len;
    }
    result
}

/// Pad a string to a specific GBK byte width
///
/// If the string is longer than the width, it will be truncated.
pub fn pad_gbk(s: &str, width: usize, align_right: bool) -> String {
    let current_width = gbk_width(s);
    if current_width >= width {
        return truncate_gbk(s, width);
    }
    let spaces = width - current_width;
    if align_right {
        format!("{}{}", " ".repeat(spaces), s)
    } else {
        format!("{}{}", s, " ".repeat(spaces))
    }
}

/// Convert mixed UTF-8 content (with ESC/POS commands) to GBK
///
/// This function preserves ASCII bytes (0x00-0x7F) exactly as is,
/// which protects ESC/POS commands from being corrupted.
/// Only bytes >= 0x80 are treated as UTF-8 sequences and converted to GBK.
///
/// Also handles:
/// - Re-enabling Chinese mode after INIT command (ESC @)
/// - Euro symbol (€) special handling
#[instrument(skip(bytes))]
pub fn convert_to_gbk(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len() * 2);

    // Enable Chinese mode at the start
    // FS & (0x1C 0x26) - Enable Chinese mode
    // FS C 1 (0x1C 0x43 0x01) - Select GBK code page
    result.extend_from_slice(&[0x1C, 0x26, 0x1C, 0x43, 0x01]);

    let mut buffer = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Check for INIT command (ESC @ = 0x1B 0x40)
        // If we see INIT, we must re-enable Chinese mode after it
        if b == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == 0x40 {
            // Flush pending non-ASCII buffer
            flush_buffer(&mut buffer, &mut result);

            // Write INIT
            result.push(0x1B);
            result.push(0x40);

            // Re-enable Chinese mode
            result.extend_from_slice(&[0x1C, 0x26]);

            i += 2;
            continue;
        }

        if b < 128 {
            // ASCII byte (Command or ASCII text)
            flush_buffer(&mut buffer, &mut result);
            result.push(b);
        } else {
            // Non-ASCII byte (Part of UTF-8 Chinese char)
            buffer.push(b);
        }
        i += 1;
    }

    // Flush remaining buffer
    flush_buffer(&mut buffer, &mut result);

    // Exit Chinese mode at the end
    // FS . (0x1C 0x2E)
    result.extend_from_slice(&[0x1C, 0x2E]);

    result
}

/// Flush the non-ASCII buffer, converting UTF-8 to GBK
fn flush_buffer(buffer: &mut Vec<u8>, result: &mut Vec<u8>) {
    if buffer.is_empty() {
        return;
    }

    let s = String::from_utf8_lossy(buffer);
    let parts: Vec<&str> = s.split('€').collect();

    for (idx, part) in parts.iter().enumerate() {
        if !part.is_empty() {
            let (gbk, _, _) = encoding_rs::GBK.encode(part);
            result.extend_from_slice(&gbk);
        }
        if idx < parts.len() - 1 {
            // Inject Euro Sequence: Exit Chinese -> PC858 -> Euro -> Enter Chinese
            result.extend_from_slice(&[0x1C, 0x2E, 0x1B, 0x74, 19, 0xD5, 0x1C, 0x26]);
        }
    }
    buffer.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbk_width() {
        assert_eq!(gbk_width("hello"), 5);
        assert_eq!(gbk_width("你好"), 4); // 2 Chinese chars = 4 bytes
        assert_eq!(gbk_width("AB中文CD"), 8); // 4 ASCII + 2 Chinese
    }

    #[test]
    fn test_truncate_gbk() {
        assert_eq!(truncate_gbk("hello world", 5), "hello");
        assert_eq!(truncate_gbk("你好世界", 4), "你好");
        assert_eq!(truncate_gbk("AB中文", 4), "AB中");
    }

    #[test]
    fn test_pad_gbk() {
        assert_eq!(pad_gbk("hi", 5, false), "hi   ");
        assert_eq!(pad_gbk("hi", 5, true), "   hi");
        assert_eq!(pad_gbk("hello world", 5, false), "hello");
    }
}
