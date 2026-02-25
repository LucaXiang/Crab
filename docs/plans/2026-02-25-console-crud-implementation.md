# Console CRUD Master-Detail 重构实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 crab-console 的 9 个 CRUD 管理页面从 DataTable+Modal 模式重构为 Master-Detail 响应式布局

**Architecture:** 新建 3 个共享组件 (MasterDetail, ItemCard, DetailPanel) 替代 DataTable+Modal 模式。桌面端左右分栏 (40%/60%)，手机端全屏列表/详情切换。保留 FilterBar, FormField, ConfirmDialog 等现有组件。

**Tech Stack:** React 19, TypeScript 5.9, Tailwind CSS v4, lucide-react, 现有 API 层不变

---

## Task 1: 创建 MasterDetail 共享组件

**Files:**
- Create: `crab-console/src/shared/components/MasterDetail/MasterDetail.tsx`
- Create: `crab-console/src/shared/components/MasterDetail/index.ts`

**Step 1: 创建 MasterDetail 组件**

```tsx
// crab-console/src/shared/components/MasterDetail/MasterDetail.tsx
import React, { useCallback, useEffect, useState } from 'react';
import { Plus, ArrowLeft, X } from 'lucide-react';
import { FilterBar } from '../FilterBar';
import { useI18n } from '@/hooks/useI18n';

type ThemeColor = 'blue' | 'teal' | 'orange' | 'purple' | 'indigo';

interface MasterDetailProps<T> {
  items: T[];
  getItemId: (item: T) => number | string;
  renderItem: (item: T, isSelected: boolean) => React.ReactNode;
  selectedId: number | string | null;
  onSelect: (item: T) => void;
  onDeselect: () => void;

  searchQuery: string;
  onSearchChange: (query: string) => void;
  totalCount: number;
  countUnit: string;

  onCreateNew: () => void;
  createLabel: string;
  isCreating: boolean;

  children: React.ReactNode;
  themeColor?: ThemeColor;
  loading?: boolean;
  emptyText?: string;
}

const themeClasses: Record<ThemeColor, { selected: string; border: string }> = {
  blue:   { selected: 'bg-blue-50 border-l-blue-500',   border: 'border-blue-200' },
  teal:   { selected: 'bg-teal-50 border-l-teal-500',   border: 'border-teal-200' },
  orange: { selected: 'bg-orange-50 border-l-orange-500', border: 'border-orange-200' },
  purple: { selected: 'bg-purple-50 border-l-purple-500', border: 'border-purple-200' },
  indigo: { selected: 'bg-indigo-50 border-l-indigo-500', border: 'border-indigo-200' },
};

export function MasterDetail<T>({
  items, getItemId, renderItem, selectedId, onSelect, onDeselect,
  searchQuery, onSearchChange, totalCount, countUnit,
  onCreateNew, createLabel, isCreating,
  children, themeColor = 'blue', loading, emptyText,
}: MasterDetailProps<T>) {
  const { t } = useI18n();
  const [isMobile, setIsMobile] = useState(false);
  const showDetail = selectedId !== null || isCreating;
  const theme = themeClasses[themeColor];

  useEffect(() => {
    const mq = window.matchMedia('(max-width: 1023px)');
    setIsMobile(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsMobile(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  const handleBack = useCallback(() => {
    onDeselect();
  }, [onDeselect]);

  // ─── 列表部分 ───
  const listContent = (
    <div className="flex flex-col h-full">
      <div className="p-3 lg:p-4 space-y-3">
        <div className="flex items-center gap-2">
          <div className="flex-1">
            <FilterBar
              searchQuery={searchQuery}
              onSearchChange={onSearchChange}
              totalCount={totalCount}
              countUnit={countUnit}
              themeColor={themeColor}
            />
          </div>
          <button
            onClick={onCreateNew}
            className="flex items-center gap-1.5 px-3 py-2 bg-primary-500 text-white rounded-lg text-sm font-medium hover:bg-primary-600 transition-colors shrink-0"
          >
            <Plus className="w-4 h-4" />
            <span className="hidden sm:inline">{createLabel}</span>
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto px-3 lg:px-4 pb-3">
        {loading ? (
          <div className="flex items-center justify-center py-12 text-gray-400">
            <div className="animate-spin w-6 h-6 border-2 border-gray-300 border-t-gray-600 rounded-full" />
          </div>
        ) : items.length === 0 ? (
          <div className="text-center py-12 text-gray-400 text-sm">
            {emptyText || t('common.label.no_data')}
          </div>
        ) : (
          <div className="space-y-1">
            {items.map((item) => {
              const id = getItemId(item);
              const isSelected = id === selectedId;
              return (
                <button
                  key={String(id)}
                  onClick={() => onSelect(item)}
                  className={`w-full text-left rounded-lg border-l-4 transition-colors cursor-pointer ${
                    isSelected
                      ? `${theme.selected} border-l-4`
                      : 'border-l-transparent hover:bg-gray-50'
                  }`}
                >
                  {renderItem(item, isSelected)}
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );

  // ─── 详情部分 ───
  const detailContent = showDetail ? (
    <div className="flex flex-col h-full">
      {/* 手机端返回头 */}
      {isMobile && (
        <div className="flex items-center gap-3 px-4 py-3 border-b border-gray-200 bg-white sticky top-0 z-10">
          <button onClick={handleBack} className="p-1 -ml-1 rounded-lg hover:bg-gray-100">
            <ArrowLeft className="w-5 h-5 text-gray-600" />
          </button>
        </div>
      )}
      <div className="flex-1 overflow-y-auto">
        {children}
      </div>
    </div>
  ) : (
    <div className="flex items-center justify-center h-full text-gray-400 text-sm">
      {t('common.label.select_item')}
    </div>
  );

  // ─── 手机端：列表或详情全屏 ───
  if (isMobile) {
    if (showDetail) {
      return (
        <div className="fixed inset-0 z-40 bg-white" style={{ animation: 'slideInRight 0.2s ease-out' }}>
          {detailContent}
        </div>
      );
    }
    return <div className="h-full">{listContent}</div>;
  }

  // ─── 桌面端：左右分栏 ───
  return (
    <div className="flex h-full border border-gray-200 rounded-xl overflow-hidden bg-white">
      <div className="w-[40%] border-r border-gray-200 overflow-hidden flex flex-col">
        {listContent}
      </div>
      <div className="flex-1 overflow-hidden flex flex-col">
        {detailContent}
      </div>
    </div>
  );
}
```

