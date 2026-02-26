//! ESC/POS command builder
//!
//! Provides a fluent API for building ESC/POS print data.

use crate::encoding::{convert_to_gbk, gbk_width};
use tracing::instrument;

/// ESC/POS command builder
///
/// Builds ESC/POS byte sequences for thermal printers.
/// All text is automatically converted to GBK encoding.
pub struct EscPosBuilder {
    buf: Vec<u8>,
    width: usize,
}

impl EscPosBuilder {
    /// Create a new builder with the specified paper width in characters
    ///
    /// Common widths:
    /// - 58mm paper: 32 characters
    /// - 80mm paper: 48 characters
    pub fn new(width: usize) -> Self {
        let mut buf = Vec::with_capacity(4096);
        // Initialize printer (ESC @)
        buf.extend_from_slice(&[0x1B, 0x40]);
        Self { buf, width }
    }

    /// Get the configured paper width
    pub fn width(&self) -> usize {
        self.width
    }

    // === Text Output ===

    /// Write raw text (will be GBK encoded)
    pub fn text(&mut self, s: &str) -> &mut Self {
        self.buf.extend_from_slice(s.as_bytes());
        self
    }

    /// Write text followed by newline
    pub fn line(&mut self, s: &str) -> &mut Self {
        self.text(s);
        self.buf.push(b'\n');
        self
    }

    /// Write empty line
    pub fn newline(&mut self) -> &mut Self {
        self.buf.push(b'\n');
        self
    }

    /// Write multiple empty lines
    pub fn feed(&mut self, lines: u8) -> &mut Self {
        // ESC d n - Print and feed n lines
        self.buf.extend_from_slice(&[0x1B, 0x64, lines]);
        self
    }

    // === Alignment ===

    /// Align text to center
    pub fn center(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x61, 0x01]);
        self
    }

    /// Align text to left (default)
    pub fn left(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x61, 0x00]);
        self
    }

    /// Align text to right
    pub fn right(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x61, 0x02]);
        self
    }

    // === Text Style ===

    /// Enable bold text
    pub fn bold(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x45, 0x01]);
        self
    }

    /// Disable bold text
    pub fn bold_off(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x45, 0x00]);
        self
    }

    /// Double width and height
    pub fn double_size(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1D, 0x21, 0x11]);
        self
    }

    /// Double height only
    pub fn double_height(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1D, 0x21, 0x01]);
        self
    }

    /// Double width only
    pub fn double_width(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1D, 0x21, 0x10]);
        self
    }

    /// Reset to normal size
    pub fn reset_size(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1D, 0x21, 0x00]);
        self
    }

    // === Separators ===

    /// Print a line of '=' characters
    pub fn sep_double(&mut self) -> &mut Self {
        self.line(&"=".repeat(self.width))
    }

    /// Print a line of '-' characters
    pub fn sep_single(&mut self) -> &mut Self {
        self.line(&"-".repeat(self.width))
    }

    /// Print a line of '_' characters
    pub fn sep_underscore(&mut self) -> &mut Self {
        self.line(&"_".repeat(self.width))
    }

    // === Layout Helpers ===

    /// Print left and right text on the same line
    ///
    /// Left text is left-aligned, right text is right-aligned,
    /// with spaces filling the gap.
    pub fn line_lr(&mut self, left: &str, right: &str) -> &mut Self {
        let lw = gbk_width(left);
        let rw = gbk_width(right);

        if lw + rw >= self.width {
            // Too long, just print with space
            self.text(left);
            self.text(" ");
            self.line(right);
        } else {
            let spaces = self.width - lw - rw;
            self.text(left);
            self.text(&" ".repeat(spaces));
            self.line(right);
        }
        self
    }

    // === Paper Control ===

    /// Cut paper (full cut)
    pub fn cut(&mut self) -> &mut Self {
        // GS V 0 - Full cut
        self.buf.extend_from_slice(&[0x1D, 0x56, 0x00]);
        self
    }

    /// Full cut with feed — feeds n lines then cuts.
    /// Uses GS V 66 n, which lets the printer manage cutter-to-head distance.
    /// This produces less top-margin waste on the next ticket compared to
    /// separate feed() + cut() calls.
    pub fn cut_feed(&mut self, lines: u8) -> &mut Self {
        // GS V 66 n - Full cut after feeding n lines
        self.buf.extend_from_slice(&[0x1D, 0x56, 0x42, lines]);
        self
    }

    /// Partial cut (leave a small connection)
    pub fn cut_partial(&mut self) -> &mut Self {
        // GS V 1 - Partial cut
        self.buf.extend_from_slice(&[0x1D, 0x56, 0x01]);
        self
    }

    // === Cash Drawer ===

    /// Open cash drawer (pin 2)
    pub fn open_drawer(&mut self) -> &mut Self {
        // ESC p m t1 t2 - Generate pulse on pin m
        self.buf.extend_from_slice(&[0x1B, 0x70, 0x00, 25, 250]);
        self
    }

    /// Open cash drawer (pin 5)
    pub fn open_drawer_pin5(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x70, 0x01, 25, 250]);
        self
    }

    // === QR Code ===

    /// Print a QR code
    ///
    /// Size: 1-16 (module size in dots)
    pub fn qr_code(&mut self, data: &str, size: u8) -> &mut Self {
        let size = size.clamp(1, 16);

        // Function 165: Select model (Model 2)
        self.buf
            .extend_from_slice(&[0x1D, 0x28, 0x6B, 0x04, 0x00, 0x31, 0x41, 0x31, 0x00]);

        // Function 167: Set module size
        self.buf
            .extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x43, size]);

        // Function 169: Set error correction (L)
        self.buf
            .extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x45, 0x31]);

        // Function 180: Store data
        let data_bytes = data.as_bytes();
        let len = data_bytes.len() + 3;
        let p_l = (len & 0xFF) as u8;
        let p_h = ((len >> 8) & 0xFF) as u8;
        self.buf
            .extend_from_slice(&[0x1D, 0x28, 0x6B, p_l, p_h, 0x31, 0x50, 0x30]);
        self.buf.extend_from_slice(data_bytes);

        // Function 181: Print
        self.buf
            .extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x51, 0x30]);

        self
    }

    // === Raw Commands ===

    /// Write raw bytes directly
    pub fn raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    /// Reset printer to default state
    pub fn reset(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&[0x1B, 0x40]);
        self
    }

    // === Build ===

    /// Build the final byte buffer with GBK encoding
    ///
    /// This converts all UTF-8 text to GBK while preserving ESC/POS commands.
    pub fn build(self) -> Vec<u8> {
        convert_to_gbk(&self.buf)
    }

    /// Build without GBK conversion (for debugging or ASCII-only content)
    pub fn build_raw(self) -> Vec<u8> {
        self.buf
    }
}

