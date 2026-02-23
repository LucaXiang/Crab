import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, SlidersHorizontal, X, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listAttributes, createAttribute, updateAttribute, deleteAttribute } from '@/infrastructure/api/catalog';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField/FormField';
import { CheckboxField } from '@/shared/components/FormField/FormField';
import type { CatalogAttribute, AttributeCreate, AttributeUpdate, AttributeOptionInput } from '@/core/types/catalog';
import { formatCurrency } from '@/utils/format';

interface FormOption {
  name: string;
  price_modifier: number;
}

export const AttributeManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [attributes, setAttributes] = useState<CatalogAttribute[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<CatalogAttribute | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields
  const [formName, setFormName] = useState('');
  const [formIsMultiSelect, setFormIsMultiSelect] = useState(false);
  const [formMaxSelections, setFormMaxSelections] = useState<number | ''>('');
  const [formOptions, setFormOptions] = useState<FormOption[]>([]);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<CatalogAttribute | null>(null);

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
    if (!searchQuery.trim()) return attributes;
    const q = searchQuery.toLowerCase();
    return attributes.filter(a => a.name.toLowerCase().includes(q));
  }, [attributes, searchQuery]);

  const openCreate = () => {
    setEditing(null);
    setFormName('');
    setFormIsMultiSelect(false);
    setFormMaxSelections('');
    setFormOptions([]);
    setFormError('');
    setModalOpen(true);
  };

  const openEdit = (attr: CatalogAttribute) => {
    setEditing(attr);
    setFormName(attr.name);
    setFormIsMultiSelect(attr.is_multi_select);
    setFormMaxSelections(attr.max_selections ?? '');
    setFormOptions(
      attr.options.map(o => ({ name: o.name, price_modifier: o.price_modifier }))
    );
    setFormError('');
    setModalOpen(true);
  };

  const addOption = () => {
    setFormOptions([...formOptions, { name: '', price_modifier: 0 }]);
  };

  const removeOption = (index: number) => {
    setFormOptions(formOptions.filter((_, i) => i !== index));
  };

  const updateOption = (index: number, field: keyof FormOption, value: string | number) => {
    setFormOptions(formOptions.map((o, i) =>
      i === index ? { ...o, [field]: value } : o
    ));
  };

  const buildOptionInputs = (): AttributeOptionInput[] =>
    formOptions.map((o, i) => ({
      name: o.name.trim(),
      price_modifier: o.price_modifier,
      display_order: i,
      enable_quantity: false,
    }));

  const handleSave = async () => {
    if (!token) return;
    if (!formName.trim()) { setFormError(t('settings.common.required_field')); return; }

    setSaving(true);
    setFormError('');
    try {
      if (editing) {
        const payload: AttributeUpdate = {
          name: formName.trim(),
          is_multi_select: formIsMultiSelect,
          max_selections: formMaxSelections !== '' ? Number(formMaxSelections) : undefined,
          options: buildOptionInputs(),
        };
        await updateAttribute(token, storeId, editing.source_id, payload);
      } else {
        const payload: AttributeCreate = {
          name: formName.trim(),
          is_multi_select: formIsMultiSelect,
          max_selections: formMaxSelections !== '' ? Number(formMaxSelections) : undefined,
          options: buildOptionInputs(),
        };
        await createAttribute(token, storeId, payload);
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
      await deleteAttribute(token, storeId, deleteTarget.source_id);
      setDeleteTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteTarget(null);
    }
  };

  const columns: Column<CatalogAttribute>[] = [
    {
      key: 'name',
      header: t('settings.common.name'),
      render: (a) => <span className="font-medium text-gray-900">{a.name}</span>,
    },
    {
      key: 'multi_select',
      header: t('settings.attribute.multi_select'),
      width: '130px',
      render: (a) => (
        <span className={`inline-flex px-2.5 py-0.5 rounded-full text-xs font-medium border ${
          a.is_multi_select
            ? 'bg-purple-50 text-purple-700 border-purple-200'
            : 'bg-gray-50 text-gray-600 border-gray-200'
        }`}>
          {a.is_multi_select ? t('settings.attribute.multi') : t('settings.attribute.single')}
        </span>
      ),
    },
    {
      key: 'options_count',
      header: t('settings.attribute.options'),
      width: '100px',
      align: 'center',
      render: (a) => (
        <span className="text-sm text-gray-600">{a.options.length}</span>
      ),
    },
    {
      key: 'status',
      header: t('settings.common.status'),
      width: '100px',
      render: (a) => (
        <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
          a.is_active ? 'bg-green-50 text-green-700' : 'bg-gray-100 text-gray-500'
        }`}>
          {a.is_active ? t('settings.common.active') : t('settings.common.inactive')}
        </span>
      ),
    },
  ];

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
          onClick={openCreate}
          className="inline-flex items-center gap-2 px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-medium hover:bg-purple-700 transition-colors shadow-sm"
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
        countUnit={t('settings.attribute.unit')}
        themeColor="purple"
      />

      {/* Table */}
      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        onEdit={openEdit}
        onDelete={(a) => setDeleteTarget(a)}
        getRowKey={(a) => a.source_id}
        themeColor="purple"
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
                {editing ? t('common.action.edit') : t('common.action.add')} {t('settings.attribute.title')}
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
                  placeholder={t('settings.attribute.name_placeholder')}
                />
              </FormField>

              <CheckboxField
                id="is_multi_select"
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

              {/* Options */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <label className="block text-sm font-medium text-gray-700">{t('settings.attribute.options')}</label>
                  <button
                    type="button"
                    onClick={addOption}
                    className="inline-flex items-center gap-1 px-3 py-1.5 text-xs font-medium text-purple-700 bg-purple-50 rounded-lg hover:bg-purple-100 transition-colors"
                  >
                    <Plus size={12} />
                    {t('settings.attribute.add_option')}
                  </button>
                </div>

                {formOptions.length === 0 && (
                  <div className="text-sm text-gray-400 text-center py-4 border border-dashed border-gray-200 rounded-xl">
                    {t('settings.attribute.no_options')}
                  </div>
                )}

                {formOptions.map((opt, idx) => (
                  <div key={idx} className="flex items-center gap-2 bg-gray-50 rounded-xl p-3">
                    <input
                      type="text"
                      value={opt.name}
                      onChange={(e) => updateOption(idx, 'name', e.target.value)}
                      className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-purple-500/20 focus:border-purple-500"
                      placeholder={t('settings.common.name')}
                    />
                    <input
                      type="number"
                      value={opt.price_modifier}
                      onChange={(e) => updateOption(idx, 'price_modifier', Number(e.target.value))}
                      className="w-28 px-3 py-2 border border-gray-200 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-purple-500/20 focus:border-purple-500"
                      placeholder={t('settings.attribute.price')}
                      step="0.01"
                    />
                    <button
                      type="button"
                      onClick={() => removeOption(idx)}
                      className="p-2 text-red-500 hover:bg-red-50 rounded-lg transition-colors"
                    >
                      <Trash2 size={14} />
                    </button>
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
                className="px-4 py-2.5 bg-purple-600 text-white rounded-xl text-sm font-medium hover:bg-purple-700 transition-colors disabled:opacity-50"
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
        description={t('settings.attribute.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
        variant="danger"
      />
    </div>
  );
};
