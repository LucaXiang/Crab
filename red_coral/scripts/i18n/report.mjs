#!/usr/bin/env node

/**
 * report.mjs — Unified i18n health report
 *
 * Runs all i18n checks and outputs a combined summary:
 * 1. Locale sync check (zh-CN <-> es-ES)
 * 2. Dynamic key coverage
 * 3. Static key analysis (unused + missing)
 *
 * Usage: node scripts/i18n/report.mjs
 */

import { syncCheck } from './sync-check.mjs';
import { dynamicKeysCheck } from './dynamic-keys.mjs';
import { staticKeysCheck } from './static-keys.mjs';

let totalIssues = 0;

console.log('\n╔══════════════════════════════════════╗');
console.log('║       i18n Health Report              ║');
console.log('╚══════════════════════════════════════╝\n');

// ============================================================================
// 1. Sync Check
// ============================================================================

console.log('─── Locale Sync (zh-CN <-> es-ES) ───\n');

try {
  const { onlyInZh, onlyInEs, commonCount } = syncCheck();

  if (onlyInZh.length > 0) {
    console.log(`\x1b[31m  ✗ Only in zh-CN: ${onlyInZh.length} keys\x1b[0m`);
    for (const k of onlyInZh.slice(0, 10)) console.log(`    - ${k}`);
    if (onlyInZh.length > 10) console.log(`    ... and ${onlyInZh.length - 10} more`);
    totalIssues += onlyInZh.length;
  }

  if (onlyInEs.length > 0) {
    console.log(`\x1b[31m  ✗ Only in es-ES: ${onlyInEs.length} keys\x1b[0m`);
    for (const k of onlyInEs.slice(0, 10)) console.log(`    - ${k}`);
    if (onlyInEs.length > 10) console.log(`    ... and ${onlyInEs.length - 10} more`);
    totalIssues += onlyInEs.length;
  }

  if (onlyInZh.length === 0 && onlyInEs.length === 0) {
    console.log(`\x1b[32m  ✓ All ${commonCount} keys in sync\x1b[0m`);
  }
} catch (err) {
  console.log(`\x1b[31m  ✗ Sync check failed: ${err.message}\x1b[0m`);
  totalIssues++;
}

// ============================================================================
// 2. Dynamic Key Coverage
// ============================================================================

console.log('\n─── Dynamic Key Coverage ───\n');

try {
  const results = dynamicKeysCheck();
  let dynamicMissing = 0;

  for (const r of results) {
    const allCovered = r.missingInZh.length === 0 && r.missingInEs.length === 0;
    if (allCovered) {
      console.log(`\x1b[32m  ✓ ${r.prefix}: ${r.expected}/${r.expected}\x1b[0m`);
    } else {
      const missing = r.missingInZh.length + r.missingInEs.length;
      console.log(`\x1b[31m  ✗ ${r.prefix}: missing ${missing}\x1b[0m`);
      if (r.missingInZh.length > 0) {
        console.log(`    zh-CN: ${r.missingInZh.join(', ')}`);
      }
      if (r.missingInEs.length > 0) {
        console.log(`    es-ES: ${r.missingInEs.join(', ')}`);
      }
      dynamicMissing += missing;
    }
  }

  totalIssues += dynamicMissing;
} catch (err) {
  console.log(`\x1b[31m  ✗ Dynamic check failed: ${err.message}\x1b[0m`);
  totalIssues++;
}

// ============================================================================
// 3. Static Key Analysis
// ============================================================================

console.log('\n─── Static Key Analysis ───\n');

try {
  const { unusedKeys, missingKeys, totalLocaleKeys, totalUsedKeys } = staticKeysCheck();

  console.log(`  Locale keys: ${totalLocaleKeys} | Code references: ${totalUsedKeys}\n`);

  if (unusedKeys.length > 0) {
    console.log(`\x1b[33m  ⚠ Unused keys (in JSON, not in code): ${unusedKeys.length}\x1b[0m`);
    for (const k of unusedKeys.slice(0, 15)) console.log(`    - ${k}`);
    if (unusedKeys.length > 15) console.log(`    ... and ${unusedKeys.length - 15} more`);
    // Unused keys are warnings, not hard errors
  } else {
    console.log('\x1b[32m  ✓ No unused keys\x1b[0m');
  }

  console.log();

  if (missingKeys.length > 0) {
    console.log(`\x1b[31m  ✗ Missing keys (in code, not in JSON): ${missingKeys.length}\x1b[0m`);
    for (const k of missingKeys.slice(0, 15)) console.log(`    - ${k}`);
    if (missingKeys.length > 15) console.log(`    ... and ${missingKeys.length - 15} more`);
    totalIssues += missingKeys.length;
  } else {
    console.log('\x1b[32m  ✓ No missing keys\x1b[0m');
  }
} catch (err) {
  console.log(`\x1b[31m  ✗ Static check failed: ${err.message}\x1b[0m`);
  totalIssues++;
}

// ============================================================================
// Summary
// ============================================================================

console.log('\n═══════════════════════════════════════');
if (totalIssues === 0) {
  console.log('\x1b[32m✓ All i18n checks passed!\x1b[0m');
} else {
  console.log(`\x1b[31m✗ ${totalIssues} issues found\x1b[0m`);
}
console.log('═══════════════════════════════════════\n');

process.exit(totalIssues > 0 ? 1 : 0);