impl Default for EscPosBuilder {
    fn default() -> Self {
        Self::new(48)
    }
}

// ============================================================================
// String-based ESC/POS Builder (for receipt rendering)
// ============================================================================

/// String-based ESC/POS command builder
///
/// Unlike `EscPosBuilder` which works with bytes and converts to GBK at the end,
/// this builder accumulates a UTF-8 String that should be converted to GBK
/// separately (e.g., using `convert_to_gbk`).
///
/// This is useful when you want to build receipt content as a String first,
/// then handle the binary conversion and printing separately.
pub struct EscPosTextBuilder {
    buf: String,
    width: usize,
}

impl EscPosTextBuilder {
    /// Create a new text builder with specified paper width in characters
    pub fn new(width: usize) -> Self {
        Self {
            buf: String::new(),
            width,
        }
    }

    /// Get the configured paper width
    pub fn width(&self) -> usize {
        self.width
    }

    // === Text Output ===

    /// Write raw text
    pub fn write(&mut self, s: &str) -> &mut Self {
        self.buf.push_str(s);
        self
    }

    /// Write text followed by newline
    pub fn write_line(&mut self, s: &str) -> &mut Self {
        self.buf.push_str(s);
        self.buf.push('\n');
        self
    }

    // === Alignment ===

    /// Align text to center
    pub fn align_center(&mut self) -> &mut Self {
        self.buf.push_str("\x1B\x61\x01");
        self
    }

    /// Align text to left (default)
    pub fn align_left(&mut self) -> &mut Self {
        self.buf.push_str("\x1B\x61\x00");
        self
    }

    /// Align text to right
    pub fn align_right(&mut self) -> &mut Self {
        self.buf.push_str("\x1B\x61\x02");
        self
    }

    // === Text Style ===

    /// Enable bold text
    pub fn bold_on(&mut self) -> &mut Self {
        self.buf.push_str("\x1B\x45\x01");
        self
    }

    /// Disable bold text
    pub fn bold_off(&mut self) -> &mut Self {
        self.buf.push_str("\x1B\x45\x00");
        self
    }

    /// Double width and height
    pub fn size_double(&mut self) -> &mut Self {
        self.buf.push_str("\x1D\x21\x11");
        self
    }

    /// Double height only
    pub fn size_double_height(&mut self) -> &mut Self {
        self.buf.push_str("\x1D\x21\x01");
        self
    }

    /// Double width only
    pub fn size_double_width(&mut self) -> &mut Self {
        self.buf.push_str("\x1D\x21\x10");
        self
    }

    /// Reset to normal size
    pub fn size_reset(&mut self) -> &mut Self {
        self.buf.push_str("\x1D\x21\x00");
        self
    }

    // === Separators ===

    /// Print a line of '=' characters
    pub fn eq_sep(&mut self) -> &mut Self {
        self.write_line(&"=".repeat(self.width))
    }

    /// Print a line of '-' characters
    pub fn dash_sep(&mut self) -> &mut Self {
        self.write_line(&"-".repeat(self.width))
    }

