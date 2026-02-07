import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Utensils, Plus, Filter, Search, ListChecks, Edit3, Trash2, Settings } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission, PrintState, ProductSpec } from '@/core/domain/types';
import { useCanDeleteProduct, useCanUpdateProduct } from '@/hooks/usePermission';
import {
  useSettingsModal,
  useDataVersion,
  useSettingsFilters,
} from '@/core/stores/settings/useSettingsStore';
import { useProductStore } from './store';
import { useCategoryStore } from '@/core/stores/resources';
import { createTauriClient } from '@/infrastructure/api';

const getApi = () => createTauriClient();
import { DataTable, Column } from '@/shared/components/DataTable';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { ProductImage } from './ProductImage';
import { formatCurrency } from '@/utils/currency';
import { displayId } from '@/utils/formatting';
import { SpecificationManagementModal } from './SpecificationManagementModal';

// ProductItem matches Product type from models.ts (snake_case naming)
interface ProductItem {
  id: number;
  name: string;
  category_id: number;
  image: string;
  receipt_name?: string | null;
  sort_order?: number;
  tax_rate?: number;
  kitchen_print_name?: string | null;
  is_kitchen_print_enabled?: PrintState;
  is_label_print_enabled?: PrintState;
  is_active?: boolean;
  external_id?: number | null;
  specs?: ProductSpec[];
}

