# 打印系统公测准备 — 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 公测前确保票据/标签打印全链路可调试，edge-server 支持 GDI+ 标签打印，零售订单显示叫号

**Architecture:** 将 GDI+ 标签渲染从 red_coral/src-tauri 移入 crab-printer crate (#[cfg(windows)])，edge-server 的 PrintExecutor 补全标签打印调用，全链路加 tracing::debug

**Tech Stack:** Rust, crab-printer (ESC/POS + GDI+), Windows GDI+, tracing

---

## 前置知识

### 文件位置

| 文件 | 用途 |
|------|------|
| `crab-printer/src/lib.rs` | crab-printer 公开 API |
| `crab-printer/src/printer.rs` | NetworkPrinter + WindowsPrinter |
| `red_coral/src-tauri/src/utils/label_printer.rs` | GDI+ 标签渲染 (待移入 crab-printer) |
| `red_coral/src-tauri/src/utils/printing.rs` | Tauri 打印分发层 |
| `red_coral/src-tauri/src/commands/printer.rs` | Tauri 命令入口 |
| `red_coral/src-tauri/src/api/printers.rs` | ReceiptData 类型定义 |
| `red_coral/src-tauri/src/utils/receipt_renderer.rs` | 票据 ESC/POS 渲染 |
| `edge-server/src/printing/executor.rs` | PrintExecutor (厨房单打印) |
| `edge-server/src/printing/worker.rs` | KitchenPrintWorker |
| `edge-server/src/printing/renderer.rs` | KitchenTicketRenderer |
| `edge-server/src/printing/service.rs` | KitchenPrintService |
| `edge-server/src/printing/types.rs` | PrintItemContext, LabelPrintRecord |
| `shared/src/models/label_template.rs` | LabelTemplate DB 模型 |
| `red_coral/src/infrastructure/print/printService.ts` | 前端打印服务 |
| `red_coral/src/core/services/order/receiptBuilder.ts` | 收据数据构建 |

### 当前缺口

1. **PrintExecutor 不处理 label_destinations** — 只打厨房单，不打标签
2. **GDI+ 代码困在 red_coral** — edge-server 无法使用
3. **ReceiptData 缺少 queue_number** — 零售订单显示桌号而非叫号
4. **receipt_renderer 输出 QR** — 现阶段不需要
5. **全链路缺少 debug 日志** — 出问题无法定位

---

## Task 1: 移动 GDI+ 标签渲染到 crab-printer

**Files:**
- Create: `crab-printer/src/label.rs`
- Modify: `crab-printer/src/lib.rs`
- Modify: `crab-printer/Cargo.toml`
- Modify: `red_coral/src-tauri/src/utils/label_printer.rs` (改为 re-export)
- Modify: `red_coral/src-tauri/Cargo.toml` (如需)

**Step 1:** 将 `red_coral/src-tauri/src/utils/label_printer.rs` 中以下内容移入 `crab-printer/src/label.rs`（整个文件用 `#[cfg(windows)]` 包裹）:

- 类型: `FitMode`, `Rotation`, `PrintOptions`, `TextAlign`, `TextStyle`, `TextField`, `ImageField`, `SeparatorField`, `TemplateField`, `LabelTemplate`
- 函数: `render_template()`, `extract_image_data()`, `decode_base64_image()`, `render_label_gdiplus()`, `render_and_print_label()`, `print_rgba_premul()`, `apply_threshold()`
- 辅助: `create_custom_printer_dc()`, `mm_to_px()`, `fit_rect()`, `build_bgra_opaque_on_white()`, `to_wide()`, `draw_rect_string()`, `draw_image()`, `extract_bitmap_data()`
- Guards: `HdcGuard`, `PrinterGuard`, `DocGuard`, `GdiplusToken`, `BitmapGuard`, `GraphicsGuard`, `FontFamilyGuard`, `BrushGuard`

**Step 2:** 更新 `crab-printer/Cargo.toml`:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_Graphics_GdiPlus",
    "Win32_Graphics_Printing",
    "Win32_Storage_Xps",
] }
base64 = "0.22"

