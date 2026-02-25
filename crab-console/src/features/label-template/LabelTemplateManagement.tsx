import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Tag, Pencil, Monitor } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { ApiError } from '@/infrastructure/api/client';
import {
  listLabelTemplates,
  createLabelTemplate,
  updateLabelTemplate,
  deleteLabelTemplate,
} from '@/infrastructure/api/store';
import type { LabelTemplate, LabelTemplateCreate, LabelFieldInput } from '@/core/types/store';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, CheckboxField, inputClass } from '@/shared/components/FormField';
import { StatusToggle } from '@/shared/components/StatusToggle/StatusToggle';
import { LabelEditorScreen } from './LabelEditorScreen';
import { DEFAULT_LABEL_TEMPLATES } from './constants';

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: LabelTemplate }
  | { type: 'delete'; item: LabelTemplate };

export const LabelTemplateManagement: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [templates, setTemplates] = useState<LabelTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);
  const [isMobile, setIsMobile] = useState(false);

  // Form state
  const [formName, setFormName] = useState('');
  const [formDescription, setFormDescription] = useState('');
  const [formWidthMm, setFormWidthMm] = useState(40);
  const [formHeightMm, setFormHeightMm] = useState(30);
  const [formIsDefault, setFormIsDefault] = useState(false);
  const [formIsActive, setFormIsActive] = useState(true);

  // Full-screen editor state
  const [editorTemplate, setEditorTemplate] = useState<LabelTemplate | null>(null);

  useEffect(() => {
    const mq = window.matchMedia('(max-width: 1023px)');
    setIsMobile(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsMobile(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  const handleError = useCallback((err: unknown) => {
    if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
    alert(err instanceof ApiError ? err.message : t('catalog.error'));
  }, [clearAuth, navigate, t]);

  const load = useCallback(async () => {
    if (!token) return;
    try { setTemplates(await listLabelTemplates(token, storeId)); }
    catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return templates;
    const q = search.toLowerCase();
    return templates.filter(tmpl => tmpl.name.toLowerCase().includes(q) || tmpl.description?.toLowerCase().includes(q));
  }, [templates, search]);

  const selectedId = panel.type === 'edit' ? panel.item.id : null;

  const openCreate = () => {
    setFormName(''); setFormDescription(''); setFormWidthMm(40); setFormHeightMm(30);
    setFormIsDefault(false); setFormIsActive(true);
    setPanel({ type: 'create' });
  };

  const openEdit = (item: LabelTemplate) => {
    setFormName(item.name);
    setFormDescription(item.description ?? '');
    setFormWidthMm(item.width_mm ?? item.width);
    setFormHeightMm(item.height_mm ?? item.height);
    setFormIsDefault(item.is_default);
    setFormIsActive(item.is_active);
    setPanel({ type: 'edit', item });
  };

  const handleSave = async () => {
    if (!token || saving) return;
    setSaving(true);
    try {
      if (panel.type === 'create') {
        const data: LabelTemplateCreate = {
          name: formName.trim(),
          description: formDescription.trim() || undefined,
          width: formWidthMm,
          height: formHeightMm,
          width_mm: formWidthMm,
          height_mm: formHeightMm,
          is_default: formIsDefault,
          is_active: formIsActive,
        };
        await createLabelTemplate(token, storeId, data);
      } else if (panel.type === 'edit') {
        await updateLabelTemplate(token, storeId, panel.item.id, {
          name: formName.trim(),
          description: formDescription.trim() || undefined,
          width: formWidthMm,
          height: formHeightMm,
          width_mm: formWidthMm,
          height_mm: formHeightMm,
          is_default: formIsDefault,
          is_active: formIsActive,
        });
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
      await deleteLabelTemplate(token, storeId, panel.item.id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const handleCreateFromPreset = async (preset: (typeof DEFAULT_LABEL_TEMPLATES)[number]) => {
    if (!token) return;
    try {
      const payload: LabelTemplateCreate = {
        name: preset.name,
        description: preset.description,
        width: preset.width,
        height: preset.height,
        padding: preset.padding,
        width_mm: preset.width_mm,
        height_mm: preset.height_mm,
        is_default: preset.is_default,
        is_active: preset.is_active,
        fields: preset.fields as LabelFieldInput[],
      };
      await createLabelTemplate(token, storeId, payload);
      await load();
    } catch (err) { handleError(err); }
  };

  const handleEditorSave = async (tmpl: LabelTemplate) => {
    if (!token) return;
    try {
      await updateLabelTemplate(token, storeId, tmpl.id, {
        name: tmpl.name,
        description: tmpl.description,
        width: tmpl.width,
        height: tmpl.height,
        padding: tmpl.padding,
        width_mm: tmpl.width_mm,
        height_mm: tmpl.height_mm,
        padding_mm_x: tmpl.padding_mm_x,
        padding_mm_y: tmpl.padding_mm_y,
        render_dpi: tmpl.render_dpi,
        test_data: tmpl.test_data,
        fields: tmpl.fields as LabelFieldInput[],
        is_default: tmpl.is_default,
        is_active: tmpl.is_active,
      });
      setEditorTemplate(null);
      await load();
    } catch (err) { handleError(err); }
  };

  const noop = (e: React.MouseEvent) => { e.stopPropagation(); };

  const renderItem = (tmpl: LabelTemplate, isSelected: boolean) => (
    <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center justify-between">
        <span className="text-sm text-slate-900 truncate">{tmpl.name}</span>
        {tmpl.is_default && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-blue-50 text-blue-600 font-medium shrink-0 ml-2">
            Default
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 mt-1 text-xs text-gray-400">
        <span className="font-mono">{tmpl.width_mm ?? tmpl.width}×{tmpl.height_mm ?? tmpl.height}mm</span>
        <span>·</span>
        <span>{tmpl.fields.length} campos</span>
        {!tmpl.is_active && <StatusToggle isActive={false} onClick={noop} disabled />}
      </div>
    </div>
  );

  // Full-screen editor mode
  if (editorTemplate) {
    return (
      <LabelEditorScreen
        template={editorTemplate}
        onSave={handleEditorSave}
        onClose={() => setEditorTemplate(null)}
      />
    );
  }

  // Mobile: desktop-only hint
  if (isMobile) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-gray-400 gap-3 px-8">
        <Monitor size={40} className="text-gray-300" />
        <p className="text-sm text-center">{t('common.hint.desktop_only')}</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-indigo-100 rounded-xl flex items-center justify-center">
          <Tag className="w-5 h-5 text-indigo-600" />
        </div>
        <h1 className="text-xl font-bold text-slate-900">{t('settings.label.templates')}</h1>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(tmpl) => tmpl.id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit="plantillas"
          onCreateNew={openCreate}
          createLabel={t('common.action.create')}
          isCreating={panel.type === 'create'}
          themeColor="indigo"
          loading={loading}
          emptyText={t('settings.label.no_templates')}
        >
          {(panel.type === 'create' || panel.type === 'edit') ? (
            <DetailPanel
              title={panel.type === 'create'
                ? t('common.action.create')
                : formName || panel.item.name}
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

              <FormField label={t('catalog.description')}>
                <input value={formDescription} onChange={e => setFormDescription(e.target.value)} className={inputClass} />
              </FormField>

              <div className="grid grid-cols-2 gap-4">
                <FormField label={`${t('settings.label.width')} (mm)`}>
                  <input
                    type="number"
                    value={formWidthMm}
                    onChange={e => setFormWidthMm(Number(e.target.value))}
                    className={inputClass}
                    min={1}
                  />
                </FormField>
                <FormField label={`${t('settings.label.height')} (mm)`}>
                  <input
                    type="number"
                    value={formHeightMm}
                    onChange={e => setFormHeightMm(Number(e.target.value))}
                    className={inputClass}
                    min={1}
                  />
                </FormField>
              </div>

              <CheckboxField
                id="is_default"
                label={t('settings.label.is_default')}
                description={t('settings.label.is_default_desc')}
                checked={formIsDefault}
                onChange={setFormIsDefault}
              />

              <CheckboxField
                id="is_active"
                label={t('settings.common.active')}
                checked={formIsActive}
                onChange={setFormIsActive}
              />

              {/* Fields summary — edit mode only */}
              {panel.type === 'edit' && panel.item.fields.length > 0 && (
                <div className="mt-2">
                  <h4 className="text-sm font-medium text-gray-700 mb-2">
                    Campos ({panel.item.fields.length})
                  </h4>
                  <div className="space-y-1">
                    {panel.item.fields.map(f => (
                      <div key={f.field_id} className="flex items-center gap-2 px-3 py-1.5 bg-gray-50 rounded-lg text-xs">
                        <span className="font-mono text-indigo-600">{f.field_type}</span>
                        <span className="text-gray-400">:</span>
                        <span className="text-gray-600 truncate">{f.data_source || f.name || f.field_id}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Visual editor entry — edit mode only */}
              {panel.type === 'edit' && (
                <button
                  onClick={() => setEditorTemplate(panel.item)}
                  className="flex items-center gap-2 w-full px-4 py-3 mt-2 text-sm font-medium text-indigo-600 bg-indigo-50 rounded-xl hover:bg-indigo-100 transition-colors"
                >
                  <Pencil size={16} />
                  Abrir editor visual
                </button>
              )}
            </DetailPanel>
          ) : (
            /* Empty state with preset buttons when no templates */
            panel.type === 'closed' && templates.length === 0 && !loading && (
              <div className="flex flex-col items-center justify-center h-full text-center px-8">
                <Tag size={40} className="text-gray-300 mb-4" />
                <h3 className="text-lg font-bold text-gray-700 mb-2">Sin plantillas</h3>
                <p className="text-sm text-gray-500 mb-6">{t('settings.label.no_templates_desc')}</p>
                <div className="flex flex-col gap-2 w-full max-w-xs">
                  {DEFAULT_LABEL_TEMPLATES.map((preset, i) => (
                    <button
                      key={i}
                      onClick={() => handleCreateFromPreset(preset)}
                      className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm font-medium"
                    >
                      {preset.name}
                    </button>
                  ))}
                </div>
              </div>
            )
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('common.action.delete')}
        description={t('settings.label.delete_confirm', { name: panel.type === 'delete' ? panel.item.name : '' })}
        variant="danger"
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
      />
    </div>
  );
};
