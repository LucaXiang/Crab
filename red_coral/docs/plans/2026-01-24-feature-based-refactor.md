# Feature-Based Architecture Refactor

## 目标

将代码按资源类型重组，拆解 EntityFormModal (879行)，UI/UX 保持不变。

## 目录结构

```
src/
├── features/                    # 按资源分类
│   ├── product/
│   ├── category/
│   ├── tag/
│   ├── attribute/
│   ├── table/
│   ├── zone/
│   ├── user/
│   ├── role/
│   └── price-rule/
│
├── shared/                      # 可复用组件
│   ├── components/
│   │   ├── DataTable/
│   │   ├── SettingLayout/
│   │   ├── FormField/
│   │   ├── FilterBar/
│   │   └── ConfirmDialog/
│   └── hooks/
│
├── screens/Settings/            # 简化为路由入口
│   ├── index.tsx
│   └── SettingsSidebar.tsx
```

## 单个 Feature 结构

```
features/product/
├── ProductManagement.tsx    # 管理页面
├── ProductModal.tsx         # 创建/编辑模态框
├── ProductForm.tsx          # 表单组件
├── ProductCard.tsx          # 卡片组件 (如适用)
├── mutations.ts             # API 业务逻辑
├── store.ts                 # Zustand store
└── index.ts                 # 统一导出
```

## mutations.ts 模式

```typescript
// 纯业务逻辑，无 UI
export async function createProduct(data: ProductFormData) {
  const payload = transformToPayload(data);
  const created = await api.createProduct(payload);
  useProductStore.getState().optimisticAdd(created);
  return created;
}

export async function updateProduct(id: string, data: ProductFormData) {
  const payload = transformToPayload(data);
  const updated = await api.updateProduct(id, payload);
  useProductStore.getState().optimisticUpdate(id, () => updated);
  return updated;
}

export async function deleteProduct(id: string) {
  await api.deleteProduct(id);
  useProductStore.getState().optimisticRemove(id);
}
```

## XxxModal.tsx 模式

```typescript
export const ProductModal: React.FC = () => {
  const { modal, closeModal } = useSettingsModal();
  const { formData, setFormField } = useSettingsFormMeta();
  const { t } = useI18n();

  const handleSave = async () => {
    try {
      if (modal.action === 'CREATE') {
        await createProduct(formData);
        toast.success(t('settings.product.message.created'));
      } else {
        await updateProduct(modal.data.id, formData);
        toast.success(t('settings.product.message.updated'));
      }
      closeModal();
    } catch (e) {
      toast.error(getErrorMessage(e));
    }
  };

  return (
    <ModalContainer title={getTitle()} onClose={closeModal}>
      <ProductForm formData={formData} onFieldChange={setFormField} />
      <ModalFooter onSave={handleSave} onCancel={closeModal} />
    </ModalContainer>
  );
};
```

## Feature 列表

| Feature | 文件 | 来源 |
|---------|------|------|
| product | ProductManagement, ProductModal, ProductForm, ProductCard, mutations, store | EntityFormModal + Settings |
| category | CategoryManagement, CategoryModal, CategoryForm, mutations, store | 同上 |
| tag | TagManagement, TagModal, TagForm, mutations, store | 同上 |
| attribute | AttributeManagement, AttributeModal, AttributeForm, OptionForm, mutations, store | 同上 |
| table | TableManagement, TableModal, TableForm, mutations, store | 同上 |
| zone | ZoneManagement, ZoneModal, ZoneForm, mutations, store | 同上 |
| user | UserManagement, UserFormModal, ResetPasswordModal, mutations, store | Settings/modals/ |
| role | RolePermissionsEditor, mutations, store | Settings/ |
| price-rule | PriceRuleManagement, PriceRuleWizard/*, mutations, store | Settings/ |

## Shared 组件

| 组件 | 来源 |
|------|------|
| DataTable | presentation/components/ui/DataTable.tsx |
| SettingLayout | 新建，提取通用布局 |
| FormField | Settings/forms/FormField.tsx |
| FilterBar | Settings/components/FilterBar.tsx |
| ConfirmDialog | presentation/components/ui/ConfirmDialog.tsx |
| DeleteConfirmation | Settings/forms/DeleteConfirmation.tsx |

## 删除的文件

- `screens/Settings/EntityFormModal.tsx` → 完全拆解到各 feature
- `screens/Settings/forms/` 目录 → 移入各 feature
- `core/stores/resources/` → 移入各 feature

## 约束

- UI/UX 完全不变
- 类型检查必须通过
- 导入路径更新
