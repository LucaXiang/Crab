use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::mem::{align_of, size_of};
use std::ptr::NonNull;

use windows::core::{w, Error, Result as WinResult, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{GetLastError, E_FAIL, E_INVALIDARG};
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, GetDeviceCaps, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DEVMODEW, DIB_RGB_COLORS, DMBIN_FORMSOURCE, DMPAPER_USER, DM_DEFAULTSOURCE, DM_PAPERLENGTH,
    DM_PAPERSIZE, DM_PAPERWIDTH, HDC, LOGPIXELSX, LOGPIXELSY, SRCCOPY,
};
use windows::Win32::Graphics::GdiPlus::{
    FontStyleBold, FontStyleRegular, GdipCreateBitmapFromScan0, GdipCreateFont,
    GdipCreateFontFamilyFromName, GdipCreatePen1, GdipCreateSolidFill, GdipDeleteBrush,
    GdipDeleteFont, GdipDeleteFontFamily, GdipDeleteGraphics, GdipDeletePen, GdipDisposeImage,
    GdipDrawLine, GdipDrawString, GdipFillRectangle, GdipGetImageGraphicsContext,
    GdipSetInterpolationMode, GdipSetSmoothingMode, GdipSetTextRenderingHint, GdiplusShutdown,
    GdiplusStartup, GdiplusStartupInput, GpBitmap, GpBrush, GpFont, GpFontFamily, GpGraphics,
    GpPen, GpStringFormat, InterpolationModeHighQualityBicubic, SmoothingModeAntiAlias,
    TextRenderingHintSingleBitPerPixelGridFit, UnitPixel,
};
use windows::Win32::Graphics::Printing::{
    ClosePrinter, DocumentPropertiesW, OpenPrinterW, PRINTER_HANDLE,
};
use windows::Win32::Storage::Xps::DOCINFOW;

#[link(name = "gdi32")]
extern "system" {
    fn AbortDoc(hdc: HDC) -> i32;
    fn EndDoc(hdc: HDC) -> i32;
    fn EndPage(hdc: HDC) -> i32;
    fn StartDocW(hdc: HDC, lpdi: *const DOCINFOW) -> i32;
    fn StartPage(hdc: HDC) -> i32;
}

#[derive(Clone, Copy, Debug)]
pub enum FitMode {
    Contain,
    Cover,
    Fill,
}

#[derive(Clone, Copy, Debug)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

#[derive(Clone, Debug)]
pub struct PrintOptions {
    pub printer_name: Option<String>,
    pub doc_name: String,
    pub label_width_mm: f32,
    pub label_height_mm: f32,
    pub copies: u32,
    pub fit: FitMode,
    pub rotate: Rotation,
    pub override_dpi: Option<f32>,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            printer_name: None,
            doc_name: "label".to_string(),
            label_width_mm: 40.0,
            label_height_mm: 30.0,
            copies: 1,
            fit: FitMode::Contain,
            rotate: Rotation::R0,
            override_dpi: None,
        }
    }
}

fn create_custom_printer_dc(printer: &str, width_mm: f32, height_mm: f32) -> WinResult<HDC> {
    unsafe {
        let mut printer_w = to_wide(printer);
        let mut hprinter = PRINTER_HANDLE::default();
        OpenPrinterW(PWSTR(printer_w.as_mut_ptr()), &mut hprinter, None)?;

        let _guard = PrinterGuard(hprinter);
        let needed =
            DocumentPropertiesW(None, hprinter, PWSTR(printer_w.as_mut_ptr()), None, None, 0);
        if needed <= 0 {
            return Err(last_win32_error());
        }

        let layout = Layout::from_size_align(needed as usize, align_of::<DEVMODEW>())
            .map_err(|_| Error::new(E_FAIL, "layout"))?;

        let raw = alloc_zeroed(layout);
        let ptr = NonNull::new(raw).ok_or_else(|| Error::new(E_FAIL, "alloc"))?;
        let devmode = ptr.as_ptr() as *mut DEVMODEW;

        let r = DocumentPropertiesW(
            None,
            hprinter,
            PWSTR(printer_w.as_mut_ptr()),
            Some(devmode),
            None,
            2,
        );
        if r != 1 {
            dealloc(ptr.as_ptr(), layout);
            return Err(last_win32_error());
        }

        let w_01mm = (width_mm * 10.0).round();
        let h_01mm = (height_mm * 10.0).round();

        (*devmode).Anonymous1.Anonymous1.dmPaperSize = DMPAPER_USER as i16;
        (*devmode).Anonymous1.Anonymous1.dmPaperWidth = w_01mm as i16;
        (*devmode).Anonymous1.Anonymous1.dmPaperLength = h_01mm as i16;
        (*devmode).Anonymous1.Anonymous1.dmDefaultSource = DMBIN_FORMSOURCE as i16;
        (*devmode).dmFields |= DM_PAPERSIZE | DM_PAPERWIDTH | DM_PAPERLENGTH | DM_DEFAULTSOURCE;

        let r = DocumentPropertiesW(
            None,
            hprinter,
            PWSTR(printer_w.as_mut_ptr()),
            Some(devmode),
            Some(devmode),
            10,
        );
        if r != 1 {
            dealloc(ptr.as_ptr(), layout);
            return Err(last_win32_error());
        }

        let hdc = CreateDCW(
            w!("WINSPOOL"),
            PCWSTR::from_raw(printer_w.as_ptr()),
            PCWSTR::null(),
            Some(devmode as *const _),
        );
        dealloc(ptr.as_ptr(), layout);

        if hdc.is_invalid() {
            return Err(last_win32_error());
        }

        Ok(hdc)
    }
}

