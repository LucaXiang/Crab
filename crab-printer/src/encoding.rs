//! GBK + CP858 encoding utilities for thermal printers
//!
//! Supports mixed Chinese (GBK) and European Latin (CP858) text.
//! - CJK characters → GBK encoding (Chinese mode)
//! - Latin extended characters (ñ, á, €, etc.) → CP858 encoding
//! - ASCII and ESC/POS commands → passed through unchanged

use tracing::instrument;

/// Get the display width of a string on the printer
///
/// CJK characters = 2 columns, everything else = 1 column.
pub fn gbk_width(s: &str) -> usize {
    s.chars().map(|c| if is_cjk(c) { 2 } else { 1 }).sum()
}

/// Truncate a string to fit within a display width
pub fn truncate_gbk(s: &str, max_width: usize) -> String {
    let mut width = 0;
    let mut result = String::new();
    for c in s.chars() {
        let w = if is_cjk(c) { 2 } else { 1 };
        if width + w > max_width {
            break;
        }
        result.push(c);
        width += w;
    }
    result
}

/// Pad a string to a specific display width
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

/// Convert mixed UTF-8 content (with ESC/POS commands) to GBK + CP858
///
/// Three classes of bytes:
/// 1. ASCII (0x00-0x7F): passed through (includes ESC/POS commands)
/// 2. CJK characters: GBK encoded (Chinese mode via FS &)
/// 3. Latin extended (ñ, á, €, etc.): CP858 encoded (via ESC t 19)
#[instrument(skip(bytes))]
pub fn convert_to_gbk(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len() * 2);

    // Start in Chinese mode
    // FS & (0x1C 0x26) - Enable Chinese mode
    // FS C 1 (0x1C 0x43 0x01) - Select GBK code page
    result.extend_from_slice(&[0x1C, 0x26, 0x1C, 0x43, 0x01]);

    let mut buffer = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Check for INIT command (ESC @ = 0x1B 0x40)
        if b == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == 0x40 {
            flush_buffer(&mut buffer, &mut result);
            result.push(0x1B);
            result.push(0x40);
            // Re-enable Chinese mode after INIT
            result.extend_from_slice(&[0x1C, 0x26]);
            i += 2;
            continue;
        }

        if b < 128 {
            // ASCII byte — flush non-ASCII buffer, then pass through
            flush_buffer(&mut buffer, &mut result);
            result.push(b);
        } else {
            // Non-ASCII byte (part of UTF-8 sequence)
            buffer.push(b);
        }
        i += 1;
    }

    flush_buffer(&mut buffer, &mut result);

    // Exit Chinese mode at the end
    result.extend_from_slice(&[0x1C, 0x2E]);

    result
}

/// Flush non-ASCII buffer: route each character to GBK or CP858
fn flush_buffer(buffer: &mut Vec<u8>, result: &mut Vec<u8>) {
    if buffer.is_empty() {
        return;
    }

    let s = String::from_utf8_lossy(buffer);

    // Classify consecutive characters into runs of CJK vs Latin
    let mut cjk_run = String::new();

    for ch in s.chars() {
        if let Some(cp858_byte) = unicode_to_cp858(ch) {
            // Latin character — flush CJK run first, then emit CP858
            if !cjk_run.is_empty() {
                let (gbk, _, _) = encoding_rs::GBK.encode(&cjk_run);
                result.extend_from_slice(&gbk);
                cjk_run.clear();
            }
            // Exit Chinese → select CP858 → byte → re-enter Chinese
            result.extend_from_slice(&[0x1C, 0x2E, 0x1B, 0x74, 19]);
            result.push(cp858_byte);
            result.extend_from_slice(&[0x1C, 0x26]);
        } else {
            // CJK or other — accumulate for GBK encoding
            cjk_run.push(ch);
        }
    }

    // Flush remaining CJK
    if !cjk_run.is_empty() {
        let (gbk, _, _) = encoding_rs::GBK.encode(&cjk_run);
        result.extend_from_slice(&gbk);
    }

    buffer.clear();
}

