import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Tag } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField';
import { listTags, createTag, updateTag, deleteTag } from '@/infrastructure/api/store';
import type { StoreTag, TagCreate, TagUpdate } from '@/core/types/store';

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: StoreTag }
  | { type: 'delete'; item: StoreTag };

export const TagManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [items, setItems] = useState<StoreTag[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
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
    try { setItems(await listTags(token, storeId)); }
    catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(tag => tag.name.toLowerCase().includes(q));
  }, [items, search]);

  const selectedId = panel.type === 'edit' ? panel.item.source_id : null;

  const openCreate = () => {
    setFormName(''); setFormColor('#6366f1'); setFormDisplayOrder(0);
    setPanel({ type: 'create' });
  };

  const openEdit = (item: StoreTag) => {
    if (item.is_system) return;
    setFormName(item.name); setFormColor(item.color || '#6366f1'); setFormDisplayOrder(item.display_order);
    setPanel({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (panel.type === 'create') {
        const data: TagCreate = { name: formName.trim(), color: formColor, display_order: formDisplayOrder };
        await createTag(token, storeId, data);
      } else if (panel.type === 'edit') {
        const data: TagUpdate = { name: formName.trim(), color: formColor, display_order: formDisplayOrder };
        await updateTag(token, storeId, panel.item.source_id, data);
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
      await deleteTag(token, storeId, panel.item.source_id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const renderItem = (tag: StoreTag, isSelected: boolean) => (
    <div className={`px-4 py-3.5 flex items-center gap-3 ${isSelected ? 'font-medium' : ''}`}>
      <div
        className="w-5 h-5 rounded-full border border-gray-200 shrink-0"
        style={{ backgroundColor: tag.color || '#6366f1' }}
      />
      <span className={`text-sm flex-1 truncate ${tag.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
        {tag.name}
      </span>
      {tag.is_system && (
        <span className="text-[10px] px-1.5 py-0.5 rounded bg-indigo-100 text-indigo-700 shrink-0">
          {t('tags.system')}
        </span>
      )}
    </div>
  );

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-indigo-100 rounded-xl flex items-center justify-center">
          <Tag className="w-5 h-5 text-indigo-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('tags.title')}</h1>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(tag) => tag.source_id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('tags.title')}
          onCreateNew={openCreate}
          createLabel={t('tags.new')}
          isCreating={panel.type === 'create'}
          themeColor="indigo"
          loading={loading}
          emptyText={t('tags.empty')}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? t('tags.new') : t('tags.edit')}
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
              <FormField label={t('tags.color')}>
                <div className="flex items-center gap-3">
                  <input
                    type="color"
                    value={formColor}
                    onChange={e => setFormColor(e.target.value)}
                    className="w-10 h-10 rounded-lg border border-gray-200 cursor-pointer p-0.5"
                  />
                  <input
                    value={formColor}
                    onChange={e => setFormColor(e.target.value)}
                    className={inputClass}
                    placeholder="#6366f1"
                  />
                </div>
              </FormField>
              <FormField label={t('catalog.sort_order')}>
                <input
                  type="number"
                  value={formDisplayOrder}
                  onChange={e => setFormDisplayOrder(Number(e.target.value))}
                  className={inputClass}
                />
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
