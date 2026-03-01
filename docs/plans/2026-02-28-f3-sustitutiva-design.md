# F3 Sustitutiva (Invoice Upgrade) Design

## Goal

允许收银员将 F2 简化发票升级为 F3 完整发票（Factura Sustitutiva），客户提供 NIF、公司名等信息后，系统生成新的 F3 发票并加入 huella + chain_entry 链。

## Architecture

F3 在 Verifactu 中属于 RegistroFacturaAlta（与 F2/R5 相同），使用 huella_alta 公式。因此复用现有 `invoice` 表，添加客户信息字段和 `factura_sustituida` 引用字段。F3 的金额完全复制原始 F2（退款 R5 独立存在不影响 F3）。

新增 UPGRADE 类型到 chain_entry 链（使用已定义的 `UpgradeChainData`）。

## Key Decisions

- **复用 Invoice 表**：F3 和 F2/R5 共享 huella 计算、sync 管线、AEAT 提交流程
- **F3 金额 = F2 金额**：不扣除退款，退款 R5 是独立发票
- **factura_sustituida 独立于 factura_rectificada**：AEAT 规范中是不同概念
- **eligibility**：有 F2 + 未作废 + 未升级过

## Data Model

### Edge SQLite (Migration 0007)

```sql
-- Invoice 表新增客户信息（F3 Sustitutiva）
ALTER TABLE invoice ADD COLUMN customer_nif TEXT;
ALTER TABLE invoice ADD COLUMN customer_nombre TEXT;
ALTER TABLE invoice ADD COLUMN customer_address TEXT;
ALTER TABLE invoice ADD COLUMN customer_email TEXT;
ALTER TABLE invoice ADD COLUMN customer_phone TEXT;

-- F3 替代的原始 F2 引用
ALTER TABLE invoice ADD COLUMN factura_sustituida_id INTEGER REFERENCES invoice(id);
ALTER TABLE invoice ADD COLUMN factura_sustituida_num TEXT;

-- 标记订单已被 F3 升级
ALTER TABLE archived_order ADD COLUMN is_upgraded INTEGER NOT NULL DEFAULT 0;
```

### Shared Models

**TipoFactura**: 新增 `F3` variant

**InvoiceSourceType**: 新增 `Upgrade` variant

**Invoice struct**: 新增 `customer_nif`, `customer_nombre`, `customer_address`, `customer_email`, `customer_phone`, `factura_sustituida_id`, `factura_sustituida_num` 字段

### Cloud PostgreSQL (Migration)

```sql
ALTER TABLE store_invoices ADD COLUMN customer_nif TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_nombre TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_address TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_email TEXT;
ALTER TABLE store_invoices ADD COLUMN customer_phone TEXT;
ALTER TABLE store_invoices ADD COLUMN factura_sustituida_id BIGINT;
ALTER TABLE store_invoices ADD COLUMN factura_sustituida_num TEXT;
```

## Service Layer

### UpgradeService (`edge-server/src/archiving/upgrade.rs`)

```
check_upgrade_eligibility(pool, order_pk) → { eligible: bool, reason?: string }
  - order.status = COMPLETED
  - order.is_anulada = 0
  - order.is_upgraded = 0
  - find_order_invoice(order_pk) returns Some(F2)

create_upgrade(pool, invoice_service, request) → Invoice (F3)
  1. 获取 hash_chain_lock
  2. 验证 eligibility
  3. BEGIN tx
  4. find_order_invoice(tx, order_pk) → F2
  5. 从 F2 复制: subtotal, tax, total
  6. 从 F2 复制 desglose 行
  7. 读 prev_huella
  8. next_invoice_number(tx, serie, date) → F3 新发票号
  9. compute_verifactu_huella_alta(tipo="F3", ...) → huella
  10. INSERT invoice (tipo=F3, customer_*, factura_sustituida_*)
  11. INSERT invoice_desglose (复制)
  12. 读 last_chain_hash
  13. compute_chain_hash(UpgradeChainData{...})
  14. INSERT chain_entry (type=UPGRADE, entry_pk=f3_invoice_id)
  15. UPDATE system_state (last_chain_hash + last_huella)
  16. UPDATE archived_order SET is_upgraded = 1
  17. COMMIT
```

### API Endpoints

```
POST /api/invoices/upgrade
  body: { order_pk, customer_nif, customer_nombre, customer_address?, customer_email?, customer_phone? }
  response: Invoice (F3)

GET /api/invoices/upgrade/eligibility/{order_pk}
  response: { eligible: bool, reason?: string }
```

### Cloud Sync

**InvoiceSync** 新增字段：`customer_nif`, `customer_nombre`, `customer_address`, `customer_email`, `customer_phone`, `factura_sustituida_id`, `factura_sustituida_num`

`verify_huella()` 不变 — huella 计算只使用发票本身字段（NIF 是商家 NIF，不是客户 NIF）。

**Cloud upsert_invoice()** 新增字段写入。