    /// Print a line of '_' characters
    pub fn underscore_sep(&mut self) -> &mut Self {
        self.write_line(&"_".repeat(self.width))
    }

    /// Get separator string (for compatibility)
    pub fn eq_sep_str(&self) -> String {
        "=".repeat(self.width)
    }

    pub fn dash_sep_str(&self) -> String {
        "-".repeat(self.width)
    }

    pub fn underscore_sep_str(&self) -> String {
        "_".repeat(self.width)
    }

    // === Layout Helpers ===

    /// Print text centered in the current line width
    pub fn text_center(&mut self, s: &str) -> &mut Self {
        self.align_center();
        self.write_line(s);
        self.align_left();
        self
    }

    /// Print left and right text on the same line
    pub fn line_lr(&mut self, left: &str, right: &str) -> &mut Self {
        let lw = crate::encoding::gbk_width(left);
        let rw = crate::encoding::gbk_width(right);

        if lw + rw >= self.width {
            self.write_line(&format!("{} {}", left, right));
        } else {
            let spaces = self.width - lw - rw;
            self.write(left);
            self.write(&" ".repeat(spaces));
            self.write_line(right);
        }
        self
    }

    /// Print a key-value pair (alias for line_lr)
    pub fn pair(&mut self, key: &str, value: &str) -> &mut Self {
        self.line_lr(key, value)
    }

    // === Build ===

    /// Finalize and return the accumulated string
    pub fn finalize(self) -> String {
        self.buf
    }

    /// Get the current buffer as a string reference
    pub fn as_str(&self) -> &str {
        &self.buf
    }
}

impl Default for EscPosTextBuilder {
    fn default() -> Self {
        Self::new(48)
    }
}

// ============================================================================
// Image Processing
// ============================================================================

/// Process an image file and return ESC/POS raster data
///
/// The image will be:
/// - Resized to fit max width (384 dots for 80mm, 384 for safety)
/// - Converted to 1-bit monochrome
/// - Encoded as GS v 0 raster graphics
#[cfg(feature = "image")]
#[instrument]
pub fn process_logo(path: &str) -> Option<Vec<u8>> {
    use image::GenericImageView;
    use tracing::{error, info};

    info!(path = path, "processing logo");

    let img = match image::open(path) {
        Ok(i) => {
            info!(dimensions = ?i.dimensions(), "logo image opened");
            i
        }
        Err(e) => {
            error!(error = %e, "open logo failed");
            return None;
        }
    };

    let (w, h) = img.dimensions();

    // Resize if too wide (max 384 dots for 58mm/80mm)
    let max_width = 384;
    let (new_w, new_h) = if w > max_width {
        let ratio = max_width as f64 / w as f64;
        (max_width, (h as f64 * ratio) as u32)
    } else {
        (w, h)
    };

    let resized = img.resize(new_w, new_h, image::imageops::FilterType::Nearest);

    // Raster bit image command GS v 0
    let x_bytes = new_w.div_ceil(8);

    let mut data = Vec::new();

    // Center align for image
    data.extend_from_slice(&[0x1B, 0x61, 0x01]);

    // GS v 0 m xL xH yL yH
    data.extend_from_slice(&[0x1D, 0x76, 0x30, 0x00]);
    data.push(x_bytes as u8);
    data.push((x_bytes >> 8) as u8);
    data.push(new_h as u8);
    data.push((new_h >> 8) as u8);

    // Convert to RGBA for transparency handling
    let rgba = resized.to_rgba8();

    for y in 0..new_h {
        for x_byte in 0..x_bytes {
            let mut byte = 0u8;
            for bit in 0..8 {
                let x = x_byte * 8 + bit;
                if x < new_w {
                    let pixel = rgba.get_pixel(x, y);

                    // Handle transparency
                    let alpha = pixel[3];
                    if alpha >= 128 {
                        // Opaque - check luminance
                        let luma = (0.299 * pixel[0] as f32
                            + 0.587 * pixel[1] as f32
                            + 0.114 * pixel[2] as f32) as u8;

                        // Dark enough = print black (1)
                        if luma < 128 {
                            byte |= 1 << (7 - bit);
                        }
                    }
                    // Transparent = white (0)
                }
            }
            data.push(byte);
        }
    }

    // Newline after image
    data.push(0x0A);

    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut b = EscPosBuilder::new(32);
        b.center()
            .double_size()
            .line("标题")
            .reset_size()
            .left()
            .line("内容");

        let data = b.build_raw();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_line_lr() {
        let mut b = EscPosBuilder::new(20);
        b.line_lr("左", "右");

        let data = b.build_raw();
        // Should contain the text
        let s = String::from_utf8_lossy(&data);
        assert!(s.contains("左"));
        assert!(s.contains("右"));
    }

    #[test]
    fn test_separators() {
        let mut b = EscPosBuilder::new(10);
        b.sep_double();

        let data = b.build_raw();
        let s = String::from_utf8_lossy(&data);
        assert!(s.contains("=========="));
    }
}
