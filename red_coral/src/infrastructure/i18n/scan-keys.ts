/**
 * i18n 键值全面扫描脚本
 * 用法: npx ts-node src/infrastructure/i18n/scan-keys.ts
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// 要扫描的目录
const SCAN_DIRS = [
  'src/screens',
  'src/presentation',
  'src/core',
  'src/hooks',
  'src/utils',
];

const FILE_EXTENSIONS = ['.ts', '.tsx'];

// 获取所有 JSON 键值路径
function getAllJsonKeys(obj: Record<string, unknown>, prefix = ''): string[] {
  const keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      keys.push(...getAllJsonKeys(value as Record<string, unknown>, fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

// 递归获取所有文件
function getAllFiles(dir: string, baseDir: string): string[] {
  const fullDir = path.join(baseDir, dir);
  if (!fs.existsSync(fullDir)) return [];

  const files: string[] = [];
  const entries = fs.readdirSync(fullDir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(fullDir, entry.name);
    if (entry.isDirectory()) {
      files.push(...getAllFiles(path.join(dir, entry.name), baseDir));
    } else if (FILE_EXTENSIONS.some(ext => entry.name.endsWith(ext))) {
      files.push(fullPath);
    }
  }

  return files;
}

interface KeyUsage {
  key: string;
  file: string;
  line: number;
  isDynamic: boolean;
}

// 从代码中提取所有使用的键值
function extractUsedKeys(filePath: string): KeyUsage[] {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const usages: KeyUsage[] = [];

  // 静态键: t('key') 或 t("key")
  const staticPattern = /t\(['"]([^'"]+)['"]\)/g;
  // 动态键: t(`...${...}...`)
  const dynamicPattern = /t\(`([^`]+)`\)/g;

  lines.forEach((line, index) => {
    // 跳过注释行
    if (line.trim().startsWith('//') || line.trim().startsWith('*')) return;

    let match;

    // 静态键
    while ((match = staticPattern.exec(line)) !== null) {
      usages.push({
        key: match[1],
        file: filePath,
        line: index + 1,
        isDynamic: false,
      });
    }

    // 动态键
    while ((match = dynamicPattern.exec(line)) !== null) {
      const template = match[1];
      if (template.includes('${')) {
        usages.push({
          key: template,
          file: filePath,
          line: index + 1,
          isDynamic: true,
        });
      }
    }
  });

  return usages;
}

async function main() {
  const baseDir = path.resolve(__dirname, '../../..');
  const localesDir = path.join(__dirname, 'locales');

  console.log('\n🔍 i18n 键值全面扫描\n');
  console.log('='.repeat(60));

  // 1. 读取定义的键值
  const zhCNPath = path.join(localesDir, 'zh-CN.json');
  const zhCN = JSON.parse(fs.readFileSync(zhCNPath, 'utf-8'));
  const definedKeys = new Set(getAllJsonKeys(zhCN));

  console.log(`\n📚 已定义键值: ${definedKeys.size} 个\n`);

  // 2. 扫描代码中使用的键值
  const allFiles: string[] = [];
  for (const dir of SCAN_DIRS) {
    allFiles.push(...getAllFiles(dir, baseDir));
  }

  console.log(`📂 扫描文件: ${allFiles.length} 个\n`);

  const allUsages: KeyUsage[] = [];
  for (const file of allFiles) {
    allUsages.push(...extractUsedKeys(file));
  }

  // 分离静态和动态键
  const staticUsages = allUsages.filter(u => !u.isDynamic);
  const dynamicUsages = allUsages.filter(u => u.isDynamic);

  const usedStaticKeys = new Set(staticUsages.map(u => u.key));

  console.log(`📝 代码中使用的静态键: ${usedStaticKeys.size} 个`);
  console.log(`🔄 代码中使用的动态键: ${dynamicUsages.length} 处\n`);

  // 3. 找出缺失的键 (代码中使用但未定义)
  const missingKeys = [...usedStaticKeys].filter(key => !definedKeys.has(key));

  // 4. 找出未使用的键 (已定义但代码中未使用)
  // 注意: 需要排除动态键可能匹配的前缀
  const dynamicPrefixes = new Set<string>();
  for (const usage of dynamicUsages) {
    // 提取动态键的静态前缀部分
    const prefix = usage.key.split('${')[0];
    if (prefix) {
      dynamicPrefixes.add(prefix);
    }
  }

  const unusedKeys = [...definedKeys].filter(key => {
    if (usedStaticKeys.has(key)) return false;
    // 检查是否可能被动态键使用
    for (const prefix of dynamicPrefixes) {
      if (key.startsWith(prefix)) return false;
    }
    return true;
  });

  // 5. 输出报告
  console.log('='.repeat(60));
  console.log('\n❌ 缺失的键值 (代码中使用但未定义):\n');

  if (missingKeys.length === 0) {
    console.log('   ✅ 无缺失键值\n');
  } else {
    // 按前缀分组
    const missingByPrefix = new Map<string, string[]>();
    for (const key of missingKeys.sort()) {
      const prefix = key.split('.')[0];
      if (!missingByPrefix.has(prefix)) {
        missingByPrefix.set(prefix, []);
      }
      missingByPrefix.get(prefix)!.push(key);
    }

    for (const [prefix, keys] of missingByPrefix) {
      console.log(`   [${prefix}]`);
      for (const key of keys) {
        // 找到使用这个键的位置
        const usage = staticUsages.find(u => u.key === key);
        const location = usage
          ? `${path.relative(baseDir, usage.file)}:${usage.line}`
          : '';
        console.log(`      - ${key}`);
        if (location) {
          console.log(`        └─ ${location}`);
        }
      }
    }
    console.log(`\n   共 ${missingKeys.length} 个缺失键值\n`);
  }

  console.log('='.repeat(60));
  console.log('\n⚠️  未使用的键值 (已定义但代码中未使用):\n');

  if (unusedKeys.length === 0) {
    console.log('   ✅ 无未使用键值\n');
  } else {
    // 按前缀分组
    const unusedByPrefix = new Map<string, string[]>();
    for (const key of unusedKeys.sort()) {
      const prefix = key.split('.')[0];
      if (!unusedByPrefix.has(prefix)) {
        unusedByPrefix.set(prefix, []);
      }
      unusedByPrefix.get(prefix)!.push(key);
    }

    for (const [prefix, keys] of unusedByPrefix) {
      console.log(`   [${prefix}] (${keys.length} 个)`);
      // 只显示前 10 个
      const displayKeys = keys.slice(0, 10);
      for (const key of displayKeys) {
        console.log(`      - ${key}`);
      }
      if (keys.length > 10) {
        console.log(`      ... 还有 ${keys.length - 10} 个`);
      }
    }
    console.log(`\n   共 ${unusedKeys.length} 个未使用键值\n`);
  }

  console.log('='.repeat(60));
  console.log('\n🔄 动态键值模式:\n');

  if (dynamicUsages.length === 0) {
    console.log('   无动态键值\n');
  } else {
    const uniqueDynamic = [...new Set(dynamicUsages.map(u => u.key))];
    for (const pattern of uniqueDynamic) {
      const usages = dynamicUsages.filter(u => u.key === pattern);
      console.log(`   - \`${pattern}\``);
      console.log(`     使用 ${usages.length} 次`);
    }
    console.log('');
  }

  // 6. 总结
  console.log('='.repeat(60));
  console.log('\n📊 总结:\n');
  console.log(`   定义键值:     ${definedKeys.size} 个`);
  console.log(`   使用静态键:   ${usedStaticKeys.size} 个`);
  console.log(`   缺失键值:     ${missingKeys.length} 个`);
  console.log(`   未使用键值:   ${unusedKeys.length} 个`);
  console.log(`   动态键模式:   ${[...new Set(dynamicUsages.map(u => u.key))].length} 个\n`);

  // 计算覆盖率
  const coverage = ((usedStaticKeys.size - missingKeys.length) / definedKeys.size * 100).toFixed(1);
  console.log(`   键值覆盖率:   ${coverage}%\n`);

  // 返回结果供进一步处理
  return {
    definedKeys: [...definedKeys],
    usedKeys: [...usedStaticKeys],
    missingKeys,
    unusedKeys,
    dynamicPatterns: [...new Set(dynamicUsages.map(u => u.key))],
  };
}

main().catch(console.error);
