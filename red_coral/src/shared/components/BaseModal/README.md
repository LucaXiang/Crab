# BaseModal 使用指南

`BaseModal` 是 Red Coral 项目的统一 Modal 基础组件,提供标准的三段式布局和一致的样式规范。

---

## 📦 导入

```tsx
import { BaseModal } from '@/shared/components/BaseModal';
import { Z_INDEX } from '@/shared/constants/zIndex';
```

---

## 🎯 基础用法

### 简单确认对话框

```tsx
import { BaseModal } from '@/shared/components/BaseModal';
import { Z_INDEX } from '@/shared/constants/zIndex';

function DeleteConfirmModal({ isOpen, onClose, onConfirm }: Props) {
  return (
    <BaseModal
      isOpen={isOpen}
      onClose={onClose}
      title="确认删除"
      headerVariant="danger"
      zIndex={Z_INDEX.MODAL_NESTED}
      maxWidth="sm"
      footer={
        <>
          <button
            onClick={onClose}
            className="px-5 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-200 transition-colors"
          >
            取消
          </button>
          <button
            onClick={onConfirm}
            className="px-5 py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
          >
            删除
          </button>
        </>
      }
    >
      <p className="text-gray-600">此操作不可撤销,确定要删除吗?</p>
    </BaseModal>
  );
}
```

---

### 标准 CRUD 表单

```tsx
import { BaseModal } from '@/shared/components/BaseModal';
import { Z_INDEX } from '@/shared/constants/zIndex';

function ProductEditModal({ isOpen, onClose, product }: Props) {
  return (
    <BaseModal
      isOpen={isOpen}
      onClose={onClose}
      title={product ? '编辑商品' : '新建商品'}
      headerVariant="primary"
      zIndex={Z_INDEX.MODAL_MANAGEMENT}
      maxWidth="2xl"
      footer={
        <>
          <button onClick={onClose} className="...">
            取消
          </button>
          <button onClick={handleSave} className="...">
            保存
          </button>
        </>
      }
    >
      <ProductForm data={product} onChange={handleChange} />
    </BaseModal>
  );
}
```

---

### 支付流程 (强调遮罩)

```tsx
import { BaseModal } from '@/shared/components/BaseModal';
import { Z_INDEX } from '@/shared/constants/zIndex';

function PaymentModal({ isOpen, onClose, amount }: Props) {
  return (
    <BaseModal
      isOpen={isOpen}
      onClose={onClose}
      title="现金支付"
      zIndex={Z_INDEX.MODAL_BUSINESS}
      maxWidth="4xl"
      emphasizedOverlay={true} // 60% 黑色遮罩
      closeOnBackdropClick={false} // 禁止点击背景关闭
    >
      <PaymentForm amount={amount} onConfirm={handlePayment} />
    </BaseModal>
  );
}
```

---

## 🎨 Props 参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `isOpen` | `boolean` | - | **必填** - 是否显示 Modal |
| `onClose` | `() => void` | - | **必填** - 关闭回调 |
| `title` | `string` | - | **必填** - Modal 标题 |
| `children` | `ReactNode` | - | **必填** - 内容区域 |
| `headerVariant` | `HeaderVariant` | `'default'` | Header 背景色变体 |
| `zIndex` | `ZIndexValue` | `Z_INDEX.MODAL_BASE` | z-index 层级 |
| `maxWidth` | `ModalMaxWidth` | `'2xl'` | 最大宽度 |
| `footer` | `ReactNode` | `undefined` | Footer 区域 (可选) |
| `showCloseButton` | `boolean` | `true` | 是否显示关闭按钮 |
| `closeOnBackdropClick` | `boolean` | `true` | 点击背景是否关闭 |
| `emphasizedOverlay` | `boolean` | `false` | 是否使用强调遮罩 (60%) |
| `className` | `string` | `''` | 自定义 className |

---

## 🎨 HeaderVariant 变体

