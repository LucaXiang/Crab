#!/usr/bin/env node

/**
 * sync-check.mjs — Check translation key consistency between locales
 *
 * Compares zh-CN.json and es-ES.json to find keys that exist
 * in one locale but not the other.
 *
 * Usage: node scripts/i18n/sync-check.mjs
 * Exit code: 0 if synced, 1 if differences found
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCALES_DIR = resolve(__dirname, '../../src/infrastructure/i18n/locales');

// Flatten nested JSON object into dot-notation keys
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

function groupByNamespace(keys) {
  const groups = {};
  for (const key of keys) {
    const ns = key.split('.').slice(0, 2).join('.');
    if (!groups[ns]) groups[ns] = [];
    groups[ns].push(key);
  }
  return groups;
}

export function syncCheck() {
  const zhKeys = new Set(Object.keys(loadLocale('zh-CN.json')));
  const esKeys = new Set(Object.keys(loadLocale('es-ES.json')));

  const onlyInZh = [...zhKeys].filter(k => !esKeys.has(k)).sort();
  const onlyInEs = [...esKeys].filter(k => !zhKeys.has(k)).sort();
  const commonCount = [...zhKeys].filter(k => esKeys.has(k)).length;

  return { onlyInZh, onlyInEs, commonCount };
}

// Run as CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  const { onlyInZh, onlyInEs, commonCount } = syncCheck();
  const hasIssues = onlyInZh.length > 0 || onlyInEs.length > 0;

  console.log('\n=== Locale Sync Check (zh-CN <-> es-ES) ===\n');

  if (onlyInZh.length > 0) {
    console.log(`\x1b[31m✗ Only in zh-CN (missing in es-ES): ${onlyInZh.length} keys\x1b[0m`);
    const groups = groupByNamespace(onlyInZh);
    for (const [ns, keys] of Object.entries(groups)) {
      console.log(`  [${ns}]`);
      for (const k of keys) console.log(`    - ${k}`);
    }
    console.log();
  }

  if (onlyInEs.length > 0) {
    console.log(`\x1b[31m✗ Only in es-ES (missing in zh-CN): ${onlyInEs.length} keys\x1b[0m`);
    const groups = groupByNamespace(onlyInEs);
    for (const [ns, keys] of Object.entries(groups)) {
      console.log(`  [${ns}]`);
      for (const k of keys) console.log(`    - ${k}`);
    }
    console.log();
  }

  console.log(`\x1b[32m✓ Synced keys: ${commonCount}\x1b[0m\n`);

  if (hasIssues) {
    console.log(`Total issues: ${onlyInZh.length + onlyInEs.length}`);
    process.exit(1);
  } else {
    console.log('All keys are in sync!');
  }
}