pub fn print_rgba_premul(
    rgba_premul: &[u8],
    width: u32,
    height: u32,
    options: &PrintOptions,
) -> WinResult<()> {
    if rgba_premul.len() != (width as usize) * (height as usize) * 4 {
        return Err(Error::new(E_INVALIDARG, "invalid rgba length"));
    }
    if width == 0 || height == 0 {
        return Err(Error::new(E_INVALIDARG, "invalid image size"));
    }
    if options.copies == 0 {
        return Err(Error::new(E_INVALIDARG, "copies must be > 0"));
    }

    let printer = crate::utils::printing::resolve_printer(options.printer_name.clone())
        .map_err(|e| Error::new(E_INVALIDARG, e))?;

    let (bgra, img_w, img_h) =
        build_bgra_opaque_on_white(rgba_premul, width, height, options.rotate);
    let doc_w = to_wide(&options.doc_name);

    let hdc = create_custom_printer_dc(&printer, options.label_width_mm, options.label_height_mm)?;

    unsafe {
        let _guard = HdcGuard(hdc);

        let di = DOCINFOW {
            cbSize: size_of::<DOCINFOW>() as i32,
            lpszDocName: PCWSTR::from_raw(doc_w.as_ptr()),
            ..Default::default()
        };
        let doc_id = StartDocW(hdc, &di);
        if doc_id <= 0 {
            return Err(last_win32_error());
        }

        let mut doc = DocGuard { hdc, active: true };

        let dpi_x = GetDeviceCaps(Some(hdc), LOGPIXELSX).max(1);
        let dpi_y = GetDeviceCaps(Some(hdc), LOGPIXELSY).max(1);

        let mut target_w = mm_to_px(options.label_width_mm, dpi_x as f32);
        let mut target_h = mm_to_px(options.label_height_mm, dpi_y as f32);
        if target_w <= 0 {
            target_w = img_w as i32;
        }
        if target_h <= 0 {
            target_h = img_h as i32;
        }

        let (draw_w, draw_h) =
            fit_rect(img_w as i32, img_h as i32, target_w, target_h, options.fit);
        let dest_x = (target_w - draw_w) / 2;
        let dest_y = (target_h - draw_h) / 2;

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: img_w as i32,
                biHeight: -(img_h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        for _ in 0..options.copies {
            let r = StartPage(hdc);
            if r <= 0 {
                doc.abort();
                return Err(last_win32_error());
            }

            let r = StretchDIBits(
                hdc,
                dest_x,
                dest_y,
                draw_w,
                draw_h,
                0,
                0,
                img_w as i32,
                img_h as i32,
                Some(bgra.as_ptr() as *const _),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            if r == 0 {
                doc.abort();
                return Err(last_win32_error());
            }

            let r = EndPage(hdc);
            if r <= 0 {
                doc.abort();
                return Err(last_win32_error());
            }
        }

        doc.end()?;
    }

    Ok(())
}

struct HdcGuard(HDC);

impl Drop for HdcGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

struct PrinterGuard(PRINTER_HANDLE);

impl Drop for PrinterGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ClosePrinter(self.0);
        }
    }
}

struct DocGuard {
    hdc: HDC,
    active: bool,
}

impl DocGuard {
    fn abort(&mut self) {
        if !self.active {
            return;
        }
        unsafe {
            let _ = AbortDoc(self.hdc);
        }
        self.active = false;
    }

    fn end(&mut self) -> WinResult<()> {
        if !self.active {
            return Ok(());
        }
        let r = unsafe { EndDoc(self.hdc) };
        self.active = false;
        if r <= 0 {
            return Err(last_win32_error());
        }
        Ok(())
    }
}

impl Drop for DocGuard {
    fn drop(&mut self) {
        self.abort();
    }
}

fn last_win32_error() -> Error {
    let code = unsafe { GetLastError().0 };
    if code == 0 {
        return Error::new(E_FAIL, "win32 error");
    }
    Error::from_hresult(hresult_from_win32(code))
}

