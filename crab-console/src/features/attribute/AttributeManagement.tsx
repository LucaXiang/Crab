import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, SlidersHorizontal, X, Trash2, Edit, Search, List, Star, Hash, DollarSign, ChefHat, ReceiptText } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import {
  listAttributes, createAttribute, updateAttribute, deleteAttribute,
  createAttributeOption, updateAttributeOption, deleteAttributeOption,
} from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField/FormField';
import type {
  StoreAttribute, StoreAttributeOption,
  AttributeCreate, AttributeUpdate,
  AttributeOptionCreate, AttributeOptionUpdate,
} from '@/core/types/store';
import { formatCurrency } from '@/utils/format';

// ── Option Modal ──

interface OptionFormState {
  name: string;
  price_modifier: string;
  receipt_name: string;
  kitchen_print_name: string;
  enable_quantity: boolean;
  max_quantity: string;
}

const emptyOptionForm: OptionFormState = {
  name: '',
  price_modifier: '0',
  receipt_name: '',
  kitchen_print_name: '',
  enable_quantity: false,
  max_quantity: '',
};

// ── Main Component ──

export const AttributeManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [attributes, setAttributes] = useState<StoreAttribute[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');

  // Selected attribute (master-detail)
  const [selectedId, setSelectedId] = useState<number | null>(null);

  // Attribute modal state
  const [attrModalOpen, setAttrModalOpen] = useState(false);
  const [editingAttr, setEditingAttr] = useState<StoreAttribute | null>(null);
  const [attrSaving, setAttrSaving] = useState(false);
  const [attrFormError, setAttrFormError] = useState('');
  const [formName, setFormName] = useState('');
  const [formIsMultiSelect, setFormIsMultiSelect] = useState(false);
  const [formMaxSelections, setFormMaxSelections] = useState<number | ''>('');

  // Option modal state
  const [optModalOpen, setOptModalOpen] = useState(false);
  const [editingOpt, setEditingOpt] = useState<StoreAttributeOption | null>(null);
  const [optSaving, setOptSaving] = useState(false);
  const [optFormError, setOptFormError] = useState('');
  const [optForm, setOptForm] = useState<OptionFormState>(emptyOptionForm);

  // Delete confirmations
  const [deleteAttrTarget, setDeleteAttrTarget] = useState<StoreAttribute | null>(null);
  const [deleteOptTarget, setDeleteOptTarget] = useState<{ attr: StoreAttribute; opt: StoreAttributeOption } | null>(null);

  const loadData = useCallback(async () => {
    if (!token) return;
    try {
      setLoading(true);
      const data = await listAttributes(token, storeId);
      setAttributes(data);
      setError('');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, t]);

  useEffect(() => { loadData(); }, [loadData]);

  const filtered = useMemo(() => {
    let list = [...attributes].sort((a, b) => a.display_order - b.display_order);
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(a => a.name.toLowerCase().includes(q));
    }
    return list;
  }, [attributes, searchQuery]);

  const selectedAttr = useMemo(() => {
    if (!selectedId) return null;
    return attributes.find(a => a.source_id === selectedId) ?? null;
  }, [attributes, selectedId]);

  const selectedOptions = useMemo(() => {
    if (!selectedAttr) return [];
    return [...selectedAttr.options].sort((a, b) => a.display_order - b.display_order);
  }, [selectedAttr]);

  // Auto-select first attribute
  useEffect(() => {
    if (!selectedId && filtered.length > 0) {
      setSelectedId(filtered[0].source_id);
    }
  }, [filtered, selectedId]);

  // Clear selection if deleted
  useEffect(() => {
    if (selectedId && !attributes.find(a => a.source_id === selectedId)) {
      setSelectedId(filtered.length > 0 ? filtered[0].source_id : null);
    }
  }, [attributes, filtered, selectedId]);

  // ── Attribute modal handlers ──

  const openCreateAttr = () => {
    setEditingAttr(null);
    setFormName('');
    setFormIsMultiSelect(false);
    setFormMaxSelections('');
    setAttrFormError('');
    setAttrModalOpen(true);
  };

  const openEditAttr = (attr: StoreAttribute) => {
    setEditingAttr(attr);
    setFormName(attr.name);
    setFormIsMultiSelect(attr.is_multi_select);
    setFormMaxSelections(attr.max_selections ?? '');
    setAttrFormError('');
    setAttrModalOpen(true);
  };

  const handleSaveAttr = async () => {
    if (!token) return;
    if (!formName.trim()) { setAttrFormError(t('settings.common.required_field')); return; }

    setAttrSaving(true);
    setAttrFormError('');
    try {
      if (editingAttr) {
        const payload: AttributeUpdate = {
          name: formName.trim(),
          is_multi_select: formIsMultiSelect,
          max_selections: formMaxSelections !== '' ? Number(formMaxSelections) : undefined,
        };
        await updateAttribute(token, storeId, editingAttr.source_id, payload);
      } else {
        const payload: AttributeCreate = {
          name: formName.trim(),
          is_multi_select: formIsMultiSelect,
          max_selections: formMaxSelections !== '' ? Number(formMaxSelections) : undefined,
        };
        await createAttribute(token, storeId, payload);
      }
      setAttrModalOpen(false);
      await loadData();
    } catch (err) {
      setAttrFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setAttrSaving(false);
    }
  };

  const handleDeleteAttr = async () => {
    if (!token || !deleteAttrTarget) return;
    try {
      await deleteAttribute(token, storeId, deleteAttrTarget.source_id);
      setDeleteAttrTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteAttrTarget(null);
    }
  };

  // ── Option modal handlers ──

  const openCreateOpt = () => {
    setEditingOpt(null);
    setOptForm(emptyOptionForm);
    setOptFormError('');
    setOptModalOpen(true);
  };

  const openEditOpt = (opt: StoreAttributeOption) => {
    setEditingOpt(opt);
    setOptForm({
      name: opt.name,
      price_modifier: String(opt.price_modifier),
      receipt_name: opt.receipt_name ?? '',
      kitchen_print_name: opt.kitchen_print_name ?? '',
      enable_quantity: opt.enable_quantity,
      max_quantity: opt.max_quantity != null ? String(opt.max_quantity) : '',
    });
    setOptFormError('');
    setOptModalOpen(true);
  };

  const handleSaveOpt = async () => {
    if (!token || !selectedAttr) return;
    if (!optForm.name.trim()) { setOptFormError(t('settings.common.required_field')); return; }

    setOptSaving(true);
    setOptFormError('');
    try {
      const priceMod = parseFloat(optForm.price_modifier) || 0;
      if (editingOpt) {
        const payload: AttributeOptionUpdate = {
          name: optForm.name.trim(),
          price_modifier: priceMod,
          receipt_name: optForm.receipt_name.trim() || undefined,
          kitchen_print_name: optForm.kitchen_print_name.trim() || undefined,
          enable_quantity: optForm.enable_quantity,
          max_quantity: optForm.enable_quantity && optForm.max_quantity ? Number(optForm.max_quantity) : undefined,
        };
        await updateAttributeOption(token, storeId, selectedAttr.source_id, editingOpt.source_id, payload);
      } else {
        const payload: AttributeOptionCreate = {
          name: optForm.name.trim(),
          price_modifier: priceMod,
          receipt_name: optForm.receipt_name.trim() || undefined,
          kitchen_print_name: optForm.kitchen_print_name.trim() || undefined,
          enable_quantity: optForm.enable_quantity,
          max_quantity: optForm.enable_quantity && optForm.max_quantity ? Number(optForm.max_quantity) : undefined,
        };
        await createAttributeOption(token, storeId, selectedAttr.source_id, payload);
      }
      setOptModalOpen(false);
      await loadData();
    } catch (err) {
      setOptFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setOptSaving(false);
    }
  };

  const handleDeleteOpt = async () => {
    if (!token || !deleteOptTarget) return;
    try {
      await deleteAttributeOption(token, storeId, deleteOptTarget.attr.source_id, deleteOptTarget.opt.source_id);
      setDeleteOptTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteOptTarget(null);
    }
  };

  // ── Default option toggle ──

  const handleToggleDefault = async (optionId: number) => {
    if (!token || !selectedAttr) return;
    const current = selectedAttr.default_option_ids ?? [];
    const isDefault = current.includes(optionId);

    let newDefaults: number[];
    if (selectedAttr.is_multi_select) {
      if (isDefault) {
        newDefaults = current.filter(id => id !== optionId);
      } else {
        if (selectedAttr.max_selections && current.length >= selectedAttr.max_selections) {
          setError(t('settings.attribute.max_defaults_reached'));
          return;
        }
        newDefaults = [...current, optionId];
      }
    } else {
      newDefaults = isDefault ? [] : [optionId];
    }

    try {
      await updateAttribute(token, storeId, selectedAttr.source_id, {
        default_option_ids: newDefaults,
      });
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    }
  };

  // ── Feature indicators ──

  const getFeatures = (attr: StoreAttribute) => ({
    hasPrice: attr.options.some(o => o.price_modifier !== 0),
    hasQuantity: attr.options.some(o => o.enable_quantity),
  });

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-purple-100 rounded-xl flex items-center justify-center">
            <SlidersHorizontal size={20} className="text-purple-600" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('settings.attribute.title')}</h2>
            <p className="text-sm text-gray-500">{t('settings.attribute.subtitle')}</p>
          </div>
        </div>
        <button
          onClick={openCreateAttr}
          className="inline-flex items-center gap-2 px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-medium hover:bg-purple-700 transition-colors shadow-sm"
        >
          <Plus size={16} />
          {t('common.action.add')}
        </button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{error}</div>
      )}

      {/* Master-Detail Panel */}
      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-sm flex" style={{ minHeight: '28rem' }}>
        {loading && attributes.length === 0 ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-gray-400 text-sm text-center flex flex-col items-center gap-3">
              <div className="w-8 h-8 border-4 border-gray-200 border-t-purple-500 rounded-full animate-spin" />
              <span>{t('auth.loading')}</span>
            </div>
          </div>
        ) : attributes.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center py-16 text-center">
            <div className="w-16 h-16 bg-gray-50 rounded-full flex items-center justify-center mb-4">
              <SlidersHorizontal className="text-gray-300" size={32} />
            </div>
            <p className="text-gray-500 font-medium">{t('settings.attribute.no_options')}</p>
          </div>
        ) : (
          <>
            {/* Left Panel - Attribute List */}
            <div className="w-72 border-r border-gray-100 bg-gray-50/50 flex flex-col shrink-0">
              <div className="p-3 border-b border-gray-100">
                <div className="relative">
                  <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder={t('settings.common.search')}
                    className="w-full pl-9 pr-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-purple-500/20 focus:border-purple-400"
                  />
                </div>
              </div>

              <div className="flex-1 overflow-y-auto">
                <div className="py-1">
                  {filtered.map((attr) => {
                    const isSelected = selectedId === attr.source_id;
                    const { hasPrice, hasQuantity } = getFeatures(attr);
                    return (
                      <div
                        key={attr.source_id}
                        onClick={() => setSelectedId(attr.source_id)}
                        className={`mx-2 my-1 px-3 py-2.5 rounded-lg cursor-pointer transition-all ${
                          isSelected
                            ? 'bg-purple-50 border-l-[3px] border-l-purple-500 shadow-sm'
                            : 'hover:bg-white border-l-[3px] border-l-transparent'
                        }`}
                      >
                        <div className="flex items-center justify-between gap-2">
                          <h3 className={`font-medium text-sm truncate ${isSelected ? 'text-purple-900' : 'text-gray-800'}`}>
                            {attr.name}
                          </h3>
                          <div className="flex items-center gap-1 shrink-0">
                            {hasQuantity && <Hash size={12} className="text-purple-500" />}
                            {hasPrice && <DollarSign size={12} className="text-orange-500" />}
                            {attr.show_on_kitchen_print && <ChefHat size={12} className="text-purple-400" />}
                            {attr.show_on_receipt && <ReceiptText size={12} className="text-blue-400" />}
                          </div>
                        </div>
                        <div className="flex items-center gap-2 mt-1 text-xs text-gray-500">
                          <span className={`px-1.5 py-0.5 rounded ${isSelected ? 'bg-purple-100 text-purple-700' : 'bg-gray-100 text-gray-600'}`}>
                            {attr.is_multi_select ? t('settings.attribute.multi') : t('settings.attribute.single')}
                          </span>
                          <span>· {attr.options.length} {t('settings.attribute.options')}</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>

            {/* Right Panel - Options Detail */}
            <div className="flex-1 flex flex-col min-w-0">
              {!selectedAttr ? (
                <div className="flex-1 flex flex-col items-center justify-center text-gray-400">
                  <List size={32} className="mb-2 text-gray-300" />
                  <p className="text-sm">{t('settings.attribute.select_attribute')}</p>
                </div>
              ) : (
                <>
                  {/* Attribute Header */}
                  <div className="p-4 border-b border-gray-100">
                    <div className="flex items-center justify-between">
                      <div>
                        <h2 className="text-lg font-semibold text-gray-900">{selectedAttr.name}</h2>
                        <div className="flex items-center gap-3 mt-1 text-sm text-gray-500">
                          <span className="px-2 py-0.5 bg-purple-50 text-purple-700 rounded font-medium text-xs">
                            {selectedAttr.is_multi_select ? t('settings.attribute.multi') : t('settings.attribute.single')}
                          </span>
                          {selectedAttr.show_on_receipt && (
                            <span className="flex items-center gap-1 text-blue-600">
                              <ReceiptText size={12} />
                              {t('settings.attribute.show_on_receipt')}
                            </span>
                          )}
                          {selectedAttr.show_on_kitchen_print && (
                            <span className="flex items-center gap-1 text-purple-600">
                              <ChefHat size={12} />
                              {t('settings.attribute.show_on_kitchen_print')}
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => openEditAttr(selectedAttr)}
                          className="px-3 py-1.5 text-sm text-gray-600 hover:text-purple-600 hover:bg-purple-50 rounded-lg transition-colors flex items-center gap-1"
                        >
                          <Edit size={14} />
                          {t('common.action.edit')}
                        </button>
                        <button
                          onClick={() => setDeleteAttrTarget(selectedAttr)}
                          className="px-3 py-1.5 text-sm text-gray-600 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors flex items-center gap-1"
                        >
                          <Trash2 size={14} />
                          {t('common.action.delete')}
                        </button>
                      </div>
                    </div>
                  </div>

                  {/* Options List Header */}
                  <div className="px-4 py-3 border-b border-gray-50 flex items-center justify-between bg-gray-50/30">
                    <h3 className="font-medium text-gray-700 text-sm">{t('settings.attribute.options')}</h3>
                    <button
                      onClick={openCreateOpt}
                      className="px-3 py-1.5 text-sm bg-purple-500 text-white rounded-lg hover:bg-purple-600 transition-colors flex items-center gap-1"
                    >
                      <Plus size={14} />
                      {t('settings.attribute.add_option')}
                    </button>
                  </div>

                  {/* Options List */}
                  <div className="flex-1 overflow-y-auto p-4">
                    {selectedOptions.length === 0 ? (
                      <div className="h-full flex flex-col items-center justify-center border-2 border-dashed border-gray-200 rounded-xl text-gray-400 min-h-[12rem]">
                        <List size={24} className="mb-2 text-gray-300" />
                        <p className="text-sm">{t('settings.attribute.no_options')}</p>
                        <button
                          onClick={openCreateOpt}
                          className="mt-2 text-purple-600 hover:text-purple-700 text-sm font-medium hover:underline"
                        >
                          {t('settings.attribute.add_first_hint')}
                        </button>
                      </div>
                    ) : (
                      <div className="space-y-2">
                        {selectedOptions.map((option) => {
                          const isDefault = selectedAttr.default_option_ids?.includes(option.source_id) ?? false;
                          const hasPriceMod = option.price_modifier !== 0;

                          return (
                            <div
                              key={option.source_id}
                              className={`p-3 rounded-lg border transition-colors group ${
                                isDefault ? 'bg-amber-50/50 border-amber-200' : 'bg-white border-gray-200 hover:border-gray-300'
                              }`}
                            >
                              {/* Row 1: Name + Price + Actions */}
                              <div className="flex items-center gap-2">
                                <button
                                  onClick={() => handleToggleDefault(option.source_id)}
                                  className={`shrink-0 p-1 rounded transition-colors ${
                                    isDefault ? 'text-amber-500 hover:text-amber-600' : 'text-gray-300 hover:text-amber-400'
                                  }`}
                                  title={isDefault ? t('settings.attribute.unset_default') : t('settings.attribute.set_default')}
                                >
                                  <Star size={16} fill={isDefault ? 'currentColor' : 'none'} />
                                </button>

                                <span className={`font-medium ${isDefault ? 'text-gray-900' : 'text-gray-800'}`}>
                                  {option.name}
                                </span>

                                <div className="flex-1" />

                                {hasPriceMod && (
                                  <span className={`text-sm font-semibold px-2 py-0.5 rounded ${
                                    option.price_modifier > 0
                                      ? 'bg-orange-50 text-orange-600 border border-orange-100'
                                      : 'bg-green-50 text-green-600 border border-green-100'
                                  }`}>
                                    {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                                  </span>
                                )}

                                <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                  <button
                                    onClick={() => openEditOpt(option)}
                                    className="p-1.5 text-gray-400 hover:text-purple-600 hover:bg-purple-50 rounded-md transition-colors"
                                  >
                                    <Edit size={14} />
                                  </button>
                                  <button
                                    onClick={() => setDeleteOptTarget({ attr: selectedAttr, opt: option })}
                                    className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-md transition-colors"
                                  >
                                    <Trash2 size={14} />
                                  </button>
                                </div>
                              </div>

                              {/* Row 2: Quantity control */}
                              {option.enable_quantity && (
                                <div className="mt-2 flex items-center gap-2 text-xs text-purple-600">
                                  <Hash size={12} />
                                  <span>{t('settings.attribute.quantity_range').replace('{max}', String(option.max_quantity ?? 99))}</span>
                                </div>
                              )}

                              {/* Row 3: Receipt/Kitchen names */}
                              {(option.receipt_name || option.kitchen_print_name) && (
                                <div className="mt-2 flex items-center gap-4 text-xs text-gray-500">
                                  {option.receipt_name && (
                                    <span className="flex items-center gap-1 text-blue-500">
                                      <ReceiptText size={11} />
                                      {option.receipt_name}
                                    </span>
                                  )}
                                  {option.kitchen_print_name && (
                                    <span className="flex items-center gap-1 text-purple-500">
                                      <ChefHat size={11} />
                                      {option.kitchen_print_name}
                                    </span>
                                  )}
                                </div>
                              )}
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          </>
        )}
      </div>

      {/* ── Attribute Modal ── */}
      {attrModalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm"
          onClick={(e) => { if (e.target === e.currentTarget) setAttrModalOpen(false); }}
        >
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md overflow-hidden" style={{ animation: 'slideUp 0.25s ease-out' }}>
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100">
              <h3 className="text-lg font-bold text-gray-900">
                {editingAttr ? t('common.action.edit') : t('common.action.add')} {t('settings.attribute.title')}
              </h3>
              <button onClick={() => setAttrModalOpen(false)} className="p-1 hover:bg-gray-100 rounded-lg transition-colors">
                <X size={20} className="text-gray-400" />
              </button>
            </div>

            <div className="px-6 py-5 space-y-4">
              {attrFormError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{attrFormError}</div>
              )}

              <FormField label={t('settings.common.name')} required>
                <input
                  type="text"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.attribute.name_placeholder')}
                />
              </FormField>

              <CheckboxField
                id="attr-is-multi-select"
                label={t('settings.attribute.multi_select')}
                description={t('settings.attribute.multi_select_desc')}
                checked={formIsMultiSelect}
                onChange={setFormIsMultiSelect}
              />

              {formIsMultiSelect && (
                <FormField label={t('settings.attribute.max_selections')}>
                  <input
                    type="number"
                    value={formMaxSelections}
                    onChange={(e) => setFormMaxSelections(e.target.value === '' ? '' : Number(e.target.value))}
                    className={inputClass}
                    placeholder={t('settings.attribute.max_selections_placeholder')}
                    min={1}
                  />
                </FormField>
              )}
            </div>

            <div className="px-6 py-4 border-t border-gray-100 flex justify-end gap-3">
              <button
                onClick={() => setAttrModalOpen(false)}
                className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleSaveAttr}
                disabled={attrSaving}
                className="px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-medium hover:bg-purple-700 transition-colors disabled:opacity-50"
              >
                {attrSaving ? t('auth.loading') : t('common.action.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Option Modal ── */}
      {optModalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm"
          onClick={(e) => { if (e.target === e.currentTarget) setOptModalOpen(false); }}
        >
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md overflow-hidden max-h-[90vh] flex flex-col" style={{ animation: 'slideUp 0.25s ease-out' }}>
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 shrink-0">
              <h3 className="text-lg font-bold text-gray-900">
                {editingOpt ? t('settings.attribute.edit_option') : t('settings.attribute.add_option')}
              </h3>
              <button onClick={() => setOptModalOpen(false)} className="p-1 hover:bg-gray-100 rounded-lg transition-colors">
                <X size={20} className="text-gray-400" />
              </button>
            </div>

            <div className="px-6 py-5 space-y-4 overflow-y-auto">
              {optFormError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{optFormError}</div>
              )}

              <FormField label={t('settings.common.name')} required>
                <input
                  type="text"
                  value={optForm.name}
                  onChange={(e) => setOptForm(prev => ({ ...prev, name: e.target.value }))}
                  className={inputClass}
                  placeholder={t('settings.common.name')}
                />
              </FormField>

              <FormField label={t('settings.attribute.price')}>
                <div className="relative">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm">€</span>
                  <input
                    type="text"
                    inputMode="decimal"
                    value={optForm.price_modifier}
                    onChange={(e) => setOptForm(prev => ({ ...prev, price_modifier: e.target.value }))}
                    className={`${inputClass} pl-7`}
                    placeholder="0.00"
                  />
                </div>
              </FormField>

              <FormField label={t('settings.attribute.receipt_name')}>
                <input
                  type="text"
                  value={optForm.receipt_name}
                  onChange={(e) => setOptForm(prev => ({ ...prev, receipt_name: e.target.value }))}
                  className={inputClass}
                  placeholder={t('settings.attribute.receipt_name')}
                />
              </FormField>

              <FormField label={t('settings.attribute.kitchen_print_name')}>
                <input
                  type="text"
                  value={optForm.kitchen_print_name}
                  onChange={(e) => setOptForm(prev => ({ ...prev, kitchen_print_name: e.target.value }))}
                  className={inputClass}
                  placeholder={t('settings.attribute.kitchen_print_name')}
                />
              </FormField>

              <CheckboxField
                id="opt-enable-quantity"
                label={t('settings.attribute.enable_quantity')}
                description={t('settings.attribute.enable_quantity_desc')}
                checked={optForm.enable_quantity}
                onChange={(v) => setOptForm(prev => ({ ...prev, enable_quantity: v }))}
              />

              {optForm.enable_quantity && (
                <FormField label={t('settings.attribute.max_quantity')}>
                  <input
                    type="number"
                    min={1}
                    max={99}
                    value={optForm.max_quantity}
                    onChange={(e) => setOptForm(prev => ({ ...prev, max_quantity: e.target.value }))}
                    className={inputClass}
                    placeholder="99"
                  />
                </FormField>
              )}
            </div>

            <div className="px-6 py-4 border-t border-gray-100 flex justify-end gap-3 shrink-0">
              <button
                onClick={() => setOptModalOpen(false)}
                className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleSaveOpt}
                disabled={optSaving}
                className="px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-medium hover:bg-purple-700 transition-colors disabled:opacity-50"
              >
                {optSaving ? t('auth.loading') : t('common.action.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Attribute Confirmation */}
      <ConfirmDialog
        isOpen={!!deleteAttrTarget}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.attribute.confirm.delete')}
        onConfirm={handleDeleteAttr}
        onCancel={() => setDeleteAttrTarget(null)}
        variant="danger"
      />

      {/* Delete Option Confirmation */}
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
