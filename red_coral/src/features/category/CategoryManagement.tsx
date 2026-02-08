import React, { useEffect, useMemo, useState } from 'react';
import { Tag, FolderOpen, ArrowUp, ArrowDown, List, Filter, Search } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useSettingsModal, useDataVersion } from '@/core/stores/settings/useSettingsStore';
import { useCategoryStore } from './store';
import { createTauriClient } from '@/infrastructure/api';

const getApi = () => createTauriClient();
import { DataTable, Column } from '@/shared/components/DataTable';
import { toast } from '@/presentation/components/Toast';
import { Permission, Category } from '@/core/domain/types';
import { useCanManageMenu } from '@/hooks/usePermission';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Plus } from 'lucide-react';

// Extracted components
import { ProductOrderModal } from '@/screens/Settings/components';

interface CategoryItem extends Category {
  originalIndex: number;
}

// Separate component for category list (used for both normal and virtual)
interface CategoryListProps {
  categories: Category[];
  setCategories: React.Dispatch<React.SetStateAction<Category[]>>;
  loading: boolean;
  isVirtual: boolean;
  searchQuery: string;
  themeColor: 'teal' | 'purple';
  onProductOrder: (category: { id: number; name: string }) => void;
}

const CategoryList: React.FC<CategoryListProps> = React.memo(({
  categories,
  setCategories,
  loading,
  isVirtual,
  searchQuery,
  themeColor,
  onProductOrder,
}) => {
  const { t } = useI18n();
  const canManageCategories = useCanManageMenu();
  const { openModal } = useSettingsModal();
  const categoryStore = useCategoryStore();

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
      await getApi().batchUpdateCategorySortOrder(updates);

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
              isVirtual
                ? 'bg-linear-to-br from-purple-100 to-purple-200'
                : 'bg-linear-to-br from-teal-100 to-teal-200'
            }`}>
              {isVirtual ? (
                <Filter size={16} className="text-purple-600" />
              ) : (
                <FolderOpen size={16} className="text-teal-600" />
              )}
            </div>
            <span className="font-medium text-gray-900">{item.name}</span>
          </div>
        ),
      },
      {
        key: 'kitchenPrinting',
        header: t('settings.product.print.kitchen_printing'),
        width: '120px',
        render: (item) => {
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
              onProductOrder({ id: item.id, name: item.name });
            }}
            disabled={!canManageCategories}
            className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap ${
              isVirtual
                ? 'bg-purple-50 text-purple-700 hover:bg-purple-100 border-purple-100'
                : 'bg-teal-50 text-teal-700 hover:bg-teal-100 border-teal-100'
            }`}
          >
            <List size={14} />
            <span>{t('settings.category.adjust_category_order')}</span>
          </button>
        ),
      },
    ],
    [t, categories, searchQuery, canManageCategories, isVirtual, onProductOrder]
  );

  return (
    <DataTable
      data={filteredItems}
      columns={columns}
      loading={loading}
      getRowKey={(item) => item.name}
      onEdit={canManageCategories ? (item) => openModal('CATEGORY', 'EDIT', item) : undefined}
      onDelete={canManageCategories ? (item) => openModal('CATEGORY', 'DELETE', item) : undefined}
      emptyText={t('common.empty.no_data')}
      themeColor={themeColor}
    />
  );
});