fn hresult_from_win32(code: u32) -> HRESULT {
    if code == 0 {
        return HRESULT(0);
    }
    let facility_win32 = 7u32;
    let h = (code & 0x0000_FFFF) | (facility_win32 << 16) | 0x8000_0000;
    HRESULT(h as i32)
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn mm_to_px(mm: f32, dpi: f32) -> i32 {
    if !mm.is_finite() || mm <= 0.0 {
        return 0;
    }
    let px = mm * dpi / 25.4;
    px.round().max(1.0) as i32
}

fn fit_rect(src_w: i32, src_h: i32, dst_w: i32, dst_h: i32, fit: FitMode) -> (i32, i32) {
    let sw = (src_w.max(1)) as f32;
    let sh = (src_h.max(1)) as f32;
    let dw = (dst_w.max(1)) as f32;
    let dh = (dst_h.max(1)) as f32;

    if let FitMode::Fill = fit {
        return (dst_w, dst_h);
    }

    let scale = match fit {
        FitMode::Contain => (dw / sw).min(dh / sh),
        FitMode::Cover => (dw / sw).max(dh / sh),
        FitMode::Fill => unreachable!(),
    };
    let w = (sw * scale).round().max(1.0) as i32;
    let h = (sh * scale).round().max(1.0) as i32;
    (w.min(dst_w), h.min(dst_h))
}

fn build_bgra_opaque_on_white(
    rgba_premul: &[u8],
    width: u32,
    height: u32,
    rotate: Rotation,
) -> (Vec<u8>, u32, u32) {
    let (out_w, out_h) = match rotate {
        Rotation::R0 | Rotation::R180 => (width, height),
        Rotation::R90 | Rotation::R270 => (height, width),
    };

    let mut out = vec![0u8; (out_w as usize) * (out_h as usize) * 4];

    let w = width as i32;
    let h = height as i32;
    let ow = out_w as i32;
    let oh = out_h as i32;

    for dy in 0..oh {
        for dx in 0..ow {
            let (sx, sy) = match rotate {
                Rotation::R0 => (dx, dy),
                Rotation::R90 => (dy, h - 1 - dx),
                Rotation::R180 => (w - 1 - dx, h - 1 - dy),
                Rotation::R270 => (w - 1 - dy, dx),
            };

            let si = ((sy as usize) * (width as usize) + (sx as usize)) * 4;
            let r = rgba_premul[si];
            let g = rgba_premul[si + 1];
            let b = rgba_premul[si + 2];
            let a = rgba_premul[si + 3];

            let add = 255u8.wrapping_sub(a);
            let rr = r.saturating_add(add);
            let gg = g.saturating_add(add);
            let bb = b.saturating_add(add);

            let di = ((dy as usize) * (out_w as usize) + (dx as usize)) * 4;
            out[di] = bb;
            out[di + 1] = gg;
            out[di + 2] = rr;
            out[di + 3] = 255;
        }
    }
    (out, out_w, out_h)
}

// ============================================================================
// GDI+ Label Rendering
// ============================================================================

use serde::{Deserialize, Serialize};

/// Text alignment
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Font style
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TextStyle {
    Regular,
    Bold,
}

/// Text field configuration with template variable support
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextField {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    #[serde(default)]
    pub font_family: Option<String>,
    pub style: TextStyle,
    pub align: TextAlign,
    pub template: String, // Template with variables like "Order: {order_id}"
}

/// Image field configuration with optional template variable for data key
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageField {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maintain_aspect_ratio: bool,
    pub data_key: String, // Key in JSON data for image data (e.g., "product_image")
}

/// Separator line field
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeparatorField {
    pub y: f32,
    pub x_start: Option<f32>, // Default: 8.0
    pub x_end: Option<f32>,   // Default: width - 8.0
}

/// Template field types
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TemplateField {
    Text(TextField),
    Image(ImageField),
    Separator(SeparatorField),
}

/// Template for label layout with dynamic fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LabelTemplate {
    pub width_mm: f32,
    pub height_mm: f32,
    #[serde(default)]
    pub padding_mm_x: f32,
    #[serde(default)]
    pub padding_mm_y: f32,
    pub fields: Vec<TemplateField>,
}

impl Default for LabelTemplate {
    fn default() -> Self {
        // Default 40mm x 30mm template
        Self {
            width_mm: 40.0,
            height_mm: 30.0,
            padding_mm_x: 0.0,
            padding_mm_y: 0.0,
            fields: vec![
                TemplateField::Text(TextField {
                    x: 8.0,
                    y: 8.0,
                    width: 304.0,
                    height: 20.0,
                    font_size: 10.0,
                    font_family: None,
                    style: TextStyle::Regular,
                    align: TextAlign::Left,
                    template: "#{order_id}".to_string(),
                }),
                TemplateField::Text(TextField {
                    x: 8.0,
                    y: 30.0,
                    width: 304.0,
                    height: 40.0,
                    font_size: 16.0,
                    font_family: None,
                    style: TextStyle::Bold,
                    align: TextAlign::Left,
                    template: "{item_name}".to_string(),
                }),
                TemplateField::Text(TextField {
                    x: 8.0,
                    y: 73.0,
                    width: 304.0,
                    height: 30.0,
                    font_size: 11.0,
                    font_family: None,
                    style: TextStyle::Regular,
                    align: TextAlign::Left,
                    template: "{specs}".to_string(),
                }),
                TemplateField::Separator(SeparatorField {
                    y: 205.0,
                    x_start: Some(8.0),
                    x_end: None,
                }),
                TemplateField::Text(TextField {
                    x: 8.0,
                    y: 212.0,
                    width: 152.0,
                    height: 20.0,
                    font_size: 10.0,
                    font_family: None,
                    style: TextStyle::Bold,
                    align: TextAlign::Left,
                    template: "{price}".to_string(),
                }),
                TemplateField::Text(TextField {
                    x: 160.0,
                    y: 212.0,
                    width: 152.0,
                    height: 20.0,
                    font_size: 10.0,
                    font_family: None,
                    style: TextStyle::Bold,
                    align: TextAlign::Right,
                    template: "{time}".to_string(),
                }),
            ],
        }
    }
}

