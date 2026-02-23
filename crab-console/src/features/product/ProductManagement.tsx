import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, Package, X, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listProducts, createProduct, updateProduct, deleteProduct, listCategories } from '@/infrastructure/api/catalog';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import { formatCurrency } from '@/utils/format';
import type {
  CatalogProduct, ProductCreate, ProductUpdate, ProductSpecInput,
  CatalogCategory,
} from '@/core/types/catalog';

interface FormSpec {
  name: string;
  price: number;
  is_default: boolean;
  is_active: boolean;
}

function computePriceDisplay(specs: { price: number; is_active: boolean }[]): string {
  const active = specs.filter(s => s.is_active);
  if (active.length === 0) return '-';
  if (active.length === 1) return formatCurrency(active[0].price);
  const prices = active.map(s => s.price);
  const min = Math.min(...prices);
  const max = Math.max(...prices);
  if (min === max) return formatCurrency(min);
  return `${formatCurrency(min)} - ${formatCurrency(max)}`;
}

export const ProductManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [products, setProducts] = useState<CatalogProduct[]>([]);
  const [categories, setCategories] = useState<CatalogCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<CatalogProduct | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields
  const [formName, setFormName] = useState('');
  const [formCategoryId, setFormCategoryId] = useState<number | ''>('');
  const [formTaxRate, setFormTaxRate] = useState<number>(0);
  const [formSortOrder, setFormSortOrder] = useState<number>(0);
  const [formSpecs, setFormSpecs] = useState<FormSpec[]>([]);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<CatalogProduct | null>(null);

  const loadData = useCallback(async () => {
    if (!token) return;
    try {
      setLoading(true);
      const [prodData, catData] = await Promise.all([
        listProducts(token, storeId),
        listCategories(token, storeId),
      ]);
      setProducts(prodData);
      setCategories(catData);
      setError('');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, t]);

  useEffect(() => { loadData(); }, [loadData]);

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return products;
    const q = searchQuery.toLowerCase();
    return products.filter(p =>
      p.name.toLowerCase().includes(q) ||
      (p.category_name?.toLowerCase().includes(q))
    );
  }, [products, searchQuery]);

  const categoryOptions = useMemo(() =>
    categories
      .filter(c => c.is_active)
      .map(c => ({ value: c.source_id, label: c.name })),
    [categories]
  );

  const openCreate = () => {
    setEditing(null);
    setFormName('');
    setFormCategoryId('');
    setFormTaxRate(0);
    setFormSortOrder(0);
    setFormSpecs([{ name: '', price: 0, is_default: true, is_active: true }]);
    setFormError('');
    setModalOpen(true);
  };

  const openEdit = (prod: CatalogProduct) => {
    setEditing(prod);
    setFormName(prod.name);
    setFormCategoryId(prod.category_source_id);
    setFormTaxRate(prod.tax_rate);
    setFormSortOrder(prod.sort_order);
    setFormSpecs(
      prod.specs.map(s => ({
        name: s.name,
        price: s.price,
        is_default: s.is_default,
        is_active: s.is_active,
      }))
    );
    setFormError('');
    setModalOpen(true);
  };

  const addSpec = () => {
    setFormSpecs([...formSpecs, { name: '', price: 0, is_default: false, is_active: true }]);
  };

  const removeSpec = (index: number) => {
    setFormSpecs(formSpecs.filter((_, i) => i !== index));
  };

  const updateSpec = (index: number, field: keyof FormSpec, value: string | number | boolean) => {
    setFormSpecs(formSpecs.map((s, i) => {
      if (i !== index) {
        // If setting default on this index, unset on others
        if (field === 'is_default' && value === true) return { ...s, is_default: false };
        return s;
      }
      return { ...s, [field]: value };
    }));
  };

  const buildSpecInputs = (): ProductSpecInput[] =>
    formSpecs.map((s, i) => ({
      name: s.name.trim(),
      price: s.price,
      display_order: i,
      is_default: s.is_default,
      is_active: s.is_active,
      is_root: formSpecs.length === 1,
    }));

  const handleSave = async () => {
    if (!token) return;
    if (!formName.trim()) { setFormError(t('settings.common.required_field')); return; }
    if (formCategoryId === '') { setFormError(t('settings.common.required_field')); return; }
    if (formSpecs.length === 0) { setFormError(t('settings.product.spec_required')); return; }

    setSaving(true);
    setFormError('');
    try {
      if (editing) {
        const payload: ProductUpdate = {
          name: formName.trim(),
          category_id: Number(formCategoryId),
          tax_rate: formTaxRate,
          sort_order: formSortOrder,
          specs: buildSpecInputs(),
        };
        await updateProduct(token, storeId, editing.source_id, payload);
      } else {
        const payload: ProductCreate = {
          name: formName.trim(),
          category_id: Number(formCategoryId),
          tax_rate: formTaxRate,
          sort_order: formSortOrder,
          specs: buildSpecInputs(),
        };
        await createProduct(token, storeId, payload);
      }
      setModalOpen(false);
      await loadData();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!token || !deleteTarget) return;
    try {
      await deleteProduct(token, storeId, deleteTarget.source_id);
      setDeleteTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteTarget(null);
    }
  };

  const columns: Column<CatalogProduct>[] = [
    {
      key: 'name',
      header: t('settings.common.name'),
      render: (p) => (
        <span className={`font-medium ${p.is_active ? 'text-gray-900' : 'text-gray-400 line-through'}`}>
          {p.name}
        </span>
      ),
    },
    {
      key: 'category',
      header: t('settings.product.category'),
      width: '140px',
      render: (p) => (
        p.category_name ? (
          <span className="inline-flex px-2.5 py-0.5 rounded-full text-xs font-medium bg-teal-50 text-teal-700 border border-teal-200">
            {p.category_name}
          </span>
        ) : (
          <span className="text-gray-400">-</span>
        )
      ),
    },
    {
      key: 'price',
      header: t('settings.product.price'),
      width: '150px',
      render: (p) => (
        <span className="text-sm font-medium text-gray-900">
          {computePriceDisplay(p.specs)}
        </span>
      ),
    },
    {
      key: 'status',
      header: t('settings.common.status'),
      width: '100px',
      render: (p) => (
        <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
          p.is_active ? 'bg-green-50 text-green-700' : 'bg-gray-100 text-gray-500'
        }`}>
          {p.is_active ? t('settings.common.active') : t('settings.common.inactive')}
        </span>
      ),
    },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-blue-100 rounded-xl flex items-center justify-center">
            <Package size={20} className="text-blue-600" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('settings.product.title')}</h2>
            <p className="text-sm text-gray-500">{t('settings.product.subtitle')}</p>
          </div>
        </div>
        <button
          onClick={openCreate}
          className="inline-flex items-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-xl text-sm font-medium hover:bg-blue-700 transition-colors shadow-sm"
        >
          <Plus size={16} />
          {t('common.action.add')}
        </button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{error}</div>
      )}

      {/* Filter */}
      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        totalCount={filtered.length}
        countUnit={t('settings.product.unit')}
        themeColor="blue"
      />

      {/* Table */}
      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        onEdit={openEdit}
        onDelete={(p) => setDeleteTarget(p)}
        getRowKey={(p) => p.source_id}
        themeColor="blue"
      />

      {/* Modal */}
      {modalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm"
          onClick={(e) => { if (e.target === e.currentTarget) setModalOpen(false); }}
        >
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-lg overflow-hidden max-h-[90vh] flex flex-col" style={{ animation: 'slideUp 0.25s ease-out' }}>
            {/* Modal Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 shrink-0">
              <h3 className="text-lg font-bold text-gray-900">
                {editing ? t('common.action.edit') : t('common.action.add')} {t('settings.product.title')}
              </h3>
              <button onClick={() => setModalOpen(false)} className="p-1 hover:bg-gray-100 rounded-lg transition-colors">
                <X size={20} className="text-gray-400" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="px-6 py-5 space-y-4 overflow-y-auto">
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.common.name')} required>
                <input
                  type="text"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.product.name_placeholder')}
                />
              </FormField>

              <SelectField
                label={t('settings.product.category')}
                value={formCategoryId}
                onChange={(v) => setFormCategoryId(Number(v))}
                options={categoryOptions}
                required
                placeholder={t('settings.product.category_placeholder')}
              />

              <div className="grid grid-cols-2 gap-4">
                <FormField label={t('settings.product.tax_rate')}>
                  <input
                    type="number"
                    value={formTaxRate}
                    onChange={(e) => setFormTaxRate(Number(e.target.value))}
                    className={inputClass}
                    step="0.01"
                    min={0}
                  />
                </FormField>

                <FormField label={t('settings.product.sort_order')}>
                  <input
                    type="number"
                    value={formSortOrder}
                    onChange={(e) => setFormSortOrder(Number(e.target.value))}
                    className={inputClass}
                    min={0}
                  />
                </FormField>
              </div>

              {/* Specs */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <label className="block text-sm font-medium text-gray-700">{t('settings.product.specs')}</label>
                  <button
                    type="button"
                    onClick={addSpec}
                    className="inline-flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-blue-700 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors"
                  >
                    <Plus size={12} />
                    {t('settings.product.add_spec')}
                  </button>
                </div>

                {formSpecs.length === 0 && (
                  <div className="text-sm text-gray-400 text-center py-4 border border-dashed border-gray-200 rounded-xl">
                    {t('settings.product.no_specs')}
                  </div>
                )}

                {formSpecs.map((spec, idx) => (
                  <div key={idx} className="bg-gray-50 rounded-xl p-3 space-y-2">
                    <div className="flex items-center gap-2">
                      <input
                        type="text"
                        value={spec.name}
                        onChange={(e) => updateSpec(idx, 'name', e.target.value)}
                        className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                        placeholder={t('settings.product.spec_name')}
                      />
                      <input
                        type="number"
                        value={spec.price}
                        onChange={(e) => updateSpec(idx, 'price', Number(e.target.value))}
                        className="w-28 px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                        placeholder={t('settings.product.price')}
                        step="0.01"
                        min={0}
                      />
                      <button
                        type="button"
                        onClick={() => removeSpec(idx)}
                        className="p-2 text-red-500 hover:bg-red-50 rounded-lg transition-colors"
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                    <div className="flex items-center gap-4 px-1">
                      <label className="flex items-center gap-2 text-xs text-gray-600 cursor-pointer">
                        <input
                          type="radio"
                          name="default_spec"
                          checked={spec.is_default}
                          onChange={() => updateSpec(idx, 'is_default', true)}
                          className="text-blue-600 focus:ring-blue-500"
                        />
                        {t('settings.product.is_default')}
                      </label>
                      <CheckboxField
                        id={`spec_active_${idx}`}
                        label={t('settings.common.active')}
                        checked={spec.is_active}
                        onChange={(v) => updateSpec(idx, 'is_active', v)}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Modal Footer */}
            <div className="px-6 py-4 border-t border-gray-100 flex justify-end gap-3 shrink-0">
              <button
                onClick={() => setModalOpen(false)}
                className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleSave}
                disabled={saving}
                className="px-4 py-2.5 bg-blue-600 text-white rounded-xl text-sm font-medium hover:bg-blue-700 transition-colors disabled:opacity-50"
              >
                {saving ? t('auth.loading') : t('common.action.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={!!deleteTarget}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.product.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
        variant="danger"
      />
    </div>
  );
};
