# Red Coral 设计系统

本文档定义 Red Coral POS 应用的设计规范,确保 UI 组件的一致性和可维护性。

---

## 🎨 Modal 弹窗组件规范

### 外观结构

#### 圆角系统
- **外框圆角**: 统一使用 `rounded-2xl` (16px)
- **内部卡片**: `rounded-xl` (12px)
- **小组件**: `rounded-lg` (8px)
- **按钮/输入框**: `rounded-xl` (12px)
- **标签/徽章**: `rounded-full`

#### 阴影层级
- **Modal 外框**: `shadow-2xl`
- **卡片组件**: `shadow-sm` ~ `shadow-lg`
- **按钮强调**: `shadow-lg shadow-{color}-600/20`

#### 遮罩 (Overlay)
- **标准遮罩**: `bg-black/50 backdrop-blur-sm`
  - 适用场景: 普通确认框、信息展示、表单编辑
- **强调遮罩**: `bg-black/60 backdrop-blur-sm`
  - 适用场景: 支付流程、危险操作 (作废/删除)、需要用户高度集中的操作

#### 最大尺寸
- **小 Modal**: `max-w-sm` (384px) - 简单确认
- **中 Modal**: `max-w-md` (448px) - 单字段表单
- **标准 Modal**: `max-w-2xl` (672px) - 多字段表单 (CRUD)
- **大 Modal**: `max-w-4xl` (896px) - 双栏配置器
- **超大 Modal**: `max-w-[95vw] h-[92vh]` - 全屏级 (快速添加)
- **高度限制**: `max-h-[90vh]` (留出状态栏空间)

---

### 布局规范

#### 三段式布局 (标准)
```tsx
<div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh]">
  {/* Header - 固定高度 */}
  <div className="shrink-0 px-6 py-4 border-b border-gray-100">
    {/* 标题 + 关闭按钮 */}
  </div>

  {/* Content - 可滚动 */}
  <div className="flex-1 overflow-y-auto p-6">
    {/* 表单内容 */}
  </div>

  {/* Footer - 固定高度 */}
  <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50">
    {/* 操作按钮 */}
  </div>
</div>
```

#### Header (头部)
- **间距**: 统一 `px-6 py-4`
- **分隔线**: `border-b border-gray-100`
- **背景色规则**:
  - **默认**: `bg-white`
  - **危险操作** (作废/删除): `bg-red-50`
  - **警告操作** (折扣/附加费): `bg-orange-50`
  - **权限敏感** (主管授权): `bg-teal-50`
  - **商品/资源编辑**: `bg-primary-50` (可选)

#### Content (内容区)
- **间距**: 统一 `p-6`
- **背景**: `bg-white` (默认) 或 `bg-gray-50/50` (配置器左侧)
- **滚动**: `overflow-y-auto` + `custom-scrollbar`

#### Footer (底部)
- **间距**: 统一 `px-6 py-4`
- **分隔线**: `border-t border-gray-100`
- **背景**: `bg-gray-50` (不使用透明度变体如 `/50`)
- **按钮布局**: `flex justify-end gap-3` (右对齐,间距 12px)

---

### 交互元素

#### 关闭按钮 (标准样式)
```tsx
<button
  onClick={onClose}
  className="p-2 hover:bg-gray-100 rounded-full transition-colors"
>
  <X size={20} className="text-gray-500" />
</button>
```
- **图标大小**: `size={20}` (Lucide React)
- **形状**: `rounded-full` (圆形)
- **位置**: Header 内右对齐
- **Hover**: `hover:bg-gray-100`

#### 关闭按钮 (特殊场景)
**绝对定位悬浮** (无 Header 的全屏 Modal):
```tsx
<button
  onClick={onClose}
  className="absolute top-4 right-4 z-10 p-2 bg-white/80 backdrop-blur-sm border border-gray-200 rounded-full shadow-sm hover:bg-white transition-colors"
>
  <X size={20} className="text-gray-500" />
</button>
```