```tsx
// crab-console/src/shared/components/MasterDetail/index.ts
export { MasterDetail } from './MasterDetail';
```

**Step 2: 在 shared/components/index.ts 添加导出**

修改: `crab-console/src/shared/components/index.ts`

添加:
```tsx
export { MasterDetail } from './MasterDetail';
```

**Step 3: 添加 slideInRight 动画到全局 CSS**

查找全局 CSS 文件 (可能是 `index.css` 或 `App.css`)，添加:

```css
@keyframes slideInRight {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}
```

**Step 4: 添加 i18n key**

在 `crab-console/src/infrastructure/i18n/locales/` 的 es.json, en.json, zh.json 添加:
- `common.label.select_item` → "Selecciona un elemento" / "Select an item" / "选择一个项目"
- `common.label.no_data` (如果不存在)

**Step 5: 验证 TypeScript 编译**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 6: Commit**

```bash
git add crab-console/src/shared/components/MasterDetail/
git commit -m "feat(console): add MasterDetail responsive layout component"
```

---

## Task 2: 创建 DetailPanel 共享组件

**Files:**
- Create: `crab-console/src/shared/components/DetailPanel/DetailPanel.tsx`
- Create: `crab-console/src/shared/components/DetailPanel/index.ts`

**Step 1: 创建 DetailPanel 组件**