/// Render template string with data from JSON
/// Supports {key} placeholders that will be replaced with values from the JSON data
/// Example: "Order: {order_id}" with {"order_id": "A123"} -> "Order: A123"
fn render_template(template: &str, data: &serde_json::Value) -> String {
    use regex::Regex;

    let re = Regex::new(r"\{([^}]+)\}").unwrap();
    let mut result = template.to_string();

    for cap in re.captures_iter(template) {
        let key = &cap[1];
        let placeholder = &cap[0];

        if let Some(value) = data.get(key) {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                _ => serde_json::to_string(value).unwrap_or_default(),
            };
            result = result.replace(placeholder, &value_str);
        } else {
            // If key not found, leave placeholder or replace with empty string
            result = result.replace(placeholder, "");
        }
    }

    result
}

/// Extract image data from JSON by key
/// Returns (RGBA data, width, height) if found
///
/// Supported formats:
/// 1. Base64 Data URI: "data:image/png;base64,iVBORw0KG..." (from QRCode/Barcode/frontend)
/// 2. JSON Object: { data: [u8...], width: u32, height: u32 }
fn extract_image_data(data: &serde_json::Value, data_key: &str) -> Option<(Vec<u8>, u32, u32)> {
    // Try to get image data from the specified key
    let img_data = data.get(data_key)?;

    // Format 1: Base64 Data URI string
    if let serde_json::Value::String(data_uri) = img_data {
        return decode_base64_image(data_uri);
    }

    // Format 2: JSON Object with { data: [u8...], width: u32, height: u32 }
    if let serde_json::Value::Object(obj) = img_data {
        let data_array = obj.get("data")?.as_array()?;
        let width = obj.get("width")?.as_u64()? as u32;
        let height = obj.get("height")?.as_u64()? as u32;

        let rgba: Vec<u8> = data_array
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as u8))
            .collect();

        if rgba.len() == (width * height * 4) as usize {
            return Some((rgba, width, height));
        }
    }

    None
}

/// Decode Base64 Data URI to RGBA image data
/// Supports: data:image/png;base64,... or data:image/jpeg;base64,...
fn decode_base64_image(data_uri: &str) -> Option<(Vec<u8>, u32, u32)> {
    use base64::Engine;

    // Check if it's a data URI
    if !data_uri.starts_with("data:image/") {
        return None;
    }

    // Extract base64 part after "base64,"
    let base64_start = data_uri.find("base64,")?;
    let base64_str = &data_uri[base64_start + 7..]; // Skip "base64,"

    // Decode base64 to bytes using standard engine
    let img_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_str)
        .ok()?;

    // Decode image using image crate
    let img = image::load_from_memory(&img_bytes).ok()?;

    // Convert to RGBA8
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    // Convert to Vec<u8>
    let rgba_data = rgba_img.into_raw();

    Some((rgba_data, width, height))
}

/// GDI+ initialization guard
struct GdiplusToken(usize);

impl GdiplusToken {
    fn init() -> WinResult<Self> {
        let mut token: usize = 0;
        let mut input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: 0,
            SuppressBackgroundThread: false.into(),
            SuppressExternalCodecs: false.into(),
        };

        unsafe {
            let status = GdiplusStartup(&mut token, &mut input, std::ptr::null_mut());
            if status.0 != 0 {
                return Err(Error::new(E_FAIL, "GdiplusStartup failed"));
            }
        }
        Ok(Self(token))
    }
}

impl Drop for GdiplusToken {
    fn drop(&mut self) {
        unsafe {
            GdiplusShutdown(self.0);
        }
    }
}

