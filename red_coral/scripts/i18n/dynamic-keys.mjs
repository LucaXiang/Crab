#!/usr/bin/env node

/**
 * dynamic-keys.mjs — Check dynamic i18n key coverage
 *
 * Maintains a registry of dynamic key prefixes and their expected values.
 * Verifies that all expected keys exist in translation files.
 *
 * Usage: node scripts/i18n/dynamic-keys.mjs
 * Exit code: 0 if all covered, 1 if missing keys found
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCALES_DIR = resolve(__dirname, '../../src/infrastructure/i18n/locales');
const SRC_DIR = resolve(__dirname, '../../src');

// ============================================================================
// Dynamic Key Registry
// ============================================================================

/**
 * Each entry declares:
 * - prefix: the i18n key prefix (e.g. 'commandError')
 * - source: how to get expected key suffixes
 *   - type: 'enum' — parse TS union type from a file
 *   - type: 'literal' — hardcoded list of expected values
 *   - type: 'keys' — read keys from translation JSON itself (self-check only)
 * - extras: additional static keys under this prefix (e.g. '_fallback')
 */
const DYNAMIC_KEY_REGISTRY = [
  {
    prefix: 'commandError',
    source: {
      type: 'enum',
      file: 'core/domain/types/orderEvent.ts',
      // Match: | 'SOME_CODE'
      pattern: /^\s*\|\s*'([A-Z_]+)'/gm,
      startAfter: 'export type CommandErrorCode =',
    },
    extras: ['_fallback'],
  },
  {
    prefix: 'calendar.days',
    source: {
      type: 'literal',
      values: ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'],
    },
  },
  {
    prefix: 'checkout.comp.preset',
    source: { type: 'keys' }, // self-check: just verify both locales have same keys
  },
  {
    prefix: 'checkout.delete_item.reason',
    source: { type: 'keys' },
  },
  {
    prefix: 'checkout.void.cancel_reason',
    source: { type: 'keys' },
  },
  {
    prefix: 'checkout.void.loss_reason',
    source: { type: 'keys' },
  },
  {
    prefix: 'audit.action',
    source: { type: 'keys' },
  },
  {
    prefix: 'audit.resource_type',
    source: { type: 'keys' },
  },
  {
    prefix: 'audit.group',
    source: { type: 'keys' },
  },
  {
    prefix: 'audit.detail.value',
    source: { type: 'keys' },
  },
  {
    prefix: 'history.void_type',
    source: {
      type: 'literal',
      values: ['CANCELLED', 'LOSS_SETTLED'],
    },
  },
  {
    prefix: 'history.loss_reason',
    source: {
      type: 'literal',
      values: ['CUSTOMER_FLED', 'REFUSED_TO_PAY', 'OTHER'],
    },
  },
  {
    prefix: 'statistics.status',
    source: {
      type: 'literal',
      values: ['completed', 'voided', 'merged'],
    },
  },
  {
    prefix: 'statistics.time',
    source: {
      type: 'literal',
      values: ['today', 'week', 'month', 'custom'],
    },
  },
  {
    prefix: 'system_issue.kind',
    source: { type: 'keys' }, // nested: system_issue.kind.*.title / *.description
    nested: true,
  },
  {
    prefix: 'system_issue.option',
    source: { type: 'keys' },
  },
  {
    prefix: 'activation.reason',
    source: { type: 'keys' },
  },
  {
    prefix: 'activation.hint',
    source: { type: 'keys' },
  },
  {
    prefix: 'subscription.status',
    source: { type: 'keys' },
  },
  {
    prefix: 'subscriptionBlocked.message',
    source: { type: 'keys' },
  },
  {
    prefix: 'subscriptionBlocked.planType',
    source: { type: 'keys' },
  },
  {
    prefix: 'settings.price_rule.scope',
    source: {
      type: 'literal',
      values: ['global', 'category', 'tag', 'product'],
    },
  },
  {
    prefix: 'settings.price_rule.stacking',
    source: {
      type: 'literal',
      values: ['exclusive', 'stackable', 'non_stackable'],
    },
  },
  {
    prefix: 'settings.price_rule.preview',
    source: { type: 'keys' },
  },
  {
    prefix: 'settings.marketing_group.scope',
    source: {
      type: 'literal',
      values: ['global', 'category', 'tag', 'product'],
    },
  },
  {
    prefix: 'settings.marketing_group.stamp.target_type',
    source: {
      type: 'literal',
      values: ['category', 'product'],
    },
  },
  {
    prefix: 'settings.marketing_group.stamp.strategy',
    source: {
      type: 'literal',
      values: ['economizador', 'generoso', 'designated'],
    },
  },
  {
    prefix: 'error.friendly',
    source: {
      type: 'literal',
      values: ['network', 'auth', 'certificate', 'port', 'activation', 'unknown'],
    },
  },
  {
    prefix: 'audit.detail.field',
    source: { type: 'keys' }, // `audit.detail.field.${field}` in AuditLog renderers
  },
  {
    prefix: 'errors',
    source: { type: 'keys' }, // `errors.${code}` in tauri-client & error/index
  },
];

// ============================================================================
// Helpers
// ============================================================================

