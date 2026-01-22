# i18n 键名 snake_case 迁移计划

## 概述

将所有 i18n 翻译键从 camelCase 统一迁移到 snake_case，并补全缺失的翻译键。

**决策：**
- 命名空间结构：嵌套结构（zh-CN 风格）
- 命名风格：全部 snake_case
- 语言支持：现阶段只支持中文，禁用 en-US

## 迁移范围

| 任务 | 数量 |
|------|------|
| camelCase → snake_case | 445 键 |
| 补全缺失的翻译键 | ~25 键 |
| 更新代码中 `t('...')` 调用 | ~50 文件 |

## 键名映射规则

```
camelCase → snake_case

expandAll → expand_all
batchDelete → batch_delete
uploadImage → upload_image
saveSuccess → save_success
avgDiningTime → avg_dining_time
isMultiSelect → is_multi_select
```

完整映射表：`src/infrastructure/i18n/key-migration-map.json`

## 执行步骤

### 步骤 1：更新 zh-CN.json

将所有 camelCase 键改为 snake_case。主要涉及：

- `common` 模块 (24 键)
- `app` 模块 (5 键)
- `auth` 模块 (11 键)
- `pos` 模块 (18 键)
- `checkout` 模块 (18 键)
- `table` 模块 (12 键)
- `history` 模块 (9 键)
- `statistics` 模块 (13 键)
- `timeline` 模块 (17 键)
- `error` 模块 (4 键)
- `settings` 模块 (310 键)
- `fonts` 模块 (4 键)

### 步骤 2：补全缺失的翻译键

**settings.attribute 模块：**
```json
"settings.attribute.form.is_multi_select": "多选",
"settings.attribute.form.is_multi_select_desc": "允许选择多个选项",
"settings.attribute.form.kitchen_print_name": "厨房打印名称",
"settings.attribute.form.kitchen_print_name_placeholder": "如为空则使用属性名称",
"settings.attribute.form.scope": "作用范围",
"settings.attribute.form.show_on_kitchen_print": "显示在厨房打印",
"settings.attribute.form.show_on_kitchen_print_hint": "选择此项后，该属性的选项将显示在厨房小票上",
"settings.attribute.scope.global": "全局",
"settings.attribute.scope.inherited": "继承自分类",
"settings.attribute.type.single_select": "单选",
"settings.attribute.type.multi_select": "多选",
"settings.attribute.option.form.kitchen_print_name": "厨房打印名称",
"settings.attribute.option.form.kitchen_print_name_placeholder": "如为空则使用选项名称"
```

**timeline 模块：**
```json
"timeline.from": "从",
"timeline.to": "至",
"timeline.table_moved": "桌台转移",
"timeline.table_reassigned": "桌台重新分配",
"timeline.order_info_updated": "订单信息更新",
"timeline.merged_out": "合并转出",
"timeline.moved_out": "转移转出",
"timeline.labels.guests": "客人",
"timeline.labels.receipt": "小票",
"timeline.labels.table": "桌台"
```

**statistics 模块：**
```json
"statistics.metric.avg_dining_time": "平均用餐时间",
"statistics.metric.avg_guest_spend": "人均消费",
"statistics.metric.card_revenue": "银行卡收入",
"statistics.metric.cash_revenue": "现金收入",
"statistics.metric.other_revenue": "其他收入",
"statistics.metric.total_discount": "总折扣",
"statistics.metric.voided_orders": "作废订单数"
```

**history 模块：**
```json
"history.info.order_id": "订单号"
```

### 步骤 3：更新代码中的 t() 调用

使用脚本批量替换代码中的翻译键调用：

```bash
# 示例：替换 expandAll → expand_all
find src -name "*.tsx" -o -name "*.ts" | xargs sed -i '' "s/t('common.action.expandAll')/t('common.action.expand_all')/g"
```

需要特别注意动态键：
```typescript
// 动态键模式（需要同步更新使用处）
t(`statistics.metric.${key}`)  // key 变量值也需要改为 snake_case
t(`statistics.status.${item.status}`)
```

### 步骤 4：禁用 en-US.json

重命名文件以禁用：
```bash
mv src/infrastructure/i18n/locales/en-US.json src/infrastructure/i18n/locales/en-US.json.disabled
```

更新 i18n 配置，移除英文支持。

## 验证清单

- [ ] 运行 `npx tsx src/infrastructure/i18n/scan-keys.ts` 确认无缺失键
- [ ] 运行 `npm run build` 确认无编译错误
- [ ] 手动测试主要页面 UI 显示正常

## 回滚方案

如果出现问题，可通过 git 回滚：
```bash
git checkout -- src/infrastructure/i18n/locales/zh-CN.json
git checkout -- src/**/*.tsx src/**/*.ts
```
