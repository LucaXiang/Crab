#!/usr/bin/env -S deno run --allow-read --allow-write

import { walk } from "https://deno.land/std@0.208.0/fs/walk.ts";
import { parse } from "https://deno.land/std@0.208.0/path/mod.ts";

// key 映射: 旧 key → 新 key
const keyMapping: Record<string, string> = {
  "common.addStation": "settings.printer.addStation",
  "permissions.group.menu": "settings.permissions.group.menu",
  "permissions.group.store": "settings.permissions.group.store",
  "settings.categoryExists": "settings.category.message.exists",
  "settings.common.description": "settings.label.description",
  "settings.common.example": "settings.label.example",
  "settings.contentTemplate": "settings.label.contentTemplate",
  "settings.displayLabel": "settings.label.displayLabel",
  "settings.enableMultiSpec": "settings.specification.enableMulti",
  "settings.enableMultiSpecHint": "settings.specification.enableMultiHint",
  "settings.field.defaultSeparator": "settings.label.field.defaultSeparator",
  "settings.field.defaultText": "settings.label.field.defaultText",
  "settings.field.image": "settings.label.field.image",
  "settings.field.line": "settings.label.field.line",
  "settings.field.newImage": "settings.label.field.newImage",
  "settings.field.newTextField": "settings.label.field.newText",
  "settings.field.text": "settings.label.field.text",
  "settings.fieldHelperHint": "settings.label.fieldHelperHint",
  "settings.fieldKey": "settings.label.fieldKey",
  "settings.fieldProperties": "settings.label.fieldProperties",
  "settings.fillSample": "settings.label.fillSample",
  "settings.fitToScreen": "settings.label.fitToScreen",
  "settings.fontFamily": "settings.label.fontFamily",
  "settings.fontSize": "settings.label.fontSize",
  "settings.fontStyle": "settings.label.fontStyle",
  "settings.heightMm": "settings.label.heightMm",
  "settings.horizontalLine": "settings.label.horizontalLine",
  "settings.imageTemplateHint": "settings.label.imageTemplateHint",
  "settings.invalidJson": "settings.label.invalidJson",
  "settings.labelPlaceholder": "settings.label.placeholder",
  "settings.manageSpecifications": "settings.specification.manage",
  "settings.multiSpecDisabled": "settings.specification.multiDisabled",
  "settings.noLayers": "settings.label.noLayers",
  "settings.noProducts": "settings.product.noProducts",
  "settings.paddingX": "settings.label.paddingX",
  "settings.paddingY": "settings.label.paddingY",
  "settings.passwordMismatch": "settings.user.form.passwordMismatch",
  "settings.passwordRequired": "settings.user.form.passwordRequired",
  "settings.passwordTooShort": "settings.user.form.passwordTooShort",
  "settings.productCreated": "settings.product.message.created",
  "settings.productUpdated": "settings.product.message.updated",
  "settings.renderDpi": "settings.label.renderDpi",
  "settings.renderDpiHint": "settings.label.renderDpiHint",
  "settings.resetPasswordFailed": "settings.user.message.resetPasswordFailed",
  "settings.resetPasswordSuccess": "settings.user.message.resetPasswordSuccess",
  "settings.selectElementHint": "settings.label.selectElementHint",
  "settings.selectFieldHint": "settings.label.selectFieldHint",
  "settings.selectPrinterFirst": "settings.label.selectPrinterFirst",
  "settings.separatorHint": "settings.label.separatorHint",
  "settings.showOffsetBorder": "settings.label.showOffsetBorder",
  "settings.specificationCreated": "settings.specification.message.created",
  "settings.specificationDeleted": "settings.specification.message.deleted",
  "settings.specificationNameRequired": "settings.specification.form.nameRequired",
  "settings.specificationUpdated": "settings.specification.message.updated",
  "settings.styleBold": "settings.label.styleBold",
  "settings.styleRegular": "settings.label.styleRegular",
  "settings.table.zone.form.fixedPerItem": "settings.table.zone.form.surchargeFixed",
  "settings.table.zone.form.percentage": "settings.table.zone.form.surchargePercentage",
  "settings.tableCreated": "settings.table.message.created",
  "settings.tableUpdated": "settings.table.message.updated",
  "settings.templateName": "settings.label.templateName",
  "settings.testDataHint": "settings.label.testDataHint",
  "settings.testDataJson": "settings.label.testDataJson",
  "settings.testFailed": "settings.label.testFailed",
  "settings.testSent": "settings.label.testSent",
  "settings.textAlign": "settings.label.textAlign",
  "settings.user.action.createSuccess": "settings.user.message.createSuccess",
  "settings.user.action.updateSuccess": "settings.user.message.updateSuccess",
  "settings.verticalAlign": "settings.label.verticalAlign",
  "settings.widthMm": "settings.label.widthMm",
  "settings.xPosition": "settings.label.xPosition",
  "settings.yPosition": "settings.label.yPosition",
  "settings.zoneCreated": "settings.zone.message.created",
  "settings.zoneUpdated": "settings.zone.message.updated",
  "settings.zoomIn": "settings.label.zoomIn",
  "settings.zoomOut": "settings.label.zoomOut",
  "statistics.averageOrderValue": "statistics.metric.avgOrderValue",
  "statistics.avgDiningTime": "statistics.metric.avgDiningTime",
  "statistics.avgGuestSpend": "statistics.metric.avgGuestSpend",
  "statistics.cardRevenue": "statistics.metric.cardRevenue",
  "statistics.cashRevenue": "statistics.metric.cashRevenue",
  "statistics.categoryReport": "statistics.report.category",
  "statistics.detailedReportComingSoon": "statistics.report.detailedComingSoon",
  "statistics.otherRevenue": "statistics.metric.otherRevenue",
  "statistics.productReport": "statistics.report.product",
  "statistics.revenueTrend": "statistics.chart.revenueTrend",
  "statistics.salesByCategory": "statistics.chart.salesByCategory",
  "statistics.salesReport": "statistics.report.sales",
  "statistics.status.all": "common.status.all",
  "statistics.status.COMPLETED": "statistics.status.completed",
  "statistics.status.MERGED": "statistics.status.merged",
  "statistics.status.VOIDED": "statistics.status.voided",
  "statistics.totalDiscount": "statistics.metric.totalDiscount",
  "statistics.voidedOrders": "statistics.metric.voidedOrders",
  "table.prePayment": "table.filter.prePayment",
  "timeline.orderMerged": "timeline.event.orderMerged",
  "timeline.orderMergedOut": "timeline.event.orderMergedOut",
  "timeline.orderMoved": "timeline.event.orderMoved",
  "timeline.orderMovedOut": "timeline.event.orderMovedOut",
  "timeline.prePaymentNote": "timeline.event.prePaymentNote",
  "timeline.prePaymentPrinted": "timeline.event.prePaymentPrinted",
};