/// Render label using JSON data and template configuration
/// Returns RGBA premultiplied pixel data and dimensions
///
/// # Arguments
/// * `data` - JSON value containing all fields referenced in template (e.g., {"order_id": "A123", "item_name": "Tea"})
/// * `template` - Template configuration with dynamic fields
/// * `dpi` - Rendering DPI (typically 300 for thermal printers)
pub fn render_label_gdiplus(
    data: &serde_json::Value,
    template: &LabelTemplate,
    dpi: f32,
) -> WinResult<(Vec<u8>, u32, u32)> {
    let width_px = (template.width_mm * dpi / 25.4).round() as u32;
    let height_px = (template.height_mm * dpi / 25.4).round() as u32;

    if width_px == 0 || height_px == 0 {
        return Err(Error::new(E_INVALIDARG, "invalid dimensions"));
    }

    let _token = GdiplusToken::init()?;

    // PixelFormat32bppARGB = 0x0026200A
    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026200A;

    unsafe {
        // Create GDI+ bitmap
        let mut bitmap: *mut GpBitmap = std::ptr::null_mut();
        let status = GdipCreateBitmapFromScan0(
            width_px as i32,
            height_px as i32,
            0,
            PIXEL_FORMAT_32BPP_ARGB,
            None,
            &mut bitmap,
        );
        if status.0 != 0 || bitmap.is_null() {
            return Err(Error::new(E_FAIL, "GdipCreateBitmap failed"));
        }
        let _bitmap_guard = BitmapGuard(bitmap);

        // Get graphics context
        let mut graphics: *mut GpGraphics = std::ptr::null_mut();
        let status = GdipGetImageGraphicsContext(bitmap as *mut _, &mut graphics);
        if status.0 != 0 || graphics.is_null() {
            return Err(Error::new(E_FAIL, "GdipGetImageGraphicsContext failed"));
        }
        let _graphics_guard = GraphicsGuard(graphics);

        // Set rendering quality (optimized for thermal printers)
        GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
        GdipSetTextRenderingHint(graphics, TextRenderingHintSingleBitPerPixelGridFit);
        GdipSetInterpolationMode(graphics, InterpolationModeHighQualityBicubic);

        // Apply scaling if DPI is not standard (e.g. super-sampling)
        // We render at high DPI, so we need to scale the drawing commands
        // Standard GDI+ UnitPixel assumes 96 DPI logic usually, but here we are drawing to a bitmap
        // The bitmap size is calculated based on DPI.
        // However, the coordinates in template are in pixels relative to... what?
        // Let's assume template coordinates are based on a reference DPI (e.g. 203 DPI or screen DPI)
        // Actually, looking at default template: width 40mm, fields x=8.0...
        // If x=8.0 is pixels, that's very small.
        // If x=8.0 is mm, then we need to convert to pixels.
        // Wait, let's check TextField struct: pub x: f32.
        // In default template: width_mm: 40.0. Field x: 8.0, width: 304.0.
        // 304.0 width for 40mm width?
        // 40mm at 203 DPI is approx 320 pixels.
        // So the template coordinates seem to be in PIXELS at ~203 DPI.

        // If we render at `dpi`, we need to scale the graphics context
        // because the template coordinates are fixed (likely designed for 203 DPI).
        // The scale factor is dpi / 203.0

        let base_dpi = 203.0;
        let scale_factor = dpi / base_dpi;

        use windows::Win32::Graphics::GdiPlus::{GdipScaleWorldTransform, MatrixOrderPrepend};
        // OrderMatrixPrepend = 0, but using the enum variant is cleaner if available,
        // or just casting 0 as i32 if the enum isn't easily accessible in windows crate version
        GdipScaleWorldTransform(graphics, scale_factor, scale_factor, MatrixOrderPrepend);

        // Calculate padding in base pixels (203 DPI) to be added to field coordinates
        // We apply padding manually to coordinates instead of using WorldTransform
        // to ensure consistent behavior and avoid matrix order confusion.
        let padding_x = (template.padding_mm_x * base_dpi / 25.4).round();
        let padding_y = (template.padding_mm_y * base_dpi / 25.4).round();

        // Fill background with white
        let mut white_brush: *mut windows::Win32::Graphics::GdiPlus::GpSolidFill =
            std::ptr::null_mut();
        GdipCreateSolidFill(0xFFFFFFFF, &mut white_brush);
        if !white_brush.is_null() {
            // Background covers the entire bitmap (paper), starting from (0,0)
            GdipFillRectangle(
                graphics,
                white_brush as *mut _,
                0.0,
                0.0,
                width_px as f32 / scale_factor, // Un-scale the width/height for filling
                height_px as f32 / scale_factor,
            );
            GdipDeleteBrush(white_brush as *mut _);
        }

        // Create font family
        let font_family_name = to_wide("Microsoft YaHei");
        let mut font_family: *mut GpFontFamily = std::ptr::null_mut();
        GdipCreateFontFamilyFromName(
            PCWSTR::from_raw(font_family_name.as_ptr()),
            std::ptr::null_mut(),
            &mut font_family,
        );

        if font_family.is_null() {
            // Fallback to Arial if YaHei not available
            let fallback_name = to_wide("Arial");
            GdipCreateFontFamilyFromName(
                PCWSTR::from_raw(fallback_name.as_ptr()),
                std::ptr::null_mut(),
                &mut font_family,
            );
        }

        if font_family.is_null() {
            return Err(Error::new(E_FAIL, "Failed to create font family"));
        }
        let _font_family_guard = FontFamilyGuard(font_family);

        // Create black brush for text
        let mut black_brush: *mut windows::Win32::Graphics::GdiPlus::GpSolidFill =
            std::ptr::null_mut();
        GdipCreateSolidFill(0xFF000000, &mut black_brush);
        let _black_brush_guard = BrushGuard(black_brush as *mut _);

        // Render all fields dynamically based on template
        for field in &template.fields {
            match field {
                TemplateField::Text(text_field) => {
                    // Render template with data
                    let rendered_text = render_template(&text_field.template, data);

                    // Create a temporary field with adjusted coordinates
                    let mut adj_field = text_field.clone();
                    adj_field.x += padding_x;
                    adj_field.y += padding_y;

                    draw_rect_string(
                        graphics,
                        font_family,
                        &rendered_text,
                        &adj_field,
                        black_brush as *mut _,
                    );
                }
                TemplateField::Image(img_field) => {
                    // Extract image data from JSON
                    if let Some((img_data, img_w, img_h)) =
                        extract_image_data(data, &img_field.data_key)
                    {
                        // Create a temporary field with adjusted coordinates
                        let mut adj_field = img_field.clone();
                        adj_field.x += padding_x;
                        adj_field.y += padding_y;

                        draw_image(graphics, &img_data, img_w, img_h, &adj_field)?;
                    }
                }
                TemplateField::Separator(sep_field) => {
                    // Draw separator line
                    let mut pen: *mut GpPen = std::ptr::null_mut();
                    GdipCreatePen1(0xFF000000, 1.0, UnitPixel, &mut pen);
                    if !pen.is_null() {
                        let x_start = sep_field.x_start.unwrap_or(8.0) + padding_x;
                        let x_end = sep_field
                            .x_end
                            .unwrap_or((width_px as f32 / scale_factor) - 8.0)
                            + padding_x; // Recalculate default end based on unscaled width
                        let y = sep_field.y + padding_y;

                        GdipDrawLine(graphics, pen, x_start, y, x_end, y);
                        GdipDeletePen(pen);
                    }
                }
            }
        }

        // Extract ARGB pixel data from bitmap
        extract_bitmap_data(bitmap, width_px, height_px)
    }
}