export const ProductManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  // Permission checks
  const canDeleteProduct = useCanDeleteProduct();
  const canUpdateProduct = useCanUpdateProduct();

  // Use resources stores for data
  const productStore = useProductStore();
  const categoryStore = useCategoryStore();
  const products = productStore.items;
  const categories = categoryStore.items;
  const loading = productStore.isLoading;

  // UI state from settings store
  const {
    productCategoryFilter: categoryFilter,
    productsPage: page,
    setProductCategoryFilter: setCategoryFilter,
    setProductsPagination: setPagination,
  } = useSettingsFilters();

  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();

  const [searchQuery, setSearchQuery] = useState('');

  // Filter products by category and search
  const filteredProducts = useMemo(() => {
    let result = products;
    if (categoryFilter !== 'all') {
      result = result.filter((p) => String(p.category_id) === categoryFilter);
    }
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter((p) =>
        p.name?.toLowerCase().includes(query) ||
        p.receipt_name?.toLowerCase().includes(query)
      );
    }
    return result;
  }, [products, categoryFilter, searchQuery]);

  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  const [isSelectionMode, setIsSelectionMode] = useState(false);

  // Specification management modal state
  const [specModalOpen, setSpecModalOpen] = useState(false);
  const [selectedProductForSpec, setSelectedProductForSpec] = useState<ProductItem | null>(null);

  const handleManageSpecs = useCallback((product: ProductItem) => {
    setSelectedProductForSpec(product);
    setSpecModalOpen(true);
  }, []);

  useEffect(() => {
    // Load data from resources stores
    categoryStore.fetchAll();
    productStore.fetchAll();
  }, [dataVersion]);

  // Update pagination when filtered products change
  useEffect(() => {
    setPagination(page, filteredProducts.length);
  }, [filteredProducts.length]);

  const handleBatchDelete = (items: ProductItem[]) => {
    setConfirmDialog({
      isOpen: true,
      title: t('settings.product.list.batch_delete_title'),
      description: t('settings.product.list.confirmBatchDelete', { count: items.length }) || `确定要删除选中的 ${items.length} 个菜品吗？此操作无法撤销。`,
      onConfirm: async () => {
        setConfirmDialog(prev => ({ ...prev, isOpen: false }));
        try {
          const ids = items.map((item) => item.id).filter((id): id is number => id != null);
          await getApi().bulkDeleteProducts(ids);
          // Optimistic update: remove from ProductStore
          items.forEach((item) => {
            useProductStore.getState().optimisticRemove(item.id);
          });
          toast.success(t('settings.product.list.batch_delete_success'));
        } catch (e) {
          console.error(e);
          toast.error(t('settings.product.list.batch_delete_failed'));
        }
      },
    });
  };

  const columns: Column<ProductItem>[] = useMemo(
    () => [
      {
        key: 'name',
        header: t('settings.product.form.name'),
        render: (item) => {
          return (
            <div className="flex items-center gap-3">
              <ProductImage
                src={item.image}
                alt={item.name}
                className="w-10 h-10 rounded-lg object-cover border border-gray-200"
              />
              <div>
                <span className="font-medium text-gray-900">{item.name}</span>
                {item.receipt_name && (
                  <div className="text-xs text-gray-400">
                    {item.receipt_name}
                  </div>
                )}
                <div className="text-xs text-gray-400 mt-0.5">
                  id: {displayId(item.id)}
                </div>
              </div>
            </div>
          );
        },
      },
      // Note: Price is in ProductSpecification, not Product. Price column removed.
      {
        key: 'external_id',
        header: t('settings.product.form.external_id'),
        width: '90px',
        render: (item) => (
          <span className="text-xs font-mono text-gray-500">
            {item.external_id ?? '-'}
          </span>
        ),
      },
      {
        key: 'price',
        header: t('settings.product.form.price'),
        width: '100px',
        render: (item) => {
          // 从 specs 中找到默认规格的价格
          const defaultSpec = item.specs?.find(s => s.is_default || s.is_root) ?? item.specs?.[0];
          const price = defaultSpec?.price ?? 0;
          return (
            <span className="text-sm font-medium text-gray-700">
              {formatCurrency(price)}
            </span>
          );
        },
      },
      {
        key: 'tax_rate',
        header: t('settings.product.form.tax_rate'),
        width: '90px',
        render: (item) => (
          <span className="text-xs text-gray-600">
            {item.tax_rate ?? 0}%
          </span>
        ),
      },
      {
        key: 'is_active',
        header: t('common.label.status'),
        width: '90px',
        render: (item) => (
          <span
            className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
              item.is_active !== false
                ? 'bg-emerald-50 text-emerald-700'
                : 'bg-gray-100 text-gray-500'
            }`}
          >
            {item.is_active !== false ? t('common.status.enabled') : t('common.status.disabled')}
          </span>
        ),
      },
      {
        key: 'kitchenPrinting',
        header: t('settings.product.print.kitchen_printing'),
        width: '110px',
        render: (item) => {
          const isDefault =
            item.is_kitchen_print_enabled === undefined || item.is_kitchen_print_enabled === null || item.is_kitchen_print_enabled === -1;

          const stateLabel = isDefault
            ? t('common.label.default')
            : item.is_kitchen_print_enabled === 1
            ? t('common.status.enabled')
            : t('common.status.disabled');

          const chipClass = isDefault
            ? 'bg-blue-50 text-blue-700'
            : item.is_kitchen_print_enabled === 1
            ? 'bg-emerald-50 text-emerald-700'
            : 'bg-gray-100 text-gray-600';

          return (
            <span
              className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${chipClass}`}
            >
              {stateLabel}
            </span>
          );
        },
      },
      {
        key: 'labelPrinting',
        header: t('settings.product.print.label_printing'),
        width: '110px',
        render: (item) => {
          const isDefault =
            item.is_label_print_enabled === undefined || item.is_label_print_enabled === null || item.is_label_print_enabled === -1;

          const stateLabel = isDefault
            ? t('common.label.default')
            : item.is_label_print_enabled === 1
            ? t('common.status.enabled')
            : t('common.status.disabled');

          const chipClass = isDefault
            ? 'bg-blue-50 text-blue-700'
            : item.is_label_print_enabled === 1
            ? 'bg-emerald-50 text-emerald-700'
            : 'bg-gray-100 text-gray-600';

          return (
             <span
               className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${chipClass}`}
             >
               {stateLabel}
             </span>
          );
        },
      },
      {
        key: 'category',
        header: t('settings.product.header.category'),
        width: '120px',
        render: (item) => {
          const categoryName = categories.find(c => c.id === item.category_id)?.name || String(item.category_id);
          return (
            <span className="inline-flex items-center px-2.5 py-1 bg-blue-50 text-blue-700 rounded-full text-xs font-medium">
              {categoryName}
            </span>
          );
        },
      },
      {
        key: 'actions',
        header: t('settings.common.actions'),
        width: '150px',
        align: 'right',
        render: (item) => (
          <div className="flex items-center justify-end gap-1">
            {canUpdateProduct && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleManageSpecs(item);
                }}
                className="p-2 bg-purple-50 text-purple-600 rounded-lg hover:bg-purple-100 transition-colors border border-purple-200/50"
                title={t('settings.specification.manage')}
              >
                <Settings size={14} />
              </button>
            )}
            {canUpdateProduct && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  openModal('PRODUCT', 'EDIT', item);
                }}
                className="p-2 bg-amber-50 text-amber-700 rounded-lg hover:bg-amber-100 transition-colors border border-amber-200/50"
                title={t('common.action.edit')}
              >
                <Edit3 size={14} />
              </button>
            )}
            {canDeleteProduct && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  openModal('PRODUCT', 'DELETE', item);
                }}
                className="p-2 bg-red-50 text-red-600 rounded-lg hover:bg-red-100 transition-colors border border-red-200/50"
                title={t('common.action.delete')}
              >
                <Trash2 size={14} />
              </button>
            )}
          </div>
        ),
      },
    ],
    [t, categories, canUpdateProduct, canDeleteProduct, openModal, handleManageSpecs]
  );

  return (
    <div className="space-y-5">
      {/* Header Card */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-orange-100 rounded-xl flex items-center justify-center">
              <Utensils size={20} className="text-orange-600" />
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {t('settings.product.title')}
              </h2>
              <p className="text-sm text-gray-500">
                {t('settings.product.description')}
              </p>
            </div>
          </div>
          <ProtectedGate permission={Permission.MENU_MANAGE}>
            <button
              onClick={() => openModal('PRODUCT', 'CREATE')}
              className="inline-flex items-center gap-2 px-4 py-2.5 bg-orange-500 text-white rounded-xl text-sm font-semibold shadow-lg shadow-orange-500/20 hover:bg-orange-600 hover:shadow-orange-500/30 transition-all"
            >
              <Plus size={16} />
              <span>{t('settings.product.add_product')}</span>
            </button>
          </ProtectedGate>
        </div>
      </div>

      {/* Filter Bar */}
      <div className="bg-white rounded-xl border border-gray-200 p-4 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 text-gray-500">
            <Filter size={16} />
            <span className="text-sm font-medium">{t('common.action.filter')}</span>
          </div>
          <div className="h-5 w-px bg-gray-200" />
          <div className="flex items-center gap-2">
            <label className="text-sm text-gray-600">{t('settings.category.title')}:</label>
            <select
              value={categoryFilter}
              onChange={(e) => setCategoryFilter(e.target.value)}
              className="border border-gray-200 rounded-lg px-3 py-1.5 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500 transition-colors min-w-[8.75rem]"
            >
              <option value="all">{t('common.status.all')}</option>
              {categories.map((c) => (
                <option key={c.name} value={c.name}>
                  {c.name}
                </option>
              ))}
            </select>
          </div>

          <div className="h-5 w-px bg-gray-200 ml-2" />
          <div className="relative flex-1 max-w-xs">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setPagination(1, filteredProducts.length);
              }}
              placeholder={t('common.hint.search_placeholder')}
              className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500"
            />
          </div>

          <div className="ml-auto flex items-center gap-3">
            <span className="text-xs text-gray-400">
              {t('common.label.total')} {filteredProducts.length} {t('common.label.items')}
            </span>
            {canDeleteProduct && !isSelectionMode && (
              <button
                onClick={() => setIsSelectionMode(true)}
                className="flex items-center gap-1 px-2 py-1 rounded-md text-xs transition-colors border border-transparent text-orange-600 bg-orange-50 hover:bg-orange-100 border-orange-100"
              >
                <ListChecks size={14} />
                <span>{t('common.action.select')}</span>
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Data Table */}
      <DataTable
        data={filteredProducts}
        columns={columns}
        loading={loading}
        getRowKey={(item) => String(item.id)}
        onBatchDelete={canDeleteProduct ? handleBatchDelete : undefined}
        emptyText={t('common.empty.no_data')}
        pageSize={5}
        totalItems={filteredProducts.length}
        currentPage={page}
        onPageChange={(newPage) => setPagination(newPage, filteredProducts.length)}
        themeColor="orange"
        isSelectionMode={isSelectionMode}
        onSelectionModeChange={setIsSelectionMode}
      />

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog(prev => ({ ...prev, isOpen: false }))}
      />

      {/* Specification Management Modal */}
      {specModalOpen && selectedProductForSpec && (
        <SpecificationManagementModal
          isOpen={specModalOpen}
          onClose={() => {
            setSpecModalOpen(false);
            setSelectedProductForSpec(null);
          }}
          productId={selectedProductForSpec.id}
          productName={selectedProductForSpec.name}
        />
      )}
    </div>
  );
});
