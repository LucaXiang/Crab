use image::GenericImageView;
use tracing::{error, info, instrument, warn};

pub fn get_gbk_width(s: &str) -> usize {
    let (cow, _, _) = encoding_rs::GBK.encode(s);
    cow.len()
}

pub fn truncate_to_gbk_width(s: &str, max_width: usize) -> String {
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

pub fn pad_to_gbk_width(s: &str, width: usize, align_right: bool) -> String {
    let current_width = get_gbk_width(s);
    if current_width >= width {
        return truncate_to_gbk_width(s, width);
    }
    let spaces = width - current_width;
    if align_right {
        format!("{}{}", " ".repeat(spaces), s)
    } else {
        format!("{}{}", s, " ".repeat(spaces))
    }
}

pub struct EscPosTextBuilder {
    buf: String,
    width: usize,
}

impl EscPosTextBuilder {
    pub fn new(width: usize) -> Self {
        Self {
            buf: String::new(),
            width,
        }
    }

    // --- Basic Operations ---
    pub fn write(&mut self, s: &str) {
        self.buf.push_str(s);
    }
    pub fn write_line(&mut self, s: &str) {
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    // --- Formatting Commands ---
    pub fn align_center(&mut self) {
        self.buf.push_str("\x1B\x61\x01");
    }
    pub fn align_left(&mut self) {
        self.buf.push_str("\x1B\x61\x00");
    }
    pub fn align_right(&mut self) {
        self.buf.push_str("\x1B\x61\x02");
    }

    pub fn bold_on(&mut self) {
        self.buf.push_str("\x1B\x45\x01");
    }
    pub fn bold_off(&mut self) {
        self.buf.push_str("\x1B\x45\x00");
    }

    pub fn size_double(&mut self) {
        self.buf.push_str("\x1D\x21\x11");
    }
    pub fn size_double_height(&mut self) {
        self.buf.push_str("\x1D\x21\x01");
    }
    pub fn size_double_width(&mut self) {
        self.buf.push_str("\x1D\x21\x10");
    }
    pub fn size_reset(&mut self) {
        self.buf.push_str("\x1D\x21\x00");
    }

    // --- Separators ---
    pub fn eq_sep(&mut self) {
        self.write_line(&"=".repeat(self.width));
    }
    pub fn dash_sep(&mut self) {
        self.write_line(&"-".repeat(self.width));
    }
    pub fn underscore_sep(&mut self) {
        self.write_line(&"_".repeat(self.width));
    }

    // Kept for backward compatibility if needed, but updated to use internal write_line
    pub fn eq_sep_str(&self) -> String {
        "=".repeat(self.width)
    }
    pub fn dash_sep_str(&self) -> String {
        "-".repeat(self.width)
    }
    pub fn underscore_sep_str(&self) -> String {
        "_".repeat(self.width)
    }

    // --- Layout Helpers ---

    /// Prints text centered in the current line width
    pub fn text_center(&mut self, s: &str) {
        self.align_center();
        self.write_line(s);
        self.align_left(); // Reset to left
    }

    /// Prints a key-value pair on the same line (Left aligned key, Right aligned value)
    pub fn line_lr(&mut self, left: &str, right: &str) {
        let lw = get_gbk_width(left);
        let rw = get_gbk_width(right);
        if lw + rw >= self.width {
            // If too long, just print with a space or wrap
            // Strategy: Print left, newline, align right print right?
            // Or just concat with space if it fits barely?
            // Current strategy: space separated
            self.write_line(&format!("{} {}", left, right));
        } else {
            let spaces = self.width - lw - rw;
            self.write(left);
            self.write(&" ".repeat(spaces));
            self.write_line(right);
        }
    }

    /// Prints a key-value pair, but with value filling the remaining space if possible?
    /// Or simply reusing line_lr logic.
    pub fn pair(&mut self, key: &str, value: &str) {
        self.line_lr(key, value);
    }

    pub fn finalize(self) -> String {
        self.buf
    }
}

/// Intelligent converter for mixed content (Binary Commands + UTF-8 Text) -> GBK
///
/// This function preserves ASCII bytes (0x00-0x7F) exactly as is, which protects
/// ESC/POS commands from being corrupted by UTF-8/GBK encoding.
///
/// Only bytes >= 0x80 are treated as potential UTF-8 sequences and converted to GBK.
///
/// # Arguments
/// * `bytes` - The input byte buffer containing mixed commands and UTF-8 text
///
/// # Returns
/// A new vector containing the converted bytes suitable for the printer
#[instrument(skip(bytes))]
pub fn convert_mixed_utf8_to_gbk(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len() * 2);

    // Always enable Chinese mode at the start to ensure GBK is interpreted correctly
    // FS & (0x1C 0x26) - Enable Chinese mode
    // FS C 1 (0x1C 0x43 0x01) - Select GBK code page (if supported)
    result.extend_from_slice(&[0x1C, 0x26, 0x1C, 0x43, 0x01]);

    let mut buffer = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];

        // Check for INIT command (ESC @ = 0x1B 0x40)
        // If we see INIT, we must re-enable Chinese mode after it,
        // because INIT resets the printer state.
        if b == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == 0x40 {
            // Process any pending non-ASCII buffer
            if !buffer.is_empty() {
                let s = String::from_utf8_lossy(&buffer);
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
            if !buffer.is_empty() {
                // Flush buffer: convert accumulated non-ASCII bytes to GBK
                let s = String::from_utf8_lossy(&buffer);
                let parts: Vec<&str> = s.split('€').collect();
                for (idx, part) in parts.iter().enumerate() {
                    if !part.is_empty() {
                        let (gbk, _, _) = encoding_rs::GBK.encode(part);
                        result.extend_from_slice(&gbk);
                    }
                    if idx < parts.len() - 1 {
                        // Inject Euro Sequence
                        result.extend_from_slice(&[0x1C, 0x2E, 0x1B, 0x74, 19, 0xD5, 0x1C, 0x26]);
                    }
                }
                buffer.clear();
            }
            result.push(b);
        } else {
            // Non-ASCII byte (Part of UTF-8 Chinese char)
            buffer.push(b);
        }
        i += 1;
    }

    // Flush remaining buffer
    if !buffer.is_empty() {
        let s = String::from_utf8_lossy(&buffer);
        let parts: Vec<&str> = s.split('€').collect();
        for (idx, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                let (gbk, _, _) = encoding_rs::GBK.encode(part);
                result.extend_from_slice(&gbk);
            }
            if idx < parts.len() - 1 {
                // Inject Euro Sequence
                result.extend_from_slice(&[0x1C, 0x2E, 0x1B, 0x74, 19, 0xD5, 0x1C, 0x26]);
            }
        }
    }

    // Optional: Exit Chinese mode at the end
    // FS . (0x1C 0x2E)
    result.extend_from_slice(&[0x1C, 0x2E]);

    result
}

