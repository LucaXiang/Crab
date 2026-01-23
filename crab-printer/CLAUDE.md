# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## crab-printer

ESC/POS 热敏打印机底层库 - 只关心**怎么打印**，不关心**打印什么**。

## 命令

```bash
cargo check -p crab-printer
cargo test -p crab-printer --lib
```

## 模块结构

```
src/
├── lib.rs      # 公开 API
├── encoding.rs # GBK 编码 (中文打印机必需)
├── escpos.rs   # ESC/POS 命令构建器
├── error.rs    # 错误类型
└── printer.rs  # 打印机适配器 (网络/Windows)
```

## 核心 API

### EscPosBuilder - 命令构建器

```rust
let mut b = EscPosBuilder::new(48); // 80mm 纸宽
b.center()
 .double_size()
 .line("厨房单")
 .reset_size()
 .sep_double()
 .left()
 .line_lr("桌号", "100")
 .cut();

let data = b.build(); // 自动 GBK 编码
```

### NetworkPrinter - 网络打印

```rust
let printer = NetworkPrinter::new("192.168.1.100", 9100)?;
printer.print(&data).await?;
```

### WindowsPrinter - Windows 驱动打印

```rust
#[cfg(windows)]
{
    let printers = WindowsPrinter::list()?;
    let printer = WindowsPrinter::new(&printers[0]);
    printer.print(&data).await?;
}
```

## 设计原则

| 原则 | 说明 |
|------|------|
| **只做底层** | 发送数据到打印机，不包含业务逻辑 |
| **GBK 编码** | 自动处理中文编码，保护 ESC/POS 命令不被破坏 |
| **跨平台** | 网络打印全平台，驱动打印仅 Windows |

## 业务层分离

- **edge-server**: 厨房票据渲染 (KitchenTicketRenderer)
- **red_coral**: 收据渲染 (ReceiptRenderer)

这些渲染器使用 crab-printer 构建和发送数据。

## 响应语言

使用中文回答。
