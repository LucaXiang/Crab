import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { FolderTree, Plus, Filter, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import { listCategories, createCategory, updateCategory, deleteCategory, listTags } from '@/infrastructure/api/store';
import type { StoreCategory, StoreTag, CategoryCreate, CategoryUpdate } from '@/core/types/store';

export const CategoryManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [items, setItems] = useState<StoreCategory[]>([]);
  const [tags, setTags] = useState<StoreTag[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [search, setSearch] = useState('');

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<StoreCategory | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form state
  const [formName, setFormName] = useState('');
  const [formSortOrder, setFormSortOrder] = useState(0);
  const [formIsVirtual, setFormIsVirtual] = useState(false);
  const [formIsDisplay, setFormIsDisplay] = useState(true);
  const [formIsActive, setFormIsActive] = useState(true);
  const [formMatchMode, setFormMatchMode] = useState('any');
  const [formTagIds, setFormTagIds] = useState<number[]>([]);
  const [formKitchenPrint, setFormKitchenPrint] = useState(false);
  const [formLabelPrint, setFormLabelPrint] = useState(false);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<StoreCategory | null>(null);

  const loadData = useCallback(async () => {
    if (!token) return;
    try {
      setLoading(true);
      const [cats, allTags] = await Promise.all([
        listCategories(token, storeId),
        listTags(token, storeId),
      ]);
      setItems(cats);
      setTags(allTags);
      setError('');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, t]);

  useEffect(() => { loadData(); }, [loadData]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(c => c.name.toLowerCase().includes(q));
  }, [items, search]);

  const openCreate = () => {
    setEditing(null);
    setFormName('');
    setFormSortOrder(0);
    setFormIsVirtual(false);
    setFormIsDisplay(true);
    setFormIsActive(true);
    setFormMatchMode('any');
    setFormTagIds([]);
    setFormKitchenPrint(false);
    setFormLabelPrint(false);
    setFormError('');
    setModalOpen(true);
  };

  const openEdit = (item: StoreCategory) => {
    setEditing(item);
    setFormName(item.name);
    setFormSortOrder(item.sort_order);
    setFormIsVirtual(item.is_virtual);
    setFormIsDisplay(item.is_display);
    setFormIsActive(item.is_active);
    setFormMatchMode(item.match_mode || 'any');
    setFormTagIds(item.tag_ids ?? []);
    setFormKitchenPrint(item.is_kitchen_print_enabled);
    setFormLabelPrint(item.is_label_print_enabled);
    setFormError('');
    setModalOpen(true);
  };

  const handleSave = async () => {
    if (!token || saving) return;
    if (!formName.trim()) { setFormError(t('settings.common.required_field')); return; }

    setSaving(true);
    setFormError('');
    try {
      if (editing) {
        const data: CategoryUpdate = {
          name: formName.trim(),
          sort_order: formSortOrder,
          is_virtual: formIsVirtual,
          is_display: formIsDisplay,
          is_active: formIsActive,
          is_kitchen_print_enabled: formKitchenPrint,
          is_label_print_enabled: formLabelPrint,
          match_mode: formIsVirtual ? formMatchMode : undefined,
          tag_ids: formIsVirtual ? formTagIds : undefined,
        };
        await updateCategory(token, storeId, editing.source_id, data);
      } else {
        const data: CategoryCreate = {
          name: formName.trim(),
          sort_order: formSortOrder,
          is_virtual: formIsVirtual,
          is_display: formIsDisplay,
          is_kitchen_print_enabled: formKitchenPrint,
          is_label_print_enabled: formLabelPrint,
          match_mode: formIsVirtual ? formMatchMode : undefined,
          tag_ids: formIsVirtual ? formTagIds : undefined,
        };
        await createCategory(token, storeId, data);
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
      await deleteCategory(token, storeId, deleteTarget.source_id);
      setDeleteTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteTarget(null);
    }
  };

  const handleToggleActive = async (cat: StoreCategory) => {
    if (!token) return;
    try {
      await updateCategory(token, storeId, cat.source_id, { is_active: !cat.is_active });
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    }
  };

  const toggleTag = (tagId: number) => {
    setFormTagIds(prev =>
      prev.includes(tagId) ? prev.filter(id => id !== tagId) : [...prev, tagId]
    );
  };

  const matchModeOptions = [
    { value: 'any', label: t('categories.match_any') },
    { value: 'all', label: t('categories.match_all') },
  ];

  const columns: Column<StoreCategory>[] = [
    {
      key: 'name',
      header: t('settings.common.name'),
      render: (c) => (
        <div className="flex items-center gap-2">
          {c.is_virtual
            ? <Filter className="w-4 h-4 text-purple-500" />
            : <FolderTree className="w-4 h-4 text-teal-500" />
          }
          <span className={`font-medium ${c.is_active ? 'text-gray-900' : 'text-gray-400 line-through'}`}>
            {c.name}
          </span>
        </div>
      ),
    },
    {
      key: 'flags',
      header: t('settings.common.status'),
      width: '160px',
      render: (c) => (
        <div className="flex items-center gap-1.5">
          {c.is_virtual && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-purple-100 text-purple-700">{t('categories.virtual')}</span>
          )}
          {!c.is_display && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-gray-100 text-gray-500">{t('categories.hidden')}</span>
          )}
          <button
            onClick={(ev) => { ev.stopPropagation(); handleToggleActive(c); }}
            className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium transition-colors cursor-pointer ${
              c.is_active ? 'bg-green-50 text-green-700 hover:bg-green-100' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
            }`}
          >
            {c.is_active ? t('settings.common.active') : t('settings.common.inactive')}
          </button>
        </div>
      ),
    },
    {
      key: 'sort_order',
      header: t('settings.common.sort_order'),
      width: '80px',
      align: 'center',
      render: (c) => <span className="text-gray-500 tabular-nums">{c.sort_order}</span>,
    },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-teal-100 rounded-xl flex items-center justify-center">
            <FolderTree size={20} className="text-teal-600" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('categories.title')}</h2>
          </div>
        </div>
        <button
          onClick={openCreate}
          className="inline-flex items-center gap-2 px-4 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-medium hover:bg-teal-700 transition-colors shadow-sm"
        >
          <Plus size={16} />
          {t('categories.new')}
        </button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{error}</div>
      )}

      <FilterBar
        searchQuery={search}
        onSearchChange={setSearch}
        totalCount={filtered.length}
        countUnit={t('categories.title')}
        themeColor="teal"
      />

      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        onEdit={openEdit}
        onDelete={(c) => setDeleteTarget(c)}
        getRowKey={(c) => c.source_id}
        themeColor="teal"
      />

      {/* Create/Edit Modal */}
      {modalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm"
          onClick={(e) => { if (e.target === e.currentTarget) setModalOpen(false); }}
        >
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md overflow-hidden max-h-[90vh] flex flex-col" style={{ animation: 'slideUp 0.25s ease-out' }}>
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 shrink-0">
              <h3 className="text-lg font-bold text-gray-900">
                {editing ? t('categories.edit') : t('categories.new')}
              </h3>
              <button onClick={() => setModalOpen(false)} className="p-1 hover:bg-gray-100 rounded-lg transition-colors">
                <X size={20} className="text-gray-400" />
              </button>
            </div>

            {/* Body */}
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
                  autoFocus
                />
              </FormField>

              <FormField label={t('settings.common.sort_order')}>
                <input
                  type="number"
                  value={formSortOrder}
                  onChange={(e) => setFormSortOrder(Number(e.target.value))}
                  className={inputClass}
                />
              </FormField>

              <CheckboxField
                id="cat-is-virtual"
                label={t('categories.virtual')}
                checked={formIsVirtual}
                onChange={setFormIsVirtual}
              />

              <CheckboxField
                id="cat-is-display"
                label={t('categories.display')}
                checked={formIsDisplay}
                onChange={setFormIsDisplay}
              />

              {/* Virtual category settings */}
              {formIsVirtual && (
                <div className="space-y-3 pl-4 border-l-2 border-purple-200">
                  <SelectField
                    label={t('categories.match_mode')}
                    value={formMatchMode}
                    onChange={(v) => setFormMatchMode(String(v))}
                    options={matchModeOptions}
                  />

                  <FormField label={t('categories.tag_filter')}>
                    <div className="flex flex-wrap gap-2">
                      {tags.filter(tag => tag.is_active).map((tag) => (
                        <button
                          key={tag.source_id}
                          type="button"
                          onClick={() => toggleTag(tag.source_id)}
                          className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors ${
                            formTagIds.includes(tag.source_id)
                              ? 'bg-purple-50 text-purple-700 border-purple-300'
                              : 'bg-white text-gray-600 border-gray-200 hover:border-gray-300'
                          }`}
                        >
                          {tag.name}
                        </button>
                      ))}
                      {tags.filter(tag => tag.is_active).length === 0 && (
                        <span className="text-xs text-gray-400">{t('settings.attribute.no_options')}</span>
                      )}
                    </div>
                  </FormField>
                </div>
              )}

              {/* Print settings (regular categories) */}
              {!formIsVirtual && (
                <div className="space-y-2">
                  <CheckboxField
                    id="cat-kitchen-print"
                    label={t('categories.kitchen_print')}
                    checked={formKitchenPrint}
                    onChange={setFormKitchenPrint}
                  />
                  <CheckboxField
                    id="cat-label-print"
                    label={t('categories.label_print')}
                    checked={formLabelPrint}
                    onChange={setFormLabelPrint}
                  />
                </div>
              )}

              {/* Active toggle (edit only) */}
              {editing && (
                <CheckboxField
                  id="cat-is-active"
                  label={t('categories.is_active')}
                  checked={formIsActive}
                  onChange={setFormIsActive}
                />
              )}
            </div>

            {/* Footer */}
            <div className="px-6 py-4 border-t border-gray-100 flex justify-end gap-3 shrink-0">
              <button
                onClick={() => setModalOpen(false)}
                className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleSave}
                disabled={saving || !formName.trim()}
                className="px-4 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-medium hover:bg-teal-700 transition-colors disabled:opacity-50"
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
        description={t('settings.category.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
        variant="danger"
      />
    </div>
  );
};
