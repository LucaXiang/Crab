# RedCoral POS - Project Guide

A full-stack Point of Sale (POS) application built with Tauri, React, TypeScript, and Rust.

## Tech Stack

- **Frontend**: React 19, TypeScript 5.8, Vite 6
- **State Management**: Zustand 5
- **Desktop Runtime**: Tauri 2.9 (Rust backend with SQLite)
- **Styling**: TailwindCSS 4
- **Charts**: Recharts
- **Database**: SQLite (via sqlx in Rust)

## Project Structure

```
red_coral/
├── src/
│   ├── core/                    # Core domain logic and types
│   │   ├── domain/
│   │   │   ├── types/          # TypeScript type definitions (must match Rust)
│   │   │   │   ├── index.ts    # Main re-export
│   │   │   │   ├── product.ts  # Product, Category types
│   │   │   │   ├── cart.ts     # CartItem, HeldOrder types
│   │   │   │   ├── table.ts    # Table, Zone, KitchenPrinter types
│   │   │   │   ├── attribute.ts# Attribute system types
│   │   │   │   ├── auth.ts     # User, Role, Permission types
│   │   │   │   ├── statistics.ts # Statistics types (must match Rust)
│   │   │   │   └── payment.ts  # Payment types
│   │   │   └── events/         # Event sourcing types
│   │   ├── services/           # Business logic
│   │   │   ├── order/          # Order processing, event sourcing
│   │   │   └── pricing/        # Price calculations
│   │   └── stores/             # Zustand stores (core/legacy)
│   │       ├── auth/
│   │       ├── cart/
│   │       ├── order/
│   │       ├── product/
│   │       ├── settings/
│   │       └── ui/
│   ├── infrastructure/         # External integrations
│   │   ├── api/                # API calls to Rust backend
│   │   ├── dataSource/         # Data source abstraction
│   │   ├── i18n/               # Internationalization
│   │   ├── persistence/        # Data persistence
│   │   └── print/              # Printing services
│   ├── presentation/           # UI components (new architecture)
│   │   ├── components/
│   │   │   ├── auth/           # Authentication components
│   │   │   ├── cart/           # Cart components
│   │   │   ├── form/           # Form components
│   │   │   ├── modals/         # Modal components
│   │   │   ├── shared/         # Shared components
│   │   │   └── ui/             # UI primitives
│   │   └── Toast.tsx
│   ├── screens/                # Page components
│   │   ├── Checkout/           # Checkout flow
│   │   ├── History/            # Order history
│   │   ├── Login/              # Authentication
│   │   ├── POS/                # Main POS interface
│   │   ├── Settings/           # System settings
│   │   ├── Statistics/         # Reports & analytics
│   │   ├── TableSelection/     # Table management
│   │   └── Unauthorized.tsx
│   ├── hooks/                  # React hooks
│   ├── stores/                 # Legacy Zustand stores
│   ├── services/               # Legacy services
│   ├── types/                  # Legacy types
│   ├── utils/                  # Utility functions
│   │   ├── currency/           # Currency calculations (Decimal.js)
│   │   ├── formatting/         # Formatting utilities
│   │   └── pricing/            # Pricing engine
│   ├── App.tsx
│   └── main.tsx
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── api/                # Tauri commands
│   │   │   ├── attributes/     # Attribute management
│   │   │   ├── categories/     # Category CRUD
│   │   │   ├── data.rs         # Data operations
│   │   │   ├── kitchen_printers.rs
│   │   │   ├── orders/         # Order operations
│   │   │   ├── price_adjustments.rs
│   │   │   ├── printers.rs
│   │   │   ├── products.rs     # Product CRUD
│   │   │   ├── specifications.rs
│   │   │   ├── statistics.rs   # Statistics API
│   │   │   ├── system.rs
│   │   │   ├── tables.rs
│   │   │   ├── users.rs
│   │   │   └── zones.rs
│   │   ├── core/
│   │   │   ├── db.rs           # Database connection
│   │   │   ├── state.rs        # App state
│   │   │   └── types.rs        # Rust type definitions
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   └── utils/
│   │       ├── escpos_text.rs
│   │       ├── label_printer.rs
│   │       ├── printing.rs
│   │       ├── query_builder.rs
│   │       └── receipt_renderer.rs
│   └── Cargo.toml
├── dist/                       # Built frontend
├── node_modules/
├── package.json
├── tsconfig.json
├── vite.config.ts
└── tauri.conf.json
```

