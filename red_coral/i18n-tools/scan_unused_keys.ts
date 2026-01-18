#!/usr/bin/env -S deno run --allow-read --allow-write

import { walk } from "https://deno.land/std@0.208.0/fs/walk.ts";
import { parse } from "https://deno.land/std@0.208.0/path/mod.ts";
import { format } from "https://deno.land/std@0.208.0/datetime/mod.ts";

// 配置
const config = {
  srcDir: "../src",
  localesDir: "../src/services/i18n/locales",
  localeFiles: ["en-US.json", "zh-CN.json"],
  // 检查 locale 文件是否存在
  skipMissingLocales: true,
  // 排除的目录
  excludeDirs: ["node_modules", "dist", "build", ".git", ".deno"],
  // t() 函数别名
  tFunctions: ["t", "translate"],
  // 是否自动删除
  dryRun: true,
};

interface KeyUsage {
  key: string;
  files: string[];
  lineNumbers: number[];
}

/**
 * 扫描文件中的 t() 调用
 */
function scanFileForKeys(filePath: string, content: string): Map<string, number[]> {
  const usages = new Map<string, number[]>();

  // 匹配 t("key") 或 t('key') 或 t(`key`
  const patterns = [
    // t("key") 或 t('key')
    /\bt\(\s*["']([^"'\n]+)["']\s*\)/g,
    // t(`key`)
    /`\s*\bt\s*\(\s*([^`\n]+)\s*\)\s*`/g,
  ];

  for (const pattern of patterns) {
    let match;
    while ((match = pattern.exec(content)) !== null) {
      const key = match[1].trim();
      if (key && !key.includes('${')) { // 排除动态 key
        const lineNo = content.substring(0, match.index).split('\n').length;
        const existing = usages.get(key) || [];
        usages.set(key, [...existing, lineNo]);
      }
    }
  }

  return usages;
}

/**
 * 读取所有 locale 文件
 */
async function readLocaleFiles(): Promise<Record<string, Record<string, string>>> {
  const locales: Record<string, Record<string, string>> = {};

  for (const file of config.localeFiles) {
    const filePath = `${config.localesDir}/${file}`;
    try {
      const content = await Deno.readTextFile(filePath);
      const json = JSON.parse(content);
      // 展平嵌套对象
      locales[file] = flattenObject(json);
      console.log(`  📖 读取 ${file}: ${Object.keys(locales[file]).length} 个 key`);
    } catch (error) {
      if (config.skipMissingLocales) {
        console.log(`  ⏭️  跳过 ${file} (文件不存在)`);
      } else {
        console.warn(`  ⚠️  无法读取 ${file}: ${error.message}`);
      }
    }
  }

  return locales;
}

/**
 * 展平嵌套对象为单层 key
 */
function flattenObject(obj: Record<string, any>, prefix: string = ''): Record<string, string> {
  const result: Record<string, string> = {};

  for (const [key, value] of Object.entries(obj)) {
    const newKey = prefix ? `${prefix}.${key}` : key;

    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      Object.assign(result, flattenObject(value, newKey));
    } else {
      result[newKey] = String(value);
    }
  }

  return result;
}

/**
 * 统计所有 locale 中的 key
 */
function getAllKeys(locales: Record<string, Record<string, string>>): Set<string> {
  const keys = new Set<string>();
  for (const locale of Object.values(locales)) {
    for (const key of Object.keys(locale)) {
      keys.add(key);
    }
  }
  return keys;
}

/**
 * 主函数
 */
async function main() {
  console.log('🔍 扫描未使用的 i18n keys...\n');

  // 1. 扫描源码中的 t() 调用
  console.log('📂 扫描源码文件...');
  const allUsages = new Map<string, KeyUsage>();

  for await (const entry of walk(config.srcDir, {
    includeDirs: false,
    skip: config.excludeDirs.map(d => new RegExp(d)),
  })) {
    const ext = parse(entry.path).ext;
    if (['.tsx', '.ts', '.jsx', '.js'].includes(ext)) {
      try {
        const content = await Deno.readTextFile(entry.path);
        const usages = scanFileForKeys(entry.path, content);

        for (const [key, lineNumbers] of usages) {
          const existing = allUsages.get(key) || {
            key,
            files: [],
            lineNumbers: [],
          };
          existing.files.push(entry.path);
          existing.lineNumbers.push(...lineNumbers);
          allUsages.set(key, existing);
        }
      } catch {
        // 忽略读取错误
      }
    }
  }

  console.log(`  📊 在源码中找到 ${allUsages.size} 个 key\n`);

  // 2. 读取 locale 文件
  console.log('📚 读取 locale 文件...');
  const locales = await readLocaleFiles();
  const allKeys = getAllKeys(locales);
  console.log('');

  // 3. 找出未使用的 keys
  const unusedKeys: KeyUsage[] = [];
  const missingKeys: KeyUsage[] = [];

  for (const key of allKeys) {
    if (!allUsages.has(key)) {
      const usage = allUsages.get(key) || { key, files: [], lineNumbers: [] };
      unusedKeys.push(usage);
    }
  }

  // 4. 找出源码中使用但 locale 中缺失的 keys
  for (const [key, usage] of allUsages) {
    if (!allKeys.has(key)) {
      missingKeys.push(usage);
    }
  }

  // 5. 输出结果
  console.log('='.repeat(60));

  // 未使用的 keys
  if (unusedKeys.length > 0) {
    console.log(`\n🗑️  未使用的 keys (${unusedKeys.length} 个):\n`);

    for (const item of unusedKeys.sort((a, b) => a.key.localeCompare(b.key))) {
      console.log(`  • ${item.key}`);
    }
  } else {
    console.log('\n✅ 没有发现未使用的 keys');
  }

  // 缺失的 keys
  if (missingKeys.length > 0) {
    console.log(`\n⚠️  locale 中缺失的 keys (${missingKeys.length} 个):\n`);

    for (const item of missingKeys) {
      console.log(`  • ${item.key}`);
      console.log(`    使用位置: ${item.files[0]}:${item.lineNumbers[0]}`);
      if (item.files.length > 1) {
        console.log(`    ... 共 ${item.files.length} 处使用`);
      }
    }
  }

  // 6. 执行清理（如果需要）
  if (unusedKeys.length > 0 && !config.dryRun) {
    console.log('\n' + '='.repeat(60));
    console.log('\n🧹 清理未使用的 keys...\n');

    for (const file of config.localeFiles) {
      if (!locales[file]) continue;

      let removedCount = 0;
      const content = await Deno.readTextFile(`${config.localesDir}/${file}`);
      const json = JSON.parse(content);

      for (const item of unusedKeys) {
        if (deleteNestedKey(json, item.key)) {
          removedCount++;
        }
      }

      if (removedCount > 0) {
        const newContent = JSON.stringify(json, null, 2) + '\n';
        await Deno.writeTextFile(`${config.localesDir}/${file}`, newContent);
        console.log(`  ✅ ${file}: 删除了 ${removedCount} 个 key`);
      } else {
        console.log(`  ℹ️  ${file}: 没有删除任何 key`);
      }
    }

    // 生成备份报告
    const timestamp = format(new Date(), 'yyyy-MM-dd_HH-mm-ss');
    const reportPath = `./unused_keys_${timestamp}.json`;
    const report = unusedKeys.map(k => k.key);
    await Deno.writeTextFile(reportPath, JSON.stringify(report, null, 2));
    console.log(`\n📄 已生成备份报告: ${reportPath}`);
  } else if (unusedKeys.length > 0) {
    console.log('\n' + '='.repeat(60));
    console.log('\n💡 使用 --write 参数来删除未使用的 keys');
  }

  console.log('\n' + '='.repeat(60));
  console.log(`\n📊 统计: ${allUsages.size} 个使用的 key, ${allKeys.size} 个 locale key`);
  console.log(`📊 未使用: ${unusedKeys.length} 个, 缺失: ${missingKeys.length} 个\n`);
}

/**
 * 删除嵌套对象中的 key
 */
function deleteNestedKey(obj: Record<string, any>, key: string): boolean {
  const parts = key.split('.');
  let current = obj;

  for (let i = 0; i < parts.length - 1; i++) {
    if (!(parts[i] in current)) return false;
    current = current[parts[i]];
  }

  const lastPart = parts[parts.length - 1];
  if (lastPart in current) {
    delete current[lastPart];
    return true;
  }

  return false;
}

// 解析命令行参数
const args = Deno.args;
if (args.includes('--write')) {
  config.dryRun = false;
}
if (args.includes('--help') || args.includes('-h')) {
  console.log(`
用法: deno run --allow-read --allow-write scan_unused_keys.ts [选项]

选项:
  --write      实际删除未使用的 keys (默认是 dry-run)
  --help, -h   显示帮助信息

示例:
  deno run --allow-read --allow-write scan_unused_keys.ts
  deno run --allow-read --allow-write scan_unused_keys.ts --write
`);
  Deno.exit(0);
}

main();
