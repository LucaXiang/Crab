# 订单层区域适配设计

**日期**: 2026-03-02
**范围**: 订单层 (Order Layer) — 不含发票层 (Verifactu/AEAT)
**目标**: 消除西班牙硬编码，使系统可售卖到任意地区

## 背景

系统原为西班牙市场开发，订单层存在大量硬编码：
- 前端 15+ 处硬编码 `€` 货币符号
- 收据渲染器 (Rust) 10 处硬编码西班牙语
- 收据构建器 (TS) 6 处硬编码西班牙语
- Console 1 处硬编码 `es-ES` / `EUR`

StoreInfo 已有 `currency_code`, `currency_symbol`, `currency_decimal_places`, `timezone` 等字段，`formatCurrency()` 已适配。但仍有散落的硬编码需清除。

## 不在范围内

- 发票层 (Verifactu/AEAT/huella) — 独立的合规模块，后续单独设计
- P12 证书 / NIF — 发票层基础设施
- 新税务合规系统 — 不实现，只留适配空间
- 新语言翻译包 — 现有 zh-CN + es-ES 足够

## 变更清单

### Phase 1: 前端硬编码 € 清除

将所有 UI 中硬编码的 `€` 替换为 `useCurrencySymbol()` hook（组件内）或 `getCurrencySymbol()`（非组件上下文）。

| 文件 | 当前 | 替换为 |
|------|------|--------|
| `features/attribute/OptionForm.tsx:202` | `€` | `{currencySymbol}` |
| `features/shift/ShiftActionModal.tsx:360` | `€` | `{currencySymbol}` |
| `features/product/ProductWizard.tsx:206,247` | `€` | `{currencySymbol}` |
| `features/product/ProductForm.tsx:163` | `€` | `{currencySymbol}` |
| `features/price-rule/components/useRuleEditor.ts:82` | `` `€${...}` `` | `` `${getCurrencySymbol()}${...}` `` |
| `features/price-rule/components/RuleDetailPanel.tsx:203` | `€` | `{currencySymbol}` |
| `features/price-rule/components/RuleListPanel.tsx:147` | `` `€${...}` `` | `` `${getCurrencySymbol()}${...}` `` |
| `features/price-rule/PriceRuleWizard/Step5Naming.tsx:54` | `` `€${...}` `` | `` `${getCurrencySymbol()}${...}` `` |
| `screens/Checkout/PriceAdjustmentModal.tsx:253` | `€` | `{currencySymbol}` |
| `screens/Checkout/payment/CashPaymentModal.tsx:222` | `€` | `{currencySymbol}` |
| `screens/Debug/OrderDebug.tsx` (7处) | `€{...}` | `{formatCurrency(...)}` |

**不动的**:
- `VirtualKeyboard.tsx:107` — 虚拟键盘上的 `€` 是字符输入键，保留
- `TimelineItem.tsx:119` — `!detail.includes('€')` 是历史数据检测逻辑，保留
- `LabelEditorScreen.tsx:690` — placeholder 示例文本，保留

### Phase 2: 收据渲染器 i18n (Rust)

**方案**: 在 `receipt_renderer.rs` 中添加 `ReceiptLocale` trait/enum，将所有文本映射到 locale。

新增 `receipt_locale` 字段到 StoreInfo（或直接复用 edge-server 当前语言配置）。收据渲染器根据 locale 选择文本。

当前硬编码 → i18n 映射：

| 当前文本 | 语义 key | es-ES | zh-CN | en |
|---------|----------|-------|-------|-----|
| `FACTURA SIMPLIFICADA` | `receipt.title` | FACTURA SIMPLIFICADA | 简易发票 | RECEIPT |
| `ANULADO` | `receipt.voided` | ANULADO | 已作废 | VOIDED |
| `CUENTA` | `receipt.bill` | CUENTA | 账单 | BILL |
| `REIMPRESION` | `receipt.reprint` | REIMPRESION | 重印 | REPRINT |
| `CIF:` | `receipt.tax_id_label` | CIF: | 税号: | Tax ID: |
| `AHORRO` | `receipt.savings` | AHORRO | 节省 | SAVINGS |
| `IVA INCLUIDO` | `receipt.tax_included` | IVA INCLUIDO | 含税 | TAX INCLUDED |
| `GRACIAS POR SU VISITA` | `receipt.farewell` | GRACIAS POR SU VISITA | 谢谢惠顾 | THANK YOU |

**实现**: Rust `HashMap<&str, &str>` 或 `match` 分发，按 StoreInfo 配置选择。

### Phase 3: 收据构建器 i18n (TypeScript)

`receiptBuilder.ts` 中的硬编码西班牙语替换为 i18n `t()`:

| 当前文本 | 替换为 |
|---------|--------|
| `'Suplemento'` | `t('pos.receipt.surcharge')` |
| `'Descuento'` | `t('pos.receipt.discount')` |
| `'Mostrador'` | `t('pos.receipt.counter')` |
| `'ANULADO'` | `t('pos.receipt.voided')` |
| `'PÉRDIDA'` | `t('pos.receipt.loss')` |

### Phase 4: Console formatCurrency

`crab-console/src/utils/format.ts` 的 `formatCurrency` 从 tenant 配置读取 currency/locale，不再硬编码 `es-ES` / `EUR`。

## 架构原则

1. **StoreInfo 是唯一配置源** — 所有区域敏感值从 StoreInfo 读取
2. **合理默认值** — 未配置时默认 EUR/€/2 位小数/Europe/Madrid/es-ES，保持向后兼容
3. **不建适配层** — 直接在使用点读取配置，不包装 adapter/wrapper
4. **发票层独立演进** — Verifactu 相关代码保持原样，后续作为独立合规模块设计
