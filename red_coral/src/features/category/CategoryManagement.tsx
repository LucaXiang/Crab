import React, { useEffect, useMemo, useState } from 'react';
import { Tag, FolderOpen, ArrowUp, ArrowDown, List, Filter } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useSettingsModal, useDataVersion } from '@/core/stores/settings/useSettingsStore';
import { useCategoryStore } from './store';
import { createTauriClient } from '@/infrastructure/api';

const api = createTauriClient();
import { DataTable, Column } from '@/shared/components/DataTable';
import { toast } from '@/presentation/components/Toast';
import { Permission, Category } from '@/core/domain/types';
import { useCanManageCategories } from '@/hooks/usePermission';

// Extracted components
import { ManagementHeader, FilterBar, ProductOrderModal } from '@/screens/Settings/components';

interface CategoryItem extends Category {
  originalIndex: number;
}

export const CategoryManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  // Permission check
  const canManageCategories = useCanManageCategories();

  // Use resources store for data
  const categoryStore = useCategoryStore();
  const storeCategories = categoryStore.items;
  const loading = categoryStore.isLoading;

  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();

  // Local state for ordered categories (for reordering)
  const [categories, setCategories] = useState<Category[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [productOrderModal, setProductOrderModal] = useState<{
    isOpen: boolean;
    category: string;
  }>({ isOpen: false, category: '' });

  // Load data on mount and when dataVersion changes
  useEffect(() => {
    categoryStore.fetchAll();
  }, [dataVersion]);

  // Sync local state with store
  useEffect(() => {
    setCategories(storeCategories);
  }, [storeCategories]);

  const categoryItems: CategoryItem[] = useMemo(
    () => categories.map((cat, index) => ({ ...cat, originalIndex: index })),
    [categories]
  );

  const filteredItems = useMemo(() => {
    if (!searchQuery.trim()) return categoryItems;
    const q = searchQuery.toLowerCase();
    return categoryItems.filter((c) => c.name.toLowerCase().includes(q));
  }, [categoryItems, searchQuery]);

  const moveCategory = async (index: number, direction: 'up' | 'down') => {
    if (searchQuery) return;

    const newCategories = [...categories];
    const targetIndex = direction === 'up' ? index - 1 : index + 1;

    if (targetIndex < 0 || targetIndex >= newCategories.length) return;

    [newCategories[index], newCategories[targetIndex]] = [newCategories[targetIndex], newCategories[index]];
    setCategories(newCategories);

    try {
      // Call backend API to persist the new order
      const updates = newCategories.map((cat, idx) => ({
        id: cat.id,
        sort_order: idx
      }));
      await api.batchUpdateCategorySortOrder(updates);

      // Refresh categories from store
      await categoryStore.fetchAll();
    } catch (e) {
      console.error(e);
      toast.error(t('settings.reorder_failed'));
      // Revert to server data
      await categoryStore.fetchAll();
    }
  };

  const columns: Column<CategoryItem>[] = useMemo(
    () => [
      {
        key: 'sort',
        header: t('settings.category.header.sort'),
        width: '100px',
        align: 'center',
        render: (item) => {
          if (searchQuery) return <span className="text-gray-300">-</span>;
          const index = item.originalIndex;
          return (
            <div className="flex items-center justify-center gap-1">
              <button
                onClick={(e) => { e.stopPropagation(); moveCategory(index, 'up'); }}
                disabled={!canManageCategories || index === 0}
                className="p-1 hover:bg-gray-100 rounded text-gray-400 hover:text-teal-600 disabled:opacity-30 transition-colors"
              >
                <ArrowUp size={14} />
              </button>
              <button
                onClick={(e) => { e.stopPropagation(); moveCategory(index, 'down'); }}
                disabled={!canManageCategories || index === categories.length - 1}
                className="p-1 hover:bg-gray-100 rounded text-gray-400 hover:text-teal-600 disabled:opacity-30 transition-colors"
              >
                <ArrowDown size={14} />
              </button>
            </div>
          );
        },
      },
      {
        key: 'name',
        header: t('settings.category.form.name'),
        render: (item) => (
          <div className="flex items-center gap-3">
            <div className={`w-9 h-9 rounded-lg flex items-center justify-center ${
              item.is_virtual
                ? 'bg-linear-to-br from-purple-100 to-purple-200'
                : 'bg-linear-to-br from-teal-100 to-teal-200'
            }`}>
              {item.is_virtual ? (
                <Filter size={16} className="text-purple-600" />
              ) : (
                <FolderOpen size={16} className="text-teal-600" />
              )}
            </div>
            <div className="flex items-center gap-2">
              <span className="font-medium text-gray-900">{item.name}</span>
              {item.is_virtual && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-purple-100 text-purple-700 font-medium">
                  {t('settings.category.label.virtual')}
                </span>
              )}
            </div>
          </div>
        ),
      },
      {
        key: 'kitchenPrinting',
        header: t('settings.product.print.kitchen_printing'),
        width: '120px',
        render: (item) => {
          // API returns boolean, check for is_kitchen_print_enabled
          const isEnabled = item.is_kitchen_print_enabled === true;
          return (
            <span
              className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                isEnabled ? 'bg-orange-50 text-orange-700' : 'bg-gray-100 text-gray-600'
              }`}
            >
              {isEnabled ? (t('common.status.enabled')) : (t('common.status.disabled'))}
            </span>
          );
        },
      },
      {
        key: 'labelPrinting',
        header: t('settings.product.print.label_printing'),
        width: '120px',
        render: (item) => {
          // API returns boolean
          const isEnabled = item.is_label_print_enabled === true;
          return (
            <span
              className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                isEnabled ? 'bg-emerald-50 text-emerald-700' : 'bg-gray-100 text-gray-600'
              }`}
            >
              {isEnabled ? (t('common.status.enabled')) : (t('common.status.disabled'))}
            </span>
          );
        },
      },
      {
        key: 'actions',
        header: t('settings.product.header.products'),
        width: '140px',
        align: 'right',
        render: (item) => (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setProductOrderModal({ isOpen: true, category: item.name });
            }}
            disabled={!canManageCategories}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-teal-50 text-teal-700 rounded-lg text-xs font-medium hover:bg-teal-100 transition-colors border border-teal-100 disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
          >
            <List size={14} />
            <span>{t('settings.category.adjust_category_order')}</span>
          </button>
        ),
      },
    ],
    [t, categories, searchQuery, canManageCategories]
  );

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={Tag}
        title={t('settings.category.title')}
        description={t('settings.category.description')}
        addButtonText={t('settings.category.add_category')}
        onAdd={() => openModal('CATEGORY', 'CREATE')}
        themeColor="teal"
        permission={Permission.MANAGE_CATEGORIES}
      />

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('common.hint.search_placeholder')}
        totalCount={filteredItems.length}
        countUnit={t('settings.category.unit')}
        themeColor="teal"
      />

      <DataTable
        data={filteredItems}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.name}
        onEdit={canManageCategories ? (item) => openModal('CATEGORY', 'EDIT', item) : undefined}
        onDelete={canManageCategories ? (item) => openModal('CATEGORY', 'DELETE', item) : undefined}
        emptyText={t('common.empty.no_data')}
        themeColor="teal"
      />

      <ProductOrderModal
        isOpen={productOrderModal.isOpen}
        category={productOrderModal.category}
        onClose={() => setProductOrderModal({ ...productOrderModal, isOpen: false })}
      />
    </div>
  );
});