function flattenObject(obj, prefix = '') {
  const result = {};
  for (const key of Object.keys(obj)) {
    const value = obj[key];
    const newKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      Object.assign(result, flattenObject(value, newKey));
    } else if (typeof value === 'string') {
      result[newKey] = value;
    }
  }
  return result;
}

function loadLocale(filename) {
  const raw = readFileSync(resolve(LOCALES_DIR, filename), 'utf-8');
  return flattenObject(JSON.parse(raw));
}

function getKeysUnderPrefix(flatKeys, prefix) {
  const p = prefix + '.';
  return Object.keys(flatKeys)
    .filter(k => k.startsWith(p))
    .map(k => k.slice(p.length));
}

function extractEnumValues(entry) {
  const filePath = resolve(SRC_DIR, entry.source.file);
  let content = readFileSync(filePath, 'utf-8');

  if (entry.source.startAfter) {
    const idx = content.indexOf(entry.source.startAfter);
    if (idx >= 0) content = content.slice(idx);
  }

  // Find the end of the type (semicolon)
  const semicolonIdx = content.indexOf(';');
  if (semicolonIdx >= 0) content = content.slice(0, semicolonIdx);

  const values = [];
  const regex = new RegExp(entry.source.pattern.source, entry.source.pattern.flags);
  let match;
  while ((match = regex.exec(content)) !== null) {
    values.push(match[1]);
  }
  return values;
}

// ============================================================================
// Main Check
// ============================================================================

export function dynamicKeysCheck() {
  const zhFlat = loadLocale('zh-CN.json');
  const esFlat = loadLocale('es-ES.json');
  const results = [];

  for (const entry of DYNAMIC_KEY_REGISTRY) {
    let expectedSuffixes;

    switch (entry.source.type) {
      case 'enum':
        expectedSuffixes = extractEnumValues(entry);
        break;

      case 'literal':
        expectedSuffixes = entry.source.values;
        break;

      case 'keys': {
        // Use zh-CN as source of truth, just check es-ES has them too
        const zhSuffixes = getKeysUnderPrefix(zhFlat, entry.prefix);
        if (entry.nested) {
          // For nested keys like system_issue.kind.abnormal_shutdown.title,
          // extract the first level after prefix
          const nestedKeys = new Set(zhSuffixes.map(s => s.split('.')[0]));
          expectedSuffixes = [...nestedKeys];
        } else {
          expectedSuffixes = zhSuffixes;
        }
        break;
      }
    }

    // Add extras
    if (entry.extras) {
      expectedSuffixes = [...expectedSuffixes, ...entry.extras];
    }

    // Check coverage in both locales
    const zhKeys = new Set(getKeysUnderPrefix(zhFlat, entry.prefix));
    const esKeys = new Set(getKeysUnderPrefix(esFlat, entry.prefix));

    const missingInZh = [];
    const missingInEs = [];

    for (const suffix of expectedSuffixes) {
      if (entry.nested) {
        // For nested, check that at least one key with this prefix exists
        const hasInZh = [...zhKeys].some(k => k.startsWith(suffix + '.') || k === suffix);
        const hasInEs = [...esKeys].some(k => k.startsWith(suffix + '.') || k === suffix);
        if (!hasInZh) missingInZh.push(suffix);
        if (!hasInEs) missingInEs.push(suffix);
      } else {
        if (!zhKeys.has(suffix)) missingInZh.push(suffix);
        if (!esKeys.has(suffix)) missingInEs.push(suffix);
      }
    }

    results.push({
      prefix: entry.prefix,
      expected: expectedSuffixes.length,
      coveredZh: expectedSuffixes.length - missingInZh.length,
      coveredEs: expectedSuffixes.length - missingInEs.length,
      missingInZh,
      missingInEs,
    });
  }

  return results;
}

// CLI output
if (import.meta.url === `file://${process.argv[1]}`) {
  const results = dynamicKeysCheck();
  let totalMissing = 0;

  console.log('\n=== Dynamic Key Coverage Check ===\n');

  for (const r of results) {
    const allCovered = r.missingInZh.length === 0 && r.missingInEs.length === 0;
    const icon = allCovered ? '\x1b[32m✓\x1b[0m' : '\x1b[31m✗\x1b[0m';

    if (allCovered) {
      console.log(`${icon} ${r.prefix}: ${r.expected}/${r.expected} covered`);
    } else {
      console.log(`${icon} ${r.prefix}: zh-CN ${r.coveredZh}/${r.expected}, es-ES ${r.coveredEs}/${r.expected}`);
      if (r.missingInZh.length > 0) {
        console.log(`    Missing in zh-CN: ${r.missingInZh.join(', ')}`);
      }
      if (r.missingInEs.length > 0) {
        console.log(`    Missing in es-ES: ${r.missingInEs.join(', ')}`);
      }
      totalMissing += r.missingInZh.length + r.missingInEs.length;
    }
  }

  console.log();
  if (totalMissing > 0) {
    console.log(`\x1b[31mTotal missing dynamic keys: ${totalMissing}\x1b[0m`);
    process.exit(1);
  } else {
    console.log('\x1b[32mAll dynamic keys covered!\x1b[0m');
  }
}