/// Map Unicode character to CP858 byte value.
///
/// Returns Some(byte) for Latin extended characters not in ASCII/GBK,
/// Returns None for CJK characters (should use GBK instead).
fn unicode_to_cp858(ch: char) -> Option<u8> {
    match ch {
        // Currency / symbols
        '€' => Some(0xD5),
        '£' => Some(0x9C),
        '¥' => Some(0xBE),
        '¢' => Some(0xBD),

        // Spanish essentials
        'ñ' => Some(0xA4),
        'Ñ' => Some(0xA5),
        '¿' => Some(0xA8),
        '¡' => Some(0xAD),

        // Vowels with acute accent
        'á' => Some(0xA0),
        'é' => Some(0x82),
        'í' => Some(0xA1),
        'ó' => Some(0xA2),
        'ú' => Some(0xA3),
        'Á' => Some(0xB5),
        'É' => Some(0x90),
        'Í' => Some(0xD6),
        'Ó' => Some(0xE0),
        'Ú' => Some(0xE9),

        // Vowels with grave accent
        'à' => Some(0x85),
        'è' => Some(0x8A),
        'ì' => Some(0x8D),
        'ò' => Some(0x95),
        'ù' => Some(0x97),
        'À' => Some(0xB7),
        'È' => Some(0xD4),
        'Ì' => Some(0xDE),
        'Ò' => Some(0xE3),
        'Ù' => Some(0xEB),

        // Vowels with circumflex
        'â' => Some(0x83),
        'ê' => Some(0x88),
        'î' => Some(0x8C),
        'ô' => Some(0x93),
        'û' => Some(0x96),
        'Â' => Some(0xB6),
        'Ê' => Some(0xD2),
        'Î' => Some(0xD7),
        'Ô' => Some(0xE2),
        'Û' => Some(0xEA),

        // Diaeresis / umlaut
        'ä' => Some(0x84),
        'ë' => Some(0x89),
        'ï' => Some(0x8B),
        'ö' => Some(0x94),
        'ü' => Some(0x81),
        'Ä' => Some(0x8E),
        'Ë' => Some(0xD3),
        'Ï' => Some(0xD8),
        'Ö' => Some(0x99),
        'Ü' => Some(0x9A),

        // Other Latin
        'ç' => Some(0x87),
        'Ç' => Some(0x80),
        'ß' => Some(0xE1),
        'ã' => Some(0xC6),
        'õ' => Some(0xE4),
        'Ã' => Some(0xC7),
        'Õ' => Some(0xE5),

        // Common symbols
        '°' => Some(0xF8),
        '±' => Some(0xF1),
        '§' => Some(0xF5),
        '«' => Some(0xAE),
        '»' => Some(0xAF),
        '©' => Some(0xB8),
        '®' => Some(0xA9),

        _ => None,
    }
}

/// Check if a character is CJK (occupies 2 columns)
fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs + extensions
    (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x20000..=0x2A6DF).contains(&cp)
        // CJK symbols, Hiragana, Katakana, Hangul
        || (0x3000..=0x303F).contains(&cp)
        || (0x3040..=0x309F).contains(&cp)
        || (0x30A0..=0x30FF).contains(&cp)
        || (0xAC00..=0xD7AF).contains(&cp)
        // Fullwidth forms
        || (0xFF01..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbk_width() {
        assert_eq!(gbk_width("hello"), 5);
        assert_eq!(gbk_width("你好"), 4); // 2 CJK chars = 4 columns
        assert_eq!(gbk_width("AB中文CD"), 8); // 4 ASCII + 2 CJK
    }

    #[test]
    fn test_gbk_width_latin() {
        // Latin extended chars are 1 column each
        assert_eq!(gbk_width("ñ"), 1);
        assert_eq!(gbk_width("café"), 4);
        assert_eq!(gbk_width("8,40 €"), 6);
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

    #[test]
    fn test_spanish_chars_encoded() {
        // Verify ñ gets CP858 encoding, not GBK fallback
        let result = convert_to_gbk("ñ".as_bytes());
        // Should contain CP858 byte 0xA4 for ñ
        assert!(
            result.windows(1).any(|w| w[0] == 0xA4),
            "should contain CP858 byte 0xA4 for ñ"
        );
    }

    #[test]
    fn test_mixed_cjk_latin() {
        // Mixed Chinese + Spanish text
        let result = convert_to_gbk("宫保鸡丁 (señor)".as_bytes());
        // Should not be empty and should be longer than input (due to mode switches)
        assert!(result.len() > 10);
        // Should contain CP858 byte for ñ
        assert!(
            result.windows(1).any(|w| w[0] == 0xA4),
            "mixed text should encode ñ via CP858"
        );
    }

    #[test]
    fn test_euro_encoded() {
        let result = convert_to_gbk("8,40 €".as_bytes());
        assert!(
            result.windows(1).any(|w| w[0] == 0xD5),
            "should contain CP858 byte 0xD5 for €"
        );
    }
}