#[instrument]
pub fn process_logo(path: &str) -> Option<Vec<u8>> {
    info!(path = path.to_string(), "processing logo");
    let img = match image::open(path) {
        Ok(i) => {
            info!(
                dimensions = format!("{:?}", i.dimensions()),
                "logo image opened"
            );
            i
        }
        Err(e) => {
            error!(error = format!("{}", e), "open logo failed");
            return None;
        }
    };
    let (w, h) = img.dimensions();

    // Resize if too wide (max 384 dots for safety on 58mm/80mm)
    let max_width = 384;
    let (new_w, new_h) = if w > max_width {
        let ratio = max_width as f64 / w as f64;
        (max_width, (h as f64 * ratio) as u32)
    } else {
        (w, h)
    };

    let resized = img.resize(new_w, new_h, image::imageops::FilterType::Nearest);

    // Raster bit image command GS v 0
    // m = 0 (Normal)
    // xL, xH = width in bytes
    // yL, yH = height in dots

    // We need width to be multiple of 8 for bytes
    let x_bytes = new_w.div_ceil(8);

    let mut data = Vec::new();

    // Center Align for Image
    data.extend_from_slice(&[0x1B, 0x61, 0x01]);

    // GS v 0
    data.push(0x1D);
    data.push(0x76);
    data.push(0x30);
    data.push(0x00);

    data.push(x_bytes as u8);
    data.push((x_bytes >> 8) as u8);

    data.push(new_h as u8);
    data.push((new_h >> 8) as u8);

    // Convert to RGBA first to handle transparency
    let rgba = resized.to_rgba8();
    let (real_w, real_h) = rgba.dimensions();
    info!(
        details = format!("{}x{} -> {}x{}", real_w, real_h, new_w, new_h),
        "logo resized"
    );

    for y in 0..new_h {
        for x_byte in 0..x_bytes {
            let mut byte = 0u8;
            for bit in 0..8 {
                let x = x_byte * 8 + bit;
                if x < new_w && x < real_w && y < real_h {
                    let pixel = rgba.get_pixel(x, y);
                    // Handle transparency: if alpha < 128, treat as white (0 in bit image? No, printer 0 is white usually? Wait.)
                    // ESC/POS GS v 0: "A bit of 1 indicates a dot to be printed (black), and a bit of 0 indicates that the dot is not to be printed (white)."

                    let alpha = pixel[3];
                    if alpha < 128 {
                        // Transparent -> White -> 0
                    } else {
                        // Opaque -> Check luminance
                        // Standard formula: 0.299*R + 0.587*G + 0.114*B
                        let luma = (0.299 * pixel[0] as f32
                            + 0.587 * pixel[1] as f32
                            + 0.114 * pixel[2] as f32) as u8;

                        // If dark enough (luma < 128), print black (1)
                        if luma < 128 {
                            byte |= 1 << (7 - bit);
                        }
                    }
                }
            }
            data.push(byte);
        }
    }

    data.push(0x0A); // New line after image

    Some(data)
}