// Helper function to draw image in a rectangle
unsafe fn draw_image(
    graphics: *mut GpGraphics,
    rgba_data: &[u8],
    img_width: u32,
    img_height: u32,
    field: &ImageField,
) -> WinResult<()> {
    use windows::Win32::Graphics::GdiPlus::GdipCreateBitmapFromScan0;

    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026200A;

    // Create GDI+ bitmap from RGBA data
    let mut img_bitmap: *mut GpBitmap = std::ptr::null_mut();

    // Convert RGBA to ARGB for GDI+
    let mut argb_data = vec![0u8; (img_width * img_height * 4) as usize];
    for i in 0..(img_width * img_height) as usize {
        let src_idx = i * 4;
        let dst_idx = i * 4;
        argb_data[dst_idx] = rgba_data[src_idx + 2]; // B
        argb_data[dst_idx + 1] = rgba_data[src_idx + 1]; // G
        argb_data[dst_idx + 2] = rgba_data[src_idx]; // R
        argb_data[dst_idx + 3] = rgba_data[src_idx + 3]; // A
    }

    let stride = (img_width * 4) as i32;
    let status = GdipCreateBitmapFromScan0(
        img_width as i32,
        img_height as i32,
        stride,
        PIXEL_FORMAT_32BPP_ARGB,
        Some(argb_data.as_ptr()),
        &mut img_bitmap,
    );

    if status.0 != 0 || img_bitmap.is_null() {
        return Err(Error::new(E_FAIL, "Failed to create image bitmap"));
    }

    let _img_guard = BitmapGuard(img_bitmap);

    // Calculate destination rectangle
    let (dest_width, dest_height) = if field.maintain_aspect_ratio {
        let aspect = img_width as f32 / img_height as f32;
        let field_aspect = field.width / field.height;

        if aspect > field_aspect {
            // Image is wider than field
            let w = field.width;
            let h = field.width / aspect;
            (w, h)
        } else {
            // Image is taller than field
            let h = field.height;
            let w = field.height * aspect;
            (w, h)
        }
    } else {
        (field.width, field.height)
    };

    // Center the image in the field
    let dest_x = field.x + (field.width - dest_width) / 2.0;
    let dest_y = field.y + (field.height - dest_height) / 2.0;

    // Draw the image
    use windows::Win32::Graphics::GdiPlus::GdipDrawImageRect;
    let status = GdipDrawImageRect(
        graphics,
        img_bitmap as *mut _,
        dest_x,
        dest_y,
        dest_width,
        dest_height,
    );

    if status.0 != 0 {
        return Err(Error::new(E_FAIL, "Failed to draw image"));
    }

    Ok(())
}

