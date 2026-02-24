import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { FolderTree, Plus, Filter } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, SelectField } from '@/shared/components/FormField';
import { listCategories, createCategory, updateCategory, deleteCategory } from '@/infrastructure/api/store';
import type { StoreCategory, CategoryCreate, CategoryUpdate } from '@/core/types/store';

type ModalState = { type: 'closed' } | { type: 'create' } | { type: 'edit'; item: StoreCategory } | { type: 'delete'; item: StoreCategory };

export const CategoryManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<StoreCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [modal, setModal] = useState<ModalState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);

  // Form state
  const [formName, setFormName] = useState('');
  const [formSortOrder, setFormSortOrder] = useState(0);
  const [formIsVirtual, setFormIsVirtual] = useState(false);
  const [formIsDisplay, setFormIsDisplay] = useState(true);

  const handleError = useCallback((err: unknown) => {
    if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
    alert(err instanceof ApiError ? err.message : t('catalog.error'));
  }, [clearAuth, navigate, t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      setItems(await listCategories(token, storeId));
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(c => c.name.toLowerCase().includes(q));
  }, [items, search]);

  const openCreate = () => {
    setFormName(''); setFormSortOrder(0); setFormIsVirtual(false); setFormIsDisplay(true);
    setModal({ type: 'create' });
  };

  const openEdit = (item: StoreCategory) => {
    setFormName(item.name); setFormSortOrder(item.sort_order);
    setFormIsVirtual(item.is_virtual); setFormIsDisplay(item.is_display);
    setModal({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (modal.type === 'create') {
        const data: CategoryCreate = { name: formName.trim(), sort_order: formSortOrder, is_virtual: formIsVirtual, is_display: formIsDisplay };
        await createCategory(token, storeId, data);
      } else if (modal.type === 'edit') {
        const data: CategoryUpdate = { name: formName.trim(), sort_order: formSortOrder, is_virtual: formIsVirtual, is_display: formIsDisplay };
        await updateCategory(token, storeId, modal.item.source_id, data);
      }
      setModal({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const handleDelete = async () => {
    if (!token || modal.type !== 'delete') return;
    setSaving(true);
    try {
      await deleteCategory(token, storeId, modal.item.source_id);
      setModal({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const columns: Column<StoreCategory>[] = useMemo(() => [
    {
      key: 'name', header: t('catalog.name'),
      render: (c) => (
        <div className="flex items-center gap-2">
          {c.is_virtual ? <Filter className="w-4 h-4 text-purple-500" /> : <FolderTree className="w-4 h-4 text-teal-500" />}
          <span className={`font-medium ${c.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>{c.name}</span>
        </div>
      ),
    },
    {
      key: 'flags', header: t('catalog.active'), width: '120px', align: 'center',
      render: (c) => (
        <div className="flex items-center justify-center gap-1.5">
          {c.is_virtual && <span className="text-xs px-1.5 py-0.5 rounded bg-purple-100 text-purple-700">{t('categories.virtual')}</span>}
          {!c.is_display && <span className="text-xs px-1.5 py-0.5 rounded bg-slate-100 text-slate-500">{t('categories.hidden')}</span>}
        </div>
      ),
    },
    { key: 'sort_order', header: t('catalog.sort_order'), width: '80px', align: 'center', render: (c) => <span className="text-slate-500 tabular-nums">{c.sort_order}</span> },
  ], [t]);

  const isFormOpen = modal.type === 'create' || modal.type === 'edit';

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-teal-100 rounded-xl flex items-center justify-center"><FolderTree className="w-5 h-5 text-teal-600" /></div>
          <h1 className="text-xl font-bold text-slate-900">{t('categories.title')}</h1>
        </div>
        <button onClick={openCreate} className="flex items-center gap-1.5 px-3 py-2 bg-primary-500 text-white rounded-lg text-sm font-medium hover:bg-primary-600 transition-colors">
          <Plus className="w-4 h-4" />{t('categories.new')}
        </button>
      </div>

      <FilterBar searchQuery={search} onSearchChange={setSearch} totalCount={filtered.length} countUnit={t('categories.title')} themeColor="teal" />

      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        emptyText={t('categories.empty')}
        getRowKey={(c) => c.source_id}
        onEdit={openEdit}
        onDelete={(c) => setModal({ type: 'delete', item: c })}
        themeColor="teal"
      />

      {/* Create/Edit Modal */}
      {isFormOpen && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm" onClick={() => setModal({ type: 'closed' })}>
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md p-6 space-y-4" onClick={e => e.stopPropagation()} style={{ animation: 'slideUp 0.25s ease-out' }}>
            <h2 className="text-lg font-bold text-slate-900">{modal.type === 'create' ? t('categories.new') : t('categories.edit')}</h2>
            <FormField label={t('catalog.name')} required>
              <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
            </FormField>
            <FormField label={t('catalog.sort_order')}>
              <input type="number" value={formSortOrder} onChange={e => setFormSortOrder(Number(e.target.value))} className={inputClass} />
            </FormField>
            <SelectField label={t('categories.virtual')} value={formIsVirtual ? 'true' : 'false'} onChange={v => setFormIsVirtual(String(v) === 'true')} options={[{ value: 'false', label: t('common.label.no') }, { value: 'true', label: t('common.label.yes') }]} />
            <SelectField label={t('categories.display')} value={formIsDisplay ? 'true' : 'false'} onChange={v => setFormIsDisplay(String(v) === 'true')} options={[{ value: 'true', label: t('common.label.yes') }, { value: 'false', label: t('common.label.no') }]} />
            <div className="flex justify-end gap-2 pt-2">
              <button onClick={() => setModal({ type: 'closed' })} className="px-4 py-2 text-sm text-slate-600 hover:bg-slate-100 rounded-lg transition-colors">{t('catalog.cancel')}</button>
              <button onClick={handleSave} disabled={saving || !formName.trim()} className="px-4 py-2 text-sm font-medium text-white bg-primary-500 hover:bg-primary-600 rounded-lg transition-colors disabled:opacity-50">{saving ? t('catalog.saving') : t('catalog.save')}</button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={modal.type === 'delete'}
        title={t('catalog.confirm_delete')}
        description={t('catalog.confirm_delete_desc')}
        onConfirm={handleDelete}
        onCancel={() => setModal({ type: 'closed' })}
        variant="danger"
      />
    </div>
  );
};
