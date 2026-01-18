#!/usr/bin/env -S deno run --allow-read --allow-write

import { walk } from "https://deno.land/std@0.208.0/fs/walk.ts";
import { parse } from "https://deno.land/std@0.208.0/path/mod.ts";

// 配置
const config = {
  scanDirs: ["./src"],
  extensions: [".tsx", ".jsx"], // 只扫描 React 组件
  excludeDirs: ["src/services/i18n", "src/assets", "src/types", "node_modules", "dist", "build", ".git", "src/core/stores", "src/core/services"],
};

// 需要检测的文本属性
const TEXT_ATTRIBUTES = [
  'placeholder',
  'title',
  'alt',
  'label',
  'aria-label',
  'description',
  'tooltip',
  'message',
  'emptyMessage',
  'noResultsMessage',
  'loadingMessage',
];

// 正则表达式
// 限制 JSX 文本匹配：单行，不包含特殊字符
const JSX_TEXT_REGEX = />\s*([^\n<>{}]{1,100}?)\s*</g;
const ATTRIBUTE_REGEX = new RegExp(`\\b(${TEXT_ATTRIBUTES.join('|')})=(["'])(.*?)\\2`, 'g');

// 清理注释
function cleanContent(content: string): string {
  return content
    .replace(/\/\*[\s\S]*?\*\//g, '') // 块注释
    .replace(/\/\/.*/g, ''); // 行注释
}

// i18n key 模式检测
function isI18nKey(str: string): boolean {
  // 如 "errors.general.success", "common.ok"
  return /^[a-z][\w-]*(\.[a-z][\w-]*)+$/.test(str.trim());
}

// 判断是否是可疑的用户-facing 字符串
function isSuspiciousString(str: string): boolean {
  const s = str.trim();
  if (!s) return false;

  // 忽略 i18n keys
  if (isI18nKey(s)) return false;

  // 忽略数字
  if (/^\d+$/.test(s)) return false;

  // 忽略布尔值
  if (s === 'true' || s === 'false') return false;

  // 忽略单字词（无空格），可能是 class 或 ID
  if (/^[\w-]+$/.test(s) && !s.includes(' ')) return false;

  // 忽略颜色值
  if (/^#[\da-fA-F]{3,8}$/.test(s)) return false;
  if (/^rgba?\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*(,\s*[\d.]+)?\s*\)$/.test(s)) return false;
  if (/^hsla?\(\s*[\d.]+\s*,\s*[\d.]+%?\s*,\s*[\d.]+%?\s*(,\s*[\d.]+)?\s*\)$/.test(s)) return false;

  // 忽略 CSS 单位
  if (/^\d+(px|rem|em|%|vh|vw)$/.test(s)) return false;

  // 忽略文件路径
  if (/^(@|\.\.?\/|\/)[/\w-]+$/.test(s)) return false;

  // 代码特征检测
  if (s.includes(';') || s.includes('=>') || s.includes('==')) return false;
  if (s.includes('const ') || s.includes('let ') || s.includes('var ')) return false;
  if (s.startsWith('(') && s.endsWith(')')) return false; // 可能是三元表达式
  if (/^[(){}:|,.?&!]+$/.test(s)) return false; // 纯标点

  // 忽略条件渲染模式: "0 && (" 或 "condition && ("
  if (/^\d+\s*&&\s*\($/.test(s)) return false;
  if (/^[a-zA-Z_][\w]*\s*&&\s*\($/.test(s)) return false;
  if (/^\d+\s*&&\s*!?[a-zA-Z_][\w.]+\s*&&\s*\($/.test(s)) return false; // "0 && !isFullyPaid && ("

  // 忽略三元表达式模式: ") : (" 或 "? xxx :"
  if (/^\)\s*:\s*\($/.test(s)) return false;
  if (/^\d+\s*\?\s*\(/.test(s)) return false;
  if (/^[a-zA-Z_][\w]*\s*\?\s*\(/.test(s)) return false;
  if (/^\d+\)\s*&&\s*\($/.test(s)) return false; // "0) && ("

  // 忽略占位符示例模式: "{xxx}" 或 JSON 示例
  if (/^\{[\w":,\s]+\}$/.test(s)) return false;

  // 忽略类型定义模式: "| Type" 或 "= value &&"
  if (/^\|\s*[a-zA-Z]/.test(s)) return false;
  if (/^=\s*\d+\s*&&\s*[a-zA-Z]/.test(s)) return false;

  // 忽略类似代码的片段
  if (/^=\s*\d+\s*&&\s*x/.test(s)) return false;
  if (/^=\s*[a-zA-Z]+\.[a-zA-Z]+\s*&&\s*[a-z]/.test(s)) return false;

  // 忽略条件渲染模式: "0 &&" 或 "0 && xxx"
  if (/^\d+\s*&&\s*$/.test(s)) return false;
  if (/^\d+\s*&&\s*[a-zA-Z_][\w.]*$/.test(s)) return false;
  if (/^\d+\s*&&\s*![a-zA-Z_][\w.]*$/.test(s)) return false;

  // 忽略多行三元表达式片段
  if (/^\)\)\s*:\s*\(\s*$/.test(s)) return false;
  if (/^\)\s*:\s*\(\s*$/.test(s)) return false;
  if (/^\s*\)\s*:\s*\(/.test(s)) return false;

  // 忽略包含换行符的多行表达式
  if (s.includes('\n') && /^[)\d\s?':=&]+$/.test(s.replace(/\n/g, ''))) return false;

  // 忽略 emoji (通常是品牌或装饰性内容)
  if (/^[\u{1F300}-\u{1F9FF}]+$/u.test(s)) return false;

  // 如果包含空格，很可能是文本
  if (s.includes(' ')) return true;

  // 如果有非 ASCII 字符（如中文），肯定是文本
  if (/[^\x00-\x7F]/.test(s)) return true;

  // 短字符串（< 3）可能是代码片段
  if (s.length < 3) return false;

  return true;
}

function scanFile(filePath: string, content: string) {
  const findings: { type: string; text: string; line: number }[] = [];
  const clean = cleanContent(content);

  // 1. 扫描 JSX 文本: <div>Hardcoded</div>
  let match;
  while ((match = JSX_TEXT_REGEX.exec(clean)) !== null) {
    const text = match[1].trim();
    if (isSuspiciousString(text)) {
      const lineNo = clean.substring(0, match.index).split('\n').length;
      findings.push({ type: 'JSX Text', text, line: lineNo });
    }
  }

  // 2. 扫描特定属性
  while ((match = ATTRIBUTE_REGEX.exec(clean)) !== null) {
    const attr = match[1];
    const text = match[3];
    if (isSuspiciousString(text)) {
      const lineNo = clean.substring(0, match.index).split('\n').length;
      findings.push({ type: `Attribute [${attr}]`, text, line: lineNo });
    }
  }

  return findings;
}

async function main() {
  console.log('🔍 Scanning for hardcoded strings...\n');

  const allFindings: Record<string, ReturnType<typeof scanFile>[]> = {};

  for (const dir of config.scanDirs) {
    for await (const entry of walk(dir, {
      includeDirs: false,
      skip: config.excludeDirs.map(d => new RegExp(d)),
    })) {
      const ext = parse(entry.path).ext;
      if (config.extensions.includes(ext)) {
        try {
          const content = Deno.readTextFileSync(entry.path);
          const findings = scanFile(entry.path, content);
          if (findings.length > 0) {
            allFindings[entry.path] = findings;
          }
        } catch {
          // 忽略读取错误
        }
      }
    }
  }

  if (Object.keys(allFindings).length === 0) {
    console.log('✅ No hardcoded strings found!');
    return;
  }

  let totalCount = 0;
  console.log('=================================');
  console.log('📋 国际化硬编码检测报告');
  console.log('=================================\n');

  // 按文件问题数量排序
  const sortedEntries = Object.entries(allFindings).sort((a, b) => b[1].length - a[1].length);

  for (const [filePath, findings] of sortedEntries) {
    const relativePath = filePath.replace('./src/', 'src/');
    console.log(`📄 ${relativePath} (${findings.length} 个问题)`);
    console.log('─'.repeat(60));

    for (const f of findings) {
      totalCount++;
      console.log(`  [Line ${f.line}] ${f.type}: "${f.text}"`);
    }
    console.log('');
  }

  console.log('=================================');
  console.log(`📊 总结: 在 ${sortedEntries.length} 个文件中发现 ${totalCount} 个问题`);
  console.log('=================================\n');
}

main();