```tsx
// crab-console/src/shared/components/DetailPanel/DetailPanel.tsx
import React from 'react';
import { X, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface DetailPanelProps {
  title: string;
  isCreating: boolean;
  onClose: () => void;
  onSave: () => void;
  onDelete?: () => void;
  saving?: boolean;
  saveDisabled?: boolean;
  saveLabel?: string;
  deleteLabel?: string;
  children: React.ReactNode;
}

export const DetailPanel: React.FC<DetailPanelProps> = ({
  title, isCreating, onClose, onSave, onDelete,
  saving, saveDisabled, saveLabel, deleteLabel, children,
}) => {
  const { t } = useI18n();

  return (
    <div className="flex flex-col h-full">
      {/* Header - 桌面端显示关闭按钮 */}
      <div className="flex items-center justify-between px-4 lg:px-6 py-3 border-b border-gray-200 bg-gray-50/50">
        <h2 className="text-lg font-bold text-slate-900">{title}</h2>
        <button
          onClick={onClose}
          className="hidden lg:flex p-1.5 rounded-lg hover:bg-gray-200 transition-colors"
        >
          <X className="w-4 h-4 text-gray-500" />
        </button>
      </div>

      {/* Form content */}
      <div className="flex-1 overflow-y-auto px-4 lg:px-6 py-4 space-y-4">
        {children}
      </div>

      {/* Footer buttons - 手机端 sticky bottom */}
      <div className="flex items-center gap-3 px-4 lg:px-6 py-3 border-t border-gray-200 bg-white sticky bottom-0">
        {!isCreating && onDelete && (
          <button
            onClick={onDelete}
            className="flex items-center gap-1.5 px-3 py-2.5 text-sm font-medium text-red-600 border border-red-200 rounded-lg hover:bg-red-50 transition-colors"
          >
            <Trash2 className="w-4 h-4" />
            {deleteLabel || t('catalog.delete')}
          </button>
        )}
        <div className="flex-1" />
        <button
          onClick={onClose}
          className="px-4 py-2.5 text-sm text-slate-600 hover:bg-slate-100 rounded-lg transition-colors"
        >
          {t('catalog.cancel')}
        </button>
        <button
          onClick={onSave}
          disabled={saving || saveDisabled}
          className="px-4 py-2.5 text-sm font-medium text-white bg-primary-500 hover:bg-primary-600 rounded-lg transition-colors disabled:opacity-50"
        >
          {saving ? t('catalog.saving') : (saveLabel || (isCreating ? t('catalog.create') : t('catalog.save')))}
        </button>
      </div>
    </div>
  );
};
```

```tsx
// crab-console/src/shared/components/DetailPanel/index.ts
export { DetailPanel } from './DetailPanel';
```

**Step 2: 添加导出到 shared/components/index.ts**

```tsx
export { DetailPanel } from './DetailPanel';
```

**Step 3: 添加 i18n keys (如缺失)**

- `catalog.create` → "Crear" / "Create" / "创建"
- `catalog.delete` → "Eliminar" / "Delete" / "删除"

**Step 4: 验证 TypeScript 编译**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 5: Commit**

```bash
git add crab-console/src/shared/components/DetailPanel/
git commit -m "feat(console): add DetailPanel form container component"
```

---

## Task 3: 用 Zone 模块验证 Master-Detail 模式 (P0)

**Files:**
- Modify: `crab-console/src/features/zone/ZoneManagement.tsx` (完全重写)

**Step 1: 重写 ZoneManagement 使用 MasterDetail + DetailPanel**