## Key Concepts

### Type Alignment (Frontend ↔ Backend)

**Critical**: TypeScript types in `src/core/domain/types/` must match Rust types in `src-tauri/src/core/types.rs` and `src-tauri/src/api/`.

When modifying types:
1. Update Rust types first (in `.rs` files)
2. Update TypeScript types to match
3. Run `npx tsc --noEmit` to verify

**Statistics types** are particularly important:
- Rust: `src-tauri/src/api/statistics.rs` defines `OverviewStats`, `RevenueTrendPoint`, `CategorySale`, `TopProduct`, etc.
- TypeScript: `src/core/domain/types/statistics.ts` must match field names exactly

Example mismatch to avoid:
```typescript
// Rust returns:
{ time: "09:00", value: 150.50 }

// TypeScript should expect:
interface RevenueTrendPoint {
  time: string;   // NOT "timestamp"
  value: number;  // NOT "revenue"
}
```

### Event Sourcing

Orders use an event sourcing pattern:
- Events stored in `timeline` array of `HeldOrder`
- Event types defined in `src/core/domain/events/`
- Reducer in `src/core/services/order/eventReducer.ts`

### Currency Handling

Always use `Currency` utility from `src/utils/currency/currency.ts` for financial calculations:

```typescript
import { Currency } from '@/utils/currency';

const total = Currency.add(itemPrice, surcharge);
const final = Currency.floor2(total);
```

Never use native JavaScript numbers for money calculations.

### State Management

Two patterns coexist:
1. **Legacy**: Direct stores in `src/stores/`
2. **New**: React hooks wrapping stores in `src/core/stores/`

When modifying state, prefer the new architecture pattern.

## Common Commands

```bash
# Development
npm run dev              # Frontend dev server
npm run tauri:dev        # Full app with Tauri

# Build
npm run build            # Build frontend
npm run tauri:build      # Build Tauri app

# Type checking
npx tsc --noEmit         # TypeScript check
npx ts-prune             # Find unused code

# Testing
npm run test             # Run tests
```

## Architecture Decisions

### Why Tauri + Rust?
- Low memory footprint compared to Electron
- Native SQLite access
- Direct hardware control (printers, cash drawers)

### Why Recharts?
- Built for React
- Good TypeScript support
- Lightweight compared to Chart.js

### Why Zustand?
- Simple API
- No context provider wrapping hell
- Works well with TypeScript

## Database Schema Notes

The SQLite database (managed by Rust/sqlx) includes:
- `products` table with JSON attributes column
- `orders` table with timeline events
- `order_items` with price tracking
- `payments` table for payment records

Migrations are managed via `src-tauri/migrations/` directory.

## Debugging Tips

1. **Frontend**: Open browser DevTools (F12)
2. **Tauri**: Enable devtools in `tauri.conf.json`
3. **Rust**: Use `println!` for logging, view in Tauri logs

## i18n

Translations in `src/services/i18n/locales/`:

```typescript
// Usage
import { useI18n } from '@/hooks/useI18n';

const { t } = useI18n();
t('statistics.revenue');  // Key format: "section.key"
```

## Adding New Features

1. Define Rust types in `src-tauri/src/api/`
2. Add Tauri command in Rust
3. Create TypeScript types in `src/core/domain/types/`
4. Add API wrapper in `src/infrastructure/api/`
5. Create React components in appropriate directory
6. Update routes in `App.tsx`

## Important Files to Know

| File | Purpose |
|------|---------|
| `src/core/domain/types/index.ts` | Central type re-exports |
| `src/core/services/order/eventReducer.ts` | Order state machine |
| `src/utils/currency/currency.ts` | Money calculations |
| `src/services/printService.ts` | Receipt/kitchen printing |
| `src/core/stores/order/useOrderStore.ts` | Order state |
| `src/core/stores/product/useProductStore.ts` | Product catalog |