async function main() {
  console.log("🔄 更新源码中的 i18n keys...\n");

  let totalFiles = 0;
  let totalReplacements = 0;

  for await (const entry of walk("../src", {
    includeDirs: false,
    skip: [/node_modules/, /dist/, /\.git/, /src\/services\/i18n/],
  })) {
    const ext = parse(entry.path).ext;
    if (!['.tsx', '.ts', '.jsx', '.js'].includes(ext)) continue;

    let content = await Deno.readTextFile(entry.path);
    let replacements = 0;

    for (const [oldKey, newKey] of Object.entries(keyMapping)) {
      // 匹配 t("oldKey") 或 t('oldKey')
      const patterns = [
        new RegExp(`t\\(\\s*"${oldKey}"\\s*\\)`, 'g'),
        new RegExp(`t\\(\\s*'${oldKey}'\\s*\\)`, 'g'),
      ];

      for (const pattern of patterns) {
        const matches = content.match(pattern);
        if (matches) {
          replacements += matches.length;
          content = content.replace(pattern, `t("${newKey}")`);
        }
      }
    }

    if (replacements > 0) {
      await Deno.writeTextFile(entry.path, content);
      totalFiles++;
      totalReplacements += replacements;
      const relativePath = entry.path.replace('../src/', 'src/');
      console.log(`  📝 ${relativePath}: ${replacements} 处修改`);
    }
  }

  console.log("\n" + "=".repeat(60));
  console.log(`\n✅ 完成!`);
  console.log(`   修改文件: ${totalFiles} 个`);
  console.log(`   总替换: ${totalReplacements} 处\n`);
}

main();