```tsx
// crab-console/src/features/zone/ZoneManagement.tsx
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { MapPin } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField';
import { listZones, createZone, updateZone, deleteZone } from '@/infrastructure/api/management';
import type { Zone, ZoneCreate, ZoneUpdate } from '@/core/types/store';

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: Zone }
  | { type: 'delete'; item: Zone };

export const ZoneManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<Zone[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);

  // Form state
  const [formName, setFormName] = useState('');
  const [formDesc, setFormDesc] = useState('');

  const handleError = useCallback((err: unknown) => {
    if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
    alert(err instanceof ApiError ? err.message : t('catalog.error'));
  }, [clearAuth, navigate, t]);

  const load = useCallback(async () => {
    if (!token) return;
    try { setItems(await listZones(token, storeId)); }
    catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(z => z.name.toLowerCase().includes(q));
  }, [items, search]);

  const selectedId = panel.type === 'edit' ? panel.item.id : null;

  const openCreate = () => {
    setFormName(''); setFormDesc('');
    setPanel({ type: 'create' });
  };

  const openEdit = (item: Zone) => {
    setFormName(item.name); setFormDesc(item.description || '');
    setPanel({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (panel.type === 'create') {
        const data: ZoneCreate = { name: formName.trim(), description: formDesc.trim() || undefined };
        await createZone(token, storeId, data);
      } else if (panel.type === 'edit') {
        const data: ZoneUpdate = { name: formName.trim(), description: formDesc.trim() || undefined };
        await updateZone(token, storeId, panel.item.id, data);
      }
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const handleDelete = async () => {
    if (!token || panel.type !== 'delete') return;
    setSaving(true);
    try {
      await deleteZone(token, storeId, panel.item.id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const renderItem = (zone: Zone, isSelected: boolean) => (
    <div className={`px-3 py-3 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center gap-2">
        <MapPin className="w-4 h-4 text-teal-500 shrink-0" />
        <span className={`text-sm ${zone.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
          {zone.name}
        </span>
      </div>
      {zone.description && (
        <p className="text-xs text-gray-400 mt-1 ml-6 truncate">{zone.description}</p>
      )}
    </div>
  );

  return (
    <div className="h-full p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4">
        <div className="w-10 h-10 bg-teal-100 rounded-xl flex items-center justify-center">
          <MapPin className="w-5 h-5 text-teal-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('zones.title')}</h1>
      </div>

      <div className="h-[calc(100%-3.5rem)]">
        <MasterDetail
          items={filtered}
          getItemId={(z) => z.id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('zones.title')}
          onCreateNew={openCreate}
          createLabel={t('zones.new')}
          isCreating={panel.type === 'create'}
          themeColor="teal"
          loading={loading}
          emptyText={t('zones.empty')}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? t('zones.new') : t('zones.edit')}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formName.trim()}
            >
              <FormField label={t('catalog.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
              </FormField>
              <FormField label={t('zones.description')}>
                <textarea value={formDesc} onChange={e => setFormDesc(e.target.value)} className={inputClass} rows={3} />
              </FormField>
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('catalog.confirm_delete')}
        description={t('catalog.confirm_delete_desc')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />
    </div>
  );
};
```

**Step 2: 验证 TypeScript 编译**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 3: 手动测试**

- 桌面端: 访问 Zone 管理页 → 左侧列表 + 右侧空状态 → 点击项 → 右侧编辑面板 → 保存/删除
- 手机端 (Chrome DevTools): 列表全屏 → 点击 → 滑入详情 → 返回

**Step 4: Commit**

```bash
git add crab-console/src/features/zone/ZoneManagement.tsx
git commit -m "feat(console): rewrite Zone CRUD with Master-Detail pattern"
```

---

## Task 4: Tag 管理 Master-Detail 重写 (P0)

**Files:**
- Modify: `crab-console/src/features/tag/TagManagement.tsx` (完全重写)

**Step 1: 重写 TagManagement**

沿用 ZoneManagement 模式，关键差异:
- `themeColor="indigo"`
- 卡片项: 颜色圆点 + 名称 + 系统标签徽章 + 排序
- 表单: name + color picker (type="color" + text input) + display_order
- API: `listTags/createTag/updateTag/deleteTag` from `@/infrastructure/api/store`
- `getItemId`: `(tag) => tag.source_id`
- 系统标签 `is_system` 不可编辑/删除: 列表 onSelect 检查 `if (tag.is_system) return`

renderItem 示例:
```tsx
const renderItem = (tag: StoreTag, isSelected: boolean) => (
  <div className={`px-3 py-3 flex items-center gap-3 ${isSelected ? 'font-medium' : ''}`}>
    <div className="w-5 h-5 rounded-full border border-gray-200 shrink-0"
         style={{ backgroundColor: tag.color || '#6366f1' }} />
    <span className={`text-sm flex-1 ${tag.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
      {tag.name}
    </span>
    {tag.is_system && (
      <span className="text-xs px-1.5 py-0.5 rounded bg-indigo-100 text-indigo-700">{t('tags.system')}</span>
    )}
  </div>
);
```

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/tag/TagManagement.tsx
git commit -m "feat(console): rewrite Tag CRUD with Master-Detail pattern"
```

---

## Task 5: Table 管理 Master-Detail 重写 (P0)

**Files:**
- Modify: `crab-console/src/features/table/TableManagement.tsx` (完全重写)

**Step 1: 重写 TableManagement**

关键差异:
- `themeColor="blue"`
- 需要加载 zones 列表用于 zone_id select: `useEffect` 中同时 `listTables + listZones`
- 卡片项: 名称 + zone 名 (从 zones 查找) + 座位数
- 表单: name + zone_id (SelectField/native select) + capacity (number)
- API: `listTables/createTable/updateTable/deleteTable` from `@/infrastructure/api/management`
- `getItemId`: `(table) => table.id`

renderItem 需要用 zones Map 查找 zone 名:
```tsx
const zoneMap = useMemo(() => new Map(zones.map(z => [z.id, z.name])), [zones]);

const renderItem = (table: DiningTable, isSelected: boolean) => (
  <div className={`px-3 py-3 ${isSelected ? 'font-medium' : ''}`}>
    <div className="flex items-center justify-between">
      <span className={`text-sm ${table.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
        {table.name}
      </span>
      <span className="text-xs text-gray-400">{table.capacity} {t('tables.seats')}</span>
    </div>
    <p className="text-xs text-gray-400 mt-0.5">{zoneMap.get(table.zone_id) || '-'}</p>
  </div>
);
```

表单 zone select:
```tsx
<FormField label={t('tables.zone')} required>
  <select value={formZoneId} onChange={e => setFormZoneId(Number(e.target.value))} className={inputClass}>
    <option value={0} disabled>{t('tables.select_zone')}</option>
    {zones.filter(z => z.is_active).map(z => (
      <option key={z.id} value={z.id}>{z.name}</option>
    ))}
  </select>
</FormField>
```

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/table/TableManagement.tsx
git commit -m "feat(console): rewrite Table CRUD with Master-Detail pattern"
```

---

## Task 6: Employee 管理 Master-Detail 重写 (P1)

**Files:**
- Modify: `crab-console/src/features/employee/EmployeeManagement.tsx` (完全重写)

**Step 1: 重写 EmployeeManagement**

关键差异:
- `themeColor="orange"`
- 卡片项: 用户名 + 显示名 + 角色徽章 (从 role_id 映射) + 活跃状态
- 表单: username + display_name + password (新建必填, 编辑可选) + role_id (select)
- API: `listEmployees/createEmployee/updateEmployee/deleteEmployee` from `@/infrastructure/api/management`
- `getItemId`: `(emp) => emp.id`
- 系统员工 `is_system` 不可删除

角色映射 (hardcoded or from API if available):
```tsx
const ROLES = [
  { id: 1, label: 'Admin' },
  { id: 2, label: 'Manager' },
  { id: 3, label: 'Cashier' },
];
```

注意: password 字段编辑时显示 placeholder "留空不修改"。

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/employee/EmployeeManagement.tsx
git commit -m "feat(console): rewrite Employee CRUD with Master-Detail pattern"
```

---

## Task 7: Category 管理 Master-Detail 重写 (P1)

**Files:**
- Modify: `crab-console/src/features/category/CategoryManagement.tsx` (完全重写)

**Step 1: 重写 CategoryManagement**

关键差异:
- `themeColor="teal"`
- 需要加载 tags 列表用于 tag_ids
- 卡片项: 名称 + 虚拟标签 (is_virtual 用 pill badge) + tag 计数 + 打印图标
- 表单: name + is_virtual (checkbox) + sort_order + tag_ids (TagPicker) + is_kitchen_print_enabled (checkbox) + is_label_print_enabled (checkbox) + match_mode (select) + is_display (checkbox) + kitchen_print_destinations + label_print_destinations
- API: `listCategories/createCategory/updateCategory/deleteCategory` from `@/infrastructure/api/store`
- `getItemId`: `(cat) => cat.source_id`

表单分组:
```
基本信息: name, sort_order, is_virtual, is_display, match_mode
标签关联: tag_ids (TagPicker)
打印设置: is_kitchen_print_enabled, is_label_print_enabled, kitchen_print_destinations, label_print_destinations
```

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/category/CategoryManagement.tsx
git commit -m "feat(console): rewrite Category CRUD with Master-Detail pattern"
```

---

## Task 8: Product 管理 Master-Detail 重写 (P2)

**Files:**
- Modify: `crab-console/src/features/product/ProductManagement.tsx` (完全重写)

**Step 1: 重写 ProductManagement**

最复杂的模块，关键差异:
- `themeColor="blue"`
- 需要加载 categories + tags 列表
- 卡片项: 名称 + 默认价格 + 分类名 + tag 小圆点 + 活跃/非活跃
- 表单:
  - 基本: name, category_id (select), sort_order, tax_rate, receipt_name, kitchen_print_name
  - 规格 (specs): 内嵌子列表，每个 spec 有 name/price/is_default/is_active/display_order/receipt_name/is_root
  - 标签: tag_ids (TagPicker)
  - 打印: is_kitchen_print_enabled, is_label_print_enabled
  - 外部: external_id
- API: `listProducts/createProduct/updateProduct/deleteProduct` from `@/infrastructure/api/store`
- `getItemId`: `(p) => p.source_id`

specs 子编辑器:
```tsx
// 在 DetailPanel 内嵌 specs 列表
// 每个 spec 一行: [名称] [价格] [默认?] [×删除]
// [+ 添加规格] 按钮
```

renderItem 显示默认规格价格:
```tsx
const defaultSpec = product.specs.find(s => s.is_default) || product.specs[0];
const price = defaultSpec ? defaultSpec.price.toFixed(2) : '-';
```

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/product/ProductManagement.tsx
git commit -m "feat(console): rewrite Product CRUD with Master-Detail pattern"
```

---

## Task 9: Attribute 管理 Master-Detail 重写 (P2)

**Files:**
- Modify: `crab-console/src/features/attribute/AttributeManagement.tsx` (完全重写)

**Step 1: 重写 AttributeManagement**

关键差异:
- `themeColor="purple"`
- 卡片项: 名称 + 选项数量 + 多选标记
- 表单:
  - 基本: name, is_multi_select (checkbox), max_selections (number, 仅多选时显示), display_order
  - 打印: show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name
  - 选项子 CRUD: 内嵌选项列表，每个选项有 name/price_modifier/display_order/receipt_name/kitchen_print_name/enable_quantity/max_quantity
- API: `listAttributes/createAttribute/updateAttribute/deleteAttribute` from `@/infrastructure/api/store`
- `getItemId`: `(attr) => attr.source_id`

选项编辑器类似 Product specs，但字段更多:
```tsx
// 选项行: [名称] [价格修饰] [数量?] [×]
// [+ 添加选项]
```

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/attribute/AttributeManagement.tsx
git commit -m "feat(console): rewrite Attribute CRUD with Master-Detail pattern"
```

---

## Task 10: PriceRule 列表改为 Master-Detail (P3)

**Files:**
- Modify: `crab-console/src/features/price-rule/PriceRuleManagement.tsx`

**Step 1: 重写列表部分为 MasterDetail**

PriceRule 特殊: 编辑保持现有向导/表单，仅列表入口改为 Master-Detail 布局。

关键差异:
- `themeColor="orange"`
- 卡片项: 名称 + 类型徽章 (DISCOUNT/SURCHARGE) + 调整值 + 活跃状态圆点
- DetailPanel 内嵌现有向导表单的所有字段
- API: `listPriceRules/createPriceRule/updatePriceRule/deletePriceRule` from `@/infrastructure/api/store`
- `getItemId`: `(rule) => rule.source_id`

renderItem:
```tsx
const renderItem = (rule: PriceRule, isSelected: boolean) => (
  <div className={`px-3 py-3 ${isSelected ? 'font-medium' : ''}`}>
    <div className="flex items-center justify-between">
      <span className="text-sm text-slate-900">{rule.name}</span>
      <span className={`text-xs px-1.5 py-0.5 rounded ${
        rule.rule_type === 'DISCOUNT' ? 'bg-amber-100 text-amber-700' : 'bg-purple-100 text-purple-700'
      }`}>{rule.rule_type}</span>
    </div>
    <div className="flex items-center gap-2 mt-0.5">
      <span className="text-xs text-gray-400">
        {rule.adjustment_type === 'PERCENTAGE' ? `${rule.adjustment_value}%` : `${rule.adjustment_value.toFixed(2)}`}
      </span>
      <div className={`w-1.5 h-1.5 rounded-full ${rule.is_active ? 'bg-green-500' : 'bg-gray-300'}`} />
    </div>
  </div>
);
```

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/price-rule/PriceRuleManagement.tsx
git commit -m "feat(console): rewrite PriceRule list with Master-Detail pattern"
```

---

## Task 11: LabelTemplate 列表改为 Master-Detail (P3)

**Files:**
- Modify: `crab-console/src/features/label-template/LabelTemplateManagement.tsx`

**Step 1: 重写列表部分为 MasterDetail**

LabelTemplate 特殊: 点击编辑→跳转到全屏编辑器 (保持不变)。MasterDetail 仅用于列表浏览 + 新建入口。

关键差异:
- `themeColor="purple"`
- 卡片项: 名称 + 尺寸 (width×height mm) + 默认标记 + 字段数
- onSelect: 跳转到编辑器路由 (如果有) 或打开编辑器
- 新建: 跳转到创建页 (如果有) 或内嵌基本信息表单

注意: 需要先阅读现有 LabelTemplateManagement 了解编辑器跳转逻辑。如果编辑器是同页面组件，则 DetailPanel 内嵌简要信息 + "打开编辑器" 按钮。

**Step 2: 验证 + Commit**

Run: `cd crab-console && npx tsc --noEmit`
```bash
git add crab-console/src/features/label-template/LabelTemplateManagement.tsx
git commit -m "feat(console): rewrite LabelTemplate list with Master-Detail pattern"
```

---

## Task 12: 清理 DataTable 组件 (可选)

**Files:**
- Check: 确认 DataTable 是否还有其他使用者 (非 CRUD 页面如 Orders, Reports 可能仍在用)
- 如果仅 CRUD 页面使用，可标记为 deprecated 或删除

**Step 1: 搜索 DataTable 引用**

Run: `grep -r "DataTable" crab-console/src/ --include="*.tsx" --include="*.ts" -l`

**Step 2: 根据结果决定是否删除**

- 如果只有 CRUD feature 模块引用 → 全部已替换 → 删除 DataTable 目录
- 如果还有其他页面引用 → 保留

**Step 3: Commit (如有变更)**

```bash
git add -A crab-console/src/shared/components/DataTable/
git commit -m "chore(console): remove unused DataTable component"
```

---

## 实现备注

### 不改动的部分
- 路由结构 (`App.tsx`) — 保持所有 `/stores/:id/*` 路由
- `StoreLayout` — 导航侧栏和移动端 tab 不变
- API 层 (`infrastructure/api/`) — 所有 API 函数不变
- 类型定义 (`core/types/store.ts`) — 不变
- `FilterBar`, `FormField`, `ConfirmDialog`, `StatusToggle`, `TagPicker` — 保留原样
- 非 CRUD 页面 (LiveOrders, Orders, Reports, Overview, Settings, DataTransfer, RedFlags)

### 每个模块重写模板

所有 9 个模块遵循统一结构:
1. 用 `useState<PanelState>` 替代 `useState<ModalState>` (create/edit/delete/closed)
2. `MasterDetail` 包裹列表 + 详情
3. `DetailPanel` 包裹表单内容
4. `ConfirmDialog` 处理删除确认
5. 无 Modal，无 DataTable，无 Column 定义
6. `renderItem` 函数定义卡片内容

### 主题色分配 (与现有一致)
| 模块 | themeColor |
|------|-----------|
| Zone | teal |
| Tag | indigo |
| Table | blue |
| Employee | orange |
| Category | teal |
| Product | blue |
| Attribute | purple |
| PriceRule | orange |
| LabelTemplate | purple |