#### 按钮样式矩阵

| 类型 | 背景色 | 文字色 | Hover | Shadow | 用途 |
|------|-------|--------|-------|--------|------|
| **主按钮** | `bg-primary-500` | `text-white` | `hover:bg-primary-600` | `shadow-lg shadow-primary-500/20` | 确认/保存 |
| **成功按钮** | `bg-green-600` | `text-white` | `hover:bg-green-700` | `shadow-lg shadow-green-600/20` | 支付/完成 |
| **危险按钮** | `bg-red-600` | `text-white` | `hover:bg-red-700` | `shadow-lg shadow-red-600/20` | 删除/作废 |
| **警告按钮** | `bg-orange-500` | `text-white` | `hover:bg-orange-600` | `shadow-lg shadow-orange-500/20` | 折扣/附加费 |
| **次级按钮** | `bg-gray-100` | `text-gray-700` | `hover:bg-gray-200` | - | 取消 |
| **禁用状态** | `bg-gray-300` | `text-gray-400` | - | - | `disabled` + `cursor-not-allowed` |

#### 按钮交互动画
- **点击缩放**: `active:scale-95 transform`
- **悬浮抬升** (可选): `hover:-translate-y-0.5 transition-all`

---

### 动画规范

#### Modal 进入动画 (标准)
```tsx
{/* 遮罩层 */}
<div className="... animate-in fade-in duration-200">
  {/* Modal 内容 */}
  <div className="... animate-in zoom-in-95 duration-200">
    ...
  </div>
</div>
```
- **遮罩**: `animate-in fade-in duration-200` (淡入)
- **内容**: `animate-in zoom-in-95 duration-200` (从 95% 缩放到 100%)

#### Modal 退出动画 (可选)
目前未实现统一退出动画,组件通过 `if (!isOpen) return null` 直接卸载。

**未来优化**:
```tsx
<div className={isOpen ? "animate-in fade-in" : "animate-out fade-out"}>
```

#### 按钮交互
- **标准**: `transition-colors` (颜色过渡)
- **增强**: `transition-all` (全属性过渡,用于位移/缩放)
- **时长**: 默认 150ms (Tailwind 默认)

---

### z-index 层级系统

#### 层级常量 (推荐在代码中定义)
```ts
// src/shared/constants/zIndex.ts
export const Z_INDEX = {
  MODAL_BASE: 50,          // 普通 Modal (订单详情)
  MODAL_BUSINESS: 60,      // 业务 Modal (POS/支付/快速添加)
  MODAL_MANAGEMENT: 80,    // 管理 Modal (Settings CRUD)
  MODAL_NESTED: 90,        // 嵌套确认弹窗 (未保存提示)
  MODAL_CONFIGURATOR: 100, // 特殊配置器 (商品属性)
  MODAL_AUTH: 9999,        // 权限升级 (主管授权)
  TOAST: 10000,            // Toast 通知
} as const;
```

#### 层级使用规则
- **z-50**: 普通信息展示 (OrderDetailModal)
- **z-60**: 业务流程关键弹窗 (CashPaymentModal, QuickAddModal)
- **z-80**: 管理后台 CRUD (ProductModal, CategoryModal)
- **z-90**: 二级确认弹窗 (嵌套在 z-80 内)
- **z-100**: 特殊交互组件 (ItemConfiguratorModal)
- **z-9999**: 全局最高优先级 (SupervisorAuthModal)

#### Portal 渲染
使用 `createPortal(component, document.body)` 避免 z-index 冲突:
```tsx
import { createPortal } from 'react-dom';

return createPortal(
  <div className="fixed inset-0 z-100 ...">
    {/* Modal Content */}
  </div>,
  document.body
);
```

---

### 配色系统

