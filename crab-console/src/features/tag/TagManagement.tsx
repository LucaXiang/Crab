import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Tag, Plus } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField';
import { listTags, createTag, updateTag, deleteTag } from '@/infrastructure/api/catalog';
import type { CatalogTag, TagCreate, TagUpdate } from '@/core/types/catalog';

type ModalState = { type: 'closed' } | { type: 'create' } | { type: 'edit'; item: CatalogTag } | { type: 'delete'; item: CatalogTag };

export const TagManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<CatalogTag[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [modal, setModal] = useState<ModalState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);

  // Form state
  const [formName, setFormName] = useState('');
  const [formColor, setFormColor] = useState('#6366f1');
  const [formDisplayOrder, setFormDisplayOrder] = useState(0);

  const handleError = useCallback((err: unknown) => {
    if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
    alert(err instanceof ApiError ? err.message : t('catalog.error'));
  }, [clearAuth, navigate, t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      setItems(await listTags(token, storeId));
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(t => t.name.toLowerCase().includes(q));
  }, [items, search]);

  const openCreate = () => {
    setFormName(''); setFormColor('#6366f1'); setFormDisplayOrder(0);
    setModal({ type: 'create' });
  };

  const openEdit = (item: CatalogTag) => {
    setFormName(item.name); setFormColor(item.color || '#6366f1'); setFormDisplayOrder(item.display_order);
    setModal({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (modal.type === 'create') {
        const data: TagCreate = { name: formName.trim(), color: formColor, display_order: formDisplayOrder };
        await createTag(token, storeId, data);
      } else if (modal.type === 'edit') {
        const data: TagUpdate = { name: formName.trim(), color: formColor, display_order: formDisplayOrder };
        await updateTag(token, storeId, modal.item.source_id, data);
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
      await deleteTag(token, storeId, modal.item.source_id);
      setModal({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const columns: Column<CatalogTag>[] = useMemo(() => [
    {
      key: 'name', header: t('catalog.name'),
      render: (tag) => (
        <div className="flex items-center gap-2">
          <div className="w-4 h-4 rounded-full border border-gray-200" style={{ backgroundColor: tag.color || '#6366f1' }} />
          <span className={`font-medium ${tag.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>{tag.name}</span>
          {tag.is_system && <span className="text-xs px-1.5 py-0.5 rounded bg-indigo-100 text-indigo-700">{t('tags.system')}</span>}
        </div>
      ),
    },
    {
      key: 'color', header: t('tags.color'), width: '100px', align: 'center',
      render: (tag) => (
        <div className="flex items-center justify-center">
          <div className="w-6 h-6 rounded-lg border border-gray-200 shadow-sm" style={{ backgroundColor: tag.color || '#6366f1' }} />
        </div>
      ),
    },
    { key: 'display_order', header: t('catalog.sort_order'), width: '80px', align: 'center', render: (tag) => <span className="text-slate-500 tabular-nums">{tag.display_order}</span> },
  ], [t]);

  const isFormOpen = modal.type === 'create' || modal.type === 'edit';

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-indigo-100 rounded-xl flex items-center justify-center"><Tag className="w-5 h-5 text-indigo-600" /></div>
          <h1 className="text-xl font-bold text-slate-900">{t('tags.title')}</h1>
        </div>
        <button onClick={openCreate} className="flex items-center gap-1.5 px-3 py-2 bg-primary-500 text-white rounded-lg text-sm font-medium hover:bg-primary-600 transition-colors">
          <Plus className="w-4 h-4" />{t('tags.new')}
        </button>
      </div>

      <FilterBar searchQuery={search} onSearchChange={setSearch} totalCount={filtered.length} countUnit={t('tags.title')} themeColor="indigo" />

      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        emptyText={t('tags.empty')}
        getRowKey={(tag) => tag.source_id}
        onEdit={openEdit}
        onDelete={(tag) => setModal({ type: 'delete', item: tag })}
        isEditable={(tag) => !tag.is_system}
        isDeletable={(tag) => !tag.is_system}
        themeColor="indigo"
      />

      {/* Create/Edit Modal */}
      {isFormOpen && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm" onClick={() => setModal({ type: 'closed' })}>
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md p-6 space-y-4" onClick={e => e.stopPropagation()} style={{ animation: 'slideUp 0.25s ease-out' }}>
            <h2 className="text-lg font-bold text-slate-900">{modal.type === 'create' ? t('tags.new') : t('tags.edit')}</h2>
            <FormField label={t('catalog.name')} required>
              <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
            </FormField>
            <FormField label={t('tags.color')}>
              <div className="flex items-center gap-3">
                <input type="color" value={formColor} onChange={e => setFormColor(e.target.value)} className="w-10 h-10 rounded-lg border border-gray-200 cursor-pointer p-0.5" />
                <input value={formColor} onChange={e => setFormColor(e.target.value)} className={inputClass} placeholder="#6366f1" />
              </div>
            </FormField>
            <FormField label={t('catalog.sort_order')}>
              <input type="number" value={formDisplayOrder} onChange={e => setFormDisplayOrder(Number(e.target.value))} className={inputClass} />
            </FormField>
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