// Helper function to draw text in a rectangle with alignment support
unsafe fn draw_rect_string(
    graphics: *mut GpGraphics,
    font_family: *mut GpFontFamily,
    text: &str,
    field: &TextField,
    brush: *mut GpBrush,
) {
    use windows::Win32::Graphics::GdiPlus::{
        GdipCreateStringFormat, GdipDeleteStringFormat, GdipSetStringFormatAlign,
        GdipSetStringFormatLineAlign, StringAlignmentCenter, StringAlignmentFar,
        StringAlignmentNear,
    };

    // Create font with specified style
    let font_style = match field.style {
        TextStyle::Regular => FontStyleRegular.0,
        TextStyle::Bold => FontStyleBold.0,
    };

    // Create font family
    // Use field-specific font family if provided, otherwise use default passed in font_family arg
    let mut field_font_family: *mut GpFontFamily = std::ptr::null_mut();
    let family_name = field.font_family.as_deref().unwrap_or("Microsoft YaHei");
    let wide_family_name = to_wide(family_name);

    GdipCreateFontFamilyFromName(
        PCWSTR::from_raw(wide_family_name.as_ptr()),
        std::ptr::null_mut(),
        &mut field_font_family,
    );

    // If custom font fails, try fallback or use the default one passed in
    if field_font_family.is_null() {
        // Fallback to the default font family passed to function (which is likely YaHei or Arial)
        // We need to clone it because GdipCreateFont takes a pointer to it
        // However, GDI+ font creation is a bit tricky with raw pointers.
        // Let's just create a new font family from "Arial" as a safe fallback if the requested one failed
        let fallback_name = to_wide("Arial");
        GdipCreateFontFamilyFromName(
            PCWSTR::from_raw(fallback_name.as_ptr()),
            std::ptr::null_mut(),
            &mut field_font_family,
        );
    }

    // If still null, we have a problem, but let's try to proceed with the passed in one if available
    // For now, let's assume one of them worked.
    // Ideally we should use the `font_family` arg if field specific one is None.
    // But `font_family` arg is a *mut GpFontFamily, we can't easily clone it in raw GDI+.
    // So logic:
    // 1. If field.font_family is set, create new family.
    // 2. If not set, use the `font_family` argument passed to function.

    let target_font_family = if let Some(_name) = &field.font_family {
        // We created `field_font_family` above
        if !field_font_family.is_null() {
            field_font_family
        } else {
            font_family // Fallback to default
        }
    } else {
        font_family // Use default
    };

    let mut font: *mut GpFont = std::ptr::null_mut();
    GdipCreateFont(
        target_font_family,
        field.font_size,
        font_style,
        UnitPixel,
        &mut font,
    );

    // Clean up if we created a local font family
    if let Some(_) = &field.font_family {
        if !field_font_family.is_null() {
            GdipDeleteFontFamily(field_font_family);
        }
    }

    if font.is_null() {
        return;
    }

    // Create string format for alignment
    let mut string_format: *mut GpStringFormat = std::ptr::null_mut();
    GdipCreateStringFormat(0, 0, &mut string_format);

    if !string_format.is_null() {
        let h_align = match field.align {
            TextAlign::Left => StringAlignmentNear,
            TextAlign::Center => StringAlignmentCenter,
            TextAlign::Right => StringAlignmentFar,
        };

        GdipSetStringFormatAlign(string_format, h_align);
        GdipSetStringFormatLineAlign(string_format, StringAlignmentNear); // Top-aligned vertically
    }

    // Convert text to wide string
    let wide_text = to_wide(text);

    // Define rectangle
    let rectf = windows::Win32::Graphics::GdiPlus::RectF {
        X: field.x,
        Y: field.y,
        Width: field.width,
        Height: field.height,
    };

    // Draw string
    GdipDrawString(
        graphics,
        PCWSTR::from_raw(wide_text.as_ptr()),
        wide_text.len() as i32 - 1, // exclude null terminator
        font,
        &rectf,
        string_format,
        brush,
    );

    // Cleanup
    if !string_format.is_null() {
        GdipDeleteStringFormat(string_format);
    }
    GdipDeleteFont(font);
}

// Extract ARGB pixel data from GDI+ bitmap
unsafe fn extract_bitmap_data(
    bitmap: *mut GpBitmap,
    width: u32,
    height: u32,
) -> WinResult<(Vec<u8>, u32, u32)> {
    use windows::Win32::Graphics::GdiPlus::{
        BitmapData, GdipBitmapLockBits, GdipBitmapUnlockBits, ImageLockModeRead,
    };

    const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026200A;

    let mut bmp_data = BitmapData {
        Width: width,
        Height: height,
        Stride: 0,
        PixelFormat: PIXEL_FORMAT_32BPP_ARGB,
        Scan0: std::ptr::null_mut(),
        Reserved: 0,
    };

    let rectf = windows::Win32::Graphics::GdiPlus::Rect {
        X: 0,
        Y: 0,
        Width: width as i32,
        Height: height as i32,
    };

    let status = GdipBitmapLockBits(
        bitmap as *mut _,
        &rectf as *const _,
        ImageLockModeRead.0 as u32,
        PIXEL_FORMAT_32BPP_ARGB,
        &mut bmp_data,
    );

    if status.0 != 0 {
        return Err(Error::new(E_FAIL, "GdipBitmapLockBits failed"));
    }

    // Copy pixel data (ARGB format)
    let stride = bmp_data.Stride.abs() as usize;
    let data_size = stride * height as usize;
    let mut pixels = vec![0u8; data_size];

    std::ptr::copy_nonoverlapping(bmp_data.Scan0 as *const u8, pixels.as_mut_ptr(), data_size);

    GdipBitmapUnlockBits(bitmap as *mut _, &mut bmp_data);

    // Convert ARGB to RGBA premultiplied
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let src_idx = y * stride + x * 4;
            let dst_idx = (y * width as usize + x) * 4;

            let b = pixels[src_idx];
            let g = pixels[src_idx + 1];
            let r = pixels[src_idx + 2];
            let a = pixels[src_idx + 3];

            // Premultiply alpha
            let alpha = a as f32 / 255.0;
            rgba[dst_idx] = (r as f32 * alpha) as u8;
            rgba[dst_idx + 1] = (g as f32 * alpha) as u8;
            rgba[dst_idx + 2] = (b as f32 * alpha) as u8;
            rgba[dst_idx + 3] = a;
        }
    }

    Ok((rgba, width, height))
}

