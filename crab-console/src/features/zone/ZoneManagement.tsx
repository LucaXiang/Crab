import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Map, Plus } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField';
import { listZones, createZone, updateZone, deleteZone } from '@/infrastructure/api/management';
import type { Zone, ZoneCreate, ZoneUpdate } from '@/core/types/catalog';

type ModalState = { type: 'closed' } | { type: 'create' } | { type: 'edit'; item: Zone } | { type: 'delete'; item: Zone };

export const ZoneManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<Zone[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [modal, setModal] = useState<ModalState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);

  // Form state
  const [formName, setFormName] = useState('');
  const [formDescription, setFormDescription] = useState('');

  const handleError = useCallback((err: unknown) => {
    if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
    alert(err instanceof ApiError ? err.message : t('catalog.error'));
  }, [clearAuth, navigate, t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      setItems(await listZones(token, storeId));
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(z => z.name.toLowerCase().includes(q) || (z.description && z.description.toLowerCase().includes(q)));
  }, [items, search]);

  const openCreate = () => {
    setFormName(''); setFormDescription('');
    setModal({ type: 'create' });
  };

  const openEdit = (item: Zone) => {
    setFormName(item.name); setFormDescription(item.description || '');
    setModal({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (modal.type === 'create') {
        const data: ZoneCreate = { name: formName.trim(), description: formDescription.trim() || undefined };
        await createZone(token, storeId, data);
      } else if (modal.type === 'edit') {
        const data: ZoneUpdate = { name: formName.trim(), description: formDescription.trim() || undefined };
        await updateZone(token, storeId, modal.item.id, data);
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
      await deleteZone(token, storeId, modal.item.id);
      setModal({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const columns: Column<Zone>[] = useMemo(() => [
    {
      key: 'name', header: t('catalog.name'),
      render: (z) => (
        <div className="flex items-center gap-2">
          <Map className="w-4 h-4 text-blue-500" />
          <span className={`font-medium ${z.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>{z.name}</span>
        </div>
      ),
    },
    {
      key: 'description', header: t('zones.description'),
      render: (z) => <span className="text-slate-500">{z.description || '-'}</span>,
    },
  ], [t]);

  const isFormOpen = modal.type === 'create' || modal.type === 'edit';

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-blue-100 rounded-xl flex items-center justify-center"><Map className="w-5 h-5 text-blue-600" /></div>
          <h1 className="text-xl font-bold text-slate-900">{t('zones.title')}</h1>
        </div>
        <button onClick={openCreate} className="flex items-center gap-1.5 px-3 py-2 bg-primary-500 text-white rounded-lg text-sm font-medium hover:bg-primary-600 transition-colors">
          <Plus className="w-4 h-4" />{t('zones.new')}
        </button>
      </div>

      <FilterBar searchQuery={search} onSearchChange={setSearch} totalCount={filtered.length} countUnit={t('zones.title')} themeColor="blue" />

      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        emptyText={t('zones.empty')}
        getRowKey={(z) => z.id}
        onEdit={openEdit}
        onDelete={(z) => setModal({ type: 'delete', item: z })}
        themeColor="blue"
      />

      {/* Create/Edit Modal */}
      {isFormOpen && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm" onClick={() => setModal({ type: 'closed' })}>
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md p-6 space-y-4" onClick={e => e.stopPropagation()} style={{ animation: 'slideUp 0.25s ease-out' }}>
            <h2 className="text-lg font-bold text-slate-900">{modal.type === 'create' ? t('zones.new') : t('zones.edit')}</h2>
            <FormField label={t('catalog.name')} required>
              <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
            </FormField>
            <FormField label={t('zones.description')}>
              <input value={formDescription} onChange={e => setFormDescription(e.target.value)} className={inputClass} />
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