[dependencies]
image = { version = "0.25", optional = true }

[features]
default = ["image"]
```

注意: 检查 red_coral/src-tauri/Cargo.toml 中 windows crate 的实际版本和 features，以确保一致。

**Step 3:** 更新 `crab-printer/src/lib.rs`:

```rust
#[cfg(windows)]
pub mod label;
```

**Step 4:** 更新 `red_coral/src-tauri/src/utils/label_printer.rs`:

```rust
// GDI+ label rendering moved to crab-printer
pub use crab_printer::label::*;
```

**Step 5:** `red_coral/src-tauri/src/utils/printing.rs` 中的 `use crate::utils::label_printer::{LabelTemplate, PrintOptions}` 改为 `use crab_printer::label::{LabelTemplate, PrintOptions}`

**Step 6:** 检查 `resolve_printer()` — label_printer.rs 中的 `print_rgba_premul()` 调用了 `crate::utils::printing::resolve_printer()`。移入 crab-printer 后需要改为使用 `crab_printer::WindowsPrinter::resolve()`。

**Step 7:** 编译验证

```bash
cargo check -p crab-printer
cargo check -p red_coral  # 或 cd red_coral && cargo check -p red-coral-tauri
```

**Step 8: Commit**

```bash
git add crab-printer/src/label.rs crab-printer/src/lib.rs crab-printer/Cargo.toml \
       red_coral/src-tauri/src/utils/label_printer.rs red_coral/src-tauri/src/utils/printing.rs