// RAII guards for GDI+ resources
struct BitmapGuard(*mut GpBitmap);
impl Drop for BitmapGuard {
    fn drop(&mut self) {
        unsafe { GdipDisposeImage(self.0 as *mut _) };
    }
}

struct GraphicsGuard(*mut GpGraphics);
impl Drop for GraphicsGuard {
    fn drop(&mut self) {
        unsafe { GdipDeleteGraphics(self.0) };
    }
}

struct FontFamilyGuard(*mut GpFontFamily);
impl Drop for FontFamilyGuard {
    fn drop(&mut self) {
        unsafe { GdipDeleteFontFamily(self.0) };
    }
}

struct BrushGuard(*mut GpBrush);
impl Drop for BrushGuard {
    fn drop(&mut self) {
        unsafe { GdipDeleteBrush(self.0) };
    }
}

/// Apply threshold (binarization) to improve thermal printer output
pub fn apply_threshold(pixels: &mut [u8], width: u32, height: u32, threshold: u8) {
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 3 >= pixels.len() {
                continue;
            }

            // Calculate grayscale using standard formula
            let r = pixels[idx];
            let g = pixels[idx + 1];
            let b = pixels[idx + 2];
            let gray = (r as f32 * 0.3 + g as f32 * 0.59 + b as f32 * 0.11) as u8;

            // Apply threshold
            let value = if gray < threshold { 0 } else { 255 };
            pixels[idx] = value; // R
            pixels[idx + 1] = value; // G
            pixels[idx + 2] = value; // B
                                     // Alpha channel remains unchanged
        }
    }
}

/// High-level function to render and print a label with JSON data
///
/// # Arguments
/// * `data` - JSON value containing all fields (e.g., {"order_id": "A123", "item_name": "Milk Tea", ...})
/// * `template` - Optional template configuration (uses default if None)
/// * `options` - Print options (printer, paper size, etc.)
///
/// # Example
/// ```rust
/// let data = serde_json::json!({
///     "order_id": "A123",
///     "item_name": "珍珠奶茶",
///     "specs": "大杯,全糖",
///     "price": "€5.50",
///     "time": "14:30"
/// });
/// render_and_print_label(&data, None, &options)?;
/// ```
pub fn render_and_print_label(
    data: &serde_json::Value,
    template: Option<&LabelTemplate>,
    options: &PrintOptions,
) -> WinResult<()> {
    // Use provided template or create default based on label size
    let mut tmpl = match template {
        Some(t) => t.clone(),
        None => LabelTemplate::default(),
    };

    // Override dimensions from options if provided (handles auto-expanded paper size)
    if options.label_width_mm > 0.0 {
        tmpl.width_mm = options.label_width_mm;
    }
    if options.label_height_mm > 0.0 {
        tmpl.height_mm = options.label_height_mm;
    }

    // Render at high DPI for better quality (300 DPI for thermal printers)
    // Update: User reports 300 DPI causes ratio issues on some printers.
    // 203.2 DPI (8 dots/mm) is standard for many thermal printers (e.g. 40mm = 320 dots).
    // Using 203.0 ensures 1:1 mapping on 203 DPI printers and proper scaling on others.
    // Allow overriding DPI via options
    let target_dpi = options.override_dpi.unwrap_or(203.0);

    // Super-sampling: Render at 2x resolution then downscale
    // This improves anti-aliasing quality, especially for small text and barcodes
    let super_sample_scale = 2.0;
    let render_dpi = target_dpi * super_sample_scale;

    let (high_res_rgba, high_res_width, high_res_height) =
        render_label_gdiplus(data, &tmpl, render_dpi)?;

    // Downscale to target resolution
    let target_width = (high_res_width as f32 / super_sample_scale).round() as u32;
    let target_height = (high_res_height as f32 / super_sample_scale).round() as u32;

    let img_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        high_res_width,
        high_res_height,
        high_res_rgba,
    )
    .ok_or_else(|| {
        windows::core::Error::new(
            windows::Win32::Foundation::E_FAIL,
            "Failed to create image buffer",
        )
    })?;

    let dynamic_image = image::DynamicImage::ImageRgba8(img_buffer);
    let resized_img = dynamic_image.resize(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut rgba_data = resized_img.to_rgba8().into_raw();
    let width = resized_img.width();
    let height = resized_img.height();

    // Apply threshold for sharper output on thermal printers
    apply_threshold(&mut rgba_data, width, height, 185);

    // DEBUG: Save to D:/debug.png
    // Ignore errors for debug saving
    let _ = image::save_buffer(
        "D:/debug.png",
        &rgba_data,
        width,
        height,
        image::ColorType::Rgba8,
    );

    // Print the rendered label
    print_rgba_premul(&rgba_data, width, height, options)
}
