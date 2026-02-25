import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Grid3X3 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, SelectField } from '@/shared/components/FormField';
import { listZones, listTables, createTable, updateTable, deleteTable } from '@/infrastructure/api/management';
import type { Zone, DiningTable, DiningTableCreate, DiningTableUpdate } from '@/core/types/store';

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: DiningTable }
  | { type: 'delete'; item: DiningTable };

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
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
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

  const zoneMap = useMemo(() => new Map(zones.map(z => [z.id, z.name])), [zones]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(tbl => {
      const zoneName = zoneMap.get(tbl.zone_id);
      return tbl.name.toLowerCase().includes(q) || (zoneName && zoneName.toLowerCase().includes(q));
    });
  }, [items, search, zoneMap]);

  const selectedId = panel.type === 'edit' ? panel.item.id : null;

  const zoneOptions = useMemo(() =>
    zones.filter(z => z.is_active).map(z => ({ value: String(z.id), label: z.name })),
  [zones]);

  const openCreate = () => {
    setFormName(''); setFormZoneId(zones[0]?.id ?? 0); setFormCapacity(4);
    setPanel({ type: 'create' });
  };

  const openEdit = (item: DiningTable) => {
    setFormName(item.name); setFormZoneId(item.zone_id); setFormCapacity(item.capacity);
    setPanel({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (panel.type === 'create') {
        const data: DiningTableCreate = { name: formName.trim(), zone_id: formZoneId, capacity: formCapacity };
        await createTable(token, storeId, data);
      } else if (panel.type === 'edit') {
        const data: DiningTableUpdate = { name: formName.trim(), zone_id: formZoneId, capacity: formCapacity };
        await updateTable(token, storeId, panel.item.id, data);
      }
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const handleDelete = async () => {
    if (!token || panel.type !== 'delete') return;
    setSaving(true);
    try {
      await deleteTable(token, storeId, panel.item.id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const renderItem = (table: DiningTable, isSelected: boolean) => (
    <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <Grid3X3 className="w-4 h-4 text-blue-500 shrink-0" />
          <span className={`text-sm ${table.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
            {table.name}
          </span>
        </div>
        <span className="text-xs text-gray-400 tabular-nums">{table.capacity} {t('tables.seats')}</span>
      </div>
      <p className="text-xs text-gray-400 mt-0.5 ml-[26px]">{zoneMap.get(table.zone_id) || '-'}</p>
    </div>
  );

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-blue-100 rounded-xl flex items-center justify-center">
          <Grid3X3 className="w-5 h-5 text-blue-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('tables.title')}</h1>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(tbl) => tbl.id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('tables.title')}
          onCreateNew={openCreate}
          createLabel={t('tables.new')}
          isCreating={panel.type === 'create'}
          themeColor="blue"
          loading={loading}
          emptyText={t('tables.empty')}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? t('tables.new') : t('tables.edit')}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formName.trim() || !formZoneId}
            >
              <FormField label={t('catalog.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
              </FormField>
              <SelectField
                label={t('tables.zone')}
                value={String(formZoneId)}
                onChange={v => setFormZoneId(Number(v))}
                options={zoneOptions}
                required
              />
              <FormField label={t('tables.capacity')}>
                <input type="number" value={formCapacity} onChange={e => setFormCapacity(Number(e.target.value))} className={inputClass} min={1} />
              </FormField>
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('catalog.confirm_delete')}
        description={t('catalog.confirm_delete_desc')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />
    </div>
  );
};
