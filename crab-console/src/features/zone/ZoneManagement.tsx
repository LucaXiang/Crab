import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { MapPin } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField';
import { listZones, createZone, updateZone, deleteZone } from '@/infrastructure/api/management';
import type { Zone, ZoneCreate, ZoneUpdate } from '@/core/types/store';

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: Zone }
  | { type: 'delete'; item: Zone };

export const ZoneManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<Zone[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
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
    try { setItems(await listZones(token, storeId)); }
    catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(z => z.name.toLowerCase().includes(q) || (z.description && z.description.toLowerCase().includes(q)));
  }, [items, search]);

  const selectedId = panel.type === 'edit' ? panel.item.id : null;

  const openCreate = () => {
    setFormName(''); setFormDescription('');
    setPanel({ type: 'create' });
  };

  const openEdit = (item: Zone) => {
    setFormName(item.name); setFormDescription(item.description || '');
    setPanel({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (panel.type === 'create') {
        const data: ZoneCreate = { name: formName.trim(), description: formDescription.trim() || undefined };
        await createZone(token, storeId, data);
      } else if (panel.type === 'edit') {
        const data: ZoneUpdate = { name: formName.trim(), description: formDescription.trim() || undefined };
        await updateZone(token, storeId, panel.item.id, data);
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
      await deleteZone(token, storeId, panel.item.id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const renderItem = (zone: Zone, isSelected: boolean) => (
    <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center gap-2.5">
        <MapPin className="w-4 h-4 text-teal-500 shrink-0" />
        <span className={`text-sm ${zone.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
          {zone.name}
        </span>
      </div>
      {zone.description && (
        <p className="text-xs text-gray-400 mt-1 ml-[26px] truncate">{zone.description}</p>
      )}
    </div>
  );

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      {/* 页面标题 */}
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-teal-100 rounded-xl flex items-center justify-center">
          <MapPin className="w-5 h-5 text-teal-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('zones.title')}</h1>
      </div>

      {/* Master-Detail 布局 */}
      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(z) => z.id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('zones.title')}
          onCreateNew={openCreate}
          createLabel={t('zones.new')}
          isCreating={panel.type === 'create'}
          themeColor="teal"
          loading={loading}
          emptyText={t('zones.empty')}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? t('zones.new') : t('zones.edit')}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formName.trim()}
            >
              <FormField label={t('catalog.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
              </FormField>
              <FormField label={t('zones.description')}>
                <textarea value={formDescription} onChange={e => setFormDescription(e.target.value)} className={inputClass} rows={3} />
              </FormField>
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      {/* 删除确认 */}
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
