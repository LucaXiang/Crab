const fs = require('fs');
const path = require('path');

const localeFile = 'src/infrastructure/i18n/locales/zh-CN.json';
const srcDir = 'src';

// 扁平化 JSON 对象
function flattenObject(obj, prefix = '') {
  const result = {};
  for (const key in obj) {
    const newKey = prefix ? `${prefix}.${key}` : key;
    if (typeof obj[key] === 'object' && obj[key] !== null) {
      Object.assign(result, flattenObject(obj[key], newKey));
    } else {
      result[newKey] = obj[key];
    }
  }
  return result;
}

// 递归获取所有源文件
function getSourceFiles(dir, files = []) {
  const items = fs.readdirSync(dir);
  for (const item of items) {
    const fullPath = path.join(dir, item);
    const stat = fs.statSync(fullPath);
    if (stat.isDirectory() && !item.includes('node_modules') && !item.includes('generated')) {
      getSourceFiles(fullPath, files);
    } else if (/\.(ts|tsx)$/.test(item) && !item.includes('.d.ts')) {
      files.push(fullPath);
    }
  }
  return files;
}

// 动态键值模式 - 这些是通过变量访问的，不应该被标记为未使用
const dynamicPrefixes = [
  'errors.',
  'checkout.payment_method.',
  'order.status.',
  'system_issue.kind.',
  'system_issue.option.',
  'calendar.days.',
  'settings.price_rule.type.',
  'settings.price_rule.scope.',
  'settings.price_rule.time.',
  'settings.price_rule.adjustment.',
  'settings.price_rule.zone.',
  'settings.attribute.type.',
  'settings.shift.status.',
  'settings.shift.variance.',
  'common.status.',
  'timeline.labels.',
  'auth.roles.',
  'permissions.',
  // 更多动态前缀
  'activation.hint.',
  'activation.reason.',
  'audit.action.',
  'audit.detail.',
  'audit.filter.',
  'audit.group.',
  'audit.resource_type.',
  'checkout.void.',
  'checkout.void_reason.',
  'checkout.comp.preset.',
  'history.loss_reason.',
  'history.void_type.',
  'subscription.status.',
  'subscriptionBlocked.message.',
  'subscriptionBlocked.planType.',
  'timeline.operation.',
];

// 加载翻译文件
const translations = JSON.parse(fs.readFileSync(localeFile, 'utf8'));
const allKeys = Object.keys(flattenObject(translations));

// 加载 key-migration-map (旧key -> 新key)
const migrationMap = JSON.parse(fs.readFileSync('src/infrastructure/i18n/key-migration-map.json', 'utf8'));
const migratedKeys = new Set(Object.values(migrationMap));

// 获取所有源文件内容
const sourceFiles = getSourceFiles(srcDir);
let allSourceCode = '';
for (const file of sourceFiles) {
  allSourceCode += fs.readFileSync(file, 'utf8') + '\n';
}

// 检查每个键是否被使用
const unusedKeys = [];
let directCount = 0;
let dynamicCount = 0;

let migratedCount = 0;

for (const key of allKeys) {
  // 检查是否是动态前缀
  const isDynamic = dynamicPrefixes.some(prefix => key.startsWith(prefix));

  if (isDynamic) {
    dynamicCount++;
    continue;
  }

  // 检查是否通过 migration-map 映射使用
  if (migratedKeys.has(key)) {
    migratedCount++;
    continue;
  }

  // 检查是否在源代码中被引用
  const patterns = [
    "'" + key + "'",
    '"' + key + '"',
    '`' + key + '`',
  ];

  const isUsed = patterns.some(pattern => allSourceCode.includes(pattern));

  if (isUsed) {
    directCount++;
  } else {
    unusedKeys.push(key);
  }
}

console.log('\n📊 统计:');
console.log('   总键数: ' + allKeys.length);
console.log('   已使用: ' + (directCount + dynamicCount + migratedCount) + ' (直接: ' + directCount + ', 动态: ' + dynamicCount + ', 映射: ' + migratedCount + ')');
console.log('   未使用: ' + unusedKeys.length);

if (unusedKeys.length > 0) {
  console.log('\n⚠️  可能未使用的键 (' + unusedKeys.length + ' 个):\n');

  // 按前缀分组
  const grouped = {};
  for (const key of unusedKeys) {
    const prefix = key.split('.').slice(0, 2).join('.');
    if (!grouped[prefix]) grouped[prefix] = [];
    grouped[prefix].push(key);
  }

  for (const prefix of Object.keys(grouped).sort()) {
    const keys = grouped[prefix];
    console.log('[' + prefix + ']');
    for (let i = 0; i < Math.min(keys.length, 15); i++) {
      console.log('  - ' + keys[i]);
    }
    if (keys.length > 15) {
      console.log('  ... 还有 ' + (keys.length - 15) + ' 个');
    }
  }
}
