import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { FolderTree, Filter } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import { TagPicker } from '@/shared/components/TagPicker/TagPicker';
import { listCategories, createCategory, updateCategory, deleteCategory, listTags, batchUpdateCategorySortOrder } from '@/infrastructure/api/store';
import type { StoreCategory, StoreTag, CategoryCreate, CategoryUpdate } from '@/core/types/store';

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: StoreCategory }
  | { type: 'delete'; item: StoreCategory };

export const CategoryManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const [items, setItems] = useState<StoreCategory[]>([]);
  const [tags, setTags] = useState<StoreTag[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
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

  const handleError = useCallback((err: unknown) => {
    alert(err instanceof ApiError ? err.message : t('auth.error_generic'));
  }, [t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      const [cats, allTags] = await Promise.all([
        listCategories(token, storeId),
        listTags(token, storeId),
      ]);
      setItems(cats);
      setTags(allTags);
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return items;
    const q = search.toLowerCase();
    return items.filter(c => c.name.toLowerCase().includes(q));
  }, [items, search]);

  const selectedId = panel.type === 'edit' ? panel.item.source_id : null;

  const matchModeOptions = [
    { value: 'any', label: t('categories.match_any') },
    { value: 'all', label: t('categories.match_all') },
  ];

  const toggleTag = (tagId: number) => {
    setFormTagIds(prev =>
      prev.includes(tagId) ? prev.filter(id => id !== tagId) : [...prev, tagId]
    );
  };

  const openCreate = () => {
    setFormName(''); setFormSortOrder(0); setFormIsVirtual(false);
    setFormIsDisplay(true); setFormIsActive(true); setFormMatchMode('any');
    setFormTagIds([]); setFormKitchenPrint(false); setFormLabelPrint(false);
    setFormError('');
    setPanel({ type: 'create' });
  };

  const openEdit = (item: StoreCategory) => {
    setFormName(item.name); setFormSortOrder(item.sort_order);
    setFormIsVirtual(item.is_virtual); setFormIsDisplay(item.is_display);
    setFormIsActive(item.is_active); setFormMatchMode(item.match_mode || 'any');
    setFormTagIds(item.tag_ids ?? []); setFormKitchenPrint(item.is_kitchen_print_enabled);
    setFormLabelPrint(item.is_label_print_enabled); setFormError('');
    setPanel({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    if (!formName.trim()) { setFormError(t('settings.common.required_field')); return; }

    setSaving(true);
    setFormError('');
    try {
      if (panel.type === 'edit') {
        const data: CategoryUpdate = {
          name: formName.trim(), sort_order: formSortOrder,
          is_virtual: formIsVirtual, is_display: formIsDisplay, is_active: formIsActive,
          is_kitchen_print_enabled: formKitchenPrint, is_label_print_enabled: formLabelPrint,
          match_mode: formIsVirtual ? formMatchMode : undefined,
          tag_ids: formIsVirtual ? formTagIds : undefined,
        };
        await updateCategory(token, storeId, panel.item.source_id, data);
      } else if (panel.type === 'create') {
        const data: CategoryCreate = {
          name: formName.trim(), sort_order: formSortOrder,
          is_virtual: formIsVirtual, is_display: formIsDisplay,
          is_kitchen_print_enabled: formKitchenPrint, is_label_print_enabled: formLabelPrint,
          match_mode: formIsVirtual ? formMatchMode : undefined,
          tag_ids: formIsVirtual ? formTagIds : undefined,
        };
        await createCategory(token, storeId, data);
      }
      setPanel({ type: 'closed' });
      await load();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally { setSaving(false); }
  };

  const handleDelete = async () => {
    if (!token || panel.type !== 'delete') return;
    setSaving(true);
    try {
      await deleteCategory(token, storeId, panel.item.source_id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const renderItem = (cat: StoreCategory, isSelected: boolean) => (
    <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center gap-2.5">
        {cat.is_virtual
          ? <Filter className="w-4 h-4 text-purple-500 shrink-0" />
          : <FolderTree className="w-4 h-4 text-teal-500 shrink-0" />
        }
        <span className={`text-sm flex-1 truncate ${cat.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
          {cat.name}
        </span>
      </div>
      <div className="flex items-center gap-1.5 mt-1 ml-[26px]">
        {cat.is_virtual && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-purple-100 text-purple-700">
            {t('categories.virtual')}
          </span>
        )}
        {!cat.is_display && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 text-gray-500">
            {t('categories.hidden')}
          </span>
        )}
        <span className="text-xs text-gray-400 tabular-nums ml-auto">#{cat.sort_order}</span>
      </div>
    </div>
  );

  const handleReorder = useCallback(async (reordered: StoreCategory[]) => {
    if (!token) return;
    const withOrder = reordered.map((c, i) => ({ ...c, sort_order: i }));
    setItems(withOrder);
    const sortItems = withOrder.map((c) => ({ id: c.source_id, sort_order: c.sort_order }));
    try { await batchUpdateCategorySortOrder(token, storeId, sortItems); }
    catch (err) { handleError(err); await load(); }
  }, [token, storeId, handleError, load]);

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-teal-100 rounded-xl flex items-center justify-center">
          <FolderTree className="w-5 h-5 text-teal-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('categories.title')}</h1>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(c) => c.source_id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('categories.title')}
          onCreateNew={openCreate}
          createLabel={t('categories.new')}
          isCreating={panel.type === 'create'}
          themeColor="teal"
          loading={loading}
          emptyText={t('categories.empty')}
          onReorder={!search.trim() ? handleReorder : undefined}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? t('categories.new') : t('categories.edit')}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formName.trim()}
            >
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.common.name')} required>
                <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
              </FormField>

              <FormField label={t('settings.common.sort_order')}>
                <input type="number" value={formSortOrder} onChange={e => setFormSortOrder(Number(e.target.value))} className={inputClass} />
              </FormField>

              <CheckboxField id="cat-is-virtual" label={t('categories.virtual')} checked={formIsVirtual} onChange={setFormIsVirtual} />
              <CheckboxField id="cat-is-display" label={t('categories.display')} checked={formIsDisplay} onChange={setFormIsDisplay} />

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
                    <TagPicker tags={tags} selectedIds={formTagIds} onToggle={toggleTag} themeColor="purple" />
                  </FormField>
                </div>
              )}

              {/* Print settings (regular categories) */}
              {!formIsVirtual && (
                <div className="space-y-2">
                  <CheckboxField id="cat-kitchen-print" label={t('categories.kitchen_print')} checked={formKitchenPrint} onChange={setFormKitchenPrint} />
                  <CheckboxField id="cat-label-print" label={t('categories.label_print')} checked={formLabelPrint} onChange={setFormLabelPrint} />
                </div>
              )}

              {/* Active toggle (edit only) */}
              {panel.type === 'edit' && (
                <CheckboxField id="cat-is-active" label={t('categories.is_active')} checked={formIsActive} onChange={setFormIsActive} />
              )}
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.category.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />
    </div>
  );
};
