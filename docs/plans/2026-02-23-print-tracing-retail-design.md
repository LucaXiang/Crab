# 打印全链路 tracing + 零售叫号 + QR 移除

**日期**: 2026-02-23
**目标**: 公测前确保票据/标签打印链路完全可调试，零售订单正确显示叫号

## 变更点

### 1. 全链路 tracing::debug

在 Windows 驱动打印路径的每个关键步骤加 `tracing::debug`。

**票据打印** (`red_coral/src-tauri/`):
- `commands/printer.rs::print_receipt` — 入口: order_id, printer_name, items 数量
- `utils/printing.rs::print_receipt` — resolve 后: 实际打印机名
- `utils/printing.rs::print_receipt` — logo: 有无, 路径, 字节数
- `utils/printing.rs::print_receipt` — render 后: 渲染字节数
- `utils/printing.rs::print_receipt` — GBK 编码后: 总字节数
- `utils/printing.rs::print_receipt` — print_sync 前后

**标签打印** (`red_coral/src-tauri/`):
- `commands/printer.rs::print_label` — 入口: data keys, template, 尺寸
- `utils/printing.rs::print_label` — 解析: template 字段数, options
- `label_printer.rs::render_and_print_label` — 模板尺寸, DPI, 超采样
- `label_printer.rs::render_label_gdiplus` — GDI+ bitmap 像素, scale_factor
- `label_printer.rs::render_label_gdiplus` — 每个字段渲染结果
- `label_printer.rs::render_and_print_label` — 降采样后最终像素
- `label_printer.rs::print_rgba_premul` — DC/DPI/fit 计算
- `label_printer.rs::print_rgba_premul` — 每页 StartPage/EndPage

**crab-printer** (`crab-printer/src/printer.rs`):
- `write_raw` — 入口: printer name, 字节数
- `check_online` — 结果
- `OpenPrinterW` — 成功
- `WritePrinter` — written 字节数

### 2. 零售订单叫号

**数据流**: OrderSnapshot.queue_number → ReceiptData.queue_number → receipt_renderer

- `ReceiptData` (Rust + TS) 新增 `queue_number: Option<u32>`
- `receiptBuilder.ts` 传入 `order.queue_number`
- `receipt_renderer.rs`: 有 queue_number 时显示 `PEDIDO: #042`，无时保持 `MESA: xxx`

### 3. 移除 QR 码

`receipt_renderer.rs` 移除 QR 渲染代码，`qr_data` 字段保留不渲染。

## 涉及文件

| 文件 | 变更 |
|------|------|
| `crab-printer/src/printer.rs` | +debug 日志 (write_raw) |
| `red_coral/src-tauri/src/commands/printer.rs` | +debug 日志 |
| `red_coral/src-tauri/src/utils/printing.rs` | +debug 日志 |
| `red_coral/src-tauri/src/utils/label_printer.rs` | +debug 日志 |
| `red_coral/src-tauri/src/api/printers.rs` | +queue_number 字段 |
| `red_coral/src-tauri/src/utils/receipt_renderer.rs` | 叫号显示 + 移除 QR |
| `red_coral/src/infrastructure/print/printService.ts` | +queue_number 字段 |
| `red_coral/src/core/services/order/receiptBuilder.ts` | 传入 queue_number |
