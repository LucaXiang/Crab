# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

RedCoral POS - A full-stack Point of Sale application built with Tauri, React, TypeScript, and Rust. Part of the Crab distributed restaurant management system.

## Tech Stack

- **Frontend**: React 19, TypeScript 5.8, Vite 6, Zustand 5, TailwindCSS 4
- **Desktop Runtime**: Tauri 2.9 (Rust backend)
- **Database**: SurrealDB (embedded, via edge-server)
- **Backend Integration**: Uses `edge-server` and `crab-client` workspace crates

## Project Structure

```
red_coral/
├── src/                        # React frontend
│   ├── core/                   # Core domain (types, stores, services)
│   │   ├── domain/types/       # TypeScript types (must match Rust)
│   │   └── stores/             # Zustand stores (auth, cart, bridge, etc.)
│   ├── screens/                # Page components (Login, POS, Setup, etc.)
│   ├── presentation/           # UI components
│   ├── utils/currency/         # Money calculations (Decimal.js)
│   └── App.tsx                 # Routes and app shell
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── commands/           # Tauri commands (auth, mode, tenant, etc.)
│   │   ├── core/               # ClientBridge, TenantManager, config
│   │   └── lib.rs              # Command registration
│   └── Cargo.toml
└── package.json
```

This app uses workspace crates from parent `/Users/xzy/workspace/crab/`:
- `edge-server` - Embedded server with SurrealDB, JWT auth, message bus
- `crab-client` - Unified client (Local/Remote) with typestate pattern
- `shared` - Common types and protocols

## Key Concepts

### ClientBridge Architecture (Server/Client Modes)

The app runs in two modes managed by `ClientBridge` (`src-tauri/src/core/client_bridge.rs`):

- **Server Mode**: Runs embedded `edge-server` with In-Process communication (LocalClient)
- **Client Mode**: Connects to remote edge-server via mTLS (RemoteClient)

```
App Startup → ClientBridge (Disconnected)
           → TenantManager loads certificates
           → User selects mode → Server/Client
           → CrabClient (Local/Remote) → edge-server APIs
```

Key files:
- `src-tauri/src/core/client_bridge.rs` - Mode management, CrabClient state transitions
- `src-tauri/src/core/tenant_manager.rs` - Multi-tenant certificate management
- `src/core/stores/bridge/useBridgeStore.ts` - Frontend state for mode/auth

### Type Alignment (Frontend ↔ Backend)

**Critical**: TypeScript types must match Rust types exactly.

When modifying types:
1. Update Rust types first (in `.rs` files)
2. Update TypeScript types to match
3. Run `npx tsc --noEmit` to verify

Key type locations:
- Rust: `src-tauri/src/core/`, `edge-server/src/api/`, `shared/src/`
- TypeScript: `src/core/domain/types/`

### Currency Handling

Always use `Currency` utility for financial calculations:

```typescript
import { Currency } from '@/utils/currency';
const total = Currency.add(itemPrice, surcharge);
const final = Currency.floor2(total);
```

### State Management

Two patterns coexist (prefer new architecture):
- **Legacy**: Direct stores in `src/stores/`
- **New**: React hooks wrapping stores in `src/core/stores/`

## Common Commands

```bash
# Development
npm run tauri:dev        # Full app with Tauri (use this)
npm run dev              # Frontend only (vite dev server)

# Build
npm run tauri:build      # Build Tauri app
npm run build            # Build frontend only

# Type checking
npx tsc --noEmit         # TypeScript check

# Testing
npm run test             # Run vitest tests
npm run deadcode         # Find unused exports (ts-prune)
```

## App Data Location

User data stored at: `~/Library/Application Support/com.xzy.pos/redcoral/`
- `config.json` - Mode and tenant configuration
- `tenants/` - Per-tenant certificate storage
- `database/` - Local database files

## Authentication Flow

1. **Setup** (`/setup`) - First-run tenant activation via Auth Server
2. **Login** (`/login`) - Employee login (uses CrabClient.login())
3. **POS** (`/pos`) - Protected route, requires authenticated session

Routes in `App.tsx`:
- `InitialRoute` - Checks first-run, auto-starts Server mode if tenant exists
- `ProtectedRoute` - Wraps authenticated routes

## Adding Tauri Commands

1. Add command in `src-tauri/src/commands/`
2. Register in `src-tauri/src/lib.rs` invoke_handler
3. Call from frontend: `invoke<ReturnType>('command_name', { args })`

## Important Files

| File | Purpose |
|------|---------|
| `src-tauri/src/core/client_bridge.rs` | Server/Client mode management |
| `src-tauri/src/core/tenant_manager.rs` | Multi-tenant certificates |
| `src/core/stores/bridge/useBridgeStore.ts` | Frontend bridge state |
| `src/utils/currency/currency.ts` | Money calculations |
| `src/App.tsx` | Routes and initial flow |
