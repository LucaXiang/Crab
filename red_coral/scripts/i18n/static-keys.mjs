#!/usr/bin/env node

/**
 * static-keys.mjs — Find unused and missing static i18n keys
 *
 * Scans all .ts/.tsx source files for t('key') calls, compares against
 * translation JSON files, and reports:
 * - Unused keys: in JSON but not referenced in any source file
 * - Missing keys: referenced in source but not in JSON
 *
 * Dynamic keys (template literals) are excluded — handled by dynamic-keys.mjs.
 *
 * Usage: node scripts/i18n/static-keys.mjs [--unused | --missing]
 */

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, dirname, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCALES_DIR = resolve(__dirname, '../../src/infrastructure/i18n/locales');
const SRC_DIR = resolve(__dirname, '../../src');

// ============================================================================
// Dynamic key prefixes to exclude (handled by dynamic-keys.mjs)
// ============================================================================

const DYNAMIC_PREFIXES = [
  'commandError.',
  'calendar.days.',
  'checkout.comp.preset.',
  'checkout.delete_item.reason.',
  'checkout.void.cancel_reason.',
  'checkout.void.loss_reason.',
  'audit.action.',
  'audit.resource_type.',
  'audit.group.',
  'audit.detail.value.',
  'audit.detail.field.',       // `audit.detail.field.${field}` in AuditLog renderers
  'errors.',                   // `errors.${code}` in tauri-client & error/index
  'history.void_type.',
  'history.loss_reason.',
  'statistics.status.',
  'statistics.time.',
  'system_issue.kind.',
  'system_issue.option.',
  'activation.reason.',
  'activation.hint.',
  'subscription.status.',
  'subscriptionBlocked.',
  'settings.price_rule.scope.',
  'settings.price_rule.stacking.',
  'settings.price_rule.preview.',
  'settings.marketing_group.scope.',
  'settings.marketing_group.stamp.target_type.',
  'settings.marketing_group.stamp.strategy.',
  'settings.marketing_group.stamp_wizard.strategy_',
  'settings.system.virtual_keyboard_',
  'settings.align',
  'error.friendly.',
];

function isDynamicKey(key) {
  return DYNAMIC_PREFIXES.some(prefix => key.startsWith(prefix));
}

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

function collectSourceFiles(dir, extensions = ['.ts', '.tsx']) {
  const files = [];
  function walk(d) {
    for (const entry of readdirSync(d)) {
      if (entry === 'node_modules' || entry === 'src-tauri' || entry === '.trae' || entry === '.shared') continue;
      const fullPath = resolve(d, entry);
      const stat = statSync(fullPath);
      if (stat.isDirectory()) {
        walk(fullPath);
      } else if (extensions.includes(extname(entry))) {
        files.push(fullPath);
      }
    }
  }
  walk(dir);
  return files;
}

/**
 * Extract all static t('key') calls from source files.
 * Matches:
 * - t('some.key')
 * - t('some.key', { param: value })
 * - translate('some.key')
 * Does NOT match template literals t(`...${var}...`)
 */
function extractStaticKeys(sourceFiles) {
  const keys = new Set();
  // Match: t('key') or t("key") — single/double quotes, no template literals
  const regex = /\bt\(\s*(['"])([^'"\n]+?)\1/g;

  for (const file of sourceFiles) {
    const content = readFileSync(file, 'utf-8');
    let match;
    while ((match = regex.exec(content)) !== null) {
      keys.add(match[2]);
    }
  }
  return keys;
}

function groupByNamespace(keys) {
  const groups = {};
  for (const key of keys) {
    const ns = key.split('.').slice(0, 2).join('.');
    if (!groups[ns]) groups[ns] = [];
    groups[ns].push(key);
  }
  return groups;
}

// ============================================================================
// Main
// ============================================================================

export function staticKeysCheck() {
  const zhFlat = loadLocale('zh-CN.json');
  const esFlat = loadLocale('es-ES.json');
  const zhKeys = new Set(Object.keys(zhFlat));
  const esKeys = new Set(Object.keys(esFlat));
  // Union of all locale keys
  const allLocaleKeys = new Set([...zhKeys, ...esKeys]);

  const sourceFiles = collectSourceFiles(SRC_DIR);
  const usedKeys = extractStaticKeys(sourceFiles);

  // Unused: in locale JSON but not referenced in code (excluding dynamic keys)
  const unusedKeys = [...allLocaleKeys]
    .filter(k => !isDynamicKey(k) && !usedKeys.has(k))
    .sort();

  // Missing: referenced in code but not in any locale JSON (excluding dynamic patterns)
  const missingKeys = [...usedKeys]
    .filter(k => !isDynamicKey(k) && !allLocaleKeys.has(k))
    .sort();

  return { unusedKeys, missingKeys, totalLocaleKeys: allLocaleKeys.size, totalUsedKeys: usedKeys.size };
}

// CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  const mode = process.argv[2]; // --unused, --missing, or none (both)
  const { unusedKeys, missingKeys, totalLocaleKeys, totalUsedKeys } = staticKeysCheck();

  console.log(`\n=== Static Key Analysis ===`);
  console.log(`Locale keys: ${totalLocaleKeys} | Code references: ${totalUsedKeys}\n`);

  if (mode !== '--missing') {
    if (unusedKeys.length > 0) {
      console.log(`\x1b[33m⚠ Unused keys (in JSON but not in code): ${unusedKeys.length}\x1b[0m`);
      const groups = groupByNamespace(unusedKeys);
      for (const [ns, keys] of Object.entries(groups)) {
        console.log(`  [${ns}] (${keys.length})`);
        for (const k of keys) console.log(`    - ${k}`);
      }
    } else {
      console.log('\x1b[32m✓ No unused keys\x1b[0m');
    }
    console.log();
  }

  if (mode !== '--unused') {
    if (missingKeys.length > 0) {
      console.log(`\x1b[31m✗ Missing keys (in code but not in JSON): ${missingKeys.length}\x1b[0m`);
      const groups = groupByNamespace(missingKeys);
      for (const [ns, keys] of Object.entries(groups)) {
        console.log(`  [${ns}] (${keys.length})`);
        for (const k of keys) console.log(`    - ${k}`);
      }
    } else {
      console.log('\x1b[32m✓ No missing keys\x1b[0m');
    }
  }

  const hasIssues = missingKeys.length > 0;
  if (hasIssues) {
    console.log(`\n\x1b[31mTotal: ${unusedKeys.length} unused, ${missingKeys.length} missing\x1b[0m`);
    process.exit(1);
  }
}