export const CategoryManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  // Permission check
  const canManageCategories = useCanManageMenu();

  // Use resources store for data
  const categoryStore = useCategoryStore();
  const storeCategories = categoryStore.items;
  const loading = categoryStore.isLoading;

  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();

  // Tab state
  const [activeTab, setActiveTab] = useState<'normal' | 'virtual'>('normal');

  // Local state for ordered categories (for reordering) - separated by type
  const [normalCategories, setNormalCategories] = useState<Category[]>([]);
  const [virtualCategories, setVirtualCategories] = useState<Category[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [productOrderModal, setProductOrderModal] = useState<{
    isOpen: boolean;
    categoryId: number;
    categoryName: string;
  }>({ isOpen: false, categoryId: 0, categoryName: '' });

  // Load data on mount and when dataVersion changes
  useEffect(() => {
    categoryStore.fetchAll();
  }, [dataVersion]);

  // Sync local state with store - separate normal and virtual
  useEffect(() => {
    const normal = storeCategories.filter(c => !c.is_virtual);
    const virtual = storeCategories.filter(c => c.is_virtual);
    setNormalCategories(normal);
    setVirtualCategories(virtual);
  }, [storeCategories]);

  // Current tab's filtered items count
  const currentItems = activeTab === 'normal' ? normalCategories : virtualCategories;
  const filteredCount = useMemo(() => {
    if (!searchQuery.trim()) return currentItems.length;
    const q = searchQuery.toLowerCase();
    return currentItems.filter((c) => c.name.toLowerCase().includes(q)).length;
  }, [currentItems, searchQuery]);

  return (
    <div className="space-y-5">
      {/* Header Card */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${
              activeTab === 'normal' ? 'bg-teal-100' : 'bg-purple-100'
            }`}>
              {activeTab === 'normal' ? (
                <Tag size={20} className="text-teal-600" />
              ) : (
                <Filter size={20} className="text-purple-600" />
              )}
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {t('settings.category.title')}
              </h2>
              <p className="text-sm text-gray-500 mt-1">
                {t('settings.category.description')}
              </p>
            </div>
          </div>
          <ProtectedGate permission={Permission.MENU_MANAGE}>
            <button
              onClick={() => openModal('CATEGORY', 'CREATE', activeTab === 'virtual' ? { is_virtual: true } : undefined)}
              className={`inline-flex items-center gap-2 px-4 py-2.5 text-white rounded-xl text-sm font-semibold shadow-lg transition-all ${
                activeTab === 'normal'
                  ? 'bg-teal-600 shadow-teal-600/20 hover:bg-teal-700 hover:shadow-teal-600/30'
                  : 'bg-purple-600 shadow-purple-600/20 hover:bg-purple-700 hover:shadow-purple-600/30'
              }`}
            >
              <Plus size={16} />
              <span>{t('settings.category.add_category')}</span>
            </button>
          </ProtectedGate>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex space-x-1 bg-gray-100 p-1 rounded-xl w-fit">
        <button
          onClick={() => setActiveTab('normal')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === 'normal'
              ? 'bg-white text-teal-600 shadow-sm'
              : 'text-gray-600 hover:text-gray-900 hover:bg-gray-200/50'
          }`}
        >
          <FolderOpen size={16} />
          {t('settings.category.tab.normal')}
          <span className={`text-xs px-1.5 py-0.5 rounded-full ${
            activeTab === 'normal' ? 'bg-teal-100 text-teal-700' : 'bg-gray-200 text-gray-500'
          }`}>
            {normalCategories.length}
          </span>
        </button>
        <button
          onClick={() => setActiveTab('virtual')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === 'virtual'
              ? 'bg-white text-purple-600 shadow-sm'
              : 'text-gray-600 hover:text-gray-900 hover:bg-gray-200/50'
          }`}
        >
          <Filter size={16} />
          {t('settings.category.tab.virtual')}
          <span className={`text-xs px-1.5 py-0.5 rounded-full ${
            activeTab === 'virtual' ? 'bg-purple-100 text-purple-700' : 'bg-gray-200 text-gray-500'
          }`}>
            {virtualCategories.length}
          </span>
        </button>
      </div>

      {/* Filter Bar */}
      <div className="bg-white rounded-xl border border-gray-200 p-4 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 text-gray-500">
            <Filter size={16} />
            <span className="text-sm font-medium">{t('common.action.filter')}</span>
          </div>
          <div className="h-5 w-px bg-gray-200" />

          <div className="relative flex-1 max-w-xs">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('common.hint.search_placeholder')}
              className={`w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 transition-colors ${
                activeTab === 'normal'
                  ? 'focus:ring-teal-500/20 focus:border-teal-500'
                  : 'focus:ring-purple-500/20 focus:border-purple-500'
              }`}
            />
          </div>

          <div className="ml-auto flex items-center gap-2">
            <div className={`w-2 h-2 rounded-full ${activeTab === 'normal' ? 'bg-teal-500' : 'bg-purple-500'}`} />
            <span className="text-sm text-gray-600">{t('common.label.total')}</span>
            <span className="text-sm font-bold text-gray-900">{filteredCount}</span>
            <span className="text-sm text-gray-600">{t('settings.category.unit')}</span>
          </div>
        </div>
      </div>

      {/* Category Lists */}
      {activeTab === 'normal' ? (
        <CategoryList
          categories={normalCategories}
          setCategories={setNormalCategories}
          loading={loading}
          isVirtual={false}
          searchQuery={searchQuery}
          themeColor="teal"
          onProductOrder={(cat) => setProductOrderModal({ isOpen: true, categoryId: cat.id, categoryName: cat.name })}
        />
      ) : (
        <CategoryList
          categories={virtualCategories}
          setCategories={setVirtualCategories}
          loading={loading}
          isVirtual={true}
          searchQuery={searchQuery}
          themeColor="purple"
          onProductOrder={(cat) => setProductOrderModal({ isOpen: true, categoryId: cat.id, categoryName: cat.name })}
        />
      )}

      <ProductOrderModal
        isOpen={productOrderModal.isOpen}
        categoryId={productOrderModal.categoryId}
        categoryName={productOrderModal.categoryName}
        onClose={() => setProductOrderModal({ ...productOrderModal, isOpen: false })}
      />
    </div>
  );
});