#### 主色系统
- **品牌色**: `primary-500` (#FF5E5E)
- **成功**: `green-600`
- **危险**: `red-600`
- **警告**: `orange-500`
- **信息**: `blue-500`
- **中性**: `gray-700`

#### 语义化配色 (价格明细)
遵循 `red_coral/CLAUDE.md` 中的颜色语言:

| 类型 | 文字颜色 | 徽标颜色 | 按钮颜色 |
|------|----------|----------|----------|
| 赠送 (comp) | `text-emerald-600` | - | `bg-emerald-500` |
| 手动折扣 | `text-orange-500` | `bg-orange-100 text-orange-700` | `bg-orange-500` |
| 规则折扣 | `text-amber-600` | `bg-amber-100 text-amber-700` | - |
| 规则附加费 | `text-purple-500` | `bg-purple-100 text-purple-700` | - |
| 整单折扣 | `text-orange-500` | - | `bg-orange-500` |
| 整单附加费 | `text-purple-500` | - | `bg-purple-500` |

#### 状态配色
- **选中**: `border-orange-500 bg-orange-50 ring-2 ring-orange-200`
- **禁用**: `bg-gray-50 text-gray-300 border-gray-100`
- **错误**: `border-red-500 bg-red-50 text-red-600`
- **成功**: `border-green-500 bg-green-50 text-green-600`

---

### 可访问性 (A11y)

#### ARIA 属性
```tsx
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
  aria-describedby="modal-description"
>
  <h2 id="modal-title">{title}</h2>
  <p id="modal-description">{description}</p>
</div>
```

#### 键盘支持
- **ESC 键**: 关闭 Modal (非 blocking 模式)
- **Tab 键**: 焦点陷阱 (Focus Trap)
- **Enter 键**: 确认操作

#### 最小触摸目标
- **按钮最小尺寸**: 44×44 CSS 像素 (WCAG 2.1)
- **当前实现**: `h-12` (48px) ✅ 或 `p-2` (~36px) ⚠️ 需要调整

---

### 响应式设计

#### 断点策略
遵循 Tailwind 默认断点:
- `sm`: 640px (小屏手机)
- `md`: 768px (平板)
- `lg`: 1024px (小桌面)
- `xl`: 1280px (标准桌面)
- `2xl`: 1536px (大桌面)

#### Modal 响应式模式
```tsx
{/* 布局切换: 移动端纵向,桌面横向 */}
<div className="flex flex-col md:flex-row">

{/* 尺寸调整 */}
<h3 className="text-xl md:text-2xl">标题</h3>

{/* 间距适配 */}
<div className="p-4 md:p-6">内容</div>

{/* Grid 列数响应 */}
<div className="grid grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
```

---

## 🎯 实施指南

### 新建 Modal 组件
1. 复制标准模板 (见下文)
2. 选择合适的 z-index 层级
3. 根据场景选择 Header 背景色
4. 使用统一的关闭按钮样式
5. 添加 `animate-in` 动画类

### 标准 Modal 模板
```tsx
import React from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface MyModalProps {
  isOpen: boolean;
  onClose: () => void;
  // ... 其他 props
}

export const MyModal: React.FC<MyModalProps> = ({ isOpen, onClose }) => {
  const { t } = useI18n();

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200"
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-bold text-gray-900">{t('my_modal.title')}</h2>
            <button
              onClick={onClose}
              className="p-2 hover:bg-gray-100 rounded-full transition-colors"
            >
              <X size={20} className="text-gray-500" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {/* 表单内容 */}
        </div>

        {/* Footer */}
        <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50 flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-5 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-200 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleConfirm}
            className="px-5 py-2.5 bg-primary-600 text-white rounded-xl text-sm font-semibold hover:bg-primary-700 transition-colors shadow-lg shadow-primary-600/20"
          >
            {t('common.action.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
```

---

## 📝 更新日志

- **2026-02-06**: 初始版本创建,基于现有 Modal 组件评估