**chain_entry UPGRADE 同步**：CloudWorker 已按 chain_entry.id 顺序同步，UPGRADE 条目通过 invoice 同步管线自动覆盖（F3 发票 sync 后 cloud 端可见）。

chain_entry.UPGRADE 本身需要同步到 cloud 的 `store_chain_entries` 表，类似 ORDER / CREDIT_NOTE / ANULACION。需要：
- `shared/src/cloud/sync.rs` 新增 `UpgradeSync` struct + `verify_hash()`
- `crab-cloud/src/db/sync_store.rs` 新增 `upsert_upgrade()`

## Frontend

### Tauri Commands (`chain_entries.rs`)

```
check_upgrade_eligibility(order_pk) → GET /api/invoices/upgrade/eligibility/{order_pk}
create_upgrade(request) → POST /api/invoices/upgrade
fetch_chain_upgrade_detail(id) → GET /api/chain-entries/upgrade/{id}
```

### Types

**ArchivedOrderDetail**: 新增 `is_upgraded: boolean`

**ChainEntryType**: 新增 `'UPGRADE'`

**ChainUpgradeDetail**: 新接口（F3 发票信息 + 客户信息 + 原 F2 引用）

### Components

**UpgradeInvoiceModal** (新组件):
- 入口：HistoryDetail header 的"升级发票"按钮
- 条件：`!isVoid && !isMerged && !is_anulada && !is_upgraded`
- 先 check eligibility
- 表单：NIF（必填）、公司名（必填）、地址（可选）、Email（可选）、电话（可选）
- 只读显示 F2 金额
- 提交后刷新

**ChainEntrySidebar**: UPGRADE 条目样式（ArrowUpCircle 图标）

**ChainUpgradeDetailView** (新组件): 显示 F3 完整信息

**HistoryDetail**: 新增"升级发票"按钮 + "已升级"标签

### i18n

zh-CN + es-ES 添加 `upgrade.*` 翻译组

## File Changelist

### Edge Server
| File | Change |
|------|--------|
| `migrations/0007_invoice_upgrade.up.sql` | 新建 |
| `migrations/0007_invoice_upgrade.down.sql` | 新建 |
| `src/archiving/upgrade.rs` | 新建：UpgradeService |
| `src/archiving/mod.rs` | 注册 upgrade 模块 |
| `src/api/invoices/` | 新建：upgrade handler + routes |
| `src/api/mod.rs` | 注册 upgrade 路由 |
| `src/api/chain_entries/handler.rs` | UNION ALL 加 UPGRADE 分支 + 详情 endpoint |
| `src/db/repository/invoice.rs` | insert() 支持新字段 + find_desglose() |
| `src/db/repository/order.rs` | OrderRow/OrderDetail 加 is_upgraded |

### Shared
| File | Change |
|------|--------|
| `src/models/invoice.rs` | TipoFactura::F3, InvoiceSourceType::Upgrade, Invoice 新字段 |
| `src/cloud/sync.rs` | InvoiceSync 新字段 + UpgradeSync struct |
| `src/order/canonical.rs` | compute_upgrade_chain_hash() wrapper |

### Cloud
| File | Change |
|------|--------|
| `migrations/` | store_invoices 新列 |
| `src/db/sync_store.rs` | upsert_invoice() 新字段 + upsert_upgrade() |

### Red Coral (Frontend)
| File | Change |
|------|--------|
| `src-tauri/src/commands/chain_entries.rs` | 3 个新命令 |
| `src-tauri/src/lib.rs` | 注册命令 |
| `src/core/domain/types/archivedOrder.ts` | is_upgraded 字段 |
| `src/core/domain/types/chainEntry.ts` | UPGRADE type + ChainUpgradeDetail |
| `src/screens/History/UpgradeInvoiceModal.tsx` | 新建 |
| `src/screens/History/ChainUpgradeDetail.tsx` | 新建 |
| `src/screens/History/ChainEntrySidebar.tsx` | UPGRADE 样式 |
| `src/screens/History/HistoryDetail.tsx` | 升级按钮 + 已升级标签 |
| `src/screens/History/index.tsx` | UPGRADE routing + detail loading |
| `src/hooks/useChainUpgradeDetail.ts` | 新建 |
| `src/infrastructure/i18n/locales/zh-CN.json` | upgrade.* |
| `src/infrastructure/i18n/locales/es-ES.json` | upgrade.* |

## Verification

```bash
cargo check --workspace
cargo clippy --workspace
cargo test -p shared --lib
cargo test -p edge-server --lib
cd red_coral && npx tsc --noEmit
```

功能验证：
1. 有 F2 且未作废/未升级的订单 → 显示"升级发票"按钮
2. 已作废/已升级 → 不显示
3. 点击 → Modal 弹出，填写客户信息
4. 提交 → F3 发票创建，chain_entry 出现 UPGRADE 条目
5. F3 金额 = F2 金额（即使有退款）
6. Cloud 同步后 store_invoices 有 F3 记录
