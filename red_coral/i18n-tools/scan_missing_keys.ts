#!/usr/bin/env -S deno run --allow-read

import { walk } from "https://deno.land/std@0.208.0/fs/walk.ts";
import { parse } from "https://deno.land/std@0.208.0/path/mod.ts";

interface MissingKey {
  originalKey: string;
  suggestedKey: string;
  translation: string;
  file: string;
  line: number;
}

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

function scanFile(content: string): Map<string, number> {
  const keys = new Map<string, number>();
  const pattern = /\bt\(\s*["']([^"'\n]+)["']\s*\)/g;
  let match;
  while ((match = pattern.exec(content)) !== null) {
    const key = match[1].trim();
    if (key && !key.includes('${')) {
      const lineNo = content.substring(0, match.index).split('\n').length;
      if (!keys.has(key)) {
        keys.set(key, lineNo);
      }
    }
  }
  return keys;
}

function getExistingPaths(obj: any, prefix: string = '', paths = new Set<string>()): Set<string> {
  for (const [key, value] of Object.entries(obj)) {
    const newPath = prefix ? `${prefix}.${key}` : key;
    paths.add(newPath);
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      getExistingPaths(value, newPath, paths);
    }
  }
  return paths;
}

function suggestTranslation(key: string, localeKeys: Record<string, string>): string {
  const parts = key.split('.');
  const lastPart = parts[parts.length - 1];
  const existingPaths = getExistingPaths(JSON.parse(Deno.readTextFileSync("../src/services/i18n/locales/zh-CN.json")));

  // 根据最后一部分推断中文含义
  const translations: Record<string, string> = {
    title: "标题",
    subtitle: "副标题",
    description: "描述",
    hint: "提示",
    placeholder: "占位符",
    success: "成功",
    failed: "失败",
    required: "必填",
    add: "添加",
    edit: "编辑",
    delete: "删除",
    save: "保存",
    cancel: "取消",
    confirm: "确认",
    name: "名称",
    noData: "暂无数据",
    noResults: "无结果",
    loading: "加载中...",
    error: "错误",
    warning: "警告",
    action: "操作",
    actions: "操作",
    form: "表单",
    empty: "为空",
    all: "全部",
    enabled: "已启用",
    disabled: "已禁用",
    active: "激活",
    inactive: "停用",
    select: "选择",
    selected: "已选",
    unit: "个",
    created: "已创建",
    updated: "已更新",
    deleted: "已删除",
    blocked: "被阻止",
    exists: "已存在",
    invalid: "无效",
    password: "密码",
    username: "用户名",
    role: "角色",
    page: "页",
    entries: "条",
    date: "日期",
    export: "导出",
    exported: "已导出",
    retry: "重试",
    to: "至",
    of: "/",
    print: "打印",
    refresh: "刷新",
    back: "返回",
    create: "创建",
    order: "订单",
    item: "项目",
    items: "项目",
    unpaid: "未付",
    zone: "区域",
    table: "桌台",
    category: "分类",
    product: "商品",
    attribute: "属性",
    specification: "规格",
    printer: "打印机",
    station: "工位",
    template: "模板",
    label: "标签",
    report: "报告",
    revenue: "收入",
    sales: "销售",
    cash: "现金",
    card: "银行卡",
    discount: "折扣",
    total: "合计",
    average: "平均",
    guest: "客人",
    time: "时间",
    value: "值",
    status: "状态",
    type: "类型",
    level: "级别",
    global: "全局",
    system: "系统",
    store: "店铺",
    menu: "菜单",
    other: "其他",
    settings: "设置",
    permissions: "权限",
    login: "登录",
    logout: "登出",
    user: "用户",
    currentUser: "当前用户",
    message: "消息",
    contact: "联系方式",
    approval: "审批",
    reason: "原因",
    void: "作废",
    split: "拆分",
    merge: "合并",
    move: "转移",
    restore: "恢复",
    orderId: "订单号",
    amount: "金额",
    processing: "处理中",
    unknown: "未知",
    tapToClose: "点击关闭",
    invalidCredentials: "凭证无效",
    emptyFields: "字段为空",
    multiZone: "多区域",
    fastCheckout: "快速结账",
    feature: "功能",
    enterDetails: "输入详情",
    supervisor: "主管",
    unauthorized: "未授权",
    forbidden: "禁止",
    runAway: "跑单",
    dineAndDash: "跑单",
    systemTest: "系统测试",
    ownerTreat: "老板请客",
    surcharge: "附加费",
    exempt: "免除",
    moved: "已转移",
    merged: "已合并",
    printed: "已打印",
    note: "备注",
    prePayment: "预付",
    horizontalLine: "水平线",
    layers: "图层",
    width: "宽度",
    height: "高度",
    padding: "边距",
    render: "渲染",
    dpi: "DPI",
    test: "测试",
    data: "数据",
    json: "JSON",
    sample: "示例",
    fill: "填充",
    zoom: "缩放",
    fit: "适应",
    screen: "屏幕",
    reorder: "重新排序",
    drag: "拖拽",
    dragToReorder: "拖拽排序",
    multiSpec: "多规格",
    manage: "管理",
    new: "新建",
    field: "字段",
    element: "元素",
    select: "选择",
    selectFirst: "请先选择",
    keyboard: "键盘",
    close: "关闭",
    untitled: "未命名",
    properties: "属性",
    example: "示例",
    key: "键",
    default: "默认",
    text: "文本",
    image: "图片",
    line: "线条",
    separator: "分隔线",
    offset: "偏移",
    border: "边框",
    productOrder: "商品排序",
    noProducts: "暂无商品",
    basicInfo: "基本信息",
    extendedInfo: "扩展信息",
    extended: "扩展",
    form: {
      name: "名称",
      namePlaceholder: "请输入名称",
      surchargeType: "附加费类型",
      fixedPerItem: "每项固定",
      percentage: "百分比",
      surchargeAmount: "附加费金额",
    }
  };

  return translations[lastPart] || lastPart;
}