| 变体 | 背景色 | 适用场景 |
|------|-------|---------|
| `'default'` | `bg-white` | 普通信息展示 |
| `'danger'` | `bg-red-50` | 删除、作废等危险操作 |
| `'warning'` | `bg-orange-50` | 折扣、附加费等警告操作 |
| `'auth'` | `bg-teal-50` | 主管授权、权限升级 |
| `'primary'` | `bg-primary-50` | 商品编辑、资源管理 |

---

## 📐 MaxWidth 预设

| 预设 | Tailwind 类 | 像素值 | 适用场景 |
|------|------------|--------|---------|
| `'sm'` | `max-w-sm` | 384px | 简单确认框 |
| `'md'` | `max-w-md` | 448px | 单字段表单 |
| `'lg'` | `max-w-lg` | 512px | - |
| `'xl'` | `max-w-xl` | 576px | - |
| `'2xl'` | `max-w-2xl` | 672px | **标准 CRUD 表单** |
| `'4xl'` | `max-w-4xl` | 896px | 双栏配置器、支付面板 |

---

## 📊 Z-Index 层级

推荐使用 `Z_INDEX` 常量而非硬编码:

```tsx
import { Z_INDEX } from '@/shared/constants/zIndex';

<BaseModal zIndex={Z_INDEX.MODAL_MANAGEMENT} ... />
```

| 常量 | 值 | 适用场景 |
|------|---|---------|
| `Z_INDEX.MODAL_BASE` | 50 | 普通信息展示 (订单详情) |
| `Z_INDEX.MODAL_BUSINESS` | 60 | 业务流程 (支付/快速添加) |
| `Z_INDEX.MODAL_MANAGEMENT` | 80 | Settings CRUD (商品/分类) |
| `Z_INDEX.MODAL_NESTED` | 90 | 嵌套确认弹窗 (未保存提示) |
| `Z_INDEX.MODAL_CONFIGURATOR` | 100 | 特殊配置器 (属性选择) |
| `Z_INDEX.MODAL_AUTH` | 9999 | 权限升级 (主管授权) |

---

## ✨ 特性

### 1. ESC 键关闭
按下 ESC 键自动关闭 Modal (除非设置 `closeOnBackdropClick={false}`)。

### 2. 点击背景关闭
点击遮罩背景关闭 Modal,可通过 `closeOnBackdropClick={false}` 禁用。

### 3. 统一动画
- 遮罩: `animate-in fade-in duration-200` (淡入)
- 内容: `animate-in zoom-in-95 duration-200` (缩放进入)

### 4. 响应式高度
最大高度 `max-h-[90vh]`,留出状态栏空间。

---

## 🔄 迁移现有 Modal

### Before (旧代码)
```tsx
function MyModal({ isOpen, onClose }: Props) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm ...">
      <div className="bg-white rounded-2xl ...">
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-100">
          <h2>标题</h2>
          <button onClick={onClose}><X /></button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto">...</div>

        {/* Footer */}
        <div className="px-6 py-4 border-t bg-gray-50">
          <button>取消</button>
          <button>确认</button>
        </div>
      </div>
    </div>
  );
}
```

### After (使用 BaseModal)
```tsx
import { BaseModal } from '@/shared/components/BaseModal';
import { Z_INDEX } from '@/shared/constants/zIndex';

function MyModal({ isOpen, onClose }: Props) {
  return (
    <BaseModal
      isOpen={isOpen}
      onClose={onClose}
      title="标题"
      zIndex={Z_INDEX.MODAL_MANAGEMENT}
      footer={
        <>
          <button onClick={onClose}>取消</button>
          <button onClick={handleConfirm}>确认</button>
        </>
      }
    >
      {/* 只需要写内容区域! */}
      <YourContent />
    </BaseModal>
  );
}
```

**优势**:
- ✅ 减少 50+ 行重复代码
- ✅ 自动支持 ESC 键关闭
- ✅ 统一样式和动画
- ✅ z-index 集中管理

---

## 📚 参考

- 设计规范: `/red_coral/DESIGN_SYSTEM.md`
- z-index 常量: `/red_coral/src/shared/constants/zIndex.ts`
- 项目规范: `/red_coral/CLAUDE.md`
