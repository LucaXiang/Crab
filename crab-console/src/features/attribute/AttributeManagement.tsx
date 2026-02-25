import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, SlidersHorizontal, Trash2, Edit, Star, Hash, DollarSign, ChefHat, ReceiptText, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import {
  listAttributes, createAttribute, updateAttribute, deleteAttribute,
  createAttributeOption, updateAttributeOption, deleteAttributeOption,
} from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField';
import { formatCurrency } from '@/utils/format';
import type {
  StoreAttribute, StoreAttributeOption,
  AttributeCreate, AttributeUpdate,
  AttributeOptionCreate, AttributeOptionUpdate,
} from '@/core/types/store';

interface OptionFormState {
  name: string;
  price_modifier: string;
  receipt_name: string;
  kitchen_print_name: string;
  enable_quantity: boolean;
  max_quantity: string;
}

const emptyOptionForm: OptionFormState = {
  name: '', price_modifier: '0', receipt_name: '',
  kitchen_print_name: '', enable_quantity: false, max_quantity: '',
};

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: StoreAttribute }
  | { type: 'delete'; item: StoreAttribute };

export const AttributeManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [attributes, setAttributes] = useState<StoreAttribute[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Attribute form
  const [formName, setFormName] = useState('');
  const [formIsMultiSelect, setFormIsMultiSelect] = useState(false);
  const [formMaxSelections, setFormMaxSelections] = useState<number | ''>('');

  // Option sub-CRUD
  const [optEditing, setOptEditing] = useState<StoreAttributeOption | null>(null);
  const [optCreating, setOptCreating] = useState(false);
  const [optForm, setOptForm] = useState<OptionFormState>(emptyOptionForm);
  const [optSaving, setOptSaving] = useState(false);
  const [optFormError, setOptFormError] = useState('');

  // Delete option
  const [deleteOptTarget, setDeleteOptTarget] = useState<{ attr: StoreAttribute; opt: StoreAttributeOption } | null>(null);

  const handleError = useCallback((err: unknown) => {
    alert(err instanceof ApiError ? err.message : t('auth.error_generic'));
  }, [t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      const data = await listAttributes(token, storeId);
      setAttributes(data);
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    let list = [...attributes].sort((a, b) => a.display_order - b.display_order);
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(a => a.name.toLowerCase().includes(q));
    }
    return list;
  }, [attributes, search]);

  const selectedId = panel.type === 'edit' ? panel.item.source_id : null;

  // Keep edit panel synced with latest data
  const selectedAttr = useMemo(() => {
    if (panel.type !== 'edit') return null;
    return attributes.find(a => a.source_id === panel.item.source_id) ?? null;
  }, [attributes, panel]);

  const selectedOptions = useMemo(() => {
    if (!selectedAttr) return [];
    return [...selectedAttr.options].sort((a, b) => a.display_order - b.display_order);
  }, [selectedAttr]);

  const getFeatures = (attr: StoreAttribute) => ({
    hasPrice: attr.options.some(o => o.price_modifier !== 0),
    hasQuantity: attr.options.some(o => o.enable_quantity),
  });

  const closeOptionForm = () => { setOptEditing(null); setOptCreating(false); setOptFormError(''); };

  const openCreate = () => {
    setFormName(''); setFormIsMultiSelect(false); setFormMaxSelections('');
    setFormError(''); closeOptionForm();
    setPanel({ type: 'create' });
  };

  const openEdit = (attr: StoreAttribute) => {
    setFormName(attr.name); setFormIsMultiSelect(attr.is_multi_select);
    setFormMaxSelections(attr.max_selections ?? '');
    setFormError(''); closeOptionForm();
    setPanel({ type: 'edit', item: attr });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    if (!formName.trim()) { setFormError(t('settings.common.required_field')); return; }

    setSaving(true); setFormError('');
    try {
      if (panel.type === 'edit') {
        const payload: AttributeUpdate = {
          name: formName.trim(), is_multi_select: formIsMultiSelect,
          max_selections: formMaxSelections !== '' ? Number(formMaxSelections) : undefined,
        };
        await updateAttribute(token, storeId, panel.item.source_id, payload);
      } else if (panel.type === 'create') {
        const payload: AttributeCreate = {
          name: formName.trim(), is_multi_select: formIsMultiSelect,
          max_selections: formMaxSelections !== '' ? Number(formMaxSelections) : undefined,
        };
        await createAttribute(token, storeId, payload);
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
      await deleteAttribute(token, storeId, panel.item.source_id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  // ── Option CRUD ──

  const openCreateOpt = () => {
    setOptEditing(null); setOptCreating(true);
    setOptForm(emptyOptionForm); setOptFormError('');
  };

  const openEditOpt = (opt: StoreAttributeOption) => {
    setOptCreating(false);
    setOptEditing(opt);
    setOptForm({
      name: opt.name, price_modifier: String(opt.price_modifier),
      receipt_name: opt.receipt_name ?? '', kitchen_print_name: opt.kitchen_print_name ?? '',
      enable_quantity: opt.enable_quantity,
      max_quantity: opt.max_quantity != null ? String(opt.max_quantity) : '',
    });
    setOptFormError('');
  };

  const handleSaveOpt = async () => {
    if (!token || !selectedAttr) return;
    if (!optForm.name.trim()) { setOptFormError(t('settings.common.required_field')); return; }

    setOptSaving(true); setOptFormError('');
    try {
      const priceMod = parseFloat(optForm.price_modifier) || 0;
      if (optEditing) {
        const payload: AttributeOptionUpdate = {
          name: optForm.name.trim(), price_modifier: priceMod,
          receipt_name: optForm.receipt_name.trim() || undefined,
          kitchen_print_name: optForm.kitchen_print_name.trim() || undefined,
          enable_quantity: optForm.enable_quantity,
          max_quantity: optForm.enable_quantity && optForm.max_quantity ? Number(optForm.max_quantity) : undefined,
        };
        await updateAttributeOption(token, storeId, selectedAttr.source_id, optEditing.source_id, payload);
      } else {
        const payload: AttributeOptionCreate = {
          name: optForm.name.trim(), price_modifier: priceMod,
          receipt_name: optForm.receipt_name.trim() || undefined,
          kitchen_print_name: optForm.kitchen_print_name.trim() || undefined,
          enable_quantity: optForm.enable_quantity,
          max_quantity: optForm.enable_quantity && optForm.max_quantity ? Number(optForm.max_quantity) : undefined,
        };
        await createAttributeOption(token, storeId, selectedAttr.source_id, payload);
      }
      closeOptionForm();
      await load();
    } catch (err) {
      setOptFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally { setOptSaving(false); }
  };

  const handleDeleteOpt = async () => {
    if (!token || !deleteOptTarget) return;
    try {
      await deleteAttributeOption(token, storeId, deleteOptTarget.attr.source_id, deleteOptTarget.opt.source_id);
      setDeleteOptTarget(null);
      await load();
    } catch (err) { handleError(err); setDeleteOptTarget(null); }
  };

  const handleToggleDefault = async (optionId: number) => {
    if (!token || !selectedAttr) return;
    const current = selectedAttr.default_option_ids ?? [];
    const isDefault = current.includes(optionId);
    let newDefaults: number[];
    if (selectedAttr.is_multi_select) {
      if (isDefault) {
        newDefaults = current.filter(id => id !== optionId);
      } else {
        if (selectedAttr.max_selections && current.length >= selectedAttr.max_selections) return;
        newDefaults = [...current, optionId];
      }
    } else {
      newDefaults = isDefault ? [] : [optionId];
    }
    try {
      await updateAttribute(token, storeId, selectedAttr.source_id, { default_option_ids: newDefaults });
      await load();
    } catch (err) { handleError(err); }
  };

  const renderItem = (attr: StoreAttribute, isSelected: boolean) => {
    const { hasPrice, hasQuantity } = getFeatures(attr);
    return (
      <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
        <div className="flex items-center justify-between gap-2">
          <span className="text-sm truncate text-slate-900">{attr.name}</span>
          <div className="flex items-center gap-1 shrink-0">
            {hasQuantity && <Hash size={12} className="text-purple-500" />}
            {hasPrice && <DollarSign size={12} className="text-orange-500" />}
            {attr.show_on_kitchen_print && <ChefHat size={12} className="text-purple-400" />}
            {attr.show_on_receipt && <ReceiptText size={12} className="text-blue-400" />}
          </div>
        </div>
        <div className="flex items-center gap-2 mt-1 text-xs text-gray-400">
          <span className={`px-1.5 py-0.5 rounded ${isSelected ? 'bg-purple-100 text-purple-700' : 'bg-gray-100 text-gray-600'}`}>
            {attr.is_multi_select ? t('settings.attribute.multi') : t('settings.attribute.single')}
          </span>
          <span>· {attr.options.length} {t('settings.attribute.options')}</span>
        </div>
      </div>
    );
  };

  // ── Option inline form (shown inside detail panel) ──
  const optionFormSection = (optCreating || optEditing) && (
    <div className="bg-purple-50/50 border border-purple-200 rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-bold text-purple-900">
          {optEditing ? t('settings.attribute.edit_option') : t('settings.attribute.add_option')}
        </h4>
        <button onClick={closeOptionForm} className="p-1 rounded-lg hover:bg-purple-100 transition-colors">
          <X size={16} className="text-purple-400" />
        </button>
      </div>

      {optFormError && (
        <div className="p-2 bg-red-50 border border-red-200 rounded-lg text-xs text-red-600">{optFormError}</div>
      )}

      <FormField label={t('settings.common.name')} required>
        <input value={optForm.name} onChange={e => setOptForm(prev => ({ ...prev, name: e.target.value }))} className={inputClass} autoFocus />
      </FormField>

      <FormField label={t('settings.attribute.price')}>
        <div className="relative">
          <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm">€</span>
          <input
            type="text" inputMode="decimal" value={optForm.price_modifier}
            onChange={e => setOptForm(prev => ({ ...prev, price_modifier: e.target.value }))}
            className={`${inputClass} pl-7`} placeholder="0.00"
          />
        </div>
      </FormField>

      <div className="grid grid-cols-2 gap-3">
        <FormField label={t('settings.attribute.receipt_name')}>
          <input value={optForm.receipt_name} onChange={e => setOptForm(prev => ({ ...prev, receipt_name: e.target.value }))} className={inputClass} />
        </FormField>
        <FormField label={t('settings.attribute.kitchen_print_name')}>
          <input value={optForm.kitchen_print_name} onChange={e => setOptForm(prev => ({ ...prev, kitchen_print_name: e.target.value }))} className={inputClass} />
        </FormField>
      </div>

      <CheckboxField
        id="opt-enable-quantity" label={t('settings.attribute.enable_quantity')}
        description={t('settings.attribute.enable_quantity_desc')}
        checked={optForm.enable_quantity} onChange={v => setOptForm(prev => ({ ...prev, enable_quantity: v }))}
      />

      {optForm.enable_quantity && (
        <FormField label={t('settings.attribute.max_quantity')}>
          <input type="number" min={1} max={99} value={optForm.max_quantity}
            onChange={e => setOptForm(prev => ({ ...prev, max_quantity: e.target.value }))}
            className={inputClass} placeholder="99"
          />
        </FormField>
      )}

      <div className="flex justify-end gap-2 pt-1">
        <button onClick={closeOptionForm} className="px-3 py-1.5 text-sm text-gray-500 hover:bg-gray-100 rounded-lg transition-colors">
          {t('common.action.cancel')}
        </button>
        <button onClick={handleSaveOpt} disabled={optSaving || !optForm.name.trim()}
          className="px-3 py-1.5 text-sm font-medium text-white bg-purple-500 hover:bg-purple-600 rounded-lg transition-colors disabled:opacity-50">
          {optSaving ? t('catalog.saving') : t('common.action.save')}
        </button>
      </div>
    </div>
  );

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-purple-100 rounded-xl flex items-center justify-center">
          <SlidersHorizontal className="w-5 h-5 text-purple-600" />
        </div>
        <div>
          <h1 className="text-xl font-bold text-slate-900">{t('settings.attribute.title')}</h1>
          <p className="text-xs text-gray-400">{t('settings.attribute.subtitle')}</p>
        </div>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(a) => a.source_id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('settings.attribute.title')}
          onCreateNew={openCreate}
          createLabel={t('common.action.add')}
          isCreating={panel.type === 'create'}
          themeColor="purple"
          loading={loading}
        >
          {panel.type === 'create' && (
            <DetailPanel
              title={`${t('common.action.add')} ${t('settings.attribute.title')}`}
              isCreating
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              saving={saving}
              saveDisabled={!formName.trim()}
            >
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}
              <FormField label={t('settings.common.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus placeholder={t('settings.attribute.name_placeholder')} />
              </FormField>
              <CheckboxField id="attr-multi-select" label={t('settings.attribute.multi_select')} description={t('settings.attribute.multi_select_desc')} checked={formIsMultiSelect} onChange={setFormIsMultiSelect} />
              {formIsMultiSelect && (
                <FormField label={t('settings.attribute.max_selections')}>
                  <input type="number" value={formMaxSelections} onChange={e => setFormMaxSelections(e.target.value === '' ? '' : Number(e.target.value))} className={inputClass} placeholder={t('settings.attribute.max_selections_placeholder')} min={1} />
                </FormField>
              )}
            </DetailPanel>
          )}

          {panel.type === 'edit' && selectedAttr && (
            <DetailPanel
              title={`${t('common.action.edit')} ${t('settings.attribute.title')}`}
              isCreating={false}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={() => setPanel({ type: 'delete', item: selectedAttr })}
              saving={saving}
              saveDisabled={!formName.trim()}
            >
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.common.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} />
              </FormField>
              <CheckboxField id="attr-multi-select" label={t('settings.attribute.multi_select')} description={t('settings.attribute.multi_select_desc')} checked={formIsMultiSelect} onChange={setFormIsMultiSelect} />
              {formIsMultiSelect && (
                <FormField label={t('settings.attribute.max_selections')}>
                  <input type="number" value={formMaxSelections} onChange={e => setFormMaxSelections(e.target.value === '' ? '' : Number(e.target.value))} className={inputClass} min={1} />
                </FormField>
              )}

              {/* ── Options section ── */}
              <div className="border-t border-gray-200 pt-4 mt-2 space-y-3">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-bold text-gray-700">{t('settings.attribute.options')} ({selectedOptions.length})</h3>
                  {!optCreating && !optEditing && (
                    <button onClick={openCreateOpt} className="inline-flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-purple-700 bg-purple-50 rounded-lg hover:bg-purple-100 transition-colors">
                      <Plus size={12} /> {t('settings.attribute.add_option')}
                    </button>
                  )}
                </div>

                {/* Option inline form */}
                {optionFormSection}

                {/* Options list */}
                {selectedOptions.length === 0 && !optCreating ? (
                  <div className="text-center py-6 border-2 border-dashed border-gray-200 rounded-xl text-gray-400">
                    <p className="text-sm">{t('settings.attribute.no_options')}</p>
                    <button onClick={openCreateOpt} className="mt-1 text-purple-600 hover:text-purple-700 text-sm font-medium hover:underline">
                      {t('settings.attribute.add_first_hint')}
                    </button>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {selectedOptions.map(option => {
                      const isDefault = selectedAttr.default_option_ids?.includes(option.source_id) ?? false;
                      const hasPriceMod = option.price_modifier !== 0;
                      return (
                        <div key={option.source_id}
                          className={`p-3 rounded-lg border transition-colors group ${isDefault ? 'bg-amber-50/50 border-amber-200' : 'bg-white border-gray-200 hover:border-gray-300'}`}
                        >
                          <div className="flex items-center gap-2">
                            <button onClick={() => handleToggleDefault(option.source_id)}
                              className={`shrink-0 p-1 rounded transition-colors ${isDefault ? 'text-amber-500 hover:text-amber-600' : 'text-gray-300 hover:text-amber-400'}`}
                              title={isDefault ? t('settings.attribute.unset_default') : t('settings.attribute.set_default')}
                            >
                              <Star size={16} fill={isDefault ? 'currentColor' : 'none'} />
                            </button>
                            <span className={`font-medium ${isDefault ? 'text-gray-900' : 'text-gray-800'}`}>{option.name}</span>
                            <div className="flex-1" />
                            {hasPriceMod && (
                              <span className={`text-sm font-semibold px-2 py-0.5 rounded ${option.price_modifier > 0 ? 'bg-orange-50 text-orange-600 border border-orange-100' : 'bg-green-50 text-green-600 border border-green-100'}`}>
                                {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                              </span>
                            )}
                            <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                              <button onClick={() => openEditOpt(option)} className="p-1.5 text-gray-400 hover:text-purple-600 hover:bg-purple-50 rounded-md transition-colors">
                                <Edit size={14} />
                              </button>
                              <button onClick={() => setDeleteOptTarget({ attr: selectedAttr, opt: option })} className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-md transition-colors">
                                <Trash2 size={14} />
                              </button>
                            </div>
                          </div>
                          {option.enable_quantity && (
                            <div className="mt-2 flex items-center gap-2 text-xs text-purple-600">
                              <Hash size={12} />
                              <span>{t('settings.attribute.quantity_range').replace('{max}', String(option.max_quantity ?? 99))}</span>
                            </div>
                          )}
                          {(option.receipt_name || option.kitchen_print_name) && (
                            <div className="mt-2 flex items-center gap-4 text-xs text-gray-500">
                              {option.receipt_name && <span className="flex items-center gap-1 text-blue-500"><ReceiptText size={11} />{option.receipt_name}</span>}
                              {option.kitchen_print_name && <span className="flex items-center gap-1 text-purple-500"><ChefHat size={11} />{option.kitchen_print_name}</span>}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.attribute.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />

      <ConfirmDialog
        isOpen={!!deleteOptTarget}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.attribute.confirm.delete_option')}
        onConfirm={handleDeleteOpt}
        onCancel={() => setDeleteOptTarget(null)}
        variant="danger"
      />
    </div>
  );
};
