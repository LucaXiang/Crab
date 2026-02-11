# i18n Tools Design

## Overview

Two-layer i18n health check toolchain for red_coral frontend.

## Architecture

```
Layer 1: i18n-unused (community package)
  - Static key detection: unused + missing
  - Custom checker for t() pattern
  - ~90% coverage of static t('literal.key') calls

Layer 2: Custom scripts (scripts/i18n/)
  ① sync-check.mjs — zh-CN ↔ es-ES key diff
  ② dynamic-keys.mjs — Dynamic key coverage via registry
  ③ report.mjs — Unified report aggregator
```

## Scripts

### sync-check.mjs
- Flatten both locale JSONs
- Set difference to find keys only in one locale
- Group output by namespace prefix
- Exit code 1 if differences found

### dynamic-keys.mjs
- Maintain a DYNAMIC_KEY_REGISTRY mapping prefixes to expected key sources
- Source types: enum (parse TS file), literal (hardcoded list), grep (regex extract)
- Compare expected keys against translation JSON
- Report missing dynamic keys per prefix

### report.mjs
- Orchestrate all three checks
- Output unified summary with counts

### package.json scripts
```json
{
  "i18n:sync": "node scripts/i18n/sync-check.mjs",
  "i18n:dynamic": "node scripts/i18n/dynamic-keys.mjs",
  "i18n:unused": "npx i18n-unused",
  "i18n:check": "node scripts/i18n/report.mjs"
}
```
