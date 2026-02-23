import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Grid3X3, Plus } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, SelectField } from '@/shared/components/FormField';
import { listZones, listTables, createTable, updateTable, deleteTable } from '@/infrastructure/api/management';
import type { Zone, DiningTable, DiningTableCreate, DiningTableUpdate } from '@/core/types/catalog';

type ModalState = { type: 'closed' } | { type: 'create' } | { type: 'edit'; item: DiningTable } | { type: 'delete'; item: DiningTable };

export const TableManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<DiningTable[]>([]);
  const [zones, setZones] = useState<Zone[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [modal, setModal] = useState<ModalState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);

  // Form state
  const [formName, setFormName] = useState('');
  const [formZoneId, setFormZoneId] = useState<number>(0);
  const [formCapacity, setFormCapacity] = useState(4);

  const handleError = useCallback((err: unknown) => {
    if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
    alert(err instanceof ApiError ? err.message : t('catalog.error'));
  }, [clearAuth, navigate, t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      const [tableData, zoneData] = await Promise.all([
        listTables(token, storeId),
        listZones(token, storeId),
      ]);
      setItems(tableData);
      setZones(zoneData);
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const zoneMap = useMemo(() => {
    const m = new Map<number, Zone>();
    zones.forEach(z => m.set(z.id, z));
    return m;
  }, [zones]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(tbl => {
      const zone = zoneMap.get(tbl.zone_id);
      return tbl.name.toLowerCase().includes(q) || (zone && zone.name.toLowerCase().includes(q));
    });
  }, [items, search, zoneMap]);

  const openCreate = () => {
    setFormName(''); setFormZoneId(zones[0]?.id ?? 0); setFormCapacity(4);
    setModal({ type: 'create' });
  };

  const openEdit = (item: DiningTable) => {
    setFormName(item.name); setFormZoneId(item.zone_id); setFormCapacity(item.capacity);
    setModal({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (modal.type === 'create') {
        const data: DiningTableCreate = { name: formName.trim(), zone_id: formZoneId, capacity: formCapacity };
        await createTable(token, storeId, data);
      } else if (modal.type === 'edit') {
        const data: DiningTableUpdate = { name: formName.trim(), zone_id: formZoneId, capacity: formCapacity };
        await updateTable(token, storeId, modal.item.id, data);
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
      await deleteTable(token, storeId, modal.item.id);
      setModal({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const zoneOptions = useMemo(() =>
    zones.map(z => ({ value: String(z.id), label: z.name })),
  [zones]);

  const columns: Column<DiningTable>[] = useMemo(() => [
    {
      key: 'name', header: t('catalog.name'),
      render: (tbl) => (
        <div className="flex items-center gap-2">
          <Grid3X3 className="w-4 h-4 text-indigo-500" />
          <span className={`font-medium ${tbl.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>{tbl.name}</span>
        </div>
      ),
    },
    {
      key: 'zone', header: t('tables.zone'), width: '160px',
      render: (tbl) => {
        const zone = zoneMap.get(tbl.zone_id);
        return zone
          ? <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 text-blue-700 font-medium">{zone.name}</span>
          : <span className="text-slate-400">-</span>;
      },
    },
    {
      key: 'capacity', header: t('tables.capacity'), width: '100px', align: 'center',
      render: (tbl) => <span className="text-slate-500 tabular-nums">{tbl.capacity}</span>,
    },
  ], [t, zoneMap]);

  const isFormOpen = modal.type === 'create' || modal.type === 'edit';

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-indigo-100 rounded-xl flex items-center justify-center"><Grid3X3 className="w-5 h-5 text-indigo-600" /></div>
          <h1 className="text-xl font-bold text-slate-900">{t('tables.title')}</h1>
        </div>
        <button onClick={openCreate} className="flex items-center gap-1.5 px-3 py-2 bg-primary-500 text-white rounded-lg text-sm font-medium hover:bg-primary-600 transition-colors">
          <Plus className="w-4 h-4" />{t('tables.new')}
        </button>
      </div>

      <FilterBar searchQuery={search} onSearchChange={setSearch} totalCount={filtered.length} countUnit={t('tables.title')} themeColor="indigo" />

      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        emptyText={t('tables.empty')}
        getRowKey={(tbl) => tbl.id}
        onEdit={openEdit}
        onDelete={(tbl) => setModal({ type: 'delete', item: tbl })}
        themeColor="indigo"
      />

      {/* Create/Edit Modal */}
      {isFormOpen && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm" onClick={() => setModal({ type: 'closed' })}>
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md p-6 space-y-4" onClick={e => e.stopPropagation()} style={{ animation: 'slideUp 0.25s ease-out' }}>
            <h2 className="text-lg font-bold text-slate-900">{modal.type === 'create' ? t('tables.new') : t('tables.edit')}</h2>
            <FormField label={t('catalog.name')} required>
              <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
            </FormField>
            <SelectField
              label={t('tables.zone')}
              value={String(formZoneId)}
              onChange={v => setFormZoneId(Number(v))}
              options={zoneOptions}
            />
            <FormField label={t('tables.capacity')}>
              <input type="number" value={formCapacity} onChange={e => setFormCapacity(Number(e.target.value))} className={inputClass} min={1} />
            </FormField>
            <div className="flex justify-end gap-2 pt-2">
              <button onClick={() => setModal({ type: 'closed' })} className="px-4 py-2 text-sm text-slate-600 hover:bg-slate-100 rounded-lg transition-colors">{t('catalog.cancel')}</button>
              <button onClick={handleSave} disabled={saving || !formName.trim() || !formZoneId} className="px-4 py-2 text-sm font-medium text-white bg-primary-500 hover:bg-primary-600 rounded-lg transition-colors disabled:opacity-50">{saving ? t('catalog.saving') : t('catalog.save')}</button>
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
