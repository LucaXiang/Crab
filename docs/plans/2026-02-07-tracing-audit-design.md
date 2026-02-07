# Tracing/Logging Audit & Cleanup

Date: 2026-02-07

## Summary

Full audit of ~730+ tracing statements across 50+ Rust files. Identified 5 categories of issues to address.

## Issues Found

### 1. Info Overuse (~40 statements to downgrade)

Many `info!` logs should be `debug!` or deleted. Key areas:

| File | Lines | Issue |
|------|-------|-------|
| `upload/mod.rs` | 44,57,62,66 | Every image serve request logs 3-4 info lines |
| `state.rs` | 735 | `broadcast_sync` logs info per sync event (high frequency) |
| `state.rs` | 737 | "Sync broadcast successful" — redundant success confirmation |
| `bridge/mod.rs` | 791,793,831,833 | Lock acquire/release step tracking |
| `bridge/mod.rs` | 867,877,883,889 | Message listener internal operations |
| `manager.rs` | 423 | "Processing command" with full payload — should be debug |
| `manager.rs` | 446,455 | Pre-generated receipt/queue number — implementation detail |
| `catalog_service.rs` | 162,193,286 | Warmup details |
| `audit/worker.rs` | 58 | Every audit entry recorded — high frequency |
| `auth/middleware.rs` | 44,56,162 | Every OPTIONS/public request logged |
| `cert.rs` | 274-343 | Self-check prints 6-8 step-by-step info lines |

### 2. Debug Trace Remnants (~20 statements to delete)

Development-phase logs with `[function_name]` prefix pattern:

- `upload/mod.rs`: 44,57,62 — `[serve_uploaded_file]` prefix
- `bridge/mod.rs`: 791,793,831,833 — `[start_server_mode]` prefix
- `auth/middleware.rs`: 44,56,162 — `[require_auth]`/`[require_admin]` prefix
- `storage.rs`: 236,247 — `next_order_count` step tracking

### 3. Mixed Chinese/English (~10 statements to unify)

All in edge-server:

- `manager.rs`: 277,301,313 — Rule snapshot errors in Chinese
- `state.rs`: 390,423,430 — Rule warmup messages in Chinese
- `processor.rs`: 186,190 — Operator lookup in Chinese
- `open_table.rs`: 49 — Price rule loading in Chinese

All should be converted to English.

### 4. Emoji in Log Messages (~30 statements to clean)

~20 emoji types used: checkmark, rocket, lock, package, broom, etc.
Replace with text labels or remove entirely.

### 5. Cross-Layer Duplication (~15 redundant statements)

| Operation | Duplicate Locations |
|-----------|-------------------|
| Employee login | `api/auth/handler.rs` + `bridge/mod.rs` + `local.rs`/`remote.rs` (3-4 info per login) |
| Command processing | `manager.rs` entry + exit (use `#[instrument]` or keep only result) |
| broadcast_sync | info "Broadcasting" + debug "successful" (remove success confirmation) |
| Cert self-check | 6-8 step info lines (consolidate to final result) |

## Rules Added

Logging specification added to root `CLAUDE.md` under "日志规范 (tracing)" section. Covers:
- Level selection criteria
- Info admission standards
- Single authority point principle
- Prohibited patterns

## Implementation Plan

If proceeding with cleanup:

1. **Phase 1**: Delete debug remnants (safe, no behavior change)
2. **Phase 2**: Downgrade info → debug (changes log output at info level)
3. **Phase 3**: Unify language to English
4. **Phase 4**: Remove emoji
5. **Phase 5**: Deduplicate cross-layer logging

Each phase should be a separate commit for easy review/revert.
