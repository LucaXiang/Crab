import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, Package, Trash2, CheckSquare, Link, Unlink } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import {
  listProducts, createProduct, updateProduct, deleteProduct,
  listCategories, listTags, listAttributes,
  listBindings, bindAttribute, unbindAttribute,
  batchUpdateProductSortOrder,
  bulkDeleteProducts,
} from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import { TagPicker } from '@/shared/components/TagPicker/TagPicker';
import { ImageUpload } from '@/shared/components/ImageUpload';
import { Thumbnail } from '@/shared/components/Thumbnail';
import { formatCurrency } from '@/utils/format';
import type {
  StoreProduct, ProductCreate, ProductUpdate, ProductSpecInput,
  StoreCategory, StoreTag, StoreAttribute, StoreBinding,
} from '@/core/types/store';

interface FormSpec {
  name: string;
  price: number;
  receipt_name: string;
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

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: StoreProduct }
  | { type: 'delete'; item: StoreProduct };

export const ProductManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [products, setProducts] = useState<StoreProduct[]>([]);
  const [categories, setCategories] = useState<StoreCategory[]>([]);
  const [tags, setTags] = useState<StoreTag[]>([]);
  const [attributes, setAttributes] = useState<StoreAttribute[]>([]);
  const [bindings, setBindings] = useState<StoreBinding[]>([]);
  const [bindingLoading, setBindingLoading] = useState(false);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields
  const [formName, setFormName] = useState('');
  const [formCategoryId, setFormCategoryId] = useState<number | ''>('');
  const [formTaxRate, setFormTaxRate] = useState<number>(0);
  const [formSortOrder, setFormSortOrder] = useState<number>(0);
  const [formReceiptName, setFormReceiptName] = useState('');
  const [formKitchenPrintName, setFormKitchenPrintName] = useState('');
  const [formIsKitchenPrint, setFormIsKitchenPrint] = useState(0);
  const [formIsLabelPrint, setFormIsLabelPrint] = useState(0);
  const [formExternalId, setFormExternalId] = useState('');
  const [formTagIds, setFormTagIds] = useState<number[]>([]);
  const [formImage, setFormImage] = useState('');
  const [formIsActive, setFormIsActive] = useState(true);
  const [formSpecs, setFormSpecs] = useState<FormSpec[]>([]);

  // Bulk selection
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkSelected, setBulkSelected] = useState<Set<number>>(new Set());
  const [bulkDeleting, setBulkDeleting] = useState(false);

  const toggleBulkItem = (id: number) => {
    setBulkSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const [showBulkConfirm, setShowBulkConfirm] = useState(false);

  const handleBulkDelete = async () => {
    if (!token || bulkSelected.size === 0) return;
    setBulkDeleting(true);
    try {
      await bulkDeleteProducts(token, storeId, Array.from(bulkSelected));
      setBulkSelected(new Set());
      setBulkMode(false);
      setShowBulkConfirm(false);
      await load();
    } catch (err) { setShowBulkConfirm(false); handleError(err); }
    finally { setBulkDeleting(false); }
  };

  const handleError = useCallback((err: unknown) => {
    alert(err instanceof ApiError ? err.message : t('auth.error_generic'));
  }, [t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      const [prodData, catData, tagData, attrData] = await Promise.all([
        listProducts(token, storeId),
        listCategories(token, storeId),
        listTags(token, storeId),
        listAttributes(token, storeId),
      ]);
      setProducts(prodData);
      setCategories(catData);
      setTags(tagData);
      setAttributes(attrData);
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search.trim()) return products;
    const q = search.toLowerCase();
    return products.filter(p =>
      p.name.toLowerCase().includes(q) || (p.category_name?.toLowerCase().includes(q))
    );
  }, [products, search]);

  const selectedId = panel.type === 'edit' ? panel.item.source_id : null;

  const categoryOptions = useMemo(() =>
    categories.filter(c => c.is_active).map(c => ({ value: c.source_id, label: c.name })),
    [categories]
  );

  const toggleTag = (tagId: number) => {
    setFormTagIds(prev =>
      prev.includes(tagId) ? prev.filter(id => id !== tagId) : [...prev, tagId]
    );
  };

  const openCreate = () => {
    setFormName(''); setFormImage(''); setFormCategoryId(''); setFormTaxRate(0); setFormSortOrder(0);
    setFormReceiptName(''); setFormKitchenPrintName('');
    setFormIsKitchenPrint(0); setFormIsLabelPrint(0); setFormExternalId('');
    setFormTagIds([]); setFormIsActive(true);
    setFormSpecs([{ name: '', price: 0, receipt_name: '', is_default: true, is_active: true }]);
    setFormError('');
    setPanel({ type: 'create' });
  };

  const loadBindings = useCallback(async (productId: number) => {
    if (!token) return;
    setBindingLoading(true);
    try {
      const data = await listBindings(token, storeId, 'product', productId);
      setBindings(data);
    } catch (err) { handleError(err); }
    finally { setBindingLoading(false); }
  }, [token, storeId, handleError]);

  const handleBind = async (attributeId: number) => {
    if (!token || panel.type !== 'edit') return;
    try {
      await bindAttribute(token, storeId, {
        owner: { type: 'Product', id: panel.item.source_id },
        attribute_id: attributeId,
      });
      await loadBindings(panel.item.source_id);
    } catch (err) { handleError(err); }
  };

  const handleUnbind = async (bindingId: number) => {
    if (!token) return;
    try {
      await unbindAttribute(token, storeId, bindingId);
      if (panel.type === 'edit') await loadBindings(panel.item.source_id);
    } catch (err) { handleError(err); }
  };

  const openEdit = (prod: StoreProduct) => {
    setFormName(prod.name); setFormImage(prod.image ?? ''); setFormCategoryId(prod.category_source_id);
    setFormTaxRate(prod.tax_rate); setFormSortOrder(prod.sort_order);
    setFormReceiptName(prod.receipt_name ?? ''); setFormKitchenPrintName(prod.kitchen_print_name ?? '');
    setFormIsKitchenPrint(prod.is_kitchen_print_enabled); setFormIsLabelPrint(prod.is_label_print_enabled);
    setFormExternalId(prod.external_id != null ? String(prod.external_id) : '');
    setFormTagIds(prod.tag_ids ?? []); setFormIsActive(prod.is_active);
    setFormSpecs(prod.specs.map(s => ({
      name: s.name, price: s.price, receipt_name: s.receipt_name ?? '',
      is_default: s.is_default, is_active: s.is_active,
    })));
    setFormError('');
    setBindings([]);
    setPanel({ type: 'edit', item: prod });
    loadBindings(prod.source_id);
  };

  const addSpec = () => {
    setFormSpecs([...formSpecs, { name: '', price: 0, receipt_name: '', is_default: false, is_active: true }]);
  };

  const removeSpec = (index: number) => {
    setFormSpecs(formSpecs.filter((_, i) => i !== index));
  };

  const updateSpec = (index: number, field: keyof FormSpec, value: string | number | boolean) => {
    setFormSpecs(formSpecs.map((s, i) => {
      if (i !== index) {
        if (field === 'is_default' && value === true) return { ...s, is_default: false };
        return s;
      }
      return { ...s, [field]: value };
    }));
  };

  const buildSpecInputs = (): ProductSpecInput[] =>
    formSpecs.map((s, i) => ({
      name: s.name.trim(), price: s.price, display_order: i,
      is_default: s.is_default, is_active: s.is_active,
      is_root: formSpecs.length === 1,
      receipt_name: s.receipt_name.trim() || undefined,
    }));

  const handleSave = async () => {
    if (!token || saving) return;
    if (!formName.trim()) { setFormError(t('settings.common.required_field')); return; }
    if (formCategoryId === '') { setFormError(t('settings.common.required_field')); return; }
    if (formSpecs.length === 0) { setFormError(t('settings.product.spec_required')); return; }

    setSaving(true);
    setFormError('');
    try {
      const common = {
        name: formName.trim(), image: formImage || undefined,
        category_id: Number(formCategoryId),
        tax_rate: formTaxRate, sort_order: formSortOrder,
        receipt_name: formReceiptName.trim() || undefined,
        kitchen_print_name: formKitchenPrintName.trim() || undefined,
        is_kitchen_print_enabled: formIsKitchenPrint, is_label_print_enabled: formIsLabelPrint,
        external_id: formExternalId ? Number(formExternalId) : undefined,
        tags: formTagIds.length > 0 ? formTagIds : undefined,
        specs: buildSpecInputs(),
      };

      if (panel.type === 'edit') {
        const payload: ProductUpdate = { ...common, is_active: formIsActive };
        await updateProduct(token, storeId, panel.item.source_id, payload);
      } else if (panel.type === 'create') {
        const payload: ProductCreate = common;
        await createProduct(token, storeId, payload);
      }
      setPanel({ type: 'closed' });
      await load();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally { setSaving(false); }
  };

  const handleDelete = async () => {
    if (!token || panel.type !== 'delete') return;
    setSaving(true);
    try {
      await deleteProduct(token, storeId, panel.item.source_id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const renderItem = (prod: StoreProduct, isSelected: boolean) => (
    <div className={`px-4 py-3.5 flex items-center gap-3 ${isSelected ? 'font-medium' : ''}`}>
      {bulkMode && (
        <input
          type="checkbox"
          checked={bulkSelected.has(prod.source_id)}
          onChange={() => toggleBulkItem(prod.source_id)}
          onClick={(e) => e.stopPropagation()}
          className="w-4 h-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 shrink-0"
        />
      )}
      <Thumbnail hash={prod.image} size={36} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between">
          <span className={`text-sm truncate ${prod.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
            {prod.name}
          </span>
          <span className="text-xs font-medium text-gray-500 tabular-nums shrink-0 ml-2">
            {computePriceDisplay(prod.specs)}
          </span>
        </div>
        {prod.category_name && (
          <p className="text-xs text-gray-400 mt-0.5">{prod.category_name}</p>
        )}
      </div>
    </div>
  );

  const handleReorder = useCallback(async (reordered: StoreProduct[]) => {
    if (!token) return;
    const withOrder = reordered.map((p, i) => ({ ...p, sort_order: i }));
    setProducts(withOrder);
    const items = withOrder.map((p) => ({ id: p.source_id, sort_order: p.sort_order }));
    try { await batchUpdateProductSortOrder(token, storeId, items); }
    catch (err) { handleError(err); await load(); }
  }, [token, storeId, handleError, load]);

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-blue-100 rounded-xl flex items-center justify-center">
          <Package className="w-5 h-5 text-blue-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('settings.product.title')}</h1>
        <div className="ml-auto flex items-center gap-2">
          {bulkMode ? (
            <>
              <span className="text-xs text-gray-500">{bulkSelected.size} {t('common.selection.selected')}</span>
              <button
                onClick={() => setShowBulkConfirm(true)}
                disabled={bulkSelected.size === 0 || bulkDeleting}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-red-500 text-white rounded-lg text-xs font-medium hover:bg-red-600 transition-colors disabled:opacity-40"
              >
                <Trash2 size={12} />
                {t('common.action.batch_delete')}
              </button>
              <button
                onClick={() => { setBulkMode(false); setBulkSelected(new Set()); }}
                className="px-3 py-1.5 text-xs font-medium text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
              >
                {t('common.action.cancel')}
              </button>
            </>
          ) : (
            <button
              onClick={() => { setBulkMode(true); setBulkSelected(new Set()); setPanel({ type: 'closed' }); }}
              className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
              title={t('common.action.batch_delete')}
            >
              <CheckSquare size={16} />
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(p) => p.source_id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={bulkMode ? (p) => toggleBulkItem(p.source_id) : openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('settings.product.unit')}
          onCreateNew={openCreate}
          createLabel={t('common.action.add')}
          isCreating={panel.type === 'create'}
          themeColor="blue"
          loading={loading}
          onReorder={!search.trim() ? handleReorder : undefined}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? `${t('common.action.add')} ${t('settings.product.title')}` : `${t('common.action.edit')} ${t('settings.product.title')}`}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formName.trim() || formCategoryId === '' || formSpecs.length === 0}
            >
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.common.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus placeholder={t('settings.product.name_placeholder')} />
              </FormField>

              <ImageUpload value={formImage} onChange={setFormImage} />

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
                  <input type="number" value={formTaxRate} onChange={e => setFormTaxRate(Number(e.target.value))} className={inputClass} step="0.01" min={0} />
                </FormField>
                <FormField label={t('settings.product.sort_order')}>
                  <input type="number" value={formSortOrder} onChange={e => setFormSortOrder(Number(e.target.value))} className={inputClass} min={0} />
                </FormField>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <FormField label={t('settings.product.receipt_name')}>
                  <input value={formReceiptName} onChange={e => setFormReceiptName(e.target.value)} className={inputClass} />
                </FormField>
                <FormField label={t('settings.product.kitchen_print_name')}>
                  <input value={formKitchenPrintName} onChange={e => setFormKitchenPrintName(e.target.value)} className={inputClass} />
                </FormField>
              </div>

              <div className="flex gap-6">
                <CheckboxField id="prod-kitchen-print" label={t('settings.product.kitchen_print')} checked={formIsKitchenPrint === 1} onChange={(v) => setFormIsKitchenPrint(v ? 1 : 0)} />
                <CheckboxField id="prod-label-print" label={t('settings.product.label_print')} checked={formIsLabelPrint === 1} onChange={(v) => setFormIsLabelPrint(v ? 1 : 0)} />
              </div>

              <FormField label={t('settings.product.external_id')}>
                <input type="number" value={formExternalId} onChange={e => setFormExternalId(e.target.value)} className={inputClass} />
              </FormField>

              {tags.filter(tag => tag.is_active).length > 0 && (
                <FormField label={t('settings.product.tags')}>
                  <TagPicker tags={tags} selectedIds={formTagIds} onToggle={toggleTag} themeColor="blue" />
                </FormField>
              )}

              {panel.type === 'edit' && (
                <CheckboxField id="prod-is-active" label={t('settings.common.active')} checked={formIsActive} onChange={setFormIsActive} />
              )}

              {/* Attribute Bindings (edit mode only) */}
              {panel.type === 'edit' && attributes.length > 0 && (
                <div className="space-y-3">
                  <label className="block text-sm font-medium text-gray-700">{t('settings.attribute.title')}</label>
                  {bindingLoading ? (
                    <div className="text-sm text-gray-400 text-center py-4">{t('common.loading')}</div>
                  ) : (
                    <div className="space-y-2">
                      {attributes.filter(a => a.is_active).map(attr => {
                        const binding = bindings.find(b => b.attribute_source_id === attr.source_id);
                        return (
                          <div key={attr.source_id} className="flex items-center justify-between px-3 py-2.5 bg-gray-50 rounded-xl">
                            <div className="min-w-0">
                              <span className="text-sm text-slate-800">{attr.name}</span>
                              <span className="ml-2 text-xs text-gray-400">
                                {attr.is_multi_select ? t('settings.attribute.multi') : t('settings.attribute.single')}
                                {attr.options.length > 0 && ` · ${attr.options.length} ${t('settings.attribute.options')}`}
                              </span>
                            </div>
                            {binding ? (
                              <button
                                type="button"
                                onClick={() => handleUnbind(binding.source_id)}
                                className="flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-red-600 bg-red-50 rounded-lg hover:bg-red-100 transition-colors"
                              >
                                <Unlink size={12} />
                                {t('common.action.unbind')}
                              </button>
                            ) : (
                              <button
                                type="button"
                                onClick={() => handleBind(attr.source_id)}
                                className="flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-blue-600 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors"
                              >
                                <Link size={12} />
                                {t('common.action.bind')}
                              </button>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}

              {/* Specs */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <label className="block text-sm font-medium text-gray-700">{t('settings.product.specs')}</label>
                  <button type="button" onClick={addSpec} className="inline-flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-blue-700 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors">
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
                        type="text" value={spec.name}
                        onChange={(e) => updateSpec(idx, 'name', e.target.value)}
                        className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                        placeholder={t('settings.product.spec_name')}
                      />
                      <input
                        type="number" value={spec.price}
                        onChange={(e) => updateSpec(idx, 'price', Number(e.target.value))}
                        className="w-28 px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                        placeholder={t('settings.product.price')} step="0.01" min={0}
                      />
                      <button type="button" onClick={() => removeSpec(idx)} className="p-2 text-red-500 hover:bg-red-50 rounded-lg transition-colors">
                        <Trash2 size={14} />
                      </button>
                    </div>
                    <input
                      type="text" value={spec.receipt_name}
                      onChange={(e) => updateSpec(idx, 'receipt_name', e.target.value)}
                      className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                      placeholder={t('settings.product.spec_receipt_name')}
                    />
                    <div className="flex items-center gap-4 px-1">
                      <label className="flex items-center gap-2 text-xs text-gray-600 cursor-pointer">
                        <input type="radio" name="default_spec" checked={spec.is_default} onChange={() => updateSpec(idx, 'is_default', true)} className="text-blue-600 focus:ring-blue-500" />
                        {t('settings.product.is_default')}
                      </label>
                      <CheckboxField id={`spec_active_${idx}`} label={t('settings.common.active')} checked={spec.is_active} onChange={(v) => updateSpec(idx, 'is_active', v)} />
                    </div>
                  </div>
                ))}
              </div>
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.product.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />

      <ConfirmDialog
        isOpen={showBulkConfirm}
        title={t('common.action.batch_delete')}
        description={`${t('common.dialog.confirm_delete')} (${bulkSelected.size})`}
        onConfirm={handleBulkDelete}
        onCancel={() => setShowBulkConfirm(false)}
        variant="danger"
      />
    </div>
  );
};