git commit -m "refactor: move GDI+ label rendering to crab-printer"
```

---

## Task 2: edge-server PrintExecutor 支持标签打印

**Files:**
- Modify: `edge-server/src/printing/executor.rs`
- Modify: `edge-server/src/printing/worker.rs`
- Modify: `edge-server/src/printing/types.rs`
- Modify: `edge-server/Cargo.toml` (确保 crab-printer 依赖包含 image feature)

**Step 1:** 在 `executor.rs` 添加 `print_label_records()` 方法:

```rust
/// 执行标签打印 (Windows GDI+ 驱动)
#[cfg(windows)]
#[instrument(skip(self, records, destinations, template), fields(record_count = records.len()))]
pub async fn print_label_records(
    &self,
    records: &[super::types::LabelPrintRecord],
    destinations: &HashMap<String, PrintDestination>,
    template: &crab_printer::label::LabelTemplate,
    table_name: Option<&str>,
    queue_number: Option<u32>,
) -> PrintExecutorResult<()> {
    use crab_printer::label::{PrintOptions, render_and_print_label};

    for record in records {
        // 按 label_destinations 分组，找到第一个可用的目的地
        for dest_id in &record.context.label_destinations {
            let dest = match destinations.get(dest_id) {
                Some(d) => d,
                None => {
                    warn!(dest_id = %dest_id, "Label destination not found, skipping");
                    continue;
                }
            };

            // 找活跃的驱动打印机
            let printer = dest.printers.iter()
                .filter(|p| p.is_active && p.connection == "driver")
                .min_by_key(|p| p.priority);

            let Some(printer) = printer else {
                warn!(dest = %dest.name, "No active driver printer for label destination");
                continue;
            };

            let driver_name = match &printer.driver_name {
                Some(name) => name.clone(),
                None => {
                    warn!("Label printer has no driver_name");
                    continue;
                }
            };

            // 构建标签数据 JSON
            let data = build_label_data(&record.context, table_name, queue_number);

            let options = PrintOptions {
                printer_name: Some(driver_name),
                doc_name: "label".to_string(),
                label_width_mm: template.width_mm,
                label_height_mm: template.height_mm,
                copies: 1,
                fit: crab_printer::label::FitMode::Contain,
                rotate: crab_printer::label::Rotation::R0,
                override_dpi: None,
            };

            // 在 blocking task 中执行 GDI+ 渲染和打印
            let template_clone = template.clone();
            let result = tokio::task::spawn_blocking(move || {
                render_and_print_label(&data, Some(&template_clone), &options)
            }).await;

            match result {
                Ok(Ok(())) => {
                    info!(
                        record_id = %record.id,
                        product = %record.context.product_name,
                        index = ?record.context.index,
                        "Label printed"
                    );
                    break; // 成功，不再尝试其他目的地
                }
                Ok(Err(e)) => {
                    error!(
                        record_id = %record.id,
                        dest = %dest.name,
                        error = %e,
                        "Label print failed"
                    );
                }
                Err(e) => {
                    error!(
                        record_id = %record.id,
                        error = %e,
                        "Label print task panicked"
                    );
                }
            }
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub async fn print_label_records(
    &self,
    _records: &[super::types::LabelPrintRecord],
    _destinations: &HashMap<String, PrintDestination>,
    _template: &crab_printer::label::LabelTemplate,
    _table_name: Option<&str>,
    _queue_number: Option<u32>,
) -> PrintExecutorResult<()> {
    warn!("Label printing requires Windows (GDI+)");
    Ok(())
}
```

**Step 2:** 添加 `build_label_data()` 辅助函数 (在 executor.rs 中):

```rust
/// 从 PrintItemContext 构建标签打印的 JSON 数据
fn build_label_data(
    ctx: &super::types::PrintItemContext,
    table_name: Option<&str>,
    queue_number: Option<u32>,
) -> serde_json::Value {
    let time = chrono::Local::now().format("%H:%M").to_string();

    serde_json::json!({
        "product_name": ctx.product_name,
        "kitchen_name": ctx.kitchen_name,
        "item_name": ctx.kitchen_name,
        "category_name": ctx.category_name,
        "spec_name": ctx.spec_name.as_deref().unwrap_or(""),
        "specs": ctx.spec_name.as_deref().unwrap_or(""),
        "options": ctx.options.join(", "),
        "quantity": ctx.quantity,
        "index": ctx.index.as_deref().unwrap_or(""),
        "note": ctx.note.as_deref().unwrap_or(""),
        "external_id": ctx.external_id.unwrap_or(0),
        "table_name": table_name.unwrap_or(""),
        "queue_number": queue_number.map(|n| format!("#{:03}", n)).unwrap_or_default(),
        "order_id": "",  // Will be set by caller if needed
        "time": time,
    })
}
```

**Step 3:** 更新 `worker.rs` — `handle_items_added()` 在创建 label records 后调用标签打印:

在 `handle_items_added` 中，成功创建 kitchen order 后：

```rust
// 执行标签打印
self.execute_label_print(
    &event.order_id,
    &kitchen_order_id,
    executor,
    table_name.as_deref(),
    queue_number,
).await;
```

新增 `execute_label_print()` 方法:

```rust
async fn execute_label_print(
    &self,
    order_id: &str,
    kitchen_order_id: &str,
    executor: &PrintExecutor,
    table_name: Option<&str>,
    queue_number: Option<u32>,
) {
    // 获取该 kitchen order 关联的 label records
    let records = match self.kitchen_print_service.get_label_records_for_kitchen_order(kitchen_order_id) {
        Ok(r) if !r.is_empty() => r,
        Ok(_) => return, // 没有标签记录
        Err(e) => {
            tracing::error!(error = ?e, "Failed to load label records");
            return;
        }
    };

    // 加载默认标签模板
    let template = match self.load_label_template().await {
        Some(t) => t,
        None => {
            tracing::warn!("No label template found, using default");
            crab_printer::label::LabelTemplate::default()
        }
    };

    // 加载打印目的地
    let destinations = match print_destination::find_all(&self.pool).await {
        Ok(d) => d.into_iter().map(|d| (d.id.to_string(), d)).collect(),
        Err(e) => {
            tracing::error!(error = ?e, "Failed to load print destinations");
            return;
        }
    };

    if let Err(e) = executor.print_label_records(
        &records, &destinations, &template, table_name, queue_number,
    ).await {
        tracing::error!(
            order_id = %order_id,
            error = %e,
            "Failed to print labels"
        );
    }
}
```

**注意:** `get_label_records_for_kitchen_order` 方法可能需要新增 — 目前 service 只有 `get_label_records_for_order(order_id)`。可以先用 `get_label_records_for_order` 过滤 `kitchen_order_id`。

**注意:** `load_label_template()` 需要从 DB 加载默认模板，然后转换为 `crab_printer::label::LabelTemplate` 格式。需要写一个转换函数 (`shared::models::LabelTemplate` → `crab_printer::label::LabelTemplate`)。

**Step 4:** 添加 DB 模板 → crab_printer 模板的转换函数。

在 `edge-server/src/printing/executor.rs` 或新文件 `edge-server/src/printing/template_convert.rs`:

```rust
/// 将 DB LabelTemplate 转换为 crab_printer::label::LabelTemplate
pub fn convert_label_template(
    db_template: &shared::models::LabelTemplate,
) -> crab_printer::label::LabelTemplate {
    let fields = db_template.fields.iter()
        .filter(|f| f.visible)
        .filter_map(|f| convert_label_field(f))
        .collect();

    crab_printer::label::LabelTemplate {
        width_mm: db_template.width_mm.unwrap_or(db_template.width),
        height_mm: db_template.height_mm.unwrap_or(db_template.height),
        padding_mm_x: db_template.padding_mm_x.unwrap_or(0.0),
        padding_mm_y: db_template.padding_mm_y.unwrap_or(0.0),
        fields,
    }
}

fn convert_label_field(f: &shared::models::LabelField) -> Option<crab_printer::label::TemplateField> {
    use shared::models::LabelFieldType;
    match f.field_type {
        LabelFieldType::Text | LabelFieldType::Datetime | LabelFieldType::Price | LabelFieldType::Counter => {
            Some(crab_printer::label::TemplateField::Text(crab_printer::label::TextField {
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                font_size: f.font_size as f32,
                font_family: f.font_family.clone(),
                style: if f.font_weight.as_deref() == Some("bold") {
                    crab_printer::label::TextStyle::Bold
                } else {
                    crab_printer::label::TextStyle::Regular
                },
                align: match f.alignment {
                    Some(shared::models::LabelFieldAlignment::Center) => crab_printer::label::TextAlign::Center,
                    Some(shared::models::LabelFieldAlignment::Right) => crab_printer::label::TextAlign::Right,
                    _ => crab_printer::label::TextAlign::Left,
                },
                template: f.template.clone().or_else(|| {
                    f.data_key.as_ref().map(|k| format!("{{{}}}", k))
                }).unwrap_or_default(),
            }))
        }
        LabelFieldType::Image | LabelFieldType::Barcode | LabelFieldType::Qrcode => {
            Some(crab_printer::label::TemplateField::Image(crab_printer::label::ImageField {
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                maintain_aspect_ratio: f.maintain_aspect_ratio.unwrap_or(true),
                data_key: f.data_key.clone().unwrap_or_else(|| f.name.clone()),
            }))
        }
        LabelFieldType::Separator => {
            Some(crab_printer::label::TemplateField::Separator(crab_printer::label::SeparatorField {
                y: f.y,
                x_start: Some(f.x),
                x_end: Some(f.x + f.width),
            }))
        }
    }
}
```

**Step 5:** Worker 中还需要获取 queue_number — 从 OrderSnapshot 获取:

在 `handle_items_added` 中，已经有获取 table_name 的代码，扩展为同时获取 queue_number:

```rust
let (table_name, queue_number) = self
    .orders_manager
    .get_snapshot(&event.order_id)
    .ok()
    .flatten()
    .map(|s| (s.table_name, s.queue_number))
    .unwrap_or((None, None));
```

**Step 6:** 编译验证

```bash
cargo check -p edge-server
```

**Step 7: Commit**

```bash
git commit -m "feat(printing): edge-server label printing via GDI+ driver"
```

---

## Task 3: 全链路 tracing::debug

**Files:**
- Modify: `crab-printer/src/printer.rs` (WindowsPrinter::write_raw)
- Modify: `crab-printer/src/label.rs` (GDI+ 渲染链路)
- Modify: `red_coral/src-tauri/src/commands/printer.rs`
- Modify: `red_coral/src-tauri/src/utils/printing.rs`
- Modify: `edge-server/src/printing/executor.rs`
- Modify: `edge-server/src/printing/worker.rs`
- Modify: `edge-server/src/printing/service.rs`

### 3a. crab-printer/src/printer.rs — WindowsPrinter::write_raw

在 `write_raw()` 中加 debug:

```rust
fn write_raw(&self, data: &[u8]) -> PrintResult<()> {
    tracing::debug!(printer = %self.name, bytes = data.len(), "write_raw: start");

    // check_online
    let online = Self::check_online(&self.name).unwrap_or(true);
    tracing::debug!(printer = %self.name, online, "write_raw: check_online");
    if !online { return Err(PrintError::Offline(...)); }

    // OpenPrinterW
    // ... existing code ...
    tracing::debug!(printer = %self.name, "write_raw: OpenPrinterW OK");

    // StartDocPrinterW + StartPagePrinter
    tracing::debug!(printer = %self.name, "write_raw: StartDoc+StartPage OK");

    // WritePrinter
    tracing::debug!(printer = %self.name, written, total = data.len(), "write_raw: WritePrinter done");

    // EndPage + EndDoc + Close
    tracing::debug!(printer = %self.name, "write_raw: EndDoc+Close OK");
}
```

### 3b. crab-printer/src/label.rs — GDI+ 渲染

```rust
// render_and_print_label() 入口
tracing::debug!(
    template_w = tmpl.width_mm, template_h = tmpl.height_mm,
    dpi = target_dpi, super_sample = super_sample_scale,
    "render_and_print_label: start"
);

// render_label_gdiplus() 内
tracing::debug!(width_px, height_px, scale_factor, "render_label_gdiplus: bitmap created");

// 每个字段
tracing::debug!(field_type = ?field, "render_label_gdiplus: rendering field");

// 降采样后
tracing::debug!(width, height, "render_and_print_label: downsampled + threshold applied");

// print_rgba_premul() 内
tracing::debug!(
    printer = %printer, dpi_x, dpi_y,
    target_w, target_h, draw_w, draw_h,
    "print_rgba_premul: DC created, printing"
);
```

### 3c. red_coral 命令层

`commands/printer.rs`:

```rust
pub fn print_receipt(...) {
    tracing::debug!(
        printer_name = ?printer_name,
        order_id = %receipt.order_id,
        items = receipt.items.len(),
        "print_receipt: entry"
    );
    // ...
}

pub fn print_label(request: LabelPrintRequest) {
    tracing::debug!(
        printer_name = ?request.printer_name,
        has_template = request.template.is_some(),
        label_w = ?request.label_width_mm,
        label_h = ?request.label_height_mm,
        "print_label: entry"
    );
    // ...
}
```

### 3d. red_coral printing.rs

```rust
pub fn print_receipt(...) {
    // resolve 后
    tracing::debug!(resolved_printer = %name, "print_receipt: printer resolved");

    // logo 后
    tracing::debug!(has_logo, logo_bytes = logo_bytes_opt.map(|b| b.len()), "print_receipt: logo processed");

    // render 后
    tracing::debug!(rendered_len = output.len(), "print_receipt: receipt rendered");

    // GBK 后
    tracing::debug!(gbk_len = text_bytes.len(), total_len = data.len(), "print_receipt: GBK encoded");

    // 打印前
    tracing::debug!("print_receipt: sending to printer");
}
```

### 3e. edge-server printing

`service.rs::process_items_added()`:

```rust
tracing::debug!(
    kitchen_enabled, label_enabled,
    items_count = items.len(),
    "process_items_added: start"
);

// 每个 item
tracing::debug!(
    product_id = item.id,
    kitchen_dests = kitchen_dests.len(),
    label_dests = label_dests.len(),
    "process_items_added: item context built"
);

// 存储后
tracing::debug!(
    kitchen_items = kitchen_items.len(),
    label_records = label_records.len(),
    "process_items_added: records stored"
);
```

`worker.rs::handle_items_added()`:

```rust
tracing::debug!(
    order_id = %event.order_id,
    table_name = ?table_name,
    queue_number = ?queue_number,
    "handle_items_added: start"
);
```

**Commit:**

```bash
git commit -m "feat: add tracing::debug to print chain for public test"
```

---

## Task 4: 零售订单叫号 (ReceiptData + receipt_renderer)

**Files:**
- Modify: `red_coral/src-tauri/src/api/printers.rs` — ReceiptData 加 queue_number
- Modify: `red_coral/src-tauri/src/utils/receipt_renderer.rs` — 叫号显示 + 移除 QR
- Modify: `red_coral/src/infrastructure/print/printService.ts` — ReceiptData 加 queue_number
- Modify: `red_coral/src/core/services/order/receiptBuilder.ts` — 传入 queue_number

### 4a. Rust ReceiptData

`api/printers.rs`:

```rust
pub struct ReceiptData {
    // ... existing fields ...
    pub queue_number: Option<u32>,  // NEW
    pub qr_data: Option<String>,
}
```

### 4b. receipt_renderer.rs

**叫号显示:** 替换第 83-85 行的 MESA 显示逻辑:

```rust
// 零售叫号 vs 桌台显示
if let Some(qn) = self.receipt.queue_number {
    let pedido_str = format!("PEDIDO: #{:03}", qn);
    b.line_lr(&pedido_str, "Terminal: 01");
} else {
    let zone_str = self.receipt.zone_name.as_deref().unwrap_or("");
    let table_full = format!("{} MESA: {}", zone_str, self.receipt.table_name);
    b.line_lr(table_full.trim(), "Terminal: 01");
}
```

**移除 QR:** 删除第 387-415 行（从 `let qr_payload` 到 `b.write("\x1D\x56\x00")`）。
保留最后的 feed + cut:

```rust
b.write("\n\n\n");
b.write("\x1D\x56\x00"); // cut
```

### 4c. TypeScript ReceiptData

`printService.ts`:

```typescript
export interface ReceiptData {
  // ... existing fields ...
  queue_number: number | null;  // NEW
  qr_data: string | null;
}
```

### 4d. receiptBuilder.ts

`buildReceiptData()` 返回对象中加:

```typescript
return {
    // ... existing fields ...
    queue_number: order.queue_number ?? null,
    qr_data: null,
};
```

`buildArchivedReceiptData()` 同理 — 注意 ArchivedOrderDetail 可能没有 queue_number 字段，先传 null。

**Commit:**

```bash
git commit -m "feat: receipt shows queue number for retail orders + remove QR"
```

---

## Task 5: 编译验证 + 类型检查

**Step 1:**

```bash
cargo clippy --workspace
```

**Step 2:**

```bash
cd red_coral && npx tsc --noEmit
```

**Step 3:** 修复所有 warning/error

**Commit:**

```bash
git commit -m "fix: resolve clippy warnings and type errors"
```

---

## 执行顺序

1. **Task 1** (GDI+ 移入 crab-printer) — 最大风险，先做
2. **Task 2** (executor 标签打印) — 依赖 Task 1
3. **Task 3** (tracing debug) — 独立，可并行
4. **Task 4** (叫号 + QR) — 独立，可并行
5. **Task 5** (编译验证) — 最后

Task 3 和 Task 4 可以并行做。
