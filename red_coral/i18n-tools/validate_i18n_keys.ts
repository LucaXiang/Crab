#!/usr/bin/env -S deno run --allow-read

import { parse } from "https://deno.land/std@0.208.0/path/mod.ts";

interface Issue {
  key: string;
  type: 'snake_case' | 'inconsistent_naming' | 'missing_form_structure' | 'suggestion';
  message: string;
  suggestion?: string;
}

function flattenObject(obj: any, prefix = ''): Record<string, string> {
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

function isSnakeCase(str: string): boolean {
  return /^[a-z]+_[a-z0-9_]+$/.test(str);
}

function isCamelCase(str: string): boolean {
  return /^[a-z]+[A-Z0-9]/.test(str) || /^[a-z]+$/.test(str);
}

function analyzeNamespace(key: string, value: string): Issue | null {
  const parts = key.split('.');
  const lastPart = parts[parts.length - 1];

  // 1. 检测 snake_case
  if (isSnakeCase(lastPart) && !key.startsWith('settings.permissions.')) {
    return {
      key,
      type: 'snake_case',
      message: `检测到 snake_case 命名: "${lastPart}"`,
      suggestion: lastPart.replace(/_([a-z])/g, (_, c) => c.toUpperCase())
    };
  }

  // 2. 检测不规范的表单命名
  const formKeys = ['form_name', 'form_name_placeholder', 'form_description'];
  if (formKeys.includes(lastPart.toLowerCase().replace(/-/g, '_'))) {
    return {
      key,
      type: 'missing_form_structure',
      message: `表单字段建议使用 form.* 结构`,
      suggestion: key.replace(lastPart, `form.${isSnakeCase(lastPart) ? lastPart.replace(/_([a-z])/g, (_, c) => c.toUpperCase()) : lastPart}`)
    };
  }

  // 3. 检测不一致的命名风格
  if (key.includes('save') && key.includes('Save')) {
    return {
      key,
      type: 'inconsistent_naming',
      message: `命名大小写不一致: "save" vs "Save"`
    };
  }

  return null;
}

async function main() {
  const localePath = Deno.args[0] || "../src/services/i18n/locales/zh-CN.json";
  console.log(`🔍 分析 ${localePath} 的命名规范...\n`);

  try {
    const content = await Deno.readTextFile(localePath);
    const data = JSON.parse(content);
    const keys = flattenObject(data);

    const issues: Issue[] = [];

    for (const [key, value] of Object.entries(keys)) {
      const issue = analyzeNamespace(key, value as string);
      if (issue) {
        issues.push(issue);
      }
    }

    console.log("=".repeat(60));

    if (issues.length === 0) {
      console.log("\n✅ 所有 keys 命名规范良好!\n");
    } else {
      console.log(`\n⚠️  发现 ${issues.length} 个命名规范问题:\n`);

      // 按类型分组
      const byType = new Map<string, Issue[]>();
      for (const issue of issues) {
        const list = byType.get(issue.type) || [];
        list.push(issue);
        byType.set(issue.type, list);
      }

      for (const [type, list] of byType) {
        console.log(`\n📋 ${type.toUpperCase().replace('_', ' ')} (${list.length} 个):\n`);
        for (const issue of list.slice(0, 20)) {
          console.log(`  • ${issue.key}`);
          console.log(`    ${issue.message}`);
          if (issue.suggestion) {
            console.log(`    建议改为: ${issue.suggestion}`);
          }
        }
        if (list.length > 20) {
          console.log(`    ... 还有 ${list.length - 20} 个`);
        }
      }
    }

    console.log("\n" + "=".repeat(60));
    console.log(`\n📊 统计: ${Object.keys(keys).length} 个 keys, ${issues.length} 个问题`);

  } catch (error) {
    console.error(`❌ 错误: ${error.message}`);
    Deno.exit(1);
  }
}

main();
