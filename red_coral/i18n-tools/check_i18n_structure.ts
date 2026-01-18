#!/usr/bin/env -S deno run --allow-read

interface Issue {
  type: 'duplicate' | 'empty' | 'inconsistent' | 'unused';
  path: string;
  description: string;
  value?: any;
}

function checkI18nStructure(obj: any, basePath = ''): Issue[] {
  const issues: Issue[] = [];
  const keys = Object.keys(obj);

  // 检查重复键（大小写不敏感）
  const keyMap = new Map<string, string[]>();
  keys.forEach(key => {
    const lowerKey = key.toLowerCase();
    if (!keyMap.has(lowerKey)) {
      keyMap.set(lowerKey, []);
    }
    keyMap.get(lowerKey)!.push(key);
  });

  keyMap.forEach((variants, lowerKey) => {
    if (variants.length > 1) {
      issues.push({
        type: 'duplicate',
        path: basePath,
        description: `可能的重复键（不同大小写）: ${variants.join(', ')}`,
      });
    }
  });

  // 检查空对象
  keys.forEach(key => {
    const fullPath = basePath ? `${basePath}.${key}` : key;
    const value = obj[key];

    if (typeof value === 'object' && value !== null) {
      if (Object.keys(value).length === 0) {
        issues.push({
          type: 'empty',
          path: fullPath,
          description: '空对象，没有任何翻译内容',
          value: value,
        });
      } else {
        // 递归检查子对象
        issues.push(...checkI18nStructure(value, fullPath));
      }
    }
  });

  return issues;
}

function findPathConflicts(obj: any, basePath = '', allPaths = new Set<string>()): Issue[] {
  const issues: Issue[] = [];
  const keys = Object.keys(obj);

  keys.forEach(key => {
    const fullPath = basePath ? `${basePath}.${key}` : key;
    const value = obj[key];

    // 检查路径冲突：某个路径既是对象又有字符串值
    if (typeof value === 'object' && value !== null) {
      // 这是一个对象节点
      if (allPaths.has(fullPath)) {
        issues.push({
          type: 'inconsistent',
          path: fullPath,
          description: '路径冲突：该路径既是分类节点又是值节点',
        });
      }
      allPaths.add(fullPath);
      issues.push(...findPathConflicts(value, fullPath, allPaths));
    } else {
      // 这是一个叶子节点
      allPaths.add(fullPath);
    }
  });

  return issues;
}

function findDuplicateValues(obj: any, basePath = '', valueMap = new Map<string, string[]>()): Issue[] {
  const issues: Issue[] = [];
  const keys = Object.keys(obj);

  keys.forEach(key => {
    const fullPath = basePath ? `${basePath}.${key}` : key;
    const value = obj[key];

    if (typeof value === 'object' && value !== null) {
      findDuplicateValues(value, fullPath, valueMap);
    } else if (typeof value === 'string') {
      if (!valueMap.has(value)) {
        valueMap.set(value, []);
      }
      valueMap.get(value)!.push(fullPath);
    }
  });

  // 只在最顶层返回重复值报告
  if (basePath === '') {
    valueMap.forEach((paths, value) => {
      if (paths.length > 1 && value.trim() !== '') {
        issues.push({
          type: 'duplicate',
          path: paths.join(' | '),
          description: `重复的翻译值: "${value}"`,
          value: paths,
        });
      }
    });
  }

  return issues;
}

function analyzeStructure(obj: any) {
  console.log('\n=================================');
  console.log('🔍 i18n 配置文件结构分析');
  console.log('=================================\n');

  // 1. 检查空对象和键名问题
  console.log('📋 检查空对象和键名问题...\n');
  const structureIssues = checkI18nStructure(obj);
  const emptyObjects = structureIssues.filter(i => i.type === 'empty');
  const duplicateKeys = structureIssues.filter(i => i.type === 'duplicate');

  if (emptyObjects.length > 0) {
    console.log('⚠️  发现空对象:');
    emptyObjects.forEach(issue => {
      console.log(`   - ${issue.path}: ${issue.description}`);
    });
    console.log();
  }

  if (duplicateKeys.length > 0) {
    console.log('⚠️  发现可能的重复键:');
    duplicateKeys.forEach(issue => {
      console.log(`   - ${issue.path}: ${issue.description}`);
    });
    console.log();
  }

  // 2. 检查路径冲突
  console.log('📋 检查路径冲突...\n');
  const pathConflicts = findPathConflicts(obj);
  if (pathConflicts.length > 0) {
    console.log('⚠️  发现路径冲突:');
    pathConflicts.forEach(issue => {
      console.log(`   - ${issue.path}: ${issue.description}`);
    });
    console.log();
  } else {
    console.log('✅ 未发现路径冲突\n');
  }

  // 3. 检查重复的翻译值
  console.log('📋 检查重复的翻译值（前20个）...\n');
  const duplicateValues = findDuplicateValues(obj);
  if (duplicateValues.length > 0) {
    console.log(`⚠️  发现 ${duplicateValues.length} 组重复的翻译值:\n`);
    duplicateValues.slice(0, 20).forEach((issue, index) => {
      console.log(`   ${index + 1}. "${issue.description}"`);
      if (Array.isArray(issue.value)) {
        issue.value.forEach(path => {
          console.log(`      - ${path}`);
        });
      }
      console.log();
    });
    if (duplicateValues.length > 20) {
      console.log(`   ... 还有 ${duplicateValues.length - 20} 组重复值\n`);
    }
  } else {
    console.log('✅ 未发现重复的翻译值\n');
  }

  // 4. 统计信息
  console.log('=================================');
  console.log('📊 统计信息');
  console.log('=================================\n');

  function countNodes(obj: any): { total: number; leaves: number; branches: number } {
    let total = 0;
    let leaves = 0;
    let branches = 0;

    Object.keys(obj).forEach(key => {
      total++;
      const value = obj[key];
      if (typeof value === 'object' && value !== null) {
        branches++;
        const sub = countNodes(value);
        total += sub.total;
        leaves += sub.leaves;
        branches += sub.branches;
      } else {
        leaves++;
      }
    });

    return { total, leaves, branches };
  }

  const stats = countNodes(obj);
  console.log(`   总键数: ${stats.total}`);
  console.log(`   翻译字符串数: ${stats.leaves}`);
  console.log(`   分类节点数: ${stats.branches}`);
  console.log(`   空对象数: ${emptyObjects.length}`);
  console.log(`   重复翻译值组数: ${duplicateValues.length}\n`);

  // 5. 总结
  console.log('=================================');
  const totalIssues = emptyObjects.length + duplicateKeys.length + pathConflicts.length;
  if (totalIssues === 0) {
    console.log('✅ 结构检查完成，未发现严重问题！');
  } else {
    console.log(`⚠️  发现 ${totalIssues} 个需要注意的问题`);
  }
  console.log('=================================\n');
}

// 主函数
async function main() {
  const args = Deno.args;

  if (args.length === 0) {
    console.log('用法: deno run --allow-read check_i18n_structure.ts <json文件路径>');
    Deno.exit(1);
  }

  const filePath = args[0];

  try {
    const content = await Deno.readTextFile(filePath);
    const data = JSON.parse(content);
    analyzeStructure(data);
  } catch (error) {
    console.error('❌ 错误:', error.message);
    Deno.exit(1);
  }
}

if (import.meta.main) {
  main();
}
