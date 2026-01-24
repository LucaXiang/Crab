# 前端代码清理计划

## 目标

清理开发阶段积累的冗余代码、重复 Store、混乱的职责分离。

## 阶段 1: 删除死代码 (P0)

### 1.1 删除 useAnimations.ts
- 文件: `src/core/stores/ui/useAnimations.ts`
- 原因: 被 `useUIStore.ts` 的 `useAnimations` selector 覆盖，完全未使用
- 依赖检查: `index.ts` 导出的是 `useUIStore` 的版本

### 1.2 删除 useLabelPrinter.ts
- 文件: `src/core/stores/ui/useLabelPrinter.ts`
- 原因: 与 `useUIStore` 的 `labelPrinter` 功能重复
- 需要修复的引用:
  - `src/screens/Settings/components/LabelEditorScreen.tsx`
  - `src/screens/Settings/components/printer/HardwareSettings.tsx`

### 1.3 合并 formValidator.ts
- 文件: `src/core/validation/formValidator.ts`
- 目标: 合并到 `src/core/domain/validators.ts`
- 然后删除 `src/core/validation/` 目录

## 阶段 2: 拆分 useUIStore (P1)

### 2.1 创建 PrinterStore
提取打印机相关状态:
- receiptPrinter / kitchenPrinter / labelPrinter
- isKitchenPrintEnabled / isLabelPrintEnabled
- activeLabelTemplateId

### 2.2 简化 useUIStore
仅保留:
- screen / viewMode (路由)
- showDebugMenu / showTableScreen / showDraftModal (模态框)
- animations (动画队列)
- selectedCategory / searchQuery (POS 过滤)

## 阶段 3: 整理 hooks 目录 (P1)

### 3.1 明确分层
- `src/core/hooks/` - 跨屏幕基础能力
- 屏幕级 hooks 移入各 `screens/*/hooks/`

### 3.2 待移动的 hooks
- useOrderHandlers → screens/Checkout/hooks/
- useHistoryOrderList → screens/History/hooks/
- useDraftHandlers → screens/POS/hooks/

## 阶段 4: 拆分大文件 (P2)

### 4.1 拆分 models.ts (803行)
按资源类型拆分到 `types/api/resources/`

### 4.2 拆分 orderEvent.ts (690行)
拆分到 `types/orderEvent/` 目录

## 执行顺序

1. [x] 阶段 1.1 - 删除 useAnimations.ts
2. [ ] 阶段 1.2 - 删除 useLabelPrinter.ts
3. [ ] 阶段 1.3 - 合并 formValidator.ts
4. [ ] 阶段 2 - 拆分 useUIStore
5. [ ] 阶段 3 - 整理 hooks
6. [ ] 阶段 4 - 拆分大文件
