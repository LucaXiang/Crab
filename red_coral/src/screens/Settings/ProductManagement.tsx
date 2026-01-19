import React, { useEffect, useMemo, useState } from 'react';
import { Utensils, Plus, Filter, Search } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';
import { useCanDeleteProduct, useCanUpdateProduct } from '@/hooks/usePermission';
import {
  useSettingsModal,
  useDataVersion,
  useSettingsFilters,
} from '@/core/stores/settings/useSettingsStore';
import { useProductStore, useCategoryStore } from '@/core/stores/resources';
import { createApiClient } from '@/infrastructure/api';

const api = createApiClient();
import { DataTable, Column } from '@/presentation/components/ui/DataTable';
import { convertFileSrc } from '@tauri-apps/api/core';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';
import DefaultImage from '../../assets/reshot.svg';
import { formatCurrency } from '@/utils/currency';

interface ProductItem {
  id: string;
  name: string;
  price: number;
  category: string;
  image: string;
  externalId: number | null;
  receiptName?: string;
  sortOrder?: number;
  taxRate?: number;
  kitchenPrinterId?: number | null;
  kitchenPrintName?: string;
  isKitchenPrintEnabled?: number | null;
  isLabelPrintEnabled?: number | null;
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
      result = result.filter((p: any) => p.category === categoryFilter || p.categoryId === categoryFilter);
    }
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter((p: any) =>
        p.name?.toLowerCase().includes(query) ||
        p.receiptName?.toLowerCase().includes(query)
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
      title: t('settings.product.list.batchDeleteTitle'),
      description: t('settings.product.list.confirmBatchDelete', { count: items.length }) || `确定要删除选中的 ${items.length} 个菜品吗？此操作无法撤销。`,
      onConfirm: async () => {
        setConfirmDialog(prev => ({ ...prev, isOpen: false }));
        try {
          const ids = items.map((item) => Number(item.id));
          await api.bulkDeleteProducts(ids);
          // Optimistic update: remove from ProductStore
          items.forEach((item) => {
            useProductStore.getState().optimisticRemove(item.id);
          });
          toast.success(t('settings.product.list.batchDeleteSuccess'));
        } catch (e) {
          console.error(e);
          toast.error(t('settings.product.list.batchDeleteFailed'));
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
          const imgSrc = item.image
            ? /^https?:\/\//.test(item.image)
              ? item.image
              : convertFileSrc(item.image)
            : DefaultImage;
          return (
            <div className="flex items-center gap-3">
              <img
                src={imgSrc}
                alt={item.name}
                className="w-10 h-10 rounded-lg object-cover border border-gray-200"
                onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
              />
              <div>
                <span className="font-medium text-gray-900">{item.name}</span>
                {item.receiptName && (
                  <div className="text-xs text-gray-400">
                    {item.receiptName}
                  </div>
                )}
                <div className="text-xs text-gray-400 mt-0.5">
                  ID: {item.id.slice(0, 8)} 
                  {item.externalId && <span className="ml-2 px-1 bg-gray-100 rounded text-gray-600">Ext: {item.externalId}</span>}
                </div>
              </div>
            </div>
          );
        },
      },
      {
        key: 'price',
        header: t('settings.product.header.price'),
        width: '120px',
        align: 'right',
        render: (item) => (
          <span className="inline-flex items-center px-3 py-1 bg-emerald-50 text-emerald-700 rounded-full text-sm font-bold">
            {formatCurrency(item.price)}
          </span>
        ),
      },
      {
        key: 'kitchenPrinting',
        header: t('settings.product.print.kitchenPrinting'),
        width: '220px',
        render: (item) => {
          const isDefault =
            item.isKitchenPrintEnabled === undefined || item.isKitchenPrintEnabled === null || item.isKitchenPrintEnabled === -1;

          const stateLabel = isDefault
            ? t('common.default')
            : item.isKitchenPrintEnabled === 1
            ? t('common.enabled')
            : t('common.disabled');

          const chipClass = isDefault
            ? 'bg-blue-50 text-blue-700'
            : item.isKitchenPrintEnabled === 1
            ? 'bg-emerald-50 text-emerald-700'
            : 'bg-gray-100 text-gray-600';

          return (
            <div className="flex flex-col gap-1 text-xs">
              <span
                className={`inline-flex items-center px-2 py-0.5 rounded-full font-medium ${chipClass}`}
              >
                {stateLabel}
              </span>
              <span className="text-gray-400">
                {item.kitchenPrinterId
                  ? `${t('settings.kitchenPrinter')} #${item.kitchenPrinterId}`
                  : t('common.default')}
              </span>
            </div>
          );
        },
      },
      {
        key: 'labelPrinting',
        header: t('settings.product.print.labelPrinting'),
        width: '120px',
        render: (item) => {
          const isDefault =
            item.isLabelPrintEnabled === undefined || item.isLabelPrintEnabled === null || item.isLabelPrintEnabled === -1;

          const stateLabel = isDefault
            ? t('common.default')
            : item.isLabelPrintEnabled === 1
            ? t('common.enabled')
            : t('common.disabled');

          const chipClass = isDefault
            ? 'bg-blue-50 text-blue-700'
            : item.isLabelPrintEnabled === 1
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
        width: '140px',
        render: (item) => (
          <span className="inline-flex items-center px-2.5 py-1 bg-blue-50 text-blue-700 rounded-full text-xs font-medium">
            {item.category}
          </span>
        ),
      },
    ],
    [t]
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
          <ProtectedGate permission={Permission.CREATE_PRODUCT}>
            <button
              onClick={() => openModal('PRODUCT', 'CREATE')}
              className="inline-flex items-center gap-2 px-4 py-2.5 bg-orange-500 text-white rounded-xl text-sm font-semibold shadow-lg shadow-orange-500/20 hover:bg-orange-600 hover:shadow-orange-500/30 transition-all"
            >
              <Plus size={16} />
              <span>{t('settings.product.action.add')}</span>
            </button>
          </ProtectedGate>
        </div>
      </div>

      {/* Filter Bar */}
      <div className="bg-white rounded-xl border border-gray-200 p-4 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 text-gray-500">
            <Filter size={16} />
            <span className="text-sm font-medium">{t('common.filter')}</span>
          </div>
          <div className="h-5 w-px bg-gray-200" />
          <div className="flex items-center gap-2">
            <label className="text-sm text-gray-600">{t('settings.category.title')}:</label>
            <select
              value={categoryFilter}
              onChange={(e) => setCategoryFilter(e.target.value as any)}
              className="border border-gray-200 rounded-lg px-3 py-1.5 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500 transition-colors min-w-[140px]"
            >
              <option value="all">{t('common.all')}</option>
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
              placeholder={t('common.searchPlaceholder')}
              className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500"
            />
          </div>

          <div className="ml-auto text-xs text-gray-400">
            {t('common.total')} {filteredProducts.length} {t('common.items')}
          </div>
        </div>
      </div>

      {/* Data Table */}
      <DataTable
        data={filteredProducts}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.id}
        onEdit={canUpdateProduct ? (item) => openModal('PRODUCT', 'EDIT', item) : undefined}
        onDelete={canDeleteProduct ? (item) => openModal('PRODUCT', 'DELETE', item) : undefined}
        onBatchDelete={canDeleteProduct ? handleBatchDelete : undefined}
        emptyText={t('settings.product.list.noData')}
        pageSize={5}
        totalItems={filteredProducts.length}
        currentPage={page}
        onPageChange={(newPage) => setPagination(newPage, total)}
        themeColor="orange"
      />

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog(prev => ({ ...prev, isOpen: false }))}
      />
    </div>
  );
});