function suggestKey(key: string, localeKeys: Record<string, string>): string {
  const parts = key.split('.');

  // 尝试根据现有结构建议更合适的键名
  const existingPaths = Object.keys(localeKeys);

  // 检查是否有相似的路径
  for (const path of existingPaths) {
    const pathParts = path.split('.');
    if (pathParts.length >= 2 && parts[0] === pathParts[0]) {
      // 相同父级，保持原键名
      return key;
    }
  }

  return key;
}

async function main() {
  const localesDir = Deno.args[0] || "../src/services/i18n/locales";
  const localeFile = Deno.args[1] || "zh-CN.json";

  console.log("🔍 扫描缺失的翻译 keys...\n");

  const localePath = `${localesDir}/${localeFile}`;
  let localeKeys: Record<string, string>;

  try {
    const content = await Deno.readTextFile(localePath);
    const data = JSON.parse(content);
    localeKeys = flattenObject(data);
    console.log(`📖 读取 ${localeFile}: ${Object.keys(localeKeys).length} 个 keys\n`);
  } catch (error) {
    console.error(`❌ 读取文件失败: ${localePath}`);
    Deno.exit(1);
  }

  const missingKeys = new Map<string, MissingKey>();

  for await (const entry of walk("../src", {
    includeDirs: false,
    skip: [/node_modules/, /dist/, /\.git/, /src\/services\/i18n/],
  })) {
    const ext = parse(entry.path).ext;
    if (!['.tsx', '.ts', '.jsx', '.js'].includes(ext)) continue;

    try {
      const content = await Deno.readTextFile(entry.path);
      const keys = scanFile(content);

      for (const [origKey, lineNo] of keys) {
        if (!(origKey in localeKeys) && !missingKeys.has(origKey)) {
          const translation = suggestTranslation(origKey, localeKeys);
          const suggestedKey = suggestKey(origKey, localeKeys);

          missingKeys.set(origKey, {
            originalKey: origKey,
            suggestedKey,
            translation,
            file: entry.path.replace("./src/", "src/"),
            line: lineNo
          });
        }
      }
    } catch {
      // 忽略读取错误
    }
  }

  console.log("=".repeat(60));
  console.log(`\n⚠️  缺失翻译的 keys: ${missingKeys.size} 个\n`);

  if (missingKeys.size > 0) {
    // 按文件分组输出
    const byFile = new Map<string, MissingKey[]>();
    for (const item of missingKeys.values()) {
      const existing = byFile.get(item.file) || [];
      existing.push(item);
      byFile.set(item.file, existing);
    }

    for (const [file, items] of byFile) {
      console.log(`📄 ${file}`);
      for (const item of items) {
        const keyDisplay = item.suggestedKey !== item.originalKey
          ? `${item.originalKey} → ${item.suggestedKey}`
          : item.originalKey;
        console.log(`  [${item.line}] ${keyDisplay} = "${item.translation}"`);
      }
      console.log("");
    }

    // 生成 JSON 输出（方便程序处理）
    console.log("=".repeat(60));
    console.log("\n📋 JSON 格式（可复制到 locale 文件）:\n");

    const jsonOutput: Record<string, string> = {};
    for (const item of missingKeys.values()) {
      jsonOutput[item.suggestedKey] = item.translation;
    }
    console.log(JSON.stringify(jsonOutput, null, 2));
  } else {
    console.log("✅ 所有 keys 都有对应的翻译!");
  }

  console.log("\n" + "=".repeat(60));
}

main();
