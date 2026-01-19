# Frontend Structure Refactor Design

## Overview

Refactor the frontend project structure to improve maintainability and extensibility without changing UI/UX functionality.

## Current Problems

1. **Incomplete migration**: `services/` and `infrastructure/` have overlapping responsibilities
2. **Scattered types**: Types spread across `src/types/`, `src/types.ts`, `core/domain/types/`, `infrastructure/api/types/`
3. **Redundant compatibility layer**: `src/types.ts` re-exports from `core/domain/types`
4. **Deprecated files**: `priceAdjustment.deprecated.ts` still exists

## Target Structure

```
src/
├── core/                          # Business core
│   ├── domain/
│   │   ├── types/                 # [UNIFIED] All type definitions
│   │   │   ├── models/            # Business entities (existing types)
│   │   │   ├── events/            # Event types (from src/types/events.ts)
│   │   │   ├── api/               # API request/response types
│   │   │   └── print/             # Print/label types (from src/types/)
│   │   ├── events/                # Event factory and adapters (keep)
│   │   └── validators.ts
│   ├── hooks/                     # System-level hooks (keep)
│   ├── stores/                    # Zustand stores (keep)
│   ├── services/                  # Business logic services (keep)
│   └── validation/                # Form validation (keep)
│
├── infrastructure/                # [UNIFIED] External interface layer
│   ├── api/                       # API client (existing)
│   ├── i18n/                      # i18n (migrate from services/)
│   ├── print/                     # Print service (merge)
│   ├── label/                     # Label printing (migrate from services/)
│   ├── persistence/               # Local storage (existing)
│   └── dataSource/                # Data source abstraction (migrate from services/)
│
├── hooks/                         # General UI hooks (keep)
├── screens/                       # Page components (unchanged)
├── presentation/                  # UI components (unchanged)
├── assets/                        # Static assets (unchanged)
└── utils/                         # Utility functions (unchanged)
```

## Migration Steps

### 1. Unify Types

- Create subdirectories in `core/domain/types/`
- Move `src/types/events.ts` → `core/domain/types/events/`
- Move `src/types/labelTemplate.ts` → `core/domain/types/print/`
- Move `src/types/priceAdjustment.ts` → `core/domain/types/models/`
- Move `src/types/print.ts` → `core/domain/types/print/`
- Move `infrastructure/api/types/` → `core/domain/types/api/`
- Delete `src/types/` directory
- Delete `src/types.ts` compatibility layer
- Delete `src/types/priceAdjustment.deprecated.ts`

### 2. Migrate Services to Infrastructure

- Move `services/i18n/` → `infrastructure/i18n/`
- Move `services/label/` → `infrastructure/label/`
- Move `services/dataSource/` → `infrastructure/dataSource/`
- Merge `services/print/` + `services/printService.ts` → `infrastructure/print/`
- Move `services/api/` contents → `infrastructure/api/` (if needed)
- Delete `src/services/` directory

### 3. Update Import Paths

- Update all imports from `@/types/` → `@/core/domain/types/`
- Update all imports from `@/services/` → `@/infrastructure/`
- Ensure all barrel exports (index.ts) are updated

### 4. Cleanup

- Remove empty directories
- Remove deprecated files
- Verify no orphaned imports

## Rollback Plan

All changes are file moves and import updates. Git history preserves everything. Rollback via `git checkout .` if needed.
