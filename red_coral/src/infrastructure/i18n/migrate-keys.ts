/**
 * i18n 键值迁移脚本
 * 用法: npx ts-node src/infrastructure/i18n/migrate-keys.ts [--dry-run]
 *
 * --dry-run: 只显示将要替换的内容，不实际修改文件
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// 旧键 → 新键 映射表
const KEY_MIGRATIONS: Record<string, string> = {
  // === 通用文本抽取到 common ===
  'common.saveSuccess': 'common.message.saveSuccess',
  'common.saveFailed': 'common.message.saveFailed',
  'common.goBack': 'common.action.back',
  'common.goHome': 'common.action.home',
  'common.ok': 'common.dialog.ok',
  'common.closeApp': 'common.dialog.closeApp',
  'common.na': 'common.label.none',
  'common.noResults': 'common.empty.noResults',
  'common.yes': 'common.dialog.yes',
  'common.no': 'common.dialog.no',
  'common.none': 'common.dialog.none',
  'common.confirmDelete': 'common.dialog.confirmDelete',
  'common.loading': 'common.message.loading',
  'common.success': 'common.message.success',
  'common.error': 'common.message.error',
  'common.submitting': 'common.message.submitting',
  'common.processing': 'common.message.processing',
  'common.invalidForm': 'common.message.invalidForm',
  'common.sentToPrinter': 'common.message.sentToPrinter',
  'common.exported': 'common.message.exported',
  'common.searchPlaceholder': 'common.hint.searchPlaceholder',
  'common.namePlaceholder': 'common.hint.namePlaceholder',
  'common.descriptionPlaceholder': 'common.hint.descriptionPlaceholder',
  'common.longPressHint': 'common.hint.longPressToEdit',
  'common.tapToClose': 'common.hint.tapToClose',
  'common.authorizationRequired': 'common.auth.required',
  'common.accessDenied': 'common.auth.denied',
  'common.accessDeniedMessage': 'common.auth.deniedMessage',
  'common.clickToAuthorize': 'common.auth.clickToAuthorize',
  'common.unknownItem': 'common.label.unknownItem',

  // === settings 重复消息移除 ===
  'settings.saveSuccess': 'common.message.saveSuccess',
  'settings.saveFailed': 'common.message.saveFailed',
  'settings.deleteFailed': 'common.message.deleteFailed',
  'settings.loadFailed': 'common.message.loadFailed',

  // === checkout.unknownItem 移到 common ===
  'checkout.unknownItem': 'common.label.unknownItem',

  // === 空状态统一到 common.empty ===
  'statistics.noData': 'common.empty.noData',
  'settings.product.list.noData': 'common.empty.noData',
  'settings.category.noData': 'common.empty.noData',
  'settings.attribute.noData': 'common.empty.noData',
  'settings.table.noData': 'common.empty.noData',
  'settings.user.noData': 'common.empty.noData',
  'settings.zone.noData': 'common.empty.noData',
  'settings.attribute.option.noData': 'common.empty.noData',

  // === 删除 error.codes 层级 ===
  'error.codes.AUTH_NOT_AUTHENTICATED': 'error.AUTH_NOT_AUTHENTICATED',
  'error.codes.AUTH_INVALID_CREDENTIALS': 'error.AUTH_INVALID_CREDENTIALS',
  'error.codes.AUTH_TOKEN_EXPIRED': 'error.AUTH_TOKEN_EXPIRED',
  'error.codes.AUTH_TOKEN_INVALID': 'error.AUTH_TOKEN_INVALID',
  'error.codes.AUTH_SESSION_EXPIRED': 'error.AUTH_SESSION_EXPIRED',
  'error.codes.AUTH_PERMISSION_DENIED': 'error.AUTH_PERMISSION_DENIED',
  'error.codes.AUTH_USER_DISABLED': 'error.AUTH_USER_DISABLED',
  'error.codes.AUTH_USER_NOT_FOUND': 'error.AUTH_USER_NOT_FOUND',
  'error.codes.BRIDGE_NOT_INITIALIZED': 'error.BRIDGE_NOT_INITIALIZED',
  'error.codes.BRIDGE_NOT_CONNECTED': 'error.BRIDGE_NOT_CONNECTED',
  'error.codes.BRIDGE_ALREADY_RUNNING': 'error.BRIDGE_ALREADY_RUNNING',
  'error.codes.BRIDGE_CONNECTION_FAILED': 'error.BRIDGE_CONNECTION_FAILED',
  'error.codes.BRIDGE_CONNECTION_LOST': 'error.BRIDGE_CONNECTION_LOST',
  'error.codes.BRIDGE_TIMEOUT': 'error.BRIDGE_TIMEOUT',
  'error.codes.TENANT_NOT_SELECTED': 'error.TENANT_NOT_SELECTED',
  'error.codes.TENANT_NOT_FOUND': 'error.TENANT_NOT_FOUND',
  'error.codes.TENANT_ACTIVATION_REQUIRED': 'error.TENANT_ACTIVATION_REQUIRED',
  'error.codes.TENANT_ACTIVATION_FAILED': 'error.TENANT_ACTIVATION_FAILED',
  'error.codes.TENANT_CERTIFICATE_INVALID': 'error.TENANT_CERTIFICATE_INVALID',
  'error.codes.TENANT_CERTIFICATE_EXPIRED': 'error.TENANT_CERTIFICATE_EXPIRED',
  'error.codes.TENANT_SUBSCRIPTION_EXPIRED': 'error.TENANT_SUBSCRIPTION_EXPIRED',
  'error.codes.TENANT_SUBSCRIPTION_INVALID': 'error.TENANT_SUBSCRIPTION_INVALID',
  'error.codes.SERVER_START_FAILED': 'error.SERVER_START_FAILED',
  'error.codes.SERVER_NOT_RUNNING': 'error.SERVER_NOT_RUNNING',
  'error.codes.SERVER_INTERNAL_ERROR': 'error.SERVER_INTERNAL_ERROR',
  'error.codes.SERVER_UNAVAILABLE': 'error.SERVER_UNAVAILABLE',
  'error.codes.SERVER_DATABASE_ERROR': 'error.SERVER_DATABASE_ERROR',
  'error.codes.UNKNOWN_ERROR': 'error.UNKNOWN_ERROR',
  'error.codes.NETWORK_ERROR': 'error.NETWORK_ERROR',
  'error.codes.PARSE_ERROR': 'error.PARSE_ERROR',

  // === 删除 table.common 层级 ===
  'table.common.zones': 'table.zones',
  'table.common.guests': 'table.guests',

  // === 删除 timeline.event 层级 ===
  'timeline.event.addItems': 'timeline.addItems',
  'timeline.event.empty': 'timeline.empty',
  'timeline.event.payment': 'timeline.payment',
  'timeline.event.splitBill': 'timeline.splitBill',
  'timeline.event.itemModified': 'timeline.itemModified',
  'timeline.event.itemRemoved': 'timeline.itemRemoved',
  'timeline.event.orderCompleted': 'timeline.orderCompleted',
  'timeline.event.orderVoided': 'timeline.orderVoided',
  'timeline.event.tableOrder': 'timeline.tableOrder',
  'timeline.event.itemRestored': 'timeline.itemRestored',
  'timeline.event.paymentCancelled': 'timeline.paymentCancelled',
  'timeline.event.orderRestored': 'timeline.orderRestored',
  'timeline.event.orderMerged': 'timeline.orderMerged',
  'timeline.event.orderMoved': 'timeline.orderMoved',
  'timeline.event.orderMovedOut': 'timeline.orderMovedOut',
  'timeline.event.orderMergedOut': 'timeline.orderMergedOut',
  'timeline.event.prePaymentPrinted': 'timeline.prePaymentPrinted',
  'timeline.event.prePaymentNote': 'timeline.prePaymentNote',

  // === settings 操作按钮重命名 ===
  'settings.zone.action.add': 'settings.zone.addZone',
  'settings.zone.action.edit': 'settings.zone.editZone',
  'settings.zone.action.delete': 'settings.zone.deleteZone',
  'settings.zone.action.deleted': 'settings.zone.zoneDeleted',
  'settings.product.action.add': 'settings.product.addProduct',
  'settings.product.action.edit': 'settings.product.editProduct',
  'settings.product.action.delete': 'settings.product.deleteProduct',
  'settings.product.action.deleted': 'settings.product.productDeleted',
  'settings.product.action.deleteFailed': 'settings.product.deleteProductFailed',
  'settings.category.action.add': 'settings.category.addCategory',
  'settings.category.action.edit': 'settings.category.editCategory',
  'settings.category.action.delete': 'settings.category.deleteCategory',
  'settings.category.action.adjustOrder': 'settings.category.adjustCategoryOrder',
  'settings.category.action.deleted': 'settings.category.categoryDeleted',
  'settings.category.action.createSuccess': 'settings.category.createCategorySuccess',
  'settings.category.action.updateSuccess': 'settings.category.updateCategorySuccess',
  'settings.attribute.action.add': 'settings.attribute.addAttribute',
  'settings.attribute.action.edit': 'settings.attribute.editAttribute',
  'settings.attribute.action.delete': 'settings.attribute.deleteAttribute',
  'settings.attribute.option.action.add': 'settings.attribute.option.addOption',
  'settings.attribute.option.action.edit': 'settings.attribute.option.editOption',
  'settings.attribute.option.delete': 'settings.attribute.option.deleteOption',
  'settings.table.action.add': 'settings.table.addTable',
  'settings.table.action.edit': 'settings.table.editTable',
  'settings.table.action.delete': 'settings.table.deleteTable',
  'settings.table.action.deleted': 'settings.table.tableDeleted',
  'settings.user.action.add': 'settings.user.addUser',
  'settings.user.action.edit': 'settings.user.editUser',
  'settings.user.action.resetPassword': 'settings.user.resetPasswordUser',
  'settings.user.action.disable': 'settings.user.disableUser',
  'settings.user.action.deletePermanently': 'settings.user.deletePermanentlyUser',
  'settings.printer.kitchenStation.action.add': 'settings.printer.kitchenStation.addStation',
  'settings.printer.kitchenStation.action.edit': 'settings.printer.kitchenStation.editStation',
  'settings.printer.template.action.create': 'settings.printer.template.createTemplate',
  'settings.printer.template.action.editDesign': 'settings.printer.template.editDesign',
  'settings.printer.template.action.duplicate': 'settings.printer.template.duplicateTemplate',
  'settings.printer.addStation': 'settings.printer.kitchenStation.addStation',

  // === common.status 映射 ===
  'common.active': 'common.status.active',
  'common.inactive': 'common.status.inactive',
  'common.void': 'common.status.void',
  'common.enabled': 'common.status.enabled',
  'common.disabled': 'common.status.disabled',
  'common.disabledGlobal': 'common.status.disabledGlobal',

  // === common.action 映射 ===
  'common.back': 'common.action.back',
  'common.cancel': 'common.action.cancel',
  'common.save': 'common.action.save',
  'common.delete': 'common.action.delete',
  'common.edit': 'common.action.edit',
  'common.create': 'common.action.create',
  'common.confirm': 'common.action.confirm',
  'common.close': 'common.action.close',
  'common.clear': 'common.action.clear',
  'common.search': 'common.action.search',
  'common.filter': 'common.action.filter',
  'common.refresh': 'common.action.refresh',
  'common.retry': 'common.action.retry',
  'common.export': 'common.action.export',
  'common.print': 'common.action.print',
  'common.select': 'common.action.select',
  'common.remove': 'common.action.remove',
  'common.enable': 'common.action.enable',
  'common.hide': 'common.action.hide',
  'common.change': 'common.action.change',
  'common.uploadImage': 'common.action.uploadImage',
  'common.batchDelete': 'common.action.batchDelete',
  'common.collapseAll': 'common.action.collapseAll',
  'common.expandAll': 'common.action.expandAll',

  // === common.label 映射 ===
  'common.name': 'common.label.name',
  'common.description': 'common.label.description',
  'common.details': 'common.label.details',
  'common.date': 'common.label.date',
  'common.quantity': 'common.label.quantity',
  'common.total': 'common.label.total',
  'common.items': 'common.label.items',
  'common.page': 'common.label.page',
  'common.of': 'common.label.of',
  'common.to': 'common.label.to',
  'common.entries': 'common.label.entries',
  'common.showing': 'common.selection.showing',
  'common.default': 'common.label.default',
  'common.required': 'common.label.required',
  'common.optional': 'common.label.optional',
  'common.paidQuantity': 'common.label.paidQuantity',

  // === common.selection 映射 ===
  'common.selectMode': 'common.selection.selectMode',
  'common.selected': 'common.selection.selected',
  'common.all': 'common.status.all',

  // === error 操作映射 ===
  'error.autoReload': 'error.action.autoReload',
  'error.copyDetails': 'error.action.copyDetails',
  'error.viewDetails': 'error.action.viewDetails',
};

// 要扫描的目录
const SCAN_DIRS = [
  'src/screens',
  'src/presentation',
  'src/core',
  'src/hooks',
  'src/utils',
];

// 文件扩展名
const FILE_EXTENSIONS = ['.ts', '.tsx'];

interface Replacement {
  file: string;
  line: number;
  oldKey: string;
  newKey: string;
  context: string;
}

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

function findReplacements(filePath: string): Replacement[] {
  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const replacements: Replacement[] = [];

  // 匹配 t('key') 或 t("key") 模式
  const pattern = /t\(['"]([^'"]+)['"]\)/g;

  lines.forEach((line, index) => {
    let match;
    while ((match = pattern.exec(line)) !== null) {
      const oldKey = match[1];
      const newKey = KEY_MIGRATIONS[oldKey];

      if (newKey && newKey !== oldKey) {
        replacements.push({
          file: filePath,
          line: index + 1,
          oldKey,
          newKey,
          context: line.trim().substring(0, 80),
        });
      }
    }
  });

  return replacements;
}

function applyReplacements(filePath: string): number {
  let content = fs.readFileSync(filePath, 'utf-8');
  let count = 0;

  for (const [oldKey, newKey] of Object.entries(KEY_MIGRATIONS)) {
    if (oldKey === newKey) continue;

    // 替换 t('oldKey') 和 t("oldKey")
    const patterns = [
      new RegExp(`t\\('${escapeRegex(oldKey)}'\\)`, 'g'),
      new RegExp(`t\\("${escapeRegex(oldKey)}"\\)`, 'g'),
    ];

    for (const pattern of patterns) {
      const matches = content.match(pattern);
      if (matches) {
        count += matches.length;
        content = content.replace(pattern, `t('${newKey}')`);
      }
    }
  }

  if (count > 0) {
    fs.writeFileSync(filePath, content, 'utf-8');
  }

  return count;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function main() {
  const isDryRun = process.argv.includes('--dry-run');
  const baseDir = path.resolve(__dirname, '../../..');

  console.log(`\n🔍 扫描目录: ${baseDir}`);
  console.log(`📝 模式: ${isDryRun ? '预览 (--dry-run)' : '执行替换'}\n`);

  // 收集所有文件
  const allFiles: string[] = [];
  for (const dir of SCAN_DIRS) {
    allFiles.push(...getAllFiles(dir, baseDir));
  }

  console.log(`📂 共找到 ${allFiles.length} 个文件\n`);

  if (isDryRun) {
    // 预览模式：显示将要替换的内容
    const allReplacements: Replacement[] = [];

    for (const file of allFiles) {
      const replacements = findReplacements(file);
      allReplacements.push(...replacements);
    }

    if (allReplacements.length === 0) {
      console.log('✅ 没有找到需要替换的键值\n');
      return;
    }

    console.log(`🔄 找到 ${allReplacements.length} 处需要替换:\n`);

    // 按文件分组显示
    const byFile = new Map<string, Replacement[]>();
    for (const r of allReplacements) {
      const list = byFile.get(r.file) || [];
      list.push(r);
      byFile.set(r.file, list);
    }

    for (const [file, replacements] of byFile) {
      const relativePath = path.relative(baseDir, file);
      console.log(`📄 ${relativePath}`);
      for (const r of replacements) {
        console.log(`   L${r.line}: "${r.oldKey}" → "${r.newKey}"`);
      }
      console.log('');
    }

    console.log('💡 运行不带 --dry-run 参数以执行替换\n');
  } else {
    // 执行模式：实际替换文件
    let totalCount = 0;
    let filesModified = 0;

    for (const file of allFiles) {
      const count = applyReplacements(file);
      if (count > 0) {
        const relativePath = path.relative(baseDir, file);
        console.log(`✅ ${relativePath}: ${count} 处替换`);
        totalCount += count;
        filesModified++;
      }
    }

    console.log(`\n🎉 完成! 共修改 ${filesModified} 个文件，替换 ${totalCount} 处键值\n`);
  }
}

main().catch(console.error);
